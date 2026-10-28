//! Models of the Recent Files popup, the Recent Locations snippets and the bookmarks (spec
//! `docs/specs/search/05-recent-and-structure-popups.md` sections 1, 2 and 7).

use crate::quick_nav::{Ranked, rank};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Files kept by the editor history shown in the popup (`recentFilesLimit`, SRCH-REC-001).
pub const RECENT_FILES_LIMIT: usize = 50;

/// The editor history: files in the order they were last selected in an editor.
#[derive(Debug, Clone, Default)]
pub struct RecentFiles {
    visited: Vec<PathBuf>,
    edited: HashSet<PathBuf>,
}

impl RecentFiles {
    /// A file was selected in an editor: it moves to the top (SRCH-REC-006). Closing a tab does
    /// not remove it.
    pub fn visit(&mut self, path: &Path) {
        self.visited.retain(|p| p != path);
        self.visited.push(path.to_owned());
    }

    /// The file was edited in this session.
    pub fn edited(&mut self, path: &Path) {
        self.edited.insert(path.to_owned());
    }

    /// Newest first, at most [`RECENT_FILES_LIMIT`].
    pub fn list(&self, edited_only: bool) -> Vec<PathBuf> {
        self.visited
            .iter()
            .rev()
            .filter(|p| !edited_only || self.edited.contains(*p))
            .take(RECENT_FILES_LIMIT)
            .cloned()
            .collect()
    }

    /// `Delete`: removes the file from the list.
    pub fn remove(&mut self, path: &Path) {
        self.visited.retain(|p| p != path);
    }
}

/// The Recent Files popup (`Ctrl+E`).
#[derive(Debug, Clone)]
pub struct RecentFilesPopup {
    edited_only: bool,
    query: String,
    selected: usize,
    focused: Option<PathBuf>,
}

impl RecentFilesPopup {
    /// Opens the popup; the selection follows SRCH-REC-002 (the first file, the second when the
    /// first is the focused one).
    pub fn open(files: &RecentFiles, focused: Option<&Path>, edited_only: bool) -> Self {
        let mut popup = Self {
            edited_only,
            query: String::new(),
            selected: 0,
            focused: focused.map(Path::to_owned),
        };
        popup.selected = popup.initial_selection(files);
        popup
    }

    fn initial_selection(&self, files: &RecentFiles) -> usize {
        let list = files.list(self.edited_only);
        if list.len() > 1 && self.focused.as_deref() == list.first().map(PathBuf::as_path) {
            1
        } else {
            0
        }
    }

    pub fn edited_only(&self) -> bool {
        self.edited_only
    }

    /// `Ctrl+E` while open toggles *Show edited only*.
    pub fn toggle_edited_only(&mut self, files: &RecentFiles) {
        self.edited_only = !self.edited_only;
        self.selected = self.initial_selection(files);
    }

    /// The rows after the speed search.
    pub fn rows(&self, files: &RecentFiles) -> Vec<Ranked<PathBuf>> {
        rank(
            &self.query,
            files.list(self.edited_only).into_iter().map(|path| {
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                (path, name)
            }),
            RECENT_FILES_LIMIT,
        )
    }

    pub fn type_text(&mut self, text: &str) {
        self.query.push_str(text);
        self.selected = 0;
    }

    pub fn move_selection(&mut self, files: &RecentFiles, down: bool) {
        let len = self.rows(files).len();
        if len > 0 {
            self.selected = if down {
                (self.selected + 1) % len
            } else {
                (self.selected + len - 1) % len
            };
        }
    }

    pub fn selected(&self, files: &RecentFiles) -> Option<PathBuf> {
        self.rows(files).get(self.selected).map(|r| r.item.clone())
    }

    /// `Delete`: removes the selected file from the history; the selection moves to the row that
    /// took its place (SRCH-REC-003). Returns the removed file.
    pub fn delete_selected(&mut self, files: &mut RecentFiles) -> Option<PathBuf> {
        let path = self.selected(files)?;
        files.remove(&path);
        let len = self.rows(files).len();
        self.selected = self.selected.min(len.saturating_sub(1));
        Some(path)
    }

    /// The text shown instead of an empty list (SRCH-REC-001, SRCH-REC-003).
    pub fn empty_text(&self, files: &RecentFiles) -> Option<String> {
        if !self.rows(files).is_empty() {
            return None;
        }
        Some(if self.query.is_empty() {
            "No recent files".to_owned()
        } else {
            "Press 'Enter' to search in Project".to_owned()
        })
    }
}

/// The lines of a Recent Locations snippet: the caret line with 2 lines before and 2 after, the
/// window sliding to keep five lines near the file boundaries (SRCH-REC-010). Zero-based, end
/// exclusive.
pub fn snippet_window(line_count: usize, caret_line: usize) -> std::ops::Range<usize> {
    if line_count == 0 {
        return 0..0;
    }
    let caret = caret_line.min(line_count - 1);
    let window = 5.min(line_count);
    let start = caret.saturating_sub(2).min(line_count - window);
    start..start + window
}

/// The text of an empty Recent Locations list.
pub const NO_RECENT_LOCATIONS: &str = "No recent locations found";

// ---------------------------------------------------------------------- bookmarks

/// A line bookmark (SRCH-BKM-001).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bookmark {
    pub path: PathBuf,
    /// Zero-based line.
    pub line: usize,
    pub mnemonic: Option<char>,
    pub description: String,
}

/// What toggling a mnemonic asks for or did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MnemonicOutcome {
    Added,
    Removed,
    Changed,
    /// The mnemonic is taken: the dialog `Rewrite Mnemonic` asks with this message.
    NeedsRewrite(String),
}

