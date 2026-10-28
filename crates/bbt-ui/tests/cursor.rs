// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use bbt_ui::{
    self as ui, components as c,
    cursor::Cursor,
    modules::{composer, settings},
};
use bbt_ui_model::layout::{Rect, TextMeasure, TextStyle};

struct Metrics;
impl TextMeasure for Metrics {
    fn width(&self, text: &str, style: &TextStyle) -> f32 {
        text.chars().count() as f32 * style.size * 0.5
    }
}
impl c::Measure for Metrics {
    fn font_metrics(&self, style: &TextStyle) -> [f32; 2] {
        [style.size * 0.8, style.size * 0.2]
    }
}
fn ctx() -> ui::Context {
    ui::Context::new(ui::Theme::Dark, ui::Density::Cozy)
}
fn find<'a>(node: &'a c::Layout, key: &str) -> &'a c::Layout {
    fn search<'a>(node: &'a c::Layout, key: &str) -> Option<&'a c::Layout> {
        if matches!(&node.kind, c::LayoutKind::Named(name) if name == key) {
            return Some(node);
        }
        node.children.iter().find_map(|node| search(node, key))
    }
    search(node, key).unwrap_or_else(|| panic!("missing retained region {key}"))
}
fn center(rect: Rect) -> [f32; 2] {
    [rect.x + rect.w * 0.5, rect.y + rect.h * 0.5]
}
fn composer_regions(controller: &composer::Controller) -> (c::Layout, composer::Regions) {
    let tree = controller
        .view
        .element(ctx(), 400., &Metrics)
        .unwrap()
        .layout(Rect::new(0., 0., 400., 0.), ctx(), &Metrics)
        .unwrap();
    let regions = composer::Regions::new(&tree).unwrap();
    (tree, regions)
}

// keeps: operator actionable-cursor — feedback follows the admitted action.
#[test]
fn composer_cursors_follow_send_validation_and_committed_editing() {
    use composer::{Address, Attachment, Command, Composer, Controller, Field, Outcome};
    let valid = Composer {
        text: "a useful message".into(),
        to: vec![Address {
            kind: "seat".into(),
            destination: "seats/a/inbox".into(),
            ..Default::default()
        }],
        focus: true,
        ..Default::default()
    };
    let mut cases = vec![(valid.clone(), true)];
    let mut empty = valid.clone();
    empty.text.clear();
    cases.push((empty, false));
    let mut missing = valid.clone();
    missing.to.clear();
    cases.push((missing, false));
    let mut unresolved = valid.clone();
    unresolved.to[0].destination.clear();
    cases.push((unresolved, false));
    let mut role = valid.clone();
    role.to[0].destination.clear();
    role.to[0].kind = "role".into();
    role.to[0].role = "oracle".into();
    cases.push((role, true));
    let mut delegate = valid.clone();
    delegate.mode = "delegate".into();
    delegate.text.clear();
    cases.push((delegate.clone(), false));
    delegate.attach.push(Attachment {
        kind: "task".into(),
        path: ".bboxy/tasks/a.md".into(),
        ..Default::default()
    });
    cases.push((delegate.clone(), true));
    delegate.attach[0].kind = "artifact".into();
    cases.push((delegate, false));
    for (view, enabled) in cases {
        let controller = Controller::new(view);
        let (tree, regions) = composer_regions(&controller);
        let send = find(&tree, "composer/send").rect;
        let priority = find(&tree, "composer/priority").rect;
        let at = [send.right() - 3., send.y + send.h * 0.5];
        assert_eq!(regions.hit(at), Some(composer::Hit::Send));
        assert_eq!(
            regions.cursor(at, &controller),
            if enabled {
                Cursor::Pointer
            } else {
                Cursor::Default
            }
        );
        assert_eq!(
            matches!(controller.clone().handle(Command::Send), Outcome::Submit(_)),
            enabled
        );
        assert_eq!(
            regions.cursor(center(priority), &controller),
            Cursor::Pointer
        );
        assert_eq!(
            regions.cursor(center(regions.body), &controller),
            Cursor::Text
        );
        assert_eq!(
            regions.cursor(center(regions.address), &controller),
            Cursor::Text
        );
        assert_eq!(regions.cursor([-1., -1.], &controller), Cursor::Default);
    }
    let mut controller = Controller::new(valid);
    for field in [Field::Body, Field::Address] {
        controller.handle(Command::Focus(field));
        controller.handle(Command::Edit(ui::text_edit::Command::Preedit {
            text: "marked".into(),
            caret: Some(6),
        }));
        let (tree, regions) = composer_regions(&controller);
        let send = find(&tree, "composer/send").rect;
        assert_eq!(
            regions.cursor([send.right() - 3., send.y + send.h * 0.5], &controller),
            Cursor::Default
        );
        assert_eq!(controller.handle(Command::Send), Outcome::Local);
        controller.handle(Command::Edit(ui::text_edit::Command::CancelPreedit));
    }
}

