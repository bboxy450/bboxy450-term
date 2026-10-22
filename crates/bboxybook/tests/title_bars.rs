use bboxybook::{parity::CasePolicy, title_bars};
use serde_json::json;
use std::collections::BTreeSet;

#[test]
fn titlebar_matrix_covers_current_states_and_keeps_legacy_diagnostic_only() {
    let mut all = title_bars::cases();
    assert_eq!(all.len(), 40);
    assert!(all.iter().all(|c| {
        let policy = CasePolicy::for_case(c).unwrap();
        !policy.historical && policy.text && policy.geometry_tolerance_px == Some(0.5)
    }));
    let old = title_bars::legacy_cases();
    assert_eq!(old.len(), 24);
    assert!(
        old.iter()
            .all(|c| CasePolicy::for_case(c).unwrap().historical)
    );
    all.extend(old);
    assert_eq!(
        all.iter()
            .map(|c| c.relative())
            .collect::<BTreeSet<_>>()
            .len(),
        64
    );
}
#[test]
fn native_titlebar_reads_public_strings_and_parent_width_not_child_diagnostics() {
    let mut case = title_bars::cases()
        .into_iter()
        .find(|c| c.density == "cozy" && c.theme == "dark")
        .unwrap();
    case.fixture = Some(
        json!({"layer":"titlebars","backdrop":{"crop":{"x":17.,"y":25.}},"host":{"x":10.,"y":20.,"w":418.,"h":38.},"args":[{"sidebarOpen":true,"crumb":"· term · Vesper Greywind","theme":"dark"}],"parts":[{"key":"bar/ttl","rect":{"x":0.,"y":0.,"w":999.,"h":1.},"css":{"font":"85px fantasy"}}]}),
    );
    let before = format!("{:?}", title_bars::layout_case(&case).unwrap());
    let f = case.fixture.as_mut().unwrap();
    f["host"]["h"] = json!(9000.);
    f["parts"][0]["rect"] = json!({"x":999.,"y":-999.,"w":1.,"h":9999.});
    f["parts"][0]["css"]["font"] = json!("1px monospace");
    assert_eq!(
        before,
        format!("{:?}", title_bars::layout_case(&case).unwrap())
    );
    case.fixture.as_mut().unwrap()["args"][0]["crumb"] = json!("another swarm");
    assert_ne!(
        before,
        format!("{:?}", title_bars::layout_case(&case).unwrap())
    );
}
