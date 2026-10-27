//! Test runner tree ("Test Results" → suites → cases) and the selected test's details.

use crate::build_window::format_duration;
use crate::icons::{self, Icon};
use crate::widgets;
use crate::{IdeApp, theme};
use archipel_ide_runner::{TestCase, TestOutcome, TestResults};
use eframe::egui::{self, Color32, RichText, vec2};
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Clone, Default)]
struct View {
    /// (suite, case); a suite alone when `case` is empty.
    selected: Option<(String, String)>,
    hide_passed: bool,
    hide_ignored: bool,
}

fn view_id() -> egui::Id {
    egui::Id::new("test_view")
}

/// Summary line of the console header, e.g. `1 test passed` / `2 tests failed, 3 passed`.
pub fn summary(results: &TestResults) -> (String, Color32) {
    let counts = results.counts();
    let tests = |n: usize| if n == 1 { "test" } else { "tests" };
    if counts.failed > 0 {
        let mut text = format!("{} {} failed", counts.failed, tests(counts.failed));
        if counts.passed > 0 {
            text.push_str(&format!(", {} passed", counts.passed));
        }
        if counts.skipped > 0 {
            text.push_str(&format!(", {} ignored", counts.skipped));
        }
        (text, *theme::ERROR)
    } else if counts.passed > 0 {
        let mut text = format!("{} {} passed", counts.passed, tests(counts.passed));
        if counts.skipped > 0 {
            text.push_str(&format!(", {} ignored", counts.skipped));
        }
        (text, *theme::SUCCESS)
    } else if counts.skipped > 0 {
        (
            format!("{} {} ignored", counts.skipped, tests(counts.skipped)),
            *theme::WARNING,
        )
    } else {
        ("No tests were found".into(), *theme::TEXT_SECONDARY)
    }
}

fn outcome_icon(failed: bool, skipped: bool) -> (Icon, Color32) {
    if failed {
        (Icon::Error, *theme::ERROR)
    } else if skipped {
        (Icon::Warning, *theme::WARNING)
    } else {
        (Icon::Success, *theme::SUCCESS)
    }
}

fn duration_text(duration: Option<Duration>) -> String {
    duration.map(format_duration).unwrap_or_default()
}

/// Paints the tree; returns whether a toolbar toggle should be drawn as active.
pub fn tree(app: &IdeApp, ui: &mut egui::Ui, running: bool) {
    let mut view: View = ui.data(|data| data.get_temp(view_id())).unwrap_or_default();
    let execution = app.state.execution();
    let runner = execution.runner();
    let results = runner.tests();
    widgets::toolbar_row(ui, |ui| {
        ui.add_space(theme::layout::TOOLBAR_LEAD);
        ui.spacing_mut().item_spacing.x = 2.0;
        let mut show_passed = !view.hide_passed;
        if toggle(
            ui,
            Icon::Success,
            *theme::SUCCESS,
            "Show Passed",
            &mut show_passed,
        ) {
            view.hide_passed = !show_passed;
        }
        let mut show_ignored = !view.hide_ignored;
        if toggle(
            ui,
            Icon::Warning,
            *theme::WARNING,
            "Show Ignored",
            &mut show_ignored,
        ) {
            view.hide_ignored = !show_ignored;
        }
    });
    egui::ScrollArea::both()
        .id_salt("test_tree")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            let counts = results.counts();
            let (icon, color) = outcome_icon(counts.failed > 0, counts.total() == 0);
            let out = widgets::tree_row(
                ui,
                widgets::TreeRow {
                    selected: view.selected.is_none(),
                    icon: (!running).then_some((icon, color)),
                    reserve_icon: running,
                    color: *theme::TEXT_BRIGHT,
                    ..widgets::TreeRow::new("Test Results")
                },
            );
            if running {
                let mut child = ui.new_child(egui::UiBuilder::new().max_rect(
                    egui::Rect::from_center_size(out.icon.center(), vec2(12.0, 12.0)),
                ));
                widgets::busy_spinner(&mut child, 12.0, *theme::ACCENT);
            }
            let (rect, response) = (out.rect, out.response);
            right_text(ui, rect, &duration_text(results.duration()));
            if response.clicked() {
                view.selected = None;
            }
            for suite in results.suites() {
                if !suite
                    .cases
                    .iter()
                    .any(|case| visible(case, view.hide_passed, view.hide_ignored))
                {
                    continue;
                }
                let selected = view
                    .selected
                    .as_ref()
                    .is_some_and(|(s, c)| s == &suite.name && c.is_empty());
                let skipped = suite
                    .cases
                    .iter()
                    .all(|case| case.outcome == TestOutcome::Skipped);
                let (icon, color) = outcome_icon(suite.failed(), skipped);
                let out = widgets::tree_row(
                    ui,
                    widgets::TreeRow {
                        depth: 1,
                        selected,
                        icon: Some((icon, color)),
                        ..widgets::TreeRow::new(short_suite(&suite.name))
                    },
                );
                let (rect, response) = (out.rect, out.response);
                right_text(ui, rect, &duration_text(suite.duration()));
                let response = response.on_hover_text(&suite.name);
                if response.clicked() {
                    view.selected = Some((suite.name.clone(), String::new()));
                }
                for case in &suite.cases {
                    if view.hide_passed && case.outcome == TestOutcome::Passed {
                        continue;
                    }
                    if view.hide_ignored && case.outcome == TestOutcome::Skipped {
                        continue;
                    }
                    let selected = view
                        .selected
                        .as_ref()
                        .is_some_and(|(s, c)| s == &suite.name && c == &case.name);
                    let (icon, color) =
                        outcome_icon(case.failed(), case.outcome == TestOutcome::Skipped);
                    let out = widgets::tree_row(
                        ui,
                        widgets::TreeRow {
                            depth: 2,
                            selected,
                            icon: Some((icon, color)),
                            ..widgets::TreeRow::new(&case.name)
                        },
                    );
                    let (rect, response) = (out.rect, out.response);
                    right_text(ui, rect, &duration_text(case.duration));
                    if response.clicked() {
                        view.selected = Some((suite.name.clone(), case.name.clone()));
                    }
                }
            }
        });
    ui.data_mut(|data| data.insert_temp(view_id(), view));
}

