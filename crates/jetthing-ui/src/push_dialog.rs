//! Push dialog (Ctrl+Shift+K): outgoing commits of the current branch, the
//! remote target, and a force-with-lease option.

use crate::IdeApp;
use crate::dialog::{self, Outcome, Select, Shell};
use archipel_kit::components::{Checkbox, Dropdown, ListRow, Tag, TextField, VirtualList};
use archipel_kit::{Theme, Tone};
use eframe::egui::{self, RichText};
use jetthing_git::PushPlan;
use serde_json::json;

/// What the dialog is showing.
enum Page {
    /// The commits about to be pushed and where they go.
    Plan,
    /// The repository has no remote: a message with the way out.
    NoRemote,
    /// The "Define remote" form.
    DefineRemote {
        name: String,
        url: String,
        error: Option<(Field, String)>,
    },
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Field {
    Name,
    Url,
}

pub struct PushDialog {
    plan: Result<PushPlan, String>,
    page: Page,
    remote: String,
    target: String,
    force: bool,
    /// Focus the first field of the page on its first frame.
    focus: bool,
}

/// Opens the dialog for the current branch (errors are shown inside it).
pub fn open(app: &mut IdeApp) {
    let plan = app.state.push_plan();
    let page = if plan.is_err() && app.state.push_needs_remote() {
        Page::NoRemote
    } else {
        Page::Plan
    };
    app.push_dialog = Some(from_plan(plan, page));
}

fn from_plan(plan: Result<PushPlan, String>, page: Page) -> PushDialog {
    let (remote, target) = plan
        .as_ref()
        .map(|plan| (plan.remote.clone(), plan.target.clone()))
        .unwrap_or_default();
    PushDialog {
        plan,
        page,
        remote,
        target,
        force: false,
        focus: true,
    }
}

pub fn show(app: &mut IdeApp, ctx: &egui::Context) {
    let Some(dialog) = app.push_dialog.as_mut() else {
        return;
    };
    let project = app
        .state
        .project
        .as_ref()
        .and_then(|project| project.root().file_name())
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let title = format!("Push Commits to {project}");
    let focus = std::mem::take(&mut dialog.focus);
    let mut push = false;
    let mut close = false;
    let mut define = false;
    let mut add_remote = None;
    match &mut dialog.page {
        Page::NoRemote => {
            let message = dialog.plan.as_ref().err().cloned().unwrap_or_default();
            let outcome = Shell::new("push_no_remote", title)
                .width(480.0)
                .ok("Define Remote…")
                .cancel(Some("Close"))
                .show(ctx, |ui| {
                    dialog::message(ui, message);
                    dialog::hint(
                        ui,
                        "Define a remote (a name and the URL of the repository) to push your commits to.",
                    );
                });
            match outcome {
                Some(Outcome::Ok) => define = true,
                Some(Outcome::Cancel) => close = true,
                _ => {}
            }
        }
        Page::DefineRemote { name, url, error } => {
            let outcome = Shell::new("push_define_remote", "Define Remote")
                .width(480.0)
                .ok("Add")
                .ok_enabled(!name.trim().is_empty() && !url.trim().is_empty())
                .show(ctx, |ui| {
                    let name_error = error.as_ref().filter(|(f, _)| *f == Field::Name);
                    let response = ui.add(
                        TextField::new(name)
                            .label("Name")
                            .placeholder("origin")
                            .error(name_error.map(|(_, m)| m.as_str())),
                    );
                    if focus {
                        dialog::focus_first(ctx, &response, Select::All);
                    }
                    if response.changed() {
                        *error = None;
                    }
                    let url_error = error.as_ref().filter(|(f, _)| *f == Field::Url);
                    let response = ui.add(
                        TextField::new(url)
                            .label("URL")
                            .placeholder("https://example.com/org/repository.git")
                            .error(url_error.map(|(_, m)| m.as_str())),
                    );
                    if response.changed() {
                        *error = None;
                    }
                });
            match outcome {
                Some(Outcome::Ok) => add_remote = Some((name.clone(), url.clone())),
                Some(Outcome::Cancel) => close = true,
                _ => {}
            }
        }
        Page::Plan => match &dialog.plan {
            Err(message) => {
                let outcome = Shell::new("push_error", title)
                    .width(480.0)
                    .ok("Close")
                    .cancel(None)
                    .show(ctx, |ui| dialog::message(ui, message.clone()));
                close = outcome.is_some();
            }
            Ok(plan) => {
                let enabled = !dialog.target.trim().is_empty()
                    && (!plan.commits.is_empty() || plan.new_branch || dialog.force);
                let outcome = Shell::new("push", title)
                    .width(560.0)
                    .ok("Push")
                    .ok_enabled(enabled)
                    .show(ctx, |ui| {
                        plan_body(
                            ui,
                            plan,
                            &mut dialog.remote,
                            &mut dialog.target,
                            &mut dialog.force,
                        )
                    });
                match outcome {
                    Some(Outcome::Ok) => push = true,
                    Some(Outcome::Cancel) => close = true,
                    _ => {}
                }
            }
        },
    }
    if define {
        dialog.page = Page::DefineRemote {
            name: "origin".into(),
            url: String::new(),
            error: None,
        };
        dialog.focus = true;
    }
    if let Some((name, url)) = add_remote {
        match app.commands.execute(
            "git.addRemote",
            &mut app.state,
            json!({"name": name, "url": url}),
        ) {
            Ok(_) => {
                let plan = app.state.push_plan();
                app.push_dialog = Some(from_plan(plan, Page::Plan));
            }
            Err(error) => {
                let message = error.to_string();
                let message = message
                    .strip_prefix("command failed: ")
                    .unwrap_or(&message)
                    .to_owned();
                let field = if message.starts_with("URL") {
                    Field::Url
                } else {
                    Field::Name
                };
                if let Some(Page::DefineRemote { error, .. }) =
                    app.push_dialog.as_mut().map(|dialog| &mut dialog.page)
                {
                    *error = Some((field, message));
                }
            }
        }
        return;
    }
    if push {
        if let Some(dialog) = app.push_dialog.take() {
            let params = json!({
                "remote": dialog.remote,
                "target": dialog.target.trim(),
                "force": dialog.force,
            });
            app.dispatch("git.push", params);
        }
    } else if close {
        app.push_dialog = None;
    }
}

/// The remote target, the outgoing commits and the force option.
fn plan_body(
    ui: &mut egui::Ui,
    plan: &PushPlan,
    remote: &mut String,
    target: &mut String,
    force: &mut bool,
) {
    let colors = Theme::colors(ui.ctx());
    ui.horizontal(|ui| {
        ui.label(RichText::new("Branch").color(colors.text_secondary));
        ui.label(RichText::new(&plan.branch).color(colors.text_strong));
        if plan.new_branch {
            ui.add(Tag::new("New").tone(Tone::Success));
        }
    });
    ui.horizontal_top(|ui| {
        ui.vertical(|ui| {
            ui.label(RichText::new("Remote").color(colors.text));
            ui.add(
                Dropdown::new("push_remote", remote, plan.remotes.clone())
                    .label("Remote")
                    .width(170.0),
            );
        });
        ui.add(TextField::new(target).label("Target branch").width(240.0));
    });
    dialog::hint(
        ui,
        crate::widgets::pluralize(plan.commits.len(), "commit", "commits"),
    );
    if plan.commits.is_empty() {
        dialog::hint(ui, "Nothing to push");
    } else {
        let mut selected = None;
        VirtualList::new("push_commits", plan.commits.len(), &mut selected)
            .name("Commits to push")
            .height(160.0)
            .show(ui, |index| {
                let commit = &plan.commits[index];
                ListRow::new(&commit.subject)
                    .secondary(format!("{}  {}", commit.short, commit.author))
            });
    }
    ui.add(Checkbox::new(force, "Force push (with lease)"));
}
