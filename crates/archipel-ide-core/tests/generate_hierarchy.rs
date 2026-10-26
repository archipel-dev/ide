//! Generate popup, Go to Super Method, Type Hierarchy and Call Hierarchy
//! through the command registry, against the scriptable `archipel-fake-lsp`.

use archipel_ide_core::{HierarchyDirection, IdeState, ToolWindow, built_in_commands};
use archipel_ide_lsp::{
    LanguageServerProvider, LaunchConfig, LaunchContext, LspError, LspManager, ServerRegistry,
    ServerSpec,
};
use archipel_ide_text::Selection;
use serde_json::json;
use std::{
    path::PathBuf,
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

struct Fake(PathBuf, ServerSpec);

impl LanguageServerProvider for Fake {
    fn spec(&self) -> &ServerSpec {
        &self.1
    }
    fn launch(&self, _: &LaunchContext<'_>) -> Result<LaunchConfig, LspError> {
        Ok(LaunchConfig::new("fake", &self.0))
    }
}

fn fake_server() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let path = exe
        .parent()?
        .parent()?
        .join(format!("archipel-fake-lsp{}", std::env::consts::EXE_SUFFIX));
    path.is_file().then_some(path)
}

fn wait(state: &mut IdeState, mut done: impl FnMut(&IdeState) -> bool) -> bool {
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        state.poll_lsp();
        if done(state) {
            return true;
        }
        thread::sleep(Duration::from_millis(10));
    }
    false
}

const SHAPE: &str = "interface Shape {\n    void area();\n}\n";
const BASE: &str =
    "class Base implements Shape {\n    void area() {\n    }\n    void helper() {\n    }\n}\n";
const CIRCLE: &str =
    "class Circle extends Base {\n    void area() {\n        helper();\n    }\n}\n";

