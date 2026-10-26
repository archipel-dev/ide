//! Markdown editor chrome: the Editor/Split/Preview island, the native live preview and
//! the floating formatting bar. All state transitions go through `markdown.*` commands.

use crate::theme;
use archipel_ide_language::LanguageAnalysis;
use archipel_ide_markdown::{Align, Block, BlockKind, Document, Inline, Layout, scroll::ScrollMap};
use eframe::egui::{self, Color32, FontId, RichText, Stroke, text::LayoutJob};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::ops::Range;
use std::path::{Path, PathBuf};

const BODY_SIZE: f32 = 14.0;
const BLOCK_GAP: f32 = 10.0;
const PADDING: f32 = 18.0;

struct Parsed {
    path: PathBuf,
    revision: u64,
    document: Document,
}

/// Per-app Markdown view state: parse cache, code highlighting cache and scroll sync.
#[derive(Default)]
pub struct State {
    parsed: Option<Parsed>,
    highlights: HashMap<u64, LanguageAnalysis>,
    map: ScrollMap,
    last_editor_y: f32,
    last_preview_y: f32,
    pending_editor_y: Option<f32>,
    pending_preview_y: Option<f32>,
    synced_path: Option<PathBuf>,
}

/// What the preview measured this frame.
pub struct PreviewOutput {
    pub scroll_y: f32,
}

impl State {
    /// Scroll offset to apply to the editor this frame (set by preview scrolling).
    pub fn take_editor_scroll(&mut self) -> Option<f32> {
        self.pending_editor_y.take()
    }

    /// Reconciles the two scroll offsets after both panes were drawn. Whichever pane
    /// the user moved drives the other. `applied_*` tell whether a pane was set
    /// programmatically this frame (its movement must not echo back).
    pub fn sync(
        &mut self,
        editor_y: f32,
        row_height: f32,
        preview_y: f32,
        applied_editor: bool,
        applied_preview: bool,
    ) -> bool {
        let moved_editor = !applied_editor && (editor_y - self.last_editor_y).abs() > 0.5;
        let moved_preview = !applied_preview && (preview_y - self.last_preview_y).abs() > 0.5;
        self.last_editor_y = editor_y;
        self.last_preview_y = preview_y;
        if self.map.is_empty() || row_height <= 0.0 {
            return false;
        }
        if moved_editor && !moved_preview {
            let target = self.map.line_to_y(editor_y / row_height);
            if (target - preview_y).abs() > 0.5 {
                self.pending_preview_y = Some(target);
                return true;
            }
        } else if moved_preview && !moved_editor {
            let target = self.map.y_to_line(preview_y) * row_height;
            if (target - editor_y).abs() > 0.5 {
                self.pending_editor_y = Some(target);
                return true;
            }
        }
        false
    }

    /// Forgets scroll positions when another file becomes active.
    pub fn reset_for(&mut self, path: &Path) {
        if self.synced_path.as_deref() != Some(path) {
            self.synced_path = Some(path.to_owned());
            self.pending_editor_y = None;
            self.pending_preview_y = None;
            self.last_editor_y = 0.0;
            self.last_preview_y = 0.0;
        }
    }

    /// Scroll offset to apply to the preview this frame (set by editor scrolling).
    pub fn take_preview_scroll(&mut self) -> Option<f32> {
        self.pending_preview_y.take()
    }
}

fn button_text(text: &str, selected: bool) -> RichText {
    let color = if selected {
        *theme::TEXT_BRIGHT
    } else {
        *theme::TEXT_SECONDARY
    };
    RichText::new(text).size(12.0).color(color)
}

