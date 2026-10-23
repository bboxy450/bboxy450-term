use bbt_ui::*;
use bbt_ui_model::STYLE as DESIGN;

#[test]
fn token_references_and_nontrivial_overrides_round_trip_and_resolve_late() {
    let original = Style {
        tokens: Preset::Card,
        overrides: Overrides {
            material: Some(Material::Glow),
            fill: Some(Paint::solid(Color::Seat)),
            radii: Some([
                Length::Radius(Radius::Row),
                Length::Px(3.),
                Length::Spacing(3),
                Length::Px(0.),
            ]),
            border: Some(Border {
                width: Length::Hairline,
                paint: Paint::solid(Color::Swarm),
                outside: true,
            }),
            padding: Some([
                Length::Spacing(2),
                Length::PadX,
                Length::Spacing(2),
                Length::PadX,
            ]),
            height: Some(Length::RowHeight),
            opacity: Some(0.5),
            dim: Some(true),
            ..Default::default()
        },
    };
    let json = serde_json::to_string_pretty(&original).unwrap();
    assert!(
        json.contains("row_height"),
        "serialize the token, not its current pixels"
    );
    let copy: Style = serde_json::from_str(&json).unwrap();
    assert_eq!(copy, original);
    let mut touch = Context::new(Theme::Light, Density::Touch);
    touch.seat = [0.1, 0.7, 0.4, 1.];
    let resolved = copy.resolve(touch).unwrap();
    assert_eq!(resolved.height, Some(44.));
    assert_eq!(resolved.padding, [4., 14., 4., 14.]);
    assert_eq!(resolved.radii, [10., 3., 6., 0.]);
    assert_eq!(resolved.fill, touch.seat);
    assert_eq!(resolved.opacity, 0.5);
    assert_eq!(resolved.material, Material::Glow);
    assert_eq!(resolved.dim, Some(Dim::default()));
    let compact = copy
        .resolve(Context {
            density: Density::Compact,
            ..touch
        })
        .unwrap();
    assert_eq!(compact.height, Some(20.));
    assert_eq!(compact.padding, [4., 8., 4., 8.]);
    let encoded = serde_json::to_string(&resolved).unwrap();
    assert_eq!(
        serde_json::from_str::<ResolvedStyle>(&encoded).unwrap(),
        resolved
    );
}

#[test]
fn presets_resolve_across_the_matrix_and_keep_effect_separate_from_fill_border() {
    for preset in Preset::ALL {
        let style = Style::new(preset);
        let copy: Style = serde_json::from_slice(&serde_json::to_vec(&style).unwrap()).unwrap();
        for theme in [Theme::Dark, Theme::Light] {
            for density in [
                Density::Touch,
                Density::Comfortable,
                Density::Cozy,
                Density::Compact,
            ] {
                assert_eq!(
                    copy.resolve(Context::new(theme, density)),
                    style.resolve(Context::new(theme, density))
                );
            }
        }
    }
    let ctx = Context::new(Theme::Dark, Density::Cozy);
    let glow = Style::new(Preset::PaneGlow).resolve(ctx).unwrap();
    // G1-G2: panes draw the silk lane, not the halo glow.
    assert_eq!(glow.material, Material::Lane);
    assert_eq!(
        glow.fill,
        Paint::solid(Color::Surface).resolve(ctx).unwrap()
    );
    assert_eq!(glow.border.unwrap().width, 1.5);
    let modal = Style::new(Preset::Modal).resolve(ctx).unwrap();
    assert_eq!(modal.material, Material::Frosted);
    assert_eq!(modal.fill[3], DESIGN.material.panel_alpha as f32);
    assert!(modal.border.unwrap().outside);
    assert_eq!(modal.shadow.unwrap().offset, [0., 40.]);
    assert_eq!(
        modal.inner_highlight.unwrap()[3],
        DESIGN.material.inner_highlight_alpha as f32
    );
    assert_eq!(
        Style::new(Preset::Row)
            .resolve(Context {
                selected: true,
                ..ctx
            })
            .unwrap()
            .fill[3],
        DESIGN.tint.selection_alpha as f32
    );
    assert_eq!(
        Style::new(Preset::Band).resolve(ctx).unwrap().height,
        Some(DESIGN.spacing.band as f32)
    );
    let light = Context::new(Theme::Light, Density::Cozy);
    assert_ne!(
        Style::new(Preset::Card).resolve(ctx).unwrap().fill,
        Style::new(Preset::Card).resolve(light).unwrap().fill
    );
}

