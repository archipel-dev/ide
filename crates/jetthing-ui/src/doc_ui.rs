//! Documentation popups: the panel beside the completion popup and the quick
//! documentation popup (Ctrl+Q, or resting the mouse over a symbol).
//!
//! Markdown from the language server is parsed with `pulldown-cmark` and laid
//! out with egui: the leading code block renders as a definition header on a
//! darker band, like the reference IDE's documentation popup.

use crate::{IdeApp, theme};
use archipel_kit::parts::Surface;
use eframe::egui::{self, Color32, FontId, Rect, RichText, Stroke, pos2, text::LayoutJob, vec2};
use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use serde_json::{Value, json};
use std::time::{Duration, Instant};

/// How long the pointer rests on a symbol before documentation is requested.
const HOVER_DELAY: Duration = Duration::from_millis(600);
const POPUP_WIDTH: f32 = 520.0;
const POPUP_MAX_HEIGHT: f32 = 360.0;

/// UI-side hover state.
#[derive(Default)]
pub struct DocView {
    /// Offset under the pointer and since when it rests there.
    resting: Option<(usize, egui::Pos2, Instant)>,
    /// Offset a mouse request was sent for.
    requested: Option<usize>,
    /// Where the mouse popup is anchored.
    anchor: Option<egui::Pos2>,
    /// Screen rect of the last popup, so moving onto it keeps it open.
    rect: Option<Rect>,
    /// The user clicked into the popup: it stays until Escape or a click outside.
    pinned: bool,
}

/// One rendered block.
enum Block {
    Text(LayoutJob),
    Code(String),
    Rule,
}

#[derive(Default, Clone, Copy)]
struct Style {
    bold: bool,
    italic: bool,
    code: bool,
    link: bool,
    strike: bool,
    heading: Option<HeadingLevel>,
}

fn format(style: Style) -> egui::TextFormat {
    let size = match style.heading {
        Some(HeadingLevel::H1) => theme::UI_FONT_SIZE + 4.0,
        Some(HeadingLevel::H2) => theme::UI_FONT_SIZE + 2.0,
        Some(_) => theme::UI_FONT_SIZE + 1.0,
        None => theme::UI_FONT_SIZE,
    };
    let font_id = if style.code {
        FontId::monospace(theme::EDITOR_FONT_SIZE - 0.5)
    } else if style.bold || style.heading.is_some() {
        theme::semibold(size)
    } else {
        FontId::proportional(size)
    };
    egui::TextFormat {
        font_id,
        color: if style.link {
            *theme::LINK
        } else if style.code {
            *theme::editor::TEXT
        } else {
            *theme::TEXT
        },
        italics: style.italic,
        background: if style.code {
            *theme::editor::BACKGROUND
        } else {
            Color32::TRANSPARENT
        },
        strikethrough: if style.strike {
            Stroke::new(1.0_f32, theme::TEXT)
        } else {
            Stroke::NONE
        },
        ..Default::default()
    }
}

