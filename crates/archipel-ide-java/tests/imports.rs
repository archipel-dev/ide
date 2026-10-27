//! Import fixes, edits and Optimize Imports: JAVA-IMP-001..061, 070..076, scenarios S1..S9 of spec 05.

mod common;
use archipel_ide_analysis::{FileId, FileRev};
use archipel_ide_java::diagnostics::{JavaSettings, apply_command, collect_detailed, first_choice};
use archipel_ide_java::import_edit::{self, ImportSettings};
use archipel_ide_java::source;
use archipel_ide_projectmodel::DependencyScope;
use common::*;

const MAIN: &str = "src/main/java/demo/Main.java";
const TEST: &str = "src/test/java/demo/CalcTest.java";

const BUG: &str = r#"package demo;

import org.junit.Test;

public class CalcTest {
    @Test
    public void adds() {
        String x = "a";
        assertThat(x).isEqualTo("a");
    }
}
"#;

fn libs() -> Option<Vec<(&'static str, std::path::PathBuf, DependencyScope)>> {
    let aj = assertj()?;
    let j4 = find_jar("junit", "junit", None)?;
    let hc = find_jar("org/hamcrest", "hamcrest", None)?;
    Some(vec![("assertj", aj, DependencyScope::Test), ("junit", j4, DependencyScope::Test), ("hamcrest", hc, DependencyScope::Test)])
}

fn rows(c: &archipel_ide_analysis::ModCommand) -> Vec<String> {
    match c {
        archipel_ide_analysis::ModCommand::ChooseFrom { rows, .. } => rows.iter().map(|r| format!("{} {}", r.label, r.detail.clone().unwrap_or_default())).collect(),
        _ => Vec::new(),
    }
}

fn fixes_at(p: &Proj, rel: &str, text: &str) -> archipel_ide_java::diagnostics::Collected {
    let tree = source::parse(text).unwrap();
    collect_detailed(&p.project, &JavaSettings::default(), &p.path(rel), text, &tree)
}

use archipel_ide_analysis::ModCommand;

fn run_static(c: &archipel_ide_java::diagnostics::Collected, text: &str, which: usize) -> (ModCommand, ModCommand) {
    let (_, fix) = &c.static_fixes[0];
    let one = fix.apply(FileId(1), FileRev(1), text, which);
    let chooser = if fix.candidates.len() > 1 {
        ModCommand::ChooseFrom {
            title: "Method to Import".into(),
            rows: fix
                .candidates
                .iter()
                .enumerate()
                .map(|(i, cd)| archipel_ide_analysis::ChoiceRow {
                    label: cd.display(),
                    detail: Some(format!("({})", cd.package())),
                    command: fix.apply(FileId(1), FileRev(1), text, i),
                })
                .collect(),
        }
    } else {
        ModCommand::Nothing
    };
    (one, chooser)
}

#[test]
fn s1_the_assertthat_bug_end_to_end() {
    // JAVA-IMP-001, 003, 005, 010, 011, 013, 014, 020, 030, 031, S1
    let Some(libs) = libs() else { return };
    let p = project(&[], &libs);
    let c = fixes_at(&p, TEST, BUG);
    let errs: Vec<_> = c.infos.iter().filter(|i| i.message.as_deref().is_some_and(|m| m.starts_with("Cannot resolve method"))).collect();
    assert_eq!(errs.len(), 1, "{:?}", c.infos.iter().map(|i| i.message.clone()).collect::<Vec<_>>());
    let e = errs[0];
    assert_eq!(&BUG[e.range.start as usize..e.range.end as usize], "assertThat");
    assert_eq!(e.range.len(), 10);
    assert_eq!(e.message.as_deref(), Some("Cannot resolve method 'assertThat(String)'"));
    // The fix: several candidates, so the fix title ends with `...` and a popup follows.
    assert_eq!(e.fixes[0].text.as_ref(), "Import static method...");
    let (first, chooser) = run_static(&c, BUG, 0);
    assert_eq!(
        rows(&chooser),
        vec![
            "Assertions.assertThat (org.assertj.core.api)",
            "AssertionsForClassTypes.assertThat (org.assertj.core.api)",
            "AssertionsForInterfaceTypes.assertThat (org.assertj.core.api)",
            "Java6Assertions.assertThat (org.assertj.core.api)",
        ],
        "WithAssertions (default methods), Hamcrest and JUnit (other arity) are not offered; deprecated Java6Assertions is last"
    );
    let after = apply_command(&first_choice(chooser), FileId(1), BUG).unwrap();
    assert_eq!(
        after,
        r#"package demo;

import org.junit.Test;

import static org.assertj.core.api.Assertions.assertThat;

public class CalcTest {
    @Test
    public void adds() {
        String x = "a";
        assertThat(x).isEqualTo("a");
    }
}
"#
    );
    let _ = first;
    // The error is gone after the import (JAVA-IMP / S6).
    let c2 = fixes_at(&p, TEST, &after);
    assert!(c2.infos.iter().all(|i| i.severity != archipel_ide_analysis::Severity::ERROR), "{:?}", c2.infos.iter().map(|i| i.message.clone()).collect::<Vec<_>>());
}

