//! The resolution environment: class lookup by FQN through the indexes (with the live
//! current file shadowing its stale index entries), class hierarchies, the file-level name
//! walk of spec 02 section 2.5 (JAVA-RES-018..023), qualified type names (JAVA-RES-026,
//! 027) and accessibility (JAVA-RES-043).

use crate::index::{ClassHit, JavaIndex, NotReady, StubRecord};
use crate::scope::ResolveScope;
use crate::stub::*;
use crate::ty;
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::Arc;

pub type Res<T> = Result<T, NotReady>;

/// The live state of the file being analyzed (its index entries may be stale).
#[derive(Debug, Clone)]
pub struct CurrentFile {
    /// The unit id of the file in the index, when it has one.
    pub unit: Option<u32>,
    pub stub: FileStub,
    records: Vec<Arc<StubRecord>>,
}

impl CurrentFile {
    pub fn new(unit: Option<u32>, stub: FileStub) -> Self {
        let records = stub
            .classes
            .iter()
            .map(|c| Arc::new(StubRecord { package: stub.package.clone(), imports: stub.imports.clone(), class: c.clone() }))
            .collect();
        Self { unit, stub, records }
    }

    pub fn unit_id(&self) -> u32 {
        self.unit.unwrap_or(u32::MAX)
    }

    pub fn hit(&self, top: usize, path: Vec<String>) -> ClassHit {
        ClassHit {
            unit: self.unit_id(),
            rank: 0,
            record: self.records[top].clone(),
            path,
            local: None,
            local_supers: None,
        }
    }

    /// Every named class of the file (top-level and nested) as hits.
    pub fn all_hits(&self) -> Vec<ClassHit> {
        let mut out = Vec::new();
        for (i, top) in self.stub.classes.iter().enumerate() {
            top.walk(&mut Vec::new(), &mut |p, _| {
                out.push(self.hit(i, p[1..].iter().map(|s| s.to_string()).collect()));
            });
        }
        out
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Via {
    File,
    SingleImport(usize),
    SingleStaticImport(usize),
    Package,
    StaticOnDemand(usize),
    OnDemand(usize),
    JavaLang,
    Member,
}

#[derive(Debug, Clone)]
pub enum NameRes {
    Class(ClassHit, Via),
    TypeVar(String),
    Ambiguous(ClassHit, ClassHit),
    NotFound,
    /// Something that could supply the name could not be resolved (an unresolved import, a missing
    /// supertype): no error may be reported for it.
    Incomplete,
}

/// Where a name is looked up: file-level context plus the enclosing class chain.
#[derive(Debug, Clone)]
pub struct Ctx<'a> {
    pub package: &'a str,
    pub imports: &'a [ImportStub],
    /// Enclosing classes, outermost first.
    pub chain: Vec<ClassHit>,
    /// Type parameters of the enclosing method(s).
    pub tparams: Vec<String>,
    /// Top-level class names of the file.
    pub file_tops: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct Supers {
    pub list: Vec<ClassHit>,
    /// Every supertype resolved.
    pub complete: bool,
}

pub struct Env<'a> {
    pub index: &'a JavaIndex,
    pub scope: &'a ResolveScope,
    pub current: Option<&'a CurrentFile>,
    depth: Cell<u32>,
    supers: RefCell<HashMap<(u32, String), Rc<Supers>>>,
}

const MAX_DEPTH: u32 = 32;

pub fn is_object(h: &ClassHit) -> bool {
    h.class().qname == "java.lang.Object" && h.class().compiled
}

impl<'a> Env<'a> {
    pub fn new(index: &'a JavaIndex, scope: &'a ResolveScope, current: Option<&'a CurrentFile>) -> Self {
        Self { index, scope, current, depth: Cell::new(0), supers: RefCell::new(HashMap::new()) }
    }

    fn guarded<T>(&self, default: T, f: impl FnOnce() -> Res<T>) -> Res<T> {
        if self.depth.get() >= MAX_DEPTH {
            return Ok(default);
        }
        self.depth.set(self.depth.get() + 1);
        let r = f();
        self.depth.set(self.depth.get() - 1);
        r
    }

