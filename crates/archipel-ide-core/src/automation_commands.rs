//! Read-only queries for automation (`archipel_ide_control`, `cargo xtask scenario`).
//!
//! They expose what the editor itself shows, in IntelliJ's vocabulary and with 1-based
//! positions (the status bar's `line:column`): the markup of the active file, the
//! completion popup rows and the text of project files. They never change state, so a
//! scenario observes the IDE exactly as a person would, through the same models the UI
//! paints.
use crate::{IdeState, spec, string_param};
use archipel_ide_command::{Command, CommandError, CommandRegistry};
use serde_json::{Value, json};

impl IdeState {
    /// Whether the active document's markup is final for its current text: the native daemon
    /// has analyzed this revision and the language server (if any) is ready and quiet.
    /// Returns `(native_ready, lsp_pending)`.
    pub fn analysis_readiness(&self) -> (bool, bool) {
        let Some(document) = self.active_document.and_then(|i| self.documents.get(i)) else {
            return (true, false);
        };
        let native_ready = self.daemon_service().is_none()
            || self.native_problems(document).is_some()
            || self.analysis.host.lookup(&document.path).is_none();
        let lsp_pending = self.active_absolute_path().is_some_and(|path| {
            self.lsp.handles(&path)
                && !self.lsp.is_idle_for(&path)
                && self.lsp.missing_servers().next().is_none()
        });
        (native_ready, lsp_pending)
    }

    /// A value that changes whenever the editor's markup or its inputs change.
    pub fn markup_stamp(&self) -> (u64, u64, u64) {
        let revision = self
            .active_document
            .and_then(|i| self.documents.get(i))
            .map_or(0, |document| document.text.revision());
        (
            revision,
            self.lsp.diagnostics_generation(),
            self.daemon_version(),
        )
    }
}

fn severity_name(severity: crate::problems::Severity) -> &'static str {
    use crate::problems::Severity;
    match severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::WeakWarning => "weak_warning",
        Severity::Hint => "hint",
    }
}

pub(crate) fn register(registry: &mut CommandRegistry<IdeState>) {
    registry
        .register(Command::new(
            spec(
                "automation.markup",
                "Automation: Markup of the Active File",
                "Automation",
                None,
                "{}",
            ),
            |state: &IdeState| state.active_document.is_some(),
            |state: &mut IdeState, _| {
                state.poll_analysis();
                let index = state
                    .active_document
                    .ok_or_else(|| CommandError::Failed("no active document".into()))?;
                let (native_ready, lsp_pending) = state.analysis_readiness();
                let problems = state.problems_for_document(index);
                let items: Vec<Value> = problems
                    .iter()
                    .map(|problem| {
                        json!({
                            "severity": severity_name(problem.severity),
                            "message": problem.message,
                            "source": problem.source,
                            "code": problem.code,
                            // 1-based; the end column is exclusive.
                            "range": {
                                "line": problem.line + 1,
                                "column": problem.column + 1,
                                "endLine": problem.end_line + 1,
                                "endColumn": problem.end_column + 1,
                            },
                            "unnecessary": problem.unnecessary,
                            "deprecated": problem.deprecated,
                        })
                    })
                    .collect();
                Ok(json!({
                    "path": state.documents[index].path,
                    "revision": state.documents[index].text.revision(),
                    "nativeReady": native_ready,
                    "lspPending": lsp_pending,
                    "markup": items,
                }))
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "automation.completion",
                "Automation: Completion Popup Rows",
                "Automation",
                None,
                "{}",
            ),
            |_| true,
            |state: &mut IdeState, _| {
                let Some(session) = state.completion.as_ref() else {
                    return Ok(json!({"open": false, "items": []}));
                };
                let items: Vec<Value> = session
                    .ranked
                    .iter()
                    .filter_map(|ranked| session.items.get(ranked.index))
                    .map(|item| {
                        json!({
                            "label": item.label,
                            "detail": item.detail,
                            "kind": item.kind.map(|kind| format!("{kind:?}")),
                        })
                    })
                    .collect();
                Ok(json!({
                    "open": session.visible(),
                    "prefix": session.prefix,
                    "selected": session.selected,
                    "indexing": session.indexing,
                    "items": items,
                }))
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "automation.documentText",
                "Automation: Text of a Project File",
                "Automation",
                None,
                r#"{"path":"string"}"#,
            ),
            |state: &IdeState| state.project.is_some(),
            |state: &mut IdeState, params| {
                let relative = string_param(&params, "path")?;
                let root = state
                    .project
                    .as_ref()
                    .ok_or_else(|| CommandError::Failed("no project".into()))?
                    .root()
                    .to_owned();
                let path = std::path::Path::new(&relative);
                if path.is_absolute()
                    || path
                        .components()
                        .any(|part| matches!(part, std::path::Component::ParentDir))
                {
                    return Err(CommandError::InvalidParams(
                        "path must be project-relative".into(),
                    ));
                }
                // The editor's text wins over disk: an applied fix may be unsaved.
                if let Some(document) = state.documents.iter().find(|d| d.path == path) {
                    return Ok(json!({
                        "text": document.text.text(),
                        "open": true,
                        "modified": document.text.is_dirty(),
                    }));
                }
                let text = std::fs::read_to_string(root.join(path))
                    .map_err(|error| CommandError::Failed(format!("{relative}: {error}")))?;
                Ok(json!({"text": text, "open": false, "modified": false}))
            },
        ))
        .expect("unique built-in id");
}

#[cfg(test)]
mod tests {
    use crate::{IdeState, built_in_commands};
    use serde_json::json;

    #[test]
    fn queries_report_markup_completion_and_text() {
        let root =
            std::env::temp_dir().join(format!("archipel-ide-automation-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("a.txt"), "hello\n").unwrap();
        let commands = built_in_commands();
        let mut state = IdeState::default();
        commands
            .execute("project.open", &mut state, json!({"path": root}))
            .unwrap();
        commands
            .execute("editor.openFile", &mut state, json!({"path": "a.txt"}))
            .unwrap();
        let markup = commands
            .execute("automation.markup", &mut state, json!({}))
            .unwrap();
        assert_eq!(markup["path"], "a.txt");
        assert_eq!(markup["markup"], json!([]));
        let completion = commands
            .execute("automation.completion", &mut state, json!({}))
            .unwrap();
        assert_eq!(completion["open"], false);
        let text = commands
            .execute(
                "automation.documentText",
                &mut state,
                json!({"path": "a.txt"}),
            )
            .unwrap();
        assert_eq!(text["text"], "hello\n");
        assert_eq!(text["open"], true);
        assert!(
            commands
                .execute(
                    "automation.documentText",
                    &mut state,
                    json!({"path": "../etc/passwd"}),
                )
                .is_err()
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}
