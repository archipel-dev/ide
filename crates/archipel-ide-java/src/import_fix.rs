//! The import fixes of spec 05: candidate search, ordering and the quick fixes registered on
//! `Cannot resolve method` (`Import static method...`) and `Cannot resolve symbol`
//! (`Import class`): JAVA-IMP-003..007, 010..020, 030..033, 040..045.

use crate::api::JavaProject;
use crate::env::{Ctx, Env, Res, TypeRes};
use crate::import_edit::{self, ImportSettings};
use crate::index::{ClassHit, MemberHit, UnitKind};
use crate::stub::*;
use crate::ty::{self, Ty};
use archipel_ide_analysis::inspect::ProblemDescriptor;
use archipel_ide_analysis::intention::{FixCx, Priority, QuickFix};
use archipel_ide_analysis::modcommand::ChoiceRow;
use archipel_ide_analysis::{Cancelled, ModCommand, TextEdit};
use std::cmp::Ordering;
use std::path::PathBuf;
use std::sync::Arc;

/// Where the fix is computed (the place of 5.2: the file and its position in a class).
#[derive(Debug, Clone)]
pub struct Place {
    pub path: PathBuf,
    pub package: String,
    /// Qualified name of the top-level class that contains the place.
    pub top_class: Option<String>,
    /// Qualified names of the classes enclosing the place that are subclasses (protected access).
    pub module: Option<String>,
    pub test: bool,
    pub imports: Vec<ImportStub>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Fit {
    None,
    Partly,
    Applicable,
}

#[derive(Debug, Clone)]
pub struct StaticCandidate {
    pub class: ClassHit,
    pub member: usize,
    pub is_field: bool,
    pub fit: Fit,
}

impl StaticCandidate {
    pub fn class_fqn(&self) -> &str {
        &self.class.class().qname
    }
    pub fn name(&self) -> &str {
        let c = self.class.class();
        if self.is_field {
            &c.fields[self.member].name
        } else {
            &c.methods[self.member].name
        }
    }
    /// `org.assertj.core.api.Assertions.assertThat`
    pub fn qualified(&self) -> String {
        format!("{}.{}", self.class_fqn(), self.name())
    }
    /// `Assertions.assertThat` (nested: `Outer.Inner.member`).
    pub fn display(&self) -> String {
        let c = self.class.class();
        let rel = c
            .qname
            .strip_prefix(&format!("{}.", self.class.package()))
            .unwrap_or(&c.qname)
            .to_string();
        format!("{rel}.{}", self.name())
    }
    pub fn package(&self) -> &str {
        self.class.package()
    }
    fn deprecated(&self) -> bool {
        let c = self.class.class();
        let m = if self.is_field {
            c.fields[self.member].deprecated
        } else {
            c.methods[self.member].deprecated
        };
        m || c.deprecated || self.class.record.class.deprecated
    }
    fn signature(&self) -> String {
        let c = self.class.class();
        if self.is_field {
            c.fields[self.member].name.clone()
        } else {
            let m = &c.methods[self.member];
            format!(
                "{}({})",
                m.name,
                m.params
                    .iter()
                    .map(|p| p.ty.as_str())
                    .collect::<Vec<_>>()
                    .join(",")
            )
        }
    }
}

// ---- applicability (5.2.3, JAVA-IMP-018) -----------------------------------------------------

#[derive(Debug, Clone)]
enum PTy {
    Prim(String),
    Ref(String),
    Array(Box<PTy>),
    /// A type variable with its erased bound (`None` = Object).
    Var(Option<String>),
    Unknown,
}

fn param_ty(env: &Env<'_>, owner: &ClassHit, m: &MethodStub, text: &str) -> Res<PTy> {
    let mut ctx: Ctx<'_> = env.ctx_of(owner);
    ctx.tparams = m.type_params.iter().map(|t| t.name.clone()).collect();
    let parts = ty::split(text);
    let base = match env.resolve_type_text(&parts.base, &ctx)? {
        TypeRes::Class(h) => PTy::Ref(h.class().qname.clone()),
        TypeRes::Prim(p) => PTy::Prim(p),
        TypeRes::TypeVar(v) => {
            // Bound of a method or class type variable.
            let bound = m
                .type_params
                .iter()
                .find(|t| t.name == v)
                .or_else(|| owner.class().type_params.iter().find(|t| t.name == v))
                .and_then(|t| t.bounds.first().cloned());
            match bound {
                Some(b) => match env.resolve_type_text(&ty::split(&b).base, &ctx)? {
                    TypeRes::Class(h) => PTy::Var(Some(h.class().qname.clone())),
                    _ => PTy::Unknown,
                },
                None => PTy::Var(None),
            }
        }
        _ => PTy::Unknown,
    };
    let mut t = base;
    for _ in 0..parts.dims {
        t = PTy::Array(Box::new(t));
    }
    Ok(t)
}

fn subtype(env: &Env<'_>, sub: &str, sup: &str) -> Res<Option<bool>> {
    if sub == sup || sup == "java.lang.Object" {
        return Ok(Some(true));
    }
    match env.find_class(sub)? {
        Some(h) => env.is_subclass(&h, sup),
        None => Ok(None),
    }
}

/// `Some(true)` assignable, `Some(false)` not, `None` unknown (treated as applicable).
fn assignable(env: &Env<'_>, arg: &Ty, to: &PTy) -> Res<Option<bool>> {
    Ok(match (arg, to) {
        (Ty::Unknown | Ty::Var(_), _) | (_, PTy::Unknown) => Some(true),
        (Ty::Null, PTy::Prim(_)) => Some(false),
        (Ty::Null, _) => Some(true),
        (Ty::Prim(p), PTy::Prim(q)) => Some(ty::prim_widens(p, q)),
        (Ty::Prim(p), PTy::Ref(f)) => subtype(env, ty::boxed_name(p), f)?,
        (Ty::Prim(p), PTy::Var(b)) => match b {
            Some(b) => subtype(env, ty::boxed_name(p), b)?,
            None => Some(true),
        },
        (Ty::Prim(_), PTy::Array(_)) => Some(false),
        (Ty::Ref { fqn, .. }, PTy::Prim(q)) => match fqn.as_deref().and_then(ty::unboxed_name) {
            Some(u) => Some(ty::prim_widens(u, q)),
            None => {
                if fqn.is_some() {
                    Some(false)
                } else {
                    None
                }
            }
        },
        (Ty::Ref { fqn: Some(f), .. }, PTy::Ref(t)) => subtype(env, f, t)?,
        (Ty::Ref { fqn: Some(f), .. }, PTy::Var(b)) => match b {
            Some(b) => subtype(env, f, b)?,
            None => Some(true),
        },
        (Ty::Ref { fqn: Some(_), .. }, PTy::Array(_)) => Some(false),
        (Ty::Ref { fqn: None, .. }, _) => Some(true),
        (Ty::Array(_), PTy::Ref(t)) => Some(matches!(
            t.as_str(),
            "java.lang.Object" | "java.lang.Cloneable" | "java.io.Serializable"
        )),
        (Ty::Array(_), PTy::Var(b)) => Some(b.as_deref().is_none_or(|b| b == "java.lang.Object")),
        (Ty::Array(_), PTy::Prim(_)) => Some(false),
        (Ty::Array(a), PTy::Array(t)) => match (&**a, &**t) {
            (Ty::Prim(p), PTy::Prim(q)) => Some(p == q),
            (Ty::Prim(_), _) | (_, PTy::Prim(_)) => Some(false),
            (a, t) => assignable(env, a, t)?,
        },
    })
}

/// Whether `args` fit the parameters of method `m` of `owner` (arity incl. varargs, loose conversion).
pub fn applicable(env: &Env<'_>, owner: &ClassHit, m: &MethodStub, args: &[Ty]) -> Res<bool> {
    let n = m.params.len();
    let arity_ok = n == args.len() || (m.varargs && args.len() + 1 >= n);
    if !arity_ok {
        return Ok(false);
    }
    let mut p: Vec<PTy> = Vec::with_capacity(n);
    for prm in &m.params {
        p.push(param_ty(env, owner, m, &prm.ty)?);
    }
    // Fixed arity first.
    if n == args.len() {
        let mut ok = true;
        for (a, t) in args.iter().zip(&p) {
            if assignable(env, a, t)? == Some(false) {
                ok = false;
                break;
            }
        }
        if ok {
            return Ok(true);
        }
        if !m.varargs {
            return Ok(false);
        }
    }
    // Variable arity: the last parameter's element type takes the remaining arguments.
    let Some(PTy::Array(elem)) = p.last().cloned() else {
        return Ok(false);
    };
    for (i, a) in args.iter().enumerate() {
        let target: &PTy = if i + 1 < n { &p[i] } else { &elem };
        if assignable(env, a, target)? == Some(false) {
            return Ok(false);
        }
    }
    Ok(true)
}

// ---- static member candidates (5.2.2) -------------------------------------------------------------

pub struct SearchInput<'a> {
    pub name: &'a str,
    /// `Some` for calls (method flavor), `None` for constants (field flavor).
    pub args: Option<&'a [Ty]>,
    /// Expected types of the place (assignment target, return type, ...).
    pub expected: &'a [Ty],
}

