//! Error-stripe data: the marks of one file, bucketed to the stripe's height.

use crate::markup::FileMarkup;
use archipel_ide_analysis::{HighlightKind, Severity};

/// One stripe mark (a problem or a highlight worth a tick).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StripeMark {
    pub start: u32,
    pub end: u32,
    pub severity: Severity,
}

/// Marks of the listed problems and (information level) caret-usage
/// highlights, sorted by position. Line markers and plain text attributes do
/// not appear.
pub fn stripe_marks(markup: &FileMarkup) -> Vec<StripeMark> {
    markup
        .iter()
        .filter(|info| {
            info.severity.is_problem()
                || matches!(info.kind, HighlightKind::Semantic("IDENTIFIER_UNDER_CARET"))
        })
        .filter(|info| !info.file_level)
        .map(|info| StripeMark {
            start: info.range.start,
            end: info.range.end,
            severity: info.severity,
        })
        .collect()
}

/// One pixel row (bucket) of the stripe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StripeBucket {
    pub index: usize,
    /// The most severe mark in the bucket.
    pub severity: Severity,
    pub count: usize,
}

/// Buckets `marks` into `buckets` rows of a file of `total_lines` lines;
/// `line_of` maps a byte offset to its line. Empty buckets are omitted.
pub fn bucketize(
    marks: &[StripeMark],
    total_lines: usize,
    buckets: usize,
    line_of: impl Fn(u32) -> usize,
) -> Vec<StripeBucket> {
    if buckets == 0 || total_lines == 0 {
        return Vec::new();
    }
    let mut rows: Vec<Option<StripeBucket>> = vec![None; buckets];
    for mark in marks {
        let line = line_of(mark.start).min(total_lines - 1);
        let index = (line * buckets / total_lines).min(buckets - 1);
        match &mut rows[index] {
            Some(bucket) => {
                bucket.severity = bucket.severity.max(mark.severity);
                bucket.count += 1;
            }
            slot => {
                *slot = Some(StripeBucket {
                    index,
                    severity: mark.severity,
                    count: 1,
                })
            }
        }
    }
    rows.into_iter().flatten().collect()
}

/// The status-bar summary: the most severe problem, if any.
pub fn top_severity(markup: &FileMarkup) -> Option<Severity> {
    markup.top_severity()
}

#[cfg(test)]
mod tests {
    use super::*;
    use archipel_ide_analysis::{FileRev, HighlightInfo, ToolId};

    #[test]
    fn marks_are_bucketed_by_line_with_the_worst_severity_winning() {
        let tool = ToolId::native("t");
        let mut markup = FileMarkup::new(FileRev(1));
        let mk = |range: std::ops::Range<u32>, severity| {
            HighlightInfo::new(range, severity, HighlightKind::Warning, tool.clone())
        };
        markup.replace(
            &tool,
            0..u32::MAX,
            vec![
                mk(0..1, Severity::WARNING),
                mk(5..6, Severity::ERROR),
                mk(95..96, Severity::WEAK_WARNING),
                mk(50..51, Severity::INFORMATION),
            ],
        );
        let marks = stripe_marks(&markup);
        assert_eq!(marks.len(), 3, "information-level problems have no tick");
        // 100 lines of one byte each, 10 buckets.
        let buckets = bucketize(&marks, 100, 10, |offset| offset as usize);
        assert_eq!(buckets.len(), 2);
        assert_eq!(
            (buckets[0].index, buckets[0].severity, buckets[0].count),
            (0, Severity::ERROR, 2)
        );
        assert_eq!(buckets[1].index, 9);
        assert!(bucketize(&marks, 0, 10, |o| o as usize).is_empty());
        assert_eq!(top_severity(&markup), Some(Severity::ERROR));
    }
}
