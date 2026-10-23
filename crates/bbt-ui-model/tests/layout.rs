use bbt_ui_model::layout::*;
use bbt_ui_model::*;

fn near(actual: f32, expected: f32) {
    assert!((actual - expected).abs() < 0.002, "{actual} != {expected}");
}
fn leaf(id: u128) -> SplitNode {
    SplitNode::Leaf(SeatId::from_u128(id))
}

fn fixture_tree(value: &serde_json::Value) -> SplitNode {
    if let Some(seat) = value.get("seat") {
        return leaf(seat.as_u64().unwrap() as u128 + 1);
    }
    let axis = if value["axis"] == "V" {
        Axis::Row
    } else {
        Axis::Column
    };
    SplitNode::split(
        axis,
        value["ratio"].as_str().unwrap().parse().unwrap(),
        fixture_tree(&value["a"]),
        fixture_tree(&value["b"]),
    )
}

#[test]
fn scene_fifteen_matches_the_tagged_lab_ratios_at_the_fixture_gutter() {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/scene15.json")).unwrap();
    let tree = fixture_tree(&fixture["tree"]);
    let bounds = Rect::new(0.0, 0.0, 1600.0, 900.0);
    // The tagged lab rects were captured at a 2 px gutter; the solver is pinned
    // to that reference, which is also the live fixed gutter (next test).
    let gutter = fixture["gutter"].as_f64().unwrap() as f32;
    assert_eq!(gutter, 2.0);
    let solved = solve_split(&tree, bounds, gutter);
    assert_eq!(solved.panes.len(), 15);
    assert_eq!(solved.dividers.len(), 14);
    for (seat, actual) in &solved.panes {
        let expected = &fixture["panes"][(seat.0.0 - 1).to_string()];
        for (actual, expected) in [actual.x, actual.y, actual.w, actual.h]
            .into_iter()
            .zip(expected.as_array().unwrap())
        {
            near(actual, expected.as_f64().unwrap() as f32);
        }
        assert!(actual.x >= 0.0 && actual.y >= 0.0);
        assert!(
            actual.right() <= bounds.right() + 0.002 && actual.bottom() <= bounds.bottom() + 0.002
        );
    }
    for (i, (_, a)) in solved.panes.iter().enumerate() {
        for (_, b) in &solved.panes[i + 1..] {
            assert!(!a.intersects(*b));
        }
    }
    for divider in &solved.dividers {
        match divider.axis {
            Axis::Row => {
                near(divider.rect.w, 2.0);
                near(divider.hit_rect.w, 10.0);
            }
            Axis::Column => {
                near(divider.rect.h, 2.0);
                near(divider.hit_rect.h, 10.0);
            }
        }
    }
    let area = solved
        .panes
        .iter()
        .map(|(_, r)| r.w as f64 * r.h as f64)
        .sum::<f64>()
        + solved
            .dividers
            .iter()
            .map(|d| d.rect.w as f64 * d.rect.h as f64)
            .sum::<f64>();
    assert!(
        (area - 1_440_000.0).abs() < 0.1,
        "no outer margins or lost layout area: {area}"
    );
    eprintln!(
        "15 tagged-lab pane rectangles and 14 two-pixel gutters match the independent decimal fixture"
    );
}

#[test]
fn the_same_split_solver_supports_minigrid_and_banner_scales() {
    let tree = SplitNode::split(
        Axis::Row,
        0.6,
        leaf(1),
        SplitNode::split(Axis::Column, 0.5, leaf(2), leaf(3)),
    );
    let mini = solve_split(
        &tree,
        Rect::new(
            0.0,
            0.0,
            TOKENS.sidebar.minigrid_w as f32,
            TOKENS.sidebar.minigrid_h as f32,
        ),
        1.0,
    );
    near(mini.panes[0].1.w, 13.8);
    near(mini.panes[1].1.h, 7.5);
    let banner = solve_split(&tree, Rect::new(0.0, 0.0, 168.0, 104.0), 3.0);
    near(banner.panes[0].1.w, 99.0);
    near(banner.panes[1].1.h, 50.5);
}

