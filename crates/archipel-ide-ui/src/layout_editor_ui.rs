//! Android layout editor: the `Code | Split | Design` toggle on layout XML
//! files, a simplified rendered preview of the view tree, the Component Tree
//! and the Attributes panel.

use crate::{IdeApp, theme};
use archipel_ide_android::layout::{LayoutNode, RenderedBox, layout_boxes};
use archipel_ide_core::LayoutMode;
use eframe::egui::{self, Color32, RichText, Sense, vec2};
use serde_json::json;

const SCREEN: (f32, f32) = (360.0, 640.0);

/// Whether the active editor file is an Android layout.
pub fn applies(app: &IdeApp) -> bool {
    app.state.active_layout().is_some()
}

/// Top-right `Code | Split | Design` toggle.
pub fn mode_bar(app: &mut IdeApp, ui: &mut egui::Ui) {
    let current = app.state.android.layout.mode;
    let mut choice = None;
    egui::TopBottomPanel::top("layout_mode_bar")
        .exact_height(26.0)
        .frame(egui::Frame::new())
        .show_inside(ui, |ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(8.0);
                for mode in LayoutMode::ALL.into_iter().rev() {
                    let selected = mode == current;
                    let text = RichText::new(mode.title()).color(if selected {
                        *theme::TEXT_BRIGHT
                    } else {
                        *theme::TEXT_SECONDARY
                    });
                    if ui
                        .add(egui::Button::new(text).selected(selected).frame(selected))
                        .clicked()
                    {
                        choice = Some(mode);
                    }
                }
            });
        });
    if let Some(mode) = choice {
        app.dispatch("android.layout.mode", json!({"mode": mode.id()}));
    }
}

/// Split mode: the preview and attributes beside the text editor.
pub fn split_panel(app: &mut IdeApp, ui: &mut egui::Ui) {
    let Some(Ok(tree)) = app.state.active_layout() else {
        return;
    };
    let mut select = None;
    egui::SidePanel::right("layout_split_panel")
        .default_width(ui.available_width() / 2.0)
        .resizable(true)
        .frame(egui::Frame::new())
        .show_inside(ui, |ui| {
            egui::TopBottomPanel::bottom("layout_split_attributes")
                .resizable(true)
                .default_height(160.0)
                .frame(egui::Frame::new())
                .show_inside(ui, |ui| attributes(app, ui, &tree));
            preview(app, ui, &tree, &mut select);
        });
    dispatch_select(app, select);
}

/// Design mode: Component Tree, preview and Attributes instead of the editor.
pub fn design_body(app: &mut IdeApp, ui: &mut egui::Ui) {
    let tree = match app.state.active_layout() {
        Some(Ok(tree)) => tree,
        Some(Err(error)) => {
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                ui.add_space(12.0);
                ui.label(
                    RichText::new(format!("The layout cannot be rendered: {error}"))
                        .color(theme::TEXT_INFO),
                );
            });
            return;
        }
        None => return,
    };
    let mut select = None;
    egui::SidePanel::left("layout_component_tree")
        .default_width(200.0)
        .resizable(true)
        .frame(egui::Frame::new())
        .show_inside(ui, |ui| component_tree(app, ui, &tree, &mut select));
    egui::SidePanel::right("layout_attributes")
        .default_width(240.0)
        .resizable(true)
        .frame(egui::Frame::new())
        .show_inside(ui, |ui| attributes(app, ui, &tree));
    egui::CentralPanel::default()
        .frame(egui::Frame::new())
        .show_inside(ui, |ui| preview(app, ui, &tree, &mut select));
    dispatch_select(app, select);
}

fn dispatch_select(app: &mut IdeApp, select: Option<usize>) {
    if let Some(node) = select
        && node != app.state.android.layout.selected
    {
        app.dispatch("android.layout.select", json!({"node": node}));
    }
}

fn heading(ui: &mut egui::Ui, title: &str) {
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.add_space(10.0);
        ui.label(
            RichText::new(title)
                .font(theme::semibold(theme::UI_FONT_SIZE))
                .color(theme::TEXT_BRIGHT),
        );
    });
    ui.add_space(4.0);
}

fn component_tree(app: &IdeApp, ui: &mut egui::Ui, tree: &LayoutNode, select: &mut Option<usize>) {
    heading(ui, "Component Tree");
    let selected = app.state.android.layout.selected;
    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let mut index = 0;
            tree_rows(ui, tree, 0, &mut index, selected, select);
        });
}

