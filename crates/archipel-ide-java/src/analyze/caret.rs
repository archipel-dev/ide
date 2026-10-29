//! What completion asks the resolver at the caret (spec 06 6.3, 6.6): the qualifier of
//! `q.|`, the variables in scope, the expected type and the statement context. The caret is
//! a placeholder identifier inserted into the (repaired) text, so every answer comes from the
//! same scope walk and expression typing as the errors: completion and analysis cannot disagree
//! (JAVA-RES-001).

use super::members::{Place, Receiver};
use super::*;

/// The identifier that stands for the caret in the probe text.
pub const PLACEHOLDER: &str = "__archipel_caret__";

/// What stands before the dot.
#[derive(Debug, Clone)]
pub enum Qual {
    /// `Type.|`
    Type(ClassHit),
    /// `expr.|` with a static type that may still be unknown.
    Value(Ty),
    /// `pkg.|`
    Package(String),
    /// Not resolvable: nothing may be offered natively.
    Unknown,
}

#[derive(Debug, Clone)]
pub struct DotQualifier {
    pub qual: Qual,
    /// `this` or `super` (or `Outer.this`): the members of the enclosing class chain.
    pub via_this: bool,
    pub is_super: bool,
    /// The qualifier is a literal or a primitive-typed expression the dot cannot follow.
    pub is_number_literal: bool,
    /// The bounds of a type variable receiver, as classes.
    pub var_bounds: Vec<ClassHit>,
}

/// A variable of a block, parameter list or pattern in scope at the caret.
#[derive(Debug, Clone)]
pub struct Local {
    pub name: String,
    pub ty: Ty,
    pub is_param: bool,
}

/// A declaration found on the way up from the caret, before its type is computed.
struct Decl<'a> {
    name: String,
    type_text: Option<String>,
    init: Option<Node<'a>>,
    param: bool,
    /// `for (var x : <expr>)`: the type is the element type of this expression.
    elem_of: Option<Node<'a>>,
}

impl<'a> Decl<'a> {
    fn new(name: String, type_text: Option<String>, init: Option<Node<'a>>, param: bool) -> Self {
        Self {
            name,
            type_text,
            init,
            param,
            elem_of: None,
        }
    }
}

/// Statement context of the caret, from the enclosing syntax.
#[derive(Debug, Clone, Default)]
pub struct Flow {
    /// Inside the body of a loop (`break` and `continue` are legal).
    pub in_loop: bool,
    /// Inside a `switch` statement or expression body (`break` is legal in a statement).
    pub in_switch: bool,
    /// The caret is directly in a `switch` block where `case` or `default` may start.
    pub at_case_position: bool,
    /// Inside a `switch` expression (`yield`).
    pub in_switch_expr: bool,
    /// Inside a method, constructor, initializer or lambda body (`return`).
    pub in_body: bool,
    /// The statement before the caret is an `if` without a final `else`.
    pub after_if: bool,
    /// The statement before the caret is a `try` that can take `catch`.
    pub after_try: bool,
    /// The statement before the caret is a `try` without `finally`.
    pub try_without_finally: bool,
    /// The enclosing member is static (instance members are not usable, JAVA-RES-008).
    pub static_context: bool,
    /// The kind of the innermost enclosing class.
    pub class_kind: Option<ClassKind>,
}

/// Everything completion needs from the tree at the caret.
#[derive(Debug, Clone)]
pub struct CaretFacts {
    pub place: Place,
    /// `Some` when the caret follows a dot.
    pub qualifier: Option<DotQualifier>,
    pub locals: Vec<Local>,
    /// The types the context expects (JAVA-CMP-032), empty when none is cheaply known.
    pub expected: Vec<Ty>,
    pub flow: Flow,
    /// Type parameter names in scope (method and class).
    pub type_params: Vec<String>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Want {
    pub qualifier: bool,
    pub scope: bool,
    pub expected: bool,
}

/// Facts at the placeholder `ph` (a byte range of `text`), `None` when it is not where a
/// complete answer can be derived (the tree around it is broken).
pub fn caret_facts(
    env: &Env<'_>,
    text: &str,
    tree: &Tree,
    file: &CurrentFile,
    ph: Range<usize>,
    want: Want,
) -> Res<Option<CaretFacts>> {
    let mut a = Analyzer {
        env,
        text,
        tree,
        file,
        incomplete: false,
        out: Analysis::default(),
        local_hits: HashMap::new(),
        pattern_cache: HashMap::new(),
        augmented: false,
        parents: parent_map(tree),
        expr_types: HashMap::new(),
    };
    a.facts(ph, want)
}

impl<'a, 'e> Analyzer<'a, 'e> {
    fn facts(&mut self, ph: Range<usize>, want: Want) -> Res<Option<CaretFacts>> {
        let root = self.tree.root_node();
        let Some(node) = root.descendant_for_byte_range(ph.start, ph.end) else {
            return Ok(None);
        };
        if self.txt(node) != PLACEHOLDER {
            return Ok(None);
        }
        let chain = self.chain_at(node)?;
        let top = chain.first().map(|h| h.record.class.qname.clone());
        let place = Place {
            package: self.file.stub.package.clone(),
            top,
            chain,
        };
        let mut facts = CaretFacts {
            place,
            qualifier: None,
            locals: Vec::new(),
            expected: Vec::new(),
            flow: self.flow_at(node),
            type_params: self.method_tparams(node),
        };
        if want.qualifier {
            facts.qualifier = self.qualifier_at(node)?;
        }
        if want.scope {
            facts.locals = self.locals_at(node)?;
        }
        if want.expected {
            facts.expected = self.expected_at(node)?;
        }
        Ok(Some(facts))
    }

    // ---- qualifier ---------------------------------------------------------------------

