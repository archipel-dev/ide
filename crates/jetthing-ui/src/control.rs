//! UI automation over the opt-in control socket (`JETTHING_CONTROL_SOCKET`).
//!
//! Requests are drained in `raw_input_hook`, before the frame runs: `key` and
//! `type` are injected as ordinary egui input events, so they travel the same
//! shortcut and focus paths as real keystrokes. Answers that depend on the
//! frame (input, commands, waits) are sent after `update`; screenshots are
//! answered when the renderer delivers the captured image.

use crate::{IdeApp, editor, findbar, toolwindows};
use eframe::egui;
use jetthing_command::CommandError;
use jetthing_control::{
    Chord, ControlServer, Pending, PointerButton, Request, Response, WaitCondition,
};
use jetthing_core::{Anchor, ToolWindow};
use serde_json::{Value, json};
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::time::Instant;

/// Frames a `wait idle` needs without injected input, so focus changes and
/// layout settle before a screenshot.
const SETTLE_FRAMES: u32 = 3;

pub struct Control {
    server: ControlServer,
    /// Answered once the current frame has run.
    after_frame: Vec<(Pending, Response)>,
    waits: Vec<(Pending, Instant)>,
    screenshots: HashMap<u64, (Pending, PathBuf)>,
    next_screenshot: u64,
    frames_since_input: u32,
    /// Injected input, one entry per frame; gestures such as Double Shift
    /// span several frames.
    input: VecDeque<InputFrame>,
    /// Set by the first injected input: from then on the window counts as
    /// focused, so injected keys reach widgets even when no window manager
    /// gives the window OS focus (Xvfb, CI).
    owns_focus: bool,
}

/// Input applied during one frame; `done` is answered after its frame ran.
pub(crate) struct InputFrame {
    modifiers: egui::Modifiers,
    events: Vec<egui::Event>,
    done: Option<Pending>,
}

impl Control {
    pub fn new(server: ControlServer) -> Self {
        Self {
            server,
            after_frame: Vec::new(),
            waits: Vec::new(),
            screenshots: HashMap::new(),
            next_screenshot: 0,
            frames_since_input: 0,
            input: VecDeque::new(),
            owns_focus: false,
        }
    }

    fn busy(&self) -> bool {
        !self.after_frame.is_empty()
            || !self.waits.is_empty()
            || !self.screenshots.is_empty()
            || !self.input.is_empty()
    }
}

impl IdeApp {
    /// Starts listening when `JETTHING_CONTROL_SOCKET` names a socket path.
    /// A failure is reported on stderr and in the status bar; the IDE keeps
    /// running without automation.
    pub fn start_control_from_env(&mut self, ctx: &egui::Context) {
        let Some(path) = std::env::var_os(jetthing_control::SOCKET_ENV) else {
            return;
        };
        let waker = ctx.clone();
        match ControlServer::bind(PathBuf::from(&path).as_path(), move || {
            waker.request_repaint()
        }) {
            Ok(server) => self.control = Some(Control::new(server)),
            Err(error) => {
                let message = format!(
                    "Control socket {} unavailable: {error}",
                    PathBuf::from(path).display()
                );
                eprintln!("jetthing: {message}");
                self.state.status = message;
            }
        }
    }

