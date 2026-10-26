//! DO RESEARCH BEFORE IMPLEMENTING: .bboxy/research/desktop-notifications-research.md
//!
//! This backend deliberately performs no native operation. See the README
//! for the activation, replacement and removal contracts a real backend owes.

/// Why this known platform currently has no implementation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Unsupported {
    pub os: &'static str,
    pub note: &'static str,
}

/// Refuse by name; no PowerShell, shell command or desktop fallback is used.
pub fn open() -> Result<(), Unsupported> {
    Err(Unsupported {
        os: "linux",
        note: "DO RESEARCH BEFORE IMPLEMENTING: .bboxy/research/desktop-notifications-research.md",
    })
}

#[cfg(test)]
mod tests {
    // keeps: spec desktop-notifications §4 — named Unsupported OS and exact public research marker
    #[test]
    fn research_marker_is_the_runtime_result() {
        let error = super::open().unwrap_err();
        assert_eq!(error.os, "linux");
        assert_eq!(
            error.note,
            "DO RESEARCH BEFORE IMPLEMENTING: .bboxy/research/desktop-notifications-research.md"
        );
    }
}
