//! Types of expressions: declared types, receiver substitution of generics, call return types
//! (spec 03, the slice the resolver needs: no inference).

use super::*;

impl<'a, 'e> Analyzer<'a, 'e> {
    pub fn ty_of_text(&mut self, text: &str, at: Node<'a>) -> Res<Ty> {
        let ctx = self.ctx_at(at)?;
        self.ty_from_ctx(text, &ctx)
    }

    pub(super) fn ty_from_ctx(&self, text: &str, ctx: &Ctx<'_>) -> Res<Ty> {
        let parts = ty::split(text);
        let base = match self.env.resolve_type_text(&parts.base, ctx)? {
            TypeRes::Class(h) => {
                let mut args = Vec::new();
                if let Some(list) = &parts.args {
                    for a in ty::split_args(list) {
                        args.push(self.type_argument(&a, ctx)?);
                    }
                }
                Ty::Ref {
                    fqn: Some(h.class().qname.clone()),
                    name: h.class().name.clone(),
                    args,
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
                    name: ty::display(&parts.base),
                    args: Vec::new(),
                }
            }
        };
        let mut t = base;
        for _ in 0..parts.dims {
            t = Ty::Array(Box::new(t));
        }
        Ok(t)
    }

    /// One type argument: a wildcard keeps its bound (JAVA-TYP-001); a read through it is the
    /// upper bound ([`Ty::read_type`]).
    pub(super) fn type_argument(&self, text: &str, ctx: &Ctx<'_>) -> Res<Ty> {
        let t = text.trim();
        if let Some(rest) = t.strip_prefix('?') {
            let rest = rest.trim();
            if let Some(bound) = rest.strip_prefix("extends") {
                return Ok(match self.ty_from_ctx(bound.trim(), ctx)? {
                    Ty::Unknown => Ty::Wildcard(Wild::Any),
                    b => Ty::Wildcard(Wild::Extends(Box::new(b))),
                });
            }
            if let Some(bound) = rest.strip_prefix("super") {
                return Ok(match self.ty_from_ctx(bound.trim(), ctx)? {
                    Ty::Unknown => Ty::Wildcard(Wild::Any),
                    b => Ty::Wildcard(Wild::Super(Box::new(b))),
                });
            }
            return Ok(Ty::Wildcard(Wild::Any));
        }
        self.ty_from_ctx(t, ctx)
    }

    /// A type text as written inside a declaration of `owner` (a field or return type), seen
    /// from the receiver type `recv` (`None`: type variables are unknown). Method type
    /// parameters `mparams` are never substituted (no inference).
    pub fn member_ty(
        &self,
        text: &str,
        owner: &ClassHit,
        mparams: &[String],
        recv: Option<&Ty>,
    ) -> Res<Ty> {
        let mut ctx = self.env.ctx_of(owner);
        ctx.tparams = mparams.to_vec();
        let t = self.ty_from_ctx(text, &ctx)?;
        if !t.has_var() {
            return Ok(t);
        }
        let mut map = HashMap::new();
        if let Some(args) = self.args_as(recv, owner)? {
            for (p, a) in owner.class().type_params.iter().zip(args) {
                map.insert(p.name.clone(), a);
            }
        }
        Ok(t.subst(&map))
    }

    /// The type of a field `owner` declares, as the code at `at` sees it: through the enclosing
    /// class that inherits it (`class Sub extends Base<String>` sees `T` of `Base<T>` as
    /// `String`; inside `Base<T>` itself `T` stays the variable).
    pub(super) fn field_ty_in_scope(
        &mut self,
        at: Node<'a>,
        text: &str,
        owner: &ClassHit,
    ) -> Res<Ty> {
        let chain = self.chain_at(at)?;
        for h in chain.iter().rev() {
            if self.env.is_subclass(h, owner.qname())?.unwrap_or(false) {
                let recv = self.class_self_type(h);
                return self.member_ty(text, owner, &[], Some(&recv));
            }
        }
        self.ty_in_class(text, owner)
    }

    pub fn ty_in_class(&self, text: &str, owner: &ClassHit) -> Res<Ty> {
        self.member_ty(text, owner, &[], None)
    }

