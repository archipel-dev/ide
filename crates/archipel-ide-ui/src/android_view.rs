//! "Android" view of the Project tool window and the Android / Project
//! view switcher in its header.

use crate::{IdeApp, icons, theme, widgets};
use archipel_ide_android::view::{NodeKind, ViewNode};
use eframe::egui::{self, vec2};
use icons::Icon;
use serde_json::json;
use std::collections::BTreeSet;

#[derive(Default)]
pub struct AndroidViewState {
    expanded: BTreeSet<String>,
    /// Ids already given their default expansion.
    seen: BTreeSet<String>,
    cache_key: Option<(usize, usize)>,
    nodes: Vec<ViewNode>,
    selected: Option<String>,
}

/// Whether the Project tool window should render the Android view.
pub fn active(app: &IdeApp) -> bool {
    app.state.is_android_project() && app.state.android().android_view
}

/// Header with the "Android ▾" / "Project ▾" view switcher. Returns whether
/// the hide button was clicked.
pub fn header(app: &mut IdeApp, ui: &mut egui::Ui) -> bool {
    let mut hide = false;
    let (rect, _) = ui.allocate_exact_size(
        vec2(
            ui.available_width(),
            widgets::tool_window_header_height(ui.ctx()),
        ),
        egui::Sense::hover(),
    );
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect.shrink2(vec2(4.0, 0.0)))
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    let title = if app.state.android().android_view {
        "Android"
    } else {
        "Project"
    };
    let response = widgets::flat_widget(
        &mut child,
        None,
        title,
        theme::semibold(theme::UI_FONT_SIZE),
        *theme::TEXT_BRIGHT,
        true,
        24.0,
    );
    egui::Popup::menu(&response).show(|ui| {
        ui.set_min_width(160.0);
        for (view, label) in [("project", "Project"), ("android", "Android")] {
            let selected = (view == "android") == app.state.android().android_view;
            if ui.selectable_label(selected, label).clicked() {
                app.dispatch("android.projectView.select", json!({"view": view}));
                ui.close();
            }
        }
    });
    child.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        if icons::button(ui, Icon::Minimize, "Hide").clicked() {
            hide = true;
        }
    });
    hide
}

/// Cheap fingerprint of the project file index; the view is rebuilt when it changes.
fn fingerprint(app: &IdeApp) -> (usize, usize) {
    let files = &app.state.files();
    (
        files.len(),
        files
            .iter()
            .map(|f| f.relative_path.as_os_str().len())
            .sum(),
    )
}

fn refresh(app: &mut IdeApp) {
    let key = fingerprint(app);
    if app.android_view.cache_key == Some(key) {
        return;
    }
    app.android_view.nodes = app.state.android_view_nodes();
    app.android_view.cache_key = Some(key);
    // Default expansion like the reference IDE: modules, manifests and source packages.
    fn defaults(nodes: &[ViewNode], depth: usize, state: &mut AndroidViewState) {
        for node in nodes {
            if state.seen.insert(node.id.clone())
                && (matches!(node.kind, NodeKind::Module | NodeKind::Package)
                    || (depth == 1
                        && matches!(node.label.as_str(), "manifests" | "java" | "kotlin+java")))
            {
                state.expanded.insert(node.id.clone());
            }
            defaults(&node.children, depth + 1, state);
        }
    }
    let nodes = std::mem::take(&mut app.android_view.nodes);
    defaults(&nodes, 0, &mut app.android_view);
    app.android_view.nodes = nodes;
}

struct Row {
    depth: usize,
    id: String,
    label: String,
    detail: Option<String>,
    kind: NodeKind,
    path: Option<std::path::PathBuf>,
    expandable: bool,
    expanded: bool,
}

fn flatten(nodes: &[ViewNode], depth: usize, expanded: &BTreeSet<String>, rows: &mut Vec<Row>) {
    for node in nodes {
        let is_expanded = expanded.contains(&node.id);
        rows.push(Row {
            depth,
            id: node.id.clone(),
            label: node.label.clone(),
            detail: node.detail.clone(),
            kind: node.kind,
            path: node.path.clone(),
            expandable: !node.children.is_empty(),
            expanded: is_expanded,
        });
        if is_expanded {
            flatten(&node.children, depth + 1, expanded, rows);
        }
    }
}

fn icon_for(row: &Row) -> Icon {
    match row.kind {
        NodeKind::Module | NodeKind::Group | NodeKind::Package | NodeKind::ResourceType => {
            Icon::Folder
        }
        NodeKind::GradleScripts => Icon::Build,
        NodeKind::SourceFile | NodeKind::ResourceGroup | NodeKind::File => Icon::for_file(
            row.path
                .as_ref()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                .unwrap_or(&row.label),
        ),
    }
}

pub fn tree(app: &mut IdeApp, ui: &mut egui::Ui) {
    refresh(app);
    let mut rows = Vec::new();
    flatten(
        &app.android_view.nodes,
        0,
        &app.android_view.expanded,
        &mut rows,
    );
    let mut toggle = None;
    let mut open = None;
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show_rows(ui, widgets::ROW_HEIGHT, rows.len(), |ui, range| {
            ui.spacing_mut().item_spacing.y = 0.0;
            for row in &rows[range] {
                let selected = app.android_view.selected.as_deref() == Some(row.id.as_str());
                let response = widgets::tree_row(
                    ui,
                    widgets::TreeRow {
                        depth: row.depth,
                        expanded: row.expandable.then_some(row.expanded),
                        selected,
                        icon: Some((icon_for(row), *icons::ICON)),
                        font: if row.kind == NodeKind::Module {
                            theme::semibold(theme::UI_FONT_SIZE)
                        } else {
                            egui::FontId::proportional(theme::UI_FONT_SIZE)
                        },
                        secondary: row.detail.as_deref().map(|d| (d, *theme::TEXT_INFO)),
                        ..widgets::TreeRow::new(&row.label)
                    },
                )
                .response;
                if response.clicked() {
                    app.android_view.selected = Some(row.id.clone());
                    if row.expandable {
                        toggle = Some(row.id.clone());
                    }
                }
                if response.double_clicked()
                    && let Some(path) = &row.path
                {
                    open = Some(path.clone());
                }
            }
        });
    if let Some(id) = toggle
        && !app.android_view.expanded.remove(&id)
    {
        app.android_view.expanded.insert(id);
    }
    if let Some(path) = open {
        app.dispatch("editor.openFile", json!({"path": path}));
    }
}
