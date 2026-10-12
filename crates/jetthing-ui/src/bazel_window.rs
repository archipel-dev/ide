//! Bazel tool window: packages → targets, with Build / Run / Test per target.

use crate::icons::{self, Icon};
use crate::widgets::{self, island_frame, search_field, tool_window_header};
use crate::{IdeApp, theme};
use eframe::egui::{self, RichText, vec2};
use jetthing_bazel::{BazelTargets, TargetRole};
use jetthing_core::ToolWindow;
use jetthing_runner::TaskKind;
use serde_json::{Value, json};
use std::collections::HashSet;

#[derive(Clone, Default)]
struct View {
    collapsed: HashSet<String>,
    selected: Option<String>,
    filter: String,
}

fn view_id() -> egui::Id {
    egui::Id::new("bazel_window_view")
}

/// Visible rows: `(package, None)` for package headers, `(package, Some(label))` for
/// targets. A filter matches labels and kinds case-insensitively.
pub fn rows(
    targets: &BazelTargets,
    collapsed: &HashSet<String>,
    filter: &str,
) -> Vec<(String, Option<String>)> {
    let filter = filter.trim().to_lowercase();
    let mut rows = Vec::new();
    for (package, members) in targets.by_package() {
        let visible: Vec<_> = members
            .into_iter()
            .filter(|target| {
                filter.is_empty()
                    || target.label.to_lowercase().contains(&filter)
                    || target.kind.to_lowercase().contains(&filter)
            })
            .collect();
        if visible.is_empty() {
            continue;
        }
        let open = !filter.is_empty() || !collapsed.contains(&package);
        rows.push((package.clone(), None));
        if open {
            rows.extend(
                visible
                    .into_iter()
                    .map(|target| (package.clone(), Some(target.label.clone()))),
            );
        }
    }
    rows
}

pub fn show(app: &mut IdeApp, ctx: &egui::Context) {
    if !app.state.tool_windows.is_visible(ToolWindow::Bazel) {
        return;
    }
    let island = egui::SidePanel::right("right_tool_window_bazel")
        .default_width(340.0)
        .min_width(220.0)
        .resizable(true)
        .show_separator_line(false)
        .frame(island_frame(ctx))
        .show(ctx, |ui| content(app, ui));
    crate::chrome::remember_island(ctx, jetthing_core::Anchor::Right, island.response.rect);
}

