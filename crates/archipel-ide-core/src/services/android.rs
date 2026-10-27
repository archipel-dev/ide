//! Android tooling as an application service (IntelliJ: the Android plugin's project and SDK
//! services).
//!
//! [`AndroidTooling`] owns the Android state of the IDE window: the SDK, the detected Android
//! Gradle project, devices and emulators, Logcat, the SDK manager, deploys, build variants, the
//! merged manifest, the resource manager, the device mirror and the layout editor. It is an
//! application service because the SDK manager and the Device Manager also work on the welcome
//! screen; opening a project re-detects the project and the SDK ([`AndroidTooling::project_opened`]).
//!
//! What happened is announced on [`ANDROID_TOPIC`] of the application bus: the project was
//! detected, the SDK changed, a deploy changed phase. The UI reads through the narrow accessors
//! (`devices()`, `logcat()`, `deploy()`, ...), each a guard of one part of the state; the state
//! is behind a [`Cell`], so reads nest and a write while a guard is alive on the thread panics in
//! debug builds instead of hanging.

use super::Cell;
use crate::android_commands::AndroidState;
use crate::android_deploy::{DeployPhase, DeployState};
use crate::android_devices::DeviceHub;
use crate::android_logcat::LogcatState;
use crate::android_manifest::MergedManifestView;
use crate::android_sdk_repo::SdkRepoState;
use crate::android_tools::{LayoutEditorState, MirrorState, ResourceManagerState};
use archipel_ide_android::{AndroidProject, SdkDetection};
use archipel_ide_platform::{MessageBus, Topic};
use parking_lot::{
    MappedRwLockReadGuard, MappedRwLockWriteGuard, RwLockReadGuard, RwLockWriteGuard,
};
use std::path::{Path, PathBuf};

/// Reports changes of the Android tooling (IntelliJ: the Android plugin's project and SDK
/// listeners).
pub trait AndroidListener: Send + Sync {
    /// A project was opened; `is_android` says whether it is an Android Gradle project.
    fn project_detected(&self, _is_android: bool) {}
    /// The SDK was detected again; `found` says whether an SDK is usable.
    fn sdk_changed(&self, _found: bool) {}
    /// A deploy moved to another phase.
    fn deploy_phase_changed(&self, _phase: &DeployPhase) {}
}

/// On the application's message bus.
pub const ANDROID_TOPIC: Topic<dyn AndroidListener> = Topic::new("AndroidListener");

/// The application service: `state.android_tooling()`.
pub struct AndroidTooling {
    state: Cell<AndroidState>,
    bus: MessageBus,
}

