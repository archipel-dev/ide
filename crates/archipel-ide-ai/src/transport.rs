//! HTTP transport behind a trait so providers can be tested without network.

use crate::AiError;
use serde_json::Value;
use std::io::Read;
use std::time::Duration;

pub type Body = Box<dyn Read + Send>;

pub trait HttpTransport: Send + Sync {
    /// POSTs `body` as JSON and returns the streaming response body of a 2xx reply.
    /// Non-2xx replies map to structured [`AiError`]s.
    fn post(&self, url: &str, headers: &[(String, String)], body: &Value) -> Result<Body, AiError>;
}

/// Blocking HTTPS transport (rustls, system roots, `HTTPS_PROXY` honoured).
pub struct UreqTransport {
    agent: ureq::Agent,
}

impl Default for UreqTransport {
    fn default() -> Self {
        Self {
            agent: ureq::AgentBuilder::new()
                .timeout_connect(Duration::from_secs(15))
                // Streams send periodic pings; a long silence means a dead connection.
                .timeout_read(Duration::from_secs(300))
                .try_proxy_from_env(true)
                .build(),
        }
    }
}

impl HttpTransport for UreqTransport {
    fn post(&self, url: &str, headers: &[(String, String)], body: &Value) -> Result<Body, AiError> {
        let mut request = self.agent.post(url);
        for (name, value) in headers {
            request = request.set(name, value);
        }
        match request.send_json(body) {
            Ok(response) => Ok(Box::new(response.into_reader())),
            Err(ureq::Error::Status(status, response)) => {
                let retry_after = response
                    .header("retry-after")
                    .and_then(|value| value.trim().parse().ok());
                let text = response.into_string().unwrap_or_default();
                Err(status_error(status, &text, retry_after))
            }
            Err(error) => Err(AiError::Network(error.to_string())),
        }
    }
}

/// Maps an HTTP error status and body (any provider's JSON error shape) to an error.
pub fn status_error(status: u16, body: &str, retry_after: Option<u64>) -> AiError {
    let json: Option<Value> = serde_json::from_str(body).ok();
    let message = json
        .as_ref()
        .and_then(|json| {
            json.pointer("/error/message")
                .or_else(|| json.get("error"))
                .or_else(|| json.get("message"))
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
        .unwrap_or_else(|| body.chars().take(300).collect());
    match status {
        401 | 403 => AiError::Auth(message),
        429 => AiError::RateLimited {
            retry_after,
            message,
        },
        529 | 503 => AiError::Overloaded(message),
        _ => AiError::Http { status, message },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_error_bodies_of_each_provider() {
        let anthropic = r#"{"type":"error","error":{"type":"authentication_error","message":"invalid x-api-key"}}"#;
        assert_eq!(
            status_error(401, anthropic, None),
            AiError::Auth("invalid x-api-key".into())
        );
        let ollama = r#"{"error":"model 'x' not found"}"#;
        assert_eq!(
            status_error(404, ollama, None),
            AiError::Http {
                status: 404,
                message: "model 'x' not found".into()
            }
        );
        assert!(matches!(
            status_error(429, "slow down", Some(7)),
            AiError::RateLimited {
                retry_after: Some(7),
                ..
            }
        ));
        assert!(matches!(
            status_error(529, "{}", None),
            AiError::Overloaded(_)
        ));
    }
}