#[test]
fn split_ratio_clamps_and_tiny_bounds_never_produce_negative_sizes() {
    let mut tree = SplitNode::split(Axis::Row, 0.5, leaf(1), leaf(2));
    if let SplitNode::Split { ratio, .. } = &mut tree {
        *ratio = 2.0;
    }
    let solved = solve_split(&tree, Rect::new(0.0, 0.0, 102.0, 20.0), 2.0);
    near(solved.panes[0].1.w, 90.0);
    near(solved.panes[1].1.w, 10.0);
    for bounds in [
        Rect::new(0.0, 0.0, 1.0, 0.0),
        Rect::new(0.0, 0.0, -2.0, f32::NAN),
    ] {
        for (_, rect) in solve_split(&tree, bounds, 2.0).panes {
            assert!(rect.w >= 0.0 && rect.h >= 0.0 && rect.w.is_finite() && rect.h.is_finite());
        }
    }
}

struct Mono;
impl TextMeasure for Mono {
    fn width(&self, text: &str, _: &TextStyle) -> f32 {
        text.chars().count() as f32 * 10.0
    }
}
fn text(value: &str) -> Child {
    Child {
        text: Some((value.into(), TextStyle::for_density(Density::Cozy))),
        cross: None,
    }
}

#[test]
fn row_box_allocates_fixed_measured_fill_and_weighted_flex() {
    let row = Box_ {
        axis: Axis::Row,
        pad: Insets::all(4.0),
        gap: 2.0,
        align: Align::Center,
        children: vec![
            (
                Size::Fixed(20.0),
                Child {
                    text: None,
                    cross: Some(10.0),
                },
            ),
            (Size::Text, text("0123456789")),
            (Size::Fill, Child::default()),
            (Size::Flex(3.0), Child::default()),
        ],
    };
    let rects = solve_box(&row, Rect::new(10.0, 20.0, 200.0, 40.0), &Mono);
    assert_eq!(rects[0], Rect::new(14.0, 35.0, 20.0, 10.0));
    near(rects[1].w, 100.0);
    near(rects[2].w, 16.5);
    near(rects[3].w, 49.5);
    near(rects[3].right(), 206.0);
}

#[test]
fn text_shrinks_before_flex_and_column_boxes_share_the_policy() {
    let mut column = Box_::new(
        Axis::Column,
        Density::Cozy,
        vec![
            (Size::Fixed(30.0), Child::default()),
            (Size::Text, text("abc")),
            (Size::Fill, Child::default()),
        ],
    );
    column.gap = 2.0;
    let rects = solve_box(&column, Rect::new(0.0, 0.0, 100.0, 40.0), &Mono);
    near(rects[0].h, 30.0);
    near(rects[1].h, 6.0);
    near(rects[2].h, 0.0);
    assert_eq!(rects[1].y, 32.0);
    let proportional = Box_ {
        axis: Axis::Row,
        gap: 0.0,
        pad: Insets::default(),
        align: Align::Stretch,
        children: vec![(Size::Text, text("abcd")), (Size::Text, text("abcdefgh"))],
    };
    let rects = solve_box(&proportional, Rect::new(0.0, 0.0, 60.0, 20.0), &Mono);
    near(rects[0].w, 20.0);
    near(rects[1].w, 40.0);
}

#[test]
fn invalid_lengths_and_zero_flex_weights_stay_finite() {
    let row = Box_ {
        axis: Axis::Row,
        gap: f32::NAN,
        pad: Insets::default(),
        align: Align::End,
        children: vec![
            (Size::Fixed(f32::NAN), Child::default()),
            (Size::Flex(-1.0), Child::default()),
            (Size::Fill, Child::default()),
        ],
    };
    let rects = solve_box(&row, Rect::new(0.0, 0.0, 100.0, 20.0), &Mono);
    assert_eq!(rects[0].w, 0.0);
    assert_eq!(rects[1].w, 0.0);
    assert_eq!(rects[2].w, 100.0);
    assert!(
        solve_box(
            &Box_::new(Axis::Row, Density::Cozy, Vec::new()),
            Rect::default(),
            &Mono
        )
        .is_empty()
    );
}

fn chrome() -> CardChrome {
    CardChrome {
        band: 24.0,
        pad_top: 0.0,
        pad_bottom: 0.0,
        bumper_open: 4.0,
        bumper_closed: 3.0,
    }
}

