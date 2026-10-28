// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use std::time::{Duration, Instant};

use bbt_ui_model::event::Update;
use bbt_ui_model::*;

fn seat(id: u128) -> SeatId {
    SeatId::from_u128(id)
}
fn swarm(id: u128) -> SwarmId {
    SwarmId::from_u128(id)
}
fn tab(id: u128) -> TabId {
    TabId::from_u128(id)
}
fn leaf(id: u128) -> SplitNode {
    SplitNode::Leaf(seat(id))
}
fn tree() -> SplitNode {
    SplitNode::split(
        Axis::Row,
        0.6,
        leaf(1),
        SplitNode::split(Axis::Column, 0.3, leaf(2), leaf(3)),
    )
}
fn fixture() -> Model {
    let swarms = vec![Swarm::new(
        swarm(1),
        "term",
        vec![
            Tab::new(tab(1), "core", tree()),
            Tab::new(tab(2), "shell", leaf(4)),
        ],
    )];
    let seats = (1..=4)
        .map(|i| {
            Seat::new(
                seat(i),
                swarm(1),
                [i as u8; 32],
                format!("Seat {i}"),
                "codex",
            )
        })
        .collect();
    Model::new(swarms, seats).unwrap()
}

// keeps: spec gui-components §1.4 and §7.2 — model edits preserve tree and lifecycle contracts
#[test]
fn prune_parent_and_replacement_match_lab_tree_algebra() {
    let original = tree();
    assert_eq!(prune(&leaf(1), seat(1)), None);
    let parent = parent_of(&original, seat(2)).unwrap();
    assert_eq!(parent.path, vec![Branch::B]);
    assert_eq!(parent.dir, Axis::Column);
    assert_eq!(parent.side, Branch::A);
    assert_eq!(parent.sibling, leaf(3));
    assert_eq!(
        prune(&original, seat(2)).unwrap(),
        SplitNode::split(Axis::Row, 0.6, leaf(1), leaf(3))
    );
    assert!(parent_of(&leaf(1), seat(1)).is_none());
    let replacement = replace_node(&original, &[Branch::B, Branch::A], leaf(4)).unwrap();
    assert_eq!(replacement.leaves(), vec![seat(1), seat(4), seat(3)]);
    assert_eq!(original.leaves(), vec![seat(1), seat(2), seat(3)]);
    assert_eq!(replace_node(&original, &[], leaf(9)), Some(leaf(9)));
    assert!(replace_node(&original, &[Branch::A, Branch::A], leaf(9)).is_none());
}

// keeps: spec gui-components §1.4 and §7.2 — model edits preserve tree and lifecycle contracts
#[test]
fn detach_removes_empty_tabs_without_touching_other_seats() {
    let model = fixture();
    let detached = detach(&model, seat(4)).unwrap();
    assert_eq!(detached.window.sidebar.swarms[0].tabs.len(), 1);
    assert_eq!(detached.window.sidebar.swarms[0].tabs[0].tree, tree());
    assert_eq!(detached.seats.len(), 4);
    assert_eq!(model.window.sidebar.swarms[0].tabs.len(), 2);
}

// keeps: spec gui-components §1.4 and §7.2 — model edits preserve tree and lifecycle contracts
#[test]
fn placement_on_all_four_sides_is_half_and_collapses_the_source() {
    for side in [Side::Left, Side::Right, Side::Top, Side::Bottom] {
        let before = fixture();
        let placed = place(
            &before,
            seat(4),
            Placement::Split {
                target: seat(2),
                side,
            },
        )
        .unwrap();
        let tabs = &placed.window.sidebar.swarms[0].tabs;
        assert_eq!(tabs.len(), 1);
        let path = parent_of(&tabs[0].tree, seat(4)).unwrap();
        assert_eq!(path.dir, side.axis());
        assert_eq!(path.side, side.branch());
        assert_eq!(path.ratio, 0.5);
        assert_eq!(tabs[0].seats, vec![seat(1), seat(2), seat(3), seat(4)]);
        assert_eq!(placed.window.stage.focused, Some(seat(4)));
        assert_eq!(before.window.sidebar.swarms[0].tabs.len(), 2);
    }
}

