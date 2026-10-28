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

mod calls;
mod refs;
mod scope;
mod types;

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
    /// Members may exist that no declaration shows (an annotation processor or an unresolved
    /// annotation could generate them): member errors are uncertain, so none is reported.
    augmented: bool,
    /// Parent of every node, built once: `Node::parent` walks down from the root each time.
    parents: HashMap<usize, Node<'a>>,
    /// Types of expressions already computed (a call chain asks for the receiver at every link).
    expr_types: HashMap<usize, Ty>,
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
    let mut a = Analyzer {
        env,
        text,
        tree,
        file,
        incomplete,
        out: Analysis::default(),
        local_hits: HashMap::new(),
        pattern_cache: HashMap::new(),
        augmented: false,
        parents: parent_map(tree),
        expr_types: HashMap::new(),
    };
    a.augmented = a.member_augmentation_possible().unwrap_or(true);
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

/// Whether an annotation text (`@Foo(..)`, `@a.b.Foo`) names a source-retained, non-JDK
/// annotation type: those are the ones annotation processors consume.
fn is_source_retained(annotations: &[String]) -> bool {
    annotations.iter().any(|a| {
        let a = a.trim_start_matches('@');
        (a.starts_with("java.lang.annotation.Retention(") || a.starts_with("Retention("))
            && a.contains("SOURCE")
    })
}

fn annotation_name(text: &str) -> &str {
    let t = text.trim().trim_start_matches('@');
    t.split('(').next().unwrap_or(t).trim()
}

fn parent_map(tree: &Tree) -> HashMap<usize, Node<'_>> {
    let mut out = HashMap::new();
    let mut cursor = tree.walk();
    let mut stack: Vec<Node<'_>> = Vec::new();
    loop {
        let node = cursor.node();
        if let Some(p) = stack.last() {
            out.insert(node.id(), *p);
        }
        if cursor.goto_first_child() {
            stack.push(node);
            continue;
        }
        loop {
            if cursor.goto_next_sibling() {
                break;
            }
            if !cursor.goto_parent() {
                return out;
            }
            stack.pop();
        }
    }
}

fn kind_is_class(k: &str) -> bool {
    source::is_class_kind(k)
}

impl<'a, 'e> Analyzer<'a, 'e> {
    /// The explicit certain/uncertain rule for member errors (spec 04 4.1.3): an annotation of
    /// the file whose type does not resolve, or whose type is source-retained and not from the
    /// JDK (what annotation processors consume), may add members this analysis cannot see.
    fn member_augmentation_possible(&self) -> Res<bool> {
        fn walk(c: &ClassStub, out: &mut Vec<String>) {
            out.extend(c.annotations.iter().cloned());
            for f in &c.fields {
                out.extend(f.annotations.iter().cloned());
            }
            for m in &c.methods {
                out.extend(m.annotations.iter().cloned());
            }
            for i in &c.inner {
                walk(i, out);
            }
        }
        let mut texts = Vec::new();
        for c in &self.file.stub.classes {
            walk(c, &mut texts);
        }
        texts.sort();
        texts.dedup();
        for t in texts {
            let name = annotation_name(&t);
            if name.is_empty() || name == "Override" || name == "FunctionalInterface" {
                continue;
            }
            let Some(top) = self.file.all_hits().into_iter().next() else {
                continue;
            };
            let ctx = self.env.ctx_of(&top);
            match self.env.resolve_type_text(name, &ctx)? {
                TypeRes::Class(h) => {
                    let q = &h.class().qname;
                    let jdk =
                        q.starts_with("java.") || q.starts_with("javax.") || q.starts_with("jdk.");
                    if !jdk && is_source_retained(&h.class().annotations) {
                        return Ok(true);
                    }
                }
                TypeRes::NotFound | TypeRes::Incomplete => return Ok(true),
                _ => {}
            }
        }
        Ok(false)
    }

    pub(super) fn parent(&self, n: Node<'a>) -> Option<Node<'a>> {
        self.parents.get(&n.id()).copied()
    }

    fn txt(&self, n: Node<'a>) -> &'a str {
        &self.text[n.byte_range()]
    }

    fn push_symbol(&mut self, n: Node<'a>, name: &str, class_ref: bool, annotation: bool) {
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
        let nodes: Vec<Node<'a>> = root
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
                            && !self.augmented
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

    fn visit(&mut self, node: Node<'a>) -> Res<()> {
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

    fn visit_children(&mut self, node: Node<'a>) -> Res<()> {
        let mut c = node.walk();
        let kids: Vec<Node<'a>> = node.children(&mut c).collect();
        for k in kids {
            self.visit(k)?;
        }
        Ok(())
    }

    /// Whether an `identifier` node is an expression name (as opposed to a declaration, label, ...).
    fn ident_role(&self, n: Node<'a>) -> Option<()> {
        let p = self.parent(n)?;
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
    fn javadoc(&mut self, n: Node<'a>) -> Res<()> {
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

    fn annotation(&mut self, n: Node<'a>) -> Res<()> {
        let Some(name) = n.child_by_field_name("name") else {
            return Ok(());
        };
        let segs = scoped_segments(name);
        let names: Vec<String> = segs.iter().map(|s| self.txt(*s).to_string()).collect();
        self.resolve_type_chain(n, &segs, &names, true)
    }

    fn type_ref(&mut self, n: Node<'a>) -> Res<()> {
        // Declarations: the name of a type parameter is not a reference.
        if let Some(p) = self.parent(n)
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

    fn scoped_type_ref(&mut self, n: Node<'a>) -> Res<()> {
        let segs = scoped_segments(n);
        let names: Vec<String> = segs.iter().map(|s| self.txt(*s).to_string()).collect();
        self.resolve_type_chain(n, &segs, &names, false)
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
