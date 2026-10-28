//! Search Everywhere, Go to Class/File/Symbol, Recent Locations and the Switcher:
//! engine side. (Code navigation through language servers lives in `navigation`.)
//!
//! The popups live in `archipel-ide-ui`; everything they list, rank or cycle is
//! decided here so it can be tested without a window. Commands (`navigate.*`)
//! only record a [`NavigationRequest`] that the UI turns into a popup, or drive
//! the [`Switcher`] state directly.

use crate::goto_rank::{
    GotoOptions, decorate, rank_files, rank_names, rank_qualified, split_position,
};
use crate::name_match::{MatchMode, NameMatcher, PREFER_START_BONUS};
use crate::{IdeState, ToolWindow, spec};
use archipel_ide_command::{Command, CommandError, CommandRegistry};
use archipel_ide_language::structure::{StructureItem, SymbolKind};
use archipel_ide_lsp::lsp_types;
use archipel_ide_lsp::{WorkspaceSymbolHit, WorkspaceSymbolRequestHandle};
use archipel_ide_project::FileEntry;
use archipel_ide_search::TextMatch;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::ops::Range;
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
    /// Matching degree plus the prefer-start and exact-match bonuses (SRCH-MAT-034, 035); `0`
    /// for an empty query.
    pub weight: i32,
}

/// The highlighted character indices of a list of fragments.
fn positions_of(fragments: &[Range<usize>]) -> Vec<usize> {
    fragments.iter().flat_map(Clone::clone).collect()
}

/// Ranks `candidates` against `query` with the name matcher of SRCH-MAT (camel humps, wildcards,
/// middle matching, prefer-start): a name equal to the query first, then the higher weight, then
/// the name ignoring case. An empty query keeps the candidates' own order.
pub fn rank<T>(
    query: &str,
    candidates: impl IntoIterator<Item = (T, String)>,
    limit: usize,
) -> Vec<Ranked<T>> {
    let candidates: Vec<(T, String)> = candidates.into_iter().collect();
    if query.is_empty() {
        return candidates
            .into_iter()
            .take(limit)
            .map(|(item, _)| Ranked {
                item,
                positions: Vec::new(),
                weight: 0,
            })
            .collect();
    }
    let names: Vec<&str> = candidates.iter().map(|(_, name)| name.as_str()).collect();
    let hits = rank_names(query, &names, GotoOptions::default());
    let mut slots: Vec<Option<T>> = candidates.into_iter().map(|(item, _)| Some(item)).collect();
    hits.into_iter()
        .take(limit)
        .filter_map(|hit| {
            Some(Ranked {
                item: slots[hit.index].take()?,
                positions: positions_of(&hit.fragments),
                weight: hit.weight,
            })
        })
        .collect()
}

/// Whether `name` matches `query` with the Go to semantics (empty matches all).
pub fn matches(query: &str, name: &str) -> bool {
    query.is_empty()
        || NameMatcher::new(&decorate(query, true), MatchMode::IgnoreCase).matches(name)
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
    /// A file with the position typed after its name (`A.java:12:5`, SRCH-SE-051).
    File {
        path: PathBuf,
        line: Option<usize>,
        column: Option<usize>,
    },
    Symbol(SymbolEntry),
    Action(ActionCandidate),
    Text(TextMatch),
}

