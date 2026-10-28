//! The status bar (TUI-GUI-094, 105..109): breadcrumbs at the left, the widgets at the right in
//! the GUI's order, each with its tone, tooltip, priority and menu.

use crate::cx::ViewCx;
use crate::toolbar::command_row;
use archipel_ide_core::{Anchor, UpdateState};
use archipel_ide_index::{IndexStatus, Phase};
use archipel_ide_language::{StructureItem, SymbolKind, structure};
use archipel_ide_lsp::{ServerInfo, ServerStatus, describe_progress};
use archipel_kit_model::{
    Breadcrumbs, Crumb, Glyph, IconKey, MenuEntry, MenuItem, StatusBar, StatusBarItem, Tone,
    Tooltip,
};

/// Ids of the status bar's widgets (stable, for events and tests).
pub mod ids {
    /// The status message.
    pub const MESSAGE: &str = "message";
    /// The busy spinner.
    pub const SPINNER: &str = "spinner";
    /// `Update <version>`.
    pub const UPDATE: &str = "update";
    /// Column mode.
    pub const COLUMN: &str = "column";
    /// Read-only.
    pub const READ_ONLY: &str = "read_only";
    /// Caret position.
    pub const CARET: &str = "caret";
    /// Line separator.
    pub const SEPARATOR: &str = "line_separator";
    /// Encoding.
    pub const ENCODING: &str = "encoding";
    /// Indent.
    pub const INDENT: &str = "indent";
    /// External system (project import).
    pub const EXTERNAL: &str = "external";
    /// Toolchain setup.
    pub const TOOLCHAIN: &str = "toolchain";
    /// Indexing.
    pub const INDEX: &str = "index";
    /// Language servers.
    pub const LSP: &str = "lsp";
    /// Agents.
    pub const AGENTS: &str = "agents";
    /// AI chat.
    pub const AI: &str = "ai";
}

/// A declaration around the caret.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeCrumb {
    /// What it is.
    pub kind: SymbolKind,
    /// Its short label.
    pub name: String,
    /// Zero-based line of its name.
    pub line: usize,
    /// Zero-based column of its name.
    pub column: usize,
}

/// Short breadcrumb label: qualified names keep their last segment, long ones are cut.
fn crumb_label(name: &str) -> String {
    let name = name.lines().next().unwrap_or_default().trim();
    let short = if name.len() > 24 && name.contains('.') && !name.contains(' ') {
        name.rsplit('.').next().unwrap_or(name)
    } else {
        name
    };
    if short.chars().count() > 32 {
        format!("{}…", short.chars().take(31).collect::<String>())
    } else {
        short.to_owned()
    }
}

/// The declarations around the caret of `document`, outermost first, from its structure.
pub fn code_crumbs(document: &archipel_ide_core::Document, items: &[StructureItem]) -> Vec<CodeCrumb> {
    let byte = document.text.char_to_byte(document.selections.primary().head);
    structure::path_at(items, byte)
        .into_iter()
        .map(|item| {
            let line_start = document.text.line_to_char(item.line).unwrap_or(0);
            let column = document
                .text
                .byte_to_char(item.name_range.start)
                .saturating_sub(line_start);
            CodeCrumb {
                kind: item.kind,
                name: crumb_label(&item.name),
                line: item.line,
                column,
            }
        })
        .collect()
}

/// The model icon of a structure kind.
pub fn symbol_icon(kind: SymbolKind) -> IconKey {
    let name = match kind {
        SymbolKind::Class => "class",
        SymbolKind::Interface => "interface",
        SymbolKind::Enum => "enum",
        SymbolKind::Function | SymbolKind::Macro => "function",
        SymbolKind::Method => "method",
        SymbolKind::Constructor => "constructor",
        SymbolKind::Field => "field",
        SymbolKind::Constant => "constant",
        SymbolKind::Module => "module",
        SymbolKind::Type => "struct",
        SymbolKind::Key => "property",
        SymbolKind::Tag => return Glyph::Code.into(),
        SymbolKind::Heading => return Glyph::FileText.into(),
        SymbolKind::Other => "variable",
    };
    IconKey::Symbol(name.to_owned())
}

/// The id of path crumb `index`.
pub fn path_crumb_id(index: usize) -> String {
    format!("path:{index}")
}

/// The id of code crumb `index`.
pub fn code_crumb_id(index: usize) -> String {
    format!("code:{index}")
}

