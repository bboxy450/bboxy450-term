// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use std::time::{Duration, Instant};

use bbt_ui_model::{
    event::{Event, LocateWhere, Update},
    input::{
        Direction, Focus, FocusContext, FocusOrigin, KeyRoute, Modifiers, Navigation,
        NavigationKey, route_navigation,
    },
    layout::{Rect, solve_split},
    *,
};

fn seat(n: u128) -> SeatId {
    SeatId::from_u128(n)
}
fn swarm(n: u128) -> SwarmId {
    SwarmId::from_u128(n)
}
fn tab(n: u128) -> TabId {
    TabId::from_u128(n)
}
fn leaf(n: u128) -> SplitNode {
    SplitNode::Leaf(seat(n))
}
fn fixture() -> Model {
    let tree = SplitNode::split(
        Axis::Row,
        0.5,
        leaf(1),
        SplitNode::split(Axis::Column, 0.5, leaf(2), leaf(3)),
    );
    let mut primary = Tab::new(tab(1), "primary", tree);
    primary.seats.reverse(); // Focus cycling must use tree order.
    let mut model = Model::new(
        vec![
            Swarm::new(
                swarm(1),
                "one",
                vec![primary, Tab::new(tab(2), "other", leaf(4))],
            ),
            Swarm::new(swarm(2), "two", vec![Tab::new(tab(3), "remote", leaf(5))]),
        ],
        (1..=6)
            .map(|id| {
                Seat::new(
                    seat(id),
                    if id == 5 { swarm(2) } else { swarm(1) },
                    [id as u8; 32],
                    format!("Seat {id}"),
                    "zsh",
                )
            })
            .collect(),
    )
    .unwrap();
    model.window.stage.focused = Some(seat(1));
    model
}
fn panes(model: &Model) -> Vec<(SeatId, Rect)> {
    let s = model
        .window
        .sidebar
        .swarms
        .iter()
        .find(|s| Some(s.id) == model.window.stage.swarm)
        .unwrap();
    let t = s
        .tabs
        .iter()
        .find(|t| Some(t.id) == model.window.stage.tab)
        .unwrap();
    solve_split(&t.tree, Rect::new(0., 0., 402., 400.), 2.).panes
}
fn changed(to: u128, from: u128, switched: bool) -> Event {
    Event::FocusChanged {
        seat: seat(to),
        prev: Some(seat(from)),
        stage_switched: switched,
    }
}
fn locate(id: u128, where_: LocateWhere) -> Event {
    Event::Locate {
        seat: seat(id),
        where_,
    }
}

// keeps: spec gui-components §4.2 and §7.8 — caller-visible focus, locate and key routing
#[test]
fn row_and_pane_activation_locate_the_other_view_without_changing_expansion() {
    let mut model = fixture();
    let mut focus = Focus::default();
    let now = Instant::now();
    let open = model.window.sidebar.open.clone();
    assert_eq!(
        focus
            .activate(&mut model, seat(2), FocusOrigin::Row, now)
            .unwrap(),
        vec![changed(2, 1, false), locate(2, LocateWhere::Pane)]
    );
    assert_eq!(
        focus
            .activate(&mut model, seat(3), FocusOrigin::Pane, now)
            .unwrap(),
        vec![changed(3, 2, false), locate(3, LocateWhere::Row)]
    );
    // Clicking an already focused pane still locates its row, once.
    assert_eq!(
        focus
            .activate(&mut model, seat(3), FocusOrigin::Pane, now)
            .unwrap(),
        vec![locate(3, LocateWhere::Row)]
    );
    assert_eq!(model.window.sidebar.open, open);
}

