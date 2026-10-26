//! Stub trees: the serialized declarations of a file, and the generic
//! builder that derives them from a grammar's tags query.
//!
//! A [`StubTree`] is a flat array of declaration nodes (kind, flags, name,
//! ranges, parent, extras such as `extends`). Unlike IntelliJ we do not
//! rebuild typed PSI from stubs: a stub hit is a navigation target whose
//! range is validated against the live tree by the caller.
//!
//! One tree is built per file revision and shared by the `Stubs` index and
//! the four stub-derived indexes (`ShortName`, `Qualified`, `SuperTypes`,
//! `Members`), see [`crate::indexes`].

use crate::codec::{Reader, Writer};
use archipel_ide_language::structure::{StructureItem, SymbolKind};
use archipel_ide_language::{LanguageId, SyntaxNode, SyntaxSnapshot};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

/// Declaration flags ([`StubNode::flags`]).
pub mod flags {
    pub const PUBLIC: u16 = 1;
    pub const PRIVATE: u16 = 1 << 1;
    pub const PROTECTED: u16 = 1 << 2;
    pub const STATIC: u16 = 1 << 3;
    pub const ABSTRACT: u16 = 1 << 4;
    pub const TEST: u16 = 1 << 5;
    pub const DEPRECATED: u16 = 1 << 6;
    pub const FINAL: u16 = 1 << 7;
}

/// Kind of an extra attached to a stub node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ExtraKey {
    Extends = 0,
    Implements = 1,
    Receiver = 2,
    Annotation = 3,
    TypeText = 4,
    Params = 5,
}

impl ExtraKey {
    fn from_code(code: u8) -> Option<Self> {
        Some(match code {
            0 => Self::Extends,
            1 => Self::Implements,
            2 => Self::Receiver,
            3 => Self::Annotation,
            4 => Self::TypeText,
            5 => Self::Params,
            _ => return None,
        })
    }
}

pub const NO_PARENT: u32 = u32::MAX;

/// One declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StubNode {
    pub kind: SymbolKind,
    pub flags: u16,
    pub name: String,
    /// Bytes of the whole declaration.
    pub range: (u32, u32),
    /// Bytes of the declared name.
    pub name_range: (u32, u32),
    /// Zero-based line of the name.
    pub line: u32,
    /// Index of the containing node, [`NO_PARENT`] at top level.
    pub parent: u32,
    /// Signature text of callables (`(String, int): void`).
    pub detail: Option<String>,
    pub extras: Vec<(ExtraKey, String)>,
}

impl StubNode {
    pub fn extras_of(&self, key: ExtraKey) -> impl Iterator<Item = &str> {
        self.extras
            .iter()
            .filter(move |(k, _)| *k == key)
            .map(|(_, v)| v.as_str())
    }
}

/// The declarations of one file.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StubTree {
    pub lang: String,
    /// Package or namespace of the file (`com.acme.shop`), when it declares one.
    pub package: Option<String>,
    pub nodes: Vec<StubNode>,
    /// `(subtype, supertype)` pairs that are not attached to a declaration
    /// node (Rust `impl Trait for Type`).
    pub relations: Vec<(String, String)>,
}

pub fn kind_code(kind: SymbolKind) -> u8 {
    match kind {
        SymbolKind::Class => 0,
        SymbolKind::Interface => 1,
        SymbolKind::Enum => 2,
        SymbolKind::Function => 3,
        SymbolKind::Method => 4,
        SymbolKind::Constructor => 5,
        SymbolKind::Field => 6,
        SymbolKind::Constant => 7,
        SymbolKind::Module => 8,
        SymbolKind::Type => 9,
        SymbolKind::Macro => 10,
        SymbolKind::Tag => 11,
        SymbolKind::Key => 12,
        SymbolKind::Heading => 13,
        SymbolKind::Other => 14,
    }
}

pub fn kind_from_code(code: u8) -> SymbolKind {
    match code {
        0 => SymbolKind::Class,
        1 => SymbolKind::Interface,
        2 => SymbolKind::Enum,
        3 => SymbolKind::Function,
        4 => SymbolKind::Method,
        5 => SymbolKind::Constructor,
        6 => SymbolKind::Field,
        7 => SymbolKind::Constant,
        8 => SymbolKind::Module,
        9 => SymbolKind::Type,
        10 => SymbolKind::Macro,
        11 => SymbolKind::Tag,
        12 => SymbolKind::Key,
        13 => SymbolKind::Heading,
        _ => SymbolKind::Other,
    }
}

