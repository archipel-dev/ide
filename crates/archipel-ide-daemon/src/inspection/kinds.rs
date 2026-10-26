//! Per-language node-kind tables the Tier-A inspections are driven by.
//! Data, not code: a language gains an inspection by listing its kinds. A
//! test checks every kind name exists in its grammar.

/// Kind names of one language, by role. Empty slices disable a rule.
#[derive(Debug, Clone, Copy)]
pub struct Kinds {
    pub lang: &'static str,
    /// Nodes whose named children are statements.
    pub containers: &'static [&'static str],
    /// Statements that never complete normally (return/break/continue/throw).
    pub terminators: &'static [&'static str],
    /// Statement wrappers around an expression (`expression_statement`).
    pub expr_statements: &'static [&'static str],
    /// First following statements that stay reachable (labels, hoisted decls).
    pub reachable_starts: &'static [&'static str],
    /// Brace/indent bodies.
    pub blocks: &'static [&'static str],
    /// `if` constructs with `consequence`/`alternative` fields.
    pub ifs: &'static [&'static str],
    /// Constructs with a `condition` field that is a plain `if`-like test.
    pub conditions: &'static [&'static str],
    /// Loops whose `while (false)` is a constant condition.
    pub whiles: &'static [&'static str],
    pub catches: &'static [&'static str],
    pub assignments: &'static [&'static str],
    /// `a = b` in a condition is a likely typo for these languages.
    pub assign_in_condition: bool,
    pub parens: &'static [&'static str],
    /// Parents whose `condition`/value child never needs parentheses.
    pub paren_free_parents: &'static [&'static str],
    pub boolean_literals: &'static [&'static str],
    pub comment_prefix: &'static str,
}

const NONE: &[&str] = &[];

const JAVA_TERMINATORS: &[&str] = &[
    "return_statement",
    "break_statement",
    "continue_statement",
    "throw_statement",
];
const C_TERMINATORS: &[&str] = &[
    "return_statement",
    "break_statement",
    "continue_statement",
    "goto_statement",
];
const CPP_TERMINATORS: &[&str] = &[
    "return_statement",
    "break_statement",
    "continue_statement",
    "goto_statement",
    "throw_statement",
];

