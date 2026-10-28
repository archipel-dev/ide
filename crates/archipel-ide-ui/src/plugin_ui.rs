//! Workbench integration of plugin contributions: right-anchored tool
//! windows, editor tabs, status widgets, menu items, keybindings, the Plugins
//! dialog with plugin settings, notifications and standalone app mode.

use crate::chrome::{self, STATUS_BAR_HEIGHT, STRIPE_WIDTH, TOOLBAR_HEIGHT};
use crate::icons::{self, Icon};
use crate::widgets::{self, chrome_frame, island_frame, tool_window_header};
use crate::{IdeApp, theme};
use archipel_ide_plugins::{
    KeyChord, NotificationLevel, PluginRegistry, PluginStatus, ViewContext, ViewFrame,
    ViewPlacement,
};
use eframe::egui::{self, Color32, Rect, RichText, Sense, vec2};
use serde_json::{Value, json};

/// Renders `context` of `plugin` into `ui`, or a placeholder when the plugin
/// is not active.
fn render_view(app: &mut IdeApp, ui: &mut egui::Ui, plugin: &str, context: ViewContext) {
    let rendered = app.state.with_plugin(plugin, |instance, host| {
        instance.render_view(&context, &mut ViewFrame::new(ui), host)
    });
    if rendered.is_none() {
        ui.centered_and_justified(|ui| {
            ui.label(
                RichText::new(format!("Plugin {plugin} is not active")).color(theme::TEXT_INFO),
            );
        });
    }
}

/// Runs commands queued by plugins and re-registers contributed commands
/// when plugins changed.
pub fn sync(app: &mut IdeApp, ctx: &egui::Context) {
    app.state.poll_plugins();
    if app.state.take_plugin_repaint() {
        ctx.request_repaint_after(std::time::Duration::from_millis(16));
    }
    let generation = app.state.plugin_host_service().registry().generation();
    if generation != app.plugin_generation {
        archipel_ide_core::sync_plugin_commands(
            &mut app.commands,
            &mut app.plugin_commands,
            &mut app.state,
        );
        app.sync_command_actions();
        app.plugin_generation = app.state.plugin_host_service().registry().generation();
    }
    // Commands queued while running these may queue more; bound the loop.
    for _ in 0..8 {
        let queued = app.state.plugin_host_service().take_queued_commands();
        if queued.is_empty() {
            break;
        }
        for (id, params) in queued {
            app.dispatch(&id, params);
        }
    }
}

/// `(egui key, modifiers)` for a contributed key chord.
pub fn egui_chord(chord: &KeyChord) -> Option<(egui::Key, egui::Modifiers)> {
    let key = egui::Key::from_name(&chord.key)?;
    let mut modifiers = egui::Modifiers::NONE;
    if chord.command {
        modifiers |= egui::Modifiers::COMMAND;
    }
    if chord.ctrl {
        modifiers |= egui::Modifiers::CTRL;
    }
    if chord.alt {
        modifiers |= egui::Modifiers::ALT;
    }
    if chord.shift {
        modifiers |= egui::Modifiers::SHIFT;
    }
    Some((key, modifiers))
}

/// Dispatches contributed keybindings of active plugins.
pub fn handle_keybindings(app: &mut IdeApp, ctx: &egui::Context) {
    if app.dialog.is_some() || app.palette.open || app.find_popup.open {
        return;
    }
    let bindings: Vec<(String, egui::Key, egui::Modifiers)> = app
        .state
        .plugin_host_service()
        .registry()
        .entries()
        .iter()
        .filter(|entry| entry.is_active())
        .flat_map(|entry| &entry.contributions.keybindings)
        .filter_map(|binding| {
            let chord = KeyChord::parse(&binding.key).ok()?;
            let (key, modifiers) = egui_chord(&chord)?;
            Some((binding.command.clone(), key, modifiers))
        })
        .collect();
    for (command, key, modifiers) in bindings {
        let hit = ctx.input_mut(|input| {
            let before = input.events.len();
            input.events.retain(|event| {
                !matches!(
                    event,
                    egui::Event::Key { key: k, modifiers: m, pressed: true, .. }
                        if *k == key && m.matches_exact(modifiers)
                )
            });
            input.events.len() != before
        });
        if hit {
            app.dispatch(&command, Value::Null);
            if modifiers.alt || modifiers.ctrl || modifiers.command {
                ctx.input_mut(|input| {
                    input
                        .events
                        .retain(|event| !matches!(event, egui::Event::Text(_)))
                });
            }
        }
    }
}

