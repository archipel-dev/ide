//! Reference analysis of one Java file: the scope walk (spec 02 section 2.3 and 2.5) and the
//! unresolved-reference errors of spec 04 section 4.2:
//!
//! * JAVA-ERR-012 / 144 `Cannot resolve symbol '<name>'` on the first unresolved segment,
//! * JAVA-ERR-007 `Cannot resolve method '<name>(<argTypes>)'` for calls with zero candidates,
//! * JAVA-ERR-011 `Cannot resolve method '<name>(<argTypes>)' in '<Class>'` for type qualifiers,
//! * JAVA-ERR-003 / JAVA-STUB-026 `Not resolved until the project is fully loaded`.
//!
//! The analyzer is deliberately conservative (spec 04 4.1.3 and the graduation rule of
//! `docs/analysis/architecture.md`): nothing is reported while a supertype, an import or a
//! generated member could supply the name, and no resolution failure is reported on an
//! expression whose type is unknown. The same walk records which imports resolved a name
//! (JAVA-RES-002) for the unused-import inspection (JAVA-IMP-070..076).

use crate::env::{Ctx, CurrentFile, Env, NameRes, Res, TypeRes, Via};
use crate::index::ClassHit;
use crate::source;
use crate::stub::*;
use crate::ty::{self, Ty};
use std::collections::{HashMap, HashSet};
use std::ops::Range;
use std::sync::Arc;
use tree_sitter::{Node, Tree};

pub const PENDING_MESSAGE: &str = "Not resolved until the project is fully loaded";

#[derive(Debug, Clone, PartialEq)]
pub enum ProblemKind {
    /// `Cannot resolve symbol 'name'` (class, variable, field, package, import segment).
    Symbol {
        name: String,
        class_ref: bool,
        annotation: bool,
    },
    /// Call with zero candidates.
    Method {
        name: String,
        args: Vec<Ty>,
        qualified: bool,
        call: Range<u32>,
    },
    /// The unresolved reference of the pending-model state.
    Pending,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Problem {
    pub range: Range<u32>,
    pub message: String,
    pub kind: ProblemKind,
}

/// Which import statements resolved at least one reference (indexes into `FileStub::imports`).
#[derive(Debug, Clone, Default)]
pub struct Usage {
    pub used: HashSet<usize>,
    /// The analysis cannot say anything reliable (index lost, project loading): no import is unused.
    pub uncertain: bool,
    /// Short names referenced through a static import (field or method).
    pub static_names: HashSet<String>,
    /// FQNs of classes that were found through an import statement (single or on demand).
    pub classes: std::collections::BTreeSet<String>,
    /// Static members used through static imports: `(class named by the import, member)`.
    pub statics: std::collections::BTreeSet<(String, String)>,
    /// Simple names that did not resolve (or could not be resolved reliably): they count as
    /// using any import that could provide them (JAVA-IMP-076).
    pub unresolved: HashSet<String>,
    /// Set when some unresolved name could come from an on-demand import.
    pub unresolved_any: bool,
}

#[derive(Debug, Clone, Default)]
pub struct Analysis {
    pub problems: Vec<Problem>,
    pub usage: Usage,
}

pub struct Analyzer<'a, 'e> {
    pub env: &'a Env<'e>,
    pub text: &'a str,
    pub tree: &'a Tree,
    pub file: &'a CurrentFile,
    /// The project is not fully loaded: unresolved references are pending, not errors.
    pub incomplete: bool,
    out: Analysis,
    local_hits: HashMap<usize, ClassHit>,
    pattern_cache: HashMap<usize, HashSet<String>>,
    lombok: bool,
}

#[derive(Debug, Clone)]
pub enum VarRes {
    Local {
        type_text: Option<String>,
        init: Option<usize>,
        node_start: usize,
    },
    Field {
        owner: ClassHit,
        index: usize,
    },
    Component {
        owner: ClassHit,
        index: usize,
    },
    StaticImport {
        class: ClassHit,
        import: usize,
        ty: Option<String>,
    },
    NotFound {
        incomplete: bool,
    },
}

fn range(n: Node<'_>) -> Range<u32> {
    n.start_byte() as u32..n.end_byte() as u32
}

pub fn analyze(
    env: &Env<'_>,
    text: &str,
    tree: &Tree,
    file: &CurrentFile,
    incomplete: bool,
) -> Analysis {
    let lombok = file
        .stub
        .imports
        .iter()
        .any(|i| i.name.starts_with("lombok"))
        || text.contains("@lombok.");
    let mut a = Analyzer {
        env,
        text,
        tree,
        file,
        incomplete,
        out: Analysis::default(),
        local_hits: HashMap::new(),
        pattern_cache: HashMap::new(),
        lombok,
    };
    // Nothing can be said for a file that cannot be parsed reliably.
    let root = tree.root_node();
    if root.has_error() && !can_analyze_with_errors(root) {
        a.out.usage.uncertain = true;
        return a.out;
    }
    if a.check_imports().is_err() || a.visit(root).is_err() {
        // Index went away mid-analysis: no problems, uncertain usage.
        a.out.problems.clear();
        a.out.usage.uncertain = true;
    }
    a.out
}

/// Files with syntax errors still get checks away from the error (spec 04 4.1.3: elements with
/// parse errors are skipped by semantic checks); here the walk skips ERROR nodes themselves.
fn can_analyze_with_errors(_root: Node<'_>) -> bool {
    true
}

fn kind_is_class(k: &str) -> bool {
    source::is_class_kind(k)
}

impl<'a, 'e> Analyzer<'a, 'e> {
    fn txt(&self, n: Node<'_>) -> &'a str {
        &self.text[n.byte_range()]
    }

    fn push_symbol(&mut self, n: Node<'_>, name: &str, class_ref: bool, annotation: bool) {
        self.push_symbol_at(range(n), name, class_ref, annotation);
    }

    fn push_symbol_at(&mut self, r: Range<u32>, name: &str, class_ref: bool, annotation: bool) {
        self.unresolved(name);
        if self.incomplete {
            self.out.problems.push(Problem {
                range: r,
                message: PENDING_MESSAGE.into(),
                kind: ProblemKind::Pending,
            });
            return;
        }
        self.out.problems.push(Problem {
            range: r,
            message: format!("Cannot resolve symbol '{name}'"),
            kind: ProblemKind::Symbol {
                name: name.to_string(),
                class_ref,
                annotation,
            },
        });
    }

    fn mark(&mut self, via: Via, class: &ClassHit) {
        match via {
            Via::SingleImport(i) | Via::OnDemand(i) => {
                self.out.usage.used.insert(i);
                self.out.usage.classes.insert(class.class().qname.clone());
            }
            Via::SingleStaticImport(i) | Via::StaticOnDemand(i) => {
                self.out.usage.used.insert(i);
                self.mark_static(i, class.class().name.clone());
            }
            _ => {}
        }
    }

    /// A member used through static import `i`.
    fn mark_static(&mut self, i: usize, member: String) {
        let Some(imp) = self.file.stub.imports.get(i) else {
            return;
        };
        let class = if imp.on_demand {
            imp.name.clone()
        } else {
            imp.name
                .rsplit_once('.')
                .map(|(c, _)| c.to_string())
                .unwrap_or_default()
        };
        self.out.usage.used.insert(i);
        self.out.usage.statics.insert((class, member.clone()));
        self.out.usage.static_names.insert(member);
    }

    fn unresolved(&mut self, name: &str) {
        self.out.usage.unresolved.insert(name.to_string());
    }

    // ---- imports ----------------------------------------------------------------------

    /// Unresolved single-type and static imports: `Cannot resolve symbol` on the first unresolved
    /// segment (JAVA-ERR-144, JAVA-RES-027).
    fn check_imports(&mut self) -> Res<()> {
        let root = self.tree.root_node();
        let mut cur = root.walk();
        let nodes: Vec<Node<'_>> = root
            .children(&mut cur)
            .filter(|n| n.kind() == "import_declaration")
            .collect();
        for n in nodes {
            let is_static = n.children(&mut n.walk()).any(|c| c.kind() == "static");
            let on_demand = n.children(&mut n.walk()).any(|c| c.kind() == "asterisk");
            let Some(name) = n
                .children(&mut n.walk())
                .find(|c| matches!(c.kind(), "identifier" | "scoped_identifier"))
            else {
                continue;
            };
            let segs = scoped_segments(name);
            if segs.is_empty() || n.has_error() {
                continue;
            }
            let names: Vec<String> = segs.iter().map(|s| self.txt(*s).to_string()).collect();
            let class_segments = if is_static && !on_demand {
                names.len() - 1
            } else {
                names.len()
            };
            if on_demand && !is_static {
                continue; // package or class on demand: not reported in this slice
            }
            if class_segments == 0 {
                continue;
            }
            // Find the class prefix.
            let mut class_end = None;
            for i in 1..=class_segments {
                if self.env.find_class(&names[..i].join("."))?.is_some() {
                    class_end = Some(i);
                    break;
                }
            }
            match class_end {
                Some(i) => {
                    // Member classes of the import (`a.B.C`): the rest must be member types.
                    let mut h = self
                        .env
                        .find_class(&names[..i].join("."))?
                        .expect("found above");
                    let mut ok = true;
                    for (k, seg) in names[i..class_segments].iter().enumerate() {
                        match self.env.member_type(&h, seg)? {
                            Some(m) => h = m,
                            None => {
                                self.push_symbol(segs[i + k], seg, true, false);
                                ok = false;
                                break;
                            }
                        }
                    }
                    if ok && is_static && !on_demand {
                        let member = names.last().cloned().unwrap_or_default();
                        let (ms, c1) = self.env.methods_named(&h, &member)?;
                        let (fs, c2, comp) = self.env.fields_named(&h, &member)?;
                        let ty = self.env.member_type(&h, &member)?;
                        if ms.is_empty()
                            && fs.is_empty()
                            && !comp
                            && ty.is_none()
                            && c1
                            && c2
                            && !self.lombok
                            && !self.env.has_synthetic_method(&h, &member)
                        {
                            self.push_symbol(
                                *segs.last().expect("non-empty"),
                                &member,
                                false,
                                false,
                            );
                        }
                    }
                }
                None => {
                    // First unknown segment: the longest known package prefix, then the next segment.
                    let mut known = 0;
                    for i in 1..class_segments {
                        if self.env.package_exists(&names[..i].join("."))? {
                            known = i;
                        } else {
                            break;
                        }
                    }
                    let at = known.min(class_segments - 1);
                    self.push_symbol(segs[at], &names[at], at + 1 == class_segments, false);
                }
            }
        }
        Ok(())
    }

    // ---- the walk -----------------------------------------------------------------------

    fn visit(&mut self, node: Node<'_>) -> Res<()> {
        match node.kind() {
            "block_comment" => {
                self.javadoc(node)?;
                return Ok(());
            }
            "import_declaration"
            | "package_declaration"
            | "line_comment"
            | "string_literal"
            | "text_block"
            | "character_literal"
            | "ERROR" => return Ok(()),
            "marker_annotation" | "annotation" => {
                self.annotation(node)?;
                // Arguments may contain references.
                if let Some(args) = node.child_by_field_name("arguments") {
                    self.visit_children(args)?;
                }
                return Ok(());
            }
            "type_identifier" => {
                self.type_ref(node)?;
                return Ok(());
            }
            "scoped_type_identifier" => {
                self.scoped_type_ref(node)?;
                return Ok(());
            }
            "method_invocation" => return self.method_invocation(node),
            "field_access" => return self.field_access(node),
            "identifier" => {
                if self.ident_role(node).is_some() {
                    self.simple_name(node)?;
                }
                return Ok(());
            }
            "method_reference" => return self.method_reference(node),
            "type_parameter" => {
                // Declared name first; bounds are references.
                let mut c = node.walk();
                for ch in node.children(&mut c).skip(1) {
                    self.visit(ch)?;
                }
                return Ok(());
            }
            "object_creation_expression" => return self.object_creation(node),
            _ => {}
        }
        self.visit_children(node)
    }

    fn visit_children(&mut self, node: Node<'_>) -> Res<()> {
        let mut c = node.walk();
        let kids: Vec<Node<'_>> = node.children(&mut c).collect();
        for k in kids {
            self.visit(k)?;
        }
        Ok(())
    }

    /// Whether an `identifier` node is an expression name (as opposed to a declaration, label, ...).
    fn ident_role(&self, n: Node<'_>) -> Option<()> {
        let p = n.parent()?;
        let field_of = |f: &str| p.child_by_field_name(f).is_some_and(|c| c.id() == n.id());
        let ok = match p.kind() {
            "binary_expression"
            | "unary_expression"
            | "update_expression"
            | "parenthesized_expression"
            | "ternary_expression"
            | "argument_list"
            | "array_initializer"
            | "return_statement"
            | "throw_statement"
            | "yield_statement"
            | "expression_statement"
            | "synchronized_statement"
            | "assert_statement"
            | "dimensions_expr"
            | "element_value_array_initializer"
            | "guard" => true,
            "assignment_expression" => true,
            "array_access" => true,
            "variable_declarator" | "resource" => field_of("value"),
            "enhanced_for_statement" => field_of("value"),
            "cast_expression" => field_of("value"),
            "instanceof_expression" => field_of("left"),
            "element_value_pair" => field_of("value"),
            "lambda_expression" => field_of("body"),
            "field_access" => field_of("object"),
            "method_invocation" => field_of("object"),
            "switch_expression" | "switch_statement" | "if_statement" | "while_statement"
            | "do_statement" => field_of("condition"),
            _ => false,
        };
        ok.then_some(())
    }

    /// Javadoc references count as uses of imports (JAVA-IMP-073); they are never reported.
    fn javadoc(&mut self, n: Node<'_>) -> Res<()> {
        let text = self.txt(n);
        if !text.starts_with("/**") || !text.contains('@') {
            return Ok(());
        }
        let mut names: Vec<String> = Vec::new();
        for tag in [
            "{@link ",
            "{@linkplain ",
            "{@value ",
            "@see ",
            "@throws ",
            "@exception ",
        ] {
            let mut rest = text;
            while let Some(i) = rest.find(tag) {
                rest = &rest[i + tag.len()..];
                let token: String = rest
                    .trim_start()
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || matches!(c, '_' | '$' | '.'))
                    .collect();
                let first = token.split('.').next().unwrap_or("");
                if !first.is_empty() && first.chars().next().is_some_and(char::is_uppercase) {
                    names.push(first.to_string());
                }
            }
        }
        if names.is_empty() {
            return Ok(());
        }
        let ctx = self.ctx_at(n)?;
        for name in names {
            if let NameRes::Class(h, via) = self.env.resolve_type_name(&name, &ctx)? {
                self.mark(via, &h);
            }
        }
        Ok(())
    }

