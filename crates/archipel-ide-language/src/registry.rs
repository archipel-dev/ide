//! Language registry: every supported language is a grammar crate plus the
//! queries that crate ships. There is no per-language Rust code here, only data.

use std::path::Path;

pub use crate::def::LanguageDef;
use crate::def::{CompletenessTier, ExtraQueries, LanguageTables, no_services};
use crate::element::TokenSetSpec;

/// Migration alias: the registry entry type is now [`LanguageDef`].
pub type LanguageSpec = LanguageDef;

/// `include_str!` of a query file of a language directory under `queries/`.
macro_rules! q {
    ($dir:literal, $file:literal) => {
        include_str!(concat!("../queries/", $dir, "/", $file))
    };
}

const NO_OVERRIDES: &[(&str, &str)] = &[];
const XML_KEYS: &[(&str, &str)] = &[
    ("tag", "XML_TAG_NAME"),
    ("property", "XML_ATTRIBUTE_NAME"),
    ("punctuation.delimiter", "XML_TAG"),
    ("markup", "XML_TAG_DATA"),
];
const HTML_KEYS: &[(&str, &str)] = &[
    ("tag", "HTML_TAG_NAME"),
    ("attribute", "HTML_ATTRIBUTE_NAME"),
    ("punctuation.bracket", "HTML_TAG"),
];
const PROPERTIES_KEYS: &[(&str, &str)] = &[("property", "PROPERTIES.KEY")];

