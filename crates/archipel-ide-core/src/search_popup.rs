//! The model of the Search Everywhere popup (spec `docs/specs/search/04-search-everywhere.md`):
//! tabs, the text field, the history, the *Include non-project items* scope with its
//! auto-widening, the empty-list texts, the frozen result list with group jumps, and the `/`
//! commands. The window and the terminal frontends draw this state; the matching and the
//! provider lists are in [`crate::quick_nav`].

use crate::quick_nav::{Provider, SearchTab};
use std::collections::BTreeMap;

/// Texts kept per history (SRCH-SE-015).
const HISTORY_LIMIT: usize = 50;
/// After this many milliseconds the result list is frozen (SRCH-SE-023).
pub const FREEZE_AFTER_MS: u64 = 800;

/// The Search Everywhere history of one IDE session (SRCH-SE-015): per tab the last 50
/// distinct texts, newest last; the All tab shows the last 50 distinct texts of all tabs.
#[derive(Debug, Clone, Default)]
pub struct SearchHistory {
    per_tab: BTreeMap<&'static str, Vec<String>>,
    all: Vec<String>,
}

fn remember(list: &mut Vec<String>, text: &str) {
    list.retain(|entry| entry != text);
    list.push(text.to_owned());
    if list.len() > HISTORY_LIMIT {
        list.remove(0);
    }
}

impl SearchHistory {
    /// Saves a text (a popup closed with a non-empty field).
    pub fn push(&mut self, tab: SearchTab, text: &str) {
        if text.is_empty() {
            return;
        }
        remember(self.per_tab.entry(tab.id()).or_default(), text);
        remember(&mut self.all, text);
    }

    /// The history shown for `tab`, oldest first.
    pub fn entries(&self, tab: SearchTab) -> &[String] {
        if tab == SearchTab::All {
            &self.all
        } else {
            self.per_tab.get(tab.id()).map_or(&[], Vec::as_slice)
        }
    }
}

/// What the context offers as the initial text (SRCH-SE-004).
#[derive(Debug, Clone, Default)]
pub struct OpenContext {
    /// A predefined text from the invoking context.
    pub predefined: Option<String>,
    /// The editor selection.
    pub selection: Option<String>,
}

/// The popup state.
#[derive(Debug, Clone)]
pub struct SearchPopup {
    open: bool,
    tab: SearchTab,
    text: String,
    text_selected: bool,
    /// *Include non-project items* per tab, remembered for the project (SRCH-SE-016).
    everywhere: BTreeMap<&'static str, bool>,
    /// The user toggled the switch by hand: no more auto-widening for this popup.
    manual_toggle: bool,
    /// The pattern whose empty result switched the scope on (SRCH-SE-012).
    widened_for: Option<String>,
    include_disabled: bool,
    /// Position in the history while the user walks it with `Alt+Down` / `Alt+Up`.
    history_cursor: Option<usize>,
    /// `se.text.search`: the Text tab exists.
    text_search: bool,
    history: SearchHistory,
}

impl Default for SearchPopup {
    fn default() -> Self {
        Self {
            open: false,
            tab: SearchTab::All,
            text: String::new(),
            text_selected: false,
            everywhere: BTreeMap::new(),
            manual_toggle: false,
            widened_for: None,
            include_disabled: false,
            history_cursor: None,
            text_search: true,
            history: SearchHistory::default(),
        }
    }
}

