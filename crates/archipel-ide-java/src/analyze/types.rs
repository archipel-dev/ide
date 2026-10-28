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
                let shown = ty::display(text.trim_end_matches("[]").trim_end_matches("..."));
                let mut args = Vec::new();
                if let Some(list) = &parts.args {
                    for a in ty::split_args(list) {
                        args.push(self.type_argument(&a, ctx)?);
                    }
                }
                Ty::Ref {
                    fqn: Some(h.class().qname.clone()),
                    display: shown,
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
                    display: ty::display(&parts.base),
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

    /// One type argument: a wildcard with an upper bound is its bound (reads are what matters
    /// for member lookup), any other wildcard is unknown.
    pub(super) fn type_argument(&self, text: &str, ctx: &Ctx<'_>) -> Res<Ty> {
        let t = text.trim();
        if let Some(rest) = t.strip_prefix('?') {
            return match rest.trim().strip_prefix("extends") {
                Some(bound) => self.ty_from_ctx(bound.trim(), ctx),
                None => Ok(Ty::Unknown),
            };
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
            "object_creation_expression" => match n.child_by_field_name("type") {
                Some(t) => {
                    let text = source::Builder { src: self.text }.compact(t);
                    if text.contains("<>") {
                        let base = text.replace("<>", "");
                        let t = self.ty_of_text(&base, n)?;
                        match t {
                            Ty::Ref { fqn, display, .. } => Ty::Ref {
                                fqn,
                                display: display.to_string(),
                                args: Vec::new(),
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
                    Some(h) if h.local.is_none() => Ty::class(&h.class().qname, &h.class().name),
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

    /// The return type of a call (no inference): the candidates of the right arity that the
    /// nearest declaring class offers must agree on the declared return type, which is then
    /// seen from the receiver type (`List<String>.get` is `String`).
    pub(super) fn call_ty(&mut self, n: Node<'a>) -> Res<Ty> {
        let Some(name) = n
            .child_by_field_name("name")
            .map(|x| self.txt(x).to_string())
        else {
            return Ok(Ty::Unknown);
        };
        let arity = n
            .child_by_field_name("arguments")
            .map_or(0, |a| a.named_child_count());
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
                let t = if obj.kind() == "identifier" {
                    // Class or variable.
                    let oname = self.txt(obj).to_string();
                    match self.resolve_variable(obj, &oname)? {
                        VarRes::NotFound { .. } => {
                            let ctx = self.ctx_at(obj)?;
                            match self.env.resolve_type_name(&oname, &ctx)? {
                                NameRes::Class(h, _) => {
                                    Ty::class(&h.class().qname, &h.class().name)
                                }
                                _ => Ty::Unknown,
                            }
                        }
                        _ => {
                            let t = self.expr_ty(obj)?;
                            recv = Some(t.clone());
                            t
                        }
                    }
                } else {
                    let t = self.expr_ty(obj)?;
                    recv = Some(t.clone());
                    t
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
        let Some((h0, _)) = fits.first() else {
            return Ok(Ty::Unknown);
        };
        // Overriding declarations in the nearest class win over inherited ones.
        let nearest: Vec<&&(ClassHit, usize)> = fits
            .iter()
            .filter(|(h, _)| h.class().qname == h0.class().qname)
            .collect();
        let mut result: Option<Ty> = None;
        for (h, i) in nearest.iter().map(|x| **x) {
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
