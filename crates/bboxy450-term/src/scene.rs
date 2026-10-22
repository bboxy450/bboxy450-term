//! The frozen scene-15 tree and native UI modules, from embedded memory only.
use bbt_ui::{
    self as ui, components as c,
    modules::{
        mini_grid::Node,
        pane::Pane,
        seat_row::SeatRow,
        sidebar::Sidebar,
        strip::Strip,
        swarm_card::SwarmCard,
        tab_row::{TabRow, TabSeat},
        title_bar::TitleBar,
    },
};
use bbt_ui_model::{
    Activity, STYLE, SeatId, SplitNode, TOKENS, WaitKind,
    draw::{Color, DrawList, TextStyle, TypeScale},
    layout::{Axis, Rect, TextMeasure, TextStyle as MeasureStyle, solve_split},
};
use std::{cell::RefCell, collections::HashMap};

pub struct Scene {
    pub seats: Vec<SeatRow>,
    tree: SplitNode,
    metrics: Metrics,
}
type FontKey = (u32, u16, bool, bool, u32);
#[derive(Default)]
pub struct Metrics {
    widths: RefCell<HashMap<(String, FontKey), f32>>,
    fonts: RefCell<HashMap<FontKey, [f32; 2]>>,
}
fn font_key(s: &MeasureStyle) -> FontKey {
    (
        s.size.to_bits(),
        s.weight,
        s.mono,
        s.italic,
        s.tracking_em.to_bits(),
    )
}
fn font(s: &MeasureStyle) -> TextStyle {
    TextStyle {
        scale: TypeScale::Custom(s.size),
        weight: s.weight,
        mono: s.mono,
        italic: s.italic,
        tracking_em: s.tracking_em,
        ..Default::default()
    }
}
impl TextMeasure for Metrics {
    fn width(&self, text: &str, style: &MeasureStyle) -> f32 {
        let key = (text.to_owned(), font_key(style));
        if let Some(width) = self.widths.borrow().get(&key) {
            return *width;
        }
        let width = bbt_render::chrome::text::rasterize(text, font(style), None, 1.)
            .expect("native font")
            .advance;
        self.widths.borrow_mut().insert(key, width);
        width
    }
}
impl c::Measure for Metrics {
    fn font_metrics(&self, style: &MeasureStyle) -> [f32; 2] {
        let key = font_key(style);
        if let Some(metrics) = self.fonts.borrow().get(&key) {
            return *metrics;
        }
        let metrics = bbt_render::chrome::text::rasterize("Hg", font(style), None, 1.)
            .expect("native font metrics");
        let result = [metrics.ascent, metrics.descent];
        self.fonts.borrow_mut().insert(key, result);
        result
    }
}
fn tree(v: &serde_json::Value) -> SplitNode {
    if let Some(seat) = v["seat"].as_u64() {
        return SplitNode::Leaf(SeatId::from_u128(seat as u128));
    }
    SplitNode::split(
        if v["axis"] == "V" {
            Axis::Row
        } else {
            Axis::Column
        },
        v["ratio"].as_str().unwrap().parse().unwrap(),
        tree(&v["a"]),
        tree(&v["b"]),
    )
}
impl Scene {
    pub fn new() -> Self {
        // lab/web/showcase.js BASE, with its frozen 15-seat split fixture.
        let data = [
            ("Vesper Greywind", "claude", "oracle", "swarm control", 0),
            ("Nyx Copperwing", "codex", "executor", "messaging spec", 0),
            ("Lumen Foxglove", "claude", "creativity", "ux lab", 1),
            ("Rook Ashwhisker", "codex", "executor", "gpu research", 2),
            ("Sable Quill", "codex", "executor", "harness hooks", 1),
            ("Pip Hollow", "pi", "executor", "relay bench", 3),
            ("Marrow Finch", "omp", "executor", "parse logs", 1),
            ("zsh", "zsh", "", "~/mydev", 1),
            ("Tamsin Voss", "codex", "executor", "cargo test", 0),
            ("Corvin Ember", "codex", "executor", "review PR 14", 1),
            ("Ilka Thistle", "pi", "executor", "relay bench", 0),
            ("Dax Riven", "codex", "executor", "mux daemon", 1),
            ("Wren Starling", "omp", "executor", "tokens", 2),
            ("Halcyon Moss", "codex", "executor", "ssh rig", 1),
            ("Juniper Vale", "claude", "executor", "parse logs", 0),
        ];
        let palette = [
            "#FFC53D", "#FF4FA3", "#B07BFF", "#2EE6A8", "#2E6BFF", "#FF7A45", "#58D7FF", "#9AA3C7",
            "#F2D16B", "#5BE08C", "#E96BFF", "#FF6B6B", "#4FD1FF", "#C9A96B", "#8AF0D0",
        ];
        let seats = data
            .into_iter()
            .enumerate()
            .map(|(i, (name, harness, role, title, state))| {
                let mut row = SeatRow::new(format!("sc{i}"), name, harness);
                row.role = (!role.is_empty()).then(|| role.into());
                row.title = title.into();
                row.activity = match state {
                    0 => Activity::Active,
                    2 => Activity::Waiting(WaitKind::Question),
                    3 => Activity::Errored,
                    _ => Activity::Idle,
                };
                row.color = Some(Color::hex(palette[i]).0);
                row.focused = i == 0;
                row.where_ = match i {
                    3 => Some("cloud".into()),
                    4 | 13 => Some("ssh".into()),
                    _ => None,
                };
                row
            })
            .collect();
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../bbt-ui-model/tests/fixtures/scene15.json"
        ))
        .expect("embedded scene-15 fixture");
        Self {
            seats,
            tree: tree(&fixture["tree"]),
            metrics: Metrics::default(),
        }
    }
    pub fn update(&mut self, sequence: usize) {
        self.seats[1].title = format!("runtime update {sequence}");
        self.seats[1].activity = if sequence.is_multiple_of(2) {
            Activity::Active
        } else {
            Activity::Idle
        };
    }
    fn mini(&self, node: &SplitNode) -> Node {
        match node {
            SplitNode::Leaf(id) => {
                let s = &self.seats[id.0.0 as usize];
                Node::Cell {
                    key: s.key.clone(),
                    name: s.name.clone(),
                    activity: s.activity,
                }
            }
            SplitNode::Split { dir, ratio, a, b } => Node::Split {
                axis: *dir,
                ratio: *ratio,
                a: Box::new(self.mini(a)),
                b: Box::new(self.mini(b)),
            },
        }
    }
    pub fn layout(&self, size: [f32; 2], effects: bool) -> Result<Vec<c::Layout>, String> {
        let ctx = ui::Context::new(ui::Theme::Dark, ui::Density::Cozy);
        let title = STYLE.spacing.title_bar as f32;
        let sidebar_width = 300_f32.min(size[0] * 0.35);
        let bounds = Rect::new(
            sidebar_width,
            title,
            (size[0] - sidebar_width).max(1.),
            (size[1] - title).max(1.),
        );
        let card = SwarmCard {
            key: "swarm".into(),
            name: "swarm".into(),
            color: Color::hex("#2E6BFF").0,
            seats: self.seats.clone(),
            expanded: true,
            closed: false,
            unfocused: false,
            body: true,
            new_seat: true,
            extra: vec![],
            shadows: vec![],
            tabs: vec![TabRow {
                name: "all fifteen".into(),
                shortcut: Some("⌘1".into()),
                tree: self.mini(&self.tree),
                seats: self
                    .seats
                    .iter()
                    .map(|row| TabSeat {
                        row: row.clone(),
                        children: vec![],
                    })
                    .collect(),
                expanded: true,
                focused_key: Some("sc0".into()),
            }],
        };
        let sidebar = Sidebar {
            cards: vec![card],
            size: [sidebar_width, size[1] - title],
            scroll: 0.,
            phone: false,
            new_swarm: true,
        };
        let mut layouts = vec![
            TitleBar {
                crumb: "swarm / all fifteen".into(),
                ..Default::default()
            }
            .element(ctx)
            .layout(Rect::new(0., 0., size[0], title), ctx, &self.metrics)
            .map_err(|e| e.to_string())?,
            sidebar
                .element(ctx)
                .layout(
                    Rect::new(0., title, sidebar_width, size[1] - title),
                    ctx,
                    &self.metrics,
                )
                .map_err(|e| e.to_string())?,
        ];
        for (id, rect) in solve_split(&self.tree, bounds, TOKENS.layout.gutter as f32).panes {
            let seat = self.seats[id.0.0 as usize].clone();
            let pane_ctx = ui::Context {
                seat: seat.color.unwrap_or(ctx.seat),
                focused: seat.focused,
                ..ctx
            };
            let pane = Pane {
                strip: Strip::new(seat),
                lines: vec![],
                gap_from: None,
                resumed: None,
                min_height: rect.h,
                unfocused: false,
                reconnecting: false,
                reconnected: false,
                glow_seed: effects.then_some(id.0.0 as u32),
                opacity: 1.,
            };
            layouts.push(
                pane.element(pane_ctx)
                    .layout(rect, pane_ctx, &self.metrics)
                    .map_err(|e| e.to_string())?,
            );
        }
        Ok(layouts)
    }
    pub fn paint(
        &self,
        layouts: &[c::Layout],
        size: [f32; 2],
        overlay: Option<&str>,
    ) -> Vec<c::Paint> {
        let ctx = ui::Context::new(ui::Theme::Dark, ui::Density::Cozy);
        let mut background = DrawList::default();
        background.rect(
            Rect::new(0., 0., size[0], size[1]),
            0.,
            Color::hex(ctx.theme.palette().bg),
        );
        let mut ops = vec![c::Paint::Draw(background)];
        for layout in layouts {
            ops.extend(c::paint(layout, ctx));
        }
        if let Some(text) = overlay {
            let mut list = DrawList::default();
            let rect = Rect::new((size[0] - 460.).max(0.), size[1] - 95., 450., 85.);
            list.rect(rect, 8., Color::hex("#070D19").alpha(0.95));
            for (i, line) in text.lines().enumerate() {
                list.text(
                    Rect::new(rect.x + 12., rect.y + 8. + i as f32 * 20., 426., 20.),
                    line,
                    TextStyle {
                        mono: true,
                        scale: TypeScale::Custom(12.),
                        ..Default::default()
                    },
                    Color::hex("#D4E7FF"),
                );
            }
            ops.push(c::Paint::Draw(list));
        }
        ops
    }
}
