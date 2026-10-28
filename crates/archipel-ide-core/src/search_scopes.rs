//! User-defined Find in Files scopes: named file masks stored per project in
//! `.archipel/searchScopes.toml` and picked in the Find in Files popup.

use crate::{IdeState, SearchScopeKind, spec, string_param};
use archipel_ide_command::{Command, CommandError, CommandRegistry};
use archipel_ide_search::FileMaskFilter;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

const FILE: &str = ".archipel/searchScopes.toml";

/// A named scope: files admitted by `mask` (comma-separated globs; `!glob`
/// excludes; patterns with `/` match the project-relative path).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomScope {
    pub name: String,
    pub mask: String,
}

#[derive(Default, Serialize, Deserialize)]
struct ScopeFile {
    #[serde(default, rename = "scope")]
    scopes: Vec<CustomScope>,
}

fn load(root: &Path) -> Vec<CustomScope> {
    std::fs::read_to_string(archipel_ide_compat::project_read_path(root.join(FILE)))
        .ok()
        .and_then(|text| toml::from_str::<ScopeFile>(&text).ok())
        .map(|file| file.scopes)
        .unwrap_or_default()
}

fn save(root: &Path, scopes: &[CustomScope]) -> Result<(), String> {
    let path = archipel_ide_compat::project_write_path(root.join(FILE));
    let text = toml::to_string_pretty(&ScopeFile {
        scopes: scopes.to_vec(),
    })
    .map_err(|error| error.to_string())?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("{}: {error}", parent.display()))?;
    }
    std::fs::write(&path, text).map_err(|error| format!("{}: {error}", path.display()))
}

impl IdeState {
    pub(crate) fn load_search_scopes(&mut self) {
        let scopes = self
            .project
            .as_ref()
            .map(|project| load(project.root()))
            .unwrap_or_default();
        if let Some(find) = self.find() {
            let mut find = find.write();
            find.custom_scopes = scopes;
            find.custom_scope = None;
        }
    }

    /// The filter of the selected custom scope; `None` outside custom scopes.
    pub(crate) fn custom_scope_filter(&self) -> Option<FileMaskFilter> {
        let find = self.find()?;
        let find = find.read();
        let name = find.custom_scope.as_ref()?;
        let scope = find
            .custom_scopes
            .iter()
            .find(|scope| &scope.name == name)?;
        FileMaskFilter::new(&scope.mask).ok()
    }

    /// Adds or replaces the scope `name` and stores the scopes in the project.
    pub fn save_search_scope(&mut self, name: &str, mask: &str) -> Result<(), String> {
        let name = name.trim();
        if name.is_empty() {
            return Err("a scope needs a name".into());
        }
        FileMaskFilter::new(mask).map_err(|error| format!("invalid scope pattern: {error}"))?;
        let root: PathBuf = self
            .project
            .as_ref()
            .ok_or("no project is open")?
            .root()
            .to_owned();
        let find = self.find().ok_or("no project is open")?;
        let scope = CustomScope {
            name: name.to_owned(),
            mask: mask.trim().to_owned(),
        };
        let mut find = find.write();
        match find.custom_scopes.iter_mut().find(|s| s.name == name) {
            Some(existing) => *existing = scope,
            None => find.custom_scopes.push(scope),
        }
        save(&root, &find.custom_scopes)
    }

    pub fn delete_search_scope(&mut self, name: &str) -> Result<(), String> {
        let root: PathBuf = self
            .project
            .as_ref()
            .ok_or("no project is open")?
            .root()
            .to_owned();
        let find = self.find().ok_or("no project is open")?;
        let mut find = find.write();
        let before = find.custom_scopes.len();
        find.custom_scopes.retain(|scope| scope.name != name);
        if find.custom_scopes.len() == before {
            return Err(format!("no search scope named `{name}`"));
        }
        if find.custom_scope.as_deref() == Some(name) {
            find.custom_scope = None;
            find.scope_kind = SearchScopeKind::Project;
        }
        save(&root, &find.custom_scopes)
    }

    /// The results of the last Find in Files run (empty without a project).
    pub fn search_results(&self) -> Vec<archipel_ide_search::TextMatch> {
        self.find()
            .map(|find| find.read().results.clone())
            .unwrap_or_default()
    }

    pub fn search_result_count(&self) -> usize {
        self.find().map_or(0, |find| find.read().results.len())
    }

    pub fn search_query(&self) -> String {
        self.find()
            .map(|find| find.read().query.clone())
            .unwrap_or_default()
    }

    /// Options of the last Find in Files run.
    pub fn search_options(&self) -> archipel_ide_search::SearchOptions {
        self.find()
            .map(|find| find.read().options.clone())
            .unwrap_or_default()
    }

    /// Whether the last run stopped at the result limit.
    pub fn search_truncated(&self) -> bool {
        self.find().is_some_and(|find| find.read().truncated)
    }