fn is_project_source(project: &JavaProject, c: &ClassHit) -> bool {
    project.index.origin(c.unit).kind == UnitKind::Source
}

/// The candidate search of 5.2.2 / 5.2.4 for `Import static method` and `Import static constant`.
pub fn static_candidates(
    project: &JavaProject,
    env: &Env<'_>,
    place: &Place,
    input: &SearchInput<'_>,
    settings: &ImportSettings,
) -> Res<Vec<StaticCandidate>> {
    let method_flavor = input.args.is_some();
    let cur_unit = env.current.and_then(|c| c.unit);
    let hits: Vec<MemberHit> = if method_flavor {
        project.index.methods_named(input.name, env.scope)?
    } else {
        project.index.fields_named(input.name, env.scope)?
    };
    // 3. filters, grouping by class in discovery order, capped at 100 classes (JAVA-IMP-017).
    let mut groups: Vec<(String, ClassHit, Vec<usize>)> = Vec::new();
    for h in hits {
        if Some(h.class.unit) == cur_unit {
            continue;
        }
        let class = h.class.class();
        let fqn = class.qname.clone();
        if method_flavor {
            let m = &class.methods[h.index];
            if m.is_ctor {
                continue; // JAVA-STUB-013
            }
            // Interface methods need an explicit `static` (JAVA-IMP-011).
            if class.is_interface_like() && m.flags & flags::STATIC == 0 {
                continue;
            }
        } else if class.fields[h.index].flags & flags::STATIC == 0 && !class.is_interface_like() {
            continue;
        }
        if settings.is_excluded(&format!("{fqn}.{}", input.name)) || settings.is_excluded(&fqn) {
            continue;
        }
        if h.class.package().is_empty() {
            continue; // the default package is not importable
        }
        match groups.iter_mut().find(|(f, _, _)| *f == fqn) {
            Some((_, _, members)) => members.push(h.index),
            None => {
                if groups.len() >= 100 {
                    continue;
                }
                groups.push((fqn, h.class.clone(), vec![h.index]));
            }
        }
    }
    // 4. per class.
    let mut applicable_list: Vec<StaticCandidate> = Vec::new();
    let mut partly_list: Vec<StaticCandidate> = Vec::new();
    let mut all: Vec<StaticCandidate> = Vec::new();
    for (_, class, members) in groups {
        let c = class.class();
        let statics: Vec<usize> = members
            .iter()
            .copied()
            .filter(|i| {
                if method_flavor {
                    c.methods[*i].flags & flags::STATIC != 0
                } else {
                    c.fields[*i].flags & flags::STATIC != 0 || c.is_interface_like()
                }
            })
            .collect();
        let Some(&first) = statics.first() else {
            continue;
        };
        let mut fallback = StaticCandidate {
            class: class.clone(),
            member: first,
            is_field: !method_flavor,
            fit: Fit::None,
        };
        let mut best: Option<StaticCandidate> = None;
        let mut done = false;
        for i in statics {
            let flags_i = if method_flavor {
                c.methods[i].flags
            } else {
                c.fields[i].flags
            };
            let accessible = env.accessible(
                &class,
                flags_i,
                &place.package,
                place.top_class.as_deref(),
                false,
            );
            if !accessible {
                continue;
            }
            let fit = if method_flavor {
                let m = &c.methods[i];
                if !applicable(env, &class, m, input.args.unwrap_or(&[]))? {
                    continue;
                }
                expected_fit(
                    env,
                    &class,
                    m.ret.as_deref(),
                    &m.type_params,
                    input.expected,
                )?
            } else {
                expected_fit(env, &class, Some(&c.fields[i].ty), &[], input.expected)?
            };
            let cand = StaticCandidate {
                class: class.clone(),
                member: i,
                is_field: !method_flavor,
                fit,
            };
            match fit {
                Fit::Applicable => {
                    applicable_list.push(cand);
                    done = true;
                    break;
                }
                Fit::Partly => {
                    if best.is_none() {
                        best = Some(cand);
                    }
                }
                Fit::None => {}
            }
        }
        if done {
            continue;
        }
        if let Some(b) = best {
            fallback = StaticCandidate {
                fit: Fit::Partly,
                ..b.clone()
            };
            partly_list.push(b);
        }
        all.push(fallback);
    }
    // 5/6. result lists.
    let sort = |v: &mut Vec<StaticCandidate>| {
        v.sort_by(|a, b| compare(project, env, place, settings, a, b));
    };
    sort(&mut applicable_list);
    sort(&mut partly_list);
    let mut result: Vec<StaticCandidate> = Vec::new();
    result.extend(applicable_list);
    for p in partly_list {
        if !result.iter().any(|r| r.class_fqn() == p.class_fqn()) {
            result.push(p);
        }
    }
    if result.is_empty() {
        sort(&mut all);
        result = all;
    }
    // Members that are inaccessible from the file and not project code are removed.
    result.retain(|c| {
        let flags_c = if c.is_field {
            c.class.class().fields[c.member].flags
        } else {
            c.class.class().methods[c.member].flags
        };
        env.accessible(
            &c.class,
            flags_c,
            &place.package,
            place.top_class.as_deref(),
            false,
        ) || is_project_source(project, &c.class)
    });
    Ok(result)
}

