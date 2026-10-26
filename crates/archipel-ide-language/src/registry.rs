//! Language registry: every supported language is a grammar crate plus the
//! queries that crate ships. There is no per-language Rust code here, only data.

use std::path::Path;
use tree_sitter_language::LanguageFn;

/// A registered language: grammar plus the upstream queries bundled with it.
pub struct LanguageSpec {
    /// Stable identifier (also used as the injection language name).
    pub id: &'static str,
    /// Display name shown in the status bar and menus.
    pub name: &'static str,
    /// File extensions without the leading dot, matched case-insensitively.
    pub extensions: &'static [&'static str],
    /// Exact file names (e.g. `BUILD.bazel`).
    pub file_names: &'static [&'static str],
    /// Interpreter names matched against a `#!` first line.
    pub shebangs: &'static [&'static str],
    pub(crate) grammar: LanguageFn,
    /// Highlight queries, concatenated in order. Base-language queries come first
    /// (C before C++, JavaScript before TypeScript), as upstream configures them.
    pub(crate) highlights: &'static [&'static str],
    pub(crate) injections: Option<&'static str>,
    pub(crate) locals: Option<&'static str>,
    /// Folding query (`folds.scm`) when the grammar ships one.
    pub(crate) folds: Option<&'static str>,
    /// Tags queries (definitions for Structure/breadcrumbs), concatenated.
    pub(crate) tags: &'static [&'static str],
    /// Capture-name overrides for languages whose IntelliJ keys differ from
    /// the defaults (e.g. `tag` → `HTML_TAG_NAME`). Data, not code.
    pub(crate) capture_keys: &'static [(&'static str, &'static str)],
}

impl LanguageSpec {
    pub fn grammar(&self) -> tree_sitter::Language {
        self.grammar.into()
    }

    pub fn has_tags(&self) -> bool {
        !self.tags.is_empty()
    }

    pub fn has_injections(&self) -> bool {
        self.injections.is_some()
    }

    pub fn has_locals(&self) -> bool {
        self.locals.is_some()
    }
}

impl std::fmt::Debug for LanguageSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LanguageSpec")
            .field("id", &self.id)
            .finish()
    }
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

/// All registered languages, in detection priority order.
pub static LANGUAGES: &[LanguageSpec] = &[
    LanguageSpec {
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
    },
    LanguageSpec {
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
    },
    LanguageSpec {
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
    },
    LanguageSpec {
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
    },
    LanguageSpec {
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
    },
    LanguageSpec {
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
    },
    LanguageSpec {
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
    },
    LanguageSpec {
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
    },
    LanguageSpec {
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
    },
    LanguageSpec {
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
    },
    LanguageSpec {
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
    },
    LanguageSpec {
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
    },
    LanguageSpec {
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
    },
    LanguageSpec {
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
    },
    LanguageSpec {
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
    },
    LanguageSpec {
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
    },
    LanguageSpec {
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
    },
    LanguageSpec {
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
    },
    LanguageSpec {
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
    },
    LanguageSpec {
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
    },
    LanguageSpec {
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
    },
    LanguageSpec {
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
