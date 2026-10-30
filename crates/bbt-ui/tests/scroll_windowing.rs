// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use bbt_ui::{self as ui, components as c};
use bbt_ui_model::layout::{Axis, Rect, TextMeasure, TextStyle};
use std::{cell::Cell, collections::BTreeMap};

#[derive(Default)]
struct Metrics(Cell<usize>);
impl TextMeasure for Metrics {
    fn width(&self, t: &str, s: &TextStyle) -> f32 {
        self.0.set(self.0.get() + 1);
        t.chars().count() as f32 * s.size * 0.5
    }
}
impl c::Measure for Metrics {
    fn font_metrics(&self, s: &TextStyle) -> [f32; 2] {
        [s.size * 0.8, s.size * 0.2]
    }
}
fn ctx() -> ui::Context {
    ui::Context::new(ui::Theme::Dark, ui::Density::Cozy)
}
fn column(children: Vec<c::Item>, gap: f32) -> c::Element {
    c::Element::Stack(c::Stack {
        direction: Axis::Column,
        gap: ui::Length::Px(gap),
        align: c::Alignment::Stretch,
        children,
    })
}
fn row(key: &str, height: f32, declared: bool) -> c::Element {
    let mut style = ui::Style::new(ui::Preset::CardRow);
    style.overrides.height = Some(ui::Length::Px(height));
    let mut t = c::Text::new(key, ui::Text::default());
    t.wrap = false;
    let content = c::Element::Box(c::Box::new(style, vec![c::Element::Text(t).into()]));
    let e = c::Element::Named {
        name: key.into(),
        content: Box::new(content),
    };
    if declared {
        e.fixed_height(height, key, 7.)
    } else {
        e
    }
}
fn view(ids: &[usize], height: f32, gap: f32, offset: f32, declared: bool) -> c::ScrollView {
    let items = ids
        .iter()
        .map(|id| row(&format!("row{id}"), height, declared).into())
        .collect();
    let total = ids.iter().map(|_| height).sum::<f32>() + gap * ids.len().saturating_sub(1) as f32;
    let content = column(items, gap);
    c::ScrollView {
        viewport: [210., 100.],
        offset: [0., offset],
        content_width: None,
        content: Box::new(if declared {
            content.fixed_height(total, "", 7.)
        } else {
            content
        }),
    }
}
fn layout(view: c::ScrollView, m: &Metrics) -> c::Layout {
    c::Element::ScrollView(view)
        .layout(Rect::new(3., 5., 210., 100.), ctx(), m)
        .unwrap()
}
fn named(n: &c::Layout, out: &mut BTreeMap<String, String>) {
    if let c::LayoutKind::Named(key) = &n.kind {
        out.insert(key.clone(), format!("{n:?}"));
    }
    for child in &n.children {
        named(child, out);
    }
}
fn records(n: &c::Layout) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    named(n, &mut out);
    out
}

// keeps: spec gui-components §1.3
#[test]
fn measurement_is_bounded_by_the_window_with_exact_extent_and_geometry() {
    let ids: Vec<_> = (0..1000).collect();
    let m = Metrics::default();
    let full = layout(view(&ids, 26.375, 1.125, 12345.625, false), &m);
    let full_calls = m.0.replace(0);
    let fast = layout(view(&ids, 26.375, 1.125, 12345.625, true), &m);
    assert_eq!(format!("{:?}", full.kind), format!("{:?}", fast.kind));
    assert_eq!(full.children[0].rect, fast.children[0].rect);
    assert_eq!(
        full.children[0].baseline.to_bits(),
        fast.children[0].baseline.to_bits()
    );
    let all = records(&full);
    let visible = records(&fast);
    assert!(visible.len() < 12, "laid out {} rows", visible.len());
    assert!(m.0.get() * 30 < full_calls, "{} vs {full_calls}", m.0.get());
    for (key, node) in &visible {
        assert_eq!(Some(node), all.get(key), "{key}");
    }
    // Full-viewport rows cannot be omitted; outsets may retain extra neighbours.
    for (i, _) in ids.iter().enumerate() {
        let mut y = 5. - 12345.625;
        for _ in 0..i {
            y += 26.375 + 1.125;
        }
        if y < 105. && y + 26.375 > 5. {
            assert!(visible.contains_key(&format!("row{i}")));
        }
    }
}

