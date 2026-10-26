//! Settings › Keymap page: keymap selector (generic setting row), action list with
//! search by name or by shortcut, per-action shortcut editing and the conflict dialog.
//! Edits go through the `keymap.bind` / `keymap.unbind` / `keymap.reset` commands.

use crate::keybindings;
use crate::theme;
use archipel_ide_actions::Catalog;
use archipel_ide_core::keymap_commands::KeymapState;
use archipel_ide_keymap::{KeyStroke, Keymap, KeymapSet, Notation, Shortcut};
use eframe::egui::{self, RichText, Sense, vec2};
use serde_json::{Value, json};

const ROW: f32 = 24.0;

#[derive(Default)]
pub struct KeymapPage {
    query: String,
    by_shortcut: bool,
    selected: Option<String>,
    /// Group headers the user collapsed (ignored while searching).
    collapsed: std::collections::BTreeSet<String>,
    capture: Option<Capture>,
    /// Keymap shown when the selector differs from the active keymap.
    preview: Option<Keymap>,
}

struct Capture {
    action: String,
    strokes: Vec<KeyStroke>,
    second_stroke: bool,
    /// Set when OK was pressed and the shortcut conflicts: ask before binding.
    confirm: Option<Vec<String>>,
}

impl KeymapPage {
    /// While a shortcut is being recorded the dialog must not treat keys as its own.
    pub fn capturing(&self) -> bool {
        self.capture.is_some()
    }

    /// Filters the action list (Find Action's Assign Shortcut).
    pub fn set_query(&mut self, query: String) {
        self.query = query;
        self.by_shortcut = false;
    }
}

/// Draws the action list for keymap `id` (the selector's current value).
pub fn show(
    ui: &mut egui::Ui,
    page: &mut KeymapPage,
    state: &KeymapState,
    id: &str,
    requests: &mut Vec<(String, Value)>,
) {
    let editable = state.keymap.id == id;
    let mut preview = page.preview.take();
    if !editable && preview.as_ref().is_none_or(|preview| preview.id != id) {
        let overrides = state.overrides.get(id).cloned().unwrap_or_default();
        preview = KeymapSet::shared()
            .ok()
            .and_then(|set| set.keymap(id, &overrides).ok());
    }
    let keymap = if editable {
        &state.keymap
    } else {
        match preview.as_ref() {
            Some(preview) => preview,
            None => return,
        }
    };
    list(ui, page, keymap, editable, requests);
    page.preview = preview;
}

fn list(
    ui: &mut egui::Ui,
    page: &mut KeymapPage,
    keymap: &Keymap,
    editable: bool,
    requests: &mut Vec<(String, Value)>,
) {
    let notation = Notation::current();
    let catalog = Catalog::shared();

    ui.horizontal(|ui| {
        let hint = if page.by_shortcut {
            "Type a shortcut, e.g. ctrl+shift+a"
        } else {
            "Search actions"
        };
        crate::widgets::search_field(
            ui,
            egui::Id::new("keymap_search"),
            &mut page.query,
            hint,
            260.0,
        );
        ui.checkbox(&mut page.by_shortcut, "Find by shortcut");
    });
    if !editable {
        ui.label(
            RichText::new("Apply to switch to this keymap before editing its shortcuts.")
                .color(theme::TEXT_INFO),
        );
    }
    ui.add_space(4.0);

    let rows = action_rows(keymap, catalog, &page.query, page.by_shortcut, notation);
    let selected_height = if page.selected.is_some() { 96.0 } else { 0.0 };
    let list_height = (ui.available_height() - selected_height - 24.0).max(120.0);
    ui.label(
        RichText::new(crate::widgets::pluralize(rows.len(), "action", "actions"))
            .color(theme::TEXT_INFO),
    );
    let entries = grouped(&rows, &page.collapsed, !page.query.trim().is_empty());
    egui::Frame::new()
        .stroke(egui::Stroke::new(1.0_f32, theme::BORDER))
        .corner_radius(4)
        .show(ui, |ui| {
            egui::ScrollArea::vertical()
                .id_salt("keymap_actions")
                .max_height(list_height)
                .auto_shrink([false, false])
                .show_rows(ui, ROW, entries.len(), |ui, range| {
                    for entry in &entries[range] {
                        match entry {
                            Entry::Action(row) => action_row(ui, page, row, notation),
                            Entry::Group { name, count, open } => {
                                if group_row(ui, name, *count, *open) {
                                    let name = name.clone();
                                    if !page.collapsed.remove(&name) {
                                        page.collapsed.insert(name);
                                    }
                                }
                            }
                        }
                    }
                });
        });

    if let Some(action) = page.selected.clone() {
        selected_panel(
            ui, page, keymap, catalog, &action, editable, notation, requests,
        );
    }
    if page.capture.is_some() {
        capture_window(ui.ctx(), page, keymap, catalog, notation, requests);
    }
}

