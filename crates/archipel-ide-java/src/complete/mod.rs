//! Native Java completion (spec 06, codeinsight 06): the position classifier, the class-name
//! contributor with import on insertion, and the weigher chain basics.
//!
//! This is the class-name slice of the plan's M3: annotation types after `@`, instantiable
//! classes after `new`, the type positions of JAVA-CMP-009 and non-imported classes in
//! declaration and statement positions (JAVA-CMP-001, 018). The answer is synchronous from the
//! indexes (no process, no network) in the resolve scope of the file (JAVA-RES-033).

pub mod class_names;
pub mod classify;

use crate::api::JavaProject;
use crate::import_edit::ImportSettings;
use crate::index::NotReady;
use crate::names::MatchClass;
use crate::stub::ClassKind;
use archipel_ide_analysis::TextEdit;
use classify::{Position, TypeFilter};
use std::path::Path;

/// One completion request, answered from the indexes and the live text of the file.
pub struct CompletionInput<'a> {
    pub project: &'a JavaProject,
    pub settings: &'a ImportSettings,
    pub path: &'a Path,
    pub text: &'a str,
    /// Caret as a byte offset into `text`.
    pub offset: usize,
    /// Consecutive presses on the same prefix: 1 for auto-popup and the first press, 2 adds
    /// statics, 3 adds library-internal classes (CI-CMP-100).
    pub count: u32,
}

/// A class offered by completion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    /// The simple name shown and matched (`Entry`).
    pub name: String,
    /// `Outer.Inner` for a member class, else the simple name.
    pub qualified: String,
    pub fqn: String,
    /// The tail text: the package, or the outer class for a member class.
    pub tail: String,
    pub kind: ClassKind,
    pub deprecated: bool,
    /// Resolvable by its simple name in the file (imported, same package, `java.lang`,
    /// declared in the file): choosing it adds no import.
    pub visible: bool,
    pub class: MatchClass,
    /// Orders items of one match class (smaller first).
    pub sort: String,
}

impl Item {
    /// `Test (org.junit.jupiter.api)`: the popup row as plain text.
    pub fn label(&self) -> String {
        if self.tail.is_empty() {
            self.name.clone()
        } else {
            format!("{} ({})", self.name, self.tail)
        }
    }
}

/// The answer to one request.
#[derive(Debug, Clone)]
pub struct Completion {
    pub position: Position,
    /// Byte offset where the typed prefix starts; the item replaces `prefix_start..offset`.
    pub prefix_start: usize,
    pub prefix: String,
    pub items: Vec<Item>,
    /// Every legal item at this position is a class name: the language server's answer adds
    /// nothing (annotation, `new`, `extends`/`implements`/`throws`/`catch`, type arguments).
    pub exclusive: bool,
    /// Why the answer may be incomplete (indexing, a classpath that failed to import).
    pub note: Option<String>,
}

/// What choosing an item inserts: the text for the prefix range and the import edit
/// (JAVA-CMP-019, 020; one undo step for both).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Insertion {
    pub text: String,
    pub edits: Vec<TextEdit>,
}

/// Whether `position` only admits class names.
pub fn is_exclusive(position: Position) -> bool {
    matches!(
        position,
        Position::Annotation { .. } | Position::New | Position::Type(_)
    )
}

/// Completes class names at the caret. `Ok(None)`: nothing native to offer here.
/// `Err(NotReady)`: the indexes are still building (dumb mode, JAVA-STUB-026).
pub fn complete(input: &CompletionInput<'_>) -> Result<Option<Completion>, NotReady> {
    class_names::complete(input)
}

/// Computes the insertion of the class with FQN `fqn` for the file text (JAVA-CMP-019..021).
pub fn insertion(input: &CompletionInput<'_>, fqn: &str) -> Result<Option<Insertion>, NotReady> {
    class_names::insertion(input, fqn)
}

/// Whether typing `typed` before the caret starts a completion on its own (CI-CMP-030): after
/// `@` and after `new `, outside comments and strings.
pub fn auto_popup_after(text_before_caret: &str) -> bool {
    let c = classify::classify(text_before_caret, text_before_caret.len());
    match c.position {
        Position::Annotation { .. } => c.prefix.is_empty() && text_before_caret.ends_with('@'),
        Position::New => c.prefix.is_empty() && text_before_caret.ends_with(' '),
        Position::Type(
            TypeFilter::Interface | TypeFilter::Throwable | TypeFilter::ExtendsClass,
        ) => c.prefix.is_empty() && text_before_caret.ends_with(' '),
        _ => false,
    }
}
