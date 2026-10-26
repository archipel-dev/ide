//! Typed read access to the built-in indexes through a [`Snapshot`].
//!
//! Every function returns [`IndexNotReady`] while the index it needs is not
//! ready (dumb mode, per index); callers degrade instead of blocking.
//! [`SymbolLookup`] has the shape of the old `quick_nav::SymbolIndex`
//! (`all`, name entries with kind, path, line and container) so the quick
//! navigation code can switch to it without changes to its consumers.

use crate::hash::word_hash;
use crate::indexes::{
    FileNameIndex, FileTypeIndex, MemberKey, MembersIndex, QualifiedIndex, ShortNameIndex,
    StubEntry, StubsIndex, SuperTypesIndex, TrigramIndex, WordsIndex, trigram_key, trigrams,
};
use crate::stubs::StubTree;
use crate::todo::{TodoIndex, TodoItem};
use archipel_ide_analysis::index::{IndexQuery, IndexReader};
use archipel_ide_analysis::resolve::{FileSet, SearchScope};
use archipel_ide_analysis::{FileId, IndexNotReady, Snapshot};
use archipel_ide_language::structure::SymbolKind;
use std::path::PathBuf;

/// A declaration found through the stub indexes (field-compatible with
/// `quick_nav::SymbolEntry`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolHit {
    pub name: String,
    pub kind: SymbolKind,
    /// Project-relative path of the declaring file.
    pub path: PathBuf,
    /// Zero-based line of the name.
    pub line: usize,
    pub container: Option<String>,
    pub file: FileId,
    pub flags: u16,
}

fn hit(snapshot: &Snapshot, file: FileId, entry: StubEntry) -> Option<SymbolHit> {
    Some(SymbolHit {
        name: entry.name,
        kind: entry.kind,
        path: snapshot.path(file)?.to_path_buf(),
        line: entry.line as usize,
        container: entry.container,
        file,
        flags: entry.flags,
    })
}

/// Reader over `Stub:ShortName` (and friends) with the old `SymbolIndex` shape.
pub struct SymbolLookup<'a> {
    snapshot: &'a Snapshot,
    short: IndexReader<ShortNameIndex>,
}

impl<'a> SymbolLookup<'a> {
    pub fn new(snapshot: &'a Snapshot) -> Result<Self, IndexNotReady> {
        Ok(Self {
            snapshot,
            short: snapshot.require::<ShortNameIndex>()?,
        })
    }

    /// Every declaration of the project (the old `SymbolIndex::all`).
    pub fn all(&self) -> Vec<SymbolHit> {
        self.matching(&|_| true)
    }

    /// Declarations whose short name satisfies `matcher`.
    pub fn matching(&self, matcher: &dyn Fn(&str) -> bool) -> Vec<SymbolHit> {
        let Ok(keys) = self.short.all_keys(&|key: &String| matcher(key)) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for key in keys {
            out.extend(self.by_name(&key));
        }
        out
    }

    /// Declarations named exactly `name`.
    pub fn by_name(&self, name: &str) -> Vec<SymbolHit> {
        let Ok(values) = self.short.values(&name.to_string(), &SearchScope::Project) else {
            return Vec::new();
        };
        values
            .into_iter()
            .filter_map(|(file, entry)| hit(self.snapshot, file, entry))
            .collect()
    }

    /// Class-like declarations (Go to Class).
    pub fn classes(&self) -> Vec<SymbolHit> {
        self.matching(&|_| true)
            .into_iter()
            .filter(|hit| {
                matches!(
                    hit.kind,
                    SymbolKind::Class | SymbolKind::Interface | SymbolKind::Enum | SymbolKind::Type
                )
            })
            .collect()
    }
}

/// Declarations with the qualified name `name` (`com.acme.Shop`).
pub fn by_qualified_name(snapshot: &Snapshot, name: &str) -> Result<Vec<SymbolHit>, IndexNotReady> {
    let reader = snapshot.require::<QualifiedIndex>()?;
    Ok(reader
        .values(&name.to_string(), &SearchScope::Project)?
        .into_iter()
        .filter_map(|(file, entry)| hit(snapshot, file, entry))
        .collect())
}

/// Declarations that extend or implement `super_name` (short name).
pub fn subtypes_of(snapshot: &Snapshot, super_name: &str) -> Result<Vec<SymbolHit>, IndexNotReady> {
    let reader = snapshot.require::<SuperTypesIndex>()?;
    Ok(reader
        .values(&super_name.to_string(), &SearchScope::Project)?
        .into_iter()
        .filter_map(|(file, entry)| hit(snapshot, file, entry))
        .collect())
}

