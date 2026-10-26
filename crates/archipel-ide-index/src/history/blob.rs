//! Content-addressed blob store: SHA-256 of the content names a file under
//! `blobs/<2 hex>/<62 hex>`; the content is stored deflate-compressed
//! (pure Rust `flate2`/miniz_oxide; zstd would add a C dependency).

use flate2::Compression;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

/// The SHA-256 of a blob's content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BlobId(pub [u8; 32]);

impl BlobId {
    pub fn of(content: &[u8]) -> Self {
        Self(Sha256::digest(content).into())
    }

    pub fn hex(&self) -> String {
        self.0.iter().map(|b| format!("{b:02x}")).collect()
    }

    pub fn from_slice(bytes: &[u8]) -> Option<Self> {
        Some(Self(bytes.try_into().ok()?))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum BlobError {
    #[error("blob {0} is missing")]
    Missing(String),
    #[error("blob {0} is damaged")]
    Damaged(String),
    #[error("blob store I/O: {0}")]
    Io(#[from] std::io::Error),
}

/// A directory of compressed blobs.
#[derive(Debug, Clone)]
pub struct BlobStore {
    root: PathBuf,
}

impl BlobStore {
    pub fn open(root: &Path) -> std::io::Result<Self> {
        std::fs::create_dir_all(root)?;
        Ok(Self {
            root: root.to_path_buf(),
        })
    }

    fn path(&self, id: &BlobId) -> PathBuf {
        let hex = id.hex();
        self.root.join(&hex[..2]).join(&hex[2..])
    }

    pub fn contains(&self, id: &BlobId) -> bool {
        self.path(id).is_file()
    }

    /// Stores `content` (a no-op when it is already stored) and returns its id.
    pub fn put(&self, content: &[u8]) -> Result<BlobId, BlobError> {
        let id = BlobId::of(content);
        let path = self.path(&id);
        if path.is_file() {
            return Ok(id);
        }
        std::fs::create_dir_all(path.parent().unwrap_or(&self.root))?;
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::fast());
        encoder.write_all(content)?;
        let packed = encoder.finish()?;
        // Write beside the target and rename: a crash never leaves a torn blob.
        let temp = path.with_extension("tmp");
        std::fs::write(&temp, packed)?;
        std::fs::rename(&temp, &path)?;
        Ok(id)
    }

    pub fn get(&self, id: &BlobId) -> Result<Vec<u8>, BlobError> {
        let packed = match std::fs::read(self.path(id)) {
            Ok(packed) => packed,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Err(BlobError::Missing(id.hex()));
            }
            Err(error) => return Err(error.into()),
        };
        let mut content = Vec::new();
        ZlibDecoder::new(packed.as_slice())
            .read_to_end(&mut content)
            .map_err(|_| BlobError::Damaged(id.hex()))?;
        if BlobId::of(&content) != *id {
            return Err(BlobError::Damaged(id.hex()));
        }
        Ok(content)
    }

    /// Size on disk of one blob.
    pub fn size(&self, id: &BlobId) -> u64 {
        std::fs::metadata(self.path(id)).map_or(0, |m| m.len())
    }

    pub fn remove(&self, id: &BlobId) {
        let _ = std::fs::remove_file(self.path(id));
    }

    /// Every stored blob with its size.
    pub fn list(&self) -> Vec<(BlobId, u64)> {
        let mut out = Vec::new();
        let Ok(prefixes) = std::fs::read_dir(&self.root) else {
            return out;
        };
        for prefix in prefixes.flatten() {
            let Some(head) = prefix.file_name().to_str().map(str::to_owned) else {
                continue;
            };
            let Ok(files) = std::fs::read_dir(prefix.path()) else {
                continue;
            };
            for file in files.flatten() {
                let Some(tail) = file.file_name().to_str().map(str::to_owned) else {
                    continue;
                };
                let hex = format!("{head}{tail}");
                let bytes: Option<Vec<u8>> = (0..hex.len() / 2)
                    .map(|i| u8::from_str_radix(hex.get(i * 2..i * 2 + 2)?, 16).ok())
                    .collect();
                if let Some(id) = bytes.and_then(|b| BlobId::from_slice(&b)) {
                    out.push((id, file.metadata().map_or(0, |m| m.len())));
                }
            }
        }
        out
    }
}
