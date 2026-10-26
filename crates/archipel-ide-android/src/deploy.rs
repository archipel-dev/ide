//! Deploying an app to a device: locating the built APK, `adb install`, and
//! starting the launcher activity with `am start`.

use crate::adb::Adb;
use crate::{AndroidModule, AndroidProject};
use serde::Serialize;
use std::path::{Path, PathBuf};

/// What a Run of an Android app does after Gradle has built the APK.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeployPlan {
    pub serial: String,
    /// Gradle task that builds the APK, e.g. `:app:assembleDebug`.
    pub gradle_task: String,
    /// Module directory relative to the project root.
    pub module_dir: PathBuf,
    /// Variant name, e.g. `freeDebug`.
    pub variant: String,
    /// APK output directory relative to the module, e.g. `build/outputs/apk/free/debug`.
    pub apk_dir: PathBuf,
    pub package: String,
    /// Fully qualified launcher activity, if the manifest declares one.
    pub activity: Option<String>,
}

impl DeployPlan {
    /// Plans a deploy of the project's application module to `serial`, using
    /// `variant` or the module's default (first debug) variant.
    pub fn for_project(
        project: &AndroidProject,
        serial: &str,
        variant: Option<&str>,
    ) -> Result<Self, String> {
        let module = project
            .app_module()
            .ok_or("No application module: no Gradle module applies com.android.application.")?;
        Self::for_module(module, serial, variant)
    }

    pub fn for_module(
        module: &AndroidModule,
        serial: &str,
        variant: Option<&str>,
    ) -> Result<Self, String> {
        let package = module.package_name().ok_or_else(|| {
            format!(
                "Cannot determine the application ID of {}: set applicationId or namespace in {}.",
                module.gradle_path,
                module.build_file.display()
            )
        })?;
        let variant = match variant {
            Some(name) => module
                .variants
                .find(name)
                .ok_or_else(|| format!("{} has no build variant `{name}`", module.gradle_path))?,
            None => module
                .variants
                .default_variant()
                .ok_or_else(|| format!("{} has no build variants", module.gradle_path))?,
        };
        let prefix = if module.gradle_path == ":" {
            String::new()
        } else {
            module.gradle_path.clone()
        };
        Ok(Self {
            serial: serial.to_owned(),
            gradle_task: format!("{prefix}:assemble{}", variant.capitalized()),
            module_dir: module.dir.clone(),
            variant: variant.name.clone(),
            apk_dir: PathBuf::from(variant.apk_dir()),
            package: package.to_owned(),
            activity: module.launcher.as_ref().map(|l| l.class_name.clone()),
        })
    }

    /// `am start` component, shortened like `pkg/.Main` when possible.
    pub fn component(&self) -> Option<String> {
        let activity = self.activity.as_deref()?;
        Some(match activity.strip_prefix(&self.package) {
            Some(rest) if rest.starts_with('.') => format!("{}/{rest}", self.package),
            _ => format!("{}/{activity}", self.package),
        })
    }
}

/// APKs in `dir` (a variant's output directory), read from AGP's
/// `output-metadata.json` and falling back to any `*.apk` there.
pub fn find_apks(dir: &Path) -> Vec<PathBuf> {
    let from_metadata = std::fs::read_to_string(dir.join("output-metadata.json"))
        .ok()
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
        .and_then(|json| {
            json.get("elements")?.as_array().map(|elements| {
                elements
                    .iter()
                    .filter_map(|element| element.get("outputFile")?.as_str())
                    .map(|file| dir.join(file))
                    .filter(|path| path.is_file())
                    .collect::<Vec<_>>()
            })
        })
        .unwrap_or_default();
    if !from_metadata.is_empty() {
        return from_metadata;
    }
    let mut apks: Vec<PathBuf> = std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .flatten()
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|e| e == "apk"))
                .collect()
        })
        .unwrap_or_default();
    apks.sort();
    apks
}

/// A failed `adb install`, with the package manager's code and a hint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallFailure {
    pub code: String,
    pub message: String,
    pub hint: Option<String>,
}

