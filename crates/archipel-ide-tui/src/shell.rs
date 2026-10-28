//! The screen's geometry and focus (TUI-GUI-010..016): row 0 toolbar, row H-1 status bar, the
//! left and right stripes (3 columns), tool window panes with a one-row header, one-column
//! splitters, the editor column (tab strip, editor, error stripe) and a bottom tool window
//! under the editor column only.
//!
//! Which windows are visible, their anchors and order are the core's (`ToolWindowLayout`,
//! shared with the GUI); the sizes of resizable areas are cells and live here (`tui.*`),
//! so a drag in the terminal never changes the window's sizes.

use archipel_ide_core::{Anchor, ToolWindow};
use archipel_kit_tui::hit::Id;
use archipel_kit_tui::input::MouseButton;
use archipel_kit_tui::pointer::{Pointer, PointerKind};
use archipel_kit_tui::ratatui::layout::Rect;

/// Columns of a stripe.
pub const STRIPE: u16 = 3;
/// Terminal smaller than this shows only a message (spec B).
pub const MIN_SIZE: (u16, u16) = (60, 16);
/// Narrowest editor column and fewest editor rows.
const MIN_EDITOR_COLS: u16 = 20;
const MIN_EDITOR_ROWS: u16 = 3;
/// The ids of the splitters in the hit map.
pub const SPLIT_LEFT: Id = Id::new("tui.split.left");
/// See [`SPLIT_LEFT`].
pub const SPLIT_RIGHT: Id = Id::new("tui.split.right");
/// See [`SPLIT_LEFT`].
pub const SPLIT_BOTTOM: Id = Id::new("tui.split.bottom");

/// Which part has keyboard focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Focus {
    /// The editor.
    #[default]
    Editor,
    /// A tool window.
    Tool(ToolWindow),
}

/// A visible tool window's rectangles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pane {
    /// The window.
    pub window: ToolWindow,
    /// Header and body.
    pub rect: Rect,
    /// The header row.
    pub header: Rect,
    /// Below the header.
    pub body: Rect,
}

fn pane(window: ToolWindow, rect: Rect) -> Pane {
    let header = Rect::new(rect.x, rect.y, rect.width, 1.min(rect.height));
    let body = Rect::new(
        rect.x,
        rect.y + header.height,
        rect.width,
        rect.height.saturating_sub(header.height),
    );
    Pane {
        window,
        rect,
        header,
        body,
    }
}

/// Where everything is on one screen.
#[derive(Debug, Clone, Default)]
pub struct Geometry {
    /// The whole screen.
    pub area: Rect,
    /// Row 0.
    pub toolbar: Rect,
    /// Row H-1.
    pub status: Rect,
    /// The left stripe.
    pub left_stripe: Rect,
    /// The right stripe.
    pub right_stripe: Rect,
    /// The left tool window.
    pub left: Option<Pane>,
    /// The right tool window.
    pub right: Option<Pane>,
    /// The bottom tool window.
    pub bottom: Option<Pane>,
    /// The editor column: tab strip row, then the editor.
    pub editor_column: Rect,
    /// The tab strip.
    pub tabs: Rect,
    /// The editor area (text, gutter, error stripe column).
    pub editor: Rect,
    /// The splitter cells and what they resize.
    pub splitters: Vec<(Id, Rect)>,
    /// The terminal is smaller than [`MIN_SIZE`].
    pub too_small: bool,
}

impl Geometry {
    /// The pane of `window`, when it is visible.
    pub fn pane_of(&self, window: ToolWindow) -> Option<&Pane> {
        [&self.left, &self.right, &self.bottom]
            .into_iter()
            .flatten()
            .find(|pane| pane.window == window)
    }
}

#[derive(Debug, Clone, Copy)]
struct Drag {
    which: Id,
    start: u16,
    original: u16,
}

/// Focus, sizes and the drag in progress.
#[derive(Debug, Clone, Default)]
pub struct Shell {
    /// Keyboard focus.
    pub focus: Focus,
    /// Left window width in cells; `None` is the default (28.8 percent, at least 24).
    pub left_cols: Option<u16>,
    /// Right window width; `None` is 25 percent.
    pub right_cols: Option<u16>,
    /// Bottom window height in rows including its header; `None` is a quarter of the screen.
    pub bottom_rows: Option<u16>,
    /// A maximized window takes the whole body.
    pub maximized: Option<ToolWindow>,
    /// The last tool window that had focus.
    pub last_tool: Option<ToolWindow>,
    /// The geometry of the last frame.
    pub geo: Geometry,
    drag: Option<Drag>,
}

