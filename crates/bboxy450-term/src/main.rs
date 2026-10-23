//! R2 app shell: event-driven native scene, memory-only UI drain and hitch meter.
mod assets;
mod budget;
mod capture;
mod display_link;
mod gpu;
mod live;
mod log;
mod scene;

use bboxy450_term::keyboard;
use bboxy450_term::launcher::{
    CaretMove, Command as LauncherCommand, Controller as LauncherController,
    Outcome as LauncherOutcome,
};
use bbt_proto::{SeatId, View};
use bbt_rt::{HitchMeter, Latest, Phases, UiInbox, UiMsg};
use bbt_ui_model::{
    Model as UiModel, Stage,
    event::Event as InputEvent,
    input::{
        Direction, Focus, FocusContext, FocusOrigin, KeyRoute, Modifiers, Navigation,
        NavigationKey, route_navigation,
    },
    layout::Rect,
    tokens::Theme,
};
use display_link::{Clock, DisplayLink, Mode};
use std::collections::{BTreeMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};
use winit::platform::macos::MonitorHandleExtMacOS;
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{ElementState, Ime, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy},
    keyboard::{Key, KeyCode, ModifiersState, NamedKey, PhysicalKey},
    window::{Window, WindowId},
};

#[derive(Debug)]
pub enum UiEvent {
    Wake,
    DemoFinished,
    BeginIdle,
    EndIdle(u64),
    EndAnimation,
    Quit,
    BackendStopped(Result<(), String>),
    BackendLost(String),
    BackendReady,
    SmokeDeadline,
    VerifyGridDeadline,
    VerifyFailed(String),
    Capture,
}
struct Wake(EventLoopProxy<UiEvent>);
impl bbt_rt::Waker for Wake {
    fn wake(&self) {
        let _ = self.0.send_event(UiEvent::Wake);
    }
}
type Prepared = Result<(gpu::Gpu, scene::Scene), String>;

#[derive(Default)]
struct IdleQuiescence {
    observed: Option<(u64, u64)>,
    epoch: u64,
    active: Option<(Instant, u64, u64)>,
    invalidations: u64,
}
impl IdleQuiescence {
    fn ready(&mut self, producer_finished: bool, pending: bool, counts: (u64, u64)) -> bool {
        if !producer_finished || pending {
            self.observed = None;
            return false;
        }
        let quiet = self.observed == Some(counts);
        self.observed = Some(counts);
        quiet
    }
    fn begin(&mut self, now: Instant, counts: (u64, u64)) -> u64 {
        self.epoch += 1;
        self.active = Some((now, counts.0, counts.1));
        self.epoch
    }
    fn interrupt(&mut self) -> bool {
        self.invalidations += 1;
        self.observed = None;
        self.active.take().is_some()
    }
    fn finish(&mut self, epoch: u64) -> Option<(Instant, u64, u64)> {
        if epoch != self.epoch {
            return None;
        }
        self.active.take()
    }
    fn deadline_error(&self) -> String {
        if self.invalidations != 0 {
            format!(
                "idle window never stable within 30 seconds (native invalidations: {})",
                self.invalidations
            )
        } else {
            "smoke probe did not finish within 30 seconds".into()
        }
    }
}

#[derive(Default)]
struct FocusState {
    controller: Focus,
    stage: Option<Stage>,
    pending_layout: bool,
}
enum FocusAction {
    Activate(SeatId, FocusOrigin),
    Navigate(Navigation),
}
impl FocusState {
    fn reconcile(&mut self, previous: Option<&UiModel>, next: &UiModel) {
        let owner = |seat| {
            if next.seats.get(&seat)?.lifecycle != bbt_ui_model::Lifecycle::Open {
                return None;
            }
            let (swarm, tab) = next.tab_of(seat)?;
            let swarm = &next.window.sidebar.swarms[swarm];
            Some(Stage {
                swarm: Some(swarm.id),
                tab: Some(swarm.tabs[tab].id),
                focused: Some(seat),
            })
        };
        // New membership may select a newly launched pane, just as the old
        // scene did. Ordinary screen/state publications never replace Stage.
        let new_seat = previous.and_then(|old| {
            let tab = self.stage.as_ref()?.tab;
            next.window
                .sidebar
                .swarms
                .iter()
                .flat_map(|s| &s.tabs)
                .find(|t| Some(t.id) == tab)?
                .tree
                .leaves()
                .into_iter()
                .find(|id| !old.seats.contains_key(id))
        });
        self.stage = new_seat
            .and_then(owner)
            .or_else(|| self.stage.as_ref().and_then(|s| s.focused).and_then(owner))
            .or_else(|| Some(next.window.stage.clone()));
    }
    fn apply(
        &mut self,
        source: &UiModel,
        action: FocusAction,
        panes: &[(SeatId, Rect)],
        now: Instant,
    ) -> Result<Vec<InputEvent>, bbt_ui_model::EditError> {
        // Clone only for an input action, never for a terminal publication or
        // ordinary frame. Focus is the sole writer of explicit Stage changes.
        let mut model = source.clone();
        if let Some(stage) = &self.stage {
            model.window.stage = stage.clone();
        }
        let events = match action {
            FocusAction::Activate(seat, origin) => {
                self.controller.activate(&mut model, seat, origin, now)
            }
            FocusAction::Navigate(Navigation::Direction(direction)) => {
                self.controller.navigate(&mut model, panes, direction, now)
            }
            FocusAction::Navigate(Navigation::Cycle { backwards }) => {
                self.controller.cycle(&mut model, backwards, now)
            }
        }?;
        self.stage = Some(model.window.stage);
        self.pending_layout |= events.iter().any(|event| {
            matches!(
                event,
                InputEvent::FocusChanged {
                    stage_switched: true,
                    ..
                }
            )
        });
        Ok(events)
    }
    fn layout_ready(&mut self, source: &UiModel, panes: &[(SeatId, Rect)]) -> Vec<InputEvent> {
        if !self.pending_layout {
            return vec![];
        }
        let mut model = source.clone();
        if let Some(stage) = &self.stage {
            model.window.stage = stage.clone();
        }
        let events = self.controller.layout_ready(&model, panes, Instant::now());
        self.pending_layout = model.window.stage.focused.is_some_and(|seat| {
            !panes
                .iter()
                .any(|(id, rect)| *id == seat && rect.w > 0. && rect.h > 0.)
        });
        events
    }
}

#[derive(Debug, PartialEq, Eq)]
enum RoutedKey {
    Quit,
    OpenLauncher,
    ToggleHud,
    Launcher(LauncherCommand),
    Pane(Navigation),
    Terminal(Vec<u8>),
    Local,
}
fn input_modifiers(mods: ModifiersState) -> Modifiers {
    Modifiers {
        command: mods.super_key(),
        alt: mods.alt_key(),
        control: mods.control_key(),
        shift: mods.shift_key(),
    }
}
const LAUNCHABLES: [(&str, &str, &str, &str, &str); 5] = [
    (
        "claude",
        "Claude",
        "executor by default",
        "#D97757",
        "Suggestions",
    ),
    (
        "codex",
        "Codex",
        "executor by default",
        "#1F2937",
        "Suggestions",
    ),
    ("pi", "pi", "executor by default", "#7C3AED", "Harnesses"),
    ("omp", "omp", "executor by default", "#0EA5E9", "Harnesses"),
    ("zsh", "zsh", "~", "#111827", "Shells"),
];

fn default_spawn(
    id: &str,
    modifiers: Modifiers,
    model: Option<&bboxy450_term::model::Model>,
) -> Option<live::UiMsg> {
    if modifiers != Modifiers::default() {
        return None;
    }
    for (kind, _, _, _, _) in LAUNCHABLES {
        let location = if id == kind {
            Some(bbt_proto::Where::Local)
        } else {
            model.and_then(|model| {
                model
                    .hosts
                    .iter()
                    .find(|host| {
                        id == format!("{}:{kind}", bboxy450_term::hosts::key(&host.location))
                    })
                    .map(|host| host.location.clone())
            })
        };
        if let Some(location) = location {
            return Some(live::UiMsg::Spawn {
                harness_kind: kind.into(),
                name: None,
                location,
            });
        }
    }
    None
}

/// This is the same routing entry point used by winit and the synthetic tests.
fn route_key(input: &keyboard::Input<'_>, modes: u64, query: Option<&str>) -> RoutedKey {
    if input.state == ElementState::Released {
        return if query.is_none() {
            keyboard::encode(input, modes).map_or(RoutedKey::Local, RoutedKey::Terminal)
        } else {
            RoutedKey::Local
        };
    }
    let key = &input.logical_key;
    let physical = input.physical_key;
    let text = input.text;
    let mods = input.modifiers.state;
    let repeat = input.repeat;
    let command = mods.super_key();
    if command && physical == PhysicalKey::Code(KeyCode::KeyQ) {
        return if repeat {
            RoutedKey::Local
        } else {
            RoutedKey::Quit
        };
    }
    if command
        && !mods.alt_key()
        && !mods.shift_key()
        && !mods.control_key()
        && physical == PhysicalKey::Code(KeyCode::KeyT)
    {
        return if repeat {
            RoutedKey::Local
        } else {
            RoutedKey::OpenLauncher
        };
    }
    if command && mods.alt_key() && physical == PhysicalKey::Code(KeyCode::KeyF) {
        return if repeat {
            RoutedKey::Local
        } else {
            RoutedKey::ToggleHud
        };
    }
    let navigation = match key {
        Key::Named(NamedKey::ArrowLeft) => NavigationKey::Arrow(Direction::Left),
        Key::Named(NamedKey::ArrowRight) => NavigationKey::Arrow(Direction::Right),
        Key::Named(NamedKey::ArrowUp) => NavigationKey::Arrow(Direction::Up),
        Key::Named(NamedKey::ArrowDown) => NavigationKey::Arrow(Direction::Down),
        Key::Named(NamedKey::Tab) => NavigationKey::Tab,
        _ => NavigationKey::Other,
    };
    match route_navigation(
        navigation,
        input_modifiers(mods),
        if query.is_some() {
            FocusContext::Launcher
        } else {
            FocusContext::Terminal
        },
    ) {
        KeyRoute::Pane(navigation) => return RoutedKey::Pane(navigation),
        KeyRoute::Terminal => {
            return keyboard::encode(input, modes).map_or(RoutedKey::Local, RoutedKey::Terminal);
        }
        _ => {}
    }
    let Some(query) = query else {
        return RoutedKey::Local;
    };
    let pick = input_modifiers(mods);
    let local = match key {
        Key::Named(NamedKey::ArrowRight) if (command && !mods.alt_key()) || mods.shift_key() => {
            LauncherCommand::MoveCaret {
                motion: if command {
                    CaretMove::End
                } else {
                    CaretMove::Right
                },
                extend: mods.shift_key(),
            }
        }
        Key::Named(NamedKey::ArrowLeft) if (command && !mods.alt_key()) || mods.shift_key() => {
            LauncherCommand::MoveCaret {
                motion: if command {
                    CaretMove::Start
                } else {
                    CaretMove::Left
                },
                extend: mods.shift_key(),
            }
        }
        Key::Named(NamedKey::Home) => LauncherCommand::MoveCaret {
            motion: CaretMove::Start,
            extend: mods.shift_key(),
        },
        Key::Named(NamedKey::End) => LauncherCommand::MoveCaret {
            motion: CaretMove::End,
            extend: mods.shift_key(),
        },
        Key::Named(NamedKey::ArrowUp) => LauncherCommand::Up,
        Key::Named(NamedKey::ArrowDown) => LauncherCommand::Down,
        Key::Named(NamedKey::Tab) if mods.shift_key() => LauncherCommand::Up,
        Key::Named(NamedKey::Tab) => LauncherCommand::Down,
        Key::Named(NamedKey::ArrowRight) => LauncherCommand::Right,
        Key::Named(NamedKey::ArrowLeft) => LauncherCommand::Left,
        Key::Named(NamedKey::Enter) if !repeat => LauncherCommand::Activate(pick),
        Key::Named(NamedKey::Escape) if !repeat => LauncherCommand::Escape,
        Key::Named(NamedKey::Backspace) => LauncherCommand::Backspace,
        Key::Named(NamedKey::Delete) => LauncherCommand::Delete,
        Key::Character(value) if command && value.eq_ignore_ascii_case("a") => {
            LauncherCommand::SelectAll
        }
        Key::Character(value)
            if (command || mods.control_key()) && value.eq_ignore_ascii_case("r") && !repeat =>
        {
            LauncherCommand::Reroll
        }
        Key::Character(value)
            if query.is_empty()
                && value.len() == 1
                && matches!(value.as_bytes()[0], b'1'..=b'9') =>
        {
            if repeat {
                return RoutedKey::Local;
            }
            LauncherCommand::Digit {
                digit: value.as_bytes()[0] - b'0',
                modifiers: pick,
            }
        }
        Key::Character(value) if !command && !mods.control_key() => {
            LauncherCommand::Insert(text.unwrap_or(value).into())
        }
        Key::Named(NamedKey::Space) if !command && !mods.control_key() => {
            LauncherCommand::Insert(" ".into())
        }
        _ => return RoutedKey::Local,
    };
    RoutedKey::Launcher(local)
}

/// Local repeats keep their existing navigation/edit behavior. Only a repeat
/// that would cross from a locally consumed press into the PTY is suppressed.
fn route_input(
    state: &mut keyboard::State,
    input: &keyboard::Input<'_>,
    modes: u64,
    query: Option<&str>,
) -> RoutedKey {
    let was_local = state.suppressed(input.physical_key, input.state, input.repeat);
    let routed = route_key(input, modes, query);
    if was_local
        && (input.state == ElementState::Released || matches!(routed, RoutedKey::Terminal(_)))
    {
        return RoutedKey::Local;
    }
    if !matches!(routed, RoutedKey::Terminal(_)) {
        state.consumed(input.physical_key, input.state);
    }
    routed
}

fn preedit_command(text: String, cursor: Option<(usize, usize)>) -> LauncherCommand {
    // winit reports UTF-8 bytes; the retained field follows UTF-16 indices.
    let caret = cursor.and_then(|(at, _)| text.get(..at).map(|s| s.encode_utf16().count()));
    LauncherCommand::Preedit { text, caret }
}

