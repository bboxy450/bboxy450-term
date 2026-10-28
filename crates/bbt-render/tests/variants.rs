// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
#![allow(
    clippy::disallowed_methods,
    reason = "test/fixture IO, not the UI thread"
)]
#![cfg(target_os = "macos")]
use bbt_render::{FontConfig, HeadlessRenderer, Scene, text::Font, write_png};

// keeps: spec renderer §4.1/§4.2/§4.3 — native headless terminal pixels and font/size/scale/theme matrix
#[test]
fn generate_smoke_font_size_scale_theme_matrix() {
    let crate_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    bbt_render::text::register_font_file(&crate_dir.join("tests/fonts/JetBrainsMono-Regular.ttf"))
        .unwrap();
    let sf_mono = std::path::Path::new(
        "/System/Applications/Utilities/Terminal.app/Contents/Resources/Fonts/SFMono-Terminal.ttf",
    );
    if sf_mono.is_file() {
        bbt_render::text::register_font_file(sf_mono).unwrap();
    }
    let renderer =
        HeadlessRenderer::new().expect("headless Metal is required; do not silently skip");
    eprintln!("render adapter: {:?}", renderer.adapter_info());
    let output = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/render/smoke");
    std::fs::create_dir_all(&output).unwrap();
    let cases = [
        ("jetbrains-mono", "JetBrains Mono"),
        ("sf-mono", "SF Mono"),
        ("menlo", "Menlo"),
        // Matrix slot reserved now; actual CJK/emoji shaping/colour coverage is later work.
        ("fallback-pending", "Menlo"),
    ];
    let mut manifest = String::from("variant\trequested_font\teffective_font\tfixture\n");
    let mut count = 0;
    for (name, family) in cases {
        for size in [11, 13, 16] {
            for scale in [1, 2] {
                for light in [false, true] {
                    let font = FontConfig {
                        family: family.into(),
                        size: size as f32,
                        scale,
                    };
                    let effective = Font::new(&font).unwrap().effective_name();
                    let mut scene = Scene::smoke(font, light).unwrap();
                    if name == "fallback-pending" {
                        scene.text[0].text = "bboxy450-term [CJK/emoji pending]".into();
                    }
                    let [w, h] = scene.pixel_size();
                    let rgba = renderer.render_to_rgba(&scene, w, h).unwrap();
                    assert_eq!(rgba.len(), w as usize * h as usize * 4);
                    let variant = format!(
                        "{name}-{size}px-{scale}x-{}",
                        if light { "light" } else { "dark" }
                    );
                    write_png(output.join(format!("{variant}.png")), &rgba, w, h).unwrap();
                    manifest.push_str(&format!(
                        "{variant}\t{family}\t{effective}\t{}\n",
                        if name == "fallback-pending" {
                            "ASCII only; CJK/emoji deferred"
                        } else {
                            "ASCII bboxy450-term"
                        }
                    ));
                    count += 1;
                }
            }
        }
    }
    std::fs::write(output.join("manifest.tsv"), manifest).unwrap();
    assert_eq!(count, 48);
    eprintln!("rendered {count} PNGs to {}", output.display());
}
