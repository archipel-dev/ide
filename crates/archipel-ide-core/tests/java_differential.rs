//! Differential check of the native Java resolve errors against jdtls on the same project
//! (spec 1.10, `docs/analysis/java-differential.md`). Ignored: needs jdtls (toolchain store), Gradle
//! and warm caches. `cargo test -p archipel-ide-core --test java_differential -- --ignored --nocapture`
//! prints one line per disagreement and a summary; the test fails when native reports an
//! unresolved reference jdtls does not (a false positive).

use archipel_ide_core::{IdeState, built_in_commands};
use serde_json::json;
use std::collections::BTreeSet;
use std::path::Path;
use std::time::{Duration, Instant};

const FILES: &[(&str, &str)] = &[
    ("settings.gradle", "rootProject.name = 'diff'\n"),
    (
        "build.gradle",
        "plugins {\n    id 'java'\n}\n\nrepositories {\n    mavenCentral()\n}\n\ndependencies {\n    testImplementation 'org.junit.jupiter:junit-jupiter:5.11.0'\n    testRuntimeOnly 'org.junit.platform:junit-platform-launcher:1.11.0'\n    testImplementation 'org.assertj:assertj-core:3.26.3'\n}\n\ntest {\n    useJUnitPlatform()\n}\n",
    ),
    (
        "src/main/java/demo/Valid.java",
        r#"package demo;

import java.util.*;
import java.util.function.Function;
import static java.lang.Math.max;
import static java.util.Collections.*;

public class Valid<T extends Comparable<T>> implements Runnable {
    private final List<String> names = new ArrayList<>();
    static int counter;
    enum Color { RED, GREEN; Color next() { return values()[(ordinal() + 1) % values().length]; } }
    record Pt(int x, int y) { int sum() { return x + y; } }
    interface Shape { double area(); default String name() { return "s"; } }
    class Inner { int v = counter; void g() { run(); names.size(); } }

    public void run() {
        int a = max(1, 2);
        List<String> e = emptyList();
        for (String n : names) { System.out.println(n.length()); }
        for (int i = 0; i < 3; i++) { a += i; }
        try (var sc = new Scanner(System.in)) { sc.nextLine(); } catch (RuntimeException | Error ex) { ex.printStackTrace(); }
        Function<Integer, Integer> f = x -> x + a2();
        Object o = "x";
        if (o instanceof String s && s.isEmpty()) { s.length(); }
        switch (o) { case Integer n when n > 1 -> n++; case String t -> t.length(); default -> {} }
        Color c = Color.RED; c.next();
        Pt p = new Pt(1, 2); p.x(); p.sum();
        Shape sh = new Shape() { public double area() { return 1; } };
        sh.name(); Math.abs(-1); String.valueOf(1).length();
        var list = new ArrayList<Map.Entry<String, Integer>>();
        list.forEach(en -> en.getKey().length());
        int[] arr = {1, 2}; arr.clone(); int len = arr.length;
        T t = null; this.names.add("a"); Valid.this.run();
        class Local { int q; }
        new Local().q++;
        java.util.Optional.of(1).ifPresent(System.out::println);
    }
    int a2() { return 1; }
}
"#,
    ),
    (
        "src/main/java/demo/Broken.java",
        r#"package demo;

import java.util.List;
import java.utl.Missing;
import zork.nothere.Foo;

public class Broken {
    Strng s;
    List<Nope> l;
    void f(String s) {
        Collections.emptyList();
        java.util.Lisst x;
        Object o = new Unknown(1);
        int y = zzz + 1;
        foo();
        foo(1, "a", y);
        s.lenght();
        Math.foo(1);
        this.nothing();
        undefinedField.call();
        s.length();
    }
}
"#,
    ),
    (
        "src/test/java/demo/CalcTest.java",
        r#"package demo;

import org.junit.jupiter.api.Test;

public class CalcTest {
    @Test
    public void adds() {
        String x = "a";
        assertThat(x).isEqualTo("a");
        assertThat(1).isEqualTo(1);
        assertEquals(1, 1);
    }
}
"#,
    ),
];

fn is_resolve_message(m: &str) -> bool {
    m.contains("cannot be resolved")
        || m.contains("is undefined for the type")
        || m.starts_with("Cannot resolve")
}