#[test]
fn s4_a_production_file_is_not_offered_the_test_scope_import() {
    // JAVA-IMP-004, 010
    let Some(libs) = libs() else { return };
    let p = project(&[], &libs);
    let text = BUG.replace("CalcTest", "Main").replace("import org.junit.Test;\n\n", "").replace("    @Test\n", "");
    let c = fixes_at(&p, MAIN, &text);
    let errs: Vec<_> = c.infos.iter().filter(|i| i.message.as_deref() == Some("Cannot resolve method 'assertThat(String)'")).collect();
    assert_eq!(errs.len(), 1);
    assert!(errs[0].fixes.is_empty() && c.static_fixes.is_empty(), "no import items for production code");
}

#[test]
fn s2_two_arguments_offer_hamcrest_then_junit() {
    // JAVA-IMP-014, 020, S2
    let Some(libs) = libs() else { return };
    let p = project(&[], &libs);
    let text = BUG.replace("assertThat(x).isEqualTo(\"a\");", "assertThat(x, org.hamcrest.Matchers.is(\"a\"));");
    let c = fixes_at(&p, TEST, &text);
    let (_, chooser) = run_static(&c, &text, 0);
    assert_eq!(rows(&chooser), vec!["MatcherAssert.assertThat (org.hamcrest)", "Assert.assertThat (org.junit)"]);
}

#[test]
fn zero_arguments_list_every_class_and_unknown_types_are_permissive() {
    // JAVA-IMP-015, 018
    let Some(libs) = libs() else { return };
    let p = project(&[], &libs);
    let text = BUG.replace("assertThat(x).isEqualTo(\"a\");", "assertThat();");
    let c = fixes_at(&p, TEST, &text);
    let (_, ch) = run_static(&c, &text, 0);
    let r = rows(&ch);
    assert!(r.iter().any(|x| x.starts_with("Assertions.assertThat")) && r.iter().any(|x| x.starts_with("MatcherAssert.assertThat")), "{r:?}");
    let text = BUG.replace("assertThat(x).isEqualTo(\"a\");", "assertThat(undefinedVar);");
    let c = fixes_at(&p, TEST, &text);
    let (_, ch) = run_static(&c, &text, 0);
    assert!(rows(&ch).iter().any(|x| x.starts_with("Assertions.assertThat")));
}

#[test]
fn exclusions_apply() {
    // JAVA-IMP-016, S5
    let Some(libs) = libs() else { return };
    let p = project(&[], &libs);
    let tree = source::parse(BUG).unwrap();
    let mut s = JavaSettings::default();
    s.imports.exclude = vec!["org.assertj.core.api.AssertionsForClassTypes".into()];
    let c = collect_detailed(&p.project, &s, &p.path(TEST), BUG, &tree);
    let (_, ch) = run_static(&c, BUG, 0);
    assert_eq!(rows(&ch).len(), 3);
    s.imports.exclude = vec!["org.assertj".into()];
    let c = collect_detailed(&p.project, &s, &p.path(TEST), BUG, &tree);
    // Nothing applicable remains, so the classes whose arguments do not fit are listed (JAVA-IMP-015).
    assert!(c.static_fixes[0].1.candidates.iter().all(|x| !x.class_fqn().starts_with("org.assertj")));
}