/// Whether the Show Passed / Show Ignored toggles let `case` through.
fn visible(case: &TestCase, hide_passed: bool, hide_ignored: bool) -> bool {
    match case.outcome {
        TestOutcome::Passed => !hide_passed,
        TestOutcome::Skipped => !hide_ignored,
        TestOutcome::Failed { .. } => true,
    }
}

/// A toolbar toggle button; returns whether it was clicked (flipping `on`).
fn toggle(ui: &mut egui::Ui, icon: Icon, color: Color32, tip: &str, on: &mut bool) -> bool {
    let tint = if *on { color } else { *theme::TEXT_DISABLED };
    let response = icons::button_colored(ui, icon, tint, tip, *on);
    let clicked = response.clicked();
    if clicked {
        *on = !*on;
    }
    clicked
}

fn right_text(ui: &egui::Ui, rect: egui::Rect, text: &str) {
    if text.is_empty() {
        return;
    }
    let font = egui::FontId::proportional(theme::UI_FONT_SIZE - 1.0);
    let galley = ui
        .painter()
        .layout_no_wrap(text.to_owned(), font, *theme::TEXT_INFO);
    let pos = egui::pos2(
        rect.right() - galley.size().x - 10.0,
        rect.center().y - galley.size().y / 2.0,
    );
    ui.painter().galley(pos, galley, *theme::TEXT_INFO);
}

/// `com.example.AppTest` → `AppTest` (the tree shows simple class names).
fn short_suite(name: &str) -> &str {
    if name.starts_with("//") || name.contains('/') {
        return name;
    }
    name.rsplit(['.', ':'])
        .find(|part| !part.is_empty())
        .unwrap_or(name)
}

/// The selected test case, if any.
pub fn selected_case(app: &IdeApp, ctx: &egui::Context) -> Option<TestCase> {
    let view: View = ctx
        .data(|data| data.get_temp(view_id()))
        .unwrap_or_default();
    let (suite, case) = view.selected?;
    app.state
        .execution()
        .runner()
        .tests()
        .suites()
        .iter()
        .find(|s| s.name == suite)?
        .cases
        .iter()
        .find(|c| c.name == case)
        .cloned()
}

/// Header above the console: status icon, summary, total count and time.
pub fn header(app: &IdeApp, ui: &mut egui::Ui) {
    let execution = app.state.execution();
    let runner = execution.runner();
    let results = runner.tests();
    let (text, color) = summary(results);
    ui.horizontal(|ui| {
        let counts = results.counts();
        let (icon, icon_color) = outcome_icon(counts.failed > 0, counts.total() == 0);
        icons::show(ui, icon, 14.0, icon_color);
        ui.label(RichText::new(text).color(color));
        ui.label(
            RichText::new(format!(
                "{} {} total{}",
                counts.total(),
                if counts.total() == 1 { "test" } else { "tests" },
                results
                    .duration()
                    .map(|d| format!(", {}", format_duration(d)))
                    .unwrap_or_default()
            ))
            .color(*theme::TEXT_INFO),
        );
    });
    let progress = ui.available_rect_before_wrap();
    let bar = egui::Rect::from_min_size(progress.min, vec2(progress.width(), 2.0));
    let counts = results.counts();
    let color = if counts.failed > 0 {
        *theme::ERROR
    } else {
        *theme::SUCCESS
    };
    ui.painter().rect_filled(bar, 0.0, color);
    ui.add_space(6.0);
}

/// A stack frame `at pkg.Class.method(File.java:12)` → (`pkg/Class` path suffix without
/// extension, file name, 1-based line).
fn parse_frame(line: &str) -> Option<(String, String, usize)> {
    let rest = line.trim().strip_prefix("at ")?;
    let (callee, location) = rest.split_once('(')?;
    let (file, line_no) = location.strip_suffix(')')?.rsplit_once(':')?;
    let line_no: usize = line_no.parse().ok()?;
    let class = callee.rsplit_once('.')?.0;
    let class = class.split('$').next()?.replace('.', "/");
    // Module prefixes such as `app//com.example.A.m` are dropped.
    let class = class.rsplit("//").next()?.to_owned();
    (!file.is_empty() && line_no > 0).then(|| (class, file.to_owned(), line_no))
}

