//! Alt+Enter assembly: ordering (golden), dedupe, cache, apply.

mod common;
use archipel_ide_analysis::inspect::{
    InspectCx, LanguageFilter, Level, LocalInspection, OptionDef, ProblemDescriptor,
    ProblemHighlightType, ProblemsHolder,
};
use archipel_ide_analysis::intention::{
    ActionCx, FixCx, IntentionAction, IntentionGroup, Presentation, Priority, QuickFix,
};
use archipel_ide_analysis::modcommand::apply_edits;
use archipel_ide_analysis::stamps::IndexId;
use archipel_ide_analysis::{
    CancelToken, Cancelled, CompletenessTier, FileId, HostConfig, ModCommand, TextEdit, TokenSet,
};
use archipel_ide_daemon::intention::{
    AlertAction, AlertCache, AlertInput, Applied, ApplyInput, Bulb, Source, apply, assemble,
};
use archipel_ide_language::{LanguageId, Registration, SyntaxNode, registry};
use common::daemon_harness::DaemonHarness;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

/// Replaces the problem range by `with`.
struct ReplaceFix {
    family: &'static str,
    name: &'static str,
    with: &'static str,
}

impl QuickFix for ReplaceFix {
    fn family(&self) -> &str {
        self.family
    }
    fn name(&self) -> String {
        self.name.to_owned()
    }
    fn fix(&self, d: &ProblemDescriptor, cx: &FixCx<'_>) -> Result<ModCommand, Cancelled> {
        Ok(ModCommand::UpdateFile {
            file: cx.file,
            edits: vec![TextEdit::new(d.range.clone(), self.with)],
            expected_rev: cx.syntax.rev(),
        })
    }
}

/// `/* ERR */` -> an error with one fix; `/* WARN */` -> a warning with two.
struct CommentTool {
    set: TokenSet,
}

impl LocalInspection for CommentTool {
    fn id(&self) -> &'static str {
        "CommentFix"
    }
    fn display_name(&self) -> &'static str {
        "Marked comments"
    }
    fn group_path(&self) -> &'static [&'static str] {
        &["Test"]
    }
    fn description_md(&self) -> &'static str {
        ""
    }
    fn languages(&self) -> LanguageFilter {
        LanguageFilter::Any
    }
    fn default_level(&self) -> Level {
        Level::Warning
    }
    fn enabled_by_default(&self) -> bool {
        true
    }
    fn min_tier(&self) -> CompletenessTier {
        CompletenessTier::G0
    }
    fn interesting(&self) -> Option<&TokenSet> {
        Some(&self.set)
    }
    fn dumb_aware(&self) -> bool {
        true
    }
    fn needs_indexes(&self) -> &[IndexId] {
        &[]
    }
    fn options(&self) -> &'static [OptionDef] {
        &[]
    }
    fn check_element(
        &self,
        n: SyntaxNode<'_>,
        h: &mut ProblemsHolder,
        _: &InspectCx<'_>,
    ) -> Result<(), Cancelled> {
        let text = n.text();
        let fixes: Vec<Arc<dyn QuickFix>> = if text.contains("ERR") {
            vec![Arc::new(ReplaceFix {
                family: "Fix error",
                name: "Fix the error",
                with: "/* ok */",
            })]
        } else if text.contains("WARN") {
            vec![
                Arc::new(ReplaceFix {
                    family: "Silence",
                    name: "Silence the warning",
                    with: "/* ok */",
                }),
                Arc::new(ReplaceFix {
                    family: "Delete",
                    name: "Delete the warning",
                    with: "",
                }),
            ]
        } else {
            return Ok(());
        };
        h.register(ProblemDescriptor {
            range: n.range(),
            message: text.to_owned(),
            highlight: if text.contains("ERR") {
                ProblemHighlightType::Error
            } else {
                ProblemHighlightType::GenericErrorOrWarning
            },
            fixes,
            on_the_fly_only: false,
        });
        Ok(())
    }
}

struct Intent {
    id: &'static str,
    text: &'static str,
    priority: Priority,
}

