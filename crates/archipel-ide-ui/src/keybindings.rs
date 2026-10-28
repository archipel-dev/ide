//! Keymap-driven shortcut dispatch.
//!
//! Key events are translated into [`KeyStroke`]s and resolved against the active keymap
//! (`IdeState::keymap`, chosen in Settings | Keymap). The resolved IntelliJ action ids
//! run either a typed command (`archipel_ide_core::keymap_commands::ACTION_COMMANDS`) or one
//! of the workbench popups below.

use crate::{IdeApp, popups, toolwindows};
use archipel_ide_actions::ActionPlace;
use archipel_ide_keymap::{KeyStroke, Lookup, Modifier, Modifiers, Notation, Shortcut};
use archipel_ide_platform::data_keys;
use eframe::egui;
use serde_json::Value;
use std::path::PathBuf;

#[cfg(test)]
pub use archipel_ide_view::actions::ui_action;
pub use archipel_ide_view::actions::{TERMINAL_ACTIONS, UiAction, is_implemented};

/// Translates an egui key press into the keymap's notation.
pub fn stroke(key: egui::Key, modifiers: egui::Modifiers) -> Option<KeyStroke> {
    use egui::Key;
    let name = match key {
        Key::ArrowDown => "Down",
        Key::ArrowLeft => "Left",
        Key::ArrowRight => "Right",
        Key::ArrowUp => "Up",
        Key::Escape => "Esc",
        Key::Tab => "Tab",
        Key::Backspace => "Backspace",
        Key::Enter => "Enter",
        Key::Space => "Space",
        Key::Insert => "Insert",
        Key::Delete => "Delete",
        Key::Home => "Home",
        Key::End => "End",
        Key::PageUp => "Page Up",
        Key::PageDown => "Page Down",
        Key::Colon => ":",
        Key::Comma => ",",
        Key::Backslash => "\\",
        Key::Slash => "/",
        Key::Pipe => "|",
        Key::Questionmark => "?",
        Key::Exclamationmark => "!",
        Key::OpenBracket => "[",
        Key::CloseBracket => "]",
        Key::OpenCurlyBracket => "{",
        Key::CloseCurlyBracket => "}",
        Key::Backtick => "`",
        Key::Minus => "-",
        Key::Period => ".",
        Key::Plus => "+",
        Key::Equals => "=",
        Key::Semicolon => ";",
        Key::Quote => "'",
        Key::Num0 => "0",
        Key::Num1 => "1",
        Key::Num2 => "2",
        Key::Num3 => "3",
        Key::Num4 => "4",
        Key::Num5 => "5",
        Key::Num6 => "6",
        Key::Num7 => "7",
        Key::Num8 => "8",
        Key::Num9 => "9",
        Key::Copy | Key::Cut | Key::Paste => return None,
        other => other.name(),
    };
    Some(KeyStroke::new(
        Modifiers {
            ctrl: modifiers.ctrl,
            alt: modifiers.alt,
            shift: modifiers.shift,
            meta: modifiers.mac_cmd,
        },
        name,
    ))
}

impl IdeApp {
    /// Label of the first shortcut bound to `action` in the active keymap.
    pub(crate) fn action_shortcut(&self, action: &str) -> Option<String> {
        self.state
            .keymap_state()
            .keymap
            .shortcuts(action)
            .iter()
            .find(|shortcut| !matches!(shortcut, Shortcut::Gesture(_)))
            .map(|shortcut| shortcut.label(Notation::current()))
    }

    /// Shortcut label for a command: from the keymap when an action maps to it,
    /// else the command's own (macOS-notation) default.
    pub(crate) fn command_shortcut(&self, spec: &archipel_ide_command::CommandSpec) -> String {
        archipel_ide_core::keymap_commands::actions_for_command(&spec.id)
            .find_map(|action| self.action_shortcut(action))
            .unwrap_or_else(|| crate::keys::display(spec.shortcut.as_deref().unwrap_or_default()))
    }

    /// Directory (project-relative) that New entries go into: the selected directory,
    /// or the parent of the selected file, like the Project view's New group.
    fn new_entry_parent(&self) -> Option<PathBuf> {
        let root = self.state.project_root()?;
        Some(
            self.tree_selection
                .as_ref()
                .map(|path| {
                    if root.join(path).is_dir() {
                        path.clone()
                    } else {
                        path.parent().map(Into::into).unwrap_or_default()
                    }
                })
                .unwrap_or_default(),
        )
    }

