//! The agents the engine can run: how to find their CLI, which arguments each protocol
//! needs and which environment variables they get.

use crate::claude::ClaudeAdapter;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AgentKind {
    Claude,
    Codex,
}

impl AgentKind {
    pub const ALL: [AgentKind; 2] = [AgentKind::Claude, AgentKind::Codex];

    pub fn id(self) -> &'static str {
        match self {
            AgentKind::Claude => "claude",
            AgentKind::Codex => "codex",
        }
    }

    pub fn parse(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.id() == id)
    }

    /// Name of the executable searched on `PATH`.
    pub fn binary(self) -> &'static str {
        self.id()
    }

    /// Environment variables that belong to the agent itself (credentials, endpoints,
    /// its own settings) on top of the common allowlist.
    fn env_prefixes(self) -> &'static [&'static str] {
        match self {
            AgentKind::Claude => &[
                "ANTHROPIC_",
                "CLAUDE_",
                "AWS_",
                "GOOGLE_",
                "CLOUD_ML_",
                "VERTEX_",
            ],
            AgentKind::Codex => &["OPENAI_", "CODEX_", "AZURE_"],
        }
    }
}

/// How much the agent may do without asking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PermissionMode {
    /// Ask before edits and commands.
    #[default]
    Ask,
    /// Edits are allowed, commands still ask.
    AutoEdit,
    /// Read-only planning.
    Plan,
}

impl PermissionMode {
    pub fn id(self) -> &'static str {
        match self {
            PermissionMode::Ask => "ask",
            PermissionMode::AutoEdit => "auto-edit",
            PermissionMode::Plan => "plan",
        }
    }

    pub fn parse(id: &str) -> Option<Self> {
        [Self::Ask, Self::AutoEdit, Self::Plan]
            .into_iter()
            .find(|mode| mode.id() == id)
    }

    /// `--permission-mode` value of Claude Code (documented values).
    fn claude_flag(self) -> &'static str {
        match self {
            PermissionMode::Ask => "default",
            PermissionMode::AutoEdit => "acceptEdits",
            PermissionMode::Plan => "plan",
        }
    }

    /// Codex `exec` cannot ask: the sandbox is the only lever.
    fn codex_sandbox(self) -> &'static str {
        match self {
            PermissionMode::Plan => "read-only",
            PermissionMode::Ask | PermissionMode::AutoEdit => "workspace-write",
        }
    }
}

/// What to start.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandPlan {
    pub args: Vec<OsString>,
    /// Written to stdin right after the start (Claude takes prompts as stream-json lines).
    pub first_input: Option<String>,
}

/// The id of the agent's own session a launch uses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionArg {
    /// Claude: pre-assign this UUID (`--session-id`).
    New(String),
    /// Claude: `--resume <id>`; Codex: `exec resume <id>`.
    Resume(String),
    /// Codex: a first turn, the thread id is learned from `thread.started`.
    Fresh,
}

pub fn build_command(
    kind: AgentKind,
    mode: PermissionMode,
    session: &SessionArg,
    prompt: &str,
    model: Option<&str>,
) -> CommandPlan {
    let s = |text: &str| OsString::from(text);
    match kind {
        AgentKind::Claude => {
            let mut args: Vec<OsString> = [
                "-p",
                "--output-format",
                "stream-json",
                "--input-format",
                "stream-json",
                "--verbose",
                "--permission-mode",
                mode.claude_flag(),
            ]
            .iter()
            .map(|a| s(a))
            .collect();
            if mode == PermissionMode::Ask {
                // [INFER] `stdio` makes permission requests arrive as `control_request`
                // lines and wait for a `control_response` (the Agent SDK mechanism).
                args.extend([s("--permission-prompt-tool"), s("stdio")]);
            }
            match session {
                SessionArg::New(id) => args.extend([s("--session-id"), s(id)]),
                SessionArg::Resume(id) => args.extend([s("--resume"), s(id)]),
                SessionArg::Fresh => {}
            }
            if let Some(model) = model.filter(|m| !m.is_empty()) {
                args.extend([s("--model"), s(model)]);
            }
            CommandPlan {
                args,
                first_input: Some(ClaudeAdapter::user_message(prompt)),
            }
        }
        AgentKind::Codex => {
            let mut args: Vec<OsString> = vec![
                s("-c"),
                s("approval_policy=\"never\""),
                s("-c"),
                s(&format!("sandbox_mode=\"{}\"", mode.codex_sandbox())),
            ];
            if let Some(model) = model.filter(|m| !m.is_empty()) {
                args.extend([s("-c"), s(&format!("model=\"{model}\""))]);
            }
            args.extend([s("exec"), s("--json")]);
            if let SessionArg::Resume(id) = session {
                args.extend([s("resume"), s(id)]);
            }
            args.extend([s("--"), s(prompt)]);
            CommandPlan {
                args,
                first_input: None,
            }
        }
    }
}