impl SearchPopup {
    pub fn new(text_search: bool) -> Self {
        Self {
            text_search,
            ..Self::default()
        }
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn tab(&self) -> SearchTab {
        self.tab
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn text_selected(&self) -> bool {
        self.text_selected
    }

    pub fn history(&self) -> &SearchHistory {
        &self.history
    }

    /// The tabs in header order (SRCH-SE-010).
    pub fn tabs(&self) -> Vec<SearchTab> {
        SearchTab::ALL
            .into_iter()
            .filter(|tab| self.text_search || *tab != SearchTab::Text)
            .collect()
    }

    /// Opens the popup on `tab`; the initial text follows SRCH-SE-004.
    pub fn open(&mut self, tab: SearchTab, context: &OpenContext) {
        self.open = true;
        self.tab = tab;
        self.manual_toggle = false;
        self.widened_for = None;
        self.history_cursor = None;
        self.text_selected = true;
        let selection = context
            .selection
            .as_deref()
            .filter(|text| !text.is_empty() && !text.contains('\n'));
        self.text = if let Some(text) = &context.predefined {
            text.clone()
        } else if let Some(text) = selection {
            text.to_owned()
        } else if let Some(last) = self.history.entries(tab).last() {
            last.clone()
        } else {
            self.text_selected = false;
            String::new()
        };
    }

    /// Closes the popup and saves the text (SRCH-SE-014, SRCH-SE-015).
    pub fn close(&mut self) {
        if self.open {
            let (tab, text) = (self.tab, self.text.clone());
            self.history.push(tab, &text);
        }
        self.open = false;
        self.history_cursor = None;
    }

    /// The shortcut of `tab` was pressed (SRCH-SE-003): closed, it opens; on the selected tab it
    /// toggles *Include non-project items*; on another tab it switches and keeps the text.
    pub fn press_tab_shortcut(&mut self, tab: SearchTab, context: &OpenContext) {
        if !self.open {
            self.open(tab, context);
        } else if self.tab == tab {
            self.toggle_everywhere();
        } else {
            self.tab = tab;
        }
    }

    /// `Tab` / `Shift+Tab`: the next or previous tab with wrap-around (SRCH-SE-010).
    pub fn cycle_tab(&mut self, backward: bool) {
        let tabs = self.tabs();
        let at = tabs.iter().position(|t| *t == self.tab).unwrap_or(0);
        let len = tabs.len();
        self.tab = tabs[(at + if backward { len - 1 } else { 1 }) % len];
    }

    /// The user typed or pasted: `text` replaces the field. The remembered pattern of an
    /// auto-widened search switches the scope back off once it is gone (SRCH-SE-012).
    pub fn set_text(&mut self, text: &str) {
        self.text = text.lines().next().unwrap_or("").to_owned();
        self.text_selected = false;
        self.history_cursor = None;
        if let Some(failed) = &self.widened_for
            && failed.chars().count() > 1
            && !self.text.contains(failed.as_str())
        {
            self.widened_for = None;
            self.everywhere.insert(self.tab.id(), false);
        }
    }

    pub fn select_all(&mut self) {
        self.text_selected = true;
    }

    /// Typing over a selected field replaces it.
    pub fn type_text(&mut self, typed: &str) {
        let next = if self.text_selected {
            typed.to_owned()
        } else {
            format!("{}{typed}", self.text)
        };
        self.set_text(&next);
    }

    pub fn everywhere(&self) -> bool {
        self.everywhere.get(self.tab.id()).copied().unwrap_or(false)
    }

    /// A manual toggle of *Include non-project items* (SRCH-SE-012: switches the automatism off).
    pub fn toggle_everywhere(&mut self) {
        self.manual_toggle = true;
        self.widened_for = None;
        let now = !self.everywhere();
        self.everywhere.insert(self.tab.id(), now);
    }

    pub fn include_disabled(&self) -> bool {
        self.include_disabled
    }

    /// `Alt+D` on the Actions tab (SRCH-SE-046).
    pub fn toggle_include_disabled(&mut self) {
        self.include_disabled = !self.include_disabled;
    }

    /// A search finished with `count` rows. Returns whether the scope was widened and the search
    /// has to run again (SRCH-SE-012).
    pub fn results_arrived(&mut self, count: usize) -> bool {
        let pattern = self.text.trim();
        let navigation = matches!(
            self.tab,
            SearchTab::Classes | SearchTab::Files | SearchTab::Symbols
        );
        if count == 0
            && navigation
            && !pattern.is_empty()
            && !self.everywhere()
            && !self.manual_toggle
        {
            self.widened_for = Some(pattern.to_owned());
            self.everywhere.insert(self.tab.id(), true);
            return true;
        }
        false
    }

    /// `Alt+Down` (older) and `Alt+Up` (newer): walks the history of the tab, wrapping around
    /// (SRCH-SE-015).
    pub fn walk_history(&mut self, older: bool) {
        let entries = self.history.entries(self.tab).to_vec();
        if entries.is_empty() {
            return;
        }
        let last = entries.len() - 1;
        let at = self.history_cursor.unwrap_or(
            entries
                .iter()
                .position(|e| *e == self.text)
                .unwrap_or(last + usize::from(older)),
        );
        let next = if older {
            if at == 0 { last } else { at - 1 }
        } else if at >= last {
            0
        } else {
            at + 1
        };
        self.text = entries[next].clone();
        self.text_selected = true;
        self.history_cursor = Some(next);
    }

    /// The filter controls of the header of the selected tab (SRCH-SE-011).
    pub fn filters(&self) -> Vec<String> {
        match self.tab {
            SearchTab::All => vec![
                format!(
                    "switch 'Include non-project items' {}",
                    on_off(self.everywhere())
                ),
                "toggle 'Preview'".to_owned(),
                "type filter menu".to_owned(),
            ],
            SearchTab::Classes | SearchTab::Symbols => vec![
                "scope chooser 'Project Files'".to_owned(),
                format!(
                    "switch 'Include non-project items' {}",
                    on_off(self.everywhere())
                ),
            ],
            SearchTab::Files => vec![
                "scope chooser 'Project Files'".to_owned(),
                "file type filter".to_owned(),
            ],
            SearchTab::Actions => vec![format!(
                "check box 'Include disabled actions' {}",
                if self.include_disabled {
                    "checked"
                } else {
                    "unchecked"
                }
            )],
            SearchTab::Text => vec![
                "toggles Match case, Words, Regex".to_owned(),
                "file type combo 'All file types'".to_owned(),
                "scope chooser".to_owned(),
                "toggle 'Preview'".to_owned(),
            ],
        }
    }
}

fn on_off(on: bool) -> &'static str {
    if on { "on" } else { "off" }
}

// ------------------------------------------------------------------ empty texts

/// What the list is waiting for or lacks (SRCH-SE-024).
#[derive(Debug, Clone, Copy, Default)]
pub struct ListState {
    pub searching: bool,
    pub has_pattern: bool,
    pub dumb_mode: bool,
    pub everywhere: bool,
    pub filters_active: bool,
}

/// The text of an empty result list (SRCH-SE-020, SRCH-SE-024); `None` when the list is
/// compact and shows nothing (an empty field on a tab that needs a pattern).
pub fn empty_text(tab: SearchTab, state: ListState) -> Option<String> {
    let navigation = matches!(
        tab,
        SearchTab::Classes | SearchTab::Files | SearchTab::Symbols
    );
    if navigation && state.dumb_mode {
        return Some(format!(
            "Search in '{}' will be available once project analysis is finished",
            tab.label()
        ));
    }
    if state.searching {
        return Some("Searching\u{2026}".to_owned());
    }
    if !state.has_pattern {
        return None;
    }
    let mut text = if tab == SearchTab::All {
        "Nothing found".to_owned()
    } else {
        format!("No {} with such a name found", tab.label().to_lowercase())
    };
    if navigation && !state.everywhere {
        text.push_str(". Set scope to All Places");
    }
    if state.filters_active {
        text.push_str(". Reset filter");
    }
    Some(text)
}

// -------------------------------------------------------------------- the list

/// One row of the result list as far as ordering is concerned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListRow {
    pub provider: Provider,
    pub id: String,
    pub weight: i32,
}

