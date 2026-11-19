// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
#![expect(
    clippy::disallowed_methods,
    reason = "Standalone checker controls perform synchronous filesystem and process I/O; no UI thread exists here."
)]
//! Broken controls for `.bboxy/specs/harness/acp_turn.den`. Each mutation puts
//! back one behaviour the harness had before (2026-11-18) and must be refuted:
//! the spec checks the fix, not only itself.
use std::path::Path;
use std::process::Command;

fn refuted(name: &str, from: &str, to: &str, code: &str, id: &str) {
    let spec = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.bboxy/specs/harness/acp_turn.den");
    let text = std::fs::read_to_string(&spec).unwrap();
    assert_eq!(text.matches(from).count(), 1, "{name}: mutation anchor moved: {from}");
    let dir = std::env::temp_dir().join(format!("bboxy450-den-acp-turn-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("acp_turn.den");
    std::fs::write(&path, text.replace(from, to)).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_bboxy450-den"))
        .arg("check")
        .arg(&path)
        .output()
        .unwrap();
    let said = String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    std::fs::remove_dir_all(&dir).unwrap();
    assert_eq!(out.status.code(), Some(1), "{name}: the broken spec must be unsound:\n{said}");
    assert!(said.contains(code) && said.contains(id), "{name}: expected {code} {id}:\n{said}");
}

#[test]
fn the_spec_is_sound() {
    let spec = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.bboxy/specs/harness/acp_turn.den");
    let out = Command::new(env!("CARGO_BIN_EXE_bboxy450-den")).arg("check").arg(&spec).output().unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stdout));
}

// Before: no watchdog. A hung agent's turn never ended (22.9 h, 12 h in logs).
#[test]
fn without_the_watchdog_a_hung_turn_never_ends() {
    refuted(
        "no-watchdog",
        "fair weak case stall if turn == Turn.Running && silent && !asking =>",
        "case stall if false && turn == Turn.Running && silent && !asking =>",
        "X651",
        "ACPTURN_L1",
    );
}

// A watchdog that ignored a question would cancel the operator's own wait.
#[test]
fn a_watchdog_that_ignores_questions_interrupts_the_operator() {
    refuted(
        "ignores-asking",
        "fair weak case stall if turn == Turn.Running && silent && !asking =>",
        "fair weak case stall if turn == Turn.Running && silent =>",
        "X601",
        "ACPTURN_001",
    );
}

// Before: a late update for a closed call opened a second, never-closed card.
#[test]
fn a_late_update_that_reopens_opens_a_second_card() {
    refuted(
        "late-reopens",
        "case record if tool == Tool.Closed => ignore KeptAsRecord",
        "case record if tool == Tool.Closed && opens < 2 => { tool = Tool.Open; opens = opens + 1 }",
        "X601",
        "ACPTURN_002",
    );
}

// Before: input sent before steering was negotiated stayed queued for the whole turn.
#[test]
fn input_parked_before_negotiation_stays_parked_without_the_reroute() {
    refuted(
        "no-reroute",
        "case negotiate if !steering && turn == Turn.Running && input == Input.Parked => { steering = true; input = Input.Delivered }",
        "case negotiate if !steering && turn == Turn.Running && input == Input.Parked => { steering = true }",
        "X601",
        "ACPTURN_003",
    );
}
