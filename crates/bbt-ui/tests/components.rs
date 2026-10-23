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

fn edit_part<'a>(node: &'a c::Layout, part: &str) -> &'a c::Layout {
    if matches!(&node.kind, c::LayoutKind::Named(key) if key == part) {
        return node;
    }
    node.children
        .iter()
        .find_map(|child| {
            fn find<'a>(n: &'a c::Layout, key: &str) -> Option<&'a c::Layout> {
                if matches!(&n.kind,c::LayoutKind::Named(k) if k == key) {
                    return Some(n);
                }
                n.children.iter().find_map(|ch| find(ch, key))
            }
            find(child, part)
        })
        .unwrap_or_else(|| panic!("missing {part}"))
}

#[test]
fn field_marks_use_utf16_boundaries_and_keep_preedit_out_of_intrinsic_size() {
    let mut field = c::EditingText {
        value: "A😀B".into(),
        placeholder: "empty".into(),
        style: Default::default(),
        placeholder_paint: Paint::solid(Color::Muted),
        edit: c::Edit {
            caret: Some(3),
            ..Default::default()
        },
        name: "field".into(),
    };
    let root = |field: &c::EditingText| {
        c::Element::EditingText(field.clone()).layout(
            Rect::new(20., 30., 300., 0.),
            context(),
            &Metrics,
        )
    };
    let before = root(&field).unwrap();
    let caret = edit_part(&before, "field/caret");
    assert_eq!(
        caret.rect.x,
        20. + 2. * field.style.step.pixels() * 0.5 - 1.
    );
    assert_eq!(caret.rect.w, 2.);
    field.edit.preedit = Some(c::Preedit {
        text: "你好".into(),
        caret: Some(1),
    });
    let marked = root(&field).unwrap();
    assert_eq!(
        before.rect, marked.rect,
        "marked text cannot resize the committed query"
    );
    assert_eq!(edit_part(&marked, "field/underline").rect.h, 1.);
    assert!(edit_part(&marked, "field/caret").rect.x > caret.rect.x);
    field.edit = c::Edit {
        select: Some([1, 3]),
        ..Default::default()
    };
    let selected = root(&field).unwrap();
    assert_eq!(
        edit_part(&selected, "field/selection").rect,
        edit_part(&selected, "field/selected").rect
    );
    assert!(!format!("{selected:?}").contains("field/caret"));
    field.edit = c::Edit {
        caret: Some(2),
        ..Default::default()
    };
    assert!(root(&field).is_err(), "a surrogate pair cannot be split");
    assert_eq!(c::utf16_byte_index("A😀B", 3), Some(5));
    assert_eq!(c::utf16_byte_index("A😀B", 2), None);
    field.value = "AoldB".into();
    field.edit = c::Edit {
        caret: Some(1),
        replacement: Some([1, 4]),
        preedit: Some(c::Preedit {
            text: "界".into(),
            caret: Some(1),
        }),
        ..Default::default()
    };
    let preview = root(&field).unwrap();
    let visible = c::paint(&preview, context())
        .into_iter()
        .filter_map(|op| match op {
            c::Paint::Draw(list) => Some(list),
            _ => None,
        })
        .flat_map(|list| list.primitives)
        .filter_map(|p| match p {
            Prim::Text { text, .. } => Some(text.to_string()),
            _ => None,
        })
        .collect::<String>();
    assert_eq!(
        visible, "A界B",
        "marked text replaces the selection only in paint"
    );
    assert_eq!(
        field.value, "AoldB",
        "the host has not committed the composition"
    );
}

#[test]
fn rects_lay_out_as_one_slot_and_paint_box_rects_at_the_node_origin() {
    let element = c::Element::Rects(c::Rects {
        size: [6., 43.],
        rects: vec![
            (Rect::new(0., 13.6, 2., 19.8), Paint::solid(Color::Accent)),
            (Rect::new(4., 41., 2., 2.), Paint::solid(Color::Muted)),
        ]
        .into(),
    });
    let layout = element
        .layout(Rect::new(26.25, 53.5, 6., 0.), context(), &Metrics)
        .unwrap();
    assert_eq!(layout.rect, Rect::new(26.25, 53.5, 6., 43.));
    assert!(
        layout.children.is_empty(),
        "one node, whatever the rect count"
    );
    let ops = c::paint(&layout, context());
    let [c::Paint::Draw(list)] = ops.as_slice() else {
        panic!("rects draw")
    };
    let rects: Vec<_> = list
        .primitives
        .iter()
        .map(|p| match p {
            Prim::Rect {
                rect,
                radii,
                stroke: None,
                inner_highlight: None,
                shadow: None,
                ..
            } if *radii == [0.; 4] => *rect,
            other => panic!("a plain filled rect, not {other:?}"),
        })
        .collect();
    // Exact fractional edges: the renderer snaps them in device space, as for boxes.
    assert_eq!(
        rects,
        [
            Rect::new(26.25, 67.1, 2., 19.8),
            Rect::new(30.25, 94.5, 2., 2.)
        ]
    );
    let bad = c::Element::Rects(c::Rects {
        size: [6., 43.],
        rects: vec![(Rect::new(0., 0., -1., 2.), Paint::solid(Color::Accent))].into(),
    });
    assert!(
        bad.layout(Rect::new(0., 0., 6., 0.), context(), &Metrics)
            .is_err()
    );
}

