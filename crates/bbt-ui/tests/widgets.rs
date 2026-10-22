use bbt_ui::{self as ui, components as c, widgets as w};
use bbt_ui_model::layout::{Rect, TextMeasure, TextStyle};
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
fn count(l: &c::Layout) -> usize {
    1 + l.children.iter().map(count).sum::<usize>()
}

#[test]
fn list_and_table_allocate_only_visible_rows_and_keep_full_scroll_extent() {
    let list = w::List {
        items: (0..1000)
            .map(|i| w::ListItem {
                label: format!("seat {i}"),
                subtitle: None,
                icon: None,
                key: None,
            })
            .collect(),
        selected: Some(500),
        more: 0,
        viewport: Some(w::Viewport {
            height: 120.,
            offset: 13_000.,
        }),
    };
    let l = list
        .element(ctx(), 300.)
        .layout(Rect::new(0., 0., 300., 0.), ctx(), &Metrics)
        .unwrap();
    assert!(
        count(&l) < 50,
        "a 1000-row model must not create 1000 layout nodes"
    );
    let c::LayoutKind::ScrollView { content, offset } = &l.kind else {
        panic!("scroll view")
    };
    assert_eq!(content[1], 25_998.);
    assert_eq!(offset[1], 13_000.);
    let table = w::Table {
        columns: vec![
            w::Column {
                title: "seat".into(),
                weight: 1.,
            },
            w::Column {
                title: "role".into(),
                weight: 1.,
            },
        ],
        rows: (0..1000)
            .map(|i| vec![format!("seat {i}"), "executor".into()])
            .collect(),
        selected: Some(500),
        zebra: true,
        viewport: Some(w::Viewport {
            height: 120.,
            offset: 12_000.,
        }),
    };
    let l = table
        .element(ctx(), 300.)
        .unwrap()
        .layout(Rect::new(0., 0., 300., 0.), ctx(), &Metrics)
        .unwrap();
    assert!(count(&l) < 50);
    let c::LayoutKind::ScrollView { content, offset } = &l.children[1].kind else {
        panic!("table body scroll view")
    };
    assert_eq!(content[1], 24_000.);
    assert_eq!(offset[1], 12_000.);
}

#[test]
fn pressed_button_scales_visually_without_changing_its_flow_slot() {
    let mut button = w::Button {
        label: "launch".into(),
        kind: w::ButtonKind::Primary,
        state: w::ButtonState::Idle,
        icon: None,
        key: None,
    };
    let idle = button.element(ctx()).unwrap();
    let before = idle.natural(300., ctx(), &Metrics).unwrap();
    button.state = w::ButtonState::Pressed;
    let pressed = button.element(ctx()).unwrap();
    assert_eq!(before, pressed.natural(300., ctx(), &Metrics).unwrap());
    let l = pressed
        .layout(Rect::new(0., 0., 100., 0.), ctx(), &Metrics)
        .unwrap();
    assert_eq!(l.rect.w, 97.);
    assert_eq!(l.rect.x, 1.5);
    assert!(l.children[0].rect.h < before.0[1]);
}

#[test]
fn side_border_consumes_only_its_edge_and_outline_rings_do_not_consume_layout() {
    let mut style = ui::Style::new(ui::Preset::Card);
    style.overrides.padding = Some([ui::Length::Px(0.); 4]);
    style.overrides.border_sides = Some([false, false, true, false]);
    style.overrides.rings = Some(vec![ui::Border {
        width: ui::Length::Px(3.),
        paint: ui::Paint::solid(ui::Color::Accent),
        outside: true,
    }]);
    let serialized = serde_json::to_vec(&style).unwrap();
    let style: ui::Style = serde_json::from_slice(&serialized).unwrap();
    let text = c::Element::Text(c::Text::new(
        "row",
        ui::Text {
            step: ui::TypeStep::Small,
            ..Default::default()
        },
    ));
    let l = c::Element::Box(c::Box::new(style, vec![text.into()]))
        .layout(Rect::new(5., 10., 100., 0.), ctx(), &Metrics)
        .unwrap();
    assert_eq!(l.rect.h, 14.5);
    assert_eq!(l.children[0].rect.x, 5.);
    assert_eq!(l.children[0].rect.y, 10.);
}

#[test]
fn empty_input_keeps_its_control_height_and_invalid_input_keeps_its_error_caption() {
    let mut input = w::Input {
        value: String::new(),
        placeholder: String::new(),
        state: w::InputState::Empty,
        icon: None,
        error: Some("type the full name".into()),
    };
    let empty = input
        .element(ctx())
        .layout(Rect::new(0., 0., 300., 0.), ctx(), &Metrics)
        .unwrap();
    assert!(empty.rect.h > 25.);
    input.state = w::InputState::Invalid;
    let invalid = input
        .element(ctx())
        .layout(Rect::new(0., 0., 300., 0.), ctx(), &Metrics)
        .unwrap();
    assert!(invalid.rect.h > empty.rect.h);
    assert_eq!(invalid.children.len(), 2);
}
