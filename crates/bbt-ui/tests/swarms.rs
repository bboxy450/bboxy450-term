// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use bbt_ui::{
    self as ui, components as c,
    modules::{seat_row::SeatRow, sidebar::Sidebar, swarm_card::SwarmCard},
};
use bbt_ui_model::{
    draw::Color,
    layout::{Rect, TextMeasure, TextStyle},
};
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
fn ctx() -> ui::Context {
    ui::Context::new(ui::Theme::Dark, ui::Density::Cozy)
}
fn card(n: usize) -> SwarmCard {
    SwarmCard {
        key: "term".into(),
        name: "term".into(),
        color: Color::hex("#40A0FF").0,
        seats: (0..n)
            .map(|i| SeatRow::new(i.to_string(), format!("Seat {i}"), "codex"))
            .collect(),
        expanded: false,
        closed: true,
        unfocused: false,
        tabs: vec![],
        body: false,
        new_seat: false,
        extra: vec![],
        shadows: vec![],
    }
}
fn find<'a>(n: &'a c::Layout, key: &str) -> Option<&'a c::Layout> {
    if matches!(&n.kind,c::LayoutKind::Named(s) if s==key) {
        Some(n)
    } else {
        n.children.iter().find_map(|n| find(n, key))
    }
}
fn count(n: &c::Layout) -> usize {
    1 + n.children.iter().map(count).sum::<usize>()
}
// keeps: spec gui-components §1.3
#[test]
fn collapsed_thousand_seat_card_keeps_a_bounded_composition_and_clips_its_material() {
    let card = card(1000);
    let layout = card
        .element(ctx())
        .layout(Rect::new(0., 0., 260., 0.), ctx(), &Metrics)
        .unwrap();
    assert!(count(&layout) < 100, "collapsed card built all seats");
    assert!(find(&layout, "body").is_none());
    assert!(find(&layout, "head/roll/ellip").is_some());
    assert!(c::paint(&layout, ctx()).iter().any(
        |p| matches!(p,c::Paint::Material{clips,..} if clips.iter().any(|c|c.radii==[bbt_ui_model::STYLE.radius.card as f32;4]))
    ));
}
// keeps: spec gui-components §1.3
#[test]
fn unfocused_card_dims_only_its_contents_and_keeps_the_identity_bumper() {
    use bbt_ui_model::draw::Prim;
    let mut card = card(2);
    let active = card
        .element(ctx())
        .layout(Rect::new(0., 0., 280., 0.), ctx(), &Metrics)
        .unwrap();
    card.unfocused = true;
    let layout = card
        .element(ctx())
        .layout(Rect::new(0., 0., 280., 0.), ctx(), &Metrics)
        .unwrap();
    let head = find(&layout, "head").unwrap();
    assert!(matches!(&head.children[0].kind,c::LayoutKind::Box{style,..} if style.dim.is_some()));
    assert_eq!(layout.rect, active.rect);
    assert_eq!(head.rect, find(&active, "head").unwrap().rect);
    let paints = c::paint(&layout, ctx());
    assert!(paints.iter().any(|p| {
        match p {
            c::Paint::Draw(d) => d
                .primitives
                .iter()
                .any(|p| matches!(p,Prim::Rect{rect,fill,..} if rect.h==3. && fill.0==card.color)),
            _ => false,
        }
    }));
    assert_eq!(
        card.seats
            .iter()
            .map(|s| s.key.as_str())
            .collect::<Vec<_>>(),
        ["0", "1"]
    );
}

