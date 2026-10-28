// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use bbt_proto::gui::{UiCompose, UiComposeTarget, UiLayout, UiRequest};
use ulid::Ulid;
use serde_json::json;

// keeps: spec ctl-cli §1a; M2 composer relay
#[test]
fn compose_targets_round_trip_in_the_existing_ui_relay() {
    for (to, value) in [
        (
            UiComposeTarget::Seat(Ulid::from(1)),
            json!({"kind":"seat","value":"00000000000000000000000001"}),
        ),
        (
            UiComposeTarget::Role("executor".into()),
            json!({"kind":"role","value":"executor"}),
        ),
        (
            UiComposeTarget::Channel("work".into()),
            json!({"kind":"channel","value":"work"}),
        ),
    ] {
        let request = UiRequest::Compose(UiCompose {
            to,
            text: "hello".into(),
        });
        request.validate().unwrap();
        let wire = json!({"kind":"compose","body":{"to":value,"text":"hello"}});
        assert_eq!(serde_json::to_value(&request).unwrap(), wire);
        assert_eq!(serde_json::from_value::<UiRequest>(wire).unwrap(), request);
    }
}

// keeps: spec ctl-cli §1a; bounded operator UI input
#[test]
fn compose_refuses_unresolved_targets_and_empty_or_oversized_text() {
    for to in [
        UiComposeTarget::Seat(Ulid::nil()),
        UiComposeTarget::Role("".into()),
        UiComposeTarget::Channel("work\nother".into()),
    ] {
        assert!(
            UiRequest::Compose(UiCompose {
                to,
                text: "hello".into()
            })
            .validate()
            .is_err()
        );
    }
    for text in [" ".into(), "x".repeat(bbt_proto::messaging::MAX_TEXT + 1)] {
        assert!(
            UiRequest::Compose(UiCompose {
                to: UiComposeTarget::Role("executor".into()),
                text
            })
            .validate()
            .is_err()
        );
    }
}

// keeps: decision #320; additive wire
#[test]
fn old_frames_do_not_claim_a_composer_send() {
    let wire = json!({"frame_seq":7,"size":[800.,600.],"swarm":null,"tab":null,"focused":null,"launcher_open":false,"page":null,"document":null,"panes":[],"tree":[]});
    let layout: UiLayout = serde_json::from_value(wire).unwrap();
    assert!(layout.composed.is_none());
    assert!(
        serde_json::to_value(layout)
            .unwrap()
            .get("composed")
            .is_none()
    );
}
