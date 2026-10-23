#![cfg(target_os = "macos")]
#![allow(
    clippy::disallowed_methods,
    reason = "offline Metal/font regression tests, not application UI"
)]
use bbt_render::{
    FontConfig,
    cells::terminal::{Cell, Cursor, CursorShape, Grid, Options, TerminalRenderer, Underline},
    device::DeviceContext,
    text::terminal::{Fonts, Format},
};

#[test]
fn coretext_fallback_styles_and_emoji_have_real_coverage() {
    let fonts = Fonts::new(&FontConfig::default(), None, false).unwrap();
    let regular = fonts.rasterize("F", false, false, 1).unwrap().unwrap();
    let bold = fonts.rasterize("F", true, false, 1).unwrap().unwrap();
    let italic = fonts.rasterize("F", false, true, 1).unwrap().unwrap();
    assert_eq!(regular.format, Format::Mask);
    assert!(regular.pixels.iter().any(|v| *v > 128));
    assert_ne!(regular.pixels, bold.pixels);
    assert_ne!(regular.pixels, italic.pixels);
    assert!(bold.faces.iter().any(|f| f.contains("Bold")));
    assert!(italic.faces.iter().any(|f| f.contains("Italic")));
    let combining = fonts
        .rasterize("e\u{301}", false, false, 1)
        .unwrap()
        .unwrap();
    assert!(combining.pixels.iter().any(|v| *v != 0));
    let cjk = fonts.rasterize("中", false, false, 2).unwrap().unwrap();
    assert!(cjk.faces.iter().all(|f| !f.contains("LastResort")));
    let emoji = fonts
        .rasterize("👩\u{200d}💻", false, false, 2)
        .unwrap()
        .unwrap();
    assert_eq!(emoji.format, Format::Bgra);
    assert!(emoji.faces.iter().any(|f| f == "AppleColorEmoji"));
    assert!(
        emoji
            .pixels
            .as_chunks::<4>()
            .0
            .iter()
            .any(|p| p[3] > 0 && (p[0] != p[1] || p[1] != p[2]))
    );
    assert!(
        emoji
            .pixels
            .as_chunks::<4>()
            .0
            .iter()
            .all(|p| p[0] <= p[3] && p[1] <= p[3] && p[2] <= p[3])
    );
}

#[test]
fn integer_cells_double_with_backing_scale_and_reject_invalid_settings() {
    let one = Fonts::new(&FontConfig::default(), None, false)
        .unwrap()
        .metrics();
    let two = Fonts::new(
        &FontConfig {
            scale: 2,
            ..Default::default()
        },
        None,
        false,
    )
    .unwrap()
    .metrics();
    assert_eq!(one.cell_width * 2, two.cell_width);
    assert_eq!(one.cell_height * 2, two.cell_height);
    assert!(
        Fonts::new(
            &FontConfig {
                size: f32::NAN,
                ..Default::default()
            },
            None,
            false
        )
        .is_err()
    );
    assert!(Fonts::new(&FontConfig::default(), Some(0.), false).is_err());
}

#[test]
fn procedural_edges_join_and_second_frame_reuses_every_glyph() {
    let context = pollster::block_on(DeviceContext::new()).unwrap();
    let mut renderer = TerminalRenderer::new(&context, Options::default()).unwrap();
    let background = [15, 20, 29, 255];
    let foreground = [232, 237, 247, 255];
    let cells = vec![
        Cell {
            text: "─".into(),
            ..Default::default()
        },
        Cell {
            text: "─".into(),
            ..Default::default()
        },
        Cell {
            text: "".into(),
            ..Default::default()
        },
        Cell {
            text: "🦊".into(),
            width: 2,
            ..Default::default()
        },
        Cell {
            text: String::new(),
            width: 0,
            ..Default::default()
        },
    ];
    let grid = Grid {
        columns: 5,
        rows: 1,
        cells,
        cursor: None,
    };
    let first = renderer.render_to_rgba(&context, &grid).unwrap();
    let n = renderer.cached_glyphs();
    let second = renderer.render_to_rgba(&context, &grid).unwrap();
    assert_eq!(n, renderer.cached_glyphs());
    assert_eq!(first.rgba, second.rgba);
    assert!(renderer.effective_faces().any(|f| f == "AppleColorEmoji"));
    let metrics = renderer.metrics();
    let y = metrics.cell_height / 2;
    for x in 0..2 * metrics.cell_width {
        assert_eq!(
            &first.rgba
                [((y * first.width + x) * 4) as usize..((y * first.width + x) * 4 + 4) as usize],
            &foreground,
            "sprite seam at {x}"
        );
    }
    assert_eq!(&first.rgba[..4], &background);
}