    fn annotation(&mut self, n: Node<'_>) -> Res<()> {
        let Some(name) = n.child_by_field_name("name") else {
            return Ok(());
        };
        let segs = scoped_segments(name);
        let names: Vec<String> = segs.iter().map(|s| self.txt(*s).to_string()).collect();
        self.resolve_type_chain(n, &segs, &names, true)
    }

    fn type_ref(&mut self, n: Node<'_>) -> Res<()> {
        // Declarations: the name of a type parameter is not a reference.
        if let Some(p) = n.parent()
            && p.kind() == "type_parameter"
            && p.child(0).is_some_and(|c| c.id() == n.id())
        {
            return Ok(());
        }
        let name = self.txt(n);
        if name == "var" {
            return Ok(());
        }
        self.resolve_type_chain(n, &[n], &[name.to_string()], false)
    }

    fn scoped_type_ref(&mut self, n: Node<'_>) -> Res<()> {
        let segs = scoped_segments(n);
        let names: Vec<String> = segs.iter().map(|s| self.txt(*s).to_string()).collect();
        self.resolve_type_chain(n, &segs, &names, false)
    }

    /// Chain of the ambient context at `at`: enclosing class hits (outermost first).
    fn chain_at(&mut self, at: Node<'_>) -> Res<Vec<ClassHit>> {
        let mut ancestors: Vec<Node<'_>> = Vec::new();
        let mut cur = at.parent();
        while let Some(a) = cur {
            ancestors.push(a);
            cur = a.parent();
        }
        ancestors.reverse();
        let mut chain: Vec<ClassHit> = Vec::new();
        let mut path: Vec<String> = Vec::new();
        let mut top: Option<usize> = None;
        let mut named_only = true;
        for a in ancestors {
            let k = a.kind();
            let is_body_of_anon = k == "class_body"
                && a.parent().is_some_and(|p| {
                    matches!(p.kind(), "object_creation_expression" | "enum_constant")
                });
            if kind_is_class(k) {
                // Only the body is inside the class scope (header, modifiers and type parameters are not).
                let in_body = a.child_by_field_name("body").is_some_and(|b| {
                    b.start_byte() <= at.start_byte() && at.end_byte() <= b.end_byte()
                });
                if !in_body {
                    // Keep the nesting bookkeeping consistent for nothing below this node.
                    continue;
                }
                let name = a
                    .child_by_field_name("name")
                    .map(|n| self.txt(n).to_string())
                    .unwrap_or_default();
                if named_only && top.is_none() && a.parent().is_some_and(|p| p.kind() == "program")
                {
                    if let Some(i) = self.file.stub.classes.iter().position(|c| c.name == name) {
                        top = Some(i);
                        chain.push(self.file.hit(i, Vec::new()));
                        continue;
                    }
                } else if named_only && top.is_some() {
                    // Member of a named class: must exist in the stub path.
                    let mut p2 = path.clone();
                    p2.push(name.clone());
                    let hit = self.file.hit(top.expect("some"), p2.clone());
                    if hit.record.class.inner_named_path(&p2).is_some() {
                        path = p2;
                        chain.push(hit);
                        continue;
                    }
                }
                // Local class (or a member of one).
                named_only = false;
                if let Some(h) = self.local_hits.get(&a.id()) {
                    chain.push(h.clone());
                    continue;
                }
                if let Some(stub) = source::class_stub_of(self.text, a, "") {
                    let h = self.make_local(&chain, a, stub)?;
                    self.local_hits.insert(a.id(), h.clone());
                    chain.push(h);
                }
            } else if is_body_of_anon {
                named_only = false;
                if let Some(h) = self.local_hits.get(&a.id()) {
                    chain.push(h.clone());
                    continue;
                }
                let base = self.anonymous_base(a);
                let stub = source::anonymous_stub(self.text, a, &base, false);
                let h = self.make_local(&chain, a, stub)?;
                self.local_hits.insert(a.id(), h.clone());
                chain.push(h);
            }
        }
        Ok(chain)
    }

