//! Client-side completion filtering and ranking.
//!
//! Matching follows the observable behaviour of IntelliJ's completion: the
//! typed prefix is split into fragments that each start at a word start
//! ("hump") of the candidate and continue consecutively, so `gRF` matches
//! `getRequiredField` and `nam` matches `getName`, but `ame` does not.
//! Lower-case pattern letters match either case; upper-case pattern letters
//! must match an upper-case letter or a word start.
//!
//! Ranking puts exact and prefix matches first, then keeps the server's
//! relevance (`sortText`), then prefers tighter matches.

use lsp_types::{CompletionItem, CompletionItemKind, CompletionItemTag, CompletionTextEdit};
use std::cmp::Ordering;

/// Result of matching a pattern against a candidate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FuzzyMatch {
    /// Higher is better.
    pub score: i32,
    /// Char indices of the candidate that matched, ascending.
    pub positions: Vec<usize>,
}

/// How a candidate matched, best first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MatchClass {
    Exact,
    CaseSensitivePrefix,
    Prefix,
    /// First fragment at the start, later fragments at humps (`gRF`).
    CamelHump,
    /// First fragment at a later hump (`name` in `getName`).
    Inner,
}

fn is_word_start(chars: &[char], index: usize) -> bool {
    let Some(&current) = chars.get(index) else {
        return false;
    };
    if index == 0 {
        return true;
    }
    let previous = chars[index - 1];
    if !current.is_alphanumeric() {
        return false;
    }
    if !previous.is_alphanumeric() {
        return true;
    }
    if current.is_uppercase() && !previous.is_uppercase() {
        return true;
    }
    // "HTTPServer": the S starts "Server".
    if current.is_uppercase()
        && previous.is_uppercase()
        && chars.get(index + 1).is_some_and(|next| next.is_lowercase())
    {
        return true;
    }
    current.is_ascii_digit() != previous.is_ascii_digit()
}

fn chars_equal_ignore_case(a: char, b: char) -> bool {
    a == b || a.to_lowercase().eq(b.to_lowercase())
}

/// Matches `pattern` against `candidate` with camel-hump semantics.
pub fn fuzzy_match(pattern: &str, candidate: &str) -> Option<FuzzyMatch> {
    let pattern: Vec<char> = pattern.chars().collect();
    let candidate: Vec<char> = candidate.chars().collect();
    if pattern.is_empty() {
        return Some(FuzzyMatch {
            score: 0,
            positions: Vec::new(),
        });
    }
    if pattern.len() > candidate.len() || pattern.len() > 64 || candidate.len() > 512 {
        return None;
    }
    let starts: Vec<bool> = (0..candidate.len())
        .map(|i| is_word_start(&candidate, i))
        .collect();
    let mut matcher = Matcher {
        pattern: &pattern,
        candidate: &candidate,
        starts: &starts,
        memo: vec![None; (pattern.len() + 1) * (candidate.len() + 1) * 2],
    };
    let score = matcher.best(0, 0, false)?;
    let positions = matcher.positions();
    Some(FuzzyMatch { score, positions })
}

struct Matcher<'a> {
    pattern: &'a [char],
    candidate: &'a [char],
    starts: &'a [bool],
    /// Best (score, matched-at) for (pattern index, candidate index, continuing).
    memo: Vec<Option<Option<(i32, usize)>>>,
}

