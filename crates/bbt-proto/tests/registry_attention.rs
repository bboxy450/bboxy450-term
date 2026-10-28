// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use bbt_proto::{
    messaging::{MessageClass, WakeMode},
    registry::{AttentionPolicy, AttentionRule, SeatRecord},
};
use serde_json::json;
use ulid::Ulid;

// keeps: spec messaging-attention §3 (legacy seats retain role defaults and bytes)
#[test]
fn legacy_seat_records_omit_attention_and_round_trip_without_new_bytes() {
    let line = include_str!("golden/gui_wire.tsv")
        .lines()
        .find(|line| line.starts_with("reply\t{\"kind\":\"Seat\","))
        .unwrap();
    let (_, row) = line.split_once('\t').unwrap();
    let row: serde_json::Value = serde_json::from_str(row).unwrap();
    let record = row["body"]["record"].clone();
    let decoded: SeatRecord = serde_json::from_value(record.clone()).unwrap();
    assert_eq!(decoded.attention, None);
    assert_eq!(serde_json::to_value(decoded).unwrap(), record);
    let new = SeatRecord::new(Ulid::new(), Ulid::new(), "seat", "zsh");
    assert!(
        serde_json::to_value(new)
            .unwrap()
            .get("attention")
            .is_none()
    );
}

// keeps: spec messaging-attention MA-2 (typed per-destination policy on the seat)
#[test]
fn all_policy_modes_and_classes_survive_a_seat_record_round_trip() {
    let mut record = SeatRecord::new(Ulid::new(), Ulid::new(), "seat", "zsh");
    record.attention = Some(AttentionPolicy {
        rules: vec![
            AttentionRule {
                destination: "channels/briefs".into(),
                class: MessageClass::Conversation,
                mode: WakeMode::Wake,
            },
            AttentionRule {
                destination: "channels/status".into(),
                class: MessageClass::Event,
                mode: WakeMode::Inbox,
            },
            AttentionRule {
                destination: "channels/reviews".into(),
                class: MessageClass::Conversation,
                mode: WakeMode::Digest,
            },
        ],
    });
    let bytes = serde_json::to_vec(&record).unwrap();
    assert_eq!(
        serde_json::from_slice::<SeatRecord>(&bytes).unwrap(),
        record
    );
}

#[test]
fn policy_has_no_sender_fields_and_refuses_unknown_classes_or_modes() {
    for rule in [
        json!({"destination":"channels/status","class":"unknown","mode":"inbox"}),
        json!({"destination":"channels/status","class":"event","mode":"mute"}),
        json!({"destination":"channels/status","class":"event","mode":"inbox","seat":"other"}),
    ] {
        assert!(serde_json::from_value::<AttentionRule>(rule).is_err());
    }
    assert!(
        serde_json::from_value::<AttentionPolicy>(json!({"rules":[],"sender":"other"})).is_err()
    );
}