impl IntentionAction for Intent {
    fn family(&self) -> &'static str {
        "Test intentions"
    }
    fn id(&self) -> &'static str {
        self.id
    }
    fn languages(&self) -> LanguageFilter {
        LanguageFilter::Any
    }
    fn dumb_aware(&self) -> bool {
        true
    }
    fn priority(&self) -> Priority {
        self.priority
    }
    fn presentation(
        &self,
        _: &ActionCx<'_>,
    ) -> Option<archipel_ide_analysis::intention::Presentation> {
        Some(Presentation {
            text: self.text.to_owned(),
        })
    }
    fn perform(&self, cx: &ActionCx<'_>) -> Result<ModCommand, Cancelled> {
        Ok(ModCommand::UpdateFile {
            file: cx.file,
            edits: vec![TextEdit::insert(0, "// intent\n")],
            expected_rev: cx.syntax.rev(),
        })
    }
}

const INTENTIONS: &[(&str, &str, Priority)] = &[
    ("normal", "Convert something", Priority::Normal),
    ("top", "Alpha top intention", Priority::Top),
    ("high", "High intention", Priority::High),
    ("low", "Zeta low intention", Priority::Low),
    ("bottom", "Bottom intention", Priority::Bottom),
    ("normal2", "Another normal", Priority::Normal),
];

struct Fixture {
    h: DaemonHarness,
    file: FileId,
}

fn fixture(src: &str, intentions: bool) -> Fixture {
    let js = registry::by_id("javascript").unwrap();
    let comments = js.token_set_table().comments.clone();
    let mut h = DaemonHarness::with(HostConfig::testing(), move |registry| {
        registry.register::<dyn LocalInspection>(
            Registration::new("comment-fix", LanguageId("javascript")).dumb_aware(true),
            Arc::new(CommentTool { set: comments }),
        );
        if intentions {
            for (id, text, priority) in INTENTIONS {
                registry.register::<dyn IntentionAction>(
                    Registration::any(id).dumb_aware(true),
                    Arc::new(Intent {
                        id,
                        text,
                        priority: *priority,
                    }),
                );
            }
        }
    });
    let file = h.open("a.js", src);
    h.settle(Duration::from_millis(100));
    Fixture { h, file }
}

fn alerts(
    f: &Fixture,
    caret: u32,
    extra: Vec<AlertAction>,
    disabled: &HashSet<String>,
) -> archipel_ide_daemon::intention::AlertSet {
    let snap = f.h.host.snapshot();
    let syntax = snap.syntax(f.file).unwrap();
    let daemon = f.h.svc.lock();
    let markup = daemon.markup(f.file).unwrap();
    let names: HashMap<String, String> =
        [("CommentFix".to_owned(), "Marked comments".to_owned())].into();
    let cancel = CancelToken::new();
    assemble(&AlertInput {
        snap: &snap,
        file: f.file,
        syntax: &syntax,
        markup,
        caret,
        selection: None,
        cancel: &cancel,
        extra,
        disabled,
        tool_names: &names,
    })
    .unwrap()
}

const SRC: &str = "foo(); /* ERR */ bar(); /* WARN */\n";

#[test]
fn golden_order_follows_group_plus_priority_weight_then_text() {
    use IntentionGroup::*;
    use Priority::*;
    let f = fixture(SRC, true);
    let caret = SRC.find("ERR").unwrap() as u32 + 1;
    let set = alerts(&f, caret, vec![], &HashSet::new());
    let top: Vec<_> = set
        .top_level()
        .map(|a| (a.weight(), a.group, a.priority, a.text.as_str()))
        .collect();
    assert_eq!(
        top,
        [
            (30, Error, Normal, "Fix the error"),
            (20, Other, Top, "Alpha top intention"),
            // WARN is elsewhere on the line: remote error fixes (10), tie by text.
            (10, RemoteError, Normal, "Delete the warning"),
            (10, RemoteError, Normal, "Silence the warning"),
            (3, Other, High, "High intention"),
            (0, Other, Normal, "Another normal"),
            (0, Other, Normal, "Convert something"),
            (-3, Other, Low, "Zeta low intention"),
            (-20, Other, Bottom, "Bottom intention"),
        ]
    );
    // The inspection's wrappers live in a submenu below its fix.
    let sub: Vec<_> = set
        .submenu("Inspection 'Marked comments'")
        .map(|a| a.text.clone())
        .collect();
    assert!(
        sub.contains(&"Disable inspection 'Marked comments'".to_owned()),
        "{sub:?}"
    );
    assert!(sub.contains(&"Suppress for file".to_owned()));
    assert_eq!(set.bulb(), Bulb::ErrorFix);
}