impl Matcher<'_> {
    fn key(&self, i: usize, j: usize, continuing: bool) -> usize {
        (i * (self.candidate.len() + 1) + j) * 2 + usize::from(continuing)
    }

    /// Can pattern[i] match candidate[j]?
    fn can_match(&self, i: usize, j: usize, continuing: bool) -> bool {
        let (p, c) = (self.pattern[i], self.candidate[j]);
        if !chars_equal_ignore_case(p, c) {
            return false;
        }
        if continuing {
            // Inside a fragment an upper-case pattern letter needs an
            // upper-case candidate letter.
            !p.is_uppercase() || c.is_uppercase()
        } else {
            self.starts[j]
        }
    }

    fn score_at(&self, i: usize, j: usize, continuing: bool) -> i32 {
        let mut score = if continuing { 6 } else { 2 };
        if self.pattern[i] == self.candidate[j] {
            score += 1;
        }
        if !continuing && j == 0 {
            score += 8;
        }
        if !continuing && i > 0 {
            // Every extra fragment costs a little.
            score -= 3;
        }
        score
    }

    /// Best score for matching pattern[i..] from candidate[j..].
    fn best(&mut self, i: usize, j: usize, continuing: bool) -> Option<i32> {
        if i == self.pattern.len() {
            return Some(0);
        }
        let key = self.key(i, j, continuing);
        if let Some(cached) = self.memo[key] {
            return cached.map(|(score, _)| score);
        }
        let mut best: Option<(i32, usize)> = None;
        // Continue the current fragment.
        if continuing
            && j < self.candidate.len()
            && self.can_match(i, j, true)
            && let Some(rest) = self.best(i + 1, j + 1, true)
        {
            best = Some((rest + self.score_at(i, j, true), j));
        }
        // Start a new fragment at a later hump. The first fragment must start
        // at a hump too.
        let mut skipped_humps = 0;
        for k in j..self.candidate.len() {
            if self.can_match(i, k, false)
                && let Some(rest) = self.best(i + 1, k + 1, true)
            {
                let score = rest + self.score_at(i, k, false) - skipped_humps;
                if best.is_none_or(|(current, _)| score > current) {
                    best = Some((score, k));
                }
            }
            if self.starts[k] && k > j {
                skipped_humps += 1;
            }
        }
        self.memo[key] = Some(best);
        best.map(|(score, _)| score)
    }

    fn positions(&self) -> Vec<usize> {
        let mut positions = Vec::with_capacity(self.pattern.len());
        let (mut j, mut continuing) = (0, false);
        for i in 0..self.pattern.len() {
            let Some(Some((_, at))) = self.memo[self.key(i, j, continuing)] else {
                break;
            };
            positions.push(at);
            j = at + 1;
            continuing = true;
        }
        positions
    }
}

/// Classifies a successful match.
pub fn match_class(pattern: &str, candidate: &str, matched: &FuzzyMatch) -> MatchClass {
    if pattern == candidate {
        return MatchClass::Exact;
    }
    if candidate.starts_with(pattern) {
        return MatchClass::CaseSensitivePrefix;
    }
    let count = pattern.chars().count();
    if matched
        .positions
        .iter()
        .enumerate()
        .all(|(index, &position)| index == position)
        && matched.positions.len() == count
    {
        return MatchClass::Prefix;
    }
    if matched.positions.first() == Some(&0) {
        MatchClass::CamelHump
    } else {
        MatchClass::Inner
    }
}

/// The text a completion item is filtered by: the identifier part of
/// `filterText`/`label` (jdtls appends signatures to its filter text).
pub fn filter_text(item: &CompletionItem) -> &str {
    let text = item.filter_text.as_deref().unwrap_or(&item.label);
    let end = text
        .char_indices()
        .find(|(_, c)| matches!(c, '(' | ' ' | ':' | '<'))
        .map_or(text.len(), |(index, _)| index);
    if end == 0 { text } else { &text[..end] }
}

/// One visible row of the completion popup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RankedItem {
    /// Index into the server's item list.
    pub index: usize,
    pub class: MatchClass,
    pub score: i32,
    /// Matched char indices in the item's label (for bold rendering).
    pub label_positions: Vec<usize>,
}

/// Filters `items` by `prefix` and sorts them for display.
///
/// `relevance` lets a language provider demote items (e.g. members inherited
/// from `java.lang.Object`); higher sorts first.
pub fn rank(
    items: &[CompletionItem],
    prefix: &str,
    relevance: impl Fn(&CompletionItem) -> i32,
) -> Vec<RankedItem> {
    rank_with_usage(items, prefix, relevance, |_| 0)
}

