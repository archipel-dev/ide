//! The source stub builder (JAVA-STUB-001..010, 019, 021): a Java syntax tree to a
//! [`FileStub`]. Bodies are skipped (JAVA-STUB-003), so an edit inside a method body
//! leaves the stub, and every index entry derived from it, unchanged.
//!
//! Deviation recorded for the spec team: local and anonymous classes, lambdas and
//! method references inside bodies get no stubs yet (JAVA-STUB-019 and 009 LATER); the
//! anonymous-base index and functional-expression stubs are not built.

use crate::stub::*;
use tree_sitter::{Node, Parser, Tree};

pub fn parse(text: &str) -> Option<Tree> {
    let mut p = Parser::new();
    p.set_language(&tree_sitter_java::LANGUAGE.into()).ok()?;
    p.parse(text, None)
}

/// Builds the stub of `text`.
pub fn build_file_stub(text: &str) -> FileStub {
    match parse(text) {
        Some(tree) => build_from_tree(&tree, text),
        None => FileStub::default(),
    }
}

pub fn build_from_tree(tree: &Tree, text: &str) -> FileStub {
    let b = Builder { src: text };
    let root = tree.root_node();
    let mut stub = FileStub::default();
    let mut cur = root.walk();
    for n in root.children(&mut cur) {
        match n.kind() {
            "package_declaration" => {
                if let Some(name) = n.named_children(&mut n.walk()).find(|c| {
                    matches!(c.kind(), "identifier" | "scoped_identifier")
                }) {
                    stub.package = b.compact(name);
                }
            }
            "import_declaration" => {
                if let Some(i) = b.import(n) {
                    stub.imports.push(i);
                }
            }
            k if is_class_kind(k) => {
                if let Some(c) = b.class(n, &stub.package) {
                    stub.classes.push(c);
                }
            }
            _ => {}
        }
    }
    stub
}

pub fn is_class_kind(k: &str) -> bool {
    matches!(
        k,
        "class_declaration"
            | "interface_declaration"
            | "enum_declaration"
            | "record_declaration"
            | "annotation_type_declaration"
    )
}

struct Builder<'a> {
    src: &'a str,
}

fn collapse(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut space = false;
    for ch in s.chars() {
        if ch.is_whitespace() {
            space = true;
        } else {
            if space && !out.is_empty() {
                out.push(' ');
            }
            space = false;
            out.push(ch);
        }
    }
    out
}

