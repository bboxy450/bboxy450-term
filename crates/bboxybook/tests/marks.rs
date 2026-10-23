use bboxybook::components::Metrics;
use bbt_render::chrome::ChromeRenderer;
use bbt_ui::{Context, Density, Paint, Theme, components as c, marks::Mark};
use bbt_ui_model::{
    Activity, WaitKind,
    draw::{Color, DrawList},
    layout::Rect,
};

#[test]
#[allow(
    clippy::disallowed_methods,
    reason = "headless mark-centering proof writes local verification images"
)]
fn seat_and_task_marks_are_pixel_centred_at_96px_in_both_painters() {
    // All semantic cases, including the check/cross that a circle-only probe misses.
    let cases = [
        ("seat idle", Mark::seat(Activity::Idle), Some([false; 4])),
        ("seat active", Mark::seat(Activity::Active), Some([true; 4])),
        (
            "seat waiting",
            Mark::seat(Activity::Waiting(WaitKind::Question)),
            Some([true, false, true, false]),
        ),
        ("seat errored", Mark::seat(Activity::Errored), None),
        ("task open", Mark::task("open"), Some([false; 4])),
        (
            "task claimed",
            Mark::task("claimed"),
            Some([false, true, false, false]),
        ),
        (
            "task running",
            Mark::task("running"),
            Some([false, true, false, true]),
        ),
        (
            "task reported",
            Mark::task("reported"),
            Some([false, true, true, true]),
        ),
        ("task accepted", Mark::task("accepted"), Some([true; 4])),
        ("task done", Mark::task("done"), None),
        ("task rejected", Mark::task("rejected"), None),
        ("task blocked", Mark::task("blocked"), None),
    ];
    let ctx = Context::new(Theme::Dark, Density::Cozy);
    let mut renderer = ChromeRenderer::new(vec![]).unwrap();
    let out = std::env::var_os("CARGO_TARGET_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target"))
        .join("mk1-proof");
    std::fs::create_dir_all(&out).unwrap();
    let mut records = vec![];
    for retained in [false, true] {
        let mut list = DrawList::default();
        list.rect(Rect::new(0., 0., 512., 384.), 0., Color::BLACK);
        for (i, (_, mark, _)) in cases.iter().enumerate() {
            let rect = Rect::new(
                16. + (i % 4) as f32 * 128.,
                16. + (i / 4) as f32 * 128.,
                96.,
                96.,
            );
            if retained {
                let layout = mark
                    .element(96., Paint::solid(bbt_ui::Color::Rgba(Color::WHITE.0)))
                    .layout(rect, ctx, &Metrics)
                    .unwrap();
                assert_eq!(layout.rect, rect);
                for paint in c::paint(&layout, ctx) {
                    match paint {
                        c::Paint::Draw(draw) => list.primitives.extend(draw.primitives),
                        _ => panic!("a mark needs only the existing vector path painter"),
                    }
                }
            } else {
                mark.paint(rect, Color::WHITE, &mut list);
            }
        }
        for scale in [1_u32, 2] {
            let (width, height) = (512 * scale, 384 * scale);
            let pixels = renderer
                .render_to_rgba(&list, width, height, scale as f32)
                .unwrap();
            for (i, (name, _, quadrants)) in cases.iter().enumerate() {
                let left = (16 + (i as u32 % 4) * 128) * scale;
                let top = (16 + (i as u32 / 4) * 128) * scale;
                let size = 96 * scale;
                let ink = |x: u32, y: u32| {
                    let at = (((top + y) * width + left + x) * 4) as usize;
                    pixels[at..at + 3]
                        .iter()
                        .map(|v| u32::from(*v))
                        .sum::<u32>()
                        > 90
                };
                let (mut x0, mut y0, mut x1, mut y1) = (size, size, 0, 0);
                for y in 3 * scale..size - 3 * scale {
                    for x in 3 * scale..size - 3 * scale {
                        if ink(x, y) {
                            x0 = x0.min(x);
                            y0 = y0.min(y);
                            x1 = x1.max(x + 1);
                            y1 = y1.max(y + 1);
                        }
                    }
                }
                assert!(x0 < x1 && y0 < y1, "{name} has ink");
                assert_eq!(
                    (x0 + x1, y0 + y1),
                    (size, size),
                    "{name}, {scale}x, retained={retained}"
                );
                if let Some(expected) = quadrants {
                    let actual = [(30, 30), (66, 30), (30, 66), (66, 66)]
                        .map(|(x, y)| ink(x * scale, y * scale));
                    assert_eq!(actual, *expected, "{name} fill orientation");
                }
                records.push(
                    serde_json::json!({"mark":name,"retained":retained,"scale":scale,
                    "ink_bounds":[x0,y0,x1,y1],"offset_px":[0,0]}),
                );
            }
            bbt_render::write_png(
                out.join(format!(
                    "native-{}-{scale}x.png",
                    if retained { "retained" } else { "atom" }
                )),
                &pixels,
                width,
                height,
            )
            .unwrap();
        }
    }
    std::fs::write(
        out.join("centering.json"),
        serde_json::to_vec_pretty(&records).unwrap(),
    )
    .unwrap();
    println!(
        "12 semantic marks x 2 painters x 2 scales: centred; {}",
        out.display()
    );
}

