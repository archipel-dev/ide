//! File structure (Structure tool window, File Structure popup, breadcrumbs)
//! from the grammars' tags queries: `@definition.<kind>` captures with an
//! `@name`, nested by containment.

use crate::queryset::{self, QueryKind};
use crate::registry::LanguageSpec;
use std::collections::HashSet;
use std::ops::Range;
use tree_sitter::{Node, QueryCursor, StreamingIterator, Tree};

/// What a structure element declares (from the `@definition.<kind>` suffix).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SymbolKind {
    Class,
    Interface,
    Enum,
    Function,
    Method,
    Constructor,
    Field,
    Constant,
    Module,
    Type,
    Macro,
    /// Markup element (XML/HTML tag).
    Tag,
    /// Data key (JSON/YAML/TOML).
    Key,
    /// Document section (Markdown heading, TOML table).
    Heading,
    Other,
}

impl SymbolKind {
    fn from_capture(kind: &str) -> Self {
        match kind {
            "class" | "struct" | "object" | "record" => Self::Class,
            "interface" | "trait" | "protocol" => Self::Interface,
            "enum" => Self::Enum,
            "function" => Self::Function,
            "method" => Self::Method,
            "constructor" => Self::Constructor,
            "field" | "property" => Self::Field,
            "constant" | "variable" => Self::Constant,
            "module" | "namespace" | "package" => Self::Module,
            "type" => Self::Type,
            "macro" => Self::Macro,
            "tag" | "element" => Self::Tag,
            "key" => Self::Key,
            "heading" | "section" => Self::Heading,
            _ => Self::Other,
        }
    }
}

/// Declared accessibility of a member.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Visibility {
    #[default]
    Unspecified,
    Public,
    Protected,
    /// Package-private, `internal`, `pub(crate)`.
    Internal,
    Private,
}

/// Modifiers of a declaration, read from `@modifier` captures and from the
/// attribute-like siblings in front of it (`#[test]`, decorators).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Modifiers {
    pub visibility: Visibility,
    pub is_static: bool,
    pub is_abstract: bool,
    pub is_final: bool,
    pub is_async: bool,
    pub is_override: bool,
    pub is_test: bool,
    pub is_deprecated: bool,
}

impl Modifiers {
    /// Reads the modifier words of `text` (`public static final`,
    /// `pub(crate) async`, `@Test`, `#[deprecated]`).
    pub fn from_text(text: &str) -> Self {
        let mut out = Modifiers::default();
        let words: Vec<&str> = text
            .split(|c: char| !(c.is_alphanumeric() || c == '_'))
            .filter(|word| !word.is_empty())
            .collect();
        for (index, word) in words.iter().enumerate() {
            match *word {
                "public" => out.visibility = Visibility::Public,
                "pub" => {
                    let restricted = matches!(
                        words.get(index + 1),
                        Some(&"crate" | &"super" | &"in" | &"self")
                    );
                    out.visibility = if restricted {
                        Visibility::Internal
                    } else {
                        Visibility::Public
                    };
                }
                "protected" => out.visibility = Visibility::Protected,
                "private" | "priv" => out.visibility = Visibility::Private,
                "internal" => out.visibility = Visibility::Internal,
                "static" => out.is_static = true,
                "abstract" => out.is_abstract = true,
                "final" | "const" | "readonly" => out.is_final = true,
                "async" | "suspend" => out.is_async = true,
                "override" => out.is_override = true,
                "test" | "Test" | "ParameterizedTest" | "RepeatedTest" => out.is_test = true,
                "deprecated" | "Deprecated" => out.is_deprecated = true,
                _ => {}
            }
        }
        out
    }

    /// Union of two readings of the same declaration.
    pub fn merge(&mut self, other: Modifiers) {
        if self.visibility == Visibility::Unspecified {
            self.visibility = other.visibility;
        }
        self.is_static |= other.is_static;
        self.is_abstract |= other.is_abstract;
        self.is_final |= other.is_final;
        self.is_async |= other.is_async;
        self.is_override |= other.is_override;
        self.is_test |= other.is_test;
        self.is_deprecated |= other.is_deprecated;
    }
}