/// Public item records, with the provider projection kept authoritative. Picks
/// are identified by id; a display label never becomes a command.
fn launcher_items(
    model: Option<&bboxy450_term::model::Model>,
) -> Vec<bbt_ui::modules::launcher::Item> {
    use bbt_ui::modules::launcher::Item;
    let launchables = || {
        LAUNCHABLES
            .into_iter()
            .map(|(id, label, sub, tile, section)| Item {
                id: id.into(),
                label: label.into(),
                sub: if id == "zsh" {
                    sub.into()
                } else {
                    bbt_mux::adapters::HarnessRecord::builtin(id).map_or_else(
                        |_| sub.into(),
                        |harness| format!("{} by default", harness.default_role.name),
                    )
                },
                harness: id.into(),
                kind: if id == "zsh" { "Shell" } else { "Harness" }.into(),
                tile: Some(tile.into()),
                section: Some(section.into()),
                aliases: vec![id.into()],
                enter_picks: true,
                ..Default::default()
            })
            .collect::<Vec<_>>()
    };
    let mut items = launchables();
    for item in &mut items {
        item.children = Some(
            [
                ("tab", "New tab"),
                ("right", "Split right"),
                ("bottom", "Split down"),
            ]
            .into_iter()
            .map(|(id, label)| Item {
                id: format!("{}:{id}", item.id),
                label: label.into(),
                harness: item.harness.clone(),
                kind: "Placement".into(),
                tile: item.tile.clone(),
                ..Default::default()
            })
            .collect(),
        );
    }
    let mut providers = model.map_or_else(Vec::new, |m| m.provider_launcher_items());
    for root in &mut providers {
        for host in root.children.iter_mut().flatten() {
            if host.children.is_some() {
                host.children = Some(
                    items
                        .clone()
                        .into_iter()
                        .map(|mut item| {
                            item.id = format!("{}:{}", host.id, item.id);
                            for child in item.children.iter_mut().flatten() {
                                child.id = format!("{}:{}", host.id, child.id);
                            }
                            item
                        })
                        .collect(),
                );
            }
        }
    }
    items.extend(providers);
    let seats = model.map_or_else(Vec::new, |m| {
        m.ui.seats
            .values()
            .filter(|s| s.lifecycle == bbt_ui_model::Lifecycle::Open)
            .map(|seat| Item {
                id: format!("seat:{}", seat.id),
                label: seat.name.clone(),
                sub: seat.title.clone(),
                harness: seat.harness.0.clone(),
                kind: "Seat".into(),
                navigation: true,
                ..Default::default()
            })
            .collect()
    });
    items.push(Item {
        id: "goto".into(),
        label: "Go to".into(),
        aliases: vec!["go".into(), "jump".into()],
        harness: "sidebarfoxopen".into(),
        kind: "Navigate".into(),
        section: Some("Navigate".into()),
        navigation: true,
        children: Some(seats),
        ..Default::default()
    });
    items.push(Item {
        id: "graph".into(),
        label: "Graph".into(),
        harness: "sidebarfoxopen".into(),
        kind: "Open".into(),
        navigation: true,
        ..Default::default()
    });
    items.push(Item {
        id: "settings".into(),
        label: "Settings".into(),
        aliases: vec!["prefs".into(), "preferences".into(), "config".into()],
        harness: "gear".into(),
        kind: "Open".into(),
        navigation: true,
        enter_picks: true,
        children: Some(
            [
                "Account",
                "Cloud",
                "Machines",
                "Roles",
                "Harnesses",
                "Appearance",
                "Shortcuts",
            ]
            .into_iter()
            .map(|section| Item {
                id: format!("settings:{}", section.to_lowercase()),
                label: section.into(),
                harness: "gear".into(),
                kind: "Section".into(),
                navigation: true,
                ..Default::default()
            })
            .collect(),
        ),
        ..Default::default()
    });
    items
}

fn new_launcher(model: Option<&bboxy450_term::model::Model>, used: usize) -> LauncherController {
    use bbt_ui::modules::launcher::{Cap, Launcher};
    LauncherController::new(
        Launcher {
            items: launcher_items(model),
            query: String::new(),
            path: vec![],
            selected: Some(0),
            placeholder: "launch or go to · type to search".into(),
            name: "new seat".into(),
            swatch: [0.18, 0.42, 1., 1.],
            actions: vec![
                ("Launch / go".into(), "↵".into()),
                ("Into".into(), "→".into()),
                ("Back".into(), "←".into()),
            ],
            cap: (used >= PENDING_SPAWNS).then(|| Cap {
                scope: "pending launches".into(),
                used,
                limit: PENDING_SPAWNS,
                counts: "queued spawn requests count".into(),
                ..Default::default()
            }),
            spotlight: true,
            edit: None,
        },
        0,
    )
}