/// Like [`rank`], with `usage` (completion statistics: how often the user
/// picked an item) ranking frequently chosen items first among equally good
/// matches, before the server's relevance.
pub fn rank_with_usage(
    items: &[CompletionItem],
    prefix: &str,
    relevance: impl Fn(&CompletionItem) -> i32,
    usage: impl Fn(&CompletionItem) -> i32,
) -> Vec<RankedItem> {
    let mut ranked: Vec<(RankedItem, (i32, i32))> = items
        .iter()
        .enumerate()
        .filter_map(|(index, item)| {
            let text = filter_text(item);
            let matched = fuzzy_match(prefix, text)?;
            let class = if prefix.is_empty() {
                MatchClass::Prefix
            } else {
                match_class(prefix, text, &matched)
            };
            let label_positions = if item.label.starts_with(text) {
                matched.positions.clone()
            } else {
                fuzzy_match(prefix, &item.label)
                    .map(|m| m.positions)
                    .unwrap_or_default()
            };
            Some((
                RankedItem {
                    index,
                    class,
                    score: matched.score,
                    label_positions,
                },
                (usage(item), relevance(item)),
            ))
        })
        .collect();
    ranked.sort_by(|(a, (a_usage, a_relevance)), (b, (b_usage, b_relevance))| {
        let (item_a, item_b) = (&items[a.index], &items[b.index]);
        // Exact and prefix matches first, as in the reference IDE; camel-hump
        // and inner matches are grouped after them.
        let group = |class: MatchClass| match class {
            MatchClass::Exact => 0,
            MatchClass::CaseSensitivePrefix | MatchClass::Prefix => 1,
            MatchClass::CamelHump => 2,
            MatchClass::Inner => 3,
        };
        group(a.class)
            .cmp(&group(b.class))
            .then_with(|| is_deprecated(item_a).cmp(&is_deprecated(item_b)))
            // Templates (postfix `nnull`, `sysout`, …) after real members.
            .then_with(|| is_snippet(item_a).cmp(&is_snippet(item_b)))
            .then_with(|| b_usage.cmp(a_usage))
            .then_with(|| b_relevance.cmp(a_relevance))
            .then_with(|| compare_sort_text(item_a, item_b))
            .then_with(|| a.class.cmp(&b.class))
            .then_with(|| b.score.cmp(&a.score))
            .then_with(|| compare_labels(&item_a.label, &item_b.label))
            .then_with(|| a.index.cmp(&b.index))
    });
    ranked.into_iter().map(|(item, _)| item).collect()
}

/// Server order (`sortText`, falling back to the label, case-insensitively).
fn compare_sort_text(a: &CompletionItem, b: &CompletionItem) -> Ordering {
    match (&a.sort_text, &b.sort_text) {
        (Some(a), Some(b)) => a.cmp(b),
        _ => compare_labels(
            a.sort_text.as_deref().unwrap_or(&a.label),
            b.sort_text.as_deref().unwrap_or(&b.label),
        ),
    }
}

fn compare_labels(a: &str, b: &str) -> Ordering {
    a.to_lowercase()
        .cmp(&b.to_lowercase())
        .then_with(|| a.cmp(b))
}

fn is_snippet(item: &CompletionItem) -> bool {
    item.kind == Some(CompletionItemKind::SNIPPET)
}

#[allow(deprecated)] // `deprecated` is the pre-3.15 field some servers still send.
pub fn is_deprecated(item: &CompletionItem) -> bool {
    item.deprecated == Some(true)
        || item
            .tags
            .as_ref()
            .is_some_and(|tags| tags.contains(&CompletionItemTag::DEPRECATED))
}

/// Text inserted when the item is accepted, ignoring ranges.
pub fn insert_text(item: &CompletionItem) -> &str {
    match &item.text_edit {
        Some(CompletionTextEdit::Edit(edit)) => &edit.new_text,
        Some(CompletionTextEdit::InsertAndReplace(edit)) => &edit.new_text,
        None => item.insert_text.as_deref().unwrap_or(&item.label),
    }
}