// Token sets by grammar kind name (`LanguageDef::token_sets`); a registry test
// checks every name exists in its grammar.
const PARENS: &[&str] = &["(", ")", "{", "}", "[", "]"];
const T_EMPTY: TokenSetSpec = TokenSetSpec::EMPTY;
const TOKENS_RUST: TokenSetSpec = TokenSetSpec {
    comments: &["line_comment", "block_comment"],
    string_literals: &["string_literal", "raw_string_literal", "char_literal"],
    identifiers: &[
        "identifier",
        "field_identifier",
        "type_identifier",
        "shorthand_field_identifier",
    ],
    brackets: PARENS,
    statement_like: &["expression_statement", "let_declaration"],
    declaration_like: &[
        "function_item",
        "struct_item",
        "enum_item",
        "trait_item",
        "impl_item",
        "mod_item",
        "const_item",
        "static_item",
        "type_item",
        "macro_definition",
    ],
    code_block_like: &["block", "declaration_list", "field_declaration_list"],
    ..T_EMPTY
};
const TOKENS_JAVA: TokenSetSpec = TokenSetSpec {
    comments: &["line_comment", "block_comment"],
    string_literals: &["string_literal", "character_literal"],
    identifiers: &["identifier", "type_identifier"],
    brackets: PARENS,
    statement_like: &[
        "expression_statement",
        "local_variable_declaration",
        "return_statement",
        "if_statement",
        "for_statement",
        "while_statement",
    ],
    declaration_like: &[
        "class_declaration",
        "interface_declaration",
        "enum_declaration",
        "record_declaration",
        "method_declaration",
        "constructor_declaration",
        "field_declaration",
    ],
    code_block_like: &[
        "block",
        "class_body",
        "interface_body",
        "enum_body",
        "constructor_body",
        "switch_block",
    ],
    ..T_EMPTY
};
const TOKENS_KOTLIN: TokenSetSpec = TokenSetSpec {
    comments: &["line_comment", "multiline_comment"],
    string_literals: &["string_literal"],
    identifiers: &["simple_identifier", "type_identifier"],
    brackets: PARENS,
    declaration_like: &[
        "class_declaration",
        "object_declaration",
        "function_declaration",
        "property_declaration",
    ],
    code_block_like: &["class_body", "function_body"],
    ..T_EMPTY
};
const TOKENS_GROOVY: TokenSetSpec = TokenSetSpec {
    comments: &["line_comment", "block_comment", "groovydoc_comment"],
    string_literals: &["string_literal"],
    identifiers: &["identifier", "type_identifier"],
    brackets: PARENS,
    code_block_like: &["block"],
    ..T_EMPTY
};
const TOKENS_XML: TokenSetSpec = TokenSetSpec::EMPTY;
const TOKENS_JSON: TokenSetSpec = TokenSetSpec {
    comments: &["comment"],
    string_literals: &["string"],
    brackets: &["{", "}", "[", "]"],
    declaration_like: &["pair"],
    code_block_like: &["object", "array"],
    ..T_EMPTY
};
const TOKENS_TOML: TokenSetSpec = TokenSetSpec {
    comments: &["comment"],
    string_literals: &["string"],
    identifiers: &["bare_key"],
    brackets: &["[", "]", "{", "}"],
    declaration_like: &["pair", "table"],
    ..T_EMPTY
};
const TOKENS_YAML: TokenSetSpec = TokenSetSpec {
    comments: &["comment"],
    string_literals: &[
        "string_scalar",
        "double_quote_scalar",
        "single_quote_scalar",
    ],
    declaration_like: &["block_mapping_pair"],
    ..T_EMPTY
};
const TOKENS_MARKDOWN: TokenSetSpec = TokenSetSpec {
    code_block_like: &["fenced_code_block", "indented_code_block"],
    ..T_EMPTY
};
const TOKENS_MARKDOWN_INLINE: TokenSetSpec = TokenSetSpec::EMPTY;
const TOKENS_STARLARK: TokenSetSpec = TokenSetSpec {
    comments: &["comment"],
    string_literals: &["string"],
    identifiers: &["identifier"],
    brackets: &["(", ")", "{", "}", "[", "]"],
    declaration_like: &["function_definition"],
    code_block_like: &["block"],
    ..T_EMPTY
};
const TOKENS_PYTHON: TokenSetSpec = TokenSetSpec {
    comments: &["comment"],
    string_literals: &["string"],
    identifiers: &["identifier"],
    brackets: PARENS,
    statement_like: &["expression_statement", "return_statement", "if_statement"],
    declaration_like: &["function_definition", "class_definition"],
    code_block_like: &["block"],
    ..T_EMPTY
};
const TOKENS_JAVASCRIPT: TokenSetSpec = TokenSetSpec {
    comments: &["comment"],
    string_literals: &["string", "template_string"],
    identifiers: &[
        "identifier",
        "property_identifier",
        "shorthand_property_identifier",
    ],
    brackets: PARENS,
    statement_like: &[
        "expression_statement",
        "lexical_declaration",
        "return_statement",
        "if_statement",
    ],
    declaration_like: &[
        "function_declaration",
        "class_declaration",
        "method_definition",
    ],
    code_block_like: &["statement_block", "class_body"],
    ..T_EMPTY
};
const TOKENS_TYPESCRIPT: TokenSetSpec = TokenSetSpec {
    identifiers: &[
        "identifier",
        "property_identifier",
        "shorthand_property_identifier",
        "type_identifier",
    ],
    declaration_like: &[
        "function_declaration",
        "class_declaration",
        "method_definition",
        "interface_declaration",
        "type_alias_declaration",
        "enum_declaration",
    ],
    ..TOKENS_JAVASCRIPT
};
const TOKENS_TSX: TokenSetSpec = TOKENS_TYPESCRIPT;
const TOKENS_GO: TokenSetSpec = TokenSetSpec {
    comments: &["comment"],
    string_literals: &["interpreted_string_literal", "raw_string_literal"],
    identifiers: &[
        "identifier",
        "field_identifier",
        "type_identifier",
        "package_identifier",
    ],
    brackets: PARENS,
    declaration_like: &[
        "function_declaration",
        "method_declaration",
        "type_declaration",
    ],
    code_block_like: &["block"],
    ..T_EMPTY
};
const TOKENS_C: TokenSetSpec = TokenSetSpec {
    comments: &["comment"],
    string_literals: &["string_literal", "char_literal"],
    identifiers: &["identifier", "field_identifier", "type_identifier"],
    brackets: PARENS,
    declaration_like: &["function_definition", "struct_specifier", "enum_specifier"],
    code_block_like: &["compound_statement", "field_declaration_list"],
    ..T_EMPTY
};
const TOKENS_CPP: TokenSetSpec = TokenSetSpec {
    string_literals: &["string_literal", "char_literal", "raw_string_literal"],
    identifiers: &[
        "identifier",
        "field_identifier",
        "type_identifier",
        "namespace_identifier",
    ],
    declaration_like: &[
        "function_definition",
        "struct_specifier",
        "enum_specifier",
        "class_specifier",
        "namespace_definition",
    ],
    ..TOKENS_C
};
const TOKENS_BASH: TokenSetSpec = TokenSetSpec {
    comments: &["comment"],
    string_literals: &["string", "raw_string", "ansi_c_string"],
    declaration_like: &["function_definition"],
    code_block_like: &["compound_statement"],
    ..T_EMPTY
};
const TOKENS_HTML: TokenSetSpec = TokenSetSpec {
    comments: &["comment"],
    declaration_like: &["element"],
    ..T_EMPTY
};
const TOKENS_CSS: TokenSetSpec = TokenSetSpec {
    comments: &["comment"],
    string_literals: &["string_value"],
    brackets: &["(", ")", "{", "}"],
    declaration_like: &["rule_set"],
    code_block_like: &["block"],
    ..T_EMPTY
};
const TOKENS_PROPERTIES: TokenSetSpec = TokenSetSpec {
    comments: &["comment"],
    declaration_like: &["property"],
    ..T_EMPTY
};

/// All registered languages, in detection priority order.
pub static LANGUAGES: &[LanguageDef] = &LANGUAGE_TABLE;