fn tree_rows(
    ui: &mut egui::Ui,
    node: &LayoutNode,
    depth: usize,
    index: &mut usize,
    selected: usize,
    select: &mut Option<usize>,
) {
    let mine = *index;
    *index += 1;
    let (rect, response) = crate::widgets::list_row(ui, mine == selected);
    let text = RichText::new(node.label()).color(if mine == selected {
        *theme::TEXT_BRIGHT
    } else {
        *theme::TEXT
    });
    ui.painter().text(
        rect.left_center() + vec2(12.0 + depth as f32 * 16.0, 0.0),
        egui::Align2::LEFT_CENTER,
        text.text(),
        egui::FontId::proportional(theme::UI_FONT_SIZE),
        if mine == selected {
            *theme::TEXT_BRIGHT
        } else {
            *theme::TEXT
        },
    );
    if response.clicked() {
        *select = Some(mine);
    }
    for child in &node.children {
        tree_rows(ui, child, depth + 1, index, selected, select);
    }
}

fn attributes(app: &IdeApp, ui: &mut egui::Ui, tree: &LayoutNode) {
    heading(ui, "Attributes");
    let Some(node) = tree.nth(app.state.android.layout.selected).or(Some(tree)) else {
        return;
    };
    egui::ScrollArea::vertical()
        .id_salt("layout_attributes_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Frame::new()
                .inner_margin(egui::Margin::symmetric(10, 0))
                .show(ui, |ui| {
                    ui.label(
                        RichText::new(&node.tag)
                            .font(theme::semibold(theme::UI_FONT_SIZE))
                            .color(theme::TEXT_BRIGHT),
                    );
                    ui.add_space(4.0);
                    egui::Grid::new("layout_attribute_grid")
                        .num_columns(2)
                        .spacing([12.0, 4.0])
                        .show(ui, |ui| {
                            for (name, value) in &node.attributes {
                                let name = name.strip_prefix("android:").unwrap_or(name);
                                ui.label(RichText::new(name).color(theme::TEXT_SECONDARY));
                                ui.label(RichText::new(value).color(*theme::TEXT));
                                ui.end_row();
                            }
                        });
                });
        });
}

fn box_fill(depth: usize) -> Color32 {
    const SHADES: [Color32; 4] = [
        Color32::from_rgb(0x2B, 0x2D, 0x30),
        Color32::from_rgb(0x32, 0x36, 0x3B),
        Color32::from_rgb(0x2E, 0x3A, 0x40),
        Color32::from_rgb(0x38, 0x34, 0x40),
    ];
    SHADES[depth % SHADES.len()]
}

fn preview(app: &IdeApp, ui: &mut egui::Ui, tree: &LayoutNode, select: &mut Option<usize>) {
    heading(ui, "Preview");
    let selected = app.state.android.layout.selected;
    let available = ui.available_size();
    let scale = (available.x / SCREEN.0)
        .min(available.y / SCREEN.1)
        .clamp(0.2, 1.5);
    let size = vec2(SCREEN.0, SCREEN.1) * scale;
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 6.0, *theme::FIELD);
    painter.rect_stroke(
        rect,
        6.0,
        egui::Stroke::new(1.0_f32, theme::BORDER),
        egui::StrokeKind::Inside,
    );
    let boxes: Vec<RenderedBox> = layout_boxes(tree, SCREEN.0, SCREEN.1);
    let to_rect = |b: &RenderedBox| {
        egui::Rect::from_min_size(
            rect.min + vec2(b.x, b.y) * scale,
            vec2(b.width, b.height) * scale,
        )
    };
    for b in &boxes {
        let box_rect = to_rect(b);
        let is_selected = b.index == selected;
        painter.rect_filled(box_rect, 2.0, box_fill(b.depth));
        painter.rect_stroke(
            box_rect,
            2.0,
            egui::Stroke::new(
                if is_selected { 2.0_f32 } else { 1.0_f32 },
                if is_selected {
                    *theme::TAB_SELECTED_BORDER
                } else {
                    *theme::BORDER
                },
            ),
            egui::StrokeKind::Inside,
        );
        painter.text(
            box_rect.left_top() + vec2(4.0, 2.0),
            egui::Align2::LEFT_TOP,
            &b.label,
            egui::FontId::proportional(11.0),
            *theme::TEXT,
        );
    }
    if response.clicked()
        && let Some(pos) = response.interact_pointer_pos()
    {
        // Deepest box under the pointer wins (boxes are emitted parents first).
        *select = boxes
            .iter()
            .rev()
            .find(|b| to_rect(b).contains(pos))
            .map(|b| b.index);
    }
}
