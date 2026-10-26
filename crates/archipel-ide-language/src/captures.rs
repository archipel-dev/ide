//! Maps tree-sitter highlight capture names (the shared vocabulary used by the
//! grammar crates' `highlights.scm`) to IntelliJ text-attribute keys.
//!
//! Lookup is hierarchical, like `tree-sitter-highlight`: `function.method.call`
//! falls back to `function.method`, then `function`.

/// Generic capture → attribute key table. Keys are IntelliJ `TextAttributesKey`
/// names resolved by [`crate::scheme::ColorScheme`].
const CAPTURE_KEYS: &[(&str, &str)] = &[
    ("keyword", "DEFAULT_KEYWORD"),
    ("include", "DEFAULT_KEYWORD"),
    ("import", "DEFAULT_KEYWORD"),
    ("repeat", "DEFAULT_KEYWORD"),
    ("conditional", "DEFAULT_KEYWORD"),
    ("exception", "DEFAULT_KEYWORD"),
    ("preproc", "DEFAULT_KEYWORD"),
    ("storageclass", "DEFAULT_KEYWORD"),
    ("media", "DEFAULT_KEYWORD"),
    ("supports", "DEFAULT_KEYWORD"),
    ("keyframes", "DEFAULT_KEYWORD"),
    ("charset", "DEFAULT_KEYWORD"),
    ("string", "DEFAULT_STRING"),
    ("string.escape", "DEFAULT_VALID_STRING_ESCAPE"),
    ("escape", "DEFAULT_VALID_STRING_ESCAPE"),
    ("string.special.key", "DEFAULT_INSTANCE_FIELD"),
    ("character", "DEFAULT_STRING"),
    ("number", "DEFAULT_NUMBER"),
    ("float", "DEFAULT_NUMBER"),
    ("boolean", "DEFAULT_KEYWORD"),
    ("constant.builtin", "DEFAULT_KEYWORD"),
    ("constant", "DEFAULT_CONSTANT"),
    ("comment", "DEFAULT_LINE_COMMENT"),
    ("comment.documentation", "DEFAULT_DOC_COMMENT"),
    ("function", "DEFAULT_FUNCTION_CALL"),
    ("method", "DEFAULT_FUNCTION_CALL"),
    ("function.macro", "DEFAULT_FUNCTION_DECLARATION"),
    ("constructor", "DEFAULT_CLASS_REFERENCE"),
    ("type", "DEFAULT_CLASS_REFERENCE"),
    ("type.builtin", "DEFAULT_KEYWORD"),
    ("property", "DEFAULT_INSTANCE_FIELD"),
    ("field", "DEFAULT_INSTANCE_FIELD"),
    ("attribute", "DEFAULT_METADATA"),
    ("variable", "DEFAULT_IDENTIFIER"),
    ("variable.builtin", "DEFAULT_KEYWORD"),
    ("parameter", "DEFAULT_IDENTIFIER"),
    ("label", "DEFAULT_LABEL"),
    ("operator", "DEFAULT_OPERATION_SIGN"),
    ("punctuation", "DEFAULT_OPERATION_SIGN"),
    ("punctuation.bracket", "DEFAULT_BRACES"),
    ("punctuation.delimiter", "DEFAULT_COMMA"),
    ("delimiter", "DEFAULT_COMMA"),
    ("tag", "XML_TAG_NAME"),
    ("markup.heading", "DEFAULT_KEYWORD"),
    ("text.title", "DEFAULT_KEYWORD"),
    ("markup.raw", "DEFAULT_STRING"),
    ("text.literal", "DEFAULT_STRING"),
    ("markup.link", "MARKDOWN_LINK_TEXT"),
    ("text.uri", "MARKDOWN_AUTO_LINK"),
    ("text.reference", "MARKDOWN_LINK_TEXT"),
];

/// Resolves a capture name to an attribute key. Language overrides are
/// consulted first at every level of the hierarchy. Captures that start with
/// `_` are query-internal and never highlight.
pub fn capture_key(name: &str, overrides: &[(&'static str, &'static str)]) -> Option<&'static str> {
    if name.starts_with('_') {
        return None;
    }
    let mut candidate = name;
    loop {
        if let Some((_, key)) = overrides
            .iter()
            .chain(CAPTURE_KEYS)
            .find(|(capture, _)| *capture == candidate)
        {
            return Some(key);
        }
        candidate = &candidate[..candidate.rfind('.')?];
    }
}

