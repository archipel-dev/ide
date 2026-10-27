//! Editor island: tabs and the viewport-native text surface.

use crate::editor_keys;
use crate::folding::{CollapsedRow, FoldView};
use crate::icons::{self, Icon};
use crate::markdown_ui;
use crate::problems_ui;
use crate::widgets::island_frame;
use crate::{IdeApp, theme};
use archipel_ide_core::Document;
use archipel_ide_language::LanguageAnalysis;
use archipel_ide_text::Selection;
use archipel_ide_text::ops;
use eframe::egui::{self, Color32, Rect, Sense, Stroke, vec2};
use serde_json::{Value, json};

const TAB_BAR_HEIGHT: f32 = 38.0;
pub(crate) const ROW_HEIGHT: f32 = 20.0;
const TEXT_LEFT_PADDING: f32 = 14.0;

/// Work the editor surface hands back to the app: named actions go through the
/// command registry; clipboard requests need the platform clipboard.
pub enum EditorRequest {
    Command(&'static str, Value),
    Copy,
    Cut,
    Paste(String),
    /// An entry of the editor context menu (`EditorPopupMenu`).
    ContextAction(String),
    /// A gutter change marker was clicked: hunk index and screen position.
    ShowChange(usize, egui::Pos2),
    /// A blame annotation was clicked: commit hash.
    ShowCommit(String),
    /// Click on a gutter run icon (line, screen position for the popup).
    Gutter(usize, egui::Pos2),
    /// Right-click on a line number: breakpoint properties at line, popup position.
    BreakpointProperties(usize, egui::Pos2),
    /// A code-vision lens was clicked: Find Usages of the name at line, column.
    Usages(usize, usize),
}

pub struct EditorOutput {
    pub changed: bool,
    pub caret_pos: Option<egui::Pos2>,
    /// Where the lightbulb goes: left of the caret line's text, if visible.
    pub bulb_anchor: Option<egui::Pos2>,
    pub requests: Vec<EditorRequest>,
    /// Text typed this frame (drives completion auto-popup and filtering).
    pub typed: String,
    /// Char offset and screen position under the resting mouse pointer.
    pub hover: Option<(usize, egui::Pos2)>,
    /// Vertical scroll offset after this frame (Markdown scroll sync).
    pub scroll_y: f32,
}

pub fn editor_id() -> egui::Id {
    egui::Id::new("archipel_editor_surface")
}

/// Interaction id of the surface being drawn: the focused editor group owns
/// `editor_id()` (keyboard focus, `request_focus` calls); the others get their
/// own so two surfaces never share an id.
fn surface_id(ctx: &egui::Context) -> egui::Id {
    match ctx.data(|data| data.get_temp::<Option<u32>>(egui::Id::new("jt_surface_group"))) {
        Some(Some(group)) => editor_id().with(group),
        _ => editor_id(),
    }
}

pub fn central(app: &mut IdeApp, ctx: &egui::Context) {
    let mut changed_document = None;
    let mut requests = Vec::new();
    let mut typed = String::new();
    let mut hover = None;
    let caret_before = app.state.active_caret();
    egui::CentralPanel::default()
        .frame(island_frame(ctx))
        .show(ctx, |ui| {
            let mut body = |app: &mut IdeApp, ui: &mut egui::Ui, strip: Option<&TabStrip>| {
                let primary = strip.is_none_or(|strip| strip.primary);
                if primary {
                    crate::agents_ui::back_banner(app, ui);
                }
                tabs(app, ui, strip);
                if app.state.diff_tab_active() {
                    crate::diff_ui::show(app, ui);
                    app.caret_screen_pos = None;
                    return;
                }
                if app.state.plugin_host_service().active_tab().is_some() {
                    crate::plugin_ui::active_tab_body(app, ui);
                    app.caret_screen_pos = None;
                    return;
                }
                if app.state.active_index().is_none() {
                    // Offer toolchain setup as soon as the project opens.
                    if primary {
                        crate::gradle_banner::show(app, ui);
                        crate::toolchain_banner::show(app, ui);
                    }
                    empty_state(ui);
                    app.caret_screen_pos = None;
                    return;
                }
                if primary {
                    crate::disk_banner::show(app, ui);
                    crate::gradle_banner::show(app, ui);
                    crate::toolchain_banner::show(app, ui);
                }
                if crate::merged_manifest_ui::applies(app) {
                    crate::merged_manifest_ui::tab_bar(app, ui);
                    if app.state.android_tooling().merged_manifest().open {
                        crate::merged_manifest_ui::body(app, ui);
                        app.caret_screen_pos = None;
                        return;
                    }
                }
                if crate::layout_editor_ui::applies(app) {
                    crate::layout_editor_ui::mode_bar(app, ui);
                    match app.state.android_tooling().layout().mode {
                        archipel_ide_core::LayoutMode::Design => {
                            crate::layout_editor_ui::design_body(app, ui);
                            app.caret_screen_pos = None;
                            return;
                        }
                        archipel_ide_core::LayoutMode::Split => {
                            crate::layout_editor_ui::split_panel(app, ui)
                        }
                        archipel_ide_core::LayoutMode::Code => {}
                    }
                }
                let md_layout = app.state.markdown_layout();
                let full = ui.available_rect_before_wrap();
                // The toggle lives in the editor's top-right toolbar: the tab strip row.
                let toolbar_right = egui::pos2(
                    full.right(),
                    ui.max_rect().top() + TAB_BAR_HEIGHT / 2.0 - 12.0,
                );
                if let Some(layout) = md_layout
                    && primary
                    && let Some(picked) =
                        markdown_ui::layout_island(ui.ctx(), toolbar_right, layout)
                {
                    requests.push(EditorRequest::Command(
                        "markdown.layout",
                        json!({"mode": picked.id()}),
                    ));
                }
                if md_layout == Some(archipel_ide_markdown::Layout::PreviewOnly) {
                    markdown_preview_only(app, ui, full);
                    app.caret_screen_pos = None;
                    return;
                }
                if app.find.open {
                    crate::findbar::show(app, ui);
                }
                let Some(index) = app.state.active_index() else {
                    return;
                };
                let problems = app.state.cached_active_problems();
                let mut highlights = if app.find.open {
                    app.find.matches(&app.state.documents()[index])
                } else {
                    Vec::new()
                };
                // Snippet tab stops are boxed like search results while active.
                if let Some(snippet) = &app.state.snippet() {
                    highlights.extend(snippet.ranges().filter(|(start, end)| start < end));
                    // The painter only visits the matches of visible lines: keep them sorted.
                    highlights.sort_unstable();
                }
                let full = ui.available_rect_before_wrap();
                let (available, preview_rect) =
                    if md_layout == Some(archipel_ide_markdown::Layout::Split) {
                        let mid = (full.left() + full.width() / 2.0).round();
                        (
                            full.with_max_x(mid),
                            Some(full.with_min_x(mid + theme::ISLAND_GAP)),
                        )
                    } else {
                        (full, None)
                    };
                let stripe_rect = Rect::from_min_max(
                    egui::pos2(
                        available.right() - problems_ui::STRIPE_WIDTH,
                        available.top(),
                    ),
                    available.max,
                );
                let text_rect = available.with_max_x(stripe_rect.left());
                ui.allocate_rect(available, Sense::hover());
                let line_count = app.state.documents()[index].text.len_lines();
                let row_height =
                    theme::editor_row_height(ui.ctx(), &theme::editor_font(ui.style()), ROW_HEIGHT);
                if let Some((line, column)) = problems_ui::error_stripe(
                    ui,
                    stripe_rect,
                    &problems[..],
                    line_count,
                    row_height,
                ) {
                    move_caret_to(app, line, column);
                    ui.memory_mut(|memory| memory.request_focus(editor_id()));
                }
                let navigable = app.state.active_has_language_server();
                let mut text_ui = ui.new_child(egui::UiBuilder::new().max_rect(text_rect));
                let path = app.state.documents()[index].path.clone();
                let breakpoints = app.state.breakpoint_lines(&path);
                let execution_line = app
                    .state
                    .execution_point()
                    .filter(|point| point.path == path)
                    .map(|point| point.line);
                // The menu model is only needed while the popup is open (or opening).
                let menu_wanted = app.editor_menu.is_none()
                    || ui.input(|input| input.pointer.secondary_clicked())
                    || egui::Popup::is_any_open(ui.ctx());
                if menu_wanted {
                    app.editor_menu =
                        Some(app.context_menu_model(&crate::context_menus::Target::Editor));
                }
                let context_model = app.editor_menu.take().expect("built above");
                let changes = app.state.document_changes(index);
                let blame = app.state.document_annotations(index);
                let (lenses, inlays) = app.state.code_vision(index);
                let inline_values = app.state.inline_values(&path);
                let gutter = run_icons(app, ui.ctx());
                let doc_path = app.state.absolute_path(&app.state.documents()[index].path);
                if md_layout.is_some() {
                    app.markdown_ui.reset_for(&doc_path);
                }
                let editor_scroll = app.markdown_ui.take_editor_scroll();
                let Some((doc, rename)) = app.state.document_for_editing(index) else {
                    return;
                };
                let rename =
                    rename.map(|session| (session.occurrences.as_slice(), session.name.as_str()));
                let surface = editor_surface(
                    &mut text_ui,
                    doc,
                    &problems[..],
                    &highlights,
                    rename,
                    &context_model,
                    GutterData {
                        changes: &changes,
                        blame: blame.as_deref(),
                        run: &gutter,
                        breakpoints: &breakpoints,
                        execution_line,
                        lenses: &lenses,
                        inlays: &inlays,
                        inline_values: &inline_values,
                    },
                    navigable,
                    editor_scroll,
                );
                app.editor_menu = Some(context_model);
                let counts = archipel_ide_core::ProblemCounts::of(&problems[..]);
                let widget_anchor = if md_layout == Some(archipel_ide_markdown::Layout::EditorOnly)
                {
                    text_rect.right_top() - vec2(MARKDOWN_ISLAND_WIDTH, 0.0)
                } else {
                    text_rect.right_top()
                };
                match problems_ui::inspection_widget(ui, widget_anchor, counts) {
                    Some(problems_ui::WidgetAction::Next) => {
                        requests.push(EditorRequest::Command("editor.nextProblem", Value::Null));
                    }
                    Some(problems_ui::WidgetAction::Previous) => {
                        requests.push(EditorRequest::Command(
                            "editor.previousProblem",
                            Value::Null,
                        ));
                    }
                    Some(problems_ui::WidgetAction::ShowProblems) => requests.push(
                        EditorRequest::Command("toolwindow.activate.problems", Value::Null),
                    ),
                    None => {}
                }
                let mut bulb = None;
                let mut editor_scroll_y = None;
                let doc = &mut app.state.documents_mut()[index];
                match surface {
                    Ok(EditorOutput {
                        changed,
                        caret_pos,
                        bulb_anchor,
                        requests: editor_requests,
                        typed: typed_text,
                        hover: hovered,
                        scroll_y,
                    }) => {
                        editor_scroll_y = Some(scroll_y);
                        hover = hovered;
                        app.caret_screen_pos = caret_pos;
                        bulb = bulb_anchor;
                        requests.extend(editor_requests);
                        typed = typed_text;
                        if changed {
                            let snapshot = doc.text.text();
                            doc.refresh_analysis(&snapshot);
                            changed_document = Some((
                                doc.path.clone(),
                                snapshot,
                                doc.text.revision().min(i32::MAX as u64) as i32,
                            ));
                        }
                    }
                    Err(error) => app.state.set_status(error),
                }
                if let Some(preview_rect) = preview_rect {
                    let doc = &app.state.documents()[index];
                    let (revision, lines) = (doc.text.revision(), doc.text.len_lines());
                    let absolute = app.state.absolute_path(&doc.path);
                    let pending = app.markdown_ui.take_preview_scroll();
                    let output = markdown_ui::preview(
                        ui,
                        &mut app.markdown_ui,
                        preview_rect,
                        &absolute,
                        revision,
                        || doc.text.text(),
                        lines,
                        pending,
                    );
                    if let Some(editor_y) = editor_scroll_y
                        && app.markdown_ui.sync(
                            editor_y,
                            row_height,
                            output.scroll_y,
                            editor_scroll.is_some(),
                            pending.is_some(),
                        )
                    {
                        ui.ctx().request_repaint();
                    }
                }
                let selection = app.state.documents()[index].selections.primary();
                if md_layout.is_some()
                    && primary
                    && !selection.is_caret()
                    && let Some(pos) = app.caret_screen_pos
                    && let Some((command, params)) =
                        markdown_ui::floating_bar(ui.ctx(), pos, text_rect)
                {
                    requests.push(EditorRequest::Command(command, params));
                }
                if primary {
                    crate::intentions_ui::lightbulb(app, ui, bulb);
                    crate::ai_inline::paint(app, ui);
                }
            };
            show_groups(app, ui, &mut body);
        });
    if let Some((path, text, version)) = changed_document {
        app.state.sync_document_lsp(&path, &text, version);
        app.state.snippet_after_change();
        app.state.completion_after_typing(&typed);
        app.state.signature_after_typing(&typed);
    } else if app.state.active_caret() != caret_before {
        app.state.snippet_after_caret_move();
        app.state.signature_after_caret_move();
        if app.state.completion().is_some() {
            app.state.refresh_completion();
        }
    }
    crate::doc_ui::track_hover(app, ctx, hover);
    for request in requests {
        handle_request(app, ctx, request);
    }
}

pub(crate) fn handle_request(app: &mut IdeApp, ctx: &egui::Context, request: EditorRequest) {
    match request {
        EditorRequest::Command(id, params) => app.dispatch(id, params),
        EditorRequest::ShowCommit(hash) => {
            app.dispatch("git.log.show", Value::Null);
            app.dispatch("git.log.selectCommit", json!({ "hash": hash }));
        }
        EditorRequest::ShowChange(hunk, position) => {
            app.change_popup = Some(crate::change_popup::ChangePopup::new(hunk, position));
        }
        EditorRequest::Copy | EditorRequest::Cut => {
            let id = if matches!(request, EditorRequest::Copy) {
                "editor.copy"
            } else {
                "editor.cut"
            };
            match app.commands.execute(id, &mut app.state, Value::Null) {
                Ok(result) => {
                    let text = result["text"].as_str().unwrap_or_default().to_owned();
                    let whole_line = result["wholeLine"].as_bool().unwrap_or(false);
                    app.clipboard_line = whole_line.then(|| text.clone());
                    ctx.copy_text(text);
                }
                Err(error) => app.state.set_status(error.to_string()),
            }
        }
        EditorRequest::ContextAction(action) => {
            app.run_context_action(&action, &crate::context_menus::Target::Editor, ctx)
        }
        EditorRequest::Gutter(line, position) => crate::gutter_popup::open(ctx, line, position),
        EditorRequest::BreakpointProperties(line, position) => {
            crate::debug_dialogs::anchor_breakpoint_popup(ctx, position);
            if let Some(index) = app.state.active_index() {
                let path = app.state.documents()[index].path.clone();
                app.dispatch("debug.editBreakpoint", json!({"path": path, "line": line}));
            }
        }
        EditorRequest::Usages(line, column) => {
            move_caret_to(app, line, column);
            app.dispatch("navigate.findUsages", Value::Null);
        }
        EditorRequest::Paste(text) => {
            let whole_line = app.clipboard_line.as_deref() == Some(text.as_str());
            app.dispatch(
                "editor.paste",
                json!({"text": text.replace("\r\n", "\n"), "wholeLine": whole_line}),
            );
        }
    }
}

/// Width reserved for the Markdown layout island next to the inspection widget.
const MARKDOWN_ISLAND_WIDTH: f32 = 190.0;

/// Preview-only layout: the rendered document fills the editor area.
fn markdown_preview_only(app: &mut IdeApp, ui: &mut egui::Ui, rect: Rect) {
    let Some(index) = app.state.active_index() else {
        return;
    };
    ui.allocate_rect(rect, Sense::hover());
    let doc = &app.state.documents()[index];
    let absolute = app.state.absolute_path(&doc.path);
    app.markdown_ui.reset_for(&absolute);
    let (revision, lines) = (doc.text.revision(), doc.text.len_lines());
    markdown_ui::preview(
        ui,
        &mut app.markdown_ui,
        rect,
        &absolute,
        revision,
        || doc.text.text(),
        lines,
        None,
    );
}

fn empty_state(ui: &mut egui::Ui) {
    let rect = ui.available_rect_before_wrap();
    let lines = [
        ("Search Everywhere", "Double ⇧"),
        ("Find Action", "⇧⌘A"),
        ("Project View", "⌘1"),
        ("Find in Files", "⇧⌘F"),
        ("Terminal", "⌥F12"),
    ];
    let mut y = rect.center().y - lines.len() as f32 * 14.0;
    for (label, shortcut) in lines {
        let painter = ui.painter();
        painter.text(
            egui::pos2(rect.center().x - 8.0, y),
            egui::Align2::RIGHT_CENTER,
            label,
            egui::FontId::proportional(theme::UI_FONT_SIZE),
            *theme::TEXT_SECONDARY,
        );
        painter.text(
            egui::pos2(rect.center().x + 8.0, y),
            egui::Align2::LEFT_CENTER,
            crate::keys::display(shortcut),
            egui::FontId::proportional(theme::UI_FONT_SIZE),
            *theme::MODIFIED,
        );
        y += 28.0;
    }
}

/// The tabs one editor group shows (all documents when the editor is not split).
struct TabStrip {
    indices: Vec<usize>,
    /// The focused group also carries the diff and plugin tabs.
    primary: bool,
}

const TAB_DRAG: &str = "jt_tab_drag";

fn tabs(app: &mut IdeApp, ui: &mut egui::Ui, strip: Option<&TabStrip>) {
    let (bar, _) =
        ui.allocate_exact_size(vec2(ui.available_width(), TAB_BAR_HEIGHT), Sense::hover());
    let mut x = bar.left() + 6.0;
    let mut activate = None;
    let mut close = None;
    let mut tab_action = None;
    let font = egui::FontId::proportional(theme::UI_FONT_SIZE);
    let indices: Vec<usize> = match strip {
        Some(strip) => strip.indices.clone(),
        None => (0..app.state.documents().len()).collect(),
    };
    let primary = strip.is_none_or(|strip| strip.primary);
    for index in indices {
        let doc = &app.state.documents()[index];
        let name = IdeApp::file_name(&doc.path);
        let preview = app.state.is_preview(index);
        let active = app.state.active_index() == Some(index)
            && !app.state.diff_active()
            && app.state.plugin_host_service().active_tab().is_none();
        let color = if active {
            *theme::TEXT_BRIGHT
        } else {
            *theme::TEXT_SECONDARY
        };
        let galley = if preview {
            // Preview tabs are italic until they become permanent.
            let mut job = egui::text::LayoutJob::default();
            job.append(
                &name,
                0.0,
                egui::TextFormat {
                    font_id: font.clone(),
                    color,
                    italics: true,
                    ..Default::default()
                },
            );
            ui.painter().layout_job(job)
        } else {
            ui.painter()
                .layout_no_wrap(name.clone(), font.clone(), color)
        };
        let width = 10.0 + 16.0 + 6.0 + galley.size().x + 6.0 + 16.0 + 8.0;
        let rect = Rect::from_min_size(egui::pos2(x, bar.top() + 5.0), vec2(width, 28.0));
        x += width + 2.0;
        if rect.left() > bar.right() {
            break;
        }
        let response = ui.interact(
            rect,
            egui::Id::new(("editor_tab", index)),
            Sense::click_and_drag(),
        );
        if response.drag_started() {
            ui.ctx()
                .data_mut(|data| data.insert_temp(egui::Id::new(TAB_DRAG), index));
        }
        let fill = if active {
            *theme::TAB_SELECTED
        } else if response.hovered() {
            *theme::HOVER
        } else {
            Color32::TRANSPARENT
        };
        ui.painter().rect_filled(rect, 6.0, fill);
        if active {
            ui.painter().rect_stroke(
                rect,
                6.0,
                Stroke::new(1.0_f32, theme::TAB_SELECTED_BORDER),
                egui::StrokeKind::Inside,
            );
        }
        let layout = TabLayout::new(rect, galley.size());
        icons::paint(
            ui.painter(),
            layout.icon,
            app.file_icon(&doc.path),
            *icons::ICON,
        );
        let text_rect = layout.text;
        ui.painter().galley(layout.text.min, galley, color);
        if app.state.problems().has_errors(&doc.path) {
            problems_ui::squiggle(
                ui.painter(),
                text_rect.left(),
                text_rect.right(),
                rect.center().y + 8.0,
                *theme::editor::ERROR_EFFECT,
            );
        }
        let close_rect = layout.close;
        let dirty = doc.text.is_dirty();
        let pinned = app.state.is_pinned(index);
        let pointer_on_close = response
            .hover_pos()
            .is_some_and(|pos| close_rect.expand(2.0).contains(pos));
        if pinned {
            // Pinned tabs show a pin in place of the close button; clicking it unpins.
            if pointer_on_close {
                ui.painter()
                    .rect_filled(close_rect.expand(2.0), 4.0, theme::HOVER);
            }
            icons::paint(
                ui.painter(),
                close_rect.shrink(1.0),
                Icon::Pin,
                theme::TEXT_SECONDARY,
            );
        } else if response.hovered() || active {
            if pointer_on_close {
                ui.painter()
                    .rect_filled(close_rect.expand(2.0), 4.0, theme::HOVER);
            }
            icons::paint(
                ui.painter(),
                close_rect.shrink(2.0),
                Icon::Close,
                *theme::TEXT_SECONDARY,
            );
        } else if dirty {
            ui.painter()
                .circle_filled(close_rect.center(), 3.0, theme::TEXT_SECONDARY);
        }
        if dirty && (response.hovered() || active) && !pointer_on_close {
            ui.painter()
                .circle_filled(layout.modified_dot, 2.5, theme::MODIFIED);
        }
        let response = response.on_hover_text(doc.path.display().to_string());
        response.context_menu(|ui| {
            let target = crate::context_menus::Target::Tab(index);
            let model = app.context_menu_model(&target);
            let mut chosen = None;
            crate::context_menus::show(ui, &model, &mut chosen);
            if let Some(action) = chosen {
                tab_action = Some((action, target));
            }
        });
        if response.clicked() {
            if pointer_on_close && pinned {
                tab_action = Some((
                    "PinActiveTabToggle".to_owned(),
                    crate::context_menus::Target::Tab(index),
                ));
            } else if pointer_on_close {
                close = Some(index);
            } else {
                activate = Some(index);
            }
        }
        if response.middle_clicked() {
            close = Some(index);
        }
    }
    if !primary {
        // Only the focused group shows the diff and plugin tabs.
    } else if let Some(tab) = app.state.diff().as_ref() {
        let title = crate::diff_ui::tab_title(tab);
        let tooltip = format!("Diff of {} against HEAD", tab.path.display());
        let (clicked, closed, end) =
            diff_tab(ui, bar, x, &title, &tooltip, app.state.diff_active());
        x = end;
        if closed {
            app.dispatch("diff.close", Value::Null);
        } else if clicked {
            app.dispatch("diff.activate", Value::Null);
        }
    }
    if primary {
        crate::plugin_ui::editor_tabs(app, ui, bar, x);
    }
    if let Some((action, target)) = tab_action {
        let ctx = ui.ctx().clone();
        app.run_context_action(&action, &target, &ctx);
    } else if let Some(index) = close {
        app.dispatch("editor.close", json!({"index": index}));
    } else if let Some(index) = activate {
        app.dispatch("editor.activate", json!({"index": index}));
        ui.memory_mut(|memory| memory.request_focus(editor_id()));
    }
}

fn model_rect(rect: Rect) -> archipel_ide_split::Rect {
    archipel_ide_split::Rect::new(rect.left(), rect.top(), rect.width(), rect.height())
}

fn from_model(rect: archipel_ide_split::Rect) -> Rect {
    Rect::from_min_size(egui::pos2(rect.x, rect.y), vec2(rect.w, rect.h))
}

/// Draws the editor groups of the split tree, each with its own tab strip, and
/// the dividers between them; `body` draws one group.
fn show_groups(
    app: &mut IdeApp,
    ui: &mut egui::Ui,
    body: &mut dyn FnMut(&mut IdeApp, &mut egui::Ui, Option<&TabStrip>),
) {
    app.state.sync_splits();
    let area = ui.available_rect_before_wrap();
    if !app.state.splits().is_split() {
        ui.ctx()
            .data_mut(|data| data.insert_temp(egui::Id::new("jt_surface_group"), None::<u32>));
        body(app, ui, None);
        tab_drag(app, ui.ctx(), &[(app.state.splits().focused_id(), area)]);
        return;
    }
    // A press inside another group focuses it before it is drawn.
    let pressed = ui
        .input(|input| input.pointer.primary_pressed())
        .then(|| ui.input(|input| input.pointer.interact_pos()))
        .flatten()
        .filter(|pos| ui.ctx().layer_id_at(*pos) == Some(ui.layer_id()));
    if let Some(pos) = pressed
        && let Some((group, _)) = app
            .state
            .splits
            .layout(model_rect(area))
            .into_iter()
            .find(|(_, rect)| rect.contains(pos.x, pos.y))
        && group != app.state.splits().focused_id()
    {
        app.dispatch("editor.focusGroup", json!({ "group": group }));
    }
    let focused = app.state.splits().focused_id();
    let layout = app.state.splits().layout(model_rect(area));
    let mut order = layout.clone();
    // The focused group is drawn last so the caret and typing state are its own.
    order.sort_by_key(|(group, _)| *group == focused);
    for (group, rect) in order {
        let rect = from_model(rect).shrink(1.0);
        let Some(info) = app.state.splits().group(group).cloned() else {
            continue;
        };
        let indices: Vec<usize> = app
            .state
            .documents
            .iter()
            .enumerate()
            .filter(|(_, doc)| info.tabs.contains(&doc.path))
            .map(|(index, _)| index)
            .collect();
        let active = info
            .active
            .as_ref()
            .and_then(|path| app.state.documents().iter().position(|d| &d.path == path));
        let primary = group == focused;
        let saved = (
            app.state.active_index(),
            app.state.diff_active(),
            app.state.plugin_host_service().active_tab(),
        );
        app.state.set_active_index(active);
        if !primary {
            app.state.set_diff_active(false);
            app.state.plugin_host_service().set_active_tab(None);
        }
        ui.ctx().data_mut(|data| {
            data.insert_temp(
                egui::Id::new("jt_surface_group"),
                (!primary).then_some(group),
            )
        });
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .id_salt(("editor_group", group))
                .max_rect(rect)
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );
        child.set_clip_rect(rect.intersect(ui.clip_rect()));
        body(app, &mut child, Some(&TabStrip { indices, primary }));
        if !primary {
            let (active, diff_active, plugin_tab) = saved;
            app.state.set_active_index(active);
            app.state.set_diff_active(diff_active);
            app.state.plugin_host_service().set_active_tab(plugin_tab);
        }
    }
    ui.ctx()
        .data_mut(|data| data.insert_temp(egui::Id::new("jt_surface_group"), None::<u32>));
    let rects: Vec<_> = layout
        .into_iter()
        .map(|(group, rect)| (group, from_model(rect)))
        .collect();
    dividers(app, ui, area);
    tab_drag(app, ui.ctx(), &rects);
}

