//! In-editor Find / Replace bar (Ctrl+F / Ctrl+R): a view over the core's
//! [`FindSession`] (spec `docs/specs/search/01-find-replace-in-file.md`).
//!
//! The session decides everything the bar shows and does: the matches and the `n/m` counter, the
//! navigation with its "no silent wrap" hint, the options, Replace and Replace All with their
//! templates. This file hands the session the document and the selection of the active editor
//! before it is used, draws what it reports and applies what it asks for (the replacements and
//! the new selection) through the `editor.applyEdits` command.

use crate::icons::{self, Icon};
use crate::{IdeApp, theme};
use archipel_ide_core::Document;
use archipel_ide_core::find_session::{FindSession, Range};
use eframe::egui::{self, RichText, vec2};
use serde_json::{Value, json};
use std::path::PathBuf;

pub struct FindBar {
    session: FindSession,
    /// The document (path and revision) the session has the text of.
    bound: Option<(PathBuf, u64)>,
    /// The contents of the text fields as egui edits them.
    query_text: String,
    replacement_text: String,
    /// The field to select and focus in the next frame.
    focus_requested: bool,
    /// The editor scrolls to the new selection in the next frame.
    reveal: bool,
}

impl Default for FindBar {
    fn default() -> Self {
        Self {
            session: FindSession::new("", Vec::new()),
            bound: None,
            query_text: String::new(),
            replacement_text: String::new(),
            focus_requested: false,
            reveal: false,
        }
    }
}

/// What the bar asks the session to do after the frame is drawn.
enum Op {
    Query(String),
    Replacement(String),
    Toggle(&'static str),
    Next,
    Previous,
    Replace,
    ReplaceAll,
    Exclude,
    FocusEditor,
    Close,
}

pub fn query_id() -> egui::Id {
    egui::Id::new("findbar_query")
}

fn replacement_id() -> egui::Id {
    egui::Id::new("findbar_replacement")
}

/// The selections of the document as the session wants them: the primary one last.
fn session_selections(document: &Document) -> Vec<Range> {
    let mut all: Vec<Range> = document
        .selections
        .selections()
        .iter()
        .map(|selection| (selection.anchor, selection.head))
        .collect();
    if all.len() > 1 {
        all.rotate_left(1);
    }
    all
}

/// The selections of the session as the document wants them: the primary one first.
fn document_selections(session: &FindSession) -> Vec<Value> {
    let mut all: Vec<Range> = session.selections().to_vec();
    if all.len() > 1 {
        all.rotate_right(1);
    }
    all.into_iter()
        .map(|(anchor, head)| json!({"anchor": anchor, "head": head}))
        .collect()
}

impl FindBar {
    pub fn is_open(&self) -> bool {
        self.session.is_open()
    }

    pub fn is_replace(&self) -> bool {
        self.session.replace_mode()
    }

    /// The text of the search field.
    pub fn query(&self) -> &str {
        self.session.query()
    }

    /// The result counter of the bar (`2/5`, `0 results`, `Bad pattern`).
    pub fn counter(&self) -> String {
        self.session.counter()
    }

    /// Closes the bar; the selection stays.
    pub fn close(&mut self) {
        self.session.close();
    }

    /// The matches to paint in the editor: none while the bar is closed.
    pub fn matches(&self) -> Vec<Range> {
        if self.session.is_open() {
            self.session.matches()
        } else {
            Vec::new()
        }
    }

    /// The editor should scroll to the selection in this frame (once per request).
    pub(crate) fn take_reveal(&mut self) -> bool {
        std::mem::take(&mut self.reveal)
    }

    /// Hands the session the text and the selections of `document`.
    fn sync(&mut self, document: &Document) {
        let key = (document.path.as_path(), document.text.revision());
        let bound = self
            .bound
            .as_ref()
            .is_some_and(|(path, revision)| path.as_path() == key.0 && *revision == key.1);
        if !bound {
            self.session.set_text(&document.text.text());
            self.bound = Some((document.path.clone(), key.1));
        }
        self.session.set_selections(&session_selections(document));
    }