#[test]
#[ignore = "needs jdtls, Gradle and warm caches"]
fn native_resolve_errors_agree_with_jdtls() {
    let root = std::env::temp_dir().join(format!("archipel-java-diff-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    for (p, t) in FILES {
        let target = root.join(p);
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::write(target, t).unwrap();
    }
    let root = std::fs::canonicalize(&root).unwrap();
    let commands = built_in_commands();
    let mut state = IdeState::default();
    commands
        .execute("project.open", &mut state, json!({"path": root}))
        .unwrap();
    let java: Vec<&str> = FILES
        .iter()
        .map(|f| f.0)
        .filter(|p| p.ends_with(".java"))
        .collect();
    for f in &java {
        commands
            .execute("editor.openFile", &mut state, json!({"path": f}))
            .unwrap();
    }
    let deadline = Instant::now() + Duration::from_secs(600);
    let report: Vec<String>;
    let false_positives: i32;
    loop {
        state.poll_project_model();
        state.poll_lsp();
        state.poll_analysis();
        std::thread::sleep(Duration::from_millis(100));
        let service = state.daemon_service().unwrap();
        let mut all_ready = true;
        let mut native_total = 0;
        let mut lsp_total = 0;
        let mut rows: Vec<String> = Vec::new();
        let mut fp = 0;
        for f in &java {
            let Some(file) = state.analysis.host.lookup(Path::new(f)) else {
                all_ready = false;
                continue;
            };
            let daemon = service.lock();
            let Some(markup) = daemon.markup(file) else {
                all_ready = false;
                continue;
            };
            let text = std::fs::read_to_string(root.join(f)).unwrap();
            let line_of = |off: u32| text[..off as usize].matches('\n').count() + 1;
            let mut native: BTreeSet<(usize, String)> = BTreeSet::new();
            let mut lsp: BTreeSet<(usize, String)> = BTreeSet::new();
            let all = markup
                .iter()
                .map(|i| (i, false))
                .chain(markup.shadowed().map(|(t, i)| (i, t.is_lsp() && false)));
            let _ = all;
            let mut entries: Vec<(
                &archipel_ide_analysis::ToolId,
                &archipel_ide_analysis::HighlightInfo,
            )> = markup.shadowed().collect();
            entries.extend(markup.iter().map(|i| (&i.tool, i)));
            for (tool, info) in entries {
                if info.severity < archipel_ide_analysis::Severity::ERROR {
                    continue;
                }
                let msg = info.message.as_deref().unwrap_or("");
                let key = (
                    line_of(info.range.start),
                    text[info.range.start as usize..info.range.end as usize].to_string(),
                );
                if tool.as_str() == "native:java-resolve" {
                    native.insert(key);
                } else if tool.is_lsp() && is_resolve_message(msg) {
                    lsp.insert(key);
                }
            }
            native_total += native.len();
            lsp_total += lsp.len();
            if lsp.is_empty() && native.is_empty() && f.contains("Broken") {
                all_ready = false;
            }
            // jdtls underlines a whole qualified name where native marks the first unknown
            // segment: two errors agree when they are on one line and one range contains the other.
            let same = |a: &(usize, String), b: &(usize, String)| {
                a.0 == b.0 && (a.1.contains(&b.1) || b.1.contains(&a.1))
            };
            for k in native.iter().filter(|k| !lsp.iter().any(|l| same(k, l))) {
                rows.push(format!("native-only {f}:{} {:?}", k.0, k.1));
                fp += 1;
            }
            for k in lsp.iter().filter(|k| !native.iter().any(|n| same(k, n))) {
                rows.push(format!("jdtls-only  {f}:{} {:?}", k.0, k.1));
            }
        }
        if all_ready
            && native_total > 0
            && lsp_total > 0
            && state.lsp.servers().iter().all(|s| s.progress.is_empty())
        {
            report = rows;
            false_positives = fp;
            eprintln!("native {native_total}, jdtls {lsp_total}");
            break;
        }
        if Instant::now() > deadline {
            panic!("timed out; native {native_total}, jdtls {lsp_total}");
        }
    }
    for r in &report {
        eprintln!("{r}");
    }
    assert_eq!(
        false_positives, 0,
        "native reported references jdtls resolves"
    );
}
