//! Lua 5.4 plugins (plugin API 1 for scripts).
//!
//! `main.lua` returns a plugin table. Contribution fields use the same names
//! and shapes as the Rust `Contributions`; functions run in a sandbox (see
//! [`sandbox`]) and receive a `ctx` table with the host services. See
//! `docs/plugins/lua.md` and `plugins/hello-lua`.

mod sandbox;
mod ui_api;

use archipel_ide_plugin_api::{
    CommandError, CommandResult, Contributions, EditorTabRequest, Notification, NotificationLevel,
    Permission, Plugin, PluginError, PluginHost, PluginKind, PluginManifest, StatusWidgetState,
    Value as Json, ViewContext, ViewFrame, ViewNode, ViewPlacement,
};
use mlua::{DeserializeOptions, Function, Lua, LuaSerdeExt, RegistryKey, Table, Value};
use sandbox::Budget;
use std::cell::RefCell;
use std::path::Path;
use std::time::Duration;

/// Time budget of `activate`, commands and `poll`.
const CALL_BUDGET: Duration = Duration::from_secs(2);
/// Time budget of one view frame.
const VIEW_BUDGET: Duration = Duration::from_millis(250);

/// Runtime for `kind = "lua"` plugins.
#[derive(Debug, Default, Clone, Copy)]
pub struct LuaLoader;

impl archipel_ide_plugins::PluginLoader for LuaLoader {
    fn kind(&self) -> PluginKind {
        PluginKind::Lua
    }

    fn load(&self, manifest: &PluginManifest, dir: &Path) -> Result<Box<dyn Plugin>, PluginError> {
        Ok(Box::new(LuaPlugin::load(manifest.clone(), dir)?))
    }
}

pub struct LuaPlugin {
    manifest: PluginManifest,
    lua: Lua,
    plugin: RegistryKey,
    budget: Budget,
    contributions: Contributions,
    /// Last view error, shown in place of the view until it renders again.
    view_error: Option<String>,
}

fn lua_error(manifest: &PluginManifest, error: impl std::fmt::Display) -> PluginError {
    PluginError::Activation {
        plugin: manifest.id.clone(),
        message: error.to_string(),
    }
}

fn options() -> DeserializeOptions {
    DeserializeOptions::new().deny_unsupported_types(false)
}

impl LuaPlugin {
    pub fn load(manifest: PluginManifest, dir: &Path) -> Result<Self, PluginError> {
        let entry = manifest.entry.clone().ok_or_else(|| {
            PluginError::Manifest(format!("{}: Lua plugins need an `entry`", manifest.id))
        })?;
        let source =
            sandbox::read_source(&dir.join(&entry)).map_err(|e| lua_error(&manifest, e))?;
        let budget = Budget::default();
        let lua =
            sandbox::new_state(&manifest.id, dir, &budget).map_err(|e| lua_error(&manifest, e))?;
        let value: Value = budget
            .run(CALL_BUDGET, || {
                lua.load(source)
                    .set_name(format!("@{entry}"))
                    .set_mode(mlua::ChunkMode::Text)
                    .eval()
            })
            .map_err(|e| lua_error(&manifest, e))?;
        let Value::Table(table) = value else {
            return Err(lua_error(
                &manifest,
                format!(
                    "{entry} must return the plugin table (got {})",
                    value.type_name()
                ),
            ));
        };
        let contributions =
            parse_contributions(&lua, &table).map_err(|e| lua_error(&manifest, e))?;
        let plugin = lua
            .create_registry_value(table)
            .map_err(|e| lua_error(&manifest, e))?;
        Ok(Self {
            manifest,
            lua,
            plugin,
            budget,
            contributions,
            view_error: None,
        })
    }

    fn table(&self) -> mlua::Result<Table> {
        self.lua.registry_value(&self.plugin)
    }