/// Navigation bar breadcrumbs: project, directories, file, then the declarations around the
/// caret. A top-level declaration named like the file stands in for the file.
pub fn breadcrumbs(cx: &ViewCx) -> Breadcrumbs {
    let state = cx.state;
    let project = crate::toolbar::project_name(cx);
    let mut first = Crumb::new(path_crumb_id(0), project);
    first.icon = Some(Glyph::Project.into());
    let mut crumbs = vec![first];
    let Some(document) = state.active_doc() else {
        return Breadcrumbs { crumbs };
    };
    let components: Vec<String> = document
        .path
        .components()
        .map(|part| part.as_os_str().to_string_lossy().into_owned())
        .collect();
    let code = &cx.ui.code_crumbs;
    let stem = document.path.file_stem().map(|s| s.to_string_lossy().into_owned());
    let file_replaced = code.first().map(|c| c.name.as_str()) == stem.as_deref() && stem.is_some();
    let shown = if file_replaced { components.len() - 1 } else { components.len() };
    for (i, part) in components.iter().take(shown).enumerate() {
        let mut crumb = Crumb::new(path_crumb_id(i + 1), part.clone());
        if i + 1 == components.len() {
            crumb.icon = Some(IconKey::File(document.path.display().to_string()));
            crumb.tone = Tone::Default;
        }
        crumbs.push(crumb);
    }
    for (i, code) in code.iter().enumerate() {
        let mut crumb = Crumb::new(code_crumb_id(i), code.name.clone());
        crumb.icon = Some(symbol_icon(code.kind));
        crumb.tone = Tone::Default;
        crumbs.push(crumb);
    }
    Breadcrumbs { crumbs }
}

/// First line of `text`, shortened to `max` characters with an ellipsis.
pub fn first_line_truncated(text: &str, max: usize) -> String {
    let line = text.lines().next().unwrap_or_default();
    if line.chars().count() > max {
        let mut short: String = line.chars().take(max.saturating_sub(1)).collect();
        short.push('…');
        short
    } else {
        line.to_owned()
    }
}

/// Caret position widget text: `line:column`, the selected character and line break counts
/// for a selection, or the caret count with several carets.
pub fn caret_position_label(
    text: &archipel_ide_text::TextDocument,
    selections: &archipel_ide_text::SelectionSet,
    (line, column): (usize, usize),
) -> String {
    let count = selections.selections().len();
    if count > 1 {
        return format!("{count} carets");
    }
    let primary = selections.primary();
    let position = format!("{}:{}", line + 1, column + 1);
    if primary.is_caret() {
        return position;
    }
    let chars = primary.end() - primary.start();
    let breaks = text.char_to_line(primary.end()) - text.char_to_line(primary.start());
    let chars_label = if chars == 1 { "char" } else { "chars" };
    match breaks {
        0 => format!("{position} ({chars} {chars_label})"),
        1 => format!("{position} ({chars} {chars_label}, 1 line break)"),
        n => format!("{position} ({chars} {chars_label}, {n} line breaks)"),
    }
}

/// Runs shorter than this many files show no indexing widget.
const MIN_VISIBLE_FILES: u32 = 10;

/// The text of a busy indexer, or `None` when nothing should show.
pub fn index_summary(status: &IndexStatus) -> Option<String> {
    if status.phase == Phase::Idle && !status.dumb {
        return None;
    }
    if !status.dumb && status.phase == Phase::Indexing && status.total < MIN_VISIBLE_FILES {
        return None;
    }
    status
        .summary()
        .or_else(|| status.dumb.then(|| "Indexing…".to_owned()))
}

/// Short language server status: (text, busy, problem), `None` when there is no server for
/// the active file and no background work.
pub fn lsp_summary(cx: &ViewCx, servers: &[ServerInfo]) -> Option<(String, bool, bool)> {
    let state = cx.state;
    if let Some((server, progress)) = servers
        .iter()
        .find_map(|server| Some((server, server.progress.first()?)))
    {
        return Some((format!("{}: {}", server.id, describe_progress(progress)), true, false));
    }
    let active = state.active_absolute_path()?;
    let (provider, _) = state.lsp().registry().for_path(&active)?;
    let id = provider.spec().id;
    if state.lsp().missing_servers().any(|missing| missing == id) {
        return Some((format!("{id}: not installed"), false, true));
    }
    let server = servers
        .iter()
        .filter(|server| server.id == id && active.starts_with(&server.root))
        .max_by_key(|server| server.root.components().count())?;
    Some(match &server.status {
        ServerStatus::Starting => (
            match &server.message {
                Some(message) => format!("{id}: {message}"),
                None => format!("{id}: starting…"),
            },
            true,
            false,
        ),
        ServerStatus::Ready if server.importing => (format!("{id}: importing the project…"), true, false),
        ServerStatus::Ready => (String::new(), false, false),
        ServerStatus::Restarting { attempt } => (format!("{id}: restarting ({attempt})…"), true, true),
        ServerStatus::Failed { .. } => (format!("{id}: failed"), false, true),
    })
}

