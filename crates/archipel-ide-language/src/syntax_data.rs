//! Per-language editing data that is not a tree pattern: comment tokens and
//! block-comment delimiters (the `Commenter`), auto-paired characters, doc
//! comment markers, region-fold markers and keyword sets for completion.
//!
//! Everything here is data keyed by the registry id of a language; the
//! generic services read it so a new grammar needs one row, not code. A test
//! checks that every registered language has a row.

/// How statements end and blocks open (Complete Current Statement).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatementStyle {
    /// No statement structure (data and markup languages).
    None,
    /// `{}` blocks; statements end with `;` when `semicolon`.
    Braces { semicolon: bool },
    /// Blocks open with `:` and are indented (Python, Starlark).
    Indent,
    /// `then`/`do` ... `fi`/`done` (shell).
    Shell,
}

/// Editing data of one language. See the module docs.
#[derive(Debug, Clone, Copy)]
pub struct LanguageData {
    /// Registry id of the language (`LanguageDef::id`).
    pub id: &'static str,
    /// Line comment prefixes; the first one is used to comment lines.
    pub line_comments: &'static [&'static str],
    /// Block comment delimiters (`/*`, `*/`).
    pub block_comment: Option<(&'static str, &'static str)>,
    /// Block comments of the language nest (Rust, Kotlin).
    pub nested_block: bool,
    /// Prefix that makes a line comment a documentation comment.
    pub doc_line_prefix: Option<&'static str>,
    /// Opening delimiter that makes a block comment a documentation comment.
    pub doc_block_open: Option<&'static str>,
    /// Characters whose opening form is auto-paired (`(`→`)`, ...).
    pub pairs: &'static [(char, char)],
    /// Quote characters that are auto-paired.
    pub quotes: &'static [char],
    /// The language has triple-quoted strings (`"""`, `'''`).
    pub triple_quotes: bool,
    /// Reserved words and common literals, for keyword completion.
    pub keywords: &'static [&'static str],
    pub statements: StatementStyle,
}

const BRACKETS: &[(char, char)] = &[('(', ')'), ('[', ']'), ('{', '}')];
const C_LIKE_QUOTES: &[char] = &['"', '\''];
const WEB_QUOTES: &[char] = &['"', '\'', '`'];
const NONE: &[&str] = &[];
const SLASH: &[&str] = &["//"];
const HASH: &[&str] = &["#"];
const C_BLOCK: Option<(&str, &str)> = Some(("/*", "*/"));
const HTML_BLOCK: Option<(&str, &str)> = Some(("<!--", "-->"));

const EMPTY: LanguageData = LanguageData {
    id: "",
    line_comments: NONE,
    block_comment: None,
    nested_block: false,
    doc_line_prefix: None,
    doc_block_open: None,
    pairs: BRACKETS,
    quotes: C_LIKE_QUOTES,
    triple_quotes: false,
    keywords: NONE,
    statements: StatementStyle::None,
};
const SEMI: StatementStyle = StatementStyle::Braces { semicolon: true };
const NO_SEMI: StatementStyle = StatementStyle::Braces { semicolon: false };

const JAVA_KEYWORDS: &[&str] = &[
    "abstract",
    "assert",
    "boolean",
    "break",
    "byte",
    "case",
    "catch",
    "char",
    "class",
    "const",
    "continue",
    "default",
    "do",
    "double",
    "else",
    "enum",
    "extends",
    "false",
    "final",
    "finally",
    "float",
    "for",
    "goto",
    "if",
    "implements",
    "import",
    "instanceof",
    "int",
    "interface",
    "long",
    "native",
    "new",
    "null",
    "package",
    "private",
    "protected",
    "public",
    "record",
    "return",
    "sealed",
    "short",
    "static",
    "strictfp",
    "super",
    "switch",
    "synchronized",
    "this",
    "throw",
    "throws",
    "transient",
    "true",
    "try",
    "var",
    "void",
    "volatile",
    "while",
    "yield",
];

const RUST_KEYWORDS: &[&str] = &[
    "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum", "extern",
    "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub",
    "ref", "return", "self", "Self", "static", "struct", "super", "trait", "true", "type", "union",
    "unsafe", "use", "where", "while",
];

