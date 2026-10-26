//! Inline (ghost text) code completion: prompt construction and cleanup of the
//! model output into text that can be inserted at the caret.

use crate::{ChatMessage, ChatRequest, ModelRole};

/// Characters of context before / after the caret sent with a request.
pub const PREFIX_CHARS: usize = 6_000;
pub const SUFFIX_CHARS: usize = 2_000;
/// Longest suggestion shown, in lines.
pub const MAX_LINES: usize = 12;
const MAX_TOKENS: u32 = 256;

pub const SYSTEM_PROMPT: &str = "You are an inline code completion engine inside an IDE. \
You receive a file with a <cursor/> marker. Reply with only the code to insert at the cursor: \
no explanations, no Markdown fences, no repetition of code before or after the cursor. \
Complete the current statement or block naturally; prefer short completions. If nothing \
sensible can be inserted, reply with an empty message.";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionContext {
    pub path: String,
    pub language: String,
    /// Text before the caret (already bounded).
    pub prefix: String,
    /// Text after the caret (already bounded).
    pub suffix: String,
}

impl CompletionContext {
    /// Builds a context from the whole document and a caret char offset.
    pub fn new(path: &str, language: &str, text: &str, caret: usize) -> Self {
        let chars: Vec<char> = text.chars().collect();
        let caret = caret.min(chars.len());
        let start = caret.saturating_sub(PREFIX_CHARS);
        let end = (caret + SUFFIX_CHARS).min(chars.len());
        Self {
            path: path.into(),
            language: language.into(),
            prefix: chars[start..caret].iter().collect(),
            suffix: chars[caret..end].iter().collect(),
        }
    }

    /// Text of the caret line after the caret.
    pub fn rest_of_line(&self) -> &str {
        self.suffix.split('\n').next().unwrap_or_default()
    }

    /// Inline completion is offered only at the end of a line (possibly before
    /// closing delimiters), after some code has been typed on it or above it.
    pub fn is_eligible(&self) -> bool {
        let rest = self.rest_of_line().trim();
        rest.chars()
            .all(|c| matches!(c, ')' | ']' | '}' | ';' | ',' | '"' | '\''))
            && rest.chars().count() <= 3
            && !self.prefix.trim().is_empty()
    }

    pub fn request(&self) -> ChatRequest {
        let message = format!(
            "<file path=\"{}\" language=\"{}\">\n{}<cursor/>{}\n</file>",
            self.path, self.language, self.prefix, self.suffix
        );
        ChatRequest {
            role: ModelRole::Completion,
            system: SYSTEM_PROMPT.into(),
            messages: vec![ChatMessage::user(message)],
            tools: Vec::new(),
            max_tokens: Some(MAX_TOKENS),
        }
    }
}

/// Turns raw model output into insertable text: strips Markdown fences and a
/// repeated prefix line, drops text that duplicates what follows the caret and
/// limits the length. Returns `None` when nothing useful remains.
pub fn clean(raw: &str, context: &CompletionContext) -> Option<String> {
    let mut text = raw.replace("\r\n", "\n");
    let trimmed = text.trim_start();
    if trimmed.starts_with("```") {
        let body = trimmed.split_once('\n').map(|(_, body)| body).unwrap_or("");
        text = body
            .rsplit_once("```")
            .map(|(code, _)| code)
            .unwrap_or(body)
            .trim_end_matches('\n')
            .to_owned();
    }
    text = text.replace("<cursor/>", "");
    // Models sometimes echo the current line; drop the already-typed part.
    let current_line = context.prefix.rsplit('\n').next().unwrap_or_default();
    if !current_line.trim().is_empty()
        && let Some(rest) = text.strip_prefix(current_line)
    {
        text = rest.to_owned();
    }
    // Do not re-insert closing delimiters that already follow the caret.
    let rest = context.rest_of_line().trim_end();
    if !rest.is_empty() && text.trim_end().ends_with(rest) {
        let cut = text.trim_end().len() - rest.len();
        text.truncate(cut);
    }
    let mut lines: Vec<&str> = text.split('\n').collect();
    lines.truncate(MAX_LINES);
    // Drop trailing lines that duplicate the start of the suffix.
    let next_lines: Vec<&str> = context.suffix.split('\n').skip(1).collect();
    while let (Some(last), Some(next)) = (lines.last(), next_lines.first())
        && lines.len() > 1
        && !last.trim().is_empty()
        && last.trim() == next.trim()
    {
        lines.pop();
    }
    let mut text = lines.join("\n").trim_end().to_owned();
    if text.trim().is_empty() {
        return None;
    }
    // A multi-line block body completed inside auto-inserted closers ends on a
    // new line so the closer keeps its own line: `{|}` → `{\n    body\n}`.
    let opens_block = current_line.trim_end().ends_with(['{', '(', '[']);
    if text.contains('\n') && opens_block && rest.starts_with(['}', ')', ']']) {
        let indent: String = current_line
            .chars()
            .take_while(|c| c.is_whitespace())
            .collect();
        text.push('\n');
        text.push_str(&indent);
    }
    Some(text)
}

