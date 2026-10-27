//! Per-file icons: the type glyph from `icons`, with Java sources peeked for the kind of
//! their type (class, interface, enum, record, annotation).

use crate::IdeApp;
use crate::icons::{Icon, TypeKind};
use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Bytes of a Java source read to find its type.
const PEEK_BYTES: usize = 8 * 1024;

/// Kinds read from disk, keyed by project-relative path and validated by mtime.
#[derive(Default)]
pub struct KindCache(HashMap<PathBuf, (SystemTime, TypeKind)>);

impl IdeApp {
    /// Icon of a project-relative file; open documents are peeked in memory, other
    /// Java files once from disk (re-read when their modification time changes).
    pub(crate) fn file_icon(&self, path: &Path) -> Icon {
        let name = Self::file_name(path);
        let icon = Icon::for_file(&name);
        if !icon.is_java_type() {
            return icon;
        }
        if let Some(document) = self.state.documents().iter().find(|d| d.path == path) {
            let len = document.text.len_chars().min(PEEK_BYTES);
            let text = document.text.slice(0, len).unwrap_or_default();
            return Icon::for_source(&name, Some(&text));
        }
        let Some(root) = self.state.project_root() else {
            return icon;
        };
        let absolute = root.join(path);
        let Some(modified) = std::fs::metadata(&absolute).and_then(|m| m.modified()).ok() else {
            return icon;
        };
        let mut cache = self.kind_cache.borrow_mut();
        if let Some((seen, kind)) = cache.0.get(path)
            && *seen == modified
        {
            return Icon::JavaType(*kind);
        }
        let mut buffer = vec![0; PEEK_BYTES];
        let read = std::fs::File::open(&absolute)
            .and_then(|mut file| file.read(&mut buffer))
            .unwrap_or(0);
        let text = String::from_utf8_lossy(&buffer[..read]);
        let kind = archipel_kit::icons::peek_java(&text);
        cache.0.insert(path.to_owned(), (modified, kind));
        Icon::JavaType(kind)
    }
}
