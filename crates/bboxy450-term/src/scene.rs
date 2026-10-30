// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Native sidebar metadata and the optional frozen scene-15 smoke fixture.
mod navigation;
mod starting;
use bboxy450_term::model::FocusSwell;
use bboxy450_term::starting::Slot;
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
use bbt_ui_model::event::LocateWhere;
use bbt_ui_model::{
    Activity, STYLE, SeatId, SplitNode, Stage, TOKENS, WaitKind,
    draw::{Color, DrawList, TextStyle, TypeScale},
    layout::{Axis, Rect, TextMeasure, TextStyle as MeasureStyle, stage_split},
};
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet, HashMap},
    time::Instant,
};

fn identity_note(identity: &bbt_proto::GuiIdentity) -> String {
    use bbt_proto::GuiIdentityState;
    match identity.state {
        GuiIdentityState::LocalDev => "unsigned, local only".into(),
        GuiIdentityState::Certified => identity
            .reason
            .as_deref()
            .filter(|reason| reason.starts_with("test-account:"))
            .unwrap_or("identity certified")
            .into(),
        GuiIdentityState::Pending | GuiIdentityState::Held => format!(
            "signing held: {}",
            identity.reason.as_deref().unwrap_or("identity pending")
        ),
    }
}

// lab.css restarted-out: hold to 85%, then the shared motion.ease until four seconds.
fn restarted_alpha(seconds: f32) -> f32 {
    if seconds <= 3.4 {
        return 1.;
    }
    if seconds >= 4. {
        return 0.;
    }
    let curve = STYLE
        .motion
        .ease
        .strip_prefix("cubic-bezier(")
        .and_then(|value| value.strip_suffix(')'))
        .expect("motion.ease is a cubic-bezier token");
    let mut coordinates = curve
        .split(',')
        .map(|value| value.trim().parse::<f32>().expect("motion.ease coordinate"));
    let [x1, y1, x2, y2] = std::array::from_fn(|_| {
        coordinates
            .next()
            .expect("motion.ease has four coordinates")
    });
    let progress = (seconds - 3.4) / 0.6;
    let (mut lo, mut hi) = (0., 1.);
    for _ in 0..16 {
        let t = (lo + hi) / 2.;
        let x = 3. * x1 * t * (1. - t) * (1. - t) + 3. * x2 * t * t * (1. - t) + t * t * t;
        if x < progress {
            lo = t;
        } else {
            hi = t;
        }
    }
    let t = (lo + hi) / 2.;
    1. - (3. * y1 * t * (1. - t) * (1. - t) + 3. * y2 * t * t * (1. - t) + t * t * t)
}

/// Settings > Rim size range (rim-glow §3).
pub const RIM_MIN: f32 = TOKENS.glow.rim_min as f32;
pub const RIM_MAX: f32 = TOKENS.glow.rim_max as f32;

/// The host retains the very layout it paints; pointer input never reconstructs
/// browser positions or guesses a row's height.
pub struct ConversationLayout {
    pub layout: c::Layout,
    pub regions: ui::modules::composer::Regions,
    pub height: f32,
    pub splitters: [Rect; 3],
    pub tasks_height: f32,
    pub available_height: f32,
    pub actions: ui::modules::stream_dock::Regions,
    context: ui::Context,
}
pub struct DocumentLayout {
    pub layout: c::Layout,
    pub regions: ui::modules::document_modal::controller::Regions,
    pub source: Option<bbt_proto::gui::UiDocumentSource>,
    context: ui::Context,
    viewport_size: [f32; 2],
}
impl DocumentLayout {
    pub fn matches(&self, size: [f32; 2], ctx: ui::Context) -> bool {
        self.viewport_size == size
            && self.context.theme == ctx.theme
            && self.context.density == ctx.density
            && self.context.seat == ctx.seat
            && self.context.swarm == ctx.swarm
            && self.context.focused == ctx.focused
            && self.context.selected == ctx.selected
    }
    pub fn paint(&self) -> Vec<c::Paint> {
        c::paint(&self.layout, self.context)
    }
}
pub struct LocalPageLayout {
    pub layout: c::Layout,
    pub viewport: Rect,
    pub close: Rect,
    pub max_scroll: f32,
    pub settings: Option<ui::modules::settings::controller::Regions>,
    focus: Option<(Rect, Rect)>,
    context: ui::Context,
}
impl LocalPageLayout {
    pub fn paint(&self) -> Vec<c::Paint> {
        let mut ops = c::paint(&self.layout, self.context);
        if let Some((rect, clip)) = self.focus {
            let mut draw = DrawList::default();
            draw.clip(clip, [0.; 4]);
            draw.push(bbt_ui_model::draw::Prim::Rect {
                rect,
                radii: [5.; 4],
                fill: Color::TRANSPARENT,
                stroke: Some((2., Color::hex(self.context.theme.palette().accent))),
                inner_highlight: None,
                shadow: None,
            });
            draw.unclip();
            ops.push(c::Paint::Draw(draw));
        }
        ops
    }
    pub fn contains(&self, at: [f32; 2]) -> bool {
        contains(self.viewport, at)
    }
    pub fn closes(&self, at: [f32; 2]) -> bool {
        contains(self.close, at)
    }
    pub fn focuses(&self, at: [f32; 2]) -> bool {
        contains(self.layout.rect, at)
    }
}
impl ConversationLayout {
    pub fn paint(&self) -> Vec<c::Paint> {
        c::paint(&self.layout, self.context)
    }
}
pub struct LauncherLayout {
    layout: c::Layout,
    #[cfg(test)]
    pub panel: Rect,
    pub viewport: Rect,
    pub max_scroll: f32,
    pub query: Rect,
    pub caret: Option<Rect>,
    query_origin: f32,
    query_font: MeasureStyle,
    context: ui::Context,
    pub rows: Vec<Rect>,
    pub selected: usize,
    row_available: Vec<bool>,
}
impl LauncherLayout {
    /// Availability belongs to these painted rows, not a new filtered projection.
    /// Origin: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
    pub fn row_available(&self, index: usize) -> bool {
        self.row_available.get(index).copied().unwrap_or(false)
    }
    pub fn rect(&self, name: &str) -> Option<Rect> {
        named(&self.layout, name).map(|l| l.rect)
    }
    pub fn solved(&self) -> &c::Layout {
        &self.layout
    }
    pub fn row_at(&self, at: [f32; 2]) -> Option<usize> {
        if !contains(self.viewport, at) {
            return None;
        }
        self.rows.iter().position(|rect| contains(*rect, at))
    }
    pub fn contains(&self, at: [f32; 2]) -> bool {
        contains(self.viewport, at)
    }
    pub fn query_contains(&self, at: [f32; 2]) -> bool {
        contains(self.viewport, at) && contains(self.query, at)
    }
    pub fn candidate_area(&self) -> Rect {
        self.caret.unwrap_or(self.query)
    }
    fn paint(&self) -> Vec<c::Paint> {
        c::paint(&self.layout, self.context)
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
fn named_suffix<'a>(layout: &'a c::Layout, name: &str) -> Option<&'a c::Layout> {
    if matches!(&layout.kind, c::LayoutKind::Named(key) if key.ends_with(name)) {
        return Some(layout);
    }
    layout
        .children
        .iter()
        .find_map(|child| named_suffix(child, name))
}

