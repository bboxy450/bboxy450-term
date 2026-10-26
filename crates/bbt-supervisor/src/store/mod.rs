//! Shared blocking ring/blob storage primitives, independent of the logic store.

use std::io;

/// Content-addressed compressed blob storage used by retained capture.
pub mod blobs;

/// Sendable, shareable storage error preserving the original failure.
pub type Error = Box<dyn std::error::Error + Send + Sync>;
/// Result type for blocking retained-capture storage operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Construct an InvalidData storage error with the supplied diagnostic.
pub fn invalid(message: impl Into<String>) -> Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into()).into()
}

/// Read Unix-epoch milliseconds, saturating unsupported clock ranges.
pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

/// Return the lowercase hexadecimal SHA-256 content digest.
pub fn digest(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Blocking content-addressed blob operations, invoked on an IO owner.
pub trait Blobs {
    /// Store bytes and return their content digest.
    ///
    /// # Errors
    /// Returns implementation-specific size, persistence or IO failures.
    fn put(&self, bytes: &[u8]) -> Result<String>;
    /// Load and validate bytes identified by the supplied digest.
    ///
    /// # Errors
    /// Returns missing, malformed, corrupt, oversized or IO failures.
    fn get(&self, digest: &str) -> Result<Vec<u8>>;
    /// List retained content digests without decoding their payloads.
    ///
    /// # Errors
    /// Returns directory, malformed-name or IO failures.
    fn list(&self) -> Result<Vec<String>>;
}