/// The Editor / Split / Preview island. Returns the layout the user picked.
pub fn layout_island(
    ctx: &egui::Context,
    top_right: egui::Pos2,
    current: Layout,
) -> Option<Layout> {
    let mut picked = None;
    egui::Area::new(egui::Id::new("markdown_layout_island"))
        .order(egui::Order::Foreground)
        .pivot(egui::Align2::RIGHT_TOP)
        .fixed_pos(top_right + egui::vec2(-8.0, 0.0))
        .show(ctx, |ui| {
            egui::Frame::new()
                .fill(*theme::POPUP)
                .stroke(Stroke::new(1.0_f32, *theme::POPUP_BORDER))
                .corner_radius(6)
                .inner_margin(egui::Margin::symmetric(3, 2))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 2.0;
                        for (layout, label, tip) in [
                            (Layout::EditorOnly, "Editor", "Editor only"),
                            (Layout::Split, "Split", "Editor and preview"),
                            (Layout::PreviewOnly, "Preview", "Preview only"),
                        ] {
                            let selected = layout == current;
                            let button = egui::Button::new(button_text(label, selected))
                                .fill(if selected {
                                    *theme::SELECTION
                                } else {
                                    Color32::TRANSPARENT
                                })
                                .corner_radius(4);
                            if ui.add(button).on_hover_text(tip).clicked() {
                                picked = Some(layout);
                            }
                        }
                    });
                });
        });
    picked
}

/// Floating formatting bar shown above a non-empty selection. Returns the command to run.
pub fn floating_bar(
    ctx: &egui::Context,
    anchor: egui::Pos2,
    bounds: egui::Rect,
) -> Option<(&'static str, Value)> {
    let mut chosen = None;
    let entries: [(&str, &str, &'static str, Value); 9] = [
        ("B", "Bold", "markdown.bold", Value::Null),
        ("I", "Italic", "markdown.italic", Value::Null),
        ("S", "Strikethrough", "markdown.strikethrough", Value::Null),
        ("</>", "Code", "markdown.code", Value::Null),
        ("Link", "Create link", "markdown.link", Value::Null),
        ("H1", "Heading 1", "markdown.heading", json!({"level": 1})),
        ("H2", "Heading 2", "markdown.heading", json!({"level": 2})),
        ("H3", "Heading 3", "markdown.heading", json!({"level": 3})),
        (
            "•",
            "Bullet list",
            "markdown.list",
            json!({"kind": "bullet"}),
        ),
    ];
    let width = 270.0;
    let x = anchor.x.clamp(
        bounds.left() + 4.0,
        (bounds.right() - width).max(bounds.left()),
    );
    let y = (anchor.y - 52.0).max(bounds.top() + 2.0);
    egui::Area::new(egui::Id::new("markdown_floating_bar"))
        .order(egui::Order::Foreground)
        .fixed_pos(egui::pos2(x, y))
        .show(ctx, |ui| {
            egui::Frame::new()
                .fill(*theme::POPUP)
                .stroke(Stroke::new(1.0_f32, *theme::POPUP_BORDER))
                .corner_radius(6)
                .inner_margin(egui::Margin::symmetric(4, 3))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 1.0;
                        for (label, tip, command, params) in entries {
                            let text = match label {
                                "B" => button_text(label, true).strong(),
                                "I" => button_text(label, true).italics(),
                                "S" => button_text(label, true).strikethrough(),
                                _ => button_text(label, true),
                            };
                            let button = egui::Button::new(text)
                                .fill(Color32::TRANSPARENT)
                                .corner_radius(4);
                            if ui.add(button).on_hover_text(tip).clicked() {
                                chosen = Some((command, params));
                            }
                        }
                    });
                });
        });
    chosen
}

#[derive(Clone, Copy, Default)]
struct Fmt {
    bold: bool,
    italic: bool,
    strike: bool,
    link: bool,
}

fn text_format(fmt: Fmt, size: f32, color: Color32) -> egui::TextFormat {
    let mut format = egui::TextFormat {
        font_id: if fmt.bold {
            theme::semibold(size)
        } else {
            FontId::proportional(size)
        },
        color: if fmt.link { *theme::ACCENT } else { color },
        italics: fmt.italic,
        ..Default::default()
    };
    if fmt.strike {
        format.strikethrough = Stroke::new(1.0_f32, color);
    }
    if fmt.link {
        format.underline = Stroke::new(1.0_f32, *theme::ACCENT);
    }
    format
}

