//! Analysis-core benchmarks (release): incremental reparse with changed
//! ranges, host edits and snapshots, edit-map composition, anchor
//! resolution, cached-value lookups, and the memory held by snapshots.
//!
//! `cargo bench -p archipel-ide-analysis --bench analysis`. With
//! `ARCHIPEL_PERF_ASSERT=1` it exits non-zero when a budget of
//! `docs/analysis/architecture.md` is exceeded.

use archipel_ide_analysis::cache::{Slot, resident_bytes};
use archipel_ide_analysis::testing::byte_edit;
use archipel_ide_analysis::{
    AnalysisHost, Anchor, CancelToken, Change, EditMap, FileRev, HostConfig, SyntaxSnapshot,
    SyntaxUpdate,
};
use archipel_ide_language::{SyntaxTree, registry};
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

const LINES: usize = 50_000;

fn java_source(lines: usize) -> String {
    let mut text = String::from("package demo;\n\npublic class Big {\n");
    let mut method = 0;
    while text.lines().count() < lines - 1 {
        text.push_str(&format!(
            "    int method{method}(int value) {{\n        int result = value * {method} + 1; // step\n        return result;\n    }}\n"
        ));
        method += 1;
    }
    text.push_str("}\n");
    text
}

struct Stats {
    name: &'static str,
    samples: Vec<Duration>,
}

impl Stats {
    fn percentile(&self, p: f64) -> Duration {
        let mut sorted = self.samples.clone();
        sorted.sort();
        sorted[((sorted.len() - 1) as f64 * p) as usize]
    }

    fn report(&self) -> Duration {
        let (p50, p95) = (self.percentile(0.5), self.percentile(0.95));
        println!(
            "{:<44} p50 {:>9.3} ms   p95 {:>9.3} ms   ({} samples)",
            self.name,
            p50.as_secs_f64() * 1e3,
            p95.as_secs_f64() * 1e3,
            self.samples.len()
        );
        p95
    }
}

fn measure(name: &'static str, runs: usize, mut f: impl FnMut(usize)) -> Stats {
    let mut samples = Vec::with_capacity(runs);
    for run in 0..runs {
        let start = Instant::now();
        f(run);
        samples.push(start.elapsed());
    }
    Stats { name, samples }
}