/// The result list with the freeze of SRCH-SE-023 and the selection rules of SRCH-SE-025/026.
#[derive(Debug, Clone, Default)]
pub struct ResultList {
    rows: Vec<ListRow>,
    selected: Option<usize>,
    frozen: bool,
    /// `(tab, text)` and the id that was selected when the popup closed (SRCH-SE-026).
    remembered: Option<(SearchTab, String, String)>,
}

impl ResultList {
    pub fn rows(&self) -> &[ListRow] {
        &self.rows
    }

    pub fn selected(&self) -> Option<&ListRow> {
        self.selected.and_then(|i| self.rows.get(i))
    }

    pub fn selected_index(&self) -> Option<usize> {
        self.selected
    }

    /// A new search replaces the list once its first item arrives.
    pub fn restart(&mut self) {
        self.rows.clear();
        self.selected = None;
        self.frozen = false;
    }

    /// Adds an item that arrived `elapsed_ms` after the search started. Before the freeze the
    /// row goes to its place by weight (higher first, then the provider's sort weight); after
    /// it, below the rows already shown (SRCH-SE-023). The first row is selected when it
    /// arrives and the selection stays on its row afterwards (SRCH-SE-025).
    pub fn insert(&mut self, row: ListRow, elapsed_ms: u64) {
        if self.rows.iter().any(|r| r.id == row.id) {
            return;
        }
        if elapsed_ms >= FREEZE_AFTER_MS {
            self.frozen = true;
        }
        let at = if self.frozen {
            self.rows.len()
        } else {
            self.rows
                .iter()
                .position(|r| {
                    (row.weight, std::cmp::Reverse(row.provider.sort_weight()))
                        > (r.weight, std::cmp::Reverse(r.provider.sort_weight()))
                })
                .unwrap_or(self.rows.len())
        };
        self.rows.insert(at, row);
        match self.selected {
            None => self.selected = Some(0),
            Some(i) if at <= i && self.selected.is_some() && !self.frozen => {
                // The selected row moved down with its content.
                self.selected = Some(i + 1);
            }
            _ => {}
        }
    }

    pub fn select(&mut self, index: usize) {
        if index < self.rows.len() {
            self.selected = Some(index);
        }
    }

