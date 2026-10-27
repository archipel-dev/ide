//! Branches popup of the VCS toolbar widget (New UI "Git Branches" popup):
//! repository actions, then local and remote branches, each with a submenu
//! of branch operations. Also hosts the small branch name/confirm dialogs.

use crate::icons::{self, Icon};
use crate::{IdeApp, theme};
use eframe::egui::{self, RichText, vec2};
use serde_json::{Value, json};

/// A dialog opened from the branches popup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitDialog {
    /// Create a branch from `from` (current HEAD when `None`).
    NewBranch {
        from: Option<String>,
        name: String,
        checkout: bool,
        error: Option<String>,
    },
    Rename {
        branch: String,
        name: String,
        error: Option<String>,
    },
    CheckoutRevision {
        revision: String,
        error: Option<String>,
    },
    /// `git branch -d` refused an unmerged branch.
    ForceDelete { branch: String },
    /// Confirm discarding local changes of project-relative paths.
    Rollback { paths: Vec<String> },
    Stash {
        message: String,
        keep_index: bool,
        include_untracked: bool,
        error: Option<String>,
    },
    Unstash {
        selected: usize,
        pop: bool,
        error: Option<String>,
    },
}

fn input_id() -> egui::Id {
    egui::Id::new("git_dialog_input")
}

/// Contents of the VCS widget popup.
pub fn menu(app: &mut IdeApp, ui: &mut egui::Ui) {
    let Some(git) = app.state.git_status() else {
        return;
    };
    let current = git.branch.clone();
    if !app.state.git_log_snapshot().loaded {
        // Branch lists come from the log state; load them on first open.
        app.dispatch("git.log.refresh", Value::Null);
    }
    let branches = app.state.git_log_snapshot().branches.clone();
    let pending = app.state.git_pending();
    ui.set_min_width(260.0);
    let mut action: Option<(&'static str, Value)> = None;
    let mut dialog = None;
    let mut push = false;
    if let Some(operation) = pending {
        ui.label(RichText::new(format!("{} in progress", operation.label())).color(theme::WARNING));
        if ui
            .button(archipel_ide_core::abort_label(operation))
            .clicked()
        {
            action = Some(("git.abortOperation", Value::Null));
        }
        ui.separator();
    }
    let busy = app.state.remote_busy();
    let query_id = egui::Id::new("branches_popup_search");
    let mut query: String = ui.data_mut(|data| data.get_temp(query_id).unwrap_or_default());
    let search = ui.add(
        egui::TextEdit::singleline(&mut query)
            .id(query_id.with("edit"))
            .hint_text("Search")
            .desired_width(f32::INFINITY)
            .margin(egui::Margin::symmetric(6, 5)),
    );
    if !search.has_focus() && query.is_empty() {
        search.request_focus();
    }
    ui.data_mut(|data| data.insert_temp(query_id, query.clone()));
    let needle = query.to_lowercase();
    let matches = |name: &str| needle.is_empty() || name.to_lowercase().contains(&needle);
    if ui.button("New Branch…").clicked() {
        dialog = Some(GitDialog::NewBranch {
            from: None,
            name: String::new(),
            checkout: true,
            error: None,
        });
    }
    if ui.button("Checkout Tag or Revision…").clicked() {
        dialog = Some(GitDialog::CheckoutRevision {
            revision: String::new(),
            error: None,
        });
    }
    if ui.add_enabled(!busy, egui::Button::new("Fetch")).clicked() {
        action = Some(("git.fetch", Value::Null));
    }
    if let Some(progress) = &app.state.remote_progress() {
        ui.label(RichText::new(progress).color(theme::TEXT_SECONDARY));
    }
    ui.separator();
    section(ui, "Local");
    for branch in branches.local.iter().filter(|branch| matches(&branch.name)) {
        let is_current = branch.name == current;
        let mut label = branch.name.clone();
        if branch.ahead > 0 {
            label.push_str(&format!("  ↑{}", branch.ahead));
        }
        if branch.behind > 0 {
            label.push_str(&format!("  ↓{}", branch.behind));
        }
        let text = if is_current {
            RichText::new(label).color(theme::TEXT_BRIGHT).strong()
        } else {
            RichText::new(label)
        };
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            icons::show(
                ui,
                Icon::Branch,
                14.0,
                if is_current {
                    theme::WARNING
                } else {
                    theme::TEXT_SECONDARY
                },
            );
            ui.menu_button(text, |ui| {
                ui.set_min_width(240.0);
                branch_actions(
                    ui,
                    &branch.name,
                    &current,
                    false,
                    &mut action,
                    &mut dialog,
                    &mut push,
                );
            });
        });
    }
    let remote: Vec<&String> = branches
        .remote
        .iter()
        .filter(|name| matches(name))
        .collect();
    if !remote.is_empty() {
        ui.separator();
        section(ui, "Remote");
        for branch in remote {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                icons::show(ui, Icon::Branch, 14.0, theme::TEXT_SECONDARY);
                ui.menu_button(branch.as_str(), |ui| {
                    ui.set_min_width(240.0);
                    branch_actions(
                        ui,
                        branch,
                        &current,
                        true,
                        &mut action,
                        &mut dialog,
                        &mut push,
                    );
                });
            });
        }
    }
    if action.is_some() || dialog.is_some() || push {
        ui.close();
    }
    if push {
        crate::push_dialog::open(app);
    }
    if let Some(dialog) = dialog {
        app.git_dialog = Some(dialog);
        app.dialog_focus = true;
    }
    if let Some((id, params)) = action {
        run(app, id, params);
    }
}