#[test]
fn path_uses_declared_viewport_and_keeps_its_retained_stroke() {
    let element = c::Element::Path(c::Path {
        size: [80., 20.],
        points: vec![
            c::PathCommand::Move([2., 10.]),
            c::PathCommand::Cubic([[20., 0.], [60., 20.], [78., 10.]]),
        ]
        .into(),
        stroke: c::PathStroke {
            width: 1.5,
            cap: c::LineCap::Round,
            ..Default::default()
        },
        fill: None,
        paint: Paint::solid(Color::Accent),
        opacity: 0.4,
    });
    let layout = element
        .layout(Rect::new(3., 7., 80., 0.), context(), &Metrics)
        .unwrap();
    assert_eq!(layout.rect, Rect::new(3., 7., 80., 20.));
    let ops = c::paint(&layout, context());
    let [c::Paint::Draw(list)] = ops.as_slice() else {
        panic!("path draw")
    };
    let [
        Prim::Path {
            origin,
            points,
            stroke,
            color,
            scale,
            fill,
        },
    ] = list.primitives.as_slice()
    else {
        panic!("retained path")
    };
    assert_eq!(*origin, [3., 7.]);
    assert_eq!(*scale, 1.);
    assert_eq!(*fill, None);
    assert_eq!(points.len(), 2);
    assert_eq!(stroke.width, 1.5);
    assert!((color.0[3] - 0.4).abs() < 1e-6);
    assert!(
        c::Element::Path(c::Path {
            size: [-1., 0.],
            points: vec![].into(),
            stroke: Default::default(),
            fill: None,
            paint: Paint::solid(Color::Accent),
            opacity: 1.
        })
        .natural(20., context(), &Metrics)
        .is_err()
    );
}

#[test]
fn intrinsic_flow_reserves_a_content_minimum_even_when_natural_width_is_zero() {
    let mut style = Style::new(Preset::Card);
    style.overrides.padding = Some([Length::Px(0.); 4]);
    style.overrides.border = Some(bbt_ui::Border {
        width: Length::Px(0.),
        paint: bbt_ui::Paint::solid(bbt_ui::Color::Transparent),
        outside: false,
    });
    let mut swatch = style.clone();
    swatch.overrides.width = Some(Length::Px(10.));
    swatch.overrides.height = Some(Length::Px(10.));
    let child = c::Item {
        element: c::Element::Box(c::Box::new(swatch, vec![])),
        extent: c::Extent::Content {
            grow: 0.,
            shrink: 1.,
            min: 2.,
            max: None,
            max_fraction: None,
        },
    };
    let parent = c::Element::Box(c::Box::new(style, vec![child]));
    let (size, _) = parent.natural(0., context(), &Metrics).unwrap();
    assert_eq!(size[0], 2.);
    let layout = parent
        .layout(Rect::new(0., 0., 0., 0.), context(), &Metrics)
        .unwrap();
    assert_eq!(layout.children[0].rect.w, 2.);
}

#[test]
fn path_fill_survives_retained_layout_and_theme_resolution() {
    let element = c::Element::Path(c::Path {
        size: [20., 20.],
        points: vec![
            c::PathCommand::Move([0., 0.]),
            c::PathCommand::Line([20., 0.]),
            c::PathCommand::Line([10., 20.]),
        ]
        .into(),
        stroke: c::PathStroke {
            width: 0.,
            ..Default::default()
        },
        paint: Paint::solid(Color::Foreground),
        fill: Some(Paint::solid(Color::Accent)),
        opacity: 0.5,
    });
    let layout = element
        .layout(Rect::new(3., 7., 20., 20.), context(), &Metrics)
        .unwrap();
    let ops = c::paint(&layout, context());
    let [c::Paint::Draw(list)] = ops.as_slice() else {
        panic!("path draw")
    };
    let [
        Prim::Path {
            fill: Some(fill),
            origin,
            stroke,
            ..
        },
    ] = list.primitives.as_slice()
    else {
        panic!("filled path")
    };
    assert_eq!(*origin, [3., 7.]);
    assert_eq!(stroke.width, 0.);
    let expected = Paint::solid(Color::Accent).resolve(context()).unwrap();
    assert_eq!(&fill.0[..3], &expected[..3]);
    assert!((fill.0[3] - 0.5).abs() < 1e-6);
}

