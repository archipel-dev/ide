//! Code completion popup ("lookup").
//!
//! Row layout follows the reference IDE: kind icon, label with the matched
//! characters in bold, the signature in grey right after it and the type
//! right-aligned; deprecated items are struck through. The popup is anchored
//! so labels line up with the identifier being completed.

use crate::{IdeApp, theme};
use archipel_kit::components::ListRow;
use archipel_kit::icons::{Glyph, Icon, Symbol, SymbolKind as Kind};
use archipel_kit::parts::{Slot, Surface};
use archipel_kit::tokens::size;
use archipel_kit::{Theme, Tone};
use eframe::egui::{self, FontId, Rect, pos2};
use jetthing_lsp::completion::{self as ranking, kind_name};
use jetthing_lsp::lsp_types::CompletionItem;
use serde_json::json;

const ROW_HEIGHT: f32 = size::ROW_HEIGHT;
const VISIBLE_ROWS: usize = 11;
const ICON_COLUMN: f32 = 26.0;
const FOOTER_HEIGHT: f32 = 26.0;
const MIN_WIDTH: f32 = 520.0;
const MAX_WIDTH: f32 = 720.0;
/// Kit code-symbol icon for an LSP completion kind; `None` for plain text and keywords.
fn kind_icon(kind: &str) -> Option<Icon> {
    let symbol = |kind| Icon::Symbol(Symbol::new(kind));
    Some(match kind {
        "method" => symbol(Kind::Method),
        "function" => symbol(Kind::Function),
        "constructor" => symbol(Kind::Constructor),
        "field" => symbol(Kind::Field),
        "property" => symbol(Kind::Property),
        "variable" | "value" | "reference" => symbol(Kind::Variable),
        "constant" => symbol(Kind::Constant),
        "enumMember" => symbol(Kind::EnumMember),
        "class" => symbol(Kind::Class),
        "struct" => symbol(Kind::Struct),
        "interface" => symbol(Kind::Interface),
        "enum" => symbol(Kind::Enum),
        "module" | "file" => symbol(Kind::Module),
        "typeParameter" => symbol(Kind::TypeParameter),
        "snippet" => Icon::Glyph(Glyph::Code),
        "event" | "operator" => symbol(Kind::Function),
        _ => return None,
    })
}

struct Row {
    label: String,
    detail: Option<String>,
    type_text: Option<String>,
    kind: &'static str,
}

fn row_for(item: &CompletionItem) -> Row {
    Row {
        label: item.label.clone(),
        detail: ranking::label_detail(item).map(str::to_owned),
        type_text: ranking::type_text(item)
            .filter(|_| item.label_details.is_some() || ranking::label_detail(item).is_none())
            .map(str::to_owned),
        kind: kind_name(item.kind),
    }
}