fn expected_fit(
    env: &Env<'_>,
    owner: &ClassHit,
    ret: Option<&str>,
    tparams: &[TypeParamStub],
    expected: &[Ty],
) -> Res<Fit> {
    if expected.is_empty() {
        return Ok(Fit::Applicable);
    }
    let Some(ret) = ret else {
        return Ok(Fit::Applicable);
    };
    let mut m = MethodStub {
        name: String::new(),
        flags: 0,
        type_params: tparams.to_vec(),
        ret: None,
        params: Vec::new(),
        throws: Vec::new(),
        annotations: Vec::new(),
        is_ctor: false,
        varargs: false,
        deprecated: false,
        has_default: false,
    };
    m.type_params = tparams.to_vec();
    let rt = param_ty(env, owner, &m, ret)?;
    if matches!(rt, PTy::Var(_) | PTy::Unknown) || ret == "void" {
        return Ok(Fit::Applicable);
    }
    // Convert the return type to a `Ty` for the assignability test.
    let as_ty = match &rt {
        PTy::Prim(p) => Ty::Prim(p.clone()),
        PTy::Ref(f) => Ty::Ref {
            fqn: Some(f.clone()),
            display: ty::display(f),
            args: Vec::new(),
        },
        PTy::Array(_) => Ty::Unknown,
        _ => Ty::Unknown,
    };
    for e in expected {
        let target = match e {
            Ty::Prim(p) => PTy::Prim(p.clone()),
            Ty::Ref { fqn: Some(f), .. } => PTy::Ref(f.clone()),
            _ => return Ok(Fit::Applicable),
        };
        if assignable(env, &as_ty, &target)? != Some(false) {
            return Ok(Fit::Applicable);
        }
    }
    Ok(Fit::None)
}

