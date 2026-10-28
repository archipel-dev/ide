//! How the Go to lists use the name matcher (SRCH-MAT section 5): pattern decoration, the
//! split of the typed text into name and position, qualified class patterns, qualified file
//! patterns and the ordering rules of Go to Class and Go to File.
//!
//! Everything here is a pure function of the typed text and the candidate names so that Search
//! Everywhere, the Go to popups and the tests share one ranking path.

use crate::name_match::{EXACT_QUALIFIED_BONUS, MatchMode, NameMatcher, PREFER_START_BONUS};
use regex::Regex;
use std::cmp::Ordering;
use std::ops::Range;
use std::sync::OnceLock;

/// Options of a Go to list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GotoOptions {
    pub mode: MatchMode,
    /// Registry `ide.goto.middle.matching` (default on): the typed text gets a leading `*`.
    pub middle_matching: bool,
    /// Whether names matched from their first character get [`PREFER_START_BONUS`].
    pub prefer_start: bool,
}

impl Default for GotoOptions {
    fn default() -> Self {
        Self {
            mode: MatchMode::IgnoreCase,
            middle_matching: true,
            prefer_start: true,
        }
    }
}

/// The typed text reduced to its name part and the position it asks for (SRCH-MAT-042).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamePosition {
    pub name: String,
    pub line: Option<usize>,
    pub column: Option<usize>,
}

fn position_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"^(.+?)(?:#L|\?l=| on line | at line |:line |:\(|:\[|[:@, #(\[])(\d+)?(?:\W(\d+))?[)\]]?$",
        )
        .expect("static pattern")
    })
}

/// Splits `Foo:12:5`, `Foo(12)`, `Foo#L12`, `Foo on line 12` into the name `Foo` and a position.
/// A trailing space of the typed text stays on the name (SRCH-MAT-042).
pub fn split_position(typed: &str) -> NamePosition {
    let trailing_space = typed.ends_with(' ');
    let body = typed.trim_end_matches(' ');
    let plain = |name: &str| NamePosition {
        name: if trailing_space {
            format!("{name} ")
        } else {
            name.to_owned()
        },
        line: None,
        column: None,
    };
    let Some(caps) = position_regex().captures(body) else {
        return plain(body);
    };
    let name = caps.get(1).map_or("", |m| m.as_str());
    let number = |index: usize| {
        caps.get(index)
            .and_then(|m| m.as_str().parse::<usize>().ok())
    };
    let (line, column) = (number(2), number(3));
    if line.is_none() && column.is_none() {
        return plain(name);
    }
    let line = line.unwrap_or(1);
    let column = column.unwrap_or(1);
    if line == 0 && column <= 1 {
        return plain(name);
    }
    NamePosition {
        name: if trailing_space {
            format!("{name} ")
        } else {
            name.to_owned()
        },
        line: Some(line.max(1)),
        column: Some(column.max(1)),
    }
}

/// The pattern the Go to lists match names with (SRCH-MAT-040): `*` + the typed text when it has
/// more than one character, a prefix match for a single character.
pub fn decorate(typed: &str, middle_matching: bool) -> String {
    let trimmed = typed.trim();
    if middle_matching && trimmed.chars().count() > 1 && !typed.starts_with('*') {
        format!("*{typed}")
    } else {
        typed.to_owned()
    }
}

/// One entry of a ranked Go to list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NameHit {
    /// Index into the candidate list.
    pub index: usize,
    /// Fragments of the (short) name that matched.
    pub fragments: Vec<Range<usize>>,
    /// Matching degree plus the prefer-start and exact-match bonuses.
    pub weight: i32,
}

fn name_weight(
    matcher: &NameMatcher,
    name: &str,
    typed: &str,
    opts: GotoOptions,
) -> Option<(Vec<Range<usize>>, i32)> {
    let (fragments, degree) = matcher.match_with_degree(name)?;
    let bonus = if opts.prefer_start
        && !typed.starts_with('*')
        && fragments.first().is_some_and(|f| f.start == 0)
    {
        PREFER_START_BONUS
    } else {
        0
    };
    Some((fragments, degree + bonus))
}

fn cmp_ignore_case(a: &str, b: &str) -> Ordering {
    a.to_lowercase().cmp(&b.to_lowercase())
}

