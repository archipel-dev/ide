//! User-level AI settings, project data-sharing policy and API-key storage.
//!
//! AI is disabled until the user explicitly enables a provider; a key in the
//! environment alone never causes network traffic.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const SETTINGS_FILE: &str = "ai.toml";
pub const SECRETS_FILE: &str = "secrets.toml";
/// Per-project policy, relative to the project root.
pub const PROJECT_POLICY_FILE: &str = ".archipel/ai.toml";

pub const ANTHROPIC_DEFAULT_URL: &str = "https://api.anthropic.com";
pub const OPENAI_DEFAULT_URL: &str = "https://api.openai.com/v1";
pub const OLLAMA_DEFAULT_URL: &str = "http://localhost:11434";
/// Most capable Claude model, used for chat, actions and agent mode.
pub const ANTHROPIC_CHAT_MODEL: &str = "claude-opus-5-5";
/// Fast, small Claude model, used for inline completion.
pub const ANTHROPIC_COMPLETION_MODEL: &str = "claude-haiku-5-5";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ProviderKind {
    #[default]
    Anthropic,
    /// Any endpoint implementing the OpenAI Chat Completions API.
    OpenAi,
    Ollama,
}

impl ProviderKind {
    pub fn id(self) -> &'static str {
        match self {
            ProviderKind::Anthropic => "anthropic",
            ProviderKind::OpenAi => "openai",
            ProviderKind::Ollama => "ollama",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            ProviderKind::Anthropic => "Anthropic",
            ProviderKind::OpenAi => "OpenAI-compatible",
            ProviderKind::Ollama => "Ollama",
        }
    }

    pub fn parse(id: &str) -> Option<Self> {
        [
            ProviderKind::Anthropic,
            ProviderKind::OpenAi,
            ProviderKind::Ollama,
        ]
        .into_iter()
        .find(|kind| kind.id() == id)
    }

    /// Environment variable consulted for the API key, if the provider needs one.
    pub fn key_env(self) -> Option<&'static str> {
        match self {
            ProviderKind::Anthropic => Some("ANTHROPIC_API_KEY"),
            ProviderKind::OpenAi => Some("OPENAI_API_KEY"),
            ProviderKind::Ollama => None,
        }
    }

    pub fn default_url(self) -> &'static str {
        match self {
            ProviderKind::Anthropic => ANTHROPIC_DEFAULT_URL,
            ProviderKind::OpenAi => OPENAI_DEFAULT_URL,
            ProviderKind::Ollama => OLLAMA_DEFAULT_URL,
        }
    }

    /// Default (chat, completion) models; other providers have no safe default.
    pub fn default_models(self) -> Option<(&'static str, &'static str)> {
        match self {
            ProviderKind::Anthropic => Some((ANTHROPIC_CHAT_MODEL, ANTHROPIC_COMPLETION_MODEL)),
            ProviderKind::OpenAi | ProviderKind::Ollama => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AiSettings {
    /// Master switch; nothing is sent anywhere while false.
    pub enabled: bool,
    pub provider: ProviderKind,
    pub chat_model: String,
    pub completion_model: String,
    /// Endpoint override (proxies, gateways, self-hosted servers).
    pub base_url: Option<String>,
    /// Reasoning effort for chat requests (`low` … `max`); `None` omits it.
    pub chat_effort: Option<String>,
    pub completion_effort: Option<String>,
    pub max_output_tokens: u32,
    /// Ask the provider to retry refused requests on a fallback model
    /// (Anthropic server-side fallbacks).
    pub refusal_fallback: bool,
    pub inline_completion: bool,
}

impl Default for AiSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            provider: ProviderKind::Anthropic,
            chat_model: ANTHROPIC_CHAT_MODEL.into(),
            completion_model: ANTHROPIC_COMPLETION_MODEL.into(),
            base_url: None,
            chat_effort: Some("medium".into()),
            completion_effort: Some("low".into()),
            max_output_tokens: 64_000,
            refusal_fallback: true,
            inline_completion: true,
        }
    }
}

