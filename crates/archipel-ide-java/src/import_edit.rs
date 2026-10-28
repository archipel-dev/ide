//! Import layout, adding imports and Optimize Imports (spec 05 section 5.6, 5.4.3, 5.7):
//! JAVA-IMP-033, 044, 050..059, 070..076.
//!
//! All operations work on the text of the file and produce one minimal [`TextEdit`], so they
//! are a single undo step (JAVA-IMP-006) and preview as a diff.

use crate::analyze::Usage;
use crate::env::Env;
use crate::source;
use crate::stub::ImportStub;
use archipel_ide_analysis::TextEdit;
use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;

/// Default Java code-style import settings (JAVA-IMP-050, JAVA-IMP-083).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportSettings {
    pub use_single_class_imports: bool,
    pub class_count_to_use_import_on_demand: usize,
    pub names_count_to_use_static_import_on_demand: usize,
    pub insert_imports_for_inner_classes: bool,
    pub layout_static_imports_separately: bool,
    pub layout_on_demand_from_same_package_first: bool,
    pub packages_to_use_import_on_demand: Vec<String>,
    pub layout: Vec<LayoutEntry>,
    /// Exclude from auto-import and completion (JAVA-IMP-016, 080).
    pub exclude: Vec<String>,
    /// Qualified names picked recently through an import fix, most recent last (JAVA-IMP-022).
    pub recent: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayoutEntry {
    AllOther,
    AllOtherStatic,
    Package {
        name: String,
        subpackages: bool,
        is_static: bool,
    },
    Blank,
}

impl Default for ImportSettings {
    fn default() -> Self {
        Self {
            use_single_class_imports: true,
            class_count_to_use_import_on_demand: 5,
            names_count_to_use_static_import_on_demand: 3,
            insert_imports_for_inner_classes: false,
            layout_static_imports_separately: true,
            layout_on_demand_from_same_package_first: true,
            packages_to_use_import_on_demand: vec!["java.awt".into(), "javax.swing".into()],
            layout: vec![
                LayoutEntry::AllOther,
                LayoutEntry::Blank,
                LayoutEntry::Package {
                    name: "javax".into(),
                    subpackages: true,
                    is_static: false,
                },
                LayoutEntry::Package {
                    name: "java".into(),
                    subpackages: true,
                    is_static: false,
                },
                LayoutEntry::Blank,
                LayoutEntry::AllOtherStatic,
            ],
            exclude: Vec::new(),
            recent: Vec::new(),
        }
    }
}

impl ImportSettings {
    /// Whether `qualified` is excluded: a dot-boundary prefix, or a `*` mask matched from the start (5.2.2 step 3a).
    pub fn is_excluded(&self, qualified: &str) -> bool {
        self.exclude.iter().any(|e| {
            if let Some(star) = e.find('*') {
                let prefix = &e[..star];
                qualified.starts_with(prefix)
                    && (prefix.ends_with('.')
                        || prefix.is_empty()
                        || qualified[prefix.len()..].starts_with('.')
                        || star == e.len() - 1 && true)
            } else {
                qualified == e
                    || (qualified.starts_with(e.as_str()) && qualified[e.len()..].starts_with('.'))
            }
        })
    }

    /// The "do not auto-import" pattern `[a-z].?` (JAVA-IMP-081).
    pub fn no_auto_import(&self, name: &str) -> bool {
        let mut chars = name.chars();
        match (chars.next(), chars.next(), chars.next()) {
            (Some(a), None, _) => a.is_ascii_lowercase(),
            (Some(a), Some(_), None) => a.is_ascii_lowercase(),
            _ => false,
        }
    }
}

/// One import statement of the file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportNode {
    pub stub: ImportStub,
    /// The statement from `import` to `;`.
    pub range: Range<usize>,
    /// Comments directly before the statement (kept with it, JAVA-IMP-058).
    pub leading_comments: Vec<Range<usize>>,
}