const KOTLIN_KEYWORDS: &[&str] = &[
    "abstract",
    "annotation",
    "as",
    "break",
    "by",
    "catch",
    "class",
    "companion",
    "const",
    "constructor",
    "continue",
    "crossinline",
    "data",
    "do",
    "else",
    "enum",
    "false",
    "final",
    "finally",
    "for",
    "fun",
    "get",
    "if",
    "import",
    "in",
    "infix",
    "init",
    "inline",
    "inner",
    "interface",
    "internal",
    "is",
    "lateinit",
    "noinline",
    "null",
    "object",
    "open",
    "operator",
    "out",
    "override",
    "package",
    "private",
    "protected",
    "public",
    "reified",
    "return",
    "sealed",
    "set",
    "super",
    "suspend",
    "this",
    "throw",
    "true",
    "try",
    "typealias",
    "val",
    "var",
    "vararg",
    "when",
    "where",
    "while",
];

const GROOVY_KEYWORDS: &[&str] = &[
    "abstract",
    "as",
    "assert",
    "boolean",
    "break",
    "case",
    "catch",
    "class",
    "continue",
    "def",
    "default",
    "do",
    "else",
    "enum",
    "extends",
    "false",
    "final",
    "finally",
    "for",
    "if",
    "implements",
    "import",
    "in",
    "instanceof",
    "interface",
    "new",
    "null",
    "package",
    "private",
    "protected",
    "public",
    "return",
    "static",
    "super",
    "switch",
    "this",
    "throw",
    "throws",
    "trait",
    "true",
    "try",
    "var",
    "void",
    "while",
];

const PYTHON_KEYWORDS: &[&str] = &[
    "False", "None", "True", "and", "as", "assert", "async", "await", "break", "class", "continue",
    "def", "del", "elif", "else", "except", "finally", "for", "from", "global", "if", "import",
    "in", "is", "lambda", "match", "nonlocal", "not", "or", "pass", "raise", "return", "try",
    "while", "with", "yield",
];

const GO_KEYWORDS: &[&str] = &[
    "break",
    "case",
    "chan",
    "const",
    "continue",
    "default",
    "defer",
    "else",
    "fallthrough",
    "for",
    "func",
    "go",
    "goto",
    "if",
    "import",
    "interface",
    "map",
    "package",
    "range",
    "return",
    "select",
    "struct",
    "switch",
    "type",
    "var",
    "true",
    "false",
    "nil",
];

const C_KEYWORDS: &[&str] = &[
    "auto", "break", "case", "char", "const", "continue", "default", "do", "double", "else",
    "enum", "extern", "float", "for", "goto", "if", "inline", "int", "long", "register",
    "restrict", "return", "short", "signed", "sizeof", "static", "struct", "switch", "typedef",
    "union", "unsigned", "void", "volatile", "while",
];

const CPP_KEYWORDS: &[&str] = &[
    "alignas",
    "alignof",
    "auto",
    "bool",
    "break",
    "case",
    "catch",
    "char",
    "class",
    "concept",
    "const",
    "consteval",
    "constexpr",
    "constinit",
    "continue",
    "co_await",
    "co_return",
    "co_yield",
    "decltype",
    "default",
    "delete",
    "do",
    "double",
    "else",
    "enum",
    "explicit",
    "export",
    "extern",
    "false",
    "final",
    "float",
    "for",
    "friend",
    "goto",
    "if",
    "inline",
    "int",
    "long",
    "mutable",
    "namespace",
    "new",
    "noexcept",
    "nullptr",
    "operator",
    "override",
    "private",
    "protected",
    "public",
    "requires",
    "return",
    "short",
    "signed",
    "sizeof",
    "static",
    "static_assert",
    "struct",
    "switch",
    "template",
    "this",
    "throw",
    "true",
    "try",
    "typedef",
    "typename",
    "union",
    "unsigned",
    "using",
    "virtual",
    "void",
    "volatile",
    "while",
];

const JS_KEYWORDS: &[&str] = &[
    "async",
    "await",
    "break",
    "case",
    "catch",
    "class",
    "const",
    "continue",
    "debugger",
    "default",
    "delete",
    "do",
    "else",
    "export",
    "extends",
    "false",
    "finally",
    "for",
    "from",
    "function",
    "if",
    "import",
    "in",
    "instanceof",
    "let",
    "new",
    "null",
    "of",
    "return",
    "static",
    "super",
    "switch",
    "this",
    "throw",
    "true",
    "try",
    "typeof",
    "undefined",
    "var",
    "void",
    "while",
    "with",
    "yield",
];

