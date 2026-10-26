//! Generic highlight engine: runs a language's bundled `highlights.scm` over a
//! syntax tree and produces non-overlapping spans keyed by IntelliJ text
//! attribute keys. Semantics follow `tree-sitter-highlight`: for a given node
//! the last matching pattern in the query wins (queries list generic patterns
//! such as `(identifier) @variable` first), and nested captures override the
//! enclosing capture for their range.
//!
//! Highlights are produced as a [`HighlightLayer`]. Further layers (e.g. LSP
//! semantic tokens) are combined with [`HighlightLayer::overlay`].

use crate::captures;
use crate::injection;
use crate::registry::{LANGUAGES, LanguageSpec};
use archipel_ide_text::ByteEdit;
use std::ops::Range;
use std::sync::OnceLock;
use tree_sitter::{Node, Query, QueryCursor, StreamingIterator, Tree};

/// One highlighted byte range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HighlightSpan {
    pub start_byte: usize,
    pub end_byte: usize,
    /// IntelliJ text attribute key (e.g. `DEFAULT_KEYWORD`).
    pub key: &'static str,
}

impl HighlightSpan {
    pub fn range(&self) -> Range<usize> {
        self.start_byte..self.end_byte
    }
}

/// Sorted, non-overlapping highlight spans for one source of highlighting.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HighlightLayer {
    spans: Vec<HighlightSpan>,
}

impl HighlightLayer {
    /// Builds a layer from arbitrary spans; overlaps are resolved so that later
    /// spans win, and empty spans are dropped. Intended for small overlay
    /// layers (e.g. semantic tokens arrive sorted and disjoint already).
    pub fn from_spans(spans: impl IntoIterator<Item = HighlightSpan>) -> Self {
        spans
            .into_iter()
            .filter(|span| span.start_byte < span.end_byte)
            .fold(Self::default(), |layer, span| {
                layer.overlay(&Self { spans: vec![span] })
            })
    }

    /// Builds a layer from spans in any order; where spans overlap, the one
    /// starting later wins (nested injections come after their host).
    fn from_unsorted(mut spans: Vec<HighlightSpan>) -> Self {
        spans.sort_by_key(|span| span.start_byte);
        let mut layer = Self::default();
        let mut overlapping = Vec::new();
        for span in spans {
            if span.start_byte >= span.end_byte {
                continue;
            }
            if layer
                .spans
                .last()
                .is_some_and(|last| last.end_byte > span.start_byte)
            {
                overlapping.push(span);
            } else {
                push_merged(&mut layer.spans, span.start_byte..span.end_byte, span.key);
            }
        }
        // Rare (overlapping injections): resolve the remainder exactly.
        for span in overlapping {
            layer = layer.overlay(&Self { spans: vec![span] });
        }
        layer
    }

    pub fn spans(&self) -> &[HighlightSpan] {
        &self.spans
    }

    pub fn is_empty(&self) -> bool {
        self.spans.is_empty()
    }