/// Signature-like detail shown in grey right after the label (`(String name)`).
pub fn label_detail(item: &CompletionItem) -> Option<&str> {
    item.label_details
        .as_ref()
        .and_then(|details| details.detail.as_deref())
        .filter(|detail| !detail.is_empty())
}

/// Type shown right-aligned (`String`, `fn(&self) -> usize`).
pub fn type_text(item: &CompletionItem) -> Option<&str> {
    item.label_details
        .as_ref()
        .and_then(|details| details.description.as_deref())
        .or(item.detail.as_deref())
        .filter(|text| !text.is_empty())
}

/// Short kind name for icons (`method`, `field`, ...).
pub fn kind_name(kind: Option<CompletionItemKind>) -> &'static str {
    match kind {
        Some(CompletionItemKind::METHOD) => "method",
        Some(CompletionItemKind::FUNCTION) => "function",
        Some(CompletionItemKind::CONSTRUCTOR) => "constructor",
        Some(CompletionItemKind::FIELD) => "field",
        Some(CompletionItemKind::VARIABLE) => "variable",
        Some(CompletionItemKind::CLASS) => "class",
        Some(CompletionItemKind::STRUCT) => "struct",
        Some(CompletionItemKind::INTERFACE) => "interface",
        Some(CompletionItemKind::MODULE) => "module",
        Some(CompletionItemKind::PROPERTY) => "property",
        Some(CompletionItemKind::ENUM) => "enum",
        Some(CompletionItemKind::ENUM_MEMBER) => "enumMember",
        Some(CompletionItemKind::CONSTANT) => "constant",
        Some(CompletionItemKind::KEYWORD) => "keyword",
        Some(CompletionItemKind::SNIPPET) => "snippet",
        Some(CompletionItemKind::TYPE_PARAMETER) => "typeParameter",
        Some(CompletionItemKind::FILE | CompletionItemKind::FOLDER) => "file",
        Some(CompletionItemKind::VALUE) => "value",
        Some(CompletionItemKind::EVENT) => "event",
        Some(CompletionItemKind::OPERATOR) => "operator",
        Some(CompletionItemKind::REFERENCE) => "reference",
        _ => "text",
    }
}

/// Whether `c` can be part of an identifier being completed.
pub fn is_identifier_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '$'
}

#[cfg(test)]
mod tests {
    use super::*;

    fn positions(pattern: &str, candidate: &str) -> Option<Vec<usize>> {
        fuzzy_match(pattern, candidate).map(|m| m.positions)
    }

    #[test]
    fn camel_humps_match_like_the_reference() {
        assert_eq!(positions("gRF", "getRequiredField"), Some(vec![0, 3, 11]));
        assert_eq!(positions("grf", "getRequiredField"), Some(vec![0, 3, 11]));
        assert_eq!(
            positions("getReq", "getRequiredField"),
            Some(vec![0, 1, 2, 3, 4, 5])
        );
        assert_eq!(positions("nam", "getName"), Some(vec![3, 4, 5]));
        assert_eq!(positions("ps", "push_str"), Some(vec![0, 5]));
        assert_eq!(positions("hs", "HTTPServer"), Some(vec![0, 4]));
        assert_eq!(positions("ame", "getName"), None);
        assert_eq!(positions("gRF", "getrequiredfield"), None);
        assert_eq!(positions("xyz", "getName"), None);
        assert_eq!(positions("", "anything"), Some(vec![]));
    }

    #[test]
    fn prefers_consecutive_runs_over_scattered_humps() {
        // "to" should take "toString"'s first two letters, not t + O-hump.
        assert_eq!(positions("to", "toObject"), Some(vec![0, 1]));
        let tight = fuzzy_match("get", "getClass").unwrap().score;
        let loose = fuzzy_match("gcl", "getClass").unwrap().score;
        assert!(tight > loose);
    }

