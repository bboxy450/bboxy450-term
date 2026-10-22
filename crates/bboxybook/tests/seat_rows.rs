use bboxybook::{gallery, seat_rows};
use std::collections::BTreeSet;
#[test]
fn seat_row_matrix_covers_lab_states_and_explicit_nested_row_fixtures() {
    let cases = seat_rows::cases();
    assert_eq!(cases.len(), 224);
    assert_eq!(
        cases
            .iter()
            .map(|c| c.relative())
            .collect::<BTreeSet<_>>()
            .len(),
        224
    );
    assert_eq!(seat_rows::STATES.len(), 26);
    for state in [
        "subagents-12",
        "muted-held",
        "authority-forked",
        "listeners",
        "task-states",
        "artifacts",
    ] {
        assert!(
            cases
                .iter()
                .any(|c| c.relative() == format!("seat-row/{state}-cozy-dark.png"))
        );
    }
    let legacy = seat_rows::legacy_cases();
    assert_eq!(legacy.len(), 72);
    assert!(
        legacy
            .iter()
            .all(|c| c.fixture.as_ref().unwrap()["reference"] == "baseline")
    );
    assert!(gallery::cases().iter().any(|c| c.component == "seat_row"));
}

#[test]
fn child_dom_rectangles_and_css_metrics_are_diagnostics_only() {
    use serde_json::json;
    let mut case = seat_rows::cases().remove(0);
    case.fixture = Some(
        json!({"layer":"seat-row","backdrop":{"crop":{"x":17.,"y":25.}},"groups":[{"host":{"x":14.,"y":14.,"w":300.},"gap":4.,"row_height":24.,"rows":[{"kind":"seatRow","args":[{"key":"row1","name":"Nyx Copperwing","harness":"codex","state":"active","role":"executor","title":"parity"},{"shortcut":"⌘2"}],"rect":{"x":14.,"y":14.,"w":300.,"h":24.},"parts":[{"key":"name","rect":{"x":43.,"y":17.25,"w":100.,"h":17.5},"css":{"font":"600 12.5px system-ui"}}]}]}]}),
    );
    let original = format!("{:?}", seat_rows::layout_case(&case).unwrap());
    let f = case.fixture.as_mut().unwrap();
    f["groups"][0]["rows"][0]["rect"] = json!({"x":9999,"y":-777,"w":1,"h":5000});
    f["groups"][0]["rows"][0]["parts"][0]["rect"] = json!({"x":-999,"y":8888,"w":3000,"h":1});
    f["groups"][0]["rows"][0]["parts"][0]["css"]["font"] = json!("900 85px fantasy");
    assert_eq!(
        original,
        format!("{:?}", seat_rows::layout_case(&case).unwrap())
    );
}
