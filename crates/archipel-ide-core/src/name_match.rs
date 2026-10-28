//! The name matcher of Go to Class / File / Symbol and Search Everywhere (SRCH-MAT, spec
//! `docs/specs/search/07-matching-and-ranking.md`).
//!
//! A [`NameMatcher`] is built from a pattern, a [`MatchMode`] and optional hard separators and
//! answers, for one name, whether it matches, which fragments of the name matched (for the
//! highlighting) and a *matching degree* (higher is better). The matcher is pure and
//! deterministic and never panics on odd input (SRCH-MAT-036). Name positions are character
//! indices, not byte offsets.

use std::ops::Range;

/// How letter case takes part in matching (SRCH-MAT section 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MatchMode {
    /// Either case matches; case only changes the score.
    #[default]
    IgnoreCase,
    /// As `IgnoreCase`, but the first pattern letter must have the case of the name's first letter.
    FirstLetter,
    /// Letters must match exactly.
    MatchCase,
}

/// Bonus added by the Go to lists when the first fragment starts at index 0 (SRCH-MAT-034).
pub const PREFER_START_BONUS: i32 = 10_000;
/// Bonus of an exact qualified match in Go to Class and Go to Symbol (SRCH-MAT-035).
pub const EXACT_QUALIFIED_BONUS: i32 = 5_000;

/// Longer patterns match without camel humps (SRCH-MAT-004).
const MAX_HUMP_PATTERN: usize = 100;

type Frags = Vec<(usize, usize)>;

/// Whether a character separates words in a pattern or a name (`_ - : + .` and whitespace).
fn is_word_separator(c: char) -> bool {
    c.is_whitespace() || matches!(c, '_' | '-' | ':' | '+' | '.')
}

fn is_ideographic(c: char) -> bool {
    matches!(c as u32,
        0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xF900..=0xFAFF | 0x20000..=0x2FA1F | 0x3040..=0x30FF)
}

fn is_digit(c: char) -> bool {
    c.is_numeric()
}

fn is_upper(c: char) -> bool {
    c.is_uppercase()
}

fn is_lower(c: char) -> bool {
    c.is_lowercase()
}

fn lower(c: char) -> char {
    c.to_lowercase().next().unwrap_or(c)
}

fn upper(c: char) -> char {
    c.to_uppercase().next().unwrap_or(c)
}

/// The end of the hump (camel-hump segment) that starts at `start` (SRCH-MAT section 3).
fn next_hump(name: &[char], start: usize) -> usize {
    let Some(&c) = name.get(start) else {
        return start + 1;
    };
    if !c.is_alphabetic() || is_ideographic(c) {
        return start + 1;
    }
    let mut i = start;
    if is_upper(c) {
        let mut j = start;
        while j < name.len() && is_upper(name[j]) {
            j += 1;
        }
        if j - start > 1 {
            // `URLConnection`: the last capital begins the next hump.
            return if j < name.len() && is_lower(name[j]) {
                j - 1
            } else {
                j
            };
        }
        i = start + 1;
    }
    while i < name.len()
        && name[i].is_alphabetic()
        && !is_upper(name[i])
        && !is_ideographic(name[i])
    {
        i += 1;
    }
    i
}

/// Whether a word of `name` starts at `i` (SRCH-MAT section 3).
fn is_word_start(name: &[char], i: usize) -> bool {
    let Some(&c) = name.get(i) else {
        return false;
    };
    if is_ideographic(c) || is_digit(c) {
        return true;
    }
    if !c.is_alphabetic() {
        return false;
    }
    if is_upper(c) {
        if i > 0 && is_upper(name[i - 1]) {
            return name.get(i + 1).is_some_and(|n| is_lower(*n));
        }
        return true;
    }
    i == 0 || !name[i - 1].is_alphanumeric()
}

