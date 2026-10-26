//! The markup model (`MarkupModel` analogue): a sorted range store of
//! [`HighlightInfo`]s per file.
//!
//! - **Per-tool replace.** Every entry has an *owner* (the pass or tool whose
//!   slice it belongs to). `replace(owner, range, infos)` swaps the owner's
//!   entries that *start* in the half-open `range` for `infos` atomically.
//! - **Recycler.** An incoming info identical to an old one of the same slice
//!   (same range, kind and message) keeps the old entry and its
//!   [`HighlightId`], so the editor sees no flicker and ids stay stable.
//! - **Edit shifting.** [`FileMarkup::shift`] moves every range through an
//!   [`EditMap`]; ranges never leave the text and never split a character.
//! - **Budget.** A tool may own at most `per_tool` entries and a file
//!   `per_file`; the rest collapses into one file-level "too many problems"
//!   info.

use archipel_ide_analysis::daemon::HighlightMarkup;
use archipel_ide_analysis::{FileId, FileRev, HighlightInfo, HighlightKind, Severity, ToolId};
use archipel_ide_language::EditMap;
use std::collections::HashMap;
use std::ops::Range;

/// Stable identity of a stored highlight (survives recycling and shifting).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct HighlightId(pub u64);

/// Entry caps (spec 1.7 rule 7).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MarkupLimits {
    pub per_tool: usize,
    pub per_file: usize,
}

impl Default for MarkupLimits {
    fn default() -> Self {
        Self {
            per_tool: 1000,
            per_file: 2000,
        }
    }
}

#[derive(Debug, Clone)]
struct Entry {
    id: HighlightId,
    owner: ToolId,
    info: HighlightInfo,
}

/// What a `replace` did.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ReplaceStats {
    pub kept: usize,
    pub added: usize,
    pub removed: usize,
    /// Infos dropped by the budget.
    pub truncated: usize,
}

/// Counts of listed problems.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MarkupCounts {
    pub errors: usize,
    pub warnings: usize,
    pub weak_warnings: usize,
}

impl MarkupCounts {
    pub fn total(self) -> usize {
        self.errors + self.warnings + self.weak_warnings
    }
}

/// Highlights of one file revision.
#[derive(Debug, Clone)]
pub struct FileMarkup {
    rev: FileRev,
    entries: Vec<Entry>,
    /// `max_end[i]` = largest `range.end` among `entries[..=i]` (monotone).
    max_end: Vec<u32>,
    next_id: u64,
    limits: MarkupLimits,
}

fn floor_boundary(text: &str, mut at: usize) -> usize {
    at = at.min(text.len());
    while !text.is_char_boundary(at) {
        at -= 1;
    }
    at
}

fn ceil_boundary(text: &str, mut at: usize) -> usize {
    at = at.min(text.len());
    while !text.is_char_boundary(at) {
        at += 1;
    }
    at
}

impl FileMarkup {
    pub fn new(rev: FileRev) -> Self {
        Self::with_limits(rev, MarkupLimits::default())
    }

    pub fn with_limits(rev: FileRev, limits: MarkupLimits) -> Self {
        Self {
            rev,
            entries: Vec::new(),
            max_end: Vec::new(),
            next_id: 1,
            limits,
        }
    }

    /// The revision the stored ranges refer to.
    pub fn rev(&self) -> FileRev {
        self.rev
    }

