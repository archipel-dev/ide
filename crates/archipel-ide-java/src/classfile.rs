//! A class-file reader (JVMS chapter 4) that produces [`ClassStub`]s
//! (JAVA-RES-034, 035, 038, 039, 041; ST3 of spec 01).
//!
//! Skipped, as in 02 section 2.8.1: synthetic members and classes, bridge
//! methods, `<clinit>`, anonymous and local classes, `module-info` and
//! `package-info`. Synthetic constructor parameters (outer instance, enum name
//! and ordinal) are removed. Parsing never panics: malformed input is an error.

use crate::stub::{ClassKind, ClassStub, FieldStub, MethodStub, ParamStub, TypeParamStub, flags};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ClassFileError {
    #[error("not a class file")]
    BadMagic,
    #[error("truncated or malformed class file: {0}")]
    Malformed(&'static str),
}

type Res<T> = Result<T, ClassFileError>;

struct Cur<'a> {
    b: &'a [u8],
    at: usize,
}

impl<'a> Cur<'a> {
    fn take(&mut self, n: usize) -> Res<&'a [u8]> {
        let end = self
            .at
            .checked_add(n)
            .ok_or(ClassFileError::Malformed("length"))?;
        let s = self
            .b
            .get(self.at..end)
            .ok_or(ClassFileError::Malformed("eof"))?;
        self.at = end;
        Ok(s)
    }
    fn u1(&mut self) -> Res<u8> {
        Ok(self.take(1)?[0])
    }
    fn u2(&mut self) -> Res<u16> {
        let s = self.take(2)?;
        Ok(u16::from_be_bytes([s[0], s[1]]))
    }
    fn u4(&mut self) -> Res<u32> {
        let s = self.take(4)?;
        Ok(u32::from_be_bytes([s[0], s[1], s[2], s[3]]))
    }
}

#[derive(Clone)]
enum Const {
    None,
    Utf8(String),
    Int(i32),
    Float(f32),
    Long(i64),
    Double(f64),
    Class(u16),
    Str(u16),
    Named(u16),
    Other,
}

/// Modified UTF-8 (JVMS 4.4.7).
fn decode_mutf8(bytes: &[u8]) -> String {
    let mut units: Vec<u16> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b & 0x80 == 0 {
            units.push(u16::from(b));
            i += 1;
        } else if b & 0xE0 == 0xC0 && i + 1 < bytes.len() {
            units.push((u16::from(b & 0x1F) << 6) | u16::from(bytes[i + 1] & 0x3F));
            i += 2;
        } else if b & 0xF0 == 0xE0 && i + 2 < bytes.len() {
            units.push(
                (u16::from(b & 0x0F) << 12)
                    | (u16::from(bytes[i + 1] & 0x3F) << 6)
                    | u16::from(bytes[i + 2] & 0x3F),
            );
            i += 3;
        } else {
            units.push(0xFFFD);
            i += 1;
        }
    }
    String::from_utf16_lossy(&units)
}

struct Pool(Vec<Const>);

impl Pool {
    fn utf8(&self, i: u16) -> &str {
        match self.0.get(i as usize) {
            Some(Const::Utf8(s)) => s,
            _ => "",
        }
    }
    fn class_name(&self, i: u16) -> &str {
        match self.0.get(i as usize) {
            Some(Const::Class(n)) => self.utf8(*n),
            _ => "",
        }
    }
}

struct Attr<'a, 'p> {
    name: &'p str,
    data: &'a [u8],
}

fn read_attrs<'a, 'p>(c: &mut Cur<'a>, pool: &'p Pool) -> Res<Vec<Attr<'a, 'p>>> {
    let n = c.u2()?;
    let mut out = Vec::with_capacity(n as usize);
    for _ in 0..n {
        let name = pool.utf8(c.u2()?);
        let len = c.u4()? as usize;
        out.push(Attr {
            name,
            data: c.take(len)?,
        });
    }
    Ok(out)
}

#[derive(Default)]
struct Inner {
    inner: String,
    outer: String,
    name: String,
    flags: u16,
}

/// Binary name to source name, using the `InnerClasses` table.
struct Names<'a> {
    inners: &'a [Inner],
}

