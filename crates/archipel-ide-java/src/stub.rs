//! The Java stub model (JAVA-STUB-005..007, 010): what one file contributes
//! to the global indexes, as text, without positions and without bodies.
//!
//! Source files and class files produce the same shape (JAVA-STUB-004). Type
//! names are stored as written (source) or as fully qualified source text
//! (class files); resolution happens later in the context of the declaring
//! file (JAVA-STUB-002).

use serde::{Deserialize, Serialize};

/// Explicit modifier bits (JAVA-STUB-005); implicit ones are computed from the owner.
pub mod flags {
    pub const PUBLIC: u32 = 1;
    pub const PRIVATE: u32 = 1 << 1;
    pub const PROTECTED: u32 = 1 << 2;
    pub const STATIC: u32 = 1 << 3;
    pub const FINAL: u32 = 1 << 4;
    pub const ABSTRACT: u32 = 1 << 5;
    pub const NATIVE: u32 = 1 << 6;
    pub const SYNCHRONIZED: u32 = 1 << 7;
    pub const TRANSIENT: u32 = 1 << 8;
    pub const VOLATILE: u32 = 1 << 9;
    pub const STRICTFP: u32 = 1 << 10;
    pub const DEFAULT: u32 = 1 << 11;
    pub const SEALED: u32 = 1 << 12;
    pub const NON_SEALED: u32 = 1 << 13;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ClassKind {
    Class,
    Interface,
    Enum,
    Annotation,
    Record,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypeParamStub {
    pub name: String,
    pub bounds: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParamStub {
    pub name: String,
    pub ty: String,
    pub varargs: bool,
    /// The name was generated (class file without parameter names).
    pub generated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MethodStub {
    pub name: String,
    pub flags: u32,
    pub type_params: Vec<TypeParamStub>,
    /// `None` for constructors.
    pub ret: Option<String>,
    pub params: Vec<ParamStub>,
    pub throws: Vec<String>,
    pub annotations: Vec<String>,
    pub is_ctor: bool,
    pub varargs: bool,
    pub deprecated: bool,
    /// Annotation element with a default value.
    pub has_default: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldStub {
    pub name: String,
    pub flags: u32,
    pub ty: String,
    /// Initializer text, kept only for constant candidates (JAVA-STUB-007).
    pub init: Option<String>,
    pub enum_constant: bool,
    pub deprecated: bool,
    pub annotations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClassStub {
    pub name: String,
    /// Fully qualified source name (`a.b.Outer.Inner`).
    pub qname: String,
    pub kind: ClassKind,
    pub flags: u32,
    pub deprecated: bool,
    pub type_params: Vec<TypeParamStub>,
    pub extends: Vec<String>,
    pub implements: Vec<String>,
    pub permits: Vec<String>,
    pub annotations: Vec<String>,
    pub fields: Vec<FieldStub>,
    pub methods: Vec<MethodStub>,
    pub inner: Vec<ClassStub>,
    pub record_components: Vec<ParamStub>,
    /// Class-file `SourceFile` attribute.
    pub source_file: Option<String>,
    /// Built from a class file (otherwise from source).
    pub compiled: bool,
    /// Class kind is implied `static` (member interfaces, enums, records) or the
    /// class is a non-static inner class (compiled only).
    pub is_static_member: bool,
}

impl ClassStub {
    pub fn is_interface_like(&self) -> bool {
        matches!(self.kind, ClassKind::Interface | ClassKind::Annotation)
    }
    pub fn has_flag(&self, f: u32) -> bool {
        self.flags & f != 0
    }
    /// Direct member class by simple name.
    pub fn inner_named(&self, name: &str) -> Option<&ClassStub> {
        self.inner.iter().find(|c| c.name == name)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ImportStub {
    /// Reference text without `import`, `static`, `;` and `.*`.
    pub name: String,
    pub on_demand: bool,
    pub is_static: bool,
}

/// The stub of one compilation unit (JAVA-STUB-001).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct FileStub {
    pub package: String,
    pub imports: Vec<ImportStub>,
    pub classes: Vec<ClassStub>,
}

impl FileStub {
    pub fn encode(&self) -> Vec<u8> {
        postcard::to_stdvec(self).unwrap_or_default()
    }
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        postcard::from_bytes(bytes).ok()
    }
}

impl ClassStub {
    pub fn encode(&self) -> Vec<u8> {
        postcard::to_stdvec(self).unwrap_or_default()
    }
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        postcard::from_bytes(bytes).ok()
    }
    /// Visits this class and every nested class (pre-order) with the nesting path.
    pub fn walk<'a>(&'a self, path: &mut Vec<&'a str>, f: &mut impl FnMut(&[&'a str], &'a ClassStub)) {
        path.push(&self.name);
        f(path, self);
        for inner in &self.inner {
            inner.walk(path, f);
        }
        path.pop();
    }
}

/// Short name of a type text: qualifier, type arguments and array suffix removed
/// (`java.util.Map.Entry<K, V>[]` becomes `Entry`).
pub fn short_type_name(text: &str) -> &str {
    let base = text.split('<').next().unwrap_or(text);
    let base = base.trim().trim_end_matches("[]").trim_end_matches("...").trim();
    base.rsplit('.').next().unwrap_or(base)
}

/// Annotation short name from its text (`@org.junit.Test(x)` gives `Test`).
pub fn annotation_short_name(text: &str) -> &str {
    let t = text.trim_start_matches('@');
    let t = t.split('(').next().unwrap_or(t).trim();
    t.rsplit('.').next().unwrap_or(t)
}
