use bbt_ui::{self as ui, components as c};
use bbt_ui_model::{
    draw::Color,
    layout::{Rect, TextMeasure, TextStyle},
};
struct Metrics;
impl TextMeasure for Metrics {
    fn width(&self, t: &str, s: &TextStyle) -> f32 {
        t.chars().count() as f32 * s.size * 0.5
    }
}
impl c::Measure for Metrics {
    fn font_metrics(&self, s: &TextStyle) -> [f32; 2] {
        [s.size * 0.8, s.size * 0.2]
    }
}
fn ctx() -> ui::Context {
    ui::Context::new(ui::Theme::Dark, ui::Density::Cozy)
}
fn modal() -> c::Element {
    ui::overlays::Modal::new(
        "Purge seat",
        c::Element::Text(c::Text::new(
            "Terminates now and moves its history to cold storage; retained for a later resume.",
            ui::Text::default(),
        )),
        vec![],
    )
    .element(ctx())
}

#[test]
fn overlay_centres_the_panel_and_narrow_bounds_reflow_its_content() {
    let element = c::Element::Overlay(c::Overlay::new(modal()));
    let wide = element
        .layout(Rect::new(10., 20., 600., 0.), ctx(), &Metrics)
        .unwrap();
    let narrow = element
        .layout(Rect::new(10., 20., 280., 0.), ctx(), &Metrics)
        .unwrap();
    assert_eq!(wide.children[0].rect.w, 380.);
    assert_eq!(wide.children[0].rect.x, 120.);
    assert_eq!(narrow.children[0].rect.w, 280.);
    assert_eq!(narrow.children[0].rect.x, 10.);
    assert!(narrow.children[0].rect.h > wide.children[0].rect.h);
    for layout in [wide, narrow] {
        let panel = layout.children[0].rect;
        assert!((panel.y + panel.h / 2. - (layout.rect.y + layout.rect.h / 2.)).abs() < 0.001);
        assert!(panel.y >= layout.rect.y && panel.bottom() <= layout.rect.bottom());
    }
}

#[test]
fn blur_parameters_round_trip_and_keep_the_backdrop_separate_from_foreground_fill() {
    let mut style = ui::Style::new(ui::Preset::Modal);
    style.overrides.width = Some(ui::Length::Px(380.));
    style.overrides.blur = Some(ui::Blur {
        radius: ui::Length::Px(14.),
        saturation: 1.15,
    });
    style.overrides.fill = Some(ui::Paint::solid(ui::Color::Rgba(Color::WHITE.0)));
    let encoded = serde_json::to_string(&style).unwrap();
    let decoded: ui::Style = serde_json::from_str(&encoded).unwrap();
    assert_eq!(decoded, style);
    let resolved = decoded.resolve(ctx()).unwrap();
    assert_eq!(resolved.width, Some(380.));
    assert_eq!(resolved.fill, [1.; 4]);
    assert_eq!(
        resolved.blur,
        Some(ui::ResolvedBlur {
            radius: 14.,
            saturation: 1.15
        })
    );
    for (radius, saturation) in [(257., 1.), (1., 4.1), (1., f32::NAN)] {
        style.overrides.blur = Some(ui::Blur {
            radius: ui::Length::Px(radius),
            saturation,
        });
        assert!(style.resolve(ctx()).is_err());
    }
}