/// A UUID (version 4 layout) for `--session-id`; the randomness comes from the standard
/// library's per-process hash keys plus the clock, which is plenty for an identifier that
/// only has to be unique.
pub fn new_uuid() -> String {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let mut bytes = [0u8; 16];
    for (i, chunk) in bytes.chunks_mut(8).enumerate() {
        let mut hasher = RandomState::new().build_hasher();
        hasher.write_u128(nanos);
        hasher.write_usize(i);
        chunk.copy_from_slice(&hasher.finish().to_le_bytes());
    }
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

// ---------------------------------------------------------------------------------------------
// CLI discovery

/// Directories searched after `PATH`, because a desktop-launched IDE often has a short one.
fn fallback_dirs(home: Option<&Path>) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(home) = home {
        for sub in [
            ".local/bin",
            ".claude/local",
            ".npm-global/bin",
            ".bun/bin",
            ".cargo/bin",
            "bin",
        ] {
            dirs.push(home.join(sub));
        }
    }
    for dir in ["/usr/local/bin", "/opt/homebrew/bin", "/usr/bin"] {
        dirs.push(PathBuf::from(dir));
    }
    dirs
}

fn is_executable(path: &Path) -> bool {
    let Ok(meta) = std::fs::metadata(path) else {
        return false;
    };
    if !meta.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        meta.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}

/// Finds the CLI of `kind`: the user's override when set (it must exist; an invalid
/// override means "not installed"), else the first match on `path_var`, else a few usual
/// install directories.
pub fn find_cli(
    kind: AgentKind,
    override_path: Option<&Path>,
    path_var: Option<&OsStr>,
    home: Option<&Path>,
) -> Option<PathBuf> {
    if let Some(path) = override_path.filter(|p| !p.as_os_str().is_empty()) {
        return is_executable(path).then(|| path.to_path_buf());
    }
    let names: Vec<String> = if cfg!(windows) {
        ["exe", "cmd", "bat"]
            .iter()
            .map(|ext| format!("{}.{ext}", kind.binary()))
            .collect()
    } else {
        vec![kind.binary().to_owned()]
    };
    let from_path: Vec<PathBuf> = path_var
        .map(|p| std::env::split_paths(p).collect())
        .unwrap_or_default();
    from_path
        .into_iter()
        .chain(fallback_dirs(home))
        .filter(|dir| !dir.as_os_str().is_empty())
        .flat_map(|dir| names.iter().map(move |name| dir.join(name)))
        .find(|candidate| is_executable(candidate))
}

// ---------------------------------------------------------------------------------------------
// Environment

const COMMON_ENV: [&str; 34] = [
    "PATH",
    "HOME",
    "USER",
    "LOGNAME",
    "SHELL",
    "LANG",
    "LANGUAGE",
    "TERM",
    "COLORTERM",
    "TMPDIR",
    "TMP",
    "TEMP",
    "TZ",
    "SSH_AUTH_SOCK",
    "XDG_CONFIG_HOME",
    "XDG_DATA_HOME",
    "XDG_CACHE_HOME",
    "XDG_STATE_HOME",
    "XDG_RUNTIME_DIR",
    "HTTP_PROXY",
    "HTTPS_PROXY",
    "NO_PROXY",
    "ALL_PROXY",
    "SSL_CERT_FILE",
    "SSL_CERT_DIR",
    "CURL_CA_BUNDLE",
    "REQUESTS_CA_BUNDLE",
    "NODE_EXTRA_CA_CERTS",
    "SYSTEMROOT",
    "USERPROFILE",
    "APPDATA",
    "LOCALAPPDATA",
    "COMSPEC",
    "PATHEXT",
];