const TS_KEYWORDS: &[&str] = &[
    "abstract",
    "any",
    "as",
    "async",
    "await",
    "boolean",
    "break",
    "case",
    "catch",
    "class",
    "const",
    "continue",
    "declare",
    "default",
    "delete",
    "do",
    "else",
    "enum",
    "export",
    "extends",
    "false",
    "finally",
    "for",
    "from",
    "function",
    "implements",
    "import",
    "in",
    "infer",
    "instanceof",
    "interface",
    "is",
    "keyof",
    "let",
    "module",
    "namespace",
    "never",
    "new",
    "null",
    "number",
    "of",
    "private",
    "protected",
    "public",
    "readonly",
    "return",
    "satisfies",
    "static",
    "string",
    "super",
    "switch",
    "this",
    "throw",
    "true",
    "try",
    "type",
    "typeof",
    "undefined",
    "unknown",
    "var",
    "void",
    "while",
    "yield",
];

const BASH_KEYWORDS: &[&str] = &[
    "case", "do", "done", "elif", "else", "esac", "export", "fi", "for", "function", "if", "in",
    "local", "readonly", "return", "select", "then", "time", "unset", "until", "while",
];

const CSS_KEYWORDS: &[&str] = &[
    "!important",
    "@charset",
    "@container",
    "@font-face",
    "@import",
    "@keyframes",
    "@layer",
    "@media",
    "@supports",
    "inherit",
    "initial",
    "unset",
    "var",
    "calc",
];

const STARLARK_KEYWORDS: &[&str] = &[
    "and", "break", "continue", "def", "elif", "else", "for", "if", "in", "lambda", "load", "not",
    "or", "pass", "return", "None", "True", "False",
];

const DATA_KEYWORDS: &[&str] = &["true", "false", "null"];
const TOML_KEYWORDS: &[&str] = &["true", "false", "inf", "nan"];

