//! VectorDrawable rendering: parses `<vector>` XML (paths, groups with
//! translate/scale/rotate, fill/stroke colors and alpha), flattens SVG path
//! data to polygons and rasterizes them with non-zero/even-odd fill and 4×4
//! supersampled anti-aliasing into an RGBA buffer.

/// A rendered image: `width * height` straight-alpha RGBA pixels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rgba {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

impl Rgba {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            pixels: vec![0; (width * height * 4) as usize],
        }
    }

    pub fn pixel(&self, x: u32, y: u32) -> [u8; 4] {
        let i = ((y * self.width + x) * 4) as usize;
        [
            self.pixels[i],
            self.pixels[i + 1],
            self.pixels[i + 2],
            self.pixels[i + 3],
        ]
    }

    /// Source-over blends `color` (ARGB) with `coverage` 0..=1 at (x, y).
    fn blend(&mut self, x: u32, y: u32, color: u32, coverage: f32) {
        let a = ((color >> 24) & 0xFF) as f32 / 255.0 * coverage.clamp(0.0, 1.0);
        if a <= 0.0 {
            return;
        }
        let i = ((y * self.width + x) * 4) as usize;
        let src = [
            ((color >> 16) & 0xFF) as f32,
            ((color >> 8) & 0xFF) as f32,
            (color & 0xFF) as f32,
        ];
        let dst_a = self.pixels[i + 3] as f32 / 255.0;
        let out_a = a + dst_a * (1.0 - a);
        for (pixel, src) in self.pixels[i..i + 3].iter_mut().zip(src) {
            let dst = *pixel as f32;
            let value = (src * a + dst * dst_a * (1.0 - a)) / out_a.max(f32::EPSILON);
            *pixel = value.round().clamp(0.0, 255.0) as u8;
        }
        self.pixels[i + 3] = (out_a * 255.0).round().clamp(0.0, 255.0) as u8;
    }

    /// Draws `other` over this image (same size).
    pub fn draw_over(&mut self, other: &Rgba) {
        if other.width != self.width || other.height != self.height {
            return;
        }
        for y in 0..self.height {
            for x in 0..self.width {
                let [r, g, b, a] = other.pixel(x, y);
                let color = (u32::from(a) << 24)
                    | (u32::from(r) << 16)
                    | (u32::from(g) << 8)
                    | u32::from(b);
                self.blend(x, y, color | 0xFF00_0000, f32::from(a) / 255.0);
            }
        }
    }
}

type Point = (f32, f32);

/// 2D affine transform `[a, b, c, d, e, f]`: x' = a*x + c*y + e, y' = b*x + d*y + f.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Transform([f32; 6]);

