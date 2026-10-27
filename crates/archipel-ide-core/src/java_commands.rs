//! Native Java intelligence in the app (Phase 4): the Java service (indexes on the project
//! model, resolve errors, import fixes, unused imports) registered as a language service of the
//! analysis host, driven once per frame by [`IdeState::poll_java`].
//!
//! State lives in the service, not in `IdeState` (platform kernel rule).

use crate::IdeState;
use archipel_ide_analysis::{Change, ServiceRegistry};
use archipel_ide_java::diagnostics::{JavaService, JavaSettings};
use archipel_ide_java::import_edit::ImportSettings;
use archipel_ide_settings::{SettingDef, SettingKind, SettingValue};
use std::sync::Arc;

/// Settings of the native Java pass (Settings | Editor | Inspections).
pub const SETTING_JAVA_NATIVE: &str = "java.native.analysis";
pub const SETTING_JAVA_UNUSED_IMPORT: &str = "java.native.unusedImport";
pub const SETTING_JAVA_EXCLUDE: &str = "java.autoImport.exclude";

/// Registers the Java service and its daemon pass.
pub(crate) fn java_services(registry: &mut ServiceRegistry) {
    JavaService::new().register(registry);
}

pub(crate) fn register_commands(registry: &mut archipel_ide_command::CommandRegistry<IdeState>) {
    use archipel_ide_command::Command;
    use serde_json::{Value, json};
    registry
        .register(Command::new(
            crate::spec("java.chooser", "Import Chooser", "Code", None, "{}"),
            |_| true,
            |state: &mut IdeState, _| {
                let chooser = state.java_service().and_then(|s| s.chooser());
                Ok(json!({
                    "open": chooser.is_some(),
                    "title": chooser.as_ref().map(|c| c.title.clone()),
                    "rows": chooser.map(|c| c.rows.iter().map(|r| match &r.detail {
                        Some(d) => format!("{} {d}", r.label),
                        None => r.label.clone(),
                    }).collect::<Vec<_>>()),
                }))
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            crate::spec(
                "java.chooseImport",
                "Choose Import",
                "Code",
                None,
                r#"{"index":0}"#,
            ),
            |state: &IdeState| state.java_service().is_some_and(|s| s.chooser().is_some()),
            |state: &mut IdeState, args: Value| {
                let index = crate::usize_param(&args, "index").unwrap_or(0);
                let command = state
                    .java_service()
                    .and_then(|s| s.choose(index))
                    .ok_or_else(|| {
                        archipel_ide_command::CommandError::Failed("no such choice".into())
                    })?;
                state
                    .execute_mod_command(&command)
                    .map(|m| json!({"message": m}))
                    .map_err(archipel_ide_command::CommandError::Failed)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            crate::spec(
                "java.cancelChooser",
                "Cancel Import Chooser",
                "Code",
                None,
                "{}",
            ),
            |_| true,
            |state: &mut IdeState, _| {
                if let Some(s) = state.java_service() {
                    s.close_chooser();
                }
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
}

impl IdeState {
    pub fn java_service(&self) -> Option<Arc<JavaService>> {
        JavaService::from_registry(self.analysis.host.registry())
    }

    fn ensure_java_settings(&mut self) {
        let settings = self.settings();
        let mut settings = settings.write();
        let registry = &mut settings.registry;
        if registry.setting(SETTING_JAVA_NATIVE).is_some()
            || registry.page(crate::INSPECTIONS_PAGE).is_none()
        {
            return;
        }
        let defs = [
            SettingDef::new(
                SETTING_JAVA_NATIVE,
                crate::INSPECTIONS_PAGE,
                "Native Java analysis",
                SettingKind::Bool,
                SettingValue::Bool(true),
            )
            .group("Java")
            .description("Cannot-resolve errors, import fixes and unused imports from the built-in Java indexes; the language server's duplicates are hidden"),
            SettingDef::new(
                SETTING_JAVA_UNUSED_IMPORT,
                crate::INSPECTIONS_PAGE,
                "Unused import",
                SettingKind::Bool,
                SettingValue::Bool(true),
            )
            .group("Java"),
            SettingDef::new(
                SETTING_JAVA_EXCLUDE,
                crate::INSPECTIONS_PAGE,
                "Exclude from auto-import",
                SettingKind::Text,
                SettingValue::Text(String::new()),
            )
            .group("Java")
            .description("Comma separated class, member or package names (org.mockito.internal.*) never offered by import fixes"),
        ];
        for def in defs {
            let _ = registry.register_setting(def);
        }
    }

    /// Per-frame driver: settings, project model, buffers. Returns true when the daemon must run
    /// its Java pass again.
    pub(crate) fn poll_java(&mut self) -> bool {
        self.ensure_java_settings();
        let Some(service) = self.java_service() else {
            return false;
        };
        let (native, unused, exclude) = {
            let settings = self.settings();
            let settings = settings.read();
            (
                settings.bool(SETTING_JAVA_NATIVE).unwrap_or(true),
                settings.bool(SETTING_JAVA_UNUSED_IMPORT).unwrap_or(true),
                settings.text(SETTING_JAVA_EXCLUDE).unwrap_or_default(),
            )
        };
        let mut wanted = JavaSettings {
            native,
            unused_imports: unused,
            imports: ImportSettings::default(),
        };
        wanted.imports.exclude = exclude
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .collect();
        let mut changed = false;
        if service.settings() != wanted {
            service.set_settings(wanted);
            if let Some(daemon) = self.daemon_service() {
                let mut shared = daemon.shared().native_wins.write();
                if native {
                    shared.insert("java".into());
                } else {
                    shared.remove("java");
                }
            }
            changed = true;
        }
        let Some(root) = self.project.as_ref().map(|p| p.root().to_path_buf()) else {
            return changed;
        };
        let (model, loading) = match self.project_model() {
            Some(pm) => {
                let state = pm.read();
                (state.model.clone(), state.is_busy())
            }
            None => (None, false),
        };
        let cache = self.analysis.index_cache_dir.clone().map(|dir| {
            dir.join("index").join(format!(
                "{:016x}",
                archipel_ide_lsp::registry::stable_hash(&root.to_string_lossy())
            ))
        });
        changed |= service.sync(&root, model, loading, cache);
        if native {
            for document in &self.documents {
                if document.path.extension().and_then(|e| e.to_str()) == Some("java") {
                    let rev = document.text.revision();
                    service.document_seen(&document.path, rev, || document.text.text().into());
                }
            }
        }
        if changed {
            self.analysis.host.apply(Change::SettingsChanged);
        }
        changed
    }
}