/// Members named `member` of the container `container` (short names).
pub fn members_named(
    snapshot: &Snapshot,
    container: &str,
    member: &str,
) -> Result<Vec<SymbolHit>, IndexNotReady> {
    let reader = snapshot.require::<MembersIndex>()?;
    Ok(reader
        .values(&MemberKey::new(container, member), &SearchScope::Project)?
        .into_iter()
        .filter_map(|(file, entry)| hit(snapshot, file, entry))
        .collect())
}

/// Every member of `container` (all member names).
pub fn members_of(snapshot: &Snapshot, container: &str) -> Result<Vec<SymbolHit>, IndexNotReady> {
    let reader = snapshot.require::<MembersIndex>()?;
    let keys = reader.all_keys(&|key: &MemberKey| key.container == container)?;
    let mut out = Vec::new();
    for key in keys {
        for (file, entry) in reader.values(&key, &SearchScope::Project)? {
            out.extend(hit(snapshot, file, entry));
        }
    }
    Ok(out)
}

/// The stub tree of a file (declarations without reading or parsing it).
pub fn stub_tree_of(snapshot: &Snapshot, file: FileId) -> Result<Option<StubTree>, IndexNotReady> {
    let reader = snapshot.require::<StubsIndex>()?;
    Ok(reader
        .values(&file.0, &SearchScope::CurrentFile(file))?
        .into_iter()
        .next()
        .map(|(_, tree)| tree))
}

/// One file name entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NameHit {
    /// Lower-case file name.
    pub name: String,
    pub file: FileId,
    pub path: PathBuf,
}

/// Files whose lower-case name satisfies `matcher` (Go to File).
pub fn files_matching(
    snapshot: &Snapshot,
    matcher: &dyn Fn(&str) -> bool,
) -> Result<Vec<NameHit>, IndexNotReady> {
    let reader = snapshot.require::<FileNameIndex>()?;
    let mut out = Vec::new();
    for name in reader.all_keys(&|name: &String| matcher(name))? {
        for file in reader.files_for_key(&name)?.iter() {
            if let Some(path) = snapshot.path(file) {
                out.push(NameHit {
                    name: name.clone(),
                    file,
                    path: path.to_path_buf(),
                });
            }
        }
    }
    Ok(out)
}

/// Files of language `language_id` (`"plain_text"` for files without one).
pub fn files_of_type(snapshot: &Snapshot, language_id: &str) -> Result<FileSet, IndexNotReady> {
    snapshot
        .require::<FileTypeIndex>()?
        .files_for_key(&language_id.to_string())
}

/// Files containing the identifier `word`, with the contexts it occurs in
/// (see [`crate::indexes::context`]).
pub fn files_with_word(
    snapshot: &Snapshot,
    word: &str,
) -> Result<Vec<(FileId, u32)>, IndexNotReady> {
    snapshot
        .require::<WordsIndex>()?
        .values(&word_hash(word), &SearchScope::Project)
}

/// Candidate files for a literal substring search: files containing every
/// trigram of `literal` (case-insensitive). `Ok(None)` when the literal is
/// shorter than three bytes (no narrowing possible).
pub fn trigram_candidates(
    snapshot: &Snapshot,
    literal: &str,
) -> Result<Option<FileSet>, IndexNotReady> {
    let keys = trigrams(literal);
    if keys.is_empty() {
        return Ok(None);
    }
    let reader = snapshot.require::<TrigramIndex>()?;
    let mut result: Option<FileSet> = None;
    for key in keys {
        let files = reader.files_for_key(&key)?;
        result = Some(match result {
            None => files,
            Some(current) => current.iter().filter(|f| files.contains(*f)).collect(),
        });
        if result.as_ref().is_some_and(FileSet::is_empty) {
            break;
        }
    }
    let _ = trigram_key;
    Ok(result)
}

/// One TODO occurrence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TodoHit {
    pub file: FileId,
    pub path: PathBuf,
    /// Index of the matching pattern in the settings list.
    pub pattern: u32,
    pub item: TodoItem,
}

/// Every TODO occurrence in `scope`, ordered by path and position.
pub fn todo_hits(snapshot: &Snapshot, scope: &SearchScope) -> Result<Vec<TodoHit>, IndexNotReady> {
    let reader = snapshot.require::<TodoIndex>()?;
    let mut out = Vec::new();
    for pattern in reader.all_keys(&|_: &u32| true)? {
        for (file, value) in reader.values(&pattern, scope)? {
            let Some(path) = snapshot.path(file) else {
                continue;
            };
            for item in value.0 {
                out.push(TodoHit {
                    file,
                    path: path.to_path_buf(),
                    pattern,
                    item,
                });
            }
        }
    }
    out.sort_by(|a, b| (&a.path, a.item.start).cmp(&(&b.path, b.item.start)));
    Ok(out)
}