/// Draggable dividers between split groups.
fn dividers(app: &mut IdeApp, ui: &mut egui::Ui, area: Rect) {
    use archipel_ide_split::Orientation;
    for (n, divider) in app
        .state
        .splits
        .dividers(model_rect(area))
        .into_iter()
        .enumerate()
    {
        let whole = from_model(divider.area);
        let (line, cursor) = match divider.orientation {
            Orientation::Columns => {
                let x = whole.left() + (whole.width() * divider.ratio).round();
                (
                    Rect::from_min_max(
                        egui::pos2(x - 2.0, whole.top()),
                        egui::pos2(x + 2.0, whole.bottom()),
                    ),
                    egui::CursorIcon::ResizeHorizontal,
                )
            }
            Orientation::Rows => {
                let y = whole.top() + (whole.height() * divider.ratio).round();
                (
                    Rect::from_min_max(
                        egui::pos2(whole.left(), y - 2.0),
                        egui::pos2(whole.right(), y + 2.0),
                    ),
                    egui::CursorIcon::ResizeVertical,
                )
            }
        };
        let response = ui
            .interact(line, egui::Id::new(("editor_divider", n)), Sense::drag())
            .on_hover_cursor(cursor);
        let stroke = Stroke::new(1.0_f32, theme::TAB_SELECTED_BORDER);
        match divider.orientation {
            Orientation::Columns => {
                ui.painter().vline(line.center().x, line.y_range(), stroke);
            }
            Orientation::Rows => {
                ui.painter().hline(line.x_range(), line.center().y, stroke);
            }
        }
        if response.dragged()
            && let Some(pos) = response.interact_pointer_pos()
        {
            let ratio = match divider.orientation {
                Orientation::Columns => (pos.x - whole.left()) / whole.width().max(1.0),
                Orientation::Rows => (pos.y - whole.top()) / whole.height().max(1.0),
            };
            app.dispatch(
                "editor.setSplitRatio",
                json!({"path": divider.path, "ratio": ratio}),
            );
        }
    }
}