fn content(app: &mut IdeApp, ui: &mut egui::Ui) {
    if tool_window_header(ui, "Bazel", |_| {}) {
        app.toggle_tool_window(ToolWindow::Bazel);
    }
    let mut view: View = ui.data(|data| data.get_temp(view_id())).unwrap_or_default();
    let syncing = app.state.runner.is_running()
        && app
            .state
            .runner
            .plan()
            .is_some_and(|plan| plan.kind == TaskKind::Sync);
    let selected = view
        .selected
        .as_ref()
        .and_then(|label| app.state.bazel_targets.as_ref()?.get(label).cloned());
    widgets::toolbar_row(ui, |ui| {
        ui.add_space(theme::layout::TOOLBAR_LEAD);
        ui.spacing_mut().item_spacing.x = 2.0;
        if syncing {
            ui.add_sized(
                vec2(26.0, 26.0),
                egui::Spinner::new().size(14.0).color(theme::ACCENT),
            );
        } else {
            let can_sync = app.commands.is_enabled("bazel.sync", &app.state);
            if ui
                .add_enabled_ui(can_sync, |ui| {
                    icons::button(ui, Icon::Refresh, "Sync Bazel Project")
                })
                .inner
                .clicked()
            {
                app.dispatch("bazel.sync", Value::Null);
            }
        }
        ui.add_space(6.0);
        let can_run = selected.is_some() && app.commands.is_enabled("bazel.runTarget", &app.state);
        for (command, icon, color, tooltip) in [
            ("build", Icon::Build, icons::ICON, "Build Target"),
            ("run", Icon::Run, theme::RUN_GREEN, "Run Target"),
            ("test", Icon::Rerun, theme::RUN_GREEN, "Test Target"),
        ] {
            let applies = selected.as_ref().is_some_and(|target| match command {
                "run" => target.role() == TargetRole::Binary,
                "test" => target.role() == TargetRole::Test,
                _ => true,
            });
            let enabled = can_run && applies;
            let color = if enabled { color } else { theme::TEXT_DISABLED };
            if ui
                .add_enabled_ui(enabled, |ui| {
                    icons::button_colored(ui, icon, color, tooltip, false)
                })
                .inner
                .clicked()
                && let Some(target) = &selected
            {
                app.dispatch(
                    "bazel.runTarget",
                    json!({"label": target.label, "command": command}),
                );
            }
        }
    });
    let Some(targets) = app.state.bazel_targets.clone() else {
        ui.add_space(16.0);
        ui.vertical_centered(|ui| {
            if syncing {
                ui.label(RichText::new("Querying Bazel targets…").color(theme::TEXT_SECONDARY));
                return;
            }
            match &app.state.bazel_error {
                Some(error) => {
                    ui.label(RichText::new(error).color(theme::ERROR));
                }
                None => {
                    ui.label(
                        RichText::new("Bazel project is not synced yet")
                            .color(theme::TEXT_SECONDARY),
                    );
                    ui.label(
                        RichText::new(format!(
                            "Sync runs bazel query '{}'.",
                            jetthing_bazel::QUERY
                        ))
                        .color(theme::TEXT_INFO),
                    );
                }
            }
            ui.add_space(6.0);
            let enabled = app.commands.is_enabled("bazel.sync", &app.state);
            if ui
                .add_enabled(enabled, egui::Button::new("Sync Bazel Project"))
                .clicked()
            {
                app.dispatch("bazel.sync", Value::Null);
            }
        });
        ui.data_mut(|data| data.insert_temp(view_id(), view));
        return;
    };
    widgets::toolbar_row(ui, |ui| {
        ui.add_space(theme::layout::CONTENT_INSET);
        search_field(
            ui,
            egui::Id::new("bazel_filter"),
            &mut view.filter,
            "Filter targets",
            ui.available_width() - 30.0,
        );
    });
    ui.add_space(4.0);
    let mut run = None;
    egui::ScrollArea::both()
        .id_salt("bazel_tree")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            for (package, label) in rows(&targets, &view.collapsed, &view.filter) {
                let selected = label.is_some() && view.selected == label;
                match label {
                    None => {
                        let open =
                            !view.filter.trim().is_empty() || !view.collapsed.contains(&package);
                        let response = widgets::tree_row(
                            ui,
                            widgets::TreeRow {
                                expanded: Some(open),
                                icon: Some((Icon::Folder, *icons::ICON)),
                                ..widgets::TreeRow::new(&package)
                            },
                        )
                        .response;
                        if response.clicked() && !view.collapsed.remove(&package) {
                            view.collapsed.insert(package.clone());
                        }
                    }
                    Some(label) => {
                        let Some(target) = targets.get(&label) else {
                            continue;
                        };
                        let (icon, color) = match target.role() {
                            TargetRole::Binary => (Icon::Run, *theme::RUN_GREEN),
                            TargetRole::Test => (Icon::Rerun, *theme::RUN_GREEN),
                            TargetRole::Library | TargetRole::Other => (Icon::Model, *icons::ICON),
                        };
                        let response = widgets::tree_row(
                            ui,
                            widgets::TreeRow {
                                depth: 1,
                                selected,
                                icon: Some((icon, color)),
                                secondary: Some((&target.kind, *theme::TEXT_INFO)),
                                ..widgets::TreeRow::new(target.name())
                            },
                        )
                        .response;
                        let response = response.on_hover_text(format!(
                            "{}\nDouble-click to {}",
                            target.label,
                            target.role().default_command()
                        ));
                        if response.clicked() {
                            view.selected = Some(label.clone());
                        }
                        if response.double_clicked() {
                            run = Some(label.clone());
                        }
                    }
                }
            }
        });
    if let Some(label) = run {
        app.dispatch("bazel.runTarget", json!({"label": label}));
    }
    ui.data_mut(|data| data.insert_temp(view_id(), view));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn targets() -> BazelTargets {
        BazelTargets::parse_label_kind(
            "java_binary rule //:app\nsh_test rule //:app_test\njava_library rule //lib:greeter"
                .lines(),
        )
    }

    #[test]
    fn rows_group_targets_by_package() {
        let rows = rows(&targets(), &HashSet::new(), "");
        let labels: Vec<_> = rows
            .iter()
            .map(|(package, label)| label.clone().unwrap_or_else(|| package.clone()))
            .collect();
        assert_eq!(
            labels,
            ["//", "//:app", "//:app_test", "//lib", "//lib:greeter"]
        );
    }

    #[test]
    fn collapsed_packages_and_filters() {
        let collapsed: HashSet<String> = ["//".to_owned()].into();
        assert_eq!(super::rows(&targets(), &collapsed, "").len(), 3);
        let filtered = super::rows(&targets(), &collapsed, "LIBRARY");
        assert_eq!(
            filtered,
            [
                ("//lib".to_owned(), None),
                ("//lib".to_owned(), Some("//lib:greeter".to_owned()))
            ]
        );
    }
}