impl Transform {
    const IDENTITY: Self = Self([1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);

    fn then(self, next: Transform) -> Transform {
        let [a, b, c, d, e, f] = self.0;
        let [na, nb, nc, nd, ne, nf] = next.0;
        Transform([
            na * a + nc * b,
            nb * a + nd * b,
            na * c + nc * d,
            nb * c + nd * d,
            na * e + nc * f + ne,
            nb * e + nd * f + nf,
        ])
    }

    fn apply(self, (x, y): Point) -> Point {
        let [a, b, c, d, e, f] = self.0;
        (a * x + c * y + e, b * x + d * y + f)
    }
}

#[derive(Debug, Clone, PartialEq)]
struct PathItem {
    polygons: Vec<Vec<Point>>,
    fill: Option<u32>,
    stroke: Option<(u32, f32)>,
    even_odd: bool,
}

/// A parsed VectorDrawable.
#[derive(Debug, Clone, PartialEq)]
pub struct VectorDrawable {
    pub viewport_width: f32,
    pub viewport_height: f32,
    items: Vec<PathItem>,
}

const ANDROID_NS: &str = "http://schemas.android.com/apk/res/android";

/// Parses `#RGB`, `#ARGB`, `#RRGGBB` and `#AARRGGBB` into ARGB.
pub fn parse_color(text: &str) -> Option<u32> {
    let hex = text.trim().strip_prefix('#')?;
    let digits: Vec<u32> = hex.chars().map(|c| c.to_digit(16)).collect::<Option<_>>()?;
    let expand = |d: u32| d * 17;
    Some(match digits.as_slice() {
        [r, g, b] => 0xFF00_0000 | expand(*r) << 16 | expand(*g) << 8 | expand(*b),
        [a, r, g, b] => expand(*a) << 24 | expand(*r) << 16 | expand(*g) << 8 | expand(*b),
        [..] if digits.len() == 6 => 0xFF00_0000 | u32::from_str_radix(hex, 16).ok()?,
        [..] if digits.len() == 8 => u32::from_str_radix(hex, 16).ok()?,
        _ => return None,
    })
}

fn with_alpha(color: u32, alpha: f32) -> u32 {
    let a = (((color >> 24) & 0xFF) as f32 * alpha.clamp(0.0, 1.0)).round() as u32;
    (a << 24) | (color & 0x00FF_FFFF)
}

fn number(text: Option<&str>) -> Option<f32> {
    let text = text?.trim();
    let digits = text.trim_end_matches(|c: char| c.is_ascii_alphabetic());
    digits.parse().ok()
}

impl VectorDrawable {
    /// Parses a `<vector>` document. `resolve_color` resolves `@color/…`
    /// references; unresolvable colors are treated as no paint.
    pub fn parse(xml: &str, resolve_color: &dyn Fn(&str) -> Option<u32>) -> Result<Self, String> {
        let document = roxmltree::Document::parse(xml).map_err(|e| e.to_string())?;
        let root = document.root_element();
        if !root.has_tag_name("vector") {
            return Err(format!(
                "<{}> is not a vector drawable",
                root.tag_name().name()
            ));
        }
        fn attr<'a>(node: roxmltree::Node<'a, '_>, name: &str) -> Option<&'a str> {
            node.attribute((ANDROID_NS, name))
        }
        let viewport_width = number(attr(root, "viewportWidth"))
            .or(number(attr(root, "width")))
            .filter(|w| *w > 0.0)
            .ok_or("missing viewportWidth")?;
        let viewport_height = number(attr(root, "viewportHeight"))
            .or(number(attr(root, "height")))
            .filter(|h| *h > 0.0)
            .ok_or("missing viewportHeight")?;
        let color = |value: Option<&str>| -> Option<u32> {
            let value = value?;
            if value.starts_with('#') {
                parse_color(value)
            } else {
                resolve_color(value)
            }
        };
        let mut items = Vec::new();
        fn walk(
            node: roxmltree::Node,
            transform: Transform,
            items: &mut Vec<PathItem>,
            color: &dyn Fn(Option<&str>) -> Option<u32>,
        ) {
            for child in node.children().filter(|n| n.is_element()) {
                let attr = |name: &str| child.attribute((ANDROID_NS, name));
                if child.has_tag_name("group") {
                    let num = |name: &str, default: f32| number(attr(name)).unwrap_or(default);
                    let (px, py) = (num("pivotX", 0.0), num("pivotY", 0.0));
                    let angle = num("rotation", 0.0).to_radians();
                    let (sin, cos) = angle.sin_cos();
                    // Android: translate(-pivot) scale rotate translate(pivot + translate).
                    let local = Transform([1.0, 0.0, 0.0, 1.0, -px, -py])
                        .then(Transform([
                            num("scaleX", 1.0),
                            0.0,
                            0.0,
                            num("scaleY", 1.0),
                            0.0,
                            0.0,
                        ]))
                        .then(Transform([cos, sin, -sin, cos, 0.0, 0.0]))
                        .then(Transform([
                            1.0,
                            0.0,
                            0.0,
                            1.0,
                            px + num("translateX", 0.0),
                            py + num("translateY", 0.0),
                        ]));
                    walk(child, local.then(transform), items, color);
                } else if child.has_tag_name("path") {
                    let Some(data) = attr("pathData") else {
                        continue;
                    };
                    let polygons: Vec<Vec<Point>> = flatten(data)
                        .into_iter()
                        .map(|poly| poly.into_iter().map(|p| transform.apply(p)).collect())
                        .collect();
                    let fill_alpha = number(attr("fillAlpha")).unwrap_or(1.0);
                    let stroke_alpha = number(attr("strokeAlpha")).unwrap_or(1.0);
                    let scale = (transform.0[0].hypot(transform.0[1])
                        + transform.0[2].hypot(transform.0[3]))
                        / 2.0;
                    items.push(PathItem {
                        polygons,
                        fill: color(attr("fillColor")).map(|c| with_alpha(c, fill_alpha)),
                        stroke: color(attr("strokeColor"))
                            .map(|c| with_alpha(c, stroke_alpha))
                            .zip(number(attr("strokeWidth")).map(|w| w * scale))
                            .filter(|(_, w)| *w > 0.0),
                        even_odd: attr("fillType") == Some("evenOdd"),
                    });
                }
            }
        }
        walk(root, Transform::IDENTITY, &mut items, &color);
        Ok(Self {
            viewport_width,
            viewport_height,
            items,
        })
    }