/// Fraction of a group's width/height that turns a tab drop into a split.
const DROP_EDGE_FRACTION: f32 = 0.25;

/// A tab dragged over the editor: highlights the drop target (an edge splits,
/// another group's area moves the tab there) and applies it on release.
fn tab_drag(app: &mut IdeApp, ctx: &egui::Context, groups: &[(archipel_ide_split::GroupId, Rect)]) {
    let id = egui::Id::new(TAB_DRAG);
    let Some(index) = ctx.data(|data| data.get_temp::<usize>(id)) else {
        return;
    };
    let Some(path) = app.state.document(index).map(|d| d.path.clone()) else {
        ctx.data_mut(|data| data.remove::<usize>(id));
        return;
    };
    let source = app.state.splits().group_of(&path);
    let only_tab = source
        .and_then(|group| app.state.splits().group(group))
        .is_some_and(|group| group.tabs.len() == 1);
    let (down, pos) =
        ctx.input(|input| (input.pointer.primary_down(), input.pointer.interact_pos()));
    // What a drop at `pos` would do: split at an edge, or move to another group.
    let target = pos.and_then(|pos| {
        let (group, rect) = groups.iter().find(|(_, rect)| rect.contains(pos))?;
        let body = rect.with_min_y(rect.top() + TAB_BAR_HEIGHT);
        let edge =
            archipel_ide_split::drop_edge(model_rect(body), pos.x, pos.y, DROP_EDGE_FRACTION);
        match edge {
            Some(_) if only_tab && Some(*group) == source => None,
            Some(edge) => Some((*group, Some(edge), *rect, body)),
            None if Some(*group) != source => Some((*group, None, *rect, body)),
            None => None,
        }
    });
    if down {
        ctx.set_cursor_icon(egui::CursorIcon::Grabbing);
        if let Some((_, edge, _, body)) = target {
            let zone = match edge {
                Some(archipel_ide_split::Edge::Left) => body.with_max_x(body.center().x),
                Some(archipel_ide_split::Edge::Right) => body.with_min_x(body.center().x),
                Some(archipel_ide_split::Edge::Top) => body.with_max_y(body.center().y),
                Some(archipel_ide_split::Edge::Bottom) => body.with_min_y(body.center().y),
                None => body,
            };
            let painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Foreground,
                egui::Id::new("jt_tab_drop"),
            ));
            painter.rect_filled(zone, 4.0, theme::TAB_SELECTED_BORDER.gamma_multiply(0.35));
        }
        ctx.request_repaint();
        return;
    }
    ctx.data_mut(|data| data.remove::<usize>(id));
    let Some((group, edge, _, _)) = target else {
        return;
    };
    match edge {
        Some(edge) => {
            let edge = match edge {
                archipel_ide_split::Edge::Left => "left",
                archipel_ide_split::Edge::Right => "right",
                archipel_ide_split::Edge::Top => "top",
                archipel_ide_split::Edge::Bottom => "bottom",
            };
            app.dispatch(
                "editor.splitAt",
                json!({"index": index, "group": group, "edge": edge}),
            );
        }
        None => app.dispatch(
            "editor.moveToGroup",
            json!({"index": index, "group": group}),
        ),
    }
}

/// Geometry of one editor tab: file icon, title, close button, and the
/// modified badge shown while the close button is visible. The badge sits on
/// the icon's top-right corner so it never covers the title.
struct TabLayout {
    icon: Rect,
    text: Rect,
    close: Rect,
    modified_dot: egui::Pos2,
}

impl TabLayout {
    fn new(rect: Rect, text_size: egui::Vec2) -> Self {
        let icon = Rect::from_min_size(rect.left_center() + vec2(10.0, -8.0), vec2(16.0, 16.0));
        let text = Rect::from_min_size(
            rect.left_center() + vec2(32.0, -text_size.y / 2.0),
            text_size,
        );
        let close = Rect::from_min_size(
            egui::pos2(rect.right() - 24.0, rect.center().y - 8.0),
            vec2(16.0, 16.0),
        );
        Self {
            icon,
            text,
            close,
            modified_dot: icon.right_top() + vec2(-1.0, 2.0),
        }
    }
}

