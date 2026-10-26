//! R2 app shell: event-driven native scene, memory-only UI drain and hitch meter.
mod animation_clock;
mod app_starting;
mod assets;
mod budget;
mod capture;
mod command_retry;
mod display_link;
mod e2e_checklist;
mod gpu;
mod headless;
mod live;
mod log;
mod scene;
mod spawn_gate;
mod stall;

use bboxy450_term::launcher::{
    CaretMove, Command as LauncherCommand, Controller as LauncherController,
    Outcome as LauncherOutcome,
};
use bboxy450_term::scrollback::Request as ScrollRequest;
use bboxy450_term::{documents, keyboard, messaging};
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
use display_link::{DisplayLink, Mode, WakeAction, wake_action};
use e2e_checklist::{LocalTab, PageCommand};
use std::collections::{BTreeMap, VecDeque};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering as AtomicOrdering},
};
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
    UiFor {
        id: bbt_mux::Ulid,
        request: bbt_proto::gui::UiRequest,
        prior_spawns: Vec<bbt_mux::Ulid>,
    },
    UiDeadline(bbt_mux::Ulid),
    StartingDeadline(bbt_mux::Ulid),
    Wake,
    CommandsReady(command_retry::Ready),
    Settings(Arc<preferences::Snapshot>),
    DemoFinished,
    BeginIdle,
    EndIdle(u64),
    EndAnimation,
    Quit,
    BackendStopped(Result<(), String>),
    BackendLost(String),
    BackendReady,
    BackendInfo {
        generation: u64,
        custody_backend: String,
    },
    SmokeDeadline,
    VerifyGridDeadline,
    VerifyScrollTop,
    VerifyClose,
    VerifyFailed(String),
    Capture,
    /// A capture asked through an operator door (`ctl capture`); answered when written.
    CaptureFor(live::CaptureAsk),
    /// Focus a seat, asked through an operator door (`ctl seat focus`).
    FocusFor(bboxy450_term::focus_ack::Ask),
    FocusDeadline(bbt_mux::Ulid),
    App(live::AppEvent),
    CloseFor {
        id: bbt_mux::Ulid,
        seat: SeatId,
        undo: bool,
    },
    /// No frame was presented for this capture in time (occluded, or the display asleep).
    CaptureDeadline(bbt_mux::Ulid),
    StallPoll,
    Notification(bboxy450_desktop_notifications::Response),
    NotificationReady,
    NotificationError(String),
    ErrorCount(u64),
}
const NONVISUAL_WAKE_INTERVAL: Duration = Duration::from_millis(8);

fn defer_wake(
    pending: Arc<AtomicBool>,
    runtime: &tokio::runtime::Handle,
    wake: impl FnOnce() + Send + 'static,
) {
    if pending.swap(true, AtomicOrdering::AcqRel) {
        return;
    }
    runtime.spawn(async move {
        tokio::time::sleep(NONVISUAL_WAKE_INTERVAL).await;
        pending.store(false, AtomicOrdering::Release);
        wake();
    });
}

struct Wake {
    proxy: UiProxy,
    deferred: Arc<AtomicBool>,
    runtime: tokio::runtime::Handle,
}
impl Wake {
    fn new(proxy: UiProxy, runtime: tokio::runtime::Handle) -> Self {
        Self {
            proxy,
            deferred: Arc::new(AtomicBool::new(false)),
            runtime,
        }
    }
}
impl bbt_rt::Waker for Wake {
    fn wake(&self) {
        let _ = self.proxy.send_event(UiEvent::Wake);
    }
    fn wake_deferred(&self) {
        let proxy = self.proxy.clone();
        defer_wake(self.deferred.clone(), &self.runtime, move || {
            let _ = proxy.send_event(UiEvent::Wake);
        });
    }
}

