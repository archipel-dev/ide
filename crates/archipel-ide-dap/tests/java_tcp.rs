//! java-debug runs inside jdtls and speaks DAP over a localhost TCP port. A scripted
//! server stands in for it: the client must connect, launch with the Java arguments,
//! configure breakpoints and report the stop with its frame.

use archipel_ide_dap::transport::{read_message, write_message};
use archipel_ide_dap::{Adapter, DebugClient, JavaLaunch, SessionState};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::io::BufReader;
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};

fn body_for(command: &str) -> Value {
    match command {
        "initialize" => json!({"supportsConfigurationDoneRequest": true}),
        "threads" => json!({"threads": [{"id": 1, "name": "main"}]}),
        "stackTrace" => json!({"stackFrames": [{
            "id": 7, "name": "App.main(String[])", "line": 12, "column": 1,
            "source": {"path": "/w/src/main/java/com/example/App.java"}
        }]}),
        "scopes" => json!({"scopes": [{"name": "Local", "variablesReference": 3}]}),
        "variables" => json!({"variables": [
            {"name": "args", "value": "String[0]@21", "type": "String[]", "variablesReference": 0}
        ]}),
        "setBreakpoints" => json!({"breakpoints": [{"verified": true, "line": 12}]}),
        _ => json!({}),
    }
}

#[test]
fn connects_to_java_debug_over_tcp_and_reaches_a_breakpoint() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let (seen, requests) = mpsc::channel();
    std::thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let mut writer = stream.try_clone().unwrap();
        let mut reader = BufReader::new(stream);
        let mut seq = 1;
        let mut send = |writer: &mut std::net::TcpStream, message: Value| {
            seq += 1;
            let mut message = message;
            message["seq"] = json!(seq);
            write_message(writer, &message).unwrap();
        };
        while let Ok(Some(request)) = read_message(&mut reader) {
            let command = request["command"].as_str().unwrap_or_default().to_owned();
            let _ = seen.send(request.clone());
            send(
                &mut writer,
                json!({"type": "response", "request_seq": request["seq"], "success": true,
                       "command": command, "body": body_for(&command)}),
            );
            match command.as_str() {
                "launch" => send(
                    &mut writer,
                    json!({"type": "event", "event": "initialized"}),
                ),
                "configurationDone" => send(
                    &mut writer,
                    json!({"type": "event", "event": "stopped", "body":
                        {"reason": "breakpoint", "threadId": 1, "allThreadsStopped": true}}),
                ),
                _ => {}
            }
        }
    });

    let launch = JavaLaunch {
        main_class: "com.example.App".into(),
        project_name: Some("app".into()),
        class_paths: vec!["/w/build/classes/java/main".into()],
        module_paths: Vec::new(),
        args: Vec::new(),
        vm_args: Vec::new(),
        cwd: PathBuf::from("/w"),
        env: Vec::new(),
    }
    .request();
    let breakpoints = BTreeMap::from([(
        PathBuf::from("/w/src/main/java/com/example/App.java"),
        BTreeSet::from([11]),
    )]);
    let mut client = DebugClient::start(
        Adapter::Java { port },
        launch,
        breakpoints,
        &std::env::temp_dir(),
    )
    .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while !(matches!(client.session().state(), SessionState::Stopped { .. })
        && client.session().selected_frame().is_some())
    {
        assert!(
            Instant::now() < deadline,
            "no stop: {:?}",
            client.session().error()
        );
        client.poll();
        std::thread::sleep(Duration::from_millis(10));
    }
    let frame = client.session().selected_frame().unwrap();
    assert_eq!(frame.line, 12);
    let launch: Vec<Value> = requests
        .try_iter()
        .filter(|request| request["command"] == "launch")
        .collect();
    assert_eq!(launch[0]["arguments"]["mainClass"], "com.example.App");
    client.kill();
}