/// Containers whose children are members.
pub fn is_container(kind: SymbolKind) -> bool {
    matches!(
        kind,
        SymbolKind::Class | SymbolKind::Interface | SymbolKind::Enum | SymbolKind::Type
    )
}

/// Kinds that are listed in name indexes (markup and data keys are not
/// symbols to navigate to).
pub fn is_symbol(kind: SymbolKind) -> bool {
    !matches!(
        kind,
        SymbolKind::Tag | SymbolKind::Key | SymbolKind::Heading
    )
}

const FORMAT: u8 = 1;

impl StubTree {
    pub fn encode(&self, out: &mut Vec<u8>) {
        let mut w = Writer::new(out);
        w.u8(FORMAT);
        w.str(&self.lang);
        w.str(self.package.as_deref().unwrap_or(""));
        w.varint(self.nodes.len() as u64);
        for node in &self.nodes {
            w.u8(kind_code(node.kind));
            w.varint(u64::from(node.flags));
            w.str(&node.name);
            w.varint(u64::from(node.range.0));
            w.varint(u64::from(node.range.1));
            w.varint(u64::from(node.name_range.0));
            w.varint(u64::from(node.name_range.1));
            w.varint(u64::from(node.line));
            w.varint(if node.parent == NO_PARENT {
                0
            } else {
                u64::from(node.parent) + 1
            });
            w.str(node.detail.as_deref().unwrap_or(""));
            w.varint(node.extras.len() as u64);
            for (key, value) in &node.extras {
                w.u8(*key as u8);
                w.str(value);
            }
        }
        w.varint(self.relations.len() as u64);
        for (sub, sup) in &self.relations {
            w.str(sub);
            w.str(sup);
        }
    }

    pub fn decode(bytes: &[u8]) -> Option<Self> {
        let mut r = Reader::new(bytes);
        if r.u8()? != FORMAT {
            return None;
        }
        let lang = r.str()?.to_string();
        let package = Some(r.str()?.to_string()).filter(|p| !p.is_empty());
        let count = r.varint()? as usize;
        let mut nodes = Vec::with_capacity(count.min(1 << 16));
        for _ in 0..count {
            let kind = kind_from_code(r.u8()?);
            let flags = u16::try_from(r.varint()?).ok()?;
            let name = r.str()?.to_string();
            let range = (r.u32()?, r.u32()?);
            let name_range = (r.u32()?, r.u32()?);
            let line = r.u32()?;
            let parent = match r.varint()? {
                0 => NO_PARENT,
                n => u32::try_from(n - 1).ok()?,
            };
            let detail = Some(r.str()?.to_string()).filter(|d| !d.is_empty());
            let extra_count = r.varint()? as usize;
            let mut extras = Vec::with_capacity(extra_count.min(64));
            for _ in 0..extra_count {
                let key = ExtraKey::from_code(r.u8()?)?;
                extras.push((key, r.str()?.to_string()));
            }
            nodes.push(StubNode {
                kind,
                flags,
                name,
                range,
                name_range,
                line,
                parent,
                detail,
                extras,
            });
        }
        let relation_count = r.varint()? as usize;
        let mut relations = Vec::with_capacity(relation_count.min(1 << 12));
        for _ in 0..relation_count {
            relations.push((r.str()?.to_string(), r.str()?.to_string()));
        }
        Some(Self {
            lang,
            package,
            nodes,
            relations,
        })
    }

