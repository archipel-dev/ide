//! The Tier-A inspection set (spec Appendix B): grammar-only checks that need
//! no resolve and run on every G0 language. Each inspection is driven by the
//! node-kind tables of [`super::kinds`], so a language gains one by listing
//! kinds, not by writing code.
//!
//! Not in this set (they need G1 locals or a dictionary): unused local
//! variable, unused import, shadowed variable, typo.

use super::engine::OptionsExt;
use super::fixes::{EditFix, whole_line_if_alone};
use super::kinds::{KINDS, Kinds, kinds_of};
use super::suppress::{CommentSuppressor, JavaSuppressor, RustSuppressor};
use archipel_ide_analysis::inspect::{
    FileCx, InspectCx, InspectionSuppressor, LanguageFilter, Level, LocalInspection, OptionDef,
    OptionKind, ProblemDescriptor, ProblemHighlightType, ProblemsHolder,
};
use archipel_ide_analysis::stamps::IndexId;
use archipel_ide_analysis::{Cancelled, CompletenessTier, TextEdit};
use archipel_ide_language::{
    LANGUAGES, LanguageDef, Order, Registration, ServiceRegistry, SyntaxNode, TokenSet,
};
use std::collections::HashMap;
use std::ops::Range;
use std::sync::Arc;

/// Static description of one inspection.
pub struct Meta {
    pub id: &'static str,
    pub name: &'static str,
    pub group: &'static [&'static str],
    pub description: &'static str,
    pub level: Level,
    pub enabled: bool,
    pub options: &'static [OptionDef],
}

/// The logic of one inspection for one language.
trait Check: Send + Sync + 'static {
    fn meta(&self) -> &'static Meta;
    /// Element kinds to dispatch on (`None` = whole file).
    fn interesting(&self, def: &'static LanguageDef) -> Option<TokenSet>;
    fn element(
        &self,
        _n: SyntaxNode<'_>,
        _h: &mut ProblemsHolder,
        _cx: &InspectCx<'_>,
    ) -> Result<(), Cancelled> {
        Ok(())
    }
    fn file(&self, _f: &FileCx<'_>, _h: &mut ProblemsHolder) -> Result<(), Cancelled> {
        Ok(())
    }
}

struct Tool<C: Check> {
    check: C,
    meta: &'static Meta,
    set: Option<TokenSet>,
    languages: LanguageFilter,
}

impl<C: Check> LocalInspection for Tool<C> {
    fn id(&self) -> &'static str {
        self.meta.id
    }
    fn display_name(&self) -> &'static str {
        self.meta.name
    }
    fn group_path(&self) -> &'static [&'static str] {
        self.meta.group
    }
    fn description_md(&self) -> &'static str {
        self.meta.description
    }
    fn languages(&self) -> LanguageFilter {
        self.languages
    }
    fn default_level(&self) -> Level {
        self.meta.level
    }
    fn enabled_by_default(&self) -> bool {
        self.meta.enabled
    }
    fn min_tier(&self) -> CompletenessTier {
        CompletenessTier::G0
    }
    fn interesting(&self) -> Option<&TokenSet> {
        self.set.as_ref()
    }
    fn dumb_aware(&self) -> bool {
        true
    }
    fn needs_indexes(&self) -> &[IndexId] {
        &[]
    }
    fn options(&self) -> &'static [OptionDef] {
        self.meta.options
    }
    fn check_element(
        &self,
        n: SyntaxNode<'_>,
        h: &mut ProblemsHolder,
        cx: &InspectCx<'_>,
    ) -> Result<(), Cancelled> {
        self.check.element(n, h, cx)
    }
    fn check_file(&self, f: &FileCx<'_>, h: &mut ProblemsHolder) -> Result<(), Cancelled> {
        self.check.file(f, h)
    }
}

fn make<C: Check>(
    check: C,
    def: &'static LanguageDef,
    any_language: bool,
) -> Arc<dyn LocalInspection> {
    let meta = check.meta();
    let set = check.interesting(def);
    Arc::new(Tool {
        check,
        meta,
        set,
        languages: if any_language {
            LanguageFilter::Any
        } else {
            LanguageFilter::Only(std::slice::from_ref(&def.id))
        },
    })
}

