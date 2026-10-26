//! The daemon in the app (WP3): registers the [`DaemonService`] into the
//! analysis registry, feeds it caret and settings once per frame
//! ([`IdeState::poll_daemon`], called from `poll_analysis`), keeps the
//! inspection profile in sync with the project and the settings, and exposes
//! the `inspection.*` commands.

use crate::{IdeState, spec, string_param};
use archipel_ide_analysis::inspect::AnalysisScope;
use archipel_ide_analysis::inspect::Level;
use archipel_ide_analysis::{AnalysisEvent, Change, FileId, ServiceRegistry};
use archipel_ide_command::{Command, CommandError, CommandRegistry};
use archipel_ide_daemon::inspection::catalog;
use archipel_ide_daemon::inspection::profile::{self, PROJECT_DEFAULT_PROFILE};
use archipel_ide_daemon::{DaemonService, DaemonSettings};
use archipel_ide_settings::{SettingDef, SettingKind, SettingValue};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

/// The Settings page "Editor > Inspections" (reference id `Errors`).
pub const INSPECTIONS_PAGE: &str = "Errors";
pub const SETTING_PROFILE: &str = "inspections.profile";
pub const SETTING_ENABLED: &str = "inspections.enabled";
pub const SETTING_DELAY: &str = "daemon.autoReparseDelayMs";

/// Registers the daemon service (passes, Tier-A inspections, suppressors).
pub(crate) fn daemon_services(registry: &mut ServiceRegistry) {
    DaemonService::with_system_clock().register(registry);
}

/// The config directory (for the intention actions that edit profiles).
pub(crate) fn config_dir_for_actions() -> Option<&'static Path> {
    config_dir()
}

fn config_dir() -> Option<&'static Path> {
    static DIR: OnceLock<Option<PathBuf>> = OnceLock::new();
    DIR.get_or_init(archipel_ide_settings::default_config_dir)
        .as_deref()
}

impl IdeState {
    /// Changes whenever the daemon's markup or "problem files" change (cache key).
    pub(crate) fn daemon_version(&self) -> u64 {
        self.daemon_service().map_or(0, |service| {
            let daemon = service.lock();
            daemon
                .store()
                .version()
                .wrapping_mul(31)
                .wrapping_add(daemon.problem_files().version())
        })
    }

    /// The daemon of this state's analysis host.
    pub fn daemon_service(&self) -> Option<Arc<DaemonService>> {
        DaemonService::from_registry(self.analysis.host.registry())
    }

    fn ensure_inspection_settings(&mut self) {
        let registry = &mut self.settings.registry;
        if registry.setting(SETTING_PROFILE).is_some() || registry.page(INSPECTIONS_PAGE).is_none()
        {
            return;
        }
        let defs = [
            SettingDef::new(
                SETTING_ENABLED,
                INSPECTIONS_PAGE,
                "Highlight problems while typing",
                SettingKind::Bool,
                SettingValue::Bool(true),
            )
            .description("Run the inspections of the active profile in the background"),
            SettingDef::new(
                SETTING_PROFILE,
                INSPECTIONS_PAGE,
                "Profile",
                SettingKind::Text,
                SettingValue::Text(PROJECT_DEFAULT_PROFILE.into()),
            )
            .group("Inspection profile")
            .description(
                "Profiles live in .archipel/inspection-profiles (project) and the config directory",
            ),
            SettingDef::new(
                SETTING_DELAY,
                INSPECTIONS_PAGE,
                "Delay before analysis (ms)",
                SettingKind::Int { min: 0, max: 5000 },
                SettingValue::Int(300),
            )
            .group("Inspection profile")
            .description("Quiet time after the last keystroke before inspections run"),
        ];
        for def in defs {
            let _ = registry.register_setting(def);
        }
    }

