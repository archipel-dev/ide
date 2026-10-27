//! Search Everywhere, Go to Class/File/Symbol, Recent Locations and the Switcher:
//! engine side. (Code navigation through language servers lives in `navigation`.)
//!
//! The popups live in `archipel-ide-ui`; everything they list, rank or cycle is
//! decided here so it can be tested without a window. Commands (`navigate.*`)
//! only record a [`NavigationRequest`] that the UI turns into a popup, or drive
//! the [`Switcher`] state directly.

use crate::{IdeState, ToolWindow, spec};
use archipel_ide_command::{Command, CommandError, CommandRegistry};
use archipel_ide_language::structure::{StructureItem, SymbolKind};
use archipel_ide_lsp::completion::{MatchClass, fuzzy_match, match_class};
use archipel_ide_lsp::lsp_types;
use archipel_ide_lsp::{WorkspaceSymbolHit, WorkspaceSymbolRequestHandle};
use archipel_ide_project::FileEntry;
use archipel_ide_search::TextMatch;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

/// Tabs of the Search Everywhere popup.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SearchTab {
    #[default]
    All,
    Classes,
    Files,
    Symbols,
    Actions,
    Text,
}

impl SearchTab {
    pub const ALL: [SearchTab; 6] = [
        SearchTab::All,
        SearchTab::Classes,
        SearchTab::Files,
        SearchTab::Symbols,
        SearchTab::Actions,
        SearchTab::Text,
    ];

    pub fn id(self) -> &'static str {
        match self {
            SearchTab::All => "all",
            SearchTab::Classes => "classes",
            SearchTab::Files => "files",
            SearchTab::Symbols => "symbols",
            SearchTab::Actions => "actions",
            SearchTab::Text => "text",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            SearchTab::All => "All",
            SearchTab::Classes => "Classes",
            SearchTab::Files => "Files",
            SearchTab::Symbols => "Symbols",
            SearchTab::Actions => "Actions",
            SearchTab::Text => "Text",
        }
    }

    pub fn parse(id: &str) -> Option<SearchTab> {
        SearchTab::ALL.into_iter().find(|tab| tab.id() == id)
    }

    /// The next (or previous) tab, wrapping around.
    pub fn step(self, backward: bool) -> SearchTab {
        let index = SearchTab::ALL
            .iter()
            .position(|tab| *tab == self)
            .unwrap_or(0);
        let len = SearchTab::ALL.len();
        SearchTab::ALL[(index + if backward { len - 1 } else { 1 }) % len]
    }
}

/// A popup the UI should open, set by a `navigate.*` command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavigationRequest {
    SearchEverywhere(SearchTab),
    RecentFiles,
    RecentLocations,
}

// ---------------------------------------------------------------- ranking

/// Candidates ranked best-first; `positions` are char indices to highlight.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ranked<T> {
    pub item: T,
    pub positions: Vec<usize>,
    class: MatchClass,
    score: i32,
}

/// Camel-hump ranks `candidates` against `query` (the completion matcher):
/// exact and prefix matches first, then humps, then tighter matches, then
/// shorter names. An empty query keeps the candidates' own order.
pub fn rank<T>(
    query: &str,
    candidates: impl IntoIterator<Item = (T, String)>,
    limit: usize,
) -> Vec<Ranked<T>> {
    rank_with(false, query, candidates, limit)
}

/// Like [`rank`], but for names made of several words (actions, file names):
/// letters scattered over the words (`main` in `Markdown: Insert Image`) do not
/// match. A strict match is a contiguous run at a word start, initials of
/// words, or camel humps inside one word.
pub fn rank_strict<T>(
    query: &str,
    candidates: impl IntoIterator<Item = (T, String)>,
    limit: usize,
) -> Vec<Ranked<T>> {
    rank_with(true, query, candidates, limit)
}

/// Whether the matched `positions` of `name` read as a deliberate match.
fn strict_match(name: &str, positions: &[usize]) -> bool {
    let chars: Vec<char> = name.chars().collect();
    let word_start = |index: usize| {
        index == 0
            || chars.get(index - 1).is_some_and(|c| !c.is_alphanumeric())
                && chars.get(index).is_some_and(|c| c.is_alphanumeric())
    };
    let Some(&first) = positions.first() else {
        return true;
    };
    let contiguous = positions.windows(2).all(|pair| pair[1] == pair[0] + 1);
    if contiguous && word_start(first) {
        return true;
    }
    if positions.len() >= 2
        && positions.iter().all(|&p| word_start(p))
        && positions.windows(2).all(|pair| pair[1] != pair[0] + 1)
    {
        return true;
    }
    // Camel humps inside a single word.
    let (lo, hi) = (first, positions[positions.len() - 1]);
    !chars[lo..=hi.min(chars.len() - 1)]
        .iter()
        .any(|c| !c.is_alphanumeric())
}

fn rank_with<T>(
    strict: bool,
    query: &str,
    candidates: impl IntoIterator<Item = (T, String)>,
    limit: usize,
) -> Vec<Ranked<T>> {
    let mut ranked: Vec<(Ranked<T>, usize)> = Vec::new();
    for (item, name) in candidates {
        let (class, score, positions) = if query.is_empty() {
            (MatchClass::Inner, 0, Vec::new())
        } else {
            let Some(found) = fuzzy_match(query, &name) else {
                continue;
            };
            if strict && !strict_match(&name, &found.positions) {
                continue;
            }
            (
                match_class(query, &name, &found),
                found.score,
                found.positions,
            )
        };
        ranked.push((
            Ranked {
                item,
                positions,
                class,
                score,
            },
            name.chars().count(),
        ));
    }
    if !query.is_empty() {
        ranked.sort_by(|(a, a_len), (b, b_len)| {
            a.class
                .cmp(&b.class)
                .then(b.score.cmp(&a.score))
                .then(a_len.cmp(b_len))
        });
    }
    ranked.into_iter().map(|(r, _)| r).take(limit).collect()
}