fn section(ui: &mut egui::Ui, title: &str) {
    ui.label(
        RichText::new(title)
            .color(theme::TEXT_SECONDARY)
            .size(theme::UI_FONT_SIZE - 1.0),
    );
}

/// Items of a branch's submenu.
fn branch_actions(
    ui: &mut egui::Ui,
    branch: &str,
    current: &str,
    remote: bool,
    action: &mut Option<(&'static str, Value)>,
    dialog: &mut Option<GitDialog>,
    push: &mut bool,
) {
    let is_current = branch == current;
    if !is_current && ui.button("Checkout").clicked() {
        *action = Some(("git.checkout", json!({ "branch": branch })));
    }
    if ui.button(format!("New Branch from '{branch}'…")).clicked() {
        *dialog = Some(GitDialog::NewBranch {
            from: Some(branch.to_owned()),
            name: String::new(),
            checkout: true,
            error: None,
        });
    }
    if is_current && !remote {
        ui.separator();
        if ui.button("Update").clicked() {
            *action = Some(("git.pull", Value::Null));
        }
        if ui.button("Push…").clicked() {
            *push = true;
        }
    }
    if !is_current {
        ui.separator();
        if ui
            .button(format!("Merge '{branch}' into '{current}'"))
            .clicked()
        {
            *action = Some(("git.merge", json!({ "branch": branch })));
        }
        if ui
            .button(format!("Rebase '{current}' onto '{branch}'"))
            .clicked()
        {
            *action = Some(("git.rebase", json!({ "onto": branch })));
        }
    }
    if !remote {
        ui.separator();
        if ui.button("Rename…").clicked() {
            *dialog = Some(GitDialog::Rename {
                branch: branch.to_owned(),
                name: branch.to_owned(),
                error: None,
            });
        }
        if !is_current && ui.button("Delete").clicked() {
            *action = Some(("git.deleteBranch", json!({ "branch": branch })));
        }
    }
}

/// Runs a branch command; an unmerged branch delete turns into a confirm.
fn run(app: &mut IdeApp, id: &str, params: Value) {
    match app.commands.execute(id, &mut app.state, params.clone()) {
        Ok(_) => {}
        Err(error) => {
            let message = error.to_string();
            if id == "git.deleteBranch" && message.contains("not fully merged") {
                app.git_dialog = Some(GitDialog::ForceDelete {
                    branch: params["branch"].as_str().unwrap_or_default().to_owned(),
                });
            }
            app.state.status = message;
        }
    }
}

fn frame(ctx: &egui::Context, title: &str, add: impl FnOnce(&mut egui::Ui)) {
    let screen = ctx.content_rect();
    let width = 380.0;
    egui::Area::new(egui::Id::new("git_dialog"))
        .order(egui::Order::Foreground)
        .fixed_pos(egui::pos2(
            screen.center().x - width / 2.0,
            screen.top() + screen.height() * 0.2,
        ))
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style())
                .corner_radius(10)
                .inner_margin(egui::Margin::same(12))
                .show(ui, |ui| {
                    ui.set_width(width - 24.0);
                    ui.label(
                        RichText::new(title)
                            .font(theme::semibold(theme::UI_FONT_SIZE))
                            .color(theme::TEXT_BRIGHT),
                    );
                    ui.add_space(6.0);
                    add(ui);
                });
        });
}

