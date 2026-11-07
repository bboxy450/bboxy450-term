// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
#![allow(
    clippy::disallowed_methods,
    reason = "private process CPU/redraw witness"
)]

#[cfg(target_os = "macos")]
fn probe(mode: &str, extra: &[&str]) {
    let control = extra.contains(&"--control") || extra.contains(&"--full-scene-control");
    let mut command = std::process::Command::new("/usr/bin/python3");
    command
        .args(["-I", "-B"])
        .arg(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/idle_cpu.py"))
        .arg(env!("CARGO_BIN_EXE_bboxy450-term"))
        .args([
            "--mode",
            mode,
            "--seconds",
            if control { "3" } else { "10" },
        ])
        .args(extra);
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "idle {mode} {extra:?}: {}\n{}\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    print!("{}", String::from_utf8_lossy(&output.stdout));
}

#[cfg(target_os = "macos")]
#[test]
fn idle_cpu_and_redraw_count_with_six_headless_seats() {
    probe("headless", &[]);
    probe("headless", &["--control"]);
}

#[cfg(target_os = "macos")]
#[test]
#[ignore = "private native window, no OS input; make test-gui-idle"]
fn idle_cpu_and_redraw_count_with_six_native_seats() {
    probe("native", &[]);
    probe("native", &["--control"]);
    probe("native", &["--waiting"]);
    probe("native", &["--waiting", "--full-scene-control"]);
    probe("native", &["--waiting", "--occluded"]);
}