struct App {
    proxy: EventLoopProxy<UiEvent>,
    inbox: UiInbox,
    logger: log::Logger,
    prepared: Arc<Latest<Prepared>>,
    updates: Arc<Latest<usize>>,
    live_updates: Arc<Latest<Arc<bboxy450_term::model::Model>>>,
    live_state: Option<Arc<bboxy450_term::model::Model>>,
    live_commands: Option<live::CommandSender>,
    pending_commands: VecDeque<live::UiMsg>,
    admission_refusals: AdmissionRefusals,
    watched: Vec<(SeatId, View)>,
    pane_sizes: BTreeMap<SeatId, (u64, u16, u16)>,
    pane_bounds: Vec<(SeatId, Rect)>,
    focus_bounds: Vec<(SeatId, Rect)>,
    focus: FocusState,
    launcher: Option<LauncherController>,
    launcher_layout: Option<scene::LauncherLayout>,
    pointer: [f32; 2],
    query_dragging: bool,
    backend_lost: bool,
    render_context: Option<tokio::sync::oneshot::Sender<live::RenderContext>>,
    closing: bool,
    smoke_bootstrap: Option<tokio::sync::oneshot::Sender<()>>,
    smoke_wake_seen: bool,
    window: Option<Arc<Window>>,
    pacer: Option<DisplayLink>,
    pacing: Mode,
    clock: Clock,
    occluded: bool,
    maximum_hz: f64,
    budget: budget::Budget,
    gpu: Option<gpu::Gpu>,
    scene: scene::Scene,
    meter: HitchMeter,
    modifiers: ModifiersState,
    keyboard: keyboard::State,
    overlay: bool,
    fake: bool,
    smoke: bool,
    demo_started: bool,
    demo_finished: bool,
    idle_scheduled: bool,
    idle_quiet: IdleQuiescence,
    redraw_pending: std::cell::Cell<bool>,
    last_update: usize,
    error: Option<String>,
    capture: Option<std::path::PathBuf>,
    capture_pending: bool,
    pending_resize: Option<LogicalSize<f64>>,
    animate: bool,
    cadence_test: bool,
    animation_start: Option<(Instant, u64)>,
    frozen_ms: f32,
    verify: Verify,
}
impl App {
    fn open_launcher(&mut self) {
        let used = self
            .pending_commands
            .iter()
            .filter(|m| m.is_spawn())
            .count();
        self.launcher = Some(new_launcher(self.live_state.as_deref(), used));
        self.launcher_layout = None;
        if let Some(window) = &self.window {
            window.set_ime_allowed(true);
        }
        self.query_dragging = false;
        self.logger.write("launcher open");
        self.request_redraw();
    }
    fn close_launcher(&mut self) {
        self.launcher = None;
        self.launcher_layout = None;
        self.query_dragging = false;
        if let Some(window) = &self.window {
            window.set_ime_allowed(false);
        }
        self.request_redraw();
    }
    fn launcher_command(&mut self, command: LauncherCommand) {
        let Some(launcher) = &mut self.launcher else {
            return;
        };
        let pointer_edit = matches!(command, LauncherCommand::SetCaret { .. });
        let outcome = launcher.handle(command);
        match outcome {
            LauncherOutcome::Close => self.close_launcher(),
            LauncherOutcome::Changed => {
                // Pointer selection changes neither query nor row membership.
                // Retain those hit regions until redraw so a second motion
                // before the frame cannot lose the drag's final position.
                if !pointer_edit {
                    self.launcher_layout = None;
                }
                self.request_redraw();
            }
            LauncherOutcome::Pick { id, modifiers } => {
                if let Some(message) = default_spawn(&id, modifiers, self.live_state.as_deref())
                    && self.live_commands.is_some()
                {
                    self.logger
                        .write(format!("launcher default spawn requested: id={id}"));
                    self.queue(message);
                    self.close_launcher();
                } else {
                    self.logger.write(format!(
                        "launcher pick not_wired_yet: id={id} modifiers={modifiers:?}"
                    ));
                }
            }
            LauncherOutcome::Local => {}
            other => self.logger.write(format!("launcher outcome: {other:?}")),
        }
    }
    fn focus_action(&mut self, action: FocusAction) {
        let Some(model) = &self.live_state else {
            return;
        };
        match self
            .focus
            .apply(&model.ui, action, &self.focus_bounds, Instant::now())
        {
            Ok(events) => {
                self.scene.set_model(
                    model,
                    self.focus.stage.as_ref().expect("focus action stage"),
                );
                for event in events {
                    self.logger.write(format!("ui input: {event:?}"));
                }
                self.request_redraw();
            }
            Err(error) => self.logger.write(format!("focus refused: {error}")),
        }
    }
    fn queue(&mut self, message: live::UiMsg) {
        if self.closing || self.live_commands.is_none() {
            return;
        }
        if let Err(kind) = queue_pending(&mut self.pending_commands, message) {
            self.admission_refusals.record(kind);
            self.logger.write(self.admission_refusals.report());
        }
        self.request_redraw();
    }
    fn flush_commands(&mut self) {
        if let Some(sender) = &self.live_commands {
            if flush_pending(sender, &mut self.pending_commands, self.backend_lost).is_err() {
                self.backend_lost = true;
                self.scene.backend_lost();
            } else if !self.backend_lost && !self.pending_commands.is_empty() {
                self.request_redraw();
            }
        }
    }
    fn watch_panes(&mut self, bounds: &[(SeatId, Rect)], scale: f32) {
        if self.live_commands.is_none() {
            return;
        }
        let Some(model) = &self.live_state else {
            return;
        };
        let mut watched: Vec<_> = bounds
            .iter()
            .filter_map(|(id, _)| {
                let generation = model.ui.seats.get(id)?.process_generation;
                let view = self
                    .watched
                    .iter()
                    .find(|(old_id, view)| old_id == id && view.generation == generation)
                    .map(|(_, view)| *view)
                    .unwrap_or(View {
                        seat: bbt_mux::Ulid::from(id.0.0),
                        generation,
                        view_generation: bbt_mux::Ulid::new(),
                    });
                Some((*id, view))
            })
            .collect();
        // IO uses the first visible pane as focus; membership is still a set.
        watched.sort_by_key(|(id, _)| Some(*id) != self.scene.focused());
        let mut sizes = Vec::new();
        for (id, rect) in bounds {
            let Some(metadata) = model.ui.seats.get(id) else {
                continue;
            };
            let cell = model.frames.get(id).zip(model.screens.get(id)).map_or(
                [8., 15.],
                |(frame, screen)| {
                    [
                        frame.width as f32 / screen.cols.max(1) as f32 / scale,
                        frame.height as f32 / screen.rows.max(1) as f32 / scale,
                    ]
                },
            );
            // While fitting the real window, avoid an intermediate PTY resize
            // that could race the fixture input. Once ready, use actual bounds.
            let (cols, rows) = self
                .verify
                .grid_size
                .filter(|_| !self.verify.grid_ready)
                .unwrap_or((
                    (rect.w / cell[0]).floor().clamp(1., 512.) as u16,
                    (rect.h / cell[1]).floor().clamp(1., 256.) as u16,
                ));
            let size = (metadata.process_generation, cols, rows);
            if self.pane_sizes.get(id) != Some(&size) {
                sizes.push((*id, size));
            }
        }
        if watched != self.watched {
            self.watched = watched.clone();
            self.queue(live::UiMsg::Watched(watched));
        }
        self.pane_sizes
            .retain(|id, _| bounds.iter().any(|(visible, _)| visible == id));
        for (seat, (generation, cols, rows)) in sizes {
            self.pane_sizes.insert(seat, (generation, cols, rows));
            self.queue(live::UiMsg::Resize {
                seat,
                generation,
                cols,
                rows,
            });
        }
    }
    fn request_close(&mut self, event_loop: &ActiveEventLoop) {
        if self.closing {
            return;
        }
        self.closing = true;
        self.pending_commands.clear();
        self.logger.write("app close requested; stopping IO owner");
        if self.live_commands.take().is_none() {
            event_loop.exit();
        }
        // The IO owner reaps both children, then sends BackendStopped. Never
        // wait for process shutdown on this thread.
    }
    fn request_redraw(&self) {
        if !self.occluded
            && let Some(pacer) = &self.pacer
        {
            pacer.request_frame();
        }
        if let Some(window) = &self.window {
            self.redraw_pending.set(true);
            window.request_redraw();
        }
    }
    fn schedule_idle_check(&self) {
        let proxy = self.proxy.clone();
        bbt_rt::handle().spawn(async move {
            tokio::time::sleep(Duration::from_secs(1)).await;
            let _ = proxy.send_event(UiEvent::BeginIdle);
        });
    }
    fn native_idle_invalidation(&mut self, reason: &str) {
        if self.smoke && !self.cadence_test && self.idle_scheduled {
            // Native window changes require a real repaint. That interval is
            // no longer idle; do not count it as the zero-work measurement.
            let interrupted = self.idle_quiet.interrupt();
            self.logger.write(format!(
                "idle probe native invalidation #{}: {reason}; measured_window_interrupted={interrupted}",
                self.idle_quiet.invalidations
            ));
            if interrupted {
                self.schedule_idle_check();
            }
        }
    }
    fn fail(&mut self, error: String, event_loop: &ActiveEventLoop) {
        self.logger.write(format!("app error: {error}"));
        self.error = Some(error);
        event_loop.exit();
    }
    fn init(&mut self, event_loop: &ActiveEventLoop) -> Result<(), String> {
        let window = Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_title("bboxy450-term")
                        .with_inner_size(LogicalSize::new(1600., 960.))
                        .with_min_inner_size(if self.verify.grid_size.is_some() {
                            LogicalSize::new(1., 1.)
                        } else {
                            LogicalSize::new(800., 600.)
                        })
                        .with_decorations(false),
                )
                .map_err(|e| e.to_string())?,
        );
        if self.launcher.is_some() {
            window.set_ime_allowed(true);
            self.logger.write("debug launcher open");
        }
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::METAL,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let surface = instance
            .create_surface(window.clone())
            .map_err(|e| e.to_string())?;
        let size = window.inner_size();
        if let Some(monitor) = window.current_monitor() {
            self.maximum_hz = display_link::maximum_hz(&monitor);
            self.meter = HitchMeter::new(self.maximum_hz);
            let max_hz = monitor
                .video_modes()
                .map(|mode| mode.refresh_rate_millihertz())
                .max()
                .unwrap_or(0);
            self.logger.write(format!(
                "panel: {:?}, current={}mHz max_mode={}mHz maximumFramesPerSecond={}",
                monitor.name(),
                monitor.refresh_rate_millihertz().unwrap_or(0),
                max_hz,
                self.maximum_hz
            ));
        }
        let prepared = self.prepared.clone();
        let scale = window.scale_factor() as f32;
        let theme = self.verify.theme.unwrap_or(Theme::Dark);
        let effects = self.animate;
        let fixture = self.fake || self.animate;
        let render_context = self.render_context.take();
        let bootstrap = if self.smoke {
            let (open, wait) = tokio::sync::oneshot::channel();
            self.smoke_bootstrap = Some(open);
            Some(wait)
        } else {
            None
        };
        bbt_rt::handle().spawn(async move {
            if let Some(wait) = bootstrap {
                let _ = wait.await;
            }
            let result = async {
                let mut gpu = gpu::Gpu::new(instance, surface, [size.width, size.height]).await?;
                if let Some(sender) = render_context {
                    let _ = sender.send(live::RenderContext {
                        context: gpu.shared_context(),
                        scale,
                        theme,
                    });
                }
                let scene = if fixture {
                    scene::Scene::new()
                } else {
                    scene::Scene::live()
                };
                let logical = [size.width as f32 / scale, size.height as f32 / scale];
                let layouts = scene.layout(logical, effects)?;
                gpu.warm(scene.paint(&layouts, logical, None, None), scale)?;
                Ok((gpu, scene))
            }
            .await;
            prepared.publish(result);
        });
        self.window = Some(window);
        if self.smoke {
            // Force an early producer wake before GPU bootstrap. This catches
            // the live-seat startup race in the real native CI probe.
            self.inbox.dirty.source(5).mark();
        }
        Ok(())
    }
    fn install_gpu(&mut self) -> Result<(), String> {
        if let Some(result) = self.prepared.take() {
            let (mut gpu, scene) = result?;
            self.scene = scene;
            if let Some(state) = &self.live_state {
                self.scene.set_model(
                    state,
                    self.focus.stage.as_ref().unwrap_or(&state.ui.window.stage),
                );
            }
            let size = self.window.as_ref().unwrap().inner_size();
            gpu.resize(size.width, size.height);
            self.logger.write(format!(
                "renderer ready: {}, Fifo, frame latency 2, scene seats={}",
                gpu.adapter_name(),
                self.scene.seats.len()
            ));
            self.logger.write(format!(
                "Metal maximumDrawableCount={:?}, configured latency={}",
                gpu.maximum_drawables(),
                gpu.config.desired_maximum_frame_latency
            ));
            let display = self
                .window
                .as_ref()
                .unwrap()
                .current_monitor()
                .map_or(0, |monitor| monitor.native_id());
            let pacer = DisplayLink::new(
                &gpu.surface,
                display,
                self.maximum_hz,
                self.inbox.dirty.source(4),
                self.pacing,
            )?;
            self.logger.write(format!(
                "pacing: {}, requested range={}/{}/{} Hz; initially paused",
                pacer.name(),
                80_f64.min(self.maximum_hz),
                self.maximum_hz,
                self.maximum_hz
            ));
            self.pacer = Some(pacer);
            self.gpu = Some(gpu);
            self.request_redraw();
        }
        Ok(())
    }
    fn drain(&mut self) {
        self.inbox.dirty.drain();
        let start = Instant::now();
        while start.elapsed() < Duration::from_millis(1) {
            let message = self.inbox.messages.try_recv();
            let output = self.inbox.output.pop();
            if message.is_none() && output.is_none() {
                break;
            }
            // These runtime notices carry no process handles or socket IO.
            if let Some(UiMsg::Exited { task, status }) = message {
                self.logger
                    .write(format!("task {} exited {:?}", task.id(), status.code));
            }
        }
        if let Some(sequence) = self.updates.take() {
            self.scene.update(sequence);
            self.last_update = sequence;
        }
        if let Some(state) = self.live_updates.take() {
            self.focus
                .reconcile(self.live_state.as_ref().map(|old| &old.ui), &state.ui);
            let refresh = self.live_state.as_ref().is_none_or(|old| {
                old.hosts != state.hosts || old.ui.seats.keys().ne(state.ui.seats.keys())
            });
            if refresh && let Some(launcher) = &mut self.launcher {
                launcher.replace_items(launcher_items(Some(&state)));
                self.launcher_layout = None;
            }
            self.scene
                .set_model(&state, self.focus.stage.as_ref().expect("published stage"));
            self.live_state = Some(state);
            self.verify();
        }
        self.flush_commands();
    }
    fn demo(&mut self) {
        if !self.fake || self.demo_started {
            return;
        }
        self.demo_started = true;
        let updates = self.updates.clone();
        let sender = self.inbox.sender.clone();
        let proxy = self.proxy.clone();
        bbt_rt::handle().spawn(async move {
            tokio::time::sleep(Duration::from_millis(300)).await;
            for sequence in 1..=20 {
                updates.publish(sequence);
                let _ = sender.try_send(UiMsg::Changed { source: 3 });
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
            // Observing update20 alone is insufficient: its Changed notice
            // may not have been posted yet if the producer was descheduled.
            let _ = proxy.send_event(UiEvent::DemoFinished);
        });
    }
    fn redraw(&mut self) -> Result<(), String> {
        self.redraw_pending.set(false);
        if self.gpu.is_none() {
            // A producer may wake us before GPU preparation finishes. Drain
            // admission even on an empty frame, or GPU-ready can lose its wake.
            self.inbox.dirty.drain();
            if self.smoke_wake_seen
                && let Some(open) = self.smoke_bootstrap.take()
            {
                self.logger
                    .write("smoke bootstrap: early producer handled before GPU ready");
                let _ = open.send(());
            }
            self.install_gpu()?;
            if self.gpu.is_none() {
                return Ok(());
            }
        }
        let frame_cpu = Instant::now();
        let size = self.window.as_ref().unwrap().inner_size();
        if size.width == 0 || size.height == 0 {
            return Ok(());
        }
        // Reconfigure at the frame boundary. AppKit may enqueue Resized from
        // inside another native callback; surface configuration can itself
        // dispatch AppKit events and must not nest that resize callback.
        if self
            .gpu
            .as_ref()
            .is_some_and(|gpu| gpu.config.width != size.width || gpu.config.height != size.height)
        {
            // Detach the display link and its old-size drawables before
            // reconfiguring the layer, then bind it to the new size.
            self.pacer.take();
            self.gpu.as_mut().unwrap().resize(size.width, size.height);
            let gpu = self.gpu.as_ref().unwrap();
            let display = self
                .window
                .as_ref()
                .unwrap()
                .current_monitor()
                .map_or(0, |monitor| monitor.native_id());
            let pacer = DisplayLink::new(
                &gpu.surface,
                display,
                self.maximum_hz,
                self.inbox.dirty.source(4),
                self.pacing,
            )?;
            pacer.set_active(self.animate && !self.occluded)?;
            self.pacer = Some(pacer);
            self.clock.pause();
            self.request_redraw();
        }
        let mut phases = Phases::default();
        let start = Instant::now();
        self.drain();
        // Drain the coalesced dirty flag before consuming its latest tick, so a
        // concurrent CV callback cannot publish a tick without a future wake.
        let tick = self.pacer.as_ref().and_then(DisplayLink::take_tick);
        let fifo = self.pacer.as_ref().is_none_or(DisplayLink::is_fifo);
        let metal = self.pacer.as_ref().is_some_and(DisplayLink::is_metal);
        if self.occluded
            || (metal && tick.is_none())
            || (self.animate && self.animation_start.is_some() && !fifo && tick.is_none())
        {
            return Ok(());
        }
        phases.drain = start.elapsed();
        let scale = self.window.as_ref().unwrap().scale_factor() as f32;
        let logical = [size.width as f32 / scale, size.height as f32 / scale];
        let start = Instant::now();
        let layouts = self
            .scene
            .layout(logical, self.animation_start.is_some() || self.animate)?;
        let bounds = self.scene.pane_bounds(&layouts);
        self.focus_bounds = self.scene.focus_bounds(&layouts);
        if let Some(model) = &self.live_state {
            for event in self.focus.layout_ready(&model.ui, &self.focus_bounds) {
                self.logger.write(format!("ui input: {event:?}"));
            }
        }
        self.fit_verify_grid(&bounds, scale);
        self.watch_panes(&bounds, scale);
        self.pane_bounds = bounds;
        self.launcher_layout = self
            .launcher
            .as_ref()
            .map(|launcher| self.scene.launcher_layout(logical, &launcher.launcher))
            .transpose()?;
        if let Some(launcher) = &self.launcher_layout {
            let caret = launcher.candidate_area();
            self.window.as_ref().unwrap().set_ime_cursor_area(
                winit::dpi::LogicalPosition::new(caret.x, caret.y),
                winit::dpi::LogicalSize::new(caret.w, caret.h),
            );
        }
        phases.layout = start.elapsed();
        let overlay = self.overlay.then(|| {
            perf_hud(
                self.meter
                    .perf_record(Instant::now(), self.budget.gpu_mean(bbt_rt::HUD_FRAMES)),
            )
        });
        let start = Instant::now();
        let live = self.live_state.is_some();
        let ops = self.scene.paint(
            &layouts,
            logical,
            if live { None } else { overlay.as_ref() },
            if live {
                None
            } else {
                self.launcher_layout.as_ref()
            },
        );
        phases.draw = start.elapsed();
        if self.animate {
            if fifo {
                self.frozen_ms = self
                    .animation_start
                    .map_or(0., |(start, _)| start.elapsed().as_secs_f32() * 1000.);
            } else if let Some(time) = tick
                .as_ref()
                .and_then(|tick| self.clock.observe(tick.target))
            {
                self.frozen_ms = time;
            }
        }
        if let Some((encode, wait)) = self.gpu.as_mut().unwrap().draw(
            ops,
            scale,
            self.frozen_ms,
            tick.and_then(|tick| tick.drawable),
            self.meter.total_frames() + 1,
            self.live_state.as_ref().map(|model| gpu::Panes {
                model,
                bounds: &self.pane_bounds,
                overlay: self.scene.foreground(
                    logical,
                    self.launcher_layout.as_ref(),
                    overlay.as_ref(),
                ),
            }),
        )? {
            phases.encode = encode;
            phases.present_wait = wait;
            if let Some((phase, time)) = self.meter.record(Instant::now(), phases) {
                self.logger.write(format!("hitch frame={} phase={phase} {:.3}ms cpu={:.3}ms drain={:.3} layout={:.3} draw={:.3} encode={:.3} present_wait={:.3}",self.meter.total_frames(),time.as_secs_f64()*1000.,phases.cpu().as_secs_f64()*1000.,phases.drain.as_secs_f64()*1000.,phases.layout.as_secs_f64()*1000.,phases.draw.as_secs_f64()*1000.,phases.encode.as_secs_f64()*1000.,phases.present_wait.as_secs_f64()*1000.));
            }
            if self.animation_start.is_some() && self.animate {
                // The Metal path never waits for acquisition: include its full
                // redraw wall time, including overlay/import/present overhead.
                let cpu =
                    frame_cpu
                        .elapsed()
                        .saturating_sub(if metal { Duration::ZERO } else { wait });
                self.budget.cpu(self.meter.total_frames(), cpu);
            }
            self.gpu.as_ref().unwrap().collect_gpu(&mut self.budget);
            if !self.animate {
                self.pacer.as_ref().unwrap().set_active(false)?;
            }
            if self.meter.total_frames() == 1 {
                self.logger.write(format!(
                    "first frame: native scene presented, seats={}",
                    self.scene.seats.len()
                ));
                self.demo();
                if self.animate {
                    self.animation_start = Some((Instant::now(), self.meter.total_frames()));
                    self.pacer.as_mut().unwrap().set_active(!self.occluded)?;
                    if self.cadence_test {
                        let proxy = self.proxy.clone();
                        bbt_rt::handle().spawn(async move {
                            tokio::time::sleep(Duration::from_secs(3)).await;
                            let _ = proxy.send_event(UiEvent::EndAnimation);
                        });
                    }
                }
            }
            if (!self.fake || self.last_update == 20)
                && self.capture_pending
                && (self.verify.grid_size.is_none() || self.verify.grid_ready)
                && let Some(path) = &self.capture
            {
                self.capture_pending = false;
                self.gpu
                    .as_ref()
                    .unwrap()
                    .capture(path.clone(), self.logger.clone());
                if self.verify.grid_hash {
                    self.dump_grid_hash(path.clone(), scale);
                }
            }
            if self.fake && self.last_update == 20 && !self.idle_scheduled {
                self.idle_scheduled = true;
                self.schedule_idle_check();
            }
        }
        if (self.animate && fifo)
            || (!self.backend_lost && !self.pending_commands.is_empty())
            || !self.inbox.messages.is_empty()
            || !self.inbox.output.is_empty()
        {
            self.request_redraw();
        }
        Ok(())
    }
}
impl ApplicationHandler<UiEvent> for App {
    fn about_to_wait(&mut self, _: &ActiveEventLoop) {
        if let Some(size) = self.pending_resize.take()
            && let Some(window) = &self.window
        {
            let _ = window.request_inner_size(size);
        }
    }
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        event_loop.set_control_flow(ControlFlow::Wait);
        if self.window.is_none()
            && let Err(error) = self.init(event_loop)
        {
            self.fail(error, event_loop);
        }
    }
    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: UiEvent) {
        match event {
            UiEvent::DemoFinished => self.demo_finished = true,
            UiEvent::Capture => {
                self.capture_pending = true;
                self.request_redraw();
            }
            UiEvent::VerifyGridDeadline => {
                if self.verify.grid_size.is_some() && !self.verify.grid_ready && !self.closing {
                    let error = format!(
                        "verification-grid-size-unreachable: expected {:?}",
                        self.verify.grid_size
                    );
                    self.logger.write(&error);
                    self.error = Some(error);
                    self.request_close(event_loop);
                }
            }
            UiEvent::VerifyFailed(error) => {
                self.logger.write(&error);
                self.error = Some(error);
                self.request_close(event_loop);
            }
            UiEvent::Wake => {
                if self.smoke && self.gpu.is_none() {
                    self.smoke_wake_seen = true;
                }
                if let Err(error) = self.install_gpu() {
                    self.fail(error, event_loop);
                    return;
                }
                self.request_redraw();
            }
            UiEvent::BeginIdle => {
                let counts = (
                    self.meter.total_frames(),
                    self.pacer.as_ref().map_or(0, DisplayLink::callbacks),
                );
                let queued = !self.inbox.messages.is_empty() || !self.inbox.output.is_empty();
                if self.fake
                    && !self.idle_quiet.ready(
                        self.demo_finished && self.last_update == 20,
                        queued || self.redraw_pending.get() || self.occluded,
                        counts,
                    )
                {
                    self.logger.write(format!(
                        "idle probe settling: producer_done={} queued={} redraw_pending={} occluded={} frames={} callbacks={}",
                        self.demo_finished, queued, self.redraw_pending.get(), self.occluded, counts.0, counts.1,
                    ));
                    if queued {
                        self.request_redraw();
                    }
                    self.schedule_idle_check();
                    return;
                }
                if self.cadence_test {
                    self.gpu.as_ref().unwrap().collect_gpu(&mut self.budget);
                    self.logger.write(self.budget.report());
                }
                let epoch = self.idle_quiet.begin(
                    Instant::now(),
                    (
                        self.meter.total_frames(),
                        self.pacer.as_ref().map_or(0, DisplayLink::callbacks),
                    ),
                );
                self.logger.write(format!(
                    "idle probe begins: frames={} last_update={}",
                    self.meter.total_frames(),
                    self.last_update
                ));
                let proxy = self.proxy.clone();
                bbt_rt::handle().spawn(async move {
                    tokio::time::sleep(Duration::from_secs(10)).await;
                    let _ = proxy.send_event(UiEvent::EndIdle(epoch));
                });
            }
            UiEvent::EndAnimation => {
                self.animate = false;
                self.clock.pause();
                let pacer = self.pacer.as_mut().expect("pacer installed");
                if let Err(error) = pacer.set_active(false) {
                    self.fail(error, event_loop);
                    return;
                }
                let (start, frames) = self.animation_start.expect("animation started");
                let seconds = start.elapsed().as_secs_f64();
                let count = self.meter.total_frames() - frames;
                self.logger.write(format!("cadence {}: frames={count} callbacks={} seconds={seconds:.6} hz={:.3} CPU_p50={:.3}ms CPU_p99={:.3}ms",pacer.name(),pacer.callbacks(), count as f64 / seconds, self.meter.cpu_percentile(0.5).as_secs_f64()*1000., self.meter.cpu_percentile(0.99).as_secs_f64()*1000.));
                let mut intervals: Vec<_> = self
                    .meter
                    .samples()
                    .iter()
                    .skip(2)
                    .filter_map(|s| s.interval)
                    .collect();
                intervals.sort_unstable();
                let dropped = intervals
                    .iter()
                    .filter(|time| time.as_secs_f64() > 1.5 / self.maximum_hz)
                    .count();
                self.logger.write(format!("cadence intervals (initial tick excluded): samples={} p50={:.3}ms p99={:.3}ms max={:.3}ms late_intervals={dropped} (>1.5 panel intervals)",intervals.len(),intervals.get(intervals.len()/2).copied().unwrap_or_default().as_secs_f64()*1000.,intervals.get((intervals.len()*99/100).min(intervals.len().saturating_sub(1))).copied().unwrap_or_default().as_secs_f64()*1000.,intervals.last().copied().unwrap_or_default().as_secs_f64()*1000.));
                let proxy = self.proxy.clone();
                bbt_rt::handle().spawn(async move {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    let _ = proxy.send_event(UiEvent::BeginIdle);
                });
            }
            UiEvent::EndIdle(epoch) => {
                let Some((start, frames, callbacks)) = self.idle_quiet.finish(epoch) else {
                    if self.smoke {
                        self.logger.write("idle probe ignored stale end timer");
                    }
                    return;
                };
                let delta = self.meter.total_frames() - frames;
                let callbacks = self.pacer.as_ref().map_or(0, DisplayLink::callbacks) - callbacks;
                let passed =
                    delta == 0 && callbacks == 0 && (self.cadence_test || self.last_update == 20);
                // Reporting patch supplied by Nyx: retain fast warm encode
                // values too, rather than relying on over-budget hitch logs.
                let mut encode: Vec<_> = self
                    .meter
                    .samples()
                    .iter()
                    .skip(1)
                    .map(|s| s.phases.encode)
                    .collect();
                encode.sort_unstable();
                let encode_percentile = |p: f64| {
                    encode
                        .get(((encode.len() as f64 * p).ceil() as usize).saturating_sub(1))
                        .copied()
                        .unwrap_or_default()
                        .as_secs_f64()
                        * 1000.
                };
                let text = format!(
                    "idle probe {}: idle {:.3}s frames={} callbacks={} updates={} CPU p50={:.3}ms p99={:.3}ms samples={} warm_encode_p50={:.3}ms warm_encode_p99={:.3}ms",
                    if passed { "PASS" } else { "FAIL" },
                    start.elapsed().as_secs_f64(),
                    delta,
                    callbacks,
                    self.last_update,
                    self.meter.cpu_percentile(0.5).as_secs_f64() * 1000.,
                    self.meter.cpu_percentile(0.99).as_secs_f64() * 1000.,
                    self.meter.samples().len(),
                    encode_percentile(0.5),
                    encode_percentile(0.99)
                );
                if !passed {
                    self.error = Some("R2 idle/update probe failed".into());
                }
                if self.smoke || self.cadence_test {
                    self.logger.finish(text, self.proxy.clone());
                } else {
                    self.logger.write(text);
                }
            }
            UiEvent::Quit => event_loop.exit(),
            UiEvent::SmokeDeadline => self.fail(self.idle_quiet.deadline_error(), event_loop),
            UiEvent::BackendStopped(result) => {
                if self.admission_refusals.input != 0 || self.admission_refusals.spawn != 0 {
                    let mut report = self.admission_refusals.report();
                    if let Err(error) = result {
                        report = format!("app error: {error}; {report}");
                        self.error = Some(error);
                    }
                    // Unlike best-effort per-event diagnostics, this final
                    // cumulative count is flushed on IO before Quit is sent.
                    self.logger.finish(report, self.proxy.clone());
                } else {
                    match result {
                        Ok(()) => event_loop.exit(),
                        Err(error) => self.fail(error, event_loop),
                    }
                }
            }
            UiEvent::BackendLost(reason) => {
                self.logger
                    .write(format!("backend lost; retaining last pane frame: {reason}"));
                self.backend_lost = true;
                self.scene.backend_lost();
                self.request_redraw();
            }
            UiEvent::BackendReady => {
                self.backend_lost = false;
                self.scene.backend_ready();
                self.flush_commands();
                self.request_redraw();
            }
        }
    }
    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                self.request_close(event_loop);
            }
            WindowEvent::Resized(size) => {
                self.native_idle_invalidation("Resized");
                if self.capture.is_some() {
                    self.logger.write(format!(
                        "verification window resize: {}x{} physical pixels",
                        size.width, size.height
                    ));
                }
                if let Some(pacer) = &self.pacer {
                    pacer.clear();
                }
                self.request_redraw();
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                self.native_idle_invalidation("ScaleFactorChanged");
                self.request_redraw();
            }
            WindowEvent::Occluded(occluded) => {
                self.native_idle_invalidation(if occluded {
                    "Occluded(true)"
                } else {
                    "Occluded(false)"
                });
                self.occluded = occluded;
                if let Some(pacer) = &mut self.pacer
                    && let Err(error) = pacer.set_active(self.animate && !occluded)
                {
                    self.fail(error, event_loop);
                    return;
                }
                self.clock.pause();
                if !occluded {
                    self.request_redraw();
                }
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.modifiers = modifiers.state();
                self.keyboard.modifiers_changed(modifiers);
            }
            WindowEvent::CursorMoved { position, .. } => {
                let scale = self
                    .window
                    .as_ref()
                    .map_or(1., |window| window.scale_factor()) as f32;
                self.pointer = [position.x as f32 / scale, position.y as f32 / scale];
                if self.query_dragging {
                    if let (Some(layout), Some(launcher)) = (&self.launcher_layout, &self.launcher)
                    {
                        let at = self.scene.query_index(
                            layout,
                            &launcher.launcher.query,
                            self.pointer[0],
                        );
                        self.launcher_command(LauncherCommand::SetCaret { at, extend: true });
                    }
                    return;
                }
                if let Some(index) = self
                    .launcher_layout
                    .as_ref()
                    .and_then(|layout| layout.row_at(self.pointer))
                {
                    self.launcher_command(LauncherCommand::Hover(index));
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                if self.launcher.is_some() {
                    if let Some(layout) = &self.launcher_layout {
                        if layout.query_contains(self.pointer) {
                            let at = self.scene.query_index(
                                layout,
                                &self.launcher.as_ref().unwrap().launcher.query,
                                self.pointer[0],
                            );
                            self.query_dragging = true;
                            self.launcher_command(LauncherCommand::SetCaret {
                                at,
                                extend: self.modifiers.shift_key(),
                            });
                        } else if let Some(index) = layout.row_at(self.pointer) {
                            self.launcher_command(LauncherCommand::ActivateAt {
                                index,
                                modifiers: input_modifiers(self.modifiers),
                            });
                        } else if !layout.contains(self.pointer) {
                            self.close_launcher();
                        }
                    }
                    return;
                }
                if let Some((id, _)) = self.focus_bounds.iter().find(|(_, rect)| {
                    self.pointer[0] >= rect.x
                        && self.pointer[0] < rect.right()
                        && self.pointer[1] >= rect.y
                        && self.pointer[1] < rect.bottom()
                }) {
                    self.focus_action(FocusAction::Activate(*id, FocusOrigin::Pane));
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Released,
                button: MouseButton::Left,
                ..
            } => self.query_dragging = false,
            WindowEvent::Ime(Ime::Preedit(text, cursor)) => {
                self.launcher_command(preedit_command(text, cursor));
            }
            WindowEvent::Ime(Ime::Disabled) => self.launcher_command(LauncherCommand::Preedit {
                text: String::new(),
                caret: None,
            }),
            WindowEvent::Ime(Ime::Commit(text)) => {
                self.launcher_command(LauncherCommand::Commit(text));
            }
            WindowEvent::KeyboardInput { event, .. } => {
                let input = self.keyboard.event(&event, self.modifiers);
                if event.state.is_pressed() {
                    self.focus.controller.key_pressed(Instant::now());
                }
                // Verification-only resize exercises the real OS Resized event
                // on borderless windows whose native edge is hard to grab.
                if event.state.is_pressed()
                    && self.capture.is_some()
                    && self.modifiers.super_key()
                    && self.modifiers.shift_key()
                    && event.physical_key == PhysicalKey::Code(KeyCode::KeyR)
                    && !event.repeat
                {
                    if let Some(window) = &self.window {
                        let width = window.inner_size().width as f64 / window.scale_factor();
                        let size = if width > 1200. {
                            LogicalSize::new(1200., 760.)
                        } else {
                            LogicalSize::new(1600., 960.)
                        };
                        self.pending_resize = Some(size);
                    }
                    self.keyboard.consumed(event.physical_key, event.state);
                    return;
                }
                // The existing --capture path can be refreshed after an
                // interactive verification scene (no readback on the UI).
                if event.state.is_pressed()
                    && self.capture.is_some()
                    && self.modifiers.super_key()
                    && self.modifiers.shift_key()
                    && event.physical_key == PhysicalKey::Code(KeyCode::KeyS)
                    && !event.repeat
                {
                    self.capture_pending = true;
                    self.request_redraw();
                    self.keyboard.consumed(event.physical_key, event.state);
                    return;
                }
                let modes = self
                    .focus
                    .stage
                    .as_ref()
                    .and_then(|stage| stage.focused)
                    .and_then(|seat| {
                        self.live_state
                            .as_ref()?
                            .screens
                            .get(&seat)
                            .map(|screen| screen.modes)
                    });
                let routed = route_input(
                    &mut self.keyboard,
                    &input,
                    modes.unwrap_or_default(),
                    self.launcher
                        .as_ref()
                        .map(|launcher| launcher.launcher.query.as_str()),
                );
                match routed {
                    RoutedKey::Quit => self.request_close(event_loop),
                    RoutedKey::OpenLauncher => self.open_launcher(),
                    RoutedKey::ToggleHud => {
                        self.overlay = !self.overlay;
                        self.logger.write(format!("hitch overlay={}", self.overlay));
                        self.request_redraw();
                    }
                    RoutedKey::Launcher(command) => self.launcher_command(command),
                    RoutedKey::Pane(navigation) => {
                        self.focus_action(FocusAction::Navigate(navigation))
                    }
                    RoutedKey::Terminal(bytes) => {
                        // No terminal input before the focused live Screen supplies modes.
                        if modes.is_some()
                            && let Some(seat) =
                                self.focus.stage.as_ref().and_then(|stage| stage.focused)
                            && let Some(generation) = self
                                .live_state
                                .as_ref()
                                .and_then(|model| model.ui.seats.get(&seat))
                                .map(|seat| seat.process_generation)
                        {
                            self.queue(live::UiMsg::Input {
                                seat,
                                generation,
                                id: bbt_mux::Ulid::new(),
                                bytes,
                            });
                        }
                    }
                    RoutedKey::Local => {}
                }
            }
            WindowEvent::RedrawRequested => {
                if let Err(error) = self.redraw() {
                    self.fail(error, event_loop);
                }
            }
            _ => {}
        }
    }
}
const PENDING_INPUT_BYTES: usize = 1024 * 1024;
const PENDING_INPUT_ENTRIES: usize = 256;
const PENDING_SPAWNS: usize = 8;

