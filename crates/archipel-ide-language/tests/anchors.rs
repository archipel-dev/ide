//! Anchors survive random edits: a resolved anchor always has the anchor's
//! kind (and name), and nodes before an edit are found again exactly.

use archipel_ide_language::anchor::name_hash;
use archipel_ide_language::{
    Anchor, EditMap, FileId, FileRev, SyntaxSnapshot, SyntaxTree, registry,
};
use archipel_ide_text::{EditJournal, TextDocument};
use proptest::prelude::*;

const SAMPLES: &[(&str, &str)] = &[
    (
        "java",
        "package a;\n\nclass App {\n    int count;\n    void run(int x) { count = x + 1; }\n    String name() { return \"n\"; }\n}\n",
    ),
    (
        "rust",
        "fn alpha(x: u32) -> u32 {\n    let y = x * 2;\n    y + 1\n}\n\nstruct Beta { a: i32 }\n\nimpl Beta { fn get(&self) -> i32 { self.a } }\n",
    ),
    (
        "python",
        "def alpha(x):\n    y = x * 2\n    return y\n\nclass Beta:\n    def get(self):\n        return 1\n",
    ),
    (
        "kotlin",
        "package a\n\nclass Main(val x: Int) {\n    fun run(): Int = x + 1\n}\n",
    ),
];

const FRAGMENTS: &[&str] = &[
    "", "x", " ", "\n", "{", "}", "(", ")", ";", "\"", "fn ", "class ", "def ", "int ", "1", "//",
    "ä", "    ",
];

fn snapshot(lang: &str, text: &str, rev: u64) -> SyntaxSnapshot {
    let lang = registry::by_id(lang).expect("language");
    SyntaxSnapshot::parse(FileId(1), FileRev(rev), lang, text.into()).expect("parse")
}

fn anchors(snap: &SyntaxSnapshot) -> Vec<(Anchor, String)> {
    snap.root()
        .preorder()
        .filter(|node| node.is_named() && !node.is_error())
        .take(300)
        .map(|node| (Anchor::new(node), node.text().to_string()))
        .collect()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(96))]

    #[test]
    fn anchors_never_resolve_to_a_different_kind_after_random_edits(
        sample in 0..SAMPLES.len(),
        edits in prop::collection::vec((0.0f64..1.0, 0usize..5, 0..FRAGMENTS.len()), 1..12),
        incremental in any::<bool>(),
    ) {
        let (lang, text) = SAMPLES[sample];
        let spec = registry::by_id(lang).expect("language");
        let original = snapshot(lang, text, 1);
        let anchors = anchors(&original);
        let mut document = TextDocument::new(text);
        document.take_edits();
        let mut tree = SyntaxTree::parse(spec, text).expect("parse");
        let mut map = EditMap::new();
        for (at, delete, insert) in edits {
            let len = document.len_chars();
            let start = ((len as f64) * at) as usize;
            let end = (start + delete).min(len);
            document.edit(start, end, FRAGMENTS[insert]).unwrap();
            let source = document.text();
            let EditJournal::Edits(journal) = document.take_edits() else {
                panic!("journal");
            };
            // The map is built incrementally (push) or in one go (from_edits).
            if incremental {
                for edit in &journal {
                    map.push(edit);
                }
            } else {
                map = map.compose(&EditMap::from_edits(&journal));
            }
            tree.apply_edits(&source, &journal).expect("reparse");
        }
        let source = document.text();
        let current = SyntaxSnapshot::from_tree(FileId(1), FileRev(2), &tree, source.into());
        for (anchor, _) in &anchors {
            let Some(found) = anchor.resolve(&current, &map) else {
                continue;
            };
            prop_assert_eq!(found.element_type(), anchor.kind, "{:?} resolved to {:?}", anchor, found);
            if anchor.is_named() {
                let name = found.name_node().map(|name| name_hash(name.text()));
                prop_assert_eq!(name, Some(anchor.name_hash));
            }
            prop_assert!(found.byte_range().end <= current.text().len());
        }
    }
}

#[test]
fn anchors_before_an_edit_resolve_exactly() {
    let (lang, text) = SAMPLES[0];
    let original = snapshot(lang, text, 1);
    let anchors = anchors(&original);
    // Append a member at the end of the class body: everything before the
    // insertion point keeps its range.
    let at = text.rfind('}').expect("brace");
    let edited = format!("{}    int added;\n{}", &text[..at], &text[at..]);
    let current = snapshot(lang, &edited, 2);
    let edit = archipel_ide_language::ByteEdit {
        start_byte: at,
        old_end_byte: at,
        new_end_byte: at + "    int added;\n".len(),
        start_point: Default::default(),
        old_end_point: Default::default(),
        new_end_point: Default::default(),
    };
    let map = EditMap::from_edits(&[edit]);
    let mut resolved = 0;
    for (anchor, old_text) in anchors.iter().filter(|(a, _)| (a.range.end as usize) < at) {
        let found = anchor.resolve(&current, &map).expect("still there");
        assert_eq!(found.text(), old_text);
        resolved += 1;
    }
    assert!(resolved > 10);
}