impl Shell {
    /// Whether a tool window has focus.
    pub fn tool_focused(&self) -> bool {
        matches!(self.focus, Focus::Tool(_))
    }

    /// The focused tool window.
    pub fn focused_tool(&self) -> Option<ToolWindow> {
        match self.focus {
            Focus::Tool(window) => Some(window),
            Focus::Editor => None,
        }
    }

    /// Focuses the editor.
    pub fn focus_editor(&mut self) {
        if let Focus::Tool(window) = self.focus {
            self.last_tool = Some(window);
        }
        self.focus = Focus::Editor;
    }

    /// Focuses `window`.
    pub fn focus_tool(&mut self, window: ToolWindow) {
        self.focus = Focus::Tool(window);
        self.last_tool = Some(window);
    }

    fn default_left(width: u16) -> u16 {
        (u32::from(width) * 288 / 1000).max(24) as u16
    }

    /// Computes the geometry for `area` with the windows the core shows.
    pub fn compute(
        &mut self,
        area: Rect,
        left: Option<ToolWindow>,
        right: Option<ToolWindow>,
        bottom: Option<ToolWindow>,
    ) -> &Geometry {
        let mut geo = Geometry {
            area,
            too_small: area.width < MIN_SIZE.0 || area.height < MIN_SIZE.1,
            ..Geometry::default()
        };
        if area.height < 3 || area.width < 8 {
            self.geo = geo;
            return &self.geo;
        }
        geo.toolbar = Rect::new(area.x, area.y, area.width, 1);
        geo.status = Rect::new(area.x, area.bottom() - 1, area.width, 1);
        let body = Rect::new(area.x, area.y + 1, area.width, area.height - 2);
        geo.left_stripe = Rect::new(body.x, body.y, STRIPE, body.height);
        geo.right_stripe = Rect::new(
            body.right().saturating_sub(STRIPE),
            body.y,
            STRIPE,
            body.height,
        );
        let inner_x = body.x + STRIPE;
        let inner_w = body.width.saturating_sub(2 * STRIPE);
        // Narrow screens collapse the right, then the left window to their stripe.
        let right = right.filter(|_| area.width >= 100);
        let left = left.filter(|_| area.width >= 70);
        if let Some(window) = self
            .maximized
            .filter(|w| [left, right, bottom].contains(&Some(*w)))
        {
            let rect = Rect::new(inner_x, body.y, inner_w, body.height);
            let p = pane(window, rect);
            match window.anchor() {
                Anchor::Left => geo.left = Some(p),
                Anchor::Right => geo.right = Some(p),
                Anchor::Bottom => geo.bottom = Some(p),
            }
            geo.editor_column = Rect::default();
            self.geo = geo;
            return &self.geo;
        }
        let max_side = (inner_w * 4 / 10).max(8);
        let mut x = inner_x;
        let mut right_edge = inner_x + inner_w;
        let mut splitters = Vec::new();
        if let Some(window) = left {
            let cols = self
                .left_cols
                .unwrap_or_else(|| Self::default_left(area.width));
            let cols = cols
                .clamp(12, max_side)
                .min(inner_w.saturating_sub(MIN_EDITOR_COLS + 2));
            geo.left = Some(pane(window, Rect::new(x, body.y, cols, body.height)));
            splitters.push((SPLIT_LEFT, Rect::new(x + cols, body.y, 1, body.height)));
            x += cols + 1;
        }
        if let Some(window) = right {
            let cols = self.right_cols.unwrap_or(inner_w / 4);
            let cols = cols
                .clamp(12, max_side)
                .min((right_edge.saturating_sub(x)).saturating_sub(MIN_EDITOR_COLS + 1));
            let rx = right_edge - cols;
            geo.right = Some(pane(window, Rect::new(rx, body.y, cols, body.height)));
            splitters.push((SPLIT_RIGHT, Rect::new(rx - 1, body.y, 1, body.height)));
            right_edge = rx - 1;
        }
        let column = Rect::new(x, body.y, right_edge.saturating_sub(x), body.height);
        geo.editor_column = column;
        let mut editor_rows = column.height;
        if let Some(window) = bottom {
            let rows = self
                .bottom_rows
                .unwrap_or_else(|| (u32::from(area.height) * 25 / 100).max(6) as u16);
            let max_rows = column.height.saturating_sub(MIN_EDITOR_ROWS + 1);
            let rows = rows.clamp(4, max_rows.max(4)).min(max_rows);
            let rect = Rect::new(column.x, column.bottom() - rows, column.width, rows);
            let p = pane(window, rect);
            splitters.push((SPLIT_BOTTOM, p.header));
            geo.bottom = Some(p);
            editor_rows = column.height - rows;
        }
        geo.tabs = Rect::new(column.x, column.y, column.width, 1.min(editor_rows));
        geo.editor = Rect::new(
            column.x,
            column.y + geo.tabs.height,
            column.width,
            editor_rows.saturating_sub(geo.tabs.height),
        );
        geo.splitters = splitters;
        self.geo = geo;
        &self.geo
    }

