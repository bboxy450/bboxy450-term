// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
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
/// Always returns the named Linux unsupported-backend marker.
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