/// Contributed tool windows as `(plugin, id, title, stripe text)`.
fn tool_windows(app: &IdeApp) -> Vec<(String, String, String, String)> {
    app.state
        .plugin_host_service()
        .registry()
        .entries()
        .iter()
        .filter(|entry| entry.is_active())
        .flat_map(|entry| {
            entry.contributions.tool_windows.iter().map(|window| {
                let icon = window.icon_text.clone().unwrap_or_else(|| {
                    window
                        .title
                        .chars()
                        .take(1)
                        .collect::<String>()
                        .to_uppercase()
                });
                (
                    entry.id().to_owned(),
                    window.id.clone(),
                    window.title.clone(),
                    icon,
                )
            })
        })
        .collect()
}

/// Right tool window stripe and the visible plugin tool window. Every
/// contributed tool window docks here in API 1.
pub fn right_side(app: &mut IdeApp, ctx: &egui::Context) {
    let windows = tool_windows(app);
    if windows.is_empty() {
        chrome::right_stripe(app, ctx);
        return;
    }
    let built_in = chrome::right_stripe_windows(app);
    egui::SidePanel::right("right_stripe")
        .exact_width(STRIPE_WIDTH)
        .resizable(false)
        .show_separator_line(false)
        .frame(chrome_frame(ctx))
        .show(ctx, |ui| {
            ui.add_space(6.0);
            ui.vertical_centered(|ui| {
                ui.spacing_mut().item_spacing.y = chrome::stripe_gap();
                for (window, icon) in built_in {
                    chrome::stripe_button(app, ui, window, icon);
                }
                chrome::stripe_button(app, ui, archipel_ide_core::ToolWindow::AiChat, Icon::Ai);
                for (plugin, id, title, icon) in &windows {
                    let selected = app.state.plugin_host_service().tool_window().as_ref()
                        == Some(&(plugin.clone(), id.clone()));
                    let (rect, response) = ui.allocate_exact_size(vec2(32.0, 32.0), Sense::click());
                    let fill = if selected {
                        *theme::ACCENT
                    } else if response.hovered() {
                        *theme::HOVER
                    } else {
                        Color32::TRANSPARENT
                    };
                    ui.painter().rect_filled(rect, 6.0, fill);
                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        icon,
                        theme::semibold(12.0),
                        if selected {
                            Color32::WHITE
                        } else {
                            *icons::ICON
                        },
                    );
                    if response.on_hover_text(title).clicked() {
                        app.dispatch(
                            "plugin.toolwindow.toggle",
                            json!({"plugin": plugin, "id": id}),
                        );
                    }
                }
            });
        });
    let Some((plugin, id)) = app.state.plugin_host_service().tool_window().clone() else {
        return;
    };
    let Some((_, _, title, _)) = windows
        .iter()
        .find(|(p, w, _, _)| *p == plugin && *w == id)
        .cloned()
    else {
        return;
    };
    let view = app
        .state
        .plugin_host_service()
        .registry()
        .get(&plugin)
        .and_then(|entry| entry.contributions.tool_windows.iter().find(|w| w.id == id))
        .map(|window| window.view.clone())
        .unwrap_or_default();
    egui::SidePanel::right("plugin_tool_window")
        .default_width(320.0)
        .min_width(180.0)
        .resizable(true)
        .show_separator_line(false)
        .frame(island_frame(ctx))
        .show(ctx, |ui| {
            if tool_window_header(ui, &title, |_| {}) {
                app.dispatch(
                    "plugin.toolwindow.toggle",
                    json!({"plugin": plugin, "id": id}),
                );
            }
            egui::Frame::new()
                .inner_margin(egui::Margin::symmetric(10, 4))
                .show(ui, |ui| {
                    render_view(
                        app,
                        ui,
                        &plugin,
                        ViewContext {
                            view,
                            placement: ViewPlacement::ToolWindow { id: id.clone() },
                        },
                    );
                });
        });
}

