//! False-positive hunt on real code: analyzes every Java file of a sources jar against a classpath
//! and prints what was reported. Ignored; configure with
//! `ARCHIPEL_JAVA_CORPUS_SOURCES=<sources.jar> ARCHIPEL_JAVA_CORPUS_LIBS=a.jar:b.jar`.

mod common;
use archipel_ide_analysis::Severity;
use archipel_ide_java::diagnostics::{JavaSettings, collect_detailed};
use archipel_ide_java::source;
use archipel_ide_projectmodel::DependencyScope;
use common::*;
use std::collections::BTreeMap;
use std::io::Read;

#[test]
#[ignore = "needs a corpus"]
fn corpus_has_no_false_unresolved_references() {
    let Ok(src) = std::env::var("ARCHIPEL_JAVA_CORPUS_SOURCES") else { return };
    let libs: Vec<(String, std::path::PathBuf, DependencyScope)> = std::env::var("ARCHIPEL_JAVA_CORPUS_LIBS")
        .unwrap_or_default()
        .split(':')
        .filter(|s| !s.is_empty())
        .enumerate()
        .map(|(i, p)| (format!("lib{i}"), std::path::PathBuf::from(p), DependencyScope::Compile))
        .collect();
    let libs_ref: Vec<(&str, std::path::PathBuf, DependencyScope)> = libs.iter().map(|(n, p, s)| (n.as_str(), p.clone(), *s)).collect();
    let mut files: Vec<(String, String)> = Vec::new();
    let mut zip = zip::ZipArchive::new(std::fs::File::open(&src).unwrap()).unwrap();
    for i in 0..zip.len() {
        let mut e = zip.by_index(i).unwrap();
        if e.name().ends_with(".java") && !e.name().ends_with("module-info.java") && !e.name().ends_with("package-info.java") {
            let mut text = String::new();
            if e.read_to_string(&mut text).is_ok() {
                files.push((format!("src/main/java/{}", e.name()), text));
            }
        }
    }
    let refs: Vec<(&str, &str)> = files.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
    let p = project(&refs, &libs_ref);
    let started = std::time::Instant::now();
    let mut by_msg: BTreeMap<String, usize> = BTreeMap::new();
    let mut samples: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut total = 0usize;
    for (rel, text) in &files {
        let tree = source::parse(text).unwrap();
        let c = collect_detailed(&p.project, &JavaSettings::default(), &p.path(rel), text, &tree);
        for i in c.infos.iter().filter(|i| i.severity >= Severity::ERROR) {
            total += 1;
            let m = i.message.as_deref().unwrap_or("").to_string();
            *by_msg.entry(m.clone()).or_default() += 1;
            let line = text[..i.range.start as usize].matches('\n').count() + 1;
            let e = samples.entry(m).or_default();
            if e.len() < 3 {
                e.push(format!("{rel}:{line}"));
            }
        }
    }
    eprintln!("{} files, {} errors in {:.1}s", files.len(), total, started.elapsed().as_secs_f32());
    let mut v: Vec<_> = by_msg.into_iter().collect();
    v.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
    for (m, n) in v.iter().take(60) {
        eprintln!("{n:5}  {m}   {:?}", samples[m]);
    }
}