#[derive(Debug, PartialEq, Eq)]
enum AdmissionRefused {
    Input,
    Spawn,
}

#[derive(Default)]
struct AdmissionRefusals {
    input: u64,
    spawn: u64,
}
impl AdmissionRefusals {
    fn record(&mut self, kind: AdmissionRefused) {
        let count = match kind {
            AdmissionRefused::Input => &mut self.input,
            AdmissionRefused::Spawn => &mut self.spawn,
        };
        *count = count.saturating_add(1);
    }
    fn report(&self) -> String {
        format!(
            "admission refused: input={} spawn={} (pending input limit={} bytes/{} entries; spawn limit={})",
            self.input, self.spawn, PENDING_INPUT_BYTES, PENDING_INPUT_ENTRIES, PENDING_SPAWNS,
        )
    }
}

fn queue_pending(
    pending: &mut VecDeque<live::UiMsg>,
    message: live::UiMsg,
) -> Result<(), AdmissionRefused> {
    if let live::UiMsg::Input { bytes, .. } = &message {
        let mut input_bytes = 0;
        let mut input_entries = 0;
        for old in pending.iter() {
            if let live::UiMsg::Input { bytes, .. } = old {
                input_bytes += bytes.len();
                input_entries += 1;
            }
        }
        if bytes.len() > PENDING_INPUT_BYTES.saturating_sub(input_bytes) {
            return Err(AdmissionRefused::Input);
        }
        // A merge does not consume another entry, even at the entry cap.
        if let live::UiMsg::Input {
            seat, generation, ..
        } = &message
            && let Some(live::UiMsg::Input {
                seat: old_seat,
                generation: old_generation,
                bytes: old_bytes,
                ..
            }) = pending.back_mut()
            && seat == old_seat
            && generation == old_generation
            && old_bytes.len() + bytes.len() <= 16 * 1024
        {
            old_bytes.extend_from_slice(bytes);
            return Ok(());
        }
        if input_entries >= PENDING_INPUT_ENTRIES {
            return Err(AdmissionRefused::Input);
        }
    }
    match &message {
        live::UiMsg::Resize { seat, .. } => pending.retain(
            |old| !matches!(old, live::UiMsg::Resize { seat: old_seat, .. } if old_seat == seat),
        ),
        // Watched carries the whole visible set, including focused-first
        // ordering. Keep only the latest set and sizes of its visible panes.
        live::UiMsg::Watched(panes) => pending.retain(|old| match old {
            live::UiMsg::Watched(_) => false,
            live::UiMsg::Resize { seat, .. } => panes.iter().any(|(id, _)| id == seat),
            _ => true,
        }),
        message
            if message.is_spawn()
                && pending.iter().filter(|old| old.is_spawn()).count() >= PENDING_SPAWNS =>
        {
            return Err(AdmissionRefused::Spawn);
        }
        _ => {}
    }
    pending.push_back(message);
    Ok(())
}
fn flush_pending(
    sender: &tokio::sync::mpsc::Sender<live::UiMsg>,
    pending: &mut VecDeque<live::UiMsg>,
    paused: bool,
) -> Result<(), ()> {
    if paused {
        return Ok(());
    }
    while let Some(message) = pending.pop_front() {
        match sender.try_send(message) {
            Ok(()) => {}
            Err(tokio::sync::mpsc::error::TrySendError::Full(message)) => {
                pending.push_front(message);
                return Ok(());
            }
            Err(tokio::sync::mpsc::error::TrySendError::Closed(message)) => {
                pending.push_front(message);
                return Err(());
            }
        }
    }
    Ok(())
}
#[cfg(test)]
fn test_key<'a>(
    key: &Key,
    physical: PhysicalKey,
    text: Option<&'a str>,
    mods: ModifiersState,
    repeat: bool,
) -> keyboard::Input<'a> {
    keyboard::Input {
        logical_key: key.clone(),
        physical_key: physical,
        unmodified_key: key.clone(),
        text,
        // These legacy fixture arguments contain no platform Ctrl supplement.
        text_all_modifiers: text.filter(|_| !mods.control_key()),
        location: winit::keyboard::KeyLocation::Standard,
        state: ElementState::Pressed,
        repeat,
        modifiers: keyboard::Modifiers {
            state: mods,
            ..Default::default()
        },
    }
}
#[cfg(test)]
fn terminal_input(key: &Key, text: Option<&str>, mods: ModifiersState) -> Option<Vec<u8>> {
    keyboard::encode(
        &test_key(key, PhysicalKey::Code(KeyCode::Space), text, mods, false),
        0,
    )
}
/// The meter's window in the HUD's record shape (the lab's field names).
fn perf_hud(r: bbt_rt::PerfRecord) -> bbt_ui::modules::perf_hud::PerfHud {
    use bbt_ui::modules::perf_hud::{PerfCpu, PerfFrame, PerfHitch, PerfHud, PerfPhases};
    let phases = |p: [f64; 5]| PerfPhases {
        drain: p[0],
        layout: p[1],
        draw: p[2],
        encode: p[3],
        present_wait: p[4],
    };
    PerfHud {
        hz: r.hz,
        interval_ms: r.interval_ms,
        budget_ms: r.budget_ms,
        cpu: PerfCpu {
            p50: r.cpu_p50,
            p99: r.cpu_p99,
        },
        gpu_ms: r.gpu_ms,
        phases: phases(r.phases),
        frames: r
            .frames
            .into_iter()
            .map(|f| match f {
                bbt_rt::HudFrame::Busy { phases: p, hitch } => PerfFrame {
                    phases: phases(p),
                    idle: false,
                    hitch: hitch.map(Into::into),
                },
                bbt_rt::HudFrame::Idle => PerfFrame {
                    idle: true,
                    ..Default::default()
                },
            })
            .collect(),
        hitch: r.hitch.map(|(phase, ms, frame)| PerfHitch {
            phase: phase.into(),
            ms,
            frame,
        }),
        late: r.late,
        idle_ms: r.idle_ms,
    }
}