#[test]
fn sidebar_at_a_thousand_seats_only_allocates_the_visible_rows() {
    let card = SidebarCard {
        swarm: SwarmId::from_u128(1),
        open: true,
        rows: vec![RowRun {
            kind: RowKind::Seat,
            count: 1000,
        }],
    };
    let layout = solve_sidebar(
        &[card],
        Rect::new(0.0, 0.0, 260.0, 240.0),
        264.0,
        Density::Cozy,
        chrome(),
    );
    assert_eq!(layout.total_height, 24028.0);
    assert_eq!(layout.cards.len(), 1);
    let rows = &layout.cards[0].rows;
    assert_eq!(rows.len(), 10);
    assert_eq!(
        rows.iter().map(|r| r.index).collect::<Vec<_>>(),
        (10..20).collect::<Vec<_>>()
    );
    assert_eq!(rows[0].rect.y, 0.0);
    assert_eq!(rows[9].rect.bottom(), 240.0);
    eprintln!(
        "1000 seat rows: only {} row rectangles allocated for a 240px viewport",
        rows.len()
    );
}

#[test]
fn sidebar_skips_closed_cards_outside_the_scroll_window() {
    let cards: Vec<_> = (1..=1000)
        .map(|id| SidebarCard {
            swarm: SwarmId::from_u128(id),
            open: false,
            rows: Vec::new(),
        })
        .collect();
    let layout = solve_sidebar(
        &cards,
        Rect::new(0.0, 0.0, 260.0, 240.0),
        3500.0,
        Density::Cozy,
        chrome(),
    );
    assert_eq!(layout.cards.len(), 7);
    assert_eq!(layout.cards[0].swarm, SwarmId::from_u128(101));
    assert!(layout.cards.iter().all(|c| c.rows.is_empty()));
}

#[test]
fn compact_and_touch_use_their_token_heights_and_scroll_stays_local() {
    let card = SidebarCard {
        swarm: SwarmId::from_u128(1),
        open: true,
        rows: vec![RowRun {
            kind: RowKind::Seat,
            count: 1000,
        }],
    };
    for density in [
        Density::Touch,
        Density::Comfortable,
        Density::Cozy,
        Density::Compact,
    ] {
        let layout = solve_sidebar(
            std::slice::from_ref(&card),
            Rect::new(0.0, 0.0, 260.0, 240.0),
            0.0,
            density,
            chrome(),
        );
        assert!(
            layout.cards[0]
                .rows
                .iter()
                .all(|r| r.rect.h == density.tokens().row_h as f32)
        );
        assert!(layout.cards[0].rows.len() <= 12);
    }
    assert_eq!(
        scroll_into_view(0.0, 240.0, 23976.0, 24.0, 24000.0),
        23760.0
    );
    assert_eq!(scroll_into_view(100.0, 240.0, 120.0, 24.0, 24000.0), 100.0);
}

#[test]
fn phone_sheet_geometry_snap_and_touch_density_match_the_lab_frame() {
    let window = Rect::new(0.0, 0.0, 320.0, 600.0);
    let rules = PhoneRules::default();
    let closed = SheetState::Closed.rect(window, rules);
    let open = SheetState::Open.rect(window, rules);
    near(closed.y, 556.0);
    near(open.y, 228.0);
    near(open.h, 372.0);
    assert_eq!(closed.w, window.w);
    assert_eq!(open.w, window.w);
    assert_eq!(
        SheetState::Dragging(100.0).snap(window, rules),
        SheetState::Open
    );
    assert_eq!(
        SheetState::Dragging(200.0).snap(window, rules),
        SheetState::Closed
    );
    assert_eq!(
        SheetState::density(599.0, Density::Cozy, rules),
        Density::Touch
    );
    assert_eq!(
        SheetState::density(600.0, Density::Cozy, rules),
        Density::Cozy
    );
    assert_eq!(rules.transition_ms, 320);
}

#[test]
fn scene_fifteen_gutters_are_the_fixed_two_pixel_divider() {
    // The gutter is a fixed 2 px divider, not the glow (rim-glow §2): the rim and its
    // bloom are drawn inside each pane, so no glow setting reaches the solver.
    assert_eq!(TOKENS.layout.gutter, 2.);
    let gutter = TOKENS.layout.gutter as f32;
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/scene15.json")).unwrap();
    let tree = fixture_tree(&fixture["tree"]);
    let solved = solve_split(&tree, Rect::new(0.0, 0.0, 1600.0, 900.0), gutter);
    assert_eq!(solved.panes.len(), 15);
    for divider in &solved.dividers {
        match divider.axis {
            Axis::Row => near(divider.rect.w, gutter),
            Axis::Column => near(divider.rect.h, gutter),
        }
    }
    for (i, (_, a)) in solved.panes.iter().enumerate() {
        for (_, b) in &solved.panes[i + 1..] {
            assert!(!a.intersects(*b), "panes overlap");
        }
    }
}