    pub fn search_scope_kind(&self) -> SearchScopeKind {
        self.find()
            .map(|find| find.read().scope_kind)
            .unwrap_or_default()
    }

    /// The project-relative directory of the Directory scope.
    pub fn search_directory(&self) -> Option<PathBuf> {
        self.project_view.search_scope.clone()
    }

    pub fn set_search_directory(&mut self, directory: Option<PathBuf>) {
        self.project_view.search_scope = directory;
    }

    pub fn search_custom_scope(&self) -> Option<String> {
        self.find()
            .and_then(|find| find.read().custom_scope.clone())
    }

    pub fn set_search_scope_kind(&self, kind: SearchScopeKind) {
        if let Some(find) = self.find() {
            find.write().scope_kind = kind;
        }
    }

    /// Selects the user-defined scope `name` in the Find in Files popup.
    pub fn select_custom_scope(&self, name: &str) {
        if let Some(find) = self.find() {
            let mut find = find.write();
            find.scope_kind = SearchScopeKind::Custom;
            find.custom_scope = Some(name.to_owned());
        }
    }

    /// The user-defined Find in Files scopes of the open project.
    pub fn custom_scopes(&self) -> Vec<CustomScope> {
        self.find()
            .map(|find| find.read().custom_scopes.clone())
            .unwrap_or_default()
    }
}

pub(crate) fn register(registry: &mut CommandRegistry<IdeState>) {
    registry
        .register(Command::new(
            spec(
                "search.saveScope",
                "Save Search Scope",
                "Search",
                None,
                r#"{"name":"string","mask":"string"}"#,
            ),
            |state: &IdeState| state.project.is_some(),
            |state: &mut IdeState, params| {
                let name = string_param(&params, "name")?;
                let mask = string_param(&params, "mask")?;
                state
                    .save_search_scope(&name, &mask)
                    .map_err(CommandError::InvalidParams)?;
                Ok(json!({"scopes": state.custom_scopes().len()}))
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "search.deleteScope",
                "Delete Search Scope",
                "Search",
                None,
                r#"{"name":"string"}"#,
            ),
            |state: &IdeState| !state.custom_scopes().is_empty(),
            |state: &mut IdeState, params| {
                let name = string_param(&params, "name")?;
                state
                    .delete_search_scope(&name)
                    .map_err(CommandError::InvalidParams)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
}

#[cfg(test)]
mod tests {
    use crate::{IdeState, built_in_commands};
    use serde_json::json;
    use std::time::{Duration, Instant};

    #[test]
    fn custom_scopes_limit_find_in_files_and_persist() {
        let root = std::env::temp_dir().join(format!("archipel-ide-scopes-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("api")).unwrap();
        std::fs::write(root.join("api/A.java"), "needle\n").unwrap();
        std::fs::write(root.join("B.java"), "needle\n").unwrap();
        std::fs::write(root.join("notes.txt"), "needle\n").unwrap();
        let commands = built_in_commands();
        let mut state = IdeState::default();
        state.open_project(&root).unwrap();
        let run = |state: &mut IdeState, params: serde_json::Value| {
            commands.execute("search.project", state, params).unwrap();
            let deadline = Instant::now() + Duration::from_secs(3);
            while state.search_running() && Instant::now() < deadline {
                state.poll_search();
                std::thread::sleep(Duration::from_millis(5));
            }
            state.poll_search();
            let mut paths: Vec<_> = state
                .find()
                .unwrap()
                .read()
                .results
                .iter()
                .map(|found| found.path.display().to_string())
                .collect();
            paths.sort();
            paths
        };
        assert!(
            commands
                .execute(
                    "search.saveScope",
                    &mut state,
                    json!({"name": "Bad", "mask": "["})
                )
                .is_err()
        );
        commands
            .execute(
                "search.saveScope",
                &mut state,
                json!({"name": "Java outside api", "mask": "*.java, !api/**"}),
            )
            .unwrap();
        let hits = run(
            &mut state,
            json!({"query": "needle", "scopeKind": "custom", "scopeName": "Java outside api"}),
        );
        assert_eq!(hits, ["B.java"]);
        let hits = run(&mut state, json!({"query": "needle"}));
        assert_eq!(hits.len(), 3);

        // Scopes live in the project and come back when it is reopened.
        let mut reopened = IdeState::default();
        reopened.open_project(&root).unwrap();
        assert_eq!(reopened.custom_scopes().len(), 1);
        assert_eq!(reopened.custom_scopes()[0].mask, "*.java, !api/**");

        commands
            .execute(
                "search.deleteScope",
                &mut state,
                json!({"name": "Java outside api"}),
            )
            .unwrap();
        assert!(state.custom_scopes().is_empty());
        assert!(
            commands
                .execute("search.deleteScope", &mut state, json!({"name": "x"}))
                .is_err()
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}