impl AiSettings {
    pub fn base_url(&self) -> &str {
        self.base_url
            .as_deref()
            .filter(|url| !url.trim().is_empty())
            .unwrap_or(self.provider.default_url())
            .trim_end_matches('/')
    }

    /// Short description for the status bar, e.g. `Anthropic · claude-opus-5-5`.
    pub fn summary(&self) -> String {
        format!("{} · {}", self.provider.label(), self.chat_model)
    }

    pub fn load(dir: &Path) -> Result<Self, String> {
        match std::fs::read_to_string(dir.join(SETTINGS_FILE)) {
            Ok(text) => toml::from_str(&text)
                .map_err(|error| format!("invalid {}: {error}", dir.join(SETTINGS_FILE).display())),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(error) => Err(error.to_string()),
        }
    }

    pub fn save(&self, dir: &Path) -> Result<(), String> {
        std::fs::create_dir_all(dir).map_err(|error| error.to_string())?;
        let text = toml::to_string_pretty(self).map_err(|error| error.to_string())?;
        std::fs::write(dir.join(SETTINGS_FILE), text).map_err(|error| error.to_string())
    }
}

/// What a project allows to be sent to the configured provider.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ProjectPolicy {
    /// Project-level opt-out of every AI feature.
    pub allow_ai: bool,
    /// Whether file contents and selections may be attached as context.
    pub share_file_contents: bool,
}

impl Default for ProjectPolicy {
    fn default() -> Self {
        Self {
            allow_ai: true,
            share_file_contents: true,
        }
    }
}

impl ProjectPolicy {
    pub fn load(project_root: &Path) -> Result<Self, String> {
        // Projects from before the rename keep the file in `.jetthing/`.
        let path = archipel_ide_compat::project_read_path(project_root.join(PROJECT_POLICY_FILE));
        match std::fs::read_to_string(&path) {
            Ok(text) => toml::from_str(&text)
                .map_err(|error| format!("invalid {}: {error}", path.display())),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(error) => Err(error.to_string()),
        }
    }
}

/// Per-user configuration directory: `ARCHIPEL_CONFIG_DIR`, else the platform
/// convention (`$XDG_CONFIG_HOME/archipel`, `~/Library/Application Support/archipel`,
/// `%APPDATA%\archipel`).
pub fn config_dir() -> Option<PathBuf> {
    if let Some(dir) = archipel_ide_compat::env_var_os("ARCHIPEL_CONFIG_DIR") {
        return Some(PathBuf::from(dir));
    }
    let home = || std::env::var_os("HOME").map(PathBuf::from);
    let dir = if cfg!(target_os = "windows") {
        std::env::var_os("APPDATA").map(|dir| PathBuf::from(dir).join("archipel"))
    } else if cfg!(target_os = "macos") {
        home().map(|home| home.join("Library/Application Support/archipel"))
    } else {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| home().map(|home| home.join(".config")))
            .map(|dir| dir.join("archipel"))
    };
    // A directory left by the Jetthing versions is moved to the new name on first use.
    dir.map(archipel_ide_compat::adopt_legacy_dir)
}

/// Storage for API keys.
pub trait SecretStore: Send + Sync {
    fn get(&self, name: &str) -> Option<String>;
    fn set(&self, name: &str, value: &str) -> Result<(), String>;
    fn delete(&self, name: &str) -> Result<(), String>;
}

/// Keys in `secrets.toml` in the config directory, readable only by the owner on Unix.
/// (OS keychain integration is a planned alternative backend.)
pub struct FileSecretStore {
    path: PathBuf,
}

impl FileSecretStore {
    pub fn new(dir: &Path) -> Self {
        Self {
            path: dir.join(SECRETS_FILE),
        }
    }

    fn read(&self) -> BTreeMap<String, String> {
        std::fs::read_to_string(&self.path)
            .ok()
            .and_then(|text| toml::from_str(&text).ok())
            .unwrap_or_default()
    }