    pub fn set_rev(&mut self, rev: FileRev) {
        self.rev = rev;
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    fn reindex(&mut self) {
        self.entries
            .sort_by_key(|e| (e.info.range.start, e.info.range.end, e.id));
        self.max_end.clear();
        let mut max = 0;
        for entry in &self.entries {
            max = max.max(entry.info.range.end);
            self.max_end.push(max);
        }
    }

    /// Replaces `owner`'s entries starting in `range` by `infos`; see the
    /// module docs. Infos starting outside `range` are ignored.
    pub fn replace(
        &mut self,
        owner: &ToolId,
        range: Range<u32>,
        infos: Vec<HighlightInfo>,
    ) -> ReplaceStats {
        let mut stats = ReplaceStats::default();
        let in_slice = |start: u32| range.start <= start && start < range.end;
        let mut infos: Vec<HighlightInfo> = infos
            .into_iter()
            .filter(|info| in_slice(info.range.start) && info.range.start <= info.range.end)
            .collect();
        if infos.len() > self.limits.per_tool {
            infos.sort_by(|a, b| {
                b.severity
                    .cmp(&a.severity)
                    .then(a.range.start.cmp(&b.range.start))
            });
            stats.truncated += infos.len() - self.limits.per_tool;
            infos.truncate(self.limits.per_tool);
            infos.push(too_many(owner, stats.truncated));
        }
        // Old entries of this slice.
        let mut old: HashMap<(u32, u32), Vec<Entry>> = HashMap::new();
        let mut rest = Vec::with_capacity(self.entries.len());
        for entry in std::mem::take(&mut self.entries) {
            if entry.owner == *owner && in_slice(entry.info.range.start) {
                old.entry((entry.info.range.start, entry.info.range.end))
                    .or_default()
                    .push(entry);
            } else {
                rest.push(entry);
            }
        }
        let mut next_id = self.next_id;
        for info in infos {
            let key = (info.range.start, info.range.end);
            let recycled = old.get_mut(&key).and_then(|candidates| {
                let at = candidates.iter().position(|e| e.info.same_as(&info))?;
                Some(candidates.swap_remove(at))
            });
            match recycled {
                Some(mut entry) => {
                    entry.info = info;
                    rest.push(entry);
                    stats.kept += 1;
                }
                None => {
                    rest.push(Entry {
                        id: HighlightId(next_id),
                        owner: owner.clone(),
                        info,
                    });
                    next_id += 1;
                    stats.added += 1;
                }
            }
        }
        stats.removed = old.values().map(Vec::len).sum();
        self.next_id = next_id;
        self.entries = rest;
        // File budget: drop the least severe entries of this owner first.
        if self.entries.len() > self.limits.per_file {
            let excess = self.entries.len() - self.limits.per_file;
            let mut candidates: Vec<(Severity, std::cmp::Reverse<u32>, HighlightId)> = self
                .entries
                .iter()
                .filter(|e| !e.info.file_level)
                .map(|e| (e.info.severity, std::cmp::Reverse(e.info.range.start), e.id))
                .collect();
            candidates.sort();
            let drop: std::collections::HashSet<HighlightId> =
                candidates.into_iter().take(excess).map(|c| c.2).collect();
            stats.truncated += drop.len();
            self.entries.retain(|e| !drop.contains(&e.id));
        }
        self.reindex();
        stats
    }

    /// Removes every entry of `owner`.
    pub fn clear_owner(&mut self, owner: &ToolId) -> usize {
        self.retain(|entry_owner, _| entry_owner != owner)
    }

    /// Keeps the entries `keep(owner, info)` accepts; returns how many went.
    pub fn retain(&mut self, mut keep: impl FnMut(&ToolId, &HighlightInfo) -> bool) -> usize {
        let before = self.entries.len();
        self.entries.retain(|e| keep(&e.owner, &e.info));
        self.reindex();
        before - self.entries.len()
    }

    /// Moves every range through `map` to the text `text` (revision `rev`).
    /// Ranges that were non-empty and collapsed are dropped; the rest are
    /// clamped to the text and snapped outward to character boundaries.
    pub fn shift(&mut self, map: &EditMap, rev: FileRev, text: &str) {
        let fix = |range: &Range<u32>| -> Range<u32> {
            let mapped = map.map_range(range.start as usize..range.end as usize);
            let start = floor_boundary(text, mapped.start);
            let end = ceil_boundary(text, mapped.end).max(start);
            start as u32..end as u32
        };
        self.entries.retain_mut(|entry| {
            let old = entry.info.range.clone();
            let new = fix(&old);
            if !old.is_empty() && new.is_empty() && !entry.info.file_level {
                return false;
            }
            entry.info.range = new;
            if let Some(gutter) = &entry.info.problem_range_for_gutter {
                entry.info.problem_range_for_gutter = Some(fix(gutter));
            }
            true
        });
        self.rev = rev;
        self.reindex();
    }

    /// Clamps every range into `text` and snaps to character boundaries
    /// (defensive: tools may report ranges for a text they misjudged).
    pub fn sanitize(&mut self, text: &str) {
        self.shift(&EditMap::new(), self.rev, text);
    }

    /// Highlights intersecting `range` (an empty `range` = touching it).
    pub fn query(&self, range: Range<u32>) -> Vec<&HighlightInfo> {
        self.query_entries(range).map(|e| &e.info).collect()
    }

    /// Like [`FileMarkup::query`] with ids and owners.
    pub fn query_with_ids(&self, range: Range<u32>) -> Vec<(HighlightId, &ToolId, &HighlightInfo)> {
        self.query_entries(range)
            .map(|e| (e.id, &e.owner, &e.info))
            .collect()
    }

    fn query_entries(&self, range: Range<u32>) -> impl Iterator<Item = &Entry> {
        let hi = self
            .entries
            .partition_point(|e| e.info.range.start < range.end.max(range.start.saturating_add(1)));
        let lo = self.max_end.partition_point(|&end| end < range.start);
        let q = range;
        self.entries[lo.min(hi)..hi]
            .iter()
            .filter(move |e| overlaps(&e.info.range, &q))
    }

    /// Every highlight, ordered by start.
    pub fn iter(&self) -> impl Iterator<Item = &HighlightInfo> {
        self.entries.iter().map(|e| &e.info)
    }

    pub fn iter_with_ids(&self) -> impl Iterator<Item = (HighlightId, &ToolId, &HighlightInfo)> {
        self.entries.iter().map(|e| (e.id, &e.owner, &e.info))
    }

    /// Highlights with severity at least `WEAK_WARNING` (the listed problems).
    pub fn problems(&self) -> impl Iterator<Item = &HighlightInfo> {
        self.iter().filter(|info| info.severity.is_problem())
    }

    pub fn counts(&self) -> MarkupCounts {
        let mut counts = MarkupCounts::default();
        for info in self.problems() {
            if info.severity >= Severity::ERROR {
                counts.errors += 1;
            } else if info.severity >= Severity::WARNING {
                counts.warnings += 1;
            } else {
                counts.weak_warnings += 1;
            }
        }
        counts
    }

    /// The most severe problem, if any.
    pub fn top_severity(&self) -> Option<Severity> {
        self.problems().map(|info| info.severity).max()
    }
}

fn overlaps(info: &Range<u32>, q: &Range<u32>) -> bool {
    if q.is_empty() {
        return info.start <= q.start && q.start <= info.end;
    }
    if info.is_empty() {
        return q.start <= info.start && info.start < q.end;
    }
    info.start < q.end && info.end > q.start
}

fn too_many(owner: &ToolId, dropped: usize) -> HighlightInfo {
    let mut info = HighlightInfo::new(
        0..0,
        Severity::WEAK_WARNING,
        HighlightKind::WeakWarning,
        owner.clone(),
    )
    .message(format!("Too many problems: {dropped} more are not shown"));
    info.file_level = true;
    info
}

impl HighlightMarkup for FileMarkup {
    fn replace(&mut self, tool: &ToolId, range: Range<u32>, infos: Vec<HighlightInfo>) {
        FileMarkup::replace(self, tool, range, infos);
    }