// keeps: spec ux-prototype §3.1 — the inactive swarm dims contents, not identity rims
#[test]
fn inactive_open_swarm_dims_its_seats_once_and_keeps_rims_outside_dim_groups() {
    use bbt_ui::modules::{
        mini_grid::Node,
        tab_row::{TabRow, TabSeat},
    };
    use bbt_ui_model::{Activity, draw::Prim, layout::Axis};
    fn visit(
        ops: &[c::Paint],
        depth: usize,
        texts: &mut Vec<usize>,
        rims: &mut Vec<(usize, ui::Rim)>,
    ) {
        for op in ops {
            match op {
                c::Paint::Group {
                    ops,
                    opacity,
                    saturation,
                    ..
                } => {
                    let dim = *opacity == bbt_ui_model::TOKENS.dim.opacity as f32
                        && *saturation == bbt_ui_model::TOKENS.dim.saturate as f32;
                    visit(ops, depth + usize::from(dim), texts, rims);
                }
                c::Paint::Draw(list) | c::Paint::Underlay(list) => {
                    for primitive in &list.primitives {
                        if matches!(primitive, Prim::Text { .. }) {
                            texts.push(depth);
                        }
                    }
                }
                c::Paint::Material {
                    kind: ui::Material::Rim,
                    rim: Some(rim),
                    ..
                } => rims.push((depth, *rim)),
                _ => {}
            }
        }
    }
    for theme in [ui::Theme::Dark, ui::Theme::Light] {
        let ctx = ui::Context::new(theme, ui::Density::Cozy);
        let mut card = card(2);
        card.closed = false;
        card.expanded = true;
        card.body = true;
        card.new_seat = true;
        for row in &mut card.seats {
            row.activity = Activity::Active;
            row.swell = 1.;
        }
        let cells: Vec<_> = card
            .seats
            .iter()
            .map(|row| Node::Cell {
                key: row.key.clone(),
                name: row.name.clone(),
                activity: row.activity,
            })
            .collect();
        card.tabs = vec![TabRow {
            name: "two seats".into(),
            shortcut: None,
            tree: Node::Split {
                axis: Axis::Row,
                ratio: 0.5,
                a: Box::new(cells[0].clone()),
                b: Box::new(cells[1].clone()),
            },
            seats: card
                .seats
                .iter()
                .cloned()
                .map(|row| TabSeat {
                    row,
                    children: vec![],
                })
                .collect(),
            expanded: true,
            focused_key: None,
        }];
        let active = card
            .element(ctx)
            .layout(Rect::new(0., 0., 320., 0.), ctx, &Metrics)
            .unwrap();
        let (mut active_text, mut active_rims) = (vec![], vec![]);
        visit(
            &c::paint(&active, ctx),
            0,
            &mut active_text,
            &mut active_rims,
        );
        assert!(!active_text.is_empty() && active_text.iter().all(|depth| *depth == 0));
        assert!(active_rims.len() >= 3, "tab header and both seat rims");
        card.unfocused = true;
        let inactive = card
            .element(ctx)
            .layout(Rect::new(0., 0., 320., 0.), ctx, &Metrics)
            .unwrap();
        for key in [
            "head",
            "body",
            "body/tab0/seat0/row",
            "body/tab0/seat1/row",
            "body/newseat",
        ] {
            assert_eq!(
                find(&active, key).unwrap().rect,
                find(&inactive, key).unwrap().rect,
                "{key}"
            );
        }
        let (mut inactive_text, mut inactive_rims) = (vec![], vec![]);
        visit(
            &c::paint(&inactive, ctx),
            0,
            &mut inactive_text,
            &mut inactive_rims,
        );
        assert_eq!(inactive_text.len(), active_text.len());
        assert!(
            inactive_text.iter().all(|depth| *depth == 1),
            "{inactive_text:?}"
        );
        assert_eq!(
            inactive_rims, active_rims,
            "identity rims keep their color and effective opacity"
        );
        let bumper_height = ui::Style::new(ui::Preset::SwarmCard)
            .resolve(ctx)
            .unwrap()
            .bumper[0];
        assert!(c::paint(&inactive, ctx).iter().any(|op| matches!(op, c::Paint::Draw(list) if list.primitives.iter().any(|p| matches!(p, Prim::Rect { rect, fill, .. } if rect.h == bumper_height && fill.0 == card.color)))));
    }
}
// keeps: spec gui-components §1.3
#[test]
fn sidebar_scroll_and_phone_handle_use_the_supplied_viewport_without_reordering_cards() {
    let mut side = Sidebar {
        cards: (0..10)
            .map(|i| {
                let mut c = card(2);
                c.key = i.to_string();
                c
            })
            .collect(),
        size: [280., 200.],
        scroll: 0.,
        phone: true,
        new_swarm: false,
    };
    let a = side
        .element(ctx())
        .layout(Rect::new(0., 0., 280., 0.), ctx(), &Metrics)
        .unwrap();
    assert_eq!(a.rect.h, 200.);
    assert_eq!(find(&a, "handle").unwrap().rect.h, 44.);
    assert_eq!(find(&a, "scroll").unwrap().rect.h, 133.);
    side.scroll = 50.;
    let b = side
        .element(ctx())
        .layout(Rect::new(0., 0., 280., 0.), ctx(), &Metrics)
        .unwrap();
    assert_eq!(
        find(&b, "swarm0/card").unwrap().rect.y,
        find(&a, "swarm0/card").unwrap().rect.y - 50.
    );
    assert_eq!(side.cards[0].key, "0");
    assert_eq!(side.cards[9].key, "9");
}
// keeps: spec gui-components §1.3
#[test]
fn a_literal_hyphen_is_a_wrap_opportunity_and_min_content_survives_flex_shrink() {
    use bbt_ui_model::layout::Axis;
    let mut text = c::Text::new(
        "cloud-eu",
        ui::Text {
            size: Some(12.),
            ..Default::default()
        },
    );
    text.wrap = true;
    let row = c::Element::Stack(c::Stack {
        direction: Axis::Row,
        gap: ui::Length::Px(0.),
        align: c::Alignment::Start,
        children: vec![
            c::Item {
                element: c::Element::Text(text),
                extent: c::Extent::MinContent,
            },
            c::Item {
                element: c::Element::Spacer,
                extent: c::Extent::Fixed(26.),
            },
        ],
    });
    let layout = row
        .layout(Rect::new(0., 0., 56., 0.), ctx(), &Metrics)
        .unwrap();
    assert_eq!(layout.children[0].rect.w, 36.);
    assert!(layout.children[0].rect.h > 30.);
    assert!(
        matches!(&layout.children[0].kind,c::LayoutKind::Text{runs,..} if runs.len()==2 && runs[0].text=="cloud-" && runs[1].text=="eu")
    );
}

