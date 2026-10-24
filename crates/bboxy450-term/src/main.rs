//! R2 app shell: event-driven native scene, memory-only UI drain and hitch meter.
mod assets;
mod budget;
mod capture;
mod display_link;
mod e2e_checklist;
mod gpu;
mod live;
mod log;
mod scene;
mod stall;

use bboxy450_term::launcher::{
    CaretMove, Command as LauncherCommand, Controller as LauncherController,
    Outcome as LauncherOutcome,
};
use bboxy450_term::scrollback::Request as ScrollRequest;
use bboxy450_term::{keyboard, messaging};
use bbt_proto::{SeatId, View};
use bbt_rt::{HitchMeter, Latest, Phases, UiInbox, UiMsg};
use bbt_ui::modules::composer::{
    self, Command as ComposeCommand, Controller as ComposeController, Field as ComposeField,
    Outcome as ComposeOutcome,
};
use bbt_ui::text_edit::Command as EditCommand;
use bbt_ui_model::{
    Model as UiModel, STYLE, Stage,
    event::Event as InputEvent,
    input::{
        Direction, Focus, FocusContext, FocusOrigin, KeyRoute, Modifiers, Navigation,
        NavigationKey, route_navigation,
    },
    layout::Rect,
    tokens::Theme,
};
use display_link::{Clock, DisplayLink, Mode};
use e2e_checklist::{LocalTab, PageCommand};
use std::collections::{BTreeMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};
use winit::platform::macos::MonitorHandleExtMacOS;
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{ElementState, Ime, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy},
    keyboard::{Key, KeyCode, ModifiersState, NamedKey, PhysicalKey},
    window::{Window, WindowId},
};

#[derive(Debug)]
pub enum UiEvent {
    Wake,
    Settings(Arc<preferences::Snapshot>),
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
    VerifyScrollTop,
    VerifyClose,
    VerifyFailed(String),
    Capture,
    /// A capture asked through an operator door (`ctl capture`); answered when written.
    CaptureFor(live::CaptureAsk),
    /// No frame was presented for this capture in time (occluded, or the display asleep).
    CaptureDeadline(bbt_mux::Ulid),
    StallPoll,
    Notification(bboxy450_desktop_notifications::Response),
    NotificationReady,
    NotificationError(String),
}
struct Wake(EventLoopProxy<UiEvent>);
impl bbt_rt::Waker for Wake {
    fn wake(&self) {
        let _ = self.0.send_event(UiEvent::Wake);
    }
}
use bboxy450_term::settings as preferences;
use bbt_ui::modules::settings::{Settings, controller as settings_controller};
#[derive(Debug, PartialEq, Eq)]
enum SettingsKey {
    Edit(EditCommand),
    Next(bool),
    Nudge(i32),
    Activate,
    Reload,
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
    fn native_invalidation(&mut self, event: &WindowEvent) -> Result<(), String> {
        let reason = match event {
            WindowEvent::Resized(_) => "Resized",
            WindowEvent::ScaleFactorChanged { .. } => "ScaleFactorChanged",
            _ => return Ok(()),
        };
        self.interrupt();
        Err(format!("idle probe invalidated by {reason}"))
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
#[derive(Clone, Copy)]
enum TextFocus {
    Composer(ComposeField),
    Reject(bbt_mux::Ulid),
}

#[derive(Debug, PartialEq, Eq)]
enum NotificationTarget {
    Pending,
    Ready,
    Refused(&'static str),
}
fn notification_target(
    seat: SeatId,
    received: Instant,
    now: Instant,
    model: Option<&bboxy450_term::model::Model>,
    window: bool,
) -> NotificationTarget {
    if now.saturating_duration_since(received) >= Duration::from_secs(30) {
        return NotificationTarget::Refused("seat unavailable within 30 seconds");
    }
    let Some(row) = model.and_then(|model| model.ui.seats.get(&seat)) else {
        return NotificationTarget::Pending;
    };
    if row.lifecycle != bbt_ui_model::Lifecycle::Open {
        return NotificationTarget::Refused("seat is closed");
    }
    if window {
        NotificationTarget::Ready
    } else {
        NotificationTarget::Pending
    }
}
impl FocusState {
    fn reconcile(&mut self, _previous: Option<&UiModel>, next: &UiModel) {
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
        // Metadata never selects a new seat. Only a correlated launch result
        // or an explicit UI focus action changes the focused pane.
        self.stage = self
            .stage
            .as_ref()
            .and_then(|s| s.focused)
            .and_then(owner)
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
    OpenGraph,
    OpenSettings,
    MuteFocused,
    SettingsKey(SettingsKey),
    LocalPage(PageCommand),
    ToggleHud,
    Launcher(LauncherCommand),
    Composer(ComposeCommand),
    ComposerVertical { backwards: bool, extend: bool },
    ComposerLineEdge { end: bool, extend: bool },
    Pane(Navigation),
    Scroll(ScrollKey),
    Terminal(Vec<u8>),
    Local,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ScrollKey {
    PageUp,
    PageDown,
    Top,
    Live,
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
        "oracle by default",
        "#D97757",
        "Suggestions",
    ),
    (
        "codex",
        "Codex",
        "executor · never an oracle",
        "#1F2937",
        "Suggestions",
    ),
    ("pi", "pi", "executor by default", "#7C3AED", "Harnesses"),
    ("omp", "omp", "executor by default", "#0EA5E9", "Harnesses"),
    ("zsh", "zsh", "~ · shell, no role", "#111827", "Shells"),
];
const ROLE_PRESETS: [(&str, &str, &str); 4] = [
    ("oracle", "Oracle", "supervises, briefs, verifies"),
    ("architect", "Architect", "specs, invariants, decisions"),
    ("creativity", "Creativity", "diverges before the pick"),
    ("executor", "Executor", "implements a scoped brief"),
];

fn default_spawn(
    id: &str,
    modifiers: Modifiers,
    model: Option<&bboxy450_term::model::Model>,
) -> Option<live::UiMsg> {
    if modifiers != Modifiers::default() {
        return None;
    }
    let (id, role) = match id.strip_prefix("claude:as:") {
        Some(role) if ROLE_PRESETS.iter().any(|(name, _, _)| *name == role) => {
            ("claude", Some(role.to_owned()))
        }
        Some(_) => return None,
        None => (id, None),
    };
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
                id: bbt_mux::Ulid::new(),
                swarm: None,
                placement: None,
                harness_kind: kind.into(),
                name: None,
                role: role.clone(),
                location,
            });
        }
    }
    None
}

fn spawn_pick(
    id: &str,
    modifiers: Modifiers,
    model: Option<&bboxy450_term::model::Model>,
    stage: Option<&Stage>,
) -> Option<live::UiMsg> {
    let (root, swarm, placement) =
        bboxy450_term::launcher::captured_placement(id, modifiers, model, stage);
    let mut message = default_spawn(root, Modifiers::default(), model)?;
    if let live::UiMsg::Spawn {
        swarm: selected,
        placement: captured,
        ..
    } = &mut message
    {
        *selected = swarm;
        *captured = Some(placement);
    }
    Some(message)
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
    if command
        && !mods.alt_key()
        && !mods.shift_key()
        && !mods.control_key()
        && physical == PhysicalKey::Code(KeyCode::KeyG)
    {
        return if repeat {
            RoutedKey::Local
        } else {
            RoutedKey::OpenGraph
        };
    }
    if command && !mods.alt_key() && !mods.shift_key() && !mods.control_key() {
        if physical == PhysicalKey::Code(KeyCode::Comma) {
            return if repeat {
                RoutedKey::Local
            } else {
                RoutedKey::OpenSettings
            };
        }
        if physical == PhysicalKey::Code(KeyCode::KeyM) {
            return if repeat {
                RoutedKey::Local
            } else {
                RoutedKey::MuteFocused
            };
        }
    }
    if query.is_none() && mods.shift_key() && !command && !mods.alt_key() && !mods.control_key() {
        let motion = match key {
            Key::Named(NamedKey::PageUp) => Some(ScrollKey::PageUp),
            Key::Named(NamedKey::PageDown) => Some(ScrollKey::PageDown),
            Key::Named(NamedKey::Home) => Some(ScrollKey::Top),
            Key::Named(NamedKey::End) => Some(ScrollKey::Live),
            _ => None,
        };
        if let Some(motion) = motion {
            return RoutedKey::Scroll(motion);
        }
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
    let routed = route_key(input, modes, query);
    finish_route(state, input, routed)
}
fn finish_route(
    state: &mut keyboard::State,
    input: &keyboard::Input<'_>,
    routed: RoutedKey,
) -> RoutedKey {
    let was_local = state.suppressed(input.physical_key, input.state, input.repeat);
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
fn route_local_page(input: &keyboard::Input<'_>, modes: u64) -> RoutedKey {
    let global = route_key(input, modes, None);
    if matches!(
        global,
        RoutedKey::Quit
            | RoutedKey::OpenLauncher
            | RoutedKey::OpenGraph
            | RoutedKey::OpenSettings
            | RoutedKey::MuteFocused
            | RoutedKey::ToggleHud
            | RoutedKey::Pane(_)
    ) {
        return global;
    }
    // Balance a key owned by the PTY before the page opened. finish_route still
    // suppresses releases for presses consumed by this local page.
    if input.state == ElementState::Released {
        return route_key(input, modes, None);
    }
    let mods = input.modifiers.state;
    if mods.super_key()
        && !mods.alt_key()
        && !mods.shift_key()
        && !mods.control_key()
        && input.physical_key == PhysicalKey::Code(KeyCode::KeyW)
    {
        return if input.repeat {
            RoutedKey::Local
        } else {
            RoutedKey::LocalPage(PageCommand::Close)
        };
    }
    if mods.super_key() || mods.control_key() || mods.alt_key() {
        return RoutedKey::Local;
    }
    let command = match &input.logical_key {
        Key::Named(NamedKey::Escape) if !input.repeat => PageCommand::Close,
        Key::Named(NamedKey::ArrowUp) => PageCommand::Up,
        Key::Named(NamedKey::ArrowDown) => PageCommand::Down,
        Key::Named(NamedKey::PageUp) => PageCommand::PageUp,
        Key::Named(NamedKey::PageDown) => PageCommand::PageDown,
        Key::Named(NamedKey::Home) => PageCommand::Top,
        Key::Named(NamedKey::End) => PageCommand::Bottom,
        _ => return RoutedKey::Local,
    };
    RoutedKey::LocalPage(command)
}
fn route_settings(input: &keyboard::Input<'_>, modes: u64, editing: bool) -> RoutedKey {
    let global = route_local_page(input, modes);
    if input.state == ElementState::Released
        || matches!(
            global,
            RoutedKey::Quit
                | RoutedKey::OpenLauncher
                | RoutedKey::OpenGraph
                | RoutedKey::OpenSettings
                | RoutedKey::MuteFocused
                | RoutedKey::ToggleHud
                | RoutedKey::Pane(_)
                | RoutedKey::LocalPage(PageCommand::Close)
        )
    {
        return global;
    }
    if input.logical_key == Key::Named(NamedKey::Tab) && !input.modifiers.state.super_key() {
        return RoutedKey::SettingsKey(SettingsKey::Next(input.modifiers.state.shift_key()));
    }
    if input.modifiers.state.super_key()
        && input.physical_key == PhysicalKey::Code(KeyCode::KeyR)
        && !input.repeat
    {
        return RoutedKey::SettingsKey(SettingsKey::Reload);
    }
    if editing {
        return match route_composer(input, modes, ComposeField::Address) {
            RoutedKey::Composer(ComposeCommand::Edit(edit)) => {
                RoutedKey::SettingsKey(SettingsKey::Edit(edit))
            }
            _ => RoutedKey::Local,
        };
    }
    if input.modifiers.state.super_key()
        || input.modifiers.state.control_key()
        || input.modifiers.state.alt_key()
    {
        return RoutedKey::Local;
    }
    match input.logical_key {
        Key::Named(NamedKey::ArrowLeft | NamedKey::ArrowUp) => {
            RoutedKey::SettingsKey(SettingsKey::Nudge(-1))
        }
        Key::Named(NamedKey::ArrowRight | NamedKey::ArrowDown) => {
            RoutedKey::SettingsKey(SettingsKey::Nudge(1))
        }
        Key::Named(NamedKey::Enter | NamedKey::Space) if !input.repeat => {
            RoutedKey::SettingsKey(SettingsKey::Activate)
        }
        _ => global,
    }
}
fn route_composer(input: &keyboard::Input<'_>, modes: u64, field: ComposeField) -> RoutedKey {
    if input.state == ElementState::Released {
        return RoutedKey::Local;
    }
    let global = route_key(input, modes, Some(""));
    if matches!(
        global,
        RoutedKey::Quit
            | RoutedKey::OpenLauncher
            | RoutedKey::OpenGraph
            | RoutedKey::OpenSettings
            | RoutedKey::MuteFocused
            | RoutedKey::ToggleHud
    ) {
        return global;
    }
    let mods = input.modifiers.state;
    let command = mods.super_key();
    let navigation = match &input.logical_key {
        Key::Named(NamedKey::ArrowLeft) => NavigationKey::Arrow(Direction::Left),
        Key::Named(NamedKey::ArrowRight) => NavigationKey::Arrow(Direction::Right),
        Key::Named(NamedKey::ArrowUp) => NavigationKey::Arrow(Direction::Up),
        Key::Named(NamedKey::ArrowDown) => NavigationKey::Arrow(Direction::Down),
        Key::Named(NamedKey::Tab) => NavigationKey::Tab,
        _ => NavigationKey::Other,
    };
    if let KeyRoute::Pane(navigation) =
        route_navigation(navigation, input_modifiers(mods), FocusContext::TextField)
    {
        return RoutedKey::Pane(navigation);
    }
    let edit = match &input.logical_key {
        Key::Named(NamedKey::Enter)
            if command
                && !mods.alt_key()
                && !mods.control_key()
                && !mods.shift_key()
                && !input.repeat =>
        {
            return RoutedKey::Composer(ComposeCommand::Send);
        }
        Key::Named(NamedKey::Enter) if command => return RoutedKey::Local,
        Key::Named(NamedKey::Enter) if field == ComposeField::Address && !input.repeat => {
            return RoutedKey::Composer(ComposeCommand::ChooseSelected);
        }
        Key::Named(NamedKey::Enter) if field == ComposeField::Body => {
            EditCommand::Insert("\n".into())
        }
        Key::Named(NamedKey::Tab) => {
            return RoutedKey::Composer(ComposeCommand::Focus(if field == ComposeField::Body {
                ComposeField::Address
            } else {
                ComposeField::Body
            }));
        }
        Key::Named(NamedKey::Escape) if !input.repeat => {
            return RoutedKey::Composer(ComposeCommand::Escape);
        }
        Key::Named(NamedKey::ArrowUp | NamedKey::ArrowDown) if field == ComposeField::Address => {
            return RoutedKey::Composer(ComposeCommand::Next {
                backwards: input.logical_key == Key::Named(NamedKey::ArrowUp),
            });
        }
        Key::Named(NamedKey::ArrowUp | NamedKey::ArrowDown) => {
            return RoutedKey::ComposerVertical {
                backwards: input.logical_key == Key::Named(NamedKey::ArrowUp),
                extend: mods.shift_key(),
            };
        }
        Key::Named(NamedKey::Home | NamedKey::End) if field == ComposeField::Body => {
            return RoutedKey::ComposerLineEdge {
                end: input.logical_key == Key::Named(NamedKey::End),
                extend: mods.shift_key(),
            };
        }
        Key::Named(NamedKey::ArrowLeft | NamedKey::ArrowRight)
            if command && field == ComposeField::Body =>
        {
            return RoutedKey::ComposerLineEdge {
                end: input.logical_key == Key::Named(NamedKey::ArrowRight),
                extend: mods.shift_key(),
            };
        }
        Key::Named(NamedKey::Home) => EditCommand::Move {
            motion: CaretMove::Start,
            extend: mods.shift_key(),
        },
        Key::Named(NamedKey::End) => EditCommand::Move {
            motion: CaretMove::End,
            extend: mods.shift_key(),
        },
        Key::Named(NamedKey::ArrowLeft) => EditCommand::Move {
            motion: if command {
                CaretMove::Start
            } else {
                CaretMove::Left
            },
            extend: mods.shift_key(),
        },
        Key::Named(NamedKey::ArrowRight) => EditCommand::Move {
            motion: if command {
                CaretMove::End
            } else {
                CaretMove::Right
            },
            extend: mods.shift_key(),
        },
        Key::Named(NamedKey::Backspace) => EditCommand::Delete { backwards: true },
        Key::Named(NamedKey::Delete) => EditCommand::Delete { backwards: false },
        Key::Character(value) if command && value.eq_ignore_ascii_case("a") => {
            EditCommand::SelectAll
        }
        Key::Character(value) if !command && !mods.control_key() => {
            EditCommand::Insert(input.text.unwrap_or(value).into())
        }
        Key::Named(NamedKey::Space) if !command && !mods.control_key() => {
            EditCommand::Insert(" ".into())
        }
        _ => return RoutedKey::Local,
    };
    RoutedKey::Composer(ComposeCommand::Edit(edit))
}

fn preedit_command(text: String, cursor: Option<(usize, usize)>) -> LauncherCommand {
    // winit reports UTF-8 bytes; the retained field follows UTF-16 indices.
    let caret = cursor.and_then(|(at, _)| text.get(..at).map(|s| s.encode_utf16().count()));
    LauncherCommand::Preedit { text, caret }
}

/// Public item records, with the provider projection kept authoritative. Picks
/// are identified by id; a display label never becomes a command.
#[cfg(test)]
fn launcher_items(
    model: Option<&bboxy450_term::model::Model>,
) -> Vec<bbt_ui::modules::launcher::Item> {
    launcher_items_for(model, None)
}

fn swarm_oracle<'a>(
    model: Option<&'a bboxy450_term::model::Model>,
    stage: Option<&Stage>,
) -> Option<&'a str> {
    let model = model?;
    let swarm = stage.unwrap_or(&model.ui.window.stage).swarm?;
    model.ui.seats.values().find_map(|seat| {
        (seat.swarm == swarm
            && seat.harness.0 == "claude"
            && seat.lifecycle == bbt_ui_model::Lifecycle::Open
            && seat.role.as_ref().is_some_and(|role| role.name == "oracle"))
        .then_some(seat.name.as_str())
    })
}