impl ImportNode {
    /// The text that orders the statement inside its layout entry (5.6.2 step 2).
    pub fn sort_key(&self) -> String {
        if self.stub.on_demand {
            format!("{}.", self.stub.name)
        } else {
            self.stub.name.clone()
        }
    }
}

pub fn parse_imports(text: &str) -> (Vec<ImportNode>, Option<usize>) {
    let Some(tree) = source::parse(text) else {
        return (Vec::new(), None);
    };
    parse_imports_in(text, &tree)
}

/// [`parse_imports`] over a tree the caller already has.
pub fn parse_imports_in(text: &str, tree: &tree_sitter::Tree) -> (Vec<ImportNode>, Option<usize>) {
    let root = tree.root_node();
    let mut out = Vec::new();
    let mut package_end = None;
    let mut pending: Vec<Range<usize>> = Vec::new();
    let mut cur = root.walk();
    for n in root.children(&mut cur) {
        match n.kind() {
            "package_declaration" => package_end = Some(n.end_byte()),
            "line_comment" | "block_comment" => pending.push(n.byte_range()),
            "import_declaration" => {
                let mut is_static = false;
                let mut on_demand = false;
                let mut name = String::new();
                let mut c = n.walk();
                for ch in n.children(&mut c) {
                    match ch.kind() {
                        "static" => is_static = true,
                        "asterisk" => on_demand = true,
                        "identifier" | "scoped_identifier" => {
                            name = text[ch.byte_range()].split_whitespace().collect()
                        }
                        _ => {}
                    }
                }
                // Only comments on the lines right above belong to the import.
                let mut leading = Vec::new();
                for r in pending.drain(..) {
                    let gap = &text[r.end..n.start_byte()];
                    if gap.matches('\n').count() <= 1
                        || out
                            .last()
                            .is_none_or(|l: &ImportNode| l.range.end < r.start)
                            && gap.trim().is_empty()
                            && gap.matches('\n').count() <= 1
                    {
                        leading.push(r);
                    }
                }
                out.push(ImportNode {
                    stub: ImportStub {
                        name,
                        on_demand,
                        is_static,
                    },
                    range: n.byte_range(),
                    leading_comments: leading,
                });
            }
            _ => pending.clear(),
        }
    }
    (out, package_end)
}

/// The layout entry index an import belongs to (5.6.2 step 1).
pub fn entry_of(settings: &ImportSettings, stub: &ImportStub) -> usize {
    let statics_separate = settings.layout_static_imports_separately;
    let is_static = stub.is_static && statics_separate;
    // The "package" the entries match: the package of a class import, the class of a static one.
    let subject: String = if stub.is_static || stub.on_demand {
        stub.name.clone()
    } else {
        stub.name
            .rsplit_once('.')
            .map(|(p, _)| p.to_string())
            .unwrap_or_default()
    };
    // For a single static import `a.B.m` the class is `a.B`.
    let subject = if stub.is_static && !stub.on_demand {
        subject
            .rsplit_once('.')
            .map(|(c, _)| c.to_string())
            .unwrap_or(subject)
    } else {
        subject
    };
    let mut best: Option<(usize, (u8, usize))> = None;
    let mut all_other = None;
    let mut all_other_static = None;
    for (i, e) in settings.layout.iter().enumerate() {
        match e {
            LayoutEntry::Package {
                name,
                subpackages,
                is_static: es,
            } if *es == is_static => {
                let exact = subject == *name;
                let below = *subpackages && subject.starts_with(&format!("{name}."));
                if exact || below {
                    let key = (
                        u8::from(exact && !*subpackages) + u8::from(exact),
                        name.matches('.').count(),
                    );
                    if best.as_ref().is_none_or(|(_, k)| key > *k) {
                        best = Some((i, key));
                    }
                }
            }
            LayoutEntry::AllOther => all_other = Some(i),
            LayoutEntry::AllOtherStatic => all_other_static = Some(i),
            _ => {}
        }
    }
    if let Some((i, _)) = best {
        return i;
    }
    if is_static {
        all_other_static.or(all_other).unwrap_or(0)
    } else {
        all_other.unwrap_or(0)
    }
}

