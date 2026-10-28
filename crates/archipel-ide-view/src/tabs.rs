//! The editor tab strip (TUI-GUI-085..088): one tab per open document (all of them, or those of
//! one split group), then the diff tab and the plugin tabs of the focused group.

use crate::cx::ViewCx;
use archipel_kit_model::{Glyph, IconKey, Tab, TabBar, TabEvent, Tooltip};

/// The id of the tab of document `index`: its project-relative path.
pub fn document_tab_id(cx: &ViewCx, index: usize) -> Option<String> {
    cx.state
        .documents()
        .get(index)
        .map(|doc| doc.path.display().to_string())
}

/// The document index of tab `id`.
pub fn document_index(cx: &ViewCx, id: &str) -> Option<usize> {
    cx.state
        .documents()
        .iter()
        .position(|doc| doc.path.display().to_string() == id)
}

/// The id of the diff tab.
pub const DIFF_TAB: &str = "diff";

/// The id of plugin tab `index`.
pub fn plugin_tab_id(index: usize) -> String {
    format!("plugin:{index}")
}

/// The tab strip. `group` lists the document indices of one editor group (`None`: every
/// document); `primary` marks the focused group, which also carries the diff and plugin tabs.
pub fn editor_tabs(cx: &ViewCx, group: Option<&[usize]>, primary: bool) -> TabBar {
    let state = cx.state;
    let documents = state.documents();
    let indices: Vec<usize> = match group {
        Some(group) => group.to_vec(),
        None => (0..documents.len()).collect(),
    };
    let plugin_active = state.plugin_host_service().active_tab().is_some();
    let mut tabs = Vec::new();
    for index in indices {
        let Some(doc) = documents.get(index) else {
            continue;
        };
        let name = doc
            .path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("untitled")
            .to_owned();
        let mut tab = Tab::new(
            doc.path.display().to_string(),
            name,
            IconKey::File(doc.path.display().to_string()),
        );
        tab.selected = state.active_index() == Some(index) && !state.diff_active() && !plugin_active;
        tab.dirty = doc.text.is_dirty();
        tab.pinned = state.is_pinned(index);
        tab.preview = state.is_preview(index);
        tab.error = state.problems().has_errors(&doc.path);
        tab.tooltip = Some(Tooltip::new(doc.path.display().to_string()));
        tabs.push(tab);
    }
    if primary {
        if let Some(diff) = state.diff().as_ref() {
            let title = format!(
                "{} (Diff)",
                diff.path.file_name().and_then(|n| n.to_str()).unwrap_or("untitled")
            );
            let mut tab = Tab::new(DIFF_TAB, title, Glyph::Diff);
            tab.selected = state.diff_active();
            tab.tooltip = Some(Tooltip::new(format!("Diff of {} against HEAD", diff.path.display())));
            tabs.push(tab);
        }
        let host = state.plugin_host_service();
        for index in 0..host.tab_count() {
            let title = host.tab_title(index).unwrap_or_default();
            let mut tab = Tab::new(plugin_tab_id(index), title, Glyph::Package);
            tab.selected = host.active_tab() == Some(index);
            tabs.push(tab);
        }
    }
    TabBar::new("editor", tabs)
}

/// The tab of the active document, for tests.
pub fn selected(bar: &TabBar) -> Option<&Tab> {
    bar.tabs.iter().find(|t| t.selected)
}

/// What a tab event means as a command, when it is one: select, close, unpin, reorder.
pub fn command_for(cx: &ViewCx, event: &TabEvent) -> Option<(String, serde_json::Value)> {
    use serde_json::json;
    match event {
        TabEvent::Select { id } if id == DIFF_TAB => Some(("diff.activate".into(), json!({}))),
        TabEvent::Select { id } if id.starts_with("plugin:") => None,
        TabEvent::Select { id } => {
            document_index(cx, id).map(|index| ("editor.activate".into(), json!({ "index": index })))
        }
        TabEvent::Close { id } if id == DIFF_TAB => Some(("diff.close".into(), json!({}))),
        TabEvent::Close { id } => {
            document_index(cx, id).map(|index| ("editor.close".into(), json!({ "index": index })))
        }
        _ => None,
    }
}