    /// Separator of qualified names in this file's language.
    pub fn separator(&self) -> &'static str {
        match self.lang.as_str() {
            "rust" | "cpp" | "c" => "::",
            _ => ".",
        }
    }

    /// Qualified name of node `index`: package, containers, name.
    pub fn qualified_name(&self, index: usize) -> Option<String> {
        let sep = self.separator();
        let mut parts = Vec::new();
        let mut at = index;
        loop {
            let node = self.nodes.get(at)?;
            parts.push(node.name.as_str());
            if node.parent == NO_PARENT {
                break;
            }
            at = node.parent as usize;
            if parts.len() > 64 {
                break;
            }
        }
        parts.reverse();
        let mut qualified = String::new();
        if let Some(package) = &self.package {
            qualified.push_str(package);
            qualified.push_str(sep);
        }
        qualified.push_str(&parts.join(sep));
        Some(qualified)
    }

    /// Short name of the container of `index`.
    pub fn container_name(&self, index: usize) -> Option<&str> {
        let parent = self.nodes.get(index)?.parent;
        (parent != NO_PARENT)
            .then(|| self.nodes.get(parent as usize))
            .flatten()
            .map(|node| node.name.as_str())
    }

    /// Hash of the tree without positions (the interface hash that drives
    /// the early-cutoff `InterfaceEpoch`).
    pub fn interface_hash(&self) -> u64 {
        let mut hasher = archipel_ide_analysis::interface::Fnv64::default();
        hasher.write(self.lang.as_bytes());
        hasher.write(self.package.as_deref().unwrap_or("").as_bytes());
        for node in &self.nodes {
            hasher.write(&[kind_code(node.kind), 0]);
            hasher.write(&node.flags.to_le_bytes());
            hasher.write(node.name.as_bytes());
            hasher.write(&node.parent.to_le_bytes());
            hasher.write(node.detail.as_deref().unwrap_or("").as_bytes());
            for (key, value) in &node.extras {
                hasher.write(&[*key as u8]);
                hasher.write(value.as_bytes());
            }
            hasher.write(&[0xff]);
        }
        for (sub, sup) in &self.relations {
            hasher.write(sub.as_bytes());
            hasher.write(b"<:");
            hasher.write(sup.as_bytes());
        }
        hasher.finish()
    }
}

/// Builds the stub tree of a parsed file. Languages register their own
/// builder (see [`StubBuilders`]) to enrich the generic one.
pub trait StubBuilder: Send + Sync {
    fn build(&self, syntax: &SyntaxSnapshot) -> StubTree;
}

/// Builders by language, falling back to [`TagsStubBuilder`].
pub struct StubBuilders {
    by_language: RwLock<HashMap<LanguageId, Arc<dyn StubBuilder>>>,
    fallback: Arc<dyn StubBuilder>,
}

impl Default for StubBuilders {
    fn default() -> Self {
        Self {
            by_language: RwLock::new(HashMap::new()),
            fallback: Arc::new(TagsStubBuilder),
        }
    }
}

impl StubBuilders {
    pub fn register(&self, language: LanguageId, builder: Arc<dyn StubBuilder>) {
        self.by_language.write().insert(language, builder);
    }

    pub fn build(&self, syntax: &SyntaxSnapshot) -> StubTree {
        let language = syntax.lang().language_id();
        let builder = self
            .by_language
            .read()
            .get(&language)
            .cloned()
            .unwrap_or_else(|| self.fallback.clone());
        builder.build(syntax)
    }
}

/// Generic builder: declarations from the tags query, enriched with
/// modifiers, annotations, supertypes and the file's package.
#[derive(Debug, Clone, Copy, Default)]
pub struct TagsStubBuilder;

impl StubBuilder for TagsStubBuilder {
    fn build(&self, syntax: &SyntaxSnapshot) -> StubTree {
        let lang = syntax.lang();
        let mut tree = StubTree {
            lang: lang.id.to_string(),
            ..StubTree::default()
        };
        if !lang.has_tags() {
            return tree;
        }
        let items = syntax.structure();
        tree.package = package_of(syntax);
        flatten(&items, NO_PARENT, syntax, &mut tree.nodes);
        if lang.id == "rust" {
            rust_impls(syntax, &mut tree);
        }
        tree
    }
}

fn flatten(items: &[StructureItem], parent: u32, syntax: &SyntaxSnapshot, out: &mut Vec<StubNode>) {
    for item in items {
        let index = out.len() as u32;
        let range = (item.range.start as u32, item.range.end as u32);
        let mut node = StubNode {
            kind: item.kind,
            flags: 0,
            name: item.name.clone(),
            range,
            name_range: (item.name_range.start as u32, item.name_range.end as u32),
            line: item.line as u32,
            parent,
            detail: item.detail.clone(),
            extras: Vec::new(),
        };
        if let Some(declaration) = declaration_node(syntax, range) {
            enrich(syntax, declaration, &mut node);
        }
        out.push(node);
        flatten(&item.children, index, syntax, out);
    }
}

