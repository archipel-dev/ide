//! Class-file reader tests: real javac output and the JDK image. JAVA-RES-034, 035,
//! 036, 038, 039, 041; JAVA-STUB-004, 007.

use archipel_ide_java::stub::*;
use archipel_ide_java::unit::{all_classes, read_class_dir, read_jdk_image};
use std::path::{Path, PathBuf};
use std::process::Command;

fn javac(dir: &Path, files: &[(&str, &str)]) -> bool {
    let mut args = vec![
        "-d".to_string(),
        dir.join("out").display().to_string(),
        "-g".into(),
    ];
    for (name, text) in files {
        let p = dir.join("src").join(name);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(&p, text).unwrap();
        args.push(p.display().to_string());
    }
    std::fs::create_dir_all(dir.join("out")).unwrap();
    Command::new("javac")
        .args(&args)
        .env_remove("JAVA_TOOL_OPTIONS")
        .status()
        .is_ok_and(|s| s.success())
}

fn jdk_modules() -> Option<PathBuf> {
    let home = std::env::var_os("JAVA_HOME")
        .map(PathBuf::from)
        .or_else(|| Some(PathBuf::from("/usr/lib/jvm/java-21-openjdk-amd64")))?;
    let p = home.join("lib/modules");
    p.exists().then_some(p)
}

const SRC: &str = r#"
package p;
import java.util.*;
@Deprecated
public abstract class Sample<T extends Comparable<T>> extends Base<T> implements Runnable, Comparable<Sample<T>> {
    public static final int MAX = 10;
    public static final String NAME = "a\"b";
    private List<String> names;
    @SuppressWarnings("x")
    public static <U> void helper(int x, List<U> list, String... rest) throws java.io.IOException {}
    public Sample(int a) {}
    class Inner { class Deep {} }
    public static class Nested<K, V> implements Map.Entry<K, V> {
        public K getKey() { return null; } public V getValue() { return null; } public V setValue(V v) { return null; }
    }
    enum Color { RED, GREEN; Color() {} }
    interface Fn { void run(); default int d() { return 1; } static void s() {} }
    record Pt(int x, List<String> ys) {}
    public int compareTo(Sample<T> o) { return 0; }
    public void run() { Runnable r = () -> {}; new Object() {}; }
}
class Base<T> {}
"#;

#[test]
fn reads_javac_output_into_stubs() {
    let dir = tempfile::tempdir().unwrap();
    if !javac(dir.path(), &[("p/Sample.java", SRC)]) {
        eprintln!("javac not available, skipping");
        return;
    }
    let stubs = read_class_dir(&dir.path().join("out"));
    let all = all_classes(&stubs);
    let get = |q: &str| {
        all.iter().find(|c| c.qname == q).unwrap_or_else(|| {
            panic!(
                "no {q}: {:?}",
                all.iter().map(|c| &c.qname).collect::<Vec<_>>()
            )
        })
    };
    let s = get("p.Sample");
    assert!(s.deprecated && s.has_flag(flags::PUBLIC) && s.has_flag(flags::ABSTRACT));
    assert_eq!(s.type_params[0].name, "T");
    assert_eq!(s.type_params[0].bounds, ["java.lang.Comparable<T>"]);
    assert_eq!(s.extends, ["p.Base<T>"]);
    assert_eq!(
        s.implements,
        ["java.lang.Runnable", "java.lang.Comparable<p.Sample<T>>"]
    );
    let max = s.fields.iter().find(|f| f.name == "MAX").unwrap();
    assert_eq!(max.init.as_deref(), Some("10"));
    assert_eq!(
        s.fields
            .iter()
            .find(|f| f.name == "NAME")
            .unwrap()
            .init
            .as_deref(),
        Some("\"a\\\"b\"")
    );
    assert_eq!(
        s.fields.iter().find(|f| f.name == "names").unwrap().ty,
        "java.util.List<java.lang.String>"
    );
    let helper = s.methods.iter().find(|m| m.name == "helper").unwrap();
    assert!(helper.varargs && helper.has_flag_static());
    assert_eq!(helper.type_params[0].name, "U");
    assert_eq!(helper.params.len(), 3);
    assert_eq!(helper.params[0].name, "x");
    assert_eq!(helper.params[2].ty, "java.lang.String[]");
    assert!(helper.params[2].varargs);
    assert_eq!(helper.throws, ["java.io.IOException"]);
    assert!(helper.annotations.is_empty() || helper.annotations == ["@java.lang.SuppressWarnings"]);
    // Constructor: not duplicated, no synthetic parameters.
    let ctors: Vec<_> = s.methods.iter().filter(|m| m.is_ctor).collect();
    assert_eq!(ctors.len(), 1);
    assert_eq!(ctors[0].params.len(), 1);
    // Bridge method compareTo(Object) is skipped; lambda$ bodies are skipped.
    assert_eq!(
        s.methods.iter().filter(|m| m.name == "compareTo").count(),
        1
    );
    assert!(s.methods.iter().all(|m| !m.name.starts_with("lambda$")));
    // Nesting from the InnerClasses table; anonymous classes skipped.
    let inner = s.inner_named("Inner").unwrap();
    assert_eq!(inner.qname, "p.Sample.Inner");
    assert_eq!(
        inner.inner_named("Deep").unwrap().qname,
        "p.Sample.Inner.Deep"
    );
    assert!(
        s.inner
            .iter()
            .all(|c| !c.name.chars().all(|ch| ch.is_ascii_digit()))
    );
    let nested = get("p.Sample.Nested");
    assert_eq!(nested.implements, ["java.util.Map.Entry<K, V>"]);
    assert!(nested.is_static_member);
    let color = get("p.Sample.Color");
    assert_eq!(color.kind, ClassKind::Enum);
    assert!(
        color
            .fields
            .iter()
            .any(|f| f.name == "RED" && f.enum_constant)
    );
    assert!(
        color
            .methods
            .iter()
            .any(|m| m.name == "values" && m.has_flag_static())
    );
    assert!(color.fields.iter().all(|f| f.name != "$VALUES"));
    // Enum constructor has no synthetic name/ordinal.
    assert!(
        color
            .methods
            .iter()
            .filter(|m| m.is_ctor)
            .all(|m| m.params.is_empty())
    );
    let fnn = get("p.Sample.Fn");
    assert_eq!(fnn.kind, ClassKind::Interface);
    assert!(fnn.methods.iter().find(|m| m.name == "d").unwrap().flags & flags::DEFAULT != 0);
    assert!(
        fnn.methods
            .iter()
            .find(|m| m.name == "s")
            .unwrap()
            .has_flag_static()
    );
    let pt = get("p.Sample.Pt");
    assert_eq!(pt.kind, ClassKind::Record);
    assert_eq!(pt.record_components.len(), 2);
    assert_eq!(
        pt.record_components[1].ty,
        "java.util.List<java.lang.String>"
    );
    // Package-private top-level class from the same file.
    assert!(all.iter().any(|c| c.qname == "p.Base"));
}

