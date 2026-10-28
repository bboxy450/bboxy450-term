// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use bboxybook::{atoms, gallery};
use bbt_render::chrome::ChromeRenderer;
use std::collections::BTreeSet;

// keeps: spec renderer.md §4.5
#[test]
fn icon_size_gallery_renders_at_one_and_two_device_pixels_per_point() {
    use bbt_ui_model::{
        draw::{Color, DrawList, Prim},
        layout::Rect,
    };
    let mut renderer = ChromeRenderer::new(bboxybook::images().unwrap()).unwrap();
    let mut list = DrawList::default();
    list.rect(Rect::new(0., 0., 560., 224.), 0., Color::BLACK);
    for (row, name) in ["claude", "codex", "pi", "omp", "zsh", "gear", "sidebarfox"]
        .into_iter()
        .enumerate()
    {
        for (column, size) in [12., 14., 16., 18., 20., 24., 32.].into_iter().enumerate() {
            list.push(Prim::Icon {
                rect: Rect::new(8. + column as f32 * 80., row as f32 * 32., size, size),
                icon: name.into(),
                color: Color::WHITE,
            });
        }
    }
    for scale in [1., 2.] {
        let width = (560. * scale) as u32;
        let height = (224. * scale) as u32;
        let image = renderer
            .render_to_rgba(&list, width, height, scale)
            .unwrap();
        assert_eq!(
            image,
            renderer
                .render_to_rgba(&list, width, height, scale)
                .unwrap()
        );
        assert!(image.as_chunks::<4>().0.iter().all(|p| p[3] == 255));
    }
}

// keeps: spec renderer.md §4.5
#[test]
fn every_atom_renders_across_density_and_theme_and_references_have_canonical_keys() {
    let cases = atoms::cases();
    assert_eq!(cases.len(), 112);
    assert_eq!(
        cases
            .iter()
            .map(|c| c.relative())
            .collect::<BTreeSet<_>>()
            .len(),
        112
    );
    assert!(
        cases
            .iter()
            .all(|c| c.fixture.as_ref().unwrap()["layer"] == "atoms")
    );
    let mut renderer = ChromeRenderer::new(bboxybook::images().unwrap()).unwrap();
    for theme in gallery::THEMES {
        for density in gallery::DENSITIES {
            let mut unique = BTreeSet::new();
            for kind in atoms::KINDS {
                let list = atoms::preview(kind, theme, density).unwrap();
                list.validate().unwrap();
                let image = renderer.render_to_rgba(&list, 640, 160, 1.).unwrap();
                assert!(image.as_chunks::<4>().0.iter().all(|p| p[3] == 255));
                let hash = image.iter().fold(0xcbf29ce484222325u64, |hash, &b| {
                    (hash ^ b as u64).wrapping_mul(0x100000001b3)
                });
                unique.insert(hash);
            }
            assert_eq!(
                unique.len(),
                atoms::KINDS.len(),
                "every entry must draw its own atom"
            );
        }
    }
}