fn settings_regions(view: &settings::Settings) -> (c::Layout, settings::controller::Regions) {
    let controller = settings::controller::Controller::new(&view.section);
    let tree = view
        .element_with(ctx(), &Metrics, 1216., 638., &controller.presentation())
        .layout(Rect::new(0., 0., 1216., 638.), ctx(), &Metrics)
        .unwrap();
    let regions = settings::controller::Regions::new(&tree, view).unwrap();
    (tree, regions)
}

// keeps: operator actionable-cursor — unavailable controls never advertise actions.
#[test]
fn settings_cursors_use_actual_fields_availability_and_scroll_clip() {
    let view = settings::Settings {
        section: "Appearance".into(),
        ..Default::default()
    };
    let (tree, regions) = settings_regions(&view);
    assert_eq!(regions.cursor(center(regions.search)), Cursor::Text);
    assert_eq!(
        regions.cursor(center(find(&tree, "nav1").rect)),
        Cursor::Pointer
    );
    let rim = regions.rim.expect("appearance slider");
    assert_eq!(regions.cursor(center(rim)), Cursor::EwResize);
    assert_eq!(
        regions.cursor([rim.x, regions.viewport.bottom() + 1.]),
        Cursor::Default
    );
    assert_eq!(regions.cursor([-1., -1.]), Cursor::Default);

    let roles = settings::Settings {
        section: "Roles".into(),
        values: settings::Records {
            roles: vec![(
                "oracle".into(),
                settings::Role {
                    prompt: "Supervise".into(),
                    cls: "oracle".into(),
                    ..Default::default()
                },
            )],
            ..Default::default()
        },
        ..Default::default()
    };
    let (tree, regions) = settings_regions(&roles);
    assert_eq!(
        regions.cursor(center(
            find(
                &tree,
                &settings::controller::role_region("oracle", "prompt")
            )
            .rect
        )),
        Cursor::Text
    );
    assert_eq!(
        regions.cursor(center(find(&tree, "card0/f2/control").rect)),
        Cursor::Default
    );
    assert_eq!(
        regions.cursor(center(find(&tree, "role/new").rect)),
        Cursor::Pointer
    );

    let mut plugins = settings::Settings {
        section: "Plugins".into(),
        values: settings::Records {
            plugins: vec![settings::plugins::Plugin {
                name: "test".into(),
                trust: "trusted".into(),
                actionable: true,
                ..Default::default()
            }],
            ..Default::default()
        },
        ..Default::default()
    };
    for enabled in [false, true] {
        plugins.values.plugin_controls = enabled;
        let (tree, regions) = settings_regions(&plugins);
        let at = center(find(&tree, "card0/plugin0/control").rect);
        assert!(
            regions.hit(at).is_some(),
            "unavailable controls retain their explanatory click"
        );
        assert_eq!(
            regions.cursor(at),
            if enabled {
                Cursor::Pointer
            } else {
                Cursor::Default
            }
        );
    }
}
