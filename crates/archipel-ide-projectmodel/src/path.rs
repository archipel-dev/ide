//! Lexical path helpers. The model never touches the file system: callers that want
//! symlinks resolved (importers do) canonicalize before they build the model.

use std::path::{Component, Path, PathBuf};

/// Removes `.` and resolves `..` without looking at the disk.
pub fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if !out.pop() {
                    out.push("..");
                }
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// Splits `lib.jar!/org/Foo.class` into the archive and the entry path inside it.
///
/// A leading `jar:` scheme is dropped. Paths without `!/` come back unchanged.
pub fn split_archive_entry(path: &Path) -> (PathBuf, Option<String>) {
    let text = path.to_string_lossy();
    let text = text.strip_prefix("jar:").unwrap_or(&text);
    match text.split_once("!/") {
        Some((archive, entry)) => (PathBuf::from(archive), Some(entry.to_owned())),
        None => match text.strip_suffix('!') {
            Some(archive) => (PathBuf::from(archive), Some(String::new())),
            None => (PathBuf::from(text), None),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_lexically() {
        assert_eq!(normalize(Path::new("/a/./b/../c")), PathBuf::from("/a/c"));
        assert_eq!(normalize(Path::new("a/../../b")), PathBuf::from("../b"));
    }

    #[test]
    fn splits_archive_entries() {
        let (jar, entry) = split_archive_entry(Path::new("jar:/m2/a.jar!/org/A.class"));
        assert_eq!(jar, PathBuf::from("/m2/a.jar"));
        assert_eq!(entry.as_deref(), Some("org/A.class"));
        let (jar, entry) = split_archive_entry(Path::new("/m2/a.jar"));
        assert_eq!((jar, entry), (PathBuf::from("/m2/a.jar"), None));
        assert_eq!(
            split_archive_entry(Path::new("/m2/a.jar!")).1.as_deref(),
            Some("")
        );
    }
}