    fn anonymous_base(&self, body: Node<'_>) -> String {
        let Some(p) = body.parent() else {
            return "java.lang.Object".into();
        };
        match p.kind() {
            "object_creation_expression" => p
                .child_by_field_name("type")
                .map(|t| source::Builder { src: self.text }.compact(t))
                .unwrap_or_else(|| "java.lang.Object".into()),
            "enum_constant" => p
                .parent()
                .and_then(|b| b.parent())
                .and_then(|e| e.child_by_field_name("name"))
                .map(|n| self.txt(n).to_string())
                .unwrap_or_else(|| "java.lang.Object".into()),
            _ => "java.lang.Object".into(),
        }
    }

    fn make_local(&mut self, chain: &[ClassHit], at: Node<'_>, stub: ClassStub) -> Res<ClassHit> {
        let top_idx = chain
            .first()
            .and_then(|h| {
                self.file
                    .stub
                    .classes
                    .iter()
                    .position(|c| c.qname == h.record.class.qname)
            })
            .unwrap_or(0);
        let mut hit = if self.file.stub.classes.is_empty() {
            return Ok(chain
                .last()
                .cloned()
                .expect("a class encloses a local class"));
        } else {
            self.file.hit(top_idx, Vec::new())
        };
        // Supertypes resolve at the declaration site.
        let ctx = Ctx {
            package: &self.file.stub.package,
            imports: &self.file.stub.imports,
            chain: chain.to_vec(),
            tparams: self.method_tparams(at),
            file_tops: self
                .file
                .stub
                .classes
                .iter()
                .map(|c| c.name.clone())
                .collect(),
        };
        let mut supers = Vec::new();
        let mut texts: Vec<String> = stub
            .extends
            .iter()
            .chain(&stub.implements)
            .cloned()
            .collect();
        match stub.kind {
            ClassKind::Enum => texts.push("java.lang.Enum".into()),
            ClassKind::Record => texts.push("java.lang.Record".into()),
            _ => {}
        }
        let mut complete = true;
        for t in &texts {
            match self.env.resolve_type_text(t, &ctx)? {
                TypeRes::Class(h) => supers.push(h),
                TypeRes::TypeVar(_) | TypeRes::Prim(_) => {}
                _ => complete = false,
            }
        }
        if let Some(o) = self.env.find_class("java.lang.Object")? {
            supers.push(o);
        }
        if !complete {
            // Recorded as an extra pseudo-supertype that cannot be resolved: callers see the
            // incompleteness through a missing `local_supers` marker; mimic with a poisoned stub.
            hit.local = Some(Arc::new(ClassStub {
                extends: stub.extends.clone(),
                ..stub.clone()
            }));
            hit.local_supers = None;
            // An unresolvable supertype is found again by `supertypes`, which marks incomplete.
            return Ok(hit);
        }
        hit.local = Some(Arc::new(stub));
        hit.local_supers = Some(Arc::new(supers));
        Ok(hit)
    }

    fn method_tparams(&self, at: Node<'_>) -> Vec<String> {
        let mut out = Vec::new();
        let mut cur = Some(at);
        while let Some(n) = cur {
            let class_header = kind_is_class(n.kind())
                && !n.child_by_field_name("body").is_some_and(|b| {
                    b.start_byte() <= at.start_byte() && at.end_byte() <= b.end_byte()
                });
            if (matches!(n.kind(), "method_declaration" | "constructor_declaration")
                || class_header)
                && let Some(tp) = n.child_by_field_name("type_parameters")
            {
                let mut c = tp.walk();
                for p in tp
                    .named_children(&mut c)
                    .filter(|p| p.kind() == "type_parameter")
                {
                    if let Some(f) = p.named_child(0) {
                        out.push(self.txt(f).to_string());
                    }
                }
            }
            cur = n.parent();
        }
        out
    }