/// Group an action is listed under: the top-level main-menu entry that
/// contains it (`File`, `Edit`, …), or `Other` for actions outside the menu.
fn group_of(action: &str) -> &'static str {
    use std::collections::HashMap;
    use std::sync::OnceLock;
    static GROUPS: OnceLock<HashMap<String, String>> = OnceLock::new();
    fn collect(
        items: &[archipel_ide_actions::MenuItem],
        group: &str,
        map: &mut HashMap<String, String>,
    ) {
        for item in items {
            match item {
                archipel_ide_actions::MenuItem::Action { id, .. } => {
                    map.entry(id.clone()).or_insert_with(|| group.to_owned());
                }
                archipel_ide_actions::MenuItem::Submenu { items, .. } => collect(items, group, map),
                archipel_ide_actions::MenuItem::Separator(_) => {}
            }
        }
    }
    let groups = GROUPS.get_or_init(|| {
        let mut map = HashMap::new();
        for item in Catalog::shared_menu("MainMenu").iter() {
            if let archipel_ide_actions::MenuItem::Submenu { label, items, .. } = item {
                collect(items, label, &mut map);
            }
        }
        map
    });
    groups.get(action).map_or(OTHER_GROUP, String::as_str)
}

const OTHER_GROUP: &str = "Other";

/// One line of the grouped tree.
enum Entry<'a> {
    Group {
        name: String,
        count: usize,
        open: bool,
    },
    Action(&'a Row),
}

/// Rows bucketed by group in main-menu order (`Other` last); a collapsed
/// group keeps only its header. Searching expands every group.
fn grouped<'a>(
    rows: &'a [Row],
    collapsed: &std::collections::BTreeSet<String>,
    searching: bool,
) -> Vec<Entry<'a>> {
    let mut order: Vec<String> = Vec::new();
    for item in Catalog::shared_menu("MainMenu").iter() {
        if let archipel_ide_actions::MenuItem::Submenu { label, .. } = item {
            order.push(label.clone());
        }
    }
    order.push(OTHER_GROUP.to_owned());
    let mut entries = Vec::new();
    for name in order {
        let members: Vec<&Row> = rows
            .iter()
            .filter(|row| row.group == name.as_str())
            .collect();
        if members.is_empty() {
            continue;
        }
        let open = searching || !collapsed.contains(&name);
        entries.push(Entry::Group {
            name,
            count: members.len(),
            open,
        });
        if open {
            entries.extend(members.into_iter().map(Entry::Action));
        }
    }
    entries
}

struct Row {
    group: &'static str,
    action: String,
    label: String,
    shortcuts: String,
    implemented: bool,
}

fn action_rows(
    keymap: &Keymap,
    catalog: &Catalog,
    query: &str,
    by_shortcut: bool,
    notation: Notation,
) -> Vec<Row> {
    let query = query.trim().to_lowercase();
    let mut ids: Vec<&str> = keymap.bindings().keys().map(String::as_str).collect();
    // Implemented actions are listed even when the keymap leaves them unbound.
    for action in catalog.actions() {
        if keybindings::is_implemented(&action.id) && !keymap.bindings().contains_key(&action.id) {
            ids.push(&action.id);
        }
    }
    let by_shortcut_hits = if by_shortcut && !query.is_empty() {
        Some(keymap.find_by_shortcut_text(&query, notation))
    } else {
        None
    };
    let mut rows: Vec<Row> = ids
        .into_iter()
        .filter(|id| catalog.get(id).is_none_or(|action| !action.internal))
        .filter_map(|id| {
            let label = catalog.label(id);
            let keep = match &by_shortcut_hits {
                Some(hits) => hits.contains(&id),
                None => {
                    query.is_empty()
                        || label.to_lowercase().contains(&query)
                        || id.to_lowercase().contains(&query)
                }
            };
            keep.then(|| Row {
                group: group_of(id),
                action: id.to_owned(),
                shortcuts: keymap
                    .shortcuts(id)
                    .iter()
                    .map(|shortcut| shortcut.label(notation))
                    .collect::<Vec<_>>()
                    .join("   "),
                implemented: keybindings::is_implemented(id),
                label,
            })
        })
        .collect();
    rows.sort_by(|a, b| {
        b.implemented
            .cmp(&a.implemented)
            .then_with(|| a.label.to_lowercase().cmp(&b.label.to_lowercase()))
            .then_with(|| a.action.cmp(&b.action))
    });
    rows
}

