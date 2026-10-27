//! Android SDK and project commands (`android.*`).

use crate::{IdeState, spec, string_param};
use archipel_ide_android::{AndroidProject, SdkDetection, SystemEnvironment};
use archipel_ide_command::{Command, CommandError, CommandRegistry};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

/// Android tooling state for the open project.
#[derive(Default)]
pub struct AndroidState {
    /// Root of the open project (for `local.properties`).
    pub(crate) root: Option<PathBuf>,
    /// SDK location chosen in the Android SDK page, if any.
    pub sdk_setting: Option<PathBuf>,
    /// Last SDK discovery result.
    pub sdk: Option<SdkDetection>,
    /// The open project, when it is an Android Gradle project.
    pub project: Option<AndroidProject>,
    /// Whether the Android SDK settings page is showing.
    pub sdk_manager_open: bool,
    pub devices: crate::android_devices::DeviceHub,
    pub sdk_repo: crate::android_sdk_repo::SdkRepoState,
    pub logcat: crate::android_logcat::LogcatState,
    pub deploy: crate::android_deploy::DeployState,
    /// Active build variant per module Gradle path (Build Variants tool window).
    pub active_variants: std::collections::BTreeMap<String, String>,
    pub merged_manifest: crate::android_manifest::MergedManifestView,
    /// Project tool window shows the Android view (default for Android projects).
    pub android_view: bool,
    pub resource_manager: crate::android_tools::ResourceManagerState,
    pub mirror: crate::android_tools::MirrorState,
    pub layout: crate::android_tools::LayoutEditorState,
}

impl AndroidState {
    pub(crate) fn project_opened(&mut self, root: &Path) {
        self.root = Some(root.to_owned());
        self.project = AndroidProject::detect(root);
        self.android_view = self.project.is_some();
        self.active_variants.clear();
        self.sdk = None;
        if self.project.is_some() {
            self.detect_sdk(Some(root));
            // Like the reference IDE, start the adb server for Android projects.
            let _ = self.ensure_device_tracking();
        }
    }

    pub fn detect_sdk(&mut self, project_root: Option<&Path>) -> &SdkDetection {
        self.sdk.insert(SdkDetection::detect(
            self.sdk_setting.as_deref(),
            project_root,
            &SystemEnvironment,
        ))
    }
}

impl IdeState {
    /// Uses `path` as the Android SDK. For an Android project it is also
    /// recorded in `local.properties`, as Android Studio does on sync.
    pub fn set_android_sdk(&mut self, path: &Path) -> Result<(), String> {
        archipel_ide_android::validate_sdk(path)
            .map_err(|problem| format!("{} is not an Android SDK: {problem}", path.display()))?;
        let root = self.project_root();
        let is_android = self.android_tooling().is_android_project();
        if let (Some(root), true) = (&root, is_android) {
            archipel_ide_android::write_local_properties_sdk_dir(root, path)
                .map_err(|error| format!("cannot update local.properties: {error}"))?;
        }
        self.android_tooling().set_sdk(path, root.as_deref());
        self.status = format!("Android SDK: {}", path.display());
        Ok(())
    }
}

impl IdeState {
    /// The Android project view, computed from the project tree's file index.
    pub fn android_view_nodes(&self) -> Vec<archipel_ide_android::view::ViewNode> {
        let tooling = self.android_tooling();
        let guard = tooling.read();
        let Some(project) = &guard.project else {
            return Vec::new();
        };
        let files: Vec<PathBuf> = self
            .files
            .iter()
            .filter(|entry| !entry.is_dir)
            .map(|entry| entry.relative_path.clone())
            .collect();
        archipel_ide_android::view::android_view(project, &files)
    }
}

fn detection_json(detection: &SdkDetection) -> Value {
    json!({
        "sdk": detection.sdk,
        "probes": detection.probes,
        "problem": detection.problem(),
    })
}

