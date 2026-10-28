// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use bbt_mux::{
    messaging::{
        attention::{self, Facts},
        wire::MessageKind,
    },
    registry::{AttentionPolicy, AttentionRule, Class, Registry, Role, SeatRecord},
    store::sqlite::SqliteStore,
};
use bbt_proto::messaging::{MessageClass, WakeMode};
use ulid::Ulid;

fn seat(name: &str, class: Class) -> SeatRecord {
    let mut seat = SeatRecord::new(Ulid::new(), Ulid::new(), "receiver", "zsh");
    seat.role = Role {
        name: name.into(),
        class,
    };
    seat
}
fn rule(destination: &str, class: MessageClass, mode: WakeMode) -> AttentionRule {
    AttentionRule {
        destination: destination.into(),
        class,
        mode,
    }
}
fn decide(seat: &SeatRecord, path: &str, kind: MessageKind, facts: Facts) -> WakeMode {
    attention::decide(seat, path, kind, facts).unwrap()
}

// keeps: spec messaging-attention §3 (exact role names first, then Oracle class)
#[test]
fn role_defaults_never_wake_an_event_and_unknown_names_use_executor_defaults() {
    for (name, class, conversation) in [
        ("executor", Class::Worker, WakeMode::Digest),
        ("shell", Class::Worker, WakeMode::Digest),
        ("unknown", Class::Worker, WakeMode::Digest),
        ("oracle", Class::Worker, WakeMode::Digest),
        ("Architect", Class::Worker, WakeMode::Digest),
        ("architect", Class::Worker, WakeMode::Inbox),
        ("creativity", Class::Worker, WakeMode::Inbox),
        ("architect", Class::Oracle, WakeMode::Inbox),
        ("creativity", Class::Oracle, WakeMode::Inbox),
        ("unknown", Class::Oracle, WakeMode::Wake),
    ] {
        let receiver = seat(name, class);
        assert_eq!(
            decide(
                &receiver,
                "channels/room",
                MessageKind::Text,
                Facts::default()
            ),
            conversation,
            "{name:?}/{class:?}"
        );
        let dm = format!("seats/{}/inbox", receiver.id);
        assert_eq!(
            decide(&receiver, &dm, MessageKind::Text, Facts::default()),
            WakeMode::Wake,
            "{name:?}/{class:?} direct conversation"
        );
        let named = Facts {
            addressed_by_name: true,
            ..Facts::default()
        };
        assert_eq!(
            decide(&receiver, "channels/room", MessageKind::Text, named),
            WakeMode::Wake,
            "{name:?}/{class:?} named channel conversation"
        );
        for kind in [
            MessageKind::Presence,
            MessageKind::SlotClaimed,
            MessageKind::Landed,
            MessageKind::GateResult,
        ] {
            assert_eq!(
                decide(&receiver, "channels/room", kind, Facts::default()),
                WakeMode::Inbox
            );
            assert_eq!(
                decide(&receiver, &dm, kind, Facts::default()),
                WakeMode::Inbox
            );
            assert_eq!(
                decide(&receiver, "channels/room", kind, named),
                WakeMode::Inbox
            );
        }
    }
}

#[test]
fn architect_and_creativity_wake_for_their_direct_or_named_conversation() {
    for name in ["architect", "creativity"] {
        let receiver = seat(name, Class::Worker);
        let dm = format!("seats/{}/inbox", receiver.id);
        assert_eq!(
            decide(&receiver, &dm, MessageKind::Text, Facts::default()),
            WakeMode::Wake
        );
        assert_eq!(
            decide(
                &receiver,
                "channels/room",
                MessageKind::Text,
                Facts::default()
            ),
            WakeMode::Inbox
        );
        let named = Facts {
            addressed_by_name: true,
            ..Facts::default()
        };
        assert_eq!(
            decide(&receiver, "channels/room", MessageKind::Text, named),
            WakeMode::Wake
        );
        assert_eq!(
            decide(&receiver, "channels/room", MessageKind::Presence, named),
            WakeMode::Inbox
        );
    }
}

