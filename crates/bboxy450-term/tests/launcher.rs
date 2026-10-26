use bboxy450_term::{
    launcher::{Command, Controller, Outcome},
    model::Model,
};
use bbt_proto::{GuiHost, HostState, Where};
use bbt_ui::modules::{
    launcher::{Cap, Item, Launcher},
    link::LinkState,
};
use bbt_ui_model::input::Modifiers;

fn item(id: &str, label: &str) -> Item {
    Item {
        id: id.into(),
        label: label.into(),
        ..Default::default()
    }
}
fn codex(id: &str) -> Item {
    Item {
        harness: "codex".into(),
        enter_picks: true,
        children: Some(vec![
            item(&format!("{id}:tab"), "New tab"),
            item(&format!("{id}:right"), "Split right"),
            item(&format!("{id}:bottom"), "Split down"),
        ]),
        ..item(id, "Codex")
    }
}
fn model(state: HostState) -> Model {
    Model {
        hosts: vec![GuiHost {
            location: Where::Ssh { host: "rig".into() },
            provider: "fixture".into(),
            label: "rig".into(),
            address: Some("agent@rig.invalid".into()),
            region: None,
            state,
        }],
        ..Default::default()
    }
}
fn catalog(model: &Model) -> Vec<Item> {
    let mut providers = model.provider_launcher_items();
    for provider in &mut providers {
        for host in provider.children.iter_mut().flatten() {
            if let Some(children) = &mut host.children {
                children.push(codex(&format!("{}:codex", host.id)));
            }
        }
    }
    let mut items = vec![codex("codex")];
    items.extend(providers);
    items.push(Item {
        navigation: true,
        enter_picks: true,
        children: Some(vec![Item {
            navigation: true,
            ..item("settings:account", "Account")
        }]),
        ..item("settings", "Settings")
    });
    items
}
fn controller() -> Controller {
    Controller::new(
        Launcher {
            items: catalog(&model(HostState::Connected { rtt_ms: 24 })),
            query: String::new(),
            path: vec![],
            selected: None,
            placeholder: "new seat".into(),
            name: "Nyx Copperwing".into(),
            swatch: [0.3, 0.6, 0.9, 1.],
            actions: vec![],
            cap: None,
            spotlight: true,
            edit: None,
        },
        1,
    )
}
fn selected(controller: &Controller) -> Option<&str> {
    let view = controller.launcher.view();
    view.rows.get(view.selected).map(|item| item.id.as_str())
}

fn focused_model() -> Model {
    use bbt_ui_model::{Model as UiModel, Seat, SeatId, SplitNode, Swarm, SwarmId, Tab, TabId};
    let mut seat = Seat::new(
        SeatId::from_u128(2),
        SwarmId::from_u128(1),
        [0; 32],
        "anchor",
        "zsh",
    );
    seat.process_generation = 7;
    Model {
        ui: UiModel::new(
            vec![Swarm::new(
                SwarmId::from_u128(1),
                "swarm",
                vec![Tab::new(
                    TabId::from_u128(3),
                    "tab",
                    SplitNode::Leaf(seat.id),
                )],
            )],
            vec![seat],
        )
        .unwrap(),
        ..Default::default()
    }
}

// keeps: spec ux-prototype §5.4
#[test]
fn placement_pick_captures_the_ui_stage_and_generation_before_any_reply() {
    use bbt_proto::seat_state::{Placement, Side};
    let mut model = focused_model();
    let stage = model.ui.window.stage.clone();
    let (root, swarm, captured) = bboxy450_term::launcher::captured_placement(
        "ssh:rig:codex:right",
        Default::default(),
        Some(&model),
        Some(&stage),
    );
    assert_eq!(root, "ssh:rig:codex");
    assert_eq!(swarm, stage.swarm);
    assert_eq!(
        captured,
        Placement::Split {
            tab: stage.tab.unwrap(),
            target: stage.focused.unwrap(),
            target_generation: 7,
            side: Side::Right
        }
    );
    model
        .ui
        .seats
        .get_mut(&stage.focused.unwrap())
        .unwrap()
        .process_generation = 8;
    model.ui.window.stage.focused = None;
    assert!(matches!(
        captured,
        Placement::Split {
            target_generation: 7,
            ..
        }
    ));
}