    /// `Ctrl+Down` / `Ctrl+Up`: the first row of the next or previous provider group.
    pub fn jump_group(&mut self, forward: bool) {
        let Some(at) = self.selected else { return };
        let here = self.rows[at].provider;
        if forward {
            if let Some(i) = (at + 1..self.rows.len()).find(|i| self.rows[*i].provider != here) {
                self.selected = Some(i);
            }
        } else if let Some(prev) = (0..at).rev().find(|i| self.rows[*i].provider != here) {
            // The first row of the previous group.
            let group = self.rows[prev].provider;
            let mut first = prev;
            while first > 0 && self.rows[first - 1].provider == group {
                first -= 1;
            }
            self.selected = Some(first);
        }
    }

    /// The popup closed: remember the selection for the next opening of the same tab and text.
    pub fn remember_selection(&mut self, tab: SearchTab, text: &str) {
        if let Some(row) = self.selected() {
            self.remembered = Some((tab, text.to_owned(), row.id.clone()));
        }
    }

    /// The same tab and text were reopened: select the remembered row if it is visible.
    pub fn restore_selection(&mut self, tab: SearchTab, text: &str) {
        if let Some((t, x, id)) = &self.remembered
            && *t == tab
            && x == text
            && let Some(at) = self.rows.iter().position(|r| &r.id == id)
        {
            self.selected = Some(at);
        }
    }
}

// ---------------------------------------------------------------------- commands

/// A `/command` of the field (SRCH-SE-060): name and description.
pub const COMMANDS: [(&str, &str); 3] = [
    ("/debug", "Start Run Configuration in 'Debug' mode"),
    ("/plugins", "Plugins: {0}"),
    ("/run", "Start Run Configuration in 'Run' mode"),
];

/// The hint of the field when it holds no `/`.
pub const COMMAND_HINT: &str = "Type / to see commands";

/// The commands listed for `typed` (it starts with the accelerator `/`): those whose name starts
/// with it first, then ascending.
pub fn commands_for(typed: &str) -> Vec<(&'static str, &'static str)> {
    if !typed.starts_with('/') {
        return Vec::new();
    }
    let word = typed.split(' ').next().unwrap_or(typed).to_lowercase();
    let mut found: Vec<(&str, &str)> = COMMANDS
        .iter()
        .copied()
        .filter(|(name, _)| name.contains(word.trim_start_matches('/')) || word == "/")
        .collect();
    found.sort_by_key(|(name, _)| (!name.starts_with(&word), *name));
    found
}

/// `Tab` completes the selected command: the text becomes `/<command> `.
pub fn complete_command(command: &str) -> String {
    format!("{command} ")
}

// ----------------------------------------------------------------------- double shift

/// Detects the double Shift gesture (SRCH-SE-002): two presses within the interval with
/// nothing in between; a third quick press does not toggle.
#[derive(Debug, Clone, Default)]
pub struct DoubleShift {
    last_press_ms: Option<u64>,
}

impl DoubleShift {
    /// Platform double-click interval in milliseconds (open question 1 of the spec).
    pub const INTERVAL_MS: u64 = 500;

    /// A Shift press at `now_ms`; returns whether it completes the gesture.
    pub fn press(&mut self, now_ms: u64) -> bool {
        match self.last_press_ms {
            Some(prev) if now_ms.saturating_sub(prev) <= Self::INTERVAL_MS => {
                self.last_press_ms = None;
                true
            }
            _ => {
                self.last_press_ms = Some(now_ms);
                false
            }
        }
    }

    /// Any other key or a mouse click resets the gesture.
    pub fn interrupt(&mut self) {
        self.last_press_ms = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_walks_older_and_wraps() {
        // SRCH-SE-015
        let mut popup = SearchPopup::default();
        for (text, tab) in [
            ("a", SearchTab::Classes),
            ("b", SearchTab::Classes),
            ("a", SearchTab::Classes),
        ] {
            popup.open(tab, &OpenContext::default());
            popup.set_text(text);
            popup.close();
        }
        popup.open(SearchTab::Classes, &OpenContext::default());
        assert_eq!(popup.text(), "a");
        popup.walk_history(true);
        assert_eq!(popup.text(), "b");
        popup.walk_history(true);
        assert_eq!(popup.text(), "a");
    }

    #[test]
    fn the_scope_widens_once_and_narrows_again() {
        // SRCH-SE-012
        let mut popup = SearchPopup::default();
        popup.open(SearchTab::Classes, &OpenContext::default());
        popup.set_text("Zzz");
        assert!(popup.results_arrived(0));
        assert!(popup.everywhere());
        popup.set_text("Hash");
        assert!(!popup.everywhere());
    }

    #[test]
    fn double_shift_needs_two_quick_presses() {
        // SRCH-SE-002
        let mut shift = DoubleShift::default();
        assert!(!shift.press(0));
        assert!(shift.press(200));
        assert!(!shift.press(300));
        assert!(!shift.press(1_000));
    }
}
