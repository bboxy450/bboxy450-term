// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
#![allow(
    clippy::disallowed_methods,
    reason = "test/fixture IO, not the UI thread"
)]
use bbt_ui::{
    self as ui, components as c,
    modules::seat_row::{SeatRow, Subagent, named},
};
use bbt_ui_model::{
    Activity, Grade, WaitKind,
    draw::Prim,
    layout::{Axis, Rect, TextMeasure, TextStyle},
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
fn context() -> ui::Context {
    ui::Context::new(ui::Theme::Dark, ui::Density::Cozy)
}

fn dim_observations(ops: &[c::Paint]) -> (Vec<usize>, Vec<(usize, ui::Rim)>) {
    fn visit(
        ops: &[c::Paint],
        depth: usize,
        text: &mut Vec<usize>,
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
                    visit(ops, depth + usize::from(dim), text, rims);
                }
                c::Paint::Draw(list) | c::Paint::Underlay(list) => {
                    for primitive in &list.primitives {
                        if matches!(primitive, Prim::Text { .. }) {
                            text.push(depth);
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
    let (mut text, mut rims) = (vec![], vec![]);
    visit(ops, 0, &mut text, &mut rims);
    (text, rims)
}

// keeps: spec ux-prototype §3-4, identity rims bypass inactive chrome dimming
#[test]
fn inactive_seat_content_dims_once_without_moving_or_dimming_its_rim() {
    for theme in [ui::Theme::Dark, ui::Theme::Light] {
        let ctx = ui::Context::new(theme, ui::Density::Cozy);
        let mut row = SeatRow::new("inactive", "Vivid identity", "claude");
        row.color = Some(bbt_ui_model::draw::Color::hex("#40A0FF").0);
        row.activity = Activity::Active;
        row.subtitle = Some("executor · local".into());
        row.swell = 1.;
        let active = row
            .element(ctx)
            .layout(Rect::new(0., 0., 360., 0.), ctx, &Metrics)
            .unwrap();
        let (active_text, active_rims) = dim_observations(&c::paint(&active, ctx));
        assert!(!active_text.is_empty() && active_text.iter().all(|depth| *depth == 0));
        assert!(!active_rims.is_empty());
        row.unfocused = true;
        let inactive = row
            .element(ctx)
            .layout(Rect::new(0., 0., 360., 0.), ctx, &Metrics)
            .unwrap();
        for key in ["row", "name", "tile", "right/glyph", "sub"] {
            assert_eq!(
                find(&active, key).unwrap().rect,
                find(&inactive, key).unwrap().rect,
                "{key}"
            );
        }
        let (inactive_text, inactive_rims) = dim_observations(&c::paint(&inactive, ctx));
        assert_eq!(inactive_text.len(), active_text.len());
        assert!(
            inactive_text.iter().all(|depth| *depth == 1),
            "{inactive_text:?}"
        );
        assert_eq!(
            inactive_rims, active_rims,
            "identity rims must not inherit the dim transform"
        );
    }
}

// keeps: spec gui-components — explicit filters, masks and opacity retain their semantics
#[test]
fn content_dim_preserves_unrelated_mask_opacity_and_explicit_filters() {
    let ctx = context();
    let mut row = SeatRow::new("mask", "Masked identity", "codex");
    row.unfocused = true;
    let mut element = row.element(ctx);
    let c::Element::Named { content, .. } = &mut element else {
        panic!("row name")
    };
    let c::Element::Box(frame) = content.as_mut() else {
        panic!("row frame")
    };
    frame.style.overrides.opacity = Some(0.6);
    let mask = c::LinearAlphaMask {
        start: [72., 0.],
        end: [100., 0.],
        alpha0: 1.,
        alpha1: 0.,
    };
    frame.mask = Some(mask);
    let layout = element
        .layout(Rect::new(13., 27., 320., 0.), ctx, &Metrics)
        .unwrap();
    let painted = c::paint(&layout, ctx);
    let [
        c::Paint::Group {
            ops,
            opacity,
            saturation,
            mask: Some(painted_mask),
        },
    ] = painted.as_slice()
    else {
        panic!("explicit opacity and mask must still enclose content and rims")
    };
    assert_eq!((*opacity, *saturation), (0.6, 1.));
    assert_eq!(*painted_mask, mask.translated([13., 27.]));
    let (text, rims) = dim_observations(ops);
    assert!(!text.is_empty() && text.iter().all(|depth| *depth == 1));
    assert!(!rims.is_empty() && rims.iter().all(|(depth, _)| *depth == 0));

    row.unfocused = false;
    let ordinary = c::Element::Filter {
        content: Box::new(row.element(ctx)),
        saturation: 0.4,
        opacity: 0.7,
    }
    .layout(Rect::new(0., 0., 320., 0.), ctx, &Metrics)
    .unwrap();
    let painted = c::paint(&ordinary, ctx);
    let [
        c::Paint::Group {
            ops,
            opacity,
            saturation,
            mask: None,
        },
    ] = painted.as_slice()
    else {
        panic!("an explicit filter still encloses the whole row")
    };
    assert_eq!((*opacity, *saturation), (0.7, 0.4));
    assert!(ops.iter().any(|op| matches!(
        op,
        c::Paint::Material {
            kind: ui::Material::Rim,
            ..
        }
    )));
}

// keeps: spec gui-components §1.3
#[test]
fn shell_role_has_no_seat_or_strip_chip_but_other_roles_keep_theirs() {
    let ctx = context();
    let mut row = SeatRow::new("shell", "Shell", "zsh");
    for (role, has_chip) in [("shell", false), ("executor", true), ("oracle", true)] {
        row.role = Some(role.into());
        let layout = row
            .element(ctx)
            .layout(Rect::new(0., 0., 320., 0.), ctx, &Metrics)
            .unwrap();
        assert_eq!(find(&layout, "rmark").is_some(), has_chip);
        let strip = bbt_ui::modules::strip::Strip::new(row.clone());
        let layout = strip
            .element(ctx)
            .layout(Rect::new(0., 0., 320., 40.), ctx, &Metrics)
            .unwrap();
        assert_eq!(find(&layout, "pane/strip/rmark").is_some(), has_chip);
    }
}

// keeps: spec gui-components §1.3
#[test]
fn waiting_halo_does_not_move_the_row_or_its_glyph_at_any_density() {
    for theme in [ui::Theme::Dark, ui::Theme::Light] {
        for density in [
            ui::Density::Touch,
            ui::Density::Comfortable,
            ui::Density::Cozy,
            ui::Density::Compact,
        ] {
            let ctx = ui::Context::new(theme, density);
            let mut row = SeatRow::new("row0", "Vesper Greywind", "claude");
            row.shortcut = Some("⌘1".into());
            let idle = row
                .element(ctx)
                .layout(Rect::new(0., 0., 300., 0.), ctx, &Metrics)
                .unwrap();
            row.activity = Activity::Waiting(WaitKind::Question);
            let waiting = row
                .element(ctx)
                .layout(Rect::new(0., 0., 300., 0.), ctx, &Metrics)
                .unwrap();
            assert_eq!(idle.rect, waiting.rect);
            assert_eq!(
                find(&idle, "right/glyph").unwrap().rect,
                find(&waiting, "right/glyph").unwrap().rect
            );
            let c::LayoutKind::Atom(list) =
                &find(&waiting, "right/glyph").unwrap().children[0].kind
            else {
                panic!("glyph atom")
            };
            assert_eq!(
                list.primitives
                    .iter()
                    .filter(|p| matches!(p, Prim::TextShadow { .. } | Prim::PathShadow { .. }))
                    .count(),
                2
            );
            let c::LayoutKind::Atom(list) = &find(&idle, "right/glyph").unwrap().children[0].kind
            else {
                panic!("glyph atom")
            };
            assert!(
                !list
                    .primitives
                    .iter()
                    .any(|p| matches!(p, Prim::TextShadow { .. } | Prim::PathShadow { .. }))
            );
        }
    }
}
fn find<'a>(l: &'a c::Layout, key: &str) -> Option<&'a c::Layout> {
    if matches!(&l.kind,c::LayoutKind::Named(n) if n==key) {
        return Some(l);
    }
    l.children.iter().find_map(|l| find(l, key))
}
fn labels(l: &c::Layout) -> Vec<String> {
    let mut out = vec![];
    if let c::LayoutKind::Text { runs, .. } = &l.kind {
        out.extend(runs.iter().map(|r| r.text.clone()));
    }
    for c in &l.children {
        out.extend(labels(c));
    }
    out
}
// keeps: spec gui-components §1.3
#[test]
fn module_sources_and_legacy_adapter_cannot_hand_draw_rows() {
    fn inspect(dir: &std::path::Path) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                inspect(&path);
                continue;
            }
            if path.extension().is_none_or(|x| x != "rs") {
                continue;
            }
            let module = std::fs::read_to_string(&path).unwrap();
            for forbidden in [
                "Prim::",
                "draw::Prim",
                "DrawList",
                "DrawCtx",
                ".push(Prim",
                ".rect(",
                ".line(",
            ] {
                assert!(
                    !module.contains(forbidden),
                    "module {} bypasses components: {forbidden}",
                    path.display()
                );
            }
        }
    }
    inspect(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/modules"));
    let old = include_str!("../../bboxybook/src/modules.rs");
    let body = old
        .split("pub fn seat_row(")
        .nth(1)
        .unwrap()
        .split("\nfn split(")
        .next()
        .unwrap();
    assert!(body.contains("crate::seat_rows::paint_model"));
    for forbidden in ["out.rect", "out.text", "Prim::", "IconTile", "StateGlyph"] {
        assert!(!body.contains(forbidden), "legacy hand painter survived");
    }
}
// keeps: spec gui-components §1.3
#[test]
fn seat_identity_authority_and_activity_stay_independent_when_name_shrinks() {
    let ctx = context();
    let mut s = SeatRow::new(
        "row1",
        "A very long seat name that cannot fit in the row",
        "codex",
    );
    s.activity = Activity::Active;
    s.authority = Some("revoked".into());
    s.role = Some("executor".into());
    s.shortcut = Some("⌘2".into());
    s.listeners = 2;
    let layout = s
        .element(ctx)
        .layout(Rect::new(0., 0., 220., 0.), ctx, &Metrics)
        .unwrap();
    let glyph = find(&layout, "right/glyph").unwrap();
    assert!(glyph.rect.right() <= 210.001);
    assert!(
        find(&layout, "name").unwrap().rect.w
            < Metrics.width(
                &s.name,
                &TextStyle {
                    size: 12.5,
                    weight: 600,
                    mono: false,
                    italic: false,
                    tracking_em: 0.
                }
            )
    );
    assert!(find(&layout, "authmark").is_some());
    let painted = c::paint(&glyph.children[0], ctx);
    assert!(painted.iter().any(|op|matches!(op,c::Paint::Draw(d) if d.primitives.iter().any(|p|matches!(p,Prim::Path{fill:Some(_),..})))));
    assert!(!painted.iter().any(|op|matches!(op,c::Paint::Draw(d) if d.primitives.iter().any(|p|matches!(p,Prim::Path{fill:None,..})))));
}
// keeps: spec gui-components §1.3
#[test]
fn subagents_filter_finished_work_and_overflow_after_seven() {
    let ctx = context();
    let mut seat = SeatRow::new("row1", "Nyx", "codex");
    seat.activity = Activity::Active;
    let agents = (0..13)
        .map(|i| Subagent {
            id: i.to_string(),
            description: format!("worker {i}"),
            detail: None,
            activity: if i == 12 {
                Activity::Idle
            } else if i == 1 {
                Activity::Waiting(WaitKind::Permission)
            } else {
                Activity::Active
            },
        })
        .collect::<Vec<_>>();
    let rows = seat.subagent_rows(&agents, ctx);
    assert_eq!(rows.len(), 8);
    let overflow = rows[7]
        .layout(Rect::new(0., 0., 300., 0.), ctx, &Metrics)
        .unwrap();
    assert!(labels(&overflow).contains(&"+5 running".into()));
    assert!(find(&overflow, "glyph").is_none());
    let first = rows[0]
        .layout(Rect::new(0., 0., 300., 0.), ctx, &Metrics)
        .unwrap();
    let g = find(&first, "glyph").unwrap();
    assert_eq!(g.rect.w, bbt_ui_model::STYLE.r#type.micro as f32);
    assert!(
        matches!(&g.children[0].kind,c::LayoutKind::Atom(d) if d.primitives.iter().any(|p|matches!(p,Prim::Rect{stroke:Some(_),..})))
    );
    seat.grade = Grade::Inferred; // Parent grade never replaces the hooks' shim grade.
    assert_eq!(seat.subagent_rows(&agents, ctx).len(), 8);
}
// keeps: spec gui-components §1.3
#[test]
fn flex_max_violation_freezes_before_redistributing_remaining_space() {
    let ctx = context();
    let text = |key: &str| {
        named(
            key,
            c::Element::Text(c::Text::new(
                "12345678901234567890",
                ui::Text {
                    size: Some(10.),
                    ..Default::default()
                },
            )),
        )
    };
    let content = |key: &str, max| c::Item {
        element: text(key),
        extent: c::Extent::Content {
            grow: 1.,
            shrink: 1.,
            min: 0.,
            max,
            max_fraction: None,
        },
    };
    let tree = c::Element::Stack(c::Stack {
        direction: Axis::Row,
        gap: ui::Length::Px(0.),
        align: c::Alignment::Start,
        children: vec![content("title", None), content("chip", Some(50.))],
    });
    let layout = tree
        .layout(Rect::new(0., 0., 150., 0.), ctx, &Metrics)
        .unwrap();
    assert!((layout.children[0].rect.w - 100.).abs() < 0.001);
    assert!((layout.children[1].rect.w - 50.).abs() < 0.001);
}
// keeps: spec gui-components §1.3
#[test]
fn naturally_sized_text_is_not_truncated_by_float_subtraction() {
    let ctx = context();
    let mut t = c::Text::new(
        "Natural text",
        ui::Text {
            size: Some(12.5),
            ..Default::default()
        },
    );
    t.wrap = false;
    t.ellipsis = true;
    let l = c::Element::Text(t)
        .layout(Rect::new(151.80307, 0., 75., 0.), ctx, &Metrics)
        .unwrap();
    for op in c::paint(&l, ctx) {
        if let c::Paint::Draw(d) = op {
            for p in d.primitives {
                if let Prim::Text { max_width, .. } = p {
                    assert_eq!(max_width, None);
                }
            }
        }
    }
}

// keeps: spec gui-components §1.3
#[test]
fn compact_artifact_row_preserves_natural_chip_height_outside_its_flow_slot() {
    use bbt_ui::modules::seat_row::ArtifactRow;
    let ctx = ui::Context::new(ui::Theme::Dark, ui::Density::Compact);
    let row = ArtifactRow {
        kind: "image".into(),
        reference: "target/render/smoke.png".into(),
        digest: Some("a723f1c9".into()),
        responsible: Some("Nyx Copperwing".into()),
        responsible_color: None,
    };
    let layout = row
        .element(ctx)
        .layout(Rect::new(0., 0., 500., 0.), ctx, &Metrics)
        .unwrap();
    assert_eq!(layout.rect.h, 14.);
    let right = find(&layout, "right").unwrap();
    assert_eq!(right.rect.h, 16.);
    assert_eq!(right.rect.y, -1.);
    let chip = find(&layout, "right/achip").unwrap();
    assert_eq!(chip.rect.h, 16.);
}