const HELP: &str = "bboxy450-term: GPU terminal for agent swarms.

  --meter              open with the perf HUD (Cmd+Opt+F toggles it)
  --run-dir DIR        runtime directory (default: the user's)

Debug and verification only, never for normal use:
  --launcher           open the launcher at startup (same controller as Cmd+T)
  --capture PNG        write the first presented frame to PNG; Cmd+Shift+S re-captures
  --capture-after SECS re-capture once, SECS after the cockpit's first model
  --spawn-zsh N        open N zsh seats once the cockpit is ready (default zsh picks)
  --theme light|dark   terminal palette for live verification (default: dark)
  --grid-size COLSxROWS fit one live pane by resizing the real window; 10s deadline
  --grid-hash          write actual visible-screen hashes to PNG's .grid.json
  BBOXY_VERIFY_INPUT   env: bytes typed into each new seat once, \\n sent as Return
  --smoke, --fake-source, --animate, --cadence-test, --pacing MODE: frozen scenes and pacing probes";

/// Verification-only automation (see HELP): seats, first input and a delayed capture,
/// driven from the app's own queue so no OS input injection is needed.
#[derive(Default)]
struct Verify {
    spawn: usize,
    capture_after: Option<Duration>,
    input: Option<Vec<u8>>,
    typed: std::collections::BTreeSet<SeatId>,
    started: bool,
    theme: Option<Theme>,
    grid_size: Option<(u16, u16)>,
    grid_hash: bool,
    grid_ready: bool,
    capture_started: bool,
}
impl Verify {
    fn from_args(args: &[String]) -> Result<Self, String> {
        let value = |flag: &str| {
            args.windows(2)
                .find(|pair| pair[0] == flag)
                .map(|pair| pair[1].clone())
        };
        let required = |flag: &str| -> Result<Option<&str>, String> {
            args.iter()
                .position(|arg| arg == flag)
                .map(|index| {
                    args.get(index + 1)
                        .map(String::as_str)
                        .filter(|value| !value.starts_with("--"))
                        .ok_or_else(|| format!("verification-missing-value: {flag}"))
                })
                .transpose()
        };
        let theme = required("--theme")?
            .map(|theme| match theme {
                "light" => Ok(Theme::Light),
                "dark" => Ok(Theme::Dark),
                _ => Err(format!("verification-invalid-theme: {theme}")),
            })
            .transpose()?;
        let grid_size = required("--grid-size")?
            .map(parse_verify_grid)
            .transpose()?;
        Ok(Self {
            spawn: value("--spawn-zsh")
                .and_then(|n| n.parse().ok())
                .unwrap_or(0),
            capture_after: value("--capture-after")
                .and_then(|s| s.parse().ok())
                .map(Duration::from_secs_f64),
            input: std::env::var("BBOXY_VERIFY_INPUT")
                .ok()
                .map(|text| text.replace("\\n", "\r").into_bytes()),
            theme,
            grid_size,
            grid_hash: args.iter().any(|arg| arg == "--grid-hash"),
            ..Default::default()
        })
    }
}
impl App {
    /// Runs on every live model: the first one means the cockpit is ready.
    fn verify(&mut self) {
        if !self.verify.started {
            self.verify.started = true;
            if self.verify.grid_size.is_some() {
                let proxy = self.proxy.clone();
                bbt_rt::handle().spawn(async move {
                    tokio::time::sleep(Duration::from_secs(10)).await;
                    let _ = proxy.send_event(UiEvent::VerifyGridDeadline);
                });
            }
            for _ in 0..self.verify.spawn {
                self.queue(live::UiMsg::SpawnZsh);
            }
        }
        if self.verify.grid_size.is_some() && !self.verify.grid_ready {
            return;
        }
        // Grid verification waits for a real resize before typing. Start its
        // settling interval here, so fitting cannot consume the capture delay.
        if !self.verify.capture_started
            && let Some(after) = self.verify.capture_after
        {
            self.verify.capture_started = true;
            let proxy = self.proxy.clone();
            bbt_rt::handle().spawn(async move {
                tokio::time::sleep(after).await;
                let _ = proxy.send_event(UiEvent::Capture);
            });
        }
        let Some(bytes) = self.verify.input.clone() else {
            return;
        };
        let fresh: Vec<_> = self
            .live_state
            .iter()
            .flat_map(|model| model.ui.seats.iter())
            .filter(|(id, _)| !self.verify.typed.contains(id))
            .map(|(id, seat)| (*id, seat.process_generation))
            .collect();
        for (seat, generation) in fresh {
            self.verify.typed.insert(seat);
            self.queue(live::UiMsg::Input {
                seat,
                generation,
                id: bbt_mux::Ulid::new(),
                bytes: bytes.clone(),
            });
        }
    }

    fn fit_verify_grid(&mut self, bounds: &[(SeatId, Rect)], scale: f32) {
        let Some(wanted) = self.verify.grid_size else {
            return;
        };
        self.verify.grid_ready = false;
        let [(seat, rect)] = bounds else {
            return;
        };
        let Some(model) = &self.live_state else {
            return;
        };
        let (Some(screen), Some(frame)) = (model.screens.get(seat), model.frames.get(seat)) else {
            return;
        };
        let cell = [
            frame.width as f32 / screen.cols as f32 / scale,
            frame.height as f32 / screen.rows as f32 / scale,
        ];
        let window = self.window.as_ref().unwrap();
        let size = window.inner_size();
        if let Some(size) = fit_grid_window(
            [size.width as f32 / scale, size.height as f32 / scale],
            *rect,
            cell,
            wanted,
        ) {
            // about_to_wait applies this outside the native resize callback.
            self.pending_resize = Some(size);
        } else if (screen.cols, screen.rows) == (usize::from(wanted.0), usize::from(wanted.1)) {
            self.verify.grid_ready = true;
            self.verify();
        }
    }

    fn dump_grid_hash(&self, png: std::path::PathBuf, scale: f32) {
        let Some(model) = self.live_state.clone() else {
            return;
        };
        let bounds = self.pane_bounds.clone();
        let frame_number = self.meter.total_frames();
        let logger = self.logger.clone();
        let proxy = self.proxy.clone();
        bbt_rt::handle().spawn(async move {
            let panes: Vec<_> = bounds
                .iter()
                .filter_map(|(seat, rect)| {
                    let screen = model.screens.get(seat)?;
                    let frame = model.frames.get(seat)?;
                    Some(
                    serde_json::json!({"seat":bbt_mux::Ulid::from(seat.0.0).to_string(),"cols":screen.cols,"rows":screen.rows,
                    "cursor":screen.cursor,"modes":screen.modes,"fnv1a64":screen_hash(screen),
                    "crop":[(rect.x*scale).round() as u32,(rect.y*scale).round() as u32,
                        frame.width.min((rect.w*scale).floor() as u32),
                        frame.height.min((rect.h*scale).floor() as u32)]}),
                    )
                })
                .collect();
            let theme = match model.ui.window.theme { Theme::Dark => "dark", Theme::Light => "light" };
            let report = serde_json::json!({"schema":"bboxy-firehose-screen-v1","png":png,
                "theme":theme,"scale":scale,"frame":frame_number,"panes":panes});
            let path = png.with_extension("grid.json");
            let result = tokio::fs::write(&path, serde_json::to_vec_pretty(&report).unwrap()).await;
            logger.write(format!("grid-hash {}: {result:?}", path.display()));
            if let Err(error) = result {
                let _ = proxy.send_event(UiEvent::VerifyFailed(format!(
                    "verification-grid-hash-write: {error}"
                )));
            }
        });
    }
}

fn parse_verify_grid(value: &str) -> Result<(u16, u16), String> {
    let size = value
        .split_once('x')
        .and_then(|(cols, rows)| Some((cols.parse::<u16>().ok()?, rows.parse::<u16>().ok()?)));
    size.filter(|&(cols, rows)| (1..=512).contains(&cols) && (1..=256).contains(&rows))
        .ok_or_else(|| format!("verification-invalid-grid-size: {value}"))
}

fn fit_grid_window(
    window: [f32; 2],
    pane: Rect,
    cell: [f32; 2],
    wanted: (u16, u16),
) -> Option<LogicalSize<f64>> {
    let actual = [
        (pane.w / cell[0]).floor() as u16,
        (pane.h / cell[1]).floor() as u16,
    ];
    if actual == [wanted.0, wanted.1] {
        return None;
    }
    // Aim halfway through the integer-cell bin, avoiding rounding oscillation.
    let fit = |current: f32, body: f32, cell: f32, cells: u16, actual: u16| {
        f64::from(if cells == actual {
            current
        } else {
            (current + (f32::from(cells) + 0.5) * cell - body).max(1.)
        })
    };
    Some(LogicalSize::new(
        fit(window[0], pane.w, cell[0], wanted.0, actual[0]),
        fit(window[1], pane.h, cell[1], wanted.1, actual[1]),
    ))
}

/// Exact schema documented in bboxy-firehose/README.md (no pixel/theme data).
fn screen_hash(screen: &bbt_proto::Screen) -> String {
    use bbt_core::Color;
    let color = |color| match color {
        Color::Indexed(index) => serde_json::json!([0, index]),
        Color::Named(name) => serde_json::json!([1, name as u16]),
        Color::Spec(rgb) => serde_json::json!([2, rgb.r, rgb.g, rgb.b]),
    };
    let cells: Vec<_> = screen
        .cells
        .iter()
        .flatten()
        .map(|cell| {
            serde_json::json!([
                cell.c as u32,
                color(cell.fg),
                color(cell.bg),
                cell.flags.bits(),
                cell.zerowidth()
                    .unwrap_or_default()
                    .iter()
                    .map(|c| *c as u32)
                    .collect::<Vec<_>>(),
                cell.underline_color().map(color),
                cell.hyperlink()
                    .map(|link| [link.id().to_owned(), link.uri().to_owned()])
            ])
        })
        .collect();
    let bytes = serde_json::to_vec(&serde_json::json!([
        screen.cols,
        screen.rows,
        screen.cursor.0,
        screen.cursor.1,
        screen.modes,
        cells
    ]))
    .unwrap();
    let hash = bytes.iter().fold(0xcbf29ce484222325u64, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
    });
    format!("{hash:016x}")
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let hook_args: Vec<_> = std::env::args().skip(1).collect();
    if hook_args.iter().any(|arg| arg == "--help" || arg == "-h") {
        println!("{HELP}");
        return Ok(());
    }
    if hook_args.iter().any(|arg| arg == "--capture") {
        std::panic::set_hook(Box::new(|info| {
            eprintln!(
                "verification panic payload: {:?}; location: {:?}",
                info.payload()
                    .downcast_ref::<&str>()
                    .copied()
                    .or_else(|| info.payload().downcast_ref::<String>().map(String::as_str)),
                info.location()
            );
            eprintln!("{}", std::backtrace::Backtrace::force_capture());
        }));
    }
    if let Some(result) = bboxy450_term::cockpit::private_entry(&hook_args) {
        return result.map_err(|error| error as Box<dyn std::error::Error>);
    }
    if hook_args.first().is_some_and(|arg| arg == "hook") {
        bbt_mux::adapters::hook::run_cli(&hook_args[1..]);
        return Ok(());
    }
    if hook_args.first().is_some_and(|arg| arg == "mcp") {
        bbt_mux::mcp::run_cli()?;
        return Ok(());
    }
    let verify = Verify::from_args(&hook_args)?;
    let mut event_loop_builder = EventLoop::<UiEvent>::with_user_event();
    #[cfg(target_os = "macos")]
    {
        use winit::platform::macos::EventLoopBuilderExtMacOS;
        // The default Quit menu sends NSApplication::terminate before our
        // async child shutdown can run. Route Command-Q through App instead.
        event_loop_builder.with_default_menu(false);
    }
    let event_loop = event_loop_builder.build()?;
    let proxy = event_loop.create_proxy();
    let inbox = bbt_rt::init_ui(Wake(proxy.clone()));
    let prepared = Arc::new(Latest::new(inbox.dirty.source(2)));
    let updates = Arc::new(Latest::new(inbox.dirty.source(3)));
    let live_updates = Arc::new(Latest::new(inbox.dirty.source(5)));
    let smoke = std::env::args().any(|arg| arg == "--smoke");
    if smoke {
        let timeout_proxy = proxy.clone();
        bbt_rt::handle().spawn(async move {
            tokio::time::sleep(Duration::from_secs(30)).await;
            let _ = timeout_proxy.send_event(UiEvent::SmokeDeadline);
        });
    }
    let fake = smoke || std::env::args().any(|arg| arg == "--fake-source");
    let overlay = std::env::args().any(|arg| arg == "--meter");
    let args: Vec<_> = std::env::args().collect();
    let cadence_test = args.iter().any(|arg| arg == "--cadence-test");
    let animate = cadence_test || args.iter().any(|arg| arg == "--animate");
    if cadence_test && fake {
        return Err("--cadence-test and fake-source/smoke are separate probes".into());
    }
    let pacing = Mode::parse(
        args.windows(2)
            .find(|pair| pair[0] == "--pacing")
            .map_or("metal", |pair| pair[1].as_str()),
    )?;
    let capture = args
        .windows(2)
        .find(|pair| pair[0] == "--capture")
        .map(|pair| std::path::PathBuf::from(&pair[1]));
    let logger = log::Logger::new();
    let (render_context, rendering) = tokio::sync::oneshot::channel();
    let live_commands = if fake || animate {
        None
    } else {
        Some(live::start(
            live_updates.clone(),
            proxy.clone(),
            logger.clone(),
            args.windows(2)
                .find(|pair| pair[0] == "--run-dir")
                .map(|pair| std::path::PathBuf::from(&pair[1])),
            rendering,
        ))
    };
    let mut app = App {
        proxy,
        inbox,
        logger,
        prepared,
        updates,
        live_updates,
        live_state: None,
        live_commands,
        pending_commands: VecDeque::new(),
        admission_refusals: AdmissionRefusals::default(),
        watched: vec![],
        pane_sizes: BTreeMap::new(),
        pane_bounds: vec![],
        focus_bounds: vec![],
        focus: FocusState::default(),
        launcher: args
            .iter()
            .any(|arg| arg == "--launcher")
            .then(|| new_launcher(None, 0)),
        launcher_layout: None,
        pointer: [0.; 2],
        query_dragging: false,
        backend_lost: false,
        render_context: (!fake && !animate).then_some(render_context),
        closing: false,
        smoke_bootstrap: None,
        smoke_wake_seen: false,
        window: None,
        pacer: None,
        pacing,
        clock: Clock::default(),
        occluded: false,
        maximum_hz: 60.,
        budget: budget::Budget::default(),
        gpu: None,
        scene: if fake || animate {
            scene::Scene::new()
        } else {
            scene::Scene::live()
        },
        meter: HitchMeter::new(120.),
        modifiers: ModifiersState::empty(),
        keyboard: keyboard::State::default(),
        overlay,
        fake,
        smoke,
        demo_started: false,
        demo_finished: false,
        idle_scheduled: false,
        idle_quiet: IdleQuiescence::default(),
        redraw_pending: std::cell::Cell::new(false),
        last_update: 0,
        error: None,
        capture,
        capture_pending: true,
        pending_resize: None,
        animate,
        cadence_test,
        animation_start: None,
        frozen_ms: 450.,
        verify,
    };
    event_loop.run_app(&mut app)?;
    if let Some(error) = app.error {
        return Err(error.into());
    }
    Ok(())
}