impl std::fmt::Display for InstallFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Installation failed: {}", self.message)?;
        if let Some(hint) = &self.hint {
            write!(f, ". {hint}")?;
        }
        Ok(())
    }
}

/// Interprets `adb install` output (`Success`, or `Failure [CODE: detail]` /
/// `adb: failed to install …: Failure [CODE]`).
pub fn parse_install_output(output: &str) -> Result<(), InstallFailure> {
    if output.lines().any(|line| line.trim() == "Success") {
        return Ok(());
    }
    let line = output
        .lines()
        .map(str::trim)
        .find(|line| line.contains("Failure") || line.contains("INSTALL_"))
        .or_else(|| output.lines().map(str::trim).rfind(|l| !l.is_empty()))
        .unwrap_or("adb install produced no output");
    let inner = line
        .split_once('[')
        .and_then(|(_, rest)| rest.rsplit_once(']'))
        .map(|(inner, _)| inner)
        .unwrap_or(line);
    let code = inner
        .split([':', ' '])
        .find(|token| token.starts_with("INSTALL_") || token.starts_with("DELETE_"))
        .unwrap_or("UNKNOWN")
        .to_owned();
    let hint = match code.as_str() {
        "INSTALL_FAILED_UPDATE_INCOMPATIBLE" => Some(
            "The installed app is signed with a different key; uninstall it from the device first",
        ),
        "INSTALL_FAILED_OLDER_SDK" => {
            Some("The device's Android version is older than the app's minSdk")
        }
        "INSTALL_FAILED_INSUFFICIENT_STORAGE" => Some("Free up space on the device"),
        "INSTALL_FAILED_VERSION_DOWNGRADE" => {
            Some("A newer version is installed; uninstall it or raise versionCode")
        }
        "INSTALL_FAILED_NO_MATCHING_ABIS" => {
            Some("The APK's native libraries do not support the device's CPU ABI")
        }
        "INSTALL_FAILED_USER_RESTRICTED" => {
            Some("Allow installs over USB in the device's developer options")
        }
        _ => None,
    }
    .map(str::to_owned);
    Err(InstallFailure {
        code,
        message: inner.trim().to_owned(),
        hint,
    })
}

/// Interprets `am start` output; it exits 0 even when the activity is missing.
pub fn parse_am_start(output: &str) -> Result<(), String> {
    let error = output.lines().map(str::trim).find(|line| {
        line.starts_with("Error") || line.starts_with("Exception") || line.contains("Exception:")
    });
    match error {
        Some(line) => Err(line.to_owned()),
        None => Ok(()),
    }
}

/// Installs `apk` on `serial` (replacing an existing install, allowing test APKs).
pub fn install(adb: &Adb, serial: &str, apk: &Path) -> Result<String, String> {
    let apk = apk.to_string_lossy();
    let output = adb
        .run(&["-s", serial, "install", "-r", "-t", &apk])
        .or_else(|error| match error {
            // `adb install` exits non-zero on Failure; keep its message for parsing.
            crate::adb::AdbError::Failed { message, .. } => Ok(message),
            other => Err(other),
        })
        .map_err(|e| e.to_string())?;
    parse_install_output(&output).map_err(|failure| failure.to_string())?;
    Ok(output)
}