#[test]
#[allow(
    clippy::disallowed_methods,
    reason = "headless coverage-cache and alpha regression"
)]
fn vector_shadow_reuses_tint_translation_and_bounds_alpha_before_blur() {
    use bbt_ui_model::draw::TextShadow;
    let mut renderer = ChromeRenderer::new(vec![]).unwrap();
    let mut list = DrawList::default();
    list.rect(Rect::new(0., 0., 128., 128.), 0., Color::BLACK);
    Mark::LeftHalf.paint(
        Rect::new(16., 16., 96., 96.),
        Color::WHITE.alpha(0.72),
        &mut list,
    );
    let pixels = renderer.render_to_rgba(&list, 128, 128, 1.).unwrap();
    let pixel = |x: usize, y: usize| &pixels[(y * 128 + x) * 4..(y * 128 + x) * 4 + 3];
    assert_eq!(
        pixel(29, 64),
        pixel(46, 64),
        "ring/pie overlap gets alpha once"
    );
    assert!(pixel(46, 64)[0] > 0);
    assert_eq!(pixel(82, 64), [0, 0, 0], "right half stays open");

    let draw = |x: f32, size: f32, tint: Color, offset: [f32; 2], alpha: f32| {
        let mut list = DrawList::default();
        list.rect(Rect::new(0., 0., 128., 96.), 0., Color::BLACK);
        Mark::LeftHalf.paint_with_shadow(
            Rect::new(x, 32.5, size, size),
            Color::WHITE,
            Some(TextShadow {
                color: tint.alpha(alpha),
                blur_radius: 6.,
                offset,
            }),
            &mut list,
        );
        list
    };
    let amber = Color::hex("#FFB454");
    let first = draw(20.25, 12., amber, [0.25, 0.5], 0.72);
    bbt_render::passes::take();
    renderer.render_to_rgba(&first, 128, 96, 1.).unwrap();
    let with_shadow = bbt_render::passes::take();
    let mut plain = first.clone();
    plain
        .primitives
        .retain(|p| !matches!(p, bbt_ui_model::draw::Prim::PathShadow { .. }));
    renderer.render_to_rgba(&plain, 128, 96, 1.).unwrap();
    let without_shadow = bbt_render::passes::take();
    assert_eq!(with_shadow.blur, 0, "the Gaussian adds no GPU blur pass");
    assert_eq!(
        with_shadow.passes(),
        without_shadow.passes(),
        "the shadow stays in the existing flat batch"
    );
    println!("mark scene passes: shadow={with_shadow:?}; plain={without_shadow:?}");
    let a = renderer.text_cache_stats().coverage;
    assert_eq!(a.shadow_blurs, 1);
    let again = draw(28.25, 12., Color::hex("#2EE6A8"), [0.25, 0.5], 0.4);
    renderer.render_to_rgba(&again, 128, 96, 1.).unwrap();
    let b = renderer.text_cache_stats().coverage;
    assert_eq!((a.misses, a.shadow_blurs), (b.misses, b.shadow_blurs));
    assert!(
        b.hits > a.hits,
        "tint, alpha and integer translation reuse coverage"
    );
    for variant in [
        draw(20.25, 18., amber, [0.25, 0.5], 0.72),
        draw(20.25, 12., amber, [0.5, 0.5], 0.72),
        draw(20.25, 12., amber, [0.25, 0.75], 0.72),
    ] {
        let before = renderer.text_cache_stats().coverage.misses;
        renderer.render_to_rgba(&variant, 128, 96, 1.).unwrap();
        assert!(
            renderer.text_cache_stats().coverage.misses > before,
            "scale and both phases bind coverage"
        );
    }
    let before = renderer.text_cache_stats().coverage.misses;
    renderer.render_to_rgba(&first, 256, 192, 2.).unwrap();
    assert!(
        renderer.text_cache_stats().coverage.misses > before,
        "DPR binds coverage"
    );
    let before = renderer.text_cache_stats().coverage.shadow_requests;
    let invisible = draw(20.25, 20., amber, [0., 0.], 0.);
    renderer.render_to_rgba(&invisible, 128, 96, 1.).unwrap();
    assert_eq!(
        renderer.text_cache_stats().coverage.shadow_requests,
        before,
        "transparent shadows skip the cache"
    );
}