fn kind_set(def: &'static LanguageDef, names: &[&[&str]]) -> Option<TokenSet> {
    let names: Vec<&str> = names.iter().flat_map(|n| n.iter().copied()).collect();
    let set = def.element_types().token_set(&names);
    (!set.is_empty()).then_some(set)
}

fn is_comment(n: SyntaxNode<'_>) -> bool {
    n.snapshot()
        .lang()
        .token_set_table()
        .comments
        .contains(n.element_type())
        || n.kind().contains("comment")
}

fn problem(
    h: &mut ProblemsHolder,
    range: Range<u32>,
    message: impl Into<String>,
    highlight: ProblemHighlightType,
) {
    h.register_problem(range, message, highlight);
}

fn problem_with_fix(
    h: &mut ProblemsHolder,
    range: Range<u32>,
    message: impl Into<String>,
    highlight: ProblemHighlightType,
    fix: Arc<dyn archipel_ide_analysis::intention::QuickFix>,
) {
    h.register(ProblemDescriptor {
        range,
        message: message.into(),
        highlight,
        fixes: vec![fix],
        on_the_fly_only: false,
    });
}

/// The node of exactly `range` with one of `kinds`, if any.
fn node_at<'s>(
    syntax: &'s archipel_ide_analysis::SyntaxSnapshot,
    range: &Range<u32>,
    kinds: &[&str],
) -> Option<SyntaxNode<'s>> {
    let mut node = syntax.covering(range.clone());
    loop {
        if node.range() == *range && kinds.contains(&node.kind()) {
            return Some(node);
        }
        match node.parent() {
            Some(parent) if parent.range() == *range => node = parent,
            _ => return None,
        }
    }
}

// ---------------------------------------------------------------------------
// Empty catch block

static EMPTY_CATCH: Meta = Meta {
    id: "EmptyCatchBlock",
    name: "Empty catch block",
    group: &["General", "Probable bugs"],
    description: "A `catch` block without statements or comments silently swallows the error.",
    level: Level::Warning,
    enabled: true,
    options: &[],
};