/// Blank lines between two layout entries: the longest run of blanks in between (5.6.2 step 3).
fn blanks_between(settings: &ImportSettings, a: usize, b: usize) -> usize {
    let (lo, hi) = if a < b { (a, b) } else { (b, a) };
    let mut longest = 0;
    let mut run = 0;
    for e in &settings.layout[lo + 1..hi] {
        if *e == LayoutEntry::Blank {
            run += 1;
            longest = longest.max(run);
        } else {
            run = 0;
        }
    }
    longest
}

fn statement(stub: &ImportStub) -> String {
    format!(
        "import {}{}{};",
        if stub.is_static { "static " } else { "" },
        stub.name,
        if stub.on_demand { ".*" } else { "" }
    )
}

/// The smallest single edit turning `old` into `new`.
pub fn minimal_edit(old: &str, new: &str) -> Option<TextEdit> {
    if old == new {
        return None;
    }
    let ob = old.as_bytes();
    let nb = new.as_bytes();
    let mut p = 0;
    while p < ob.len() && p < nb.len() && ob[p] == nb[p] {
        p += 1;
    }
    while !old.is_char_boundary(p) || !new.is_char_boundary(p) {
        p -= 1;
    }
    let mut s = 0;
    while s < ob.len() - p && s < nb.len() - p && ob[ob.len() - 1 - s] == nb[nb.len() - 1 - s] {
        s += 1;
    }
    while !old.is_char_boundary(ob.len() - s) || !new.is_char_boundary(nb.len() - s) {
        s -= 1;
    }
    Some(TextEdit::new(
        p as u32..(ob.len() - s) as u32,
        new[p..nb.len() - s].to_string(),
    ))
}

fn newline_of(text: &str) -> &'static str {
    if text.contains("\r\n") { "\r\n" } else { "\n" }
}

/// Inserts the statement for `stub` at its place in the layout (5.6.2 step 4) and returns the new text.
pub fn insert_import(text: &str, stub: &ImportStub, settings: &ImportSettings) -> String {
    let (imports, package_end) = parse_imports(text);
    let nl = newline_of(text);
    let stmt = statement(stub);
    let entry = entry_of(settings, stub);
    let key = if stub.on_demand {
        format!("{}.", stub.name)
    } else {
        stub.name.clone()
    };
    let same_entry: Vec<&ImportNode> = imports
        .iter()
        .filter(|i| entry_of(settings, &i.stub) == entry)
        .collect();
    if let Some(first_greater) = same_entry.iter().find(|i| i.sort_key() > key) {
        // Before the first statement that sorts after it (with its leading comments).
        let at = first_greater
            .leading_comments
            .first()
            .map_or(first_greater.range.start, |r| r.start);
        return format!("{}{}{}{}", &text[..at], stmt, nl, &text[at..]);
    }
    if let Some(last) = same_entry.last() {
        return format!(
            "{}{}{}{}",
            &text[..last.range.end],
            nl,
            stmt,
            &text[last.range.end..]
        );
    }
    // Empty entry: after the previous non-empty entry, else before the next one.
    let used: BTreeSet<usize> = imports
        .iter()
        .map(|i| entry_of(settings, &i.stub))
        .collect();
    let prev = used.range(..entry).next_back().copied();
    let next = used.range(entry + 1..).next().copied();
    if let Some(p) = prev {
        let last = imports
            .iter()
            .filter(|i| entry_of(settings, &i.stub) == p)
            .max_by_key(|i| i.range.end)
            .expect("non-empty");
        let blanks = blanks_between(settings, p, entry);
        let sep = nl.repeat(blanks + 1);
        return format!(
            "{}{}{}{}",
            &text[..last.range.end],
            sep,
            stmt,
            &text[last.range.end..]
        );
    }
    if let Some(n) = next {
        let first = imports
            .iter()
            .filter(|i| entry_of(settings, &i.stub) == n)
            .min_by_key(|i| i.range.start)
            .expect("non-empty");
        let at = first
            .leading_comments
            .first()
            .map_or(first.range.start, |r| r.start);
        let blanks = blanks_between(settings, entry, n);
        return format!(
            "{}{}{}{}",
            &text[..at],
            stmt,
            nl.repeat(blanks + 1),
            &text[at..]
        );
    }
    // No imports at all: after the package statement, else at the top of the file.
    match package_end {
        Some(end) => format!("{}{}{}{}{}", &text[..end], nl, nl, stmt, &text[end..]),
        None => format!("{}{}{}{}", stmt, nl, nl, text),
    }
}