impl Hit {
    /// The typed commands that open this row (SRCH-SE-050, SRCH-SE-051): the window, the terminal
    /// frontend and the tests dispatch these in order. Actions have none (they run through the
    /// action registry).
    pub fn open_commands(&self, right_split: bool) -> Vec<(&'static str, Value)> {
        let (path, params) = match self {
            Hit::File { path, line, column } => {
                let mut params = json!({"path": path});
                if let Some(line) = line {
                    params["line"] = json!(line);
                    params["column"] = json!(column.unwrap_or(1));
                }
                (path, params)
            }
            Hit::Symbol(symbol) => (
                &symbol.path,
                json!({"path": symbol.path, "line": symbol.line + 1}),
            ),
            Hit::Text(found) => (
                &found.path,
                json!({
                    "path": found.path,
                    "line": found.line,
                    "column": found.column,
                    "length": found.length,
                }),
            ),
            Hit::Action(_) => return Vec::new(),
        };
        let mut commands = Vec::new();
        if right_split {
            commands.push(("editor.openInRightSplit", json!({"path": path})));
        }
        commands.push(("editor.openFile", params));
        commands
    }
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

/// Rows per provider on the All tab (SRCH-SE-023) and on a single-provider tab.
const ALL_SECTION: usize = 15;
const TAB_LIMIT: usize = 30;

/// The providers of Search Everywhere with their sort weight (SRCH-SE-021 rule 4): the weight
/// that breaks a tie between equal item weights.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provider {
    RecentFiles,
    Classes,
    Files,
    Symbols,
    Actions,
    Text,
}

impl Provider {
    pub fn sort_weight(self) -> u32 {
        match self {
            Provider::RecentFiles => 70,
            Provider::Classes => 100,
            Provider::Files => 200,
            Provider::Symbols => 300,
            Provider::Actions => 400,
            Provider::Text => 1500,
        }
    }

    /// The group name of the type filter and the list texts.
    pub fn name(self) -> &'static str {
        match self {
            Provider::RecentFiles => "Recent Files",
            Provider::Classes => "Classes",
            Provider::Files => "Files",
            Provider::Symbols => "Symbols",
            Provider::Actions => "Actions",
            Provider::Text => "Text",
        }
    }
}

/// Runs the provider(s) of `tab`. The All tab merges the providers by weight (SRCH-SE-021).
pub fn search(tab: SearchTab, query: &str, sources: &SearchSources) -> Vec<Ranked<Hit>> {
    let query = query.trim_start();
    if tab == SearchTab::All {
        return search_all(query, sources);
    }
    match tab {
        SearchTab::Files => files(query, sources, TAB_LIMIT),
        SearchTab::Classes => symbols(query, sources, TAB_LIMIT, true),
        SearchTab::Symbols => symbols(query, sources, TAB_LIMIT, false),
        SearchTab::Actions => rank_actions(query, sources.actions, TAB_LIMIT),
        SearchTab::Text => text_hits(sources, TAB_LIMIT),
        SearchTab::All => Vec::new(),
    }
}

fn text_hits(sources: &SearchSources, limit: usize) -> Vec<Ranked<Hit>> {
    sources
        .text
        .iter()
        .take(limit)
        .map(|found| Ranked {
            item: Hit::Text(found.clone()),
            positions: Vec::new(),
            weight: i32::MIN / 2,
        })
        .collect()
}

/// The All tab: every provider's own order, merged by weight and then the provider's sort weight
/// (SRCH-SE-021). A file that is both a recent file and a file stays once, as the file
/// (SRCH-SE-022). With nothing typed only the recent files are listed (SRCH-SE-033).
fn search_all(query: &str, sources: &SearchSources) -> Vec<Ranked<Hit>> {
    if query.trim().is_empty() {
        return recent_hits(query, sources, ALL_SECTION);
    }
    let file_hits = files(query, sources, ALL_SECTION);
    let known: std::collections::HashSet<&Path> = file_hits
        .iter()
        .filter_map(|hit| match &hit.item {
            Hit::File { path, .. } => Some(path.as_path()),
            _ => None,
        })
        .collect();
    let recent: Vec<Ranked<Hit>> = recent_hits(query, sources, ALL_SECTION)
        .into_iter()
        .filter(|hit| match &hit.item {
            Hit::File { path, .. } => !known.contains(path.as_path()),
            _ => true,
        })
        .collect();
    let lists: Vec<(Provider, Vec<Ranked<Hit>>)> = vec![
        (Provider::RecentFiles, recent),
        (
            Provider::Classes,
            symbols(query, sources, ALL_SECTION, true),
        ),
        (Provider::Files, file_hits),
        (
            Provider::Symbols,
            symbols(query, sources, ALL_SECTION, false),
        ),
        (
            Provider::Actions,
            rank_actions(query, sources.actions, ALL_SECTION),
        ),
        (Provider::Text, text_hits(sources, ALL_SECTION)),
    ];
    merge_providers(lists)
}