/// Plugin tabs after the document tabs in the editor tab bar. Returns the
/// x position after the last tab.
pub fn editor_tabs(app: &mut IdeApp, ui: &mut egui::Ui, bar: Rect, mut x: f32) -> f32 {
    let font = egui::FontId::proportional(theme::UI_FONT_SIZE);
    let mut activate = None;
    let mut close = None;
    for index in 0..app.state.plugin_host_service().tab_count() {
        let title = app
            .state
            .plugin_host_service()
            .tab_title(index)
            .unwrap_or_default();
        let active = app.state.plugin_host_service().active_tab() == Some(index);
        let color = if active {
            *theme::TEXT_BRIGHT
        } else {
            *theme::TEXT_SECONDARY
        };
        let galley = ui
            .painter()
            .layout_no_wrap(title.clone(), font.clone(), color);
        let width = 10.0 + galley.size().x + 6.0 + 16.0 + 8.0;
        let rect = Rect::from_min_size(egui::pos2(x, bar.top() + 5.0), vec2(width, 28.0));
        x += width + 2.0;
        if rect.left() > bar.right() {
            break;
        }
        let response = ui.interact(rect, egui::Id::new(("plugin_tab", index)), Sense::click());
        let fill = if active {
            *theme::TAB_SELECTED
        } else if response.hovered() {
            *theme::HOVER
        } else {
            Color32::TRANSPARENT
        };
        ui.painter().rect_filled(rect, 6.0, fill);
        if active {
            ui.painter().rect_stroke(
                rect,
                6.0,
                egui::Stroke::new(1.0_f32, theme::TAB_SELECTED_BORDER),
                egui::StrokeKind::Inside,
            );
        }
        let text_pos = rect.left_center() + vec2(10.0, -galley.size().y / 2.0);
        ui.painter().galley(text_pos, galley, color);
        let close_rect = Rect::from_min_size(
            egui::pos2(rect.right() - 24.0, rect.center().y - 8.0),
            vec2(16.0, 16.0),
        );
        let on_close = response
            .hover_pos()
            .is_some_and(|pos| close_rect.expand(2.0).contains(pos));
        if response.hovered() || active {
            if on_close {
                ui.painter()
                    .rect_filled(close_rect.expand(2.0), 4.0, theme::HOVER);
            }
            icons::paint(
                ui.painter(),
                close_rect.shrink(2.0),
                Icon::Close,
                *theme::TEXT_SECONDARY,
            );
        }
        if response.clicked() {
            if on_close {
                close = Some(index);
            } else {
                activate = Some(index);
            }
        }
        if response.middle_clicked() {
            close = Some(index);
        }
    }
    if let Some(index) = close {
        app.dispatch("plugin.tab.close", json!({"index": index}));
    } else if let Some(index) = activate {
        app.dispatch("plugin.tab.activate", json!({"index": index}));
    }
    x
}

/// Body of the active plugin tab (below the tab bar).
pub fn active_tab_body(app: &mut IdeApp, ui: &mut egui::Ui) {
    let Some(tab) = app.state.active_plugin_tab() else {
        return;
    };
    egui::Frame::new()
        .inner_margin(egui::Margin::symmetric(12, 8))
        .show(ui, |ui| {
            ui.set_min_size(ui.available_size());
            render_view(app, ui, &tab.plugin, tab.context());
        });
}

/// Contributed status bar widgets (call inside a right-to-left layout).
/// `only` restricts them to one plugin (standalone app mode).
pub fn status_widgets(
    app: &mut IdeApp,
    ui: &mut egui::Ui,
    font: &egui::FontId,
    only: Option<&str>,
) {
    for widget in app
        .state
        .plugin_status_widgets()
        .into_iter()
        .filter(|widget| only.is_none_or(|plugin| widget.plugin == plugin))
    {
        let mut response = widgets::flat_widget(
            ui,
            None,
            &widget.state.text,
            font.clone(),
            *theme::TEXT_SECONDARY,
            false,
            22.0,
        );
        if let Some(tooltip) = &widget.state.tooltip {
            response = response.on_hover_text(tooltip);
        }
        if response.clicked()
            && let Some(command) = &widget.command
        {
            app.dispatch(command, Value::Null);
        }
    }
}