// ---- ordering (5.3) -----------------------------------------------------------------------------

fn discouraged(fqn: &str) -> bool {
    fqn.starts_with("com.sun.")
        || fqn.starts_with("sun.")
        || fqn.starts_with("org.omg.")
        || fqn == "java.awt.List"
        || fqn.starts_with("jdk.internal.")
}

fn known_element(fqn: &str, place: &Place, is_class: bool) -> i32 {
    let (pkg, name) = fqn.rsplit_once('.').unwrap_or(("", fqn));
    let mut v = if pkg == "java.lang"
        && matches!(
            name,
            "String"
                | "Class"
                | "System"
                | "Runnable"
                | "Exception"
                | "Throwable"
                | "RuntimeException"
        )
        || pkg == "java.util" && matches!(name, "ArrayList" | "HashMap" | "HashSet")
    {
        8
    } else if pkg == "java.util" {
        7
    } else if pkg == "java.lang" {
        6
    } else if pkg.starts_with("java.lang.") {
        5
    } else if pkg.starts_with("java.util.") {
        4
    } else if pkg == "javax.swing" || pkg.starts_with("javax.swing.") {
        3
    } else if pkg.starts_with("java.") {
        2
    } else if pkg.starts_with("javax.") {
        1
    } else {
        0
    };
    if (fqn.starts_with("com.") || fqn.starts_with("net.")) && v == 0 {
        v = -1;
    }
    if place.test && is_class && (fqn.contains("junit") || fqn.contains("test")) {
        v = v.max(1);
    }
    if fqn.starts_with("org.testng.internal") {
        v = -1;
    }
    v
}