// keeps: spec messaging-attention §3 (receiver overrides are per destination/class)
#[test]
fn each_mode_overrides_only_its_exact_destination_and_class() {
    for mode in [WakeMode::Wake, WakeMode::Inbox, WakeMode::Digest] {
        let mut receiver = seat("executor", Class::Worker);
        receiver.attention =
            Some(attention::updated(None, rule("#room", MessageClass::Event, mode)).unwrap());
        assert_eq!(
            decide(
                &receiver,
                "channels/room",
                MessageKind::Landed,
                Facts::default()
            ),
            mode
        );
        assert_eq!(
            decide(
                &receiver,
                "channels/other",
                MessageKind::Landed,
                Facts::default()
            ),
            WakeMode::Inbox
        );
        assert_eq!(
            decide(
                &receiver,
                "channels/room",
                MessageKind::Text,
                Facts::default()
            ),
            WakeMode::Digest
        );
    }
}

// keeps: spec messaging-attention §3 (always wake applies to conversation only)
#[test]
fn only_operator_delegation_and_own_thread_conversation_override_inbox_or_digest() {
    let exceptions = [
        Facts {
            operator: true,
            ..Facts::default()
        },
        Facts {
            delegated: true,
            ..Facts::default()
        },
        Facts {
            own_thread_reply: true,
            ..Facts::default()
        },
    ];
    for mode in [WakeMode::Inbox, WakeMode::Digest] {
        let mut receiver = seat("executor", Class::Worker);
        receiver.attention = Some(
            attention::updated(
                None,
                rule("channels/work", MessageClass::Conversation, mode),
            )
            .unwrap(),
        );
        for facts in exceptions {
            assert_eq!(
                decide(&receiver, "channels/work", MessageKind::Task, facts),
                WakeMode::Wake
            );
            assert_eq!(
                decide(&receiver, "channels/work", MessageKind::Presence, facts),
                WakeMode::Inbox
            );
        }
        // An Oracle-class sender is not the operator principal, and a name is not a brief.
        assert_eq!(
            decide(
                &receiver,
                "channels/work",
                MessageKind::Text,
                Facts::default()
            ),
            mode
        );
        assert_eq!(
            decide(
                &receiver,
                "channels/work",
                MessageKind::Text,
                Facts {
                    addressed_by_name: true,
                    ..Facts::default()
                }
            ),
            mode
        );
    }
}

#[test]
fn event_wake_requires_an_explicit_receiver_choice_even_for_operator_status() {
    let mut receiver = seat("oracle", Class::Oracle);
    let all = Facts {
        operator: true,
        delegated: true,
        own_thread_reply: true,
        addressed_by_name: true,
    };
    assert_eq!(
        decide(&receiver, "channels/status", MessageKind::Presence, all),
        WakeMode::Inbox
    );
    receiver.attention = Some(
        attention::updated(
            None,
            rule("channels/status", MessageClass::Event, WakeMode::Digest),
        )
        .unwrap(),
    );
    assert_eq!(
        decide(&receiver, "channels/status", MessageKind::Presence, all),
        WakeMode::Digest
    );
}

#[test]
fn a_full_policy_can_replace_a_rule_but_never_grow_past_its_bound() {
    let mut policy = AttentionPolicy::default();
    for i in 0..attention::MAX_RULES {
        policy = attention::updated(
            Some(&policy),
            rule(
                &format!("channels/{i}"),
                MessageClass::Event,
                WakeMode::Inbox,
            ),
        )
        .unwrap();
    }
    let full = policy.clone();
    assert!(
        attention::updated(
            Some(&policy),
            rule("channels/extra", MessageClass::Event, WakeMode::Wake)
        )
        .is_err()
    );
    assert_eq!(policy, full);
    let replaced = attention::updated(
        Some(&policy),
        rule("#0", MessageClass::Event, WakeMode::Wake),
    )
    .unwrap();
    assert_eq!(replaced.rules.len(), attention::MAX_RULES);
    assert_eq!(replaced.rules[0].mode, WakeMode::Wake);
}

