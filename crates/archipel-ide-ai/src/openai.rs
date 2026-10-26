//! OpenAI-compatible Chat Completions provider (`POST {base}/chat/completions`).
//!
//! Works with any server implementing the streaming Chat Completions wire format
//! (hosted gateways, vLLM, LM Studio, llama.cpp server, Ollama's `/v1`).

use crate::settings::AiSettings;
use crate::sse::SseParser;
use crate::transport::HttpTransport;
use crate::{
    AiError, CancelToken, ChatMessage, ChatRequest, ChatResponse, ContentBlock, ModelRole,
    Provider, Role, StopReason, StreamEvent,
};
use serde_json::{Value, json};
use std::io::Read;
use std::sync::Arc;

pub struct OpenAiProvider {
    transport: Arc<dyn HttpTransport>,
    settings: AiSettings,
    api_key: Option<String>,
}

impl OpenAiProvider {
    pub fn new(
        settings: AiSettings,
        api_key: Option<String>,
        transport: Arc<dyn HttpTransport>,
    ) -> Self {
        Self {
            transport,
            settings,
            api_key,
        }
    }

    pub fn body(&self, request: &ChatRequest) -> Value {
        let mut messages = Vec::new();
        if !request.system.is_empty() {
            messages.push(json!({"role": "system", "content": request.system}));
        }
        for message in &request.messages {
            messages.extend(wire_messages(message));
        }
        let mut body = json!({
            "model": self.model(request.role),
            "stream": true,
            "messages": messages,
        });
        if let Some(max) = request.max_tokens {
            body["max_tokens"] = json!(max);
        }
        if !request.tools.is_empty() {
            body["tools"] = request
                .tools
                .iter()
                .map(|tool| {
                    json!({"type": "function", "function": {
                        "name": tool.name,
                        "description": tool.description,
                        "parameters": tool.input_schema,
                    }})
                })
                .collect();
        }
        body
    }
}

/// One conversation message maps to one or more Chat Completions messages
/// (tool results become separate `tool` messages).
fn wire_messages(message: &ChatMessage) -> Vec<Value> {
    let text = message.text();
    match message.role {
        Role::User => {
            let mut out: Vec<Value> = message
                .content
                .iter()
                .filter_map(|block| match block {
                    ContentBlock::ToolResult {
                        tool_use_id,
                        content,
                        ..
                    } => Some(
                        json!({"role": "tool", "tool_call_id": tool_use_id, "content": content}),
                    ),
                    _ => None,
                })
                .collect();
            if !text.is_empty() {
                out.push(json!({"role": "user", "content": text}));
            }
            out
        }
        Role::Assistant => {
            let calls: Vec<Value> = message
                .content
                .iter()
                .filter_map(|block| match block {
                    ContentBlock::ToolUse { id, name, input } => Some(json!({
                        "id": id,
                        "type": "function",
                        "function": {"name": name, "arguments": input.to_string()},
                    })),
                    _ => None,
                })
                .collect();
            let mut value = json!({"role": "assistant", "content": text});
            if !calls.is_empty() {
                value["tool_calls"] = Value::Array(calls);
            }
            vec![value]
        }
    }
}

impl Provider for OpenAiProvider {
    fn id(&self) -> &str {
        "openai"
    }

    fn model(&self, role: ModelRole) -> String {
        match role {
            ModelRole::Chat => self.settings.chat_model.clone(),
            ModelRole::Completion => self.settings.completion_model.clone(),
        }
    }

    fn stream(
        &self,
        request: &ChatRequest,
        cancel: &CancelToken,
        sink: &mut dyn FnMut(StreamEvent),
    ) -> Result<ChatResponse, AiError> {
        let mut headers = vec![("content-type".to_owned(), "application/json".to_owned())];
        if let Some(key) = &self.api_key {
            headers.push(("authorization".into(), format!("Bearer {key}")));
        }
        let url = format!("{}/chat/completions", self.settings.base_url());
        let body = self.transport.post(&url, &headers, &self.body(request))?;
        read_stream(body, cancel, sink)
    }
}

#[derive(Default)]
struct ToolCall {
    id: String,
    name: String,
    arguments: String,
}