#[cfg(test)]
mod verification_tests {
    use super::*;

    #[test]
    fn verification_grid_validation_and_window_fit_use_real_cell_bounds() {
        assert_eq!(parse_verify_grid("80x24").unwrap(), (80, 24));
        for invalid in ["0x24", "513x24", "80x257", "80", "80x24x2"] {
            assert!(
                parse_verify_grid(invalid)
                    .unwrap_err()
                    .starts_with("verification-invalid-grid-size:")
            );
        }
        let args = ["--theme", "light", "--grid-size", "80x24", "--grid-hash"].map(String::from);
        let verify = Verify::from_args(&args).unwrap();
        assert_eq!(verify.theme, Some(Theme::Light));
        assert_eq!(verify.grid_size, Some((80, 24)));
        assert!(verify.grid_hash);
        assert!(Verify::from_args(&["--theme".into(), "bogus".into()]).is_err());
        assert!(Verify::from_args(&["--grid-size".into()]).is_err());
        let pane = Rect::new(320., 80., 1280., 840.);
        let next = fit_grid_window([1600., 960.], pane, [8., 15.], (80, 24)).unwrap();
        assert_eq!(next, LogicalSize::new(964., 487.5));
        let fitted = Rect::new(
            pane.x,
            pane.y,
            pane.w + next.width as f32 - 1600.,
            pane.h + next.height as f32 - 960.,
        );
        assert!(
            fit_grid_window(
                [next.width as f32, next.height as f32],
                fitted,
                [8., 15.],
                (80, 24)
            )
            .is_none()
        );
    }

    #[test]
    fn live_screen_hash_matches_frozen_palette_unicode_and_link_fixtures() {
        let expected = include_str!("../../bboxy-firehose/corpus/screen-hashes.txt");
        for (name, bytes) in [
            (
                "palette",
                include_bytes!("../../bboxy-firehose/corpus/palette.ansi").as_slice(),
            ),
            (
                "unicode",
                include_bytes!("../../bboxy-firehose/corpus/unicode.ansi").as_slice(),
            ),
            (
                "scroll-links-sync",
                include_bytes!("../../bboxy-firehose/corpus/scroll-links-sync.ansi").as_slice(),
            ),
        ] {
            let mut term = bbt_core::Term::new(80, 24, 0);
            term.advance(b"\x1b[?2026l\x1b[?1049l\x1b[0m\x1b[r\x1b[2J\x1b[H");
            term.advance(bytes);
            term.advance(b"\x1b[0m\x1b[r\x1b[?2026l\x1b[?25h\x1b[24;1H");
            let screen = bbt_proto::Screen::from_term(&term, String::new(), None, None);
            let frozen = expected
                .lines()
                .find_map(|line| {
                    line.split_once(' ')
                        .filter(|(scene, _)| *scene == name)
                        .map(|(_, hash)| hash)
                })
                .unwrap();
            assert_eq!(screen_hash(&screen), frozen, "{name}");
        }
    }
}

#[cfg(test)]
mod smoke_tests {
    use super::IdleQuiescence;
    use std::time::{Duration, Instant};

    #[test]
    fn flapping_native_events_never_pass_and_deadline_has_a_named_reason() {
        let mut quiet = IdleQuiescence::default();
        assert_eq!(
            quiet.deadline_error(),
            "smoke probe did not finish within 30 seconds"
        );
        let now = Instant::now();
        let first = quiet.begin(now, (21, 21));
        for second in 0..30 {
            quiet.interrupt();
            assert!(!quiet.ready(true, false, (21 + second, 21 + second)));
            assert!(quiet.finish(first).is_none());
        }
        assert_eq!(quiet.invalidations, 30);
        assert_eq!(
            quiet.deadline_error(),
            "idle window never stable within 30 seconds (native invalidations: 30)"
        );
    }

    #[test]
    fn native_invalidation_restarts_settling_and_old_timers_cannot_end_the_new_window() {
        let mut quiet = IdleQuiescence::default();
        let now = Instant::now();
        assert!(!quiet.ready(true, false, (21, 21)));
        assert!(quiet.ready(true, false, (21, 21)));
        let first = quiet.begin(now, (21, 21));
        assert!(quiet.interrupt());
        assert!(
            !quiet.interrupt(),
            "only one replacement check is scheduled"
        );
        assert!(quiet.finish(first).is_none());
        // Four legitimate unocclusion repaints occurred. A hidden window or
        // any pending redraw must not begin the replacement measurement.
        assert!(!quiet.ready(true, true, (25, 25)));
        assert!(!quiet.ready(true, false, (25, 25)));
        assert!(quiet.ready(true, false, (25, 25)));
        let replacement_start = now + Duration::from_secs(2);
        let replacement = quiet.begin(replacement_start, (25, 25));
        assert!(
            quiet.finish(first).is_none(),
            "old timer cannot shorten the full ten seconds"
        );
        assert_eq!(quiet.finish(replacement), Some((replacement_start, 25, 25)));
        assert!(
            quiet.finish(replacement).is_none(),
            "duplicate timer is harmless"
        );
    }

    #[test]
    fn idle_waits_for_producer_queued_work_and_late_frames_before_starting() {
        let mut quiet = IdleQuiescence::default();
        // The final value was rendered, but its producer can still be paused
        // before posting the matching Changed notice. A timer is not a barrier.
        assert!(!quiet.ready(false, false, (21, 21)));
        assert!(!quiet.ready(false, false, (21, 21)));
        assert!(!quiet.ready(true, true, (21, 21)));
        // Draining the notice can leave a native redraw and then a callback.
        assert!(!quiet.ready(true, false, (22, 22)));
        assert!(!quiet.ready(true, false, (23, 22)));
        assert!(!quiet.ready(true, false, (23, 23)));
        // Pending work invalidates an earlier observation even if it has not
        // changed either counter yet. Only a full quiet interval may start idle.
        assert!(!quiet.ready(true, true, (23, 23)));
        assert!(!quiet.ready(true, false, (23, 23)));
        assert!(quiet.ready(true, false, (23, 23)));
    }
}

#[cfg(test)]
mod input_tests {
    use super::*;

    fn input(seat: SeatId, id: u128, bytes: Vec<u8>) -> live::UiMsg {
        live::UiMsg::Input {
            seat,
            generation: 7,
            id: bbt_mux::Ulid::from(id),
            bytes,
        }
    }

    #[test]
    fn admission_limits_preserve_accepted_bytes_and_allow_merging_at_the_entry_cap() {
        let seats = [SeatId::from_u128(42), SeatId::from_u128(43)];
        let mut pending = VecDeque::new();
        assert_eq!(
            queue_pending(
                &mut pending,
                input(seats[0], 1, vec![0; PENDING_INPUT_BYTES + 1])
            ),
            Err(AdmissionRefused::Input)
        );
        assert!(pending.is_empty());
        queue_pending(
            &mut pending,
            input(seats[0], 2, vec![b'x'; PENDING_INPUT_BYTES]),
        )
        .unwrap();
        assert_eq!(
            queue_pending(&mut pending, input(seats[0], 3, b"refused".to_vec())),
            Err(AdmissionRefused::Input)
        );
        assert!(
            matches!(pending.pop_front().unwrap(), live::UiMsg::Input { id, bytes, .. }
            if id == bbt_mux::Ulid::from(2) && bytes == vec![b'x'; PENDING_INPUT_BYTES])
        );

        for index in 0..PENDING_INPUT_ENTRIES {
            queue_pending(
                &mut pending,
                input(seats[index % 2], index as u128 + 1, vec![b'x']),
            )
            .unwrap();
        }
        assert_eq!(
            queue_pending(&mut pending, input(seats[0], 300, b"refused".to_vec())),
            Err(AdmissionRefused::Input)
        );
        // The existing last entry can grow without admitting another entry.
        queue_pending(&mut pending, input(seats[1], 301, b"m".to_vec())).unwrap();
        assert_eq!(pending.len(), PENDING_INPUT_ENTRIES);
        assert!(
            matches!(pending.back().unwrap(), live::UiMsg::Input { id, bytes, .. }
            if *id == bbt_mux::Ulid::from(256) && bytes == b"xm")
        );
    }