#[test]
fn malformed_destinations_and_broken_stored_policies_are_refused() {
    for path in [
        "",
        "channels/*",
        "channels/a b",
        "../seats",
        "channels/a\nb",
    ] {
        assert!(
            attention::updated(None, rule(path, MessageClass::Event, WakeMode::Inbox)).is_err(),
            "{path:?}"
        );
    }
    for rules in [
        vec![rule("#alias", MessageClass::Event, WakeMode::Inbox)],
        vec![rule("channels/x", MessageClass::Event, WakeMode::Inbox); 2],
    ] {
        let mut receiver = seat("executor", Class::Worker);
        receiver.attention = Some(AttentionPolicy { rules });
        assert!(
            attention::decide(&receiver, "channels/x", MessageKind::Text, Facts::default())
                .is_err()
        );
    }
}

// keeps: spec messaging-attention MA-2 (one durable registry, no mute mutation)
#[test]
fn policy_survives_reopen_unrelated_record_updates_and_process_restart() {
    let temp = tempfile::tempdir().unwrap();
    let receiver = seat("executor", Class::Worker);
    let expected = {
        let registry = Registry::new(SqliteStore::open(temp.path()).unwrap());
        registry.insert(&receiver).unwrap();
        registry
            .set_attention(
                receiver.id,
                1,
                rule("#status", MessageClass::Event, WakeMode::Inbox),
            )
            .unwrap()
    };
    let registry = Registry::new(SqliteStore::open(temp.path()).unwrap());
    registry
        .update(receiver.id, |record| {
            record.name = "renamed".into();
            record.muted = true;
            record.generation = 2;
            Ok(())
        })
        .unwrap();
    assert_eq!(
        registry.get(receiver.id).unwrap().attention,
        Some(expected.clone())
    );
    assert!(
        registry
            .set_attention(
                receiver.id,
                1,
                rule("channels/status", MessageClass::Event, WakeMode::Wake)
            )
            .is_err()
    );
    assert_eq!(registry.get(receiver.id).unwrap().attention, Some(expected));
    registry
        .set_attention(
            receiver.id,
            2,
            rule("channels/status", MessageClass::Event, WakeMode::Wake),
        )
        .unwrap();
    assert!(registry.get(receiver.id).unwrap().muted);
}

// keeps: spec messaging-attention §3 MA-8 — operator reset is sparse and generation fenced.
#[test]
fn operator_attention_changes_one_cell_preserves_hidden_rules_and_refuses_a_stale_generation() {
    let root = tempfile::tempdir().unwrap();
    let registry = Registry::new(SqliteStore::open(root.path()).unwrap());
    let receiver = seat("executor", Class::Worker);
    registry.insert(&receiver).unwrap();
    registry
        .set_attention(
            receiver.id,
            receiver.generation,
            rule(
                "channels/hidden",
                MessageClass::Conversation,
                WakeMode::Inbox,
            ),
        )
        .unwrap();
    let mut edit = bbt_proto::gui::AttentionEdit {
        generation: receiver.generation,
        destination: "channels/status".into(),
        class: MessageClass::Event,
        mode: Some(WakeMode::Wake),
    };
    registry.change_attention(receiver.id, &edit).unwrap();
    assert_eq!(
        registry
            .get(receiver.id)
            .unwrap()
            .attention
            .as_ref()
            .unwrap()
            .rules
            .len(),
        2
    );
    let accepted = registry.get(receiver.id).unwrap();
    edit.generation += 1;
    assert!(
        registry
            .change_attention(receiver.id, &edit)
            .unwrap_err()
            .to_string()
            .contains("stale")
    );
    assert_eq!(registry.get(receiver.id).unwrap(), accepted);
    edit.generation = receiver.generation;
    edit.mode = Some(WakeMode::Inbox); // Selecting the role default is an exact reset.
    registry.change_attention(receiver.id, &edit).unwrap();
    assert_eq!(
        registry
            .get(receiver.id)
            .unwrap()
            .attention
            .as_ref()
            .unwrap()
            .rules,
        [rule(
            "channels/hidden",
            MessageClass::Conversation,
            WakeMode::Inbox
        )]
    );
    edit.destination = "channels/hidden".into();
    edit.class = MessageClass::Conversation;
    edit.mode = None;
    registry.change_attention(receiver.id, &edit).unwrap();
    let reopened = Registry::new(SqliteStore::open(root.path()).unwrap());
    assert_eq!(reopened.get(receiver.id).unwrap().attention, None);
}
