//! Locating source and documentation jars next to a binary jar, in Gradle's module cache
//! (`modules-2/files-2.1/<group>/<name>/<version>/<hash>/`) and in Maven repository layouts
//! (`<group path>/<name>/<version>/`). Nothing is downloaded: only what is already on disk.

use std::path::{Path, PathBuf};

/// Source and javadoc jars that exist for `jar`.
pub fn sibling_artifacts(jar: &Path) -> (Vec<PathBuf>, Vec<PathBuf>) {
    let Some(stem) = jar.file_stem().and_then(|s| s.to_str()) else {
        return (Vec::new(), Vec::new());
    };
    let Some(dir) = jar.parent() else {
        return (Vec::new(), Vec::new());
    };
    let mut directories = vec![dir.to_owned()];
    // Gradle stores every artifact of a version in its own hash directory.
    if let Some(version_dir) = dir.parent()
        && version_dir
            .ancestors()
            .skip(1)
            .take(3)
            .any(|p| p.ends_with("files-2.1"))
        && let Ok(entries) = std::fs::read_dir(version_dir)
    {
        directories.extend(entries.flatten().map(|e| e.path()).filter(|p| p.is_dir()));
    }
    let find = |suffix: &str| {
        let wanted = format!("{stem}-{suffix}.jar");
        directories
            .iter()
            .map(|d| d.join(&wanted))
            .find(|candidate| candidate.is_file())
    };
    (
        find("sources").into_iter().collect(),
        find("javadoc").into_iter().collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn touch(path: &Path) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, b"").unwrap();
    }

    #[test]
    fn finds_maven_layout_siblings() {
        let root =
            std::env::temp_dir().join(format!("archipel-artifacts-m2-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let dir = root.join("org/acme/lib/1.0");
        touch(&dir.join("lib-1.0.jar"));
        touch(&dir.join("lib-1.0-sources.jar"));
        let (sources, docs) = sibling_artifacts(&dir.join("lib-1.0.jar"));
        assert_eq!(sources, [dir.join("lib-1.0-sources.jar")]);
        assert!(docs.is_empty());
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn finds_gradle_cache_siblings_in_other_hash_directories() {
        let root =
            std::env::temp_dir().join(format!("archipel-artifacts-gr-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let version = root.join("modules-2/files-2.1/org.acme/lib/1.0");
        touch(&version.join("aaa/lib-1.0.jar"));
        touch(&version.join("bbb/lib-1.0-sources.jar"));
        touch(&version.join("ccc/lib-1.0-javadoc.jar"));
        let (sources, docs) = sibling_artifacts(&version.join("aaa/lib-1.0.jar"));
        assert_eq!(sources, [version.join("bbb/lib-1.0-sources.jar")]);
        assert_eq!(docs, [version.join("ccc/lib-1.0-javadoc.jar")]);
        std::fs::remove_dir_all(&root).unwrap();
    }
}