fn push_inlines(job: &mut LayoutJob, inlines: &[Inline], fmt: Fmt, size: f32, color: Color32) {
    for inline in inlines {
        match inline {
            Inline::Text(text) => job.append(text, 0.0, text_format(fmt, size, color)),
            Inline::Code(code) => {
                let mut format = text_format(fmt, size * 0.92, color);
                format.font_id = FontId::monospace(size * 0.92);
                format.background = *theme::editor::BACKGROUND_PANEL;
                job.append(code, 0.0, format);
            }
            Inline::Emphasis(children) => {
                push_inlines(
                    job,
                    children,
                    Fmt {
                        italic: true,
                        ..fmt
                    },
                    size,
                    color,
                );
            }
            Inline::Strong(children) => {
                push_inlines(job, children, Fmt { bold: true, ..fmt }, size, color);
            }
            Inline::Strikethrough(children) => {
                push_inlines(
                    job,
                    children,
                    Fmt {
                        strike: true,
                        ..fmt
                    },
                    size,
                    color,
                );
            }
            Inline::Link { children, .. } => {
                push_inlines(job, children, Fmt { link: true, ..fmt }, size, color);
            }
            Inline::SoftBreak => job.append(" ", 0.0, text_format(fmt, size, color)),
            Inline::HardBreak => job.append("\n", 0.0, text_format(fmt, size, color)),
            Inline::Image { alt, .. } => job.append(alt, 0.0, text_format(fmt, size, color)),
            Inline::Html(_) => {}
        }
    }
}

fn inline_job(inlines: &[Inline], size: f32, color: Color32) -> LayoutJob {
    let mut job = LayoutJob::default();
    push_inlines(&mut job, inlines, Fmt::default(), size, color);
    job
}

/// Resolves an image destination to a `file://` URI next to the document, if local.
fn image_uri(base: &Path, url: &str) -> Option<String> {
    if url.contains("://") || url.starts_with("data:") {
        return None;
    }
    let url = url.split(['?', '#']).next().unwrap_or(url);
    let path = Path::new(url);
    let full = if path.is_absolute() {
        path.to_owned()
    } else {
        base.join(path)
    };
    full.is_file().then(|| format!("file://{}", full.display()))
}

struct Ctx<'a> {
    base: &'a Path,
    highlights: &'a mut HashMap<u64, LanguageAnalysis>,
}

fn paragraph(ui: &mut egui::Ui, ctx: &Ctx, inlines: &[Inline], size: f32, color: Color32) {
    // Images are laid out on their own rows between text runs.
    let mut run: Vec<Inline> = Vec::new();
    let flush = |ui: &mut egui::Ui, run: &mut Vec<Inline>| {
        if run.iter().any(|i| !matches!(i, Inline::SoftBreak)) {
            ui.add(egui::Label::new(inline_job(run, size, color)).wrap());
        }
        run.clear();
    };
    for inline in inlines {
        if let Inline::Image { url, alt } = inline {
            flush(ui, &mut run);
            match image_uri(ctx.base, url) {
                Some(uri) => {
                    ui.add(
                        egui::Image::new(uri)
                            .max_width(ui.available_width())
                            .fit_to_original_size(1.0),
                    )
                    .on_hover_text(alt);
                }
                None => {
                    ui.label(
                        RichText::new(format!("🖼 {alt} ({url})"))
                            .size(size)
                            .color(*theme::TEXT_SECONDARY),
                    );
                }
            }
        } else {
            run.push(inline.clone());
        }
    }
    flush(ui, &mut run);
}

