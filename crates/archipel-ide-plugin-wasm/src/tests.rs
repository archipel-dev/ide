use super::*;
use archipel_ide_plugin_api::json;
use archipel_ide_plugin_api::testing::RecordingHost;
use std::fs;

fn example_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../plugins/hello-wasm")
}

fn manifest(permissions: &str) -> PluginManifest {
    let text = fs::read_to_string(example_dir().join("archipel-plugin.toml")).unwrap();
    PluginManifest::parse(&text.replace("permissions = [\"fs.project\"]", permissions)).unwrap()
}

fn load(permissions: &str) -> WasmPlugin {
    let loader = WasmLoader::new().unwrap();
    WasmPlugin::load(&loader.engine, manifest(permissions), &example_dir()).unwrap()
}

fn host_with_project(project: &Path, permissions: &[Permission]) -> RecordingHost {
    let mut host = RecordingHost::new("dev.archipel.hello-wasm").with_permissions(permissions);
    host.project = Some(project.to_path_buf());
    host
}

#[test]
fn component_declares_contributions_and_runs_commands() {
    let mut plugin = load("permissions = []");
    let contributions = plugin.contributions();
    assert!(
        contributions.problems().is_empty(),
        "{:?}",
        contributions.problems()
    );
    assert_eq!(contributions.commands.len(), 3);
    assert_eq!(
        contributions.tool_windows[0].icon_text.as_deref(),
        Some("W")
    );
    let mut host = RecordingHost::new("dev.archipel.hello-wasm");
    plugin.activate(&mut host).unwrap();
    assert_eq!(host.statuses, ["Hello WASM activated"]);
    host.settings.insert("greeting".into(), json!("Hi"));
    let result = plugin
        .execute("hello_wasm.greet", json!({"name": "Ada"}), &mut host)
        .unwrap();
    assert_eq!(result["message"], "Hi, Ada!");
    assert_eq!(host.statuses.last().unwrap(), "Hi, Ada!");
    let error = plugin
        .execute("hello_wasm.greet", json!({}), &mut host)
        .unwrap_err()
        .to_string();
    assert!(error.contains("expected string field"), "{error}");
    assert!(matches!(
        plugin.execute("nope.nope", Json::Null, &mut host),
        Err(CommandError::Unknown(_))
    ));
    assert_eq!(plugin.status_widget("count").unwrap().text, "WASM × 1");
}

#[test]
fn project_files_need_the_fs_project_permission() {
    let project =
        std::env::temp_dir().join(format!("archipel-ide-wasm-project-{}", std::process::id()));
    let _ = fs::remove_dir_all(&project);
    fs::create_dir_all(&project).unwrap();
    fs::write(project.join("Cargo.toml"), "").unwrap();
    fs::write(project.join("README.md"), "").unwrap();

    let mut allowed = load("permissions = [\"fs.project\"]");
    let mut host = host_with_project(&project, &[Permission::ProjectFiles]);
    allowed.activate(&mut host).unwrap();
    let files = allowed
        .execute("hello_wasm.listProject", Json::Null, &mut host)
        .unwrap();
    assert_eq!(files["files"], json!(["Cargo.toml", "README.md"]));
    assert_eq!(host.tool_windows, ["hello_wasm"]);

    let mut denied = load("permissions = []");
    let mut host = host_with_project(&project, &[]);
    denied.activate(&mut host).unwrap();
    let files = denied
        .execute("hello_wasm.listProject", Json::Null, &mut host)
        .unwrap();
    let first = files["files"][0].as_str().unwrap();
    assert!(first.contains("/project unavailable"), "{first}");
    fs::remove_dir_all(&project).unwrap();
}

#[test]
fn runaway_calls_run_out_of_fuel_and_the_plugin_recovers() {
    let mut plugin = load("permissions = []");
    // `hello_wasm.spin` is not contributed; declare it so execute calls the guest.
    plugin
        .contributions
        .commands
        .push(archipel_ide_plugin_api::CommandContribution::new(
            "hello_wasm.spin",
            "Spin",
        ));
    let mut host = RecordingHost::new("dev.archipel.hello-wasm");
    let error = plugin
        .execute("hello_wasm.spin", Json::Null, &mut host)
        .unwrap_err()
        .to_string();
    assert!(error.contains("out of fuel"), "{error}");
    // A fresh instance serves the next call.
    let result = plugin
        .execute("hello_wasm.sayHello", Json::Null, &mut host)
        .unwrap();
    assert_eq!(result["message"], "Hello, WASM!");
}

