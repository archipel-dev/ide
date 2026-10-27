//! The god object may only shrink.
//!
//! `IdeState` once held every feature's state. Features now live in services of the platform
//! kernel (`archipel-ide-platform`; see `docs/architecture/platform-kernel.md`), and each move
//! lowers [`IDE_STATE_FIELD_BASELINE`]. This test fails when `IdeState` has more fields than the
//! baseline, so a new feature must bring its own service instead of adding a field.
//!
//! - Moved fields into a service? Lower the baseline to the new count (the failure message
//!   after your change says "lower the baseline").
//! - Need new state? Add a service (docs: "Add a service") rather than a field.

/// The number of fields `IdeState` may have. Lower it when fields leave; never raise it.
const IDE_STATE_FIELD_BASELINE: usize = 83;

/// The fields of `pub struct IdeState`, read from the source: the lines of the struct body that
/// start a field at the struct's indentation (doc comments, attributes and continuation lines of
/// multi-line types are skipped).
fn ide_state_fields() -> Vec<String> {
    let source = include_str!("../src/lib.rs");
    let start = source
        .find("pub struct IdeState {")
        .expect("lib.rs declares `pub struct IdeState`");
    let body = &source[start..];
    let end = body
        .find("\n}")
        .expect("the struct ends with a closing brace");
    body[..end]
        .lines()
        .skip(1)
        .filter_map(|line| {
            let field = line.strip_prefix("    ")?;
            if field.starts_with([' ', '/', '#', '}', ')']) {
                return None;
            }
            let field = field
                .strip_prefix("pub(crate) ")
                .or_else(|| field.strip_prefix("pub "))
                .unwrap_or(field);
            let (name, _) = field.split_once(':')?;
            name.chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
                .then(|| name.to_owned())
        })
        .collect()
}

#[test]
fn the_field_reader_finds_the_fields() {
    let fields = ide_state_fields();
    for known in ["platform", "project", "documents", "status", "waker"] {
        assert!(
            fields.iter().any(|f| f == known),
            "expected to find the field `{known}`, found {fields:?}"
        );
    }
    let unique: std::collections::BTreeSet<_> = fields.iter().collect();
    assert_eq!(unique.len(), fields.len(), "field names are unique");
}

#[test]
fn ide_state_only_shrinks() {
    let fields = ide_state_fields();
    assert!(
        fields.len() <= IDE_STATE_FIELD_BASELINE,
        "IdeState has {} fields, the baseline is {IDE_STATE_FIELD_BASELINE}. Do not add fields \
         to the god object: give the feature a service in the platform kernel \
         (docs/architecture/platform-kernel.md, \"Add a service\"). Fields: {fields:?}",
        fields.len()
    );
    assert!(
        fields.len() >= IDE_STATE_FIELD_BASELINE,
        "IdeState has {} fields, fewer than the baseline {IDE_STATE_FIELD_BASELINE}: lower the \
         baseline in tests/ide_state_size.rs to {} so it cannot grow back.",
        fields.len(),
        fields.len()
    );
}

/// The migration checklist in the kernel doc names every remaining field, so it cannot go stale.
#[test]
fn every_field_is_in_the_migration_checklist() {
    let doc = include_str!("../../../docs/architecture/platform-kernel.md");
    let missing: Vec<String> = ide_state_fields()
        .into_iter()
        .filter(|field| !doc.contains(&format!("`{field}`")))
        .collect();
    assert!(
        missing.is_empty(),
        "docs/architecture/platform-kernel.md has no checklist entry for: {missing:?}"
    );
}