static LANGUAGE_TABLE: [LanguageDef; 22] = [
    LanguageDef {
        id: "rust",
        name: "Rust",
        extensions: &["rs"],
        file_names: &[],
        shebangs: &[],
        grammar: tree_sitter_rust::LANGUAGE,
        highlights: &[tree_sitter_rust::HIGHLIGHTS_QUERY],
        injections: Some(tree_sitter_rust::INJECTIONS_QUERY),
        locals: Some(q!("rust", "locals.scm")),
        folds: Some(q!("rust", "folds.scm")),
        tags: &[q!("rust", "tags.scm")],
        capture_keys: NO_OVERRIDES,
        base: None,
        tier: CompletenessTier::G0,
        token_sets: TOKENS_RUST,
        extra_queries: ExtraQueries {
            indents: Some(q!("rust", "indents.scm")),
            brackets: Some(q!("rust", "brackets.scm")),
            textobjects: Some(q!("rust", "textobjects.scm")),
            sticky: Some(q!("rust", "sticky.scm")),
            doc_comments: Some(q!("rust", "doc.scm")),
            ..ExtraQueries::NONE
        },
        services: no_services,
        tables: LanguageTables::new(),
    },
    LanguageDef {
        id: "java",
        name: "Java",
        extensions: &["java"],
        file_names: &[],
        shebangs: &[],
        grammar: tree_sitter_java::LANGUAGE,
        highlights: &[tree_sitter_java::HIGHLIGHTS_QUERY],
        injections: None,
        locals: Some(q!("java", "locals.scm")),
        folds: Some(q!("java", "folds.scm")),
        tags: &[q!("java", "tags.scm")],
        capture_keys: NO_OVERRIDES,
        base: None,
        tier: CompletenessTier::G0,
        token_sets: TOKENS_JAVA,
        extra_queries: ExtraQueries {
            indents: Some(q!("java", "indents.scm")),
            brackets: Some(q!("java", "brackets.scm")),
            textobjects: Some(q!("java", "textobjects.scm")),
            sticky: Some(q!("java", "sticky.scm")),
            doc_comments: Some(q!("java", "doc.scm")),
            ..ExtraQueries::NONE
        },
        services: no_services,
        tables: LanguageTables::new(),
    },
    LanguageDef {
        id: "kotlin",
        name: "Kotlin",
        extensions: &["kt", "kts"],
        file_names: &[],
        shebangs: &[],
        grammar: tree_sitter_kotlin_sg::LANGUAGE,
        highlights: &[
            tree_sitter_kotlin_sg::HIGHLIGHTS_QUERY,
            include_str!("../queries/kotlin/highlights.scm"),
        ],
        injections: None,
        locals: Some(q!("kotlin", "locals.scm")),
        folds: Some(q!("kotlin", "folds.scm")),
        tags: &[q!("kotlin", "tags.scm")],
        capture_keys: NO_OVERRIDES,
        base: None,
        tier: CompletenessTier::G0,
        token_sets: TOKENS_KOTLIN,
        extra_queries: ExtraQueries {
            indents: Some(q!("kotlin", "indents.scm")),
            brackets: Some(q!("kotlin", "brackets.scm")),
            textobjects: Some(q!("kotlin", "textobjects.scm")),
            sticky: Some(q!("kotlin", "sticky.scm")),
            doc_comments: Some(q!("kotlin", "doc.scm")),
            ..ExtraQueries::NONE
        },
        services: no_services,
        tables: LanguageTables::new(),
    },
    LanguageDef {
        id: "groovy",
        name: "Groovy",
        extensions: &["groovy", "gradle", "gvy", "gy", "gsh"],
        file_names: &["Jenkinsfile"],
        shebangs: &["groovy"],
        grammar: tree_sitter_groovy::LANGUAGE,
        highlights: &[tree_sitter_groovy::HIGHLIGHTS_QUERY],
        injections: None,
        locals: Some(q!("groovy", "locals.scm")),
        folds: Some(q!("groovy", "folds.scm")),
        tags: &[q!("groovy", "tags.scm")],
        capture_keys: NO_OVERRIDES,
        base: None,
        tier: CompletenessTier::G0,
        token_sets: TOKENS_GROOVY,
        extra_queries: ExtraQueries {
            indents: Some(q!("groovy", "indents.scm")),
            brackets: Some(q!("groovy", "brackets.scm")),
            textobjects: Some(q!("groovy", "textobjects.scm")),
            sticky: Some(q!("groovy", "sticky.scm")),
            doc_comments: Some(q!("groovy", "doc.scm")),
            ..ExtraQueries::NONE
        },
        services: no_services,
        tables: LanguageTables::new(),
    },
    LanguageDef {
        id: "xml",
        name: "XML",
        extensions: &["xml", "xsd", "xsl", "xslt", "svg", "plist", "iml"],
        file_names: &[],
        shebangs: &[],
        grammar: tree_sitter_xml::LANGUAGE_XML,
        highlights: &[tree_sitter_xml::XML_HIGHLIGHT_QUERY],
        injections: None,
        locals: Some(q!("xml", "locals.scm")),
        folds: Some(q!("xml", "folds.scm")),
        tags: &[q!("xml", "tags.scm")],
        capture_keys: XML_KEYS,
        base: None,
        tier: CompletenessTier::G0,
        token_sets: TOKENS_XML,
        extra_queries: ExtraQueries {
            indents: Some(q!("xml", "indents.scm")),
            brackets: Some(q!("xml", "brackets.scm")),
            textobjects: Some(q!("xml", "textobjects.scm")),
            sticky: Some(q!("xml", "sticky.scm")),
            doc_comments: Some(q!("xml", "doc.scm")),
            ..ExtraQueries::NONE
        },
        services: no_services,
        tables: LanguageTables::new(),
    },
    LanguageDef {
        id: "json",
        name: "JSON",
        extensions: &["json", "jsonc", "json5"],
        file_names: &[".babelrc", ".eslintrc"],
        shebangs: &[],
        grammar: tree_sitter_json::LANGUAGE,
        highlights: &[tree_sitter_json::HIGHLIGHTS_QUERY],
        injections: None,
        locals: Some(q!("json", "locals.scm")),
        folds: Some(q!("json", "folds.scm")),
        tags: &[q!("json", "tags.scm")],
        capture_keys: NO_OVERRIDES,
        base: None,
        tier: CompletenessTier::G0,
        token_sets: TOKENS_JSON,
        extra_queries: ExtraQueries {
            indents: Some(q!("json", "indents.scm")),
            brackets: Some(q!("json", "brackets.scm")),
            textobjects: Some(q!("json", "textobjects.scm")),
            sticky: Some(q!("json", "sticky.scm")),
            doc_comments: Some(q!("json", "doc.scm")),
            ..ExtraQueries::NONE
        },
        services: no_services,
        tables: LanguageTables::new(),
    },
    LanguageDef {
        id: "toml",
        name: "TOML",
        extensions: &["toml"],
        file_names: &["Cargo.lock"],
        shebangs: &[],
        grammar: tree_sitter_toml_ng::LANGUAGE,
        highlights: &[tree_sitter_toml_ng::HIGHLIGHTS_QUERY],
        injections: None,
        locals: Some(q!("toml", "locals.scm")),
        folds: Some(q!("toml", "folds.scm")),
        tags: &[q!("toml", "tags.scm")],
        capture_keys: NO_OVERRIDES,
        base: None,
        tier: CompletenessTier::G0,
        token_sets: TOKENS_TOML,
        extra_queries: ExtraQueries {
            indents: Some(q!("toml", "indents.scm")),
            brackets: Some(q!("toml", "brackets.scm")),
            textobjects: Some(q!("toml", "textobjects.scm")),
            sticky: Some(q!("toml", "sticky.scm")),
            doc_comments: Some(q!("toml", "doc.scm")),
            ..ExtraQueries::NONE
        },
        services: no_services,
        tables: LanguageTables::new(),
    },
    LanguageDef {
        id: "yaml",
        name: "YAML",
        extensions: &["yaml", "yml"],
        file_names: &[],
        shebangs: &[],
        grammar: tree_sitter_yaml::LANGUAGE,
        highlights: &[tree_sitter_yaml::HIGHLIGHTS_QUERY],
        injections: None,
        locals: Some(q!("yaml", "locals.scm")),
        folds: Some(q!("yaml", "folds.scm")),
        tags: &[q!("yaml", "tags.scm")],
        capture_keys: NO_OVERRIDES,
        base: None,
        tier: CompletenessTier::G0,
        token_sets: TOKENS_YAML,
        extra_queries: ExtraQueries {
            indents: Some(q!("yaml", "indents.scm")),
            brackets: Some(q!("yaml", "brackets.scm")),
            textobjects: Some(q!("yaml", "textobjects.scm")),
            sticky: Some(q!("yaml", "sticky.scm")),
            doc_comments: Some(q!("yaml", "doc.scm")),
            ..ExtraQueries::NONE
        },
        services: no_services,
        tables: LanguageTables::new(),
    },
    LanguageDef {
        id: "markdown",
        name: "Markdown",
        extensions: &["md", "markdown"],
        file_names: &[],
        shebangs: &[],
        grammar: tree_sitter_md::LANGUAGE,
        highlights: &[tree_sitter_md::HIGHLIGHT_QUERY_BLOCK],
        injections: Some(tree_sitter_md::INJECTION_QUERY_BLOCK),
        locals: Some(q!("markdown", "locals.scm")),
        folds: Some(q!("markdown", "folds.scm")),
        tags: &[q!("markdown", "tags.scm")],
        capture_keys: NO_OVERRIDES,
        base: None,
        tier: CompletenessTier::G0,
        token_sets: TOKENS_MARKDOWN,
        extra_queries: ExtraQueries {
            brackets: Some(q!("markdown", "brackets.scm")),
            textobjects: Some(q!("markdown", "textobjects.scm")),
            sticky: Some(q!("markdown", "sticky.scm")),
            doc_comments: Some(q!("markdown", "doc.scm")),
            ..ExtraQueries::NONE
        },
        services: no_services,
        tables: LanguageTables::new(),
    },
    LanguageDef {
        // Only reached through Markdown's `(inline) @injection.content`.
        id: "markdown_inline",
        name: "Markdown",
        extensions: &[],
        file_names: &[],
        shebangs: &[],
        grammar: tree_sitter_md::INLINE_LANGUAGE,
        highlights: &[tree_sitter_md::HIGHLIGHT_QUERY_INLINE],
        injections: Some(tree_sitter_md::INJECTION_QUERY_INLINE),
        locals: None,
        folds: None,
        tags: &[],
        capture_keys: NO_OVERRIDES,
        base: None,
        tier: CompletenessTier::G0,
        token_sets: TOKENS_MARKDOWN_INLINE,
        extra_queries: ExtraQueries::NONE,
        services: no_services,
        tables: LanguageTables::new(),
    },
    LanguageDef {
        id: "starlark",
        name: "Starlark",
        extensions: &["bzl", "bazel", "star", "sky"],
        file_names: &[
            "BUILD",
            "BUILD.bazel",
            "WORKSPACE",
            "WORKSPACE.bazel",
            "WORKSPACE.bzlmod",
            "MODULE.bazel",
            "Tiltfile",
        ],
        shebangs: &[],
        grammar: tree_sitter_starlark::LANGUAGE,
        highlights: &[tree_sitter_starlark::HIGHLIGHTS_QUERY],
        injections: Some(tree_sitter_starlark::INJECTIONS_QUERY),
        locals: Some(q!("starlark", "locals.scm")),
        folds: Some(q!("starlark", "folds.scm")),
        tags: &[q!("starlark", "tags.scm")],
        capture_keys: NO_OVERRIDES,
        base: None,
        tier: CompletenessTier::G0,
        token_sets: TOKENS_STARLARK,
        extra_queries: ExtraQueries {
            indents: Some(q!("starlark", "indents.scm")),
            brackets: Some(q!("starlark", "brackets.scm")),
            textobjects: Some(q!("starlark", "textobjects.scm")),
            sticky: Some(q!("starlark", "sticky.scm")),
            doc_comments: Some(q!("starlark", "doc.scm")),
            ..ExtraQueries::NONE
        },
        services: no_services,
        tables: LanguageTables::new(),
    },
    LanguageDef {
        id: "python",
        name: "Python",
        extensions: &["py", "pyi", "pyw"],
        file_names: &[],
        shebangs: &["python", "python3", "python2"],
        grammar: tree_sitter_python::LANGUAGE,
        highlights: &[tree_sitter_python::HIGHLIGHTS_QUERY],
        injections: None,
        locals: Some(q!("python", "locals.scm")),
        folds: Some(q!("python", "folds.scm")),
        tags: &[q!("python", "tags.scm")],
        capture_keys: NO_OVERRIDES,
        base: None,
        tier: CompletenessTier::G0,
        token_sets: TOKENS_PYTHON,
        extra_queries: ExtraQueries {
            indents: Some(q!("python", "indents.scm")),
            brackets: Some(q!("python", "brackets.scm")),
            textobjects: Some(q!("python", "textobjects.scm")),
            sticky: Some(q!("python", "sticky.scm")),
            doc_comments: Some(q!("python", "doc.scm")),
            ..ExtraQueries::NONE
        },
        services: no_services,
        tables: LanguageTables::new(),
    },
    LanguageDef {
        id: "javascript",
        name: "JavaScript",
        extensions: &["js", "mjs", "cjs", "jsx"],
        file_names: &[],
        shebangs: &["node", "nodejs", "deno", "bun"],
        grammar: tree_sitter_javascript::LANGUAGE,
        highlights: &[
            tree_sitter_javascript::JSX_HIGHLIGHT_QUERY,
            tree_sitter_javascript::HIGHLIGHT_QUERY,
        ],
        injections: Some(tree_sitter_javascript::INJECTIONS_QUERY),
        locals: Some(q!("javascript", "locals.scm")),
        folds: Some(q!("javascript", "folds.scm")),
        tags: &[q!("javascript", "tags.scm")],
        capture_keys: HTML_KEYS,
        base: None,
        tier: CompletenessTier::G0,
        token_sets: TOKENS_JAVASCRIPT,
        extra_queries: ExtraQueries {
            indents: Some(q!("javascript", "indents.scm")),
            brackets: Some(q!("javascript", "brackets.scm")),
            textobjects: Some(q!("javascript", "textobjects.scm")),
            sticky: Some(q!("javascript", "sticky.scm")),
            doc_comments: Some(q!("javascript", "doc.scm")),
            ..ExtraQueries::NONE
        },
        services: no_services,
        tables: LanguageTables::new(),
    },
    LanguageDef {
        id: "typescript",
        name: "TypeScript",
        extensions: &["ts", "mts", "cts"],
        file_names: &[],
        shebangs: &["ts-node"],
        grammar: tree_sitter_typescript::LANGUAGE_TYPESCRIPT,
        highlights: &[
            tree_sitter_typescript::HIGHLIGHTS_QUERY,
            tree_sitter_javascript::HIGHLIGHT_QUERY,
        ],
        injections: Some(tree_sitter_javascript::INJECTIONS_QUERY),
        locals: Some(q!("typescript", "locals.scm")),
        folds: Some(q!("typescript", "folds.scm")),
        tags: &[q!("typescript", "tags.scm")],
        capture_keys: NO_OVERRIDES,
        base: Some("javascript"),
        tier: CompletenessTier::G0,
        token_sets: TOKENS_TYPESCRIPT,
        extra_queries: ExtraQueries {
            indents: Some(q!("typescript", "indents.scm")),
            brackets: Some(q!("typescript", "brackets.scm")),
            textobjects: Some(q!("typescript", "textobjects.scm")),
            sticky: Some(q!("typescript", "sticky.scm")),
            doc_comments: Some(q!("typescript", "doc.scm")),
            ..ExtraQueries::NONE
        },
        services: no_services,
        tables: LanguageTables::new(),
    },
    LanguageDef {
        id: "tsx",
        name: "TypeScript JSX",
        extensions: &["tsx"],
        file_names: &[],
        shebangs: &[],
        grammar: tree_sitter_typescript::LANGUAGE_TSX,
        highlights: &[
            tree_sitter_javascript::JSX_HIGHLIGHT_QUERY,
            tree_sitter_typescript::HIGHLIGHTS_QUERY,
            tree_sitter_javascript::HIGHLIGHT_QUERY,
        ],
        injections: Some(tree_sitter_javascript::INJECTIONS_QUERY),
        locals: Some(q!("typescript", "locals.scm")),
        folds: Some(q!("typescript", "folds.scm")),
        tags: &[q!("typescript", "tags.scm")],
        capture_keys: HTML_KEYS,
        base: Some("typescript"),
        tier: CompletenessTier::G0,
        token_sets: TOKENS_TSX,
        extra_queries: ExtraQueries {
            indents: Some(q!("typescript", "indents.scm")),
            brackets: Some(q!("typescript", "brackets.scm")),
            textobjects: Some(q!("typescript", "textobjects.scm")),
            sticky: Some(q!("typescript", "sticky.scm")),
            doc_comments: Some(q!("typescript", "doc.scm")),
            ..ExtraQueries::NONE
        },
        services: no_services,
        tables: LanguageTables::new(),
    },
    LanguageDef {
        id: "go",
        name: "Go",
        extensions: &["go"],
        file_names: &[],
        shebangs: &[],
        grammar: tree_sitter_go::LANGUAGE,
        highlights: &[tree_sitter_go::HIGHLIGHTS_QUERY],
        injections: None,
        locals: Some(q!("go", "locals.scm")),
        folds: Some(q!("go", "folds.scm")),
        tags: &[q!("go", "tags.scm")],
        capture_keys: NO_OVERRIDES,
        base: None,
        tier: CompletenessTier::G0,
        token_sets: TOKENS_GO,
        extra_queries: ExtraQueries {
            indents: Some(q!("go", "indents.scm")),
            brackets: Some(q!("go", "brackets.scm")),
            textobjects: Some(q!("go", "textobjects.scm")),
            sticky: Some(q!("go", "sticky.scm")),
            doc_comments: Some(q!("go", "doc.scm")),
            ..ExtraQueries::NONE
        },
        services: no_services,
        tables: LanguageTables::new(),
    },
    LanguageDef {
        id: "c",
        name: "C",
        extensions: &["c", "h"],
        file_names: &[],
        shebangs: &[],
        grammar: tree_sitter_c::LANGUAGE,
        highlights: &[tree_sitter_c::HIGHLIGHT_QUERY],
        injections: None,
        locals: Some(q!("c", "locals.scm")),
        folds: Some(q!("c", "folds.scm")),
        tags: &[q!("c", "tags.scm")],
        capture_keys: NO_OVERRIDES,
        base: None,
        tier: CompletenessTier::G0,
        token_sets: TOKENS_C,
        extra_queries: ExtraQueries {
            indents: Some(q!("c", "indents.scm")),
            brackets: Some(q!("c", "brackets.scm")),
            textobjects: Some(q!("c", "textobjects.scm")),
            sticky: Some(q!("c", "sticky.scm")),
            doc_comments: Some(q!("c", "doc.scm")),
            ..ExtraQueries::NONE
        },
        services: no_services,
        tables: LanguageTables::new(),
    },
    LanguageDef {
        id: "cpp",
        name: "C++",
        extensions: &[
            "cc", "cpp", "cxx", "c++", "hh", "hpp", "hxx", "h++", "ipp", "inl",
        ],
        file_names: &[],
        shebangs: &[],
        grammar: tree_sitter_cpp::LANGUAGE,
        highlights: &[
            tree_sitter_c::HIGHLIGHT_QUERY,
            tree_sitter_cpp::HIGHLIGHT_QUERY,
        ],
        injections: None,
        locals: Some(q!("cpp", "locals.scm")),
        folds: Some(q!("cpp", "folds.scm")),
        tags: &[q!("cpp", "tags.scm")],
        capture_keys: NO_OVERRIDES,
        base: None,
        tier: CompletenessTier::G0,
        token_sets: TOKENS_CPP,
        extra_queries: ExtraQueries {
            indents: Some(q!("cpp", "indents.scm")),
            brackets: Some(q!("cpp", "brackets.scm")),
            textobjects: Some(q!("cpp", "textobjects.scm")),
            sticky: Some(q!("cpp", "sticky.scm")),
            doc_comments: Some(q!("cpp", "doc.scm")),
            ..ExtraQueries::NONE
        },
        services: no_services,
        tables: LanguageTables::new(),
    },
    LanguageDef {
        id: "bash",
        name: "Shell Script",
        extensions: &["sh", "bash", "zsh", "ksh"],
        file_names: &[
            ".bashrc",
            ".bash_profile",
            ".profile",
            ".zshrc",
            "gradlew",
            "PKGBUILD",
        ],
        shebangs: &["sh", "bash", "zsh", "ksh", "dash"],
        grammar: tree_sitter_bash::LANGUAGE,
        highlights: &[tree_sitter_bash::HIGHLIGHT_QUERY],
        injections: None,
        locals: Some(q!("bash", "locals.scm")),
        folds: Some(q!("bash", "folds.scm")),
        tags: &[q!("bash", "tags.scm")],
        capture_keys: NO_OVERRIDES,
        base: None,
        tier: CompletenessTier::G0,
        token_sets: TOKENS_BASH,
        extra_queries: ExtraQueries {
            indents: Some(q!("bash", "indents.scm")),
            brackets: Some(q!("bash", "brackets.scm")),
            textobjects: Some(q!("bash", "textobjects.scm")),
            sticky: Some(q!("bash", "sticky.scm")),
            doc_comments: Some(q!("bash", "doc.scm")),
            ..ExtraQueries::NONE
        },
        services: no_services,
        tables: LanguageTables::new(),
    },
    LanguageDef {
        id: "html",
        name: "HTML",
        extensions: &["html", "htm", "xhtml"],
        file_names: &[],
        shebangs: &[],
        grammar: tree_sitter_html::LANGUAGE,
        highlights: &[tree_sitter_html::HIGHLIGHTS_QUERY],
        injections: Some(tree_sitter_html::INJECTIONS_QUERY),
        locals: Some(q!("html", "locals.scm")),
        folds: Some(q!("html", "folds.scm")),
        tags: &[q!("html", "tags.scm")],
        capture_keys: HTML_KEYS,
        base: None,
        tier: CompletenessTier::G0,
        token_sets: TOKENS_HTML,
        extra_queries: ExtraQueries {
            indents: Some(q!("html", "indents.scm")),
            brackets: Some(q!("html", "brackets.scm")),
            textobjects: Some(q!("html", "textobjects.scm")),
            sticky: Some(q!("html", "sticky.scm")),
            doc_comments: Some(q!("html", "doc.scm")),
            ..ExtraQueries::NONE
        },
        services: no_services,
        tables: LanguageTables::new(),
    },
    LanguageDef {
        id: "css",
        name: "CSS",
        extensions: &["css"],
        file_names: &[],
        shebangs: &[],
        grammar: tree_sitter_css::LANGUAGE,
        highlights: &[tree_sitter_css::HIGHLIGHTS_QUERY],
        injections: None,
        locals: Some(q!("css", "locals.scm")),
        folds: Some(q!("css", "folds.scm")),
        tags: &[q!("css", "tags.scm")],
        capture_keys: NO_OVERRIDES,
        base: None,
        tier: CompletenessTier::G0,
        token_sets: TOKENS_CSS,
        extra_queries: ExtraQueries {
            indents: Some(q!("css", "indents.scm")),
            brackets: Some(q!("css", "brackets.scm")),
            textobjects: Some(q!("css", "textobjects.scm")),
            sticky: Some(q!("css", "sticky.scm")),
            doc_comments: Some(q!("css", "doc.scm")),
            ..ExtraQueries::NONE
        },
        services: no_services,
        tables: LanguageTables::new(),
    },
    LanguageDef {
        id: "properties",
        name: "Properties",
        extensions: &["properties"],
        file_names: &[],
        shebangs: &[],
        grammar: tree_sitter_properties::LANGUAGE,
        highlights: &[tree_sitter_properties::HIGHLIGHTS_QUERY],
        injections: None,
        locals: Some(q!("properties", "locals.scm")),
        folds: Some(q!("properties", "folds.scm")),
        tags: &[tree_sitter_properties::TAGS_QUERY],
        capture_keys: PROPERTIES_KEYS,
        base: None,
        tier: CompletenessTier::G0,
        token_sets: TOKENS_PROPERTIES,
        extra_queries: ExtraQueries {
            brackets: Some(q!("properties", "brackets.scm")),
            textobjects: Some(q!("properties", "textobjects.scm")),
            sticky: Some(q!("properties", "sticky.scm")),
            doc_comments: Some(q!("properties", "doc.scm")),
            ..ExtraQueries::NONE
        },
        services: no_services,
        tables: LanguageTables::new(),
    },
];

