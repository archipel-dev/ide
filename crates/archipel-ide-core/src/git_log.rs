//! State and commands behind the Git tool window's Log tab: history with a
//! commit graph, branch/text filters, and the selected commit's details.

use crate::{CommandError, IdeState, ToolWindow, spec, string_param};
use archipel_ide_command::{Command, CommandRegistry};
use archipel_ide_git::{
    Branches, CommitDetails, CommitInfo, GraphRow, LogQuery, branches, commit_details, graph, log,
};
use serde_json::{Value, json};
use std::path::PathBuf;

/// Commits loaded into the log at once.
const LOG_LIMIT: usize = 2_000;

/// Date filter of the log (the reference "Date" combo).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum DateRange {
    #[default]
    Any,
    Today,
    Last7Days,
    Last30Days,
    LastYear,
}

impl DateRange {
    pub const ALL: [DateRange; 5] = [
        DateRange::Any,
        DateRange::Today,
        DateRange::Last7Days,
        DateRange::Last30Days,
        DateRange::LastYear,
    ];

    pub fn label(self) -> &'static str {
        match self {
            DateRange::Any => "Date",
            DateRange::Today => "Today",
            DateRange::Last7Days => "Last 7 days",
            DateRange::Last30Days => "Last 30 days",
            DateRange::LastYear => "Last year",
        }
    }

    pub fn id(self) -> &'static str {
        match self {
            DateRange::Any => "any",
            DateRange::Today => "today",
            DateRange::Last7Days => "7d",
            DateRange::Last30Days => "30d",
            DateRange::LastYear => "1y",
        }
    }

    pub fn parse(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|range| range.id() == id)
    }

    /// Age in seconds a commit may have, `None` for no limit.
    fn max_age(self) -> Option<i64> {
        match self {
            DateRange::Any => None,
            DateRange::Today => Some(86_400),
            DateRange::Last7Days => Some(7 * 86_400),
            DateRange::Last30Days => Some(30 * 86_400),
            DateRange::LastYear => Some(365 * 86_400),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct GitLogState {
    pub commits: Vec<CommitInfo>,
    /// Graph row per entry of `commits`.
    pub graph: Vec<GraphRow>,
    pub branches: Branches,
    /// Branch filter; `None` shows all branches.
    pub branch: Option<String>,
    /// Only commits touching this repository-relative path (file history).
    pub path: Option<PathBuf>,
    /// Sibling `History: <file>` tabs next to the `Log` tab. A tab is active
    /// while `path` equals its file; the tabs share the other filters.
    pub history_tabs: Vec<PathBuf>,
    /// Text or hash filter.
    pub filter: String,
    /// Treat `filter` as a regular expression (`.*` toggle).
    pub regex: bool,
    /// Match case of `filter` (`Cc` toggle).
    pub match_case: bool,
    /// Only commits by this author.
    pub user: Option<String>,
    pub date: DateRange,
    /// Hide the changed files / details pane.
    pub hide_details: bool,
    pub selected: Option<String>,
    pub details: Option<CommitDetails>,
    pub error: Option<String>,
    /// Whether the log reflects the current repository state.
    pub loaded: bool,
}

impl GitLogState {
    /// The history tab currently shown, if any.
    pub fn active_history_tab(&self) -> Option<&PathBuf> {
        self.path
            .as_ref()
            .filter(|path| self.history_tabs.contains(path))
    }

    /// Indices into `commits` that pass the text, user and date filters.
    pub fn visible(&self) -> Vec<usize> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |duration| duration.as_secs() as i64);
        self.visible_at(now)
    }

    pub fn visible_at(&self, now: i64) -> Vec<usize> {
        let text = self.filter.trim();
        let pattern = (self.regex && !text.is_empty())
            .then(|| {
                regex::RegexBuilder::new(text)
                    .case_insensitive(!self.match_case)
                    .build()
                    .ok()
            })
            .flatten();
        let max_age = self.date.max_age();
        self.commits
            .iter()
            .enumerate()
            .filter(|(_, commit)| {
                let text_ok = match &pattern {
                    Some(pattern) => {
                        pattern.is_match(&commit.subject)
                            || pattern.is_match(&commit.author)
                            || pattern.is_match(&commit.hash)
                    }
                    None if self.match_case && !text.is_empty() => {
                        commit.hash.starts_with(text)
                            || commit.subject.contains(text)
                            || commit.author.contains(text)
                    }
                    None => commit.matches(text),
                };
                text_ok
                    && self.user.as_ref().is_none_or(|user| &commit.author == user)
                    && max_age.is_none_or(|age| now - commit.time <= age)
            })
            .map(|(index, _)| index)
            .collect()
    }

    /// Distinct commit authors, for the User combo.
    pub fn users(&self) -> Vec<String> {
        let mut users: Vec<String> = self.commits.iter().map(|c| c.author.clone()).collect();
        users.sort();
        users.dedup();
        users
    }

    /// Whether the text filter is a regex that does not compile.
    pub fn regex_error(&self) -> bool {
        self.regex
            && !self.filter.trim().is_empty()
            && regex::Regex::new(self.filter.trim()).is_err()
    }
}