/// Plugin notification balloons, newest at the bottom.
pub fn notifications(app: &mut IdeApp, ctx: &egui::Context) {
    let notifications = app.state.notifications();
    if notifications.plugin_notification_count() == 0 {
        return;
    }
    let screen = ctx.content_rect();
    let width: f32 = 340.0;
    let mut dismiss = None;
    egui::Area::new(egui::Id::new("plugin_notifications"))
        .order(crate::notifications::BALLOON_ORDER)
        .anchor(
            egui::Align2::RIGHT_BOTTOM,
            vec2(-16.0, -(STATUS_BAR_HEIGHT + 12.0)),
        )
        .show(ctx, |ui| {
            ui.set_max_width(width.min(screen.width() - 32.0));
            for (index, plugin, notification) in &notifications.recent_plugin_notifications(3) {
                let index = *index;
                egui::Frame::new()
                    .fill(*theme::NOTIFICATION)
                    .stroke(egui::Stroke::new(1.0_f32, theme::POPUP_BORDER))
                    .corner_radius(8)
                    .inner_margin(egui::Margin::same(12))
                    .show(ui, |ui| {
                        ui.set_width(width - 24.0);
                        ui.horizontal(|ui| {
                            match notification.level {
                                NotificationLevel::Info => {
                                    let (rect, _) =
                                        ui.allocate_exact_size(vec2(16.0, 16.0), Sense::hover());
                                    ui.painter()
                                        .circle_filled(rect.center(), 4.0, theme::MODIFIED);
                                }
                                NotificationLevel::Warning => {
                                    icons::show(ui, Icon::Warning, 16.0, *theme::WARNING);
                                }
                                NotificationLevel::Error => {
                                    icons::show(ui, Icon::Error, 16.0, *theme::ERROR);
                                }
                            }
                            ui.label(
                                RichText::new(&notification.title)
                                    .font(theme::semibold(theme::UI_FONT_SIZE))
                                    .color(theme::TEXT_BRIGHT),
                            );
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if icons::button(ui, Icon::Close, "Dismiss").clicked() {
                                        dismiss = Some(index);
                                    }
                                },
                            );
                        });
                        if !notification.body.is_empty() {
                            ui.label(RichText::new(&notification.body).color(theme::TEXT));
                        }
                        ui.label(RichText::new(plugin).small().color(theme::TEXT_INFO));
                    });
                ui.add_space(6.0);
            }
        });
    if let Some(index) = dismiss {
        notifications.dismiss_plugin_notification(index);
    }
}

/// Why `plugin` cannot run in standalone app mode, with the alternatives.
pub fn app_mode_error(registry: &PluginRegistry, plugin: &str) -> Result<(), String> {
    let apps: Vec<&str> = registry
        .entries()
        .iter()
        .filter(|entry| entry.manifest.app.is_some())
        .map(|entry| entry.id())
        .collect();
    let alternatives = if apps.is_empty() {
        "none".to_owned()
    } else {
        apps.join(", ")
    };
    let entry = registry.get(plugin).ok_or_else(|| {
        format!("unknown plugin `{plugin}`; plugins with an app view: {alternatives}")
    })?;
    if entry.manifest.app.is_none() {
        return Err(format!(
            "plugin `{plugin}` has no [app] view in its manifest; plugins with an app view: {alternatives}"
        ));
    }
    if !entry.is_active() {
        let reason = match &entry.status {
            PluginStatus::Failed(reason) | PluginStatus::Unsupported(reason) => reason.clone(),
            other => other.label().to_owned(),
        };
        return Err(format!(
            "plugin `{plugin}` is not active ({reason}); enable it in File ▸ Plugins… or check its error there"
        ));
    }
    Ok(())
}

/// Standalone app mode: the plugin's app view with minimal chrome.
pub fn app_window(app: &mut IdeApp, ctx: &egui::Context, plugin: &str) {
    let manifest = app
        .state
        .plugin_host_service()
        .registry()
        .get(plugin)
        .map(|entry| entry.manifest.clone());
    let title = manifest
        .as_ref()
        .map(|manifest| manifest.app_title())
        .unwrap_or_else(|| plugin.to_owned());
    let view = manifest
        .and_then(|manifest| manifest.app)
        .map(|app| app.view)
        .unwrap_or_default();
    egui::TopBottomPanel::top("app_toolbar")
        .exact_height(TOOLBAR_HEIGHT)
        .show_separator_line(false)
        .frame(chrome_frame(ctx))
        .show(ctx, |ui| {
            let bar = ui.max_rect();
            let drag = ui.interact(
                bar,
                egui::Id::new("app_toolbar_drag"),
                Sense::click_and_drag(),
            );
            if drag.drag_started() {
                ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
            }
            if drag.double_clicked() {
                let maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
                ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
            }
            let mut content = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(bar.shrink2(vec2(12.0, 0.0)))
                    .layout(egui::Layout::left_to_right(egui::Align::Center)),
            );
            content.label(
                RichText::new(&title)
                    .font(theme::semibold(theme::UI_FONT_SIZE))
                    .color(theme::TEXT_BRIGHT),
            );
            content.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if chrome::custom_title_bar() {
                    chrome::window_controls(ui);
                }
            });
        });
    egui::TopBottomPanel::bottom("app_status_bar")
        .exact_height(STATUS_BAR_HEIGHT)
        .show_separator_line(false)
        .frame(chrome_frame(ctx).inner_margin(egui::Margin::symmetric(8, 0)))
        .show(ctx, |ui| {
            ui.horizontal_centered(|ui| {
                let font = egui::FontId::proportional(theme::UI_FONT_SIZE - 1.0);
                ui.label(
                    RichText::new(app.state.status())
                        .font(font.clone())
                        .color(theme::TEXT_INFO),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    status_widgets(app, ui, &font, Some(plugin));
                });
            });
        });
    egui::CentralPanel::default()
        .frame(island_frame(ctx))
        .show(ctx, |ui| {
            // Editor tabs opened by the app share the window with its main view.
            if app.state.plugin_host_service().tab_count() > 0 {
                app_tab_bar(app, ui, &title);
            }
            if app.state.plugin_host_service().active_tab().is_some() {
                active_tab_body(app, ui);
                return;
            }
            egui::Frame::new()
                .inner_margin(egui::Margin::same(12))
                .show(ui, |ui| {
                    ui.set_min_size(ui.available_size());
                    render_view(
                        app,
                        ui,
                        plugin,
                        ViewContext {
                            view,
                            placement: ViewPlacement::App,
                        },
                    );
                });
        });
}

