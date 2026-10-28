//! The "File Cache Conflict" dialog (PLAT-VFS-101, PLAT-SAVE-079): a document with unsaved
//! edits whose file changed on disk. The texts and choices come from
//! `archipel_ide_view::conflict`; resolving is a core command.

use crate::app::{DialogKind, Msg, TuiApp};
use crate::dialogs::Input;
use archipel_ide_core::{DiffLine, DiffTag};
use archipel_ide_view::conflict::{Choice, command, prompt};
use archipel_kit::openapi::ui::ExitCode;
use archipel_kit_tui::dialog::{Dialog, DialogContent};
use archipel_kit_tui::input::Event;
use archipel_kit_tui::ratatui::buffer::Buffer;
use archipel_kit_tui::ratatui::layout::Rect;
use archipel_kit_tui::ratatui::style::Style;
use archipel_kit_tui::runtime::Runtime;

/// Rows of the difference listing.
const DIFF_ROWS: u16 = 14;

impl TuiApp {
    /// Opens the conflict dialog for the first document with an unresolved outside change,
    /// unless another layer is open or the user closed the dialog with Escape (saving and
    /// Reload All from Disk ask again).
    pub(crate) fn offer_conflict(&mut self, rt: &mut Runtime) {
        let state = self.ide.state();
        let conflicted = |app: &Self| {
            app.ide
                .state()
                .documents()
                .iter()
                .filter(|d| d.conflict.is_some())
                .map(|d| d.path.clone())
                .collect::<Vec<_>>()
        };
        // A resolved conflict is forgotten, so the next one asks again.
        let open = conflicted(self);
        self.conflict_snoozed.retain(|path| open.contains(path));
        if !self.layers.is_empty() {
            return;
        }
        let Some(index) = state
            .documents()
            .iter()
            .position(|d| d.conflict.is_some() && !self.conflict_snoozed.contains(&d.path))
        else {
            return;
        };
        let document = &state.documents()[index];
        let Some(conflict) = document.conflict else {
            return;
        };
        let name = document.path.display().to_string();
        let prompt = prompt(&name, conflict);
        let mut dialog = Dialog::new(prompt.title).message(prompt.message).width(96);
        for (slot, (label, _)) in prompt.buttons.iter().enumerate() {
            dialog = match slot {
                0 => dialog.primary(*label),
                1 => dialog.secondary(*label),
                _ => dialog.tertiary(*label),
            };
        }
        let kind = DialogKind::Conflict { index };
        let dialog =
            dialog.on_exit(move |code| vec![Msg::Dialog(kind.clone(), code, Input::default())]);
        self.layers.push(rt, dialog);
    }

    /// A button of the conflict dialog.
    pub(crate) fn conflict_answer(&mut self, rt: &mut Runtime, index: usize, code: ExitCode) {
        let Some(document) = self.ide.state().documents().get(index) else {
            return;
        };
        let Some(conflict) = document.conflict else {
            return;
        };
        let path = document.path.clone();
        let name = path.display().to_string();
        let prompt = prompt(&name, conflict);
        let slot = match code {
            ExitCode::Primary => 0,
            ExitCode::Secondary => 1,
            ExitCode::Tertiary => 2,
            _ => {
                self.conflict_snoozed.push(path);
                self.ide.state_mut().set_status(format!(
                    "{name} is not saved: load the file system changes or keep yours first"
                ));
                return;
            }
        };
        let Some((_, choice)) = prompt.buttons.get(slot).copied() else {
            return;
        };
        match choice {
            Choice::ShowDifference => self.show_difference(rt, index, prompt.pane_titles),
            choice => {
                if let Some((id, params)) = command(choice, index) {
                    self.command(rt, id, params);
                }
            }
        }
    }

    /// Lists the difference between the file system content and the memory content.
    fn show_difference(&mut self, rt: &mut Runtime, index: usize, titles: (&str, &str)) {
        let lines = self
            .ide
            .state()
            .conflict_diff(index, 400)
            .unwrap_or_default();
        let dialog = Dialog::new("Difference")
            .content(DiffContent::new(lines, titles))
            .width(100)
            .primary("Close")
            .on_exit(|code| vec![Msg::Dialog(DialogKind::Notice, code, Input::default())]);
        self.layers.push(rt, dialog);
    }
}

/// The two texts as one listing: `-` lines only on disk, `+` lines only in memory.
pub struct DiffContent {
    lines: Vec<DiffLine>,
    titles: (String, String),
    top: usize,
}

impl DiffContent {
    pub fn new(lines: Vec<DiffLine>, titles: (&str, &str)) -> Self {
        Self {
            lines,
            titles: (titles.0.to_owned(), titles.1.to_owned()),
            top: 0,
        }
    }
}

impl DialogContent for DiffContent {
    fn height(&self, _width: u16) -> u16 {
        DIFF_ROWS + 1
    }

    fn draw(&mut self, rt: &mut Runtime, area: Rect, buf: &mut Buffer) {
        let p = rt.look.palette;
        let header = format!("- {}   + {}", self.titles.0, self.titles.1);
        buf.set_stringn(
            area.x,
            area.y,
            header,
            area.width as usize,
            Style::default().fg(p.light_grey),
        );
        for row in 0..DIFF_ROWS.min(area.height.saturating_sub(1)) {
            let Some(line) = self.lines.get(self.top + row as usize) else {
                break;
            };
            let (mark, color) = match line.tag {
                DiffTag::Same => (' ', p.white),
                DiffTag::Disk => ('-', p.red),
                DiffTag::Memory => ('+', p.green),
                DiffTag::Gap => ('~', p.grey),
            };
            let text = if line.tag == DiffTag::Gap {
                "...".to_owned()
            } else {
                line.text.clone()
            };
            buf.set_stringn(
                area.x,
                area.y + 1 + row,
                format!("{mark} {text}"),
                area.width as usize,
                Style::default().fg(color),
            );
        }
    }

    fn event(&mut self, _rt: &mut Runtime, event: &Event) -> bool {
        use archipel_kit_tui::input::KeyCode;
        let Event::Key(key) = event else {
            return false;
        };
        let max = self.lines.len().saturating_sub(DIFF_ROWS as usize);
        match key.code {
            KeyCode::Down => self.top = (self.top + 1).min(max),
            KeyCode::Up => self.top = self.top.saturating_sub(1),
            KeyCode::PageDown => self.top = (self.top + DIFF_ROWS as usize).min(max),
            KeyCode::PageUp => self.top = self.top.saturating_sub(DIFF_ROWS as usize),
            _ => return false,
        }
        true
    }
}
