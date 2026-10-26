//! Property tests of the structural features over the golden sources, also
//! after random damage (deleted spans, stray brackets and quotes): extend
//! selection grows strictly up to the whole file, fold regions never cross,
//! bracket pairs are symmetric.

use archipel_ide_language::brackets;
use archipel_ide_language::{FileId, FileRev, LANGUAGES, SyntaxSnapshot};
use proptest::prelude::*;

const CARET: char = '\u{2038}';

fn samples() -> Vec<(&'static str, String)> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden");
    LANGUAGES
        .iter()
        .filter_map(|spec| {
            let text = std::fs::read_to_string(root.join(spec.id).join("folds.src")).ok()?;
            Some((spec.id, text.replace(CARET, "")))
        })
        .collect()
}

fn snapshot(lang: &str, text: &str) -> Option<SyntaxSnapshot> {
    let spec = archipel_ide_language::registry::by_id(lang)?;
    SyntaxSnapshot::parse(FileId(1), FileRev(1), spec, text.into())
}

fn boundary(text: &str, mut at: usize) -> usize {
    at = at.min(text.len());
    while !text.is_char_boundary(at) {
        at -= 1;
    }
    at
}

/// A sample with a deleted span and one inserted stray character.
fn damaged(text: &str, cut: (usize, usize), stray: (usize, usize)) -> String {
    const STRAYS: &[&str] = &[
        "{", "}", "(", ")", "[", "]", "\"", "'", "<", ">", "\n", "/*", "`",
    ];
    let start = boundary(text, cut.0 % (text.len() + 1));
    let end = boundary(text, (start + cut.1 % 40).min(text.len()));
    let mut out = format!("{}{}", &text[..start], &text[end..]);
    let at = boundary(&out, stray.0 % (out.len() + 1));
    out.insert_str(at, STRAYS[stray.1 % STRAYS.len()]);
    out
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(160))]

    #[test]
    fn extend_selection_grows_strictly_up_to_the_whole_file(
        lang in 0usize..32,
        cut in (0usize..4000, 0usize..40),
        stray in (0usize..4000, 0usize..13),
        start in 0usize..4000,
        width in 0usize..6,
        damage in proptest::bool::ANY,
    ) {
        let all = samples();
        let (id, text) = &all[lang % all.len()];
        let text = if damage { damaged(text, cut, stray) } else { text.clone() };
        let Some(snap) = snapshot(id, &text) else { return Ok(()) };
        let from = boundary(&text, start % (text.len() + 1));
        let to = boundary(&text, from + width);
        let mut range = from..to;
        let mut steps = 0;
        while let Some(next) = snap.expand_selection(range.clone()) {
            prop_assert!(next.start <= range.start && range.end <= next.end, "{id}: {range:?} -> {next:?}");
            prop_assert!(next.len() > range.len(), "{id}: not strictly larger {range:?} -> {next:?}");
            prop_assert!(next.end <= text.len());
            range = next;
            steps += 1;
            prop_assert!(steps < 500, "{id}: selection does not terminate");
        }
        let trimmed_start = text.len() - text.trim_start().len();
        let trimmed_end = text.trim_end().len();
        prop_assert!(
            range.start <= trimmed_start && range.end >= trimmed_end,
            "{id}: ended at {range:?} of {} bytes",
            text.len()
        );
    }

    #[test]
    fn folds_do_not_cross_and_start_once_per_line(
        lang in 0usize..32,
        cut in (0usize..4000, 0usize..40),
        stray in (0usize..4000, 0usize..13),
        damage in proptest::bool::ANY,
    ) {
        let all = samples();
        let (id, text) = &all[lang % all.len()];
        let text = if damage { damaged(text, cut, stray) } else { text.clone() };
        let Some(snap) = snapshot(id, &text) else { return Ok(()) };
        let folds = snap.folds_in(0..text.len());
        let mut start_lines = std::collections::HashSet::new();
        for fold in &folds {
            prop_assert!(fold.start_byte <= fold.end_byte && fold.end_byte <= text.len());
            prop_assert!(fold.start_line < fold.end_line);
            prop_assert!(start_lines.insert(fold.start_line), "{id}: two folds start on line {}", fold.start_line);
        }
        for (i, a) in folds.iter().enumerate() {
            for b in &folds[i + 1..] {
                let disjoint = a.end_byte <= b.start_byte || b.end_byte <= a.start_byte;
                let nested = (a.start_byte <= b.start_byte && b.end_byte <= a.end_byte)
                    || (b.start_byte <= a.start_byte && a.end_byte <= b.end_byte);
                prop_assert!(disjoint || nested, "{id}: folds cross: {a:?} {b:?}");
            }
        }
    }

    #[test]
    fn bracket_pairs_are_symmetric(
        lang in 0usize..32,
        cut in (0usize..4000, 0usize..40),
        stray in (0usize..4000, 0usize..13),
        damage in proptest::bool::ANY,
    ) {
        let all = samples();
        let (id, text) = &all[lang % all.len()];
        let text = if damage { damaged(text, cut, stray) } else { text.clone() };
        let Some(snap) = snapshot(id, &text) else { return Ok(()) };
        for pair in brackets::all_pairs(&snap) {
            prop_assert!(pair.open < pair.close, "{id}: {pair:?}");
            let from_open = brackets::pair_at(&snap, pair.open);
            let from_close = brackets::pair_at(&snap, pair.close);
            prop_assert_eq!(from_open, Some(pair), "{} open side", id);
            prop_assert_eq!(from_close, Some(pair), "{} close side", id);
        }
    }
}