/// Whether `name` matches `query` with camel-hump semantics (empty matches all).
pub fn matches(query: &str, name: &str) -> bool {
    query.is_empty() || fuzzy_match(query, name).is_some()
}

/// Splits `Name:12` into the name pattern and a one-based line.
pub fn split_line_suffix(query: &str) -> (&str, Option<usize>) {
    if let Some((name, line)) = query.rsplit_once(':')
        && !line.is_empty()
        && let Ok(line) = line.parse::<usize>()
    {
        return (name, Some(line.max(1)));
    }
    (query, None)
}

// ------------------------------------------------------------ symbol index

/// A class or symbol the popups can navigate to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolEntry {
    pub name: String,
    pub kind: SymbolKind,
    /// Project-relative for indexed files, absolute for files outside the project.
    pub path: PathBuf,
    /// Zero-based line.
    pub line: usize,
    pub container: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Stamp {
    len: u64,
    modified: Option<SystemTime>,
}

/// Tree-sitter outlines of project files, cached by size and modification time.
#[derive(Default)]
pub struct SymbolIndex {
    files: HashMap<PathBuf, (Option<Stamp>, Vec<SymbolEntry>)>,
}

const MAX_INDEXED_FILE: u64 = 1_000_000;
const MAX_INDEXED_FILES: usize = 5_000;

impl SymbolIndex {
    /// Indexes uncached files until `budget` runs out; returns whether every
    /// supported file is now indexed. `open` supplies the live text of open
    /// documents, which always win over the file on disk.
    pub fn refresh(
        &mut self,
        root: &Path,
        files: &[FileEntry],
        open: &[(PathBuf, String)],
        budget: Duration,
    ) -> bool {
        let started = Instant::now();
        for (path, text) in open {
            let symbols = entries(path, outline(path, text));
            self.files.insert(path.clone(), (None, symbols));
        }
        let mut complete = true;
        for entry in files
            .iter()
            .filter(|entry| !entry.is_dir)
            .take(MAX_INDEXED_FILES)
        {
            let path = &entry.relative_path;
            if open.iter().any(|(open, _)| open == path) {
                continue;
            }
            if archipel_ide_language::registry::detect(path, "").is_none_or(|spec| !spec.has_tags())
            {
                continue;
            }
            let Ok(meta) = std::fs::metadata(root.join(path)) else {
                continue;
            };
            let stamp = Stamp {
                len: meta.len(),
                modified: meta.modified().ok(),
            };
            if self
                .files
                .get(path)
                .is_some_and(|(cached, _)| cached.as_ref() == Some(&stamp))
            {
                continue;
            }
            if started.elapsed() >= budget {
                complete = false;
                break;
            }
            let symbols = if meta.len() > MAX_INDEXED_FILE {
                Vec::new()
            } else {
                std::fs::read_to_string(root.join(path))
                    .map(|text| entries(path, outline(path, &text)))
                    .unwrap_or_default()
            };
            self.files.insert(path.clone(), (Some(stamp), symbols));
        }
        complete
    }

    pub fn clear(&mut self) {
        self.files.clear();
    }

    pub fn all(&self) -> impl Iterator<Item = &SymbolEntry> {
        self.files.values().flat_map(|(_, symbols)| symbols.iter())
    }
}

/// Declarations of `source` in file order, flattened from the file structure
/// (tree-sitter tags queries); empty for languages without one.
fn outline(path: &Path, source: &str) -> Vec<(String, SymbolKind, usize)> {
    fn flatten(items: &[StructureItem], out: &mut Vec<(String, SymbolKind, usize)>) {
        for item in items {
            out.push((item.name.clone(), item.kind, item.line));
            flatten(&item.children, out);
        }
    }
    let mut out = Vec::new();
    flatten(
        &archipel_ide_language::analyze(path, source).structure(),
        &mut out,
    );
    out.sort_by_key(|(_, _, line)| *line);
    out
}

/// Declarations Go to Class lists: classes, interfaces, enums and type aliases.
pub fn is_class_like(kind: SymbolKind) -> bool {
    matches!(
        kind,
        SymbolKind::Class | SymbolKind::Interface | SymbolKind::Enum | SymbolKind::Type
    )
}

fn entries(path: &Path, symbols: Vec<(String, SymbolKind, usize)>) -> Vec<SymbolEntry> {
    symbols
        .into_iter()
        .map(|(name, kind, line)| SymbolEntry {
            name,
            kind,
            path: path.to_owned(),
            line,
            container: None,
        })
        .collect()
}

fn lsp_kind(kind: lsp_types::SymbolKind) -> SymbolKind {
    use lsp_types::SymbolKind as K;
    match kind {
        K::CLASS | K::STRUCT | K::OBJECT => SymbolKind::Class,
        K::INTERFACE => SymbolKind::Interface,
        K::ENUM => SymbolKind::Enum,
        K::FUNCTION => SymbolKind::Function,
        K::METHOD | K::CONSTRUCTOR => SymbolKind::Method,
        K::FIELD | K::PROPERTY | K::ENUM_MEMBER => SymbolKind::Field,
        K::CONSTANT | K::VARIABLE => SymbolKind::Constant,
        K::MODULE | K::NAMESPACE | K::PACKAGE => SymbolKind::Module,
        K::TYPE_PARAMETER => SymbolKind::Type,
        _ => SymbolKind::Other,
    }
}