    fn write(&self, secrets: &BTreeMap<String, String>) -> Result<(), String> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        let text = toml::to_string(secrets).map_err(|error| error.to_string())?;
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create(true).truncate(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options
            .open(&self.path)
            .map_err(|error| error.to_string())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            file.set_permissions(std::fs::Permissions::from_mode(0o600))
                .map_err(|error| error.to_string())?;
        }
        std::io::Write::write_all(&mut file, text.as_bytes()).map_err(|error| error.to_string())
    }
}

impl SecretStore for FileSecretStore {
    fn get(&self, name: &str) -> Option<String> {
        self.read().remove(name).filter(|value| !value.is_empty())
    }

    fn set(&self, name: &str, value: &str) -> Result<(), String> {
        let mut secrets = self.read();
        secrets.insert(name.into(), value.into());
        self.write(&secrets)
    }

    fn delete(&self, name: &str) -> Result<(), String> {
        let mut secrets = self.read();
        if secrets.remove(name).is_some() {
            self.write(&secrets)?;
        }
        Ok(())
    }
}

/// Name under which a provider's key is stored.
pub fn secret_name(provider: ProviderKind) -> String {
    format!("{}.api_key", provider.id())
}

/// Resolves the API key: stored key first, then the provider's environment variable.
pub fn resolve_api_key(
    provider: ProviderKind,
    store: &dyn SecretStore,
    env: &dyn Fn(&str) -> Option<String>,
) -> Option<String> {
    store
        .get(&secret_name(provider))
        .or_else(|| provider.key_env().and_then(env))
        .map(|key| key.trim().to_owned())
        .filter(|key| !key.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("archipel-ide-ai-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn disabled_by_default_and_round_trips() {
        let dir = temp("settings");
        let mut settings = AiSettings::load(&dir).unwrap();
        assert!(!settings.enabled);
        assert_eq!(settings.chat_model, ANTHROPIC_CHAT_MODEL);
        settings.enabled = true;
        settings.base_url = Some("https://gateway.example/".into());
        settings.save(&dir).unwrap();
        let loaded = AiSettings::load(&dir).unwrap();
        assert_eq!(loaded, settings);
        assert_eq!(loaded.base_url(), "https://gateway.example");
        std::fs::write(dir.join(SETTINGS_FILE), "enabled = 3").unwrap();
        assert!(AiSettings::load(&dir).unwrap_err().contains("invalid"));
    }

    #[test]
    fn stored_key_wins_over_environment_and_is_private() {
        let dir = temp("secrets");
        let store = FileSecretStore::new(&dir);
        let env = |name: &str| (name == "ANTHROPIC_API_KEY").then(|| "env-key".to_owned());
        assert_eq!(
            resolve_api_key(ProviderKind::Anthropic, &store, &env).as_deref(),
            Some("env-key")
        );
        assert_eq!(resolve_api_key(ProviderKind::Ollama, &store, &env), None);
        store
            .set(&secret_name(ProviderKind::Anthropic), "stored")
            .unwrap();
        assert_eq!(
            resolve_api_key(ProviderKind::Anthropic, &store, &env).as_deref(),
            Some("stored")
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(dir.join(SECRETS_FILE))
                .unwrap()
                .permissions()
                .mode();
            assert_eq!(mode & 0o777, 0o600);
        }
        store.delete(&secret_name(ProviderKind::Anthropic)).unwrap();
        assert_eq!(store.get(&secret_name(ProviderKind::Anthropic)), None);
    }

    #[test]
    fn project_policy_defaults_to_allowed() {
        let dir = temp("policy");
        assert_eq!(ProjectPolicy::load(&dir).unwrap(), ProjectPolicy::default());
        std::fs::create_dir_all(dir.join(".archipel")).unwrap();
        std::fs::write(dir.join(PROJECT_POLICY_FILE), "share_file_contents = false").unwrap();
        let policy = ProjectPolicy::load(&dir).unwrap();
        assert!(policy.allow_ai && !policy.share_file_contents);
    }
}