/// Looks a language up by its registry id (or a common alias, as used by
/// Markdown code fences and other injections).
pub fn by_id(id: &str) -> Option<&'static LanguageSpec> {
    let id = id.trim().to_ascii_lowercase();
    let id = match id.as_str() {
        "rs" => "rust",
        "kt" | "kts" | "gradle.kts" => "kotlin",
        "js" | "jsx" | "node" => "javascript",
        "ts" => "typescript",
        "py" | "python3" => "python",
        "sh" | "shell" | "zsh" | "console" => "bash",
        "c++" | "cc" | "cxx" | "hpp" => "cpp",
        "yml" => "yaml",
        "md" => "markdown",
        "bzl" | "bazel" | "starlark" | "skylark" => "starlark",
        "golang" => "go",
        "htm" | "xhtml" => "html",
        "jsonc" | "json5" => "json",
        other => other,
    };
    LANGUAGES.iter().find(|spec| spec.id == id)
}

/// Detects the language of a file from its name, extension or shebang line.
pub fn detect(path: &Path, source: &str) -> Option<&'static LanguageSpec> {
    let file_name = path.file_name().and_then(|name| name.to_str());
    if let Some(file_name) = file_name
        && let Some(spec) = LANGUAGES
            .iter()
            .find(|spec| spec.file_names.contains(&file_name))
    {
        return Some(spec);
    }
    if let Some(extension) = path.extension().and_then(|value| value.to_str()) {
        let extension = extension.to_ascii_lowercase();
        if let Some(spec) = LANGUAGES
            .iter()
            .find(|spec| spec.extensions.contains(&extension.as_str()))
        {
            return Some(spec);
        }
    }
    detect_shebang(source)
}

