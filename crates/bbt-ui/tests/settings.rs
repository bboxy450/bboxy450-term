use bbt_ui::{self as ui, components as c, modules::settings::*};
use bbt_ui_model::layout::{Rect, TextMeasure, TextStyle};
struct Metrics;
impl TextMeasure for Metrics {
    fn width(&self, s: &str, t: &TextStyle) -> f32 {
        s.chars().count() as f32 * t.size * 0.5
    }
}
impl c::Measure for Metrics {
    fn font_metrics(&self, t: &TextStyle) -> [f32; 2] {
        [t.size * 0.8, t.size * 0.2]
    }
}
fn layout(view: &Settings) -> c::Layout {
    let ctx = ui::Context::new(ui::Theme::Dark, ui::Density::Cozy);
    view.element(ctx, &Metrics, 1216., 638.)
        .layout(Rect::new(0., 0., 1216., 638.), ctx, &Metrics)
        .unwrap()
}
fn labels(l: &c::Layout) -> String {
    let mut s = if let c::LayoutKind::Text { runs, .. } = &l.kind {
        runs.iter().map(|r| r.text.as_str()).collect()
    } else {
        String::new()
    };
    for child in &l.children {
        s += &labels(child)
    }
    s
}
#[test]
fn account_projects_the_pinned_identity_and_recovery_limits_without_actions() {
    let view = Settings {
        section: "Account".into(),
        values: Records {
            account: Account {
                name: "A test operator".into(),
                fingerprint: "0123456789ABCDEF0123456789ABCDEF01234567".into(),
                trust: vec![
                    "GPG primary".into(),
                    "certified device key".into(),
                    "certified seat key".into(),
                ],
                ..Default::default()
            },
            ..Default::default()
        },
        ..Default::default()
    };
    let s = labels(&layout(&view));
    assert!(s.contains("0123456789ABCDEF0123456789ABCDEF01234567"));
    assert!(s.contains("never exported"));
    assert!(s.contains("restores identity only"));
    assert!(s.contains("peers learn on next contact"));
    assert!(s.contains("assumed, not decided"));
    assert!(!s.contains("create root"));
    assert!(!s.contains("password"));
}
#[test]
fn custom_role_order_and_defaults_drive_the_changed_marker() {
    let role = Role {
        mark: "♜".into(),
        color: "#abcdef".into(),
        prompt: "the edited public prompt".into(),
        send: "operator DM only".into(),
        cls: "worker".into(),
        custom: true,
    };
    let mut view = Settings {
        section: "Roles".into(),
        values: Records {
            roles: vec![
                ("custom worker".into(), role.clone()),
                ("custom observer".into(), role.clone()),
            ],
            ..Default::default()
        },
        defaults: Records {
            roles: vec![(
                "custom worker".into(),
                Role {
                    prompt: "old default".into(),
                    ..role.clone()
                },
            )],
            ..Default::default()
        },
        ..Default::default()
    };
    let s = labels(&layout(&view));
    assert!(s.contains("reset"));
    assert!(s.find("CUSTOM WORKER").unwrap() < s.find("CUSTOM OBSERVER").unwrap());
    assert!(s.contains("changing class re-issues"));
    view.defaults = view.values.clone();
    assert!(!labels(&layout(&view)).contains("reset"));
}
#[test]
fn draft_harness_never_claims_an_inferred_ready_state_can_submit() {
    let mut view = Settings {
        section: "Harnesses".into(),
        draft_harness: Some(DraftHarness {
            name: "custom harness".into(),
            command: "my-tool".into(),
            wake: "pty".into(),
            ..Default::default()
        }),
        ..Default::default()
    };
    let before = labels(&layout(&view));
    assert!(before.contains("never submits on its own"));
    assert!(before.contains("state is inferred from output"));
    view.draft_harness.as_mut().unwrap().hooks = true;
    let after = labels(&layout(&view));
    assert!(after.contains("types after an idle hook"));
    assert!(!after.contains("never submits on its own"));
}
#[test]
fn records_roundtrip_with_ordered_custom_harnesses_and_roles() {
    let v = Settings {
        section: "Harnesses".into(),
        values: Records {
            roles: vec![
                ("z custom".into(), Role::default()),
                ("a custom".into(), Role::default()),
            ],
            harness: vec![(
                "custom".into(),
                Harness {
                    command: "private command supplied by the host".into(),
                    tile: "#345678".into(),
                    custom: true,
                    ..Default::default()
                },
            )],
            ..Default::default()
        },
        ..Default::default()
    };
    let round: Settings = serde_json::from_str(&serde_json::to_string(&v).unwrap()).unwrap();
    assert_eq!(round.values.roles[0].0, "z custom");
    assert!(labels(&layout(&round)).contains("private command supplied by the host"));
}
