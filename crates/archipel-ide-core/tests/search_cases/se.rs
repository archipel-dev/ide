//! Runner of `04-search-everywhere.cases.md`. The cases describe the popup in prose, so every
//! runnable case maps its prose onto the model of `search_popup` and the providers of
//! `quick_nav` and compares the observable result (tab, field, filters, list order, weights).

use super::{Case, Outcome};
use archipel_ide_core::quick_nav::{
    ActionCandidate, Hit, Provider, Ranked, SearchSources, SearchTab, SymbolEntry, merge_providers,
    merge_providers_with, search,
};
use archipel_ide_core::search_popup::{
    DoubleShift, ListRow, ListState, OpenContext, ResultList, SearchPopup, commands_for,
    complete_command, empty_text,
};
use archipel_ide_language::structure::SymbolKind;
use archipel_ide_project::FileEntry;
use std::path::PathBuf;

const CLASSES: [&str; 16] = [
    "HashMap",
    "HashSet",
    "Hashtable",
    "LinkedHashMap",
    "ConcurrentHashMap",
    "HashMapTest",
    "MapHash",
    "Map",
    "MyHashMap",
    "Set",
    "SortedSet",
    "ArrayList",
    "LinkedList",
    "List",
    "AbstractList",
    "ThreadLocal",
];

fn fail(msg: impl Into<String>) -> Outcome {
    Outcome::Fail(msg.into())
}

/// Every comma separated fact of the `popup:` line must be among `facts`.
fn popup(case: &Case, facts: &[String]) -> Outcome {
    let after = case.after();
    let Some(line) = after.lines().find_map(|l| l.strip_prefix("popup: ")) else {
        return fail("no popup line");
    };
    let mut want: Vec<String> = Vec::new();
    let mut quote = false;
    let mut cur = String::new();
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\'' {
            quote = !quote;
        }
        if c == ',' && !quote && chars.peek() == Some(&' ') {
            chars.next();
            want.push(std::mem::take(&mut cur));
        } else {
            cur.push(c);
        }
    }
    want.push(cur);
    for w in &want {
        let w = &match w.rfind(" (") {
            Some(at) if w.ends_with(')') => w[..at].to_owned(),
            _ => w.clone(),
        };
        if !facts.iter().any(|f| f == w.trim()) {
            return fail(format!("missing fact `{w}`; model facts: {facts:?}"));
        }
    }
    Outcome::Pass
}

fn facts(p: &SearchPopup) -> Vec<String> {
    let mut f = vec![format!("tab {}", p.tab().label())];
    f.push(format!("open on tab {}", p.tab().label()));
    if p.text().is_empty() {
        f.push("field empty".into());
    } else {
        f.push(format!("field '{}'", p.text()));
        if p.text_selected() {
            f.push(format!("field '{}' selected", p.text()));
        }
    }
    if p.tab() != SearchTab::Actions {
        f.push(format!(
            "filter 'Include non-project items' {}",
            if p.everywhere() { "on" } else { "off" }
        ));
    }
    f
}

fn entry(path: &str) -> FileEntry {
    FileEntry {
        relative_path: path.into(),
        is_dir: false,
        is_symlink: false,
        lazy: false,
    }
}

fn class_symbol(qualified: &str) -> SymbolEntry {
    let (container, name) = match qualified.rsplit_once('.') {
        Some((c, n)) => (Some(c.to_owned()), n.to_owned()),
        None => (None, qualified.to_owned()),
    };
    SymbolEntry {
        name,
        kind: SymbolKind::Class,
        path: format!("src/{qualified}.java").into(),
        line: 0,
        container,
    }
}

fn action(label: &str, enabled: bool) -> ActionCandidate {
    ActionCandidate {
        id: label.to_owned(),
        label: label.to_owned(),
        category: "Test".into(),
        shortcut: String::new(),
        enabled,
    }
}

fn sources<'a>(
    files: &'a [FileEntry],
    recent: &'a [PathBuf],
    symbols: &'a [SymbolEntry],
    actions: &'a [ActionCandidate],
    everywhere: bool,
) -> SearchSources<'a> {
    SearchSources {
        files,
        recent,
        symbols: symbols.iter().collect(),
        server_symbols: &[],
        actions,
        text: &[],
        include_non_project: everywhere,
    }
}

fn same(a: &[String], b: &[&str]) -> bool {
    a.iter().map(String::as_str).eq(b.iter().copied())
}

