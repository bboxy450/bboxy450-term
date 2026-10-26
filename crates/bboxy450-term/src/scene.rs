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
    layout::{Axis, Rect, TextMeasure, TextStyle as MeasureStyle, stage_split},
};
use std::{
    cell::RefCell,
    collections::{BTreeMap, HashMap},
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

/// Settings > Rim size range (rim-glow §3).
pub const RIM_MIN: f32 = TOKENS.glow.rim_min as f32;
pub const RIM_MAX: f32 = TOKENS.glow.rim_max as f32;

/// The host retains the very layout it paints; pointer input never reconstructs
/// browser positions or guesses a row's height.
pub struct ConversationLayout {
    pub layout: c::Layout,
    pub regions: ui::modules::composer::Regions,
    pub height: f32,
    pub actions: ui::modules::stream_dock::Regions,
    context: ui::Context,
}
pub struct DocumentLayout {
    pub layout: c::Layout,
    pub regions: ui::modules::document_modal::controller::Regions,
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
    pub panel: Rect,
    pub query: Rect,
    pub caret: Option<Rect>,
    query_origin: f32,
    query_font: MeasureStyle,
    context: ui::Context,
    pub rows: Vec<Rect>,
}
impl LauncherLayout {
    pub fn solved(&self) -> &c::Layout {
        &self.layout
    }
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

pub struct Scene {
    #[cfg(test)]
    projections: usize,
    hosts: Vec<bbt_proto::GuiHost>,
    closed: Vec<(SeatId, Instant, String, String)>,
    layout_revision: u64,
    swarms: Vec<bbt_ui_model::Swarm>,
    pub seats: Vec<SeatRow>,
    tasks: Vec<(SeatId, String, ui::modules::seat_row::TaskRow, Option<i32>)>,
    tree: Option<SplitNode>,
    seat_index: HashMap<SeatId, usize>,
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
    swells: HashMap<SeatId, Instant>,
    /// Seats waiting for the operator (attention §2): since when, and whether muted.
    attention: HashMap<SeatId, (Instant, bool)>,
    attention_origins: HashMap<SeatId, Instant>,
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
            #[cfg(test)]
            projections: 0,
            seats,
            tasks: vec![],
            closed: vec![],
            layout_revision: 0,
            swarms: vec![],
            tree: Some(tree(&fixture["tree"])),
            seat_index: (0..15).map(|i| (SeatId::from_u128(i as u128), i)).collect(),
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
    /// Plays the focus swell on a seat's blocks from `at` (rim-glow §7): it replaces the
    /// locate ripple and is paint only.
    pub fn swell(&mut self, seat: SeatId, at: Instant) {
        self.swells.insert(seat, at);
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
                .any(|at| (now.saturating_duration_since(*at).as_secs_f32() * 1000.) < ms)
    }
    /// The seat's swell envelope at `now` and whether it is the attention loop: |sin| for
    /// attention, one half sine over glow.swell_ms for a focus swell, else 0.
    fn envelope(&self, seat: SeatId, now: Instant) -> (f32, bool) {
        let ms = TOKENS.glow.swell_ms as f32;
        let t = |at: &Instant| now.saturating_duration_since(*at).as_secs_f32() * 1000.;
        if let Some((since, false)) = self.attention.get(&seat) {
            return ((std::f32::consts::PI * t(since) / ms).sin().abs(), true);
        }
        match self.swells.get(&seat).map(t) {
            Some(t) if t < ms => ((std::f32::consts::PI * t / ms).sin(), false),
            _ => (0., false),
        }
    }
    pub fn live() -> Self {
        Self {
            #[cfg(test)]
            projections: 0,
            seats: vec![],
            tasks: vec![],
            closed: vec![],
            layout_revision: 0,
            swarms: vec![],
            tree: None,
            seat_index: HashMap::new(),
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
        self.identity = model.identity.clone();
        self.hosts = model.hosts.clone();
        self.swarms = model.ui.window.sidebar.swarms.clone();
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
        if let Some((seat, at)) = model.focus_swell
            && model.ui.seats.contains_key(&seat)
        {
            self.swell(seat, at);
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
                row.role = seat
                    .role
                    .as_ref()
                    .filter(|role| role.name != "shell")
                    .map(|role| role.name.clone());
                row.title = seat.title.clone();
                row.subtitle = self
                    .subtitle
                    .then(|| model.subtitles.get(id).cloned())
                    .flatten();
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
    /// Retained hit boxes: never a guessed strip/pane coordinate.
    pub fn close_regions(&self, layouts: &[c::Layout]) -> Vec<(SeatId, Rect, bool)> {
        let mut out = vec![];
        for (id, layout) in self
            .tree
            .as_ref()
            .map(SplitNode::leaves)
            .unwrap_or_default()
            .into_iter()
            .zip(layouts.iter().skip(2))
        {
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
    pub fn focused(&self) -> Option<SeatId> {
        self.focused
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
    /// Read the actual pane body after layout, including its padding.
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
        self.layout_contents(size, false, dock_height, now, false)
    }
    pub fn layout_docked_at(
        &self,
        size: [f32; 2],
        effects: bool,
        dock_height: f32,
        now: Instant,
    ) -> Result<Vec<c::Layout>, String> {
        self.layout_contents(size, effects, dock_height, now, true)
    }
    fn layout_contents(
        &self,
        size: [f32; 2],
        effects: bool,
        dock_height: f32,
        now: Instant,
        panes: bool,
    ) -> Result<Vec<c::Layout>, String> {
        let ctx = self.ui_context();
        // Rows, tabs and panes carry each seat's swell and attention (rim-glow §7-8).
        let mut seats = self.seats.clone();
        for (id, index) in &self.seat_index {
            (seats[*index].swell, seats[*index].attention) = self.envelope(*id, now);
        }
        let title = STYLE.spacing.title_bar as f32;
        let sidebar_width = 300_f32.min(size[0] * 0.35);
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
        let mut extra: Vec<_> = if self.tree.is_none() {
            seats.iter().map(|s| s.element_in_swarm(ctx)).collect()
        } else {
            vec![]
        };
        extra.extend(
            self.closed
                .iter()
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
            seats: seats.clone(),
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
        let mut cards = if !self.live || self.swarms.is_empty() {
            vec![card]
        } else {
            self.swarms
                .iter()
                .map(|swarm| {
                    let mut view = card.clone();
                    view.key = swarm.id.to_string();
                    view.name = swarm.name.clone();
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
                    view
                })
                .collect()
        };
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
        // The gutter between panes is the rim size and the stage inset half of it (rim-glow
        // §2): neighbouring rims' outer halves meet; depth never reaches the layout.
        for (id, rect) in self
            .tree
            .as_ref()
            .filter(|_| panes)
            .map(|tree| stage_split(tree, bounds, self.rim).panes)
            .unwrap_or_default()
        {
            let seat = seats[self.seat_index[&id]].clone();
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
        let sidebar = 300_f32.min(size[0] * 0.35);
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
        dock: f32,
        view: &ui::modules::settings::Settings,
        state: &ui::modules::settings::controller::Presentation,
        status: &str,
    ) -> Result<LocalPageLayout, String> {
        let width = (size[0] - 300_f32.min(size[0] * 0.35)).max(1.);
        let height = (size[1] - STYLE.spacing.title_bar as f32 - dock - 44.).max(1.);
        let body = view.element_with(self.ui_context(), &self.metrics, width, height, state);
        let mut page = self.local_page_frame(
            size,
            dock,
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
        let width = (size[0] - 300_f32.min(size[0] * 0.35)).max(1.);
        let graph = ui::modules::graph_tab::GraphTab {
            e2e: Some(checklist.clone()),
            only: Some(vec!["e2e".into()]),
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
    pub fn conversation_layout(
        &self,
        size: [f32; 2],
        view: &ui::modules::composer::Conversation,
    ) -> Result<ConversationLayout, String> {
        let ctx = self.ui_context();
        let width = 300_f32.min(size[0] * 0.35);
        let footer = if self.identity.is_some() { 24. } else { 0. };
        let available = (size[1] - STYLE.spacing.title_bar as f32 - footer).max(1.);
        let height = (available * 0.45).max(200.).min(available);
        let element = view
            .element(ctx, [width, height], &self.metrics)
            .map_err(|e| e.to_string())?;
        let measured = element
            .layout(Rect::new(0., 0., width, 0.), ctx, &self.metrics)
            .map_err(|e| e.to_string())?;
        let height = measured.rect.h;
        let layout = element
            .layout(
                Rect::new(0., size[1] - footer - height, width, height),
                ctx,
                &self.metrics,
            )
            .map_err(|e| e.to_string())?;
        let regions = ui::modules::composer::Regions::new(&layout).map_err(|e| e.to_string())?;
        let mut actions = ui::modules::stream_dock::Regions::dock(&layout, &view.stream);
        actions.record_text_links(&layout, &self.metrics);
        actions.attachments("composer/attachments", &view.composer.attach);
        Ok(ConversationLayout {
            context: ctx,
            layout,
            regions,
            height,
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
    pub fn launcher_layout(
        &self,
        size: [f32; 2],
        launcher: &Launcher,
    ) -> Result<LauncherLayout, String> {
        let ctx = self.ui_context();
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
            context: ctx,
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
                prepared.markdown.hosted_body_element(
                    &prepared.document.title,
                    ctx,
                    &self.metrics,
                    width,
                    &host.links,
                    host.scroll.horizontal(),
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
        let regions = ui::modules::document_modal::controller::Regions::retained(&layout);
        Ok(DocumentLayout {
            layout,
            regions,
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
        document: Option<&DocumentLayout>,
    ) -> Vec<c::Paint> {
        let mut ops = self.foreground_with_conversation(size, launcher, None, conversation);
        if let Some(document) = document {
            ops.extend(document.paint());
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
    ) -> Vec<c::Paint> {
        let mut ops = conversation.map_or_else(Vec::new, ConversationLayout::paint);
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
        assert_eq!(page.viewport.x, 300.);
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
        let sidebar_edge = 300_f32.min(1600. * 0.35);
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
        let key = &scene.seats[scene.seat_index[&seat]].key;
        let ctx = ui::Context::new(ui::Theme::Dark, ui::Density::Cozy);
        let mut out = vec![];
        for layout in scene.layout_at([1600., 960.], true, now).unwrap() {
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
        scene.swell(seat, t0);
        let peak = rims_of(&scene, seat, ms(TOKENS.glow.swell_ms as u64 / 2));
        assert!(peak.len() >= 2, "the row and the pane swell: {peak:?}");
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
        scene.swell(seat, frozen.now);
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
        let rims = rims_of(&scene, seat, peak.now);
        assert!(rims.len() >= 2, "pane and row must both carry the swell");
        assert!(
            rims.iter()
                .all(|(envelope, attention)| (*envelope - 1.).abs() < 1e-6 && !attention)
        );
        assert_eq!(bounds(peak.now), before);
        let settled = clock.apply(UiClockCommand::Advance { ms: 450 }).unwrap();
        assert!(
            rims_of(&scene, seat, settled.now)
                .iter()
                .all(|rim| rim.0.abs() < 1e-6)
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
        assert_eq!(scene.envelope(seat, clock.now()), (0., true));
        clock.apply(UiClockCommand::Advance { ms: 450 }).unwrap();
        for _ in 0..3 {
            scene.set_model_with_clock(&model, &model.ui.window.stage, &clock);
            let (phase, attention) = scene.envelope(seat, clock.now());
            assert!((phase - 1.).abs() < 1e-6 && attention);
            assert_eq!(model.attention[&seat].since, source);
        }
        model.attention.get_mut(&seat).unwrap().muted = true;
        scene.set_model_with_clock(&model, &model.ui.window.stage, &clock);
        assert_eq!(scene.envelope(seat, clock.now()), (0., false));
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
        assert!(model.attention_focus(seat, 1, key, Some(peak)));
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
        let foreground =
            scene.foreground_with_document(size, None, Some(&hud), None, Some(&document));
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
        assert!(model.attention_focus(seat, generation, None, Some(landed)));
        let mut scene = Scene::live();
        for _ in 0..3 {
            model.document_reply = Some(std::sync::Arc::new(bboxy450_term::documents::Reply {
                id: bbt_mux::Ulid::new(),
                response: bboxy450_term::documents::Response::Failed("not_found: retained".into()),
            }));
            scene.set_model(&model, &model.ui.window.stage);
            let peak = landed + std::time::Duration::from_millis(450);
            let rims = depth_tests::rims_of(&scene, seat, peak);
            assert!(rims.len() >= 2);
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
            .settings_page_layout(
                [1600., 960.],
                180.,
                &view,
                &controller.presentation(),
                "saved",
            )
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
            (left - 304.).abs() < 0.01,
            "saved rim must produce its 4 px stage inset"
        );
        assert!(page.viewport.bottom() <= 780.);
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
                reason: None
            }),
            "identity certified"
        );
        assert_eq!(
            identity_note(&GuiIdentity {
                state: LocalDev,
                reason: None
            }),
            "unsigned, local only"
        );
        let label = "test-account:01ARZ3NDEKTSV4RRFFQ69G5FAV";
        assert_eq!(
            identity_note(&GuiIdentity {
                state: Certified,
                reason: Some(label.into())
            }),
            label
        );
        let reason = format!("{label} · key unavailable");
        assert_eq!(
            identity_note(&GuiIdentity {
                state: Held,
                reason: Some(reason.clone())
            }),
            format!("signing held: {reason}")
        );
    }
}
