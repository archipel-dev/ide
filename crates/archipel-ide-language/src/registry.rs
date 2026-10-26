//! Language registry: every supported language is a grammar crate plus the
//! queries that crate ships. There is no per-language Rust code here, only data.

use std::path::Path;

pub use crate::def::LanguageDef;
use crate::def::{CompletenessTier, ExtraQueries, LanguageTables, no_services};
use crate::element::TokenSetSpec;

/// Migration alias: the registry entry type is now [`LanguageDef`].
pub type LanguageSpec = LanguageDef;

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
        locals: None,
        folds: None,
        tags: &[tree_sitter_rust::TAGS_QUERY],
        capture_keys: NO_OVERRIDES,
        base: None,
        tier: CompletenessTier::G0,
        token_sets: TOKENS_RUST,
        extra_queries: ExtraQueries::NONE,
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
        locals: None,
        folds: None,
        tags: &[
            tree_sitter_java::TAGS_QUERY,
            include_str!("../queries/java/tags.scm"),
        ],
        capture_keys: NO_OVERRIDES,
        base: None,
        tier: CompletenessTier::G0,
        token_sets: TOKENS_JAVA,
        extra_queries: ExtraQueries::NONE,
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
        locals: None,
        folds: None,
        tags: &[include_str!("../queries/kotlin/tags.scm")],
        capture_keys: NO_OVERRIDES,
        base: None,
        tier: CompletenessTier::G0,
        token_sets: TOKENS_KOTLIN,
        extra_queries: ExtraQueries::NONE,
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
        locals: None,
        folds: None,
        tags: &[],
        capture_keys: NO_OVERRIDES,
        base: None,
        tier: CompletenessTier::G0,
        token_sets: TOKENS_GROOVY,
        extra_queries: ExtraQueries::NONE,
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
        locals: None,
        folds: None,
        tags: &[include_str!("../queries/xml/tags.scm")],
        capture_keys: XML_KEYS,
        base: None,
        tier: CompletenessTier::G0,
        token_sets: TOKENS_XML,
        extra_queries: ExtraQueries::NONE,
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
        locals: None,
        folds: None,
        tags: &[include_str!("../queries/json/tags.scm")],
        capture_keys: NO_OVERRIDES,
        base: None,
        tier: CompletenessTier::G0,
        token_sets: TOKENS_JSON,
        extra_queries: ExtraQueries::NONE,
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
        locals: None,
        folds: None,
        tags: &[include_str!("../queries/toml/tags.scm")],
        capture_keys: NO_OVERRIDES,
        base: None,
        tier: CompletenessTier::G0,
        token_sets: TOKENS_TOML,
        extra_queries: ExtraQueries::NONE,
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
        locals: None,
        folds: None,
        tags: &[include_str!("../queries/yaml/tags.scm")],
        capture_keys: NO_OVERRIDES,
        base: None,
        tier: CompletenessTier::G0,
        token_sets: TOKENS_YAML,
        extra_queries: ExtraQueries::NONE,
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
        locals: None,
        folds: None,
        tags: &[include_str!("../queries/markdown/tags.scm")],
        capture_keys: NO_OVERRIDES,
        base: None,
        tier: CompletenessTier::G0,
        token_sets: TOKENS_MARKDOWN,
        extra_queries: ExtraQueries::NONE,
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
        locals: Some(tree_sitter_starlark::LOCALS_QUERY),
        folds: Some(tree_sitter_starlark::FOLDS_QUERY),
        tags: &[],
        capture_keys: NO_OVERRIDES,
        base: None,
        tier: CompletenessTier::G0,
        token_sets: TOKENS_STARLARK,
        extra_queries: ExtraQueries::NONE,
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
        locals: None,
        folds: None,
        tags: &[tree_sitter_python::TAGS_QUERY],
        capture_keys: NO_OVERRIDES,
        base: None,
        tier: CompletenessTier::G0,
        token_sets: TOKENS_PYTHON,
        extra_queries: ExtraQueries::NONE,
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
        locals: Some(tree_sitter_javascript::LOCALS_QUERY),
        folds: None,
        tags: &[tree_sitter_javascript::TAGS_QUERY],
        capture_keys: HTML_KEYS,
        base: None,
        tier: CompletenessTier::G0,
        token_sets: TOKENS_JAVASCRIPT,
        extra_queries: ExtraQueries::NONE,
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
        locals: Some(tree_sitter_typescript::LOCALS_QUERY),
        folds: None,
        tags: &[tree_sitter_typescript::TAGS_QUERY],
        capture_keys: NO_OVERRIDES,
        base: Some("javascript"),
        tier: CompletenessTier::G0,
        token_sets: TOKENS_TYPESCRIPT,
        extra_queries: ExtraQueries::NONE,
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
        locals: Some(tree_sitter_typescript::LOCALS_QUERY),
        folds: None,
        tags: &[tree_sitter_typescript::TAGS_QUERY],
        capture_keys: HTML_KEYS,
        base: Some("typescript"),
        tier: CompletenessTier::G0,
        token_sets: TOKENS_TSX,
        extra_queries: ExtraQueries::NONE,
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
        locals: None,
        folds: None,
        tags: &[tree_sitter_go::TAGS_QUERY],
        capture_keys: NO_OVERRIDES,
        base: None,
        tier: CompletenessTier::G0,
        token_sets: TOKENS_GO,
        extra_queries: ExtraQueries::NONE,
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
        locals: None,
        folds: None,
        tags: &[tree_sitter_c::TAGS_QUERY],
        capture_keys: NO_OVERRIDES,
        base: None,
        tier: CompletenessTier::G0,
        token_sets: TOKENS_C,
        extra_queries: ExtraQueries::NONE,
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
        locals: None,
        folds: None,
        tags: &[tree_sitter_cpp::TAGS_QUERY],
        capture_keys: NO_OVERRIDES,
        base: None,
        tier: CompletenessTier::G0,
        token_sets: TOKENS_CPP,
        extra_queries: ExtraQueries::NONE,
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
        locals: None,
        folds: None,
        tags: &[],
        capture_keys: NO_OVERRIDES,
        base: None,
        tier: CompletenessTier::G0,
        token_sets: TOKENS_BASH,
        extra_queries: ExtraQueries::NONE,
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
        locals: None,
        folds: None,
        tags: &[include_str!("../queries/html/tags.scm")],
        capture_keys: HTML_KEYS,
        base: None,
        tier: CompletenessTier::G0,
        token_sets: TOKENS_HTML,
        extra_queries: ExtraQueries::NONE,
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
        locals: None,
        folds: None,
        tags: &[],
        capture_keys: NO_OVERRIDES,
        base: None,
        tier: CompletenessTier::G0,
        token_sets: TOKENS_CSS,
        extra_queries: ExtraQueries::NONE,
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
        locals: None,
        folds: None,
        tags: &[tree_sitter_properties::TAGS_QUERY],
        capture_keys: PROPERTIES_KEYS,
        base: None,
        tier: CompletenessTier::G0,
        token_sets: TOKENS_PROPERTIES,
        extra_queries: ExtraQueries::NONE,
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
