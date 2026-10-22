use bboxybook::{graphs, parity::CasePolicy};
use serde_json::json;
#[test]
fn graph_matrix_covers_each_live_card_and_the_empty_swarm() {
    let cases = graphs::cases();
    assert_eq!(cases.len(), 64);
    assert_eq!(graphs::legacy_cases().len(), 72);
    assert!(cases.iter().all(|c| {
        let p = CasePolicy::for_case(c).unwrap();
        p.text && !p.historical && p.geometry_tolerance_px == Some(0.5)
    }));
}
#[test]
fn graph_layout_is_driven_by_records_not_browser_child_positions() {
    let mut case = graphs::cases()
        .into_iter()
        .find(|c| c.state == "agents" && c.density == "cozy" && c.theme == "dark")
        .unwrap();
    case.fixture = Some(
        json!({"layer":"graphs","backdrop":{"crop":{"x":10,"y":20}},"frames":[{"host":{"x":3,"y":4,"w":900,"h":10},"view":{"swarm":"public swarm","only":["agents"],"seats":[{"name":"worker","harness":"codex","state":"active"}]},"parts":[{"key":"agents","rect":{"x":99999,"y":-9,"w":1,"h":5},"css":{"font":"900px fantasy"}}]}]}),
    );
    let first = format!("{:?}", graphs::layout_case(&case).unwrap());
    let f = case.fixture.as_mut().unwrap();
    f["frames"][0]["parts"] = json!([]);
    f["frames"][0]["host"]["h"] = json!(1000000);
    assert_eq!(first, format!("{:?}", graphs::layout_case(&case).unwrap()));
    case.fixture.as_mut().unwrap()["frames"][0]["view"]["swarm"] = json!("different public name");
    assert_ne!(first, format!("{:?}", graphs::layout_case(&case).unwrap()));
}
#[test]
fn former_graph_and_provenance_hand_painters_are_removed() {
    let legacy = include_str!("../src/fixtures.rs");
    for old in ["fn graph(", "fn provenance(", "fn subscriptions("] {
        assert!(!legacy.contains(old));
    }
    assert!(legacy.contains("crate::graphs::paint_model"));
}

#[test]
fn channel_layout_never_consumes_browser_svg_diagnostics() {
    let mut case = graphs::cases()
        .into_iter()
        .find(|c| c.state == "channels")
        .unwrap();
    case.fixture = Some(
        json!({"layer":"graphs","backdrop":{"crop":{"x":0,"y":0}},"frames":[{"host":{"x":0,"y":0,"w":1000,"h":1},"view":{"only":["channels"],"channels":{"folders":[{"path":"c","label":"public folder"}],"sinks":[{"id":"one","name":"worker"}],"links":[{"from":"c","to":"one"}]}},"parts":[{"key":"channels/tree","rect":{"x":9999,"y":9999,"w":1,"h":1},"svg":{"d":"M999 999"}}]}]}),
    );
    let first = format!("{:?}", graphs::layout_case(&case).unwrap());
    case.fixture.as_mut().unwrap()["frames"][0]["parts"] = json!([]);
    assert_eq!(first, format!("{:?}", graphs::layout_case(&case).unwrap()));
    case.fixture.as_mut().unwrap()["frames"][0]["view"]["channels"]["folders"][0]["label"] =
        json!("a different published label");
    assert_ne!(first, format!("{:?}", graphs::layout_case(&case).unwrap()));
}
