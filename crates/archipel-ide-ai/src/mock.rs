//! Scripted provider for tests and offline demos; never touches the network.

use crate::{
    AiError, CancelToken, ChatRequest, ChatResponse, ContentBlock, ModelRole, Provider, StopReason,
    StreamEvent,
};
use serde_json::Value;
use std::collections::VecDeque;
use std::sync::Mutex;
use std::time::Duration;

/// One scripted assistant turn.
#[derive(Debug, Clone, Default)]
pub struct MockReply {
    /// Text streamed piece by piece.
    pub chunks: Vec<String>,
    /// Tool calls `(name, input)` issued after the text.
    pub tool_calls: Vec<(String, Value)>,
    pub error: Option<AiError>,
    /// Pause before each chunk, to exercise cancellation and streaming UI.
    pub delay: Duration,
}

impl MockReply {
    pub fn text(text: &str) -> Self {
        Self {
            chunks: text.split_inclusive(' ').map(str::to_owned).collect(),
            ..Self::default()
        }
    }

    pub fn tool(name: &str, input: Value) -> Self {
        Self {
            tool_calls: vec![(name.into(), input)],
            ..Self::default()
        }
    }

    pub fn error(error: AiError) -> Self {
        Self {
            error: Some(error),
            ..Self::default()
        }
    }
}

#[derive(Default)]
pub struct MockProvider {
    replies: Mutex<VecDeque<MockReply>>,
    requests: Mutex<Vec<ChatRequest>>,
    /// Reply used once the script is exhausted; `None` echoes the last user message.
    fallback: Option<fn(&ChatRequest) -> MockReply>,
}

impl MockProvider {
    pub fn new(replies: impl IntoIterator<Item = MockReply>) -> Self {
        Self {
            replies: Mutex::new(replies.into_iter().collect()),
            ..Self::default()
        }
    }

    /// A provider that answers every request with `reply(request)`.
    pub fn responding(reply: fn(&ChatRequest) -> MockReply) -> Self {
        Self {
            fallback: Some(reply),
            ..Self::default()
        }
    }

    /// Requests received so far.
    pub fn requests(&self) -> Vec<ChatRequest> {
        self.requests.lock().map(|r| r.clone()).unwrap_or_default()
    }
}

impl Provider for MockProvider {
    fn id(&self) -> &str {
        "mock"
    }

    fn model(&self, role: ModelRole) -> String {
        match role {
            ModelRole::Chat => "mock-chat".into(),
            ModelRole::Completion => "mock-completion".into(),
        }
    }

    fn stream(
        &self,
        request: &ChatRequest,
        cancel: &CancelToken,
        sink: &mut dyn FnMut(StreamEvent),
    ) -> Result<ChatResponse, AiError> {
        if let Ok(mut requests) = self.requests.lock() {
            requests.push(request.clone());
        }
        let scripted = self.replies.lock().ok().and_then(|mut r| r.pop_front());
        let reply = scripted.unwrap_or_else(|| match self.fallback {
            Some(reply) => reply(request),
            None => MockReply::text(&format!(
                "Mock reply to: {}",
                request
                    .messages
                    .last()
                    .map(|m| m.text())
                    .unwrap_or_default()
            )),
        });
        let model = self.model(request.role);
        sink(StreamEvent::Started {
            model: model.clone(),
        });
        let mut text = String::new();
        for chunk in &reply.chunks {
            if !reply.delay.is_zero() {
                std::thread::sleep(reply.delay);
            }
            if cancel.is_cancelled() {
                return Err(AiError::Cancelled);
            }
            text.push_str(chunk);
            sink(StreamEvent::TextDelta(chunk.clone()));
        }
        if let Some(error) = reply.error {
            return Err(error);
        }
        let mut content = Vec::new();
        if !text.is_empty() {
            content.push(ContentBlock::Text { text });
        }
        for (index, (name, input)) in reply.tool_calls.iter().enumerate() {
            let id = format!("mock_tool_{index}");
            sink(StreamEvent::ToolUse {
                id: id.clone(),
                name: name.clone(),
                input: input.clone(),
            });
            content.push(ContentBlock::ToolUse {
                id,
                name: name.clone(),
                input: input.clone(),
            });
        }
        Ok(ChatResponse {
            model,
            stop_reason: if reply.tool_calls.is_empty() {
                StopReason::EndTurn
            } else {
                StopReason::ToolUse
            },
            content,
            ..ChatResponse::default()
        })
    }
}
