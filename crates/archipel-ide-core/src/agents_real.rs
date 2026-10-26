//! The real Agents mode backend: Claude Code and Codex in git worktrees.
//!
//! A thin adapter from [`archipel_ide_agents::Engine`] (which owns processes, protocol
//! parsing, worktrees and persistence) to the UI's [`AgentsBackend`] contract. No toolkit
//! types here. Only agents whose CLI is installed are offered.

use crate::agent_events::AgentEvent;
use crate::agents::{
    AgentNews, AgentProfile, AgentState, AgentsBackend, AgentsSettings, ChangeKind, ChangedFile,
    MergeStrategy, SessionId, SessionInfo, TaskId, TaskInfo,
};
use archipel_ide_agents::engine::SessionSnapshot;
use archipel_ide_agents::workspace::ChangeKind as EngineChange;
use archipel_ide_agents::{AgentKind, Engine, EngineConfig, News, Outcome, PermissionMode, State};
use archipel_ide_git::FileDiff;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// How often the installed CLIs are looked up again (a user may install one while the IDE
/// runs).
const RESCAN: Duration = Duration::from_secs(5);

pub struct RealAgentsBackend {
    engine: Engine,
    available: Vec<AgentProfile>,
    since_scan: Duration,
}

fn kind_of(profile: AgentProfile) -> Option<AgentKind> {
    match profile {
        AgentProfile::Claude => Some(AgentKind::Claude),
        AgentProfile::Codex => Some(AgentKind::Codex),
        AgentProfile::Gemini | AgentProfile::Opencode => None,
    }
}

fn profile_of(kind: AgentKind) -> AgentProfile {
    match kind {
        AgentKind::Claude => AgentProfile::Claude,
        AgentKind::Codex => AgentProfile::Codex,
    }
}

fn info(s: SessionSnapshot) -> SessionInfo {
    SessionInfo {
        id: s.id,
        task: s.task,
        parent: s.parent,
        profile: profile_of(s.kind),
        name: s.name,
        state: match s.state {
            State::Working => AgentState::Working,
            State::NeedsInput => AgentState::NeedsInput,
            State::Done => AgentState::Done,
            State::Failed => AgentState::Error,
            State::Stopped => AgentState::Stopped,
        },
        unread: s.unread,
        elapsed: s.elapsed,
        worktree: s.worktree.display().to_string(),
        branch: s.branch,
        tokens: s.tokens,
        question: s.question,
        files: s.files,
        added: s.added,
        removed: s.removed,
        outcome: s.outcome.map(|o| match o {
            Outcome::Squash => MergeStrategy::Squash,
            Outcome::Merge => MergeStrategy::Merge,
            Outcome::Discard => MergeStrategy::Discard,
        }),
    }
}

impl RealAgentsBackend {
    pub fn new(data_dir: PathBuf) -> Self {
        Self::with_config(EngineConfig {
            data_dir,
            ..EngineConfig::default()
        })
    }

    pub fn with_config(config: EngineConfig) -> Self {
        let engine = Engine::new(config);
        let mut backend = Self {
            engine,
            available: Vec::new(),
            since_scan: Duration::ZERO,
        };
        backend.rescan();
        backend
    }

    pub fn engine(&self) -> &Engine {
        &self.engine
    }

    pub fn engine_mut(&mut self) -> &mut Engine {
        &mut self.engine
    }

    fn rescan(&mut self) {
        self.available = self
            .engine
            .available()
            .into_iter()
            .map(profile_of)
            .collect();
        self.since_scan = Duration::ZERO;
    }
}

impl AgentsBackend for RealAgentsBackend {
    fn set_project(&mut self, root: Option<&Path>) {
        self.engine.set_project(root);
    }

    fn tasks(&self) -> Vec<TaskInfo> {
        self.engine
            .tasks()
            .into_iter()
            .map(|t| TaskInfo {
                id: t.id,
                title: t.title,
                prompt: t.prompt,
                sessions: t.sessions,
            })
            .collect()
    }

