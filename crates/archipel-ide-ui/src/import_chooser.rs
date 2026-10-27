//! The chooser popup of the import fixes (`Method to Import`, `Field to Import`, `Class to
//! Import`, spec 05 section 5.4.2): a kit list popup with a title, `Class.member` rows and the
//! package as the grey shortcut column, speed search on and no auto-select of a lone match.
//! The rows are owned by the import fix provider of the language pack; choosing one dispatches
//! its `<provider>.chooseImport` command.

use crate::IdeApp;
use archipel_kit::ui::popup::{BaseListPopupStep, ListPopup};
use eframe::egui;
use serde_json::json;

/// What the popup shows (rebuilt when the chooser's rows change).
pub struct ImportChooser {
    key: Vec<String>,
    popup: ListPopup<usize>,
}

fn open_flag() -> egui::Id {
    egui::Id::new("import_chooser_open")
}

/// Whether the chooser was open last frame; the editor then leaves keys to the popup.
pub fn is_open(ctx: &egui::Context) -> bool {
    ctx.data(|d| d.get_temp::<bool>(open_flag()).unwrap_or(false))
}

pub fn show(app: &mut IdeApp, ctx: &egui::Context) {
    let chooser = app.state.import_chooser();
    ctx.data_mut(|d| d.insert_temp(open_flag(), chooser.is_some()));
    let Some((provider, chooser)) = chooser else {
        app.import_chooser = None;
        return;
    };
    let key: Vec<String> = std::iter::once(chooser.title.clone())
        .chain(
            chooser
                .rows
                .iter()
                .map(|r| format!("{} {}", r.label, r.detail.clone().unwrap_or_default())),
        )
        .collect();
    if app.import_chooser.as_ref().is_none_or(|c| c.key != key) {
        let labels: Vec<String> = chooser.rows.iter().map(|r| r.label.clone()).collect();
        let details: Vec<Option<String>> = chooser.rows.iter().map(|r| r.detail.clone()).collect();
        let step = ImportStep {
            inner: BaseListPopupStep::new(
                chooser.title.clone(),
                (0..labels.len()).collect::<Vec<usize>>(),
            )
            .speed_search(true),
            labels,
            details,
        };
        let mut popup = ListPopup::new("import_chooser", step);
        popup.open(ctx);
        app.import_chooser = Some(ImportChooser { key, popup });
    }
    let anchor = app
        .caret_screen_pos
        .unwrap_or_else(|| ctx.content_rect().center());
    let Some(c) = app.import_chooser.as_mut() else {
        return;
    };
    let response = c.popup.show_at(ctx, anchor + egui::vec2(-20.0, 20.0));
    if let Some(index) = response.chosen {
        app.import_chooser = None;
        app.dispatch(&format!("{provider}.chooseImport"), json!({"index": index}));
        ctx.memory_mut(|m| m.request_focus(crate::editor::editor_id()));
    } else if response.cancelled || !c.popup.is_open() {
        app.import_chooser = None;
        app.dispatch(
            &format!("{provider}.cancelChooser"),
            serde_json::Value::Null,
        );
        ctx.memory_mut(|m| m.request_focus(crate::editor::editor_id()));
    }
}

/// Row text `Class.member`, package detail on the right, speed search on.
struct ImportStep {
    inner: BaseListPopupStep<usize>,
    labels: Vec<String>,
    details: Vec<Option<String>>,
}

impl archipel_kit::ui::popup::PopupStep<usize> for ImportStep {
    fn title(&self) -> Option<&str> {
        self.inner.title()
    }
    fn values(&self) -> &[usize] {
        self.inner.values()
    }
    fn text_for(&self, value: &usize) -> String {
        self.labels.get(*value).cloned().unwrap_or_default()
    }
    fn shortcut_for(&self, value: &usize) -> Option<String> {
        self.details.get(*value).cloned().flatten()
    }
    fn is_speed_search_enabled(&self) -> bool {
        true
    }
    fn is_auto_selection_enabled(&self) -> bool {
        false
    }
}