/// Group header row; returns whether it was clicked (toggles collapse).
fn group_row(ui: &mut egui::Ui, name: &str, count: usize, open: bool) -> bool {
    let (rect, response) = crate::widgets::list_row(ui, false);
    crate::icons::paint(
        ui.painter(),
        egui::Rect::from_center_size(
            egui::pos2(rect.left() + 14.0, rect.center().y),
            vec2(12.0, 12.0),
        ),
        if open {
            crate::icons::Icon::ChevronDown
        } else {
            crate::icons::Icon::ChevronRight
        },
        *theme::TEXT_SECONDARY,
    );
    crate::widgets::row_text(
        ui,
        rect,
        rect.left() + 26.0,
        &format!("{name}  ({count})"),
        theme::semibold(theme::UI_FONT_SIZE),
        *theme::TEXT_BRIGHT,
    );
    response.clicked()
}

fn action_row(ui: &mut egui::Ui, page: &mut KeymapPage, row: &Row, notation: Notation) {
    let _ = notation;
    let selected = page.selected.as_deref() == Some(row.action.as_str());
    let (rect, response) = crate::widgets::list_row(ui, selected);
    let color = if row.implemented {
        *theme::TEXT
    } else {
        *theme::TEXT_INFO
    };
    let x = crate::widgets::row_text(
        ui,
        rect,
        rect.left() + 28.0,
        &row.label,
        egui::FontId::proportional(theme::UI_FONT_SIZE),
        color,
    );
    if !row.implemented {
        crate::widgets::row_text(
            ui,
            rect,
            x + 8.0,
            "not available in Archipel IDE",
            egui::FontId::proportional(theme::UI_FONT_SIZE - 2.0),
            *theme::TEXT_DISABLED,
        );
    }
    ui.painter().text(
        egui::pos2(rect.right() - 10.0, rect.center().y),
        egui::Align2::RIGHT_CENTER,
        &row.shortcuts,
        egui::FontId::proportional(theme::UI_FONT_SIZE),
        *theme::TEXT_SECONDARY,
    );
    let response = response.on_hover_text(&row.action);
    if response.clicked() {
        page.selected = Some(row.action.clone());
    }
    if response.double_clicked() {
        page.capture = Some(Capture {
            action: row.action.clone(),
            strokes: Vec::new(),
            second_stroke: false,
            confirm: None,
        });
    }
}

#[allow(clippy::too_many_arguments)]
fn selected_panel(
    ui: &mut egui::Ui,
    page: &mut KeymapPage,
    keymap: &Keymap,
    catalog: &Catalog,
    action: &str,
    editable: bool,
    notation: Notation,
    requests: &mut Vec<(String, Value)>,
) {
    ui.add_space(8.0);
    ui.label(
        RichText::new(catalog.label(action))
            .font(theme::semibold(theme::UI_FONT_SIZE))
            .color(theme::TEXT_BRIGHT),
    );
    ui.horizontal_wrapped(|ui| {
        let shortcuts = keymap.shortcuts(action);
        if shortcuts.is_empty() {
            ui.label(RichText::new("No shortcuts").color(theme::TEXT_INFO));
        }
        for shortcut in shortcuts {
            ui.label(shortcut.label(notation));
            if editable
                && crate::icons::button(ui, crate::icons::Icon::Close, "Remove this shortcut")
                    .clicked()
            {
                requests.push((
                    "keymap.unbind".into(),
                    json!({"action": action, "shortcut": shortcut.to_string()}),
                ));
            }
            ui.add_space(8.0);
        }
    });
    ui.horizontal(|ui| {
        if ui
            .add_enabled(editable, egui::Button::new("Add Keyboard Shortcut"))
            .clicked()
        {
            page.capture = Some(Capture {
                action: action.to_owned(),
                strokes: Vec::new(),
                second_stroke: false,
                confirm: None,
            });
        }
        let bundled = KeymapSet::shared()
            .ok()
            .and_then(|set| set.get(&keymap.id))
            .and_then(|bundled| bundled.bindings.get(action))
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        let modified = keymap.shortcuts(action) != bundled;
        if ui
            .add_enabled(editable && modified, egui::Button::new("Reset"))
            .on_hover_text("Restore the keymap's default shortcuts")
            .clicked()
        {
            requests.push(("keymap.reset".into(), json!({"action": action})));
        }
    });
}

