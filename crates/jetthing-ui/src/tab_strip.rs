//! One editor tab, built from kit parts.
//!
//! The kit's `TabBar` draws a whole strip and answers only "selected" and "closed". Editor
//! tabs also need a response per tab (drag, double-click, context menu, tooltip), distinct
//! hover/pressed/selected states, a pin and a modified marker, so each tab is assembled from
//! the kit's parts instead (`State` for the states, `row_pill` and `focus_ring` for the
//! shapes, `IconButton` for the pin and close buttons, `DragSource` for moves). Colors are
//! kit tones. See "Kit gaps" in `docs/ui-migration.md`.

use archipel_kit::behaviour::{Behaviour, DragSource};
use archipel_kit::components::{IconButton, TooltipExt};
use archipel_kit::icons::{self, Glyph, Icon};
use archipel_kit::parts::{State, focus_ring, row_pill};
use archipel_kit::tokens::{radius, size};
use archipel_kit::{Theme, Tone};
use eframe::egui::{self, Rect, Response, Sense, Ui, pos2, vec2};

/// Height of a tab inside the 38 pt tab bar.
pub const TAB_HEIGHT: f32 = 28.0;
const PAD_LEFT: f32 = 10.0;
const GAP: f32 = 6.0;
const PAD_RIGHT: f32 = 6.0;

/// The payload of a tab being dragged: the document index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TabDrag(pub usize);

/// What to draw for one tab.
pub struct Tab<'a> {
    pub id: egui::Id,
    pub rect: Rect,
    pub name: &'a str,
    pub icon: Icon,
    pub tooltip: &'a str,
    pub active: bool,
    /// Italic title until the tab becomes permanent.
    pub preview: bool,
    pub modified: bool,
    pub pinned: bool,
    /// `Some(index)` makes the tab a drag source carrying [`TabDrag`].
    pub drag: Option<usize>,
    /// This tab is the one being dragged: drawn faded.
    pub dragging: bool,
}

/// What the user did with a tab this frame.
pub struct TabOut {
    /// The tab itself (drag, double click, context menu attach here).
    pub response: Response,
    pub clicked: bool,
    pub closed: bool,
    pub unpinned: bool,
    pub double_clicked: bool,
    /// Where the title sits, for decorations such as the error squiggle.
    pub text: Rect,
}

/// Geometry of a tab: file icon, title, the pin/close button and the modified badge shown
/// while that button is visible. The badge sits on the icon's top-right corner so it never
/// covers the title.
pub struct Layout {
    pub icon: Rect,
    pub text: Rect,
    pub button: Rect,
    pub modified_dot: egui::Pos2,
}

impl Layout {
    pub fn new(rect: Rect, text_size: egui::Vec2) -> Self {
        let icon = Rect::from_min_size(
            rect.left_center() + vec2(PAD_LEFT, -size::ICON / 2.0),
            vec2(size::ICON, size::ICON),
        );
        let text = Rect::from_min_size(
            pos2(icon.right() + GAP, rect.center().y - text_size.y / 2.0),
            text_size,
        );
        let button = Rect::from_center_size(
            pos2(
                rect.right() - PAD_RIGHT - size::ICON_BUTTON_SMALL / 2.0,
                rect.center().y,
            ),
            vec2(size::ICON_BUTTON_SMALL, size::ICON_BUTTON_SMALL),
        );
        Self {
            icon,
            text,
            button,
            modified_dot: icon.right_top() + vec2(-1.0, 2.0),
        }
    }
}

/// Width of a tab titled `name`.
pub fn width(ui: &Ui, name: &str, preview: bool) -> f32 {
    PAD_LEFT
        + size::ICON
        + GAP
        + title(ui, name, preview, egui::Color32::WHITE).size().x
        + GAP
        + size::ICON_BUTTON_SMALL
        + PAD_RIGHT
}

fn title(ui: &Ui, name: &str, preview: bool, color: egui::Color32) -> std::sync::Arc<egui::Galley> {
    let mut job = egui::text::LayoutJob::default();
    job.append(
        name,
        0.0,
        egui::TextFormat {
            font_id: archipel_kit::tokens::typography::body(archipel_kit::tokens::typography::BODY),
            color,
            italics: preview,
            ..Default::default()
        },
    );
    ui.painter().layout_job(job)
}

