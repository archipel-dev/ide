//! The class-name contributor (spec 06 6.4, JAVA-CMP-018..021, 025): classes by name from the
//! short-name index in the resolve scope of the file, filtered by the position, ranked by the
//! weigher chain basics, inserted with an import through the import machinery of spec 05.

use super::classify::{Position, TypeFilter, classify as classify_position, target_fits};
use super::{Completion, CompletionInput, Insertion, Item, is_exclusive};
use crate::classpath::Completeness;
use crate::import_edit::{add_class_import, minimal_edit};
use crate::index::{JavaIndexState, NotReady};
use crate::names::{MatchClass, NameEntry, NameMatcher, is_discouraged, is_internal_package};
use crate::scope::ResolveScope;
use crate::source;
use crate::stub::{ClassKind, ClassStub, FileStub, flags};
use std::collections::{HashMap, HashSet};

/// Items returned at most (after ranking).
const MAX_ITEMS: usize = 150;
/// Candidates taken from the index before the weigher chain.
const MAX_CANDIDATES: usize = 600;

/// The classes resolvable by simple name in a file: simple name to FQN (JAVA-RES-018..023 as far
/// as the completion needs: declared in the file, single imports, same package, on-demand
/// imports, `java.lang`).
pub struct Visible {
    names: HashMap<String, String>,
    pub package: String,
}

impl Visible {
    pub fn build(
        index: &crate::index::JavaIndex,
        scope: &ResolveScope,
        stub: &FileStub,
    ) -> Result<Self, NotReady> {
        let mut names: HashMap<String, String> = HashMap::new();
        let add_package = |names: &mut HashMap<String, String>, package: &str| {
            if let Ok(list) = index.classes_in_package(package, scope) {
                for e in list.iter().filter(|e| !e.is_nested()) {
                    names.insert(e.name().to_string(), e.fqn.to_string());
                }
            }
        };
        // Lowest priority first, later insertions win.
        add_package(&mut names, "java.lang");
        for imp in stub.imports.iter().filter(|i| i.on_demand && !i.is_static) {
            add_package(&mut names, &imp.name);
        }
        add_package(&mut names, &stub.package);
        for imp in stub.imports.iter().filter(|i| !i.on_demand && !i.is_static) {
            if let Some(name) = imp.name.rsplit('.').next() {
                names.insert(name.to_string(), imp.name.clone());
            }
        }
        for imp in stub.imports.iter().filter(|i| !i.on_demand && i.is_static) {
            if let Some(name) = imp.name.rsplit('.').next() {
                // A statically imported member type is visible by its simple name.
                names
                    .entry(name.to_string())
                    .or_insert_with(|| imp.name.clone());
            }
        }
        for top in &stub.classes {
            let fqn = top.qname.clone();
            names.insert(top.name.clone(), fqn);
        }
        Ok(Self {
            names,
            package: stub.package.clone(),
        })
    }

    pub fn get(&self, name: &str) -> Option<&str> {
        self.names.get(name).map(String::as_str)
    }
}

fn local_classes(stub: &FileStub) -> Vec<(&ClassStub, bool)> {
    let mut out = Vec::new();
    for top in &stub.classes {
        top.walk(&mut Vec::new(), &mut |p, c| out.push((c, p.len() > 1)));
    }
    out
}

fn kind_fits(
    position: Position,
    kind: ClassKind,
    class_flags: u32,
    targets: Option<u16>,
    throwable: &HashSet<String>,
    fqn: &str,
) -> bool {
    let is_abstract = class_flags & flags::ABSTRACT != 0
        || matches!(kind, ClassKind::Interface | ClassKind::Annotation);
    match position {
        Position::Annotation { target } => {
            kind == ClassKind::Annotation && target_fits(targets, target)
        }
        Position::New => matches!(kind, ClassKind::Class | ClassKind::Record) && !is_abstract,
        Position::Type(TypeFilter::ExtendsClass) => {
            kind == ClassKind::Class && class_flags & flags::FINAL == 0
        }
        Position::Type(TypeFilter::Interface) => kind == ClassKind::Interface,
        Position::Type(TypeFilter::Throwable) => {
            kind == ClassKind::Class && (fqn == "java.lang.Throwable" || throwable.contains(fqn))
        }
        Position::Type(TypeFilter::Any | TypeFilter::Argument) | Position::Mixed { .. } => {
            kind != ClassKind::Annotation
        }
        Position::None => false,
    }
}