fn main() {
    let assert = std::env::var_os("ARCHIPEL_PERF_ASSERT").is_some();
    let mut failures = Vec::new();
    let source = java_source(LINES);
    let typical = java_source(2_000);
    let lang = registry::by_id("java").expect("java");
    println!("Java source: {} lines, {} KiB", LINES, source.len() / 1024);

    // 1. Incremental reparse + changed ranges (typing in the middle).
    for (label, file, fraction) in [
        ("2k-line file, middle", &typical, 0.5),
        ("50k-line file, start", &source, 0.001),
        ("50k-line file, middle", &source, 0.5),
    ] {
        let mut tree = SyntaxTree::parse(lang, file).expect("parse");
        let mut text = file.clone();
        let first = (text.len() as f64 * fraction) as usize;
        let first = text[..first].rfind('\n').map_or(0, |i| i + 1) + 8;
        let mut samples = Vec::new();
        let mut parse_only = Vec::new();
        for (run, at) in (0..300).zip(first..) {
            let insert = if run % 10 == 9 { " " } else { "x" };
            let edit = byte_edit(&text, at..at, insert);
            text.insert_str(at, insert);
            // Raw tree-sitter incremental parse of the same edit (the floor).
            let mut raw = tree.tree().clone();
            raw.edit(&tree_sitter::InputEdit {
                start_byte: edit.start_byte,
                old_end_byte: edit.old_end_byte,
                new_end_byte: edit.new_end_byte,
                start_position: tree_sitter::Point::new(
                    edit.start_point.row,
                    edit.start_point.column,
                ),
                old_end_position: tree_sitter::Point::new(
                    edit.old_end_point.row,
                    edit.old_end_point.column,
                ),
                new_end_position: tree_sitter::Point::new(
                    edit.new_end_point.row,
                    edit.new_end_point.column,
                ),
            });
            let start = Instant::now();
            let mut parser = tree_sitter::Parser::new();
            parser.set_language(&lang.grammar()).expect("grammar");
            let reparsed = parser.parse(&text, Some(&raw)).expect("parse");
            std::hint::black_box(reparsed.changed_ranges(&raw).count());
            parse_only.push(start.elapsed());
            let start = Instant::now();
            let result = tree.reparse(&text, &[edit]).expect("reparse");
            samples.push(start.elapsed());
            assert!(!result.highlight.is_empty());
        }
        println!("typing in a {label}:");
        let reparse = Stats {
            name: "  reparse + changed ranges (keystroke)",
            samples,
        };
        let raw = Stats {
            name: "  raw tree-sitter reparse + changed_ranges",
            samples: parse_only,
        }
        .report();
        // Budget: 2 ms, or 25 % over the raw tree-sitter floor on a loaded machine.
        let floor = raw.mul_f64(1.25).max(Duration::from_millis(2));
        if reparse.report() > floor && file.len() == typical.len() {
            failures.push("reparse p95 over budget (2k-line file)");
        }
    }

    // 2. Host apply(Edit) with the editor's snapshot + taking a snapshot.
    let mut host = AnalysisHost::with_defaults(HostConfig {
        threads: 0,
        interface_debounce: Duration::from_secs(3600),
        ..HostConfig::default()
    });
    let file = host.file_id(Path::new("/bench/Big.java"));
    let mut tree = SyntaxTree::parse(lang, &source).expect("parse");
    let mut text = source.clone();
    let at = text.len() / 2;
    host.apply(Change::Reset {
        file,
        new_rev: FileRev(1),
        text: text.as_str().into(),
        syntax: Some(SyntaxSnapshot::from_tree(
            file,
            FileRev(1),
            &tree,
            text.as_str().into(),
        )),
    });
    // Keep a snapshot alive so every apply pays copy-on-write.
    let mut held = host.snapshot();
    let mut texts: Vec<Arc<str>> = Vec::new();
    let mut host_samples = Vec::new();
    for (run, at) in (0..300).zip(at..) {
        let edit = byte_edit(&text, at..at, "y");
        text.insert(at, 'y');
        let reparse = tree.reparse(&text, &[edit]).expect("reparse");
        texts.push(text.as_str().into());
        let rev = FileRev(run as u64 + 2);
        let snapshot = SyntaxSnapshot::from_tree(file, rev, &tree, texts[run].clone());
        let changed = reparse.changed;
        let start = Instant::now();
        host.apply(Change::Edit {
            file,
            edits: vec![edit],
            new_rev: rev,
            text: texts[run].clone(),
            syntax: Some(SyntaxUpdate { snapshot, changed }),
        });
        let next = host.snapshot();
        let _ = host.take_events();
        let elapsed = start.elapsed();
        // Freeing the previous revision's text and tree is the editor's
        // cost either way; keep it out of the measurement.
        let previous = std::mem::replace(&mut held, next);
        texts[run] = "".into();
        drop(previous);
        host_samples.push(elapsed);
    }
    let host_stats = Stats {
        name: "host apply(Edit) + snapshot",
        samples: host_samples,
    };
    if host_stats.report() > Duration::from_micros(500) {
        failures.push("host apply p95 > 0.5 ms");
    }
    let since = measure("edits_since over 300 revisions", 50, |_| {
        let map = held.edits_since(file, FileRev(1)).expect("journal");
        assert!(!map.is_identity());
    });
    since.report();

    // 3. Edit-map composition of 10k keystrokes.
    let mut journal = Vec::new();
    for (i, len) in (0..10_000usize).zip(100_000usize..) {
        let at = (i * 7919) % len;
        journal.push(archipel_ide_language::ByteEdit {
            start_byte: at,
            old_end_byte: at,
            new_end_byte: at + 1,
            start_point: Default::default(),
            old_end_point: Default::default(),
            new_end_point: Default::default(),
        });
    }
    let mut map = EditMap::new();
    let build = measure("EditMap push (10k scattered keystrokes)", 1, |_| {
        map = EditMap::from_edits(&journal);
    });
    build.report();
    let lookups = measure("EditMap map (10k pieces)", 10_000, |i| {
        std::hint::black_box(map.map(i * 10));
    });
    lookups.report();

    // 4. Anchor resolution after an edit far away.
    let snap = held.syntax(file).expect("syntax");
    let methods: Vec<Anchor> = snap
        .root()
        .preorder()
        .filter(|n| n.kind() == "method_declaration")
        .step_by(97)
        .map(Anchor::new)
        .collect();
    let edit_map = EditMap::from_edits(&[byte_edit(snap.text(), 20..20, "// x\n")]);
    let mut moved = snap.text().to_string();
    moved.insert_str(20, "// x\n");
    let after = SyntaxSnapshot::parse(file, FileRev(9999), lang, moved.into()).expect("parse");
    let mut index = 0;
    let anchors = measure(
        "Anchor::resolve (exact, 50k-line file)",
        methods.len(),
        |_| {
            let found = methods[index].resolve(&after, &edit_map);
            assert!(found.is_some());
            index += 1;
        },
    );
    anchors.report();

    // 5. Cached-value hits.
    const SLOT: Slot<u32, u64> = Slot::new("bench");
    let db = held.db();
    let cancel = CancelToken::new();
    for key in 0..1000 {
        db.cached(SLOT, key, &cancel, |cx| {
            cx.file(file);
            Ok(u64::from(key))
        })
        .expect("value");
    }
    let hits = measure("Db::cached hit (1k entries)", 10_000, |i| {
        let value = db
            .cached(SLOT, (i % 1000) as u32, &cancel, |_| Ok(0))
            .expect("hit");
        std::hint::black_box(value);
    });
    hits.report();

    // 6. Memory: snapshots share trees and text.
    let base = resident_bytes();
    let snapshots: Vec<SyntaxSnapshot> = (0..1000)
        .map(|_| held.syntax(file).expect("syntax"))
        .collect();
    let grown = resident_bytes();
    if let (Some(base), Some(grown)) = (base, grown) {
        let per = grown.saturating_sub(base) / snapshots.len() as u64;
        println!(
            "memory: 1000 snapshots of the 50k-line file: +{} KiB total ({per} B each)",
            grown.saturating_sub(base) / 1024
        );
        if per > 4096 {
            failures.push("snapshots copy data");
        }
    }
    let full = SyntaxTree::parse(lang, &source).expect("parse");
    let before = resident_bytes();
    let copies: Vec<SyntaxTree> = (0..20)
        .map(|_| SyntaxTree::parse(lang, &source).expect("parse"))
        .collect();
    if let (Some(before), Some(after)) = (before, resident_bytes()) {
        println!(
            "memory: one parsed 50k-line tree: ~{} KiB",
            after.saturating_sub(before) / 1024 / copies.len() as u64
        );
    }
    drop((full, copies, snapshots));
    println!("cache: {:?}", host.cache().stats());

    if assert && !failures.is_empty() {
        eprintln!("budgets exceeded: {failures:?}");
        std::process::exit(1);
    }
}
