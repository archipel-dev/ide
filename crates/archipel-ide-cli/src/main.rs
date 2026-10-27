use archipel_ide_control::{ControlClient, PointerButton, Request, WaitCondition};
use archipel_ide_core::{IdeState, built_in_commands};
use serde_json::Value;
use std::path::PathBuf;

const UI_USAGE: &str = "usage: archipel-cli ui [--socket PATH] <request>
requests:
  execute <command-id> [json-args]
  state
  key <chord>                  e.g. Ctrl+Shift+A, Alt+1, Escape
  type <text>
  click <x> <y> [--button primary|secondary|middle] [--count N]
  move <x> <y>
  screenshot <path.png>
  wait <idle|popup|text|analysis> [text] [--timeout MS]
  raw <json-request>
The socket defaults to $ARCHIPEL_CONTROL_SOCKET.";

fn main() {
    let first = std::env::args_os().nth(1);
    if let Some(text) = archipel_ide_core::build_info::cli_flag_output(
        "archipel-cli",
        "usage: archipel-cli <commands|run|ui> …\n  -V, --version   print version and build information",
        first.as_deref().and_then(|arg| arg.to_str()),
    ) {
        println!("{text}");
        return;
    }
    let mut args = std::env::args().skip(1);
    let mode = args.next().unwrap_or_else(|| "commands".into());
    let registry = built_in_commands();
    if mode == "commands" {
        println!(
            "{}",
            serde_json::to_string_pretty(&registry.specs()).expect("specs serialize")
        );
        return;
    }
    if mode == "run" {
        let Some(id) = args.next() else {
            eprintln!("usage: archipel-cli run <command-id> [json-params]");
            std::process::exit(2);
        };
        let params = parse_json(args.next());
        let mut state = IdeState::default();
        match registry.execute(&id, &mut state, params) {
            Ok(value) => println!("{value}"),
            Err(error) => {
                eprintln!("{error}");
                std::process::exit(1);
            }
        }
        return;
    }
    if mode == "ui" {
        ui(args.collect());
        return;
    }
    eprintln!("usage: archipel-cli <commands|run|ui>");
    std::process::exit(2);
}

fn parse_json(arg: Option<String>) -> Value {
    arg.map(|s| serde_json::from_str(&s))
        .transpose()
        .unwrap_or_else(|e| {
            eprintln!("invalid JSON: {e}");
            std::process::exit(2)
        })
        .unwrap_or(Value::Null)
}

