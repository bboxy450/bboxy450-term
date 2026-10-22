use bboxybook::{parity::CasePolicy, settings};
use serde_json::json;
#[test]
fn settings_matrix_covers_all_sections_draft_and_tab_in_every_density_and_theme() {
    let cases = settings::cases();
    assert_eq!(cases.len(), 72);
    assert_eq!(settings::legacy_cases().len(), 72);
    assert!(cases.iter().all(|c| {
        let p = CasePolicy::for_case(c).unwrap();
        p.text && !p.historical && p.geometry_tolerance_px == Some(0.5)
    }));
}
#[test]
fn settings_uses_public_records_and_viewport_not_diagnostic_geometry_or_css() {
    let mut case = settings::cases().remove(0);
    case.fixture = Some(
        json!({"layer":"settings","backdrop":{"crop":{"x":10,"y":20}},"frames":[{"host":{"x":3,"y":4,"w":1216,"h":638},"view":{"section":"Cloud","values":{"cloud":[["host-owned provider","public region",true,"#345678"]]}},"parts":[{"key":"card0","rect":{"x":999999,"y":-99,"w":1,"h":1},"css":{"font":"900px fantasy"}}]}]}),
    );
    let first = format!("{:?}", settings::layout_case(&case).unwrap());
    case.fixture.as_mut().unwrap()["frames"][0]["parts"] = json!([]);
    assert_eq!(
        first,
        format!("{:?}", settings::layout_case(&case).unwrap())
    );
    case.fixture.as_mut().unwrap()["frames"][0]["view"]["values"]["cloud"][0][0] =
        json!("a different host-owned provider");
    assert_ne!(
        first,
        format!("{:?}", settings::layout_case(&case).unwrap())
    );
}
#[test]
fn placeholder_settings_hand_painter_is_removed() {
    let source = include_str!("../src/modules.rs");
    assert!(source.contains("crate::settings::paint_legacy"));
    assert!(!source.contains("let nav = 150.0"));
}