struct EmptyCatch(&'static Kinds);

impl Check for EmptyCatch {
    fn meta(&self) -> &'static Meta {
        &EMPTY_CATCH
    }
    fn interesting(&self, def: &'static LanguageDef) -> Option<TokenSet> {
        kind_set(def, &[self.0.catches])
    }
    fn element(
        &self,
        n: SyntaxNode<'_>,
        h: &mut ProblemsHolder,
        _: &InspectCx<'_>,
    ) -> Result<(), Cancelled> {
        let body = n
            .child_by_field_name("body")
            .filter(|b| self.0.blocks.contains(&b.kind()))
            .or_else(|| {
                n.named_children()
                    .find(|c| self.0.blocks.contains(&c.kind()))
            });
        let empty = match body {
            Some(body) => body.named_children().next().is_none(),
            None => {
                self.0.lang == "kotlin" && !n.named_children().any(|c| c.kind() == "statements")
            }
        };
        if empty {
            problem(
                h,
                n.range(),
                "Empty `catch` block swallows the error",
                ProblemHighlightType::GenericErrorOrWarning,
            );
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Empty if/else body

static EMPTY_BODY: Meta = Meta {
    id: "EmptyStatementBody",
    name: "Empty `if` or `else` body",
    group: &["General", "Code style issues"],
    description: "An `if` or `else` branch without statements does nothing.",
    level: Level::WeakWarning,
    enabled: true,
    options: &[],
};

struct EmptyBody(&'static Kinds);

impl Check for EmptyBody {
    fn meta(&self) -> &'static Meta {
        &EMPTY_BODY
    }
    fn interesting(&self, def: &'static LanguageDef) -> Option<TokenSet> {
        kind_set(def, &[self.0.blocks])
    }
    fn element(
        &self,
        n: SyntaxNode<'_>,
        h: &mut ProblemsHolder,
        _: &InspectCx<'_>,
    ) -> Result<(), Cancelled> {
        if n.named_children().next().is_some() {
            return Ok(());
        }
        let Some(parent) = n.parent() else {
            return Ok(());
        };
        let branch = if self.0.ifs.contains(&parent.kind()) {
            if parent.child_by_field_name("consequence") == Some(n) {
                Some("if")
            } else if parent.child_by_field_name("alternative") == Some(n) {
                Some("else")
            } else {
                None
            }
        } else if parent.kind() == "else_clause"
            && parent
                .parent()
                .is_some_and(|g| self.0.ifs.contains(&g.kind()))
        {
            Some("else")
        } else {
            None
        };
        if let Some(branch) = branch {
            let message = if branch == "if" {
                "Empty `if` body"
            } else {
                "Empty `else` branch"
            };
            problem(
                h,
                n.range(),
                message,
                ProblemHighlightType::GenericErrorOrWarning,
            );
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Unreachable code

static UNREACHABLE: Meta = Meta {
    id: "UnreachableCode",
    name: "Unreachable code",
    group: &["General", "Probable bugs"],
    description: "Statements after `return`, `break`, `continue` or `throw` in the same block never run.",
    level: Level::Warning,
    enabled: true,
    options: &[],
};

struct Unreachable(&'static Kinds);

impl Unreachable {
    fn is_terminator(&self, n: SyntaxNode<'_>) -> bool {
        if self.0.terminators.contains(&n.kind()) {
            return true;
        }
        self.0.expr_statements.contains(&n.kind())
            && n.named_children()
                .find(|c| !is_comment(*c))
                .is_some_and(|c| self.0.terminators.contains(&c.kind()))
    }
}

impl Check for Unreachable {
    fn meta(&self) -> &'static Meta {
        &UNREACHABLE
    }
    fn interesting(&self, def: &'static LanguageDef) -> Option<TokenSet> {
        kind_set(def, &[self.0.containers])
    }
    fn element(
        &self,
        n: SyntaxNode<'_>,
        h: &mut ProblemsHolder,
        _: &InspectCx<'_>,
    ) -> Result<(), Cancelled> {
        let children: Vec<SyntaxNode<'_>> =
            n.named_children().filter(|c| !is_comment(*c)).collect();
        let Some(at) = children.iter().position(|c| self.is_terminator(*c)) else {
            return Ok(());
        };
        let rest = &children[at + 1..];
        if let (Some(first), Some(last)) = (rest.first(), rest.last())
            && !self.0.reachable_starts.contains(&first.kind())
        {
            problem(
                h,
                first.range().start..last.range().end,
                "Unreachable code",
                ProblemHighlightType::LikeUnusedSymbol,
            );
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Assignment in condition

static ASSIGN_IN_COND: Meta = Meta {
    id: "AssignmentInCondition",
    name: "Assignment in condition",
    group: &["General", "Probable bugs"],
    description: "`if (a = b)` assigns instead of comparing; this is usually a typo for `==`.",
    level: Level::Warning,
    enabled: true,
    options: &[],
};

struct AssignInCondition(&'static Kinds);

fn unwrap_parens<'s>(mut n: SyntaxNode<'s>, parens: &[&str]) -> SyntaxNode<'s> {
    while parens.contains(&n.kind()) {
        let mut named = n.named_children();
        match (named.next(), named.next()) {
            (Some(inner), None) => n = inner,
            _ => break,
        }
    }
    n
}

fn replace_assign_with_eq(
    d: &ProblemDescriptor,
    cx: &archipel_ide_analysis::intention::FixCx<'_>,
) -> Option<Vec<TextEdit>> {
    let kinds = kinds_of(cx.syntax.lang().id)?;
    let node = node_at(cx.syntax, &d.range, kinds.assignments)?;
    let left = node.child_by_field_name("left")?;
    let right = node.child_by_field_name("right")?;
    let text = cx.syntax.text();
    let gap = text.get(left.range().end as usize..right.range().start as usize)?;
    let at = left.range().end as usize + gap.find('=')?;
    (gap.trim() == "=").then(|| vec![TextEdit::new(at as u32..at as u32 + 1, "==")])
}

impl Check for AssignInCondition {
    fn meta(&self) -> &'static Meta {
        &ASSIGN_IN_COND
    }
    fn interesting(&self, def: &'static LanguageDef) -> Option<TokenSet> {
        self.0
            .assign_in_condition
            .then(|| kind_set(def, &[self.0.conditions, self.0.whiles]))
            .flatten()
    }
    fn element(
        &self,
        n: SyntaxNode<'_>,
        h: &mut ProblemsHolder,
        _: &InspectCx<'_>,
    ) -> Result<(), Cancelled> {
        let Some(cond) = n.child_by_field_name("condition") else {
            return Ok(());
        };
        let inner = unwrap_parens(cond, self.0.parens);
        if self.0.assignments.contains(&inner.kind()) {
            problem_with_fix(
                h,
                inner.range(),
                "Assignment in condition; did you mean `==`?",
                ProblemHighlightType::GenericErrorOrWarning,
                EditFix::arc(
                    "Replace assignment with comparison",
                    "Replace '=' with '=='",
                    replace_assign_with_eq,
                ),
            );
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Constant condition

static CONSTANT_COND: Meta = Meta {
    id: "ConstantCondition",
    name: "Constant condition",
    group: &["General", "Probable bugs"],
    description: "A condition that is the literal `true` or `false` makes one branch dead.",
    level: Level::Warning,
    enabled: true,
    options: &[],
};

struct ConstantCondition(&'static Kinds);

impl Check for ConstantCondition {
    fn meta(&self) -> &'static Meta {
        &CONSTANT_COND
    }
    fn interesting(&self, def: &'static LanguageDef) -> Option<TokenSet> {
        kind_set(def, &[self.0.conditions, self.0.whiles])
    }
    fn element(
        &self,
        n: SyntaxNode<'_>,
        h: &mut ProblemsHolder,
        _: &InspectCx<'_>,
    ) -> Result<(), Cancelled> {
        let Some(cond) = n.child_by_field_name("condition") else {
            return Ok(());
        };
        let inner = unwrap_parens(cond, self.0.parens);
        if inner.first_child().is_some() && !self.0.boolean_literals.contains(&inner.kind()) {
            return Ok(());
        }
        let truth = match inner.text() {
            "true" | "True" => true,
            "false" | "False" => false,
            _ => return Ok(()),
        };
        // `while (true)` is the idiomatic endless loop.
        if self.0.whiles.contains(&n.kind()) && truth {
            return Ok(());
        }
        problem(
            h,
            inner.range(),
            format!("Condition `{}` is always {truth}", inner.text()),
            ProblemHighlightType::GenericErrorOrWarning,
        );
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Redundant parentheses

static REDUNDANT_PARENS: Meta = Meta {
    id: "RedundantParentheses",
    name: "Redundant parentheses",
    group: &["General", "Code style issues"],
    description: "Parentheses around an expression that is already delimited.",
    level: Level::WeakWarning,
    enabled: true,
    options: &[],
};

struct RedundantParens(&'static Kinds);

fn remove_parens(
    d: &ProblemDescriptor,
    cx: &archipel_ide_analysis::intention::FixCx<'_>,
) -> Option<Vec<TextEdit>> {
    let kinds = kinds_of(cx.syntax.lang().id)?;
    let node = node_at(cx.syntax, &d.range, kinds.parens)?;
    let text = cx.syntax.text();
    let (start, end) = (node.range().start as usize, node.range().end as usize);
    if !text[start..end].starts_with('(') || !text[start..end].ends_with(')') {
        return None;
    }
    // `if(x)` must not become `ifx`.
    let glued = text[..start]
        .chars()
        .next_back()
        .is_some_and(|c| c.is_alphanumeric() || c == '_');
    let open = if glued { " " } else { "" };
    Some(vec![
        TextEdit::new(start as u32..start as u32 + 1, open),
        TextEdit::delete(end as u32 - 1..end as u32),
    ])
}

impl Check for RedundantParens {
    fn meta(&self) -> &'static Meta {
        &REDUNDANT_PARENS
    }
    fn interesting(&self, def: &'static LanguageDef) -> Option<TokenSet> {
        kind_set(def, &[self.0.parens])
    }
    fn element(
        &self,
        n: SyntaxNode<'_>,
        h: &mut ProblemsHolder,
        _: &InspectCx<'_>,
    ) -> Result<(), Cancelled> {
        let Some(parent) = n.parent() else {
            return Ok(());
        };
        let doubled =
            self.0.parens.contains(&parent.kind()) && parent.named_children().count() == 1;
        let free = self.0.paren_free_parents.contains(&parent.kind())
            && (parent.child_by_field_name("condition") == Some(n)
                || parent.child_by_field_name("value") == Some(n));
        if doubled || free {
            problem_with_fix(
                h,
                n.range(),
                "Redundant parentheses",
                ProblemHighlightType::GenericErrorOrWarning,
                EditFix::arc(
                    "Remove redundant parentheses",
                    "Remove redundant parentheses",
                    remove_parens,
                ),
            );
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Self assignment

static SELF_ASSIGN: Meta = Meta {
    id: "SelfAssignment",
    name: "Self assignment",
    group: &["General", "Probable bugs"],
    description: "`x = x` has no effect.",
    level: Level::Warning,
    enabled: true,
    options: &[],
};

struct SelfAssignment(&'static Kinds);

fn simple_path(text: &str) -> bool {
    !text.is_empty()
        && text
            .chars()
            .all(|c| c.is_alphanumeric() || matches!(c, '_' | '$' | '.' | ':' | '>' | '-' | '*'))
}

fn remove_statement(
    d: &ProblemDescriptor,
    cx: &archipel_ide_analysis::intention::FixCx<'_>,
) -> Option<Vec<TextEdit>> {
    let kinds = kinds_of(cx.syntax.lang().id)?;
    let node = node_at(cx.syntax, &d.range, kinds.assignments)?;
    let statement = node
        .parent()
        .filter(|p| kinds.expr_statements.contains(&p.kind()))
        .unwrap_or(node);
    Some(vec![TextEdit::delete(whole_line_if_alone(
        cx.syntax.text(),
        statement.range(),
    ))])
}

impl Check for SelfAssignment {
    fn meta(&self) -> &'static Meta {
        &SELF_ASSIGN
    }
    fn interesting(&self, def: &'static LanguageDef) -> Option<TokenSet> {
        kind_set(def, &[self.0.assignments])
    }
    fn element(
        &self,
        n: SyntaxNode<'_>,
        h: &mut ProblemsHolder,
        cx: &InspectCx<'_>,
    ) -> Result<(), Cancelled> {
        let left = n
            .child_by_field_name("left")
            .or_else(|| n.named_children().next());
        let right = n
            .child_by_field_name("right")
            .or_else(|| n.named_children().last());
        let (Some(left), Some(right)) = (left, right) else {
            return Ok(());
        };
        if left == right {
            return Ok(());
        }
        let text = cx.syntax.text();
        let gap = text
            .get(left.range().end as usize..right.range().start as usize)
            .unwrap_or("");
        if gap.trim() == "=" && left.text() == right.text() && simple_path(left.text()) {
            problem_with_fix(
                h,
                n.range(),
                format!("`{}` is assigned to itself", left.text()),
                ProblemHighlightType::GenericErrorOrWarning,
                EditFix::arc(
                    "Remove self-assignment",
                    "Remove the assignment",
                    remove_statement,
                ),
            );
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Duplicate branches

static DUP_BRANCHES: Meta = Meta {
    id: "DuplicateBranches",
    name: "Identical `if` and `else` branches",
    group: &["General", "Probable bugs"],
    description: "Both branches of the `if` have the same body, so the condition does not matter.",
    level: Level::Warning,
    enabled: true,
    options: &[],
};

struct DuplicateBranches(&'static Kinds);

impl Check for DuplicateBranches {
    fn meta(&self) -> &'static Meta {
        &DUP_BRANCHES
    }
    fn interesting(&self, def: &'static LanguageDef) -> Option<TokenSet> {
        kind_set(def, &[self.0.ifs])
    }
    fn element(
        &self,
        n: SyntaxNode<'_>,
        h: &mut ProblemsHolder,
        _: &InspectCx<'_>,
    ) -> Result<(), Cancelled> {
        let (Some(then), Some(mut other)) = (
            n.child_by_field_name("consequence"),
            n.child_by_field_name("alternative"),
        ) else {
            return Ok(());
        };
        if other.kind() == "else_clause" {
            let Some(body) = other
                .child_by_field_name("body")
                .or_else(|| other.named_children().next())
            else {
                return Ok(());
            };
            other = body;
        }
        if self.0.ifs.contains(&other.kind()) || other.kind() == "elif_clause" {
            return Ok(());
        }
        let squash = |s: &str| s.chars().filter(|c| !c.is_whitespace()).collect::<String>();
        let (a, b) = (squash(then.text()), squash(other.text()));
        if a == b && a.len() > 2 {
            problem(
                h,
                other.range(),
                "`else` branch is identical to the `if` branch",
                ProblemHighlightType::GenericErrorOrWarning,
            );
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// TODO / FIXME comments

static TODO: Meta = Meta {
    id: "TodoComment",
    name: "TODO and FIXME comments",
    group: &["General", "Comments"],
    description: "Comments marked `TODO` or `FIXME` (informational).",
    level: Level::Information,
    enabled: true,
    options: &[OptionDef {
        id: "markers",
        label: "Markers (comma separated)",
        kind: OptionKind::Text("TODO,FIXME"),
    }],
};

struct TodoComment;

impl Check for TodoComment {
    fn meta(&self) -> &'static Meta {
        &TODO
    }
    fn interesting(&self, def: &'static LanguageDef) -> Option<TokenSet> {
        let named = def.element_types().token_set(&[
            "comment",
            "Comment",
            "line_comment",
            "block_comment",
            "html_comment",
        ]);
        let set = def.token_set_table().comments.union(&named);
        (!set.is_empty()).then_some(set)
    }
    fn element(
        &self,
        n: SyntaxNode<'_>,
        h: &mut ProblemsHolder,
        cx: &InspectCx<'_>,
    ) -> Result<(), Cancelled> {
        let markers = cx.options.text("markers", "TODO,FIXME");
        let text = n.text();
        let base = n.range().start;
        let mut offset = 0;
        for line in text.split_inclusive('\n') {
            let found = markers
                .split(',')
                .map(str::trim)
                .filter(|m| !m.is_empty())
                .filter_map(|marker| {
                    let mut from = 0;
                    while let Some(at) = line[from..].find(marker) {
                        let at = from + at;
                        let before = line[..at].chars().next_back();
                        let after = line[at + marker.len()..].chars().next();
                        if !before.is_some_and(char::is_alphanumeric)
                            && !after.is_some_and(char::is_alphanumeric)
                        {
                            return Some(at);
                        }
                        from = at + marker.len();
                    }
                    None
                })
                .min();
            if let Some(at) = found {
                let shown = line[at..].trim_end_matches(['\n', '\r', '*', '/', ' ', '-', '>']);
                let start = base + (offset + at) as u32;
                problem(
                    h,
                    start..start + shown.len() as u32,
                    shown.to_owned(),
                    ProblemHighlightType::Information,
                );
            }
            offset += line.len();
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Duplicate keys

static DUP_KEY: Meta = Meta {
    id: "DuplicateKey",
    name: "Duplicate key",
    group: &["Data files", "Probable bugs"],
    description: "A key (or XML attribute) appears twice in the same mapping; later values override or conflict with earlier ones.",
    level: Level::Warning,
    enabled: true,
    options: &[],
};

struct DupRules {
    lang: &'static str,
    containers: &'static [&'static str],
    pairs: &'static [&'static str],
    key: fn(SyntaxNode<'_>) -> Option<SyntaxNode<'_>>,
}

fn field_key<'s>(n: SyntaxNode<'s>) -> Option<SyntaxNode<'s>> {
    n.child_by_field_name("key")
}
fn first_named<'s>(n: SyntaxNode<'s>) -> Option<SyntaxNode<'s>> {
    n.named_children().next()
}

static DUP_RULES: &[DupRules] = &[
    DupRules {
        lang: "json",
        containers: &["object"],
        pairs: &["pair"],
        key: field_key,
    },
    DupRules {
        lang: "yaml",
        containers: &["block_mapping", "flow_mapping"],
        pairs: &["block_mapping_pair", "flow_pair"],
        key: field_key,
    },
    DupRules {
        lang: "toml",
        containers: &["document", "table", "table_array_element"],
        pairs: &["pair", "table"],
        key: first_named,
    },
    DupRules {
        lang: "properties",
        containers: &["file"],
        pairs: &["property"],
        key: first_named,
    },
    DupRules {
        lang: "xml",
        containers: &["STag", "EmptyElemTag"],
        pairs: &["Attribute"],
        key: first_named,
    },
];

struct DuplicateKey(&'static DupRules);

impl Check for DuplicateKey {
    fn meta(&self) -> &'static Meta {
        &DUP_KEY
    }
    fn interesting(&self, def: &'static LanguageDef) -> Option<TokenSet> {
        kind_set(def, &[self.0.containers])
    }
    fn element(
        &self,
        n: SyntaxNode<'_>,
        h: &mut ProblemsHolder,
        _: &InspectCx<'_>,
    ) -> Result<(), Cancelled> {
        let mut seen: HashMap<(bool, String), ()> = HashMap::new();
        for pair in n
            .named_children()
            .filter(|c| self.0.pairs.contains(&c.kind()))
        {
            let Some(key) = (self.0.key)(pair) else {
                continue;
            };
            let name = key.text().trim().trim_matches(['"', '\'']).to_owned();
            if name.is_empty() {
                continue;
            }
            let is_table = pair.kind() == "table";
            if seen.insert((is_table, name.clone()), ()).is_some() {
                let what = if is_table { "table" } else { "key" };
                problem(
                    h,
                    key.range(),
                    format!("Duplicate {what} '{name}'"),
                    ProblemHighlightType::GenericErrorOrWarning,
                );
            }
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Whitespace checks (whole file)

static TRAILING_WS: Meta = Meta {
    id: "TrailingWhitespace",
    name: "Trailing whitespace",
    group: &["General", "Code style issues"],
    description: "Spaces or tabs at the end of a line.",
    level: Level::WeakWarning,
    enabled: false,
    options: &[],
};

static MIXED_INDENT: Meta = Meta {
    id: "MixedIndentation",
    name: "Mixed tabs and spaces in indentation",
    group: &["General", "Code style issues"],
    description: "A tab that follows spaces in a line's indentation renders differently with other tab widths.",
    level: Level::WeakWarning,
    enabled: true,
    options: &[],
};

static LONG_LINE: Meta = Meta {
    id: "LongLine",
    name: "Very long line",
    group: &["General", "Code style issues"],
    description: "A line longer than the configured number of characters.",
    level: Level::WeakWarning,
    enabled: false,
    options: &[OptionDef {
        id: "max",
        label: "Maximum line length",
        kind: OptionKind::Int {
            default: 160,
            min: 40,
            max: 1000,
        },
    }],
};

fn delete_range(
    d: &ProblemDescriptor,
    _: &archipel_ide_analysis::intention::FixCx<'_>,
) -> Option<Vec<TextEdit>> {
    Some(vec![TextEdit::delete(d.range.clone())])
}

fn tabs_to_spaces(
    d: &ProblemDescriptor,
    cx: &archipel_ide_analysis::intention::FixCx<'_>,
) -> Option<Vec<TextEdit>> {
    let text = cx
        .syntax
        .text()
        .get(d.range.start as usize..d.range.end as usize)?;
    if !text.chars().all(|c| c == ' ' || c == '\t') {
        return None;
    }
    let mut width = 0;
    for c in text.chars() {
        width = if c == '\t' {
            (width / 4 + 1) * 4
        } else {
            width + 1
        };
    }
    Some(vec![TextEdit::new(d.range.clone(), " ".repeat(width))])
}

/// Lines of `text` with their byte offsets, polling `cancel`.
fn lines<'t>(
    text: &'t str,
    cancel: &archipel_ide_analysis::CancelToken,
    mut each: impl FnMut(usize, &'t str),
) -> Result<(), Cancelled> {
    let mut checkpoint = cancel.checkpoint();
    let mut offset = 0;
    for line in text.split('\n') {
        checkpoint.tick()?;
        each(offset, line);
        offset += line.len() + 1;
    }
    Ok(())
}

struct TrailingWhitespace;
struct MixedIndentation;
struct LongLine;

impl Check for TrailingWhitespace {
    fn meta(&self) -> &'static Meta {
        &TRAILING_WS
    }
    fn interesting(&self, _: &'static LanguageDef) -> Option<TokenSet> {
        None
    }
    fn file(&self, f: &FileCx<'_>, h: &mut ProblemsHolder) -> Result<(), Cancelled> {
        lines(f.syntax.text(), f.cancel, |offset, line| {
            let line = line.strip_suffix('\r').unwrap_or(line);
            let trimmed = line.trim_end_matches([' ', '\t']);
            if trimmed.len() < line.len() && !trimmed.is_empty() {
                problem_with_fix(
                    h,
                    (offset + trimmed.len()) as u32..(offset + line.len()) as u32,
                    "Trailing whitespace",
                    ProblemHighlightType::GenericErrorOrWarning,
                    EditFix::arc(
                        "Remove trailing whitespace",
                        "Remove trailing whitespace",
                        delete_range,
                    ),
                );
            }
        })
    }
}

impl Check for MixedIndentation {
    fn meta(&self) -> &'static Meta {
        &MIXED_INDENT
    }
    fn interesting(&self, _: &'static LanguageDef) -> Option<TokenSet> {
        None
    }
    fn file(&self, f: &FileCx<'_>, h: &mut ProblemsHolder) -> Result<(), Cancelled> {
        lines(f.syntax.text(), f.cancel, |offset, line| {
            let indent_len = line.len() - line.trim_start_matches([' ', '\t']).len();
            let indent = &line[..indent_len];
            if let Some(tab) = indent.find('\t')
                && indent[..tab].contains(' ')
            {
                problem_with_fix(
                    h,
                    offset as u32..(offset + indent_len) as u32,
                    "Tab after spaces in indentation",
                    ProblemHighlightType::GenericErrorOrWarning,
                    EditFix::arc(
                        "Convert indentation to spaces",
                        "Convert indentation to spaces",
                        tabs_to_spaces,
                    ),
                );
            }
        })
    }
}

impl Check for LongLine {
    fn meta(&self) -> &'static Meta {
        &LONG_LINE
    }
    fn interesting(&self, _: &'static LanguageDef) -> Option<TokenSet> {
        None
    }
    fn file(&self, f: &FileCx<'_>, h: &mut ProblemsHolder) -> Result<(), Cancelled> {
        let max = f.options.int("max", 160).max(1) as usize;
        lines(f.syntax.text(), f.cancel, |offset, line| {
            if line.len() <= max {
                return;
            }
            let Some((cut, _)) = line.char_indices().nth(max) else {
                return;
            };
            let count = line.chars().count();
            problem(
                h,
                (offset + cut) as u32..(offset + line.len()) as u32,
                format!("Line is {count} characters long (limit {max})"),
                ProblemHighlightType::GenericErrorOrWarning,
            );
        })
    }
}

// ---------------------------------------------------------------------------
// Registration

fn languages_without_text_checks(id: &str) -> bool {
    matches!(id, "markdown" | "markdown_inline")
}

/// Registers the Tier-A inspections and the generic/native suppressors.
pub fn register(registry: &mut ServiceRegistry) {
    let add = |registry: &mut ServiceRegistry,
               tool: Arc<dyn LocalInspection>,
               language: &'static LanguageDef| {
        let registration = Registration::new(tool.id(), language.language_id()).dumb_aware(true);
        registry.register::<dyn LocalInspection>(registration, tool);
    };
    for kinds in KINDS {
        let Some(def) = LANGUAGES.iter().find(|l| l.id == kinds.lang) else {
            continue;
        };
        let checks: Vec<Arc<dyn LocalInspection>> = vec![
            make(EmptyCatch(kinds), def, false),
            make(EmptyBody(kinds), def, false),
            make(Unreachable(kinds), def, false),
            make(AssignInCondition(kinds), def, false),
            make(ConstantCondition(kinds), def, false),
            make(RedundantParens(kinds), def, false),
            make(SelfAssignment(kinds), def, false),
            make(DuplicateBranches(kinds), def, false),
        ];
        for tool in checks {
            // Skip tools that dispatch on nothing in this grammar.
            if tool.interesting().is_some() {
                add(registry, tool, def);
            }
        }
    }
    for rules in DUP_RULES {
        if let Some(def) = LANGUAGES.iter().find(|l| l.id == rules.lang) {
            let tool = make(DuplicateKey(rules), def, false);
            if tool.interesting().is_some() {
                add(registry, tool, def);
            }
        }
    }
    for def in LANGUAGES.iter() {
        let tool = make(TodoComment, def, false);
        if tool.interesting().is_some() {
            add(registry, tool, def);
        }
        if !languages_without_text_checks(def.id) {
            for tool in [
                make(TrailingWhitespace, def, false),
                make(MixedIndentation, def, false),
                make(LongLine, def, false),
            ] {
                add(registry, tool, def);
            }
        }
    }
    let any = |id| Registration::any(id).dumb_aware(true);
    registry.register::<dyn InspectionSuppressor>(
        any("comment-suppressor"),
        Arc::new(CommentSuppressor),
    );
    registry.register::<dyn InspectionSuppressor>(
        Registration::new(
            "java-suppress-warnings",
            archipel_ide_language::LanguageId("java"),
        )
        .dumb_aware(true)
        .order(Order::First),
        Arc::new(JavaSuppressor),
    );
    registry.register::<dyn InspectionSuppressor>(
        Registration::new("rust-allow", archipel_ide_language::LanguageId("rust"))
            .dumb_aware(true)
            .order(Order::First),
        Arc::new(RustSuppressor),
    );
}