    /// Takes queued requests and screenshot replies; runs before each frame.
    pub(crate) fn control_before_frame(
        &mut self,
        ctx: &egui::Context,
        raw_input: &mut egui::RawInput,
    ) {
        let Some(mut control) = self.control.take() else {
            return;
        };
        for event in &raw_input.events {
            if let egui::Event::Screenshot {
                user_data, image, ..
            } = event
                && let Some(id) = user_data
                    .data
                    .as_ref()
                    .and_then(|data| data.downcast_ref::<u64>())
                && let Some((pending, path)) = control.screenshots.remove(id)
            {
                let [width, height] = image.size;
                let response = match jetthing_control::write_png(
                    &path,
                    width as u32,
                    height as u32,
                    image.as_raw(),
                ) {
                    Ok(()) => Response::ok(json!({
                        "path": path,
                        "width": width,
                        "height": height,
                    })),
                    Err(error) => Response::error(error.to_string()),
                };
                pending.respond(response);
            }
        }
        while let Some(pending) = control.server.try_recv() {
            match pending.request.clone() {
                Request::Execute { command, args } => {
                    let response = self.control_execute(ctx, &command, args);
                    control.after_frame.push((pending, response));
                }
                Request::State => {
                    let state = self.control_state(ctx);
                    pending.respond(Response::ok(state));
                }
                Request::Key { chord } => match key_frames(&chord) {
                    Ok(mut frames) => {
                        if let Some(last) = frames.last_mut() {
                            last.done = Some(pending);
                        }
                        control.input.extend(frames);
                    }
                    Err(error) => pending.respond(Response::error(error)),
                },
                Request::Type { text } => control.input.push_back(InputFrame {
                    modifiers: egui::Modifiers::NONE,
                    events: vec![egui::Event::Text(text)],
                    done: Some(pending),
                }),
                Request::Click {
                    x,
                    y,
                    button,
                    count,
                } => {
                    let mut frames = click_frames(egui::pos2(x, y), button, count);
                    if let Some(last) = frames.last_mut() {
                        last.done = Some(pending);
                    }
                    control.input.extend(frames);
                }
                Request::Move { x, y } => control.input.push_back(InputFrame {
                    modifiers: egui::Modifiers::NONE,
                    events: vec![egui::Event::PointerMoved(egui::pos2(x, y))],
                    done: Some(pending),
                }),
                Request::Screenshot { path } => {
                    let id = control.next_screenshot;
                    control.next_screenshot += 1;
                    control.screenshots.insert(id, (pending, path));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::new(
                        id,
                    )));
                }
                Request::Wait {
                    until, ref text, ..
                } => {
                    if until == WaitCondition::Text && text.is_none() {
                        pending.respond(Response::error("wait until=text needs \"text\""));
                    } else {
                        let deadline = Instant::now() + pending.request.wait_timeout();
                        control.waits.push((pending, deadline));
                    }
                }
            }
        }
        if let Some(frame) = control.input.pop_front() {
            control.owns_focus = true;
            raw_input.modifiers = frame.modifiers;
            raw_input.events.extend(frame.events);
            control.frames_since_input = 0;
            if let Some(pending) = frame.done {
                control
                    .after_frame
                    .push((pending, Response::ok(Value::Null)));
            }
        }
        if control.owns_focus {
            raw_input.focused = true;
        }
        if control.busy() {
            ctx.request_repaint();
        }
        self.control = Some(control);
    }

    /// Answers requests that waited for the frame to run.
    pub(crate) fn control_after_frame(&mut self, ctx: &egui::Context) {
        let Some(mut control) = self.control.take() else {
            return;
        };
        control.frames_since_input = control.frames_since_input.saturating_add(1);
        for (pending, response) in control.after_frame.drain(..) {
            pending.respond(response);
        }
        let now = Instant::now();
        let waits = std::mem::take(&mut control.waits);
        for (pending, deadline) in waits {
            let Request::Wait { until, text, .. } = &pending.request else {
                continue;
            };
            let met = match until {
                WaitCondition::Idle => {
                    control.screenshots.is_empty()
                        && control.frames_since_input >= SETTLE_FRAMES
                        && self.background_idle()
                }
                WaitCondition::Popup => self.popup_visible(ctx),
                WaitCondition::Text => text.as_deref().is_some_and(|text| self.shows_text(text)),
            };
            if met {
                pending.respond(Response::ok(Value::Null));
            } else if now >= deadline {
                let condition = serde_json::to_value(until).unwrap_or(Value::Null);
                pending.respond(Response::error(format!(
                    "timed out waiting until {condition}"
                )));
            } else {
                control.waits.push((pending, deadline));
            }
        }
        if control.busy() {
            ctx.request_repaint();
        }
        self.control = Some(control);
    }

    fn control_execute(&mut self, ctx: &egui::Context, command: &str, args: Value) -> Response {
        let root_before = self.state.project.as_ref().map(|p| p.root().to_owned());
        let result = self.commands.execute(command, &mut self.state, args);
        let root_after = self.state.project.as_ref().map(|p| p.root().to_owned());
        if root_after.is_some() && root_after != root_before {
            self.project_opened(ctx);
        }
        match result {
            Ok(value) => Response::ok(value),
            Err(error) => {
                if !matches!(error, CommandError::Unknown(_)) {
                    self.state.status = error.to_string();
                }
                Response::error(error.to_string())
            }
        }
    }

    fn background_idle(&self) -> bool {
        !self.state.search.is_running()
            && !self.state.quick_nav.text.is_running()
            && !self.find_popup.pending()
            && !self.state.runner.is_running()
            && !self.state.updates.is_busy()
    }

    fn popup_visible(&self, ctx: &egui::Context) -> bool {
        self.palette.open
            || self.find_popup.open
            || self.dialog.is_some()
            || self.state.about_open
            || self.state.first_run_open
            || self.completion_visible()
            || egui::Popup::is_any_open(ctx)
            || self.state.docs.popup.is_some()
    }

    fn completion_visible(&self) -> bool {
        self.state
            .completion
            .as_ref()
            .is_some_and(|session| session.visible())
    }

    fn terminal_text(&self) -> Option<String> {
        self.state
            .terminal
            .as_ref()
            .map(|terminal| terminal.screen().contents())
    }

    fn shows_text(&self, text: &str) -> bool {
        self.state.status.contains(text)
            || self
                .terminal_text()
                .is_some_and(|screen| screen.contains(text))
            || self
                .state
                .active_document
                .and_then(|index| self.state.documents.get(index))
                .is_some_and(|document| document.text.text().contains(text))
    }

    /// Workbench snapshot returned by the `state` request.
    pub(crate) fn control_state(&self, ctx: &egui::Context) -> Value {
        let root = self.state.project.as_ref().map(|p| p.root().to_owned());
        let relative = |path: &std::path::Path| {
            root.as_deref()
                .and_then(|root| path.strip_prefix(root).ok())
                .unwrap_or(path)
                .to_string_lossy()
                .replace('\\', "/")
        };
        let tabs: Vec<Value> = self
            .state
            .documents
            .iter()
            .map(|document| {
                json!({
                    "path": relative(&document.path),
                    "modified": document.text.is_dirty(),
                    "diskConflict": document.conflict.map(|conflict| match conflict {
                        jetthing_core::DiskConflict::Modified => "modified",
                        jetthing_core::DiskConflict::Deleted => "deleted",
                    }),
                })
            })
            .collect();
        let active = self
            .state
            .active_document
            .and_then(|index| self.state.documents.get(index));
        let editor = active.map(|document| {
            let caret = document.selections.primary().end();
            let (line, column) = document.text.char_to_line_column(caret).unwrap_or((0, 0));
            json!({
                "path": relative(&document.path),
                "text": document.text.text(),
                "caret": {"offset": caret, "line": line + 1, "column": column + 1},
                "selections": document.selections.selections().len(),
            })
        });
        let tool_window = |anchor| self.state.tool_windows.visible(anchor).map(ToolWindow::id);
        let focused = ctx.memory(|memory| memory.focused());
        let focus = focused.map(|id| {
            if id == editor::editor_id() {
                "editor"
            } else if id == toolwindows::terminal_input_id() {
                "terminal"
            } else if id == toolwindows::find_input_id() {
                "findInFiles"
            } else if id == findbar::query_id() {
                "findBar"
            } else if self.palette.open {
                "searchEverywhere"
            } else if self.dialog.is_some() {
                "dialog"
            } else {
                "other"
            }
        });
        json!({
            "project": root,
            "tabs": tabs,
            "activeTab": active.map(|document| relative(&document.path)),
            "toolWindows": {
                "left": tool_window(Anchor::Left),
                "bottom": tool_window(Anchor::Bottom),
            },
            "focus": focus,
            "projectTree": {
                "selected": self.tree_selection.as_deref().map(relative),
                "expanded": self.state.expanded_dirs.iter().map(|path| relative(path.as_path())).collect::<Vec<_>>(),
            },
            "editor": editor,
            "terminal": self.terminal_text(),
            "popups": {
                "searchEverywhere": self.palette.open.then(|| self.palette.tab_name()),
                "findInFilesPopup": self.find_popup.open,
                "switcher": self.state.quick_nav.switcher.as_ref().map(|switcher| match switcher.selected_entry() {
                    jetthing_core::quick_nav::SwitcherEntry::File(path) => relative(path),
                    jetthing_core::quick_nav::SwitcherEntry::ToolWindow(window) => window.title().to_owned(),
                }),
                "dialog": self.dialog.as_ref().map(|dialog| dialog.kind()),
                "findBar": self.find.open,
                "completion": self.completion_visible(),
                "notification": self.state.update_notification,
                "about": self.state.about_open,
                "customize": self.state.first_run_open,
                "crashReport": self.state.crash_report.is_some(),
                "menu": egui::Popup::is_any_open(ctx),
                "settings": self.settings_dialog.is_some(),
            },
            "window": {
                "width": ctx.content_rect().width(),
                "height": ctx.content_rect().height(),
            },
            "findInFiles": {
                "query": self.state.search_query,
                "results": self.state.search_results.len(),
                "popup": self.find_popup.open,
                "scope": self.state.search_scope_kind.id(),
                "selected": self.find_popup.selected(),
            },
            "problems": active.map(|_| self.state.active_problems().len()),
            "run": {
                "task": self.state.runner.plan().map(|plan| plan.name.clone()),
                "running": self.state.runner.is_running(),
            },
            "status": self.state.status,
            "busy": !self.background_idle(),
        })
    }
}