pub fn complete(input: &CompletionInput<'_>) -> Result<Option<Completion>, NotReady> {
    let classified = classify_position(input.text, input.offset);
    let position = classified.position;
    if position == Position::None || !input.project.applies_to(input.path) {
        return Ok(None);
    }
    // Declaration and statement positions: classes join the other variants only once the
    // prefix can start a class name (JAVA-CMP-001); every press after the first takes any.
    if matches!(position, Position::Mixed { .. }) {
        let upper = classified
            .prefix
            .chars()
            .next()
            .is_some_and(char::is_uppercase);
        if classified.prefix.is_empty() || !upper && input.count < 2 {
            return Ok(None);
        }
    }
    let project = input.project;
    let index = &project.index;
    let note = match index.state() {
        JavaIndexState::Ready => classpath_note(project, input),
        _ => return Err(NotReady),
    };
    let scope = project.scope_for(input.path);
    let stub = source::build_file_stub(input.text);
    let visible = Visible::build(index, &scope, &stub)?;
    let own_package = stub.package.clone();
    let cur_unit = index.unit_for_path(input.path);
    let throwable: HashSet<String> = if position == Position::Type(TypeFilter::Throwable) {
        index.inheritor_closure("Throwable", &scope)?
    } else {
        HashSet::new()
    };
    let settings = input.settings;
    let count = input.count;
    let accept = |e: &NameEntry| -> bool {
        if Some(e.unit) == cur_unit && cur_unit.is_some() {
            return false; // the live file's own classes come from its fresh stub
        }
        let package = e.package();
        if count < 3 && is_internal_package(package) {
            return false; // library-internal classes appear on the third press only
        }
        if settings.is_excluded(&e.fqn) {
            return false;
        }
        if package.is_empty() && !own_package.is_empty() {
            return false; // the default package is not importable
        }
        let accessible = if e.is_nested() {
            e.is_public() && (e.flags & flags::STATIC != 0 || e.kind != ClassKind::Class)
        } else {
            e.is_public() || package == own_package || count >= 3
        };
        accessible && kind_fits(position, e.kind, e.flags, e.targets, &throwable, &e.fqn)
    };
    let hits = index.names_matching(&scope, &classified.prefix, &accept, MAX_CANDIDATES)?;
    let matcher = NameMatcher::new(&classified.prefix);
    let mut items: Vec<Item> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    // Classes of the file itself: always visible, never imported.
    for (c, nested) in local_classes(&stub) {
        let Some(class) = matcher.classify(&c.name) else {
            continue;
        };
        let targets = if c.kind == ClassKind::Annotation {
            crate::names::parse_targets(&c.annotations)
        } else {
            None
        };
        if !kind_fits(position, c.kind, c.flags, targets, &throwable, &c.qname) {
            continue;
        }
        if !seen.insert(c.qname.clone()) {
            continue;
        }
        items.push(make_item(
            &c.name,
            c.qname.clone(),
            &stub.package,
            &stub.package,
            nested,
            c.kind,
            c.deprecated,
            true,
            class,
            0,
        ));
    }
    for h in hits {
        let e = &h.entry;
        if !seen.insert(e.fqn.to_string()) {
            continue;
        }
        let top_visible = if e.is_nested() {
            visible.get(e.top_fqn().rsplit('.').next().unwrap_or("")) == Some(e.top_fqn())
        } else {
            visible.get(e.name()) == Some(&*e.fqn)
        };
        let mut item = make_item(
            e.name(),
            e.fqn.to_string(),
            e.package(),
            &own_package,
            e.is_nested(),
            e.kind,
            e.deprecated,
            top_visible,
            h.class,
            h.unit_rank,
        );
        if is_discouraged(e.package(), &e.fqn) {
            item.sort.insert(0, '9');
        }
        items.push(item);
    }
    items.sort_by(|a, b| {
        a.class
            .cmp(&b.class)
            .then_with(|| a.sort.cmp(&b.sort))
            .then_with(|| a.fqn.cmp(&b.fqn))
    });
    items.truncate(MAX_ITEMS);
    Ok(Some(Completion {
        position,
        prefix_start: classified.prefix_start,
        prefix: classified.prefix,
        items,
        exclusive: is_exclusive(position),
        note,
    }))
}