pub fn read_stream(
    mut body: impl Read,
    cancel: &CancelToken,
    sink: &mut dyn FnMut(StreamEvent),
) -> Result<ChatResponse, AiError> {
    let mut parser = SseParser::default();
    let mut response = ChatResponse::default();
    let mut text = String::new();
    let mut calls: Vec<ToolCall> = Vec::new();
    let mut done = false;
    let mut buffer = [0u8; 8192];
    while !done {
        if cancel.is_cancelled() {
            return Err(AiError::Cancelled);
        }
        let read = body
            .read(&mut buffer)
            .map_err(|error| AiError::Network(error.to_string()))?;
        let events: Vec<_> = if read == 0 {
            parser.finish().into_iter().collect()
        } else {
            parser.push(&buffer[..read])
        };
        for event in events {
            if event.data.trim() == "[DONE]" {
                done = true;
                break;
            }
            let data: Value = serde_json::from_str(&event.data)
                .map_err(|error| AiError::Protocol(format!("invalid chunk JSON: {error}")))?;
            if let Some(message) = data.pointer("/error/message").and_then(Value::as_str) {
                return Err(AiError::Protocol(message.to_owned()));
            }
            if response.model.is_empty()
                && let Some(model) = data["model"].as_str()
            {
                response.model = model.to_owned();
                sink(StreamEvent::Started {
                    model: model.to_owned(),
                });
            }
            if let Some(usage) = data.get("usage").filter(|usage| usage.is_object()) {
                response.usage.input_tokens = usage["prompt_tokens"].as_u64().unwrap_or_default();
                response.usage.output_tokens =
                    usage["completion_tokens"].as_u64().unwrap_or_default();
            }
            let Some(choice) = data["choices"].get(0) else {
                continue;
            };
            let delta = &choice["delta"];
            if let Some(piece) = delta["content"].as_str().filter(|piece| !piece.is_empty()) {
                text.push_str(piece);
                sink(StreamEvent::TextDelta(piece.to_owned()));
            }
            for call in delta["tool_calls"].as_array().into_iter().flatten() {
                let index = call["index"].as_u64().unwrap_or(0) as usize;
                if calls.len() <= index {
                    calls.resize_with(index + 1, ToolCall::default);
                }
                let target = &mut calls[index];
                if let Some(id) = call["id"].as_str() {
                    target.id = id.to_owned();
                }
                if let Some(name) = call["function"]["name"].as_str() {
                    target.name.push_str(name);
                }
                if let Some(arguments) = call["function"]["arguments"].as_str() {
                    target.arguments.push_str(arguments);
                }
            }
            if let Some(reason) = choice["finish_reason"].as_str() {
                response.stop_reason = StopReason::parse(reason);
            }
        }
        if read == 0 {
            break;
        }
    }
    if !done {
        return Err(AiError::Network("the response stream ended early".into()));
    }
    if !text.is_empty() {
        response.content.push(ContentBlock::Text { text });
    }
    for (index, call) in calls.into_iter().enumerate() {
        let id = if call.id.is_empty() {
            format!("call_{index}")
        } else {
            call.id
        };
        let source = if call.arguments.trim().is_empty() {
            "{}"
        } else {
            &call.arguments
        };
        let input = match serde_json::from_str::<Value>(source) {
            Ok(input) if input.is_object() => input,
            _ => {
                response
                    .invalid_tool_inputs
                    .push((id.clone(), call.arguments.clone()));
                json!({})
            }
        };
        sink(StreamEvent::ToolUse {
            id: id.clone(),
            name: call.name.clone(),
            input: input.clone(),
        });
        response.content.push(ContentBlock::ToolUse {
            id,
            name: call.name,
            input,
        });
    }
    if response.stop_reason == StopReason::Refusal {
        return Err(AiError::Refusal(None));
    }
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ToolSpec;

    #[test]
    fn maps_tool_history_and_tools() {
        let settings = AiSettings {
            provider: crate::ProviderKind::OpenAi,
            chat_model: "local-model".into(),
            ..AiSettings::default()
        };
        let provider = OpenAiProvider::new(
            settings,
            None,
            Arc::new(crate::transport::UreqTransport::default()),
        );
        let mut request = ChatRequest::chat(
            "sys",
            vec![
                ChatMessage::user("read it"),
                ChatMessage {
                    role: Role::Assistant,
                    content: vec![ContentBlock::ToolUse {
                        id: "c1".into(),
                        name: "read_file".into(),
                        input: json!({"path": "a"}),
                    }],
                },
                ChatMessage {
                    role: Role::User,
                    content: vec![ContentBlock::ToolResult {
                        tool_use_id: "c1".into(),
                        content: "text".into(),
                        is_error: false,
                    }],
                },
            ],
        );
        request.tools.push(ToolSpec {
            name: "read_file".into(),
            description: "d".into(),
            input_schema: json!({"type": "object"}),
        });
        let body = provider.body(&request);
        assert_eq!(body["model"], "local-model");
        let messages = body["messages"].as_array().unwrap();
        assert_eq!(messages[0]["role"], "system");
        assert_eq!(
            messages[2]["tool_calls"][0]["function"]["arguments"],
            "{\"path\":\"a\"}"
        );
        assert_eq!(
            messages[3],
            json!({"role": "tool", "tool_call_id": "c1", "content": "text"})
        );
        assert_eq!(body["tools"][0]["function"]["name"], "read_file");
    }

    #[test]
    fn streams_text_and_split_tool_calls() {
        let chunks = [
            json!({"model": "m1", "choices": [{"delta": {"role": "assistant", "content": "Hi"}}]}),
            json!({"choices": [{"delta": {"content": " there"}}]}),
            json!({"choices": [{"delta": {"tool_calls": [{"index": 0, "id": "c9", "function": {"name": "read_file", "arguments": "{\"pa"}}]}}]}),
            json!({"choices": [{"delta": {"tool_calls": [{"index": 0, "function": {"arguments": "th\":\"x\"}"}}]}, "finish_reason": "tool_calls"}]}),
        ];
        let mut body: String = chunks.iter().map(|c| format!("data: {c}\n\n")).collect();
        body.push_str("data: [DONE]\n\n");
        let mut events = Vec::new();
        let response = read_stream(body.as_bytes(), &CancelToken::default(), &mut |e| {
            events.push(e)
        })
        .unwrap();
        assert_eq!(response.text(), "Hi there");
        assert_eq!(response.stop_reason, StopReason::ToolUse);
        let (id, name, input) = response.tool_uses().next().unwrap();
        assert_eq!(
            (id, name, input["path"].as_str()),
            ("c9", "read_file", Some("x"))
        );
        assert_eq!(events[0], StreamEvent::Started { model: "m1".into() });
        assert!(matches!(
            read_stream(
                "data: {\"choices\":[]}\n\n".as_bytes(),
                &CancelToken::default(),
                &mut |_| {}
            ),
            Err(AiError::Network(_))
        ));
    }
}
