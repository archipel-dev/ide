//! Archipel IDE plugin entry point: the HTTP client as an editor tab (Tools › HTTP Client)
//! or as the standalone app (`archipel-ide --app dev.archipel.nexus-http`). `.http` and
//! `.rest` files open in the same editor and are imported into the workspace.

use crate::NexusApp;
use archipel_ide_plugin_api::{
    CommandContribution, CommandError, CommandResult, Contributions, EditorContribution,
    EditorTabRequest, MenuLocation, Notification, Plugin, PluginHost, PluginManifest, Value,
    ViewContext, ViewFrame, ViewPlacement,
};
use nexus_http_core::import::http_file;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub const VIEW: &str = "nexus.main";
pub const OPEN: &str = "nexus.open";
/// Extensions of request files the editor opens.
pub const FILE_EXTENSIONS: [&str; 2] = ["http", "rest"];

pub struct NexusHttpPlugin {
    manifest: PluginManifest,
    /// Created on first render, once the project and data directories are known.
    app: Option<NexusApp>,
    imported: HashSet<PathBuf>,
}

impl Default for NexusHttpPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl NexusHttpPlugin {
    pub fn new() -> Self {
        Self {
            manifest: PluginManifest::parse(include_str!("../archipel-plugin.toml"))
                .expect("bundled manifest is valid"),
            app: None,
            imported: HashSet::new(),
        }
    }

    /// The workspace lives in `<project>/.nexus` (so it can be committed), or in the
    /// plugin's data directory when no project is open (standalone mode).
    fn app(&mut self, host: &dyn PluginHost) -> &mut NexusApp {
        self.app.get_or_insert_with(|| {
            let dir = host
                .project_root()
                .map(Path::to_path_buf)
                .or_else(|| host.data_dir().ok())
                .unwrap_or_else(|| std::env::temp_dir().join("nexus-http"));
            NexusApp::open(&dir)
        })
    }

    fn import_file(&mut self, path: &Path, host: &mut dyn PluginHost) {
        if !self.imported.insert(path.to_path_buf()) {
            return;
        }
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let result = std::fs::read_to_string(path)
            .map_err(|e| e.to_string())
            .and_then(|text| http_file::import(&text, &name).map_err(|e| e.to_string()));
        match result {
            Ok(imported) => {
                let count = imported.workspace.requests().len();
                self.app(host).state.merge_import(imported.workspace);
                host.show_status(&format!("Imported {count} request(s) from {name}"));
            }
            Err(error) => host.notify(Notification::error(
                "Cannot open request file",
                format!("{name}: {error}"),
            )),
        }
    }
}

impl Plugin for NexusHttpPlugin {
    fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }

    fn contributions(&self) -> Contributions {
        let mut editor = EditorContribution::new(VIEW, "HTTP Client");
        for extension in FILE_EXTENSIONS {
            editor = editor.file_extension(extension);
        }
        Contributions::new()
            .command(CommandContribution::new(OPEN, "HTTP Client (nexus-http)"))
            .menu(MenuLocation::Tools, OPEN)
            .editor(editor)
    }

    fn execute(
        &mut self,
        command: &str,
        _params: Value,
        host: &mut dyn PluginHost,
    ) -> CommandResult {
        match command {
            OPEN => {
                host.open_editor_tab(EditorTabRequest::new(VIEW, "HTTP Client"));
                Ok(Value::Null)
            }
            other => Err(CommandError::Unknown(other.into())),
        }
    }

    fn render_view(
        &mut self,
        view: &ViewContext,
        frame: &mut ViewFrame<'_>,
        host: &mut dyn PluginHost,
    ) {
        let Some(ui) = archipel_ide_plugin_ui::ui(frame) else {
            return;
        };
        if view.view != VIEW {
            ui.label(format!("Unknown view {}", view.view));
            return;
        }
        if let ViewPlacement::EditorTab {
            path: Some(path), ..
        } = &view.placement
        {
            self.import_file(path, host);
        }
        self.app(host).ui(ui);
    }

    fn poll(&mut self, host: &mut dyn PluginHost) {
        if let Some(app) = &mut self.app
            && app.poll()
        {
            host.request_repaint();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use archipel_ide_plugin_api::testing::RecordingHost;

    #[test]
    fn contributes_the_open_command_menu_entry_and_request_file_editor() {
        let plugin = NexusHttpPlugin::new();
        let contributions = plugin.contributions();
        assert!(contributions.commands.iter().any(|c| c.id == OPEN));
        assert!(contributions.problems().is_empty());
        let editor = &contributions.editors[0];
        assert_eq!(editor.view, VIEW);
        assert_eq!(editor.file_extensions, ["http", "rest"]);
        assert_eq!(
            plugin.manifest().app.as_ref().map(|a| a.view.as_str()),
            Some(VIEW)
        );
    }

    #[test]
    fn open_command_opens_the_editor_tab() {
        let mut plugin = NexusHttpPlugin::new();
        let mut host = RecordingHost::default();
        plugin.execute(OPEN, Value::Null, &mut host).unwrap();
        assert_eq!(host.editor_tabs[0].view, VIEW);
        assert!(
            plugin
                .execute("nexus.nope", Value::Null, &mut host)
                .is_err()
        );
    }

    #[test]
    fn http_files_are_imported_once_into_the_workspace() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("api.http");
        std::fs::write(&file, "### Ping\nGET https://example.test/ping\n").unwrap();
        let mut host = RecordingHost {
            project: Some(dir.path().to_path_buf()),
            ..RecordingHost::default()
        };
        let mut plugin = NexusHttpPlugin::new();
        plugin.import_file(&file, &mut host);
        plugin.import_file(&file, &mut host);
        let requests = plugin.app(&host).state.workspace.requests().len();
        assert_eq!(requests, 1);
        assert_eq!(host.statuses.len(), 1);
        plugin.import_file(&dir.path().join("missing.http"), &mut host);
        assert_eq!(host.notifications.len(), 1);
    }
}