/// The environment an agent gets: the common allowlist, the agent's own variables, and
/// `extra` (explicit values such as test hooks) last. Everything else is dropped, so an
/// unrelated secret in the IDE's environment never reaches an agent.
pub fn agent_env(
    kind: AgentKind,
    vars: impl IntoIterator<Item = (OsString, OsString)>,
    extra: &[(String, String)],
) -> Vec<(OsString, OsString)> {
    let mut out: Vec<(OsString, OsString)> = Vec::new();
    for (key, value) in vars {
        let upper = key.to_string_lossy().to_uppercase();
        let allowed = COMMON_ENV.contains(&upper.as_str())
            || upper.starts_with("LC_")
            || kind.env_prefixes().iter().any(|p| upper.starts_with(p));
        if allowed {
            out.push((key, value));
        }
    }
    out.push(("GIT_TERMINAL_PROMPT".into(), "0".into()));
    for (key, value) in extra {
        out.retain(|(k, _)| k != OsStr::new(key));
        out.push((key.into(), value.into()));
    }
    out
}

/// Values of the variables that look like secrets (for redacting transcripts).
pub fn secret_values(vars: impl IntoIterator<Item = (String, String)>) -> Vec<String> {
    vars.into_iter()
        .filter(|(key, value)| {
            let upper = key.to_uppercase();
            value.len() >= 8
                && ["KEY", "TOKEN", "SECRET", "PASSWORD", "PASSWD", "CREDENTIAL"]
                    .iter()
                    .any(|word| upper.contains(word))
        })
        .map(|(_, value)| value)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(args: &[OsString]) -> Vec<String> {
        args.iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn claude_arguments_follow_the_documented_flags() {
        let plan = build_command(
            AgentKind::Claude,
            PermissionMode::AutoEdit,
            &SessionArg::New("11111111-2222-4333-8444-555555555555".into()),
            "fix it",
            None,
        );
        let args = strings(&plan.args);
        assert_eq!(
            &args[..8],
            [
                "-p",
                "--output-format",
                "stream-json",
                "--input-format",
                "stream-json",
                "--verbose",
                "--permission-mode",
                "acceptEdits"
            ]
        );
        assert!(
            args.windows(2)
                .any(|w| w == ["--session-id", "11111111-2222-4333-8444-555555555555"])
        );
        assert!(!args.contains(&"--permission-prompt-tool".to_string()));
        assert!(
            !args.iter().any(|a| a == "fix it"),
            "the prompt goes to stdin, not argv"
        );
        assert!(plan.first_input.unwrap().contains("fix it"));

        let ask = build_command(
            AgentKind::Claude,
            PermissionMode::Ask,
            &SessionArg::Resume("abc".into()),
            "x",
            Some("opus"),
        );
        let args = strings(&ask.args);
        assert!(
            args.windows(2)
                .any(|w| w == ["--permission-prompt-tool", "stdio"])
        );
        assert!(args.windows(2).any(|w| w == ["--resume", "abc"]));
        assert!(args.windows(2).any(|w| w == ["--model", "opus"]));
        let plan = build_command(
            AgentKind::Claude,
            PermissionMode::Plan,
            &SessionArg::Fresh,
            "x",
            None,
        );
        assert!(
            strings(&plan.args)
                .windows(2)
                .any(|w| w == ["--permission-mode", "plan"])
        );
    }

    #[test]
    fn codex_arguments_use_exec_json_and_resume() {
        let plan = build_command(
            AgentKind::Codex,
            PermissionMode::Plan,
            &SessionArg::Fresh,
            "-rm -rf",
            None,
        );
        let args = strings(&plan.args);
        assert!(
            args.windows(2)
                .any(|w| w == ["-c", "sandbox_mode=\"read-only\""])
        );
        let exec = args.iter().position(|a| a == "exec").unwrap();
        assert_eq!(args[exec + 1], "--json");
        assert_eq!(
            &args[args.len() - 2..],
            ["--", "-rm -rf"],
            "a prompt can never be an option"
        );
        let resumed = build_command(
            AgentKind::Codex,
            PermissionMode::Ask,
            &SessionArg::Resume("th".into()),
            "go",
            None,
        );
        let args = strings(&resumed.args);
        assert!(args.windows(3).any(|w| w == ["--json", "resume", "th"]));
        assert!(plan.first_input.is_none());
    }

    #[test]
    fn modes_round_trip() {
        for mode in [
            PermissionMode::Ask,
            PermissionMode::AutoEdit,
            PermissionMode::Plan,
        ] {
            assert_eq!(PermissionMode::parse(mode.id()), Some(mode));
        }
        assert_eq!(PermissionMode::parse("yolo"), None);
    }

    #[test]
    fn uuids_look_like_v4_and_differ() {
        let a = new_uuid();
        let b = new_uuid();
        assert_ne!(a, b);
        assert_eq!(a.len(), 36);
        assert_eq!(a.as_bytes()[14], b'4');
        assert!(matches!(a.as_bytes()[19], b'8' | b'9' | b'a' | b'b'));
    }

    #[test]
    fn env_is_an_allowlist_plus_the_agents_own_variables() {
        let vars = [
            ("PATH", "/bin"),
            ("HOME", "/h"),
            ("LC_ALL", "C"),
            ("ANTHROPIC_API_KEY", "k"),
            ("OPENAI_API_KEY", "o"),
            ("GITHUB_TOKEN", "ghp_secret"),
            ("AWS_SECRET_ACCESS_KEY", "s"),
            ("DATABASE_URL", "postgres://x"),
        ]
        .map(|(k, v)| (OsString::from(k), OsString::from(v)));
        let claude: Vec<String> = agent_env(AgentKind::Claude, vars.clone(), &[])
            .into_iter()
            .map(|(k, _)| k.to_string_lossy().into_owned())
            .collect();
        assert!(claude.contains(&"ANTHROPIC_API_KEY".to_string()));
        assert!(claude.contains(&"AWS_SECRET_ACCESS_KEY".to_string()));
        assert!(!claude.contains(&"OPENAI_API_KEY".to_string()));
        assert!(!claude.contains(&"GITHUB_TOKEN".to_string()));
        assert!(!claude.contains(&"DATABASE_URL".to_string()));
        assert!(claude.contains(&"LC_ALL".to_string()));
        let codex = agent_env(AgentKind::Codex, vars, &[("FAKE".into(), "1".into())]);
        assert!(codex.iter().any(|(k, _)| k == "OPENAI_API_KEY"));
        assert!(codex.iter().any(|(k, v)| k == "FAKE" && v == "1"));
        assert!(!codex.iter().any(|(k, _)| k == "ANTHROPIC_API_KEY"));
    }

    #[cfg(unix)]
    #[test]
    fn cli_lookup_honours_overrides_and_path() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let bin = dir.path().join("claude");
        std::fs::write(&bin, "#!/bin/sh\n").unwrap();
        std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
        let path = std::env::join_paths([dir.path()]).unwrap();
        let home = dir.path().join("nohome");
        assert_eq!(
            find_cli(AgentKind::Claude, None, Some(&path), Some(&home)),
            Some(bin.clone())
        );
        assert_eq!(
            find_cli(AgentKind::Codex, None, Some(&path), Some(&home))
                .filter(|p| p.starts_with(dir.path())),
            None
        );
        assert_eq!(
            find_cli(AgentKind::Claude, Some(&bin), None, None),
            Some(bin)
        );
        assert_eq!(
            find_cli(
                AgentKind::Claude,
                Some(&dir.path().join("missing")),
                Some(&path),
                None
            ),
            None
        );
        let plain = dir.path().join("plain");
        std::fs::write(&plain, "x").unwrap();
        assert_eq!(
            find_cli(AgentKind::Claude, Some(&plain), None, None),
            None,
            "not executable"
        );
    }

    #[test]
    fn secret_values_pick_credentials_only() {
        let values = secret_values([
            ("API_KEY".to_string(), "abcdefgh12".to_string()),
            ("PATH".to_string(), "/usr/bin:/bin".to_string()),
            ("TOKEN".to_string(), "short".to_string()),
        ]);
        assert_eq!(values, ["abcdefgh12"]);
    }
}
