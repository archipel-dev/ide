//! Small reusable pieces of the workbench chrome.

use crate::icons::{self, Icon};
use crate::theme;
use archipel_kit::behaviour::{Behaviour, Behaviours};
use archipel_kit::components::{ListRow, MenuItem};
use archipel_kit::parts::{State, Surface, Tone, row_pill};
use archipel_kit::tokens::size;
use archipel_kit::{Theme, cursor};
use eframe::egui::{self, Color32, Rect, Sense, vec2};

/// Rounded "island" container used for the editor and docked tool windows: the kit's
/// island surface without its inner padding (rows inset themselves), with the gap between
/// islands as outer margin.
pub fn island_frame(ctx: &egui::Context) -> egui::Frame {
    Surface::Island
        .frame(ctx)
        .inner_margin(egui::Margin::ZERO)
        .outer_margin(egui::Margin::same(theme::ISLAND_GAP as i8))
}

/// Flat frame for chrome areas (toolbar, stripes, status bar): the window frame color.
pub fn chrome_frame(ctx: &egui::Context) -> egui::Frame {
    egui::Frame::new().fill(Theme::colors(ctx).frame)
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

/// Full-width list row with hover/selection painting, for callers that paint the
/// content themselves ([`row_text`]). Rows keep the arrow cursor (the kit's rule: rows
/// select, buttons act). Prefer the kit's `ListRow` where its content model fits.
pub fn list_row(ui: &mut egui::Ui, selected: bool) -> (Rect, egui::Response) {
    let (rect, response) =
        ui.allocate_exact_size(vec2(ui.available_width(), ROW_HEIGHT), Sense::click());
    cursor::on_hover(&response, egui::CursorIcon::Default);
    if ui.is_rect_visible(rect) {
        let colors = Theme::colors(ui.ctx());
        let state = State::of(&response).selected(selected);
        row_pill(ui.painter(), rect, state.row_fill(&colors));
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
    cursor::on_hover_action(&response);
    let colors = Theme::colors(ui.ctx());
    let fill = State::of(&response).flat_fill(&colors);
    ui.painter()
        .rect_filled(rect, archipel_kit::tokens::radius::ICON_BUTTON, fill);
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
            colors.text_secondary,
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

/// Single-line search/input field. It has the look of the kit's `TextField::search`
/// (field surface, focus outline, placeholder color, 28 pt height) but keeps `id` on the
/// text edit, because tool windows move keyboard focus by id (the kit's `TextField` has no
/// id yet).
pub fn search_field(
    ui: &mut egui::Ui,
    id: egui::Id,
    text: &mut String,
    hint: &str,
    width: f32,
) -> egui::Response {
    let ctx = ui.ctx().clone();
    let colors = Theme::colors(&ctx);
    let focused = ui.memory(|memory| memory.has_focus(id));
    Surface::Field
        .frame(&ctx)
        .stroke(Surface::field_stroke(&ctx, focused, false))
        .show(ui, |ui| {
            ui.set_min_height(size::CONTROL_HEIGHT - 2.0);
            ui.horizontal_centered(|ui| {
                icons::show(ui, Icon::Search, size::ICON, colors.text_placeholder);
                ui.add(
                    egui::TextEdit::singleline(text)
                        .id(id)
                        .frame(false)
                        .desired_width(width)
                        .margin(vec2(0.0, 0.0))
                        .hint_text(egui::RichText::new(hint).color(colors.text_placeholder)),
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
///
/// Rows are the kit's `ListRow`. A row that needs something the kit's row does not have
/// yet (a checkbox slot, a custom icon painter) is drawn by the same parts with the same
/// geometry, so mixed lists stay aligned; see `docs/ui-migration.md`.
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
    /// Icon and its color; the color picks the kit tone with the same value (icon gray by
    /// default).
    pub icon: Option<(Icon, Color32)>,
    /// Custom icon painter (badges) in place of `icon`; gets the 16px icon rect.
    pub paint_icon: Option<IconPainter<'a>>,
    pub text: &'a str,
    /// Used by rows drawn by hand; the kit's row uses its own label font.
    pub font: egui::FontId,
    /// Label color; picks the kit tone with the same value (see `tone`).
    pub color: Color32,
    /// Label tone, taking precedence over `color`.
    pub tone: Option<Tone>,
    pub secondary: Option<(&'a str, Color32)>,
    /// Tooltips, context menus, drag and drop: attached to the row's response.
    pub behaviours: Behaviours<'a>,
    /// Use the kit's expander: the chevron is its own button ("Expand name"), and a click
    /// on it, a double-click or Left/Right on a focused row reports [`TreeRowOut::toggled`]
    /// instead of a click on the row. Off by default so rows that test
    /// [`TreeRowOut::on_chevron`] keep working until their area migrates.
    pub kit_expander: bool,
}

impl<'a> TreeRow<'a> {
    /// Adds a kit behaviour (`ContextMenu`, `DragSource`, `DropTarget`, `Tooltip` …).
    pub fn with(mut self, behaviour: impl Behaviour + 'a) -> Self {
        self.behaviours.push(behaviour);
        self
    }

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
            tone: None,
            secondary: None,
            behaviours: Behaviours::default(),
            kit_expander: false,
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
    /// The kit's expander asked to flip the node (see [`TreeRow::kit_expander`]).
    pub toggled: bool,
}

impl TreeRowOut {
    /// Whether the last click landed on the chevron column.
    pub fn on_chevron(&self) -> bool {
        self.response
            .interact_pointer_pos()
            .is_some_and(|pos| pos.x < self.chevron.right() + 2.0)
    }
}

/// The kit tone whose color in the active theme is `color`, or `fallback`.
pub fn tone_of(ctx: &egui::Context, color: Color32, fallback: Tone) -> Tone {
    let colors = Theme::colors(ctx);
    Tone::ALL
        .into_iter()
        .find(|tone| tone.color(&colors) == color)
        .unwrap_or(fallback)
}

/// Gap between the chevron column and the content of a kit row.
const CHEVRON_GAP: f32 = 2.0;

/// Paints a row with the shared tree geometry; see [`TreeRow`].
pub fn tree_row(ui: &mut egui::Ui, spec: TreeRow<'_>) -> TreeRowOut {
    let ctx = ui.ctx().clone();
    let colors = Theme::colors(&ctx);
    if let (false, Some(tint)) = (spec.selected, spec.tint) {
        // Under the row: the kit row paints on top and an opaque selection hides it.
        let below = Rect::from_min_size(ui.cursor().min, vec2(ui.available_width(), ROW_HEIGHT));
        row_pill(ui.painter(), below, tint);
    }
    let label_tone = spec
        .tone
        .unwrap_or_else(|| tone_of(&ctx, spec.color, Tone::Default));
    let custom = spec.paint_icon.is_some() || spec.reserve > 0.0 || spec.reserve_icon;
    let has_icon = spec.icon.is_some() || spec.paint_icon.is_some();
    let behaviours = spec.behaviours;
    let mut toggled = false;
    let response = if custom {
        let (rect, response) = list_row(ui, spec.selected);
        response.widget_info(|| {
            egui::WidgetInfo::selected(
                egui::WidgetType::SelectableLabel,
                true,
                spec.selected,
                spec.text,
            )
        });
        let _ = rect;
        behaviours.attach(ui, response)
    } else {
        let mut row = ListRow::new(spec.text)
            .depth(spec.depth)
            .expanded(spec.expanded.filter(|_| spec.kit_expander))
            .selected(spec.selected)
            .label_tone(label_tone);
        if let Some((icon, color)) = spec.icon {
            row = row.icon(icon).icon_tone(tone_of(&ctx, color, Tone::Icon));
        }
        if let Some((secondary, _)) = spec.secondary {
            row = row.secondary(secondary);
        }
        let shown = row
            .with(move |ui: &mut egui::Ui, response: egui::Response| {
                behaviours.attach(ui, response)
            })
            .show(ui);
        toggled = shown.toggled;
        shown.response
    };
    let rect = response.rect;
    let cy = rect.center().y;
    let chevron = Rect::from_min_size(
        egui::pos2(
            rect.left() + size::indent(spec.depth) - size::ROW_PILL_INSET,
            cy - size::CHEVRON_COLUMN / 2.0,
        ),
        vec2(size::CHEVRON_COLUMN, size::CHEVRON_COLUMN),
    );
    let mut x = chevron.right() + CHEVRON_GAP;
    let reserved = Rect::from_min_size(egui::pos2(x, cy - 7.0), vec2(spec.reserve, 14.0));
    if spec.reserve > 0.0 {
        x += spec.reserve + size::ICON_TEXT_GAP;
    }
    let icon = icon_rect(x, cy);
    if let (Some(expanded), true) = (spec.expanded, custom || !spec.kit_expander) {
        icons::paint(
            ui.painter(),
            chevron,
            if expanded {
                Icon::ChevronDown
            } else {
                Icon::ChevronRight
            },
            colors.text_secondary,
        );
    }
    if custom {
        if let Some(paint) = spec.paint_icon {
            paint(ui.painter(), icon);
        } else if let Some((glyph, color)) = spec.icon {
            icons::paint(ui.painter(), icon, glyph, color);
        }
    }
    if spec.reserve_icon || has_icon {
        x += size::ICON + size::ICON_TEXT_GAP;
    }
    let text_start = x;
    let (text_end, elided) = if custom {
        let (end, elided) = row_text_elided(ui, rect, text_start, spec.text, spec.font, spec.color);
        if let Some((secondary, color)) = spec.secondary {
            row_text(
                ui,
                rect,
                end + size::SECONDARY_GAP,
                secondary,
                egui::FontId::proportional(theme::UI_FONT_SIZE),
                color,
            );
        }
        (end, elided)
    } else {
        let width = ui
            .painter()
            .layout_no_wrap(
                spec.text.to_owned(),
                egui::FontId::proportional(theme::UI_FONT_SIZE),
                Color32::WHITE,
            )
            .size()
            .x;
        (text_start + width, text_start + width > rect.right() - 6.0)
    };
    TreeRowOut {
        rect,
        response,
        chevron,
        reserved,
        icon,
        text_start,
        text_end,
        elided,
        toggled,
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

/// A menu or popup entry: the kit's `MenuItem` (selection-colored hover, arrow cursor,
/// icon column). Add it with `ui.add(menu_item("Open"))`.
pub fn menu_item(label: impl Into<String>) -> MenuItem {
    MenuItem::new(label)
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
        crate::theme::install(&ctx);
        let input = || egui::RawInput {
            screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, vec2(400.0, 300.0))),
            events: vec![egui::Event::PointerMoved(pointer)],
            ..Default::default()
        };
        let mut cursor = egui::CursorIcon::Default;
        for _ in 0..2 {
            let output = ctx.run(input(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| add(ui));
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
        assert_eq!(second.rect.height(), theme::layout::ROW_HEIGHT);
        // One indent step per depth, icon centred on the row.
        assert_eq!(nested.icon.left() - second.icon.left(), size::TREE_INDENT);
        assert_eq!(first.icon.center().y, first.rect.center().y);
        // The kit's row: the chevron column starts one pill inset left of the indent, and
        // the content follows it after a 2 pt gap.
        assert_eq!(
            first.icon.left() - first.rect.left(),
            theme::layout::CONTENT_INSET - size::ROW_PILL_INSET
                + size::CHEVRON_COLUMN
                + CHEVRON_GAP
        );
    }

    #[test]
    fn kit_buttons_show_the_pointing_hand_and_rows_keep_the_arrow() {
        let over_button = cursor_over(egui::pos2(20.0, 15.0), |ui| {
            icons::button(ui, Icon::Refresh, "Refresh");
        });
        assert_eq!(over_button, egui::CursorIcon::PointingHand);
        let over_row = cursor_over(egui::pos2(20.0, 12.0), |ui| {
            list_row(ui, false);
        });
        assert_eq!(over_row, egui::CursorIcon::Default);
        let over_tree_row = cursor_over(egui::pos2(60.0, 12.0), |ui| {
            tree_row(ui, TreeRow::new("name"));
        });
        assert_eq!(over_tree_row, egui::CursorIcon::Default);
        let over_flat_widget = cursor_over(egui::pos2(20.0, 12.0), |ui| {
            flat_widget(
                ui,
                None,
                "main",
                egui::FontId::proportional(13.0),
                Color32::WHITE,
                false,
                24.0,
            );
        });
        assert_eq!(over_flat_widget, egui::CursorIcon::PointingHand);
    }

    #[test]
    fn tree_rows_pick_the_kit_tone_of_their_color() {
        let ctx = egui::Context::default();
        crate::theme::install(&ctx);
        let colors = Theme::colors(&ctx);
        // Tones that share a color (modified files and links) are interchangeable: the
        // tone found draws the same color.
        let found = tone_of(&ctx, colors.vcs_modified, Tone::Default);
        assert_eq!(found.color(&colors), colors.vcs_modified);
        assert_ne!(found, Tone::Default);
        assert_eq!(
            tone_of(&ctx, colors.text_strong, Tone::Default),
            Tone::Strong
        );
        assert_eq!(
            tone_of(&ctx, Color32::from_rgb(1, 2, 3), Tone::Icon),
            Tone::Icon
        );
    }
}