/// A compiled pattern.
#[derive(Debug, Clone)]
pub struct NameMatcher {
    pattern: Vec<char>,
    /// The pattern including a trailing literal space, for the score.
    full: Vec<char>,
    mode: MatchMode,
    hard_separators: Vec<char>,
    is_upper: Vec<bool>,
    is_lower: Vec<bool>,
    is_sep: Vec<bool>,
    has_humps: bool,
    has_dots: bool,
    /// `foo ` (not after a capital or digit): the name needs a space after the last fragment.
    trailing_space: bool,
    /// Index of the first non-wildcard pattern character.
    first_real: usize,
    min_name_len: usize,
}

impl NameMatcher {
    /// A matcher for `pattern`. A trailing `* ` is dropped (SRCH-MAT-001).
    pub fn new(pattern: &str, mode: MatchMode) -> Self {
        Self::with_separators(pattern, mode, &[])
    }

    /// Like [`NameMatcher::new`] with hard separators (SRCH-MAT-015) that code completion passes.
    pub fn with_separators(pattern: &str, mode: MatchMode, hard_separators: &[char]) -> Self {
        let mut chars: Vec<char> = pattern.chars().collect();
        while chars.len() >= 2 && chars[chars.len() - 2] == '*' && chars[chars.len() - 1] == ' ' {
            chars.truncate(chars.len() - 2);
        }
        let is_upper_v: Vec<bool> = chars.iter().map(|c| is_upper(*c)).collect();
        let is_lower_v: Vec<bool> = chars.iter().map(|c| is_lower(*c)).collect();
        let is_sep: Vec<bool> = chars.iter().map(|c| is_word_separator(*c)).collect();
        let first_real = chars
            .iter()
            .position(|c| !matches!(c, '*' | ' '))
            .unwrap_or(chars.len());
        let has_humps = is_upper_v.iter().skip(first_real + 1).any(|b| *b)
            && is_lower_v.iter().skip(first_real).any(|b| *b);
        let has_dots = chars.iter().skip(first_real).any(|c| *c == '.');
        let trailing_space = chars.len() >= 2
            && chars[chars.len() - 1] == ' '
            && !matches!(chars[chars.len() - 2], '*' | ' ')
            && !is_upper(chars[chars.len() - 2])
            && !is_digit(chars[chars.len() - 2]);
        let full = chars.clone();
        if trailing_space {
            chars.pop();
        }
        let min_name_len = chars.iter().filter(|c| !matches!(c, '*' | ' ')).count();
        Self {
            pattern: chars,
            full,
            mode,
            hard_separators: hard_separators.to_vec(),
            is_upper: is_upper_v,
            is_lower: is_lower_v,
            is_sep,
            has_humps,
            has_dots,
            trailing_space,
            first_real,
            min_name_len,
        }
    }

    /// The pattern the matcher was built from (without a dropped trailing `* `).
    pub fn pattern(&self) -> String {
        self.pattern.iter().collect()
    }

    pub fn is_empty(&self) -> bool {
        self.pattern.is_empty()
    }

    /// The fragments of `name` the pattern matches, or `None` (SRCH-MAT section 1).
    pub fn match_name(&self, name: &str) -> Option<Vec<Range<usize>>> {
        let chars: Vec<char> = name.chars().collect();
        self.match_chars(&chars)
            .map(|f| f.into_iter().map(|(s, e)| s..e).collect())
    }

    /// Fragments and degree of `name`, or `None` when it does not match.
    pub fn match_with_degree(&self, name: &str) -> Option<(Vec<Range<usize>>, i32)> {
        let chars: Vec<char> = name.chars().collect();
        let frags = self.match_chars(&chars)?;
        let degree = self.degree_chars(&chars, &frags);
        Some((frags.into_iter().map(|(s, e)| s..e).collect(), degree))
    }