trait MethodExt {
    fn has_flag_static(&self) -> bool;
}
impl MethodExt for MethodStub {
    fn has_flag_static(&self) -> bool {
        self.flags & flags::STATIC != 0
    }
}

#[test]
fn rejects_garbage_without_panicking() {
    use archipel_ide_java::classfile::parse_class;
    assert!(parse_class(b"").is_err());
    assert!(parse_class(b"hello world").is_err());
    let mut bytes = vec![0xCA, 0xFE, 0xBA, 0xBE, 0, 0, 0, 65, 0, 200];
    bytes.extend(std::iter::repeat_n(7u8, 50));
    assert!(parse_class(&bytes).is_err());
    // Every truncation of a real class file is an error or a skip, never a panic.
    let dir = tempfile::tempdir().unwrap();
    if javac(dir.path(), &[("p/Sample.java", SRC)]) {
        let real = std::fs::read(dir.path().join("out/p/Sample.class")).unwrap();
        for cut in (0..real.len()).step_by(7) {
            let _ = parse_class(&real[..cut]);
        }
    }
}

#[test]
fn reads_the_jdk_image() {
    let Some(modules) = jdk_modules() else {
        eprintln!("no JDK image, skipping");
        return;
    };
    let stubs = read_jdk_image(&modules).unwrap();
    let all = all_classes(&stubs);
    assert!(all.len() > 5000, "{}", all.len());
    let get = |q: &str| {
        all.iter()
            .find(|c| c.qname == q)
            .unwrap_or_else(|| panic!("no {q}"))
    };
    let list = get("java.util.List");
    assert_eq!(list.kind, ClassKind::Interface);
    assert!(
        list.methods
            .iter()
            .any(|m| m.name == "of" && m.flags & flags::STATIC != 0)
    );
    let entry = get("java.util.Map.Entry");
    assert_eq!(entry.kind, ClassKind::Interface);
    let optional = get("java.util.Optional");
    assert!(
        optional
            .methods
            .iter()
            .any(|m| m.name == "orElse" && m.params[0].ty == "T")
    );
    assert!(all.iter().any(|c| c.qname == "java.lang.ProcessHandle"));
    assert!(all.iter().any(|c| c.qname == "java.awt.List"));
    // Non-exported internals are not part of the API surface.
    assert!(all.iter().all(|c| !c.qname.starts_with("jdk.internal.")));
    // Parameter names come from debug info or are generated.
    let max = get("java.lang.Math")
        .methods
        .iter()
        .find(|m| m.name == "max" && m.params[0].ty == "int")
        .unwrap();
    assert!(max.params.iter().all(|p| !p.name.is_empty()));
    // Strings are decoded.
    assert_eq!(
        get("java.lang.Integer")
            .fields
            .iter()
            .find(|f| f.name == "MAX_VALUE")
            .unwrap()
            .init
            .as_deref(),
        Some("2147483647")
    );
}

#[test]
fn reads_assertj_from_a_jar() {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default();
    let dir = home.join(".m2/repository/org/assertj/assertj-core");
    let Some(jar) = std::fs::read_dir(&dir).ok().and_then(|d| {
        d.filter_map(Result::ok).find_map(|v| {
            let p = v.path();
            let n = p.file_name()?.to_string_lossy().into_owned();
            let j = p.join(format!("assertj-core-{n}.jar"));
            j.exists().then_some(j)
        })
    }) else {
        eprintln!("no assertj jar, skipping");
        return;
    };
    let stubs = archipel_ide_java::unit::read_jar(&jar, 21).unwrap();
    let all = all_classes(&stubs);
    let assertions = all
        .iter()
        .find(|c| c.qname == "org.assertj.core.api.Assertions")
        .unwrap();
    let at: Vec<_> = assertions
        .methods
        .iter()
        .filter(|m| m.name == "assertThat")
        .collect();
    assert!(at.len() > 20);
    assert!(at.iter().all(|m| m.flags & flags::STATIC != 0));
    let with = all
        .iter()
        .find(|c| c.qname == "org.assertj.core.api.WithAssertions")
        .unwrap();
    assert_eq!(with.kind, ClassKind::Interface);
    assert!(
        with.methods
            .iter()
            .filter(|m| m.name == "assertThat")
            .all(|m| m.flags & flags::DEFAULT != 0)
    );
}