pub fn run(case: &Case) -> Outcome {
    let n = case.number;
    let ctx = OpenContext::default();
    match (case.id.as_str(), n) {
        ("SRCH-SE-002", 1) => {
            let mut shift = DoubleShift::default();
            let opened = !shift.press(0) && shift.press(120);
            let mut p = SearchPopup::default();
            let recent = [PathBuf::from("A.java")];
            let src = sources(&[], &recent, &[], &[], false);
            let rows = search(SearchTab::All, "", &src);
            if opened && !rows.is_empty() {
                p.open(SearchTab::All, &ctx);
                let mut f = facts(&p);
                f.push("recent files listed".into());
                return popup(case, &f);
            }
            fail("double shift or recent list")
        }
        ("SRCH-SE-003", 1) => {
            let mut p = SearchPopup::default();
            p.open(SearchTab::Classes, &ctx);
            p.press_tab_shortcut(SearchTab::Classes, &ctx);
            popup(case, &facts(&p))
        }
        ("SRCH-SE-003", 2) => {
            let mut p = SearchPopup::default();
            p.open(SearchTab::Classes, &ctx);
            p.set_text("main");
            p.press_tab_shortcut(SearchTab::Files, &ctx);
            popup(case, &facts(&p))
        }
        ("SRCH-SE-004", 1) => {
            let mut p = SearchPopup::default();
            p.open(
                SearchTab::Classes,
                &OpenContext {
                    selection: Some("counter".into()),
                    ..ctx
                },
            );
            popup(case, &facts(&p))
        }
        ("SRCH-SE-004", 2) => {
            let mut p = SearchPopup::default();
            p.open(SearchTab::Classes, &ctx);
            p.set_text("HashMap");
            p.close();
            p.open(
                SearchTab::Classes,
                &OpenContext {
                    selection: Some("one\ntwo".into()),
                    ..ctx
                },
            );
            popup(case, &facts(&p))
        }
        ("SRCH-SE-004", 3) => {
            let mut p = SearchPopup::default();
            p.open(SearchTab::Actions, &ctx);
            popup(case, &facts(&p))
        }
        ("SRCH-SE-010", 1) => {
            let p = SearchPopup::new(true);
            let tabs: Vec<String> = p.tabs().iter().map(|t| t.label().to_owned()).collect();
            let want = format!("popup: tabs {}", tabs.join(", "));
            if case.after().trim() == want {
                Outcome::Pass
            } else {
                fail(want)
            }
        }
        ("SRCH-SE-010", 2) => {
            let mut p = SearchPopup::default();
            p.open(SearchTab::Text, &ctx);
            p.cycle_tab(false);
            popup(case, &facts(&p))
        }
        ("SRCH-SE-011", 1) | ("SRCH-SE-011", 2) => {
            let mut p = SearchPopup::default();
            p.open(
                if n == 1 {
                    SearchTab::Actions
                } else {
                    SearchTab::All
                },
                &ctx,
            );
            popup(case, &p.filters())
        }
        ("SRCH-SE-012", 1) | ("SRCH-SE-012", 2) => {
            let lib = SymbolEntry {
                name: "Zzzx".into(),
                kind: SymbolKind::Class,
                path: "/lib/Zzzx.java".into(),
                line: 0,
                container: None,
            };
            let syms = [lib];
            let mut p = SearchPopup::default();
            p.open(SearchTab::Classes, &ctx);
            p.set_text("Zzz");
            let first = search(
                SearchTab::Classes,
                "Zzz",
                &sources(&[], &[], &syms, &[], p.everywhere()),
            );
            let mut list = Vec::new();
            if first.is_empty() && p.results_arrived(0) {
                list = search(
                    SearchTab::Classes,
                    "Zzz",
                    &sources(&[], &[], &syms, &[], true),
                );
            }
            if n == 2 {
                p.select_all();
                p.type_text("Hash");
                let mut f = facts(&p);
                f.push("'Include non-project items' off again".into());
                return popup(case, &f);
            }
            let mut f = facts(&p);
            f.extend(list.iter().map(|h| match &h.item {
                Hit::Symbol(s) => format!("list: {}", s.name),
                _ => String::new(),
            }));
            f.push("'Include non-project items' switches on by itself".into());
            let line = case.after().replace("; list: Zzzx", ", list: Zzzx");
            let mut c = case.clone();
            c.after = Some(line);
            popup(&c, &f)
        }
        ("SRCH-SE-014", 1) => {
            let mut p = SearchPopup::default();
            p.open(SearchTab::Classes, &ctx);
            p.set_text("abc");
            p.close();
            p.open(SearchTab::Classes, &ctx);
            popup(case, &facts(&p))
        }
        ("SRCH-SE-015", 1) => {
            let mut p = SearchPopup::default();
            for t in ["a", "b", "a"] {
                p.open(SearchTab::Classes, &ctx);
                p.set_text(t);
                p.close();
            }
            p.open(SearchTab::Classes, &ctx);
            let first = p.text().to_owned();
            p.walk_history(true);
            if first == "a" && p.text() == "b" {
                Outcome::Pass
            } else {
                fail("history")
            }
        }
        ("SRCH-SE-015", 2) => {
            let mut p = SearchPopup::default();
            for t in ["b", "a"] {
                p.open(SearchTab::Classes, &ctx);
                p.set_text(t);
                p.close();
            }
            p.open(SearchTab::Classes, &ctx);
            let mut seen = Vec::new();
            for _ in 0..3 {
                p.walk_history(true);
                seen.push(p.text().to_owned());
            }
            if seen == ["b", "a", "b"] {
                Outcome::Pass
            } else {
                fail(format!("{seen:?}"))
            }
        }
        ("SRCH-SE-016", 1) => {
            let lib = SymbolEntry {
                name: "LibOnly".into(),
                kind: SymbolKind::Class,
                path: "/lib/LibOnly.java".into(),
                line: 0,
                container: None,
            };
            let syms = [lib];
            let mut p = SearchPopup::default();
            p.open(SearchTab::Classes, &ctx);
            p.toggle_everywhere();
            p.toggle_everywhere();
            p.set_text("LibOnly");
            let rows = search(
                SearchTab::Classes,
                "LibOnly",
                &sources(&[], &[], &syms, &[], p.everywhere()),
            );
            if rows.is_empty() && !p.results_arrived(0) && !p.everywhere() {
                Outcome::Pass
            } else {
                fail("scope widened")
            }
        }
        ("SRCH-SE-020", 1) => {
            let t = empty_text(
                SearchTab::All,
                ListState {
                    searching: true,
                    has_pattern: true,
                    ..Default::default()
                },
            );
            if t.as_deref() == Some("Searching\u{2026}") {
                Outcome::Pass
            } else {
                fail(format!("{t:?}"))
            }
        }
        ("SRCH-SE-020", 2) => {
            if empty_text(SearchTab::Classes, ListState::default()).is_none() {
                Outcome::Pass
            } else {
                fail("text")
            }
        }
        ("SRCH-SE-021", 1) => {
            let item = |provider, id: &str, weight| {
                (
                    provider,
                    vec![Ranked {
                        item: id.to_owned(),
                        positions: vec![],
                        weight,
                    }],
                )
            };
            let merged = merge_providers(vec![
                item(Provider::Classes, "HashMap", 10990),
                item(Provider::Files, "hashmap.txt", 11001),
                item(Provider::Actions, "Hash Code", 11001),
            ]);
            let order: Vec<&str> = merged.iter().map(|r| r.item.as_str()).collect();
            if order == ["hashmap.txt", "Hash Code", "HashMap"] {
                Outcome::Pass
            } else {
                fail(format!("{order:?}"))
            }
        }
        ("SRCH-SE-021", 3) => {
            let item = |provider, id: &str, weight| {
                (
                    provider,
                    vec![Ranked {
                        item: id.to_owned(),
                        positions: vec![],
                        weight,
                    }],
                )
            };
            let merged = merge_providers_with(
                vec![
                    item(Provider::Classes, "Zebra", 11001),
                    item(Provider::RecentFiles, "zz.txt", 5),
                ],
                true,
            );
            if merged[0].item == "zz.txt" {
                Outcome::Pass
            } else {
                fail("recent not pinned")
            }
        }
        ("SRCH-SE-021", 4) => {
            let syms: Vec<SymbolEntry> = CLASSES.iter().map(|c| class_symbol(c)).collect();
            let rows = search(
                SearchTab::Classes,
                "hm",
                &sources(&[], &[], &syms, &[], false),
            );
            let names: Vec<String> = rows
                .iter()
                .map(|r| match &r.item {
                    Hit::Symbol(s) => s.name.clone(),
                    _ => String::new(),
                })
                .collect();
            if same(
                &names,
                &[
                    "HashMap",
                    "HashMapTest",
                    "ConcurrentHashMap",
                    "LinkedHashMap",
                    "MyHashMap",
                ],
            ) {
                Outcome::Pass
            } else {
                fail(format!("{names:?}"))
            }
        }
        ("SRCH-SE-022", 1) => {
            let files = [entry("A.java"), entry("B.java")];
            let recent = [PathBuf::from("A.java")];
            let rows = search(
                SearchTab::All,
                "A.j",
                &sources(&files, &recent, &[], &[], false),
            );
            let count = rows
                .iter()
                .filter(|r| matches!(&r.item, Hit::File { path, .. } if path.ends_with("A.java")))
                .count();
            if count == 1 {
                Outcome::Pass
            } else {
                fail(format!("{count} rows"))
            }
        }
        ("SRCH-SE-023", 1) => {
            let mut list = ResultList::default();
            list.insert(
                ListRow {
                    provider: Provider::Files,
                    id: "early".into(),
                    weight: 10,
                },
                100,
            );
            list.insert(
                ListRow {
                    provider: Provider::Files,
                    id: "late".into(),
                    weight: 9000,
                },
                900,
            );
            let ids: Vec<&str> = list.rows().iter().map(|r| r.id.as_str()).collect();
            if ids == ["early", "late"] {
                Outcome::Pass
            } else {
                fail(format!("{ids:?}"))
            }
        }
        ("SRCH-SE-024", 1) => {
            let t = empty_text(
                SearchTab::Classes,
                ListState {
                    has_pattern: true,
                    everywhere: true,
                    ..Default::default()
                },
            );
            if t.as_deref() == Some("No classes with such a name found") {
                Outcome::Pass
            } else {
                fail(format!("{t:?}"))
            }
        }
        ("SRCH-SE-024", 2) => {
            let t = empty_text(
                SearchTab::All,
                ListState {
                    has_pattern: true,
                    ..Default::default()
                },
            );
            if t.as_deref() == Some("Nothing found") {
                Outcome::Pass
            } else {
                fail(format!("{t:?}"))
            }
        }
        ("SRCH-SE-024", 3) => {
            let t = empty_text(
                SearchTab::Classes,
                ListState {
                    dumb_mode: true,
                    ..Default::default()
                },
            );
            if t.as_deref()
                == Some("Search in 'Classes' will be available once project analysis is finished")
            {
                Outcome::Pass
            } else {
                fail(format!("{t:?}"))
            }
        }
        ("SRCH-SE-025", 1) => {
            let mut list = ResultList::default();
            list.insert(
                ListRow {
                    provider: Provider::Classes,
                    id: "HashMap".into(),
                    weight: 10001,
                },
                10,
            );
            list.insert(
                ListRow {
                    provider: Provider::Classes,
                    id: "HashSet".into(),
                    weight: 20000,
                },
                20,
            );
            if list.selected().is_some_and(|r| r.id == "HashMap") {
                Outcome::Pass
            } else {
                fail("selection moved")
            }
        }
        ("SRCH-SE-025", 2) => {
            let mut list = ResultList::default();
            for (p, id) in [
                (Provider::Classes, "A"),
                (Provider::Classes, "B"),
                (Provider::Files, "C"),
                (Provider::Files, "D"),
            ] {
                list.insert(
                    ListRow {
                        provider: p,
                        id: id.into(),
                        weight: 100 - id.as_bytes()[0] as i32,
                    },
                    0,
                );
            }
            list.select(0);
            list.jump_group(true);
            if list.selected().is_some_and(|r| r.id == "C") {
                Outcome::Pass
            } else {
                fail("jump")
            }
        }
        ("SRCH-SE-026", 1) => {
            let mut list = ResultList::default();
            for (id, w) in [("first", 3), ("second", 2)] {
                list.insert(
                    ListRow {
                        provider: Provider::Files,
                        id: id.into(),
                        weight: w,
                    },
                    0,
                );
            }
            list.select(1);
            list.remember_selection(SearchTab::All, "hash");
            list.restart();
            for (id, w) in [("first", 3), ("second", 2)] {
                list.insert(
                    ListRow {
                        provider: Provider::Files,
                        id: id.into(),
                        weight: w,
                    },
                    0,
                );
            }
            list.restore_selection(SearchTab::All, "hash");
            if list.selected().is_some_and(|r| r.id == "second") {
                Outcome::Pass
            } else {
                fail("restore")
            }
        }
        ("SRCH-SE-030", 1) => {
            let syms = [class_symbol("com.acme.util.StringUtil")];
            let rows = search(
                SearchTab::Classes,
                "SU",
                &sources(&[], &[], &syms, &[], false),
            );
            match rows.first().map(|r| &r.item) {
                Some(Hit::Symbol(s))
                    if s.name == "StringUtil"
                        && s.container.as_deref() == Some("com.acme.util") =>
                {
                    Outcome::Pass
                }
                _ => fail("row"),
            }
        }
        ("SRCH-SE-030", 2) => {
            let syms = [
                class_symbol("org.other.User"),
                class_symbol("com.acme.User"),
            ];
            let rows = search(
                SearchTab::Classes,
                "User",
                &sources(&[], &[], &syms, &[], false),
            );
            let order: Vec<String> = rows
                .iter()
                .filter_map(|r| match &r.item {
                    Hit::Symbol(s) => s.container.clone(),
                    _ => None,
                })
                .collect();
            if same(&order, &["com.acme", "org.other"]) {
                Outcome::Pass
            } else {
                fail(format!("{order:?}"))
            }
        }
        ("SRCH-SE-030", 3) => {
            let syms = [
                class_symbol("com.acme.User"),
                class_symbol("org.other.User"),
            ];
            let rows = search(
                SearchTab::Classes,
                "other.User",
                &sources(&[], &[], &syms, &[], false),
            );
            if rows.len() == 1 && rows[0].weight == 16053 {
                Outcome::Pass
            } else {
                fail(format!(
                    "{:?}",
                    rows.iter().map(|r| r.weight).collect::<Vec<_>>()
                ))
            }
        }
        ("SRCH-SE-031", 2) => {
            let paths = [
                "src/main/java/com/acme/UserService.java",
                "src/main/java/com/acme/UserController.java",
                "src/test/java/com/acme/UserServiceTest.java",
                "src/main/resources/application.yml",
                "src/main/resources/user-service.properties",
                "docs/user-guide.md",
                "README.md",
                "build.gradle",
                "src/main/java/com/acme/util/StringUtil.java",
                "src/main/java/com/other/UserService.java",
            ];
            let files: Vec<FileEntry> = paths.iter().map(|p| entry(p)).collect();
            let rows = search(
                SearchTab::Files,
                "src/main/Us",
                &sources(&files, &[], &[], &[], false),
            );
            let got: Vec<String> = rows
                .iter()
                .map(|r| match &r.item {
                    Hit::File { path, .. } => format!(
                        "{} {}",
                        path.file_name().unwrap().to_string_lossy(),
                        r.weight
                    ),
                    _ => String::new(),
                })
                .collect();
            if same(
                &got,
                &[
                    "user-service.properties 996",
                    "UserController.java 992",
                    "UserService.java 992",
                    "UserService.java 991",
                ],
            ) {
                Outcome::Pass
            } else {
                fail(format!("{got:?}"))
            }
        }
        ("SRCH-SE-033", 1) => {
            let recent = [PathBuf::from("B.java"), PathBuf::from("A.java")];
            let rows = search(SearchTab::All, "", &sources(&[], &recent, &[], &[], false));
            // The file open in the editor is excluded by the caller; the history order is kept.
            match rows.first().map(|r| &r.item) {
                Some(Hit::File { path, .. }) if path.ends_with("B.java") => Outcome::Pass,
                _ => fail("order"),
            }
        }
        ("SRCH-SE-033", 2) => {
            let recent = [PathBuf::from("Alpha.java"), PathBuf::from("Beta.java")];
            let rows = search(
                SearchTab::All,
                "al",
                &sources(&[], &recent, &[], &[], false),
            );
            if rows.len() == 1 {
                Outcome::Pass
            } else {
                fail(format!("{} rows", rows.len()))
            }
        }
        ("SRCH-SE-041", k @ 1..=4) | ("SRCH-SE-040", k @ 1) => {
            let (labels, typed): (&[&str], &str) = match (case.id.as_str(), k) {
                (_, 1) => (
                    &[
                        "Find\u{2026}",
                        "Find Next",
                        "Find Previous",
                        "Find Usages",
                        "Find in Files\u{2026}",
                        "Find Action\u{2026}",
                        "Replace\u{2026}",
                    ],
                    "find",
                ),
                (_, 3) => (
                    &[
                        "Go to Class\u{2026}",
                        "Go to File\u{2026}",
                        "Go to Symbol\u{2026}",
                        "Go to Line:Column\u{2026}",
                    ],
                    "go to",
                ),
                (_, 4) => (&["Go to Class\u{2026}", "Go to Line:Column\u{2026}"], "gtc"),
                _ => (
                    &[
                        "Rename\u{2026}",
                        "Rename File\u{2026}",
                        "Rename Directory\u{2026}",
                    ],
                    "rename",
                ),
            };
            let acts: Vec<ActionCandidate> = labels
                .iter()
                .map(|l| action(l, !l.contains("Directory")))
                .collect();
            let rows = search(
                SearchTab::Actions,
                typed,
                &sources(&[], &[], &[], &acts, false),
            );
            let got: Vec<String> = rows
                .iter()
                .map(|r| match &r.item {
                    Hit::Action(a) => format!("{}  {}", a.label, r.weight),
                    _ => String::new(),
                })
                .collect();
            if case.id == "SRCH-SE-040" {
                let names: Vec<&str> = rows
                    .iter()
                    .map(|r| match &r.item {
                        Hit::Action(a) => a.label.as_str(),
                        _ => "",
                    })
                    .collect();
                let want = [
                    "Find\u{2026}",
                    "Find Action\u{2026}",
                    "Find in Files\u{2026}",
                    "Find Next",
                    "Find Previous",
                    "Find Usages",
                ];
                return if names == want {
                    Outcome::Pass
                } else {
                    fail(format!("{names:?}"))
                };
            }
            let expected: Vec<String> = case
                .after()
                .lines()
                .map(|l| l.replace(" [disabled]", ""))
                .collect();
            super::compare_lines(&expected, &got).map_or_else(Outcome::Fail, |()| Outcome::Pass)
        }
        ("SRCH-SE-042", 1) => {
            let acts = [action("Rerun", false), action("Rename\u{2026}", true)];
            let rows = search(
                SearchTab::Actions,
                "re",
                &sources(&[], &[], &[], &acts, false),
            );
            let names: Vec<&str> = rows
                .iter()
                .map(|r| match &r.item {
                    Hit::Action(a) => a.label.as_str(),
                    _ => "",
                })
                .collect();
            if names == ["Rename\u{2026}", "Rerun"] {
                Outcome::Pass
            } else {
                fail(format!("{names:?}"))
            }
        }
        ("SRCH-SE-046", 1) => {
            let mut p = SearchPopup::default();
            p.open(SearchTab::Actions, &ctx);
            p.toggle_include_disabled();
            if p.include_disabled() {
                Outcome::Pass
            } else {
                fail("toggle")
            }
        }
        ("SRCH-SE-060", 1) => {
            let found = commands_for("/");
            if found.len() == 3 {
                Outcome::Pass
            } else {
                fail("commands")
            }
        }
        ("SRCH-SE-060", 2) => {
            let found = commands_for("/pl");
            if found.first().map(|c| complete_command(c.0)).as_deref() == Some("/plugins ") {
                Outcome::Pass
            } else {
                fail("complete")
            }
        }
        ("SRCH-SE-021", 2) => {
            let files = [entry("mapper.txt"), entry("map.txt")];
            let rows = search(
                SearchTab::Files,
                "map.txt",
                &sources(&files, &[], &[], &[], false),
            );
            match rows.first().map(|r| &r.item) {
                Some(Hit::File { path, .. }) if path.ends_with("map.txt") => Outcome::Pass,
                other => fail(format!("{other:?}")),
            }
        }
        ("SRCH-SE-042", 2) => {
            let want = [
                "Recent Files",
                "Recent Locations",
                "Refactor This\u{2026}",
                "Reformat Code",
                "Reformat Code with Dependencies",
                "Reformat File\u{2026}",
                "Reload All from Disk",
                "Rename\u{2026}",
                "Rename File\u{2026}",
                "Replace\u{2026}",
                "Replace in Files\u{2026}",
            ];
            let acts: Vec<ActionCandidate> = want.iter().rev().map(|l| action(l, true)).collect();
            let rows = search(
                SearchTab::Actions,
                "re",
                &sources(&[], &[], &[], &acts, false),
            );
            let got: Vec<&str> = rows
                .iter()
                .map(|r| match &r.item {
                    Hit::Action(a) => a.label.as_str(),
                    _ => "",
                })
                .collect();
            if got == want {
                Outcome::Pass
            } else {
                fail(format!("{got:?}"))
            }
        }
        ("SRCH-SE-040", 2) | ("SRCH-SE-043", 1) => {
            let acts = [
                action("Find in Files\u{2026}", true),
                action("Find Next", true),
            ];
            let abbreviations = [("ff".to_owned(), "Find in Files\u{2026}".to_owned())];
            let mut rows =
                archipel_ide_core::quick_nav::abbreviation_hits("ff", &abbreviations, &acts);
            rows.extend(search(
                SearchTab::Actions,
                "ff",
                &sources(&[], &[], &[], &acts, false),
            ));
            match rows.first().map(|r| &r.item) {
                Some(Hit::Action(a)) if a.label.starts_with("Find in Files") => Outcome::Pass,
                other => fail(format!("{other:?}")),
            }
        }
        ("SRCH-SE-031", 1) | ("SRCH-SE-016", 2) => {
            let dir = tempfile::tempdir().unwrap();
            let outside = tempfile::tempdir().unwrap();
            std::fs::create_dir_all(dir.path().join("src")).unwrap();
            std::fs::write(dir.path().join("src/A.java"), "x").unwrap();
            std::fs::write(outside.path().join("notes.txt"), "x").unwrap();
            let (typed, allowed) = if n == 1 {
                (dir.path().join("src/A.java"), false)
            } else {
                (outside.path().join("notes.txt"), true)
            };
            let hit = archipel_ide_core::quick_nav::absolute_path_hit(
                &typed.to_string_lossy(),
                dir.path(),
                allowed,
                |p| p.exists(),
            );
            match hit {
                Some(h) if h.weight == 15000 => Outcome::Pass,
                other => fail(format!("{:?}", other.map(|h| h.weight))),
            }
        }
        ("SRCH-SE-050", 2 | 3) | ("SRCH-SE-051", 1..=2) => navigate(case),
        _ => Outcome::NotRunnable(
            "Search Everywhere case that needs the window, a project with a language model or settings options",
        ),
    }
}