impl IdeState {
    /// Reloads history and branches, keeping the selection when possible.
    pub fn refresh_git_log(&mut self) -> Result<(), String> {
        let root = self
            .git_status()
            .ok_or("the project is not a Git repository")?
            .root
            .clone();
        let service = self.git_log().ok_or("no project is open")?;
        let (branch, path) = service.with_mut(|state| {
            state.loaded = true;
            (state.branch.clone(), state.path.clone())
        });
        let result = branches(&root).and_then(|branches| {
            let commits = log(
                &root,
                &LogQuery {
                    revision: branch,
                    path,
                    limit: LOG_LIMIT,
                },
            )?;
            Ok((branches, commits))
        });
        let (branches, commits) = match result {
            Ok(loaded) => loaded,
            Err(error) => {
                service.with_mut(|state| state.error = Some(error.to_string()));
                return Err(error.to_string());
            }
        };
        let selected = service.with_mut(|state| {
            state.graph = graph(&commits);
            state.commits = commits;
            state.branches = branches;
            state.error = None;
            let keep = state
                .selected
                .clone()
                .filter(|hash| state.commits.iter().any(|commit| &commit.hash == hash));
            keep.or_else(|| state.commits.first().map(|commit| commit.hash.clone()))
        });
        match selected {
            Some(hash) => self.select_commit(&hash),
            None => {
                service.with_mut(|state| {
                    state.selected = None;
                    state.details = None;
                });
                Ok(())
            }
        }
    }

    /// Marks the log stale; it reloads the next time it is shown. A repository change does
    /// this by itself (the service subscribes to it); this is for changes Git status did not
    /// see, such as the end of a fetch.
    pub(crate) fn invalidate_git_log(&self) {
        if let Some(service) = self.git_log() {
            service.invalidate();
        }
    }

    pub fn select_commit(&mut self, hash: &str) -> Result<(), String> {
        let root = self
            .git_status()
            .ok_or("the project is not a Git repository")?
            .root
            .clone();
        let service = self.git_log().ok_or("no project is open")?;
        if !service
            .snapshot()
            .commits
            .iter()
            .any(|commit| commit.hash == hash)
        {
            return Err(format!("commit {hash} is not in the loaded log"));
        }
        let details = commit_details(&root, hash).map_err(|error| error.to_string())?;
        service.with_mut(|state| {
            state.selected = Some(hash.to_owned());
            state.details = Some(details);
        });
        Ok(())
    }

    /// Changes the Git Log tab's filters and selection.
    pub(crate) fn git_log_update<R>(&self, f: impl FnOnce(&mut GitLogState) -> R) -> Option<R> {
        Some(self.git_log()?.with_mut(f))
    }
}

