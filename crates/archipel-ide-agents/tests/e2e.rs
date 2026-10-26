//! End to end: the real engine, real processes (a fake agent CLI replaying fixtures) and a
//! real temporary git repository. Covers launch, events, permission answers, changes,
//! squash merge and cleanup, stop and kill, errors, unknown events and restart/resume.

use archipel_ide_agents::events::{AgentEvent, ToolKind, ToolStatus};
use archipel_ide_agents::{
    AgentKind, Engine, EngineConfig, News, Outcome, PermissionMode, SessionId, State,
};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

const FAKE: &str = env!("CARGO_BIN_EXE_fake-agent");

fn fixture(name: &str) -> String {
    format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"))
}

fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .expect("git runs");
    assert!(
        out.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_owned()
}

struct Env {
    repo: tempfile::TempDir,
    data: tempfile::TempDir,
    engine: Engine,
    news: Vec<News>,
}

fn config(data: &Path, script: &str, resume: Option<&str>) -> EngineConfig {
    let mut extra = vec![("FAKE_AGENT_SCRIPT".to_owned(), fixture(script))];
    if let Some(resume) = resume {
        extra.push(("FAKE_AGENT_RESUME_SCRIPT".to_owned(), fixture(resume)));
    }
    EngineConfig {
        data_dir: data.to_path_buf(),
        claude_path: Some(PathBuf::from(FAKE)),
        codex_path: Some(PathBuf::from(FAKE)),
        stop_grace: Duration::from_millis(400),
        extra_env: extra,
        ..EngineConfig::default()
    }
}

impl Env {
    fn new(script: &str, resume: Option<&str>) -> Self {
        let repo = tempfile::tempdir().unwrap();
        let data = tempfile::tempdir().unwrap();
        let root = repo.path();
        git(root, &["init", "-q", "-b", "main"]);
        git(root, &["config", "user.email", "dev@example.com"]);
        git(root, &["config", "user.name", "Dev"]);
        git(root, &["config", "commit.gpgsign", "false"]);
        std::fs::write(root.join("README.md"), "# demo\n").unwrap();
        git(root, &["add", "-A"]);
        git(root, &["commit", "-q", "-m", "initial"]);
        let mut engine = Engine::new(config(data.path(), script, resume));
        engine.set_project(Some(root));
        Self {
            repo,
            data,
            engine,
            news: Vec::new(),
        }
    }

    fn script(&mut self, script: &str, resume: Option<&str>) {
        self.engine
            .set_config(config(self.data.path(), script, resume));
    }

