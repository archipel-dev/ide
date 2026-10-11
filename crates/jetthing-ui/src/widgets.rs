//! Small reusable pieces of the workbench chrome.

use crate::icons::{self, Icon};
use crate::theme;
use eframe::egui::{self, Color32, Rect, Sense, vec2};

/// Rounded "island" container used for the editor and docked tool windows.
pub fn island_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(*theme::ISLAND)
        .corner_radius(theme::ISLAND_RADIUS)
        .outer_margin(egui::Margin::same(theme::ISLAND_GAP as i8))
}

/// Flat frame for chrome areas (toolbar, stripes, status bar).
pub fn chrome_frame() -> egui::Frame {
    egui::Frame::new().fill(*theme::FRAME)
}

/// Like [`chrome_frame`] without a fill: the window clears to the frame color, so
/// the toolbar glow shows through (the stripe beside the toolbar's strongest glow).
pub fn glow_through_frame() -> egui::Frame {
    egui::Frame::new()
}

pub const TOOL_WINDOW_HEADER_HEIGHT: f32 = theme::layout::TOOLBAR_HEIGHT;
pub const ROW_HEIGHT: f32 = theme::layout::ROW_HEIGHT;

/// Tool window title bar. `actions` adds right-aligned icon buttons; returns
/// `true` when the hide button was clicked.
pub fn tool_window_header(
    ui: &mut egui::Ui,
    title: &str,
    actions: impl FnOnce(&mut egui::Ui),
) -> bool {
    let mut hide = false;
    let (rect, _) = ui.allocate_exact_size(
        vec2(ui.available_width(), TOOL_WINDOW_HEADER_HEIGHT),
        Sense::hover(),
    );
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect.shrink2(vec2(theme::layout::CONTENT_INSET, 0.0)))
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    child.label(
        egui::RichText::new(title)
            .font(theme::semibold(theme::UI_FONT_SIZE))
            .color(theme::TEXT_BRIGHT),
    );
    child.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        if icons::button(ui, Icon::Minimize, "Hide").clicked() {
            hide = true;
        }
        actions(ui);
    });
    hide
}

/// Full-width list row with hover/selection painting.
pub fn list_row(ui: &mut egui::Ui, selected: bool) -> (Rect, egui::Response) {
    let (rect, response) =
        ui.allocate_exact_size(vec2(ui.available_width(), ROW_HEIGHT), Sense::click());
    keep_arrow_cursor(ui.ctx(), response.id);
    let fill = if selected {
        *theme::SELECTION
    } else if response.hovered() {
        *theme::HOVER
    } else {
        Color32::TRANSPARENT
    };
    if fill != Color32::TRANSPARENT {
        ui.painter().rect_filled(
            rect.shrink2(vec2(theme::layout::ROW_PILL_INSET, 0.0)),
            theme::layout::ROW_PILL_RADIUS,
            fill,
        );
    }
    (rect, response)
}

/// Paints left-aligned text inside a row starting at `x`, returns the end x.
pub fn row_text(
    ui: &egui::Ui,
    rect: Rect,
    x: f32,
    text: &str,
    font: egui::FontId,
    color: impl Into<Color32>,
) -> f32 {
    row_text_elided(ui, rect, x, text, font, color.into()).0
}

/// Single-line layout that ends in "…" when wider than `max_width`.
pub fn elided_job(
    text: &str,
    font: egui::FontId,
    color: Color32,
    max_width: f32,
) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::simple_singleline(text.to_owned(), font, color);
    job.wrap = egui::text::TextWrapping {
        max_width: max_width.max(0.0),
        max_rows: 1,
        break_anywhere: true,
        overflow_character: Some('…'),
    };
    job
}

