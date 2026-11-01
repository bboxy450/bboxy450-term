// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Pure P0 controls; no real client is launched from workspace unit tests.
#![allow(
    clippy::disallowed_methods,
    reason = "unattended filesystem and process fixtures, never a UI turn"
)]
#[test]
fn probe_witnesses_and_isolation_controls() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let status = std::process::Command::new("python3")
        .arg("-B")
        .arg(root.join("probes/check.py"))
        .stdin(std::process::Stdio::null())
        .status()
        .expect("python3");
    assert!(status.success(), "probe controls failed: {status}");
}