// keeps: spec gui-components §4.2 and §7.8 — caller-visible focus, locate and key routing
#[test]
fn cross_stage_locate_waits_for_the_target_layout_and_fires_once() {
    let mut model = fixture();
    let mut focus = Focus::default();
    let now = Instant::now();
    let old_panes = panes(&model);
    let open = model.window.sidebar.open.clone();
    assert_eq!(
        focus
            .activate(&mut model, seat(5), FocusOrigin::Row, now)
            .unwrap(),
        vec![changed(5, 1, true)]
    );
    assert_eq!(model.window.stage.swarm, Some(swarm(2)));
    assert_eq!(model.window.stage.tab, Some(tab(3)));
    assert_eq!(model.window.sidebar.open, open);
    assert!(focus.layout_ready(&model, &old_panes, now).is_empty());
    assert_eq!(
        focus.layout_ready(&model, &panes(&model), now),
        vec![locate(5, LocateWhere::Pane)]
    );
    assert!(focus.layout_ready(&model, &panes(&model), now).is_empty());
}

// keeps: spec gui-components §4.2 and §7.8 — caller-visible focus, locate and key routing
#[test]
fn a_second_activation_does_not_bypass_the_pending_stage_layout() {
    let mut model = fixture();
    let mut focus = Focus::default();
    let now = Instant::now();
    let old = panes(&model);
    focus
        .activate(&mut model, seat(5), FocusOrigin::Row, now)
        .unwrap();
    assert!(
        focus
            .activate(&mut model, seat(5), FocusOrigin::Pane, now)
            .unwrap()
            .is_empty()
    );
    assert!(focus.layout_ready(&model, &old, now).is_empty());
    assert_eq!(
        focus.layout_ready(&model, &panes(&model), now),
        vec![locate(5, LocateWhere::Row)]
    );
}

// keeps: spec gui-components §4.2 and §7.8 — caller-visible focus, locate and key routing
#[test]
fn cycle_uses_tree_order_wraps_and_never_switches_a_tab_or_swarm() {
    let mut model = fixture();
    let mut focus = Focus::default();
    let now = Instant::now();
    for (to, from, back) in [(2, 1, false), (3, 2, false), (1, 3, false), (3, 1, true)] {
        assert_eq!(
            focus.cycle(&mut model, back, now).unwrap(),
            vec![changed(to, from, false), locate(to, LocateWhere::Pane)]
        );
        assert_eq!(model.window.stage.tab, Some(tab(1)));
    }
}

// keeps: spec gui-components §4.2 and §7.8 — caller-visible focus, locate and key routing
#[test]
fn directional_navigation_scores_along_plus_twice_across_and_breaks_ties_in_tree_order() {
    let mut model = fixture();
    let mut focus = Focus::default();
    let now = Instant::now();
    let mut rects = panes(&model);
    rects.reverse();
    // Both right-hand panes have the same centre distance; reverse rect input
    // order cannot change the split-tree tie break. The other tab is excluded.
    rects.push((seat(4), Rect::new(202., 150., 200., 100.)));
    assert_eq!(
        focus
            .navigate(&mut model, &rects, Direction::Right, now)
            .unwrap(),
        vec![changed(2, 1, false), locate(2, LocateWhere::Pane)]
    );
    assert_eq!(
        focus
            .navigate(&mut model, &rects, Direction::Down, now)
            .unwrap(),
        vec![changed(3, 2, false), locate(3, LocateWhere::Pane)]
    );
    assert_eq!(
        focus
            .navigate(&mut model, &rects, Direction::Up, now)
            .unwrap(),
        vec![changed(2, 3, false), locate(2, LocateWhere::Pane)]
    );
    assert_eq!(
        focus
            .navigate(&mut model, &rects, Direction::Left, now)
            .unwrap(),
        vec![changed(1, 2, false), locate(1, LocateWhere::Pane)]
    );
    assert!(
        focus
            .navigate(&mut model, &rects, Direction::Left, now)
            .unwrap()
            .is_empty()
    );
    // A side-by-side pane can have a lower centre while overlapping vertically;
    // that is not a pane below the current one.
    assert!(
        focus
            .navigate(&mut model, &rects, Direction::Down, now)
            .unwrap()
            .is_empty()
    );
}

