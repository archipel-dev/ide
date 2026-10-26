//! Structure tool window (Alt+7), File Structure popup (Ctrl+F12) and the
//! code part of the navigation-bar breadcrumbs, all from the language
//! analysis' tags-query structure.

use crate::icons::Icon;
use crate::widgets;
use crate::{IdeApp, theme};
use archipel_ide_core::Document;
use archipel_ide_language::{StructureItem, SymbolKind, structure};
use archipel_kit::behaviour::PopupHandler;
use archipel_kit::action_system::ActionMenuItem;
use archipel_kit::ui::{ColoredListCellRenderer, JBList};
use archipel_kit::icons::{Glyph, Symbol, SymbolKind as Kind};
use eframe::egui;
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;

/// Last computed structure and the edit it is waiting out.
#[derive(Clone)]
struct Cached {
    path: PathBuf,
    /// Text revision the items were computed from.
    items_revision: u64,
    items: Arc<Vec<StructureItem>>,
    /// Newest revision seen, and when (egui time) it was first seen.
    seen_revision: u64,
    last_edit: f64,
}

/// While the user types, the previous structure is reused until this long
/// after the latest edit: the tags query walks the whole file, which takes
/// hundreds of milliseconds on very large files, too slow for every keystroke.
const STALE_FOR: f64 = 0.25;

/// Structure of `document`. A newly opened file is computed at once; after an
/// edit the previous items are kept until typing pauses for [`STALE_FOR`].
pub fn items(ctx: &egui::Context, document: &Document) -> Arc<Vec<StructureItem>> {
    let id = egui::Id::new("structure_view_cache");
    let revision = document.text.revision();
    let now = ctx.input(|input| input.time);
    if let Some(mut cached) = ctx
        .data(|data| data.get_temp::<Cached>(id))
        .filter(|cached| cached.path == document.path)
    {
        if cached.items_revision == revision {
            return cached.items;
        }
        if cached.seen_revision != revision {
            cached.seen_revision = revision;
            cached.last_edit = now;
        }
        if now - cached.last_edit < STALE_FOR {
            ctx.request_repaint_after(std::time::Duration::from_secs_f64(STALE_FOR));
            let items = cached.items.clone();
            ctx.data_mut(|data| data.insert_temp(id, cached));
            return items;
        }
    }
    let items = Arc::new(document.analysis.structure());
    ctx.data_mut(|data| {
        data.insert_temp(
            id,
            Cached {
                path: document.path.clone(),
                items_revision: revision,
                items: items.clone(),
                seen_revision: revision,
                last_edit: now,
            },
        )
    });
    items
}

/// Depth-first rows of the structure tree.
pub fn flatten<'a>(
    items: &'a [StructureItem],
    depth: usize,
    out: &mut Vec<(usize, &'a StructureItem)>,
) {
    for item in items {
        out.push((depth, item));
        flatten(&item.children, depth + 1, out);
    }
}

/// The kit icon of a structure item: its code symbol (class, method, field …), or a glyph
/// for kinds that are not code (markup tags, document sections).
pub fn icon(kind: SymbolKind) -> archipel_kit::Icon {
    let symbol = |kind| archipel_kit::Icon::Symbol(Symbol::new(kind));
    match kind {
        SymbolKind::Class => symbol(Kind::Class),
        SymbolKind::Interface => symbol(Kind::Interface),
        SymbolKind::Enum => symbol(Kind::Enum),
        SymbolKind::Function => symbol(Kind::Function),
        SymbolKind::Method => symbol(Kind::Method),
        SymbolKind::Constructor => symbol(Kind::Constructor),
        SymbolKind::Field => symbol(Kind::Field),
        SymbolKind::Constant => symbol(Kind::Constant),
        SymbolKind::Module => symbol(Kind::Module),
        SymbolKind::Type => symbol(Kind::Struct),
        SymbolKind::Macro => symbol(Kind::Function),
        SymbolKind::Key => symbol(Kind::Property),
        SymbolKind::Tag => archipel_kit::Icon::Glyph(Glyph::Code),
        SymbolKind::Heading => archipel_kit::Icon::Glyph(Glyph::FileText),
        SymbolKind::Other => symbol(Kind::Variable),
    }
}

/// Paints one structure row (the File Structure popup); returns its geometry.
/// `expanded` draws a chevron for items with children (the tool window collapses them).
pub fn row(
    ui: &mut egui::Ui,
    depth: usize,
    item: &StructureItem,
    selected: bool,
    expanded: Option<bool>,
) -> widgets::TreeRowOut {
    widgets::tree_row(
        ui,
        widgets::TreeRow {
            depth,
            expanded,
            selected,
            icon: Some((Icon::Kit(icon(item.kind)), *theme::ICON)),
            tone: Some(archipel_kit::Tone::Strong),
            secondary: item
                .detail
                .as_deref()
                .map(|detail| (detail, *theme::TEXT_SECONDARY)),
            ..widgets::TreeRow::new(&item.name)
        },
    )
}

