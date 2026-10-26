//! Golden tests of the generic structural features: for every language and
//! feature, `golden/<lang>/<feature>.src` is rendered and compared with
//! `golden/<lang>/<feature>.expected`. Run with `UPDATE_SNAPSHOTS=1` to
//! (re)write the expected files after an intended change, then review the diff.

use archipel_ide_language::brackets::{self, PairKind};
use archipel_ide_language::indent::{IndentEngine, IndentOptions};
use archipel_ide_language::lines::LineIndex;
use archipel_ide_language::{
    FileId, FileRev, LANGUAGES, StructureItem, SyntaxSnapshot, locals, sticky, textobjects,
};
use std::fmt::Write as _;
use std::path::PathBuf;

pub const CARET: char = '\u{2038}';
pub const FEATURES: &[&str] = &[
    "folds",
    "structure",
    "brackets",
    "selection",
    "indents",
    "sticky",
    "textobjects",
    "locals",
];

fn dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden")
}

fn snap(lang: &str, text: &str) -> SyntaxSnapshot {
    let spec = archipel_ide_language::registry::by_id(lang).expect("language");
    SyntaxSnapshot::parse(FileId(1), FileRev(1), spec, text.into()).expect("parse")
}

fn pos(lines: &LineIndex, offset: usize) -> String {
    let line = lines.line_of(offset);
    format!("{}:{}", line + 1, offset - lines.line_start(line) + 1)
}

fn excerpt(text: &str) -> String {
    let flat: String = text
        .chars()
        .map(|c| if c == '\n' { '\u{23ce}' } else { c })
        .collect();
    let mut out: String = flat.chars().take(48).collect();
    if flat.chars().count() > 48 {
        out.push('\u{2026}');
    }
    out
}

fn render_folds(snap: &SyntaxSnapshot) -> String {
    let mut out = String::new();
    for fold in snap.folds_in(0..snap.text().len()) {
        let _ = writeln!(
            out,
            "{}-{} {:?} {:?}{}{}",
            fold.start_line + 1,
            fold.end_line + 1,
            fold.kind,
            fold.placeholder,
            if fold.collapsed_by_default {
                " collapsed"
            } else {
                ""
            },
            fold.group.map(|_| " group").unwrap_or_default()
        );
    }
    out
}

fn outline(items: &[StructureItem], depth: usize, out: &mut String) {
    for item in items {
        let m = item.modifiers;
        let mut mods = Vec::new();
        if m.visibility != Default::default() {
            mods.push(format!("{:?}", m.visibility).to_lowercase());
        }
        for (flag, name) in [
            (m.is_static, "static"),
            (m.is_abstract, "abstract"),
            (m.is_final, "final"),
            (m.is_async, "async"),
            (m.is_override, "override"),
            (m.is_test, "test"),
            (m.is_deprecated, "deprecated"),
        ] {
            if flag {
                mods.push(name.to_owned());
            }
        }
        let _ = writeln!(
            out,
            "{}{:?} {}{}{}",
            "  ".repeat(depth),
            item.kind,
            item.name,
            item.detail.as_deref().unwrap_or(""),
            if mods.is_empty() {
                String::new()
            } else {
                format!(" [{}]", mods.join(" "))
            }
        );
        outline(&item.children, depth + 1, out);
    }
}

fn render_structure(snap: &SyntaxSnapshot) -> String {
    let mut out = String::new();
    outline(&snap.structure(), 0, &mut out);
    out
}

fn render_brackets(snap: &SyntaxSnapshot) -> String {
    let text = snap.text();
    let lines = LineIndex::new(text);
    let mut out = String::new();
    for pair in brackets::all_pairs(snap) {
        let kind = match pair.kind {
            PairKind::Paren => "paren",
            PairKind::Bracket => "bracket",
            PairKind::Brace => "brace",
            PairKind::Block => "block",
            PairKind::Angle => "angle",
            PairKind::Tag => "tag",
        };
        let _ = writeln!(
            out,
            "{} {} <-> {} {}",
            kind,
            pos(&lines, pair.open),
            pos(&lines, pair.close),
            excerpt(
                &text[pair.open..(pair.open + 6).min(text.len())]
                    .chars()
                    .take(3)
                    .collect::<String>()
            )
        );
    }
    out
}

fn render_selection(text_with_caret: &str, lang: &str) -> String {
    let at = text_with_caret.find(CARET).expect("caret marker");
    let text = text_with_caret.replacen(CARET, "", 1);
    let snap = snap(lang, &text);
    let lines = LineIndex::new(&text);
    let mut out = format!("caret {}\n", pos(&lines, at));
    let mut range = at..at;
    let mut last_len = 0;
    while let Some(next) = snap.expand_selection(range.clone()) {
        assert!(
            next.start <= range.start
                && range.end <= next.end
                && next.len() > last_len.min(range.len()),
            "selection must grow strictly: {range:?} -> {next:?}"
        );
        let _ = writeln!(
            out,
            "{}-{} {}",
            pos(&lines, next.start),
            pos(&lines, next.end),
            excerpt(&text[next.clone()])
        );
        last_len = next.len();
        range = next;
    }
    out
}

