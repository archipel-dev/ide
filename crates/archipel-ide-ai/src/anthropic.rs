//! Anthropic Messages API provider (`POST /v1/messages`, streamed over SSE).

use crate::settings::{ANTHROPIC_DEFAULT_URL, AiSettings};
use crate::sse::{SseEvent, SseParser};
use crate::transport::HttpTransport;
use crate::{
    AiError, CancelToken, ChatMessage, ChatRequest, ChatResponse, ContentBlock, ModelRole,
    Provider, Role, StopReason, StreamEvent,
};
use serde_json::{Map, Value, json};
use std::io::Read;
use std::sync::Arc;

const API_VERSION: &str = "2023-06-01";
/// Server-side refusal fallback (`fallbacks: "default"`).
const FALLBACK_BETA: &str = "server-side-fallback-2026-07-01";
/// Models that accept `fallbacks: "default"` on the first-party API.
const FALLBACK_MODELS: [&str; 4] = [
    "claude-fable-5-1",
    "claude-opus-5-5",
    "claude-opus-5",
    "claude-sonnet-5-5",
];

pub struct AnthropicProvider {
    transport: Arc<dyn HttpTransport>,
    settings: AiSettings,
    api_key: String,
}

impl AnthropicProvider {
    pub fn new(settings: AiSettings, api_key: String, transport: Arc<dyn HttpTransport>) -> Self {
        Self {
            transport,
            settings,
            api_key,
        }
    }

    fn uses_fallbacks(&self, model: &str) -> bool {
        self.settings.refusal_fallback
            && self.settings.base_url() == ANTHROPIC_DEFAULT_URL
            && FALLBACK_MODELS.contains(&model)
    }

    /// Request headers (the API key is included; never log them).
    pub fn headers(&self, model: &str) -> Vec<(String, String)> {
        let mut headers = vec![
            ("x-api-key".into(), self.api_key.clone()),
            ("anthropic-version".into(), API_VERSION.into()),
            ("content-type".into(), "application/json".into()),
        ];
        if self.uses_fallbacks(model) {
            headers.push(("anthropic-beta".into(), FALLBACK_BETA.into()));
        }
        headers
    }

    pub fn body(&self, request: &ChatRequest) -> Value {
        let model = self.model(request.role);
        let mut body = Map::new();
        body.insert("model".into(), json!(model));
        body.insert(
            "max_tokens".into(),
            json!(
                request
                    .max_tokens
                    .unwrap_or(self.settings.max_output_tokens)
            ),
        );
        body.insert("stream".into(), json!(true));
        // Automatic prompt caching: the stable prefix (tools, system, earlier turns)
        // is cached across chat turns and agent iterations.
        body.insert("cache_control".into(), json!({"type": "ephemeral"}));
        if !request.system.is_empty() {
            body.insert(
                "system".into(),
                json!([{"type": "text", "text": request.system}]),
            );
        }
        body.insert(
            "messages".into(),
            Value::Array(request.messages.iter().map(wire_message).collect()),
        );
        if !request.tools.is_empty() {
            let tools = request
                .tools
                .iter()
                .map(|tool| {
                    json!({
                        "name": tool.name,
                        "description": tool.description,
                        "input_schema": tool.input_schema,
                        // Large inputs (file bodies) stream as generated; inputs are
                        // parsed strictly and validated by the caller.
                        "eager_input_streaming": true,
                    })
                })
                .collect();
            body.insert("tools".into(), Value::Array(tools));
        }
        let effort = match request.role {
            ModelRole::Chat => &self.settings.chat_effort,
            ModelRole::Completion => &self.settings.completion_effort,
        };
        if let Some(effort) = effort.as_deref().filter(|effort| !effort.is_empty()) {
            body.insert("output_config".into(), json!({"effort": effort}));
        }
        if self.uses_fallbacks(&model) {
            body.insert("fallbacks".into(), json!("default"));
        }
        Value::Object(body)
    }
}

fn wire_message(message: &ChatMessage) -> Value {
    let role = match message.role {
        Role::User => "user",
        Role::Assistant => "assistant",
    };
    let content: Vec<Value> = message
        .content
        .iter()
        .map(|block| match block {
            ContentBlock::Text { text } => json!({"type": "text", "text": text}),
            ContentBlock::ToolUse { id, name, input } => {
                json!({"type": "tool_use", "id": id, "name": name, "input": input})
            }
            ContentBlock::ToolResult {
                tool_use_id,
                content,
                is_error,
            } => {
                let mut value =
                    json!({"type": "tool_result", "tool_use_id": tool_use_id, "content": content});
                if *is_error {
                    value["is_error"] = json!(true);
                }
                value
            }
            ContentBlock::Raw { value } => value.clone(),
        })
        .collect();
    json!({"role": role, "content": content})
}

