//! Secrets never reach the disk: every line written to a transcript log goes through a
//! [`Redactor`] first.
//!
//! It removes (1) the values of environment variables whose names look like credentials, as
//! they are in the IDE's own environment, and (2) strings that look like well-known tokens
//! (API keys, GitHub tokens, cloud keys, bearer tokens, `NAME_KEY=value` assignments). The
//! replacement text is JSON-safe, so a redacted JSONL line stays valid JSON.

use regex::Regex;
use std::borrow::Cow;

pub const REPLACEMENT: &str = "[redacted]";

pub struct Redactor {
    exact: Vec<String>,
    patterns: Vec<(Regex, &'static str)>,
}

impl Redactor {
    /// `secret_values` are literal values to scrub wherever they appear.
    pub fn new(secret_values: impl IntoIterator<Item = String>) -> Self {
        let mut exact: Vec<String> = Vec::new();
        for value in secret_values {
            if value.len() < 8 {
                continue;
            }
            // The JSON-escaped form is what appears inside a serialized line.
            let escaped = serde_json::to_string(&value)
                .map(|s| s.trim_matches('"').to_owned())
                .unwrap_or_default();
            if escaped != value && !escaped.is_empty() {
                exact.push(escaped);
            }
            exact.push(value);
        }
        // Longest first, so a value containing another is removed whole.
        exact.sort_by_key(|v| std::cmp::Reverse(v.len()));
        exact.dedup();
        let table: [(&str, &str); 7] = [
            (r"sk-ant-[A-Za-z0-9_\-]{8,}", REPLACEMENT),
            (r"\bsk-[A-Za-z0-9_\-]{20,}", REPLACEMENT),
            (r"\bgh[pousr]_[A-Za-z0-9]{20,}", REPLACEMENT),
            (r"\bgithub_pat_[A-Za-z0-9_]{20,}", REPLACEMENT),
            (r"\b(?:AKIA|ASIA)[0-9A-Z]{16}\b", REPLACEMENT),
            (
                r"(?i)(\bbearer\s+)[A-Za-z0-9._~+/=\-]{16,}",
                "${1}[redacted]",
            ),
            (
                r#"(?i)(\b[A-Z0-9_]*(?:API_?KEY|TOKEN|SECRET|PASSWORD|PASSWD)[A-Z0-9_]*\s*[=:]\s*\\?["']?)[^\s"'\\,;]{6,}"#,
                "${1}[redacted]",
            ),
        ];
        let patterns = table
            .into_iter()
            .filter_map(|(pattern, replacement)| {
                Regex::new(pattern).ok().map(|regex| (regex, replacement))
            })
            .collect();
        Self { exact, patterns }
    }

    pub fn redact<'a>(&self, text: &'a str) -> Cow<'a, str> {
        let mut out = Cow::Borrowed(text);
        for value in &self.exact {
            if out.contains(value.as_str()) {
                out = Cow::Owned(out.replace(value.as_str(), REPLACEMENT));
            }
        }
        for (regex, replacement) in &self.patterns {
            if regex.is_match(&out) {
                out = Cow::Owned(regex.replace_all(&out, *replacement).into_owned());
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_token_shapes_and_assignments_are_removed() {
        let r = Redactor::new([]);
        for secret in [
            "sk-ant-api03-abcdefghijklmnop",
            "sk-abcdefghijklmnopqrstuvwxyz",
            "ghp_abcdefghijklmnopqrstuvwxyz0123",
            "AKIAABCDEFGHIJKLMNOP",
            "github_pat_11ABCDEFG0123456789_abcdef",
        ] {
            let line = format!(r#"{{"text":"key is {secret} ok"}}"#);
            let out = r.redact(&line);
            assert!(!out.contains(secret), "{secret} leaked: {out}");
            assert!(serde_json::from_str::<serde_json::Value>(&out).is_ok());
        }
        let out = r.redact(r#"{"text":"export OPENAI_API_KEY=abc123xyz789 && run; Authorization: Bearer abcdefghijklmnopqrstuv"}"#);
        assert!(!out.contains("abc123xyz789"));
        assert!(!out.contains("abcdefghijklmnopqrstuv"));
        assert!(out.contains("OPENAI_API_KEY="));
        assert!(serde_json::from_str::<serde_json::Value>(&out).is_ok());
    }

    #[test]
    fn environment_secret_values_are_removed_even_when_json_escaped() {
        let r = Redactor::new(["pa\"ss\\word-123456".to_string(), "short".to_string()]);
        let line = serde_json::json!({"text": "the pa\"ss\\word-123456 and short"}).to_string();
        let out = r.redact(&line);
        assert!(!out.contains("word-123456"), "{out}");
        assert!(out.contains("short"), "values under 8 chars are not treated as secrets");
        assert!(serde_json::from_str::<serde_json::Value>(&out).is_ok());
    }

    #[test]
    fn ordinary_text_is_untouched() {
        let r = Redactor::new([]);
        let text = r#"{"text":"fn main() { println!(\"hi\"); } // the token count is 5"}"#;
        assert_eq!(r.redact(text), text);
    }
}
