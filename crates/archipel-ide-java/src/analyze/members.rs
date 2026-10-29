//! Members of a type as seen through a receiver (spec 02 2.3.2 member lookup, spec 06 6.3 position
//! 2, JAVA-TYP-013 substitution through supertypes): the one enumeration completion, and later
//! applicability, ask. Fields, methods and member types of the receiver class and of every
//! supertype, inherited ones included, overridden declarations collapsed, accessibility decided
//! (JAVA-RES-043), type parameters of the receiver substituted into the signatures.

use super::*;

/// A receiver whose members are wanted.
#[derive(Debug, Clone)]
pub struct Receiver {
    /// The classes whose members count: one for a class type, the bounds of a type variable
    /// (an intersection), `Object` for an array.
    pub hits: Vec<ClassHit>,
    /// The static type, for the substitution of type arguments (`List<String>`).
    pub ty: Option<Ty>,
    /// The qualifier is a type name: only static members and member types (`Type.|`).
    pub type_qualifier: bool,
    /// An array: `length` and `clone()` exist in addition.
    pub array: Option<Ty>,
    /// `this`, `super`, or no qualifier at all: protected members of the enclosing class chain's
    /// supertypes are reachable (JLS 6.6.2).
    pub via_this: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemberKind {
    Method,
    Field,
    EnumConstant,
    /// A member class, interface, enum, record or annotation type.
    Type(ClassKind),
}

/// One parameter as shown in a signature.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParamInfo {
    pub ty: String,
    /// `None` when the name was generated (class files without parameter names).
    pub name: Option<String>,
}

/// One declared (or synthetic) member found through a receiver.
#[derive(Debug, Clone)]
pub struct MemberInfo {
    pub name: String,
    pub kind: MemberKind,
    /// The declaring class.
    pub owner: ClassHit,
    /// Index into the owner's `methods`, `fields` or `inner`; `usize::MAX` for a synthetic member.
    pub index: usize,
    pub is_static: bool,
    pub deprecated: bool,
    pub accessible: bool,
    /// Declared in `java.lang.Object` (ranked after the domain members, JAVA-CMP-027).
    pub from_object: bool,
    /// Distance of the declaring class from the receiver in the hierarchy walk (0: the receiver).
    pub depth: usize,
    pub params: Vec<ParamInfo>,
    pub varargs: bool,
    /// Return type of a method, declared type of a field, as presented.
    pub ty: String,
    /// The type as a model value (substituted; `Unknown` where it depends on an inference).
    pub ty_model: Ty,
    pub flags: u32,
}

impl MemberInfo {
    pub fn is_method(&self) -> bool {
        self.kind == MemberKind::Method
    }
}

/// Where the members are used from, for accessibility (JAVA-RES-043).
#[derive(Debug, Clone, Default)]
pub struct Place {
    pub package: String,
    /// Qualified name of the top-level class that contains the place.
    pub top: Option<String>,
    /// The classes enclosing the place, outermost first.
    pub chain: Vec<ClassHit>,
}

/// The members of a receiver and whether the walk saw the whole hierarchy.
#[derive(Debug, Clone, Default)]
pub struct MemberSet {
    pub members: Vec<MemberInfo>,
    /// Every supertype resolved: the list is what javac would see.
    pub complete: bool,
}

pub(super) fn erased_key(t: &Ty) -> String {
    match t {
        Ty::Ref { fqn: Some(f), .. } => f.clone(),
        Ty::Ref { name, .. } => name.clone(),
        Ty::Prim(p) => p.clone(),
        Ty::Array(e) => format!("{}[]", erased_key(e)),
        _ => "?".into(),
    }
}

fn varargs_display(t: &Ty, varargs: bool) -> String {
    let shown = t.display();
    if varargs && let Some(base) = shown.strip_suffix("[]") {
        return format!("{base}...");
    }
    shown
}