/// Removes the statement at `range` with its line.
fn delete_statement(text: &str, node: &ImportNode) -> String {
    let start_of = node
        .leading_comments
        .first()
        .map_or(node.range.start, |r| r.start);
    let line_start = text[..start_of].rfind('\n').map_or(0, |i| i + 1);
    let before_is_blank = text[line_start..start_of].trim().is_empty();
    let after = &text[node.range.end..];
    let eol = after.find('\n').map_or(after.len(), |i| i + 1);
    let rest_is_blank = after[..eol].trim().is_empty();
    if before_is_blank && rest_is_blank {
        format!("{}{}", &text[..line_start], &text[node.range.end + eol..])
    } else {
        format!("{}{}", &text[..start_of], &text[node.range.end..])
    }
}

pub fn delete_statement_public(text: &str, node: &ImportNode) -> String {
    delete_statement(text, node)
}

/// Replaces the first of `nodes` by `stub` in place and deletes the others; plain insertion when
/// there are none.
fn collapse_into(
    text: &str,
    nodes: &[&ImportNode],
    stub: &ImportStub,
    settings: &ImportSettings,
) -> String {
    let Some(first) = nodes.iter().min_by_key(|n| n.range.start) else {
        return insert_import(text, stub, settings);
    };
    let mut out = text.to_string();
    let mut rest: Vec<&&ImportNode> = nodes.iter().filter(|n| n.range != first.range).collect();
    rest.sort_by_key(|n| std::cmp::Reverse(n.range.start));
    // Back to front: later ranges first, then the first one is replaced.
    let mut ordered: Vec<(&ImportNode, bool)> = rest.into_iter().map(|n| (&**n, false)).collect();
    ordered.push((first, true));
    ordered.sort_by_key(|(n, _)| std::cmp::Reverse(n.range.start));
    for (n, replace) in ordered {
        if replace {
            out.replace_range(n.range.clone(), &statement(stub));
        } else {
            out = delete_statement(&out, n);
        }
    }
    out
}

/// Result of adding an import.
#[derive(Debug, Clone)]
pub struct AddOutcome {
    pub text: String,
    /// The import was already present or implicit: nothing to do.
    pub unchanged: bool,
}

/// `import static <class>.<member>;` per 5.4.3 step 2: skip when equivalent, collapse to `.*`
/// at the names threshold, else a single static import.
pub fn add_static_import(
    text: &str,
    class: &str,
    member: &str,
    settings: &ImportSettings,
    always_on_demand: bool,
) -> AddOutcome {
    let (imports, _) = parse_imports(text);
    let on_demand_present = imports
        .iter()
        .any(|i| i.stub.is_static && i.stub.on_demand && i.stub.name == class);
    let single_present = imports.iter().any(|i| {
        i.stub.is_static && !i.stub.on_demand && i.stub.name == format!("{class}.{member}")
    });
    if on_demand_present || single_present {
        return AddOutcome {
            text: text.to_string(),
            unchanged: true,
        };
    }
    let same_class: Vec<&ImportNode> = imports
        .iter()
        .filter(|i| {
            i.stub.is_static
                && !i.stub.on_demand
                && i.stub.name.rsplit_once('.').map(|(c, _)| c) == Some(class)
        })
        .collect();
    let k = same_class.len();
    let collapse = always_on_demand || k + 1 >= settings.names_count_to_use_static_import_on_demand;
    if collapse {
        let stub = ImportStub {
            name: class.to_string(),
            on_demand: true,
            is_static: true,
        };
        return AddOutcome {
            text: collapse_into(text, &same_class, &stub, settings),
            unchanged: false,
        };
    }
    let stub = ImportStub {
        name: format!("{class}.{member}"),
        on_demand: false,
        is_static: true,
    };
    AddOutcome {
        text: insert_import(text, &stub, settings),
        unchanged: false,
    }
}

