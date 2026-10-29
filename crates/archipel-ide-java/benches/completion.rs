//! Index and completion benchmarks of the native Java engine (release), the budgets of
//! `docs/architecture/java-engine-plan.md` section 3.5.
//!
//! `cargo bench -p archipel-ide-java --bench completion`. With `ARCHIPEL_PERF_ASSERT=1` it exits
//! non-zero when a budget is exceeded. The numbers are recorded in `docs/perf/java-engine.md`.
//! Fixture: the JDK of this machine, our hermetic JUnit-like jar, and generated sources of
//! about 180k classes in 600 files (so the name table has the size the plan budgets for).

#[path = "../tests/common/mod.rs"]
mod common;

use archipel_ide_java::api::JavaProject;
use archipel_ide_java::complete::{self, CompletionInput};
use archipel_ide_java::import_edit::ImportSettings;
use archipel_ide_java::index::JavaIndex;
use archipel_ide_projectmodel::{
    DependencyScope, JpsModuleSourceRootType, Library, Module, OrderEntry, ProjectModel, Sdk,
    SourceFolder,
};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

const FILES: usize = 600;
const CLASSES_PER_FILE: usize = 300;

struct Stats {
    name: String,
    samples: Vec<Duration>,
}

impl Stats {
    fn percentile(&self, p: f64) -> Duration {
        let mut sorted = self.samples.clone();
        sorted.sort();
        sorted[((sorted.len() - 1) as f64 * p) as usize]
    }
}

fn report(
    stats: &Stats,
    p95_budget_ms: f64,
    p99_budget_ms: Option<f64>,
    failures: &mut Vec<String>,
) {
    let (p50, p95, p99) = (
        stats.percentile(0.5),
        stats.percentile(0.95),
        stats.percentile(0.99),
    );
    println!(
        "{:<52} p50 {:>8.3} ms  p95 {:>8.3} ms  p99 {:>8.3} ms  (budget p95 < {p95_budget_ms} ms)",
        stats.name,
        p50.as_secs_f64() * 1e3,
        p95.as_secs_f64() * 1e3,
        p99.as_secs_f64() * 1e3,
    );
    if p95.as_secs_f64() * 1e3 > p95_budget_ms {
        failures.push(format!(
            "{}: p95 {:.3} ms over {p95_budget_ms} ms",
            stats.name,
            p95.as_secs_f64() * 1e3
        ));
    }
    if let Some(b) = p99_budget_ms
        && p99.as_secs_f64() * 1e3 > b
    {
        failures.push(format!(
            "{}: p99 {:.3} ms over {b} ms",
            stats.name,
            p99.as_secs_f64() * 1e3
        ));
    }
}

fn once(name: &str, budget_s: f64, took: Duration, failures: &mut Vec<String>) {
    println!(
        "{name:<52} {:>8.3} s   (budget < {budget_s} s)",
        took.as_secs_f64()
    );
    if took.as_secs_f64() > budget_s {
        failures.push(format!(
            "{name}: {:.3} s over {budget_s} s",
            took.as_secs_f64()
        ));
    }
}

fn measure(name: impl Into<String>, runs: usize, mut f: impl FnMut(usize)) -> Stats {
    for i in 0..3 {
        f(i);
    }
    let mut samples = Vec::with_capacity(runs);
    for i in 0..runs {
        let start = Instant::now();
        f(i);
        samples.push(start.elapsed());
    }
    Stats {
        name: name.into(),
        samples,
    }
}

fn jdk() -> PathBuf {
    archipel_ide_java::classpath::detect_jdk_home().expect("a JDK is needed for the benchmark")
}

fn model(root: &Path, with_libs: bool) -> ProjectModel {
    let mut model = ProjectModel::new("bench", root);
    let mut app = Module::new("app");
    app.content_entry_mut(root)
        .add_source_folder(SourceFolder::new(
            root.join("src/main/java"),
            JpsModuleSourceRootType::Source,
        ))
        .add_source_folder(SourceFolder::new(
            root.join("src/test/java"),
            JpsModuleSourceRootType::TestSource,
        ));
    if with_libs {
        model.put_library(Library::new("junit5").with_classes(common::fixture_jar("junit5")));
        app.add_order_entry(OrderEntry::library("junit5", DependencyScope::Test));
    }
    model.modules.push(app);
    model.put_sdk(Sdk::jdk_at(jdk(), None));
    model.project_sdk = Some(model.sdks[0].name.clone());
    model
}

fn write_sources(root: &Path) {
    for f in 0..FILES {
        let mut text = format!("package gen.p{};\n\n", f % 60);
        for c in 0..CLASSES_PER_FILE {
            let n = f * CLASSES_PER_FILE + c;
            let kind = match n % 7 {
                0 => "public interface",
                1 => "public @interface",
                2 => "public enum",
                _ => "public class",
            };
            text.push_str(&format!("{kind} Svc{n:06}Handler {{}}\n"));
        }
        let path = root.join(format!("src/main/java/gen/p{}/File{f}.java", f % 60));
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        // Several classes per file: package-private is legal, public is enough for the index.
        std::fs::write(path, text.replace("public ", "")).unwrap();
    }
}

