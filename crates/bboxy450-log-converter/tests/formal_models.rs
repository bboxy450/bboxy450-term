// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Native append-log contracts; the workspace formal runner exports Java parity.
#![allow(
    clippy::disallowed_methods,
    reason = "unattended filesystem and process fixtures, never a UI turn"
)]
#[path = "formal/append_log.rs"]
mod append_log;
#[path = "formal/export.rs"]
mod export;
#[test]
fn append_log_contract_and_three_controls() {
    let private = tempfile::tempdir().unwrap();
    let output = std::env::var_os("BBOXY_FORMAL_OUTPUT");
    let root = output
        .as_deref()
        .map_or(private.path(), std::path::Path::new);
    let mut manifest = if output.is_some() {
        std::fs::read_to_string(root.join("manifest.tsv")).unwrap()
    } else {
        String::new()
    };
    for (name, broken, expected) in [
        ("LcAppendLog", 0, "pass"),
        ("BrokenLcAppendLogTornRead", 1, "safety"),
        ("BrokenLcAppendLogEarlyIndex", 2, "safety"),
        ("BrokenLcAppendLogMixedTable", 3, "safety"),
    ] {
        export::export(
            append_log::model(name, broken),
            expected,
            root,
            &mut manifest,
        )
        .unwrap();
    }
    if output.is_some() {
        std::fs::write(root.join("manifest.tsv"), manifest).unwrap();
    }
}