/// The bookmarks of a project: one default list.
#[derive(Debug, Clone, Default)]
pub struct Bookmarks {
    list: Vec<Bookmark>,
    /// Names of the lists (`Create Bookmark List…`).
    lists: Vec<String>,
}

impl Bookmarks {
    pub fn all(&self) -> &[Bookmark] {
        &self.list
    }

    fn at(&self, path: &Path, line: usize) -> Option<usize> {
        self.list
            .iter()
            .position(|b| b.path == path && b.line == line)
    }

    /// `F11` (SRCH-BKM-001): adds or removes the line bookmark; the description is the selected
    /// text when it is not blank.
    pub fn toggle(&mut self, path: &Path, line: usize, selection: Option<&str>) -> bool {
        if let Some(i) = self.at(path, line) {
            self.list.remove(i);
            return false;
        }
        self.list.push(Bookmark {
            path: path.to_owned(),
            line,
            mnemonic: None,
            description: selection
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_default()
                .to_owned(),
        });
        true
    }

    /// `Ctrl+Shift+<digit>` (SRCH-BKM-002).
    pub fn toggle_mnemonic(&mut self, path: &Path, line: usize, mnemonic: char) -> MnemonicOutcome {
        if let Some(i) = self.at(path, line)
            && self.list[i].mnemonic == Some(mnemonic)
        {
            self.list.remove(i);
            return MnemonicOutcome::Removed;
        }
        if let Some(taken) = self.list.iter().find(|b| b.mnemonic == Some(mnemonic))
            && (taken.path != path || taken.line != line)
        {
            return MnemonicOutcome::NeedsRewrite(format!(
                "\u{2018}{mnemonic}\u{2019} mnemonic is already taken by \u{2018}{}:{}\u{2019}. Do you want to rewrite it?",
                taken.path.file_name().unwrap_or_default().to_string_lossy(),
                taken.line + 1
            ));
        }
        match self.at(path, line) {
            Some(i) => {
                self.list[i].mnemonic = Some(mnemonic);
                MnemonicOutcome::Changed
            }
            None => {
                self.list.push(Bookmark {
                    path: path.to_owned(),
                    line,
                    mnemonic: Some(mnemonic),
                    description: String::new(),
                });
                MnemonicOutcome::Added
            }
        }
    }

    /// Accepting the `Rewrite Mnemonic` dialog: the old bookmark loses the mnemonic.
    pub fn rewrite_mnemonic(&mut self, path: &Path, line: usize, mnemonic: char) {
        for b in &mut self.list {
            if b.mnemonic == Some(mnemonic) {
                b.mnemonic = None;
            }
        }
        match self.at(path, line) {
            Some(i) => self.list[i].mnemonic = Some(mnemonic),
            None => self.list.push(Bookmark {
                path: path.to_owned(),
                line,
                mnemonic: Some(mnemonic),
                description: String::new(),
            }),
        }
    }

    /// `Ctrl+<digit>` (SRCH-BKM-004): the bookmark with that mnemonic.
    pub fn goto_mnemonic(&self, mnemonic: char) -> Option<&Bookmark> {
        self.list.iter().find(|b| b.mnemonic == Some(mnemonic))
    }

    /// Next Line Bookmark: all bookmarks ordered by file path then line, cycling.
    pub fn next(&self, path: &Path, line: usize, forward: bool) -> Option<&Bookmark> {
        let mut sorted: Vec<&Bookmark> = self.list.iter().collect();
        sorted.sort_by(|a, b| a.path.cmp(&b.path).then(a.line.cmp(&b.line)));
        let at = sorted.iter().position(|b| b.path == path && b.line == line);
        let len = sorted.len();
        if len == 0 {
            return None;
        }
        let index = match (at, forward) {
            (Some(i), true) => (i + 1) % len,
            (Some(i), false) => (i + len - 1) % len,
            (None, true) => sorted
                .iter()
                .position(|b| (b.path.as_path(), b.line) > (path, line))
                .unwrap_or(0),
            (None, false) => sorted
                .iter()
                .rposition(|b| (b.path.as_path(), b.line) < (path, line))
                .unwrap_or(len - 1),
        };
        Some(sorted[index])
    }

    /// Next Line Bookmark in Editor: the bookmarks of the file by line, never staying on the
    /// same line; `None` (disabled) when there is no other one.
    pub fn next_in_editor(&self, path: &Path, line: usize, forward: bool) -> Option<&Bookmark> {
        let mut own: Vec<&Bookmark> = self.list.iter().filter(|b| b.path == path).collect();
        own.sort_by_key(|b| b.line);
        let others: Vec<&&Bookmark> = own.iter().filter(|b| b.line != line).collect();
        if others.is_empty() {
            return None;
        }
        let pick = if forward {
            others.iter().find(|b| b.line > line).or(others.first())
        } else {
            others
                .iter()
                .rev()
                .find(|b| b.line < line)
                .or(others.last())
        };
        pick.map(|b| **b)
    }

    /// `Create Bookmark List…`: the error for an existing name.
    pub fn create_list(&mut self, name: &str) -> Result<(), String> {
        if self.lists.iter().any(|l| l == name) {
            return Err("A list with the specified name already exists".into());
        }
        self.lists.push(name.to_owned());
        Ok(())
    }

    /// The confirmation text of deleting a list.
    pub fn delete_list_prompt(name: &str) -> String {
        format!(
            "Are you sure you want to delete '{name}' bookmark list? This action can't be undone."
        )
    }
}