#[test]
fn declarative_views_render_and_events_reach_the_guest() {
    let mut plugin = load("permissions = []");
    let mut host = RecordingHost::new("dev.archipel.hello-wasm");
    let ctx = egui::Context::default();
    let view = ViewContext {
        view: "main".into(),
        placement: ViewPlacement::ToolWindow {
            id: "hello_wasm".into(),
        },
    };
    let _ = ctx.run(egui::RawInput::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            plugin.render_view(&view, &mut ViewFrame::new(ui), &mut host);
        });
    });
    assert_eq!(plugin.view_error, None);
    // Events go through on-view-event.
    let event =
        serde_json::to_string(&archipel_ide_plugin_api::ViewEvent::Clicked { id: "hello".into() })
            .unwrap();
    plugin
        .call(CALL_FUEL, Some(&mut host), |p, store| {
            p.call_on_view_event(store, "main", &event)
        })
        .unwrap();
    assert_eq!(host.statuses.last().unwrap(), "Hello, WASM!");
}

#[test]
fn invalid_components_fail_with_actionable_errors() {
    let dir = std::env::temp_dir().join(format!("archipel-ide-wasm-bad-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("hello_wasm.wasm"), b"\0asm not really").unwrap();
    let loader = WasmLoader::new().unwrap();
    let error = WasmPlugin::load(&loader.engine, manifest("permissions = []"), &dir)
        .err()
        .unwrap()
        .to_string();
    assert!(error.contains("not a valid component"), "{error}");
    fs::remove_file(dir.join("hello_wasm.wasm")).unwrap();
    let error = WasmPlugin::load(&loader.engine, manifest("permissions = []"), &dir)
        .err()
        .unwrap()
        .to_string();
    assert!(error.contains("hello_wasm.wasm"), "{error}");
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn loader_registers_wasm_plugins_with_the_registry() {
    use archipel_ide_plugins::{PluginLoader, PluginRegistry, PluginRuntime, PluginStatus};
    use std::rc::Rc;
    let root =
        std::env::temp_dir().join(format!("archipel-ide-wasm-registry-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let target = root.join("hello-wasm");
    fs::create_dir_all(&target).unwrap();
    for file in ["archipel-plugin.toml", "hello_wasm.wasm"] {
        fs::copy(example_dir().join(file), target.join(file)).unwrap();
    }
    let loader = WasmLoader::new().unwrap();
    assert_eq!(loader.kind(), PluginKind::Wasm);
    let mut registry = PluginRegistry::default();
    let mut runtime = PluginRuntime::default();
    runtime.add_loader(Rc::new(loader));
    registry.discover(&mut runtime, &root);
    assert_eq!(
        registry.get("dev.archipel.hello-wasm").unwrap().status,
        PluginStatus::Enabled
    );
    runtime
        .ensure_instance(&registry, "dev.archipel.hello-wasm")
        .unwrap();
    fs::remove_dir_all(&root).unwrap();
}

/// The example component as built before the rename: it targets the `jetthing:plugin@1.0.0`
/// world (kept in `tests/fixtures`; the checked-in example targets `archipel:plugin@1.0.0`).
fn legacy_fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/hello_wasm_legacy.wasm")
}

#[test]
fn the_example_targets_the_archipel_world_and_old_components_the_legacy_one() {
    let loader = WasmLoader::new().unwrap();
    let current =
        Component::from_file(&loader.engine, example_dir().join("hello_wasm.wasm")).unwrap();
    assert!(!targets_legacy_world(&loader.engine, &current));
    let legacy = Component::from_file(&loader.engine, legacy_fixture()).unwrap();
    assert!(targets_legacy_world(&loader.engine, &legacy));
}

#[test]
fn components_built_for_the_old_jetthing_world_still_load_and_run() {
    let dir = std::env::temp_dir().join(format!("archipel-ide-wasm-legacy-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::copy(legacy_fixture(), dir.join("hello_wasm.wasm")).unwrap();
    let loader = WasmLoader::new().unwrap();
    let mut plugin =
        WasmPlugin::load(&loader.engine, manifest("permissions = []"), &dir).expect("loads");
    assert_eq!(plugin.contributions().commands.len(), 3);
    let mut host = RecordingHost::new("dev.archipel.hello-wasm");
    plugin.activate(&mut host).unwrap();
    assert_eq!(host.statuses, ["Hello WASM activated"]);
    host.settings.insert("greeting".into(), json!("Hi"));
    let result = plugin
        .execute("hello_wasm.greet", json!({"name": "Ada"}), &mut host)
        .unwrap();
    assert_eq!(result["message"], "Hi, Ada!");
    assert_eq!(plugin.status_widget("count").unwrap().text, "WASM × 1");
    fs::remove_dir_all(&dir).unwrap();
}