/// Starts `component` (`pkg/.Activity`) as the launcher would.
pub fn launch(adb: &Adb, serial: &str, component: &str) -> Result<String, String> {
    let output = adb
        .run(&[
            "-s",
            serial,
            "shell",
            "am",
            "start",
            "-n",
            component,
            "-a",
            "android.intent.action.MAIN",
            "-c",
            "android.intent.category.LAUNCHER",
        ])
        .map_err(|e| e.to_string())?;
    parse_am_start(&output)?;
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LauncherActivity, ModuleKind};

    fn module() -> AndroidModule {
        AndroidModule {
            gradle_path: ":app".into(),
            dir: PathBuf::from("app"),
            kind: ModuleKind::Application,
            build_file: PathBuf::from("app/build.gradle.kts"),
            namespace: Some("com.example.app".into()),
            application_id: Some("com.example.app.dev".into()),
            compile_sdk: None,
            min_sdk: None,
            target_sdk: None,
            manifest: None,
            variants: crate::variants::VariantModel::parse(
                "android { productFlavors { create(\"free\") {} } }",
            ),
            launcher: Some(LauncherActivity {
                class_name: "com.example.app.MainActivity".into(),
                declared_name: ".MainActivity".into(),
            }),
        }
    }

    #[test]
    fn plans_debug_deploys() {
        let plan = DeployPlan::for_module(&module(), "emulator-5554", None).unwrap();
        assert_eq!(plan.gradle_task, ":app:assembleFreeDebug");
        assert_eq!(plan.variant, "freeDebug");
        assert_eq!(plan.apk_dir, PathBuf::from("build/outputs/apk/free/debug"));
        assert_eq!(plan.package, "com.example.app.dev");
        // applicationId differs from the namespace: the class keeps its full name.
        assert_eq!(
            plan.component().as_deref(),
            Some("com.example.app.dev/com.example.app.MainActivity")
        );
        let mut same = module();
        same.application_id = None;
        let plan = DeployPlan::for_module(&same, "s", Some("freeRelease")).unwrap();
        assert_eq!(plan.gradle_task, ":app:assembleFreeRelease");
        assert!(
            DeployPlan::for_module(&same, "s", Some("paidDebug"))
                .unwrap_err()
                .contains("no build variant")
        );
        assert_eq!(
            plan.component().as_deref(),
            Some("com.example.app/.MainActivity")
        );
        same.namespace = None;
        assert!(
            DeployPlan::for_module(&same, "s", None)
                .unwrap_err()
                .contains("applicationId")
        );
    }

    #[test]
    fn install_output_is_classified() {
        assert!(parse_install_output("Performing Streamed Install\nSuccess\n").is_ok());
        let failure = parse_install_output(
            "Performing Streamed Install\nadb: failed to install app-debug.apk: Failure [INSTALL_FAILED_UPDATE_INCOMPATIBLE: Existing package com.example.app signatures do not match newer version; ignoring!]\n",
        )
        .unwrap_err();
        assert_eq!(failure.code, "INSTALL_FAILED_UPDATE_INCOMPATIBLE");
        assert!(failure.to_string().contains("uninstall it from the device"));
        let failure = parse_install_output("Failure [INSTALL_FAILED_OLDER_SDK]").unwrap_err();
        assert!(failure.hint.unwrap().contains("minSdk"));
        let failure = parse_install_output("adb: device offline").unwrap_err();
        assert_eq!(failure.code, "UNKNOWN");
        assert_eq!(failure.message, "adb: device offline");
        assert!(parse_install_output("").is_err());
    }

    #[test]
    fn am_start_errors_are_detected() {
        assert!(parse_am_start("Starting: Intent { cmp=com.example.app/.MainActivity }\n").is_ok());
        let error = parse_am_start(
            "Starting: Intent { cmp=com.example.app/.Missing }\nError type 3\nError: Activity class {com.example.app/com.example.app.Missing} does not exist.\n",
        )
        .unwrap_err();
        assert_eq!(error, "Error type 3");
    }

    #[test]
    fn finds_apks_from_metadata_or_glob() {
        let dir = crate::adb::tests::temp("apks");
        let out = dir.join("app/build/outputs/apk/debug");
        std::fs::create_dir_all(&out).unwrap();
        assert!(find_apks(&out).is_empty());
        std::fs::write(out.join("other.apk"), "").unwrap();
        assert_eq!(find_apks(&out), [out.join("other.apk")]);
        std::fs::write(out.join("app-debug.apk"), "").unwrap();
        std::fs::write(
            out.join("output-metadata.json"),
            r#"{"version":3,"artifactType":{"type":"APK","kind":"Directory"},"applicationId":"com.example.app","variantName":"debug","elements":[{"type":"SINGLE","filters":[],"outputFile":"app-debug.apk"}]}"#,
        )
        .unwrap();
        assert_eq!(find_apks(&out), [out.join("app-debug.apk")]);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