/// The proximity chain of 5.3.1 for ranks 3, 4, 9, 10, 11, 12 and 13 (higher is better).
fn proximity(
    project: &JavaProject,
    place: &Place,
    class: &ClassHit,
    deprecated: bool,
    accessible: bool,
    is_class: bool,
) -> [i32; 7] {
    let fqn = class.class().qname.as_str();
    let access = if !accessible {
        0
    } else if deprecated {
        1
    } else if discouraged(fqn) {
        2
    } else {
        3
    };
    let pkg = class.package();
    let explicit = if place
        .imports
        .iter()
        .any(|i| i.is_static && !i.on_demand && i.name.starts_with(&format!("{fqn}.")))
    {
        11
    } else if pkg == place.package && class.path.is_empty() {
        9
    } else if place
        .imports
        .iter()
        .any(|i| !i.is_static && !i.on_demand && i.name == fqn)
    {
        8
    } else if pkg == "java.lang" {
        6
    } else if place
        .imports
        .iter()
        .any(|i| !i.is_static && i.on_demand && i.name == pkg)
    {
        5
    } else if place.imports.iter().any(|i| {
        !i.is_static && !i.on_demand && i.name.rsplit_once('.').map(|(p, _)| p) == Some(pkg)
    }) {
        1
    } else {
        0
    };
    let origin = project.index.origin(class.unit);
    let module = match (&origin.module, &place.module) {
        (Some(a), Some(b)) if a == b => 2,
        (Some(_), _) => 1,
        _ => 0,
    };
    let dir = match (&origin.kind, origin.test) {
        (UnitKind::Source, false) => 3,
        (UnitKind::Source, true) => 2,
        _ => 0,
    };
    let jdk = i32::from(origin.kind == UnitKind::Jdk);
    [
        access,
        explicit,
        module,
        known_element(fqn, place, is_class),
        1,
        dir,
        jdk,
    ]
}