/// The syntax node of the declaration: the deepest node with exactly the
/// item's range.
fn declaration_node(syntax: &SyntaxSnapshot, range: (u32, u32)) -> Option<SyntaxNode<'_>> {
    if range.0 >= range.1 {
        return None;
    }
    let node = syntax.covering(range.0..range.1);
    (node.range() == (range.0..range.1)).then_some(node)
}

fn last_segment(text: &str) -> &str {
    let text = text.trim();
    let text = text.split(['<', '(', '[']).next().unwrap_or(text).trim();
    text.rsplit([':', '.', '\\', '/']).next().unwrap_or(text)
}

fn annotation_name(text: &str) -> String {
    let trimmed = text
        .trim()
        .trim_start_matches("#!")
        .trim_start_matches('#')
        .trim_start_matches('[')
        .trim_start_matches('@')
        .trim_end_matches(']');
    last_segment(trimmed).to_string()
}

fn is_annotation_kind(kind: &str) -> bool {
    kind.contains("annotation") || kind.contains("attribute") || kind.contains("decorator")
}

fn apply_annotation(name: &str, node: &mut StubNode) {
    if name.is_empty() {
        return;
    }
    let lower = name.to_lowercase();
    if lower.contains("test") {
        node.flags |= flags::TEST;
    }
    if lower == "deprecated" {
        node.flags |= flags::DEPRECATED;
    }
    node.extras.push((ExtraKey::Annotation, name.to_string()));
}

fn apply_modifier_words(text: &str, node: &mut StubNode) {
    for word in text.split_whitespace() {
        if word.starts_with('@') {
            continue;
        }
        let word = word.split('(').next().unwrap_or(word);
        node.flags |= match word {
            "public" | "pub" | "export" | "open" => flags::PUBLIC,
            "private" => flags::PRIVATE,
            "protected" => flags::PROTECTED,
            "static" | "companion" => flags::STATIC,
            "abstract" => flags::ABSTRACT,
            "final" | "const" => flags::FINAL,
            _ => 0,
        };
    }
}

fn enrich(syntax: &SyntaxSnapshot, declaration: SyntaxNode<'_>, node: &mut StubNode) {
    // Attributes, annotations and decorators written before the item.
    let mut previous = declaration.prev_sibling();
    let mut steps = 0;
    while let Some(sibling) = previous {
        if !is_annotation_kind(sibling.kind()) || steps > 8 {
            break;
        }
        apply_annotation(&annotation_name(sibling.text()), node);
        previous = sibling.prev_sibling();
        steps += 1;
    }
    scan_declaration(declaration, node);
    // Name-based conventions of languages without modifiers.
    match syntax.lang().id {
        "python" => {
            if node.name.starts_with('_') && !node.name.starts_with("__") {
                node.flags |= flags::PRIVATE;
            } else if !node.name.starts_with('_') {
                node.flags |= flags::PUBLIC;
            }
            if node.name.starts_with("test") {
                node.flags |= flags::TEST;
            }
        }
        "go" => {
            if node.name.chars().next().is_some_and(char::is_uppercase) {
                node.flags |= flags::PUBLIC;
                if node.name.starts_with("Test") && node.kind == SymbolKind::Function {
                    node.flags |= flags::TEST;
                }
            } else {
                node.flags |= flags::PRIVATE;
            }
        }
        _ => {}
    }
}

fn scan_declaration(declaration: SyntaxNode<'_>, node: &mut StubNode) {
    for (position, child) in declaration.children().enumerate() {
        let kind = child.kind();
        if kind.contains("body") || kind.ends_with("block") || kind == "{" {
            break;
        }
        if position > 24 {
            break;
        }
        if kind.contains("modifier") || kind.contains("visibility") {
            apply_modifier_words(child.text(), node);
            for inner in child.children() {
                if is_annotation_kind(inner.kind()) {
                    apply_annotation(&annotation_name(inner.text()), node);
                }
            }
        } else if is_annotation_kind(kind) {
            apply_annotation(&annotation_name(child.text()), node);
        } else if let Some(key) = heritage_key(kind) {
            let mut names = Vec::new();
            collect_type_names(child, &mut names, 0);
            for name in names {
                if name != node.name {
                    node.extras.push((key, name));
                }
            }
        }
    }
}

