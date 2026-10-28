//! The "File Cache Conflict" dialog (PLAT-VFS-101): a file changed on disk while its document
//! has unsaved edits. Both frontends show it with these texts and choices; resolving a choice
//! is a core command.

use archipel_ide_core::DiskConflict;
use serde_json::{Value, json};

/// What a button of the dialog does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Choice {
    /// Take the disk text (or close the tab of a deleted file).
    LoadDisk,
    /// Keep the memory text; the next save overwrites the file.
    KeepMemory,
    /// Show the two texts side by side; the dialog stays.
    ShowDifference,
}

/// The dialog: title, message and its buttons in order (the first is the default button).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prompt {
    pub title: &'static str,
    pub message: String,
    pub buttons: Vec<(&'static str, Choice)>,
    /// Header of the two panes of Show Difference: (disk side, memory side).
    pub pane_titles: (&'static str, &'static str),
}

/// The dialog of `conflict` for the file `name`.
pub fn prompt(name: &str, conflict: DiskConflict) -> Prompt {
    match conflict {
        DiskConflict::Modified => Prompt {
            title: "File Cache Conflict",
            message: format!("Changes have been made to '{name}' in memory and on disk."),
            buttons: vec![
                ("Load File System Changes", Choice::LoadDisk),
                ("Keep Memory Changes", Choice::KeepMemory),
                ("Show Difference", Choice::ShowDifference),
            ],
            pane_titles: ("File system content", "Memory content"),
        },
        DiskConflict::Deleted => Prompt {
            title: "File Cache Conflict",
            message: format!("'{name}' was deleted on disk, but it has unsaved changes in memory."),
            buttons: vec![
                ("Close File", Choice::LoadDisk),
                ("Keep Memory Changes", Choice::KeepMemory),
            ],
            pane_titles: ("File system content", "Memory content"),
        },
    }
}

/// The command (id, params) that applies `choice` to document `index`; `None` for the
/// choices that start no command.
pub fn command(choice: Choice, index: usize) -> Option<(&'static str, Value)> {
    match choice {
        Choice::LoadDisk => Some(("editor.reloadFromDisk", json!({ "index": index }))),
        Choice::KeepMemory => Some(("editor.keepMemoryVersion", json!({ "index": index }))),
        Choice::ShowDifference => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plat_vfs_101_dialog_texts_and_choices() {
        let p = prompt("App.java", DiskConflict::Modified);
        assert_eq!(p.title, "File Cache Conflict");
        assert_eq!(
            p.message,
            "Changes have been made to 'App.java' in memory and on disk."
        );
        let labels: Vec<_> = p.buttons.iter().map(|b| b.0).collect();
        assert_eq!(
            labels,
            [
                "Load File System Changes",
                "Keep Memory Changes",
                "Show Difference"
            ]
        );
        assert_eq!(
            command(Choice::LoadDisk, 2).unwrap().0,
            "editor.reloadFromDisk"
        );
        assert!(command(Choice::ShowDifference, 0).is_none());
    }
}