    /// The fields show what the session holds (it seeds the query from the selection, takes a
    /// history entry, ...).
    fn pull_fields(&mut self) {
        if self.session.query() != self.query_text {
            self.query_text = self.session.query().to_owned();
        }
        if self.session.replacement() != self.replacement_text {
            self.replacement_text = self.session.replacement().to_owned();
        }
    }
}

impl IdeApp {
    /// Runs `f` on the session after it was told about the active editor, then applies its
    /// edits and its selection to the document.
    fn with_find(&mut self, f: impl FnOnce(&mut FindSession), status: bool) {
        let Some(index) = self.state.active_index() else {
            return;
        };
        let Some(document) = self.state.documents().get(index) else {
            return;
        };
        self.find.sync(document);
        f(&mut self.find.session);
        let edits = self.find.session.take_edits();
        let current = session_selections(&self.state.documents()[index]);
        let selections = self.find.session.selections().to_vec();
        if edits.is_empty() && selections == current {
            return;
        }
        let mut params = json!({
            "edits": edits
                .iter()
                .map(|((start, end), text)| json!({"start": start, "end": end, "text": text}))
                .collect::<Vec<_>>(),
            "selections": document_selections(&self.find.session),
        });
        if status && let Some(status) = self.find.session.status() {
            params["status"] = json!(status);
        }
        self.dispatch("editor.applyEdits", params);
        // The document now has the text of the session: no need to copy it again.
        if let Some(document) = self.state.documents().get(index) {
            self.find.bound = Some((document.path.clone(), document.text.revision()));
        }
        self.find.reveal = true;
    }

    /// Ctrl+F / Ctrl+R.
    pub(crate) fn open_find_bar(&mut self, replace: bool) {
        self.with_find(|session| session.open_bar(replace), false);
        self.find.focus_requested = true;
        self.find.pull_fields();
    }

    /// F3 / Shift+F3: the next or previous match of the last search; the bar opens when there is
    /// none yet.
    pub(crate) fn find_step(&mut self, forward: bool) {
        if self.find.session.query().is_empty() {
            self.open_find_bar(false);
            return;
        }
        self.with_find(
            |session| {
                if forward {
                    session.find_next();
                } else {
                    session.find_previous();
                }
            },
            false,
        );
    }

    /// Escape in the bar or the editor.
    pub(crate) fn close_find_bar(&mut self, ctx: &egui::Context) {
        self.find.close();
        ctx.memory_mut(|memory| memory.request_focus(crate::editor::editor_id()));
    }

    fn find_op(&mut self, op: Op, ctx: &egui::Context) {
        match op {
            Op::Query(text) => self.with_find(|session| session.set_query(&text), false),
            Op::Replacement(text) => {
                self.with_find(|session| session.set_replacement(&text), false);
            }
            Op::Toggle(option) => self.with_find(|session| session.toggle(option), false),
            Op::Next => self.with_find(FindSession::find_next, false),
            Op::Previous => self.with_find(FindSession::find_previous, false),
            Op::Replace => self.with_find(FindSession::replace_current, false),
            Op::ReplaceAll => self.with_find(FindSession::replace_all, true),
            Op::Exclude => self.with_find(FindSession::toggle_exclude, false),
            Op::FocusEditor => {
                self.with_find(FindSession::focus_editor, false);
                if !self.find.is_open() {
                    self.close_find_bar(ctx);
                } else {
                    ctx.memory_mut(|memory| memory.request_focus(crate::editor::editor_id()));
                }
            }
            Op::Close => self.close_find_bar(ctx),
        }
    }
}

fn toggle(ui: &mut egui::Ui, label: &str, on: bool, tooltip: &str) -> bool {
    let (rect, response) = ui.allocate_exact_size(vec2(24.0, 22.0), egui::Sense::click());
    let fill = if on {
        *theme::SELECTION
    } else if response.hovered() {
        *theme::HOVER
    } else {
        egui::Color32::TRANSPARENT
    };
    ui.painter().rect_filled(rect, 4.0, fill);
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        theme::semibold(theme::UI_FONT_SIZE - 1.0),
        if on {
            *theme::TEXT_BRIGHT
        } else {
            *theme::TEXT_SECONDARY
        },
    );
    response.on_hover_text(tooltip).clicked()
}

