//! The rows of the pickers: Search Everywhere, Go to File, Find Action, Recent Files and the
//! run configurations. Ranking and sources are the core's (`quick_nav`); this only turns the
//! hits into rows.

use crate::app::TuiApp;
use crate::layout::file_icon;
use crate::picker::{PickerKind, PickerRow, Target};
use archipel_ide_actions::{ActionPlace, Presentation};
use archipel_ide_command::CommandSpec;
use archipel_ide_core::TextMatch;
use archipel_ide_core::keymap_commands::actions_for_command;
use archipel_ide_core::quick_nav::{
    ActionCandidate, Hit, Ranked, SearchSources, SearchTab, SymbolEntry, file_name, matches, search,
};
use archipel_ide_language::structure::SymbolKind;
use archipel_ide_platform::data_keys;
use archipel_kit_tui::Glyph;
use archipel_kit_tui::components::RowIcon;
use serde_json::Value;
use std::time::Duration;

const INDEX_BUDGET: Duration = Duration::from_millis(15);
const MAX_ROWS: usize = 60;

/// Commands that can run without parameters (all declared fields optional).
fn parameterless(spec: &CommandSpec) -> bool {
    match serde_json::from_str::<Value>(&spec.params) {
        Ok(Value::Object(fields)) => fields
            .values()
            .all(|value| value.as_str().is_some_and(|kind| kind.ends_with('?'))),
        _ => true,
    }
}

/// The actions of this frontend that no command stands for, with their menu texts.
const TUI_ACTIONS: [(&str, &str, &str); 6] = [
    ("GotoLine", "Go to Line:Column…", "Navigate"),
    ("FindInPath", "Find in Files…", "Search"),
    ("NewFile", "New File…", "File"),
    ("OpenFile", "Open Folder…", "File"),
    ("Exit", "Exit", "File"),
    ("Archipel.NextTheme", "Switch Theme", "View"),
];

/// Commands whose reference action this frontend implements itself.
const OWN_COMMANDS: [(&str, &str); 3] = [
    ("editor.saveAll", "SaveAll"),
    ("editor.gotoLine", "GotoLine"),
    ("search.project", "FindInPath"),
];

fn symbol_icon(symbol: &SymbolEntry) -> RowIcon {
    use archipel_kit_tui::{Symbol, SymbolKind as Kind};
    let kind = match symbol.kind {
        SymbolKind::Class => Kind::Class,
        SymbolKind::Interface => Kind::Interface,
        SymbolKind::Enum => Kind::Enum,
        SymbolKind::Method => Kind::Method,
        SymbolKind::Function => Kind::Function,
        SymbolKind::Constructor => Kind::Constructor,
        SymbolKind::Field => Kind::Field,
        SymbolKind::Constant => Kind::Constant,
        SymbolKind::Module => Kind::Module,
        _ => return RowIcon::Glyph(Glyph::File),
    };
    RowIcon::Symbol(Symbol::new(kind))
}

impl TuiApp {
    /// Fills the open picker when its query or tab changed or a search is still running.
    pub(crate) fn refresh_picker(&mut self) {
        let Some(shared) = self.picker.clone() else {
            return;
        };
        if self.layers.is_empty() {
            self.picker = None;
            return;
        }
        if !shared.borrow().stale() {
            return;
        }
        let (kind, tab, query) = {
            let picker = shared.borrow();
            (picker.kind, picker.tab, picker.query.text().to_owned())
        };
        let (rows, busy) = match kind {
            PickerKind::SearchEverywhere => {
                let tab = SearchTab::ALL[tab.min(SearchTab::ALL.len() - 1)];
                self.search_rows(tab, &query)
            }
            PickerKind::Files => self.search_rows(SearchTab::Files, &query),
            PickerKind::Actions => self.search_rows(SearchTab::Actions, &query),
            PickerKind::Recent => (self.recent_rows(tab, &query), false),
            PickerKind::Configs => (self.config_rows(&query), false),
        };
        let mut picker = shared.borrow_mut();
        picker.busy = busy;
        picker.set_rows(rows);
    }

