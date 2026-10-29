//! Native Java completion (spec 06, codeinsight 06): the position classifier, the contributors
//! (member access, reference variants, keywords, class names, static members on the second
//! press) and the weigher chain.
//!
//! The answer is synchronous from the indexes (no process, no network) in the resolve scope of
//! the file (JAVA-RES-033). Where the receiver or the scope cannot be known (an unresolved
//! symbol, an incomplete classpath) a contributor offers nothing rather than something wrong:
//! the language server's rows, when there is one, fill the gap (no false positives).
//!
//! Layout: [`classify`] reads the tokens before the caret; [`probe`] builds the tree the caret
//! sits in (the prefix replaced by a placeholder, the statement repaired); [`members`], [`variants`],
//! [`keywords`], [`class_names`] and [`statics`] contribute items; [`weigh`] orders them;
//! [`insertion`] writes what choosing an item inserts.

pub mod class_names;
pub mod classify;
mod insert;
mod keywords;
mod members;
mod probe;
mod statics;
mod variants;
pub mod weigh;

use crate::api::JavaProject;
use crate::import_edit::ImportSettings;
use crate::index::NotReady;
use crate::names::MatchClass;
use crate::stub::ClassKind;
use archipel_ide_analysis::TextEdit;
use classify::{Classified, Position, Slot, TypeFilter};
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

/// What an item stands for (its icon, its tier in the kind weigher).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemKind {
    Class(ClassKind),
    Method,
    Field,
    /// An enum constant.
    Constant,
    /// A local variable, a pattern binding.
    Local,
    Parameter,
    Keyword,
    Package,
}

/// A row of the popup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    /// The simple name shown and matched (`Entry`, `size`).
    pub name: String,
    pub kind: ItemKind,
    /// What [`insertion`] needs to write the row; classes use their FQN.
    pub key: String,
    /// `Outer.Inner` for a member class, else the simple name.
    pub qualified: String,
    /// The FQN of a class row, empty otherwise.
    pub fqn: String,
    /// The tail text of a class: the package, or the outer class for a member class.
    pub tail: String,
    /// The parameter list of a method (`(int index, E element)`), merged over its overloads.
    pub signature: String,
    /// The right-aligned type: return type of a method, type of a field or variable.
    pub ty: String,
    pub deprecated: bool,
    /// Resolvable by its simple name in the file (imported, same package, `java.lang`,
    /// declared in the file): choosing it adds no import.
    pub visible: bool,
    pub class: MatchClass,
    /// Orders items of one match class (smaller first), the weigher chain of [`weigh`].
    pub sort: String,
    /// What the typed prefix is matched against, when it is not the name.
    pub filter: Option<String>,
    /// The text of the row when it is not the name (`Assertions.assertThat`).
    pub display: Option<String>,
    /// Extra marker shown with the type (`private`: a member the place cannot reach, JAVA-CMP-008).
    pub note: String,
}

impl Item {
    pub(crate) fn blank() -> Item {
        Item {
            name: String::new(),
            kind: ItemKind::Keyword,
            key: String::new(),
            qualified: String::new(),
            fqn: String::new(),
            tail: String::new(),
            signature: String::new(),
            ty: String::new(),
            deprecated: false,
            visible: true,
            class: MatchClass::Exact,
            sort: String::new(),
            filter: None,
            display: None,
            note: String::new(),
        }
    }

    /// `Test (org.junit.jupiter.api)`: the popup row as plain text; members and keywords are their
    /// name (a method is one row per name, JAVA-CMP-016).
    pub fn label(&self) -> String {
        let shown = self.display.as_deref().unwrap_or(&self.name);
        if self.tail.is_empty() {
            shown.to_string()
        } else {
            format!("{shown} ({})", self.tail)
        }
    }

