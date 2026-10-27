//! End-to-end checks against a real Eclipse JDT LS (ignored by default).
//!
//! Needs jdtls installed through the toolchain store (`archipel-cli run
//! toolchain.install '{"component":"jdtls","wait":true}'`), a JDK 21+ and
//! network access for the build tool's dependencies. Run with
//! `ARCHIPEL_REAL_JDTLS_PROJECT=<dir> cargo test -p archipel-ide-core --test
//! real_jdtls -- --ignored --nocapture`.

use archipel_ide_core::{IdeState, built_in_commands};
use serde_json::json;
use std::time::{Duration, Instant};

#[test]
#[ignore = "requires jdtls, a JDK and network access"]
fn probe() {
    let Ok(project) = std::env::var("ARCHIPEL_REAL_JDTLS_PROJECT") else {
        return;
    };
    let commands = built_in_commands();
    let mut state = IdeState::default();
    commands
        .execute("project.open", &mut state, json!({"path": project}))
        .unwrap();
    commands
        .execute(
            "editor.openFile",
            &mut state,
            json!({"path": "src/test/java/com/example/AppTest.java"}),
        )
        .unwrap();
    {
        let document = &mut state.documents[state.active_document.unwrap()];
        let at = document
            .text
            .text()
            .find(&std::env::var("PROBE_IDENT").unwrap_or("assertThat".into()))
            .unwrap()
            + 3;
        document
            .selections
            .set_single(archipel_ide_text::Selection::caret(at));
    }
    let start = Instant::now();
    let mut asked = false;
    let mut last = String::new();
    while start.elapsed() < Duration::from_secs(150) {
        state.poll_lsp();
        state.poll_analysis();
        let servers: Vec<_> = state
            .lsp
            .servers()
            .iter()
            .map(|s| format!("{:?} {:?} {:?}", s.status, s.progress, s.message))
            .collect();
        let problems = state.active_problems();
        let line = format!(
            "{servers:?} problems={:?}",
            problems
                .iter()
                .map(|p| p.message.clone())
                .collect::<Vec<_>>()
        );
        if line != last {
            eprintln!("[{:>5.1}s] {line}", start.elapsed().as_secs_f32());
            last = line;
        }
        if !asked
            && !problems.is_empty()
            && state.lsp.servers().iter().all(|s| s.progress.is_empty())
        {
            asked = true;
            state.request_code_actions(true).unwrap();
        }
        if asked && let Some(list) = state.code_actions.list.clone() {
            eprintln!("status {}", state.status);
            for item in &list.items {
                eprintln!("ACTION {:?} {:?} {}", item.kind, item.title, item.quick_fix);
            }
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}
