//! Assembling and applying the Alt+Enter list (`CachedIntentions` analogue).
//!
//! Sources, in the order of the spec:
//! 1. fixes attached to the highlights of the caret line: the **Error**
//!    group for errors at the caret, **Inspection** for lesser problems,
//!    **RemoteError** for fixes of problems elsewhere on the line;
//! 2. every available [`IntentionAction`] at the caret/selection (**Other**);
//! 3. actions of registered [`AlertSource`]s (the LSP adapter of WP8) and
//!    any `extra` the caller passes (the existing LSP code-action list).
//!
//! Generic wrappers hang off every inspection problem at the caret: *Suppress
//! for statement/declaration/file*, *Fix all ... in file*, *Disable
//! inspection*, *Edit inspection profile*.
//!
//! Rows are deduplicated by (family, text) with native actions preferred,
//! then sorted by `group priority + priority weight` descending, ties by
//! text ([`sort_actions`]). The result is cached per (file revision, caret,
//! selection, markup version).

use crate::inspection::suppress::{SuppressScope, suppress_command};
use crate::markup::FileMarkup;
use archipel_ide_analysis::highlight::FixDescriptor;
use archipel_ide_analysis::inspect::{ProblemDescriptor, ProblemHighlightType};
use archipel_ide_analysis::intention::{
    ActionCx, FixCx, IntentionAction, IntentionGroup, Priority, QuickFix, sort_actions,
};
use archipel_ide_analysis::{
    CancelToken, Cancelled, FileId, FileRev, HighlightInfo, ModCommand, Severity, Snapshot,
    SyntaxSnapshot, TextEdit,
};
use std::collections::{HashMap, HashSet};
use std::ops::Range;
use std::sync::Arc;

/// Who offers an action (native wins a dedupe).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Source {
    Native,
    Lsp,
}

/// What choosing an action does.
#[derive(Clone)]
pub enum ActionKind {
    /// A quick fix of a highlight (`highlight` is its current range/message).
    Fix {
        range: Range<u32>,
        message: String,
        tool: String,
        fix: Arc<dyn QuickFix>,
    },
    /// Apply the fix to every highlight of the same tool and family.
    FixAll {
        tool: String,
        family: String,
    },
    Intention(Arc<dyn IntentionAction>),
    Suppress {
        tool: String,
        scope: SuppressScope,
        range: Range<u32>,
    },
    DisableInspection {
        tool: String,
    },
    EditProfile {
        tool: String,
    },
    /// An action owned by another provider; `token` is opaque to the daemon.
    External {
        token: String,
    },
}

impl std::fmt::Debug for ActionKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ActionKind::Fix { tool, .. } => write!(f, "Fix({tool})"),
            ActionKind::FixAll { tool, family } => write!(f, "FixAll({tool}, {family})"),
            ActionKind::Intention(action) => write!(f, "Intention({})", action.id()),
            ActionKind::Suppress { tool, scope, .. } => write!(f, "Suppress({tool}, {scope:?})"),
            ActionKind::DisableInspection { tool } => write!(f, "Disable({tool})"),
            ActionKind::EditProfile { tool } => write!(f, "EditProfile({tool})"),
            ActionKind::External { token } => write!(f, "External({token})"),
        }
    }
}

/// One row of the popup.
#[derive(Debug, Clone)]
pub struct AlertAction {
    pub text: String,
    pub family: String,
    pub group: IntentionGroup,
    pub priority: Priority,
    /// Shown inside a submenu with this title (e.g. an inspection's options).
    pub submenu: Option<String>,
    pub source: Source,
    pub kind: ActionKind,
}

impl AlertAction {
    /// An action owned by another provider (the LSP adapter): `token` comes
    /// back to its owner when the action is chosen.
    pub fn external(
        text: impl Into<String>,
        group: IntentionGroup,
        priority: Priority,
        source: Source,
        token: impl Into<String>,
    ) -> Self {
        let text = text.into();
        Self {
            family: text.clone(),
            text,
            group,
            priority,
            submenu: None,
            source,
            kind: ActionKind::External {
                token: token.into(),
            },
        }
    }

    /// `group + priority` weight used for ordering.
    pub fn weight(&self) -> i32 {
        self.group.priority() + self.priority.weight()
    }
}

/// The bulb of the gutter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bulb {
    /// Nothing to offer.
    None,
    /// Intentions only (yellow).
    Intention,
    /// At least one fix of an error (red).
    ErrorFix,
}

