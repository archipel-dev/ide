//! Provider-neutral conversation model.
//!
//! The shape follows the Anthropic Messages API (content blocks, tool use and tool
//! results) because it is the richest of the supported wire formats; the other
//! providers translate to and from it.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
}

/// One block of message content.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    Text {
        text: String,
    },
    ToolUse {
        id: String,
        name: String,
        input: Value,
    },
    ToolResult {
        tool_use_id: String,
        content: String,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        is_error: bool,
    },
    /// Provider-internal blocks (thinking with signatures, fallback markers, …)
    /// that must be echoed back verbatim in later requests.
    #[serde(rename = "raw")]
    Raw {
        value: Value,
    },
}

impl ContentBlock {
    pub fn text(text: impl Into<String>) -> Self {
        ContentBlock::Text { text: text.into() }
    }

    /// Wire type of the block (`text`, `tool_use`, `thinking`, …).
    pub fn kind(&self) -> &str {
        match self {
            ContentBlock::Text { .. } => "text",
            ContentBlock::ToolUse { .. } => "tool_use",
            ContentBlock::ToolResult { .. } => "tool_result",
            ContentBlock::Raw { value } => value.get("type").and_then(Value::as_str).unwrap_or(""),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: Role,
    pub content: Vec<ContentBlock>,
}

impl ChatMessage {
    pub fn user(text: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: vec![ContentBlock::text(text)],
        }
    }

    pub fn assistant(text: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: vec![ContentBlock::text(text)],
        }
    }

