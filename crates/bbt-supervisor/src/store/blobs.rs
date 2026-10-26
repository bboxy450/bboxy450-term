use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use super::{Blobs, Result, digest, invalid};

/// Largest decoded blob accepted by this backend, in bytes.
pub const MAX_BLOB: usize = 64 * 1024 * 1024;

/// Local compressed blobs named by SHA-256 digest under the selected object directory.
#[derive(Debug, Clone)]
pub struct LocalBlobs {
    root: PathBuf,
}

impl LocalBlobs {
    /// Create or reopen the selected object directory's `blobs` child.
    ///
    /// # Errors
    /// Returns directory creation or filesystem IO failures.
    ///
    /// # Panics
    /// In debug builds, panics when called on the registered UI thread.
    pub fn open(object: &Path) -> Result<Self> {
        bbt_rt::assert_not_ui_thread();
        let root = object.join("blobs");
        std::fs::create_dir_all(&root)?;
        Ok(Self { root })
    }
    fn path(&self, digest: &str) -> Result<PathBuf> {
        if digest.len() != 64
            || !digest
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        {
            return Err(invalid("invalid blob digest"));
        }
        Ok(self.root.join(format!("{digest}.zst")))
    }
    pub(crate) fn put_ordered(&self, bytes: &[u8]) -> Result<String> {
        self.put_tier(bytes, false)
    }
    /// The shared capture writer supplies the subsequent device-wide full
    /// flush. File and directory ordering must be established before returning.
    fn put_tier(&self, bytes: &[u8], strong: bool) -> Result<String> {
        bbt_rt::assert_not_ui_thread();
        if bytes.len() > MAX_BLOB {
            return Err(invalid("blob exceeds 64 MiB"));
        }
        let hash = digest(bytes);
        let path = self.path(&hash)?;
        if path.exists() {
            self.get(&hash)?;
            return Ok(hash);
        }
        let temp = self.root.join(format!(".{}.tmp", ulid::Ulid::new()));
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)?;
        let compressed = zstd::stream::encode_all(bytes, 3)?;
        file.write_all(&compressed)?;
        crate::durability::sync_file(&file, strong)?;
        std::fs::rename(temp, path)?;
        crate::durability::sync_directory(&std::fs::File::open(&self.root)?)?;
        if strong {
            crate::durability::sync_file(&file, true)?;
        }
        Ok(hash)
    }
}

impl Blobs for LocalBlobs {
    fn put(&self, bytes: &[u8]) -> Result<String> {
        self.put_tier(bytes, true)
    }
    fn get(&self, hash: &str) -> Result<Vec<u8>> {
        bbt_rt::assert_not_ui_thread();
        let file = std::fs::File::open(self.path(hash)?)?;
        let mut decoder = zstd::stream::read::Decoder::new(file)?.take((MAX_BLOB + 1) as u64);
        let mut bytes = Vec::new();
        decoder.read_to_end(&mut bytes)?;
        if bytes.len() > MAX_BLOB || digest(&bytes) != hash {
            return Err(invalid("corrupt or oversized blob"));
        }
        Ok(bytes)
    }
    fn list(&self) -> Result<Vec<String>> {
        bbt_rt::assert_not_ui_thread();
        let mut result = Vec::new();
        for entry in std::fs::read_dir(&self.root)? {
            let entry = entry?;
            if let Some(name) = entry
                .file_name()
                .to_str()
                .and_then(|n| n.strip_suffix(".zst"))
            {
                self.path(name)?;
                result.push(name.to_string());
            }
        }
        result.sort();
        Ok(result)
    }
}