/// The assembled popup.
#[derive(Debug, Clone, Default)]
pub struct AlertSet {
    pub actions: Vec<AlertAction>,
}

impl AlertSet {
    pub fn bulb(&self) -> Bulb {
        if self.actions.is_empty() {
            Bulb::None
        } else if self.actions.iter().any(|a| {
            matches!(a.group, IntentionGroup::Error | IntentionGroup::RemoteError)
                && a.submenu.is_none()
        }) {
            Bulb::ErrorFix
        } else {
            Bulb::Intention
        }
    }

    /// Top-level rows (no submenu), in order.
    pub fn top_level(&self) -> impl Iterator<Item = &AlertAction> {
        self.actions.iter().filter(|a| a.submenu.is_none())
    }

    /// The rows of the submenu `title`.
    pub fn submenu<'a>(&'a self, title: &'a str) -> impl Iterator<Item = &'a AlertAction> {
        self.actions
            .iter()
            .filter(move |a| a.submenu.as_deref() == Some(title))
    }

    pub fn texts(&self) -> Vec<&str> {
        self.actions.iter().map(|a| a.text.as_str()).collect()
    }
}

/// Provides actions of another origin (WP8's LSP adapter registers one,
/// `Order::Last`).
pub trait AlertSource: Send + Sync {
    fn actions(&self, cx: &ActionCx<'_>, markup: &FileMarkup) -> Vec<AlertAction>;
}

/// Everything the assembly needs.
pub struct AlertInput<'a> {
    pub snap: &'a Snapshot,
    pub file: FileId,
    pub syntax: &'a SyntaxSnapshot,
    pub markup: &'a FileMarkup,
    pub caret: u32,
    pub selection: Option<Range<u32>>,
    pub cancel: &'a CancelToken,
    /// Actions of the caller (the existing LSP list, converted).
    pub extra: Vec<AlertAction>,
    /// Intention ids switched off in the settings.
    pub disabled: &'a HashSet<String>,
    /// Display names of inspections by id (for submenu titles).
    pub tool_names: &'a HashMap<String, String>,
}

fn line_range(text: &str, offset: u32) -> Range<u32> {
    let at = (offset as usize).min(text.len());
    let start = text[..at].rfind('\n').map_or(0, |i| i + 1);
    let end = text[at..].find('\n').map_or(text.len(), |i| at + i);
    start as u32..end as u32
}

fn tool_id(info: &HighlightInfo) -> Option<&str> {
    info.tool.as_str().strip_prefix("native:")
}

fn fix_row(
    info: &HighlightInfo,
    fix: &FixDescriptor,
    group: IntentionGroup,
    tool: &str,
) -> Option<AlertAction> {
    let action = fix.action.clone()?;
    Some(AlertAction {
        text: fix.text.to_string(),
        family: fix.family.to_string(),
        group,
        priority: Priority::Normal,
        submenu: None,
        source: Source::Native,
        kind: ActionKind::Fix {
            range: info.range.clone(),
            message: info.message.as_deref().unwrap_or("").to_owned(),
            tool: tool.to_owned(),
            fix: action,
        },
    })
}

