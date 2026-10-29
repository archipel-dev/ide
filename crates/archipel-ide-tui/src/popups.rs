//! The popups at the caret: code completion and the Alt+Enter actions. Their state lives in
//! the core (`state.completion()`, `state.code_actions()`); this draws it and sends the keys
//! back as commands.

use crate::app::TuiApp;
use crate::draw::{clip, fill, put, width};
use archipel_ide_view::popups::chooser;
use archipel_ide_view::popups::completion::{self as completion_model, CompletionRow};
use archipel_ide_view::popups::intentions::{self as intention_model, Choice, IntentionRow};
use archipel_kit_tui::input::{KeyCode, KeyEvent, KeyModifiers};
use archipel_kit_tui::ratatui::buffer::Buffer;
use archipel_kit_tui::ratatui::layout::Rect;
use archipel_kit_tui::ratatui::style::{Modifier, Style};
use archipel_kit_tui::runtime::Runtime;
use serde_json::json;

/// Where the popups go and which row of the actions popup is selected.
#[derive(Default)]
pub struct Popups {
    pub caret_cell: Option<(u16, u16)>,
    pub editor_area: Rect,
    pub intention_selected: usize,
    intention_open: bool,
    /// The submenu of the actions popup that is open.
    pub intention_expanded: Option<String>,
    /// The selected row of the chooser of several navigation targets.
    pub choice_selected: usize,
}

const MAX_ROWS: usize = 10;