fn code_block(ui: &mut egui::Ui, ctx: &mut Ctx, lang: Option<&str>, code: &str) {
    let key = {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        (lang, code).hash(&mut hasher);
        hasher.finish()
    };
    let analysis = ctx.highlights.entry(key).or_insert_with(|| {
        let extension = lang
            .and_then(archipel_ide_language::registry::by_id)
            .and_then(|spec| spec.extensions.first().copied());
        match extension {
            Some(extension) => {
                archipel_ide_language::analyze(Path::new(&format!("snippet.{extension}")), code)
            }
            None => LanguageAnalysis::default(),
        }
    });
    let font = FontId::monospace(BODY_SIZE * 0.93);
    egui::Frame::new()
        .fill(*theme::editor::BACKGROUND_PANEL)
        .stroke(Stroke::new(1.0_f32, *theme::POPUP_BORDER))
        .corner_radius(6)
        .inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = 0.0;
            if let Some(lang) = lang {
                ui.label(RichText::new(lang).size(11.0).color(*theme::TEXT_SECONDARY));
            }
            let mut offset = 0;
            for line in code.trim_end_matches('\n').split('\n') {
                let job = crate::editor::highlighted_line_job(line, offset, analysis, &font);
                ui.add(egui::Label::new(job).extend());
                offset += line.len() + 1;
            }
        });
}

fn list(
    ui: &mut egui::Ui,
    ctx: &mut Ctx,
    start: Option<u64>,
    items: &[archipel_ide_markdown::ListItem],
) {
    for (index, item) in items.iter().enumerate() {
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            ui.add_space(6.0);
            let (marker_rect, _) =
                ui.allocate_exact_size(egui::vec2(20.0, BODY_SIZE + 4.0), egui::Sense::hover());
            let painter = ui.painter();
            match item.task {
                Some(checked) => {
                    let boxed =
                        egui::Rect::from_center_size(marker_rect.center(), egui::vec2(13.0, 13.0));
                    painter.rect_stroke(
                        boxed,
                        2,
                        Stroke::new(1.3_f32, *theme::TEXT_SECONDARY),
                        egui::StrokeKind::Inside,
                    );
                    if checked {
                        painter.rect_filled(boxed.shrink(1.0), 2, *theme::ACCENT);
                        painter.line_segment(
                            [
                                boxed.left_center() + egui::vec2(2.5, 0.5),
                                boxed.center_bottom() + egui::vec2(-1.0, -3.0),
                            ],
                            Stroke::new(1.6_f32, Color32::WHITE),
                        );
                        painter.line_segment(
                            [
                                boxed.center_bottom() + egui::vec2(-1.0, -3.0),
                                boxed.right_top() + egui::vec2(-2.5, 3.0),
                            ],
                            Stroke::new(1.6_f32, Color32::WHITE),
                        );
                    }
                }
                None => {
                    let text = match start {
                        Some(first) => format!("{}.", first + index as u64),
                        None => "•".to_owned(),
                    };
                    painter.text(
                        marker_rect.right_top() + egui::vec2(-2.0, 1.0),
                        egui::Align2::RIGHT_TOP,
                        text,
                        FontId::proportional(BODY_SIZE),
                        *theme::TEXT_SECONDARY,
                    );
                }
            }
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 3.0;
                for block in &item.blocks {
                    render_block(ui, ctx, block, 1);
                }
            });
        });
    }
}

fn table(
    ui: &mut egui::Ui,
    id: usize,
    align: &[Align],
    header: &[Vec<Inline>],
    rows: &[Vec<Vec<Inline>>],
) {
    let cell = |ui: &mut egui::Ui, column: usize, inlines: &[Inline], bold: bool| {
        let mut job = LayoutJob::default();
        push_inlines(
            &mut job,
            inlines,
            Fmt {
                bold,
                ..Fmt::default()
            },
            BODY_SIZE,
            *theme::editor::TEXT,
        );
        let layout = match align.get(column) {
            Some(Align::Center) => egui::Layout::top_down(egui::Align::Center),
            Some(Align::Right) => egui::Layout::top_down(egui::Align::Max),
            _ => egui::Layout::top_down(egui::Align::Min),
        };
        ui.with_layout(layout, |ui| ui.add(egui::Label::new(job).extend()));
    };
    egui::Frame::new()
        .stroke(Stroke::new(1.0_f32, *theme::BORDER))
        .corner_radius(4)
        .inner_margin(egui::Margin::same(2))
        .show(ui, |ui| {
            egui::Grid::new(("markdown_table", id))
                .striped(true)
                .spacing(egui::vec2(18.0, 6.0))
                .show(ui, |ui| {
                    for (column, inlines) in header.iter().enumerate() {
                        cell(ui, column, inlines, true);
                    }
                    ui.end_row();
                    for row in rows {
                        for (column, inlines) in row.iter().enumerate() {
                            cell(ui, column, inlines, false);
                        }
                        ui.end_row();
                    }
                });
        });
}