    /// The matching degree of a successful match (`i32::MIN` for none, `0` without fragments).
    pub fn degree(&self, name: &str, fragments: &[Range<usize>]) -> i32 {
        let chars: Vec<char> = name.chars().collect();
        let frags: Frags = fragments.iter().map(|r| (r.start, r.end)).collect();
        self.degree_chars(&chars, &frags)
    }

    /// Whether `name` matches (the empty pattern matches everything).
    pub fn matches(&self, name: &str) -> bool {
        self.match_name(name).is_some()
    }

    // ------------------------------------------------------------------ matching

    fn is_wildcard(&self, i: usize) -> bool {
        matches!(self.pattern.get(i), Some('*' | ' '))
    }

    fn is_star_before(&self, pi: usize) -> bool {
        pi > 0 && self.pattern.get(pi - 1) == Some(&'*')
    }

    fn ignore_case(&self) -> bool {
        self.mode != MatchMode::MatchCase
    }

    fn char_eq(&self, pi: usize, c: char) -> bool {
        let p = self.pattern[pi];
        p == c || (self.ignore_case() && (lower(p) == lower(c) || upper(p) == upper(c)))
    }

    fn match_chars(&self, name: &[char]) -> Option<Frags> {
        if self.pattern.is_empty() {
            return Some(Vec::new());
        }
        if name.len() < self.min_name_len {
            return None;
        }
        if self.min_name_len == 0 {
            return Some(Vec::new());
        }
        if !self.quick_accept(name) {
            return None;
        }
        let mut frags = if self.pattern.len() > MAX_HUMP_PATTERN {
            self.match_plain(name)?
        } else {
            self.match_wildcards(name, 0, Some(0))?
        };
        if self.trailing_space
            && let Some(last) = frags.last_mut()
            && last.1 != name.len()
        {
            if name.get(last.1) != Some(&' ') {
                return None;
            }
            last.1 += 1;
        }
        Some(merge(frags))
    }

    /// SRCH-MAT-003: every non-wildcard pattern character occurs in order.
    fn quick_accept(&self, name: &[char]) -> bool {
        let mut pos = 0;
        for &p in &self.pattern {
            if matches!(p, '*' | ' ') {
                continue;
            }
            let Some(found) = name[pos..].iter().position(|&c| {
                c == p || (self.ignore_case() && (lower(c) == lower(p) || upper(c) == upper(p)))
            }) else {
                return false;
            };
            pos += found + 1;
        }
        true
    }

    /// SRCH-MAT-004: plain matching without humps for very long patterns.
    fn match_plain(&self, name: &[char]) -> Option<Frags> {
        let wanted: Vec<usize> = (0..self.pattern.len())
            .filter(|i| !self.is_wildcard(*i))
            .collect();
        let anywhere = self.pattern.first() == Some(&'*');
        let try_at = |start: usize| -> Option<Frags> {
            let mut ni = start;
            for &pi in &wanted {
                while ni < name.len() && name[ni] == ' ' && self.pattern[pi] != ' ' {
                    ni += 1;
                }
                if ni >= name.len() || !self.char_eq(pi, name[ni]) {
                    return None;
                }
                ni += 1;
            }
            Some(vec![(start, ni)])
        };
        if anywhere {
            (0..name.len()).find_map(try_at)
        } else {
            try_at(0)
        }
    }

    fn match_wildcards(&self, name: &[char], pi: usize, ni: Option<usize>) -> Option<Frags> {
        let ni = ni?;
        if !self.is_wildcard(pi) {
            if pi >= self.pattern.len() {
                return Some(Vec::new());
            }
            return self.match_fragment(name, pi, ni);
        }
        let mut pos = pi;
        while self.is_wildcard(pos) {
            pos += 1;
        }
        if pos >= self.pattern.len() {
            return Some(Vec::new());
        }
        self.match_skipping_words(name, pos, ni)
    }

    fn match_fragment(&self, name: &[char], pi: usize, ni: usize) -> Option<Frags> {
        let len = self.max_matching_fragment(name, pi, ni);
        if len == 0 {
            None
        } else {
            self.match_inside_fragment(name, pi, ni, len)
        }
    }

