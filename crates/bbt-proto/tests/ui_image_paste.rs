// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use bbt_proto::gui::{UiLayout, UiRequest};
use serde_json::json;
#[test]
fn image_paste_operator_request_has_exact_bounded_wire_shape() {
    let request = UiRequest::PasteImage {
        path: "/tmp/fixture.png".into(),
    };
    request.validate().unwrap();
    let wire = json!({"kind":"paste_image","body":{"path":"/tmp/fixture.png"}});
    assert_eq!(serde_json::to_value(&request).unwrap(), wire);
    assert_eq!(serde_json::from_value::<UiRequest>(wire).unwrap(), request);
    for path in [
        "relative.png".into(),
        "/tmp/bad\n.png".into(),
        "x".repeat(4097),
    ] {
        assert!(UiRequest::PasteImage { path }.validate().is_err());
    }
}
#[test]
fn image_paste_reply_is_additive_to_existing_layouts() {
    let wire = json!({"frame_seq":7,"size":[800.,600.],"swarm":null,"tab":null,"focused":null,"launcher_open":false,"page":null,"document":null,"panes":[],"tree":[]});
    let layout: UiLayout = serde_json::from_value(wire).unwrap();
    assert!(layout.pasted_image.is_none());
    assert!(
        serde_json::to_value(layout)
            .unwrap()
            .get("pasted_image")
            .is_none()
    );
}