/// Parses Markdown into renderable blocks.
fn blocks(markdown: &str) -> Vec<Block> {
    let mut out = Vec::new();
    let mut job = LayoutJob::default();
    let mut style = Style::default();
    let mut code: Option<String> = None;
    let mut lists: Vec<Option<u64>> = Vec::new();
    let flush = |job: &mut LayoutJob, out: &mut Vec<Block>| {
        if !job.text.trim().is_empty() {
            out.push(Block::Text(std::mem::take(job)));
        } else {
            *job = LayoutJob::default();
        }
    };
    let parser = Parser::new_ext(
        markdown,
        Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TABLES,
    );
    for event in parser {
        match event {
            Event::Start(Tag::CodeBlock(_)) => {
                flush(&mut job, &mut out);
                code = Some(String::new());
            }
            Event::End(TagEnd::CodeBlock) => {
                if let Some(text) = code.take() {
                    out.push(Block::Code(text.trim_end().to_owned()));
                }
            }
            Event::Text(text) if code.is_some() => {
                if let Some(code) = code.as_mut() {
                    code.push_str(&text);
                }
            }
            Event::Start(Tag::Heading { level, .. }) => {
                flush(&mut job, &mut out);
                style.heading = Some(level);
            }
            Event::End(TagEnd::Heading(_)) => {
                style.heading = None;
                flush(&mut job, &mut out);
            }
            Event::Start(Tag::Paragraph) => {}
            Event::End(TagEnd::Paragraph) => {
                if lists.is_empty() {
                    flush(&mut job, &mut out);
                }
            }
            Event::Start(Tag::List(start)) => {
                flush(&mut job, &mut out);
                lists.push(start);
            }
            Event::End(TagEnd::List(_)) => {
                lists.pop();
                flush(&mut job, &mut out);
            }
            Event::Start(Tag::Item) => {
                flush(&mut job, &mut out);
                let depth = lists.len().saturating_sub(1);
                let marker = match lists.last_mut() {
                    Some(Some(number)) => {
                        let marker = format!("{number}. ");
                        *number += 1;
                        marker
                    }
                    _ => "• ".to_owned(),
                };
                job.append(
                    &format!("{}{marker}", "    ".repeat(depth)),
                    0.0,
                    format(Style::default()),
                );
            }
            Event::End(TagEnd::Item) => flush(&mut job, &mut out),
            Event::Start(Tag::Emphasis) => style.italic = true,
            Event::End(TagEnd::Emphasis) => style.italic = false,
            Event::Start(Tag::Strong) => style.bold = true,
            Event::End(TagEnd::Strong) => style.bold = false,
            Event::Start(Tag::Strikethrough) => style.strike = true,
            Event::End(TagEnd::Strikethrough) => style.strike = false,
            Event::Start(Tag::Link { .. }) => style.link = true,
            Event::End(TagEnd::Link) => style.link = false,
            Event::Code(text) => {
                let mut code_style = style;
                code_style.code = true;
                job.append(&text, 0.0, format(code_style));
            }
            Event::Text(text) => job.append(&text, 0.0, format(style)),
            Event::SoftBreak => job.append(" ", 0.0, format(style)),
            Event::HardBreak => job.append("\n", 0.0, format(style)),
            Event::Rule => {
                flush(&mut job, &mut out);
                out.push(Block::Rule);
            }
            Event::Html(html) | Event::InlineHtml(html) => {
                // Servers sometimes embed simple HTML; show its text.
                let text = strip_tags(&html);
                if !text.is_empty() {
                    job.append(&text, 0.0, format(style));
                }
            }
            Event::End(TagEnd::TableCell) => job.append("  ", 0.0, format(style)),
            Event::End(TagEnd::TableRow | TagEnd::TableHead) => flush(&mut job, &mut out),
            _ => {}
        }
    }
    flush(&mut job, &mut out);
    out
}