/// Paints `text` from `x`, elided with "…" at the row's right edge (minus
/// padding). Returns the end x and whether the text was shortened, so callers
/// can offer the full text as a tooltip.
pub fn row_text_elided(
    ui: &egui::Ui,
    rect: Rect,
    x: f32,
    text: &str,
    font: egui::FontId,
    color: Color32,
) -> (f32, bool) {
    let max_width = rect.right() - 6.0 - x;
    let galley = ui
        .painter()
        .layout_job(elided_job(text, font, color, max_width));
    let width = galley.size().x;
    let elided = galley.elided;
    let pos = egui::pos2(x, rect.center().y - galley.size().y / 2.0);
    ui.painter().with_clip_rect(rect).galley(pos, galley, color);
    (x + width, elided)
}

/// Hoverable text widget for the status bar and toolbar dropdowns.
pub fn flat_widget(
    ui: &mut egui::Ui,
    icon: Option<(Icon, Color32)>,
    text: &str,
    font: egui::FontId,
    color: impl Into<Color32>,
    chevron: bool,
    height: f32,
) -> egui::Response {
    let color = color.into();
    let galley = ui.painter().layout_no_wrap(text.to_owned(), font, color);
    let icon_width = if icon.is_some() { 22.0 } else { 0.0 };
    let chevron_width = if chevron { 18.0 } else { 0.0 };
    let width = 8.0 + icon_width + galley.size().x + chevron_width + 8.0;
    let (rect, response) = ui.allocate_exact_size(vec2(width, height), Sense::click());
    let fill = if response.is_pointer_button_down_on() {
        *theme::PRESSED
    } else if response.hovered() {
        *theme::HOVER
    } else {
        Color32::TRANSPARENT
    };
    ui.painter().rect_filled(rect, 6.0, fill);
    let mut x = rect.left() + 8.0;
    if let Some((icon, icon_color)) = icon {
        icons::paint(
            ui.painter(),
            Rect::from_min_size(egui::pos2(x, rect.center().y - 8.0), vec2(16.0, 16.0)),
            icon,
            icon_color,
        );
        x += icon_width;
    }
    let text_pos = egui::pos2(x, rect.center().y - galley.size().y / 2.0);
    x += galley.size().x;
    ui.painter().galley(text_pos, galley, color);
    if chevron {
        icons::paint(
            ui.painter(),
            Rect::from_min_size(egui::pos2(x + 2.0, rect.center().y - 7.0), vec2(14.0, 14.0)),
            Icon::ChevronDown,
            *theme::TEXT_SECONDARY,
        );
    }
    response
}

/// [`flat_widget`] whose text is elided with "…" so it never grows past
/// `max_width` (status messages must not run over the breadcrumbs).
pub fn flat_widget_elided(
    ui: &mut egui::Ui,
    text: &str,
    font: egui::FontId,
    color: impl Into<Color32>,
    max_width: f32,
    height: f32,
) -> egui::Response {
    let color = color.into();
    let galley =
        ui.painter()
            .layout_job(elided_job(text, font, color, (max_width - 16.0).max(0.0)));
    let width = 8.0 + galley.size().x + 8.0;
    let (rect, response) = ui.allocate_exact_size(vec2(width, height), Sense::click());
    let fill = if response.is_pointer_button_down_on() {
        *theme::PRESSED
    } else if response.hovered() {
        *theme::HOVER
    } else {
        Color32::TRANSPARENT
    };
    ui.painter().rect_filled(rect, 6.0, fill);
    let text_pos = egui::pos2(rect.left() + 8.0, rect.center().y - galley.size().y / 2.0);
    ui.painter().galley(text_pos, galley, color);
    response
}

/// Single-line search/input field in the reference look.
pub fn search_field(
    ui: &mut egui::Ui,
    id: egui::Id,
    text: &mut String,
    hint: &str,
    width: f32,
) -> egui::Response {
    let frame = egui::Frame::new()
        .fill(*theme::FIELD)
        .stroke(egui::Stroke::new(1.0_f32, theme::BORDER))
        .corner_radius(4)
        .inner_margin(egui::Margin::symmetric(6, 2));
    frame
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                icons::show(ui, Icon::Search, 14.0, *theme::TEXT_SECONDARY);
                ui.add(
                    egui::TextEdit::singleline(text)
                        .id(id)
                        .frame(false)
                        .desired_width(width)
                        .hint_text(hint),
                )
            })
            .inner
        })
        .inner
}