/// Refines a resolved key using the captured text and its syntax node, for
/// distinctions IntelliJ makes that queries do not (doc vs. line comments,
/// declarations vs. calls). Purely structural: no language-specific rules.
pub(crate) fn refine(
    capture: &str,
    key: &'static str,
    text: &str,
    node: tree_sitter::Node<'_>,
) -> &'static str {
    match key {
        "DEFAULT_LINE_COMMENT" | "DEFAULT_DOC_COMMENT" => comment_key(text, key),
        "DEFAULT_KEYWORD" | "DEFAULT_CONSTANT" if is_numeric(text) => "DEFAULT_NUMBER",
        // `type.builtin` covers both primitives (`int`, `u32`: keywords in
        // IntelliJ) and library types (`String`, `Int`: class references).
        "DEFAULT_KEYWORD"
            if capture.starts_with("type.builtin")
                && text.starts_with(|c: char| c.is_ascii_uppercase()) =>
        {
            "DEFAULT_CLASS_REFERENCE"
        }
        _ if is_callable_capture(capture) && is_declaration_name(node) => {
            "DEFAULT_FUNCTION_DECLARATION"
        }
        _ => key,
    }
}

/// Captures naming functions, methods or constructors, which IntelliJ colors
/// differently at their declaration than at call sites.
fn is_callable_capture(name: &str) -> bool {
    let root = name.split('.').next().unwrap_or_default();
    matches!(root, "function" | "method" | "constructor")
        && !name.starts_with("function.builtin")
        && !name.starts_with("function.macro")
}

/// Some grammars capture numeric literals as `@constant.builtin`.
fn is_numeric(text: &str) -> bool {
    let text = text.trim_start_matches(['-', '+']);
    text.starts_with(|c: char| c.is_ascii_digit())
        || (text.starts_with('.') && text[1..].starts_with(|c: char| c.is_ascii_digit()))
}

fn comment_key(text: &str, fallback: &'static str) -> &'static str {
    let text = text.trim_start();
    if (text.starts_with("/**") && !text.starts_with("/**/"))
        || (text.starts_with("///") && !text.starts_with("////"))
        || text.starts_with("//!")
        || text.starts_with("/*!")
    {
        "DEFAULT_DOC_COMMENT"
    } else if text.starts_with("/*") {
        "DEFAULT_BLOCK_COMMENT"
    } else {
        fallback
    }
}

/// True when `node` names the declaration it belongs to (function, method or
/// constructor), judged from grammar node-type naming conventions.
fn is_declaration_name(node: tree_sitter::Node<'_>) -> bool {
    let Some(parent) = node.parent() else {
        return false;
    };
    let kind = parent.kind();
    let declaration = kind.contains("declaration")
        || kind.contains("definition")
        || kind.contains("declarator")
        || kind.ends_with("_item")
        || kind.ends_with("_signature");
    if !declaration {
        return false;
    }
    // Only the declared name, not e.g. a call inside a default argument. For
    // grammars without a `name` field, the first identifier child is the name.
    let name = parent.child_by_field_name("name").or_else(|| {
        let mut cursor = parent.walk();
        parent
            .named_children(&mut cursor)
            .find(|child| child.kind().contains("identifier"))
    });
    name.is_some_and(|name| name.id() == node.id())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hierarchical_fallback_and_overrides() {
        assert_eq!(capture_key("keyword", &[]), Some("DEFAULT_KEYWORD"));
        assert_eq!(capture_key("keyword.return", &[]), Some("DEFAULT_KEYWORD"));
        assert_eq!(
            capture_key("function.method.call", &[]),
            Some("DEFAULT_FUNCTION_CALL")
        );
        assert_eq!(capture_key("type.builtin", &[]), Some("DEFAULT_KEYWORD"));
        assert_eq!(
            capture_key("string.special.symbol", &[]),
            Some("DEFAULT_STRING")
        );
        assert_eq!(capture_key("_name", &[]), None);
        assert_eq!(capture_key("spell", &[]), None);
        assert_eq!(
            capture_key("tag", &[("tag", "HTML_TAG_NAME")]),
            Some("HTML_TAG_NAME")
        );
    }

    #[test]
    fn numeric_constants_are_numbers() {
        assert!(is_numeric("42"));
        assert!(is_numeric("-1.5e3"));
        assert!(is_numeric(".5"));
        assert!(!is_numeric("true"));
        assert!(!is_numeric("."));
    }

    #[test]
    fn comment_kinds_follow_their_delimiters() {
        assert_eq!(comment_key("/** doc */", "x"), "DEFAULT_DOC_COMMENT");
        assert_eq!(comment_key("/**/", "x"), "DEFAULT_BLOCK_COMMENT");
        assert_eq!(comment_key("/* block */", "x"), "DEFAULT_BLOCK_COMMENT");
        assert_eq!(comment_key("/// doc", "x"), "DEFAULT_DOC_COMMENT");
        assert_eq!(comment_key("//// rule", "x"), "x");
        assert_eq!(comment_key("# hash", "x"), "x");
    }
}
