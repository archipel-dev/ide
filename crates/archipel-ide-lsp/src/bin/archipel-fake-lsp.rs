//! Scriptable fake language server used by Archipel IDE's integration tests.
//!
//! Flags:
//! - `--progress`: create a work-done progress token and report indexing.
//! - `--crash-once <file>`: exit with code 3 after `initialized` unless `<file>`
//!   exists (the file is created first, so the restarted server survives).
//! - `--completion-delay-ms <n>`: answer completion requests after a delay.
//! - `--publish <relative path>`: after `initialized`, publish one error for a
//!   file below the workspace root that was never opened.
//!
//! Navigation works on words: definition is the word's first occurrence in
//! the open documents (by URI), typeDefinition the same as a `LocationLink`,
//! implementation every later occurrence and references all of them.
//!
//! Code actions: one preferred quick fix per diagnostic in the request
//! context (replacing the marker with `fixed`), a refactoring that needs
//! `codeAction/resolve` (inserting `// resolved` at the top) and a command
//! (`fake.apply`) whose execution sends `workspace/applyEdit`.
//!
//! Refactoring: `prepareRename` answers the word's range and placeholder
//! (`null` off a word; absent with `--no-prepare-rename`), `rename` replaces
//! the word in every open document, `formatting`/`rangeFormatting` trim
//! trailing whitespace and `codeAction` requests with `context.only` return
//! `source.organizeImports` (sorts the `import` lines), `refactor.extract.*`
//! and `refactor.inline` edits instead of the diagnostic fixes above.
//!
//! Generate: `codeAction` requests with `context.only` containing `source`
//! also return the jdtls-style `source.generate.*` / `source.overrideMethods`
//! actions (each inserts a `// generated ...` comment at the top).
//!
//! Hierarchies (ASCII text): `class A extends B implements C` and `interface`
//! lines declare types (the range runs to the next declaration);
//! `void name(` lines declare methods. `prepareTypeHierarchy`,
//! `typeHierarchy/supertypes|subtypes`, `prepareCallHierarchy` and
//! `callHierarchy/incomingCalls|outgoingCalls` answer from the open documents.
//!
//! Completion returns a fixed Java-like member list. `textDocument/didOpen`
//! and `didChange` publish one diagnostic per `@error`, `@warning`, `@weak`,
//! `@hint` and `@unused` marker in the text, or one warning at 0:0 when the
//! text has no markers.
//! - `--incomplete`: mark completion lists `isIncomplete`.
//! - `--pull-diagnostics`: like rust-analyzer, declare `diagnosticProvider` and serve the
//!   diagnostics of the text only through `textDocument/diagnostic` (nothing is pushed on
//!   `didOpen`/`didChange`; after each of them the server asks the client to pull again with
//!   `workspace/diagnostic/refresh`). `didSave` still pushes the "check" warning, like the
//!   `cargo check` of rust-analyzer.
//! - `--push-on-save-only`: like a server that analyzes on save: `didOpen` and `didChange`
//!   publish nothing, `didSave` publishes the marker diagnostics and the "check" warning.
//! - `--log <file>`: append every request and notification the server receives to `<file>`,
//!   one line each (`textDocument/didChange v=3`), for tests that count them.
//! - `--importing <n>`: like jdtls importing a Gradle project, report a
//!   "Building" progress from `initialized` on, answer the first `n`
//!   completion requests with an empty list, and end the progress 300 ms
//!   after the last of them.
//!   It also reports jdtls's `language/status` (`Starting` ... `ServiceReady`).
//! - `--java-commands`: quick fixes come as `java.apply.workspaceEdit`
//!   commands (jdtls hands edits to clients that way) instead of edits.
//! - `--java-debug <port>`: answer the java-debug plugin's `vscode.java.*` commands
//!   (main class `demo.App` in `<root>/App.java`, a one-entry class path, and the
//!   given debug port).
//!
//! Completion lists start with an item `trigger<kind>` naming the request's
//! `triggerKind`.
//!
//! Completion returns a fixed Java-like member list; resolve adds Markdown
//! documentation and an import edit; hover and signature help return fixed
//! answers. `textDocument/didOpen` and `didChange` publish one diagnostic
//! per document.