#[test]
fn problems_at_the_caret_are_inspection_fixes_and_others_are_remote() {
    use IntentionGroup::*;
    let f = fixture(SRC, true);
    let caret = SRC.find("WARN").unwrap() as u32 + 1;
    let set = alerts(&f, caret, vec![], &HashSet::new());
    let top: Vec<_> = set
        .top_level()
        .map(|a| (a.weight(), a.group, a.text.as_str()))
        .collect();
    assert_eq!(
        &top[..4],
        [
            // Inspection fixes (20) tie with the top intention (0 + 20): by text.
            (20, Other, "Alpha top intention"),
            (20, Inspection, "Delete the warning"),
            (20, Inspection, "Silence the warning"),
            (10, RemoteError, "Fix the error"),
        ]
    );
    assert_eq!(
        set.bulb(),
        Bulb::ErrorFix,
        "a remote error fix on the line still colors the bulb"
    );
}

#[test]
fn bulb_and_empty_states() {
    let f = fixture("foo(); /* WARN */\n", false);
    let caret = 10;
    let set = alerts(&f, caret, vec![], &HashSet::new());
    assert_eq!(set.bulb(), Bulb::Intention, "warnings only: no error fix");
    let clean = fixture("foo();\n", false);
    let none = alerts(&clean, 2, vec![], &HashSet::new());
    assert!(none.actions.is_empty());
    assert_eq!(none.bulb(), Bulb::None);
    let with_intentions = fixture("foo();\n", true);
    assert_eq!(
        alerts(&with_intentions, 2, vec![], &HashSet::new()).bulb(),
        Bulb::Intention
    );
}

#[test]
fn dedupe_prefers_native_and_disabled_intentions_vanish() {
    let f = fixture(SRC, true);
    let caret = SRC.find("ERR").unwrap() as u32 + 1;
    let lsp = |text: &str| {
        AlertAction::external(
            text,
            IntentionGroup::Other,
            Priority::Normal,
            Source::Lsp,
            format!("lsp:{text}"),
        )
    };
    let mut disabled = HashSet::new();
    disabled.insert("bottom".to_owned());
    let set = alerts(&f, caret, vec![lsp("Server-only refactor")], &disabled);
    let texts = set.texts();
    assert!(!texts.contains(&"Bottom intention"), "disabled intention");
    assert_eq!(
        texts
            .iter()
            .filter(|t| **t == "Server-only refactor")
            .count(),
        1
    );
    // Same (family, text) from the server collapses to the native row.
    let same = AlertAction {
        family: "Test intentions".into(),
        ..lsp("Convert something")
    };
    let set = alerts(&f, caret, vec![same], &HashSet::new());
    let rows: Vec<_> = set
        .actions
        .iter()
        .filter(|a| a.text == "Convert something")
        .collect();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].source, Source::Native);
    // A different family is a different row.
    let set = alerts(&f, caret, vec![lsp("Convert something")], &HashSet::new());
    assert_eq!(
        set.actions
            .iter()
            .filter(|a| a.text == "Convert something")
            .count(),
        2
    );
}

#[test]
fn the_cache_is_keyed_by_revision_caret_and_markup() {
    let f = fixture(SRC, true);
    let snap = f.h.host.snapshot();
    let syntax = snap.syntax(f.file).unwrap();
    let daemon = f.h.svc.lock();
    let markup = daemon.markup(f.file).unwrap();
    let names = HashMap::new();
    let disabled = HashSet::new();
    let cancel = CancelToken::new();
    let input = |caret| AlertInput {
        snap: &snap,
        file: f.file,
        syntax: &syntax,
        markup,
        caret,
        selection: None,
        cancel: &cancel,
        extra: vec![],
        disabled: &disabled,
        tool_names: &names,
    };
    let mut cache = AlertCache::default();
    let a = cache.get_or_compute(&input(3), 1).unwrap();
    let b = cache.get_or_compute(&input(3), 1).unwrap();
    assert!(Arc::ptr_eq(&a, &b), "same state: cached");
    assert!(
        !Arc::ptr_eq(&a, &cache.get_or_compute(&input(4), 1).unwrap()),
        "caret moved"
    );
    let c = cache.get_or_compute(&input(4), 1).unwrap();
    assert!(
        !Arc::ptr_eq(&c, &cache.get_or_compute(&input(4), 2).unwrap()),
        "highlights changed"
    );
    cache.forget(f.file);
    assert!(cache.is_empty());
}

