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
//! Broken controls for `.bboxy/specs/harness/codex_reply.den`: each mutation
//! puts back one way the card read codex-acp replies before 2026-11-18.
use std::path::Path;
use std::process::Command;

fn spec() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.bboxy/specs/harness/codex_reply.den")
}
fn refuted(name: &str, from: &str, to: &str, id: &str) {
    let text = std::fs::read_to_string(spec()).unwrap();
    assert_eq!(text.matches(from).count(), 1, "{name}: mutation anchor moved: {from}");
    let dir = std::env::temp_dir().join(format!("bboxy450-den-codex-reply-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("codex_reply.den");
    std::fs::write(&path, text.replace(from, to)).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_bboxy450-den")).arg("check").arg(&path).output().unwrap();
    let said = String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    std::fs::remove_dir_all(&dir).unwrap();
    assert_eq!(out.status.code(), Some(1), "{name}: the broken spec must be unsound:\n{said}");
    assert!(said.contains("X601") && said.contains(id), "{name}: expected X601 {id}:\n{said}");
}

#[test]
fn the_spec_is_sound() {
    let out = Command::new(env!("CARGO_BIN_EXE_bboxy450-den")).arg("check").arg(spec()).output().unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stdout));
}

// Before: `{result, error: null}` was taken as the run record: no status, the
// "pending" chip, and the whole envelope rendered as the run's value.
#[test]
fn reading_the_envelope_as_the_record_shows_pending_and_the_raw_envelope() {
    refuted(
        "envelope-as-record",
        "{ shape = s; run = r; card = Card.Done; body = Body.Result }",
        "{ shape = s; run = r; card = Card.Pending; body = Body.RawEnvelope }",
        "CODEXREPLY_001",
    );
}

// Before: a failed MCP call showed its envelope instead of its message.
#[test]
fn an_error_envelope_shown_raw_hides_the_message() {
    refuted(
        "error-raw",
        "{ shape = s; card = Card.FailedCard; body = Body.Message }",
        "{ shape = s; card = Card.FailedCard; body = Body.RawEnvelope }",
        "CODEXREPLY_003",
    );
}

// Before: a run that returned a handle kept spinning after its turn ended.
#[test]
fn an_unpolled_handle_spins_forever_without_the_turn_end_rule() {
    refuted(
        "spins-forever",
        "case stop if !turn_ended && card == Card.Spinning => { turn_ended = true; card = Card.NotPolled }",
        "case stop if !turn_ended && card == Card.Spinning => { turn_ended = true }",
        "CODEXREPLY_004",
    );
}