    /// Renders into a `size`×`size` image, preserving the aspect ratio.
    pub fn render(&self, size: u32) -> Rgba {
        let size = size.clamp(1, 1024);
        let mut image = Rgba::new(size, size);
        let scale = size as f32 / self.viewport_width.max(self.viewport_height);
        let offset = (
            (size as f32 - self.viewport_width * scale) / 2.0,
            (size as f32 - self.viewport_height * scale) / 2.0,
        );
        let map = |(x, y): Point| (x * scale + offset.0, y * scale + offset.1);
        for item in &self.items {
            let polygons: Vec<Vec<Point>> = item
                .polygons
                .iter()
                .map(|p| p.iter().copied().map(map).collect())
                .collect();
            if let Some(fill) = item.fill {
                fill_polygons(&mut image, &polygons, fill, item.even_odd);
            }
            if let Some((color, width)) = item.stroke {
                let outline = stroke_polygons(&polygons, width * scale);
                fill_polygons(&mut image, &outline, color, false);
            }
        }
        image
    }
}

/// Turns each segment into a quad so strokes can reuse the fill rasterizer.
fn stroke_polygons(polygons: &[Vec<Point>], width: f32) -> Vec<Vec<Point>> {
    let half = width / 2.0;
    let mut quads = Vec::new();
    for polygon in polygons {
        for pair in polygon.windows(2) {
            let ((x0, y0), (x1, y1)) = (pair[0], pair[1]);
            let (dx, dy) = (x1 - x0, y1 - y0);
            let len = dx.hypot(dy);
            if len < 1e-6 {
                continue;
            }
            let (nx, ny) = (-dy / len * half, dx / len * half);
            quads.push(vec![
                (x0 + nx, y0 + ny),
                (x1 + nx, y1 + ny),
                (x1 - nx, y1 - ny),
                (x0 - nx, y0 - ny),
            ]);
        }
    }
    quads
}

/// Scanline fill with 4×4 supersampling per pixel.
fn fill_polygons(image: &mut Rgba, polygons: &[Vec<Point>], color: u32, even_odd: bool) {
    const SAMPLES: usize = 4;
    let edges: Vec<(Point, Point)> = polygons
        .iter()
        .filter(|p| p.len() >= 3)
        .flat_map(|p| {
            p.iter()
                .copied()
                .zip(p.iter().copied().cycle().skip(1))
                .take(p.len())
        })
        .filter(|((_, y0), (_, y1))| (y0 - y1).abs() > f32::EPSILON)
        .collect();
    if edges.is_empty() {
        return;
    }
    let (min_y, max_y) = edges
        .iter()
        .fold((f32::MAX, f32::MIN), |(lo, hi), ((_, a), (_, b))| {
            (lo.min(*a).min(*b), hi.max(*a).max(*b))
        });
    let y_start = min_y.floor().max(0.0) as u32;
    let y_end = (max_y.ceil() as u32).min(image.height);
    let mut coverage = vec![0f32; image.width as usize];
    let mut crossings: Vec<(f32, i32)> = Vec::new();
    for py in y_start..y_end {
        coverage.iter_mut().for_each(|c| *c = 0.0);
        for sy in 0..SAMPLES {
            let y = py as f32 + (sy as f32 + 0.5) / SAMPLES as f32;
            crossings.clear();
            for &((x0, y0), (x1, y1)) in &edges {
                if (y0 <= y && y < y1) || (y1 <= y && y < y0) {
                    let x = x0 + (y - y0) / (y1 - y0) * (x1 - x0);
                    crossings.push((x, if y1 > y0 { 1 } else { -1 }));
                }
            }
            crossings.sort_by(|a, b| a.0.total_cmp(&b.0));
            let mut winding = 0;
            for pair in crossings.windows(2) {
                winding += pair[0].1;
                let inside = if even_odd {
                    winding % 2 != 0
                } else {
                    winding != 0
                };
                if !inside {
                    continue;
                }
                for sx in 0..SAMPLES {
                    let offset = (sx as f32 + 0.5) / SAMPLES as f32;
                    // Sample columns whose sample point lies in [x_left, x_right).
                    let first = (pair[0].0 - offset).ceil().max(0.0) as i64;
                    let last = ((pair[1].0 - offset).ceil() as i64).min(image.width as i64);
                    for px in first.max(0)..last {
                        coverage[px as usize] += 1.0 / (SAMPLES * SAMPLES) as f32;
                    }
                }
            }
        }
        for (px, value) in coverage.iter().enumerate() {
            if *value > 0.0 {
                image.blend(px as u32, py, color, *value);
            }
        }
    }
}

/// Flattens SVG/VectorDrawable path data into polygons (curves sampled).
fn flatten(data: &str) -> Vec<Vec<Point>> {
    let tokens = tokenize(data);
    let mut polygons: Vec<Vec<Point>> = Vec::new();
    let mut current: Vec<Point> = Vec::new();
    let mut pos: Point = (0.0, 0.0);
    let mut start: Point = (0.0, 0.0);
    let mut last_control: Option<Point> = None;
    let mut last_quad: Option<Point> = None;
    let mut index = 0;
    let mut command = 'M';
    let flush = |current: &mut Vec<Point>, polygons: &mut Vec<Vec<Point>>| {
        if current.len() > 1 {
            polygons.push(std::mem::take(current));
        } else {
            current.clear();
        }
    };
    while index < tokens.len() {
        if let Token::Command(c) = tokens[index] {
            command = c;
            index += 1;
            if c == 'z' || c == 'Z' {
                current.push(start);
                flush(&mut current, &mut polygons);
                pos = start;
                last_control = None;
                last_quad = None;
                continue;
            }
        }
        let arity = match command.to_ascii_uppercase() {
            'M' | 'L' | 'T' => 2,
            'H' | 'V' => 1,
            'C' => 6,
            'S' | 'Q' => 4,
            'A' => 7,
            _ => {
                index += 1;
                continue;
            }
        };
        let mut args = [0f32; 7];
        let mut taken = 0;
        while taken < arity {
            match tokens.get(index) {
                Some(Token::Number(n)) => {
                    args[taken] = *n;
                    taken += 1;
                    index += 1;
                }
                _ => break,
            }
        }
        if taken < arity {
            // Malformed: skip to the next command.
            while index < tokens.len() && matches!(tokens[index], Token::Number(_)) {
                index += 1;
            }
            if index < tokens.len() && taken == 0 && !matches!(tokens[index], Token::Command(_)) {
                index += 1;
            }
            if taken == 0 && index >= tokens.len() {
                break;
            }
            continue;
        }
        let relative = command.is_ascii_lowercase();
        let abs = |x: f32, y: f32| {
            if relative {
                (pos.0 + x, pos.1 + y)
            } else {
                (x, y)
            }
        };
        let mut control = None;
        let mut quad = None;
        match command.to_ascii_uppercase() {
            'M' => {
                flush(&mut current, &mut polygons);
                pos = abs(args[0], args[1]);
                start = pos;
                current.push(pos);
                // Subsequent pairs are implicit line-tos.
                command = if relative { 'l' } else { 'L' };
            }
            'L' => {
                pos = abs(args[0], args[1]);
                current.push(pos);
            }
            'H' => {
                pos = (if relative { pos.0 + args[0] } else { args[0] }, pos.1);
                current.push(pos);
            }
            'V' => {
                pos = (pos.0, if relative { pos.1 + args[0] } else { args[0] });
                current.push(pos);
            }
            'C' | 'S' => {
                let (c1, c2, end) = if command.eq_ignore_ascii_case(&'C') {
                    (
                        abs(args[0], args[1]),
                        abs(args[2], args[3]),
                        abs(args[4], args[5]),
                    )
                } else {
                    let c1 = last_control.map_or(pos, |c| (2.0 * pos.0 - c.0, 2.0 * pos.1 - c.1));
                    (c1, abs(args[0], args[1]), abs(args[2], args[3]))
                };
                cubic(&mut current, pos, c1, c2, end);
                control = Some(c2);
                pos = end;
            }
            'Q' | 'T' => {
                let (c, end) = if command.eq_ignore_ascii_case(&'Q') {
                    (abs(args[0], args[1]), abs(args[2], args[3]))
                } else {
                    let c = last_quad.map_or(pos, |c| (2.0 * pos.0 - c.0, 2.0 * pos.1 - c.1));
                    (c, abs(args[0], args[1]))
                };
                let c1 = (
                    pos.0 + 2.0 / 3.0 * (c.0 - pos.0),
                    pos.1 + 2.0 / 3.0 * (c.1 - pos.1),
                );
                let c2 = (
                    end.0 + 2.0 / 3.0 * (c.0 - end.0),
                    end.1 + 2.0 / 3.0 * (c.1 - end.1),
                );
                cubic(&mut current, pos, c1, c2, end);
                quad = Some(c);
                pos = end;
            }
            'A' => {
                let end = abs(args[5], args[6]);
                arc(
                    &mut current,
                    pos,
                    args[0],
                    args[1],
                    args[2],
                    args[3] != 0.0,
                    args[4] != 0.0,
                    end,
                );
                pos = end;
            }
            _ => {}
        }
        if current.is_empty() {
            current.push(pos);
        }
        last_control = control;
        last_quad = quad;
    }
    flush(&mut current, &mut polygons);
    polygons
}

fn cubic(out: &mut Vec<Point>, p0: Point, p1: Point, p2: Point, p3: Point) {
    if out.is_empty() {
        out.push(p0);
    }
    const STEPS: usize = 16;
    for i in 1..=STEPS {
        let t = i as f32 / STEPS as f32;
        let u = 1.0 - t;
        let (a, b, c, d) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
        out.push((
            a * p0.0 + b * p1.0 + c * p2.0 + d * p3.0,
            a * p0.1 + b * p1.1 + c * p2.1 + d * p3.1,
        ));
    }
}

/// Endpoint-parameterized elliptical arc (SVG implementation notes F.6).
#[allow(clippy::too_many_arguments)]
fn arc(
    out: &mut Vec<Point>,
    from: Point,
    rx: f32,
    ry: f32,
    rotation: f32,
    large: bool,
    sweep: bool,
    to: Point,
) {
    if out.is_empty() {
        out.push(from);
    }
    let (mut rx, mut ry) = (rx.abs(), ry.abs());
    if rx < 1e-6 || ry < 1e-6 || (from.0 - to.0).hypot(from.1 - to.1) < 1e-6 {
        out.push(to);
        return;
    }
    let phi = rotation.to_radians();
    let (sin, cos) = phi.sin_cos();
    let dx = (from.0 - to.0) / 2.0;
    let dy = (from.1 - to.1) / 2.0;
    let x1 = cos * dx + sin * dy;
    let y1 = -sin * dx + cos * dy;
    let lambda = (x1 * x1) / (rx * rx) + (y1 * y1) / (ry * ry);
    if lambda > 1.0 {
        rx *= lambda.sqrt();
        ry *= lambda.sqrt();
    }
    let num = rx * rx * ry * ry - rx * rx * y1 * y1 - ry * ry * x1 * x1;
    let den = rx * rx * y1 * y1 + ry * ry * x1 * x1;
    let mut coef = (num / den).max(0.0).sqrt();
    if large == sweep {
        coef = -coef;
    }
    let cx1 = coef * rx * y1 / ry;
    let cy1 = -coef * ry * x1 / rx;
    let cx = cos * cx1 - sin * cy1 + (from.0 + to.0) / 2.0;
    let cy = sin * cx1 + cos * cy1 + (from.1 + to.1) / 2.0;
    let angle = |ux: f32, uy: f32, vx: f32, vy: f32| (ux * vy - uy * vx).atan2(ux * vx + uy * vy);
    let theta1 = angle(1.0, 0.0, (x1 - cx1) / rx, (y1 - cy1) / ry);
    let mut delta = angle(
        (x1 - cx1) / rx,
        (y1 - cy1) / ry,
        (-x1 - cx1) / rx,
        (-y1 - cy1) / ry,
    );
    if !sweep && delta > 0.0 {
        delta -= std::f32::consts::TAU;
    } else if sweep && delta < 0.0 {
        delta += std::f32::consts::TAU;
    }
    let steps = ((delta.abs() / std::f32::consts::TAU) * 32.0)
        .ceil()
        .max(2.0) as usize;
    for i in 1..=steps {
        let t = theta1 + delta * i as f32 / steps as f32;
        let (st, ct) = t.sin_cos();
        out.push((
            cx + rx * ct * cos - ry * st * sin,
            cy + rx * ct * sin + ry * st * cos,
        ));
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Token {
    Command(char),
    Number(f32),
}

fn tokenize(data: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = data.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_ascii_alphabetic() && c != 'e' && c != 'E' {
            tokens.push(Token::Command(c));
            i += 1;
        } else if c.is_ascii_digit() || c == '-' || c == '+' || c == '.' {
            let start = i;
            let mut seen_dot = c == '.';
            i += 1;
            while i < chars.len() {
                let d = chars[i];
                if d.is_ascii_digit() {
                    i += 1;
                } else if d == '.' && !seen_dot {
                    seen_dot = true;
                    i += 1;
                } else if (d == 'e' || d == 'E')
                    && chars
                        .get(i + 1)
                        .is_some_and(|n| n.is_ascii_digit() || *n == '-' || *n == '+')
                {
                    i += 2;
                } else {
                    break;
                }
            }
            let text: String = chars[start..i].iter().collect();
            if let Ok(value) = text.parse::<f32>() {
                tokens.push(Token::Number(value));
            }
        } else {
            i += 1;
        }
    }
    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    fn no_refs(_: &str) -> Option<u32> {
        None
    }

    #[test]
    fn parses_colors() {
        assert_eq!(parse_color("#FF0000"), Some(0xFFFF_0000));
        assert_eq!(parse_color("#80FF0000"), Some(0x80FF_0000));
        assert_eq!(parse_color("#F00"), Some(0xFFFF_0000));
        assert_eq!(parse_color("#8F00"), Some(0x88FF_0000));
        assert_eq!(parse_color("red"), None);
        assert_eq!(parse_color("#12345"), None);
    }

    #[test]
    fn tokenizes_compact_path_data() {
        let tokens = tokenize("M12,2C6.48,2 2,6.48 2,12s4.48,10 10,10-1.5e1.5.5z");
        assert_eq!(tokens[0], Token::Command('M'));
        assert_eq!(tokens[1], Token::Number(12.0));
        assert!(tokens.contains(&Token::Command('s')));
        assert_eq!(tokens.last(), Some(&Token::Command('z')));
        // "-1.5e1.5.5" → -1.5e1 (=-15), .5, .5
        let t = tokenize("-1.5e1.5.5");
        assert_eq!(
            t,
            [Token::Number(-15.0), Token::Number(0.5), Token::Number(0.5)]
        );
    }

    #[test]
    fn renders_a_filled_square_with_antialiased_edges() {
        let xml = r##"<vector xmlns:android="http://schemas.android.com/apk/res/android"
            android:width="24dp" android:height="24dp" android:viewportWidth="24" android:viewportHeight="24">
            <path android:fillColor="#FF0000FF" android:pathData="M6,6h12v12h-12z"/>
        </vector>"##;
        let vector = VectorDrawable::parse(xml, &no_refs).unwrap();
        let image = vector.render(24);
        assert_eq!(image.pixel(12, 12), [0, 0, 255, 255]);
        assert_eq!(image.pixel(2, 2)[3], 0);
        assert_eq!(image.pixel(5, 12)[3], 0);
        assert_eq!(image.pixel(6, 12)[3], 255);
        // Scaled render keeps proportions: the square covers the middle half.
        let big = vector.render(48);
        assert_eq!(big.pixel(24, 24)[3], 255);
        assert_eq!(big.pixel(11, 24)[3], 0);
    }

    #[test]
    fn groups_transform_and_evenodd_holes() {
        let xml = r##"<vector xmlns:android="http://schemas.android.com/apk/res/android"
            android:viewportWidth="10" android:viewportHeight="10">
            <group android:translateX="5">
                <path android:fillColor="#000" android:pathData="M0,0h5v5h-5z"/>
            </group>
            <path android:fillColor="#F00" android:fillType="evenOdd"
                  android:pathData="M0,5h5v5h-5z M1,6h3v3h-3z"/>
        </vector>"##;
        let image = VectorDrawable::parse(xml, &no_refs).unwrap().render(10);
        assert_eq!(image.pixel(7, 2), [0, 0, 0, 255], "translated group");
        assert_eq!(image.pixel(2, 2)[3], 0);
        assert_eq!(image.pixel(0, 9), [255, 0, 0, 255]);
        assert_eq!(image.pixel(2, 7)[3], 0, "even-odd hole");
    }

    #[test]
    fn circles_via_arcs_and_curves() {
        let xml = r##"<vector xmlns:android="http://schemas.android.com/apk/res/android"
            android:viewportWidth="20" android:viewportHeight="20">
            <path android:fillColor="@color/brand" android:pathData="M10,2a8,8 0,1 1,0 16a8,8 0,1 1,0 -16z"/>
            <path android:strokeColor="#FFFFFF" android:strokeWidth="2" android:pathData="M0,19L20,19"/>
        </vector>"##;
        let resolve = |name: &str| (name == "@color/brand").then_some(0xFF00_8000);
        let image = VectorDrawable::parse(xml, &resolve).unwrap().render(20);
        assert_eq!(image.pixel(10, 10), [0, 128, 0, 255]);
        assert_eq!(image.pixel(1, 1)[3], 0, "outside the circle");
        assert_eq!(image.pixel(10, 19), [255, 255, 255, 255], "stroke");
    }

    #[test]
    fn rejects_non_vectors_and_tolerates_bad_path_data() {
        assert!(VectorDrawable::parse("<shape/>", &no_refs).is_err());
        assert!(VectorDrawable::parse("<vector/>", &no_refs).is_err());
        let xml = r##"<vector xmlns:android="http://schemas.android.com/apk/res/android"
            android:viewportWidth="10" android:viewportHeight="10">
            <path android:fillColor="#000" android:pathData="M1,1 L 5 C z Q1 X 9,9 A"/>
        </vector>"##;
        let image = VectorDrawable::parse(xml, &no_refs).unwrap().render(10);
        assert_eq!(image.width, 10);
    }
}
