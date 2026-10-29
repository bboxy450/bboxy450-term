// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use bbt_proto::gui::{
    MAX_SECTION_WHEEL_DY, UiLayout, UiRequest, UiSectionScroll, UiSidebarSections,
};
use serde_json::json;

#[test]
fn section_wheel_controls_keep_exact_pixel_wire_and_refuse_nonfinite_or_oversized_input() {
    for (request, wire) in [
        (
            UiRequest::ChatWheel { dy: -28.5 },
            json!({"kind":"chat_wheel","body":{"dy":-28.5}}),
        ),
        (
            UiRequest::TaskWheel { dy: 56. },
            json!({"kind":"task_wheel","body":{"dy":56.0}}),
        ),
    ] {
        request.validate().unwrap();
        assert_eq!(serde_json::to_value(&request).unwrap(), wire);
        assert_eq!(serde_json::from_value::<UiRequest>(wire).unwrap(), request);
    }
    for dy in [
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        MAX_SECTION_WHEEL_DY + 1.,
        -MAX_SECTION_WHEEL_DY - 1.,
    ] {
        assert!(UiRequest::ChatWheel { dy }.validate().is_err());
        assert!(UiRequest::TaskWheel { dy }.validate().is_err());
    }
    for dy in [-MAX_SECTION_WHEEL_DY, 0., MAX_SECTION_WHEEL_DY] {
        UiRequest::ChatWheel { dy }.validate().unwrap();
        UiRequest::TaskWheel { dy }.validate().unwrap();
    }
}
#[test]
fn section_receipts_round_trip_actual_offsets_without_changing_old_frames() {
    let old = json!({"frame_seq":7,"size":[800.,600.],"swarm":null,"tab":null,"focused":null,"launcher_open":false,"page":null,"document":null,"panes":[],"tree":[]});
    let layout: UiLayout = serde_json::from_value(old.clone()).unwrap();
    assert!(layout.sidebar_sections.is_none());
    assert!(
        serde_json::to_value(layout)
            .unwrap()
            .get("sidebar_sections")
            .is_none()
    );
    let sections = UiSidebarSections {
        chat: UiSectionScroll {
            offset: 120.,
            max: 120.,
            following: true,
        },
        tasks: UiSectionScroll {
            offset: 20.,
            max: 80.,
            following: false,
        },
    };
    assert!(sections.valid());
    let mut new = old;
    new["sidebar_sections"] = serde_json::to_value(sections).unwrap();
    let layout: UiLayout = serde_json::from_value(new).unwrap();
    assert_eq!(layout.sidebar_sections, Some(sections));
}
#[test]
fn malformed_section_receipts_cannot_claim_nonfinite_unclamped_or_following_task_offsets() {
    for value in [
        json!({"offset":-1.,"max":10.,"following":false}),
        json!({"offset":11.,"max":10.,"following":false}),
        json!({"offset":0.,"max":-1.,"following":false}),
        json!({"offset":null,"max":10.,"following":false}),
    ] {
        assert!(serde_json::from_value::<UiSectionScroll>(value).is_err());
    }
    assert!(
        serde_json::from_str::<UiSectionScroll>(
            r#"{"offset":1e100,"max":1e100,"following":false}"#
        )
        .is_err()
    );
    assert!(serde_json::from_value::<UiSidebarSections>(json!({"chat":{"offset":0.,"max":0.,"following":true},"tasks":{"offset":0.,"max":0.,"following":true}})).is_err());
}