    fn first_char_matching(&self, name: &[char], ni: usize, pi: usize) -> bool {
        let Some(&c) = name.get(ni) else {
            return false;
        };
        if !self.char_eq(pi, c) {
            return false;
        }
        if self.mode == MatchMode::FirstLetter && pi == self.first_real {
            let p = self.pattern[pi];
            let first = name[0];
            let has_case = is_upper(p) || is_lower(p);
            if has_case && is_upper(p) != is_upper(first) && (is_upper(first) || is_lower(first)) {
                return false;
            }
        }
        true
    }

    fn max_matching_fragment(&self, name: &[char], pi: usize, ni: usize) -> usize {
        if !self.first_char_matching(name, ni, pi) {
            return 0;
        }
        let mut i = 1;
        while ni + i < name.len() && pi + i < self.pattern.len() {
            if !self.char_eq(pi + i, name[ni + i]) {
                // SRCH-MAT-017: a pattern digit after a digit must not skip a different name digit.
                if is_digit(self.pattern[pi + i])
                    && is_digit(self.pattern[pi + i - 1])
                    && is_digit(name[ni + i])
                {
                    return 0;
                }
                break;
            }
            i += 1;
        }
        i
    }

    fn is_middle_match(&self, name: &[char], pi: usize, ni: usize) -> bool {
        self.is_star_before(pi) && !is_word_separator(name[ni]) && !is_word_start(name, ni)
    }

    fn seems_like_fragment_start(&self, name: &[char], pi: usize, ni: usize) -> bool {
        // An uppercase pattern letter lands on an uppercase letter or a word start, or anywhere
        // when the pattern is not mixed case and case does not matter (SRCH-MAT-013).
        !self.is_upper[pi]
            || is_upper(name[ni])
            || is_word_start(name, ni)
            || (!self.has_humps && self.mode != MatchMode::MatchCase)
    }

    fn match_skipping_words(&self, name: &[char], pi: usize, ni: usize) -> Option<Frags> {
        let mut max_found = 0;
        let mut idx = Some(ni);
        while let Some(i) = idx {
            if i >= name.len() {
                return None;
            }
            let len = if self.seems_like_fragment_start(name, pi, i) {
                self.max_matching_fragment(name, pi, i)
            } else {
                0
            };
            let middle = self.is_middle_match(name, pi, i);
            if len > 0 && (len > max_found || (i + len == name.len() && middle)) {
                if !middle {
                    max_found = len;
                }
                if let Some(r) = self.match_inside_fragment(name, pi, i, len) {
                    return Some(r);
                }
            }
            idx = self.next_candidate(name, pi, i, ni);
        }
        None
    }

    /// The next place a fragment starting at pattern index `pi` may begin after `current`.
    fn next_candidate(
        &self,
        name: &[char],
        pi: usize,
        current: usize,
        origin: usize,
    ) -> Option<usize> {
        let any_position = self.is_star_before(pi) || self.is_sep[pi];
        if current + 1 > name.len() {
            return None;
        }
        let mut next = if any_position {
            let mut j = current + 1;
            loop {
                if j >= name.len() {
                    return None;
                }
                if self.char_eq(pi, name[j]) {
                    break j;
                }
                j += 1;
            }
        } else {
            self.index_of_word_start(name, pi, current)?
        };
        // SRCH-MAT-016: a skip does not cross a name dot unless the pattern char before is a dot.
        if self.has_dots && pi > self.first_real && !matches!(self.pattern[pi - 1], '.' | '*') {
            let crossed = name[origin.min(next)..next].contains(&'.');
            if crossed {
                return None;
            }
        }
        // SRCH-MAT-015: hard separators are not skipped by patterns without separators.
        if !self.hard_separators.is_empty()
            && !self.has_humps
            && !self.pattern.iter().any(|c| is_word_separator(*c))
            && name[origin.min(next)..next]
                .iter()
                .any(|c| self.hard_separators.contains(c))
        {
            return None;
        }
        if next < current + 1 {
            next = current + 1;
        }
        Some(next)
    }