// keeps: spec ux-prototype §5.4
#[test]
fn explicit_placement_rows_override_modifiers_and_empty_stage_opens_a_tab() {
    use bbt_proto::seat_state::{Placement, Side};
    let model = focused_model();
    let mods = Modifiers {
        command: true,
        shift: true,
        ..Default::default()
    };
    let (_, _, tab) =
        bboxy450_term::launcher::captured_placement("codex:tab", mods, Some(&model), None);
    assert!(matches!(tab, Placement::NewTab { .. }));
    let (_, _, right) =
        bboxy450_term::launcher::captured_placement("codex:right", mods, Some(&model), None);
    assert!(matches!(
        right,
        Placement::Split {
            side: Side::Right,
            ..
        }
    ));
    let (_, _, bottom) =
        bboxy450_term::launcher::captured_placement("codex", mods, Some(&model), None);
    assert!(matches!(
        bottom,
        Placement::Split {
            side: Side::Bottom,
            ..
        }
    ));
    let (_, _, empty) =
        bboxy450_term::launcher::captured_placement("codex:right", Default::default(), None, None);
    assert!(matches!(empty, Placement::NewTab { .. }));
}

// keeps: spec ux-prototype §5.4
#[test]
fn typed_descent_then_right_commits_the_existing_projection_path() {
    let mut control = controller();
    assert_eq!(
        control.handle(Command::Query("ssh rig cod".into())),
        Outcome::Changed
    );
    assert_eq!(selected(&control), Some("ssh:rig:codex"));
    assert_eq!(control.handle(Command::Right), Outcome::Changed);
    assert_eq!(control.launcher.path, ["ssh", "ssh:rig", "ssh:rig:codex"]);
    assert_eq!(control.launcher.query, "");
    assert_eq!(selected(&control), Some("ssh:rig:codex:tab"));
    control.handle(Command::Query("split".into()));
    assert_eq!(selected(&control), Some("ssh:rig:codex:right"));
    assert_eq!(control.handle(Command::Down), Outcome::Changed);
    assert_eq!(selected(&control), Some("ssh:rig:codex:bottom"));
}

// keeps: spec ux-prototype §5.4
#[test]
fn selection_wraps_and_empty_results_are_safe() {
    let mut control = controller();
    assert_eq!(control.handle(Command::Up), Outcome::Changed);
    assert_eq!(selected(&control), Some("settings"));
    assert_eq!(control.handle(Command::Down), Outcome::Changed);
    assert_eq!(selected(&control), Some("codex"));
    control.handle(Command::Hover(2));
    assert_eq!(selected(&control), Some("cloud"));
    assert_eq!(control.handle(Command::Hover(99)), Outcome::Local);
    assert_eq!(selected(&control), Some("cloud"));
    control.handle(Command::Query("zzqx".into()));
    assert_eq!(selected(&control), None);
    for command in [
        Command::Up,
        Command::Down,
        Command::Right,
        Command::Hover(0),
        Command::Activate(Modifiers::default()),
    ] {
        assert_eq!(control.handle(command), Outcome::Local);
    }
}

// keeps: spec ux-prototype §5.4
#[test]
fn left_and_backspace_leave_nonempty_queries_to_the_text_field() {
    let mut control = controller();
    control.handle(Command::Right);
    assert_eq!(control.launcher.path, ["codex"]);
    control.handle(Command::Query("split".into()));
    assert_eq!(control.handle(Command::Left), Outcome::Changed);
    assert_eq!(control.launcher.query, "split");
    assert_eq!(control.handle(Command::Backspace), Outcome::Changed);
    assert_eq!(control.launcher.query, "splt");
    assert_eq!(control.launcher.path, ["codex"]);
    control.handle(Command::Query(String::new()));
    assert_eq!(control.handle(Command::Backspace), Outcome::Changed);
    assert!(control.launcher.path.is_empty());
    assert_eq!(control.handle(Command::Left), Outcome::Local);
}

