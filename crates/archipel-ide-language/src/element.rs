//! Element types and token sets: the `IElementType`/`TokenSet` analogue built
//! from a grammar's node kinds, so generic features need no per-language enum.

use std::collections::HashMap;

/// A node kind of one language: the grammar's kind id (tree-sitter
/// `Node::kind_id`), interned per language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ElementType(pub u16);

impl ElementType {
    /// Kind id tree-sitter gives ERROR nodes.
    pub const ERROR: ElementType = ElementType(u16::MAX);
}

/// A grammar field ("named slot" such as `name`, `body`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FieldId(pub u16);

/// The kinds and fields of one grammar.
#[derive(Debug)]
pub struct ElementTypeTable {
    names: Vec<&'static str>,
    named: Vec<bool>,
    by_name: HashMap<&'static str, Vec<u16>>,
    fields: HashMap<&'static str, FieldId>,
}

impl ElementTypeTable {
    pub fn build(language: &tree_sitter::Language) -> Self {
        let count = language.node_kind_count();
        let mut names = Vec::with_capacity(count);
        let mut named = Vec::with_capacity(count);
        let mut by_name: HashMap<&'static str, Vec<u16>> = HashMap::new();
        for id in 0..count {
            let Ok(id) = u16::try_from(id) else { break };
            let name = language.node_kind_for_id(id).unwrap_or("");
            names.push(name);
            named.push(language.node_kind_is_named(id));
            by_name.entry(name).or_default().push(id);
        }
        let mut fields = HashMap::new();
        for id in 1..=language.field_count() {
            let Ok(id) = u16::try_from(id) else { break };
            if let Some(name) = language.field_name_for_id(id) {
                fields.insert(name, FieldId(id));
            }
        }
        Self {
            names,
            named,
            by_name,
            fields,
        }
    }

    pub fn len(&self) -> usize {
        self.names.len()
    }

    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }

    /// Kind name (`"ERROR"` for error nodes, `""` for unknown ids).
    pub fn name(&self, ty: ElementType) -> &'static str {
        if ty == ElementType::ERROR {
            return "ERROR";
        }
        self.names.get(usize::from(ty.0)).copied().unwrap_or("")
    }

    pub fn is_named(&self, ty: ElementType) -> bool {
        self.named.get(usize::from(ty.0)).copied().unwrap_or(false)
    }

    /// All kind ids with this name (grammars alias some kinds).
    pub fn lookup(&self, name: &str) -> impl Iterator<Item = ElementType> + '_ {
        self.by_name
            .get(name)
            .into_iter()
            .flatten()
            .map(|id| ElementType(*id))
    }

    pub fn contains_name(&self, name: &str) -> bool {
        self.by_name.contains_key(name)
    }

    pub fn field(&self, name: &str) -> Option<FieldId> {
        self.fields.get(name).copied()
    }

    /// A token set of every kind named in `names` (unknown names are
    /// ignored; [`crate::LanguageDef::unknown_token_names`] reports them).
    pub fn token_set(&self, names: &[&str]) -> TokenSet {
        let mut set = TokenSet::with_capacity(self.len());
        for name in names {
            for ty in self.lookup(name) {
                set.insert(ty);
            }
        }
        set
    }
}

/// A bitset over a language's [`ElementType`]s.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TokenSet {
    bits: Vec<u64>,
}

impl TokenSet {
    pub const fn empty() -> Self {
        Self { bits: Vec::new() }
    }

    pub fn with_capacity(kinds: usize) -> Self {
        Self {
            bits: vec![0; kinds.div_ceil(64)],
        }
    }

    pub fn of(types: &[ElementType]) -> Self {
        let mut set = Self::empty();
        for ty in types {
            set.insert(*ty);
        }
        set
    }

    pub fn insert(&mut self, ty: ElementType) {
        let word = usize::from(ty.0) / 64;
        if word >= self.bits.len() {
            self.bits.resize(word + 1, 0);
        }
        self.bits[word] |= 1 << (ty.0 % 64);
    }

    pub fn contains(&self, ty: ElementType) -> bool {
        self.bits
            .get(usize::from(ty.0) / 64)
            .is_some_and(|word| word & (1 << (ty.0 % 64)) != 0)
    }

    pub fn is_empty(&self) -> bool {
        self.bits.iter().all(|word| *word == 0)
    }