fn command_text(src: &str, applied: Applied) -> String {
    match applied {
        Applied::Command(ModCommand::UpdateFile { edits, .. }) => apply_edits(src, &edits).unwrap(),
        other => panic!("expected an edit, got {other:?}"),
    }
}

fn apply_row(f: &Fixture, caret: u32, text: &str) -> Applied {
    let set = alerts(f, caret, vec![], &HashSet::new());
    let row = set
        .actions
        .iter()
        .find(|a| a.text == text)
        .unwrap_or_else(|| panic!("no row `{text}` in {:?}", set.texts()));
    let snap = f.h.host.snapshot();
    let syntax = snap.syntax(f.file).unwrap();
    let daemon = f.h.svc.lock();
    let cancel = CancelToken::new();
    apply(
        row,
        &ApplyInput {
            snap: &snap,
            file: f.file,
            syntax: &syntax,
            markup: daemon.markup(f.file).unwrap(),
            caret,
            selection: None,
            cancel: &cancel,
        },
    )
    .unwrap()
}

#[test]
fn choosing_rows_returns_pure_commands() {
    let f = fixture(SRC, true);
    let caret = SRC.find("ERR").unwrap() as u32 + 1;
    assert_eq!(
        command_text(SRC, apply_row(&f, caret, "Fix the error")),
        "foo(); /* ok */ bar(); /* WARN */\n"
    );
    assert_eq!(
        command_text(SRC, apply_row(&f, caret, "Alpha top intention")),
        format!("// intent\n{SRC}")
    );
    assert_eq!(
        apply_row(&f, caret, "Disable inspection 'Marked comments'"),
        Applied::DisableInspection {
            tool: "CommentFix".into()
        }
    );
    // Suppress for file inserts the comment (which then silences the tool).
    let text = command_text(SRC, apply_row(&f, caret, "Suppress for file"));
    assert!(text.starts_with("// noinspection-file CommentFix\n"));
}

#[test]
fn fix_all_merges_the_fixes_of_every_occurrence() {
    let src = "a(); /* WARN */ b(); /* WARN */ c(); /* WARN */\n";
    let f = fixture(src, false);
    let caret = src.find("WARN").unwrap() as u32 + 1;
    let set = alerts(&f, caret, vec![], &HashSet::new());
    assert!(
        set.submenu("Inspection 'Marked comments'")
            .any(|a| a.text == "Fix all 'Silence' problems in file"),
        "{:?}",
        set.texts()
    );
    let text = command_text(
        src,
        apply_row(&f, caret, "Fix all 'Silence' problems in file"),
    );
    assert_eq!(text, "a(); /* ok */ b(); /* ok */ c(); /* ok */\n");
}

#[test]
fn suppression_from_the_popup_silences_the_problem_after_reanalysis() {
    let mut f = fixture(SRC, false);
    let caret = SRC.find("WARN").unwrap() as u32 + 1;
    let text = command_text(SRC, apply_row(&f, caret, "Suppress for file"));
    assert!(
        f.h.problems(f.file)
            .iter()
            .any(|(t, _)| t == "native:CommentFix")
    );
    let new = f.h.edit(f.file, SRC, 0..0, text.strip_suffix(SRC).unwrap());
    assert_eq!(new, text);
    f.h.settle(Duration::from_millis(100));
    assert!(
        !f.h.problems(f.file)
            .iter()
            .any(|(t, _)| t == "native:CommentFix"),
        "{:?}",
        f.h.problems(f.file)
    );
}
