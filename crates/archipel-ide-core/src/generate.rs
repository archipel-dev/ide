//! The Generate popup (Alt+Insert / ⌘N): constructor, getters and setters,
//! `equals()`/`hashCode()`, `toString()` and Override Methods.
//!
//! The entries are the `source.generate.*` / `source.overrideMethods` code
//! actions of the language server (jdtls offers them for Java). The popup
//! lists every Generate entry; the ones the server does not offer at the caret
//! are greyed out with a reason. Choosing one runs the server's action through
//! the same path as an intention, so edits (or `workspace/applyEdit`) apply as
//! one undo step.
use crate::code_actions::{CodeActionItem, CodeActionList, file_uri};
use crate::{IdeState, spec};
use archipel_ide_command::{Command, CommandError, CommandRegistry};
use archipel_ide_lsp::{Feature, RequestHandle};
use serde::Serialize;
use serde_json::{Value, json};
use std::path::PathBuf;

/// An entry of the Generate popup.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum GenerateKind {
    Constructor,
    Getter,
    Setter,
    GetterAndSetter,
    EqualsHashCode,
    ToString,
    OverrideMethods,
}

impl GenerateKind {
    pub const ALL: [GenerateKind; 7] = [
        GenerateKind::Constructor,
        GenerateKind::Getter,
        GenerateKind::Setter,
        GenerateKind::GetterAndSetter,
        GenerateKind::EqualsHashCode,
        GenerateKind::ToString,
        GenerateKind::OverrideMethods,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Constructor => "Constructor",
            Self::Getter => "Getter",
            Self::Setter => "Setter",
            Self::GetterAndSetter => "Getter and Setter",
            Self::EqualsHashCode => "equals() and hashCode()",
            Self::ToString => "toString()",
            Self::OverrideMethods => "Override Methods…",
        }
    }

    pub fn command(self) -> &'static str {
        match self {
            Self::Constructor => "code.generate.constructor",
            Self::Getter => "code.generate.getter",
            Self::Setter => "code.generate.setter",
            Self::GetterAndSetter => "code.generate.getterAndSetter",
            Self::EqualsHashCode => "code.generate.equalsHashCode",
            Self::ToString => "code.generate.toString",
            Self::OverrideMethods => "code.generate.overrideMethods",
        }
    }

    /// Whether the server's code action is this entry.
    fn matches(self, item: &CodeActionItem) -> bool {
        let Some(kind) = item.kind.as_deref() else {
            return false;
        };
        let title = item.title.to_lowercase();
        let getters = title.contains("getter");
        let setters = title.contains("setter");
        match self {
            Self::Constructor => kind.starts_with("source.generate.constructor"),
            Self::Getter => kind.starts_with("source.generate.accessors") && getters && !setters,
            Self::Setter => kind.starts_with("source.generate.accessors") && setters && !getters,
            Self::GetterAndSetter => {
                kind.starts_with("source.generate.accessors") && getters && setters
            }
            Self::EqualsHashCode => kind.starts_with("source.generate.hashCodeEquals"),
            Self::ToString => kind.starts_with("source.generate.toString"),
            Self::OverrideMethods => kind.starts_with("source.overrideMethods"),
        }
    }
}

#[derive(Debug)]
struct Job {
    handle: RequestHandle,
    /// `None` opens the popup; `Some` runs that entry directly.
    kind: Option<GenerateKind>,
    path: PathBuf,
    offset: usize,
    revision: u64,
}

#[derive(Debug, Default)]
pub struct GenerateState {
    jobs: Vec<Job>,
}

