// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use bboxybook::{graphs, parity::CasePolicy};
use serde_json::json;
// keeps: spec renderer.md §4.5
#[test]
fn graph_matrix_covers_each_live_card_and_the_empty_swarm() {
    let cases = graphs::cases();
    assert_eq!(
        cases.len(),
        (graphs::STATES.len() + graphs::E2E_STATES.len()) * 4 * 2
    );
    assert_eq!(cases.iter().filter(|c| c.page == "e2e.html").count(), 64);
    assert_eq!(graphs::legacy_cases().len(), 72);
    assert!(cases.iter().all(|c| {
        let p = CasePolicy::for_case(c).unwrap();
        p.text && !p.historical && p.geometry_tolerance_px == Some(0.5)
    }));
}
// keeps: spec renderer.md §4.5
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

// keeps: spec renderer.md §4.5
#[test]
fn checklist_layout_uses_public_promises_and_records_without_child_diagnostics() {
    let mut case = graphs::cases()
        .into_iter()
        .find(|c| c.page == "e2e.html" && c.density == "cozy" && c.theme == "dark")
        .unwrap();
    case.fixture = Some(json!({
        "layer":"graphs", "backdrop":{"crop":{"x":0,"y":0}},
        "frames":[{"standalone":true,"host":{"x":14,"y":14,"w":1218,"h":1},
            "view":{"only":["e2e"],"e2e":{
                "promises":[{"id":"T1","group":"terminal","text":"a public promise"}],
                "records":[{"function":"T1","status":"pass","last_run":"2026-10-24T03:12:00Z","run_by":"seat:worker"}],
                "tasks":[]}},
            "parts":[{"key":"e2e/head","rect":{"x":9999,"y":-99,"w":1,"h":1},"css":{"font":"900px fantasy"}}]}]
    }));
    let first = format!("{:?}", graphs::layout_case(&case).unwrap());
    case.fixture.as_mut().unwrap()["frames"][0]["parts"] = json!([]);
    case.fixture.as_mut().unwrap()["frames"][0]["host"]["h"] = json!(99999);
    assert_eq!(first, format!("{:?}", graphs::layout_case(&case).unwrap()));
    case.fixture.as_mut().unwrap()["frames"][0]["view"]["e2e"]["records"][0]["status"] =
        json!("blocked");
    assert_ne!(first, format!("{:?}", graphs::layout_case(&case).unwrap()));
}

// keeps: spec renderer.md §4.5
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