pub fn pluralize(count: usize, singular: &str, plural: &str) -> String {
    format!("{count} {}", if count == 1 { singular } else { plural })
}

/// Square of [`theme::layout::ICON_SIZE`] centred on `(x + size / 2, center_y)`.
pub fn icon_rect(x: f32, center_y: f32) -> Rect {
    let size = theme::layout::ICON_SIZE;
    Rect::from_min_size(egui::pos2(x, center_y - size / 2.0), vec2(size, size))
}

/// Icon followed by text in one line. The icon is centred on the text's line box
/// (the galley's row height, not the font size). Returns the allocated rect.
pub fn icon_label(
    ui: &mut egui::Ui,
    icon: Icon,
    icon_color: impl Into<Color32>,
    text: &str,
    color: impl Into<Color32>,
) -> Rect {
    let galley = ui.painter().layout_no_wrap(
        text.to_owned(),
        egui::FontId::proportional(theme::UI_FONT_SIZE),
        color.into(),
    );
    let height = galley.size().y.max(theme::layout::ICON_SIZE);
    let width = theme::layout::ICON_SIZE + theme::layout::ICON_TEXT_GAP + galley.size().x;
    let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
    icons::paint(
        ui.painter(),
        icon_rect(rect.left(), rect.center().y),
        icon,
        icon_color,
    );
    let text_pos = egui::pos2(
        rect.left() + theme::layout::ICON_SIZE + theme::layout::ICON_TEXT_GAP,
        rect.center().y - galley.size().y / 2.0,
    );
    let color = ui.visuals().text_color();
    ui.painter().galley(text_pos, galley, color);
    rect
}

/// What [`tree_row`] draws. Every tree and list in a tool window goes through it
/// so rows share one inset, indent step, chevron column and icon alignment.
/// Paints a custom icon (badge) into the 16px icon slot of a [`TreeRow`].
pub type IconPainter<'a> = &'a dyn Fn(&egui::Painter, Rect);

pub struct TreeRow<'a> {
    pub depth: usize,
    /// `Some(expanded)` draws a chevron; `None` leaves the column empty.
    pub expanded: Option<bool>,
    pub selected: bool,
    /// Faint fill for unselected rows (excluded or test folders).
    pub tint: Option<Color32>,
    /// Width reserved between the chevron column and the icon (a checkbox).
    pub reserve: f32,
    /// Keeps the icon column empty so text lines up with sibling rows that have icons.
    pub reserve_icon: bool,
    pub icon: Option<(Icon, Color32)>,
    /// Custom icon painter (badges) in place of `icon`; gets the 16px icon rect.
    pub paint_icon: Option<IconPainter<'a>>,
    pub text: &'a str,
    pub font: egui::FontId,
    pub color: Color32,
    pub secondary: Option<(&'a str, Color32)>,
}

impl<'a> TreeRow<'a> {
    pub fn new(text: &'a str) -> Self {
        Self {
            depth: 0,
            expanded: None,
            selected: false,
            tint: None,
            reserve: 0.0,
            reserve_icon: false,
            icon: None,
            paint_icon: None,
            text,
            font: egui::FontId::proportional(theme::UI_FONT_SIZE),
            color: *theme::TEXT,
            secondary: None,
        }
    }
}

/// Geometry and interaction of a painted [`TreeRow`].
pub struct TreeRowOut {
    pub rect: Rect,
    pub response: egui::Response,
    pub chevron: Rect,
    /// Where the reserved checkbox space starts (centred vertically on the row).
    pub reserved: Rect,
    pub icon: Rect,
    pub text_start: f32,
    /// End of the primary text (before any secondary text).
    pub text_end: f32,
    pub elided: bool,
}