/// Merges the ordered lists of several providers: the list whose next item has the higher
/// weight goes first, equal weights go to the lower provider sort weight.
pub fn merge_providers<T>(lists: Vec<(Provider, Vec<Ranked<T>>)>) -> Vec<Ranked<T>> {
    merge_providers_with(lists, false)
}

/// Like [`merge_providers`]; with `recent_first` (`search.everywhere.recent.at.top`) the Recent
/// Files provider is pinned above the others (SRCH-SE-021 rule 1).
pub fn merge_providers_with<T>(
    lists: Vec<(Provider, Vec<Ranked<T>>)>,
    recent_first: bool,
) -> Vec<Ranked<T>> {
    let mut queues: Vec<(Provider, std::collections::VecDeque<Ranked<T>>)> = lists
        .into_iter()
        .map(|(provider, items)| (provider, items.into()))
        .collect();
    let mut out = Vec::new();
    loop {
        let best = queues
            .iter()
            .enumerate()
            .filter_map(|(i, (provider, items))| {
                items
                    .front()
                    .map(|item| (i, item.weight, provider.sort_weight()))
            })
            .max_by(|a, b| {
                let pinned = |i: usize| recent_first && queues[i].0 == Provider::RecentFiles;
                pinned(a.0)
                    .cmp(&pinned(b.0))
                    .then(a.1.cmp(&b.1))
                    .then(b.2.cmp(&a.2))
                    .then(b.0.cmp(&a.0))
            });
        let Some((index, ..)) = best else {
            return out;
        };
        if let Some(item) = queues[index].1.pop_front() {
            out.push(item);
        }
    }
}

/// Recent files filtered by the typed text, matched as `*text` with prefer-start; the weight is
/// the name degree, newer first on equal weights (SRCH-SE-033).
fn recent_hits(query: &str, sources: &SearchSources, limit: usize) -> Vec<Ranked<Hit>> {
    let position = split_position(query);
    let pattern = decorate(&position.name, true);
    let matcher = NameMatcher::new(&pattern, MatchMode::IgnoreCase);
    let mut hits: Vec<Ranked<Hit>> = sources
        .recent
        .iter()
        .filter_map(|path| {
            let name = file_name(path);
            let (fragments, weight) = if position.name.trim().is_empty() {
                (Vec::new(), 0)
            } else {
                let (fragments, degree) = matcher.match_with_degree(&name)?;
                let bonus = if fragments.first().is_some_and(|f| f.start == 0)
                    && !position.name.starts_with('*')
                {
                    PREFER_START_BONUS
                } else {
                    0
                };
                (fragments, degree + bonus)
            };
            Some(Ranked {
                item: Hit::File {
                    path: path.clone(),
                    line: position.line,
                    column: position.column.filter(|_| position.line.is_some()),
                },
                positions: positions_of(&fragments),
                weight,
            })
        })
        .collect();
    hits.sort_by_key(|hit| std::cmp::Reverse(hit.weight));
    hits.truncate(limit);
    hits
}

