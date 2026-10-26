#![doc = include_str!("../README.md")]
#![warn(rustdoc::broken_intra_doc_links)]
#![deny(missing_docs)]

/// Why this known platform currently has no implementation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Unsupported {
    /// The platform whose native backend is unavailable.
    pub os: &'static str,
    /// The research marker describing the missing capability.
    pub note: &'static str,
}

/// Refuse by name; no PowerShell, shell command or desktop fallback is used.
///
/// # Errors
/// Always returns the named Windows unsupported-backend marker.
pub fn open() -> Result<(), Unsupported> {
    Err(Unsupported {
        os: "windows",
        note: "DO RESEARCH BEFORE IMPLEMENTING: .bboxy/research/desktop-notifications-research.md",
    })
}

#[cfg(test)]
mod tests {
    // keeps: spec desktop-notifications §4 — named Unsupported OS and exact public research marker
    #[test]
    fn research_marker_is_the_runtime_result() {
        let error = super::open().unwrap_err();
        assert_eq!(error.os, "windows");
        assert_eq!(
            error.note,
            "DO RESEARCH BEFORE IMPLEMENTING: .bboxy/research/desktop-notifications-research.md"
        );
    }
}
