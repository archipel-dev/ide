//! File interface summaries (the early-cutoff input of D5): a hash of a
//! file's declarations without their positions. Edits that keep it unchanged
//! do not bump the file's [`crate::InterfaceEpoch`].

use archipel_ide_language::SyntaxSnapshot;
use archipel_ide_language::structure::StructureItem;

/// Computes a file's interface hash. Registered per language (a stub builder
/// in WP2); the default [`StructureInterface`] serves every language.
pub trait InterfaceSummarizer: Send + Sync {
    fn interface_hash(&self, snap: &SyntaxSnapshot) -> u64;
}

/// Default: kinds, names, details and nesting of the tags-based structure.
/// Languages without a tags query hash their whole text (any edit changes
/// the interface, which is conservative).
#[derive(Debug, Clone, Copy, Default)]
pub struct StructureInterface;

/// FNV-1a 64 over a byte stream.
#[derive(Debug, Clone, Copy)]
pub struct Fnv64(pub u64);

impl Default for Fnv64 {
    fn default() -> Self {
        Self(0xcbf2_9ce4_8422_2325)
    }
}

impl Fnv64 {
    pub fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 ^= u64::from(*byte);
            self.0 = self.0.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }

    pub fn finish(self) -> u64 {
        self.0
    }
}

fn hash_items(items: &[StructureItem], hasher: &mut Fnv64) {
    hasher.write(b"[");
    for item in items {
        hasher.write(format!("{:?}", item.kind).as_bytes());
        hasher.write(b"\0");
        hasher.write(item.name.as_bytes());
        hasher.write(b"\0");
        if let Some(detail) = &item.detail {
            hasher.write(detail.as_bytes());
        }
        hash_items(&item.children, hasher);
    }
    hasher.write(b"]");
}

impl InterfaceSummarizer for StructureInterface {
    fn interface_hash(&self, snap: &SyntaxSnapshot) -> u64 {
        let mut hasher = Fnv64::default();
        hasher.write(snap.lang().id.as_bytes());
        if snap.lang().has_tags() {
            hash_items(&snap.structure(), &mut hasher);
        } else {
            hasher.write(snap.text().as_bytes());
        }
        hasher.finish()
    }
}