    /// Spans that intersect `range`, clipped to it.
    pub fn in_range(&self, range: Range<usize>) -> impl Iterator<Item = HighlightSpan> + '_ {
        let first = self
            .spans
            .partition_point(|span| span.end_byte <= range.start);
        self.spans[first..]
            .iter()
            .take_while(move |span| span.start_byte < range.end)
            .map(move |span| HighlightSpan {
                start_byte: span.start_byte.max(range.start),
                end_byte: span.end_byte.min(range.end),
                key: span.key,
            })
    }

    /// Shifts spans across a buffer edit. Spans overlapping the replaced bytes
    /// are cut around them; the caller re-highlights the edited range. Returns
    /// the post-edit extent of any cut span: its key was computed from text
    /// the edit changed (e.g. a comment that lost its `/*`), so it is stale.
    pub fn apply_edit(&mut self, edit: &ByteEdit) -> Option<Range<usize>> {
        let shift = |offset: usize| {
            offset
                .saturating_add(edit.new_end_byte)
                .saturating_sub(edit.old_end_byte)
        };
        // Spans are sorted and disjoint: [first, last) overlap the replaced bytes.
        let first = self
            .spans
            .partition_point(|span| span.end_byte <= edit.start_byte);
        let last = self
            .spans
            .partition_point(|span| span.start_byte < edit.old_end_byte)
            .max(first);
        let mut pieces = Vec::new();
        let mut stale: Option<Range<usize>> = None;
        for span in &self.spans[first..last] {
            let start = span.start_byte.min(edit.start_byte);
            let end = shift(span.end_byte).max(edit.new_end_byte);
            stale = Some(stale.map_or(start..end, |range| {
                range.start.min(start)..range.end.max(end)
            }));
            if span.start_byte < edit.start_byte {
                pieces.push(HighlightSpan {
                    start_byte: span.start_byte,
                    end_byte: edit.start_byte,
                    key: span.key,
                });
            }
            if span.end_byte > edit.old_end_byte {
                pieces.push(HighlightSpan {
                    start_byte: edit.new_end_byte,
                    end_byte: shift(span.end_byte),
                    key: span.key,
                });
            }
        }
        for span in &mut self.spans[last..] {
            span.start_byte = shift(span.start_byte);
            span.end_byte = shift(span.end_byte);
        }
        self.spans.splice(first..last, pieces);
        stale
    }

    /// Grows `range` to fully cover the spans crossing its boundaries.
    pub fn covering(&self, range: Range<usize>) -> Range<usize> {
        let first = self
            .spans
            .partition_point(|span| span.end_byte <= range.start);
        let last = self
            .spans
            .partition_point(|span| span.start_byte < range.end);
        if first >= last {
            return range;
        }
        range.start.min(self.spans[first].start_byte)..range.end.max(self.spans[last - 1].end_byte)
    }

    /// Replaces everything inside `range` with `replacement` (already clipped
    /// to `range`), keeping spans outside it.
    pub fn splice(&mut self, range: Range<usize>, replacement: HighlightLayer) {
        let first = self
            .spans
            .partition_point(|span| span.end_byte <= range.start);
        let last = self
            .spans
            .partition_point(|span| span.start_byte < range.end)
            .max(first);
        let mut window: Vec<HighlightSpan> = Vec::with_capacity(replacement.spans.len() + 4);
        // Keep the neighbours so adjacent equal keys merge across the seams.
        let lead = first.saturating_sub(1);
        window.extend(self.spans[lead..first].iter().cloned());
        if let Some(head) = self.spans[first..last].first()
            && head.start_byte < range.start
        {
            push_merged(&mut window, head.start_byte..range.start, head.key);
        }
        for span in replacement.spans {
            push_merged(&mut window, span.start_byte..span.end_byte, span.key);
        }
        if let Some(tail) = self.spans[first..last].last()
            && tail.end_byte > range.end
        {
            push_merged(&mut window, range.end..tail.end_byte, tail.key);
        }
        let trail = (last + 1).min(self.spans.len());
        for span in &self.spans[last..trail] {
            push_merged(&mut window, span.start_byte..span.end_byte, span.key);
        }
        self.spans.splice(lead..trail, window);
    }

    /// Returns a new layer where `top` replaces this layer wherever it has a span.
    pub fn overlay(&self, top: &HighlightLayer) -> HighlightLayer {
        let mut pieces: Vec<HighlightSpan> = Vec::with_capacity(self.spans.len() + top.spans.len());
        let mut first = 0;
        for base in &self.spans {
            while first < top.spans.len() && top.spans[first].end_byte <= base.start_byte {
                first += 1;
            }
            let mut cursor = base.start_byte;
            for over in top.spans[first..]
                .iter()
                .take_while(|over| over.start_byte < base.end_byte)
            {
                if over.start_byte > cursor {
                    pieces.push(HighlightSpan {
                        start_byte: cursor,
                        end_byte: over.start_byte,
                        key: base.key,
                    });
                }
                cursor = cursor.max(over.end_byte);
            }
            if cursor < base.end_byte {
                pieces.push(HighlightSpan {
                    start_byte: cursor,
                    end_byte: base.end_byte,
                    key: base.key,
                });
            }
        }
        pieces.extend(top.spans.iter().cloned());
        pieces.sort_by_key(|span| span.start_byte);
        let mut spans = Vec::with_capacity(pieces.len());
        for piece in pieces {
            push_merged(&mut spans, piece.start_byte..piece.end_byte, piece.key);
        }
        HighlightLayer { spans }
    }
}