/// Draws one tab and reports what happened.
pub fn show(ui: &mut Ui, tab: &Tab<'_>) -> TabOut {
    let colors = Theme::colors(ui.ctx());
    let mut response = ui.interact(tab.rect, tab.id, Sense::click());
    if let Some(index) = tab.drag {
        response = DragSource(TabDrag(index)).attach(ui, response);
    }
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::SelectableLabel,
            true,
            tab.active,
            tab.name,
        )
    });
    // The pin and close buttons sit on the tab, so the tab is "hovered" while the pointer
    // is anywhere over it.
    let mut state = State::of(&response).selected(tab.active);
    state.hovered |= ui.rect_contains_pointer(tab.rect);
    let show_button = tab.pinned || state.hovered || tab.active;

    if tab.dragging {
        ui.set_opacity(0.45);
    }
    let fill = match (tab.active, state.pressed, state.hovered) {
        (_, true, _) => colors.pressed,
        (true, ..) => colors.tab_selected,
        (false, _, true) => colors.hover,
        _ => egui::Color32::TRANSPARENT,
    };
    let painter = ui.painter().clone();
    row_pill(&painter, tab.rect, fill);
    if tab.active {
        let underline = Rect::from_min_max(
            pos2(tab.rect.left(), tab.rect.bottom() - 3.0),
            tab.rect.right_bottom(),
        );
        row_pill(&painter, underline, colors.tab_selected_border);
    }
    if response.has_focus() {
        focus_ring(&painter, tab.rect, radius::CONTROL, &colors);
    }

    let tone = if tab.active {
        Tone::Strong
    } else {
        Tone::Secondary
    };
    let galley = title(ui, tab.name, tab.preview, state.text(tone, &colors));
    let layout = Layout::new(tab.rect, galley.size());
    icons::paint(
        &painter,
        layout.icon,
        tab.icon,
        state.icon(Tone::Icon, &colors),
    );
    painter.galley(layout.text.min, galley, state.text(tone, &colors));

    let mut out = TabOut {
        clicked: response.clicked(),
        closed: response.middle_clicked(),
        unpinned: false,
        double_clicked: response.double_clicked(),
        text: layout.text,
        response,
    };
    if show_button {
        let button = if tab.pinned {
            IconButton::new(Glyph::Pin, format!("Unpin {}", tab.name))
        } else {
            IconButton::new(Glyph::Close, format!("Close {}", tab.name))
        };
        let pressed = ui.put(layout.button, button.small(true)).clicked();
        // A click on the button is not a click on the tab.
        if pressed {
            out.clicked = false;
            out.double_clicked = false;
            if tab.pinned {
                out.unpinned = true;
            } else {
                out.closed = true;
            }
        }
    }
    if tab.modified {
        if show_button {
            painter.circle_filled(layout.modified_dot, 2.5, Tone::Modified.color(&colors));
        } else {
            painter.circle_filled(layout.button.center(), 3.0, colors.text_secondary);
        }
    }
    if tab.dragging {
        ui.set_opacity(1.0);
    }
    out.response = out.response.tooltip(tab.tooltip);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modified_badge_does_not_cover_the_tab_title() {
        for text_width in [20.0, 87.0, 240.0] {
            let width = PAD_LEFT
                + size::ICON
                + GAP
                + text_width
                + GAP
                + size::ICON_BUTTON_SMALL
                + PAD_RIGHT;
            let rect = Rect::from_min_size(pos2(100.0, 5.0), vec2(width, TAB_HEIGHT));
            let layout = Layout::new(rect, vec2(text_width, 16.0));
            let badge = Rect::from_center_size(layout.modified_dot, vec2(5.0, 5.0));
            assert!(
                !badge.intersects(layout.text),
                "badge over title at {text_width}"
            );
            assert!(!badge.intersects(layout.button));
            assert!(!layout.text.intersects(layout.button));
            assert!(rect.contains_rect(badge));
            assert!(rect.contains_rect(layout.button));
        }
    }
}
