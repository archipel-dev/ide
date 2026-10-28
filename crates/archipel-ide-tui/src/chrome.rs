//! The chrome of the terminal: the kit controls that draw the toolbar, the main menu, the
//! stripes, the tool window headers, the tab strip and the status bar from the props of
//! `archipel-ide-view`, and the routing of their events to effects (TUI-GUI-020..110).
//!
//! This file has no menu list, label, shortcut text or enabled rule: those are the view
//! crate's. It only places the controls and forwards events.

use archipel_ide_core::{Anchor, ToolWindow};
use archipel_ide_view::context_menu::{Target, context_action, context_menu};
use archipel_ide_view::dispatch::{Effect, Event as ViewEvent, Popup};
use archipel_ide_view::{Chrome, toolbar};
use archipel_kit_model::{MenuEntry, Notation, Toolbar, ToolbarItem};
use archipel_kit_tui::bar::ToolbarControl;
use archipel_kit_tui::header::PanelHeaderControl;
use archipel_kit_tui::hit::Id;
use archipel_kit_tui::input::Event;
use archipel_kit_tui::menubar::{MenuBarControl, MenuPopup, menu_popup};
use archipel_kit_tui::popup::{Placement, PopupState};
use archipel_kit_tui::ratatui::buffer::Buffer;
use archipel_kit_tui::ratatui::layout::Rect;
use archipel_kit_tui::runtime::Runtime;
use archipel_kit_tui::statusbar::StatusItemsControl;
use archipel_kit_tui::stripe::ToolStripeControl;
use archipel_kit_tui::tabbar::TabBarControl;

/// What routing an event through the chrome produced.
#[derive(Default)]
pub struct Routed {
    /// Events for the view crate's dispatch.
    pub events: Vec<ViewEvent>,
    /// Whether a modal popup or the menu bar took the event.
    pub consumed: bool,
    /// A row chosen from a popup: its kind and id.
    pub chosen: Option<(Popup, String)>,
}

/// A popup menu that is open, and what chose from it means.
pub struct ActivePopup {
    /// The menu.
    pub popup: MenuPopup,
    /// Which surface opened it.
    pub kind: Popup,
}

/// The controls of the chrome.
pub struct ChromeUi {
    /// Row 0.
    pub toolbar: ToolbarControl,
    /// The main menu titles.
    pub menu: MenuBarControl,
    /// The left stripe.
    pub left: ToolStripeControl,
    /// The right stripe.
    pub right: ToolStripeControl,
    /// The header of the left window.
    pub header_left: PanelHeaderControl,
    /// The header of the right window.
    pub header_right: PanelHeaderControl,
    /// The header of the bottom window.
    pub header_bottom: PanelHeaderControl,
    /// The editor tab strip.
    pub tabs: TabBarControl,
    /// The status bar.
    pub status: StatusItemsControl,
    /// What the view crate built for the last frame.
    pub built: Option<Chrome>,
    /// The open popup menu.
    pub popup: Option<ActivePopup>,
}

impl Default for ChromeUi {
    fn default() -> Self {
        Self {
            toolbar: ToolbarControl::new(Id::new("tui.toolbar")),
            menu: MenuBarControl::new(Id::new("tui.menubar")),
            left: ToolStripeControl::new(Id::new("tui.stripe.left")),
            right: ToolStripeControl::new(Id::new("tui.stripe.right")),
            header_left: PanelHeaderControl::new(Id::new("tui.header.left")),
            header_right: PanelHeaderControl::new(Id::new("tui.header.right")),
            header_bottom: PanelHeaderControl::new(Id::new("tui.header.bottom")),
            tabs: TabBarControl::new(Id::new("tui.tabbar")),
            status: StatusItemsControl::new(Id::new("tui.statusbar")),
            built: None,
            popup: None,
        }
    }
}

