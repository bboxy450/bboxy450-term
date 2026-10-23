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
