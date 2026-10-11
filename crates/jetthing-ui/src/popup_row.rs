//! The row of a popup list whose content the caller paints (a highlighted match, a code
//! preview): the kit's row pill and states, the arrow cursor, and an accessible name.
//! Rows whose content fits `ListRow` use that instead.

use archipel_kit::Theme;
use archipel_kit::parts::{State, row_pill};
use archipel_kit::tokens::size;
use eframe::egui::{self, Rect, Response, Sense, WidgetInfo, WidgetType, vec2};

/// A full-width row called `name` for screen readers. A disabled row (an unavailable
/// action) is dimmed by its caller, ignores clicks and is skipped by Tab.
pub fn row(ui: &mut egui::Ui, selected: bool, enabled: bool, name: &str) -> (Rect, Response) {
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, response) =
        ui.allocate_exact_size(vec2(ui.available_width(), size::ROW_HEIGHT), sense);
    archipel_kit::cursor::on_hover(&response, egui::CursorIcon::Default);
    response
        .widget_info(|| WidgetInfo::selected(WidgetType::SelectableLabel, enabled, selected, name));
    if ui.is_rect_visible(rect) {
        let colors = Theme::colors(ui.ctx());
        let state = State::of(&response).selected(selected);
        row_pill(ui.painter(), rect, state.row_fill(&colors));
    }
    (rect, response)
}
