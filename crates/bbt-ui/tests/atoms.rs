use bbt_ui::atoms::{Atom, AtomCtx, Glyph, Hairline, Icon, Spinner};
use bbt_ui_model::{
    Activity, Density, Grade, Theme, WaitKind,
    draw::{Color, DrawCtx, DrawList, Prim},
    layout::{Rect, TextMeasure, TextStyle},
};

#[test]
fn glyph_shadow_inherits_kind_grade_and_ancestor_opacity_once() {
    use bbt_ui::{self as ui, components as c};
    fn collect<'a>(ops: &'a [c::Paint], alpha: f32, out: &mut Vec<(&'a Prim, f32)>) {
        for op in ops {
            match op {
                c::Paint::Draw(list) => out.extend(list.primitives.iter().map(|p| (p, alpha))),
                c::Paint::Group { ops, opacity, .. } => collect(ops, alpha * opacity, out),
                _ => {}
            }
        }
    }
    let ctx = ui::Context::new(Theme::Dark, Density::Compact);
    let shadow = c::TextShadow {
        color: ui::Paint {
            color: ui::Color::Rgba([0.2, 0.4, 0.8, 1.]),
            alpha: ui::Alpha::Value(0.5),
        },
        blur_radius: 6.,
        offset: [2., -1.],
    };
    let glyph = |shadow| c::Element::Filter {
        opacity: 0.5,
        saturation: 1.,
        content: Box::new(c::Element::Glyph {
            shadow,
            kind_colors: Default::default(),
            activity: Activity::Waiting(WaitKind::Question),
            grade: Grade::Inferred,
            color: ui::Paint::solid(ui::Color::Foreground),
            size: 10.,
            scale: 1.,
        }),
    };
    let bounds = Rect::new(20., 20., 10., 10.);
    let layout = glyph(Some(shadow)).layout(bounds, ctx, &Metrics).unwrap();
    let plain = glyph(None).layout(bounds, ctx, &Metrics).unwrap();
    assert_eq!((layout.rect, layout.baseline), (plain.rect, plain.baseline));
    let ops = c::paint(&layout, ctx);
    let mut primitives = Vec::new();
    collect(&ops, 1., &mut primitives);
    let shadows: Vec<_> = primitives
        .iter()
        .enumerate()
        .filter_map(|(i, (p, alpha))| {
            let shadow = match p {
                Prim::TextShadow { shadow, .. } | Prim::PathShadow { shadow, .. } => shadow,
                _ => return None,
            };
            assert!((shadow.color.0[3] * alpha - 0.18).abs() < 0.00001);
            assert_eq!((shadow.blur_radius, shadow.offset), (6., [2., -1.]));
            Some((i, *p))
        })
        .collect();
    assert_eq!(shadows.len(), 2, "vector symbol and inherited kind text");
    for (shadow_index, shadow) in shadows {
        let (foreground_index, _) = primitives
            .iter()
            .enumerate()
            .find(|(_, (p, _))| match (shadow, *p) {
                (
                    Prim::TextShadow {
                        text,
                        origin,
                        style,
                        ..
                    },
                    Prim::Text {
                        text: t,
                        origin: o,
                        style: s,
                        ..
                    },
                ) => text == t && origin == o && style == s,
                (
                    Prim::PathShadow {
                        origin,
                        scale,
                        points,
                        stroke,
                        filled,
                        ..
                    },
                    Prim::Path {
                        origin: o,
                        scale: s,
                        points: p,
                        stroke: st,
                        fill,
                        ..
                    },
                ) => {
                    origin == o
                        && scale == s
                        && points == p
                        && stroke == st
                        && *filled == fill.is_some()
                }
                _ => false,
            })
            .unwrap();
        assert!(shadow_index < foreground_index);
    }
}

