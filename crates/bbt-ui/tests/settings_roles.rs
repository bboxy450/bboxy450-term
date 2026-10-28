// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use bbt_ui::{
    self as ui, components as c,
    modules::settings::{
        controller::{Action, Controller, Field, Focus, Hit, Patch, Regions, role_region},
        *,
    },
    text_edit::Command,
};
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
fn role(custom: bool) -> Role {
    Role {
        prompt: "Call orient first".into(),
        send: "own swarm · operator DM".into(),
        color: "#8B5CF6".into(),
        mark: "♜".into(),
        cls: "worker".into(),
        custom,
    }
}
fn records() -> Records {
    Records {
        roles: vec![
            (
                "oracle".into(),
                Role {
                    cls: "oracle".into(),
                    ..role(false)
                },
            ),
            ("executor".into(), role(false)),
            ("creativity".into(), role(false)),
            ("reviewer".into(), role(true)),
        ],
        harness: vec![
            (
                "claude".into(),
                Harness {
                    role: "oracle".into(),
                    available_roles: vec!["oracle".into(), "executor".into()],
                    eligible_roles: vec![
                        "oracle".into(),
                        "executor".into(),
                        "creativity".into(),
                        "reviewer".into(),
                    ],
                    tile: "#D97757".into(),
                    command: "claude".into(),
                    ..Default::default()
                },
            ),
            (
                "codex".into(),
                Harness {
                    role: "executor".into(),
                    available_roles: vec!["executor".into(), "creativity".into()],
                    eligible_roles: vec!["executor".into(), "creativity".into(), "reviewer".into()],
                    tile: "#1F2937".into(),
                    command: "codex".into(),
                    ..Default::default()
                },
            ),
        ],
        ..Default::default()
    }
}
fn layout(view: &Settings, control: &Controller) -> c::Layout {
    let ctx = ui::Context::new(ui::Theme::Dark, ui::Density::Cozy);
    view.element_with(ctx, &Metrics, 1216., 900., &control.presentation())
        .layout(Rect::new(0., 0., 1216., 900.), ctx, &Metrics)
        .unwrap()
}
fn named<'a>(node: &'a c::Layout, key: &str) -> Option<&'a c::Layout> {
    if matches!(&node.kind,c::LayoutKind::Named(name) if name == key) {
        return Some(node);
    }
    node.children.iter().find_map(|node| named(node, key))
}
fn click(control: &mut Controller, view: &mut Settings, key: &str) -> Option<Patch> {
    let frame = layout(view, control);
    let r = named(&frame, key)
        .unwrap_or_else(|| panic!("missing {key}"))
        .rect;
    let hit = Regions::new(&frame, view)
        .unwrap()
        .hit([r.x + r.w / 2., r.y + r.h / 2.])
        .unwrap_or_else(|| panic!("no retained hit for {key}"));
    let action = match hit {
        Hit::Toggle(field) => Action::Toggle(field),
        Hit::Reset(field) => Action::Reset(field),
        Hit::Section(section) => Action::Section(section),
        _ => panic!("unexpected {hit:?}"),
    };
    control.handle(action, &mut view.values, &view.defaults)
}
fn act(control: &mut Controller, values: &mut Records, action: Action) -> Option<Patch> {
    control.handle(action, values, &records())
}

#[test]
fn hosted_creation_uses_retained_hits_and_keeps_a_draft_through_search_and_ime() {
    let mut view = Settings {
        section: "Roles".into(),
        values: records(),
        defaults: records(),
        ..Default::default()
    };
    let mut control = Controller::new("Roles");
    assert!(click(&mut control, &mut view, "role/new").is_none());
    assert!(control.editing());
    act(
        &mut control,
        &mut view.values,
        Action::Search(Command::Insert("  Build Guide  ".into())),
    );
    act(&mut control, &mut view.values, Action::Focus(Focus::Search));
    act(
        &mut control,
        &mut view.values,
        Action::Search(Command::Insert("not a role name".into())),
    );
    assert_eq!(
        control.presentation().role_draft.unwrap().name,
        "  Build Guide  "
    );
    act(
        &mut control,
        &mut view.values,
        Action::Section("Roles".into()),
    );
    assert!(click(&mut control, &mut view, "role/draft/prompt").is_none());
    act(
        &mut control,
        &mut view.values,
        Action::Search(Command::Preedit {
            text: "起床".into(),
            caret: Some(2),
        }),
    );
    assert_eq!(control.presentation().role_draft.unwrap().role.prompt, "");
    let focused = layout(&view, &control);
    assert!(
        Regions::new(&focused, &view).unwrap().caret.is_some(),
        "role editor supplies the IME anchor"
    );
    act(
        &mut control,
        &mut view.values,
        Action::Search(Command::Commit("First call orient".into())),
    );
    act(&mut control, &mut view.values, Action::Commit);
    click(&mut control, &mut view, "role/draft/color");
    click(&mut control, &mut view, "settings/choice/1");
    click(&mut control, &mut view, "role/draft/mark");
    click(&mut control, &mut view, "settings/choice/2");
    let patch = click(&mut control, &mut view, "role/draft/add").unwrap();
    let Patch::RoleUpsert { name, role } = patch else {
        panic!("role creation patch")
    };
    assert_eq!(name, "Build Guide");
    assert_eq!(role.prompt, "First call orient");
    assert_eq!(role.color, "#2E6BFF");
    assert_eq!(role.mark, "♟");
    assert_eq!(role.cls, "worker");
    assert!(role.custom);
    assert!(control.presentation().role_draft.is_none());
    assert!(named(&layout(&view, &control), &role_region(&name, "prompt")).is_some());
}

