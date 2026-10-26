//! Piecewise-linear mapping between editor source lines and preview scroll offsets.

use std::ops::Range;

#[derive(Debug, Clone, Copy, PartialEq)]
struct Anchor {
    line: f32,
    y: f32,
}

/// Maps fractional source lines to preview y offsets and back. Built from the measured
/// top and bottom of every rendered top-level block.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ScrollMap {
    anchors: Vec<Anchor>,
}

impl ScrollMap {
    /// `blocks` are `(source lines, top y, bottom y)` in document order.
    pub fn new(
        blocks: &[(Range<usize>, f32, f32)],
        total_lines: usize,
        content_height: f32,
    ) -> Self {
        let mut anchors = vec![Anchor { line: 0.0, y: 0.0 }];
        let mut push = |line: f32, y: f32| {
            let last = anchors
                .last()
                .copied()
                .unwrap_or(Anchor { line: 0.0, y: 0.0 });
            if !line.is_finite() || !y.is_finite() {
                return;
            }
            // Keep both axes monotonic so the mapping is invertible.
            let line = line.max(last.line);
            let y = y.max(last.y);
            if line == last.line && y == last.y {
                return;
            }
            anchors.push(Anchor { line, y });
        };
        for (lines, top, bottom) in blocks {
            push(lines.start as f32, *top);
            push(lines.end as f32, *bottom);
        }
        push(total_lines as f32, content_height);
        Self { anchors }
    }

    pub fn is_empty(&self) -> bool {
        self.anchors.len() < 2
    }

    /// Preview y of a (fractional) source line.
    pub fn line_to_y(&self, line: f32) -> f32 {
        self.interpolate(line, |a| a.line, |a| a.y)
    }

    /// (Fractional) source line shown at preview offset `y`.
    pub fn y_to_line(&self, y: f32) -> f32 {
        self.interpolate(y, |a| a.y, |a| a.line)
    }

    fn interpolate(&self, value: f32, from: fn(&Anchor) -> f32, to: fn(&Anchor) -> f32) -> f32 {
        let (Some(first), Some(last)) = (self.anchors.first(), self.anchors.last()) else {
            return 0.0;
        };
        if value <= from(first) {
            return to(first);
        }
        if value >= from(last) {
            return to(last);
        }
        let upper = self.anchors.partition_point(|a| from(a) <= value);
        let (low, high) = (&self.anchors[upper - 1], &self.anchors[upper]);
        let span = from(high) - from(low);
        if span <= f32::EPSILON {
            return to(low);
        }
        to(low) + (to(high) - to(low)) * (value - from(low)) / span
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map() -> ScrollMap {
        // 10 lines: a heading (line 0, 40px), a paragraph (lines 2..4, 100px) with gaps.
        ScrollMap::new(&[(0..1, 0.0, 40.0), (2..4, 60.0, 160.0)], 10, 300.0)
    }

    #[test]
    fn maps_lines_to_offsets() {
        let map = map();
        assert_eq!(map.line_to_y(0.0), 0.0);
        assert_eq!(map.line_to_y(1.0), 40.0);
        assert_eq!(map.line_to_y(2.0), 60.0);
        assert_eq!(map.line_to_y(3.0), 110.0);
        assert_eq!(map.line_to_y(10.0), 300.0);
        assert_eq!(map.line_to_y(99.0), 300.0);
        assert_eq!(map.line_to_y(-3.0), 0.0);
    }

    #[test]
    fn maps_offsets_back_to_lines() {
        let map = map();
        assert_eq!(map.y_to_line(110.0), 3.0);
        assert_eq!(map.y_to_line(0.0), 0.0);
        assert_eq!(map.y_to_line(1000.0), 10.0);
        for line in [0.0, 0.5, 2.5, 3.9, 7.0] {
            let round = map.y_to_line(map.line_to_y(line));
            assert!((round - line).abs() < 1e-3, "{line} -> {round}");
        }
    }

    #[test]
    fn degenerate_inputs_are_safe() {
        let empty = ScrollMap::default();
        assert!(empty.is_empty());
        assert_eq!(empty.line_to_y(5.0), 0.0);
        assert_eq!(empty.y_to_line(5.0), 0.0);
        // Overlapping or NaN blocks never produce a non-monotonic map.
        let odd = ScrollMap::new(&[(3..5, 50.0, 20.0), (1..2, f32::NAN, 70.0)], 6, 100.0);
        let mut last = 0.0;
        for i in 0..=60 {
            let y = odd.line_to_y(i as f32 / 10.0);
            assert!(y >= last);
            last = y;
        }
    }
}