    fn sessions(&self) -> Vec<SessionInfo> {
        self.engine.sessions().into_iter().map(info).collect()
    }

    fn launch(&mut self, prompt: &str, profiles: &[AgentProfile]) -> TaskId {
        self.launch_with_mode(prompt, profiles, None)
    }

    fn launch_with_mode(
        &mut self,
        prompt: &str,
        profiles: &[AgentProfile],
        mode: Option<&str>,
    ) -> TaskId {
        let kinds: Vec<AgentKind> = profiles.iter().filter_map(|p| kind_of(*p)).collect();
        self.engine
            .launch(prompt, &kinds, mode.and_then(PermissionMode::parse))
    }

    fn send(&mut self, session: SessionId, input: &str) {
        self.engine.send(session, input);
    }

    fn stop(&mut self, session: SessionId) {
        self.engine.stop(session);
    }

    fn changes(&self, session: SessionId) -> Vec<ChangedFile> {
        self.engine
            .changes(session)
            .iter()
            .map(|c| ChangedFile {
                path: c.path.clone(),
                kind: match c.kind {
                    EngineChange::Added => ChangeKind::Added,
                    EngineChange::Modified | EngineChange::Deleted => ChangeKind::Modified,
                },
                added: c.added,
                removed: c.removed,
                first_line: c.first_line,
            })
            .collect()
    }

    fn diff(&self, session: SessionId, path: &Path) -> Option<FileDiff> {
        self.engine.diff(session, path)
    }

    fn merge(&mut self, session: SessionId, strategy: MergeStrategy) -> Result<String, String> {
        self.engine.merge(
            session,
            match strategy {
                MergeStrategy::Squash => Outcome::Squash,
                MergeStrategy::Merge => Outcome::Merge,
                MergeStrategy::Discard => Outcome::Discard,
            },
        )
    }

    fn events(&self, session: SessionId) -> &[AgentEvent] {
        self.engine.events(session)
    }

    fn mark_read(&mut self, session: SessionId) {
        self.engine.mark_read(session);
    }

    fn advance(&mut self, dt: Duration) -> Vec<AgentNews> {
        self.since_scan += dt;
        if self.since_scan >= RESCAN {
            self.rescan();
        }
        self.engine
            .poll()
            .into_iter()
            .map(|news| match news {
                News::NeedsInput(id) => AgentNews::NeedsInput(id),
                News::Done(id) => AgentNews::Done(id),
                News::Failed(id) => AgentNews::Failed(id),
                News::Spawned { parent, child } => AgentNews::Spawned { parent, child },
            })
            .collect()
    }

    fn available_profiles(&self) -> Vec<AgentProfile> {
        self.available.clone()
    }

    fn configure(&mut self, settings: &AgentsSettings) {
        let mut config = self.engine.config().clone();
        config.max_parallel = settings.max_parallel.max(1);
        config.default_mode = PermissionMode::parse(&settings.permission_mode).unwrap_or_default();
        config.worktree_base = settings.worktree_base.clone();
        config.claude_path = settings.claude_path.clone();
        config.codex_path = settings.codex_path.clone();
        self.engine.set_config(config);
        self.rescan();
    }

    fn busy(&self) -> bool {
        self.engine.busy()
    }

    fn shutdown(&mut self) {
        self.engine.shutdown();
    }

