//! The rows of the pickers: Search Everywhere, Go to File, Find Action, Recent Files and the
//! run configurations. Ranking and sources are the core's (`quick_nav`); this only turns the
//! hits into rows.

use crate::app::TuiApp;
use crate::layout::file_icon;
use crate::picker::{PickerKind, PickerRow, Target};
use archipel_ide_core::quick_nav::{SearchTab, file_name, matches};
use archipel_ide_view::popups::search::{self as model, ActionExtra, SearchRow};
use archipel_kit_tui::Glyph;
use archipel_kit_tui::components::RowIcon;

const MAX_ROWS: usize = 60;

/// The actions of this frontend that no command stands for, with their menu texts.
/// Commands whose reference action this frontend implements itself.
const OWN_COMMANDS: [(&str, &str); 3] = [
    ("editor.saveAll", "SaveAll"),
    ("editor.gotoLine", "GotoLine"),
    ("search.project", "FindInPath"),
];

const TUI_ACTIONS: [ActionExtra; 6] = [
    ActionExtra {
        id: "GotoLine",
        label: "Go to Line:Column…",
        category: "Navigate",
    },
    ActionExtra {
        id: "FindInPath",
        label: "Find in Files…",
        category: "Search",
    },
    ActionExtra {
        id: "NewFile",
        label: "New File…",
        category: "File",
    },
    ActionExtra {
        id: "OpenFile",
        label: "Open Folder…",
        category: "File",
    },
    ActionExtra {
        id: "Exit",
        label: "Exit",
        category: "File",
    },
    ActionExtra {
        id: "Archipel.NextTheme",
        label: "Switch Theme",
        category: "View",
    },
];

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

    fn search_rows(&mut self, tab: SearchTab, query: &str) -> (Vec<PickerRow>, bool) {
        let busy = model::prepare(self.ide.state_mut(), tab, query, &mut self.se_server_query);
        let (include_non_project, include_disabled) = self
            .picker
            .as_ref()
            .map(|shared| {
                let shared = shared.borrow();
                (shared.include_non_project, shared.include_disabled)
            })
            .unwrap_or_default();
        let cx = self.ide.cx();
        let candidates = if model::needs_actions(tab) {
            // The All tab never lists disabled actions; the Actions tab has the checkbox.
            model::action_candidates(
                &cx,
                &TUI_ACTIONS,
                "",
                &OWN_COMMANDS,
                tab == SearchTab::Actions && include_disabled,
                &|_| String::new(),
            )
        } else {
            Vec::new()
        };
        let found = model::hits(&cx, tab, query, include_non_project, &candidates);
        let rows = model::rows(tab, !query.is_empty(), &found);
        (rows.into_iter().map(row_of).collect(), busy)
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
            .chain(
                // What the build system detected (Cargo, Gradle, Maven, npm, Go): a target that
                // cannot start is listed greyed out with the reason.
                self.ide
                    .state()
                    .detected_targets()
                    .into_iter()
                    .filter(|target| matches(query, &target.label))
                    .map(|target| {
                        let mut row =
                            PickerRow::new(target.label.clone(), Target::Command(target.command));
                        row.icon = Some(RowIcon::Glyph(Glyph::Run));
                        row.detail = target.kind.clone();
                        row.enabled = target.unavailable.is_none();
                        row.footer = target
                            .unavailable
                            .unwrap_or_else(|| "Enter runs the target".to_owned());
                        row
                    }),
            )
            .collect()
    }
}

fn row_of(shared: SearchRow) -> PickerRow {
    let mut row = PickerRow::new(shared.title.clone(), Target::Hit(shared.hit.clone()));
    row.icon_key = Some(shared.icon.clone());
    row.detail = shared.inline.clone();
    row.trailing = shared.trailing.clone();
    row.enabled = shared.enabled;
    row.footer = shared.footer.clone();
    row.positions = shared.positions.clone();
    row.group = shared.group;
    row
}