    pub fn len(&self) -> usize {
        self.bits
            .iter()
            .map(|word| word.count_ones() as usize)
            .sum()
    }

    pub fn union(&self, other: &TokenSet) -> TokenSet {
        let len = self.bits.len().max(other.bits.len());
        let word = |set: &TokenSet, index: usize| set.bits.get(index).copied().unwrap_or(0);
        TokenSet {
            bits: (0..len)
                .map(|index| word(self, index) | word(other, index))
                .collect(),
        }
    }

    pub fn intersection(&self, other: &TokenSet) -> TokenSet {
        TokenSet {
            bits: self
                .bits
                .iter()
                .zip(&other.bits)
                .map(|(a, b)| a & b)
                .collect(),
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = ElementType> + '_ {
        self.bits.iter().enumerate().flat_map(|(index, word)| {
            (0..64)
                .filter(move |bit| word & (1u64 << bit) != 0)
                .map(move |bit| ElementType((index * 64 + bit) as u16))
        })
    }
}

/// Token sets of a language by kind NAME (data, resolved against the grammar's
/// [`ElementTypeTable`] on first use). The IntelliJ `ParserDefinition` token
/// sets plus the structural groups generic features need.
#[derive(Debug, Clone, Copy)]
pub struct TokenSetSpec {
    /// Usually empty: tree-sitter has no whitespace nodes.
    pub whitespace: &'static [&'static str],
    pub comments: &'static [&'static str],
    pub string_literals: &'static [&'static str],
    pub identifiers: &'static [&'static str],
    pub brackets: &'static [&'static str],
    pub statement_like: &'static [&'static str],
    pub declaration_like: &'static [&'static str],
    pub code_block_like: &'static [&'static str],
}

impl TokenSetSpec {
    pub const EMPTY: TokenSetSpec = TokenSetSpec {
        whitespace: &[],
        comments: &[],
        string_literals: &[],
        identifiers: &[],
        brackets: &[],
        statement_like: &[],
        declaration_like: &[],
        code_block_like: &[],
    };

    pub(crate) fn names(&self) -> impl Iterator<Item = &'static str> {
        [
            self.whitespace,
            self.comments,
            self.string_literals,
            self.identifiers,
            self.brackets,
            self.statement_like,
            self.declaration_like,
            self.code_block_like,
        ]
        .into_iter()
        .flatten()
        .copied()
    }
}

/// Resolved [`TokenSetSpec`].
#[derive(Debug, Clone, Default)]
pub struct TokenSets {
    pub whitespace: TokenSet,
    pub comments: TokenSet,
    pub string_literals: TokenSet,
    pub identifiers: TokenSet,
    pub brackets: TokenSet,
    pub statement_like: TokenSet,
    pub declaration_like: TokenSet,
    pub code_block_like: TokenSet,
}

impl TokenSets {
    pub fn resolve(spec: &TokenSetSpec, table: &ElementTypeTable) -> Self {
        Self {
            whitespace: table.token_set(spec.whitespace),
            comments: table.token_set(spec.comments),
            string_literals: table.token_set(spec.string_literals),
            identifiers: table.token_set(spec.identifiers),
            brackets: table.token_set(spec.brackets),
            statement_like: table.token_set(spec.statement_like),
            declaration_like: table.token_set(spec.declaration_like),
            code_block_like: table.token_set(spec.code_block_like),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_set_operations() {
        let mut a = TokenSet::empty();
        a.insert(ElementType(3));
        a.insert(ElementType(130));
        let b = TokenSet::of(&[ElementType(3), ElementType(7)]);
        assert!(a.contains(ElementType(130)));
        assert!(!a.contains(ElementType(7)));
        assert_eq!(a.union(&b).len(), 3);
        assert_eq!(
            a.intersection(&b).iter().collect::<Vec<_>>(),
            vec![ElementType(3)]
        );
        assert!(TokenSet::empty().is_empty());
    }

    #[test]
    fn table_is_built_from_the_grammar() {
        let table = ElementTypeTable::build(&tree_sitter_rust::LANGUAGE.into());
        let item = table.lookup("function_item").next().expect("kind");
        assert_eq!(table.name(item), "function_item");
        assert!(table.is_named(item));
        assert!(table.field("name").is_some());
        assert!(!table.token_set(&["line_comment", "nope"]).is_empty());
    }
}