// keeps: spec gui-components §1.3
#[test]
fn invalid_style_or_gap_keeps_the_fallback_layout_error_instead_of_panicking() {
    for invalid_style in [true, false] {
        let mut sidebar = sidebar(2);
        let mut style = ui::Style::new(ui::Preset::Card);
        if invalid_style {
            style.overrides.height = Some(ui::Length::Px(f32::NAN));
        }
        let mut b = c::Box::new(style, vec![]);
        b.direction = Axis::Column;
        b.align = c::Alignment::Stretch;
        if !invalid_style {
            b.gap = ui::Length::Px(f32::NAN);
        }
        sidebar.cards[0].extra.push(c::Element::Box(b));
        let e = sidebar.element(ctx());
        assert!(sidebar.anchor(ctx()).unwrap().is_none());
        assert!(
            e.layout(Rect::new(0., 0., 260., 558.), ctx(), &Metrics::default())
                .is_err()
        );
    }
}

// keeps: spec gui-components §1.3
#[test]
fn fractional_prefix_keeps_solver_addition_order_even_when_extent_sum_rounds_differently() {
    let ids: Vec<_> = (0..300).collect();
    let full = layout(view(&ids, 19.17, 0.37, 3100.3, false), &Metrics::default());
    let fast = layout(view(&ids, 19.17, 0.37, 3100.3, true), &Metrics::default());
    assert_eq!(format!("{:?}", full.kind), format!("{:?}", fast.kind));
    let all = records(&full);
    for (key, node) in records(&fast) {
        assert_eq!(Some(&node), all.get(&key), "{key}");
    }
}

// keeps: spec gui-components §1.3
#[test]
fn undeclared_content_uses_the_original_full_layout_without_measurement_shortcuts() {
    let mut mixed = view(&(0..50).collect::<Vec<_>>(), 25., 2., 200., false);
    if let c::Element::Stack(s) = mixed.content.as_mut() {
        for item in &mut s.children[..49] {
            item.element = item.element.clone().fixed_height(25., "", 0.);
        }
    }
    let m = Metrics::default();
    let mixed = layout(mixed, &m);
    let calls = m.0.replace(0);
    let full = layout(view(&(0..50).collect::<Vec<_>>(), 25., 2., 200., false), &m);
    assert_eq!(format!("{mixed:?}"), format!("{full:?}"));
    assert_eq!(calls, m.0.get());
}

// keeps: spec gui-components §1.3
#[test]
fn anchor_survives_insert_remove_and_width_density_changes_by_stable_identity() {
    let old = view(&(0..20).collect::<Vec<_>>(), 24., 0., 125., true);
    let anchor = old.anchor(ctx()).unwrap().unwrap(); // row5, 5px in
    let mut next = view(&(0..20).rev().collect::<Vec<_>>(), 38., 0., 0., true);
    next.viewport[0] = 160.;
    next.restore_anchor(
        &anchor,
        ui::Context::new(ui::Theme::Light, ui::Density::Touch),
    )
    .unwrap();
    assert_eq!(next.offset[1], 14. * 38. + 5.);
    // The current anchor is removed: same intra-row offset on the nearest
    // surviving old neighbour, choosing the following neighbour on a tie.
    let mut next = view(
        &(0..20).filter(|i| *i != 5).collect::<Vec<_>>(),
        30.,
        0.,
        0.,
        true,
    );
    assert!(next.restore_anchor(&anchor, ctx()).unwrap());
    assert_eq!(next.offset[1], 5. * 30. + 5.); // row6
    let mut empty = view(&[], 30., 0., 17., true);
    assert!(!empty.restore_anchor(&anchor, ctx()).unwrap());
}

