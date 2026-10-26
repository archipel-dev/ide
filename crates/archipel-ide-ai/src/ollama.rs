//! Local Ollama provider (`POST {base}/api/chat`, newline-delimited JSON stream).

use crate::settings::AiSettings;
use crate::transport::HttpTransport;
use crate::{
    AiError, CancelToken, ChatMessage, ChatRequest, ChatResponse, ContentBlock, ModelRole,
    Provider, Role, StopReason, StreamEvent,
};
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Read};
use std::sync::Arc;

pub struct OllamaProvider {
    transport: Arc<dyn HttpTransport>,
    settings: AiSettings,
}

impl OllamaProvider {
    pub fn new(settings: AiSettings, transport: Arc<dyn HttpTransport>) -> Self {
        Self {
            transport,
            settings,
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
            body["options"] = json!({"num_predict": max});
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

fn wire_messages(message: &ChatMessage) -> Vec<Value> {
    let text = message.text();
    match message.role {
        Role::User => {
            let mut out: Vec<Value> = message
                .content
                .iter()
                .filter_map(|block| match block {
                    ContentBlock::ToolResult { content, .. } => {
                        Some(json!({"role": "tool", "content": content}))
                    }
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
                    ContentBlock::ToolUse { name, input, .. } => {
                        Some(json!({"function": {"name": name, "arguments": input}}))
                    }
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

impl Provider for OllamaProvider {
    fn id(&self) -> &str {
        "ollama"
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
        let url = format!("{}/api/chat", self.settings.base_url());
        let headers = [("content-type".to_owned(), "application/json".to_owned())];
        let body = self
            .transport
            .post(&url, &headers, &self.body(request))
            .map_err(|error| match error {
                AiError::Network(message) => AiError::Network(format!(
                    "{message} (is Ollama running at {}? start it with `ollama serve`)",
                    self.settings.base_url()
                )),
                other => other,
            })?;
        read_stream(body, cancel, sink)
    }
}

pub fn read_stream(
    body: impl Read,
    cancel: &CancelToken,
    sink: &mut dyn FnMut(StreamEvent),
) -> Result<ChatResponse, AiError> {
    let mut response = ChatResponse::default();
    let mut text = String::new();
    let mut done = false;
    for line in BufReader::new(body).lines() {
        if cancel.is_cancelled() {
            return Err(AiError::Cancelled);
        }
        let line = line.map_err(|error| AiError::Network(error.to_string()))?;
        if line.trim().is_empty() {
            continue;
        }
        let data: Value = serde_json::from_str(&line)
            .map_err(|error| AiError::Protocol(format!("invalid chunk JSON: {error}")))?;
        if let Some(error) = data["error"].as_str() {
            return Err(AiError::Protocol(error.to_owned()));
        }
        if response.model.is_empty()
            && let Some(model) = data["model"].as_str()
        {
            response.model = model.to_owned();
            sink(StreamEvent::Started {
                model: model.to_owned(),
            });
        }
        let message = &data["message"];
        if let Some(piece) = message["content"]
            .as_str()
            .filter(|piece| !piece.is_empty())
        {
            text.push_str(piece);
            sink(StreamEvent::TextDelta(piece.to_owned()));
        }
        for call in message["tool_calls"].as_array().into_iter().flatten() {
            let id = format!("call_{}", response.content.len());
            let name = call["function"]["name"]
                .as_str()
                .unwrap_or_default()
                .to_owned();
            let input = call["function"]["arguments"].clone();
            let input = if input.is_object() {
                input
            } else {
                response
                    .invalid_tool_inputs
                    .push((id.clone(), input.to_string()));
                json!({})
            };
            sink(StreamEvent::ToolUse {
                id: id.clone(),
                name: name.clone(),
                input: input.clone(),
            });
            response
                .content
                .push(ContentBlock::ToolUse { id, name, input });
        }
        if data["done"].as_bool() == Some(true) {
            response.stop_reason = data["done_reason"]
                .as_str()
                .map(StopReason::parse)
                .unwrap_or_default();
            if response.content.iter().any(|b| b.kind() == "tool_use") {
                response.stop_reason = StopReason::ToolUse;
            }
            response.usage.input_tokens = data["prompt_eval_count"].as_u64().unwrap_or_default();
            response.usage.output_tokens = data["eval_count"].as_u64().unwrap_or_default();
            done = true;
            break;
        }
    }
    if cancel.is_cancelled() {
        return Err(AiError::Cancelled);
    }
    if !done {
        return Err(AiError::Network("the response stream ended early".into()));
    }
    if !text.is_empty() {
        response.content.insert(0, ContentBlock::Text { text });
    }
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn streams_ndjson_with_tool_calls() {
        let body = [
            json!({"model": "qwen", "message": {"role": "assistant", "content": "Sure"}, "done": false}),
            json!({"model": "qwen", "message": {"role": "assistant", "content": "", "tool_calls": [{"function": {"name": "read_file", "arguments": {"path": "a.rs"}}}]}, "done": false}),
            json!({"model": "qwen", "message": {"role": "assistant", "content": ""}, "done": true, "done_reason": "stop", "prompt_eval_count": 5, "eval_count": 7}),
        ]
        .iter()
        .map(|line| format!("{line}\n"))
        .collect::<String>();
        let response = read_stream(body.as_bytes(), &CancelToken::default(), &mut |_| {}).unwrap();
        assert_eq!(response.text(), "Sure");
        assert_eq!(response.stop_reason, StopReason::ToolUse);
        assert_eq!(response.usage.output_tokens, 7);
        assert_eq!(response.tool_uses().next().unwrap().2["path"], "a.rs");
        assert!(matches!(
            read_stream("{\"error\":\"model not found\"}\n".as_bytes(), &CancelToken::default(), &mut |_| {}),
            Err(AiError::Protocol(message)) if message == "model not found"
        ));
    }
}