/// One declaration in the file and the declarations nested in it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructureItem {
    pub name: String,
    pub kind: SymbolKind,
    /// Parameter list for callables (`(String name)`), whitespace-collapsed.
    pub detail: Option<String>,
    /// Bytes of the whole declaration.
    pub range: Range<usize>,
    /// Bytes of the declared name (the navigation target).
    pub name_range: Range<usize>,
    /// Zero-based line of the name.
    pub line: usize,
    pub modifiers: Modifiers,
    pub children: Vec<StructureItem>,
}

fn compact(text: &str, limit: usize) -> String {
    let collapsed = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut out: String = collapsed.chars().take(limit).collect();
    if collapsed.chars().count() > limit {
        out.push('…');
    }
    out
}

/// A type annotation's text without its leading `:` (TypeScript).
fn type_text(text: &str) -> &str {
    text.trim_start_matches(':').trim_start()
}

/// The type of a parameter or declaration: its `type` field, else its last
/// named child whose kind names a type.
fn type_of<'a>(node: Node<'a>) -> Option<Node<'a>> {
    node.child_by_field_name("type")
        .or_else(|| node.child_by_field_name("return_type"))
        .or_else(|| {
            let mut cursor = node.walk();
            node.named_children(&mut cursor)
                .filter(|child| {
                    child.kind().contains("type") && !child.kind().contains("parameter")
                })
                .last()
        })
}

/// IntelliJ-style signature `(String, int): String` built from the parameter
/// and return types in the tree; falls back to the parameter list text.
fn signature(node: Node<'_>, source: &str) -> Option<String> {
    let params = node.child_by_field_name("parameters").or_else(|| {
        let mut cursor = node.walk();
        node.named_children(&mut cursor)
            .find(|child| child.kind().contains("parameters"))
    })?;
    let mut cursor = params.walk();
    let parameters: Vec<Node<'_>> = params
        .named_children(&mut cursor)
        .filter(|child| !child.kind().contains("comment"))
        .collect();
    let types: Option<Vec<String>> = parameters
        .iter()
        .map(|param| {
            let ty = type_of(*param)?;
            let mut ty_text = compact(type_text(source.get(ty.byte_range())?), 40);
            // Java `String args[]` keeps its dimensions next to the name.
            if let Some(dims) = param.child_by_field_name("dimensions") {
                ty_text.push_str(source.get(dims.byte_range()).unwrap_or_default());
            }
            Some(ty_text)
        })
        .collect();
    let mut detail = match types {
        Some(types) => format!("({})", types.join(", ")),
        None => compact(source.get(params.byte_range())?, 80),
    };
    // The return type follows the parameters (`String f()`, `fun f(): String`).
    if let Some(ret) = node
        .child_by_field_name("type")
        .or_else(|| node.child_by_field_name("return_type"))
        .or_else(|| {
            let mut cursor = node.walk();
            node.named_children(&mut cursor).find(|child| {
                child.start_byte() >= params.end_byte() && child.kind().contains("type")
            })
        })
    {
        detail.push_str(": ");
        detail.push_str(&compact(type_text(source.get(ret.byte_range())?), 40));
    }
    Some(detail)
}

/// Ranges of the attribute-like siblings (`@doc.skip`) of the document.
fn attribute_nodes(spec: &'static LanguageSpec, tree: &Tree, source: &str) -> HashSet<usize> {
    let mut out = HashSet::new();
    let Some(query) = queryset::query(spec, QueryKind::Doc) else {
        return out;
    };
    let names = query.capture_names();
    let mut cursor = QueryCursor::new();
    let mut captures = cursor.captures(query, tree.root_node(), source.as_bytes());
    while let Some((found, index)) = captures.next() {
        let capture = found.captures[*index];
        if names[capture.index as usize] == "doc.skip" {
            out.insert(capture.node.id());
        }
    }
    out
}