fn push_merged(spans: &mut Vec<HighlightSpan>, range: Range<usize>, key: &'static str) {
    if range.start >= range.end {
        return;
    }
    if let Some(last) = spans.last_mut()
        && last.end_byte == range.start
        && last.key == key
    {
        last.end_byte = range.end;
        return;
    }
    spans.push(HighlightSpan {
        start_byte: range.start,
        end_byte: range.end,
        key,
    });
}

/// A language's compiled highlight query and per-capture attribute keys.
pub(crate) struct CompiledHighlights {
    pub query: Query,
    pub capture_keys: Vec<Option<&'static str>>,
    /// Captures named `punctuation*`; inside strings they take the string color.
    pub punctuation: Vec<bool>,
}

/// Errors compiling a bundled query (reported, never panicked on).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryError {
    pub language: &'static str,
    pub message: String,
}

impl std::fmt::Display for QueryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} highlight query: {}", self.language, self.message)
    }
}

impl std::error::Error for QueryError {}

/// Compiles (once) and returns the highlight query for `spec`.
pub(crate) fn compiled(
    spec: &'static LanguageSpec,
) -> Result<&'static CompiledHighlights, QueryError> {
    static CACHE: OnceLock<Vec<OnceLock<Result<CompiledHighlights, QueryError>>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| LANGUAGES.iter().map(|_| OnceLock::new()).collect());
    let index = LANGUAGES
        .iter()
        .position(|candidate| std::ptr::eq(candidate, spec))
        .ok_or_else(|| QueryError {
            language: spec.id,
            message: "language is not registered".into(),
        })?;
    cache[index]
        .get_or_init(|| compile(spec))
        .as_ref()
        .map_err(Clone::clone)
}

fn compile(spec: &'static LanguageSpec) -> Result<CompiledHighlights, QueryError> {
    let source = spec.highlights.join("\n");
    let query = Query::new(&spec.grammar(), &source).map_err(|error| QueryError {
        language: spec.id,
        message: error.to_string(),
    })?;
    let capture_keys = query
        .capture_names()
        .iter()
        .map(|name| captures::capture_key(name, spec.capture_keys))
        .collect();
    let punctuation = query
        .capture_names()
        .iter()
        .map(|name| name.starts_with("punctuation"))
        .collect();
    Ok(CompiledHighlights {
        query,
        capture_keys,
        punctuation,
    })
}

struct RawCapture {
    range: Range<usize>,
    pattern: usize,
    key: &'static str,
    punctuation: bool,
}

/// Highlights the bytes in `range` of `source`, which `tree` was parsed from,
/// including injected languages.
pub fn highlight(
    spec: &'static LanguageSpec,
    tree: &Tree,
    source: &str,
    range: Range<usize>,
) -> Result<HighlightLayer, QueryError> {
    highlight_at_depth(spec, tree, source, range, 0)
}