// keeps: spec ux-prototype §5.4
#[test]
fn escape_climbs_one_explicit_level_before_closing_even_with_a_query() {
    let mut control = controller();
    control.handle(Command::Query("ssh rig cod".into()));
    control.handle(Command::Right);
    control.handle(Command::Query("split".into()));
    for remaining in [2, 1, 0] {
        assert_eq!(control.handle(Command::Escape), Outcome::Changed);
        assert_eq!(control.launcher.path.len(), remaining);
        assert!(control.launcher.query.is_empty());
    }
    control.handle(Command::Query("cod".into()));
    assert_eq!(control.handle(Command::Escape), Outcome::Close);
}

// keeps: spec ux-prototype §5.4
#[test]
fn reroll_is_a_pure_name_request_and_does_not_discard_navigation() {
    let mut control = controller();
    control.handle(Command::Right);
    control.handle(Command::Query("split".into()));
    for seed in [2, 3] {
        assert_eq!(control.handle(Command::Reroll), Outcome::Reroll { seed });
        assert_eq!(control.launcher.name, "Nyx Copperwing");
        assert_eq!(control.launcher.query, "split");
        assert_eq!(control.launcher.path, ["codex"]);
    }
}

// keeps: spec ux-prototype §5.4
#[test]
fn preedit_never_filters_or_activates_and_commit_replaces_the_original_selection() {
    let mut control = controller();
    control.handle(Command::Query("codex".into()));
    let before = control
        .launcher
        .view()
        .rows
        .iter()
        .map(|r| r.id.clone())
        .collect::<Vec<_>>();
    control.handle(Command::SelectAll);
    control.handle(Command::Preedit {
        text: "Claude".into(),
        caret: Some(6),
    });
    assert_eq!(control.launcher.query, "codex");
    assert_eq!(control.handle(Command::Insert("x".into())), Outcome::Local);
    assert_eq!(control.launcher.query, "codex");
    assert_eq!(
        control
            .launcher
            .view()
            .rows
            .iter()
            .map(|r| r.id.clone())
            .collect::<Vec<_>>(),
        before
    );
    assert_eq!(
        control.handle(Command::Activate(Modifiers::default())),
        Outcome::Local
    );
    assert_eq!(
        control.launcher.edit.as_ref().unwrap().replacement,
        Some([0, 5])
    );
    control.handle(Command::Commit("Claude".into()));
    assert_eq!(control.launcher.query, "Claude");
    assert!(control.launcher.edit.as_ref().unwrap().preedit.is_none());
    assert_eq!(control.launcher.edit.as_ref().unwrap().caret, Some(6));
}

// keeps: spec ux-prototype §5.4
#[test]
fn field_selection_insertion_and_deletion_never_split_a_surrogate_pair() {
    use bboxy450_term::launcher::CaretMove;
    let mut control = controller();
    control.handle(Command::Query("A😀B".into()));
    control.handle(Command::SetCaret {
        at: 3,
        extend: false,
    });
    control.handle(Command::MoveCaret {
        motion: CaretMove::Left,
        extend: true,
    });
    assert_eq!(control.launcher.edit.as_ref().unwrap().select, Some([1, 3]));
    control.handle(Command::Insert("界".into()));
    assert_eq!(control.launcher.query, "A界B");
    control.handle(Command::Backspace);
    assert_eq!(control.launcher.query, "AB");
    control.handle(Command::Delete);
    assert_eq!(control.launcher.query, "A");
    control.handle(Command::Query("😀".into()));
    assert_eq!(
        control.handle(Command::SetCaret {
            at: 1,
            extend: false
        }),
        Outcome::Local
    );
    assert_eq!(control.launcher.edit.as_ref().unwrap().caret, Some(2));
}

// keeps: spec ux-prototype §5.4
#[test]
fn cancelling_composition_restores_committed_text_and_the_selection() {
    let mut control = controller();
    control.handle(Command::Query("zsh".into()));
    control.handle(Command::SelectAll);
    control.handle(Command::Preedit {
        text: "你好".into(),
        caret: None,
    });
    assert!(control.launcher.edit.as_ref().unwrap().hide_caret);
    assert_eq!(control.handle(Command::Escape), Outcome::Changed);
    assert_eq!(control.launcher.query, "zsh");
    assert_eq!(control.launcher.edit.as_ref().unwrap().select, Some([0, 3]));
}

