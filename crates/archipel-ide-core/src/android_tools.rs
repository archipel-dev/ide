//! Resource Manager (`android.resourceManager.*`) and Running Devices
//! (`android.mirror.*`) tool window state and commands.

use crate::{AndroidState, IdeState, spec, string_param};
use archipel_ide_android::mirror::{MirrorEvent, ScreenMirror};
use archipel_ide_android::resource_manager::{ResourceItem, ResourceTab, catalog};
use archipel_ide_command::{Command, CommandError, CommandRegistry};
use serde_json::{Value, json};
use std::path::PathBuf;
use std::sync::Arc;

/// Resource Manager selection and the catalog built when it was last refreshed.
#[derive(Default)]
pub struct ResourceManagerState {
    /// Module directory chosen in the module combo; `None` is the first module.
    pub module: Option<PathBuf>,
    pub tab: ResourceTab,
    pub query: String,
    pub items: Vec<ResourceItem>,
}

impl ResourceManagerState {
    /// Items of the current module and tab matching the search query.
    pub fn visible<'a>(&'a self, modules: &[PathBuf]) -> Vec<&'a ResourceItem> {
        let module = self.module.as_ref().or(modules.first());
        let query = self.query.to_lowercase();
        self.items
            .iter()
            .filter(|item| {
                item.tab == self.tab
                    && Some(&item.module) == module
                    && item.name.to_lowercase().contains(&query)
            })
            .collect()
    }
}

/// Layout editor presentation of a layout XML file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LayoutMode {
    #[default]
    Code,
    Split,
    Design,
}

impl LayoutMode {
    pub const ALL: [LayoutMode; 3] = [LayoutMode::Code, LayoutMode::Split, LayoutMode::Design];

    pub fn id(self) -> &'static str {
        match self {
            LayoutMode::Code => "code",
            LayoutMode::Split => "split",
            LayoutMode::Design => "design",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            LayoutMode::Code => "Code",
            LayoutMode::Split => "Split",
            LayoutMode::Design => "Design",
        }
    }
}

#[derive(Debug, Default)]
pub struct LayoutEditorState {
    pub mode: LayoutMode,
    /// Pre-order index of the selected view in the Component Tree.
    pub selected: usize,
}

/// Screen mirroring of the selected device.
#[derive(Default)]
pub struct MirrorState {
    session: Option<ScreenMirror>,
    /// Newest PNG capture and a counter that changes with every frame.
    pub frame: Option<Arc<Vec<u8>>>,
    pub frame_id: u64,
    pub error: Option<String>,
    /// Serial of the mirrored device.
    pub serial: Option<String>,
}

impl MirrorState {
    pub fn is_running(&self) -> bool {
        self.session.is_some()
    }

    pub(crate) fn poll(&mut self) -> bool {
        let events = self
            .session
            .as_ref()
            .map(ScreenMirror::poll)
            .unwrap_or_default();
        let mut changed = false;
        for event in events {
            changed = true;
            match event {
                MirrorEvent::Frame(png) => {
                    self.frame = Some(Arc::new(png));
                    self.frame_id += 1;
                    self.error = None;
                }
                MirrorEvent::Ended(message) => {
                    self.session = None;
                    self.error = Some(message);
                }
            }
        }
        changed
    }
}

impl IdeState {
    /// The view tree of the active editor file when it is an Android layout.
    pub fn active_layout(
        &self,
    ) -> Option<Result<archipel_ide_android::layout::LayoutNode, String>> {
        self.android.project.as_ref()?;
        let document = self.documents.get(self.active_document?)?;
        if !archipel_ide_android::layout::is_layout_file(&document.path) {
            return None;
        }
        Some(archipel_ide_android::layout::parse_layout(
            &document.text.text(),
        ))
    }
}

impl AndroidState {
    fn start_mirror(&mut self) -> Result<String, String> {
        let _ = self.ensure_device_tracking();
        let serial = self
            .devices
            .selected_device()
            .filter(|d| d.is_online())
            .map(|d| d.serial.clone())
            .ok_or("No connected device to mirror. Connect a device or start an emulator.")?;
        let adb = self.adb()?;
        self.mirror.session = Some(ScreenMirror::start(adb, &serial));
        self.mirror.serial = Some(serial.clone());
        self.mirror.error = None;
        Ok(serial)
    }

    fn stop_mirror(&mut self) {
        self.mirror.session = None;
    }
}

impl IdeState {
    /// Module directories of the open Android project, in declaration order.
    pub fn android_module_dirs(&self) -> Vec<PathBuf> {
        self.android
            .project
            .as_ref()
            .map(|p| p.modules.iter().map(|m| m.dir.clone()).collect())
            .unwrap_or_default()
    }