pub(crate) fn register(registry: &mut CommandRegistry<IdeState>) {
    registry
        .register(Command::new(
            spec(
                "android.projectView.select",
                "Select Project View",
                "Android",
                None,
                r#"{"view":"android|project"}"#,
            ),
            |state: &IdeState| state.android_tooling().read().project.is_some(),
            |state: &mut IdeState, params| {
                state.android_tooling().write().android_view =
                    match string_param(&params, "view")?.as_str() {
                        "android" => true,
                        "project" => false,
                        other => {
                            return Err(CommandError::InvalidParams(format!(
                                "unknown project view `{other}`; expected android or project"
                            )));
                        }
                    };
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "android.projectView.tree",
                "Android Project View",
                "Android",
                None,
                "{}",
            ),
            |state: &IdeState| state.android_tooling().read().project.is_some(),
            |state: &mut IdeState, _| Ok(json!({ "nodes": state.android_view_nodes() })),
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "android.sdk.detect",
                "Detect Android SDK",
                "Android",
                None,
                "{}",
            ),
            |_| true,
            |state: &mut IdeState, _| {
                let root = state.project_root();
                let tooling = state.android_tooling();
                let mut guard = tooling.write();
                let detection = guard.detect_sdk(root.as_deref());
                Ok(detection_json(detection))
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec("android.sdk.manager", "SDK Manager", "Android", None, "{}"),
            |_| true,
            |state: &mut IdeState, _| {
                let root = state.project_root();
                state.android_tooling().detect_sdk(root.as_deref());
                state.android_tooling().write().sdk_manager_open = true;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "android.sdk.setLocation",
                "Set Android SDK Location",
                "Android",
                None,
                r#"{"path":"string"}"#,
            ),
            |_| true,
            |state: &mut IdeState, params| {
                let path = string_param(&params, "path")?;
                let path = path.trim();
                if path.is_empty() {
                    return Err(CommandError::InvalidParams(
                        "the Android SDK location is empty".into(),
                    ));
                }
                state
                    .set_android_sdk(Path::new(path))
                    .map_err(CommandError::Failed)?;
                Ok(detection_json(
                    state
                        .android_tooling()
                        .read()
                        .sdk
                        .as_ref()
                        .expect("detected"),
                ))
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "android.project.detect",
                "Detect Android Project",
                "Android",
                None,
                "{}",
            ),
            |state: &IdeState| state.project.is_some(),
            |state: &mut IdeState, _| {
                let root = state
                    .project_root()
                    .ok_or_else(|| CommandError::Disabled("android.project.detect".into()))?;
                state.android_tooling().project_opened(&root);
                Ok(json!({ "project": state.android_tooling().read().project }))
            },
        ))
        .expect("unique built-in id");
}

