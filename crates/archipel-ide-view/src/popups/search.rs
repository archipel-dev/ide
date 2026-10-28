//! Search Everywhere and its siblings (TUI-GUI-135, TUI-GUI-136, UI-POP-010..012, M13, M15):
//! the tabs, the header checkbox, the hint, the rows with their group separators and the
//! footer. Go to File, Go to Class, Go to Symbol, Text and Find Action are this popup with a
//! tab preselected. The window and the terminal draw these values; the matching and the
//! ranking are `archipel_ide_core::quick_nav`.

use crate::ViewCx;
use archipel_ide_actions::{ActionPlace, Presentation};
use archipel_ide_command::CommandSpec;
use archipel_ide_core::IdeState;
use archipel_ide_core::keymap_commands::actions_for_command;
use archipel_ide_core::quick_nav::{
    ActionCandidate, Hit, Ranked, SearchSources, SearchTab, file_name, search,
};
use archipel_ide_language::structure::SymbolKind;
use archipel_ide_platform::data_keys;
use archipel_kit_model::{Glyph, IconKey};
use serde_json::Value;
use std::time::Duration;

/// The hint at the right of the empty field (UI-POP-011).
pub use archipel_ide_core::search_popup::COMMAND_HINT as HINT;

/// The placeholder of the empty field.
pub const PLACEHOLDER: &str = "Type to search classes, files, symbols, actions and text";

/// Rows listed on the All tab and on a single tab.
const ALL_ROWS: usize = 24;
const TAB_ROWS: usize = 40;

/// Time spent indexing symbols per call while the popup is open.
pub const INDEX_BUDGET: Duration = Duration::from_millis(15);

/// The footer when nothing is selected.
pub const IDLE_FOOTER: &str =
    "↑↓ to navigate · Enter to open · Shift+Enter to open in right split · Esc to close";

/// One row of the list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchRow {
    /// The icon.
    pub icon: IconKey,
    /// The name: a file name, a symbol, the action, the matching line.
    pub title: String,
    /// Character positions of `title` to highlight.
    pub positions: Vec<usize>,
    /// Secondary text after the title: the folder, the container, the shortcut of an action.
    pub inline: String,
    /// Secondary text at the right edge: the location, the group of an action.
    pub trailing: String,
    /// Whether the row can be chosen (a disabled action cannot).
    pub enabled: bool,
    /// The title of the group this row starts (All tab with a query), a separator row above.
    pub group: Option<&'static str>,
    /// The footer while the row is selected: its path or its action id.
    pub footer: String,
    /// What the row stands for.
    pub hit: Hit,
}

impl SearchRow {
    /// The action id when the row is an action.
    pub fn action(&self) -> Option<&ActionCandidate> {
        match &self.hit {
            Hit::Action(action) => Some(action),
            _ => None,
        }
    }
}

/// The group a hit belongs to on the All tab.
fn group_of(hit: &Hit, recent: bool) -> &'static str {
    match hit {
        Hit::File { .. } if recent => "Recent Files",
        Hit::File { .. } => "Files",
        Hit::Symbol(symbol) => match symbol.kind {
            SymbolKind::Class | SymbolKind::Interface | SymbolKind::Enum => "Classes",
            _ => "Symbols",
        },
        Hit::Action(_) => "Actions",
        Hit::Text(_) => "Text",
    }
}

/// The symbol kind name of the icon of a symbol.
fn symbol_icon(kind: SymbolKind) -> IconKey {
    let name = match kind {
        SymbolKind::Class => "class",
        SymbolKind::Interface => "interface",
        SymbolKind::Enum => "enum",
        SymbolKind::Method => "method",
        SymbolKind::Function => "function",
        SymbolKind::Constructor => "constructor",
        SymbolKind::Field => "field",
        SymbolKind::Constant => "constant",
        SymbolKind::Module => "module",
        _ => return IconKey::Glyph(Glyph::File),
    };
    IconKey::Symbol(name.to_owned())
}

fn parent_text(path: &std::path::Path) -> String {
    path.parent()
        .map(|parent| parent.display().to_string())
        .unwrap_or_default()
}

