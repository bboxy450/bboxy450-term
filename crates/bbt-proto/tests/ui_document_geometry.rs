// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use bbt_proto::gui::*;
use serde_json::json;

fn observation() -> UiDocumentObservation {
    serde_json::from_value(json!({
        "frame_seq": 7,
        "private_test_run":"00000000000000000000000002",
        "source": {"record_id":"fixture", "number":1, "path":"e2e/D1.md",
            "digest":"digest", "etag":"etag", "root":"root", "provider_revision":2,
            "workspace":{"swarm":"00000000000000000000000001","revision":3}},
        "clips":[{"rect":[10,10,400,300],"radii":[3,3,3,3]}],
        "texts":[{"key":"block0", "text":"Native glyphs", "origin":[20,30],
            "rect":[20,30,90,21], "baseline":46, "font_size":14, "line_height":21,
            "weight":400, "mono":false, "italic":false, "max_width":null,
            "clip":[10,10,400,300], "clips":[0], "color":[1,1,1,1]}],
        "paths":[{"key":"block1/edge/0", "origin":[20,40], "scale":1,
            "commands":[{"op":"move","points":[0,0]},
                {"op":"cubic","points":[[1,2],[3,4],[5,6]]},
                {"op":"line","points":[8,10]},{"op":"close"}],
            "stroke_width":1.2,"clip":[10,10,400,300],"clips":[0],"color":[1,1,1,1],"fill":null}],
        "omitted":{"text_runs":0,"paths":0,"path_commands":0,"bytes":0,
            "walk_truncated":false,"unsupported_paint":false}
    }))
    .unwrap()
}

#[test]
fn actual_geometry_round_trips_without_changing_absent_legacy_layout_bytes() {
    let sample = observation();
    sample.validate().unwrap();
    assert!(sample.omitted.is_empty());
    let bytes = serde_json::to_vec(&sample).unwrap();
    assert_eq!(
        serde_json::from_slice::<UiDocumentObservation>(&bytes).unwrap(),
        sample
    );
    let legacy = json!({"frame_seq":1,"size":[800,600],"swarm":null,"tab":null,
        "focused":null,"launcher_open":false,"page":null,"document":null,
        "panes":[],"tree":[]});
    let layout: UiLayout = serde_json::from_value(legacy).unwrap();
    assert!(layout.document_observation.is_none());
    assert!(
        !serde_json::to_value(layout)
            .unwrap()
            .as_object()
            .unwrap()
            .contains_key("document_observation")
    );
    const { assert!(UI_CONTROL_VERSION >= 8) };
}

#[test]
fn count_command_byte_and_coordinate_bounds_refuse_malformed_observations() {
    let sample = observation();
    let decode = |v| serde_json::from_value::<UiDocumentObservation>(v);
    let mut value = serde_json::to_value(&sample).unwrap();
    value["texts"] = json!(vec![sample.texts[0].clone(); DOCUMENT_TEXT_RUNS + 1]);
    assert!(decode(value).is_err());
    let mut value = serde_json::to_value(&sample).unwrap();
    value["paths"] = json!(vec![sample.paths[0].clone(); DOCUMENT_PATHS + 1]);
    assert!(decode(value).is_err());
    let mut huge = sample.clone();
    huge.paths[0].commands = vec![UiDocumentPathCommand::Close; DOCUMENT_PATH_COMMANDS + 1];
    assert!(huge.validate().is_err());
    assert!(decode(serde_json::to_value(huge).unwrap()).is_err());
    let mut huge = sample.clone();
    huge.paths = vec![sample.paths[0].clone(); 3];
    for path in &mut huge.paths {
        path.commands = vec![UiDocumentPathCommand::Close; 800];
    }
    assert!(decode(serde_json::to_value(huge).unwrap()).is_err());
    let mut huge = sample.clone();
    huge.texts = vec![sample.texts[0].clone(); DOCUMENT_TEXT_RUNS];
    for text in &mut huge.texts {
        text.text = "x".repeat(1024);
    }
    assert!(decode(serde_json::to_value(huge).unwrap()).is_err());
    for (key, value) in [
        ("font_size", json!(-1)),
        ("origin", json!([null, 2])),
        ("rect", json!([0, 0, -1, 1])),
        ("clip", json!([0, 0, 1, -1])),
    ] {
        let mut invalid = serde_json::to_value(&sample).unwrap();
        invalid["texts"][0][key] = value;
        assert!(decode(invalid).is_err(), "{key}");
    }
    let mut invalid = serde_json::to_value(&sample).unwrap();
    invalid["paths"][0]["commands"][0] = json!({"op":"javascript","points":[1,2]});
    assert!(decode(invalid).is_err());
    for field in ["texts", "paths"] {
        let mut invalid = serde_json::to_value(&sample).unwrap();
        invalid[field][0]["clips"] = json!([1]);
        assert!(decode(invalid).is_err(), "unknown shared clip");
        let mut invalid = serde_json::to_value(&sample).unwrap();
        invalid[field][0]["clips"] = json!(vec![0; 17]);
        assert!(decode(invalid).is_err(), "clip stack bound");
    }
    let mut invalid = serde_json::to_value(&sample).unwrap();
    invalid["clips"] = json!(vec![sample.clips[0].clone(); DOCUMENT_CLIPS + 1]);
    assert!(decode(invalid).is_err());
    let mut invalid = serde_json::to_value(&sample).unwrap();
    invalid["clips"][0]["radii"] = json!([-1, 0, 0, 0]);
    assert!(decode(invalid).is_err());
    for (key, value) in [
        ("frame_seq", json!(0)),
        ("private_test_run", json!("00000000000000000000000000")),
    ] {
        let mut invalid = serde_json::to_value(&sample).unwrap();
        invalid[key] = value;
        assert!(decode(invalid).is_err());
    }
    let mut invalid = sample;
    invalid.paths[0].origin[0] = f32::INFINITY;
    assert!(invalid.validate().is_err());
}

#[test]
fn truncation_is_explicit_and_never_a_successful_full_observation() {
    let mut sample = observation();
    sample.omitted.walk_truncated = true;
    sample.omitted.text_runs = 1;
    let restored: UiDocumentObservation =
        serde_json::from_value(serde_json::to_value(&sample).unwrap()).unwrap();
    assert!(!restored.omitted.is_empty());
    assert_eq!(restored.omitted, sample.omitted);
}

#[test]
fn verified_test_run_is_optional_typed_and_independent_of_reason_text() {
    use bbt_proto::{GuiIdentity, GuiIdentityState};
    let legacy = json!({"state":"certified","reason":"test-account:01ARZ3NDEKTSV4RRFFQ69G5FAV"});
    let value: GuiIdentity = serde_json::from_value(legacy.clone()).unwrap();
    assert_eq!(value.private_test_run, None);
    assert_eq!(serde_json::to_value(value).unwrap(), legacy);
    let mut value = GuiIdentity {
        state: GuiIdentityState::Certified,
        reason: None,
        private_test_run: Some(ulid::Ulid::from(1)),
    };
    let restored: GuiIdentity =
        serde_json::from_value(serde_json::to_value(&value).unwrap()).unwrap();
    assert_eq!(restored.private_test_run, value.private_test_run);
    value.private_test_run = Some(ulid::Ulid::nil());
    assert!(serde_json::from_value::<GuiIdentity>(serde_json::to_value(value).unwrap()).is_err());
}