    /// Rebuilds the Resource Manager catalog from the project files.
    pub fn refresh_resource_manager(&mut self) {
        let Some(project) = self.project.as_ref() else {
            return;
        };
        let files: Vec<PathBuf> = self
            .files
            .iter()
            .filter(|f| !f.is_dir)
            .map(|f| f.relative_path.clone())
            .collect();
        let modules = self.android_module_dirs();
        self.android.resource_manager.items = catalog(project.root(), &files, &modules);
    }

    pub(crate) fn resource_manager_opened(&mut self) {
        self.refresh_resource_manager();
    }

    pub(crate) fn running_devices_opened(&mut self) {
        // Shares the right anchor with plugin tool windows.
        self.plugin_host.tool_window = None;
        if !self.android.mirror.is_running() {
            let _ = self.android.start_mirror();
        }
    }
}

pub(crate) fn register(registry: &mut CommandRegistry<IdeState>) {
    registry
        .register(Command::new(
            spec(
                "android.resourceManager.select",
                "Resource Manager Selection",
                "Android",
                None,
                r#"{"module":"string?","tab":"drawable|color|layout|mipmap|string?","query":"string?"}"#,
            ),
            |state: &IdeState| state.android.project.is_some(),
            |state: &mut IdeState, params| {
                let modules = state.android_module_dirs();
                let manager = &mut state.android.resource_manager;
                if let Some(module) = params.get("module").and_then(Value::as_str) {
                    let module = PathBuf::from(module);
                    if !modules.contains(&module) {
                        return Err(CommandError::InvalidParams(format!(
                            "no module directory {}",
                            module.display()
                        )));
                    }
                    manager.module = Some(module);
                }
                if let Some(tab) = params.get("tab").and_then(Value::as_str) {
                    manager.tab = ResourceTab::parse(tab).ok_or_else(|| {
                        CommandError::InvalidParams(format!("unknown resource tab `{tab}`"))
                    })?;
                }
                if let Some(query) = params.get("query").and_then(Value::as_str) {
                    manager.query = query.to_owned();
                }
                let count = manager.visible(&modules).len();
                Ok(json!({"count": count}))
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "android.resourceManager.refresh",
                "Refresh Resource Manager",
                "Android",
                None,
                "{}",
            ),
            |state: &IdeState| state.android.project.is_some(),
            |state: &mut IdeState, _| {
                state.refresh_resource_manager();
                Ok(json!({"count": state.android.resource_manager.items.len()}))
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "android.resourceManager.open",
                "Open Resource Declaration",
                "Android",
                None,
                r#"{"path":"string","line":"number?"}"#,
            ),
            |state: &IdeState| state.android.project.is_some(),
            |state: &mut IdeState, params| {
                let path = PathBuf::from(string_param(&params, "path")?);
                let line = params.get("line").and_then(Value::as_u64).unwrap_or(1) as usize;
                let def = archipel_ide_android::resources::ResourceDef {
                    kind: String::new(),
                    name: String::new(),
                    path,
                    line: line.max(1),
                    qualifier: String::new(),
                };
                state.open_resource(&def).map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "android.layout.mode",
                "Layout Editor Mode",
                "Android",
                None,
                r#"{"mode":"code|split|design"}"#,
            ),
            |state: &IdeState| state.active_layout().is_some(),
            |state: &mut IdeState, params| {
                let mode = string_param(&params, "mode")?;
                let mode = LayoutMode::ALL
                    .into_iter()
                    .find(|m| m.id() == mode)
                    .ok_or_else(|| {
                        CommandError::InvalidParams(format!("unknown layout mode `{mode}`"))
                    })?;
                state.android.layout.mode = mode;
                Ok(json!({"mode": mode.id()}))
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "android.layout.select",
                "Select Layout Component",
                "Android",
                None,
                r#"{"node":"number"}"#,
            ),
            |state: &IdeState| state.active_layout().is_some(),
            |state: &mut IdeState, params| {
                let node =
                    params.get("node").and_then(Value::as_u64).ok_or_else(|| {
                        CommandError::InvalidParams("`node` must be a number".into())
                    })? as usize;
                let tree = state
                    .active_layout()
                    .and_then(Result::ok)
                    .ok_or_else(|| CommandError::Failed("the layout does not parse".into()))?;
                let selected = tree
                    .nth(node)
                    .ok_or_else(|| CommandError::InvalidParams(format!("no component {node}")))?;
                let tag = selected.tag.clone();
                state.android.layout.selected = node;
                Ok(json!({"tag": tag}))
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "android.mirror.start",
                "Start Screen Mirroring",
                "Android",
                None,
                "{}",
            ),
            |state: &IdeState| state.android.project.is_some(),
            |state: &mut IdeState, _| {
                let serial = state.android.start_mirror().map_err(|error| {
                    state.android.mirror.error = Some(error.clone());
                    CommandError::Failed(error)
                })?;
                Ok(json!({"serial": serial}))
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "android.mirror.stop",
                "Stop Screen Mirroring",
                "Android",
                None,
                "{}",
            ),
            |state: &IdeState| state.android.mirror.is_running(),
            |state: &mut IdeState, _| {
                state.android.stop_mirror();
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
}

#[cfg(test)]
mod tests {
    use crate::{IdeState, ToolWindow, built_in_commands};
    use serde_json::json;
    use std::path::Path;

    fn write(root: &Path, rel: &str, text: &str) {
        let path = root.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    fn android_state(name: &str) -> (IdeState, std::path::PathBuf) {
        let root =
            std::env::temp_dir().join(format!("archipel-ide-core-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        write(&root, "settings.gradle.kts", "include(\":app\")\n");
        write(
            &root,
            "app/build.gradle.kts",
            "plugins { id(\"com.android.application\") }\nandroid { namespace = \"a.b\" }\n",
        );
        write(
            &root,
            "app/src/main/res/values/strings.xml",
            "<resources>\n<string name=\"app_name\">My App</string>\n<string name=\"other\">x</string>\n</resources>\n",
        );
        write(
            &root,
            "app/src/main/res/layout/main.xml",
            "<LinearLayout/>\n",
        );
        let commands = built_in_commands();
        let mut state = IdeState::default();
        commands
            .execute("project.open", &mut state, json!({"path": root}))
            .unwrap();
        (state, root)
    }

    #[test]
    fn resource_manager_tabs_search_and_open() {
        let (mut state, root) = android_state("resmgr");
        let commands = built_in_commands();
        commands
            .execute("toolwindow.activate.resourceManager", &mut state, json!({}))
            .unwrap();
        assert!(state.tool_windows.is_visible(ToolWindow::ResourceManager));
        let count = |state: &mut IdeState, params| {
            commands
                .execute("android.resourceManager.select", state, params)
                .unwrap()["count"]
                .as_u64()
                .unwrap()
        };
        assert_eq!(count(&mut state, json!({"tab": "string"})), 2);
        assert_eq!(count(&mut state, json!({"query": "app"})), 1);
        assert_eq!(count(&mut state, json!({"query": "", "tab": "layout"})), 1);
        assert!(
            commands
                .execute(
                    "android.resourceManager.select",
                    &mut state,
                    json!({"tab": "nope"})
                )
                .is_err()
        );
        commands
            .execute(
                "android.resourceManager.open",
                &mut state,
                json!({"path": "app/src/main/res/layout/main.xml"}),
            )
            .unwrap();
        assert!(state.active_document.is_some());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn layout_modes_and_selection() {
        let (mut state, root) = android_state("layout");
        let commands = built_in_commands();
        assert!(
            commands
                .execute("android.layout.mode", &mut state, json!({"mode": "design"}))
                .is_err(),
            "needs an open layout file"
        );
        write(
            &root,
            "app/src/main/res/layout/main.xml",
            "<LinearLayout xmlns:android=\"http://schemas.android.com/apk/res/android\">\n<Button android:id=\"@+id/ok\"/>\n</LinearLayout>\n",
        );
        commands
            .execute(
                "editor.openFile",
                &mut state,
                json!({"path": "app/src/main/res/layout/main.xml"}),
            )
            .unwrap();
        commands
            .execute("android.layout.mode", &mut state, json!({"mode": "split"}))
            .unwrap();
        assert_eq!(state.android.layout.mode, super::LayoutMode::Split);
        assert!(
            commands
                .execute("android.layout.mode", &mut state, json!({"mode": "x"}))
                .is_err()
        );
        let selected = commands
            .execute("android.layout.select", &mut state, json!({"node": 1}))
            .unwrap();
        assert_eq!(selected["tag"], "Button");
        assert!(
            commands
                .execute("android.layout.select", &mut state, json!({"node": 9}))
                .is_err()
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn running_devices_opens_with_no_device() {
        let (mut state, root) = android_state("mirror");
        let commands = built_in_commands();
        commands
            .execute("toolwindow.activate.runningDevices", &mut state, json!({}))
            .unwrap();
        assert!(state.tool_windows.is_visible(ToolWindow::RunningDevices));
        assert!(!state.android.mirror.is_running());
        let error = commands
            .execute("android.mirror.start", &mut state, json!({}))
            .unwrap_err();
        assert!(error.to_string().contains("device") || error.to_string().contains("adb"));
        assert!(state.android.mirror.error.is_some());
        let _ = std::fs::remove_dir_all(&root);
    }
}