/// Shows the popup; returns its screen rect for the documentation panel.
pub fn show(app: &mut IdeApp, ctx: &egui::Context) -> Option<Rect> {
    let session = app.state.completion.as_ref()?;
    if !session.visible() && !session.indexing_only() {
        return None;
    }
    let caret = app.caret_screen_pos?;
    // A click that takes keyboard focus away from the editor (project tree,
    // tool window, toolbar) dismisses the popup, as in the reference IDE.
    // Clicks on the popup itself select a row and are handled below.
    if session.visible() && !app.editor_focused(ctx) {
        let popup_rect = ctx.memory(|memory| memory.area_rect(egui::Id::new("completion_popup")));
        let pressed_outside = ctx.input(|input| {
            input.pointer.any_pressed()
                && input
                    .pointer
                    .interact_pos()
                    .is_some_and(|pos| !popup_rect.is_some_and(|rect| rect.contains(pos)))
        });
        if pressed_outside {
            app.dispatch("completion.close", serde_json::Value::Null);
            return None;
        }
    }
    let indexing = session.indexing.clone();
    if indexing.is_some() {
        // Re-query and refresh the notice when the server finishes.
        ctx.request_repaint_after(std::time::Duration::from_millis(200));
    }
    let rows: Vec<Row> = session
        .ranked
        .iter()
        .map(|ranked| row_for(&session.items[ranked.index]))
        .collect();
    let selected = session.selected;
    let prefix_chars = session.prefix.chars().count() as f32;

    let mono = FontId::monospace(theme::EDITOR_FONT_SIZE);
    let small = FontId::monospace(theme::EDITOR_FONT_SIZE - 1.0);
    let char_width = ctx.fonts_mut(|fonts| fonts.glyph_width(&mono, 'M'));
    // Width from the widest of the first rows, like the reference popup.
    let content_width = rows
        .iter()
        .take(60)
        .map(|row| {
            let label = row.label.chars().count() as f32 * char_width;
            let detail = row
                .detail
                .as_ref()
                .map_or(0.0, |d| d.chars().count() as f32 * char_width);
            let kind = row.type_text.as_ref().map_or(0.0, |t| {
                t.chars().count().min(40) as f32 * char_width + 24.0
            });
            ICON_COLUMN + 8.0 + label + detail + kind + 16.0
        })
        .fold(MIN_WIDTH, f32::max)
        .min(MAX_WIDTH)
        .max(if indexing.is_some() { 360.0 } else { 0.0 });
    let visible = rows.len().min(VISIBLE_ROWS);
    let height = visible as f32 * ROW_HEIGHT + FOOTER_HEIGHT + 2.0;
    let screen = ctx.content_rect();
    // Align the label column with the start of the typed identifier.
    let mut x = caret.x - prefix_chars * char_width - ICON_COLUMN - 6.0;
    x = x
        .min(screen.right() - content_width - 8.0)
        .max(screen.left() + 4.0);
    let mut y = caret.y + 2.0;
    if y + height > screen.bottom() - crate::chrome::STATUS_BAR_HEIGHT {
        y = caret.y - ROW_HEIGHT - height - 2.0;
    }

    let mut clicked = None;
    let colors = Theme::colors(ctx);
    let area = egui::Area::new(egui::Id::new("completion_popup"))
        .order(egui::Order::Foreground)
        .sense(egui::Sense::hover())
        .fixed_pos(pos2(x, y))
        .show(ctx, |ui| {
            Surface::Popup.frame(ctx).show(ui, |ui| {
                ui.set_width(content_width);
                ui.spacing_mut().item_spacing.y = 0.0;
                // The editor keeps keyboard focus while the popup is open: Up/Down/
                // Enter/Tab reach the list as `completion.*` commands from the editor keys.
                // Keep the selected row in view (the rows are virtualized, so this is an
                // offset, not a `scroll_to_rect`).
                let view_height = visible as f32 * ROW_HEIGHT;
                let offset_id = egui::Id::new("completion_scroll");
                let (last_selected, last_offset): (usize, f32) =
                    ctx.data(|data| data.get_temp(offset_id).unwrap_or((usize::MAX, 0.0)));
                let top = selected as f32 * ROW_HEIGHT;
                let mut area = egui::ScrollArea::vertical()
                    .id_salt("completion_rows")
                    .max_height(view_height)
                    .auto_shrink([false, true]);
                if last_selected != selected {
                    if top < last_offset {
                        area = area.vertical_scroll_offset(top);
                    } else if top + ROW_HEIGHT > last_offset + view_height {
                        area = area.vertical_scroll_offset(top + ROW_HEIGHT - view_height);
                    }
                }
                let scrolled = area.show_rows(ui, ROW_HEIGHT, rows.len(), |ui, range| {
                        for index in range {
                            let row = &rows[index];
                            let mut view = ListRow::new(row.label.as_str())
                                .flat(true)
                                .selected(index == selected);
                            if let Some(icon) = kind_icon(row.kind) {
                                view = view.icon(icon);
                            }
                            if let Some(detail) = &row.detail {
                                view = view.secondary(detail.as_str());
                            }
                            if let Some(type_text) = &row.type_text {
                                view = view
                                    .trailing(Slot::text_tone(type_text.as_str(), Tone::Secondary));
                            }
                            if view.show(ui).response.clicked() {
                                clicked = Some(index);
                            }
                        }
                });
                ctx.data_mut(|data| {
                    data.insert_temp(offset_id, (selected, scrolled.state.offset.y))
                });
                ui.separator();
                let (hint, color) = match indexing.as_deref() {
                    Some(status) => (indexing_notice(status), colors.warning),
                    None => (shortcut_hint(), colors.text_secondary),
                };
                ui.add_space(2.0);
                ui.horizontal(|ui| {
                    ui.add_space(12.0);
                    ui.label(
                        egui::RichText::new(hint)
                            .size(small.size - 0.5)
                            .color(color),
                    );
                });
            });
        });
    if let Some(row) = clicked {
        app.dispatch("completion.accept", json!({"row": row}));
    }
    Some(area.response.rect)
}

fn shortcut_hint() -> String {
    if cfg!(target_os = "macos") {
        "⏎ inserts, ⇥ replaces the word at the caret, ⎋ closes".to_owned()
    } else {
        "Enter inserts, Tab replaces the word at the caret, Esc closes".to_owned()
    }
}

/// "Indexing… results may be incomplete (jdtls: Building 0%)".
pub(crate) fn indexing_notice(status: &str) -> String {
    format!("Indexing… results may be incomplete ({status})")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indexing_notice_names_the_server_activity() {
        assert_eq!(
            indexing_notice("jdtls: Building 0%"),
            "Indexing… results may be incomplete (jdtls: Building 0%)"
        );
    }

    #[test]
    fn rows_use_label_details_for_signature_and_type() {
        let item: CompletionItem = serde_json::from_value(json!({
            "label": "greeting", "kind": 2, "detail": "App.greeting(String name) : String",
            "labelDetails": {"detail": "(String name)", "description": "String"}
        }))
        .unwrap();
        let row = row_for(&item);
        assert_eq!(row.detail.as_deref(), Some("(String name)"));
        assert_eq!(row.type_text.as_deref(), Some("String"));
        assert_eq!(row.kind, "method");
        let plain: CompletionItem =
            serde_json::from_value(json!({"label": "len", "detail": "fn(&self) -> usize"}))
                .unwrap();
        assert_eq!(
            row_for(&plain).type_text.as_deref(),
            Some("fn(&self) -> usize")
        );
        assert!(kind_icon("method").is_some());
        assert!(kind_icon("text").is_none());
    }
}