/// Builds the structure tree of a parsed document. Empty when the language
/// has no tags query.
pub fn structure(spec: &'static LanguageSpec, tree: &Tree, source: &str) -> Vec<StructureItem> {
    let Some(query) = queryset::query(spec, QueryKind::Tags) else {
        return Vec::new();
    };
    let names = query.capture_names();
    let attributes = attribute_nodes(spec, tree, source);
    let mut flat: Vec<StructureItem> = Vec::new();
    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(query, tree.root_node(), source.as_bytes());
    while let Some(query_match) = matches.next() {
        let mut name = None;
        let mut definition = None;
        let mut modifiers = Modifiers::default();
        let mut detail_text: Vec<&str> = Vec::new();
        for capture in query_match.captures {
            let capture_name = names[capture.index as usize];
            if capture_name == "name" {
                name = Some(capture.node);
            } else if let Some(kind) = capture_name.strip_prefix("definition.") {
                definition = Some((capture.node, SymbolKind::from_capture(kind)));
            } else if capture_name == "modifier" || capture_name.starts_with("modifier.") {
                modifiers.merge(Modifiers::from_text(text_of(source, capture.node)));
            } else if capture_name == "detail" {
                detail_text.push(text_of(source, capture.node));
            }
        }
        let (Some(name), Some((node, kind))) = (name, definition) else {
            continue;
        };
        let Some(text) = source.get(name.byte_range()) else {
            continue;
        };
        // Decorators and attributes in front of the declaration.
        let mut sibling = node.prev_named_sibling();
        while let Some(previous) = sibling.filter(|previous| attributes.contains(&previous.id())) {
            modifiers.merge(Modifiers::from_text(text_of(source, previous)));
            sibling = previous.prev_named_sibling();
        }
        let signature = matches!(
            kind,
            SymbolKind::Function | SymbolKind::Method | SymbolKind::Constructor
        )
        .then(|| signature(node, source))
        .flatten();
        let detail = signature.or_else(|| {
            let joined = compact(&detail_text.join(" "), 60);
            (!joined.is_empty()).then(|| {
                let prefix = queryset::setting(query, query_match.pattern_index, "detail.prefix")
                    .unwrap_or_default();
                format!("{prefix}{joined}")
            })
        });
        flat.push(StructureItem {
            name: text.to_owned(),
            kind,
            detail,
            range: node.byte_range(),
            name_range: name.byte_range(),
            line: name.start_position().row,
            modifiers,
            children: Vec::new(),
        });
    }
    // One element per declaration: the earliest pattern wins (e.g. Rust's
    // `method` inside an impl before the generic `function`); the modifiers
    // of the other readings are merged in.
    flat.sort_by(|a, b| {
        a.range
            .start
            .cmp(&b.range.start)
            .then(b.range.end.cmp(&a.range.end))
    });
    flat.dedup_by(|later, earlier| {
        let same = later.name_range == earlier.name_range;
        if same {
            earlier.modifiers.merge(later.modifiers);
            if earlier.detail.is_none() {
                earlier.detail = later.detail.take();
            }
        }
        same
    });
    nest(flat)
}

fn text_of<'a>(source: &'a str, node: Node<'_>) -> &'a str {
    source.get(node.byte_range()).unwrap_or_default()
}

/// Nests sorted, flat items by range containment.
fn nest(flat: Vec<StructureItem>) -> Vec<StructureItem> {
    let mut roots: Vec<StructureItem> = Vec::new();
    let mut stack: Vec<StructureItem> = Vec::new();
    for item in flat {
        while let Some(top) = stack.last() {
            if item.range.start >= top.range.start && item.range.end <= top.range.end {
                break;
            }
            let done = stack.pop().expect("non-empty");
            match stack.last_mut() {
                Some(parent) => parent.children.push(done),
                None => roots.push(done),
            }
        }
        stack.push(item);
    }
    while let Some(done) = stack.pop() {
        match stack.last_mut() {
            Some(parent) => parent.children.push(done),
            None => roots.push(done),
        }
    }
    roots
}

