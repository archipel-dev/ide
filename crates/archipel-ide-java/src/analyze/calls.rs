//! Method calls: unqualified and qualified, the zero-candidate errors JAVA-ERR-007 and 011.

use super::*;

impl<'a, 'e> Analyzer<'a, 'e> {
    pub(super) fn method_invocation(&mut self, n: Node<'a>) -> Res<()> {
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
                let targs: Vec<Node<'a>> = n
                    .children(&mut c)
                    .filter(|x| x.kind() == "type_arguments")
                    .collect();
                for t in targs {
                    self.visit(t)?;
                }
                match obj.kind() {
                    "identifier" => self.name_chain(n, &[obj, name_node], Some((n, args))),
                    "field_access" => {
                        let mut segs: Vec<Node<'a>> = Vec::new();
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

    pub(super) fn unqualified_call(
        &mut self,
        call: Node<'a>,
        name_node: Node<'a>,
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
        if incomplete || self.augmented || chain.is_empty() {
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

    pub(super) fn push_method(
        &mut self,
        call: Node<'a>,
        name_node: Node<'a>,
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

    pub(super) fn check_qualified_call(
        &mut self,
        call: Node<'a>,
        _call2: Node<'a>,
        name_node: Node<'a>,
        name: &str,
        args: &[Ty],
        q: Qualifier,
    ) -> Res<()> {
        let (hit, is_type) = match &q {
            // JAVA-ERR-019: a primitive-typed qualifier has no members.
            Qualifier::Value(Ty::Prim(p)) if !self.incomplete => {
                self.out.problems.push(Problem {
                    range: range(name_node),
                    message: format!("Cannot call methods on '{p}' type"),
                    kind: ProblemKind::Method {
                        name: name.to_string(),
                        args: args.to_vec(),
                        qualified: true,
                        call: range(call),
                    },
                });
                return Ok(());
            }
            Qualifier::Class(h) => (h.clone(), true),
            Qualifier::Value(Ty::Ref { fqn: Some(fqn), .. }) => match self.env.find_class(fqn)? {
                Some(h) => (h, false),
                None => return Ok(()),
            },
            _ => return Ok(()),
        };
        let (ms, complete) = self.env.methods_named(&hit, name)?;
        if !ms.is_empty()
            || self.env.has_synthetic_method(&hit, name)
            || !complete
            || self.augmented
        {
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
}