/// Tab bar of standalone mode: the app's main view, then its editor tabs.
fn app_tab_bar(app: &mut IdeApp, ui: &mut egui::Ui, title: &str) {
    let (bar, _) = ui.allocate_exact_size(vec2(ui.available_width(), 38.0), Sense::hover());
    let font = egui::FontId::proportional(theme::UI_FONT_SIZE);
    let active = app.state.plugin_host_service().active_tab().is_none();
    let color = if active {
        theme::TEXT_BRIGHT
    } else {
        theme::TEXT_SECONDARY
    };
    let galley = ui.painter().layout_no_wrap(title.to_owned(), font, *color);
    let rect = Rect::from_min_size(
        egui::pos2(bar.left() + 6.0, bar.top() + 5.0),
        vec2(galley.size().x + 20.0, 28.0),
    );
    let response = ui.interact(rect, egui::Id::new("app_main_tab"), Sense::click());
    if active {
        ui.painter()
            .rect_filled(rect, 6.0, Color32::from_rgb(0x23, 0x35, 0x58));
    } else if response.hovered() {
        ui.painter().rect_filled(rect, 6.0, theme::HOVER);
    }
    ui.painter().galley(
        rect.left_center() + vec2(10.0, -galley.size().y / 2.0),
        galley,
        *color,
    );
    if response.clicked() {
        app.dispatch("plugin.tab.activate", json!({}));
    }
    editor_tabs(app, ui, bar, rect.right() + 2.0);
}

#[cfg(test)]
mod tests {
    use super::*;
    use archipel_ide_plugins::{Plugin, PluginManifest};
    use std::rc::Rc;

    struct App(PluginManifest);

    impl Plugin for App {
        fn manifest(&self) -> &PluginManifest {
            &self.0
        }
    }

    fn app_plugin(id: &'static str, app: bool) -> archipel_ide_plugins::PluginFactory {
        Rc::new(move || {
            let app_table = if app { "[app]\nview='main'\n" } else { "" };
            Box::new(App(PluginManifest::parse(&format!(
                "id='{id}'\nname='A'\nversion='1'\napi_version=1\nkind='rust'\n{app_table}"
            ))
            .unwrap()))
        })
    }

    #[test]
    fn standalone_mode_explains_unusable_plugins() {
        let mut registry = PluginRegistry::default();
        let mut runtime = archipel_ide_plugins::PluginRuntime::default();
        registry
            .add_bundled(&mut runtime, app_plugin("x.app", true))
            .unwrap();
        registry
            .add_bundled(&mut runtime, app_plugin("x.lib", false))
            .unwrap();
        let unknown = app_mode_error(&registry, "x.nope").unwrap_err();
        assert!(
            unknown.contains("unknown plugin") && unknown.contains("x.app"),
            "{unknown}"
        );
        assert!(
            app_mode_error(&registry, "x.lib")
                .unwrap_err()
                .contains("no [app] view")
        );
        // Enabled but not activated yet.
        assert!(
            app_mode_error(&registry, "x.app")
                .unwrap_err()
                .contains("not active")
        );
        registry.set_status("x.app", PluginStatus::Active);
        assert!(app_mode_error(&registry, "x.app").is_ok());
    }

    #[test]
    fn contributed_chords_map_to_egui_keys() {
        let (key, modifiers) = egui_chord(&KeyChord::parse("Mod+Alt+H").unwrap()).unwrap();
        assert_eq!(key, egui::Key::H);
        assert!(modifiers.matches_exact(egui::Modifiers::COMMAND | egui::Modifiers::ALT));
        let (key, _) = egui_chord(&KeyChord::parse("Shift+F7").unwrap()).unwrap();
        assert_eq!(key, egui::Key::F7);
        assert_eq!(
            egui_chord(&KeyChord::parse("Ctrl+1").unwrap()).unwrap().0,
            egui::Key::Num1
        );
    }
}
