//! Context menus built from the reference popup groups (`ProjectViewPopupMenu`,
//! `EditorPopupMenu`, `EditorTabPopupMenu`; see `docs/spec/menus.md`).
//!
//! Like the reference IDE's popups, a context menu shows only the actions available
//! for its target: Archipel IDE's implemented actions, in reference order and grouping.
//! The model is computed before the menu is drawn so the drawing code needs no
//! mutable access to the app; the chosen action runs afterwards.

use crate::IdeApp;
use archipel_kit_model::MenuEntry;
use eframe::egui;

pub use archipel_ide_view::context_menu::Target;

/// A context menu ready to draw.
pub struct Model {
    entries: Vec<MenuEntry>,
}

impl IdeApp {
    /// The rows of the context menu of `target`: the view crate's model (the same one the
    /// terminal draws).
    pub(crate) fn context_menu_model(&self, target: &Target) -> Model {
        Model {
            entries: self.with_view(|cx| archipel_ide_view::context_menu::context_menu(cx, target)),
        }
    }

    /// Runs the action chosen in a context menu.
    pub(crate) fn run_context_action(
        &mut self,
        action: &str,
        target: &Target,
        ctx: &egui::Context,
    ) {
        let effects = self
            .with_view(|cx| archipel_ide_view::context_menu::context_action(cx, action, target));
        self.apply_view_effects(effects, ctx);
    }
}

/// Draws `model` into an open context menu; sets `chosen` when an item is clicked.
pub fn show(ui: &mut egui::Ui, model: &Model, chosen: &mut Option<String>) {
    fit_width(ui, &model.entries);
    render(ui, &model.entries, chosen);
}

/// The shortcut text of a row (empty when it has none).
fn shortcut_text(item: &archipel_kit_model::MenuItem) -> String {
    item.shortcut
        .as_ref()
        .map(|shortcut| shortcut.label(archipel_ide_view::Caps::gui().notation()))
        .unwrap_or_default()
}

/// Text start of a menu row from the menu's left edge (UI-POP-020: icon column, then text).
const TEXT_OFFSET: f32 = 36.0;
/// Least space between the longest label and its shortcut (UI-POP-023).
const SHORTCUT_GAP: f32 = 24.0;
/// Space after the shortcut or the sub-menu arrow.
const RIGHT_PADDING: f32 = 26.0;

/// Makes the menu as wide as its widest row (UI-POP-023) instead of the popup's room.
fn fit_width(ui: &mut egui::Ui, entries: &[MenuEntry]) {
    let font = egui::FontId::proportional(crate::theme::UI_FONT_SIZE);
    let text_width = |text: &str| {
        ui.painter()
            .layout_no_wrap(text.to_owned(), font.clone(), egui::Color32::PLACEHOLDER)
            .size()
            .x
    };
    let width = entries
        .iter()
        .map(|entry| match entry {
            MenuEntry::Item(item) if item.submenu.is_some() => {
                text_width(&item.label) + SHORTCUT_GAP
            }
            MenuEntry::Item(item) => {
                let shortcut = shortcut_text(item);
                text_width(&item.label)
                    + if shortcut.is_empty() {
                        0.0
                    } else {
                        SHORTCUT_GAP + text_width(&shortcut)
                    }
            }
            MenuEntry::Separator { .. } => 0.0,
        })
        .fold(0.0_f32, f32::max)
        + TEXT_OFFSET
        + RIGHT_PADDING;
    let width = width.ceil().max(MIN_MENU_WIDTH);
    ui.set_min_width(width);
    ui.set_max_width(width);
}

/// Narrowest menu.
const MIN_MENU_WIDTH: f32 = 160.0;

/// The icon of an action's row, for the actions that have one in the reference menus.
fn icon_of(id: &str) -> Option<archipel_kit::Glyph> {
    use archipel_kit::Glyph;
    Some(match id {
        "$Copy" => Glyph::Copy,
        "$Paste" => Glyph::Paste,
        "$Delete" | "SafeDelete" => Glyph::Delete,
        "FindInPath" => Glyph::FindInFiles,
        "RenameElement" => Glyph::Edit,
        "Run" | "RunClass" => Glyph::Run,
        "Debug" | "DebugClass" => Glyph::Debug,
        "Synchronize" => Glyph::Refresh,
        _ => return None,
    })
}

fn render(ui: &mut egui::Ui, entries: &[MenuEntry], chosen: &mut Option<String>) {
    for entry in entries {
        match entry {
            MenuEntry::Item(item) => match &item.submenu {
                None => {
                    let mut row = archipel_kit::action_system::ActionMenuItem::new(&item.label)
                        .enabled(item.enabled);
                    if let Some(icon) = icon_of(&item.id) {
                        row = row.icon(icon);
                    }
                    let shortcut = shortcut_text(item);
                    if !shortcut.is_empty() {
                        row = row.shortcut(&shortcut);
                    }
                    if ui.add(row).clicked() {
                        *chosen = Some(item.id.clone());
                        ui.close();
                    }
                }
                Some(children) => {
                    // The kit has no submenu item yet: a kit `MenuItem` with an arrow, opened
                    // by egui's submenu logic, so it lines up with the actions around it.
                    let row = ui.add(
                        archipel_kit::action_system::ActionMenuItem::new(&item.label).shortcut("›"),
                    );
                    egui::containers::menu::SubMenu::new().show(ui, &row, |ui| {
                        fit_width(ui, children);
                        render(ui, children, chosen);
                    });
                }
            },
            MenuEntry::Separator { .. } => {
                ui.add(archipel_kit::ui::popup::ListSeparator::new());
            }
        }
    }
}

/// Labels of the top-level entries (tests and evidence).
#[cfg(test)]
fn labels(entries: &[MenuEntry]) -> Vec<String> {
    entries
        .iter()
        .map(|entry| match entry {
            MenuEntry::Item(item) if item.submenu.is_some() => format!("{} \u{25b8}", item.label),
            MenuEntry::Item(item) => item.label.clone(),
            MenuEntry::Separator { .. } => "---".into(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_window_draws_the_view_crates_menus() {
        let (app, _ctx) = crate::tests::headless_app("tree-menu", "App.java", "class App {}\n");
        let menu = |path: &str, is_dir: bool| {
            let model = app.context_menu_model(&Target::Tree {
                path: path.into(),
                is_dir,
            });
            labels(&model.entries)
        };
        let dir = menu("src", true);
        assert!(
            dir.contains(&"Mark Directory as \u{25b8}".to_owned()),
            "{dir:?}"
        );
        let file = menu("App.java", false);
        assert!(file.contains(&"Run 'App'".to_owned()), "{file:?}");
        let editor = labels(&app.context_menu_model(&Target::Editor).entries);
        assert!(editor.contains(&"Paste".to_owned()), "{editor:?}");
    }
}