#[cfg(test)]
#[allow(
    clippy::disallowed_methods,
    reason = "paused-clock test drivers run outside the app UI thread"
)]
mod wake_post_tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    #[tokio::test(start_paused = true)]
    async fn a_visual_frame_services_raw_work_before_its_deadline_and_keeps_the_newest_model() {
        let posts = Arc::new(AtomicUsize::new(0));
        let posted = posts.clone();
        let waker = Wake::new(
            UiProxy::Headless(Arc::new(move |event| {
                assert!(matches!(event, UiEvent::Wake));
                posted.fetch_add(1, AtomicOrdering::SeqCst);
            })),
            tokio::runtime::Handle::current(),
        );
        let dirty = bbt_rt::DirtySet::new(6, Arc::new(waker));
        let (sender, receiver) = bbt_rt::ui_channel(2, dirty.deferred_source(0));
        let latest = Latest::new(dirty.paint_source(5));
        sender.try_send(UiMsg::Changed { source: 7 }).unwrap();
        latest.publish(7);
        assert_eq!(posts.load(AtomicOrdering::SeqCst), 1);
        assert_eq!(dirty.drain(), vec![0, 5]);
        assert!(matches!(
            receiver.try_recv(),
            Some(UiMsg::Changed { source: 7 })
        ));
        receiver.complete_drain();
        assert_eq!(latest.take(), Some(7));
        latest.publish(8);
        assert_eq!(posts.load(AtomicOrdering::SeqCst), 1);
        latest.complete_drain();
        assert_eq!(posts.load(AtomicOrdering::SeqCst), 2);
        assert_eq!(dirty.drain(), vec![5]);
        assert_eq!(latest.take(), Some(8));
        latest.complete_drain();
        tokio::task::yield_now().await;
        tokio::time::advance(NONVISUAL_WAKE_INTERVAL).await;
        tokio::task::yield_now().await;
        assert_eq!(posts.load(AtomicOrdering::SeqCst), 3);
        assert!(
            dirty.drain().is_empty(),
            "the old deadline has no work left"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn sustained_output_is_deadline_bounded_without_ticks_or_paint() {
        let posts = Arc::new(AtomicUsize::new(0));
        let posted = posts.clone();
        let waker = Wake::new(
            UiProxy::Headless(Arc::new(move |event| {
                assert!(matches!(event, UiEvent::Wake));
                posted.fetch_add(1, AtomicOrdering::SeqCst);
            })),
            tokio::runtime::Handle::current(),
        );
        let dirty = bbt_rt::DirtySet::new(2, Arc::new(waker));
        let (sender, receiver) = bbt_rt::ui_channel(128, dirty.deferred_source(0));
        let mut seen = vec![false; 10_000];
        let mut serviced = 0;
        let mut received = 0;
        for millisecond in 0..=1008 {
            if millisecond < 1000 {
                for offset in 0..10 {
                    sender
                        .try_send(UiMsg::Changed {
                            source: millisecond * 10 + offset,
                        })
                        .unwrap();
                }
            }
            tokio::task::yield_now().await;
            tokio::time::advance(Duration::from_millis(1)).await;
            tokio::task::yield_now().await;
            let posted = posts.load(AtomicOrdering::SeqCst);
            if posted != serviced {
                assert!(matches!(
                    wake_action(&dirty.drain(), false),
                    WakeAction::Drain
                ));
                while let Some(UiMsg::Changed { source }) = receiver.try_recv() {
                    assert!(!std::mem::replace(&mut seen[source], true));
                    received += 1;
                }
                receiver.complete_drain();
                serviced = posted;
            }
        }
        assert_eq!(received, 10_000);
        assert!(seen.into_iter().all(|value| value));
        assert!(
            posts.load(AtomicOrdering::SeqCst)
                <= 1000 / NONVISUAL_WAKE_INTERVAL.as_millis() as usize + 3
        );
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

/// The logical window size that holds `cols`x`rows` terminal cells (`ctl app resize`): the
/// cell a laid-out pane measures, else the default 8x15, clamped to a usable window.
fn cell_grid_size(
    model: &bboxy450_term::model::Model,
    cols: u16,
    rows: u16,
    scale: f32,
) -> [f32; 2] {
    let cell = model
        .frames
        .iter()
        .find_map(|(id, frame)| {
            let screen = model.screens.get(id)?;
            Some([
                frame.width as f32 / screen.cols.max(1) as f32 / scale,
                frame.height as f32 / screen.rows.max(1) as f32 / scale,
            ])
        })
        .unwrap_or([8., 15.]);
    [
        (f32::from(cols) * cell[0]).clamp(320., 4096.),
        (f32::from(rows) * cell[1]).clamp(200., 2304.),
    ]
}
#[allow(
    clippy::too_many_arguments,
    reason = "one readiness DTO shared by both GUI owners"
)]
fn app_status(
    mode: &str,
    unattended: bool,
    backend_ready: bool,
    gui_ready: bool,
    model: &bboxy450_term::model::Model,
    generation: u64,
    frame_seq: u64,
    windows: usize,
) -> bbt_proto::gui::AppStatus {
    bbt_proto::gui::AppStatus {
        running: true,
        mode: mode.into(),
        unattended,
        backend_ready,
        gui_ready,
        events_ready: matches!(
            model.events.state(),
            bboxy450_term::model::EventLaneState::Active
        ),
        // The IO owner stamps the actual store backend into every relay.
        custody_backend: "unknown".into(),
        generation,
        frame_seq,
        windows,
        close: bboxy450_term::close::status(model, Instant::now()),
        notifications: serde_json::Value::Null,
        history: serde_json::Value::Null,
    }
}

/// Recording can never select the operator's native notification account.
fn notification_recording(args: &[String]) -> Result<bool, String> {
    let mut selected = false;
    for (index, arg) in args.iter().enumerate() {
        let mode = if arg == "--notifications" {
            Some(
                args.get(index + 1)
                    .map(String::as_str)
                    .ok_or("--notifications needs record")?,
            )
        } else {
            arg.strip_prefix("--notifications=")
        };
        let Some(mode) = mode else { continue };
        if selected || mode != "record" {
            return Err("--notifications supports one explicit record selector".into());
        }
        selected = true;
    }
    if selected
        && (!cfg!(debug_assertions)
            || bboxy450_term::cockpit::Config::test_account_from_args(args)
                .map_err(|e| e.to_string())?
                .is_none())
    {
        return Err("notification recording requires a debug build and --test-account DIR".into());
    }
    Ok(selected)
}

/// Both GUI owners derive close requests from the same retained scene.
fn close_request(
    scene: &scene::Scene,
    model: &bboxy450_term::model::Model,
    seat: SeatId,
    id: bbt_mux::Ulid,
    undo: bool,
    now: Instant,
) -> Result<live::UiMsg, String> {
    if undo {
        model
            .closes
            .undo_in(&model.ui, seat, now)
            .map(|_| live::UiMsg::UndoClose { seat })
    } else {
        scene
            .close_intent(model, seat, id, now)
            .map(live::UiMsg::Close)
            .map_err(|reason| reason.to_string())
    }
}

fn now_ms() -> u64 {
    unix_ms(Some(std::time::SystemTime::now())).unwrap_or(0)
}

/// The startup pipe carries only public readiness; all logs stay on stderr.
fn write_app_ready(mut status: bbt_proto::gui::AppStatus) {
    // The startup pipe is a small readiness line. Retained UI is requested
    // separately through `app status`, under the admitted GUI frame bound.
    status.close = serde_json::Value::Null;
    bbt_rt::handle().spawn(async move {
        use tokio::io::AsyncWriteExt;
        if let Ok(mut bytes) = serde_json::to_vec(&status) {
            bytes.push(b'\n');
            let _ = tokio::io::stdout().write_all(&bytes).await;
        }
    });
}

#[allow(
    clippy::too_many_arguments,
    clippy::type_complexity,
    reason = "identical visible-pane admission for window and offscreen GUI"
)]
fn pane_controls(
    model: &bboxy450_term::model::Model,
    bounds: &[(SeatId, Rect)],
    previous: &[(SeatId, View)],
    old_sizes: &BTreeMap<SeatId, (u64, u16, u16)>,
    focused: Option<SeatId>,
    scale: f32,
    grid: Option<(u16, u16)>,
) -> (Vec<(SeatId, View)>, Vec<(SeatId, (u64, u16, u16))>) {
    let mut watched: Vec<_> = bounds
        .iter()
        .filter_map(|(id, _)| {
            let generation = model.ui.seats.get(id)?.process_generation;
            let view = previous
                .iter()
                .find(|(old, view)| old == id && view.generation == generation)
                .map(|(_, v)| *v)
                .unwrap_or(View {
                    seat: bbt_mux::Ulid::from(id.0.0),
                    generation,
                    view_generation: bbt_mux::Ulid::new(),
                });
            Some((*id, view))
        })
        .collect();
    watched.sort_by_key(|(id, _)| Some(*id) != focused);
    let mut sizes = Vec::new();
    for (id, rect) in bounds {
        let Some(metadata) = model.ui.seats.get(id) else {
            continue;
        };
        let cell =
            model
                .frames
                .get(id)
                .zip(model.screens.get(id))
                .map_or([8., 15.], |(frame, screen)| {
                    [
                        frame.width as f32 / screen.cols.max(1) as f32 / scale,
                        frame.height as f32 / screen.rows.max(1) as f32 / scale,
                    ]
                });
        let (cols, rows) = grid.unwrap_or((
            (rect.w / cell[0]).floor().clamp(1., 512.) as u16,
            (rect.h / cell[1]).floor().clamp(1., 256.) as u16,
        ));
        let size = (metadata.process_generation, cols, rows);
        if old_sizes.get(id) != Some(&size) {
            sizes.push((*id, size));
        }
    }
    (watched, sizes)
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
    fn restore(&mut self, stage: Stage, source: &UiModel) {
        self.stage = Some(stage);
        self.reconcile(None, source);
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

fn rect_array(rect: Rect) -> [f32; 4] {
    [rect.x, rect.y, rect.w, rect.h]
}

#[derive(Clone, PartialEq, Eq)]
struct UiVisible {
    launcher: bool,
    page: Option<String>,
    document: Option<String>,
    focused: Option<bbt_mux::Ulid>,
    tab: Option<bbt_mux::Ulid>,
}
impl UiVisible {
    fn from_layout(layout: &bbt_proto::gui::UiLayout) -> Self {
        Self {
            launcher: layout.launcher_open,
            page: layout.page.clone(),
            document: layout.document.clone(),
            focused: layout.focused,
            tab: layout.tab,
        }
    }
}
#[derive(Debug)]
enum UiEffect {
    Compose(bbt_proto::gui::UiComposed),
    Spawn(bbt_mux::Ulid),
    Operation(bbt_mux::Ulid),
    Plugin {
        op: bbt_mux::Ulid,
        before: u64,
    },
    Mute(SeatId, bool),
    Scroll(bbt_mux::Ulid, SeatId),
    Lifecycle(bboxy450_term::close_frame::Target),
    Document,
    Graph,
    Preferences {
        saved: bool,
    },
    Clock {
        state: bbt_proto::gui::UiClockState,
        landed: Option<(SeatId, Instant)>,
    },
    /// The window has solved a layout at this logical size (`ctl app resize`).
    Size([f32; 2]),
    Refused(String),
    StartingDismissed(bbt_mux::Ulid),
}
impl UiEffect {
    fn from_message(
        message: &live::UiMsg,
        model: Option<&bboxy450_term::model::Model>,
    ) -> Option<Self> {
        match message {
            live::UiMsg::Spawn { id, .. } | live::UiMsg::SpawnSpec { id, .. } => {
                Some(Self::Spawn(*id))
            }
            live::UiMsg::Messaging {
                request: bbt_proto::messaging::Request::Mute { seat, muted },
                ..
            } => Some(Self::Mute(SeatId::from_u128(seat.0), *muted)),
            live::UiMsg::Messaging { id, .. }
            | live::UiMsg::Task { id, .. }
            | live::UiMsg::SetAttention { id, .. }
            | live::UiMsg::GetAttention { id, .. } => Some(Self::Operation(*id)),
            live::UiMsg::PluginControl { id, .. } | live::UiMsg::KnowledgeControl { id, .. } => {
                Some(Self::Plugin {
                    op: *id,
                    before: model
                        .and_then(|m| m.operation_results.rows().find(|row| row.op == *id))
                        .map_or(0, |row| row.serial),
                })
            }
            live::UiMsg::Scroll { seat, request, .. } => {
                request.op.map(|id| Self::Scroll(id, *seat))
            }
            live::UiMsg::Close(intent) => Some(Self::Lifecycle(
                bboxy450_term::close_frame::Target::close(intent),
            )),
            live::UiMsg::UndoClose { seat } => Some(
                model
                    .ok_or_else(|| "ui_not_ready: no live model".to_owned())
                    .and_then(|model| {
                        bboxy450_term::close_frame::Target::undo(model, *seat, Instant::now())
                    })
                    .map(Self::Lifecycle)
                    .unwrap_or_else(Self::Refused),
            ),
            _ => None,
        }
    }
}
struct UiWait {
    id: bbt_mux::Ulid,
    after: Option<u64>,
    visible: Option<UiVisible>,
    effects: Vec<UiEffect>,
    spawn: Option<UiSpawnCapture>,
    timer: tokio::task::AbortHandle,
}
struct UiSpawnCapture {
    op: bbt_mux::Ulid,
    swarm: bbt_proto::SwarmId,
    placement: bbt_proto::seat_state::Placement,
    layout_revision: u64,
    seat: Option<SeatId>,
    seat_row: Option<bbt_proto::GuiSeat>,
}
impl Drop for UiWait {
    fn drop(&mut self) {
        self.timer.abort();
    }
}
fn ui_layout_node(
    layout: &bbt_ui::components::Layout,
    remaining: &mut usize,
    remaining_text: &mut usize,
) -> Result<bbt_proto::gui::UiLayoutNode, String> {
    ui_layout_node_in_clip(layout, None, remaining, remaining_text)?
        .ok_or_else(|| "ui_layout_unavailable: empty root".into())
}
fn ui_layout_node_in_clip(
    layout: &bbt_ui::components::Layout,
    clip: Option<Rect>,
    remaining: &mut usize,
    remaining_text: &mut usize,
) -> Result<Option<bbt_proto::gui::UiLayoutNode>, String> {
    use bbt_ui::components::LayoutKind as K;
    if clip.is_some_and(|clip| clip.w <= 0. || clip.h <= 0.) {
        return Ok(None);
    }
    // The retained scroll content can be much taller than its viewport. Match
    // the renderer's ScrollView clip while keeping structural ancestors and
    // the original solved geometry of every visible child (ICD ctl-app #347).
    let child_clip = if matches!(&layout.kind, K::ScrollView { .. }) {
        Some(clip.map_or(layout.rect, |clip| {
            let x = clip.x.max(layout.rect.x);
            let y = clip.y.max(layout.rect.y);
            Rect::new(
                x,
                y,
                (clip.right().min(layout.rect.right()) - x).max(0.),
                (clip.bottom().min(layout.rect.bottom()) - y).max(0.),
            )
        }))
    } else {
        clip
    };
    let children: Vec<_> = layout
        .children
        .iter()
        .filter_map(|child| {
            ui_layout_node_in_clip(child, child_clip, remaining, remaining_text).transpose()
        })
        .collect::<Result<_, _>>()?;
    if children.is_empty() && clip.is_some_and(|clip| !layout.rect.intersects(clip)) {
        return Ok(None);
    }
    *remaining = remaining
        .checked_sub(1)
        .ok_or("ui_layout_oversize: more than 4096 nodes")?;
    let kind = match &layout.kind {
        K::Named(name) => format!("named:{name}"),
        K::Filter { .. } => "filter".into(),
        K::Atom(_) => "atom".into(),
        K::Box { .. } => "box".into(),
        K::Text { .. } => "text".into(),
        K::Stack => "stack".into(),
        K::ScrollView { .. } => "scroll_view".into(),
        K::Spacer => "spacer".into(),
        K::Icon { .. } => "icon".into(),
        K::Image { .. } => "image".into(),
        K::Path { .. } => "path".into(),
        K::Rects { .. } => "rects".into(),
    };
    let text = if let K::Text { runs, .. } = &layout.kind {
        let length = runs
            .iter()
            .try_fold(0usize, |length, run| length.checked_add(run.text.len()));
        length
            .filter(|length| *length <= *remaining_text)
            .map(|length| {
                *remaining_text -= length;
                runs.iter().map(|run| run.text.as_str()).collect()
            })
    } else {
        None
    };
    Ok(Some(bbt_proto::gui::UiLayoutNode {
        kind,
        rect: rect_array(layout.rect),
        children,
        text,
    }))
}
fn chrome_input(name: &str) -> Result<keyboard::Input<'_>, String> {
    if bbt_proto::gui::ui_text_key(name) {
        let logical_key = Key::Character(name.into());
        return Ok(keyboard::Input {
            unmodified_key: logical_key.clone(),
            logical_key,
            physical_key: PhysicalKey::Unidentified(winit::keyboard::NativeKeyCode::Unidentified),
            text: Some(name),
            text_all_modifiers: Some(name),
            location: winit::keyboard::KeyLocation::Standard,
            state: ElementState::Pressed,
            repeat: false,
            modifiers: keyboard::Modifiers {
                state: ModifiersState::empty(),
                locks: Default::default(),
            },
        });
    }
    let mut parts = name.split('-').collect::<Vec<_>>();
    let name = parts.pop().ok_or("invalid_ui_key: missing key")?;
    let mut state = ModifiersState::empty();
    for modifier in parts {
        state.insert(match modifier {
            "cmd" => ModifiersState::SUPER,
            "alt" => ModifiersState::ALT,
            "shift" => ModifiersState::SHIFT,
            _ => return Err("invalid_ui_key: unsupported modifier".into()),
        });
    }
    let (logical_key, code) = match name {
        "t" => (Key::Character("t".into()), KeyCode::KeyT),
        "g" => (Key::Character("g".into()), KeyCode::KeyG),
        "m" => (Key::Character("m".into()), KeyCode::KeyM),
        "w" => (Key::Character("w".into()), KeyCode::KeyW),
        "z" => (Key::Character("z".into()), KeyCode::KeyZ),
        "r" => (Key::Character("r".into()), KeyCode::KeyR),
        "pageup" => (Key::Named(NamedKey::PageUp), KeyCode::PageUp),
        "pagedown" => (Key::Named(NamedKey::PageDown), KeyCode::PageDown),
        "home" => (Key::Named(NamedKey::Home), KeyCode::Home),
        "end" => (Key::Named(NamedKey::End), KeyCode::End),
        "comma" => (Key::Character(",".into()), KeyCode::Comma),
        "esc" => (Key::Named(NamedKey::Escape), KeyCode::Escape),
        "enter" => (Key::Named(NamedKey::Enter), KeyCode::Enter),
        "tab" => (Key::Named(NamedKey::Tab), KeyCode::Tab),
        "backspace" => (Key::Named(NamedKey::Backspace), KeyCode::Backspace),
        "up" => (Key::Named(NamedKey::ArrowUp), KeyCode::ArrowUp),
        "down" => (Key::Named(NamedKey::ArrowDown), KeyCode::ArrowDown),
        "left" => (Key::Named(NamedKey::ArrowLeft), KeyCode::ArrowLeft),
        "right" => (Key::Named(NamedKey::ArrowRight), KeyCode::ArrowRight),
        _ => return Err("invalid_ui_key: unsupported chrome key".into()),
    };
    Ok(keyboard::Input {
        unmodified_key: logical_key.clone(),
        logical_key,
        physical_key: PhysicalKey::Code(code),
        text: None,
        text_all_modifiers: None,
        location: winit::keyboard::KeyLocation::Standard,
        state: ElementState::Pressed,
        repeat: false,
        modifiers: keyboard::Modifiers {
            state,
            locks: Default::default(),
        },
    })
}
#[derive(Debug, PartialEq, Eq)]
enum RoutedKey {
    Quit,
    OpenLauncher,
    OpenGraph,
    OpenSettings,
    MuteFocused,
    UndoClose,
    DismissStarting(bbt_mux::Ulid),
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
    let shortcut = keyboard::shortcut_key(input);
    let text = input.text;
    let mods = input.modifiers.state;
    let repeat = input.repeat;
    let command = mods.super_key();
    if command && shortcut == Some('q') {
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
        && shortcut == Some('t')
    {
        return if repeat {
            RoutedKey::Local
        } else {
            RoutedKey::OpenLauncher
        };
    }
    if command && mods.alt_key() && shortcut == Some('f') {
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
        && shortcut == Some('g')
    {
        return if repeat {
            RoutedKey::Local
        } else {
            RoutedKey::OpenGraph
        };
    }
    if command && !mods.alt_key() && !mods.shift_key() && !mods.control_key() {
        if shortcut == Some(',') {
            return if repeat {
                RoutedKey::Local
            } else {
                RoutedKey::OpenSettings
            };
        }
        if query.is_none() && shortcut == Some('z') {
            return if repeat {
                RoutedKey::Local
            } else {
                RoutedKey::UndoClose
            };
        }
        if shortcut == Some('m') {
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
        Key::Character(_) if command && shortcut == Some('a') => {
            LauncherCommand::SelectAll
        }
        Key::Character(_)
            if (command || mods.control_key()) && shortcut == Some('r') && !repeat =>
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
        && keyboard::shortcut_key(input) == Some('w')
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
        && keyboard::shortcut_key(input) == Some('r')
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
fn route_document(input: &keyboard::Input<'_>, modes: u64) -> RoutedKey {
    let routed = route_local_page(input, modes);
    if matches!(
        routed,
        RoutedKey::Quit | RoutedKey::ToggleHud | RoutedKey::LocalPage(_)
    ) || input.state == ElementState::Released
    {
        routed
    } else {
        RoutedKey::Local
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
        Key::Character(_) if command && keyboard::shortcut_key(input) == Some('a') => {
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

#[derive(Clone)]
enum UiProxy {
    Native(EventLoopProxy<UiEvent>),
    Headless(Arc<dyn Fn(UiEvent) + Send + Sync>),
}
impl UiProxy {
    fn send_event(&self, event: UiEvent) -> Result<(), ()> {
        match self {
            Self::Native(proxy) => proxy.send_event(event).map_err(|_| ()),
            Self::Headless(send) => {
                send(event);
                Ok(())
            }
        }
    }
}
struct App {
    proxy: UiProxy,
    inbox: UiInbox,
    wake_counters: std::cell::RefCell<stall::WakeCounters>,
    logger: log::Logger,
    unread_errors: u64,
    prepared: Arc<Latest<Prepared>>,
    updates: Arc<Latest<usize>>,
    live_updates: Arc<Latest<Arc<bboxy450_term::model::Model>>>,
    live_state: Option<Arc<bboxy450_term::model::Model>>,
    scene_dirty: bool,
    live_commands: Option<live::CommandSender>,
    pending_commands: VecDeque<live::UiMsg>,
    command_retry: command_retry::Retry,
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
    notification_ui: Option<bboxy450_term::focus_ack::Ask>,
    #[cfg(debug_assertions)]
    notification_recorder: Option<bboxy450_term::notifications::recording::Recorder>,
    spawn_results_seen: BTreeMap<bbt_mux::Ulid, bboxy450_term::model::SpawnOutcome>,
    launcher: Option<LauncherController>,
    launcher_layout: Option<scene::LauncherLayout>,
    launcher_parent: Option<TextFocus>,
    preferences: preferences::State,
    preferences_io: Option<preferences::Io>,
    rim_preview: Option<f32>,
    mute: preferences::Mute,
    custody: preferences::CustodyControl,
    settings_attention: Option<(bbt_mux::Ulid, bbt_mux::Ulid, bool)>,
    settings_plugins: preferences::registry::Requests,
    settings_knowledge: preferences::knowledge::Requests,
    settings_attention_target: Option<(bbt_mux::Ulid, u64, bbt_proto::registry::Role)>,
    system_theme: Theme,
    local_tab: Option<LocalTab>,
    local_page_layout: Option<scene::LocalPageLayout>,
    document: documents::Host,
    document_return: Option<Stage>,
    document_layout: Option<scene::DocumentLayout>,
    composer: ComposeController,
    composer_client: messaging::Client,
    composer_scope: Option<bbt_proto::SwarmId>,
    task_records: messaging::Records,
    message_history: messaging::History,
    events_seen: Option<Arc<bboxy450_term::model::EventLane>>,
    task_regions: bbt_ui::modules::stream_dock::Regions,
    close_regions: Vec<(SeatId, Rect, bool)>,
    starting_close_regions: Vec<(bbt_mux::Ulid, Rect)>,
    starting: bboxy450_term::starting::Starting,
    spawn_gate: spawn_gate::Gate<live::UiMsg>,
    starting_timers: BTreeMap<bbt_mux::Ulid, tokio::task::AbortHandle>,
    submitted_frame: u64,
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
    app_generation: u64,
    app_custody: String,
    app_ready_sent: bool,
    render_context: Option<tokio::sync::oneshot::Sender<live::RenderContext>>,
    closing: bool,
    smoke_bootstrap: Option<tokio::sync::oneshot::Sender<()>>,
    smoke_wake_seen: bool,
    window: Option<Arc<Window>>,
    pacer: Option<DisplayLink>,
    pacing: Mode,
    clock: animation_clock::Clock,
    clock_frame: animation_clock::Frame,
    animation_landing: Option<(SeatId, Instant)>,
    test_account: bool,
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
    redraw_queued: std::cell::Cell<bool>,
    last_update: usize,
    error: Option<String>,
    capture: Option<std::path::PathBuf>,
    capture_pending: bool,
    /// Operator captures waiting for the next presented frame.
    ctl_captures: Vec<live::CaptureAsk>,
    ctl_focus: bboxy450_term::focus_ack::Pending,
    ui_observe: bool,
    frame_observations: bboxy450_term::frame_observation::Frames,
    ui_frame: Option<(u64, Result<bbt_proto::gui::UiLayout, String>)>,
    ui_wait: Option<UiWait>,
    ui_focus: Option<bbt_mux::Ulid>,
    ui_collect: Option<Vec<UiEffect>>,
    /// `ctl app resize` for the offscreen owner: the logical size its next frame takes.
    requested_size: Option<[f32; 2]>,
    window_restored: bool,
    window_first_frame: bool,
    window_last: Option<bbt_proto::gui::WindowState>,
    restored_size: Option<[f32; 2]>,
    /// The logical size the last solved layout used.
    solved_size: [f32; 2],
    /// When the last frame was presented; a capture of it names this time.
    last_presented: Option<std::time::SystemTime>,
    pending_resize: Option<LogicalSize<f64>>,
    animate: bool,
    cadence_test: bool,
    animation_start: Option<(Instant, u64)>,
    verify: Verify,
    stall: Option<stall::Native>,
}
impl App {
    /// The same controller owner is used by winit and the windowless driver.
    fn owner(
        proxy: UiProxy,
        inbox: UiInbox,
        logger: log::Logger,
        live_updates: Arc<Latest<Arc<bboxy450_term::model::Model>>>,
        args: &[String],
        verify: Verify,
    ) -> Self {
        let prepared = Arc::new(Latest::new(inbox.dirty.source(2)));
        let updates = Arc::new(Latest::new(inbox.dirty.paint_source(3)));
        let smoke = args.iter().any(|a| a == "--smoke");
        let fake = !args.iter().any(|a| a == "--headless")
            && (smoke || args.iter().any(|a| a == "--fake-source"));
        let overlay = args.iter().any(|a| a == "--meter");
        let cadence_test = args.iter().any(|a| a == "--cadence-test");
        let animate = cadence_test || args.iter().any(|a| a == "--animate");
        let pacing = Mode::Metal;
        let capture = args
            .windows(2)
            .find(|v| v[0] == "--capture")
            .map(|v| std::path::PathBuf::from(&v[1]));
        let clock = animation_clock::Clock::default();
        let clock_frame = clock.frame();
        Self {
            proxy,
            inbox,
            wake_counters: Default::default(),
            logger,
            prepared,
            updates,
            live_updates,
            live_state: None,
            scene_dirty: true,
            live_commands: None,
            pending_commands: VecDeque::new(),
            command_retry: Default::default(),
            admission_refusals: AdmissionRefusals::default(),
            watched: vec![],
            pane_sizes: BTreeMap::new(),
            pane_bounds: vec![],
            focus_bounds: vec![],
            scroll_wheel: None,
            focus: FocusState::default(),
            notifications: None,
            frontmost: false,
            pending_notification: None,
            notification_ui: None,
            #[cfg(debug_assertions)]
            notification_recorder: None,
            spawn_results_seen: BTreeMap::new(),
            launcher: args
                .iter()
                .any(|arg| arg == "--launcher")
                .then(|| new_launcher(None, 0)),
            launcher_layout: None,
            launcher_parent: None,
            preferences: preferences::State::default(),
            preferences_io: None,
            rim_preview: None,
            mute: preferences::Mute::default(),
            custody: preferences::CustodyControl::default(),
            settings_attention: None,
            settings_plugins: Default::default(),
            settings_knowledge: Default::default(),
            settings_attention_target: None,
            system_theme: Theme::Dark,
            local_tab: None,
            local_page_layout: None,
            document: {
                let mut host = documents::Host::default();
                if let Some(pair) = args.windows(2).find(|p| p[0] == "--document") {
                    host.show(pair[1].clone());
                }
                host
            },
            document_return: None,
            document_layout: None,
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
            close_regions: vec![],
            starting_close_regions: Vec::new(),
            starting: Default::default(),
            spawn_gate: Default::default(),
            starting_timers: BTreeMap::new(),
            submitted_frame: 0,
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
            app_generation: 0,
            app_custody: "unknown".into(),
            app_ready_sent: false,
            render_context: None,
            closing: false,
            smoke_bootstrap: None,
            smoke_wake_seen: false,
            window: None,
            pacer: None,
            pacing,
            clock,
            clock_frame,
            animation_landing: None,
            test_account: bboxy450_term::cockpit::Config::test_account_from_args(args)
                .is_ok_and(|account| account.is_some()),
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
            redraw_queued: std::cell::Cell::new(false),
            last_update: 0,
            error: None,
            unread_errors: 0,
            capture,
            capture_pending: true,
            ctl_captures: Vec::new(),
            ctl_focus: Default::default(),
            ui_observe: args.iter().any(|arg| arg == "--ctl"),
            frame_observations: Default::default(),
            ui_frame: None,
            ui_wait: None,
            ui_focus: None,
            ui_collect: None,
            requested_size: None,
            window_restored: false,
            window_first_frame: false,
            window_last: None,
            restored_size: None,
            solved_size: [0.; 2],
            last_presented: None,
            pending_resize: None,
            animate,
            cadence_test,
            animation_start: None,
            verify,
            stall: None,
        }
    }
    fn finish_log(&self, text: String) {
        match &self.proxy {
            UiProxy::Native(proxy) => self.logger.finish(text, proxy.clone()),
            UiProxy::Headless(send) => self.logger.finish_with_sink(text, send.clone()),
        }
    }
    fn open_document(&mut self, target: String) {
        if !self.document.open {
            self.document_return = self.focus.stage.clone();
        }
        self.document.show(target);
        self.document_layout = None;
        self.composer_dragging = false;
        self.query_dragging = false;
        self.sync_text_ime();
        self.drive_document();
        self.request_redraw();
    }
    fn close_document(&mut self) {
        self.document.close();
        if let Some(stage) = self.document_return.take()
            && let Some(model) = &self.live_state
        {
            self.focus.restore(stage, &model.ui);
            self.scene_dirty = true;
        }
        self.document_layout = None;
        self.sync_text_ime();
        self.request_redraw();
    }
    fn drive_document(&mut self) {
        if self.document.open
            && (self.live_commands.is_none() || (self.backend_lost && !self.document.settled()))
        {
            if self.document.unavailable(
                "document_backend_unavailable: open the record again after the connection recovers",
            ) {
                self.document_layout = None;
                self.request_redraw();
            }
            return;
        }
        if self.live_commands.is_some()
            && let Some((id, read)) = self.document.next_read()
        {
            self.queue(live::UiMsg::DocumentRead { id, read });
        }
    }
    fn document_command(&mut self, command: PageCommand) {
        if matches!(command, PageCommand::Close) {
            self.close_document();
            return;
        }
        let Some(layout) = &self.document_layout else {
            return;
        };
        let height = layout.regions.viewport.map_or(200., |r| r.h * 0.9);
        let y = match command {
            PageCommand::Up => -28.,
            PageCommand::Down => 28.,
            PageCommand::PageUp => -height,
            PageCommand::PageDown => height,
            PageCommand::Top => -f32::MAX,
            PageCommand::Bottom => f32::MAX,
            PageCommand::Close => 0.,
        };
        if self.document.scroll.scroll([0., y], layout.regions.extent) {
            self.document_layout = None;
            self.request_redraw();
        }
    }
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
            commands.observe_knowledge(matches!(&self.local_tab, Some(LocalTab::Graph(_)))
                || matches!(&self.local_tab, Some(LocalTab::Settings {controller,..}) if controller.section == "Knowledge"));
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
    fn request_attention_read(&mut self, force: bool) {
        if self.live_commands.is_none() || self.backend_lost {
            self.settings_attention_target = None;
            return;
        }
        let target = self
            .focus
            .stage
            .as_ref()
            .and_then(|stage| stage.focused)
            .and_then(|focused| {
                self.live_state
                    .as_ref()?
                    .gui_seats
                    .iter()
                    .find(|seat| SeatId::from_u128(seat.record.id.0) == focused)
                    .map(|seat| {
                        (
                            seat.record.id,
                            seat.record.generation,
                            seat.record.role.clone(),
                        )
                    })
            });
        if !force && self.settings_attention_target == target {
            return;
        }
        self.settings_attention_target = target.clone();
        if let Some((seat, generation, _)) = target {
            let id = bbt_mux::Ulid::new();
            self.settings_attention = Some((id, seat, false));
            self.queue(live::UiMsg::GetAttention {
                id,
                seat,
                generation,
            });
        }
    }
    fn plugin_request(&mut self, request: bbt_proto::gui::PluginRequest) -> Result<(), String> {
        if self.backend_lost || self.live_commands.is_none() || self.closing {
            return Err("Plugin registry connection is unavailable".into());
        }
        let model = self
            .live_state
            .as_deref()
            .ok_or("Plugin registry snapshot is unavailable")?;
        if let Some(message) = self.settings_plugins.issue(model, request)? {
            self.queue(message);
        }
        Ok(())
    }
    fn knowledge_request(
        &mut self,
        request: bbt_proto::gui::KnowledgeRequest,
    ) -> Result<(), String> {
        if self.backend_lost || self.live_commands.is_none() || self.closing {
            return Err("Knowledge connection is unavailable".into());
        }
        let model = self
            .live_state
            .as_deref()
            .ok_or("Knowledge snapshot unavailable")?;
        if let Some(message) = self.settings_knowledge.issue(model, request)? {
            self.queue(message);
        }
        Ok(())
    }
    fn open_settings(&mut self, section: &str) {
        if let Err(reason) = self.knowledge_request(bbt_proto::gui::KnowledgeRequest::Status) {
            self.logger.write(reason);
        }
        if let Err(reason) = self.plugin_request(bbt_proto::gui::PluginRequest::List) {
            self.logger.write(reason);
        }
        if self.live_commands.is_some()
            && !self.backend_lost
            && let Some(message) = self.custody.request(false, Instant::now())
        {
            self.queue(message);
        }
        self.request_attention_read(true);
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
            if let Some(commands) = &self.live_commands {
                commands.observe_knowledge(section == "Knowledge");
            }
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
            controller: Box::new(settings_controller::Controller::new(section)),
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
        let mut view = Settings {
            section: section.into(),
            values: preferences::records(self.live_state.as_deref(), &self.preferences.preferences),
            defaults: preferences::records(
                self.live_state.as_deref(),
                &preferences::Preferences::default(),
            ),
            ..Default::default()
        };
        if let Some(model) = self.live_state.as_deref() {
            self.settings_plugins.decorate(&mut view.values, model);
            self.settings_knowledge.decorate(&mut view.values, model);
        }
        view.values.account.custody = Some(self.custody.view());
        view.values.messaging = preferences::messaging::view(
            self.live_state.as_deref(),
            self.focus.stage.as_ref().and_then(|stage| stage.focused),
        );
        if let Some((op, seat, writing)) = self.settings_attention
            && view.values.messaging.seat.id == seat.to_string()
        {
            let observation_error = view.values.messaging.error.take();
            view.values.messaging.error = self
                .live_state
                .as_ref()
                .and_then(|model| model.operation_results.rows().find(|row| row.op == op))
                .map_or_else(
                    || {
                        Some(
                            if writing {
                                "Updating messaging policy…"
                            } else {
                                "Reading messaging policy…"
                            }
                            .into(),
                        )
                    },
                    |row| match &row.reply {
                        bboxy450_term::messaging::Reply::Attention => observation_error,
                        bboxy450_term::messaging::Reply::Refused(reason)
                        | bboxy450_term::messaging::Reply::Unknown(reason) => Some(reason.clone()),
                        _ => Some("Unexpected messaging policy response".into()),
                    },
                );
        }
        if self.backend_lost || self.live_commands.is_none() {
            view.values.account.custody.as_mut().unwrap().enabled = false;
            view.values.plugin_controls = false;
            view.values.plugin_notice = Some("Plugin registry connection unavailable; retained observations cannot authorize a control.".into());
            for root in &mut view.values.knowledge.roots {
                root.answering = "read connection unavailable".into();
                root.refused = Some("Knowledge connection is unavailable; the retained view cannot confirm current reads. Provider switching is unavailable in this build.".into());
            }
        }
        view
    }
    fn settings_ime(&self) -> bool {
        matches!(&self.local_tab,Some(LocalTab::Settings {controller,..}) if controller.editing())
    }
    fn sync_text_ime(&self) {
        if let Some(window) = &self.window {
            window.set_ime_allowed(
                !self.document.open
                    && (self.launcher.is_some()
                        || self.composer.view.focus
                        || self.reject_focus
                        || self.settings_ime()),
            );
        }
    }
    fn settings_action(&mut self, action: settings_controller::Action) {
        let read_plugins = matches!(&action, settings_controller::Action::Section(_));
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
        if let Some(commands) = &self.live_commands {
            commands.observe_knowledge(
                controller.section == "Knowledge" || !controller.query.trim().is_empty(),
            );
        }
        let plugin = controller.take_plugin();
        let plugin_request = plugin.map(|intent| {
            self.live_state
                .as_deref()
                .ok_or_else(|| "Plugin registry snapshot unavailable".to_owned())
                .and_then(|model| preferences::registry::request(model, intent))
        });
        let knowledge = controller.take_knowledge().map(|intent| {
            self.live_state
                .as_deref()
                .ok_or_else(|| "Knowledge snapshot unavailable".to_owned())
                .and_then(|model| preferences::knowledge::request(model, intent))
        });
        let unlock = controller.take_identity_unlock();
        let attention = controller.take_attention();
        let attention_message = attention.and_then(|rule| {
            if self.backend_lost || self.live_commands.is_none() {
                controller.error = Some("Messaging connection is unavailable".into());
                None
            } else {
                match preferences::messaging::command(&view.values.messaging, rule) {
                    Ok(message) => Some(message),
                    Err(reason) => {
                        controller.error = Some(reason);
                        None
                    }
                }
            }
        });
        if unlock && let Some(message) = self.custody.request(true, Instant::now()) {
            self.queue(message);
        }
        if let Some(message) = attention_message {
            if let live::UiMsg::SetAttention { id, seat, .. } = &message {
                self.settings_attention = Some((*id, *seat, true));
            }
            self.queue(message);
        }
        if let Some(request) = knowledge
            && let Err(reason) = request.and_then(|r| self.knowledge_request(r))
            && let Some(LocalTab::Settings { controller, .. }) = &mut self.local_tab
        {
            controller.error = Some(reason);
        }
        if let Some(request) = plugin_request {
            if let Err(reason) = request.and_then(|request| self.plugin_request(request))
                && let Some(LocalTab::Settings { controller, .. }) = &mut self.local_tab
            {
                controller.error = Some(reason);
            }
        } else if read_plugins {
            let _ = self.plugin_request(bbt_proto::gui::PluginRequest::List);
        }
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
        self.scene_dirty = true;
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
        if self.closing || self.backend_lost {
            return;
        }
        admit_reads(
            self.live_commands.as_deref(),
            &mut self.operation_backlog,
            || {
                let mut reads = Vec::new();
                if viewed && let Some(action) = self.message_history.viewed_all() {
                    reads.push(action.into());
                }
                reads.extend(
                    self.message_history
                        .actions()
                        .into_iter()
                        .map(live::UiMsg::from),
                );
                reads.extend(self.message_history.commands());
                reads
            },
        );
    }
    fn task_reads(&mut self) {
        if self.closing || self.backend_lost {
            return;
        }
        admit_reads(
            self.live_commands.as_deref(),
            &mut self.operation_backlog,
            || self.task_records.commands(),
        );
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
        if let Some(swarm) = self.live_state.as_ref().and_then(|m| {
            m.ui.window
                .sidebar
                .swarms
                .iter()
                .find(|s| Some(s.id) == self.composer_scope)
        }) {
            self.conversation
                .entries
                .extend(swarm.stream.iter().filter_map(|item| {
                    if let bbt_ui_model::StreamItem::Presence { text, time, .. } = item {
                        Some(bbt_ui::modules::stream_dock::Entry::Message {
                            data: Box::new(bbt_ui::modules::stream_dock::Message {
                                text: text.clone(),
                                time: time.clone(),
                                presence: true,
                                kind: Some("presence".into()),
                                ..Default::default()
                            }),
                            show_channel: None,
                        })
                    } else {
                        None
                    }
                }));
            self.conversation
                .entries
                .extend(self.scene.restart_lines(swarm.id).map(|text| {
                    bbt_ui::modules::stream_dock::Entry::Message {
                        data: Box::new(bbt_ui::modules::stream_dock::Message {
                            text: text.into(),
                            presence: true,
                            kind: Some("presence".into()),
                            ..Default::default()
                        }),
                        show_channel: None,
                    }
                }));
        }
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
            Action::OpenRecord(target) => self.open_document(target),
            Action::Task(id) => self.open_document(id),
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
        self.task_reads();
        self.refresh_conversation();
        if let Some(window) = &self.window {
            window.set_ime_allowed(
                !self.document.open
                    && (self.launcher.is_some() || self.composer.view.focus || self.reject_focus),
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
                            Ok(batch) => {
                                self.admit_composer(batch);
                            }
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
    /// Native sends and the UI relay admit the same prepared composer batch.
    fn admit_composer(&mut self, batch: messaging::Batch) -> Vec<bbt_mux::Ulid> {
        let commands = self.composer_client.admitted(batch);
        let ids = commands
            .iter()
            .filter_map(|command| match command {
                live::UiMsg::Messaging { id, .. } => Some(*id),
                _ => None,
            })
            .collect();
        self.operation_backlog.extend(commands);
        ids
    }

    fn compose_ui(&mut self, request: bbt_proto::gui::UiCompose) -> Result<(), String> {
        if self.live_commands.is_none() || self.closing || self.backend_lost {
            return Err("backend_unavailable".into());
        }
        if self.document.open || self.launcher.is_some() {
            return Err("composer_not_visible: close the document or launcher first".into());
        }
        let (model, stage) = self
            .live_state
            .as_deref()
            .zip(self.focus.stage.as_ref())
            .ok_or("swarm_required")?;
        let draft = messaging::ui_compose_draft(model, stage, &request)?;
        let label = draft.to[0].label.clone();
        let mut controller = ComposeController::new(composer::Composer {
            to: draft.to.clone(),
            text: draft.text.clone(),
            kind: draft.kind.clone(),
            focus: true,
            ..Default::default()
        });
        let ComposeOutcome::Submit(draft) = controller.handle(ComposeCommand::Send) else {
            return Err("composer_submission_refused".into());
        };
        // In particular, a role over 32 refuses here before draft, queue or group changes.
        let batch = self.composer_client.prepare_for(model, stage, draft)?;
        self.composer_catalog();
        self.composer = controller;
        let messages = self.admit_composer(batch);
        self.message_history.channel("all");
        self.conversation_reads(false);
        self.composer_notice();
        self.ui_collect
            .as_mut()
            .expect("UI command collection")
            .push(UiEffect::Compose(bbt_proto::gui::UiComposed {
                to: request.to,
                text: request.text,
                messages,
                label,
                rect: [0.; 4],
            }));
        Ok(())
    }

    fn composed_entry(&self, sent: &bbt_proto::gui::UiComposed) -> Option<usize> {
        let ids: Vec<_> = sent.messages.iter().map(ToString::to_string).collect();
        self.conversation.entries.iter().position(|entry| {
            matches!(entry, bbt_ui::modules::stream_dock::Entry::Message {data, ..}
                if data.message_ids == ids && data.text == sent.text
                    && (!matches!(sent.to, bbt_proto::gui::UiComposeTarget::Role(_))
                        || data.to_label.as_deref() == Some(format!("{} · {} recipients", sent.label, ids.len()).as_str())))
        })
    }

    fn composed_frame(
        &self,
        sent: &bbt_proto::gui::UiComposed,
    ) -> Option<bbt_proto::gui::UiComposed> {
        fn named<'a>(
            layout: &'a bbt_ui::components::Layout,
            name: &str,
        ) -> Option<&'a bbt_ui::components::Layout> {
            if matches!(&layout.kind, bbt_ui::components::LayoutKind::Named(key) if key == name) {
                return Some(layout);
            }
            layout.children.iter().find_map(|child| named(child, name))
        }
        let index = self.composed_entry(sent)?;
        let layout = &self.conversation_layout.as_ref()?.layout;
        let viewport = named(layout, "conversation/rows")?.rect;
        let row = named(layout, &format!("e{index}"))?.rect;
        let (x, y) = (row.x.max(viewport.x), row.y.max(viewport.y));
        let (w, h) = (
            (row.right().min(viewport.right()) - x).max(0.),
            (row.bottom().min(viewport.bottom()) - y).max(0.),
        );
        if w <= 0. || h <= 0. {
            return None;
        }
        let mut receipt = sent.clone();
        receipt.rect = [x, y, w, h];
        Some(receipt)
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
        if self.backend_lost || self.closing || self.live_commands.is_none() {
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
        self.starting.release_focus();
        if let Some(answer) = self
            .ctl_focus
            .fail("focus_superseded: a newer UI focus action replaced this request")
        {
            self.answer_focus(answer);
        }
        self.focus_action_applied(action);
    }
    fn focus_action_applied(&mut self, action: FocusAction) {
        if self.document.open {
            self.close_document();
        }
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
                self.scene_dirty = true;
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
        let now = self.clock.now();
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
            if let live::UiMsg::AttentionFocus {
                seat,
                landed: Some(at),
                ..
            } = &action
            {
                self.animation_landing = Some((*seat, *at));
            }
            self.queue(action);
        }
    }
    fn recording_active(&self) -> bool {
        #[cfg(debug_assertions)]
        {
            self.notification_recorder.is_some()
        }
        #[cfg(not(debug_assertions))]
        {
            false
        }
    }
    fn recording_status(&self) -> serde_json::Value {
        #[cfg(debug_assertions)]
        if let Some(recorder) = &self.notification_recorder {
            // Read the completion fence before the recording; a settled
            // snapshot therefore includes every effect of that plan.
            let (submitted, processed) = self
                .notifications
                .as_ref()
                .map_or((0, 0), |worker| worker.progress());
            return match recorder.snapshot() {
                Ok(mut state) => {
                    state["available"] = serde_json::json!(true);
                    state["plan_revision"] = serde_json::json!(submitted);
                    state["processed_revision"] = serde_json::json!(processed);
                    state["settled"] = serde_json::json!(submitted == processed);
                    state["waiting"]=serde_json::json!(self.live_state.as_ref().map(|model|model.attention.iter().take(64).map(|(seat,attention)|serde_json::json!({"seat":bbt_mux::Ulid(seat.0.0),"muted":attention.muted})).collect::<Vec<_>>()).unwrap_or_default());
                    state["focused"] = serde_json::json!(
                        self.focus
                            .stage
                            .as_ref()
                            .and_then(|stage| stage.focused)
                            .map(|seat| bbt_mux::Ulid(seat.0.0))
                    );
                    bbt_mux::debug::sanitize(state)
                }
                Err(reason) => {
                    serde_json::json!({"backend":"recording","available":false,"reason":reason,"os_presentation":false})
                }
            };
        }
        serde_json::json!({"backend":"not_recording","available":false,
            "reason":"start a debug app with --test-account DIR --notifications record"})
    }
    fn install_recording(&mut self) -> Result<(), String> {
        #[cfg(debug_assertions)]
        {
            let proxy = self.proxy.clone();
            let recorder =
                bboxy450_term::notifications::recording::Recorder::new(Arc::new(move |response| {
                    let _ = proxy.send_event(UiEvent::Notification(response));
                }));
            let proxy = self.proxy.clone();
            self.notifications = Some(
                bboxy450_term::notifications::Worker::start(
                    Box::new(recorder.clone()),
                    Arc::new(move |event| {
                        let _ = proxy.send_event(match event {
                            bboxy450_term::notifications::Event::Ready => {
                                UiEvent::NotificationReady
                            }
                            bboxy450_term::notifications::Event::Error(reason) => {
                                UiEvent::NotificationError(reason)
                            }
                        });
                    }),
                )
                .map_err(|e| e.to_string())?,
            );
            self.notification_recorder = Some(recorder);
            Ok(())
        }
        #[cfg(not(debug_assertions))]
        {
            Err("notification recording needs debug assertions".into())
        }
    }
    fn recording_click(&self, seat: bbt_mux::Ulid) -> Result<(), String> {
        #[cfg(debug_assertions)]
        {
            self.notification_recorder
                .as_ref()
                .ok_or("notification recording is not selected")?
                .activate(&bboxy450_desktop_notifications::NotificationId(
                    seat.to_string(),
                ))
                .map_err(String::from)
        }
        #[cfg(not(debug_assertions))]
        {
            let _ = seat;
            Err("notification recording needs debug assertions".into())
        }
    }
    /// A switched pane lands only after the driver solves an actual frame.
    /// Both native and headless use the same focus owner and cover rules.
    fn focus_layout_ready(&mut self) {
        if let Some(model) = &self.live_state
            && self.local_tab.is_none()
            && !self.document.open
        {
            let events = self.focus.layout_ready(&model.ui, &self.focus_bounds);
            self.attention_focus_events(&events, false);
            for event in events {
                self.logger.write(format!("ui input: {event:?}"));
            }
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
                if self.recording_active() {
                    if self
                        .notification_ui
                        .as_ref()
                        .is_some_and(|ask| ask.seat == SeatId::from_u128(id.0))
                    {
                        let ask = self
                            .notification_ui
                            .take()
                            .expect("matching recording click");
                        self.begin_focus(ask);
                    }
                    return;
                }
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
            self.window.is_some() || self.recording_active(),
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
    fn answer_focus(&mut self, answer: bboxy450_term::focus_ack::Answer) {
        if self.ui_focus == Some(answer.id) {
            self.ui_focus = None;
            let result = answer
                .result
                .map_err(|reason| {
                    if reason.starts_with("focus_timeout:") {
                        format!("timeout: {reason}")
                    } else {
                        format!("ui_frame_refused: {reason}")
                    }
                })
                .and_then(|frame| {
                    self.last_ui_frame().and_then(|layout| {
                        if layout.frame_seq >= frame.frame_seq
                            && layout.focused == Some(bbt_mux::Ulid(frame.seat.0.0))
                        {
                            Ok(layout)
                        } else {
                            Err(
                                "ui_frame_refused: no produced layout contains the focused pane"
                                    .into(),
                            )
                        }
                    })
                });
            self.answer_ui(answer.id, result);
            return;
        }
        self.queue(live::UiMsg::Focused {
            id: answer.id,
            result: answer.result,
        });
    }
    fn begin_focus(&mut self, ask: bboxy450_term::focus_ack::Ask) {
        let id = ask.id;
        if let Some(answer) = self.ctl_focus.set(ask) {
            self.answer_focus(answer);
        }
        let proxy = self.proxy.clone();
        let timer = bbt_rt::handle().spawn(async move {
            tokio::time::sleep(bboxy450_term::focus_ack::DEADLINE).await;
            let _ = proxy.send_event(UiEvent::FocusDeadline(id));
        });
        self.ctl_focus.arm(timer.abort_handle());
    }
    fn poll_focus(&mut self) {
        // The headless driver supplies its own submitted/completed counters.
        if self.window.is_some() {
            let completed = self.meter.total_frames();
            self.poll_focus_frame(completed, completed + 1);
        }
    }
    fn poll_focus_frame(&mut self, completed: u64, next_frame: u64) -> bool {
        let Some(model) = self.live_state.as_deref() else {
            return false;
        };
        let effect = self.ctl_focus.poll(
            model,
            self.scene.focused(),
            |seat| self.pane_bounds.iter().any(|(id, _)| *id == seat),
            completed,
        );
        match effect {
            Some(bboxy450_term::focus_ack::Effect::Apply(seat)) => {
                let generation = model.ui.seats[&seat].process_generation;
                self.focus_action_applied(FocusAction::Activate(seat, FocusOrigin::Row));
                if self.focus.stage.as_ref().and_then(|stage| stage.focused) == Some(seat) {
                    self.ctl_focus.applied(next_frame, generation);
                    self.request_redraw();
                    return true;
                } else if let Some(answer) = self
                    .ctl_focus
                    .fail("not_on_screen: focus could not activate the requested pane")
                {
                    self.answer_focus(answer);
                }
            }
            Some(bboxy450_term::focus_ack::Effect::Answer(answer)) => self.answer_focus(answer),
            None => (),
        }
        false
    }
    fn close_seat(&mut self, seat: SeatId, id: bbt_mux::Ulid, undo: bool) {
        if self.closing || self.live_commands.is_none() {
            if let Some(effects) = &mut self.ui_collect {
                effects.push(UiEffect::Refused("ui_not_ready: no command sender".into()));
            }
            return;
        }
        let Some(model) = &self.live_state else {
            return;
        };
        match close_request(&self.scene, model, seat, id, undo, Instant::now()) {
            Ok(message) => self.queue(message),
            Err(reason) => {
                self.logger.write(format!(
                    "{} {seat} refused: {reason}",
                    if undo { "undo close" } else { "close" }
                ));
                if let Some(effects) = &mut self.ui_collect {
                    effects.push(UiEffect::Refused(reason));
                }
            }
        }
    }
    fn undo_latest_close(&mut self) {
        if let Some(seat) = self
            .live_state
            .as_ref()
            .and_then(|m| m.closes.latest(Instant::now()))
        {
            self.close_seat(seat, bbt_mux::Ulid::new(), true);
        }
    }
    fn queue(&mut self, mut message: live::UiMsg) {
        if self.closing || self.live_commands.is_none() {
            return;
        }
        let launch = match self.prepare_starting(&mut message) {
            Ok(launch) => launch,
            Err(reason) => {
                self.logger.write(format!("launch refused: {reason}"));
                if let Some(effects) = &mut self.ui_collect {
                    effects.push(UiEffect::Refused(reason));
                }
                return;
            }
        };
        let superseding_scroll = match &message {
            live::UiMsg::Input { seat, .. } => Some(*seat),
            live::UiMsg::Scroll { seat, request, .. } if request.op.is_none() => Some(*seat),
            _ => None,
        };
        let effect = self
            .ui_collect
            .as_ref()
            .and_then(|_| UiEffect::from_message(&message, self.live_state.as_deref()));
        match queue_pending(&mut self.pending_commands, message) {
            Err(kind) => {
                self.admission_refusals.record(kind);
                self.logger.write(self.admission_refusals.report());
                if let Some(effects) = &mut self.ui_collect {
                    effects.push(UiEffect::Refused("ui_command_queue_full".into()));
                }
            }
            Ok(()) => {
                if let Some(launch) = launch {
                    let op = launch.op;
                    self.admit_starting(launch);
                    let pending = &mut self.pending_commands;
                    self.spawn_gate.capture(op, || {
                        let index = pending
                            .iter()
                            .position(|message| match message {
                                live::UiMsg::Spawn { id, .. }
                                | live::UiMsg::SpawnSpec { id, .. } => *id == op,
                                _ => false,
                            })
                            .expect("newly admitted spawn is still queued");
                        pending.remove(index).expect("admitted queue entry")
                    });
                }
                if superseding_scroll.is_some_and(|seat| self.ui_wait.as_ref().is_some_and(|wait| wait.effects.iter().any(|effect| matches!(effect, UiEffect::Scroll(_, target) if *target == seat)))) {
                    self.cancel_ui("ui_scroll_refused: superseded by admitted input/navigation");
                }
                if let (Some(effect), Some(effects)) = (effect, &mut self.ui_collect) {
                    effects.push(effect);
                }
            }
        }
        self.flush_commands();
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
                    self.answer_capture(
                        ask.id,
                        Err(format!(
                            "not_on_screen: seat {seat} has no pane on screen in frame {}",
                            frame.frame_seq
                        )),
                    );
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
                    frame_seq: self.meter.total_frames(),
                    ..Default::default()
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
        if self.closing || self.backend_lost || self.live_commands.is_none() {
            self.command_retry.cancel();
            return;
        }
        // Refill bounded operation batches in this delivery turn. Once Busy,
        // channel capacity, rather than a rendered frame, resumes delivery.
        loop {
            self.flush_operations();
            let sender = self.live_commands.as_ref().unwrap();
            if flush_pending(sender, &mut self.pending_commands, false).is_err() {
                self.command_delivery_lost();
                return;
            }
            if !self.pending_commands.is_empty() {
                let proxy = self.proxy.clone();
                self.command_retry.arm(sender, move |ready| {
                    let _ = proxy.send_event(UiEvent::CommandsReady(ready));
                });
                return;
            }
            if self.operation_backlog.is_empty() {
                self.command_retry.cancel();
                return;
            }
        }
    }
    fn commands_ready(&mut self, ready: command_retry::Ready) {
        let Some(permit) = self.command_retry.accept(ready) else {
            return;
        };
        if self.closing || self.backend_lost || self.live_commands.is_none() {
            return;
        }
        self.flush_operations();
        match permit {
            Ok(permit) => {
                if let Some(message) = self.pending_commands.pop_front() {
                    permit.send(message);
                }
            }
            Err(()) => self.command_delivery_lost(),
        }
        self.flush_commands();
    }
    fn command_delivery_lost(&mut self) {
        self.command_retry.cancel();
        if !self.backend_lost {
            self.backend_lost = true;
            self.scene.backend_lost();
            // A changed backend status is paint work; retained commands aren't.
            self.request_redraw();
        }
    }
    fn watch_panes(&mut self, bounds: &[(SeatId, Rect)], scale: f32) {
        if self.live_commands.is_none() {
            return;
        }
        let Some(model) = &self.live_state else {
            return;
        };
        let (watched, sizes) = pane_controls(
            model,
            bounds,
            &self.watched,
            &self.pane_sizes,
            self.scene.focused(),
            scale,
            self.verify.grid_size.filter(|_| !self.verify.grid_ready),
        );
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
        self.command_retry.cancel();
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
        self.request_redraw_sources(&[stall::DIRECT_REDRAW_SOURCE]);
    }
    fn request_redraw_sources(&self, sources: &[usize]) {
        self.wake_counters.borrow_mut().redraw(sources);
        let first = !self.redraw_pending.replace(true);
        if !self.occluded
            && let Some(pacer) = &self.pacer
        {
            pacer.request_frame();
        }
        // Paint demand survives a tickless callback. Only a tick, GPU-ready
        // notice or direct window action can request its next delivery. FIFO
        // and inactive CV paths have no future Metal tick to wait for.
        let explicit = sources.iter().any(|source| {
            matches!(
                *source,
                2 | display_link::TICK_SOURCE | stall::DIRECT_REDRAW_SOURCE
            )
        });
        let unpaced = self
            .pacer
            .as_ref()
            .is_some_and(|pacer| pacer.is_fifo() || (!pacer.is_metal() && !self.animate));
        if first || explicit || unpaced {
            self.queue_redraw(sources);
        } else {
            self.wake_counters.borrow_mut().coalesced(sources);
        }
    }
    fn queue_redraw(&self, sources: &[usize]) {
        if self.redraw_queued.replace(true) {
            self.wake_counters.borrow_mut().coalesced(sources);
            return;
        }
        self.wake_counters.borrow_mut().scheduled(sources);
        if let Some(window) = &self.window {
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
        if !self.document.open
            && (self.launcher.is_some() || self.composer.view.focus || self.settings_ime())
        {
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
            self.inbox.dirty.paint_source(5).mark();
        }
        Ok(())
    }
    fn install_gpu(&mut self) -> Result<(), String> {
        if let Some(result) = self.prepared.take() {
            let (mut gpu, scene) = result?;
            self.scene = scene;
            self.apply_preferences();
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
    fn drain_runtime_notices(&self) {
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
        // A budgeted drain must make progress even if the producer has stopped.
        // These memory-only queues are not inputs to the visible scene.
        self.inbox.messages.complete_drain();
        self.inbox.output.complete_drain();
    }
    fn drain(&mut self) -> Vec<usize> {
        let _measurement = bbt_rt::diagnostics::span(bbt_rt::diagnostics::Phase::UiDrain);
        if let Some(stall) = &self.stall {
            stall.drain();
        }
        let sources = self.inbox.dirty.drain();
        self.wake_counters.borrow_mut().drained(&sources);
        self.drain_runtime_notices();
        if let Some(sequence) = self.updates.take() {
            self.scene.update(sequence);
            self.last_update = sequence;
        }
        let latest_measurement =
            bbt_rt::diagnostics::span(bbt_rt::diagnostics::Phase::UiLatestTake);
        let state = self.live_updates.take();
        drop(latest_measurement);
        let received_at = bbt_rt::diagnostics::now();
        if let Some(state) = state {
            if !self.window_restored
                && state.metadata_initialized
                && let Some(saved) = &state.window_restore
            {
                self.window_restored = true;
                self.window_first_frame = true;
                self.window_last = Some((**saved).clone());
                if let Some(window) = &self.window {
                    self.restored_size = Some(saved.size);
                    let _ = window.request_inner_size(LogicalSize::new(
                        f64::from(saved.size[0]),
                        f64::from(saved.size[1]),
                    ));
                } else {
                    self.requested_size = Some(saved.size);
                }
            }

            if self.ui_observe {
                self.frame_observations.ingest(&state);
            }
            if self.starting.observe(&state) {
                self.starting_changed();
            }
            self.mute.observe(&state);
            if let Some(reply) = &state.custody_reply {
                self.custody.observe(reply, Instant::now());
            }
            let signing_ready = state.identity.as_ref().is_some_and(|identity| {
                matches!(
                    identity.state,
                    bbt_proto::gui::GuiIdentityState::LocalDev
                        | bbt_proto::gui::GuiIdentityState::Certified
                )
            });
            if let Some(message) = self.custody.signing_changed(signing_ready, Instant::now()) {
                self.queue(message);
            }
            self.composer_notice();
            if let Some(reply) = &state.document_reply
                && self.document.apply(reply)
            {
                self.queue(live::UiMsg::ConsumeDocument { id: reply.id });
                self.logger.write(format!(
                    "document {}: {:?}",
                    self.document.target,
                    match &reply.response {
                        documents::Response::Resolved(_) => "resolved",
                        documents::Response::Fetched(_) => "loaded",
                        documents::Response::Failed(_) => "refused",
                    }
                ));
                self.document_layout = None;
                self.request_redraw();
            }
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
                stall.model_received_at(&state, received_at);
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
            let restored = state.ui.window.stage.focused.filter(|id| {
                self.live_state.as_ref().is_some_and(|old| {
                    old.ui
                        .seats
                        .get(id)
                        .zip(state.ui.seats.get(id))
                        .is_some_and(|(a, b)| {
                            matches!(a.lifecycle, bbt_ui_model::Lifecycle::ClosedGrace { .. })
                                && b.lifecycle == bbt_ui_model::Lifecycle::Open
                                && a.process_generation == b.process_generation
                        })
                })
            });
            self.focus
                .reconcile(self.live_state.as_ref().map(|old| &old.ui), &state.ui);
            if self.document.open && self.document_return.is_none() {
                self.document_return = self.focus.stage.clone();
            }
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
            self.scene_dirty = true;
            let root_changed = self
                .live_state
                .as_ref()
                .and_then(|m| m.knowledge_roots.as_ref())
                .and_then(|s| s.roots.first())
                .map(|r| r.revision)
                != state
                    .knowledge_roots
                    .as_ref()
                    .and_then(|s| s.roots.first())
                    .map(|r| r.revision);
            if root_changed && self.document.open {
                self.document.show(self.document.target.clone());
            }
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
            {
                let _drop = bbt_rt::diagnostics::span(bbt_rt::diagnostics::Phase::UiOldModelDrop);
                self.live_state = Some(state);
            }
            if matches!(self.local_tab, Some(LocalTab::Settings { .. })) {
                self.request_attention_read(false);
            }
            if let Some(seat) = restored {
                self.focus_action(FocusAction::Activate(seat, FocusOrigin::Keyboard));
            }
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
            self.task_reads();
            self.refresh_conversation();
            for (op, outcome) in results {
                if let bboxy450_term::model::SpawnOutcome::Placed(seat) = &outcome {
                    if !self.document.open && self.starting.should_focus(op) {
                        self.focus_action_applied(FocusAction::Activate(
                            *seat,
                            FocusOrigin::Keyboard,
                        ));
                    } else {
                        self.logger
                            .write(format!("spawn {op} placed; document return focus retained"));
                    }
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
        self.drive_document();
        self.poll_focus();
        if self.window.is_some() {
            self.poll_ui_frame(self.meter.total_frames());
        }
        self.flush_commands();
        sources
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
    /// Choose a UI command without changing the physical keyboard lifecycle.
    /// ctl sends logical chords; only the native adapter tracks held keys.
    fn route_chrome(&self, input: &keyboard::Input<'_>, modes: u64) -> RoutedKey {
        if self.launcher.is_none()
            && !self.document.open
            && self.local_tab.is_none()
            && input.state == ElementState::Pressed
            && !input.repeat
            && (matches!(&input.logical_key, Key::Named(NamedKey::Escape))
                || (input.modifiers.state.super_key()
                    && keyboard::shortcut_key(input) == Some('w')))
            && let Some(op) = self.starting.focused()
            && self
                .starting
                .get(op)
                .is_some_and(|launch| launch.refused.is_some())
        {
            return RoutedKey::DismissStarting(op);
        }
        if self.document.open {
            route_document(input, modes)
        } else if self.local_tab.is_some()
            && self.launcher.is_none()
            && !self.composer.view.focus
            && !self.reject_focus
        {
            if matches!(self.local_tab, Some(LocalTab::Settings { .. })) {
                route_settings(input, modes, self.settings_ime())
            } else {
                route_local_page(input, modes)
            }
        } else if self.launcher.is_none() && (self.composer.view.focus || self.reject_focus) {
            route_composer(
                input,
                modes,
                if self.reject_focus {
                    ComposeField::Body
                } else if self.composer.view.kind_menu || self.composer.view.attach_pick.is_some() {
                    ComposeField::Address
                } else {
                    self.composer.view.field
                },
            )
        } else {
            route_key(
                input,
                modes,
                self.launcher
                    .as_ref()
                    .map(|launcher| launcher.launcher.query.as_str()),
            )
        }
    }
    fn dispatch_chrome(&mut self, routed: RoutedKey) -> Option<RoutedKey> {
        match routed {
            RoutedKey::OpenLauncher => self.open_launcher(),
            RoutedKey::OpenGraph => self.open_graph(),
            RoutedKey::OpenSettings => self.open_settings("Account"),
            RoutedKey::SettingsKey(key) => self.settings_key(key),
            RoutedKey::MuteFocused => self.mute_focused(),
            RoutedKey::UndoClose => self.undo_latest_close(),
            RoutedKey::DismissStarting(op) => {
                if self.dismiss_starting(op)
                    && let Some(effects) = &mut self.ui_collect
                {
                    effects.push(UiEffect::StartingDismissed(op));
                }
            }
            RoutedKey::LocalPage(command) => {
                if self.document.open {
                    self.document_command(command);
                } else {
                    self.local_page_command(command);
                }
            }
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
            RoutedKey::Pane(navigation) => self.focus_action(FocusAction::Navigate(navigation)),
            other => return Some(other),
        }
        None
    }
    /// Both a Wake and a ctl read can consume the coalesced data/tick sources.
    fn dispatch_wake_sources(&self, sources: &[usize]) {
        if sources.iter().any(|source| matches!(*source, 0 | 1)) {
            self.drain_runtime_notices();
        }
        match wake_action(
            sources,
            self.pacer.as_ref().is_some_and(DisplayLink::is_active),
        ) {
            WakeAction::Ignore => {
                if sources.contains(&display_link::TICK_SOURCE) {
                    self.wake_counters.borrow_mut().ignored_tick();
                }
                if let Some(pacer) = &self.pacer
                    && !pacer.is_active()
                {
                    pacer.clear();
                }
            }
            WakeAction::Drain => {
                // The smoke's first producer releases GPU preparation from a
                // pre-GPU callback. Keep that one startup handshake, even when
                // its notice carries no paint change.
                if self.gpu.is_none() && self.smoke_wake_seen && self.smoke_bootstrap.is_some() {
                    self.request_redraw();
                }
            }
            WakeAction::Tick => {
                self.wake_counters.borrow_mut().redraw(sources);
                self.redraw_pending.set(true);
                self.queue_redraw(sources);
            }
            WakeAction::Invalidate => self.request_redraw_sources(sources),
        }
    }
    fn answer_ui(&mut self, id: bbt_mux::Ulid, result: Result<bbt_proto::gui::UiLayout, String>) {
        if let Err(kind) = queue_pending(
            &mut self.pending_commands,
            live::UiMsg::UiReply {
                id,
                result: Box::new(result),
            },
        ) {
            self.admission_refusals.record(kind);
            self.logger.write(self.admission_refusals.report());
        }
        self.flush_commands();
    }
    fn last_ui_frame(&self) -> Result<bbt_proto::gui::UiLayout, String> {
        self.ui_frame
            .as_ref()
            .map(|(_, layout)| layout.clone())
            .unwrap_or_else(|| Err("ui_not_ready: no produced layout frame".into()))
    }
    fn ui_visible(&self) -> UiVisible {
        let stage = self
            .focus
            .stage
            .as_ref()
            .or_else(|| self.live_state.as_ref().map(|model| &model.ui.window.stage));
        UiVisible {
            launcher: self.launcher.is_some(),
            page: self.local_tab.as_ref().map(|tab| match tab {
                LocalTab::Graph(_) => "graph".into(),
                LocalTab::Settings { .. } => "settings".into(),
            }),
            document: self.document.open.then(|| self.document.target.clone()),
            focused: stage
                .and_then(|stage| stage.focused)
                .map(|seat| bbt_mux::Ulid(seat.0.0)),
            tab: stage
                .and_then(|stage| stage.tab)
                .map(|tab| bbt_mux::Ulid(tab.0.0)),
        }
    }
    fn cancel_ui(&mut self, reason: &str) {
        if let Some(ask) = self.notification_ui.take() {
            self.ui_focus = None;
            self.answer_ui(ask.id, Err(reason.into()));
        }
        if let Some(wait) = self.ui_wait.take() {
            self.answer_ui(wait.id, Err(reason.into()));
        }
        if self.ui_focus.is_some()
            && let Some(answer) = self.ctl_focus.fail(reason)
        {
            self.answer_focus(answer);
        }
    }
    fn ui_scroll_position(
        &self,
        id: bbt_mux::Ulid,
        seat: SeatId,
    ) -> Result<Option<bbt_proto::gui::UiScrollPosition>, String> {
        let Some(model) = self.live_state.as_ref() else {
            return Ok(None);
        };
        let Some(reply) = model
            .scroll
            .reply
            .as_ref()
            .filter(|reply| reply.id == id && reply.seat == seat)
        else {
            return Ok(None);
        };
        if !model.ui.seats.get(&seat).is_some_and(|record| {
            record.process_generation == reply.generation
                && record.lifecycle == bbt_ui_model::Lifecycle::Open
        }) {
            return Err("ui_scroll_refused: pane generation is no longer live".into());
        }
        let expected = reply
            .result
            .as_ref()
            .map_err(|reason| format!("ui_scroll_refused: {reason}"))?;
        let actual = model
            .scroll
            .position(seat, model.history.get(&seat).map(Arc::as_ref));
        if actual.live != expected.live || (!actual.live && actual.top_row != expected.top_row) {
            return Err("ui_scroll_refused: display changed before the produced frame".into());
        }
        Ok(Some(*expected))
    }
    fn ui_effect_ready(&self, effect: &UiEffect) -> Result<bool, String> {
        use bboxy450_term::model::SpawnOutcome;
        let Some(model) = &self.live_state else {
            return Ok(false);
        };
        match effect {
            UiEffect::Compose(sent) => Ok(self.composed_entry(sent).is_some()),
            UiEffect::Spawn(id) => {
                if let Some(launch) = self.starting.get(*id) {
                    return launch
                        .refused
                        .as_ref()
                        .map_or(Ok(false), |reason| Err(reason.clone()));
                }
                if let Some(seat) = self.starting.completed_seat(*id) {
                    return Ok(model.ui.tab_of(seat).is_some());
                }
                match self
                    .spawn_results_seen
                    .get(id)
                    .or_else(|| model.spawns.get(id).map(|request| &request.outcome))
                {
                    Some(SpawnOutcome::Placed(seat)) => Ok(model.ui.tab_of(*seat).is_some()),
                    Some(SpawnOutcome::Pending | SpawnOutcome::AwaitingSeat(_)) | None => Ok(false),
                    Some(outcome) => Err(format!("ui_frame_refused: spawn {id}: {outcome:?}")),
                }
            }
            UiEffect::Plugin { op, before } => Ok(model
                .operation_results
                .rows()
                .any(|row| row.op == *op && row.serial > *before)),
            UiEffect::Operation(id) => {
                Ok(model.operation_results.rows().any(|reply| reply.op == *id))
            }
            UiEffect::Scroll(id, seat) => self
                .ui_scroll_position(*id, *seat)
                .map(|position| position.is_some()),
            UiEffect::Mute(seat, muted) => {
                if model
                    .ui
                    .seats
                    .get(seat)
                    .is_some_and(|record| record.muted == *muted)
                {
                    Ok(true)
                } else if self.mute.pending() {
                    Ok(false)
                } else {
                    Err(format!("ui_frame_refused: mute for {seat} was not applied"))
                }
            }
            UiEffect::Lifecycle(target) => target.ready(model),
            UiEffect::Document => {
                Ok(self.document.prepared.is_some() || self.document.error.is_some())
            }
            UiEffect::Graph => Ok(model.e2e.is_some()),
            UiEffect::Preferences { saved } => Ok(self.preferences.loaded
                && (!*saved || self.preferences.saved() || self.preferences.error.is_some())),
            UiEffect::Clock { state, landed } => {
                if self.clock.revision() == state.revision {
                    // A clock step must not outrun the earlier focus episode whose
                    // phase it is about to render. The model keeps the original
                    // App-clock landing instant; publication is the completion.
                    Ok(landed.is_none_or(|target| model.focus_swell == Some(target)))
                } else {
                    Err(
                        "ui_frame_refused: a newer animation clock command superseded this request"
                            .into(),
                    )
                }
            }
            UiEffect::Size([w, h]) => {
                Ok((self.solved_size[0] - w).abs() < 1. && (self.solved_size[1] - h).abs() < 1.)
            }
            UiEffect::Refused(reason) => Err(format!("ui_frame_refused: {reason}")),
            UiEffect::StartingDismissed(op) => {
                Ok(self.starting.get(*op).is_none_or(|launch| launch.dismissed))
            }
        }
    }
    fn poll_ui_frame(&mut self, submitted: u64) -> bool {
        let Some(wait) = &self.ui_wait else {
            return false;
        };
        if wait.after.is_some() {
            return false;
        }
        let ready = wait.effects.iter().try_fold(true, |ready, effect| {
            self.ui_effect_ready(effect).map(|next| ready && next)
        });
        match ready {
            Err(reason) => {
                let wait = self.ui_wait.take().expect("UI waiter observed");
                self.answer_ui(wait.id, Err(reason));
                false
            }
            Ok(false) => false,
            Ok(true) => {
                let visible = self.ui_visible();
                let placed = self
                    .ui_wait
                    .as_ref()
                    .and_then(|wait| wait.spawn.as_ref())
                    .and_then(|spawn| {
                        self.starting.completed_seat(spawn.op).or_else(|| {
                            match self.spawn_results_seen.get(&spawn.op).or_else(|| {
                                self.live_state
                                    .as_ref()?
                                    .spawns
                                    .get(&spawn.op)
                                    .map(|s| &s.outcome)
                            }) {
                                Some(bboxy450_term::model::SpawnOutcome::Placed(seat)) => {
                                    Some(*seat)
                                }
                                _ => None,
                            }
                        })
                    });
                let seat_row = placed.and_then(|seat| {
                    self.live_state
                        .as_ref()?
                        .gui_seats
                        .iter()
                        .find(|row| row.record.id.0 == seat.0.0)
                        .cloned()
                });
                let wait = self.ui_wait.as_mut().expect("UI waiter observed");
                if let Some(spawn) = &mut wait.spawn {
                    spawn.seat = placed;
                    spawn.seat_row = seat_row;
                }
                wait.after = Some(submitted + 1);
                wait.visible = Some(visible);
                self.request_redraw();
                true
            }
        }
    }
    fn present_ui(&mut self, frame: u64, mut layout: Result<bbt_proto::gui::UiLayout, String>) {
        if let Ok(layout) = &mut layout {
            layout.frame_seq = frame;
            for launch in &mut layout.starting {
                launch.first_frame_seq =
                    self.starting.first(launch.op).map(|first| first.frame_seq);
            }
        }
        self.ui_frame = Some((frame, layout.clone()));
        if self
            .ui_wait
            .as_ref()
            .is_some_and(|wait| wait.after.is_some_and(|after| frame >= after))
        {
            let wait = self.ui_wait.take().expect("qualifying UI frame");
            let result = layout.and_then(|mut layout| {
                for effect in &wait.effects {
                    if let UiEffect::Compose(sent) = effect {
                        let actual = layout.composed.as_ref().ok_or("ui_frame_refused: sent group is not visible in the produced frame")?;
                        if actual.to != sent.to || actual.messages != sent.messages || actual.text != sent.text || actual.label != sent.label {
                            return Err("ui_frame_refused: produced frame shows a different composer send".into());
                        }
                    }
                    if let UiEffect::Clock { state: expected, .. } = effect {
                        let actual = layout.clock.ok_or("ui_frame_refused: frame has no animation clock")?;
                        if actual.revision != expected.revision
                            || actual.frozen != expected.frozen
                            || (expected.frozen && actual.time_ms != expected.time_ms)
                            || actual.time_ms < expected.time_ms
                        {
                            return Err("ui_frame_refused: frame does not show the requested animation time".into());
                        }
                    }
                    if let UiEffect::Lifecycle(target) = effect {
                        let model = self.live_state.as_ref().ok_or("ui_not_ready: no live model")?;
                        if !target.ready(model)?
                            || !target.shown(
                                layout.focused.map(|seat| SeatId::from_u128(seat.0)),
                                |seat| layout.panes.iter().any(|pane| pane.seat.0 == seat.0.0),
                            )
                        {
                            return Err("close_superseded: the produced frame does not show the requested lifecycle".into());
                        }
                    }
                    if let UiEffect::Scroll(id, seat) = effect {
                        let expected = self.ui_scroll_position(*id, *seat)?.ok_or("ui_scroll_refused: completion no longer available")?;
                        let actual = &layout.panes.iter().find(|pane| pane.seat.0 == seat.0.0).ok_or("not_on_screen: scroll pane left the produced frame")?.scroll;
                        if actual.live != expected.live || (!actual.live && actual.top_row != expected.top_row) {
                            return Err("ui_scroll_refused: produced frame has a different scroll position".into());
                        }
                    }
                }
                if wait.visible.as_ref() != Some(&UiVisible::from_layout(&layout)) {
                    return Err("ui_frame_refused: a newer UI state superseded this operation before its frame".into());
                }
                if let Some(spawn) = &wait.spawn {
                    let seat = spawn.seat.ok_or(
                        "ui_frame_refused: spawn target was not captured before its frame",
                    )?;
                    let id = bbt_mux::Ulid(seat.0.0);
                    if layout.focused != Some(id)
                        || !layout.panes.iter().any(|pane| pane.seat == id)
                    {
                        return Err("ui_frame_refused: produced frame does not show the spawned pane".into());
                    }
                    layout.seats = Some(vec![spawn.seat_row.clone()
                        .ok_or("ui_frame_refused: spawned seat metadata is unavailable")?]);
                    layout.spawned = Some(bbt_proto::gui::UiSpawned {
                        seat: id,
                        op: spawn.op,
                        swarm: bbt_mux::Ulid(spawn.swarm.0.0),
                        placement: spawn.placement.clone(),
                        layout_revision: spawn.layout_revision,
                        applied_layout_revision: layout
                            .layout_revision
                            .ok_or("ui_frame_refused: frame revision is unavailable")?,
                        starting: self.starting.first(spawn.op).cloned(),
                    });
                }
                Ok(layout)
            });
            self.answer_ui(wait.id, result);
        }
    }
    fn begin_ui(
        &mut self,
        id: bbt_mux::Ulid,
        request: bbt_proto::gui::UiRequest,
        logical: [f32; 2],
        submitted: u64,
        prior_spawns: Vec<bbt_mux::Ulid>,
    ) -> bool {
        use bbt_proto::gui::UiRequest;
        self.submitted_frame = submitted;
        if let Err(reason) = request.validate() {
            self.answer_ui(id, Err(reason.into()));
            return false;
        }
        if matches!(request, UiRequest::Seats) {
            let sources = self.drain();
            self.dispatch_wake_sources(&sources);
            self.answer_seats(id);
            return false;
        }
        if matches!(request, UiRequest::Layout) {
            self.answer_ui(id, self.last_ui_frame());
            return false;
        }
        if let UiRequest::SpawnGate(command) = request {
            let result = (|| {
                // Fixture control preserves the pending UI action and reports
                // the last completed frame, rather than predicting a new one.
                let layout = self.last_ui_frame()?;
                match command {
                    bbt_proto::gui::UiSpawnGateCommand::Arm => {
                        self.spawn_gate
                            .arm(self.test_account)
                            .map_err(str::to_owned)?;
                    }
                    bbt_proto::gui::UiSpawnGateCommand::Release { op } => {
                        let message = self
                            .spawn_gate
                            .release(self.test_account, op)
                            .map_err(str::to_owned)?;
                        // Move the existing bounded reservation; never prepare
                        // or admit a second launch for the release command.
                        self.pending_commands.push_front(message);
                        self.flush_commands();
                    }
                }
                Ok(layout)
            })();
            self.answer_ui(id, result);
            return false;
        }
        self.cancel_ui("ui_frame_refused: a newer UI operation superseded this request");
        let sources = self.drain();
        self.dispatch_wake_sources(&sources);
        if self.live_state.is_none() {
            self.answer_ui(id, Err("ui_not_ready: no live model".into()));
            return false;
        }
        self.ui_observe = true;
        if let UiRequest::Focus { seat } = request {
            self.ui_focus = Some(id);
            self.begin_focus(bboxy450_term::focus_ack::Ask {
                id,
                seat: SeatId::from_u128(seat.0),
                prior_spawns,
            });
            self.request_redraw();
            return true;
        }
        if let UiRequest::NotificationClick { seat } = request {
            self.ui_focus = Some(id);
            self.notification_ui = Some(bboxy450_term::focus_ack::Ask {
                id,
                seat: SeatId::from_u128(seat.0),
                prior_spawns,
            });
            if let Err(reason) = self.recording_click(seat) {
                self.notification_ui = None;
                self.ui_focus = None;
                self.answer_ui(id, Err(reason));
                return false;
            }
            self.request_redraw();
            return true;
        }
        if let Err(reason) = self.solve_ui(logical) {
            self.answer_ui(id, Err(reason));
            return false;
        }
        let previous_ops: std::collections::BTreeSet<_> = self
            .operation_backlog
            .iter()
            .filter_map(|message| match message {
                live::UiMsg::Messaging { id, .. }
                | live::UiMsg::Task { id, .. }
                | live::UiMsg::SetAttention { id, .. }
                | live::UiMsg::GetAttention { id, .. }
                | live::UiMsg::PluginControl { id, .. }
                | live::UiMsg::KnowledgeControl { id, .. } => Some(*id),
                _ => None,
            })
            .collect();
        let preference_revision = self.preferences.revision;
        self.ui_collect = Some(Vec::new());
        let result = self.apply_ui(id, request);
        let mut effects = self.ui_collect.take().expect("UI command collection");
        effects.extend(
            self.operation_backlog
                .iter()
                .filter_map(|message| match message {
                    live::UiMsg::Messaging { id, .. }
                    | live::UiMsg::Task { id, .. }
                    | live::UiMsg::SetAttention { id, .. }
                    | live::UiMsg::GetAttention { id, .. }
                    | live::UiMsg::PluginControl { id, .. }
                    | live::UiMsg::KnowledgeControl { id, .. }
                        if !previous_ops.contains(id) =>
                    {
                        Some(UiEffect::Operation(*id))
                    }
                    _ => None,
                }),
        );
        let spawn = match result {
            Ok(spawn) => spawn,
            Err(reason) => {
                self.answer_ui(id, Err(reason));
                return true;
            }
        };
        if self.document.open {
            effects.push(UiEffect::Document);
        }
        match &self.local_tab {
            Some(LocalTab::Graph(_)) => effects.push(UiEffect::Graph),
            Some(LocalTab::Settings { .. }) => effects.push(UiEffect::Preferences {
                saved: self.preferences.revision != preference_revision,
            }),
            None => {}
        }
        let spawn = spawn.or_else(|| {
            effects.iter().find_map(|effect| match effect {
                UiEffect::Spawn(op) => self.starting_capture(*op),
                _ => None,
            })
        });
        self.await_ui_frame(id, effects, submitted, spawn);
        self.request_redraw();
        true
    }
    /// Close relays use the same lifecycle effect and produced-frame waiter as UI keys.
    fn begin_close(
        &mut self,
        id: bbt_mux::Ulid,
        seat: SeatId,
        undo: bool,
        logical: [f32; 2],
        submitted: u64,
    ) {
        self.cancel_ui("ui_frame_refused: a newer UI operation superseded this request");
        let sources = self.drain();
        self.dispatch_wake_sources(&sources);
        if self.closing || self.live_commands.is_none() || self.live_state.is_none() {
            self.answer_ui(id, Err("ui_not_ready: no live command sender".into()));
            return;
        }
        if let Err(reason) = self.solve_ui(logical) {
            self.answer_ui(id, Err(reason));
            return;
        }
        self.ui_observe = true;
        self.ui_collect = Some(Vec::new());
        self.close_seat(seat, id, undo);
        let effects = self.ui_collect.take().expect("close command collection");
        self.await_ui_frame(id, effects, submitted, None);
    }
    fn await_ui_frame(
        &mut self,
        id: bbt_mux::Ulid,
        effects: Vec<UiEffect>,
        submitted: u64,
        spawn: Option<UiSpawnCapture>,
    ) {
        let proxy = self.proxy.clone();
        let timer = bbt_rt::handle().spawn(async move {
            tokio::time::sleep(bboxy450_term::focus_ack::DEADLINE).await;
            let _ = proxy.send_event(UiEvent::UiDeadline(id));
        });
        self.ui_wait = Some(UiWait {
            id,
            after: None,
            visible: None,
            effects,
            spawn,
            timer: timer.abort_handle(),
        });
        self.poll_ui_frame(submitted);
    }
    fn ui_deadline(&mut self, id: bbt_mux::Ulid) {
        if self.ui_wait.as_ref().is_some_and(|wait| wait.id == id) {
            let wait = self.ui_wait.take().expect("UI deadline matched");
            let effects: Vec<_> = wait
                .effects
                .iter()
                .map(|effect| format!("{effect:?}={:?}", self.ui_effect_ready(effect)))
                .collect();
            self.answer_ui(
                id,
                Err(format!("timeout: UI frame incomplete; target={:?}, last={}, pending={}, effects={effects:?}",
                    wait.after, self.ui_frame.as_ref().map_or(0, |(frame, _)| *frame), self.pending_commands.len())),
            );
        }
    }
    fn apply_ui(
        &mut self,
        id: bbt_mux::Ulid,
        request: bbt_proto::gui::UiRequest,
    ) -> Result<Option<UiSpawnCapture>, String> {
        use bbt_proto::gui::{UiRequest, UiTarget};
        match request {
            UiRequest::Compose(request) => self.compose_ui(request)?,
            UiRequest::Spawn(pick) => {
                let model = self.live_state.as_deref().ok_or("ui_not_ready: no model")?;
                let stage = self.focus.stage.as_ref().unwrap_or(&model.ui.window.stage);
                let captured = bboxy450_term::launcher::operator_placement(
                    *pick,
                    model,
                    stage,
                    self.scene.layout_revision(),
                )?;
                let op = bbt_mux::Ulid::new();
                if self.document.open {
                    self.close_document();
                }
                if self.local_tab.is_some() {
                    self.close_local_tab();
                }
                if self.launcher.is_some() {
                    self.close_launcher();
                }
                self.queue(live::UiMsg::SpawnSpec {
                    id: op,
                    spec: Box::new(captured.spec.clone()),
                });
                return Ok(Some(UiSpawnCapture {
                    op,
                    swarm: captured.spec.swarm,
                    placement: captured
                        .spec
                        .placement
                        .clone()
                        .expect("explicit captured placement"),
                    layout_revision: captured.layout_revision,
                    seat: None,
                    seat_row: None,
                }));
            }
            UiRequest::Clock(command) => {
                if !self.test_account {
                    return Err(
                        "test_account_required: UI clock control requires a test-account owner"
                            .into(),
                    );
                }
                let frame = self.clock.apply(command)?;
                if let Some(effects) = &mut self.ui_collect {
                    effects.push(UiEffect::Clock {
                        state: frame.state,
                        landed: self.animation_landing,
                    });
                }
                if let Some(pacer) = &self.pacer {
                    pacer.set_active(
                        !self.clock.is_frozen()
                            && !self.occluded
                            && (self.animate || self.scene.animating(self.clock.now())),
                    )?;
                }
            }
            UiRequest::Resize { cols, rows } => {
                let empty = bboxy450_term::model::Model::default();
                let model = self.live_state.as_deref().unwrap_or(&empty);
                let scale = self.window.as_ref().map_or(1., |w| w.scale_factor() as f32);
                let target = cell_grid_size(model, cols, rows, scale);
                if self.window.is_some() {
                    self.pending_resize =
                        Some(LogicalSize::new(f64::from(target[0]), f64::from(target[1])));
                } else {
                    self.requested_size = Some(target);
                }
                if let Some(effects) = &mut self.ui_collect {
                    effects.push(UiEffect::Size(target));
                }
            }
            UiRequest::Open(UiTarget::Launcher) => self.open_launcher(),
            UiRequest::Open(UiTarget::Graph) => self.open_graph(),
            UiRequest::Open(UiTarget::Settings) => self.open_settings("Account"),
            UiRequest::Open(UiTarget::Docs(path)) => self.open_document(path),
            UiRequest::Key(keys) => {
                for (applied, key) in keys.into_iter().enumerate() {
                    let input = chrome_input(&key)?;
                    let modes = self
                        .focus
                        .stage
                        .as_ref()
                        .and_then(|s| s.focused)
                        .and_then(|seat| {
                            self.live_state
                                .as_ref()?
                                .screens
                                .get(&seat)
                                .map(|s| s.modes)
                        })
                        .unwrap_or_default();
                    let routed = self.route_chrome(&input, modes);
                    if matches!(
                        routed,
                        RoutedKey::Terminal(_)
                            | RoutedKey::Scroll(_)
                            | RoutedKey::Local
                            | RoutedKey::Quit
                    ) {
                        return Err(format!(
                            "ui_key_not_applicable: {key} after {applied} earlier chrome keys; terminal input uses seat key"
                        ));
                    }
                    if self.dispatch_chrome(routed).is_some() {
                        return Err("ui_key_not_applicable".into());
                    }
                }
            }
            UiRequest::Scroll { seat, motion } => {
                let seat = SeatId::from_u128(seat.0);
                if self.local_tab.is_some()
                    || self.document.open
                    || self.launcher.is_some()
                    || !self.pane_bounds.iter().any(|(id, _)| *id == seat)
                {
                    return Err("not_on_screen: requested scroll pane is not visible".into());
                }
                let screen = self
                    .live_state
                    .as_ref()
                    .and_then(|model| model.screens.get(&seat))
                    .ok_or("history_view_unavailable")?;
                let page = i64::try_from(screen.rows.saturating_sub(1).max(1))
                    .map_err(|_| "history_dimensions_unsupported")?;
                let request = match motion {
                    bbt_proto::gui::UiScroll::Top => ScrollRequest::top(),
                    bbt_proto::gui::UiScroll::Bottom => ScrollRequest::live(),
                    bbt_proto::gui::UiScroll::PageUp => ScrollRequest::lines(page),
                    bbt_proto::gui::UiScroll::PageDown => ScrollRequest::lines(-page),
                    bbt_proto::gui::UiScroll::Row(row) => ScrollRequest::row(row),
                };
                self.scroll_pane(seat, request.correlated(id));
            }
            UiRequest::Focus { .. }
            | UiRequest::SpawnGate(_)
            | UiRequest::NotificationClick { .. }
            | UiRequest::Seats
            | UiRequest::Layout => {
                return Err("UI request uses the frame acknowledgment path".into());
            }
        }
        Ok(None)
    }
    fn ui_layout(
        &self,
        size: [f32; 2],
        layouts: &[bbt_ui::components::Layout],
    ) -> Result<bbt_proto::gui::UiLayout, String> {
        use bbt_proto::gui::{UiLayout, UiPane};
        let model = self.live_state.as_ref().ok_or("ui_not_ready: no model")?;
        let stage = self.focus.stage.as_ref().unwrap_or(&model.ui.window.stage);
        let scale = self.window.as_ref().map_or(1., |w| w.scale_factor() as f32);
        let (_, sizes) = pane_controls(
            model,
            &self.pane_bounds,
            &self.watched,
            &BTreeMap::new(),
            stage.focused,
            scale,
            self.verify.grid_size,
        );
        let mut terminal_text = 16 * 1024;
        let panes = self
            .pane_bounds
            .iter()
            .filter_map(|(seat, rect)| {
                let (_, (_, cols, rows)) = sizes.iter().find(|(id, _)| id == seat)?;
                Some(UiPane {
                    seat: bbt_mux::Ulid::from(seat.0.0),
                    rect: rect_array(*rect),
                    cols: *cols,
                    rows: *rows,
                    focused: stage.focused == Some(*seat),
                    scroll: model
                        .scroll
                        .position(*seat, model.history.get(seat).map(Arc::as_ref)),
                    rendered: model.prepared_screens.get(seat).and_then(|source| {
                        self.frame_observations
                            .read(*seat, source, &mut terminal_text)
                            .map(|mut rendered| {
                                rendered.pixel_size =
                                    model.frames.get(seat).map(|grid| [grid.width, grid.height]);
                                rendered
                            })
                    }),
                })
            })
            .collect();
        let mut roots: Vec<_> = layouts.iter().collect();
        if let Some(page) = &self.local_page_layout {
            roots.push(&page.layout);
        }
        if let Some(dock) = &self.conversation_layout {
            roots.push(&dock.layout);
        }
        if let Some(launcher) = &self.launcher_layout {
            roots.push(launcher.solved());
        }
        if let Some(document) = &self.document_layout {
            roots.push(&document.layout);
        }
        let mut remaining = 4096;
        let mut remaining_text = 16 * 1024;
        let tree = roots
            .into_iter()
            .map(|l| ui_layout_node(l, &mut remaining, &mut remaining_text))
            .collect::<Result<_, _>>()?;
        let starting_stage = self.scene.starting_stage();
        let (swarm, tab, focused) = starting_stage.map_or(
            (stage.swarm, stage.tab, stage.focused),
            |(swarm, tab, focused)| {
                (
                    swarm,
                    tab,
                    focused.and_then(|slot| match slot {
                        bboxy450_term::starting::Slot::Seat(seat) => Some(seat),
                        bboxy450_term::starting::Slot::Launch(_) => None,
                    }),
                )
            },
        );
        let layout = UiLayout {
            frame_seq: 0, // filled only by the actual produced-frame callback
            layout_revision: Some(self.scene.layout_revision()),
            clock: Some(self.clock_frame.state),
            size,
            swarm: swarm.map(|id| bbt_mux::Ulid::from(id.0.0)),
            tab: tab.map(|id| bbt_mux::Ulid::from(id.0.0)),
            focused: focused.map(|id| bbt_mux::Ulid::from(id.0.0)),
            launcher_open: self.launcher.is_some(),
            page: self.local_tab.as_ref().map(|tab| {
                match tab {
                    LocalTab::Graph(_) => "graph",
                    LocalTab::Settings { .. } => "settings",
                }
                .to_owned()
            }),
            document: self.document.open.then(|| self.document.target.clone()),
            panes,
            tree,
            spawned: None,
            starting: self.starting_layout(layouts),
            seats: None,
            composed: self.ui_wait.as_ref().and_then(|wait| {
                wait.effects.iter().find_map(|effect| {
                    if let UiEffect::Compose(sent) = effect {
                        self.composed_frame(sent)
                    } else {
                        None
                    }
                })
            }),
        };
        if serde_json::to_vec(&layout)
            .map_err(|e| e.to_string())?
            .len()
            > 128 * 1024
        {
            return Err("ui_layout_oversize: solved layout exceeds 128 KiB".into());
        }
        Ok(layout)
    }
    fn solve_ui(&mut self, logical: [f32; 2]) -> Result<Vec<bbt_ui::components::Layout>, String> {
        self.clock_frame = self.clock.frame();
        self.solved_size = logical;
        let sending = self.ui_wait.as_ref().is_some_and(|wait| {
            wait.effects
                .iter()
                .any(|effect| matches!(effect, UiEffect::Compose(_)))
        });
        self.conversation_layout = if self.live_state.is_some() || self.compose_startup {
            Some(self.scene.conversation_layout(
                logical,
                &composer::Conversation {
                    stream: self.conversation.clone(),
                    composer: self.composer.view.clone(),
                    scroll: if sending {
                        f32::MAX
                    } else {
                        self.conversation_scroll
                    },
                },
            )?)
        } else {
            None
        };
        if sending {
            self.conversation_scroll = self
                .conversation_layout
                .as_ref()
                .and_then(|layout| {
                    bbt_ui::modules::stream_dock::Regions::scroll_extent(
                        &layout.layout,
                        "conversation/rows",
                    )
                })
                .map_or(0., |extent| extent[1]);
        }
        let dock = self.conversation_layout.as_ref().map_or(0., |l| l.height);
        let layouts = if self.local_tab.is_some() {
            self.scene
                .local_page_chrome_at(logical, dock, self.clock_frame.now)?
        } else {
            self.scene.layout_docked_at(
                logical,
                self.animation_start.is_some() || self.animate,
                dock,
                self.clock_frame.now,
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
                    let custody_notice = (controller.section == "Account")
                        .then(|| self.custody.notice())
                        .flatten();
                    let issue = self
                        .preferences
                        .error
                        .as_deref()
                        .or(controller.error.as_deref())
                        .or(custody_notice);
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
        self.close_regions = self.scene.close_regions(&layouts);
        self.starting_close_regions = self.scene.starting_close_regions(&layouts);
        self.focus_bounds = self.scene.focus_bounds(&layouts);
        self.pane_bounds = bounds;
        self.launcher_layout = self
            .launcher
            .as_ref()
            .map(|launcher| self.scene.launcher_layout(logical, &launcher.launcher))
            .transpose()?;
        if self.document.open {
            let context = self.scene.ui_context();
            if self
                .document_layout
                .as_ref()
                .is_none_or(|layout| !layout.matches(logical, context))
            {
                let layout = self.scene.document_layout(logical, &self.document)?;
                self.document.scroll.scroll([0.; 2], layout.regions.extent);
                self.document_layout = Some(layout);
            }
        } else {
            self.document_layout = None;
        }
        if let Some(model) = &self.live_state
            && model.metadata_initialized
        {
            let stage = self.focus.stage.as_ref().unwrap_or(&model.ui.window.stage);
            let state = bboxy450_term::window_state::capture(
                model,
                stage,
                logical,
                self.window_last.as_ref(),
            );
            if self.window_last.as_ref() != Some(&state) {
                self.window_last = Some(state.clone());
                self.queue(live::UiMsg::WindowState(state));
            }
        }
        Ok(layouts)
    }
    /// Data and focus may change between ticks; only a usable frame projects
    /// their newest state. A refused drawable leaves the pending work intact.
    fn project_scene(&mut self, frame_ready: bool) {
        if !frame_ready {
            return;
        }
        if self.scene_dirty {
            let Some(model) = &self.live_state else {
                return;
            };
            self.scene.set_model_with_clock(
                model,
                self.focus.stage.as_ref().unwrap_or(&model.ui.window.stage),
                &self.clock,
            );
            self.scene.set_starting(
                &self.starting,
                model,
                self.focus.stage.as_ref().unwrap_or(&model.ui.window.stage),
            );
            self.scene_dirty = false;
            if let Some(stall) = &self.stall {
                stall.model_applied();
            }
        }
        if self.scene.advance_restarts(self.clock.now()) {
            self.refresh_conversation();
        }
    }
    // Layout/watch work drained the current model; a later IO publication must wake us.
    // This retires no produced frame and keeps the bootstrap gate intact.
    fn window_data_wait(&self) {
        self.updates.complete_drain();
        self.live_updates.complete_drain();
    }
    fn window_restore_waits(&self) -> bool {
        self.live_state.as_ref().is_some_and(|model| {
            self.pane_bounds.iter().any(|(id, _)| {
                model.window_waiting.contains(id)
                    || (self.window_first_frame && !model.frames.contains_key(id))
            })
        })
    }
    fn complete_paint_sources(&mut self) {
        self.window_first_frame = false;
        self.updates.complete_drain();
        self.live_updates.complete_drain();
    }
    fn redraw(&mut self) -> Result<(), String> {
        // Delivery retires only the queued callback. Paint remains pending
        // through a missing tick/drawable until a frame is actually drawn.
        self.redraw_queued.set(false);
        if self.gpu.is_none() {
            // A producer may wake us before GPU preparation finishes. Drain
            // admission even on an empty frame, or GPU-ready can lose its wake.
            let sources = self.inbox.dirty.drain();
            self.wake_counters.borrow_mut().drained(&sources);
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
                self.inbox.dirty.source(display_link::TICK_SOURCE),
                self.pacing,
            )?;
            pacer.set_active(self.animate && !self.occluded && !self.clock.is_frozen())?;
            self.pacer = Some(pacer);
            self.request_redraw();
        }
        let mut phases = Phases::default();
        let start = Instant::now();
        let projections_before =
            bbt_rt::diagnostics::calls(bbt_rt::diagnostics::Phase::UiProjection);
        let sources = self.drain();
        // A native redraw can consume data before its queued Wake arrives.
        // If no drawable is pending, preserve that invalidation before the
        // later empty Wake is ignored.
        if self
            .pacer
            .as_ref()
            .is_some_and(|pacer| pacer.is_metal() && !pacer.is_active())
            && wake_action(&sources, false) == WakeAction::Invalidate
        {
            self.request_redraw_sources(&sources);
        }
        // Drain the coalesced dirty flag before consuming its latest tick, so a
        // concurrent CV callback cannot publish a tick without a future wake.
        let mut tick = self.pacer.as_ref().and_then(DisplayLink::take_tick);
        let fifo = self.pacer.as_ref().is_none_or(DisplayLink::is_fifo);
        let metal = self.pacer.as_ref().is_some_and(DisplayLink::is_metal);
        if self.occluded
            || (metal && tick.is_none())
            || (self.animate && self.animation_start.is_some() && !fifo && tick.is_none())
        {
            if metal && tick.is_none() {
                self.wake_counters.borrow_mut().tickless();
                bbt_rt::diagnostics::span(bbt_rt::diagnostics::Phase::TicklessRedraw).finish(
                    bbt_rt::diagnostics::calls(bbt_rt::diagnostics::Phase::UiProjection)
                        .unwrap_or(0)
                        .saturating_sub(projections_before.unwrap_or(0)),
                    0,
                );
            }
            return Ok(());
        }
        let frame = self
            .gpu
            .as_mut()
            .unwrap()
            .acquire(tick.as_mut().and_then(|tick| tick.drawable.take()))?;
        let project_start = self.stall.as_ref().map(|_| Instant::now());
        self.project_scene(frame.is_some());
        let project_scene = project_start.map(|start| start.elapsed());
        let Some(frame) = frame else {
            if (self.animate && fifo)
                || !self.inbox.messages.is_empty()
                || !self.inbox.output.is_empty()
            {
                self.request_redraw();
            }
            return Ok(());
        };
        phases.drain = start.elapsed().saturating_sub(frame.wait());
        let scale = self.window.as_ref().unwrap().scale_factor() as f32;
        let logical = [size.width as f32 / scale, size.height as f32 / scale];
        let start = Instant::now();
        let layouts = self.solve_ui(logical)?;
        let bounds = self.pane_bounds.clone();
        self.focus_layout_ready();
        self.fit_verify_grid(&bounds, scale);
        self.watch_panes(&bounds, scale);
        if self.window_restore_waits() {
            self.window_data_wait();
            return Ok(());
        }
        if let Some(target) = self.restored_size {
            if (logical[0] - target[0]).abs() > 1. || (logical[1] - target[1]).abs() > 1. {
                return Ok(());
            }
            self.restored_size = None;
        }
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
            ops.extend(self.scene.foreground_with_document(
                logical,
                self.launcher_layout.as_ref(),
                overlay.as_ref(),
                self.conversation_layout.as_ref(),
                self.document_layout.as_ref(),
            ));
        }
        let mut pane_overlay = if let Some(model) = &self.live_state {
            let mut ops = self
                .scene
                .history_foreground(model, &self.pane_bounds, scale)?;
            ops.extend(self.scene.foreground_with_document(
                logical,
                self.launcher_layout.as_ref(),
                overlay.as_ref(),
                self.conversation_layout.as_ref(),
                self.document_layout.as_ref(),
            ));
            ops
        } else {
            vec![]
        };
        if live { pane_overlay.extend(self.scene.error_overlay(logical, self.unread_errors)); } else { ops.extend(self.scene.error_overlay(logical, self.unread_errors)); }
        phases.draw = start.elapsed();
        if let Some((encode, wait)) = self.gpu.as_mut().unwrap().draw(
            ops,
            scale,
            self.clock_frame.time_ms,
            frame,
            self.meter.total_frames() + 1,
            self.live_state.as_ref().map(|model| gpu::Panes {
                model,
                bounds: &self.pane_bounds,
                overlay: pane_overlay,
            }),
        )? {
            self.redraw_pending.set(false);
            self.complete_paint_sources();
            self.wake_counters.borrow_mut().frame();
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
                    stall::FramePhases {
                        raw: phases,
                        project_scene,
                    },
                    cpu,
                    scale,
                );
            }
            let stall = &mut self.stall;
            self.gpu
                .as_ref()
                .unwrap()
                .collect_gpu(&mut self.budget, |sample| {
                    if let Some(stall) = stall {
                        stall.gpu_execution(sample.frame, sample.time);
                        stall.gpu_complete(sample.frame, sample.completed_at);
                    }
                });
            if !self.animate || self.clock.is_frozen() {
                // Idle frames pause the link, except while a swell or attention loop runs.
                let swelling = !self.clock.is_frozen()
                    && self.scene.animating(self.clock_frame.now)
                    && !self.occluded;
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
                    self.pacer
                        .as_mut()
                        .unwrap()
                        .set_active(!self.occluded && !self.clock.is_frozen())?;
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
                && (!self.document.open || self.document.settled())
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
            self.starting_produced(self.meter.total_frames(), &layouts);
            if self.ui_observe {
                if let Some(model) = &self.live_state {
                    self.frame_observations.produced(
                        self.meter.total_frames(),
                        model,
                        self.pane_bounds.iter().map(|(seat, _)| *seat),
                    );
                }
                let layout = self.ui_layout(logical, &layouts);
                self.present_ui(self.meter.total_frames(), layout);
            }
            if !self.app_ready_sent
                && !self.backend_lost
                && self.live_state.is_some()
                && self.app_generation > 0
                && std::env::args().any(|arg| arg == "--app-ready")
            {
                self.app_ready_sent = true;
                let mut status = app_status(
                    "window",
                    std::env::args().any(|a| {
                        a == "--unattended"
                            || a == "--test-account"
                            || a.starts_with("--test-account=")
                    }),
                    true,
                    true,
                    self.live_state.as_deref().unwrap(),
                    self.app_generation,
                    self.meter.total_frames(),
                    1,
                );
                status.custody_backend = self.app_custody.clone();
                write_app_ready(status);
            }
            self.poll_focus();
            for ask in std::mem::take(&mut self.ctl_captures) {
                let frame = bbt_proto::gui::CaptureFrame {
                    stale: false,
                    occluded: false,
                    presented_at_ms: unix_ms(self.last_presented),
                    frame_seq: self.meter.total_frames(),
                    ..Default::default()
                };
                self.capture_for(ask, scale, frame);
            }
            if self.fake && self.last_update == 20 && !self.idle_scheduled {
                self.idle_scheduled = true;
                self.schedule_idle_check();
            }
        }
        if (self.animate && fifo)
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
        let _callback = bbt_rt::diagnostics::callback(bbt_rt::diagnostics::Phase::AboutToWait);
        self.apply_notification_click();
        let pending = self.custody.wake_at();
        if let Some(message) = self.custody.next(Instant::now(), !self.backend_lost) {
            self.queue(message);
        }
        if pending.is_some() && self.custody.wake_at().is_none() {
            self.pending_commands
                .retain(|message| !matches!(message, live::UiMsg::IdentityControl { .. }));
            self.local_page_layout = None;
            self.request_redraw();
        }
        event_loop.set_control_flow(
            self.pending_notification
                .map(|(_, at)| at + Duration::from_secs(30))
                .into_iter()
                .chain(self.custody.wake_at())
                .min()
                .map_or(ControlFlow::Wait, ControlFlow::WaitUntil),
        );
        if let Some(size) = self.pending_resize.take()
            && let Some(window) = &self.window
        {
            let _ = window.request_inner_size(size);
        }
    }
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let _callback = bbt_rt::diagnostics::callback(bbt_rt::diagnostics::Phase::Resumed);
        event_loop.set_control_flow(ControlFlow::Wait);
        if self.window.is_none()
            && let Err(error) = self.init(event_loop)
        {
            self.fail(error, event_loop);
        }
    }
    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: UiEvent) {
        let _callback = bbt_rt::diagnostics::callback(match &event {
            UiEvent::Wake => bbt_rt::diagnostics::Phase::UserWake,
            UiEvent::StallPoll => bbt_rt::diagnostics::Phase::UserStallPoll,
            _ => bbt_rt::diagnostics::Phase::UserOther,
        });
        match event {
            UiEvent::ErrorCount(count) => {
                if self.unread_errors != count { self.unread_errors = count; self.request_redraw(); }
            }
            UiEvent::CommandsReady(ready) => self.commands_ready(ready),
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
                    gpu.collect_gpu(&mut self.budget, |sample| {
                        if let Some(stall) = stall {
                            stall.gpu_execution(sample.frame, sample.time);
                            stall.gpu_complete(sample.frame, sample.completed_at);
                        }
                    });
                }
                if let Some(stall) = &mut self.stall {
                    let wakes = self.wake_counters.borrow().snapshot().with_pending(
                        self.redraw_pending.get(),
                        !self.inbox.messages.is_empty(),
                        !self.inbox.output.is_empty(),
                    );
                    let result = stall.poll_with_wakes(
                        self.meter.total_frames(),
                        self.pacer.as_ref().map_or(0, DisplayLink::callbacks),
                        wakes,
                        self.maximum_hz,
                    );
                    if let Some(line) = stall.settle_line(!matches!(&result, Ok(None))) {
                        self.logger.write(line);
                    }
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
            UiEvent::CloseFor { id, seat, undo } => {
                if let Some(window) = &self.window {
                    let scale = window.scale_factor() as f32;
                    let size = window.inner_size();
                    let logical = [size.width as f32 / scale, size.height as f32 / scale];
                    self.begin_close(id, seat, undo, logical, self.meter.total_frames());
                } else {
                    self.answer_ui(id, Err("endpoint_unavailable: window is not ready".into()));
                }
            }
            UiEvent::FocusFor(ask) => {
                self.begin_focus(ask);
                self.drain();
                self.request_redraw();
            }
            UiEvent::UiFor {
                id,
                request,
                prior_spawns,
            } => {
                let size = self
                    .window
                    .as_ref()
                    .map(|window| {
                        let scale = window.scale_factor() as f32;
                        let size = window.inner_size();
                        [size.width as f32 / scale, size.height as f32 / scale]
                    })
                    .ok_or_else(|| "endpoint_unavailable: window is not ready".to_owned());
                match size {
                    Ok(size) => {
                        self.begin_ui(id, request, size, self.meter.total_frames(), prior_spawns);
                    }
                    Err(reason) => self.answer_ui(id, Err(reason)),
                }
            }
            UiEvent::UiDeadline(id) => self.ui_deadline(id),
            UiEvent::StartingDeadline(op) => self.starting_deadline(op),
            UiEvent::FocusDeadline(id) => {
                if let Some(answer) = self.ctl_focus.expire(id) {
                    self.answer_focus(answer);
                }
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
            UiEvent::App(live::AppEvent::Ask { id, request }) => {
                if request == bbt_proto::gui::AppRequest::Activate
                    && let Some(window) = &self.window
                {
                    window.set_minimized(false);
                    window.focus_window();
                }
                let empty = bboxy450_term::model::Model::default();
                let mut status = app_status(
                    "window",
                    std::env::args().any(|a| {
                        a == "--unattended"
                            || a == "--test-account"
                            || a.starts_with("--test-account=")
                    }),
                    !self.backend_lost && self.live_state.is_some(),
                    self.meter.total_frames() > 0,
                    self.live_state.as_deref().unwrap_or(&empty),
                    self.app_generation,
                    self.meter.total_frames(),
                    usize::from(self.window.is_some()),
                );
                if request == bbt_proto::gui::AppRequest::Notifications {
                    status.notifications = self.recording_status();
                }
                self.queue(if request == bbt_proto::gui::AppRequest::HistoryMetrics {
                    live::UiMsg::HistoryMetrics { id, status }
                } else {
                    live::UiMsg::AppReply { id, status }
                });
            }
            UiEvent::App(live::AppEvent::Stop { id }) => {
                self.logger
                    .write(format!("app stop {id}: orderly Session shutdown"));
                self.request_close(event_loop);
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
                let sources = self.inbox.dirty.drain();
                self.wake_counters.borrow_mut().wake(&sources);
                if self.smoke && self.gpu.is_none() {
                    self.smoke_wake_seen = true;
                }
                if let Err(error) = self.install_gpu() {
                    self.fail(error, event_loop);
                    return;
                }
                self.dispatch_wake_sources(&sources);
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
                        .collect_gpu(&mut self.budget, |_| {});
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
                    self.finish_log(text);
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
                    self.finish_log(report);
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
                self.command_retry.cancel();
                self.custody.next(Instant::now(), false);
                self.pending_commands
                    .retain(|message| !matches!(message, live::UiMsg::IdentityControl { .. }));
                self.scene.backend_lost();
                self.request_redraw();
            }
            UiEvent::BackendReady => {
                self.backend_lost = false;
                self.scene.backend_ready();
                self.conversation_reads(false);
                self.task_reads();
                self.flush_commands();
                self.request_redraw();
            }
            UiEvent::BackendInfo {
                generation,
                custody_backend,
            } => {
                self.app_generation = generation;
                self.app_custody = custody_backend;
            }
        }
    }
    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        let _callback = stall::window_callback(&event);
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
                    && let Err(error) =
                        pacer.set_active(self.animate && !occluded && !self.clock.is_frozen())
                {
                    self.fail(error, event_loop);
                    return;
                }
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
                if self.document.open {
                    return;
                }
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
            WindowEvent::MouseWheel { delta, .. } if self.document.open => {
                let [mut dx, mut dy] = match delta {
                    MouseScrollDelta::LineDelta(x, y) => [-x * 28., -y * 28.],
                    MouseScrollDelta::PixelDelta(p) => {
                        let scale = self.window.as_ref().map_or(1., |w| w.scale_factor() as f32);
                        [-p.x as f32 / scale, -p.y as f32 / scale]
                    }
                };
                if self.modifiers.shift_key() && dx == 0. {
                    dx = dy;
                    dy = 0.;
                }
                if let Some(layout) = &self.document_layout {
                    let horizontal = layout.regions.horizontal_at(self.pointer).cloned();
                    let extent = layout.regions.extent;
                    let root_dx = if horizontal.as_ref().is_some_and(|r| r.root) {
                        dx
                    } else {
                        0.
                    };
                    let mut changed = self.document.scroll.scroll([root_dx, dy], extent);
                    if let Some(region) = horizontal.filter(|r| !r.root) {
                        changed |= self.document.scroll.pan(&region.key, dx, region.max);
                    }
                    if changed {
                        self.document_layout = None;
                        self.request_redraw();
                    }
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
                    self.task_reads();
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
                if self.document.open {
                    use bbt_ui::modules::document_modal::controller::Action;
                    let action = self
                        .document_layout
                        .as_ref()
                        .and_then(|l| l.regions.hit(self.pointer));
                    match action {
                        Some(Action::Open(target)) => self.open_document(target),
                        Some(Action::Close) => self.close_document(),
                        Some(Action::Unavailable(name)) => {
                            self.document.notice = Some(format!("{name} is not available yet"));
                            self.document_layout = None;
                            self.request_redraw();
                        }
                        None => {}
                    }
                    return;
                }
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
                    if hit.is_none()
                        && let Some(bbt_ui::modules::stream_dock::Action::OpenRecord(target)) =
                            layout.actions.hit(self.pointer)
                    {
                        self.open_document(target);
                        return;
                    }
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
                        Hit::Attention(rule) => Action::Attention(rule),
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
                if let Some((op, _)) =
                    self.starting_close_regions
                        .iter()
                        .copied()
                        .find(|(_, rect)| {
                            self.pointer[0] >= rect.x
                                && self.pointer[0] < rect.right()
                                && self.pointer[1] >= rect.y
                                && self.pointer[1] < rect.bottom()
                        })
                {
                    self.dismiss_starting(op);
                    return;
                }
                if let Some((seat, _, undo)) =
                    self.close_regions.iter().copied().find(|(_, rect, _)| {
                        self.pointer[0] >= rect.x
                            && self.pointer[0] < rect.right()
                            && self.pointer[1] >= rect.y
                            && self.pointer[1] < rect.bottom()
                    })
                {
                    self.close_seat(seat, bbt_mux::Ulid::new(), undo);
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
            WindowEvent::Ime(_) if self.document.open => {}
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
                if event.state == ElementState::Pressed {
                    self.cancel_ui(
                        "ui_frame_refused: physical keyboard input superseded this request",
                    );
                }
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
                    && keyboard::shortcut_key(&input) == Some('r')
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
                    && keyboard::shortcut_key(&input) == Some('s')
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
                let routed = if self.document.open
                    || (self.launcher.is_none()
                        && (self.local_tab.is_some()
                            || self.composer.view.focus
                            || self.reject_focus))
                {
                    let routed = self.route_chrome(&input, modes.unwrap_or_default());
                    finish_route(&mut self.keyboard, &input, routed)
                } else {
                    // The live PTY boundary keeps physical repeat/release
                    // suppression and the focused Screen's actual modes.
                    route_input(
                        &mut self.keyboard,
                        &input,
                        modes.unwrap_or_default(),
                        self.launcher
                            .as_ref()
                            .map(|launcher| launcher.launcher.query.as_str()),
                    )
                };
                match self.dispatch_chrome(routed) {
                    None => {}
                    Some(RoutedKey::Quit) => self.request_close(event_loop),
                    Some(RoutedKey::Scroll(motion)) => {
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
                    Some(RoutedKey::Terminal(bytes)) => {
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
                    Some(RoutedKey::Local) => {}
                    Some(_) => unreachable!("chrome routed above"),
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
        live::UiMsg::Messaging { .. }
            | live::UiMsg::Task { .. }
            | live::UiMsg::GetAttention { .. }
            | live::UiMsg::SetAttention { .. }
            | live::UiMsg::Close(_)
            | live::UiMsg::UndoClose { .. }
            | live::UiMsg::PluginControl { .. }
            | live::UiMsg::KnowledgeControl { .. }
            | live::UiMsg::IdentityControl { .. }
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
fn admit_reads(
    sender: Option<&tokio::sync::mpsc::Sender<live::UiMsg>>,
    backlog: &mut VecDeque<live::UiMsg>,
    reads: impl FnOnce() -> Vec<live::UiMsg>,
) {
    // Calling the controllers consumes their demand and assigns pending ids.
    // A fixture with no IO owner must leave that demand untouched.
    if sender.is_some() {
        backlog.extend(reads());
    }
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
  --headless           run the app and renderer without opening a window
  --ctl                enable the operator's ctl endpoint

Debug and verification only, never for normal use:
  --launcher           open the launcher at startup (same controller as Cmd+T)
  --document <record>  open a record via Knowledge (debug capture only)
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

fn diagnostic_root(args: &[String]) -> Result<std::path::PathBuf, String> {
    if let Some(path) = args.windows(2).find(|p| p[0] == "--run-dir").map(|p| &p[1]) {
        return Ok(path.into());
    }
    if let Some(path) = std::env::var_os("BBOXY_RUN_DIR") { return Ok(path.into()); }
    if let Some(account) = bboxy450_term::cockpit::Config::test_account_from_args(args).map_err(|e| e.to_string())? { return Ok(account.join("run")); }
    bbt_mux::transport::default_socket_path().map_err(|e| e.to_string())?.parent().map(std::path::Path::to_path_buf).ok_or("default diagnostics directory unavailable".into())
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let hook_args: Vec<_> = std::env::args().skip(1).collect();
    let diagnostics_root = diagnostic_root(&hook_args)?;
    bbt_rt::error_journal::configure(&diagnostics_root);
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
    // `ctl --help` and `e2e --help` are their own usages, printed by them.
    if hook_args
        .first()
        .is_none_or(|arg| arg != "ctl" && arg != "e2e")
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
    if hook_args.first().is_some_and(|arg| arg == "harness-exec") {
        return bbt_mux::adapters::launch::exec_when_ready(&hook_args[1..])
            .map_err(|error| error as Box<dyn std::error::Error>);
    }
    if hook_args.first().is_some_and(|arg| arg == "hook") {
        bbt_mux::adapters::hook::run_cli(&hook_args[1..]);
        return Ok(());
    }
    if hook_args.first().is_some_and(|arg| arg == "mcp") {
        bbt_mux::mcp::run_cli().inspect_err(|e| { bbt_rt::error_journal::report(bbt_rt::error_journal::Failure::new("mcp", "bridge_exit", e)); })?;
        return Ok(());
    }
    if hook_args.first().is_some_and(|arg| arg == "e2e") {
        // The unattended e2e runner (#282): its own windowless app, ctl scripts as verdicts.
        match bboxy450_term::e2e_night::main(&hook_args[1..]) {
            Ok(green) => std::process::exit(bboxy450_term::e2e_night::exit_code(green)),
            Err(reason) => {
                eprintln!("{reason}");
                std::process::exit(2);
            }
        }
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
    bboxy450_term::cockpit::Config::test_account_from_args(&hook_args)
        .map_err(|error| error as Box<dyn std::error::Error>)?;
    let record_notifications = notification_recording(&hook_args)?;
    let verify = Verify::from_args(&hook_args)?;
    if hook_args.iter().any(|arg| arg == "--headless") {
        return headless::run(&hook_args).map_err(Into::into);
    }
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
    let diagnostics_proxy = proxy.clone();
    let _diagnostics = bbt_rt::error_journal::Server::start(diagnostics_root, move |n| { let _ = diagnostics_proxy.send_event(UiEvent::ErrorCount(n)); }).ok();
    let inbox = bbt_rt::init_ui(Wake::new(
        UiProxy::Native(proxy.clone()),
        bbt_rt::handle().clone(),
    ));
    let prepared = Arc::new(Latest::new(inbox.dirty.source(2)));
    let updates = Arc::new(Latest::new(inbox.dirty.paint_source(3)));
    let live_updates = Arc::new(Latest::new(inbox.dirty.paint_source(5)));
    let smoke = std::env::args().any(|arg| arg == "--smoke");
    if smoke {
        let timeout_proxy = proxy.clone();
        bbt_rt::handle().spawn(async move {
            tokio::time::sleep(Duration::from_secs(30)).await;
            let _ = timeout_proxy.send_event(UiEvent::SmokeDeadline);
        });
    }
    let fake = smoke || std::env::args().any(|arg| arg == "--fake-source");
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
    let logger = log::Logger::new();
    // The delegate is installed before run_app finishes application launch.
    // Authorization and native completion waits run only on the bounded worker.
    let callback_proxy = proxy.clone();
    let notifications = if record_notifications {
        None
    } else {
        match bboxy450_desktop_notifications::open(Box::new(move |response| {
            let _ = callback_proxy.send_event(UiEvent::Notification(response));
        })) {
            Ok(notifier) => {
                let progress = proxy.clone();
                match bboxy450_term::notifications::Worker::start(
                    notifier,
                    Arc::new(move |event| {
                        let _ = progress.send_event(match event {
                            bboxy450_term::notifications::Event::Ready => {
                                UiEvent::NotificationReady
                            }
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
    let mut app = App::owner(
        UiProxy::Native(proxy),
        inbox,
        logger,
        live_updates,
        &args,
        verify,
    );
    app.prepared = prepared;
    app.updates = updates;
    app.live_commands = live_commands;
    app.notifications = notifications;
    if record_notifications {
        app.install_recording()?;
    }
    app.preferences = preference_state;
    app.preferences_io = preferences_io;
    app.pacing = pacing;
    app.render_context = (!fake && !animate).then_some(render_context);
    app.stall = stall;
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
mod lazy_projection_tests {
    use super::*;
    use bbt_ui_model::{Axis, Seat, SplitNode, Swarm, SwarmId, Tab, TabId};

    fn published_model(revision: usize) -> Arc<bboxy450_term::model::Model> {
        let id = SeatId::from_u128(1);
        let swarm = SwarmId::from_u128(2);
        let mut seat = Seat::new(id, swarm, [1; 32], "fixture", "zsh");
        seat.title = format!("revision {revision}");
        Arc::new(bboxy450_term::model::Model {
            ui: UiModel::new(
                vec![Swarm::new(
                    swarm,
                    "fixture",
                    vec![Tab::new(
                        TabId::from_u128(3),
                        "fixture",
                        SplitNode::split(
                            Axis::Row,
                            0.5,
                            SplitNode::Leaf(id),
                            SplitNode::Leaf(SeatId::from_u128(4)),
                        ),
                    )],
                )],
                vec![
                    seat,
                    Seat::new(SeatId::from_u128(4), swarm, [4; 32], "other", "zsh"),
                ],
            )
            .unwrap(),
            ..Default::default()
        })
    }

    #[allow(
        clippy::disallowed_methods,
        reason = "the fixture owns an isolated process-wide UI thread registration"
    )]
    fn isolated_ui_case(name: &str) -> bool {
        const CHILD: &str = "BBOXY_R5J_UI_FIXTURE_CHILD";
        if std::env::var_os(CHILD).is_none() {
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", name, "--nocapture", "--test-threads=1"])
                .env(CHILD, "1")
                .output()
                .unwrap();
            assert!(
                output.status.success()
                    && String::from_utf8_lossy(&output.stdout).contains("1 passed; 0 failed;"),
                "isolated UI fixture {:?}:\n{}\n{}",
                output.status,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            return true;
        }
        false
    }
    fn fixture() -> (App, Arc<Latest<Arc<bboxy450_term::model::Model>>>) {
        struct QuietWake;
        impl bbt_rt::Waker for QuietWake {
            fn wake(&self) {}
        }
        let inbox = bbt_rt::init_ui(QuietWake);
        let latest = Arc::new(Latest::new(inbox.dirty.paint_source(5)));
        let app = App::owner(
            UiProxy::Headless(Arc::new(|_| {})),
            inbox,
            log::Logger::new(),
            latest.clone(),
            &["--headless".into()],
            Verify::default(),
        );
        (app, latest)
    }
    // keeps: decision #344; a deferred first frame cannot suppress history-ready wakes
    #[test]
    fn a_restore_wait_rearms_model_wakes_without_claiming_a_frame() {
        if isolated_ui_case(
            "lazy_projection_tests::a_restore_wait_rearms_model_wakes_without_claiming_a_frame",
        ) {
            return;
        }
        let (mut app, latest) = fixture();
        app.window_first_frame = true;
        latest.publish(published_model(1));
        assert!(app.drain().contains(&5));
        app.window_data_wait();
        latest.publish(published_model(2));
        assert!(app.drain().contains(&5), "history-ready data lost its wake");
        assert!(
            app.window_first_frame,
            "a wait must not retire the first-frame gate"
        );
        assert_eq!(app.meter.total_frames(), 0);
        assert_eq!(app.scene.projections(), 0);
    }
    #[test]
    fn model_publishes_between_frames_do_not_project_during_drain() {
        if isolated_ui_case(
            "lazy_projection_tests::model_publishes_between_frames_do_not_project_during_drain",
        ) {
            return;
        }
        let (mut app, latest) = fixture();
        for revision in 0..64 {
            let model = published_model(revision);
            latest.publish(model.clone());
            let sources = app.drain();
            assert!(sources.contains(&5), "the data invalidation must survive");
            assert!(Arc::ptr_eq(app.live_state.as_ref().unwrap(), &model));
            assert_eq!(
                app.scene.projections(),
                0,
                "drain projected without a frame"
            );
            app.project_scene(false);
            assert_eq!(app.scene.projections(), 0, "a missing drawable projected");
        }
        app.project_scene(true);
        app.complete_paint_sources();
        assert_eq!(app.scene.projections(), 1);
        assert_eq!(app.scene.seats[0].title, "revision 63");
        for revision in 64..128 {
            let model = published_model(revision);
            latest.publish(model.clone());
            app.drain();
            assert!(Arc::ptr_eq(app.live_state.as_ref().unwrap(), &model));
            app.project_scene(false);
            assert_eq!(app.scene.projections(), 1);
        }
        app.project_scene(true);
        app.complete_paint_sources();
        assert_eq!(
            app.scene.projections(),
            2,
            "N publishes added one projection"
        );
        assert_eq!(app.scene.seats[0].title, "revision 127");
        app.project_scene(true);
        app.complete_paint_sources();
        assert_eq!(
            app.scene.projections(),
            2,
            "unchanged state is already projected"
        );

        for id in [4, 1, 4] {
            app.focus_action_applied(FocusAction::Activate(
                SeatId::from_u128(id),
                FocusOrigin::Keyboard,
            ));
        }
        assert_eq!(
            app.focus.stage.as_ref().unwrap().focused,
            Some(SeatId::from_u128(4))
        );
        assert_eq!(
            app.scene.projections(),
            2,
            "focus changes only mark pending work"
        );
        app.project_scene(false);
        assert_eq!(app.scene.projections(), 2);
        app.project_scene(true);
        app.complete_paint_sources();
        assert_eq!(app.scene.projections(), 3);
        assert_eq!(app.scene.focused(), Some(SeatId::from_u128(4)));

        let target = SeatId::from_u128(1);
        app.ctl_focus.set(bboxy450_term::focus_ack::Ask {
            id: bbt_mux::Ulid(9),
            seat: target,
            prior_spawns: vec![],
        });
        assert!(app.poll_focus_frame(10, 11));
        assert_eq!(app.focus.stage.as_ref().unwrap().focused, Some(target));
        assert_eq!(app.scene.focused(), Some(SeatId::from_u128(4)));
        let model = app.live_state.clone().unwrap();
        assert!(
            app.ctl_focus
                .poll(&model, app.scene.focused(), |_| true, 10)
                .is_none(),
            "accepted focus still waits for its produced frame"
        );
        app.project_scene(false);
        assert_eq!(app.scene.projections(), 3);
        app.project_scene(true);
        app.complete_paint_sources();
        assert_eq!(app.scene.projections(), 4);
        let Some(bboxy450_term::focus_ack::Effect::Answer(answer)) =
            app.ctl_focus
                .poll(&model, app.scene.focused(), |_| true, 11)
        else {
            panic!("the projected completed frame must acknowledge focus");
        };
        let frame = answer.result.unwrap();
        assert_eq!(frame.seat, target);
        assert_eq!(frame.frame_seq, 11);
    }

    #[test]
    fn one_thousand_publications_between_two_ticks_schedule_at_most_two_callbacks() {
        if isolated_ui_case(
            "lazy_projection_tests::one_thousand_publications_between_two_ticks_schedule_at_most_two_callbacks",
        ) {
            return;
        }
        let (mut app, latest) = fixture();
        let first = published_model(0);
        latest.publish(first);
        app.drain();
        app.project_scene(true);
        app.complete_paint_sources();
        assert_eq!(app.scene.projections(), 1);
        let mut callbacks = 0;
        for revision in 1..=1000 {
            latest.publish(published_model(revision));
            let sources = app.inbox.dirty.drain();
            app.dispatch_wake_sources(&sources);
            if revision == 1 {
                assert!(app.redraw_queued.get());
                callbacks += 1;
                // Deliver the first callback before a tick. Native redraw
                // retires its queue entry, but must retain the paint demand.
                app.redraw().unwrap();
                assert!(!app.redraw_queued.get());
                app.drain();
                app.project_scene(false);
            } else {
                assert!(
                    !app.redraw_queued.get(),
                    "publication {revision} queued another callback"
                );
            }
            assert!(
                app.redraw_pending.get(),
                "tickless return retired paint demand"
            );
            assert_eq!(app.scene.projections(), 1);
        }
        // The tick and final model notification may share one dirty-set wake.
        app.dispatch_wake_sources(&[display_link::TICK_SOURCE, 5]);
        assert!(app.redraw_queued.get());
        app.dispatch_wake_sources(&[display_link::TICK_SOURCE, 5]);
        assert!(app.redraw_queued.get());
        app.redraw().unwrap();
        assert!(!app.redraw_queued.get());
        callbacks += 1;
        app.drain();
        app.project_scene(true);
        app.complete_paint_sources();
        assert_eq!(app.scene.seats[0].title, "revision 1000");
        assert_eq!(app.scene.projections(), 2);
        assert_eq!(callbacks, 2);

        app.redraw_pending.set(false);
        for _ in 0..1000 {
            app.inbox
                .sender
                .try_send(UiMsg::Changed { source: 6 })
                .unwrap();
            app.inbox.dirty.source(1).mark();
            let sources = app.inbox.dirty.drain();
            app.dispatch_wake_sources(&sources);
            assert!(
                !app.redraw_queued.get(),
                "a non-paint notice queued a frame"
            );
            assert!(!app.redraw_pending.get());
        }
        assert!(app.inbox.messages.is_empty());
        // GPU preparation can follow an early, GPU-unready callback.
        app.redraw_pending.set(true);
        app.dispatch_wake_sources(&[2]);
        assert!(
            app.redraw_queued.get(),
            "GPU readiness was stranded behind paint demand"
        );
        // The unattended smoke deliberately holds preparation until its first
        // pre-GPU callback has handled a producer notice.
        app.redraw_queued.set(false);
        app.smoke_wake_seen = true;
        let (open, mut ready) = tokio::sync::oneshot::channel();
        app.smoke_bootstrap = Some(open);
        app.dispatch_wake_sources(&[0]);
        assert!(app.redraw_queued.get());
        app.redraw().unwrap();
        assert_eq!(ready.try_recv(), Ok(()));
    }
}

#[cfg(test)]
mod verification_tests {
    use super::*;

    #[test]
    fn no_sender_model_retains_read_demand_without_undeliverable_commands() {
        let scope = bbt_proto::SwarmId::from_u128(42);
        let model = bboxy450_term::model::Model {
            ui: UiModel::new(
                vec![bbt_ui_model::Swarm::new(scope, "fixture", vec![])],
                vec![],
            )
            .unwrap(),
            ..Default::default()
        };
        let mut history = messaging::History::default();
        let mut records = messaging::Records::default();
        let mut backlog = VecDeque::new();
        for _ in 0..120 {
            let scope = model.ui.window.sidebar.swarms[0].id;
            history.select(scope);
            records.select(scope);
            admit_reads(None, &mut backlog, || {
                let mut reads: Vec<_> = history
                    .actions()
                    .into_iter()
                    .map(live::UiMsg::from)
                    .collect();
                reads.extend(history.commands());
                reads.extend(records.commands());
                reads
            });
            assert!(
                backlog.is_empty(),
                "a fixture without IO has no deliverable reads"
            );
        }
        // Admission resumes from the original demand, without a fake pending
        // operation/cursor left behind by the sender-less publications.
        let (sender, _receiver) = tokio::sync::mpsc::channel(1);
        admit_reads(Some(&sender), &mut backlog, || {
            let mut reads: Vec<_> = history
                .actions()
                .into_iter()
                .map(live::UiMsg::from)
                .collect();
            reads.extend(history.commands());
            reads.extend(records.commands());
            reads
        });
        assert_eq!(backlog.len(), 7); // cursor, page, message list, four record reads
        assert!(matches!(backlog[0], live::UiMsg::EventCursor { .. }));
        assert!(matches!(
            backlog[1],
            live::UiMsg::EventPage { after: 0, .. }
        ));
        assert_eq!(history.commands().len(), 0);
        assert_eq!(records.commands().len(), 0);
    }

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

    #[test]
    fn produced_text_budget_omits_whole_runs_and_keeps_empty_distinct() {
        use bbt_ui::components::{Layout, LayoutKind, PositionedRun};
        let rect = bbt_ui_model::layout::Rect::new(0., 0., 200., 30.);
        let style = bbt_ui::Style::new(bbt_ui::Preset::Card)
            .resolve(bbt_ui::Context::new(Theme::Dark, bbt_ui::Density::Cozy))
            .unwrap()
            .text;
        let text = |value: &str| Layout {
            rect,
            baseline: 20.,
            kind: LayoutKind::Text {
                runs: vec![PositionedRun {
                    text: value.into(),
                    style: style.clone(),
                    rect,
                    line_height: 30.,
                    baseline: 20.,
                }],
                selectable: false,
                ellipsis: false,
                shadow: None,
            },
            children: vec![],
        };
        let layout = Layout {
            rect,
            baseline: 20.,
            kind: LayoutKind::Stack,
            children: vec![text("界界"), text("é"), text("x"), text("")],
        };
        let (mut nodes, mut bytes) = (5, 3);
        let observed = ui_layout_node(&layout, &mut nodes, &mut bytes).unwrap();
        assert_eq!(nodes, 0);
        assert_eq!(bytes, 0);
        assert_eq!(observed.children[0].text, None);
        assert_eq!(observed.children[1].text.as_deref(), Some("é"));
        assert_eq!(observed.children[2].text.as_deref(), Some("x"));
        assert_eq!(observed.children[3].text.as_deref(), Some(""));
    }

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
                live::UiMsg::SpawnZsh
                | live::UiMsg::Spawn { .. }
                | live::UiMsg::SpawnSpec { .. } => spawns += 1,
                live::UiMsg::ConsumeSpawn { .. }
                | live::UiMsg::Scroll { .. }
                | live::UiMsg::DocumentRead { .. }
                | live::UiMsg::ConsumeDocument { .. } => {}
                live::UiMsg::Close(_)
                | live::UiMsg::UndoClose { .. }
                | live::UiMsg::Messaging { .. }
                | live::UiMsg::GetAttention { .. }
                | live::UiMsg::SetAttention { .. }
                | live::UiMsg::PluginControl { .. }
                | live::UiMsg::KnowledgeControl { .. }
                | live::UiMsg::IdentityControl { .. }
                | live::UiMsg::Task { .. }
                | live::UiMsg::EventPage { .. }
                | live::UiMsg::EventCursor { .. }
                | live::UiMsg::AttentionFocus { .. }
                | live::UiMsg::MarkEventsRead { .. }
                | live::UiMsg::Focused { .. }
                | live::UiMsg::UiReply { .. }
                | live::UiMsg::WindowState(..)
                | live::UiMsg::AppReply { .. }
                | live::UiMsg::HistoryMetrics { .. }
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
mod layout_shortcut_tests {
    use super::*;

    fn character(
        logical: &'static str,
        unmodified: &str,
        physical: KeyCode,
        native_text: Option<&'static str>,
        modifiers: ModifiersState,
    ) -> keyboard::Input<'static> {
        let mut input = test_key(
            &Key::Character(logical.into()),
            PhysicalKey::Code(physical),
            Some(logical),
            modifiers,
            false,
        );
        input.unmodified_key = Key::Character(unmodified.into());
        input.text_all_modifiers = native_text;
        input
    }

    #[test]
    fn programmer_dvorak_command_t_uses_the_layout_not_the_us_position() {
        let command_t = character("t", "t", KeyCode::KeyK, Some("t"), ModifiersState::SUPER);
        assert_eq!(route_key(&command_t, 0, None), RoutedKey::OpenLauncher);
        let physical_t = character("y", "y", KeyCode::KeyT, Some("y"), ModifiersState::SUPER);
        assert_eq!(route_key(&physical_t, 0, None), RoutedKey::Local);
    }

    #[test]
    fn character_shortcuts_and_chrome_edits_use_the_unmodified_layout() {
        for (logical, base, expected) in [
            ("Q", "q", RoutedKey::Quit),
            ("g", "g", RoutedKey::OpenGraph),
            (",", ",", RoutedKey::OpenSettings),
            ("z", "z", RoutedKey::UndoClose),
            ("m", "m", RoutedKey::MuteFocused),
        ] {
            let key = character(
                logical,
                base,
                KeyCode::KeyJ,
                Some(logical),
                ModifiersState::SUPER,
            );
            assert_eq!(route_key(&key, 0, None), expected, "{base}");
        }
        let hud = character(
            "ƒ",
            "f",
            KeyCode::KeyY,
            Some("ƒ"),
            ModifiersState::SUPER | ModifiersState::ALT,
        );
        assert_eq!(route_key(&hud, 0, None), RoutedKey::ToggleHud);
        let close = character("w", "w", KeyCode::Comma, Some("w"), ModifiersState::SUPER);
        assert_eq!(
            route_local_page(&close, 0),
            RoutedKey::LocalPage(PageCommand::Close)
        );
        let reload = character("r", "r", KeyCode::KeyO, Some("r"), ModifiersState::SUPER);
        assert_eq!(
            route_settings(&reload, 0, false),
            RoutedKey::SettingsKey(SettingsKey::Reload)
        );
        let select = character(
            "å",
            "a",
            KeyCode::KeyJ,
            Some("å"),
            ModifiersState::SUPER | ModifiersState::ALT,
        );
        assert_eq!(
            route_key(&select, 0, Some("query")),
            RoutedKey::Launcher(LauncherCommand::SelectAll)
        );
        assert_eq!(
            route_composer(&select, 0, ComposeField::Body),
            RoutedKey::Composer(ComposeCommand::Edit(EditCommand::SelectAll))
        );
        assert_eq!(
            route_key(&reload, 0, Some("query")),
            RoutedKey::Launcher(LauncherCommand::Reroll)
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn russian_command_shortcuts_use_the_native_ascii_command_translation() {
        // ЙЦУКЕН: physical T is е without modifiers; macOS Command translates it to t.
        let command_t = character("е", "е", KeyCode::KeyT, Some("t"), ModifiersState::SUPER);
        assert_eq!(route_key(&command_t, 0, None), RoutedKey::OpenLauncher);
        // The same native fallback reaches editing commands, not just global chords.
        let select = character("ф", "ф", KeyCode::KeyA, Some("a"), ModifiersState::SUPER);
        assert_eq!(
            route_composer(&select, 0, ComposeField::Body),
            RoutedKey::Composer(ComposeCommand::Edit(EditCommand::SelectAll))
        );
        let plain = character("е", "е", KeyCode::KeyT, Some("е"), ModifiersState::empty());
        assert_eq!(
            route_key(&plain, 0, None),
            RoutedKey::Terminal("е".as_bytes().to_vec())
        );
    }

    #[test]
    fn typing_and_chrome_text_keep_layout_resolved_unicode() {
        for (text, base, physical) in [
            ("y", "y", KeyCode::KeyT),
            ("Y", "y", KeyCode::KeyT),
            ("ж", "ж", KeyCode::Semicolon),
        ] {
            let input = character(text, base, physical, Some(text), ModifiersState::empty());
            assert_eq!(
                route_key(&input, 0, None),
                RoutedKey::Terminal(text.as_bytes().to_vec())
            );
            assert_eq!(
                route_key(&input, 0, Some("query")),
                RoutedKey::Launcher(LauncherCommand::Insert(text.into()))
            );
            assert_eq!(
                route_composer(&input, 0, ComposeField::Body),
                RoutedKey::Composer(ComposeCommand::Edit(EditCommand::Insert(text.into())))
            );
            assert_eq!(
                route_settings(&input, 0, true),
                RoutedKey::SettingsKey(SettingsKey::Edit(EditCommand::Insert(text.into())))
            );
        }
    }

    #[test]
    fn layout_shortcut_suppresses_its_original_physical_repeat_and_release() {
        let mut input = character("t", "t", KeyCode::KeyK, Some("t"), ModifiersState::SUPER);
        let mut state = keyboard::State::default();
        assert_eq!(
            route_input(&mut state, &input, 0, None),
            RoutedKey::OpenLauncher
        );
        input.modifiers.state = ModifiersState::empty();
        input.repeat = true;
        assert_eq!(route_input(&mut state, &input, 0, None), RoutedKey::Local);
        input.repeat = false;
        input.state = ElementState::Released;
        assert_eq!(route_input(&mut state, &input, 0, None), RoutedKey::Local);
        input.state = ElementState::Pressed;
        assert_eq!(
            route_input(&mut state, &input, 0, None),
            RoutedKey::Terminal(b"t".to_vec())
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
    fn document_return_anchor_survives_refresh_and_reconciles_a_removed_seat() {
        let model = fixture();
        let mut focus = FocusState::default();
        focus.reconcile(None, &model);
        focus
            .apply(
                &model,
                FocusAction::Activate(SeatId::from_u128(2), FocusOrigin::Row),
                &[],
                Instant::now(),
            )
            .unwrap();
        let anchor = focus.stage.clone().unwrap();
        let mut next = model.clone();
        next.seats.get_mut(&SeatId::from_u128(2)).unwrap().title = "updated while reading".into();
        focus.restore(anchor.clone(), &next);
        assert_eq!(
            focus.stage.as_ref().unwrap().focused,
            Some(SeatId::from_u128(2))
        );
        next.apply(bbt_ui_model::event::Update::SeatClosedByDaemon {
            seat: SeatId::from_u128(2),
        })
        .unwrap();
        focus.restore(anchor, &next);
        let focused = focus.stage.as_ref().unwrap().focused;
        assert_ne!(focused, Some(SeatId::from_u128(2)));
        assert!(focused.is_some_and(|id| next.seats.contains_key(&id)));
    }
    #[test]
    fn document_keys_never_navigate_the_covered_pane_and_balance_prior_pty_releases() {
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
        let routed = route_document(&letter, 3 << 18);
        assert!(matches!(
            finish_route(&mut state, &letter, routed),
            RoutedKey::Terminal(_)
        ));
        letter.state = ElementState::Pressed;
        let routed = route_document(&letter, 3 << 18);
        assert_eq!(finish_route(&mut state, &letter, routed), RoutedKey::Local);
        letter.state = ElementState::Released;
        let routed = route_document(&letter, 3 << 18);
        assert_eq!(finish_route(&mut state, &letter, routed), RoutedKey::Local);
        let navigation = test_key(
            &Key::Named(NamedKey::ArrowLeft),
            PhysicalKey::Code(KeyCode::ArrowLeft),
            None,
            ModifiersState::SUPER | ModifiersState::ALT,
            false,
        );
        assert_eq!(route_document(&navigation, 0), RoutedKey::Local);
        let down = test_key(
            &Key::Named(NamedKey::PageDown),
            PhysicalKey::Code(KeyCode::PageDown),
            None,
            ModifiersState::empty(),
            false,
        );
        assert_eq!(
            route_document(&down, 0),
            RoutedKey::LocalPage(PageCommand::PageDown)
        );
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
    fn captured() -> live::UiMsg {
        live::UiMsg::SpawnSpec {
            id: bbt_mux::Ulid::new(),
            spec: Box::new(bbt_proto::SpawnSpec {
                swarm: bbt_proto::SwarmId::from_u128(1),
                harness_kind: "zsh".into(),
                name: None,
                role: None,
                cwd: "/fixture".into(),
                location: bbt_proto::Where::Local,
                resume: None,
                placement: Some(bbt_proto::seat_state::Placement::NewTab {
                    tab: bbt_proto::TabId::from_u128(1),
                }),
            }),
        }
    }
    #[test]
    fn generic_and_legacy_spawns_share_the_same_cap() {
        let mut pending = VecDeque::new();
        for n in 0..PENDING_SPAWNS {
            queue_pending(
                &mut pending,
                match n % 3 {
                    0 => generic(),
                    1 => live::UiMsg::SpawnZsh,
                    _ => captured(),
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
        assert_eq!(
            queue_pending(&mut pending, captured()),
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
    #[test]
    fn t7_command_z_is_local_once_and_control_z_remains_terminal_input() {
        let logical = Key::Character("z".into());
        let press = test_key(
            &logical,
            PhysicalKey::Code(KeyCode::KeyZ),
            Some("z"),
            ModifiersState::SUPER,
            false,
        );
        let mut keyboard = keyboard::State::default();
        assert_eq!(
            route_input(&mut keyboard, &press, 3 << 18, None),
            RoutedKey::UndoClose
        );
        let mut repeat = press.clone();
        repeat.repeat = true;
        assert_eq!(
            route_input(&mut keyboard, &repeat, 3 << 18, None),
            RoutedKey::Local
        );
        let mut release = press.clone();
        release.state = ElementState::Released;
        assert_eq!(
            route_input(&mut keyboard, &release, 3 << 18, None),
            RoutedKey::Local
        );
        let control = test_key(
            &logical,
            PhysicalKey::Code(KeyCode::KeyZ),
            Some("z"),
            ModifiersState::CONTROL,
            false,
        );
        assert_eq!(route_key(&control, 0, None), RoutedKey::Terminal(vec![26]));
        assert_ne!(
            route_composer(&press, 0, ComposeField::Body),
            RoutedKey::UndoClose
        );
        assert_ne!(route_settings(&press, 0, true), RoutedKey::UndoClose);
        assert_ne!(route_key(&press, 0, Some("")), RoutedKey::UndoClose);
    }
}

#[cfg(test)]
mod ctl_ui_layout_tests {
    use super::*;

    // keeps: spec ctl-cli §1a (the visible Graph read stays inside its reply bound)
    #[test]
    fn a_full_graph_has_a_bounded_visible_layout_projection() {
        let scene = scene::Scene::new();
        let graph = e2e_checklist::Graph::new(vec![], vec![]).unwrap();
        let page = scene
            .graph_page_layout([1500., 900.], 240., &graph.checklist, 0.)
            .unwrap();
        assert!(page.max_scroll > 0.);
        let mut nodes = 4096;
        let mut text = 16 * 1024;
        let chrome = scene.local_page_chrome([1500., 900.], 240.).unwrap();
        let projected: Vec<_> = chrome
            .iter()
            .chain(std::iter::once(&page.layout))
            .map(|root| ui_layout_node(root, &mut nodes, &mut text).unwrap())
            .collect();
        let bytes = serde_json::to_vec(&projected).unwrap();
        eprintln!("Graph and chrome projection: {} bytes", bytes.len());
        assert!(
            bytes.len() <= 128 * 1024,
            "Graph layout exceeds its read bound: {} bytes",
            bytes.len()
        );
    }

    // keeps: ICD ctl-app #347 (visible scroll children and their ancestors)
    #[test]
    fn a_scroll_projection_keeps_visible_geometry_and_omits_offscreen_children() {
        use bbt_ui::components::{Layout, LayoutKind};
        let leaf = |name: &str, y| Layout {
            rect: Rect::new(10., y, 20., 10.),
            baseline: 0.,
            kind: LayoutKind::Named(name.into()),
            children: vec![],
        };
        let scroll = Layout {
            rect: Rect::new(0., 0., 100., 100.),
            baseline: 0.,
            kind: LayoutKind::ScrollView {
                content: [100., 300.],
                offset: [0., 0.],
            },
            children: vec![leaf("visible", 10.), leaf("offscreen", 200.)],
        };
        let mut nodes = 4096;
        let mut text = 16 * 1024;
        let projected = ui_layout_node(&scroll, &mut nodes, &mut text).unwrap();
        assert_eq!(projected.rect, [0., 0., 100., 100.]);
        assert_eq!(projected.children.len(), 1);
        assert_eq!(projected.children[0].kind, "named:visible");
        assert_eq!(projected.children[0].rect, [10., 10., 20., 10.]);
    }
}

#[cfg(test)]
mod ctl_ui_key_tests {
    use super::*;
    #[test]
    fn virtual_chords_leave_native_pressed_keys_and_releases_alone() {
        struct QuietWake;
        impl bbt_rt::Waker for QuietWake {
            fn wake(&self) {}
        }
        let inbox = bbt_rt::init_ui(QuietWake);
        let updates = Arc::new(Latest::new(inbox.dirty.paint_source(5)));
        let mut app = App::owner(
            UiProxy::Headless(Arc::new(|_| {})),
            inbox,
            log::Logger::new(),
            updates,
            &["--headless".into()],
            Verify::default(),
        );
        let key = PhysicalKey::Code(KeyCode::KeyT);
        let input = chrome_input("cmd-t").unwrap();
        // A real local press still owns its later release after a ctl chord.
        app.keyboard.consumed(key, ElementState::Pressed);
        let routed = app.route_chrome(&input, 0);
        assert!(app.dispatch_chrome(routed).is_none());
        assert!(app.keyboard.suppressed(key, ElementState::Released, false));
        app.close_launcher();
        // A ctl chord cannot suppress a release for an earlier PTY press.
        let routed = app.route_chrome(&input, 0);
        assert!(app.dispatch_chrome(routed).is_none());
        assert!(!app.keyboard.suppressed(key, ElementState::Released, false));
    }
    #[test]
    fn every_advertised_chrome_key_uses_the_native_input_boundary() {
        for key in bbt_proto::gui::UI_KEYS {
            let input = chrome_input(key).unwrap_or_else(|error| panic!("{key}: {error}"));
            assert_eq!(input.state, ElementState::Pressed);
            assert!(input.text.is_none());
        }
        assert_eq!(
            route_key(&chrome_input("cmd-t").unwrap(), 0, None),
            RoutedKey::OpenLauncher
        );
        assert_eq!(
            route_key(&chrome_input("cmd-g").unwrap(), 0, None),
            RoutedKey::OpenGraph
        );
        assert_eq!(
            route_key(&chrome_input("cmd-m").unwrap(), 0, None),
            RoutedKey::MuteFocused
        );
        assert!(matches!(
            route_key(&chrome_input("cmd-alt-tab").unwrap(), 0, None),
            RoutedKey::Pane(_)
        ));
    }
}