/// Builds the popup content; see the module docs.
pub fn assemble(input: &AlertInput<'_>) -> Result<AlertSet, Cancelled> {
    let text = input.syntax.text();
    let lang = input.syntax.lang().language_id();
    let dumb = input.snap.is_dumb();
    let line = line_range(text, input.caret);
    let mut rows: Vec<AlertAction> = Vec::new();
    let mut wrapped: HashSet<(String, Option<String>)> = HashSet::new();

    // (a) fixes of the highlights on the caret line.
    for (_, _, info) in input.markup.query_with_ids(line.clone()) {
        input.cancel.check()?;
        let at_caret = info.file_level
            || (info.range.start <= input.caret && input.caret <= info.range.end)
            || input
                .selection
                .as_ref()
                .is_some_and(|s| info.range.start < s.end && s.start < info.range.end);
        let Some(tool) = tool_id(info) else { continue };
        let group = if !at_caret {
            IntentionGroup::RemoteError
        } else if info.severity >= Severity::ERROR {
            IntentionGroup::Error
        } else {
            IntentionGroup::Inspection
        };
        for fix in &info.fixes {
            rows.extend(fix_row(info, fix, group, tool));
        }
        let Some(name) = input.tool_names.get(tool) else {
            continue;
        };
        if !at_caret
            || !wrapped.insert((
                tool.to_owned(),
                Some(format!("{}:{}", info.range.start, info.range.end)),
            ))
        {
            continue;
        }
        let submenu = format!("Inspection '{name}'");
        let sub = |text: String, kind: ActionKind| AlertAction {
            family: text.clone(),
            text,
            group: IntentionGroup::Inspection,
            priority: Priority::Low,
            submenu: Some(submenu.clone()),
            source: Source::Native,
            kind,
        };
        for fix in &info.fixes {
            let same = input
                .markup
                .iter()
                .filter(|other| {
                    tool_id(other) == Some(tool)
                        && other.fixes.iter().any(|f| f.family == fix.family)
                })
                .count();
            if same > 1 && fix.action.as_ref().is_some_and(|a| a.batch_ok()) {
                rows.push(sub(
                    format!("Fix all '{}' problems in file", fix.family),
                    ActionKind::FixAll {
                        tool: tool.to_owned(),
                        family: fix.family.to_string(),
                    },
                ));
            }
        }
        for (scope, label) in [
            (SuppressScope::Statement, "Suppress for statement"),
            (SuppressScope::Declaration, "Suppress for declaration"),
            (SuppressScope::File, "Suppress for file"),
        ] {
            if crate::inspection::suppress::suppress_edit(input.syntax, &info.range, tool, scope)
                .is_some()
            {
                rows.push(sub(
                    label.to_owned(),
                    ActionKind::Suppress {
                        tool: tool.to_owned(),
                        scope,
                        range: info.range.clone(),
                    },
                ));
            }
        }
        rows.push(sub(
            format!("Disable inspection '{name}'"),
            ActionKind::DisableInspection {
                tool: tool.to_owned(),
            },
        ));
        rows.push(sub(
            "Edit inspection profile settings".into(),
            ActionKind::EditProfile {
                tool: tool.to_owned(),
            },
        ));
    }

    // (b) intentions at the caret/selection.
    let cx = ActionCx {
        snap: input.snap,
        file: input.file,
        syntax: input.syntax,
        caret: input.caret,
        selection: input.selection.clone(),
        cancel: input.cancel,
    };
    let intentions: Vec<Arc<dyn IntentionAction>> = input.snap.registry().all_available(lang, dumb);
    for action in intentions {
        input.cancel.check()?;
        if input.disabled.contains(action.id()) || (dumb && !action.dumb_aware()) {
            continue;
        }
        if let Some(presentation) = action.presentation(&cx) {
            rows.push(AlertAction {
                text: presentation.text,
                family: action.family().to_owned(),
                group: IntentionGroup::Other,
                priority: action.priority(),
                submenu: None,
                source: Source::Native,
                kind: ActionKind::Intention(action),
            });
        }
    }

    // (c) other providers.
    let sources: Vec<Arc<dyn AlertSource>> = input.snap.registry().all_available(lang, dumb);
    for source in sources {
        input.cancel.check()?;
        rows.extend(source.actions(&cx, input.markup));
    }
    rows.extend(input.extra.iter().cloned());

    Ok(AlertSet {
        actions: finish(rows),
    })
}

/// Dedupes by (family, text, submenu) preferring native rows, then sorts.
fn finish(mut rows: Vec<AlertAction>) -> Vec<AlertAction> {
    rows.sort_by_key(|row| row.source);
    let mut seen: HashSet<(String, String, Option<String>)> = HashSet::new();
    rows.retain(|row| seen.insert((row.family.clone(), row.text.clone(), row.submenu.clone())));
    let mut keyed: Vec<(IntentionGroup, Priority, String, AlertAction)> = rows
        .into_iter()
        .map(|row| (row.group, row.priority, row.text.clone(), row))
        .collect();
    sort_actions(&mut keyed);
    keyed.into_iter().map(|(_, _, _, row)| row).collect()
}

/// What choosing an action asks the host to do.
#[derive(Debug, Clone, PartialEq)]
pub enum Applied {
    Command(ModCommand),
    /// Turn the inspection off in the active profile.
    DisableInspection {
        tool: String,
    },
    /// Open the settings page at the inspection.
    EditProfile {
        tool: String,
    },
    /// The owner of `token` performs it.
    External {
        token: String,
    },
}

/// Inputs to [`apply`].
pub struct ApplyInput<'a> {
    pub snap: &'a Snapshot,
    pub file: FileId,
    pub syntax: &'a SyntaxSnapshot,
    pub markup: &'a FileMarkup,
    pub caret: u32,
    pub selection: Option<Range<u32>>,
    pub cancel: &'a CancelToken,
}

