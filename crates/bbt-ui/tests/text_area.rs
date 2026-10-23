use bbt_ui::{self as ui, components as c, widgets::text_area::TextArea};
use bbt_ui_model::layout::{Rect, TextMeasure, TextStyle};
struct Metrics;
impl TextMeasure for Metrics {
    fn width(&self, s: &str, _: &TextStyle) -> f32 {
        s.chars().count() as f32 * 5.
    }
}
impl c::Measure for Metrics {
    fn font_metrics(&self, _: &TextStyle) -> [f32; 2] {
        [9., 3.]
    }
}
fn runs(n: &c::Layout, out: &mut Vec<(String, Rect)>) {
    if let c::LayoutKind::Text { runs, .. } = &n.kind {
        out.extend(runs.iter().map(|r| (r.text.clone(), r.rect)));
    }
    for child in &n.children {
        runs(child, out);
    }
}
fn node<'a>(n: &'a c::Layout, key: &str) -> Option<&'a c::Layout> {
    if matches!(&n.kind,c::LayoutKind::Named(name) if name==key) {
        Some(n)
    } else {
        n.children.iter().find_map(|c| node(c, key))
    }
}
#[test]
fn editing_never_reflows_the_committed_paragraph_and_uses_shaped_positions() {
    let ctx = ui::Context::new(ui::Theme::Dark, ui::Density::Cozy);
    let style = ui::Text {
        size: Some(12.),
        line_height: 1.45,
        ..Default::default()
    };
    let rect = Rect::new(0., 0., 30., 0.);
    let value = "one two three";
    let paragraph = c::Element::Text(c::Text::new(value, style))
        .layout(rect, ctx, &Metrics)
        .unwrap();
    let area = TextArea {
        value: value.into(),
        placeholder: String::new(),
        style,
        edit: Some(c::Edit {
            caret: Some(6),
            ..Default::default()
        }),
        min_lines: 2,
        name: "body".into(),
    };
    let edited = area
        .element(30., ctx, &Metrics)
        .unwrap()
        .layout(rect, ctx, &Metrics)
        .unwrap();
    let (mut ordinary, mut focused) = (vec![], vec![]);
    runs(&paragraph, &mut ordinary);
    runs(&edited, &mut focused);
    assert_eq!(ordinary, focused);
    let caret = node(&edited, "body/caret").unwrap().rect;
    assert_eq!(caret.x, 9.);
    // Inline caret has vertical-align:-.2em; its top is one pixel above this line.
    assert_eq!(caret.y, 16.390625);
    assert_eq!(edited.rect.h, paragraph.rect.h);
}
#[test]
fn invalid_surrogate_boundaries_are_rejected_before_layout() {
    let area = TextArea {
        value: "🙂".into(),
        placeholder: String::new(),
        style: Default::default(),
        edit: Some(c::Edit {
            caret: Some(1),
            ..Default::default()
        }),
        min_lines: 2,
        name: "body".into(),
    };
    let ctx = ui::Context::new(ui::Theme::Dark, ui::Density::Cozy);
    assert!(area.element(200., ctx, &Metrics).is_err());
}
