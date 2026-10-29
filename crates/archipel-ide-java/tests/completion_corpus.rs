//! False-positive hunt for member completion on real code (ignored; a developer tool like
//! `corpus.rs`). In valid code the member used after every dot must be among the members the
//! engine offers for the receiver it typed; when it is not, either the receiver type or the member
//! enumeration is wrong. The engine may also answer nothing (an unknown receiver): that is the
//! graceful fallback and only counted.
//!
//! `ARCHIPEL_JAVA_CORPUS_SOURCES=<sources.jar> ARCHIPEL_JAVA_CORPUS_LIBS=a.jar:b.jar
//! ARCHIPEL_JAVA_CORPUS_LIMIT=40 cargo test -p archipel-ide-java --release --test completion_corpus
//! -- --ignored --nocapture` (`ARCHIPEL_JAVA_CORPUS_PREFIX` selects a directory of the archive,
//! `ARCHIPEL_JAVA_CORPUS_MODULES` strips the module directory of a JDK `src.zip`).

mod common;
use archipel_ide_java::complete::{self, CompletionInput};
use archipel_ide_java::import_edit::ImportSettings;
use archipel_ide_java::source;
use common::*;
use std::io::Read;

fn member_sites(text: &str) -> Vec<(usize, String, usize)> {
    let Some(tree) = source::parse(text) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let mut stack = vec![tree.root_node()];
    while let Some(n) = stack.pop() {
        let (field, object) = match n.kind() {
            "method_invocation" => (
                n.child_by_field_name("name"),
                n.child_by_field_name("object"),
            ),
            "field_access" => (
                n.child_by_field_name("field"),
                n.child_by_field_name("object"),
            ),
            _ => (None, None),
        };
        if let (Some(f), Some(o)) = (field, object)
            && f.kind() == "identifier"
            && !n.has_error()
        {
            // `super.foo()` and `a.new B()` are other shapes; the name is what must be offered.
            // The caret after the whole name: the prefix narrows the list, so a receiver with
            // hundreds of members (`Opcodes`) cannot hide it behind the row limit.
            out.push((
                f.end_byte(),
                text[f.byte_range()].to_string(),
                o.start_byte(),
            ));
        }
        let mut c = n.walk();
        stack.extend(n.children(&mut c));
    }
    out.sort();
    out
}

#[test]
#[ignore = "needs a corpus"]
fn members_used_in_valid_code_are_offered_for_their_receivers() {
    let Ok(src) = std::env::var("ARCHIPEL_JAVA_CORPUS_SOURCES") else {
        return;
    };
    let prefix = std::env::var("ARCHIPEL_JAVA_CORPUS_PREFIX").unwrap_or_default();
    let limit: usize = std::env::var("ARCHIPEL_JAVA_CORPUS_LIMIT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(30);
    let mut zip = zip::ZipArchive::new(std::fs::File::open(&src).unwrap()).unwrap();
    let mut files: Vec<(String, String)> = Vec::new();
    for i in 0..zip.len() {
        let mut e = zip.by_index(i).unwrap();
        let name = e.name().to_string();
        if name.starts_with(&prefix)
            && name.ends_with(".java")
            && !name.ends_with("module-info.java")
            && !name.ends_with("package-info.java")
            && files.len() < limit
            && std::env::var("ARCHIPEL_JAVA_CORPUS_FILTER").is_ok_and(|f| name.contains(&f))
                == std::env::var("ARCHIPEL_JAVA_CORPUS_FILTER").is_ok()
        {
            let mut text = String::new();
            if e.read_to_string(&mut text).is_ok() {
                // A JDK `src.zip` has a module directory first (`java.base/java/util/X.java`),
                // a sources jar does not.
                let rel = if std::env::var("ARCHIPEL_JAVA_CORPUS_MODULES").is_ok() {
                    name.split_once('/').map_or(name.as_str(), |(_, r)| r)
                } else {
                    name.as_str()
                };
                files.push((format!("src/main/java/{rel}"), text));
            }
        }
    }
    let refs: Vec<(&str, &str)> = files
        .iter()
        .map(|(a, b)| (a.as_str(), b.as_str()))
        .collect();
    let libs: Vec<(
        String,
        std::path::PathBuf,
        archipel_ide_projectmodel::DependencyScope,
    )> = std::env::var("ARCHIPEL_JAVA_CORPUS_LIBS")
        .unwrap_or_default()
        .split(':')
        .filter(|s| !s.is_empty())
        .enumerate()
        .map(|(i, p)| {
            (
                format!("lib{i}"),
                std::path::PathBuf::from(p),
                archipel_ide_projectmodel::DependencyScope::Compile,
            )
        })
        .collect();
    let libs_ref: Vec<(
        &str,
        std::path::PathBuf,
        archipel_ide_projectmodel::DependencyScope,
    )> = libs
        .iter()
        .map(|(n, p, s)| (n.as_str(), p.clone(), *s))
        .collect();
    let p = project(&refs, &libs_ref);
    let settings = ImportSettings::default();
    let (mut sites, mut answered, mut misses) = (0usize, 0usize, Vec::new());
    let mut incomplete_answers = 0usize;
    let started = std::time::Instant::now();
    for (path, text) in &files {
        let abs = p.path(path);
        for (offset, name, recv_start) in member_sites(text) {
            sites += 1;
            let input = CompletionInput {
                project: &p.project,
                settings: &settings,
                path: &abs,
                text,
                offset,
                count: 1,
            };
            let Ok(Some(c)) = complete::complete(&input) else {
                continue;
            };
            if c.items.is_empty() && c.exclusive {
                continue; // a primitive: nothing, by design (the code is not valid Java then)
            }
            answered += 1;
            // A missing library leaves a supertype unresolved: the answer says so in its note and
            // is a subset, not a wrong one.
            let incomplete = c.note.as_deref().is_some_and(|n| n.contains("supertypes"));
            if incomplete {
                incomplete_answers += 1;
            }
            if !incomplete && !c.items.iter().any(|i| i.name == name) {
                let line = text[..offset].matches('\n').count() + 1;
                let some: Vec<&str> = c.items.iter().take(40).map(|i| i.name.as_str()).collect();
                misses.push(format!(
                    "{path}:{line}: `{name}` not among {} items {some:?} after `{}`",
                    c.items.len(),
                    text[recv_start..offset].lines().next().unwrap_or("")
                ));
            }
        }
    }
    println!(
        "{} files, {sites} member sites, {answered} answered natively ({:.0}%, {incomplete_answers} with a missing supertype), {} misses, {:.1}s",
        files.len(),
        100.0 * answered as f64 / sites.max(1) as f64,
        misses.len(),
        started.elapsed().as_secs_f64()
    );
    for m in misses.iter().take(60) {
        println!("  {m}");
    }
    assert!(misses.len() * 100 <= answered.max(1), "too many misses");
}