// keeps: spec gui-components §1.4 and §7.2 — model edits preserve tree and lifecycle contracts
#[test]
fn move_within_one_split_recomputes_the_target_after_pruning() {
    let model = fixture();
    let moved = place(
        &model,
        seat(1),
        Placement::Split {
            target: seat(3),
            side: Side::Bottom,
        },
    )
    .unwrap();
    let tree = &moved.window.sidebar.swarms[0].tabs[0].tree;
    assert_eq!(tree.leaves(), vec![seat(2), seat(3), seat(1)]);
    assert_eq!(parent_of(tree, seat(1)).unwrap().ratio, 0.5);
    assert_eq!(
        place(
            &model,
            seat(1),
            Placement::Split {
                target: seat(1),
                side: Side::Left
            }
        )
        .unwrap(),
        model
    );
}

// keeps: spec gui-components §1.4 and §7.2 — model edits preserve tree and lifecycle contracts
#[test]
fn new_tab_uses_caller_id_and_preserves_the_remaining_order() {
    let model = fixture();
    let next = place(
        &model,
        seat(2),
        Placement::Tab {
            swarm: swarm(1),
            tab: tab(3),
        },
    )
    .unwrap();
    let tabs = &next.window.sidebar.swarms[0].tabs;
    assert_eq!(
        tabs.iter().map(|t| t.id).collect::<Vec<_>>(),
        vec![tab(1), tab(2), tab(3)]
    );
    assert_eq!(tabs[2].tree, leaf(2));
    assert_eq!(tabs[2].shortcut, Some(3));
    assert_eq!(next.window.stage.tab, Some(tab(3)));
}

// keeps: spec ux-prototype §5.3 — exact close/undo restoration and honest refusal
#[test]
fn close_and_undo_restore_the_exact_split_and_ratio() {
    let model = fixture();
    let now = Instant::now();
    for id in [seat(1), seat(2), seat(3)] {
        let closed = close(&model, id, now, Duration::from_secs(60)).unwrap();
        assert!(!closed.window.sidebar.swarms[0].tabs[0].tree.contains(id));
        assert!(matches!(
            closed.seats[&id].lifecycle,
            Lifecycle::ClosedGrace { .. }
        ));
        let restored = undo(&closed, id, now + Duration::from_secs(59)).unwrap();
        assert_eq!(restored.window.sidebar.swarms[0].tabs[0].tree, tree());
        assert!(restored.window.sidebar.swarms[0].closed.is_empty());
        assert_eq!(restored.seats[&id].lifecycle, Lifecycle::Open);
        assert_eq!(restored.window.stage.focused, Some(id));
    }
}

// keeps: spec ux-prototype §5.3 — exact close/undo restoration and honest refusal
#[test]
fn undo_restores_a_removed_tab_at_its_original_index() {
    let mut model = fixture();
    model.window.sidebar.swarms[0].tabs[1].shortcut = Some(8);
    let now = Instant::now();
    let closed = close(&model, seat(4), now, Duration::from_secs(60)).unwrap();
    let restored = undo(&closed, seat(4), now).unwrap();
    assert_eq!(
        restored.window.sidebar.swarms[0].tabs,
        model.window.sidebar.swarms[0].tabs
    );
}

// keeps: spec ux-prototype §5.3 — exact close/undo restoration and honest refusal
#[test]
fn reverse_undo_restores_multiple_closed_seats() {
    let model = fixture();
    let now = Instant::now();
    let grace = Duration::from_secs(60);
    let one = close(&model, seat(1), now, grace).unwrap();
    let two = close(&one, seat(2), now, grace).unwrap();
    assert_eq!(
        undo(&two, seat(1), now).unwrap_err(),
        EditError::LayoutConflict
    );
    let restored = undo(&undo(&two, seat(2), now).unwrap(), seat(1), now).unwrap();
    assert_eq!(restored.window.sidebar.swarms[0].tabs[0].tree, tree());
}

