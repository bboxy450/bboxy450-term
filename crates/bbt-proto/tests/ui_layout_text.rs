// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! L1: text observations are additive; an absent observation never means empty text.
use bbt_proto::gui::UiLayoutNode;
use serde_json::json;

// keeps: ctl-cli §1a and e2e L1 — old solved-layout bytes remain unchanged.
#[test]
fn legacy_layout_nodes_omit_unavailable_text_and_keep_their_wire_shape() {
    let value = json!({"kind":"text","rect":[1.0,2.0,3.0,4.0],"children":[]});
    let decoded: UiLayoutNode = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(decoded.text, None);
    assert_eq!(serde_json::to_value(decoded).unwrap(), value);
}

// keeps: e2e L1 — scripts read the actual positioned label, not a model reconstruction.
#[test]
fn observed_text_preserves_unicode_and_distinguishes_empty_from_unavailable() {
    let value = json!({"kind":"text","rect":[1.0,2.0,3.0,4.0],"children":[],
        "text":"Claude ▸ Architect"});
    let mut decoded: UiLayoutNode = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(decoded.text.as_deref(), Some("Claude ▸ Architect"));
    assert_eq!(serde_json::to_value(&decoded).unwrap(), value);
    decoded.text = Some(String::new());
    assert_eq!(serde_json::to_value(decoded).unwrap()["text"], "");
}

// keeps: ctl-cli §1a — query input stays bounded and never carries control bytes.
#[test]
fn query_keystrokes_admit_literals_without_admitting_control_sequences() {
    use bbt_proto::gui::UiRequest;
    for key in ["c", "o", "d", "e", "x", " ", "-", "λ", "cmd-t"] {
        assert!(UiRequest::Key(vec![key.into()]).validate().is_ok(), "{key}");
    }
    for key in ["\0", "\u{1b}", "\n", "ab", "paste:codex"] {
        assert!(UiRequest::Key(vec![key.into()]).validate().is_err());
    }
    assert!(UiRequest::Key(vec!["x".into(); 33]).validate().is_err());
}
