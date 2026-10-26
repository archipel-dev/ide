//! Project-level "files with problems" (`ProblemFiles` / the Wolf analogue):
//! per-file counts derived from the markup, plus the ancestor directories of
//! files with errors for the project tree and editor tabs.

use crate::markup::{FileMarkup, MarkupCounts};
use archipel_ide_analysis::{FileId, Severity};
use std::collections::BTreeMap;

/// Summary of one file.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FileSummary {
    pub counts: MarkupCounts,
    pub top: Option<Severity>,
}

/// See the module docs.
#[derive(Debug, Clone, Default)]
pub struct ProblemFiles {
    files: BTreeMap<FileId, FileSummary>,
    version: u64,
}

impl ProblemFiles {
    pub fn new() -> Self {
        Self::default()
    }

    /// Recomputes `file` from its markup (`None` = the file is gone).
    pub fn update(&mut self, file: FileId, markup: Option<&FileMarkup>) {
        let summary = markup.map(|m| FileSummary {
            counts: m.counts(),
            top: m.top_severity(),
        });
        let changed = match summary {
            Some(summary) if summary.counts.total() > 0 => {
                self.files.insert(file, summary) != Some(summary)
            }
            _ => self.files.remove(&file).is_some(),
        };
        if changed {
            self.version += 1;
        }
    }

    pub fn summary(&self, file: FileId) -> Option<FileSummary> {
        self.files.get(&file).copied()
    }

    /// Files with at least one listed problem.
    pub fn files(&self) -> impl Iterator<Item = (FileId, FileSummary)> + '_ {
        self.files.iter().map(|(file, summary)| (*file, *summary))
    }

    pub fn files_with_errors(&self) -> impl Iterator<Item = FileId> + '_ {
        self.files
            .iter()
            .filter(|(_, s)| s.counts.errors > 0)
            .map(|(file, _)| *file)
    }

    pub fn counts(&self) -> MarkupCounts {
        let mut total = MarkupCounts::default();
        for summary in self.files.values() {
            total.errors += summary.counts.errors;
            total.warnings += summary.counts.warnings;
            total.weak_warnings += summary.counts.weak_warnings;
        }
        total
    }

    /// Bumped when any summary changed.
    pub fn version(&self) -> u64 {
        self.version
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use archipel_ide_analysis::{FileRev, HighlightInfo, HighlightKind, ToolId};

    fn markup(severities: &[Severity]) -> FileMarkup {
        let mut m = FileMarkup::new(FileRev(1));
        let tool = ToolId::native("t");
        let infos = severities
            .iter()
            .enumerate()
            .map(|(i, s)| {
                HighlightInfo::new(
                    i as u32..i as u32 + 1,
                    *s,
                    HighlightKind::Warning,
                    tool.clone(),
                )
            })
            .collect();
        m.replace(&tool, 0..u32::MAX, infos);
        m
    }

    #[test]
    fn summaries_follow_the_markup() {
        let mut files = ProblemFiles::new();
        let (a, b) = (FileId(1), FileId(2));
        files.update(a, Some(&markup(&[Severity::ERROR, Severity::WARNING])));
        files.update(
            b,
            Some(&markup(&[Severity::WEAK_WARNING, Severity::INFORMATION])),
        );
        assert_eq!(files.files_with_errors().collect::<Vec<_>>(), [a]);
        assert_eq!(
            files.counts(),
            MarkupCounts {
                errors: 1,
                warnings: 1,
                weak_warnings: 1
            }
        );
        let version = files.version();
        files.update(a, Some(&markup(&[Severity::ERROR, Severity::WARNING])));
        assert_eq!(files.version(), version, "no change, no new version");
        files.update(a, None);
        assert!(files.summary(a).is_none());
        assert_eq!(files.files().count(), 1);
        // Information-only files are not listed.
        files.update(b, Some(&markup(&[Severity::INFORMATION])));
        assert_eq!(files.files().count(), 0);
    }
}