fn files(query: &str, sources: &SearchSources, limit: usize) -> Vec<Ranked<Hit>> {
    let position = split_position(query);
    if position.name.trim().is_empty() {
        return recent_hits(query, sources, limit);
    }
    let entries: Vec<&FileEntry> = sources.files.iter().filter(|entry| !entry.is_dir).collect();
    let paths: Vec<String> = entries
        .iter()
        .map(|entry| entry.relative_path.to_string_lossy().replace('\\', "/"))
        .collect();
    let refs: Vec<&str> = paths.iter().map(String::as_str).collect();
    rank_files(query, &refs, None, GotoOptions::default())
        .into_iter()
        .take(limit)
        .map(|hit| Ranked {
            item: Hit::File {
                path: entries[hit.index].relative_path.clone(),
                line: position.line,
                column: position.column.filter(|_| position.line.is_some()),
            },
            positions: positions_of(&hit.fragments),
            weight: hit.weight,
        })
        .collect()
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
    let candidates: Vec<&SymbolEntry> = sources
        .symbols
        .iter()
        .copied()
        .chain(sources.server_symbols.iter())
        .filter(|symbol| keep(symbol.kind))
        .filter(|symbol| sources.include_non_project || !symbol.path.is_absolute())
        .filter(|symbol| seen.insert((symbol.path.clone(), symbol.name.clone(), symbol.line)))
        .collect();
    if query.trim().is_empty() {
        let mut all: Vec<&SymbolEntry> = candidates;
        all.sort_by(|a, b| a.name.cmp(&b.name));
        return all
            .into_iter()
            .take(limit)
            .map(|symbol| Ranked {
                item: Hit::Symbol(symbol.clone()),
                positions: Vec::new(),
                weight: 0,
            })
            .collect();
    }
    let qualified: Vec<String> = candidates
        .iter()
        .map(|symbol| match &symbol.container {
            Some(container) if !container.is_empty() => format!("{container}.{}", symbol.name),
            _ => symbol.name.clone(),
        })
        .collect();
    let refs: Vec<&str> = qualified.iter().map(String::as_str).collect();
    rank_qualified(query, &refs, &['.'], GotoOptions::default())
        .into_iter()
        .take(limit)
        .map(|hit| Ranked {
            item: Hit::Symbol(candidates[hit.index].clone()),
            positions: positions_of(&hit.fragments),
            weight: hit.weight,
        })
        .collect()
}

/// The weight of a file typed as a full path (SRCH-SE-031).
pub const EXACT_PATH_WEIGHT: i32 = 15_000;

/// A typed absolute path (or one relative to the project directory) that names an existing file
/// gives that file with the exact weight, first in the list (SRCH-SE-031, SRCH-SE-016). Paths
/// outside the project need `outside_allowed` (`search.everywhere.absolute.path.outside.project`).
pub fn absolute_path_hit(
    typed: &str,
    root: &Path,
    outside_allowed: bool,
    exists: impl Fn(&Path) -> bool,
) -> Option<Ranked<Hit>> {
    let typed = split_position(typed);
    let path = Path::new(typed.name.trim());
    let (candidate, display): (PathBuf, PathBuf) = if path.is_absolute() {
        match path.strip_prefix(root) {
            Ok(relative) => (path.to_owned(), relative.to_owned()),
            Err(_) if outside_allowed => (path.to_owned(), path.to_owned()),
            Err(_) => return None,
        }
    } else if path.components().count() > 1 && exists(&root.join(path)) {
        (root.join(path), path.to_owned())
    } else {
        return None;
    };
    if !exists(&candidate) {
        return None;
    }
    let name_len = file_name(&display).chars().count();
    Some(Ranked {
        item: Hit::File {
            path: display,
            line: typed.line,
            column: typed.column.filter(|_| typed.line.is_some()),
        },
        positions: (0..name_len).collect(),
        weight: EXACT_PATH_WEIGHT,
    })
}

/// Actions whose abbreviation equals the typed text (ignoring case) come first, with the weight
/// of the abbreviation match (SRCH-SE-040, SRCH-SE-043). `abbreviations` are
/// `(abbreviation, action id)` pairs.
pub fn abbreviation_hits(
    typed: &str,
    abbreviations: &[(String, String)],
    actions: &[ActionCandidate],
) -> Vec<Ranked<Hit>> {
    let typed = typed.trim();
    abbreviations
        .iter()
        .filter(|(abbreviation, _)| abbreviation.eq_ignore_ascii_case(typed))
        .filter_map(|(abbreviation, id)| {
            let action = actions.iter().find(|a| &a.id == id)?;
            Some(Ranked {
                item: Hit::Action(action.clone()),
                positions: Vec::new(),
                weight: action_weight(typed, abbreviation).unwrap_or(0),
            })
        })
        .collect()
}

// ------------------------------------------------------------------- actions

