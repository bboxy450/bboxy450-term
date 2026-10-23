//! Native sidebar metadata and the optional frozen scene-15 smoke fixture.
use bbt_ui::{
    self as ui, components as c,
    modules::{
        launcher::Launcher,
        mini_grid::Node,
        pane::Pane,
        perf_hud::PerfHud,
        seat_row::SeatRow,
        sidebar::Sidebar,
        strip::Strip,
        swarm_card::SwarmCard,
        tab_row::{TabRow, TabSeat},
        title_bar::TitleBar,
    },
};
use bbt_ui_model::{
    Activity, STYLE, SeatId, SplitNode, Stage, TOKENS, WaitKind,
    draw::{Color, DrawList, TextStyle, TypeScale},
    layout::{Axis, Rect, TextMeasure, TextStyle as MeasureStyle, solve_split},
};
use std::{cell::RefCell, collections::HashMap};

/// Settings > Glow lane range (operator, 2026-10-22).
pub const LANE_MIN: f32 = TOKENS.glow.lane_min as f32;
pub const LANE_MAX: f32 = TOKENS.glow.lane_max as f32;

/// The host retains the very layout it paints; pointer input never reconstructs
/// browser positions or guesses a row's height.
pub struct LauncherLayout {
    layout: c::Layout,
    pub panel: Rect,
    pub query: Rect,
    pub caret: Option<Rect>,
    query_origin: f32,
    query_font: MeasureStyle,
    pub rows: Vec<Rect>,
}
impl LauncherLayout {
    pub fn row_at(&self, at: [f32; 2]) -> Option<usize> {
        self.rows.iter().position(|rect| contains(*rect, at))
    }
    pub fn contains(&self, at: [f32; 2]) -> bool {
        contains(self.panel, at)
    }
    pub fn query_contains(&self, at: [f32; 2]) -> bool {
        contains(self.query, at)
    }
    pub fn candidate_area(&self) -> Rect {
        self.caret.unwrap_or(self.query)
    }
    fn paint(&self) -> Vec<c::Paint> {
        c::paint(
            &self.layout,
            ui::Context::new(ui::Theme::Dark, ui::Density::Cozy),
        )
    }
}
fn contains(r: Rect, at: [f32; 2]) -> bool {
    at[0] >= r.x && at[0] < r.right() && at[1] >= r.y && at[1] < r.bottom()
}
fn named<'a>(layout: &'a c::Layout, name: &str) -> Option<&'a c::Layout> {
    if matches!(&layout.kind, c::LayoutKind::Named(key) if key == name) {
        return Some(layout);
    }
    layout.children.iter().find_map(|child| named(child, name))
}