#[test]
fn creation_validation_cancel_and_builtin_class_guards_keep_existing_records() {
    let mut values = records();
    let original = serde_json::to_value(&values).unwrap();
    let mut control = Controller::new("Roles");
    act(&mut control, &mut values, Action::Toggle(Field::RoleNew));
    act(
        &mut control,
        &mut values,
        Action::Search(Command::Insert("Oracle".into())),
    );
    assert!(act(&mut control, &mut values, Action::Toggle(Field::RoleAdd)).is_none());
    assert!(control.error.as_deref().unwrap().contains("reserved"));
    assert!(control.can_cancel());
    act(&mut control, &mut values, Action::Cancel);
    act(&mut control, &mut values, Action::Cancel);
    assert!(!control.can_cancel());
    assert!(
        act(
            &mut control,
            &mut values,
            Action::Toggle(Field::RoleClass("oracle".into()))
        )
        .is_none()
    );
    assert!(
        act(
            &mut control,
            &mut values,
            Action::Toggle(Field::RoleDelete("oracle".into()))
        )
        .is_none()
    );
    assert!(
        act(
            &mut control,
            &mut values,
            Action::Set(Patch::RoleUpsert {
                name: "oracle".into(),
                role: Role {
                    cls: "worker".into(),
                    ..role(false)
                }
            })
        )
        .is_none()
    );
    assert_eq!(serde_json::to_value(&values).unwrap(), original);
}

#[test]
fn prompt_edits_commit_on_blur_and_a_second_control_patch_is_not_lost() {
    let mut values = records();
    let mut control = Controller::new("Roles");
    act(
        &mut control,
        &mut values,
        Action::Toggle(Field::RolePrompt("reviewer".into())),
    );
    act(
        &mut control,
        &mut values,
        Action::Search(Command::SelectAll),
    );
    assert!(
        act(
            &mut control,
            &mut values,
            Action::Search(Command::Insert("New orient instructions".into()))
        )
        .is_none()
    );
    assert_eq!(values.roles[3].1.prompt, "Call orient first");
    let first = act(
        &mut control,
        &mut values,
        Action::Toggle(Field::HarnessAvailableRole {
            harness: "codex".into(),
            role: "reviewer".into(),
        }),
    )
    .unwrap();
    assert!(
        matches!(first, Patch::RoleUpsert { name, role } if name == "reviewer" && role.prompt == "New orient instructions")
    );
    let extra = control.take_patches();
    assert!(
        matches!(extra.as_slice(), [Patch::HarnessAvailableRoles { harness, roles }] if harness == "codex" && roles.contains("reviewer"))
    );
    assert!(control.take_patches().is_empty());
    assert_eq!(values.roles[3].1.prompt, "New orient instructions");
    act(
        &mut control,
        &mut values,
        Action::Toggle(Field::RolePrompt("reviewer".into())),
    );
    act(
        &mut control,
        &mut values,
        Action::Search(Command::Insert(" discarded".into())),
    );
    assert!(act(&mut control, &mut values, Action::Cancel).is_none());
    assert_eq!(values.roles[3].1.prompt, "New orient instructions");
}

#[test]
fn harness_defaults_offer_only_available_roles_and_cannot_be_removed_from_the_set() {
    let mut view = Settings {
        section: "Harnesses".into(),
        values: records(),
        defaults: records(),
        ..Default::default()
    };
    let mut control = Controller::new("Harnesses");
    assert!(
        act(
            &mut control,
            &mut view.values,
            Action::Toggle(Field::HarnessAvailableRole {
                harness: "codex".into(),
                role: "executor".into()
            })
        )
        .is_none()
    );
    click(&mut control, &mut view, "harness/codex/role");
    assert_eq!(
        control.presentation().picker.as_ref().unwrap().choices,
        ["executor", "creativity"]
    );
    let closed_size = layout(&view, &Controller::new("Harnesses")).rect;
    assert_eq!(
        layout(&view, &control).rect,
        closed_size,
        "a picker overlays rather than reflows cards"
    );
    let patch = click(&mut control, &mut view, "settings/choice/1").unwrap();
    assert_eq!(
        patch,
        Patch::HarnessRole {
            harness: "codex".into(),
            role: "creativity".into()
        }
    );
    let removed = click(&mut control, &mut view, "harness/codex/available/executor").unwrap();
    assert!(
        matches!(removed, Patch::HarnessAvailableRoles { roles, .. } if roles == ["creativity".into()].into())
    );
    assert!(
        act(
            &mut control,
            &mut view.values,
            Action::Set(Patch::HarnessRole {
                harness: "codex".into(),
                role: "oracle".into()
            })
        )
        .is_none()
    );
    assert_eq!(view.values.harness[1].1.role, "creativity");
}

