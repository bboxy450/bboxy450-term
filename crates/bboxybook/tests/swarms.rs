use bboxybook::swarms;
use serde_json::json;
use std::collections::BTreeSet;
// keeps: spec renderer.md §4.5
#[test]
fn swarm_matrix_covers_every_current_and_legacy_state_with_distinct_keys() {
    let mut cases = swarms::cases();
    assert_eq!(cases.len(), 72);
    let legacy = swarms::legacy_cases();
    assert_eq!(legacy.len(), 64);
    assert!(
        legacy
            .iter()
            .all(|c| c.fixture.as_ref().unwrap()["reference"] == "baseline")
    );
    cases.extend(legacy);
    assert_eq!(
        cases
            .iter()
            .map(|c| c.relative())
            .collect::<BTreeSet<_>>()
            .len(),
        136
    );
}
// keeps: spec renderer.md §4.5
#[test]
fn captured_child_geometry_and_fonts_do_not_enter_card_or_sidebar_layout() {
    let seat = json!({"key":"nyx","name":"Nyx","harness":"codex","state":"active"});
    let tab = json!({"name":"test","seats":[seat.clone()],"tree":seat,"shortcut":""});
    let space = json!({"key":"term","name":"term","tabs":[tab]});
    for (page, kind, args) in [
        ("space-header", "card", json!([space.clone(), {}])),
        (
            "sidebar",
            "sidebar",
            json!([{"spaces":[space.clone()]},{"openSpaces":["term"]}]),
        ),
    ] {
        let mut case = swarms::cases()
            .into_iter()
            .find(|c| c.component == page)
            .unwrap();
        case.fixture = Some(
            json!({"layer":"swarms","kind":kind,"args":args,"body_tabs":[],"host":{"x":10.,"y":12.,"w":280.,"h":400.},"backdrop":{"crop":{"x":17.,"y":25.}},"phone":false,"scroll":0.,"parts":[{"key":"head","rect":{"x":4.,"y":4.,"w":20.,"h":9.},"css":{"font":"12px system-ui"}}]}),
        );
        let before = format!("{:?}", swarms::layout_case(&case).unwrap());
        let f = case.fixture.as_mut().unwrap();
        f["parts"][0]["rect"] = json!({"x":9999.,"y":-555.,"w":1.,"h":9000.});
        f["parts"][0]["css"]["font"] = json!("100px fantasy");
        assert_eq!(before, format!("{:?}", swarms::layout_case(&case).unwrap()));
    }
}
