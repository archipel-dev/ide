//! Merged Manifest view (`android.manifest.merged`).

use crate::{IdeState, spec};
use archipel_ide_android::merged_manifest::MergedManifest;
use archipel_ide_command::{Command, CommandError, CommandRegistry};
use serde_json::{Value, json};

/// The merged manifest shown in the view, or why it is unavailable.
#[derive(Debug, Default)]
pub struct MergedManifestView {
    pub open: bool,
    pub title: String,
    /// Gradle task that produces the blame report, e.g. `:app:assembleDebug`.
    pub build_task: Option<String>,
    pub result: Option<Result<MergedManifest, String>>,
}

impl IdeState {
    /// Loads the merged manifest of the app module's active variant.
    pub fn load_merged_manifest(&mut self, variant: Option<String>) -> Result<(), String> {
        let project = self
            .android
            .project
            .as_ref()
            .ok_or("The open project is not an Android project.")?;
        let module = project
            .app_module()
            .or(project.modules.first())
            .ok_or("No Android module.")?;
        let gradle_path = module.gradle_path.clone();
        let module_dir = project.root.join(&module.dir);
        let variant = variant
            .or_else(|| self.active_variant(&gradle_path))
            .ok_or("No build variant.")?;
        let view = &mut self.android.merged_manifest;
        view.title = format!("Merged Manifest — {gradle_path} ({variant})");
        let mut capitalised = variant.chars();
        let capitalised: String = capitalised
            .next()
            .map(|c| c.to_uppercase().chain(capitalised).collect())
            .unwrap_or_default();
        view.build_task = Some(format!("{gradle_path}:assemble{capitalised}"));
        view.result = Some(MergedManifest::load(&module_dir, &variant));
        view.open = true;
        Ok(())
    }
}

pub(crate) fn register(registry: &mut CommandRegistry<IdeState>) {
    registry
        .register(Command::new(
            spec(
                "android.manifest.merged",
                "Show Merged Manifest",
                "Android",
                None,
                r#"{"variant":"string?"}"#,
            ),
            |state: &IdeState| state.android.project.is_some(),
            |state: &mut IdeState, params| {
                let variant = params
                    .get("variant")
                    .and_then(Value::as_str)
                    .map(str::to_owned);
                state
                    .load_merged_manifest(variant)
                    .map_err(CommandError::Failed)?;
                match state.android.merged_manifest.result.as_ref() {
                    Some(Ok(merged)) => Ok(json!({
                        "lines": merged.lines.len(),
                        "sources": merged.sources,
                        "blameFile": merged.blame_file,
                    })),
                    Some(Err(error)) => Err(CommandError::Failed(error.clone())),
                    None => Ok(Value::Null),
                }
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "android.manifest.text",
                "Show Manifest Text",
                "Android",
                None,
                "{}",
            ),
            |state: &IdeState| state.android.merged_manifest.open,
            |state: &mut IdeState, _| {
                state.android.merged_manifest.open = false;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
}

#[cfg(test)]
mod tests {
    use crate::{IdeState, built_in_commands};
    use serde_json::json;

    #[test]
    fn loads_the_blame_report_of_the_active_variant() {
        let root =
            std::env::temp_dir().join(format!("archipel-ide-core-merged-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let write = |rel: &str, text: &str| {
            let path = root.join(rel);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        };
        write("settings.gradle.kts", "include(\":app\")\n");
        write(
            "app/build.gradle.kts",
            "plugins { id(\"com.android.application\") }\nandroid { namespace = \"a.b\" }\n",
        );
        let commands = built_in_commands();
        let mut state = IdeState::default();
        commands
            .execute("project.open", &mut state, json!({"path": root}))
            .unwrap();
        let error = commands
            .execute("android.manifest.merged", &mut state, json!({}))
            .unwrap_err();
        assert!(error.to_string().contains("assembleDebug"), "{error}");
        assert!(
            state.android.merged_manifest.open,
            "the view explains the missing build"
        );
        assert_eq!(
            state.android.merged_manifest.build_task.as_deref(),
            Some(":app:assembleDebug")
        );
        write(
            "app/build/intermediates/manifest_merge_blame_file/debug/processDebugMainManifest/manifest-merger-blame-debug-report.txt",
            "1<manifest package=\"a.b\">\n2    <application />\n2-->/w/app/src/main/AndroidManifest.xml:2:5-20\n3</manifest>\n",
        );
        let result = commands
            .execute("android.manifest.merged", &mut state, json!({}))
            .unwrap();
        assert_eq!(result["lines"], 3);
        assert_eq!(result["sources"][0]["kind"], "injected");
        assert_eq!(result["sources"][1]["kind"], "file");
        assert_eq!(
            state.android.merged_manifest.title,
            "Merged Manifest — :app (debug)"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn text_command_returns_to_the_text_tab() {
        let commands = built_in_commands();
        let mut state = IdeState::default();
        assert!(!commands.is_enabled("android.manifest.text", &state));
        state.android.merged_manifest.open = true;
        commands
            .execute("android.manifest.text", &mut state, json!({}))
            .unwrap();
        assert!(!state.android.merged_manifest.open);
    }
}
