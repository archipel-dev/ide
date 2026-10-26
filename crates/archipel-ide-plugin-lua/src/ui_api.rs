//! Immediate-mode panel API for Lua views. The view function receives a
//! `ui` table whose functions draw into the current egui `Ui` this frame:
//!
//! ```lua
//! function views.main(ctx, ui)
//!   ui.heading("Hello")
//!   state.name = ui.text_input("name", state.name, "Your name")
//!   if ui.button("Greet") then ctx.status("Hi " .. state.name) end
//!   ui.horizontal(function(row) row.label("a"); row.label("b") end)
//! end
//! ```

use archipel_ide_plugin_api::ViewNode;
use egui::{RichText, Ui};
use mlua::{Function, Lua, LuaSerdeExt, Scope, Table, Value};
use std::cell::RefCell;

fn borrow_error() -> mlua::Error {
    mlua::Error::runtime(
        "this ui is not active; inside a layout callback use the ui passed to the callback",
    )
}

/// Builds the `ui` table for `ui`; `scope_id` keeps widget ids unique.
pub fn ui_table<'scope, 'env, 'u>(
    lua: &Lua,
    scope: &'scope Scope<'scope, 'env>,
    ui: &'env RefCell<&'u mut Ui>,
    scope_id: &'env str,
) -> mlua::Result<Table>
where
    'u: 'env,
{
    let table = lua.create_table()?;
    let with = move |f: &mut dyn FnMut(&mut Ui)| -> mlua::Result<()> {
        let mut guard = ui.try_borrow_mut().map_err(|_| borrow_error())?;
        f(&mut guard);
        Ok(())
    };
    let id = move |name: &str| egui::Id::new(("lua_view", scope_id, name.to_owned()));

    table.set(
        "heading",
        scope.create_function(move |_, text: String| {
            with(&mut |ui| {
                ui.label(RichText::new(&text).strong().size(15.0));
            })
        })?,
    )?;
    table.set(
        "label",
        scope.create_function(move |_, text: String| {
            with(&mut |ui| {
                ui.label(&text);
            })
        })?,
    )?;
    table.set(
        "muted",
        scope.create_function(move |_, text: String| {
            with(&mut |ui| {
                let color = archipel_ide_plugin_ui::secondary_text(ui);
                ui.label(RichText::new(&text).color(color));
            })
        })?,
    )?;
    table.set(
        "section",
        scope.create_function(move |_, text: String| {
            with(&mut |ui| archipel_ide_plugin_ui::section(ui, &text))
        })?,
    )?;
    table.set(
        "property",
        scope.create_function(move |_, (key, value): (String, String)| {
            with(&mut |ui| archipel_ide_plugin_ui::property(ui, &key, &value))
        })?,
    )?;
    table.set(
        "separator",
        scope.create_function(move |_, ()| {
            with(&mut |ui| {
                ui.separator();
            })
        })?,
    )?;
    table.set(
        "space",
        scope.create_function(move |_, amount: Option<f32>| {
            with(&mut |ui| ui.add_space(amount.unwrap_or(8.0).clamp(0.0, 200.0)))
        })?,
    )?;
    table.set(
        "code",
        scope.create_function(move |_, text: String| {
            with(&mut |ui| {
                let mut text = text.as_str();
                ui.add(
                    egui::TextEdit::multiline(&mut text)
                        .code_editor()
                        .desired_width(f32::INFINITY),
                );
            })
        })?,
    )?;
    table.set(
        "empty_state",
        scope.create_function(move |_, (title, hint): (String, Option<String>)| {
            with(&mut |ui| {
                archipel_ide_plugin_ui::empty_state(ui, &title, hint.as_deref().unwrap_or_default())
            })
        })?,
    )?;
    table.set(
        "button",
        scope.create_function(move |_, label: String| {
            let mut clicked = false;
            with(&mut |ui| clicked = ui.button(&label).clicked())?;
            Ok(clicked)
        })?,
    )?;
    table.set(
        "selectable",
        scope.create_function(move |_, (label, selected): (String, Option<bool>)| {
            let mut clicked = false;
            with(&mut |ui| {
                clicked = ui
                    .selectable_label(selected.unwrap_or(false), &label)
                    .clicked()
            })?;
            Ok(clicked)
        })?,
    )?;
    table.set(
        "checkbox",
        scope.create_function(move |_, (label, value): (String, Option<bool>)| {
            let mut value = value.unwrap_or(false);
            with(&mut |ui| {
                ui.checkbox(&mut value, label.as_str());
            })?;
            Ok(value)
        })?,
    )?;
    for (name, multiline) in [("text_input", false), ("text_area", true)] {
        table.set(
            name,
            scope.create_function(
                move |_, (key, value, hint): (String, Option<String>, Option<String>)| {
                    let mut text = value.unwrap_or_default();
                    with(&mut |ui| {
                        let edit = if multiline {
                            egui::TextEdit::multiline(&mut text)
                        } else {
                            egui::TextEdit::singleline(&mut text)
                        };
                        ui.add(
                            edit.id(id(&key))
                                .hint_text(hint.as_deref().unwrap_or_default())
                                .desired_width(archipel_ide_plugin_ui::input_width(ui, multiline)),
                        );
                    })?;
                    Ok(text)
                },
            )?,
        )?;
    }
    table.set(
        "choice",
        scope.create_function(
            move |_, (key, selected, options): (String, String, Vec<String>)| {
                let mut current = selected;
                with(&mut |ui| {
                    egui::ComboBox::from_id_salt(id(&key))
                        .selected_text(current.clone())
                        .show_ui(ui, |ui| {
                            for option in &options {
                                if ui.selectable_label(*option == current, option).clicked() {
                                    current = option.clone();
                                }
                            }
                        });
                })?;
                Ok(current)
            },
        )?,
    )?;
    table.set(
        "width",
        scope.create_function(move |_, ()| {
            let mut width = 0.0;
            with(&mut |ui| width = ui.available_width())?;
            Ok(width)
        })?,
    )?;
    // Declarative tree inside an immediate view; returns the events.
    table.set(
        "render",
        scope.create_function(move |lua, node: Value| {
            let node: ViewNode = lua.from_value(node)?;
            let mut events = Vec::new();
            with(&mut |ui| events = archipel_ide_plugin_ui::render_node(ui, scope_id, &node))?;
            lua.to_value(&events)
        })?,
    )?;
    for layout in ["horizontal", "vertical", "group", "scroll"] {
        table.set(
            layout,
            scope.create_function(move |lua, callback: Function| {
                let mut guard = ui.try_borrow_mut().map_err(|_| borrow_error())?;
                let run = |child: &mut Ui| -> mlua::Result<()> {
                    let cell = RefCell::new(child);
                    lua.scope(|inner| {
                        let child_table = ui_table(lua, inner, &cell, scope_id)?;
                        callback.call::<()>(child_table)
                    })
                };
                match layout {
                    "horizontal" => guard.horizontal(run).inner,
                    "vertical" => guard.vertical(run).inner,
                    "group" => guard.group(run).inner,
                    _ => {
                        egui::ScrollArea::vertical()
                            .id_salt(id("scroll"))
                            .auto_shrink([false, true])
                            .show(&mut guard, run)
                            .inner
                    }
                }
            })?,
        )?;
    }
    Ok(table)
}
