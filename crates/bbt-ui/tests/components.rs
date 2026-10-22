use bbt_ui::{
    Color, Context, Density, Length, Overrides, Paint, Preset, Style, Theme, components as c,
};
use bbt_ui_model::{
    draw::Prim,
    layout::{Axis, Rect, TextMeasure, TextStyle},
};
struct Metrics;
impl TextMeasure for Metrics {
    fn width(&self, text: &str, s: &TextStyle) -> f32 {
        text.chars().count() as f32 * s.size * 0.5
    }
}
impl c::Measure for Metrics {
    fn font_metrics(&self, s: &TextStyle) -> [f32; 2] {
        [s.size * 0.8, s.size * 0.2]
    }
}
fn context() -> Context {
    Context::new(Theme::Dark, Density::Cozy)
}
fn text(value: &str, size: bbt_ui::TypeStep) -> c::Element {
    c::Element::Text(c::Text::new(
        value,
        bbt_ui::Text {
            step: size,
            ..Default::default()
        },
    ))
}
fn box_at_height(h: f32) -> c::Element {
    let mut s = Style::new(Preset::Row);
    s.overrides.height = Some(Length::Px(h));
    c::Element::Box(c::Box::new(
        s,
        vec![text("A", bbt_ui::TypeStep::Small).into()],
    ))
}
fn stack(axis: Axis, gap: f32, align: c::Alignment, children: Vec<c::Item>) -> c::Element {
    c::Element::Stack(c::Stack {
        direction: axis,
        gap: Length::Px(gap),
        align,
        children,
    })
}
#[test]
fn column_snapshot_and_flex_spacer_are_solved_by_the_shared_engine() {
    let column = stack(
        Axis::Column,
        6.,
        c::Alignment::Stretch,
        vec![box_at_height(20.).into(), box_at_height(30.).into()],
    );
    let l = column
        .layout(Rect::new(10., 20., 100., 0.), context(), &Metrics)
        .unwrap();
    assert_eq!(l.rect, Rect::new(10., 20., 100., 56.));
    assert_eq!(l.children[0].rect, Rect::new(10., 20., 100., 20.));
    assert_eq!(l.children[1].rect, Rect::new(10., 46., 100., 30.));
    let row = stack(
        Axis::Row,
        6.,
        c::Alignment::Center,
        vec![
            text("left", bbt_ui::TypeStep::Small).into(),
            c::Element::Spacer.into(),
            text("right", bbt_ui::TypeStep::Small).into(),
        ],
    );
    let l = row
        .layout(Rect::new(10., 20., 200., 0.), context(), &Metrics)
        .unwrap();
    assert_eq!(l.children[2].rect.right(), 210.);
    assert_eq!(l.children[1].rect.w, 143.);
}
#[test]
fn mixed_type_rows_share_a_baseline() {
    let row = stack(
        Axis::Row,
        8.,
        c::Alignment::Baseline,
        vec![
            text("micro", bbt_ui::TypeStep::Micro).into(),
            text("title", bbt_ui::TypeStep::Title).into(),
        ],
    );
    let l = row
        .layout(Rect::new(0., 0., 200., 0.), context(), &Metrics)
        .unwrap();
    let baseline = |i: usize| l.children[i].rect.y + l.children[i].baseline;
    assert!((baseline(0) - baseline(1)).abs() < 0.0001);
    assert!(l.children[0].rect.y > l.children[1].rect.y);
}
#[test]
fn scroll_offset_clamps_and_every_child_draw_is_clipped() {
    let content = stack(
        Axis::Column,
        5.,
        c::Alignment::Stretch,
        vec![
            box_at_height(30.).into(),
            box_at_height(30.).into(),
            box_at_height(30.).into(),
        ],
    );
    let scroll = c::Element::ScrollView(c::ScrollView {
        viewport: [100., 40.],
        offset: [0., 90.],
        content_width: None,
        content: std::boxed::Box::new(content),
    });
    let l = scroll
        .layout(Rect::new(10., 20., 100., 40.), context(), &Metrics)
        .unwrap();
    assert!(matches!(
        l.kind,
        c::LayoutKind::ScrollView {
            offset: [0., 60.],
            content: [100., 100.]
        }
    ));
    assert_eq!(l.children[0].rect.y, -40.);
    for op in c::paint(&l, context()) {
        if let c::Paint::Draw(list) = op {
            list.validate().unwrap();
            assert!(matches!(list.primitives.first(),Some(Prim::Clip {rect,..}) if *rect==l.rect));
        }
    }
}
#[test]
fn wrapping_and_ellipsis_use_the_allocated_width() {
    let t = text("alpha beta gamma", bbt_ui::TypeStep::Small);
    let l = t
        .layout(Rect::new(0., 0., 40., 0.), context(), &Metrics)
        .unwrap();
    assert!((l.rect.h - 40.5).abs() < 0.001);
    let mut t = c::Text::new(
        "a long label",
        bbt_ui::Text {
            step: bbt_ui::TypeStep::Small,
            ..Default::default()
        },
    );
    t.wrap = false;
    t.ellipsis = true;
    let s = Style {
        tokens: Preset::Card,
        overrides: Overrides {
            padding: Some([Length::Px(0.); 4]),
            border: Some(bbt_ui::Border {
                width: Length::Px(0.),
                paint: Paint::solid(Color::Transparent),
                outside: false,
            }),
            ..Default::default()
        },
    };
    let tree = c::Element::Box(c::Box::new(s, vec![c::Element::Text(t).into()]));
    let l = tree
        .layout(Rect::new(0., 0., 30., 0.), context(), &Metrics)
        .unwrap();
    assert_eq!(l.children[0].rect.w, 30.);
    let ops = c::paint(&l, context());
    assert!(ops.iter().any(|op|matches!(op,c::Paint::Draw(list) if list.primitives.iter().any(|p|matches!(p,Prim::Text {max_width:Some(w),..} if *w==30.)))));
}

#[test]
fn a_shrunk_row_remeasures_wrapped_text_before_choosing_its_height() {
    let row = stack(
        Axis::Row,
        10.,
        c::Alignment::Start,
        vec![
            c::Item {
                element: box_at_height(10.),
                extent: c::Extent::Fixed(30.),
            },
            text("alpha beta gamma", bbt_ui::TypeStep::Small).into(),
        ],
    );
    let l = row
        .layout(Rect::new(0., 0., 80., 0.), context(), &Metrics)
        .unwrap();
    assert_eq!(l.children[1].rect.w, 40.);
    assert_eq!(l.rect.h, 40.5);
    assert_eq!(l.children[1].rect.h, 40.5);
}

#[test]
fn mixed_text_runs_align_baselines_and_keep_selection_metadata() {
    let mut t = c::Text::new(
        "small",
        bbt_ui::Text {
            step: bbt_ui::TypeStep::Micro,
            ..Default::default()
        },
    );
    t.runs.push(c::TextRun {
        value: "large".into(),
        style: bbt_ui::Text {
            step: bbt_ui::TypeStep::Title,
            ..Default::default()
        },
    });
    let l = c::Element::Text(t)
        .layout(Rect::new(0., 0., 200., 0.), context(), &Metrics)
        .unwrap();
    let c::LayoutKind::Text {
        runs, selectable, ..
    } = &l.kind
    else {
        panic!("Text layout")
    };
    assert!(*selectable);
    assert_eq!(runs.len(), 2);
    assert!(runs[0].rect.y > runs[1].rect.y);
    assert_eq!(l.baseline, 15.);
}
