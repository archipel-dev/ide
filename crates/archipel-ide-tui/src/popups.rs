//! The popups at the caret: code completion and the Alt+Enter actions. Their state lives in
//! the core (`state.completion()`, `state.code_actions()`); this draws it and sends the keys
//! back as commands.

use crate::app::TuiApp;
use crate::draw::{clip, fill, put, width};
use archipel_ide_lsp::completion::{kind_name, label_detail};
use archipel_ide_lsp::lsp_types::CompletionItem;
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
}

const MAX_ROWS: usize = 10;

fn kind_letter(kind: &str) -> char {
    match kind {
        "method" | "function" | "constructor" => 'm',
        "field" => 'f',
        "property" => 'p',
        "variable" | "value" | "reference" => 'v',
        "constant" | "enumMember" => 'c',
        "class" | "struct" => 'C',
        "interface" => 'I',
        "enum" => 'E',
        "module" | "file" => 'M',
        "typeParameter" => 'T',
        "snippet" => 's',
        "keyword" => 'k',
        _ => '·',
    }
}

impl TuiApp {
    /// Keys of an open popup. Returns whether the popup took the key.
    pub(crate) fn popup_key(&mut self, rt: &mut Runtime, key: &KeyEvent) -> bool {
        let plain = !key
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT);
        if !plain || !self.layers.is_empty() || self.wb.focused_tool().is_some() {
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
            let rows = self.intention_rows().len();
            if rows == 0 {
                // Still computing: Esc cancels, the rest waits.
                if key.code == KeyCode::Esc {
                    self.ide.state_mut().code_actions_mut().popup = false;
                    return true;
                }
                return false;
            }
            match key.code {
                KeyCode::Down => {
                    self.popups.intention_selected = (self.popups.intention_selected + 1) % rows;
                }
                KeyCode::Up => {
                    self.popups.intention_selected =
                        (self.popups.intention_selected + rows - 1) % rows;
                }
                KeyCode::Esc => self.ide.state_mut().code_actions_mut().popup = false,
                KeyCode::Enter => {
                    let selected = self.popups.intention_selected.min(rows - 1);
                    let index = self.intention_rows()[selected].0;
                    self.command(rt, "code.applyAlert", json!({ "index": index }));
                    self.ide.state_mut().code_actions_mut().popup = false;
                }
                _ => return false,
            }
            return true;
        }
        self.popups.intention_open = false;
        false
    }

    /// The rows of the actions popup: (index into the alert set, text, is a fix).
    pub(crate) fn intention_rows(&mut self) -> Vec<(usize, String, bool)> {
        use archipel_ide_core::intentions::IntentionGroup;
        let Some(set) = self.ide.state_mut().current_alerts() else {
            return Vec::new();
        };
        set.actions
            .iter()
            .enumerate()
            .filter(|(_, action)| action.submenu.is_none())
            .map(|(index, action)| {
                let fix = matches!(
                    action.group,
                    IntentionGroup::Error | IntentionGroup::RemoteError
                );
                (index, action.text.clone(), fix)
            })
            .collect()
    }

    /// Draws the popup that is open, near the caret.
    pub(crate) fn draw_popups(&mut self, rt: &mut Runtime, buf: &mut Buffer) {
        let Some((cx, cy)) = self.popups.caret_cell else {
            return;
        };
        let area = self.popups.editor_area;
        if self.ide.state().code_actions().popup {
            if !self.popups.intention_open {
                self.popups.intention_open = true;
                self.popups.intention_selected = 0;
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
        let rows: Vec<(char, String, String, Vec<usize>, bool)> = session
            .ranked
            .iter()
            .filter_map(|ranked| {
                let item: &CompletionItem = session.items.get(ranked.index)?;
                let kind = kind_name(item.kind);
                Some((
                    kind_letter(kind),
                    item.label.clone(),
                    label_detail(item).unwrap_or_default().to_owned(),
                    ranked.label_positions.clone(),
                    item.deprecated.unwrap_or(false),
                ))
            })
            .collect();
        let selected = session.selected;
        let p = rt.look.palette;
        let shown = rows.len().min(MAX_ROWS);
        let first = selected
            .saturating_sub(shown.saturating_sub(1))
            .min(rows.len().saturating_sub(shown));
        let first = if selected < first { selected } else { first };
        let content_w = rows
            .iter()
            .map(|(_, label, detail, _, _)| width(label) + width(detail) + 6)
            .max()
            .unwrap_or(20)
            .clamp(24, 70);
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
            let (letter, label, detail, positions, deprecated) = &rows[index];
            let cy = rect.y + offset as u16;
            let chosen = index == selected;
            let row = Rect::new(rect.x, cy, rect.width, 1);
            let base = if chosen {
                Style::default().bg(p.nord_blue).fg(p.black)
            } else {
                Style::default().bg(p.pmenu_bg).fg(p.white)
            };
            fill(buf, row, base);
            put(
                buf,
                row,
                rect.x + 1,
                cy,
                &letter.to_string(),
                if chosen { base } else { base.fg(p.orange) },
            );
            let mut x = rect.x + 3;
            for (i, ch) in label.chars().enumerate() {
                let mut style = base;
                if positions.contains(&i) {
                    style = style.add_modifier(Modifier::BOLD);
                    if !chosen {
                        style = style.fg(p.nord_blue);
                    }
                }
                if *deprecated {
                    style = style.add_modifier(Modifier::CROSSED_OUT);
                }
                x = put(buf, row, x, cy, ch.encode_utf8(&mut [0; 4]), style);
            }
            if !detail.is_empty() && x + 2 < row.right() {
                put(
                    buf,
                    row,
                    x + 1,
                    cy,
                    &clip(detail, row.right().saturating_sub(x + 2)),
                    if chosen { base } else { base.fg(p.grey_fg) },
                );
            }
        }
    }

    fn draw_intentions(
        &mut self,
        rt: &mut Runtime,
        buf: &mut Buffer,
        area: Rect,
        (cx, cy): (u16, u16),
        rows: &[(usize, String, bool)],
    ) {
        let p = rt.look.palette;
        let shown = rows.len().clamp(1, MAX_ROWS);
        let content_w = rows
            .iter()
            .map(|(_, text, _)| width(text) + 5)
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
            let (_, text, fix) = &rows[index];
            let cy = rect.y + offset as u16;
            let row = Rect::new(rect.x, cy, rect.width, 1);
            let chosen = index == selected;
            let base = if chosen {
                Style::default().bg(p.nord_blue).fg(p.black)
            } else {
                Style::default().bg(p.pmenu_bg).fg(p.white)
            };
            fill(buf, row, base);
            let bulb = rt.look.glyph(archipel_kit_tui::Glyph::Lightning);
            put(
                buf,
                row,
                rect.x + 1,
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
                row,
                rect.x + 3,
                cy,
                &clip(text, rect.width.saturating_sub(4)),
                base,
            );
        }
    }
}