// keeps: spec ux-prototype §5.3 — exact close/undo restoration and honest refusal
#[test]
fn undo_keeps_a_sibling_subtrees_intervening_resize() {
    let model = fixture();
    let now = Instant::now();
    let mut closed = close(&model, seat(1), now, Duration::from_secs(60)).unwrap();
    if let SplitNode::Split { ratio, .. } = &mut closed.window.sidebar.swarms[0].tabs[0].tree {
        *ratio = 0.7;
    }
    let restored = undo(&closed, seat(1), now).unwrap();
    assert_eq!(
        restored.window.sidebar.swarms[0].tabs[0].tree,
        SplitNode::split(
            Axis::Row,
            0.6,
            leaf(1),
            SplitNode::split(Axis::Column, 0.7, leaf(2), leaf(3))
        )
    );
}

// keeps: spec ux-prototype §5.3 — exact close/undo restoration and honest refusal
#[test]
fn expired_replaced_and_conflicting_undo_do_not_mutate_the_model() {
    let model = fixture();
    let now = Instant::now();
    let grace = Duration::from_secs(60);
    let closed = close(&model, seat(2), now, grace).unwrap();
    assert_eq!(
        undo(&closed, seat(2), now + grace).unwrap_err(),
        EditError::Expired
    );
    let mut replaced = closed.clone();
    replaced
        .apply(Update::SeatGeneration {
            seat: seat(2),
            generation: 2,
        })
        .unwrap();
    assert_eq!(
        undo(&replaced, seat(2), now).unwrap_err(),
        EditError::ProcessReplaced
    );
    let conflict = detach(&closed, seat(3)).unwrap();
    assert_eq!(
        undo(&conflict, seat(2), now).unwrap_err(),
        EditError::LayoutConflict
    );
    assert_eq!(closed.window.sidebar.swarms[0].closed.len(), 1);
    let mut retained = expire_close(&closed, seat(2), now + grace).unwrap();
    assert!(matches!(
        retained.seats[&seat(2)].lifecycle,
        Lifecycle::ClosedGrace { .. }
    ));
    assert_eq!(
        undo(&retained, seat(2), now + grace).unwrap_err(),
        EditError::Expired
    );
    retained
        .apply(Update::SeatClosedByDaemon { seat: seat(2) })
        .unwrap();
    assert_eq!(
        retained.seats[&seat(2)].lifecycle,
        Lifecycle::ClosedRetained
    );
    assert_eq!(
        retained.seats.len(),
        4,
        "retained seat history is not deleted"
    );
}

// keeps: spec gui-components §1.4 and §7.2 — model edits preserve tree and lifecycle contracts
#[test]
fn mux_updates_do_not_locate_or_steal_focus() {
    let mut model = fixture();
    let stage = model.window.stage.clone();
    model
        .apply(Update::SeatActivity {
            seat: seat(2),
            activity: Activity::Waiting(WaitKind::Auth),
            grade: Grade::Reported,
        })
        .unwrap();
    model
        .apply(Update::MessageArrived {
            seat: seat(3),
            unread: 7,
            role: Role {
                name: "custom".into(),
                class: RoleClass::Worker,
            },
        })
        .unwrap();
    assert_eq!(model.window.stage, stage);
    assert_eq!(
        model.seats[&seat(2)].activity,
        Activity::Waiting(WaitKind::Auth)
    );
    assert_eq!(model.seats[&seat(3)].unread.as_ref().unwrap().0, 7);
    model
        .apply(Update::SeatClosedByDaemon { seat: seat(2) })
        .unwrap();
    assert_eq!(model.seats[&seat(2)].lifecycle, Lifecycle::ClosedRetained);
    assert!(
        !model.window.sidebar.swarms[0].tabs[0]
            .tree
            .contains(seat(2))
    );
}

