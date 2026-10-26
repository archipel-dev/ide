//! Markup shifting through random edits: highlights never exceed the text and
//! never split a character.

use archipel_ide_analysis::testing::byte_edit;
use archipel_ide_analysis::{FileRev, HighlightInfo, HighlightKind, Severity, ToolId};
use archipel_ide_daemon::FileMarkup;
use archipel_ide_language::EditMap;
use proptest::prelude::*;

fn text_strategy() -> impl Strategy<Value = String> {
    proptest::collection::vec(
        prop_oneof![
            Just('a'),
            Just('b'),
            Just('ä'),
            Just('日'),
            Just('\n'),
            Just('𝔸'),
            Just(' ')
        ],
        0..60,
    )
    .prop_map(|chars| chars.into_iter().collect())
}

fn boundaries(text: &str) -> Vec<usize> {
    text.char_indices()
        .map(|(i, _)| i)
        .chain([text.len()])
        .collect()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn shifted_highlights_stay_in_bounds_on_char_boundaries(
        text in text_strategy(),
        picks in proptest::collection::vec((any::<prop::sample::Index>(), any::<prop::sample::Index>()), 0..12),
        edits in proptest::collection::vec(
            (any::<prop::sample::Index>(), any::<prop::sample::Index>(), "[a-zä日𝔸\n ]{0,6}"),
            0..8,
        ),
    ) {
        let tool = ToolId::native("t");
        let mut markup = FileMarkup::new(FileRev(1));
        let bounds = boundaries(&text);
        let infos: Vec<_> = picks.iter().map(|(a, b)| {
            let (x, y) = (bounds[a.index(bounds.len())], bounds[b.index(bounds.len())]);
            HighlightInfo::new(x.min(y) as u32..x.max(y) as u32, Severity::WARNING, HighlightKind::Warning, tool.clone()).message("m")
        }).collect();
        markup.replace(&tool, 0..u32::MAX, infos);

        let mut current = text;
        let mut map = EditMap::new();
        for (a, b, insert) in edits {
            let bounds = boundaries(&current);
            let (x, y) = (bounds[a.index(bounds.len())], bounds[b.index(bounds.len())]);
            let range = x.min(y)..x.max(y);
            let edit = byte_edit(&current, range.clone(), &insert);
            current.replace_range(range, &insert);
            map.push(&edit);
        }
        markup.shift(&map, FileRev(2), &current);
        for info in markup.iter() {
            let (s, e) = (info.range.start as usize, info.range.end as usize);
            prop_assert!(s <= e, "start after end");
            prop_assert!(e <= current.len(), "range {s}..{e} exceeds {}", current.len());
            prop_assert!(current.is_char_boundary(s) && current.is_char_boundary(e), "splits a char");
        }
        // Queries over the whole text never panic and agree with iteration.
        let all = markup.query(0..current.len() as u32 + 1);
        prop_assert!(all.len() <= markup.len());
    }

    #[test]
    fn untouched_highlights_move_by_the_edit_delta(
        prefix in "[a-z]{1,10}", body in "[a-z]{1,10}", suffix in "[a-z]{0,10}", insert in "[a-z日]{0,8}",
    ) {
        let text = format!("{prefix}{body}{suffix}");
        let tool = ToolId::native("t");
        let mut markup = FileMarkup::new(FileRev(1));
        let range = prefix.len() as u32..(prefix.len() + body.len()) as u32;
        markup.replace(&tool, 0..u32::MAX, vec![HighlightInfo::new(range.clone(), Severity::ERROR, HighlightKind::Error, tool.clone())]);
        // Insert in front of the highlight.
        let edit = byte_edit(&text, 0..0, &insert); // strictly before the highlight
        let mut new_text = text.clone();
        new_text.insert_str(0, &insert);
        markup.shift(&EditMap::from_edits(&[edit]), FileRev(2), &new_text);
        let moved = markup.iter().next().unwrap().range.clone();
        prop_assert_eq!(moved, range.start + insert.len() as u32..range.end + insert.len() as u32);
    }
}
