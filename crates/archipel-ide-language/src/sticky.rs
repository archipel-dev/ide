//! Sticky scopes (IntelliJ sticky lines): the enclosing containers whose first
//! line has scrolled above the viewport. Scopes are the multi-line
//! declarations of `tags.scm` plus the nodes `sticky.scm` marks `@sticky`
//! (control flow); outermost first.

use crate::lines::LineIndex;
use crate::node::SyntaxSnapshot;
use crate::queryset::{self, QueryKind};
use crate::structure::StructureItem;
use std::ops::Range;

/// Default for the "maximum sticky lines" setting (IntelliJ: five).
pub const DEFAULT_MAX_STICKY: usize = 5;

/// One pinned scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StickyScope {
    /// Bytes of the scope's first line without leading and trailing whitespace.
    pub header: Range<usize>,
    pub start_line: usize,
    pub end_line: usize,
}

fn declarations(items: &[StructureItem], out: &mut Vec<Range<usize>>) {
    for item in items {
        out.push(item.range.clone());
        declarations(&item.children, out);
    }
}

/// The scopes containing `first_visible_line` whose first line is above it,
/// outermost first, at most `max` (the outermost ones when there are more).
pub fn sticky_scopes(
    snap: &SyntaxSnapshot,
    first_visible_line: usize,
    max: usize,
) -> Vec<StickyScope> {
    let text = snap.text();
    let lines = LineIndex::new(text);
    sticky_scopes_with(snap, &lines, &snap.structure(), first_visible_line, max)
}

/// [`sticky_scopes`] with a prebuilt line index and structure (cached per
/// snapshot by callers, as this runs every frame).
pub fn sticky_scopes_with(
    snap: &SyntaxSnapshot,
    lines: &LineIndex,
    structure: &[StructureItem],
    first_visible_line: usize,
    max: usize,
) -> Vec<StickyScope> {
    let text = snap.text();
    if first_visible_line == 0 || max == 0 || first_visible_line >= lines.line_count() {
        return Vec::new();
    }
    let point = lines.line_start(first_visible_line);
    let mut ranges: Vec<Range<usize>> = Vec::new();
    if let Some(query) = queryset::query(snap.lang(), QueryKind::Sticky) {
        ranges.extend(
            queryset::captures_in(query, snap.tree(), text, point..point + 1)
                .into_iter()
                .filter(|captured| captured.name == "sticky")
                .map(|captured| captured.node.byte_range()),
        );
    }
    // Declarations come from the structure; only those around the line matter.
    let mut declared = Vec::new();
    declarations(structure, &mut declared);
    ranges.extend(declared);
    let mut scopes: Vec<StickyScope> = ranges
        .into_iter()
        .filter(|range| range.start <= point && point < range.end.max(point + 1))
        .filter_map(|range| {
            let start_line = lines.line_of(range.start);
            let end_line = lines.line_of(range.end.saturating_sub(1));
            if start_line >= first_visible_line || end_line < first_visible_line {
                return None;
            }
            let line = lines.line_range(start_line, text);
            let raw = &text[line.clone()];
            let header_start = line.start + (raw.len() - raw.trim_start().len());
            let header_end = line.start + raw.trim_end().len();
            (header_start < header_end).then_some(StickyScope {
                header: header_start..header_end,
                start_line,
                end_line,
            })
        })
        .collect();
    // One scope per header line (the outermost), outermost first.
    scopes.sort_by_key(|scope| (scope.start_line, std::cmp::Reverse(scope.end_line)));
    scopes.dedup_by_key(|scope| scope.start_line);
    scopes.truncate(max);
    scopes
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snap(id: &str, source: &str) -> SyntaxSnapshot {
        let spec = crate::registry::by_id(id).expect("language");
        SyntaxSnapshot::parse(Default::default(), Default::default(), spec, source.into())
            .expect("parse")
    }

    #[test]
    fn declarations_and_control_flow_stay_pinned_outermost_first() {
        let source = "class A {\n    void f(int n) {\n        for (int i = 0; i < n; i++) {\n            g(i);\n            h(i);\n        }\n    }\n}\n";
        let snapshot = snap("java", source);
        let scopes = sticky_scopes(&snapshot, 3, 5);
        let headers: Vec<_> = scopes.iter().map(|s| &source[s.header.clone()]).collect();
        assert_eq!(
            headers,
            [
                "class A {",
                "void f(int n) {",
                "for (int i = 0; i < n; i++) {"
            ]
        );
        assert_eq!(sticky_scopes(&snapshot, 3, 2).len(), 2);
        assert!(sticky_scopes(&snapshot, 0, 5).is_empty());
        // The scope that ends on the first visible line is still listed; the
        // one that ended above it is not.
        let after_loop = sticky_scopes(&snapshot, 6, 5);
        let headers: Vec<_> = after_loop
            .iter()
            .map(|s| &source[s.header.clone()])
            .collect();
        assert_eq!(headers, ["class A {", "void f(int n) {"]);
    }
}