    /// The next word start after `start_from` where pattern character `pi` can begin a fragment.
    fn index_of_word_start(&self, name: &[char], pi: usize, start_from: usize) -> Option<usize> {
        if start_from >= name.len()
            || (self.has_humps && self.is_lower[pi] && !(pi > 0 && self.is_sep[pi - 1]))
        {
            return None;
        }
        let mut s = start_from;
        loop {
            s = next_hump(name, s);
            if s >= name.len() {
                return None;
            }
            if self.char_eq(pi, name[s]) {
                return Some(s);
            }
        }
    }

    fn match_inside_fragment(
        &self,
        name: &[char],
        pi: usize,
        ni: usize,
        len: usize,
    ) -> Option<Frags> {
        // A middle fragment is at least three characters unless it ends the pattern.
        let min_fragment = if self.is_middle_match(name, pi, ni) {
            3
        } else {
            1
        };
        self.improve_camel_humps(name, pi, ni, len, min_fragment)
            .or_else(|| self.longest_matching_prefix(name, pi, ni, len, min_fragment))
    }

    fn improve_camel_humps(
        &self,
        name: &[char],
        pi: usize,
        ni: usize,
        max: usize,
        min: usize,
    ) -> Option<Frags> {
        for i in min..max {
            if self.is_upper[pi + i] && self.pattern[pi + i] != name[ni + i] {
                let next = self.index_of_word_start(name, pi + i, ni + i);
                if let Some(mut rest) = self.match_wildcards(name, pi + i, next) {
                    rest.insert(0, (ni, ni + i));
                    return Some(rest);
                }
            }
        }
        None
    }

    fn longest_matching_prefix(
        &self,
        name: &[char],
        pi: usize,
        ni: usize,
        len: usize,
        min: usize,
    ) -> Option<Frags> {
        if pi + len >= self.pattern.len() {
            return Some(vec![(ni, ni + len)]);
        }
        let mut i = len;
        while i >= min || (i > 0 && self.is_wildcard(pi + i)) {
            let rest = if self.is_wildcard(pi + i) {
                self.match_wildcards(name, pi + i, Some(ni + i))
            } else {
                self.match_skipping_words(name, pi + i, ni + i)
            };
            if let Some(mut rest) = rest {
                rest.insert(0, (ni, ni + i));
                return Some(rest);
            }
            if i == 0 {
                break;
            }
            i -= 1;
        }
        None
    }

    // -------------------------------------------------------------------- degree

    fn degree_chars(&self, name: &[char], frags: &Frags) -> i32 {
        let Some(&(first_start, _)) = frags.first() else {
            return 0;
        };
        let start_match = first_start == 0;
        let mut matching_case = 0i32;
        let mut p: isize = -1;
        let mut skipped_humps = 0i32;
        let mut next_hump_start = 0usize;
        let mut hump_start_upper = false;
        for (fi, &(start, end)) in frags.iter().enumerate() {
            for i in start..end {
                let after_gap = i == start && fi > 0;
                let mut is_hump_start = false;
                while next_hump_start <= i {
                    if next_hump_start == i {
                        is_hump_start = true;
                    } else if after_gap {
                        skipped_humps += 1;
                    }
                    next_hump_start = next_hump(name, next_hump_start);
                }
                let c = name[i];
                let mut found = None;
                let mut k = (p + 1) as usize;
                while k < self.full.len() {
                    let pc = self.full[k];
                    if pc != '*' && (pc == c || lower(pc) == lower(c) || upper(pc) == upper(c)) {
                        found = Some(k);
                        break;
                    }
                    k += 1;
                }
                let Some(k) = found else {
                    break;
                };
                p = k as isize;
                let pc = self.full[k];
                if is_hump_start {
                    hump_start_upper = is_upper(pc);
                }
                matching_case +=
                    self.case_term(pc, c, i, after_gap, is_hump_start, hump_start_upper);
            }
        }
        let after_separator = name[..first_start.min(name.len())]
            .iter()
            .any(|c| self.hard_separators.contains(c));
        let whole_word_start = first_start == 0
            || (is_word_start(name, first_start) && !is_word_start(name, first_start - 1));
        let final_match = frags.last().is_some_and(|f| f.1 == name.len());
        (if whole_word_start { 1000 } else { 0 }) + matching_case
            - frags.len() as i32
            - skipped_humps * 10
            + if after_separator { 0 } else { 2 }
            + i32::from(start_match)
            + i32::from(final_match)
    }