/// The diff viewer's tab, after the document tabs. Returns (clicked, closed, x after
/// the tab).
fn diff_tab(
    ui: &mut egui::Ui,
    bar: Rect,
    x: f32,
    title: &str,
    tooltip: &str,
    active: bool,
) -> (bool, bool, f32) {
    let font = egui::FontId::proportional(theme::UI_FONT_SIZE);
    let color = if active {
        *theme::TEXT_BRIGHT
    } else {
        *theme::TEXT_SECONDARY
    };
    let galley = ui.painter().layout_no_wrap(title.to_owned(), font, color);
    let width = 10.0 + 16.0 + 6.0 + galley.size().x + 6.0 + 16.0 + 8.0;
    let rect = Rect::from_min_size(egui::pos2(x, bar.top() + 5.0), vec2(width, 28.0));
    if rect.left() > bar.right() {
        return (false, false, rect.right() + 2.0);
    }
    let response = ui.interact(rect, egui::Id::new("editor_diff_tab"), Sense::click());
    let fill = if active {
        *theme::TAB_SELECTED
    } else if response.hovered() {
        *theme::HOVER
    } else {
        Color32::TRANSPARENT
    };
    ui.painter().rect_filled(rect, 6.0, fill);
    if active {
        ui.painter().rect_stroke(
            rect,
            6.0,
            Stroke::new(1.0_f32, theme::TAB_SELECTED_BORDER),
            egui::StrokeKind::Inside,
        );
    }
    icons::paint(
        ui.painter(),
        Rect::from_min_size(rect.left_center() + vec2(10.0, -8.0), vec2(16.0, 16.0)),
        Icon::Commit,
        *icons::ICON,
    );
    ui.painter().galley(
        rect.left_center() + vec2(32.0, -galley.size().y / 2.0),
        galley,
        color,
    );
    let close_rect = Rect::from_min_size(
        egui::pos2(rect.right() - 24.0, rect.center().y - 8.0),
        vec2(16.0, 16.0),
    );
    let on_close = response
        .hover_pos()
        .is_some_and(|pos| close_rect.expand(2.0).contains(pos));
    if response.hovered() || active {
        if on_close {
            ui.painter()
                .rect_filled(close_rect.expand(2.0), 4.0, theme::HOVER);
        }
        icons::paint(
            ui.painter(),
            close_rect.shrink(2.0),
            Icon::Close,
            *theme::TEXT_SECONDARY,
        );
    }
    let response = response.on_hover_text(tooltip);
    let clicked = response.clicked();
    (
        clicked && !on_close,
        (clicked && on_close) || response.middle_clicked(),
        rect.right() + 2.0,
    )
}

pub(crate) fn highlighted_line_job(
    text: &str,
    line_start_byte: usize,
    analysis: &LanguageAnalysis,
    font: &egui::FontId,
) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    let base = egui::TextFormat {
        font_id: font.clone(),
        color: *theme::editor::TEXT,
        ..Default::default()
    };
    let line_end_byte = line_start_byte + text.len();
    let mut cursor = line_start_byte;
    for span in analysis.highlights.in_range(line_start_byte..line_end_byte) {
        let (start, end) = (span.start_byte, span.end_byte);
        if start < cursor
            || end < start
            || !text.is_char_boundary(start - line_start_byte)
            || !text.is_char_boundary(end - line_start_byte)
        {
            continue;
        }
        job.append(
            &text[cursor - line_start_byte..start - line_start_byte],
            0.0,
            base.clone(),
        );
        let fragment = &text[start - line_start_byte..end - line_start_byte];
        job.append(fragment, 0.0, crate::syntax::text_format(span.key, &base));
        cursor = end;
    }
    job.append(&text[cursor - line_start_byte..], 0.0, base);
    job
}

/// Width of a parameter-name hint pill.
fn hint_width(text: &str, char_width: f32) -> f32 {
    text.chars().count() as f32 * char_width * 0.85 + 8.0
}

/// Character column (fractional) under `x`, measured from the text origin, on
/// a line with parameter hints `(column, width, text)` sorted by column.
fn column_at(hints: &[(usize, f32, &str)], x: f32, char_width: f32) -> f32 {
    let mut shift = 0.0;
    for &(column, width, _) in hints {
        let start = column as f32 * char_width + shift;
        if x < start {
            break;
        }
        if x < start + width {
            return column as f32;
        }
        shift += width;
    }
    (x - shift) / char_width
}

/// Paints a parameter-name hint: small grey text on a rounded pill.
fn paint_hint(
    painter: &egui::Painter,
    origin: egui::Pos2,
    width: f32,
    label: &str,
    font: &egui::FontId,
    row_height: f32,
) {
    painter.rect_filled(
        Rect::from_min_size(origin + vec2(1.0, 3.0), vec2(width - 2.0, row_height - 6.0)),
        3.0,
        *theme::editor::CARET_ROW,
    );
    painter.text(
        origin + vec2(width / 2.0, row_height / 2.0),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::new(font.size * 0.85, font.family.clone()),
        *theme::editor::LINE_NUMBER_CARET,
    );
}

/// Paints a collapsed region's placeholder box and the text after the region
/// on its last line, starting at `origin` (top-left of the placeholder).
fn paint_collapsed_tail(
    painter: &egui::Painter,
    document: &Document,
    row: &CollapsedRow,
    origin: egui::Pos2,
    char_width: f32,
    font: &egui::FontId,
    row_height: f32,
) {
    let folded = crate::syntax::attributes("FOLDED_TEXT_ATTRIBUTES");
    let width = row.placeholder.chars().count() as f32 * char_width;
    painter.rect_filled(
        Rect::from_min_size(origin + vec2(0.0, 3.0), vec2(width, row_height - 6.0)),
        3.0,
        folded.background.unwrap_or(*theme::editor::MATCHED_BRACE),
    );
    painter.text(
        origin + vec2(0.0, row_height / 2.0),
        egui::Align2::LEFT_CENTER,
        &row.placeholder,
        font.clone(),
        folded
            .foreground
            .unwrap_or(*theme::editor::LINE_NUMBER_CARET),
    );
    let end_line_start = document.text.line_to_char(row.end_line).unwrap_or(0);
    let end_text = document.text.line(row.end_line).unwrap_or_default();
    let end_content = end_text.trim_end_matches(['\r', '\n']);
    let skip = row.end_char.saturating_sub(end_line_start);
    let start = end_content
        .char_indices()
        .nth(skip)
        .map_or(end_content.len(), |(index, _)| index);
    let tail = &end_content[start..];
    if tail.is_empty() {
        return;
    }
    let tail_byte = document.text.line_to_byte(row.end_line).unwrap_or(0) + start;
    let job = highlighted_line_job(tail, tail_byte, &document.analysis, font);
    let galley = painter.layout_job(job);
    let text_y = origin.y + (row_height - galley.size().y) / 2.0;
    painter.galley(egui::pos2(origin.x + width, text_y), galley, Color32::WHITE);
}

/// Moves the primary caret to a zero-based line/column and focuses the editor.
pub fn move_caret_to(app: &mut IdeApp, line: usize, column: usize) {
    let Some(index) = app.state.active_index() else {
        return;
    };
    let doc = &mut app.state.documents_mut()[index];
    let line = line.min(doc.text.len_lines().saturating_sub(1));
    let line_text = doc.text.line(line).unwrap_or_default();
    let length = line_text.trim_end_matches(['\r', '\n']).chars().count();
    let offset = doc.text.line_to_char(line).unwrap_or(0) + column.min(length);
    doc.selections.set_single(Selection::caret(offset));
    app.state.clear_status();
}

fn line_length(document: &Document, line: usize) -> usize {
    document
        .text
        .line(line)
        .unwrap_or_default()
        .trim_end_matches(['\r', '\n'])
        .chars()
        .count()
}

/// Character offset for moving `head` vertically by `delta` visual rows
/// (collapsed regions count as one row), keeping the column.
fn vertical_target(document: &Document, folds: &FoldView, head: usize, delta: isize) -> usize {
    let Ok((line, column)) = document.text.char_to_line_column(head) else {
        return head;
    };
    let last_row = folds.row_count(document.text.len_lines()).saturating_sub(1) as isize;
    let row = (folds.line_to_row(line) as isize + delta).clamp(0, last_row) as usize;
    let target = folds.row_to_line(row);
    if target == line {
        return if delta < 0 {
            document.text.line_to_char(line).unwrap_or(0)
        } else {
            document.text.line_to_char(line).unwrap_or(0) + line_length(document, line)
        };
    }
    document.text.line_to_char(target).unwrap_or(0) + column.min(line_length(document, target))
}

fn move_primary(document: &mut Document, head: usize, extend: bool) {
    let current = document.selections.primary();
    let anchor = if extend { current.anchor } else { head };
    document.selections.set_single(Selection { anchor, head });
}

/// Horizontal layout of the editor gutter. Left to right: blame annotations,
/// line numbers (breakpoint dots and the execution arrow sit in their left
/// margin), run icons (when the file has any), folding markers, the
/// change bar, then the text.
#[derive(Debug, Clone, Copy, PartialEq)]
struct GutterLayout {
    /// Left edge of the line-number column (right of the blame annotations).
    numbers_left: f32,
    /// Center of the breakpoint dot in the line-number column.
    breakpoint_center: f32,
    /// Right edge of the right-aligned line numbers.
    numbers_right: f32,
    /// Center of the run-icon column (equal to `numbers_right` without icons).
    run_center: f32,
    run_width: f32,
    /// Center of the folding-marker column.
    fold_center: f32,
    /// Total gutter width; the change bar sits just inside it.
    width: f32,
}

impl GutterLayout {
    const FOLD_COLUMN: f32 = 14.0;
    const RUN_COLUMN: f32 = 16.0;
    /// Space for the change bar (`change_popup` paints it 7 px inside the edge).
    const CHANGE_COLUMN: f32 = 14.0;

    fn new(blame_width: f32, numbers_width: f32, run_icons: bool) -> Self {
        let numbers_right = blame_width + 20.0 + numbers_width;
        let run_width = if run_icons { Self::RUN_COLUMN } else { 0.0 };
        let run_left = numbers_right + 4.0;
        let fold_left = run_left + run_width;
        Self {
            numbers_left: blame_width,
            breakpoint_center: blame_width + 12.0,
            numbers_right,
            run_center: run_left + run_width / 2.0,
            run_width,
            fold_center: fold_left + Self::FOLD_COLUMN / 2.0,
            width: fold_left + Self::FOLD_COLUMN + Self::CHANGE_COLUMN,
        }
    }

    fn in_number_column(&self, x: f32) -> bool {
        x >= self.numbers_left && x < self.numbers_right + 2.0
    }

    fn in_run_column(&self, x: f32) -> bool {
        self.run_width > 0.0 && (x - self.run_center).abs() <= self.run_width / 2.0
    }

    fn in_fold_column(&self, x: f32) -> bool {
        (x - self.fold_center).abs() <= Self::FOLD_COLUMN / 2.0
    }
}

/// Corner rounding of one line's selection rectangle: a multi-line selection
/// reads as one shape, rounded only at the top of its first line and the bottom
/// of its last.
fn selection_corners(first_line: bool, last_line: bool) -> egui::CornerRadius {
    const RADIUS: u8 = 2;
    let top = if first_line { RADIUS } else { 0 };
    let bottom = if last_line { RADIUS } else { 0 };
    egui::CornerRadius {
        nw: top,
        ne: top,
        sw: bottom,
        se: bottom,
    }
}