/// The comparator of 5.3 with the deterministic Archipel tie-break (JAVA-IMP-020).
fn compare(
    project: &JavaProject,
    env: &Env<'_>,
    place: &Place,
    settings: &ImportSettings,
    a: &StaticCandidate,
    b: &StaticCandidate,
) -> Ordering {
    // 1. not deprecated before deprecated
    let o = a.deprecated().cmp(&b.deprecated());
    if o != Ordering::Equal {
        return o;
    }
    // 2. top-level before nested
    let o = (!a.class.path.is_empty()).cmp(&(!b.class.path.is_empty()));
    if o != Ordering::Equal {
        return o;
    }
    // 3. usage recency, when both have a record
    let ra = settings.recent.iter().rposition(|r| *r == a.qualified());
    let rb = settings.recent.iter().rposition(|r| *r == b.qualified());
    if let (Some(x), Some(y)) = (ra, rb) {
        let o = y.cmp(&x);
        if o != Ordering::Equal {
            return o;
        }
    }
    // 4. proximity
    let acc = |c: &StaticCandidate| {
        let cl = c.class.class();
        let f = if c.is_field {
            cl.fields[c.member].flags
        } else {
            cl.methods[c.member].flags
        };
        env.accessible(
            &c.class,
            f,
            &place.package,
            place.top_class.as_deref(),
            false,
        )
    };
    let pa = proximity(project, place, &a.class, a.deprecated(), acc(a), false);
    let pb = proximity(project, place, &b.class, b.deprecated(), acc(b), false);
    let o = pb.cmp(&pa);
    if o != Ordering::Equal {
        return o;
    }
    // Archipel tie-break: containing class name case-insensitively, then the signature.
    a.class_fqn()
        .to_lowercase()
        .cmp(&b.class_fqn().to_lowercase())
        .then_with(|| a.class_fqn().cmp(b.class_fqn()))
        .then_with(|| a.signature().cmp(&b.signature()))
}

// ---- class candidates (5.5.2) -----------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct ClassRefInfo {
    /// The reference is `Foo.member` / `Foo::member`: the member name and whether it must be static.
    pub qualified_member: Option<(String, bool)>,
    pub annotation: bool,
    /// Qualifier written before the name (`util.List`), when it is a package-like prefix.
    pub package_qualifier: Option<String>,
    pub type_args: bool,
    /// Simple names of single imports in the file that do not resolve.
    pub unresolved_import_names: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ClassCandidate {
    pub class: ClassHit,
}

impl ClassCandidate {
    pub fn fqn(&self) -> &str {
        &self.class.class().qname
    }
}

