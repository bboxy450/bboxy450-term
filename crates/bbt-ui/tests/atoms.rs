use bbt_ui::atoms::{Atom, AtomCtx, Glyph, Hairline, Icon, Spinner};
use bbt_ui_model::{
    Activity, Density, Grade, Theme, WaitKind,
    draw::{Color, DrawCtx, DrawList, Prim},
    layout::{Rect, TextMeasure, TextStyle},
};
struct Metrics;
impl TextMeasure for Metrics {
    fn width(&self, text: &str, style: &TextStyle) -> f32 {
        text.chars().count() as f32 * style.size * 0.6
    }
}

#[test]
fn glyph_state_grade_and_wait_kind_are_independent_and_identity_is_not_recoloured() {
    for theme in [Theme::Dark, Theme::Light] {
        let ctx = AtomCtx {
            draw: DrawCtx::new(theme, Density::Cozy),
            measure: &Metrics,
            scale: 1.,
        };
        let mut variants = std::collections::BTreeSet::new();
        for activity in [
            Activity::Active,
            Activity::Idle,
            Activity::Waiting(WaitKind::Permission),
            Activity::Waiting(WaitKind::Question),
            Activity::Waiting(WaitKind::Auth),
            Activity::Errored,
        ] {
            for grade in [Grade::Reported, Grade::Shim, Grade::Inferred] {
                let mut list = DrawList::default();
                Glyph {
                    activity,
                    grade,
                    seat_color: Color::hex("#19A573"),
                }
                .paint(Rect::new(20., 20., 12., 12.), &ctx, &mut list);
                list.validate().unwrap();
                variants.insert(format!("{:?}", list.primitives));
                let (text, color) = list
                    .primitives
                    .iter()
                    .find_map(|p| {
                        if let Prim::Text { text, color, .. } = p {
                            Some((text, color))
                        } else {
                            None
                        }
                    })
                    .unwrap();
                if activity == Activity::Active {
                    assert_eq!(text.as_ref(), "●");
                    assert_eq!(&color.0[..3], &Color::hex("#19A573").0[..3]);
                }
                assert_eq!(color.0[3], if grade == Grade::Inferred { 0.72 } else { 1. });
            }
        }
        assert_eq!(
            variants.len(),
            18,
            "state, grade and waiting kind each carry a distinct signal"
        );
    }
}

#[test]
fn hairline_is_one_physical_pixel_and_spinner_phase_is_explicit() {
    for scale in [1., 2., 3.] {
        let ctx = AtomCtx {
            draw: DrawCtx::new(Theme::Dark, Density::Cozy),
            measure: &Metrics,
            scale,
        };
        let mut list = DrawList::default();
        Hairline.paint(Rect::new(5., 8.3, 30., 0.), &ctx, &mut list);
        let Prim::Hairline { from, width, .. } = list.primitives[0] else {
            panic!("not a line")
        };
        assert_eq!(width * scale, 1.);
        assert!(((from[1] * scale).fract() - 0.5).abs() < 0.0001);
        let mut a = DrawList::default();
        let mut b = DrawList::default();
        Spinner { phase: 0. }.paint(Rect::new(0., 0., 20., 20.), &ctx, &mut a);
        Spinner { phase: 0.125 }.paint(Rect::new(0., 0., 20., 20.), &ctx, &mut b);
        assert_ne!(a.primitives, b.primitives);
        let mut again = DrawList::default();
        Spinner { phase: 0. }.paint(Rect::new(0., 0., 20., 20.), &ctx, &mut again);
        assert_eq!(a.primitives, again.primitives);
    }
}

#[test]
fn icons_keep_the_requested_logical_name_bounds_and_tint() {
    let ctx = AtomCtx {
        draw: DrawCtx::new(Theme::Light, Density::Touch),
        measure: &Metrics,
        scale: 2.,
    };
    let r = Rect::new(12., 18., 32., 32.);
    let color = Color::hex("#0033AA");
    let mut list = DrawList::default();
    Icon {
        name: "custom-harness",
        color,
    }
    .paint(r, &ctx, &mut list);
    assert!(
        matches!(&list.primitives[..],[Prim::Icon {rect,icon,color:c}] if *rect==r&&icon.as_ref()=="custom-harness"&&*c==color)
    );
}

#[test]
fn a_waiting_kind_background_and_text_receive_dimming_once() {
    let ctx = AtomCtx {
        draw: DrawCtx::new(Theme::Dark, Density::Cozy).dimmed(),
        measure: &Metrics,
        scale: 1.,
    };
    let mut list = DrawList::default();
    Glyph {
        activity: Activity::Waiting(WaitKind::Question),
        grade: Grade::Inferred,
        seat_color: Color::WHITE,
    }
    .paint(Rect::new(20., 20., 12., 12.), &ctx, &mut list);
    let alpha = list
        .primitives
        .iter()
        .find_map(|p| {
            if let Prim::Text { color, .. } = p {
                Some(color.0[3])
            } else {
                None
            }
        })
        .unwrap();
    let background = list
        .primitives
        .iter()
        .rev()
        .find_map(|p| {
            if let Prim::Rect { fill, .. } = p {
                Some(fill.0[3])
            } else {
                None
            }
        })
        .unwrap();
    assert_eq!(alpha, 0.45 * 0.72);
    assert_eq!(background, alpha);
}