    pub(crate) fn run_ui_action(&mut self, action: UiAction, ctx: &egui::Context) {
        match action {
            UiAction::GotoLine => self.open_dialog(popups::Dialog::GotoLine {
                input: String::new(),
            }),
            UiAction::FileStructure => self.open_dialog(popups::Dialog::FileStructure {
                query: String::new(),
                selected: None,
            }),
            UiAction::Find | UiAction::Replace => {
                let selection = self.primary_selection_text();
                self.find.show(action == UiAction::Replace, selection);
            }
            UiAction::FindNext | UiAction::FindPrevious => {
                let id = if action == UiAction::FindNext {
                    "editor.findNext"
                } else {
                    "editor.findPrevious"
                };
                self.dispatch(id, self.find.params());
            }
            UiAction::FindInPath => {
                let selection = self.primary_selection_text();
                self.open_find_popup(selection);
            }
            UiAction::SaveAll => self.autosave(),
            UiAction::NewProject => self.new_project_dialog(),
            UiAction::OpenProject => self.open_project_dialog(),
            // Manage Projects shows the welcome screen's project list.
            UiAction::ManageProjects => self.dispatch("project.close", Value::Null),
            UiAction::NewFile | UiAction::NewDirectory => {
                if let Some(parent) = self.new_entry_parent() {
                    self.open_dialog(popups::Dialog::NewEntry {
                        parent,
                        directory: action == UiAction::NewDirectory,
                        name: String::new(),
                    });
                }
            }
            UiAction::NewJavaClass | UiAction::NewKotlinFile => {
                if let Some(dir) = self.new_entry_parent() {
                    self.open_dialog(popups::Dialog::NewClass {
                        dir,
                        kotlin: action == UiAction::NewKotlinFile,
                        name: String::new(),
                        kind: 0,
                        error: None,
                    });
                }
            }
            UiAction::Push => crate::push_dialog::open(self),
            UiAction::Exit => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
            // Clipboard actions go through the editor so the system clipboard is updated.
            UiAction::Copy => {
                crate::editor::handle_request(self, ctx, crate::editor::EditorRequest::Copy)
            }
            UiAction::Cut => {
                crate::editor::handle_request(self, ctx, crate::editor::EditorRequest::Cut)
            }
            UiAction::Paste => ctx.send_viewport_cmd(egui::ViewportCommand::RequestPaste),
        }
    }

    /// Runs the first applicable action; returns whether the key was handled. Each action
    /// decides from the context whether the key is its business (see `CommandAction::update`).
    fn run_keymap_actions(
        &mut self,
        actions: &[String],
        ctx: &egui::Context,
        terminal_focused: bool,
    ) -> bool {
        let editor_focused = self.editor_focused(ctx);
        for action in actions {
            if terminal_focused && !TERMINAL_ACTIONS.contains(&action.as_str()) {
                continue;
            }
            // Escape closes popups, dialogs and the find bar before the editor uses it
            // (`handle_shortcuts` runs the editor's own Escape last).
            if action == "EditorEscape" {
                continue;
            }
            let effect = self.with_data_values(
                |data| data.with_value(&data_keys::EDITOR_FOCUSED, editor_focused),
                |registry, data| registry.perform(action, ActionPlace::Keymap, data),
            );
            if let Some(effect) = effect {
                self.apply_effect(effect, ActionPlace::Keymap, ctx);
                return true;
            }
        }
        false
    }

    /// Dispatches a command that a key press started, with the window's reactions to it.
    pub(crate) fn dispatch_from_key(
        &mut self,
        command: &str,
        mut params: Value,
        ctx: &egui::Context,
    ) {
        match command {
            // Ctrl+Shift+Tab walks the Switcher backwards.
            "navigate.switcher" => params = crate::actions::switcher_params(ctx),
            "code.showIntentions" => crate::intentions_ui::reset_selection(ctx),
            _ => {}
        }
        self.dispatch(command, params);
        if matches!(command, "navigate.back" | "navigate.forward") {
            ctx.memory_mut(|memory| memory.request_focus(crate::editor::editor_id()));
        }
    }

    /// Resolves this frame's key presses against the active keymap. Handled presses
    /// are removed from the input so focused widgets do not see them.
    pub(crate) fn dispatch_keymap(&mut self, ctx: &egui::Context, terminal_focused: bool) -> bool {
        let events = ctx.input(|input| input.events.clone());
        let mut handled = Vec::new();
        let mut chord_modifier = false;
        for (index, event) in events.iter().enumerate() {
            let egui::Event::Key {
                key,
                modifiers,
                pressed: true,
                ..
            } = event
            else {
                continue;
            };
            let Some(stroke) = stroke(*key, *modifiers) else {
                continue;
            };
            let pending = self.pending_chord.take();
            let mut strokes: Vec<KeyStroke> = pending.iter().cloned().collect();
            strokes.push(stroke.clone());
            let lookup = self.state.keymap_state().keymap.lookup(&strokes);
            let modified = stroke.modifiers.ctrl || stroke.modifiers.alt || stroke.modifiers.meta;
            let consumed = match lookup {
                Lookup::Actions(actions) => {
                    self.run_keymap_actions(&actions, ctx, terminal_focused) || pending.is_some()
                }
                // Only modified strokes start chords, so plain typing is never swallowed.
                Lookup::Prefix if modified && !terminal_focused => {
                    self.state.set_status(format!(
                        "{} was pressed. Waiting for the second stroke…",
                        stroke.label(Notation::current())
                    ));
                    self.pending_chord = Some(stroke);
                    true
                }
                Lookup::Prefix | Lookup::None => {
                    if let Some(first) = &pending {
                        self.state.set_status(format!(
                            "No action is bound to {}, {}",
                            first.label(Notation::current()),
                            stroke.label(Notation::current())
                        ));
                    }
                    pending.is_some()
                }
            };
            if consumed {
                handled.push(index);
                chord_modifier |= modified;
            }
        }
        if !handled.is_empty() {
            ctx.input_mut(|input| {
                let mut index = 0;
                input.events.retain(|_| {
                    let keep = !handled.contains(&index);
                    index += 1;
                    keep
                });
            });
        }
        chord_modifier
    }