    /// The type arguments of `target` as a supertype of the receiver type (`ArrayList<String>`
    /// seen as `List` has `[String]`); `None` when unknown or raw.
    pub(super) fn args_as(&self, recv: Option<&Ty>, target: &ClassHit) -> Res<Option<Vec<Ty>>> {
        let Some(Ty::Ref {
            fqn: Some(f), args, ..
        }) = recv
        else {
            return Ok(None);
        };
        let Some(h) = self.env.find_class(f)? else {
            return Ok(None);
        };
        self.walk_super(&h, args, &target.class().qname, 0)
    }

    pub(super) fn walk_super(
        &self,
        h: &ClassHit,
        args: &[Ty],
        target: &str,
        depth: usize,
    ) -> Res<Option<Vec<Ty>>> {
        if h.class().qname == target {
            return Ok(Some(args.to_vec()));
        }
        if depth > 12 {
            return Ok(None);
        }
        let c = h.class();
        let mut map = HashMap::new();
        if args.len() == c.type_params.len() {
            for (p, a) in c.type_params.iter().zip(args) {
                map.insert(p.name.clone(), a.clone());
            }
        }
        let mut ctx = self.env.ctx_of(h);
        if h.local.is_none() {
            ctx.chain.pop();
        }
        ctx.tparams = c.type_params.iter().map(|t| t.name.clone()).collect();
        for text in c.extends.iter().chain(c.implements.iter()) {
            let st = self.ty_from_ctx(text, &ctx)?;
            let Ty::Ref {
                fqn: Some(sf),
                args: sargs,
                ..
            } = st.subst(&map)
            else {
                continue;
            };
            let Some(sh) = self.env.find_class(&sf)? else {
                continue;
            };
            if let Some(found) = self.walk_super(&sh, &sargs, target, depth + 1)? {
                return Ok(Some(found));
            }
        }
        Ok(None)
    }

    pub fn expr_ty(&mut self, n: Node<'a>) -> Res<Ty> {
        if let Some(t) = self.expr_types.get(&n.id()) {
            return Ok(t.clone());
        }
        let t = self.expr_ty_uncached(n)?;
        self.expr_types.insert(n.id(), t.clone());
        Ok(t)
    }