#[test]
fn malformed_override_data_fails_instead_of_silently_changing_the_style() {
    assert!(
        serde_json::from_str::<Style>(r#"{"tokens":"card","overrides":{"raduis":2}}"#).is_err()
    );
    let ctx = Context::new(Theme::Dark, Density::Cozy);
    for length in [Length::Px(-1.), Length::Px(f32::NAN), Length::Spacing(0)] {
        assert!(length.resolve(ctx.density).is_err());
    }
    let bad = Style {
        tokens: Preset::Card,
        overrides: Overrides {
            opacity: Some(1.1),
            ..Default::default()
        },
    };
    assert!(bad.resolve(ctx).is_err());
    assert!(
        Paint::solid(Color::Rgba([0., 2., 0., 1.]))
            .resolve(ctx)
            .is_err()
    );
}

#[test]
fn glow_state_round_trips_and_rejects_invalid_shader_inputs() {
    let ctx = Context::new(Theme::Dark, Density::Cozy);
    let mut style = Style::new(Preset::PaneGlow);
    assert_eq!(style.resolve(ctx).unwrap().glow, None);
    for activity in [
        GlowActivity::Idle,
        GlowActivity::Active,
        GlowActivity::Waiting,
        GlowActivity::Errored,
    ] {
        let input = Glow {
            activity,
            inferred_multiplier: 0.5,
            seed: 12345,
            ..Default::default()
        };
        style.overrides.glow = Some(input);
        let copy: Style = serde_json::from_slice(&serde_json::to_vec(&style).unwrap()).unwrap();
        assert_eq!(copy.resolve(ctx).unwrap().glow, Some(input));
    }
    for inferred_multiplier in [-0.1, 1.1, f32::NAN] {
        style.overrides.glow = Some(Glow {
            inferred_multiplier,
            ..Default::default()
        });
        assert!(style.resolve(ctx).is_err());
    }
    for glow in [
        Glow {
            halo_alpha: Some(1.1),
            ..Default::default()
        },
        Glow {
            particles: Some(49),
            ..Default::default()
        },
    ] {
        style.overrides.glow = Some(glow);
        assert!(style.resolve(ctx).is_err());
    }
    style.overrides.glow = Some(Glow {
        seed: (1 << 24) + 1,
        ..Default::default()
    });
    assert!(style.resolve(ctx).is_err());
}
#[test]
fn positioned_shadow_round_trips_validates_and_preserves_custom_defaults() {
    use bbt_ui::{Context, Density, Preset, Shadow, Style, Theme};
    let ctx = Context::new(Theme::Dark, Density::Cozy);
    let mut s = Style::new(Preset::Card);
    s.overrides.shadow = Some(Shadow::Positioned {
        offset: [-3., 20.],
        blur: 60.,
        color: [0., 0., 0., 0.45],
    });
    let json = serde_json::to_string(&s).unwrap();
    assert_eq!(serde_json::from_str::<Style>(&json).unwrap(), s);
    let shadow = s.resolve(ctx).unwrap().shadow.unwrap();
    assert_eq!(shadow.offset, [-3., 20.]);
    assert_eq!(shadow.blur, 60.);
    s.overrides.shadow = Some(Shadow::Custom {
        blur: 60.,
        color: [0., 0., 0., 0.45],
    });
    assert_eq!(s.resolve(ctx).unwrap().shadow.unwrap().offset, [0., 0.]);
    for bad in [
        Shadow::Positioned {
            offset: [f32::NAN, 0.],
            blur: 60.,
            color: [0.; 4],
        },
        Shadow::Positioned {
            offset: [0., f32::INFINITY],
            blur: 60.,
            color: [0.; 4],
        },
        Shadow::Positioned {
            offset: [0.; 2],
            blur: -1.,
            color: [0.; 4],
        },
        Shadow::Positioned {
            offset: [0.; 2],
            blur: 1.,
            color: [0., 0., 0., 1.1],
        },
    ] {
        s.overrides.shadow = Some(bad);
        assert!(s.resolve(ctx).is_err());
    }
}
