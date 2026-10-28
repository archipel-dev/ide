//! Fixtures: an IDE core over small temporary projects.

#![allow(dead_code)]

use archipel_ide_command::CommandRegistry;
use archipel_ide_core::{IdeState, built_in_commands};
use archipel_ide_view::{Caps, UiData, ViewCx};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

/// A started core with its commands.
pub struct Fixture {
    pub state: IdeState,
    pub commands: CommandRegistry<IdeState>,
    pub ui: UiData,
    pub dir: Option<PathBuf>,
}

impl Fixture {
    /// A core without a project.
    pub fn empty() -> Self {
        let mut state = IdeState::default();
        state.init_plugins(
            archipel_ide_plugins::PluginStore::default(),
            archipel_ide_plugins::PluginSet::default(),
        );
        let mut fixture = Self {
            state,
            commands: built_in_commands(),
            ui: UiData::default(),
            dir: None,
        };
        fixture.register();
        fixture
    }

    fn register(&mut self) {
        archipel_ide_view::actions::register(self.state.action_registry());
        let mut actions = archipel_ide_core::ide_actions::CommandActions::default();
        let _ = actions.sync(self.state.application().extensions(), &self.commands);
        archipel_ide_view::actions::register_context_rules(self.state.action_registry());
    }

    /// A core with a Java Gradle project of two files (one opened dirty).
    pub fn java(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("archipel-view-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("src/main/java/com/example")).expect("mkdir");
        std::fs::write(dir.join("build.gradle.kts"), "plugins { java }\n").expect("write");
        std::fs::write(
            dir.join("src/main/java/com/example/Main.java"),
            "package com.example;\npublic class Main {\n    public static void main(String[] a) {}\n}\n",
        )
        .expect("write");
        std::fs::write(dir.join("README.md"), "# demo\n").expect("write");
        let mut fixture = Self::empty();
        fixture.run("project.open", json!({ "path": dir }));
        fixture.run("editor.openFile", json!({"path": "src/main/java/com/example/Main.java"}));
        fixture.run("editor.openFile", json!({"path": "README.md"}));
        fixture.dir = Some(dir);
        fixture
    }

    /// Runs a command; failures are ignored (a fixture state is what it is).
    pub fn run(&mut self, id: &str, params: Value) {
        let _ = self.commands.execute(id, &mut self.state, params);
    }

    /// A view context with `caps`.
    pub fn cx(&self, caps: Caps) -> ViewCx<'_> {
        ViewCx::new(&self.state, &self.commands, &self.ui, caps)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        if let Some(dir) = &self.dir {
            let _ = std::fs::remove_dir_all(dir);
        }
    }
}

pub fn path(p: &str) -> &Path {
    Path::new(p)
}
