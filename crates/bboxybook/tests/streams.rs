use bboxybook::{parity::CasePolicy, streams};
use serde_json::json;
use std::collections::BTreeSet;
// keeps: spec renderer.md §4.5
#[test]
fn stream_matrix_covers_every_current_state_and_labels_legacy_cases() {
    let current = streams::cases();
    let old = streams::legacy_cases();
    assert_eq!(current.len(), 416);
    assert_eq!(old.len(), 80);
    assert!(current.iter().all(|c| {
        let p = CasePolicy::for_case(c).unwrap();
        p.text && !p.historical && p.geometry_tolerance_px == Some(0.5)
    }));
    assert!(
        old.iter()
            .all(|c| CasePolicy::for_case(c).unwrap().historical)
    );
    assert_eq!(
        current
            .iter()
            .chain(&old)
            .map(|c| c.relative())
            .collect::<BTreeSet<_>>()
            .len(),
        496
    );
    for (component, state) in [
        ("stream", "subchannel unread · more"),
        ("message-row", "priority and issuer"),
        ("message-row", "receipts"),
        ("compose", "delegate · reject refused"),
        ("compose", "attach · typed"),
    ] {
        assert_eq!(
            current
                .iter()
                .filter(|case| case.component == component && case.state == state)
                .count(),
            8
        );
    }
}
// keeps: spec renderer.md §4.5
#[test]
fn stream_layout_consumes_public_entries_but_never_browser_child_diagnostics() {
    let mut case = streams::cases()
        .into_iter()
        .find(|c| c.density == "cozy" && c.theme == "dark")
        .unwrap();
    case.fixture = Some(
        json!({"layer":"streams","backdrop":{"crop":{"x":10.,"y":20.}},"frames":[{"host":{"x":3.,"y":5.,"w":300.,"h":42.},"view":{"dock":true,"framed":true,"entries":[{"kind":"message","data":{"from":"worker","text":"a real message","role":"executor"}}]},"parts":[{"key":"e0","rect":{"x":900.,"y":-50.,"w":1.,"h":5.},"css":{"font":"900px fantasy"}}]}]}),
    );
    let before = format!("{:?}", streams::layout_case(&case).unwrap());
    case.fixture.as_mut().unwrap()["frames"][0]["host"]["h"] = json!(90000.);
    case.fixture.as_mut().unwrap()["frames"][0]["parts"] = json!([]);
    assert_eq!(
        before,
        format!("{:?}", streams::layout_case(&case).unwrap())
    );
    case.fixture.as_mut().unwrap()["frames"][0]["view"]["entries"][0]["data"]["text"] =
        json!("different public text");
    assert_ne!(
        before,
        format!("{:?}", streams::layout_case(&case).unwrap())
    );
}
