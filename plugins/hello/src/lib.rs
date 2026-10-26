//! Example Archipel IDE plugin. It uses every contribution point of plugin API 1
//! and is the reference for `docs/plugins/api.md`.

use archipel_ide_plugin_api::{
    CommandContribution, CommandError, CommandResult, Contributions, EditorContribution,
    EditorTabRequest, MenuLocation, Notification, Plugin, PluginError, PluginHost, PluginManifest,
    SettingField, SettingKind, SettingsPage, StatusWidgetContribution, StatusWidgetState,
    ToolWindowContribution, Value, ViewContext, ViewFrame, json,
};
use archipel_ide_plugin_ui::egui;

pub const ID: &str = "dev.archipel.hello";

pub struct HelloPlugin {
    manifest: PluginManifest,
    greetings: u32,
    name: String,
}

impl Default for HelloPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl HelloPlugin {
    pub fn new() -> Self {
        Self {
            manifest: PluginManifest::parse(include_str!("../archipel-plugin.toml"))
                .expect("bundled manifest is valid"),
            greetings: 0,
            name: "world".into(),
        }
    }

    fn greeting(&self, host: &dyn PluginHost) -> String {
        let word = host
            .setting("greeting")
            .and_then(|value| value.as_str().map(str::to_owned))
            .unwrap_or_else(|| "Hello".into());
        format!("{word}, {}!", self.name)
    }

    fn say_hello(&mut self, host: &mut dyn PluginHost) -> String {
        self.greetings += 1;
        let message = self.greeting(host);
        host.show_status(&message);
        if host
            .setting("notify")
            .and_then(|v| v.as_bool())
            .unwrap_or(true)
        {
            host.notify(Notification::info("Hello", message.clone()));
        }
        message
    }

    fn main_view(&mut self, ui: &mut egui::Ui, host: &mut dyn PluginHost) {
        archipel_ide_plugin_ui::header(ui, "Hello", |_| {});
        ui.horizontal(|ui| {
            ui.label("Name");
            ui.text_edit_singleline(&mut self.name);
        });
        ui.add_space(6.0);
        if ui.button("Say Hello").clicked() {
            self.say_hello(host);
        }
        archipel_ide_plugin_ui::section(ui, "Statistics");
        archipel_ide_plugin_ui::property(ui, "Greetings", &self.greetings.to_string());
        archipel_ide_plugin_ui::property(
            ui,
            "Project",
            &host
                .project_root()
                .map(|root| root.display().to_string())
                .unwrap_or_else(|| "none".into()),
        );
    }
}

impl Plugin for HelloPlugin {
    fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }

    fn contributions(&self) -> Contributions {
        Contributions::new()
            .command(CommandContribution::new("hello.sayHello", "Say Hello"))
            .command(
                CommandContribution::new("hello.greet", "Greet Someone")
                    .params(r#"{"name":"string"}"#),
            )
            .command(CommandContribution::new("hello.openTab", "Open Hello Tab"))
            .menu(MenuLocation::Tools, "hello.sayHello")
            .menu(MenuLocation::Tools, "hello.openTab")
            .keybinding("hello.sayHello", "Mod+Alt+H")
            .tool_window(ToolWindowContribution::new("hello", "Hello", "hello.main").icon_text("H"))
            .editor(EditorContribution::new("hello.main", "Hello").file_extension("hello"))
            .settings_page(
                SettingsPage::new("hello", "Hello")
                    .field(SettingField::new(
                        "greeting",
                        "Greeting",
                        SettingKind::Text {
                            default: "Hello".into(),
                        },
                    ))
                    .field(
                        SettingField::new(
                            "notify",
                            "Show notification",
                            SettingKind::Bool { default: true },
                        )
                        .description("Show a balloon in addition to the status bar message."),
                    ),
            )
            .status_widget(StatusWidgetContribution::new("hello.count").command("hello.sayHello"))
    }

    fn activate(&mut self, host: &mut dyn PluginHost) -> Result<(), PluginError> {
        host.show_status("Hello plugin activated");
        Ok(())
    }

    fn execute(
        &mut self,
        command: &str,
        params: Value,
        host: &mut dyn PluginHost,
    ) -> CommandResult {
        match command {
            "hello.sayHello" => Ok(json!({"message": self.say_hello(host)})),
            "hello.greet" => {
                let name = params
                    .get("name")
                    .and_then(Value::as_str)
                    .filter(|name| !name.trim().is_empty())
                    .ok_or_else(|| {
                        CommandError::InvalidParams("expected string field `name`".into())
                    })?;
                self.name = name.trim().to_owned();
                Ok(json!({"message": self.say_hello(host)}))
            }
            "hello.openTab" => {
                host.open_editor_tab(EditorTabRequest::new("hello.main", "Hello"));
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
        match view.view.as_str() {
            "hello.main" => self.main_view(ui, host),
            other => {
                ui.label(format!("Unknown view {other}"));
            }
        }
    }

    fn status_widget(&self, id: &str) -> Option<StatusWidgetState> {
        (id == "hello.count").then(|| StatusWidgetState {
            text: format!("Hello × {}", self.greetings),
            tooltip: Some("Greetings so far — click to say hello".into()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use archipel_ide_plugin_api::testing::RecordingHost as ApiRecordingHost;
    use archipel_plugin_testing::RecordingHost;

    #[test]
    fn contributions_are_consistent() {
        let plugin = HelloPlugin::new();
        assert_eq!(plugin.manifest().id, ID);
        assert!(plugin.contributions().problems().is_empty());
        let specs = plugin.contributions().command_specs();
        let say = specs
            .iter()
            .find(|spec| spec.id == "hello.sayHello")
            .unwrap();
        assert_eq!(say.shortcut.as_deref(), Some("⌥⌘H"));
    }

    #[test]
    fn commands_use_settings_and_host_services() {
        let mut plugin = HelloPlugin::new();
        // The SDK's fake host from `archipel-plugin-testing`.
        let mut host = RecordingHost::new(ID);
        host.settings.insert("greeting".into(), json!("Hi"));
        host.settings.insert("notify".into(), json!(false));
        let result = plugin
            .execute("hello.greet", json!({"name": " Ada "}), &mut host)
            .unwrap();
        assert_eq!(result["message"], "Hi, Ada!");
        assert_eq!(host.statuses, ["Hi, Ada!"]);
        assert!(host.notifications.is_empty());
        assert!(matches!(
            plugin.execute("hello.greet", json!({}), &mut host),
            Err(CommandError::InvalidParams(_))
        ));
        plugin
            .execute("hello.openTab", Value::Null, &mut host)
            .unwrap();
        assert_eq!(host.editor_tabs[0].view, "hello.main");
        assert_eq!(
            plugin.status_widget("hello.count").unwrap().text,
            "Hello × 1"
        );
    }

    #[test]
    fn main_view_renders_in_egui() {
        let mut plugin = HelloPlugin::new();
        let mut host = ApiRecordingHost::new(ID);
        let ctx = egui::Context::default();
        let view = ViewContext {
            view: "hello.main".into(),
            placement: archipel_ide_plugin_api::ViewPlacement::App,
        };
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                plugin.render_view(&view, &mut ViewFrame::new(ui), &mut host);
            });
        });
        assert_eq!(plugin.name, "world");
    }
}