impl<'a, 'e> Analyzer<'a, 'e> {
    /// Like [`Self::member_ty`] for presentation: type parameters the receiver does not bind stay
    /// as variables (`T[] toArray(T[])`) instead of becoming unknown.
    pub fn member_ty_keep(
        &self,
        text: &str,
        owner: &ClassHit,
        mparams: &[String],
        recv: Option<&Ty>,
    ) -> Res<Ty> {
        let mut ctx = self.env.ctx_of(owner);
        ctx.tparams = mparams.to_vec();
        let t = self.ty_from_ctx(text, &ctx)?;
        if !t.has_var() {
            return Ok(t);
        }
        let mut map = HashMap::new();
        if let Some(args) = self.args_as(recv, owner)? {
            for (p, a) in owner.class().type_params.iter().zip(args) {
                map.insert(p.name.clone(), a);
            }
        }
        Ok(t.subst_partial(&map))
    }

    /// Whether a member with `flags` declared in `owner` is reachable from `place` through
    /// `recv` (JLS 6.6.1, JAVA-RES-043): public ones are (the receiver's type is accessible, a
    /// public member inherited from a package-private class stays reachable through its public
    /// subclass), private ones inside the top-level class that declares them, protected ones from
    /// the package or through a subclass, package-private ones from the package.
    pub fn member_accessible(
        &self,
        owner: &ClassHit,
        flags: u32,
        place: &Place,
        recv: &Receiver,
    ) -> Res<bool> {
        let interface_member = owner.class().is_interface_like() && flags & flags::PRIVATE == 0;
        if flags & flags::PUBLIC != 0 || interface_member {
            return Ok(true);
        }
        if flags & flags::PRIVATE != 0 {
            return Ok(place.top.as_deref() == Some(owner.record.class.qname.as_str()));
        }
        if owner.package() == place.package {
            return Ok(true);
        }
        if flags & flags::PROTECTED == 0 {
            return Ok(false);
        }
        // A hierarchy that is not fully known (a missing library) cannot rule the access out:
        // `None` counts as a subclass here, the row is then shown, not hidden.
        for c in place.chain.iter().rev() {
            if self.env.is_subclass(c, owner.qname())?.unwrap_or(true) {
                // The qualifier type must be that class or a subclass of it (JLS 6.6.2.1).
                let through = recv.via_this
                    || match recv.hits.first() {
                        Some(q) => self.env.is_subclass(q, c.qname())?.unwrap_or(true),
                        None => false,
                    };
                if through {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    /// The members of `recv`, nearest declaration first (JAVA-CMP-008, JAVA-TYP-013). With
    /// `include_inaccessible` the members the place cannot reach are returned too (flagged).
    pub fn members_of(
        &mut self,
        recv: &Receiver,
        place: &Place,
        include_inaccessible: bool,
    ) -> Res<MemberSet> {
        let mut out = MemberSet {
            members: Vec::new(),
            complete: true,
        };
        let mut seen_methods: HashSet<(String, Vec<String>)> = HashSet::new();
        let mut seen_fields: HashSet<String> = HashSet::new();
        let mut seen_types: HashSet<String> = HashSet::new();
        let mut order: Vec<(usize, ClassHit)> = Vec::new();
        let mut seen_classes: HashSet<(u32, String)> = HashSet::new();
        for root in &recv.hits {
            let (all, complete) = self.env.hierarchy(root)?;
            out.complete &= complete;
            for (depth, h) in all.into_iter().enumerate() {
                if seen_classes.insert((h.unit, h.class().qname.clone())) {
                    order.push((depth, h));
                }
            }
        }
        // The receiver's own members first, then by distance: stable over the roots.
        order.sort_by_key(|(d, _)| *d);
        if let Some(elem) = &recv.array {
            // `length` is a public final field, `clone()` returns the array type (JLS 10.7).
            if let Some(object) = order.iter().map(|(_, h)| h.clone()).next() {
                out.members.push(MemberInfo {
                    name: "length".into(),
                    kind: MemberKind::Field,
                    owner: object.clone(),
                    index: usize::MAX,
                    is_static: false,
                    deprecated: false,
                    accessible: true,
                    from_object: false,
                    depth: 0,
                    params: Vec::new(),
                    varargs: false,
                    ty: "int".into(),
                    ty_model: Ty::Prim("int".into()),
                    flags: flags::PUBLIC | flags::FINAL,
                });
                out.members.push(MemberInfo {
                    name: "clone".into(),
                    kind: MemberKind::Method,
                    owner: object,
                    index: usize::MAX,
                    is_static: false,
                    deprecated: false,
                    accessible: true,
                    from_object: false,
                    depth: 0,
                    params: Vec::new(),
                    varargs: false,
                    ty: elem.display(),
                    ty_model: elem.clone(),
                    flags: flags::PUBLIC,
                });
                seen_methods.insert(("clone".into(), Vec::new()));
            }
        }
        for (depth, h) in order {
            let class = h.class().clone();
            let from_object = super::super::env::is_object(&h);
            let interface = class.is_interface_like();
            // Substitution of the receiver's type arguments into this supertype's signatures.
            let recv_ty = recv.ty.as_ref();
            // Methods.
            let mut method_list: Vec<(usize, Cow<'_, MethodStub>)> = class
                .methods
                .iter()
                .enumerate()
                .map(|(i, m)| (i, Cow::Borrowed(m)))
                .collect();
            for synthetic in self.synthetic_methods(&h) {
                method_list.push((usize::MAX, Cow::Owned(synthetic)));
            }
            for (index, m) in method_list {
                if m.is_ctor || m.name.is_empty() {
                    continue;
                }
                let is_static = m.flags & flags::STATIC != 0;
                if recv.type_qualifier {
                    // Static interface methods are not inherited (JLS 8.4.8): only through the
                    // interface itself.
                    if !is_static || (interface && depth > 0) {
                        continue;
                    }
                } else if is_static && interface {
                    // ... and cannot be called on an instance either (JLS 15.12.3).
                    continue;
                }
                let mparams: Vec<String> = m.type_params.iter().map(|t| t.name.clone()).collect();
                let mut params = Vec::new();
                let mut key = Vec::new();
                for p in &m.params {
                    let t = self.member_ty_keep(&p.ty, &h, &mparams, recv_ty)?;
                    key.push(erased_key(&t));
                    params.push(ParamInfo {
                        ty: varargs_display(&t, p.varargs),
                        name: (!p.generated).then(|| p.name.clone()),
                    });
                }
                // An override (same name, same erased parameters) hides the inherited one.
                if !seen_methods.insert((m.name.clone(), key)) {
                    continue;
                }
                let accessible = self.member_accessible(&h, m.flags, place, recv)?;
                if !accessible && !include_inaccessible {
                    continue;
                }
                let ret = match m.ret.as_deref() {
                    Some(r) => self.member_ty_keep(r, &h, &mparams, recv_ty)?,
                    None => Ty::Unknown,
                };
                // The model type is the strict substitution (no inference: unknown stays unknown).
                let ty_model = match m.ret.as_deref() {
                    Some("void") => Ty::Prim("void".into()),
                    Some(r) => self.member_ty(r, &h, &mparams, recv_ty)?,
                    None => Ty::Unknown,
                };
                out.members.push(MemberInfo {
                    name: m.name.clone(),
                    kind: MemberKind::Method,
                    owner: h.clone(),
                    index,
                    is_static,
                    deprecated: m.deprecated || class.deprecated,
                    accessible,
                    from_object,
                    depth,
                    params,
                    varargs: m.varargs,
                    ty: match m.ret.as_deref() {
                        Some("void") => "void".into(),
                        _ => ret.display(),
                    },
                    ty_model,
                    flags: m.flags,
                });
            }
            // Fields and enum constants.
            for (index, f) in class.fields.iter().enumerate() {
                let is_static = f.flags & flags::STATIC != 0 || f.enum_constant || interface;
                if recv.type_qualifier && !is_static {
                    continue;
                }
                if !seen_fields.insert(f.name.clone()) {
                    continue; // hidden by a nearer field of the same name
                }
                let accessible = self.member_accessible(&h, f.flags, place, recv)?;
                if !accessible && !include_inaccessible {
                    continue;
                }
                let t = self.member_ty_keep(&f.ty, &h, &[], recv_ty)?;
                let ty_model = self.member_ty(&f.ty, &h, &[], recv_ty)?;
                out.members.push(MemberInfo {
                    name: f.name.clone(),
                    kind: if f.enum_constant {
                        MemberKind::EnumConstant
                    } else {
                        MemberKind::Field
                    },
                    owner: h.clone(),
                    index,
                    is_static,
                    deprecated: f.deprecated || class.deprecated,
                    accessible,
                    from_object,
                    depth,
                    params: Vec::new(),
                    varargs: false,
                    ty: t.display(),
                    ty_model,
                    flags: f.flags,
                });
            }
            // Record components are fields (private) with public accessors: the accessors are
            // synthetic methods above; the components themselves are not reachable from outside.
            // Member types, through a type qualifier only.
            if recv.type_qualifier {
                for (index, inner) in class.inner.iter().enumerate() {
                    if !seen_types.insert(inner.name.clone()) {
                        continue;
                    }
                    let Some(ih) = h.inner(&inner.name) else {
                        continue;
                    };
                    let accessible =
                        self.env
                            .class_accessible(&ih, &place.package, place.top.as_deref());
                    if !accessible && !include_inaccessible {
                        continue;
                    }
                    out.members.push(MemberInfo {
                        name: inner.name.clone(),
                        kind: MemberKind::Type(inner.kind),
                        owner: h.clone(),
                        index,
                        is_static: true,
                        deprecated: inner.deprecated,
                        accessible,
                        from_object,
                        depth,
                        params: Vec::new(),
                        varargs: false,
                        ty: String::new(),
                        ty_model: Ty::Unknown,
                        flags: inner.flags,
                    });
                }
            }
        }
        Ok(out)
    }

    /// The members javac adds to a class that its declaration does not show (JAVA-STUB-022/023):
    /// `values()` and `valueOf(String)` of an enum, the accessors of a source record.
    fn synthetic_methods(&self, h: &ClassHit) -> Vec<MethodStub> {
        let c = h.class();
        let mut out = Vec::new();
        let method = |name: &str, flags: u32, ret: String, params: Vec<ParamStub>| MethodStub {
            name: name.into(),
            flags,
            type_params: Vec::new(),
            ret: Some(ret),
            params,
            throws: Vec::new(),
            annotations: Vec::new(),
            is_ctor: false,
            varargs: false,
            deprecated: false,
            has_default: false,
        };
        match c.kind {
            ClassKind::Enum if !c.compiled => {
                let me = c.qname.clone();
                if !c.methods.iter().any(|m| m.name == "values") {
                    out.push(method(
                        "values",
                        flags::PUBLIC | flags::STATIC,
                        format!("{me}[]"),
                        Vec::new(),
                    ));
                }
                if !c.methods.iter().any(|m| m.name == "valueOf") {
                    out.push(method(
                        "valueOf",
                        flags::PUBLIC | flags::STATIC,
                        me,
                        vec![ParamStub {
                            name: "name".into(),
                            ty: "java.lang.String".into(),
                            varargs: false,
                            generated: false,
                        }],
                    ));
                }
            }
            ClassKind::Record if !c.compiled => {
                for comp in &c.record_components {
                    if !c
                        .methods
                        .iter()
                        .any(|m| m.name == comp.name && m.params.is_empty())
                    {
                        out.push(method(
                            &comp.name,
                            flags::PUBLIC,
                            comp.ty.clone(),
                            Vec::new(),
                        ));
                    }
                }
            }
            _ => {}
        }
        out
    }
}
