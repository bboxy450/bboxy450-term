use bboxybook::styles;
use bbt_render::chrome::ChromeRenderer;
use bbt_ui::{Color, Context, Density, Overrides, Paint, Preset, Style, Theme};
use std::collections::BTreeSet;

#[test]
fn per_node_glow_drives_state_grade_and_seed_without_changing_the_box() {
    use bbt_ui::{Glow, GlowActivity, Material};
    let mut renderer = ChromeRenderer::new(vec![]).unwrap();
    let ctx = Context::new(Theme::Dark, Density::Cozy);
    let mut style = Style::new(Preset::PaneGlow);
    style.overrides.material = Some(Material::Flat);
    let no_effect = styles::preview(&mut renderer, &style, ctx).unwrap();
    style.overrides.material = Some(Material::Glow);
    let mut rendered = BTreeSet::new();
    for activity in [
        GlowActivity::Idle,
        GlowActivity::Active,
        GlowActivity::Waiting,
        GlowActivity::Errored,
    ] {
        style.overrides.glow = Some(Glow {
            activity,
            inferred_multiplier: 0.,
            seed: 123,
            ..Default::default()
        });
        assert_eq!(
            styles::preview(&mut renderer, &style, ctx).unwrap(),
            no_effect,
            "zero grade must remove the effect for every selected state colour"
        );
        style.overrides.glow.as_mut().unwrap().inferred_multiplier = 1.;
        let image = styles::preview(&mut renderer, &style, ctx).unwrap();
        assert_ne!(image, no_effect);
        rendered.insert(image);
    }
    assert_eq!(
        rendered.len(),
        4,
        "each state must reach the existing L2 effect"
    );
    style.overrides.glow = Some(Glow {
        halo_alpha: Some(0.),
        particles: Some(0),
        ..Default::default()
    });
    assert_eq!(
        styles::preview(&mut renderer, &style, ctx).unwrap(),
        no_effect,
        "independent halo/count inputs must reach L2"
    );
    style.overrides.glow = Some(Glow::default());
    let first = styles::preview(&mut renderer, &style, ctx).unwrap();
    style.overrides.glow.as_mut().unwrap().seed += 1;
    assert_ne!(styles::preview(&mut renderer, &style, ctx).unwrap(), first);
}

#[test]
fn style_matrix_is_canonical_and_every_preset_renders_on_the_native_device() {
    let cases = styles::cases();
    assert_eq!(cases.len(), 112); // Fourteen Style records, across four densities and two themes.
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
            .any(|c| c.relative() == "style/pane-glow-cozy-dark.png")
    );
    let mut renderer = ChromeRenderer::new(vec![]).unwrap();
    for theme in [Theme::Dark, Theme::Light] {
        let ctx = Context::new(theme, Density::Cozy);
        let mut fingerprints = BTreeSet::new();
        for preset in Preset::ALL {
            let image = styles::preview(&mut renderer, &Style::new(preset), ctx).unwrap();
            assert_eq!(image.len(), 360 * 180 * 4);
            assert!(
                image.chunks_exact(4).all(|p| p[3] == 255),
                "foreground composition must preserve the opaque fixture"
            );
            let hash = image.iter().fold(0xcbf29ce484222325u64, |hash, &v| {
                (hash ^ v as u64).wrapping_mul(0x100000001b3)
            });
            fingerprints.insert(hash);
        }
        assert_eq!(
            fingerprints.len(),
            Preset::ALL.len(),
            "the gallery must resolve and draw each Style, not reuse a generic fallback"
        );
        let custom = Style {
            tokens: Preset::Card,
            overrides: Overrides {
                fill: Some(Paint::solid(Color::Rgba([
                    64. / 255.,
                    128. / 255.,
                    192. / 255.,
                    1.,
                ]))),
                ..Default::default()
            },
        };
        let image = styles::preview(&mut renderer, &custom, ctx).unwrap();
        let offset = (100 * 360 + 200) * 4;
        assert_eq!(
            &image[offset..offset + 4],
            &[64, 128, 192, 255],
            "the Style-owned fill must be composited independently of its material and border"
        );
    }
}