#[test]
fn natural_text_width_survives_float_subtraction_but_a_css_unit_still_wraps() {
    let mut text = c::Text::new("one two", Default::default());
    text.wrap = true;
    let element = c::Element::Text(text);
    let (full, _) = element.natural(f32::INFINITY, context(), &Metrics).unwrap();
    let (roundoff, _) = element
        .natural(full[0] - 0.0001, context(), &Metrics)
        .unwrap();
    assert_eq!(roundoff[1], full[1]);
    let (short, _) = element
        .natural(full[0] - 1. / 64., context(), &Metrics)
        .unwrap();
    assert!(short[1] > full[1]);
}

#[test]
fn image_has_square_intrinsics_a_baseline_and_validated_asset_geometry() {
    let make = |asset: &str, size, radius| c::Element::Image {
        asset: asset.into(),
        size: Length::Px(size),
        radius: Length::Px(radius),
    };
    let image = make("bboxy", 18., 9.);
    let layout = image
        .layout(Rect::new(3., 7., 18., 0.), context(), &Metrics)
        .unwrap();
    assert_eq!(layout.rect, Rect::new(3., 7., 18., 18.));
    assert_eq!(layout.baseline, 18.);
    assert!(matches!(&layout.kind,c::LayoutKind::Image {asset,radius:9.} if asset=="bboxy"));
    let mut parent = c::Box::new(Style::new(Preset::Row), vec![image.into()]);
    parent.scale = 0.5;
    let scaled = c::Element::Box(parent)
        .layout(Rect::new(0., 0., 50., 0.), context(), &Metrics)
        .unwrap();
    assert_eq!(scaled.children[0].rect.w, 9.);
    assert!(matches!(
        &scaled.children[0].kind,
        c::LayoutKind::Image { radius: 4.5, .. }
    ));
    for bad in [
        make("", 18., 9.),
        make("bboxy", -1., 9.),
        make("bboxy", 18., f32::NAN),
    ] {
        assert!(
            bad.layout(Rect::new(0., 0., 18., 0.), context(), &Metrics)
                .is_err()
        );
    }
}

#[test]
fn box_opacity_dim_and_forced_isolation_preserve_unfiltered_children_and_layout() {
    let mut style = Style::new(Preset::Row);
    style.overrides.fill = Some(Paint::solid(Color::Rgba([1., 0., 0., 1.])));
    style.overrides.opacity = Some(0.5);
    style.overrides.dim = Some(true);
    let element = c::Element::Box(c::Box::new(
        style,
        vec![text("overlap", bbt_ui::TypeStep::Body).into()],
    ));
    let layout = element
        .layout(Rect::new(0., 0., 100., 0.), context(), &Metrics)
        .unwrap();
    let ops = c::paint(&layout, context());
    let [
        c::Paint::Group {
            ops,
            opacity,
            saturation,
            ..
        },
    ] = ops.as_slice()
    else {
        panic!("missing isolated Box")
    };
    assert_eq!(
        (*opacity, *saturation),
        (
            0.5 * bbt_ui_model::TOKENS.dim.opacity as f32,
            bbt_ui_model::TOKENS.dim.saturate as f32
        )
    );
    assert!(ops.iter().any(|p|matches!(p,c::Paint::Draw(d) if d.primitives.iter().any(|p|matches!(p,Prim::Rect {fill,..} if fill.0==[1.,0.,0.,1.])))));
    assert!(ops.iter().any(|p|matches!(p,c::Paint::Draw(d) if d.primitives.iter().any(|p|matches!(p,Prim::Text {color,..} if color.0[3]==1.)))));
    let mut plain = c::Box::new(Style::new(Preset::Row), vec![]);
    let before = c::Element::Box(plain.clone())
        .layout(Rect::new(0., 0., 100., 0.), context(), &Metrics)
        .unwrap();
    plain.group = true;
    let after = c::Element::Box(plain)
        .layout(Rect::new(0., 0., 100., 0.), context(), &Metrics)
        .unwrap();
    assert_eq!(before.rect, after.rect);
    assert_eq!(before.baseline, after.baseline);
    assert!(matches!(
        c::paint(&after, context()).as_slice(),
        [c::Paint::Group {
            opacity: 1.,
            saturation: 1.,
            ..
        }]
    ));
}

