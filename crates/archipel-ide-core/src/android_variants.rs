//! Build Variants tool window state (`android.variants.*`).

use crate::{IdeState, spec, string_param};
use archipel_ide_command::{Command, CommandError, CommandRegistry};
use serde_json::{Value, json};

impl IdeState {
    /// Active variant of a module: the selection, else the module default.
    pub fn active_variant(&self, gradle_path: &str) -> Option<String> {
        let tooling = self.android_tooling();
        if let Some(name) = tooling.active_variant(gradle_path) {
            return Some(name);
        }
        let project = tooling.project()?;
        let module = project
            .modules
            .iter()
            .find(|m| m.gradle_path == gradle_path)?;
        module.variants.default_variant().map(|v| v.name.clone())
    }

    pub fn select_variant(&mut self, gradle_path: &str, variant: &str) -> Result<(), String> {
        let project = self
            .android_tooling()
            .project()
            .ok_or("The open project is not an Android project.")?;
        let module = project
            .modules
            .iter()
            .find(|m| m.gradle_path == gradle_path)
            .ok_or_else(|| format!("no Android module {gradle_path}"))?;
        if module.variants.find(variant).is_none() {
            let known: Vec<_> = module
                .variants
                .variants
                .iter()
                .map(|v| v.name.as_str())
                .collect();
            return Err(format!(
                "{gradle_path} has no build variant `{variant}`; known: {}",
                known.join(", ")
            ));
        }
        self.android_tooling()
            .write()
            .active_variants
            .insert(gradle_path.to_owned(), variant.to_owned());
        self.status = format!("{gradle_path}: active build variant {variant}");
        Ok(())
    }
}

pub(crate) fn register(registry: &mut CommandRegistry<IdeState>) {
    registry
        .register(Command::new(
            spec(
                "android.variants.list",
                "List Build Variants",
                "Android",
                None,
                "{}",
            ),
            |state: &IdeState| state.android_tooling().is_android_project(),
            |state: &mut IdeState, _| {
                let modules: Vec<Value> = state
                    .android_tooling().read()
                    .project
                    .iter()
                    .flat_map(|p| p.modules.iter())
                    .map(|m| {
                        json!({
                            "module": m.gradle_path,
                            "active": state.active_variant(&m.gradle_path),
                            "variants": m.variants.variants.iter().map(|v| &v.name).collect::<Vec<_>>(),
                        })
                    })
                    .collect();
                Ok(json!({ "modules": modules }))
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "android.variants.select",
                "Select Build Variant",
                "Android",
                None,
                r#"{"module":"string","variant":"string"}"#,
            ),
            |state: &IdeState| state.android_tooling().is_android_project(),
            |state: &mut IdeState, params| {
                let module = string_param(&params, "module")?;
                let variant = string_param(&params, "variant")?;
                state
                    .select_variant(&module, &variant)
                    .map_err(CommandError::InvalidParams)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
}

#[cfg(test)]
mod tests {
    use crate::{IdeState, ToolWindow, built_in_commands};
    use serde_json::json;

    #[test]
    fn select_and_list_variants() {
        let root =
            std::env::temp_dir().join(format!("archipel-ide-core-variants-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("app")).unwrap();
        std::fs::write(root.join("settings.gradle.kts"), "include(\":app\")\n").unwrap();
        std::fs::write(
            root.join("app/build.gradle.kts"),
            "plugins { id(\"com.android.application\") }\nandroid {\n  namespace = \"a.b\"\n  productFlavors {\n    create(\"free\") {}\n    create(\"paid\") {}\n  }\n}\n",
        )
        .unwrap();
        let commands = built_in_commands();
        let mut state = IdeState::default();
        commands
            .execute("project.open", &mut state, json!({"path": root}))
            .unwrap();
        let listed = commands
            .execute("android.variants.list", &mut state, json!({}))
            .unwrap();
        assert_eq!(listed["modules"][0]["active"], "freeDebug");
        assert_eq!(
            listed["modules"][0]["variants"].as_array().unwrap().len(),
            4
        );
        commands
            .execute(
                "android.variants.select",
                &mut state,
                json!({"module": ":app", "variant": "paidRelease"}),
            )
            .unwrap();
        assert_eq!(state.active_variant(":app").as_deref(), Some("paidRelease"));
        let error = commands
            .execute(
                "android.variants.select",
                &mut state,
                json!({"module": ":app", "variant": "nope"}),
            )
            .unwrap_err();
        assert!(error.to_string().contains("known: freeDebug"), "{error}");
        commands
            .execute("toolwindow.activate.buildVariants", &mut state, json!({}))
            .unwrap();
        assert!(state.tool_windows.is_visible(ToolWindow::BuildVariants));
        std::fs::remove_dir_all(root).unwrap();
    }
}