/// Run icons are recomputed this long after the latest edit; until then the
/// previous icons stay (finding runnables walks the whole syntax tree, too slow
/// for every keystroke in large files).
const RUN_ICONS_STALE_FOR: f64 = 0.25;

/// Gutter run icons of the active editor, refreshed once typing pauses.
fn run_icons(app: &mut IdeApp, ctx: &egui::Context) -> Vec<archipel_ide_core::GutterMark> {
    let id = egui::Id::new("run_icons_last_edit");
    let revision = app
        .state
        .active_doc()
        .map(|document| document.text.revision());
    let now = ctx.input(|input| input.time);
    let (seen, last_edit) = ctx
        .data(|data| data.get_temp::<(Option<u64>, f64)>(id))
        .unwrap_or((None, f64::NEG_INFINITY));
    let last_edit = if seen != revision { now } else { last_edit };
    ctx.data_mut(|data| data.insert_temp(id, (revision, last_edit)));
    if now - last_edit < RUN_ICONS_STALE_FOR
        && let Some(marks) = app.state.cached_gutter_marks()
    {
        ctx.request_repaint_after(std::time::Duration::from_secs_f64(RUN_ICONS_STALE_FOR));
        return marks;
    }
    app.state.gutter_marks()
}

/// What the gutter shows next to the line numbers.
struct GutterData<'a> {
    /// Change markers against `HEAD`.
    changes: &'a [archipel_ide_git::Hunk],
    /// Blame annotations, when shown.
    blame: Option<&'a [archipel_ide_git::BlameLine]>,
    /// Run icons (mains, tests, Bazel rules).
    run: &'a [archipel_ide_core::GutterMark],
    /// Zero-based lines with breakpoints.
    breakpoints: &'a [usize],
    /// Zero-based line where the debugger is suspended.
    execution_line: Option<usize>,
    /// Usages / author lenses shown after declarations.
    lenses: &'a [archipel_ide_core::CodeLens],
    /// Parameter-name hints shown inside lines.
    inlays: &'a [archipel_ide_core::InlayLabel],
    /// End-of-line variable values while the debugger is suspended in this file.
    inline_values: &'a [archipel_ide_core::InlineValue],
}