fn render_indents(snap: &SyntaxSnapshot) -> String {
    let text = snap.text();
    let engine = IndentEngine::new(snap, IndentOptions::detect(text));
    let lines = LineIndex::new(text);
    let fixes: std::collections::HashMap<usize, usize> =
        engine.reindent(0..lines.line_count()).into_iter().collect();
    let mut out = String::new();
    for line in 0..lines.line_count() {
        let range = lines.line_range(line, text);
        if text[range.clone()].trim().is_empty() {
            continue;
        }
        let current = engine.indent_of(line);
        let enter = engine.for_enter(range.end);
        match fixes.get(&line) {
            Some(target) => {
                let _ = writeln!(out, "{:>3}: {current} -> {target}; enter {enter}", line + 1);
            }
            None => {
                let _ = writeln!(out, "{:>3}: {current}; enter {enter}", line + 1);
            }
        }
    }
    out
}

fn render_sticky(snap: &SyntaxSnapshot) -> String {
    let text = snap.text();
    let lines = LineIndex::new(text);
    let structure = snap.structure();
    let mut out = String::new();
    for line in 1..lines.line_count() {
        let scopes = sticky::sticky_scopes_with(snap, &lines, &structure, line, 5);
        if scopes.is_empty() {
            continue;
        }
        let headers: Vec<String> = scopes
            .iter()
            .map(|s| excerpt(&text[s.header.clone()]))
            .collect();
        let _ = writeln!(out, "{:>3}: {}", line + 1, headers.join(" | "));
    }
    out
}

fn render_textobjects(snap: &SyntaxSnapshot) -> String {
    let text = snap.text();
    let lines = LineIndex::new(text);
    let mut out = String::new();
    for object in textobjects::objects(snap) {
        let _ = writeln!(
            out,
            "{:?} {}-{}{}",
            object.kind,
            pos(&lines, object.outer.start),
            pos(&lines, object.outer.end),
            object
                .inner
                .map(|i| format!(" inner {}-{}", pos(&lines, i.start), pos(&lines, i.end)))
                .unwrap_or_default()
        );
    }
    for start in textobjects::method_starts(snap) {
        let _ = writeln!(out, "method {}", pos(&lines, start));
    }
    out
}

fn render_locals(snap: &SyntaxSnapshot) -> String {
    let text = snap.text();
    let lines = LineIndex::new(text);
    let found = locals::locals(snap);
    let mut out = String::new();
    for def in &found.defs {
        let _ = writeln!(
            out,
            "def {} {} {}",
            def.kind,
            def.name,
            pos(&lines, def.range.start)
        );
    }
    for reference in &found.refs {
        let target = reference
            .def
            .map(|i| {
                format!(
                    "{} {}",
                    found.defs[i].kind,
                    pos(&lines, found.defs[i].range.start)
                )
            })
            .unwrap_or_else(|| "unresolved".into());
        let _ = writeln!(
            out,
            "ref {} {} -> {target}",
            reference.name,
            pos(&lines, reference.range.start)
        );
    }
    out
}

pub fn render(lang: &str, feature: &str, source: &str) -> String {
    if feature == "selection" {
        return render_selection(source, lang);
    }
    let snap = snap(lang, source);
    match feature {
        "folds" => render_folds(&snap),
        "structure" => render_structure(&snap),
        "brackets" => render_brackets(&snap),
        "indents" => render_indents(&snap),
        "sticky" => render_sticky(&snap),
        "textobjects" => render_textobjects(&snap),
        "locals" => render_locals(&snap),
        other => panic!("unknown feature {other}"),
    }
}

#[test]
fn golden_files_match_the_rendered_features() {
    let bless = std::env::var_os("UPDATE_SNAPSHOTS").is_some();
    let mut failures = Vec::new();
    let mut checked = 0;
    for spec in LANGUAGES {
        for feature in FEATURES {
            let base = dir().join(spec.id).join(feature);
            let src = base.with_extension("src");
            let Ok(source) = std::fs::read_to_string(&src) else {
                continue;
            };
            let actual = render(spec.id, feature, &source);
            let expected_path = base.with_extension("expected");
            if bless {
                std::fs::write(&expected_path, &actual).expect("write expected");
                continue;
            }
            checked += 1;
            match std::fs::read_to_string(&expected_path) {
                Ok(expected) if expected == actual => {}
                Ok(expected) => failures.push(format!(
                    "{}/{feature}: output differs (UPDATE_SNAPSHOTS=1 to bless)\n--- expected\n{expected}--- actual\n{actual}",
                    spec.id
                )),
                Err(_) => failures.push(format!("{}/{feature}: missing .expected (UPDATE_SNAPSHOTS=1)", spec.id)),
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    assert!(
        bless || checked > 100,
        "only {checked} golden files checked"
    );
}
