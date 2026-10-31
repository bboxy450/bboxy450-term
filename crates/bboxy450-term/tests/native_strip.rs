// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Standing night boundary: owns native windows; never captures the desktop.
#![allow(
    clippy::disallowed_methods,
    reason = "unattended private process fixture, never the UI thread"
)]

#[test]
#[ignore = "real native app, Codex local provider and private GPU observations belong to the night"]
fn native_window_codex_recording_and_replay_reject_blank_strips() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let result = std::process::Command::new("python3")
        .args(["-I", "-B"])
        .arg(root.join("scripts/e2e/native_strip.py"))
        .arg("--binary")
        .arg(env!("CARGO_BIN_EXE_bboxy450-term"))
        .arg("--night")
        .stdin(std::process::Stdio::null())
        .output()
        .expect("run the private native strip detector");
    println!("{}", String::from_utf8_lossy(&result.stdout));
    assert!(
        result.status.success(),
        "native strip detector exit {:?}: {}",
        result.status.code(),
        String::from_utf8_lossy(&result.stderr)
    );
}