use archipel_ide_lsp::transport::{read_message, write_message};
use serde_json::{Value, json};
use std::io::{BufReader, stdin, stdout};
use std::time::Duration;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let flag = |name: &str| args.iter().any(|arg| arg == name);
    let value = |name: &str| {
        args.iter()
            .position(|arg| arg == name)
            .and_then(|index| args.get(index + 1))
            .cloned()
    };
    let progress = flag("--progress");
    let incomplete = flag("--incomplete");
    let crash_file = value("--crash-once");
    let delay = value("--completion-delay-ms")
        .and_then(|ms| ms.parse().ok())
        .map(Duration::from_millis);
    let publish = value("--publish");
    let java_debug_port: Option<u16> = value("--java-debug").and_then(|p| p.parse().ok());
    let prepare_rename = !flag("--no-prepare-rename");
    let java_commands = flag("--java-commands");
    let pull_diagnostics = flag("--pull-diagnostics");
    let push_on_save_only = flag("--push-on-save-only");
    let log = value("--log");
    let mut refresh_ids = 0u32;
    let mut importing: u32 = value("--importing")
        .and_then(|n| n.parse().ok())
        .unwrap_or(0);
    let mut root = String::new();
    let mut documents: std::collections::BTreeMap<String, String> = Default::default();
    let mut input = BufReader::new(stdin().lock());
    let mut output = stdout().lock();
    let mut send = |message: Value| {
        if write_message(&mut output, &message).is_err() {
            std::process::exit(1);
        }
    };
    loop {
        let Ok(message) = read_message(&mut input) else {
            std::process::exit(0);
        };
        let method = message.get("method").and_then(Value::as_str).unwrap_or("");
        let id = message.get("id").cloned();
        let params = message.get("params").cloned().unwrap_or(Value::Null);
        if let Some(file) = &log
            && !method.is_empty()
            && let Ok(mut log) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(file)
        {
            use std::io::Write as _;
            let version = params["textDocument"]["version"].as_i64();
            let _ = match version {
                Some(version) => writeln!(log, "{method} v={version}"),
                None => writeln!(log, "{method}"),
            };
        }
        match (method, id) {
            ("initialize", Some(id)) => {
                root = params["rootUri"].as_str().unwrap_or_default().to_owned();
                if progress {
                    send(
                        json!({"jsonrpc":"2.0","id":"p1","method":"window/workDoneProgress/create","params":{"token":"index"}}),
                    );
                    send(
                        json!({"jsonrpc":"2.0","method":"$/progress","params":{"token":"index","value":{"kind":"begin","title":"Indexing","percentage":0}}}),
                    );
                    send(
                        json!({"jsonrpc":"2.0","method":"$/progress","params":{"token":"index","value":{"kind":"report","message":"fake","percentage":45}}}),
                    );
                }
                let rename_provider = if prepare_rename {
                    json!({"prepareProvider": true})
                } else {
                    json!(true)
                };
                send(json!({"jsonrpc":"2.0","id":id,"result":{"capabilities":{
                    "renameProvider": rename_provider,
                    "documentFormattingProvider": true,
                    "documentRangeFormattingProvider": true,
                    "textDocumentSync": {"openClose": true, "change": 1, "save": {"includeText": false}},
                    "completionProvider": {"triggerCharacters": ["."], "resolveProvider": true},
                    "hoverProvider": true,
                    "signatureHelpProvider": {"triggerCharacters": ["(", ","]},
                    "definitionProvider": true,
                    "typeDefinitionProvider": true,
                    "implementationProvider": true,
                    "referencesProvider": true,
                    "inlayHintProvider": true,
                    "diagnosticProvider": if pull_diagnostics {
                        json!({"identifier":"fake","interFileDependencies":false,"workspaceDiagnostics":false})
                    } else {
                        Value::Null
                    },
                    "codeActionProvider": {"resolveProvider": true},
                    "callHierarchyProvider": true,
                    "executeCommandProvider": {"commands": ["fake.apply"]}
                },"serverInfo":{"name":"fake"}}}));
            }
            ("initialized", None) if importing > 0 => {
                send(
                    json!({"jsonrpc":"2.0","id":"p2","method":"window/workDoneProgress/create","params":{"token":"import"}}),
                );
                send(
                    json!({"jsonrpc":"2.0","method":"$/progress","params":{"token":"import","value":{"kind":"begin","title":"Building","percentage":0}}}),
                );
                send(
                    json!({"jsonrpc":"2.0","method":"language/status","params":{"type":"Starting","message":"70% Starting Java Language Server"}}),
                );
            }
            ("initialized", None) => {
                if let Some(file) = &crash_file
                    && !std::path::Path::new(file).exists()
                {
                    let _ = std::fs::write(file, "crashed");
                    eprintln!("fake server crashing on purpose");
                    std::process::exit(3);
                }
                send(
                    json!({"jsonrpc":"2.0","method":"workspace/configuration","id":"c1","params":{"items":[{"section":"fake"}]}}),
                );
                if let Some(relative) = &publish {
                    let uri = format!("{}/{relative}", root.trim_end_matches('/'));
                    send(
                        json!({"jsonrpc":"2.0","method":"textDocument/publishDiagnostics","params":{
                            "uri": uri, "diagnostics": [{"range":{"start":{"line":1,"character":0},"end":{"line":1,"character":3}},
                            "severity":1,"source":"fake","message":"fake project error"}]
                        }}),
                    );
                }
            }
            ("textDocument/didOpen" | "textDocument/didChange", None) => {
                let uri = params["textDocument"]["uri"].clone();
                let text = params["textDocument"]["text"]
                    .as_str()
                    .or_else(|| params["contentChanges"][0]["text"].as_str())
                    .unwrap_or_default();
                documents.insert(uri.as_str().unwrap_or_default().to_owned(), text.to_owned());
                if pull_diagnostics {
                    // The client pulls; the server asks it to, as rust-analyzer does.
                    refresh_ids += 1;
                    send(
                        json!({"jsonrpc":"2.0","id":format!("refresh{refresh_ids}"),"method":"workspace/diagnostic/refresh","params":null}),
                    );
                } else if !push_on_save_only {
                    let mut diagnostics = marker_diagnostics(text);
                    if diagnostics.is_empty() {
                        diagnostics.push(json!({"range":{"start":{"line":0,"character":0},"end":{"line":0,"character":1}},
                            "severity":2,"message":format!("fake diagnostic ({method})")}));
                    }
                    send(
                        json!({"jsonrpc":"2.0","method":"textDocument/publishDiagnostics","params":{
                            "uri": uri, "diagnostics": diagnostics
                        }}),
                    );
                }
            }
            ("textDocument/diagnostic", Some(id)) => {
                let uri = params["textDocument"]["uri"].as_str().unwrap_or_default();
                let text = documents.get(uri).map_or("", String::as_str);
                let mut items = marker_diagnostics(text);
                if items.is_empty() {
                    items.push(json!({"range":{"start":{"line":0,"character":0},"end":{"line":0,"character":1}},
                        "severity":2,"message":"fake diagnostic (pull)"}));
                }
                send(json!({"jsonrpc":"2.0","id":id,"result":{"kind":"full","items":items}}));
            }
            ("textDocument/didSave", None) => {
                let uri = params["textDocument"]["uri"].clone();
                let mut diagnostics = vec![
                    json!({"range":{"start":{"line":0,"character":0},"end":{"line":0,"character":1}},
                    "severity":2,"source":"check","message":"fake check warning"}),
                ];
                if push_on_save_only {
                    let text = documents
                        .get(uri.as_str().unwrap_or_default())
                        .map_or("", String::as_str);
                    diagnostics.extend(marker_diagnostics(text));
                }
                send(
                    json!({"jsonrpc":"2.0","method":"textDocument/publishDiagnostics","params":{
                        "uri": uri, "diagnostics": diagnostics
                    }}),
                );
            }
            (
                "textDocument/definition"
                | "textDocument/typeDefinition"
                | "textDocument/implementation"
                | "textDocument/references",
                Some(id),
            ) => {
                let uri = params["textDocument"]["uri"].as_str().unwrap_or_default();
                let line = params["position"]["line"].as_u64().unwrap_or(0) as usize;
                let character = params["position"]["character"].as_u64().unwrap_or(0) as usize;
                let word = documents
                    .get(uri)
                    .and_then(|text| word_at(text, line, character))
                    .unwrap_or_default();
                let occurrences: Vec<Value> = documents
                    .iter()
                    .flat_map(|(uri, text)| occurrences(uri, text, &word))
                    .collect();
                let result = match method {
                    _ if word.is_empty() => Value::Null,
                    "textDocument/definition" => {
                        occurrences.first().cloned().unwrap_or(Value::Null)
                    }
                    "textDocument/typeDefinition" => match occurrences.first() {
                        Some(first) => {
                            json!([{"targetUri": first["uri"], "targetRange": first["range"],
                            "targetSelectionRange": first["range"]}])
                        }
                        None => Value::Null,
                    },
                    "textDocument/implementation" => {
                        json!(occurrences.iter().skip(1).collect::<Vec<_>>())
                    }
                    _ => json!(occurrences),
                };
                send(json!({"jsonrpc":"2.0","id":id,"result":result}));
            }
            ("textDocument/prepareRename", Some(id)) => {
                let uri = params["textDocument"]["uri"].as_str().unwrap_or_default();
                let line = params["position"]["line"].as_u64().unwrap_or(0) as usize;
                let character = params["position"]["character"].as_u64().unwrap_or(0) as usize;
                let word = documents
                    .get(uri)
                    .and_then(|text| word_at(text, line, character))
                    .unwrap_or_default();
                let range = occurrences(uri, documents.get(uri).map_or("", String::as_str), &word)
                    .into_iter()
                    .find(|o| {
                        o["range"]["start"]["line"].as_u64() == Some(line as u64)
                            && o["range"]["start"]["character"].as_u64() <= Some(character as u64)
                            && o["range"]["end"]["character"].as_u64() >= Some(character as u64)
                    })
                    .map(|o| o["range"].clone());
                let result = match range {
                    Some(range) => json!({"range": range, "placeholder": word}),
                    None => Value::Null,
                };
                send(json!({"jsonrpc":"2.0","id":id,"result":result}));
            }
            ("textDocument/rename", Some(id)) => {
                let uri = params["textDocument"]["uri"].as_str().unwrap_or_default();
                let line = params["position"]["line"].as_u64().unwrap_or(0) as usize;
                let character = params["position"]["character"].as_u64().unwrap_or(0) as usize;
                let new_name = params["newName"].as_str().unwrap_or_default();
                let word = documents
                    .get(uri)
                    .and_then(|text| word_at(text, line, character))
                    .unwrap_or_default();
                if new_name.is_empty() || word.is_empty() {
                    send(json!({"jsonrpc":"2.0","id":id,"error":{"code":-32602,
                        "message":"cannot rename here"}}));
                    continue;
                }
                let mut changes = serde_json::Map::new();
                for (uri, text) in &documents {
                    let edits: Vec<Value> = occurrences(uri, text, &word)
                        .into_iter()
                        .map(|o| json!({"range": o["range"], "newText": new_name}))
                        .collect();
                    if !edits.is_empty() {
                        changes.insert(uri.clone(), Value::Array(edits));
                    }
                }
                send(json!({"jsonrpc":"2.0","id":id,"result":{"changes": changes}}));
            }
            ("textDocument/formatting" | "textDocument/rangeFormatting", Some(id)) => {
                let uri = params["textDocument"]["uri"].as_str().unwrap_or_default();
                let first = params["range"]["start"]["line"].as_u64().unwrap_or(0) as usize;
                let last = params["range"]["end"]["line"].as_u64().map(|l| l as usize);
                let edits: Vec<Value> = documents
                    .get(uri)
                    .map(|text| {
                        text.lines()
                            .enumerate()
                            .filter(|(line, content)| {
                                *line >= first
                                    && last.is_none_or(|last| *line <= last)
                                    && content.len() != content.trim_end().len()
                            })
                            .map(|(line, content)| {
                                let keep: usize =
                                    content.trim_end().chars().map(char::len_utf16).sum();
                                let all: usize = content.chars().map(char::len_utf16).sum();
                                json!({"range": {"start": {"line": line, "character": keep},
                                    "end": {"line": line, "character": all}}, "newText": ""})
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                send(json!({"jsonrpc":"2.0","id":id,"result":edits}));
            }
            ("textDocument/prepareTypeHierarchy", Some(id)) => {
                let uri = params["textDocument"]["uri"].as_str().unwrap_or_default();
                let line = params["position"]["line"].as_u64().unwrap_or(0) as usize;
                let character = params["position"]["character"].as_u64().unwrap_or(0) as usize;
                let word = documents
                    .get(uri)
                    .and_then(|text| word_at(text, line, character))
                    .unwrap_or_default();
                let types = type_decls(&documents);
                let found = types.iter().find(|t| t.name == word).or_else(|| {
                    types
                        .iter()
                        .find(|t| t.uri == uri && t.line <= line && line <= t.end)
                });
                let result = found.map_or(Value::Null, |t| json!([t.item()]));
                send(json!({"jsonrpc":"2.0","id":id,"result":result}));
            }
            ("typeHierarchy/supertypes", Some(id)) => {
                let name = params["item"]["name"].as_str().unwrap_or_default();
                let types = type_decls(&documents);
                let supers: Vec<Value> = types
                    .iter()
                    .find(|t| t.name == name)
                    .map(|t| {
                        t.supers
                            .iter()
                            .filter_map(|s| types.iter().find(|u| &u.name == s))
                            .map(TypeDecl::item)
                            .collect()
                    })
                    .unwrap_or_default();
                send(json!({"jsonrpc":"2.0","id":id,"result":supers}));
            }
            ("typeHierarchy/subtypes", Some(id)) => {
                let name = params["item"]["name"].as_str().unwrap_or_default();
                let subs: Vec<Value> = type_decls(&documents)
                    .iter()
                    .filter(|t| t.supers.iter().any(|s| s == name))
                    .map(TypeDecl::item)
                    .collect();
                send(json!({"jsonrpc":"2.0","id":id,"result":subs}));
            }
            ("textDocument/prepareCallHierarchy", Some(id)) => {
                let uri = params["textDocument"]["uri"].as_str().unwrap_or_default();
                let line = params["position"]["line"].as_u64().unwrap_or(0) as usize;
                let character = params["position"]["character"].as_u64().unwrap_or(0) as usize;
                let word = documents
                    .get(uri)
                    .and_then(|text| word_at(text, line, character))
                    .unwrap_or_default();
                let methods = method_decls(&documents);
                let result = methods
                    .iter()
                    .find(|m| m.name == word && m.uri == uri && m.line == line)
                    .or_else(|| methods.iter().find(|m| m.name == word))
                    .map_or(Value::Null, |m| json!([m.item()]));
                send(json!({"jsonrpc":"2.0","id":id,"result":result}));
            }
            ("callHierarchy/incomingCalls", Some(id)) => {
                let name = params["item"]["name"].as_str().unwrap_or_default();
                let methods = method_decls(&documents);
                let mut calls: Vec<Value> = Vec::new();
                for caller in &methods {
                    let ranges: Vec<Value> = call_sites(&documents, caller, name);
                    if !ranges.is_empty() {
                        calls.push(json!({"from": caller.item(), "fromRanges": ranges}));
                    }
                }
                send(json!({"jsonrpc":"2.0","id":id,"result":calls}));
            }
            ("callHierarchy/outgoingCalls", Some(id)) => {
                let name = params["item"]["name"].as_str().unwrap_or_default();
                let methods = method_decls(&documents);
                let mut calls: Vec<Value> = Vec::new();
                let uri = params["item"]["uri"].as_str().unwrap_or_default();
                if let Some(caller) = methods.iter().find(|m| m.name == name && m.uri == uri) {
                    for callee in &methods {
                        let ranges = call_sites(&documents, caller, &callee.name);
                        if !ranges.is_empty() {
                            calls.push(json!({"to": callee.item(), "fromRanges": ranges}));
                        }
                    }
                }
                send(json!({"jsonrpc":"2.0","id":id,"result":calls}));
            }
            ("textDocument/codeAction", Some(id)) if params["context"]["only"].is_array() => {
                let uri = params["textDocument"]["uri"].as_str().unwrap_or_default();
                let only: Vec<&str> = params["context"]["only"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .collect();
                let range = params["range"].clone();
                let selected = documents
                    .get(uri)
                    .and_then(|text| selection_text(text, &range))
                    .unwrap_or_default();
                let at = |line: u64, character: u64| json!({"line": line, "character": character});
                let mut actions = Vec::new();
                let mut offer = |title: &str, kind: &str, edit: Value| {
                    if only.iter().any(|only| kind.starts_with(only)) {
                        actions.push(json!({"title": title, "kind": kind, "edit": edit}));
                    }
                };
                if let Some(text) = documents.get(uri) {
                    let imports: Vec<(usize, &str)> = text
                        .lines()
                        .enumerate()
                        .filter(|(_, line)| line.starts_with("import "))
                        .collect();
                    let mut sorted: Vec<&str> = imports.iter().map(|(_, l)| *l).collect();
                    sorted.sort_unstable();
                    sorted.dedup();
                    if let (Some(first), Some(last)) = (imports.first(), imports.last()) {
                        let end = text.lines().nth(last.0).map_or(0, |l| l.len()) as u64;
                        offer(
                            "Organize Imports",
                            "source.organizeImports",
                            json!({"changes": {uri: [{"range": {"start": at(first.0 as u64, 0),
                                "end": at(last.0 as u64, end)}, "newText": sorted.join("\n")}]}}),
                        );
                    }
                }
                if !selected.is_empty() {
                    let (start, end) = (range["start"].clone(), range["end"].clone());
                    offer(
                        "Extract to local variable",
                        "refactor.extract.variable",
                        json!({"changes": {uri: [
                            {"range": {"start": start, "end": end}, "newText": "extracted"},
                            {"range": {"start": at(start["line"].as_u64().unwrap_or(0), 0),
                                "end": at(start["line"].as_u64().unwrap_or(0), 0)},
                                "newText": format!("let extracted = {selected};\n")}]}}),
                    );
                    offer(
                        "Extract to method",
                        "refactor.extract.function",
                        json!({"changes": {uri: [
                            {"range": {"start": start, "end": end}, "newText": "extracted()"},
                            {"range": {"start": at(0, 0), "end": at(0, 0)},
                                "newText": format!("fn extracted() {{ {selected} }}\n")}]}}),
                    );
                    offer(
                        "Extract to constant",
                        "refactor.extract.constant",
                        json!({"changes": {uri: [
                            {"range": {"start": start, "end": end}, "newText": "EXTRACTED"},
                            {"range": {"start": at(0, 0), "end": at(0, 0)},
                                "newText": format!("const EXTRACTED = {selected};\n")}]}}),
                    );
                }
                for (title, kind) in [
                    ("Generate Constructors...", "source.generate.constructors"),
                    (
                        "Generate Getters and Setters...",
                        "source.generate.accessors",
                    ),
                    ("Generate Getters", "source.generate.accessors"),
                    ("Generate Setters", "source.generate.accessors"),
                    (
                        "Generate hashCode() and equals()...",
                        "source.generate.hashCodeEquals",
                    ),
                    ("Generate toString()...", "source.generate.toString"),
                    ("Override/Implement Methods...", "source.overrideMethods"),
                ] {
                    offer(
                        title,
                        kind,
                        json!({"changes": {uri: [{"range": {"start": at(0, 0), "end": at(0, 0)},
                            "newText": format!("// generated {title}\n")}]}}),
                    );
                }
                offer(
                    "Inline variable",
                    "refactor.inline",
                    json!({"changes": {uri: [{"range": {"start": at(0, 0), "end": at(0, 0)},
                        "newText": "// inlined\n"}]}}),
                );
                send(json!({"jsonrpc":"2.0","id":id,"result":actions}));
            }
            ("textDocument/codeAction", Some(id)) => {
                let uri = params["textDocument"]["uri"].clone();
                let mut actions: Vec<Value> = params["context"]["diagnostics"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(|diagnostic| {
                        let edit = json!({"changes": {uri.as_str().unwrap_or_default():
                            [{"range": diagnostic["range"], "newText": "fixed"}]}});
                        let title = format!("Fix {}", diagnostic["message"].as_str().unwrap_or(""));
                        if java_commands {
                            json!({"title": title, "kind": "quickfix", "diagnostics": [diagnostic],
                                "command": {"title": title, "command": "java.apply.workspaceEdit",
                                    "arguments": [edit]}})
                        } else {
                            json!({"title": title, "kind": "quickfix", "isPreferred": true,
                                "diagnostics": [diagnostic], "edit": edit})
                        }
                    })
                    .collect();
                actions.push(
                    json!({"title": "Add header comment", "kind": "refactor.rewrite",
                    "data": {"uri": uri}}),
                );
                actions.push(json!({"title": "Run fake command", "command": "fake.apply",
                    "arguments": [uri]}));
                send(json!({"jsonrpc":"2.0","id":id,"result":actions}));
            }
            ("codeAction/resolve", Some(id)) => {
                let mut action = params.clone();
                let uri = params["data"]["uri"]
                    .as_str()
                    .unwrap_or_default()
                    .to_owned();
                action["edit"] = json!({"documentChanges": [{
                    "textDocument": {"uri": uri, "version": null},
                    "edits": [{"range": {"start": {"line": 0, "character": 0}, "end": {"line": 0, "character": 0}},
                        "newText": "// resolved\n"}]}]});
                send(json!({"jsonrpc":"2.0","id":id,"result":action}));
            }
            ("workspace/executeCommand", Some(id))
                if java_debug_port.is_some()
                    && params["command"]
                        .as_str()
                        .is_some_and(|c| c.starts_with("vscode.java.")) =>
            {
                let result = match params["command"].as_str().unwrap_or_default() {
                    "vscode.java.resolveMainClass" => json!([{
                        "mainClass": "demo.App", "projectName": "app",
                        "filePath": format!("{}/App.java", root.trim_start_matches("file://"))}]),
                    "vscode.java.resolveClasspath" => json!([
                        [],
                        [format!(
                            "{}/build/classes",
                            root.trim_start_matches("file://")
                        )]
                    ]),
                    "vscode.java.startDebugSession" => json!(java_debug_port),
                    _ => Value::Null,
                };
                send(json!({"jsonrpc":"2.0","id":id,"result":result}));
            }
            ("workspace/executeCommand", Some(id)) => {
                let uri = params["arguments"][0]
                    .as_str()
                    .unwrap_or_default()
                    .to_owned();
                send(
                    json!({"jsonrpc":"2.0","id":"apply1","method":"workspace/applyEdit","params":{
                    "label": "Fake command",
                    "edit": {"changes": {uri: [{"range": {"start": {"line": 0, "character": 0},
                        "end": {"line": 0, "character": 0}}, "newText": "// applied\n"}]}}}}),
                );
                send(json!({"jsonrpc":"2.0","id":id,"result":null}));
            }
            ("textDocument/completion", Some(id)) if importing > 0 => {
                send(json!({"jsonrpc":"2.0","id":id,"result":{"isIncomplete":false,"items":[]}}));
                importing -= 1;
                if importing == 0 {
                    std::thread::sleep(std::time::Duration::from_millis(300));
                    send(
                        json!({"jsonrpc":"2.0","method":"language/status","params":{"type":"ServiceReady","message":"ServiceReady"}}),
                    );
                    send(
                        json!({"jsonrpc":"2.0","method":"$/progress","params":{"token":"import","value":{"kind":"end"}}}),
                    );
                }
            }
            ("textDocument/completion", Some(id)) => {
                if let Some(delay) = delay {
                    std::thread::sleep(delay);
                }
                let kind = params["context"]["triggerKind"].as_u64().unwrap_or(0);
                send(
                    json!({"jsonrpc":"2.0","id":id,"result":{"isIncomplete":incomplete,"items":[
                        {"label":format!("trigger{kind}"),"kind":1,"sortText":"9"},
                        {"label":"greeting","kind":2,"commitCharacters":[";"],"detail":"String","labelDetails":{"detail":"(String name)","description":"String"},"sortText":"0"},
                        {"label":"getRequiredField","kind":2,"detail":"Field","sortText":"1"},
                        {"label":"getClass","kind":2,"detail":"Class<?>","sortText":"2"},
                        {"label":"hashCode","kind":2,"detail":"int","sortText":"3"}
                    ]}}),
                );
            }
            ("completionItem/resolve", Some(id)) => {
                let mut item = params.clone();
                item["documentation"] = json!({"kind": "markdown",
                    "value": format!("**{}**\n\nReturns a `String`.", params["label"].as_str().unwrap_or(""))});
                item["additionalTextEdits"] = json!([{"range":{"start":{"line":0,"character":0},"end":{"line":0,"character":0}},"newText":"import fake.Greeting;\n"}]);
                send(json!({"jsonrpc":"2.0","id":id,"result":item}));
            }
            ("textDocument/hover", Some(id)) => {
                send(
                    json!({"jsonrpc":"2.0","id":id,"result":{"contents":{"kind":"markdown",
                    "value":"```java\nString greeting(String name)\n```\nGreets *name*."}}}),
                );
            }
            ("textDocument/inlayHint", Some(id)) => {
                // A `name:` parameter hint after every `greeting(` call.
                let uri = params["textDocument"]["uri"].as_str().unwrap_or_default();
                let mut hints = Vec::new();
                for (line, content) in documents
                    .get(uri)
                    .map_or("", String::as_str)
                    .lines()
                    .enumerate()
                {
                    for (at, _) in content.match_indices("greeting(\"") {
                        hints.push(json!({"position": {"line": line, "character": at + 9},
                            "label": "name:", "kind": 2}));
                    }
                }
                send(json!({"jsonrpc":"2.0","id":id,"result":hints}));
            }
            ("textDocument/signatureHelp", Some(id)) => {
                send(json!({"jsonrpc":"2.0","id":id,"result":{"signatures":[
                    {"label":"greeting(String name, int times)","parameters":[{"label":"String name"},{"label":[22,31]}]}
                ],"activeSignature":0,"activeParameter":1}}));
            }
            ("shutdown", Some(id)) => send(json!({"jsonrpc":"2.0","id":id,"result":null})),
            ("exit", None) => std::process::exit(0),
            ("$/cancelRequest", None) => {}
            (_, Some(id)) if !method.is_empty() => send(json!({"jsonrpc":"2.0","id":id,
                "error":{"code":-32601,"message":format!("unhandled {method}")}})),
            _ => {}
        }
    }
}

/// One diagnostic per marker; positions are UTF-16 like a real server's.
fn marker_diagnostics(text: &str) -> Vec<Value> {
    let markers = [
        ("@error", 1, None),
        ("@warning", 2, None),
        ("@weak", 3, None),
        ("@hint", 4, None),
        ("@unused", 2, Some(1)),
    ];
    let mut diagnostics = Vec::new();
    for (line, content) in text.lines().enumerate() {
        for (marker, severity, tag) in markers {
            let mut from = 0;
            while let Some(found) = content[from..].find(marker) {
                let byte = from + found;
                let start: usize = content[..byte].chars().map(char::len_utf16).sum();
                let end = start + marker.len();
                let mut diagnostic = json!({
                    "range":{"start":{"line":line,"character":start},"end":{"line":line,"character":end}},
                    "severity": severity,
                    "source": "fake",
                    "code": format!("F{severity}"),
                    "message": format!("marker {marker}")
                });
                if let Some(tag) = tag {
                    diagnostic["tags"] = json!([tag]);
                }
                diagnostics.push(diagnostic);
                from = byte + marker.len();
            }
        }
    }
    diagnostics
}

/// The identifier at a UTF-16 position.
fn word_at(text: &str, line: usize, character: usize) -> Option<String> {
    let content = text.lines().nth(line)?;
    let chars: Vec<char> = content.chars().collect();
    let mut units = 0;
    let mut index = chars.len();
    for (i, c) in chars.iter().enumerate() {
        if units >= character {
            index = i;
            break;
        }
        units += c.len_utf16();
    }
    let is_word = |c: &char| c.is_alphanumeric() || *c == '_';
    let start = chars[..index]
        .iter()
        .rposition(|c| !is_word(c))
        .map_or(0, |p| p + 1);
    let end = chars[index..]
        .iter()
        .position(|c| !is_word(c))
        .map_or(chars.len(), |p| index + p);
    (start < end).then(|| chars[start..end].iter().collect())
}

/// Whole-word occurrences of `word` as LSP locations.
fn occurrences(uri: &str, text: &str, word: &str) -> Vec<Value> {
    let mut found = Vec::new();
    if word.is_empty() {
        return found;
    }
    let is_word = |c: char| c.is_alphanumeric() || c == '_';
    for (line, content) in text.lines().enumerate() {
        let mut from = 0;
        while let Some(offset) = content[from..].find(word) {
            let byte = from + offset;
            let end = byte + word.len();
            let before = content[..byte].chars().next_back();
            let after = content[end..].chars().next();
            if !before.is_some_and(is_word) && !after.is_some_and(is_word) {
                let start: usize = content[..byte].chars().map(char::len_utf16).sum();
                let stop = start + word.chars().map(char::len_utf16).sum::<usize>();
                found.push(json!({"uri": uri, "range": {
                    "start": {"line": line, "character": start},
                    "end": {"line": line, "character": stop}}}));
            }
            from = end;
        }
    }
    found
}

/// The text inside a single-line LSP range (ASCII documents only).
fn selection_text(text: &str, range: &Value) -> Option<String> {
    let line = range["start"]["line"].as_u64()? as usize;
    if range["end"]["line"].as_u64()? as usize != line {
        return None;
    }
    let start = range["start"]["character"].as_u64()? as usize;
    let end = range["end"]["character"].as_u64()? as usize;
    let content = text.lines().nth(line)?;
    content
        .get(start..end)
        .filter(|s| !s.is_empty())
        .map(String::from)
}

/// A `class`/`interface` declaration of an open document.
struct TypeDecl {
    name: String,
    uri: String,
    line: usize,
    column: usize,
    end: usize,
    supers: Vec<String>,
}

impl TypeDecl {
    fn item(&self) -> Value {
        json!({"name": self.name, "kind": 5, "detail": "fake", "uri": self.uri,
            "range": {"start": {"line": self.line, "character": 0},
                "end": {"line": self.end, "character": 200}},
            "selectionRange": {"start": {"line": self.line, "character": self.column},
                "end": {"line": self.line, "character": self.column + self.name.len()}}})
    }
}

fn type_decls(documents: &std::collections::BTreeMap<String, String>) -> Vec<TypeDecl> {
    let mut decls = Vec::new();
    for (uri, text) in documents {
        let first = decls.len();
        let lines = text.lines().count();
        for (line, content) in text.lines().enumerate() {
            let tokens: Vec<&str> = content
                .split(|c: char| c.is_whitespace() || c == ',')
                .filter(|t| !t.is_empty())
                .collect();
            let Some(at) = tokens
                .iter()
                .position(|t| *t == "class" || *t == "interface")
            else {
                continue;
            };
            let Some(name) = tokens.get(at + 1).map(|n| n.trim_end_matches('{')) else {
                continue;
            };
            let supers = tokens[at + 2..]
                .iter()
                .take_while(|t| **t != "{")
                .filter(|t| **t != "extends" && **t != "implements")
                .map(|t| t.trim_end_matches('{').to_owned())
                .filter(|t| !t.is_empty())
                .collect();
            decls.push(TypeDecl {
                name: name.to_owned(),
                uri: uri.clone(),
                line,
                column: content.find(name).unwrap_or(0),
                end: lines.saturating_sub(1),
                supers,
            });
        }
        for i in first..decls.len().saturating_sub(1) {
            decls[i].end = decls[i + 1].line.saturating_sub(1);
        }
    }
    decls
}

/// A `void name(` declaration of an open document.
struct MethodDecl {
    name: String,
    uri: String,
    line: usize,
    column: usize,
    end: usize,
}

impl MethodDecl {
    fn item(&self) -> Value {
        json!({"name": self.name, "kind": 6, "detail": "fake", "uri": self.uri,
            "range": {"start": {"line": self.line, "character": 0},
                "end": {"line": self.end, "character": 200}},
            "selectionRange": {"start": {"line": self.line, "character": self.column},
                "end": {"line": self.line, "character": self.column + self.name.len()}}})
    }
}

fn method_decls(documents: &std::collections::BTreeMap<String, String>) -> Vec<MethodDecl> {
    let mut decls = Vec::new();
    for (uri, text) in documents {
        let first = decls.len();
        let lines = text.lines().count();
        for (line, content) in text.lines().enumerate() {
            let Some(at) = content.find("void ") else {
                continue;
            };
            let rest = &content[at + 5..];
            let Some(open) = rest.find('(') else { continue };
            let name = rest[..open].trim();
            decls.push(MethodDecl {
                name: name.to_owned(),
                uri: uri.clone(),
                line,
                column: at + 5 + rest[..open].find(name).unwrap_or(0),
                end: lines.saturating_sub(1),
            });
        }
        for i in first..decls.len().saturating_sub(1) {
            decls[i].end = decls[i + 1].line.saturating_sub(1);
        }
    }
    decls
}

/// Call sites (`name(` outside a `void` declaration) inside `caller`'s body.
fn call_sites(
    documents: &std::collections::BTreeMap<String, String>,
    caller: &MethodDecl,
    name: &str,
) -> Vec<Value> {
    let Some(text) = documents.get(&caller.uri) else {
        return Vec::new();
    };
    text.lines()
        .enumerate()
        .skip(caller.line + 1)
        .take(caller.end.saturating_sub(caller.line))
        .filter(|(_, content)| !content.contains("void "))
        .filter_map(|(line, content)| {
            let column = content.find(&format!("{name}("))?;
            Some(json!({"start": {"line": line, "character": column},
                "end": {"line": line, "character": column + name.len()}}))
        })
        .collect()
}