// keeps: spec gui-components §4.2 and §7.8 — caller-visible focus, locate and key routing
#[test]
fn directional_score_prefers_alignment_over_the_nearest_centre() {
    let tree = SplitNode::split(
        Axis::Row,
        100. / 302.,
        leaf(1),
        SplitNode::split(
            Axis::Row,
            0.5,
            SplitNode::split(Axis::Column, 0.5, leaf(2), leaf(3)),
            leaf(4),
        ),
    );
    let rects = solve_split(&tree, Rect::new(0., 0., 304., 204.), 2.).panes;
    let mut model = Model::new(
        vec![Swarm::new(
            swarm(1),
            "one",
            vec![Tab::new(tab(1), "staggered", tree)],
        )],
        (1..=4)
            .map(|id| {
                Seat::new(
                    seat(id),
                    swarm(1),
                    [id as u8; 32],
                    format!("Seat {id}"),
                    "zsh",
                )
            })
            .collect(),
    )
    .unwrap();
    // The upper/lower middle panes are closer by centre distance, but their
    // across-axis penalty puts the aligned far pane first.
    assert_eq!(
        Focus::default()
            .navigate(&mut model, &rects, Direction::Right, Instant::now())
            .unwrap(),
        vec![changed(4, 1, false), locate(4, LocateWhere::Pane)]
    );
}

// keeps: spec gui-components §4.2 and §7.8 — caller-visible focus, locate and key routing
#[test]
fn dwell_needs_150_ms_on_a_pane_and_never_locates_or_repeats() {
    let mut model = fixture();
    let mut focus = Focus::default();
    let now = Instant::now();
    let rects = panes(&model);
    focus.pointer_moved(&rects, [-10., 20.], now); // Sidebar/outside panes.
    assert!(
        focus
            .tick(&mut model, &rects, now + Duration::from_secs(1))
            .unwrap()
            .is_empty()
    );
    focus.pointer_moved(&rects, [250., 20.], now);
    assert!(
        focus
            .tick(&mut model, &rects, now + Duration::from_millis(149))
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        focus
            .tick(&mut model, &rects, now + Duration::from_millis(150))
            .unwrap(),
        vec![changed(2, 1, false)]
    );
    assert!(
        focus
            .tick(&mut model, &rects, now + Duration::from_secs(2))
            .unwrap()
            .is_empty()
    );
}

// keeps: spec gui-components §4.2 and §7.8 — caller-visible focus, locate and key routing
#[test]
fn typing_guard_is_strict_and_a_blocked_dwell_waits_for_reentry() {
    let mut model = fixture();
    let mut focus = Focus::default();
    let now = Instant::now();
    let rects = panes(&model);
    focus.key_pressed(now);
    focus.pointer_moved(&rects, [250., 20.], now + Duration::from_millis(850));
    assert!(
        focus
            .tick(&mut model, &rects, now + Duration::from_secs(1))
            .unwrap()
            .is_empty()
    );
    // The expired timer was consumed. Merely waiting out the guard or moving
    // within the same pane does not focus it later.
    focus.pointer_moved(&rects, [251., 21.], now + Duration::from_millis(1100));
    assert!(
        focus
            .tick(&mut model, &rects, now + Duration::from_secs(2))
            .unwrap()
            .is_empty()
    );
    focus.pointer_left();
    focus.pointer_moved(&rects, [250., 20.], now + Duration::from_secs(2));
    assert_eq!(
        focus
            .tick(&mut model, &rects, now + Duration::from_millis(2150))
            .unwrap(),
        vec![changed(2, 1, false)]
    );
}

// keeps: spec gui-components §4.2 and §7.8 — caller-visible focus, locate and key routing
#[test]
fn layout_changes_rehit_the_pointer_and_leave_cancels_dwell() {
    let mut model = fixture();
    let mut focus = Focus::default();
    let now = Instant::now();
    let rects = panes(&model);
    focus.pointer_moved(&rects, [250., 20.], now);
    let moved = vec![
        (seat(1), rects[0].1),
        (seat(3), rects[1].1),
        (seat(2), rects[2].1),
    ];
    assert!(
        focus
            .tick(&mut model, &moved, now + Duration::from_millis(150))
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        focus
            .tick(&mut model, &moved, now + Duration::from_millis(300))
            .unwrap(),
        vec![changed(3, 1, false)]
    );
    focus.pointer_moved(&rects, [250., 20.], now + Duration::from_secs(1));
    focus.pointer_left();
    assert!(
        focus
            .tick(&mut model, &rects, now + Duration::from_secs(2))
            .unwrap()
            .is_empty()
    );
}