impl Names<'_> {
    fn source(&self, binary: &str) -> String {
        if let Some(e) = self.inners.iter().find(|e| e.inner == binary)
            && !e.outer.is_empty()
            && !e.name.is_empty()
        {
            return format!("{}.{}", self.source(&e.outer), e.name);
        }
        binary.replace('/', ".")
    }
}

/// Signature / descriptor to Java source text.
struct SigParser<'a> {
    s: &'a [u8],
    at: usize,
    names: &'a Names<'a>,
}

impl<'a> SigParser<'a> {
    fn peek(&self) -> u8 {
        self.s.get(self.at).copied().unwrap_or(0)
    }
    fn bump(&mut self) -> u8 {
        let b = self.peek();
        self.at += 1;
        b
    }
    fn ident_until(&mut self, stops: &[u8]) -> String {
        let start = self.at;
        while self.at < self.s.len() && !stops.contains(&self.s[self.at]) {
            self.at += 1;
        }
        String::from_utf8_lossy(&self.s[start..self.at]).into_owned()
    }
    fn ty(&mut self) -> String {
        match self.bump() {
            b'B' => "byte".into(),
            b'C' => "char".into(),
            b'D' => "double".into(),
            b'F' => "float".into(),
            b'I' => "int".into(),
            b'J' => "long".into(),
            b'S' => "short".into(),
            b'Z' => "boolean".into(),
            b'V' => "void".into(),
            b'[' => format!("{}[]", self.ty()),
            b'T' => {
                let n = self.ident_until(b";");
                self.bump();
                n
            }
            b'L' => self.class_ty(),
            _ => "?".into(),
        }
    }
    fn class_ty(&mut self) -> String {
        let mut binary = self.ident_until(b"<;.");
        let mut text = self.names.source(&binary);
        loop {
            match self.peek() {
                b'<' => {
                    self.bump();
                    let mut args = Vec::new();
                    while self.peek() != b'>' && self.at < self.s.len() {
                        args.push(self.type_arg());
                    }
                    self.bump();
                    text.push('<');
                    text.push_str(&args.join(", "));
                    text.push('>');
                }
                b'.' => {
                    self.bump();
                    let n = self.ident_until(b"<;.");
                    binary = format!("{binary}${n}");
                    text.push('.');
                    text.push_str(&n);
                }
                _ => {
                    self.bump();
                    return text;
                }
            }
        }
    }
    fn type_arg(&mut self) -> String {
        match self.peek() {
            b'*' => {
                self.bump();
                "?".into()
            }
            b'+' => {
                self.bump();
                format!("? extends {}", self.ty())
            }
            b'-' => {
                self.bump();
                format!("? super {}", self.ty())
            }
            _ => self.ty(),
        }
    }
    fn type_params(&mut self) -> Vec<TypeParamStub> {
        let mut out = Vec::new();
        if self.peek() != b'<' {
            return out;
        }
        self.bump();
        while self.peek() != b'>' && self.at < self.s.len() {
            let name = self.ident_until(b":");
            let mut bounds = Vec::new();
            while self.peek() == b':' {
                self.bump();
                if self.peek() == b':' {
                    continue; // empty class bound
                }
                let b = self.ty();
                if b != "java.lang.Object" || !bounds.is_empty() {
                    bounds.push(b);
                }
            }
            out.push(TypeParamStub { name, bounds });
        }
        self.bump();
        out
    }
}

fn skip_element_value(c: &mut Cur<'_>) -> Res<()> {
    match c.u1()? {
        b'e' => {
            c.u2()?;
            c.u2()?;
        }
        b'@' => {
            skip_annotation(c)?;
        }
        b'[' => {
            for _ in 0..c.u2()? {
                skip_element_value(c)?;
            }
        }
        _ => {
            c.u2()?;
        }
    }
    Ok(())
}

fn skip_annotation(c: &mut Cur<'_>) -> Res<()> {
    c.u2()?;
    for _ in 0..c.u2()? {
        c.u2()?;
        skip_element_value(c)?;
    }
    Ok(())
}

/// Enum constant names of an element value (`@Target({METHOD, TYPE})`).
fn element_enum_names(c: &mut Cur<'_>, pool: &Pool, out: &mut Vec<String>) -> Res<()> {
    match c.u1()? {
        b'e' => {
            c.u2()?;
            let name = c.u2()?;
            out.push(pool.utf8(name).to_string());
        }
        b'[' => {
            for _ in 0..c.u2()? {
                element_enum_names(c, pool, out)?;
            }
        }
        b'@' => skip_annotation(c)?,
        _ => {
            c.u2()?;
        }
    }
    Ok(())
}