pub fn class_candidates(
    project: &JavaProject,
    env: &Env<'_>,
    place: &Place,
    name: &str,
    info: &ClassRefInfo,
    settings: &ImportSettings,
) -> Res<Vec<ClassCandidate>> {
    if settings.no_auto_import(name) || info.unresolved_import_names.iter().any(|n| n == name) {
        return Ok(Vec::new());
    }
    let cur_unit = env.current.and_then(|c| c.unit);
    let mut found: Vec<ClassHit> = Vec::new();
    for h in project.index.classes_by_short_name(name, env.scope)? {
        if Some(h.unit) == cur_unit {
            continue;
        }
        let c = h.class();
        let fqn = &c.qname;
        if info.annotation && c.kind != ClassKind::Annotation {
            continue;
        }
        if !fqn.contains('.') || h.package().is_empty() || settings.is_excluded(fqn) {
            continue;
        }
        if !settings.insert_imports_for_inner_classes && !h.path.is_empty() {
            // Nested classes are offered through their outer class only when asked for.
            continue;
        }
        if found.iter().any(|f| f.class().qname == *fqn) {
            continue; // one class per FQN (JAVA-IMP-013)
        }
        found.push(h);
    }
    // 4. inaccessible classes go when something accessible exists.
    let acc = |h: &ClassHit| env.class_accessible(h, &place.package, place.top_class.as_deref());
    if found.iter().any(&acc) {
        found.retain(|h| acc(h) || h.package() == place.package);
    }
    // 5. `Foo.bar`: classes with such a member.
    if let Some((member, is_static)) = &info.qualified_member {
        let with: Vec<ClassHit> = found
            .iter()
            .filter(|h| {
                let c = h.class();
                c.fields.iter().any(|f| {
                    f.name == *member
                        && (!*is_static || f.flags & flags::STATIC != 0 || c.is_interface_like())
                }) || c
                    .methods
                    .iter()
                    .any(|m| m.name == *member && (!*is_static || m.flags & flags::STATIC != 0))
                    || c.inner_named(member).is_some()
            })
            .cloned()
            .collect();
        if !with.is_empty() {
            found = with;
        }
    }
    // 7. package-like qualifier.
    if let Some(q) = &info.package_qualifier {
        found.retain(|h| h.package() == q);
    }
    if info.type_args {
        let with: Vec<ClassHit> = found
            .iter()
            .filter(|h| !h.class().type_params.is_empty())
            .cloned()
            .collect();
        if !with.is_empty() {
            found = with;
        }
    }
    let mut cands: Vec<ClassCandidate> = found
        .into_iter()
        .map(|class| ClassCandidate { class })
        .collect();
    cands.sort_by(|a, b| {
        let dep = |c: &ClassCandidate| c.class.class().deprecated;
        dep(a)
            .cmp(&dep(b))
            .then_with(|| (!a.class.path.is_empty()).cmp(&(!b.class.path.is_empty())))
            .then_with(|| {
                let ra = settings.recent.iter().rposition(|r| r == a.fqn());
                let rb = settings.recent.iter().rposition(|r| r == b.fqn());
                match (ra, rb) {
                    (Some(x), Some(y)) => y.cmp(&x),
                    _ => Ordering::Equal,
                }
            })
            .then_with(|| {
                let pa = proximity(project, place, &a.class, dep(a), acc(&a.class), true);
                let pb = proximity(project, place, &b.class, dep(b), acc(&b.class), true);
                pb.cmp(&pa)
            })
            .then_with(|| a.fqn().to_lowercase().cmp(&b.fqn().to_lowercase()))
            .then_with(|| a.fqn().cmp(b.fqn()))
    });
    Ok(cands)
}

// ---- quick fixes ----------------------------------------------------------------------------------------

/// `Import static method...` / `Import static constant...` (JAVA-IMP-003, 030..033).
pub struct ImportStaticFix {
    pub candidates: Vec<StaticCandidate>,
    pub settings: ImportSettings,
    pub constant: bool,
    pub always_on_demand: Vec<String>,
}

impl ImportStaticFix {
    pub fn text(&self) -> String {
        let what = if self.constant { "constant" } else { "method" };
        match self.candidates.as_slice() {
            [one] => format!("Import static {what} '{}'", one.qualified()),
            _ => format!("Import static {what}..."),
        }
    }

    fn edit_for(&self, text: &str, c: &StaticCandidate) -> Option<TextEdit> {
        let out = import_edit::add_static_import(
            text,
            c.class_fqn(),
            c.name(),
            &self.settings,
            self.always_on_demand.iter().any(|p| p == c.class_fqn()),
        );
        import_edit::minimal_edit(text, &out.text)
    }