fn capture_window(
    ctx: &egui::Context,
    page: &mut KeymapPage,
    keymap: &Keymap,
    catalog: &Catalog,
    notation: Notation,
    requests: &mut Vec<(String, Value)>,
) {
    let Some(capture) = page.capture.as_mut() else {
        return;
    };
    let mut close = false;
    if capture.confirm.is_none() {
        record_keys(ctx, capture, &mut close);
    }
    let shortcut = (!capture.strokes.is_empty()).then(|| Shortcut::Keys(capture.strokes.clone()));
    let mut conflicts = shortcut
        .as_ref()
        .map(|shortcut| keymap.conflicts_with(&capture.action, shortcut))
        .unwrap_or_default();
    // Name the conflicts that matter in Archipel IDE first.
    conflicts.sort_by_key(|action| !keybindings::is_implemented(action));
    egui::Area::new(egui::Id::new("keymap_capture"))
        .order(egui::Order::Tooltip)
        .anchor(egui::Align2::CENTER_CENTER, vec2(0.0, 0.0))
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style())
                .inner_margin(egui::Margin::same(theme::layout::DIALOG_PADDING as i8))
                .show(ui, |ui| {
                    ui.set_width(420.0);
                    if let Some(others) = &capture.confirm {
                        ui.label(
                            RichText::new("Shortcut Conflict")
                                .font(theme::semibold(theme::UI_FONT_SIZE))
                                .color(theme::TEXT_BRIGHT),
                        );
                        ui.add_space(6.0);
                        ui.label("The shortcut is already assigned to other actions.");
                        ui.label("Do you want to remove other assignments?");
                        for other in others.iter().take(6) {
                            ui.label(
                                RichText::new(format!("• {}", catalog.label(other)))
                                    .color(theme::TEXT_SECONDARY),
                            );
                        }
                        if others.len() > 6 {
                            ui.label(
                                RichText::new(format!("… and {} more", others.len() - 6))
                                    .color(theme::TEXT_INFO),
                            );
                        }
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            let bind = |remove: bool| {
                                (
                                    "keymap.bind".to_owned(),
                                    json!({
                                        "action": capture.action,
                                        "shortcut": shortcut.as_ref().map(ToString::to_string),
                                        "removeConflicts": remove,
                                    }),
                                )
                            };
                            if ui.button("Remove").clicked() {
                                requests.push(bind(true));
                                close = true;
                            }
                            if ui.button("Leave").clicked() {
                                requests.push(bind(false));
                                close = true;
                            }
                            if ui.button("Cancel").clicked() {
                                close = true;
                            }
                        });
                        return;
                    }
                    ui.label(
                        RichText::new(format!(
                            "Keyboard Shortcut: {}",
                            catalog.label(&capture.action)
                        ))
                        .font(theme::semibold(theme::UI_FONT_SIZE))
                        .color(theme::TEXT_BRIGHT),
                    );
                    ui.add_space(6.0);
                    let (rect, _) =
                        ui.allocate_exact_size(vec2(ui.available_width(), 30.0), Sense::hover());
                    ui.painter().rect(
                        rect,
                        4.0,
                        theme::FIELD,
                        egui::Stroke::new(1.0_f32, theme::ACCENT),
                        egui::StrokeKind::Inside,
                    );
                    let text = match &shortcut {
                        Some(shortcut) => shortcut.label(notation),
                        None => "Press keys…".to_owned(),
                    };
                    ui.painter().text(
                        rect.left_center() + vec2(8.0, 0.0),
                        egui::Align2::LEFT_CENTER,
                        text,
                        egui::FontId::proportional(theme::UI_FONT_SIZE),
                        *theme::TEXT_BRIGHT,
                    );
                    ui.checkbox(&mut capture.second_stroke, "Second stroke");
                    if !conflicts.is_empty() {
                        let names: Vec<String> = conflicts
                            .iter()
                            .take(3)
                            .map(|id| catalog.label(id))
                            .collect();
                        let more = conflicts.len().saturating_sub(3);
                        let suffix = if more > 0 {
                            format!(" and {more} more")
                        } else {
                            String::new()
                        };
                        ui.label(
                            RichText::new(format!(
                                "Already assigned to: {}{suffix}",
                                names.join(", ")
                            ))
                            .color(theme::WARNING),
                        );
                    }
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui
                            .add_enabled(shortcut.is_some(), egui::Button::new("OK"))
                            .clicked()
                        {
                            if conflicts.is_empty() {
                                requests.push((
                                    "keymap.bind".into(),
                                    json!({
                                        "action": capture.action,
                                        "shortcut": shortcut.as_ref().map(ToString::to_string),
                                    }),
                                ));
                                close = true;
                            } else {
                                capture.confirm = Some(conflicts.clone());
                            }
                        }
                        if ui.button("Clear").clicked() {
                            capture.strokes.clear();
                        }
                        if ui.button("Cancel").clicked() {
                            close = true;
                        }
                    });
                });
        });
    if close {
        page.capture = None;
    }
}