impl TreeRowOut {
    /// Whether the last click landed on the chevron column.
    pub fn on_chevron(&self) -> bool {
        self.response
            .interact_pointer_pos()
            .is_some_and(|pos| pos.x < self.chevron.right() + 2.0)
    }
}

/// Paints a row with the shared tree geometry; see [`TreeRow`].
pub fn tree_row(ui: &mut egui::Ui, spec: TreeRow<'_>) -> TreeRowOut {
    let (rect, response) = list_row(ui, spec.selected);
    if let (false, Some(tint)) = (spec.selected, spec.tint) {
        ui.painter().rect_filled(
            rect.shrink2(vec2(theme::layout::ROW_PILL_INSET, 0.0)),
            theme::layout::ROW_PILL_RADIUS,
            tint,
        );
    }
    let cy = rect.center().y;
    let mut x = rect.left() + theme::layout::indent(spec.depth);
    let chevron = icon_rect(x, cy);
    if let Some(expanded) = spec.expanded {
        icons::paint(
            ui.painter(),
            chevron,
            if expanded {
                Icon::ChevronDown
            } else {
                Icon::ChevronRight
            },
            theme::TEXT_SECONDARY,
        );
    }
    x += theme::layout::CHEVRON_COLUMN;
    let reserved = Rect::from_min_size(egui::pos2(x, cy - 7.0), vec2(spec.reserve, 14.0));
    if spec.reserve > 0.0 {
        x += spec.reserve + theme::layout::ICON_TEXT_GAP;
    }
    let icon = icon_rect(x, cy);
    if let Some(paint) = spec.paint_icon {
        paint(ui.painter(), icon);
    } else if let Some((glyph, color)) = spec.icon {
        icons::paint(ui.painter(), icon, glyph, color);
    }
    if spec.reserve_icon || spec.icon.is_some() || spec.paint_icon.is_some() {
        x += theme::layout::ICON_SIZE + theme::layout::ICON_TEXT_GAP;
    }
    let text_start = x;
    let (text_end, elided) =
        row_text_elided(ui, rect, text_start, spec.text, spec.font, spec.color);
    if let Some((secondary, color)) = spec.secondary {
        row_text(
            ui,
            rect,
            text_end + theme::layout::SECONDARY_GAP,
            secondary,
            egui::FontId::proportional(theme::UI_FONT_SIZE),
            color,
        );
    }
    TreeRowOut {
        rect,
        response,
        chevron,
        reserved,
        icon,
        text_start,
        text_end,
        elided,
    }
}

/// A toolbar line that clips at the panel edge and wraps onto a second line
/// instead of painting over neighbouring panels.
pub fn toolbar_row<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let bounds = Rect::from_min_max(
        ui.cursor().min,
        egui::pos2(ui.max_rect().right(), f32::INFINITY),
    );
    ui.scope(|ui| {
        ui.set_clip_rect(ui.clip_rect().intersect(bounds));
        ui.spacing_mut().item_spacing.x = theme::layout::TOOLBAR_GAP;
        ui.horizontal_wrapped(|ui| {
            ui.set_min_height(theme::layout::ICON_BUTTON);
            add(ui)
        })
        .inner
    })
    .inner
}

