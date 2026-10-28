//! The whole chrome for a state, built in one call (TUI-CMP-022), and the paths that reach an
//! action through visible UI (TUI-GUI-003).

use crate::context_menu::{Target, context_menu};
use crate::cx::ViewCx;
use crate::header::tool_window_header;
use crate::main_menu::main_menu;
use crate::status::status_bar;
use crate::stripes::{left_stripe, right_stripe};
use crate::tabs::editor_tabs;
use crate::toolbar::{ids, main_toolbar};
use archipel_ide_core::{Anchor, ToolWindow};
use archipel_kit_model::{MenuBar, MenuEntry, PanelHeader, StatusBar, TabBar, ToolStripe, Toolbar};

/// What the chrome shows for a state.
#[derive(Debug, Clone, PartialEq)]
pub struct Chrome {
    /// Row 0 of the terminal.
    pub toolbar: Toolbar,
    /// The main menu behind the hamburger.
    pub menu: MenuBar,
    /// The left stripe.
    pub left: ToolStripe,
    /// The right stripe.
    pub right: ToolStripe,
    /// The header of each visible tool window.
    pub headers: Vec<(ToolWindow, PanelHeader)>,
    /// The editor tab strip.
    pub tabs: TabBar,
    /// Breadcrumbs and widgets.
    pub status: StatusBar,
}

impl Chrome {
    /// Builds every part of the chrome. The only entry the frontends use for it.
    pub fn build(cx: &ViewCx) -> Chrome {
        Self::build_with(cx, true)
    }

    /// Like [`Chrome::build`]; the main menu (hundreds of action updates) is built only when
    /// `menu` is true, else it is an empty bar.
    pub fn build_with(cx: &ViewCx, menu: bool) -> Chrome {
        let layout = cx.state.tool_windows();
        let headers = [Anchor::Left, Anchor::Bottom, Anchor::Right]
            .into_iter()
            .filter_map(|anchor| layout.visible(anchor))
            .map(|window| (window, tool_window_header(cx, window)))
            .collect();
        Chrome {
            toolbar: main_toolbar(cx),
            menu: if menu {
                main_menu(cx)
            } else {
                MenuBar::default()
            },
            left: left_stripe(cx),
            right: right_stripe(cx),
            headers,
            tabs: editor_tabs(cx, None, true),
            status: status_bar(cx),
        }
    }

    /// The path of visible UI that starts `action` by clicks: a menu path, a toolbar button, a
    /// stripe button, a header action, a context menu, or Search Everywhere's Actions tab as
    /// the last resort. `None` when no path exists.
    pub fn reach(&self, cx: &ViewCx, action: &str) -> Option<Vec<String>> {
        if let Some(path) = menu_path(
            &self
                .menu
                .headers
                .iter()
                .map(|h| (h.label.as_str(), h.id.as_str(), h.entries.as_slice()))
                .collect::<Vec<_>>(),
            action,
        ) {
            return Some(path);
        }
        let toolbar_action = match action {
            "Run" => Some(ids::RUN),
            "CompileDirty" => Some(ids::BUILD),
            "Debug" => Some(ids::DEBUG),
            "Stop" => Some(ids::STOP),
            "SearchEverywhere" => Some(ids::SEARCH),
            "ShowSettings" => Some(ids::SETTINGS),
            "MainMenu" => Some(ids::MAIN_MENU),
            "ChooseRunConfiguration" => Some(ids::RUN_CONFIG),
            _ => None,
        };
        if let Some(id) = toolbar_action {
            return Some(vec!["toolbar".into(), id.into()]);
        }
        for window in ToolWindow::ALL {
            if crate::icons::activation_action(window) == Some(action) {
                return Some(vec!["stripe".into(), window.id().into()]);
            }
        }
        for target in [
            Target::Editor,
            Target::Tab(0),
            Target::Tree {
                path: "src".into(),
                is_dir: false,
            },
        ] {
            let entries = context_menu(cx, &target);
            if let Some(path) = menu_path(&[("context", "context", &entries)], action) {
                return Some(path);
            }
        }
        if crate::actions::is_implemented(action) || cx.state.action_registry().contains(action) {
            return Some(vec![
                "Search Everywhere".into(),
                "Actions".into(),
                action.into(),
            ]);
        }
        None
    }
}

/// The path of labels to the item `action` in `menus` (title, id, entries).
fn menu_path(menus: &[(&str, &str, &[MenuEntry])], action: &str) -> Option<Vec<String>> {
    fn walk(entries: &[MenuEntry], action: &str, path: &mut Vec<String>) -> bool {
        for entry in entries {
            if let MenuEntry::Item(item) = entry {
                path.push(item.label.clone());
                if item.id == action && item.enabled_or_conditional() {
                    return true;
                }
                if let Some(sub) = &item.submenu
                    && walk(sub, action, path)
                {
                    return true;
                }
                path.pop();
            }
        }
        false
    }
    for (title, _, entries) in menus {
        let mut path = vec![(*title).to_owned()];
        if walk(entries, action, &mut path) {
            return Some(path);
        }
    }
    None
}

trait Reachable {
    fn enabled_or_conditional(&self) -> bool;
}

impl Reachable for archipel_kit_model::MenuItem {
    /// A disabled item is still a visible path: it is enabled when its condition holds.
    fn enabled_or_conditional(&self) -> bool {
        self.disabled_reason.is_none()
    }
}