impl TuiApp {
    /// Keys of an open popup. Returns whether the popup took the key.
    pub(crate) fn popup_key(&mut self, rt: &mut Runtime, key: &KeyEvent) -> bool {
        let plain = !key
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT);
        if !plain || !self.layers.is_empty() || self.shell.tool_focused() {
            return false;
        }
        let completion_open = self
            .ide
            .state()
            .completion()
            .as_ref()
            .is_some_and(|session| session.visible());
        if completion_open {
            let command = match key.code {
                KeyCode::Down => Some(("completion.move", json!({ "delta": 1 }))),
                KeyCode::Up => Some(("completion.move", json!({ "delta": -1 }))),
                KeyCode::PageDown => Some(("completion.move", json!({ "delta": MAX_ROWS as i64 }))),
                KeyCode::PageUp => {
                    Some(("completion.move", json!({ "delta": -(MAX_ROWS as i64) })))
                }
                KeyCode::Enter => Some(("completion.accept", json!({ "replace": false }))),
                KeyCode::Tab => Some(("completion.accept", json!({ "replace": true }))),
                KeyCode::Esc => Some(("completion.close", serde_json::Value::Null)),
                _ => None,
            };
            if let Some((id, params)) = command {
                self.command(rt, id, params);
                return true;
            }
        }
        if self.ide.state().code_actions().popup {
            let rows = self.intention_rows();
            if rows.is_empty() {
                // Still computing: Esc cancels, the rest waits.
                if key.code == KeyCode::Esc {
                    self.ide.state_mut().code_actions_mut().popup = false;
                    return true;
                }
                return false;
            }
            let count = rows.len();
            let selected = self.popups.intention_selected.min(count - 1);
            match key.code {
                KeyCode::Down => self.popups.intention_selected = (selected + 1) % count,
                KeyCode::Up => self.popups.intention_selected = (selected + count - 1) % count,
                KeyCode::Right => {
                    // Opens the submenu of the selected header.
                    if let IntentionRow::Submenu { title, open: false } = &rows[selected] {
                        self.popups.intention_expanded = Some(title.clone());
                    }
                }
                KeyCode::Left => {
                    // Closes the open submenu and selects its header.
                    if let Some(title) = self.popups.intention_expanded.take()
                        && let Some(at) = rows.iter().position(|row| {
                            matches!(row, IntentionRow::Submenu { title: t, .. } if *t == title)
                        })
                    {
                        self.popups.intention_selected = at;
                    }
                }
                KeyCode::Esc => self.ide.state_mut().code_actions_mut().popup = false,
                KeyCode::Enter => match intention_model::choose(&rows[selected]) {
                    Choice::Apply(index) => {
                        self.command(rt, "code.applyAlert", json!({ "index": index }));
                        self.ide.state_mut().code_actions_mut().popup = false;
                    }
                    Choice::Expand(next) => self.popups.intention_expanded = next,
                },
                _ => return false,
            }
            return true;
        }
        self.popups.intention_open = false;
        false
    }

    /// Keys of the chooser of several navigation targets. Returns whether it took the key.
    pub(crate) fn choices_key(&mut self, rt: &mut Runtime, key: &KeyEvent) -> bool {
        let plain = !key
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT);
        if !plain || !self.layers.is_empty() {
            return false;
        }
        let Some(list) = self.ide.state().navigation().choices.clone() else {
            self.popups.choice_selected = 0;
            return false;
        };
        let count = list.items.len();
        if count == 0 {
            return false;
        }
        let selected = self.popups.choice_selected.min(count - 1);
        match key.code {
            KeyCode::Down => self.popups.choice_selected = (selected + 1) % count,
            KeyCode::Up => self.popups.choice_selected = (selected + count - 1) % count,
            KeyCode::PageDown => self.popups.choice_selected = (selected + MAX_ROWS).min(count - 1),
            KeyCode::PageUp => self.popups.choice_selected = selected.saturating_sub(MAX_ROWS),
            KeyCode::Esc => self.ide.state_mut().navigation_mut().choices = None,
            KeyCode::Enter => self.open_choice(rt, selected),
            _ => return false,
        }
        true
    }

    /// Opens target `index` of the chooser and closes it.
    pub(crate) fn open_choice(&mut self, rt: &mut Runtime, index: usize) {
        let Some(item) = self
            .ide
            .state()
            .navigation()
            .choices
            .as_ref()
            .and_then(|list| list.items.get(index))
            .cloned()
        else {
            return;
        };
        self.ide.state_mut().navigation_mut().choices = None;
        let (command, params) = chooser::goto(&item);
        self.command(rt, command, params);
        self.focus_editor();
    }

    /// The rows of the actions popup, submenus included.
    pub(crate) fn intention_rows(&mut self) -> Vec<IntentionRow> {
        let expanded = self.popups.intention_expanded.clone();
        let Some(set) = self.ide.state_mut().current_alerts() else {
            return Vec::new();
        };
        intention_model::rows(&set, expanded.as_deref())
    }

    /// Draws the popup that is open, near the caret.
    pub(crate) fn draw_popups(&mut self, rt: &mut Runtime, buf: &mut Buffer) {
        let Some((cx, cy)) = self.popups.caret_cell else {
            return;
        };
        let area = self.popups.editor_area;
        if self.ide.state().navigation().choices.is_some() {
            self.draw_choices(rt, buf, area, (cx, cy));
            return;
        }
        self.popups.choice_selected = 0;
        if self.ide.state().code_actions().popup {
            if !self.popups.intention_open {
                self.popups.intention_open = true;
                self.popups.intention_selected = 0;
                self.popups.intention_expanded = None;
            }
            let rows = self.intention_rows();
            self.draw_intentions(rt, buf, area, (cx, cy), &rows);
            return;
        }
        self.popups.intention_open = false;
        let state = self.ide.state();
        let completion = state.completion();
        let Some(session) = completion.as_ref() else {
            return;
        };
        if !session.visible() {
            return;
        }
        let rows = completion_model::rows(session);
        let selected = session.selected;
        let p = rt.look.palette;
        let shown = rows.len().min(MAX_ROWS);
        let first = selected
            .saturating_sub(shown.saturating_sub(1))
            .min(rows.len().saturating_sub(shown));
        let first = if selected < first { selected } else { first };
        // Badge, label, signature, a gap, the tail at the right edge (UI-POP-030).
        let content_w = rows
            .iter()
            .map(|row| {
                width(&row.label)
                    + row.detail.as_deref().map_or(0, width)
                    + row.tail.as_deref().map_or(0, |tail| width(tail) + 2)
                    + 6
            })
            .max()
            .unwrap_or(20)
            .clamp(24, 90);
        let w = content_w.min(area.width);
        let h = shown as u16;
        let x = cx.min(area.right().saturating_sub(w)).max(area.x);
        let mut y = cy + 1;
        if y + h > area.bottom() {
            y = cy.saturating_sub(h);
        }
        let rect = Rect::new(x, y, w, h);
        fill(buf, rect, Style::default().bg(p.pmenu_bg).fg(p.white));
        for (offset, index) in (first..rows.len()).take(shown).enumerate() {
            let row = &rows[index];
            let cy = rect.y + offset as u16;
            let chosen = index == selected;
            let line = Rect::new(rect.x, cy, rect.width, 1);
            let base = if chosen {
                Style::default().bg(p.nord_blue).fg(p.black)
            } else {
                Style::default().bg(p.pmenu_bg).fg(p.white)
            };
            fill(buf, line, base);
            self.draw_completion_row(rt, buf, line, row, base, chosen);
        }
    }

    fn draw_completion_row(
        &self,
        rt: &Runtime,
        buf: &mut Buffer,
        line: Rect,
        row: &CompletionRow,
        base: Style,
        chosen: bool,
    ) {
        let p = rt.look.palette;
        let cy = line.y;
        let secondary = if chosen { base } else { base.fg(p.grey_fg) };
        let badge = row.badge().map_or('·', |badge| badge.letter);
        put(
            buf,
            line,
            line.x + 1,
            cy,
            &badge.to_string(),
            if chosen { base } else { base.fg(p.orange) },
        );
        // The tail first: it keeps its place at the right edge.
        let mut limit = line.right().saturating_sub(1);
        if let Some(tail) = &row.tail {
            let room = line.width.saturating_sub(width(&row.label) + 6).max(6);
            let tail = clip(tail, room);
            let at = limit.saturating_sub(width(&tail));
            put(buf, line, at, cy, &tail, secondary);
            limit = at.saturating_sub(1);
        }
        let mut x = line.x + 3;
        for (i, ch) in row.label.chars().enumerate() {
            let mut style = base;
            if row.matched.contains(&i) {
                style = style.add_modifier(Modifier::BOLD);
                if !chosen {
                    style = style.fg(p.nord_blue);
                }
            }
            if row.deprecated {
                style = style.add_modifier(Modifier::CROSSED_OUT);
            }
            x = put(buf, line, x, cy, ch.encode_utf8(&mut [0; 4]), style);
        }
        if let Some(detail) = &row.detail
            && x + 2 < limit
        {
            put(
                buf,
                line,
                x,
                cy,
                &clip(detail, limit.saturating_sub(x)),
                secondary,
            );
        }
    }

    fn draw_intentions(
        &mut self,
        rt: &mut Runtime,
        buf: &mut Buffer,
        area: Rect,
        (cx, cy): (u16, u16),
        rows: &[IntentionRow],
    ) {
        let p = rt.look.palette;
        let shown = rows.len().clamp(1, MAX_ROWS);
        let content_w = rows
            .iter()
            .map(|row| width(row.text()) + 7)
            .max()
            .unwrap_or(22)
            .clamp(22, 80);
        let w = content_w.min(area.width);
        let h = shown as u16;
        let x = cx
            .saturating_sub(2)
            .min(area.right().saturating_sub(w))
            .max(area.x);
        let mut y = cy + 1;
        if y + h > area.bottom() {
            y = cy.saturating_sub(h);
        }
        let rect = Rect::new(x, y, w, h);
        fill(buf, rect, Style::default().bg(p.pmenu_bg).fg(p.white));
        if rows.is_empty() {
            put(
                buf,
                rect,
                rect.x + 1,
                rect.y,
                "Looking for actions…",
                Style::default().bg(p.pmenu_bg).fg(p.grey_fg),
            );
            return;
        }
        let selected = self.popups.intention_selected.min(rows.len() - 1);
        let first = (selected + 1).saturating_sub(shown);
        for (offset, index) in (first..rows.len()).take(shown).enumerate() {
            let cy = rect.y + offset as u16;
            let line = Rect::new(rect.x, cy, rect.width, 1);
            let chosen = index == selected;
            let base = if chosen {
                Style::default().bg(p.nord_blue).fg(p.black)
            } else {
                Style::default().bg(p.pmenu_bg).fg(p.white)
            };
            fill(buf, line, base);
            match &rows[index] {
                IntentionRow::Action {
                    text, fix, nested, ..
                } => {
                    let indent = if *nested { 2 } else { 0 };
                    let bulb = rt.look.glyph(archipel_kit_tui::Glyph::Lightning);
                    put(
                        buf,
                        line,
                        rect.x + 1 + indent,
                        cy,
                        &bulb,
                        if chosen {
                            base
                        } else {
                            base.fg(if *fix { p.red } else { p.yellow })
                        },
                    );
                    put(
                        buf,
                        line,
                        rect.x + 3 + indent,
                        cy,
                        &clip(text, rect.width.saturating_sub(4 + indent)),
                        base,
                    );
                }
                IntentionRow::Submenu { title, open } => {
                    put(
                        buf,
                        line,
                        rect.x + 3,
                        cy,
                        &clip(title, rect.width.saturating_sub(6)),
                        base,
                    );
                    put(
                        buf,
                        line,
                        rect.right().saturating_sub(2),
                        cy,
                        if *open { "▾" } else { "▸" },
                        if chosen { base } else { base.fg(p.grey_fg) },
                    );
                }
            }
        }
    }

    /// The chooser of several declarations or implementations and the Show Usages popup:
    /// a header with the title and the count, then one row per target.
    fn draw_choices(
        &mut self,
        rt: &mut Runtime,
        buf: &mut Buffer,
        area: Rect,
        (cx, cy): (u16, u16),
    ) {
        let Some(list) = self.ide.state().navigation().choices.clone() else {
            return;
        };
        let p = rt.look.palette;
        let rows = chooser::rows(&list);
        let head = chooser::header(&list);
        let shown = rows.len().clamp(1, MAX_ROWS);
        let content_w = rows
            .iter()
            .map(|row| {
                3 + width(&row.before)
                    + width(&row.symbol)
                    + width(&row.after)
                    + width(&row.location)
                    + 4
            })
            .chain([width(&head.title) + width(&head.count) + 4])
            .max()
            .unwrap_or(30)
            .clamp(36, 110);
        let w = content_w.min(area.width);
        let h = shown as u16 + 1;
        let x = cx
            .saturating_sub(2)
            .min(area.right().saturating_sub(w))
            .max(area.x);
        let mut y = cy + 1;
        if y + h > area.bottom() {
            y = cy.saturating_sub(h);
        }
        let rect = Rect::new(x, y, w, h);
        let body = Style::default().bg(p.pmenu_bg).fg(p.white);
        fill(buf, rect, body);
        let after_title = put(
            buf,
            rect,
            rect.x + 1,
            rect.y,
            &head.title,
            body.add_modifier(Modifier::BOLD),
        );
        put(
            buf,
            rect,
            after_title + 1,
            rect.y,
            &head.count,
            body.fg(p.grey_fg),
        );
        let selected = self
            .popups
            .choice_selected
            .min(rows.len().saturating_sub(1));
        let first = (selected + 1).saturating_sub(shown);
        for (offset, index) in (first..rows.len()).take(shown).enumerate() {
            let row = &rows[index];
            let cy = rect.y + 1 + offset as u16;
            let line = Rect::new(rect.x, cy, rect.width, 1);
            let chosen = index == selected;
            let base = if chosen {
                Style::default().bg(p.nord_blue).fg(p.black)
            } else {
                body
            };
            fill(buf, line, base);
            let (glyph, color) = {
                let tokens =
                    archipel_kit_tui::chrome::Tokens::new(&rt.look, rt.look.theme.colors.popup);
                archipel_kit_tui::chrome::icon_cell(
                    &rt.look,
                    &tokens,
                    &row.icon,
                    archipel_kit_model::Tone::Default,
                )
            };
            put(
                buf,
                line,
                rect.x + 1,
                cy,
                &glyph,
                if chosen { base } else { base.fg(color) },
            );
            let location_w = width(&row.location);
            let at = line.right().saturating_sub(location_w + 1);
            let code_area = Rect::new(line.x, cy, at.saturating_sub(line.x + 1), 1);
            let mut x = put(buf, code_area, rect.x + 3, cy, &row.before, base);
            let marked = if chosen {
                base.add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
            } else {
                base.fg(p.nord_blue).add_modifier(Modifier::BOLD)
            };
            x = put(buf, code_area, x, cy, &row.symbol, marked);
            put(buf, code_area, x, cy, &row.after, base);
            put(
                buf,
                line,
                at,
                cy,
                &row.location,
                if chosen { base } else { base.fg(p.grey_fg) },
            );
        }
    }
}