    /// The qualifier when `node` is the member of `q.node`.
    fn qualifier_at(&mut self, node: Node<'a>) -> Res<Option<DotQualifier>> {
        let Some(p) = self.parent(node) else {
            return Ok(None);
        };
        let object = match p.kind() {
            "field_access"
                if p.child_by_field_name("field")
                    .is_some_and(|f| f.id() == node.id()) =>
            {
                p.child_by_field_name("object")
            }
            "method_invocation"
                if p.child_by_field_name("name")
                    .is_some_and(|f| f.id() == node.id()) =>
            {
                // `Iface.super.|`: the object is the interface, a `super` keyword follows it
                // (the grammar keeps both as children of the call).
                let mut c = p.walk();
                let has_super = p.children(&mut c).any(|k| k.kind() == "super");
                if has_super && let Some(object) = p.child_by_field_name("object") {
                    let q = self.qualifier_of(object)?;
                    return Ok(Some(match q.qual {
                        Qual::Type(h) => {
                            // The interface itself, or the superclass of the enclosing class.
                            let t = if h.class().is_interface_like() {
                                self.class_self_type(&h)
                            } else {
                                self.super_type_of(&h)?
                            };
                            let mut q = Self::value_qualifier(t);
                            q.via_this = true;
                            q.is_super = true;
                            q
                        }
                        _ => Self::unknown_qualifier(),
                    }));
                }
                p.child_by_field_name("object")
            }
            "scoped_type_identifier" | "scoped_identifier" => {
                // `Map.Ent|` in a type position: the qualifier is the part before.
                let mut c = p.walk();
                let first = p.named_children(&mut c).next();
                first.filter(|f| f.id() != node.id())
            }
            _ => None,
        };
        let Some(object) = object else {
            return Ok(None);
        };
        Ok(Some(self.qualifier_of(object)?))
    }

    fn unknown_qualifier() -> DotQualifier {
        DotQualifier {
            qual: Qual::Unknown,
            via_this: false,
            is_super: false,
            is_number_literal: false,
            var_bounds: Vec::new(),
        }
    }

    fn value_qualifier(t: Ty) -> DotQualifier {
        DotQualifier {
            qual: if t.is_unknown() {
                Qual::Unknown
            } else {
                Qual::Value(t)
            },
            via_this: false,
            is_super: false,
            is_number_literal: false,
            var_bounds: Vec::new(),
        }
    }

    /// Classifies the expression before a dot as a variable value, a type or a package
    /// (JLS 6.5.2: variable, then type, then package), and types it.
    pub(super) fn qualifier_of(&mut self, obj: Node<'a>) -> Res<DotQualifier> {
        match obj.kind() {
            "identifier" | "type_identifier" => {
                let name = self.txt(obj).to_string();
                if obj.kind() == "identifier" {
                    match self.resolve_variable(obj, &name)? {
                        VarRes::NotFound { .. } => {}
                        _ => {
                            let t = self.expr_ty(obj)?;
                            return Ok(Self::value_qualifier(t));
                        }
                    }
                }
                if self.local_type_declared(obj, &name) {
                    // A local class: its members are the stub of the class in the chain.
                    return Ok(Self::unknown_qualifier());
                }
                let ctx = self.ctx_at(obj)?;
                match self.env.resolve_type_name(&name, &ctx)? {
                    NameRes::Class(h, _) => Ok(DotQualifier {
                        qual: Qual::Type(h),
                        ..Self::unknown_qualifier()
                    }),
                    NameRes::NotFound => {
                        if self.env.package_exists(&name)? {
                            Ok(DotQualifier {
                                qual: Qual::Package(name),
                                ..Self::unknown_qualifier()
                            })
                        } else {
                            Ok(Self::unknown_qualifier())
                        }
                    }
                    _ => Ok(Self::unknown_qualifier()),
                }
            }
            "this" => {
                let t = self.expr_ty(obj)?;
                let mut q = Self::value_qualifier(t);
                q.via_this = true;
                Ok(q)
            }
            "super" => {
                let chain = self.chain_at(obj)?;
                let t = match chain.last() {
                    Some(h) => self.super_type_of(h)?,
                    None => Ty::Unknown,
                };
                let mut q = Self::value_qualifier(t);
                q.via_this = true;
                q.is_super = true;
                Ok(q)
            }
            "field_access" => {
                let Some(field) = obj.child_by_field_name("field") else {
                    return Ok(Self::unknown_qualifier());
                };
                let Some(inner) = obj.child_by_field_name("object") else {
                    return Ok(Self::unknown_qualifier());
                };
                let fname = self.txt(field).to_string();
                if field.kind() == "this" {
                    // `Outer.this`
                    let q = self.qualifier_of(inner)?;
                    return Ok(match q.qual {
                        Qual::Type(h) => {
                            let t = self.class_self_type(&h);
                            let mut q = Self::value_qualifier(t);
                            q.via_this = true;
                            q
                        }
                        _ => Self::unknown_qualifier(),
                    });
                }
                if field.kind() == "super" {
                    return Ok(Self::unknown_qualifier());
                }
                let q = self.qualifier_of(inner)?;
                match q.qual {
                    Qual::Package(p) => {
                        let full = format!("{p}.{fname}");
                        if let Some(h) = self.env.find_class(&full)? {
                            return Ok(DotQualifier {
                                qual: Qual::Type(h),
                                ..Self::unknown_qualifier()
                            });
                        }
                        if self.env.package_exists(&full)? {
                            return Ok(DotQualifier {
                                qual: Qual::Package(full),
                                ..Self::unknown_qualifier()
                            });
                        }
                        Ok(Self::unknown_qualifier())
                    }
                    Qual::Type(h) => {
                        if let Some(m) = self.env.member_type(&h, &fname)? {
                            return Ok(DotQualifier {
                                qual: Qual::Type(m),
                                ..Self::unknown_qualifier()
                            });
                        }
                        let (fs, complete, comp) = self.env.fields_named(&h, &fname)?;
                        if let Some((owner, idx)) = fs.into_iter().next() {
                            let ft = owner.class().fields[idx].ty.clone();
                            let t = self.ty_in_class(&ft, &owner)?;
                            return Ok(Self::value_qualifier(t));
                        }
                        let _ = (complete, comp);
                        Ok(Self::unknown_qualifier())
                    }
                    Qual::Value(t) => match self.field_type_of(&t, &fname)? {
                        FieldLookup::Found(nt) => Ok(Self::value_qualifier(nt)),
                        _ => Ok(Self::unknown_qualifier()),
                    },
                    Qual::Unknown => Ok(Self::unknown_qualifier()),
                }
            }
            "decimal_integer_literal"
            | "hex_integer_literal"
            | "octal_integer_literal"
            | "binary_integer_literal"
            | "decimal_floating_point_literal"
            | "hex_floating_point_literal" => Ok(DotQualifier {
                is_number_literal: true,
                ..Self::unknown_qualifier()
            }),
            _ => {
                let t = self.expr_ty(obj)?;
                Ok(Self::value_qualifier(t))
            }
        }
    }

