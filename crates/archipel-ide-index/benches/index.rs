//! Indexing benchmark on a generated tree (default 20,000 files).
//!
//! `cargo bench -p archipel-ide-index --bench index` prints, for a cold
//! persistent index and a warm restart: time until `FileName` is ready (Go to
//! File works), time until every index is ready, resident memory, store size
//! and query latencies. `ARCHIPEL_BENCH_FILES` changes the file count;
//! `ARCHIPEL_PERF_ASSERT=1` turns the spec 1.6 targets into assertions
//! (FileName within 1 s, Go to File query within 50 ms).

use archipel_ide_analysis::vfs::DiskVfs;
use archipel_ide_analysis::{AnalysisHost, HostConfig, IndexState, default_registry};
use archipel_ide_index::indexes::ids;
use archipel_ide_index::readers::{self, SymbolLookup};
use archipel_ide_index::{IndexConfig, IndexService};
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn pick<'a>(&mut self, items: &[&'a str]) -> &'a str {
        items[(self.next() % items.len() as u64) as usize]
    }
}

const WORDS: &[&str] = &[
    "order", "cart", "price", "total", "user", "session", "token", "buffer", "config", "client",
    "server", "request", "response", "handler", "cache", "index", "query", "result", "stream",
    "event", "state", "value", "count", "limit", "offset", "name", "path", "file", "node", "tree",
];

fn ident(rng: &mut Rng) -> String {
    let a = rng.pick(WORDS);
    let b = rng.pick(WORDS);
    format!("{a}{}{}", b[..1].to_uppercase(), &b[1..])
}

fn java(n: usize, rng: &mut Rng) -> String {
    let mut out = format!(
        "package com.acme.m{};\n\nimport java.util.List;\nimport java.util.Map;\n\n",
        n % 97
    );
    out.push_str(&format!(
        "/** Service {n}. */\npublic class Service{n} extends Base{} implements Api{} {{\n",
        n % 40,
        n % 25
    ));
    for m in 0..8 {
        let name = ident(rng);
        out.push_str(&format!(
            "    // {} {}\n    public {} {name}{m}(String {}, int {}) {{\n        String s = \"{}\";\n        for (int i = 0; i < {}; i++) {{ {}.add({}); }}\n        return {}.get({});\n    }}\n",
            if rng.next().is_multiple_of(17) { "TODO" } else { "note" },
            ident(rng),
            rng.pick(&["int", "String", "void", "boolean", "long"]),
            ident(rng),
            ident(rng),
            ident(rng),
            rng.next() % 100,
            ident(rng),
            ident(rng),
            ident(rng),
            ident(rng),
        ));
    }
    out.push_str("}\n");
    out
}

fn rust(n: usize, rng: &mut Rng) -> String {
    let mut out = format!(
        "pub struct Item{n} {{ pub {}: u32 }}\n\npub trait Tr{n} {{ fn run(&self) -> u32; }}\n\nimpl Tr{n} for Item{n} {{ fn run(&self) -> u32 {{ self.{} }} }}\n",
        ident(rng),
        ident(rng)
    );
    for m in 0..8 {
        out.push_str(&format!(
            "// {} {}\npub fn {}_{m}(a: u32, b: &str) -> u32 {{\n    let {} = a + {};\n    println!(\"{{}} {{}}\", {}, b);\n    {}\n}}\n",
            if rng.next().is_multiple_of(19) { "FIXME" } else { "doc" },
            ident(rng),
            ident(rng),
            ident(rng),
            rng.next() % 1000,
            ident(rng),
            ident(rng),
        ));
    }
    out
}

fn text(rng: &mut Rng) -> String {
    let mut out = String::new();
    for _ in 0..30 {
        out.push_str(&format!(
            "{} {} {} {}\n",
            ident(rng),
            ident(rng),
            ident(rng),
            ident(rng)
        ));
    }
    out
}

fn generate(root: &Path, files: usize) {
    let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
    for n in 0..files {
        let dir = root.join(format!("module{}/src/pkg{}", n % 40, n % 13));
        std::fs::create_dir_all(&dir).expect("dir");
        let (name, body) = match n % 10 {
            0..=5 => (format!("Service{n}.java"), java(n, &mut rng)),
            6..=8 => (format!("item{n}.rs"), rust(n, &mut rng)),
            _ => (format!("notes{n}.txt"), text(&mut rng)),
        };
        std::fs::write(dir.join(name), body).expect("write");
    }
}

fn rss_mib() -> f64 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|status| {
            status
                .lines()
                .find(|l| l.starts_with("VmRSS:"))
                .and_then(|l| l.split_whitespace().nth(1)?.parse::<f64>().ok())
        })
        .map_or(0.0, |kib| kib / 1024.0)
}

fn dir_size(path: &Path) -> u64 {
    walkdir::WalkDir::new(path)
        .into_iter()
        .filter_map(Result::ok)
        .filter_map(|e| e.metadata().ok())
        .filter(|m| m.is_file())
        .map(|m| m.len())
        .sum()
}

struct Run {
    names_ready: Duration,
    /// Every index but the text-only Trigram is ready (navigation works).
    navigation_ready: Duration,
    all_ready: Duration,
    rss_mib: f64,
}

