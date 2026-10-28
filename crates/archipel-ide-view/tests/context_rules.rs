//! The context menu availability rules (moved from the GUI's `actions.rs`): which rows of the
//! tree, editor and tab menus are enabled for their target.

use archipel_ide_view::actions::{ContextFacts, ContextKind, context_rule};

fn facts(kind: ContextKind) -> ContextFacts {
    ContextFacts {
        kind,
        open_tabs: 3,
        has_closed_tabs: false,
        has_clipboard: false,
        has_document: true,
    }
}

#[test]
fn tab_rules_follow_the_tab_strip() {
    let rule = |action, kind| context_rule(action, &facts(kind));
    assert_eq!(rule("CloseAllToTheLeft", ContextKind::Tab(0)), Some(false));
    assert_eq!(rule("CloseAllToTheLeft", ContextKind::Tab(1)), Some(true));
    assert_eq!(rule("CloseAllToTheRight", ContextKind::Tab(2)), Some(false));
    assert_eq!(rule("CloseAllToTheRight", ContextKind::Tab(1)), Some(true));
    assert_eq!(rule("ReopenClosedTab", ContextKind::Tab(0)), Some(false));
    assert_eq!(rule("CloseAllEditors", ContextKind::Tab(0)), Some(true));
    let single = ContextFacts {
        open_tabs: 1,
        ..facts(ContextKind::Tab(0))
    };
    assert_eq!(
        context_rule("CloseAllEditorsButActive", &single),
        Some(false)
    );
}

#[test]
fn paste_depends_on_the_target() {
    assert_eq!(
        context_rule("$Paste", &facts(ContextKind::Tree)),
        Some(false)
    );
    let with_clipboard = ContextFacts {
        has_clipboard: true,
        ..facts(ContextKind::Tree)
    };
    assert_eq!(context_rule("$Paste", &with_clipboard), Some(true));
    assert_eq!(
        context_rule("$Paste", &facts(ContextKind::Editor)),
        Some(true)
    );
}

#[test]
fn tree_entries_are_available_and_history_is_not() {
    let tree = facts(ContextKind::Tree);
    assert_eq!(context_rule("jt.git.stage", &tree), Some(true));
    assert_eq!(context_rule("LocalHistory", &tree), Some(false));
    assert_eq!(context_rule("SafeDelete", &tree), Some(true));
    // Anything else is the action's own business.
    assert_eq!(context_rule("GotoLine", &tree), None);
    assert_eq!(
        context_rule("SafeDelete", &facts(ContextKind::Editor)),
        None
    );
}