fn ui(args: Vec<String>) {
    let (socket, request) = match parse_ui_args(args) {
        Ok(parsed) => parsed,
        Err(message) => {
            eprintln!("{message}\n{UI_USAGE}");
            std::process::exit(2);
        }
    };
    let response = ControlClient::connect(&socket).and_then(|mut client| client.send(&request));
    match response {
        Ok(response) if response.ok => match response.result.unwrap_or(Value::Null) {
            Value::Null => {}
            value => println!(
                "{}",
                serde_json::to_string_pretty(&value).unwrap_or_default()
            ),
        },
        Ok(response) => {
            eprintln!(
                "{}",
                response.error.unwrap_or_else(|| "request failed".into())
            );
            std::process::exit(1);
        }
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}

fn parse_ui_args(args: Vec<String>) -> Result<(PathBuf, Request), String> {
    let mut socket =
        archipel_ide_compat::env_var_os(archipel_ide_control::SOCKET_ENV).map(PathBuf::from);
    let mut timeout_ms = None;
    let mut button = PointerButton::Primary;
    let mut count = 1u8;
    let mut positional = Vec::new();
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--socket" => {
                socket = Some(args.next().ok_or("--socket needs a path")?.into());
            }
            "--timeout" => {
                let value = args.next().ok_or("--timeout needs milliseconds")?;
                timeout_ms = Some(
                    value
                        .parse::<u64>()
                        .map_err(|_| format!("invalid --timeout {value:?}"))?,
                );
            }
            "--button" => {
                button = match args.next().as_deref() {
                    Some("primary") => PointerButton::Primary,
                    Some("secondary") => PointerButton::Secondary,
                    Some("middle") => PointerButton::Middle,
                    other => return Err(format!("invalid --button {other:?}")),
                };
            }
            "--count" => {
                let value = args.next().ok_or("--count needs a number")?;
                count = value
                    .parse()
                    .map_err(|_| format!("invalid --count {value:?}"))?;
            }
            _ => positional.push(arg),
        }
    }
    let socket = socket.ok_or_else(|| {
        format!(
            "no control socket: pass --socket or set {}",
            archipel_ide_control::SOCKET_ENV
        )
    })?;
    let mut positional = positional.into_iter();
    let op = positional.next().ok_or("missing request")?;
    let mut required = |name: &str| {
        positional
            .next()
            .ok_or_else(|| format!("{op} needs <{name}>"))
    };
    let request = match op.as_str() {
        "execute" => {
            let command = required("command-id")?;
            let args = match positional.next() {
                Some(json) => {
                    serde_json::from_str(&json).map_err(|e| format!("invalid JSON: {e}"))?
                }
                None => Value::Null,
            };
            Request::Execute { command, args }
        }
        "state" => Request::State,
        "key" => Request::Key {
            chord: required("chord")?,
        },
        "type" => Request::Type {
            text: required("text")?,
        },
        "click" | "move" => {
            let mut coordinate = |name: &str| -> Result<f32, String> {
                let value = required(name)?;
                value
                    .parse::<f32>()
                    .map_err(|_| format!("invalid {name} {value:?}"))
            };
            let (x, y) = (coordinate("x")?, coordinate("y")?);
            if op == "move" {
                Request::Move { x, y }
            } else {
                Request::Click {
                    x,
                    y,
                    button,
                    count,
                }
            }
        }
        "screenshot" => Request::Screenshot {
            path: required("path")?.into(),
        },
        "wait" => {
            let until = match required("condition")?.as_str() {
                "idle" => WaitCondition::Idle,
                "popup" => WaitCondition::Popup,
                "text" => WaitCondition::Text,
                "analysis" => WaitCondition::Analysis,
                other => return Err(format!("unknown wait condition {other:?}")),
            };
            let text = positional.next();
            if until == WaitCondition::Text && text.is_none() {
                return Err("wait text needs <text>".into());
            }
            Request::Wait {
                until,
                text,
                timeout_ms,
            }
        }
        "raw" => serde_json::from_str(&required("json-request")?)
            .map_err(|e| format!("invalid request: {e}"))?,
        other => return Err(format!("unknown request {other:?}")),
    };
    Ok((socket, request))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<Request, String> {
        let mut all = vec!["--socket".to_owned(), "/tmp/x.sock".to_owned()];
        all.extend(args.iter().map(|s| s.to_string()));
        parse_ui_args(all).map(|(socket, request)| {
            assert_eq!(socket, PathBuf::from("/tmp/x.sock"));
            request
        })
    }

    #[test]
    fn ui_arguments_map_to_requests() {
        assert_eq!(
            parse(&["execute", "editor.openFile", r#"{"path":"a.rs"}"#]).unwrap(),
            Request::Execute {
                command: "editor.openFile".into(),
                args: serde_json::json!({"path": "a.rs"})
            }
        );
        assert_eq!(parse(&["state"]).unwrap(), Request::State);
        assert_eq!(
            parse(&["wait", "text", "fn main", "--timeout", "500"]).unwrap(),
            Request::Wait {
                until: WaitCondition::Text,
                text: Some("fn main".into()),
                timeout_ms: Some(500)
            }
        );
        assert_eq!(
            parse(&["raw", r#"{"op":"key","chord":"Escape"}"#]).unwrap(),
            Request::Key {
                chord: "Escape".into()
            }
        );
        assert_eq!(
            parse(&[
                "click",
                "18",
                "20.5",
                "--button",
                "secondary",
                "--count",
                "2"
            ])
            .unwrap(),
            Request::Click {
                x: 18.0,
                y: 20.5,
                button: PointerButton::Secondary,
                count: 2
            }
        );
        assert_eq!(
            parse(&["move", "1", "2"]).unwrap(),
            Request::Move { x: 1.0, y: 2.0 }
        );
        assert!(parse(&["click", "x", "2"]).is_err());
        assert!(parse(&["wait", "text"]).is_err());
        assert!(parse(&["wait", "forever"]).is_err());
        assert!(parse(&["key"]).is_err());
        assert!(parse(&["fly"]).is_err());
        assert!(parse(&["execute", "x", "{bad"]).is_err());
    }
}