// keeps: spec ux-prototype §5.4
#[test]
fn a_provider_disconnect_invalidates_its_path_and_cannot_dive_again() {
    let mut control = controller();
    control.handle(Command::Query("ssh rig cod".into()));
    control.handle(Command::Right);
    let updated = model(HostState::Disconnected {
        reason: "fixture link lost".into(),
    });
    control.replace_items(catalog(&updated));
    assert_eq!(control.launcher.path, ["ssh"]);
    assert_eq!(selected(&control), Some("ssh:rig"));
    assert_eq!(control.handle(Command::Right), Outcome::HostUnavailable);
    assert_eq!(control.launcher.path, ["ssh"]);
    let row = control.launcher.view().rows[0];
    assert_eq!(row.link.as_ref().unwrap().label(), "fixture link lost");
}

// keeps: spec ux-prototype §5.4
#[test]
fn catalog_refresh_preserves_the_selected_id_across_reordering() {
    let mut control = controller();
    control.handle(Command::Hover(1));
    assert_eq!(selected(&control), Some("ssh"));
    let mut items = catalog(&model(HostState::Connected { rtt_ms: 8 }));
    items.reverse();
    control.replace_items(items);
    assert_eq!(selected(&control), Some("ssh"));
    assert_eq!(control.launcher.view().selected, 2);
}

// keeps: spec ux-prototype §5.4
#[test]
fn enter_picks_the_parent_default_and_right_still_opens_placements() {
    let mut control = controller();
    let modifiers = Modifiers {
        command: true,
        shift: true,
        alt: true,
        control: true,
    };
    assert_eq!(
        control.handle(Command::Activate(modifiers)),
        Outcome::Pick {
            id: "codex".into(),
            modifiers
        }
    );
    assert!(control.launcher.path.is_empty());
    assert_eq!(control.handle(Command::Right), Outcome::Changed);
    assert_eq!(control.launcher.path, ["codex"]);
    assert_eq!(
        control.handle(Command::Activate(modifiers)),
        Outcome::Pick {
            id: "codex:tab".into(),
            modifiers
        }
    );
}

// keeps: spec ux-prototype §5.4
#[test]
fn enter_descends_ordinary_parents_and_picks_the_typed_harness_default() {
    let mut control = controller();
    let modifiers = Modifiers::default();
    control.handle(Command::Query("ssh".into()));
    assert_eq!(
        control.handle(Command::Activate(modifiers)),
        Outcome::Changed
    );
    assert_eq!(control.launcher.path, ["ssh"]);
    assert_eq!(
        control.handle(Command::Activate(modifiers)),
        Outcome::Changed
    );
    assert_eq!(control.launcher.path, ["ssh", "ssh:rig"]);
    assert_eq!(
        control.handle(Command::Activate(modifiers)),
        Outcome::Pick {
            id: "ssh:rig:codex".into(),
            modifiers
        }
    );
    let mut typed = controller();
    typed.handle(Command::Query("ssh rig cod".into()));
    assert_eq!(
        typed.handle(Command::Activate(modifiers)),
        Outcome::Pick {
            id: "ssh:rig:codex".into(),
            modifiers
        }
    );
    assert_eq!(typed.launcher.query, "ssh rig cod");
    assert!(typed.launcher.path.is_empty());
}

// keeps: spec ux-prototype §5.4
#[test]
fn digits_pick_or_dive_only_on_an_empty_query_and_preserve_modifiers() {
    let mut control = controller();
    let modifiers = Modifiers {
        alt: true,
        ..Default::default()
    };
    control.handle(Command::Hover(3));
    assert_eq!(
        control.handle(Command::Digit {
            digit: 1,
            modifiers
        }),
        Outcome::Pick {
            id: "codex".into(),
            modifiers
        }
    );
    assert_eq!(selected(&control), Some("settings"));
    assert_eq!(
        control.handle(Command::Digit {
            digit: 2,
            modifiers
        }),
        Outcome::Changed
    );
    assert_eq!(control.launcher.path, ["ssh"]);
    control.handle(Command::Query("rig".into()));
    assert_eq!(
        control.handle(Command::Digit {
            digit: 1,
            modifiers
        }),
        Outcome::Local
    );
    assert_eq!(control.launcher.query, "rig");
    control.handle(Command::Query(String::new()));
    for digit in [0, 9, 10] {
        assert_eq!(
            control.handle(Command::Digit { digit, modifiers }),
            Outcome::Local
        );
    }
}

