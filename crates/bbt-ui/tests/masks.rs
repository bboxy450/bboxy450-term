use bbt_ui::{Context, Density, Preset, Style, Theme, components as c};
use bbt_ui_model::layout::{Rect, TextMeasure, TextStyle};
struct Metrics;
impl TextMeasure for Metrics {
    fn width(&self, t: &str, s: &TextStyle) -> f32 {
        t.len() as f32 * s.size * 0.5
    }
}
impl c::Measure for Metrics {
    fn font_metrics(&self, s: &TextStyle) -> [f32; 2] {
        [s.size * 0.8, s.size * 0.2]
    }
}
// keeps: spec gui-components §3.2
#[test]
fn mask_keeps_layout_and_translates_local_coordinates_only_when_painted() {
    let ctx = Context::new(Theme::Dark, Density::Cozy);
    let mut b = c::Box::new(Style::new(Preset::Row), vec![]);
    let bounds = Rect::new(13., 27., 100., 24.);
    let plain = c::Element::Box(b.clone())
        .layout(bounds, ctx, &Metrics)
        .unwrap();
    let mask = c::LinearAlphaMask {
        start: [72., 0.],
        end: [100., 0.],
        alpha0: 1.,
        alpha1: 0.,
    };
    b.mask = Some(mask);
    let masked = c::Element::Box(b.clone())
        .layout(bounds, ctx, &Metrics)
        .unwrap();
    assert_eq!(plain.rect, masked.rect);
    assert_eq!(plain.baseline, masked.baseline);
    let ops = c::paint(&masked, ctx);
    let [
        c::Paint::Group {
            mask: Some(painted),
            opacity: 1.,
            saturation: 1.,
            ..
        },
    ] = ops.as_slice()
    else {
        panic!("missing mask group")
    };
    assert_eq!(*painted, mask.translated([masked.rect.x, masked.rect.y]));
    b.scale = 0.5;
    let scaled = c::Element::Box(b.clone())
        .layout(bounds, ctx, &Metrics)
        .unwrap();
    let c::LayoutKind::Box {
        mask: Some(scaled_mask),
        ..
    } = scaled.kind
    else {
        panic!("missing scaled mask")
    };
    assert_eq!(scaled_mask.start, [36., 0.]);
    assert_eq!(scaled_mask.end, [50., 0.]);
    b.mask = Some(c::LinearAlphaMask {
        end: mask.start,
        ..mask
    });
    assert!(c::Element::Box(b).layout(bounds, ctx, &Metrics).is_err());
}
