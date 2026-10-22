//! Per-object durable storage. Call these blocking APIs on a logic/IO worker.

use std::io;
use std::path::{Path, PathBuf};

pub mod blobs;
pub mod events;
pub mod sqlite;

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Result<T> = std::result::Result<T, Error>;

pub fn invalid(message: impl Into<String>) -> Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into()).into()
}

pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

pub fn digest(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

pub fn object_dir(root: &Path, name: &str) -> Result<PathBuf> {
    bbt_rt::assert_not_ui_thread();
    if name.is_empty()
        || name.len() > 128
        || !name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_".contains(&b))
    {
        return Err(invalid("invalid object name"));
    }
    let path = root.join(name);
    std::fs::create_dir_all(&path)?;
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700))?;
    Ok(path)
}

pub trait Store {
    fn get(&self, key: &str) -> Result<Option<Vec<u8>>>;
    fn put(&self, key: &str, value: &[u8]) -> Result<()>;
    fn list(&self, prefix: &str) -> Result<Vec<(String, Vec<u8>)>>;
    fn sql(
        &self,
        query: &str,
        args: &[rusqlite::types::Value],
    ) -> Result<Vec<Vec<rusqlite::types::Value>>>;
    fn alarm(&self, key: &str, deadline_ms: u64) -> Result<()>;
    fn due_alarms(&self, now_ms: u64) -> Result<Vec<String>>;
}

pub trait Blobs {
    fn put(&self, bytes: &[u8]) -> Result<String>;
    fn get(&self, digest: &str) -> Result<Vec<u8>>;
    fn list(&self) -> Result<Vec<String>>;
}

pub trait Events {
    fn append(
        &self,
        event: events::Draft,
        key: &ed25519_dalek::SigningKey,
    ) -> Result<events::Envelope>;
    fn after(&self, sequence: u64, limit: usize) -> Result<Vec<events::Envelope>>;
    fn verify_chain(&self) -> Result<u64>;
}

pub(crate) fn sqlint(value: u64) -> Result<i64> {
    Ok(value.try_into()?)
}