/// Takes this frame's key presses as shortcut strokes (they never reach other widgets).
fn record_keys(ctx: &egui::Context, capture: &mut Capture, close: &mut bool) {
    let mut taken = Vec::new();
    ctx.input_mut(|input| {
        input.events.retain(|event| match event {
            egui::Event::Key {
                key,
                modifiers,
                pressed: true,
                ..
            } => {
                taken.push((*key, *modifiers));
                false
            }
            egui::Event::Text(_) => false,
            _ => true,
        });
    });
    for (key, modifiers) in taken {
        if key == egui::Key::Escape && modifiers.is_none() && capture.strokes.is_empty() {
            *close = true;
            return;
        }
        let Some(stroke) = keybindings::stroke(key, modifiers) else {
            continue;
        };
        let limit = if capture.second_stroke { 2 } else { 1 };
        if capture.strokes.len() >= limit {
            capture.strokes.clear();
        }
        capture.strokes.push(stroke);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(action: &str) -> Row {
        Row {
            group: group_of(action),
            action: action.to_owned(),
            label: action.to_owned(),
            shortcuts: String::new(),
            implemented: true,
        }
    }

    #[test]
    fn rows_are_grouped_and_collapsible() {
        let rows = vec![row("$Undo"), row("no.such.action")];
        let none = std::collections::BTreeSet::new();
        let entries = grouped(&rows, &none, false);
        let groups = entries
            .iter()
            .filter(|entry| matches!(entry, Entry::Group { .. }))
            .count();
        assert_eq!(entries.len(), groups + 2);
        let collapsed = [OTHER_GROUP.to_owned()].into();
        assert_eq!(grouped(&rows, &collapsed, false).len(), entries.len() - 1);
        // Searching expands every group.
        assert_eq!(grouped(&rows, &collapsed, true).len(), entries.len());
    }

    #[test]
    fn rows_search_by_name_and_shortcut() {
        let keymap = KeymapSet::shared()
            .unwrap()
            .keymap("$default", &Default::default())
            .unwrap();
        let catalog = Catalog::shared();
        let all = action_rows(&keymap, catalog, "", false, Notation::Text);
        assert!(all.len() > 500);
        // Implemented actions come first.
        assert!(all[0].implemented);
        let by_name = action_rows(&keymap, catalog, "find action", false, Notation::Text);
        assert!(by_name.iter().any(|row| row.action == "GotoAction"));
        let by_shortcut = action_rows(&keymap, catalog, "ctrl+shift+a", true, Notation::Text);
        assert!(by_shortcut.iter().any(|row| row.action == "GotoAction"));
        assert!(!by_shortcut.iter().any(|row| row.action == "ShowSettings"));
        let settings = all.iter().find(|row| row.action == "ShowSettings").unwrap();
        assert_eq!(settings.label, "Settings…");
        assert!(settings.shortcuts.contains("Ctrl+Alt+S"));
    }
}
