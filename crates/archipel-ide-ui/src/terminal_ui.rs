//! Interactive PTY grid. Filesystem and session actions belong to typed commands.
use crate::{
    IdeApp,
    icons::{self, Icon},
    theme,
    widgets::tool_window_header,
};
use archipel_ide_terminal::Color;
use eframe::egui::{self, Color32, Rect, Sense, Stroke, vec2};
use serde_json::{Value, json};

pub type GridPoint = (u16, u16);
pub fn input_id() -> egui::Id {
    egui::Id::new("terminal_input")
}

pub fn show(app: &mut IdeApp, ui: &mut egui::Ui) {
    let mut restart = false;
    let mut clear = false;
    let exited = app
        .state
        .terminal()
        .session()
        .as_ref()
        .is_some_and(|t| t.is_exited());
    let ai_available = app.action_enabled("ai.terminal.suggest");
    let ai_open = app.ai_ui.terminal_open;
    let mut ai_toggle = false;
    let tabs = app.state.terminal_tab_titles();
    let mut tab_action: Option<(&'static str, Value)> = None;
    let hide = tool_window_header(ui, "Terminal", |ui| {
        // `Local ×` tabs with `+` and a session list.
        for (index, (title, active)) in tabs.iter().enumerate() {
            let label = egui::RichText::new(title).size(theme::UI_FONT_SIZE);
            if ui.selectable_label(*active, label).clicked() && !active {
                tab_action = Some(("terminal.selectTab", json!({ "index": index })));
            }
            if tabs.len() > 1 && icons::button(ui, Icon::Close, "Close Session").clicked() {
                tab_action = Some(("terminal.closeTab", json!({ "index": index })));
            }
        }
        if icons::button(ui, Icon::Plus, "New Session").clicked() {
            tab_action = Some(("terminal.newTab", Value::Null));
        }
        if tabs.len() > 1 {
            ui.menu_button("▾", |ui| {
                for (index, (title, active)) in tabs.iter().enumerate() {
                    if ui.selectable_label(*active, title).clicked() {
                        tab_action = Some(("terminal.selectTab", json!({ "index": index })));
                        ui.close();
                    }
                }
            });
        }
        ui.separator();
        if exited {
            ui.label(egui::RichText::new("process exited").color(theme::TEXT_INFO));
        }
        let error = app
            .state
            .terminal()
            .session()
            .as_ref()
            .and_then(|t| t.error().map(str::to_owned));
        if let Some(error) = error {
            ui.label(egui::RichText::new(error).color(theme::ERROR));
        }
        clear = ui.button("Clear").clicked();
        restart = icons::button(ui, Icon::Refresh, "Restart shell").clicked();
        ai_toggle = crate::ai_terminal::header_button(ui, ai_available, ai_open);
    });
    if hide {
        app.toggle_tool_window(archipel_ide_core::ToolWindow::Terminal);
        return;
    }
    if let Some((id, params)) = tab_action {
        app.dispatch(id, params);
        app.terminal_selection = None;
    }
    if restart {
        app.dispatch("terminal.restart", Value::Null);
        app.terminal_selection = None;
    }
    if clear {
        app.dispatch("terminal.clear", Value::Null);
        app.terminal_selection = None;
    }
    if ai_toggle {
        crate::ai_terminal::toggle(app);
    }
    crate::ai_terminal::bar(app, ui);
    let font = egui::FontId::monospace(theme::EDITOR_FONT_SIZE);
    let (cell_width, cell_height) =
        ui.fonts_mut(|fonts| (fonts.glyph_width(&font, 'M'), fonts.row_height(&font)));
    let available = ui.available_rect_before_wrap().shrink2(vec2(8.0, 4.0));
    let rows = (available.height() / cell_height).floor().clamp(1.0, 200.0) as u16;
    let cols = (available.width() / cell_width).floor().clamp(1.0, 500.0) as u16;
    if app
        .state
        .terminal()
        .session()
        .as_ref()
        .is_some_and(|t| t.screen().size() != (rows, cols))
    {
        app.dispatch("terminal.resize", json!({"rows": rows,"cols": cols}));
        app.terminal_selection = None;
    }
    let rect = Rect::from_min_size(
        available.min,
        vec2(cols as f32 * cell_width, rows as f32 * cell_height),
    );
    ui.allocate_rect(rect, Sense::hover());
    let response = ui.interact(rect, input_id(), Sense::click_and_drag());
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, ui.is_enabled(), "Terminal")
    });
    if response.clicked() || response.drag_started() {
        response.request_focus();
    }
    ui.memory_mut(|memory| {
        memory.set_focus_lock_filter(
            input_id(),
            egui::EventFilter {
                tab: true,
                horizontal_arrows: true,
                vertical_arrows: true,
                escape: true,
            },
        )
    });
    let point = |position: egui::Pos2| -> GridPoint {
        (
            ((position.y - rect.top()) / cell_height)
                .floor()
                .clamp(0.0, rows.saturating_sub(1) as f32) as u16,
            ((position.x - rect.left()) / cell_width)
                .floor()
                .clamp(0.0, cols.saturating_sub(1) as f32) as u16,
        )
    };
    if response.drag_started()
        && let Some(origin) = ui.input(|i| i.pointer.press_origin())
    {
        let start = point(origin);
        app.terminal_selection = Some((start, start));
    } else if response.clicked() {
        app.terminal_selection = None;
    }
    if response.dragged()
        && let Some(position) = response.interact_pointer_pos()
    {
        let end = point(position);
        if let Some((_, head)) = app.terminal_selection.as_mut() {
            *head = end;
        }
    }
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0, theme::ISLAND);
    let terminal = app.state.terminal();
    let session = terminal.session();
    let Some(terminal) = session.as_ref() else {
        return;
    };
    let screen = terminal.screen();
    let selection = app
        .terminal_selection
        .map(|selection| selection_bounds(selection, cols));
    for row in 0..rows {
        for col in 0..cols {
            let Some(cell) = screen.cell(row, col) else {
                continue;
            };
            if cell.is_wide_continuation() {
                continue;
            }
            let cell_rect = Rect::from_min_size(
                rect.min + vec2(col as f32 * cell_width, row as f32 * cell_height),
                vec2(
                    cell_width * if cell.is_wide() { 2.0 } else { 1.0 },
                    cell_height,
                ),
            );
            let mut fg = ansi_color(cell.fgcolor(), *theme::editor::TEXT, cell.bold());
            let mut bg = ansi_color(cell.bgcolor(), *theme::ISLAND, false);
            if cell.inverse() {
                std::mem::swap(&mut fg, &mut bg);
            }
            if selection.is_some_and(|(start, end)| (row, col) >= start && (row, col) < end) {
                bg = *theme::editor::SELECTION;
            }
            if bg != *theme::ISLAND {
                painter.rect_filled(cell_rect, 0, bg);
            }
            if !cell.contents().is_empty() {
                painter.text(
                    cell_rect.min,
                    egui::Align2::LEFT_TOP,
                    cell.contents(),
                    font.clone(),
                    fg,
                );
            }
            if cell.underline() {
                painter.line_segment(
                    [cell_rect.left_bottom(), cell_rect.right_bottom()],
                    Stroke::new(1.0_f32, fg),
                );
            }
        }
    }
    if response.has_focus() && !screen.hide_cursor() && screen.scrollback() == 0 {
        let (row, col) = screen.cursor_position();
        let cursor = Rect::from_min_size(
            rect.min + vec2(col as f32 * cell_width, row as f32 * cell_height),
            vec2(cell_width, cell_height),
        );
        painter.rect_stroke(
            cursor,
            0,
            Stroke::new(1.0_f32, theme::TEXT_BRIGHT),
            egui::StrokeKind::Inside,
        );
    }
    let application_cursor = screen.application_cursor();
    drop(session);
    if response.hovered() {
        let delta = ui.input(|i| i.smooth_scroll_delta.y);
        if delta.abs() > 1.0 {
            let lines = (delta / cell_height).round() as i64;
            if lines != 0 {
                app.dispatch("terminal.scroll", json!({"lines":lines}));
                app.terminal_selection = None;
            }
        }
    }
    if !response.has_focus() || app.dialog.is_some() || app.palette.open || app.find_popup.open {
        return;
    }
    let events = ui.input_mut(|input| {
        let mut events = Vec::new();
        input.events.retain(|event| {
            if matches!(
                event,
                egui::Event::Text(_)
                    | egui::Event::Paste(_)
                    | egui::Event::Copy
                    | egui::Event::Cut
                    | egui::Event::Key { pressed: true, .. }
            ) {
                events.push(event.clone());
                false
            } else {
                true
            }
        });
        events
    });
    let mut text = String::new();
    for event in events {
        match event {
            egui::Event::Copy => {
                if cfg!(target_os = "macos")
                    || ui.input(|input| input.modifiers.shift)
                    || app.terminal_selection.is_some()
                {
                    copy_selection(app, ui.ctx());
                } else {
                    text.push('\u{3}');
                }
            }
            egui::Event::Cut if !cfg!(target_os = "macos") => text.push('\u{18}'),
            egui::Event::Paste(paste) => {
                if !text.is_empty() {
                    app.dispatch(
                        "terminal.sendText",
                        json!({"text":std::mem::take(&mut text)}),
                    );
                }
                app.dispatch("terminal.paste", json!({"text":paste}));
            }
            egui::Event::Text(value) => text.push_str(&value),
            egui::Event::Key {
                key,
                modifiers,
                pressed: true,
                ..
            } => {
                if modifiers.ctrl && modifiers.shift && key == egui::Key::C {
                    copy_selection(app, ui.ctx());
                } else if let Some(value) = key_sequence(key, modifiers, application_cursor) {
                    text.push_str(&value);
                }
            }
            _ => {}
        }
    }
    if !text.is_empty() {
        app.terminal_selection = None;
        app.dispatch("terminal.sendText", json!({"text":text}));
    }
}

