#![doc = include_str!("../README.md")]
#![warn(rustdoc::broken_intra_doc_links)]
#![deny(missing_docs)]

#[cfg(windows)]
pub mod private_files;

/// Native vault access has no file, insecure-storage or interactive fallback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Unsupported {
    /// The platform whose native backend is unavailable.
    pub os: &'static str,
    /// The research marker describing the missing capability.
    pub note: &'static str,
}

/// Future native vault entry point; never displays a dialog or reads a secret.
///
/// # Errors
/// Always returns the named Windows unsupported-backend marker.
pub fn open() -> Result<(), Unsupported> {
    Err(Unsupported {
        os: "windows",
        note: "DO RESEARCH BEFORE IMPLEMENTING: see this crate's README.md",
    })
}
