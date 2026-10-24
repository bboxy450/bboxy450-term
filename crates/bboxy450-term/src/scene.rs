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
}
impl ConversationLayout {
    pub fn paint(&self) -> Vec<c::Paint> {
        c::paint(
            &self.layout,
            ui::Context::new(ui::Theme::Dark, ui::Density::Cozy),
        )
    }
}
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
    /// When each seat's focus swell started (rim-glow §7); it runs glow.swell_ms.
    swells: HashMap<SeatId, Instant>,
    /// Seats waiting for the operator (attention §2): since when, and whether muted.
    attention: HashMap<SeatId, (Instant, bool)>,
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
            tasks: vec![],
            tree: Some(tree(&fixture["tree"])),
            seat_index: (0..15).map(|i| (SeatId::from_u128(i as u128), i)).collect(),
            focused: Some(SeatId::from_u128(0)),
            live: false,
            backend_lost: false,
            identity: None,
            hosts: vec![],
            metrics: Metrics::default(),
            rim: TOKENS.glow.rim as f32,
            swells: HashMap::new(),
            attention: HashMap::new(),
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
    // Fed from Model.attention once ATTENTION (Kestrel) lands; tests drive it until then.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn set_attention(&mut self, seat: SeatId, state: Option<(Instant, bool)>) {
        match state {
            Some(state) => self.attention.insert(seat, state),
            None => self.attention.remove(&seat),
        };
    }
    /// Whether any swell or attention loop still needs frames at `now`.
    pub fn animating(&self, now: Instant) -> bool {
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
            seats: vec![],
            tasks: vec![],
            tree: None,
            seat_index: HashMap::new(),
            focused: None,
            live: true,
            backend_lost: false,
            identity: None,
            hosts: vec![],
            metrics: Metrics::default(),
            rim: TOKENS.glow.rim as f32,
            swells: HashMap::new(),
            attention: HashMap::new(),
        }
        .with_env_rim()
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
        self.layout_at(size, effects, Instant::now())
    }
    /// As `layout`, with every seat's swell and attention evaluated at `now`.
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
    fn layout_docked_at(
        &self,
        size: [f32; 2],
        effects: bool,
        dock_height: f32,
        now: Instant,
    ) -> Result<Vec<c::Layout>, String> {
        let ctx = ui::Context::new(ui::Theme::Dark, ui::Density::Cozy);
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
            extra: if self.tree.is_none() {
                seats
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
        // The gutter between panes is the rim size and the stage inset half of it (rim-glow
        // §2): neighbouring rims' outer halves meet; depth never reaches the layout.
        for (id, rect) in self
            .tree
            .as_ref()
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
        // Every block, then every rim, so a swell paints over its neighbours (rim-glow §7).
        let mut rims = vec![];
        for layout in layouts {
            let (body, rim) = c::split_rims(c::paint(layout, ctx));
            ops.extend(body);
            rims.extend(rim);
        }
        ops.extend(rims);
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
    pub fn conversation_layout(
        &self,
        size: [f32; 2],
        view: &ui::modules::composer::Conversation,
    ) -> Result<ConversationLayout, String> {
        let ctx = ui::Context::new(ui::Theme::Dark, ui::Density::Cozy);
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
        let actions = ui::modules::stream_dock::Regions::dock(&layout, &view.stream);
        Ok(ConversationLayout {
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
        let ctx = ui::Context::new(model.ui.window.theme, ui::Density::Cozy);
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
        let ctx = ui::Context::new(ui::Theme::Dark, ui::Density::Cozy);
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
        scene.set_rim(20.);
        assert_eq!(scene.rim, RIM_MAX);
        scene.set_rim(0.5);
        assert_eq!(scene.rim, RIM_MIN);
    }

    /// Every rim the scene paints for one seat, its row (phase from the key) and its pane
    /// (seeded by the seat id), as (swell, attention).
    fn rims_of(scene: &Scene, seat: SeatId, now: Instant) -> Vec<(f32, bool)> {
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
