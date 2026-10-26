use bboxybook::components;
use bbt_render::chrome::ChromeRenderer;
use bbt_ui::{Context, Density, Theme, components as c};
use bbt_ui_model::{
    draw::{Color, DrawList},
    layout::Rect,
};
use std::collections::BTreeSet;

// keeps: spec renderer.md §4.5
#[test]
fn retained_image_preserves_asset_colour_rounding_and_group_filtering() {
    let mut renderer = ChromeRenderer::new(vec![bbt_render::chrome::icons::Image {
        name: "portrait".into(),
        width: 8,
        height: 8,
        rgba: [64, 128, 192, 255].repeat(64),
    }])
    .unwrap();
    let ctx = Context::new(Theme::Dark, Density::Cozy);
    let image = c::Element::Image {
        asset: "portrait".into(),
        size: bbt_ui::Length::Px(18.),
        radius: bbt_ui::Length::Px(9.),
    };
    let layout = image
        .clone()
        .layout(Rect::new(2., 2., 18., 0.), ctx, &components::Metrics)
        .unwrap();
    let base = [0, 0, 0, 255].repeat(24 * 24);
    let pixels = components::execute(
        &mut renderer,
        c::paint(&layout, ctx),
        base.clone(),
        [24, 24],
        ctx,
    )
    .unwrap();
    let pixel =
        |pixels: &[u8], x: usize, y: usize| pixels[(y * 24 + x) * 4..(y * 24 + x) * 4 + 4].to_vec();
    assert_eq!(pixel(&pixels, 11, 11), [64, 128, 192, 255]);
    assert_eq!(pixel(&pixels, 2, 2), [0, 0, 0, 255]);
    assert_eq!(pixel(&pixels, 21, 11), [0, 0, 0, 255]);
    let layout = c::Element::Filter {
        content: Box::new(image),
        opacity: 0.5,
        saturation: 0.,
    }
    .layout(Rect::new(2., 2., 18., 0.), ctx, &components::Metrics)
    .unwrap();
    let dimmed =
        components::execute(&mut renderer, c::paint(&layout, ctx), base, [24, 24], ctx).unwrap();
    assert!(
        pixel(&dimmed, 11, 11)
            .iter()
            .zip([60, 60, 60, 255])
            .all(|(a, b)| a.abs_diff(b) <= 1)
    );
}

// keeps: spec renderer.md §4.5
#[test]
fn a_retained_group_keeps_material_coverage_and_filters_the_completed_layer() {
    let mut renderer = ChromeRenderer::new(vec![]).unwrap();
    let ctx = Context::new(Theme::Dark, Density::Cozy);
    let ops = || {
        vec![c::Paint::Material {
            rect: Rect::new(20., 20., 24., 24.),
            radii: [4.; 4],
            kind: bbt_ui::Material::Glow,
            color: Color::WHITE,
            blur: None,
            clips: vec![],
            rim: Some(bbt_ui::Rim {
                halo_alpha: Some(0.8),
                particles: Some(0),
                ..Default::default()
            }),
        }]
    };
    let base = vec![0; 64 * 64 * 4];
    let normal = components::execute(&mut renderer, ops(), base.clone(), [64, 64], ctx).unwrap();
    assert!(
        normal.as_chunks::<4>().0.iter().all(|p| p[3] == 0),
        "ordinary fx preserve destination alpha"
    );
    let grouped = components::execute(
        &mut renderer,
        vec![c::Paint::Group {
            ops: ops(),
            opacity: 0.45,
            saturation: 0.,
            mask: None,
        }],
        base,
        [64, 64],
        ctx,
    )
    .unwrap();
    assert!(
        grouped.as_chunks::<4>().0.iter().any(|p| p[3] > 0),
        "offscreen fx must retain their coverage"
    );
    for p in grouped.as_chunks::<4>().0 {
        assert!(p[..3].iter().all(|c| *c <= p[3] + 1));
        assert!(p[0].abs_diff(p[1]) <= 1 && p[1].abs_diff(p[2]) <= 1);
    }
}

// keeps: spec renderer.md §4.5
#[test]
fn retained_scroll_content_moves_but_never_paints_outside_the_viewport() {
    let mut renderer = ChromeRenderer::new(vec![]).unwrap();
    let ctx = Context::new(Theme::Dark, Density::Cozy);
    let mut background = DrawList::default();
    background.rect(Rect::new(0., 0., 640., 260.), 0., Color::hex("#192433"));
    let base = renderer.render_to_rgba(&background, 640, 260, 1.).unwrap();
    let mut samples = vec![];
    for name in ["scroll-top", "scroll-end"] {
        let layout = components::preview(name, ctx).unwrap();
        let image = components::execute(
            &mut renderer,
            c::paint(&layout, ctx),
            base.clone(),
            [640, 260],
            ctx,
        )
        .unwrap();
        let mut changed_inside = 0;
        for y in 0..260 {
            for x in 0..640 {
                let n = (y * 640 + x) * 4;
                if (24..616).contains(&x) && (64..224).contains(&y) {
                    changed_inside += usize::from(image[n..n + 4] != base[n..n + 4]);
                } else {
                    assert_eq!(
                        image[n..n + 4],
                        base[n..n + 4],
                        "{name}: painted outside the viewport at {x},{y}"
                    );
                }
            }
        }
        assert!(changed_inside > 1000, "scroll view must contain real rows");
        samples.push(image);
    }
    assert_ne!(
        samples[0], samples[1],
        "the end offset must display different content"
    );
}

// keeps: spec renderer.md §4.5
#[test]
fn component_cases_are_canonical_and_box_gallery_covers_materials_and_radii() {
    let cases = components::cases();
    assert_eq!(cases.len(), 40);
    assert_eq!(
        cases
            .iter()
            .map(|c| c.relative())
            .collect::<BTreeSet<_>>()
            .len(),
        40
    );
    assert!(
        cases
            .iter()
            .any(|c| c.relative() == "components/text-cozy-dark.png")
    );
    for name in ["box-flat", "box-gloss", "box-frosted", "box-glow"] {
        let l = components::preview(name, Context::new(Theme::Light, Density::Cozy)).unwrap();
        assert_eq!(l.children.len(), 4);
        for (child, radius) in l.children.iter().zip([0., 6., 12., 20.]) {
            let c::LayoutKind::Box { style, .. } = &child.kind else {
                panic!("Box layout")
            };
            assert_eq!(style.radii, [radius; 4]);
            assert_eq!(child.rect.h, 112.);
        }
    }
}