fn strip_tags(html: &str) -> String {
    let mut out = String::new();
    let mut inside = false;
    for c in html.chars() {
        match c {
            '<' => inside = true,
            '>' => inside = false,
            _ if !inside => out.push(c),
            _ => {}
        }
    }
    out.replace("&nbsp;", " ")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

/// Renders Markdown into `ui`, wrapping at the current width.
pub fn render_markdown(ui: &mut egui::Ui, markdown: &str) {
    let width = ui.available_width();
    ui.spacing_mut().item_spacing.y = 6.0;
    for (index, block) in blocks(markdown).into_iter().enumerate() {
        match block {
            Block::Text(mut job) => {
                job.wrap.max_width = width;
                ui.label(job);
            }
            Block::Code(code) => {
                // The leading code block is the definition header.
                let fill = if index == 0 {
                    theme::editor::BACKGROUND_PANEL
                } else {
                    theme::editor::BACKGROUND
                };
                egui::Frame::new()
                    .fill(*fill)
                    .corner_radius(4)
                    .inner_margin(egui::Margin::symmetric(8, 6))
                    .show(ui, |ui| {
                        ui.set_width(width - 16.0);
                        let mut job = LayoutJob::single_section(
                            code,
                            egui::TextFormat {
                                font_id: FontId::monospace(theme::EDITOR_FONT_SIZE),
                                color: *theme::editor::TEXT,
                                ..Default::default()
                            },
                        );
                        job.wrap.max_width = width - 16.0;
                        ui.label(job);
                    });
            }
            Block::Rule => {
                ui.separator();
            }
        }
    }
}

fn popup_frame(ctx: &egui::Context) -> egui::Frame {
    Surface::Popup
        .frame(ctx)
        .inner_margin(egui::Margin::same(12))
}

/// Shows a documentation popup with its top-left at `pos`, kept on screen.
fn doc_popup(ctx: &egui::Context, id: &str, pos: egui::Pos2, markdown: &str) -> Rect {
    let screen = ctx.content_rect();
    let x = pos
        .x
        .min(screen.right() - POPUP_WIDTH - 8.0)
        .max(screen.left() + 4.0);
    let y = pos.y.min(screen.bottom() - 80.0);
    // Keyed by content: an Area remembers its size, which would clip a
    // longer document shown in the same place.
    egui::Area::new(egui::Id::new(id).with(markdown))
        .order(egui::Order::Foreground)
        .sense(egui::Sense::hover())
        .fixed_pos(pos2(x, y))
        .show(ctx, |ui| {
            popup_frame(ctx).show(ui, |ui| {
                ui.set_width(POPUP_WIDTH - 24.0);
                egui::ScrollArea::vertical()
                    .max_height(POPUP_MAX_HEIGHT)
                    .auto_shrink([false, true])
                    .show(ui, |ui| render_markdown(ui, markdown));
            });
        })
        .response
        .rect
}

/// The panel beside the completion popup (enabled with Ctrl+Q).
pub fn show_completion_panel(app: &mut IdeApp, ctx: &egui::Context, lookup: Option<Rect>) {
    let Some(lookup) = lookup else {
        return;
    };
    let Some(markdown) = app.state.completion_documentation() else {
        if app.state.docs.completion_panel && app.state.completion.is_some() {
            // Resolving: keep the panel's place without flicker.
            egui::Area::new(egui::Id::new("completion_docs"))
                .order(egui::Order::Foreground)
                .sense(egui::Sense::hover())
                .fixed_pos(pos2(lookup.right() + 4.0, lookup.top()))
                .show(ctx, |ui| {
                    popup_frame(ctx).show(ui, |ui| {
                        ui.label(RichText::new("No documentation").color(theme::TEXT_INFO));
                    });
                });
        }
        return;
    };
    let screen = ctx.content_rect();
    let pos = if lookup.right() + POPUP_WIDTH + 8.0 < screen.right() {
        pos2(lookup.right() + 4.0, lookup.top())
    } else {
        pos2(lookup.left(), lookup.bottom() + 4.0)
    };
    doc_popup(ctx, "completion_docs", pos, &markdown);
}

/// The quick documentation popup.
pub fn show_quick_doc(app: &mut IdeApp, ctx: &egui::Context) {
    let Some(popup) = app.state.docs.popup.clone() else {
        app.doc_view.rect = None;
        return;
    };
    let anchor = if popup.from_mouse {
        app.doc_view.anchor
    } else {
        app.caret_screen_pos
    };
    let Some(anchor) = anchor else {
        // Nowhere to show it: a popup nobody can see must not hold Escape.
        app.dispatch("language.closeQuickDoc", Value::Null);
        app.doc_view.pinned = false;
        return;
    };
    let rect = doc_popup(ctx, "quick_doc", anchor + vec2(-8.0, 4.0), &popup.markdown);
    app.doc_view.rect = Some(rect);
    let pressed = ctx.input(|input| input.pointer.primary_pressed())
        && ctx
            .input(|input| input.pointer.interact_pos())
            .is_some_and(|pos| rect.contains(pos));
    if pressed {
        // Clicking into the documentation focuses it (to select and copy text).
        app.doc_view.pinned = true;
    }
}

/// True when `offset` lies on the identifier that `requested` is part of.
fn on_same_identifier(app: &IdeApp, requested: usize, offset: usize) -> bool {
    if requested == offset {
        return true;
    }
    let Some(document) = app
        .state
        .active_document
        .and_then(|index| app.state.documents.get(index))
    else {
        return false;
    };
    let (start, end) = jetthing_text::ops::word_at(&document.text, requested);
    (start..end).contains(&offset)
}

/// Parameter info above the caret line: every overload on its own line, the
/// active one bright with its current parameter in bold.
pub fn show_parameter_info(app: &mut IdeApp, ctx: &egui::Context) {
    let Some(info) = app.state.signature.popup.as_ref() else {
        return;
    };
    let Some(caret) = app.caret_screen_pos else {
        return;
    };
    let jobs: Vec<LayoutJob> = info.signatures.iter().map(signature_job).collect();
    // `caret_screen_pos` is the bottom of the caret line. The popup sits above
    // that line, or below the caret when the overloads do not fit above.
    let line_top = caret.y - 20.0;
    let height = jobs.len() as f32 * (theme::EDITOR_FONT_SIZE + 6.0) + 14.0;
    let screen = ctx.content_rect();
    let (pivot, y) = if line_top - height > screen.top() + 48.0 {
        (egui::Align2::LEFT_BOTTOM, line_top - 2.0)
    } else {
        (egui::Align2::LEFT_TOP, caret.y + 2.0)
    };
    egui::Area::new(egui::Id::new("parameter_info"))
        .order(egui::Order::Foreground)
        .sense(egui::Sense::hover())
        .pivot(pivot)
        .fixed_pos(pos2(caret.x - 12.0, y))
        .show(ctx, |ui| {
            popup_frame(ctx)
                .inner_margin(egui::Margin::symmetric(10, 6))
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 2.0;
                    for job in jobs {
                        ui.add(egui::Label::new(job).wrap_mode(egui::TextWrapMode::Extend));
                    }
                });
        });
}

