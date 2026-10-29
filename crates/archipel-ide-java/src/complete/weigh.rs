//! The weigher chain of Java completion (spec 06 6.5, JAVA-CMP-027/028): every item gets a sort
//! key, a string that compares in chain order, so the popup, the case runner and the tests agree.
//!
//! The platform ranking compares the prefix match quality and the usage statistics first (they
//! need the typed prefix and the user's history); what is left to the engine is the rest of the
//! chain, most significant first:
//!
//! 1. the kind tier (6.5 item 5): probable keyword, variable of the expected type, variable,
//!    normal members, expected-type methods, suitable class, class names and statics not yet
//!    imported, improbable keyword;
//! 2. discouraged packages after the others (JAVA-CMP-025), deprecated after non-deprecated;
//! 3. the expected-type match (item 6): assignable before the rest;
//! 4. the statics preference (item 7): after `Class.` statics first, elsewhere instance members
//!    before statics;
//! 5. proximity (item 11): usable from this place, explicitly imported, nearer on the classpath,
//!    nearer in the hierarchy, members of `Object` last;
//! 6. shorter names first (item 12).

/// The tiers of the Java kind weigher (smaller is better), in the order of spec 06 6.5 item 5
/// (JAVA-CMP-028: variables of the expected type, then variables, then methods, then classes).
pub mod tier {
    pub const PROBABLE_KEYWORD: u8 = 1;
    pub const VARIABLE_OF_EXPECTED_TYPE: u8 = 3;
    pub const VARIABLE: u8 = 6;
    /// Statement keywords and modifiers valid at the position.
    pub const KEYWORD: u8 = 8;
    /// Primitive types as declaration types.
    pub const PRIMITIVE_TYPE: u8 = 14;
    pub const NORMAL: u8 = 10;
    pub const EXPECTED_TYPE_METHOD: u8 = 9;
    pub const SUITABLE_CLASS: u8 = 15;
    pub const CLASS_NAME: u8 = 17;
    pub const IMPROBABLE_KEYWORD: u8 = 19;
    pub const REST: u8 = 20;
}

/// Match of an item's type with the expected type (JAVA-CMP-027 "expected-type match").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fit {
    Matches,
    /// No expected type, or the type of the item is not known.
    Unknown,
    DoesNotMatch,
}

/// The inputs of one sort key.
#[derive(Debug, Clone)]
pub struct Weights<'a> {
    pub tier: u8,
    pub discouraged: bool,
    pub deprecated: bool,
    pub fit: Fit,
    /// 0: preferred by the statics rule, 1: not.
    pub statics: u8,
    /// Smaller is nearer (usable, explicit import, classpath rank, hierarchy distance), up to 99999.
    pub proximity: u32,
    pub name: &'a str,
}

impl Weights<'_> {
    pub fn key(&self) -> String {
        format!(
            "{:02}{}{}{}{}{:05}{:04}{}",
            self.tier,
            u8::from(self.discouraged),
            u8::from(self.deprecated),
            match self.fit {
                Fit::Matches => 0,
                Fit::Unknown => 1,
                Fit::DoesNotMatch => 2,
            },
            self.statics,
            self.proximity.min(99_999),
            self.name.len().min(9_999),
            self.name.to_lowercase()
        )
    }
}

/// The key of a class name: classes the file already sees, then the nearer classpath entries.
pub fn class_key(
    suitable: bool,
    discouraged: bool,
    deprecated: bool,
    visible: bool,
    unit_rank: u32,
    name: &str,
) -> String {
    Weights {
        tier: if suitable {
            tier::SUITABLE_CLASS
        } else {
            tier::CLASS_NAME
        },
        discouraged,
        deprecated,
        fit: Fit::Unknown,
        statics: 0,
        proximity: u32::from(!visible) * 50_000 + unit_rank.min(49_999),
        name,
    }
    .key()
}
