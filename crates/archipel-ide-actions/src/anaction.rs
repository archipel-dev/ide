//! `AnAction`: one command a user can start, with a presentation that follows the context.
//!
//! This is the shape the menus, toolbars, context menus, Find Action and the keymap share
//! (IntelliJ: `AnAction`, `AnActionEvent`, `Presentation`). The catalog in the rest of this
//! crate names the reference actions; this module is how an implementation of one is described:
//!
//! - [`AnAction::update`] answers "is it enabled, visible, and what does it say" from a
//!   [`DataContext`], never from the feature's own state. It must be cheap: menus call it every
//!   frame while open.
//! - [`AnAction::perform`] does not run anything. It returns an [`ActionEffect`], so an action
//!   needs no `&mut` access to the IDE: a command to dispatch, or a request the host (the UI)
//!   understands. The host runs the effect with the exclusive access it has.
//! - Actions are contributions to the [`ACTIONS`] extension point, id-addressable, and a
//!   plugin or language pack can add or remove them like any other extension.

use archipel_ide_platform::{DataContext, ExtensionError, ExtensionPointName, ExtensionRegistry};
use archipel_ide_platform::{DisposableId, LoadingOrder};
use serde_json::Value;
use std::any::Any;
use std::sync::Arc;

/// Where an action is shown or started from (IntelliJ: `ActionPlaces`). Most actions look the
/// same everywhere; a few hide in a context menu or label themselves differently in a toolbar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ActionPlace {
    MainMenu,
    ContextMenu,
    Toolbar,
    ToolWindow,
    /// The Actions tab of Search Everywhere.
    FindAction,
    /// A keyboard shortcut.
    Keymap,
    Unknown,
}

/// What an action looks like right now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Presentation {
    pub enabled: bool,
    pub visible: bool,
    /// Replaces the action's catalog text when set ("Stop 'app'").
    pub text: Option<String>,
    pub description: Option<String>,
    /// An icon name the host maps to its icon set.
    pub icon: Option<String>,
}

impl Default for Presentation {
    fn default() -> Self {
        Self::enabled()
    }
}

impl Presentation {
    pub fn enabled() -> Self {
        Self {
            enabled: true,
            visible: true,
            text: None,
            description: None,
            icon: None,
        }
    }

    pub fn disabled() -> Self {
        Self {
            enabled: false,
            ..Self::enabled()
        }
    }

    /// Not shown at all (a disabled action stays visible; a hidden one does not).
    pub fn hidden() -> Self {
        Self {
            enabled: false,
            visible: false,
            ..Self::enabled()
        }
    }

    /// Enabled when `enabled` holds, disabled otherwise.
    pub fn when(enabled: bool) -> Self {
        Self {
            enabled,
            ..Self::enabled()
        }
    }

    #[must_use]
    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(text.into());
        self
    }

    #[must_use]
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    #[must_use]
    pub fn with_icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }
}

/// What an action asks the host to do.
#[derive(Clone)]
pub enum ActionEffect {
    /// Nothing (the action found nothing to do).
    None,
    /// Run a registered command with these parameters.
    Command { id: String, params: Value },
    /// A host-specific request, which the host downcasts (the UI opens a dialog this way).
    Request(Arc<dyn Any + Send + Sync>),
}

impl ActionEffect {
    pub fn command(id: impl Into<String>, params: Value) -> Self {
        Self::Command {
            id: id.into(),
            params,
        }
    }

    /// The request, if it is a `T`.
    pub fn request<T: 'static>(&self) -> Option<&T> {
        match self {
            Self::Request(request) => request.downcast_ref::<T>(),
            _ => None,
        }
    }
}

impl std::fmt::Debug for ActionEffect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => f.write_str("None"),
            Self::Command { id, params } => write!(f, "Command({id}, {params})"),
            Self::Request(_) => f.write_str("Request(..)"),
        }
    }
}

/// The situation an action is updated or performed in.
pub struct AnActionEvent<'a> {
    pub place: ActionPlace,
    data: &'a DataContext<'a>,
}

impl<'a> AnActionEvent<'a> {
    pub fn new(place: ActionPlace, data: &'a DataContext<'a>) -> Self {
        Self { place, data }
    }

    /// Where the user is: project, file, selection (see `archipel_ide_platform::data_keys`).
    pub fn data(&self) -> &DataContext<'a> {
        self.data
    }
}

/// One command a user can start.
pub trait AnAction: Send + Sync {
    /// Presentation for the current context. Called often; read the context, do no work.
    /// The default is enabled and visible.
    fn update(&self, _event: &AnActionEvent) -> Presentation {
        Presentation::enabled()
    }

    /// What to do when the user starts the action. Only called when `update` was enabled.
    fn perform(&self, event: &AnActionEvent) -> ActionEffect;
}

/// The extension point actions register on. The extension id is the action id.
pub const ACTIONS: ExtensionPointName<dyn AnAction> =
    ExtensionPointName::new("dev.archipel.action");

/// Lookup, update and registration of actions on an application's extension registry.
#[derive(Clone, Copy)]
pub struct ActionRegistry<'a> {
    extensions: &'a ExtensionRegistry,
}

impl<'a> ActionRegistry<'a> {
    pub fn new(extensions: &'a ExtensionRegistry) -> Self {
        Self { extensions }
    }