// keeps: spec gui-components §1.3
#[test]
fn nested_ancestors_keep_full_material_group_and_clip_rectangles() {
    let mut full = view(&(0..100).collect::<Vec<_>>(), 27., 0., 600., false);
    let mut style = ui::Style::new(ui::Preset::Card);
    style.overrides.padding = Some([ui::Length::Px(3.); 4]);
    let mut b = c::Box::new(style, vec![(*full.content).into()]);
    b.direction = Axis::Column;
    b.align = c::Alignment::Stretch;
    b.gap = ui::Length::Px(0.);
    b.clip = true;
    b.group = true;
    full.content = Box::new(c::Element::Box(b));
    let mut fast = full.clone();
    if let c::Element::Box(b) = fast.content.as_mut() {
        b.children[0].element = view(&(0..100).collect::<Vec<_>>(), 27., 0., 600., true)
            .content
            .as_ref()
            .clone();
    }
    let total = full
        .content
        .natural(210., ctx(), &Metrics::default())
        .unwrap()
        .0[1];
    fast.content = Box::new(fast.content.fixed_height(total, "", 8.));
    let full = layout(full, &Metrics::default());
    let fast = layout(fast, &Metrics::default());
    for (a, b) in [(&full, &fast), (&full.children[0], &fast.children[0])] {
        assert_eq!(a.rect, b.rect);
        assert_eq!(a.baseline.to_bits(), b.baseline.to_bits());
        assert_eq!(format!("{:?}", a.kind), format!("{:?}", b.kind));
    }
    let all = records(&full);
    for (key, node) in records(&fast) {
        assert_eq!(Some(&node), all.get(&key));
    }
}

fn sidebar(n: usize) -> ui::modules::sidebar::Sidebar {
    use ui::modules::{
        mini_grid::Node,
        seat_row::SeatRow,
        swarm_card::SwarmCard,
        tab_row::{TabRow, TabSeat},
    };
    let seats: Vec<_> = (0..n)
        .map(|i| SeatRow::new(format!("stable-{i}"), format!("Seat {i}"), "codex"))
        .collect();
    let tabs = seats
        .chunks(2)
        .map(|rows| TabRow {
            name: "tab".into(),
            shortcut: None,
            tree: Node::Cell {
                key: rows[0].key.clone(),
                name: rows[0].name.clone(),
                activity: rows[0].activity,
            },
            seats: rows
                .iter()
                .map(|row| TabSeat {
                    row: row.clone(),
                    children: vec![],
                })
                .collect(),
            expanded: true,
            focused_key: None,
        })
        .collect();
    ui::modules::sidebar::Sidebar {
        cards: vec![SwarmCard {
            key: "stable-swarm".into(),
            name: "swarm".into(),
            color: [0.2, 0.4, 0.8, 1.],
            seats,
            expanded: true,
            closed: false,
            unfocused: true,
            tabs,
            body: true,
            new_seat: true,
            extra: vec![],
            shadows: vec![],
        }],
        size: [260., 558.],
        scroll: 8700.,
        phone: false,
        new_swarm: true,
    }
}
fn without_windowing(e: &mut c::Element) {
    match e {
        c::Element::ScrollView(s) => {
            if let c::Element::FixedHeight { content, .. } = s.content.as_mut() {
                *s.content = std::mem::replace(content.as_mut(), c::Element::Spacer);
            }
        }
        c::Element::Named { content, .. } => without_windowing(content),
        c::Element::Box(b) => {
            for item in &mut b.children {
                without_windowing(&mut item.element);
            }
        }
        _ => {}
    }
}
fn scrolls(n: &c::Layout, out: &mut Vec<(Rect, [f32; 2], [f32; 2])>) {
    if let c::LayoutKind::ScrollView { content, offset } = n.kind {
        out.push((n.rect, content, offset));
    }
    for child in &n.children {
        scrolls(child, out);
    }
}
// keeps: spec gui-components §1.3
#[test]
fn real_thousand_open_sidebar_matches_full_visible_geometry_and_extent_to_the_bit() {
    let mut sidebar = sidebar(1000);
    // Production cards always carry this launch-default row. An undeclared
    // height here used to disable windowing for every seat in the card.
    sidebar.cards[0]
        .extra
        .push(ui::modules::swarm_card::workdir_row(
            "stable-swarm",
            None,
            ctx(),
        ));
    let fast = sidebar.element(ctx());
    let mut full = fast.clone();
    without_windowing(&mut full);
    let m = Metrics::default();
    let bounds = Rect::new(0., 0., 260., 558.);
    let full = full.layout(bounds, ctx(), &m).unwrap();
    let full_calls = m.0.replace(0);
    let fast = fast.layout(bounds, ctx(), &m).unwrap();
    assert!(m.0.get() * 15 < full_calls, "{} vs {full_calls}", m.0.get());
    let mut full_scrolls = vec![];
    scrolls(&full, &mut full_scrolls);
    let mut fast_scrolls = vec![];
    scrolls(&fast, &mut fast_scrolls);
    assert_eq!(full_scrolls, fast_scrolls);
    let all = records(&full);
    let visible = records(&fast);
    // Ancestor Named subtrees differ intentionally; every retained node's
    // own rect, baseline and kind must still match exactly.
    fn flatten(n: &c::Layout, prefix: &str, out: &mut BTreeMap<String, (Rect, u32, String)>) {
        let key = if let c::LayoutKind::Named(name) = &n.kind {
            name.clone()
        } else {
            prefix.into()
        };
        out.insert(
            key.clone(),
            (n.rect, n.baseline.to_bits(), format!("{:?}", n.kind)),
        );
        for (i, child) in n.children.iter().enumerate() {
            // Named children have stable keys; the anonymous layout child of
            // a Named wrapper is compared under that key's private suffix.
            if matches!(n.kind, c::LayoutKind::Named(_)) {
                flatten(child, &format!("{key}/@{i}"), out);
            } else if matches!(child.kind, c::LayoutKind::Named(_)) {
                flatten(child, "", out);
            } else {
                for sub in &child.children {
                    flatten(sub, "", out);
                }
            }
        }
    }
    let mut a = BTreeMap::new();
    flatten(&full, "root", &mut a);
    let mut b = BTreeMap::new();
    flatten(&fast, "root", &mut b);
    for (key, node) in b.iter().filter(|(key, _)| !key.is_empty()) {
        assert_eq!(Some(node), a.get(key), "{key}");
    }
    assert!(visible.len() * 15 < all.len());
    let clip = full_scrolls[0].0;
    fn check_visible(n: &c::Layout, clip: Rect, fast: &BTreeMap<String, String>) {
        if let c::LayoutKind::Named(key) = &n.kind
            && n.rect.y < clip.bottom()
            && n.rect.bottom() > clip.y
            && n.rect.x < clip.right()
            && n.rect.right() > clip.x
        {
            assert!(fast.contains_key(key), "visible part {key} omitted");
        }
        for child in &n.children {
            check_visible(child, clip, fast);
        }
    }
    check_visible(&full, clip, &visible);
}