/// What the session reports for one frame of the bar.
struct View {
    replace: bool,
    flags: archipel_ide_core::find_session::Flags,
    counter: String,
    not_found: bool,
    hint: Option<String>,
    dialog: Option<String>,
    can_replace: bool,
    placeholder: String,
    exclude: &'static str,
}

/// The placeholder of the search field names the options that are on (SRCH-FIL-002 table).
fn placeholder(flags: archipel_ide_core::find_session::Flags) -> String {
    if flags.in_selection {
        return "Search in selection only".into();
    }
    let on: Vec<&str> = [
        (flags.match_case, "match case"),
        (flags.whole_words, "words"),
        (flags.regex, "regex"),
    ]
    .into_iter()
    .filter_map(|(on, name)| on.then_some(name))
    .collect();
    if on.is_empty() {
        "Search".into()
    } else {
        on.join(" and ")
    }
}

pub fn show(app: &mut IdeApp, ui: &mut egui::Ui) {
    let Some(index) = app.state.active_index() else {
        return;
    };
    app.find.sync(&app.state.documents()[index]);
    app.find.pull_fields();
    let session = &app.find.session;
    let view = View {
        replace: session.replace_mode(),
        flags: session.flags(),
        counter: session.counter(),
        not_found: session.field_in_error_state(),
        hint: session.hint().map(str::to_owned),
        dialog: session
            .dialog()
            .map(|(title, message)| format!("{title}: {message}")),
        can_replace: !session.matches().is_empty() && session.error().is_none(),
        placeholder: placeholder(session.flags()),
        exclude: session.exclude_label(),
    };
    let ctx = ui.ctx().clone();
    let mut op: Option<Op> = None;
    // The mnemonics of the bar (SRCH-FIL-002): Alt+C, Alt+W and Alt+X toggle the options, Alt+E
    // Preserve case; Alt+P, Alt+A and Alt+L are the buttons Replace, Replace All and Exclude.
    let keys = [
        (egui::Key::C, Op::Toggle("case")),
        (egui::Key::W, Op::Toggle("words")),
        (egui::Key::X, Op::Toggle("regex")),
    ];
    let replace_keys = [
        (egui::Key::E, Op::Toggle("preserve")),
        (egui::Key::P, Op::Replace),
        (egui::Key::A, Op::ReplaceAll),
        (egui::Key::L, Op::Exclude),
    ];
    for (key, wanted) in keys
        .into_iter()
        .chain(replace_keys.into_iter().filter(|_| view.replace))
    {
        if ctx.input_mut(|input| input.consume_key(egui::Modifiers::ALT, key)) {
            op = Some(wanted);
        }
    }
    // Ctrl+Enter: back to the editor, or close when the field is empty (SRCH-FIL-003).
    if ctx.input_mut(|input| input.consume_key(egui::Modifiers::COMMAND, egui::Key::Enter)) {
        op = Some(Op::FocusEditor);
    }
    egui::Frame::new()
        .fill(*theme::editor::BACKGROUND_PANEL)
        .inner_margin(egui::Margin::symmetric(10, 4))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                icons::show(ui, Icon::Search, 14.0, *theme::TEXT_SECONDARY);
                let response = ui.add(
                    egui::TextEdit::singleline(&mut app.find.query_text)
                        .id(query_id())
                        .frame(false)
                        .desired_width(260.0)
                        .hint_text(&view.placeholder),
                );
                if std::mem::take(&mut app.find.focus_requested) {
                    let mut state = egui::text_edit::TextEditState::load(ui.ctx(), query_id())
                        .unwrap_or_default();
                    state
                        .cursor
                        .set_char_range(Some(egui::text::CCursorRange::two(
                            egui::text::CCursor::new(0),
                            egui::text::CCursor::new(app.find.query_text.chars().count()),
                        )));
                    state.store(ui.ctx(), query_id());
                    response.request_focus();
                }
                if response.changed() {
                    op = Some(Op::Query(app.find.query_text.clone()));
                }
                if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    op = Some(if ui.input(|i| i.modifiers.shift) {
                        Op::Previous
                    } else {
                        Op::Next
                    });
                    response.request_focus();
                }
                for (label, tooltip, option, on) in [
                    ("Cc", "Match Case (Alt+C)", "case", view.flags.match_case),
                    ("W", "Words (Alt+W)", "words", view.flags.whole_words),
                    (".*", "Regex (Alt+X)", "regex", view.flags.regex),
                ] {
                    if toggle(ui, label, on, tooltip) {
                        op = Some(Op::Toggle(option));
                    }
                }
                let color = if view.not_found {
                    *theme::ERROR
                } else {
                    *theme::TEXT_SECONDARY
                };
                ui.label(RichText::new(&view.counter).color(color));
                if icons::button(ui, Icon::ChevronUp, "Previous Occurrence (⇧Enter)").clicked() {
                    op = Some(Op::Previous);
                }
                if icons::button(ui, Icon::ChevronDown, "Next Occurrence (Enter)").clicked() {
                    op = Some(Op::Next);
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if icons::button(ui, Icon::Close, "Close (Escape)").clicked() {
                        op = Some(Op::Close);
                    }
                });
            });
            if view.replace {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    ui.add_space(18.0);
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut app.find.replacement_text)
                            .id(replacement_id())
                            .frame(false)
                            .desired_width(260.0)
                            .hint_text("Replace"),
                    );
                    if response.changed() {
                        op = Some(Op::Replacement(app.find.replacement_text.clone()));
                    }
                    if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        op = Some(Op::Replace);
                        response.request_focus();
                    }
                    if toggle(ui, "AB", view.flags.preserve_case, "Preserve Case (Alt+E)") {
                        op = Some(Op::Toggle("preserve"));
                    }
                    if ui
                        .add_enabled(view.can_replace, egui::Button::new("Replace"))
                        .on_hover_text("Replace (Alt+P)")
                        .clicked()
                    {
                        op = Some(Op::Replace);
                    }
                    if ui
                        .add_enabled(view.can_replace, egui::Button::new("Replace All"))
                        .on_hover_text("Replace All (Alt+A)")
                        .clicked()
                    {
                        op = Some(Op::ReplaceAll);
                    }
                    if ui
                        .add_enabled(view.can_replace, egui::Button::new(view.exclude))
                        .on_hover_text("Exclude or include the current match (Alt+L)")
                        .clicked()
                    {
                        op = Some(Op::Exclude);
                    }
                });
            }
            if let Some(message) = view.dialog.as_ref().or(view.hint.as_ref()) {
                let color = if view.dialog.is_some() {
                    *theme::ERROR
                } else {
                    *theme::TEXT_SECONDARY
                };
                ui.label(RichText::new(message).color(color));
            }
        });
    let rect = ui.min_rect();
    ui.painter().hline(
        rect.x_range(),
        ui.cursor().top(),
        egui::Stroke::new(1.0_f32, theme::POPUP_BORDER),
    );
    if let Some(op) = op {
        app.find_op(op, &ctx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{click, headless_app, idle, press, type_text};

    fn text(app: &IdeApp) -> String {
        app.state.documents()[0].text.text()
    }

    fn selected(app: &IdeApp) -> String {
        let document = &app.state.documents()[0];
        let primary = document.selections.primary();
        document.text.slice(primary.start(), primary.end()).unwrap()
    }

    #[test]
    fn the_session_is_handed_the_document_and_the_selection() {
        let mut bar = FindBar::default();
        let mut document = archipel_ide_core::Document::new("n.txt".into(), "a needle, Needle");
        bar.sync(&document);
        bar.session.open_bar(false);
        bar.session.set_query("needle");
        assert_eq!(bar.matches().len(), 2);
        assert_eq!(bar.counter(), "1/2");
        bar.session.toggle("case");
        assert_eq!(bar.matches().len(), 1, "options reach the matches");
        // An edit of the editor is picked up by the next sync.
        document.text.edit(0, 0, "needle ").unwrap();
        bar.sync(&document);
        assert_eq!(bar.matches().len(), 2, "edits invalidate the matches");
        // The matches are not painted while the bar is closed.
        bar.close();
        assert!(bar.matches().is_empty());
    }

    #[test]
    fn invalid_regular_expressions_show_bad_pattern() {
        let mut bar = FindBar::default();
        bar.sync(&archipel_ide_core::Document::new(
            "n.txt".into(),
            "needle neeedles neeedle",
        ));
        bar.session.open_bar(false);
        bar.session.toggle("regex");
        bar.session.set_query(r"ne+dle\b");
        assert_eq!(bar.matches().len(), 2);
        bar.session.set_query("(");
        assert_eq!(bar.counter(), "Bad pattern");
        bar.session.toggle("regex");
        assert_eq!(bar.counter(), "0 results", "plain text is never invalid");
    }

    #[test]
    fn selections_are_handed_over_with_the_primary_last_and_back_with_it_first() {
        let mut document = archipel_ide_core::Document::new("n.txt".into(), "one two three");
        document.selections.set_all(vec![
            archipel_ide_text::Selection { anchor: 0, head: 3 },
            archipel_ide_text::Selection { anchor: 4, head: 7 },
        ]);
        let handed = session_selections(&document);
        assert_eq!(handed, vec![(4, 7), (0, 3)]);
        let session = FindSession::new("one two three", handed);
        let back = document_selections(&session);
        assert_eq!(back[0], json!({"anchor": 0, "head": 3}));
    }

    #[test]
    fn ctrl_f_typing_and_enter_walk_through_the_matches_with_the_counter() {
        let (mut app, ctx) = headless_app("findbar-keys", "a.txt", "one two one\nthree one\n");
        app.focus_editor(&ctx);
        idle(&mut app, &ctx);
        press(&mut app, &ctx, "Ctrl+F");
        idle(&mut app, &ctx);
        assert!(app.find.is_open());
        type_text(&mut app, &ctx, "one");
        assert_eq!(app.find.query(), "one");
        assert_eq!(app.find.counter(), "1/3");
        assert_eq!(selected(&app), "one");
        let primary = app.state.documents()[0].selections.primary();
        assert_eq!((primary.start(), primary.end()), (0, 3));
        press(&mut app, &ctx, "Enter");
        assert_eq!(app.find.counter(), "2/3");
        press(&mut app, &ctx, "Enter");
        assert_eq!(app.find.counter(), "3/3");
        // The end of the text: no silent wrap, the hint says so (SRCH-FIL-031).
        press(&mut app, &ctx, "Enter");
        assert_eq!(app.find.counter(), "3/3");
        assert!(
            app.find
                .session
                .hint()
                .is_some_and(|hint| hint.contains("not found")),
            "{:?}",
            app.find.session.hint()
        );
        press(&mut app, &ctx, "Enter");
        assert_eq!(app.find.counter(), "1/3", "the second press wraps");
        // Shift+Enter goes back, with the same hint at the top first.
        press(&mut app, &ctx, "Shift+Enter");
        assert_eq!(app.find.counter(), "1/3");
        press(&mut app, &ctx, "Shift+Enter");
        assert_eq!(app.find.counter(), "3/3");
        press(&mut app, &ctx, "Shift+Enter");
        assert_eq!(app.find.counter(), "2/3");
        // Escape closes the bar and leaves the selection.
        press(&mut app, &ctx, "Escape");
        assert!(!app.find.is_open());
        assert_eq!(selected(&app), "one");
    }

    #[test]
    fn alt_c_alt_w_and_alt_x_toggle_the_options_and_the_counter_follows() {
        let (mut app, ctx) = headless_app("findbar-toggles", "a.txt", "Needle needle needles\n");
        app.focus_editor(&ctx);
        idle(&mut app, &ctx);
        press(&mut app, &ctx, "Ctrl+F");
        idle(&mut app, &ctx);
        type_text(&mut app, &ctx, "needle");
        assert_eq!(app.find.counter(), "1/3");
        press(&mut app, &ctx, "Alt+C");
        assert_eq!(app.find.counter(), "1/2", "Match Case");
        press(&mut app, &ctx, "Alt+W");
        assert_eq!(app.find.counter(), "1/1", "Words");
        assert_eq!(
            placeholder(app.find.session.flags()),
            "match case and words"
        );
        press(&mut app, &ctx, "Alt+C");
        press(&mut app, &ctx, "Alt+W");
        assert_eq!(app.find.counter(), "2/3");
        press(&mut app, &ctx, "Alt+X");
        assert!(app.find.session.flags().regex);
        type_text(&mut app, &ctx, "(");
        assert_eq!(app.find.counter(), "Bad pattern");
    }

    #[test]
    fn replace_and_replace_all_edit_the_document_as_the_session_says() {
        let (mut app, ctx) = headless_app("findbar-replace", "a.txt", "aaa b aaa\nlast aaa\n");
        app.focus_editor(&ctx);
        idle(&mut app, &ctx);
        press(&mut app, &ctx, "Ctrl+R");
        idle(&mut app, &ctx);
        assert!(app.find.is_open() && app.find.is_replace());
        type_text(&mut app, &ctx, "aaa");
        assert_eq!(app.find.counter(), "1/3");
        // A click into the replace field, the text, and Enter replaces the current match.
        let field = ctx
            .read_response(replacement_id())
            .expect("the replace field")
            .rect;
        click(&mut app, &ctx, field.center());
        type_text(&mut app, &ctx, "x");
        assert_eq!(app.find.session.replacement(), "x");
        press(&mut app, &ctx, "Enter");
        assert_eq!(text(&app), "x b aaa\nlast aaa\n");
        assert_eq!(app.find.counter(), "1/2", "the next match is selected");
        // Replace All (Alt+A) is one undo step and does not rescan its result (SRCH-FIL-041).
        type_text(&mut app, &ctx, "aaa");
        assert_eq!(app.find.session.replacement(), "xaaa");
        press(&mut app, &ctx, "Alt+A");
        assert_eq!(text(&app), "x b xaaa\nlast xaaa\n");
        assert_eq!(app.state.status(), "2 occurrences replaced");
        app.dispatch("editor.undo", Value::Null);
        assert_eq!(text(&app), "x b aaa\nlast aaa\n", "one undo step");
    }

    #[test]
    fn f3_repeats_the_search_with_the_bar_closed() {
        let (mut app, ctx) = headless_app("findbar-f3", "a.txt", "one two one\n");
        app.focus_editor(&ctx);
        idle(&mut app, &ctx);
        // Without a search F3 does nothing (the action is disabled).
        press(&mut app, &ctx, "F3");
        assert!(!app.find.is_open());
        press(&mut app, &ctx, "Ctrl+F");
        idle(&mut app, &ctx);
        type_text(&mut app, &ctx, "one");
        press(&mut app, &ctx, "Escape");
        assert!(!app.find.is_open());
        assert_eq!(selected(&app), "one");
        let before = app.state.documents()[0].selections.primary().start();
        press(&mut app, &ctx, "F3");
        let after = app.state.documents()[0].selections.primary().start();
        assert!(!app.find.is_open());
        assert_ne!(before, after, "F3 moved to the next match");
        assert_eq!(selected(&app), "one");
    }
}