macro_rules! parts {
    ($($(#[$doc:meta])* $name:ident, $name_mut:ident: $field:ident: $ty:ty;)*) => {
        impl AndroidTooling {
            $(
                $(#[$doc])*
                pub fn $name(&self) -> MappedRwLockReadGuard<'_, $ty> {
                    self.state.read_map(|state| &state.$field)
                }

                $(#[$doc])*
                pub fn $name_mut(&self) -> MappedRwLockWriteGuard<'_, $ty> {
                    self.state.write_map(|state| &mut state.$field)
                }
            )*
        }
    };
}

parts! {
    /// Connected devices, emulators and the adb tracker.
    devices, devices_mut: devices: DeviceHub;
    /// The SDK manager's remote package list and jobs.
    sdk_repo, sdk_repo_mut: sdk_repo: SdkRepoState;
    /// Logcat sessions, filters and history.
    logcat, logcat_mut: logcat: LogcatState;
    /// Run on device.
    deploy, deploy_mut: deploy: DeployState;
    /// The merged manifest view.
    merged_manifest, merged_manifest_mut: merged_manifest: MergedManifestView;
    /// The resource manager tool window.
    resource_manager, resource_manager_mut: resource_manager: ResourceManagerState;
    /// The running device mirror.
    mirror, mirror_mut: mirror: MirrorState;
    /// The layout editor.
    layout, layout_mut: layout: LayoutEditorState;
}

impl AndroidTooling {
    pub fn new(bus: MessageBus) -> Self {
        Self {
            state: Cell::new(AndroidState::default(), "the Android tooling"),
            bus,
        }
    }

    pub(crate) fn read(&self) -> RwLockReadGuard<'_, AndroidState> {
        self.state.read()
    }

    pub(crate) fn write(&self) -> RwLockWriteGuard<'_, AndroidState> {
        self.state.write()
    }

    /// Detects the Android project and the SDK for a newly opened project at `root`, and starts
    /// the adb server for Android projects. Publishes `project_detected`, and `sdk_changed` for
    /// an Android project.
    pub fn project_opened(&self, root: &Path) {
        let (is_android, found) = {
            let mut state = self.write();
            state.project_opened(root);
            (
                state.project.is_some(),
                state.sdk.as_ref().is_some_and(|sdk| sdk.sdk.is_some()),
            )
        };
        self.bus.publish(ANDROID_TOPIC, |l| {
            l.project_detected(is_android);
            if is_android {
                l.sdk_changed(found);
            }
        });
    }

    /// Uses `path` as the SDK (the Android SDK settings page) and detects it again.
    pub fn set_sdk(&self, path: &Path, project_root: Option<&Path>) {
        let found = {
            let mut state = self.write();
            state.sdk_setting = Some(path.to_owned());
            state.detect_sdk(project_root).sdk.is_some()
        };
        self.bus.publish(ANDROID_TOPIC, |l| l.sdk_changed(found));
    }

    /// Detects the SDK again (`project_root` for `local.properties`) and announces it.
    pub fn detect_sdk(&self, project_root: Option<&Path>) -> SdkDetection {
        let detection = self.write().detect_sdk(project_root).clone();
        let found = detection.sdk.is_some();
        self.bus.publish(ANDROID_TOPIC, |l| l.sdk_changed(found));
        detection
    }

    /// Moves a deploy to `phase` and announces it (no event when the phase is the same).
    pub(crate) fn set_deploy_phase(&self, phase: DeployPhase) {
        {
            let mut state = self.write();
            if state.deploy.phase == phase {
                return;
            }
            state.deploy.phase = phase.clone();
        }
        self.bus
            .publish(ANDROID_TOPIC, |l| l.deploy_phase_changed(&phase));
    }

    /// Whether the open project is an Android Gradle project.
    pub fn is_android_project(&self) -> bool {
        self.read().project.is_some()
    }

    /// The detected Android project.
    pub fn project(&self) -> Option<AndroidProject> {
        self.read().project.clone()
    }

    /// The last SDK detection result.
    pub fn sdk(&self) -> Option<SdkDetection> {
        self.read().sdk.clone()
    }

    pub fn has_sdk_detection(&self) -> bool {
        self.read().sdk.is_some()
    }

    /// The SDK the user chose in the SDK settings page.
    pub fn sdk_setting(&self) -> Option<PathBuf> {
        self.read().sdk_setting.clone()
    }

    /// Whether the Android SDK settings page is showing.
    pub fn sdk_manager_open(&self) -> bool {
        self.read().sdk_manager_open
    }

    pub fn set_sdk_manager_open(&self, open: bool) {
        self.write().sdk_manager_open = open;
    }

    /// Whether the Project tool window shows the Android view.
    pub fn android_view(&self) -> bool {
        self.read().android_view
    }

    pub fn set_android_view(&self, on: bool) {
        self.write().android_view = on;
    }

    /// The active build variant of a module, by Gradle path.
    pub fn active_variant(&self, gradle_path: &str) -> Option<String> {
        self.read().active_variants.get(gradle_path).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use archipel_ide_platform::{Application, ComponentManager};
    use parking_lot::Mutex;
    use std::sync::Arc;

    #[derive(Default)]
    struct Recorder(Mutex<Vec<String>>);

    impl AndroidListener for Recorder {
        fn project_detected(&self, is_android: bool) {
            self.0.lock().push(format!("project {is_android}"));
        }
        fn sdk_changed(&self, found: bool) {
            self.0.lock().push(format!("sdk {found}"));
        }
        fn deploy_phase_changed(&self, phase: &DeployPhase) {
            self.0.lock().push(format!("deploy {phase:?}"));
        }
    }

    fn setup() -> (Application, Arc<AndroidTooling>, Arc<Recorder>) {
        let application = Application::new();
        application
            .register_service(archipel_ide_platform::ServiceDescriptor::new(
                |app: &Application| AndroidTooling::new(app.message_bus().clone()),
            ))
            .unwrap();
        let recorder = Arc::new(Recorder::default());
        application
            .message_bus()
            .subscribe(application.disposable(), ANDROID_TOPIC, recorder.clone())
            .unwrap();
        (application.clone(), application.service(), recorder)
    }

    #[test]
    fn a_plain_project_is_not_android_and_says_so() {
        let (_application, tooling, recorder) = setup();
        let dir = std::env::temp_dir().join(format!("ide-android-plain-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        tooling.project_opened(&dir);
        assert!(!tooling.is_android_project());
        assert!(!tooling.android_view());
        assert_eq!(*recorder.0.lock(), ["project false"]);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn deploy_phases_are_announced_once() {
        let (_application, tooling, recorder) = setup();
        tooling.set_deploy_phase(DeployPhase::Building);
        tooling.set_deploy_phase(DeployPhase::Building);
        tooling.set_deploy_phase(DeployPhase::Done("ok".into()));
        assert_eq!(
            *recorder.0.lock(),
            ["deploy Building", "deploy Done(\"ok\")"]
        );
        assert_eq!(tooling.deploy().phase, DeployPhase::Done("ok".into()));
    }

    #[test]
    fn reads_nest_and_a_listener_may_read_the_service() {
        let (_application, tooling, _) = setup();
        tooling.set_sdk_manager_open(true);
        let devices = tooling.devices();
        // A second read, while the first guard is alive, as the UI does while painting.
        assert!(tooling.sdk_manager_open());
        assert!(!devices.is_tracking());
    }

    #[test]
    fn narrow_writes_reach_the_shared_state() {
        let (_application, tooling, _) = setup();
        tooling.logcat_mut().paused = true;
        tooling.set_android_view(true);
        assert!(tooling.logcat().paused);
        assert!(tooling.android_view());
    }
}