#[test]
fn cursor_shapes_and_bad_snapshots_are_observable() {
    let context = pollster::block_on(DeviceContext::new()).unwrap();
    let mut renderer = TerminalRenderer::new(&context, Options::default()).unwrap();
    let mut grid = Grid {
        columns: 2,
        rows: 1,
        cells: vec![Cell::default(); 2],
        cursor: None,
    };
    let empty = renderer.render_to_rgba(&context, &grid).unwrap().rgba;
    let mut counts = Vec::new();
    for shape in [CursorShape::Block, CursorShape::Bar, CursorShape::Underline] {
        grid.cursor = Some(Cursor {
            column: 0,
            row: 0,
            shape,
            focused: true,
            color: [255; 4],
        });
        let image = renderer.render_to_rgba(&context, &grid).unwrap();
        counts.push(
            image
                .rgba
                .as_chunks::<4>()
                .0
                .iter()
                .zip(empty.as_chunks::<4>().0)
                .filter(|(a, b)| a != b)
                .count(),
        );
    }
    assert!(counts[0] > counts[1] && counts[0] > counts[2]);
    assert_ne!(counts[1], counts[2]);
    let metrics = renderer.metrics();
    // The focused bar and underline are one logical pixel, as in the reference.
    assert_eq!(
        counts,
        [
            metrics.cell_width * metrics.cell_height,
            metrics.cell_height,
            metrics.cell_width
        ]
        .map(|n| n as usize)
    );
    grid.cursor.as_mut().unwrap().column = 2;
    assert!(renderer.render_to_rgba(&context, &grid).is_err());
}

#[test]
fn patterned_underlines_continue_across_cells_and_restart_after_a_gap() {
    let context = pollster::block_on(DeviceContext::new()).unwrap();
    for scale in [1, 2] {
        let mut renderer = TerminalRenderer::new(
            &context,
            Options {
                font: FontConfig {
                    scale,
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();
        let metrics = renderer.metrics();
        for (style, period, on) in [(Underline::Dotted, 2, 1), (Underline::Dashed, 5, 3)] {
            for (foreground, background) in [
                ([232, 237, 247, 255], [15, 20, 29, 255]),
                ([23, 31, 43, 255], [242, 244, 248, 255]),
            ] {
                let grid = Grid {
                    columns: 8,
                    rows: 1,
                    cells: (0..8)
                        .map(|col| Cell {
                            foreground,
                            background,
                            underline_color: foreground,
                            underline: if matches!(col, 1 | 2 | 4..=6) {
                                style
                            } else {
                                Underline::None
                            },
                            ..Default::default()
                        })
                        .collect(),
                    cursor: None,
                };
                let image = renderer.render_to_rgba(&context, &grid).unwrap();
                let y = metrics.cell_height - 2 * scale;
                for x in 0..image.width {
                    let start = match x / metrics.cell_width {
                        1 | 2 => Some(metrics.cell_width),
                        4..=6 => Some(4 * metrics.cell_width),
                        _ => None,
                    };
                    let ink = start.is_some_and(|start| ((x - start) / scale) % period < on);
                    let offset = ((y * image.width + x) * 4) as usize;
                    assert_eq!(
                        &image.rgba[offset..offset + 4],
                        if ink { &foreground } else { &background },
                        "{style:?}, {scale}x, x={x}"
                    );
                }
            }
        }
    }
}

#[test]
fn metal_grayscale_blending_preserves_coverage_in_both_polarities() {
    let context = pollster::block_on(DeviceContext::new()).unwrap();
    let options = Options::default();
    let fonts = Fonts::new(&options.font, options.line_height, options.thicken).unwrap();
    let glyph = fonts.rasterize("F", false, false, 1).unwrap().unwrap();
    let mut renderer = TerminalRenderer::new(&context, options).unwrap();
    for (foreground, background) in [(255, 0), (0, 255)] {
        let grid = Grid {
            columns: 1,
            rows: 1,
            cells: vec![Cell {
                text: "F".into(),
                foreground: [foreground, foreground, foreground, 255],
                background: [background, background, background, 255],
                ..Default::default()
            }],
            cursor: None,
        };
        let image = renderer.render_to_rgba(&context, &grid).unwrap();
        let mut partial_pixels = 0;
        for (i, coverage) in glyph.pixels.iter().copied().enumerate() {
            let x = (i as u32 % glyph.width) as i32 + i32::from(glyph.left);
            let y = (i as u32 / glyph.width) as i32 + i32::from(glyph.top);
            if !(16..240).contains(&coverage)
                || x < 0
                || x >= image.width as i32
                || y < 0
                || y >= image.height as i32
            {
                continue;
            }
            let offset = ((y as u32 * image.width + x as u32) * 4) as usize;
            let pixel = &image.rgba[offset..offset + 4];
            let expected = if background == 0 {
                coverage
            } else {
                255 - coverage
            };
            // A plain linear alpha blend would brighten white-on-black edges;
            // the luminance remap preserves gamma-space coverage instead.
            assert!((i16::from(pixel[0]) - i16::from(expected)).abs() <= 2);
            assert_eq!(pixel[0], pixel[1]);
            assert_eq!(pixel[1], pixel[2]);
            assert_eq!(pixel[3], 255);
            partial_pixels += 1;
        }
        assert!(partial_pixels > 0);
    }
}
