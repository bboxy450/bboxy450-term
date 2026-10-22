#![allow(clippy::disallowed_methods, reason = "test/fixture IO, not the UI thread")]
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
    let painted = c::paint(&layout, ctx);
    assert!(painted.iter().any(|op|matches!(op,c::Paint::Draw(d) if d.primitives.iter().any(|p|matches!(p,Prim::Text{text,..} if text.as_ref()=="●")))));
    assert!(!painted.iter().any(|op|matches!(op,c::Paint::Draw(d) if d.primitives.iter().any(|p|matches!(p,Prim::Text{text,..} if text.as_ref()=="✕")))));
}
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
    assert!((g.rect.w - 8.8).abs() < 0.001);
    assert!(
        matches!(&g.children[0].kind,c::LayoutKind::Atom(d) if d.primitives.iter().any(|p|matches!(p,Prim::Rect{stroke:Some(_),..})))
    );
    seat.grade = Grade::Inferred; // Parent grade never replaces the hooks' shim grade.
    assert_eq!(seat.subagent_rows(&agents, ctx).len(), 8);
}
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
