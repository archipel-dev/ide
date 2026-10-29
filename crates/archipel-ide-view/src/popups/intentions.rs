//! The rows of the Alt+Enter popup (TUI-GUI-139, M18): the actions in the set's order, with
//! each submenu collapsed into one header row below the actions of its inspection, expanded
//! on demand. Both frontends show these rows and apply the chosen action through
//! `code.applyAlert`.

use archipel_ide_core::intentions::{AlertSet, IntentionGroup};

/// One visible row of the popup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntentionRow {
    /// An action.
    Action {
        /// Index into the alert set (the `index` of `code.applyAlert`).
        index: usize,
        /// The text.
        text: String,
        /// The action fixes a problem (red bulb) instead of offering an intention (yellow).
        fix: bool,
        /// The row belongs to an expanded submenu (drawn indented).
        nested: bool,
    },
    /// The header of a submenu.
    Submenu {
        /// Its title.
        title: String,
        /// Whether its actions are listed below.
        open: bool,
    },
}

impl IntentionRow {
    /// The text of the row.
    pub fn text(&self) -> &str {
        match self {
            IntentionRow::Action { text, .. } => text,
            IntentionRow::Submenu { title, .. } => title,
        }
    }

    /// The text with the marker of a submenu header (`>` closed, `v` open) the GUI writes.
    pub fn label(&self) -> String {
        match self {
            IntentionRow::Action { text, .. } => text.clone(),
            IntentionRow::Submenu { title, open } => {
                format!("{} {title}", if *open { "v" } else { ">" })
            }
        }
    }
}

/// The rows of `set` with the submenu titled `expanded` open.
pub fn rows(set: &AlertSet, expanded: Option<&str>) -> Vec<IntentionRow> {
    let action = |index: usize, nested: bool| {
        let action = &set.actions[index];
        IntentionRow::Action {
            index,
            text: action.text.clone(),
            fix: matches!(
                action.group,
                IntentionGroup::Error | IntentionGroup::RemoteError
            ),
            nested,
        }
    };
    let mut rows = Vec::new();
    let mut titles: Vec<&str> = Vec::new();
    for (index, item) in set.actions.iter().enumerate() {
        match &item.submenu {
            None => rows.push(action(index, false)),
            Some(title) if !titles.contains(&title.as_str()) => titles.push(title),
            Some(_) => {}
        }
    }
    for title in titles {
        let open = expanded == Some(title);
        rows.push(IntentionRow::Submenu {
            title: title.to_owned(),
            open,
        });
        if open {
            for (index, item) in set.actions.iter().enumerate() {
                if item.submenu.as_deref() == Some(title) {
                    rows.push(action(index, true));
                }
            }
        }
    }
    rows
}

/// What choosing a row does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Choice {
    /// Apply the action at this index of the set.
    Apply(usize),
    /// Open (`Some`) or close (`None`) the submenu.
    Expand(Option<String>),
}

/// The effect of Enter or a click on `row`.
pub fn choose(row: &IntentionRow) -> Choice {
    match row {
        IntentionRow::Action { index, .. } => Choice::Apply(*index),
        IntentionRow::Submenu { title, open } => Choice::Expand((!open).then(|| title.clone())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use archipel_ide_analysis::intention::Priority;
    use archipel_ide_core::intentions::AlertAction;
    use archipel_ide_daemon::intention::Source;

    fn set() -> AlertSet {
        let mut sub = AlertAction::external(
            "Suppress for method",
            IntentionGroup::Inspection,
            Priority::Normal,
            Source::Lsp,
            "t1",
        );
        sub.submenu = Some("Unused import".into());
        let mut sub2 = sub.clone();
        sub2.text = "Suppress for class".into();
        AlertSet {
            actions: vec![
                AlertAction::external(
                    "Import class",
                    IntentionGroup::Error,
                    Priority::Normal,
                    Source::Lsp,
                    "t0",
                ),
                sub,
                sub2,
            ],
        }
    }

    /// QA: the submenu entries were dropped from the terminal popup.
    #[test]
    fn a_submenu_is_a_header_row_and_its_actions_appear_when_it_is_open() {
        let set = set();
        let closed = rows(&set, None);
        assert_eq!(closed.len(), 2);
        assert!(matches!(
            &closed[1],
            IntentionRow::Submenu { open: false, .. }
        ));
        let open = rows(&set, Some("Unused import"));
        assert_eq!(open.len(), 4);
        assert!(matches!(
            &open[2],
            IntentionRow::Action {
                nested: true,
                index: 1,
                ..
            }
        ));
        assert_eq!(
            choose(&closed[1]),
            Choice::Expand(Some("Unused import".into()))
        );
        assert_eq!(choose(&open[1]), Choice::Expand(None));
        assert_eq!(choose(&open[0]), Choice::Apply(0));
        assert!(matches!(&open[0], IntentionRow::Action { fix: true, .. }));
    }
}