    /// The command that applies candidate `index` to `text` at `rev`.
    pub fn apply(
        &self,
        cx_file: archipel_ide_analysis::FileId,
        rev: archipel_ide_analysis::FileRev,
        text: &str,
        index: usize,
    ) -> ModCommand {
        let Some(c) = self.candidates.get(index) else {
            return ModCommand::Nothing;
        };
        match self.edit_for(text, c) {
            Some(edit) => ModCommand::UpdateFile {
                file: cx_file,
                edits: vec![edit],
                expected_rev: rev,
            },
            None => ModCommand::Nothing,
        }
    }
}

impl QuickFix for ImportStaticFix {
    fn family(&self) -> &str {
        if self.constant {
            "Import static constant"
        } else {
            "Import static method"
        }
    }
    fn name(&self) -> String {
        self.text()
    }
    fn priority(&self) -> Priority {
        Priority::Top
    }
    fn fix(&self, _d: &ProblemDescriptor, cx: &FixCx<'_>) -> Result<ModCommand, Cancelled> {
        let text = cx.syntax.text();
        let rev = cx.syntax.rev();
        match self.candidates.len() {
            0 => Ok(ModCommand::Nothing),
            1 => Ok(self.apply(cx.file, rev, text, 0)),
            _ => Ok(ModCommand::ChooseFrom {
                title: if self.constant {
                    "Field to Import".into()
                } else {
                    "Method to Import".into()
                },
                rows: self
                    .candidates
                    .iter()
                    .enumerate()
                    .map(|(i, c)| ChoiceRow {
                        label: c.display(),
                        detail: (!c.package().is_empty()).then(|| format!("({})", c.package())),
                        command: self.apply(cx.file, rev, text, i),
                    })
                    .collect(),
            }),
        }
    }
}

/// `Import class` (JAVA-IMP-040..044).
pub struct ImportClassFix {
    pub candidates: Vec<ClassCandidate>,
    pub settings: ImportSettings,
    pub package: String,
}

impl ImportClassFix {
    pub fn apply(
        &self,
        file: archipel_ide_analysis::FileId,
        rev: archipel_ide_analysis::FileRev,
        text: &str,
        index: usize,
    ) -> ModCommand {
        let Some(c) = self.candidates.get(index) else {
            return ModCommand::Nothing;
        };
        let out = import_edit::add_class_import(text, c.fqn(), &self.package, &self.settings);
        match import_edit::minimal_edit(text, &out.text) {
            Some(edit) => ModCommand::UpdateFile {
                file,
                edits: vec![edit],
                expected_rev: rev,
            },
            None => ModCommand::Nothing,
        }
    }
}

impl QuickFix for ImportClassFix {
    fn family(&self) -> &str {
        "Import class"
    }
    fn name(&self) -> String {
        "Import class".into()
    }
    fn priority(&self) -> Priority {
        Priority::Top
    }
    fn fix(&self, _d: &ProblemDescriptor, cx: &FixCx<'_>) -> Result<ModCommand, Cancelled> {
        let text = cx.syntax.text();
        let rev = cx.syntax.rev();
        match self.candidates.len() {
            0 => Ok(ModCommand::Nothing),
            1 => Ok(self.apply(cx.file, rev, text, 0)),
            _ => Ok(ModCommand::ChooseFrom {
                title: "Class to Import".into(),
                rows: self
                    .candidates
                    .iter()
                    .enumerate()
                    .map(|(i, c)| ChoiceRow {
                        label: c.fqn().to_string(),
                        detail: None,
                        command: self.apply(cx.file, rev, text, i),
                    })
                    .collect(),
            }),
        }
    }
}

pub fn shared<T: QuickFix + 'static>(fix: T) -> Arc<dyn QuickFix> {
    Arc::new(fix)
}