/// How an action matched (SRCH-SE-042 rule 2: the order of the modes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum ActionMatchMode {
    Name,
    Group,
}

/// Splits at whitespace, `-` and `/` for the word-prefix match of a pattern with a space.
fn words(text: &str) -> Vec<String> {
    text.split(|c: char| c.is_whitespace() || c == '-' || c == '/')
        .filter(|word| !word.is_empty())
        .map(str::to_lowercase)
        .collect()
}

/// Every pattern word is a case-insensitive prefix of some name word (SRCH-SE `NAME` mode).
fn word_prefix_match(pattern: &str, name: &str) -> bool {
    let name_words = words(name);
    let pattern_words = words(pattern);
    !pattern_words.is_empty()
        && pattern_words.iter().all(|word| {
            name_words
                .iter()
                .any(|candidate| candidate.starts_with(word))
        })
}

/// `text` without leading and trailing non-alphanumeric characters (`Find…` is `Find`).
fn trim_non_alphanumeric(text: &str) -> &str {
    text.trim_matches(|c: char| !c.is_alphanumeric())
}

/// The weight of an action text for a typed pattern (SRCH-SE-041).
pub fn action_weight(pattern: &str, text: &str) -> Option<i32> {
    let trimmed = pattern.trim();
    let matcher = NameMatcher::new(&decorate(trimmed, true), MatchMode::IgnoreCase);
    let weigh = |name: &str| -> Option<i32> {
        let (fragments, degree) = matcher.match_with_degree(name)?;
        let bonus = if fragments.first().is_some_and(|f| f.start == 0) && !trimmed.starts_with('*')
        {
            PREFER_START_BONUS
        } else {
            0
        };
        Some(degree + bonus)
    };
    let stripped = trim_non_alphanumeric(text);
    let degree = weigh(stripped).filter(|d| *d != 0).or_else(|| weigh(text));
    let space = if trimmed.contains(' ') { 100 } else { 0 };
    match degree {
        Some(d) if d != 0 => Some(d + space),
        _ => {
            // No camel-hump weight: the fallback rank.
            let plain = text.trim_end_matches(['.', '…']).to_lowercase();
            let wanted = trimmed.to_lowercase();
            let rank = if plain == wanted {
                3
            } else if plain.starts_with(&wanted) {
                2
            } else if plain.contains(&wanted) {
                1
            } else {
                0
            };
            Some(rank + 1 + space)
        }
    }
}

/// The Actions provider (SRCH-SE-040 to 042): filter by name, group or word prefix, weigh and
/// order equal weights.
fn rank_actions(query: &str, actions: &[ActionCandidate], limit: usize) -> Vec<Ranked<Hit>> {
    let query = query.trim_start();
    if query.trim().is_empty() {
        return actions
            .iter()
            .take(limit)
            .map(|action| Ranked {
                item: Hit::Action(action.clone()),
                positions: Vec::new(),
                weight: 0,
            })
            .collect();
    }
    let trimmed = query.trim();
    let has_space = trimmed.contains(' ');
    let name_matcher = NameMatcher::new(&decorate(trimmed, true), MatchMode::IgnoreCase);
    struct Row<'a> {
        action: &'a ActionCandidate,
        weight: i32,
        mode: ActionMatchMode,
        positions: Vec<usize>,
    }
    let mut rows: Vec<Row> = Vec::new();
    for action in actions {
        let mode = if !has_space && name_matcher.matches(&action.label)
            || has_space && word_prefix_match(trimmed, &action.label)
        {
            Some(ActionMatchMode::Name)
        } else if !action.category.is_empty()
            && (word_prefix_match(trimmed, &format!("{} {}", action.category, action.label))
                || word_prefix_match(trimmed, &format!("{} {}", action.label, action.category)))
            && has_space
        {
            Some(ActionMatchMode::Group)
        } else {
            None
        };
        let Some(mode) = mode else { continue };
        let Some(weight) = action_weight(trimmed, &action.label) else {
            continue;
        };
        let positions = name_matcher
            .match_name(&action.label)
            .map(|f| positions_of(&f))
            .unwrap_or_default();
        rows.push(Row {
            action,
            weight,
            mode,
            positions,
        });
    }
    // Equal weights: available before unavailable, match mode, text, shorter, group.
    let strip = |text: &str| text.trim_end_matches(['.', '…']).to_lowercase();
    rows.sort_by(|a, b| {
        b.weight
            .cmp(&a.weight)
            .then(b.action.enabled.cmp(&a.action.enabled))
            .then(a.mode.cmp(&b.mode))
            .then_with(|| strip(&a.action.label).cmp(&strip(&b.action.label)))
            .then(
                a.action
                    .label
                    .chars()
                    .count()
                    .cmp(&b.action.label.chars().count()),
            )
            .then_with(|| a.action.category.cmp(&b.action.category))
    });
    rows.into_iter()
        .take(limit)
        .map(|row| Ranked {
            item: Hit::Action(row.action.clone()),
            positions: row.positions,
            weight: row.weight,
        })
        .collect()
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