pub struct Scene {
    hosts: Vec<bbt_proto::GuiHost>,
    pub seats: Vec<SeatRow>,
    tree: Option<SplitNode>,
    seat_index: HashMap<SeatId, usize>,
    focused: Option<SeatId>,
    live: bool,
    backend_lost: bool,
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
            seat_index: (0..15).map(|i| (SeatId::from_u128(i as u128), i)).collect(),
            focused: Some(SeatId::from_u128(0)),
            live: false,
            backend_lost: false,
            identity: None,
            hosts: vec![],
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
            seat_index: HashMap::new(),
            focused: None,
            live: true,
            backend_lost: false,
            identity: None,
            hosts: vec![],
            metrics: Metrics::default(),
            lane: TOKENS.glow.lane as f32,
        }
        .with_env_lane()
    }
    pub fn set_model(&mut self, model: &bboxy450_term::model::Model, stage: &Stage) {
        self.identity = model.identity.clone();
        self.hosts = model.hosts.clone();
        let tab = model
            .ui
            .window
            .sidebar
            .swarms
            .iter()
            .flat_map(|swarm| &swarm.tabs)
            .find(|tab| Some(tab.id) == stage.tab);
        self.tree = tab.map(|tab| tab.tree.clone());
        // Stage is owned by the UI input adapter. Scene only projects it.
        self.focused = stage.focused;
        self.seat_index = model
            .ui
            .seats
            .keys()
            .enumerate()
            .map(|(i, id)| (*id, i))
            .collect();
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
                row.focused = Some(*id) == self.focused;
                if self.backend_lost {
                    row.activity = Activity::Errored;
                    row.title = "backend lost".into();
                }
                row.link = self
                    .hosts
                    .iter()
                    .find(|host| host.location == seat.where_)
                    .map(|host| bboxy450_term::hosts::link(&host.state));
                row.where_ = match &seat.where_ {
                    bbt_ui_model::Where::Local => None,
                    bbt_ui_model::Where::Ssh { .. } => Some("ssh".into()),
                    bbt_ui_model::Where::Cloud { .. } => Some("cloud".into()),
                };
                row
            })
            .collect();
    }
    pub fn focused(&self) -> Option<SeatId> {
        self.focused
    }
    pub fn backend_lost(&mut self) {
        self.backend_lost = true;
        for row in &mut self.seats {
            row.activity = Activity::Errored;
            row.title = "backend lost".into();
        }
    }
    pub fn backend_ready(&mut self) {
        self.backend_lost = false;
    }
    /// Read the actual pane body after lane-aware layout, including its padding.
    pub fn pane_bounds(&self, layouts: &[c::Layout]) -> Vec<(SeatId, Rect)> {
        fn body(layout: &c::Layout) -> Option<Rect> {
            if matches!(&layout.kind, c::LayoutKind::Named(name) if name == "pane/term") {
                let mut rect = layout.rect;
                if let Some(c::Layout {
                    kind: c::LayoutKind::Box { style, .. },
                    ..
                }) = layout.children.first()
                {
                    let [top, right, bottom, left] = style.padding;
                    rect = Rect::new(
                        rect.x + left,
                        rect.y + top,
                        (rect.w - left - right).max(0.),
                        (rect.h - top - bottom).max(0.),
                    );
                }
                return Some(rect);
            }
            layout.children.iter().find_map(body)
        }
        self.tree
            .as_ref()
            .map(SplitNode::leaves)
            .unwrap_or_default()
            .into_iter()
            .zip(layouts.iter().skip(2))
            .filter_map(|(id, layout)| body(layout).map(|rect| (id, rect)))
            .collect()
    }
    /// Spatial focus uses the whole pane, independently of terminal body sizes.
    pub fn focus_bounds(&self, layouts: &[c::Layout]) -> Vec<(SeatId, Rect)> {
        self.tree
            .as_ref()
            .map(SplitNode::leaves)
            .unwrap_or_default()
            .into_iter()
            .zip(layouts.iter().skip(2).map(|layout| layout.rect))
            .collect()
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
                let s = &self.seats[self.seat_index[id]];
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
                        name: if self.live { "seats" } else { "all fifteen" }.into(),
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
                        focused_key: self
                            .focused
                            .and_then(|id| self.seat_index.get(&id))
                            .map(|index| self.seats[*index].key.clone()),
                    }]
                })
                .unwrap_or_default(),
        };
        let mut cards = vec![card];
        cards.extend(bboxy450_term::hosts::sidebar_cards(&self.hosts));
        let sidebar = Sidebar {
            cards,
            size: [sidebar_width, sidebar_height],
            scroll: 0.,
            phone: false,
            new_swarm: true,
        };
        let mut layouts = vec![
            TitleBar {
                crumb: if !self.live {
                    "swarm / all fifteen"
                } else {
                    "swarm / seats"
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
            let seat = self.seats[self.seat_index[&id]].clone();
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
        overlay: Option<&PerfHud>,
        launcher: Option<&LauncherLayout>,
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
        ops.extend(self.foreground(size, launcher, overlay));
        ops
    }
    pub fn launcher_layout(
        &self,
        size: [f32; 2],
        launcher: &Launcher,
    ) -> Result<LauncherLayout, String> {
        let ctx = ui::Context::new(ui::Theme::Dark, ui::Density::Cozy);
        let width = 760_f32.min(size[0] * 0.92);
        let side = (size[0] - width) * 0.5;
        let mut overlay = c::Overlay::new(launcher.element(ctx));
        overlay.horizontal = c::Alignment::Stretch;
        overlay.vertical = c::Alignment::Start;
        overlay.style.overrides.height = Some(ui::Length::Px(size[1]));
        overlay.style.overrides.radii = Some([ui::Length::Px(0.); 4]);
        overlay.style.overrides.padding =
            Some([size[1] * 0.09, side, 0., side].map(ui::Length::Px));
        overlay.style.overrides.blur = Some(ui::Blur {
            radius: ui::Length::Px(STYLE.material.backdrop_blur_px as f32),
            saturation: 1.15,
        });
        let layout = c::Element::Overlay(overlay)
            .layout(Rect::new(0., 0., size[0], size[1]), ctx, &self.metrics)
            .map_err(|e| e.to_string())?;
        let panel = named(&layout, "picker")
            .ok_or("launcher panel missing")?
            .rect;
        let query = named(&layout, "picker/inrow/input")
            .ok_or("launcher query missing")?
            .rect;
        let caret = named(&layout, "picker/inrow/input/edit/caret").map(|n| n.rect);
        let prefix = named(&layout, "picker/inrow/input/edit/prefix")
            .or_else(|| named(&layout, "picker/inrow/input/text"));
        let query_origin = prefix.map_or(query.x, |n| n.rect.x);
        let query_font = MeasureStyle {
            size: if launcher.spotlight {
                STYLE.r#type.input
            } else {
                STYLE.r#type.body
            } as f32,
            weight: 400,
            mono: false,
            italic: false,
            tracking_em: 0.,
        };
        let rows = (0..launcher.view().rows.len())
            .map(|index| {
                let key = if index == 0 {
                    "picker/div/opt".into()
                } else {
                    format!("picker/div/opt{index}")
                };
                named(&layout, &key)
                    .map(|layout| layout.rect)
                    .ok_or("launcher row missing")
            })
            .collect::<Result<_, _>>()?;
        Ok(LauncherLayout {
            layout,
            panel,
            query,
            caret,
            query_origin,
            query_font,
            rows,
        })
    }
    /// Pointer hit testing uses this host's shaping metrics and UTF-16
    /// boundaries. It never derives an index from character count or pixels.
    pub fn query_index(&self, layout: &LauncherLayout, value: &str, x: f32) -> usize {
        let mut units = 0;
        let mut boundaries = vec![(0, 0)];
        for (byte, ch) in value.char_indices() {
            units += ch.len_utf16();
            boundaries.push((byte + ch.len_utf8(), units));
        }
        let advance = |i: usize| {
            self.metrics
                .width(&value[..boundaries[i].0], &layout.query_font)
        };
        let wanted = x - layout.query_origin;
        let (mut lo, mut hi) = (0, boundaries.len() - 1);
        while lo < hi {
            let mid = (lo + hi) / 2;
            if advance(mid) < wanted {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        let index = if lo > 0 && wanted - advance(lo - 1) < advance(lo) - wanted {
            lo - 1
        } else {
            lo
        };
        boundaries[index].1
    }
    /// Cells precede these ops in the live path. Leave the frosted launcher
    /// unwrapped so its blur samples the painted window; the meter stays last.
    pub fn foreground(
        &self,
        size: [f32; 2],
        launcher: Option<&LauncherLayout>,
        hud: Option<&PerfHud>,
    ) -> Vec<c::Paint> {
        let mut ops = launcher.map_or_else(Vec::new, LauncherLayout::paint);
        ops.extend(self.overlay(size, hud));
        ops
    }
    /// The perf HUD (Cmd+Opt+F), top right as the lab's perf.html floats it over the app.
    pub fn overlay(&self, size: [f32; 2], hud: Option<&PerfHud>) -> Vec<c::Paint> {
        let Some(hud) = hud else { return vec![] };
        let ctx = ui::Context::new(ui::Theme::Dark, ui::Density::Cozy);
        let width = PerfHud::width(ctx);
        let rect = Rect::new((size[0] - 14. - width).max(0.), 40., width, 0.);
        match hud.element(ctx, "hud").layout(rect, ctx, &self.metrics) {
            Ok(layout) => c::paint(&layout, ctx),
            Err(_) => vec![],
        }
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

#[cfg(test)]
mod launcher_tests {
    use super::*;

    #[test]
    fn ime_candidate_area_follows_the_retained_caret_and_pointer_uses_utf16() {
        use bboxy450_term::launcher::Command;
        let scene = Scene::new();
        let mut controller = crate::new_launcher(None, 0);
        controller.handle(Command::Query("A😀B".into()));
        controller.handle(Command::SetCaret {
            at: 3,
            extend: false,
        });
        let layout = scene
            .launcher_layout([1600., 960.], &controller.launcher)
            .unwrap();
        assert_eq!(layout.candidate_area(), layout.caret.unwrap());
        assert_eq!(layout.candidate_area().w, 2.);
        assert!(layout.candidate_area().w < layout.query.w);
        let font = &layout.query_font;
        for (value, units) in [("", 0), ("A", 1), ("A😀", 3), ("A😀B", 4)] {
            let x = layout.query_origin + scene.metrics.width(value, font);
            assert_eq!(scene.query_index(&layout, "A😀B", x), units);
        }
    }

    #[test]
    fn retained_launcher_hit_regions_and_unwrapped_foreground_order() {
        let scene = Scene::new();
        let size = [1600., 960.];
        let layouts = scene.layout(size, false).unwrap();
        let before = scene.pane_bounds(&layouts);
        let outer = scene.focus_bounds(&layouts);
        assert_eq!(outer.len(), before.len());
        for ((id, frame), (body_id, body)) in outer.iter().zip(&before) {
            assert_eq!(id, body_id);
            assert!(body.x >= frame.x && body.y > frame.y);
            assert!(body.right() <= frame.right() && body.bottom() <= frame.bottom());
        }
        let controller = crate::new_launcher(None, 0);
        let launcher = scene.launcher_layout(size, &controller.launcher).unwrap();
        assert_eq!(launcher.panel.w, 760.);
        assert!(launcher.query.w > 0.);
        assert!(launcher.contains([launcher.query.x + 1., launcher.query.y + 1.]));
        assert!(!launcher.contains([0., 0.]));
        assert_eq!(launcher.rows.len(), controller.launcher.view().rows.len());
        for (index, rect) in launcher.rows.iter().enumerate() {
            assert_eq!(
                launcher.row_at([rect.x + rect.w * 0.5, rect.y + rect.h * 0.5]),
                Some(index)
            );
        }
        let hud = crate::perf_hud(
            bbt_rt::HitchMeter::new(120.).perf_record(std::time::Instant::now(), 0.),
        );
        let foreground = scene.foreground(size, Some(&launcher), Some(&hud));
        let launcher_ops = launcher.paint();
        let hud_ops = scene.overlay(size, Some(&hud));
        assert_eq!(foreground.len(), launcher_ops.len() + hud_ops.len());
        assert_eq!(
            format!("{:?}", &foreground[..launcher_ops.len()]),
            format!("{launcher_ops:?}")
        );
        assert_eq!(
            format!("{:?}", &foreground[launcher_ops.len()..]),
            format!("{hud_ops:?}")
        );
        assert!(launcher_ops.iter().any(|op| matches!(
            op,
            c::Paint::Material {
                kind: ui::Material::Frosted,
                ..
            }
        )));
        assert!(
            launcher_ops
                .iter()
                .all(|op| !matches!(op, c::Paint::Group { .. }))
        );
        assert!(scene.foreground(size, None, None).is_empty());
        assert_eq!(
            before,
            scene.pane_bounds(&scene.layout(size, false).unwrap())
        );
        let base_ops = scene.paint(&layouts, size, None, None);
        let frozen = scene.paint(&layouts, size, Some(&hud), Some(&launcher));
        assert_eq!(
            format!("{:?}", &frozen[..base_ops.len()]),
            format!("{base_ops:?}")
        );
        assert_eq!(
            format!("{:?}", &frozen[base_ops.len()..]),
            format!("{foreground:?}")
        );
    }

    /// Captured also on the unchanged accepted source. Compare the complete
    /// bytes between these markers, not a screenshot of a moving live scene.
    #[test]
    fn no_launcher_paint_proof() {
        let scene = Scene::new();
        let size = [1600., 960.];
        let layouts = scene.layout(size, false).unwrap();
        let paints = scene.paint(&layouts, size, None, None);
        println!("L8_PAINT_BEGIN{paints:#?}L8_PAINT_END");
        assert_eq!(scene.focus_bounds(&layouts).len(), 15);
    }
}
