//! Reading entry names of a jar (a zip central directory) without decompressing anything, and
//! answering "which library on this classpath provides this class".
//!
//! This is the minimum the project model needs to demonstrate library lookup; class-file
//! stubs and a persistent jar index belong to the Java analysis (Phase 4).

use archipel_ide_projectmodel::{ClasspathKind, Library, Module, ProjectModel};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

const EOCD_SIGNATURE: u32 = 0x0605_4b50;
const CENTRAL_SIGNATURE: u32 = 0x0201_4b50;

fn u16_at(bytes: &[u8], at: usize) -> usize {
    bytes
        .get(at..at + 2)
        .map_or(0, |b| u16::from_le_bytes([b[0], b[1]]) as usize)
}

fn u32_at(bytes: &[u8], at: usize) -> u32 {
    bytes
        .get(at..at + 4)
        .map_or(0, |b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

/// Entry names of the zip archive at `path` (files and directories as stored).
pub fn entry_names(path: &Path) -> std::io::Result<Vec<String>> {
    let mut file = File::open(path)?;
    let length = file.metadata()?.len();
    // The end-of-central-directory record is in the last 64 KiB + 22 bytes.
    let tail_len = length.min(65_557);
    file.seek(SeekFrom::Start(length - tail_len))?;
    let mut tail = vec![0_u8; tail_len as usize];
    file.read_exact(&mut tail)?;
    let eocd = (0..tail.len().saturating_sub(21))
        .rev()
        .find(|&at| u32_at(&tail, at) == EOCD_SIGNATURE)
        .ok_or_else(|| std::io::Error::other("not a zip archive"))?;
    let count = u16_at(&tail, eocd + 10);
    let size = u32_at(&tail, eocd + 12) as u64;
    let offset = u32_at(&tail, eocd + 16) as u64;
    if offset == u32::MAX as u64 || count == 0xFFFF {
        return Err(std::io::Error::other("zip64 archives are not supported"));
    }
    file.seek(SeekFrom::Start(offset))?;
    let mut directory = vec![0_u8; size as usize];
    file.read_exact(&mut directory)?;
    let mut names = Vec::with_capacity(count);
    let mut at = 0;
    while at + 46 <= directory.len() && u32_at(&directory, at) == CENTRAL_SIGNATURE {
        let name_len = u16_at(&directory, at + 28);
        let extra_len = u16_at(&directory, at + 30);
        let comment_len = u16_at(&directory, at + 32);
        let name = directory
            .get(at + 46..at + 46 + name_len)
            .map(|b| String::from_utf8_lossy(b).into_owned())
            .unwrap_or_default();
        names.push(name);
        at += 46 + name_len + extra_len + comment_len;
    }
    Ok(names)
}

/// Class file names a qualified name can be stored under: `org.acme.Outer.Inner` is
/// `org/acme/Outer.class`'s nested class `org/acme/Outer$Inner.class`.
fn class_entry_candidates(qualified_name: &str) -> Vec<String> {
    let segments: Vec<&str> = qualified_name.split('.').collect();
    (0..segments.len())
        .map(|k| {
            let package = segments[..k].join("/");
            let class = segments[k..].join("$");
            if package.is_empty() {
                format!("{class}.class")
            } else {
                format!("{package}/{class}.class")
            }
        })
        .collect()
}

/// Whether the jar contains the class `org.acme.Outer.Inner` (as `org/acme/Outer.class` or
/// `org/acme/Outer$Inner.class`).
pub fn contains_class(jar: &Path, qualified_name: &str) -> bool {
    let Ok(names) = entry_names(jar) else {
        return false;
    };
    let candidates = class_entry_candidates(qualified_name);
    names.iter().any(|name| candidates.contains(name))
}

/// The first library on `module`'s classpath of `kind` that has a classes root providing
/// `qualified_name`: "which jar provides `org.assertj.core.api.Assertions`".
pub fn provider_of<'a>(
    model: &'a ProjectModel,
    module: &'a Module,
    kind: ClasspathKind,
    qualified_name: &str,
) -> Option<&'a Library> {
    model
        .libraries_for(module, kind)
        .into_iter()
        .find(|library| {
            library
                .classes
                .iter()
                .any(|jar| contains_class(jar, qualified_name))
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// A stored (uncompressed) zip with the given entry names, written by hand.
    fn write_zip(path: &Path, names: &[&str]) {
        let mut out: Vec<u8> = Vec::new();
        let mut central: Vec<u8> = Vec::new();
        for name in names {
            let offset = out.len() as u32;
            out.extend_from_slice(&0x0403_4b50_u32.to_le_bytes());
            out.extend_from_slice(&[10, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
            out.extend_from_slice(&0_u32.to_le_bytes()); // crc
            out.extend_from_slice(&0_u32.to_le_bytes()); // compressed
            out.extend_from_slice(&0_u32.to_le_bytes()); // uncompressed
            out.extend_from_slice(&(name.len() as u16).to_le_bytes());
            out.extend_from_slice(&0_u16.to_le_bytes());
            out.extend_from_slice(name.as_bytes());
            central.extend_from_slice(&CENTRAL_SIGNATURE.to_le_bytes());
            central.extend_from_slice(&[20, 0, 10, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
            central.extend_from_slice(&[0; 12]); // crc, sizes
            central.extend_from_slice(&(name.len() as u16).to_le_bytes());
            central.extend_from_slice(&[0; 12]); // extra, comment, disk, attrs
            central.extend_from_slice(&offset.to_le_bytes());
            central.extend_from_slice(name.as_bytes());
        }
        let central_offset = out.len() as u32;
        out.extend_from_slice(&central);
        out.extend_from_slice(&EOCD_SIGNATURE.to_le_bytes());
        out.extend_from_slice(&[0, 0, 0, 0]);
        out.extend_from_slice(&(names.len() as u16).to_le_bytes());
        out.extend_from_slice(&(names.len() as u16).to_le_bytes());
        out.extend_from_slice(&(central.len() as u32).to_le_bytes());
        out.extend_from_slice(&central_offset.to_le_bytes());
        out.extend_from_slice(&0_u16.to_le_bytes());
        std::fs::File::create(path)
            .unwrap()
            .write_all(&out)
            .unwrap();
    }

    #[test]
    fn lists_entries_and_finds_classes_including_nested_ones() {
        let dir = std::env::temp_dir().join(format!("archipel-jar-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let jar = dir.join("a.jar");
        write_zip(
            &jar,
            &[
                "META-INF/MANIFEST.MF",
                "org/assertj/core/api/Assertions.class",
                "org/assertj/core/api/Assertions$Inner.class",
                "Top.class",
            ],
        );
        assert_eq!(entry_names(&jar).unwrap().len(), 4);
        assert!(contains_class(&jar, "org.assertj.core.api.Assertions"));
        assert!(contains_class(
            &jar,
            "org.assertj.core.api.Assertions.Inner"
        ));
        assert!(contains_class(&jar, "Top"));
        assert!(!contains_class(&jar, "org.assertj.core.api.Missing"));
        assert!(!contains_class(&dir.join("nope.jar"), "Top"));
        std::fs::write(dir.join("bad.jar"), b"not a zip").unwrap();
        assert!(entry_names(&dir.join("bad.jar")).is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn finds_the_providing_library_on_a_classpath() {
        use archipel_ide_projectmodel::{DependencyScope, OrderEntry};
        let dir =
            std::env::temp_dir().join(format!("archipel-jar-provider-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let assertj = dir.join("assertj.jar");
        let junit = dir.join("junit.jar");
        write_zip(&assertj, &["org/assertj/core/api/Assertions.class"]);
        write_zip(&junit, &["org/junit/Test.class"]);
        let mut model = ProjectModel::new("p", "/p");
        model.libraries = vec![
            Library::new("assertj").with_classes(&assertj),
            Library::new("junit").with_classes(&junit),
        ];
        let mut module = Module::new("m");
        module.add_order_entry(OrderEntry::library("assertj", DependencyScope::Test));
        module.add_order_entry(OrderEntry::library("junit", DependencyScope::Compile));
        model.modules.push(module);
        let module = &model.modules[0];
        let name = "org.assertj.core.api.Assertions";
        assert_eq!(
            provider_of(&model, module, ClasspathKind::TestCompile, name)
                .unwrap()
                .name,
            "assertj"
        );
        assert!(provider_of(&model, module, ClasspathKind::ProductionCompile, name).is_none());
        assert_eq!(
            provider_of(
                &model,
                module,
                ClasspathKind::ProductionCompile,
                "org.junit.Test"
            )
            .unwrap()
            .name,
            "junit"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