/// Matches `typed` against `names` and returns the hits in display order (SRCH-MAT-041):
/// a name equal to the typed text ignoring case first, then higher weight, then the name
/// ignoring case; equal names keep their candidate order.
pub fn rank_names(typed: &str, names: &[&str], opts: GotoOptions) -> Vec<NameHit> {
    let typed = split_position(typed).name;
    let typed = typed.as_str();
    let pattern = decorate(typed, opts.middle_matching);
    let matcher = NameMatcher::new(&pattern, opts.mode);
    let wanted = typed.trim();
    let mut hits: Vec<(NameHit, bool)> = Vec::new();
    for (index, name) in names.iter().enumerate() {
        if let Some((fragments, weight)) = name_weight(&matcher, name, typed, opts) {
            hits.push((
                NameHit {
                    index,
                    fragments,
                    weight,
                },
                !wanted.is_empty() && name.to_lowercase() == wanted.to_lowercase(),
            ));
        }
    }
    hits.sort_by(|(a, a_exact), (b, b_exact)| {
        b_exact
            .cmp(a_exact)
            .then(b.weight.cmp(&a.weight))
            .then_with(|| cmp_ignore_case(names[a.index], names[b.index]))
    });
    hits.into_iter().map(|(hit, _)| hit).collect()
}

// ------------------------------------------------------------ qualified names

/// The short name of a qualified name (the text after the last separator).
fn short_name<'a>(qualified: &'a str, separators: &[char]) -> &'a str {
    qualified
        .rsplit(|c| separators.contains(&c))
        .next()
        .unwrap_or(qualified)
}

/// `text` as the qualified filter pattern: a leading `*` and every separator wrapped in `*`.
fn qualified_pattern(text: &str, separators: &[char]) -> String {
    let mut out = String::from("*");
    for c in text.chars() {
        if separators.contains(&c) {
            out.push('*');
            out.push(c);
            out.push('*');
        } else {
            out.push(c);
        }
    }
    out
}

/// Whether `qualified` ends with `typed`, the match starting at a separator or at the start
/// (SRCH-MAT-035).
fn is_exact_qualified(qualified: &str, typed: &str, separators: &[char]) -> bool {
    let (q, t) = (qualified.to_lowercase(), typed.to_lowercase());
    let Some(head) = q.strip_suffix(&t) else {
        return false;
    };
    head.chars()
        .last()
        .is_none_or(|c| separators.contains(&c) || !(c.is_alphanumeric() || c == '_'))
}

/// Ranks qualified class or symbol names (SRCH-MAT-044, SRCH-MAT-035, SRCH-MAT-047). The typed
/// text may contain separators (`util.StringUtil`) and inner `*` (`acme*Util`); the text after
/// the last separator names the short name and the whole text filters the qualified name.
pub fn rank_qualified(
    typed: &str,
    qualified: &[&str],
    separators: &[char],
    opts: GotoOptions,
) -> Vec<NameHit> {
    let typed = split_position(typed).name;
    let typed = typed.trim_end_matches(' ');
    let has_separator = typed.chars().any(|c| separators.contains(&c));
    let inner_star = typed.trim_start_matches('*').contains('*');
    let name_part_raw = typed
        .trim_end_matches(|c| separators.contains(&c))
        .rsplit(|c| separators.contains(&c))
        .next()
        .unwrap_or(typed);
    let qualified_matcher = (has_separator || inner_star)
        .then(|| NameMatcher::new(&qualified_pattern(typed, separators), opts.mode));
    // SRCH-MAT-047: with an inner `*` the short name is also tried against each suffix.
    let mut name_patterns: Vec<String> = vec![decorate(name_part_raw, opts.middle_matching)];
    if inner_star {
        let mut rest = name_part_raw;
        while let Some(at) = rest.trim_start_matches('*').find('*') {
            let skipped = rest.len() - rest.trim_start_matches('*').len();
            rest = &rest[skipped + at + 1..];
            if rest.is_empty() {
                break;
            }
            name_patterns.push(decorate(rest, opts.middle_matching));
        }
    }
    let matchers: Vec<NameMatcher> = name_patterns
        .iter()
        .map(|p| NameMatcher::new(p, opts.mode))
        .collect();
    let exact_bonus_allowed = has_separator && !typed.contains('*');
    let wanted = name_part_raw.trim();
    struct Row {
        hit: NameHit,
        equal_typed: bool,
        exact: bool,
    }
    let mut rows: Vec<Row> = Vec::new();
    for (index, full) in qualified.iter().enumerate() {
        let short = short_name(full, separators);
        let Some((fragments, mut weight)) = matchers
            .iter()
            .find_map(|m| name_weight(m, short, name_part_raw, opts))
        else {
            continue;
        };
        if let Some(q) = &qualified_matcher
            && !q.matches(full)
        {
            continue;
        }
        let exact = exact_bonus_allowed && is_exact_qualified(full, typed, separators);
        if exact {
            weight += EXACT_QUALIFIED_BONUS;
        }
        rows.push(Row {
            hit: NameHit {
                index,
                fragments,
                weight,
            },
            equal_typed: !wanted.is_empty() && short.to_lowercase() == wanted.to_lowercase(),
            exact,
        });
    }
    rows.sort_by(|a, b| {
        let (an, bn) = (
            short_name(qualified[a.hit.index], separators),
            short_name(qualified[b.hit.index], separators),
        );
        b.equal_typed
            .cmp(&a.equal_typed)
            .then(b.hit.weight.cmp(&a.hit.weight))
            .then_with(|| cmp_ignore_case(an, bn))
            .then(b.exact.cmp(&a.exact))
            .then_with(|| qualified[a.hit.index].cmp(qualified[b.hit.index]))
    });
    rows.into_iter().map(|r| r.hit).collect()
}