fn descriptor(range: Range<u32>, message: &str) -> ProblemDescriptor {
    ProblemDescriptor {
        range,
        message: message.to_owned(),
        highlight: ProblemHighlightType::GenericErrorOrWarning,
        fixes: Vec::new(),
        on_the_fly_only: false,
    }
}

/// Performs `action`: pure, returns the command to execute.
pub fn apply(action: &AlertAction, input: &ApplyInput<'_>) -> Result<Applied, Cancelled> {
    let fix_cx = FixCx {
        snap: input.snap,
        file: input.file,
        syntax: input.syntax,
        cancel: input.cancel,
    };
    Ok(match &action.kind {
        ActionKind::Fix {
            range,
            message,
            fix,
            ..
        } => Applied::Command(fix.fix(&descriptor(range.clone(), message), &fix_cx)?),
        ActionKind::FixAll { tool, family } => {
            Applied::Command(fix_all_command(input, tool, family)?)
        }
        ActionKind::Intention(intention) => {
            let cx = ActionCx {
                snap: input.snap,
                file: input.file,
                syntax: input.syntax,
                caret: input.caret,
                selection: input.selection.clone(),
                cancel: input.cancel,
            };
            Applied::Command(intention.perform(&cx)?)
        }
        ActionKind::Suppress { tool, scope, range } => Applied::Command(
            suppress_command(input.file, input.syntax, range, tool, *scope)
                .unwrap_or(ModCommand::Nothing),
        ),
        ActionKind::DisableInspection { tool } => Applied::DisableInspection { tool: tool.clone() },
        ActionKind::EditProfile { tool } => Applied::EditProfile { tool: tool.clone() },
        ActionKind::External { token } => Applied::External {
            token: token.clone(),
        },
    })
}

/// "Fix all in file": the fix of every highlight of `tool` with a fix of
/// `family`, merged into one `UpdateFile` (overlapping edits are skipped).
pub fn fix_all_command(
    input: &ApplyInput<'_>,
    tool: &str,
    family: &str,
) -> Result<ModCommand, Cancelled> {
    let fix_cx = FixCx {
        snap: input.snap,
        file: input.file,
        syntax: input.syntax,
        cancel: input.cancel,
    };
    let mut edits: Vec<TextEdit> = Vec::new();
    let mut taken: Vec<Range<u32>> = Vec::new();
    let mut rev: Option<FileRev> = None;
    for info in input.markup.iter() {
        input.cancel.check()?;
        if tool_id(info) != Some(tool) {
            continue;
        }
        for fix in info.fixes.iter().filter(|f| &*f.family == family) {
            let Some(action) = &fix.action else { continue };
            if !action.batch_ok() {
                continue;
            }
            let command = action.fix(
                &descriptor(info.range.clone(), info.message.as_deref().unwrap_or("")),
                &fix_cx,
            )?;
            if let ModCommand::UpdateFile {
                file,
                edits: new,
                expected_rev,
            } = command
                && file == input.file
                && !new.iter().any(|e| {
                    taken
                        .iter()
                        .any(|t| e.range.start < t.end && t.start < e.range.end)
                })
            {
                rev = Some(expected_rev);
                taken.extend(new.iter().map(|e| e.range.clone()));
                edits.extend(new);
            }
        }
    }
    Ok(match rev {
        Some(expected_rev) if !edits.is_empty() => ModCommand::UpdateFile {
            file: input.file,
            edits,
            expected_rev,
        },
        _ => ModCommand::Nothing,
    })
}

/// Cache of the last assembled set per file.
#[derive(Debug, Default)]
pub struct AlertCache {
    entries: HashMap<FileId, (CacheKey, Arc<AlertSet>)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CacheKey {
    rev: FileRev,
    caret: u32,
    selection: Option<Range<u32>>,
    markup_version: u64,
    extra: usize,
}

impl AlertCache {
    /// The cached set for exactly this state, else `compute`d and stored.
    pub fn get_or_compute(
        &mut self,
        input: &AlertInput<'_>,
        markup_version: u64,
    ) -> Result<Arc<AlertSet>, Cancelled> {
        let key = CacheKey {
            rev: input.syntax.rev(),
            caret: input.caret,
            selection: input.selection.clone(),
            markup_version,
            extra: input.extra.len(),
        };
        if let Some((cached, set)) = self.entries.get(&input.file)
            && *cached == key
        {
            return Ok(set.clone());
        }
        let set = Arc::new(assemble(input)?);
        self.entries.insert(input.file, (key, set.clone()));
        Ok(set)
    }

    pub fn forget(&mut self, file: FileId) {
        self.entries.remove(&file);
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}