    pub fn is_class(&self) -> bool {
        matches!(self.kind, ItemKind::Class(_))
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
    /// Every legal item at this position is among the items: the language server's answer adds
    /// nothing (annotation, `new`, `extends`/`implements`/`throws`/`catch`, type arguments, a
    /// dot after a primitive).
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
    /// `text` holds `$0`, where the caret goes (inside the parentheses of a call).
    pub snippet: bool,
}

/// Whether `position` only admits class names.
pub fn is_exclusive(position: Position) -> bool {
    matches!(
        position,
        Position::Annotation { .. } | Position::New | Position::Type(_)
    )
}

/// Completes at the caret (JAVA-CMP-005: the contributors of 6.2, the method merger inside the
/// member contributor, the main contributor split by position, the class names last).
/// `Ok(None)`: nothing native to offer here.
/// `Err(NotReady)`: the indexes are still building (dumb mode, JAVA-STUB-026).
pub fn complete(input: &CompletionInput<'_>) -> Result<Option<Completion>, NotReady> {
    let classified = classify::classify(input.text, input.offset);
    if classified.position == Position::None || !input.project.applies_to(input.path) {
        return Ok(None);
    }
    if !input.project.index.is_ready() {
        return Err(NotReady);
    }
    let mut items: Vec<Item> = Vec::new();
    let mut note: Option<String> = None;
    let mut exclusive = is_exclusive(classified.position);
    let mut answered = false;
    match classified.position {
        Position::Qualified => match members::complete(input, &classified)? {
            Some(m) => {
                answered = true;
                exclusive = m.exclusive;
                items = m.items;
                if m.incomplete {
                    note = Some(
                        "classpath incomplete: some supertypes of the receiver are not resolved"
                            .into(),
                    );
                }
            }
            None => return Ok(None),
        },
        Position::Annotation { .. } | Position::New | Position::Type(_) => {
            let probe = probe::at(input, &classified);
            let suitable = match (&classified.position, &probe) {
                (Position::New, Some(p)) => p.suitable_classes(input)?,
                _ => Default::default(),
            };
            let classes = class_names::collect(input, &classified, &suitable)?;
            note = classes.note;
            items = classes.items;
            if classified.position == Position::New {
                items.extend(keywords::primitive_types(&classified));
            }
            answered = true;
        }
        Position::Mixed { slot, .. } => {
            let probe = probe::at(input, &classified);
            if class_names::wanted(&classified, input.count) {
                let suitable = match &probe {
                    Some(p) => p.suitable_classes(input)?,
                    None => Default::default(),
                };
                let classes = class_names::collect(input, &classified, &suitable)?;
                note = classes.note;
                items.extend(classes.items);
                answered = true;
            }
            if let Some(p) = &probe
                && matches!(slot, Slot::Statement | Slot::Expression)
            {
                let vars = variants::collect(input, &classified, p)?;
                answered |= !vars.is_empty();
                items.extend(vars);
                if input.count >= 2 && !classified.prefix.is_empty() {
                    let st = statics::collect(input, &classified, p)?;
                    answered |= !st.is_empty();
                    items.extend(st);
                }
            }
            let kw = keywords::collect(&classified, probe.as_ref().map(|p| p.flow()));
            answered |= !kw.is_empty();
            items.extend(kw);
        }
        Position::None => return Ok(None),
    }
    if !answered && items.is_empty() {
        return Ok(None);
    }
    if note.is_none() {
        note = class_names::classpath_note(input.project, input);
    }
    // Weigher chain: prefix quality first, then the sort key of `weigh`.
    items.sort_by(|a, b| {
        a.class
            .cmp(&b.class)
            .then_with(|| a.sort.cmp(&b.sort))
            .then_with(|| a.key.cmp(&b.key))
    });
    if classified.position == Position::New {
        items = class_names::constructible(input, items, MAX_ITEMS)?;
    }
    items.truncate(MAX_ITEMS);
    Ok(Some(Completion {
        position: classified.position,
        prefix_start: classified.prefix_start,
        prefix: classified.prefix,
        items,
        exclusive,
        note,
    }))
}

/// Items returned at most (after ranking).
pub(crate) const MAX_ITEMS: usize = 150;

/// Computes the insertion of the row `key` for the file text (JAVA-CMP-019..021, 035).
pub fn insertion(input: &CompletionInput<'_>, key: &str) -> Result<Option<Insertion>, NotReady> {
    insert::insertion(input, key)
}

/// Whether typing `typed` before the caret starts a completion on its own (CI-CMP-030,
/// JAVA-CMP-004: never in comments and strings, never after a number literal): after
/// `@`, after `new `, after a dot that follows an expression, outside comments and strings.
pub fn auto_popup_after(text_before_caret: &str) -> bool {
    let c = classify::classify(text_before_caret, text_before_caret.len());
    match c.position {
        Position::Annotation { .. } => c.prefix.is_empty() && text_before_caret.ends_with('@'),
        Position::New => c.prefix.is_empty() && text_before_caret.ends_with(' '),
        Position::Type(
            TypeFilter::Interface | TypeFilter::Throwable | TypeFilter::ExtendsClass,
        ) => c.prefix.is_empty() && text_before_caret.ends_with(' '),
        Position::Qualified => c.prefix.is_empty() && text_before_caret.ends_with('.'),
        _ => false,
    }
}

/// The classification, for callers that only need the position.
pub fn classify_at(text: &str, offset: usize) -> Classified {
    classify::classify(text, offset)
}
