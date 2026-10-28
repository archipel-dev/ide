//! Name references: simple names, field access chains and the first-unresolved-segment rule
//! (JAVA-RES-005, JAVA-ERR-012).

use super::*;

impl<'a, 'e> Analyzer<'a, 'e> {
    /// A standalone or leftmost expression name.
    pub(super) fn simple_name(&mut self, n: Node<'a>) -> Res<()> {
        let name = self.txt(n).to_string();
        // A leftmost qualifier may also be a type or a package; a standalone name is a variable.
        let is_qualifier = self.parent(n).is_some_and(|p| {
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
                if incomplete || self.augmented {
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

    pub(super) fn field_access(&mut self, n: Node<'a>) -> Res<()> {
        // Flatten a chain of plain names `a.b.c`.
        let mut segs: Vec<Node<'a>> = Vec::new();
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
    pub(super) fn name_chain(
        &mut self,
        at: Node<'a>,
        segs: &[Node<'a>],
        call: Option<(Node<'a>, Vec<Ty>)>,
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
                                if incomplete || self.augmented {
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
                        } else if complete && !comp && !self.augmented {
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

    pub(super) fn member_chain_after_expr(&mut self, obj: Node<'a>, segs: &[Node<'a>]) -> Res<()> {
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

    pub(super) fn field_type_of(&mut self, t: &Ty, name: &str) -> Res<FieldLookup> {
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
                    return Ok(FieldLookup::Found(self.member_ty(
                        &ft,
                        &owner,
                        &[],
                        Some(t),
                    )?));
                }
                if comp {
                    return Ok(FieldLookup::Unknown);
                }
                Ok(if complete && !self.augmented {
                    FieldLookup::Missing
                } else {
                    FieldLookup::Unknown
                })
            }
            _ => Ok(FieldLookup::Unknown),
        }
    }

    pub(super) fn method_reference(&mut self, n: Node<'a>) -> Res<()> {
        // `Qualifier::name`: only the qualifier is checked.
        let mut c = n.walk();
        let first = n.children(&mut c).next();
        if let Some(f) = first {
            self.visit(f)?;
        }
        Ok(())
    }

    pub(super) fn object_creation(&mut self, n: Node<'a>) -> Res<()> {
        // `outer.new Inner()`: the type is resolved relative to the outer instance: skip it.
        let has_outer = n.child(0).is_some_and(|c| {
            c.kind() == "object_creation_expression"
                || c.kind() == "identifier"
                || c.kind() == "field_access"
                || c.kind() == "method_invocation"
        });
        let mut c = n.walk();
        let kids: Vec<Node<'a>> = n.children(&mut c).collect();
        for k in kids {
            if has_outer && k.id() == n.child_by_field_name("type").map(|t| t.id()).unwrap_or(0) {
                continue;
            }
            self.visit(k)?;
        }
        Ok(())
    }
}