// ----------------------------------------------------------------- file paths

fn file_name(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

fn stem_lower(name: &str) -> String {
    let stem = match name.rfind('.') {
        Some(at) if at > 0 => &name[..at],
        _ => name,
    };
    stem.to_lowercase()
}

/// Ranks project-relative file paths for Go to File (SRCH-MAT-045, SRCH-MAT-046). `current` is
/// the path of the file being edited, for the closeness tie-break.
pub fn rank_files(
    typed: &str,
    paths: &[&str],
    current: Option<&str>,
    opts: GotoOptions,
) -> Vec<NameHit> {
    let typed = split_position(typed).name.replace('\\', "/");
    let typed = typed.trim_matches('/');
    let (dir_part, name_part) = match typed.rfind('/') {
        Some(at) => (&typed[..=at], &typed[at + 1..]),
        None => ("", typed),
    };
    let name_pattern = decorate(name_part, opts.middle_matching);
    let name_matcher = NameMatcher::new(&name_pattern, opts.mode);
    let dir_matcher = (!dir_part.is_empty()).then(|| {
        let mut p = String::from("*");
        for c in dir_part.chars() {
            if c == '/' {
                p.push_str("*/*");
            } else {
                p.push(c);
            }
        }
        NameMatcher::new(&p, opts.mode)
    });
    struct Cand {
        index: usize,
        fragments: Vec<Range<usize>>,
        name_weight: i32,
        weight: i32,
        in_top_tier: bool,
    }
    let mut cands: Vec<Cand> = Vec::new();
    for (index, path) in paths.iter().enumerate() {
        let name = file_name(path);
        let Some((fragments, name_w)) = name_weight(&name_matcher, name, name_part, opts) else {
            continue;
        };
        let mut weight = name_w;
        if let Some(dir) = &dir_matcher {
            let parent = &path[..path.len() - name.len()];
            let Some((dir_frags, degree)) = dir.match_with_degree(parent) else {
                continue;
            };
            let parent_len = parent.chars().count();
            let last_end = dir_frags.last().map_or(0, |f| f.end);
            weight = degree - parent_len.saturating_sub(last_end) as i32;
            if path.to_lowercase().ends_with(&typed.to_lowercase()) {
                weight += EXACT_QUALIFIED_BONUS;
            }
        } else if typed.contains('/') {
            continue;
        }
        cands.push(Cand {
            index,
            fragments,
            name_weight: name_w,
            weight,
            in_top_tier: !name_part.is_empty() && name.starts_with(name_part),
        });
    }
    let depth = |p: &str| p.matches('/').count();
    let closeness = |p: &str| -> usize {
        let Some(cur) = current else { return 0 };
        let a: Vec<&str> = p.split('/').collect();
        let b: Vec<&str> = cur.split('/').collect();
        a.iter().zip(&b).take_while(|(x, y)| x == y).count()
    };
    cands.sort_by(|a, b| {
        let (pa, pb) = (paths[a.index], paths[b.index]);
        b.in_top_tier
            .cmp(&a.in_top_tier)
            .then_with(|| {
                if a.in_top_tier && b.in_top_tier {
                    Ordering::Equal
                } else {
                    b.name_weight.cmp(&a.name_weight)
                }
            })
            .then_with(|| stem_lower(file_name(pa)).cmp(&stem_lower(file_name(pb))))
            .then_with(|| closeness(pb).cmp(&closeness(pa)))
            .then_with(|| depth(pa).cmp(&depth(pb)))
            .then_with(|| pa.cmp(pb))
    });
    cands.sort_by_key(|c| std::cmp::Reverse(c.weight));
    cands
        .into_iter()
        .map(|c| NameHit {
            index: c.index,
            fragments: c.fragments,
            weight: c.weight,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn position_suffixes_reduce_to_the_name() {
        // SRCH-MAT-042
        let p = split_position("Foo:12:5");
        assert_eq!(
            (p.name.as_str(), p.line, p.column),
            ("Foo", Some(12), Some(5))
        );
        let p = split_position("Foo#L12");
        assert_eq!((p.name.as_str(), p.line), ("Foo", Some(12)));
        assert_eq!(split_position("Foo:abc").name, "Foo:abc");
    }

    #[test]
    fn single_characters_match_as_prefix_only() {
        // SRCH-MAT-040
        let hits = rank_names("s", &["Set", "HashSet"], GotoOptions::default());
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].index, 0);
    }
}