// -------------------------------------------------------------- providers

/// An action row: a command with its keymap shortcut.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionCandidate {
    pub id: String,
    pub label: String,
    pub category: String,
    pub shortcut: String,
    pub enabled: bool,
}

/// One row of the popup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Hit {
    File { path: PathBuf, line: Option<usize> },
    Symbol(SymbolEntry),
    Action(ActionCandidate),
    Text(TextMatch),
}

/// What the providers draw from.
pub struct SearchSources<'a> {
    pub files: &'a [FileEntry],
    /// Most recently used first; listed for an empty Files query.
    pub recent: &'a [PathBuf],
    pub symbols: Vec<&'a SymbolEntry>,
    /// Extra results from a running language server (deduplicated here).
    pub server_symbols: &'a [SymbolEntry],
    pub actions: &'a [ActionCandidate],
    /// Occurrences of the query in project files (from the background text search).
    pub text: &'a [TextMatch],
    /// "Include non-project items": symbols of libraries outside the project root.
    pub include_non_project: bool,
}

/// Rows per section on the All tab.
const ALL_SECTION: [(SearchTab, usize); 5] = [
    (SearchTab::Classes, 4),
    (SearchTab::Symbols, 4),
    (SearchTab::Files, 4),
    (SearchTab::Actions, 5),
    (SearchTab::Text, 5),
];
const TAB_LIMIT: usize = 30;

/// Runs the provider(s) of `tab`. The All tab concatenates short sections.
pub fn search(tab: SearchTab, query: &str, sources: &SearchSources) -> Vec<Ranked<Hit>> {
    let query = query.trim();
    if tab == SearchTab::All {
        return ALL_SECTION
            .into_iter()
            .flat_map(|(tab, limit)| provider(tab, query, sources, limit))
            .collect();
    }
    provider(tab, query, sources, TAB_LIMIT)
}

fn provider(
    tab: SearchTab,
    query: &str,
    sources: &SearchSources,
    limit: usize,
) -> Vec<Ranked<Hit>> {
    match tab {
        SearchTab::Files => files(query, sources, limit),
        SearchTab::Classes => symbols(query, sources, limit, true),
        SearchTab::Symbols => symbols(query, sources, limit, false),
        SearchTab::Text => sources
            .text
            .iter()
            .take(limit)
            .map(|found| Ranked {
                item: Hit::Text(found.clone()),
                positions: Vec::new(),
                class: MatchClass::Inner,
                score: 0,
            })
            .collect(),
        SearchTab::Actions => rank_strict(
            query,
            sources
                .actions
                .iter()
                .map(|action| (Hit::Action(action.clone()), action.label.clone())),
            limit,
        ),
        SearchTab::All => Vec::new(),
    }
}

fn files(query: &str, sources: &SearchSources, limit: usize) -> Vec<Ranked<Hit>> {
    let (pattern, line) = split_line_suffix(query);
    if pattern.is_empty() {
        return sources
            .recent
            .iter()
            .take(limit)
            .map(|path| Ranked {
                item: Hit::File {
                    path: path.clone(),
                    line,
                },
                positions: Vec::new(),
                class: MatchClass::Inner,
                score: 0,
            })
            .collect();
    }
    rank_strict(
        pattern,
        sources
            .files
            .iter()
            .filter(|entry| !entry.is_dir)
            .map(|entry| {
                let name = file_name(&entry.relative_path);
                (
                    Hit::File {
                        path: entry.relative_path.clone(),
                        line,
                    },
                    name,
                )
            }),
        limit,
    )
}

fn symbols(query: &str, sources: &SearchSources, limit: usize, classes: bool) -> Vec<Ranked<Hit>> {
    let keep = |kind: SymbolKind| {
        if classes {
            is_class_like(kind)
        } else {
            !is_class_like(kind)
        }
    };
    let mut seen = std::collections::HashSet::new();
    let candidates: Vec<(Hit, String)> = sources
        .symbols
        .iter()
        .copied()
        .chain(sources.server_symbols.iter())
        .filter(|symbol| keep(symbol.kind))
        .filter(|symbol| sources.include_non_project || !symbol.path.is_absolute())
        .filter(|symbol| seen.insert((symbol.path.clone(), symbol.name.clone(), symbol.line)))
        .map(|symbol| (Hit::Symbol(symbol.clone()), symbol.name.clone()))
        .collect();
    let mut ranked = rank(query, candidates, usize::MAX);
    if query.is_empty() {
        ranked.sort_by(|a, b| hit_name(&a.item).cmp(hit_name(&b.item)));
    }
    ranked.truncate(limit);
    ranked
}

fn hit_name(hit: &Hit) -> &str {
    match hit {
        Hit::Symbol(symbol) => &symbol.name,
        _ => "",
    }
}

/// File name of a project-relative path.
pub fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

// -------------------------------------------------------- server symbols

/// A `workspace/symbol` query in flight and its latest answer.
#[derive(Default)]
pub struct ServerSymbols {
    request: Option<(WorkspaceSymbolRequestHandle, String)>,
    /// Query the current `results` answer.
    pub answered: Option<String>,
    pub results: Vec<SymbolEntry>,
}

