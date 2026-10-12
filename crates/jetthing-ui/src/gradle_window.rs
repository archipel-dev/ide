//! Gradle tool window: synced projects → Tasks → groups → tasks.

use crate::icons::{self, Icon};
use crate::widgets::{self, island_frame, search_field, tool_window_header};
use crate::{IdeApp, theme};
use eframe::egui::{self, RichText, vec2};
use jetthing_core::ToolWindow;
use jetthing_gradle::{GradleModel, ProjectModel};
use jetthing_runner::TaskKind;
use serde_json::{Value, json};
use std::collections::HashSet;

#[derive(Clone, Default)]
struct View {
    initialized: bool,
    expanded: HashSet<String>,
    selected: Option<String>,
    filter: String,
    execute: Option<String>,
    execute_focused: bool,
}

fn view_id() -> egui::Id {
    egui::Id::new("gradle_window_view")
}

/// One visible tree row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub id: String,
    pub depth: usize,
    pub label: String,
    pub kind: RowKind,
    pub expandable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RowKind {
    Project,
    Folder,
    Task {
        path: String,
        description: Option<String>,
    },
}

/// Flattens the model into visible rows. A non-empty `filter` shows matching tasks
/// (case-insensitive substring of name or path) with all their parents expanded.
pub fn rows(model: &GradleModel, expanded: &HashSet<String>, filter: &str) -> Vec<Row> {
    let filter = filter.trim().to_lowercase();
    let mut rows = Vec::new();
    if let Some(root) = model.root() {
        project_rows(model, root, 0, expanded, &filter, &mut rows);
    }
    rows
}

/// Configurations listed under a project's `Dependencies` node.
const DEPENDENCY_CONFIGURATIONS: &[&str] = &[
    "compileClasspath",
    "runtimeClasspath",
    "testRuntimeClasspath",
];

/// `:app:` for project `:app`, empty for the root project.
fn task_prefix(project_path: &str) -> String {
    match project_path.trim_matches(':') {
        "" => String::new(),
        path => format!(":{path}:"),
    }
}

fn project_rows(
    model: &GradleModel,
    project: &ProjectModel,
    depth: usize,
    expanded: &HashSet<String>,
    filter: &str,
    rows: &mut Vec<Row>,
) -> bool {
    let matches = |name: &str, path: &str| {
        filter.is_empty()
            || name.to_lowercase().contains(filter)
            || path.to_lowercase().contains(filter)
    };
    let project_id = format!("p:{}", project.path);
    let start = rows.len();
    rows.push(Row {
        id: project_id.clone(),
        depth,
        label: project.name.clone(),
        kind: RowKind::Project,
        expandable: true,
    });
    let open = |id: &str| !filter.is_empty() || expanded.contains(id);
    let mut any = false;
    if open(&project_id) {
        let groups = project.task_groups();
        let tasks_id = format!("t:{}", project.path);
        let tasks_start = rows.len();
        if !project.tasks.is_empty() {
            rows.push(Row {
                id: tasks_id.clone(),
                depth: depth + 1,
                label: "Tasks".into(),
                kind: RowKind::Folder,
                expandable: true,
            });
        }
        let mut any_task = false;
        if open(&tasks_id) {
            for (group, tasks) in &groups {
                let group_id = format!("g:{}:{group}", project.path);
                let visible: Vec<_> = tasks
                    .iter()
                    .filter(|task| matches(&task.name, &task.path))
                    .collect();
                if visible.is_empty() {
                    continue;
                }
                any_task = true;
                rows.push(Row {
                    id: group_id.clone(),
                    depth: depth + 2,
                    label: group.clone(),
                    kind: RowKind::Folder,
                    expandable: true,
                });
                if open(&group_id) {
                    for task in visible {
                        rows.push(Row {
                            id: format!("task:{}", task.path),
                            depth: depth + 3,
                            label: task.name.clone(),
                            kind: RowKind::Task {
                                path: task.path.clone(),
                                description: task.description.clone(),
                            },
                            expandable: false,
                        });
                    }
                }
            }
        } else {
            any_task = project
                .tasks
                .iter()
                .any(|task| matches(&task.name, &task.path));
        }
        if !any_task && !filter.is_empty() {
            rows.truncate(tasks_start);
        }
        // Dependencies: the `dependencies` report of the main classpaths.
        let deps_id = format!("d:{}", project.path);
        let has_report = project.tasks.iter().any(|task| task.name == "dependencies");
        if has_report && filter.is_empty() {
            rows.push(Row {
                id: deps_id.clone(),
                depth: depth + 1,
                label: "Dependencies".into(),
                kind: RowKind::Folder,
                expandable: true,
            });
            if open(&deps_id) {
                for configuration in DEPENDENCY_CONFIGURATIONS {
                    rows.push(Row {
                        id: format!("dep:{}:{configuration}", project.path),
                        depth: depth + 2,
                        label: (*configuration).to_owned(),
                        kind: RowKind::Task {
                            path: format!(
                                "{}dependencies --configuration {configuration}",
                                task_prefix(&project.path)
                            ),
                            description: Some(format!("Print the {configuration} dependency tree")),
                        },
                        expandable: false,
                    });
                }
            }
        }
        any |= any_task;
        for child in project.children(model) {
            any |= project_rows(model, child, depth + 1, expanded, filter, rows);
        }
    }
    if !filter.is_empty() && !any {
        rows.truncate(start);
    }
    any || filter.is_empty()
}

