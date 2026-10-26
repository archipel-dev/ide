//! A stand-in for the `claude` and `codex` CLIs, used by the engine's end-to-end tests.
//!
//! It replays a fixture (`FAKE_AGENT_SCRIPT`, or `FAKE_AGENT_RESUME_SCRIPT` when the arguments
//! ask to resume) to stdout. A fixture line is printed as is (after `@CWD@` is replaced by the
//! working directory) unless it starts with `#`:
//!
//! - `#read`: wait for one stdin line and drop it (the prompt);
//! - `#echo`: wait for one stdin line and print it as an assistant text `echo: <line>`;
//! - `#init`: print a Claude `system/init` with the session id taken from the arguments;
//! - `#args` / `#args-codex`: print the command-line arguments as an assistant message;
//! - `#write <path>|<text>`: create a file in the working directory (`\n` is a newline);
//! - `#sleep <ms>`, `#stderr <text>`, `#exit <code>`, `#hang`;
//! - `#serve`: answer every further stdin line with `echo: <text>` and a successful result.
//!
//! `FAKE_AGENT_DEAF=1` makes the process ignore SIGTERM (it re-executes itself through a shell
//! that sets the signal to ignored), to exercise the kill after the grace period.

use serde_json::{Value, json};
use std::io::{self, BufRead, Write};
use std::time::Duration;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if std::env::var_os("FAKE_AGENT_DEAF").is_some()
        && std::env::var_os("FAKE_AGENT_REEXEC").is_none()
    {
        reexec_deaf(&args);
    }
    let resume = args.iter().any(|a| a == "--resume" || a == "resume");
    let script_var = if resume && std::env::var_os("FAKE_AGENT_RESUME_SCRIPT").is_some() {
        "FAKE_AGENT_RESUME_SCRIPT"
    } else {
        "FAKE_AGENT_SCRIPT"
    };
    let Some(path) = std::env::var_os(script_var) else {
        eprintln!("fake-agent: {script_var} is not set");
        std::process::exit(64);
    };
    let script = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        eprintln!("fake-agent: cannot read {}: {e}", path.to_string_lossy());
        std::process::exit(65);
    });
    let cwd = std::env::current_dir().unwrap_or_default();
    let cwd_text = cwd.to_string_lossy().replace('\\', "/");
    let stdin = io::stdin();
    let mut input = stdin.lock();
    let mut out = io::stdout();
    let mut emit = |line: &str| {
        let _ = writeln!(out, "{line}");
        let _ = out.flush();
    };
    let session_id = args
        .windows(2)
        .find(|w| w[0] == "--session-id" || w[0] == "--resume")
        .map(|w| w[1].clone())
        .unwrap_or_else(|| "fake-session".into());

    for raw in script.lines() {
        let line = raw.trim_end().replace("@CWD@", &cwd_text);
        if line.is_empty() {
            continue;
        }
        let Some(directive) = line.strip_prefix('#') else {
            emit(&line);
            continue;
        };
        let (name, rest) = directive.split_once(' ').unwrap_or((directive, ""));
        match name {
            "read" => {
                if read_line(&mut input).is_none() {
                    return;
                }
            }
            "echo" => {
                let Some(text) = read_line(&mut input) else {
                    return;
                };
                emit(&echo(&text));
            }
            "init" => emit(
                &json!({"type":"system","subtype":"init","session_id":session_id,"model":"fake-model","cwd":cwd_text})
                    .to_string(),
            ),
            "args" => emit(&assistant(&format!("args: {}", args.join(" ")))),
            "args-codex" => emit(
                &json!({"type":"item.completed","item":{"id":"args","type":"agent_message","text":format!("args: {}", args.join(" "))}})
                    .to_string(),
            ),
            "write" => {
                if let Some((file, text)) = rest.split_once('|') {
                    let target = cwd.join(file);
                    if let Some(parent) = target.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    let _ = std::fs::write(target, text.replace("\\n", "\n"));
                }
            }
            "sleep" => std::thread::sleep(Duration::from_millis(rest.trim().parse().unwrap_or(0))),
            "stderr" => eprintln!("{rest}"),
            "exit" => std::process::exit(rest.trim().parse().unwrap_or(0)),
            "hang" => loop {
                std::thread::sleep(Duration::from_secs(1));
            },
            "serve" => {
                while let Some(text) = read_line(&mut input) {
                    emit(&assistant(&format!("echo: {text}")));
                    emit(
                        &json!({"type":"result","subtype":"success","is_error":false,"duration_ms":5,"total_cost_usd":0.001,"usage":{"input_tokens":3,"output_tokens":2}})
                            .to_string(),
                    );
                }
                return;
            }
            _ => emit(&assistant(&format!("unknown directive {name}"))),
        }
    }
}

fn assistant(text: &str) -> String {
    json!({"type":"assistant","message":{"content":[{"type":"text","text":text}]}}).to_string()
}

fn echo(line: &str) -> String {
    assistant(&format!("echo: {line}"))
}

/// One stdin line; for a Claude user message the text of its first content block.
fn read_line(input: &mut impl BufRead) -> Option<String> {
    let mut line = String::new();
    match input.read_line(&mut line) {
        Ok(0) | Err(_) => None,
        Ok(_) => {
            let line = line.trim_end().to_owned();
            match serde_json::from_str::<Value>(&line) {
                Ok(v) => match v.pointer("/message/content/0/text").and_then(Value::as_str) {
                    Some(text) => Some(text.to_owned()),
                    None => Some(line),
                },
                Err(_) => Some(line),
            }
        }
    }
}

#[cfg(unix)]
fn reexec_deaf(args: &[String]) -> ! {
    use std::os::unix::process::CommandExt;
    let exe = std::env::current_exe().unwrap_or_default();
    let error = std::process::Command::new("sh")
        .arg("-c")
        .arg("trap '' TERM; exec \"$0\" \"$@\"")
        .arg(exe)
        .args(args)
        .env("FAKE_AGENT_REEXEC", "1")
        .exec();
    eprintln!("fake-agent: re-exec failed: {error}");
    std::process::exit(66);
}

#[cfg(not(unix))]
fn reexec_deaf(_args: &[String]) -> ! {
    std::process::exit(67);
}