// keeps: spec ux-prototype §5.3 — exact close/undo restoration and honest refusal
#[test]
fn settings_closes_directly_without_harness_grace() {
    let mut model = fixture();
    model.seats.get_mut(&seat(4)).unwrap().settings = true;
    let closed = close(&model, seat(4), Instant::now(), Duration::from_secs(60)).unwrap();
    assert!(!closed.seats.contains_key(&seat(4)));
    assert!(closed.window.sidebar.swarms[0].closed.is_empty());
    assert_eq!(closed.window.sidebar.swarms[0].tabs.len(), 1);
}

// keeps: spec gui-components §1.4 and §7.2 — model edits preserve tree and lifecycle contracts
#[test]
fn cross_swarm_move_is_an_explicit_error_per_grant_contract() {
    let mut model = fixture();
    model.window.sidebar.swarms.push(Swarm::new(
        swarm(2),
        "other",
        vec![Tab::new(tab(9), "other", leaf(9))],
    ));
    model.seats.insert(
        seat(9),
        Seat::new(seat(9), swarm(2), [9; 32], "Other", "custom-harness"),
    );
    assert_eq!(
        place(
            &model,
            seat(1),
            Placement::Split {
                target: seat(9),
                side: Side::Right
            }
        )
        .unwrap_err(),
        EditError::CrossSwarm
    );
    assert_eq!(
        place(
            &model,
            seat(1),
            Placement::Tab {
                swarm: swarm(2),
                tab: tab(10)
            }
        )
        .unwrap_err(),
        EditError::CrossSwarm
    );
}

// keeps: spec gui-components §1.4 and §7.2 — model edits preserve tree and lifecycle contracts
#[test]
fn duplicate_and_mismatched_tree_ids_are_rejected() {
    let duplicate = SplitNode::split(Axis::Row, 0.5, leaf(1), leaf(1));
    let seats = vec![Seat::new(seat(1), swarm(1), [1; 32], "one", "custom")];
    assert_eq!(
        Model::new(
            vec![Swarm::new(
                swarm(1),
                "bad",
                vec![Tab::new(tab(1), "bad", duplicate)]
            )],
            seats
        )
        .unwrap_err(),
        EditError::InvalidTree
    );
}

// keeps: spec gui-components §1.4 and §7.2 — model edits preserve tree and lifecycle contracts
#[test]
fn open_harness_role_class_and_color_override_survive_placement() {
    let mut model = fixture();
    let s = model.seats.get_mut(&seat(1)).unwrap();
    s.harness = HarnessId("my-harness-v2".into());
    s.role = Some(Role {
        name: "cartographer".into(),
        class: RoleClass::Oracle,
    });
    s.where_ = Where::Cloud {
        provider: "custom".into(),
        host: "region-1".into(),
    };
    let color = Hsl {
        hue: 120.0,
        saturation: 50.0,
        lightness: 40.0,
    };
    s.color = SeatColor::Override(color);
    let edited = place(
        &model,
        seat(1),
        Placement::Split {
            target: seat(3),
            side: Side::Right,
        },
    )
    .unwrap();
    let s = &edited.seats[&seat(1)];
    assert_eq!(s.harness.0, "my-harness-v2");
    assert_eq!(
        s.where_,
        Where::Cloud {
            provider: "custom".into(),
            host: "region-1".into()
        }
    );
    assert_eq!(s.color.resolve("any-key", Theme::Dark), color);
    assert_eq!(s.color.resolve("any-key", Theme::Light), color);
    assert_eq!(s.role.as_ref().unwrap().class, RoleClass::Oracle);
    let dark = SeatColor::Derived.resolve("sc0", Theme::Dark);
    let light = SeatColor::Derived.resolve("sc0", Theme::Light);
    assert_eq!(dark.hue, light.hue);
    assert_ne!(dark.lightness, light.lightness);
}