pub fn show(app: &mut IdeApp, ctx: &egui::Context) {
    if !app.state.tool_windows.is_visible(ToolWindow::Gradle) {
        return;
    }
    let island = egui::SidePanel::right("right_tool_window")
        .default_width(340.0)
        .min_width(220.0)
        .resizable(true)
        .show_separator_line(false)
        .frame(island_frame(ctx))
        .show(ctx, |ui| content(app, ui));
    crate::chrome::remember_island(ctx, jetthing_core::Anchor::Right, island.response.rect);
}

fn content(app: &mut IdeApp, ui: &mut egui::Ui) {
    if tool_window_header(ui, "Gradle", |_| {}) {
        app.toggle_tool_window(ToolWindow::Gradle);
    }
    let mut view: View = ui.data(|data| data.get_temp(view_id())).unwrap_or_default();
    let syncing = app.state.runner.is_running()
        && app
            .state
            .runner
            .plan()
            .is_some_and(|plan| plan.kind == TaskKind::Sync);
    widgets::toolbar_row(ui, |ui| {
        ui.add_space(theme::layout::TOOLBAR_LEAD);
        ui.spacing_mut().item_spacing.x = 2.0;
        let can_sync = app.commands.is_enabled("gradle.sync", &app.state);
        if syncing {
            ui.add_sized(
                vec2(26.0, 26.0),
                egui::Spinner::new().size(14.0).color(theme::ACCENT),
            );
        } else if ui
            .add_enabled_ui(can_sync, |ui| {
                icons::button(ui, Icon::Refresh, "Sync All Gradle Projects")
            })
            .inner
            .clicked()
        {
            app.dispatch("gradle.sync", Value::Null);
        }
        if icons::button_colored(
            ui,
            Icon::Run,
            theme::RUN_GREEN,
            "Execute Gradle Task",
            view.execute.is_some(),
        )
        .clicked()
        {
            view.execute = match view.execute {
                Some(_) => None,
                None => Some(String::new()),
            };
        }
        ui.add_space(6.0);
        let offline = app
            .state
            .settings
            .bool(jetthing_settings::ids::GRADLE_OFFLINE)
            .unwrap_or(false);
        if ui
            .selectable_label(
                offline,
                RichText::new("Offline").size(theme::UI_FONT_SIZE - 1.0),
            )
            .on_hover_text("Toggle Offline Mode (--offline)")
            .clicked()
        {
            app.dispatch(
                "settings.set",
                json!({"id": jetthing_settings::ids::GRADLE_OFFLINE, "value": !offline}),
            );
        }
        let can_run = app.commands.is_enabled("gradle.runTask", &app.state);
        if ui
            .add_enabled_ui(can_run, |ui| {
                icons::button(ui, Icon::Structure, "Analyze Dependencies")
            })
            .inner
            .clicked()
        {
            app.dispatch("gradle.runTask", json!({"task": "dependencies"}));
        }
        if icons::button(ui, Icon::MainMenu, "Gradle Settings").clicked() {
            app.dispatch(
                "settings.open",
                json!({"page": "reference.settingsdialog.project.gradle"}),
            );
        }
        ui.add_space(6.0);
        if icons::button(ui, Icon::ExpandAll, "Expand All").clicked()
            && let Some(model) = &app.state.gradle_model
        {
            view.expanded = all_ids(model);
        }
        if icons::button(ui, Icon::CollapseAll, "Collapse All").clicked() {
            view.expanded.clear();
        }
    });
    if let Some(mut command) = view.execute.take() {
        let mut keep = true;
        ui.horizontal(|ui| {
            ui.add_space(theme::layout::CONTENT_INSET);
            let id = egui::Id::new("gradle_execute_input");
            let response = search_field(
                ui,
                id,
                &mut command,
                "Tasks and options, e.g. :app:test --offline",
                ui.available_width() - 30.0,
            );
            if !view.execute_focused {
                response.request_focus();
                view.execute_focused = true;
            }
            if response.lost_focus() {
                if ui.input(|input| input.key_pressed(egui::Key::Enter))
                    && !command.trim().is_empty()
                {
                    app.dispatch("gradle.runTask", json!({"task": command.trim()}));
                }
                keep = false;
            }
        });
        if keep {
            view.execute = Some(command);
        } else {
            view.execute_focused = false;
        }
    }

    let Some(model) = app.state.gradle_model.clone() else {
        empty_state(app, ui, syncing);
        ui.data_mut(|data| data.insert_temp(view_id(), view));
        return;
    };
    if !view.initialized {
        view.initialized = true;
        view.expanded.insert("p::".into());
        view.expanded.insert("t::".into());
    }
    widgets::toolbar_row(ui, |ui| {
        ui.add_space(theme::layout::CONTENT_INSET);
        search_field(
            ui,
            egui::Id::new("gradle_filter"),
            &mut view.filter,
            "Filter tasks",
            ui.available_width() - 30.0,
        );
    });
    ui.add_space(4.0);
    let mut run = None;
    egui::ScrollArea::both()
        .id_salt("gradle_tree")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            for row in rows(&model, &view.expanded, &view.filter) {
                let selected = view.selected.as_deref() == Some(row.id.as_str());
                let open = !view.filter.trim().is_empty() || view.expanded.contains(&row.id);
                let icon = match row.kind {
                    RowKind::Project => Icon::Model,
                    RowKind::Folder => Icon::Folder,
                    RowKind::Task { .. } => Icon::Task,
                };
                let response = widgets::tree_row(
                    ui,
                    widgets::TreeRow {
                        depth: row.depth,
                        expanded: row.expandable.then_some(open),
                        selected,
                        icon: Some((icon, *icons::ICON)),
                        ..widgets::TreeRow::new(&row.label)
                    },
                )
                .response;
                let response = match &row.kind {
                    RowKind::Task { path, description } => {
                        response.on_hover_text(match description {
                            Some(description) => format!("{path}\n{description}"),
                            None => path.clone(),
                        })
                    }
                    _ => response,
                };
                if response.clicked() {
                    view.selected = Some(row.id.clone());
                    if row.expandable && !view.expanded.remove(&row.id) {
                        view.expanded.insert(row.id.clone());
                    }
                }
                if response.double_clicked()
                    && let RowKind::Task { path, .. } = &row.kind
                {
                    run = Some(path.clone());
                }
            }
        });
    if let Some(task) = run {
        app.dispatch("gradle.runTask", json!({"task": task}));
    }
    ui.data_mut(|data| data.insert_temp(view_id(), view));
}

