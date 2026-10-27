//! The app only knows languages through their packs (PLAT-LANG-005, `docs/architecture/
//! language-packs.md`): with the Java pack installed Java intelligence is there; removing the
//! pack removes it again, and nothing else in the app notices (no panic, no leftover).

use archipel_ide_analysis::daemon::Pass;
use archipel_ide_analysis::{ModCommand, ServiceRegistry};
use archipel_ide_core::{IdeState, built_in_commands};
use archipel_ide_language::LanguageId;
use serde_json::json;
use std::path::Path;

fn pass_ids(registry: &ServiceRegistry) -> Vec<&'static str> {
    registry
        .entries::<dyn Pass>(LanguageId("java"))
        .into_iter()
        .map(|(registration, _)| registration.id)
        .collect()
}

#[test]
fn removing_the_java_pack_disables_java_features_without_breaking_the_rest() {
    let commands = built_in_commands();
    let mut state = IdeState::default();
    state.poll_analysis();

    // With the pack: the native pass, the activity, the server and the chooser commands.
    assert!(pass_ids(state.analysis.host.registry()).contains(&"java-resolve"));
    assert!(state.language_packs().is_installed("java"));
    assert!(state.language_status().iter().any(|(id, _)| *id == "java"));
    assert!(
        state
            .lsp
            .registry()
            .for_path(Path::new("App.java"))
            .is_some()
    );
    assert!(state.daemon_service().is_some());
    assert!(commands.contains("java.chooseImport"));
    assert!(state.import_chooser().is_none());
    state
        .execute_mod_command(&ModCommand::ChooseFrom {
            title: "Method to Import".into(),
            rows: Vec::new(),
        })
        .unwrap();
    let (provider, chooser) = state.import_chooser().expect("the chooser opened");
    assert_eq!(provider, "java");
    assert_eq!(chooser.title, "Method to Import");
    commands
        .execute("java.cancelChooser", &mut state, json!({}))
        .unwrap();
    assert!(state.import_chooser().is_none());

    // Without it.
    assert!(state.language_packs().uninstall("java"));
    state.poll_analysis();
    state.poll_daemon(&[]);
    let passes = pass_ids(state.analysis.host.registry());
    assert!(!passes.contains(&"java-resolve"));
    // The platform's own passes are still there.
    assert!(passes.contains(&"syntax"));
    assert!(!state.language_status().iter().any(|(id, _)| *id == "java"));
    assert!(
        state
            .lsp
            .registry()
            .for_path(Path::new("App.java"))
            .is_none()
    );
    // Other languages and the platform's own services stay.
    assert!(
        state
            .lsp
            .registry()
            .for_path(Path::new("main.rs"))
            .is_some()
    );
    assert!(state.daemon_service().is_some());
    // The commands still exist (they are registered once at start) and refuse politely.
    assert!(state.import_chooser().is_none());
    assert_eq!(
        state
            .execute_mod_command(&ModCommand::ChooseFrom {
                title: "x".into(),
                rows: Vec::new(),
            })
            .unwrap_err(),
        "no chooser available"
    );
    assert!(
        commands
            .execute("java.chooseImport", &mut state, json!({"index": 0}))
            .is_err()
    );
    commands
        .execute("java.cancelChooser", &mut state, json!({}))
        .unwrap();
    // Polling a frame with no Java pack is a no-op, not a failure.
    state.poll_analysis();
}