#[test]
fn s3_collapse_to_on_demand_at_the_third_name() {
    // JAVA-IMP-033, S3
    let text = r#"package demo;

import static org.assertj.core.api.Assertions.assertThat;
import static org.assertj.core.api.Assertions.assertThatThrownBy;

public class CalcTest {
}
"#;
    let out = import_edit::add_static_import(text, "org.assertj.core.api.Assertions", "assertThatCode", &ImportSettings::default(), false);
    assert_eq!(
        out.text,
        r#"package demo;

import static org.assertj.core.api.Assertions.*;

public class CalcTest {
}
"#
    );
    // Already equivalent: nothing to do.
    let again = import_edit::add_static_import(&out.text, "org.assertj.core.api.Assertions", "assertThat", &ImportSettings::default(), false);
    assert!(again.unchanged);
}

#[test]
fn layout_of_added_imports_follows_the_default_layout() {
    // JAVA-IMP-051, 054
    let s = ImportSettings::default();
    let base = "package a;\n\nimport com.acme.Util;\nimport org.junit.Test;\n\nimport javax.crypto.Cipher;\nimport java.io.File;\n\nimport static org.junit.Assert.assertEquals;\n\nclass X {}\n";
    // Sorted into its group.
    let t = import_edit::add_class_import(base, "org.apache.Foo", "a", &s).text;
    assert!(t.contains("import com.acme.Util;\nimport org.apache.Foo;\nimport org.junit.Test;\n"), "{t}");
    // java.* after javax.* without a blank line.
    let t = import_edit::add_class_import(base, "java.util.List", "a", &s).text;
    assert!(t.contains("import java.io.File;\nimport java.util.List;\n\nimport static"), "{t}");
    // A new first import in an empty file after the package.
    let t = import_edit::add_class_import("package a;\n\nclass X {}\n", "java.util.List", "a", &s).text;
    assert_eq!(t, "package a;\n\nimport java.util.List;\n\nclass X {}\n");
    // No package: top of the file.
    let t = import_edit::add_class_import("class X {}\n", "java.util.List", "", &s).text;
    assert_eq!(t, "import java.util.List;\n\nclass X {}\n");
    // java.lang and the own package need no import (JAVA-IMP-044).
    assert!(import_edit::add_class_import(base, "java.lang.String", "a", &s).unchanged);
    assert!(import_edit::add_class_import(base, "a.Other", "a", &s).unchanged);
    // Static group first when there are only non-static imports below: static goes last.
    let t = import_edit::add_static_import("package a;\n\nimport org.junit.Test;\n\nclass X {}\n", "org.junit.Assert", "assertEquals", &s, false).text;
    assert_eq!(t, "package a;\n\nimport org.junit.Test;\n\nimport static org.junit.Assert.assertEquals;\n\nclass X {}\n");
    // The class count threshold collapses to on-demand (JAVA-IMP-053).
    let mut t = String::from("package a;\n\nimport java.util.List;\nimport java.util.Map;\nimport java.util.Set;\nimport java.util.Optional;\n\nclass X {}\n");
    t = import_edit::add_class_import(&t, "java.util.Objects", "a", &s).text;
    assert_eq!(t, "package a;\n\nimport java.util.*;\n\nclass X {}\n");
    // Windows line endings are kept.
    let t = import_edit::add_class_import("package a;\r\n\r\nimport org.x.A;\r\n\r\nclass X {}\r\n", "org.x.B", "a", &s).text;
    assert!(t.contains("import org.x.A;\r\nimport org.x.B;\r\n"), "{t:?}");
}