    /// Actions bound to pressing `modifier` twice (Search Everywhere on Double Shift).
    pub(crate) fn run_double_modifier(&mut self, modifier: Modifier, ctx: &egui::Context) {
        let actions = self
            .state
            .keymap_state()
            .keymap
            .actions_for(&Shortcut::DoubleModifier(modifier))
            .to_vec();
        let terminal = ctx.memory(|m| m.has_focus(toolwindows::terminal_input_id()));
        self.run_keymap_actions(&actions, ctx, terminal);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use archipel_ide_keymap::{KeymapSet, Overrides};

    #[test]
    fn egui_keys_map_to_keymap_strokes() {
        let ctrl_shift = egui::Modifiers {
            ctrl: true,
            shift: true,
            command: true,
            ..Default::default()
        };
        assert_eq!(
            stroke(egui::Key::A, ctrl_shift).unwrap(),
            KeyStroke::new(
                Modifiers {
                    ctrl: true,
                    shift: true,
                    ..Modifiers::NONE
                },
                "A"
            )
        );
        assert_eq!(
            stroke(egui::Key::F12, egui::Modifiers::NONE).unwrap().key,
            "F12"
        );
        assert_eq!(
            stroke(egui::Key::PageDown, egui::Modifiers::NONE)
                .unwrap()
                .key,
            "Page Down"
        );
        assert_eq!(
            stroke(egui::Key::Slash, egui::Modifiers::NONE).unwrap().key,
            "/"
        );
        assert!(stroke(egui::Key::Copy, egui::Modifiers::NONE).is_none());
        let mac_cmd = egui::Modifiers {
            mac_cmd: true,
            command: true,
            ..Default::default()
        };
        assert!(stroke(egui::Key::Comma, mac_cmd).unwrap().modifiers.meta);
    }

    #[test]
    fn primary_keymaps_resolve_the_workbench_shortcuts() {
        let set = KeymapSet::shared().unwrap();
        let check = |keymap: &str, key: egui::Key, modifiers: egui::Modifiers, action: &str| {
            let keymap = set.keymap(keymap, &Overrides::default()).unwrap();
            match keymap.lookup(&[stroke(key, modifiers).unwrap()]) {
                Lookup::Actions(actions) => assert!(
                    actions.iter().any(|a| a == action),
                    "{}: {action} not in {actions:?}",
                    keymap.id
                ),
                other => panic!("{}: {action} unresolved: {other:?}", keymap.id),
            }
            assert!(is_implemented(action), "{action} is not implemented");
        };
        let ctrl = egui::Modifiers::CTRL;
        let ctrl_shift = egui::Modifiers::CTRL | egui::Modifiers::SHIFT;
        let ctrl_alt = egui::Modifiers::CTRL | egui::Modifiers::ALT;
        let cmd = egui::Modifiers::MAC_CMD;
        for keymap in [
            "$default",
            "Default for GNOME",
            "Default for KDE",
            "Default for XWin",
        ] {
            check(keymap, egui::Key::A, ctrl_shift, "GotoAction");
            check(keymap, egui::Key::E, ctrl, "RecentFiles");
            check(keymap, egui::Key::S, ctrl_alt, "ShowSettings");
            check(keymap, egui::Key::N, ctrl_shift, "GotoFile");
            // KDE reserves Ctrl+F-keys for desktops; its keymap builds with Ctrl+9.
            let build = if keymap == "Default for KDE" {
                egui::Key::Num9
            } else {
                egui::Key::F9
            };
            check(keymap, build, ctrl, "CompileDirty");
            check(keymap, egui::Key::F10, egui::Modifiers::SHIFT, "Run");
            check(
                keymap,
                egui::Key::Num1,
                egui::Modifiers::ALT,
                "ActivateProjectToolWindow",
            );
            check(keymap, egui::Key::D, ctrl, "EditorDuplicate");
        }
        check(
            "Mac OS X 10.5+",
            egui::Key::A,
            cmd | egui::Modifiers::SHIFT,
            "GotoAction",
        );
        check("Mac OS X 10.5+", egui::Key::Comma, cmd, "ShowSettings");
        check("Mac OS X 10.5+", egui::Key::R, ctrl, "Run");
        check("Mac OS X 10.5+", egui::Key::L, cmd, "GotoLine");
    }

    #[test]
    fn every_terminal_action_is_implemented() {
        for action in TERMINAL_ACTIONS {
            assert!(is_implemented(action), "{action}");
        }
    }
}
