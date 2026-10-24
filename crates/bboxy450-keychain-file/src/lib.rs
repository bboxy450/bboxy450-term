//! Explicit file custody. The file is the secret; there is no wrapping key.
//!
//! Each immutable slot is published atomically in a caller-owned private
//! directory. Opening this store never selects a native credential service.
use std::{fmt, path::Path};

use zeroize::Zeroize;

#[cfg(unix)]
mod unix;
#[cfg(unix)]
use unix as platform;
#[cfg(windows)]
mod windows;
#[cfg(windows)]
use windows as platform;

/// Largest supported secret. Reads refuse larger files before allocating.
pub const MAX_SECRET_BYTES: usize = 64 * 1024;

/// Safe diagnostics never contain a secret or a credential-service response.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    InvalidLabel,
    UnsafePath,
    Permissions,
    Conflict,
    EmptySecret,
    TooLarge,
    Io(std::io::ErrorKind),
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLabel => f.write_str("file custody: invalid label"),
            Self::UnsafePath => f.write_str("file custody: unsafe path or non-regular file"),
            Self::Permissions => f.write_str("file custody: owner-only permissions required"),
            Self::Conflict => f.write_str("file custody: label already exists"),
            Self::EmptySecret => f.write_str("file custody: empty secret"),
            Self::TooLarge => f.write_str("file custody: secret exceeds 64 KiB"),
            Self::Io(kind) => write!(f, "file custody: IO {kind:?}"),
        }
    }
}
impl std::error::Error for Error {}
impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error.kind())
    }
}

/// An owned secret whose buffer is erased on drop and hidden from Debug.
pub struct Secret(Vec<u8>);
impl Secret {
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}
impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Secret(<redacted>)")
    }
}
impl Drop for Secret {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

/// Portable statement of the store's owner-only access policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Acl {
    OwnerOnly,
}

/// Independent directory handle. There is no process-wide lock or queue.
pub struct Store(platform::Store);
impl Store {
    /// Create or reopen an absolute private directory; never repair broad modes.
    pub fn open(directory: &Path) -> Result<Self, Error> {
        Ok(Self(platform::Store::open(directory)?))
    }
    /// Read an immutable slot, refusing symlinks, broad modes and oversized data.
    pub fn get(&self, label: &str) -> Result<Option<Secret>, Error> {
        label_ok(label)?;
        self.0.get(label).map(|value| value.map(Secret))
    }
    /// Atomically create a slot. An existing label is a conflict, never replaced.
    pub fn put(&self, label: &str, secret: &[u8], acl: Acl) -> Result<(), Error> {
        label_ok(label)?;
        if secret.is_empty() {
            return Err(Error::EmptySecret);
        }
        if secret.len() > MAX_SECRET_BYTES {
            return Err(Error::TooLarge);
        }
        let Acl::OwnerOnly = acl;
        self.0.put(label, secret)
    }
    /// Explicitly remove a slot; absence is harmless and unsafe files are refused.
    pub fn remove(&self, label: &str) -> Result<(), Error> {
        label_ok(label)?;
        self.0.remove(label)
    }
    /// Inspect access metadata without reading the secret bytes.
    pub fn acl(&self, label: &str) -> Result<Option<Acl>, Error> {
        label_ok(label)?;
        self.0.acl(label)
    }
}
fn label_ok(label: &str) -> Result<(), Error> {
    if label.is_empty()
        || label.len() > 128
        || !label
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"_-".contains(&byte))
    {
        return Err(Error::InvalidLabel);
    }
    Ok(())
}