impl IdeState {
    /// Alt+Insert: asks the server for its source actions at the caret and
    /// opens the Generate popup (`kind == None`), or runs one entry directly.
    pub fn request_generate(&mut self, kind: Option<GenerateKind>) -> Result<(), String> {
        let index = self.active_document.ok_or("no active document")?;
        let absolute = self.active_absolute_path().ok_or("no project is open")?;
        if !self.lsp.supports(&absolute, Feature::CodeAction) {
            return Err("The language server does not support Generate".into());
        }
        let document = &self.documents[index];
        let selection = document.selections.primary();
        let position = |offset| {
            document
                .text
                .lsp_position(offset)
                .map(|(line, character)| json!({"line": line, "character": character}))
                .map_err(|error| error.to_string())
        };
        let params = json!({
            "textDocument": {"uri": file_uri(&absolute)?},
            "range": {"start": position(selection.start())?, "end": position(selection.end())?},
            "context": {"diagnostics": [], "only": ["source"], "triggerKind": 1}
        });
        let job = (
            document.path.clone(),
            selection.head,
            document.text.revision(),
        );
        let handle = self
            .lsp
            .request_with_params(&absolute, Feature::CodeAction, params)
            .map_err(|error| error.to_string())?;
        self.generate.jobs.push(Job {
            handle,
            kind,
            path: job.0,
            offset: job.1,
            revision: job.2,
        });
        Ok(())
    }

    /// Finishes Generate requests whose answers arrived.
    pub fn poll_generate(&mut self) {
        let jobs = std::mem::take(&mut self.generate.jobs);
        for job in jobs {
            match self.lsp.take_response(&job.handle) {
                None => self.generate.jobs.push(job),
                Some(Ok(value)) => self.finish_generate(job, &value),
                Some(Err(error)) => self.status = error.to_string(),
            }
        }
    }

    fn finish_generate(&mut self, job: Job, value: &Value) {
        let fresh = self
            .documents
            .iter()
            .find(|document| document.path == job.path)
            .is_some_and(|document| document.text.revision() == job.revision);
        if !fresh {
            self.status = "Generate: the text changed; try again".into();
            return;
        }
        let offered: Vec<CodeActionItem> = value
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(CodeActionItem::parse)
            .collect();
        let entry = |kind: GenerateKind| -> CodeActionItem {
            let found = offered
                .iter()
                .find(|item| kind.matches(item) && item.disabled.is_none())
                .or_else(|| offered.iter().find(|item| kind.matches(item)));
            match found {
                Some(item) => CodeActionItem {
                    title: kind.label().into(),
                    ..item.clone()
                },
                None => CodeActionItem {
                    title: kind.label().into(),
                    kind: None,
                    quick_fix: false,
                    preferred: false,
                    disabled: Some("not offered by the language server here".into()),
                    raw: Value::Null,
                },
            }
        };
        if let Some(kind) = job.kind {
            let item = entry(kind);
            if let Some(reason) = &item.disabled {
                self.status = format!("{}: {reason}", kind.label());
            } else if let Err(error) = self.run_code_action(&item.raw, &job.path) {
                self.status = error;
            }
            return;
        }
        let items: Vec<CodeActionItem> = GenerateKind::ALL.into_iter().map(entry).collect();
        if items.iter().all(|item| item.disabled.is_some()) {
            self.status = "Nothing to generate here".into();
            return;
        }
        self.code_actions.list = Some(CodeActionList {
            path: job.path.clone(),
            offset: job.offset,
            line: self
                .documents
                .iter()
                .find(|document| document.path == job.path)
                .map_or(0, |document| document.text.char_to_line(job.offset)),
            revision: job.revision,
            diagnostics: self.lsp.diagnostics_generation(),
            items,
        });
        self.code_actions.popup = true;
    }
}

pub(crate) fn register(registry: &mut CommandRegistry<IdeState>) {
    let ready = |state: &IdeState| {
        state
            .active_absolute_path()
            .is_some_and(|path| state.lsp.supports(&path, Feature::CodeAction))
    };
    registry
        .register(Command::new(
            spec("code.generate", "Generate…", "Code", Some("⌘N"), "{}"),
            ready,
            |state: &mut IdeState, _| {
                state.request_generate(None).map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    for kind in GenerateKind::ALL {
        registry
            .register(Command::new(
                spec(kind.command(), kind.label(), "Generate", None, "{}"),
                ready,
                move |state: &mut IdeState, _| {
                    state
                        .request_generate(Some(kind))
                        .map_err(CommandError::Failed)?;
                    Ok(Value::Null)
                },
            ))
            .expect("unique built-in id");
    }
}