/// `import <fqn>;` for a class (5.6.3): not for `java.lang` and the own package (JAVA-IMP-044);
/// on demand once the class-count threshold is reached or the package is always on demand.
pub fn add_class_import(
    text: &str,
    fqn: &str,
    own_package: &str,
    settings: &ImportSettings,
) -> AddOutcome {
    let (package, _) = fqn.rsplit_once('.').unwrap_or(("", fqn));
    if package == "java.lang" || package == own_package || package.is_empty() {
        return AddOutcome {
            text: text.to_string(),
            unchanged: true,
        };
    }
    let (imports, _) = parse_imports(text);
    if imports.iter().any(|i| {
        !i.stub.is_static
            && ((!i.stub.on_demand && i.stub.name == fqn)
                || (i.stub.on_demand && i.stub.name == package))
    }) {
        return AddOutcome {
            text: text.to_string(),
            unchanged: true,
        };
    }
    let singles_from_package = imports
        .iter()
        .filter(|i| {
            !i.stub.is_static
                && !i.stub.on_demand
                && i.stub.name.rsplit_once('.').map(|(p, _)| p) == Some(package)
        })
        .count();
    let on_demand_pkg = settings
        .packages_to_use_import_on_demand
        .iter()
        .any(|p| p == package);
    let use_on_demand = !settings.use_single_class_imports
        || on_demand_pkg
        || singles_from_package + 1 >= settings.class_count_to_use_import_on_demand;
    if use_on_demand {
        let nodes: Vec<&ImportNode> = imports
            .iter()
            .filter(|i| {
                !i.stub.is_static
                    && !i.stub.on_demand
                    && i.stub.name.rsplit_once('.').map(|(p, _)| p) == Some(package)
            })
            .collect();
        let stub = ImportStub {
            name: package.to_string(),
            on_demand: true,
            is_static: false,
        };
        return AddOutcome {
            text: collapse_into(text, &nodes, &stub, settings),
            unchanged: false,
        };
    }
    let stub = ImportStub {
        name: fqn.to_string(),
        on_demand: false,
        is_static: false,
    };
    AddOutcome {
        text: insert_import(text, &stub, settings),
        unchanged: false,
    }
}

// ---- Optimize Imports -----------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Optimized {
    pub text: String,
    pub removed: usize,
    pub added: usize,
}

impl Optimized {
    /// The notification text of 5.6.4 step 8; `None` when nothing changed.
    pub fn message(&self, changed: bool) -> Option<String> {
        if !changed {
            return None;
        }
        if self.removed == 0 {
            return Some("Rearranged imports".into());
        }
        let mut m = format!(
            "Removed {} import{}",
            self.removed,
            if self.removed == 1 { "" } else { "s" }
        );
        if self.added > 0 {
            m.push_str(&format!(
                ", added {} import{}",
                self.added,
                if self.added == 1 { "" } else { "s" }
            ));
        }
        Some(m)
    }
}