fn optimize(p: &Proj, rel: &str, text: &str) -> import_edit::Optimized {
    let tree = source::parse(text).unwrap();
    let (analysis, current, scope) = p.project.analyze_tree(&p.path(rel), text, &tree).unwrap();
    let env = archipel_ide_java::env::Env::new(&p.project.index, &scope, Some(&current));
    import_edit::optimize(text, &analysis.usage, &current.stub.package, &env, &ImportSettings::default())
}

#[test]
fn s7_optimize_imports_layout() {
    // JAVA-IMP-051, 052, S7
    let p = project(&[], &[("junit", find_jar("junit", "junit", None).unwrap_or_default(), DependencyScope::Test)]);
    if find_jar("junit", "junit", None).is_none() {
        return;
    }
    let text = r#"package demo;

import java.util.List;
import static org.junit.Assert.assertEquals;
import org.junit.Test;
import javax.crypto.Cipher;
import java.io.File;
import java.util.Map;

public class CalcTest {
    List<String> l; File f; Cipher c;
    @Test public void t() { assertEquals(1, 1); }
}
"#;
    let o = optimize(&p, TEST, text);
    assert_eq!(
        o.text,
        r#"package demo;

import org.junit.Test;

import javax.crypto.Cipher;
import java.io.File;
import java.util.List;

import static org.junit.Assert.assertEquals;

public class CalcTest {
    List<String> l; File f; Cipher c;
    @Test public void t() { assertEquals(1, 1); }
}
"#
    );
    assert_eq!((o.removed, o.added), (1, 0));
    assert_eq!(o.message(true).as_deref(), Some("Removed 1 import"));
}

#[test]
fn s8_on_demand_thresholds() {
    // JAVA-IMP-053, S8
    let p = project(&[], &[]);
    let o = optimize(&p, MAIN, "package demo;\nimport javax.swing.JButton;\npublic class Main { JButton b; }\n");
    assert!(o.text.contains("import javax.swing.*;"), "{}", o.text);
    let five = "package demo;\nimport java.util.List;\nimport java.util.Map;\nimport java.util.Set;\nimport java.util.Optional;\nimport java.util.Objects;\npublic class Main { List a; Map b; Set c; Optional d; Objects e; }\n";
    assert!(optimize(&p, MAIN, five).text.contains("import java.util.*;"));
    let four = "package demo;\nimport java.util.List;\nimport java.util.Map;\nimport java.util.Set;\nimport java.util.Optional;\npublic class Main { List a; Map b; Set c; Optional d; }\n";
    let o4 = optimize(&p, MAIN, four);
    assert_eq!(o4.text.matches("import java.util.").count(), 4);
    assert!(!o4.text.contains(".*"));
    let st3 = "package demo;\nimport static java.lang.Math.max;\nimport static java.lang.Math.min;\nimport static java.lang.Math.abs;\npublic class Main { int a = max(1, 2) + min(1, 2) + abs(1); }\n";
    assert!(optimize(&p, MAIN, st3).text.contains("import static java.lang.Math.*;"));
    let st2 = "package demo;\nimport static java.lang.Math.max;\nimport static java.lang.Math.min;\npublic class Main { int a = max(1, 2) + min(1, 2); }\n";
    let o2 = optimize(&p, MAIN, st2);
    assert!(o2.text.contains("Math.max;") && o2.text.contains("Math.min;"));
}

#[test]
fn s9_conflicts_keep_names_unambiguous() {
    // JAVA-IMP-056, S9
    let p = project(&[], &[]);
    let text = "package demo;\nimport java.util.Date;\nimport java.sql.Date;\nimport java.util.List;\nimport java.util.Map;\nimport java.util.Set;\nimport java.util.Optional;\nimport java.sql.Connection;\nimport java.sql.Statement;\nimport java.sql.ResultSet;\nimport java.sql.Time;\npublic class Main { }\n";
    // `Date` appears twice (invalid Java, only the invariant matters): both stay single.
    let used = "package demo;\nimport java.util.List;\nimport java.util.Map;\nimport java.util.Set;\nimport java.util.Optional;\nimport java.util.Objects;\nimport java.util.Date;\nimport java.sql.Connection;\nimport java.sql.Statement;\nimport java.sql.ResultSet;\nimport java.sql.Time;\nimport java.sql.Timestamp;\npublic class Main { List a; Map b; Set c; Optional d; Objects e; Date date; Connection f; Statement g; ResultSet h; Time i; Timestamp j; }\n";
    let _ = text;
    let o = optimize(&p, MAIN, used);
    // Everything still resolves to the same classes: no new errors.
    let errs = p.analyze(MAIN, &o.text);
    assert!(errs.is_empty(), "{errs:?}\n{}", o.text);
    assert!(o.text.contains("import java.util.*;") && o.text.contains("import java.sql.*;"), "{}", o.text);
}