    /// Optional function `name` of the plugin table.
    fn function(&self, name: &str) -> mlua::Result<Option<Function>> {
        Ok(match self.table()?.get::<Value>(name)? {
            Value::Function(function) => Some(function),
            _ => None,
        })
    }

    /// Runs `f` with a `ctx` table bound to `host`.
    fn with_ctx<'env, 'h: 'env, R>(
        &self,
        host: &'env RefCell<&'h mut dyn PluginHost>,
        budget: Duration,
        f: impl for<'scope> FnOnce(&Lua, &'scope mlua::Scope<'scope, 'env>, Table) -> mlua::Result<R>,
    ) -> mlua::Result<R> {
        let lua = &self.lua;
        self.budget.run(budget, || {
            lua.scope(|scope| {
                let scope = &*scope;
                let ctx = context_table(lua, scope, host)?;
                f(lua, scope, ctx)
            })
        })
    }

    /// Calls the optional plugin function `name(ctx)`.
    fn call_hook(&self, name: &str, host: &mut dyn PluginHost) -> mlua::Result<()> {
        let Some(function) = self.function(name)? else {
            return Ok(());
        };
        let host = RefCell::new(host);
        self.with_ctx(&host, CALL_BUDGET, |_, _, ctx| function.call::<()>(ctx))
    }
}

/// Parses a contribution field (`commands`, `menus`, …) with serde; Lua
/// functions in the tables are ignored here.
fn field<T: serde::de::DeserializeOwned + Default>(
    lua: &Lua,
    table: &Table,
    name: &str,
) -> mlua::Result<T> {
    match table.get::<Value>(name)? {
        Value::Nil => Ok(T::default()),
        value => lua
            .from_value_with(value, options())
            .map_err(|e| mlua::Error::runtime(format!("plugin.{name}: {e}"))),
    }
}

fn parse_contributions(lua: &Lua, table: &Table) -> mlua::Result<Contributions> {
    Ok(Contributions {
        commands: field(lua, table, "commands")?,
        menus: field(lua, table, "menus")?,
        keybindings: field(lua, table, "keybindings")?,
        settings_pages: field(lua, table, "settings_pages")?,
        tool_windows: field(lua, table, "tool_windows")?,
        editors: field(lua, table, "editors")?,
        status_widgets: field(lua, table, "status_widgets")?,
    })
}

fn permission(name: &str) -> Option<Permission> {
    Permission::ALL.into_iter().find(|p| p.id() == name)
}

