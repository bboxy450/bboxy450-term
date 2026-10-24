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

#[test]
fn role_prompt_grows_to_its_content_then_retains_a_bounded_scroll_view() {
    fn find<'a>(n: &'a c::Layout, key: &str) -> Option<&'a c::Layout> {
        if matches!(&n.kind,c::LayoutKind::Named(k)if k==key) {
            Some(n)
        } else {
            n.children.iter().find_map(|n| find(n, key))
        }
    }
    let mut view = Settings {
        section: "Roles".into(),
        values: Records {
            roles: vec![(
                "test".into(),
                Role {
                    prompt: "short".into(),
                    color: "#abcdef".into(),
                    ..Default::default()
                },
            )],
            ..Default::default()
        },
        ..Default::default()
    };
    let short = layout(&view);
    assert_eq!(find(&short, "card0/f0/control").unwrap().rect.h, 88.);
    view.values.roles[0].1.prompt = "line of supplied prompt
"
    .repeat(80);
    let long = layout(&view);
    let control = find(&long, "card0/f0/control").unwrap();
    assert_eq!(control.rect.h, 240.);
    fn scroll(n: &c::Layout) -> bool {
        matches!(&n.kind,c::LayoutKind::ScrollView{content,..}if content[1]>n.rect.h)
            || n.children.iter().any(scroll)
    }
    assert!(scroll(control));
}

#[test]
fn hosted_settings_search_keeps_retained_control_regions_and_unavailable_fields_honest() {
    use controller::{Action, Controller, Field, Focus, Hit, Patch};
    let ctx = ui::Context::new(ui::Theme::Light, ui::Density::Cozy);
    let mut controller = Controller::new("Account");
    let mut values = Records::default();
    let defaults = Records::default();
    controller.handle(Action::Focus(Focus::Search), &mut values, &defaults);
    controller.handle(
        Action::Search(ui::text_edit::Command::Insert("Appearance".into())),
        &mut values,
        &defaults,
    );
    let view = Settings {
        section: controller.section.clone(),
        values: values.clone(),
        defaults: defaults.clone(),
        ..Default::default()
    };
    let state = controller.presentation();
    let tree = view
        .element_with(ctx, &Metrics, 1216., 638., &state)
        .layout(Rect::new(0., 0., 1216., 638.), ctx, &Metrics)
        .unwrap();
    let regions = controller::Regions::new(&tree, &view).unwrap();
    assert!(regions.fields().contains(&Field::Theme));
    assert!(regions.fields().contains(&Field::Rim));
    assert!(labels(&tree).contains("not yet applied to the window"));
    assert!(
        controller
            .handle(
                Action::Set(Patch::Theme("light".into())),
                &mut values,
                &defaults
            )
            .is_some()
    );
    assert_eq!(values.appearance.theme, "light");
    if let Some(rim) = regions.rim {
        assert!(matches!(
            regions.hit([rim.x + rim.w / 2., rim.y + rim.h / 2.]),
            Some(Hit::Rim)
        ));
        assert!(
            regions
                .hit([rim.x, regions.viewport.bottom() + 1.])
                .is_none()
        );
    }
    controller.handle(Action::Section("Account".into()), &mut values, &defaults);
    let view = Settings {
        section: "Account".into(),
        ..Default::default()
    };
    let tree = view
        .element_with(ctx, &Metrics, 1216., 638., &controller.presentation())
        .layout(Rect::new(0., 0., 1216., 638.), ctx, &Metrics)
        .unwrap();
    let text = labels(&tree);
    assert!(text.contains("Not exposed by the runtime"));
    assert!(!text.contains("signed in"));
}

#[test]
fn hosted_notification_master_off_retains_values_and_visible_resets_still_work() {
    use controller::{Action, Controller, Field};
    let mut values = Records::default();
    values.notifications.enabled = false;
    values.notifications.only_when_not_frontmost = true;
    let mut controller = Controller::new("Notifications");
    let defaults = Records::default();
    assert!(
        controller
            .handle(Action::Toggle(Field::Background), &mut values, &defaults)
            .is_none()
    );
    assert!(
        controller
            .handle(Action::Reset(Field::Background), &mut values, &defaults)
            .is_some()
    );
    assert!(!values.notifications.only_when_not_frontmost);
    assert!(!values.notifications.enabled);
}

#[test]
fn changed_notification_harness_rows_keep_the_icon_and_inline_label_inside_diff_layers() {
    let mut records = Records::default();
    records.harness.push((
        "codex".into(),
        Harness {
            icon: "codex".into(),
            ..Default::default()
        },
    ));
    records
        .notifications
        .disabled_harnesses
        .insert("codex".into());
    let view = Settings {
        section: "Notifications".into(),
        values: records,
        ..Default::default()
    };
    let tree = layout(&view);
    fn find<'a>(node: &'a c::Layout, key: &str) -> Option<&'a c::Layout> {
        if matches!(&node.kind,c::LayoutKind::Named(name) if name==key) {
            Some(node)
        } else {
            node.children.iter().find_map(|n| find(n, key))
        }
    }
    fn has_icon(node: &c::Layout) -> bool {
        matches!(&node.kind,c::LayoutKind::Icon {name,..} if name=="codex")
            || node.children.iter().any(has_icon)
    }
    let label = find(&tree, "card0/r2/label").unwrap();
    assert!(has_icon(label));
    assert!(find(&tree, "card0/r2/diff").is_some());
}
