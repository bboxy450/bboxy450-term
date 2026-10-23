//! Native sidebar metadata and the optional frozen scene-15 smoke fixture.
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

/// Settings > Glow lane range (operator, 2026-10-22).
pub const LANE_MIN: f32 = TOKENS.glow.lane_min as f32;
pub const LANE_MAX: f32 = TOKENS.glow.lane_max as f32;
pub struct Scene {
    pub seats: Vec<SeatRow>,
    tree: Option<SplitNode>,
    identity: Option<bbt_proto::GuiIdentity>,
    metrics: Metrics,
    /// The glow lane (Settings > Glow lane, 2-8 px): the gutter between panes,
    /// the inset of the panes area and every pane's effect width. Runtime state.
    lane: f32,
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
        let width = bbt_render::chrome::text::measure(text, font(style), None)
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
        let metrics = bbt_render::chrome::text::measure("Hg", font(style), None)
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
            tree: Some(tree(&fixture["tree"])),
            identity: None,
            metrics: Metrics::default(),
            lane: TOKENS.glow.lane as f32,
        }
        .with_env_lane()
    }
    /// BBOXY_GLOW_LANE=2..8 for manual checks until Settings is hosted in the app.
    fn with_env_lane(mut self) -> Self {
        if let Some(px) = std::env::var("BBOXY_GLOW_LANE")
            .ok()
            .and_then(|v| v.parse().ok())
        {
            self.set_lane(px);
        }
        self
    }
    pub fn lane(&self) -> f32 {
        self.lane
    }
    /// Takes effect on the next layout; nothing else changes.
    pub fn set_lane(&mut self, px: f32) {
        self.lane = px.clamp(LANE_MIN, LANE_MAX);
    }
    pub fn live() -> Self {
        Self {
            seats: vec![],
            tree: None,
            identity: None,
            metrics: Metrics::default(),
            lane: TOKENS.glow.lane as f32,
        }
        .with_env_lane()
    }
    pub fn set_model(&mut self, model: &bboxy450_term::model::Model) {
        self.identity = model.identity.clone();
        self.seats = model
            .ui
            .seats
            .iter()
            .map(|(id, seat)| {
                let mut row = SeatRow::new(id.to_string(), &seat.name, &seat.harness.0);
                row.role = seat.role.as_ref().map(|role| role.name.clone());
                row.title = seat.title.clone();
                row.activity = seat.activity;
                row.grade = seat.grade;
                row.muted = seat.muted;
                row.where_ = match &seat.where_ {
                    bbt_ui_model::Where::Local => None,
                    bbt_ui_model::Where::Ssh { .. } => Some("ssh".into()),
                    bbt_ui_model::Where::Cloud { .. } => Some("cloud".into()),
                };
                row
            })
            .collect();
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
        let footer_height = if self.identity.is_some() { 24. } else { 0. };
        let sidebar_height = (size[1] - title - footer_height).max(1.);
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
            extra: if self.tree.is_none() {
                self.seats
                    .iter()
                    .map(|seat| seat.element_in_swarm(ctx))
                    .collect()
            } else {
                vec![]
            },
            shadows: vec![],
            tabs: self
                .tree
                .as_ref()
                .map(|tree| {
                    vec![TabRow {
                        name: "all fifteen".into(),
                        shortcut: Some("⌘1".into()),
                        tree: self.mini(tree),
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
                    }]
                })
                .unwrap_or_default(),
        };
        let sidebar = Sidebar {
            cards: vec![card],
            size: [sidebar_width, sidebar_height],
            scroll: 0.,
            phone: false,
            new_swarm: true,
        };
        let mut layouts = vec![
            TitleBar {
                crumb: if self.tree.is_some() {
                    "swarm / all fifteen"
                } else {
                    "local / seats"
                }
                .into(),
                ..Default::default()
            }
            .element(ctx)
            .layout(Rect::new(0., 0., size[0], title), ctx, &self.metrics)
            .map_err(|e| e.to_string())?,
            sidebar
                .element(ctx)
                .layout(
                    Rect::new(0., title, sidebar_width, sidebar_height),
                    ctx,
                    &self.metrics,
                )
                .map_err(|e| e.to_string())?,
        ];
        // Every pane owns a lane of L px outside its box: L at the panes area's
        // outer edges, and one shared lane as the gutter between panes.
        let l = self.lane();
        let bounds = Rect::new(
            bounds.x + l,
            bounds.y + l,
            (bounds.w - 2. * l).max(1.),
            (bounds.h - 2. * l).max(1.),
        );
        for (id, rect) in self
            .tree
            .as_ref()
            .map(|tree| solve_split(tree, bounds, l).panes)
            .unwrap_or_default()
        {
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
                lane: l,
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
        if let Some(identity) = &self.identity {
            use bbt_proto::GuiIdentityState;
            let note = match identity.state {
                GuiIdentityState::LocalDev => "unsigned, local only".into(),
                GuiIdentityState::Certified => "identity certified".into(),
                GuiIdentityState::Pending | GuiIdentityState::Held => format!(
                    "signing held: {}",
                    identity.reason.as_deref().unwrap_or("identity pending")
                ),
            };
            let mut footer = DrawList::default();
            footer.text(
                Rect::new(
                    12.,
                    size[1] - 20.,
                    (300_f32.min(size[0] * 0.35) - 24.).max(1.),
                    16.,
                ),
                note,
                TextStyle {
                    scale: TypeScale::Caption,
                    ..Default::default()
                },
                Color::hex("#8B92AA"),
            );
            ops.push(c::Paint::Draw(footer));
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

#[cfg(test)]
mod lane_tests {
    use super::*;

    /// Pane rects (layouts after the title bar and the sidebar).
    fn panes(scene: &Scene) -> Vec<Rect> {
        scene.layout([1600., 960.], true).unwrap()[2..]
            .iter()
            .map(|l| l.rect)
            .collect()
    }

    #[test]
    fn the_lane_sets_gutter_and_inset_and_relayouts_at_runtime() {
        let mut scene = Scene::new();
        for l in [LANE_MAX, LANE_MIN, LANE_MAX] {
            scene.set_lane(l);
            let rects = panes(&scene);
            let left = rects.iter().map(|r| r.x).fold(f32::MAX, f32::min);
            let right = rects.iter().map(|r| r.right()).fold(f32::MIN, f32::max);
            let top = rects.iter().map(|r| r.y).fold(f32::MAX, f32::min);
            let bottom = rects.iter().map(|r| r.bottom()).fold(f32::MIN, f32::max);
            // The panes area as Scene::layout computes it, before the lane inset.
            let sidebar = 300_f32.min(1600. * 0.35);
            let title = STYLE.spacing.title_bar as f32;
            assert!(
                (left - (sidebar + l)).abs() < 0.01,
                "L={l}: left inset {}",
                left - sidebar
            );
            assert!(
                (right - (1600. - l)).abs() < 0.01,
                "L={l}: right inset {}",
                1600. - right
            );
            assert!(
                (top - (title + l)).abs() < 0.01,
                "L={l}: top inset {}",
                top - title
            );
            assert!(
                (bottom - (960. - l)).abs() < 0.01,
                "L={l}: bottom inset {}",
                960. - bottom
            );
            // Neighbours are exactly one lane apart.
            let mut gaps = 0;
            for a in &rects {
                for b in &rects {
                    let horizontal =
                        (b.x - a.right() - l).abs() < 0.01 && a.y < b.bottom() && b.y < a.bottom();
                    let vertical =
                        (b.y - a.bottom() - l).abs() < 0.01 && a.x < b.right() && b.x < a.right();
                    gaps += (horizontal || vertical) as usize;
                }
            }
            assert!(gaps > 0, "L={l}: no neighbour one lane apart");
        }
        scene.set_lane(20.);
        assert_eq!(scene.lane(), LANE_MAX);
        scene.set_lane(0.5);
        assert_eq!(scene.lane(), LANE_MIN);
    }

    #[test]
    fn pane_rects_are_identical_across_activity_states() {
        let mut scene = Scene::new();
        for l in [LANE_MIN, LANE_MAX] {
            scene.set_lane(l);
            let before = panes(&scene);
            for sequence in 0..4 {
                scene.update(sequence);
                assert_eq!(
                    panes(&scene),
                    before,
                    "L={l}: an activity change moved a pane"
                );
            }
        }
    }
}