impl Provider for AnthropicProvider {
    fn id(&self) -> &str {
        "anthropic"
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
        let model = self.model(request.role);
        let url = format!("{}/v1/messages", self.settings.base_url());
        let body = self
            .transport
            .post(&url, &self.headers(&model), &self.body(request))?;
        read_stream(body, cancel, sink)
    }
}

/// A content block while it streams.
enum Partial {
    Text(String),
    Tool {
        id: String,
        name: String,
        json: String,
    },
    Raw(Value),
}

#[derive(Default)]
struct Assembler {
    response: ChatResponse,
    blocks: Vec<Option<Partial>>,
    refusal_category: Option<String>,
    stopped: bool,
}

impl Assembler {
    fn slot(&mut self, index: usize) -> &mut Option<Partial> {
        if self.blocks.len() <= index {
            self.blocks.resize_with(index + 1, || None);
        }
        &mut self.blocks[index]
    }

    fn event(
        &mut self,
        event: &SseEvent,
        sink: &mut dyn FnMut(StreamEvent),
    ) -> Result<(), AiError> {
        let data: Value = serde_json::from_str(&event.data)
            .map_err(|error| AiError::Protocol(format!("invalid event JSON: {error}")))?;
        let kind = data
            .get("type")
            .and_then(Value::as_str)
            .or(event.event.as_deref())
            .unwrap_or_default();
        let index = data
            .get("index")
            .and_then(Value::as_u64)
            .map(|index| index as usize);
        match kind {
            "message_start" => {
                let message = &data["message"];
                self.response.model = message["model"].as_str().unwrap_or_default().to_owned();
                self.usage(&message["usage"]);
                sink(StreamEvent::Started {
                    model: self.response.model.clone(),
                });
            }
            "content_block_start" => {
                let index = index.ok_or_else(|| AiError::Protocol("block without index".into()))?;
                let block = &data["content_block"];
                let partial = match block["type"].as_str().unwrap_or_default() {
                    "text" => Partial::Text(block["text"].as_str().unwrap_or_default().to_owned()),
                    "tool_use" => Partial::Tool {
                        id: block["id"].as_str().unwrap_or_default().to_owned(),
                        name: block["name"].as_str().unwrap_or_default().to_owned(),
                        json: String::new(),
                    },
                    "thinking" | "redacted_thinking" => {
                        sink(StreamEvent::Thinking);
                        Partial::Raw(block.clone())
                    }
                    "fallback" => {
                        let model = |side: &str| {
                            block[side]["model"]
                                .as_str()
                                .or_else(|| block[side].as_str())
                                .unwrap_or_default()
                                .to_owned()
                        };
                        sink(StreamEvent::Fallback {
                            from: model("from"),
                            to: model("to"),
                        });
                        Partial::Raw(block.clone())
                    }
                    _ => Partial::Raw(block.clone()),
                };
                *self.slot(index) = Some(partial);
            }
            "content_block_delta" => {
                let index = index.ok_or_else(|| AiError::Protocol("delta without index".into()))?;
                let delta = &data["delta"];
                let text = |field: &str| delta[field].as_str().unwrap_or_default();
                match (self.slot(index).as_mut(), delta["type"].as_str()) {
                    (Some(Partial::Text(buffer)), Some("text_delta")) => {
                        buffer.push_str(text("text"));
                        sink(StreamEvent::TextDelta(text("text").to_owned()));
                    }
                    (Some(Partial::Tool { json, .. }), Some("input_json_delta")) => {
                        json.push_str(text("partial_json"));
                    }
                    (Some(Partial::Raw(value)), Some("thinking_delta")) => {
                        let current = value["thinking"].as_str().unwrap_or_default().to_owned();
                        value["thinking"] = json!(current + text("thinking"));
                    }
                    (Some(Partial::Raw(value)), Some("signature_delta")) => {
                        value["signature"] = json!(text("signature"));
                    }
                    _ => {}
                }
            }
            "content_block_stop" => {
                let index = index.ok_or_else(|| AiError::Protocol("stop without index".into()))?;
                let block = match self.slot(index).take() {
                    Some(Partial::Text(text)) => ContentBlock::Text { text },
                    Some(Partial::Tool { id, name, json }) => {
                        // Inputs stream unvalidated: parse strictly, never repair.
                        let source = if json.trim().is_empty() {
                            "{}"
                        } else {
                            json.as_str()
                        };
                        let input = match serde_json::from_str::<Value>(source) {
                            Ok(input) if input.is_object() => input,
                            _ => {
                                self.response.invalid_tool_inputs.push((id.clone(), json));
                                json!({})
                            }
                        };
                        sink(StreamEvent::ToolUse {
                            id: id.clone(),
                            name: name.clone(),
                            input: input.clone(),
                        });
                        ContentBlock::ToolUse { id, name, input }
                    }
                    Some(Partial::Raw(value)) => ContentBlock::Raw { value },
                    None => return Ok(()),
                };
                self.response.content.push(block);
            }
            "message_delta" => {
                let delta = &data["delta"];
                if let Some(reason) = delta["stop_reason"].as_str() {
                    self.response.stop_reason = StopReason::parse(reason);
                }
                let details = if delta["stop_details"].is_object() {
                    &delta["stop_details"]
                } else {
                    &data["stop_details"]
                };
                self.refusal_category = details["category"].as_str().map(str::to_owned);
                if let Some(model) = data["message"]["model"].as_str() {
                    self.response.model = model.to_owned();
                }
                self.usage(&data["usage"]);
            }
            "message_stop" => self.stopped = true,
            "error" => {
                let error = &data["error"];
                let message = error["message"]
                    .as_str()
                    .unwrap_or("stream error")
                    .to_owned();
                return Err(match error["type"].as_str() {
                    Some("overloaded_error") => AiError::Overloaded(message),
                    Some("rate_limit_error") => AiError::RateLimited {
                        retry_after: None,
                        message,
                    },
                    Some("authentication_error" | "permission_error") => AiError::Auth(message),
                    _ => AiError::Protocol(message),
                });
            }
            _ => {}
        }
        Ok(())
    }