#[test]
fn import_class_ordering_and_edit() {
    // JAVA-IMP-040..044, 042
    let p = project(&[], &[]);
    let text = "package demo;\n\npublic class Main { List l; }\n";
    let c = fixes_at(&p, MAIN, text);
    let (_, fix) = &c.class_fixes[0];
    let names: Vec<_> = fix.candidates.iter().map(|c| c.fqn().to_string()).collect();
    assert_eq!(names[0], "java.util.List");
    assert!(names.contains(&"java.awt.List".to_string()), "{names:?}");
    let cmd = fix.apply(FileId(1), FileRev(1), text, 0);
    assert_eq!(apply_command(&cmd, FileId(1), text).unwrap(), "package demo;\n\nimport java.util.List;\n\npublic class Main { List l; }\n");
    // Info on the fix itself.
    assert_eq!(c.infos.iter().find(|i| i.message.as_deref() == Some("Cannot resolve symbol 'List'")).unwrap().fixes[0].text.as_ref(), "Import class");
    // `Foo.bar` narrows by member; no auto-import for `[a-z].?` names (JAVA-IMP-081).
    let text = "package demo;\npublic class Main { void f() { Collectors.joining(); } }\n";
    let c = fixes_at(&p, MAIN, text);
    assert_eq!(c.class_fixes[0].1.candidates[0].fqn(), "java.util.stream.Collectors");
    let text = "package demo;\npublic class Main { void f() { ab.go(); } }\n";
    assert!(fixes_at(&p, MAIN, text).class_fixes.is_empty());
}

#[test]
fn unused_imports_are_reported_and_removed() {
    // JAVA-IMP-070, 071, 072, 073, 074
    let p = project(&[("src/main/java/demo/Other.java", "package demo; public class Other {}")], &[]);
    let text = r#"package demo;

import java.util.List;
import java.util.Map;
import java.util.Optional;
import java.io.File;
import demo.Other;
import static java.lang.Math.*;
import static java.util.Collections.*;

/** Uses {@link File} in docs. */
public class Main {
    Map<String, String> m;
    int x = max(1, 2);
    Optional<String> o;
}
"#;
    let c = fixes_at(&p, MAIN, text);
    let unused: Vec<_> = c.infos.iter().filter(|i| i.message.as_deref() == Some("Unused import statement")).map(|i| text[i.range.start as usize..i.range.end as usize].to_string()).collect();
    assert_eq!(unused, vec!["import java.util.List;", "import demo.Other;", "import static java.util.Collections.*;"]);
    assert!(c.infos.iter().filter(|i| i.message.as_deref() == Some("Unused import statement")).all(|i| i.severity == archipel_ide_analysis::Severity::WARNING && i.kind == archipel_ide_analysis::HighlightKind::Unused));
}

#[test]
fn unused_imports_are_not_reported_while_names_are_unresolved() {
    // JAVA-IMP-076
    let p = project(&[], &[]);
    let text = "package demo;\nimport org.missing.Lib;\nimport java.util.List;\npublic class Main { Lib l; }\n";
    let c = fixes_at(&p, MAIN, text);
    let unused: Vec<_> = c.infos.iter().filter(|i| i.message.as_deref() == Some("Unused import statement")).map(|i| text[i.range.start as usize..i.range.end as usize].to_string()).collect();
    assert_eq!(unused, vec!["import java.util.List;"]);
}
