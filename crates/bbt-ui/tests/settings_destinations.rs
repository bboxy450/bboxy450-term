// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use bbt_ui::{
    self as ui, components as c,
    modules::{
        link::{Cloud, LinkState, MachineState},
        settings::{
            Records, Settings,
            controller::{Action, Controller, Field, Focus, Hit, Patch, Regions},
        },
    },
};
use bbt_ui_model::layout::{Rect, TextMeasure, TextStyle};
struct Metrics;
impl TextMeasure for Metrics {
    fn width(&self, value: &str, style: &TextStyle) -> f32 {
        value.chars().count() as f32 * style.size * 0.5
    }
}
impl c::Measure for Metrics {
    fn font_metrics(&self, style: &TextStyle) -> [f32; 2] {
        [style.size * 0.8, style.size * 0.2]
    }
}
fn records() -> Records {
    Records {
        cloud: vec![Cloud(
            "Display cloud".into(),
            "eu-test".into(),
            true,
            "#2E6BFF".into(),
            Some(LinkState::Degraded { rtt_ms: 280 }),
        )],
        machines: vec![(
            "Display machine".into(),
            "user@example.invalid".into(),
            "hardware".into(),
            "key".into(),
            MachineState::Link(LinkState::Connected { rtt_ms: 37 }),
        )],
        destination_ids: [
            ("machine/0".into(), "ssh:stable-machine".into()),
            ("cloud/0".into(), "cloud:stable-provider".into()),
        ]
        .into(),
        ..Default::default()
    }
}
fn frame(view: &Settings, control: &Controller, hosted: bool) -> c::Layout {
    let ctx = ui::Context::new(ui::Theme::Dark, ui::Density::Cozy);
    let element = if hosted {
        view.element_with(ctx, &Metrics, 1216., 638., &control.presentation())
    } else {
        view.element(ctx, &Metrics, 1216., 638.)
    };
    element
        .layout(Rect::new(0., 0., 1216., 638.), ctx, &Metrics)
        .unwrap()
}
fn text(node: &c::Layout) -> String {
    let mut out = if let c::LayoutKind::Text { runs, .. } = &node.kind {
        runs.iter().map(|run| run.text.as_str()).collect()
    } else {
        String::new()
    };
    for child in &node.children {
        out += &text(child);
    }
    out
}
fn observed(values: &Records) -> serde_json::Value {
    serde_json::json!({"cloud": values.cloud, "machines": values.machines})
}

#[test]
fn mapped_destinations_have_real_switches_and_clicks_never_change_observed_links() {
    for (section, id) in [
        ("Machines", "ssh:stable-machine"),
        ("Cloud", "cloud:stable-provider"),
    ] {
        let mut view = Settings {
            section: section.into(),
            values: records(),
            ..Default::default()
        };
        let mut control = Controller::new(section);
        let before = observed(&view.values);
        for available in [false, true] {
            let layout = frame(&view, &control, true);
            let regions = Regions::new(&layout, &view).unwrap();
            let field = Field::DestinationAvailable(id.into());
            assert_eq!(regions.fields().as_slice(), std::slice::from_ref(&field));
            let rect = regions
                .control_regions()
                .iter()
                .find(|(key, _)| *key == field)
                .unwrap()
                .1;
            assert!(text(&layout).contains("Available in launcher"));
            let hit = regions
                .hit([rect.x + rect.w / 2., rect.y + rect.h / 2.])
                .unwrap();
            let Hit::Toggle(clicked) = hit else {
                panic!("availability is a retained toggle")
            };
            assert_eq!(clicked, field);
            let patch = control
                .handle(Action::Toggle(clicked), &mut view.values, &view.defaults)
                .unwrap();
            assert_eq!(
                patch,
                Patch::DestinationAvailable {
                    id: id.into(),
                    available
                }
            );
            assert_eq!(view.values.disabled_destinations.contains(id), !available);
            assert_eq!(
                observed(&view.values),
                before,
                "availability must never simulate a connection transition"
            );
        }
        assert!(!text(&frame(&view, &control, true)).contains("+ add a machine"));
        assert!(!text(&frame(&view, &control, true)).contains("+ connect a provider"));
    }
}

#[test]
fn keyboard_activation_and_loaded_disabled_state_use_the_same_membership_intent() {
    for (section, id) in [
        ("Machines", "ssh:stable-machine"),
        ("Cloud", "cloud:stable-provider"),
    ] {
        let mut values = records();
        values.disabled_destinations.insert(id.into());
        let before = observed(&values);
        let mut control = Controller::new(section);
        control.handle(
            Action::Focus(Focus::Navigation),
            &mut values,
            &Default::default(),
        );
        control.handle(Action::NextFocus(false), &mut values, &Default::default());
        assert_eq!(
            control.focus,
            Focus::Control(Field::DestinationAvailable(id.into()))
        );
        assert_eq!(
            control.activate(&mut values, &Default::default()),
            Some(Patch::DestinationAvailable {
                id: id.into(),
                available: true
            })
        );
        assert!(!values.disabled_destinations.contains(id));
        assert_eq!(observed(&values), before);
    }
}

#[test]
fn missing_stale_or_wrong_kind_mappings_cannot_create_controls_or_effects() {
    for section in ["Machines", "Cloud"] {
        let mut values = records();
        values.destination_ids = [
            ("machine/99".into(), "ssh:stale".into()),
            ("machine/0".into(), "cloud:wrong-kind".into()),
            ("cloud/0".into(), "ssh:wrong-kind".into()),
        ]
        .into();
        let before = serde_json::to_value(&values).unwrap();
        let mut control = Controller::new(section);
        let view = Settings {
            section: section.into(),
            values: values.clone(),
            ..Default::default()
        };
        assert!(
            Regions::new(&frame(&view, &control, true), &view)
                .unwrap()
                .fields()
                .is_empty()
        );
        assert!(!text(&frame(&view, &control, true)).contains("Available in launcher"));
        for id in [
            "ssh:stale",
            "cloud:wrong-kind",
            "ssh:wrong-kind",
            "ssh:Display machine",
            "cloud:Display cloud",
        ] {
            assert!(
                control
                    .handle(
                        Action::Set(Patch::DestinationAvailable {
                            id: id.into(),
                            available: false
                        }),
                        &mut values,
                        &Default::default()
                    )
                    .is_none()
            );
        }
        assert_eq!(
            serde_json::to_value(&values).unwrap(),
            before,
            "labels cannot confer a destination ID"
        );
    }
}

#[test]
fn gallery_composition_keeps_its_reference_and_hosted_cloud_has_one_switch() {
    for section in ["Machines", "Cloud"] {
        let view = Settings {
            section: section.into(),
            values: records(),
            ..Default::default()
        };
        let control = Controller::new(section);
        let gallery = frame(&view, &control, false);
        assert!(!text(&gallery).contains("Available in launcher"));
        assert!(Regions::new(&gallery, &view).unwrap().fields().is_empty());
        let layout = frame(&view, &control, true);
        let regions = Regions::new(&layout, &view).unwrap();
        assert_eq!(regions.fields().len(), 1);
        if section == "Cloud" {
            fn switches(node: &c::Layout) -> usize {
                usize::from(
                    matches!(&node.kind,c::LayoutKind::Box { style, .. } if style.height == Some(18.)),
                ) + node.children.iter().map(switches).sum::<usize>()
            }
            assert_eq!(
                switches(&layout),
                1,
                "hosted cloud has availability, not a second inert connection switch"
            );
        }
    }
}
