//! Hierarchy tool window: the Type / Call Hierarchy tree. State and requests
//! live in `archipel_ide_core::hierarchy`; rows dispatch `hierarchy.*` commands.

use crate::widgets::{list_row, row_text, tool_window_header};
use crate::{IdeApp, theme};
use archipel_ide_core::ToolWindow;
use eframe::egui::{self, Rect, vec2};
use serde_json::json;

const INDENT: f32 = 16.0;

pub fn show(app: &mut IdeApp, ui: &mut egui::Ui) {
    let Some(hierarchy) = app.state.hierarchy.current.clone() else {
        tool_window_header(ui, "Hierarchy", |_| {});
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.add_space(12.0);
            ui.label(
                egui::RichText::new("Press Ctrl+H or Ctrl+Alt+H on a symbol to show its hierarchy")
                    .color(theme::TEXT_SECONDARY),
            );
        });
        return;
    };
    let mut switch = false;
    let mut close = false;
    let hide = tool_window_header(ui, &hierarchy.title(), |ui| {
        if crate::icons::button(ui, crate::icons::Icon::Close, "Close Hierarchy").clicked() {
            close = true;
        }
        let other = match hierarchy.direction {
            archipel_ide_core::HierarchyDirection::Up => {
                archipel_ide_core::HierarchyDirection::Down
            }
            archipel_ide_core::HierarchyDirection::Down => {
                archipel_ide_core::HierarchyDirection::Up
            }
        };
        let label = format!("Show {}", hierarchy.kind.direction_label(other));
        if ui.button(label).clicked() {
            switch = true;
        }
    });
    let font = egui::FontId::proportional(theme::UI_FONT_SIZE);
    let mut toggle = None;
    let mut open = None;
    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            for row in hierarchy.rows() {
                let node = &hierarchy.nodes[row.node];
                let (rect, response) = list_row(ui, row.node == hierarchy.selected);
                let x = rect.left() + 12.0 + row.depth as f32 * INDENT;
                let arrow = if node.loading {
                    "…"
                } else if node.loaded && node.children.is_empty() {
                    " "
                } else if node.expanded {
                    "▾"
                } else {
                    "▸"
                };
                let arrow_rect =
                    Rect::from_min_size(egui::pos2(x, rect.top()), vec2(14.0, rect.height()));
                row_text(ui, rect, x, arrow, font.clone(), theme::TEXT_SECONDARY);
                let mut end = row_text(
                    ui,
                    rect,
                    x + 16.0,
                    &node.label,
                    font.clone(),
                    theme::TEXT_BRIGHT,
                );
                if let Some(detail) = &node.detail {
                    end = row_text(ui, rect, end + 8.0, detail, font.clone(), theme::TEXT_INFO);
                }
                if let Some(target) = &node.target {
                    row_text(
                        ui,
                        rect,
                        end + 8.0,
                        &target.path.display().to_string(),
                        font.clone(),
                        theme::TEXT_DISABLED,
                    );
                }
                let on_arrow = response
                    .interact_pointer_pos()
                    .is_some_and(|pos| arrow_rect.contains(pos));
                if response.double_clicked() {
                    open = Some(row.node);
                } else if response.clicked() {
                    if on_arrow || !node.loaded {
                        toggle = Some(row.node);
                    } else {
                        app.state.hierarchy_select(row.node);
                    }
                }
            }
        });
    if switch {
        app.dispatch("hierarchy.toggleDirection", json!({}));
    }
    if let Some(node) = toggle {
        app.dispatch("hierarchy.toggleNode", json!({"node": node}));
    }
    if let Some(node) = open {
        app.dispatch("hierarchy.openNode", json!({"node": node}));
    }
    if close {
        app.dispatch("hierarchy.close", json!({}));
    } else if hide {
        app.toggle_tool_window(ToolWindow::Hierarchy);
    }
}
