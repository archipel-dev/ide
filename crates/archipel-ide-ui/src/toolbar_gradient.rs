//! Project-color glow behind the main toolbar (Settings | Appearance | "Use project
//! colors in main toolbar").
//!
//! The glow is painted on the frame background, underneath the toolbar widgets and the
//! islands, so it shows through the toolbar and the gaps between islands. Its profile
//! follows `reference/android-studio/04-editor-project-tree.png`: it rises from the
//! window's left edge to the project badge, stays flat across the badge, then fades
//! out linearly about 570 px to the right of it, with a slight vertical falloff.

use eframe::egui::{self, Color32, Pos2, Rect, Shape, epaint::Mesh, pos2};

/// Horizontal fade distance from the badge centre to fully neutral.
const REACH: f32 = 570.0;
/// Flat part to the right of the badge.
const PLATEAU: f32 = 30.0;
/// Share of the glow left at the toolbar's bottom edge.
const BOTTOM: f32 = 0.85;

/// Where the glow is anchored: the toolbar band and the project badge.
#[derive(Debug, Clone, Copy)]
pub struct Anchor {
    pub bar: Rect,
    pub badge_left: f32,
    pub badge_right: f32,
}

impl Anchor {
    fn plateau_end(&self) -> f32 {
        self.badge_right + PLATEAU
    }

    fn fade_end(&self) -> f32 {
        (self.badge_left + self.badge_right) / 2.0 + REACH
    }

    /// Glow strength in `0..=1` at `x`.
    fn horizontal(&self, x: f32) -> f32 {
        let left = self.bar.left();
        if x <= left || x >= self.fade_end() {
            0.0
        } else if x < self.badge_left {
            (x - left) / (self.badge_left - left).max(1.0)
        } else if x <= self.plateau_end() {
            1.0
        } else {
            (self.fade_end() - x) / (self.fade_end() - self.plateau_end()).max(1.0)
        }
    }

    /// Vertical factor: full at the top, [`BOTTOM`] at the toolbar's bottom edge,
    /// fading to nothing half a toolbar height further down (the island gaps).
    fn vertical(&self, y: f32) -> f32 {
        let (top, height) = (self.bar.top(), self.bar.height().max(1.0));
        let t = (y - top) / height;
        if t <= 0.0 {
            1.0
        } else if t <= 1.0 {
            1.0 - (1.0 - BOTTOM) * t
        } else {
            (BOTTOM * (1.0 - (t - 1.0) * 2.0)).max(0.0)
        }
    }

    /// Lowest point the glow reaches.
    fn bottom(&self) -> f32 {
        self.bar.bottom() + self.bar.height() / 2.0
    }
}

fn lerp(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let channel = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t).round() as u8;
    Color32::from_rgb(
        channel(a.r(), b.r()),
        channel(a.g(), b.g()),
        channel(a.b(), b.b()),
    )
}

/// Full-strength glow color over `frame` for a project badge colored `project`.
/// Dark themes use a slightly lightened project color at about a third of its
/// strength; light themes use a fainter tint.
pub fn peak(frame: Color32, project: Color32, dark: bool) -> Color32 {
    if dark {
        lerp(frame, lerp(project, Color32::WHITE, 0.12), 0.32)
    } else {
        lerp(frame, project, 0.16)
    }
}

/// Opaque color of the frame background at `pos`.
pub fn color_at(anchor: &Anchor, frame: Color32, peak: Color32, pos: Pos2) -> Color32 {
    lerp(
        frame,
        peak,
        anchor.horizontal(pos.x) * anchor.vertical(pos.y),
    )
}

/// Mesh covering the glow. Vertices sit on every bend of the piecewise-linear
/// profile, so vertex-color interpolation reproduces [`color_at`] exactly.
pub fn mesh(anchor: &Anchor, frame: Color32, peak: Color32) -> Mesh {
    let left = anchor.bar.left();
    let mut xs = vec![
        left,
        anchor.badge_left.max(left),
        anchor.plateau_end().max(left),
        anchor.fade_end().min(anchor.bar.right()),
    ];
    xs.dedup_by(|a, b| (*a - *b).abs() < 0.5);
    let ys = [anchor.bar.top(), anchor.bar.bottom(), anchor.bottom()];
    let mut mesh = Mesh::default();
    for &y in &ys {
        for &x in &xs {
            mesh.colored_vertex(pos2(x, y), color_at(anchor, frame, peak, pos2(x, y)));
        }
    }
    let columns = xs.len() as u32;
    for row in 0..ys.len() as u32 - 1 {
        for column in 0..columns.saturating_sub(1) {
            let a = row * columns + column;
            let c = a + columns;
            mesh.add_triangle(a, a + 1, c);
            mesh.add_triangle(a + 1, c + 1, c);
        }
    }
    mesh
}