    /// SRCH-MAT-033: the case term of one matched character.
    fn case_term(
        &self,
        pattern_char: char,
        name_char: char,
        _name_index: usize,
        after_gap: bool,
        is_hump_start: bool,
        hump_start_upper: bool,
    ) -> i32 {
        if after_gap && is_hump_start && is_lower(pattern_char) {
            -10
        } else if pattern_char == name_char {
            if is_upper(name_char) {
                50
            } else if is_hump_start {
                1
            } else {
                0
            }
        } else if is_hump_start {
            -1
        } else if hump_start_upper && is_lower(pattern_char) && is_upper(name_char) {
            -1
        } else {
            0
        }
    }
}

/// Merges adjacent fragments (the matcher reports each run once).
fn merge(frags: Frags) -> Frags {
    let mut out: Frags = Vec::with_capacity(frags.len());
    for (s, e) in frags {
        if s == e {
            continue;
        }
        match out.last_mut() {
            Some(last) if last.1 == s => last.1 = e,
            _ => out.push((s, e)),
        }
    }
    out
}

/// Renders `name` with the fragments in square brackets (`[Repl]ace[Map]`).
pub fn bracket(name: &str, fragments: &[Range<usize>]) -> String {
    let chars: Vec<char> = name.chars().collect();
    let mut out = String::new();
    let mut at = 0;
    for r in fragments {
        out.extend(&chars[at..r.start.min(chars.len())]);
        out.push('[');
        out.extend(&chars[r.start.min(chars.len())..r.end.min(chars.len())]);
        out.push(']');
        at = r.end.min(chars.len());
    }
    out.extend(&chars[at..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn degree(pattern: &str, name: &str) -> Option<i32> {
        NameMatcher::new(pattern, MatchMode::IgnoreCase)
            .match_with_degree(name)
            .map(|(_, d)| d)
    }

    #[test]
    fn camel_humps_and_wildcards_score_like_the_reference() {
        // SRCH-MAT-001, SRCH-MAT-010, SRCH-MAT-034
        assert_eq!(degree("repl map", "ReplaceMap"), Some(991));
        assert_eq!(degree("NU", "NameUtilTest"), Some(1101));
        assert_eq!(degree("nt", "Nettest"), None);
        assert_eq!(degree("*foo", "barFoo"), Some(1001));
    }

    #[test]
    fn odd_input_never_panics() {
        // SRCH-MAT-036
        for pattern in ["", "*", " ", "* ", "a", "é", "日本"] {
            let matcher = NameMatcher::new(pattern, MatchMode::IgnoreCase);
            for name in ["", "a", "日本語", "😀x", "é"] {
                let _ = matcher.match_with_degree(name);
            }
        }
    }

    #[test]
    fn brackets_mark_fragments() {
        let m = NameMatcher::new("hm", MatchMode::IgnoreCase);
        let (frags, _) = m.match_with_degree("HashMap").unwrap();
        assert_eq!(bracket("HashMap", &frags), "[H]ash[M]ap");
    }
}
