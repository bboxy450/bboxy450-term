use bboxybook::{documents, parity::CasePolicy};
use serde_json::json;
#[test]
fn document_matrix_covers_all_types_and_both_modal_presentations_and_source() {
    let cases = documents::cases();
    assert_eq!(cases.len(), 120);
    assert!(documents::legacy_cases().is_empty());
    assert_eq!(
        cases.iter().filter(|c| c.slug == "as-modal-source").count(),
        8
    );
    assert!(cases.iter().all(|c| {
        let p = CasePolicy::for_case(c).unwrap();
        p.text
            && !p.historical
            && p.changed_pixel_budget == 0.02
            && p.geometry_tolerance_px == Some(0.5)
    }));
}
#[test]
fn native_document_layout_consumes_public_records_not_css_or_child_geometry() {
    let mut case = documents::cases().remove(0);
    case.fixture = Some(
        json!({"layer":"documents","backdrop":{"crop":{"x":10,"y":20}},"frames":[{"view":{"document":{"type":"note","title":"Host record","path":"README.md","body":"A simple static paragraph."}},"host":{"x":4,"y":5,"w":1218,"h":0},"parts":[{"key":"view","rect":{"x":99999,"y":99999,"w":1,"h":1},"css":{"font":"900px fantasy"}}],"probes":[{"margin":"10000px"}]}]}),
    );
    let before = format!("{:?}", documents::layout_case(&case).unwrap());
    case.fixture.as_mut().unwrap()["frames"][0]["parts"] = json!([]);
    case.fixture.as_mut().unwrap()["frames"][0]["probes"] = json!([]);
    assert_eq!(
        before,
        format!("{:?}", documents::layout_case(&case).unwrap())
    );
    case.fixture.as_mut().unwrap()["frames"][0]["view"]["document"]["title"] =
        json!("Changed by the host");
    assert_ne!(
        before,
        format!("{:?}", documents::layout_case(&case).unwrap())
    );
}