// keeps: spec gui-components §1.3
#[test]
fn css_filter_uses_luminance_saturation_and_preserves_layout() {
    use bbt_ui_model::draw::Prim;
    let mut style = ui::Style::new(ui::Preset::Card);
    style.overrides.fill = Some(ui::Paint::solid(ui::Color::Rgba([1., 0., 0., 1.])));
    style.overrides.height = Some(ui::Length::Px(20.));
    style.overrides.border = Some(ui::Border {
        width: ui::Length::Px(0.),
        paint: ui::Paint::solid(ui::Color::Transparent),
        outside: false,
    });
    let content = c::Element::Box(c::Box::new(style, vec![]));
    let plain = content
        .layout(Rect::new(0., 0., 100., 0.), ctx(), &Metrics)
        .unwrap();
    let filtered = c::Element::Filter {
        content: Box::new(content.clone()),
        saturation: 0.1,
        opacity: 0.45,
    }
    .layout(Rect::new(0., 0., 100., 0.), ctx(), &Metrics)
    .unwrap();
    assert_eq!(plain.rect, filtered.rect);
    let ops = c::paint(&filtered, ctx());
    let [
        c::Paint::Group {
            ops,
            opacity,
            saturation,
            ..
        },
    ] = ops.as_slice()
    else {
        panic!("the completed content must have one filter boundary");
    };
    assert_eq!((*opacity, *saturation), (0.45, 0.1));
    let fills = ops
        .iter()
        .flat_map(|p| match p {
            c::Paint::Draw(d) => d.primitives.iter().collect::<Vec<_>>(),
            _ => vec![],
        })
        .filter_map(|p| match p {
            Prim::Rect { fill, .. } => Some(fill.0),
            _ => None,
        })
        .collect::<Vec<_>>();
    // Child colours stay full-strength; the L1 compositor filters their overlap.
    assert!(fills.contains(&[1., 0., 0., 1.]));
    assert!(
        c::Element::Filter {
            content: Box::new(content),
            saturation: 0.1,
            opacity: 1.1
        }
        .layout(Rect::new(0., 0., 100., 0.), ctx(), &Metrics)
        .is_err()
    );
}

// keeps: spec gui-components §1.3
#[test]
fn caption_line_limit_preserves_two_lines_and_marks_truncated_content() {
    let mut text = c::Text::new(
        "one two three four five six seven eight",
        ui::Text {
            size: Some(12.),
            line_height: 1.25,
            ..Default::default()
        },
    );
    let whole = c::Element::Text(text.clone())
        .layout(Rect::new(0., 0., 60., 0.), ctx(), &Metrics)
        .unwrap();
    text.max_lines = Some(2);
    let limited = c::Element::Text(text.clone())
        .layout(Rect::new(0., 0., 60., 0.), ctx(), &Metrics)
        .unwrap();
    assert_eq!(limited.rect.h, 30.);
    assert!(whole.rect.h > limited.rect.h);
    let c::LayoutKind::Text { runs, .. } = &limited.kind else {
        panic!("expected text")
    };
    assert!(runs.last().unwrap().text.ends_with('…'));
    assert!(runs.iter().all(|r| r.rect.right() <= 60.));
    assert!(!runs.iter().any(|r| r.text.contains("eight")));
    text.max_lines = Some(0);
    assert!(
        c::Element::Text(text)
            .layout(Rect::new(0., 0., 60., 0.), ctx(), &Metrics)
            .is_err()
    );
}