#[test]
fn text_shadow_color_is_independent_of_foreground_alpha_and_flow() {
    use bbt_ui::{self as ui, components as c};
    let ctx = ui::Context::new(Theme::Light, Density::Cozy);
    let mut text = c::Text::new(
        "alpha",
        ui::Text {
            paint: ui::Paint {
                color: ui::Color::Foreground,
                alpha: ui::Alpha::Value(0.2),
            },
            ..Default::default()
        },
    );
    let plain = c::Element::Text(text.clone())
        .layout(Rect::new(5., 8., 200., 0.), ctx, &Metrics)
        .unwrap();
    text.shadow = Some(c::TextShadow {
        color: ui::Paint {
            color: ui::Color::Accent,
            alpha: ui::Alpha::Value(0.7),
        },
        blur_radius: 6.,
        offset: [2., 1.],
    });
    let layout = c::Element::Text(text)
        .layout(Rect::new(5., 8., 200., 0.), ctx, &Metrics)
        .unwrap();
    assert_eq!((layout.rect, layout.baseline), (plain.rect, plain.baseline));
    let ops = c::paint(&layout, ctx);
    let c::Paint::Draw(list) = &ops[0] else {
        panic!("text paint")
    };
    let shadow = list
        .primitives
        .iter()
        .position(
            |p| matches!(p,Prim::TextShadow{shadow,..} if (shadow.color.0[3]-0.7).abs()<0.00001),
        )
        .unwrap();
    let foreground = list
        .primitives
        .iter()
        .position(|p| matches!(p,Prim::Text{color,..} if (color.0[3]-0.2).abs()<0.00001))
        .unwrap();
    assert!(shadow < foreground);
}
struct Metrics;
impl bbt_ui::components::Measure for Metrics {
    fn font_metrics(&self, style: &TextStyle) -> [f32; 2] {
        [style.size * 0.8, style.size * 0.2]
    }
}
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
                let color = list
                    .primitives
                    .iter()
                    .find_map(|p| match p {
                        Prim::Path { color, .. } => Some(color),
                        _ => None,
                    })
                    .unwrap();
                assert!(
                    !list
                        .primitives
                        .iter()
                        .any(|p| matches!(p, Prim::Text {text,..}
                    if ["●","○","◐","✕"].contains(&text.as_ref())))
                );
                if activity == Activity::Active {
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

#[test]
fn kind_colours_do_not_recolour_state_or_apply_grade_and_dim_twice() {
    use bbt_ui::atoms::KindColors;
    for dim in [false, true] {
        let draw = DrawCtx::new(Theme::Dark, Density::Cozy);
        let ctx = AtomCtx {
            draw: if dim { draw.dimmed() } else { draw },
            measure: &Metrics,
            scale: 1.,
        };
        let glyph = Glyph {
            activity: Activity::Waiting(WaitKind::Question),
            grade: Grade::Inferred,
            seat_color: Color::WHITE,
        };
        let rect = Rect::new(20., 20., 12., 12.);
        let mut ordinary = DrawList::default();
        let mut strip = DrawList::default();
        glyph.paint(rect, &ctx, &mut ordinary);
        glyph.paint_with_kind_colors(
            rect,
            &ctx,
            &mut strip,
            KindColors {
                foreground: Some(Color::WHITE),
                background: Some(Color::TRANSPARENT),
            },
        );
        assert_eq!(
            ordinary.primitives[0], strip.primitives[0],
            "main state signal must be unchanged"
        );
        let alpha = if dim { 0.45 * 0.72 } else { 0.72 };
        assert!(
            matches!(strip.primitives.last(),Some(Prim::Text {color,..}) if color.0==[1.,1.,1.,alpha])
        );
        assert!(
            strip
                .primitives
                .iter()
                .any(|p| matches!(p,Prim::Rect {fill,stroke:None,..} if fill.0[3]==0.))
        );
    }
    let element = bbt_ui::components::Element::Glyph {
        shadow: None,
        kind_colors: KindColors {
            foreground: Some(Color::WHITE),
            background: Some(Color::TRANSPARENT),
        },
        activity: Activity::Waiting(WaitKind::Question),
        grade: Grade::Inferred,
        color: bbt_ui::Paint::solid(bbt_ui::Color::Seat),
        size: 12.,
        scale: 1.,
    };
    let layout = element
        .layout(
            Rect::new(20., 20., 12., 12.),
            bbt_ui::Context::new(Theme::Dark, Density::Cozy),
            &Metrics,
        )
        .unwrap();
    let bbt_ui::components::LayoutKind::Atom(list) = layout.kind else {
        panic!("not an atom")
    };
    assert!(
        matches!(list.primitives.last(),Some(Prim::Text {color,..}) if color.0==[1.,1.,1.,0.72]),
        "retained Glyph must forward the annotation colours"
    );
}