fn signature_job(line: &jetthing_core::SignatureLine) -> LayoutJob {
    let base = if line.current {
        theme::TEXT_BRIGHT
    } else {
        theme::TEXT_INFO
    };
    let mut job = LayoutJob::default();
    let normal = egui::TextFormat {
        font_id: FontId::monospace(theme::EDITOR_FONT_SIZE),
        color: *base,
        ..Default::default()
    };
    let bold = egui::TextFormat {
        font_id: theme::mono_bold(theme::EDITOR_FONT_SIZE),
        color: if line.current { *theme::LINK } else { *base },
        ..Default::default()
    };
    let chars: Vec<char> = line.label.chars().collect();
    let (start, end) = line.active.unwrap_or((chars.len(), chars.len()));
    let start = start.min(chars.len());
    let end = end.clamp(start, chars.len());
    let piece = |range: std::ops::Range<usize>| chars[range].iter().collect::<String>();
    job.append(&piece(0..start), 0.0, normal.clone());
    job.append(&piece(start..end), 0.0, bold);
    job.append(&piece(end..chars.len()), 0.0, normal);
    job
}

/// Requests documentation when the pointer rests on a symbol, and closes a
/// mouse popup when the pointer moves away from both the symbol and the popup.
pub fn track_hover(app: &mut IdeApp, ctx: &egui::Context, hover: Option<(usize, egui::Pos2)>) {
    let pointer = ctx.input(|input| input.pointer.hover_pos());
    let over_popup = pointer
        .zip(app.doc_view.rect)
        .is_some_and(|(pointer, rect)| rect.expand(8.0).contains(pointer));
    let busy = app.state.completion.is_some()
        || app.palette.open
        || app.find_popup.open
        || app.dialog.is_some()
        || ctx.input(|input| input.pointer.any_down());
    let pressed_outside = ctx.input(|input| input.pointer.primary_pressed())
        && pointer.is_some_and(|pointer| !app.doc_view.rect.is_some_and(|r| r.contains(pointer)));
    match app.state.docs.popup.as_ref().map(|popup| popup.from_mouse) {
        Some(true) => {
            let on_symbol = hover
                .zip(app.doc_view.requested)
                .is_some_and(|((offset, _), requested)| on_same_identifier(app, requested, offset));
            if pressed_outside {
                app.doc_view.pinned = false;
            }
            // One hover popup at a time: it closes when the pointer leaves both the
            // identifier and the popup, unless the user clicked into it.
            if !over_popup && !on_symbol && !app.doc_view.pinned {
                app.dispatch("language.closeQuickDoc", Value::Null);
                app.doc_view.requested = None;
            }
        }
        _ => app.doc_view.pinned = false,
    }
    let Some((offset, pos)) = hover.filter(|_| !busy && !over_popup && !app.doc_view.pinned) else {
        app.doc_view.resting = None;
        return;
    };
    match app.doc_view.resting {
        Some((resting, _, since)) if resting == offset => {
            let ready = since.elapsed() >= HOVER_DELAY;
            let shown = app.doc_view.requested == Some(offset);
            if ready && !shown && app.state.active_has_language_server() {
                app.doc_view.requested = Some(offset);
                app.doc_view.anchor = Some(pos);
                // Missing servers and empty hovers are silent for the mouse.
                let _ = app.commands.execute(
                    "language.hoverAt",
                    &mut app.state,
                    json!({"offset": offset}),
                );
            } else if !ready {
                ctx.request_repaint_after(HOVER_DELAY);
            }
        }
        _ => {
            app.doc_view.resting = Some((offset, pos, Instant::now()));
            ctx.request_repaint_after(HOVER_DELAY);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app_with_mouse_popup() -> IdeApp {
        let mut app = IdeApp::new(None, Vec::new());
        app.state.docs.popup = Some(jetthing_core::QuickDoc {
            path: "A.java".into(),
            offset: 3,
            markdown: "docs".into(),
            from_mouse: true,
        });
        app.doc_view.requested = Some(3);
        app
    }

    #[test]
    fn mouse_popup_closes_when_the_pointer_leaves_the_symbol() {
        let mut app = app_with_mouse_popup();
        track_hover(&mut app, &egui::Context::default(), None);
        assert!(app.state.docs.popup.is_none());
    }

    #[test]
    fn a_pinned_mouse_popup_stays_until_a_click_outside() {
        let mut app = app_with_mouse_popup();
        app.doc_view.pinned = true;
        let ctx = egui::Context::default();
        track_hover(&mut app, &ctx, None);
        assert!(app.state.docs.popup.is_some(), "clicked into: stays");
        app.doc_view.rect = Some(Rect::from_min_size(pos2(10.0, 10.0), vec2(50.0, 50.0)));
        ctx.begin_pass(egui::RawInput {
            events: vec![
                egui::Event::PointerMoved(pos2(400.0, 400.0)),
                egui::Event::PointerButton {
                    pos: pos2(400.0, 400.0),
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
            ..Default::default()
        });
        track_hover(&mut app, &ctx, None);
        let _ = ctx.end_pass();
        assert!(
            app.state.docs.popup.is_none(),
            "a press outside unpins and closes"
        );
    }

    #[test]
    fn markdown_splits_into_definition_text_and_lists() {
        let blocks = blocks(
            "```java\nString greeting(String name)\n```\nGreets **name** with `Hello`.\n\n- one\n- two\n\n---\n# Title",
        );
        let kinds: Vec<&str> = blocks
            .iter()
            .map(|block| match block {
                Block::Text(_) => "text",
                Block::Code(_) => "code",
                Block::Rule => "rule",
            })
            .collect();
        assert_eq!(kinds, ["code", "text", "text", "text", "rule", "text"]);
        let Block::Code(code) = &blocks[0] else {
            panic!("expected code");
        };
        assert_eq!(code, "String greeting(String name)");
        let Block::Text(text) = &blocks[1] else {
            panic!("expected text");
        };
        assert_eq!(text.text, "Greets name with Hello.");
        let Block::Text(item) = &blocks[2] else {
            panic!("expected list item");
        };
        assert_eq!(item.text, "• one");
    }

    #[test]
    fn signature_lines_bold_the_active_parameter() {
        let job = signature_job(&jetthing_core::SignatureLine {
            label: "greeting(String name, int times)".into(),
            active: Some((22, 31)),
            current: true,
        });
        assert_eq!(job.text, "greeting(String name, int times)");
        let bold: Vec<&str> = job
            .sections
            .iter()
            .filter(|section| section.format.font_id == theme::mono_bold(theme::EDITOR_FONT_SIZE))
            .map(|section| &job.text[section.byte_range.clone()])
            .collect();
        assert_eq!(bold, ["int times"]);
        // Out-of-range offsets from a server cannot panic.
        let odd = signature_job(&jetthing_core::SignatureLine {
            label: "f()".into(),
            active: Some((5, 9)),
            current: false,
        });
        assert_eq!(odd.text, "f()");
    }

    #[test]
    fn html_in_documentation_is_reduced_to_text() {
        assert_eq!(strip_tags("<b>a</b>&nbsp;&lt;T&gt;"), "a <T>");
        let blocks = blocks("Returns <code>true</code> if empty.");
        let Block::Text(text) = &blocks[0] else {
            panic!("expected text");
        };
        assert_eq!(text.text, "Returns true if empty.");
    }
}
