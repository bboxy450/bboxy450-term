use bboxybook::{fixtures, gallery::Case};
use bbt_ui_model::{Theme, draw::Prim};

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
        assert!(
            labels.contains(&seat.name.as_str()),
            "{} is absent from native draw data",
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
