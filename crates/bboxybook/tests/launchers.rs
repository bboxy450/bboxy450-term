use bboxybook::launchers;
use bbt_ui::components as c;
use std::collections::BTreeSet;

#[test]
fn matrix_has_every_current_lab_state_and_separate_historical_keys() {
    let mut all = launchers::cases();
    assert_eq!(all.len(), 208);
    assert_eq!(
        all.iter().filter(|c| c.component == "launcher").count(),
        144
    );
    assert_eq!(all.iter().filter(|c| c.component == "picker").count(), 64);
    let old = launchers::legacy_cases();
    assert_eq!(old.len(), 48);
    assert!(
        old.iter().all(|c| c.component == "launcher-v2"
            && c.fixture.as_ref().unwrap()["reference"] == "baseline")
    );
    all.extend(old);
    assert_eq!(
        all.iter()
            .map(|c| c.relative())
            .collect::<BTreeSet<_>>()
            .len(),
        256
    );
    for state in [
        "query · focused empty",
        "query · caret",
        "query · selection",
        "query · preedit",
    ] {
        assert_eq!(
            all.iter()
                .filter(|c| c.component == "launcher" && c.state == state)
                .count(),
            8
        );
    }
    assert!(
        all.iter()
            .all(|c| bboxybook::parity::CasePolicy::for_case(c).unwrap().text)
    );
}

#[test]
fn geometry_uses_constructor_values_instead_of_child_dom_diagnostics() {
    let mut case = launchers::cases()
        .into_iter()
        .find(|c| c.density == "cozy" && c.theme == "dark")
        .unwrap();
    let mut fixture: serde_json::Value =
        serde_json::from_str(include_str!("../fixtures/launcher-machines.json")).unwrap();
    fixture["parts"] = serde_json::json!([{"key":"picker/div/opt2/sub","rect":{"x":-999.,"y":8888.,"w":1.,"h":9000.},"font_diagnostics":{"font":"90px fantasy","baseline":1.}}]);
    case.fixture = Some(fixture);
    let before = format!("{:?}", launchers::layout_case(&case).unwrap());
    let fixture = case.fixture.as_mut().unwrap();
    fixture["host"]["h"] = serde_json::json!(99999.);
    fixture["parts"][0]["rect"]["w"] = serde_json::json!(99999.);
    fixture["parts"][0]["font_diagnostics"]["baseline"] = serde_json::json!(-999.);
    assert_eq!(
        before,
        format!("{:?}", launchers::layout_case(&case).unwrap())
    );
    case.fixture.as_mut().unwrap()["args"]["query"] = serde_json::json!("studio");
    assert_ne!(
        before,
        format!("{:?}", launchers::layout_case(&case).unwrap())
    );
}
#[test]
fn a_machine_subtitle_with_room_stays_on_one_line() {
    let mut case = launchers::cases()
        .into_iter()
        .find(|c| c.density == "cozy" && c.theme == "dark")
        .unwrap();
    case.fixture =
        Some(serde_json::from_str(include_str!("../fixtures/launcher-machines.json")).unwrap());
    let root = launchers::layout_case(&case).unwrap();
    fn find(n: &c::Layout) -> Option<&c::Layout> {
        if matches!(&n.kind,c::LayoutKind::Named(k) if k=="picker/div/opt2/sub") {
            return Some(n);
        }
        n.children.iter().find_map(find)
    }
    let sub = find(&root).unwrap();
    assert_eq!(sub.rect.h, 21., "{sub:#?}");
}
