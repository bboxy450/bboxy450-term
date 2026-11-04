// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Public probe entry point, private directories and no backend or GUI startup.
#![allow(clippy::disallowed_methods, reason = "bounded owned child fixture")]
use std::{
    process::{Command, Stdio},
    time::{Duration, Instant},
};

#[test]
fn ui_status_public_probe_refuses_cold_start_without_creating_owners() {
    let root = tempfile::tempdir().unwrap();
    let account = root.path().join("account");
    let run = account.join("run");
    let repository = root.path().join("repository");
    std::fs::create_dir_all(&repository).unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_bboxy450-term"))
        .args([
            "--gui-handoff-probe",
            "--preserve-backend-check",
            "--test-account",
        ])
        .arg(&account)
        .arg("--run-dir")
        .arg(&run)
        .arg("--repository")
        .arg(&repository)
        .env("BBOXY_RUN_DIR", root.path().join("wrong-inherited-run"))
        .env("PRIVATE_FIXTURE_SECRET", "do-not-disclose-token")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let started = Instant::now();
    while child.try_wait().unwrap().is_none() {
        if started.elapsed() > Duration::from_secs(10) {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("owned public probe exceeded 10 seconds");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let output = child.wait_with_output().unwrap();
    assert!(!output.status.success());
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(text.contains("cold startup refused"), "{text}");
    assert!(!text.contains("do-not-disclose-token"));
    assert!(!run.exists(), "public probe created a cold run directory");
    assert!(!root.path().join("wrong-inherited-run").exists());
}