fn highlight_at_depth(
    spec: &'static LanguageSpec,
    tree: &Tree,
    source: &str,
    range: Range<usize>,
    depth: usize,
) -> Result<HighlightLayer, QueryError> {
    let raw = collect(spec, tree, source, range.clone())?;
    let mut layer = flatten(raw, range.clone());
    if depth >= injection::MAX_DEPTH {
        return Ok(layer);
    }
    // Injected layers are gathered and merged in one pass: overlaying each
    // one onto the whole layer would be quadratic for files with many
    // injections (every Rust macro call is one).
    let mut injected_spans: Vec<HighlightSpan> = Vec::new();
    for injection in injection::injections(spec, tree, source, range.clone()) {
        let extent = injection.extent();
        let clip = range.start.max(extent.start)..range.end.min(extent.end);
        if clip.start >= clip.end {
            continue;
        }
        let Some(injected) = injection.parse(source) else {
            continue;
        };
        // Injected languages without highlight queries contribute nothing.
        if let Ok(injected) = highlight_at_depth(injection.spec, &injected, source, clip, depth + 1)
        {
            injected_spans.extend(injected.spans);
        }
    }
    if !injected_spans.is_empty() {
        layer = layer.overlay(&HighlightLayer::from_unsorted(injected_spans));
    }
    Ok(layer)
}

/// Like [`highlight`], but first grows `range` until it covers every capture
/// and injected region intersecting it, so captures whose key depends on
/// their whole node (e.g. a comment turned into a doc comment) are recomputed
/// entirely. Returns the grown range and its highlights; used for incremental
/// re-highlighting.
pub fn highlight_covering(
    spec: &'static LanguageSpec,
    tree: &Tree,
    source: &str,
    mut range: Range<usize>,
) -> Result<(Range<usize>, HighlightLayer), QueryError> {
    loop {
        let raw = collect(spec, tree, source, range.clone())?;
        let grow = |acc: Range<usize>, other: Range<usize>| {
            if other.start < acc.end && other.end > acc.start {
                acc.start.min(other.start)..acc.end.max(other.end)
            } else {
                acc
            }
        };
        let mut covered = raw.iter().fold(range.clone(), |acc, capture| {
            grow(acc, capture.range.clone())
        });
        // An injection that merely touches the range may have gained or lost
        // text at that edge, which changes its whole injected parse.
        let touching = range.start.saturating_sub(1)..(range.end + 1).min(source.len());
        covered = injection::injections(spec, tree, source, touching)
            .iter()
            .fold(covered, |acc, injection| {
                let extent = injection.extent();
                if extent.start <= acc.end && extent.end >= acc.start {
                    acc.start.min(extent.start)..acc.end.max(extent.end)
                } else {
                    acc
                }
            });
        if covered == range {
            return Ok((range.clone(), highlight(spec, tree, source, range)?));
        }
        range = covered;
    }
}

fn collect(
    spec: &'static LanguageSpec,
    tree: &Tree,
    source: &str,
    range: Range<usize>,
) -> Result<Vec<RawCapture>, QueryError> {
    let compiled = compiled(spec)?;
    let mut cursor = QueryCursor::new();
    cursor.set_byte_range(range);
    let mut raw: Vec<RawCapture> = Vec::new();
    let mut captures = cursor.captures(&compiled.query, tree.root_node(), source.as_bytes());
    while let Some((query_match, index)) = captures.next() {
        let capture = query_match.captures[*index];
        let Some(Some(key)) = compiled.capture_keys.get(capture.index as usize) else {
            continue;
        };
        let node: Node<'_> = capture.node;
        let capture_name = compiled.query.capture_names()[capture.index as usize];
        let text = source.get(node.byte_range()).unwrap_or_default();
        raw.push(RawCapture {
            range: node.byte_range(),
            pattern: query_match.pattern_index,
            key: captures::refine(capture_name, key, text, node),
            punctuation: compiled.punctuation[capture.index as usize],
        });
    }
    Ok(raw)
}

