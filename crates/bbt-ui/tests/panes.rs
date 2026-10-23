use bbt_ui::{
    self as ui, components as c,
    modules::{
        pane::{Pane, Resumed, Span},
        seat_row::SeatRow,
        strip::Strip,
    },
};
use bbt_ui_model::{
    Activity, Grade, TOKENS,
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
fn pane() -> Pane {
    let mut row = SeatRow::new("nyx", "Nyx Copperwing", "codex");
    row.focused = true;
    row.activity = Activity::Active;
    Pane {
        strip: Strip::new(row),
        lines: vec![
            vec![Span::Text {
                key: "pane/term/run0".into(),
                value: "   output".into(),
                foreground: false,
                hover: false,
            }],
            vec![Span::Cursor {
                key: "pane/term/cur".into(),
            }],
        ],
        gap_from: None,
        resumed: None,
        min_height: 140.,
        unfocused: false,
        reconnecting: false,
        reconnected: false,
        glow_seed: Some(7),
        opacity: 1.,
        depth: bbt_ui_model::TOKENS.glow.depth as f32,
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
fn labels(l: &c::Layout) -> String {
    let mut s = String::new();
    if let c::LayoutKind::Text { runs, .. } = &l.kind {
        for r in runs {
            s.push_str(&r.text);
        }
    }
    for ch in &l.children {
        s.push_str(&labels(ch));
    }
    s
}
#[test]
fn pane_preserves_history_content_and_capability_actions_while_disconnected() {
    let mut p = pane();
    p.reconnecting = true;
    p.unfocused = true;
    p.strip.seat.grade = Grade::Inferred;
    p.gap_from = Some(Some("09:14:07".into()));
    p.resumed = Some(Resumed {
        generation: 5,
        command: Some("codex resume session-id".into()),
    });
    let layout = p
        .element(context())
        .layout(Rect::new(10., 20., 400., 0.), context(), &Metrics)
        .unwrap();
    assert_eq!(layout.rect.h, 140.);
    assert_eq!(
        find(&layout, "pane/strip").unwrap().rect.w,
        // A constant 1 px border on every activity (G1): 400 - 2 x 1.
        398.,
        "frame must stretch over the caller width, independently of natural terminal content"
    );
    assert!(find(&layout, "pane/strip/stop").is_none());
    assert!(find(&layout, "pane/strip/mute").is_none());
    let s = labels(&layout);
    assert!(s.contains("retained from 09:14:07"));
    assert!(s.contains("process generation 5"));
    assert!(s.contains("codex resume session-id"));
    assert!(s.contains("\u{a0}\u{a0}\u{a0}output"));
    assert!(s.contains("reconnecting…"));
    let ops = c::paint(&layout, context());
    let state = ops
        .iter()
        .find_map(|p| {
            if let c::Paint::Material { glow: Some(g), .. } = p {
                Some(g)
            } else {
                None
            }
        })
        .unwrap();
    assert_eq!(state.activity, ui::GlowActivity::Active);
    assert_eq!(state.inferred_multiplier, 0.5);
    assert_eq!(
        state.particles,
        Some(0),
        "a stale fixture has no newly emitted particles"
    );
    let fx = ops
        .iter()
        .position(|p| {
            matches!(
                p,
                c::Paint::Material {
                    kind: ui::Material::Rim,
                    ..
                }
            )
        })
        .unwrap();
    // The terminal's own text, wherever it sits: a stale pane dims its strip and terminal
    // inside an isolated group.
    fn has_output(p: &c::Paint) -> bool {
        match p {
            c::Paint::Draw(d) => d.primitives.iter().any(|p| {
                matches!(p, bbt_ui_model::draw::Prim::Text { text, .. } if text.contains("output"))
            }),
            c::Paint::Group { ops, .. } => ops.iter().any(has_output),
            _ => false,
        }
    }
    let content = ops.iter().position(has_output).unwrap();
    assert!(
        fx > content,
        "the rim lies inside the pane, painted above its strip and terminal (rim-glow)"
    );
    p.strip.may_interrupt = true;
    p.strip.may_mute = true;
    let allowed = p
        .element(context())
        .layout(Rect::new(0., 0., 400., 0.), context(), &Metrics)
        .unwrap();
    assert!(find(&allowed, "pane/strip/stop").is_some());
    assert!(find(&allowed, "pane/strip/mute").is_some());
}
#[test]
fn pane_height_is_content_or_minimum_and_an_unavailable_gap_never_invents_a_range() {
    let mut p = pane();
    p.min_height = 80.;
    p.lines = vec![
        vec![Span::Cursor {
            key: "cursor".into()
        }];
        12
    ];
    p.gap_from = Some(None);
    p.strip.seat.activity = Activity::Errored;
    let l = p
        .element(context())
        .layout(Rect::new(0., 0., 200., 9999.), context(), &Metrics)
        .unwrap();
    assert_eq!(l.rect.h, p.height());
    assert!(l.rect.h > 250.);
    let text = labels(&l);
    assert!(text.contains("earlier output unavailable"));
    assert!(!text.contains("retained from"));
    let ops = c::paint(&l, context());
    assert!(ops.iter().any(|op|matches!(op,c::Paint::Draw(d) if d.primitives.iter().any(|p|matches!(p,bbt_ui_model::draw::Prim::Rect{stroke:Some((_,color)),..} if color.0==bbt_ui_model::draw::Color::hex(TOKENS.color.state.errored).0)))));
}
