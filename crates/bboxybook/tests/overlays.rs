use bboxybook::{components, overlays};
use bbt_render::chrome::ChromeRenderer;
use bbt_ui::{self as ui, components as c};
use bbt_ui_model::{
    draw::{Color, DrawList},
    layout::Rect,
};
use std::collections::BTreeSet;

// keeps: spec renderer.md §4.5
#[test]
fn layer_matrix_distinguishes_full_primitives_from_module_surfaces() {
    let primitives = overlays::cases();
    let surfaces = overlays::surface_cases();
    assert_eq!(primitives.len(), 16);
    assert_eq!(surfaces.len(), 24);
    let keys = primitives
        .iter()
        .chain(&surfaces)
        .map(|c| c.relative())
        .collect::<BTreeSet<_>>();
    assert_eq!(keys.len(), 40);
    assert!(keys.contains("components/modal-cozy-dark.png"));
    assert!(keys.contains("launcher/root-cozy-dark.png"));
    assert!(keys.contains("launcher/root-over-pattern-cozy-dark.png"));
    assert!(keys.contains("documents/as-modal-cozy-dark.png"));
    for case in &surfaces {
        let p = bboxybook::parity::CasePolicy::for_case(case).unwrap();
        assert!(!p.text);
        assert_eq!(p.changed_pixel_budget, 0.001);
        assert!(p.scope.starts_with("container surfaces only"));
    }
}

// keeps: spec renderer.md §4.5
#[test]
fn overlay_blurs_only_the_backdrop_and_preserves_foreground_and_outside_pixels() {
    let ctx = ui::Context::new(ui::Theme::Dark, ui::Density::Cozy);
    let mut renderer = ChromeRenderer::new(vec![]).unwrap();
    let mut background = DrawList::default();
    for x in (0..200).step_by(4) {
        background.rect(
            Rect::new(x as f32, 0., 4., 160.),
            0.,
            if x % 8 == 0 {
                Color::BLACK
            } else {
                Color::WHITE
            },
        );
    }
    let base = renderer.render_to_rgba(&background, 200, 160, 1.).unwrap();
    let mut samples = vec![];
    for radius in [0., 14.] {
        let mut s = ui::Style::new(ui::Preset::Card);
        s.overrides.width = Some(ui::Length::Px(80.));
        s.overrides.height = Some(ui::Length::Px(40.));
        s.overrides.fill = Some(ui::Paint::solid(ui::Color::Rgba(Color::WHITE.0)));
        s.overrides.border = Some(ui::Border {
            width: ui::Length::Px(0.),
            paint: ui::Paint::solid(ui::Color::Transparent),
            outside: false,
        });
        s.overrides.radii = Some([ui::Length::Px(0.); 4]);
        let mut o = c::Overlay::new(c::Element::Box(c::Box::new(s, vec![])));
        o.min_height = ui::Length::Px(120.);
        o.style.overrides.fill = Some(ui::Paint::solid(ui::Color::Transparent));
        o.style.overrides.blur = Some(ui::Blur {
            radius: ui::Length::Px(radius),
            saturation: 1.,
        });
        let layout = c::Element::Overlay(o)
            .layout(Rect::new(20., 20., 160., 0.), ctx, &components::Metrics)
            .unwrap();
        let pixels = components::execute(
            &mut renderer,
            c::paint(&layout, ctx),
            base.clone(),
            [200, 160],
            ctx,
        )
        .unwrap();
        for y in 0..160 {
            for x in 0..200 {
                let n = (y * 200 + x) * 4;
                if !(20..180).contains(&x) || !(20..140).contains(&y) {
                    assert_eq!(
                        pixels[n..n + 4],
                        base[n..n + 4],
                        "blur leaked outside Overlay"
                    );
                }
                if (64..136).contains(&x) && (64..96).contains(&y) {
                    assert_eq!(
                        pixels[n..n + 4],
                        [255; 4],
                        "foreground was blurred or dimmed"
                    );
                }
            }
        }
        samples.push(pixels);
    }
    let contrast = |p: &Vec<u8>| {
        (40..160)
            .map(|x| p[(40 * 200 + x) * 4].abs_diff(p[(40 * 200 + x + 1) * 4]) as u32)
            .sum::<u32>()
    };
    assert!(
        contrast(&samples[1]) < contrast(&samples[0]) / 8,
        "background must visibly blur"
    );
}