/// Optimize Imports (JAVA-IMP-052..057). `usage` comes from the analysis of the same text.
pub fn optimize(
    text: &str,
    usage: &Usage,
    own_package: &str,
    env: &Env<'_>,
    settings: &ImportSettings,
) -> Optimized {
    let (imports, package_end) = parse_imports(text);
    if imports.is_empty() {
        return Optimized {
            text: text.to_string(),
            removed: 0,
            added: 0,
        };
    }
    let nl = newline_of(text);
    // 1. used names. Unresolved imports are kept as placeholders (JAVA-IMP-057).
    let mut classes: BTreeSet<String> = usage.classes.iter().cloned().collect();
    let mut statics: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (c, m) in &usage.statics {
        statics.entry(c.clone()).or_default().insert(m.clone());
    }
    let mut placeholders: Vec<ImportStub> = Vec::new();
    for imp in &imports {
        let resolves = if imp.stub.is_static {
            let class = if imp.stub.on_demand {
                imp.stub.name.clone()
            } else {
                imp.stub
                    .name
                    .rsplit_once('.')
                    .map(|(c, _)| c.to_string())
                    .unwrap_or_default()
            };
            env.find_class(&class).ok().flatten().is_some()
        } else if imp.stub.on_demand {
            env.package_exists(&imp.stub.name).unwrap_or(true)
                || env.find_class(&imp.stub.name).ok().flatten().is_some()
        } else {
            env.find_class(&imp.stub.name).ok().flatten().is_some()
        };
        if !resolves {
            placeholders.push(imp.stub.clone());
        }
    }
    // A name used as a method or field through an existing static import that nobody could resolve stays too.
    for (i, imp) in imports.iter().enumerate() {
        let _ = i;
        if imp.stub.is_static && !imp.stub.on_demand {
            let (class, member) = imp
                .stub
                .name
                .rsplit_once('.')
                .unwrap_or(("", &imp.stub.name));
            if usage.unresolved.contains(member)
                && !statics.get(class).is_some_and(|s| s.contains(member))
            {
                statics
                    .entry(class.to_string())
                    .or_default()
                    .insert(member.to_string());
            }
        }
        if !imp.stub.is_static && !imp.stub.on_demand {
            let short = imp.stub.name.rsplit('.').next().unwrap_or("");
            if usage.unresolved.contains(short) {
                classes.insert(imp.stub.name.clone());
            }
        }
    }
    // Own-package and java.lang classes need no import (JAVA-IMP-055).
    classes.retain(|c| {
        let p = c.rsplit_once('.').map(|(p, _)| p).unwrap_or("");
        p != own_package && !(p == "java.lang")
    });
    // Nested classes are imported through their own name only when asked; otherwise the outer is.
    let mut names: BTreeSet<(bool, String, String)> = BTreeSet::new(); // (static, class/package, member or simple name)
    for c in &classes {
        let (p, n) = c.rsplit_once('.').unwrap_or(("", c));
        names.insert((false, p.to_string(), n.to_string()));
    }
    for (c, ms) in &statics {
        for m in ms {
            names.insert((true, c.clone(), m.clone()));
        }
    }
    // 3. on-demand decisions.
    let mut per_group: BTreeMap<(bool, String), Vec<String>> = BTreeMap::new();
    for (st, g, n) in &names {
        per_group
            .entry((*st, g.clone()))
            .or_default()
            .push(n.clone());
    }
    let mut on_demand: BTreeSet<(bool, String)> = BTreeSet::new();
    for ((st, g), ns) in &per_group {
        let limit = if *st {
            settings.names_count_to_use_static_import_on_demand
        } else {
            settings.class_count_to_use_import_on_demand
        };
        let always = !*st
            && settings
                .packages_to_use_import_on_demand
                .iter()
                .any(|p| p == g);
        if !settings.use_single_class_imports || ns.len() >= limit || always {
            on_demand.insert((*st, g.clone()));
        }
    }
    // Existing on-demand imports whose package still provides something used are kept as they were.
    // 4. conflicts: names that on-demand imports would make ambiguous or change.
    let mut force_single: BTreeSet<(bool, String, String)> = BTreeSet::new();
    let static_member_groups: BTreeMap<String, Vec<&String>> = {
        let mut m: BTreeMap<String, Vec<&String>> = BTreeMap::new();
        for (st, g, n) in &names {
            if *st {
                m.entry(n.clone()).or_default().push(g);
            }
        }
        m
    };
    for (st, g) in &on_demand {
        if *st {
            // The same member name from several classes: neither is imported on demand.
            for n in per_group.get(&(true, g.clone())).into_iter().flatten() {
                if static_member_groups.get(n).is_some_and(|gs| gs.len() > 1) {
                    force_single.insert((true, g.clone(), n.clone()));
                }
            }
            continue;
        }
        for n in per_group.get(&(false, g.clone())).into_iter().flatten() {
            let in_pkg = if own_package.is_empty() {
                n.clone()
            } else {
                format!("{own_package}.{n}")
            };
            let clash_same_pkg = env.find_class(&in_pkg).ok().flatten().is_some();
            let clash_lang = g != "java.lang"
                && env
                    .find_class(&format!("java.lang.{n}"))
                    .ok()
                    .flatten()
                    .is_some();
            let clash_other = on_demand.iter().any(|(s2, g2)| {
                !*s2 && g2 != g
                    && env
                        .find_class(&format!("{g2}.{n}"))
                        .ok()
                        .flatten()
                        .is_some()
            });
            // A name that is also provided by a different group in `names` (like Date from util and sql).
            let clash_used = names.iter().any(|(s2, g2, n2)| !*s2 && n2 == n && g2 != g);
            if clash_same_pkg || clash_lang || clash_other || clash_used {
                force_single.insert((false, g.clone(), n.clone()));
            }
        }
    }
    // 5. a group whose names all stayed single is not on demand.
    let mut final_stubs: Vec<ImportStub> = Vec::new();
    for ((st, g), ns) in &per_group {
        let singles: Vec<&String> = ns
            .iter()
            .filter(|n| force_single.contains(&(*st, g.clone(), (*n).clone())))
            .collect();
        let is_on_demand = on_demand.contains(&(*st, g.clone())) && singles.len() < ns.len();
        if is_on_demand {
            final_stubs.push(ImportStub {
                name: g.clone(),
                on_demand: true,
                is_static: *st,
            });
            for n in singles {
                final_stubs.push(ImportStub {
                    name: format!("{g}.{n}"),
                    on_demand: false,
                    is_static: *st,
                });
            }
        } else {
            for n in ns {
                final_stubs.push(ImportStub {
                    name: format!("{g}.{n}"),
                    on_demand: false,
                    is_static: *st,
                });
            }
        }
    }
    for p in placeholders {
        if !final_stubs.contains(&p) {
            final_stubs.push(p);
        }
    }
    // 2./6. group by layout entry, sort, render.
    let mut by_entry: BTreeMap<usize, Vec<ImportStub>> = BTreeMap::new();
    for s in final_stubs {
        by_entry.entry(entry_of(settings, &s)).or_default().push(s);
    }
    let mut rendered = String::new();
    let mut prev_entry: Option<usize> = None;
    // Comments attached to kept imports stay (JAVA-IMP-058).
    let comment_for = |stub: &ImportStub| -> Option<String> {
        imports
            .iter()
            .find(|i| i.stub == *stub && !i.leading_comments.is_empty())
            .map(|i| {
                i.leading_comments
                    .iter()
                    .map(|r| text[r.clone()].to_string())
                    .collect::<Vec<_>>()
                    .join(nl)
            })
    };
    for (entry, mut list) in by_entry {
        list.sort_by(|a, b| {
            let ka = if a.on_demand {
                format!("{}.", a.name)
            } else {
                a.name.clone()
            };
            let kb = if b.on_demand {
                format!("{}.", b.name)
            } else {
                b.name.clone()
            };
            ka.cmp(&kb)
        });
        if let Some(p) = prev_entry {
            rendered.push_str(&nl.repeat(blanks_between(settings, p, entry) + 1));
        }
        for (i, s) in list.iter().enumerate() {
            if i > 0 {
                rendered.push_str(nl);
            }
            if let Some(c) = comment_for(s) {
                rendered.push_str(&c);
                rendered.push_str(nl);
            }
            rendered.push_str(&statement(s));
        }
        prev_entry = Some(entry);
    }
    let first = imports
        .iter()
        .map(|i| {
            i.leading_comments
                .first()
                .map_or(i.range.start, |r| r.start)
        })
        .min()
        .unwrap_or(0);
    let last = imports.iter().map(|i| i.range.end).max().unwrap_or(0);
    let mut new_text = if rendered.is_empty() {
        // Everything removed: drop the whole import block including one adjoining blank line.
        let head = text[..first].trim_end_matches([' ', '\t']);
        let tail = text[last..].trim_start_matches([' ', '\t', '\r', '\n']);
        let head_trim = head.trim_end_matches(['\r', '\n']);
        if package_end.is_some() || !head_trim.is_empty() {
            format!("{head_trim}{nl}{nl}{tail}")
        } else {
            tail.to_string()
        }
    } else {
        format!("{}{}{}", &text[..first], rendered, &text[last..])
    };
    if rendered.is_empty() && new_text.trim().is_empty() {
        new_text = String::new();
    }
    let old: BTreeSet<&ImportStub> = imports.iter().map(|i| &i.stub).collect();
    let (final_imports, _) = parse_imports(&new_text);
    let new: BTreeSet<&ImportStub> = final_imports.iter().map(|i| &i.stub).collect();
    Optimized {
        removed: old.difference(&new).count(),
        added: new.difference(&old).count(),
        text: new_text,
    }
}