    #[test]
    fn handles_unicode_and_long_inputs_without_panicking() {
        assert_eq!(positions("ä", "äpfel"), Some(vec![0]));
        assert!(fuzzy_match("a", &"a".repeat(10_000)).is_none());
        assert!(fuzzy_match(&"a".repeat(100), "a").is_none());
    }

    fn item(label: &str, sort: &str) -> CompletionItem {
        CompletionItem {
            label: label.into(),
            sort_text: Some(sort.into()),
            ..Default::default()
        }
    }

    fn labels(items: &[CompletionItem], prefix: &str) -> Vec<String> {
        rank(items, prefix, |_| 0)
            .into_iter()
            .map(|ranked| items[ranked.index].label.clone())
            .collect()
    }

    #[test]
    fn ranks_prefix_matches_before_camel_humps_and_keeps_server_order() {
        let items = vec![
            item("getRequiredField", "1"),
            item("greeting", "0"),
            item("getClass", "2"),
            item("hashCode", "3"),
            item("gRF", "9"),
        ];
        assert_eq!(
            labels(&items, ""),
            [
                "greeting",
                "getRequiredField",
                "getClass",
                "hashCode",
                "gRF"
            ]
        );
        assert_eq!(labels(&items, "gRF"), ["gRF", "getRequiredField"]);
        assert_eq!(labels(&items, "ge"), ["getRequiredField", "getClass"]);
        assert_eq!(
            labels(&items, "g"),
            ["greeting", "getRequiredField", "getClass", "gRF"]
        );
        assert_eq!(labels(&items, "code"), ["hashCode"]);
    }

    #[test]
    fn relevance_hook_and_deprecation_reorder_equal_items() {
        let mut items = vec![
            item("clone", "5"),
            item("greeting", "5"),
            item("finalize", "5"),
        ];
        #[allow(deprecated)]
        {
            items[0].deprecated = Some(false);
            items[2].deprecated = Some(true);
        }
        let ranked: Vec<_> = rank(&items, "", |item| i32::from(item.label == "greeting"))
            .into_iter()
            .map(|ranked| items[ranked.index].label.as_str())
            .collect();
        assert_eq!(ranked, ["greeting", "clone", "finalize"]);
    }

    #[test]
    fn frequently_chosen_items_rank_first_among_equal_matches() {
        let items = vec![
            item("greeting", "0"),
            item("getClass", "1"),
            item("hashCode", "2"),
        ];
        let used = |item: &CompletionItem| i32::from(item.label == "hashCode") * 4;
        let ranked: Vec<_> = rank_with_usage(&items, "", |_| 0, used)
            .into_iter()
            .map(|row| items[row.index].label.as_str())
            .collect();
        assert_eq!(ranked, ["hashCode", "greeting", "getClass"]);
        // A better match class still wins over usage.
        let ranked: Vec<_> = rank_with_usage(&items, "ge", |_| 0, used)
            .into_iter()
            .map(|row| items[row.index].label.as_str())
            .collect();
        assert_eq!(ranked, ["getClass"]);
    }

    #[test]
    fn templates_rank_after_members_unless_they_match_better() {
        let mut items = vec![
            item("nnull", "1"),
            item("notify", "5"),
            item("getClass", "5"),
        ];
        items[0].kind = Some(CompletionItemKind::SNIPPET);
        assert_eq!(labels(&items, ""), ["getClass", "notify", "nnull"]);
        assert_eq!(labels(&items, "nn"), ["nnull"]);
        assert_eq!(labels(&items, "n"), ["notify", "nnull"]);
    }

    #[test]
    fn filter_text_drops_signatures_appended_by_jdtls() {
        let mut jdtls = item("greeting", "1");
        jdtls.filter_text = Some("greeting(String name) : String".into());
        assert_eq!(filter_text(&jdtls), "greeting");
        let ra = item("push_str(…)", "1");
        assert_eq!(filter_text(&ra), "push_str");
        let ranked = rank(std::slice::from_ref(&jdtls), "gree", |_| 0);
        assert_eq!(ranked[0].label_positions, [0, 1, 2, 3]);
    }
}