#[test]
fn keyboard_picker_cancel_tab_and_custom_class_change_share_control_state() {
    let mut values = records();
    let mut control = Controller::new("Roles");
    act(
        &mut control,
        &mut values,
        Action::Toggle(Field::RoleClass("reviewer".into())),
    );
    act(&mut control, &mut values, Action::Nudge(1));
    act(&mut control, &mut values, Action::Cancel);
    assert_eq!(values.roles[3].1.cls, "worker");
    act(
        &mut control,
        &mut values,
        Action::Toggle(Field::RoleClass("reviewer".into())),
    );
    act(&mut control, &mut values, Action::Nudge(1));
    let patch = act(&mut control, &mut values, Action::Commit).unwrap();
    assert!(matches!(patch, Patch::RoleUpsert { role, .. } if role.cls == "oracle"));
    act(&mut control, &mut values, Action::Toggle(Field::RoleNew));
    act(
        &mut control,
        &mut values,
        Action::Search(Command::Insert("Draft survives Tab".into())),
    );
    act(&mut control, &mut values, Action::NextFocus(false));
    assert_eq!(control.focus, Focus::Control(Field::RoleDraftMark));
    assert_eq!(
        control.presentation().role_draft.unwrap().name,
        "Draft survives Tab"
    );
    assert_ne!(role_region("draft", "prompt"), "role/draft/prompt");
    assert_ne!(role_region("a/b", "prompt"), role_region("a%2Fb", "prompt"));
}

#[test]
fn hosted_destinations_and_harnesses_have_no_creation_or_fake_authority_controls() {
    for section in ["Harnesses", "Cloud", "Machines"] {
        let view = Settings {
            section: section.into(),
            values: records(),
            ..Default::default()
        };
        let frame = layout(&view, &Controller::new(section));
        fn text(node: &c::Layout) -> String {
            let mut s = if let c::LayoutKind::Text { runs, .. } = &node.kind {
                runs.iter().map(|r| r.text.as_str()).collect()
            } else {
                String::new()
            };
            for child in &node.children {
                s += &text(child);
            }
            s
        }
        let labels = text(&frame);
        for absent in [
            "+ new harness",
            "+ connect a provider",
            "+ add a machine",
            "Read-only runtime projection",
        ] {
            assert!(!labels.contains(absent), "{section}: {labels}");
        }
        let fields = Regions::new(&frame, &view).unwrap().fields();
        assert!(
            fields.iter().all(|f| matches!(
                f,
                Field::HarnessRole(_) | Field::HarnessAvailableRole { .. }
            )),
            "{fields:?}"
        );
    }
}

#[test]
fn shell_has_no_prompt_editor_and_prompt_limits_match_the_catalog() {
    let mut values = records();
    let shell = Role {
        prompt: String::new(),
        mark: String::new(),
        ..role(false)
    };
    values.roles.push(("shell".into(), shell.clone()));
    let mut control = Controller::new("Roles");
    assert!(
        act(
            &mut control,
            &mut values,
            Action::Toggle(Field::RolePrompt("shell".into()))
        )
        .is_none()
    );
    assert!(!control.editing());
    let view = Settings {
        section: "Roles".into(),
        values,
        ..Default::default()
    };
    let frame = layout(&view, &control);
    assert!(
        !Regions::new(&frame, &view)
            .unwrap()
            .fields()
            .contains(&Field::RolePrompt("shell".into()))
    );
    assert!(
        Patch::RoleUpsert {
            name: "shell".into(),
            role: shell
        }
        .validate()
        .is_ok()
    );
    assert!(
        Patch::RoleUpsert {
            name: "reviewer".into(),
            role: Role {
                prompt: "a".repeat(32 * 1024 + 1),
                ..role(true)
            }
        }
        .validate()
        .is_err()
    );
}

#[test]
fn a_long_draft_name_keeps_its_caret_inside_the_header_control() {
    let mut values = records();
    let mut control = Controller::new("Roles");
    act(&mut control, &mut values, Action::Toggle(Field::RoleNew));
    act(
        &mut control,
        &mut values,
        Action::Search(Command::Insert("a".repeat(100))),
    );
    let view = Settings {
        section: "Roles".into(),
        values,
        ..Default::default()
    };
    let frame = layout(&view, &control);
    let field = named(&frame, "role/draft/name").unwrap().rect;
    let caret = Regions::new(&frame, &view).unwrap().caret.unwrap();
    assert!(
        caret.x >= field.x && caret.right() <= field.right(),
        "{caret:?} outside {field:?}"
    );
}