/// The `ctx` table: host services for the running call.
fn context_table<'scope, 'env, 'h>(
    lua: &Lua,
    scope: &'scope mlua::Scope<'scope, 'env>,
    host: &'env RefCell<&'h mut dyn PluginHost>,
) -> mlua::Result<Table>
where
    'h: 'env,
{
    let ctx = lua.create_table()?;
    let busy = || mlua::Error::runtime("ctx is busy (re-entrant call)");
    ctx.set("plugin_id", host.borrow().plugin_id().to_owned())?;
    ctx.set(
        "project_root",
        host.borrow()
            .project_root()
            .map(|root| root.display().to_string()),
    )?;
    ctx.set(
        "status",
        scope.create_function(move |_, message: String| {
            host.try_borrow_mut()
                .map_err(|_| busy())?
                .show_status(&message);
            Ok(())
        })?,
    )?;
    ctx.set(
        "notify",
        scope.create_function(
            move |_, (title, body, level): (String, Option<String>, Option<String>)| {
                let level = match level.as_deref() {
                    Some("error") => NotificationLevel::Error,
                    Some("warning") => NotificationLevel::Warning,
                    _ => NotificationLevel::Info,
                };
                host.try_borrow_mut()
                    .map_err(|_| busy())?
                    .notify(Notification {
                        level,
                        title,
                        body: body.unwrap_or_default(),
                    });
                Ok(())
            },
        )?,
    )?;
    ctx.set(
        "dispatch",
        scope.create_function(move |lua, (command, params): (String, Option<Value>)| {
            let params: Json = match params {
                Some(value) => lua.from_value_with(value, options())?,
                None => Json::Null,
            };
            host.try_borrow_mut()
                .map_err(|_| busy())?
                .dispatch(&command, params);
            Ok(())
        })?,
    )?;
    ctx.set(
        "open_tool_window",
        scope.create_function(move |_, id: String| {
            host.try_borrow_mut()
                .map_err(|_| busy())?
                .open_tool_window(&id);
            Ok(())
        })?,
    )?;
    ctx.set(
        "open_tab",
        scope.create_function(
            move |_, (view, title, key): (String, String, Option<String>)| {
                let mut request = EditorTabRequest::new(view, title);
                if let Some(key) = key {
                    request = request.key(key);
                }
                host.try_borrow_mut()
                    .map_err(|_| busy())?
                    .open_editor_tab(request);
                Ok(())
            },
        )?,
    )?;
    ctx.set(
        "setting",
        scope.create_function(move |lua, key: String| {
            let value = host.try_borrow().map_err(|_| busy())?.setting(&key);
            match value {
                Some(value) => lua.to_value(&value),
                None => Ok(Value::Nil),
            }
        })?,
    )?;
    ctx.set(
        "set_setting",
        scope.create_function(move |lua, (key, value): (String, Value)| {
            let value: Json = lua.from_value_with(value, options())?;
            host.try_borrow_mut()
                .map_err(|_| busy())?
                .set_setting(&key, value)
                .map_err(mlua::Error::runtime)
        })?,
    )?;
    ctx.set(
        "has_permission",
        scope.create_function(move |_, name: String| {
            Ok(permission(&name)
                .is_some_and(|p| host.try_borrow().is_ok_and(|h| h.has_permission(p))))
        })?,
    )?;
    ctx.set(
        "request_repaint",
        scope.create_function(move |_, ()| {
            host.try_borrow_mut().map_err(|_| busy())?.request_repaint();
            Ok(())
        })?,
    )?;
    ctx.set(
        "refresh_contributions",
        scope.create_function(move |_, ()| {
            host.try_borrow_mut()
                .map_err(|_| busy())?
                .refresh_contributions();
            Ok(())
        })?,
    )?;
    Ok(ctx)
}

fn placement_table(lua: &Lua, view: &ViewContext) -> mlua::Result<Table> {
    let table = lua.create_table()?;
    table.set("view", view.view.clone())?;
    match &view.placement {
        ViewPlacement::ToolWindow { id } => {
            table.set("kind", "tool_window")?;
            table.set("id", id.clone())?;
        }
        ViewPlacement::EditorTab { key, path } => {
            table.set("kind", "editor_tab")?;
            table.set("key", key.clone())?;
            table.set("path", path.as_ref().map(|p| p.display().to_string()))?;
        }
        ViewPlacement::App => table.set("kind", "app")?,
        ViewPlacement::SettingsPage { id } => {
            table.set("kind", "settings_page")?;
            table.set("id", id.clone())?;
        }
    }
    Ok(table)
}