/// Annotation types of a `Runtime(In)VisibleAnnotations` attribute. `@Target` keeps its
/// `ElementType` constants (`@java.lang.annotation.Target(METHOD, TYPE)`) so that the name
/// index can offer annotation types applicable to the annotated element.
fn annotations(data: &[u8], pool: &Pool, names: &Names<'_>) -> Vec<String> {
    let mut c = Cur { b: data, at: 0 };
    let mut out = Vec::new();
    let Ok(n) = c.u2() else { return out };
    for _ in 0..n {
        let Ok(t) = c.u2() else { break };
        let desc = pool.utf8(t);
        let mut p = SigParser {
            s: desc.as_bytes(),
            at: 0,
            names,
        };
        let ty = p.ty();
        let Ok(pairs) = c.u2() else { break };
        let is_target = ty == "java.lang.annotation.Target";
        let mut constants = Vec::new();
        let mut ok = true;
        for _ in 0..pairs {
            let read = c.u2().and_then(|_| {
                if is_target {
                    element_enum_names(&mut c, pool, &mut constants)
                } else {
                    skip_element_value(&mut c)
                }
            });
            if read.is_err() {
                ok = false;
                break;
            }
        }
        if is_target {
            out.push(format!("@{ty}({})", constants.join(", ")));
        } else {
            out.push(format!("@{ty}"));
        }
        if !ok {
            break;
        }
    }
    out
}

fn java_string_literal(s: &str) -> String {
    let mut out = String::from("\"");
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn float_text(v: f64, suffix: &str) -> String {
    if v.is_nan() {
        format!("0.0{suffix} / 0.0{suffix}")
    } else if v.is_infinite() {
        format!(
            "{}1.0{suffix} / 0.0{suffix}",
            if v < 0.0 { "-" } else { "" }
        )
    } else if v == v.trunc() && v.abs() < 1e15 {
        format!("{v:.1}{suffix}")
    } else {
        format!("{v}{suffix}")
    }
}

fn read_pool(c: &mut Cur<'_>) -> Res<Pool> {
    let count = c.u2()? as usize;
    let mut consts = vec![Const::None; count.max(1)];
    let mut i = 1;
    while i < count {
        let tag = c.u1()?;
        let entry = match tag {
            1 => {
                let len = c.u2()? as usize;
                Const::Utf8(decode_mutf8(c.take(len)?))
            }
            3 => Const::Int(c.u4()? as i32),
            4 => Const::Float(f32::from_bits(c.u4()?)),
            5 => {
                let hi = u64::from(c.u4()?);
                let lo = u64::from(c.u4()?);
                Const::Long(((hi << 32) | lo) as i64)
            }
            6 => {
                let hi = u64::from(c.u4()?);
                let lo = u64::from(c.u4()?);
                Const::Double(f64::from_bits((hi << 32) | lo))
            }
            7 => Const::Class(c.u2()?),
            8 => Const::Str(c.u2()?),
            9..=12 | 17 | 18 => {
                c.take(4)?;
                Const::Other
            }
            15 => {
                c.take(3)?;
                Const::Other
            }
            16 => {
                c.take(2)?;
                Const::Other
            }
            19 | 20 => Const::Named(c.u2()?),
            _ => return Err(ClassFileError::Malformed("constant tag")),
        };
        consts[i] = entry;
        i += if matches!(tag, 5 | 6) { 2 } else { 1 };
    }
    Ok(Pool(consts))
}

/// Unqualified `exports` of a `module-info.class` (module name, exported packages).
pub fn parse_module_exports(bytes: &[u8]) -> Res<(String, Vec<String>)> {
    let mut c = Cur { b: bytes, at: 0 };
    if c.u4()? != 0xCAFE_BABE {
        return Err(ClassFileError::BadMagic);
    }
    c.u4()?;
    let pool = read_pool(&mut c)?;
    c.take(6)?;
    for _ in 0..c.u2()? {
        c.u2()?;
    }
    for _ in 0..2 {
        // fields, methods (none in module-info, but be safe)
        let n = c.u2()?;
        for _ in 0..n {
            c.take(6)?;
            read_attrs(&mut c, &pool)?;
        }
    }
    let attrs = read_attrs(&mut c, &pool)?;
    let d = find(&attrs, "Module").ok_or(ClassFileError::Malformed("no Module attribute"))?;
    let mut m = Cur { b: d, at: 0 };
    let named = |i: u16| match pool.0.get(i as usize) {
        Some(Const::Named(n)) => pool.utf8(*n).to_string(),
        _ => String::new(),
    };
    let name = named(m.u2()?);
    m.take(4)?;
    for _ in 0..m.u2()? {
        m.take(6)?;
    }
    let mut out = Vec::new();
    for _ in 0..m.u2()? {
        let pkg = named(m.u2()?);
        m.u2()?;
        let to = m.u2()?;
        for _ in 0..to {
            m.u2()?;
        }
        if to == 0 {
            out.push(pkg.replace('/', "."));
        }
    }
    Ok((name, out))
}

/// One class file, before nesting.
#[derive(Debug, Clone)]
pub struct ParsedClass {
    /// Binary name with slashes (`a/b/Outer$Inner`).
    pub binary_name: String,
    /// Binary name of the declaring class when this is a member class.
    pub outer: Option<String>,
    pub stub: ClassStub,
}

struct Member<'a, 'p> {
    access: u16,
    name: &'p str,
    desc: &'p str,
    attrs: Vec<Attr<'a, 'p>>,
}

fn read_members<'a, 'p>(c: &mut Cur<'a>, pool: &'p Pool) -> Res<Vec<Member<'a, 'p>>> {
    let n = c.u2()?;
    let mut v = Vec::new();
    for _ in 0..n {
        let access = c.u2()?;
        let name = pool.utf8(c.u2()?);
        let desc = pool.utf8(c.u2()?);
        let attrs = read_attrs(c, pool)?;
        v.push(Member {
            access,
            name,
            desc,
            attrs,
        });
    }
    Ok(v)
}

fn find<'x, 'a, 'p>(attrs: &'x [Attr<'a, 'p>], name: &str) -> Option<&'a [u8]> {
    attrs.iter().find(|a| a.name == name).map(|a| a.data)
}

