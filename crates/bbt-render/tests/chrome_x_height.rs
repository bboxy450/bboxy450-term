// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
#![cfg(target_os = "macos")]
use bbt_render::chrome::text::x_height;
use bbt_ui_model::draw::{TextStyle, TypeScale};
// keeps: spec renderer §1a — logical CoreText x-height and invalid-size refusal
#[test]
fn coretext_x_height_uses_the_requested_logical_font_and_rejects_invalid_size() {
    for mono in [false, true] {
        for italic in [false, true] {
            for weight in [400, 500, 600, 700] {
                let style = TextStyle {
                    scale: TypeScale::Custom(12.),
                    mono,
                    italic,
                    weight,
                    ..Default::default()
                };
                let x = x_height(style).unwrap();
                assert!(x > 4. && x < 9.);
                let bigger = x_height(TextStyle {
                    scale: TypeScale::Custom(24.),
                    ..style
                })
                .unwrap();
                assert!(bigger > x * 1.5);
            }
        }
    }
    assert!(
        x_height(TextStyle {
            scale: TypeScale::Custom(f32::NAN),
            ..Default::default()
        })
        .is_err()
    );
}