    /// The type of `this` inside `h`: the class with its own type parameters as arguments.
    pub(super) fn class_self_type(&self, h: &ClassHit) -> Ty {
        let c = h.class();
        Ty::Ref {
            fqn: Some(c.qname.clone()),
            name: c.name.clone(),
            args: c
                .type_params
                .iter()
                .map(|p| Ty::Var(p.name.clone()))
                .collect(),
        }
    }

    /// The type of `super` inside `h`: its superclass with arguments, `Object` for a class
    /// without `extends` (and for a body that only implements interfaces).
    pub(super) fn super_type_of(&self, h: &ClassHit) -> Res<Ty> {
        let c = h.class();
        let object = || Ty::class("java.lang.Object", "Object");
        let mut ctx = self.env.ctx_of(h);
        if h.local.is_none() {
            ctx.chain.pop();
        }
        ctx.tparams = c.type_params.iter().map(|t| t.name.clone()).collect();
        let Some(text) = c.extends.first() else {
            return Ok(match c.kind {
                ClassKind::Enum => Ty::class("java.lang.Enum", "Enum"),
                ClassKind::Record => Ty::class("java.lang.Record", "Record"),
                _ => object(),
            });
        };
        if c.is_interface_like() {
            return Ok(object());
        }
        let t = self.ty_from_ctx(text, &ctx)?;
        if let Some(f) = t.class_fqn()
            && let Some(sh) = self.env.find_class(f)?
            && sh.class().is_interface_like()
        {
            return Ok(object());
        }
        Ok(t)
    }

    /// The bounds of the type variable `name` visible at `at`, as class hits (`T extends
    /// Comparable<T> & Serializable`): empty for an unbounded variable (`Object`), `None` when a
    /// declared bound does not resolve (a missing library): its members are not known.
    pub(super) fn var_bound_hits(
        &mut self,
        at: Node<'a>,
        name: &str,
    ) -> Res<Option<Vec<ClassHit>>> {
        let mut texts: Vec<String> = Vec::new();
        let mut cur = Some(at);
        'walk: while let Some(n) = cur {
            if let Some(tp) = n.child_by_field_name("type_parameters") {
                let mut c = tp.walk();
                for p in tp
                    .named_children(&mut c)
                    .filter(|p| p.kind() == "type_parameter")
                {
                    let mut pc = p.walk();
                    let kids: Vec<Node<'a>> = p.named_children(&mut pc).collect();
                    if kids.first().is_some_and(|f| self.txt(*f) == name) {
                        for k in kids.iter().skip(1) {
                            if k.kind() == "type_bound" {
                                let mut bc = k.walk();
                                for b in k.named_children(&mut bc) {
                                    texts.push(source::Builder { src: self.text }.compact(b));
                                }
                            }
                        }
                        break 'walk;
                    }
                }
            }
            cur = self.parent(n);
        }
        let ctx = self.ctx_at(at)?;
        let mut out = Vec::new();
        for t in texts {
            match self.env.resolve_type_text(&t, &ctx)? {
                TypeRes::Class(h) => out.push(h),
                // A bound that is itself a type variable, or one that does not resolve.
                _ => return Ok(None),
            }
        }
        Ok(Some(out))
    }

    /// The receiver whose members a qualifier offers, `None` when they cannot be known.
    pub fn receiver_of(&mut self, at: Node<'a>, q: &DotQualifier) -> Res<Option<Receiver>> {
        match &q.qual {
            Qual::Type(h) => Ok(Some(Receiver {
                hits: vec![h.clone()],
                ty: None,
                type_qualifier: true,
                array: None,
                via_this: q.via_this,
            })),
            Qual::Value(t) => self.receiver_of_ty(at, t, q.via_this),
            Qual::Package(_) | Qual::Unknown => Ok(None),
        }
    }

    pub(super) fn receiver_of_ty(
        &mut self,
        at: Node<'a>,
        t: &Ty,
        via_this: bool,
    ) -> Res<Option<Receiver>> {
        match t {
            Ty::Ref { fqn: Some(f), .. } => Ok(self.env.find_class(f)?.map(|h| Receiver {
                hits: vec![h],
                ty: Some(t.clone()),
                type_qualifier: false,
                array: None,
                via_this,
            })),
            Ty::Array(elem) => {
                let Some(object) = self.env.find_class("java.lang.Object")? else {
                    return Ok(None);
                };
                Ok(Some(Receiver {
                    hits: vec![object],
                    ty: None,
                    type_qualifier: false,
                    array: Some(Ty::Array(elem.clone())),
                    via_this,
                }))
            }
            Ty::Var(name) => {
                let Some(mut hits) = self.var_bound_hits(at, name)? else {
                    return Ok(None);
                };
                if hits.is_empty() {
                    let Some(object) = self.env.find_class("java.lang.Object")? else {
                        return Ok(None);
                    };
                    hits.push(object);
                }
                Ok(Some(Receiver {
                    hits,
                    ty: None,
                    type_qualifier: false,
                    array: None,
                    via_this,
                }))
            }
            _ => Ok(None),
        }
    }

    // ---- variables in scope ----------------------------------------------------------------

    fn local_ty(
        &mut self,
        type_text: Option<String>,
        init: Option<Node<'a>>,
        at: Node<'a>,
    ) -> Res<Ty> {
        match (type_text, init) {
            (Some(t), _) => self.ty_of_text(&t, at),
            (None, Some(init)) if init.end_byte() <= at.start_byte() => self.expr_ty(init),
            _ => Ok(Ty::Unknown),
        }
    }