// keeps: spec gui-components §1.3
#[test]
fn real_sidebar_falls_back_for_a_variable_extra_and_anchors_survive_rebuilds() {
    let mut sidebar = sidebar(30);
    sidebar.scroll = 110.;
    let anchor = sidebar
        .anchor(ctx())
        .unwrap()
        .expect("fixed sidebar anchor");
    let mut inserted = sidebar.cards[0].tabs[0].clone();
    for (i, seat) in inserted.seats.iter_mut().enumerate() {
        seat.row.key = format!("inserted-{i}");
    }
    sidebar.cards[0].tabs.insert(0, inserted);
    let rebuilt = sidebar.element_anchored(ctx(), Some(&anchor));
    assert!(
        rebuilt
            .layout(Rect::new(0., 0., 260., 558.), ctx(), &Metrics::default())
            .is_ok()
    );
    sidebar.cards[0].extra.push(c::Element::Text(c::Text::new(
        "variable height",
        ui::Text::default(),
    )));
    assert!(sidebar.anchor(ctx()).unwrap().is_none());
    let actual = sidebar.element(ctx());
    let mut fallback = actual.clone();
    without_windowing(&mut fallback);
    let bounds = Rect::new(0., 0., 260., 558.);
    assert_eq!(
        format!(
            "{:?}",
            actual.layout(bounds, ctx(), &Metrics::default()).unwrap()
        ),
        format!(
            "{:?}",
            fallback.layout(bounds, ctx(), &Metrics::default()).unwrap()
        )
    );
}
