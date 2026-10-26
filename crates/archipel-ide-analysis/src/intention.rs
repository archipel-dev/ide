//! Intention and quick-fix contracts (implemented by WP3, WP7 and language
//! crates) plus the Alt+Enter ordering formula.

use crate::cancel::{CancelToken, Cancelled};
use crate::file::FileId;
use crate::host::Snapshot;
use crate::inspect::{LanguageFilter, ProblemDescriptor};
use crate::modcommand::ModCommand;
use archipel_ide_language::SyntaxSnapshot;
use std::ops::Range;

/// `PriorityAction.Priority`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Priority {
    Top,
    High,
    Normal,
    Low,
    Bottom,
}

impl Priority {
    /// Weight added to the group priority.
    pub fn weight(self) -> i32 {
        match self {
            Priority::Top => 20,
            Priority::High => 3,
            Priority::Normal => 0,
            Priority::Low => -3,
            Priority::Bottom => -20,
        }
    }
}

/// Popup groups of the Alt+Enter list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IntentionGroup {
    Error,
    Inspection,
    /// Fixes of problems elsewhere on the caret line.
    RemoteError,
    Notification,
    Other,
    Gutter,
    Advertisement,
    EmptyAction,
}

impl IntentionGroup {
    pub fn priority(self) -> i32 {
        match self {
            IntentionGroup::Error => 30,
            IntentionGroup::Inspection => 20,
            IntentionGroup::RemoteError => 10,
            IntentionGroup::Notification => 7,
            IntentionGroup::Other => 0,
            IntentionGroup::Gutter => -5,
            IntentionGroup::Advertisement | IntentionGroup::EmptyAction => -10,
        }
    }
}

/// Sorts `(group, priority, text)` rows: weight descending, then text.
pub fn sort_actions<T>(rows: &mut [(IntentionGroup, Priority, String, T)]) {
    rows.sort_by(|a, b| {
        let weight =
            |row: &(IntentionGroup, Priority, String, T)| row.0.priority() + row.1.weight();
        weight(b).cmp(&weight(a)).then_with(|| a.2.cmp(&b.2))
    });
}

/// What the popup shows for an available action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Presentation {
    pub text: String,
}

/// Context of an intention at the caret/selection.
pub struct ActionCx<'a> {
    pub snap: &'a Snapshot,
    pub file: FileId,
    pub syntax: &'a SyntaxSnapshot,
    pub caret: u32,
    pub selection: Option<Range<u32>>,
    pub cancel: &'a CancelToken,
}

/// An intention (`IntentionAction`/`ModCommandAction`).
pub trait IntentionAction: Send + Sync {
    fn family(&self) -> &'static str;
    /// Stable id for enable/disable settings.
    fn id(&self) -> &'static str;
    fn languages(&self) -> LanguageFilter;
    fn dumb_aware(&self) -> bool;
    fn priority(&self) -> Priority {
        Priority::Normal
    }
    /// `None` = not available here (isAvailable + text).
    fn presentation(&self, cx: &ActionCx<'_>) -> Option<Presentation>;
    /// Pure: returns a command, mutates nothing.
    fn perform(&self, cx: &ActionCx<'_>) -> Result<ModCommand, Cancelled>;
}

/// Context of a quick fix.
pub struct FixCx<'a> {
    pub snap: &'a Snapshot,
    pub file: FileId,
    pub syntax: &'a SyntaxSnapshot,
    pub cancel: &'a CancelToken,
}

/// A fix attached to a problem (`LocalQuickFix`).
pub trait QuickFix: Send + Sync {
    fn family(&self) -> &str;
    fn name(&self) -> String;
    fn fix(&self, d: &ProblemDescriptor, cx: &FixCx<'_>) -> Result<ModCommand, Cancelled>;
    /// Usable in "fix all".
    fn batch_ok(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alt_enter_order_follows_the_weight_formula() {
        let mut rows = vec![
            (
                IntentionGroup::Other,
                Priority::Normal,
                "b intention".to_string(),
                (),
            ),
            (
                IntentionGroup::Error,
                Priority::Low,
                "fix error".to_string(),
                (),
            ),
            (
                IntentionGroup::Other,
                Priority::Top,
                "top intention".to_string(),
                (),
            ),
            (
                IntentionGroup::Inspection,
                Priority::Normal,
                "inspection".to_string(),
                (),
            ),
            (
                IntentionGroup::Other,
                Priority::Normal,
                "a intention".to_string(),
                (),
            ),
        ];
        sort_actions(&mut rows);
        let texts: Vec<_> = rows.iter().map(|row| row.2.as_str()).collect();
        // Error 30-3 = 27 > top 0+20 = 20 = inspection 20+0 (tie by text).
        assert_eq!(
            texts,
            [
                "fix error",
                "inspection",
                "top intention",
                "a intention",
                "b intention"
            ]
        );
    }
}