/// The chain of declarations containing `byte`, outermost first (breadcrumbs).
pub fn path_at(items: &[StructureItem], byte: usize) -> Vec<&StructureItem> {
    let mut path = Vec::new();
    let mut level = items;
    while let Some(item) = level
        .iter()
        .find(|item| item.range.start <= byte && byte <= item.range.end)
    {
        path.push(item);
        level = &item.children;
    }
    path
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SyntaxTree, registry};

    fn items(id: &str, source: &str) -> Vec<StructureItem> {
        let spec = registry::by_id(id).expect("language");
        let syntax = SyntaxTree::parse(spec, source).expect("parse");
        structure(spec, syntax.tree(), source)
    }

    fn outline(items: &[StructureItem], depth: usize, out: &mut Vec<String>) {
        for item in items {
            out.push(format!(
                "{}{:?} {}{}",
                "  ".repeat(depth),
                item.kind,
                item.name,
                item.detail.as_deref().unwrap_or("")
            ));
            outline(&item.children, depth + 1, out);
        }
    }

    fn render(id: &str, source: &str) -> Vec<String> {
        let mut out = Vec::new();
        outline(&items(id, source), 0, &mut out);
        out
    }

    #[test]
    fn java_classes_members_and_constructors() {
        let source = "package a;\n\npublic class App {\n    private static final int MAX = 1;\n    App() {}\n    public String greeting(String name) { return name; }\n    public static void main(String[] args) {}\n    enum Mode { ON, OFF }\n}\n";
        assert_eq!(
            render("java", source),
            vec![
                "Class App",
                "  Field MAX: int",
                "  Constructor App()",
                "  Method greeting(String): String",
                "  Method main(String[]): void",
                "  Enum Mode",
                "    Constant ON",
                "    Constant OFF",
            ]
        );
    }

    #[test]
    fn kotlin_and_rust_structure() {
        let kotlin = "class MainActivity : AppCompatActivity() {\n    override fun onCreate(savedInstanceState: Bundle?) {\n    }\n    private fun greet(name: String): String = \"Hello $name!\"\n}\n";
        assert_eq!(
            render("kotlin", kotlin),
            vec![
                "Class MainActivity",
                "  Function onCreate(Bundle?)",
                "  Function greet(String): String",
            ]
        );
        let rust = "struct S;\nimpl S {\n    fn new() -> Self { S }\n}\nfn main() {}\n";
        let rendered = render("rust", rust);
        assert!(rendered.contains(&"Class S".to_owned()), "{rendered:?}");
        assert!(
            rendered.iter().any(|line| line.contains("new()")),
            "{rendered:?}"
        );
        assert!(
            rendered.contains(&"Function main()".to_owned()),
            "{rendered:?}"
        );
    }

    #[test]
    fn markup_and_data_languages_have_structure() {
        assert_eq!(
            render(
                "markdown",
                "# Title\n\ntext\n\n## Usage\n\nmore\n\n## API\n"
            ),
            vec!["Heading Title", "  Heading Usage", "  Heading API"]
        );
        assert_eq!(
            render(
                "xml",
                "<LinearLayout a=\"b\">\n  <TextView/>\n  <Button></Button>\n</LinearLayout>\n"
            ),
            vec!["Tag LinearLayout", "  Tag TextView", "  Tag Button"]
        );
        assert_eq!(
            render("json", "{\"name\": \"x\", \"deps\": {\"a\": 1}}"),
            vec!["Key name", "Key deps", "  Key a"]
        );
        assert_eq!(
            render("yaml", "build:\n  steps:\n    - run: x\nname: y\n"),
            vec!["Key build", "  Key steps", "    Key run", "Key name"]
        );
        assert_eq!(
            render(
                "toml",
                "[package]\nname = \"x\"\n\n[dependencies]\nserde = \"1\"\n"
            ),
            vec![
                "Heading package",
                "  Key name",
                "Heading dependencies",
                "  Key serde"
            ]
        );
        assert_eq!(
            render("html", "<html><body><p>hi</p></body></html>"),
            vec!["Tag html", "  Tag body", "    Tag p"]
        );
    }

    #[test]
    fn breadcrumb_path_follows_containment() {
        let source = "class A {\n    void f() {\n        int x = 1;\n    }\n}\n";
        let found = items("java", source);
        let at = source.find("int x").unwrap();
        let names: Vec<_> = path_at(&found, at)
            .iter()
            .map(|item| item.name.as_str())
            .collect();
        assert_eq!(names, vec!["A", "f"]);
        assert!(path_at(&found, source.len() + 10).is_empty());
    }

    #[test]
    fn every_bundled_tags_query_compiles() {
        for spec in crate::registry::LANGUAGES
            .iter()
            .filter(|spec| spec.has_tags())
        {
            assert!(
                queryset::query(spec, QueryKind::Tags).is_some(),
                "{} tags do not compile",
                spec.id
            );
        }
    }
}