fn copy_selection(app: &mut IdeApp, ctx: &egui::Context) {
    if let Some(selection) = app.terminal_selection {
        let Some(cols) = app
            .state
            .terminal()
            .session()
            .as_ref()
            .map(|terminal| terminal.screen().size().1)
        else {
            return;
        };
        let (start, end) = selection_bounds(selection, cols);
        match app.commands.execute(
            "terminal.copy",
            &mut app.state,
            json!({"startRow":start.0,"startCol":start.1,"endRow":end.0,"endCol":end.1}),
        ) {
            Ok(result) => {
                if let Some(text) = result["text"].as_str() {
                    ctx.copy_text(text.to_owned());
                }
            }
            Err(error) => app.state.set_status(error.to_string()),
        }
    }
}

fn selection_bounds((anchor, head): (GridPoint, GridPoint), cols: u16) -> (GridPoint, GridPoint) {
    let (start, last) = if anchor <= head {
        (anchor, head)
    } else {
        (head, anchor)
    };
    (start, (last.0, last.1.saturating_add(1).min(cols)))
}

pub fn key_sequence(key: egui::Key, m: egui::Modifiers, application: bool) -> Option<String> {
    use egui::Key;
    // macOS Command chords belong to the IDE/clipboard, not the shell.
    if m.mac_cmd {
        return None;
    }
    if m.ctrl {
        let letters = [
            Key::A,
            Key::B,
            Key::C,
            Key::D,
            Key::E,
            Key::F,
            Key::G,
            Key::H,
            Key::I,
            Key::J,
            Key::K,
            Key::L,
            Key::M,
            Key::N,
            Key::O,
            Key::P,
            Key::Q,
            Key::R,
            Key::S,
            Key::T,
            Key::U,
            Key::V,
            Key::W,
            Key::X,
            Key::Y,
            Key::Z,
        ];
        if let Some(index) = letters.iter().position(|candidate| *candidate == key) {
            return Some(char::from(index as u8 + 1).to_string());
        }
        if key == Key::Space {
            return Some("\0".into());
        }
    }
    let sequence = match key {
        Key::Enter => "\r",
        Key::Backspace => "\u{7f}",
        Key::Tab if m.shift => "\u{1b}[Z",
        Key::Tab => "\t",
        Key::Escape => "\u{1b}",
        Key::ArrowLeft if m.alt => "\u{1b}b",
        Key::ArrowRight if m.alt => "\u{1b}f",
        Key::ArrowLeft if m.ctrl => "\u{1b}[1;5D",
        Key::ArrowRight if m.ctrl => "\u{1b}[1;5C",
        Key::ArrowUp if application => "\u{1b}OA",
        Key::ArrowDown if application => "\u{1b}OB",
        Key::ArrowRight if application => "\u{1b}OC",
        Key::ArrowLeft if application => "\u{1b}OD",
        Key::ArrowUp => "\u{1b}[A",
        Key::ArrowDown => "\u{1b}[B",
        Key::ArrowRight => "\u{1b}[C",
        Key::ArrowLeft => "\u{1b}[D",
        Key::Home => "\u{1b}[H",
        Key::End => "\u{1b}[F",
        Key::Delete => "\u{1b}[3~",
        Key::Insert => "\u{1b}[2~",
        Key::PageUp => "\u{1b}[5~",
        Key::PageDown => "\u{1b}[6~",
        Key::F1 => "\u{1b}OP",
        Key::F2 => "\u{1b}OQ",
        Key::F3 => "\u{1b}OR",
        Key::F4 => "\u{1b}OS",
        Key::F5 => "\u{1b}[15~",
        Key::F6 => "\u{1b}[17~",
        Key::F7 => "\u{1b}[18~",
        Key::F8 => "\u{1b}[19~",
        Key::F9 => "\u{1b}[20~",
        Key::F10 => "\u{1b}[21~",
        Key::F11 => "\u{1b}[23~",
        Key::F12 => "\u{1b}[24~",
        _ => return None,
    };
    Some(sequence.into())
}

