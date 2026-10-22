//! Shared blocking ring/blob storage primitives, independent of the logic store.

use std::io;

pub mod blobs;

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

pub trait Blobs {
    fn put(&self, bytes: &[u8]) -> Result<String>;
    fn get(&self, digest: &str) -> Result<Vec<u8>>;
    fn list(&self) -> Result<Vec<String>>;
}