    fn expr_ty_uncached(&mut self, n: Node<'a>) -> Res<Ty> {
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
            // An anonymous class declares members of its own that the named type does not
            // have: its type is not `Base` (no member list is derived from it).
            "object_creation_expression"
                if n.children(&mut n.walk()).any(|c| c.kind() == "class_body") =>
            {
                Ty::Unknown
            }
            "object_creation_expression" => match n.child_by_field_name("type") {
                Some(t) => {
                    let text = source::Builder { src: self.text }.compact(t);
                    if text.contains("<>") {
                        let base = text.replace("<>", "");
                        // Diamond: the arguments are inferred (M5); the raw class is what is known.
                        self.ty_of_text(&base, n)?
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
                    Some(h) if h.local.is_none() => self.class_self_type(h),
                    _ => Ty::Unknown,
                }
            }
            "class_literal" => {
                // `Foo.class` is a `Class<Foo>` (JLS 15.8.2).
                let target = n
                    .named_child(0)
                    .map(|t| source::Builder { src: self.text }.compact(t));
                match target {
                    Some(t) => {
                        let arg = match self.ty_of_text(&t, n)? {
                            Ty::Prim(p) => Ty::class(
                                ty::boxed_name(&p),
                                ty::boxed_name(&p).rsplit('.').next().unwrap_or(""),
                            ),
                            other => other,
                        };
                        Ty::generic("java.lang.Class", "Class", vec![arg])
                    }
                    None => Ty::Unknown,
                }
            }
            "assignment_expression" => match n.child_by_field_name("left") {
                Some(l) => self.expr_ty(l)?,
                None => Ty::Unknown,
            },
            "field_access" => {
                // `a.b.c`, `this.items`, `Type.CONSTANT`: the qualifier walk of completion.
                match self.qualifier_of(n)?.qual {
                    Qual::Value(t) => t,
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
                        self.field_ty_in_scope(n, &t, &owner)?
                    }
                    VarRes::StaticImport {
                        ty: Some(t), class, ..
                    } => self.ty_in_class(&t, &class)?,
                    VarRes::Component { owner, .. } => {
                        let t = owner
                            .class()
                            .record_components
                            .iter()
                            .find(|p| p.name == name)
                            .map(|p| p.ty.clone());
                        match t {
                            Some(t) => self.ty_in_class(&t, &owner)?,
                            None => Ty::Unknown,
                        }
                    }
                    _ => Ty::Unknown,
                }
            }
            "method_invocation" => self.call_ty(n)?,
            _ => Ty::Unknown,
        })
    }

    /// The return type of a call (no inference). The candidates are the methods of that name
    /// the receiver offers (an overridden declaration hides the inherited one), of the right
    /// arity, applicable to the argument types that are known; when they all declare the same
    /// return type, that type is seen from the receiver (`List<String>.get` is `String`),
    /// otherwise it is unknown (JAVA-TYP-032: no guess).
    pub(super) fn call_ty(&mut self, n: Node<'a>) -> Res<Ty> {
        let Some(name) = n
            .child_by_field_name("name")
            .map(|x| self.txt(x).to_string())
        else {
            return Ok(Ty::Unknown);
        };
        let mut arg_tys: Vec<Ty> = Vec::new();
        if let Some(args) = n.child_by_field_name("arguments") {
            let mut c = args.walk();
            for a in args
                .named_children(&mut c)
                .filter(|x| !matches!(x.kind(), "line_comment" | "block_comment"))
            {
                arg_tys.push(self.expr_ty(a)?);
            }
        }
        let arity = arg_tys.len();
        let mut recv: Option<Ty> = None;
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
                let q = self.qualifier_of(obj)?;
                match q.qual {
                    Qual::Type(h) => self.env.methods_named(&h, &name)?.0,
                    Qual::Value(t) => {
                        let found = match &t {
                            Ty::Ref { fqn: Some(f), .. } => match self.env.find_class(f)? {
                                Some(h) => self.env.methods_named(&h, &name)?.0,
                                None => Vec::new(),
                            },
                            _ => Vec::new(),
                        };
                        recv = Some(t);
                        found
                    }
                    _ => Vec::new(),
                }
            }
        };
        // Of the right arity, nearest declaration first; an inherited method that a nearer one
        // overrides (same erased parameters) does not count.
        let mut seen: HashSet<Vec<String>> = HashSet::new();
        let mut cands: Vec<(ClassHit, usize)> = Vec::new();
        for (h, i) in hits {
            let m = &h.class().methods[i];
            if !(m.params.len() == arity || (m.varargs && arity + 1 >= m.params.len())) {
                continue;
            }
            let mparams: Vec<String> = m.type_params.iter().map(|t| t.name.clone()).collect();
            let mut key = Vec::new();
            for p in &m.params {
                key.push(members::erased_key(&self.member_ty_keep(
                    &p.ty,
                    &h,
                    &mparams,
                    recv.as_ref(),
                )?));
            }
            if seen.insert(key) {
                cands.push((h, i));
            }
        }
        // Overloads: keep those that accept the arguments (unknown argument types accept all).
        if cands.len() > 1 {
            let mut fit = Vec::new();
            for (h, i) in &cands {
                if crate::import_fix::applicable(self.env, h, &h.class().methods[*i], &arg_tys)? {
                    fit.push((h.clone(), *i));
                }
            }
            if !fit.is_empty() {
                cands = fit;
            }
        }
        let mut result: Option<Ty> = None;
        for (h, i) in &cands {
            let m = &h.class().methods[*i];
            let mparams: Vec<String> = m.type_params.iter().map(|t| t.name.clone()).collect();
            let Some(ret) = m.ret.as_deref() else {
                return Ok(Ty::Unknown);
            };
            let t = self.member_ty(ret, h, &mparams, recv.as_ref())?;
            match &result {
                None => result = Some(t),
                Some(r) if *r == t => {}
                Some(_) => return Ok(Ty::Unknown),
            }
        }
        Ok(result.unwrap_or(Ty::Unknown))
    }
}