    /// Per-frame daemon driver; returns true when highlights changed.
    pub fn poll_daemon(&mut self, events: &[AnalysisEvent]) -> bool {
        self.ensure_inspection_settings();
        let Some(service) = self.daemon_service() else {
            return false;
        };
        let selected = self
            .settings
            .text(SETTING_PROFILE)
            .filter(|name| !name.trim().is_empty())
            .unwrap_or_else(|| PROJECT_DEFAULT_PROFILE.into());
        let root = self.project.as_ref().map(|p| p.root().to_owned());
        let (reloaded, errors) =
            service.reload_profiles(root.as_deref(), config_dir(), &selected, false);
        if reloaded {
            if let Some(error) = errors.first() {
                self.status = error.to_string();
            }
            self.analysis.host.apply(Change::SettingsChanged);
        }
        let settings = DaemonSettings {
            debounce: Duration::from_millis(
                self.settings.int(SETTING_DELAY).unwrap_or(300).max(0) as u64
            ),
            enabled: self.settings.bool(SETTING_ENABLED).unwrap_or(true),
            ..service.lock().settings()
        };
        let caret = self.active_document.and_then(|index| {
            let document = self.documents.get(index)?;
            let file = self.analysis.host.lookup(&document.path)?;
            let char_index = document
                .selections
                .primary()
                .head
                .min(document.text.len_chars());
            Some((file, document.text.char_to_byte(char_index) as u32))
        });
        let mut daemon = service.lock();
        if daemon.settings() != settings {
            daemon.set_settings(settings);
        }
        if let Some((file, byte)) = caret {
            daemon.set_caret(file, byte);
        }
        let mut changed = daemon.tick(&mut self.analysis.host, events);
        drop(daemon);
        changed |= service.poll_batch();
        changed
    }

    /// Files of the project to inspect: open documents by their editor id (so
    /// unsaved text is used), every other file with a known language by its
    /// absolute path.
    fn inspection_files(&mut self) -> Vec<FileId> {
        let Some(project) = self.project.clone() else {
            return Vec::new();
        };
        let mut files = Vec::new();
        for entry in project.entries(200_000) {
            if entry.is_dir {
                continue;
            }
            let absolute = project.root().join(&entry.relative_path);
            if archipel_ide_language::registry::detect(&absolute, "").is_none() {
                continue;
            }
            let open = self.documents.iter().any(|d| d.path == entry.relative_path);
            let id = if open {
                self.analysis.host.file_id(&entry.relative_path)
            } else {
                self.analysis.host.file_id(&absolute)
            };
            files.push(id);
        }
        files
    }

    /// Starts "Inspect Code" over `scope` (`project` or `file`).
    pub fn run_inspection(&mut self, scope: &str) -> Result<usize, String> {
        let service = self
            .daemon_service()
            .ok_or("the analysis daemon is not available")?;
        let (analysis_scope, label) = match scope {
            "file" => {
                let index = self.active_document.ok_or("no active document")?;
                let path = self.documents[index].path.clone();
                let file = self.analysis.host.file_id(&path);
                (
                    AnalysisScope::Files(vec![file]),
                    format!("File '{}'", path.display()),
                )
            }
            "project" => {
                let files = self.inspection_files();
                (AnalysisScope::Files(files), "Whole project".to_owned())
            }
            other => {
                return Err(format!(
                    "unknown scope `{other}`; expected `file` or `project`"
                ));
            }
        };
        let count = match &analysis_scope {
            AnalysisScope::Files(files) => files.len(),
            _ => 0,
        };
        service.start_batch(&mut self.analysis.host, analysis_scope, &label);
        self.poll_daemon(&[]);
        Ok(count)
    }
}

fn level_name(level: Level) -> &'static str {
    profile::level_name(level)
}

