//! AI engine: provider abstraction, streaming providers (Anthropic, OpenAI-compatible,
//! Ollama, mock), background request service, chat model and edit previews.
//!
//! The crate is UI-independent. Nothing here performs network I/O unless a caller
//! builds a provider from settings the user explicitly enabled.

pub mod actions;
pub mod agent;
pub mod anthropic;
pub mod chat;
pub mod commit;
pub mod completion;
pub mod diff;
pub mod markdown;
pub mod mock;
pub mod ollama;
pub mod openai;
pub mod service;
pub mod settings;
pub mod shell;
pub mod sse;
pub mod transport;
mod types;

pub use mock::{MockProvider, MockReply};
pub use service::{AiService, AiUpdate};
pub use settings::{AiSettings, FileSecretStore, ProjectPolicy, ProviderKind, SecretStore};
pub use types::*;

use std::sync::Arc;
use transport::{HttpTransport, UreqTransport};

/// Builds the configured provider, or explains what configuration is missing.
pub fn build_provider(
    settings: &AiSettings,
    store: &dyn SecretStore,
    env: &dyn Fn(&str) -> Option<String>,
) -> Result<Arc<dyn Provider>, AiError> {
    build_provider_with(settings, store, env, Arc::new(UreqTransport::default()))
}

pub fn build_provider_with(
    settings: &AiSettings,
    store: &dyn SecretStore,
    env: &dyn Fn(&str) -> Option<String>,
    transport: Arc<dyn HttpTransport>,
) -> Result<Arc<dyn Provider>, AiError> {
    if !settings.enabled {
        return Err(AiError::NotConfigured(
            "enable a provider with “AI: Configure Provider”".into(),
        ));
    }
    if settings.chat_model.trim().is_empty() {
        return Err(AiError::NotConfigured(format!(
            "set a chat model for {}",
            settings.provider.label()
        )));
    }
    let key = settings::resolve_api_key(settings.provider, store, env);
    Ok(match settings.provider {
        ProviderKind::Anthropic => {
            let key = key.ok_or_else(|| AiError::MissingApiKey {
                env: "ANTHROPIC_API_KEY".into(),
            })?;
            Arc::new(anthropic::AnthropicProvider::new(
                settings.clone(),
                key,
                transport,
            ))
        }
        ProviderKind::OpenAi => Arc::new(openai::OpenAiProvider::new(
            settings.clone(),
            key,
            transport,
        )),
        ProviderKind::Ollama => Arc::new(ollama::OllamaProvider::new(settings.clone(), transport)),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    #[derive(Default)]
    struct MemoryStore(Mutex<HashMap<String, String>>);

    impl SecretStore for MemoryStore {
        fn get(&self, name: &str) -> Option<String> {
            self.0.lock().unwrap().get(name).cloned()
        }
        fn set(&self, name: &str, value: &str) -> Result<(), String> {
            self.0.lock().unwrap().insert(name.into(), value.into());
            Ok(())
        }
        fn delete(&self, name: &str) -> Result<(), String> {
            self.0.lock().unwrap().remove(name);
            Ok(())
        }
    }

    #[test]
    fn providers_require_explicit_enablement_and_keys() {
        let store = MemoryStore::default();
        let with_key = |name: &str| (name == "ANTHROPIC_API_KEY").then(|| "k".to_owned());
        let no_env = |_: &str| None;
        let mut settings = AiSettings::default();
        // A key in the environment alone never enables AI.
        assert!(matches!(
            build_provider(&settings, &store, &with_key),
            Err(AiError::NotConfigured(_))
        ));
        settings.enabled = true;
        assert!(matches!(
            build_provider(&settings, &store, &no_env),
            Err(AiError::MissingApiKey { .. })
        ));
        let provider = build_provider(&settings, &store, &with_key).unwrap();
        assert_eq!(provider.id(), "anthropic");
        assert_eq!(provider.model(ModelRole::Chat), "claude-opus-5-5");
        assert_eq!(provider.model(ModelRole::Completion), "claude-haiku-5-5");

        settings.provider = ProviderKind::Ollama;
        settings.chat_model = String::new();
        assert!(matches!(
            build_provider(&settings, &store, &no_env),
            Err(AiError::NotConfigured(_))
        ));
        settings.chat_model = "qwen2.5-coder".into();
        assert_eq!(
            build_provider(&settings, &store, &no_env).unwrap().id(),
            "ollama"
        );
        settings.provider = ProviderKind::OpenAi;
        assert_eq!(
            build_provider(&settings, &store, &no_env).unwrap().id(),
            "openai"
        );
    }
}
