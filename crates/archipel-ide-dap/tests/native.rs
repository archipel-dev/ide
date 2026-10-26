//! Debugs a tiny C program with the installed native adapter (lldb-dap or gdb).
//! Skipped with a message when no compiler or adapter is available.

use archipel_ide_dap::{Adapter, DebugClient, SessionState, Step};
use archipel_ide_runner::find_program;
use std::collections::{BTreeMap, BTreeSet};
use std::time::{Duration, Instant};

fn wait(client: &mut DebugClient, what: &str, done: impl Fn(&DebugClient) -> bool) {
    let deadline = Instant::now() + Duration::from_secs(60);
    while !done(client) {
        assert!(
            Instant::now() < deadline,
            "timed out waiting for {what}: {:?}",
            client.session().state()
        );
        client.poll();
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn stops_at_breakpoint_shows_locals_and_steps() {
    let Some(compiler) = ["cc", "gcc", "clang"]
        .into_iter()
        .find(|c| find_program(c).is_some())
    else {
        eprintln!("skipping: no C compiler on PATH");
        return;
    };
    let adapter = match Adapter::discover_native() {
        Ok(adapter) => adapter,
        Err(error) => {
            eprintln!("skipping: {error}");
            return;
        }
    };
    let dir = std::env::temp_dir().join(format!("archipel-ide-dap-native-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let source = dir.join("main.c");
    std::fs::write(
        &source,
        "#include <stdio.h>\nint main(void) {\n    int answer = 41;\n    answer = answer + 1;\n    printf(\"answer=%d\\n\", answer);\n    return 0;\n}\n",
    )
    .unwrap();
    let program = dir.join("app");
    let status = std::process::Command::new(compiler)
        .args(["-g", "-O0", "-o"])
        .arg(&program)
        .arg(&source)
        .status()
        .unwrap();
    assert!(status.success());

    let mut breakpoints = BTreeMap::new();
    breakpoints.insert(source.clone(), BTreeSet::from([3])); // `answer = answer + 1;`
    let launch = adapter.launch_request(&program, &[], &dir, &[]);
    let mut client = DebugClient::start(adapter, launch, breakpoints, &dir).unwrap();
    wait(&mut client, "breakpoint stop", |c| {
        matches!(c.session().state(), SessionState::Stopped { .. })
            && c.session().selected_frame().is_some()
            || c.session().is_terminated()
    });
    assert!(
        !client.session().is_terminated(),
        "{:?} {:?}",
        client.session().error(),
        client.session().output()
    );
    let frame = client.session().selected_frame().unwrap().clone();
    assert_eq!(frame.line, 4, "{frame:?}");
    assert!(frame.name.contains("main"), "{frame:?}");
    wait(&mut client, "locals", |c| {
        c.session()
            .scopes()
            .first()
            .and_then(|scope| c.session().variables(scope.reference))
            .is_some()
    });
    let locals = client.session().scopes()[0].reference;
    let answer = client
        .session()
        .variables(locals)
        .unwrap()
        .iter()
        .find(|variable| variable.name == "answer")
        .cloned()
        .expect("local `answer`");
    assert_eq!(answer.value, "41");
    client.with_session(|session| session.step(Step::Next));
    wait(&mut client, "step", |c| {
        c.session()
            .selected_frame()
            .is_some_and(|frame| frame.line == 5)
    });
    client.with_session(|session| session.step(Step::Continue));
    wait(&mut client, "exit", |c| c.session().is_terminated());
    let output: String = client
        .session()
        .output()
        .iter()
        .map(|line| line.text.as_str())
        .collect();
    assert!(output.contains("answer=42"), "{output}");
    let _ = std::fs::remove_dir_all(&dir);
}