impl Builder<'_> {
    fn text(&self, n: Node<'_>) -> &str {
        &self.src[n.byte_range()]
    }

    /// Source text with whitespace collapsed and comments dropped, as written otherwise.
    fn compact(&self, n: Node<'_>) -> String {
        if n.child_count() == 0 {
            return self.text(n).to_string();
        }
        let mut parts = String::new();
        let mut last = n.start_byte();
        let mut cur = n.walk();
        let mut stack = vec![n];
        // Gather leaf text with original inter-token gaps collapsed.
        let _ = &mut cur;
        while let Some(node) = stack.pop() {
            if node.child_count() == 0 {
                if matches!(node.kind(), "line_comment" | "block_comment") {
                    continue;
                }
                if node.start_byte() > last {
                    let gap = &self.src[last..node.start_byte()];
                    if gap.chars().any(char::is_whitespace) {
                        parts.push(' ');
                    }
                }
                parts.push_str(self.text(node));
                last = node.end_byte();
            } else {
                let mut c = node.walk();
                let kids: Vec<Node<'_>> = node.children(&mut c).collect();
                for k in kids.into_iter().rev() {
                    stack.push(k);
                }
            }
        }
        collapse(&parts)
    }

    fn import(&self, n: Node<'_>) -> Option<ImportStub> {
        let mut is_static = false;
        let mut on_demand = false;
        let mut name = None;
        let mut c = n.walk();
        for ch in n.children(&mut c) {
            match ch.kind() {
                "static" => is_static = true,
                "asterisk" => on_demand = true,
                "identifier" | "scoped_identifier" => name = Some(self.compact(ch).replace(' ', "")),
                _ => {}
            }
        }
        Some(ImportStub {
            name: name?,
            on_demand,
            is_static,
        })
    }

    fn modifiers(&self, n: Node<'_>) -> (u32, Vec<String>) {
        let mut f = 0;
        let mut ann = Vec::new();
        let Some(m) = n.children(&mut n.walk()).find(|c| c.kind() == "modifiers") else {
            return (0, ann);
        };
        let mut c = m.walk();
        for ch in m.children(&mut c) {
            f |= match ch.kind() {
                "public" => flags::PUBLIC,
                "private" => flags::PRIVATE,
                "protected" => flags::PROTECTED,
                "static" => flags::STATIC,
                "final" => flags::FINAL,
                "abstract" => flags::ABSTRACT,
                "native" => flags::NATIVE,
                "synchronized" => flags::SYNCHRONIZED,
                "transient" => flags::TRANSIENT,
                "volatile" => flags::VOLATILE,
                "strictfp" => flags::STRICTFP,
                "default" => flags::DEFAULT,
                "sealed" => flags::SEALED,
                "non-sealed" => flags::NON_SEALED,
                "marker_annotation" | "annotation" => {
                    ann.push(self.compact(ch));
                    0
                }
                _ => 0,
            };
        }
        (f, ann)
    }

    /// `@deprecated` in the doc comment right before `n` (JAVA-STUB-021).
    fn javadoc_deprecated(&self, n: Node<'_>) -> bool {
        let mut prev = n.prev_sibling();
        while let Some(p) = prev {
            match p.kind() {
                "block_comment" => {
                    let t = self.text(p);
                    return t.starts_with("/**") && t.contains("@deprecated");
                }
                "line_comment" => prev = p.prev_sibling(),
                _ => return false,
            }
        }
        false
    }

    fn type_params(&self, n: Node<'_>) -> Vec<TypeParamStub> {
        let Some(tp) = n.child_by_field_name("type_parameters") else {
            return Vec::new();
        };
        let mut out = Vec::new();
        let mut c = tp.walk();
        for p in tp.named_children(&mut c).filter(|p| p.kind() == "type_parameter") {
            let mut name = String::new();
            let mut bounds = Vec::new();
            let mut cc = p.walk();
            for ch in p.named_children(&mut cc) {
                match ch.kind() {
                    "type_identifier" | "identifier" if name.is_empty() => name = self.text(ch).to_string(),
                    "type_bound" => {
                        let mut bc = ch.walk();
                        for t in ch.named_children(&mut bc) {
                            bounds.push(self.compact(t));
                        }
                    }
                    _ => {}
                }
            }
            out.push(TypeParamStub { name, bounds });
        }
        out
    }

    fn type_list(&self, n: Node<'_>) -> Vec<String> {
        let mut out = Vec::new();
        let mut c = n.walk();
        for ch in n.named_children(&mut c) {
            if ch.kind() == "type_list" {
                let mut cc = ch.walk();
                out.extend(ch.named_children(&mut cc).map(|t| self.compact(t)));
            } else if !matches!(ch.kind(), "line_comment" | "block_comment") {
                out.push(self.compact(ch));
            }
        }
        out
    }

    fn dims(&self, n: Option<Node<'_>>) -> String {
        n.map(|d| "[]".repeat(self.text(d).matches('[').count())).unwrap_or_default()
    }

    fn params(&self, n: Node<'_>) -> (Vec<ParamStub>, bool) {
        let mut out = Vec::new();
        let mut varargs = false;
        let mut c = n.walk();
        for p in n.named_children(&mut c) {
            match p.kind() {
                "formal_parameter" => {
                    let ty = p.child_by_field_name("type").map(|t| self.compact(t)).unwrap_or_default();
                    let name = p.child_by_field_name("name").map(|t| self.text(t).to_string()).unwrap_or_default();
                    let dims = self.dims(p.child_by_field_name("dimensions"));
                    out.push(ParamStub { name, ty: format!("{ty}{dims}"), varargs: false, generated: false });
                }
                "spread_parameter" => {
                    let mut ty = String::new();
                    let mut name = String::new();
                    let mut pc = p.walk();
                    for ch in p.named_children(&mut pc) {
                        match ch.kind() {
                            "modifiers" | "line_comment" | "block_comment" => {}
                            "variable_declarator" => {
                                if let Some(nm) = ch.child_by_field_name("name") {
                                    name = self.text(nm).to_string();
                                }
                            }
                            _ => ty = self.compact(ch),
                        }
                    }
                    varargs = true;
                    out.push(ParamStub { name, ty: format!("{ty}[]"), varargs: true, generated: false });
                }
                _ => {}
            }
        }
        (out, varargs)
    }

    fn is_constant_candidate(&self, n: Node<'_>) -> bool {
        match n.kind() {
            "decimal_integer_literal" | "hex_integer_literal" | "octal_integer_literal"
            | "binary_integer_literal" | "decimal_floating_point_literal"
            | "hex_floating_point_literal" | "string_literal" | "character_literal" | "true"
            | "false" | "null_literal" | "identifier" | "type_identifier" | "text_block" => true,
            "field_access" | "scoped_identifier" => {
                let mut c = n.walk();
                let ok = n.named_children(&mut c).all(|ch| self.is_constant_candidate(ch));
                ok
            }
            "parenthesized_expression" | "unary_expression" | "binary_expression" | "cast_expression"
            | "ternary_expression" => {
                let mut c = n.walk();
                let ok = n
                    .named_children(&mut c)
                    .all(|ch| self.is_constant_candidate(ch) || matches!(ch.kind(), "integral_type" | "floating_point_type" | "boolean_type" | "type_identifier"));
                ok
            }
            _ => false,
        }
    }

    fn fields(&self, n: Node<'_>, owner_is_interface: bool, out: &mut Vec<FieldStub>) {
        let (mut fl, ann) = self.modifiers(n);
        if owner_is_interface {
            fl |= flags::PUBLIC | flags::STATIC | flags::FINAL;
        }
        let base = n.child_by_field_name("type").map(|t| self.compact(t)).unwrap_or_default();
        let deprecated = self.javadoc_deprecated(n) || ann.iter().any(|a| crate::stub::annotation_short_name(a) == "Deprecated");
        let mut c = n.walk();
        for d in n.children_by_field_name("declarator", &mut c) {
            let name = d.child_by_field_name("name").map(|t| self.text(t).to_string()).unwrap_or_default();
            let dims = self.dims(d.child_by_field_name("dimensions"));
            let init = d
                .child_by_field_name("value")
                .filter(|v| self.is_constant_candidate(*v))
                .map(|v| self.compact(v));
            out.push(FieldStub {
                name,
                flags: fl,
                ty: format!("{base}{dims}"),
                init,
                enum_constant: false,
                deprecated,
                annotations: ann.clone(),
            });
        }
    }

    fn method(&self, n: Node<'_>, class_name: &str, interface: bool) -> MethodStub {
        let (mut fl, ann) = self.modifiers(n);
        let is_ctor = matches!(n.kind(), "constructor_declaration" | "compact_constructor_declaration");
        let (params, varargs) = n
            .child_by_field_name("parameters")
            .map(|p| self.params(p))
            .unwrap_or_default();
        let throws = n
            .children(&mut n.walk())
            .find(|c| c.kind() == "throws")
            .map(|t| self.type_list(t))
            .unwrap_or_default();
        let ret = (!is_ctor).then(|| {
            let t = n.child_by_field_name("type").map(|t| self.compact(t)).unwrap_or_else(|| "void".into());
            format!("{t}{}", self.dims(n.child_by_field_name("dimensions")))
        });
        if interface && fl & (flags::PRIVATE) == 0 {
            // implicit `public` stays implicit (JAVA-STUB-010); abstract-ness is computed from the body
        }
        let has_body = n.child_by_field_name("body").is_some();
        if interface && !has_body && fl & (flags::STATIC | flags::DEFAULT | flags::PRIVATE) == 0 {
            fl |= flags::ABSTRACT;
        }
        let name = if is_ctor {
            class_name.to_string()
        } else {
            n.child_by_field_name("name").map(|t| self.text(t).to_string()).unwrap_or_default()
        };
        MethodStub {
            name,
            flags: fl,
            type_params: self.type_params(n),
            ret,
            params,
            throws,
            deprecated: self.javadoc_deprecated(n) || ann.iter().any(|a| crate::stub::annotation_short_name(a) == "Deprecated"),
            annotations: ann,
            is_ctor,
            varargs,
            has_default: false,
        }
    }

    fn class(&self, n: Node<'_>, scope: &str) -> Option<ClassStub> {
        let name = self.text(n.child_by_field_name("name")?).to_string();
        let qname = if scope.is_empty() { name.clone() } else { format!("{scope}.{name}") };
        let kind = match n.kind() {
            "class_declaration" => ClassKind::Class,
            "interface_declaration" => ClassKind::Interface,
            "enum_declaration" => ClassKind::Enum,
            "record_declaration" => ClassKind::Record,
            _ => ClassKind::Annotation,
        };
        let (fl, ann) = self.modifiers(n);
        let interface = matches!(kind, ClassKind::Interface | ClassKind::Annotation);
        let mut stub = ClassStub {
            name,
            qname: qname.clone(),
            kind,
            flags: fl,
            deprecated: self.javadoc_deprecated(n) || ann.iter().any(|a| crate::stub::annotation_short_name(a) == "Deprecated"),
            type_params: self.type_params(n),
            extends: Vec::new(),
            implements: Vec::new(),
            permits: Vec::new(),
            annotations: ann,
            fields: Vec::new(),
            methods: Vec::new(),
            inner: Vec::new(),
            record_components: Vec::new(),
            source_file: None,
            compiled: false,
            is_static_member: false,
        };
        let mut c = n.walk();
        for ch in n.children(&mut c) {
            match ch.kind() {
                "superclass" => stub.extends = self.type_list(ch),
                "super_interfaces" => stub.implements = self.type_list(ch),
                "extends_interfaces" => stub.extends = self.type_list(ch),
                "permits" => stub.permits = self.type_list(ch),
                _ => {}
            }
        }
        if kind == ClassKind::Record
            && let Some(p) = n.child_by_field_name("parameters")
        {
            stub.record_components = self.params(p).0;
        }
        if let Some(body) = n.child_by_field_name("body") {
            self.members(body, &mut stub, interface);
        }
        // Member types of interfaces, enums and records are implicitly static.
        Some(stub)
    }

    fn members(&self, body: Node<'_>, stub: &mut ClassStub, interface: bool) {
        let mut c = body.walk();
        for m in body.children(&mut c) {
            match m.kind() {
                "field_declaration" | "constant_declaration" => {
                    self.fields(m, interface, &mut stub.fields);
                }
                "method_declaration" | "constructor_declaration" | "compact_constructor_declaration" => {
                    stub.methods.push(self.method(m, &stub.name.clone(), interface));
                }
                "annotation_type_element_declaration" => {
                    let ty = m.child_by_field_name("type").map(|t| self.compact(t)).unwrap_or_default();
                    stub.methods.push(MethodStub {
                        name: m.child_by_field_name("name").map(|t| self.text(t).to_string()).unwrap_or_default(),
                        flags: flags::ABSTRACT,
                        type_params: Vec::new(),
                        ret: Some(ty),
                        params: Vec::new(),
                        throws: Vec::new(),
                        annotations: Vec::new(),
                        is_ctor: false,
                        varargs: false,
                        deprecated: self.javadoc_deprecated(m),
                        has_default: m.child_by_field_name("value").is_some(),
                    });
                }
                "enum_constant" => {
                    let (_, ann) = self.modifiers(m);
                    stub.fields.push(FieldStub {
                        name: m.child_by_field_name("name").map(|t| self.text(t).to_string()).unwrap_or_default(),
                        flags: flags::PUBLIC | flags::STATIC | flags::FINAL,
                        ty: stub.name.clone(),
                        init: None,
                        enum_constant: true,
                        deprecated: self.javadoc_deprecated(m),
                        annotations: ann,
                    });
                }
                "enum_body_declarations" => self.members(m, stub, interface),
                k if is_class_kind(k) => {
                    if let Some(mut inner) = self.class(m, &stub.qname) {
                        inner.is_static_member = interface
                            || matches!(stub.kind, ClassKind::Enum | ClassKind::Record)
                            || inner.flags & flags::STATIC != 0
                            || matches!(inner.kind, ClassKind::Interface | ClassKind::Enum | ClassKind::Record | ClassKind::Annotation);
                        stub.inner.push(inner);
                    }
                }
                _ => {}
            }
        }
    }
}