    fn query(&self, range: Range<u32>) -> Vec<&HighlightInfo> {
        FileMarkup::query(self, range)
    }
}

/// The markup of every file the daemon covers.
#[derive(Debug, Default)]
pub struct MarkupStore {
    files: HashMap<FileId, FileMarkup>,
    limits: MarkupLimits,
    version: u64,
}

impl MarkupStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_limits(limits: MarkupLimits) -> Self {
        Self {
            limits,
            ..Self::default()
        }
    }

    /// Bumped by every mutation (a cheap "did anything change" for caches).
    pub fn version(&self) -> u64 {
        self.version
    }

    pub fn touch(&mut self) {
        self.version += 1;
    }

    pub fn get(&self, file: FileId) -> Option<&FileMarkup> {
        self.files.get(&file)
    }

    pub fn get_mut(&mut self, file: FileId) -> Option<&mut FileMarkup> {
        self.version += 1;
        self.files.get_mut(&file)
    }

    pub fn ensure(&mut self, file: FileId, rev: FileRev) -> &mut FileMarkup {
        self.version += 1;
        let limits = self.limits;
        self.files
            .entry(file)
            .or_insert_with(|| FileMarkup::with_limits(rev, limits))
    }

    pub fn remove(&mut self, file: FileId) -> Option<FileMarkup> {
        self.version += 1;
        self.files.remove(&file)
    }