fn classpath_note(
    project: &crate::api::JavaProject,
    input: &CompletionInput<'_>,
) -> Option<String> {
    let snapshot = project.classpath_for(input.path);
    match &snapshot.completeness {
        Completeness::Complete => None,
        c => c.describe().map(|d| format!("classpath incomplete: {d}")),
    }
}

/// Weigher chain basics (JAVA-CMP-027): the key sorts, within one match class, discouraged and
/// deprecated classes after the others, classes the file already sees before those it would
/// import, nearer classpath entries first, shorter names first.
#[allow(clippy::too_many_arguments)]
fn make_item(
    name: &str,
    fqn: String,
    package: &str,
    own_package: &str,
    nested: bool,
    kind: ClassKind,
    deprecated: bool,
    visible: bool,
    class: MatchClass,
    unit_rank: u32,
) -> Item {
    let qualified = if nested {
        // `a.b.Outer.Inner` -> `Outer.Inner`
        let rest = if package.is_empty() {
            fqn.as_str()
        } else {
            &fqn[package.len() + 1..]
        };
        rest.to_string()
    } else {
        name.to_string()
    };
    // No package hint for classes of the file or its package (6.4.1 step 8).
    let tail = if nested {
        fqn.rsplit_once('.')
            .map_or(String::new(), |(o, _)| o.to_string())
    } else if package == own_package {
        String::new()
    } else {
        package.to_string()
    };
    let sort = format!(
        "{}{}{:05}{:04}{}",
        u8::from(deprecated),
        u8::from(!visible),
        unit_rank.min(99_999),
        name.len().min(9_999),
        name.to_lowercase()
    );
    Item {
        name: name.to_string(),
        qualified,
        fqn,
        tail,
        kind,
        deprecated,
        visible,
        class,
        sort,
    }
}

/// What choosing `fqn` inserts (JAVA-CMP-019..021): the simple name and an import; the FQN when
/// the simple name belongs to another class (name clash, JAVA-CMP-020); nothing but the name
/// for classes the file already sees.
pub fn insertion(input: &CompletionInput<'_>, fqn: &str) -> Result<Option<Insertion>, NotReady> {
    let project = input.project;
    let index = &project.index;
    let scope = project.scope_for(input.path);
    let stub = source::build_file_stub(input.text);
    let visible = Visible::build(index, &scope, &stub)?;
    // Class found in the index, or declared in the file itself.
    let (package, top_fqn) = if local_classes(&stub).iter().any(|(c, _)| c.qname == fqn) {
        let top = stub
            .classes
            .iter()
            .find(|t| fqn == t.qname || fqn.starts_with(&format!("{}.", t.qname)))
            .map_or(fqn.to_string(), |t| t.qname.clone());
        (stub.package.clone(), top)
    } else {
        let Some(hit) = index.find_class(fqn, &scope)? else {
            return Ok(None);
        };
        (hit.package().to_string(), hit.record.class.qname.clone())
    };
    let qualified = if package.is_empty() {
        fqn.to_string()
    } else {
        fqn[package.len() + 1..].to_string()
    };
    let top_name = qualified
        .split('.')
        .next()
        .unwrap_or(&qualified)
        .to_string();
    match visible.get(&top_name) {
        Some(v) if v == top_fqn => {
            return Ok(Some(Insertion {
                text: qualified,
                edits: Vec::new(),
            }));
        }
        Some(_) => {
            // The simple name is taken by another class: write the qualified name.
            return Ok(Some(Insertion {
                text: fqn.to_string(),
                edits: Vec::new(),
            }));
        }
        None => {}
    }
    let outcome = add_class_import(input.text, &top_fqn, &visible.package, input.settings);
    let edits = if outcome.unchanged {
        Vec::new()
    } else {
        minimal_edit(input.text, &outcome.text)
            .into_iter()
            .collect()
    };
    Ok(Some(Insertion {
        text: qualified,
        edits,
    }))
}
