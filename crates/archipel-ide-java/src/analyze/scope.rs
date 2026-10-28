//! The scope walk (spec 02 section 2.3): the enclosing class chain, local and member variables,
//! type-name lookup around a node.

use super::*;

impl<'a, 'e> Analyzer<'a, 'e> {
    /// Chain of the ambient context at `at`: enclosing class hits (outermost first).
    pub(super) fn chain_at(&mut self, at: Node<'a>) -> Res<Vec<ClassHit>> {
        let mut ancestors: Vec<Node<'a>> = Vec::new();
        let mut cur = self.parent(at);
        while let Some(a) = cur {
            ancestors.push(a);
            cur = self.parent(a);
        }
        ancestors.reverse();
        let mut chain: Vec<ClassHit> = Vec::new();
        let mut path: Vec<String> = Vec::new();
        let mut top: Option<usize> = None;
        let mut named_only = true;
        for a in ancestors {
            let k = a.kind();
            let is_body_of_anon = k == "class_body"
                && self.parent(a).is_some_and(|p| {
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
                if named_only
                    && top.is_none()
                    && self.parent(a).is_some_and(|p| p.kind() == "program")
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

    pub(super) fn anonymous_base(&self, body: Node<'a>) -> String {
        let Some(p) = self.parent(body) else {
            return "java.lang.Object".into();
        };
        match p.kind() {
            "object_creation_expression" => p
                .child_by_field_name("type")
                .map(|t| source::Builder { src: self.text }.compact(t))
                .unwrap_or_else(|| "java.lang.Object".into()),
            "enum_constant" => self
                .parent(p)
                .and_then(|b| self.parent(b))
                .and_then(|e| e.child_by_field_name("name"))
                .map(|n| self.txt(n).to_string())
                .unwrap_or_else(|| "java.lang.Object".into()),
            _ => "java.lang.Object".into(),
        }
    }

    pub(super) fn make_local(
        &mut self,
        chain: &[ClassHit],
        at: Node<'a>,
        stub: ClassStub,
    ) -> Res<ClassHit> {
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

    pub(super) fn method_tparams(&self, at: Node<'a>) -> Vec<String> {
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
            cur = self.parent(n);
        }
        out
    }

    pub fn ctx_at(&mut self, at: Node<'a>) -> Res<Ctx<'a>> {
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
    pub(super) fn local_type_declared(&self, at: Node<'a>, name: &str) -> bool {
        let mut child = at;
        let mut cur = self.parent(at);
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
            cur = self.parent(a);
        }
        false
    }

    /// Resolves a (qualified) type name; reports the first unresolved segment (JAVA-RES-005).
    pub(super) fn resolve_type_chain(
        &mut self,
        at: Node<'a>,
        segs: &[Node<'a>],
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

    pub(super) fn pattern_names(&mut self, boundary: Node<'a>) -> &HashSet<String> {
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

    pub(super) fn declarators(
        &self,
        decl: Node<'a>,
        name: &str,
        limit: Option<usize>,
    ) -> Option<VarRes> {
        let ty = decl
            .child_by_field_name("type")
            .map(|t| source::Builder { src: self.text }.compact(t));
        let mut c = decl.walk();
        for d in decl.children_by_field_name("declarator", &mut c) {
            // The scope of a declarator starts at itself: later declarators of the same
            // declaration (`int a = 1, b = a;`) see it.
            if limit.is_some_and(|l| d.start_byte() > l) {
                break;
            }
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
    pub fn resolve_variable(&mut self, at: Node<'a>, name: &str) -> Res<VarRes> {
        let mut chain = self.chain_at(at)?;
        let mut incomplete = false;
        let mut child = at;
        let mut cur = self.parent(at);
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
                        if sib.start_byte() > child.start_byte() {
                            break;
                        }
                        // A declaration of an earlier group of an old-style switch is in scope in
                        // the later groups (the scope is the rest of the switch block).
                        if k == "switch_block"
                            && sib.kind() == "switch_block_statement_group"
                            && sib.id() != child.id()
                        {
                            let mut gc = sib.walk();
                            for st in sib.children(&mut gc) {
                                if st.kind() == "local_variable_declaration"
                                    && let Some(v) = self.declarators(st, name, None)
                                {
                                    return Ok(v);
                                }
                            }
                            continue;
                        }
                        // The statement holding the reference counts for its own earlier
                        // declarators only.
                        let own = sib.id() == child.id();
                        if sib.start_byte() == child.start_byte() && !own {
                            break;
                        }
                        if sib.kind() == "local_variable_declaration"
                            && let Some(v) =
                                self.declarators(sib, name, own.then(|| at.start_byte()))
                        {
                            return Ok(v);
                        }
                    }
                }
                "for_statement" => {
                    if let Some(init) = a.child_by_field_name("init")
                        && init.kind() == "local_variable_declaration"
                        && let Some(v) = self.declarators(init, name, None)
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
                                // Each resource is in scope in the later resources and the body.
                                && r.end_byte() <= at.start_byte()
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
                && self.parent(a).is_some_and(|p| {
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
            cur = self.parent(a);
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
    pub(super) fn param_named(&self, params: Node<'a>, name: &str) -> Option<Option<String>> {
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
}
