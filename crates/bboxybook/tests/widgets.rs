use bboxybook::{components, widgets};
use bbt_render::chrome::ChromeRenderer;
use bbt_ui::{Context, Density, Theme, components as c};
use bbt_ui_model::{
    draw::{Color, DrawList},
    layout::Rect,
};
use std::collections::BTreeSet;

// keeps: spec renderer.md §4.5
#[test]
fn control_matrix_has_canonical_keys_and_all_constructor_families() {
    let cases = widgets::cases();
    assert_eq!(cases.len(), 64);
    assert_eq!(
        cases
            .iter()
            .map(|c| c.relative())
            .collect::<BTreeSet<_>>()
            .len(),
        64
    );
    for state in ["button", "list", "table", "input"] {
        assert!(
            cases
                .iter()
                .any(|c| c.relative() == format!("components/{state}-cozy-dark.png"))
        );
    }
    for state in [
        "input · focused empty",
        "input · caret",
        "input · selection",
        "input · preedit",
    ] {
        assert_eq!(cases.iter().filter(|c| c.state == state).count(), 8);
    }
}

// keeps: spec renderer.md §4.5
#[test]
fn transparent_layout_border_does_not_cut_the_background_fill() {
    let mut renderer = ChromeRenderer::new(vec![]).unwrap();
    let ctx = Context::new(Theme::Dark, Density::Cozy);
    let mut style = bbt_ui::Style::new(bbt_ui::Preset::ButtonPrimary);
    style.overrides.height = Some(bbt_ui::Length::Px(30.));
    style.overrides.radii = Some([bbt_ui::Length::Px(0.); 4]);
    style.overrides.border = Some(bbt_ui::Border {
        width: bbt_ui::Length::Px(1.),
        paint: bbt_ui::Paint::solid(bbt_ui::Color::Transparent),
        outside: false,
    });
    let expected = style
        .resolve(ctx)
        .unwrap()
        .fill
        .map(|v| (v * 255.).round() as u8);
    let layout = c::Element::Box(c::Box::new(style, vec![]))
        .layout(Rect::new(20., 20., 80., 0.), ctx, &components::Metrics)
        .unwrap();
    let pixels = components::execute(
        &mut renderer,
        c::paint(&layout, ctx),
        vec![0; 120 * 70 * 4],
        [120, 70],
        ctx,
    )
    .unwrap();
    for y in [20, 21, 48, 49] {
        let n = (y * 120 + 60) * 4;
        assert_eq!(
            pixels[n..n + 4],
            expected,
            "background must reach the transparent border edge"
        );
    }
}

// keeps: spec renderer.md §4.5
#[test]
fn thousand_row_views_clip_output_and_the_table_header_stays_fixed() {
    let mut renderer = ChromeRenderer::new(bboxybook::images().unwrap()).unwrap();
    let ctx = Context::new(Theme::Dark, Density::Cozy);
    let mut bg = DrawList::default();
    bg.rect(Rect::new(0., 0., 640., 280.), 0., Color::hex("#192433"));
    let base = renderer.render_to_rgba(&bg, 640, 280, 1.).unwrap();
    for kind in ["list", "table"] {
        let mut samples = vec![];
        let mut header = None;
        for position in ["top", "middle", "end"] {
            let layout = widgets::preview(kind, ctx, position).unwrap();
            let r = layout.rect;
            let pixels = components::execute(
                &mut renderer,
                c::paint(&layout, ctx),
                base.clone(),
                [640, 280],
                ctx,
            )
            .unwrap();
            let mut changed = 0;
            for y in 0..280 {
                for x in 0..640 {
                    let n = (y * 640 + x) * 4;
                    // A stroke's antialias coverage can extend one pixel outside the Box.
                    let inside = x as f32 >= r.x - 1.
                        && x as f32 <= r.x + r.w + 1.
                        && y as f32 >= r.y - 1.
                        && y as f32 <= r.y + r.h + 1.;
                    if inside {
                        changed += usize::from(pixels[n..n + 4] != base[n..n + 4]);
                    } else {
                        assert_eq!(
                            pixels[n..n + 4],
                            base[n..n + 4],
                            "{kind}/{position} leaked at {x},{y}"
                        );
                    }
                }
            }
            assert!(
                changed > 1000,
                "the viewport must contain real painted rows"
            );
            if kind == "table" {
                let r = layout.children[0].rect;
                let mut crop = vec![];
                for y in r.y.ceil() as usize..(r.y + r.h).floor() as usize {
                    let start = (y * 640 + r.x.ceil() as usize) * 4;
                    let end = (y * 640 + (r.x + r.w).floor() as usize) * 4;
                    crop.extend_from_slice(&pixels[start..end]);
                }
                if let Some(ref first) = header {
                    assert_eq!(first, &crop, "header scrolled with the body");
                } else {
                    header = Some(crop);
                }
            }
            samples.push(pixels);
        }
        assert_ne!(samples[0], samples[1], "middle must paint different rows");
        assert_ne!(samples[1], samples[2], "end must paint different rows");
    }
}