pub(crate) fn register(registry: &mut CommandRegistry<IdeState>) {
    registry
        .register(Command::new(
            spec("inspection.list", "List Inspections", "Code", None, "{}"),
            |_| true,
            |state: &mut IdeState, _| {
                state.poll_analysis();
                let tools = catalog::inspections(state.analysis.host.registry());
                let service = state.daemon_service();
                let profile = service.as_ref().map(|s| s.profile());
                let path = state
                    .active_document
                    .and_then(|i| state.documents.get(i))
                    .map(|d| d.path.clone())
                    .unwrap_or_default();
                Ok(json!({
                    "profile": service.as_ref().map(|s| s.profiles().selected.clone()),
                    "tools": tools.iter().map(|t| {
                        let (enabled, level) = match &profile {
                            Some(profile) => profile.state_for(t.id, t.enabled_by_default, t.default_level, &path),
                            None => (t.enabled_by_default, t.default_level),
                        };
                        json!({
                            "id": t.id, "name": t.name, "group": t.group, "description": t.description,
                            "enabled": enabled, "level": level_name(level), "default": level_name(t.default_level),
                            "languages": t.languages,
                            "options": t.options.iter().map(|o| json!({"id": o.id, "label": o.label})).collect::<Vec<_>>(),
                        })
                    }).collect::<Vec<_>>(),
                }))
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec("inspection.status", "Daemon Status", "Code", None, "{}"),
            |_| true,
            |state: &mut IdeState, _| {
                state.poll_analysis();
                let Some(service) = state.daemon_service() else {
                    return Ok(json!({"available": false}));
                };
                let daemon = service.lock();
                let passes = ["syntax", "general", "local-inspections", "external-annotators", "identifier-highlighter", "line-markers"];
                let counts = daemon.problem_files().counts();
                Ok(json!({
                    "available": true,
                    "idle": daemon.all_idle(),
                    "appliedResults": daemon.applied_results(),
                    "debounceMs": daemon.settings().debounce.as_millis() as u64,
                    "problems": {"errors": counts.errors, "warnings": counts.warnings, "weakWarnings": counts.weak_warnings},
                    "passes": passes.iter().map(|id| {
                        let s = daemon.pass_stats(archipel_ide_analysis::daemon::PassId(id));
                        json!({"id": id, "runs": s.runs, "completed": s.completed, "cancelled": s.cancelled, "lastMs": s.last_time.as_secs_f64() * 1000.0})
                    }).collect::<Vec<_>>(),
                }))
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "inspection.profile.list",
                "List Inspection Profiles",
                "Code",
                None,
                "{}",
            ),
            |_| true,
            |state: &mut IdeState, _| {
                state.poll_analysis();
                let service = state
                    .daemon_service()
                    .ok_or_else(|| CommandError::Failed("daemon unavailable".into()))?;
                let profiles = service.profiles();
                Ok(json!({"selected": profiles.selected, "profiles": profiles.store.names()}))
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "inspection.profile.select",
                "Select Inspection Profile",
                "Code",
                None,
                r#"{"name":"string"}"#,
            ),
            |_| true,
            |state: &mut IdeState, params| {
                let name = string_param(&params, "name")?;
                state
                    .settings
                    .store
                    .set(
                        &state.settings.registry,
                        SETTING_PROFILE,
                        SettingValue::Text(name.clone()),
                    )
                    .map_err(|e| CommandError::Failed(e.to_string()))?;
                state.poll_analysis();
                let service = state
                    .daemon_service()
                    .ok_or_else(|| CommandError::Failed("daemon unavailable".into()))?;
                let selected = service.profiles().selected.clone();
                Ok(json!({"selected": selected}))
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "inspection.profile.edit",
                "Edit Inspection Profile",
                "Code",
                None,
                r#"{"tool":"string","enabled":"bool?","level":"string?","option":"string?","value":"any?"}"#,
            ),
            |_| true,
            |state: &mut IdeState, params| {
                let tool = string_param(&params, "tool")?;
                let service = state.daemon_service().ok_or_else(|| CommandError::Failed("daemon unavailable".into()))?;
                let level = match params.get("level").and_then(Value::as_str) {
                    Some(name) => Some(profile::parse_level(name).ok_or_else(|| CommandError::InvalidParams(format!("unknown level `{name}`")))?),
                    None => None,
                };
                let enabled = params.get("enabled").and_then(Value::as_bool);
                let option = params.get("option").and_then(Value::as_str).map(str::to_owned);
                let value = params.get("value").and_then(|v| match v {
                    Value::Bool(b) => Some(toml::Value::Boolean(*b)),
                    Value::Number(n) => n.as_i64().map(toml::Value::Integer),
                    Value::String(s) => Some(toml::Value::String(s.clone())),
                    _ => None,
                });
                let root = state.project.as_ref().map(|p| p.root().to_owned());
                let name = service
                    .edit_profile(root.as_deref(), config_dir(), |file| {
                        if enabled.is_some() {
                            file.set_enabled(&tool, enabled);
                        }
                        if level.is_some() {
                            file.set_level(&tool, level);
                        }
                        if let Some(option) = &option {
                            file.set_option(&tool, option, value);
                        }
                    })
                    .map_err(|e| CommandError::Failed(e.to_string()))?;
                state
                    .settings
                    .store
                    .set(&state.settings.registry, SETTING_PROFILE, SettingValue::Text(name.clone()))
                    .map_err(|e| CommandError::Failed(e.to_string()))?;
                state.analysis.host.apply(Change::SettingsChanged);
                state.poll_analysis();
                Ok(json!({"profile": name}))
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "inspection.runCode",
                "Inspect Code…",
                "Code",
                None,
                r#"{"scope":"project|file?"}"#,
            ),
            |state: &IdeState| state.project.is_some(),
            |state: &mut IdeState, params| {
                let scope = params
                    .get("scope")
                    .and_then(Value::as_str)
                    .unwrap_or("project");
                let files = state.run_inspection(scope).map_err(CommandError::Failed)?;
                Ok(json!({"scope": scope, "files": files}))
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "inspection.results",
                "Inspection Results",
                "Code",
                None,
                "{}",
            ),
            |_| true,
            |state: &mut IdeState, _| {
                state.poll_analysis();
                let service = state
                    .daemon_service()
                    .ok_or_else(|| CommandError::Failed("daemon unavailable".into()))?;
                let snap = state.analysis.host.snapshot();
                let batch = service.batch();
                Ok(json!({
                    "label": batch.label,
                    "running": batch.running,
                    "files": batch.summary.map(|s| s.files),
                    "problems": batch.problems.iter().map(|p| {
                        let (line, column) = snap
                            .file_text(p.file)
                            .map(|text| line_column(&text, p.problem.range.start as usize))
                            .unwrap_or((0, 0));
                        json!({
                            "path": snap.path(p.file),
                            "tool": p.tool,
                            "level": level_name(p.level),
                            "message": p.problem.message,
                            "line": line, "column": column,
                        })
                    }).collect::<Vec<_>>(),
                }))
            },
        ))
        .expect("unique built-in id");
}

/// 0-based line and character column of a byte offset.
pub(crate) fn line_column(text: &str, byte: usize) -> (usize, usize) {
    let byte = byte.min(text.len());
    let mut start = byte;
    while !text.is_char_boundary(start) {
        start -= 1;
    }
    let line = text[..start].matches('\n').count();
    let line_start = text[..start].rfind('\n').map_or(0, |i| i + 1);
    (line, text[line_start..start].chars().count())
}

#[cfg(test)]
mod tests {
    use crate::{Document, IdeState, built_in_commands};
    use archipel_ide_analysis::HostConfig;
    use serde_json::json;
    use std::path::Path;

    const JAVA: &str =
        "class A {\n  void f() {\n    try { g(); } catch (Exception e) { }\n  }\n}\n";

    fn state_with(path: &str, text: &str) -> IdeState {
        let mut state = IdeState {
            analysis: crate::AnalysisServices::new(HostConfig::testing()),
            ..IdeState::default()
        };
        state.documents.push(Document::new(path.into(), text));
        state.active_document = Some(0);
        state
    }

    fn settle(state: &mut IdeState) {
        // The daemon runs on the system clock: wait out the 300 ms debounce.
        for _ in 0..200 {
            state.poll_analysis();
            if state.daemon_service().is_some_and(|s| s.lock().all_idle()) {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }

    fn tools(state: &IdeState, file: &str) -> Vec<String> {
        let file = state
            .analysis
            .host
            .lookup(Path::new(file))
            .expect("file id");
        let service = state.daemon_service().expect("daemon");
        let daemon = service.lock();
        daemon
            .markup(file)
            .map(|m| m.iter().map(|i| i.tool.as_str().to_owned()).collect())
            .unwrap_or_default()
    }

    #[test]
    fn open_documents_are_analyzed_by_the_daemon() {
        let mut state = state_with("A.java", JAVA);
        settle(&mut state);
        assert!(
            tools(&state, "A.java").contains(&"native:EmptyCatchBlock".to_owned()),
            "{:?}",
            tools(&state, "A.java")
        );
        let status = built_in_commands()
            .execute("inspection.status", &mut state, json!({}))
            .expect("status");
        assert_eq!(status["available"], true);
        assert!(status["passes"].as_array().is_some_and(|p| {
            p.iter()
                .any(|p| p["id"] == "local-inspections" && p["runs"].as_u64() > Some(0))
        }));
        let list = built_in_commands()
            .execute("inspection.list", &mut state, json!({}))
            .expect("list");
        let tools = list["tools"].as_array().expect("tools");
        assert!(tools.iter().any(|t| t["id"] == "EmptyCatchBlock"
            && t["enabled"] == true
            && t["level"] == "warning"));
        assert!(
            tools
                .iter()
                .any(|t| t["id"] == "LongLine" && t["enabled"] == false)
        );
    }

    #[test]
    fn editing_a_profile_persists_and_reanalyzes() {
        let root = std::env::temp_dir().join(format!(
            "archipel-inspection-profile-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("A.java"), JAVA).unwrap();
        let mut state = IdeState {
            analysis: crate::AnalysisServices::new(HostConfig::testing()),
            ..IdeState::default()
        };
        state.open_project(&root).unwrap();
        state.open_file(Path::new("A.java")).unwrap();
        settle(&mut state);
        assert!(tools(&state, "A.java").contains(&"native:EmptyCatchBlock".to_owned()));
        built_in_commands()
            .execute(
                "inspection.profile.edit",
                &mut state,
                json!({"tool": "EmptyCatchBlock", "enabled": false}),
            )
            .expect("edit");
        settle(&mut state);
        assert!(!tools(&state, "A.java").contains(&"native:EmptyCatchBlock".to_owned()));
        let file = root.join(".archipel/inspection-profiles/Project Default.toml");
        let saved = std::fs::read_to_string(&file).expect("profile file");
        assert!(
            saved.contains("[tools.EmptyCatchBlock]") && saved.contains("enabled = false"),
            "{saved}"
        );
        // Level change on another tool; the listing reflects it.
        built_in_commands()
            .execute(
                "inspection.profile.edit",
                &mut state,
                json!({"tool": "UnreachableCode", "level": "error"}),
            )
            .expect("edit level");
        let list = built_in_commands()
            .execute("inspection.list", &mut state, json!({}))
            .unwrap();
        assert!(
            list["tools"]
                .as_array()
                .unwrap()
                .iter()
                .any(|t| t["id"] == "UnreachableCode" && t["level"] == "error")
        );
        assert!(
            built_in_commands()
                .execute(
                    "inspection.profile.edit",
                    &mut state,
                    json!({"tool": "X", "level": "loud"})
                )
                .is_err()
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn inspect_code_streams_results_for_file_and_project() {
        let root =
            std::env::temp_dir().join(format!("archipel-inspect-code-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("src/A.java"), JAVA).unwrap();
        std::fs::write(root.join("src/b.json"), "{\"a\":1,\"a\":2}").unwrap();
        std::fs::write(root.join("notes.txt"), "plain").unwrap();
        let mut state = IdeState {
            analysis: crate::AnalysisServices::new(HostConfig::testing()),
            ..IdeState::default()
        };
        state.open_project(&root).unwrap();
        let out = built_in_commands()
            .execute(
                "inspection.runCode",
                &mut state,
                json!({"scope": "project"}),
            )
            .expect("run");
        assert_eq!(out["files"], 2);
        for _ in 0..10 {
            state.poll_analysis();
        }
        let results = built_in_commands()
            .execute("inspection.results", &mut state, json!({}))
            .expect("results");
        assert_eq!(results["running"], false, "{results}");
        let problems = results["problems"].as_array().unwrap();
        let tools: Vec<_> = problems.iter().filter_map(|p| p["tool"].as_str()).collect();
        assert!(
            tools.contains(&"EmptyCatchBlock") && tools.contains(&"DuplicateKey"),
            "{tools:?}"
        );
        let catch = problems
            .iter()
            .find(|p| p["tool"] == "EmptyCatchBlock")
            .unwrap();
        assert_eq!(
            (catch["line"].as_u64(), catch["column"].as_u64()),
            (Some(2), Some(17))
        );
        state.open_file(Path::new("src/A.java")).unwrap();
        assert_eq!(
            built_in_commands()
                .execute("inspection.runCode", &mut state, json!({"scope": "file"}))
                .unwrap()["files"],
            1
        );
        assert!(
            built_in_commands()
                .execute("inspection.runCode", &mut state, json!({"scope": "galaxy"}))
                .is_err()
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn quick_fixes_apply_as_one_undo_step_and_are_ordered() {
        let src = "class A {\n  void f(int a) {\n    a = a;\n    int b = ((1));\n  }\n}\n";
        let mut state = state_with("A.java", src);
        settle(&mut state);
        let caret = src.find("a = a").unwrap() + 2;
        state.documents[0]
            .selections
            .set_single(archipel_ide_text::Selection::caret(caret));
        state.poll_analysis();
        let list = built_in_commands()
            .execute("code.alerts", &mut state, json!({}))
            .expect("alerts");
        let actions = list["actions"].as_array().expect("actions");
        let top: Vec<_> = actions
            .iter()
            .filter(|a| a["submenu"].is_null())
            .map(|a| a["text"].as_str().unwrap())
            .collect();
        // The self-assignment fix at the caret (inspection group, 20) comes before the
        // remote fix of the redundant parentheses on another line is not even listed.
        assert_eq!(top, ["Remove the assignment"], "{list}");
        assert!(
            actions
                .iter()
                .any(|a| a["submenu"] == "Inspection 'Self assignment'"
                    && a["text"] == "Suppress for statement")
        );
        let index = actions
            .iter()
            .position(|a| a["text"] == "Remove the assignment")
            .unwrap();
        built_in_commands()
            .execute("code.applyAlert", &mut state, json!({"index": index}))
            .expect("apply");
        assert_eq!(
            state.documents[0].text.text(),
            "class A {\n  void f(int a) {\n    int b = ((1));\n  }\n}\n"
        );
        // One undo step restores the file.
        assert!(state.documents[0].text.undo());
        assert_eq!(state.documents[0].text.text(), src);
        assert!(!state.documents[0].text.can_undo());
        // Suppress from the popup: the comment is inserted and silences the problem.
        let document = &mut state.documents[0];
        document.refresh_analysis(src);
        settle(&mut state);
        let caret = src.find("a = a").unwrap() + 2;
        state.documents[0]
            .selections
            .set_single(archipel_ide_text::Selection::caret(caret));
        state.poll_analysis();
        let list = built_in_commands()
            .execute("code.alerts", &mut state, json!({}))
            .unwrap();
        let actions = list["actions"].as_array().unwrap();
        let index = actions
            .iter()
            .position(|a| a["text"] == "Suppress for statement")
            .unwrap();
        built_in_commands()
            .execute("code.applyAlert", &mut state, json!({"index": index}))
            .unwrap();
        assert!(
            state.documents[0]
                .text
                .text()
                .contains("// noinspection SelfAssignment\n    a = a;")
        );
        settle(&mut state);
        assert!(!tools(&state, "A.java").contains(&"native:SelfAssignment".to_owned()));
    }

    #[test]
    fn stale_fixes_are_refused() {
        let src = "class A { void f(int a) { a = a; } }\n";
        let mut state = state_with("A.java", src);
        settle(&mut state);
        let file = state.analysis.host.lookup(Path::new("A.java")).unwrap();
        let command = archipel_ide_analysis::ModCommand::UpdateFile {
            file,
            edits: vec![archipel_ide_analysis::TextEdit::insert(0, "x")],
            expected_rev: archipel_ide_analysis::FileRev(999),
        };
        let error = state.execute_mod_command(&command).unwrap_err();
        assert!(error.contains("changed since"), "{error}");
        assert!(
            state
                .execute_mod_command(&archipel_ide_analysis::ModCommand::CreateFile {
                    path: "x".into(),
                    text: String::new()
                })
                .is_err()
        );
    }

    #[test]
    fn line_columns_count_characters() {
        assert_eq!(super::line_column("a\n日本x", 8), (1, 2));
        assert_eq!(super::line_column("abc", 99), (0, 3));
    }
}