fn detect_shebang(source: &str) -> Option<&'static LanguageSpec> {
    let line = source.lines().next()?.strip_prefix("#!")?;
    let mut words = line.split_whitespace();
    let program = words.next()?;
    let mut interpreter = program.rsplit('/').next()?;
    if interpreter == "env" {
        interpreter = words.find(|word| !word.starts_with('-'))?;
    }
    let interpreter = interpreter.trim_end_matches(|c: char| c.is_ascii_digit() || c == '.');
    LANGUAGES.iter().find(|spec| {
        spec.shebangs
            .iter()
            .any(|name| name.trim_end_matches(|c: char| c.is_ascii_digit()) == interpreter)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(path: &str, source: &str) -> Option<&'static str> {
        detect(Path::new(path), source).map(|spec| spec.id)
    }

    #[test]
    fn detects_by_extension_file_name_and_shebang() {
        assert_eq!(id("src/App.java", ""), Some("java"));
        assert_eq!(id("app/build.gradle.kts", ""), Some("kotlin"));
        assert_eq!(id("res/layout/main.XML", ""), Some("xml"));
        assert_eq!(id("pkg/BUILD.bazel", ""), Some("starlark"));
        assert_eq!(id("BUILD", ""), Some("starlark"));
        assert_eq!(id("MODULE.bazel", ""), Some("starlark"));
        assert_eq!(id("WORKSPACE", ""), Some("starlark"));
        assert_eq!(id("defs.bzl", ""), Some("starlark"));
        assert_eq!(id("Cargo.lock", ""), Some("toml"));
        assert_eq!(id("gradlew", ""), Some("bash"));
        assert_eq!(id("app/build.gradle", ""), Some("groovy"));
        assert_eq!(id("Jenkinsfile", ""), Some("groovy"));
        assert_eq!(
            id("tool", "#!/usr/bin/env python3\nprint(1)\n"),
            Some("python")
        );
        assert_eq!(id("run", "#!/bin/bash\necho hi\n"), Some("bash"));
        assert_eq!(
            id("serve", "#!/usr/bin/env -S node --flag\n"),
            Some("javascript")
        );
        assert_eq!(id("notes.txt", "hello"), None);
        assert_eq!(id("noext", ""), None);
    }

    #[test]
    fn ids_and_aliases_resolve() {
        assert_eq!(by_id("kt").map(|spec| spec.id), Some("kotlin"));
        assert_eq!(by_id("Rust").map(|spec| spec.id), Some("rust"));
        assert_eq!(by_id("c++").map(|spec| spec.id), Some("cpp"));
        assert!(by_id("cobol").is_none());
    }

    #[test]
    fn token_set_names_exist_in_their_grammars() {
        for spec in LANGUAGES {
            assert!(
                spec.unknown_token_names().is_empty(),
                "{}: unknown kinds {:?}",
                spec.id,
                spec.unknown_token_names()
            );
            if let Some(base) = spec.base {
                assert!(by_id(base).is_some(), "{}: unknown base {base}", spec.id);
            }
        }
        let rust = by_id("rust").expect("rust");
        let table = rust.element_types();
        let comment = table.lookup("line_comment").next().expect("kind");
        assert!(rust.token_set_table().comments.contains(comment));
    }

    #[test]
    fn registry_ids_and_extensions_are_unique() {
        let mut ids = std::collections::HashSet::new();
        let mut extensions = std::collections::HashSet::new();
        for spec in LANGUAGES {
            assert!(ids.insert(spec.id), "duplicate id {}", spec.id);
            for extension in spec.extensions {
                assert!(extensions.insert(*extension), "duplicate .{extension}");
            }
        }
    }
}