fn lsp_tooltip(cx: &ViewCx, servers: &[ServerInfo]) -> Tooltip {
    let mut lines = Vec::new();
    if servers.is_empty() {
        lines.push("No language server is running.".to_owned());
    }
    for server in servers {
        let state = match &server.status {
            ServerStatus::Starting => "starting".to_owned(),
            ServerStatus::Ready if server.importing => "importing the project".to_owned(),
            ServerStatus::Ready => "ready".to_owned(),
            ServerStatus::Restarting { attempt } => format!("restarting (attempt {attempt})"),
            ServerStatus::Failed { reason } => {
                format!("failed: {}", reason.lines().next().unwrap_or_default())
            }
        };
        lines.push(format!("{} ({}) — {state}", server.id, server.language));
        for progress in &server.progress {
            lines.push(describe_progress(progress));
        }
    }
    for missing in cx.state.lsp().missing_servers() {
        lines.push(format!("{missing} — not installed"));
    }
    Tooltip::new("Language servers").description(lines.join("\n"))
}

fn item(id: &str, text: impl Into<String>) -> StatusBarItem {
    StatusBarItem::new(id, text)
}

/// The status bar for the state: breadcrumbs and the widgets in the GUI's visual order.
pub fn status_bar(cx: &ViewCx) -> StatusBar {
    let state = cx.state;
    let mut items: Vec<StatusBarItem> = Vec::new();

    // Status message and spinner.
    if !state.status().is_empty() {
        let mut message = item(ids::MESSAGE, first_line_truncated(state.status(), 70));
        message.tone = Tone::Info;
        message.elide = true;
        message.priority = 30;
        message.tooltip = Some(Tooltip::new(state.status().to_owned()));
        message.clickable = state.tool_windows().visible(Anchor::Bottom).is_none();
        // The GUI places the spinner before the message in the right-to-left layout.
        items.push(message);
    }
    if state.execution().is_running() || state.search_running() {
        let mut spinner = item(ids::SPINNER, "");
        spinner.spinner = true;
        spinner.clickable = false;
        spinner.priority = 31;
        items.push(spinner);
    }
    let update = state.update_state();
    if let UpdateState::Available { version, .. } | UpdateState::Installed(version) = &update {
        let label = if matches!(update, UpdateState::Installed(_)) {
            "Restart to update".to_owned()
        } else {
            format!("Update {version}")
        };
        let mut u = item(ids::UPDATE, label);
        u.icon = Some(Glyph::Refresh.into());
        u.tone = Tone::Modified;
        u.priority = 75;
        items.push(u);
    }
    if let Some(document) = state.active_doc() {
        if document.selections.column_mode() {
            let mut column = item(ids::COLUMN, "Column");
            column.clickable = false;
            column.priority = 40;
            items.push(column);
        }
        if cx.ui.read_only {
            let mut ro = item(ids::READ_ONLY, "Read-only");
            ro.tone = Tone::Warning;
            ro.priority = 70;
            ro.tooltip = Some(Tooltip::new("File is read-only"));
            items.push(ro);
        }
        let caret = document.selections.primary().head;
        let (line, column) = document.text.char_to_line_column(caret).unwrap_or((0, 0));
        let mut c = item(
            ids::CARET,
            caret_position_label(&document.text, &document.selections, (line, column)),
        );
        c.priority = 90;
        c.tooltip = Some(Tooltip::new("Go to Line:Column"));
        items.push(c);
        let separator = archipel_ide_text::format::detect_line_separator(&document.text);
        let mut s = item(ids::SEPARATOR, separator.label());
        s.tooltip = Some(Tooltip::new("Line separator"));
        s.clickable = false;
        s.priority = 55;
        items.push(s);
        let mut e = item(ids::ENCODING, "UTF-8");
        e.clickable = false;
        e.priority = 56;
        items.push(e);
        let indent = archipel_ide_text::format::detect_indent(&document.text)
            .unwrap_or(archipel_ide_text::format::Indent::Spaces(4));
        let mut i = item(ids::INDENT, indent.label());
        i.clickable = false;
        i.priority = 65;
        items.push(i);
    }
    // External system (project import).
    if let Some(model) = state.project_model() {
        let (progress, busy) = {
            let model = model.read();
            (model.progress_text(), model.is_busy())
        };
        if let Some(text) = progress.or_else(|| busy.then(|| "Build files changed…".to_owned())) {
            let mut e = item(ids::EXTERNAL, text);
            e.spinner = true;
            e.priority = 20;
            e.tooltip = Some(Tooltip::new(
                "Importing the project model from the build files. Click to cancel.",
            ));
            e.menu = vec![MenuEntry::Item(MenuItem::new(
                command_row("externalSystem.cancel", &serde_json::json!({})),
                "Cancel Import",
            ))];
            items.push(e);
        }
    }
    // Toolchain setup.
    if let Some((text, fraction)) = state.toolchains().read().progress() {
        let mut t = item(ids::TOOLCHAIN, text);
        match fraction {
            Some(fraction) => t.progress = Some(fraction),
            None => t.spinner = true,
        }
        t.priority = 22;
        t.tooltip = Some(Tooltip::new(
            "Toolchain setup in progress. Click to cancel; the download resumes next time.",
        ));
        t.menu = vec![MenuEntry::Item(MenuItem::new(
            command_row("toolchain.cancel", &serde_json::json!({})),
            "Cancel Setup",
        ))];
        items.push(t);
    }
    // Indexing.
    if state.analysis().index.is_attached() {
        let status = state.index_status();
        if let Some(text) = index_summary(&status) {
            let mut x = item(ids::INDEX, text);
            x.tone = Tone::Info;
            x.spinner = true;
            x.priority = 50;
            let mut lines = Vec::new();
            if status.dumb {
                lines.push("Some features are limited until indexing finishes.".to_owned());
            }
            for info in &status.indexes {
                let state = match info.state {
                    archipel_ide_analysis::IndexState::Building { done, total } => {
                        format!("building {done} / {total}")
                    }
                    other => archipel_ide_index::progress::state_label(other).to_owned(),
                };
                lines.push(format!("{} — {state}", info.id.0));
            }
            x.tooltip = Some(Tooltip::new("Indexing").description(lines.join("\n")));
            x.menu = vec![MenuEntry::Item(MenuItem::new(
                command_row("index.rebuild", &serde_json::json!({})),
                "Rebuild Indexes",
            ))];
            items.push(x);
        }
    }
    // Language servers.
    let servers = state.lsp().servers();
    if let Some((text, busy, problem)) = lsp_summary(cx, &servers) {
        let mut l = item(ids::LSP, text.clone());
        l.priority = 85;
        if problem {
            l.icon = Some(Glyph::Warning.into());
        } else if text.is_empty() {
            l.icon = Some(Glyph::Success.into());
        }
        l.tone = if busy && !problem { Tone::Info } else { Tone::Secondary };
        l.spinner = busy;
        l.tooltip = Some(lsp_tooltip(cx, &servers));
        l.menu = vec![MenuEntry::Item(MenuItem::new(
            command_row("language.restartServers", &serde_json::Value::Null),
            "Restart Language Servers",
        ))];
        items.push(l);
    }
    // Agents.
    if !state.agents_manager().sessions().is_empty() {
        let counts = state.agents_manager().counts();
        let text = match counts.status_text() {
            text if text.is_empty() => "Agents".to_owned(),
            text => text,
        };
        let mut a = item(ids::AGENTS, text);
        a.icon = Some(Glyph::Agent.into());
        a.tone = if counts.needs_input > 0 {
            Tone::Warning
        } else if counts.failed > 0 {
            Tone::Error
        } else if counts.working > 0 {
            Tone::Run
        } else {
            Tone::Secondary
        };
        a.priority = 14;
        a.tooltip = Some(Tooltip::new("Agents"));
        items.push(a);
    }
    // AI chat.
    if state.has_project() {
        let ai = state.ai();
        let (text, tooltip) = if !ai.policy.allow_ai {
            (
                String::new(),
                format!(
                    "AI is disabled for this project ({})",
                    archipel_ide_ai::settings::PROJECT_POLICY_FILE
                ),
            )
        } else if !ai.is_enabled() {
            (
                String::new(),
                "AI Assistant is off — click to set up a provider".to_owned(),
            )
        } else if ai.chat.is_streaming() {
            ("AI: generating…".to_owned(), ai.summary())
        } else {
            (String::new(), format!("AI Assistant: {}", ai.summary()))
        };
        let mut w = item(ids::AI, text);
        w.icon = Some(Glyph::Sparkle.into());
        w.tone = if ai.is_enabled() { Tone::Secondary } else { Tone::Info };
        w.priority = 12;
        w.tooltip = Some(Tooltip::new(tooltip));
        items.push(w);
    }
    // Plugin widgets.
    for (n, widget) in state.plugin_status_widgets().into_iter().enumerate() {
        let mut w = item(&format!("plugin:{n}"), widget.state.text.clone());
        w.priority = 10;
        w.clickable = widget.command.is_some();
        w.tooltip = widget.state.tooltip.clone().map(Tooltip::new);
        items.push(w);
    }
    StatusBar {
        breadcrumbs: breadcrumbs(cx),
        items,
    }
}
