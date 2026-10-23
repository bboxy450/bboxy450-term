use bboxybook::{atoms, gallery};
use bbt_render::chrome::ChromeRenderer;
use std::collections::BTreeSet;

#[test]
#[allow(
    clippy::disallowed_methods,
    reason = "headless device-phase probe writes local verification images"
)]
fn icon_phase_probe_compares_snapped_origins_and_baked_fractional_coverage() {
    use bbt_render::chrome::icons::{Assets, IconAtlas, Image};
    use bbt_ui_model::{
        draw::{Color, DrawList, Prim},
        layout::Rect,
    };
    let source = bboxybook::BAKED_ICONS
        .iter()
        .find(|(name, _)| *name == "codex")
        .unwrap()
        .1;
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/f1/icon-sizes");
    std::fs::create_dir_all(&out).unwrap();
    for scale in [1., 2.] {
        let size = (16. * scale) as u32;
        let mut images = Vec::new();
        for (column, phase) in [0., 0.25, 0.5, 0.75].into_iter().enumerate() {
            // Shift the SVG viewBox to produce the competing baked-phase
            // coverage, independently of the production placement rule.
            let offset = -phase * 24. / (16. * scale);
            let shifted = source.replace(
                "viewBox=\"0 0 24 24\"",
                &format!("viewBox=\"{offset} {offset} 24 24\""),
            );
            for (mode, source) in [("baked", shifted.as_str()), ("zero", source)] {
                let mut atlas =
                    IconAtlas::new(Assets::with_svgs(vec![], &[("probe", source)])).unwrap();
                let uv = atlas.raster("probe", [size as f32; 2], scale).unwrap();
                let mut rgba = Vec::new();
                for y in 0..size {
                    for x in 0..size {
                        let at =
                            (((uv[1] as u32 + y) * atlas.size + uv[0] as u32 + x) * 4) as usize;
                        // The fixture is white, so its straight RGB is white.
                        rgba.extend_from_slice(&[255, 255, 255, atlas.pixels[at + 3]]);
                    }
                }
                images.push(Image {
                    name: format!("{mode}-{column}"),
                    width: size,
                    height: size,
                    rgba,
                });
            }
        }
        let mut renderer = ChromeRenderer::new(images).unwrap();
        for mode in ["device-snapped", "css-snapped", "baked"] {
            let mut list = DrawList::default();
            list.rect(Rect::new(0., 0., 320., 40.), 0., Color::BLACK);
            for (column, phase) in [0_f32, 0.25, 0.5, 0.75].into_iter().enumerate() {
                let offset = match mode {
                    "device-snapped" => phase.round() / scale,
                    "css-snapped" => (phase / scale).round(),
                    _ => phase.floor() / scale,
                };
                list.push(Prim::Image {
                    rect: Rect::new(8. + column as f32 * 80. + offset, 8. + offset, 16., 16.),
                    image: format!(
                        "{}-{column}",
                        if mode == "baked" { "baked" } else { "zero" }
                    )
                    .into(),
                    radius: 0.,
                });
            }
            let (width, height) = ((320. * scale) as u32, (40. * scale) as u32);
            let pixels = renderer
                .render_to_rgba(&list, width, height, scale)
                .unwrap();
            bbt_render::write_png(
                out.join(format!("phase-{mode}-{}x.png", scale as u32)),
                &pixels,
                width,
                height,
            )
            .unwrap();
        }
    }
}

#[test]
#[allow(
    clippy::disallowed_methods,
    reason = "headless GPU gallery test writes local verification images"
)]
fn icon_size_gallery_renders_at_one_and_two_device_pixels_per_point() {
    use bbt_ui_model::{
        draw::{Color, DrawList, Prim},
        layout::Rect,
    };
    let mut renderer = ChromeRenderer::new(bboxybook::images().unwrap()).unwrap();
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/f1/icon-sizes");
    std::fs::create_dir_all(&out).unwrap();
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
        bbt_render::write_png(
            out.join(format!("native-{}x.png", scale as u32)),
            &image,
            width,
            height,
        )
        .unwrap();
    }
}

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