// keeps: spec ux-prototype §5.4
#[test]
fn the_cap_refuses_parent_and_leaf_picks_while_navigation_and_right_stay_live() {
    let mut control = controller();
    let modifiers = Modifiers::default();
    control.launcher.cap = Some(Cap {
        scope: "swarm".into(),
        used: 8,
        limit: 8,
        ..Default::default()
    });
    assert_eq!(selected(&control), Some("settings"));
    assert_eq!(
        control.handle(Command::Activate(modifiers)),
        Outcome::Pick {
            id: "settings".into(),
            modifiers
        }
    );
    assert_eq!(
        control.handle(Command::ActivateAt {
            index: 0,
            modifiers
        }),
        Outcome::Capped
    );
    assert_eq!(selected(&control), Some("settings"));
    control.handle(Command::Hover(0));
    assert_eq!(
        control.handle(Command::Activate(modifiers)),
        Outcome::Capped
    );
    assert_eq!(control.handle(Command::Right), Outcome::Changed);
    assert_eq!(control.launcher.path, ["codex"]);
    assert_eq!(
        control.handle(Command::Activate(modifiers)),
        Outcome::Capped
    );
    control.handle(Command::Escape);
    control.handle(Command::Hover(3));
    control.handle(Command::Right);
    assert_eq!(
        control.handle(Command::Activate(modifiers)),
        Outcome::Pick {
            id: "settings:account".into(),
            modifiers
        }
    );
}

// keeps: spec ux-prototype §5.4
#[test]
fn each_down_provider_state_refuses_both_pick_and_dive() {
    for state in [
        HostState::Offline,
        HostState::Disconnected {
            reason: "fixture disconnected".into(),
        },
    ] {
        let mut control = controller();
        control.replace_items(catalog(&model(state)));
        control.handle(Command::Query("ssh".into()));
        control.handle(Command::Right);
        for command in [Command::Right, Command::Activate(Modifiers::default())] {
            assert_eq!(control.handle(command), Outcome::HostUnavailable);
            assert_eq!(control.launcher.path, ["ssh"]);
        }
    }
    let mut control = controller();
    control.replace_items(catalog(&model(HostState::Degraded { rtt_ms: 380 })));
    control.handle(Command::Query("ssh rig cod".into()));
    assert!(matches!(
        control.handle(Command::Activate(Modifiers::default())),
        Outcome::Pick { .. }
    ));
}

// keeps: spec ux-prototype §5.4
#[test]
fn a_connecting_host_keeps_the_existing_browseable_projection() {
    let mut control = controller();
    control.replace_items(catalog(&model(HostState::Connecting)));
    control.handle(Command::Query("ssh".into()));
    assert_eq!(control.handle(Command::Right), Outcome::Changed);
    assert_eq!(selected(&control), Some("ssh:rig"));
    assert_eq!(control.handle(Command::Right), Outcome::Changed);
    assert_eq!(control.launcher.path, ["ssh", "ssh:rig"]);
    // Actual spawning is a separate bounded dispatch, guarded by the provider.
}

// keeps: spec ux-prototype §5.4
#[test]
fn link_state_blocks_activation_even_without_the_legacy_down_flag() {
    let mut control = controller();
    control.launcher.items[0].link = Some(LinkState::Disconnected {
        reason: "lost".into(),
    });
    assert!(!control.launcher.items[0].down);
    assert_eq!(
        control.handle(Command::Activate(Modifiers::default())),
        Outcome::HostUnavailable
    );
    assert_eq!(control.handle(Command::Right), Outcome::HostUnavailable);
}

// keeps: spec ux-prototype §5.4
#[test]
fn direct_activation_outside_the_rows_is_local_and_preserves_selection() {
    let mut control = controller();
    assert_eq!(
        control.handle(Command::ActivateAt {
            index: 99,
            modifiers: Modifiers::default()
        }),
        Outcome::Local
    );
    assert_eq!(selected(&control), Some("codex"));
    control.launcher.selected = Some(99);
    assert_eq!(
        control.handle(Command::Activate(Modifiers::default())),
        Outcome::Local
    );
}