    /// Registers the splitters with the hit map.
    pub fn register(&self, rt: &mut archipel_kit_tui::runtime::Runtime) {
        for (id, rect) in &self.geo.splitters {
            // The header row of the bottom window is also where its title is: the splitter is
            // registered below the header widgets, so they win.
            rt.clickable(*id, *rect);
        }
    }

    /// Mouse handling of the splitters: press, drag, release, double click resets. Returns
    /// whether the pointer was for a splitter.
    pub fn handle_pointer(&mut self, p: &Pointer) -> bool {
        let sizes = |this: &Self, id: Id| -> u16 {
            if id == SPLIT_LEFT {
                this.geo.left.map_or(0, |p| p.rect.width)
            } else if id == SPLIT_RIGHT {
                this.geo.right.map_or(0, |p| p.rect.width)
            } else {
                this.geo.bottom.map_or(0, |p| p.rect.height)
            }
        };
        match p.kind {
            PointerKind::Down(MouseButton::Left) => {
                let hit = self
                    .geo
                    .splitters
                    .iter()
                    .find(|(id, _)| p.over(*id))
                    .map(|(id, _)| *id);
                if let Some(id) = hit {
                    if p.clicks >= 2 {
                        self.reset(id);
                        return true;
                    }
                    let start = if id == SPLIT_BOTTOM { p.y } else { p.x };
                    self.drag = Some(Drag {
                        which: id,
                        start,
                        original: sizes(self, id),
                    });
                    return true;
                }
                false
            }
            PointerKind::Drag(MouseButton::Left) => {
                let Some(drag) = self.drag else {
                    return false;
                };
                let (pos, delta): (u16, i32) = if drag.which == SPLIT_BOTTOM {
                    (p.y, i32::from(drag.start) - i32::from(p.y))
                } else if drag.which == SPLIT_LEFT {
                    (p.x, i32::from(p.x) - i32::from(drag.start))
                } else {
                    (p.x, i32::from(drag.start) - i32::from(p.x))
                };
                let _ = pos;
                let size = (i32::from(drag.original) + delta).max(1) as u16;
                self.set(drag.which, size);
                true
            }
            PointerKind::Up(MouseButton::Left) => self.drag.take().is_some(),
            _ => false,
        }
    }

    fn set(&mut self, id: Id, size: u16) {
        if id == SPLIT_LEFT {
            self.left_cols = Some(size);
        } else if id == SPLIT_RIGHT {
            self.right_cols = Some(size);
        } else {
            self.bottom_rows = Some(size);
        }
    }

    fn reset(&mut self, id: Id) {
        if id == SPLIT_LEFT {
            self.left_cols = None;
        } else if id == SPLIT_RIGHT {
            self.right_cols = None;
        } else {
            self.bottom_rows = None;
        }
    }

    /// Resizes the pane of `anchor` by `cells` (the GUI's Ctrl+Alt+Shift+arrows move by two).
    pub fn resize(&mut self, anchor: Anchor, cells: i32) {
        let (current, id) = match anchor {
            Anchor::Left => (self.geo.left.map_or(0, |p| p.rect.width), SPLIT_LEFT),
            Anchor::Right => (self.geo.right.map_or(0, |p| p.rect.width), SPLIT_RIGHT),
            Anchor::Bottom => (self.geo.bottom.map_or(0, |p| p.rect.height), SPLIT_BOTTOM),
        };
        if current > 0 {
            self.set(id, (i32::from(current) + cells).max(1) as u16);
        }
    }
}
