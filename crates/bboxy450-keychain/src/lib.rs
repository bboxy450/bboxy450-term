#![doc = include_str!("../README.md")]
#![warn(rustdoc::broken_intra_doc_links)]
#![deny(missing_docs)]

use std::path::PathBuf;

pub use bboxy450_keychain_file::{Acl, Error, MAX_SECRET_BYTES, Secret};

/// A private directory selected by the caller, independent of the native store.
#[derive(Debug, Clone)]
pub enum Backend {
    /// Immutable secret slots in an explicitly selected private directory.
    File {
        /// Absolute caller-owned directory, validated by the backend.
        directory: PathBuf,
    },
}

/// Custody operations run on the caller's IO owner, never a UI callback.
pub trait Store: Send + Sync {
    /// Read one secret, or None when absent.
    ///
    /// # Errors
    /// Returns label, path, permission, size or IO failures from the selected backend.
    fn get(&self, label: &str) -> Result<Option<Secret>, Error>;
    /// Atomically create a nonempty immutable slot with the requested ACL.
    ///
    /// # Errors
    /// Refuses invalid input, unsafe storage, an existing slot or IO failure.
    fn put(&self, label: &str, secret: &[u8], acl: Acl) -> Result<(), Error>;
    /// Remove a slot explicitly; absence succeeds.
    ///
    /// # Errors
    /// Refuses invalid labels, unsafe storage or IO failure.
    fn remove(&self, label: &str) -> Result<(), Error>;
    /// Read access metadata without loading secret bytes.
    ///
    /// # Errors
    /// Refuses invalid labels, unsafe storage or IO failure.
    fn acl(&self, label: &str) -> Result<Option<Acl>, Error>;
}

/// Open only the selected backend; errors never select another identity.
///
/// # Errors
/// Returns the selected backend's unsafe-path, permission or IO failure.
pub fn open(backend: Backend) -> Result<Box<dyn Store>, Error> {
    match backend {
        Backend::File { directory } => {
            Ok(Box::new(bboxy450_keychain_file::Store::open(&directory)?))
        }
    }
}
impl Store for bboxy450_keychain_file::Store {
    fn get(&self, label: &str) -> Result<Option<Secret>, Error> {
        self.get(label)
    }
    fn put(&self, label: &str, secret: &[u8], acl: Acl) -> Result<(), Error> {
        self.put(label, secret, acl)
    }
    fn remove(&self, label: &str) -> Result<(), Error> {
        self.remove(label)
    }
    fn acl(&self, label: &str) -> Result<Option<Acl>, Error> {
        self.acl(label)
    }
}