// keeps: spec gui-components §4.2 and §7.8 — caller-visible focus, locate and key routing
#[test]
fn daemon_updates_do_not_focus_or_locate_and_cancel_stale_user_intent() {
    let mut model = fixture();
    let mut focus = Focus::default();
    let now = Instant::now();
    model
        .apply(Update::SeatActivity {
            seat: seat(2),
            activity: Activity::Active,
            grade: Grade::Reported,
        })
        .unwrap();
    assert_eq!(model.window.stage.focused, Some(seat(1)));
    assert!(focus.layout_ready(&model, &panes(&model), now).is_empty());
    focus
        .activate(&mut model, seat(4), FocusOrigin::Keyboard, now)
        .unwrap();
    model
        .apply(Update::SeatClosedByDaemon { seat: seat(4) })
        .unwrap();
    assert!(focus.layout_ready(&model, &panes(&model), now).is_empty());
}

// keeps: spec gui-components §4.2 and §7.8 — caller-visible focus, locate and key routing
#[test]
fn invalid_activation_preserves_the_model_and_pending_locate() {
    let mut model = fixture();
    let mut focus = Focus::default();
    let now = Instant::now();
    focus
        .activate(&mut model, seat(4), FocusOrigin::Keyboard, now)
        .unwrap();
    let before = model.clone();
    assert_eq!(
        focus.activate(&mut model, seat(999), FocusOrigin::Row, now),
        Err(EditError::MissingSeat)
    );
    assert_eq!(
        focus.activate(&mut model, seat(6), FocusOrigin::Row, now),
        Err(EditError::MissingTab)
    );
    assert_eq!(model, before);
    assert_eq!(
        focus.layout_ready(&model, &panes(&model), now),
        vec![locate(4, LocateWhere::Pane)]
    );
}

// keeps: spec gui-components §4.2 and §7.8 — caller-visible focus, locate and key routing
#[test]
fn focus_change_clears_unread_once_without_mutating_the_activity_signal() {
    let mut model = fixture();
    let mut focus = Focus::default();
    let now = Instant::now();
    model
        .apply(Update::MessageArrived {
            seat: seat(2),
            unread: 3,
            role: Role {
                name: "reviewer".into(),
                class: RoleClass::Worker,
            },
        })
        .unwrap();
    let activity = model.seats[&seat(2)].activity;
    assert_eq!(
        focus
            .activate(&mut model, seat(2), FocusOrigin::Row, now)
            .unwrap(),
        vec![
            changed(2, 1, false),
            Event::UnreadCleared { seat: seat(2) },
            locate(2, LocateWhere::Pane)
        ]
    );
    assert!(model.seats[&seat(2)].unread.is_none());
    assert_eq!(model.seats[&seat(2)].activity, activity);
    assert_eq!(
        focus
            .activate(&mut model, seat(2), FocusOrigin::Row, now)
            .unwrap(),
        vec![locate(2, LocateWhere::Pane)]
    );
    model
        .apply(Update::MessageArrived {
            seat: seat(2),
            unread: 1,
            role: Role {
                name: "reviewer".into(),
                class: RoleClass::Worker,
            },
        })
        .unwrap();
    // A new unread notification can arrive while this pane is focused.
    // Clicking it reads that notification without emitting FocusChanged.
    assert_eq!(
        focus
            .activate(&mut model, seat(2), FocusOrigin::Pane, now)
            .unwrap(),
        vec![
            Event::UnreadCleared { seat: seat(2) },
            locate(2, LocateWhere::Row)
        ]
    );
    assert!(model.seats[&seat(2)].unread.is_none());
}