impl ServerSymbols {
    pub fn clear(&mut self) {
        self.request = None;
        self.answered = None;
        self.results.clear();
    }
}

// --------------------------------------------------------------- switcher

/// An entry of the Switcher: an open file or a tool window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SwitcherEntry {
    File(PathBuf),
    ToolWindow(ToolWindow),
}

/// The Ctrl+Tab popup: held Ctrl cycles, releasing it commits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Switcher {
    pub entries: Vec<SwitcherEntry>,
    pub selected: usize,
}

impl Switcher {
    /// Opens on the entry after (or, backward, before) the current one, which
    /// is first in the list: Ctrl+Tab jumps to the previous file at once.
    pub fn open(entries: Vec<SwitcherEntry>, backward: bool) -> Option<Switcher> {
        if entries.is_empty() {
            return None;
        }
        let mut switcher = Switcher {
            entries,
            selected: 0,
        };
        switcher.cycle(backward);
        Some(switcher)
    }

    pub fn cycle(&mut self, backward: bool) {
        let len = self.entries.len();
        self.selected = (self.selected + if backward { len - 1 } else { 1 }) % len;
    }

    pub fn selected_entry(&self) -> &SwitcherEntry {
        &self.entries[self.selected]
    }
}

// ------------------------------------------------------ recent locations

/// A place the caret was, for Recent Locations (Ctrl+Shift+E).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecentLocation {
    pub path: PathBuf,
    /// Zero-based line.
    pub line: usize,
    /// Text of the line when the location was recorded.
    pub preview: String,
}

const MAX_LOCATIONS: usize = 50;
/// Caret moves within this many lines of the newest location update it.
const SAME_PLACE_LINES: usize = 5;

/// Navigation state kept by [`IdeState`].
#[derive(Default)]
pub struct QuickNavState {
    /// Popup the UI should open next.
    pub request: Option<NavigationRequest>,
    pub switcher: Option<Switcher>,
    /// Newest first.
    pub locations: Vec<RecentLocation>,
    pub index: SymbolIndex,
    /// Whether the last [`SymbolIndex::refresh`] covered every file.
    pub index_complete: bool,
    pub server: ServerSymbols,
    /// Background text search feeding the Text tab of Search Everywhere.
    pub text: parking_lot::Mutex<archipel_ide_search::SearchService>,
    pub text_results: Vec<TextMatch>,
    pub text_query: String,
}

impl QuickNavState {
    /// Records the caret at `path:line`. Small moves update the newest entry
    /// instead of piling up near-duplicates.
    pub fn note_location(&mut self, path: &Path, line: usize, preview: &str) {
        if let Some(newest) = self.locations.first_mut()
            && newest.path == path
            && newest.line.abs_diff(line) <= SAME_PLACE_LINES
        {
            newest.line = line;
            newest.preview = preview.to_owned();
            return;
        }
        self.locations.retain(|existing| {
            !(existing.path == path && existing.line.abs_diff(line) <= SAME_PLACE_LINES)
        });
        self.locations.insert(
            0,
            RecentLocation {
                path: path.to_owned(),
                line,
                preview: preview.to_owned(),
            },
        );
        self.locations.truncate(MAX_LOCATIONS);
    }

    pub fn forget(&mut self, path: &Path) {
        self.locations
            .retain(|location| !location.path.starts_with(path));
    }
}

impl IdeState {
    /// Entries for the Switcher: open files, most recently used first, then
    /// the tool windows that are currently visible.
    pub fn switcher_entries(&self) -> Vec<SwitcherEntry> {
        let open: Vec<&PathBuf> = self
            .documents
            .iter()
            .map(|document| &document.path)
            .collect();
        let mut files: Vec<PathBuf> = self
            .recent_files
            .iter()
            .filter(|path| open.contains(path))
            .cloned()
            .collect();
        let rest: Vec<PathBuf> = open
            .into_iter()
            .filter(|path| !files.contains(path))
            .cloned()
            .collect();
        files.extend(rest);
        // The active document leads so the first Ctrl+Tab lands on the previous one.
        if let Some(active) = self.active_document.and_then(|i| self.documents.get(i))
            && let Some(position) = files.iter().position(|path| *path == active.path)
        {
            let active = files.remove(position);
            files.insert(0, active);
        }
        files
            .into_iter()
            .map(SwitcherEntry::File)
            .chain(
                ToolWindow::ALL
                    .into_iter()
                    .filter(|window| self.tool_windows.is_visible(*window))
                    .map(SwitcherEntry::ToolWindow),
            )
            .collect()
    }

    /// The popup a `navigate.*` command asked for; the UI takes it once per frame.
    pub fn take_navigation_request(&self) -> Option<NavigationRequest> {
        self.search_everywhere()?.write().request.take()
    }

    /// The pending popup request, without taking it.
    pub fn navigation_request(&self) -> Option<NavigationRequest> {
        self.search_everywhere()?.read().request
    }

    /// Recent Locations, newest first.
    pub fn recent_locations(&self) -> Vec<RecentLocation> {
        self.search_everywhere()
            .map(|nav| nav.read().locations.clone())
            .unwrap_or_default()
    }

    /// A copy of the open Switcher.
    pub fn switcher(&self) -> Option<Switcher> {
        self.search_everywhere()?.read().switcher.clone()
    }

    /// Selects `index` in the open Switcher.
    pub fn select_switcher_entry(&self, index: usize) {
        if let Some(nav) = self.search_everywhere()
            && let Some(switcher) = nav.write().switcher.as_mut()
        {
            switcher.selected = index;
        }
    }

