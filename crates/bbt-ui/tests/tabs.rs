use bbt_ui::{
    self as ui, components as c,
    modules::{
        mini_grid::{MiniGrid, Node},
        seat_row::SeatRow,
        tab_row::{TabRow, TabSeat},
    },
};
use bbt_ui_model::{
    Activity, WaitKind,
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
fn ctx() -> ui::Context {
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
        out.extend(runs.iter().map(|r| r.text.clone()))
    }
    for ch in &l.children {
        out.extend(labels(ch))
    }
    out
}
fn cell(k: &str, state: Activity) -> Node {
    Node::Cell {
        key: k.into(),
        name: format!("{k} name"),
        activity: state,
    }
}
fn split(axis: Axis, ratio: f32, a: Node, b: Node) -> Node {
    Node::Split {
        axis,
        ratio,
        a: Box::new(a),
        b: Box::new(b),
    }
}
fn tab(states: &[Activity]) -> TabRow {
    let seats = states
        .iter()
        .enumerate()
        .map(|(i, state)| {
            let mut row = SeatRow::new(i.to_string(), format!("Seat {i}"), "codex");
            row.activity = *state;
            TabSeat {
                row,
                children: vec![],
            }
        })
        .collect::<Vec<_>>();
    let tree = if seats.len() == 1 {
        cell("0", states[0])
    } else {
        split(Axis::Row, 0.5, cell("0", states[0]), cell("1", states[1]))
    };
    TabRow {
        name: "test tab".into(),
        shortcut: Some("⌘3".into()),
        tree,
        seats,
        expanded: false,
        focused_key: None,
    }
}
fn layout(e: c::Element, w: f32) -> c::Layout {
    e.layout(Rect::new(0., 0., w, 0.), ctx(), &Metrics).unwrap()
}
#[test]
fn grid_preserves_ratio_nested_axis_and_big_root_gap() {
    let grid = MiniGrid {
        tree: split(
            Axis::Row,
            0.3,
            cell("a", Activity::Idle),
            split(
                Axis::Column,
                0.5,
                cell("b", Activity::Idle),
                cell("c", Activity::Idle),
            ),
        ),
        focused_key: None,
        size: [240., 160.],
        big: true,
    };
    let l = layout(grid.element(ctx()), 240.);
    let a = find(&l, "grid/cell").unwrap().rect;
    let b = find(&l, "grid/mg/cell").unwrap().rect;
    let c = find(&l, "grid/mg/cell1").unwrap().rect;
    assert!((a.w - 71.1).abs() < 0.001);
    assert!((b.x - a.right() - 3.).abs() < 0.001);
    assert!((c.y - b.bottom() - 1.).abs() < 0.001);
    assert_eq!(labels(&l), ["a", "b", "c"]);
    let c = find(&l, "grid/mg/cell/text").unwrap().rect;
    assert!((c.x + c.w * 0.5 - (b.x + b.w * 0.5)).abs() < 0.001);
}
#[test]
fn deeply_nested_grids_shrink_to_zero_but_their_cells_keep_the_two_pixel_minimum() {
    let mut tree = cell("end", Activity::Idle);
    for key in ["e", "d", "c", "b", "a"] {
        tree = split(Axis::Column, 0.5, cell(key, Activity::Idle), tree)
    }
    let grid = MiniGrid {
        tree,
        focused_key: None,
        size: [24., 16.],
        big: false,
    };
    let l = layout(grid.element(ctx()), 24.);
    assert_eq!(find(&l, "grid/cell").unwrap().rect.h, 7.5);
    assert_eq!(find(&l, "grid/mg").unwrap().rect.h, 7.5);
    assert_eq!(find(&l, "grid/mg/mg/mg").unwrap().rect.h, 0.25);
    assert_eq!(find(&l, "grid/mg/mg/mg/cell").unwrap().rect.h, 2.);
    assert!(matches!(
        find(&l, "grid/mg").unwrap().children[0].kind,
        c::LayoutKind::Box { clip: true, .. }
    ));
    let row = MiniGrid {
        tree: split(
            Axis::Row,
            0.5,
            cell("a", Activity::Idle),
            cell("b", Activity::Idle),
        ),
        focused_key: None,
        size: [24., 0.],
        big: false,
    };
    let l = layout(row.element(ctx()), 24.);
    assert_eq!(find(&l, "grid").unwrap().rect.h, 0.);
    assert_eq!(find(&l, "grid/cell").unwrap().rect.h, 2.);
    assert_eq!(find(&l, "grid/cell1").unwrap().rect.h, 2.);
}
#[test]
fn state_color_wins_over_focus_and_a_single_cell_keeps_the_grid_wrapper() {
    use bbt_ui_model::draw::{Color, Prim};
    let grid = MiniGrid {
        tree: cell("a", Activity::Waiting(WaitKind::Permission)),
        focused_key: Some("a".into()),
        size: [24., 16.],
        big: false,
    };
    let l = layout(grid.element(ctx()), 24.);
    assert_eq!(
        find(&l, "grid").unwrap().rect,
        find(&l, "grid/cell").unwrap().rect
    );
    let fills = c::paint(&l, ctx())
        .into_iter()
        .flat_map(|op| match op {
            c::Paint::Draw(d) | c::Paint::Underlay(d) => d.primitives,
            _ => vec![],
        })
        .filter_map(|p| match p {
            Prim::Rect { fill, .. } => Some(fill),
            _ => None,
        })
        .collect::<Vec<_>>();
    let expected = Color::hex(bbt_ui_model::TOKENS.color.state.waiting)
        .mix(Color::hex(ui::Theme::Dark.palette().surface_2), 0.25);
    assert!(fills.contains(&expected));
    assert!(!fills.contains(&Color::hex(ui::Theme::Dark.palette().accent)));
}
#[test]
fn collapsed_rollup_is_stable_worst_first_and_capped() {
    let t = tab(&[
        Activity::Idle,
        Activity::Active,
        Activity::Waiting(WaitKind::Auth),
        Activity::Errored,
        Activity::Active,
        Activity::Idle,
        Activity::Errored,
        Activity::Active,
    ]);
    let element = t.element(ctx());
    let l = layout(element.clone(), 300.);
    let mut activities = vec![];
    fn atoms(n: &c::Element, out: &mut Vec<Activity>) {
        match n {
            c::Element::Glyph { activity, .. } => out.push(*activity),
            c::Element::Named { content, .. } => atoms(content, out),
            c::Element::Box(b) => {
                for ch in &b.children {
                    atoms(&ch.element, out)
                }
            }
            c::Element::Stack(s) => {
                for ch in &s.children {
                    atoms(&ch.element, out)
                }
            }
            _ => {}
        }
    }
    atoms(&element, &mut activities);
    assert_eq!(
        activities,
        [
            Activity::Errored,
            Activity::Errored,
            Activity::Waiting(WaitKind::Auth),
            Activity::Active,
            Activity::Active,
            Activity::Active
        ]
    );
    assert_eq!(labels(find(&l, "header/roll/ellip").unwrap()), ["…"]);
    assert!(find(&l, "body").is_none());
}
#[test]
fn single_seat_reduces_to_the_accepted_row_and_keeps_its_children() {
    let mut t = tab(&[Activity::Idle]);
    t.focused_key = Some("0".into());
    let mut nested = c::Text::new("nested task", ui::Text::default());
    nested.wrap = false;
    t.seats[0].children.push(ui::modules::seat_row::named(
        "row",
        c::Element::Text(nested),
    ));
    let l = layout(t.element(ctx()), 300.);
    assert!(find(&l, "header").is_none());
    assert!(find(&l, "header/right/mg").is_none());
    assert!(find(&l, "seat0/row").is_some());
    assert!(find(&l, "seat0/child0/row").is_some());
    assert!(labels(&l).contains(&"⌘3".into()));
    assert!(labels(&l).join(" ").contains("nested task"));
}
#[test]
fn expanded_tab_has_indented_seats_and_only_a_parent_shortcut() {
    let mut t = tab(&[Activity::Idle, Activity::Active, Activity::Idle]);
    t.expanded = true;
    t.focused_key = Some("1".into());
    let l = layout(t.element(ctx()), 300.);
    assert!(find(&l, "header/roll").is_none());
    assert_eq!(labels(&l).iter().filter(|s| s.as_str() == "⌘3").count(), 1);
    let a = find(&l, "seat0/row").unwrap().rect;
    let b = find(&l, "seat1/row").unwrap().rect;
    assert_eq!(a.x, ctx().density.tokens().pad_x as f32 + 2.);
    assert_eq!(b.y - a.y, ctx().density.tokens().row_h as f32);
    let mut direct = t.seats[1].row.clone();
    direct.focused = true;
    let expected = layout(direct.element(ctx()), a.w);
    let actual = find(&l, "seat1/row").unwrap().rect;
    let expected = find(&expected, "row").unwrap().rect;
    assert_eq!([actual.w, actual.h], [expected.w, expected.h]);
}