    #[test]
    fn pane_controls_coalesce_to_the_latest_visible_set_and_size() {
        let seats = [SeatId::from_u128(42), SeatId::from_u128(43)];
        let view = |seat: SeatId, token| View {
            seat: bbt_mux::Ulid::from(seat.0.0),
            generation: 7,
            view_generation: bbt_mux::Ulid::from(token),
        };
        let mut pending = VecDeque::new();
        queue_pending(
            &mut pending,
            live::UiMsg::Watched(vec![
                (seats[0], view(seats[0], 1)),
                (seats[1], view(seats[1], 1)),
            ]),
        )
        .unwrap();
        for seat in seats {
            queue_pending(
                &mut pending,
                live::UiMsg::Resize {
                    seat,
                    generation: 7,
                    cols: 80,
                    rows: 24,
                },
            )
            .unwrap();
        }
        queue_pending(&mut pending, input(seats[0], 1, b"kept".to_vec())).unwrap();
        queue_pending(
            &mut pending,
            live::UiMsg::Resize {
                seat: seats[0],
                generation: 8,
                cols: 120,
                rows: 30,
            },
        )
        .unwrap();
        queue_pending(
            &mut pending,
            live::UiMsg::Watched(vec![(seats[0], view(seats[0], 2))]),
        )
        .unwrap();
        assert_eq!(pending.len(), 3);
        assert!(
            matches!(pending.front().unwrap(), live::UiMsg::Input { bytes, .. } if bytes == b"kept")
        );
        assert!(
            pending.iter().any(|message| matches!(message,
            live::UiMsg::Resize { seat, generation: 8, cols: 120, rows: 30 } if *seat == seats[0]))
        );
        assert!(
            matches!(pending.back().unwrap(), live::UiMsg::Watched(panes)
            if panes == &vec![(seats[0], view(seats[0], 2))])
        );
        queue_pending(&mut pending, live::UiMsg::Watched(vec![])).unwrap();
        assert_eq!(pending.len(), 2);
        assert!(
            matches!(pending.front().unwrap(), live::UiMsg::Input { bytes, .. } if bytes == b"kept")
        );
        assert!(matches!(pending.back().unwrap(), live::UiMsg::Watched(panes) if panes.is_empty()));
    }

    #[test]
    fn ten_minute_disconnect_bounds_admission_and_drains_accepted_input_in_order() {
        let seats = [SeatId::from_u128(42), SeatId::from_u128(43)];
        let mut pending = VecDeque::new();
        let mut refused = AdmissionRefusals::default();
        let mut expected = Vec::new();
        let (sender, mut receiver) = tokio::sync::mpsc::channel(1);
        // 600 simulated seconds, each with 100 input bursts of64 bytes.
        // No wall-clock sleeps: the disconnected consumer never drains.
        for second in 0..600 {
            let focused = second % 2;
            let panes = [focused, 1 - focused]
                .map(|index| {
                    let seat = seats[index];
                    (
                        seat,
                        View {
                            seat: bbt_mux::Ulid::from(seat.0.0),
                            generation: 7,
                            view_generation: bbt_mux::Ulid::from(second as u128 + 1),
                        },
                    )
                })
                .to_vec();
            queue_pending(&mut pending, live::UiMsg::Watched(panes)).unwrap();
            for (index, seat) in seats.into_iter().enumerate() {
                queue_pending(
                    &mut pending,
                    live::UiMsg::Resize {
                        seat,
                        generation: 7,
                        cols: 80 + (second % 20) as u16 + index as u16,
                        rows: 24,
                    },
                )
                .unwrap();
            }
            if let Err(kind) = queue_pending(&mut pending, live::UiMsg::SpawnZsh) {
                refused.record(kind);
            }
            for key in 0..100 {
                let ordinal = second * 100 + key;
                let bytes = vec![(ordinal % 251) as u8; 64];
                match queue_pending(
                    &mut pending,
                    input(seats[focused], ordinal as u128 + 1, bytes.clone()),
                ) {
                    Ok(()) => expected.extend_from_slice(&bytes),
                    Err(kind) => refused.record(kind),
                }
                let inputs: Vec<_> = pending
                    .iter()
                    .filter_map(|message| match message {
                        live::UiMsg::Input { bytes, .. } => Some(bytes),
                        _ => None,
                    })
                    .collect();
                assert!(inputs.len() <= PENDING_INPUT_ENTRIES);
                assert!(
                    inputs.iter().map(|bytes| bytes.len()).sum::<usize>() <= PENDING_INPUT_BYTES
                );
                assert!(pending.len() <= PENDING_INPUT_ENTRIES + PENDING_SPAWNS + seats.len() + 1);
            }
            flush_pending(&sender, &mut pending, true).unwrap();
            assert!(receiver.try_recv().is_err());
        }
        assert_eq!(expected.len(), PENDING_INPUT_BYTES);
        assert_eq!(refused.input, 60_000 - (PENDING_INPUT_BYTES / 64) as u64);
        assert_eq!(refused.spawn, 600 - PENDING_SPAWNS as u64);

        let mut delivered = Vec::new();
        let mut sizes = BTreeMap::new();
        let mut watched = Vec::new();
        let mut spawns = 0;
        while !pending.is_empty() {
            flush_pending(&sender, &mut pending, false).unwrap();
            match receiver
                .try_recv()
                .expect("capacity-one reconnect must make progress")
            {
                live::UiMsg::Input {
                    generation, bytes, ..
                } => {
                    assert_eq!(generation, 7);
                    delivered.extend(bytes);
                }
                live::UiMsg::Resize {
                    seat, cols, rows, ..
                } => {
                    sizes.insert(seat, (cols, rows));
                }
                live::UiMsg::Watched(panes) => watched = panes,
                live::UiMsg::SpawnZsh | live::UiMsg::Spawn { .. } => spawns += 1,
            }
        }
        assert_eq!(delivered, expected);
        assert_eq!(spawns, PENDING_SPAWNS);
        assert_eq!(
            sizes,
            BTreeMap::from([(seats[0], (99, 24)), (seats[1], (100, 24))])
        );
        assert_eq!(watched.len(), 2);
        assert_eq!(watched[0].0, seats[1]);
        assert!(
            watched
                .iter()
                .all(|(_, view)| view.view_generation == bbt_mux::Ulid::from(600))
        );
        assert!(receiver.try_recv().is_err());
        eprintln!(
            "PASS600 simulated seconds:60000 input bursts,{} accepted bytes,{}, ordered reconnect drain",
            delivered.len(),
            refused.report()
        );
    }

    #[test]
    fn text_bursts_keep_byte_order_first_id_and_generation_boundaries() {
        let seat = SeatId::from_u128(42);
        let first = bbt_mux::Ulid::new();
        let mut pending = VecDeque::new();
        for (generation, id, bytes) in [
            (7, first, b"echo ".to_vec()),
            (7, bbt_mux::Ulid::new(), b"hi\r".to_vec()),
            (8, bbt_mux::Ulid::new(), b"new".to_vec()),
        ] {
            queue_pending(
                &mut pending,
                live::UiMsg::Input {
                    seat,
                    generation,
                    id,
                    bytes,
                },
            )
            .unwrap();
        }
        assert_eq!(pending.len(), 2);
        assert!(
            matches!(pending.pop_front().unwrap(), live::UiMsg::Input { generation: 7, id, bytes, .. } if id == first && bytes == b"echo hi\r")
        );
        assert!(
            matches!(pending.pop_front().unwrap(), live::UiMsg::Input { generation: 8, bytes, .. } if bytes == b"new")
        );
    }
    #[test]
    fn input_queued_while_backend_is_lost_survives_resume_and_a_full_queue() {
        let (sender, mut receiver) = tokio::sync::mpsc::channel(1);
        let seat = SeatId::from_u128(42);
        let ids = [bbt_mux::Ulid::new(), bbt_mux::Ulid::new()];
        let mut pending = VecDeque::new();
        for (id, bytes) in ids.into_iter().zip([b"first".to_vec(), b"second".to_vec()]) {
            pending.push_back(live::UiMsg::Input {
                seat,
                generation: 7,
                id,
                bytes,
            });
        }
        flush_pending(&sender, &mut pending, true).unwrap();
        assert_eq!(pending.len(), 2);
        assert!(receiver.try_recv().is_err());
        flush_pending(&sender, &mut pending, false).unwrap();
        assert_eq!(pending.len(), 1);
        assert!(
            matches!(receiver.try_recv().unwrap(), live::UiMsg::Input { generation: 7, id, bytes, .. } if id == ids[0] && bytes == b"first")
        );
        flush_pending(&sender, &mut pending, false).unwrap();
        assert!(pending.is_empty());
        assert!(
            matches!(receiver.try_recv().unwrap(), live::UiMsg::Input { generation: 7, id, bytes, .. } if id == ids[1] && bytes == b"second")
        );
    }
    #[test]
    fn operator_keys_use_pty_bytes_and_keep_command_shortcuts_out_of_the_shell() {
        let plain = ModifiersState::empty();
        assert_eq!(
            terminal_input(&Key::Named(NamedKey::Enter), None, plain),
            Some(vec![b'\r'])
        );
        assert_eq!(
            terminal_input(&Key::Named(NamedKey::Space), Some(" "), plain),
            Some(vec![b' '])
        );
        assert_eq!(
            terminal_input(&Key::Named(NamedKey::Backspace), None, plain),
            Some(vec![0x7f])
        );
        assert_eq!(
            terminal_input(
                &Key::Character("c".into()),
                Some("c"),
                ModifiersState::CONTROL
            ),
            Some(vec![3])
        );
        assert_eq!(
            terminal_input(&Key::Character("界".into()), Some("界"), plain),
            Some("界".as_bytes().to_vec())
        );
        assert_eq!(
            terminal_input(&Key::Named(NamedKey::ArrowLeft), None, plain),
            Some(b"\x1b[D".to_vec())
        );
        assert_eq!(
            terminal_input(
                &Key::Character("t".into()),
                Some("t"),
                ModifiersState::SUPER
            ),
            None
        );
    }
}

#[cfg(test)]
mod launcher_host_tests {
    use super::*;
    use bbt_ui_model::{Axis, Seat, SplitNode, Swarm, SwarmId, Tab, TabId};

    #[test]
    fn ime_preedit_converts_utf8_cursor_bytes_to_utf16_without_committing() {
        assert_eq!(
            preedit_command("A😀界".into(), Some((5, 5))),
            LauncherCommand::Preedit {
                text: "A😀界".into(),
                caret: Some(3)
            }
        );
        assert_eq!(
            preedit_command("😀".into(), Some((1, 1))),
            LauncherCommand::Preedit {
                text: "😀".into(),
                caret: None
            }
        );
    }

