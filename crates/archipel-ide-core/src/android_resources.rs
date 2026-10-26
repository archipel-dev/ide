//! Android resource navigation (`android.resources.*`): `R.string.x` and
//! `@string/x` under the caret jump to their declarations.

use crate::{IdeState, spec, string_param};
use archipel_ide_android::resources::{ResourceDef, ResourceIndex, ResourceRef, reference_at};
use archipel_ide_command::{Command, CommandError, CommandRegistry};
use archipel_ide_text::{Selection, ops};
use serde_json::{Value, json};
use std::path::PathBuf;

impl IdeState {
    fn resource_index(&self) -> Result<ResourceIndex, String> {
        let project = self.project.as_ref().ok_or("no project is open")?;
        if self.android.project.is_none() {
            return Err("The open project is not an Android project.".into());
        }
        let files: Vec<PathBuf> = self
            .files
            .iter()
            .filter(|f| !f.is_dir)
            .map(|f| f.relative_path.clone())
            .collect();
        Ok(ResourceIndex::build(project.root(), &files))
    }

    /// The resource reference under the primary caret of the active editor.
    pub fn resource_at_caret(&self) -> Option<ResourceRef> {
        let document = self.documents.get(self.active_document?)?;
        let caret = document.selections.primary().head;
        let text = &document.text;
        let line = text.char_to_line(caret.min(text.len_chars()));
        let (start, end, _) = ops::line_bounds(text, line);
        let line_text = text.slice(start, end).ok()?;
        reference_at(&line_text, caret.saturating_sub(start))
    }

    /// Opens `def` and puts the caret at the start of its line.
    pub(crate) fn open_resource(&mut self, def: &ResourceDef) -> Result<(), String> {
        self.open_file(&def.path)?;
        self.edit_active(|d| {
            let line = (def.line - 1).min(d.text.len_lines().saturating_sub(1));
            let (start, end, _) = ops::line_bounds(&d.text, line);
            let indent = d
                .text
                .slice(start, end)
                .unwrap_or_default()
                .chars()
                .take_while(|c| c.is_whitespace())
                .count();
            d.selections.set_single(Selection::caret(start + indent));
            Ok(())
        })
    }
}

fn defs_json(defs: &[&ResourceDef]) -> Vec<Value> {
    defs.iter()
        .map(|d| json!({"path": d.path, "line": d.line, "qualifier": d.qualifier}))
        .collect()
}

pub(crate) fn register(registry: &mut CommandRegistry<IdeState>) {
    registry
        .register(Command::new(
            spec(
                "android.resources.gotoDeclaration",
                "Go to Android Resource",
                "Android",
                Some("⌘B"),
                "{}",
            ),
            |state: &IdeState| state.android.project.is_some() && state.active_document.is_some(),
            |state: &mut IdeState, _| {
                let reference = state.resource_at_caret().ok_or_else(|| {
                    CommandError::Failed("No Android resource reference at the caret.".into())
                })?;
                let index = state.resource_index().map_err(CommandError::Failed)?;
                let defs = index.lookup(&reference);
                let first = defs.first().copied().cloned().ok_or_else(|| {
                    CommandError::Failed(format!(
                        "Cannot find declaration of R.{}.{}",
                        reference.kind, reference.name
                    ))
                })?;
                let targets = defs_json(&defs);
                state.open_resource(&first).map_err(CommandError::Failed)?;
                state.status = if targets.len() > 1 {
                    format!(
                        "R.{}.{}: {} declarations, opened {}",
                        reference.kind,
                        reference.name,
                        targets.len(),
                        first.path.display()
                    )
                } else {
                    format!("R.{}.{}", reference.kind, reference.name)
                };
                Ok(json!({"reference": reference, "targets": targets}))
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "android.resources.find",
                "Find Android Resource",
                "Android",
                None,
                r#"{"type":"string","name":"string"}"#,
            ),
            |state: &IdeState| state.android.project.is_some(),
            |state: &mut IdeState, params| {
                let reference = ResourceRef {
                    kind: string_param(&params, "type")?,
                    name: string_param(&params, "name")?,
                };
                let index = state.resource_index().map_err(CommandError::Failed)?;
                Ok(json!({"targets": defs_json(&index.lookup(&reference))}))
            },
        ))
        .expect("unique built-in id");
}

#[cfg(test)]
mod tests {
    use crate::{IdeState, built_in_commands};
    use serde_json::json;
    use std::path::Path;

    fn write(root: &Path, rel: &str, text: &str) {
        let path = root.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    #[test]
    fn goto_declaration_from_kotlin_and_xml() {
        let root =
            std::env::temp_dir().join(format!("archipel-ide-core-res-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        write(&root, "settings.gradle.kts", "include(\":app\")\n");
        write(
            &root,
            "app/build.gradle.kts",
            "plugins { id(\"com.android.application\") }\nandroid { namespace = \"a.b\" }\n",
        );
        write(
            &root,
            "app/src/main/java/a/b/Main.kt",
            "package a.b\nclass Main {\n    val title = getString(R.string.app_name)\n}\n",
        );
        write(
            &root,
            "app/src/main/AndroidManifest.xml",
            "<manifest>\n  <application android:label=\"@string/app_name\"/>\n</manifest>\n",
        );
        write(
            &root,
            "app/src/main/res/values/strings.xml",
            "<resources>\n    <string name=\"other\">x</string>\n    <string name=\"app_name\">App</string>\n</resources>\n",
        );
        let commands = built_in_commands();
        let mut state = IdeState::default();
        commands
            .execute("project.open", &mut state, json!({"path": root}))
            .unwrap();
        commands
            .execute(
                "editor.openFile",
                &mut state,
                json!({"path": "app/src/main/java/a/b/Main.kt"}),
            )
            .unwrap();
        commands
            .execute(
                "editor.gotoLine",
                &mut state,
                json!({"line": 3, "column": 35}),
            )
            .unwrap();
        let result = commands
            .execute("android.resources.gotoDeclaration", &mut state, json!({}))
            .unwrap();
        assert_eq!(
            result["reference"],
            json!({"kind": "string", "name": "app_name"})
        );
        let active = &state.documents[state.active_document.unwrap()];
        assert_eq!(
            active.path,
            Path::new("app/src/main/res/values/strings.xml")
        );
        let caret = active.selections.primary().head;
        assert_eq!(active.text.char_to_line(caret), 2, "third line, zero-based");

        commands
            .execute(
                "editor.openFile",
                &mut state,
                json!({"path": "app/src/main/AndroidManifest.xml"}),
            )
            .unwrap();
        commands
            .execute(
                "editor.gotoLine",
                &mut state,
                json!({"line": 2, "column": 40}),
            )
            .unwrap();
        commands
            .execute("android.resources.gotoDeclaration", &mut state, json!({}))
            .unwrap();
        assert!(
            state.status.contains("R.string.app_name"),
            "{}",
            state.status
        );

        commands
            .execute(
                "editor.gotoLine",
                &mut state,
                json!({"line": 1, "column": 1}),
            )
            .unwrap();
        let error = commands
            .execute("android.resources.gotoDeclaration", &mut state, json!({}))
            .unwrap_err();
        assert!(
            error.to_string().contains("No Android resource reference"),
            "{error}"
        );
        let found = commands
            .execute(
                "android.resources.find",
                &mut state,
                json!({"type": "string", "name": "nope"}),
            )
            .unwrap();
        assert_eq!(found["targets"], json!([]));
        std::fs::remove_dir_all(root).unwrap();
    }
}