#[test]
fn nested_filters_retain_boundaries_and_ancestor_clips_without_recoloring_the_atoms() {
    let leaf = text("grey once per group", bbt_ui::TypeStep::Body);
    let inner = c::Element::Filter {
        content: Box::new(leaf),
        saturation: 0.4,
        opacity: 0.5,
    };
    let outer = c::Element::Filter {
        content: Box::new(inner),
        saturation: 0.1,
        opacity: 0.7,
    };
    let layout = outer
        .layout(Rect::new(0., 0., 180., 0.), context(), &Metrics)
        .unwrap();
    let clip = c::Clip {
        rect: Rect::new(2., 2., 50., 20.),
        radii: [4.; 4],
    };
    let ops = c::paint_with_ancestor_clips(
        &layout,
        bbt_ui_model::draw::DrawCtx::new(Theme::Dark, Density::Cozy),
        &[clip],
    );
    let [
        c::Paint::Group {
            ops,
            opacity: 0.7,
            saturation: 0.1,
            ..
        },
    ] = ops.as_slice()
    else {
        panic!("outer boundary lost")
    };
    let [
        c::Paint::Group {
            ops,
            opacity: 0.5,
            saturation: 0.4,
            ..
        },
    ] = ops.as_slice()
    else {
        panic!("inner boundary lost")
    };
    let [c::Paint::Draw(list)] = ops.as_slice() else {
        panic!("text missing")
    };
    assert!(matches!(list.primitives.first(),Some(Prim::Clip {rect,..}) if *rect==clip.rect));
    assert!(matches!(list.primitives.last(), Some(Prim::Unclip)));
    assert!(list.validate().is_ok());
}

#[test]
fn a_retained_box_keeps_its_own_glow_state_until_after_its_content() {
    let state = bbt_ui::Glow {
        activity: bbt_ui::GlowActivity::Waiting,
        inferred_multiplier: 0.5,
        seed: 42,
        ..Default::default()
    };
    let mut s = Style::new(Preset::PaneGlow);
    s.overrides.glow = Some(state);
    let element = c::Element::Box(c::Box::new(
        s,
        vec![text("foreground", bbt_ui::TypeStep::Body).into()],
    ));
    let layout = element
        .layout(Rect::new(0., 0., 200., 0.), context(), &Metrics)
        .unwrap();
    let ops = c::paint(&layout, context());
    assert!(
        matches!(ops.last(), Some(c::Paint::Material { glow: Some(actual), .. }) if *actual==state)
    );
    assert!(ops.iter().any(|op| matches!(op,c::Paint::Draw(d) if d.primitives.iter().any(|p|matches!(p,Prim::Text{..})))));
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

#[test]
fn inline_middle_uses_the_parent_x_height_without_moving_the_content_box() {
    struct XHeight;
    impl bbt_ui_model::layout::TextMeasure for XHeight {
        fn width(&self, value: &str, style: &bbt_ui_model::layout::TextStyle) -> f32 {
            value.len() as f32 * style.size * 0.5
        }
    }
    impl c::Measure for XHeight {
        fn font_metrics(&self, _: &bbt_ui_model::layout::TextStyle) -> [f32; 2] {
            [11., 3.]
        }
        fn x_height(&self, _: &bbt_ui_model::layout::TextStyle) -> f32 {
            6.
        }
    }
    let ctx = bbt_ui::Context::new(bbt_ui::Theme::Dark, bbt_ui::Density::Cozy);
    let mut style = bbt_ui::Style::new(bbt_ui::Preset::Card);
    style.overrides.width = Some(bbt_ui::Length::Px(40.));
    style.overrides.height = Some(bbt_ui::Length::Px(16.));
    style.overrides.padding = Some([bbt_ui::Length::Px(0.); 4]);
    let plain = c::Element::Box(c::Box::new(style, vec![]));
    let normal = plain
        .layout(
            bbt_ui_model::layout::Rect::new(5., 7., 40., 0.),
            ctx,
            &XHeight,
        )
        .unwrap();
    let middle = c::Element::InlineMiddle {
        content: Box::new(plain),
        parent: bbt_ui::Text::default(),
    }
    .layout(
        bbt_ui_model::layout::Rect::new(5., 7., 40., 0.),
        ctx,
        &XHeight,
    )
    .unwrap();
    assert_eq!(normal.rect, middle.rect);
    assert_eq!(middle.baseline, 11.);
    let mut box_style = bbt_ui::Style::new(bbt_ui::Preset::Card);
    box_style.overrides.padding = Some([bbt_ui::Length::Px(0.); 4]);
    box_style.overrides.text = Some(bbt_ui::Text {
        size: Some(11.5),
        line_height: 1.4,
        ..Default::default()
    });
    let mut parent = c::Box::new(
        box_style,
        vec![
            c::Element::InlineMiddle {
                content: Box::new(c::Element::Spacer),
                parent: bbt_ui::Text::default(),
            }
            .into(),
        ],
    );
    parent.inline = true;
    parent.align = c::Alignment::Baseline;
    assert!(
        c::Element::Box(parent)
            .layout(
                bbt_ui_model::layout::Rect::new(0., 0., 100., 0.),
                ctx,
                &XHeight
            )
            .is_ok()
    );
}
