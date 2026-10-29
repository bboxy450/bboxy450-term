// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use bbt_proto::{GuiAction, GuiSeat, NotificationEpisode};
use serde_json::json;
use ulid::Ulid;

#[test]
fn legacy_gui_hello_bytes_and_new_readonly_observer_are_distinct() {
    let old = json!({"kind":"GuiHello","body":{"id":Ulid(3),"claim":false}});
    let decoded: GuiAction = serde_json::from_value(old.clone()).unwrap();
    assert!(matches!(
        decoded,
        GuiAction::GuiHello {
            observer: false,
            ..
        }
    ));
    assert_eq!(serde_json::to_value(decoded).unwrap(), old);
    let observer = GuiAction::GuiHello {
        id: Ulid(3),
        claim: false,
        observer: true,
    };
    assert_eq!(
        serde_json::to_value(observer).unwrap()["body"]["observer"],
        true
    );
}

#[test]
fn old_seat_omits_the_additive_key_and_current_episode_round_trips() {
    let seat = GuiSeat {
        record: bbt_proto::registry::SeatRecord::new(Ulid(1), Ulid(2), "fixture", "claude"),
        notification_episode: None,
        attention: None,
        state: bbt_proto::SeatState {
            generation: 1,
            process: bbt_proto::ProcessStatus::Running { pid: 7 },
            activity: bbt_proto::Activity::Waiting(bbt_proto::WaitKind::Question),
            grade: bbt_proto::Grade::Shim,
            title: String::new(),
            detail: None,
            turn_open: true,
            observed_ms: 1,
            subagents: Vec::new(),
        },
    };
    let legacy = serde_json::to_value(&seat).unwrap();
    assert!(legacy.get("notification_episode").is_none());
    let mut decoded: GuiSeat = serde_json::from_value(legacy).unwrap();
    assert_eq!(decoded.notification_episode, None);
    let key = NotificationEpisode {
        epoch: Ulid(9),
        generation: 1,
        ordinal: 4,
    };
    decoded.notification_episode = Some(key);
    let roundtrip: GuiSeat =
        serde_json::from_slice(&serde_json::to_vec(&decoded).unwrap()).unwrap();
    assert_eq!(roundtrip.notification_episode, Some(key));
}