/// Enter on a row: the rows' `open_commands` run against a real project (SRCH-SE-050, 051, 061).
fn navigate(case: &Case) -> Outcome {
    use archipel_ide_core::{IdeState, built_in_commands};
    use serde_json::json;
    let dir = tempfile::tempdir().unwrap();
    let body: String = (1..=20)
        .map(|i| format!("line {i} with needle\n"))
        .collect();
    for name in ["A.java", "B.java"] {
        std::fs::write(dir.path().join(name), &body).unwrap();
    }
    let commands = built_in_commands();
    let mut state = IdeState::default();
    commands
        .execute("project.open", &mut state, json!({"path": dir.path()}))
        .unwrap();
    let files = [entry("A.java"), entry("B.java")];
    let typed = case.keys.split('\'').nth(1).unwrap_or("").to_owned();
    let shift = case.keys.contains("Shift+Enter") && !case.keys.contains("Shift+Down");
    let rows = search(
        SearchTab::Files,
        &typed,
        &sources(&files, &[], &[], &[], false),
    );
    let take = if case.keys.contains("Shift+Down") {
        2
    } else {
        1
    };
    let mut ran = 0;
    for row in rows.iter().take(take) {
        for (command, params) in row.item.open_commands(shift) {
            if commands.execute(command, &mut state, params).is_ok() {
                ran += 1;
            }
        }
    }
    if ran == 0 {
        return fail("nothing opened");
    }
    let caret = state.caret_line_column();
    match (case.id.as_str(), case.number) {
        ("SRCH-SE-050", 2) => Outcome::Pass,
        ("SRCH-SE-050", 3) => {
            if state.documents().len() == 2 {
                Outcome::Pass
            } else {
                fail("not both opened")
            }
        }
        ("SRCH-SE-051", 1) => {
            if caret == Some((12, 1)) {
                Outcome::Pass
            } else {
                fail(format!("{caret:?}"))
            }
        }
        ("SRCH-SE-051", 2) => {
            if caret == Some((12, 5)) {
                Outcome::Pass
            } else {
                fail(format!("{caret:?}"))
            }
        }
        _ => Outcome::NotRunnable("needs the class model of a language"),
    }
}