fn empty_state(app: &mut IdeApp, ui: &mut egui::Ui, syncing: bool) {
    ui.add_space(16.0);
    ui.vertical_centered(|ui| {
        if !app.state.is_gradle_project() {
            ui.label(RichText::new("This is not a Gradle project").color(theme::TEXT_SECONDARY));
            return;
        }
        if syncing {
            ui.label(RichText::new("Syncing Gradle project…").color(theme::TEXT_SECONDARY));
            return;
        }
        if let Some(error) = &app.state.gradle_error {
            ui.label(RichText::new(error).color(theme::ERROR));
        } else {
            ui.label(
                RichText::new("Gradle project is not synced yet").color(theme::TEXT_SECONDARY),
            );
            ui.label(
                RichText::new("Sync runs the project's Gradle to load projects and tasks.")
                    .color(theme::TEXT_INFO),
            );
        }
        ui.add_space(6.0);
        let enabled = app.commands.is_enabled("gradle.sync", &app.state);
        if ui
            .add_enabled(enabled, egui::Button::new("Sync All Gradle Projects"))
            .clicked()
        {
            app.dispatch("gradle.sync", Value::Null);
        }
    });
}

fn all_ids(model: &GradleModel) -> HashSet<String> {
    let mut ids = HashSet::new();
    for project in &model.projects {
        ids.insert(format!("p:{}", project.path));
        ids.insert(format!("t:{}", project.path));
        ids.insert(format!("d:{}", project.path));
        for group in project.task_groups().keys() {
            ids.insert(format!("g:{}:{group}", project.path));
        }
    }
    ids
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model() -> GradleModel {
        GradleModel::parse(
            r#"{"gradleVersion":"8.14.3","rootDir":"/w","projects":[
                {"path":":","name":"demo","dir":"/w","tasks":[{"name":"help","path":":help","group":"help"}]},
                {"path":":app","name":"app","dir":"/w/app","tasks":[
                    {"name":"run","path":":app:run","group":"application"},
                    {"name":"test","path":":app:test","group":"verification"}]}
            ]}"#,
        )
        .unwrap()
    }

    fn labels(rows: &[Row]) -> Vec<String> {
        rows.iter()
            .map(|row| format!("{}{}", "  ".repeat(row.depth), row.label))
            .collect()
    }

    #[test]
    fn collapsed_tree_shows_only_root() {
        assert_eq!(labels(&rows(&model(), &HashSet::new(), "")), ["demo"]);
    }

    #[test]
    fn expanded_tree_nests_tasks_groups_and_sub_projects() {
        let model = model();
        let rows = rows(&model, &all_ids(&model), "");
        assert_eq!(
            labels(&rows),
            [
                "demo",
                "  Tasks",
                "    help",
                "      help",
                "  app",
                "    Tasks",
                "      application",
                "        run",
                "      verification",
                "        test",
            ]
        );
        assert_eq!(
            rows[7].kind,
            RowKind::Task {
                path: ":app:run".into(),
                description: None
            }
        );
    }

    #[test]
    fn projects_with_a_dependencies_task_get_a_dependencies_node() {
        let model = GradleModel::parse(
            r#"{"gradleVersion":"8.14.3","rootDir":"/w","projects":[
                {"path":":","name":"demo","dir":"/w","tasks":[]},
                {"path":":app","name":"app","dir":"/w/app","tasks":[
                    {"name":"dependencies","path":":app:dependencies","group":"help"}]}
            ]}"#,
        )
        .unwrap();
        let rows = rows(&model, &all_ids(&model), "");
        let labels = labels(&rows);
        assert!(labels.contains(&"  app".to_owned()));
        assert!(labels.contains(&"    Dependencies".to_owned()));
        let report = rows
            .iter()
            .find(|row| row.label == "runtimeClasspath")
            .unwrap();
        assert_eq!(
            report.kind,
            RowKind::Task {
                path: ":app:dependencies --configuration runtimeClasspath".into(),
                description: Some("Print the runtimeClasspath dependency tree".into())
            }
        );
        assert_eq!(task_prefix(":"), "");
        assert!(
            !labels
                .iter()
                .any(|label| label.trim() == "demo Dependencies")
        );
    }

    #[test]
    fn filter_keeps_matching_tasks_and_their_parents() {
        let rows = rows(&model(), &HashSet::new(), "TES");
        assert_eq!(
            labels(&rows),
            [
                "demo",
                "  app",
                "    Tasks",
                "      verification",
                "        test"
            ]
        );
        assert!(super::rows(&model(), &HashSet::new(), "nothing-matches").is_empty());
    }
}