/// Moves the caret to the item's name and focuses the editor.
pub fn navigate(app: &mut IdeApp, ctx: &egui::Context, item: &StructureItem) {
    goto(app, item);
    ctx.memory_mut(|memory| memory.request_focus(crate::editor::editor_id()));
}

/// Moves the caret to the item's name, leaving keyboard focus where it is (arrow keys in
/// the Structure window follow the caret without stealing focus).
fn goto(app: &mut IdeApp, item: &StructureItem) {
    let Some(document) = app
        .state
        .active_document
        .and_then(|i| app.state.documents.get(i))
    else {
        return;
    };
    let line_start = document.text.line_to_char(item.line).unwrap_or(0);
    let column = document
        .text
        .byte_to_char(item.name_range.start)
        .saturating_sub(line_start);
    app.dispatch(
        "editor.gotoLine",
        json!({"line": item.line + 1, "column": column + 1}),
    );
}

/// The innermost item containing the primary caret (to highlight in lists).
pub fn item_at_caret<'a>(
    document: &Document,
    items: &'a [StructureItem],
) -> Option<&'a StructureItem> {
    let byte = document
        .text
        .char_to_byte(document.selections.primary().head);
    structure::path_at(items, byte).last().copied()
}

/// Names of the declarations around the caret, outermost first (breadcrumbs).
pub fn breadcrumbs(ctx: &egui::Context, document: &Document) -> Vec<(SymbolKind, String)> {
    let items = items(ctx, document);
    let byte = document
        .text
        .char_to_byte(document.selections.primary().head);
    structure::path_at(&items, byte)
        .into_iter()
        .map(|item| (item.kind, crumb_label(&item.name)))
        .collect()
}

/// Short breadcrumb label: qualified names keep their last segment
/// (`androidx.…ConstraintLayout` → `ConstraintLayout`), long ones are cut.
fn crumb_label(name: &str) -> String {
    let name = name.lines().next().unwrap_or_default().trim();
    let short = if name.len() > 24 && name.contains('.') && !name.contains(' ') {
        name.rsplit('.').next().unwrap_or(name)
    } else {
        name
    };
    if short.chars().count() > 32 {
        format!("{}…", short.chars().take(31).collect::<String>())
    } else {
        short.to_owned()
    }
}

/// What a Structure row's context menu asked for.
#[derive(Clone, Copy)]
enum Choice {
    Navigate,
    CopyName,
}

