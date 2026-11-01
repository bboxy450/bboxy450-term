// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md

use bboxy450_log_converter::{ir::*, parse::claude::Parser};
use std::io::Cursor;
pub fn parse(bytes: &[u8]) -> (Session, Vec<Event>) {
    let mut p = Parser::new(Cursor::new(bytes), SessionId(ulid::Ulid::from(1u128))).unwrap();
    let mut events = vec![];
    while let Some(batch) = p.next_record().unwrap() {
        events.extend(batch)
    }
    while let Some(batch) = p.finish_tail().unwrap() {
        events.extend(batch)
    }
    (p.session().clone(), events)
}
pub fn fixture() -> Vec<u8> {
    include_bytes!("../fixtures/synthetic/claude.jsonl").to_vec()
}
