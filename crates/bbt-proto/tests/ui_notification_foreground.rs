// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use bbt_proto::gui::UiLayout;
use serde_json::json;

#[test]
fn old_and_ordinary_layouts_do_not_invent_private_foreground_observations() {
    let wire = json!({"frame_seq":7,"size":[800.,600.],"swarm":null,"tab":null,"focused":null,"launcher_open":false,"page":null,"document":null,"panes":[],"tree":[]});
    let mut layout: UiLayout = serde_json::from_value(wire).unwrap();
    assert_eq!(layout.notification_frontmost, None);
    assert!(
        serde_json::to_value(&layout)
            .unwrap()
            .get("notification_frontmost")
            .is_none()
    );
    for frontmost in [false, true] {
        layout.notification_frontmost = Some(frontmost);
        let wire = serde_json::to_value(&layout).unwrap();
        assert_eq!(wire["frame_seq"], 7);
        assert_eq!(wire["notification_frontmost"], frontmost);
        assert_eq!(serde_json::from_value::<UiLayout>(wire).unwrap(), layout);
    }
}