    /// Whether the Text tab's background search is running.
    pub fn text_search_running(&self) -> bool {
        self.search_everywhere()
            .is_some_and(|nav| nav.read().text.lock().is_running())
    }

    /// Opens the Switcher, or cycles it when already open.
    pub fn switcher_advance(&mut self, backward: bool) {
        let Some(nav) = self.search_everywhere() else {
            return;
        };
        let entries = self.switcher_entries();
        let mut nav = nav.write();
        if let Some(switcher) = &mut nav.switcher {
            switcher.cycle(backward);
        } else {
            nav.switcher = Switcher::open(entries, backward);
        }
    }

    /// Records the active document's caret as a recent location.
    pub fn record_caret_location(&mut self) {
        let Some(document) = self
            .active_document
            .and_then(|index| self.documents.get(index))
        else {
            return;
        };
        let Ok(line) = document
            .text
            .char_to_line_column(document.selections.primary().head)
            .map(|(line, _)| line)
        else {
            return;
        };
        let preview = document.text.line(line).unwrap_or_default();
        let (path, preview) = (document.path.clone(), preview.trim().to_owned());
        if let Some(nav) = self.search_everywhere() {
            nav.write().note_location(&path, line, &preview);
        }
    }

    /// Indexes more files for Go to Class / Symbol within `budget`.
    pub fn refresh_symbol_index(&mut self, budget: Duration) {
        let Some(project) = &self.project else {
            return;
        };
        let open: Vec<(PathBuf, String)> = self
            .documents
            .iter()
            .map(|document| (document.path.clone(), document.text.text()))
            .collect();
        let root = project.root().to_owned();
        if let Some(nav) = self.search_everywhere() {
            let mut nav = nav.write();
            nav.index_complete = nav.index.refresh(&root, &self.files, &open, budget);
        }
    }

    /// Starts (or restarts) a `workspace/symbol` request for `query` through
    /// a running language server; a no-op without one.
    pub fn request_server_symbols(&mut self, query: &str) {
        let Some(nav) = self.search_everywhere() else {
            return;
        };
        let mut nav = nav.write();
        let server = &mut nav.server;
        if query.is_empty() {
            server.clear();
            return;
        }
        if server.answered.as_deref() == Some(query)
            || server
                .request
                .as_ref()
                .is_some_and(|(_, asked)| asked == query)
        {
            return;
        }
        let (Some(project), Some(active)) = (
            &self.project,
            self.active_document.and_then(|i| self.documents.get(i)),
        ) else {
            return;
        };
        let anchor = project.root().join(&active.path);
        if let Some((handle, _)) = server.request.take() {
            self.lsp.cancel_workspace_symbols(&handle);
        }
        if let Ok(handle) = self.lsp.request_workspace_symbols(&anchor, query) {
            nav.server.request = Some((handle, query.to_owned()));
        }
    }

    /// Collects an arrived `workspace/symbol` answer.
    pub fn poll_server_symbols(&mut self) {
        let Some(nav) = self.search_everywhere() else {
            return;
        };
        let Some((handle, query)) = nav.read().server.request.clone() else {
            return;
        };
        let Some(answer) = self.lsp.take_workspace_symbols(&handle) else {
            return;
        };
        let root = self
            .project
            .as_ref()
            .map(|project| project.root().to_owned());
        let mut nav = nav.write();
        let server = &mut nav.server;
        server.request = None;
        server.answered = Some(query);
        server.results = answer
            .map(|hits| {
                hits.into_iter()
                    .map(|hit| server_entry(hit, root.as_deref()))
                    .collect()
            })
            .unwrap_or_default();
    }
}

fn server_entry(hit: WorkspaceSymbolHit, root: Option<&Path>) -> SymbolEntry {
    let path = root
        .and_then(|root| hit.path.strip_prefix(root).ok())
        .map(Path::to_owned)
        .unwrap_or(hit.path);
    SymbolEntry {
        name: hit.name,
        kind: lsp_kind(hit.kind),
        path,
        line: hit.line,
        container: hit.container,
    }
}

// --------------------------------------------------------------- commands

