// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
#![cfg(target_os = "macos")]
use bbt_render::{FontConfig, Scene, render_to_rgba, write_png};

fn pixel(rgba: &[u8], width: u32, x: u32, y: u32) -> &[u8] {
    let i = ((y * width + x) * 4) as usize;
    &rgba[i..i + 4]
}

// keeps: spec renderer §4.1/§4.2/§4.3 — native headless terminal pixels and font/size/scale/theme matrix
#[test]
fn metal_quad_and_80_by_24_cell_backgrounds() {
    // 67*4 is not 256-byte aligned: every readback row must lose its GPU padding.
    let solid = Scene::solid([17, 83, 201, 255], 67, 31);
    let rgba = render_to_rgba(&solid, 67, 31).unwrap();
    assert_eq!(rgba.len(), 67 * 31 * 4);
    assert!(
        rgba.as_chunks::<4>()
            .0
            .iter()
            .all(|p| *p == [17, 83, 201, 255])
    );
    write_png(output("quad.png"), &rgba, 67, 31).unwrap();
    let mut scene = Scene::smoke(FontConfig::default(), false).unwrap();
    scene.text.clear();
    let [w, h] = scene.pixel_size();
    let rgba = render_to_rgba(&scene, w, h).unwrap();
    for (col, row) in [(0, 0), (2, 6), (17, 11), (77, 17), (79, 23)] {
        assert_eq!(
            pixel(
                &rgba,
                w,
                col * scene.cell_size[0] + 2,
                row * scene.cell_size[1] + 2
            ),
            scene.backgrounds[(row * 80 + col) as usize]
        );
    }
    write_png(output("backgrounds.png"), &rgba, w, h).unwrap();
}

// keeps: spec renderer §4.1/§4.2/§4.3 — native headless terminal pixels and font/size/scale/theme matrix
#[test]
fn coretext_ink_lands_only_in_expected_ascii_cells() {
    let scene = Scene::smoke(FontConfig::default(), false).unwrap();
    let [w, h] = scene.pixel_size();
    let rendered = render_to_rgba(&scene, w, h).unwrap();
    let mut empty = scene.clone();
    empty.text.clear();
    let reference = render_to_rgba(&empty, w, h).unwrap();
    let word = &scene.text[0];
    let mut ink = vec![0; word.text.len()];
    for y in 0..h {
        for x in 0..w {
            if pixel(&rendered, w, x, y) != pixel(&reference, w, x, y) {
                let col = x / scene.cell_size[0];
                let row = y / scene.cell_size[1];
                assert_eq!(
                    row,
                    u32::from(word.row),
                    "ink outside fixture row at {x},{y}"
                );
                assert!(
                    (u32::from(word.column)..u32::from(word.column) + word.text.len() as u32)
                        .contains(&col),
                    "ink outside fixture cells at {x},{y}"
                );
                ink[(col - u32::from(word.column)) as usize] += 1;
            }
        }
    }
    assert!(ink.iter().all(|n| *n > 0), "missing cell ink: {ink:?}");
    write_png(output("text.png"), &rendered, w, h).unwrap();
    assert!(render_to_rgba(&scene, 0, h).is_err());
}

fn output(name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/render")
        .join(name)
}

// keeps: spec renderer §4.1/§4.2/§4.3 — native headless terminal pixels and font/size/scale/theme matrix
#[test]
fn asymmetric_glyph_is_upright() {
    let font = bbt_render::text::Font::new(&FontConfig {
        size: 32.0,
        ..Default::default()
    })
    .unwrap();
    let glyph = font.rasterize('F').unwrap().unwrap();
    let mut top_right = 0u32;
    let mut bottom_right = 0u32;
    for y in 0..glyph.height {
        for x in glyph.width / 2..glyph.width {
            let value = u32::from(glyph.mask[(y * glyph.width + x) as usize]);
            if y < glyph.height / 2 {
                top_right += value;
            } else {
                bottom_right += value;
            }
        }
    }
    assert!(
        top_right > bottom_right * 2,
        "the F's upper bars must be above its stem: {top_right}/{bottom_right}"
    );
}
