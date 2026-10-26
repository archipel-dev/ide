//! AI actions on code (Explain, Refactor, Generate tests/docs, Find problems,
//! Fix with AI) and helpers for build-failure context.

/// What an action asks the model to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AiAction {
    Explain,
    Refactor,
    GenerateTests,
    GenerateDocs,
    FindProblems,
    /// Fix the listed problems (`line: message`) in the attached code.
    FixProblems(Vec<String>),
    /// Explain and fix a failed build or run.
    FixBuild {
        command: String,
    },
}

impl AiAction {
    /// Command id suffix (`ai.action.<id>`).
    pub fn id(&self) -> &'static str {
        match self {
            AiAction::Explain => "explain",
            AiAction::Refactor => "refactor",
            AiAction::GenerateTests => "generateTests",
            AiAction::GenerateDocs => "generateDocs",
            AiAction::FindProblems => "findProblems",
            AiAction::FixProblems(_) => "fixProblems",
            AiAction::FixBuild { .. } => "fixBuild",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            AiAction::Explain => "Explain Code",
            AiAction::Refactor => "Suggest Refactoring",
            AiAction::GenerateTests => "Generate Unit Tests",
            AiAction::GenerateDocs => "Generate Documentation",
            AiAction::FindProblems => "Find Problems",
            AiAction::FixProblems(_) => "Fix with AI",
            AiAction::FixBuild { .. } => "Fix Build with AI",
        }
    }

    /// Whether the answer is a replacement for the attached code, so the diff
    /// preview opens as soon as it completes.
    pub fn edits_code(&self) -> bool {
        matches!(
            self,
            AiAction::Refactor | AiAction::GenerateDocs | AiAction::FixProblems(_)
        )
    }

    /// The message shown in the chat and sent with the attached context.
    pub fn prompt(&self) -> String {
        match self {
            AiAction::Explain => "/explain".into(),
            AiAction::Refactor => "/refactor".into(),
            AiAction::GenerateTests => "/tests".into(),
            AiAction::GenerateDocs => "/docs".into(),
            AiAction::FindProblems => "Review the attached code for bugs, edge cases and \
                risky constructs. List each problem with its line and a one-line fix; do not \
                rewrite the code."
                .into(),
            AiAction::FixProblems(problems) => format!(
                "Fix these problems in the attached code and return the complete corrected \
                 code in one block:\n{}",
                problems
                    .iter()
                    .map(|problem| format!("- {problem}"))
                    .collect::<Vec<_>>()
                    .join("\n")
            ),
            AiAction::FixBuild { command } => format!(
                "`{command}` failed. Explain the cause from the attached output and show the \
                 fix. When the fix is in the attached file, return the complete corrected \
                 file in one block."
            ),
        }
    }
}

/// The last `max_lines` lines of build output.
pub fn output_tail(output: &str, max_lines: usize) -> String {
    let lines: Vec<&str> = output.lines().collect();
    let start = lines.len().saturating_sub(max_lines);
    lines[start..].join("\n")
}

/// First `(path, 1-based line)` reported as an error in compiler output.
///
/// Understands rustc (`--> src/main.rs:3:5`), javac/gcc/clang
/// (`src/Main.java:3: error:` / `a.c:3:5: error:`), Kotlin (`e: file:///p/A.kt:3:5`)
/// and Gradle-style `w:`/`e:` prefixes.
pub fn first_error_location(output: &str) -> Option<(String, usize)> {
    let mut lines = output.lines().peekable();
    while let Some(line) = lines.next() {
        let trimmed = line.trim_start();
        let is_error = trimmed.starts_with("error")
            || trimmed.contains(": error")
            || trimmed.starts_with("e: ");
        if !is_error {
            continue;
        }
        if let Some(location) = trimmed.strip_prefix("e: ") {
            let location = location.strip_prefix("file://").unwrap_or(location);
            if let Some(found) = parse_location(location) {
                return Some(found);
            }
        }
        if trimmed.contains(": error")
            && let Some(found) = parse_location(trimmed)
        {
            return Some(found);
        }
        // rustc: the location follows on a `-->` line.
        for next in lines.by_ref().take(4) {
            if let Some(location) = next.trim_start().strip_prefix("--> ") {
                if let Some(found) = parse_location(location) {
                    return Some(found);
                }
                break;
            }
        }
    }
    None
}

/// Parses `path:line[:column][: …]`.
fn parse_location(text: &str) -> Option<(String, usize)> {
    let mut parts = text.split(':');
    let mut path = parts.next()?.trim().to_owned();
    // Windows drive letters (`C:\…`).
    if path.len() == 1 && path.chars().all(|c| c.is_ascii_alphabetic()) {
        path = format!("{path}:{}", parts.next()?);
    }
    let line = parts.next()?.trim().parse::<usize>().ok()?;
    (!path.is_empty() && line > 0).then_some((path, line))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompts_and_edit_actions() {
        assert_eq!(AiAction::Explain.prompt(), "/explain");
        assert!(AiAction::Refactor.edits_code());
        assert!(!AiAction::Explain.edits_code());
        let fix = AiAction::FixProblems(vec!["4: mismatched types".into()]);
        assert!(fix.prompt().ends_with("\n- 4: mismatched types"));
        assert_eq!(fix.id(), "fixProblems");
        assert!(
            AiAction::FixBuild {
                command: "cargo build".into()
            }
            .prompt()
            .starts_with("`cargo build` failed.")
        );
        assert_eq!(output_tail("a\nb\nc", 2), "b\nc");
        assert_eq!(output_tail("a", 5), "a");
    }

    #[test]
    fn finds_first_error_locations_across_toolchains() {
        let rustc = "   Compiling demo v0.1.0\nwarning: unused\n --> src/lib.rs:1:1\nerror[E0308]: mismatched types\n --> src/main.rs:4:18\n  |\n";
        assert_eq!(first_error_location(rustc), Some(("src/main.rs".into(), 4)));
        let javac = "> Task :app:compileJava\n/p/app/src/Main.java:7: error: ';' expected\n";
        assert_eq!(
            first_error_location(javac),
            Some(("/p/app/src/Main.java".into(), 7))
        );
        let kotlin = "e: file:///p/app/src/A.kt:12:5 Unresolved reference: foo\n";
        assert_eq!(
            first_error_location(kotlin),
            Some(("/p/app/src/A.kt".into(), 12))
        );
        let gcc = "a.c:3:5: error: unknown type name\n";
        assert_eq!(first_error_location(gcc), Some(("a.c".into(), 3)));
        assert_eq!(first_error_location("all good\n"), None);
        assert_eq!(first_error_location("error: could not compile\n"), None);
    }
}
