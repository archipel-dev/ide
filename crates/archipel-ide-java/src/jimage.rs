//! Reader for the JDK runtime image `lib/modules` (the "jimage" container).
//!
//! Choice (documented in `docs/architecture/java-intelligence.md`): the JDK is read from its
//! runtime image, as in JAVA-RES-036 and PLAT-IDX/PSI "JRT reader", not from `ct.sym`:
//! `lib/modules` holds the exact classes of the SDK the project runs on, one file, and
//! every JDK 9+ has it. `ct.sym` (older API levels, `--release`) is LATER (JAVA-RES-040).
//! Compressed resources (jlink `--compress`) are not supported and reported as skipped.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub enum JImageError {
    #[error("cannot read the runtime image: {0}")]
    Io(#[from] std::io::Error),
    #[error("not a jimage file")]
    Format,
}

/// One class resource of the image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageEntry {
    pub module: String,
    /// Path inside the module without extension (`java/lang/String`).
    pub name: String,
    pub offset: u64,
    pub size: u64,
    pub compressed: bool,
}

pub struct JImage {
    file: File,
    pub entries: Vec<ImageEntry>,
    pub skipped_compressed: usize,
}

fn le32(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

fn cstr(strings: &[u8], at: usize) -> String {
    let rest = strings.get(at..).unwrap_or(&[]);
    let end = rest.iter().position(|b| *b == 0).unwrap_or(rest.len());
    String::from_utf8_lossy(&rest[..end]).into_owned()
}

impl JImage {
    /// Opens the image and reads its location table (a few MB; resources are read on demand).
    pub fn open(path: &Path) -> Result<Self, JImageError> {
        let mut file = File::open(path)?;
        let mut header = [0u8; 28];
        file.read_exact(&mut header)?;
        if le32(&header, 0) != Some(0xCAFE_DADA) {
            return Err(JImageError::Format);
        }
        let table = le32(&header, 16).ok_or(JImageError::Format)? as usize;
        let locations_size = le32(&header, 20).ok_or(JImageError::Format)? as usize;
        let strings_size = le32(&header, 24).ok_or(JImageError::Format)? as usize;
        let index_size = 28 + table * 8 + locations_size + strings_size;
        if table > 1 << 24 || locations_size > 1 << 28 || strings_size > 1 << 28 {
            return Err(JImageError::Format);
        }
        let mut rest = vec![0u8; index_size - 28];
        file.read_exact(&mut rest)?;
        let offsets = &rest[table * 4..table * 8];
        let locations = &rest[table * 8..table * 8 + locations_size];
        let strings = &rest[table * 8 + locations_size..];
        let mut entries = Vec::new();
        let mut skipped = 0;
        for i in 0..table {
            let Some(loc_off) = le32(offsets, i * 4) else { break };
            let mut at = loc_off as usize;
            let mut attrs = [0u64; 8];
            while let Some(&b) = locations.get(at) {
                let kind = (b >> 3) as usize;
                if kind == 0 {
                    break;
                }
                let len = (b & 7) as usize + 1;
                let mut v = 0u64;
                for k in 0..len {
                    v = (v << 8) | u64::from(*locations.get(at + 1 + k).unwrap_or(&0));
                }
                if kind < 8 {
                    attrs[kind] = v;
                }
                at += 1 + len;
            }
            let module = cstr(strings, attrs[1] as usize);
            let parent = cstr(strings, attrs[2] as usize);
            let base = cstr(strings, attrs[3] as usize);
            let ext = cstr(strings, attrs[4] as usize);
            if module.is_empty() || ext != "class" {
                continue;
            }
            let name = if parent.is_empty() {
                base
            } else {
                format!("{parent}/{base}")
            };
            if attrs[6] != 0 {
                skipped += 1;
                continue;
            }
            entries.push(ImageEntry {
                module,
                name,
                offset: index_size as u64 + attrs[5],
                size: attrs[7],
                compressed: false,
            });
        }
        entries.sort_by_key(|e| e.offset);
        Ok(Self {
            file,
            entries,
            skipped_compressed: skipped,
        })
    }

    /// Bytes of one resource.
    pub fn read(&mut self, entry: &ImageEntry) -> Result<Vec<u8>, JImageError> {
        self.file.seek(SeekFrom::Start(entry.offset))?;
        let mut buf = vec![0u8; entry.size as usize];
        self.file.read_exact(&mut buf)?;
        Ok(buf)
    }
}