pub(crate) fn register(registry: &mut CommandRegistry<IdeState>) {
    let has_git = |state: &IdeState| state.has_git();
    registry
        .register(Command::new(
            spec("git.log.refresh", "Refresh Log", "Git", None, "{}"),
            has_git,
            |state: &mut IdeState, _| {
                state.refresh_git_log().map_err(CommandError::Failed)?;
                Ok(json!({"commits": state.git_log_snapshot().commits.len()}))
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec("git.log.show", "Show Git Log", "Git", None, "{}"),
            has_git,
            |state: &mut IdeState, _| {
                state.tool_windows.show(ToolWindow::Git);
                if !state.git_log_snapshot().loaded {
                    state.refresh_git_log().map_err(CommandError::Failed)?;
                }
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "git.log.selectCommit",
                "Select Commit",
                "Git",
                None,
                r#"{"hash":"string"}"#,
            ),
            has_git,
            |state: &mut IdeState, params| {
                state
                    .select_commit(&string_param(&params, "hash")?)
                    .map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "git.log.filterBranch",
                "Filter Log by Branch",
                "Git",
                None,
                r#"{"branch":"string?"}"#,
            ),
            has_git,
            |state: &mut IdeState, params| {
                let branch = params
                    .get("branch")
                    .and_then(Value::as_str)
                    .filter(|branch| !branch.is_empty())
                    .map(String::from);
                state.git_log_update(|log| log.branch = branch);
                state.refresh_git_log().map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "git.log.filterText",
                "Filter Log by Text or Hash",
                "Git",
                None,
                r#"{"text":"string","regex":"bool?","matchCase":"bool?"}"#,
            ),
            has_git,
            |state: &mut IdeState, params| {
                let text = string_param(&params, "text")?;
                let regex = params.get("regex").and_then(Value::as_bool);
                let case = params.get("matchCase").and_then(Value::as_bool);
                state.git_log_update(|log| {
                    log.filter = text;
                    if let Some(regex) = regex {
                        log.regex = regex;
                    }
                    if let Some(case) = case {
                        log.match_case = case;
                    }
                });
                Ok(json!({"visible": state.git_log_snapshot().visible().len()}))
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "git.log.filterUser",
                "Filter Log by User",
                "Git",
                None,
                r#"{"user":"string?"}"#,
            ),
            has_git,
            |state: &mut IdeState, params| {
                let user = params
                    .get("user")
                    .and_then(Value::as_str)
                    .filter(|user| !user.is_empty())
                    .map(String::from);
                state.git_log_update(|log| log.user = user);
                Ok(json!({"visible": state.git_log_snapshot().visible().len()}))
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "git.log.filterDate",
                "Filter Log by Date",
                "Git",
                None,
                r#"{"range":"any|today|7d|30d|1y"}"#,
            ),
            has_git,
            |state: &mut IdeState, params| {
                let id = string_param(&params, "range")?;
                let date = DateRange::parse(&id)
                    .ok_or_else(|| CommandError::Failed(format!("unknown date range '{id}'")))?;
                state.git_log_update(|log| log.date = date);
                Ok(json!({"visible": state.git_log_snapshot().visible().len()}))
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "git.log.filterPath",
                "Filter Log by Path",
                "Git",
                None,
                r#"{"path":"string?"}"#,
            ),
            has_git,
            |state: &mut IdeState, params| {
                let path = params
                    .get("path")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|path| !path.is_empty())
                    .map(PathBuf::from);
                state.git_log_update(|log| log.path = path);
                state.refresh_git_log().map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "git.log.selectTab",
                "Select Log Tab",
                "Git",
                None,
                r#"{"path":"string?"}"#,
            ),
            has_git,
            |state: &mut IdeState, params| {
                let path = params
                    .get("path")
                    .and_then(Value::as_str)
                    .map(PathBuf::from);
                if let Some(path) = &path
                    && !state.git_log_snapshot().history_tabs.contains(path)
                {
                    return Err(CommandError::InvalidParams(format!(
                        "no history tab for `{}`",
                        path.display()
                    )));
                }
                state.git_log_update(|log| log.path = path);
                state.refresh_git_log().map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "git.log.closeTab",
                "Close History Tab",
                "Git",
                None,
                r#"{"path":"string"}"#,
            ),
            has_git,
            |state: &mut IdeState, params| {
                let path = PathBuf::from(
                    params
                        .get("path")
                        .and_then(Value::as_str)
                        .ok_or_else(|| CommandError::InvalidParams("missing `path`".into()))?,
                );
                let (removed, was_active) = state
                    .git_log_update(|log| {
                        let before = log.history_tabs.len();
                        log.history_tabs.retain(|tab| *tab != path);
                        let removed = log.history_tabs.len() != before;
                        let was_active = removed && log.path.as_ref() == Some(&path);
                        if was_active {
                            log.path = None;
                        }
                        (removed, was_active)
                    })
                    .unwrap_or_default();
                if !removed {
                    return Err(CommandError::InvalidParams(format!(
                        "no history tab for `{}`",
                        path.display()
                    )));
                }
                if was_active {
                    state.refresh_git_log().map_err(CommandError::Failed)?;
                }
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec("git.log.toggleDetails", "Show Details", "Git", None, "{}"),
            has_git,
            |state: &mut IdeState, _| {
                let hidden = state
                    .git_log_update(|log| {
                        log.hide_details = !log.hide_details;
                        log.hide_details
                    })
                    .unwrap_or_default();
                Ok(json!({"visible": !hidden}))
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "git.log.showFileDiff",
                "Show Diff for Commit File",
                "Git",
                None,
                r#"{"hash":"string","path":"string"}"#,
            ),
            has_git,
            |state: &mut IdeState, params| {
                let hash = string_param(&params, "hash")?;
                let path = PathBuf::from(string_param(&params, "path")?);
                state
                    .show_commit_diff(&hash, &path)
                    .map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
}

#[cfg(test)]
mod tests {
    use crate::{IdeState, ToolWindow, built_in_commands};
    use serde_json::{Value, json};
    use std::fs;
    use std::path::Path;

    fn git(root: &Path, args: &[&str]) {
        assert!(
            std::process::Command::new("git")
                .arg("-C")
                .arg(root)
                .args(args)
                .output()
                .unwrap()
                .status
                .success()
        );
    }

    #[test]
    fn loads_filters_and_inspects_history() {
        let root =
            std::env::temp_dir().join(format!("archipel-ide-core-log-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        git(&root, &["init", "-q", "-b", "main"]);
        git(&root, &["config", "user.email", "dev@example.invalid"]);
        git(&root, &["config", "user.name", "Dev"]);
        fs::write(root.join("a.txt"), "one\n").unwrap();
        git(&root, &["add", "."]);
        git(&root, &["commit", "-qm", "First"]);
        git(&root, &["checkout", "-qb", "topic"]);
        fs::write(root.join("a.txt"), "two\n").unwrap();
        git(&root, &["commit", "-qam", "Second on topic"]);
        git(&root, &["checkout", "-q", "main"]);

        let commands = built_in_commands();
        let mut state = IdeState::default();
        commands
            .execute("project.open", &mut state, json!({"path": root}))
            .unwrap();
        commands
            .execute("git.log.show", &mut state, Value::Null)
            .unwrap();
        assert!(state.tool_windows.is_visible(ToolWindow::Git));
        assert_eq!(state.git_log_snapshot().commits.len(), 2);
        assert_eq!(state.git_log_snapshot().graph.len(), 2);
        // Newest commit is selected with its details.
        let newest = state.git_log_snapshot().commits[0].hash.clone();
        assert_eq!(state.git_log_snapshot().selected.as_ref(), Some(&newest));
        assert_eq!(
            state
                .git_log_snapshot()
                .details
                .as_ref()
                .unwrap()
                .files
                .len(),
            1
        );

        let result = commands
            .execute("git.log.filterText", &mut state, json!({"text": "topic"}))
            .unwrap();
        assert_eq!(result["visible"], 1);
        commands
            .execute(
                "git.log.filterBranch",
                &mut state,
                json!({"branch": "main"}),
            )
            .unwrap();
        assert_eq!(state.git_log_snapshot().commits.len(), 1);
        commands
            .execute("git.log.filterBranch", &mut state, json!({}))
            .unwrap();
        assert_eq!(state.git_log_snapshot().commits.len(), 2);

        let result = commands
            .execute("git.log.filterUser", &mut state, json!({"user": "Nobody"}))
            .unwrap();
        assert_eq!(result["visible"], 0);
        commands
            .execute("git.log.filterUser", &mut state, json!({}))
            .unwrap();
        let result = commands
            .execute(
                "git.log.filterText",
                &mut state,
                json!({"text": "^Sec.*topic$", "regex": true}),
            )
            .unwrap();
        assert_eq!(result["visible"], 1);
        let result = commands
            .execute(
                "git.log.filterText",
                &mut state,
                json!({"text": "second", "regex": false, "matchCase": true}),
            )
            .unwrap();
        assert_eq!(result["visible"], 0);
        commands
            .execute("git.log.filterText", &mut state, json!({"text": ""}))
            .unwrap();
        commands
            .execute("git.log.filterDate", &mut state, json!({"range": "today"}))
            .unwrap();
        assert_eq!(state.git_log_snapshot().visible().len(), 2);
        commands
            .execute("git.log.filterDate", &mut state, json!({"range": "any"}))
            .unwrap();
        assert!(
            commands
                .execute("git.log.filterDate", &mut state, json!({"range": "x"}))
                .is_err()
        );
        commands
            .execute("git.log.filterPath", &mut state, json!({"path": "a.txt"}))
            .unwrap();
        assert_eq!(state.git_log_snapshot().commits.len(), 2);
        commands
            .execute("git.log.filterPath", &mut state, json!({"path": "missing"}))
            .unwrap();
        assert!(state.git_log_snapshot().commits.is_empty());
        commands
            .execute("git.log.filterPath", &mut state, json!({}))
            .unwrap();

        commands
            .execute(
                "git.log.showFileDiff",
                &mut state,
                json!({"hash": newest, "path": "a.txt"}),
            )
            .unwrap();
        let tab = state.diff.as_ref().unwrap();
        assert_eq!(tab.diff.old_text, "one\n");
        assert_eq!(tab.diff.new_text, "two\n");
        assert!(tab.new_revision.is_some());
        assert!(
            commands
                .execute("git.log.selectCommit", &mut state, json!({"hash": "ffff"}))
                .is_err()
        );
        fs::remove_dir_all(root).unwrap();
    }
}