fn heritage_key(kind: &str) -> Option<ExtraKey> {
    if kind.contains("implements") || kind == "super_interfaces" || kind.contains("interfaces") {
        return Some(ExtraKey::Implements);
    }
    if kind.contains("superclass")
        || kind.contains("extends")
        || kind.contains("supertrait")
        || kind == "trait_bounds"
        || kind.contains("heritage")
        || kind.contains("delegation_specifier")
        || kind.contains("base_class")
        || kind == "superclasses"
    {
        return Some(ExtraKey::Extends);
    }
    None
}

fn collect_type_names(node: SyntaxNode<'_>, out: &mut Vec<String>, depth: usize) {
    if depth > 6 || out.len() > 16 {
        return;
    }
    let kind = node.kind();
    if kind.contains("type_arguments")
        || kind.contains("type_parameters")
        || kind == "generic_type_arguments"
    {
        return;
    }
    if kind.contains("scoped")
        || kind.contains("qualified")
        || kind == "navigation_expression"
        || kind == "attribute"
        || kind == "field_expression"
        || kind == "nested_type_identifier"
    {
        let name = last_segment(node.text());
        if !name.is_empty() {
            out.push(name.to_string());
        }
        return;
    }
    let leaf = node.first_child().is_none();
    if leaf && (kind.ends_with("identifier") || kind == "name" || kind == "constant") {
        let text = node.text().trim();
        if !text.is_empty() {
            out.push(text.to_string());
        }
        return;
    }
    for child in node.named_children() {
        collect_type_names(child, out, depth + 1);
    }
}

fn package_of(syntax: &SyntaxSnapshot) -> Option<String> {
    let root = syntax.root();
    for child in root.named_children().take(8) {
        let name = match child.kind() {
            "package_declaration" | "package_header" | "namespace_definition" => {
                let text = child.text().trim();
                let text = text
                    .trim_start_matches("package")
                    .trim_start_matches("namespace")
                    .trim()
                    .trim_end_matches(';')
                    .trim();
                let text = text.split(['{', '\n']).next().unwrap_or(text).trim();
                Some(text.to_string())
            }
            "package_clause" => child
                .named_children()
                .find(|c| c.kind().contains("identifier"))
                .map(|c| c.text().to_string()),
            _ => None,
        };
        if let Some(name) = name.filter(|n| !n.is_empty()) {
            return Some(name);
        }
    }
    None
}

/// `impl Trait for Type` and `impl<T> Trait for Type<T>`.
fn rust_impls(syntax: &SyntaxSnapshot, tree: &mut StubTree) {
    for item in syntax.root().named_children() {
        collect_impls(item, tree, 0);
    }
}

fn collect_impls(node: SyntaxNode<'_>, tree: &mut StubTree, depth: usize) {
    if depth > 4 {
        return;
    }
    if node.kind() == "impl_item" {
        if let (Some(trait_node), Some(type_node)) = (
            node.child_by_field_name("trait"),
            node.child_by_field_name("type"),
        ) {
            let sup = last_segment(trait_node.text());
            let sub = last_segment(type_node.text());
            if !sup.is_empty() && !sub.is_empty() {
                tree.relations.push((sub.to_string(), sup.to_string()));
            }
        }
        return;
    }
    if node.kind() == "mod_item" || node.kind() == "declaration_list" {
        for child in node.named_children() {
            collect_impls(child, tree, depth + 1);
        }
    }
}

/// Interface hash from the stub tree (declarations, flags and supertypes
/// without positions): the early-cutoff input of the host. Register it for
/// the "any" language ahead of the structure-based default.
#[derive(Debug, Clone, Copy, Default)]
pub struct StubInterface;

impl archipel_ide_analysis::InterfaceSummarizer for StubInterface {
    fn interface_hash(&self, snap: &SyntaxSnapshot) -> u64 {
        if snap.lang().has_tags() {
            TagsStubBuilder.build(snap).interface_hash()
        } else {
            archipel_ide_analysis::interface::StructureInterface.interface_hash(snap)
        }
    }
}