    /// The action candidates with their presentation in Find Action.
    fn action_candidates(&self, include_disabled: bool) -> Vec<ActionCandidate> {
        let ide = &self.ide;
        ide.with_data(
            |data| data,
            |registry, data| {
                let present = |id: &str| {
                    registry
                        .update(id, ActionPlace::FindAction, data)
                        .unwrap_or_else(Presentation::disabled)
                };
                let own = TUI_ACTIONS
                    .iter()
                    .map(|(id, label, category)| (present(id), id, label, category))
                    .filter(|(presentation, ..)| presentation.visible)
                    .map(|(presentation, id, label, category)| ActionCandidate {
                        enabled: presentation.enabled,
                        shortcut: ide.shortcut_label(id).unwrap_or_default(),
                        id: (*id).to_owned(),
                        label: presentation.text.unwrap_or_else(|| (*label).to_owned()),
                        category: (*category).to_owned(),
                    })
                    .filter(|candidate| include_disabled || candidate.enabled);
                ide.specs()
                    .into_iter()
                    .filter(parameterless)
                    .map(|spec| (present(&spec.id), spec))
                    .filter(|(presentation, _)| presentation.visible)
                    .filter(|(presentation, _)| include_disabled || presentation.enabled)
                    .filter(|(presentation, spec)| {
                        spec.category != "Android"
                            || data.flag(&data_keys::ANDROID_PROJECT)
                            || presentation.enabled
                    })
                    .map(|(presentation, spec)| {
                        let own = OWN_COMMANDS
                            .iter()
                            .find(|(command, _)| *command == spec.id)
                            .map(|(_, action)| *action);
                        let shortcut = actions_for_command(&spec.id)
                            .chain(own)
                            .find_map(|action| ide.shortcut_label(action))
                            .unwrap_or_default();
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

    fn search_rows(&mut self, tab: SearchTab, query: &str) -> (Vec<PickerRow>, bool) {
        let state = self.ide.state_mut();
        if matches!(
            tab,
            SearchTab::All | SearchTab::Classes | SearchTab::Symbols
        ) {
            state.refresh_symbol_index(INDEX_BUDGET);
            if self.se_server_query != query {
                self.se_server_query = query.to_owned();
                self.ide.state_mut().request_server_symbols(query);
            }
            self.ide.state_mut().poll_server_symbols();
        }
        let mut busy = false;
        if matches!(tab, SearchTab::All | SearchTab::Text) {
            self.ide.state_mut().request_text_search(query);
            self.ide.state_mut().poll_text_search();
            busy = self.ide.state().text_search_running();
        }
        let actions = if matches!(tab, SearchTab::All | SearchTab::Actions) {
            let include =
                tab == SearchTab::Actions && self.ide.state().actions().read().include_disabled;
            self.action_candidates(include)
        } else {
            Vec::new()
        };
        let state = self.ide.state();
        let manager = state.search_everywhere();
        let navigation = manager.read();
        let sources = SearchSources {
            files: state.files(),
            recent: state.recent_files(),
            symbols: navigation.index.all().collect(),
            server_symbols: &navigation.server.results,
            actions: &actions,
            text: &navigation.text_results,
            include_non_project: false,
        };
        let mut found = search(tab, query, &sources);
        if tab == SearchTab::All && query.is_empty() {
            found.retain(|hit| matches!(hit.item, Hit::File { .. }));
        }
        found.truncate(MAX_ROWS);
        (found.iter().map(row_of).collect(), busy)
    }

    fn recent_rows(&self, tab: usize, query: &str) -> Vec<PickerRow> {
        let state = self.ide.state();
        if tab == 0 {
            state
                .recent_files()
                .iter()
                .filter(|path| matches(query, &file_name(path)))
                .map(|path| {
                    let mut row = PickerRow::new(
                        file_name(path),
                        Target::File {
                            path: path.clone(),
                            line: None,
                            column: None,
                            length: None,
                        },
                    );
                    row.icon = Some(file_icon(path));
                    row.detail = path
                        .parent()
                        .map(|parent| parent.display().to_string())
                        .unwrap_or_default();
                    row.footer = path.display().to_string();
                    row
                })
                .take(MAX_ROWS)
                .collect()
        } else {
            state
                .recent_locations()
                .iter()
                .filter(|location| matches(query, &location.preview))
                .map(|location| {
                    let mut row = PickerRow::new(
                        location.preview.clone(),
                        Target::File {
                            path: location.path.clone(),
                            line: Some(location.line + 1),
                            column: None,
                            length: None,
                        },
                    );
                    row.icon = Some(file_icon(&location.path));
                    row.trailing = format!("{}:{}", file_name(&location.path), location.line + 1);
                    row.footer = location.path.display().to_string();
                    row
                })
                .take(MAX_ROWS)
                .collect()
        }
    }

    fn config_rows(&self, query: &str) -> Vec<PickerRow> {
        let execution = self.ide.state().execution();
        let configurations = execution.configurations();
        configurations
            .configurations()
            .iter()
            .filter(|config| matches(query, &config.name))
            .map(|config| {
                let mut row =
                    PickerRow::new(config.name.clone(), Target::Config(config.name.clone()));
                row.icon = Some(RowIcon::Glyph(Glyph::Run));
                row.detail = config.target.type_name().to_owned();
                row.footer = "Enter runs the configuration".to_owned();
                row
            })
            .collect()
    }
}

fn row_of(ranked: &Ranked<Hit>) -> PickerRow {
    let mut row = match &ranked.item {
        Hit::Action(action) => {
            let mut row = PickerRow::new(action.label.clone(), Target::Action(action.id.clone()));
            row.icon = Some(RowIcon::Glyph(Glyph::Run));
            row.detail = action.category.clone();
            row.trailing = action.shortcut.clone();
            row.enabled = action.enabled;
            row.footer = action.id.clone();
            row
        }
        Hit::File { path, line, .. } => {
            let mut row = PickerRow::new(
                file_name(path),
                Target::File {
                    path: path.clone(),
                    line: *line,
                    column: None,
                    length: None,
                },
            );
            row.icon = Some(file_icon(path));
            row.detail = path
                .parent()
                .map(|parent| parent.display().to_string())
                .unwrap_or_default();
            row.trailing = line.map(|line| format!(":{line}")).unwrap_or_default();
            row.footer = path.display().to_string();
            row
        }
        Hit::Symbol(symbol) => {
            let mut row = PickerRow::new(
                symbol.name.clone(),
                Target::File {
                    path: symbol.path.clone(),
                    line: Some(symbol.line + 1),
                    column: None,
                    length: None,
                },
            );
            row.icon = Some(symbol_icon(symbol));
            row.detail = symbol.container.clone().unwrap_or_default();
            row.trailing = format!("{}:{}", file_name(&symbol.path), symbol.line + 1);
            row.footer = symbol.path.display().to_string();
            row
        }
        Hit::Text(found) => text_row(found),
    };
    row.positions = match &ranked.item {
        Hit::Text(found) => found
            .highlights
            .iter()
            .flat_map(|range| range.clone())
            .collect(),
        _ => ranked.positions.clone(),
    };
    row
}

fn text_row(found: &TextMatch) -> PickerRow {
    let mut row = PickerRow::new(
        found.preview.clone(),
        Target::File {
            path: found.path.clone(),
            line: Some(found.line),
            column: Some(found.column),
            length: Some(found.length),
        },
    );
    row.icon = Some(file_icon(&found.path));
    row.trailing = format!("{}:{}", file_name(&found.path), found.line);
    row.footer = found.path.display().to_string();
    row
}