/// Entries of Recent Locations (`recentLocationsLimit`, SRCH-REC-010).
const MAX_LOCATIONS: usize = 25;
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
        self.search_everywhere().write().request.take()
    }

    /// The pending popup request, without taking it.
    pub fn navigation_request(&self) -> Option<NavigationRequest> {
        self.search_everywhere().read().request
    }

    /// Recent Locations, newest first.
    pub fn recent_locations(&self) -> Vec<RecentLocation> {
        self.search_everywhere().read().locations.clone()
    }

    /// A copy of the open Switcher.
    pub fn switcher(&self) -> Option<Switcher> {
        self.search_everywhere().read().switcher.clone()
    }

    /// Selects `index` in the open Switcher.
    pub fn select_switcher_entry(&self, index: usize) {
        if let Some(switcher) = self.search_everywhere().write().switcher.as_mut() {
            switcher.selected = index;
        }
    }

    /// Whether the Text tab's background search is running.
    pub fn text_search_running(&self) -> bool {
        self.search_everywhere().read().text.lock().is_running()
    }

    /// Opens the Switcher, or cycles it when already open.
    pub fn switcher_advance(&mut self, backward: bool) {
        let nav = self.search_everywhere();
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
        {
            let nav = self.search_everywhere();
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
        {
            let nav = self.search_everywhere();
            let mut nav = nav.write();
            nav.index_complete = nav.index.refresh(&root, &self.files, &open, budget);
        }
    }

    /// Starts (or restarts) a `workspace/symbol` request for `query` through
    /// a running language server; a no-op without one.
    pub fn request_server_symbols(&mut self, query: &str) {
        let nav = self.search_everywhere();
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
        let nav = self.search_everywhere();
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
                    {
                        let nav = state.search_everywhere();
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
                {
                    let nav = state.search_everywhere();
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
                {
                    let nav = state.search_everywhere();
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
                    .read()
                    .switcher
                    .as_ref()
                    .map(|s| s.selected);
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
                let Some(switcher) = state.search_everywhere().write().switcher.take() else {
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
                {
                    let nav = state.search_everywhere();
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
        // Middle matching (SRCH-MAT-040): a mid-word `gn` still matches but ranks after humps.
        let at = |name: &str| order.iter().position(|n| n == name).unwrap();
        assert!(at("getName") < at("signName"), "{order:?}");
        assert!(at("GNode") < at("longgn"), "{order:?}");
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
        // The middle match `Dra[ma]tic` ranks last (SRCH-MAT-040).
        assert_eq!(order.last().map(String::as_str), Some("Dramatic"));
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

    fn hit_name(hit: &Hit) -> &str {
        match hit {
            Hit::Symbol(symbol) => &symbol.name,
            _ => "",
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
                line: Some(30),
                column: Some(1)
            }
        );
        assert_eq!(
            search(SearchTab::Files, "", &sources)[0].item,
            Hit::File {
                path: "README.md".into(),
                line: None,
                column: None
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
        let commands = built_in_commands();
        let mut state = IdeState::default();
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
