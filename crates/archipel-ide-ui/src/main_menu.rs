//! The main menu bar of the window. The menus are the model of `archipel-ide-view`
//! (`main_menu`, the same one the terminal draws: reference structure, what is hidden or limited,
//! appended commands, plugin items, enabled state); this file draws them with egui and sends the
//! chosen row back through the view crate's `dispatch`.

use crate::{IdeApp, icons, theme};
use archipel_kit_model::{MenuEntry, Notation};
use eframe::egui::{self, RichText};

pub fn show(app: &mut IdeApp, ui: &mut egui::Ui) -> bool {
    if !app.menu_bar_open {
        if icons::toolbar_button(ui, icons::Icon::MainMenu, "Main Menu").clicked() {
            app.menu_bar_open = true;
            app.menu_bar_popup_seen = false;
        }
        return false;
    }
    let menu = app.with_view(archipel_ide_view::main_menu::main_menu);
    let mut chosen: Option<String> = None;
    let height = ui.available_height();
    let bar = ui
        .allocate_ui(egui::vec2(ui.available_width().min(720.0), height), |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                let mut titles = Vec::new();
                for header in &menu.headers {
                    let title = ui.menu_button(&header.label, |ui| {
                        scrolled(ui, |ui| {
                            if let Some(id) = render(ui, &header.entries) {
                                chosen = Some(id);
                            }
                        });
                    });
                    titles.push(title.response);
                }
                switch_on_hover(ui.ctx(), &titles);
            })
        })
        .response;
    let ctx = ui.ctx().clone();
    if let Some(id) = chosen {
        let effects = app.with_view(|cx| archipel_ide_view::dispatch::menu_row(cx, &id));
        app.apply_view_effects(effects, &ctx);
    }
    // Collapse after a menu closes, on a click elsewhere, or on Escape.
    let popup_open = egui::Popup::is_any_open(&ctx);
    let clicked_outside = ctx.input(|input| {
        input.pointer.any_click()
            && input
                .pointer
                .interact_pos()
                .is_some_and(|pos| !bar.rect.contains(pos))
    });
    let escape = ctx.input(|input| input.key_pressed(egui::Key::Escape));
    if popup_open {
        app.menu_bar_popup_seen = true;
    } else if app.menu_bar_popup_seen || clicked_outside || escape {
        app.menu_bar_open = false;
    }
    true
}

/// While one menu of the bar is open, hovering another title opens that one instead.
fn switch_on_hover(ctx: &egui::Context, titles: &[egui::Response]) {
    let popup = |response: &egui::Response| egui::Popup::default_response_id(response);
    let open = titles
        .iter()
        .any(|title| egui::Popup::is_id_open(ctx, popup(title)));
    if !open {
        return;
    }
    if let Some(hovered) = titles
        .iter()
        .find(|title| title.hovered() && !egui::Popup::is_id_open(ctx, popup(title)))
    {
        egui::Popup::close_all(ctx);
        egui::Popup::open_id(ctx, popup(hovered));
    }
}

/// Long menus scroll instead of running off the screen.
fn scrolled(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    ui.set_min_width(260.0);
    let max_height = (ui.ctx().content_rect().height() - 80.0).max(200.0);
    egui::ScrollArea::vertical()
        .max_height(max_height)
        .min_scrolled_height(max_height)
        .show(ui, add);
}

/// Draws `entries`; the id of the row the user chose, if any.
fn render(ui: &mut egui::Ui, entries: &[MenuEntry]) -> Option<String> {
    let mut chosen = None;
    for entry in entries {
        match entry {
            MenuEntry::Item(item) => {
                if let Some(sub) = &item.submenu {
                    ui.menu_button(&item.label, |ui| {
                        scrolled(ui, |ui| {
                            if let Some(id) = render(ui, sub) {
                                chosen = Some(id);
                            }
                        })
                    });
                    continue;
                }
                let shortcut = item
                    .shortcut
                    .as_ref()
                    .map(|shortcut| shortcut.label(Notation::Text))
                    .unwrap_or_default();
                let button = egui::Button::new(&item.label).shortcut_text(shortcut);
                let response = ui.add_enabled(item.enabled, button);
                if !item.enabled {
                    if let Some(reason) = &item.disabled_reason {
                        response.on_disabled_hover_text(reason);
                    }
                } else if response.clicked() {
                    chosen = Some(item.id.clone());
                    ui.close();
                }
            }
            MenuEntry::Separator { title } => {
                ui.separator();
                if let Some(title) = title {
                    ui.label(
                        RichText::new(title)
                            .size(theme::UI_FONT_SIZE - 2.0)
                            .color(theme::TEXT_INFO),
                    );
                }
            }
        }
    }
    chosen
}
