//! Runner of `07-matching-and-ranking.cases.md`: the name matcher (SRCH-MAT-001 to 036), the
//! Go to Class and Go to File rankers (SRCH-MAT-034 to 047) and the position suffix
//! (SRCH-MAT-042).

use super::{Case, Outcome, compare_lines};
use archipel_ide_core::goto_rank::{
    GotoOptions, rank_files, rank_names, rank_qualified, split_position,
};
use archipel_ide_core::name_match::{MatchMode, NameMatcher, bracket};

/// The quoted argument of the first `<verb> "<text>"` in `keys`.
fn quoted(keys: &str) -> Option<String> {
    let start = keys.find('"')?;
    let end = keys.rfind('"')?;
    (end > start).then(|| keys[start + 1..end].to_owned())
}

fn mode_of(case: &Case) -> MatchMode {
    match case.setting("mode").as_deref() {
        Some("MATCH_CASE") => MatchMode::MatchCase,
        Some("FIRST_LETTER") => MatchMode::FirstLetter,
        _ => MatchMode::IgnoreCase,
    }
}

fn lines(text: &str) -> Vec<String> {
    text.lines().map(str::to_owned).collect()
}

pub fn run(case: &Case) -> Outcome {
    let keys = case.keys.as_str();
    let expected = lines(case.after());
    let before = lines(case.before());
    let actual: Vec<String> = if keys.starts_with("match \"") {
        let pattern = quoted(keys).unwrap_or_default();
        let matcher = NameMatcher::new(&pattern, mode_of(case));
        before
            .iter()
            .map(|name| match matcher.match_with_degree(name) {
                Some((frags, degree)) => format!("{}  =>  {degree}", bracket(name, &frags)),
                None => format!("{name}  =>  no match"),
            })
            .collect()
    } else if keys.starts_with("typing each text") {
        before
            .iter()
            .map(|typed| {
                let p = split_position(typed);
                let show = |v: Option<usize>| v.map_or("-".to_owned(), |v| v.to_string());
                format!(
                    "'{typed}' -> name='{}', line={}, column={}",
                    p.name,
                    show(p.line),
                    show(p.column)
                )
            })
            .collect()
    } else if case.language == "java" {
        return Outcome::NotRunnable("needs a Java symbol model (Outer#member, anonymous classes)");
    } else if case.settings.starts_with("Go to File") {
        let typed = quoted(keys).unwrap_or_default();
        let paths: Vec<&str> = before.iter().map(String::as_str).collect();
        rank_files(&typed, &paths, None, GotoOptions::default())
            .iter()
            .map(|h| format!("{}  {}", paths[h.index], h.weight))
            .collect()
    } else if case.settings.starts_with("Go to Class, separators") {
        let typed = quoted(keys).unwrap_or_default();
        let names: Vec<&str> = before.iter().map(String::as_str).collect();
        let names_only = expected.iter().all(|l| {
            super::normalize_line(l)
                .rsplit(' ')
                .next()
                .is_some_and(|t| t.parse::<i32>().is_err())
        });
        rank_qualified(&typed, &names, &['.'], GotoOptions::default())
            .iter()
            .map(|h| {
                if names_only {
                    names[h.index].to_owned()
                } else {
                    format!("{}  {}", names[h.index], h.weight)
                }
            })
            .collect()
    } else if keys.starts_with("Go to Class, type") {
        let typed = quoted(keys).unwrap_or_default();
        let names: Vec<&str> = before.iter().map(String::as_str).collect();
        let opts = GotoOptions {
            mode: mode_of(case),
            middle_matching: case.setting("middle-matching").as_deref() != Some("off"),
            prefer_start: case.setting("prefer-start").as_deref() != Some("off"),
        };
        rank_names(&typed, &names, opts)
            .iter()
            .map(|h| {
                format!(
                    "{}  {}  {}",
                    names[h.index],
                    bracket(names[h.index], &h.fragments),
                    h.weight
                )
            })
            .collect()
    } else {
        return Outcome::NotRunnable("unrecognized case shape");
    };
    match compare_lines(&expected, &actual) {
        Ok(()) => Outcome::Pass,
        Err(reason) => Outcome::Fail(reason),
    }
}