/// Opens Shape, Base and Circle (so the fake server knows them) and leaves
/// the last-opened file active.
fn project(server: PathBuf, name: &str) -> (PathBuf, IdeState) {
    let root = std::env::temp_dir().join(format!("archipel-ide-gh-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    for (file, text) in [("Shape", SHAPE), ("Base", BASE), ("Circle", CIRCLE)] {
        std::fs::write(root.join(format!("{file}.fake")), text).unwrap();
    }
    let mut registry = ServerRegistry::empty();
    registry.register(Arc::new(Fake(
        server,
        ServerSpec {
            id: "fake",
            language: "Fake",
            extensions: &[("fake", "fake")],
            file_names: &[],
            candidates: &[],
            root_markers: &[],
            install_hint: "",
            notify_missing: false,
            initialization_options: None,
        },
    )));
    let commands = built_in_commands();
    let mut state = IdeState::default();
    state.lsp = LspManager::new(registry);
    commands
        .execute("project.open", &mut state, json!({"path": root}))
        .unwrap();
    for file in ["Shape", "Base", "Circle"] {
        commands
            .execute(
                "editor.openFile",
                &mut state,
                json!({"path": format!("{file}.fake")}),
            )
            .unwrap();
    }
    assert!(wait(&mut state, |state| {
        state.active_absolute_path().is_some_and(|path| {
            state
                .lsp
                .supports(&path, archipel_ide_lsp::Feature::CodeAction)
        })
    }));
    // Let the opens and the first diagnostics settle.
    for _ in 0..30 {
        state.poll_lsp();
        thread::sleep(Duration::from_millis(10));
    }
    (root, state)
}

/// Opens `file` and puts the caret on the first `word` of `line`.
fn caret_on(state: &mut IdeState, file: &str, line: usize, word: &str) {
    built_in_commands()
        .execute("editor.openFile", state, json!({"path": file}))
        .unwrap();
    let document = &mut state.documents[state.active_document.unwrap()];
    let start = document.text.line_to_char(line).unwrap();
    let text = document.text.line(line).unwrap();
    let column = text.find(word).unwrap();
    document
        .selections
        .set_single(Selection::caret(start + column));
}

fn active(state: &IdeState) -> (String, usize) {
    let document = &state.documents[state.active_document.unwrap()];
    (
        document.path.display().to_string(),
        document
            .text
            .char_to_line(document.selections.primary().head),
    )
}

#[test]
fn generate_popup_lists_the_entries_and_applies_the_servers_action() {
    let Some(server) = fake_server() else {
        eprintln!("archipel-fake-lsp not built; run `cargo test --workspace`");
        return;
    };
    let (root, mut state) = project(server, "generate");
    let commands = built_in_commands();
    caret_on(&mut state, "Circle.fake", 0, "Circle");
    commands
        .execute("code.generate", &mut state, json!({}))
        .unwrap();
    assert!(wait(&mut state, |state| state.code_actions.popup));
    let titles: Vec<String> = state
        .current_code_actions()
        .expect("fresh list")
        .items
        .iter()
        .map(|item| item.title.clone())
        .collect();
    assert_eq!(
        titles,
        [
            "Constructor",
            "Getter",
            "Setter",
            "Getter and Setter",
            "equals() and hashCode()",
            "toString()",
            "Override Methods…"
        ]
    );
    assert!(
        state
            .current_code_actions()
            .unwrap()
            .items
            .iter()
            .all(|item| item.disabled.is_none())
    );
    commands
        .execute("code.applyAction", &mut state, json!({"index": 1}))
        .unwrap();
    assert!(
        state.documents[state.active_document.unwrap()]
            .text
            .text()
            .starts_with("// generated Generate Getters\n")
    );

    // A direct entry skips the popup.
    commands
        .execute("code.generate.toString", &mut state, json!({}))
        .unwrap();
    assert!(wait(&mut state, |state| {
        state.documents[state.active_document.unwrap()]
            .text
            .text()
            .starts_with("// generated Generate toString()...\n")
    }));
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn go_to_super_method_walks_the_supertypes() {
    let Some(server) = fake_server() else {
        eprintln!("archipel-fake-lsp not built; run `cargo test --workspace`");
        return;
    };
    let (root, mut state) = project(server, "super");
    let commands = built_in_commands();
    // Circle.area overrides Base.area, which implements Shape.area.
    caret_on(&mut state, "Circle.fake", 1, "area");
    commands
        .execute("navigate.superMethod", &mut state, json!({}))
        .unwrap();
    assert!(wait(&mut state, |state| active(state).0 == "Base.fake"));
    assert_eq!(active(&state), ("Base.fake".to_string(), 1));
    commands
        .execute("navigate.superMethod", &mut state, json!({}))
        .unwrap();
    assert!(wait(&mut state, |state| active(state).0 == "Shape.fake"));
    assert_eq!(active(&state), ("Shape.fake".to_string(), 1));

    // On a class name it goes to the superclass.
    caret_on(&mut state, "Circle.fake", 0, "Circle");
    commands
        .execute("navigate.superMethod", &mut state, json!({}))
        .unwrap();
    assert!(wait(&mut state, |state| active(state).0 == "Base.fake"));
    assert_eq!(active(&state).1, 0);

    // `helper` overrides nothing.
    caret_on(&mut state, "Base.fake", 3, "helper");
    commands
        .execute("navigate.superMethod", &mut state, json!({}))
        .unwrap();
    assert!(wait(&mut state, |state| state.status == "No super method found"));
    assert_eq!(active(&state), ("Base.fake".to_string(), 3));
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn type_hierarchy_expands_lazily_and_switches_direction() {
    let Some(server) = fake_server() else {
        eprintln!("archipel-fake-lsp not built; run `cargo test --workspace`");
        return;
    };
    let (root, mut state) = project(server, "types");
    let commands = built_in_commands();
    caret_on(&mut state, "Base.fake", 0, "Base");
    commands
        .execute("hierarchy.type", &mut state, json!({}))
        .unwrap();
    assert!(wait(&mut state, |state| {
        state
            .hierarchy
            .current
            .as_ref()
            .is_some_and(|h| h.nodes.first().is_some_and(|n| n.loaded))
    }));
    assert!(state.tool_windows.is_visible(ToolWindow::Hierarchy));
    let hierarchy = state.hierarchy.current.clone().unwrap();
    assert_eq!(hierarchy.direction, HierarchyDirection::Down);
    let labels: Vec<&str> = hierarchy
        .rows()
        .iter()
        .map(|row| hierarchy.nodes[row.node].label.as_str())
        .collect();
    assert_eq!(labels, ["Base", "Circle"]);

    // Supertypes of Base, then open the interface.
    commands
        .execute("hierarchy.toggleDirection", &mut state, json!({}))
        .unwrap();
    assert!(wait(&mut state, |state| {
        state
            .hierarchy
            .current
            .as_ref()
            .is_some_and(|h| h.nodes.len() == 2 && h.nodes[0].loaded)
    }));
    let hierarchy = state.hierarchy.current.clone().unwrap();
    assert_eq!(hierarchy.nodes[1].label, "Shape");
    commands
        .execute("hierarchy.openNode", &mut state, json!({"node": 1}))
        .unwrap();
    assert_eq!(active(&state), ("Shape.fake".to_string(), 0));
    // Collapsing keeps the loaded children.
    commands
        .execute("hierarchy.toggleNode", &mut state, json!({"node": 0}))
        .unwrap();
    let rows = state.hierarchy.current.as_ref().unwrap().rows();
    assert_eq!(rows.len(), 1);
    commands
        .execute("hierarchy.close", &mut state, json!({}))
        .unwrap();
    assert!(state.hierarchy.current.is_none());
    assert!(!state.tool_windows.is_visible(ToolWindow::Hierarchy));
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn call_hierarchy_shows_callers_and_callees() {
    let Some(server) = fake_server() else {
        eprintln!("archipel-fake-lsp not built; run `cargo test --workspace`");
        return;
    };
    let (root, mut state) = project(server, "calls");
    let commands = built_in_commands();
    caret_on(&mut state, "Base.fake", 3, "helper");
    commands
        .execute("hierarchy.call", &mut state, json!({}))
        .unwrap();
    assert!(wait(&mut state, |state| {
        state
            .hierarchy
            .current
            .as_ref()
            .is_some_and(|h| h.nodes.first().is_some_and(|n| n.loaded))
    }));
    let hierarchy = state.hierarchy.current.clone().unwrap();
    assert_eq!(hierarchy.direction, HierarchyDirection::Up);
    assert_eq!(hierarchy.nodes.len(), 2);
    assert_eq!(hierarchy.nodes[1].label, "area");
    let target = hierarchy.nodes[1].target.clone().unwrap();
    assert_eq!(target.path, PathBuf::from("Circle.fake"));
    commands
        .execute("hierarchy.openNode", &mut state, json!({"node": 1}))
        .unwrap();
    assert_eq!(active(&state), ("Circle.fake".to_string(), 1));

    // Callees of Circle.area: helper.
    caret_on(&mut state, "Circle.fake", 1, "area");
    commands
        .execute("hierarchy.call", &mut state, json!({}))
        .unwrap();
    assert!(wait(&mut state, |state| {
        state.hierarchy.current.as_ref().is_some_and(|h| {
            h.nodes
                .first()
                .is_some_and(|n| n.loaded && n.label == "area")
        })
    }));
    commands
        .execute("hierarchy.toggleDirection", &mut state, json!({}))
        .unwrap();
    assert!(wait(&mut state, |state| {
        state
            .hierarchy
            .current
            .as_ref()
            .is_some_and(|h| h.direction == HierarchyDirection::Down && h.nodes.len() == 2)
    }));
    let hierarchy = state.hierarchy.current.clone().unwrap();
    assert_eq!(hierarchy.nodes[1].label, "helper");
    let _ = std::fs::remove_dir_all(root);
}
