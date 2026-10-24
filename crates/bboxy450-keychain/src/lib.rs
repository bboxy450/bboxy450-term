//! Explicit custody selection. File custody never falls back to an OS vault.
use std::path::PathBuf;

pub use bboxy450_keychain_file::{Acl, Error, MAX_SECRET_BYTES, Secret};

/// A private directory selected by the caller, independent of the native store.
#[derive(Debug, Clone)]
pub enum Backend {
    File { directory: PathBuf },
}

/// Custody operations run on the caller's IO owner, never a UI callback.
pub trait Store: Send + Sync {
    fn get(&self, label: &str) -> Result<Option<Secret>, Error>;
    fn put(&self, label: &str, secret: &[u8], acl: Acl) -> Result<(), Error>;
    fn remove(&self, label: &str) -> Result<(), Error>;
    fn acl(&self, label: &str) -> Result<Option<Acl>, Error>;
}

/// Open only the selected backend; errors never select another identity.
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