    // ---- class lookup ---------------------------------------------------------------

    pub fn find_class(&self, fqn: &str) -> Res<Option<ClassHit>> {
        if let Some(cur) = self.current {
            for (i, top) in cur.stub.classes.iter().enumerate() {
                let mut found = None;
                top.walk(&mut Vec::new(), &mut |p, c| {
                    if found.is_none() && c.qname == fqn {
                        found = Some(p[1..].iter().map(|s| s.to_string()).collect::<Vec<_>>());
                    }
                });
                if let Some(path) = found {
                    return Ok(Some(cur.hit(i, path)));
                }
            }
        }
        let cur_unit = self.current.and_then(|c| c.unit);
        Ok(self.index.find_classes(fqn, self.scope)?.into_iter().find(|h| Some(h.unit) != cur_unit))
    }

    pub fn package_exists(&self, package: &str) -> Res<bool> {
        if let Some(cur) = self.current
            && (cur.stub.package == package || cur.stub.package.starts_with(&format!("{package}.")))
            && !package.is_empty()
        {
            return Ok(true);
        }
        self.index.has_package(package, self.scope)
    }

    // ---- hierarchy ------------------------------------------------------------------

    pub fn ctx_of<'c>(&self, hit: &'c ClassHit) -> Ctx<'c> {
        let mut chain = Vec::new();
        let record = hit.record.clone();
        let mut prefix: Vec<String> = Vec::new();
        chain.push(ClassHit { path: Vec::new(), local: None, local_supers: None, ..hit.clone() });
        for name in &hit.path {
            prefix.push(name.clone());
            chain.push(ClassHit { path: prefix.clone(), local: None, local_supers: None, record: record.clone(), ..hit.clone() });
        }
        if hit.local.is_some() {
            chain.push(hit.clone());
        }
        Ctx {
            package: &hit.record.package,
            imports: &hit.record.imports,
            chain,
            tparams: Vec::new(),
            file_tops: vec![hit.record.class.name.clone()],
        }
    }

    pub fn supertypes(&self, hit: &ClassHit) -> Res<Rc<Supers>> {
        if let Some(sup) = &hit.local_supers {
            return Ok(Rc::new(Supers { list: sup.to_vec(), complete: true }));
        }
        let key = (hit.unit, hit.class().qname.clone());
        if hit.local.is_none()
            && let Some(s) = self.supers.borrow().get(&key)
        {
            return Ok(s.clone());
        }
        let class = hit.class();
        let mut texts: Vec<String> = class.extends.iter().chain(class.implements.iter()).cloned().collect();
        match class.kind {
            ClassKind::Enum => texts.push("java.lang.Enum".into()),
            ClassKind::Record => texts.push("java.lang.Record".into()),
            ClassKind::Annotation => texts.push("java.lang.annotation.Annotation".into()),
            _ => {}
        }
        let is_obj = is_object(hit);
        // The header of a class is outside its own scope: resolve in the enclosing context.
        let mut ctx = self.ctx_of(hit);
        if hit.local.is_none() {
            ctx.chain.pop();
        }
        ctx.tparams = class.type_params.iter().map(|t| t.name.clone()).collect();
        let mut out = Supers { list: Vec::new(), complete: true };
        self.guarded((), || {
            for t in &texts {
                match self.resolve_type_text(t, &ctx)? {
                    TypeRes::Class(h) => out.list.push(h),
                    TypeRes::TypeVar(_) | TypeRes::Prim(_) => {}
                    TypeRes::NotFound | TypeRes::Incomplete => out.complete = false,
                }
            }
            if !is_obj && let Some(o) = self.find_class("java.lang.Object")? {
                out.list.push(o);
            }
            Ok(())
        })?;
        let rc = Rc::new(out);
        if hit.local.is_none() {
            self.supers.borrow_mut().insert(key, rc.clone());
        }
        Ok(rc)
    }

    /// The class and all its supertypes, transitively, each once, in lookup order
    /// (class, superclasses before interfaces breadth-first as declared), plus completeness.
    pub fn hierarchy(&self, hit: &ClassHit) -> Res<(Vec<ClassHit>, bool)> {
        let mut seen: HashSet<(u32, String)> = HashSet::new();
        let mut out = Vec::new();
        let mut complete = true;
        let mut queue = vec![hit.clone()];
        let mut i = 0;
        while i < queue.len() {
            let h = queue[i].clone();
            i += 1;
            if !seen.insert((h.unit, h.class().qname.clone())) {
                continue;
            }
            let s = self.supertypes(&h)?;
            complete &= s.complete;
            queue.extend(s.list.iter().cloned());
            out.push(h);
        }
        Ok((out, complete))
    }

    /// Whether `sub` is `sup` or inherits from it; `None` when the hierarchy is incomplete.
    pub fn is_subclass(&self, sub: &ClassHit, sup_fqn: &str) -> Res<Option<bool>> {
        let (all, complete) = self.hierarchy(sub)?;
        if all.iter().any(|h| h.class().qname == sup_fqn) {
            return Ok(Some(true));
        }
        Ok(complete.then_some(false))
    }

    /// Member type `name` of `hit`: own first, then inherited.
    pub fn member_type(&self, hit: &ClassHit, name: &str) -> Res<Option<ClassHit>> {
        let (all, _) = self.hierarchy(hit)?;
        for h in all {
            if let Some(i) = h.inner(name) {
                return Ok(Some(i));
            }
        }
        Ok(None)
    }

    /// Methods named `name` of `hit` and its supertypes (own first), with the completeness of the walk.
    pub fn methods_named(&self, hit: &ClassHit, name: &str) -> Res<(Vec<(ClassHit, usize)>, bool)> {
        let (all, complete) = self.hierarchy(hit)?;
        let mut out = Vec::new();
        for h in all {
            let c = h.class();
            for (i, m) in c.methods.iter().enumerate() {
                if m.name == name && !m.is_ctor {
                    out.push((h.clone(), i));
                }
            }
        }
        Ok((out, complete))
    }

    /// Whether a synthetic member (enum `values`/`valueOf`, record accessor) named `name` exists.
    pub fn has_synthetic_method(&self, hit: &ClassHit, name: &str) -> bool {
        let c = hit.class();
        match c.kind {
            ClassKind::Enum => matches!(name, "values" | "valueOf"),
            ClassKind::Record => c.record_components.iter().any(|p| p.name == name),
            _ => false,
        }
    }

    /// Fields named `name` (enum constants, record components, inherited ones), with completeness.
    pub fn fields_named(&self, hit: &ClassHit, name: &str) -> Res<(Vec<(ClassHit, usize)>, bool, bool)> {
        let (all, complete) = self.hierarchy(hit)?;
        let mut out = Vec::new();
        let mut component = false;
        for h in all {
            let c = h.class();
            for (i, f) in c.fields.iter().enumerate() {
                if f.name == name {
                    out.push((h.clone(), i));
                }
            }
            if c.record_components.iter().any(|p| p.name == name) {
                component = true;
            }
        }
        Ok((out, complete, component))
    }

    // ---- type names -----------------------------------------------------------------

    /// Resolves a simple type name in the scope walk of spec 02 section 2.3.2 and 2.5.
    pub fn resolve_type_name(&self, name: &str, ctx: &Ctx<'_>) -> Res<NameRes> {
        self.guarded(NameRes::Incomplete, || self.resolve_type_name_inner(name, ctx))
    }

    fn resolve_type_name_inner(&self, name: &str, ctx: &Ctx<'_>) -> Res<NameRes> {
        if ctx.tparams.iter().any(|t| t == name) {
            return Ok(NameRes::TypeVar(name.to_string()));
        }
        // Class scopes, innermost first.
        for h in ctx.chain.iter().rev() {
            let c = h.class();
            if c.type_params.iter().any(|t| t.name == name) {
                return Ok(NameRes::TypeVar(name.to_string()));
            }
            if c.name == name {
                return Ok(NameRes::Class(h.clone(), Via::Member));
            }
            if let Some(m) = self.member_type(h, name)? {
                return Ok(NameRes::Class(m, Via::Member));
            }
            if !self.supertypes(h)?.complete {
                // An unresolved supertype may supply the name: do not fall through to an
                // outer match that could be wrong, and never report an error.
                return Ok(NameRes::Incomplete);
            }
        }
        // Stage A.
        if ctx.file_tops.iter().any(|n| n == name) {
            let fqn = if ctx.package.is_empty() { name.to_string() } else { format!("{}.{name}", ctx.package) };
            if let Some(h) = self.find_class(&fqn)? {
                return Ok(NameRes::Class(h, Via::File));
            }
        }
        for (i, imp) in ctx.imports.iter().enumerate() {
            if imp.is_static || imp.on_demand || imp.name.rsplit('.').next() != Some(name) {
                continue;
            }
            return Ok(match self.find_class(&imp.name)? {
                Some(h) => NameRes::Class(h, Via::SingleImport(i)),
                None => NameRes::Incomplete, // JAVA-RES-023: an unresolved import reserves the name
            });
        }
        for (i, imp) in ctx.imports.iter().enumerate() {
            if !imp.is_static || imp.on_demand || imp.name.rsplit('.').next() != Some(name) {
                continue;
            }
            let Some((cls, _)) = imp.name.rsplit_once('.') else { continue };
            if let Some(owner) = self.find_class(cls)?
                && let Some(m) = self.member_type(&owner, name)?
            {
                return Ok(NameRes::Class(m, Via::SingleStaticImport(i)));
            }
        }
        // Stage B.
        let fqn = if ctx.package.is_empty() { name.to_string() } else { format!("{}.{name}", ctx.package) };
        if let Some(h) = self.find_class(&fqn)? {
            return Ok(NameRes::Class(h, Via::Package));
        }
        for (i, imp) in ctx.imports.iter().enumerate() {
            if imp.is_static && imp.on_demand
                && let Some(owner) = self.find_class(&imp.name)?
                && let Some(m) = self.member_type(&owner, name)?
                && m.class().flags & flags::STATIC != 0
            {
                return Ok(NameRes::Class(m, Via::StaticOnDemand(i)));
            }
        }
        let mut found: Vec<(ClassHit, Via)> = Vec::new();
        let mut unknown_import = false;
        for (i, imp) in ctx.imports.iter().enumerate() {
            if imp.is_static || !imp.on_demand {
                continue;
            }
            if let Some(h) = self.find_class(&format!("{}.{name}", imp.name))? {
                if !found.iter().any(|(f, _)| f.qname() == h.qname()) {
                    found.push((h, Via::OnDemand(i)));
                }
            } else if !self.package_exists(&imp.name)? && self.find_class(&imp.name)?.is_none() {
                unknown_import = true;
            }
        }
        if let Some(h) = self.find_class(&format!("java.lang.{name}"))
            .ok()
            .flatten()
            && !found.iter().any(|(f, _)| f.qname() == h.qname())
        {
            found.push((h, Via::JavaLang));
        }
        // Rank: the default package loses; accessible beats inaccessible (not modeled here).
        if found.len() > 1 {
            found.retain(|(h, _)| !h.package().is_empty());
        }
        match found.len() {
            0 if unknown_import => Ok(NameRes::Incomplete),
            0 => Ok(NameRes::NotFound),
            1 => {
                let (h, v) = found.remove(0);
                Ok(NameRes::Class(h, v))
            }
            _ => {
                found.sort_by(|a, b| a.0.qname().cmp(b.0.qname()));
                let (a, _) = found.remove(0);
                let (b, _) = found.remove(0);
                Ok(NameRes::Ambiguous(a, b))
            }
        }
    }

    /// Resolves a (possibly qualified, possibly generic) type text in `ctx`.
    pub fn resolve_type_text(&self, text: &str, ctx: &Ctx<'_>) -> Res<TypeRes> {
        let parts = ty::split(text);
        let base = parts.base.as_str();
        if base.is_empty() {
            return Ok(TypeRes::NotFound);
        }
        if ty::is_primitive(base) || base == "void" {
            return Ok(TypeRes::Prim(base.to_string()));
        }
        if base == "?" {
            return Ok(TypeRes::Incomplete);
        }
        let segs: Vec<&str> = base.split('.').collect();
        match self.resolve_type_name(segs[0], ctx)? {
            NameRes::Class(mut h, _) => {
                for seg in &segs[1..] {
                    match self.member_type(&h, seg)? {
                        Some(m) => h = m,
                        None => return Ok(TypeRes::NotFound),
                    }
                }
                return Ok(TypeRes::Class(h));
            }
            NameRes::TypeVar(v) if segs.len() == 1 => return Ok(TypeRes::TypeVar(v)),
            NameRes::Incomplete => return Ok(TypeRes::Incomplete),
            NameRes::Ambiguous(a, _) if segs.len() == 1 => return Ok(TypeRes::Class(a)),
            _ => {}
        }
        if segs.len() == 1 {
            return Ok(TypeRes::NotFound);
        }
        // Fully qualified: the shortest prefix that names a class wins (JAVA-RES-026).
        for i in 1..=segs.len() {
            if let Some(mut h) = self.find_class(&segs[..i].join("."))? {
                for seg in &segs[i..] {
                    match self.member_type(&h, seg)? {
                        Some(m) => h = m,
                        None => return Ok(TypeRes::NotFound),
                    }
                }
                return Ok(TypeRes::Class(h));
            }
        }
        Ok(TypeRes::NotFound)
    }

    // ---- accessibility ----------------------------------------------------------------

    /// Whether a member with `member_flags` declared in `owner` is accessible from code in
    /// `place_package` inside the top-level class `place_top` (JLS 6.6, JAVA-RES-043);
    /// `subclass` tells whether the place is in a subclass of the owner (protected).
    pub fn accessible(&self, owner: &ClassHit, member_flags: u32, place_package: &str, place_top: Option<&str>, subclass: bool) -> bool {
        let owner_class = owner.class();
        let implicit_public = owner_class.is_interface_like() && member_flags & flags::PRIVATE == 0;
        if member_flags & flags::PUBLIC != 0 || implicit_public {
            return self.class_accessible(owner, place_package, place_top);
        }
        let same_package = owner.package() == place_package;
        if member_flags & flags::PRIVATE != 0 {
            return place_top.is_some_and(|t| t == owner.record.class.qname);
        }
        if same_package {
            return true;
        }
        member_flags & flags::PROTECTED != 0 && subclass && self.class_accessible(owner, place_package, place_top)
    }

    pub fn class_accessible(&self, c: &ClassHit, place_package: &str, place_top: Option<&str>) -> bool {
        let class = c.class();
        let outer_ok = if c.path.is_empty() {
            true
        } else {
            let mut path = c.path.clone();
            path.pop();
            let outer = ClassHit { path, local: None, local_supers: None, ..c.clone() };
            self.class_accessible(&outer, place_package, place_top)
        };
        if !outer_ok {
            return false;
        }
        if c.path.is_empty() || !c.record.class.is_interface_like() || class.flags & flags::PRIVATE != 0 {
            if class.flags & flags::PRIVATE != 0 {
                return place_top.is_some_and(|t| t == c.record.class.qname);
            }
            if class.flags & flags::PUBLIC != 0 {
                return true;
            }
            if class.flags & flags::PROTECTED != 0 && !c.path.is_empty() {
                return c.package() == place_package || true;
            }
            return c.package() == place_package;
        }
        true
    }
}

#[derive(Debug, Clone)]
pub enum TypeRes {
    Class(ClassHit),
    TypeVar(String),
    Prim(String),
    NotFound,
    Incomplete,
}
