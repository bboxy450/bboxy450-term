// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
#![cfg(target_os = "macos")]
#![allow(
    clippy::disallowed_methods,
    reason = "private headless pixel capture fixture"
)]
use bbt_render::chrome::{ChromeRenderer, icons::Assets};
use bbt_ui_model::{
    draw::{Color, DrawList, TextStyle, TypeScale},
    layout::Rect,
};

fn chromatic(p: &[u8; 4]) -> bool {
    p[3] > 32 && p[..3].iter().max().unwrap() - p[..3].iter().min().unwrap() > 24
}

// keeps: renderer native color glyph contract — every UI text primitive uses native emoji.
#[test]
fn native_color_emoji_survive_ui_styles_tints_scales_and_retained_atlas_reuse() {
    let mut renderer = ChromeRenderer::new(Assets::default()).unwrap();
    let clusters = ["🦊", "👩🏽‍💻", "👨‍👩‍👧‍👦", "🇺🇦", "❤️", "1️⃣", "🏳️‍🌈"];
    for scale in [1., 2.] {
        for (index, cluster) in clusters.iter().enumerate() {
            let style = TextStyle {
                scale: TypeScale::Custom(22.),
                weight: 600,
                mono: index % 2 == 0,
                uppercase: index % 3 == 0,
                ..Default::default()
            };
            let draw = |color| {
                let mut list = DrawList::default();
                list.text(Rect::new(8., 8., 64., 36.), *cluster, style, color);
                list
            };
            let width = (80. * scale) as u32;
            let height = (52. * scale) as u32;
            let white = renderer
                .render_to_rgba(&draw(Color::WHITE), width, height, scale)
                .unwrap();
            let tinted = renderer
                .render_to_rgba(&draw(Color([1., 0., 1., 1.])), width, height, scale)
                .unwrap();
            assert_eq!(
                white, tinted,
                "{cluster} {scale}x: foreground must not recolor emoji"
            );
            assert!(
                white
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .filter(|p| chromatic(p))
                    .count()
                    > 16,
                "{cluster} {scale}x: actual colored pixels required"
            );
            assert!(
                white
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .all(|p| p[..3].iter().all(|v| *v <= p[3])),
                "premultiplied output"
            );
            let repeated = renderer
                .render_to_rgba(&draw(Color::WHITE), width, height, scale)
                .unwrap();
            assert_eq!(white, repeated);
            let half = renderer
                .render_to_rgba(&draw(Color([0., 1., 0., 0.5])), width, height, scale)
                .unwrap();
            assert!(half.as_chunks::<4>().0.iter().all(|p| p[3] <= 128));
            assert!(half.as_chunks::<4>().0.iter().any(chromatic));
        }
    }
    assert!(renderer.text_cache_stats().coverage.color_entries >= clusters.len());
    assert_eq!(renderer.text_cache_stats().whole_line_masks, 0);
}

// keeps: native color everywhere — headless title/sidebar/menu/chat/input text capture.
#[test]
fn mixed_ui_text_gallery_has_colored_pixels_on_each_surface_and_real_alpha_shadows() {
    use bbt_ui_model::draw::{Prim, TextShadow};
    let mut renderer = ChromeRenderer::new(Assets::default()).unwrap();
    let mut list = DrawList::default();
    let labels = [
        "Titlebar 🦊 Swarm",
        "Sidebar 👩🏽‍💻 Builders",
        "Menu 🇺🇦 Rename",
        "Chat 👨‍👩‍👧‍👦 Hello",
        "Input ❤️ 1️⃣ 🏳️‍🌈",
    ];
    for (row, label) in labels.into_iter().enumerate() {
        let y = row as f32 * 44.;
        list.rect(
            Rect::new(0., y, 340., 44.),
            0.,
            Color([0.04, 0.04, 0.04, 1.]),
        );
        let style = TextStyle {
            scale: TypeScale::Custom(20.),
            weight: if row < 3 { 600 } else { 400 },
            mono: row == 4,
            ..Default::default()
        };
        list.text(
            Rect::new(10., y + 5., 320., 36.),
            label,
            style,
            Color::WHITE,
        );
    }
    let width = 680;
    let height = 440;
    let pixels = renderer.render_to_rgba(&list, width, height, 2.).unwrap();
    for (row, label) in labels.iter().enumerate() {
        let start = row * 88 * width as usize * 4;
        let end = start + 88 * width as usize * 4;
        assert!(
            pixels[start..end]
                .as_chunks::<4>()
                .0
                .iter()
                .filter(|p| chromatic(p))
                .count()
                > 100,
            "surface {} has native emoji pixels",
            label
        );
    }
    if let Some(root) = std::env::var_os("BBOXY_EMOJI_EVIDENCE") {
        let root = std::path::PathBuf::from(root);
        std::fs::create_dir_all(&root).unwrap();
        let file = std::fs::File::create(root.join("native-color-emoji-ui.png")).unwrap();
        let mut encoder = png::Encoder::new(file, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .unwrap()
            .write_image_data(&pixels)
            .unwrap();
    }
    for blur_radius in [0., 4.] {
        let mut shadow = DrawList::default();
        shadow.push(Prim::TextShadow {
            origin: [8., 8.],
            baseline: None,
            text: "🦊".into(),
            style: TextStyle {
                scale: TypeScale::Custom(22.),
                ..Default::default()
            },
            max_width: None,
            shadow: TextShadow {
                offset: [0., 0.],
                blur_radius,
                color: Color([0., 1., 0., 1.]),
            },
        });
        let pixels = renderer.render_to_rgba(&shadow, 80, 52, 1.).unwrap();
        assert!(pixels.as_chunks::<4>().0.iter().any(|p| p[1] > 16));
        assert!(
            pixels
                .as_chunks::<4>()
                .0
                .iter()
                .all(|p| p[0] == 0 && p[2] == 0),
            "emoji shadow is coverage colored by shadow style"
        );
    }
}