fn render_block(ui: &mut egui::Ui, ctx: &mut Ctx, block: &Block, depth: usize) {
    let text = *theme::editor::TEXT;
    match &block.kind {
        BlockKind::Heading { level, inlines } => {
            let size = [28.0, 23.0, 19.0, 16.5, 15.0, 14.0][usize::from(*level).clamp(1, 6) - 1];
            let mut job = LayoutJob::default();
            push_inlines(
                &mut job,
                inlines,
                Fmt {
                    bold: true,
                    ..Fmt::default()
                },
                size,
                *theme::TEXT_BRIGHT,
            );
            ui.add(egui::Label::new(job).wrap());
            if *level <= 2 {
                ui.add_space(2.0);
                let (rect, _) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width(), 1.0),
                    egui::Sense::hover(),
                );
                ui.painter().rect_filled(rect, 0, *theme::BORDER);
            }
        }
        BlockKind::Paragraph(inlines) => paragraph(ui, ctx, inlines, BODY_SIZE, text),
        BlockKind::CodeBlock { lang, code } => code_block(ui, ctx, lang.as_deref(), code),
        BlockKind::BlockQuote(blocks) => {
            let top = ui.cursor().top();
            ui.horizontal_top(|ui| {
                ui.add_space(14.0);
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;
                    for inner in blocks {
                        render_block(ui, ctx, inner, depth + 1);
                    }
                });
            });
            let bottom = ui.min_rect().bottom();
            let left = ui.min_rect().left();
            ui.painter().rect_filled(
                egui::Rect::from_min_max(
                    egui::pos2(left + 2.0, top),
                    egui::pos2(left + 5.0, bottom),
                ),
                1,
                *theme::BORDER,
            );
        }
        BlockKind::List { start, items } => list(ui, ctx, *start, items),
        BlockKind::Table {
            align,
            header,
            rows,
        } => table(ui, block.lines.start + depth * 100_000, align, header, rows),
        BlockKind::Rule => {
            ui.add_space(4.0);
            let (rect, _) =
                ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), egui::Sense::hover());
            ui.painter().rect_filled(rect, 0, *theme::BORDER);
        }
        BlockKind::Html(html) => {
            ui.label(
                RichText::new(html.trim_end())
                    .monospace()
                    .size(12.0)
                    .color(*theme::TEXT_SECONDARY),
            );
        }
    }
}