const ACC_PUBLIC: u16 = 0x1;
const ACC_PRIVATE: u16 = 0x2;
const ACC_PROTECTED: u16 = 0x4;
const ACC_STATIC: u16 = 0x8;
const ACC_FINAL: u16 = 0x10;
const ACC_SYNCHRONIZED: u16 = 0x20;
const ACC_VOLATILE: u16 = 0x40;
const ACC_BRIDGE: u16 = 0x40;
const ACC_TRANSIENT: u16 = 0x80;
const ACC_VARARGS: u16 = 0x80;
const ACC_NATIVE: u16 = 0x100;
const ACC_INTERFACE: u16 = 0x200;
const ACC_ABSTRACT: u16 = 0x400;
const ACC_STRICT: u16 = 0x800;
const ACC_SYNTHETIC: u16 = 0x1000;
const ACC_ANNOTATION: u16 = 0x2000;
const ACC_ENUM: u16 = 0x4000;

fn modifier_flags(access: u16, method: bool) -> u32 {
    let mut f = 0;
    let mut set = |bit: u16, flag: u32| {
        if access & bit != 0 {
            f |= flag;
        }
    };
    set(ACC_PUBLIC, flags::PUBLIC);
    set(ACC_PRIVATE, flags::PRIVATE);
    set(ACC_PROTECTED, flags::PROTECTED);
    set(ACC_STATIC, flags::STATIC);
    set(ACC_FINAL, flags::FINAL);
    set(ACC_ABSTRACT, flags::ABSTRACT);
    if method {
        set(ACC_SYNCHRONIZED, flags::SYNCHRONIZED);
        set(ACC_NATIVE, flags::NATIVE);
        set(ACC_STRICT, flags::STRICTFP);
    } else {
        set(ACC_TRANSIENT, flags::TRANSIENT);
        set(ACC_VOLATILE, flags::VOLATILE);
    }
    f
}