fn ansi_color(color: Color, default: Color32, bold: bool) -> Color32 {
    const COLORS: [[u8; 3]; 16] = [
        [0, 0, 0],
        [205, 49, 49],
        [13, 188, 121],
        [229, 229, 16],
        [36, 114, 200],
        [188, 63, 188],
        [17, 168, 205],
        [229, 229, 229],
        [102, 102, 102],
        [241, 76, 76],
        [35, 209, 139],
        [245, 245, 67],
        [59, 142, 234],
        [214, 112, 214],
        [41, 184, 219],
        [255, 255, 255],
    ];
    match color {
        Color::Default => default,
        Color::Rgb(r, g, b) => Color32::from_rgb(r, g, b),
        Color::Idx(index) => {
            let index = if bold && index < 8 { index + 8 } else { index };
            let [r, g, b] = if index < 16 {
                COLORS[index as usize]
            } else if index < 232 {
                let n = index - 16;
                let level = |v: u8| if v == 0 { 0 } else { 55 + 40 * v };
                [level(n / 36), level(n / 6 % 6), level(n % 6)]
            } else {
                let gray = 8 + 10 * (index - 232);
                [gray, gray, gray]
            };
            Color32::from_rgb(r, g, b)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn forward_and_backward_cell_selection_copy_the_same_text() {
        assert_eq!(selection_bounds(((0, 0), (0, 2)), 80), ((0, 0), (0, 3)));
        assert_eq!(selection_bounds(((0, 2), (0, 0)), 80), ((0, 0), (0, 3)));
    }
    #[test]
    fn forwards_shell_keys_and_application_arrows() {
        assert_eq!(
            key_sequence(egui::Key::C, egui::Modifiers::CTRL, false).as_deref(),
            Some("\u{3}")
        );
        assert_eq!(
            key_sequence(egui::Key::R, egui::Modifiers::CTRL, false).as_deref(),
            Some("\u{12}")
        );
        assert_eq!(
            key_sequence(egui::Key::Tab, egui::Modifiers::NONE, false).as_deref(),
            Some("\t")
        );
        assert_eq!(
            key_sequence(egui::Key::ArrowUp, egui::Modifiers::NONE, false).as_deref(),
            Some("\u{1b}[A")
        );
        assert_eq!(
            key_sequence(egui::Key::ArrowUp, egui::Modifiers::NONE, true).as_deref(),
            Some("\u{1b}OA")
        );
    }
}