/// A text field that reports Enter; returns (submitted, response).
fn field(ui: &mut egui::Ui, text: &mut String, focus: bool) -> bool {
    let submit = ui.memory(|memory| memory.has_focus(input_id()))
        && ui.input(|input| input.key_pressed(egui::Key::Enter));
    let response = ui.add(
        egui::TextEdit::singleline(text)
            .id(input_id())
            .desired_width(f32::INFINITY)
            .margin(egui::Margin::symmetric(6, 5)),
    );
    if focus {
        response.request_focus();
    }
    submit
}

fn error_line(ui: &mut egui::Ui, error: &Option<String>) {
    if let Some(error) = error {
        ui.add_space(4.0);
        ui.label(RichText::new(error).color(theme::ERROR));
    }
}

fn buttons(ui: &mut egui::Ui, primary: &str) -> (bool, bool) {
    ui.add_space(8.0);
    let mut ok = false;
    let mut cancel = false;
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        let button = egui::Button::new(RichText::new(primary).color(egui::Color32::WHITE))
            .fill(theme::ACCENT)
            .min_size(vec2(72.0, 26.0));
        ok = ui.add(button).clicked();
        cancel = ui.button("Cancel").clicked();
    });
    (ok, cancel)
}

/// Draws the open branch dialog, if any.
pub fn show_dialog(app: &mut IdeApp, ctx: &egui::Context) {
    let Some(mut dialog) = app.git_dialog.take() else {
        return;
    };
    let focus = std::mem::take(&mut app.dialog_focus);
    let mut keep = true;
    let mut command: Option<(&'static str, Value)> = None;
    match &mut dialog {
        GitDialog::NewBranch {
            from,
            name,
            checkout,
            error,
        } => {
            let title = match from {
                Some(from) => format!("Create New Branch from {from}"),
                None => "Create New Branch".to_owned(),
            };
            frame(ctx, &title, |ui| {
                let submit = field(ui, name, focus);
                ui.checkbox(checkout, "Checkout branch");
                error_line(ui, error);
                let (ok, cancel) = buttons(ui, "Create");
                keep = !cancel;
                if (ok || submit) && !name.trim().is_empty() {
                    command = Some((
                        "git.newBranch",
                        json!({"name": name.trim(), "from": from, "checkout": *checkout}),
                    ));
                }
            });
        }
        GitDialog::Rename {
            branch,
            name,
            error,
        } => {
            frame(ctx, &format!("Rename {branch}"), |ui| {
                let submit = field(ui, name, focus);
                error_line(ui, error);
                let (ok, cancel) = buttons(ui, "Rename");
                keep = !cancel;
                if (ok || submit) && !name.trim().is_empty() && name.trim() != branch {
                    command = Some((
                        "git.renameBranch",
                        json!({"branch": branch, "name": name.trim()}),
                    ));
                }
            });
        }
        GitDialog::CheckoutRevision { revision, error } => {
            frame(ctx, "Checkout Tag or Revision", |ui| {
                let submit = field(ui, revision, focus);
                error_line(ui, error);
                let (ok, cancel) = buttons(ui, "Checkout");
                keep = !cancel;
                if (ok || submit) && !revision.trim().is_empty() {
                    command = Some(("git.checkout", json!({"branch": revision.trim()})));
                }
            });
        }
        GitDialog::ForceDelete { branch } => {
            frame(ctx, "Delete Branch", |ui| {
                ui.label(format!(
                    "Branch {branch} is not fully merged. Its commits will only be \
                     reachable by hash. Delete it anyway?"
                ));
                let (ok, cancel) = buttons(ui, "Delete");
                keep = !cancel;
                if ok {
                    command = Some(("git.deleteBranch", json!({"branch": branch, "force": true})));
                }
            });
        }
        GitDialog::Rollback { paths } => {
            let title = format!(
                "Rollback {}",
                crate::widgets::pluralize(paths.len(), "File", "Files")
            );
            frame(ctx, &title, |ui| {
                ui.label("Local changes of these files will be discarded:");
                ui.add_space(4.0);
                egui::ScrollArea::vertical()
                    .max_height(180.0)
                    .show(ui, |ui| {
                        for path in paths.iter() {
                            ui.label(RichText::new(path).color(theme::TEXT_BRIGHT));
                        }
                    });
                ui.label(
                    RichText::new("Newly added files are kept on disk as unversioned files.")
                        .color(theme::TEXT_SECONDARY),
                );
                let (ok, cancel) = buttons(ui, "Rollback");
                keep = !cancel;
                if ok {
                    command = Some(("git.rollback", json!({ "paths": paths })));
                }
            });
        }
        GitDialog::Stash {
            message,
            keep_index,
            include_untracked,
            error,
        } => {
            frame(ctx, "Stash Changes", |ui| {
                ui.label(RichText::new("Message").color(theme::TEXT_SECONDARY));
                let submit = field(ui, message, focus);
                ui.checkbox(keep_index, "Keep index");
                ui.checkbox(include_untracked, "Include unversioned files");
                error_line(ui, error);
                let (ok, cancel) = buttons(ui, "Create Stash");
                keep = !cancel;
                if ok || submit {
                    command = Some((
                        "git.stash",
                        json!({
                            "message": message.trim(),
                            "keepIndex": *keep_index,
                            "includeUntracked": *include_untracked,
                        }),
                    ));
                }
            });
        }
        GitDialog::Unstash {
            selected,
            pop,
            error,
        } => {
            let stashes = app.state.stashes();
            frame(ctx, "Unstash Changes", |ui| {
                if stashes.is_empty() {
                    ui.label(RichText::new("There are no stashes").color(theme::TEXT_SECONDARY));
                }
                egui::ScrollArea::vertical()
                    .max_height(200.0)
                    .show(ui, |ui| {
                        for entry in &stashes {
                            let label = format!("stash@{{{}}}  {}", entry.index, entry.message);
                            if ui
                                .selectable_label(*selected == entry.index, label)
                                .clicked()
                            {
                                *selected = entry.index;
                            }
                        }
                    });
                ui.checkbox(pop, "Pop stash (drop it after applying)");
                error_line(ui, error);
                ui.add_space(8.0);
                let mut cancel = false;
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let enabled = *selected < stashes.len();
                    let apply =
                        egui::Button::new(RichText::new("Apply Stash").color(egui::Color32::WHITE))
                            .fill(theme::ACCENT)
                            .min_size(vec2(72.0, 26.0));
                    if ui.add_enabled(enabled, apply).clicked() {
                        let id = if *pop {
                            "git.stash.pop"
                        } else {
                            "git.stash.apply"
                        };
                        command = Some((id, json!({ "index": *selected })));
                    }
                    cancel = ui.button("Cancel").clicked();
                    if ui.add_enabled(enabled, egui::Button::new("Drop")).clicked() {
                        command = Some(("git.stash.drop", json!({ "index": *selected })));
                    }
                });
                keep = !cancel;
            });
        }
    }
    if let Some((id, params)) = command {
        match app.commands.execute(id, &mut app.state, params) {
            Ok(_) if id == "git.stash.drop" => {
                if let GitDialog::Unstash { selected, .. } = &mut dialog {
                    *selected = 0;
                }
            }
            Ok(_) => keep = false,
            Err(failure) => {
                let message = failure.to_string();
                match &mut dialog {
                    GitDialog::NewBranch { error, .. }
                    | GitDialog::Rename { error, .. }
                    | GitDialog::CheckoutRevision { error, .. }
                    | GitDialog::Stash { error, .. }
                    | GitDialog::Unstash { error, .. } => {
                        *error = Some(message.clone());
                    }
                    GitDialog::ForceDelete { .. } | GitDialog::Rollback { .. } => keep = false,
                }
                app.state.status = message;
            }
        }
    }
    if keep {
        app.git_dialog = Some(dialog);
    }
}