/// Status-bar spinner that advances in 10 Hz steps instead of repainting every
/// frame like `egui::Spinner`, so a busy background task costs ~10 frames/s.
pub fn busy_spinner(ui: &mut egui::Ui, size: f32, color: Color32) {
    const STEP_SECONDS: f64 = 0.1;
    const STEPS: u32 = 10;
    let (rect, _) = ui.allocate_exact_size(vec2(size, size), Sense::hover());
    if !ui.is_rect_visible(rect) {
        return;
    }
    let time = ui.input(|input| input.time);
    let step = (time / STEP_SECONDS) as u32 % STEPS;
    let start = step as f32 / STEPS as f32 * std::f32::consts::TAU;
    let radius = size / 2.0 - 2.0;
    let points: Vec<egui::Pos2> = (0..=8)
        .map(|i| {
            let angle = start + i as f32 / 8.0 * std::f32::consts::PI * 1.5;
            rect.center() + radius * vec2(angle.cos(), angle.sin())
        })
        .collect();
    ui.painter()
        .add(egui::Shape::line(points, egui::Stroke::new(3.0_f32, color)));
    ui.ctx()
        .request_repaint_after(std::time::Duration::from_secs_f64(STEP_SECONDS));
}

/// A borderless menu-style button (popup and dropdown entries) that still shows
/// the hover and pressed fill; `Button::frame(false)` would paint nothing.
pub fn menu_button(button: egui::Button<'_>) -> egui::Button<'_> {
    button
        .frame(true)
        .frame_when_inactive(false)
        .stroke(egui::Stroke::NONE)
}

fn arrow_cursor_ids() -> egui::Id {
    egui::Id::new("widgets_arrow_cursor_ids")
}

/// Marks a clickable widget (list or tree row, tab) that keeps the arrow cursor
/// on hover instead of the pointing hand buttons get.
pub fn keep_arrow_cursor(ctx: &egui::Context, id: egui::Id) {
    ctx.data_mut(|data| {
        data.get_temp_mut_or_default::<Vec<egui::Id>>(arrow_cursor_ids())
            .push(id);
    });
}

/// Shows the pointing hand over hovered, enabled click-only widgets (buttons,
/// links, icon buttons). Runs once at the end of the frame; widgets that set
/// their own cursor, drag targets and rows marked with [`keep_arrow_cursor`]
/// keep theirs.
pub fn apply_pointer_cursor(ctx: &egui::Context) {
    let arrow_ids = ctx.data_mut(|data| {
        data.remove_temp::<Vec<egui::Id>>(arrow_cursor_ids())
            .unwrap_or_default()
    });
    if ctx.output(|output| output.cursor_icon) != egui::CursorIcon::Default
        || ctx.dragged_id().is_some()
    {
        return;
    }
    let hovered = ctx.interaction_snapshot(|snapshot| snapshot.hovered.clone());
    let clickable = hovered.iter().any(|id| {
        !arrow_ids.contains(id)
            && ctx.read_response(*id).is_some_and(|response| {
                response.enabled() && response.sense.senses_click() && !response.sense.senses_drag()
            })
    });
    if clickable {
        ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pluralizes() {
        assert_eq!(pluralize(1, "problem", "problems"), "1 problem");
        assert_eq!(pluralize(3, "problem", "problems"), "3 problems");
    }

    #[test]
    fn long_names_are_elided_with_an_ellipsis() {
        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            let font = egui::FontId::proportional(13.0);
            let layout = |text: &str, width: f32| {
                ctx.fonts_mut(|fonts| {
                    fonts.layout_job(elided_job(text, font.clone(), Color32::WHITE, width))
                })
            };
            let long = layout("ExtremelyLongFileNameThatDoesNotFit.kt", 80.0);
            assert!(long.elided);
            assert!(long.size().x <= 80.0 + 1.0, "{}", long.size().x);
            let last = long.rows.last().and_then(|row| row.glyphs.last());
            assert_eq!(last.map(|glyph| glyph.chr), Some('…'));
            let short = layout("App.kt", 80.0);
            assert!(!short.elided);
            assert_eq!(short.text(), "App.kt");
        });
    }

    /// Runs two frames with the pointer at `pointer` (hover is resolved from the
    /// previous frame) and returns the cursor of the second one.
    fn cursor_over(pointer: egui::Pos2, add: impl Fn(&mut egui::Ui)) -> egui::CursorIcon {
        let ctx = egui::Context::default();
        let input = || egui::RawInput {
            screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, vec2(400.0, 300.0))),
            events: vec![egui::Event::PointerMoved(pointer)],
            ..Default::default()
        };
        let mut cursor = egui::CursorIcon::Default;
        for _ in 0..2 {
            let output = ctx.run(input(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| add(ui));
                apply_pointer_cursor(ctx);
            });
            cursor = output.platform_output.cursor_icon;
        }
        cursor
    }

    fn in_frame<R>(add: impl FnOnce(&mut egui::Ui) -> R) -> R {
        let ctx = egui::Context::default();
        crate::theme::install(&ctx);
        let mut out = None;
        let mut add = Some(add);
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, vec2(400.0, 300.0))),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default()
                    .frame(egui::Frame::NONE)
                    .show(ctx, |ui| {
                        if let Some(add) = add.take() {
                            out = Some(add(ui));
                        }
                    });
            },
        );
        out.expect("frame ran")
    }

    #[test]
    fn icon_label_centres_the_icon_on_the_text_line_box() {
        let (rect, line_center, icon_center) = in_frame(|ui| {
            let rect = icon_label(ui, Icon::Folder, Color32::WHITE, "Changes", Color32::WHITE);
            let galley = ui.painter().layout_no_wrap(
                "Changes".into(),
                egui::FontId::proportional(theme::UI_FONT_SIZE),
                Color32::WHITE,
            );
            let text_top = rect.center().y - galley.size().y / 2.0;
            let line_center = text_top + galley.rows[0].rect().center().y;
            (
                rect,
                line_center,
                icon_rect(rect.left(), rect.center().y).center().y,
            )
        });
        assert!(
            (icon_center - line_center).abs() <= 0.5,
            "{icon_center} vs {line_center}"
        );
        assert!(rect.height() >= theme::layout::ICON_SIZE);
    }

    #[test]
    fn tree_rows_share_one_inset_and_icon_column() {
        let (first, second, nested) = in_frame(|ui| {
            let plain = |ui: &mut egui::Ui, depth, expanded| {
                tree_row(
                    ui,
                    TreeRow {
                        depth,
                        expanded,
                        icon: Some((Icon::Folder, Color32::WHITE)),
                        ..TreeRow::new("name")
                    },
                )
            };
            (
                plain(ui, 0, Some(true)),
                plain(ui, 0, None),
                plain(ui, 1, None),
            )
        });
        // Same pill inset whether or not the row has a chevron.
        assert_eq!(first.rect.left(), second.rect.left());
        assert_eq!(first.icon.left(), second.icon.left());
        assert_eq!(first.rect.height(), theme::layout::ROW_HEIGHT);
        // One indent step per depth, icon centred on the row.
        assert_eq!(
            nested.icon.left() - second.icon.left(),
            theme::layout::TREE_INDENT
        );
        assert_eq!(first.icon.center().y, first.rect.center().y);
        assert_eq!(
            first.icon.left() - first.rect.left(),
            theme::layout::CONTENT_INSET + theme::layout::CHEVRON_COLUMN
        );
    }

    #[test]
    fn buttons_show_the_pointing_hand_and_rows_keep_the_arrow() {
        let over_button = cursor_over(egui::pos2(20.0, 15.0), |ui| {
            ui.add_sized(vec2(120.0, 30.0), egui::Button::new("Create"));
        });
        assert_eq!(over_button, egui::CursorIcon::PointingHand);
        let over_row = cursor_over(egui::pos2(20.0, 12.0), |ui| {
            list_row(ui, false);
        });
        assert_eq!(over_row, egui::CursorIcon::Default);
        let over_disabled = cursor_over(egui::pos2(20.0, 15.0), |ui| {
            ui.add_enabled_ui(false, |ui| {
                ui.add_sized(vec2(120.0, 30.0), egui::Button::new("Create"));
            });
        });
        assert_eq!(over_disabled, egui::CursorIcon::Default);
    }
}