/// Parameter names from the `LocalVariableTable` of a method's `Code` (debug info, `-g`),
/// one per descriptor parameter; empty when absent.
fn local_variable_names(code: &[u8], desc: &str, is_static: bool, pool: &Pool) -> Vec<String> {
    let go = || -> Res<Vec<String>> {
        let mut c = Cur { b: code, at: 0 };
        c.u2()?;
        c.u2()?;
        let len = c.u4()? as usize;
        c.take(len)?;
        let ex = c.u2()? as usize;
        c.take(ex * 8)?;
        let attrs = read_attrs(&mut c, pool)?;
        let Some(lvt) = find(&attrs, "LocalVariableTable") else {
            return Ok(Vec::new());
        };
        let mut l = Cur { b: lvt, at: 0 };
        let mut by_slot = std::collections::HashMap::new();
        for _ in 0..l.u2()? {
            let start = l.u2()?;
            l.u2()?;
            let name = l.u2()?;
            l.u2()?;
            let slot = l.u2()?;
            if start == 0 {
                by_slot.insert(slot, pool.utf8(name).to_string());
            }
        }
        // Slots of the descriptor parameters.
        let bytes = desc.as_bytes();
        let mut at = 1;
        let mut slot: u16 = if is_static { 0 } else { 1 };
        let mut out = Vec::new();
        while at < bytes.len() && bytes[at] != b')' {
            let wide = matches!(bytes[at], b'J' | b'D');
            while bytes[at] == b'[' {
                at += 1;
            }
            if bytes[at] == b'L' {
                while at < bytes.len() && bytes[at] != b';' {
                    at += 1;
                }
            }
            at += 1;
            out.push(by_slot.get(&slot).cloned().unwrap_or_default());
            slot += if wide { 2 } else { 1 };
        }
        Ok(out)
    };
    go().unwrap_or_default()
}

fn generated_param_name(ty: &str, taken: &[String]) -> String {
    let base = crate::stub::short_type_name(ty);
    let mut name: String = base
        .chars()
        .next()
        .map(|c| c.to_ascii_lowercase().to_string())
        .unwrap_or_else(|| "p".into());
    if matches!(name.as_str(), "int" | "do" | "if" | "for" | "new" | "try") {
        name.push('_');
    }
    let mut candidate = name.clone();
    let mut n = 1;
    while taken.contains(&candidate) {
        n += 1;
        candidate = format!("{name}{n}");
    }
    candidate
}