fn run(project: &Path, cache: &Path) -> (Run, AnalysisHost, IndexService) {
    let mut host = AnalysisHost::new(
        HostConfig {
            threads: 3,
            ..HostConfig::default()
        },
        Arc::new(DiskVfs),
        default_registry(),
    );
    let cfg = IndexConfig {
        store_dir: Some(cache.to_path_buf()),
        ..IndexConfig::default()
    };
    let mut svc = IndexService::new(cfg);
    let started = Instant::now();
    svc.attach(&mut host, project);
    let mut names_ready = None;
    let mut navigation_ready = None;
    loop {
        host.poll();
        svc.poll(&mut host);
        let snapshot = host.snapshot();
        if names_ready.is_none() && snapshot.index_state(ids::FILE_NAME) == IndexState::Ready {
            names_ready = Some(started.elapsed());
        }
        if navigation_ready.is_none() && snapshot.index_state(ids::TODO) == IndexState::Ready {
            navigation_ready = Some(started.elapsed());
        }
        drop(snapshot);
        if svc.is_idle() {
            break;
        }
        if started.elapsed() > Duration::from_secs(600) {
            panic!("indexing did not finish");
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    let all_ready = started.elapsed();
    (
        Run {
            names_ready: names_ready.unwrap_or(all_ready),
            navigation_ready: navigation_ready.unwrap_or(all_ready),
            all_ready,
            rss_mib: rss_mib(),
        },
        host,
        svc,
    )
}

fn time<T>(f: impl FnOnce() -> T) -> (T, Duration) {
    let started = Instant::now();
    let value = f();
    (value, started.elapsed())
}

fn main() {
    let files: usize = std::env::var("ARCHIPEL_BENCH_FILES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(20_000);
    let assert_targets = std::env::var("ARCHIPEL_PERF_ASSERT").is_ok();
    let base = std::env::var("ARCHIPEL_BENCH_DIR")
        .map_or_else(|_| std::env::temp_dir(), std::path::PathBuf::from);
    let tmp = tempfile::tempdir_in(base).expect("tmp");
    let project = tmp.path().join("project");
    let cache = tmp.path().join("cache");
    let (_, generate_time) = time(|| generate(&project, files));
    let bytes = dir_size(&project);
    println!(
        "tree: {files} files, {:.1} MiB, generated in {generate_time:.2?}",
        bytes as f64 / 1_048_576.0
    );
    let rss_start = rss_mib();

    let (cold, host, svc) = run(&project, &cache);
    let stats = svc.stats();
    println!(
        "cold: FileName ready {:.2?}, stubs+words+todo ready {:.2?}, trigram ready {:.2?} ({} files indexed, {} skipped), RSS {:.0} MiB (start {:.0} MiB)",
        cold.names_ready,
        cold.navigation_ready,
        cold.all_ready,
        stats.indexed,
        svc.status().skipped_total,
        cold.rss_mib,
        rss_start
    );
    let snap = host.snapshot();
    let ((hits, _), go_to_file) = time(|| {
        (
            readers::files_matching(&snap, &|n| n.contains("service1234")).map(|h| h.len()),
            (),
        )
    });
    println!("Go to File query: {:?} hit(s) in {go_to_file:.2?}", hits);
    let ((symbols, classes), symbols_time) = time(|| {
        let lookup = SymbolLookup::new(&snap).expect("ready");
        (lookup.all().len(), lookup.classes().len())
    });
    println!("all symbols: {symbols} ({classes} classes) in {symbols_time:.2?}");
    let (by_name, by_name_time) = time(|| {
        SymbolLookup::new(&snap)
            .expect("ready")
            .by_name("Service1234")
            .len()
    });
    println!("symbol by name: {by_name} in {by_name_time:.2?}");
    let (words, words_time) =
        time(|| readers::files_with_word(&snap, "orderTotal").map(|f| f.len()));
    println!("word lookup: {words:?} files in {words_time:.2?}");
    let (tri, tri_time) =
        time(|| readers::trigram_candidates(&snap, "sessionToken").map(|f| f.map(|f| f.len())));
    println!("trigram candidates: {tri:?} in {tri_time:.2?}");
    let (subs, subs_time) = time(|| readers::subtypes_of(&snap, "Base1").map(|f| f.len()));
    println!("subtypes: {subs:?} in {subs_time:.2?}");
    println!(
        "store size: {:.1} MiB",
        dir_size(&cache) as f64 / 1_048_576.0
    );
    drop(snap);
    drop((svc, host));

    let (warm, host, svc) = run(&project, &cache);
    println!(
        "warm restart: FileName ready {:.2?}, idle {:.2?} ({} reindexed, {} unchanged), RSS {:.0} MiB",
        warm.names_ready,
        warm.all_ready,
        svc.stats().indexed,
        svc.stats().unchanged,
        warm.rss_mib
    );
    drop((svc, host));

    if assert_targets {
        assert!(
            cold.names_ready < Duration::from_secs(1),
            "FileName within 1 s: {:?}",
            cold.names_ready
        );
        assert!(
            go_to_file < Duration::from_millis(50),
            "Go to File query: {go_to_file:?}"
        );
    }
}
