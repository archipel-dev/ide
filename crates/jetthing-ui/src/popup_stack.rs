//! The IDE's one popup stack (interaction spec §1).
//!
//! Every transient layer (Search Everywhere, the Switcher, chooser popups, the gutter menu,
//! the rename field, quick documentation, completion, parameter info) is a [`PopupId`] on a
//! [`PopupStack`] in opening order. The workbench reconciles the stack with what is visible once per frame
//! ([`PopupStack::sync`]), so a hidden or stale layer is never on it, and <kbd>Escape</kbd>
//! closes [`PopupStack::top`] only. Areas that own other popups register them with
//! [`PopupId::Other`] and are closed by the hook in `IdeApp::close_popup`.
//!
//! The stack holds identities, not state: a popup stays owned by the code that draws it and
//! is closed through its own state, so the stack cannot get out of step with the screen.

use eframe::egui;

/// A layer on the popup stack.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PopupId {
    SearchEverywhere,
    FindInFiles,
    Switcher,
    NavigationChoices,
    GutterRun,
    Rename,
    QuickDoc,
    Completion,
    ParameterInfo,
    #[allow(dead_code)]
    /// A popup registered by another area; close it in `IdeApp::close_popup`.
    Other(&'static str),
}

/// Open layers, oldest first.
#[derive(Debug, Default)]
pub struct PopupStack {
    layers: Vec<PopupId>,
}

#[allow(dead_code)] // `push`, `remove` and `contains` are the API for areas that register later
impl PopupStack {
    /// Puts `id` on top (moving it there when it is already open).
    pub fn push(&mut self, id: PopupId) {
        self.remove(id);
        self.layers.push(id);
    }

    /// Removes and returns the top-most layer.
    pub fn pop(&mut self) -> Option<PopupId> {
        self.layers.pop()
    }

    /// The top-most layer: the one Escape closes.
    pub fn top(&self) -> Option<PopupId> {
        self.layers.last().copied()
    }

    pub fn contains(&self, id: PopupId) -> bool {
        self.layers.contains(&id)
    }

    pub fn remove(&mut self, id: PopupId) {
        self.layers.retain(|layer| *layer != id);
    }

    /// Open layers, oldest first (for the `state` snapshot).
    pub fn layers(&self) -> &[PopupId] {
        &self.layers
    }

    /// Reconciles the stack with the layers visible this frame: closed ones leave, the
    /// ones that opened since the last frame go on top in `visible` order.
    pub fn sync(&mut self, visible: &[PopupId]) {
        self.layers.retain(|layer| visible.contains(layer));
        for id in visible {
            if !self.layers.contains(id) {
                self.layers.push(*id);
            }
        }
    }
}

/// A pointer press this frame landed outside `rect` (and outside `anchor`, for popups
/// opened from a toggle control, whose own press only closes them). `opened` is the pass
/// that opened the popup: the click that opened it must not close it.
pub fn pressed_outside(
    ctx: &egui::Context,
    rect: egui::Rect,
    anchor: Option<egui::Rect>,
    opened: Option<u64>,
) -> bool {
    if opened.is_some_and(|opened| ctx.cumulative_pass_nr() <= opened) {
        return false;
    }
    ctx.input(|input| {
        input.pointer.any_pressed()
            && input.pointer.press_origin().is_some_and(|origin| {
                !rect.contains(origin) && !anchor.is_some_and(|anchor| anchor.contains(origin))
            })
    })
}

/// The window lost keyboard focus since the last frame: transient popups close. (A window
/// that never had focus, e.g. under a bare X server, does not count as a loss.)
pub fn window_focus_lost(ctx: &egui::Context) -> bool {
    let id = egui::Id::new("popup_stack_window_focus");
    let focused = ctx.input(|input| input.focused);
    let pass = ctx.cumulative_pass_nr();
    // (pass, focused in the pass before, focused now); every popup asks once per frame.
    let (recorded, previous, current) = ctx
        .data(|data| data.get_temp::<(u64, bool, bool)>(id))
        .unwrap_or((pass, focused, focused));
    let (previous, current) = if recorded == pass {
        (previous, current)
    } else {
        (current, focused)
    };
    ctx.data_mut(|data| data.insert_temp(id, (pass, previous, current)));
    previous && !current
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_target_is_the_most_recently_opened_visible_layer() {
        let mut stack = PopupStack::default();
        stack.sync(&[PopupId::Completion]);
        stack.sync(&[PopupId::Completion, PopupId::SearchEverywhere]);
        assert_eq!(stack.top(), Some(PopupId::SearchEverywhere));
        assert_eq!(stack.pop(), Some(PopupId::SearchEverywhere));
        assert_eq!(stack.top(), Some(PopupId::Completion));
    }

    #[test]
    fn a_layer_that_is_no_longer_visible_cannot_take_escape() {
        let mut stack = PopupStack::default();
        stack.sync(&[PopupId::QuickDoc, PopupId::Switcher]);
        // The quick-doc anchor scrolled away: only the Switcher is left.
        stack.sync(&[PopupId::Switcher]);
        assert_eq!(stack.layers(), &[PopupId::Switcher]);
        stack.sync(&[]);
        assert_eq!(stack.top(), None);
    }

    #[test]
    fn push_moves_an_open_layer_to_the_top_without_duplicating_it() {
        let mut stack = PopupStack::default();
        stack.push(PopupId::Other("a"));
        stack.push(PopupId::Other("b"));
        stack.push(PopupId::Other("a"));
        assert_eq!(stack.layers(), &[PopupId::Other("b"), PopupId::Other("a")]);
        assert!(stack.contains(PopupId::Other("b")));
        stack.remove(PopupId::Other("a"));
        assert_eq!(stack.top(), Some(PopupId::Other("b")));
    }

    #[test]
    fn layers_keep_their_order_across_syncs() {
        let mut stack = PopupStack::default();
        stack.sync(&[PopupId::Rename]);
        stack.sync(&[PopupId::GutterRun, PopupId::Rename]);
        assert_eq!(stack.layers(), &[PopupId::Rename, PopupId::GutterRun]);
    }
}
