// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use bbt_ui::{
    self as ui, components as c,
    modules::launcher::{Cap, Item, Launcher},
};
use bbt_ui_model::layout::{Rect, TextMeasure, TextStyle};
fn item(id: &str, label: &str) -> Item {
    Item {
        id: id.into(),
        label: label.into(),
        harness: "codex".into(),
        ..Default::default()
    }
}

// keeps: spec gui-components §1.3
#[test]
fn enter_picks_is_a_defaulted_public_constructor_input() {
    let ordinary: Item = serde_json::from_str(r#"{"id":"ssh","children":[]}"#).unwrap();
    assert!(!ordinary.enter_picks);
    let harness: Item =
        serde_json::from_str(r#"{"id":"codex","enterPicks":true,"children":[{"id":"codex:tab"}]}"#)
            .unwrap();
    assert!(harness.enter_picks);
    assert!(!harness.children.unwrap()[0].enter_picks);
}
fn launch() -> Launcher {
    let mut remote = item("ssh", "SSH");
    let mut machine = item("ssh:rig", "rig");
    let mut codex = item("ssh:rig:codex", "Codex");
    codex.children = Some(vec![
        item("codex:tab", "New tab"),
        item("codex:right", "Split right"),
        item("codex:bottom", "Split down"),
    ]);
    machine.children = Some(vec![codex]);
    remote.children = Some(vec![machine]);
    Launcher {
        items: vec![remote],
        query: String::new(),
        path: vec![],
        selected: None,
        placeholder: "pick".into(),
        name: "Fenn Calloway".into(),
        swatch: [0.4, 0.6, 0.8, 1.],
        actions: vec![("Open".into(), "↵".into())],
        cap: None,
        spotlight: true,
        edit: None,
        params: None,
    }
}
// keeps: spec gui-components §1.3
#[test]
fn typed_virtual_descent_and_explicit_paths_select_the_same_placement_level() {
    let mut a = launch();
    a.query = "ssh rig codex split".into();
    let av = a.view();
    assert_eq!(
        av.crumbs.iter().map(|x| x.id.as_str()).collect::<Vec<_>>(),
        ["ssh", "ssh:rig", "ssh:rig:codex"]
    );
    assert_eq!(
        av.rows.iter().map(|x| x.id.as_str()).collect::<Vec<_>>(),
        ["codex:right", "codex:bottom"]
    );
    let mut b = launch();
    b.path = vec!["ssh".into(), "ssh:rig".into(), "ssh:rig:codex".into()];
    b.query = "split".into();
    assert_eq!(
        b.view()
            .rows
            .iter()
            .map(|x| x.id.as_str())
            .collect::<Vec<_>>(),
        av.rows.iter().map(|x| x.id.as_str()).collect::<Vec<_>>()
    );
    assert_eq!(
        a.path,
        Vec::<String>::new(),
        "presentation does not mutate navigation state"
    );
}
// keeps: spec gui-components §1.3
#[test]
fn lexical_tiers_are_stable_and_every_word_must_match() {
    let mut a = launch();
    let mut facets = item("seat-a", "Nyx");
    facets.tags = vec!["codex".into(), "ssh".into()];
    let mut sub = item("seat-b", "Rook");
    sub.sub = "codex executor · rig".into();
    a.items = vec![
        facets.clone(),
        sub,
        item("co-1", "Codex B"),
        item("co-2", "Codex A"),
    ];
    a.query = "cod".into();
    assert_eq!(
        a.view()
            .rows
            .iter()
            .map(|x| x.id.as_str())
            .collect::<Vec<_>>(),
        ["co-1", "co-2", "seat-a", "seat-b"]
    );
    a.query = "codex ssh".into();
    assert_eq!(a.view().rows.len(), 1);
    assert_eq!(a.view().rows[0].id, "seat-a");
    a.query = "nxyz".into();
    assert!(a.view().rows.is_empty());
}
struct Metrics;
impl TextMeasure for Metrics {
    fn width(&self, t: &str, s: &TextStyle) -> f32 {
        t.chars().count() as f32 * s.size * 0.5
    }
}
impl c::Measure for Metrics {
    fn font_metrics(&self, s: &TextStyle) -> [f32; 2] {
        [s.size * 0.8, s.size * 0.2]
    }
}
// keeps: spec gui-components §1.3
#[test]
fn cap_keeps_navigation_available_and_dims_completed_launch_row_groups() {
    let mut a = launch();
    let mut go = item("goto", "Go to");
    go.navigation = true;
    go.children = Some(vec![]);
    a.items.push(go);
    a.cap = Some(Cap {
        scope: "swarm".into(),
        name: "term".into(),
        used: 8,
        limit: 8,
        counts: "6 running · 1 starting · 1 in grace".into(),
    });
    assert_eq!(a.view().selected, 1);
    assert_eq!(a.view().rows.len(), 2);
    let ctx = ui::Context::new(ui::Theme::Dark, ui::Density::Cozy);
    let layout = a
        .element(ctx)
        .layout(Rect::new(0., 0., 760., 0.), ctx, &Metrics)
        .unwrap();
    fn filter(n: &c::Layout) -> usize {
        usize::from(matches!(
            &n.kind,
            c::LayoutKind::Filter { opacity: 0.38, .. }
        )) + n.children.iter().map(filter).sum::<usize>()
    }
    assert_eq!(filter(&layout), 1);
    assert!(
        c::paint(&layout, ctx)
            .iter()
            .any(|p| matches!(p, c::Paint::Group { opacity: 0.38, .. }))
    );
}
// keeps: spec gui-components §1.3
#[test]
fn legacy_launcher_delegates_without_hand_painting() {
    let old = include_str!("../../bboxybook/src/modules.rs");
    let body = old
        .split("fn launcher(")
        .nth(1)
        .unwrap()
        .split("\nfn settings(")
        .next()
        .unwrap();
    assert!(body.contains("crate::launchers::paint_legacy"));
    for bad in ["out.rect", "out.line", "Prim::", "text(", "IconTile"] {
        assert!(!body.contains(bad));
    }
}

// keeps: spec gui-components §1.3
#[test]
fn compact_selection_is_twenty_eight_percent_accent_over_surface() {
    use bbt_ui_model::draw::Color;
    let mut launcher = launch();
    launcher.spotlight = false;
    for theme in [ui::Theme::Dark, ui::Theme::Light] {
        let ctx = ui::Context::new(theme, ui::Density::Cozy);
        let root = launcher
            .element(ctx)
            .layout(Rect::new(0., 0., 300., 0.), ctx, &Metrics)
            .unwrap();
        fn find(n: &c::Layout) -> Option<&c::Layout> {
            if matches!(&n.kind,c::LayoutKind::Named(k) if k=="picker/div/opt") {
                return Some(n);
            }
            n.children.iter().find_map(find)
        }
        let c::LayoutKind::Box { style, .. } = &find(&root).unwrap().children[0].kind else {
            panic!("selected row")
        };
        let surface = Color::hex(theme.palette().surface).0;
        let accent = Color::hex(theme.palette().accent).0;
        for i in 0..4 {
            assert!((style.fill[i] - (surface[i] * 0.72 + accent[i] * 0.28)).abs() < 0.000001);
        }
    }
}

#[test]
fn parameter_level_launches_on_enter_and_changes_values_on_right_or_click() {
    use bbt_ui::modules::launcher::{
        FieldKey, Source, SpawnField, ValueChoice,
        controller::{Command, Controller, Outcome},
    };
    let mut presentation = launch();
    let mut claude = item("claude", "Claude");
    claude.parameters = true;
    presentation.items = vec![claude];
    let mut controller = Controller::new(presentation, 0);
    assert!(
        matches!(controller.handle(Command::Activate(Default::default())),Outcome::Pick {ref id,..} if id=="claude")
    );
    assert_eq!(
        controller.handle(Command::Right),
        Outcome::Parameters {
            id: "claude".into()
        }
    );
    assert!(controller.open_parameters(
        "claude",
        vec![
            SpawnField {
                key: FieldKey::Role,
                value: "oracle".into(),
                source: Source::Settings,
                fixed: false,
                choices: Some(vec![
                    ValueChoice {
                        value: "oracle".into(),
                        label: "Oracle".into(),
                        purpose: "supervises".into()
                    },
                    ValueChoice {
                        value: "architect".into(),
                        label: "Architect".into(),
                        purpose: "specs, invariants, decisions".into()
                    },
                ])
            },
            SpawnField {
                key: FieldKey::Directory,
                value: "/swarm".into(),
                source: Source::LastChoice,
                fixed: false,
                choices: Some(vec![])
            },
            SpawnField {
                key: FieldKey::Arguments,
                value: "--old".into(),
                source: Source::LastChoice,
                fixed: false,
                choices: None
            },
        ]
    ));
    assert_eq!(
        controller.launcher.view().crumbs.last().unwrap().label,
        "Claude"
    );
    assert_eq!(controller.handle(Command::Right), Outcome::Changed);
    assert_eq!(
        controller.launcher.view().crumbs.last().unwrap().label,
        "Role"
    );
    controller.handle(Command::Down);
    controller.handle(Command::Activate(Default::default()));
    assert_eq!(
        controller.launcher.params.as_ref().unwrap().fields[0].value,
        "architect"
    );
    controller.handle(Command::Query("no parameter matches this".into()));
    assert_eq!(
        controller.handle(Command::Activate(Default::default())),
        Outcome::Local
    );
    controller.handle(Command::Query(String::new()));
    assert!(
        matches!(controller.handle(Command::Activate(Default::default())),Outcome::LaunchParams {ref id,..} if id=="claude")
    );
    controller.handle(Command::ActivateAt {
        index: 2,
        modifiers: Default::default(),
    });
    controller.handle(Command::Insert("--new 'two words'".into()));
    controller.handle(Command::Escape);
    assert_eq!(
        controller.launcher.params.as_ref().unwrap().fields[2].value,
        "--old"
    );
    controller.handle(Command::ActivateAt {
        index: 2,
        modifiers: Default::default(),
    });
    controller.handle(Command::Insert("--new 'two words'".into()));
    controller.handle(Command::Activate(Default::default()));
    assert_eq!(
        controller.launcher.params.as_ref().unwrap().fields[2].value,
        "--new 'two words'"
    );
    controller.handle(Command::ActivateAt {
        index: 1,
        modifiers: Default::default(),
    });
    controller.handle(Command::Insert("/typed new folder".into()));
    assert_eq!(
        controller.handle(Command::Activate(Default::default())),
        Outcome::Directory {
            value: "/typed new folder".into()
        }
    );
    assert_eq!(
        controller.launcher.params.as_ref().unwrap().fields[1].value,
        "/swarm"
    );
    controller.accept_directory("/typed new folder".into());
    assert_eq!(
        controller.launcher.params.as_ref().unwrap().fields[1].value,
        "/typed new folder"
    );
    controller.handle(Command::ActivateAt {
        index: 2,
        modifiers: Default::default(),
    });
    controller.handle(Command::Digit {
        digit: 1,
        modifiers: Default::default(),
    });
    controller.handle(Command::Insert("23".into()));
    controller.handle(Command::Activate(Default::default()));
    assert_eq!(
        controller.launcher.params.as_ref().unwrap().fields[2].value,
        "123"
    );
    assert!(matches!(
        controller.handle(Command::Digit {
            digit: 1,
            modifiers: Default::default()
        }),
        Outcome::LaunchParams { .. }
    ));
    controller.handle(Command::Escape);
    assert!(controller.launcher.params.is_none());
}