    fn fixture() -> UiModel {
        let swarm = SwarmId::from_u128(1);
        UiModel::new(
            vec![Swarm::new(
                swarm,
                "swarm",
                vec![
                    Tab::new(
                        TabId::from_u128(1),
                        "split",
                        SplitNode::split(
                            Axis::Row,
                            0.5,
                            SplitNode::Leaf(SeatId::from_u128(1)),
                            SplitNode::Leaf(SeatId::from_u128(2)),
                        ),
                    ),
                    Tab::new(
                        TabId::from_u128(2),
                        "other",
                        SplitNode::Leaf(SeatId::from_u128(3)),
                    ),
                ],
            )],
            (1..=3)
                .map(|id| {
                    Seat::new(
                        SeatId::from_u128(id),
                        swarm,
                        [id as u8; 32],
                        format!("seat{id}"),
                        "zsh",
                    )
                })
                .collect(),
        )
        .unwrap()
    }
    fn route(key: Key, mods: ModifiersState, repeat: bool, query: Option<&str>) -> RoutedKey {
        route_key(
            &test_key(&key, PhysicalKey::Code(KeyCode::Space), None, mods, repeat),
            0,
            query,
        )
    }
    #[test]
    fn terminal_keys_stay_byte_exact_while_explicit_chords_move_focus() {
        let plain = ModifiersState::empty();
        for (key, mods, bytes) in [
            (Key::Named(NamedKey::Tab), plain, b"\t".to_vec()),
            (
                Key::Named(NamedKey::Tab),
                ModifiersState::SHIFT,
                b"\x1b[Z".to_vec(),
            ),
            (Key::Named(NamedKey::ArrowLeft), plain, b"\x1b[D".to_vec()),
            (
                Key::Named(NamedKey::ArrowUp),
                ModifiersState::ALT,
                b"\x1b\x1b[A".to_vec(),
            ),
            (Key::Character("c".into()), ModifiersState::CONTROL, vec![3]),
            (Key::Character("界".into()), plain, "界".as_bytes().to_vec()),
            (Key::Named(NamedKey::Enter), plain, b"\r".to_vec()),
        ] {
            assert_eq!(
                route(key.clone(), mods, false, None),
                RoutedKey::Terminal(bytes.clone())
            );
            assert_eq!(route(key, mods, true, None), RoutedKey::Terminal(bytes));
        }
        let nav = ModifiersState::SUPER | ModifiersState::ALT;
        assert_eq!(
            route(Key::Named(NamedKey::ArrowLeft), nav, true, None),
            RoutedKey::Pane(Navigation::Direction(Direction::Left))
        );
        assert_eq!(
            route(
                Key::Named(NamedKey::Tab),
                nav | ModifiersState::SHIFT,
                false,
                None
            ),
            RoutedKey::Pane(Navigation::Cycle { backwards: true })
        );
    }
    #[test]
    fn terminal_route_uses_live_modes_and_forwards_only_requested_releases() {
        let mut input = test_key(
            &Key::Named(NamedKey::Escape),
            PhysicalKey::Code(KeyCode::Escape),
            None,
            ModifiersState::empty(),
            false,
        );
        assert_eq!(route_key(&input, 0, None), RoutedKey::Terminal(vec![27]));
        assert_eq!(
            route_key(&input, 1 << 18, None),
            RoutedKey::Terminal(b"\x1b[27u".to_vec())
        );
        input.state = ElementState::Released;
        assert_eq!(route_key(&input, 1 << 18, None), RoutedKey::Local);
        assert_eq!(
            route_key(&input, 3 << 18, None),
            RoutedKey::Terminal(b"\x1b[27;1:3u".to_vec())
        );
        assert_eq!(route_key(&input, 31 << 18, Some("")), RoutedKey::Local);
        input.state = ElementState::Pressed;
        input.repeat = true;
        assert_eq!(
            route_key(&input, 3 << 18, None),
            RoutedKey::Terminal(b"\x1b[27;1:2u".to_vec())
        );
        // A local Cmd chord stays local after Cmd is released ahead of its key.
        let mut state = keyboard::State::default();
        let physical = PhysicalKey::Code(KeyCode::KeyT);
        let command = test_key(
            &Key::Character("t".into()),
            physical,
            Some("t"),
            ModifiersState::SUPER,
            false,
        );
        assert_eq!(route_key(&command, 31 << 18, None), RoutedKey::OpenLauncher);
        state.consumed(physical, ElementState::Pressed);
        assert!(state.suppressed(physical, ElementState::Pressed, true));
        assert!(state.suppressed(physical, ElementState::Released, false));
    }
    #[test]
    fn consumed_local_presses_keep_local_repeats_without_leaking_into_the_pty() {
        let mut state = keyboard::State::default();
        let mut arrow = test_key(
            &Key::Named(NamedKey::ArrowLeft),
            PhysicalKey::Code(KeyCode::ArrowLeft),
            None,
            ModifiersState::SUPER | ModifiersState::ALT,
            false,
        );
        let navigation = RoutedKey::Pane(Navigation::Direction(Direction::Left));
        assert_eq!(route_input(&mut state, &arrow, 31 << 18, None), navigation);
        arrow.repeat = true;
        assert_eq!(route_input(&mut state, &arrow, 31 << 18, None), navigation);
        arrow.modifiers.state = ModifiersState::empty();
        assert_eq!(
            route_input(&mut state, &arrow, 31 << 18, None),
            RoutedKey::Local
        );
        arrow.state = ElementState::Released;
        arrow.repeat = false;
        assert_eq!(
            route_input(&mut state, &arrow, 31 << 18, None),
            RoutedKey::Local
        );
        arrow.state = ElementState::Pressed;
        assert_eq!(
            route_input(&mut state, &arrow, 31 << 18, None),
            RoutedKey::Terminal(b"\x1b[D".to_vec())
        );

        let mut letter = test_key(
            &Key::Character("a".into()),
            PhysicalKey::Code(KeyCode::KeyA),
            Some("a"),
            ModifiersState::empty(),
            false,
        );
        assert_eq!(
            route_input(&mut state, &letter, 0, Some("z")),
            RoutedKey::Launcher(LauncherCommand::Insert("a".into()))
        );
        letter.repeat = true;
        assert_eq!(
            route_input(&mut state, &letter, 0, Some("za")),
            RoutedKey::Launcher(LauncherCommand::Insert("a".into()))
        );
        assert_eq!(
            route_input(&mut state, &letter, 31 << 18, None),
            RoutedKey::Local
        );
    }
    #[test]
    fn modal_input_never_reaches_a_pty_and_picks_do_not_repeat() {
        let plain = ModifiersState::empty();
        assert_eq!(
            route(Key::Named(NamedKey::Tab), plain, false, Some("")),
            RoutedKey::Launcher(LauncherCommand::Down)
        );
        assert_eq!(
            route(
                Key::Named(NamedKey::ArrowLeft),
                ModifiersState::SUPER | ModifiersState::ALT,
                false,
                Some("")
            ),
            RoutedKey::Launcher(LauncherCommand::Left)
        );
        assert_eq!(
            route(
                Key::Character("c".into()),
                ModifiersState::CONTROL,
                false,
                Some("zsh")
            ),
            RoutedKey::Local
        );
        for key in [
            Key::Named(NamedKey::Enter),
            Key::Named(NamedKey::Escape),
            Key::Character("5".into()),
        ] {
            assert_eq!(route(key, plain, true, Some("")), RoutedKey::Local);
        }
        assert_eq!(
            route(Key::Named(NamedKey::Enter), plain, false, Some("zsh")),
            RoutedKey::Launcher(LauncherCommand::Activate(Modifiers::default()))
        );
        assert_eq!(
            route(Key::Character("界".into()), plain, false, Some("z")),
            RoutedKey::Launcher(LauncherCommand::Insert("界".into()))
        );
        assert_eq!(
            route(Key::Named(NamedKey::Backspace), plain, true, Some("z界")),
            RoutedKey::Launcher(LauncherCommand::Backspace)
        );
        assert_eq!(
            route_key(
                &test_key(
                    &Key::Character("t".into()),
                    PhysicalKey::Code(KeyCode::KeyT),
                    Some("t"),
                    ModifiersState::SUPER,
                    false
                ),
                0,
                None
            ),
            RoutedKey::OpenLauncher
        );
        assert_eq!(
            route_key(
                &test_key(
                    &Key::Character("t".into()),
                    PhysicalKey::Code(KeyCode::KeyT),
                    Some("t"),
                    ModifiersState::SUPER,
                    true
                ),
                0,
                None
            ),
            RoutedKey::Local
        );
    }
    #[test]
    fn actual_controller_filters_picks_climbs_and_preserves_the_named_gaps() {
        let mut launcher = new_launcher(None, 0);
        launcher.handle(LauncherCommand::Query("zsh".into()));
        assert_eq!(
            launcher.handle(LauncherCommand::Activate(Modifiers::default())),
            LauncherOutcome::Pick {
                id: "zsh".into(),
                modifiers: Modifiers::default()
            }
        );
        assert!(
            matches!(default_spawn("zsh", Modifiers::default(), None), Some(live::UiMsg::Spawn { harness_kind, .. }) if harness_kind == "zsh")
        );
        for id in [
            "settings",
            "zsh:right",
            "ssh:rig:zsh",
            "anything labelled zsh",
        ] {
            assert!(default_spawn(id, Modifiers::default(), None).is_none());
        }
        assert!(
            default_spawn(
                "zsh",
                Modifiers {
                    command: true,
                    ..Default::default()
                },
                None
            )
            .is_none()
        );
        launcher.handle(LauncherCommand::Right);
        assert_eq!(launcher.launcher.path, ["zsh"]);
        assert_eq!(
            launcher.handle(LauncherCommand::Escape),
            LauncherOutcome::Changed
        );
        assert_eq!(
            launcher.handle(LauncherCommand::Escape),
            LauncherOutcome::Close
        );
        let mut capped = new_launcher(None, PENDING_SPAWNS);
        capped.handle(LauncherCommand::Query("zsh".into()));
        assert_eq!(
            capped.handle(LauncherCommand::Activate(Modifiers::default())),
            LauncherOutcome::Capped
        );
    }
    #[test]
    fn published_screens_do_not_undo_local_focus_or_create_locate() {
        let model = fixture();
        let mut focus = FocusState::default();
        focus.reconcile(None, &model);
        focus
            .apply(
                &model,
                FocusAction::Navigate(Navigation::Cycle { backwards: false }),
                &[],
                Instant::now(),
            )
            .unwrap();
        assert_eq!(
            focus.stage.as_ref().unwrap().focused,
            Some(SeatId::from_u128(2))
        );
        let mut next = model.clone();
        next.seats.get_mut(&SeatId::from_u128(1)).unwrap().title = "fresh terminal title".into();
        focus.reconcile(Some(&model), &next);
        assert_eq!(
            focus.stage.as_ref().unwrap().focused,
            Some(SeatId::from_u128(2))
        );
        assert!(!focus.pending_layout);
        assert!(focus.layout_ready(&next, &[]).is_empty());
        let mut scene = scene::Scene::live();
        scene.set_model(
            &bboxy450_term::model::Model {
                ui: next.clone(),
                ..Default::default()
            },
            focus.stage.as_ref().unwrap(),
        );
        assert_eq!(scene.focused(), Some(SeatId::from_u128(2)));
        next.apply(bbt_ui_model::event::Update::SeatClosedByDaemon {
            seat: SeatId::from_u128(2),
        })
        .unwrap();
        focus.reconcile(Some(&model), &next);
        assert_eq!(
            focus.stage.as_ref().unwrap().focused,
            Some(SeatId::from_u128(1))
        );
    }
    #[test]
    fn stage_switch_waits_for_the_new_outer_pane_before_locate_once() {
        let model = fixture();
        let mut focus = FocusState::default();
        focus.reconcile(None, &model);
        let events = focus
            .apply(
                &model,
                FocusAction::Activate(SeatId::from_u128(3), FocusOrigin::Row),
                &[],
                Instant::now(),
            )
            .unwrap();
        assert_eq!(focus.stage.as_ref().unwrap().tab, Some(TabId::from_u128(2)));
        assert!(
            events
                .iter()
                .all(|e| !matches!(e, InputEvent::Locate { .. }))
        );
        assert!(
            focus
                .layout_ready(
                    &model,
                    &[(SeatId::from_u128(1), Rect::new(0., 0., 100., 100.))]
                )
                .is_empty()
        );
        assert!(focus.pending_layout);
        let pane = [(SeatId::from_u128(3), Rect::new(310., 40., 600., 500.))];
        assert!(
            matches!(focus.layout_ready(&model, &pane).as_slice(), [InputEvent::Locate { seat, .. }] if *seat == SeatId::from_u128(3))
        );
        assert!(focus.layout_ready(&model, &pane).is_empty());
    }
}

#[cfg(test)]
mod spawn_admission_tests {
    use super::*;

    fn generic() -> live::UiMsg {
        live::UiMsg::Spawn {
            harness_kind: "codex".into(),
            name: Some("named seat".into()),
            location: bbt_proto::Where::Local,
        }
    }
    #[test]
    fn generic_and_legacy_spawns_share_the_same_cap() {
        let mut pending = VecDeque::new();
        for n in 0..PENDING_SPAWNS {
            queue_pending(
                &mut pending,
                if n % 2 == 0 {
                    generic()
                } else {
                    live::UiMsg::SpawnZsh
                },
            )
            .unwrap();
        }
        assert_eq!(
            queue_pending(&mut pending, generic()),
            Err(AdmissionRefused::Spawn)
        );
        assert_eq!(
            queue_pending(&mut pending, live::UiMsg::SpawnZsh),
            Err(AdmissionRefused::Spawn)
        );
        assert_eq!(pending.len(), PENDING_SPAWNS);
        assert!(pending.iter().all(live::UiMsg::is_spawn));
        queue_pending(&mut pending, live::UiMsg::Watched(vec![])).unwrap();
        assert_eq!(pending.len(), PENDING_SPAWNS + 1);
        assert!(!pending.back().unwrap().is_spawn());
    }
}

#[cfg(test)]
mod default_spawn_tests {
    use super::*;

    #[test]
    fn default_picks_are_keyed_by_id_and_leave_placement_and_navigation_deferred() {
        for (kind, _, _, _, _) in LAUNCHABLES {
            assert!(
                matches!(default_spawn(kind, Modifiers::default(), None), Some(live::UiMsg::Spawn { harness_kind, name: None, location: bbt_proto::Where::Local }) if harness_kind == kind)
            );
            for place in ["tab", "right", "bottom"] {
                assert!(
                    default_spawn(&format!("{kind}:{place}"), Modifiers::default(), None).is_none()
                );
            }
        }
        for id in ["settings", "graph", "seat:zsh", "something labelled zsh"] {
            assert!(default_spawn(id, Modifiers::default(), None).is_none());
        }
        for modifiers in [
            Modifiers {
                command: true,
                ..Default::default()
            },
            Modifiers {
                alt: true,
                ..Default::default()
            },
            Modifiers {
                shift: true,
                ..Default::default()
            },
        ] {
            assert!(default_spawn("zsh", modifiers, None).is_none());
        }
        let rows = launcher_items(None);
        assert_eq!(
            rows.iter().find(|row| row.id == "claude").unwrap().sub,
            "executor by default"
        );
    }

    #[test]
    fn host_picks_preserve_structured_location_without_parsing_labels() {
        let mut model = bboxy450_term::model::Model::default();
        for location in [
            bbt_proto::Where::Ssh { host: "rig".into() },
            bbt_proto::Where::Cloud {
                provider: "cloud:one".into(),
                host: "worker:two".into(),
            },
        ] {
            model.hosts = vec![bbt_proto::GuiHost {
                location: location.clone(),
                provider: "fixture".into(),
                label: "a label is not an executable".into(),
                address: None,
                region: None,
                state: bbt_proto::HostState::Connected { rtt_ms: 4 },
            }];
            let host = bboxy450_term::hosts::key(&location);
            let id = format!("{host}:codex");
            assert!(
                matches!(default_spawn(&id, Modifiers::default(), Some(&model)), Some(live::UiMsg::Spawn { harness_kind, name: None, location: actual }) if harness_kind == "codex" && actual == location)
            );
            assert!(
                default_spawn(&format!("{id}:right"), Modifiers::default(), Some(&model)).is_none()
            );
            let root = if matches!(location, bbt_proto::Where::Ssh { .. }) {
                "ssh"
            } else {
                "cloud"
            };
            let rows = launcher_items(Some(&model));
            let host_row = rows
                .iter()
                .find(|row| row.id == root)
                .unwrap()
                .children
                .as_ref()
                .unwrap()
                .iter()
                .find(|row| row.id == host)
                .unwrap();
            let codex = host_row
                .children
                .as_ref()
                .unwrap()
                .iter()
                .find(|row| row.id == id)
                .unwrap();
            assert!(codex.enter_picks);
            assert!(
                codex
                    .children
                    .as_ref()
                    .unwrap()
                    .iter()
                    .any(|row| row.id == format!("{id}:right"))
            );
        }
        assert!(default_spawn("ssh:missing:codex", Modifiers::default(), Some(&model)).is_none());
    }
}