// keeps: spec gui-components §4.2 and §7.8 — caller-visible focus, locate and key routing
#[test]
fn terminal_plain_keys_and_control_or_alt_keys_never_become_pane_navigation() {
    for bits in 0..8 {
        let mods = Modifiers {
            control: bits & 1 != 0,
            alt: bits & 2 != 0,
            shift: bits & 4 != 0,
            ..Default::default()
        };
        for key in [
            NavigationKey::Tab,
            NavigationKey::Other,
            NavigationKey::Arrow(Direction::Left),
            NavigationKey::Arrow(Direction::Right),
            NavigationKey::Arrow(Direction::Up),
            NavigationKey::Arrow(Direction::Down),
        ] {
            assert_eq!(
                route_navigation(key, mods, FocusContext::Terminal),
                KeyRoute::Terminal
            );
        }
    }
}

// keeps: spec gui-components §4.2 and §7.8 — caller-visible focus, locate and key routing
#[test]
fn terminal_navigation_needs_command_option_and_keeps_reverse_tree_order_explicit() {
    let mods = Modifiers {
        command: true,
        alt: true,
        ..Default::default()
    };
    for direction in [
        Direction::Left,
        Direction::Right,
        Direction::Up,
        Direction::Down,
    ] {
        assert_eq!(
            route_navigation(
                NavigationKey::Arrow(direction),
                mods,
                FocusContext::Terminal
            ),
            KeyRoute::Pane(Navigation::Direction(direction))
        );
    }
    assert_eq!(
        route_navigation(NavigationKey::Tab, mods, FocusContext::Terminal),
        KeyRoute::Pane(Navigation::Cycle { backwards: false })
    );
    assert_eq!(
        route_navigation(
            NavigationKey::Tab,
            Modifiers {
                shift: true,
                ..mods
            },
            FocusContext::Terminal
        ),
        KeyRoute::Pane(Navigation::Cycle { backwards: true })
    );
    assert_eq!(
        route_navigation(NavigationKey::Other, mods, FocusContext::Terminal),
        KeyRoute::App
    );
    assert_eq!(
        route_navigation(
            NavigationKey::Tab,
            Modifiers {
                command: true,
                ..Default::default()
            },
            FocusContext::Terminal
        ),
        KeyRoute::App
    );
}

// keeps: spec gui-components §4.2 and §7.8 — caller-visible focus, locate and key routing
#[test]
fn chrome_can_navigate_plainly_while_fields_and_modal_controls_keep_local_keys() {
    let plain = Modifiers::default();
    assert_eq!(
        route_navigation(
            NavigationKey::Arrow(Direction::Right),
            plain,
            FocusContext::Chrome
        ),
        KeyRoute::Pane(Navigation::Direction(Direction::Right))
    );
    assert_eq!(
        route_navigation(
            NavigationKey::Tab,
            Modifiers {
                shift: true,
                ..plain
            },
            FocusContext::Chrome
        ),
        KeyRoute::Pane(Navigation::Cycle { backwards: true })
    );
    for context in [
        FocusContext::TextField,
        FocusContext::Launcher,
        FocusContext::Overlay,
    ] {
        for key in [
            NavigationKey::Tab,
            NavigationKey::Other,
            NavigationKey::Arrow(Direction::Left),
        ] {
            assert_eq!(route_navigation(key, plain, context), KeyRoute::Local);
        }
    }
    let explicit = Modifiers {
        command: true,
        alt: true,
        ..plain
    };
    assert_eq!(
        route_navigation(
            NavigationKey::Arrow(Direction::Left),
            explicit,
            FocusContext::TextField
        ),
        KeyRoute::Pane(Navigation::Direction(Direction::Left))
    );
    for context in [FocusContext::Launcher, FocusContext::Overlay] {
        assert_eq!(
            route_navigation(NavigationKey::Arrow(Direction::Left), explicit, context),
            KeyRoute::Local
        );
    }
}