/// Splits off the next word of a suggestion (Ctrl+→): leading whitespace plus one
/// run of word characters, or a single other character.
pub fn next_word(suggestion: &str) -> (&str, &str) {
    if suggestion.starts_with('\n') {
        return suggestion.split_at(1);
    }
    let blank = suggestion
        .find(|c: char| !c.is_whitespace() || c == '\n')
        .unwrap_or(suggestion.len());
    let rest = &suggestion[blank..];
    let word = |c: char| c.is_alphanumeric() || c == '_';
    let len = match rest.chars().next() {
        None | Some('\n') => 0,
        Some(c) if word(c) => rest.find(|c: char| !word(c)).unwrap_or(rest.len()),
        Some(c) => c.len_utf8(),
    };
    suggestion.split_at(blank + len)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context(text: &str) -> CompletionContext {
        let caret = text.find('|').unwrap();
        let text = text.replace('|', "");
        CompletionContext::new("src/lib.rs", "rust", &text, text[..caret].chars().count())
    }

    #[test]
    fn eligibility_and_bounded_context() {
        assert!(context("fn a() {\n    let x = |\n}").is_eligible());
        assert!(context("call(|)").is_eligible());
        assert!(!context("let x = |value;").is_eligible());
        assert!(!context("|").is_eligible());
        let long = format!(
            "{}|{}",
            "a".repeat(PREFIX_CHARS + 10),
            "b".repeat(SUFFIX_CHARS + 10)
        );
        let context = context(&long);
        assert_eq!(context.prefix.len(), PREFIX_CHARS);
        assert_eq!(context.suffix.len(), SUFFIX_CHARS);
        let request = context.request();
        assert_eq!(request.role, ModelRole::Completion);
        assert_eq!(request.max_tokens, Some(MAX_TOKENS));
        assert!(request.messages[0].text().contains("<cursor/>"));
    }

    #[test]
    fn cleans_fences_echoes_and_suffix_overlap() {
        let ctx = context("fn main() {\n    let total = |\n}");
        assert_eq!(
            clean("```rust\nitems.iter().sum();\n```", &ctx).as_deref(),
            Some("items.iter().sum();")
        );
        assert_eq!(
            clean("    let total = items.len();", &ctx).as_deref(),
            Some("items.len();")
        );
        assert_eq!(clean("   \n", &ctx), None);
        let ctx = context("call(|)");
        assert_eq!(clean("a, b)", &ctx).as_deref(), Some("a, b"));
        let ctx = context("if x {|\n}\n");
        assert_eq!(clean("\n    y();\n}", &ctx).as_deref(), Some("\n    y();"));
        let ctx = context("    fn a() {|}");
        assert_eq!(
            clean("\n        b();\n    }", &ctx).as_deref(),
            Some("\n        b();\n    ")
        );
        assert_eq!(clean("b()", &ctx).as_deref(), Some("b()"));
        let many = "x\n".repeat(30);
        assert_eq!(
            clean(&many, &context("a|")).unwrap().lines().count(),
            MAX_LINES
        );
    }

    #[test]
    fn splits_words_for_partial_acceptance() {
        assert_eq!(next_word("items.iter()"), ("items", ".iter()"));
        assert_eq!(next_word(".iter()"), (".", "iter()"));
        assert_eq!(next_word("  foo bar"), ("  foo", " bar"));
        assert_eq!(next_word("\n    y();"), ("\n", "    y();"));
        assert_eq!(next_word("größe + 1"), ("größe", " + 1"));
        assert_eq!(next_word(""), ("", ""));
    }
}