#[allow(clippy::too_many_arguments)] // paint inputs of one editor frame
fn editor_surface(
    ui: &mut egui::Ui,
    document: &mut Document,
    problems: &[archipel_ide_core::Problem],
    highlights: &[(usize, usize)],
    rename: Option<(&[(usize, usize)], &str)>,
    context_model: &crate::context_menus::Model,
    gutter_data: GutterData<'_>,
    navigable: bool,
    scroll_to: Option<f32>,
) -> Result<EditorOutput, String> {
    let GutterData {
        changes,
        blame,
        run: gutter,
        breakpoints,
        execution_line,
        lenses,
        inlays,
        inline_values,
    } = gutter_data;
    let mut requests = Vec::new();
    let font = theme::editor_font(ui.style());
    // Rows scale with Settings | Editor | Font (size and line height).
    let row_height = theme::editor_row_height(ui.ctx(), &font, ROW_HEIGHT);
    // The color scheme's background (equal to the island in the theme's own scheme).
    ui.painter().rect_filled(
        ui.available_rect_before_wrap(),
        egui::CornerRadius {
            sw: theme::ISLAND_RADIUS,
            se: theme::ISLAND_RADIUS,
            ..Default::default()
        },
        theme::editor::BACKGROUND,
    );
    let char_width = ui.fonts_mut(|fonts| fonts.glyph_width(&font, 'm'));
    let line_count = document.text.len_lines();
    // Carets never stay inside collapsed text (Go to Line, Find, arrows).
    document.reveal_carets();
    let folds = FoldView::new(document);
    let lens_by_line: std::collections::HashMap<usize, &archipel_ide_core::CodeLens> =
        lenses.iter().map(|lens| (lens.line, lens)).collect();
    let row_count = folds.row_count(line_count);
    // Highlights are computed lazily: make sure last frame's visible rows plus
    // a page of margin are highlighted before painting.
    let viewport_id = egui::Id::new(("editor_viewport_rows", &document.path));
    let (seen_first, seen_last) = ui
        .data(|data| data.get_temp::<(usize, usize)>(viewport_id))
        .unwrap_or((0, 80));
    if row_count > 0 {
        let first_line = folds.row_to_line(seen_first.saturating_sub(60).min(row_count - 1));
        let last_line = folds.row_to_line((seen_last + 60).min(row_count - 1));
        let start = document.text.line_to_byte(first_line).unwrap_or(0);
        let end = document
            .text
            .line_to_byte(last_line + 1)
            .unwrap_or_else(|| document.text.char_to_byte(document.text.len_chars()));
        document.analysis.ensure_highlighted(start..end);
    }
    let digits = line_count.max(1).to_string().len().max(2);
    let blame_width = if blame.is_some() {
        crate::blame_gutter::WIDTH
    } else {
        0.0
    };
    let layout = GutterLayout::new(blame_width, digits as f32 * char_width, !gutter.is_empty());
    let gutter_width = layout.width;
    let text_left = gutter_width + TEXT_LEFT_PADDING;
    // Parameter-name hints take room inside their line: (column, width, text).
    let mut hints_by_line: std::collections::HashMap<usize, Vec<(usize, f32, &str)>> =
        std::collections::HashMap::new();
    for inlay in inlays {
        let width = hint_width(&inlay.text, char_width);
        hints_by_line.entry(inlay.line).or_default().push((
            inlay.column,
            width,
            inlay.text.as_str(),
        ));
    }
    for hints in hints_by_line.values_mut() {
        hints.sort_by_key(|hint| hint.0);
    }
    // Horizontal offset of `column` on `line` (before the glyph).
    let shifted = |line: usize, column: usize| -> f32 {
        let hints = hints_by_line.get(&line).map_or(&[][..], Vec::as_slice);
        column as f32 * char_width
            + hints
                .iter()
                .filter(|hint| hint.0 <= column)
                .map(|hint| hint.1)
                .sum::<f32>()
    };
    let caret_line = document
        .text
        .char_to_line_column(document.selections.primary().head)
        .map(|(line, _)| line)
        .unwrap_or(0);
    let mut caret_pos = None;
    let mut bulb_anchor = None;
    let mut hover = None;
    let brace_pair: Vec<usize> = {
        let head = document.selections.primary().head;
        // Structural matching from the syntax tree when the language has a
        // grammar (brackets in strings/comments never match); text otherwise.
        let find = |offset: usize| {
            if document.analysis.syntax.is_some() {
                let byte = document.text.char_to_byte(offset);
                document
                    .analysis
                    .matching_bracket(byte)
                    .map(|(a, b)| (document.text.byte_to_char(a), document.text.byte_to_char(b)))
            } else {
                ops::matching_bracket(&document.text, offset)
            }
        };
        find(head)
            .or_else(|| head.checked_sub(1).and_then(find))
            .into_iter()
            .flat_map(|(a, b)| [a, b])
            .collect()
    };
    let mut scroll_area = egui::ScrollArea::both().auto_shrink([false, false]);
    if let Some(y) = scroll_to {
        scroll_area = scroll_area.vertical_scroll_offset(y);
    }
    let output = scroll_area.show_viewport(ui, |ui, viewport| {
        let longest = 400.0_f32;
        let content_size = egui::vec2(
            ui.available_width().max(text_left + longest * char_width),
            row_count as f32 * row_height + ui.available_height() * 0.5,
        );
        let (rect, response) = ui.allocate_exact_size(content_size, Sense::click_and_drag());
        let response = ui
            .interact(rect, surface_id(ui.ctx()), Sense::click_and_drag())
            .union(response);
        if response.clicked() || response.drag_started() {
            response.request_focus();
        }
        let row_at = |position: egui::Pos2| {
            (((position.y - rect.top()) / row_height).floor().max(0.0) as usize)
                .min(row_count.saturating_sub(1))
        };
        let position_to_offset = |position: egui::Pos2| {
            let line = folds.row_to_line(row_at(position));
            let line_text = document.text.line(line).unwrap_or_default();
            let content = line_text.trim_end_matches(['\r', '\n']);
            let column = (column_at(
                hints_by_line.get(&line).map_or(&[][..], Vec::as_slice),
                position.x - rect.left() - text_left,
                char_width,
            )
            .round()
            .max(0.0) as usize)
                .min(content.chars().count());
            document.text.line_to_char(line).unwrap_or(0) + column
        };
        // Line/column under the pointer; the column may lie past the line end.
        let position_to_point = |position: egui::Pos2| archipel_ide_text::multicaret::BlockPoint {
            line: folds.row_to_line(row_at(position)),
            column: ((position.x - rect.left() - text_left) / char_width)
                .round()
                .max(0.0) as usize,
        };
        let block_drag_id = surface_id(ui.ctx()).with("block_drag_anchor");
        // The character under the pointer (not the nearest boundary),
        // only over text, for hover documentation.
        hover = response.hover_pos().and_then(|position| {
            let line = ((position.y - rect.top()) / row_height).floor();
            if line < 0.0 || line as usize >= line_count {
                return None;
            }
            let line = line as usize;
            let line_text = document.text.line(line).unwrap_or_default();
            let length = line_text.trim_end_matches(['\r', '\n']).chars().count();
            let column = (position.x - rect.left() - text_left) / char_width;
            (column >= 0.0 && (column as usize) < length).then(|| {
                (
                    document.text.line_to_char(line).unwrap_or(0) + column as usize,
                    egui::pos2(position.x, rect.top() + (line + 1) as f32 * row_height),
                )
            })
        });
        // Clicking a change marker opens the change popup instead of
        // moving the caret.
        let marker_hit = response
            .interact_pointer_pos()
            .filter(|position| {
                crate::change_popup::in_marker_column(position.x - rect.left(), layout.width)
            })
            .and_then(|position| {
                let line = folds.row_to_line(row_at(position));
                archipel_ide_core::hunk_at_line(changes, line).map(|hunk| (hunk, position))
            });
        let blame_line = response
            .hover_pos()
            .or(response.interact_pointer_pos())
            .filter(|position| position.x - rect.left() < blame_width)
            .map(|position| folds.row_to_line(row_at(position)));
        if let (Some(lines), Some(line)) = (blame, blame_line) {
            if let Some(text) = crate::blame_gutter::tooltip(lines, line) {
                response.clone().on_hover_text_at_pointer(text);
            }
            if response.clicked()
                && let Some(entry) = lines.get(line).filter(|entry| !entry.is_uncommitted())
            {
                requests.push(EditorRequest::ShowCommit(entry.hash.clone()));
            }
        }
        // Run icons sit between the line numbers and the text.
        let gutter_hit = response
            .interact_pointer_pos()
            .filter(|position| layout.in_run_column(position.x - rect.left()))
            .and_then(|position| {
                let line = folds.row_to_line(row_at(position));
                gutter
                    .iter()
                    .any(|mark| mark.line == line)
                    .then_some((line, position))
            });
        // Hovering a line number previews the breakpoint a click would set.
        let hovered_number_line = response
            .hover_pos()
            .filter(|position| layout.in_number_column(position.x - rect.left()))
            .map(|position| folds.row_to_line(row_at(position)));
        // Clicking a line number toggles a breakpoint, as in the reference.
        let number_hit = response
            .interact_pointer_pos()
            .filter(|position| layout.in_number_column(position.x - rect.left()))
            .map(|position| folds.row_to_line(row_at(position)));
        // Fold markers sit in the gutter just left of the text; placeholders
        // expand their region when clicked.
        let fold_click = response
            .clicked()
            .then(|| response.interact_pointer_pos())
            .flatten()
            .and_then(|position| {
                let line = folds.row_to_line(row_at(position));
                let x = position.x - rect.left();
                let in_marker_column = layout.in_fold_column(x);
                let on_placeholder = folds.collapsed_on(line).is_some_and(|row| {
                    let line_start = document.text.line_to_char(line).unwrap_or(0);
                    let column = (x - text_left) / char_width;
                    let start = (row.start_char - line_start) as f32;
                    column >= start && column <= start + row.placeholder.chars().count() as f32
                });
                (in_marker_column || on_placeholder).then_some(line)
            });
        if response.clicked() && blame_line.is_some() {
            // Handled above: annotation clicks never move the caret.
        } else if let Some(line) = fold_click {
            requests.push(EditorRequest::Command(
                "editor.toggleFold",
                serde_json::json!({ "line": line }),
            ));
        } else if response.clicked()
            && let Some((hunk, position)) = marker_hit
        {
            requests.push(EditorRequest::ShowChange(hunk, position));
        } else if response.clicked()
            && let Some((line, position)) = gutter_hit
        {
            requests.push(EditorRequest::Gutter(line, position));
        } else if response.clicked()
            && let Some(line) = number_hit
        {
            requests.push(EditorRequest::Command(
                "debug.toggleBreakpoint",
                json!({"path": document.path, "line": line}),
            ));
        } else if response.clicked()
            && let Some(position) = response.interact_pointer_pos()
        {
            let offset = position_to_offset(position);
            let modifiers = ui.input(|input| input.modifiers);
            if modifiers.command && navigable {
                // Ctrl/⌘+click: Go to Declaration; with Shift the type
                // declaration, with Alt the implementations.
                document.selections.set_single(Selection::caret(offset));
                let id = if modifiers.shift {
                    "navigate.typeDeclaration"
                } else if modifiers.alt {
                    "navigate.implementation"
                } else {
                    "navigate.declaration"
                };
                requests.push(EditorRequest::Command(id, Value::Null));
            } else if modifiers.alt && modifiers.shift {
                editor_keys::toggle_caret(&mut document.selections, offset);
            } else if modifiers.shift {
                let anchor = document.selections.primary().anchor;
                document.selections.set_single(Selection {
                    anchor,
                    head: offset,
                });
            } else {
                document.selections.set_single(Selection::caret(offset));
            }
        }
        if response.double_clicked()
            && let Some(position) = response.interact_pointer_pos()
        {
            let (start, end) = ops::word_at(&document.text, position_to_offset(position));
            document.selections.set_single(Selection {
                anchor: start,
                head: end,
            });
        }
        if response.triple_clicked()
            && let Some(position) = response.interact_pointer_pos()
        {
            let line = document.text.char_to_line(position_to_offset(position));
            let (start, _, full_end) = ops::line_bounds(&document.text, line);
            document.selections.set_single(Selection {
                anchor: start,
                head: full_end,
            });
        }
        if response.drag_started()
            && let Some(position) = ui
                .input(|input| input.pointer.press_origin())
                .or(response.interact_pointer_pos())
        {
            // Alt-drag, middle-drag or column mode: rectangular selection.
            let block = ui.input(|input| input.modifiers.alt)
                || response.dragged_by(egui::PointerButton::Middle)
                || document.selections.column_mode();
            let anchor = block.then(|| position_to_point(position));
            ui.data_mut(|data| data.insert_temp(block_drag_id, anchor));
            document
                .selections
                .set_single(Selection::caret(position_to_offset(position)));
        }
        // Right-click on a line number opens the breakpoint's properties instead of the
        // editor menu, anchored where the user clicked.
        let breakpoint_properties = response
            .secondary_clicked()
            .then(|| response.interact_pointer_pos())
            .flatten()
            .filter(|position| layout.in_number_column(position.x - rect.left()))
            .map(|position| (folds.row_to_line(row_at(position)), position));
        if let Some((line, position)) = breakpoint_properties {
            requests.push(EditorRequest::BreakpointProperties(line, position));
        }
        response.context_menu(|ui| {
            if breakpoint_properties.is_some() {
                ui.close();
                return;
            }
            let mut chosen = None;
            crate::context_menus::show(ui, context_model, &mut chosen);
            match chosen.as_deref() {
                // Copy/Cut also fill the system clipboard.
                Some("$Copy") => requests.push(EditorRequest::Copy),
                Some("$Cut") => requests.push(EditorRequest::Cut),
                Some(action) => requests.push(EditorRequest::ContextAction(action.to_owned())),
                None => {}
            }
            ui.separator();
            ui.menu_button("Git", |ui| {
                ui.set_min_width(220.0);
                let annotate = if blame.is_some() {
                    "Close Annotations"
                } else {
                    "Annotate with Git Blame"
                };
                for (label, id) in [
                    (annotate, "git.annotate"),
                    ("Show History", "git.fileHistory"),
                    ("Show Diff", "vcs.showChangeDiff"),
                    ("Rollback Lines", "vcs.rollbackChange"),
                ] {
                    if ui.button(label).clicked() {
                        requests.push(EditorRequest::Command(id, Value::Null));
                        ui.close();
                    }
                }
            });
            ui.separator();
            if let Some((id, params)) = crate::ai_actions::editor_menu(ui) {
                requests.push(EditorRequest::Command(id, params));
                ui.close();
            }
        });
        let block_anchor: Option<archipel_ide_text::multicaret::BlockPoint> =
            ui.data(|data| data.get_temp(block_drag_id)).flatten();
        if response.dragged()
            && let Some(position) = response.interact_pointer_pos()
            && let Some(anchor) = block_anchor
        {
            editor_keys::drag_block(
                &document.text,
                &mut document.selections,
                anchor,
                position_to_point(position),
            );
        } else if response.dragged()
            && let Some(position) = response.interact_pointer_pos()
        {
            let offset = position_to_offset(position);
            let anchor = document.selections.primary().anchor;
            document.selections.set_single(Selection {
                anchor,
                head: offset,
            });
        }

        let painter = ui.painter().clone();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |duration| duration.as_secs() as i64);
        // Gutter background matches the editor in Islands; draw caret row first.
        let first_row = ((viewport.top() / row_height).floor().max(0.0) as usize).min(row_count);
        let last_row = ((viewport.bottom() / row_height).ceil() as usize + 1).min(row_count);
        if (first_row, last_row) != (seen_first, seen_last) {
            ui.data_mut(|data| data.insert_temp(viewport_id, (first_row, last_row)));
            // Newly exposed rows get highlighted on the next frame.
            if !document.analysis.fully_highlighted() {
                ui.ctx().request_repaint();
            }
        }
        // Gutter fold markers for regions starting in the viewport.
        let gutter_hovered = ui
            .input(|input| input.pointer.hover_pos())
            .is_some_and(|pos| rect.contains(pos) && pos.x - rect.left() < layout.width + 4.0);
        let markers: Vec<(usize, bool)> = if first_row < last_row {
            let first_line = folds.row_to_line(first_row);
            let last_line = folds.row_to_line(last_row - 1);
            let start = document.text.line_to_char(first_line).unwrap_or(0);
            let end = document
                .text
                .line_to_char(last_line + 1)
                .unwrap_or(document.text.len_chars());
            document
                .folds_in_chars(start, end)
                .iter()
                .map(|fold| (fold.start_line, document.folds.is_collapsed(fold)))
                .collect()
        } else {
            Vec::new()
        };
        // I-beam over the text only. The gutter keeps the arrow, and its clickable
        // marks (run icons, fold markers, change bars, blame) show the hand.
        if let Some(pointer) = response.hover_pos() {
            let x = pointer.x - rect.left();
            let line = folds.row_to_line(row_at(pointer));
            let clickable = layout.in_run_column(x) && gutter.iter().any(|mark| mark.line == line)
                || layout.in_fold_column(x) && markers.iter().any(|(start, _)| *start == line)
                || crate::change_popup::in_marker_column(x, layout.width)
                    && archipel_ide_core::hunk_at_line(changes, line).is_some()
                || blame.is_some() && x < blame_width;
            ui.ctx().set_cursor_icon(if x >= gutter_width {
                egui::CursorIcon::Text
            } else if clickable {
                egui::CursorIcon::PointingHand
            } else {
                egui::CursorIcon::Default
            });
        }
        let focused = response.has_focus();
        let blink_on = {
            let time = ui.input(|input| input.time);
            (time * 2.0) as i64 % 2 == 0
        };
        for row_index in first_row..last_row {
            let line = folds.row_to_line(row_index);
            let y = rect.top() + row_index as f32 * row_height;
            let line_start = document.text.line_to_char(line).unwrap_or(0);
            let line_text = document.text.line(line).unwrap_or_default();
            let content = line_text.trim_end_matches(['\r', '\n']);
            let line_end = line_start + content.chars().count();
            let row =
                Rect::from_min_size(egui::pos2(rect.left(), y), vec2(rect.width(), row_height));
            if execution_line == Some(line) {
                painter.rect_filled(row, 0.0, theme::editor::EXECUTION_LINE);
            } else if breakpoints.contains(&line) {
                painter.rect_filled(row, 0.0, theme::editor::BREAKPOINT_LINE);
            } else if line == caret_line && document.selections.primary().is_caret() {
                painter.rect_filled(row, 0.0, theme::editor::CARET_ROW);
            }
            let dot = egui::pos2(rect.left() + layout.breakpoint_center, y + row_height / 2.0);
            if breakpoints.contains(&line) {
                painter.circle_filled(dot, 6.0, *theme::ERROR);
            } else if hovered_number_line == Some(line) {
                painter.circle_filled(dot, 6.0, theme::ERROR.gamma_multiply(0.35));
            }
            if execution_line == Some(line) {
                crate::debug_window::paint_execution_arrow(&painter, dot);
            }
            painter.text(
                egui::pos2(rect.left() + layout.numbers_right, y + row_height / 2.0),
                egui::Align2::RIGHT_CENTER,
                format!("{}", line + 1),
                font.clone(),
                if line == caret_line {
                    *theme::editor::LINE_NUMBER_CARET
                } else {
                    *theme::editor::LINE_NUMBER
                },
            );
            if let Some(lines) = blame {
                crate::blame_gutter::paint(
                    &painter,
                    lines,
                    line,
                    egui::pos2(rect.left(), y),
                    row_height,
                    now,
                );
            }
            crate::change_popup::paint_marker(
                &painter,
                changes,
                line,
                egui::pos2(rect.left() + gutter_width, y),
                row_height,
            );
            if let Some(mark) = gutter.iter().find(|mark| mark.line == line) {
                crate::gutter_popup::paint_icon(
                    &painter,
                    egui::pos2(rect.left() + layout.run_center, y + row_height / 2.0),
                    mark.kind,
                );
            }
            if let Some(&(_, collapsed)) = markers.iter().find(|(start, _)| *start == line)
                && (collapsed || gutter_hovered)
            {
                crate::folding::paint_marker(
                    &painter,
                    egui::pos2(rect.left() + layout.fold_center, y + row_height / 2.0),
                    collapsed,
                    *theme::editor::LINE_NUMBER_CARET,
                );
            }
            // Indent guides every four columns of leading whitespace.
            let indent = content.chars().take_while(|c| *c == ' ').count();
            for level in (4..=indent.saturating_sub(1)).step_by(4) {
                let x = rect.left() + text_left + level as f32 * char_width;
                painter.line_segment(
                    [egui::pos2(x, y), egui::pos2(x, y + row_height)],
                    Stroke::new(1.0_f32, theme::editor::INDENT_GUIDE),
                );
            }
            // Matches are sorted and disjoint: only visit those on this line.
            let first_match = highlights.partition_point(|&(_, end)| end <= line_start);
            let visible_matches = highlights[first_match..]
                .iter()
                .take_while(|&&(start, _)| start <= line_end);
            for &(match_start, match_end) in visible_matches {
                let start = match_start.max(line_start).min(line_end);
                let end = match_end.max(line_start).min(line_end);
                if start < end {
                    let x1 = rect.left() + text_left + shifted(line, start - line_start);
                    let x2 = rect.left() + text_left + shifted(line, end - line_start);
                    painter.rect_filled(
                        Rect::from_min_max(egui::pos2(x1, y), egui::pos2(x2, y + row_height)),
                        2.0,
                        theme::editor::SEARCH_RESULT,
                    );
                }
            }
            for &offset in &brace_pair {
                if offset >= line_start && offset < line_end {
                    let x1 = rect.left() + text_left + shifted(line, offset - line_start);
                    painter.rect_filled(
                        Rect::from_min_size(egui::pos2(x1, y), vec2(char_width, row_height)),
                        2.0,
                        theme::editor::MATCHED_BRACE,
                    );
                }
            }
            for selection in document.selections.selections() {
                let start = selection.start().max(line_start).min(line_end);
                let end = selection.end().max(line_start).min(line_end);
                let spans_newline = selection.end() > line_end && selection.start() <= line_end;
                if start < end || (spans_newline && !selection.is_caret()) {
                    let x1 = rect.left() + text_left + shifted(line, start - line_start);
                    let mut x2 = rect.left() + text_left + shifted(line, end - line_start);
                    if spans_newline {
                        x2 += char_width * 0.6;
                    }
                    painter.rect_filled(
                        Rect::from_min_max(egui::pos2(x1, y), egui::pos2(x2, y + row_height)),
                        selection_corners(
                            selection.start() >= line_start,
                            selection.end() <= line_end,
                        ),
                        theme::editor::SELECTION,
                    );
                }
            }
            let line_start_byte = document.text.line_to_byte(line).unwrap_or(0);
            let collapsed = folds.collapsed_on(line);
            // A collapsed region shows the text before it, its placeholder,
            // then the rest of its last line.
            let visible = match collapsed {
                Some(row) => {
                    let prefix_chars = row.start_char.saturating_sub(line_start);
                    let end = content
                        .char_indices()
                        .nth(prefix_chars)
                        .map_or(content.len(), |(index, _)| index);
                    &content[..end]
                }
                None => content,
            };
            // Text is laid out in segments around the parameter hints.
            let mut segment_start = 0;
            let mut x = rect.left() + text_left;
            let line_hints = hints_by_line.get(&line).map_or(&[][..], Vec::as_slice);
            let visible_chars = visible.chars().count();
            let cuts = line_hints
                .iter()
                .map(|&(column, width, label)| (column.min(visible_chars), Some((width, label))))
                .chain(std::iter::once((visible_chars, None)));
            for (column, hint) in cuts {
                if column > segment_start || hint.is_none() {
                    let from = visible
                        .char_indices()
                        .nth(segment_start)
                        .map_or(visible.len(), |(index, _)| index);
                    let to = visible
                        .char_indices()
                        .nth(column)
                        .map_or(visible.len(), |(index, _)| index);
                    let job = highlighted_line_job(
                        &visible[from..to],
                        line_start_byte + from,
                        &document.analysis,
                        &font,
                    );
                    let galley = ui.fonts_mut(|fonts| fonts.layout_job(job));
                    let text_y = y + (row_height - galley.size().y) / 2.0;
                    painter.galley(egui::pos2(x, text_y), galley, Color32::WHITE);
                    x += (column - segment_start) as f32 * char_width;
                    segment_start = column;
                }
                if let Some((width, label)) = hint {
                    paint_hint(&painter, egui::pos2(x, y), width, label, &font, row_height);
                    x += width;
                }
            }
            if collapsed.is_none()
                && let Some(lens) = lens_by_line.get(&line)
            {
                let origin = egui::pos2(
                    rect.left() + text_left + shifted(line, visible_chars) + 3.0 * char_width,
                    y,
                );
                let galley = painter.layout_no_wrap(
                    lens.text(),
                    egui::FontId::proportional(theme::UI_FONT_SIZE - 2.0),
                    *theme::editor::LINE_NUMBER_CARET,
                );
                let lens_rect = Rect::from_min_size(
                    egui::pos2(origin.x, y + (row_height - galley.size().y) / 2.0),
                    galley.size(),
                );
                painter.galley(lens_rect.min, galley, Color32::WHITE);
                if let Some(pointer) = response.hover_pos()
                    && lens_rect.contains(pointer)
                {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    if response.clicked() {
                        requests.push(EditorRequest::Usages(lens.line, lens.column));
                    }
                }
            }
            if collapsed.is_none()
                && let Some(value) = inline_values.iter().find(|value| value.line == line)
            {
                let origin = egui::pos2(
                    rect.left() + text_left + shifted(line, visible_chars) + 3.0 * char_width,
                    y,
                );
                crate::debug_dialogs::paint_inline_value(&painter, origin, row_height, &value.text);
            }
            if let Some(row) = collapsed {
                paint_collapsed_tail(
                    &painter,
                    document,
                    row,
                    egui::pos2(
                        rect.left() + text_left + visible.chars().count() as f32 * char_width,
                        y,
                    ),
                    char_width,
                    &font,
                    row_height,
                );
            }

            // Rename preview: the occurrences show the name being typed. The rest
            // of the line is repainted (without highlighting) so a longer or
            // shorter name pushes it along like the real text would.
            if let Some((occurrences, name)) = rename {
                let here: Vec<(usize, usize)> = occurrences
                    .iter()
                    .copied()
                    .filter(|&(start, end)| start >= line_start && end <= line_end)
                    .collect();
                if let Some(&(first, _)) = here.first() {
                    let chars: Vec<char> = content.chars().collect();
                    let mut tail = String::new();
                    let mut boxes = Vec::new();
                    let mut cursor = first - line_start;
                    let mut column = cursor;
                    for &(start, end) in &here {
                        let (start, end) = (start - line_start, end - line_start);
                        if start < cursor {
                            continue;
                        }
                        tail.extend(&chars[cursor..start]);
                        column += start - cursor;
                        boxes.push((column, name.chars().count()));
                        tail.push_str(name);
                        column += name.chars().count();
                        cursor = end;
                    }
                    tail.extend(&chars[cursor.min(chars.len())..]);
                    let x1 = rect.left() + text_left + (first - line_start) as f32 * char_width;
                    let background =
                        if line == caret_line && document.selections.primary().is_caret() {
                            *theme::editor::CARET_ROW
                        } else {
                            *theme::editor::BACKGROUND
                        };
                    painter.rect_filled(
                        Rect::from_min_max(
                            egui::pos2(x1, y),
                            egui::pos2(rect.right(), y + row_height),
                        ),
                        0.0,
                        background,
                    );
                    painter.text(
                        egui::pos2(x1, y + row_height / 2.0),
                        egui::Align2::LEFT_CENTER,
                        &tail,
                        font.clone(),
                        *theme::editor::TEXT,
                    );
                    for (column, width) in boxes {
                        let left = rect.left() + text_left + column as f32 * char_width;
                        painter.rect_stroke(
                            Rect::from_min_size(
                                egui::pos2(left, y + 1.0),
                                vec2(width as f32 * char_width, row_height - 2.0),
                            ),
                            3.0,
                            Stroke::new(1.0_f32, *theme::editor::RENAME_OCCURRENCE),
                            egui::StrokeKind::Inside,
                        );
                    }
                }
            }

            let column_x = |column: usize| rect.left() + text_left + shifted(line, column);
            problems_ui::paint_line(
                &painter,
                problems,
                line,
                content.chars().count(),
                column_x,
                row,
            );
            for selection in document.selections.selections() {
                if selection.head >= line_start && selection.head <= line_end {
                    let x = column_x(selection.head - line_start);
                    if *selection == document.selections.primary() {
                        caret_pos = Some(egui::pos2(x, y + row_height));
                        bulb_anchor = Some(egui::pos2(
                            rect.left() + gutter_width + 2.0,
                            y + row_height / 2.0,
                        ));
                    }
                    if focused && blink_on {
                        painter.line_segment(
                            [egui::pos2(x, y + 2.0), egui::pos2(x, y + row_height - 2.0)],
                            Stroke::new(2.0_f32, theme::editor::CARET),
                        );
                    }
                }
            }
        }
        if let Some(pointer) = response.hover_pos() {
            let line = folds.row_to_line(row_at(pointer));
            let column = (pointer.x - rect.left() - text_left) / char_width;
            if line < line_count && column >= 0.0 {
                let line_len = document
                    .text
                    .line(line)
                    .unwrap_or_default()
                    .trim_end_matches(['\r', '\n'])
                    .chars()
                    .count();
                problems_ui::hover_tooltip(
                    &response,
                    problems,
                    line,
                    column.floor() as usize,
                    line_len,
                );
                // Ctrl/⌘+hover turns the identifier into a link.
                let column = column.floor() as usize;
                if navigable && ui.input(|input| input.modifiers.command) && column < line_len {
                    let line_start = document.text.line_to_char(line).unwrap_or(0);
                    let (start, end) = ops::word_at(&document.text, line_start + column);
                    if start < end && (start..end).contains(&(line_start + column)) {
                        let y = rect.top() + (line + 1) as f32 * ROW_HEIGHT - 3.0;
                        let x = |offset: usize| {
                            rect.left() + text_left + (offset - line_start) as f32 * char_width
                        };
                        painter.line_segment(
                            [egui::pos2(x(start), y), egui::pos2(x(end), y)],
                            Stroke::new(1.0_f32, crate::navigation_ui::LINK),
                        );
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }
                }
            }
        }
        if focused {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(500));
        }
        // Keep the primary caret visible after keyboard navigation.
        if focused
            && ui.input(|input| !input.events.is_empty())
            && let Some(pos) = caret_pos
        {
            let caret_rect =
                Rect::from_min_size(pos - vec2(0.0, row_height), vec2(char_width, row_height));
            if !viewport
                .translate(rect.min.to_vec2())
                .contains_rect(caret_rect)
            {
                ui.scroll_to_rect(caret_rect.expand2(vec2(char_width * 4.0, row_height)), None);
            }
        }
        response
    });

    let scroll_y = output.state.offset.y;
    let response = output.inner;
    if !response.has_focus() {
        return Ok(EditorOutput {
            changed: false,
            caret_pos,
            bulb_anchor,
            requests,
            typed: String::new(),
            hover,
            scroll_y,
        });
    }
    ui.memory_mut(|memory| {
        memory.set_focus_lock_filter(
            response.id,
            egui::EventFilter {
                tab: true,
                horizontal_arrows: true,
                vertical_arrows: true,
                // egui surrenders focus on Escape by default; the workbench's
                // Escape handler closes popups instead and the editor keeps focus.
                escape: true,
            },
        )
    });
    // The import chooser popup owns the keyboard while it is open (it is drawn after the editor).
    let chooser_open = crate::import_chooser::is_open(ui.ctx());
    let events: Vec<egui::Event> = ui
        .input(|input| input.events.clone())
        .into_iter()
        .filter(|e| !(chooser_open && matches!(e, egui::Event::Key { .. } | egui::Event::Text(_))))
        .collect();
    let initial_revision = document.text.revision();
    let page = (ui.available_height() / row_height).max(1.0) as isize;
    let mac = cfg!(target_os = "macos");
    let text_error = |error: archipel_ide_text::TextError| error.to_string();
    let command = |id: &'static str| EditorRequest::Command(id, Value::Null);
    let clone_armed = editor_keys::clone_gesture_armed(ui.ctx(), response.id, mac);
    let mut typed = String::new();
    let pairing = theme::pair_options(ui.ctx());
    for event in events {
        let (key, modifiers) = match event {
            egui::Event::Text(text) if !text.is_empty() => {
                typed.push_str(&text);
                let mut chars = text.chars();
                let single = chars.next().filter(|_| chars.next().is_none());
                let handled = match single {
                    Some(c) => ops::type_char_with(
                        &mut document.text,
                        &mut document.selections,
                        c,
                        pairing,
                    )
                    .map_err(text_error)?,
                    None => false,
                };
                if !handled {
                    document
                        .selections
                        .insert(&mut document.text, &text)
                        .map_err(text_error)?;
                }
                continue;
            }
            egui::Event::Copy => {
                requests.push(EditorRequest::Copy);
                continue;
            }
            egui::Event::Cut => {
                requests.push(EditorRequest::Cut);
                continue;
            }
            egui::Event::Paste(text) if !text.is_empty() => {
                requests.push(EditorRequest::Paste(text));
                continue;
            }
            egui::Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } => (key, modifiers),
            _ => continue,
        };
        if let Some(id) = editor_keys::action_for(key, modifiers, mac) {
            requests.push(command(id));
            continue;
        }
        if clone_armed && matches!(key, egui::Key::ArrowUp | egui::Key::ArrowDown) {
            requests.push(command(if key == egui::Key::ArrowUp {
                "editor.cloneCaretAbove"
            } else {
                "editor.cloneCaretBelow"
            }));
            continue;
        }
        if document.selections.column_mode()
            && let Some((lines, columns)) = editor_keys::column_delta(key, modifiers)
        {
            editor_keys::extend_block(document, lines, columns);
            continue;
        }
        let head = document.selections.primary().head;
        let len = document.text.len_chars();
        let extend = modifiers.shift;
        // Word-wise movement: ⌥ on macOS, Ctrl elsewhere.
        let word = if mac { modifiers.alt } else { modifiers.ctrl };
        // Duplicate, Delete Line, Comment and Move Line come from the active keymap
        // (`keybindings`), which consumes their keys before the editor sees them.
        match key {
            egui::Key::Enter if modifiers.shift => {
                // Start New Line: open a line below without splitting the current one.
                let line = document.text.char_to_line(head);
                let (_, content_end, _) = ops::line_bounds(&document.text, line);
                document
                    .selections
                    .set_single(Selection::caret(content_end));
                ops::newline(&mut document.text, &mut document.selections).map_err(text_error)?;
            }
            egui::Key::Enter => {
                ops::newline(&mut document.text, &mut document.selections).map_err(text_error)?
            }
            egui::Key::Tab if modifiers.shift => {
                requests.push(command("editor.unindent"));
            }
            egui::Key::Tab => {
                let primary = document.selections.primary();
                let multi_line = document.text.char_to_line(primary.start())
                    != document.text.char_to_line(primary.end());
                if multi_line {
                    requests.push(command("editor.indent"));
                } else {
                    // Expands a live template abbreviation; otherwise indents.
                    requests.push(command("template.expand"));
                }
            }
            egui::Key::Backspace if word && document.selections.primary().is_caret() => {
                let target = ops::word_left(&document.text, head);
                document.selections.set_single(Selection {
                    anchor: head,
                    head: target,
                });
                document
                    .selections
                    .insert(&mut document.text, "")
                    .map_err(text_error)?;
            }
            egui::Key::Backspace => {
                ops::backspace(&mut document.text, &mut document.selections).map_err(text_error)?
            }
            egui::Key::Delete if word && document.selections.primary().is_caret() => {
                let target = ops::word_right(&document.text, head);
                document.selections.set_single(Selection {
                    anchor: head,
                    head: target,
                });
                document
                    .selections
                    .insert(&mut document.text, "")
                    .map_err(text_error)?;
            }
            egui::Key::Delete => document
                .selections
                .delete_forward(&mut document.text)
                .map_err(text_error)?,
            egui::Key::A if modifiers.command => {
                document.selections.set_single(Selection {
                    anchor: 0,
                    head: len,
                });
            }
            egui::Key::ArrowLeft if word => {
                editor_keys::move_carets(document, extend, |d, s| ops::word_left(&d.text, s.head))
            }
            egui::Key::ArrowRight if word => {
                editor_keys::move_carets(document, extend, |d, s| ops::word_right(&d.text, s.head))
            }
            egui::Key::ArrowLeft => editor_keys::move_carets(document, extend, |_, s| {
                if !extend && !s.is_caret() {
                    s.start()
                } else {
                    s.head.saturating_sub(1)
                }
            }),
            egui::Key::ArrowRight => editor_keys::move_carets(document, extend, |_, s| {
                if !extend && !s.is_caret() {
                    s.end()
                } else {
                    (s.head + 1).min(len)
                }
            }),
            egui::Key::ArrowUp => editor_keys::move_carets(document, extend, |d, s| {
                vertical_target(d, &folds, s.head, -1)
            }),
            egui::Key::ArrowDown => editor_keys::move_carets(document, extend, |d, s| {
                vertical_target(d, &folds, s.head, 1)
            }),
            egui::Key::PageUp => move_primary(
                document,
                vertical_target(document, &folds, head, -page),
                extend,
            ),
            egui::Key::PageDown => move_primary(
                document,
                vertical_target(document, &folds, head, page),
                extend,
            ),
            egui::Key::Home if modifiers.command => move_primary(document, 0, extend),
            egui::Key::Home => editor_keys::move_carets(document, extend, |d, s| {
                let line = d.text.char_to_line(s.head);
                let start = d.text.line_to_char(line).unwrap_or(0);
                let indent = d
                    .text
                    .line(line)
                    .unwrap_or_default()
                    .chars()
                    .take_while(|c| c.is_whitespace() && *c != '\n')
                    .count();
                // Smart home: toggle between first non-blank character and column 0.
                if s.head == start + indent {
                    start
                } else {
                    start + indent
                }
            }),
            egui::Key::End if modifiers.command => move_primary(document, len, extend),
            egui::Key::End => editor_keys::move_carets(document, extend, |d, s| {
                let line = d.text.char_to_line(s.head);
                d.text.line_to_char(line).unwrap_or(0) + line_length(d, line)
            }),
            egui::Key::Escape if document.selections.selections().len() > 1 => {
                let primary = document.selections.primary();
                document
                    .selections
                    .set_single(Selection::caret(primary.head));
            }
            _ => {}
        }
    }
    Ok(EditorOutput {
        changed: document.text.revision() != initial_revision,
        caret_pos,
        bulb_anchor,
        requests,
        typed,
        hover,
        scroll_y,
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn gutter_columns_follow_the_reference_order() {
        let plain = super::GutterLayout::new(0.0, 20.0, false);
        assert!(!plain.in_run_column(plain.numbers_right + 2.0));
        assert!(plain.numbers_right < plain.fold_center);
        assert!(plain.in_number_column(plain.breakpoint_center));
        assert!(!plain.in_number_column(plain.fold_center));
        assert!(plain.breakpoint_center < plain.numbers_right);
        assert!(
            plain.fold_center + super::GutterLayout::FOLD_COLUMN / 2.0 <= plain.width - 7.0 - 4.0
        );
        let full = super::GutterLayout::new(70.0, 20.0, true);
        // blame | line numbers | run icons | folding | change bar | text
        assert!(70.0 < full.numbers_right);
        assert!(full.numbers_right < full.run_center);
        assert!(full.run_center < full.fold_center);
        assert!(full.in_run_column(full.run_center));
        assert!(!full.in_fold_column(full.run_center));
        assert!(full.in_fold_column(full.fold_center));
        // The change bar (4 px wide, 7 px inside the edge) is right of the fold column.
        assert!(full.fold_center + super::GutterLayout::FOLD_COLUMN / 2.0 <= full.width - 11.0);
        assert!(!crate::change_popup::in_marker_column(
            full.fold_center,
            full.width
        ));
    }

    use super::*;

    #[test]
    fn multi_line_selections_round_only_their_outer_corners() {
        let single = selection_corners(true, true);
        assert_eq!(single, egui::CornerRadius::same(2));
        let first = selection_corners(true, false);
        assert_eq!((first.nw, first.ne, first.sw, first.se), (2, 2, 0, 0));
        let middle = selection_corners(false, false);
        assert_eq!(middle, egui::CornerRadius::ZERO);
        let last = selection_corners(false, true);
        assert_eq!((last.nw, last.ne, last.sw, last.se), (0, 0, 2, 2));
    }

    #[test]
    fn modified_badge_does_not_cover_the_tab_title() {
        for text_width in [20.0, 87.0, 240.0] {
            let width = 10.0 + 16.0 + 6.0 + text_width + 6.0 + 16.0 + 8.0;
            let rect = Rect::from_min_size(egui::pos2(100.0, 5.0), vec2(width, 28.0));
            let layout = TabLayout::new(rect, vec2(text_width, 16.0));
            let badge = Rect::from_center_size(layout.modified_dot, vec2(5.0, 5.0));
            assert!(
                !badge.intersects(layout.text),
                "badge over title at {text_width}"
            );
            assert!(!badge.intersects(layout.close));
            assert!(!layout.text.intersects(layout.close));
            assert!(rect.contains_rect(badge));
        }
    }

    #[test]
    fn highlighting_skips_spans_that_split_characters() {
        let mut analysis = LanguageAnalysis::default();
        analysis.language = Some("Rust");
        analysis.highlights = archipel_ide_language::HighlightLayer::from_spans([
            archipel_ide_language::HighlightSpan {
                start_byte: 1,
                end_byte: 2,
                key: "DEFAULT_KEYWORD",
            },
        ]);
        let job = highlighted_line_job("ä", 0, &analysis, &egui::FontId::monospace(13.0));
        assert_eq!(job.text, "ä");
    }
}