impl Plugin for LuaPlugin {
    fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }

    fn contributions(&self) -> Contributions {
        self.contributions.clone()
    }

    fn activate(&mut self, host: &mut dyn PluginHost) -> Result<(), PluginError> {
        self.call_hook("activate", host)
            .map_err(|e| lua_error(&self.manifest, e))
    }

    fn deactivate(&mut self, host: &mut dyn PluginHost) {
        if let Err(error) = self.call_hook("deactivate", host) {
            eprintln!("[{}] deactivate failed: {error}", self.manifest.id);
        }
    }

    fn execute(&mut self, command: &str, params: Json, host: &mut dyn PluginHost) -> CommandResult {
        let failed = |e: mlua::Error| CommandError::Failed(format!("{command}: {e}"));
        let commands: Table = self
            .table()
            .and_then(|t| t.get("commands"))
            .map_err(failed)?;
        let mut run = None;
        for entry in commands.sequence_values::<Table>() {
            let entry = entry.map_err(failed)?;
            if entry.get::<String>("id").map_err(failed)? == command {
                run = Some(entry.get::<Function>("run").map_err(|_| {
                    CommandError::Failed(format!("{command}: command has no `run` function"))
                })?);
                break;
            }
        }
        let run = run.ok_or_else(|| CommandError::Unknown(command.into()))?;
        let host = RefCell::new(host);
        self.with_ctx(&host, CALL_BUDGET, |lua, _, ctx| {
            let params = lua.to_value(&params)?;
            let result: Value = run.call((ctx, params))?;
            lua.from_value_with::<Json>(result, options())
        })
        .map_err(failed)
    }

    fn render_view(
        &mut self,
        view: &ViewContext,
        frame: &mut ViewFrame<'_>,
        host: &mut dyn PluginHost,
    ) {
        let Some(ui) = archipel_ide_plugin_ui::ui(frame) else {
            return;
        };
        let render = self
            .table()
            .and_then(|t| t.get::<Table>("views"))
            .and_then(|views| views.get::<Value>(view.view.as_str()));
        let function = match render {
            Ok(Value::Function(function)) => function,
            _ => {
                ui.colored_label(
                    ui.visuals().error_fg_color,
                    format!("Lua view `{}` is not defined", view.view),
                );
                return;
            }
        };
        let on_event = self.function("on_view_event").ok().flatten();
        let scope_id = format!("{}:{}", self.manifest.id, view.view);
        let host = RefCell::new(host);
        let cell = RefCell::new(ui);
        let result = self.with_ctx(&host, VIEW_BUDGET, |lua, scope, ctx| {
            let ui_table = ui_api::ui_table(lua, scope, &cell, &scope_id)?;
            let placement = placement_table(lua, view)?;
            let returned: Value = function.call((ctx.clone(), ui_table, placement))?;
            if returned.is_nil() {
                return Ok(());
            }
            // A returned table is a declarative view tree.
            let node: ViewNode = lua.from_value_with(returned, options())?;
            let events = {
                let mut guard = cell
                    .try_borrow_mut()
                    .map_err(|_| mlua::Error::runtime("ui busy"))?;
                archipel_ide_plugin_ui::render_node(&mut guard, &scope_id, &node)
            };
            if let Some(on_event) = &on_event {
                for event in events {
                    let event: Value = lua.to_value(&event)?;
                    on_event.call::<()>((ctx.clone(), view.view.clone(), event))?;
                }
            }
            Ok(())
        });
        match result {
            Ok(()) => self.view_error = None,
            Err(error) => {
                let message = error.to_string();
                let ui = cell.into_inner();
                ui.colored_label(ui.visuals().error_fg_color, format!("Lua error: {message}"));
                self.view_error = Some(message);
            }
        }
    }

    fn status_widget(&self, id: &str) -> Option<StatusWidgetState> {
        let widgets: Table = self.table().ok()?.get("status_widgets").ok()?;
        for widget in widgets.sequence_values::<Table>().flatten() {
            if widget.get::<String>("id").ok()? != id {
                continue;
            }
            let text = match widget.get::<Value>("text").ok()? {
                Value::Function(text) => self
                    .budget
                    .run(VIEW_BUDGET, || text.call::<String>(()))
                    .unwrap_or_else(|error| format!("error: {error}")),
                Value::String(text) => text.to_string_lossy(),
                _ => return None,
            };
            let tooltip = widget.get::<Option<String>>("tooltip").ok().flatten();
            return Some(StatusWidgetState { text, tooltip });
        }
        None
    }

    fn poll(&mut self, host: &mut dyn PluginHost) {
        if let Err(error) = self.call_hook("poll", host) {
            eprintln!("[{}] poll failed: {error}", self.manifest.id);
        }
    }
}

#[cfg(test)]
mod tests;