/// Reserves a slot on the background layer, below everything painted later in the
/// frame (toolbar widgets, islands). Fill it with [`paint`] once the badge is laid out.
pub fn reserve(ctx: &egui::Context) -> egui::layers::ShapeIdx {
    background(ctx).add(Shape::Noop)
}

/// Paints the glow into the slot from [`reserve`].
pub fn paint(ctx: &egui::Context, slot: egui::layers::ShapeIdx, anchor: Anchor, project: Color32) {
    let frame = *crate::theme::FRAME;
    let dark = crate::theme_engine::active().dark;
    let mesh = mesh(&anchor, frame, peak(frame, project, dark));
    background(ctx).set(slot, Shape::mesh(mesh));
}

fn background(ctx: &egui::Context) -> egui::Painter {
    ctx.layer_painter(egui::LayerId::background())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Reference geometry: toolbar band y 19..63 below the window manager's title
    /// bar, "MA" badge at x 60..80, frame (38,40,44), badge (36,152,157).
    fn reference() -> (Anchor, Color32, Color32) {
        let anchor = Anchor {
            bar: Rect::from_min_max(pos2(0.0, 19.0), pos2(1600.0, 63.0)),
            badge_left: 60.0,
            badge_right: 80.0,
        };
        let frame = Color32::from_rgb(38, 40, 44);
        (
            anchor,
            frame,
            peak(frame, Color32::from_rgb(36, 152, 157), true),
        )
    }

    fn assert_close(actual: Color32, expected: (u8, u8, u8), at: Pos2) {
        let diff = |a: u8, b: u8| (a as i16 - b as i16).abs();
        assert!(
            diff(actual.r(), expected.0) <= 3
                && diff(actual.g(), expected.1) <= 3
                && diff(actual.b(), expected.2) <= 3,
            "at {at:?}: got {actual:?}, reference {expected:?}"
        );
    }

    #[test]
    fn matches_the_reference_samples() {
        let (anchor, frame, peak) = reference();
        for (x, y, expected) in [
            (60.0, 22.0, (46, 80, 81)),
            (300.0, 22.0, (44, 66, 68)),
            (450.0, 22.0, (41, 53, 56)),
            (550.0, 22.0, (40, 45, 49)),
            (650.0, 22.0, (38, 40, 44)),
            (20.0, 22.0, (41, 53, 56)),
            (200.0, 60.0, (44, 70, 72)),
            (45.0, 60.0, (44, 66, 68)),
        ] {
            let at = pos2(x, y);
            assert_close(color_at(&anchor, frame, peak, at), expected, at);
        }
    }

    #[test]
    fn neutral_outside_the_glow() {
        let (anchor, frame, peak) = reference();
        assert_eq!(color_at(&anchor, frame, peak, pos2(0.0, 30.0)), frame);
        assert_eq!(color_at(&anchor, frame, peak, pos2(900.0, 30.0)), frame);
        assert_eq!(color_at(&anchor, frame, peak, pos2(200.0, 90.0)), frame);
    }

    #[test]
    fn light_themes_get_a_fainter_tint() {
        let frame = Color32::from_rgb(235, 236, 240);
        let project = Color32::from_rgb(36, 163, 148);
        let light = peak(frame, project, false);
        let dark = peak(frame, project, true);
        assert!(light.r() > dark.r() && light.g() > 200);
    }

    #[test]
    fn mesh_vertices_follow_the_profile() {
        let (anchor, frame, peak) = reference();
        let mesh = mesh(&anchor, frame, peak);
        assert_eq!(mesh.vertices.len(), 4 * 3);
        assert_eq!(mesh.indices.len(), 3 * 2 * 2 * 3);
        for vertex in &mesh.vertices {
            assert_eq!(vertex.color, color_at(&anchor, frame, peak, vertex.pos));
        }
    }
}
