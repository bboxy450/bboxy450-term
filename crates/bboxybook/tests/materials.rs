use bboxybook::materials as gallery;
use bbt_render::chrome::{ChromeRenderer, materials::*};
use bbt_ui_model::{Theme, draw::Color};
fn render(
    renderer: &mut ChromeRenderer,
    kind: Kind,
    theme: Theme,
    material: Material,
    time_ms: f32,
) -> Vec<u8> {
    let backdrop = gallery::backdrop(kind, theme);
    let items = [Surface {
        material,
        ..gallery::surface(kind, theme)
    }];
    render_to_rgba(
        renderer,
        Scene {
            backdrop: Backdrop::DrawList(&backdrop),
            items: &items,
            width: gallery::SIZE,
            height: gallery::SIZE,
            scale: 1.,
            time_ms,
            theme,
        },
    )
    .unwrap()
}
fn pixel(data: &[u8], x: usize, y: usize) -> [u8; 4] {
    data[(y * gallery::SIZE as usize + x) * 4..][..4]
        .try_into()
        .unwrap()
}
fn contrast(data: &[u8]) -> u64 {
    (100..220)
        .flat_map(|y| (100..220).map(move |x| (x, y)))
        .map(|(x, y)| {
            let a = pixel(data, x, y);
            let b = pixel(data, x + 1, y);
            (0..3).map(|n| a[n].abs_diff(b[n]) as u64).sum::<u64>()
        })
        .sum()
}
#[test]
fn material_flat_gloss_blur_and_glow_are_proven_by_gpu_pixels() {
    let mut renderer = ChromeRenderer::new(vec![]).unwrap();
    for theme in [Theme::Dark, Theme::Light] {
        let flat = render(
            &mut renderer,
            Kind::Flat,
            theme,
            Material::Flat(Color::hex("#4080C0")),
            0.,
        );
        assert_eq!(pixel(&flat, 160, 160), [64, 128, 192, 255]);
        let gloss = render(
            &mut renderer,
            Kind::Gloss,
            theme,
            gallery::material(Kind::Gloss, theme),
            0.,
        );
        let top = pixel(&gloss, 160, 70);
        let middle = pixel(&gloss, 160, 169);
        let bottom = pixel(&gloss, 160, 248);
        assert!(
            top[1] > middle[1] && middle[1] > bottom[1],
            "gloss must have the lab's light top and dark bottom"
        );
        let raw = Frosted {
            tint: Color::WHITE.alpha(0.),
            blur_radius: 0.,
            saturation: 1.,
        };
        let sharp = render(
            &mut renderer,
            Kind::Frosted,
            theme,
            Material::Frosted(raw),
            0.,
        );
        let blurred = render(
            &mut renderer,
            Kind::Frosted,
            theme,
            Material::Frosted(Frosted {
                blur_radius: 13.,
                ..raw
            }),
            0.,
        );
        assert!(
            contrast(&blurred) * 2 < contrast(&sharp),
            "frosted must filter the actual backdrop, not just paint alpha"
        );
        let Material::Glow(glow) = gallery::material(Kind::Glow, theme) else {
            panic!()
        };
        let active = render(&mut renderer, Kind::Glow, theme, Material::Glow(glow), 450.);
        let zero = render(
            &mut renderer,
            Kind::Glow,
            theme,
            Material::Glow(Glow {
                color: glow.color.alpha(0.),
                waiting: glow.waiting.alpha(0.),
                errored: glow.errored.alpha(0.),
                ..glow
            }),
            450.,
        );
        let a = pixel(&active, 52, 160);
        let b = pixel(&zero, 52, 160);
        if theme == Theme::Dark {
            assert!((0..3).any(|i| a[i] > b[i]));
        } else {
            assert!((0..3).all(|i| a[i] <= b[i]));
            assert!((0..3).any(|i| a[i] < b[i]));
        }
        assert_ne!(
            active,
            render(&mut renderer, Kind::Glow, theme, Material::Glow(glow), 0.)
        );
        let idle = Glow {
            activity: Activity::Idle,
            particles: 6,
            ..glow
        };
        assert_eq!(
            render(&mut renderer, Kind::Glow, theme, Material::Glow(idle), 0.),
            render(&mut renderer, Kind::Glow, theme, Material::Glow(idle), 800.)
        );
        let error = Glow {
            activity: Activity::Errored,
            particles: 0,
            ..glow
        };
        assert_eq!(
            render(&mut renderer, Kind::Glow, theme, Material::Glow(error), 0.),
            render(
                &mut renderer,
                Kind::Glow,
                theme,
                Material::Glow(error),
                800.
            )
        );
        let waiting = Glow {
            activity: Activity::Waiting,
            ..glow
        };
        assert_ne!(
            render(
                &mut renderer,
                Kind::Glow,
                theme,
                Material::Glow(waiting),
                0.
            ),
            render(
                &mut renderer,
                Kind::Glow,
                theme,
                Material::Glow(waiting),
                500.
            )
        );
    }
}

#[test]
fn backdrop_blur_mirrors_at_its_border_box_without_outside_colour_bleed() {
    use bbt_ui_model::{draw::DrawList, layout::Rect};
    let mut renderer = ChromeRenderer::new(vec![]).unwrap();
    let mut backdrop = DrawList::default();
    backdrop.rect(Rect::new(0., 0., 80., 80.), 0., Color::hex("#D00000"));
    let rect = Rect::new(20., 20., 40., 40.);
    backdrop.rect(rect, 0., Color::hex("#102080"));
    let items = [Surface {
        rect,
        radii: [0.; 4],
        clips: vec![],
        material: Material::Frosted(Frosted {
            tint: Color::TRANSPARENT,
            blur_radius: 16.,
            saturation: 1.,
        }),
    }];
    let image = render_to_rgba(
        &mut renderer,
        Scene {
            backdrop: Backdrop::DrawList(&backdrop),
            items: &items,
            width: 80,
            height: 80,
            scale: 1.,
            time_ms: 0.,
            theme: Theme::Dark,
        },
    )
    .unwrap();
    for (x, y) in [(21, 30), (58, 30), (30, 21), (30, 58), (40, 40)] {
        let pixel = &image[(y * 80 + x) * 4..][..4];
        for (&actual, expected) in pixel.iter().zip([16, 32, 128, 255]) {
            assert!(
                actual.abs_diff(expected) <= 1,
                "mirror edge leaked outside red: {pixel:?}"
            );
        }
    }
}

#[test]
fn source_over_glow_keeps_destination_alpha() {
    use bbt_ui_model::layout::Rect;
    let mut renderer = ChromeRenderer::new(vec![]).unwrap();
    let pixels: Vec<u8> = (0..80 * 80)
        .flat_map(|i| [16, 32, 64, (i % 200 + 20) as u8])
        .collect();
    let items = [Surface {
        rect: Rect::new(20., 20., 40., 40.),
        radii: [6.; 4],
        clips: vec![],
        material: gallery::material(Kind::Glow, Theme::Dark),
    }];
    let result = render_to_rgba(
        &mut renderer,
        Scene {
            backdrop: Backdrop::Rgba(&pixels),
            items: &items,
            width: 80,
            height: 80,
            scale: 1.,
            time_ms: 450.,
            theme: Theme::Dark,
        },
    )
    .unwrap();
    assert!(
        result
            .as_chunks::<4>().0.iter()
            .zip(pixels.as_chunks::<4>().0)
            .all(|(a, b)| a[3] == b[3])
    );
    assert_ne!(
        result, pixels,
        "the effect must draw colour while preserving alpha"
    );
}
