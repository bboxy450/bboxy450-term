//! Windows Credential Manager is a named, researched future backend.
//! Owner-only file permissions support the separate, portable file backend.

#[cfg(windows)]
pub mod private_files;

/// Native vault access has no file, insecure-storage or interactive fallback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Unsupported {
    pub os: &'static str,
    pub note: &'static str,
}

/// Future native vault entry point; never displays a dialog or reads a secret.
pub fn open() -> Result<(), Unsupported> {
    Err(Unsupported {
        os: "windows",
        note: "DO RESEARCH BEFORE IMPLEMENTING: see this crate's README.md",
    })
}