    pub fn ctx_at(&mut self, at: Node<'_>) -> Res<Ctx<'a>> {
        let chain = self.chain_at(at)?;
        let file: &'a CurrentFile = self.file;
        Ok(Ctx {
            package: &file.stub.package,
            imports: &file.stub.imports,
            chain,
            tparams: self.method_tparams(at),
            file_tops: file.stub.classes.iter().map(|c| c.name.clone()).collect(),
        })
    }

    /// Whether a local class, record, interface or enum called `name` is declared before `at` in an
    /// enclosing block (JAVA-RES-006: local classes are scoped like local variables).
    fn local_type_declared(&self, at: Node<'_>, name: &str) -> bool {
        let mut child = at;
        let mut cur = at.parent();
        while let Some(a) = cur {
            if matches!(
                a.kind(),
                "block"
                    | "constructor_body"
                    | "switch_block"
                    | "switch_block_statement_group"
                    | "switch_rule"
            ) {
                let mut c = a.walk();
                for sib in a.children(&mut c) {
                    if sib.start_byte() >= child.start_byte() {
                        break;
                    }
                    if kind_is_class(sib.kind())
                        && sib
                            .child_by_field_name("name")
                            .is_some_and(|n| self.txt(n) == name)
                    {
                        return true;
                    }
                }
            }
            child = a;
            cur = a.parent();
        }
        false
    }

    /// Resolves a (qualified) type name; reports the first unresolved segment (JAVA-RES-005).
    fn resolve_type_chain(
        &mut self,
        at: Node<'_>,
        segs: &[Node<'_>],
        names: &[String],
        annotation: bool,
    ) -> Res<()> {
        if names.is_empty() {
            return Ok(());
        }
        if self.local_type_declared(at, &names[0]) {
            return Ok(());
        }
        let ctx = self.ctx_at(at)?;
        match self.env.resolve_type_name(&names[0], &ctx)? {
            NameRes::Class(mut h, via) => {
                self.mark(via, &h);
                for (k, seg) in names.iter().enumerate().skip(1) {
                    match self.env.member_type(&h, seg)? {
                        Some(m) => h = m,
                        None => {
                            if self.env.hierarchy(&h)?.1 {
                                self.push_symbol(segs[k], seg, true, false);
                            }
                            return Ok(());
                        }
                    }
                }
            }
            NameRes::TypeVar(_) | NameRes::Ambiguous(..) => {}
            NameRes::Incomplete => {
                self.unresolved(&names[0]);
                self.out.usage.unresolved_any = true;
            }
            NameRes::NotFound => {
                if names.len() == 1 {
                    self.out.usage.uncertain |= self.incomplete;
                    self.push_symbol(segs[0], &names[0], true, annotation);
                    return Ok(());
                }
                // Package-qualified: shortest prefix that is a class, else the first unknown segment.
                for i in 1..=names.len() {
                    if let Some(mut h) = self.env.find_class(&names[..i].join("."))? {
                        for (k, seg) in names.iter().enumerate().skip(i) {
                            match self.env.member_type(&h, seg)? {
                                Some(m) => h = m,
                                None => {
                                    self.push_symbol(segs[k], seg, true, false);
                                    return Ok(());
                                }
                            }
                        }
                        return Ok(());
                    }
                }
                let mut known = 0;
                for i in 1..names.len() {
                    if self.env.package_exists(&names[..i].join("."))? {
                        known = i;
                    } else {
                        break;
                    }
                }
                let at_seg = known.min(names.len() - 1);
                self.push_symbol(
                    segs[at_seg],
                    &names[at_seg],
                    at_seg + 1 == names.len(),
                    annotation && at_seg + 1 == names.len(),
                );
            }
        }
        Ok(())
    }

    // ---- variables ----------------------------------------------------------------------

    fn pattern_names(&mut self, boundary: Node<'_>) -> &HashSet<String> {
        let id = boundary.id();
        if !self.pattern_cache.contains_key(&id) {
            let mut set = HashSet::new();
            let mut stack = vec![boundary];
            while let Some(n) = stack.pop() {
                match n.kind() {
                    "type_pattern" => {
                        if let Some(last) = n.named_child(n.named_child_count().saturating_sub(1)) {
                            set.insert(self.txt(last).to_string());
                        }
                    }
                    "record_pattern_component" => {
                        if let Some(last) = n.named_child(n.named_child_count().saturating_sub(1))
                            && last.kind() == "identifier"
                        {
                            set.insert(self.txt(last).to_string());
                        }
                    }
                    "instanceof_expression" => {
                        if let Some(nm) = n.child_by_field_name("name") {
                            set.insert(self.txt(nm).to_string());
                        }
                    }
                    _ => {}
                }
                let mut c = n.walk();
                for ch in n.children(&mut c) {
                    stack.push(ch);
                }
            }
            self.pattern_cache.insert(id, set);
        }
        &self.pattern_cache[&id]
    }

    fn declarators(&self, decl: Node<'_>, name: &str) -> Option<VarRes> {
        let ty = decl
            .child_by_field_name("type")
            .map(|t| source::Builder { src: self.text }.compact(t));
        let mut c = decl.walk();
        for d in decl.children_by_field_name("declarator", &mut c) {
            if d.child_by_field_name("name")
                .is_some_and(|n| self.txt(n) == name)
            {
                let dims = d
                    .child_by_field_name("dimensions")
                    .map(|x| "[]".repeat(self.txt(x).matches('[').count()))
                    .unwrap_or_default();
                let tt = ty
                    .clone()
                    .filter(|t| t != "var")
                    .map(|t| format!("{t}{dims}"));
                return Some(VarRes::Local {
                    type_text: tt,
                    init: d.child_by_field_name("value").map(|v| v.id()),
                    node_start: d.start_byte(),
                });
            }
        }
        None
    }

    /// Looks `name` up as a variable from `at` outwards: locals and parameters, then fields of the
    /// enclosing classes (JAVA-RES-006, 007), then static imports (JAVA-RES-021).
    pub fn resolve_variable(&mut self, at: Node<'_>, name: &str) -> Res<VarRes> {
        let mut chain = self.chain_at(at)?;
        let mut incomplete = false;
        let mut child = at;
        let mut cur = at.parent();
        while let Some(a) = cur {
            let k = a.kind();
            match k {
                "block"
                | "constructor_body"
                | "switch_block"
                | "switch_block_statement_group"
                | "switch_rule" => {
                    let mut c = a.walk();
                    for sib in a.children(&mut c) {
                        if sib.start_byte() >= child.start_byte() {
                            break;
                        }
                        if sib.kind() == "local_variable_declaration"
                            && let Some(v) = self.declarators(sib, name)
                        {
                            return Ok(v);
                        }
                    }
                }
                "for_statement" => {
                    if let Some(init) = a.child_by_field_name("init")
                        && init.kind() == "local_variable_declaration"
                        && let Some(v) = self.declarators(init, name)
                    {
                        return Ok(v);
                    }
                }
                "enhanced_for_statement" => {
                    if a.child_by_field_name("name")
                        .is_some_and(|n| self.txt(n) == name)
                        && a.child_by_field_name("body")
                            .is_some_and(|b| b.id() == child.id())
                    {
                        let t = a
                            .child_by_field_name("type")
                            .map(|t| source::Builder { src: self.text }.compact(t))
                            .filter(|t| t != "var");
                        return Ok(VarRes::Local {
                            type_text: t,
                            init: None,
                            node_start: a.start_byte(),
                        });
                    }
                }
                "try_with_resources_statement" => {
                    if let Some(rs) = a.child_by_field_name("resources") {
                        let mut c = rs.walk();
                        for r in rs.named_children(&mut c).filter(|r| r.kind() == "resource") {
                            if r.child_by_field_name("name")
                                .is_some_and(|n| self.txt(n) == name)
                                && r.start_byte() < child.start_byte()
                            {
                                let t = r
                                    .child_by_field_name("type")
                                    .map(|t| source::Builder { src: self.text }.compact(t))
                                    .filter(|t| t != "var");
                                return Ok(VarRes::Local {
                                    type_text: t,
                                    init: r.child_by_field_name("value").map(|v| v.id()),
                                    node_start: r.start_byte(),
                                });
                            }
                        }
                    }
                }
                "catch_clause" => {
                    let mut c = a.walk();
                    for p in a
                        .children(&mut c)
                        .filter(|p| p.kind() == "catch_formal_parameter")
                    {
                        if p.child_by_field_name("name")
                            .is_some_and(|n| self.txt(n) == name)
                        {
                            let t = p
                                .children(&mut p.walk())
                                .find(|x| x.kind() == "catch_type")
                                .map(|t| self.txt(t).to_string());
                            return Ok(VarRes::Local {
                                type_text: t.filter(|t| !t.contains('|')),
                                init: None,
                                node_start: p.start_byte(),
                            });
                        }
                    }
                }
                "lambda_expression" => {
                    if let Some(ps) = a.child_by_field_name("parameters") {
                        let found = match ps.kind() {
                            "identifier" => (self.txt(ps) == name).then_some(None),
                            "inferred_parameters" => {
                                let mut c = ps.walk();
                                let hit = ps.named_children(&mut c).any(|i| self.txt(i) == name);
                                hit.then_some(None)
                            }
                            _ => self.param_named(ps, name).map(Some),
                        };
                        if let Some(t) = found {
                            return Ok(VarRes::Local {
                                type_text: t.flatten(),
                                init: None,
                                node_start: a.start_byte(),
                            });
                        }
                    }
                    if self.pattern_names(a).contains(name) {
                        return Ok(VarRes::Local {
                            type_text: None,
                            init: None,
                            node_start: a.start_byte(),
                        });
                    }
                }
                "method_declaration" | "constructor_declaration" => {
                    if let Some(ps) = a.child_by_field_name("parameters")
                        && let Some(t) = self.param_named(ps, name)
                    {
                        return Ok(VarRes::Local {
                            type_text: t,
                            init: None,
                            node_start: a.start_byte(),
                        });
                    }
                    if self.pattern_names(a).contains(name) {
                        return Ok(VarRes::Local {
                            type_text: None,
                            init: None,
                            node_start: a.start_byte(),
                        });
                    }
                }
                "static_initializer" | "field_declaration" | "compact_constructor_declaration"
                    if self.pattern_names(a).contains(name) =>
                {
                    return Ok(VarRes::Local {
                        type_text: None,
                        init: None,
                        node_start: a.start_byte(),
                    });
                }
                _ => {}
            }
            let is_anon_body = k == "class_body"
                && a.parent().is_some_and(|p| {
                    matches!(p.kind(), "object_creation_expression" | "enum_constant")
                });
            if (kind_is_class(k) || is_anon_body)
                && let Some(h) = chain.pop()
            {
                let (fields, complete, comp) = self.env.fields_named(&h, name)?;
                if let Some((owner, index)) = fields.into_iter().next() {
                    return Ok(VarRes::Field { owner, index });
                }
                if comp {
                    return Ok(VarRes::Component { owner: h, index: 0 });
                }
                incomplete |= !complete;
            }
            child = a;
            cur = a.parent();
        }
        // Static imports: single, then on demand.
        let imports = self.file.stub.imports.clone();
        for (i, imp) in imports.iter().enumerate() {
            if !imp.is_static {
                continue;
            }
            if !imp.on_demand && imp.name.rsplit('.').next() == Some(name) {
                let Some((cls, _)) = imp.name.rsplit_once('.') else {
                    continue;
                };
                match self.env.find_class(cls)? {
                    Some(h) => {
                        let (fields, _, _) = self.env.fields_named(&h, name)?;
                        if let Some((owner, idx)) = fields.into_iter().next() {
                            let ty = owner.class().fields[idx].ty.clone();
                            return Ok(VarRes::StaticImport {
                                class: owner,
                                import: i,
                                ty: Some(ty),
                            });
                        }
                    }
                    None => incomplete = true,
                }
            }
        }
        for (i, imp) in imports.iter().enumerate() {
            if imp.is_static && imp.on_demand {
                match self.env.find_class(&imp.name)? {
                    Some(h) => {
                        let (fields, complete, _) = self.env.fields_named(&h, name)?;
                        if let Some((owner, idx)) = fields.into_iter().find(|(o, i)| {
                            o.class().fields[*i].flags & flags::STATIC != 0
                                || o.class().is_interface_like()
                        }) {
                            let ty = owner.class().fields[idx].ty.clone();
                            return Ok(VarRes::StaticImport {
                                class: owner,
                                import: i,
                                ty: Some(ty),
                            });
                        }
                        incomplete |= !complete;
                    }
                    None => incomplete = true,
                }
            }
        }
        Ok(VarRes::NotFound { incomplete })
    }

    /// `Some(type)` when `params` declares `name` (type `None` for untyped lambda parameters).
    fn param_named(&self, params: Node<'_>, name: &str) -> Option<Option<String>> {
        let mut c = params.walk();
        for p in params.named_children(&mut c) {
            match p.kind() {
                "formal_parameter" | "receiver_parameter" => {
                    if p.child_by_field_name("name")
                        .is_some_and(|n| self.txt(n) == name)
                    {
                        let dims = p
                            .child_by_field_name("dimensions")
                            .map(|d| "[]".repeat(self.txt(d).matches('[').count()))
                            .unwrap_or_default();
                        let t = p
                            .child_by_field_name("type")
                            .map(|t| {
                                format!("{}{dims}", source::Builder { src: self.text }.compact(t))
                            })
                            .filter(|t| t != "var");
                        return Some(t);
                    }
                }
                "spread_parameter" => {
                    let mut pc = p.walk();
                    let declared = p
                        .named_children(&mut pc)
                        .find(|x| x.kind() == "variable_declarator")
                        .and_then(|d| d.child_by_field_name("name"))
                        .is_some_and(|n| self.txt(n) == name);
                    if declared {
                        let t = p
                            .named_children(&mut p.walk())
                            .find(|x| !matches!(x.kind(), "modifiers" | "variable_declarator"))
                            .map(|t| {
                                format!("{}[]", source::Builder { src: self.text }.compact(t))
                            });
                        return Some(t);
                    }
                }
                _ => {}
            }
        }
        None
    }

    // ---- names, chains, calls ---------------------------------------------------------------

    /// A standalone or leftmost expression name.
    fn simple_name(&mut self, n: Node<'_>) -> Res<()> {
        let name = self.txt(n).to_string();
        // A leftmost qualifier may also be a type or a package; a standalone name is a variable.
        let is_qualifier = n.parent().is_some_and(|p| {
            matches!(p.kind(), "field_access" | "method_invocation")
                && p.child_by_field_name("object")
                    .is_some_and(|o| o.id() == n.id())
        });
        match self.resolve_variable(n, &name)? {
            VarRes::NotFound { incomplete } => {
                if self.local_type_declared(n, &name) {
                    return Ok(());
                }
                let ctx = self.ctx_at(n)?;
                match self.env.resolve_type_name(&name, &ctx)? {
                    NameRes::Class(h, via) => {
                        self.mark(via, &h);
                        return Ok(());
                    }
                    NameRes::TypeVar(_) | NameRes::Ambiguous(..) => return Ok(()),
                    NameRes::Incomplete => {
                        self.unresolved(&name);
                        self.out.usage.unresolved_any = true;
                        return Ok(());
                    }
                    NameRes::NotFound => {}
                }
                if is_qualifier && self.env.package_exists(&name)? {
                    return Ok(());
                }
                if incomplete || self.lombok {
                    self.unresolved(&name);
                    self.out.usage.unresolved_any = true;
                    return Ok(());
                }
                self.out.usage.uncertain |= self.incomplete;
                self.push_symbol(n, &name, false, false);
            }
            VarRes::StaticImport { import, .. } => self.mark_static(import, name),
            _ => {}
        }
        Ok(())
    }

    fn field_access(&mut self, n: Node<'_>) -> Res<()> {
        // Flatten a chain of plain names `a.b.c`.
        let mut segs: Vec<Node<'_>> = Vec::new();
        let mut cur = n;
        loop {
            let Some(field) = cur.child_by_field_name("field") else {
                return self.visit_children(n);
            };
            if field.kind() != "identifier" {
                return self.visit_children(n); // `A.this`
            }
            segs.push(field);
            let Some(obj) = cur.child_by_field_name("object") else {
                return self.visit_children(n);
            };
            match obj.kind() {
                "field_access" => cur = obj,
                "identifier" => {
                    segs.push(obj);
                    break;
                }
                _ => {
                    // Qualifier is a general expression: check it, then the member when the type is known.
                    self.visit(obj)?;
                    segs.reverse();
                    return self.member_chain_after_expr(obj, &segs);
                }
            }
        }
        segs.reverse();
        self.name_chain(n, &segs, None)
    }

    /// `a.b.c` where every part is a plain name. With `call` set, the last segment is a method name.
    fn name_chain(
        &mut self,
        at: Node<'_>,
        segs: &[Node<'_>],
        call: Option<(Node<'_>, Vec<Ty>)>,
    ) -> Res<()> {
        let names: Vec<String> = segs.iter().map(|s| self.txt(*s).to_string()).collect();
        let last_is_method = call.is_some();
        let qual_len = if last_is_method {
            names.len() - 1
        } else {
            names.len()
        };
        // Head classification (JLS 6.5.2): variable, then type, then package.
        let head = &names[0];
        let mut cls: Option<ClassHit> = None;
        let mut next = 1;
        let mut head_ty: Option<Ty> = None;
        match self.resolve_variable(segs[0], head)? {
            VarRes::Local {
                type_text, init, ..
            } => {
                head_ty = Some(match type_text {
                    Some(t) => self.ty_of_text(&t, segs[0])?,
                    None => match init.and_then(|id| find_by_id(self.tree.root_node(), id)) {
                        Some(init) => self.expr_ty(init)?,
                        None => Ty::Unknown,
                    },
                });
            }
            VarRes::Field { owner, index } => {
                let t = owner.class().fields[index].ty.clone();
                head_ty = Some(self.ty_in_class(&t, &owner)?);
            }
            VarRes::Component { owner, .. } => {
                let c = owner.class();
                let t = c
                    .record_components
                    .iter()
                    .find(|p| &p.name == head)
                    .map(|p| p.ty.clone());
                head_ty = Some(match t {
                    Some(t) => self.ty_in_class(&t, &owner)?,
                    None => Ty::Unknown,
                });
            }
            VarRes::StaticImport { import, ty, class } => {
                self.mark_static(import, head.clone());
                head_ty = Some(match ty {
                    Some(t) => self.ty_in_class(&t, &class)?,
                    None => Ty::Unknown,
                });
            }
            VarRes::NotFound { incomplete } => {
                if self.local_type_declared(segs[0], head) {
                    return Ok(());
                }
                let ctx = self.ctx_at(segs[0])?;
                match self.env.resolve_type_name(head, &ctx)? {
                    NameRes::Class(h, via) => {
                        self.mark(via, &h);
                        cls = Some(h);
                    }
                    NameRes::Incomplete | NameRes::Ambiguous(..) | NameRes::TypeVar(_) => {
                        self.unresolved(head);
                        self.out.usage.unresolved_any = true;
                        return Ok(());
                    }
                    NameRes::NotFound => {
                        // Package prefix: the shortest prefix that names a class.
                        let mut found = None;
                        for i in 2..=names.len() {
                            if let Some(h) = self.env.find_class(&names[..i].join("."))? {
                                found = Some((i, h));
                                break;
                            }
                        }
                        match found {
                            Some((i, h)) => {
                                cls = Some(h);
                                next = i;
                            }
                            None => {
                                if incomplete || self.lombok {
                                    self.unresolved(head);
                                    self.out.usage.unresolved_any = true;
                                    return Ok(());
                                }
                                let mut known = 0;
                                for i in 1..names.len() {
                                    if self.env.package_exists(&names[..i].join("."))? {
                                        known = i;
                                    } else {
                                        break;
                                    }
                                }
                                if known == 0 && qual_len == 1 && call.is_none() {
                                    // `a.b`: unknown head.
                                }
                                let at_seg = known.min(qual_len.saturating_sub(1));
                                let _ = at_seg;
                                // An unknown leftmost name is reported on the name; a known package
                                // prefix moves the error to the next segment (JAVA-RES-027).
                                let idx = known.min(names.len() - 1);
                                if last_is_method && idx == names.len() - 1 {
                                    // `foo.bar()` where `foo` is nothing: report `foo`.
                                    self.push_symbol(segs[0], &names[0], false, false);
                                } else {
                                    self.push_symbol(segs[idx], &names[idx], false, false);
                                }
                                self.out.usage.uncertain |= self.incomplete;
                                return Ok(());
                            }
                        }
                    }
                }
            }
        }
        // Walk the remaining qualifier segments.
        let mut i = next;
        while i < qual_len {
            match (&cls, &head_ty) {
                (Some(h), _) => {
                    let seg = &names[i];
                    if let Some(m) = self.env.member_type(h, seg)? {
                        cls = Some(m);
                    } else {
                        let (fs, complete, comp) = self.env.fields_named(h, seg)?;
                        if let Some((owner, idx)) = fs.into_iter().next() {
                            let t = owner.class().fields[idx].ty.clone();
                            head_ty = Some(self.ty_in_class(&t, &owner)?);
                            cls = None;
                        } else if complete && !comp && !self.lombok {
                            self.push_symbol(segs[i], seg, false, false);
                            return Ok(());
                        } else {
                            return Ok(());
                        }
                    }
                }
                (None, Some(t)) => {
                    // Field of a value of known type.
                    let seg = &names[i];
                    match self.field_type_of(t, seg)? {
                        FieldLookup::Found(nt) => head_ty = Some(nt),
                        FieldLookup::Missing => {
                            self.push_symbol(segs[i], seg, false, false);
                            return Ok(());
                        }
                        FieldLookup::Unknown => return Ok(()),
                    }
                }
                _ => return Ok(()),
            }
            i += 1;
        }
        if let Some((call_node, args)) = call {
            let mname = names.last().cloned().unwrap_or_default();
            let qualifier = match (&cls, &head_ty) {
                (Some(h), _) => Qualifier::Class(h.clone()),
                (None, Some(t)) => Qualifier::Value(t.clone()),
                _ => Qualifier::Unknown,
            };
            self.check_qualified_call(
                at,
                call_node,
                segs[segs.len() - 1],
                &mname,
                &args,
                qualifier,
            )?;
        }
        Ok(())
    }

    fn member_chain_after_expr(&mut self, obj: Node<'_>, segs: &[Node<'_>]) -> Res<()> {
        // `expr.f1.f2`: report a missing field only when the type of `expr` is known.
        let mut t = self.expr_ty(obj)?;
        for s in segs {
            let name = self.txt(*s).to_string();
            match self.field_type_of(&t, &name)? {
                FieldLookup::Found(nt) => t = nt,
                FieldLookup::Missing => {
                    self.push_symbol(*s, &name, false, false);
                    return Ok(());
                }
                FieldLookup::Unknown => return Ok(()),
            }
        }
        Ok(())
    }

    fn field_type_of(&mut self, t: &Ty, name: &str) -> Res<FieldLookup> {
        match t {
            Ty::Array(_) => Ok(if name == "length" {
                FieldLookup::Found(Ty::Prim("int".into()))
            } else {
                FieldLookup::Unknown
            }),
            Ty::Ref { fqn: Some(fqn), .. } => {
                let Some(h) = self.env.find_class(fqn)? else {
                    return Ok(FieldLookup::Unknown);
                };
                let (fs, complete, comp) = self.env.fields_named(&h, name)?;
                if let Some((owner, idx)) = fs.into_iter().next() {
                    let ft = owner.class().fields[idx].ty.clone();
                    return Ok(FieldLookup::Found(self.ty_in_class(&ft, &owner)?));
                }
                if comp {
                    return Ok(FieldLookup::Unknown);
                }
                Ok(if complete && !self.lombok {
                    FieldLookup::Missing
                } else {
                    FieldLookup::Unknown
                })
            }
            _ => Ok(FieldLookup::Unknown),
        }
    }

    fn method_reference(&mut self, n: Node<'_>) -> Res<()> {
        // `Qualifier::name`: only the qualifier is checked.
        let mut c = n.walk();
        let first = n.children(&mut c).next();
        if let Some(f) = first {
            self.visit(f)?;
        }
        Ok(())
    }

    fn object_creation(&mut self, n: Node<'_>) -> Res<()> {
        // `outer.new Inner()`: the type is resolved relative to the outer instance: skip it.
        let has_outer = n.child(0).is_some_and(|c| {
            c.kind() == "object_creation_expression"
                || c.kind() == "identifier"
                || c.kind() == "field_access"
                || c.kind() == "method_invocation"
        });
        let mut c = n.walk();
        let kids: Vec<Node<'_>> = n.children(&mut c).collect();
        for k in kids {
            if has_outer && k.id() == n.child_by_field_name("type").map(|t| t.id()).unwrap_or(0) {
                continue;
            }
            self.visit(k)?;
        }
        Ok(())
    }

    fn method_invocation(&mut self, n: Node<'_>) -> Res<()> {
        let Some(name_node) = n.child_by_field_name("name") else {
            return self.visit_children(n);
        };
        let name = self.txt(name_node).to_string();
        let args_node = n.child_by_field_name("arguments");
        // Arguments are references too.
        if let Some(a) = args_node {
            self.visit_children(a)?;
        }
        let object = n.child_by_field_name("object");
        let args: Vec<Ty> = match args_node {
            Some(a) => {
                let mut v = Vec::new();
                let mut c = a.walk();
                for arg in a
                    .named_children(&mut c)
                    .filter(|x| !matches!(x.kind(), "line_comment" | "block_comment"))
                {
                    v.push(self.expr_ty(arg)?);
                }
                v
            }
            None => Vec::new(),
        };
        if n.has_error() {
            return Ok(());
        }
        match object {
            None => self.unqualified_call(n, name_node, &name, &args),
            Some(obj) => {
                // Type arguments after the dot (`List.<String>of()`) are references.
                let mut c = n.walk();
                let targs: Vec<Node<'_>> = n
                    .children(&mut c)
                    .filter(|x| x.kind() == "type_arguments")
                    .collect();
                for t in targs {
                    self.visit(t)?;
                }
                match obj.kind() {
                    "identifier" => self.name_chain(n, &[obj, name_node], Some((n, args))),
                    "field_access" => {
                        let mut segs: Vec<Node<'_>> = Vec::new();
                        let mut cur = obj;
                        let mut plain = true;
                        loop {
                            let Some(f) = cur.child_by_field_name("field") else {
                                plain = false;
                                break;
                            };
                            if f.kind() != "identifier" {
                                plain = false;
                                break;
                            }
                            segs.push(f);
                            match cur.child_by_field_name("object") {
                                Some(o) if o.kind() == "field_access" => cur = o,
                                Some(o) if o.kind() == "identifier" => {
                                    segs.push(o);
                                    break;
                                }
                                _ => {
                                    plain = false;
                                    break;
                                }
                            }
                        }
                        if plain {
                            segs.reverse();
                            segs.push(name_node);
                            self.name_chain(n, &segs, Some((n, args)))
                        } else {
                            self.visit(obj)?;
                            let t = self.expr_ty(obj)?;
                            self.check_qualified_call(
                                n,
                                n,
                                name_node,
                                &name,
                                &args,
                                Qualifier::Value(t),
                            )
                        }
                    }
                    "super" => Ok(()),
                    _ => {
                        self.visit(obj)?;
                        let t = self.expr_ty(obj)?;
                        self.check_qualified_call(
                            n,
                            n,
                            name_node,
                            &name,
                            &args,
                            Qualifier::Value(t),
                        )
                    }
                }
            }
        }
    }

    fn unqualified_call(
        &mut self,
        call: Node<'_>,
        name_node: Node<'_>,
        name: &str,
        args: &[Ty],
    ) -> Res<()> {
        let chain = self.chain_at(call)?;
        let mut incomplete = false;
        for h in chain.iter().rev() {
            let (ms, complete) = self.env.methods_named(h, name)?;
            incomplete |= !complete;
            if !ms.is_empty() || self.env.has_synthetic_method(h, name) {
                return Ok(()); // candidates exist at this level (JAVA-RES-011): no zero-candidate error
            }
        }
        // Static imports.
        let imports = self.file.stub.imports.clone();
        for (i, imp) in imports.iter().enumerate() {
            if !imp.is_static {
                continue;
            }
            if imp.on_demand {
                match self.env.find_class(&imp.name)? {
                    Some(h) => {
                        let (ms, complete) = self.env.methods_named(&h, name)?;
                        incomplete |= !complete;
                        if !ms.is_empty() {
                            self.mark_static(i, name.to_string());
                            return Ok(());
                        }
                    }
                    None => incomplete = true,
                }
            } else if imp.name.rsplit('.').next() == Some(name) {
                let Some((cls, _)) = imp.name.rsplit_once('.') else {
                    continue;
                };
                match self.env.find_class(cls)? {
                    Some(h) => {
                        let (ms, complete) = self.env.methods_named(&h, name)?;
                        incomplete |= !complete;
                        if !ms.is_empty() {
                            self.mark_static(i, name.to_string());
                            return Ok(());
                        }
                    }
                    None => incomplete = true,
                }
            }
        }
        if incomplete || self.lombok || chain.is_empty() {
            self.unresolved(name);
            self.out.usage.unresolved_any = true;
            return Ok(());
        }
        if args.iter().any(|a| matches!(a, Ty::Unknown)) && false {
            return Ok(());
        }
        self.push_method(call, name_node, name, args, false);
        Ok(())
    }

    fn push_method(
        &mut self,
        call: Node<'_>,
        name_node: Node<'_>,
        name: &str,
        args: &[Ty],
        qualified: bool,
    ) {
        self.unresolved(name);
        if self.incomplete {
            self.out.problems.push(Problem {
                range: range(name_node),
                message: PENDING_MESSAGE.into(),
                kind: ProblemKind::Pending,
            });
            self.out.usage.uncertain = true;
            return;
        }
        let list: Vec<String> = args.iter().map(Ty::display).collect();
        self.out.problems.push(Problem {
            range: range(name_node),
            message: format!("Cannot resolve method '{name}({})'", list.join(", ")),
            kind: ProblemKind::Method {
                name: name.to_string(),
                args: args.to_vec(),
                qualified,
                call: range(call),
            },
        });
    }

    fn check_qualified_call(
        &mut self,
        call: Node<'_>,
        _call2: Node<'_>,
        name_node: Node<'_>,
        name: &str,
        args: &[Ty],
        q: Qualifier,
    ) -> Res<()> {
        let (hit, is_type) = match &q {
            Qualifier::Class(h) => (h.clone(), true),
            Qualifier::Value(Ty::Ref { fqn: Some(fqn), .. }) => match self.env.find_class(fqn)? {
                Some(h) => (h, false),
                None => return Ok(()),
            },
            _ => return Ok(()),
        };
        let (ms, complete) = self.env.methods_named(&hit, name)?;
        if !ms.is_empty() || self.env.has_synthetic_method(&hit, name) || !complete || self.lombok {
            return Ok(());
        }
        // Annotation-processor generated and `Object` members are covered by the hierarchy.
        if self.incomplete {
            self.push_method(call, name_node, name, args, true);
            return Ok(());
        }
        let list: Vec<String> = args.iter().map(Ty::display).collect();
        let sig = format!("{name}({})", list.join(", "));
        let message = if is_type {
            format!("Cannot resolve method '{sig}' in '{}'", hit.class().name)
        } else {
            format!("Cannot resolve method '{sig}'")
        };
        self.out.problems.push(Problem {
            range: range(name_node),
            message,
            kind: ProblemKind::Method {
                name: name.to_string(),
                args: args.to_vec(),
                qualified: true,
                call: range(call),
            },
        });
        Ok(())
    }

    // ---- expression types -------------------------------------------------------------------

    pub fn ty_of_text(&mut self, text: &str, at: Node<'_>) -> Res<Ty> {
        let ctx = self.ctx_at(at)?;
        self.ty_from_ctx(text, &ctx)
    }

    fn ty_from_ctx(&self, text: &str, ctx: &Ctx<'_>) -> Res<Ty> {
        let parts = ty::split(text);
        let base = match self.env.resolve_type_text(&parts.base, ctx)? {
            TypeRes::Class(h) => {
                let shown = ty::display(text.trim_end_matches("[]").trim_end_matches("..."));
                Ty::Ref {
                    fqn: Some(h.class().qname.clone()),
                    display: shown,
                }
            }
            TypeRes::Prim(p) => Ty::Prim(p),
            TypeRes::TypeVar(v) => Ty::Var(v),
            _ => {
                if parts.base == "var" {
                    return Ok(Ty::Unknown);
                }
                Ty::Ref {
                    fqn: None,
                    display: ty::display(&parts.base),
                }
            }
        };
        let mut t = base;
        for _ in 0..parts.dims {
            t = Ty::Array(Box::new(t));
        }
        Ok(t)
    }

    /// A type text as written inside a declaration of `owner` (a field or return type).
    pub fn ty_in_class(&self, text: &str, owner: &ClassHit) -> Res<Ty> {
        let ctx = self.env.ctx_of(owner);
        let t = self.ty_from_ctx(text, &ctx)?;
        Ok(match t {
            Ty::Var(_) => Ty::Unknown,
            other => other,
        })
    }

    pub fn expr_ty(&mut self, n: Node<'_>) -> Res<Ty> {
        Ok(match n.kind() {
            "string_literal" | "text_block" => Ty::string(),
            "decimal_integer_literal"
            | "hex_integer_literal"
            | "octal_integer_literal"
            | "binary_integer_literal" => {
                let t = self.txt(n);
                if t.ends_with('l') || t.ends_with('L') {
                    Ty::Prim("long".into())
                } else {
                    Ty::Prim("int".into())
                }
            }
            "decimal_floating_point_literal" | "hex_floating_point_literal" => {
                let t = self.txt(n);
                if t.ends_with('f') || t.ends_with('F') {
                    Ty::Prim("float".into())
                } else {
                    Ty::Prim("double".into())
                }
            }
            "character_literal" => Ty::Prim("char".into()),
            "true" | "false" => Ty::Prim("boolean".into()),
            "null_literal" => Ty::Null,
            "parenthesized_expression" => match n.named_child(0) {
                Some(c) => self.expr_ty(c)?,
                None => Ty::Unknown,
            },
            "cast_expression" => match n.child_by_field_name("type") {
                Some(t) => {
                    let text = source::Builder { src: self.text }.compact(t);
                    self.ty_of_text(&text, n)?
                }
                None => Ty::Unknown,
            },
            "object_creation_expression" => match n.child_by_field_name("type") {
                Some(t) => {
                    let text = source::Builder { src: self.text }.compact(t);
                    if text.contains("<>") {
                        let base = text.replace("<>", "");
                        let t = self.ty_of_text(&base, n)?;
                        match t {
                            Ty::Ref { fqn, display } => Ty::Ref {
                                fqn,
                                display: display.to_string(),
                            },
                            o => o,
                        }
                    } else {
                        self.ty_of_text(&text, n)?
                    }
                }
                None => Ty::Unknown,
            },
            "array_creation_expression" => match n.child_by_field_name("type") {
                Some(t) => {
                    let text = source::Builder { src: self.text }.compact(t);
                    let mut t = self.ty_of_text(&text, n)?;
                    let mut dims = 0;
                    let mut c = n.walk();
                    for d in n.children(&mut c) {
                        if matches!(d.kind(), "dimensions" | "dimensions_expr") {
                            dims += self.txt(d).matches('[').count();
                        }
                    }
                    for _ in 0..dims.max(1) {
                        t = Ty::Array(Box::new(t));
                    }
                    t
                }
                None => Ty::Unknown,
            },
            "binary_expression" => {
                let op = n
                    .child_by_field_name("operator")
                    .map(|o| self.txt(o))
                    .unwrap_or("");
                match op {
                    "==" | "!=" | "<" | ">" | "<=" | ">=" | "&&" | "||" => {
                        Ty::Prim("boolean".into())
                    }
                    "+" | "-" | "*" | "/" | "%" | "&" | "|" | "^" | "<<" | ">>" | ">>>" => {
                        let l = match n.child_by_field_name("left") {
                            Some(x) => self.expr_ty(x)?,
                            None => Ty::Unknown,
                        };
                        let r = match n.child_by_field_name("right") {
                            Some(x) => self.expr_ty(x)?,
                            None => Ty::Unknown,
                        };
                        if op == "+" && (l == Ty::string() || r == Ty::string()) {
                            Ty::string()
                        } else {
                            numeric_promotion(&l, &r)
                        }
                    }
                    _ => Ty::Unknown,
                }
            }
            "instanceof_expression" => Ty::Prim("boolean".into()),
            "unary_expression" => match n.child_by_field_name("operand") {
                Some(o) => {
                    let op = n.child(0).map(|c| self.txt(c)).unwrap_or("");
                    if op == "!" {
                        Ty::Prim("boolean".into())
                    } else {
                        self.expr_ty(o)?
                    }
                }
                None => Ty::Unknown,
            },
            "ternary_expression" => {
                let a = match n.child_by_field_name("consequence") {
                    Some(x) => self.expr_ty(x)?,
                    None => Ty::Unknown,
                };
                let b = match n.child_by_field_name("alternative") {
                    Some(x) => self.expr_ty(x)?,
                    None => Ty::Unknown,
                };
                if a == b { a } else { Ty::Unknown }
            }
            "this" => {
                let chain = self.chain_at(n)?;
                match chain.last() {
                    Some(h) if h.local.is_none() => Ty::Ref {
                        fqn: Some(h.class().qname.clone()),
                        display: h.class().name.clone(),
                    },
                    _ => Ty::Unknown,
                }
            }
            "array_access" => match n.child_by_field_name("array") {
                Some(a) => match self.expr_ty(a)? {
                    Ty::Array(e) => *e,
                    _ => Ty::Unknown,
                },
                None => Ty::Unknown,
            },
            "identifier" => {
                let name = self.txt(n).to_string();
                match self.resolve_variable(n, &name)? {
                    VarRes::Local {
                        type_text: Some(t), ..
                    } => self.ty_of_text(&t, n)?,
                    VarRes::Local {
                        init: Some(id),
                        type_text: None,
                        node_start,
                    } => {
                        // `var x = init;` (guard against self reference)
                        match find_by_id(self.tree.root_node(), id) {
                            Some(init)
                                if init.start_byte() != node_start
                                    && init.start_byte() < n.start_byte() =>
                            {
                                self.expr_ty(init)?
                            }
                            _ => Ty::Unknown,
                        }
                    }
                    VarRes::Field { owner, index } => {
                        let t = owner.class().fields[index].ty.clone();
                        self.ty_in_class(&t, &owner)?
                    }
                    VarRes::StaticImport {
                        ty: Some(t), class, ..
                    } => self.ty_in_class(&t, &class)?,
                    _ => Ty::Unknown,
                }
            }
            "method_invocation" => self.call_ty(n)?,
            _ => Ty::Unknown,
        })
    }

    /// The return type of a call when its candidates agree on it (no inference).
    fn call_ty(&mut self, n: Node<'_>) -> Res<Ty> {
        let Some(name) = n
            .child_by_field_name("name")
            .map(|x| self.txt(x).to_string())
        else {
            return Ok(Ty::Unknown);
        };
        let arity = n
            .child_by_field_name("arguments")
            .map_or(0, |a| a.named_child_count());
        let hits: Vec<(ClassHit, usize)> = match n.child_by_field_name("object") {
            None => {
                let chain = self.chain_at(n)?;
                let mut found = Vec::new();
                for h in chain.iter().rev() {
                    let (ms, _) = self.env.methods_named(h, &name)?;
                    if !ms.is_empty() {
                        found = ms;
                        break;
                    }
                }
                found
            }
            Some(obj) => {
                let t = if obj.kind() == "identifier" {
                    // Class or variable.
                    let oname = self.txt(obj).to_string();
                    match self.resolve_variable(obj, &oname)? {
                        VarRes::NotFound { .. } => {
                            let ctx = self.ctx_at(obj)?;
                            match self.env.resolve_type_name(&oname, &ctx)? {
                                NameRes::Class(h, _) => Ty::Ref {
                                    fqn: Some(h.class().qname.clone()),
                                    display: h.class().name.clone(),
                                },
                                _ => Ty::Unknown,
                            }
                        }
                        _ => self.expr_ty(obj)?,
                    }
                } else {
                    self.expr_ty(obj)?
                };
                match t {
                    Ty::Ref { fqn: Some(f), .. } => match self.env.find_class(&f)? {
                        Some(h) => self.env.methods_named(&h, &name)?.0,
                        None => Vec::new(),
                    },
                    _ => Vec::new(),
                }
            }
        };
        let fits: Vec<&(ClassHit, usize)> = hits
            .iter()
            .filter(|(h, i)| {
                let m = &h.class().methods[*i];
                m.params.len() == arity || (m.varargs && arity + 1 >= m.params.len())
            })
            .collect();
        let Some((h0, i0)) = fits.first() else {
            return Ok(Ty::Unknown);
        };
        let ret0 = h0.class().methods[*i0].ret.clone().unwrap_or_default();
        if !fits
            .iter()
            .all(|(h, i)| h.class().methods[*i].ret.as_deref() == Some(ret0.as_str()))
        {
            return Ok(Ty::Unknown);
        }
        let m = &h0.class().methods[*i0];
        if !m.type_params.is_empty() {
            return Ok(Ty::Unknown);
        }
        self.ty_in_class(&ret0, h0)
    }
}

enum FieldLookup {
    Found(Ty),
    Missing,
    Unknown,
}

pub enum Qualifier {
    Class(ClassHit),
    Value(Ty),
    Unknown,
}

fn numeric_promotion(l: &Ty, r: &Ty) -> Ty {
    let rank = |t: &Ty| match t {
        Ty::Prim(p) => match p.as_str() {
            "double" => 5,
            "float" => 4,
            "long" => 3,
            "int" | "short" | "byte" | "char" => 2,
            _ => 0,
        },
        _ => 0,
    };
    match (rank(l), rank(r)) {
        (0, _) | (_, 0) => Ty::Unknown,
        (a, b) => match a.max(b) {
            5 => Ty::Prim("double".into()),
            4 => Ty::Prim("float".into()),
            3 => Ty::Prim("long".into()),
            _ => Ty::Prim("int".into()),
        },
    }
}

/// Segments of `a.b.C` (an `identifier`, `scoped_identifier` or `scoped_type_identifier`), left to right.
pub fn scoped_segments(n: Node<'_>) -> Vec<Node<'_>> {
    match n.kind() {
        "identifier" | "type_identifier" => vec![n],
        "scoped_identifier" | "scoped_type_identifier" => {
            let mut out = Vec::new();
            let mut c = n.walk();
            let kids: Vec<Node<'_>> = n.named_children(&mut c).collect();
            for k in kids {
                match k.kind() {
                    "scoped_identifier"
                    | "scoped_type_identifier"
                    | "identifier"
                    | "type_identifier" => out.extend(scoped_segments(k)),
                    _ => {}
                }
            }
            out
        }
        _ => Vec::new(),
    }
}

pub fn find_by_id(root: Node<'_>, id: usize) -> Option<Node<'_>> {
    let mut stack = vec![root];
    while let Some(n) = stack.pop() {
        if n.id() == id {
            return Some(n);
        }
        let mut c = n.walk();
        for ch in n.children(&mut c) {
            stack.push(ch);
        }
    }
    None
}

impl ClassStub {
    /// The nested class at `path` (names below this class).
    pub fn inner_named_path(&self, path: &[String]) -> Option<&ClassStub> {
        let mut c = self;
        for n in path {
            c = c.inner_named(n)?;
        }
        Some(c)
    }
}
