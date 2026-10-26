use bboxybook::{gallery, tabs};
use serde_json::json;
use std::collections::BTreeSet;
// keeps: spec renderer.md §4.5
#[test]
fn all_current_and_legacy_tab_grid_cases_are_paired_without_key_collisions() {
    let mut cases = tabs::cases();
    assert_eq!(cases.len(), 104);
    assert_eq!(tabs::GRID_STATES.len(), 8);
    assert_eq!(tabs::TAB_STATES.len(), 5);
    let legacy = tabs::legacy_cases();
    assert_eq!(legacy.len(), 104);
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
        208
    );
    assert!(gallery::cases().iter().any(|c| c.component == "tab_row"));
}
// keeps: spec renderer.md §4.5
#[test]
fn tab_grid_layout_does_not_consume_child_dom_rectangles_fonts_or_backdrop_height() {
    for kind in ["tab-row", "mini-grid"] {
        let mut c = tabs::cases()
            .into_iter()
            .find(|c| c.component == kind)
            .unwrap();
        let tree = json!({"dir":"v","ratio":0.3,"a":{"key":"a","name":"A","harness":"codex","state":"active"},"b":{"key":"b","name":"B","harness":"claude","state":"waiting"}});
        let args = if kind == "tab-row" {
            json!([{"name":"scoped task","tree":tree,"shortcut":"⌘3"},{"expanded":true,"focusedKey":"b"}])
        } else {
            json!([tree, "b", true])
        };
        c.fixture = Some(
            json!({"layer":"tabs","kind":kind,"host":{"x":14.,"y":14.,"w":300.,"h":999},"gap":24.,"backdrop":{"crop":{"x":17.,"y":25.}},"items":[{"args":args,"size":[240.,160.],"parts":[{"key":"name","rect":{"x":5.,"y":9.,"w":70.,"h":15.},"css":{"font":"600 12.5px system-ui"}}]}]}),
        );
        let original = format!("{:?}", tabs::layout_case(&c).unwrap());
        let f = c.fixture.as_mut().unwrap();
        f["host"]["h"] = json!(1.);
        f["items"][0]["parts"][0]["rect"] = json!({"x":-999,"y":8888,"w":1,"h":3000});
        f["items"][0]["parts"][0]["css"]["font"] = json!("900 85px fantasy");
        assert_eq!(original, format!("{:?}", tabs::layout_case(&c).unwrap()));
    }
}