fn launcher_items_for(
    model: Option<&bboxy450_term::model::Model>,
    stage: Option<&Stage>,
) -> Vec<bbt_ui::modules::launcher::Item> {
    use bbt_ui::modules::launcher::Item;
    fn placements(item: &Item, role: &str) -> Vec<Item> {
        let prefix = if item.role.as_deref() == Some(role) {
            item.id.clone()
        } else {
            format!("{}:as:{role}", item.id)
        };
        [
            ("tab", "New tab"),
            ("right", "Split right"),
            ("bottom", "Split down"),
        ]
        .into_iter()
        .map(|(id, label)| Item {
            id: format!("{prefix}:{id}"),
            label: label.into(),
            harness: item.harness.clone(),
            role: Some(role.into()),
            kind: "Placement".into(),
            tile: item.tile.clone(),
            ..Default::default()
        })
        .collect()
    }
    let launchables = || {
        LAUNCHABLES
            .into_iter()
            .map(|(id, label, sub, tile, section)| Item {
                id: id.into(),
                label: label.into(),
                sub: sub.into(),
                harness: id.into(),
                role: Some(
                    match id {
                        "claude" => "oracle",
                        "zsh" => "shell",
                        _ => "executor",
                    }
                    .into(),
                ),
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
    let note =
        swarm_oracle(model, stage).map(|name| format!("this swarm already has an Oracle: {name}"));
    for item in &mut items {
        item.children = Some(if item.id == "claude" {
            if let Some(note) = &note {
                item.sub.push_str(&format!(" · {note}"));
            }
            ROLE_PRESETS
                .into_iter()
                .map(|(role, label, say)| Item {
                    id: format!("{}:as:{role}", item.id),
                    label: label.into(),
                    sub: if role == "oracle" {
                        format!("default · {}", note.as_deref().unwrap_or(say))
                    } else {
                        say.into()
                    },
                    harness: item.harness.clone(),
                    role: Some(role.into()),
                    kind: "Role".into(),
                    tile: item.tile.clone(),
                    enter_picks: true,
                    children: Some(placements(item, role)),
                    ..Default::default()
                })
                .collect()
        } else {
            placements(item, item.role.as_deref().expect("launch default role"))
        });
    }
    let mut providers = model.map_or_else(Vec::new, |m| m.provider_launcher_items());
    for root in &mut providers {
        for host in root.children.iter_mut().flatten() {
            if host.children.is_some() {
                host.children = Some(
                    launchables()
                        .into_iter()
                        .map(|mut item| {
                            item.id = format!("{}:{}", host.id, item.id);
                            item.children = Some(placements(
                                &item,
                                item.role.as_deref().expect("launch default role"),
                            ));
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
                "Notifications",
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

fn mute_command(
    model: Option<&bboxy450_term::model::Model>,
    stage: Option<&Stage>,
    panes: &[(SeatId, Rect)],
    visible: bool,
) -> Option<live::UiMsg> {
    if !visible {
        return None;
    }
    let model = model?;
    let id = stage?.focused?;
    let seat = model.ui.seats.get(&id)?;
    if !panes.iter().any(|(pane, _)| *pane == id) {
        return None;
    }
    Some(live::UiMsg::Messaging {
        id: bbt_mux::Ulid::new(),
        swarm_id: bbt_proto::SwarmId::from_u128(seat.swarm.0.0),
        request: bbt_proto::messaging::Request::Mute {
            seat: bbt_mux::Ulid(id.0.0),
            muted: !seat.muted,
        },
    })
}
fn new_launcher(model: Option<&bboxy450_term::model::Model>, used: usize) -> LauncherController {
    new_launcher_for(model, used, None)
}

fn new_launcher_for(
    model: Option<&bboxy450_term::model::Model>,
    used: usize,
    stage: Option<&Stage>,
) -> LauncherController {
    use bbt_ui::modules::launcher::{Cap, Launcher};
    LauncherController::new(
        Launcher {
            items: launcher_items_for(model, stage),
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
    scroll_wheel: Option<(SeatId, f64)>,
    focus: FocusState,
    notifications: Option<bboxy450_term::notifications::Worker>,
    frontmost: bool,
    pending_notification: Option<(SeatId, Instant)>,
    spawn_results_seen: BTreeMap<bbt_mux::Ulid, bboxy450_term::model::SpawnOutcome>,
    launcher: Option<LauncherController>,
    launcher_layout: Option<scene::LauncherLayout>,
    launcher_parent: Option<TextFocus>,
    preferences: preferences::State,
    preferences_io: Option<preferences::Io>,
    rim_preview: Option<f32>,
    mute: preferences::Mute,
    system_theme: Theme,
    local_tab: Option<LocalTab>,
    local_page_layout: Option<scene::LocalPageLayout>,
    composer: ComposeController,
    composer_client: messaging::Client,
    composer_scope: Option<bbt_proto::SwarmId>,
    task_records: messaging::Records,
    message_history: messaging::History,
    events_seen: Option<Arc<bboxy450_term::model::EventLane>>,
    task_regions: bbt_ui::modules::stream_dock::Regions,
    reject: Option<(bbt_mux::Ulid, composer::RejectController)>,
    reject_report: Option<composer::Report>,
    reject_focus: bool,
    conversation: bbt_ui::modules::stream_dock::StreamDock,
    conversation_scroll: f32,
    conversation_layout: Option<scene::ConversationLayout>,
    composer_dragging: bool,
    compose_startup: bool,
    operation_backlog: VecDeque<live::UiMsg>,
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
    /// Operator captures waiting for the next presented frame.
    ctl_captures: Vec<live::CaptureAsk>,
    /// When the last frame was presented; a capture of it names this time.
    last_presented: Option<std::time::SystemTime>,
    pending_resize: Option<LogicalSize<f64>>,
    animate: bool,
    cadence_test: bool,
    animation_start: Option<(Instant, u64)>,
    frozen_ms: f32,
    verify: Verify,
    stall: Option<stall::Native>,
}
impl App {
    fn open_local_tab(&mut self, tab: LocalTab) {
        if self.rim_preview.take().is_some() {
            self.apply_preferences();
        }
        self.close_launcher();
        self.blur_reject();
        self.composer.handle(ComposeCommand::Blur);
        self.composer_dragging = false;
        self.launcher_parent = None;
        self.local_tab = Some(tab);
        if let Some(commands) = &self.live_commands {
            commands.observe_knowledge(matches!(self.local_tab, Some(LocalTab::Graph(_))));
        }
        self.local_page_layout = None;
        if let Some(window) = &self.window {
            window.set_ime_allowed(false);
        }
        self.logger.write(
            if matches!(self.local_tab, Some(LocalTab::Settings { .. })) {
                "local tab opened: Settings"
            } else {
                "local tab opened: Graph"
            },
        );
        self.request_redraw();
    }
    fn open_graph(&mut self) {
        if !matches!(self.local_tab, Some(LocalTab::Graph(_))) {
            match e2e_checklist::Graph::new(vec![], vec![]) {
                Ok(graph) => self.open_local_tab(LocalTab::Graph(graph)),
                Err(error) => self.logger.write(format!("Graph refused: {error}")),
            }
        } else {
            self.close_launcher();
            self.blur_reject();
            self.composer.handle(ComposeCommand::Blur);
            self.composer_dragging = false;
            if let Some(window) = &self.window {
                window.set_ime_allowed(false);
            }
            self.request_redraw();
        }
    }
    fn open_settings(&mut self, section: &str) {
        let section = bbt_ui::modules::settings::SECTIONS
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case(section))
            .map_or("Account", |(name, _)| *name);
        if let Some(LocalTab::Settings { controller, .. }) = &mut self.local_tab {
            controller.handle(
                settings_controller::Action::Section(section.into()),
                &mut Default::default(),
                &Default::default(),
            );
            self.close_launcher();
            if let Some(window) = &self.window {
                window.set_ime_allowed(false);
            }
            self.request_redraw();
            return;
        }
        let swarm = self.focus.stage.as_ref().and_then(|s| s.swarm);
        self.open_local_tab(LocalTab::Settings {
            swarm,
            controller: settings_controller::Controller::new(section),
        });
    }
    fn effective_preferences(&self) -> preferences::Preferences {
        let mut values = self.preferences.preferences.clone();
        if let Some(rim) = self.rim_preview {
            values.appearance.rim = rim;
        }
        values
    }
    fn settings_view(&self, section: &str) -> Settings {
        Settings {
            section: section.into(),
            values: preferences::records(self.live_state.as_deref(), &self.preferences.preferences),
            defaults: preferences::records(
                self.live_state.as_deref(),
                &preferences::Preferences::default(),
            ),
            ..Default::default()
        }
    }
    fn settings_ime(&self) -> bool {
        matches!(&self.local_tab,Some(LocalTab::Settings {controller,..}) if controller.editing())
    }
    fn sync_text_ime(&self) {
        if let Some(window) = &self.window {
            window.set_ime_allowed(
                self.launcher.is_some()
                    || self.composer.view.focus
                    || self.reject_focus
                    || self.settings_ime(),
            );
        }
    }
    fn settings_action(&mut self, action: settings_controller::Action) {
        let pointer_drag = matches!(action, settings_controller::Action::Drag(_));
        let commit_drag = matches!(action, settings_controller::Action::EndDrag)
            .then(|| self.rim_preview.take())
            .flatten();
        let Some(LocalTab::Settings { controller, .. }) = &self.local_tab else {
            return;
        };
        let mut view = self.settings_view(&controller.section);
        let Some(LocalTab::Settings { controller, .. }) = &mut self.local_tab else {
            return;
        };
        let patch = controller
            .handle(action, &mut view.values, &view.defaults)
            .or_else(|| commit_drag.map(settings_controller::Patch::Rim));
        if pointer_drag {
            if let Some(settings_controller::Patch::Rim(rim)) = patch {
                self.rim_preview = Some(rim);
                let retained = self.local_page_layout.take();
                self.apply_preferences();
                self.local_page_layout = retained;
            }
            self.request_redraw();
            return;
        }
        if let Some(patch) = patch {
            match self.preferences.edit(patch) {
                Ok(request) => match &self.preferences_io {
                    Some(io) => {
                        if let Err(reason) = io.submit(request) {
                            self.preferences.refused(reason);
                        }
                    }
                    None => self
                        .preferences
                        .refused("Settings persistence is unavailable"),
                },
                Err(reason) => self.preferences.refused(reason),
            }
            self.apply_preferences();
        }
        self.sync_text_ime();
        if !pointer_drag {
            self.local_page_layout = None;
        }
        self.request_redraw();
    }
    fn settings_key(&mut self, key: SettingsKey) {
        use settings_controller::{Action, Focus};
        let action = match key {
            SettingsKey::Reload => {
                let request = self.preferences.reload();
                if let Some(io) = &self.preferences_io
                    && let Err(reason) = io.submit(request)
                {
                    self.preferences.refused(reason);
                }
                self.request_redraw();
                return;
            }
            SettingsKey::Edit(edit) => Action::Search(edit),
            SettingsKey::Next(back) => {
                if let (Some(LocalTab::Settings { controller, .. }), Some(regions)) = (
                    &self.local_tab,
                    self.local_page_layout
                        .as_ref()
                        .and_then(|p| p.settings.as_ref()),
                ) {
                    let mut order = vec![Focus::Search, Focus::Navigation];
                    order.extend(regions.fields().into_iter().map(Focus::Control));
                    let at = order
                        .iter()
                        .position(|f| *f == controller.focus)
                        .unwrap_or(if back { 0 } else { order.len() - 1 });
                    Action::Focus(
                        order[(at + if back { order.len() - 1 } else { 1 }) % order.len()].clone(),
                    )
                } else {
                    Action::NextFocus(back)
                }
            }
            SettingsKey::Nudge(n) => {
                if matches!(&self.local_tab,Some(LocalTab::Settings {controller,..}) if controller.focus==Focus::None)
                {
                    self.local_scroll(n as f32 * 28.);
                    return;
                }
                Action::Nudge(n)
            }
            SettingsKey::Activate => {
                let Some(LocalTab::Settings { controller, .. }) = &self.local_tab else {
                    return;
                };
                let Focus::Control(field) = &controller.focus else {
                    return;
                };
                Action::Toggle(field.clone())
            }
        };
        self.settings_action(action);
    }
    fn apply_preferences(&mut self) {
        let system = self.verify.theme.unwrap_or(self.system_theme);
        self.scene
            .set_appearance(&self.effective_preferences(), system);
        if let Some(model) = &self.live_state
            && let Some(stage) = &self.focus.stage
        {
            self.scene.set_model(model, stage);
        }
        if let Some(commands) = &self.live_commands {
            let ctx = self.scene.ui_context();
            commands.set_context(preferences::ContextUpdate {
                theme: ctx.theme,
                density: ctx.density,
                snapshot: self.preferences.snapshot(),
            });
        }
        self.local_page_layout = None;
        self.launcher_layout = None;
        self.conversation_layout = None;
        self.sync_notifications();
    }
    fn mute_focused(&mut self) {
        if self.mute.pending() {
            return;
        }
        let visible = self.local_tab.is_none()
            && self.launcher.is_none()
            && !self.composer.view.focus
            && !self.reject_focus;
        if let Some(message) = mute_command(
            self.live_state.as_deref(),
            self.focus.stage.as_ref(),
            &self.pane_bounds,
            visible,
        ) {
            self.mute.requested(&message);
            self.composer_notice();
            self.queue(message);
            self.request_redraw();
        }
    }
    fn close_local_tab(&mut self) {
        if self.rim_preview.take().is_some() {
            self.apply_preferences();
        }
        if let Some(commands) = &self.live_commands {
            commands.observe_knowledge(false);
        }
        self.local_tab = None;
        self.sync_text_ime();
        self.local_page_layout = None;
        self.logger.write("local tab closed: returning to panes");
        self.request_redraw();
    }
    fn local_scroll(&mut self, delta: f32) {
        let max = self
            .local_page_layout
            .as_ref()
            .map_or(0., |page| page.max_scroll);
        let changed = match &mut self.local_tab {
            Some(LocalTab::Graph(graph)) => graph.scroll_by(delta, max),
            Some(LocalTab::Settings { controller, .. }) => controller.scroll_by(delta, max),
            None => false,
        };
        if changed {
            self.request_redraw();
        }
    }
    fn local_page_command(&mut self, command: PageCommand) {
        let page = self.local_page_layout.as_ref().map_or(0., |p| p.viewport.h);
        match command {
            PageCommand::Close => self.close_local_tab(),
            PageCommand::Up => self.local_scroll(-28.),
            PageCommand::Down => self.local_scroll(28.),
            PageCommand::PageUp => self.local_scroll(-page),
            PageCommand::PageDown => self.local_scroll(page),
            PageCommand::Top => self.local_scroll(-f32::MAX),
            PageCommand::Bottom => self.local_scroll(f32::MAX),
        }
    }
    fn composer_catalog(&mut self) {
        let Some(model) = &self.live_state else {
            return;
        };
        let Some(stage) = &self.focus.stage else {
            return;
        };
        let Some((channel, mut choices)) = messaging::catalog(model, stage) else {
            return;
        };
        if self.composer_scope != stage.swarm {
            if self.composer_scope.is_some() {
                self.composer.view.to.clear();
                self.composer_client
                    .refused("swarm changed; choose a destination");
            } else if self.composer.view.to.is_empty() {
                self.composer.view.to = vec![channel];
            }
            self.composer_scope = stage.swarm;
            self.task_records
                .select(stage.swarm.expect("catalog has swarm"));
            self.message_history
                .select(stage.swarm.expect("catalog has swarm"));
            self.events_seen = None;
            self.reject = None;
            self.reject_report = None;
            self.reject_focus = false;
            self.conversation_scroll = 0.;
        }
        self.conversation_events();
        for (folder, title, kind) in [
            ("channels/", "Channels", "channel"),
            ("groups/", "Groups", "group"),
        ] {
            let items: Vec<_> = self
                .task_records
                .nodes()
                .iter()
                .filter(|n| n.path.starts_with(folder))
                .map(|n| composer::Address {
                    kind: kind.into(),
                    id: n.path.clone(),
                    label: n.path.trim_start_matches(folder).into(),
                    destination: n.path.clone(),
                    ..Default::default()
                })
                .collect();
            if let Some(group) = choices.iter_mut().find(|g| g.title == title) {
                group.items.extend(items);
            } else if !items.is_empty() {
                choices.push(composer::PickGroup {
                    title: title.into(),
                    items,
                });
            }
        }
        self.composer.replace_choices(choices);
        self.composer.replace_attachments(
            self.task_records
                .tasks()
                .iter()
                .chain(self.task_records.unassigned())
                .map(messaging::task_attachment)
                .chain(self.message_history.artifacts().cloned())
                .collect(),
        );
    }
    fn conversation_events(&mut self) {
        let Some(model) = &self.live_state else {
            return;
        };
        if self
            .events_seen
            .as_ref()
            .is_some_and(|old| Arc::ptr_eq(old, &model.events))
        {
            return;
        }
        if self
            .events_seen
            .as_ref()
            .is_some_and(|old| !matches!(old.state(), bboxy450_term::model::EventLaneState::Active))
            && matches!(
                model.events.state(),
                bboxy450_term::model::EventLaneState::Active
            )
        {
            self.message_history.reconnect();
            self.task_records.refresh();
        }
        for task in self.message_history.observe(&model.events) {
            self.task_records.task_notice(task);
        }
        self.events_seen = Some(model.events.clone());
    }
    fn conversation_reads(&mut self, viewed: bool) {
        if viewed && let Some(action) = self.message_history.viewed_all() {
            self.operation_backlog.push_back(action.into());
        }
        self.operation_backlog.extend(
            self.message_history
                .actions()
                .into_iter()
                .map(live::UiMsg::from),
        );
        self.operation_backlog
            .extend(self.message_history.commands());
    }
    fn blur_reject(&mut self) {
        self.reject_focus = false;
        if let Some((_, reject)) = &mut self.reject {
            reject.focus(false);
        }
    }
    fn refresh_conversation(&mut self) {
        let Some(model) = self.live_state.as_deref() else {
            return;
        };
        self.conversation.entries = self.message_history.entries(
            model,
            self.composer_client.groups(),
            self.task_records.nodes(),
        );
        self.conversation.entries.extend(
            self.task_records
                .entries(model, self.reject.as_ref().map(|(id, r)| (*id, &r.view))),
        );
        if let Some((id, editor)) = &self.reject
            && !self.conversation.entries.iter().any(|entry| matches!(entry, bbt_ui::modules::stream_dock::Entry::Report {id: actual,..} if actual == &id.to_string()))
            && let Some(snapshot) = &self.reject_report {
            let mut report = snapshot.clone();
            if let Some(card) = self.task_records.task(*id) { report.status = card.status.clone(); report.task = card.title.clone(); }
            let mut reason = editor.view.clone();
            if reason.notice.is_none() { reason.notice = Some(composer::Notice {kind:"refused".into(),text:"Previous report · refreshing · reason kept".into()}); }
            report.reject = Some(reason);
            self.conversation.entries.push(bbt_ui::modules::stream_dock::Entry::Report {id:id.to_string(),data:report,review:false});
        }
        self.scene.set_tasks(
            self.task_records
                .tasks()
                .iter()
                .filter_map(|card| {
                    let seat = messaging::assignee_seat(card.assignee.as_deref()?)?;
                    let (row, exit) = messaging::task_row(card);
                    Some((seat, card.id.to_string(), row, exit))
                })
                .collect(),
        );
        let unread = model
            .ui
            .seats
            .iter()
            .filter(|(_, seat)| Some(seat.swarm) == self.composer_scope)
            .map(|(id, _)| (*id, self.message_history.seat_unread(&id.to_string())))
            .collect();
        self.scene.set_unread(&unread);
        self.conversation_layout = None;
    }
    fn task_action(&mut self, action: bbt_ui::modules::stream_dock::Action) {
        use bbt_ui::modules::stream_dock::Action;
        let parse = |id: &str| id.parse::<bbt_mux::Ulid>().ok();
        match action {
            Action::Task(id) => {
                if let Some(id) = parse(&id) {
                    self.blur_reject();
                    self.task_records.detail(id);
                }
            }
            Action::Delegate(id) => {
                if let Some(task) = parse(&id).and_then(|id| self.task_records.task(id)) {
                    let attachment = messaging::task_attachment(task);
                    self.blur_reject();
                    self.composer_command(ComposeCommand::Delegate(attachment));
                }
            }
            Action::Accept(id) => {
                if let Some(id) = parse(&id) {
                    self.review_task(id, None);
                }
            }
            Action::Reject(id) => {
                if let Some(id) = parse(&id) {
                    let snapshot = self
                        .task_records
                        .task(id)
                        .zip(self.task_records.doc(id))
                        .and_then(|(card, doc)| messaging::task_report(card, doc).ok().flatten());
                    if let Some(snapshot) = snapshot {
                        self.composer.handle(ComposeCommand::Blur);
                        if self.reject.as_ref().is_none_or(|(old, _)| *old != id) {
                            self.reject = Some((id, composer::RejectController::default()));
                        }
                        self.reject_report = Some(snapshot);
                        if let Some((_, reject)) = &mut self.reject {
                            reject.focus(true);
                        }
                        self.reject_focus = true;
                    } else {
                        self.task_records.detail(id);
                        self.task_records.issue = Some("Report is refreshing".into());
                    }
                }
            }
            Action::CancelReject(_) => {
                self.reject = None;
                self.reject_report = None;
                self.reject_focus = false;
            }
            Action::SubmitReject(id) => {
                if let Some(id) = parse(&id)
                    && let Some((actual, reason)) = &mut self.reject
                    && *actual == id
                {
                    match reason.submission() {
                        Ok(reason) => self.review_task(id, Some(reason)),
                        Err(reason) => self.reject.as_mut().unwrap().1.refused(reason),
                    }
                }
            }
            Action::RejectInput(id) => {
                if let Some(id) = parse(&id)
                    && let Some((actual, reason)) = &mut self.reject
                    && *actual == id
                {
                    self.composer.handle(ComposeCommand::Blur);
                    self.reject_focus = true;
                    reason.focus(true);
                    if let Some(layout) = &self.conversation_layout
                        && let Some(at) = self.scene.reject_index(
                            layout,
                            &id.to_string(),
                            &reason.view.reason,
                            self.pointer,
                        )
                    {
                        reason.handle(EditCommand::SetCaret {
                            at,
                            extend: self.modifiers.shift_key(),
                        });
                    }
                    self.composer_dragging = true;
                }
            }
            Action::Channel(name) => {
                self.message_history.channel(&if name == "all" {
                    name
                } else {
                    format!("channels/{}", name.trim_start_matches("channels/"))
                });
                self.conversation_scroll = 0.;
                self.conversation_reads(true);
            }
        }
        self.operation_backlog.extend(self.task_records.commands());
        self.refresh_conversation();
        if let Some(window) = &self.window {
            window.set_ime_allowed(
                self.launcher.is_some() || self.composer.view.focus || self.reject_focus,
            );
        }
        self.request_redraw();
    }
    fn review_task(&mut self, id: bbt_mux::Ulid, reason: Option<String>) {
        if self.backend_lost || self.closing || self.live_commands.is_none() {
            self.task_records.issue = Some("backend_unavailable".into());
            if let Some((actual, editor)) = &mut self.reject
                && *actual == id
            {
                editor.refused("backend_unavailable");
            }
            return;
        }
        match self.task_records.review(id, reason) {
            Ok(command) => self.operation_backlog.push_back(command),
            Err(reason) => {
                self.task_records.issue = Some(reason.into());
                if let Some((actual, edit)) = &mut self.reject
                    && *actual == id
                {
                    edit.refused(reason);
                }
            }
        }
    }
    fn composer_notice(&mut self) {
        if let Some(notice) = &self.mute.notice {
            self.composer.view.notice = Some(notice.clone());
            return;
        }
        self.composer.view.notice = match &self.composer_client.status {
            messaging::Status::Idle => self.mute.notice.clone(),
            messaging::Status::Pending => Some(composer::Notice {
                kind: "pending".into(),
                text: "sending".into(),
            }),
            messaging::Status::Stored { id, sends } => Some(composer::Notice {
                kind: "sent".into(),
                text: if *sends > 1 {
                    format!("stored · {sends} recipients · {id}")
                } else {
                    format!("stored · {id}")
                },
            }),
            messaging::Status::Held(reason) => Some(composer::Notice {
                kind: "held".into(),
                text: reason.clone(),
            }),
            messaging::Status::Refused(reason) => Some(composer::Notice {
                kind: "refused".into(),
                text: reason.clone(),
            }),
            messaging::Status::Unknown(reason) => Some(composer::Notice {
                kind: "refused".into(),
                text: format!("outcome unknown · Send retries the same operation · {reason}"),
            }),
            messaging::Status::Partial { stored, reason } => Some(composer::Notice {
                kind: "refused".into(),
                text: format!("{stored} stored · {reason}"),
            }),
        };
    }
    fn composer_command(&mut self, command: ComposeCommand) {
        if !self.mute.pending() {
            self.mute.notice = None;
        }

        if self.reject_focus {
            if let Some((_, reason)) = &mut self.reject {
                match command {
                    ComposeCommand::Edit(EditCommand::Insert(text) | EditCommand::Commit(text))
                        if text.contains(['\n', '\r']) => {}
                    ComposeCommand::Edit(edit) => {
                        reason.handle(edit);
                    }
                    ComposeCommand::Escape => {
                        if reason.composing() {
                            reason.handle(EditCommand::CancelPreedit);
                        } else {
                            self.blur_reject();
                        }
                    }
                    ComposeCommand::Focus(field) => {
                        self.blur_reject();
                        self.composer.handle(ComposeCommand::Focus(field));
                    }
                    _ => {}
                }
            }
            self.refresh_conversation();
            if let Some(window) = &self.window {
                window.set_ime_allowed(
                    self.launcher.is_some() || self.composer.view.focus || self.reject_focus,
                );
            }
            self.request_redraw();
            return;
        }
        if matches!(command, ComposeCommand::Focus(ComposeField::Address)) {
            self.composer_catalog();
        }
        if matches!(command, ComposeCommand::Send)
            && !self.composer.composing()
            && matches!(self.composer_client.status, messaging::Status::Unknown(_))
        {
            if !self.backend_lost
                && self.live_commands.is_some()
                && self.operation_backlog.is_empty()
            {
                self.operation_backlog.extend(self.composer_client.retry());
                self.composer_client.retry_admitted();
            }
        } else {
            match self.composer.handle(command) {
                ComposeOutcome::Submit(draft) => {
                    if self.live_commands.is_none() || self.closing || self.backend_lost {
                        self.composer_client.refused("backend_unavailable");
                    } else {
                        let prepared =
                            match self.live_state.as_deref().zip(self.focus.stage.as_ref()) {
                                Some((model, stage)) => {
                                    self.composer_client.prepare_for(model, stage, draft)
                                }
                                None => Err("swarm_required"),
                            };
                        match prepared {
                            Ok(batch) => self
                                .operation_backlog
                                .extend(self.composer_client.admitted(batch)),
                            Err(reason) => self.composer_client.refused(reason),
                        }
                    }
                }
                ComposeOutcome::Refused(reason) => self.composer_client.refused(reason),
                _ => {}
            }
        }
        self.composer_notice();
        if let Some(window) = &self.window {
            window.set_ime_allowed(self.launcher.is_some() || self.composer.view.focus);
        }
        self.request_redraw();
    }
    fn composer_move(&mut self, vertical: Option<bool>, end: Option<bool>, extend: bool) {
        if self.reject_focus {
            if let Some(end) = end {
                self.composer_command(ComposeCommand::Edit(EditCommand::Move {
                    motion: if end {
                        bbt_ui::text_edit::Motion::End
                    } else {
                        bbt_ui::text_edit::Motion::Start
                    },
                    extend,
                }));
            }
            return;
        }
        if self.composer.view.attach_pick.is_some() {
            if let Some(end) = end {
                self.composer_command(ComposeCommand::Edit(EditCommand::Move {
                    motion: if end {
                        bbt_ui::text_edit::Motion::End
                    } else {
                        bbt_ui::text_edit::Motion::Start
                    },
                    extend,
                }));
            }
            return;
        }
        if self.composer.composing() {
            return;
        }
        let Some(layout) = &self.conversation_layout else {
            return;
        };
        let caret = self
            .composer
            .view
            .edit
            .as_ref()
            .and_then(|e| e.caret)
            .unwrap_or(self.composer.view.text.encode_utf16().count());
        let mut at = self
            .scene
            .conversation_position(layout, &self.composer.view.text, caret);
        if let Some(backwards) = vertical {
            at[1] += if backwards { -1. } else { 1. }
                * (STYLE.r#type.caption as f32 * 1.45 * 64.).floor()
                / 64.;
        }
        if let Some(end) = end {
            at[0] = if end { 1_000_000. } else { -1_000_000. };
        }
        let index = self
            .scene
            .conversation_index(layout, &self.composer.view.text, at);
        self.composer_command(ComposeCommand::Edit(EditCommand::SetCaret {
            at: index,
            extend,
        }));
    }
    fn flush_operations(&mut self) {
        if self.backend_lost || self.closing {
            return;
        }
        while self
            .pending_commands
            .iter()
            .filter(|m| is_operation(m))
            .count()
            < PENDING_OPERATIONS
        {
            let Some(message) = self.operation_backlog.pop_front() else {
                break;
            };
            queue_pending(&mut self.pending_commands, message).expect("reserved operation slot");
        }
    }
    fn open_launcher(&mut self) {
        if self.launcher.is_none() {
            self.launcher_parent = if self.reject_focus {
                self.reject.as_ref().map(|(id, _)| TextFocus::Reject(*id))
            } else {
                self.composer
                    .view
                    .focus
                    .then_some(TextFocus::Composer(self.composer.view.field))
            };
        }
        self.blur_reject();
        self.composer.handle(ComposeCommand::Blur);
        self.composer_dragging = false;
        let used = self
            .pending_commands
            .iter()
            .filter(|m| m.is_spawn())
            .count();
        self.launcher = Some(new_launcher_for(
            self.live_state.as_deref(),
            used,
            self.focus.stage.as_ref(),
        ));
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
        match self.launcher_parent.take() {
            Some(TextFocus::Composer(field)) => {
                self.composer.handle(ComposeCommand::Focus(field));
            }
            Some(TextFocus::Reject(id)) => {
                if let Some((actual, reason)) = &mut self.reject
                    && *actual == id
                {
                    reason.focus(true);
                    self.reject_focus = true;
                }
            }
            None => {}
        }
        if let Some(window) = &self.window {
            window.set_ime_allowed(
                self.composer.view.focus || self.reject_focus || self.settings_ime(),
            );
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
                if id == "graph" {
                    self.open_graph();
                    return;
                }
                if id == "settings" || id.starts_with("settings:") {
                    self.open_settings(id.strip_prefix("settings:").unwrap_or("Account"));
                    return;
                }
                if let Some(message) = spawn_pick(
                    &id,
                    modifiers,
                    self.live_state.as_deref(),
                    self.focus.stage.as_ref(),
                ) && self.live_commands.is_some()
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
        if self.local_tab.is_some() {
            self.close_local_tab();
        }
        self.launcher_parent = None;
        self.blur_reject();
        self.composer.handle(ComposeCommand::Blur);
        self.composer_dragging = false;
        if self.launcher.is_none()
            && let Some(window) = &self.window
        {
            window.set_ime_allowed(false);
        }
        let Some(model) = self.live_state.clone() else {
            return;
        };
        match self
            .focus
            .apply(&model.ui, action, &self.focus_bounds, Instant::now())
        {
            Ok(events) => {
                self.scene.set_model(
                    &model,
                    self.focus.stage.as_ref().expect("focus action stage"),
                );
                self.attention_focus_events(&events, true);
                for event in events {
                    self.logger.write(format!("ui input: {event:?}"));
                }
                self.request_redraw();
                self.sync_notifications();
            }
            Err(error) => self.logger.write(format!("focus refused: {error}")),
        }
    }
    fn attention_focus_events(&mut self, events: &[InputEvent], explicit: bool) {
        let Some(model) = &self.live_state else {
            return;
        };
        let now = Instant::now();
        let mut landed: BTreeMap<SeatId, Option<Instant>> = BTreeMap::new();
        if explicit && let Some(seat) = self.focus.stage.as_ref().and_then(|stage| stage.focused) {
            landed.insert(seat, None);
        }
        for event in events {
            if let InputEvent::Locate { seat, .. } = event {
                landed.insert(*seat, Some(now));
            }
        }
        let actions: Vec<_> = landed
            .into_iter()
            .filter_map(|(seat, landed)| {
                let row = model.ui.seats.get(&seat)?;
                Some(live::UiMsg::AttentionFocus {
                    seat,
                    generation: row.process_generation,
                    episode: model.attention_key(seat),
                    landed,
                })
            })
            .collect();
        for action in actions {
            self.queue(action);
        }
    }
    fn sync_notifications(&mut self) {
        let policy = self.preferences.notification_policy();
        if let (Some(worker), Some(model)) = (&mut self.notifications, &self.live_state) {
            let focused = self.focus.stage.as_ref().and_then(|stage| stage.focused);
            let known = model
                .ui
                .seats
                .keys()
                .map(|seat| bboxy450_desktop_notifications::NotificationId(seat.0.to_string()))
                .collect();
            worker.submit(model.notifications(&policy, self.frontmost, focused), known);
        }
    }
    fn notification_clicked(&mut self, id: bboxy450_desktop_notifications::NotificationId) {
        match id.0.parse::<bbt_mux::Ulid>() {
            Ok(id) => {
                self.pending_notification = Some((SeatId::from_u128(id.0), Instant::now()));
                self.apply_notification_click();
            }
            Err(_) => self
                .logger
                .write("notification focus refused: invalid seat id"),
        }
    }
    fn apply_notification_click(&mut self) {
        let Some((seat, at)) = self.pending_notification else {
            return;
        };
        match notification_target(
            seat,
            at,
            Instant::now(),
            self.live_state.as_deref(),
            self.window.is_some(),
        ) {
            NotificationTarget::Pending => return,
            NotificationTarget::Refused(reason) => {
                self.pending_notification = None;
                self.logger
                    .write(format!("notification focus refused: {reason}"));
                return;
            }
            NotificationTarget::Ready => {}
        }
        self.pending_notification = None;
        // The same controller used for a sidebar activation owns tab selection
        // and the deferred Locate after the target tab's layout becomes ready.
        self.focus_action(FocusAction::Activate(seat, FocusOrigin::Row));
        if self.focus.stage.as_ref().and_then(|stage| stage.focused) == Some(seat)
            && let Some(window) = &self.window
        {
            window.set_minimized(false);
            window.focus_window();
            self.logger
                .write(format!("notification focused seat {seat}"));
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
    fn scroll_pane(&mut self, seat: SeatId, request: ScrollRequest) {
        let Some(generation) = self
            .live_state
            .as_ref()
            .and_then(|model| model.ui.seats.get(&seat))
            .map(|seat| seat.process_generation)
        else {
            return;
        };
        self.queue(live::UiMsg::Scroll {
            seat,
            generation,
            request,
        });
    }
    /// Writes the presented frame for an operator capture, cropped to the seat's pane.
    fn capture_for(&self, ask: live::CaptureAsk, scale: f32, frame: bbt_proto::gui::CaptureFrame) {
        let crop = match ask.seat {
            None => None,
            Some(seat) => match self.pane_bounds.iter().find(|(id, _)| *id == seat) {
                Some((_, rect)) => Some(capture::Crop {
                    x: (rect.x * scale).round() as u32,
                    y: (rect.y * scale).round() as u32,
                    width: (rect.w * scale).round() as u32,
                    height: (rect.h * scale).round() as u32,
                }),
                None => {
                    self.answer_capture(ask.id, Err(format!("seat {seat} has no pane on screen")));
                    return;
                }
            },
        };
        let Some(sender) = self.live_commands.as_ref().map(|sender| (**sender).clone()) else {
            return;
        };
        let id = ask.id;
        self.gpu
            .as_ref()
            .unwrap()
            .capture(ask.path, crop, self.logger.clone(), move |result| {
                bbt_rt::handle().spawn(async move {
                    let _ = sender
                        .send(live::UiMsg::Captured {
                            id,
                            error: result.err(),
                            frame,
                        })
                        .await;
                });
            });
    }
    /// Answers a capture that cannot wait for a presented frame: the last composed frame
    /// stays in the render target, so it is encoded as `--capture` would; with none, the
    /// reason is the answer.
    fn capture_now(&self, ask: live::CaptureAsk, step: CaptureStep) {
        match step {
            CaptureStep::Refuse(reason) => self.answer_capture(ask.id, Err(reason.into())),
            _ => {
                let scale = self.window.as_ref().map_or(1., |w| w.scale_factor() as f32);
                self.logger.write(format!(
                    "capture {}: no frame presented in time (occluded={}); encoding the last one",
                    ask.id, self.occluded
                ));
                let frame = bbt_proto::gui::CaptureFrame {
                    stale: true,
                    occluded: self.occluded,
                    presented_at_ms: unix_ms(self.last_presented),
                };
                self.capture_for(ask, scale, frame);
            }
        }
    }
    fn answer_capture(&self, id: bbt_mux::Ulid, result: Result<(), String>) {
        if let Some(sender) = &self.live_commands {
            let _ = sender.try_send(live::UiMsg::Captured {
                id,
                error: result.err(),
                frame: Default::default(),
            });
        }
    }
    fn flush_commands(&mut self) {
        self.flush_operations();
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
        if let Some(stall) = &mut self.stall {
            stall.cancel();
            return;
        }
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
        if self.launcher.is_some() || self.composer.view.focus {
            window.set_ime_allowed(true);
            self.logger.write("debug text field open");
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
        self.system_theme = if window.theme() == Some(winit::window::Theme::Light) {
            Theme::Light
        } else {
            Theme::Dark
        };
        let theme = self
            .preferences
            .preferences
            .context(self.verify.theme.unwrap_or(self.system_theme))
            .theme;
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
        self.apply_notification_click();
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
            self.apply_preferences();
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
        if let Some(stall) = &self.stall {
            stall.drain();
        }
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
            self.mute.observe(&state);
            self.composer_notice();
            if let Some(snapshot) = &state.e2e
                && !self
                    .live_state
                    .as_ref()
                    .and_then(|old| old.e2e.as_ref())
                    .is_some_and(|old| Arc::ptr_eq(old, snapshot))
                && let Some(LocalTab::Graph(graph)) = &mut self.local_tab
            {
                graph.checklist = snapshot.clone();
                self.local_page_layout = None;
            }
            if let Some(stall) = &mut self.stall {
                stall.model_received(&state);
            }
            self.spawn_results_seen
                .retain(|op, _| state.spawns.contains_key(op));
            let results: Vec<_> = state
                .spawns
                .iter()
                .filter_map(|(&op, request)| {
                    (!request.consumed
                        && request.outcome.is_result()
                        && self.spawn_results_seen.get(&op) != Some(&request.outcome))
                    .then_some((op, request.outcome.clone()))
                })
                .collect();
            self.focus
                .reconcile(self.live_state.as_ref().map(|old| &old.ui), &state.ui);
            let refresh = self.live_state.as_ref().is_none_or(|old| {
                old.hosts != state.hosts
                    || old.ui.seats.keys().ne(state.ui.seats.keys())
                    || (self.launcher.is_some()
                        && swarm_oracle(Some(old), self.focus.stage.as_ref())
                            != swarm_oracle(Some(&state), self.focus.stage.as_ref()))
            });
            if refresh && let Some(launcher) = &mut self.launcher {
                launcher.replace_items(launcher_items_for(Some(&state), self.focus.stage.as_ref()));
                self.launcher_layout = None;
            }
            self.scene
                .set_model(&state, self.focus.stage.as_ref().expect("published stage"));
            let (completed, follow_up) = self.composer_client.replies(&state.operation_results);
            if let Some(draft) = completed {
                if draft.delegate {
                    self.task_records.refresh();
                }
                self.composer.stored(&draft);
                self.conversation_layout = None;
            }
            self.operation_backlog.extend(follow_up);
            self.composer_notice();
            self.live_state = Some(state);
            let records_changed = self
                .task_records
                .replies(&self.live_state.as_ref().unwrap().operation_results);
            self.message_history
                .replies(&self.live_state.as_ref().unwrap().operation_results);
            if records_changed
                && let Some((id, reason)) = &mut self.reject
                && let Some((actual, issue)) = &self.task_records.review_issue
                && *actual == *id
            {
                reason.refused(issue);
            }
            if let Some((task, submitted)) = self.task_records.take_review_completion()
                && let Some((id, editor)) = &self.reject
                && *id == task
                && submitted.as_deref() == Some(editor.view.reason.as_str())
                && !editor.composing()
            {
                self.reject = None;
                self.reject_report = None;
                self.reject_focus = false;
            }
            self.composer_catalog();
            self.conversation_reads(false);
            self.operation_backlog.extend(self.task_records.commands());
            self.refresh_conversation();
            for (op, outcome) in results {
                if let bboxy450_term::model::SpawnOutcome::Placed(seat) = &outcome {
                    self.focus_action(FocusAction::Activate(*seat, FocusOrigin::Keyboard));
                } else {
                    self.logger
                        .write(format!("spawn {op} outcome: {outcome:?}"));
                }
                self.spawn_results_seen.insert(op, outcome);
                self.queue(live::UiMsg::ConsumeSpawn { id: op });
            }
            self.apply_notification_click();
            self.sync_notifications();
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
        self.conversation_layout = if self.live_state.is_some() || self.compose_startup {
            Some(self.scene.conversation_layout(
                logical,
                &composer::Conversation {
                    stream: self.conversation.clone(),
                    composer: self.composer.view.clone(),
                    scroll: self.conversation_scroll,
                },
            )?)
        } else {
            None
        };
        let dock = self.conversation_layout.as_ref().map_or(0., |l| l.height);
        let layouts = if self.local_tab.is_some() {
            self.scene.local_page_chrome(logical, dock)?
        } else {
            self.scene.layout_with_dock(
                logical,
                self.animation_start.is_some() || self.animate,
                dock,
            )?
        };
        self.local_page_layout = self
            .local_tab
            .as_ref()
            .map(|tab| match tab {
                LocalTab::Graph(graph) => {
                    self.scene
                        .graph_page_layout(logical, dock, &graph.checklist, graph.scroll)
                }
                LocalTab::Settings { controller, swarm } => {
                    let view = self.settings_view(&controller.section);
                    let issue = self
                        .preferences
                        .error
                        .as_deref()
                        .or(controller.error.as_deref());
                    let status = issue.unwrap_or(if self.preferences.saved() {
                        "saved"
                    } else if !self.preferences.loaded {
                        "loading"
                    } else {
                        "saving"
                    });
                    let status = if self.focus.stage.as_ref().and_then(|s| s.swarm) != *swarm {
                        format!("{status} · local to original swarm")
                    } else {
                        status.into()
                    };
                    self.scene.settings_page_layout(
                        logical,
                        dock,
                        &view,
                        &controller.presentation(),
                        &status,
                    )
                }
            })
            .transpose()?;
        if let (Some(LocalTab::Graph(graph)), Some(layout)) =
            (&mut self.local_tab, &self.local_page_layout)
        {
            graph.scroll = graph.scroll.min(layout.max_scroll);
        }
        if let (Some(LocalTab::Settings { controller, .. }), Some(layout)) =
            (&mut self.local_tab, &self.local_page_layout)
        {
            controller.scroll = controller.scroll.min(layout.max_scroll);
        }
        let bounds = self.scene.pane_bounds(&layouts);
        self.task_regions = self.scene.task_regions(&layouts);
        self.focus_bounds = self.scene.focus_bounds(&layouts);
        if let Some(model) = &self.live_state
            && self.local_tab.is_none()
        {
            let events = self.focus.layout_ready(&model.ui, &self.focus_bounds);
            self.attention_focus_events(&events, false);
            for event in events {
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
        if self.launcher.is_none()
            && (self.composer.view.focus || self.reject_focus)
            && let Some(layout) = &self.conversation_layout
        {
            let caret = if self.reject_focus {
                self.reject
                    .as_ref()
                    .and_then(|(id, _)| layout.actions.candidate_area(&id.to_string()))
                    .unwrap_or_else(|| layout.regions.candidate_area())
            } else {
                layout.regions.candidate_area()
            };
            self.window.as_ref().unwrap().set_ime_cursor_area(
                winit::dpi::LogicalPosition::new(caret.x, caret.y),
                winit::dpi::LogicalSize::new(caret.w, caret.h),
            );
        }
        if self.launcher.is_none()
            && self.settings_ime()
            && let Some(regions) = self
                .local_page_layout
                .as_ref()
                .and_then(|page| page.settings.as_ref())
        {
            let caret = regions.caret.unwrap_or(regions.search);
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
        let mut ops = self.scene.paint(&layouts, logical, None, None);
        if let Some(page) = &self.local_page_layout {
            ops.extend(page.paint());
        }
        if !live {
            ops.extend(self.scene.foreground_with_conversation(
                logical,
                self.launcher_layout.as_ref(),
                overlay.as_ref(),
                self.conversation_layout.as_ref(),
            ));
        }
        let pane_overlay = if let Some(model) = &self.live_state {
            let mut ops = self
                .scene
                .history_foreground(model, &self.pane_bounds, scale)?;
            ops.extend(self.scene.foreground_with_conversation(
                logical,
                self.launcher_layout.as_ref(),
                overlay.as_ref(),
                self.conversation_layout.as_ref(),
            ));
            ops
        } else {
            vec![]
        };
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
                overlay: pane_overlay,
            }),
        )? {
            phases.encode = encode;
            phases.present_wait = wait;
            if let Some((phase, time)) = self.meter.record(Instant::now(), phases) {
                self.logger.write(format!("hitch frame={} phase={phase} {:.3}ms cpu={:.3}ms drain={:.3} layout={:.3} draw={:.3} encode={:.3} present_wait={:.3}",self.meter.total_frames(),time.as_secs_f64()*1000.,phases.cpu().as_secs_f64()*1000.,phases.drain.as_secs_f64()*1000.,phases.layout.as_secs_f64()*1000.,phases.draw.as_secs_f64()*1000.,phases.encode.as_secs_f64()*1000.,phases.present_wait.as_secs_f64()*1000.));
            }
            // Under the meter, the render-pass mix of every 120th frame (R12-3 profiling).
            if self.overlay && self.meter.total_frames().is_multiple_of(120) {
                let p = self.gpu.as_ref().unwrap().passes;
                self.logger.write(format!(
                    "passes frame={} total={} flat={} clear={} path={} group={} blur={} material={} cells={} copy={}",
                    self.meter.total_frames(), p.passes(), p.flat, p.clear, p.path, p.group, p.blur, p.material, p.cells, p.copy
                ));
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
            if let Some(stall) = &mut self.stall
                && let Some(model) = &self.live_state
            {
                let cpu = bbt_rt::fault::thread_cpu_time().map_err(|e| e.to_string())?;
                stall.frame(
                    self.meter.total_frames(),
                    model,
                    &self.pane_bounds,
                    phases,
                    cpu,
                    scale,
                );
            }
            let stall = &mut self.stall;
            self.gpu
                .as_ref()
                .unwrap()
                .collect_gpu(&mut self.budget, |id, at| {
                    if let Some(stall) = stall {
                        stall.gpu_complete(id, at);
                    }
                });
            if !self.animate {
                // Idle frames pause the link, except while a swell or attention loop runs.
                let swelling = self.scene.animating(Instant::now()) && !self.occluded;
                self.pacer.as_ref().unwrap().set_active(swelling)?;
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
                    .capture(path.clone(), None, self.logger.clone(), |_| {});
                if self.verify.grid_hash {
                    self.dump_grid_hash(path.clone(), scale);
                }
            }
            self.last_presented = Some(std::time::SystemTime::now());
            for ask in std::mem::take(&mut self.ctl_captures) {
                let frame = bbt_proto::gui::CaptureFrame {
                    stale: false,
                    occluded: false,
                    presented_at_ms: unix_ms(self.last_presented),
                };
                self.capture_for(ask, scale, frame);
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
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.apply_notification_click();
        event_loop.set_control_flow(
            self.pending_notification
                .map_or(ControlFlow::Wait, |(_, at)| {
                    ControlFlow::WaitUntil(at + Duration::from_secs(30))
                }),
        );
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
            UiEvent::Notification(bboxy450_desktop_notifications::Response::Clicked(id)) => {
                self.notification_clicked(id)
            }
            UiEvent::Notification(bboxy450_desktop_notifications::Response::Dismissed(_)) => {}
            UiEvent::NotificationReady => {
                if let Some(worker) = &mut self.notifications {
                    worker.flush();
                }
            }
            UiEvent::NotificationError(reason) => self.logger.write(reason),
            UiEvent::Settings(snapshot) => {
                if self.preferences.accept(&snapshot) {
                    self.apply_preferences();
                    self.request_redraw();
                }
            }
            UiEvent::StallPoll => {
                let stall = &mut self.stall;
                if let Some(gpu) = &self.gpu {
                    gpu.collect_gpu(&mut self.budget, |id, at| {
                        if let Some(stall) = stall {
                            stall.gpu_complete(id, at);
                        }
                    });
                }
                if let Some(stall) = &mut self.stall {
                    let pending = self.redraw_pending.get()
                        || !self.inbox.messages.is_empty()
                        || !self.inbox.output.is_empty();
                    let result = stall.poll(
                        self.meter.total_frames(),
                        self.pacer.as_ref().map_or(0, DisplayLink::callbacks),
                        pending,
                        self.maximum_hz,
                    );
                    match result {
                        Ok(Some(report)) => {
                            if !report.failures.is_empty() {
                                self.error = Some(format!("r5-gate-failed: {:?}", report.failures));
                            }
                            event_loop.exit();
                        }
                        Err(error) => {
                            self.error.get_or_insert(error);
                            // The R5 IO owner has already completed teardown.
                            // Its final poll may also be the last timer wake.
                            event_loop.exit();
                        }
                        Ok(None) => {}
                    }
                }
            }
            UiEvent::DemoFinished => self.demo_finished = true,
            UiEvent::Capture => {
                self.capture_pending = true;
                self.request_redraw();
            }
            UiEvent::CaptureFor(ask) => {
                if self.gpu.is_none() {
                    self.answer_capture(ask.id, Err("the window is not presenting".into()));
                    return;
                }
                match capture_step(false, self.occluded, self.meter.total_frames()) {
                    CaptureStep::Wait => {
                        let (proxy, id) = (self.proxy.clone(), ask.id);
                        bbt_rt::handle().spawn(async move {
                            tokio::time::sleep(CAPTURE_WAIT).await;
                            let _ = proxy.send_event(UiEvent::CaptureDeadline(id));
                        });
                        self.ctl_captures.push(ask);
                        self.request_redraw();
                    }
                    step => self.capture_now(ask, step),
                }
            }
            UiEvent::CaptureDeadline(id) => {
                if let Some(at) = self.ctl_captures.iter().position(|ask| ask.id == id) {
                    let ask = self.ctl_captures.remove(at);
                    let step = capture_step(true, self.occluded, self.meter.total_frames());
                    self.capture_now(ask, step);
                }
            }
            UiEvent::VerifyScrollTop => {
                self.logger.write(format!(
                    "scroll verification start: frame={}",
                    self.meter.total_frames()
                ));
                self.verify.scroll_frame = Some(self.meter.total_frames());
                self.verify.scroll_due = true;
                self.verify();
            }
            UiEvent::VerifyClose => self.request_close(event_loop),
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
                    self.gpu
                        .as_ref()
                        .unwrap()
                        .collect_gpu(&mut self.budget, |_, _| {});
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
        if self.smoke && !self.cadence_test {
            if let WindowEvent::Occluded(occluded) = &event {
                // A locked display can toggle occlusion throughout the night.
                // The smoke fixture keeps draining/rendering its producer, but
                // occlusion alone must neither repaint nor reset its idle probe.
                self.logger.write(format!(
                    "idle probe ignores native occlusion: occluded={occluded}"
                ));
                return;
            }
            if self.idle_scheduled
                && let Err(error) = self.idle_quiet.native_invalidation(&event)
            {
                self.fail(error, event_loop);
                return;
            }
        }
        match event {
            WindowEvent::Focused(focused) => {
                self.frontmost = focused;
                if !focused {
                    if self.rim_preview.take().is_some() {
                        self.apply_preferences();
                    }
                    if matches!(self.local_tab, Some(LocalTab::Settings { .. })) {
                        self.settings_action(settings_controller::Action::Blur);
                    }
                }
                self.sync_notifications();
            }
            WindowEvent::ThemeChanged(theme) => {
                self.system_theme = if theme == winit::window::Theme::Light {
                    Theme::Light
                } else {
                    Theme::Dark
                };
                self.apply_preferences();
                self.request_redraw();
            }
            WindowEvent::CloseRequested => {
                self.request_close(event_loop);
            }
            WindowEvent::Resized(size) => {
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
                self.request_redraw();
            }
            WindowEvent::Occluded(occluded) => {
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
                if matches!(&self.local_tab,Some(LocalTab::Settings {controller,..}) if controller.dragging())
                {
                    if let Some(fraction) = self
                        .local_page_layout
                        .as_ref()
                        .and_then(|p| p.settings.as_ref())
                        .and_then(|r| r.rim_fraction(self.pointer[0]))
                    {
                        self.settings_action(settings_controller::Action::Drag(fraction));
                    }
                    return;
                }
                if self.composer_dragging && self.launcher.is_none() {
                    if self.reject_focus {
                        if let Some((id, reason)) = &self.reject
                            && let Some(layout) = &self.conversation_layout
                            && let Some(at) = self.scene.reject_index(
                                layout,
                                &id.to_string(),
                                &reason.view.reason,
                                self.pointer,
                            )
                        {
                            self.composer_command(ComposeCommand::Edit(EditCommand::SetCaret {
                                at,
                                extend: true,
                            }));
                        }
                        return;
                    }
                    if let Some(layout) = &self.conversation_layout {
                        let at = if let Some(p) = &self.composer.view.attach_pick {
                            self.scene
                                .conversation_attachment_index(layout, &p.query, self.pointer)
                        } else {
                            self.scene.conversation_index(
                                layout,
                                &self.composer.view.text,
                                self.pointer,
                            )
                        };
                        self.composer_command(ComposeCommand::Edit(EditCommand::SetCaret {
                            at,
                            extend: true,
                        }));
                    }
                    return;
                }
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
            WindowEvent::MouseWheel { delta, .. }
                if self.launcher.is_none()
                    && self
                        .local_page_layout
                        .as_ref()
                        .is_some_and(|p| p.contains(self.pointer)) =>
            {
                let delta = match delta {
                    MouseScrollDelta::LineDelta(_, y) => -y * 28.,
                    MouseScrollDelta::PixelDelta(p) => {
                        -p.y as f32 / self.window.as_ref().map_or(1., |w| w.scale_factor() as f32)
                    }
                };
                self.local_scroll(delta);
            }
            WindowEvent::MouseWheel { .. }
                if self.launcher.is_none()
                    && self
                        .local_page_layout
                        .as_ref()
                        .is_some_and(|p| p.focuses(self.pointer)) => {}
            WindowEvent::MouseWheel { delta, .. }
                if self.launcher.is_none()
                    && self.conversation_layout.as_ref().is_some_and(|layout| {
                        layout.actions.contains("conversation/rows", self.pointer)
                    }) =>
            {
                let dy = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y * 28.,
                    MouseScrollDelta::PixelDelta(p) => {
                        p.y as f32 / self.window.as_ref().map_or(1., |w| w.scale_factor() as f32)
                    }
                };
                let max = self
                    .conversation_layout
                    .as_ref()
                    .and_then(|layout| {
                        bbt_ui::modules::stream_dock::Regions::scroll_extent(
                            &layout.layout,
                            "conversation/rows",
                        )
                    })
                    .map_or(0., |r| r[1]);
                self.conversation_scroll = (self.conversation_scroll - dy).clamp(0., max);
                if dy < 0. {
                    self.message_history.more();
                    self.task_records.more_tasks();
                    self.operation_backlog.extend(self.task_records.commands());
                }
                self.conversation_reads(true);
                self.request_redraw();
            }
            WindowEvent::MouseWheel { delta, .. } if self.launcher.is_none() => {
                if let Some((seat, rect)) = self.focus_bounds.iter().find(|(_, rect)| {
                    self.pointer[0] >= rect.x
                        && self.pointer[0] < rect.right()
                        && self.pointer[1] >= rect.y
                        && self.pointer[1] < rect.bottom()
                }) {
                    let seat = *seat;
                    let rows = self
                        .live_state
                        .as_ref()
                        .and_then(|model| model.screens.get(&seat))
                        .map_or(1, |screen| screen.rows.max(1));
                    let lines = match delta {
                        MouseScrollDelta::LineDelta(_, y) => f64::from(y),
                        MouseScrollDelta::PixelDelta(position) => {
                            let scale = self.window.as_ref().map_or(1., |w| w.scale_factor());
                            position.y / (f64::from(rect.h) / rows as f64 * scale).max(1.)
                        }
                    };
                    if lines.is_finite() {
                        let remainder = self.scroll_wheel.get_or_insert((seat, 0.));
                        if remainder.0 != seat {
                            *remainder = (seat, 0.);
                        }
                        remainder.1 += lines;
                        let whole = remainder.1.trunc() as i64;
                        remainder.1 -= whole as f64;
                        if whole != 0 {
                            self.scroll_pane(seat, ScrollRequest::lines(whole));
                        }
                    }
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
                if let Some(layout) = &self.conversation_layout
                    && layout.regions.contains(self.pointer)
                {
                    use composer::Hit;
                    let hit = layout.regions.hit(self.pointer);
                    let at = if hit == Some(Hit::AttachQuery) {
                        self.composer.view.attach_pick.as_ref().map(|p| {
                            self.scene
                                .conversation_attachment_index(layout, &p.query, self.pointer)
                        })
                    } else if hit == Some(Hit::Body) {
                        Some(self.scene.conversation_index(
                            layout,
                            &self.composer.view.text,
                            self.pointer,
                        ))
                    } else {
                        None
                    };
                    self.blur_reject();
                    match hit {
                        Some(Hit::Body) => {
                            self.composer_command(ComposeCommand::Focus(ComposeField::Body));
                            self.composer_dragging = true;
                            self.composer_command(ComposeCommand::Edit(EditCommand::SetCaret {
                                at: at.unwrap(),
                                extend: self.modifiers.shift_key(),
                            }));
                        }
                        Some(Hit::Address) => {
                            self.composer_command(ComposeCommand::Focus(ComposeField::Address))
                        }
                        Some(Hit::Choose(index)) => {
                            self.composer_command(ComposeCommand::Choose(index))
                        }
                        Some(Hit::Remove(index)) => {
                            self.composer_command(ComposeCommand::Remove(index))
                        }
                        Some(Hit::Priority) => self.composer_command(ComposeCommand::Priority(
                            (self.composer.view.priority + 1) % 10,
                        )),
                        Some(Hit::Kind) => self.composer_command(ComposeCommand::ToggleKindMenu),
                        Some(Hit::KindChoice(index)) => self.composer_command(
                            ComposeCommand::Kind(composer::SEND_KINDS[index].into()),
                        ),
                        Some(Hit::Send) => self.composer_command(ComposeCommand::Send),
                        Some(Hit::Attach) => self.composer_command(ComposeCommand::PickAttachments),
                        Some(Hit::AttachChoice(index)) => {
                            self.composer_command(ComposeCommand::ChooseAttachment(index))
                        }
                        Some(Hit::AttachQuery) => {
                            self.composer_dragging = true;
                            self.composer_command(ComposeCommand::Edit(EditCommand::SetCaret {
                                at: at.unwrap(),
                                extend: self.modifiers.shift_key(),
                            }));
                        }
                        _ => {}
                    }
                    return;
                }
                if self
                    .local_page_layout
                    .as_ref()
                    .is_some_and(|p| p.closes(self.pointer))
                {
                    self.close_local_tab();
                    return;
                }
                if let Some(regions) = self
                    .local_page_layout
                    .as_ref()
                    .and_then(|p| p.settings.as_ref())
                    && regions.hit(self.pointer).is_some()
                {
                    use settings_controller::{Action, Focus, Hit};
                    let action = match regions.hit(self.pointer).unwrap() {
                        Hit::Search => Action::Focus(Focus::Search),
                        Hit::Section(name) => Action::Section(name),
                        Hit::Set(patch) => Action::Set(patch),
                        Hit::Toggle(field) => Action::Toggle(field),
                        Hit::Reset(field) => Action::Reset(field),
                        Hit::Rim => {
                            Action::Drag(regions.rim_fraction(self.pointer[0]).unwrap_or(0.))
                        }
                    };
                    self.blur_reject();
                    self.composer.handle(ComposeCommand::Blur);
                    self.settings_action(action);
                    return;
                }
                if self
                    .local_page_layout
                    .as_ref()
                    .is_some_and(|p| p.focuses(self.pointer))
                {
                    self.blur_reject();
                    self.composer.handle(ComposeCommand::Blur);
                    self.composer_dragging = false;
                    if matches!(self.local_tab, Some(LocalTab::Settings { .. })) {
                        self.settings_action(settings_controller::Action::Blur);
                    }
                    if let Some(window) = &self.window {
                        window.set_ime_allowed(false);
                    }
                    self.request_redraw();
                    return;
                }
                if let Some(action) = self
                    .conversation_layout
                    .as_ref()
                    .and_then(|layout| layout.actions.hit(self.pointer))
                    .or_else(|| self.task_regions.hit(self.pointer))
                {
                    self.task_action(action);
                    return;
                }
                if self.conversation_layout.as_ref().is_some_and(|layout| {
                    layout.actions.contains("conversation/rows", self.pointer)
                }) {
                    self.conversation_reads(true);
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
            } => {
                self.query_dragging = false;
                self.composer_dragging = false;
                if matches!(self.local_tab, Some(LocalTab::Settings { .. })) {
                    self.settings_action(settings_controller::Action::EndDrag);
                }
            }
            WindowEvent::Ime(Ime::Preedit(text, cursor)) => {
                if self.launcher.is_some() {
                    self.launcher_command(preedit_command(text, cursor));
                } else if self.settings_ime() {
                    let LauncherCommand::Preedit { text, caret } = preedit_command(text, cursor)
                    else {
                        unreachable!()
                    };
                    self.settings_action(settings_controller::Action::Search(
                        EditCommand::Preedit { text, caret },
                    ));
                } else if self.composer.view.focus || self.reject_focus {
                    let LauncherCommand::Preedit { text, caret } = preedit_command(text, cursor)
                    else {
                        unreachable!()
                    };
                    self.composer_command(ComposeCommand::Edit(EditCommand::Preedit {
                        text,
                        caret,
                    }));
                }
            }
            WindowEvent::Ime(Ime::Disabled) => {
                if self.settings_ime() {
                    self.settings_action(settings_controller::Action::Search(
                        EditCommand::CancelPreedit,
                    ));
                }
                self.launcher_command(LauncherCommand::Preedit {
                    text: String::new(),
                    caret: None,
                });
                if self.composer.view.focus || self.reject_focus {
                    self.composer_command(ComposeCommand::Edit(EditCommand::CancelPreedit));
                }
            }
            WindowEvent::Ime(Ime::Commit(text)) => {
                if self.launcher.is_some() {
                    self.launcher_command(LauncherCommand::Commit(text));
                } else if self.settings_ime() {
                    self.settings_action(settings_controller::Action::Search(EditCommand::Commit(
                        text,
                    )));
                } else if self.composer.view.focus || self.reject_focus {
                    self.composer_command(ComposeCommand::Edit(EditCommand::Commit(text)));
                }
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
                let routed = if self.local_tab.is_some()
                    && self.launcher.is_none()
                    && !self.composer.view.focus
                    && !self.reject_focus
                {
                    let routed = if matches!(self.local_tab, Some(LocalTab::Settings { .. })) {
                        route_settings(&input, modes.unwrap_or_default(), self.settings_ime())
                    } else {
                        route_local_page(&input, modes.unwrap_or_default())
                    };
                    finish_route(&mut self.keyboard, &input, routed)
                } else if self.launcher.is_none() && (self.composer.view.focus || self.reject_focus)
                {
                    let routed = route_composer(
                        &input,
                        modes.unwrap_or_default(),
                        if self.reject_focus {
                            ComposeField::Body
                        } else if self.composer.view.kind_menu
                            || self.composer.view.attach_pick.is_some()
                        {
                            ComposeField::Address
                        } else {
                            self.composer.view.field
                        },
                    );
                    finish_route(&mut self.keyboard, &input, routed)
                } else {
                    route_input(
                        &mut self.keyboard,
                        &input,
                        modes.unwrap_or_default(),
                        self.launcher
                            .as_ref()
                            .map(|launcher| launcher.launcher.query.as_str()),
                    )
                };
                match routed {
                    RoutedKey::Quit => self.request_close(event_loop),
                    RoutedKey::OpenLauncher => self.open_launcher(),
                    RoutedKey::OpenGraph => self.open_graph(),
                    RoutedKey::OpenSettings => self.open_settings("Account"),
                    RoutedKey::SettingsKey(key) => self.settings_key(key),
                    RoutedKey::MuteFocused => self.mute_focused(),
                    RoutedKey::LocalPage(command) => self.local_page_command(command),
                    RoutedKey::ToggleHud => {
                        self.overlay = !self.overlay;
                        self.logger.write(format!("hitch overlay={}", self.overlay));
                        self.request_redraw();
                    }
                    RoutedKey::Launcher(command) => self.launcher_command(command),
                    RoutedKey::Composer(command) => self.composer_command(command),
                    RoutedKey::ComposerVertical { backwards, extend } => {
                        self.composer_move(Some(backwards), None, extend)
                    }
                    RoutedKey::ComposerLineEdge { end, extend } => {
                        self.composer_move(None, Some(end), extend)
                    }
                    RoutedKey::Pane(navigation) => {
                        self.focus_action(FocusAction::Navigate(navigation))
                    }
                    RoutedKey::Scroll(motion) => {
                        if let Some(seat) = self.focus.stage.as_ref().and_then(|s| s.focused) {
                            let page = self
                                .live_state
                                .as_ref()
                                .and_then(|model| model.screens.get(&seat))
                                .map_or(1, |screen| screen.rows.saturating_sub(1).max(1))
                                as i64;
                            let request = match motion {
                                ScrollKey::PageUp => ScrollRequest::lines(page),
                                ScrollKey::PageDown => ScrollRequest::lines(-page),
                                ScrollKey::Top => ScrollRequest::top(),
                                ScrollKey::Live => ScrollRequest::live(),
                            };
                            self.scroll_pane(seat, request);
                        }
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
const PENDING_SPAWNS: usize = bboxy450_term::model::PENDING_SPAWNS;
const PENDING_OPERATIONS: usize = 32;
fn is_operation(message: &live::UiMsg) -> bool {
    matches!(
        message,
        live::UiMsg::Messaging { .. } | live::UiMsg::Task { .. }
    )
}

#[derive(Debug, PartialEq, Eq)]
enum AdmissionRefused {
    Input,
    Spawn,
    Operation,
}

#[derive(Default)]
struct AdmissionRefusals {
    input: u64,
    spawn: u64,
    operation: u64,
}
impl AdmissionRefusals {
    fn record(&mut self, kind: AdmissionRefused) {
        let count = match kind {
            AdmissionRefused::Input => &mut self.input,
            AdmissionRefused::Spawn => &mut self.spawn,
            AdmissionRefused::Operation => &mut self.operation,
        };
        *count = count.saturating_add(1);
    }
    fn report(&self) -> String {
        format!(
            "admission refused: input={} spawn={} operation={} (pending input limit={} bytes/{} entries; spawn limit={}; operation limit={})",
            self.input,
            self.spawn,
            self.operation,
            PENDING_INPUT_BYTES,
            PENDING_INPUT_ENTRIES,
            PENDING_SPAWNS,
            PENDING_OPERATIONS,
        )
    }
}

fn queue_pending(
    pending: &mut VecDeque<live::UiMsg>,
    mut message: live::UiMsg,
) -> Result<(), AdmissionRefused> {
    if is_operation(&message)
        && pending.iter().filter(|m| is_operation(m)).count() >= PENDING_OPERATIONS
    {
        return Err(AdmissionRefused::Operation);
    }
    if let live::UiMsg::AttentionFocus {
        seat,
        generation,
        episode,
        landed,
    } = &message
        && let Some(index) = pending.iter().position(
            |old| matches!(old, live::UiMsg::AttentionFocus { seat: old, .. } if old == seat),
        )
    {
        if let live::UiMsg::AttentionFocus {
            generation: old_generation,
            episode: old_episode,
            landed: old_landed,
            ..
        } = &mut pending[index]
            && *old_generation == *generation
        {
            *old_episode = *episode;
            *old_landed = landed.or(*old_landed);
            return Ok(());
        }
        pending.remove(index);
    }
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
    if let live::UiMsg::Scroll {
        seat,
        generation,
        request,
    } = &mut message
        && let Some(index) = pending
            .iter()
            .position(|old| matches!(old, live::UiMsg::Scroll { seat: id, .. } if id == seat))
        && let Some(live::UiMsg::Scroll {
            generation: old_generation,
            request: mut old,
            ..
        }) = pending.remove(index)
        && old_generation == *generation
    {
        old.merge(*request);
        *request = old;
    }
    match &message {
        // An admitted key returns live before it is delivered. Older scroll
        // intents are superseded, so a later wheel delta cannot inherit their
        // Top anchor across this input while the worker is backpressured.
        live::UiMsg::Input { seat, .. } => pending
            .retain(|old| !matches!(old,live::UiMsg::Scroll {seat:old_seat,..} if old_seat==seat)),
        live::UiMsg::Resize { seat, .. } => pending.retain(
            |old| !matches!(old, live::UiMsg::Resize { seat: old_seat, .. } if old_seat == seat),
        ),
        // Watched carries the whole visible set, including focused-first
        // ordering. Keep only the latest set and sizes of its visible panes.
        live::UiMsg::Watched(panes) => pending.retain(|old| match old {
            live::UiMsg::Watched(_) => false,
            live::UiMsg::Resize { seat, .. } | live::UiMsg::Scroll { seat, .. } => {
                panes.iter().any(|(id, _)| id == seat)
            }
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
  --compose            focus the inline composer at startup
  --settings SECTION   open the local Settings tab
  --settings-config P  use this preferences TOML path (for isolated probes)
  --capture PNG        write the first presented frame to PNG; Cmd+Shift+S re-captures
  --capture-after SECS re-capture once, SECS after the cockpit's first model
  --spawn-zsh N        open N zsh seats once the cockpit is ready (default zsh picks)
  --theme light|dark   terminal palette for live verification (default: dark)
  --grid-size COLSxROWS fit one live pane by resizing the real window; 10s deadline
  --grid-hash          write actual visible-screen hashes to PNG's .grid.json
  --scroll-top-after SECS scroll panes to the capture's first row (0..300)
  --quit-after SECS    close this verification instance cleanly (0..300)
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
    scroll_after: Option<Duration>,
    quit_after: Option<Duration>,
    scroll_due: bool,
    scrolled: std::collections::BTreeSet<(SeatId, u64)>,
    scroll_frame: Option<u64>,
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
        let seconds = |flag: &str| -> Result<Option<Duration>, String> {
            required(flag)?
                .map(|value| {
                    value
                        .parse::<u64>()
                        .ok()
                        .filter(|seconds| *seconds <= 300)
                        .map(Duration::from_secs)
                        .ok_or_else(|| format!("verification-invalid-delay: {flag} {value}"))
                })
                .transpose()
        };
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
            scroll_after: seconds("--scroll-top-after")?,
            quit_after: seconds("--quit-after")?,
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
            for (after, event) in [
                (self.verify.scroll_after, UiEvent::VerifyScrollTop),
                (self.verify.quit_after, UiEvent::VerifyClose),
            ] {
                if let Some(after) = after {
                    let proxy = self.proxy.clone();
                    bbt_rt::handle().spawn(async move {
                        tokio::time::sleep(after).await;
                        let _ = proxy.send_event(event);
                    });
                }
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
        if self.verify.scroll_due {
            let panes: Vec<_> = self
                .live_state
                .iter()
                .flat_map(|m| {
                    m.screens.keys().filter_map(|seat| {
                        let generation = m.ui.seats.get(seat)?.process_generation;
                        (!self.verify.scrolled.contains(&(*seat, generation)))
                            .then_some((*seat, generation))
                    })
                })
                .collect();
            for (seat, generation) in panes {
                self.verify.scrolled.insert((seat, generation));
                self.scroll_pane(seat, ScrollRequest::top());
            }
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
        let scroll_meter=self.verify.scroll_frame.map(|start| {
            let first=frame_number.saturating_sub(self.meter.samples().len() as u64);
            let samples:Vec<_>=self.meter.samples().iter().enumerate().filter(|(index,_)|first+*index as u64>=start)
                .map(|(_,sample)|sample.phases.cpu().as_secs_f64()*1000.).collect();
            serde_json::json!({"first_frame":start+1,"last_frame":frame_number,"samples":samples.len(),
                "cpu_max_ms":samples.iter().copied().fold(0.,f64::max),"cpu_ms":samples})
        });
        let logger = self.logger.clone();
        let proxy = self.proxy.clone();
        bbt_rt::handle().spawn(async move {
            let panes: Vec<_> = bounds
                .iter()
                .filter_map(|(seat, rect)| {
                    let live = model.screens.get(seat)?;
                    let historical=model.history.get(seat);
                    let screen=historical.map_or(live,|history|&history.screen);
                    let frame = model.frames.get(seat)?;
                    let mut pane=serde_json::json!({"seat":bbt_mux::Ulid::from(seat.0.0).to_string(),"cols":screen.cols,"rows":screen.rows,
                    "cursor":screen.cursor,"modes":screen.modes,"fnv1a64":screen_hash(screen),
                    "crop":[(rect.x*scale).round() as u32,(rect.y*scale).round() as u32,
                        frame.width.min((rect.w*scale).floor() as u32),
                        frame.height.min((rect.h*scale).floor() as u32)]});
                    if let Some(history)=historical {
                        let lines:Vec<String>=screen.cells.iter().map(|row|row.iter().map(|cell|cell.c).collect::<String>().trim_end().to_owned()).collect();
                        pane["history"]=serde_json::json!({"lines":lines,"live_fnv1a64":screen_hash(live),
                            "gaps":history.gaps.iter().map(|gap|serde_json::json!({"row":gap.row,"from":gap.from,"to":gap.to})).collect::<Vec<_>>()});
                    }
                    Some(pane)
                })
                .collect();
            let theme = match model.ui.window.theme { Theme::Dark => "dark", Theme::Light => "light" };
            let mut report = serde_json::json!({"schema":"bboxy-firehose-screen-v1","png":png,
                "theme":theme,"scale":scale,"frame":frame_number,"panes":panes});
            if let Some(scroll_meter)=scroll_meter {report["scroll_meter"]=scroll_meter;}
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
    if let Some(result) = stall::private_entry(&hook_args) {
        return result.map_err(Into::into);
    }
    if hook_args.first().is_some_and(|arg| arg == "plugin") {
        let code = bbt_mux::plugins::run_cli(&hook_args[1..]);
        if code != 0 {
            std::process::exit(code);
        }
        return Ok(());
    }
    // `ctl --help` is ctl's own usage, printed by ctl.
    if hook_args.first().is_none_or(|arg| arg != "ctl")
        && hook_args.iter().any(|arg| arg == "--help" || arg == "-h")
    {
        println!("{HELP}\n{}", stall::HELP);
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
    if hook_args.first().is_some_and(|arg| arg == "ctl") {
        // The operator's door from outside the app (e2e-acceptance §3, decision #260):
        // the run directory comes from BBOXY_RUN_DIR, else the default one.
        let run_dir = match std::env::var_os("BBOXY_RUN_DIR") {
            Some(dir) => std::path::PathBuf::from(dir),
            None => bbt_mux::transport::default_socket_path()?
                .parent()
                .map(std::path::Path::to_path_buf)
                .ok_or("default run directory unavailable")?,
        };
        let endpoint = run_dir.join(bbt_mux::ctl_door::ENDPOINT);
        if let Err(reason) = bboxy450_term::ctl::main(&hook_args[1..], &endpoint) {
            println!("{}", serde_json::json!({"kind": "error", "reason": reason}));
            std::process::exit(1);
        }
        return Ok(());
    }
    let verify = Verify::from_args(&hook_args)?;
    let stall_config = stall::Config::parse(&hook_args)?;
    if let Some(config) = stall_config.as_ref().filter(|config| config.headless) {
        return stall::headless(config.clone()).map_err(Into::into);
    }
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
    // The delegate is installed before run_app finishes application launch.
    // Authorization and native completion waits run only on the bounded worker.
    let callback_proxy = proxy.clone();
    let notifications = match bboxy450_desktop_notifications::open(Box::new(move |response| {
        let _ = callback_proxy.send_event(UiEvent::Notification(response));
    })) {
        Ok(notifier) => {
            let progress = proxy.clone();
            match bboxy450_term::notifications::Worker::start(
                notifier,
                Arc::new(move |event| {
                    let _ = progress.send_event(match event {
                        bboxy450_term::notifications::Event::Ready => UiEvent::NotificationReady,
                        bboxy450_term::notifications::Event::Error(reason) => {
                            UiEvent::NotificationError(reason)
                        }
                    });
                }),
            ) {
                Ok(worker) => Some(worker),
                Err(error) => {
                    logger.write(format!("notifications unavailable: {error}"));
                    None
                }
            }
        }
        Err(error) => {
            logger.write(error.to_string());
            None
        }
    };
    let (render_context, rendering) = tokio::sync::oneshot::channel();
    let (stall, live_commands) = if let Some(config) = stall_config {
        let proxy = proxy.clone();
        (
            Some(stall::Native::start(
                config,
                live_updates.clone(),
                rendering,
                inbox.sender.clone(),
                move || {
                    let _ = proxy.send_event(UiEvent::StallPoll);
                },
            )),
            None,
        )
    } else if fake || animate {
        (None, None)
    } else {
        (
            None,
            Some(live::start(
                live_updates.clone(),
                proxy.clone(),
                logger.clone(),
                args.windows(2)
                    .find(|pair| pair[0] == "--run-dir")
                    .map(|pair| std::path::PathBuf::from(&pair[1])),
                // The operator's ctl door (decision #260): opt-in, closed by default.
                args.iter().any(|arg| arg == "--ctl"),
                rendering,
            )),
        )
    };
    let settings_path = args
        .windows(2)
        .find(|v| v[0] == "--settings-config")
        .map(|v| std::path::PathBuf::from(&v[1]));
    let settings_path = if settings_path.is_some() || (!fake && !animate) {
        Some(settings_path.map_or_else(preferences::default_path, Ok)?)
    } else {
        None
    };
    let preferences_io = settings_path.map(|path| {
        let proxy = proxy.clone();
        preferences::Io::start(path, bbt_rt::handle(), move |snapshot| {
            let _ = proxy.send_event(UiEvent::Settings(snapshot));
        })
    });
    let mut preference_state = preferences::State::default();
    if preferences_io.is_none() {
        preference_state.accept(&preferences::Snapshot {
            loaded: true,
            ..Default::default()
        });
    }
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
        scroll_wheel: None,
        focus: FocusState::default(),
        notifications,
        frontmost: false,
        pending_notification: None,
        spawn_results_seen: BTreeMap::new(),
        launcher: args
            .iter()
            .any(|arg| arg == "--launcher")
            .then(|| new_launcher(None, 0)),
        launcher_layout: None,
        launcher_parent: None,
        preferences: preference_state,
        preferences_io,
        rim_preview: None,
        mute: preferences::Mute::default(),
        system_theme: Theme::Dark,
        local_tab: None,
        local_page_layout: None,
        composer: ComposeController::new(composer::Composer {
            focus: args.iter().any(|arg| arg == "--compose"),
            ..Default::default()
        }),
        composer_client: messaging::Client::default(),
        composer_scope: None,
        task_records: messaging::Records::default(),
        message_history: messaging::History::default(),
        events_seen: None,
        task_regions: Default::default(),
        reject: None,
        reject_report: None,
        reject_focus: false,
        conversation: bbt_ui::modules::stream_dock::StreamDock {
            dock: true,
            framed: false,
            ..Default::default()
        },
        conversation_layout: None,
        conversation_scroll: 0.,
        composer_dragging: false,
        compose_startup: args.iter().any(|arg| arg == "--compose"),
        operation_backlog: VecDeque::new(),
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
        ctl_captures: Vec::new(),
        last_presented: None,
        pending_resize: None,
        animate,
        cadence_test,
        animation_start: None,
        frozen_ms: 450.,
        verify,
        stall,
    };
    if let Some(pair) = args.windows(2).find(|pair| pair[0] == "--settings") {
        app.open_settings(&pair[1]);
    }
    event_loop.run_app(&mut app)?;
    if let Some(error) = app.error {
        return Err(error.into());
    }
    Ok(())
}

fn unix_ms(at: Option<std::time::SystemTime>) -> Option<u64> {
    at?.duration_since(std::time::UNIX_EPOCH)
        .ok()
        .map(|since| since.as_millis() as u64)
}

/// How long an operator capture waits for a presented frame before encoding the last one.
const CAPTURE_WAIT: Duration = Duration::from_secs(1);

#[derive(Debug, PartialEq, Eq)]
enum CaptureStep {
    /// Capture the next presented frame.
    Wait,
    /// No frame will come soon (occluded, or none came in time): encode the last one.
    LastFrame,
    Refuse(&'static str),
}
/// What an operator capture does now: `waited` once `CAPTURE_WAIT` has passed.
fn capture_step(waited: bool, occluded: bool, frames: u64) -> CaptureStep {
    match (waited || occluded, frames) {
        (false, _) => CaptureStep::Wait,
        (true, 0) => CaptureStep::Refuse("window occluded, no frame presented"),
        (true, _) => CaptureStep::LastFrame,
    }
}

#[cfg(test)]
mod capture_step_tests {
    use super::{CaptureStep, capture_step};
    #[test]
    fn an_occluded_window_answers_from_its_last_frame_or_says_why_not() {
        // Visible and presenting: the next frame.
        assert_eq!(capture_step(false, false, 9), CaptureStep::Wait);
        // Occluded: the last composed frame, now; never a timeout.
        assert_eq!(capture_step(false, true, 9), CaptureStep::LastFrame);
        // Visible, but no frame came in time (display asleep): the last frame.
        assert_eq!(capture_step(true, false, 9), CaptureStep::LastFrame);
        // Nothing ever presented: the reason, not a timeout.
        for (waited, occluded) in [(false, true), (true, false), (true, true)] {
            assert_eq!(
                capture_step(waited, occluded, 0),
                CaptureStep::Refuse("window occluded, no frame presented")
            );
        }
    }
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
    use super::{IdleQuiescence, WindowEvent};
    use std::time::{Duration, Instant};
    use winit::dpi::PhysicalSize;

    #[test]
    fn interrupted_idle_windows_never_pass_and_deadline_has_a_named_reason() {
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
    fn occlusion_keeps_the_full_idle_window_and_old_timers_cannot_end_a_new_one() {
        let mut quiet = IdleQuiescence::default();
        let now = Instant::now();
        assert!(!quiet.ready(true, false, (21, 21)));
        assert!(quiet.ready(true, false, (21, 21)));
        let first = quiet.begin(now, (21, 21));
        for event in (0..100).map(|n| WindowEvent::Occluded(n % 2 == 0)) {
            quiet.native_invalidation(&event).unwrap();
        }
        assert_eq!(quiet.invalidations, 0);
        assert_eq!(quiet.finish(first), Some((now, 21, 21)));
        let replacement_start = now + Duration::from_secs(2);
        let replacement = quiet.begin(replacement_start, (21, 21));
        assert!(
            quiet.finish(first).is_none(),
            "old timer cannot shorten the full ten seconds"
        );
        assert_eq!(quiet.finish(replacement), Some((replacement_start, 21, 21)));
        assert!(
            quiet.finish(replacement).is_none(),
            "duplicate timer is harmless"
        );
    }

    #[test]
    fn a_real_resize_fails_the_probe_even_without_any_frame_or_callback() {
        let mut quiet = IdleQuiescence::default();
        let epoch = quiet.begin(Instant::now(), (21, 21));
        let error = quiet
            .native_invalidation(&WindowEvent::Resized(PhysicalSize::new(1600, 960)))
            .unwrap_err();
        assert_eq!(error, "idle probe invalidated by Resized");
        assert_eq!(quiet.invalidations, 1);
        assert!(quiet.finish(epoch).is_none());
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
                live::UiMsg::ConsumeSpawn { .. } | live::UiMsg::Scroll { .. } => {}
                live::UiMsg::Messaging { .. }
                | live::UiMsg::Task { .. }
                | live::UiMsg::EventPage { .. }
                | live::UiMsg::EventCursor { .. }
                | live::UiMsg::AttentionFocus { .. }
                | live::UiMsg::MarkEventsRead { .. }
                | live::UiMsg::Captured { .. } => {
                    panic!("pane admission fixture produced a messaging intent")
                }
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
mod scroll_input_tests {
    use super::*;

    #[test]
    fn admitted_input_resets_a_queued_scroll_anchor_and_refusal_does_not() {
        let seat = SeatId::from_u128(1);
        let scroll = |request| live::UiMsg::Scroll {
            seat,
            generation: 1,
            request,
        };
        let input = |bytes: Vec<u8>| live::UiMsg::Input {
            seat,
            generation: 1,
            id: bbt_mux::Ulid::new(),
            bytes,
        };
        let mut pending = VecDeque::new();
        queue_pending(&mut pending, scroll(ScrollRequest::top())).unwrap();
        queue_pending(&mut pending, input(b"x".to_vec())).unwrap();
        queue_pending(&mut pending, scroll(ScrollRequest::lines(1))).unwrap();
        assert_eq!(pending.len(), 2);
        assert!(matches!(pending.front(),Some(live::UiMsg::Input {bytes,..}) if bytes==b"x"));
        assert!(
            matches!(pending.back(),Some(live::UiMsg::Scroll {request,..}) if *request==ScrollRequest::lines(1)),
            "a post-key wheel step starts behind live, not at an older Top anchor"
        );
        pending.clear();
        queue_pending(&mut pending, scroll(ScrollRequest::top())).unwrap();
        assert_eq!(
            queue_pending(&mut pending, input(vec![0; PENDING_INPUT_BYTES + 1])),
            Err(AdmissionRefused::Input)
        );
        assert!(
            matches!(pending.front(),Some(live::UiMsg::Scroll {request,..}) if *request==ScrollRequest::top()),
            "a refused key must not alter the pending viewport intent"
        );
    }

    #[test]
    fn scroll_shortcuts_stay_local_for_repeats_and_kitty_releases() {
        for (key, physical, motion) in [
            (NamedKey::PageUp, KeyCode::PageUp, ScrollKey::PageUp),
            (NamedKey::PageDown, KeyCode::PageDown, ScrollKey::PageDown),
            (NamedKey::Home, KeyCode::Home, ScrollKey::Top),
            (NamedKey::End, KeyCode::End, ScrollKey::Live),
        ] {
            let mut state = keyboard::State::default();
            let mut input = keyboard::Input {
                logical_key: Key::Named(key),
                unmodified_key: Key::Named(key),
                physical_key: PhysicalKey::Code(physical),
                text: None,
                text_all_modifiers: None,
                location: winit::keyboard::KeyLocation::Standard,
                state: ElementState::Pressed,
                repeat: false,
                modifiers: keyboard::Modifiers {
                    state: ModifiersState::SHIFT,
                    locks: Default::default(),
                },
            };
            assert_eq!(
                route_input(&mut state, &input, 31 << 18, None),
                RoutedKey::Scroll(motion)
            );
            input.repeat = true;
            assert_eq!(
                route_input(&mut state, &input, 31 << 18, None),
                RoutedKey::Scroll(motion)
            );
            input.state = ElementState::Released;
            input.modifiers.state = ModifiersState::empty();
            assert_eq!(
                route_input(&mut state, &input, 31 << 18, None),
                RoutedKey::Local
            );
        }
    }

    #[test]
    fn ten_minutes_of_scroll_pressure_keep_one_composed_intent_per_pane() {
        let seat = SeatId::from_u128(1);
        let mut pending = VecDeque::new();
        let scroll = |generation, request| live::UiMsg::Scroll {
            seat,
            generation,
            request,
        };
        queue_pending(&mut pending, scroll(1, ScrollRequest::top())).unwrap();
        for _ in 0..60_000 {
            queue_pending(&mut pending, scroll(1, ScrollRequest::lines(-1))).unwrap();
            assert_eq!(pending.len(), 1);
        }
        assert!(
            matches!(pending.front(), Some(live::UiMsg::Scroll {request, ..})
            if request.anchor == bboxy450_term::scrollback::Anchor::Top && request.lines == -60_000)
        );
        queue_pending(&mut pending, scroll(2, ScrollRequest::lines(3))).unwrap();
        assert!(
            matches!(pending.front(), Some(live::UiMsg::Scroll {generation: 2,request, ..})
            if *request == ScrollRequest::lines(3))
        );
        queue_pending(&mut pending, scroll(2, ScrollRequest::live())).unwrap();
        queue_pending(&mut pending, scroll(2, ScrollRequest::lines(1))).unwrap();
        assert!(
            matches!(pending.front(), Some(live::UiMsg::Scroll {request, ..})
            if request.anchor == bboxy450_term::scrollback::Anchor::Live && request.lines == 1)
        );
        queue_pending(&mut pending, live::UiMsg::Watched(vec![])).unwrap();
        assert_eq!(pending.len(), 1);
        assert!(matches!(pending.front(), Some(live::UiMsg::Watched(_))));
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
    fn graph_navigation_is_local_and_global_shortcuts_still_reach_the_app() {
        let graph = test_key(
            &Key::Character("g".into()),
            PhysicalKey::Code(KeyCode::KeyG),
            Some("g"),
            ModifiersState::SUPER,
            false,
        );
        assert_eq!(route_key(&graph, 0, None), RoutedKey::OpenGraph);
        assert_eq!(
            route_composer(&graph, 0, ComposeField::Body),
            RoutedKey::OpenGraph
        );
        let down = test_key(
            &Key::Named(NamedKey::ArrowDown),
            PhysicalKey::Code(KeyCode::ArrowDown),
            None,
            ModifiersState::empty(),
            true,
        );
        assert_eq!(
            route_local_page(&down, 0),
            RoutedKey::LocalPage(PageCommand::Down)
        );
        let escape = test_key(
            &Key::Named(NamedKey::Escape),
            PhysicalKey::Code(KeyCode::Escape),
            None,
            ModifiersState::empty(),
            false,
        );
        assert_eq!(
            route_local_page(&escape, 0),
            RoutedKey::LocalPage(PageCommand::Close)
        );
        let launcher = test_key(
            &Key::Character("t".into()),
            PhysicalKey::Code(KeyCode::KeyT),
            Some("t"),
            ModifiersState::SUPER,
            false,
        );
        assert_eq!(route_local_page(&launcher, 0), RoutedKey::OpenLauncher);
        let navigate = test_key(
            &Key::Named(NamedKey::ArrowLeft),
            PhysicalKey::Code(KeyCode::ArrowLeft),
            None,
            ModifiersState::SUPER | ModifiersState::ALT,
            false,
        );
        assert_eq!(
            route_local_page(&navigate, 0),
            RoutedKey::Pane(Navigation::Direction(Direction::Left))
        );
    }
    #[test]
    fn a_graph_press_never_reaches_the_pty_but_an_earlier_pty_press_gets_its_release() {
        let mut state = keyboard::State::default();
        let mut letter = test_key(
            &Key::Character("a".into()),
            PhysicalKey::Code(KeyCode::KeyA),
            Some("a"),
            ModifiersState::empty(),
            false,
        );
        assert!(matches!(
            route_input(&mut state, &letter, 3 << 18, None),
            RoutedKey::Terminal(_)
        ));
        letter.state = ElementState::Released;
        let route = route_local_page(&letter, 3 << 18);
        assert_eq!(
            finish_route(&mut state, &letter, route),
            RoutedKey::Terminal(b"\x1b[97;1:3u".to_vec())
        );
        letter.state = ElementState::Pressed;
        let route = route_local_page(&letter, 3 << 18);
        assert_eq!(finish_route(&mut state, &letter, route), RoutedKey::Local);
        letter.state = ElementState::Released;
        let route = route_local_page(&letter, 3 << 18);
        assert_eq!(finish_route(&mut state, &letter, route), RoutedKey::Local);
        letter.state = ElementState::Pressed;
        assert!(matches!(
            route_input(&mut state, &letter, 3 << 18, None),
            RoutedKey::Terminal(_)
        ));
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
    fn cold_notification_waits_for_its_exact_seat_and_never_selects_a_substitute() {
        let seat = SeatId::from_u128(3);
        let now = Instant::now();
        let mut model = bboxy450_term::model::Model::default();
        assert_eq!(
            notification_target(seat, now, now, None, false),
            NotificationTarget::Pending
        );
        model.metadata_initialized = true;
        assert_eq!(
            notification_target(seat, now, now, Some(&model), true),
            NotificationTarget::Pending
        );
        model.ui = fixture();
        let stage = model.ui.window.stage.clone();
        assert_eq!(
            notification_target(seat, now, now, Some(&model), false),
            NotificationTarget::Pending
        );
        assert_eq!(
            notification_target(seat, now, now, Some(&model), true),
            NotificationTarget::Ready
        );
        assert_eq!(
            model.ui.window.stage, stage,
            "readiness and native responses only emit an explicit focus intent"
        );
        assert!(matches!(
            notification_target(seat, now, now + Duration::from_secs(30), Some(&model), true),
            NotificationTarget::Refused(_)
        ));
        model.ui.seats.get_mut(&seat).unwrap().lifecycle = bbt_ui_model::Lifecycle::ClosedRetained;
        assert_eq!(
            notification_target(seat, now, now, Some(&model), true),
            NotificationTarget::Refused("seat is closed")
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

    #[test]
    fn claude_presets_reach_spawn_and_only_claude_gets_presets() {
        let items = launcher_items(None);
        let claude = items.iter().find(|item| item.id == "claude").unwrap();
        assert_eq!(claude.role.as_deref(), Some("oracle"));
        assert!(claude.enter_picks);
        for (preset, (role, _, _)) in claude.children.as_ref().unwrap().iter().zip(ROLE_PRESETS) {
            assert_eq!(preset.id, format!("claude:as:{role}"));
            assert_eq!(preset.kind, "Role");
            assert_eq!(preset.role.as_deref(), Some(role));
            let live::UiMsg::Spawn { role: selected, .. } =
                spawn_pick(&preset.id, Default::default(), None, None).unwrap()
            else {
                panic!("preset must admit a spawn")
            };
            assert_eq!(selected.as_deref(), Some(role));
            for placement in preset.children.as_ref().unwrap() {
                assert_eq!(placement.role.as_deref(), Some(role));
                let live::UiMsg::Spawn { role: selected, .. } =
                    spawn_pick(&placement.id, Default::default(), None, None).unwrap()
                else {
                    panic!("placement must admit a spawn")
                };
                assert_eq!(selected.as_deref(), (role != "oracle").then_some(role));
            }
        }
        for id in [
            "codex:as:oracle",
            "claude:as:shell",
            "claude:as:director",
            "ssh:rig:claude:as:architect",
        ] {
            assert!(spawn_pick(id, Default::default(), None, None).is_none());
        }
        for (id, role) in [("codex", "executor"), ("zsh", "shell")] {
            let item = items.iter().find(|item| item.id == id).unwrap();
            assert!(
                item.children
                    .as_ref()
                    .unwrap()
                    .iter()
                    .all(|child| child.kind == "Placement" && child.role.as_deref() == Some(role))
            );
        }
    }

    #[test]
    fn existing_oracle_note_uses_the_displayed_swarm_and_never_blocks_the_default() {
        use bbt_ui_model::{Role, RoleClass, Seat, SwarmId};
        let mut model = bboxy450_term::model::Model::default();
        let mut seat = Seat::new(
            SeatId::from_u128(11),
            SwarmId::from_u128(1),
            [0; 32],
            "Existing Oracle",
            "claude",
        );
        seat.role = Some(Role {
            name: "oracle".into(),
            class: RoleClass::Oracle,
        });
        model.ui.seats.insert(seat.id, seat);
        let mut stage = Stage {
            swarm: Some(SwarmId::from_u128(1)),
            ..Default::default()
        };
        let items = launcher_items_for(Some(&model), Some(&stage));
        let claude = items.iter().find(|item| item.id == "claude").unwrap();
        assert!(
            claude
                .sub
                .ends_with("this swarm already has an Oracle: Existing Oracle")
        );
        assert!(claude.enter_picks);
        assert_eq!(claude.role.as_deref(), Some("oracle"));
        assert!(spawn_pick("claude", Default::default(), Some(&model), Some(&stage)).is_some());
        let oracle = &claude.children.as_ref().unwrap()[0];
        assert_eq!(
            oracle.sub,
            "default · this swarm already has an Oracle: Existing Oracle"
        );
        stage.swarm = Some(SwarmId::from_u128(2));
        let items = launcher_items_for(Some(&model), Some(&stage));
        assert_eq!(
            items.iter().find(|item| item.id == "claude").unwrap().sub,
            "oracle by default"
        );
    }

    fn generic() -> live::UiMsg {
        live::UiMsg::Spawn {
            id: bbt_mux::Ulid::new(),
            swarm: None,
            placement: None,
            harness_kind: "codex".into(),
            name: Some("named seat".into()),
            role: None,
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
                matches!(default_spawn(kind, Modifiers::default(), None), Some(live::UiMsg::Spawn { harness_kind, name: None, location: bbt_proto::Where::Local, .. }) if harness_kind == kind)
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
            "oracle by default"
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
                matches!(default_spawn(&id, Modifiers::default(), Some(&model)), Some(live::UiMsg::Spawn { harness_kind, name: None, location: actual, .. }) if harness_kind == "codex" && actual == location)
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
            let claude = host_row
                .children
                .as_ref()
                .unwrap()
                .iter()
                .find(|row| row.id == format!("{host}:claude"))
                .unwrap();
            assert_eq!(claude.role.as_deref(), Some("oracle"));
            assert!(
                claude.children.as_ref().unwrap().iter().all(|row| {
                    row.kind == "Placement" && row.role.as_deref() == Some("oracle")
                })
            );
        }
        assert!(default_spawn("ssh:missing:codex", Modifiers::default(), Some(&model)).is_none());
    }
}

#[cfg(test)]
mod placement_tests {
    use super::*;
    use bbt_ui_model::{Axis, Seat, SplitNode, Swarm, SwarmId, Tab, TabId};

    fn model() -> bboxy450_term::model::Model {
        let swarm = SwarmId::from_u128(1);
        let mut a = Seat::new(SeatId::from_u128(2), swarm, [0; 32], "first", "zsh");
        a.process_generation = 7;
        let mut b = Seat::new(SeatId::from_u128(3), swarm, [1; 32], "second", "zsh");
        b.process_generation = 11;
        let tree = SplitNode::split(Axis::Row, 0.5, SplitNode::Leaf(a.id), SplitNode::Leaf(b.id));
        bboxy450_term::model::Model {
            ui: UiModel::new(
                vec![Swarm::new(
                    swarm,
                    "swarm",
                    vec![Tab::new(TabId::from_u128(4), "tab", tree)],
                )],
                vec![a, b],
            )
            .unwrap(),
            ..Default::default()
        }
    }

    #[test]
    fn spawn_pick_uses_ui_focus_and_only_known_catalogue_ids() {
        let model = model();
        let mut stage = model.ui.window.stage.clone();
        stage.focused = Some(SeatId::from_u128(3));
        let live::UiMsg::Spawn {
            id,
            swarm,
            placement,
            harness_kind,
            ..
        } = spawn_pick(
            "codex:bottom",
            Default::default(),
            Some(&model),
            Some(&stage),
        )
        .unwrap()
        else {
            panic!("spawn intent")
        };
        assert_ne!(id, bbt_mux::Ulid::nil());
        assert_eq!(swarm, stage.swarm);
        assert_eq!(harness_kind, "codex");
        assert_eq!(
            placement,
            Some(bbt_proto::seat_state::Placement::Split {
                tab: stage.tab.unwrap(),
                target: SeatId::from_u128(3),
                target_generation: 11,
                side: bbt_proto::seat_state::Side::Bottom,
            })
        );
        for unknown in ["graph", "settings", "something labelled zsh", "zsh:invalid"] {
            assert!(spawn_pick(unknown, Default::default(), Some(&model), Some(&stage)).is_none());
        }
    }

    #[test]
    fn new_membership_does_not_focus_without_a_matching_spawn_result() {
        let before = model();
        let mut focus = FocusState::default();
        focus.reconcile(None, &before.ui);
        let added = Seat::new(
            SeatId::from_u128(5),
            SwarmId::from_u128(1),
            [2; 32],
            "unrelated",
            "zsh",
        );
        let mut next = before.ui.clone();
        next.seats.insert(added.id, added);
        let next = bbt_ui_model::place(
            &next,
            SeatId::from_u128(5),
            bbt_ui_model::Placement::Split {
                target: SeatId::from_u128(2),
                side: bbt_ui_model::Side::Right,
            },
        )
        .unwrap();
        focus.reconcile(Some(&before.ui), &next);
        assert_eq!(
            focus.stage.as_ref().unwrap().focused,
            Some(SeatId::from_u128(2))
        );
        focus
            .apply(
                &next,
                FocusAction::Activate(SeatId::from_u128(5), FocusOrigin::Keyboard),
                &[],
                Instant::now(),
            )
            .unwrap();
        assert_eq!(
            focus.stage.as_ref().unwrap().focused,
            Some(SeatId::from_u128(5))
        );
    }
}

#[cfg(test)]
mod composer_host_tests {
    use super::*;
    #[test]
    fn composer_submit_is_focus_local_and_newline_tab_navigation_remain_local() {
        let enter = Key::Named(NamedKey::Enter);
        let input = test_key(
            &enter,
            PhysicalKey::Code(KeyCode::Enter),
            None,
            ModifiersState::SUPER,
            false,
        );
        assert_eq!(
            route_composer(&input, 0, ComposeField::Body),
            RoutedKey::Composer(ComposeCommand::Send)
        );
        assert!(!matches!(
            route_key(&input, 0, None),
            RoutedKey::Composer(_)
        ));
        let plain = test_key(
            &enter,
            PhysicalKey::Code(KeyCode::Enter),
            None,
            ModifiersState::empty(),
            false,
        );
        assert_eq!(
            route_composer(&plain, 0, ComposeField::Body),
            RoutedKey::Composer(ComposeCommand::Edit(EditCommand::Insert("\n".into())))
        );
        assert_eq!(
            route_key(&plain, 0, None),
            RoutedKey::Terminal(b"\r".to_vec())
        );
        let tab = test_key(
            &Key::Named(NamedKey::Tab),
            PhysicalKey::Code(KeyCode::Tab),
            None,
            ModifiersState::empty(),
            false,
        );
        assert_eq!(
            route_composer(&tab, 0, ComposeField::Body),
            RoutedKey::Composer(ComposeCommand::Focus(ComposeField::Address))
        );
        assert_eq!(
            route_key(&tab, 0, None),
            RoutedKey::Terminal(b"\t".to_vec())
        );
    }
    #[test]
    fn consumed_composer_press_never_leaks_a_kitty_release_after_focus_changes() {
        let mut state = keyboard::State::default();
        let mut input = test_key(
            &Key::Character("x".into()),
            PhysicalKey::Code(KeyCode::KeyX),
            Some("x"),
            ModifiersState::empty(),
            false,
        );
        let routed = route_composer(&input, 3 << 18, ComposeField::Body);
        assert!(matches!(
            finish_route(&mut state, &input, routed),
            RoutedKey::Composer(_)
        ));
        input.state = ElementState::Released;
        assert!(matches!(
            route_key(&input, 3 << 18, None),
            RoutedKey::Terminal(_)
        ));
        assert_eq!(
            route_input(&mut state, &input, 3 << 18, None),
            RoutedKey::Local
        );
    }
    #[test]
    fn operation_admission_is_bounded_without_consuming_terminal_input_capacity() {
        let mut pending = VecDeque::new();
        let operation = || live::UiMsg::Task {
            id: bbt_mux::Ulid::new(),
            request: bbt_proto::messaging::TaskRequest::List {
                unassigned: false,
                swarm_id: None,
                status: None,
                assignee: None,
                after: None,
                limit: Some(1),
            },
        };
        for _ in 0..PENDING_OPERATIONS {
            queue_pending(&mut pending, operation()).unwrap();
        }
        assert_eq!(
            queue_pending(&mut pending, operation()),
            Err(AdmissionRefused::Operation)
        );
        queue_pending(
            &mut pending,
            live::UiMsg::Input {
                seat: SeatId::from_u128(1),
                generation: 1,
                id: bbt_mux::Ulid::new(),
                bytes: b"x".to_vec(),
            },
        )
        .unwrap();
        assert_eq!(pending.len(), PENDING_OPERATIONS + 1);
    }
    #[test]
    fn settings_keyboard_routes_edits_and_balances_only_the_prior_pty_release() {
        let comma = test_key(
            &Key::Character(",".into()),
            PhysicalKey::Code(KeyCode::Comma),
            Some(","),
            ModifiersState::SUPER,
            false,
        );
        assert_eq!(route_key(&comma, 0, None), RoutedKey::OpenSettings);
        let mut repeat = comma.clone();
        repeat.repeat = true;
        assert_eq!(route_key(&repeat, 0, None), RoutedKey::Local);
        let letter = test_key(
            &Key::Character("x".into()),
            PhysicalKey::Code(KeyCode::KeyX),
            Some("x"),
            ModifiersState::empty(),
            false,
        );
        let mut state = keyboard::State::default();
        let routed = route_settings(&letter, 3 << 18, true);
        assert!(
            matches!(&routed,RoutedKey::SettingsKey(SettingsKey::Edit(EditCommand::Insert(s))) if s=="x")
        );
        finish_route(&mut state, &letter, routed);
        let mut release = letter.clone();
        release.state = ElementState::Released;
        let routed = route_settings(&release, 3 << 18, true);
        assert_eq!(finish_route(&mut state, &release, routed), RoutedKey::Local);
        let mut state = keyboard::State::default();
        let routed = route_settings(&release, 3 << 18, false);
        assert!(matches!(
            finish_route(&mut state, &release, routed),
            RoutedKey::Terminal(_)
        ));
        let mute = test_key(
            &Key::Character("m".into()),
            PhysicalKey::Code(KeyCode::KeyM),
            Some("m"),
            ModifiersState::SUPER,
            false,
        );
        assert_eq!(route_key(&mute, 0, None), RoutedKey::MuteFocused);
        let mut repeat = mute.clone();
        repeat.repeat = true;
        assert_eq!(route_key(&repeat, 0, None), RoutedKey::Local);
    }
    #[test]
    fn settings_mute_requires_a_visible_real_focused_seat_and_awaits_metadata() {
        let id = SeatId::from_u128(1);
        let swarm = bbt_ui_model::SwarmId::from_u128(2);
        let mut model = bboxy450_term::model::Model::default();
        model.ui.seats.insert(
            id,
            bbt_ui_model::Seat::new(id, swarm, [0; 32], "real", "codex"),
        );
        let stage = Stage {
            swarm: Some(swarm),
            tab: None,
            focused: Some(id),
        };
        let panes = vec![(id, Rect::new(0., 0., 100., 100.))];
        assert!(mute_command(Some(&model), Some(&stage), &panes, false).is_none());
        assert!(mute_command(Some(&model), Some(&stage), &[], true).is_none());
        assert!(
            matches!(mute_command(Some(&model),Some(&stage),&panes,true),Some(live::UiMsg::Messaging {request:bbt_proto::messaging::Request::Mute {seat,muted:true},..}) if seat.0==id.0.0)
        );
        assert!(!model.ui.seats[&id].muted);
        model.ui.seats.get_mut(&id).unwrap().muted = true;
        assert!(matches!(
            mute_command(Some(&model), Some(&stage), &panes, true),
            Some(live::UiMsg::Messaging {
                request: bbt_proto::messaging::Request::Mute { muted: false, .. },
                ..
            })
        ));
    }
}
