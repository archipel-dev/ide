//! Identities shared by the syntax layer and the analysis pipeline.

/// Stable identity of a file for the lifetime of a project session (and, via
/// the index store, across restarts). Interned by `archipel-ide-analysis`'s
/// `FileTable`; survives renames reported as rename pairs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct FileId(pub u32);

/// Revision of one file's text (`TextDocument::revision` for open buffers, a
/// load counter for disk contents). Compared, never derived from text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct FileRev(pub u64);

/// A language's registry id (`"rust"`, `"java"`, ...). Copy and cheap to
/// compare; the "any language" slot of the service registry is
/// [`LanguageId::ANY`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LanguageId(pub &'static str);

impl LanguageId {
    /// The "any language" key (IntelliJ registers such extensions with
    /// `language=""`).
    pub const ANY: LanguageId = LanguageId("");

    pub fn as_str(self) -> &'static str {
        self.0
    }

    pub fn is_any(self) -> bool {
        self.0.is_empty()
    }
}

impl std::fmt::Display for LanguageId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(if self.is_any() { "<any>" } else { self.0 })
    }
}