    fn is_real(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::AgentsState;

    #[test]
    fn only_installed_agents_are_offered_and_gemini_is_never_real() {
        let dir = tempfile::tempdir().unwrap();
        let mut backend = RealAgentsBackend::new(dir.path().to_path_buf());
        backend.configure(&AgentsSettings {
            claude_path: Some(dir.path().join("missing-claude")),
            codex_path: Some(dir.path().join("missing-codex")),
            ..AgentsSettings::default()
        });
        assert!(backend.available_profiles().is_empty());
        assert!(backend.is_real());
        // A launch with nothing installed makes no session for unsupported agents.
        assert!(backend.tasks().is_empty());
        let task = backend.launch("x", &[AgentProfile::Gemini]);
        assert!(task > 0);
        assert!(backend.sessions().is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn an_executable_override_makes_the_agent_available() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let bin = dir.path().join("my-claude");
        std::fs::write(&bin, "#!/bin/sh\n").unwrap();
        std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
        let mut backend = RealAgentsBackend::new(dir.path().to_path_buf());
        backend.configure(&AgentsSettings {
            claude_path: Some(bin),
            codex_path: Some(dir.path().join("nope")),
            ..AgentsSettings::default()
        });
        assert_eq!(backend.available_profiles(), [AgentProfile::Claude]);
    }

    #[test]
    fn the_demo_never_replaces_real_sessions_but_replaces_an_empty_real_backend() {
        let dir = tempfile::tempdir().unwrap();
        let mut state = crate::IdeState::default();
        state.agents = AgentsState::new(Box::new(RealAgentsBackend::new(dir.path().to_path_buf())));
        assert!(state.agents.backend.is_real());
        state.seed_agents_demo();
        assert!(
            !state.agents.backend.is_real(),
            "empty real backend gives way to the demo"
        );
        assert!(!state.agents.sessions().is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn settings_reach_the_backend_and_launch_explains_a_missing_cli() {
        use crate::built_in_commands;
        use archipel_ide_settings::ids;
        use serde_json::json;
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("proj");
        std::fs::create_dir_all(&project).unwrap();
        let commands = built_in_commands();
        let mut state = crate::IdeState::default();
        state.load_user_settings(&dir.path().join("config"));
        commands
            .execute("project.open", &mut state, json!({"path": project}))
            .unwrap();
        state.agents = AgentsState::new(Box::new(RealAgentsBackend::new(dir.path().join("data"))));
        for (id, value) in [
            (ids::AGENTS_CLAUDE_PATH, json!(dir.path().join("no-claude"))),
            (ids::AGENTS_CODEX_PATH, json!(dir.path().join("no-codex"))),
            (ids::AGENTS_MAX_PARALLEL, json!(2)),
            (ids::AGENTS_PERMISSION_MODE, json!("plan")),
        ] {
            commands
                .execute(
                    "settings.set",
                    &mut state,
                    json!({"id": id, "value": value}),
                )
                .unwrap();
        }
        let settings = state.agents_settings();
        assert_eq!(settings.max_parallel, 2);
        assert_eq!(settings.permission_mode, "plan");
        state.poll_agents();
        assert!(state.agents.backend.available_profiles().is_empty());
        let error = commands
            .execute("agents.launch", &mut state, json!({"prompt": "do it"}))
            .unwrap_err()
            .to_string();
        assert!(error.contains("Set up agents"), "{error}");

        // Installing (here: pointing the override at an executable) makes the agent appear.
        let bin = dir.path().join("claude");
        std::fs::write(&bin, "#!/bin/sh\nexit 0\n").unwrap();
        std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
        commands
            .execute(
                "settings.set",
                &mut state,
                json!({"id": ids::AGENTS_CLAUDE_PATH, "value": bin}),
            )
            .unwrap();
        state.poll_agents();
        assert_eq!(
            state.agents.backend.available_profiles(),
            [AgentProfile::Claude]
        );
        let bad_agent = commands
            .execute(
                "agents.launch",
                &mut state,
                json!({"prompt": "x", "profiles": ["codex"]}),
            )
            .unwrap_err()
            .to_string();
        assert!(bad_agent.contains("not installed"), "{bad_agent}");
        let bad_mode = commands
            .execute(
                "agents.launch",
                &mut state,
                json!({"prompt": "x", "mode": "yolo"}),
            )
            .unwrap_err()
            .to_string();
        assert!(bad_mode.contains("permission mode"), "{bad_mode}");
    }
}