pub struct Scene {
    pub sidebar_scroll: f32,
    pub sidebar_width: f32,
    pub sidebar_layout: ui::modules::composer::SidebarLayout,
    pub open_swarms: BTreeSet<bbt_ui_model::SwarmId>,
    pub sidebar_sections_closed: BTreeSet<String>,
    sidebar_reveals: BTreeMap<String, ui::modules::accordion::Reveal>,
    sidebar_motion_now: Instant,
    selected_swarm: Option<bbt_ui_model::SwarmId>,
    #[cfg(test)]
    projections: usize,
    hosts: Vec<bbt_proto::GuiHost>,
    closed: Vec<(SeatId, Instant, String, String)>,
    layout_revision: u64,
    swarms: Vec<bbt_ui_model::Swarm>,
    pub seats: Vec<SeatRow>,
    tasks: Vec<(SeatId, String, ui::modules::seat_row::TaskRow, Option<i32>)>,
    tree: Option<SplitNode>,
    starting: Option<starting::StartingScene>,
    seat_index: HashMap<SeatId, usize>,
    terminal_backgrounds: HashMap<SeatId, [u8; 4]>,
    focused: Option<SeatId>,
    live: bool,
    backend_lost: bool,
    identity: Option<bbt_proto::GuiIdentity>,
    metrics: Metrics,
    /// The rim size R (Settings > Rim size, 2-8 px), centred on each pane's border: the
    /// gutter between panes is R and the stage inset R/2, so changing it relayouts.
    rim: f32,
    context: ui::Context,
    glow: bool,
    subtitle: bool,
    /// When each seat's focus swell started (rim-glow §7); it runs glow.swell_ms.
    swells: HashMap<SeatId, FocusSwell>,
    /// Seats waiting for the operator (attention §2): since when, and whether muted.
    attention: HashMap<SeatId, (Instant, bool)>,
    attention_origins: HashMap<SeatId, Instant>,
    restart_generations: HashMap<SeatId, (u64, Option<Instant>, bbt_ui_model::SwarmId)>,
    restart_lines: Vec<(bbt_ui_model::SwarmId, String)>,
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
fn pane_body(layout: &c::Layout) -> Option<Rect> {
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
    layout.children.iter().find_map(pane_body)
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
        let fixture: serde_json::Value = serde_json::from_str(include_str!(concat!(
            env!("BBOXY_INPUT_ROOT"),
            "/crates/bbt-ui-model/tests/fixtures/scene15.json"
        )))
        .expect("embedded scene-15 fixture");
        Self {
            sidebar_scroll: 0.,
            sidebar_width: TOKENS.sidebar.width as f32,
            sidebar_layout: Default::default(),
            open_swarms: BTreeSet::new(),
            sidebar_sections_closed: BTreeSet::new(),
            sidebar_reveals: BTreeMap::new(),
            sidebar_motion_now: Instant::now(),
            selected_swarm: None,
            #[cfg(test)]
            projections: 0,
            seats,
            tasks: vec![],
            closed: vec![],
            layout_revision: 0,
            swarms: vec![],
            tree: Some(tree(&fixture["tree"])),
            starting: None,
            seat_index: (0..15).map(|i| (SeatId::from_u128(i as u128), i)).collect(),
            terminal_backgrounds: HashMap::new(),
            focused: Some(SeatId::from_u128(0)),
            live: false,
            backend_lost: false,
            identity: None,
            hosts: vec![],
            metrics: Metrics::default(),
            rim: TOKENS.glow.rim as f32,
            context: ui::Context::new(ui::Theme::Dark, ui::Density::Cozy),
            glow: true,
            subtitle: false,
            swells: HashMap::new(),
            restart_generations: HashMap::new(),
            restart_lines: vec![],
            attention: HashMap::new(),
            attention_origins: HashMap::new(),
        }
        .with_env_rim()
    }
    /// BBOXY_RIM_SIZE=2..8 for manual checks until Settings is hosted in the app.
    fn with_env_rim(mut self) -> Self {
        if let Some(px) = std::env::var("BBOXY_RIM_SIZE")
            .ok()
            .and_then(|v| v.parse().ok())
        {
            self.set_rim(px);
        }
        self
    }
    /// Takes effect at the next layout: the gutter and the stage inset follow it.
    pub fn ui_context(&self) -> ui::Context {
        self.context
    }
    pub fn set_context(&mut self, context: ui::Context) {
        self.context = context;
    }
    pub fn set_appearance(
        &mut self,
        preferences: &bboxy450_term::settings::Preferences,
        system: ui::Theme,
    ) {
        self.set_context(preferences.context(system));
        self.set_rim(preferences.appearance.rim);
        self.glow = preferences.appearance.glow;
        self.subtitle = preferences.appearance.subtitle;
    }
    pub fn set_rim(&mut self, px: f32) {
        self.rim = px.clamp(RIM_MIN, RIM_MAX);
    }
    /// One directional focus cue replaces the previous cue, without touching attention.
    pub fn swell(&mut self, seat: SeatId, pulse: FocusSwell) {
        self.swells.clear();
        self.swells.insert(seat, pulse);
    }
    /// A seat waiting for the operator loops the swell in its waiting amber until handled
    /// (attention §2); muted, it keeps the ordinary waiting rim. None: handled.
    #[cfg(test)]
    pub fn set_attention(&mut self, seat: SeatId, state: Option<(Instant, bool)>) {
        if let Some((since, _)) = state {
            self.attention_origins.insert(seat, since);
        } else {
            self.attention_origins.remove(&seat);
        }
        match state {
            Some(state) => self.attention.insert(seat, state),
            None => self.attention.remove(&seat),
        };
    }
    /// Whether any swell or attention loop still needs frames at `now`.
    pub fn animating(&self, now: Instant) -> bool {
        if self.sidebar_reveals.values().any(|r| r.active(now)) {
            return true;
        }
        if self.restart_generations.values().any(|(_, at, _)| {
            at.is_some_and(|at| now.saturating_duration_since(at).as_secs_f32() < 4.)
        }) {
            return true;
        }
        if self.closed.iter().any(|(_, until, _, _)| now < *until) {
            return true;
        }
        if !self.glow {
            return false;
        }
        let ms = TOKENS.glow.swell_ms as f32;
        self.attention.values().any(|(_, muted)| !muted)
            || self
                .swells
                .values()
                .any(|pulse| (now.saturating_duration_since(pulse.at).as_secs_f32() * 1000.) < ms)
    }
    /// The seat's swell envelope at `now` and whether it is the attention loop: |sin| for
    /// attention, one half sine over glow.swell_ms for a focus swell, else 0.
    fn envelope(&self, seat: SeatId, now: Instant, surface: LocateWhere) -> (f32, bool) {
        let ms = TOKENS.glow.swell_ms as f32;
        let t = |at: &Instant| now.saturating_duration_since(*at).as_secs_f32() * 1000.;
        if let Some((since, false)) = self.attention.get(&seat) {
            return ((std::f32::consts::PI * t(since) / ms).sin().abs(), true);
        }
        match self
            .swells
            .get(&seat)
            .filter(|pulse| pulse.where_ == surface || pulse.where_ == LocateWhere::Both)
            .map(|pulse| t(&pulse.at))
        {
            Some(t) if t < ms => ((std::f32::consts::PI * t / ms).sin(), false),
            _ => (0., false),
        }
    }
    pub fn live() -> Self {
        Self {
            sidebar_scroll: 0.,
            sidebar_width: TOKENS.sidebar.width as f32,
            sidebar_layout: Default::default(),
            open_swarms: BTreeSet::new(),
            sidebar_sections_closed: BTreeSet::new(),
            sidebar_reveals: BTreeMap::new(),
            sidebar_motion_now: Instant::now(),
            selected_swarm: None,
            #[cfg(test)]
            projections: 0,
            seats: vec![],
            tasks: vec![],
            closed: vec![],
            layout_revision: 0,
            swarms: vec![],
            tree: None,
            starting: None,
            seat_index: HashMap::new(),
            terminal_backgrounds: HashMap::new(),
            focused: None,
            live: true,
            backend_lost: false,
            identity: None,
            hosts: vec![],
            metrics: Metrics::default(),
            rim: TOKENS.glow.rim as f32,
            context: ui::Context::new(ui::Theme::Dark, ui::Density::Cozy),
            glow: true,
            subtitle: false,
            swells: HashMap::new(),
            restart_generations: HashMap::new(),
            restart_lines: vec![],
            attention: HashMap::new(),
            attention_origins: HashMap::new(),
        }
        .with_env_rim()
    }
    /// Uncontrolled fixture projection; the App uses `set_model_with_clock`.
    pub fn set_model(&mut self, model: &bboxy450_term::model::Model, stage: &Stage) {
        self.set_model_at(model, stage, None);
    }
    pub fn set_model_with_clock(
        &mut self,
        model: &bboxy450_term::model::Model,
        stage: &Stage,
        clock: &crate::animation_clock::Clock,
    ) {
        self.set_model_at(model, stage, clock.controlled().then(|| clock.now()));
    }
    fn set_model_at(
        &mut self,
        model: &bboxy450_term::model::Model,
        stage: &Stage,
        animation_now: Option<Instant>,
    ) {
        #[cfg(test)]
        {
            self.projections += 1;
        }
        let measurement = bbt_rt::diagnostics::span(bbt_rt::diagnostics::Phase::UiProjection);
        bbt_rt::diagnostics::probe_after(
            bbt_rt::diagnostics::ProbeStage::LatestTaken,
            bbt_rt::diagnostics::ProbeStage::ProjectionStarted,
        );
        self.starting = None;
        self.identity = model.identity.clone();
        self.hosts = model.hosts.clone();
        self.swarms = model.ui.window.sidebar.swarms.clone();
        self.open_swarms = model.ui.window.sidebar.open.clone();
        self.sidebar_sections_closed = model
            .window_restore
            .as_ref()
            .map(|saved| saved.sidebar_sections_closed.iter().cloned().collect())
            .unwrap_or_default();
        self.sidebar_width = model.ui.window.sidebar.width.clamp(
            bbt_ui_model::input::MIN_WIDTH,
            bbt_ui_model::input::MAX_WIDTH,
        );
        self.selected_swarm = stage.swarm;
        self.restart_generations.retain(|seat, _| {
            model
                .ui
                .seats
                .get(seat)
                .is_some_and(|seat| seat.lifecycle == bbt_ui_model::Lifecycle::Open)
        });
        for (id, seat) in &model.ui.seats {
            if seat.lifecycle != bbt_ui_model::Lifecycle::Open {
                continue;
            }
            let current = self.restart_generations.entry(*id).or_insert((
                seat.process_generation,
                None,
                seat.swarm,
            ));
            if current.0 != seat.process_generation {
                current.1 = (seat.process_generation > current.0)
                    .then(|| animation_now.unwrap_or_else(Instant::now));
                current.0 = seat.process_generation;
            }
            current.2 = seat.swarm;
        }
        self.attention
            .retain(|seat, _| model.attention.contains_key(seat));
        self.attention_origins
            .retain(|seat, _| model.attention.contains_key(seat));
        for (seat, state) in &model.attention {
            if self.attention_origins.get(seat) != Some(&state.since) {
                self.attention_origins.insert(*seat, state.since);
                self.attention
                    .insert(*seat, (animation_now.unwrap_or(state.since), state.muted));
            } else if let Some((_, muted)) = self.attention.get_mut(seat) {
                *muted = state.muted;
            }
        }
        self.swells
            .retain(|seat, _| model.ui.seats.contains_key(seat));
        if let Some((seat, pulse)) = model.focus_swell
            && model.ui.seats.contains_key(&seat)
        {
            self.swell(seat, pulse);
        }
        let tab = model
            .ui
            .window
            .sidebar
            .swarms
            .iter()
            .flat_map(|swarm| &swarm.tabs)
            .find(|tab| Some(tab.id) == stage.tab);
        let tree = tab.map(|tab| tab.tree.clone());
        if self.tree != tree {
            self.layout_revision = self.layout_revision.saturating_add(1);
        }
        self.tree = tree;
        self.terminal_backgrounds = self
            .tree
            .as_ref()
            .into_iter()
            .flat_map(SplitNode::leaves)
            .filter_map(|id| {
                model.screens.get(&id).map(|screen| {
                    (
                        id,
                        bboxy450_term::model::terminal_background(screen, model.ui.window.theme),
                    )
                })
            })
            .collect();
        self.closed = model
            .ui
            .window
            .sidebar
            .swarms
            .iter()
            .flat_map(|s| &s.closed)
            .filter_map(|c| {
                let seat = model.ui.seats.get(&c.seat)?;
                Some((c.seat, c.until, seat.name.clone(), seat.harness.0.clone()))
            })
            .collect();
        // Stage is owned by the UI input adapter. Scene only projects it.
        self.focused = stage.focused;
        self.seat_index = model
            .ui
            .seats
            .iter()
            .filter(|(_, s)| s.lifecycle == bbt_ui_model::Lifecycle::Open)
            .enumerate()
            .map(|(i, (id, _))| (*id, i))
            .collect();
        let rows = bbt_rt::diagnostics::span(bbt_rt::diagnostics::Phase::UiSeatRows);
        self.seats = model
            .ui
            .seats
            .iter()
            .filter(|(_, s)| s.lifecycle == bbt_ui_model::Lifecycle::Open)
            .map(|(id, seat)| {
                let mut row = SeatRow::new(id.to_string(), &seat.name, &seat.harness.0);
                row.role = seat.role.as_ref().map(|role| role.name.clone());
                row.title = model
                    .history_errors
                    .get(id)
                    .cloned()
                    .unwrap_or_else(|| seat.title.clone());
                row.subtitle = self
                    .subtitle
                    .then(|| model.subtitles.get(id).cloned())
                    .flatten();
                row.activity = seat.activity;
                row.grade = seat.grade;
                row.muted = seat.muted;
                row.may_mute = self.live;
                row.deferred = model.messaging_deferred.get(id).cloned();
                row.focused = Some(*id) == self.focused;
                // An inactive card dims its whole body once. In the selected
                // card, dim only rows other than the active seat.
                row.unfocused = Some(seat.swarm) == stage.swarm && !row.focused;
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
        rows.finish(self.seats.len() as u64, 0);
        measurement.finish(model.ui.seats.len() as u64, 0);
        bbt_rt::diagnostics::probe_after(
            bbt_rt::diagnostics::ProbeStage::LatestTaken,
            bbt_rt::diagnostics::ProbeStage::ProjectionFinished,
        );
    }
    #[cfg(test)]
    pub fn projections(&self) -> usize {
        self.projections
    }
    pub fn close_intent(
        &self,
        model: &bboxy450_term::model::Model,
        seat: SeatId,
        operation: bbt_mux::Ulid,
        now: Instant,
    ) -> Result<bboxy450_term::close::Intent, bbt_ui_model::EditError> {
        let (si, ti) = model
            .ui
            .tab_of(seat)
            .ok_or(bbt_ui_model::EditError::MissingTab)?;
        let tab = &model.ui.window.sidebar.swarms[si].tabs[ti];
        let tree = self
            .tree
            .as_ref()
            .filter(|tree| tree.leaves().contains(&seat))
            .unwrap_or(&tab.tree);
        bboxy450_term::close::Intent::prepare(
            &model.ui,
            tab.id,
            tree,
            seat,
            self.layout_revision,
            operation,
            now,
        )
    }
    pub fn layout_revision(&self) -> u64 {
        self.layout_revision
    }
    pub fn sidebar_width_for(&self, size: [f32; 2]) -> f32 {
        Self::fitted_sidebar_width(self.sidebar_width, size)
    }
    pub fn sidebar_open(&self) -> BTreeMap<String, bool> {
        self.swarms
            .iter()
            .map(|swarm| {
                (
                    format!("swarm:{}", swarm.id),
                    self.open_swarms.contains(&swarm.id),
                )
            })
            .chain(
                self.provider_cards()
                    .into_iter()
                    .map(|card| (card.key, card.expanded)),
            )
            .chain(
                ["panels/Tasks", "panels/Chat"]
                    .map(|key| (key.to_owned(), !self.sidebar_sections_closed.contains(key))),
            )
            .collect()
    }
    pub fn animate_sidebar_open(&mut self, key: &str, open: bool, now: Instant) {
        let Some(current) = self.sidebar_open().get(key).copied() else {
            return;
        };
        let from = self.sidebar_reveal(key, current, now);
        if self.set_sidebar_open(key, open) {
            self.sidebar_reveals.insert(
                key.into(),
                ui::modules::accordion::Reveal::new(from, open, now),
            );
        }
    }
    pub fn sidebar_animating(&self, key: &str, now: Instant) -> bool {
        self.sidebar_reveals.get(key).is_some_and(|r| r.active(now))
    }
    pub fn advance_sidebar_motion(&mut self, now: Instant) {
        self.sidebar_motion_now = now;
        self.sidebar_reveals.retain(|_, r| r.active(now));
    }
    fn sidebar_reveal(&self, key: &str, open: bool, now: Instant) -> f32 {
        self.sidebar_reveals
            .get(key)
            .filter(|r| r.target == open)
            .map_or(u8::from(open) as f32, |r| r.value(now))
    }
    fn panel_reveal(&self) -> [f32; 2] {
        let open = self.panel_open();
        ["panels/Tasks", "panels/Chat"].map(|key| {
            let index = usize::from(key == "panels/Chat");
            self.sidebar_reveal(key, open[index], self.sidebar_motion_now)
        })
    }
    pub fn set_sidebar_open(&mut self, key: &str, open: bool) -> bool {
        if let Some(id) = key
            .strip_prefix("swarm:")
            .and_then(|id| id.parse::<bbt_ui_model::SwarmId>().ok())
        {
            if !self.swarms.iter().any(|swarm| swarm.id == id) {
                return false;
            }
            if open {
                self.open_swarms.insert(id);
            } else {
                self.open_swarms.remove(&id);
            }
        } else if bbt_proto::gui::SIDEBAR_SECTIONS.contains(&key) {
            if open {
                self.sidebar_sections_closed.remove(key);
            } else {
                self.sidebar_sections_closed.insert(key.into());
            }
        } else {
            return false;
        }
        true
    }
    fn provider_cards(&self) -> Vec<SwarmCard> {
        bboxy450_term::hosts::sidebar_cards(&self.hosts)
            .into_iter()
            .map(|mut card| {
                let open = !self.sidebar_sections_closed.contains(&card.key);
                card.expanded = open;
                card.body = open;
                card.closed = !open;
                card
            })
            .collect()
    }
    pub fn fitted_sidebar_width(width: f32, size: [f32; 2]) -> f32 {
        // Keep a usable stage when the window cannot fit the saved preference.
        width
            .clamp(
                bbt_ui_model::input::MIN_WIDTH,
                bbt_ui_model::input::MAX_WIDTH,
            )
            .min((size[0] - 160.).max(1.))
    }
    /// The invisible eight-pixel grab zone follows the solved sidebar border.
    pub fn sidebar_resize_region(&self, layouts: &[c::Layout], size: [f32; 2]) -> Option<Rect> {
        let sidebar = layouts.get(1)?.rect;
        let x = (sidebar.right() - 4.).max(0.);
        let right = (sidebar.right() + 4.).min(size[0]);
        (right > x && size[1] > sidebar.y)
            .then(|| Rect::new(x, sidebar.y, right - x, size[1] - sidebar.y))
    }
    /// Retained hit boxes: never a guessed strip/pane coordinate.
    pub fn close_regions(&self, layouts: &[c::Layout]) -> Vec<(SeatId, Rect, bool)> {
        let mut out = vec![];
        for (slot, layout) in self.pane_keys().zip(layouts.iter().skip(2)) {
            let Slot::Seat(id) = slot else {
                continue;
            };
            if let Some(rect) = named(layout, "pane/strip/close").map(|layout| layout.rect) {
                out.push((id, rect, false));
            }
        }
        fn undo(layout: &c::Layout, id: SeatId) -> Option<Rect> {
            if matches!(&layout.kind,c::LayoutKind::Named(name) if name.ends_with(&format!("closed/{id}/undo")))
            {
                return Some(layout.rect);
            }
            layout.children.iter().find_map(|child| undo(child, id))
        }
        if let Some(sidebar) = layouts.get(1) {
            for (id, _, _, _) in &self.closed {
                if let Some(rect) = undo(sidebar, *id) {
                    out.push((*id, rect, true));
                }
            }
        }
        out
    }
    pub fn swarm_folder_regions(
        &self,
        layouts: &[c::Layout],
    ) -> Vec<(bbt_proto::SwarmId, Rect, bool)> {
        let mut out = vec![];
        if let Some(sidebar) = layouts.get(1) {
            let viewport = self
                .sidebar_extent(layouts)
                .map_or(sidebar.rect, |(r, _)| r);
            for swarm in &self.swarms {
                for (action, reset) in [("choose", false), ("default", true)] {
                    if let Some(button) =
                        named_suffix(sidebar, &format!("swarm-folder/{}/{action}", swarm.id))
                    {
                        let r = button.rect;
                        let x = r.x.max(viewport.x);
                        let y = r.y.max(viewport.y);
                        let w = r.right().min(viewport.right()) - x;
                        let h = r.bottom().min(viewport.bottom()) - y;
                        if w > 0. && h > 0. {
                            out.push((swarm.id, Rect::new(x, y, w, h), reset));
                        }
                    }
                }
            }
        }
        out
    }
    /// Reveal an empty swarm's controls using solved geometry. An explicit
    /// offscreen reveal may probe the full sidebar; ordinary frames stay windowed.
    pub fn swarm_folder_scroll(
        &self,
        layouts: &[c::Layout],
        swarm: bbt_proto::SwarmId,
    ) -> Option<f32> {
        let visible_sidebar = layouts.get(1)?;
        let (viewport, max) = self.sidebar_extent(layouts)?;
        let name = format!("swarm-folder/{swarm}/choose");
        let full;
        let sidebar = if named_suffix(visible_sidebar, &name).is_some() {
            visible_sidebar
        } else {
            // Recreate exactly this sidebar height, excluding any dock. This is
            // an on-demand geometry query, never paint or an interactive hit tree.
            let footer = if self.identity.is_some() { 24. } else { 0. };
            full = self
                .layout_contents(
                    [layouts.first()?.rect.w, visible_sidebar.rect.bottom() + footer],
                    false,
                    0.,
                    Instant::now(),
                    false,
                    true,
                )
                .ok()?;
            full.get(1)?
        };
        let button = named_suffix(sidebar, &name)?.rect;
        let index = self.swarms.iter().position(|s| s.id == swarm)?;
        let head = named(sidebar, &format!("swarm{index}/head"))?.rect;
        let top = if button.bottom() - head.y <= viewport.h {
            head.y
        } else {
            button.y
        };
        let delta = if top < viewport.y {
            top - viewport.y
        } else {
            (button.bottom() - viewport.bottom()).max(0.)
        };
        Some((self.sidebar_scroll + delta).clamp(0., max))
    }
    pub fn focused(&self) -> Option<SeatId> {
        match self
            .starting
            .as_ref()
            .and_then(|state| state.projection.focused)
        {
            Some(Slot::Launch(_)) => None,
            Some(Slot::Seat(id)) => Some(id),
            None => self.focused,
        }
    }
    pub fn set_tasks(
        &mut self,
        tasks: Vec<(SeatId, String, ui::modules::seat_row::TaskRow, Option<i32>)>,
    ) {
        self.tasks = tasks;
    }
    pub fn set_unread(&mut self, unread: &BTreeMap<SeatId, ui::modules::stream_dock::UnreadCount>) {
        for row in &mut self.seats {
            row.unread = row
                .key
                .parse::<SeatId>()
                .ok()
                .and_then(|id| unread.get(&id))
                .filter(|count| count.any())
                .map(|count| (count.to_string(), row.role.clone().unwrap_or_default()));
        }
    }
    fn task_children(&self, row: &SeatRow, ctx: ui::Context) -> Vec<c::Element> {
        self.tasks
            .iter()
            .filter(|(seat, ..)| seat.to_string() == row.key)
            .map(|(_, id, task, exit)| {
                let key = format!("task/{id}");
                let mut element = task.element_with_verify(ctx, *exit);
                c::namespace(&mut element, &format!("{key}/"));
                ui::modules::seat_row::named(key, element)
            })
            .collect()
    }
    pub fn task_regions(&self, layouts: &[c::Layout]) -> ui::modules::stream_dock::Regions {
        let Some(sidebar) = layouts.get(1) else {
            return Default::default();
        };
        let mut regions = ui::modules::stream_dock::Regions::retained(sidebar);
        for (_, id, task, _) in &self.tasks {
            regions.task_row(&format!("task/{id}"), id, task.may_accept);
        }
        regions
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
    /// Retire each four-second notice once; bounded stream lines survive projections.
    pub fn advance_restarts(&mut self, now: Instant) -> bool {
        let mut changed = false;
        for (id, (generation, at, swarm)) in &mut self.restart_generations {
            if at.is_some_and(|at| now.saturating_duration_since(at).as_secs_f32() >= 4.) {
                *at = None;
                if let Some(row) = self
                    .seat_index
                    .get(id)
                    .and_then(|index| self.seats.get(*index))
                {
                    self.restart_lines.push((
                        *swarm,
                        format!("{} restarted · process generation {generation}", row.name),
                    ));
                    changed = true;
                }
            }
        }
        if self.restart_lines.len() > 512 {
            self.restart_lines.drain(..self.restart_lines.len() - 512);
        }
        changed
    }
    pub fn restart_lines(&self, swarm: bbt_ui_model::SwarmId) -> impl Iterator<Item = &str> {
        self.restart_lines
            .iter()
            .filter_map(move |(owner, text)| (*owner == swarm).then_some(text.as_str()))
    }
    /// Read the actual pane body after layout, including its padding.
    pub fn pane_bounds(&self, layouts: &[c::Layout]) -> Vec<(SeatId, Rect)> {
        self.pane_keys()
            .zip(layouts.iter().skip(2))
            .filter_map(|(slot, layout)| {
                if let Slot::Seat(id) = slot {
                    pane_body(layout).map(|rect| (id, rect))
                } else {
                    None
                }
            })
            .collect()
    }
    /// Spatial focus uses the whole pane, independently of terminal body sizes.
    pub fn focus_bounds(&self, layouts: &[c::Layout]) -> Vec<(SeatId, Rect)> {
        self.pane_keys()
            .zip(layouts.iter().skip(2))
            .filter_map(|(slot, layout)| {
                if let Slot::Seat(id) = slot {
                    Some((id, layout.rect))
                } else {
                    None
                }
            })
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
        self.layout_with_dock(size, effects, 0.)
    }
    /// As `layout`, with every seat's swell and attention evaluated at `now`.
    #[cfg(test)]
    pub fn layout_at(
        &self,
        size: [f32; 2],
        effects: bool,
        now: Instant,
    ) -> Result<Vec<c::Layout>, String> {
        self.layout_docked_at(size, effects, 0., now)
    }
    pub fn layout_with_dock(
        &self,
        size: [f32; 2],
        effects: bool,
        dock_height: f32,
    ) -> Result<Vec<c::Layout>, String> {
        self.layout_docked_at(size, effects, dock_height, Instant::now())
    }
    #[cfg(test)]
    pub fn local_page_chrome(
        &self,
        size: [f32; 2],
        dock_height: f32,
    ) -> Result<Vec<c::Layout>, String> {
        self.local_page_chrome_at(size, dock_height, Instant::now())
    }
    pub fn local_page_chrome_at(
        &self,
        size: [f32; 2],
        dock_height: f32,
        now: Instant,
    ) -> Result<Vec<c::Layout>, String> {
        self.layout_contents(size, false, dock_height, now, false, false)
    }
    pub fn layout_docked_at(
        &self,
        size: [f32; 2],
        effects: bool,
        dock_height: f32,
        now: Instant,
    ) -> Result<Vec<c::Layout>, String> {
        self.layout_contents(size, effects, dock_height, now, true, false)
    }
    fn layout_contents(
        &self,
        size: [f32; 2],
        effects: bool,
        dock_height: f32,
        now: Instant,
        panes: bool,
        full_sidebar: bool,
    ) -> Result<Vec<c::Layout>, String> {
        let ctx = self.ui_context();
        // Rows, tabs and panes carry each seat's swell and attention (rim-glow §7-8).
        let mut seats = self.seats.clone();
        for (id, index) in &self.seat_index {
            (seats[*index].swell, seats[*index].attention) =
                self.envelope(*id, now, LocateWhere::Row);
            if let Some((_, Some(at), _)) = self.restart_generations.get(id) {
                let elapsed = now.saturating_duration_since(*at).as_secs_f32();
                seats[*index].restarted = elapsed < 4.;
                seats[*index].restarted_alpha = restarted_alpha(elapsed);
            }
        }
        let title = STYLE.spacing.title_bar as f32;
        let sidebar_width = self.sidebar_width_for(size);
        let footer_height = if self.identity.is_some() { 24. } else { 0. };
        let sidebar_height = (size[1] - title - footer_height - dock_height).max(1.);
        let bounds = Rect::new(
            sidebar_width,
            title,
            (size[0] - sidebar_width).max(1.),
            (size[1] - title).max(1.),
        );
        let card_style = ui::Style::new(ui::Preset::SwarmCard)
            .resolve(ctx)
            .map_err(|e| e.to_string())?;
        let row_style = ui::Style::new(ui::Preset::CardRow)
            .resolve(ctx)
            .map_err(|e| e.to_string())?;
        let strip_width = (sidebar_width
            - 2. * STYLE.spacing.s3 as f32
            - STYLE.material.hairline_px as f32
            - TOKENS.sidebar.edge as f32
            - card_style.padding[1]
            - card_style.padding[3]
            - row_style.margin[1]
            - row_style.margin[3])
            .max(1.);
        // Adapted from bboxy450-term signed candidate 9739ea66 (scene.rs).
        // Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
        // Live swarms replace these three fields below. Do not populate and
        // clone an all-seat template that is never laid out or painted.
        let scoped = self.live && !self.swarms.is_empty();
        let mut extra: Vec<_> = if !scoped && self.tree.is_none() {
            seats.iter().map(|s| s.element_in_swarm(ctx)).collect()
        } else {
            vec![]
        };
        extra.extend(
            self.closed
                .iter()
                .filter(|_| !scoped)
                .filter(|(_, until, _, _)| now < *until)
                .map(|(id, until, name, harness)| {
                    ui::modules::closed_strip::ClosedStrip {
                        key: id.to_string(),
                        name: name.clone(),
                        harness: harness.clone(),
                        remaining_ms: until.saturating_duration_since(now).as_millis() as u64,
                        grace_ms: bboxy450_term::close::GRACE.as_millis() as u64,
                    }
                    .element(ctx, &self.metrics, strip_width)
                }),
        );
        let card = SwarmCard {
            key: "swarm".into(),
            name: "swarm".into(),
            color: Color::hex("#2E6BFF").0,
            seats: if scoped { vec![] } else { seats.clone() },
            expanded: true,
            closed: false,
            unfocused: false,
            body: true,
            new_seat: true,
            extra,
            shadows: vec![],
            tabs: self
                .tree
                .as_ref()
                .filter(|_| !scoped)
                .map(|tree| {
                    vec![TabRow {
                        name: if self.live { "seats" } else { "all fifteen" }.into(),
                        shortcut: Some("⌘1".into()),
                        tree: self.mini(tree),
                        seats: seats
                            .iter()
                            .map(|row| TabSeat {
                                row: row.clone(),
                                children: self.task_children(row, ctx),
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
        let mut cards = if !scoped {
            let mut card = card;
            card.extra.extend(self.starting_rows(None, ctx));
            vec![card]
        } else {
            self.swarms
                .iter()
                .map(|swarm| {
                    let mut view = card.clone();
                    view.key = swarm.id.to_string();
                    view.name = swarm.name.clone();
                    let open = self.open_swarms.contains(&swarm.id);
                    view.expanded = open;
                    view.closed = !open;
                    view.body = open;
                    view.unfocused = Some(swarm.id) != self.selected_swarm;
                    view.seats = swarm
                        .tabs
                        .iter()
                        .flat_map(|tab| &tab.seats)
                        .filter_map(|id| self.seat_index.get(id).map(|i| seats[*i].clone()))
                        .collect();
                    view.tabs = swarm
                        .tabs
                        .iter()
                        .map(|tab| TabRow {
                            name: tab.name.clone(),
                            shortcut: tab.shortcut.map(|n| format!("⌘{n}")),
                            tree: self.mini(&tab.tree),
                            seats: tab
                                .seats
                                .iter()
                                .filter_map(|id| {
                                    self.seat_index.get(id).map(|i| TabSeat {
                                        row: seats[*i].clone(),
                                        children: self.task_children(&seats[*i], ctx),
                                    })
                                })
                                .collect(),
                            expanded: tab.expanded,
                            focused_key: self.focused.map(|id| id.to_string()),
                        })
                        .collect();
                    view.extra = self
                        .closed
                        .iter()
                        .filter(|(id, until, _, _)| {
                            now < *until && swarm.closed.iter().any(|c| c.seat == *id)
                        })
                        .map(|(id, until, name, harness)| {
                            ui::modules::closed_strip::ClosedStrip {
                                key: id.to_string(),
                                name: name.clone(),
                                harness: harness.clone(),
                                remaining_ms: until.saturating_duration_since(now).as_millis()
                                    as u64,
                                grace_ms: bboxy450_term::close::GRACE.as_millis() as u64,
                            }
                            .element(ctx, &self.metrics, strip_width)
                        })
                        .collect();
                    view.extra.extend(self.starting_rows(Some(swarm.id), ctx));
                    view.extra.push(ui::modules::swarm_card::workdir_row(
                        &swarm.id.to_string(),
                        swarm.workdir.as_deref(),
                        ctx,
                    ));
                    view
                })
                .collect()
        };
        cards.extend(self.provider_cards());
        let sidebar = Sidebar {
            cards,
            size: [sidebar_width, sidebar_height],
            scroll: self.sidebar_scroll,
            phone: false,
            new_swarm: true,
        };
        let mut layouts = vec![
            TitleBar {
                crumb: if !self.live {
                    "swarm / all fifteen".into()
                } else {
                    self.swarms
                        .iter()
                        .find(|s| Some(s.id) == self.selected_swarm)
                        .map_or_else(
                            || "swarm / seats".into(),
                            |s| {
                                let tab = s
                                    .tabs
                                    .iter()
                                    .find(|t| self.focused.is_some_and(|id| t.tree.contains(id)));
                                tab.map_or_else(
                                    || s.name.clone(),
                                    |t| format!("{} / {}", s.name, t.name),
                                )
                            },
                        )
                },
                ..Default::default()
            }
            .element(ctx)
            .layout(Rect::new(0., 0., size[0], title), ctx, &self.metrics)
            .map_err(|e| e.to_string())?,
            {
                let element = sidebar.element_with_reveals(
                    ctx,
                    None,
                    &self
                        .sidebar_reveals
                        .iter()
                        .filter(|(_, r)| r.active(now))
                        .map(|(key, r)| {
                            (
                                key.strip_prefix("swarm:").unwrap_or(key).to_owned(),
                                r.value(now),
                            )
                        })
                        .collect(),
                    &self.metrics,
                );
                let rect = Rect::new(0., title, sidebar_width, sidebar_height);
                if full_sidebar {
                    element.layout_unwindowed(rect, ctx, &self.metrics)
                } else {
                    element.layout(rect, ctx, &self.metrics)
                }
                .map_err(|e| e.to_string())?
            },
        ];
        // The gutter between panes is the rim size and the stage inset half of it (rim-glow
        // §2): neighbouring rims' outer halves meet; depth never reaches the layout.
        for (slot, rect) in self.pane_rects(bounds, panes) {
            let mut seat = self.slot_row(slot, &seats);
            if let Slot::Seat(id) = slot {
                (seat.swell, seat.attention) = self.envelope(id, now, LocateWhere::Pane);
            }
            seat.unfocused = false; // The pane dims its strip and content together.
            let pane_ctx = ui::Context {
                seat: seat.color.unwrap_or(ctx.seat),
                focused: seat.focused,
                ..ctx
            };
            let pane = Pane {
                strip: Strip::new(seat),
                lines: self.slot_lines(slot),
                gap_from: None,
                resumed: None,
                min_height: rect.h,
                unfocused: !pane_ctx.focused,
                reconnecting: false,
                reconnected: false,
                glow_seed: match slot {
                    Slot::Seat(id) => effects.then_some(id.0.0 as u32),
                    Slot::Launch(_) => None,
                },
                opacity: 1.,
                depth: TOKENS.glow.depth as f32,
                rim: self.rim,
            };
            layouts.push(
                pane.element(pane_ctx)
                    .layout(rect, pane_ctx, &self.metrics)
                    .map_err(|e| e.to_string())?,
            );
        }
        Ok(layouts)
    }
    pub fn local_page_layout(
        &self,
        size: [f32; 2],
        dock: f32,
        label: &str,
        body: c::Element,
        scroll: f32,
    ) -> Result<LocalPageLayout, String> {
        self.local_page_frame(size, dock, label, body, scroll, false)
    }
    fn local_page_frame(
        &self,
        size: [f32; 2],
        dock: f32,
        label: &str,
        body: c::Element,
        scroll: f32,
        bounded: bool,
    ) -> Result<LocalPageLayout, String> {
        let ctx = self.ui_context();
        let sidebar = self.sidebar_width_for(size);
        let rect = Rect::new(
            sidebar,
            STYLE.spacing.title_bar as f32,
            (size[0] - sidebar).max(1.),
            (size[1] - STYLE.spacing.title_bar as f32 - dock).max(1.),
        );
        let header = 44_f32.min(rect.h);
        let viewport = Rect::new(rect.x, rect.y + header, rect.w, (rect.h - header).max(1.));
        let content_height = if bounded {
            viewport.h
        } else {
            body.layout(Rect::new(0., 0., rect.w, 0.), ctx, &self.metrics)
                .map_err(|e| e.to_string())?
                .rect
                .h
        };
        let max_scroll = (content_height - viewport.h).max(0.);
        let mut close_style = ui::Style::new(ui::Preset::ButtonGhost);
        close_style.overrides.text = Some(ui::Text {
            size: Some(STYLE.r#type.label as f32),
            line_height: 1.4,
            ..Default::default()
        });
        let close = c::Element::Named {
            name: "local/close".into(),
            content: Box::new(c::Element::Box(c::Box::new(
                close_style,
                vec![
                    c::Element::Text(c::Text::new(
                        "Back to panes · Esc",
                        ui::Text {
                            size: Some(STYLE.r#type.label as f32),
                            line_height: 1.4,
                            ..Default::default()
                        },
                    ))
                    .into(),
                ],
            ))),
        };
        let mut style = ui::Style::new(ui::Preset::Card);
        style.overrides.fill = Some(ui::Paint::solid(ui::Color::Background));
        style.overrides.border = Some(ui::Border {
            width: ui::Length::Px(0.),
            paint: ui::Paint::solid(ui::Color::Transparent),
            outside: false,
        });
        style.overrides.radii = Some([ui::Length::Px(0.); 4]);
        style.overrides.padding = Some([ui::Length::Px(0.); 4]);
        let mut hs = style.clone();
        hs.overrides.padding = Some([0., 16., 0., 16.].map(ui::Length::Px));
        hs.overrides.height = Some(ui::Length::Px(header));
        let mut head = c::Box::new(
            hs,
            vec![
                c::Element::Text(c::Text::new(
                    label,
                    ui::Text {
                        size: Some(STYLE.r#type.row as f32),
                        weight: 600,
                        line_height: 1.4,
                        ..Default::default()
                    },
                ))
                .into(),
                c::Item {
                    element: c::Element::Spacer,
                    extent: c::Extent::Fill,
                },
                close.into(),
            ],
        );
        head.gap = ui::Length::Px(0.);
        let content = c::Element::Named {
            name: "local/viewport".into(),
            content: Box::new(if bounded {
                body
            } else {
                c::Element::ScrollView(c::ScrollView {
                    viewport: [viewport.w, viewport.h],
                    offset: [0., scroll.clamp(0., max_scroll)],
                    content_width: Some(viewport.w),
                    content: Box::new(body),
                })
            }),
        };
        let mut root = c::Box::new(
            style,
            vec![
                c::Item {
                    element: c::Element::Box(head),
                    extent: c::Extent::Fixed(header),
                },
                c::Item {
                    element: content,
                    extent: c::Extent::Fill,
                },
            ],
        );
        root.direction = Axis::Column;
        root.align = c::Alignment::Stretch;
        root.gap = ui::Length::Px(0.);
        let layout = c::Element::Box(root)
            .layout(rect, ctx, &self.metrics)
            .map_err(|e| e.to_string())?;
        let close = named(&layout, "local/close")
            .ok_or("local page close control missing")?
            .rect;
        Ok(LocalPageLayout {
            layout,
            viewport,
            close,
            max_scroll,
            settings: None,
            focus: None,
            context: ctx,
        })
    }
    pub fn settings_page_layout(
        &self,
        size: [f32; 2],
        view: &ui::modules::settings::Settings,
        state: &ui::modules::settings::controller::Presentation,
        status: &str,
    ) -> Result<LocalPageLayout, String> {
        let width = (size[0] - self.sidebar_width_for(size)).max(1.);
        // The conversation dock occupies the sidebar only. Settings owns the
        // full stage below its title and header, including in tall windows.
        let height = (size[1] - STYLE.spacing.title_bar as f32 - 44.).max(1.);
        let body = view.element_with(self.ui_context(), &self.metrics, width, height, state);
        let mut page = self.local_page_frame(
            size,
            0.,
            &format!("Settings · {} · {status}", view.section),
            body,
            0.,
            true,
        )?;
        let regions = ui::modules::settings::controller::Regions::new(&page.layout, view)
            .ok_or("Settings hit regions missing")?;
        page.viewport = regions.viewport;
        page.max_scroll = regions.max_scroll;
        let clip = if matches!(
            state.focus,
            ui::modules::settings::controller::Focus::Control(_)
        ) {
            regions.viewport
        } else {
            page.layout.rect
        };
        page.focus = regions.focus_rect(&state.focus).map(|rect| (rect, clip));
        page.settings = Some(regions);
        Ok(page)
    }
    pub fn graph_page_layout(
        &self,
        size: [f32; 2],
        dock: f32,
        checklist: &ui::modules::e2e_checklist::Checklist,
        scroll: f32,
    ) -> Result<LocalPageLayout, String> {
        let ctx = self.ui_context();
        let width = (size[0] - self.sidebar_width_for(size)).max(1.);
        let graph = ui::modules::graph_tab::GraphTab {
            decisions: checklist.decisions.clone(),
            e2e: Some(checklist.clone()),
            only: Some(vec!["decisions".into(), "e2e".into()]),
            ..Default::default()
        };
        self.local_page_layout(
            size,
            dock,
            "Graph · E2E checklist",
            graph.element_width(ctx, &self.metrics, width),
            scroll,
        )
    }
    pub fn paint(
        &self,
        layouts: &[c::Layout],
        size: [f32; 2],
        overlay: Option<&PerfHud>,
        launcher: Option<&LauncherLayout>,
    ) -> Vec<c::Paint> {
        let ctx = self.ui_context();
        let mut background = DrawList::default();
        background.rect(
            Rect::new(0., 0., size[0], size[1]),
            0.,
            Color::hex(ctx.theme.palette().bg),
        );
        let mut ops = vec![c::Paint::Draw(background)];
        // Every block, then every rim, so a swell paints over its neighbours (rim-glow §7).
        let mut rims = vec![];
        for layout in layouts {
            let (body, rim) = c::split_rims(c::paint(layout, ctx));
            ops.extend(body);
            rims.extend(rim);
        }
        if self.live {
            // Real PTYs replace the kit's transparent terminal. Carry their
            // background through the padding without changing cell/rim bounds.
            let mut padding = DrawList::default();
            for (slot, layout) in self.pane_keys().zip(layouts.iter().skip(2)) {
                if let Slot::Seat(id) = slot
                    && let Some(term) = named(layout, "pane/term")
                    && let Some(background) = self.terminal_backgrounds.get(&id)
                {
                    padding.rect(
                        term.rect,
                        0.,
                        crate::gpu::terminal_background(
                            *background,
                            ctx.theme,
                            self.focused() == Some(id),
                        ),
                    );
                }
            }
            ops.push(c::Paint::Draw(padding));
        }
        if self.glow {
            ops.extend(rims);
        }
        if let Some(identity) = &self.identity {
            let note = identity_note(identity);
            let mut footer = DrawList::default();
            footer.text(
                Rect::new(
                    12.,
                    size[1] - 20.,
                    (self.sidebar_width_for(size) - 24.).max(1.),
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
    pub(super) fn conversation_geometry(
        &self,
        size: [f32; 2],
        requested_width: Option<f32>,
        sidebar: ui::modules::composer::SidebarLayout,
    ) -> crate::conversation_cache::Geometry {
        crate::conversation_cache::Geometry {
            size,
            width: Self::fitted_sidebar_width(requested_width.unwrap_or(self.sidebar_width), size),
            footer: self.identity.is_some(),
            sidebar,
            context: self.ui_context(),
            open: self.panel_open(),
            reveal: self.panel_reveal(),
        }
    }
    fn panel_open(&self) -> [bool; 2] {
        [
            !self.sidebar_sections_closed.contains("panels/Tasks"),
            !self.sidebar_sections_closed.contains("panels/Chat"),
        ]
    }
    pub fn conversation_layout(
        &self,
        size: [f32; 2],
        view: &ui::modules::composer::Conversation,
    ) -> Result<ConversationLayout, String> {
        let ctx = self.ui_context();
        let width = self.sidebar_width_for(size);
        let footer = if self.identity.is_some() { 24. } else { 0. };
        let available = (size[1] - STYLE.spacing.title_bar as f32 - footer).max(1.);
        let open = self.panel_open();
        let reveal = self.panel_reveal();
        let mut height = 0.;
        for bits in 0..4 {
            let state = [bits & 1 != 0, bits & 2 != 0];
            let weight = (if state[0] { reveal[0] } else { 1. - reveal[0] })
                * (if state[1] { reveal[1] } else { 1. - reveal[1] });
            if weight == 0. {
                continue;
            }
            let minimum = view
                .minimum_sidebar_height(ctx, width, &self.metrics, state)?
                .min(available);
            let maximum = (available - 96.).max(minimum);
            let wanted = if state[1] {
                available * self.sidebar_layout.conversation_fraction
            } else if state[0] {
                self.sidebar_layout.tasks_height.unwrap_or(96.)
                    + ui::modules::accordion::Accordion::chrome_height(ctx, false)
                    + TOKENS.layout.cohort_gap as f32
            } else {
                minimum
            };
            height += weight * wanted.clamp(minimum, maximum);
        }
        let element = view
            .element_sidebar_revealed(
                ctx,
                [width, height],
                &self.metrics,
                [
                    self.sidebar_layout.composer_body_height,
                    self.sidebar_layout.tasks_height,
                ],
                reveal,
            )
            .map_err(|e| e.to_string())?;
        let layout = element
            .layout(
                Rect::new(0., size[1] - footer - height, width, height),
                ctx,
                &self.metrics,
            )
            .map_err(|e| e.to_string())?;
        let regions = if open[1] && reveal[1] > 0. {
            ui::modules::composer::Regions::new(&layout).map_err(|e| e.to_string())?
        } else {
            Default::default()
        };
        let body_top =
            named(&layout, "composer-margin").map_or(layout.rect.bottom(), |node| node.rect.y);
        let handle = |y: f32| {
            Rect::new(
                0.,
                (y - 4.).max(STYLE.spacing.title_bar as f32),
                (width - 4.).max(0.),
                8_f32.min((size[1] - footer - y + 4.).max(0.)),
            )
        };
        let tasks = named(&layout, "conversation/task-section").filter(|part| part.rect.h > 0.);
        let tasks_height = tasks.map_or(0., |part| part.rect.h);
        let splitters = [
            handle(layout.rect.y),
            if open[1] {
                handle(body_top)
            } else {
                Rect::default()
            },
            if open[0] {
                tasks.map_or(Rect::default(), |part| handle(part.rect.bottom()))
            } else {
                Rect::default()
            },
        ];
        let mut actions = ui::modules::stream_dock::Regions::dock(&layout, &view.stream);
        if let Some(tasks) = &view.tasks {
            actions.extend(ui::modules::stream_dock::Regions::dock_from(
                &layout,
                tasks,
                view.stream.entries.len(),
            ));
        }
        actions.record_text_links(&layout, &self.metrics);
        actions.attachments("composer/attachments", &view.composer.attach);
        Ok(ConversationLayout {
            context: ctx,
            layout,
            regions,
            height,
            splitters,
            tasks_height,
            available_height: available,
            actions,
        })
    }
    pub fn conversation_position(
        &self,
        layout: &ConversationLayout,
        value: &str,
        index: usize,
    ) -> [f32; 2] {
        layout
            .regions
            .position_at(&layout.layout, value, index, &self.metrics)
    }
    pub fn conversation_index(
        &self,
        layout: &ConversationLayout,
        value: &str,
        at: [f32; 2],
    ) -> usize {
        layout
            .regions
            .index_at(&layout.layout, value, at, &self.metrics)
    }
    pub fn conversation_attachment_index(
        &self,
        layout: &ConversationLayout,
        value: &str,
        at: [f32; 2],
    ) -> usize {
        layout
            .regions
            .attachment_index_at(&layout.layout, value, at, &self.metrics)
    }
    pub fn reject_index(
        &self,
        layout: &ConversationLayout,
        id: &str,
        value: &str,
        at: [f32; 2],
    ) -> Option<usize> {
        layout
            .actions
            .index_at(&layout.layout, id, value, at, &self.metrics)
    }
    #[cfg(test)]
    pub fn launcher_layout(
        &self,
        size: [f32; 2],
        launcher: &Launcher,
    ) -> Result<LauncherLayout, String> {
        self.launcher_layout_with_preview(size, launcher, None, 0.)
    }
    pub fn launcher_layout_with_preview(
        &self,
        size: [f32; 2],
        launcher: &Launcher,
        preview: Option<c::Element>,
        scroll: f32,
    ) -> Result<LauncherLayout, String> {
        let ctx = self.ui_context();
        let width = 760_f32.min(size[0] * 0.92);
        let side = (size[0] - width) * 0.5;
        let with_preview = preview.is_some();
        let content = match preview {
            Some(preview) => c::Element::Stack(c::Stack {
                direction: Axis::Column,
                gap: ui::Length::Px(12.),
                align: c::Alignment::Stretch,
                children: vec![preview.into(), launcher.element(ctx).into()],
            }),
            None => launcher.element(ctx),
        };
        let content = if with_preview {
            let height = content
                .layout(Rect::new(0., 0., width, 0.), ctx, &self.metrics)
                .map_err(|e| e.to_string())?
                .rect
                .h;
            let viewport = [width, height.min(size[1] * 0.82)];
            ui::modules::seat_row::named(
                "launcher-scroll",
                c::Element::ScrollView(c::ScrollView {
                    viewport,
                    offset: [0., scroll],
                    content_width: Some(width),
                    content: Box::new(content),
                }),
            )
        } else {
            content
        };
        let mut overlay = c::Overlay::new(content);
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
        let view = launcher.view();
        let row_available = view
            .rows
            .iter()
            .map(|item| {
                item.kind != "fixed"
                    && !item.down
                    && !item.link.as_ref().is_some_and(|link| link.down())
            })
            .collect();
        let rows = (0..view.rows.len())
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
        let viewport = named(&layout, "launcher-scroll").map_or(panel, |l| l.rect);
        fn scroll_extent(layout: &c::Layout) -> f32 {
            if let c::LayoutKind::ScrollView { content, .. } = &layout.kind {
                return (content[1] - layout.rect.h).max(0.);
            }
            layout.children.iter().map(scroll_extent).fold(0., f32::max)
        }
        let max_scroll = scroll_extent(&layout);
        Ok(LauncherLayout {
            viewport,
            max_scroll,
            context: ctx,
            layout,
            #[cfg(test)]
            panel,
            query,
            caret,
            query_origin,
            query_font,
            rows,
            selected: view.selected,
            row_available,
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
    /// Notices use the same row coordinates and clip as the prepared cell grid.
    /// These are layout/paint commands only; replay and pages remain on IO.
    pub fn history_foreground(
        &self,
        model: &bboxy450_term::model::Model,
        bounds: &[(SeatId, Rect)],
        scale: f32,
    ) -> Result<Vec<c::Paint>, String> {
        let mut ops = vec![];
        if !scale.is_finite() || scale <= 0. {
            return Ok(ops);
        }
        let ctx = self.ui_context();
        for (seat, body) in bounds {
            let (Some(history), Some(frame)) = (model.history.get(seat), model.frames.get(seat))
            else {
                continue;
            };
            if history.screen.rows == 0 {
                continue;
            }
            for gap in &history.gaps {
                let Some(rect) = history_row_rect(
                    *body,
                    [frame.width, frame.height],
                    history.screen.rows,
                    gap.row,
                    scale,
                ) else {
                    continue;
                };
                let key = format!("pane/history/{seat}/{}/gap", gap.from);
                let marker = bbt_ui::modules::pane::gap_marker(&key, None, rect.h);
                let layout = marker
                    .layout(rect, ctx, &self.metrics)
                    .map_err(|error| format!("history marker: {error}"))?;
                ops.extend(c::paint(&layout, ctx));
            }
        }
        Ok(ops)
    }
    /// App overlays accept the toolkit's arbitrary component tree and use the
    /// same shaping/context as every other native surface.
    pub fn context_menu_layout(
        &self,
        menu: &ui::overlays::context_menu::Menu,
        anchor: [f32; 2],
        size: [f32; 2],
    ) -> Result<c::Layout, String> {
        menu.layout(
            anchor,
            Rect::new(0., 0., size[0], size[1]),
            self.ui_context(),
            &self.metrics,
        )
        .map_err(Into::into)
    }
    pub fn app_dialog_layout(
        &self,
        mut modal: ui::overlays::Modal,
        size: [f32; 2],
    ) -> Result<c::Layout, String> {
        let width = 480_f32.min((size[0] - 24.).max(1.));
        modal.style.overrides.width = Some(ui::Length::Px(width));
        let element = modal.element(self.ui_context());
        let natural = element
            .layout(
                Rect::new(0., 0., width, 0.),
                self.ui_context(),
                &self.metrics,
            )
            .map_err(str::to_owned)?;
        element
            .layout(
                Rect::new(
                    (size[0] - width) / 2.,
                    ((size[1] - natural.rect.h) / 2.).max(12.),
                    width,
                    0.,
                ),
                self.ui_context(),
                &self.metrics,
            )
            .map_err(Into::into)
    }
    pub fn app_dialog_backdrop(&self, size: [f32; 2]) -> Vec<c::Paint> {
        let mut backdrop = c::Overlay::new(c::Element::Spacer);
        backdrop.style.overrides.width = Some(ui::Length::Px(size[0]));
        backdrop.style.overrides.height = Some(ui::Length::Px(size[1]));
        backdrop.style.overrides.radii = Some([ui::Length::Px(0.); 4]);
        backdrop.min_height = ui::Length::Px(size[1]);
        c::Element::Overlay(backdrop)
            .layout(
                Rect::new(0., 0., size[0], size[1]),
                self.ui_context(),
                &self.metrics,
            )
            .map(|layout| c::paint(&layout, self.ui_context()))
            .unwrap_or_default()
    }
    pub fn document_layout(
        &self,
        size: [f32; 2],
        host: &bboxy450_term::documents::Host,
    ) -> Result<DocumentLayout, String> {
        let ctx = self.ui_context();
        let document = host.view();
        let shell = ui::modules::document_modal::DocumentModal {
            document,
            modal: true,
            viewport: size,
            scroll: host.scroll.offset,
            record_links: Some(host.links.clone()),
            notice: host.error.as_ref().or(host.notice.as_ref()).cloned(),
            ..Default::default()
        };
        let element = shell.element_with_body(ctx, &self.metrics, size, &|width| {
            if let Some(prepared) = &host.prepared {
                prepared.markdown.hosted_body_element_with_controller(
                    &prepared.document.title,
                    ctx,
                    &self.metrics,
                    width,
                    &host.links,
                    &host.scroll,
                )
            } else {
                c::Element::Text(c::Text::new(
                    shell.document.body.clone(),
                    ui::Text::default(),
                ))
            }
        });
        let w = 980_f32.min(size[0] * 0.94);
        let h = size[1] * 0.82;
        let rect = Rect::new((size[0] - w) / 2., (size[1] - h) / 2., w, h);
        let layout = element
            .layout(rect, ctx, &self.metrics)
            .map_err(|e| e.to_string())?;
        let regions = ui::modules::document_modal::controller::Regions::retained_modal(&layout);
        Ok(DocumentLayout {
            layout,
            regions,
            source: host.observation_source(),
            context: ctx,
            viewport_size: size,
        })
    }
    pub fn foreground_with_document(
        &self,
        size: [f32; 2],
        launcher: Option<&LauncherLayout>,
        hud: Option<&PerfHud>,
        conversation: Option<&ConversationLayout>,
        document: Option<Vec<c::Paint>>,
        rims: Vec<c::Paint>,
    ) -> Vec<c::Paint> {
        let mut ops = self.foreground_with_conversation(size, launcher, None, conversation, rims);
        if let Some(document) = document {
            ops.extend(document);
        }
        ops.extend(self.overlay(size, hud));
        ops
    }
    pub fn foreground_with_conversation(
        &self,
        size: [f32; 2],
        launcher: Option<&LauncherLayout>,
        hud: Option<&PerfHud>,
        conversation: Option<&ConversationLayout>,
        rims: Vec<c::Paint>,
    ) -> Vec<c::Paint> {
        let mut ops = conversation.map_or_else(Vec::new, ConversationLayout::paint);
        // The dock is ordinary window content: the seat's outer swell must
        // cover it too. Retain the original rim clips; true overlays stay above.
        ops.extend(rims);
        ops.extend(self.foreground(size, launcher, hud));
        ops
    }
    /// The perf HUD (Cmd+Opt+F), top right as the lab's perf.html floats it over the app.
    pub fn overlay(&self, size: [f32; 2], hud: Option<&PerfHud>) -> Vec<c::Paint> {
        let Some(hud) = hud else { return vec![] };
        let ctx = self.ui_context();
        let width = PerfHud::width(ctx);
        let rect = Rect::new((size[0] - 14. - width).max(0.), 40., width, 0.);
        match hud.element(ctx, "hud").layout(rect, ctx, &self.metrics) {
            Ok(layout) => c::paint(&layout, ctx),
            Err(_) => vec![],
        }
    }
    /// Run errors stay visible even when the performance HUD is closed.
    pub fn error_overlay(&self, size: [f32; 2], unread: u64) -> Vec<c::Paint> {
        if unread == 0 {
            return Vec::new();
        }
        let ctx = self.ui_context();
        let text = c::Element::Text(c::Text::new(
            format!("{unread} unread errors · ctl debug errors"),
            ui::Text {
                step: ui::TypeStep::Micro,
                paint: ui::Paint::solid(ui::Color::Foreground),
                ..Default::default()
            },
        ));
        let card = c::Element::Box(c::Box::new(
            ui::Style::new(ui::Preset::Hud),
            vec![text.into()],
        ));
        let rect = Rect::new((size[0] - 330.).max(0.), (size[1] - 42.).max(0.), 316., 0.);
        card.layout(rect, ctx, &self.metrics)
            .map(|layout| c::paint(&layout, ctx))
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod error_hud_tests {
    use super::*;
    fn texts(ops: &[c::Paint]) -> Vec<String> {
        let mut out = Vec::new();
        for op in ops {
            match op {
                c::Paint::Group { ops, .. } => out.extend(texts(ops)),
                c::Paint::Draw(list) | c::Paint::Underlay(list) => {
                    for primitive in &list.primitives {
                        if let bbt_ui_model::draw::Prim::Text { text, .. } = primitive {
                            out.push(text.to_string());
                        }
                    }
                }
                _ => {}
            }
        }
        out
    }
    // keeps: #56 — failures are visible with the perf overlay closed, clear on read.
    #[test]
    fn unread_errors_paint_their_count_without_a_performance_hud() {
        let scene = Scene::live();
        assert!(scene.error_overlay([800., 600.], 0).is_empty());
        let painted = texts(&scene.error_overlay([800., 600.], 3));
        assert_eq!(painted.concat(), "3 unread errors · ctl debug errors");
    }
}

fn history_row_rect(
    body: Rect,
    grid: [u32; 2],
    rows: usize,
    row: usize,
    scale: f32,
) -> Option<Rect> {
    if rows == 0 || row >= rows || !scale.is_finite() || scale <= 0. {
        return None;
    }
    let body = body.normalized();
    // Match draw_pane's device origin and scissor, including fractional layouts.
    let x = (body.x * scale).round().max(0.) / scale;
    let top = (body.y * scale).round().max(0.) / scale;
    let width = (grid[0] as f32).min((body.w * scale).floor().max(0.)) / scale;
    let bottom = top + (body.h * scale).floor().max(0.) / scale;
    let row_height = grid[1] as f32 / scale / rows as f32;
    let y = top + row as f32 * row_height;
    let height = row_height.min((bottom - y).max(0.));
    (width > 0. && height > 0.).then_some(Rect::new(x, y, width, height))
}

#[cfg(test)]
mod depth_tests {
    use super::*;

    #[test]
    fn scrollback_refusal_reaches_the_visible_pane_strip() {
        let id = SeatId::from_u128(83);
        let swarm = bbt_ui_model::SwarmId::from_u128(1);
        let mut model = bboxy450_term::model::Model::default();
        model.ui.seats.insert(
            id,
            bbt_ui_model::Seat::new(id, swarm, [0; 32], "fixture", "zsh"),
        );
        model.history_errors.insert(
            id,
            "Scrollback unavailable: viewport exceeds the bounded replay memory allowance".into(),
        );
        let mut scene = Scene::new();
        scene.set_model(&model, &model.ui.window.stage);
        assert_eq!(scene.seats[0].title, model.history_errors[&id]);
    }

    // Adapted from this project's tests/r5_stall.rs owned child driver.
    // Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
    #[test]
    #[ignore = "owned synthetic64/50 phase packet; existing R5 functional assertions"]
    #[allow(
        clippy::disallowed_methods,
        reason = "owns this private child and its logs"
    )]
    fn live_swarm_template_functional_phase_packet() {
        use std::{
            process::{Command, Stdio},
            time::Duration,
        };
        let directory = tempfile::Builder::new()
            .prefix("scene-template-phase-")
            .tempdir()
            .unwrap();
        let root = directory.keep();
        let report = root.join("report.json");
        let mut child = Command::new(
            std::env::var("BBOXY_SCENE_TEMPLATE_BINARY")
                .expect("use make test-scene-template-phase"),
        )
        .args([
            "--stall-test",
            "--stall-headless",
            "--stall-seats",
            "64",
            "--stall-watched",
            "50",
            "--stall-seconds",
            "5",
            "--stall-idle",
            "2",
            "--stall-report",
        ])
        .arg(&report)
        .arg("--stall-run-dir")
        .arg(root.join("fixture"))
        .env("BBOXY_TEST_CUSTODY", "memory")
        .stdout(Stdio::from(
            std::fs::File::create(root.join("stdout.log")).unwrap(),
        ))
        .stderr(Stdio::from(
            std::fs::File::create(root.join("stderr.log")).unwrap(),
        ))
        .spawn()
        .unwrap();
        std::fs::write(root.join("owned-pid"), child.id().to_string()).unwrap();
        println!("private synthetic64/50 phase evidence: {}", root.display());
        let started = Instant::now();
        let status = loop {
            if let Some(status) = child.try_wait().unwrap() {
                break status;
            }
            if started.elapsed() > Duration::from_secs(90) {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!(
                    "owned phase child exceeded unchanged 90s boundary; {}",
                    root.display()
                );
            }
            std::thread::sleep(Duration::from_millis(20));
        };
        let value: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&report).expect("actual R5 report")).unwrap();
        println!(
            "actual child exit={status}; load={}→{}; failures={}; phase_packet={}",
            value["host_load_before"],
            value["host_load_after"],
            value["failures"],
            report.display()
        );
        assert!(
            status.success(),
            "actual R5 child failure; no ignored idle or timing result"
        );
        assert_eq!(value["kind"], "synthetic-render-state");
        assert_eq!(value["rows"], 64);
        assert_eq!(value["watched"], 50);
        assert_eq!(value["real_ptys"], 0);
        assert_eq!(value["timing_gated"], false);
        assert_eq!(value["stdout_bytes"], 1_000_000_000u64);
        assert!(value["fault_read_ms"].as_f64().unwrap() >= 2000.);
        assert_eq!(
            value["metrics"]["encoded_changes"]
                .as_object()
                .unwrap()
                .len(),
            50
        );
        assert_eq!(value["metrics"]["idle_frames"], 0);
        assert_eq!(value["metrics"]["guard_violations"], 0);
        assert_eq!(value["failures"], serde_json::json!([]));
        assert!(value["metrics"]["frame_budget"].is_object());
        assert_eq!(value["metrics"]["diagnostics"]["enabled"], true);
    }

    // keeps: spec ux-prototype §5; live and implicit single-swarm chrome agree
    #[test]
    #[allow(
        clippy::disallowed_methods,
        reason = "writes only an explicitly requested private paint comparison fixture"
    )]
    fn live_swarm_template_keeps_fallback_paint_and_pane_geometry() {
        use bbt_ui_model::{Swarm, SwarmId, Tab, TabId};
        let mut snapshots = Vec::new();
        for (theme, density) in [
            (ui::Theme::Dark, ui::Density::Cozy),
            (ui::Theme::Dark, ui::Density::Compact),
            (ui::Theme::Light, ui::Density::Cozy),
            (ui::Theme::Light, ui::Density::Compact),
        ] {
            for live in [false, true] {
                let mut scene = Scene::new();
                scene.live = live;
                scene.context = ui::Context::new(theme, density);
                // Real live projections use the seat identity as the row key.
                for (id, index) in &scene.seat_index {
                    scene.seats[*index].key = id.to_string();
                }
                let size = [1600., 960.];
                let at = Instant::now();
                let fallback = scene.layout_at(size, false, at).unwrap();
                let paint = format!("{:?}", scene.paint(&fallback, size, None, None));
                let panes = scene.pane_bounds(&fallback);
                let mut tab = Tab::new(
                    TabId::from_u128(1),
                    if live { "seats" } else { "all fifteen" },
                    scene.tree.clone().unwrap(),
                );
                tab.shortcut = Some(1);
                tab.seats = (0..scene.seats.len())
                    .map(|i| SeatId::from_u128(i as u128))
                    .collect();
                scene.swarms = vec![Swarm::new(SwarmId::from_u128(1), "swarm", vec![tab])];
                scene.open_swarms.insert(SwarmId::from_u128(1));
                scene.selected_swarm = Some(SwarmId::from_u128(1));
                let explicit = scene.layout_at(size, false, at).unwrap();
                assert_eq!(scene.pane_bounds(&explicit), panes);
                let explicit_paint = format!("{:?}", scene.paint(&explicit, size, None, None));
                if !live {
                    assert_eq!(
                        explicit_paint, paint,
                        "unused explicit data cannot alter fallback paint"
                    );
                }
                snapshots.push(serde_json::json!({"theme":format!("{theme:?}"),"density":format!("{density:?}"),"live":live,"fallback":paint,"explicit":explicit_paint,"panes":format!("{panes:?}")}));
            }
        }
        // The canonical Make comparison runs this same fixture against the old
        // production population hunk and this cut, comparing every paint op.
        if let Ok(path) = std::env::var("BBOXY_SCENE_TEMPLATE_PAINT") {
            std::fs::write(path, serde_json::to_vec(&snapshots).unwrap()).unwrap();
        }
    }

    // keeps: spec c1-live-seats; each live swarm retains only its own rows
    #[test]
    fn live_swarm_template_keeps_rows_scoped_and_the_active_stage_unchanged() {
        use bbt_ui_model::{Swarm, SwarmId, Tab, TabId};
        fn text(layout: &c::Layout, out: &mut String) {
            if let c::LayoutKind::Text { runs, .. } = &layout.kind {
                for run in runs {
                    out.push_str(&run.text);
                    out.push(' ');
                }
            }
            for child in &layout.children {
                text(child, out);
            }
        }
        let mut scene = Scene::new();
        scene.live = true;
        scene.seats.truncate(4);
        scene.seat_index.retain(|_, index| *index < 4);
        for (id, index) in &scene.seat_index {
            scene.seats[*index].key = id.to_string();
            scene.seats[*index].name = ["a0", "a1", "b0", "b1"][*index].into();
        }
        let ids: Vec<_> = (0..4).map(SeatId::from_u128).collect();
        let a = SplitNode::split(
            Axis::Row,
            0.5,
            SplitNode::Leaf(ids[0]),
            SplitNode::Leaf(ids[1]),
        );
        let b = SplitNode::split(
            Axis::Column,
            0.5,
            SplitNode::Leaf(ids[2]),
            SplitNode::Leaf(ids[3]),
        );
        scene.tree = Some(a.clone());
        scene.swarms = vec![
            Swarm::new(
                SwarmId::from_u128(1),
                "alpha",
                vec![Tab::new(TabId::from_u128(1), "A", a)],
            ),
            Swarm::new(
                SwarmId::from_u128(2),
                "beta",
                vec![Tab::new(TabId::from_u128(2), "B", b)],
            ),
        ];
        scene
            .open_swarms
            .extend(scene.swarms.iter().map(|swarm| swarm.id));
        scene.selected_swarm = Some(SwarmId::from_u128(1));
        let at = Instant::now();
        for (index, label) in [(0, "a-closed"), (1, "b-closed")] {
            let id = SeatId::from_u128(90 + index as u128);
            let until = at + std::time::Duration::from_secs(2);
            scene.closed.push((id, until, label.into(), "zsh".into()));
            scene.swarms[index].closed.push(bbt_ui_model::ClosedStrip {
                seat: id,
                until,
                undo: bbt_ui_model::UndoRecord {
                    tab: TabId::from_u128(1),
                    tab_index: 0,
                    tab_name: "old".into(),
                    shortcut: None,
                    expanded: true,
                    parent: None,
                    process_generation: 1,
                },
            });
        }
        scene.starting = Some(starting::StartingScene {
            projection: bboxy450_term::starting::Projection {
                swarm: Some(SwarmId::from_u128(1)),
                tab: Some(TabId::from_u128(1)),
                focused: Some(Slot::Seat(ids[0])),
                tree: Some(SplitNode::split(
                    Axis::Row,
                    0.5,
                    SplitNode::Leaf(Slot::Seat(ids[0])),
                    SplitNode::Leaf(Slot::Seat(ids[1])),
                )),
            },
            rows: [
                (
                    bbt_mux::Ulid(91),
                    SeatRow::new("starting/a", "a-starting", "zsh"),
                ),
                (
                    bbt_mux::Ulid(92),
                    SeatRow::new("starting/b", "b-starting", "zsh"),
                ),
            ]
            .into(),
            unbound: vec![
                (SwarmId::from_u128(1), bbt_mux::Ulid(91)),
                (SwarmId::from_u128(2), bbt_mux::Ulid(92)),
            ],
        });
        let layouts = scene.layout_at([1600., 960.], false, at).unwrap();
        for (index, own, other) in [
            (0, ["a0", "a1"], ["b0", "b1"]),
            (1, ["b0", "b1"], ["a0", "a1"]),
        ] {
            let card = named(&layouts[1], &format!("swarm{index}/card")).unwrap();
            let mut labels = String::new();
            text(card, &mut labels);
            assert!(
                own.iter().all(|name| labels.contains(name)),
                "missing own rows: {labels}"
            );
            assert!(
                other.iter().all(|name| !labels.contains(name)),
                "foreign rows: {labels}"
            );
            assert!(labels.contains(if index == 0 { "a-closed" } else { "b-closed" }));
            assert!(labels.contains(if index == 0 {
                "a-starting"
            } else {
                "b-starting"
            }));
            assert!(!labels.contains(if index == 0 { "b-closed" } else { "a-closed" }));
            assert!(!labels.contains(if index == 0 {
                "b-starting"
            } else {
                "a-starting"
            }));
        }
        assert_eq!(
            scene
                .pane_bounds(&layouts)
                .iter()
                .map(|(id, _)| *id)
                .collect::<Vec<_>>(),
            ids[..2]
        );
        assert_eq!(scene.focused(), Some(ids[0]));
    }
    // keeps: spec ux-prototype §5
    #[test]
    fn local_graph_uses_a_clipped_retained_viewport_and_keeps_the_terminal_stage() {
        let scene = Scene::new();
        let focus = scene.focused();
        let chrome = scene.local_page_chrome([1500., 900.], 240.).unwrap();
        assert_eq!(chrome.len(), 2);
        assert!(scene.pane_bounds(&chrome).is_empty());
        let graph = crate::e2e_checklist::Graph::new(vec![], vec![]).unwrap();
        let page = scene
            .graph_page_layout([1500., 900.], 240., &graph.checklist, 100.)
            .unwrap();
        assert_eq!(page.viewport.x, scene.sidebar_width);
        assert_eq!(page.viewport.y, STYLE.spacing.title_bar as f32 + 44.);
        assert_eq!(page.viewport.y + page.viewport.h, 900. - 240.);
        assert!(page.max_scroll > 0.);
        assert!(page.contains([page.viewport.x + 10., page.viewport.y + 10.]));
        assert!(page.closes([
            page.close.x + page.close.w * 0.5,
            page.close.y + page.close.h * 0.5
        ]));
        assert!(!page.closes([10., 10.]));
        assert_eq!(scene.focused(), focus);
        assert!(!page.paint().is_empty());
    }

    // keeps: spec ux-prototype §5
    #[test]
    fn historical_gap_uses_cell_device_origin_and_last_row_scissor_at_both_scales() {
        let body = Rect::new(10.3, 20.3, 100.8, 33.8);
        assert_eq!(
            history_row_rect(body, [100, 36], 3, 1, 1.),
            Some(Rect::new(10., 32., 100., 12.))
        );
        assert_eq!(
            history_row_rect(body, [200, 72], 3, 1, 2.),
            Some(Rect::new(10.5, 32.5, 100., 12.))
        );
        assert_eq!(
            history_row_rect(body, [200, 72], 3, 2, 2.),
            Some(Rect::new(10.5, 44.5, 100., 9.5))
        );
        assert!(history_row_rect(body, [200, 72], 3, 3, 2.).is_none());
    }

    /// Pane rects (layouts after the title bar and the sidebar).
    fn panes(scene: &Scene) -> Vec<Rect> {
        scene.layout([1600., 960.], true).unwrap()[2..]
            .iter()
            .map(|l| l.rect)
            .collect()
    }

    // keeps: spec ux-prototype §5
    #[test]
    fn the_rim_size_moves_the_gutter_and_inset_and_never_the_sidebar() {
        let mut scene = Scene::new();
        let sidebar_edge = scene.sidebar_width;
        let chrome = |scene: &Scene| {
            scene.layout([1600., 960.], true).unwrap()[..2]
                .iter()
                .map(|l| l.rect)
                .collect::<Vec<_>>()
        };
        let reference = chrome(&scene);
        // rim-glow §6.5: rim 2, 4, 8 give gutter 2, 4, 8 and inset 1, 2, 4.
        for (rim, gutter, inset) in [(2., 2., 1.), (4., 4., 2.), (8., 8., 4.)] {
            scene.set_rim(rim);
            let rects = panes(&scene);
            let left = rects.iter().map(|r| r.x).fold(f32::MAX, f32::min);
            assert!(
                (left - (sidebar_edge + inset)).abs() < 0.01,
                "rim {rim}: inset {}",
                left - sidebar_edge
            );
            let right = rects.iter().map(|r| r.right()).fold(f32::MIN, f32::max);
            assert!(
                (1600. - right - inset).abs() < 0.01,
                "rim {rim}: right inset {}",
                1600. - right
            );
            let gaps: Vec<f32> = rects
                .iter()
                .flat_map(|a| rects.iter().map(move |b| (a, b)))
                .filter(|(a, b)| a.y < b.bottom() && b.y < a.bottom() && b.x > a.right() - 0.01)
                .map(|(a, b)| b.x - a.right())
                .collect();
            assert!(
                !gaps.is_empty() && gaps.iter().any(|g| (g - gutter).abs() < 0.01),
                "rim {rim}: gaps {gaps:?}"
            );
            assert!(
                gaps.iter().all(|g| *g >= gutter - 0.01),
                "rim {rim}: a gap under the gutter: {gaps:?}"
            );
            assert_eq!(
                chrome(&scene),
                reference,
                "rim {rim} moved the title bar or the sidebar"
            );
        }
        scene.set_rim(RIM_MAX);
        let maximum = panes(&scene);
        scene.set_rim(20.);
        assert_eq!(panes(&scene), maximum);
        scene.set_rim(RIM_MIN);
        let minimum = panes(&scene);
        scene.set_rim(0.5);
        assert_eq!(panes(&scene), minimum);
    }

    /// Every rim the scene paints for one seat, its row (phase from the key) and its pane
    /// (seeded by the seat id), as (swell, attention).
    pub(super) fn rims_of(scene: &Scene, seat: SeatId, now: Instant) -> Vec<(f32, bool)> {
        [LocateWhere::Row, LocateWhere::Pane]
            .into_iter()
            .flat_map(|surface| rims_on(scene, seat, now, surface))
            .collect()
    }
    pub(super) fn rims_on(
        scene: &Scene,
        seat: SeatId,
        now: Instant,
        surface: LocateWhere,
    ) -> Vec<(f32, bool)> {
        let key = &scene.seats[scene.seat_index[&seat]].key;
        let ctx = ui::Context::new(ui::Theme::Dark, ui::Density::Cozy);
        let mut out = vec![];
        for (index, layout) in scene
            .layout_at([1600., 960.], true, now)
            .unwrap()
            .into_iter()
            .enumerate()
        {
            if (surface == LocateWhere::Row && index != 1)
                || (surface == LocateWhere::Pane && index < 2)
            {
                continue;
            }
            for op in c::paint(&layout, ctx) {
                if let c::Paint::Material {
                    kind: ui::Material::Rim,
                    rim: Some(r),
                    ..
                } = op
                    && (r.seed == ui::rim_phase(key) || r.seed == seat.0.0 as u32)
                {
                    out.push((r.swell, r.attention));
                }
            }
        }
        out
    }

    // keeps: spec ux-prototype §5
    #[test]
    fn the_focus_swell_rises_and_settles_over_swell_ms_and_keeps_frames_until_then() {
        let mut scene = Scene::new();
        let t0 = Instant::now();
        let ms = |n: u64| t0 + std::time::Duration::from_millis(n);
        let seat = SeatId::from_u128(3);
        assert!(!scene.animating(t0));
        assert!(rims_of(&scene, seat, t0).iter().all(|r| *r == (0., false)));
        scene.swell(
            seat,
            FocusSwell {
                at: t0,
                where_: LocateWhere::Pane,
            },
        );
        let peak = rims_on(
            &scene,
            seat,
            ms(TOKENS.glow.swell_ms as u64 / 2),
            LocateWhere::Pane,
        );
        assert!(!peak.is_empty(), "pane must swell: {peak:?}");
        assert!(
            peak.iter().all(|(e, a)| (*e - 1.).abs() < 1e-3 && !a),
            "{peak:?}"
        );
        assert!(scene.animating(ms(100)));
        let done = ms(TOKENS.glow.swell_ms as u64 + 1);
        assert!(!scene.animating(done));
        assert!(
            rims_of(&scene, seat, done)
                .iter()
                .all(|r| *r == (0., false))
        );
        // Another seat never swells with it.
        assert!(
            rims_of(&scene, SeatId::from_u128(4), ms(450))
                .iter()
                .all(|r| r.0 == 0.)
        );
    }

    // keeps: spec ux-prototype §5
    #[test]
    fn app_clock_steps_the_actual_focus_rims_to_450_ms_without_moving_layout() {
        use bbt_proto::gui::UiClockCommand;
        let mut clock = crate::animation_clock::Clock::default();
        let frozen = clock.apply(UiClockCommand::Freeze).unwrap();
        let mut scene = Scene::new();
        let seat = SeatId::from_u128(3);
        scene.swell(
            seat,
            FocusSwell {
                at: frozen.now,
                where_: LocateWhere::Pane,
            },
        );
        let bounds = |now| {
            scene
                .layout_at([1600., 960.], true, now)
                .unwrap()
                .into_iter()
                .map(|layout| layout.rect)
                .collect::<Vec<_>>()
        };
        let before = bounds(frozen.now);
        assert!(
            rims_of(&scene, seat, frozen.now)
                .iter()
                .all(|rim| rim.0 == 0.)
        );
        let peak = clock.apply(UiClockCommand::Advance { ms: 450 }).unwrap();
        let rims = rims_on(&scene, seat, peak.now, LocateWhere::Pane);
        assert!(!rims.is_empty(), "pane must carry the swell");
        assert!(
            rims.iter()
                .all(|(envelope, attention)| (*envelope - 1.).abs() < 1e-6 && !attention)
        );
        assert_eq!(bounds(peak.now), before);
        assert!(
            rims_on(&scene, seat, peak.now, LocateWhere::Row)
                .iter()
                .all(|rim| *rim == (0., false))
        );
        let settled = clock.apply(UiClockCommand::Advance { ms: 450 }).unwrap();
        assert!(
            rims_of(&scene, seat, settled.now)
                .iter()
                .all(|rim| rim.0.abs() < 1e-6)
        );
    }

    #[test]
    fn focus_swell_targets_one_surface_and_replaces_rapid_reversals() {
        let mut scene = Scene::new();
        let seat = SeatId::from_u128(3);
        let other = SeatId::from_u128(4);
        let at = Instant::now();
        let peak = at + std::time::Duration::from_millis(450);
        for target in [LocateWhere::Pane, LocateWhere::Row, LocateWhere::Pane] {
            scene.swell(seat, FocusSwell { at, where_: target });
            for surface in [LocateWhere::Row, LocateWhere::Pane] {
                let rims = rims_on(&scene, seat, peak, surface);
                assert!(!rims.is_empty());
                let expected = if surface == target { 1. } else { 0. };
                assert!(
                    rims.iter()
                        .all(|(e, attention)| !attention && (*e - expected).abs() < 1e-3),
                    "{target:?} on {surface:?}: {rims:?}"
                );
            }
        }
        scene.swell(
            other,
            FocusSwell {
                at,
                where_: LocateWhere::Row,
            },
        );
        assert!(
            rims_of(&scene, seat, peak)
                .iter()
                .all(|rim| *rim == (0., false)),
            "a newer seat focus cancels the old one-shot cue"
        );
        scene.set_attention(seat, Some((at, false)));
        assert!(
            rims_of(&scene, seat, peak)
                .iter()
                .all(|(e, attention)| *attention && (*e - 1.).abs() < 1e-3),
            "waiting attention still pulses both surfaces"
        );
    }

    // keeps: spec ux-prototype §5
    #[test]
    fn controlled_attention_keeps_its_logical_phase_across_metadata_publications() {
        use bbt_proto::gui::UiClockCommand;
        let record =
            bbt_proto::SeatRecord::new(bbt_mux::Ulid(3), bbt_mux::Ulid::new(), "Clock", "zsh");
        let seat = SeatId::from_u128(record.id.0);
        let mut model = bboxy450_term::model::build(
            &[record],
            &bboxy450_term::grids::Grids::new(),
            &Default::default(),
        );
        let mut clock = crate::animation_clock::Clock::default();
        clock.apply(UiClockCommand::Freeze).unwrap();
        clock.apply(UiClockCommand::Advance { ms: 10_000 }).unwrap();
        let source = Instant::now();
        model.attention.insert(
            seat,
            bboxy450_term::attention::Attention {
                since: source,
                muted: false,
            },
        );
        let mut scene = Scene::live();
        scene.set_model_with_clock(&model, &model.ui.window.stage, &clock);
        assert_eq!(
            scene.envelope(seat, clock.now(), LocateWhere::Pane),
            (0., true)
        );
        clock.apply(UiClockCommand::Advance { ms: 450 }).unwrap();
        for _ in 0..3 {
            scene.set_model_with_clock(&model, &model.ui.window.stage, &clock);
            let (phase, attention) = scene.envelope(seat, clock.now(), LocateWhere::Pane);
            assert!((phase - 1.).abs() < 1e-6 && attention);
            assert_eq!(model.attention[&seat].since, source);
        }
        model.attention.get_mut(&seat).unwrap().muted = true;
        scene.set_model_with_clock(&model, &model.ui.window.stage, &clock);
        assert_eq!(
            scene.envelope(seat, clock.now(), LocateWhere::Pane),
            (0., false)
        );
        model.attention.clear();
        scene.set_model_with_clock(&model, &model.ui.window.stage, &clock);
        assert!(
            rims_of(&scene, seat, clock.now())
                .iter()
                .all(|r| *r == (0., false))
        );
    }

    // keeps: spec ux-prototype §5
    #[test]
    fn attention_loops_until_handled_and_a_muted_seat_stays_quiet() {
        let mut scene = Scene::new();
        let t0 = Instant::now();
        let seat = SeatId::from_u128(5);
        scene.set_attention(seat, Some((t0, false)));
        // |sin(pi t / swell_ms)|: a peak every swell_ms, forever, and frames keep coming.
        for n in 0..4 {
            let at = t0 + std::time::Duration::from_millis(TOKENS.glow.swell_ms as u64 * n + 450);
            assert!(
                rims_of(&scene, seat, at)
                    .iter()
                    .all(|(e, a)| (*e - 1.).abs() < 1e-3 && *a)
            );
            assert!(scene.animating(at));
        }
        scene.set_attention(seat, Some((t0, true)));
        assert!(!scene.animating(t0));
        assert!(
            rims_of(&scene, seat, t0 + std::time::Duration::from_millis(450))
                .iter()
                .all(|r| *r == (0., false))
        );
        scene.set_attention(seat, None);
        assert!(!scene.animating(t0));
    }

    // keeps: spec ux-prototype §5
    #[test]
    fn published_attention_drives_both_blocks_without_moving_focus_or_restarting_swell() {
        use bboxy450_term::{grids::Grids, model};
        use bbt_mux::Ulid;
        use bbt_proto::{Grade, GuiSeat, ProcessStatus, SeatState, registry::SeatRecord};

        let seat = SeatId::from_u128(3);
        let mut current = GuiSeat {
            notification_episode: None,
            attention: None,
            record: SeatRecord::new(Ulid(3), Ulid(2), "fixture", "zsh"),
            state: SeatState {
                generation: 1,
                process: ProcessStatus::Running { pid: 9 },
                activity: bbt_proto::Activity::Waiting(bbt_proto::WaitKind::Question),
                grade: Grade::Shim,
                title: String::new(),
                detail: Some("Your answer is needed".into()),
                turn_open: true,
                observed_ms: 10,
                subagents: vec![],
            },
        };
        let mut model = model::build(
            std::slice::from_ref(&current.record),
            &Grids::new(),
            &model::Model::default(),
        );
        let stage = model.ui.window.stage.clone();
        let mut scene = Scene::live();
        scene.set_model(&model, &stage);
        let rects = panes(&scene);
        model.apply_seat_states(std::slice::from_ref(&current));
        let since = model.attention[&seat].since;
        scene.set_model(&model, &stage);
        let peak = since + std::time::Duration::from_millis(TOKENS.glow.swell_ms as u64 * 4 + 450);
        let rims = rims_of(&scene, seat, peak);
        assert!(rims.len() >= 2, "row and pane must loop: {rims:?}");
        assert!(rims.iter().all(|(e, a)| (*e - 1.).abs() < 1e-3 && *a));
        assert!(scene.animating(peak));
        assert_eq!(scene.focused(), stage.focused);
        assert_eq!(panes(&scene), rects);

        current.record.muted = true;
        model.apply_seat_states(std::slice::from_ref(&current));
        scene.set_model(&model, &stage);
        assert!(!scene.animating(peak));
        assert!(
            rims_of(&scene, seat, peak)
                .iter()
                .all(|r| *r == (0., false))
        );

        let key = model.attention_key(seat);
        assert!(model.attention_focus(
            seat,
            1,
            key,
            Some(FocusSwell {
                at: peak,
                where_: LocateWhere::Pane
            })
        ));
        scene.set_model(&model, &stage);
        assert!(
            rims_of(&scene, seat, peak)
                .iter()
                .all(|(_, attention)| !attention)
        );
        assert!(scene.animating(peak));
        let done = peak + std::time::Duration::from_millis(TOKENS.glow.swell_ms as u64 + 1);
        scene.set_model(&model, &stage);
        assert!(
            !scene.animating(done),
            "a repeated publication must keep the original start"
        );
        assert!(
            rims_of(&scene, seat, done)
                .iter()
                .all(|r| *r == (0., false))
        );
        assert_eq!(scene.focused(), stage.focused);
        assert_eq!(panes(&scene), rects);

        current.state.activity = bbt_proto::Activity::Idle;
        model.apply_seat_states(&[current]);
        scene.set_model(&model, &stage);
        assert!(
            rims_of(&scene, seat, peak)
                .iter()
                .all(|(_, attention)| !attention)
        );
    }

    // keeps: spec ux-prototype §5
    #[test]
    fn restart_notice_is_one_shot_on_generation_change_and_then_one_stream_line() {
        use bboxy450_term::{grids::Grids, model};
        use bbt_mux::Ulid;
        use bbt_proto::registry::SeatRecord;
        use std::time::Duration;
        let mut record = SeatRecord::new(Ulid(3), Ulid(2), "fixture", "zsh");
        let mut model = model::build(
            std::slice::from_ref(&record),
            &Grids::new(),
            &model::Model::default(),
        );
        let mut scene = Scene::live();
        let start = Instant::now();
        scene.set_model_at(&model, &model.ui.window.stage, Some(start));
        assert!(
            scene.restart_generations[&SeatId::from_u128(3)].1.is_none(),
            "initial metadata is not a restart"
        );
        record.generation += 1;
        model = model::build(std::slice::from_ref(&record), &Grids::new(), &model);
        scene.set_model_at(&model, &model.ui.window.stage, Some(start));
        let at = |ms| start + Duration::from_millis(ms);
        assert!(scene.animating(at(3_999)));
        assert!(!scene.advance_restarts(at(3_999)));
        scene.set_model_at(&model, &model.ui.window.stage, Some(at(3_999)));
        assert_eq!(
            scene.restart_generations[&SeatId::from_u128(3)].1,
            Some(start),
            "reprojection restarted the timer"
        );
        assert_eq!(restarted_alpha(3.4), 1.);
        assert!((restarted_alpha(3.7) - 0.053921).abs() < 0.001);
        assert!(scene.advance_restarts(at(4_000)));
        assert!(!scene.advance_restarts(at(8_000)), "notice emitted twice");
        assert_eq!(
            scene
                .restart_lines(bbt_ui_model::SwarmId::from_u128(2))
                .collect::<Vec<_>>(),
            ["fixture restarted · process generation 2"]
        );
        assert!(!scene.animating(at(8_000)));
    }

    // keeps: spec ux-prototype §5
    #[test]
    fn pane_rects_are_identical_across_activity_states() {
        let mut scene = Scene::new();
        let before = panes(&scene);
        for sequence in 0..4 {
            scene.update(sequence);
            assert_eq!(panes(&scene), before, "an activity change moved a pane");
        }
    }
}

#[cfg(test)]
mod launcher_tests {
    use super::*;

    // keeps: operator ROLE-first identity uses the assigned role on every surface.
    #[test]
    fn live_seat_projection_preserves_assigned_shell_role_for_every_harness() {
        for harness in ["claude", "codex", "zsh"] {
            let id = bbt_mux::Ulid(1);
            let mut record =
                bbt_proto::SeatRecord::new(id, bbt_mux::Ulid(100), "Vesper Greywind", harness);
            record.role.name = "shell".into();
            let model = bboxy450_term::model::build_at(
                std::slice::from_ref(&record),
                &bboxy450_term::grids::Grids::new(),
                &Default::default(),
                Instant::now(),
                1000,
            );
            let mut scene = Scene::live();
            scene.set_model(&model, &model.ui.window.stage);
            let row = &scene.seats[scene.seat_index[&SeatId::from_u128(id.0)]];
            assert_eq!(row.role.as_deref(), Some("shell"), "{harness}");
            assert_eq!(
                bbt_ui::modules::seat_row::role_label(row.role.as_deref(), &row.harness),
                "SHELL"
            );
            assert_eq!(row.name, record.name);
            assert_eq!(row.key, id.to_string());
        }
    }

    #[test]
    fn launch_parameters_wire_round_trips_the_native_retained_layout() {
        let scene = Scene::new();
        let mut controller = crate::new_launcher(None, 0);
        controller.launcher.items[0].children = None;
        let fields = bboxy450_term::spawn_params::resolve(
            "claude",
            &Default::default(),
            None,
            "/var/folders/zd/mv_mr2d92p5_1r8yghzbs5vw0000gn/T/.tmppAx8ea/project with spaces",
        );
        assert!(controller.open_parameters("claude", fields));
        controller.launcher.params.as_mut().unwrap().runs="cd '/var/folders/zd/mv_mr2d92p5_1r8yghzbs5vw0000gn/T/.tmppAx8ea/project with spaces' && claude".into();
        let solved = scene
            .launcher_layout([1280., 720.], &controller.launcher)
            .unwrap();
        let sub = named(solved.solved(), "picker/div/opt1/sub").unwrap().rect;
        let source = named(solved.solved(), "picker/div/opt1/kind").unwrap().rect;
        assert!(
            sub.right() <= source.x,
            "long directories must not overlap their source: {sub:?} {source:?}"
        );
        let node = crate::ui_layout_node(solved.solved(), &mut 4096, &mut (16 * 1024)).unwrap();
        let layout: bbt_proto::gui::UiLayout = serde_json::from_value(serde_json::json!({
            "frame_seq":1,"size":[1280,720],"swarm":null,"tab":null,"focused":null,
            "launcher_open":true,"page":null,"document":null,"panes":[],"tree":[node]
        }))
        .unwrap();
        let request = bbt_proto::gui::GuiRequest {
            version: bbt_proto::gui::VERSION,
            capability: [0; 32],
            action: bbt_proto::gui::GuiAction::UiReply {
                id: bbt_mux::Ulid::new(),
                result: Box::new(Ok(layout)),
            },
        };
        let encoded = serde_json::to_vec(&request).unwrap();
        serde_json::from_slice::<bbt_proto::gui::GuiRequest>(&encoded)
            .expect("native parameter layout must fit the admitted frame decoder");
    }

    // keeps: spec ux-prototype §5
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
        controller.handle(Command::SetCaret {
            at: 0,
            extend: true,
        });
        let selected = scene
            .launcher_layout([1600., 960.], &controller.launcher)
            .unwrap();
        assert_eq!(
            (layout.panel, layout.query, &layout.rows),
            (selected.panel, selected.query, &selected.rows),
            "pointer-selection hit regions survive until the next paint"
        );
    }

    // keeps: spec ux-prototype §5
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

    // keeps: spec ux-prototype §5
    #[test]
    fn assigned_task_uses_its_seat_subtree_and_retained_review_hit_without_moving_panes() {
        let record = bbt_proto::SeatRecord::new(
            bbt_mux::Ulid::new(),
            bbt_mux::Ulid::new(),
            "Assigned",
            "zsh",
        );
        let id = SeatId::from_u128(record.id.0);
        let model = bboxy450_term::model::build(
            &[record],
            &bboxy450_term::grids::Grids::new(),
            &Default::default(),
        );
        let mut scene = Scene::live();
        scene.set_model(&model, &model.ui.window.stage);
        let size = [1600., 960.];
        let before = scene.pane_bounds(&scene.layout(size, false).unwrap());
        scene.set_tasks(vec![(
            id,
            "real-task-id".into(),
            ui::modules::seat_row::TaskRow {
                title: "Reported work".into(),
                state: "reported".into(),
                bug: false,
                violates: None,
                constrains: None,
                waits: None,
                may_accept: true,
            },
            Some(0),
        )]);
        let layouts = scene.layout(size, false).unwrap();
        assert_eq!(before, scene.pane_bounds(&layouts));
        fn accept(n: &c::Layout) -> Option<Rect> {
            if matches!(&n.kind, c::LayoutKind::Named(name) if name.ends_with("task/real-task-id/right/accept"))
            {
                Some(n.rect)
            } else {
                n.children.iter().find_map(accept)
            }
        }
        let r = accept(&layouts[1]).expect("task under its assignee");
        assert_eq!(
            scene
                .task_regions(&layouts)
                .hit([r.x + r.w * 0.5, r.y + r.h * 0.5]),
            Some(ui::modules::stream_dock::Action::Accept(
                "real-task-id".into()
            ))
        );
    }

    // keeps: spec ux-prototype §5
    #[test]
    fn dock_precedes_retained_rims_and_launcher_document_and_hud_stay_above() {
        let scene = Scene::new();
        let size = [1600., 960.];
        let conversation = scene
            .conversation_layout(size, &ui::modules::composer::Conversation::default())
            .unwrap();
        let layouts = scene
            .layout_with_dock(size, true, conversation.height)
            .unwrap();
        let (_, rims) = c::split_rims(scene.paint(&layouts, size, None, None));
        assert!(!rims.is_empty());
        let launcher = scene
            .launcher_layout(size, &crate::new_launcher(None, 0).launcher)
            .unwrap();
        let mut host = bboxy450_term::documents::Host::default();
        host.show("#44".into());
        let document = scene.document_layout(size, &host).unwrap();
        let hud = crate::perf_hud(
            bbt_rt::HitchMeter::new(120.).perf_record(std::time::Instant::now(), 0.),
        );
        let expected = [
            conversation.paint(),
            rims.clone(),
            launcher.paint(),
            document.paint(),
            scene.overlay(size, Some(&hud)),
        ];
        let actual = scene.foreground_with_document(
            size,
            Some(&launcher),
            Some(&hud),
            Some(&conversation),
            Some(document.paint()),
            rims,
        );
        let mut start = 0;
        for layer in expected {
            let end = start + layer.len();
            assert_eq!(format!("{:?}", &actual[start..end]), format!("{layer:?}"));
            start = end;
        }
        assert_eq!(actual.len(), start);
    }

    // keeps: spec ux-prototype §5
    #[test]
    fn document_uses_effective_context_without_moving_panes_and_keeps_hud_last() {
        let mut scene = Scene::new();
        let mut preferences = bboxy450_term::settings::Preferences::default();
        preferences.appearance.theme = "light".into();
        preferences.appearance.density = "compact".into();
        scene.set_appearance(&preferences, ui::Theme::Dark);
        let size = [1600., 960.];
        let panes = scene.pane_bounds(&scene.layout(size, false).unwrap());
        let mut host = bboxy450_term::documents::Host::default();
        host.show("#44".into());
        let document = scene.document_layout(size, &host).unwrap();
        assert_eq!(document.context.theme, ui::Theme::Light);
        assert_eq!(document.context.density, ui::Density::Compact);
        assert!(document.matches(size, scene.ui_context()));
        let hud = crate::perf_hud(
            bbt_rt::HitchMeter::new(120.).perf_record(std::time::Instant::now(), 0.),
        );
        let document_ops = document.paint();
        let hud_ops = scene.overlay(size, Some(&hud));
        let foreground = scene.foreground_with_document(
            size,
            None,
            Some(&hud),
            None,
            Some(document.paint()),
            vec![],
        );
        assert_eq!(foreground.len(), document_ops.len() + hud_ops.len());
        assert_eq!(
            format!("{:?}", &foreground[..document_ops.len()]),
            format!("{document_ops:?}")
        );
        assert_eq!(
            format!("{:?}", &foreground[document_ops.len()..]),
            format!("{hud_ops:?}")
        );
        assert_eq!(
            panes,
            scene.pane_bounds(&scene.layout(size, false).unwrap())
        );
    }

    // keeps: spec ux-prototype §5
    #[test]
    fn repeated_document_publications_keep_the_original_focus_swell_instant() {
        let record =
            bbt_proto::SeatRecord::new(bbt_mux::Ulid(3), bbt_mux::Ulid::new(), "Anchor", "zsh");
        let seat = SeatId::from_u128(record.id.0);
        let generation = record.generation;
        let mut model = bboxy450_term::model::build(
            &[record],
            &bboxy450_term::grids::Grids::new(),
            &Default::default(),
        );
        let landed = Instant::now();
        assert!(model.attention_focus(
            seat,
            generation,
            None,
            Some(FocusSwell {
                at: landed,
                where_: LocateWhere::Pane
            })
        ));
        let mut scene = Scene::live();
        for _ in 0..3 {
            model.document_reply = Some(std::sync::Arc::new(bboxy450_term::documents::Reply {
                id: bbt_mux::Ulid::new(),
                response: bboxy450_term::documents::Response::Failed("not_found: retained".into()),
            }));
            scene.set_model(&model, &model.ui.window.stage);
            let peak = landed + std::time::Duration::from_millis(450);
            let rims = depth_tests::rims_on(&scene, seat, peak, LocateWhere::Pane);
            assert!(!rims.is_empty());
            assert!(
                rims.iter()
                    .all(|(phase, attention)| (*phase - 1.).abs() < 1e-6 && !attention)
            );
            let done = landed + std::time::Duration::from_millis(TOKENS.glow.swell_ms as u64 + 1);
            assert!(
                depth_tests::rims_of(&scene, seat, done)
                    .iter()
                    .all(|r| *r == (0., false))
            );
        }
    }

    // keeps: spec ux-prototype §5
    #[test]
    fn settings_owns_one_inner_scroller_and_uses_the_effective_theme_density_and_rim() {
        let mut scene = Scene::new();
        let mut preferences = bboxy450_term::settings::Preferences::default();
        preferences.appearance.theme = "light".into();
        preferences.appearance.density = "compact".into();
        preferences.appearance.rim = 8.;
        scene.set_appearance(&preferences, ui::Theme::Dark);
        let view = ui::modules::settings::Settings {
            section: "Appearance".into(),
            values: bboxy450_term::settings::records(None, &preferences),
            defaults: bboxy450_term::settings::records(None, &Default::default()),
            ..Default::default()
        };
        let controller = ui::modules::settings::controller::Controller::new("Appearance");
        let page = scene
            .settings_page_layout([1600., 960.], &view, &controller.presentation(), "saved")
            .unwrap();
        fn count(node: &c::Layout) -> usize {
            usize::from(matches!(node.kind, c::LayoutKind::ScrollView { .. }))
                + node.children.iter().map(count).sum::<usize>()
        }
        assert_eq!(count(&page.layout), 1);
        assert_eq!(page.context.theme, ui::Theme::Light);
        assert_eq!(page.context.density, ui::Density::Compact);
        let pane_layouts = scene.layout([1600., 960.], true).unwrap();
        let left = pane_layouts[2..]
            .iter()
            .map(|pane| pane.rect.x)
            .fold(f32::MAX, f32::min);
        assert!(
            (left - (scene.sidebar_width + 4.)).abs() < 0.01,
            "saved rim must produce its 4 px stage inset"
        );
        assert!((page.viewport.bottom() - 960.).abs() < 0.01);
        assert!(page.settings.as_ref().unwrap().rim.is_some());
        let mut text = String::new();
        fn labels(node: &c::Layout, text: &mut String) {
            if let c::LayoutKind::Text { runs, .. } = &node.kind {
                for run in runs {
                    text.push_str(&run.text);
                }
            }
            for child in &node.children {
                labels(child, text);
            }
        }
        labels(&page.layout, &mut text);
        assert!(text.contains("not yet applied to the window"));
    }

    // keeps: spec ux-prototype §5 — sidebar chat never reduces Settings height.
    #[test]
    fn settings_fills_the_stage_while_sidebar_chat_keeps_its_own_height() {
        let scene = Scene::new();
        let preferences = bboxy450_term::settings::Preferences::default();
        let view = ui::modules::settings::Settings {
            section: "Roles".into(),
            values: bboxy450_term::settings::records(None, &preferences),
            defaults: bboxy450_term::settings::records(None, &preferences),
            ..Default::default()
        };
        let controller = ui::modules::settings::controller::Controller::new("Roles");
        let mut previous_scroll = f32::INFINITY;
        for height in [480., 720., 960., 1200.] {
            let size = [1280., height];
            let conversation = scene
                .conversation_layout(size, &Default::default())
                .unwrap();
            assert!(conversation.height > 0. && conversation.height < height);
            let sidebar_height = scene.local_page_chrome(size, 0.).unwrap()[1].rect.h;
            for dock in [0., 180., conversation.height] {
                let chrome = scene.local_page_chrome(size, dock).unwrap();
                assert!((sidebar_height - chrome[1].rect.h - dock).abs() < 0.01);
                let page = scene
                    .settings_page_layout(size, &view, &controller.presentation(), "saved")
                    .unwrap();
                assert!((page.layout.rect.bottom() - height).abs() < 0.01);
                assert!((page.viewport.bottom() - height).abs() < 0.01);
                assert!((named(&page.layout, "side").unwrap().rect.bottom() - height).abs() < 0.01);
                assert!(page.max_scroll >= 0. && page.max_scroll <= previous_scroll);
                previous_scroll = page.max_scroll;
            }
        }
    }
    // keeps: spec ux-prototype §5
    #[test]
    fn t7_close_and_undo_hits_are_the_actual_retained_buttons() {
        let mut records: Vec<_> = (1..=3)
            .map(|id| {
                bbt_proto::SeatRecord::new(bbt_mux::Ulid(id), bbt_mux::Ulid(4), "fixture", "zsh")
            })
            .collect();
        let grids = bboxy450_term::grids::Grids::new();
        let now = Instant::now();
        let mut model =
            bboxy450_term::model::build_at(&records, &grids, &Default::default(), now, 1_000);
        let victim = SeatId::from_u128(3);
        let mut scene = Scene::live();
        scene.set_model(&model, &model.ui.window.stage);
        let layouts = scene.layout([1000., 640.], false).unwrap();
        let hits = scene.close_regions(&layouts);
        assert_eq!(hits.iter().filter(|(_, _, undo)| !*undo).count(), 3);
        assert!(hits.iter().all(|(_, r, _)| r.w > 0. && r.h > 0.));
        let intent = scene
            .close_intent(&model, victim, bbt_mux::Ulid(9), now)
            .unwrap();
        records[2].lifecycle = bbt_proto::registry::Lifecycle::ClosedGrace;
        records[2].close = Some(bbt_proto::registry::CloseRecord {
            operation: intent.operation,
            revision: 1,
            deadline_ms: 61_000,
            position: intent.position.clone(),
        });
        model.closes.register(intent).unwrap();
        let model = bboxy450_term::model::build_at(&records, &grids, &model, now, 1_000);
        scene.set_model(&model, &model.ui.window.stage);
        let layouts = scene.layout([1000., 640.], false).unwrap();
        let hits = scene.close_regions(&layouts);
        assert_eq!(hits.iter().filter(|(_, _, undo)| !*undo).count(), 2);
        let undos: Vec<_> = hits.iter().filter(|(_, _, undo)| *undo).collect();
        assert_eq!(undos.len(), 1);
        assert_eq!(undos[0].0, victim);
    }

    #[test]
    fn swarm_workdir_buttons_use_retained_nonoverlapping_hit_regions() {
        let records = vec![bbt_proto::SeatRecord::new(
            bbt_mux::Ulid(1),
            bbt_mux::Ulid(2),
            "fixture",
            "zsh",
        )];
        let mut model = bboxy450_term::model::build_at(
            &records,
            &bboxy450_term::grids::Grids::new(),
            &Default::default(),
            Instant::now(),
            1000,
        );
        let mut scene = Scene::live();
        scene.set_model(&model, &model.ui.window.stage);
        let layouts = scene.layout([1000., 640.], false).unwrap();
        let hits = scene.swarm_folder_regions(&layouts);
        assert_eq!(hits.len(), 1);
        assert!(!hits[0].2);
        assert!(hits[0].1.w > 0. && hits[0].1.h > 0.);
        model.ui.window.sidebar.swarms[0].workdir =
            Some("/a long project folder with spaces".into());
        scene.set_model(&model, &model.ui.window.stage);
        let layouts = scene.layout([1000., 640.], false).unwrap();
        let hits = scene.swarm_folder_regions(&layouts);
        assert_eq!(hits.len(), 2);
        assert!(!hits[0].2 && hits[1].2);
        assert!(hits.iter().all(|(_, rect, _)| rect.w > 0. && rect.h > 0.));
        assert!(hits[0].1.right() <= hits[1].1.x);
    }

    #[test]
    fn sidebar_width_drives_stage_and_grab_region_and_survives_narrow_layout() {
        let mut scene = Scene::new();
        for width in [160., 300., 420., 520.] {
            scene.sidebar_width = width;
            let layouts = scene.layout([1000., 700.], false).unwrap();
            assert_eq!(layouts[1].rect.w, width);
            let handle = scene
                .sidebar_resize_region(&layouts, [1000., 700.])
                .unwrap();
            assert_eq!(handle.x + handle.w / 2., width);
            assert_eq!(handle.w, 8.);
            assert_eq!(handle.bottom(), 700.);
            assert!(
                scene
                    .pane_bounds(&layouts)
                    .iter()
                    .all(|(_, rect)| rect.x >= width)
            );
        }
        scene.sidebar_width = 520.;
        assert_eq!(scene.sidebar_width_for([320., 700.]), 160.);
        let narrow = scene.layout([320., 700.], false).unwrap();
        assert!(
            scene
                .pane_bounds(&narrow)
                .iter()
                .all(|(_, r)| r.x >= 160. && r.right() <= 320.)
        );
        assert_eq!(
            scene.sidebar_width, 520.,
            "window constraints do not overwrite the saved preference"
        );
        assert_eq!(scene.sidebar_width_for([1000., 700.]), 520.);
    }

    #[test]
    fn selected_swarm_and_active_seat_control_exactly_one_dim_even_when_empty() {
        let records: Vec<_> = [(1, 100), (2, 100), (3, 200)]
            .into_iter()
            .map(|(seat, swarm)| {
                bbt_proto::SeatRecord::new(bbt_mux::Ulid(seat), bbt_mux::Ulid(swarm), "seat", "zsh")
            })
            .collect();
        let mut model = bboxy450_term::model::build_at(
            &records,
            &bboxy450_term::grids::Grids::new(),
            &Default::default(),
            Instant::now(),
            1000,
        );
        let mut stage = model.ui.window.stage.clone();
        stage.swarm = Some(bbt_ui_model::SwarmId::from_u128(100));
        stage.focused = Some(SeatId::from_u128(1));
        let mut scene = Scene::live();
        scene.set_model(&model, &stage);
        let row = |id| &scene.seats[scene.seat_index[&SeatId::from_u128(id)]];
        assert!(row(1).focused && !row(1).unfocused);
        assert!(!row(2).focused && row(2).unfocused);
        assert!(
            !row(3).focused && !row(3).unfocused,
            "the other card dims its body once"
        );
        let is_dim = |layout: &c::Layout, key: &str| {
            let head = named(layout, key).unwrap();
            let c::LayoutKind::Box { style, .. } = &head.children[0].kind else {
                panic!("swarm head")
            };
            style.dim.is_some()
        };
        let layout = scene.layout([1200., 900.], false).unwrap();
        assert!(!is_dim(&layout[1], "swarm0/head"));
        assert!(is_dim(&layout[1], "swarm1/head"));
        let empty = bbt_ui_model::SwarmId::from_u128(300);
        model
            .ui
            .window
            .sidebar
            .swarms
            .push(bbt_ui_model::Swarm::new(empty, "empty", vec![]));
        stage = Stage {
            swarm: Some(empty),
            tab: None,
            focused: None,
        };
        scene.set_model(&model, &stage);
        let layout = scene.layout([1200., 900.], false).unwrap();
        assert!(is_dim(&layout[1], "swarm0/head"));
        assert!(is_dim(&layout[1], "swarm1/head"));
        assert!(
            !is_dim(&layout[1], "swarm2/head"),
            "an empty selected swarm is still active"
        );
        assert!(
            scene.seats.iter().all(|row| !row.unfocused),
            "inactive cards own their rows' one dim"
        );
    }

    #[test]
    fn new_swarm_workdir_is_revealed_and_hidden_controls_cannot_take_clicks() {
        let mut model = bboxy450_term::model::build_at(
            &[],
            &bboxy450_term::grids::Grids::new(),
            &Default::default(),
            Instant::now(),
            1000,
        );
        let ids: Vec<_> = (100..112).map(bbt_proto::SwarmId::from_u128).collect();
        model.ui.window.sidebar.swarms = ids
            .iter()
            .map(|id| bbt_ui_model::Swarm::new(*id, "empty", vec![]))
            .collect();
        model.ui.window.sidebar.open.extend(ids.iter().copied());
        let target = *ids.last().unwrap();
        model.ui.window.stage = bbt_ui_model::Stage {
            swarm: Some(target),
            tab: None,
            focused: None,
        };
        let mut scene = Scene::live();
        scene.set_model(&model, &model.ui.window.stage);
        let layouts = scene.layout([1000., 400.], false).unwrap();
        assert!(
            !scene
                .swarm_folder_regions(&layouts)
                .iter()
                .any(|(id, _, _)| *id == target)
        );
        scene.sidebar_scroll = scene.swarm_folder_scroll(&layouts, target).unwrap();
        assert!(scene.sidebar_scroll > 0.);
        let layouts = scene.layout([1000., 400.], false).unwrap();
        let (viewport, _) = scene.sidebar_extent(&layouts).unwrap();
        let hits = scene.swarm_folder_regions(&layouts);
        assert!(hits.iter().any(|(id, _, _)| *id == target));
        assert!(hits.len() < ids.len());
        for (_, rect, _) in hits {
            assert!(rect.y >= viewport.y && rect.bottom() <= viewport.bottom());
            assert!(rect.x >= viewport.x && rect.right() <= viewport.right());
        }
        assert_eq!(
            scene.swarm_folder_scroll(&layouts, target),
            Some(scene.sidebar_scroll)
        );
    }
}

#[cfg(test)]
mod identity_note_tests {
    use super::*;
    // keeps: spec ux-prototype §5
    #[test]
    fn the_frame_labels_test_accounts_without_changing_real_identity_notes() {
        use bbt_proto::{GuiIdentity, GuiIdentityState::*};
        assert_eq!(
            identity_note(&GuiIdentity {
                state: Certified,
                private_test_run: None,
                reason: None
            }),
            "identity certified"
        );
        assert_eq!(
            identity_note(&GuiIdentity {
                state: LocalDev,
                private_test_run: None,
                reason: None
            }),
            "unsigned, local only"
        );
        let label = "test-account:01ARZ3NDEKTSV4RRFFQ69G5FAV";
        assert_eq!(
            identity_note(&GuiIdentity {
                state: Certified,
                private_test_run: None,
                reason: Some(label.into())
            }),
            label
        );
        let reason = format!("{label} · key unavailable");
        assert_eq!(
            identity_note(&GuiIdentity {
                state: Held,
                private_test_run: None,
                reason: Some(reason.clone())
            }),
            format!("signing held: {reason}")
        );
    }
}

impl Scene {
    pub fn selection_runs(
        &self,
        layout: &c::Layout,
        clip: Rect,
        out: &mut Vec<bbt_ui::text_selection::Run>,
    ) {
        bbt_ui::text_selection::collect(layout, clip, out);
    }
}

impl Scene {
    pub fn text_metrics(&self) -> &dyn TextMeasure {
        &self.metrics
    }
}