/// A source file of about `methods * 5` lines: generic collections, streams, calls on library
/// receivers, and a `tail` method at the end where the caret goes.
fn big_source(methods: usize) -> String {
    let mut t = String::from(
        "package p;\n\nimport java.util.*;\nimport java.util.stream.*;\n\nclass Big {\n    private final List<String> names = new ArrayList<>();\n",
    );
    for i in 0..methods {
        t.push_str(&format!(
            "    int m{i}(int a) {{\n        String s = \"x{i}\";\n        List<String> l = new ArrayList<>();\n        l.add(s);\n        return a + l.size() + names.size();\n    }}\n"
        ));
    }
    t.push_str(
        "    void tail() {\n        List<String> xs = new ArrayList<>();\n        names.add(\"a\");\n        xs.stream().filter(x -> x.isEmpty()).count();\n        xs.get(0).length();\n        List<String> more = new ArrayList<>();\n        more = null;\n        String name = \"\";\n        na;\n        re;\n        ma;\n        List<String> target = new Arr;\n    }\n}\n",
    );
    t
}

fn main() {
    let assert = std::env::var("ARCHIPEL_PERF_ASSERT").is_ok();
    let mut failures: Vec<String> = Vec::new();
    let tmp = Path::new(env!("CARGO_TARGET_TMPDIR")).join("java-bench");
    let _ = std::fs::remove_dir_all(&tmp);
    let root = tmp.join("project");
    std::fs::create_dir_all(root.join("src/test/java/p")).unwrap();
    std::fs::write(
        root.join("src/test/java/p/AppTest.java"),
        "package p;\n\nclass AppTest {\n    @Te\n    void greets() {\n        Object o = new ArrayLi\n    }\n}\n",
    )
    .unwrap();
    write_sources(&root);

    // Cold index of the JDK image alone (budget: < 8 s for JDK 21 exported packages).
    {
        let jdk_only = tmp.join("jdk-only");
        std::fs::create_dir_all(jdk_only.join("src/main/java")).unwrap();
        let m = Arc::new(model(&jdk_only, false));
        let index = JavaIndex::in_memory();
        let start = Instant::now();
        index.sync_blocking(m);
        once(
            "cold index: JDK image (exported packages)",
            8.0,
            start.elapsed(),
            &mut failures,
        );
    }

    // Cold index of the JDK, the library jar and the generated sources (180k classes).
    let m = Arc::new(model(&root, true));
    let store = tmp.join("store");
    let index = JavaIndex::open(Some(&store));
    let start = Instant::now();
    index.sync_blocking(m.clone());
    once(
        "cold index: JDK + jar + 180k generated classes",
        20.0,
        start.elapsed(),
        &mut failures,
    );

    // Warm start: reopen the persisted index and sync the unchanged model.
    drop(index);
    let index = JavaIndex::open(Some(&store));
    let start = Instant::now();
    index.sync_blocking(m.clone());
    once(
        "warm start to Ready (persisted, unchanged)",
        0.5,
        start.elapsed(),
        &mut failures,
    );

    let project = JavaProject::new(index.clone(), m.clone(), false);
    let test = root.join("src/test/java/p/AppTest.java");
    let scope = project.scope_for(&test);
    let table = index
        .names_matching(&scope, "", &|_| true, 0)
        .unwrap()
        .len();
    println!("name table: {table} classes in the scope of the test file");

    // Class-name query over the table.
    let patterns = [
        "Svc1", "Svc12", "Svc00", "ArLi", "Te", "Str", "SvcH", "Han", "Hs", "NPE",
    ];
    if std::env::var("ARCHIPEL_BENCH_VERBOSE").is_ok() {
        for pattern in patterns {
            let q = measure(format!("  pattern `{pattern}`"), 50, |_| {
                let r = index
                    .names_matching(&scope, pattern, &|_| true, 300)
                    .unwrap();
                std::hint::black_box(r);
            });
            println!(
                "{:<30} p50 {:>7.2} ms p95 {:>7.2} ms",
                q.name,
                q.percentile(0.5).as_secs_f64() * 1e3,
                q.percentile(0.95).as_secs_f64() * 1e3
            );
        }
    }
    let q = measure(format!("class-name query over {table} names"), 200, |i| {
        let r = index
            .names_matching(&scope, patterns[i % patterns.len()], &|_| true, 300)
            .unwrap();
        std::hint::black_box(r);
    });
    report(&q, 10.0, None, &mut failures);

    // Completion at a caret (position classifier, class names, ranking), p95 < 50 ms.
    let settings = ImportSettings::default();
    let text = std::fs::read_to_string(&test).unwrap();
    let carets = [
        text.find("@Te").unwrap() + 3,
        text.find("new ArrayLi").unwrap() + "new ArrayLi".len(),
    ];
    let c = measure("completion: @Te and new ArrayLi (class names)", 200, |i| {
        let input = CompletionInput {
            project: &project,
            settings: &settings,
            path: &test,
            text: &text,
            offset: carets[i % 2],
            count: 1,
        };
        let r = complete::complete(&input).unwrap().unwrap();
        assert!(!r.items.is_empty());
        std::hint::black_box(r);
    });
    report(&c, 50.0, Some(100.0), &mut failures);

    let big = "package p;\n\nclass AppTest {\n    private Svc00\n}\n";
    let c = measure(
        "completion: type position, 180k generated names",
        200,
        |_| {
            let input = CompletionInput {
                project: &project,
                settings: &settings,
                path: &test,
                text: big,
                offset: big.find("Svc00").unwrap() + 5,
                count: 1,
            };
            std::hint::black_box(complete::complete(&input).unwrap());
        },
    );
    report(&c, 50.0, Some(100.0), &mut failures);

    // Member access, reference variants, keywords, statics and the expected type in a file of
    // about 2000 lines (JAVA-CMP-008, 011, 022, 027): the probe parses the buffer, builds its
    // stub, walks the scope; p95 < 50 ms.
    let big_text = big_source(400);
    let spots: Vec<(&str, usize, u32)> = [
        ("member access: names.", "names.", 1u32),
        ("member access: chain stream().", "xs.stream().", 1),
        ("member access: String after get(0).", "xs.get(0).", 1),
        ("variants: unqualified identifier", "    na", 1),
        ("keywords: statement start", "    re", 1),
        ("statics, second press: ma", "    ma", 2),
        ("expected type: new Arr", "new Arr", 1),
    ]
    .into_iter()
    .map(|(n, marker, count)| {
        // The last occurrence is the one in `tail()`.
        let at = big_text.rfind(marker).expect("marker") + marker.len();
        (n, at, count)
    })
    .collect();
    for (name, offset, count) in spots {
        let c = measure(format!("completion: {name}"), 100, |_| {
            let input = CompletionInput {
                project: &project,
                settings: &settings,
                path: &test,
                text: &big_text,
                offset,
                count,
            };
            let r = complete::complete(&input).unwrap();
            assert!(r.is_some_and(|r| !r.items.is_empty()), "{name}");
        });
        report(&c, 50.0, Some(100.0), &mut failures);
    }

    // A receiver with a deep hierarchy and about a thousand members (Swing), prefix typed.
    let swing = "package p;\n\nclass Swing {\n    void f(javax.swing.JFrame frame) {\n        frame.set;\n    }\n}\n";
    let offset = swing.find("frame.set").unwrap() + "frame.set".len();
    let c = measure(
        "completion: member access, JFrame (1000+ members)",
        100,
        |_| {
            let input = CompletionInput {
                project: &project,
                settings: &settings,
                path: &test,
                text: swing,
                offset,
                count: 1,
            };
            let r = complete::complete(&input).unwrap();
            assert!(r.is_some_and(|r| !r.items.is_empty()));
        },
    );
    report(&c, 50.0, Some(100.0), &mut failures);

    // Inserting: the import edit of the chosen row.
    let i = measure("insertion: import edit of the chosen class", 200, |_| {
        let input = CompletionInput {
            project: &project,
            settings: &settings,
            path: &test,
            text: &text,
            offset: carets[1],
            count: 1,
        };
        std::hint::black_box(complete::insertion(&input, "java.util.ArrayList").unwrap());
    });
    report(&i, 50.0, None, &mut failures);

    // One changed jar, 100 changed source files.
    {
        let mut with_more = (*m).clone();
        with_more
            .put_library(Library::new("hamcrest").with_classes(common::fixture_jar("hamcrest")));
        with_more.modules[0]
            .add_order_entry(OrderEntry::library("hamcrest", DependencyScope::Compile));
        let start = Instant::now();
        index.sync_blocking(Arc::new(with_more));
        once("add one jar", 1.0, start.elapsed(), &mut failures);
        let start = Instant::now();
        for f in 0..100 {
            let path = root.join(format!("src/main/java/gen/p{}/File{f}.java", f % 60));
            let mut t = std::fs::read_to_string(&path).unwrap();
            t.push_str(&format!("class Extra{f} {{}}\n"));
            index.update_source(&path, &t, Some(("app", false)));
        }
        once(
            "100 changed source files",
            2.0,
            start.elapsed(),
            &mut failures,
        );
    }

    if !failures.is_empty() {
        println!("\nbudgets exceeded:");
        for f in &failures {
            println!("  {f}");
        }
        if assert {
            std::process::exit(1);
        }
    }
}
