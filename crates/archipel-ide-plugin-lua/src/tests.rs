use super::*;
use archipel_ide_plugin_api::testing::RecordingHost;
use archipel_ide_plugin_api::{MenuLocation, json};
use std::fs;
use std::path::PathBuf;

fn example_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../plugins/hello-lua")
}

fn example() -> LuaPlugin {
    let dir = example_dir();
    let manifest =
        PluginManifest::parse(&fs::read_to_string(dir.join("archipel-plugin.toml")).unwrap())
            .unwrap();
    LuaPlugin::load(manifest, &dir).unwrap()
}

/// A throwaway plugin directory with `main.lua` = `source`.
fn scratch(name: &str, source: &str) -> (PathBuf, PluginManifest) {
    let dir = std::env::temp_dir().join(format!("archipel-ide-lua-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("main.lua"), source).unwrap();
    let manifest = PluginManifest::parse(&format!(
        "id='test.{name}'\nname='T'\nversion='1'\napi_version=1\nkind='lua'\nentry='main.lua'\n"
    ))
    .unwrap();
    (dir, manifest)
}

fn load_error(name: &str, source: &str) -> String {
    let (dir, manifest) = scratch(name, source);
    let error = LuaPlugin::load(manifest, &dir)
        .err()
        .expect("load fails")
        .to_string();
    fs::remove_dir_all(&dir).unwrap();
    error
}

fn render(plugin: &mut LuaPlugin, host: &mut RecordingHost, view: &str, input: egui::RawInput) {
    let ctx = egui::Context::default();
    let context = ViewContext {
        view: view.into(),
        placement: ViewPlacement::App,
    };
    let _ = ctx.run(input, |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            plugin.render_view(&context, &mut ViewFrame::new(ui), host);
        });
    });
}

#[test]
fn example_plugin_declares_its_contributions() {
    let plugin = example();
    let contributions = plugin.contributions();
    assert!(
        contributions.problems().is_empty(),
        "{:?}",
        contributions.problems()
    );
    assert_eq!(contributions.commands.len(), 3);
    assert_eq!(contributions.commands[0].category, "Tools");
    assert_eq!(contributions.menus[0].menu, MenuLocation::Tools);
    assert_eq!(
        contributions.tool_windows[0].icon_text.as_deref(),
        Some("L")
    );
    assert_eq!(contributions.settings_pages[0].fields.len(), 2);
    let say = contributions
        .command_specs()
        .into_iter()
        .find(|s| s.id == "hello_lua.sayHello")
        .unwrap();
    assert_eq!(say.shortcut.as_deref(), Some("⌥⌘L"));
}

#[test]
fn commands_use_ctx_settings_and_report_lua_errors() {
    let mut plugin = example();
    let mut host = RecordingHost::new("dev.archipel.hello-lua");
    plugin.activate(&mut host).unwrap();
    assert_eq!(host.statuses, ["Hello Lua activated"]);
    host.settings.insert("greeting".into(), json!("Hi"));
    host.settings.insert("notify".into(), json!(false));
    let result = plugin
        .execute("hello_lua.greet", json!({"name": "Ada"}), &mut host)
        .unwrap();
    assert_eq!(result["message"], "Hi, Ada!");
    assert!(host.notifications.is_empty());
    let error = plugin
        .execute("hello_lua.greet", json!({}), &mut host)
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("main.lua") && error.contains("expected string field"),
        "{error}"
    );
    assert!(matches!(
        plugin.execute("nope.nope", Json::Null, &mut host),
        Err(CommandError::Unknown(_))
    ));
    plugin
        .execute("hello_lua.openTodos", Json::Null, &mut host)
        .unwrap();
    assert_eq!(host.editor_tabs[0].view, "todos");
    assert_eq!(plugin.status_widget("count").unwrap().text, "Lua × 1");
}

#[test]
fn immediate_and_declarative_views_render() {
    let mut plugin = example();
    let mut host = RecordingHost::new("dev.archipel.hello-lua");
    render(&mut plugin, &mut host, "main", egui::RawInput::default());
    assert_eq!(plugin.view_error, None);
    render(&mut plugin, &mut host, "todos", egui::RawInput::default());
    assert_eq!(plugin.view_error, None);
    render(&mut plugin, &mut host, "missing", egui::RawInput::default());
    assert_eq!(
        plugin.view_error, None,
        "undefined views show a label, not an error"
    );
}