fn header_control(ui: &mut ChromeUi, window: ToolWindow) -> &mut PanelHeaderControl {
    match window.anchor() {
        Anchor::Left => &mut ui.header_left,
        Anchor::Right => &mut ui.header_right,
        Anchor::Bottom => &mut ui.header_bottom,
    }
}

impl ChromeUi {
    /// The notation of the runtime.
    pub fn notation(rt: &Runtime) -> Notation {
        archipel_kit_tui::chrome::notation(rt)
    }

    /// Whether a popup menu or a menu of the bar is open (it takes the keys).
    pub fn modal(&self) -> bool {
        self.popup.as_ref().is_some_and(|p| p.popup.is_open()) || self.menu.is_open()
    }

    /// Whether the menu bar is expanded.
    pub fn menu_expanded(&self) -> bool {
        self.menu.is_expanded()
    }

    /// Draws the toolbar row; with the menu bar expanded the titles replace the project and
    /// VCS widgets after the hamburger.
    pub fn draw_toolbar(
        &mut self,
        rt: &mut Runtime,
        bar: &Toolbar,
        menu: &archipel_kit_model::MenuBar,
        row: Rect,
        buf: &mut Buffer,
    ) {
        if self.menu.is_expanded() {
            let first: Vec<ToolbarItem> = bar.leading.iter().take(1).cloned().collect();
            let shown = Toolbar {
                leading: first,
                ..bar.clone()
            };
            self.toolbar.draw(rt, &shown, row, buf);
            self.menu.draw(rt, menu, row, 4, buf);
        } else {
            self.toolbar.draw(rt, bar, row, buf);
        }
    }

    /// Draws the popups above everything else.
    pub fn draw_popups(&mut self, rt: &mut Runtime, buf: &mut Buffer) {
        self.menu.draw_popup(rt, buf);
        self.header_left.draw_popup(rt, buf);
        self.header_right.draw_popup(rt, buf);
        self.header_bottom.draw_popup(rt, buf);
        self.tabs.draw_popup(rt, buf);
        if let Some(active) = &mut self.popup {
            active.popup.draw(rt, buf);
        }
    }

    /// Runs the timers of the popups.
    pub fn tick(&mut self, rt: &Runtime) {
        self.menu.tick(rt);
        self.tabs.tick(rt);
        if let Some(active) = &mut self.popup {
            active.popup.tick(rt);
        }
    }

    /// The next time a timer needs a tick.
    pub fn deadline(&self) -> Option<f64> {
        [
            self.menu.deadline(),
            self.tabs.deadline(),
            self.popup.as_ref().and_then(|p| p.popup.deadline()),
        ]
        .into_iter()
        .flatten()
        .reduce(f64::min)
    }

    /// Opens a popup menu of `entries` for `kind` at `placement`.
    pub fn open_popup(
        &mut self,
        rt: &Runtime,
        kind: Popup,
        entries: Vec<MenuEntry>,
        placement: Placement,
        by_keyboard: bool,
    ) {
        if entries.is_empty() {
            return;
        }
        let mut popup = menu_popup(entries, Self::notation(rt), placement);
        popup.chain.forget_close();
        popup.open(rt, by_keyboard);
        self.popup = Some(ActivePopup { popup, kind });
    }

