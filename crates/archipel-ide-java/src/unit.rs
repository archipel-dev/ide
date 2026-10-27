//! Index units: the granule of incremental, persistent indexing. A unit is one
//! source file, one jar, one class directory or the JDK image (JAVA-RES-042: the
//! names of a library are indexed per jar with a content signature, so an unchanged
//! jar is never reparsed).

use crate::classfile::{self, ParsedClass};
use crate::jimage::JImage;
use crate::stub::{ClassStub, FileStub};
use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;
use std::path::Path;

/// Everything one unit contributes (before it is split into index entries).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UnitStubs {
    /// Compilation units: one for a source file, one per package-less group for class roots.
    pub files: Vec<FileStub>,
    pub skipped: usize,
}

fn group_by_package(classes: Vec<ParsedClass>) -> Vec<FileStub> {
    // Class files of one root, grouped by package; nesting is resolved per package.
    let mut by_pkg: BTreeMap<String, Vec<ParsedClass>> = BTreeMap::new();
    for c in classes {
        let pkg = c
            .binary_name
            .rsplit_once('/')
            .map(|(p, _)| p.replace('/', "."))
            .unwrap_or_default();
        by_pkg.entry(pkg).or_default().push(c);
    }
    by_pkg
        .into_iter()
        .map(|(package, classes)| FileStub {
            package,
            imports: Vec::new(),
            classes: classfile::nest(classes),
        })
        .collect()
}

fn valid_package_dir(path: &str) -> bool {
    // PLAT-IDX-094: every directory between the root and the class must be an identifier
    // (multi-release version directories are handled by the caller).
    path.split('/').all(|seg| {
        let mut chars = seg.chars();
        chars
            .next()
            .is_some_and(|c| c.is_alphabetic() || c == '_' || c == '$')
            && chars.all(|c| c.is_alphanumeric() || c == '_' || c == '$')
    })
}

/// Reads a jar (JAVA-RES-034). Multi-release entries (`META-INF/versions/N/...`) are ignored
/// except that the highest version not above `max_release` replaces the base class (JAVA-RES-031).
pub fn read_jar(path: &Path, max_release: u32) -> Result<UnitStubs, String> {
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipArchive::new(std::io::BufReader::new(file)).map_err(|e| e.to_string())?;
    let mut chosen: BTreeMap<String, (u32, usize)> = BTreeMap::new();
    for i in 0..zip.len() {
        let Ok(entry) = zip.by_index_raw(i) else {
            continue;
        };
        let name = entry.name().to_string();
        let Some(stem) = name.strip_suffix(".class") else {
            continue;
        };
        let (release, class) = match stem.strip_prefix("META-INF/versions/") {
            Some(rest) => match rest.split_once('/') {
                Some((v, c)) => match v.parse::<u32>() {
                    Ok(v) if v <= max_release => (v, c.to_string()),
                    _ => continue,
                },
                None => continue,
            },
            None => (0, stem.to_string()),
        };
        if !valid_package_dir(&class) {
            continue;
        }
        match chosen.get(&class) {
            Some((r, _)) if *r >= release => {}
            _ => {
                chosen.insert(class, (release, i));
            }
        }
    }
    let mut parsed = Vec::new();
    let mut skipped = 0;
    for (_, (_, index)) in chosen {
        let Ok(mut entry) = zip.by_index(index) else {
            skipped += 1;
            continue;
        };
        let mut bytes = Vec::with_capacity(entry.size() as usize);
        if entry.read_to_end(&mut bytes).is_err() {
            skipped += 1;
            continue;
        }
        match classfile::parse_class(&bytes) {
            Ok(Some(c)) => parsed.push(c),
            Ok(None) => {}
            Err(_) => skipped += 1,
        }
    }
    Ok(UnitStubs {
        files: group_by_package(parsed),
        skipped,
    })
}

/// Reads a directory of class files (compiler output used as a classes root).
pub fn read_class_dir(root: &Path) -> UnitStubs {
    let mut parsed = Vec::new();
    let mut skipped = 0;
    for entry in walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
    {
        let p = entry.path();
        if p.extension().and_then(|e| e.to_str()) != Some("class") {
            continue;
        }
        let Ok(rel) = p.strip_prefix(root) else {
            continue;
        };
        let rel = rel.with_extension("").to_string_lossy().replace('\\', "/");
        if !valid_package_dir(&rel) {
            continue;
        }
        match std::fs::read(p).map(|b| classfile::parse_class(&b)) {
            Ok(Ok(Some(c))) => parsed.push(c),
            Ok(Ok(None)) => {}
            _ => skipped += 1,
        }
    }
    UnitStubs {
        files: group_by_package(parsed),
        skipped,
    }
}

/// Reads the JDK runtime image: only packages exported without qualification by
/// their module are indexed (JAVA-RES-032 for the platform modules).
pub fn read_jdk_image(path: &Path) -> Result<UnitStubs, String> {
    let mut image = JImage::open(path).map_err(|e| e.to_string())?;
    let entries = image.entries.clone();
    let mut exports: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for e in &entries {
        if e.name == "module-info"
            && let Ok(bytes) = image.read(e)
            && let Ok((_, pkgs)) = classfile::parse_module_exports(&bytes)
        {
            exports.insert(e.module.clone(), pkgs.into_iter().collect());
        }
    }
    let mut parsed = Vec::new();
    let mut skipped = image.skipped_compressed;
    for e in &entries {
        if e.name.ends_with("module-info") || e.name.ends_with("package-info") {
            continue;
        }
        let pkg = e
            .name
            .rsplit_once('/')
            .map(|(p, _)| p.replace('/', "."))
            .unwrap_or_default();
        if exports.get(&e.module).is_none_or(|set| !set.contains(&pkg)) {
            continue;
        }
        // Nested classes of exported packages are read with their outer class; a quick
        // pre-filter on `$` keeps anonymous classes (`Foo$1`) out cheaply.
        if e.name.rsplit('/').next().is_some_and(|n| {
            n.split('$')
                .skip(1)
                .any(|part| part.chars().next().is_some_and(|c| c.is_ascii_digit()))
        }) {
            continue;
        }
        let Ok(bytes) = image.read(e) else {
            skipped += 1;
            continue;
        };
        match classfile::parse_class(&bytes) {
            Ok(Some(c)) => parsed.push(c),
            Ok(None) => {}
            Err(_) => skipped += 1,
        }
    }
    Ok(UnitStubs {
        files: group_by_package(parsed),
        skipped,
    })
}

/// All named classes of a stub tree with their top-level and nested path (used by tests and indexes).
pub fn all_classes(stubs: &UnitStubs) -> Vec<&ClassStub> {
    let mut out = Vec::new();
    for f in &stubs.files {
        for c in &f.classes {
            c.walk(&mut Vec::new(), &mut |_, c| out.push(c));
        }
    }
    out
}