/// Parses a class file. `Ok(None)` for classes that are not indexed
/// (synthetic, anonymous, local, `module-info`, `package-info`).
pub fn parse_class(bytes: &[u8]) -> Res<Option<ParsedClass>> {
    let mut c = Cur { b: bytes, at: 0 };
    if c.u4()? != 0xCAFE_BABE {
        return Err(ClassFileError::BadMagic);
    }
    c.u2()?;
    c.u2()?;
    let pool = read_pool(&mut c)?;
    let access = c.u2()?;
    let this = pool.class_name(c.u2()?).to_string();
    let super_idx = c.u2()?;
    let super_name = if super_idx == 0 {
        String::new()
    } else {
        pool.class_name(super_idx).to_string()
    };
    let mut iface_names = Vec::new();
    for _ in 0..c.u2()? {
        iface_names.push(pool.class_name(c.u2()?).to_string());
    }
    let fields = read_members(&mut c, &pool)?;
    let methods = read_members(&mut c, &pool)?;
    let attrs = read_attrs(&mut c, &pool)?;

    if this.ends_with("module-info")
        || this.ends_with("package-info")
        || access & ACC_SYNTHETIC != 0
    {
        return Ok(None);
    }
    if find(&attrs, "EnclosingMethod").is_some() {
        return Ok(None);
    }
    // InnerClasses
    let mut inners: Vec<Inner> = Vec::new();
    if let Some(d) = find(&attrs, "InnerClasses") {
        let mut ic = Cur { b: d, at: 0 };
        for _ in 0..ic.u2()? {
            let inner = ic.u2()?;
            let outer = ic.u2()?;
            let name = ic.u2()?;
            let fl = ic.u2()?;
            inners.push(Inner {
                inner: pool.class_name(inner).to_string(),
                outer: if outer == 0 {
                    String::new()
                } else {
                    pool.class_name(outer).to_string()
                },
                name: if name == 0 {
                    String::new()
                } else {
                    pool.utf8(name).to_string()
                },
                flags: fl,
            });
        }
    }
    let me_inner = inners.iter().find(|e| e.inner == this);
    // Anonymous (no name) or local (no outer, has name): not indexed.
    if let Some(e) = me_inner
        && (e.name.is_empty() || e.outer.is_empty())
    {
        return Ok(None);
    }
    let names = Names { inners: &inners };
    let qname = names.source(&this);
    let simple = qname.rsplit('.').next().unwrap_or(&qname).to_string();
    let effective_access = me_inner.map_or(access, |e| e.flags | (access & ACC_INTERFACE));
    let kind = if access & ACC_ANNOTATION != 0 {
        ClassKind::Annotation
    } else if access & ACC_INTERFACE != 0 {
        ClassKind::Interface
    } else if access & ACC_ENUM != 0 {
        ClassKind::Enum
    } else if super_name == "java/lang/Record" {
        ClassKind::Record
    } else {
        ClassKind::Class
    };
    let is_interface = matches!(kind, ClassKind::Interface | ClassKind::Annotation);
    let outer = me_inner.map(|e| e.outer.clone());
    let is_static_member = me_inner.is_some_and(|e| e.flags & ACC_STATIC != 0);
    let inner_non_static = me_inner.is_some() && !is_static_member && kind == ClassKind::Class;

    let mut class_flags = modifier_flags(effective_access, false);
    if is_interface {
        class_flags |= flags::ABSTRACT;
        class_flags &= !flags::FINAL;
    }
    let mut stub = ClassStub {
        name: simple,
        qname: qname.clone(),
        kind,
        flags: class_flags,
        deprecated: find(&attrs, "Deprecated").is_some(),
        type_params: Vec::new(),
        extends: Vec::new(),
        implements: Vec::new(),
        permits: Vec::new(),
        annotations: Vec::new(),
        fields: Vec::new(),
        methods: Vec::new(),
        inner: Vec::new(),
        record_components: Vec::new(),
        source_file: find(&attrs, "SourceFile").map(|d| {
            let idx = u16::from_be_bytes([
                d.first().copied().unwrap_or(0),
                d.get(1).copied().unwrap_or(0),
            ]);
            pool.utf8(idx).to_string()
        }),
        compiled: true,
        is_static_member: is_static_member
            || (is_interface || matches!(kind, ClassKind::Enum | ClassKind::Record))
                && me_inner.is_some(),
    };
    // Supertypes and type parameters.
    if let Some(d) = find(&attrs, "Signature") {
        let idx = u16::from_be_bytes([d[0], d[1]]);
        let sig = pool.utf8(idx);
        let mut p = SigParser {
            s: sig.as_bytes(),
            at: 0,
            names: &names,
        };
        stub.type_params = p.type_params();
        let sup = p.ty();
        let mut ifaces = Vec::new();
        while p.at < sig.len() {
            ifaces.push(p.ty());
        }
        if is_interface {
            stub.extends = ifaces;
        } else {
            if sup != "java.lang.Object" && !matches!(kind, ClassKind::Enum | ClassKind::Record) {
                stub.extends.push(sup);
            }
            stub.implements = ifaces;
        }
    } else {
        let name_of = |b: &str| names.source(b);
        if is_interface {
            stub.extends = iface_names.iter().map(|n| name_of(n)).collect();
        } else {
            if !super_name.is_empty()
                && super_name != "java/lang/Object"
                && !matches!(kind, ClassKind::Enum | ClassKind::Record)
            {
                stub.extends.push(name_of(&super_name));
            }
            stub.implements = iface_names.iter().map(|n| name_of(n)).collect();
        }
    }
    if matches!(kind, ClassKind::Annotation) {
        stub.extends.clear();
    }
    for key in ["RuntimeVisibleAnnotations", "RuntimeInvisibleAnnotations"] {
        if let Some(d) = find(&attrs, key) {
            stub.annotations.extend(annotations(d, &pool, &names));
        }
    }
    if stub
        .annotations
        .iter()
        .any(|a| a == "@java.lang.Deprecated")
    {
        stub.deprecated = true;
    }
    if let Some(d) = find(&attrs, "PermittedSubclasses") {
        let mut pc = Cur { b: d, at: 0 };
        for _ in 0..pc.u2()? {
            stub.permits.push(names.source(pool.class_name(pc.u2()?)));
        }
    }

    // Fields
    for f in &fields {
        if f.access & ACC_SYNTHETIC != 0 {
            continue;
        }
        let sig_idx = find(&f.attrs, "Signature").map(|d| u16::from_be_bytes([d[0], d[1]]));
        let text = pool.utf8(sig_idx.unwrap_or(0));
        let src = if sig_idx.is_some() { text } else { f.desc };
        let ty = SigParser {
            s: src.as_bytes(),
            at: 0,
            names: &names,
        }
        .ty();
        let init = find(&f.attrs, "ConstantValue").and_then(|d| {
            let idx = u16::from_be_bytes([*d.first()?, *d.get(1)?]);
            let is_final_static = f.access & ACC_STATIC != 0 && f.access & ACC_FINAL != 0;
            if !is_final_static {
                return None;
            }
            Some(match pool.0.get(idx as usize)? {
                Const::Int(v) => match ty.as_str() {
                    "boolean" => (*v != 0).to_string(),
                    "char" => format!("(char) {v}"),
                    _ => v.to_string(),
                },
                Const::Long(v) => format!("{v}L"),
                Const::Float(v) => float_text(f64::from(*v), "f"),
                Const::Double(v) => float_text(*v, ""),
                Const::Str(s) => java_string_literal(pool.utf8(*s)),
                _ => return None,
            })
        });
        let mut ann = Vec::new();
        for key in ["RuntimeVisibleAnnotations", "RuntimeInvisibleAnnotations"] {
            if let Some(d) = find(&f.attrs, key) {
                ann.extend(annotations(d, &pool, &names));
            }
        }
        let deprecated = find(&f.attrs, "Deprecated").is_some()
            || ann.iter().any(|a| a == "@java.lang.Deprecated");
        stub.fields.push(FieldStub {
            name: f.name.to_string(),
            flags: modifier_flags(f.access, false),
            ty,
            init,
            enum_constant: f.access & ACC_ENUM != 0,
            deprecated,
            annotations: ann,
        });
    }

    // Methods
    for m in &methods {
        if m.access & (ACC_SYNTHETIC | ACC_BRIDGE) != 0 || m.name == "<clinit>" {
            continue;
        }
        let is_ctor = m.name == "<init>";
        let sig_idx = find(&m.attrs, "Signature").map(|d| u16::from_be_bytes([d[0], d[1]]));
        let has_sig = sig_idx.is_some();
        let src = if has_sig {
            pool.utf8(sig_idx.unwrap_or(0))
        } else {
            m.desc
        };
        let mut p = SigParser {
            s: src.as_bytes(),
            at: 0,
            names: &names,
        };
        let type_params = p.type_params();
        let mut ptypes = Vec::new();
        if p.peek() == b'(' {
            p.bump();
            while p.peek() != b')' && p.at < src.len() {
                ptypes.push(p.ty());
            }
            p.bump();
        }
        let ret = p.ty();
        let mut throws = Vec::new();
        while p.peek() == b'^' {
            p.bump();
            throws.push(p.ty());
        }
        if throws.is_empty()
            && let Some(d) = find(&m.attrs, "Exceptions")
        {
            let mut ec = Cur { b: d, at: 0 };
            for _ in 0..ec.u2()? {
                throws.push(names.source(pool.class_name(ec.u2()?)));
            }
        }
        // Synthetic leading parameters (only when the descriptor was used).
        if is_ctor && !has_sig {
            let drop = if kind == ClassKind::Enum {
                2
            } else if inner_non_static {
                1
            } else {
                0
            };
            if ptypes.len() >= drop {
                ptypes.drain(..drop);
            }
        }
        let varargs = m.access & ACC_VARARGS != 0;
        // Parameter names (MethodParameters), else generated.
        let mut given: Vec<String> = Vec::new();
        if let Some(d) = find(&m.attrs, "MethodParameters") {
            let mut mc = Cur { b: d, at: 0 };
            if let Ok(n) = mc.u1() {
                for _ in 0..n {
                    let (Ok(ni), Ok(fl)) = (mc.u2(), mc.u2()) else {
                        break;
                    };
                    if fl & 0x9000 != 0 {
                        continue; // synthetic or mandated
                    }
                    given.push(if ni == 0 {
                        String::new()
                    } else {
                        pool.utf8(ni).to_string()
                    });
                }
            }
        }
        if given.is_empty()
            && let Some(code) = find(&m.attrs, "Code")
        {
            given = local_variable_names(code, m.desc, m.access & ACC_STATIC != 0, &pool);
        }
        if given.len() > ptypes.len() {
            // Leading synthetic parameters (outer instance, enum name and ordinal).
            let extra = given.len() - ptypes.len();
            given.drain(..extra);
        }
        let use_given = given.len() == ptypes.len() && given.iter().all(|n| !n.is_empty());
        let mut taken: Vec<String> = Vec::new();
        let mut params = Vec::new();
        for (k, ty) in ptypes.iter().enumerate() {
            let (name, generated) = if use_given {
                (given[k].clone(), false)
            } else {
                (generated_param_name(ty, &taken), true)
            };
            taken.push(name.clone());
            params.push(ParamStub {
                name,
                ty: ty.clone(),
                varargs: varargs && k + 1 == ptypes.len(),
                generated,
            });
        }
        let mut ann = Vec::new();
        for key in ["RuntimeVisibleAnnotations", "RuntimeInvisibleAnnotations"] {
            if let Some(d) = find(&m.attrs, key) {
                ann.extend(annotations(d, &pool, &names));
            }
        }
        let deprecated = find(&m.attrs, "Deprecated").is_some()
            || ann.iter().any(|a| a == "@java.lang.Deprecated");
        let mut mflags = modifier_flags(m.access, true);
        if is_interface && m.access & (ACC_ABSTRACT | ACC_STATIC | ACC_PRIVATE) == 0 {
            mflags |= flags::DEFAULT;
        }
        stub.methods.push(MethodStub {
            name: if is_ctor {
                stub.name.clone()
            } else {
                m.name.to_string()
            },
            flags: mflags,
            type_params,
            ret: if is_ctor { None } else { Some(ret) },
            params,
            throws,
            annotations: ann,
            is_ctor,
            varargs,
            deprecated,
            has_default: find(&m.attrs, "AnnotationDefault").is_some(),
        });
    }
    // Record components.
    if let Some(d) = find(&attrs, "Record") {
        let mut rc = Cur { b: d, at: 0 };
        for _ in 0..rc.u2()? {
            let name = pool.utf8(rc.u2()?).to_string();
            let desc = pool.utf8(rc.u2()?);
            let mut sig = desc.to_string();
            for _ in 0..rc.u2()? {
                let an = pool.utf8(rc.u2()?);
                let len = rc.u4()? as usize;
                let data = rc.take(len)?;
                if an == "Signature" && data.len() >= 2 {
                    sig = pool
                        .utf8(u16::from_be_bytes([data[0], data[1]]))
                        .to_string();
                }
            }
            let ty = SigParser {
                s: sig.as_bytes(),
                at: 0,
                names: &names,
            }
            .ty();
            stub.record_components.push(ParamStub {
                name,
                ty,
                varargs: false,
                generated: false,
            });
        }
    }
    Ok(Some(ParsedClass {
        binary_name: this,
        outer: outer.filter(|o| !o.is_empty()),
        stub,
    }))
}