/// The row of one ranked hit.
pub fn row(ranked: &Ranked<Hit>) -> SearchRow {
    let mut positions = ranked.positions.clone();
    let (icon, title, inline, trailing, enabled, footer) = match &ranked.item {
        Hit::Action(action) => (
            IconKey::Glyph(Glyph::Run),
            action.label.clone(),
            action.shortcut.clone(),
            action.category.clone(),
            action.enabled,
            action.id.clone(),
        ),
        Hit::File { path, line, .. } => (
            IconKey::File(file_name(path)),
            file_name(path),
            parent_text(path),
            line.map(|line| format!(":{line}")).unwrap_or_default(),
            true,
            path.display().to_string(),
        ),
        Hit::Symbol(symbol) => (
            symbol_icon(symbol.kind),
            symbol.name.clone(),
            symbol.container.clone().unwrap_or_default(),
            format!("{}:{}", file_name(&symbol.path), symbol.line + 1),
            true,
            symbol.path.display().to_string(),
        ),
        Hit::Text(found) => {
            positions = found
                .highlights
                .iter()
                .flat_map(|range| range.clone())
                .collect();
            (
                IconKey::File(file_name(&found.path)),
                found.preview.clone(),
                String::new(),
                format!("{} {}", file_name(&found.path), found.line),
                true,
                found.path.display().to_string(),
            )
        }
    };
    SearchRow {
        icon,
        title,
        positions,
        inline,
        trailing,
        enabled,
        group: None,
        footer,
        hit: ranked.item.clone(),
    }
}

/// The rows of `hits` for `tab`; on the All tab the first row of each group carries its title
/// (a separator row in the list, TUI-GUI-135). `typed` is whether the field holds text: with
/// nothing typed the All tab lists the recent files.
pub fn rows(tab: SearchTab, typed: bool, hits: &[Ranked<Hit>]) -> Vec<SearchRow> {
    let mut rows: Vec<SearchRow> = hits.iter().map(row).collect();
    if tab == SearchTab::All {
        let mut last = None;
        for row in &mut rows {
            let group = group_of(&row.hit, !typed);
            if last != Some(group) {
                row.group = Some(group);
                last = Some(group);
            }
        }
    }
    rows
}

/// The checkbox at the right of the header band.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeaderCheckbox {
    /// Its label.
    pub label: &'static str,
    /// Whether it is checked.
    pub checked: bool,
}

/// The header checkbox of `tab`: "Include disabled actions" on Actions, none on Files and
/// Text, "Include non-project items" elsewhere (UI-POP-011).
pub fn checkbox(
    tab: SearchTab,
    include_non_project: bool,
    include_disabled: bool,
) -> Option<HeaderCheckbox> {
    match tab {
        SearchTab::Actions => Some(HeaderCheckbox {
            label: "Include disabled actions",
            checked: include_disabled,
        }),
        SearchTab::Files | SearchTab::Text => None,
        _ => Some(HeaderCheckbox {
            label: "Include non-project items",
            checked: include_non_project,
        }),
    }
}

/// The footer: the location of the selected row at the left, the alternative way to open it
/// at the right (UI-POP-011).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Footer {
    /// The location (path, action id) or the key hints.
    pub left: String,
    /// The link: "Open In Right Split  Shift+Enter", "Assign Shortcut  Alt+Enter".
    pub link: &'static str,
}

/// The link at the right of the footer for `tab`.
pub fn link(tab: SearchTab, has_selection: bool) -> &'static str {
    if tab == SearchTab::Actions {
        "Assign Shortcut  Alt+Enter"
    } else if has_selection {
        "Open In Right Split  Shift+Enter"
    } else {
        ""
    }
}

/// The footer for `selected` on `tab`.
pub fn footer(tab: SearchTab, selected: Option<&SearchRow>) -> Footer {
    Footer {
        left: selected.map_or_else(|| IDLE_FOOTER.to_owned(), |row| row.footer.clone()),
        link: link(tab, selected.is_some()),
    }
}

/// The tabs of the header band, in order.
pub fn tabs() -> [SearchTab; 6] {
    SearchTab::ALL
}

/// What the list says when it is empty and nothing is running.
pub fn empty_text(searching: bool) -> &'static str {
    if searching {
        "Searching…"
    } else {
        "Nothing found"
    }
}

/// Commands that can run without parameters (all declared fields optional).
pub fn parameterless(spec: &CommandSpec) -> bool {
    match serde_json::from_str::<Value>(&spec.params) {
        Ok(Value::Object(fields)) => fields
            .values()
            .all(|value| value.as_str().is_some_and(|kind| kind.ends_with('?'))),
        _ => false,
    }
}