/// Structure tool window body: the kit's virtualized list of `ListRow`s, with a context
/// menu on every row. Expanding and collapsing is kept per file.
pub fn tool_window(app: &mut IdeApp, ui: &mut egui::Ui) {
    let Some(document) = app
        .state
        .active_document
        .and_then(|i| app.state.documents.get(i))
    else {
        ui.label(egui::RichText::new("No file open").color(theme::TEXT_SECONDARY));
        return;
    };
    let items = items(ui.ctx(), document);
    if items.is_empty() {
        ui.label(egui::RichText::new("No structure for this file").color(theme::TEXT_SECONDARY));
        return;
    }
    let current = item_at_caret(document, &items).map(|item| item.name_range.clone());
    let mut rows = Vec::new();
    flatten(&items, 0, &mut rows);
    let collapsed_id = egui::Id::new("structure_collapsed").with(&document.path);
    let mut collapsed: std::collections::HashSet<usize> = ui
        .data(|data| data.get_temp(collapsed_id))
        .unwrap_or_default();
    let key = |item: &StructureItem| item.name_range.start;
    // Children of a collapsed node are not visible rows.
    let mut visible: Vec<(usize, &StructureItem)> = Vec::new();
    let mut hidden_below = None;
    for (depth, item) in &rows {
        if let Some(limit) = hidden_below {
            if *depth > limit {
                continue;
            }
            hidden_below = None;
        }
        visible.push((*depth, *item));
        if !item.children.is_empty() && collapsed.contains(&key(item)) {
            hidden_below = Some(*depth);
        }
    }
    // The selection follows the caret; a click or arrow key moves the caret.
    let mut selected = visible
        .iter()
        .position(|(_, item)| current.as_ref() == Some(&item.name_range));
    let chosen = std::cell::Cell::new(None::<(usize, Choice)>);
    let out = JBList::new(("structure", &document.path), visible.len(), &mut selected)
        .name("Structure")
        .height(ui.available_height())
        .show(ui, |index| {
            let (depth, item) = visible[index];
            let mut row = ColoredListCellRenderer::new(&item.name)
                .icon(icon(item.kind))
                .depth(depth)
                .expanded((!item.children.is_empty()).then(|| !collapsed.contains(&key(item))));
            if let Some(detail) = &item.detail {
                row = row.secondary(detail);
            }
            row.with(PopupHandler::new({
                let chosen = &chosen;
                move |ui| {
                    if ui.add(ActionMenuItem::new("Navigate to Source")).clicked() {
                        chosen.set(Some((index, Choice::Navigate)));
                    }
                    if ui.add(ActionMenuItem::new("Copy Name")).clicked() {
                        chosen.set(Some((index, Choice::CopyName)));
                    }
                }
            }))
        });
    let mut toggle = out.toggled;
    if out.response.has_focus()
        && let Some(index) = selected
    {
        // The list reports Up/Down/Home/End itself; Left and Right fold and unfold.
        let (depth, item) = visible[index];
        let (left, right) = ui.input(|input| {
            (
                input.key_pressed(egui::Key::ArrowLeft),
                input.key_pressed(egui::Key::ArrowRight),
            )
        });
        let folded = collapsed.contains(&key(item));
        if !item.children.is_empty() && ((right && folded) || (left && !folded)) {
            toggle = Some(index);
        } else if left && let Some(parent) = visible[..index].iter().rposition(|(d, _)| *d < depth)
        {
            selected = Some(parent);
        }
    }
    if let Some(index) = toggle
        && !collapsed.remove(&key(visible[index].1))
    {
        collapsed.insert(key(visible[index].1));
    }
    let by_key = ui.input(|input| {
        use egui::Key::*;
        [ArrowUp, ArrowDown, ArrowLeft, Home, End, PageUp, PageDown]
            .into_iter()
            .any(|key| input.key_pressed(key))
    });
    let target = |index: usize| (*visible[index].1).clone();
    let navigate_to = match (out.activated, selected) {
        (Some(index), _) => Some((target(index), true)),
        (None, Some(index)) if out.selection_changed && toggle.is_none() => {
            Some((target(index), !by_key))
        }
        _ => None,
    };
    let from_menu = match chosen.get() {
        Some((index, Choice::Navigate)) => Some((target(index), true)),
        Some((index, Choice::CopyName)) => {
            ui.ctx().copy_text(visible[index].1.name.clone());
            None
        }
        None => None,
    };
    ui.data_mut(|data| data.insert_temp(collapsed_id, collapsed));
    if let Some((item, focus_editor)) = from_menu.or(navigate_to) {
        if focus_editor {
            navigate(app, ui.ctx(), &item);
        } else {
            goto(app, &item);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_are_depth_first_and_cache_follows_revisions() {
        let ctx = egui::Context::default();
        let mut document = Document::new(
            "A.java".into(),
            "class A {\n    void f() {}\n    class B {\n        void g() {}\n    }\n}\n",
        );
        let first = items(&ctx, &document);
        let mut rows = Vec::new();
        flatten(&first, 0, &mut rows);
        let names: Vec<_> = rows
            .iter()
            .map(|(depth, item)| (*depth, item.name.as_str()))
            .collect();
        assert_eq!(names, vec![(0, "A"), (1, "f"), (1, "B"), (2, "g")]);
        assert!(Arc::ptr_eq(&first, &items(&ctx, &document)));

        document.text.edit(0, 0, "// x\n").unwrap();
        let source = document.text.text();
        document.refresh_analysis(&source);
        // While typing, the previous structure is reused...
        let at = |time: f64| egui::RawInput {
            time: Some(time),
            ..Default::default()
        };
        let mut during = None;
        let _ = ctx.run(at(10.0), |ctx| during = Some(items(ctx, &document)));
        assert!(Arc::ptr_eq(&first, &during.unwrap()));
        let mut soon = None;
        let _ = ctx.run(at(10.0 + STALE_FOR / 2.0), |ctx| {
            soon = Some(items(ctx, &document))
        });
        assert!(Arc::ptr_eq(&first, &soon.unwrap()));
        // ...and rebuilt once typing pauses.
        let mut later = None;
        let _ = ctx.run(at(10.0 + STALE_FOR + 0.01), |ctx| {
            later = Some(items(ctx, &document))
        });
        assert!(!Arc::ptr_eq(&first, &later.unwrap()));
        let caret = source.find("g()").unwrap();
        document
            .selections
            .set_single(archipel_ide_text::Selection::caret(caret));
        let crumbs: Vec<_> = breadcrumbs(&ctx, &document)
            .into_iter()
            .map(|(_, name)| name)
            .collect();
        assert_eq!(crumbs, vec!["A", "B", "g"]);
        assert_eq!(
            crumb_label("androidx.constraintlayout.widget.ConstraintLayout"),
            "ConstraintLayout"
        );
        assert_eq!(
            crumb_label("Getting started with the very long heading"),
            "Getting started with the very l…"
        );
    }
}