    pub fn files(&self) -> impl Iterator<Item = (FileId, &FileMarkup)> {
        self.files.iter().map(|(file, markup)| (*file, markup))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use archipel_ide_language::ByteEdit;

    pub(crate) fn info(range: Range<u32>, severity: Severity, msg: &str) -> HighlightInfo {
        HighlightInfo::new(range, severity, HighlightKind::Warning, ToolId::native("t"))
            .message(msg)
    }

    fn edit(start: usize, old_end: usize, new_end: usize) -> ByteEdit {
        let p = archipel_ide_language::BytePoint { row: 0, column: 0 };
        ByteEdit {
            start_byte: start,
            old_end_byte: old_end,
            new_end_byte: new_end,
            start_point: p,
            old_end_point: p,
            new_end_point: p,
        }
    }

    #[test]
    fn replace_is_per_owner_and_per_slice() {
        let a = ToolId::native("a");
        let b = ToolId::native("b");
        let mut m = FileMarkup::new(FileRev(1));
        m.replace(
            &a,
            0..u32::MAX,
            vec![
                info(0..2, Severity::WARNING, "a1"),
                info(50..52, Severity::WARNING, "a2"),
            ],
        );
        m.replace(&b, 0..u32::MAX, vec![info(1..3, Severity::ERROR, "b1")]);
        assert_eq!(m.len(), 3);
        // Replace only a's visible slice 0..10.
        let stats = m.replace(&a, 0..10, vec![info(4..5, Severity::WARNING, "a3")]);
        assert_eq!((stats.added, stats.removed, stats.kept), (1, 1, 0));
        let messages: Vec<_> = m
            .iter()
            .map(|i| i.message.clone().unwrap().to_string())
            .collect();
        assert_eq!(messages, ["b1", "a3", "a2"]);
        // Infos outside the slice are ignored.
        m.replace(&a, 0..10, vec![info(70..71, Severity::WARNING, "out")]);
        assert!(m.iter().all(|i| i.message.as_deref() != Some("out")));
        assert_eq!(
            m.counts(),
            MarkupCounts {
                errors: 1,
                warnings: 1,
                weak_warnings: 0
            }
        );
    }

    #[test]
    fn identical_infos_are_recycled_with_stable_ids() {
        let a = ToolId::native("a");
        let mut m = FileMarkup::new(FileRev(1));
        m.replace(
            &a,
            0..u32::MAX,
            vec![
                info(0..2, Severity::WARNING, "x"),
                info(5..6, Severity::WARNING, "y"),
            ],
        );
        let ids: Vec<_> = m.iter_with_ids().map(|(id, _, _)| id).collect();
        let stats = m.replace(
            &a,
            0..u32::MAX,
            vec![
                info(5..6, Severity::ERROR, "y"),
                info(0..2, Severity::WARNING, "x"),
                info(8..9, Severity::WARNING, "z"),
            ],
        );
        assert_eq!((stats.kept, stats.added, stats.removed), (2, 1, 0));
        let after: Vec<_> = m
            .iter_with_ids()
            .map(|(id, _, i)| (id, i.severity))
            .collect();
        assert_eq!(after[0].0, ids[0]);
        assert_eq!(
            after[1],
            (ids[1], Severity::ERROR),
            "recycled entry takes the new payload"
        );
    }

    #[test]
    fn query_finds_overlaps_including_long_ranges_and_empty_infos() {
        let a = ToolId::native("a");
        let mut m = FileMarkup::new(FileRev(1));
        m.replace(
            &a,
            0..u32::MAX,
            vec![
                info(0..100, Severity::WARNING, "long"),
                info(10..12, Severity::WARNING, "short"),
                info(30..30, Severity::ERROR, "empty"),
            ],
        );
        let msgs = |r: Range<u32>| -> Vec<String> {
            m.query(r)
                .iter()
                .map(|i| i.message.clone().unwrap().to_string())
                .collect()
        };
        assert_eq!(msgs(50..60), ["long"]);
        assert_eq!(msgs(11..11), ["long", "short"]);
        assert_eq!(msgs(29..31), ["long", "empty"]);
        assert_eq!(msgs(100..120), Vec::<String>::new());
    }

    #[test]
    fn edits_shift_highlights_and_drop_deleted_ones() {
        let a = ToolId::native("a");
        let mut m = FileMarkup::new(FileRev(1));
        m.replace(
            &a,
            0..u32::MAX,
            vec![
                info(2..4, Severity::WARNING, "x"),
                info(6..8, Severity::WARNING, "y"),
            ],
        );
        // Insert 3 bytes at 0, delete 5..9 (covers y entirely).
        let mut map = EditMap::from_edits(&[edit(0, 0, 3)]);
        map.push(&edit(8, 12, 8));
        m.shift(&map, FileRev(3), "abcdefgh");
        let ranges: Vec<_> = m.iter().map(|i| i.range.clone()).collect();
        assert_eq!(ranges.len(), 1);
        assert_eq!(ranges[0], 5..7);
        assert_eq!(m.rev(), FileRev(3));
    }

    #[test]
    fn shifting_never_splits_characters() {
        let a = ToolId::native("a");
        let mut m = FileMarkup::new(FileRev(1));
        m.replace(&a, 0..u32::MAX, vec![info(1..3, Severity::WARNING, "x")]);
        // New text "aäb": 'ä' occupies 1..3 -> fine; force a bad mapping.
        m.shift(&EditMap::from_edits(&[edit(0, 0, 0)]), FileRev(2), "ää");
        let r = m.iter().next().unwrap().range.clone();
        assert!("ää".is_char_boundary(r.start as usize) && "ää".is_char_boundary(r.end as usize));
    }

    #[test]
    fn budgets_collapse_into_a_file_level_marker() {
        let a = ToolId::native("a");
        let mut m = FileMarkup::with_limits(
            FileRev(1),
            MarkupLimits {
                per_tool: 3,
                per_file: 100,
            },
        );
        let infos: Vec<_> = (0..10)
            .map(|i| info(i..i + 1, Severity::WARNING, "w"))
            .collect();
        let stats = m.replace(&a, 0..u32::MAX, infos);
        assert_eq!(stats.truncated, 7);
        assert_eq!(m.len(), 4);
        assert!(
            m.iter()
                .any(|i| i.file_level && i.message.as_deref().unwrap().contains("7 more"))
        );
        let mut m = FileMarkup::with_limits(
            FileRev(1),
            MarkupLimits {
                per_tool: 100,
                per_file: 5,
            },
        );
        let mut infos: Vec<_> = (0..8)
            .map(|i| info(i..i + 1, Severity::WARNING, "w"))
            .collect();
        infos.push(info(20..21, Severity::ERROR, "err"));
        m.replace(&a, 0..u32::MAX, infos);
        assert_eq!(m.len(), 5);
        assert!(
            m.iter().any(|i| i.severity == Severity::ERROR),
            "errors survive the cap"
        );
    }
}