    /// The variables, parameters and pattern bindings visible at `at`, innermost first, one per
    /// name (a nearer declaration hides the farther one; JAVA-RES-006).
    pub(super) fn locals_at(&mut self, at: Node<'a>) -> Res<Vec<Local>> {
        let mut out: Vec<Local> = Vec::new();
        let mut seen: HashSet<String> = HashSet::new();
        let mut child = at;
        let mut cur = self.parent(at);
        while let Some(a) = cur {
            let k = a.kind();
            let mut found: Vec<Decl<'a>> = Vec::new();
            match k {
                "block"
                | "constructor_body"
                | "switch_block"
                | "switch_block_statement_group"
                | "switch_rule" => {
                    let mut c = a.walk();
                    let sibs: Vec<Node<'a>> = a.children(&mut c).collect();
                    for sib in sibs {
                        if sib.start_byte() >= child.start_byte() && sib.id() != child.id() {
                            break;
                        }
                        let decls: Vec<Node<'a>> = if k == "switch_block"
                            && sib.kind() == "switch_block_statement_group"
                        {
                            // The scope of a declaration in an earlier group is the rest of the block.
                            let mut gc = sib.walk();
                            sib.children(&mut gc)
                                .filter(|s| s.kind() == "local_variable_declaration")
                                .collect()
                        } else if sib.kind() == "local_variable_declaration" {
                            vec![sib]
                        } else {
                            Vec::new()
                        };
                        for d in decls {
                            let own = d.id() == child.id()
                                || (d.start_byte() <= at.start_byte()
                                    && at.end_byte() <= d.end_byte());
                            self.declared_in(d, own.then(|| at.start_byte()), &mut found);
                        }
                    }
                }
                "for_statement" => {
                    if let Some(init) = a.child_by_field_name("init")
                        && init.kind() == "local_variable_declaration"
                    {
                        self.declared_in(init, None, &mut found);
                    }
                }
                "enhanced_for_statement"
                    if a.child_by_field_name("body")
                        .is_some_and(|b| b.id() == child.id()) =>
                {
                    if let Some(n) = a.child_by_field_name("name") {
                        let t = a
                            .child_by_field_name("type")
                            .map(|t| source::Builder { src: self.text }.compact(t))
                            .filter(|t| t != "var");
                        let mut d = Decl::new(self.txt(n).to_string(), t.clone(), None, false);
                        if t.is_none() {
                            // `var x : xs`: the element type of the iterated expression.
                            d.elem_of = a.child_by_field_name("value");
                        }
                        found.push(d);
                    }
                }
                "try_with_resources_statement" => {
                    if let Some(rs) = a.child_by_field_name("resources") {
                        let mut c = rs.walk();
                        for r in rs.named_children(&mut c).filter(|r| r.kind() == "resource") {
                            if r.end_byte() <= at.start_byte()
                                && let Some(n) = r.child_by_field_name("name")
                            {
                                let t = r
                                    .child_by_field_name("type")
                                    .map(|t| source::Builder { src: self.text }.compact(t))
                                    .filter(|t| t != "var");
                                found.push(Decl::new(
                                    self.txt(n).to_string(),
                                    t,
                                    r.child_by_field_name("value"),
                                    false,
                                ));
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
                        if let Some(n) = p.child_by_field_name("name") {
                            let t = p
                                .children(&mut p.walk())
                                .find(|x| x.kind() == "catch_type")
                                .map(|t| self.txt(t).to_string())
                                .filter(|t| !t.contains('|'));
                            found.push(Decl::new(self.txt(n).to_string(), t, None, true));
                        }
                    }
                }
                "lambda_expression" => {
                    if let Some(ps) = a.child_by_field_name("parameters") {
                        match ps.kind() {
                            "identifier" => {
                                found.push(Decl::new(self.txt(ps).to_string(), None, None, true))
                            }
                            "inferred_parameters" => {
                                let mut c = ps.walk();
                                for i in ps.named_children(&mut c) {
                                    found.push(Decl::new(
                                        self.txt(i).to_string(),
                                        None,
                                        None,
                                        true,
                                    ));
                                }
                            }
                            _ => self.params_in(ps, &mut found),
                        }
                    }
                }
                "method_declaration" | "constructor_declaration" => {
                    if let Some(ps) = a.child_by_field_name("parameters") {
                        self.params_in(ps, &mut found);
                    }
                }
                "compact_constructor_declaration" => {
                    // The components of the record are the parameters.
                    if let Some(h) = self.chain_at(a)?.last().cloned() {
                        for comp in &h.class().record_components {
                            found.push(Decl::new(
                                comp.name.clone(),
                                Some(comp.ty.clone()),
                                None,
                                true,
                            ));
                        }
                    }
                }
                _ => {}
            }
            // Pattern bindings of the statement or expression the caret is inside of.
            if matches!(
                k,
                "if_statement"
                    | "while_statement"
                    | "for_statement"
                    | "do_statement"
                    | "binary_expression"
                    | "ternary_expression"
                    | "switch_rule"
                    | "switch_block_statement_group"
                    | "lambda_expression"
                    | "method_declaration"
                    | "block"
                    | "expression_statement"
            ) {
                self.pattern_bindings_in(a, &mut found);
            }
            for d in found {
                if seen.insert(d.name.clone()) {
                    let ty = match d.elem_of {
                        Some(iterated) => self.element_ty(iterated)?,
                        None => self.local_ty(d.type_text, d.init, at)?,
                    };
                    out.push(Local {
                        name: d.name,
                        ty,
                        is_param: d.param,
                    });
                }
            }
            child = a;
            cur = self.parent(a);
        }
        Ok(out)
    }

    /// The element type of the expression of an enhanced `for`: an array's element, the
    /// argument of `Iterable<X>` (JLS 14.14.2).
    fn element_ty(&mut self, iterated: Node<'a>) -> Res<Ty> {
        let t = self.expr_ty(iterated)?;
        match &t {
            Ty::Array(e) => Ok((**e).clone()),
            Ty::Ref { fqn: Some(_), .. } => {
                let Some(iterable) = self.env.find_class("java.lang.Iterable")? else {
                    return Ok(Ty::Unknown);
                };
                Ok(match self.args_as(Some(&t), &iterable)? {
                    Some(args) if args.len() == 1 => args[0].read_type(),
                    _ => Ty::Unknown,
                })
            }
            _ => Ok(Ty::Unknown),
        }
    }

    /// Declarators of a local variable declaration that precede `limit` (a byte offset).
    fn declared_in(&self, decl: Node<'a>, limit: Option<usize>, out: &mut Vec<Decl<'a>>) {
        let ty = decl
            .child_by_field_name("type")
            .map(|t| source::Builder { src: self.text }.compact(t));
        let mut c = decl.walk();
        for d in decl.children_by_field_name("declarator", &mut c) {
            // A variable is not offered inside its own initializer.
            if limit.is_some_and(|l| d.end_byte() > l) {
                break;
            }
            let Some(n) = d.child_by_field_name("name") else {
                continue;
            };
            let dims = d
                .child_by_field_name("dimensions")
                .map(|x| "[]".repeat(self.txt(x).matches('[').count()))
                .unwrap_or_default();
            let tt = ty
                .clone()
                .filter(|t| t != "var")
                .map(|t| format!("{t}{dims}"));
            out.push(Decl::new(
                self.txt(n).to_string(),
                tt,
                d.child_by_field_name("value"),
                false,
            ));
        }
    }

    fn params_in(&self, params: Node<'a>, out: &mut Vec<Decl<'a>>) {
        let mut c = params.walk();
        for p in params.named_children(&mut c) {
            match p.kind() {
                "formal_parameter" | "receiver_parameter" => {
                    let Some(n) = p.child_by_field_name("name") else {
                        continue;
                    };
                    let dims = p
                        .child_by_field_name("dimensions")
                        .map(|d| "[]".repeat(self.txt(d).matches('[').count()))
                        .unwrap_or_default();
                    let t = p
                        .child_by_field_name("type")
                        .map(|t| format!("{}{dims}", source::Builder { src: self.text }.compact(t)))
                        .filter(|t| t != "var");
                    out.push(Decl::new(self.txt(n).to_string(), t, None, true));
                }
                "spread_parameter" => {
                    let mut pc = p.walk();
                    let name = p
                        .named_children(&mut pc)
                        .find(|x| x.kind() == "variable_declarator")
                        .and_then(|d| d.child_by_field_name("name"));
                    let t = p
                        .named_children(&mut p.walk())
                        .find(|x| !matches!(x.kind(), "modifiers" | "variable_declarator"))
                        .map(|t| format!("{}[]", source::Builder { src: self.text }.compact(t)));
                    if let Some(n) = name {
                        out.push(Decl::new(self.txt(n).to_string(), t, None, true));
                    }
                }
                _ => {}
            }
        }
    }

    /// Pattern variables (`o instanceof String s`) declared inside `boundary` (JAVA-RES-009
    /// simplification: every binding of the statement counts as in scope).
    fn pattern_bindings_in(&self, boundary: Node<'a>, out: &mut Vec<Decl<'a>>) {
        // Do not descend into nested blocks and class bodies: those are other scopes.
        let mut stack = vec![boundary];
        while let Some(n) = stack.pop() {
            match n.kind() {
                "instanceof_expression" => {
                    if let Some(nm) = n.child_by_field_name("name") {
                        let t = n
                            .child_by_field_name("right")
                            .map(|t| source::Builder { src: self.text }.compact(t));
                        out.push(Decl::new(self.txt(nm).to_string(), t, None, false));
                    }
                    if let Some(pat) = n.child_by_field_name("pattern") {
                        self.record_pattern_bindings(pat, out);
                    }
                }
                "type_pattern" => {
                    // `case String s ->`: the type, then the name.
                    let count = n.named_child_count();
                    if count >= 2
                        && let (Some(t), Some(nm)) = (n.named_child(0), n.named_child(count - 1))
                    {
                        out.push(Decl::new(
                            self.txt(nm).to_string(),
                            Some(source::Builder { src: self.text }.compact(t)),
                            None,
                            false,
                        ));
                    }
                }
                "block" | "class_body" | "lambda_expression" if n.id() != boundary.id() => {
                    // Bindings of the boundary's own condition only.
                    continue;
                }
                _ => {}
            }
            let mut c = n.walk();
            stack.extend(n.children(&mut c));
        }
    }

    fn record_pattern_bindings(&self, pat: Node<'a>, out: &mut Vec<Decl<'a>>) {
        let mut stack = vec![pat];
        while let Some(n) = stack.pop() {
            if n.kind() == "record_pattern_component" {
                let count = n.named_child_count();
                if count == 2
                    && let (Some(t), Some(nm)) = (n.named_child(0), n.named_child(1))
                    && nm.kind() == "identifier"
                {
                    let tt = source::Builder { src: self.text }.compact(t);
                    out.push(Decl::new(
                        self.txt(nm).to_string(),
                        (tt != "var").then_some(tt),
                        None,
                        false,
                    ));
                }
            }
            let mut c = n.walk();
            for ch in n.children(&mut c) {
                stack.push(ch);
            }
        }
    }

    // ---- statement context -------------------------------------------------------------------

    fn flow_at(&self, at: Node<'a>) -> Flow {
        let mut f = Flow::default();
        let mut child = at;
        let mut cur = self.parent(at);
        let mut crossed_lambda = false;
        let mut member_seen = false;
        // The statement that holds the caret: the child of the nearest block-like ancestor.
        let mut stmt: Option<Node<'a>> = None;
        while let Some(a) = cur {
            let k = a.kind();
            match k {
                "for_statement" | "enhanced_for_statement" | "while_statement" | "do_statement"
                    if !crossed_lambda =>
                {
                    let body = a.child_by_field_name("body");
                    if body.is_some_and(|b| b.id() == child.id()) {
                        f.in_loop = true;
                    }
                }
                "switch_block" if !crossed_lambda => {
                    f.in_switch = true;
                    if child.kind() == "switch_block_statement_group"
                        || child.kind() == "switch_rule"
                    {
                        f.at_case_position = false;
                    }
                }
                // The grammar parses a `switch` statement as a `switch_expression` too: it is an
                // expression only where an operand stands.
                "switch_expression"
                    if !crossed_lambda
                        && self.parent(a).is_some_and(|p| {
                            matches!(
                                p.kind(),
                                "variable_declarator"
                                    | "assignment_expression"
                                    | "return_statement"
                                    | "argument_list"
                                    | "parenthesized_expression"
                                    | "binary_expression"
                                    | "ternary_expression"
                                    | "yield_statement"
                                    | "cast_expression"
                                    | "array_initializer"
                                    | "lambda_expression"
                            )
                        }) =>
                {
                    f.in_switch_expr = true
                }
                "lambda_expression" => {
                    crossed_lambda = true;
                    f.in_body = true;
                }
                "block" | "constructor_body" | "switch_block_statement_group" | "switch_rule"
                    if stmt.is_none() =>
                {
                    stmt = Some(child);
                    if a.kind() == "switch_block_statement_group" || a.kind() == "switch_rule" {
                        f.at_case_position = false;
                    }
                }
                "method_declaration"
                | "constructor_declaration"
                | "static_initializer"
                | "compact_constructor_declaration"
                    if !member_seen =>
                {
                    member_seen = true;
                    f.in_body = true;
                    if k == "static_initializer" || self.has_modifier(a, "static") {
                        f.static_context = true;
                    }
                }
                "field_declaration" if !member_seen => {
                    member_seen = true;
                    if self.has_modifier(a, "static") {
                        f.static_context = true;
                    }
                }
                _ => {}
            }
            if source::is_class_kind(k) {
                if f.class_kind.is_none() {
                    f.class_kind = Some(match k {
                        "interface_declaration" => ClassKind::Interface,
                        "enum_declaration" => ClassKind::Enum,
                        "record_declaration" => ClassKind::Record,
                        "annotation_type_declaration" => ClassKind::Annotation,
                        _ => ClassKind::Class,
                    });
                }
                if member_seen || f.class_kind.is_some() {
                    break;
                }
            }
            child = a;
            cur = self.parent(a);
        }
        // `case` and `default` start where a statement of a switch block starts.
        if let Some(st) = stmt
            && let Some(p) = self.parent(st)
        {
            f.at_case_position = p.kind() == "switch_block"
                || p.kind() == "switch_block_statement_group"
                || p.kind() == "switch_rule";
        }
        // `else`, `catch`, `finally`: what the previous statement is.
        let mut holder = at;
        while let Some(p) = self.parent(holder) {
            if matches!(
                p.kind(),
                "block" | "constructor_body" | "switch_block_statement_group"
            ) {
                break;
            }
            holder = p;
        }
        if let Some(prev) = holder.prev_named_sibling() {
            match prev.kind() {
                "if_statement" => {
                    let mut last = prev;
                    while let Some(alt) = last.child_by_field_name("alternative") {
                        if alt.kind() == "if_statement" {
                            last = alt;
                        } else {
                            return Flow {
                                after_if: false,
                                ..f
                            };
                        }
                    }
                    f.after_if = true;
                }
                "try_statement" | "try_with_resources_statement" => {
                    let mut c = prev.walk();
                    let has_finally = prev.children(&mut c).any(|x| x.kind() == "finally_clause");
                    f.try_without_finally = !has_finally;
                    f.after_try = !has_finally;
                }
                _ => {}
            }
        }
        f
    }

    fn has_modifier(&self, decl: Node<'a>, modifier: &str) -> bool {
        let mut c = decl.walk();
        decl.children(&mut c).any(|m| {
            m.kind() == "modifiers" && {
                let mut mc = m.walk();
                m.children(&mut mc).any(|x| self.txt(x) == modifier)
            }
        })
    }

    // ---- expected type -------------------------------------------------------------------------

    /// The types the context expects at `at` (JAVA-CMP-032, the cheap contexts): an initializer
    /// or assignment target, a return value, a condition, a `throw`, and the parameter of a call
    /// whose candidates agree on it.
    pub(super) fn expected_at(&mut self, at: Node<'a>) -> Res<Vec<Ty>> {
        let mut node = at;
        // `new __c__()`: the expression is the creation, not the type name.
        if let Some(p) = self.parent(node)
            && p.kind() == "object_creation_expression"
            && p.child_by_field_name("type")
                .is_some_and(|t| t.id() == node.id())
        {
            node = p;
        }
        loop {
            let Some(p) = self.parent(node) else {
                return Ok(Vec::new());
            };
            match p.kind() {
                "parenthesized_expression" => {
                    // `if (|)`: a condition; elsewhere the parentheses are transparent.
                    if let Some(g) = self.parent(p)
                        && matches!(
                            g.kind(),
                            "if_statement" | "while_statement" | "do_statement"
                        )
                        && g.child_by_field_name("condition")
                            .is_some_and(|c| c.id() == p.id())
                    {
                        return Ok(vec![Ty::Prim("boolean".into())]);
                    }
                    node = p;
                }
                "ternary_expression" => {
                    if p.child_by_field_name("condition")
                        .is_some_and(|c| c.id() == node.id())
                    {
                        return Ok(vec![Ty::Prim("boolean".into())]);
                    }
                    node = p;
                }
                "variable_declarator" => {
                    if !p
                        .child_by_field_name("value")
                        .is_some_and(|v| v.id() == node.id())
                    {
                        return Ok(Vec::new());
                    }
                    let Some(decl) = self.parent(p) else {
                        return Ok(Vec::new());
                    };
                    let ty = decl
                        .child_by_field_name("type")
                        .map(|t| source::Builder { src: self.text }.compact(t));
                    let dims = p
                        .child_by_field_name("dimensions")
                        .map(|x| "[]".repeat(self.txt(x).matches('[').count()))
                        .unwrap_or_default();
                    return Ok(match ty.filter(|t| t != "var") {
                        Some(t) => vec![self.ty_of_text(&format!("{t}{dims}"), p)?],
                        None => Vec::new(),
                    });
                }
                "assignment_expression" => {
                    let is_right = p
                        .child_by_field_name("right")
                        .is_some_and(|r| r.id() == node.id());
                    let plain = p
                        .child_by_field_name("operator")
                        .is_some_and(|o| self.txt(o) == "=");
                    if !is_right || !plain {
                        return Ok(Vec::new());
                    }
                    return Ok(match p.child_by_field_name("left") {
                        Some(l) => {
                            let t = self.expr_ty(l)?;
                            if t.is_unknown() { Vec::new() } else { vec![t] }
                        }
                        None => Vec::new(),
                    });
                }
                "return_statement" => return self.return_expected(p),
                "throw_statement" => {
                    return Ok(vec![Ty::class("java.lang.Throwable", "Throwable")]);
                }
                "argument_list" => return self.argument_expected(p, node),
                _ => return Ok(Vec::new()),
            }
        }
    }

    fn return_expected(&mut self, ret: Node<'a>) -> Res<Vec<Ty>> {
        let mut cur = self.parent(ret);
        while let Some(a) = cur {
            match a.kind() {
                "lambda_expression" => return Ok(Vec::new()),
                "method_declaration" => {
                    let Some(t) = a.child_by_field_name("type") else {
                        return Ok(Vec::new());
                    };
                    let text = source::Builder { src: self.text }.compact(t);
                    if text == "void" {
                        return Ok(Vec::new());
                    }
                    let dims = a
                        .child_by_field_name("dimensions")
                        .map(|x| "[]".repeat(self.txt(x).matches('[').count()))
                        .unwrap_or_default();
                    let ty = self.ty_of_text(&format!("{text}{dims}"), ret)?;
                    return Ok(if ty.is_unknown() {
                        Vec::new()
                    } else {
                        vec![ty]
                    });
                }
                k if source::is_class_kind(k) => return Ok(Vec::new()),
                _ => {}
            }
            cur = self.parent(a);
        }
        Ok(Vec::new())
    }

    /// The type of parameter `index` of the call that owns `args`, when every candidate of the
    /// right arity agrees on its class.
    fn argument_expected(&mut self, args: Node<'a>, arg: Node<'a>) -> Res<Vec<Ty>> {
        let Some(call) = self.parent(args) else {
            return Ok(Vec::new());
        };
        let mut idx = 0usize;
        let mut c = args.walk();
        for a in args.named_children(&mut c) {
            if matches!(a.kind(), "line_comment" | "block_comment") {
                continue;
            }
            if a.id() == arg.id() {
                break;
            }
            idx += 1;
        }
        let arity = args
            .named_children(&mut args.walk())
            .filter(|x| !matches!(x.kind(), "line_comment" | "block_comment"))
            .count();
        let mut cands: Vec<(ClassHit, usize)> = Vec::new();
        let mut recv_ty: Option<Ty> = None;
        match call.kind() {
            "method_invocation" => {
                let Some(name) = call
                    .child_by_field_name("name")
                    .map(|n| self.txt(n).to_string())
                else {
                    return Ok(Vec::new());
                };
                match call.child_by_field_name("object") {
                    None => {
                        let chain = self.chain_at(call)?;
                        for h in chain.iter().rev() {
                            let (ms, _) = self.env.methods_named(h, &name)?;
                            if !ms.is_empty() {
                                cands = ms;
                                break;
                            }
                        }
                    }
                    Some(obj) => {
                        let q = self.qualifier_of(obj)?;
                        match q.qual {
                            Qual::Type(h) => cands = self.env.methods_named(&h, &name)?.0,
                            Qual::Value(t) => {
                                if let Some(f) = t.class_fqn()
                                    && let Some(h) = self.env.find_class(f)?
                                {
                                    cands = self.env.methods_named(&h, &name)?.0;
                                    recv_ty = Some(t.clone());
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            "object_creation_expression" => {
                let Some(t) = call.child_by_field_name("type") else {
                    return Ok(Vec::new());
                };
                let text = source::Builder { src: self.text }.compact(t);
                let ty = self.ty_of_text(&text, call)?;
                if let Some(f) = ty.class_fqn()
                    && let Some(h) = self.env.find_class(f)?
                {
                    for (i, m) in h.class().methods.iter().enumerate() {
                        if m.is_ctor {
                            cands.push((h.clone(), i));
                        }
                    }
                    recv_ty = Some(ty);
                }
            }
            _ => return Ok(Vec::new()),
        }
        let mut out: Vec<Ty> = Vec::new();
        for (h, i) in cands {
            let m = &h.class().methods[i];
            let fits = m.params.len() == arity || (m.varargs && arity + 1 >= m.params.len());
            if !fits {
                continue;
            }
            let p = if idx < m.params.len() {
                &m.params[idx]
            } else if m.varargs {
                match m.params.last() {
                    Some(p) => p,
                    None => continue,
                }
            } else {
                continue;
            };
            let mparams: Vec<String> = m.type_params.iter().map(|t| t.name.clone()).collect();
            let mut t = self.member_ty(&p.ty, &h, &mparams, recv_ty.as_ref())?;
            if p.varargs
                && idx >= m.params.len().saturating_sub(1)
                && let Ty::Array(e) = t
            {
                t = *e;
            }
            if !t.is_unknown() && !out.contains(&t) {
                out.push(t);
            }
        }
        Ok(out)
    }
}

// ---- the answers completion asks for -----------------------------------------------------------

/// The answer for the caret after a dot.
#[derive(Debug, Clone)]
pub enum DotAnswer {
    /// The qualifier is not known: nothing may be offered natively.
    Unknown,
    /// The qualifier is a value of primitive type, or a literal: the dot has no members
    /// (JAVA-CMP-003).
    NoMembers,
    /// The classes and subpackages of a package.
    Package(String),
    /// The members of a type or of a value's static type.
    Members {
        set: members::MemberSet,
        place: Place,
        /// `Type.|`: only statics and member types.
        type_qualifier: bool,
        /// A type-position qualifier (`Map.Ent|` in a declaration): member types only.
        type_context: bool,
        /// `Outer.|` where `Outer` encloses the caret: `this` forms are legal.
        enclosing: bool,
        /// The qualifier type is an array.
        array: bool,
        /// The hierarchy of the qualifier was seen completely.
        complete: bool,
    },
}

/// The answer for an identifier typed without a qualifier.
#[derive(Debug, Clone)]
pub struct ScopeAnswer {
    pub facts: CaretFacts,
    /// Members of the enclosing classes, innermost first.
    pub levels: Vec<Level>,
    /// Members reachable through static imports: `(index into the file's imports, members)`.
    pub imported: Vec<(usize, members::MemberSet)>,
}

/// The members of one enclosing class.
#[derive(Debug, Clone)]
pub struct Level {
    pub class: ClassHit,
    pub set: members::MemberSet,
    /// Instance members can be used here (not in a static context).
    pub instance_ok: bool,
}

/// What a probe of the text yields at the placeholder.
#[derive(Debug, Clone)]
pub struct ProbeAnswer {
    pub flow: Flow,
    pub dot: Option<DotAnswer>,
    pub scope: Option<ScopeAnswer>,
    pub expected: Vec<Ty>,
}

/// What a probe computes.
#[derive(Debug, Clone, Copy, Default)]
pub struct ProbeWants {
    /// The members of the qualifier of a dot.
    pub dot: bool,
    /// The variables and members in scope.
    pub scope: bool,
    /// The expected type.
    pub expected: bool,
    /// Offer the members the place cannot reach too (flagged).
    pub include_inaccessible: bool,
}

/// Everything one request needs, computed with one analyzer so the tree is walked once.
pub fn probe_answer(
    env: &Env<'_>,
    text: &str,
    tree: &Tree,
    file: &CurrentFile,
    ph: Range<usize>,
    wants: ProbeWants,
) -> Res<Option<ProbeAnswer>> {
    let ProbeWants {
        dot,
        scope,
        expected,
        include_inaccessible,
    } = wants;
    let Some(node) = tree
        .root_node()
        .descendant_for_byte_range(ph.start, ph.end)
        .filter(|n| &text[n.byte_range()] == PLACEHOLDER)
    else {
        return Ok(None);
    };
    let mut a = Analyzer {
        env,
        text,
        tree,
        file,
        incomplete: false,
        out: Analysis::default(),
        local_hits: HashMap::new(),
        pattern_cache: HashMap::new(),
        augmented: false,
        parents: parent_map_around(tree, node),
        expr_types: HashMap::new(),
    };
    let facts = a.facts(
        ph,
        Want {
            qualifier: dot,
            scope,
            expected,
        },
    )?;
    let Some(facts) = facts else {
        return Ok(None);
    };
    let mut out = ProbeAnswer {
        flow: facts.flow.clone(),
        dot: None,
        scope: None,
        expected: facts.expected.clone(),
    };
    if dot {
        out.dot = Some(a.dot_answer(node, &facts, include_inaccessible)?);
    }
    if scope {
        out.scope = Some(a.scope_answer(facts, include_inaccessible)?);
    }
    Ok(Some(out))
}

impl<'a, 'e> Analyzer<'a, 'e> {
    fn dot_answer(
        &mut self,
        node: Node<'a>,
        facts: &CaretFacts,
        include_inaccessible: bool,
    ) -> Res<DotAnswer> {
        let Some(q) = facts.qualifier.clone() else {
            return Ok(DotAnswer::Unknown);
        };
        if q.is_number_literal {
            return Ok(DotAnswer::NoMembers);
        }
        let type_context = self
            .parent(node)
            .is_some_and(|p| matches!(p.kind(), "scoped_type_identifier" | "scoped_identifier"));
        match &q.qual {
            Qual::Package(p) => return Ok(DotAnswer::Package(p.clone())),
            Qual::Unknown => return Ok(DotAnswer::Unknown),
            Qual::Value(Ty::Prim(_)) => return Ok(DotAnswer::NoMembers),
            _ => {}
        }
        let enclosing = match &q.qual {
            Qual::Type(h) => facts
                .place
                .chain
                .iter()
                .any(|c| c.class().qname == h.class().qname),
            _ => false,
        };
        let Some(mut recv) = self.receiver_of(node, &q)? else {
            return Ok(DotAnswer::Unknown);
        };
        if q.is_super {
            recv.via_this = true;
        }
        let set = self.members_of(&recv, &facts.place, include_inaccessible)?;
        Ok(DotAnswer::Members {
            complete: set.complete,
            set,
            place: facts.place.clone(),
            type_qualifier: recv.type_qualifier,
            type_context,
            enclosing,
            array: recv.array.is_some(),
        })
    }

    fn scope_answer(&mut self, facts: CaretFacts, include_inaccessible: bool) -> Res<ScopeAnswer> {
        let mut levels = Vec::new();
        let mut instance_ok = !facts.flow.static_context;
        let chain = facts.place.chain.clone();
        for h in chain.iter().rev() {
            let recv = members::Receiver {
                hits: vec![h.clone()],
                ty: Some(self.class_self_type(h)),
                type_qualifier: false,
                array: None,
                via_this: true,
            };
            let set = self.members_of(&recv, &facts.place, include_inaccessible)?;
            levels.push(Level {
                class: h.clone(),
                set,
                instance_ok,
            });
            // Leaving a static nested class: the outer instance is out of reach.
            let c = h.class();
            if c.flags & flags::STATIC != 0
                || c.is_static_member
                || !matches!(c.kind, ClassKind::Class)
            {
                instance_ok = false;
            }
        }
        let mut imported = Vec::new();
        let imports = self.file.stub.imports.clone();
        for (i, imp) in imports.iter().enumerate() {
            if !imp.is_static {
                continue;
            }
            let class_name = if imp.on_demand {
                imp.name.clone()
            } else {
                match imp.name.rsplit_once('.') {
                    Some((c, _)) => c.to_string(),
                    None => continue,
                }
            };
            let Some(h) = self.env.find_class(&class_name)? else {
                continue;
            };
            let recv = members::Receiver {
                hits: vec![h],
                ty: None,
                type_qualifier: true,
                array: None,
                via_this: false,
            };
            let mut set = self.members_of(&recv, &facts.place, include_inaccessible)?;
            if !imp.on_demand
                && let Some(member) = imp.name.rsplit('.').next()
            {
                set.members.retain(|m| m.name == member);
            }
            if !set.members.is_empty() {
                imported.push((i, set));
            }
        }
        Ok(ScopeAnswer {
            facts,
            levels,
            imported,
        })
    }
}
