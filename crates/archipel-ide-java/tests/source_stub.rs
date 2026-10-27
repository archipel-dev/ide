//! Source stubs: JAVA-STUB-001..007, 010, 019-021 (scenarios ST1 and ST2 of spec 01).

use archipel_ide_java::source::build_file_stub;
use archipel_ide_java::stub::*;

const ST1: &str = r#"package p;

import java.util.List;
import static org.junit.Assert.*;

@Deprecated
public abstract class Sample<T extends Comparable<T>> extends Base<T> implements Runnable, Comparable<Sample<T>> {
    public static final int MAX = 10;
    private List<String> names;
    @org.junit.Test
    public static <U> void helper(int x, List<U> list, String... rest) throws java.io.IOException { /* body */ }
    class Inner {}
}
"#;

#[test]
fn st1_outline() {
    // JAVA-STUB-005, 006, 007, 010
    let f = build_file_stub(ST1);
    assert_eq!(f.package, "p");
    assert_eq!(
        f.imports,
        [
            ImportStub {
                name: "java.util.List".into(),
                on_demand: false,
                is_static: false
            },
            ImportStub {
                name: "org.junit.Assert".into(),
                on_demand: true,
                is_static: true
            },
        ]
    );
    let c = &f.classes[0];
    assert_eq!(c.qname, "p.Sample");
    assert!(c.deprecated);
    assert_eq!(c.flags, flags::PUBLIC | flags::ABSTRACT);
    assert_eq!(
        c.type_params,
        [TypeParamStub {
            name: "T".into(),
            bounds: vec!["Comparable<T>".into()]
        }]
    );
    assert_eq!(c.extends, ["Base<T>"]);
    assert_eq!(c.implements, ["Runnable", "Comparable<Sample<T>>"]);
    assert_eq!(c.fields[0].name, "MAX");
    assert_eq!(c.fields[0].init.as_deref(), Some("10"));
    assert_eq!(c.fields[1].ty, "List<String>");
    assert_eq!(c.fields[1].init, None);
    let m = &c.methods[0];
    assert_eq!(m.name, "helper");
    assert!(m.varargs);
    assert_eq!(m.ret.as_deref(), Some("void"));
    assert_eq!(
        m.params
            .iter()
            .map(|p| (p.name.as_str(), p.ty.as_str()))
            .collect::<Vec<_>>(),
        [("x", "int"), ("list", "List<U>"), ("rest", "String[]")]
    );
    assert_eq!(m.throws, ["java.io.IOException"]);
    assert_eq!(m.annotations, ["@org.junit.Test"]);
    assert_eq!(c.inner[0].qname, "p.Sample.Inner");
}

#[test]
fn st2_body_edits_keep_the_stub() {
    // JAVA-STUB-003
    let a = build_file_stub(ST1);
    let b = build_file_stub(&ST1.replace("/* body */", "{ System.out.println(1); int q = 5; }"));
    assert_eq!(a.encode(), b.encode());
    let c = build_file_stub(&ST1.replace("/* body */", "{ Runnable r = () -> {}; }"));
    // Lambda stubs are LATER (JAVA-STUB-009); names are unchanged either way.
    assert_eq!(a.classes[0].methods, c.classes[0].methods);
    // A whitespace-only edit does not change the serialized stub either (PLAT-IDX-052).
    let d = build_file_stub(&ST1.replace(
        "public static final int MAX = 10;",
        "public   static final   int  MAX  =  10 ;",
    ));
    assert_eq!(a.encode(), d.encode());
}

#[test]
fn kinds_members_and_javadoc() {
    let f = build_file_stub(
        r#"package q;
public interface I extends J, K {
    int C = 1;
    void f();
    default void g() {}
    static int s() { return 1; }
    class N {}
}
enum E { A(1) { }, B; E(int x) {} static E of() { return A; } }
record R(int x, String... y) { R {} }
@interface An { int v() default 1; String[] names(); }
/** @deprecated use other */
class Old { /** @deprecated */ int a, b[]; @Deprecated void m() {} }
"#,
    );
    let i = &f.classes[0];
    assert_eq!(i.kind, ClassKind::Interface);
    assert_eq!(i.extends, ["J", "K"]);
    assert_eq!(
        i.fields[0].flags,
        flags::PUBLIC | flags::STATIC | flags::FINAL
    );
    assert!(i.methods[0].flags & flags::ABSTRACT != 0);
    assert!(i.methods[1].flags & flags::DEFAULT != 0);
    assert!(i.inner[0].is_static_member);
    let e = &f.classes[1];
    assert_eq!(e.kind, ClassKind::Enum);
    assert_eq!(e.fields.iter().filter(|f| f.enum_constant).count(), 2);
    assert_eq!(e.methods.iter().filter(|m| m.is_ctor).count(), 1);
    let r = &f.classes[2];
    assert_eq!(r.kind, ClassKind::Record);
    assert_eq!(r.record_components.len(), 2);
    assert!(r.record_components[1].varargs);
    let an = &f.classes[3];
    assert_eq!(an.kind, ClassKind::Annotation);
    assert!(an.methods[0].has_default && !an.methods[1].has_default);
    let old = &f.classes[4];
    assert!(old.deprecated);
    assert_eq!(old.fields.len(), 2);
    assert!(old.fields[0].deprecated);
    assert_eq!(old.fields[1].ty, "int[]");
    assert!(old.methods[0].deprecated);
}

#[test]
fn broken_input_never_panics() {
    for text in [
        "",
        "class",
        "package;",
        "class A { void m( { }",
        "import ;",
        "\u{0}\u{1}",
        "class A extends { int }",
    ] {
        let _ = build_file_stub(text);
    }
}