/// Resolves captured ranges into non-overlapping spans: the latest pattern
/// wins for identical ranges and inner ranges override enclosing ones.
fn flatten(mut raw: Vec<RawCapture>, clip: Range<usize>) -> HighlightLayer {
    // Matches intersecting the range may carry captures outside it.
    raw.retain(|capture| capture.range.start < clip.end && capture.range.end > clip.start);
    raw.sort_by(|a, b| {
        a.range
            .start
            .cmp(&b.range.start)
            .then(b.range.end.cmp(&a.range.end))
            .then(b.pattern.cmp(&a.pattern))
    });
    raw.dedup_by(|later, earlier| later.range == earlier.range);
    let mut spans = Vec::new();
    let mut stack: Vec<RawCapture> = Vec::new();
    let mut pos = clip.start;
    let clip_end = clip.end;
    let emit_until = |target: usize,
                      stack: &mut Vec<RawCapture>,
                      pos: &mut usize,
                      spans: &mut Vec<HighlightSpan>| {
        let target = target.min(clip_end);
        while let Some(top) = stack.last() {
            let end = top.range.end.min(target);
            if end > *pos {
                push_merged(spans, *pos..end, top.key);
                *pos = end;
            }
            if top.range.end <= target {
                stack.pop();
            } else {
                break;
            }
        }
        *pos = (*pos).max(target);
    };
    for capture in raw {
        emit_until(capture.range.start, &mut stack, &mut pos, &mut spans);
        // String delimiters are part of the literal, as in IntelliJ.
        if capture.punctuation
            && stack.last().is_some_and(|parent| {
                parent.key == "DEFAULT_STRING" && parent.range.end >= capture.range.end
            })
        {
            continue;
        }
        stack.push(capture);
    }
    emit_until(clip.end, &mut stack, &mut pos, &mut spans);
    HighlightLayer { spans }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn span(range: Range<usize>, key: &'static str) -> HighlightSpan {
        HighlightSpan {
            start_byte: range.start,
            end_byte: range.end,
            key,
        }
    }

    fn raw(range: Range<usize>, pattern: usize, key: &'static str) -> RawCapture {
        RawCapture {
            range,
            pattern,
            key,
            punctuation: key == "PUNCT",
        }
    }

    #[test]
    fn flatten_prefers_inner_ranges_and_last_patterns() {
        let layer = flatten(
            vec![
                raw(0..10, 3, "STRING"),
                raw(4..6, 9, "ESCAPE"),
                raw(4..6, 5, "OTHER"),
                raw(12..14, 1, "KEYWORD"),
                raw(15..19, 1, "DEFAULT_STRING"),
                raw(15..16, 2, "PUNCT"),
            ],
            0..20,
        );
        assert_eq!(
            layer.spans(),
            &[
                span(0..4, "STRING"),
                span(4..6, "ESCAPE"),
                span(6..10, "STRING"),
                span(12..14, "KEYWORD"),
                span(15..19, "DEFAULT_STRING"),
            ]
        );
    }

    #[test]
    fn overlay_replaces_covered_ranges() {
        let base = HighlightLayer {
            spans: vec![span(0..10, "A"), span(10..20, "B")],
        };
        let top = HighlightLayer {
            spans: vec![span(5..12, "T"), span(25..30, "U")],
        };
        assert_eq!(
            base.overlay(&top).spans(),
            &[
                span(0..5, "A"),
                span(5..12, "T"),
                span(12..20, "B"),
                span(25..30, "U")
            ]
        );
        assert_eq!(base.overlay(&HighlightLayer::default()), base);
        assert_eq!(HighlightLayer::default().overlay(&top), top);
    }

    #[test]
    fn layer_range_queries_clip() {
        let layer = HighlightLayer::from_spans([span(0..4, "A"), span(6..10, "B")]);
        let clipped: Vec<_> = layer.in_range(2..8).collect();
        assert_eq!(clipped, vec![span(2..4, "A"), span(6..8, "B")]);
    }

    #[test]
    fn every_bundled_highlight_query_compiles() {
        for spec in LANGUAGES {
            let compiled = compiled(spec).unwrap_or_else(|error| panic!("{error}"));
            assert!(
                compiled.capture_keys.iter().any(Option::is_some),
                "{} maps no captures",
                spec.id
            );
        }
    }
}
