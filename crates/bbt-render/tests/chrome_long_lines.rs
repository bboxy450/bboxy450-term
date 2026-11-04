// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
#![cfg(target_os = "macos")]
use bbt_render::chrome::{ChromeRenderer, icons::Assets, text};
use bbt_ui_model::{
    draw::{Color, DrawList, Prim, TextStyle, TypeScale},
    layout::Rect,
};

// keeps: rich-text-safety — full shaping is independent of bitmap canvas limits.
#[test]
fn rich_text_safety_long_line_scroll_preserves_native_tail_and_cache_bounds() {
    let mut renderer = ChromeRenderer::new(Assets::default()).unwrap();
    let style = TextStyle {
        mono: true,
        scale: TypeScale::Custom(12.),
        ..Default::default()
    };
    let prefix = "x".repeat(4096);
    let tail = "TAIL_e\u{301}_👩🏽‍💻_مرحبا_許_AVfi";
    let full = format!("{prefix}{tail}");
    let advance = text::measure(&prefix, style, None).unwrap().advance;
    let all = text::measure(&full, style, None).unwrap();
    assert!(
        all.advance > 16_384.,
        "legacy line-canvas control must reject this input"
    );
    // The reference bitmap API must keep its allocation guard. The production
    // renderer uses the existing bounded native glyph/component rasterizer.
    assert!(text::rasterize(&full, style, None, 1.).is_err());
    for scale in [1., 1.5, 2., 4.] {
        for phase in [0., 0.25, 0.5, 0.75] {
            let draw = |value: &str, x: f32| {
                let mut list = DrawList::default();
                list.clip(Rect::new(12., 0., 300., 52.), [0.; 4]);
                list.push(Prim::Text {
                    origin: [x, 0.],
                    baseline: Some(30.),
                    text: value.into(),
                    style,
                    max_width: None,
                    color: Color::WHITE,
                });
                list.unclip();
                list
            };
            let origin = 16. + phase / scale;
            let width = (324. * scale) as u32;
            let height = (52. * scale) as u32;
            // Cut four pixels into the tail to exclude the final prefix glyph;
            // comparisons include exact native clusters, bidi and color fallback.
            let actual = renderer
                .render_to_rgba(&draw(&full, origin - advance), width, height, scale)
                .unwrap();
            let expected = renderer
                .render_to_rgba(&draw(tail, origin), width, height, scale)
                .unwrap();
            let left = (20. * scale) as usize;
            for row in 0..height as usize {
                let range = (row * width as usize + left) * 4..(row + 1) * width as usize * 4;
                assert_eq!(
                    actual[range.clone()],
                    expected[range],
                    "native tail changed at {scale}x phase {phase}"
                );
            }
            assert!(actual.iter().any(|n| *n != 0), "tail must paint");
            let stats = renderer.text_cache_stats();
            assert_eq!(stats.whole_line_masks, 0);
            assert!(stats.coverage.cached_mask_bytes <= 32 * 1024 * 1024);
            assert!(stats.coverage.glyph_entries <= 32_768);
            assert!(stats.coverage.line_plan_bytes <= 2 * 1024 * 1024);
        }
    }
}