pub static KINDS: &[Kinds] = &[
    Kinds {
        lang: "java",
        containers: &["block"],
        terminators: JAVA_TERMINATORS,
        expr_statements: &["expression_statement"],
        reachable_starts: &["labeled_statement"],
        blocks: &["block"],
        ifs: &["if_statement"],
        conditions: &["if_statement"],
        whiles: &["while_statement"],
        catches: &["catch_clause"],
        assignments: &["assignment_expression"],
        assign_in_condition: true,
        parens: &["parenthesized_expression"],
        paren_free_parents: NONE,
        boolean_literals: &["true", "false"],
        comment_prefix: "//",
    },
    Kinds {
        lang: "groovy",
        containers: &["block"],
        terminators: JAVA_TERMINATORS,
        expr_statements: &["expression_statement"],
        reachable_starts: NONE,
        blocks: &["block"],
        ifs: &["if_statement"],
        conditions: &["if_statement"],
        whiles: NONE,
        catches: &["catch_clause"],
        assignments: &["assignment_expression"],
        assign_in_condition: true,
        parens: NONE,
        paren_free_parents: NONE,
        boolean_literals: NONE,
        comment_prefix: "//",
    },
    Kinds {
        lang: "rust",
        containers: &["block"],
        terminators: &[
            "return_expression",
            "break_expression",
            "continue_expression",
        ],
        expr_statements: &["expression_statement"],
        reachable_starts: &[
            "function_item",
            "struct_item",
            "enum_item",
            "impl_item",
            "use_declaration",
        ],
        blocks: &["block"],
        ifs: &["if_expression"],
        conditions: &["if_expression"],
        whiles: &["while_expression"],
        catches: NONE,
        assignments: &["assignment_expression"],
        assign_in_condition: false,
        parens: &["parenthesized_expression"],
        paren_free_parents: &["if_expression", "while_expression", "let_declaration"],
        boolean_literals: &["boolean_literal"],
        comment_prefix: "//",
    },
    Kinds {
        lang: "python",
        containers: &["block"],
        terminators: &[
            "return_statement",
            "break_statement",
            "continue_statement",
            "raise_statement",
        ],
        expr_statements: &["expression_statement"],
        reachable_starts: NONE,
        blocks: &["block"],
        ifs: &["if_statement"],
        conditions: &["if_statement", "elif_clause"],
        whiles: &["while_statement"],
        catches: NONE,
        assignments: &["assignment"],
        assign_in_condition: false,
        parens: &["parenthesized_expression"],
        paren_free_parents: &["if_statement", "elif_clause", "while_statement"],
        boolean_literals: &["true", "false"],
        comment_prefix: "#",
    },
    Kinds {
        lang: "javascript",
        containers: &["statement_block"],
        terminators: JAVA_TERMINATORS,
        expr_statements: &["expression_statement"],
        reachable_starts: &[
            "function_declaration",
            "class_declaration",
            "labeled_statement",
        ],
        blocks: &["statement_block"],
        ifs: &["if_statement"],
        conditions: &["if_statement"],
        whiles: &["while_statement"],
        catches: &["catch_clause"],
        assignments: &["assignment_expression"],
        assign_in_condition: true,
        parens: &["parenthesized_expression"],
        paren_free_parents: NONE,
        boolean_literals: &["true", "false"],
        comment_prefix: "//",
    },
    Kinds {
        lang: "typescript",
        containers: &["statement_block"],
        terminators: JAVA_TERMINATORS,
        expr_statements: &["expression_statement"],
        reachable_starts: &[
            "function_declaration",
            "class_declaration",
            "labeled_statement",
        ],
        blocks: &["statement_block"],
        ifs: &["if_statement"],
        conditions: &["if_statement"],
        whiles: &["while_statement"],
        catches: &["catch_clause"],
        assignments: &["assignment_expression"],
        assign_in_condition: true,
        parens: &["parenthesized_expression"],
        paren_free_parents: NONE,
        boolean_literals: &["true", "false"],
        comment_prefix: "//",
    },
    Kinds {
        lang: "tsx",
        containers: &["statement_block"],
        terminators: JAVA_TERMINATORS,
        expr_statements: &["expression_statement"],
        reachable_starts: &[
            "function_declaration",
            "class_declaration",
            "labeled_statement",
        ],
        blocks: &["statement_block"],
        ifs: &["if_statement"],
        conditions: &["if_statement"],
        whiles: &["while_statement"],
        catches: &["catch_clause"],
        assignments: &["assignment_expression"],
        assign_in_condition: true,
        parens: &["parenthesized_expression"],
        paren_free_parents: NONE,
        boolean_literals: &["true", "false"],
        comment_prefix: "//",
    },
    Kinds {
        lang: "go",
        containers: &["statement_list"],
        terminators: &[
            "return_statement",
            "break_statement",
            "continue_statement",
            "goto_statement",
        ],
        expr_statements: &["expression_statement"],
        reachable_starts: &["labeled_statement"],
        blocks: &["block"],
        ifs: &["if_statement"],
        conditions: &["if_statement"],
        whiles: NONE,
        catches: NONE,
        assignments: &["assignment_statement"],
        assign_in_condition: false,
        parens: &["parenthesized_expression"],
        paren_free_parents: &["if_statement"],
        boolean_literals: &["true", "false"],
        comment_prefix: "//",
    },
    Kinds {
        lang: "c",
        containers: &["compound_statement", "case_statement"],
        terminators: C_TERMINATORS,
        expr_statements: &["expression_statement"],
        reachable_starts: &["labeled_statement", "case_statement"],
        blocks: &["compound_statement"],
        ifs: &["if_statement"],
        conditions: &["if_statement"],
        whiles: &["while_statement"],
        catches: NONE,
        assignments: &["assignment_expression"],
        assign_in_condition: true,
        parens: &["parenthesized_expression"],
        paren_free_parents: NONE,
        boolean_literals: &["true", "false"],
        comment_prefix: "//",
    },
    Kinds {
        lang: "cpp",
        containers: &["compound_statement", "case_statement"],
        terminators: CPP_TERMINATORS,
        expr_statements: &["expression_statement"],
        reachable_starts: &["labeled_statement", "case_statement"],
        blocks: &["compound_statement"],
        ifs: &["if_statement"],
        conditions: &["if_statement"],
        whiles: &["while_statement"],
        catches: &["catch_clause"],
        assignments: &["assignment_expression"],
        assign_in_condition: true,
        parens: &["parenthesized_expression"],
        paren_free_parents: NONE,
        boolean_literals: &["true", "false"],
        comment_prefix: "//",
    },
    Kinds {
        lang: "kotlin",
        containers: &["statements"],
        terminators: &["jump_expression"],
        expr_statements: NONE,
        reachable_starts: NONE,
        blocks: &["control_structure_body"],
        ifs: &["if_expression"],
        conditions: &["if_expression"],
        whiles: NONE,
        catches: &["catch_block"],
        assignments: &["assignment"],
        assign_in_condition: false,
        parens: &["parenthesized_expression"],
        paren_free_parents: NONE,
        boolean_literals: &["boolean_literal"],
        comment_prefix: "//",
    },
];

/// The table of `lang`, if it has one.
pub fn kinds_of(lang: &str) -> Option<&'static Kinds> {
    KINDS.iter().find(|k| k.lang == lang)
}

/// Line-comment prefix used for generated suppression comments (`None` =
/// the language has no line comments; use [`block_comment`]).
pub fn line_comment_prefix(lang: &str) -> Option<&'static str> {
    Some(match lang {
        "rust" | "java" | "kotlin" | "groovy" | "go" | "c" | "cpp" | "javascript"
        | "typescript" | "tsx" | "css" => "//",
        "python" | "yaml" | "toml" | "bash" | "properties" | "starlark" => "#",
        _ => return None,
    })
}

/// `(open, close)` of block comments for languages without line comments.
pub fn block_comment(lang: &str) -> Option<(&'static str, &'static str)> {
    match lang {
        "xml" | "html" | "markdown" => Some(("<!--", "-->")),
        "css" => Some(("/*", "*/")),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use archipel_ide_language::registry;

    #[test]
    fn every_listed_kind_exists_in_its_grammar() {
        for kinds in KINDS {
            let def = registry::by_id(kinds.lang).expect("registered language");
            let table = def.element_types();
            for name in kinds
                .containers
                .iter()
                .chain(kinds.terminators)
                .chain(kinds.expr_statements)
                .chain(kinds.reachable_starts)
                .chain(kinds.blocks)
                .chain(kinds.ifs)
                .chain(kinds.conditions)
                .chain(kinds.whiles)
                .chain(kinds.catches)
                .chain(kinds.assignments)
                .chain(kinds.parens)
                .chain(kinds.paren_free_parents)
                .chain(kinds.boolean_literals)
            {
                assert!(
                    table.contains_name(name),
                    "{}: grammar has no kind `{name}`",
                    kinds.lang
                );
            }
        }
    }
}