    /// Registers `action` under `id`. A duplicate id is an error, not an override. With a
    /// `parent`, the action is removed again when the parent is disposed (plugin unload).
    pub fn register(
        &self,
        id: impl Into<String>,
        action: Arc<dyn AnAction>,
        parent: Option<DisposableId>,
    ) -> Result<(), ExtensionError> {
        self.extensions
            .register(ACTIONS, id, LoadingOrder::Anywhere, action, parent)
    }

    pub fn unregister(&self, id: &str) -> bool {
        self.extensions.unregister(ACTIONS, id)
    }

    pub fn find(&self, id: &str) -> Option<Arc<dyn AnAction>> {
        self.extensions.find_by_id(ACTIONS, id)
    }

    pub fn contains(&self, id: &str) -> bool {
        self.find(id).is_some()
    }

    pub fn ids(&self) -> Vec<String> {
        self.extensions.ids(ACTIONS)
    }

    /// The presentation of `id`; `None` when no such action is registered.
    pub fn update(&self, id: &str, place: ActionPlace, data: &DataContext) -> Option<Presentation> {
        let action = self.find(id)?;
        Some(action.update(&AnActionEvent::new(place, data)))
    }

    /// The effect of starting `id`. `None` when it is not registered or not enabled in this
    /// context; the action's `perform` never runs for a disabled action.
    pub fn perform(
        &self,
        id: &str,
        place: ActionPlace,
        data: &DataContext,
    ) -> Option<ActionEffect> {
        let action = self.find(id)?;
        let event = AnActionEvent::new(place, data);
        let presentation = action.update(&event);
        (presentation.enabled && presentation.visible).then(|| action.perform(&event))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use archipel_ide_platform::{Application, data_keys};
    use serde_json::json;
    use std::path::PathBuf;

    /// Enabled while a file is open.
    struct NeedsFile;

    impl AnAction for NeedsFile {
        fn update(&self, event: &AnActionEvent) -> Presentation {
            match event.data().get(&data_keys::ACTIVE_FILE) {
                Some(file) => {
                    Presentation::enabled().with_text(format!("Close {}", file.display()))
                }
                None => Presentation::disabled(),
            }
        }

        fn perform(&self, event: &AnActionEvent) -> ActionEffect {
            let file = event.data().get(&data_keys::ACTIVE_FILE).unwrap();
            ActionEffect::command("editor.close", json!({"path": file}))
        }
    }

    struct Always;

    impl AnAction for Always {
        fn perform(&self, _: &AnActionEvent) -> ActionEffect {
            ActionEffect::Request(Arc::new(7_u8))
        }
    }

    fn registry(app: &Application) -> ActionRegistry<'_> {
        ActionRegistry::new(app.extensions())
    }

    #[test]
    fn presentation_follows_the_data_context() {
        let app = Application::new();
        let actions = registry(&app);
        actions
            .register("Close", Arc::new(NeedsFile), None)
            .unwrap();

        let empty = DataContext::new();
        let none = actions
            .update("Close", ActionPlace::MainMenu, &empty)
            .unwrap();
        assert!(!none.enabled && none.visible);

        let with_file =
            DataContext::new().with_value(&data_keys::ACTIVE_FILE, PathBuf::from("a.rs"));
        let shown = actions
            .update("Close", ActionPlace::MainMenu, &with_file)
            .unwrap();
        assert!(shown.enabled);
        assert_eq!(shown.text.as_deref(), Some("Close a.rs"));
        assert!(
            actions
                .update("Missing", ActionPlace::MainMenu, &empty)
                .is_none()
        );
    }

    #[test]
    fn perform_is_refused_while_disabled() {
        let app = Application::new();
        let actions = registry(&app);
        actions
            .register("Close", Arc::new(NeedsFile), None)
            .unwrap();
        let empty = DataContext::new();
        assert!(
            actions
                .perform("Close", ActionPlace::Keymap, &empty)
                .is_none()
        );
        let with_file =
            DataContext::new().with_value(&data_keys::ACTIVE_FILE, PathBuf::from("a.rs"));
        match actions.perform("Close", ActionPlace::Keymap, &with_file) {
            Some(ActionEffect::Command { id, params }) => {
                assert_eq!(id, "editor.close");
                assert_eq!(params["path"], "a.rs");
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn requests_carry_host_types() {
        let app = Application::new();
        let actions = registry(&app);
        actions.register("Always", Arc::new(Always), None).unwrap();
        let context = DataContext::new();
        let effect = actions
            .perform("Always", ActionPlace::Toolbar, &context)
            .unwrap();
        assert_eq!(effect.request::<u8>(), Some(&7));
        assert_eq!(effect.request::<String>(), None);
    }

    #[test]
    fn ids_are_unique_and_removable() {
        let app = Application::new();
        let actions = registry(&app);
        actions.register("A", Arc::new(Always), None).unwrap();
        assert!(actions.register("A", Arc::new(Always), None).is_err());
        assert_eq!(actions.ids(), vec!["A".to_owned()]);
        assert!(actions.unregister("A"));
        assert!(!actions.contains("A"));
    }

    #[test]
    fn presentation_builders() {
        assert!(Presentation::default().enabled);
        let hidden = Presentation::hidden();
        assert!(!hidden.enabled && !hidden.visible);
        assert!(!Presentation::when(false).enabled);
        let shown = Presentation::when(true)
            .with_icon("run")
            .with_description("d");
        assert_eq!(shown.icon.as_deref(), Some("run"));
        assert_eq!(shown.description.as_deref(), Some("d"));
    }
}