#[test]
fn view_events_reach_on_view_event() {
    let (dir, manifest) = scratch(
        "events",
        r#"
        local clicked = 0
        return {
          views = { main = function(ctx, ui) return { type = "button", id = "go", label = "Go" } end },
          on_view_event = function(ctx, view, event) clicked = clicked + 1; ctx.status(view .. ":" .. event.id) end,
        }
        "#,
    );
    let mut plugin = LuaPlugin::load(manifest, &dir).unwrap();
    let mut host = RecordingHost::new("test.events");
    // Frame 1 lays out; frames 2-3 press and release over the button.
    let ctx = egui::Context::default();
    let context = ViewContext {
        view: "main".into(),
        placement: ViewPlacement::App,
    };
    let mut button = None;
    for frame in 0..3 {
        let mut input = egui::RawInput::default();
        if let Some(pos) = button {
            input.events.push(egui::Event::PointerMoved(pos));
            input.events.push(egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: frame == 1,
                modifiers: egui::Modifiers::NONE,
            });
        }
        let _ = ctx.run(input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let start = ui.cursor().min;
                plugin.render_view(&context, &mut ViewFrame::new(ui), &mut host);
                button.get_or_insert(start + egui::vec2(10.0, 8.0));
            });
        });
    }
    assert_eq!(plugin.view_error, None);
    assert_eq!(host.statuses, ["main:go"]);
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn sandbox_blocks_dangerous_globals_and_bounds_runaway_scripts() {
    for (name, source, needle) in [
        ("io", "return io.open('/etc/passwd')", "io"),
        ("os", "return os.execute('true')", "os"),
        ("load", "return load('return 1')()", "load"),
        ("dofile", "return dofile('/etc/passwd')", "dofile"),
        (
            "require",
            "return require('../secret')",
            "invalid module name",
        ),
        ("loop", "while true do end", "time budget"),
        (
            "memory",
            "local t = {} for i = 1, 1e9 do t[i] = string.rep('x', 1024) end",
            "memory",
        ),
        ("notable", "return 42", "must return the plugin table"),
        ("syntax", "return {", "main.lua"),
        (
            "shape",
            "return { commands = { { label = 'x' } } }",
            "plugin.commands",
        ),
    ] {
        let error = load_error(name, source);
        assert!(error.contains(needle), "{name}: {error}");
    }
}

#[test]
fn archipel_global_and_the_old_jetthing_alias_expose_the_api_version() {
    let (dir, manifest) = scratch(
        "alias",
        "assert(archipel.api_version == jetthing.api_version and archipel == jetthing)\nreturn {}",
    );
    LuaPlugin::load(manifest, &dir).expect("both globals exist");
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn runaway_commands_time_out_without_killing_the_plugin() {
    let (dir, manifest) = scratch(
        "spin",
        r#"return { commands = {
            { id = "t.spin", label = "Spin", run = function() while true do end end },
            { id = "t.ok", label = "Ok", run = function() return 1 end },
        } }"#,
    );
    let mut plugin = LuaPlugin::load(manifest, &dir).unwrap();
    let mut host = RecordingHost::new("test.spin");
    let error = plugin
        .execute("t.spin", Json::Null, &mut host)
        .unwrap_err()
        .to_string();
    assert!(error.contains("time budget"), "{error}");
    assert_eq!(
        plugin.execute("t.ok", Json::Null, &mut host).unwrap(),
        json!(1)
    );
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn loader_registers_lua_plugins_with_the_registry() {
    use archipel_ide_plugins::{PluginLoader, PluginRegistry, PluginRuntime, PluginStatus};
    use std::rc::Rc;
    let root =
        std::env::temp_dir().join(format!("archipel-ide-lua-registry-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    let target = root.join("hello-lua");
    fs::create_dir_all(&target).unwrap();
    for file in ["archipel-plugin.toml", "main.lua", "util.lua"] {
        fs::copy(example_dir().join(file), target.join(file)).unwrap();
    }
    let mut registry = PluginRegistry::default();
    let mut runtime = PluginRuntime::default();
    assert_eq!(LuaLoader.kind(), PluginKind::Lua);
    runtime.add_loader(Rc::new(LuaLoader));
    registry.discover(&mut runtime, &root);
    assert_eq!(
        registry.get("dev.archipel.hello-lua").unwrap().status,
        PluginStatus::Enabled
    );
    runtime
        .ensure_instance(&registry, "dev.archipel.hello-lua")
        .unwrap();
    assert!(runtime.instance("dev.archipel.hello-lua").is_some());
    fs::remove_dir_all(&root).unwrap();
}