pub(crate) fn register(registry: &mut CommandRegistry<IdeState>) {
    for (id, label, tab) in [
        (
            "navigate.searchEverywhere",
            "Search Everywhere",
            SearchTab::All,
        ),
        ("navigate.gotoClass", "Go to Class", SearchTab::Classes),
        ("navigate.gotoFile", "Go to File", SearchTab::Files),
        ("navigate.gotoSymbol", "Go to Symbol", SearchTab::Symbols),
        ("navigate.findAction", "Find Action", SearchTab::Actions),
    ] {
        registry
            .register(Command::new(
                spec(id, label, "Navigate", None, r#"{"tab":"string?"}"#),
                |_| true,
                move |state: &mut IdeState, params| {
                    let tab = match params.get("tab").and_then(Value::as_str) {
                        Some(name) => SearchTab::parse(name).ok_or_else(|| {
                            CommandError::InvalidParams(format!("unknown search tab `{name}`"))
                        })?,
                        None => tab,
                    };
                    if let Some(nav) = state.search_everywhere() {
                        nav.write().request = Some(NavigationRequest::SearchEverywhere(tab));
                    }
                    Ok(json!({"tab": tab.id()}))
                },
            ))
            .expect("unique built-in id");
    }
    registry
        .register(Command::new(
            spec(
                "navigate.recentFiles",
                "Recent Files",
                "Navigate",
                None,
                "{}",
            ),
            |_| true,
            |state: &mut IdeState, _| {
                if let Some(nav) = state.search_everywhere() {
                    nav.write().request = Some(NavigationRequest::RecentFiles);
                }
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "navigate.recentLocations",
                "Recent Locations",
                "Navigate",
                None,
                "{}",
            ),
            |_| true,
            |state: &mut IdeState, _| {
                if let Some(nav) = state.search_everywhere() {
                    nav.write().request = Some(NavigationRequest::RecentLocations);
                }
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "navigate.switcher",
                "Switcher",
                "Navigate",
                None,
                r#"{"backward":"bool?"}"#,
            ),
            |state: &IdeState| !state.documents.is_empty() || state.project.is_some(),
            |state: &mut IdeState, params| {
                let backward = params
                    .get("backward")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                state.switcher_advance(backward);
                let selected = state
                    .search_everywhere()
                    .and_then(|nav| nav.read().switcher.as_ref().map(|s| s.selected));
                Ok(json!({ "selected": selected }))
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "navigate.switcherCommit",
                "Switch to Selected Entry",
                "Navigate",
                None,
                "{}",
            ),
            |state: &IdeState| state.switcher_open(),
            |state: &mut IdeState, _| {
                let Some(switcher) = state
                    .search_everywhere()
                    .and_then(|nav| nav.write().switcher.take())
                else {
                    return Ok(Value::Null);
                };
                match switcher.selected_entry() {
                    SwitcherEntry::File(path) => {
                        let index = state
                            .documents
                            .iter()
                            .position(|document| &document.path == path)
                            .ok_or_else(|| CommandError::Failed("file is no longer open".into()))?;
                        state
                            .activate_document(index)
                            .map_err(CommandError::Failed)?;
                    }
                    SwitcherEntry::ToolWindow(window) => state.tool_windows.show(*window),
                }
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "navigate.switcherCancel",
                "Close Switcher",
                "Navigate",
                None,
                "{}",
            ),
            |state: &IdeState| state.switcher_open(),
            |state: &mut IdeState, _| {
                if let Some(nav) = state.search_everywhere() {
                    nav.write().switcher = None;
                }
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::built_in_commands;

    fn names<T: Clone>(ranked: &[Ranked<T>], name: impl Fn(&T) -> String) -> Vec<String> {
        ranked.iter().map(|r| name(&r.item)).collect()
    }

    fn candidates(names: &[&str]) -> Vec<(String, String)> {
        names
            .iter()
            .map(|n| ((*n).to_owned(), (*n).to_owned()))
            .collect()
    }

    #[test]
    fn ranking_prefers_exact_then_prefix_then_humps_then_inner() {
        let ranked = rank(
            "gn",
            candidates(&["signName", "getName", "gn", "GNode", "longgn"]),
            10,
        );
        let order = names(&ranked, |s| s.clone());
        assert_eq!(order[0], "gn");
        assert!(order.contains(&"getName".to_owned()));
        // Fragments must start at a hump, so mid-word `gn` does not match.
        assert!(!order.contains(&"signName".to_owned()));
        assert!(!order.contains(&"longgn".to_owned()));
    }

    #[test]
    fn camel_humps_match_class_names() {
        let ranked = rank(
            "MA",
            candidates(&["MainActivity", "Mapper", "Dramatic", "MyApplication"]),
            10,
        );
        let order = names(&ranked, |s| s.clone());
        assert!(order.contains(&"MainActivity".to_owned()));
        assert!(order.contains(&"MyApplication".to_owned()));
        assert!(!order.contains(&"Dramatic".to_owned()));
        assert_eq!(ranked[0].positions.first(), Some(&0));
    }

    #[test]
    fn shorter_names_win_ties_and_empty_query_keeps_order() {
        let ranked = rank("Act", candidates(&["ActivityManager", "Activity"]), 10);
        assert_eq!(ranked[0].item, "Activity");
        let all = rank("", candidates(&["b", "a"]), 10);
        assert_eq!(names(&all, |s| s.clone()), ["b", "a"]);
        assert_eq!(rank("x", candidates(&["a", "b"]), 10).len(), 0);
        assert_eq!(rank("", candidates(&["a", "b", "c"]), 2).len(), 2);
    }

    #[test]
    fn line_suffix_is_split_off() {
        assert_eq!(split_line_suffix("Main:42"), ("Main", Some(42)));
        assert_eq!(split_line_suffix("Main:0"), ("Main", Some(1)));
        assert_eq!(split_line_suffix("Main:"), ("Main:", None));
        assert_eq!(split_line_suffix("Main"), ("Main", None));
    }

    fn entry(path: &str) -> FileEntry {
        FileEntry {
            relative_path: path.into(),
            is_dir: false,
            is_symlink: false,
            lazy: false,
        }
    }

    fn symbol(name: &str, kind: SymbolKind, path: &str, line: usize) -> SymbolEntry {
        SymbolEntry {
            name: name.into(),
            kind,
            path: path.into(),
            line,
            container: None,
        }
    }

    fn action(id: &str, label: &str) -> ActionCandidate {
        ActionCandidate {
            id: id.into(),
            label: label.into(),
            category: "Test".into(),
            shortcut: "Ctrl+S".into(),
            enabled: true,
        }
    }

    #[test]
    fn providers_split_classes_symbols_files_and_actions() {
        let files = [entry("src/MainActivity.java"), entry("README.md")];
        let recent = [PathBuf::from("README.md")];
        let indexed = [
            symbol(
                "MainActivity",
                SymbolKind::Class,
                "src/MainActivity.java",
                0,
            ),
            symbol("onCreate", SymbolKind::Method, "src/MainActivity.java", 4),
        ];
        let server = [
            symbol(
                "MainActivity",
                SymbolKind::Class,
                "src/MainActivity.java",
                0,
            ),
            symbol("mainLoop", SymbolKind::Function, "src/lib.rs", 9),
        ];
        let actions = [
            action("editor.saveAll", "Save All"),
            action("run.project", "Run"),
        ];
        let sources = SearchSources {
            files: &files,
            recent: &recent,
            symbols: indexed.iter().collect(),
            server_symbols: &server,
            actions: &actions,
            text: &[],
            include_non_project: true,
        };
        let classes = search(SearchTab::Classes, "MA", &sources);
        assert_eq!(classes.len(), 1, "server duplicate is merged: {classes:?}");
        let symbols = search(SearchTab::Symbols, "main", &sources);
        assert_eq!(
            names(&symbols, |hit| hit_name(hit).to_owned()),
            ["mainLoop"],
            "server symbols are included, classes are not"
        );
        assert_eq!(
            search(SearchTab::Files, "mainact:30", &sources)[0].item,
            Hit::File {
                path: "src/MainActivity.java".into(),
                line: Some(30)
            }
        );
        assert_eq!(
            search(SearchTab::Files, "", &sources)[0].item,
            Hit::File {
                path: "README.md".into(),
                line: None
            }
        );
        assert!(matches!(
            search(SearchTab::Actions, "sa", &sources)[0].item,
            Hit::Action(ref a) if a.id == "editor.saveAll"
        ));
        let all = search(SearchTab::All, "m", &sources);
        assert!(all.iter().any(|r| matches!(r.item, Hit::Symbol(_))));
        assert!(all.iter().any(|r| matches!(r.item, Hit::File { .. })));
    }

    #[test]
    fn symbol_index_prefers_open_buffers_and_skips_unsupported_files() {
        let root =
            std::env::temp_dir().join(format!("archipel-ide-nav-index-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("a.rs"), "struct OnDisk;\n").unwrap();
        std::fs::write(root.join("notes.txt"), "struct Nope;\n").unwrap();
        let files = [entry("a.rs"), entry("notes.txt")];
        let mut index = SymbolIndex::default();
        assert!(index.refresh(&root, &files, &[], Duration::from_secs(5)));
        let names: Vec<_> = index.all().map(|s| s.name.clone()).collect();
        assert_eq!(names, ["OnDisk"]);
        let open = [(PathBuf::from("a.rs"), "struct Edited;\n".to_owned())];
        index.refresh(&root, &files, &open, Duration::from_secs(5));
        let names: Vec<_> = index.all().map(|s| s.name.clone()).collect();
        assert_eq!(names, ["Edited"]);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn recent_locations_merge_nearby_moves_and_cap() {
        let mut nav = QuickNavState::default();
        nav.note_location(Path::new("a.rs"), 10, "x");
        nav.note_location(Path::new("a.rs"), 12, "y");
        assert_eq!(nav.locations.len(), 1);
        assert_eq!(nav.locations[0].line, 12);
        nav.note_location(Path::new("b.rs"), 1, "b");
        nav.note_location(Path::new("a.rs"), 100, "far");
        let order: Vec<_> = nav
            .locations
            .iter()
            .map(|l| (l.path.clone(), l.line))
            .collect();
        assert_eq!(
            order,
            [
                (PathBuf::from("a.rs"), 100),
                (PathBuf::from("b.rs"), 1),
                (PathBuf::from("a.rs"), 12)
            ]
        );
        for line in 0..80 {
            nav.note_location(Path::new("c.rs"), line * 10, "");
        }
        assert_eq!(nav.locations.len(), MAX_LOCATIONS);
    }

    #[test]
    fn switcher_cycles_forward_and_backward_with_wraparound() {
        let entries = vec![
            SwitcherEntry::File("a".into()),
            SwitcherEntry::File("b".into()),
            SwitcherEntry::ToolWindow(ToolWindow::Project),
        ];
        let mut forward = Switcher::open(entries.clone(), false).unwrap();
        assert_eq!(forward.selected, 1);
        forward.cycle(false);
        forward.cycle(false);
        assert_eq!(forward.selected, 0);
        let mut backward = Switcher::open(entries, true).unwrap();
        assert_eq!(backward.selected, 2);
        backward.cycle(true);
        assert_eq!(backward.selected, 1);
        assert!(Switcher::open(Vec::new(), false).is_none());
    }

    #[test]
    fn switcher_command_orders_recent_files_and_commits_the_selection() {
        let root =
            std::env::temp_dir().join(format!("archipel-ide-nav-switch-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        for name in ["a.txt", "b.txt", "c.txt"] {
            std::fs::write(root.join(name), name).unwrap();
        }
        let commands = built_in_commands();
        let mut state = IdeState::default();
        commands
            .execute("project.open", &mut state, json!({"path": root}))
            .unwrap();
        for name in ["a.txt", "b.txt", "c.txt", "a.txt"] {
            commands
                .execute("editor.openFile", &mut state, json!({"path": name}))
                .unwrap();
        }
        // Recent Files ordering: newest first, no duplicates.
        let recent: Vec<_> = state
            .recent_files
            .iter()
            .map(|p| p.display().to_string())
            .collect();
        assert_eq!(recent, ["a.txt", "c.txt", "b.txt"]);
        // Ctrl+Tab lands on the previously used file, a second press on the one before.
        commands
            .execute("navigate.switcher", &mut state, Value::Null)
            .unwrap();
        let switcher = state.switcher().unwrap();
        assert_eq!(
            switcher.selected_entry(),
            &SwitcherEntry::File("c.txt".into())
        );
        commands
            .execute("navigate.switcher", &mut state, Value::Null)
            .unwrap();
        assert_eq!(
            state.switcher().unwrap().selected_entry(),
            &SwitcherEntry::File("b.txt".into())
        );
        commands
            .execute("navigate.switcherCommit", &mut state, Value::Null)
            .unwrap();
        assert!(!state.switcher_open());
        let active = &state.documents[state.active_document.unwrap()];
        assert_eq!(active.path, Path::new("b.txt"));
        commands
            .execute("navigate.switcher", &mut state, json!({"backward": true}))
            .unwrap();
        commands
            .execute("navigate.switcherCancel", &mut state, Value::Null)
            .unwrap();
        assert!(!state.switcher_open());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn navigate_commands_request_the_matching_tab() {
        let root =
            std::env::temp_dir().join(format!("archipel-ide-nav-request-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let commands = built_in_commands();
        let mut state = IdeState::default();
        // The manager is a project service: requests need an open project.
        state.open_project(&root).unwrap();
        for (id, tab) in [
            ("navigate.gotoClass", SearchTab::Classes),
            ("navigate.gotoFile", SearchTab::Files),
            ("navigate.gotoSymbol", SearchTab::Symbols),
            ("navigate.findAction", SearchTab::Actions),
            ("navigate.searchEverywhere", SearchTab::All),
        ] {
            commands.execute(id, &mut state, Value::Null).unwrap();
            assert_eq!(
                state.take_navigation_request(),
                Some(NavigationRequest::SearchEverywhere(tab))
            );
        }
        commands
            .execute(
                "navigate.searchEverywhere",
                &mut state,
                json!({"tab": "symbols"}),
            )
            .unwrap();
        assert_eq!(
            state.navigation_request(),
            Some(NavigationRequest::SearchEverywhere(SearchTab::Symbols))
        );
        assert!(
            commands
                .execute(
                    "navigate.searchEverywhere",
                    &mut state,
                    json!({"tab": "nope"})
                )
                .is_err()
        );
        assert_eq!(SearchTab::All.step(true), SearchTab::Text);
        assert_eq!(SearchTab::Text.step(false), SearchTab::All);
    }

    #[test]
    fn outlines_come_from_the_file_structure() {
        let rust = outline(
            Path::new("a.rs"),
            "struct Greeter;\nfn greet() {}\nimpl Greeter { fn hello(&self) {} }\n",
        );
        assert!(
            rust.iter()
                .any(|(n, k, _)| n == "Greeter" && is_class_like(*k))
        );
        assert!(
            rust.iter()
                .any(|(n, k, l)| n == "hello" && *l == 2 && !is_class_like(*k))
        );
        let java = outline(
            Path::new("A.java"),
            "public class A {\n  void run() {}\n}\n",
        );
        assert!(java.iter().any(|(n, k, _)| n == "A" && is_class_like(*k)));
        assert!(java.iter().any(|(n, _, l)| n == "run" && *l == 1));
        assert!(outline(Path::new("n.txt"), "hello").is_empty());
    }

    #[test]
    fn strict_ranking_drops_letters_scattered_over_words() {
        let names = [
            "Markdown: Insert Image",
            "Main Menu",
            "Build Project",
            "Run Main",
            "getName",
        ];
        let rank = |query: &str| {
            names_of(rank_strict(
                query,
                names.iter().map(|n| (n.to_string(), n.to_string())),
                10,
            ))
        };
        let found = rank("main");
        assert!(found.contains(&"Main Menu".to_owned()), "{found:?}");
        assert!(found.contains(&"Run Main".to_owned()), "{found:?}");
        assert!(
            !found.contains(&"Markdown: Insert Image".to_owned()),
            "{found:?}"
        );
        assert_eq!(rank("bp"), ["Build Project"], "word initials still match");
        assert_eq!(rank("gN"), ["getName"], "humps inside one word match");
    }

    fn names_of(ranked: Vec<Ranked<String>>) -> Vec<String> {
        ranked.into_iter().map(|r| r.item).collect()
    }

    #[test]
    fn text_tab_lists_occurrences_and_library_symbols_are_optional() {
        let found = TextMatch {
            path: "a.txt".into(),
            line: 3,
            column: 1,
            length: 4,
            preview: "main".into(),
            highlights: std::iter::once(0..4).collect(),
        };
        let library = SymbolEntry {
            name: "main".into(),
            kind: SymbolKind::Function,
            path: "/jdk/Foo.java".into(),
            line: 0,
            container: None,
        };
        let sources = |include| SearchSources {
            files: &[],
            recent: &[],
            symbols: Vec::new(),
            server_symbols: std::slice::from_ref(&library),
            actions: &[],
            text: std::slice::from_ref(&found),
            include_non_project: include,
        };
        let text = search(SearchTab::Text, "main", &sources(false));
        assert_eq!(text[0].item, Hit::Text(found.clone()));
        assert!(search(SearchTab::Symbols, "main", &sources(false)).is_empty());
        assert_eq!(search(SearchTab::Symbols, "main", &sources(true)).len(), 1);
        let all = search(SearchTab::All, "main", &sources(true));
        assert!(matches!(all[0].item, Hit::Symbol(_)), "symbols before text");
    }
}