    fn usage(&mut self, usage: &Value) {
        let field = |name: &str| usage[name].as_u64();
        let target = &mut self.response.usage;
        if let Some(value) = field("input_tokens") {
            target.input_tokens = value;
        }
        if let Some(value) = field("output_tokens") {
            target.output_tokens = value;
        }
        if let Some(value) = field("cache_read_input_tokens") {
            target.cache_read_input_tokens = value;
        }
        if let Some(value) = field("cache_creation_input_tokens") {
            target.cache_creation_input_tokens = value;
        }
    }
}

/// Parses an Anthropic SSE body into a response, honouring cancellation between chunks.
pub fn read_stream(
    mut body: impl Read,
    cancel: &CancelToken,
    sink: &mut dyn FnMut(StreamEvent),
) -> Result<ChatResponse, AiError> {
    let mut parser = SseParser::default();
    let mut assembler = Assembler::default();
    let mut buffer = [0u8; 8192];
    loop {
        if cancel.is_cancelled() {
            return Err(AiError::Cancelled);
        }
        let read = body
            .read(&mut buffer)
            .map_err(|error| AiError::Network(error.to_string()))?;
        let events = if read == 0 {
            parser.finish().into_iter().collect()
        } else {
            parser.push(&buffer[..read])
        };
        for event in events {
            assembler.event(&event, sink)?;
        }
        if read == 0 || assembler.stopped {
            break;
        }
    }
    if cancel.is_cancelled() {
        return Err(AiError::Cancelled);
    }
    if !assembler.stopped {
        return Err(AiError::Network("the response stream ended early".into()));
    }
    if assembler.response.stop_reason == StopReason::Refusal {
        return Err(AiError::Refusal(assembler.refusal_category));
    }
    Ok(assembler.response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ToolSpec;
    use crate::transport::Body;
    use std::sync::Mutex;

    /// `(url, headers, body)` of a recorded request.
    type Seen = (String, Vec<(String, String)>, Value);

    /// Replays a canned SSE body and records the request.
    pub(crate) struct CannedTransport {
        pub body: String,
        pub seen: Mutex<Vec<Seen>>,
    }

    impl HttpTransport for CannedTransport {
        fn post(
            &self,
            url: &str,
            headers: &[(String, String)],
            body: &Value,
        ) -> Result<Body, AiError> {
            self.seen
                .lock()
                .unwrap()
                .push((url.into(), headers.to_vec(), body.clone()));
            Ok(Box::new(std::io::Cursor::new(
                self.body.clone().into_bytes(),
            )))
        }
    }

    fn sse(events: &[Value]) -> String {
        events
            .iter()
            .map(|event| {
                format!(
                    "event: {}\ndata: {}\n\n",
                    event["type"].as_str().unwrap(),
                    event
                )
            })
            .collect()
    }

    fn provider(body: String) -> (AnthropicProvider, Arc<CannedTransport>) {
        let transport = Arc::new(CannedTransport {
            body,
            seen: Mutex::default(),
        });
        let settings = AiSettings {
            enabled: true,
            ..AiSettings::default()
        };
        (
            AnthropicProvider::new(settings, "sk-test".into(), transport.clone()),
            transport,
        )
    }

    #[test]
    fn request_uses_latest_models_caching_effort_and_fallbacks() {
        let (provider, _) = provider(String::new());
        let mut request = ChatRequest::chat("Be brief.", vec![ChatMessage::user("hi")]);
        request.tools.push(ToolSpec {
            name: "read_file".into(),
            description: "Read a file".into(),
            input_schema: json!({"type": "object"}),
        });
        let body = provider.body(&request);
        assert_eq!(body["model"], "claude-opus-5-5");
        assert_eq!(body["stream"], true);
        assert_eq!(body["cache_control"]["type"], "ephemeral");
        assert_eq!(body["system"][0]["text"], "Be brief.");
        assert_eq!(body["output_config"]["effort"], "medium");
        assert_eq!(body["fallbacks"], "default");
        assert_eq!(body["tools"][0]["eager_input_streaming"], true);
        assert!(body.get("thinking").is_none());
        let headers = provider.headers("claude-opus-5-5");
        assert!(headers.contains(&("anthropic-beta".into(), FALLBACK_BETA.into())));
        assert!(headers.contains(&("x-api-key".into(), "sk-test".into())));

        let mut completion = ChatRequest::chat("", vec![ChatMessage::user("x")]);
        completion.role = ModelRole::Completion;
        completion.max_tokens = Some(128);
        let body = provider.body(&completion);
        assert_eq!(body["model"], "claude-haiku-5-5");
        assert_eq!(body["max_tokens"], 128);
        assert_eq!(body["output_config"]["effort"], "low");
        assert!(body.get("fallbacks").is_none() && body.get("system").is_none());
        assert!(
            provider
                .headers("claude-haiku-5-5")
                .iter()
                .all(|(n, _)| n != "anthropic-beta")
        );
    }

    #[test]
    fn streams_text_thinking_and_tool_use() {
        let body = sse(&[
            json!({"type": "message_start", "message": {"model": "claude-opus-5-5", "usage": {"input_tokens": 12, "cache_read_input_tokens": 10}}}),
            json!({"type": "content_block_start", "index": 0, "content_block": {"type": "thinking", "thinking": ""}}),
            json!({"type": "content_block_delta", "index": 0, "delta": {"type": "signature_delta", "signature": "sig"}}),
            json!({"type": "content_block_stop", "index": 0}),
            json!({"type": "ping"}),
            json!({"type": "content_block_start", "index": 1, "content_block": {"type": "text", "text": ""}}),
            json!({"type": "content_block_delta", "index": 1, "delta": {"type": "text_delta", "text": "Let me "}}),
            json!({"type": "content_block_delta", "index": 1, "delta": {"type": "text_delta", "text": "look."}}),
            json!({"type": "content_block_stop", "index": 1}),
            json!({"type": "content_block_start", "index": 2, "content_block": {"type": "tool_use", "id": "toolu_1", "name": "read_file", "input": {}}}),
            json!({"type": "content_block_delta", "index": 2, "delta": {"type": "input_json_delta", "partial_json": "{\"path\": \"src/"}}),
            json!({"type": "content_block_delta", "index": 2, "delta": {"type": "input_json_delta", "partial_json": "main.rs\"}"}}),
            json!({"type": "content_block_stop", "index": 2}),
            json!({"type": "message_delta", "delta": {"stop_reason": "tool_use"}, "usage": {"output_tokens": 40}}),
            json!({"type": "message_stop"}),
        ]);
        let (provider, transport) = provider(body);
        let mut events = Vec::new();
        let response = provider
            .stream(
                &ChatRequest::chat("", vec![ChatMessage::user("read main")]),
                &CancelToken::default(),
                &mut |event| events.push(event),
            )
            .unwrap();
        assert_eq!(
            transport.seen.lock().unwrap()[0].0,
            "https://api.anthropic.com/v1/messages"
        );
        assert_eq!(response.text(), "Let me look.");
        assert_eq!(response.stop_reason, StopReason::ToolUse);
        assert_eq!(response.usage.output_tokens, 40);
        assert_eq!(response.usage.cache_read_input_tokens, 10);
        let tools: Vec<_> = response.tool_uses().collect();
        assert_eq!(tools[0].2["path"], "src/main.rs");
        assert_eq!(response.content[0].kind(), "thinking");
        // The thinking block keeps its signature so it can be echoed back.
        let history = wire_message(&ChatMessage {
            role: Role::Assistant,
            content: response.history_content(),
        });
        assert_eq!(history["content"][0]["signature"], "sig");
        assert_eq!(
            events,
            vec![
                StreamEvent::Started {
                    model: "claude-opus-5-5".into()
                },
                StreamEvent::Thinking,
                StreamEvent::TextDelta("Let me ".into()),
                StreamEvent::TextDelta("look.".into()),
                StreamEvent::ToolUse {
                    id: "toolu_1".into(),
                    name: "read_file".into(),
                    input: json!({"path": "src/main.rs"})
                },
            ]
        );
    }

    #[test]
    fn invalid_streamed_tool_input_is_reported_not_repaired() {
        let body = sse(&[
            json!({"type": "message_start", "message": {"model": "m", "usage": {}}}),
            json!({"type": "content_block_start", "index": 0, "content_block": {"type": "tool_use", "id": "t", "name": "write_file"}}),
            json!({"type": "content_block_delta", "index": 0, "delta": {"type": "input_json_delta", "partial_json": "{\"path\": \"a"}}),
            json!({"type": "content_block_stop", "index": 0}),
            json!({"type": "message_delta", "delta": {"stop_reason": "max_tokens"}, "usage": {}}),
            json!({"type": "message_stop"}),
        ]);
        let response = read_stream(body.as_bytes(), &CancelToken::default(), &mut |_| {}).unwrap();
        assert_eq!(response.stop_reason, StopReason::MaxTokens);
        assert_eq!(
            response.invalid_tool_inputs,
            vec![("t".into(), "{\"path\": \"a".into())]
        );
    }

    #[test]
    fn refusals_errors_truncation_and_cancellation_are_structured() {
        let refusal = sse(&[
            json!({"type": "message_start", "message": {"model": "m", "usage": {}}}),
            json!({"type": "message_delta", "delta": {"stop_reason": "refusal", "stop_details": {"type": "refusal", "category": "cyber"}}, "usage": {}}),
            json!({"type": "message_stop"}),
        ]);
        assert_eq!(
            read_stream(refusal.as_bytes(), &CancelToken::default(), &mut |_| {}),
            Err(AiError::Refusal(Some("cyber".into())))
        );
        let overloaded = sse(&[
            json!({"type": "error", "error": {"type": "overloaded_error", "message": "Overloaded"}}),
        ]);
        assert!(matches!(
            read_stream(overloaded.as_bytes(), &CancelToken::default(), &mut |_| {}),
            Err(AiError::Overloaded(_))
        ));
        let truncated =
            sse(&[json!({"type": "message_start", "message": {"model": "m", "usage": {}}})]);
        assert!(matches!(
            read_stream(truncated.as_bytes(), &CancelToken::default(), &mut |_| {}),
            Err(AiError::Network(_))
        ));
        let cancel = CancelToken::default();
        cancel.cancel();
        assert_eq!(
            read_stream(truncated.as_bytes(), &cancel, &mut |_| {}),
            Err(AiError::Cancelled)
        );
        assert!(matches!(
            read_stream(
                "data: {nope\n\n".as_bytes(),
                &CancelToken::default(),
                &mut |_| {}
            ),
            Err(AiError::Protocol(_))
        ));
    }

    #[test]
    fn reports_server_side_fallback() {
        let body = sse(&[
            json!({"type": "message_start", "message": {"model": "claude-opus-4-8", "usage": {}}}),
            json!({"type": "content_block_start", "index": 0, "content_block": {"type": "fallback", "from": {"model": "claude-opus-5-5"}, "to": {"model": "claude-opus-4-8"}}}),
            json!({"type": "content_block_stop", "index": 0}),
            json!({"type": "content_block_start", "index": 1, "content_block": {"type": "text", "text": "ok"}}),
            json!({"type": "content_block_stop", "index": 1}),
            json!({"type": "message_delta", "delta": {"stop_reason": "end_turn"}, "usage": {}}),
            json!({"type": "message_stop"}),
        ]);
        let mut events = Vec::new();
        let response = read_stream(body.as_bytes(), &CancelToken::default(), &mut |e| {
            events.push(e)
        })
        .unwrap();
        assert_eq!(response.text(), "ok");
        assert!(events.contains(&StreamEvent::Fallback {
            from: "claude-opus-5-5".into(),
            to: "claude-opus-4-8".into()
        }));
    }
}