/// Draws the preview of `document` in `rect`; `scroll_to` sets its vertical offset.
/// Returns its scroll offset after the frame and records block positions for syncing.
#[allow(clippy::too_many_arguments)] // inputs of one preview frame
pub fn preview(
    ui: &mut egui::Ui,
    state: &mut State,
    rect: egui::Rect,
    path: &Path,
    revision: u64,
    source: impl FnOnce() -> String,
    total_lines: usize,
    scroll_to: Option<f32>,
) -> PreviewOutput {
    egui_extras::install_image_loaders(ui.ctx());
    if state
        .parsed
        .as_ref()
        .is_none_or(|parsed| parsed.path != path || parsed.revision != revision)
    {
        state.parsed = Some(Parsed {
            path: path.to_owned(),
            revision,
            document: archipel_ide_markdown::parse(&source()),
        });
    }
    let State {
        parsed,
        highlights,
        map,
        ..
    } = state;
    let Some(parsed) = parsed.as_ref() else {
        return PreviewOutput { scroll_y: 0.0 };
    };
    ui.painter().rect_filled(
        rect,
        egui::CornerRadius {
            sw: theme::ISLAND_RADIUS,
            se: theme::ISLAND_RADIUS,
            ..Default::default()
        },
        *theme::editor::BACKGROUND,
    );
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect));
    let mut area = egui::ScrollArea::vertical()
        .id_salt("markdown_preview_scroll")
        .auto_shrink([false, false]);
    if let Some(y) = scroll_to {
        area = area.vertical_scroll_offset(y);
    }
    let base = path.parent().unwrap_or(Path::new("."));
    let mut measured: Vec<(Range<usize>, f32, f32)> = Vec::new();
    let mut content_height = 0.0;
    let output = area.show(&mut child, |ui| {
        let origin = ui.cursor().top();
        ui.add_space(PADDING);
        egui::Frame::new()
            .inner_margin(egui::Margin::symmetric(PADDING as i8, 0))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                let mut ctx = Ctx { base, highlights };
                for block in &parsed.document.blocks {
                    let top = ui.cursor().top() - origin;
                    render_block(ui, &mut ctx, block, 0);
                    let bottom = ui.min_rect().bottom().max(ui.cursor().top()) - origin;
                    measured.push((block.lines.clone(), top, bottom));
                    ui.add_space(BLOCK_GAP);
                }
            });
        ui.add_space(rect.height() * 0.5);
        content_height = ui.cursor().top() - origin;
    });
    *map = ScrollMap::new(&measured, total_lines, content_height);
    PreviewOutput {
        scroll_y: output.state.offset.y,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> State {
        // 10 lines over 300px of preview: 30px per line, editor rows are 20px.
        State {
            map: ScrollMap::new(&[(0..10, 0.0, 300.0)], 10, 300.0),
            ..State::default()
        }
    }

    #[test]
    fn editor_scrolling_drives_the_preview() {
        let mut state = state();
        assert!(state.sync(40.0, 20.0, 0.0, false, false));
        assert_eq!(state.take_preview_scroll(), Some(60.0));
        assert_eq!(state.take_editor_scroll(), None);
    }

    #[test]
    fn preview_scrolling_drives_the_editor_and_does_not_echo() {
        let mut state = state();
        assert!(state.sync(0.0, 20.0, 90.0, false, false));
        assert_eq!(state.take_editor_scroll(), Some(60.0));
        // Next frame the editor is at the target, and was set programmatically.
        assert!(!state.sync(60.0, 20.0, 90.0, true, false));
        assert!(!state.sync(60.0, 20.0, 90.0, false, false));
    }

    #[test]
    fn clamped_follower_does_not_bounce_back() {
        let mut state = state();
        assert!(state.sync(100.0, 20.0, 0.0, false, false));
        // The preview could only reach 50 of the requested 150: no feedback.
        assert!(!state.sync(100.0, 20.0, 50.0, false, true));
        assert!(!state.sync(100.0, 20.0, 50.0, false, false));
    }

    #[test]
    fn switching_files_resets_positions() {
        let mut state = state();
        state.reset_for(Path::new("a.md"));
        state.last_editor_y = 99.0;
        state.reset_for(Path::new("a.md"));
        assert_eq!(state.last_editor_y, 99.0);
        state.reset_for(Path::new("b.md"));
        assert_eq!(state.last_editor_y, 0.0);
    }

    #[test]
    fn local_images_resolve_next_to_the_document() {
        let dir = std::env::temp_dir().join(format!("archipel-ide-md-img-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("img")).unwrap();
        std::fs::write(dir.join("img/a.png"), b"x").unwrap();
        assert!(image_uri(&dir, "img/a.png?raw=1").is_some_and(|u| u.starts_with("file://")));
        assert_eq!(image_uri(&dir, "img/missing.png"), None);
        assert_eq!(image_uri(&dir, "https://x.dev/a.png"), None);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