/// Rows for every registered language (`markdown_inline` shares Markdown's).
pub static LANGUAGE_DATA: &[LanguageData] = &[
    LanguageData {
        id: "rust",
        statements: SEMI,
        line_comments: SLASH,
        block_comment: C_BLOCK,
        nested_block: true,
        doc_line_prefix: Some("///"),
        doc_block_open: Some("/**"),
        keywords: RUST_KEYWORDS,
        ..EMPTY
    },
    LanguageData {
        id: "java",
        statements: SEMI,
        line_comments: SLASH,
        block_comment: C_BLOCK,
        doc_block_open: Some("/**"),
        keywords: JAVA_KEYWORDS,
        triple_quotes: true,
        ..EMPTY
    },
    LanguageData {
        id: "kotlin",
        statements: NO_SEMI,
        line_comments: SLASH,
        block_comment: C_BLOCK,
        nested_block: true,
        doc_block_open: Some("/**"),
        keywords: KOTLIN_KEYWORDS,
        triple_quotes: true,
        ..EMPTY
    },
    LanguageData {
        id: "groovy",
        statements: NO_SEMI,
        line_comments: SLASH,
        block_comment: C_BLOCK,
        doc_block_open: Some("/**"),
        keywords: GROOVY_KEYWORDS,
        triple_quotes: true,
        ..EMPTY
    },
    LanguageData {
        id: "python",
        statements: StatementStyle::Indent,
        line_comments: HASH,
        keywords: PYTHON_KEYWORDS,
        triple_quotes: true,
        ..EMPTY
    },
    LanguageData {
        id: "go",
        statements: NO_SEMI,
        line_comments: SLASH,
        block_comment: C_BLOCK,
        quotes: WEB_QUOTES,
        keywords: GO_KEYWORDS,
        ..EMPTY
    },
    LanguageData {
        id: "c",
        statements: SEMI,
        line_comments: SLASH,
        block_comment: C_BLOCK,
        doc_block_open: Some("/**"),
        keywords: C_KEYWORDS,
        ..EMPTY
    },
    LanguageData {
        id: "cpp",
        statements: SEMI,
        line_comments: SLASH,
        block_comment: C_BLOCK,
        doc_line_prefix: Some("///"),
        doc_block_open: Some("/**"),
        keywords: CPP_KEYWORDS,
        ..EMPTY
    },
    LanguageData {
        id: "javascript",
        statements: SEMI,
        line_comments: SLASH,
        block_comment: C_BLOCK,
        doc_block_open: Some("/**"),
        quotes: WEB_QUOTES,
        keywords: JS_KEYWORDS,
        ..EMPTY
    },
    LanguageData {
        id: "typescript",
        statements: SEMI,
        line_comments: SLASH,
        block_comment: C_BLOCK,
        doc_block_open: Some("/**"),
        quotes: WEB_QUOTES,
        keywords: TS_KEYWORDS,
        ..EMPTY
    },
    LanguageData {
        id: "tsx",
        statements: SEMI,
        line_comments: SLASH,
        block_comment: C_BLOCK,
        doc_block_open: Some("/**"),
        quotes: WEB_QUOTES,
        keywords: TS_KEYWORDS,
        ..EMPTY
    },
    LanguageData {
        id: "bash",
        statements: StatementStyle::Shell,
        line_comments: HASH,
        keywords: BASH_KEYWORDS,
        ..EMPTY
    },
    LanguageData {
        id: "css",
        block_comment: C_BLOCK,
        keywords: CSS_KEYWORDS,
        ..EMPTY
    },
    LanguageData {
        id: "html",
        block_comment: HTML_BLOCK,
        keywords: NONE,
        ..EMPTY
    },
    LanguageData {
        id: "xml",
        block_comment: HTML_BLOCK,
        keywords: NONE,
        ..EMPTY
    },
    LanguageData {
        id: "json",
        line_comments: SLASH,
        block_comment: C_BLOCK,
        keywords: DATA_KEYWORDS,
        quotes: &['"'],
        ..EMPTY
    },
    LanguageData {
        id: "yaml",
        line_comments: HASH,
        keywords: DATA_KEYWORDS,
        ..EMPTY
    },
    LanguageData {
        id: "toml",
        line_comments: HASH,
        keywords: TOML_KEYWORDS,
        triple_quotes: true,
        ..EMPTY
    },
    LanguageData {
        id: "properties",
        line_comments: &["#", "!"],
        pairs: &[],
        quotes: &[],
        ..EMPTY
    },
    LanguageData {
        id: "markdown",
        block_comment: HTML_BLOCK,
        pairs: &[('(', ')'), ('[', ']')],
        quotes: &['`'],
        ..EMPTY
    },
    LanguageData {
        id: "markdown_inline",
        block_comment: HTML_BLOCK,
        pairs: &[('(', ')'), ('[', ']')],
        quotes: &['`'],
        ..EMPTY
    },
    LanguageData {
        id: "starlark",
        statements: StatementStyle::Indent,
        line_comments: HASH,
        keywords: STARLARK_KEYWORDS,
        triple_quotes: true,
        ..EMPTY
    },
];

/// Plain text: no comments, default pairs.
pub static PLAIN_TEXT: LanguageData = LanguageData {
    id: "plain",
    ..EMPTY
};

/// The data row of a language id (`None` for ids without a row).
pub fn language_data(id: &str) -> Option<&'static LanguageData> {
    LANGUAGE_DATA.iter().find(|row| row.id == id)
}

impl LanguageData {
    /// The prefix used to comment lines (`//`, `#`), if the language has line comments.
    pub fn line_prefix(&self) -> Option<&'static str> {
        self.line_comments.first().copied()
    }

    /// True when `ch` is an opening character of an auto-paired bracket pair.
    pub fn closing_bracket(&self, open: char) -> Option<char> {
        self.pairs
            .iter()
            .find_map(|&(o, c)| (o == open).then_some(c))
    }

    /// True when `ch` closes one of the language's bracket pairs.
    pub fn is_closing_bracket(&self, ch: char) -> bool {
        self.pairs.iter().any(|&(_, c)| c == ch)
    }

    pub fn is_quote(&self, ch: char) -> bool {
        self.quotes.contains(&ch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::LANGUAGES;

    #[test]
    fn every_language_has_a_data_row_and_ids_are_unique() {
        for spec in LANGUAGES {
            assert!(language_data(spec.id).is_some(), "{} has no row", spec.id);
        }
        let mut ids: Vec<_> = LANGUAGE_DATA.iter().map(|row| row.id).collect();
        ids.sort_unstable();
        let count = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), count, "duplicate rows");
    }

    #[test]
    fn block_comments_have_distinct_delimiters() {
        for row in LANGUAGE_DATA {
            if let Some((open, close)) = row.block_comment {
                assert_ne!(open, close, "{}", row.id);
            }
        }
    }
}
