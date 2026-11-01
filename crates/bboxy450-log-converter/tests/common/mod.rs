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

#![allow(dead_code)]
use bboxy450_log_converter::{bxl::*, ir::*};
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
pub fn session() -> Session {
    Session {
        id: SessionId(ulid::Ulid::from(1u128)),
        source: Source {
            provider: Provider::Claude,
            native_id: "synthetic".into(),
            cli_version: None,
            parser_revision: 1,
        },
        lineage: vec![],
        cwd: None,
        git: None,
        created_at: Some("2026-01-01T00:00:00.000Z".into()),
    }
}
pub fn event(seq: u64, text: &str) -> Event {
    Event {
        seq,
        id: EventId::tagged(Provider::Claude, "synthetic", &seq.to_string(), "block-0"),
        record: RecordId::native(Provider::Claude, "synthetic", &seq.to_string()),
        parent: seq
            .checked_sub(1)
            .map(|n| RecordId::native(Provider::Claude, "synthetic", &n.to_string())),
        turn: Some(TurnId("turn-0".into())),
        at: None,
        fidelity: Fidelity::Full,
        raw: Some(Raw {
            provider: Provider::Claude,
            bytes: format!("{{\"type\":\"synthetic\",\"seq\":{seq},\"text\":\"{text}\"}}")
                .into_bytes(),
        }),
        kind: EventKind::Message {
            role: Role::Assistant,
            blocks: vec![Block::Text { text: text.into() }],
        },
    }
}
pub fn record(seq: u64) -> Record {
    Record::event(&event(seq, "synthetic payload")).unwrap()
}
pub fn seqs(root: &std::path::Path) -> Vec<u64> {
    let mut reader = Reader::open(root).unwrap();
    let mut out = vec![];
    while let Some(event) = reader.next_event().unwrap() {
        out.push(event.value().seq);
    }
    out
}
#[derive(Default)]
pub struct ManualClock(pub AtomicU64);
impl Clock for ManualClock {
    fn now_ms(&self) -> u64 {
        self.0.load(Ordering::SeqCst)
    }
}
pub fn options() -> (Options, Arc<ManualClock>) {
    let clock = Arc::new(ManualClock::default());
    (
        Options {
            clock: clock.clone(),
            ..Options::default()
        },
        clock,
    )
}