/// Attaches member classes to their outer classes (JAVA-RES-039). A class
/// whose outer class is missing stays a top-level-like entity. Returns the
/// top-level stubs.
pub fn nest(classes: Vec<ParsedClass>) -> Vec<ClassStub> {
    let mut by_name: std::collections::BTreeMap<String, ParsedClass> = classes
        .into_iter()
        .map(|c| (c.binary_name.clone(), c))
        .collect();
    // Deepest first so that inner classes are complete before they move.
    let mut order: Vec<String> = by_name.keys().cloned().collect();
    order.sort_by_key(|n| std::cmp::Reverse(n.matches('$').count()));
    for name in order {
        let Some(outer) = by_name.get(&name).and_then(|c| c.outer.clone()) else {
            continue;
        };
        if outer == name || !by_name.contains_key(&outer) {
            continue;
        }
        if let Some(child) = by_name.remove(&name)
            && let Some(parent) = by_name.get_mut(&outer)
        {
            parent.stub.inner.push(child.stub);
        }
    }
    let mut out: Vec<ClassStub> = by_name.into_values().map(|c| c.stub).collect();
    fn sort_inner(c: &mut ClassStub) {
        c.inner.sort_by(|a, b| a.name.cmp(&b.name));
        c.inner.iter_mut().for_each(sort_inner);
    }
    out.iter_mut().for_each(sort_inner);
    out
}