    fn wait(&mut self, what: &str, mut done: impl FnMut(&Engine) -> bool) {
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            let news = self.engine.poll();
            self.news.extend(news);
            if done(&self.engine) {
                return;
            }
            assert!(Instant::now() < deadline, "timed out waiting for {what}");
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    fn wait_state(&mut self, id: SessionId, state: State) {
        self.wait(&format!("{state:?}"), |e| {
            e.snapshot(id).is_some_and(|s| s.state == state)
        });
    }

    fn texts(&self, id: SessionId) -> Vec<String> {
        self.engine
            .events(id)
            .iter()
            .filter_map(|e| match e {
                AgentEvent::AssistantText(t) => Some(t.clone()),
                _ => None,
            })
            .collect()
    }

    fn launch(&mut self, prompt: &str, kind: AgentKind, mode: PermissionMode) -> SessionId {
        self.engine.launch(prompt, &[kind], Some(mode));
        self.engine.sessions().last().expect("a session").id
    }
}

fn tool_rows(events: &[AgentEvent]) -> Vec<(ToolKind, String, ToolStatus)> {
    events
        .iter()
        .filter_map(|e| match e {
            AgentEvent::ToolCall {
                kind,
                title,
                status,
                ..
            } => Some((*kind, title.clone(), *status)),
            _ => None,
        })
        .collect()
}

fn pid_alive(pid: u32) -> bool {
    Command::new("kill")
        .args(["-0", &pid.to_string()])
        .output()
        .is_ok_and(|o| o.status.success())
}

#[test]
fn claude_full_turn_then_follow_up_then_squash_merge_and_cleanup() {
    let mut env = Env::new("claude_full_turn.jsonl", None);
    let id = env.launch(
        "Add a greeting file",
        AgentKind::Claude,
        PermissionMode::AutoEdit,
    );
    let snap = env.engine.snapshot(id).unwrap();
    assert_eq!(snap.state, State::Working, "launch returns at once");
    assert!(
        snap.worktree.starts_with(env.data.path()),
        "worktree lives under the data dir"
    );
    assert_eq!(snap.branch, "archipel/add-a-greeting-file");
    env.wait_state(id, State::Done);
    assert!(env.news.contains(&News::Done(id)));

    // The transcript: every documented event kind, unknown ones tolerated.
    let events = env.engine.events(id);
    assert!(matches!(&events[0], AgentEvent::UserPrompt(p) if p == "Add a greeting file"));
    assert!(
        events
            .iter()
            .any(|e| matches!(e, AgentEvent::Thinking(t) if t.starts_with("Plan")))
    );
    let rows = tool_rows(events);
    assert!(rows.contains(&(ToolKind::Read, "Read README.md".into(), ToolStatus::Ok)));
    assert!(rows.contains(&(ToolKind::Write, "Write greeting.txt".into(), ToolStatus::Ok)));
    assert!(rows.contains(&(
        ToolKind::Bash,
        "Bash cat greeting.txt".into(),
        ToolStatus::Ok
    )));
    assert!(rows.contains(&(ToolKind::Search, "Grep hello".into(), ToolStatus::Ok)));
    assert!(rows.iter().any(|(k, t, s)| *k == ToolKind::Other
        && t.starts_with("Fetch")
        && *s == ToolStatus::Error));
    assert!(
        rows.iter()
            .any(|(k, _, s)| *k == ToolKind::Spawn && *s == ToolStatus::Ok)
    );
    assert!(
        rows.iter().any(|(_, t, _)| t == "Event future_event_kind"),
        "unknown event became a generic row"
    );
    assert!(events.iter().any(
        |e| matches!(e, AgentEvent::SubagentSpawned { name, .. } if name == "Check the docs")
    ));
    assert!(!events.iter().any(|e| matches!(e, AgentEvent::Error(_))));
    let snap = env.engine.snapshot(id).unwrap();
    assert_eq!(snap.tokens, 150);
    assert!((snap.cost_usd - 0.0123).abs() < 1e-9);
    assert_eq!(snap.model.as_deref(), Some("fake-model"));
    assert!(env.engine.provider_session(id).is_some());

    // Changes against the merge base, computed off-thread, appear in the turn result.
    env.wait("changes", |e| e.changes(id).len() == 1);
    let changes = env.engine.changes(id).to_vec();
    assert_eq!(changes[0].path, PathBuf::from("greeting.txt"));
    assert_eq!((changes[0].added, changes[0].removed), (1, 0));
    let diff = env
        .engine
        .diff(id, Path::new("greeting.txt"))
        .expect("cached diff");
    assert_eq!(diff.new_text, "hello from the agent\n");
    assert_eq!(diff.hunks.len(), 1);
    env.wait("turn stats", |e| {
        e.events(id).iter().any(|x| {
            matches!(
                x,
                AgentEvent::TurnResult {
                    files: 1,
                    added: 1,
                    ..
                }
            )
        })
    });
    let result = env.engine.events(id).iter().find_map(|e| match e {
        AgentEvent::TurnResult {
            duration, tokens, ..
        } => Some((*duration, *tokens)),
        _ => None,
    });
    assert_eq!(result, Some((Duration::from_millis(2100), 150)));

    // Follow-up goes to the same process over stdin.
    let pids = env.engine.process_ids(id);
    assert_eq!(pids.len(), 1);
    env.engine.send(id, "now add a farewell\r");
    assert_eq!(env.engine.snapshot(id).unwrap().state, State::Working);
    env.wait("follow-up echo", |e| {
        e.events(id)
            .iter()
            .any(|x| matches!(x, AgentEvent::AssistantText(t) if t == "echo: now add a farewell"))
    });
    env.wait_state(id, State::Done);
    assert_eq!(env.engine.process_ids(id), pids, "no new process");

    // Secrets never reach the transcript log.
    let log = std::fs::read_dir(env.data.path().join("archipel/agents"))
        .unwrap()
        .flatten()
        .next()
        .unwrap()
        .path()
        .join(format!("sessions/{id}/transcript.jsonl"));
    let raw = std::fs::read_to_string(log).unwrap();
    assert!(raw.contains("I will add a greeting file"));
    assert!(!raw.contains("LEAKLEAK"), "api key written to disk");
    assert!(
        env.texts(id).iter().any(|t| t.contains("LEAKLEAK")),
        "memory keeps the real text"
    );

    // Squash merge into main, then cleanup.
    let worktree = snap.worktree.clone();
    assert!(worktree.join("greeting.txt").exists());
    let message = env.engine.merge(id, Outcome::Squash).expect("merge");
    assert!(
        message.starts_with("Squash-merged add-a-greeting-file into main"),
        "{message}"
    );
    let root = env.repo.path();
    assert_eq!(
        std::fs::read_to_string(root.join("greeting.txt")).unwrap(),
        "hello from the agent\n"
    );
    assert_eq!(
        git(root, &["rev-list", "--count", "main"]),
        "2",
        "one squash commit"
    );
    assert_eq!(
        git(root, &["log", "-1", "--format=%s"]),
        "Add a greeting file"
    );
    assert!(git(root, &["status", "--porcelain"]).is_empty());
    assert!(!worktree.exists(), "worktree removed");
    assert!(
        git(root, &["branch", "--list", "archipel/*"]).is_empty(),
        "branch deleted"
    );
    assert_eq!(
        env.engine.snapshot(id).unwrap().outcome,
        Some(Outcome::Squash)
    );
    assert!(
        env.engine.merge(id, Outcome::Squash).is_err(),
        "handled once"
    );
    assert!(env.engine.process_ids(id).is_empty());
}

#[test]
fn permission_round_trip_allow_and_deny() {
    let mut env = Env::new("claude_permission.jsonl", None);
    let a = env.launch("Write the notes", AgentKind::Claude, PermissionMode::Ask);
    env.wait_state(a, State::NeedsInput);
    assert!(env.news.contains(&News::NeedsInput(a)));
    let snap = env.engine.snapshot(a).unwrap();
    assert_eq!(snap.question.as_deref(), Some("Allow Write notes.txt?"));
    assert!(env.engine.events(a).iter().any(|e| e.is_open_question()));
    env.engine.send(a, "y");
    assert_eq!(env.engine.snapshot(a).unwrap().state, State::Working);
    env.wait_state(a, State::Done);
    let answered = env.engine.events(a).iter().any(
        |e| matches!(e, AgentEvent::PermissionRequest { answer: Some(x), .. } if x == "Allow once"),
    );
    assert!(answered);
    let echoed = env
        .texts(a)
        .into_iter()
        .find(|t| t.starts_with("echo: {"))
        .expect("control response echoed");
    let body: serde_json::Value =
        serde_json::from_str(echoed.trim_start_matches("echo: ")).unwrap();
    assert_eq!(body["type"], "control_response");
    assert_eq!(body["response"]["request_id"], "req-1");
    assert_eq!(body["response"]["response"]["behavior"], "allow");

    let b = env.launch(
        "Write the notes again",
        AgentKind::Claude,
        PermissionMode::Ask,
    );
    env.wait_state(b, State::NeedsInput);
    // Typing instead of clicking declines the card and sends the text on.
    env.engine.send(b, "no, use a different file\r");
    env.wait("deny echo", |e| {
        e.events(b)
            .iter()
            .any(|x| matches!(x, AgentEvent::AssistantText(t) if t.contains("\"deny\"")))
    });
    assert!(env.engine.events(b).iter().any(|e| {
        matches!(e, AgentEvent::PermissionRequest { answer: Some(x), .. } if x == "Deny")
    }));
}

#[test]
fn errors_become_failed_sessions_with_the_reason() {
    let mut env = Env::new("claude_error.jsonl", None);
    let a = env.launch("Do it", AgentKind::Claude, PermissionMode::AutoEdit);
    env.wait_state(a, State::Failed);
    assert!(env.news.contains(&News::Failed(a)));
    assert!(
        env.engine
            .events(a)
            .iter()
            .any(|e| matches!(e, AgentEvent::Error(m) if m.contains("401")))
    );

    env.script("claude_crash.jsonl", None);
    let b = env.launch("Crash", AgentKind::Claude, PermissionMode::AutoEdit);
    env.wait_state(b, State::Failed);
    let message = env.engine.events(b).iter().find_map(|e| match e {
        AgentEvent::Error(m) => Some(m.clone()),
        _ => None,
    });
    let message = message.expect("error row");
    assert!(message.contains("exited with code 2"), "{message}");
    assert!(
        message.contains("fatal: the agent crashed"),
        "stderr tail is shown: {message}"
    );

    env.script("codex_failed.jsonl", None);
    let c = env.launch("Codex fails", AgentKind::Codex, PermissionMode::Ask);
    env.wait_state(c, State::Failed);
    let errors: Vec<_> = env
        .engine
        .events(c)
        .iter()
        .filter(|e| matches!(e, AgentEvent::Error(_)))
        .collect();
    assert_eq!(errors.len(), 1, "the same error is not shown twice");
}

#[test]
fn a_missing_cli_and_the_parallel_limit_fail_with_a_reason() {
    let mut env = Env::new("claude_slow.jsonl", None);
    let mut cfg = config(env.data.path(), "claude_slow.jsonl", None);
    cfg.claude_path = Some(PathBuf::from("/definitely/not/claude"));
    env.engine.set_config(cfg);
    assert_eq!(env.engine.available(), [AgentKind::Codex]);
    let a = env.launch("x", AgentKind::Claude, PermissionMode::Ask);
    assert_eq!(env.engine.snapshot(a).unwrap().state, State::Failed);
    assert!(
        env.engine
            .events(a)
            .iter()
            .any(|e| matches!(e, AgentEvent::Error(m) if m.contains("not found")))
    );

    let mut cfg = config(env.data.path(), "claude_slow.jsonl", None);
    cfg.max_parallel = 1;
    env.engine.set_config(cfg);
    env.engine
        .launch("one two", &[AgentKind::Claude, AgentKind::Codex], None);
    let sessions = env.engine.sessions();
    let (first, second) = (&sessions[sessions.len() - 2], &sessions[sessions.len() - 1]);
    assert_eq!(first.state, State::Working);
    assert_eq!(second.state, State::Failed);
    assert!(
        env.engine
            .events(second.id)
            .iter()
            .any(|e| matches!(e, AgentEvent::Error(m) if m.contains("limit")))
    );
    let first_id = first.id;
    env.engine.stop(first_id);
}

#[test]
fn spawn_depth_is_limited_and_children_follow_their_parent() {
    let mut env = Env::new("claude_slow.jsonl", None);
    let mut cfg = config(env.data.path(), "claude_slow.jsonl", None);
    cfg.max_parallel = 8;
    cfg.max_spawn_depth = 2;
    env.engine.set_config(cfg);
    let root = env.launch("top level", AgentKind::Claude, PermissionMode::Ask);
    let child = env
        .engine
        .launch_child(root, "child task", AgentKind::Claude)
        .unwrap();
    let grandchild = env
        .engine
        .launch_child(child, "grandchild task", AgentKind::Claude)
        .unwrap();
    let err = env
        .engine
        .launch_child(grandchild, "too deep", AgentKind::Claude)
        .unwrap_err();
    assert!(err.contains("depth"), "{err}");
    let order: Vec<_> = env
        .engine
        .sessions()
        .iter()
        .map(|s| (s.id, s.parent))
        .collect();
    assert_eq!(
        order,
        [(root, None), (child, Some(root)), (grandchild, Some(child))]
    );
    assert!(
        env.engine
            .events(root)
            .iter()
            .any(|e| matches!(e, AgentEvent::SubagentSpawned { child: c, .. } if *c == child))
    );
    let news = env.engine.poll();
    assert!(news.contains(&News::Spawned {
        parent: root,
        child
    }));
    // Parallel limit counts spawned agents too.
    let mut cfg = config(env.data.path(), "claude_slow.jsonl", None);
    cfg.max_parallel = 3;
    env.engine.set_config(cfg);
    assert!(
        env.engine
            .launch_child(root, "one more", AgentKind::Claude)
            .unwrap_err()
            .contains("limit")
    );
    for id in [root, child, grandchild] {
        env.engine.stop(id);
    }
}

#[test]
fn stop_is_graceful_and_a_deaf_agent_is_killed_after_the_grace_period() {
    let mut env = Env::new("claude_slow.jsonl", None);
    let a = env.launch("polite", AgentKind::Claude, PermissionMode::AutoEdit);
    env.wait("first output", |e| {
        !e.events(a)
            .iter()
            .all(|x| matches!(x, AgentEvent::UserPrompt(_)))
    });
    let pid = env.engine.process_ids(a)[0];
    assert!(pid_alive(pid));
    env.engine.stop(a);
    assert_eq!(env.engine.snapshot(a).unwrap().state, State::Stopped);
    assert!(
        env.engine
            .events(a)
            .iter()
            .any(|e| matches!(e, AgentEvent::Error(m) if m.contains("Interrupted")))
    );
    env.wait("the process to end", |_| !pid_alive(pid));

    // A process that ignores SIGTERM survives the grace period only until the kill.
    let mut cfg = config(env.data.path(), "claude_slow.jsonl", None);
    cfg.extra_env.push(("FAKE_AGENT_DEAF".into(), "1".into()));
    cfg.stop_grace = Duration::from_millis(600);
    env.engine.set_config(cfg);
    let b = env.launch("stubborn", AgentKind::Claude, PermissionMode::AutoEdit);
    env.wait("first output", |e| {
        !e.events(b)
            .iter()
            .all(|x| matches!(x, AgentEvent::UserPrompt(_)))
    });
    let pid = env.engine.process_ids(b)[0];
    env.engine.stop(b);
    std::thread::sleep(Duration::from_millis(250));
    assert!(
        pid_alive(pid),
        "SIGTERM is ignored, the agent lives until the grace period ends"
    );
    env.wait("the kill", |_| !pid_alive(pid));
}

#[test]
fn codex_turn_follow_up_resumes_the_thread_and_discard_cleans_up() {
    let mut env = Env::new("codex_turn.jsonl", Some("codex_resume.jsonl"));
    let id = env.launch("Add codex.txt", AgentKind::Codex, PermissionMode::AutoEdit);
    env.wait_state(id, State::Done);
    let events = env.engine.events(id);
    let rows = tool_rows(events);
    assert!(rows.contains(&(ToolKind::Bash, "Bash ls".into(), ToolStatus::Ok)));
    assert!(rows.contains(&(ToolKind::Write, "Write codex.txt".into(), ToolStatus::Ok)));
    assert!(
        rows.iter().any(|(_, t, _)| t == "Item hologram"),
        "unknown item kept as a generic row"
    );
    assert!(events.iter().any(|e| matches!(e, AgentEvent::Thinking(_))));
    assert!(env.texts(id).contains(&"Added codex.txt.".to_string()));
    assert_eq!(
        env.engine.provider_session(id).as_deref(),
        Some("0199-thread-1")
    );
    assert_eq!(env.engine.snapshot(id).unwrap().tokens, 150);
    env.wait("changes", |e| e.changes(id).len() == 1);
    assert_eq!(env.engine.changes(id)[0].path, PathBuf::from("codex.txt"));

    // The next prompt is a new `codex exec resume` process.
    env.engine.send(id, "and one more thing\r");
    env.wait("resumed turn", |e| {
        e.snapshot(id).is_some_and(|s| s.state == State::Done)
            && e.events(id)
                .iter()
                .any(|x| matches!(x, AgentEvent::AssistantText(t) if t == "Follow-up handled."))
    });
    let args = env
        .texts(id)
        .into_iter()
        .find(|t| t.starts_with("args:"))
        .unwrap();
    assert!(
        args.contains("exec --json resume 0199-thread-1 -- and one more thing"),
        "{args}"
    );
    assert!(args.contains("approval_policy=\"never\""), "{args}");
    assert!(args.contains("sandbox_mode=\"workspace-write\""), "{args}");

    let worktree = env.engine.snapshot(id).unwrap().worktree;
    let message = env.engine.merge(id, Outcome::Discard).unwrap();
    assert!(message.starts_with("Discarded"));
    assert!(!worktree.exists());
    assert!(!env.repo.path().join("codex.txt").exists());
    assert!(git(env.repo.path(), &["branch", "--list", "archipel/*"]).is_empty());
    assert_eq!(git(env.repo.path(), &["rev-list", "--count", "main"]), "1");
}

#[test]
fn merge_refuses_a_dirty_or_wrong_branch_and_leaves_everything_in_place() {
    let mut env = Env::new("codex_turn.jsonl", None);
    let id = env.launch("Add codex.txt", AgentKind::Codex, PermissionMode::AutoEdit);
    env.wait_state(id, State::Done);
    let root = env.repo.path().to_path_buf();
    std::fs::write(root.join("README.md"), "# edited\n").unwrap();
    let err = env.engine.merge(id, Outcome::Squash).unwrap_err();
    assert!(err.contains("uncommitted"), "{err}");
    git(&root, &["checkout", "-q", "--", "README.md"]);
    git(&root, &["checkout", "-q", "-b", "other"]);
    let err = env.engine.merge(id, Outcome::Merge).unwrap_err();
    assert!(err.contains("check out `main`"), "{err}");
    git(&root, &["checkout", "-q", "main"]);
    assert!(env.engine.snapshot(id).unwrap().outcome.is_none());
    let message = env.engine.merge(id, Outcome::Merge).unwrap();
    assert!(
        message.starts_with("Merged add-codextxt into main"),
        "{message}"
    );
    assert_eq!(
        git(&root, &["log", "-1", "--format=%p"]).split(' ').count(),
        2,
        "a merge commit"
    );
    assert!(root.join("codex.txt").exists());
}

#[test]
fn restart_makes_running_sessions_stopped_and_resume_reuses_the_agent_session() {
    let mut env = Env::new("claude_slow.jsonl", Some("claude_resume.jsonl"));
    let id = env.launch(
        "Long running job",
        AgentKind::Claude,
        PermissionMode::AutoEdit,
    );
    env.wait("first output", |e| {
        e.events(id)
            .iter()
            .any(|x| matches!(x, AgentEvent::AssistantText(_)))
    });
    let provider = env
        .engine
        .provider_session(id)
        .expect("pre-assigned session id");
    let before: Vec<AgentEvent> = env.engine.events(id).to_vec();
    let Env {
        repo, data, engine, ..
    } = env;

    // Graceful close: the IDE stops its agents.
    drop(engine);
    // Simulated crash on top: the stored state says "working" again.
    let meta_path = std::fs::read_dir(data.path().join("archipel/agents"))
        .unwrap()
        .flatten()
        .next()
        .unwrap()
        .path()
        .join(format!("sessions/{id}/meta.json"));
    let stored = std::fs::read_to_string(&meta_path).unwrap();
    assert!(stored.contains("\"state\": \"stopped\""), "{stored}");
    assert!(!stored.contains("ANTHROPIC"), "no environment in metadata");
    std::fs::write(
        &meta_path,
        stored.replace("\"state\": \"stopped\"", "\"state\": \"working\""),
    )
    .unwrap();

    let mut engine = Engine::new(config(
        data.path(),
        "claude_slow.jsonl",
        Some("claude_resume.jsonl"),
    ));
    engine.set_project(Some(repo.path()));
    let snap = engine.snapshot(id).expect("session restored");
    assert_eq!(snap.state, State::Stopped);
    assert_eq!(snap.name, "long-running-job");
    assert_eq!(
        engine.provider_session(id).as_deref(),
        Some(provider.as_str())
    );
    let events = engine.events(id).to_vec();
    assert!(events.starts_with(&before), "the transcript is replayed");
    let stops = events
        .iter()
        .filter(|e| matches!(e, AgentEvent::Error(m) if m.contains("Stopped when the IDE closed")))
        .count();
    // One from the graceful close, one from the simulated crash.
    assert_eq!(stops, 2);
    assert_eq!(engine.tasks()[0].sessions, [id]);
    drop(engine);

    let mut env = Env {
        repo,
        data,
        engine: Engine::new(config(&PathBuf::new(), "x", None)),
        news: Vec::new(),
    };
    env.engine = Engine::new(config(
        env.data.path(),
        "claude_slow.jsonl",
        Some("claude_resume.jsonl"),
    ));
    env.engine.set_project(Some(env.repo.path()));
    assert_eq!(
        env.engine
            .events(id)
            .iter()
            .filter(
                |e| matches!(e, AgentEvent::Error(m) if m.contains("Stopped when the IDE closed"))
            )
            .count(),
        2,
        "a plain restart adds no notice"
    );

    // Sending input resumes the same agent session in the same worktree.
    env.engine.send(id, "carry on\r");
    env.wait("resumed", |e| {
        e.events(id)
            .iter()
            .any(|x| matches!(x, AgentEvent::AssistantText(t) if t == "Resumed and continuing."))
    });
    env.wait_state(id, State::Done);
    let args = env
        .texts(id)
        .into_iter()
        .find(|t| t.starts_with("args:"))
        .expect("args echoed");
    assert!(args.contains(&format!("--resume {provider}")), "{args}");
    assert!(!args.contains("--session-id"), "{args}");
    assert!(args.contains("--output-format stream-json"));
    assert!(args.contains("--input-format stream-json"));
    assert!(args.contains("--verbose"));
    assert!(args.contains("--permission-mode acceptEdits"));
    // New ids continue after the restored ones.
    let next = env.launch("another", AgentKind::Claude, PermissionMode::AutoEdit);
    assert!(next > id);
}

#[test]
fn changing_project_stops_agents_and_hides_foreign_sessions() {
    let mut env = Env::new("claude_slow.jsonl", None);
    let id = env.launch("job", AgentKind::Claude, PermissionMode::AutoEdit);
    env.wait("first output", |e| {
        e.events(id)
            .iter()
            .any(|x| matches!(x, AgentEvent::AssistantText(_)))
    });
    let pid = env.engine.process_ids(id)[0];
    let other = tempfile::tempdir().unwrap();
    git(other.path(), &["init", "-q", "-b", "main"]);
    env.engine.set_project(Some(other.path()));
    assert!(env.engine.sessions().is_empty());
    env.wait("old agent to end", |_| !pid_alive(pid));
    env.engine.set_project(Some(env.repo.path()));
    assert_eq!(env.engine.snapshot(id).unwrap().state, State::Stopped);
}