/// Frames for a chord. `DoubleShift` is the modifier-only Search Everywhere
/// gesture: Shift pressed and released twice on consecutive frames.
/// Frames for a click: move, then press and release `count` times, each on
/// its own frame so egui sees hover, press and (double) click like real input.
pub(crate) fn click_frames(pos: egui::Pos2, button: PointerButton, count: u8) -> Vec<InputFrame> {
    let button = match button {
        PointerButton::Primary => egui::PointerButton::Primary,
        PointerButton::Secondary => egui::PointerButton::Secondary,
        PointerButton::Middle => egui::PointerButton::Middle,
    };
    let frame = |events| InputFrame {
        modifiers: egui::Modifiers::NONE,
        events,
        done: None,
    };
    let press = |pressed| egui::Event::PointerButton {
        pos,
        button,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    let mut frames = vec![frame(vec![egui::Event::PointerMoved(pos)])];
    for _ in 0..count.clamp(1, 3) {
        frames.push(frame(vec![press(true)]));
        frames.push(frame(vec![press(false)]));
    }
    frames
}

pub(crate) fn key_frames(chord: &str) -> Result<Vec<InputFrame>, String> {
    let frame = |modifiers, events| InputFrame {
        modifiers,
        events,
        done: None,
    };
    if chord.trim().eq_ignore_ascii_case("doubleshift") {
        let shift = egui::Modifiers::SHIFT;
        let none = egui::Modifiers::NONE;
        return Ok(vec![
            frame(shift, Vec::new()),
            frame(none, Vec::new()),
            frame(shift, Vec::new()),
            frame(none, Vec::new()),
        ]);
    }
    let (modifiers, events) = key_events(chord)?;
    Ok(vec![frame(modifiers, events)])
}

/// Translates a chord into the modifiers and key events egui-winit would
/// produce, including the text event for printable keys typed without
/// Ctrl/⌘ (platforms emit Alt+digit text too, which shortcuts must swallow).
fn key_events(chord: &str) -> Result<(egui::Modifiers, Vec<egui::Event>), String> {
    let chord = Chord::parse(chord).map_err(|error| error.to_string())?;
    let key = egui::Key::from_name(&chord.key)
        .ok_or_else(|| format!("unknown key {:?} in chord", chord.key))?;
    let mac = cfg!(target_os = "macos");
    let modifiers = egui::Modifiers {
        alt: chord.alt,
        ctrl: chord.ctrl || (!mac && chord.command),
        shift: chord.shift,
        mac_cmd: mac && chord.command,
        command: if mac {
            chord.command
        } else {
            chord.ctrl || chord.command
        },
    };
    let mut events = vec![egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers,
    }];
    let printable = chord.key.chars().count() == 1 || key == egui::Key::Space;
    if printable && !modifiers.ctrl && !modifiers.mac_cmd {
        let text = if key == egui::Key::Space {
            " ".to_owned()
        } else if chord.shift {
            chord.key.to_uppercase()
        } else {
            chord.key.to_lowercase()
        };
        events.push(egui::Event::Text(text));
    }
    events.push(egui::Event::Key {
        key,
        physical_key: None,
        pressed: false,
        repeat: false,
        modifiers,
    });
    Ok((modifiers, events))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chords_become_press_text_release_events() {
        let (modifiers, events) = key_events("Ctrl+Shift+A").unwrap();
        assert!(modifiers.ctrl && modifiers.shift && modifiers.command);
        assert_eq!(events.len(), 2, "no text for Ctrl chords");
        assert!(matches!(
            events[0],
            egui::Event::Key {
                key: egui::Key::A,
                pressed: true,
                ..
            }
        ));
        let (_, events) = key_events("Shift+a").unwrap();
        assert!(matches!(&events[1], egui::Event::Text(text) if text == "A"));
        let (modifiers, events) = key_events("Alt+7").unwrap();
        assert!(modifiers.alt);
        assert!(matches!(&events[1], egui::Event::Text(text) if text == "7"));
        assert_eq!(key_events("Escape").unwrap().1.len(), 2);
        assert!(key_events("Ctrl+Bogus").is_err());
        assert!(key_events("Hyper+A").is_err());
        let frames = key_frames("DoubleShift").unwrap();
        assert_eq!(frames.len(), 4);
        assert!(frames[0].modifiers.shift_only() && frames[2].modifiers.shift_only());
        assert!(frames.iter().all(|frame| frame.events.is_empty()));
    }

    #[test]
    fn clicks_move_then_press_and_release_per_frame() {
        let frames = click_frames(egui::pos2(18.0, 20.0), PointerButton::Secondary, 2);
        assert_eq!(frames.len(), 5);
        assert!(matches!(frames[0].events[0], egui::Event::PointerMoved(_)));
        assert!(matches!(
            frames[1].events[0],
            egui::Event::PointerButton {
                button: egui::PointerButton::Secondary,
                pressed: true,
                ..
            }
        ));
        assert!(matches!(
            frames[4].events[0],
            egui::Event::PointerButton { pressed: false, .. }
        ));
        assert_eq!(
            click_frames(egui::Pos2::ZERO, PointerButton::Primary, 0).len(),
            3
        );
    }

    #[test]
    fn double_shift_frames_open_search_everywhere() {
        let mut app = IdeApp::with_update_check(None, Vec::new(), false);
        let ctx = egui::Context::default();
        for frame in key_frames("DoubleShift").unwrap() {
            let input = egui::RawInput {
                modifiers: frame.modifiers,
                events: frame.events,
                ..Default::default()
            };
            let _ = ctx.run(input, |ctx| app.detect_double_shift(ctx));
        }
        assert_eq!(
            app.state.quick_nav.request,
            Some(
                jetthing_core::quick_nav::NavigationRequest::SearchEverywhere(
                    jetthing_core::quick_nav::SearchTab::All
                )
            )
        );
    }

    #[test]
    fn state_snapshot_describes_an_empty_workbench() {
        let app = IdeApp::with_update_check(None, Vec::new(), false);
        let state = app.control_state(&egui::Context::default());
        assert_eq!(state["project"], Value::Null);
        assert_eq!(state["tabs"], json!([]));
        assert_eq!(state["popups"]["searchEverywhere"], Value::Null);
        assert_eq!(state["focus"], Value::Null);
    }

    #[test]
    fn state_snapshot_reports_tabs_editor_and_popups() {
        let dir = std::env::temp_dir().join(format!("jetthing-control-ui-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(dir.join("src/main.rs"), "fn main() {}\n").unwrap();
        let mut app =
            IdeApp::with_update_check(Some(dir.clone()), vec![PathBuf::from("src/main.rs")], false);
        app.palette
            .show_tab(jetthing_core::quick_nav::SearchTab::All);
        let ctx = egui::Context::default();
        let response = app.control_execute(
            &ctx,
            "editor.insertText",
            json!({"position": 0, "text": "// hi\n"}),
        );
        assert!(response.ok, "{response:?}");
        let state = app.control_state(&ctx);
        assert_eq!(state["activeTab"], "src/main.rs");
        assert_eq!(state["tabs"][0]["modified"], true);
        assert!(
            state["editor"]["text"]
                .as_str()
                .unwrap()
                .starts_with("// hi")
        );
        assert_eq!(state["editor"]["caret"]["line"], 2);
        assert_eq!(state["popups"]["searchEverywhere"], "all");
        assert!(app.popup_visible(&ctx));
        assert!(app.shows_text("fn main"));
        let unknown = app.control_execute(&ctx, "no.such.command", Value::Null);
        assert!(!unknown.ok);
        assert_eq!(app.control_state(&ctx)["popups"]["about"], false);
        assert!(app.control_execute(&ctx, "app.about", Value::Null).ok);
        assert_eq!(app.control_state(&ctx)["popups"]["about"], true);
        let _ = std::fs::remove_dir_all(dir);
    }
}