/// The imports reported by the "Unused import" inspection (JAVA-IMP-070, 072, 074): not used,
/// or from the file's own package. Empty when the analysis is not reliable (JAVA-IMP-076).
pub fn unused_imports(
    text: &str,
    usage: &Usage,
    own_package: &str,
    env: &Env<'_>,
) -> Vec<ImportNode> {
    match source::parse(text) {
        Some(tree) => unused_imports_in(text, &tree, usage, own_package, env),
        None => Vec::new(),
    }
}

/// [`unused_imports`] over a tree the caller already has.
pub fn unused_imports_in(
    text: &str,
    tree: &tree_sitter::Tree,
    usage: &Usage,
    own_package: &str,
    env: &Env<'_>,
) -> Vec<ImportNode> {
    if usage.uncertain {
        return Vec::new();
    }
    let (imports, _) = parse_imports_in(text, tree);
    let mut out = Vec::new();
    for (i, imp) in imports.into_iter().enumerate() {
        let st = &imp.stub;
        // Redundant: single or on-demand import of the file's own package.
        if !st.is_static {
            let pkg = if st.on_demand {
                st.name.clone()
            } else {
                st.name
                    .rsplit_once('.')
                    .map(|(p, _)| p.to_string())
                    .unwrap_or_default()
            };
            if pkg == own_package && !own_package.is_empty() {
                // A class of the own package named by a single import that is a nested class is not redundant.
                let is_member = !st.on_demand
                    && env
                        .find_class(&st.name)
                        .ok()
                        .flatten()
                        .is_some_and(|h| !h.path.is_empty());
                if !is_member {
                    out.push(imp);
                    continue;
                }
            }
        }
        // `import` statements that failed to resolve are errors elsewhere, not unused.
        let used = usage_has(usage, i, &imp.stub);
        if !used {
            // Unresolved names could belong to this import (JAVA-IMP-076).
            let short = st.name.rsplit('.').next().unwrap_or("");
            let by_unresolved = if st.on_demand {
                usage.unresolved_any
            } else {
                usage.unresolved.contains(short)
            };
            if !by_unresolved {
                out.push(imp);
            }
        }
    }
    out
}

fn usage_has(usage: &Usage, index: usize, _stub: &ImportStub) -> bool {
    usage.used.contains(&index)
}
