// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
// Origin: P2 task #5084, ported from bboxy450-log-converter into bboxy450-term.
// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-log-converter — https://github.com/bboxy450/bboxy450-log-converter
// License: https://github.com/bboxy450/bboxy450-log-converter/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-log-converter/blob/main/ATTRIBUTION.md

#![allow(
    clippy::disallowed_methods,
    reason = "unattended filesystem and process fixtures, never a UI turn"
)]
mod common;
use bboxy450_log_converter::bxl::*;
use common::*;
#[test]
fn same_ir_clock_and_flush_schedule_produce_identical_segments_and_indices() {
    let left = tempfile::tempdir().unwrap();
    let right = tempfile::tempdir().unwrap();
    for root in [left.path(), right.path()] {
        let (options, _clock) = options();
        let mut writer = Writer::with_options(root, &session(), options).unwrap();
        for seq in 0..8 {
            writer.append(&[record(seq)], None).unwrap();
            if seq % 2 == 1 {
                writer.sync().unwrap();
            }
            if seq == 3 {
                writer.seal().unwrap();
            }
        }
        writer.close().unwrap();
    }
    let mut names = std::fs::read_dir(left.path())
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect::<Vec<_>>();
    names.sort();
    for name in names {
        assert_eq!(
            std::fs::read(left.path().join(&name)).unwrap(),
            std::fs::read(right.path().join(&name)).unwrap(),
            "{name:?}"
        );
    }
}

#[path="support/claude_determinism.rs"]
mod claude;
