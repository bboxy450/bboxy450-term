// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use bboxybook::{fixtures, gallery::Case};
use bbt_ui::modules::seat_row::{first_name, role_label};
use bbt_ui_model::{Theme, draw::Prim};

// keeps: spec renderer.md §4.5
#[test]
fn sidebar_scene_uses_the_same_ten_seats_and_stage_as_the_web() {
    let snapshot: serde_json::Value =
        serde_json::from_str(include_str!("../fixtures/web-inputs-v2.json")).unwrap();
    let case: Case = serde_json::from_value(snapshot["cases"][0].clone()).unwrap();
    let calls = case.fixture.as_ref().unwrap()["calls"].as_array().unwrap();
    let call = calls.iter().find(|c| c["name"] == "sidebar").unwrap();
    let model = fixtures::model(&call["args"][0], &call["args"][1], Theme::Dark).unwrap();
    assert_eq!(model.seats.len(), 10);
    assert_eq!(
        model
            .window
            .sidebar
            .swarms
            .iter()
            .map(|s| (
                s.name.as_str(),
                s.tabs.iter().map(|t| t.seats.len()).sum::<usize>()
            ))
            .collect::<Vec<_>>(),
        vec![("term", 3), ("winnow", 6), ("ontology", 1)]
    );
    assert_eq!(calls.iter().filter(|c| c["name"] == "pane").count(), 1);
    let mut source_names: Vec<_> = call["args"][0]["spaces"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|swarm| swarm["tabs"].as_array().unwrap())
        .flat_map(|tab| tab["seats"].as_array().unwrap())
        .map(|seat| seat["name"].as_str().unwrap())
        .collect();
    let mut model_names: Vec<_> = model
        .seats
        .values()
        .map(|seat| seat.name.as_str())
        .collect();
    source_names.sort_unstable();
    model_names.sort_unstable();
    assert_eq!(
        model_names, source_names,
        "canonical full names are retained"
    );
    let list = bboxybook::gallery::draw(&case).unwrap();
    fn texts(primitives: &[Prim]) -> Vec<&str> {
        primitives
            .iter()
            .flat_map(|p| match p {
                Prim::Group { primitives, .. } => texts(primitives),
                Prim::Text { text, .. } => vec![text.as_ref()],
                _ => vec![],
            })
            .collect()
    }
    let labels = texts(&list.primitives);
    for seat in model.seats.values() {
        let role = role_label(
            seat.role.as_ref().map(|role| role.name.as_str()),
            &seat.harness.0,
        );
        let name = first_name(&seat.name);
        assert!(
            labels.windows(2).any(|pair| pair == [role.as_str(), name]),
            "role then first name for {} is absent from native draw data",
            seat.name
        );
    }
    // Retained Text shapes the uppercase band before emitting paint records.
    assert!(
        labels
            .iter()
            .any(|text| text.eq_ignore_ascii_case("ontology"))
    );
}
