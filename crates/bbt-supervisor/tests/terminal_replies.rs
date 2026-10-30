// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use bbt_supervisor::{
    Event, SeatSize, Ulid,
    ring::{Playback, Saved},
};

#[test]
fn live_cursor_queries_reply_once_at_their_parsed_positions_while_replay_stays_quiet() {
    let seat = Ulid::new();
    let event = Event::Output {
        seat,
        generation: 1,
        seq: 1,
        bytes: b"\x1b[3;7H\x1b[6n\x1b[5;9H\x1b[6n".to_vec(),
    };
    let mut live = Playback::new(
        seat,
        1,
        SeatSize {
            cols: 80,
            rows: 24,
            ..Default::default()
        },
    );
    let saved = live.saved().encode().unwrap();
    let mut replies = Vec::new();
    live.apply_with_effects(&event, |effect| {
        if let bbt_core::Event::PtyWrite(reply) = effect {
            replies.push(reply);
        }
    })
    .unwrap();
    assert_eq!(replies, ["\x1b[3;7R", "\x1b[5;9R"]);
    assert!(
        live.apply_with_effects(&event, |_| panic!("duplicate capture emitted an effect"))
            .is_err()
    );
    let stale = Event::Output {
        seat,
        generation: 2,
        seq: 2,
        bytes: b"\x1b[6n".to_vec(),
    };
    assert!(
        live.apply_with_effects(&stale, |_| panic!("wrong generation emitted an effect"))
            .is_err()
    );
    let mut replay = Playback::restore(Saved::decode(&saved).unwrap());
    replay.apply(&event).unwrap();
    assert!(replay.term.take_events().is_empty());
    let mut restored = Playback::restore(live.saved());
    assert!(restored.term.take_events().is_empty());
    assert_eq!(restored.sequence, 1);
}