    /// Concatenated text blocks.
    pub fn text(&self) -> String {
        self.content
            .iter()
            .filter_map(|block| match block {
                ContentBlock::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect()
    }
}

/// A client-side tool the model may call.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    /// JSON Schema of the tool input.
    pub input_schema: Value,
}

/// Which configured model a request is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelRole {
    /// Chat, actions and agent mode: the most capable configured model.
    Chat,
    /// Inline completion: a fast, small model.
    Completion,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChatRequest {
    pub role: ModelRole,
    pub system: String,
    pub messages: Vec<ChatMessage>,
    pub tools: Vec<ToolSpec>,
    /// Overrides the configured output limit.
    pub max_tokens: Option<u32>,
}

impl ChatRequest {
    pub fn chat(system: impl Into<String>, messages: Vec<ChatMessage>) -> Self {
        Self {
            role: ModelRole::Chat,
            system: system.into(),
            messages,
            tools: Vec::new(),
            max_tokens: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    #[default]
    EndTurn,
    MaxTokens,
    ToolUse,
    StopSequence,
    /// A safety classifier or the model declined; partial output must be discarded.
    Refusal,
    PauseTurn,
    Other(String),
}

impl StopReason {
    pub fn parse(text: &str) -> Self {
        match text {
            "end_turn" | "stop" => StopReason::EndTurn,
            "max_tokens" | "length" => StopReason::MaxTokens,
            "tool_use" | "tool_calls" => StopReason::ToolUse,
            "stop_sequence" => StopReason::StopSequence,
            "refusal" | "content_filter" => StopReason::Refusal,
            "pause_turn" => StopReason::PauseTurn,
            other => StopReason::Other(other.into()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Usage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_input_tokens: u64,
    pub cache_creation_input_tokens: u64,
}

/// The complete assistant turn assembled from a stream.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ChatResponse {
    pub model: String,
    pub content: Vec<ContentBlock>,
    pub stop_reason: StopReason,
    pub usage: Usage,
    /// Tool calls whose streamed input was not a valid JSON object, as
    /// `(tool_use_id, raw input)`. Their `ToolUse` blocks carry `{}`; callers must
    /// answer them with an error result instead of running the tool.
    pub invalid_tool_inputs: Vec<(String, String)>,
}

impl ChatResponse {
    pub fn text(&self) -> String {
        self.content
            .iter()
            .filter_map(|block| match block {
                ContentBlock::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect()
    }

    pub fn tool_uses(&self) -> impl Iterator<Item = (&str, &str, &Value)> {
        self.content.iter().filter_map(|block| match block {
            ContentBlock::ToolUse { id, name, input } => Some((id.as_str(), name.as_str(), input)),
            _ => None,
        })
    }

    /// Content to append to the conversation history.
    ///
    /// After a mid-output server-side fallback, blocks of the declined attempt that
    /// only the declined model can interpret (thinking, tool calls, unknown internal
    /// blocks) are dropped; text and everything after the last `fallback` marker
    /// is kept.
    pub fn history_content(&self) -> Vec<ContentBlock> {
        let Some(boundary) = self
            .content
            .iter()
            .rposition(|block| block.kind() == "fallback")
        else {
            return self.content.clone();
        };
        self.content
            .iter()
            .enumerate()
            .filter(|(index, block)| {
                *index >= boundary || matches!(block, ContentBlock::Text { .. })
            })
            .map(|(_, block)| block.clone())
            .collect()
    }
}

/// Incremental output delivered while a response streams.
#[derive(Debug, Clone, PartialEq)]
pub enum StreamEvent {
    /// The serving model is known (it may be a fallback model).
    Started {
        model: String,
    },
    TextDelta(String),
    /// The model is reasoning; no visible text yet.
    Thinking,
    /// A tool call has fully streamed and its input parsed.
    ToolUse {
        id: String,
        name: String,
        input: Value,
    },
    /// The provider switched to a fallback model after a refusal.
    Fallback {
        from: String,
        to: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum AiError {
    #[error("AI is not configured: {0}")]
    NotConfigured(String),
    #[error("no API key: set {env} or enter a key in AI settings")]
    MissingApiKey { env: String },
    #[error("authentication failed ({0}); check the API key in AI settings")]
    Auth(String),
    #[error("rate limited{}: {message}", retry_after.map(|s| format!(", retry in {s}s")).unwrap_or_default())]
    RateLimited {
        retry_after: Option<u64>,
        message: String,
    },
    #[error("the provider is overloaded, try again shortly: {0}")]
    Overloaded(String),
    #[error("request rejected (HTTP {status}): {message}")]
    Http { status: u16, message: String },
    #[error("network error: {0}")]
    Network(String),
    #[error("unexpected response from provider: {0}")]
    Protocol(String),
    #[error("the request was declined by the model's safety policy{}", .0.as_ref().map(|c| format!(" ({c})")).unwrap_or_default())]
    Refusal(Option<String>),
    #[error("cancelled")]
    Cancelled,
}

/// Cooperative cancellation shared with the streaming thread.
#[derive(Debug, Clone, Default)]
pub struct CancelToken(Arc<AtomicBool>);

impl CancelToken {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

/// A model backend. Implementations block the calling thread while streaming;
/// [`crate::AiService`] runs them on background threads.
pub trait Provider: Send + Sync {
    /// Stable provider id (`anthropic`, `openai`, `ollama`, `mock`).
    fn id(&self) -> &str;

    /// Model that serves `role`.
    fn model(&self, role: ModelRole) -> String;

    /// Streams one assistant turn, reporting increments to `sink`, and returns the
    /// assembled response. Must return [`AiError::Cancelled`] promptly once `cancel`
    /// is set.
    fn stream(
        &self,
        request: &ChatRequest,
        cancel: &CancelToken,
        sink: &mut dyn FnMut(StreamEvent),
    ) -> Result<ChatResponse, AiError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn history_drops_declined_internal_blocks_before_fallback_marker() {
        let response = ChatResponse {
            content: vec![
                ContentBlock::Raw {
                    value: json!({"type": "thinking", "thinking": "", "signature": "s"}),
                },
                ContentBlock::text("partial "),
                ContentBlock::ToolUse {
                    id: "t1".into(),
                    name: "read".into(),
                    input: json!({}),
                },
                ContentBlock::Raw {
                    value: json!({"type": "fallback"}),
                },
                ContentBlock::Raw {
                    value: json!({"type": "thinking", "thinking": "", "signature": "x"}),
                },
                ContentBlock::text("rest"),
            ],
            ..ChatResponse::default()
        };
        let kinds: Vec<_> = response
            .history_content()
            .iter()
            .map(|b| b.kind().to_owned())
            .collect();
        assert_eq!(kinds, ["text", "fallback", "thinking", "text"]);
        assert_eq!(response.text(), "partial rest");
    }

    #[test]
    fn stop_reasons_map_across_providers() {
        assert_eq!(StopReason::parse("tool_calls"), StopReason::ToolUse);
        assert_eq!(StopReason::parse("length"), StopReason::MaxTokens);
        assert_eq!(StopReason::parse("refusal"), StopReason::Refusal);
        assert_eq!(
            StopReason::parse("weird"),
            StopReason::Other("weird".into())
        );
    }
}