/// Finds `<class path>`'s sibling `file` below `root`, skipping build output.
fn find_source(root: &Path, class: &str, file: &str) -> Option<PathBuf> {
    let package = class
        .rsplit_once('/')
        .map(|(package, _)| package)
        .unwrap_or("");
    let suffix = Path::new(package).join(file);
    let mut pending = vec![root.to_owned()];
    let mut visited = 0;
    while let Some(dir) = pending.pop() {
        visited += 1;
        if visited > 5000 {
            return None;
        }
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if path.is_dir() {
                if !matches!(
                    name.as_ref(),
                    "build" | "target" | "node_modules" | ".git" | ".gradle" | "out"
                ) {
                    pending.push(path);
                }
            } else if name == file && path.ends_with(&suffix) {
                return Some(path);
            }
        }
    }
    None
}

/// Failure message, stack trace and captured output of a test. Returns the source
/// location of a clicked stack frame.
pub fn details(
    ui: &mut egui::Ui,
    case: &TestCase,
    root: Option<&Path>,
) -> Option<(PathBuf, usize)> {
    let mut opened = None;
    egui::ScrollArea::both()
        .id_salt("test_details")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let mono = egui::FontId::monospace(theme::UI_FONT_SIZE - 0.5);
            match &case.outcome {
                TestOutcome::Failed { message, details } => {
                    ui.label(
                        RichText::new(message)
                            .font(mono.clone())
                            .color(*theme::ERROR),
                    );
                    if !details.trim().is_empty() && details.trim() != message.trim() {
                        for line in details.trim_end().lines() {
                            let frame = parse_frame(line);
                            let text = RichText::new(line).font(mono.clone());
                            if let Some((class, file, line_no)) = frame {
                                let link = ui
                                    .add(
                                        egui::Label::new(text.color(*theme::ACCENT))
                                            .sense(egui::Sense::click()),
                                    )
                                    .on_hover_cursor(egui::CursorIcon::PointingHand);
                                if link.clicked()
                                    && let Some(path) =
                                        root.and_then(|root| find_source(root, &class, &file))
                                {
                                    opened = Some((path, line_no));
                                }
                            } else {
                                ui.label(text.color(*theme::editor::TEXT));
                            }
                        }
                    }
                }
                TestOutcome::Skipped => {
                    ui.label(RichText::new("Test ignored").color(*theme::WARNING));
                }
                TestOutcome::Passed => {
                    ui.label(RichText::new("Test passed").color(*theme::SUCCESS));
                }
            }
            if !case.output.trim().is_empty() {
                ui.add_space(8.0);
                ui.label(
                    RichText::new(case.output.trim_end())
                        .font(mono)
                        .color(*theme::editor::TEXT),
                );
            }
        });
    opened
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summaries_read_like_the_reference() {
        let passed = TestResults::parse_junit(
            r#"<testsuite name="A"><testcase classname="A" name="greets()" time="0.021"/></testsuite>"#,
        )
        .unwrap();
        assert_eq!(summary(&passed).0, "1 test passed");
        let mixed = TestResults::parse_junit(
            r#"<testsuite name="A"><testcase classname="A" name="a"/><testcase classname="A" name="b"><failure/></testcase><testcase classname="A" name="c"><skipped/></testcase></testsuite>"#,
        )
        .unwrap();
        assert_eq!(summary(&mixed).0, "1 test failed, 1 passed, 1 ignored");
        assert_eq!(summary(&TestResults::default()).0, "No tests were found");
    }

    #[test]
    fn stack_frames_resolve_to_files_and_lines() {
        assert_eq!(
            parse_frame("\tat com.example.AppTest.greets(AppTest.java:12)"),
            Some((
                "com/example".to_owned() + "/AppTest",
                "AppTest.java".into(),
                12
            ))
        );
        assert_eq!(
            parse_frame("\tat java.base/java.lang.Thread.run(Native Method)"),
            None
        );
        assert_eq!(parse_frame("Caused by: x"), None);
        let case = |outcome| TestCase {
            name: "t".into(),
            outcome,
            duration: None,
            output: String::new(),
        };
        assert!(!visible(&case(TestOutcome::Passed), true, false));
        assert!(!visible(&case(TestOutcome::Skipped), false, true));
        assert!(visible(&case(TestOutcome::Skipped), true, false));
    }

    #[test]
    fn suite_names_are_shortened_like_class_names() {
        assert_eq!(short_suite("com.example.AppTest"), "AppTest");
        assert_eq!(short_suite("tests::math"), "math");
        assert_eq!(short_suite("//:app_test"), "//:app_test");
        assert_eq!(short_suite("example.com/calc"), "example.com/calc");
    }
}
