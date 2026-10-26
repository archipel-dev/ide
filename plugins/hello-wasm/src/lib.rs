//! Example Archipel IDE plugin as a WebAssembly component (`archipel:plugin`
//! world). Structured values cross the boundary as JSON.

use serde_json::{Value, json};
use std::cell::RefCell;

wit_bindgen::generate!({
    path: "../../crates/archipel-ide-plugin-wasm/wit",
    world: "plugin",
});

use archipel::plugin::host;

#[derive(Default)]
struct State {
    greetings: u32,
    name: String,
    files: Vec<String>,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State { name: "WASM".into(), ..State::default() });
}

fn greeting() -> String {
    let word = host::setting("greeting")
        .and_then(|json| serde_json::from_str::<Value>(&json).ok())
        .and_then(|value| value.as_str().map(str::to_owned))
        .unwrap_or_else(|| "Hello".into());
    STATE.with_borrow_mut(|state| {
        state.greetings += 1;
        format!("{word}, {}!", state.name)
    })
}

/// Lists the project root, visible at /project with the `fs.project` permission.
fn list_project() -> Vec<String> {
    match std::fs::read_dir("/project") {
        Ok(entries) => {
            let mut names: Vec<String> = entries
                .filter_map(Result::ok)
                .map(|entry| entry.file_name().to_string_lossy().into_owned())
                .collect();
            names.sort();
            names.truncate(20);
            names
        }
        Err(error) => vec![format!("/project unavailable: {error}")],
    }
}

struct HelloWasm;

impl Guest for HelloWasm {
    fn contributions() -> String {
        json!({
            "commands": [
                {"id": "hello_wasm.sayHello", "label": "Say Hello (WASM)"},
                {"id": "hello_wasm.greet", "label": "Greet Someone (WASM)", "params": "{\"name\":\"string\"}"},
                {"id": "hello_wasm.listProject", "label": "List Project Files (WASM)"}
            ],
            "menus": [{"menu": "Tools", "command": "hello_wasm.sayHello"}],
            "keybindings": [{"command": "hello_wasm.sayHello", "key": "Mod+Alt+W"}],
            "tool_windows": [{"id": "hello_wasm", "title": "Hello WASM", "view": "main", "icon_text": "W"}],
            "status_widgets": [{"id": "count", "command": "hello_wasm.sayHello"}],
            "settings_pages": [{
                "id": "hello_wasm", "title": "Hello WASM",
                "fields": [{"key": "greeting", "label": "Greeting", "kind": {"type": "text", "default": "Hello"}}]
            }]
        })
        .to_string()
    }

    fn activate() -> Result<(), String> {
        host::status("Hello WASM activated");
        Ok(())
    }

    fn deactivate() {}

    fn execute(command: String, params_json: String) -> Result<String, String> {
        let params: Value = serde_json::from_str(&params_json).unwrap_or(Value::Null);
        match command.as_str() {
            "hello_wasm.sayHello" => {
                let message = greeting();
                host::status(&message);
                host::notify("info", "Hello WASM", &message);
                Ok(json!({"message": message}).to_string())
            }
            "hello_wasm.greet" => {
                let name = params
                    .get("name")
                    .and_then(Value::as_str)
                    .filter(|name| !name.is_empty())
                    .ok_or("expected string field `name`")?;
                STATE.with_borrow_mut(|state| state.name = name.to_owned());
                let message = greeting();
                host::status(&message);
                Ok(json!({"message": message}).to_string())
            }
            "hello_wasm.listProject" => {
                let files = list_project();
                STATE.with_borrow_mut(|state| state.files = files.clone());
                host::open_tool_window("hello_wasm");
                Ok(json!({"files": files}).to_string())
            }
            "hello_wasm.spin" => loop {
                std::hint::black_box(());
            },
            other => Err(format!("unknown command {other}")),
        }
    }

    fn render_view(view: String, _placement_json: String) -> String {
        STATE.with_borrow(|state| {
            let mut children = vec![
                json!({"type": "heading", "text": "Hello from WebAssembly"}),
                json!({"type": "row", "children": [
                    {"type": "text_input", "id": "name", "value": state.name, "hint": "Name"},
                    {"type": "button", "id": "hello", "label": "Say Hello"}
                ]}),
                json!({"type": "section", "text": "Statistics"}),
                json!({"type": "property", "key": "Greetings", "value": state.greetings.to_string()}),
                json!({"type": "property", "key": "Project", "value": host::project_root().unwrap_or_else(|| "none".into())}),
                json!({"type": "property", "key": "fs.project", "value": host::has_permission("fs.project").to_string()}),
                json!({"type": "button", "id": "list", "label": "List /project"}),
            ];
            if !state.files.is_empty() {
                let items: Vec<Value> = state
                    .files
                    .iter()
                    .map(|name| json!({"id": name, "label": name}))
                    .collect();
                children.push(json!({"type": "list", "id": "files", "items": items}));
            }
            let _ = view;
            json!({"type": "column", "children": children}).to_string()
        })
    }

    fn on_view_event(_view: String, event_json: String) {
        let event: Value = serde_json::from_str(&event_json).unwrap_or(Value::Null);
        match event["id"].as_str() {
            Some("name") => {
                let name = event["value"].as_str().unwrap_or_default().to_owned();
                STATE.with_borrow_mut(|state| state.name = name);
            }
            Some("hello") => {
                let message = greeting();
                host::status(&message);
            }
            Some("list") => {
                let files = list_project();
                STATE.with_borrow_mut(|state| state.files = files);
            }
            _ => {}
        }
    }

    fn status_widget(id: String) -> Option<String> {
        (id == "count").then(|| STATE.with_borrow(|state| format!("WASM × {}", state.greetings)))
    }

    fn poll() {}
}

export!(HelloWasm);
