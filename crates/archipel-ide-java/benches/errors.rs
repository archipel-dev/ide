//! Keystroke-to-markup benchmark of the native Java error pass (plan 3.5, M2): the time the
//! `java-resolve` pass needs for a 2000-line file after an edit, warm, the number the daemon's
//! 100 ms debounce is added to (budget: pass p95 < 150 ms).
//!
//! `cargo bench -p archipel-ide-java --bench errors`; `ARCHIPEL_PERF_ASSERT=1` exits non-zero when
//! the budget is exceeded. Recorded in `docs/perf/java-engine.md`.

#[path = "../tests/common/mod.rs"]
mod common;

use archipel_ide_java::diagnostics::{JavaSettings, collect};
use archipel_ide_java::source;
use std::time::{Duration, Instant};

/// A realistic file of about `lines` lines: fields, generics, streams, lambdas, inner classes,
/// static imports, library calls and a few calls to project methods.
fn generate(lines: usize) -> String {
    let mut s = String::from(
        "package demo;\n\nimport static java.util.stream.Collectors.toList;\n\nimport java.util.ArrayList;\nimport java.util.HashMap;\nimport java.util.List;\nimport java.util.Map;\nimport java.util.Optional;\nimport java.util.function.Function;\n\npublic class Big {\n    private final Map<String, List<Integer>> index = new HashMap<>();\n    private final StringBuilder log = new StringBuilder();\n\n",
    );
    let mut n = 0;
    while s.lines().count() < lines {
        s.push_str(&format!(
            "    static class Holder{n} {{\n        int value = {n};\n        int get() {{ return value; }}\n    }}\n\n    List<String> method{n}(List<String> input, Function<String, Integer> f) {{\n        List<String> out = new ArrayList<>();\n        for (String item : input) {{\n            Integer len = f.apply(item);\n            if (len != null && len > {n}) {{\n                out.add(item.trim().toUpperCase());\n            }}\n        }}\n        Holder{n} h = new Holder{n}();\n        index.computeIfAbsent(\"k{n}\", k -> new ArrayList<>()).add(h.get());\n        Optional<String> first = out.stream().filter(x -> x.length() > 1).findFirst();\n        log.append(first.orElse(\"none\")).append(out.size()).append('\\n');\n        List<String> sorted = out.stream().sorted().collect(toList());\n        return sorted.isEmpty() ? out : sorted;\n    }}\n\n"
        ));
        n += 1;
    }
    s.push_str("}\n");
    s
}

fn percentile(samples: &mut [Duration], p: f64) -> Duration {
    samples.sort();
    samples[((samples.len() - 1) as f64 * p) as usize]
}

fn main() {
    let text = generate(2000);
    let lines = text.lines().count();
    let proj = common::project(&[("src/main/java/demo/Big.java", &text)], &[]);
    let path = proj.path("src/main/java/demo/Big.java");
    let settings = JavaSettings::default();
    // Warm: JDK stubs for every class the file touches.
    let tree = source::parse(&text).unwrap();
    let infos = collect(&proj.project, &settings, &path, &text, &tree);
    assert!(
        infos.is_empty(),
        "the generated file must be clean: {infos:?}"
    );

    // One keystroke at a time: a call to a missing method is typed in the middle of the file
    // (a broken statement every other sample), then fixed.
    let at = text.find("        Holder1 h").unwrap();
    let mut cases: Vec<(&str, Vec<Duration>)> = vec![
        ("clean file, 2000 lines (parse + analysis)", Vec::new()),
        ("typing `foo()` without `;` in the middle", Vec::new()),
        ("typing `Bar b` without `;` in the middle", Vec::new()),
    ];
    let edits = [
        text.clone(),
        format!("{}        foo()\n{}", &text[..at], &text[at..]),
        format!("{}        Bar b\n{}", &text[..at], &text[at..]),
    ];
    for (i, edited) in edits.iter().enumerate() {
        for _ in 0..40 {
            let start = Instant::now();
            let tree = source::parse(edited).unwrap();
            let infos = collect(&proj.project, &settings, &path, edited, &tree);
            cases[i].1.push(start.elapsed());
            assert_eq!(infos.len(), if i == 0 { 0 } else { 1 }, "{infos:?}");
        }
    }
    // Where the time goes (clean file).
    let mut parse = Vec::new();
    let mut analysis = Vec::new();
    for _ in 0..20 {
        let start = Instant::now();
        let tree = source::parse(&text).unwrap();
        parse.push(start.elapsed());
        let start = Instant::now();
        let _ = proj.project.analyze_tree(&path, &text, &tree);
        analysis.push(start.elapsed());
    }
    println!(
        "breakdown p50: parse {:.1} ms, stub + analysis {:.1} ms",
        percentile(&mut parse, 0.5).as_secs_f64() * 1e3,
        percentile(&mut analysis, 0.5).as_secs_f64() * 1e3
    );
    let mut failed = false;
    println!("file: {lines} lines, {} bytes", text.len());
    for (name, samples) in &mut cases {
        let (p50, p95) = (percentile(samples, 0.5), percentile(samples, 0.95));
        println!(
            "{name:<48} p50 {:>8.3} ms  p95 {:>8.3} ms  (budget p95 < 150 ms)",
            p50.as_secs_f64() * 1e3,
            p95.as_secs_f64() * 1e3
        );
        failed |= p95 > Duration::from_millis(150);
    }
    if failed && std::env::var_os("ARCHIPEL_PERF_ASSERT").is_some() {
        std::process::exit(1);
    }
}