#[cfg(test)]
mod tests {
    use crate::{IdeState, built_in_commands};
    use serde_json::json;
    use std::path::{Path, PathBuf};

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "archipel-ide-core-android-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir.canonicalize().unwrap()
    }

    fn write(root: &Path, rel: &str, text: &str) {
        let path = root.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    fn fake_sdk(root: &Path) {
        write(
            root,
            "platforms/android-35/package.xml",
            r#"<r><localPackage path="platforms;android-35"><type-details><api-level>35</api-level></type-details><revision><major>2</major></revision><display-name>Android SDK Platform 35</display-name></localPackage></r>"#,
        );
        write(
            root,
            "platform-tools/package.xml",
            r#"<r><localPackage path="platform-tools"><revision><major>37</major><minor>0</minor><micro>1</micro></revision><display-name>Android SDK Platform-Tools</display-name></localPackage></r>"#,
        );
    }

    fn android_project(root: &Path, sdk: &Path) {
        write(root, "settings.gradle.kts", "include(\":app\")\n");
        write(
            root,
            "app/build.gradle.kts",
            "plugins { id(\"com.android.application\") }\nandroid {\n    namespace = \"com.example.app\"\n    compileSdk = 35\n}\n",
        );
        write(
            root,
            "app/src/main/AndroidManifest.xml",
            r#"<manifest xmlns:android="http://schemas.android.com/apk/res/android"><application><activity android:name=".MainActivity"><intent-filter><action android:name="android.intent.action.MAIN"/><category android:name="android.intent.category.LAUNCHER"/></intent-filter></activity></application></manifest>"#,
        );
        write(
            root,
            "local.properties",
            &format!("# local\nsdk.dir={}\n", sdk.display()),
        );
    }

    #[test]
    fn opening_an_android_project_detects_modules_and_sdk() {
        let base = temp("open");
        let sdk = base.join("sdk");
        fake_sdk(&sdk);
        let project = base.join("project");
        android_project(&project, &sdk);
        let commands = built_in_commands();
        let mut state = IdeState::default();
        commands
            .execute("project.open", &mut state, json!({"path": project}))
            .unwrap();
        let android = state.android_tooling().project().unwrap();
        assert_eq!(
            android
                .app_module()
                .unwrap()
                .launcher
                .as_ref()
                .unwrap()
                .class_name,
            "com.example.app.MainActivity"
        );
        let result = commands
            .execute("android.sdk.detect", &mut state, json!({}))
            .unwrap();
        assert_eq!(result["sdk"]["source"], "localProperties");
        assert_eq!(result["sdk"]["packages"][0]["path"], "platforms;android-35");
        assert_eq!(result["problem"], serde_json::Value::Null);

        assert!(
            state.android_tooling().read().android_view,
            "Android view is the default"
        );
        let view = commands
            .execute("android.projectView.tree", &mut state, json!({}))
            .unwrap();
        assert_eq!(view["nodes"][0]["label"], "app");
        assert_eq!(view["nodes"][0]["children"][0]["label"], "manifests");
        assert_eq!(view["nodes"][1]["label"], "Gradle Scripts");
        commands
            .execute(
                "android.projectView.select",
                &mut state,
                json!({"view": "project"}),
            )
            .unwrap();
        assert!(!state.android_tooling().read().android_view);
        assert!(
            commands
                .execute(
                    "android.projectView.select",
                    &mut state,
                    json!({"view": "packages"})
                )
                .is_err()
        );
        let project_json = commands
            .execute("android.project.detect", &mut state, json!({}))
            .unwrap();
        assert_eq!(project_json["project"]["modules"][0]["gradlePath"], ":app");

        commands
            .execute("android.sdk.manager", &mut state, json!({}))
            .unwrap();
        assert!(state.android_tooling().read().sdk_manager_open);
        std::fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn set_location_validates_and_updates_local_properties() {
        let base = temp("set");
        let sdk = base.join("other-sdk");
        fake_sdk(&sdk);
        let project = base.join("project");
        android_project(&project, &base.join("missing"));
        let commands = built_in_commands();
        let mut state = IdeState::default();
        commands
            .execute("project.open", &mut state, json!({"path": project}))
            .unwrap();
        let error = commands
            .execute(
                "android.sdk.setLocation",
                &mut state,
                json!({"path": base.join("nope")}),
            )
            .unwrap_err();
        assert!(error.to_string().contains("does not exist"), "{error}");
        assert!(
            commands
                .execute("android.sdk.setLocation", &mut state, json!({"path": " "}))
                .is_err()
        );
        let result = commands
            .execute("android.sdk.setLocation", &mut state, json!({"path": sdk}))
            .unwrap();
        assert_eq!(result["sdk"]["source"], "setting");
        let properties = std::fs::read_to_string(project.join("local.properties")).unwrap();
        assert!(properties.starts_with("# local\nsdk.dir="), "{properties}");
        assert!(!properties.contains("missing"), "{properties}");
        std::fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn non_android_project_has_no_android_state() {
        let base = temp("plain");
        write(&base, "Cargo.toml", "[package]\nname='x'\n");
        let mut state = IdeState::default();
        built_in_commands()
            .execute("project.open", &mut state, json!({"path": base}))
            .unwrap();
        assert!(state.android_tooling().read().project.is_none());
        assert!(state.android_tooling().read().sdk.is_none());
        std::fs::remove_dir_all(base).unwrap();
    }
}