    /// Routes `event` through the controls; returns the events the view crate should turn
    /// into effects (and whether a modal popup took the event).
    pub fn route(
        &mut self,
        rt: &mut Runtime,
        event: &Event,
        windows: &[(ToolWindow, bool)],
    ) -> Routed {
        let mut out = Vec::new();
        let mut chosen = None;
        // The open popup menu is modal.
        if let Some(active) = &mut self.popup
            && active.popup.is_open()
        {
            match active.popup.event(rt, event) {
                PopupState::Chosen => {
                    let id = active.popup.take_choice();
                    let kind = self.popup.take().map(|p| p.kind);
                    chosen = id.zip(kind).map(|(id, kind)| (kind, id));
                }
                PopupState::Cancelled => {
                    self.popup = None;
                }
                PopupState::Open => {}
            }
            return Routed {
                events: out,
                consumed: true,
                chosen,
            };
        }
        let Some(chrome) = self.built.clone() else {
            return Routed::default();
        };
        // The main menu.
        let before_open = self.menu.is_open();
        let menu_events = self.menu.handle(rt, &chrome.menu, event);
        let menu_took = before_open || self.menu.is_open() || !menu_events.is_empty();
        for e in menu_events {
            out.push(ViewEvent::Menu(e));
        }
        if menu_took {
            return Routed {
                events: out,
                consumed: true,
                chosen,
            };
        }
        for e in self.toolbar.handle(rt, event) {
            out.push(ViewEvent::Toolbar(e));
        }
        for e in self.left.handle(rt, event) {
            out.push(ViewEvent::Stripe(e));
        }
        for e in self.right.handle(rt, event) {
            out.push(ViewEvent::Stripe(e));
        }
        for (window, _) in windows {
            let control = header_control(self, *window);
            for e in control.handle(rt, event) {
                out.push(ViewEvent::Header(*window, e));
            }
        }
        for e in self.tabs.handle(rt, &chrome.tabs, event) {
            out.push(ViewEvent::Tab(e));
        }
        for e in self.status.handle(rt, event) {
            out.push(ViewEvent::Status(e));
        }
        Routed {
            events: out,
            consumed: false,
            chosen,
        }
    }

    /// Where the widget behind a popup effect is, for anchoring.
    pub fn anchor_of(&self, popup: &Popup) -> Option<Rect> {
        match popup {
            Popup::Project => self.toolbar.rect_of(toolbar::ids::PROJECT),
            Popup::Vcs => self.toolbar.rect_of(toolbar::ids::VCS),
            Popup::RunConfig => self.toolbar.rect_of(toolbar::ids::RUN_CONFIG),
            Popup::Device => self.toolbar.rect_of(toolbar::ids::DEVICE),
            Popup::Status(id) => self.status.rect_of(id),
            Popup::ProjectOptions => self.header_left.rect_of("project.options"),
            Popup::ProjectNew => self.header_left.rect_of("project.new"),
            Popup::Stripe(id) => self.left.rect_of(id).or_else(|| self.right.rect_of(id)),
            Popup::Context(_) => None,
        }
    }
}

/// The entries of the popup `kind` for the state, from the view crate.
pub fn popup_entries(
    cx: &archipel_ide_view::ViewCx,
    chrome: Option<&Chrome>,
    kind: &Popup,
) -> Vec<MenuEntry> {
    match kind {
        Popup::Project => toolbar::project_popup(cx),
        Popup::Vcs => toolbar::vcs_popup(cx),
        Popup::RunConfig => toolbar::run_config_popup(cx),
        Popup::Status(id) => chrome
            .and_then(|c| c.status.items.iter().find(|i| i.id == *id))
            .map(|item| item.menu.clone())
            .unwrap_or_default(),
        Popup::Context(target) => context_menu(cx, target),
        Popup::ProjectOptions => archipel_ide_view::header::project_options_menu(cx),
        Popup::ProjectNew => archipel_ide_view::header::project_new_menu(cx),
        Popup::Stripe(id) => archipel_ide_view::stripes::stripe_menu(cx, id),
        Popup::Device => Vec::new(),
    }
}

/// The effects of a choice from a popup of `kind`.
pub fn choice_effects(cx: &archipel_ide_view::ViewCx, kind: &Popup, id: &str) -> Vec<Effect> {
    match kind {
        Popup::Context(target) => context_action(cx, id, target),
        _ => archipel_ide_view::dispatch::menu_row(cx, id),
    }
}

/// A target for tests and the right click on the editor.
pub fn editor_target() -> Target {
    Target::Editor
}