/// An action of a frontend that no command stands for (Go to Line, New File, Exit).
#[derive(Debug, Clone, Copy)]
pub struct ActionExtra {
    /// The action id of the registry.
    pub id: &'static str,
    /// The text.
    pub label: &'static str,
    /// The group shown at the right.
    pub category: &'static str,
}

/// The candidates of Find Action: every parameterless command and the `extras` the frontend
/// adds, each with its presentation in `ActionPlace::FindAction`. Disabled actions are only
/// listed with `include_disabled`. `extra_prefix` goes before the id of an extra,
/// `own_commands` maps commands to the frontend's own actions for their shortcut, and
/// `command_default` supplies a shortcut for a command that no keymap action maps to.
pub fn action_candidates(
    cx: &ViewCx,
    extras: &[ActionExtra],
    extra_prefix: &str,
    own_commands: &[(&str, &'static str)],
    include_disabled: bool,
    command_default: &dyn Fn(&CommandSpec) -> String,
) -> Vec<ActionCandidate> {
    let notation = cx.caps.notation();
    let shortcut = |action: &str| cx.shortcut(action).map(|s| s.label(notation));
    cx.with_data(
        |data| data,
        |registry, data| {
            let present = |id: &str| {
                registry
                    .update(id, ActionPlace::FindAction, data)
                    .unwrap_or_else(Presentation::disabled)
            };
            let own = extras
                .iter()
                .map(|extra| (present(extra.id), extra))
                .filter(|(presentation, _)| presentation.visible)
                .map(|(presentation, extra)| ActionCandidate {
                    enabled: presentation.enabled,
                    shortcut: shortcut(extra.id).unwrap_or_default(),
                    id: format!("{extra_prefix}{}", extra.id),
                    label: presentation.text.unwrap_or_else(|| extra.label.to_owned()),
                    category: extra.category.to_owned(),
                })
                .filter(|candidate| include_disabled || candidate.enabled);
            cx.commands
                .specs()
                .into_iter()
                .filter(parameterless)
                .map(|spec| (present(&spec.id), spec))
                .filter(|(presentation, _)| presentation.visible)
                .filter(|(presentation, _)| include_disabled || presentation.enabled)
                // Android actions only apply to Android projects; the SDK manager is global.
                .filter(|(presentation, spec)| {
                    spec.category != "Android"
                        || data.flag(&data_keys::ANDROID_PROJECT)
                        || presentation.enabled
                })
                .map(|(presentation, spec)| {
                    let own = own_commands
                        .iter()
                        .find(|(command, _)| *command == spec.id)
                        .map(|(_, action)| *action);
                    let shortcut = actions_for_command(&spec.id)
                        .chain(own)
                        .find_map(shortcut)
                        .unwrap_or_else(|| command_default(&spec));
                    ActionCandidate {
                        enabled: presentation.enabled,
                        shortcut,
                        id: spec.id,
                        label: presentation.text.unwrap_or(spec.label),
                        category: spec.category,
                    }
                })
                .chain(own)
                .collect()
        },
    )
}

/// Starts the background work a query needs (the symbol index, the language server's
/// symbols, the text search) and collects what has arrived. Returns whether a search is
/// still running, so the frontend redraws without input. `server_query` is the query last
/// sent to the language server.
pub fn prepare(
    state: &mut IdeState,
    tab: SearchTab,
    query: &str,
    server_query: &mut String,
) -> bool {
    if matches!(
        tab,
        SearchTab::All | SearchTab::Classes | SearchTab::Symbols
    ) {
        state.refresh_symbol_index(INDEX_BUDGET);
        if server_query != query {
            *server_query = query.to_owned();
            state.request_server_symbols(query);
        }
        state.poll_server_symbols();
    }
    let mut busy = false;
    if matches!(tab, SearchTab::All | SearchTab::Text) {
        state.request_text_search(query);
        state.poll_text_search();
        busy = state.text_search_running();
    }
    busy
}

/// The ranked hits of `tab` for `query`; `actions` are the candidates (only needed on the
/// All and Actions tabs).
pub fn hits(
    cx: &ViewCx,
    tab: SearchTab,
    query: &str,
    include_non_project: bool,
    actions: &[ActionCandidate],
) -> Vec<Ranked<Hit>> {
    let state = cx.state;
    let manager = state.search_everywhere();
    let navigation = manager.read();
    let sources = SearchSources {
        files: state.files(),
        recent: state.recent_files(),
        symbols: navigation.index.all().collect(),
        server_symbols: &navigation.server.results,
        actions,
        text: &navigation.text_results,
        include_non_project,
    };
    let mut found = search(tab, query, &sources);
    if tab == SearchTab::All && query.is_empty() {
        // Nothing typed: recent files only, like the reference IDE.
        found.retain(|hit| matches!(hit.item, Hit::File { .. }));
    }
    found.truncate(if tab == SearchTab::All {
        ALL_ROWS
    } else {
        TAB_ROWS
    });
    found
}

/// Whether `tab` lists actions (the candidates are needed).
pub fn needs_actions(tab: SearchTab) -> bool {
    matches!(tab, SearchTab::All | SearchTab::Actions)
}

#[cfg(test)]
mod tests {
    use super::*;
    use archipel_ide_core::quick_nav::SymbolEntry;
    use std::path::PathBuf;

    fn ranked(item: Hit) -> Ranked<Hit> {
        Ranked {
            item,
            positions: vec![0],
            weight: 0,
        }
    }

    fn class(name: &str) -> Ranked<Hit> {
        ranked(Hit::Symbol(SymbolEntry {
            name: name.into(),
            kind: SymbolKind::Class,
            path: PathBuf::from("src/util/List.java"),
            line: 9,
            container: Some("java.util".into()),
        }))
    }

    fn file(path: &str) -> Ranked<Hit> {
        ranked(Hit::File {
            path: PathBuf::from(path),
            line: None,
            column: None,
        })
    }

    #[test]
    fn rows_carry_name_container_location_and_footer() {
        let row = row(&class("ArrayList"));
        assert_eq!(row.title, "ArrayList");
        assert_eq!(row.inline, "java.util");
        assert_eq!(row.trailing, "List.java:10");
        assert_eq!(row.footer, "src/util/List.java");
        assert_eq!(row.icon, IconKey::Symbol("class".into()));
        assert_eq!(row.positions, vec![0]);
    }

    /// TUI-GUI-135: group separator rows on the All tab only, one per run of a group.
    #[test]
    fn the_all_tab_has_group_separators() {
        let hits = vec![class("A"), class("B"), file("src/a.rs"), file("src/b.rs")];
        let all = rows(SearchTab::All, true, &hits);
        let groups: Vec<_> = all.iter().map(|row| row.group).collect();
        assert_eq!(groups, [Some("Classes"), None, Some("Files"), None]);
        let single = rows(SearchTab::Classes, true, &hits);
        assert!(single.iter().all(|row| row.group.is_none()));
        let recent = rows(SearchTab::All, false, &[file("src/a.rs")]);
        assert_eq!(recent[0].group, Some("Recent Files"));
    }

    #[test]
    fn parameterless_commands_are_detected_from_their_schema() {
        let specs = archipel_ide_core::built_in_commands().specs();
        let find = |id: &str| specs.iter().find(|spec| spec.id == id).unwrap();
        assert!(parameterless(find("editor.save")));
        assert!(parameterless(find("editor.close")));
        assert!(!parameterless(find("editor.openFile")));
        assert!(!parameterless(find("toolwindow.toggle")));
    }

    #[test]
    fn the_header_checkbox_depends_on_the_tab() {
        assert_eq!(
            checkbox(SearchTab::All, true, false).map(|c| (c.label, c.checked)),
            Some(("Include non-project items", true))
        );
        assert_eq!(
            checkbox(SearchTab::Actions, true, false).map(|c| (c.label, c.checked)),
            Some(("Include disabled actions", false))
        );
        assert!(checkbox(SearchTab::Files, false, false).is_none());
        assert!(checkbox(SearchTab::Text, false, false).is_none());
    }

    #[test]
    fn the_footer_names_the_location_and_the_alternative_open() {
        let row = row(&file("src/a.rs"));
        let footer = footer(SearchTab::All, Some(&row));
        assert_eq!(footer.left, "src/a.rs");
        assert_eq!(footer.link, "Open In Right Split  Shift+Enter");
        assert_eq!(footer_of_none().link, "");
        assert_eq!(
            super::footer(SearchTab::Actions, None).link,
            "Assign Shortcut  Alt+Enter"
        );
    }

    fn footer_of_none() -> Footer {
        footer(SearchTab::All, None)
    }
}
