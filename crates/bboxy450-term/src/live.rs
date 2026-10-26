//! GUI socket IO, immutable model publication and worker-only preparation.
use std::{
    collections::{BTreeMap, VecDeque},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

use crate::{UiEvent, log::Logger};
use bboxy450_term::{
    cockpit::{self, Session},
    documents,
    grids::Grids,
    knowledge,
    model::{self, Model, PanePreparer},
    scrollback::History,
};
use bbt_mux::{
    Ulid,
    store::{Result, invalid},
};
use bbt_proto::events::{EventGap, GuiEventPayload, GuiEventRecord};
use bbt_proto::{
    GuiAction, GuiIdentity, GuiReply, GuiSeat, ProcessStatus, SeatId, SpawnSpec, SwarmId, Update,
    View, Where,
};
use bbt_render::{cells::terminal, device::DeviceContext};
use bbt_rt::Latest;
use tokio::sync::{mpsc, oneshot};
use winit::event_loop::EventLoopProxy;

pub struct RenderContext {
    pub context: Arc<DeviceContext>,
    pub scale: f32,
    pub theme: bbt_ui_model::tokens::Theme,
}
pub use bboxy450_term::cockpit::UiMsg;
const COMMAND_CAPACITY: usize = 64;

/// Pure translation on the IO side. UI inputs never choose argv, cwd, role
/// authority or a process handle; the existing backend validates the spec.
fn spawn_spec(message: &UiMsg, swarm: Ulid, cwd: &Path) -> Option<SpawnSpec> {
    if let UiMsg::SpawnSpec { spec, .. } = message {
        return Some(spec.as_ref().clone());
    }
    let (harness_kind, name, role, location, selected_swarm, placement) = match message {
        UiMsg::Spawn {
            harness_kind,
            name,
            role,
            location,
            swarm,
            placement,
            ..
        } => (
            harness_kind.as_str(),
            name.clone(),
            role.clone(),
            location.clone(),
            *swarm,
            placement.clone(),
        ),
        UiMsg::SpawnZsh => ("zsh", None, None, Where::Local, None, None),
        _ => return None,
    };
    Some(SpawnSpec {
        swarm: selected_swarm.unwrap_or_else(|| SwarmId::from_u128(swarm.0)),
        harness_kind: harness_kind.into(),
        name,
        role,
        cwd: cwd.to_string_lossy().into_owned(),
        location,
        resume: None,
        placement,
    })
}

pub struct CommandSender {
    sender: mpsc::Sender<UiMsg>,
    knowledge: Arc<KnowledgeDemand>,
    context: tokio::sync::watch::Sender<Option<bboxy450_term::settings::ContextUpdate>>,
    // Dropping the UI owner wakes shutdown even when the queue and retained
    // retry slot are full. No polling timer and no UI wait are needed.
    _stop: oneshot::Sender<()>,
}
#[derive(Default)]
struct KnowledgeDemand {
    enabled: AtomicBool,
    notify: tokio::sync::Notify,
}
impl CommandSender {
    pub fn sender_clone(&self) -> mpsc::Sender<UiMsg> {
        self.sender.clone()
    }
    pub fn set_context(&self, context: bboxy450_term::settings::ContextUpdate) {
        self.context.send_replace(Some(context));
    }

    /// A desired observer state, independent of the bounded command queue.
    pub fn observe_knowledge(&self, enabled: bool) {
        if self.knowledge.enabled.swap(enabled, Ordering::AcqRel) != enabled {
            self.knowledge.notify.notify_one();
        }
    }
}
impl std::ops::Deref for CommandSender {
    type Target = mpsc::Sender<UiMsg>;
    fn deref(&self) -> &Self::Target {
        &self.sender
    }
}
struct Commands {
    messages: mpsc::Receiver<UiMsg>,
    stop: oneshot::Receiver<()>,
    knowledge: Arc<KnowledgeDemand>,
    context: tokio::sync::watch::Receiver<Option<bboxy450_term::settings::ContextUpdate>>,
}
fn command_channel() -> (CommandSender, Commands) {
    let (sender, messages) = mpsc::channel(COMMAND_CAPACITY);
    let (context, contexts) = tokio::sync::watch::channel(None);
    let (stop, stopped) = oneshot::channel();
    let knowledge = Arc::new(KnowledgeDemand::default());
    (
        CommandSender {
            sender,
            knowledge: knowledge.clone(),
            context,
            _stop: stop,
        },
        Commands {
            messages,
            stop: stopped,
            knowledge,
            context: contexts,
        },
    )
}

pub fn start(
    updates: Arc<Latest<Arc<Model>>>,
    proxy: EventLoopProxy<UiEvent>,
    logger: Logger,
    run_dir: Option<PathBuf>,
    ctl: bool,
    rendering: oneshot::Receiver<RenderContext>,
) -> CommandSender {
    start_with_sink(
        updates,
        Arc::new(move |event| {
            let _ = proxy.send_event(event);
        }),
        logger,
        run_dir,
        ctl,
        rendering,
    )
}

/// The same IO owner for a window or an offscreen GUI. Event delivery never
/// depends on an AppKit event loop or waits for the consumer to draw.
pub fn start_with_sink(
    updates: Arc<Latest<Arc<Model>>>,
    events: Arc<dyn Fn(UiEvent) + Send + Sync>,
    logger: Logger,
    run_dir: Option<PathBuf>,
    ctl: bool,
    rendering: oneshot::Receiver<RenderContext>,
) -> CommandSender {
    let (sender, commands) = command_channel();
    bbt_rt::handle().spawn(async move {
        let result = async {
            let mut config = cockpit::Config::discover(run_dir)?;
            config.ctl = ctl;
            let cwd = config.repository.clone();
            let session = Session::start(config).await?;
            events(UiEvent::BackendInfo {
                generation: session.endpoint.generation,
                custody_backend: session.endpoint.custody_backend.clone(),
            });
            events(UiEvent::BackendReady);
            run_with_control(
                updates,
                commands,
                logger.clone(),
                session,
                cwd,
                rendering,
                |reason| {
                    let event = reason.map_or(UiEvent::BackendReady, UiEvent::BackendLost);
                    events(event);
                },
                {
                    let events = events.clone();
                    move |ask| {
                        events(match ask {
                            OperatorAsk::Ui {
                                id,
                                request,
                                prior_spawns,
                            } => UiEvent::UiFor {
                                id,
                                request,
                                prior_spawns,
                            },
                            OperatorAsk::Capture(ask) => UiEvent::CaptureFor(ask),
                            OperatorAsk::Focus(ask) => UiEvent::FocusFor(ask),
                            OperatorAsk::Close { id, seat, undo } => {
                                UiEvent::CloseFor { id, seat, undo }
                            }
                        });
                    }
                },
                {
                    let events = events.clone();
                    move |ask| events(UiEvent::App(ask))
                },
            )
            .await
        }
        .await;
        if let Err(error) = &result {
            logger.write(format!("cockpit stopped: {error}"));
        }
        events(UiEvent::BackendStopped(result.map_err(|e| e.to_string())));
    });
    sender
}

/// What another operator door asks of the window, relayed by logic.
#[derive(Debug)]
pub enum OperatorAsk {
    Ui {
        id: Ulid,
        request: bbt_proto::gui::UiRequest,
        prior_spawns: Vec<Ulid>,
    },
    Capture(CaptureAsk),
    /// Focus this seat, as the window's own row click does (`ctl seat focus`).
    Focus(bboxy450_term::focus_ack::Ask),
    Close {
        id: Ulid,
        seat: SeatId,
        undo: bool,
    },
}
/// A capture logic relays from an operator door: the window writes the next presented
/// frame (cropped to `seat`'s pane when named) to `path` and answers with
/// [`UiMsg::Captured`].
#[derive(Debug)]
pub struct CaptureAsk {
    pub id: Ulid,
    pub path: PathBuf,
    pub seat: Option<SeatId>,
}

/// Lifecycle messages sent only by the admitted GUI relay.
#[derive(Debug)]
pub enum AppEvent {
    Ask {
        id: Ulid,
        request: bbt_proto::gui::AppRequest,
    },
    Stop {
        id: Ulid,
    },
}

fn publish(
    seats: &BTreeMap<Ulid, GuiSeat>,
    identity: &GuiIdentity,
    grids: &Grids,
    previous: &mut Arc<Model>,
    preparer: &mut PanePreparer,
    updates: &Latest<Arc<Model>>,
) -> Result<()> {
    let _measurement = bbt_rt::diagnostics::span(bbt_rt::diagnostics::Phase::ModelPublish);
    let seats: Vec<_> = seats.values().cloned().collect();
    let records: Vec<_> = seats.iter().map(|seat| seat.record.clone()).collect();
    let mut next = model::build(&records, grids, previous);
    next.apply_seat_states(&seats);
    next.identity = Some(identity.clone());
    preparer.prepare(&mut next)?;
    let next = Arc::new(next);
    {
        let _drop = bbt_rt::diagnostics::span(bbt_rt::diagnostics::Phase::IoOldModelDrop);
        *previous = next;
    }
    let _latest = bbt_rt::diagnostics::span(bbt_rt::diagnostics::Phase::LatestPublish);
    updates.publish(previous.clone());
    Ok(())
}

fn queue_action(outgoing: &mut VecDeque<GuiAction>, action: GuiAction) {
    if let GuiAction::Unwatch { seat } = &action
        && outgoing
            .iter()
            .rev()
            .find(|old| matches!(old, GuiAction::Watch { seat: old_seat, .. } | GuiAction::Unwatch { seat: old_seat } if old_seat == seat))
            .is_some_and(|old| matches!(old, GuiAction::Unwatch { .. }))
    {
        return;
    }
    if matches!(action, GuiAction::Applied { .. })
        && let Some(index) = outgoing
            .iter()
            .position(|old| matches!(old, GuiAction::GetHistory { .. }))
    {
        outgoing.insert(index, action);
        return;
    }
    outgoing.push_back(action);
}

fn scroll_readback(history: &mut History, model: &mut Arc<Model>, grids: &Grids) -> bool {
    let position = history.take_position();
    let reply = history.take_completion();
    if position.is_none() && reply.is_none() {
        return false;
    }
    let readback = Arc::make_mut(&mut Arc::make_mut(model).scroll);
    if let Some(located) = position {
        readback.displayed(
            located.seat,
            grids.baseline(located.seat),
            located.baseline,
            located.position,
        );
    }
    if let Some(reply) = reply {
        readback.reply = Some(reply);
    }
    true
}
fn scroll_refused(
    model: &mut Arc<Model>,
    seat: SeatId,
    generation: u64,
    request: bboxy450_term::scrollback::Request,
    reason: &str,
) -> bool {
    let Some(id) = request.op else { return false };
    Arc::make_mut(&mut Arc::make_mut(model).scroll).reply =
        Some(bboxy450_term::scrollback::Reply {
            id,
            seat,
            generation,
            result: Err(reason.chars().take(256).collect()),
        });
    true
}

fn return_live(history: &mut History, model: &mut Arc<Model>, seat: SeatId) -> bool {
    history.forget(seat);
    if model.scroll.observed(seat, None) {
        Arc::make_mut(&mut Arc::make_mut(model).scroll).forget(seat);
    }
    if model.history.contains_key(&seat) {
        Arc::make_mut(model).history.remove(&seat);
        true
    } else {
        false
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ActionReply {
    Operation,
    Seats,
    Hosts,
    History(SeatId),
    Messaging(Ulid),
    Task(Ulid),
    Identity(Ulid),
    Attention(Ulid),
    Knowledge(Ulid),
    Plugin(Ulid, bool),
    Events { stream: String },
    EventCursor,
}

/// Ordinary terminal results retain request order, including GetHistory's
/// operation-less Error. Knowledge completes independently by operation ID;
/// neither it nor state/grid pushes consume an ordinary request's slot.
#[derive(Default)]
struct ActionReplies(VecDeque<ActionReply>);
impl ActionReplies {
    fn full(&self) -> bool {
        self.0.len() >= 128
    }
    fn sent(&mut self, action: &GuiAction) {
        debug_assert!(!self.full());
        self.0.push_back(match action {
            GuiAction::ListSeats => ActionReply::Seats,
            GuiAction::ListHosts => ActionReply::Hosts,
            GuiAction::GetHistory { seat, .. } => ActionReply::History(*seat),
            GuiAction::Messaging { id, .. } => ActionReply::Messaging(*id),
            GuiAction::Task { id, .. } => ActionReply::Task(*id),
            GuiAction::GetAttention { id, .. } | GuiAction::SetAttention { id, .. } => {
                ActionReply::Attention(*id)
            }
            GuiAction::Identity(call) => ActionReply::Identity(call.id),
            GuiAction::Knowledge { id, .. } => ActionReply::Knowledge(*id),
            GuiAction::Plugin { id, request } => ActionReply::Plugin(*id, !request.mutation()),
            GuiAction::GetEvents { stream, .. } => ActionReply::Events {
                stream: stream.clone(),
            },
            GuiAction::GetEventCursor { .. } | GuiAction::MarkEventsRead { .. } => {
                ActionReply::EventCursor
            }
            _ => ActionReply::Operation,
        });
    }
    fn take(&mut self, reply: &GuiReply) -> Option<ActionReply> {
        let independent = match reply {
            GuiReply::Knowledge { op, .. } => self
                .0
                .iter()
                .position(|r| *r == ActionReply::Knowledge(*op)),
            GuiReply::Plugin { op, .. } => self
                .0
                .iter()
                .position(|r| matches!(r,ActionReply::Plugin(id,_) if id==op)),
            GuiReply::Error { op, .. } => self.0.iter().position(
                |r| matches!(r,ActionReply::Knowledge(id)|ActionReply::Plugin(id,_) if id==op),
            ),
            _ => None,
        };
        if let Some(at) = independent {
            return self.0.remove(at);
        }
        let at = self
            .0
            .iter()
            .position(|r| !matches!(r, ActionReply::Knowledge(_) | ActionReply::Plugin(..)))?;
        let expected = &self.0[at];
        let matches = match reply {
            // Close/undo return Seat as their terminal result. Mute sends its
            // committed Seat before Messaging; that snapshot cannot finish
            // the pending Messaging slot or a later ordinary operation.
            GuiReply::Ok { .. } | GuiReply::Spawned { .. } | GuiReply::Seat(_) => {
                *expected == ActionReply::Operation
            }
            GuiReply::Error { op, .. } => match expected {
                ActionReply::Messaging(id)
                | ActionReply::Task(id)
                | ActionReply::Knowledge(id)
                | ActionReply::Identity(id)
                | ActionReply::Attention(id) => op == id,
                _ => *op != Ulid::nil(),
            },
            GuiReply::Messaging { op, .. } => *expected == ActionReply::Messaging(*op),
            GuiReply::Task { op, .. } => *expected == ActionReply::Task(*op),
            GuiReply::Attention { op, .. } => *expected == ActionReply::Attention(*op),
            GuiReply::IdentityControl { op, .. } => *expected == ActionReply::Identity(*op),
            GuiReply::Knowledge { op, .. } => *expected == ActionReply::Knowledge(*op),
            GuiReply::Seats(_) => *expected == ActionReply::Seats,
            GuiReply::Hosts(_) => *expected == ActionReply::Hosts,
            GuiReply::History { seat, .. } => *expected == ActionReply::History(*seat),
            GuiReply::Events { page } => {
                matches!(expected, ActionReply::Events { stream } if *stream == page.cursor.stream)
            }
            GuiReply::EventCursor { .. } => *expected == ActionReply::EventCursor,
            _ => false,
        };
        if matches { self.0.remove(at) } else { None }
    }
    fn clear(&mut self) {
        self.0.clear();
    }
}

fn event_model(model: &mut Arc<Model>) -> &mut model::EventLane {
    Arc::make_mut(&mut Arc::make_mut(model).events)
}

fn publish_page(
    model: &mut Arc<Model>,
    updates: &Latest<Arc<Model>>,
    page: bbt_proto::events::GuiEventPage,
) -> Result<()> {
    if event_model(model).apply_page(page).map_err(invalid)? {
        updates.publish(model.clone());
    }
    Ok(())
}

fn publish_page_refusal(
    model: &mut Arc<Model>,
    updates: &Latest<Arc<Model>>,
    terminal: Option<&ActionReply>,
    reason: &str,
) -> Result<()> {
    if let Some(ActionReply::Events { stream }) = terminal
        && event_model(model)
            .page_refused(stream, reason)
            .map_err(invalid)?
    {
        updates.publish(model.clone());
    }
    Ok(())
}

fn publish_delivery(
    model: &mut Arc<Model>,
    updates: &Latest<Arc<Model>>,
    record: GuiEventRecord,
) -> Result<Option<GuiAction>> {
    let (stream, seq) = match &record {
        GuiEventRecord::Event(row) => (
            row.metadata.reference.position.stream.clone(),
            row.metadata.reference.position.sequence,
        ),
        GuiEventRecord::Gap(gap) => (gap.stream.clone(), gap.last),
    };
    let seat = match &record {
        GuiEventRecord::Event(row) => match &row.payload {
            GuiEventPayload::Seat { seat } => Some(seat.clone()),
            _ => None,
        },
        GuiEventRecord::Gap(_) => None,
    };
    if event_model(model).apply(record).map_err(invalid)? {
        if let Some(seat) = seat {
            Arc::make_mut(model).observe_attention(&seat, Instant::now());
        }
        updates.publish(model.clone());
    }
    Ok((seq != 0).then_some(GuiAction::EventAck { stream, seq }))
}

fn attach_events(model: &mut Arc<Model>, outgoing: &mut VecDeque<GuiAction>) {
    let id = Ulid::new();
    event_model(model).begin_attach(id);
    queue_action(outgoing, GuiAction::AttachEvents { id });
    // Reconcile state changed between the initial catalogue and Start::New.
    queue_action(outgoing, GuiAction::ListSeats);
    queue_action(outgoing, GuiAction::ListHosts);
}

fn disconnected_events(
    model: &mut Arc<Model>,
    outgoing: &mut VecDeque<GuiAction>,
    reason: &str,
) -> bool {
    let changed = event_model(model).disconnected(reason);
    // A new socket must bind its device lease before any retained ACK/query.
    outgoing.retain(|action| {
        !matches!(
            action,
            GuiAction::Spawn { .. }
                | GuiAction::AttachEvents { .. }
                | GuiAction::EventAck { .. }
                | GuiAction::GetEvents { .. }
                | GuiAction::GetEventCursor { .. }
                | GuiAction::MarkEventsRead { .. }
        )
    });
    changed
}

fn discard_uncertain_actions(outgoing: &mut VecDeque<GuiAction>) {
    outgoing.retain(|action| {
        !matches!(
            action,
            GuiAction::Spawn { .. }
                | GuiAction::Messaging { .. }
                | GuiAction::Task { .. }
                | GuiAction::Identity(_)
                | GuiAction::GetAttention { .. }
                | GuiAction::SetAttention { .. }
                | GuiAction::Knowledge { .. }
        )
    });
}

const CONTROL_RETRY: Duration = Duration::from_millis(100);

struct DesiredPane {
    view: Option<View>,
    size: Option<(u64, u16, u16)>,
    retry_at: Instant,
    retry: bool,
}

/// Resize and Watch are desired state. Their server-generated operation IDs
/// cannot identify a refused request, so reconcile against the actual Screen.
/// Keep one retry slot per visible pane and no timer once every pane converges.
#[derive(Default)]
struct PaneControls(BTreeMap<SeatId, DesiredPane>);
impl PaneControls {
    fn set_watched(
        &mut self,
        panes: &[(SeatId, View)],
        outgoing: &mut VecDeque<GuiAction>,
        now: Instant,
    ) {
        let panes: BTreeMap<_, _> = panes
            .iter()
            .copied()
            .filter(|(seat, view)| seat.0.0 == view.seat.0)
            .collect();
        self.0.retain(|seat, _| panes.contains_key(seat));
        for (seat, view) in panes {
            let pane = self.0.entry(seat).or_insert(DesiredPane {
                view: None,
                size: None,
                retry_at: now,
                retry: true,
            });
            if pane.view != Some(view) {
                outgoing.retain(|action| Self::seat(action) != Some(seat));
                pane.view = Some(view);
                pane.size = pane
                    .size
                    .filter(|&(generation, _, _)| generation == view.generation);
                pane.retry_at = now;
                pane.retry = true;
            }
        }
        // A queued control for an old view must not outlive an Unwatch or a
        // generation change. Input/Applied retain their existing queue order.
        outgoing.retain(|action| match action {
            GuiAction::Watch { seat, view } => {
                self.0.get(seat).is_some_and(|p| p.view == Some(*view))
            }
            GuiAction::Resize {
                seat, generation, ..
            } => self
                .0
                .get(seat)
                .is_some_and(|p| p.view.is_some_and(|view| view.generation == *generation)),
            _ => true,
        });
    }

    fn resize(
        &mut self,
        seat: SeatId,
        generation: u64,
        cols: u16,
        rows: u16,
        grids: &Grids,
        outgoing: &mut VecDeque<GuiAction>,
    ) {
        // UI coalescing can place a Resize before its Watched message. The
        // caller admits only known seats, bounding these early intents by the
        // seat set; the next Watched message prunes invisible panes.
        let pane = self.0.entry(seat).or_insert(DesiredPane {
            view: None,
            size: None,
            retry_at: Instant::now(),
            retry: false,
        });
        if pane.view.is_none_or(|view| view.generation <= generation) {
            // Use the same limits as logic; otherwise a clamped request would
            // remain different from its Screen and retry forever.
            let cols = cols.clamp(2, 512);
            pane.size = Some((generation, cols, rows.clamp(1, 256).min(32768 / cols)));
            // Let an outstanding Watch produce its baseline first. A new size
            // must not manufacture another Watch while that one is in flight.
            pane.retry |= grids.screen(seat).is_some();
            outgoing.retain(
                |old| !matches!(old, GuiAction::Resize {seat: old_seat, ..} if *old_seat == seat),
            );
        }
    }

    fn forget(&mut self, seat: SeatId, outgoing: &mut VecDeque<GuiAction>) {
        self.0.remove(&seat);
        outgoing.retain(|action| Self::seat(action) != Some(seat));
    }

    fn seat(action: &GuiAction) -> Option<SeatId> {
        match action {
            GuiAction::Watch { seat, .. } | GuiAction::Resize { seat, .. } => Some(*seat),
            _ => None,
        }
    }

    fn wanted(seat: SeatId, pane: &DesiredPane, grids: &Grids) -> Option<GuiAction> {
        let view = pane.view?;
        match grids.screen(seat) {
            None => Some(GuiAction::Watch { seat, view }),
            Some(screen) => pane
                .size
                .filter(|&(generation, cols, rows)| {
                    generation == view.generation
                        && (usize::from(cols), usize::from(rows)) != (screen.cols, screen.rows)
                })
                .map(|(generation, cols, rows)| GuiAction::Resize {
                    seat,
                    generation,
                    cols,
                    rows,
                }),
        }
    }

    fn queue_due(&self, grids: &Grids, outgoing: &mut VecDeque<GuiAction>, now: Instant) {
        for (&seat, pane) in &self.0 {
            if pane.retry
                && now >= pane.retry_at
                && !outgoing
                    .iter()
                    .any(|action| Self::seat(action) == Some(seat))
                && let Some(action) = Self::wanted(seat, pane, grids)
            {
                queue_action(outgoing, action);
            }
        }
    }

    fn sent(&mut self, action: &GuiAction, now: Instant) {
        if let Some(pane) = Self::seat(action).and_then(|seat| self.0.get_mut(&seat)) {
            pane.retry_at = now + CONTROL_RETRY;
            pane.retry = false;
        }
    }

    fn refused(&mut self, reason: &str, now: Instant) {
        if reason.starts_with("Busy:") {
            for pane in self.0.values_mut() {
                if !pane.retry {
                    pane.retry_at = pane.retry_at.max(now + CONTROL_RETRY);
                }
                pane.retry = true;
            }
        }
    }

    fn observed(&mut self, seat: SeatId, grids: &Grids) {
        if let Some(pane) = self.0.get_mut(&seat) {
            pane.retry = Self::wanted(seat, pane, grids).is_some();
        }
    }

    fn deadline(&self, grids: &Grids, outgoing: &VecDeque<GuiAction>) -> Option<Instant> {
        self.0
            .iter()
            .filter(|(seat, pane)| {
                pane.retry
                    && !outgoing
                        .iter()
                        .any(|action| Self::seat(action) == Some(**seat))
                    && Self::wanted(**seat, pane, grids).is_some()
            })
            .map(|(_, pane)| pane.retry_at)
            .min()
    }
}

fn operation_admit(model: &mut Arc<Model>, updates: &Latest<Arc<Model>>, op: Ulid) -> bool {
    let admitted = Arc::make_mut(&mut Arc::make_mut(model).operation_results).requested(op);
    if !admitted {
        updates.publish(model.clone());
    }
    admitted
}
fn operation_reply(
    model: &mut Arc<Model>,
    updates: &Latest<Arc<Model>>,
    op: Ulid,
    reply: bboxy450_term::messaging::Reply,
) {
    if Arc::make_mut(&mut Arc::make_mut(model).operation_results).complete(op, reply) {
        updates.publish(model.clone());
    }
}
fn operations_disconnected(model: &mut Arc<Model>, updates: &Latest<Arc<Model>>, reason: &str) {
    if Arc::make_mut(&mut Arc::make_mut(model).operation_results).disconnected(reason) {
        updates.publish(model.clone());
    }
}

fn publish_plugins(
    model: &mut Arc<Model>,
    updates: &Latest<Arc<Model>>,
    op: Ulid,
    entries: Vec<bbt_proto::gui::PluginEntry>,
    all: bool,
) {
    let target = &mut Arc::make_mut(model).plugin_registry;
    let previous = target
        .as_ref()
        .map_or(&[][..], |entries| entries.as_slice());
    *target = Some(Arc::new(bboxy450_term::settings::registry::merge(
        previous, entries, all,
    )));
    operation_reply(model, updates, op, bboxy450_term::messaging::Reply::Plugins);
}

fn publish_identity_control(
    model: &mut Arc<Model>,
    updates: &Latest<Arc<Model>>,
    op: Ulid,
    result: std::result::Result<bbt_proto::identity::IdentityControl, String>,
) {
    let revision = model
        .custody_reply
        .as_ref()
        .map_or(1, |reply| reply.revision.saturating_add(1));
    Arc::make_mut(model).custody_reply = Some(Arc::new(bboxy450_term::settings::IdentityReply {
        revision,
        op,
        result,
    }));
    updates.publish(model.clone());
}

#[allow(clippy::too_many_arguments)] // The session, its channels and its two UI callbacks.
#[cfg(test)]
async fn run(
    updates: Arc<Latest<Arc<Model>>>,
    commands: Commands,
    logger: Logger,
    session: Session,
    cwd: PathBuf,
    rendering: oneshot::Receiver<RenderContext>,
    backend_lost: impl Fn(Option<String>),
    capture: impl Fn(OperatorAsk),
) -> Result<()> {
    run_with_control(
        updates,
        commands,
        logger,
        session,
        cwd,
        rendering,
        backend_lost,
        capture,
        |_| {},
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn run_with_control(
    updates: Arc<Latest<Arc<Model>>>,
    mut commands: Commands,
    logger: Logger,
    mut session: Session,
    cwd: PathBuf,
    rendering: oneshot::Receiver<RenderContext>,
    backend_lost: impl Fn(Option<String>),
    capture: impl Fn(OperatorAsk),
    app: impl Fn(AppEvent),
) -> Result<()> {
    logger.write(format!(
        "cockpit ready: supervisor={} logic={}",
        session.supervisor.id().unwrap_or(0),
        session.logic.id().unwrap_or(0)
    ));
    let mut seats: BTreeMap<_, _> = session
        .list()
        .await?
        .into_iter()
        .map(|seat| (seat.record.id, seat))
        .collect();
    let mut identity = session.endpoint.identity.clone();
    let custody = if cfg!(debug_assertions)
        && std::env::var("BBOXY_TEST_CUSTODY").as_deref() == Ok("memory")
    {
        "memory custody (debug)"
    } else {
        "native custody"
    };
    logger.write(format!(
        "initial identity status: {:?}, reason={:?}, selector={custody}",
        identity.state, identity.reason
    ));
    let swarm = seats
        .values()
        .next()
        .map_or_else(Ulid::new, |seat| seat.record.swarm);
    let rendering = rendering
        .await
        .map_err(|_| invalid("renderer bootstrap stopped"))?;
    let mut options = terminal::Options::default();
    options.font.scale = rendering.scale.round().max(1.0) as u32;
    let mut preparer = PanePreparer::with_options(rendering.context, options)?;
    let mut grids = Grids::new();
    let mut initial = Model::default();
    initial.ui.window.theme = rendering.theme;
    let mut model = Arc::new(initial);
    publish(
        &seats,
        &identity,
        &grids,
        &mut model,
        &mut preparer,
        &updates,
    )?;
    let mut lost = false;
    let mut retained = VecDeque::new();
    let mut controls = PaneControls::default();
    let mut history = History::default();
    let mut action_replies = ActionReplies::default();
    let mut document_reader = documents::Reader::default();
    let mut document_preparation: Option<(Ulid, tokio::task::JoinHandle<Arc<documents::Reply>>)> =
        None;
    let mut knowledge_watch = knowledge::Watch::new(knowledge::promises().map_err(invalid)?);
    knowledge_watch.observe(commands.knowledge.enabled.load(Ordering::Acquire));
    let mut in_flight = None;
    let mut closing = false;
    // Accept one UI command's actions at a time. Watch diffs are finite;
    // the stream allows one unapplied update per watch. Deduplicate Unwatch
    // notifications so a full writer cannot create an unbounded control queue.
    let mut outgoing = VecDeque::new();
    attach_events(&mut model, &mut outgoing);
    updates.publish(model.clone());
    while !closing {
        knowledge_watch.connection(!lost);
        if let Some(action) = knowledge_watch.next_action() {
            queue_action(&mut outgoing, action);
        }
        let mut writer_blocked = false;
        if !lost {
            controls.queue_due(&grids, &mut outgoing, Instant::now());
            history.retain(|seat, view, screen| {
                grids
                    .baseline(seat)
                    .is_some_and(|(current, _)| current == view)
                    && grids.screen(seat).is_some_and(|current| {
                        (current.cols, current.rows) == (screen.cols, screen.rows)
                    })
                    && seats
                        .get(&Ulid(seat.0.0))
                        .is_some_and(|seat| seat.record.generation == view.generation)
            });
            if !outgoing
                .iter()
                .any(|action| matches!(action, GuiAction::GetHistory { .. }))
                && let Some(action) = history.next_action(Instant::now())
            {
                queue_action(&mut outgoing, action);
            }
            while let Some(action) = outgoing.front() {
                if action_replies.full() {
                    break;
                }
                match session.send(action.clone()).await {
                    Ok(()) => {
                        action_replies.sent(action);
                        controls.sent(action, Instant::now());
                        outgoing.pop_front();
                    }
                    Err(error) if error.is::<cockpit::Busy>() => {
                        writer_blocked = true;
                        break;
                    }
                    Err(error) => {
                        logger.write(format!("GUI writer stopped: {error}"));
                        operations_disconnected(&mut model, &updates, &error.to_string());
                        backend_lost(Some(error.to_string()));
                        lost = true;
                        history.disconnected();
                        action_replies.clear();
                        if Arc::make_mut(&mut model).spawns_disconnected(&error.to_string()) {
                            updates.publish(model.clone());
                        }
                        discard_uncertain_actions(&mut outgoing);
                        outgoing.retain(|action| !matches!(action, GuiAction::GetHistory { .. }));
                        if disconnected_events(&mut model, &mut outgoing, &error.to_string()) {
                            updates.publish(model.clone());
                        }
                        break;
                    }
                }
            }
        }
        if lost
            && let Some(reply) =
                document_reader.disconnected("Knowledge connection lost; open again to retry")
        {
            Arc::make_mut(&mut model).document_reply = Some(reply);
            updates.publish(model.clone());
        }
        knowledge_watch.connection(!lost);
        if let Some(snapshot) = knowledge_watch.publication() {
            Arc::make_mut(&mut model).e2e = Some(snapshot);
            updates.publish(model.clone());
        }
        let retry_at = controls
            .deadline(&grids, &outgoing)
            .into_iter()
            .chain(history.deadline())
            .min();
        let writer_ready = session.writer_ready();
        let result:Result<()>=async {
            tokio::select! {
                changed=commands.context.changed()=> {
                    if changed.is_ok() {
                        let context=commands.context.borrow_and_update().clone();
                        if let Some(context)=context {
                            let current=Arc::make_mut(&mut model);
                            current.ui.window.theme=context.theme;current.ui.window.density=context.density;current.settings=Some(context.snapshot);
                            publish(&seats,&identity,&grids,&mut model,&mut preparer,&updates)?;
                        }
                    }
                },
                prepared=async {(&mut document_preparation.as_mut().expect("guarded document worker").1).await}, if document_preparation.is_some()=> {
                    let (id,_)=document_preparation.take().expect("completed document worker");
                    let reply=prepared.unwrap_or_else(|error|Arc::new(documents::Reply {id,response:documents::Response::Failed(format!("document_preparation_failed: {error}"))}));
                    Arc::make_mut(&mut model).document_reply=Some(reply); updates.publish(model.clone());
                },
                _=commands.knowledge.notify.notified()=> {
                    knowledge_watch.observe(commands.knowledge.enabled.load(Ordering::Acquire));
                },
                command=async {
                    if !lost && let Some(message)=retained.pop_front() { return Some(message); }
                    commands.messages.recv().await
                }, if (lost && retained.len() < COMMAND_CAPACITY) || (!lost && outgoing.is_empty())=>match command {
                    Some(message) if lost=>retained.push_back(message),
                    Some(message)=> {
                    let message=match message {
                        UiMsg::SpawnZsh=>UiMsg::Spawn {id:Ulid::new(),swarm:None,placement:None,harness_kind:"zsh".into(),name:None,role:None,location:Where::Local},
                        message=>message,
                    };
                    in_flight=Some(message);
                    match in_flight.as_ref().unwrap() {
                    message @ (UiMsg::Spawn {id,..}|UiMsg::SpawnSpec {id,..})=> {
                        let spec=spawn_spec(message,swarm,&cwd).expect("spawn intent branch");
                        match Arc::make_mut(&mut model).request_spawn(*id,spec.clone()) {
                            Ok(true)=> {
                                queue_action(&mut outgoing,GuiAction::Spawn {id:*id,spec});
                                publish(&seats,&identity,&grids,&mut model,&mut preparer,&updates)?;
                            }
                            Ok(false)=>logger.write(format!("spawn {id} already retained; not automatically resubmitted")),
                            Err(reason)=>logger.write(format!("spawn {id} refused: {reason}")),
                        }
                    },
                    UiMsg::Close(intent)=> {
                        match Arc::make_mut(&mut model).closes.register(intent.clone()) {
                            Ok(true)=>queue_action(&mut outgoing,GuiAction::ScheduleClose {seat:intent.seat,id:intent.operation,position:intent.position.clone()}),
                            Ok(false)=>(),Err(reason)=>logger.write(format!("close {} refused: {reason}",intent.seat)),
                        }
                    },
                    UiMsg::UndoClose {seat}=> {
                        match model.closes.undo_in(&model.ui,*seat,Instant::now()) {
                            Ok((close_id,layout_revision))=>queue_action(&mut outgoing,GuiAction::UndoClose {seat:*seat,close_id,layout_revision}),
                            Err(reason)=>logger.write(format!("undo close {seat} refused: {reason}")),
                        }
                    },
                    UiMsg::SpawnZsh=>unreachable!("normalized at IO admission"),
                    UiMsg::ConsumeSpawn {id}=> {
                        Arc::make_mut(&mut model).consume_spawn(*id);
                        publish(&seats,&identity,&grids,&mut model,&mut preparer,&updates)?;
                    },
                    UiMsg::Messaging {id,swarm_id,request}=> { if operation_admit(&mut model,&updates,*id) { queue_action(&mut outgoing,GuiAction::Messaging {id:*id,swarm_id:*swarm_id,request:request.clone()}); } },
                    UiMsg::Task {id,request}=> { if operation_admit(&mut model,&updates,*id) { queue_action(&mut outgoing,GuiAction::Task {id:*id,request:request.clone()}); } },
                    UiMsg::GetAttention {id,seat,generation}=> {
                        if operation_admit(&mut model,&updates,*id) {
                            queue_action(&mut outgoing,GuiAction::GetAttention {id:*id,seat:*seat,generation:*generation});
                        }
                    },
                    UiMsg::SetAttention {id,seat,edit}=> {
                        if operation_admit(&mut model,&updates,*id) {
                            queue_action(&mut outgoing,GuiAction::SetAttention {id:*id,seat:*seat,edit:edit.clone()});
                        }
                    },
                    UiMsg::PluginControl {id,request}=> { if operation_admit(&mut model,&updates,*id) { queue_action(&mut outgoing,GuiAction::Plugin {id:*id,request:request.clone()}); } },
                    UiMsg::IdentityControl {id,request}=>queue_action(&mut outgoing,GuiAction::Identity(bbt_proto::identity::IdentityCall {id:*id,request:*request})),
                    UiMsg::Input {seat,generation,id,bytes}=> {
                        if return_live(&mut history,&mut model,*seat) {
                            publish(&seats,&identity,&grids,&mut model,&mut preparer,&updates)?;
                        }
                        queue_action(&mut outgoing, GuiAction::Input {seat:*seat,generation:*generation,id:*id,bytes:bytes.clone()});
                    },
                    UiMsg::EventPage {stream,after,limit}=>queue_action(&mut outgoing,GuiAction::GetEvents {stream:stream.clone(),after:*after,limit:*limit}),
                    UiMsg::EventCursor {stream}=>queue_action(&mut outgoing,GuiAction::GetEventCursor {stream:stream.clone()}),
                    UiMsg::Captured {id,error,frame}=>queue_action(&mut outgoing,GuiAction::Captured {id:*id,error:error.clone(),frame:*frame}),
                    UiMsg::Focused {id,result}=>queue_action(&mut outgoing,GuiAction::Focused {id:*id,result:result.clone()}),
                    UiMsg::UiReply {id,result}=>queue_action(&mut outgoing,GuiAction::UiReply {id:*id,result:result.clone()}),
                    UiMsg::HistoryMetrics {id,status}=> {
                        let mut status=status.clone();
                        status.generation=session.endpoint.generation;
                        status.custody_backend=session.endpoint.custody_backend.clone();
                        status.history=history.metrics(&model.history);
                        status.history["r5i"]=crate::stall::diagnostic_report();
                        queue_action(&mut outgoing,GuiAction::AppReply {id:*id,status});
                    },
                    UiMsg::AppReply {id,status}=> {
                        let mut status=status.clone();
                        status.generation=session.endpoint.generation;
                        status.custody_backend=session.endpoint.custody_backend.clone();
                        queue_action(&mut outgoing,GuiAction::AppReply {id:*id,status});
                    },
                    UiMsg::MarkEventsRead {stream,sequence}=>queue_action(&mut outgoing,GuiAction::MarkEventsRead {stream:stream.clone(),seq:*sequence}),
                    UiMsg::AttentionFocus {seat,generation,episode,landed}=> {
                        if Arc::make_mut(&mut model).attention_focus(*seat,*generation,*episode,*landed) {
                            updates.publish(model.clone());
                        }
                    }
                    UiMsg::Resize {seat,generation,cols,rows}=> {
                        if seats.get(&Ulid(seat.0.0)).is_some_and(|metadata| metadata.state.generation==*generation) {
                            if grids.screen(*seat).is_some_and(|screen| (screen.cols,screen.rows) != (usize::from(*cols),usize::from(*rows)))
                                && return_live(&mut history,&mut model,*seat) {
                                publish(&seats,&identity,&grids,&mut model,&mut preparer,&updates)?;
                            }
                            controls.resize(*seat,*generation,*cols,*rows,&grids,&mut outgoing);
                        }
                    }
                    UiMsg::Scroll {seat,generation,request}=> {
                        if let (Some((view,sequence)),Some(screen)) = (grids.baseline(*seat),grids.screen(*seat))
                            && view.generation == *generation {
                                if let Err(reason) = history.request(*seat,view,sequence,screen,*request) {
                                    logger.write(format!("history {seat}: {reason}"));
                                    history.failed(*seat,reason);
                                }
                                let changed = !history.contains(*seat) && model.history.contains_key(seat);
                                if changed { Arc::make_mut(&mut model).history.remove(seat); }
                                let completed = scroll_readback(&mut history,&mut model,&grids);
                                if changed { publish(&seats,&identity,&grids,&mut model,&mut preparer,&updates)?; }
                                else if completed { updates.publish(model.clone()); }
                        } else if scroll_refused(&mut model,*seat,*generation,*request,"history_view_unavailable") { updates.publish(model.clone()); }
                    }
                    UiMsg::DocumentRead {id,read}=> {
                        if let Some(action)=document_reader.admit(*id,read.clone()) { queue_action(&mut outgoing,action); }
                    },
                    UiMsg::ConsumeDocument {id}=> {
                        if model.document_reply.as_ref().is_some_and(|r|r.id==*id) {Arc::make_mut(&mut model).document_reply=None;}
                    },
                    UiMsg::Watched(panes)=> {
                        Arc::make_mut(&mut model).ui.window.stage.focused = panes.first().map(|(seat, _)| *seat);
                        controls.set_watched(panes,&mut outgoing,Instant::now());
                        for action in grids.set_watched(panes) {
                            if matches!(action,GuiAction::Unwatch {..}) {queue_action(&mut outgoing, action);}
                        }
                        publish(&seats,&identity,&grids,&mut model,&mut preparer,&updates)?;
                    }
                    }
                    in_flight=None;
                    }
                    None=>closing=true,
                },
                _=&mut commands.stop=>closing=true,
                _=tokio::time::sleep_until(tokio::time::Instant::from_std(retry_at.unwrap_or_else(Instant::now))), if retry_at.is_some() && !lost=>{},
                ready=writer_ready, if writer_blocked && !lost=>{ready?;},
                _=async {}, if history.has_work() && !lost=> {
                    if let Some((seat,result)) = history.step() {
                        match result {
                            Ok(Some(screen))=> {Arc::make_mut(&mut model).history.insert(seat,screen);},
                            Ok(None)=> {Arc::make_mut(&mut model).history.remove(&seat);},
                            Err(reason)=> {
                                Arc::make_mut(&mut model).history.remove(&seat);
                                logger.write(format!("history {seat}: {reason}"));
                            }
                        }
                        scroll_readback(&mut history,&mut model,&grids);
                        publish(&seats,&identity,&grids,&mut model,&mut preparer,&updates)?;
                    }
                },
                event=session.event()=>match event.unwrap_or_else(|error| cockpit::Event::Disconnected(error.to_string())) {
                    cockpit::Event::Reply(reply)=> {
                    let _apply = bbt_rt::diagnostics::span(bbt_rt::diagnostics::Phase::GuiApply).work(1, 0);
                    // Classify the raw reply before an async Seat event becomes a state update.
                    let terminal_reply = action_replies.take(&reply);
                      let mut ack=None;
                      let reply=match *reply {
                        GuiReply::Event {stream,seq,subject,envelope}=> {
                            if envelope.metadata.reference.position.stream!=stream
                                || envelope.metadata.reference.position.sequence!=seq
                                || envelope.metadata.subject!=subject {
                                logger.write("event routing metadata disagrees; no ACK");
                                return Ok(());
                            }
                            let payload=envelope.payload.clone();
                            match publish_delivery(&mut model,&updates,GuiEventRecord::Event(envelope)) {
                                Ok(action)=>ack=action,
                                Err(error)=> {logger.write(format!("event rejected (no ACK): {error}"));return Ok(());}
                            }
                            if subject.starts_with("doc/") || subject.starts_with("activity/") || subject.starts_with("task/") {
                                knowledge_watch.changed();
                            }
                            match payload {
                                GuiEventPayload::Seat {seat}=>Some(GuiReply::Seat(*seat)),
                                GuiEventPayload::Identity {identity}=>Some(GuiReply::Identity(identity)),
                                GuiEventPayload::Host {host,..}=>Some(GuiReply::HostState {host}),
                                GuiEventPayload::Activity {..}|GuiEventPayload::Message {..}=>None,
                            }
                        }
                        GuiReply::Gap {stream,first,last,reason}=> {
                            match publish_delivery(&mut model,&updates,GuiEventRecord::Gap(EventGap {stream,first,last,reason})) {
                                Ok(action)=>ack=action,
                                Err(error)=> {logger.write(format!("event gap rejected (no ACK): {error}"));return Ok(());}
                            }
                            knowledge_watch.changed();
                            None
                        }
                        GuiReply::Events {page}=> {
                            publish_page(&mut model,&updates,page)?;
                            None
                        }
                        GuiReply::EventCursor {cursor}=> {
                            if event_model(&mut model).apply_cursor(cursor).map_err(invalid)? {updates.publish(model.clone());}
                            None
                        }
                        GuiReply::EventStatus {attached,reason}=> {
                            if event_model(&mut model).status(if attached {None} else {Some(reason.as_deref().unwrap_or("events held"))}) {
                                updates.publish(model.clone());
                                if attached {queue_action(&mut outgoing,GuiAction::GetEventCursor {stream:format!("swarm-{swarm}")});}
                            }
                            None
                        }
                        GuiReply::Plugin {op,entries}=> {
                            if let Some(ActionReply::Plugin(expected,all)) = terminal_reply && expected==op {
                                publish_plugins(&mut model, &updates, op, entries, all);
                            }
                            None
                        },
                        GuiReply::Attention {op,mut seat}=> {
                            if terminal_reply == Some(ActionReply::Attention(op)) {
                                Arc::make_mut(&mut model).attention_read=Some(Arc::new(seat.clone()));
                                seat.attention=bboxy450_term::settings::messaging::observation(seat.attention.as_ref());
                                operation_reply(&mut model,&updates,op,bboxy450_term::messaging::Reply::Attention);
                                Some(GuiReply::Seat(seat))
                            } else { None }
                        },
                        reply=>Some(reply),
                      };
                      if let Some(reply)=reply {match reply {
                        GuiReply::Hosts(hosts)=> {
                            if model.hosts != hosts {
                                Arc::make_mut(&mut model).hosts = hosts;
                                publish(&seats,&identity,&grids,&mut model,&mut preparer,&updates)?;
                            }
                        }
                        GuiReply::HostState {host}=> {
                            let current = &mut Arc::make_mut(&mut model).hosts;
                            let changed = if let Some(old) = current.iter_mut().find(|old| old.location == host.location) {
                                if old == &host { false } else { *old = host; true }
                            } else { current.push(host); true };
                            if changed { publish(&seats,&identity,&grids,&mut model,&mut preparer,&updates)?; }
                        }
                        GuiReply::Identity(next)=> {
                            if identity!=next {
                                identity=next;
                                 for (id,spec) in Arc::make_mut(&mut model).resume_held_spawns() {
                                     queue_action(&mut outgoing,GuiAction::Spawn {id,spec});
                                 }
                                logger.write(format!("identity status: {:?}",identity.state));
                                publish(&seats,&identity,&grids,&mut model,&mut preparer,&updates)?;
                            }
                        }
                        GuiReply::Seats(next)=> {
                            seats=next.into_iter().map(|seat|(seat.record.id,seat)).collect();
                            Arc::make_mut(&mut Arc::make_mut(&mut model).scroll).retain(|seat| seats.contains_key(&Ulid(seat.0.0)));
                            publish(&seats,&identity,&grids,&mut model,&mut preparer,&updates)?;
                            if lost {
                                lost=false; backend_lost(None);
                                attach_events(&mut model,&mut outgoing);
                                updates.publish(model.clone());
                            }
                        }
                        GuiReply::Seat(seat)=> {
                            if !seats.get(&seat.record.id).is_some_and(|old|old==&seat||old.state.generation>seat.state.generation) {
                            let id=SeatId::from_u128(seat.record.id.0);
                            if matches!(seat.state.process,ProcessStatus::Exited {..}|ProcessStatus::Retained) {
                                grids.forget(id);
                                return_live(&mut history,&mut model,id);
                                controls.forget(id,&mut outgoing);
                                queue_action(&mut outgoing, GuiAction::Unwatch {seat:id});
                            }
                            logger.write(format!("seat {} activity={:?} process={:?}",seat.record.id,seat.state.activity,seat.state.process));
                            seats.insert(seat.record.id,seat);
                            publish(&seats,&identity,&grids,&mut model,&mut preparer,&updates)?;
                            }
                        }
                         GuiReply::Spawned {op,seat}=> {
                             if Arc::make_mut(&mut model).spawned(op,seat) {
                                 if !seats.contains_key(&Ulid(seat.0.0)) {
                                     queue_action(&mut outgoing,GuiAction::ListSeats);
                                 }
                                 publish(&seats,&identity,&grids,&mut model,&mut preparer,&updates)?;
                             }
                         }
                        GuiReply::Update {seat,update}=> {
                            let (view,sequence)=match &update {
                                Update::Snapshot {snapshot}=>(snapshot.view,snapshot.sequence),
                                Update::Delta {delta}=>(delta.view,delta.sequence),
                            };
                            match grids.apply(seat,&update) {
                                Ok(changed)=> {
                                    let scroll_changed = model.scroll.observed(seat,grids.baseline(seat));
                                    if scroll_changed { Arc::make_mut(&mut Arc::make_mut(&mut model).scroll).observe(seat,grids.baseline(seat)); }
                                    if changed.is_some() {publish(&seats,&identity,&grids,&mut model,&mut preparer,&updates)?;}
                                    else if scroll_changed {updates.publish(model.clone());}
                                    controls.observed(seat,&grids);
                                    // A valid byte-equivalent update still needs
                                    // its ack; an unwatched in-flight one does not.
                                    if grids.screen(seat).is_some() {
                                        queue_action(&mut outgoing, GuiAction::Applied {seat,view,sequence});
                                    }
                                }
                                Err(error)=>logger.write(format!("grid update rejected (no ack): {error}")),
                            }
                        }
                        GuiReply::Ok {op}=> {
                            if event_model(&mut model).attach_result(op,None) {
                                updates.publish(model.clone());
                                queue_action(&mut outgoing,GuiAction::GetEventCursor {stream:format!("swarm-{swarm}")});
                            }
                            logger.write(format!("GUI operation {op} accepted"));
                        },
                        GuiReply::Error {op,reason}=> {
                            Arc::make_mut(&mut model).closes.refused(op);
                            if terminal_reply == Some(ActionReply::Identity(op)) { publish_identity_control(&mut model,&updates,op,Err(reason.clone())); }
                            if terminal_reply == Some(ActionReply::Knowledge(op)) {
                                let result=Err(bbt_mux::knowledge::data_source::CommandError::new("transport",reason.clone()));
                                if let Some(work)=document_reader.response(op,result.clone()) {
                                    document_preparation=Some((op,tokio::task::spawn_blocking(move ||work.prepare())));
                                } else { knowledge_watch.response(op,result); }
                            }
                            let result = if matches!(terminal_reply,Some(ActionReply::Plugin(..))) && reason.starts_with("unknown:") {
                                bboxy450_term::messaging::Reply::Unknown(reason.clone())
                            } else { bboxy450_term::messaging::Reply::Refused(reason.clone()) };
                            operation_reply(&mut model,&updates,op,result);
                            publish_page_refusal(&mut model,&updates,terminal_reply.as_ref(),&reason)?;
                            if matches!(&terminal_reply,Some(ActionReply::History(_)))
                                && let Some(seat) = history.rejected(reason.starts_with("Busy:"),Instant::now(),&reason)
                                && return_live(&mut history,&mut model,seat) {
                                publish(&seats,&identity,&grids,&mut model,&mut preparer,&updates)?;
                            }
                            if event_model(&mut model).attach_result(op,Some(&reason)) {updates.publish(model.clone());}
                            controls.refused(&reason,Instant::now());
                             if Arc::make_mut(&mut model).spawn_rejected(op,reason.clone()) {
                                 publish(&seats,&identity,&grids,&mut model,&mut preparer,&updates)?;
                             }
                            logger.write(format!("GUI operation {op} rejected: {reason}"));
                        }
                        GuiReply::Messaging {op,response}=>operation_reply(&mut model,&updates,op,bboxy450_term::messaging::Reply::Messaging(response)),
                        GuiReply::Task {op,response}=>operation_reply(&mut model,&updates,op,bboxy450_term::messaging::Reply::Task(response)),
                        GuiReply::IdentityControl {op,response}=> {
                            if terminal_reply == Some(ActionReply::Identity(op)) { publish_identity_control(&mut model,&updates,op,Ok(response)); }
                        },
                        GuiReply::Knowledge {op,response}=> {
                            if terminal_reply == Some(ActionReply::Knowledge(op)) {
                                let result=serde_json::from_value(response).unwrap_or_else(|error| Err(bbt_mux::knowledge::data_source::CommandError::new("invalid_provider_result",format!("invalid knowledge reply: {error}"))));
                                if let Some(work)=document_reader.response(op,result.clone()) {
                                    document_preparation=Some((op,tokio::task::spawn_blocking(move ||work.prepare())));
                                } else {knowledge_watch.response(op,result);}
                            }
                        },
                        GuiReply::History {seat,records,next}=> {
                            if terminal_reply == Some(ActionReply::History(seat))
                                && let Err(reason) = history.accept(seat,records,next) {
                                history.failed(seat,reason);
                                let changed = return_live(&mut history,&mut model,seat);
                                logger.write(format!("history {seat}: {reason}"));
                                if changed {publish(&seats,&identity,&grids,&mut model,&mut preparer,&updates)?;}
                            }
                        },
                        GuiReply::UiAsk {op,request}=>capture(OperatorAsk::Ui {id:op,request,prior_spawns:model.spawns.keys().copied().collect()}),
                        GuiReply::Ui {..}=>{},
                        GuiReply::AppAsk {op,request}=>app(AppEvent::Ask {id:op,request}),
                        GuiReply::AppStopping {op}=>app(AppEvent::Stop {id:op}),
                        GuiReply::App {..}=>{},
                        GuiReply::CaptureAsk {op,path,seat}=>capture(OperatorAsk::Capture(CaptureAsk {id:op,path:PathBuf::from(path),seat:seat.map(|seat| SeatId::from_u128(seat.0))})),
                        GuiReply::OperatorFocus {op,seat}=>capture(OperatorAsk::Focus(bboxy450_term::focus_ack::Ask {id:op,seat,prior_spawns:model.spawns.keys().copied().collect()})),
                        GuiReply::OperatorClose {op,seat}=>capture(OperatorAsk::Close {id:op,seat,undo:false}),
                        GuiReply::OperatorUndoClose {op,seat}=>capture(OperatorAsk::Close {id:op,seat,undo:true}),
                        GuiReply::Captured {op,path,..}=>logger.write(format!("capture {op} written: {path}")),
                        // The window never asks for a screen as text; ctl does.
                        GuiReply::GuiHello {op,..}|GuiReply::Plugin {op,..}|GuiReply::Debug {op,..}|GuiReply::Focused {op,..}|GuiReply::Screen {op,..}|GuiReply::EventsRead {op,..}|GuiReply::EventsStreams {op,..}=>logger.write(format!("ctl reply {op} unused by the window")),
                        GuiReply::OperatorSpawned {op,seat,spec}=> {
                            // Placed and focused as the launcher's own pick is.
                            let placed=Arc::make_mut(&mut model);
                            match placed.request_spawn(op,spec) {
                                Ok(_)=>if placed.spawned(op,seat) {
                                    if !seats.contains_key(&Ulid(seat.0.0)) {
                                        queue_action(&mut outgoing,GuiAction::ListSeats);
                                    }
                                    publish(&seats,&identity,&grids,&mut model,&mut preparer,&updates)?;
                                },
                                Err(reason)=>logger.write(format!("operator spawn {op} not placed: {reason}")),
                            }
                        }
                        GuiReply::Attention {..}|GuiReply::Event {..}|GuiReply::Gap {..}|GuiReply::Events {..}|GuiReply::EventCursor {..}|GuiReply::EventStatus {..}=>unreachable!("handled before state dispatch"),
                      }}
                      // Every accepted projection is in the published Model
                      // before its transport ACK can enter the writer queue.
                      if let Some(action)=ack {queue_action(&mut outgoing,action);}
                    },
                    cockpit::Event::Exited {process,status}=> {
                        let reason=format!("{process} exited: {status}");
                        logger.write(&reason);
                         if !lost { operations_disconnected(&mut model,&updates,&reason); backend_lost(Some(reason.clone())); }
                        lost=true;
                        history.disconnected();
                        action_replies.clear();
                         if Arc::make_mut(&mut model).spawns_disconnected(&reason) {
                             updates.publish(model.clone());
                         }
                         discard_uncertain_actions(&mut outgoing);
                         outgoing.retain(|action| !matches!(action,GuiAction::GetHistory {..}));
                         if disconnected_events(&mut model,&mut outgoing,&reason) {updates.publish(model.clone());}
                    }
                    cockpit::Event::Disconnected(reason)=> {
                        logger.write(format!("GUI disconnected: {reason}"));
                         if !lost { operations_disconnected(&mut model,&updates,&reason); backend_lost(Some(reason.clone())); }
                        lost=true;
                        history.disconnected();
                        action_replies.clear();
                         if Arc::make_mut(&mut model).spawns_disconnected(&reason) {
                             updates.publish(model.clone());
                         }
                         discard_uncertain_actions(&mut outgoing);
                         outgoing.retain(|action| !matches!(action,GuiAction::GetHistory {..}));
                         if disconnected_events(&mut model,&mut outgoing,&reason) {updates.publish(model.clone());}
                    }
                }
            }
        Ok(())
        }.await;
        if let Err(error) = result {
            logger.write(format!("live IO stopped: {error}"));
            if !lost {
                operations_disconnected(&mut model, &updates, &error.to_string());
                backend_lost(Some(error.to_string()));
            }
            lost = true;
            history.disconnected();
            action_replies.clear();
            if Arc::make_mut(&mut model).spawns_disconnected(&error.to_string()) {
                updates.publish(model.clone());
            }
            discard_uncertain_actions(&mut outgoing);
            outgoing.retain(|action| !matches!(action, GuiAction::GetHistory { .. }));
            if disconnected_events(&mut model, &mut outgoing, &error.to_string()) {
                updates.publish(model.clone());
            }
            if let Some(message) = in_flight.take() {
                if let UiMsg::Watched(panes) = &message {
                    for (seat, _) in panes {
                        grids.forget(*seat);
                    }
                }
                retained.push_front(message);
            }
        }
        if scroll_readback(&mut history, &mut model, &grids) {
            updates.publish(model.clone());
        }
    }
    let stopped = session.shutdown().await;
    if stopped.is_ok() {
        logger.write("cockpit children reaped after orderly shutdown");
    }
    stopped
}

#[cfg(test)]
mod tests {
    use super::*;

    // keeps: spec c1-live-seats §2a
    #[test]
    fn native_terminal_replies_are_correlated_and_async_identity_never_releases_them() {
        let mut replies = ActionReplies::default();
        let op = Ulid::new();
        replies.sent(&GuiAction::Identity(bbt_proto::identity::IdentityCall {
            id: op,
            request: bbt_proto::identity::IdentityRequest::Status,
        }));
        assert!(
            replies
                .take(&GuiReply::Identity(bbt_proto::GuiIdentity {
                    state: bbt_proto::gui::GuiIdentityState::Pending,
                    reason: Some("held".into())
                }))
                .is_none()
        );
        assert!(
            replies
                .take(&GuiReply::Error {
                    op: Ulid::new(),
                    reason: "unrelated".into()
                })
                .is_none()
        );
        let response = bbt_proto::identity::IdentityControl {
            custody: None,
            acl: bbt_proto::identity::CustodyAcl::Unknown,
            hold: None,
            clear_actions: vec![],
            outcome: None,
            pending: vec![],
            reopen_failed: None,
        };
        assert!(
            replies
                .take(&GuiReply::IdentityControl {
                    op: Ulid::new(),
                    response: response.clone()
                })
                .is_none()
        );
        assert_eq!(
            replies.take(&GuiReply::IdentityControl { op, response }),
            Some(ActionReply::Identity(op))
        );
        replies.sent(&GuiAction::Identity(bbt_proto::identity::IdentityCall {
            id: op,
            request: bbt_proto::identity::IdentityRequest::Unlock,
        }));
        assert_eq!(
            replies.take(&GuiReply::Error {
                op,
                reason: "refused".into()
            }),
            Some(ActionReply::Identity(op))
        );
    }

    // keeps: spec c1-live-seats §2a
    #[test]
    fn native_retained_results_publish_every_revision_and_survive_metadata_rebuild() {
        let dirty = bbt_rt::DirtySet::new(1, Arc::new(NoWake));
        let updates = Latest::new(dirty.source(0));
        let mut model = Arc::new(Model::default());
        let op = Ulid::new();
        for revision in 1..=2 {
            publish_identity_control(&mut model, &updates, op, Err("unknown".into()));
            let published = updates.take().unwrap();
            assert_eq!(published.custody_reply.as_ref().unwrap().revision, revision);
        }
        let rebuilt = model::build(&[], &Default::default(), &model);
        assert!(Arc::ptr_eq(
            rebuilt.custody_reply.as_ref().unwrap(),
            model.custody_reply.as_ref().unwrap()
        ));
        let mut outgoing =
            VecDeque::from([GuiAction::Identity(bbt_proto::identity::IdentityCall {
                id: op,
                request: bbt_proto::identity::IdentityRequest::Unlock,
            })]);
        discard_uncertain_actions(&mut outgoing);
        assert!(
            outgoing.is_empty(),
            "disconnect must never replay native intents"
        );
    }

    #[allow(
        clippy::disallowed_methods,
        reason = "test/fixture IO, not the UI thread"
    )]
    // keeps: spec c1-live-seats §2a
    #[tokio::test]
    async fn knowledge_demand_survives_a_full_command_queue_and_coalesces() {
        let (sender, commands) = command_channel();
        for _ in 0..COMMAND_CAPACITY {
            sender.try_send(UiMsg::SpawnZsh).unwrap();
        }
        assert!(sender.try_send(UiMsg::SpawnZsh).is_err());
        sender.observe_knowledge(true);
        sender.observe_knowledge(false);
        sender.observe_knowledge(true);
        use std::{
            future::Future,
            task::{Context, Poll, Waker},
        };
        let mut cx = Context::from_waker(Waker::noop());
        assert_eq!(
            std::pin::pin!(commands.knowledge.notify.notified())
                .as_mut()
                .poll(&mut cx),
            Poll::Ready(())
        );
        assert!(commands.knowledge.enabled.load(Ordering::Acquire));
        assert_eq!(
            std::pin::pin!(commands.knowledge.notify.notified())
                .as_mut()
                .poll(&mut cx),
            Poll::Pending
        );
    }

    // keeps: spec c1-live-seats §2a
    #[test]
    fn knowledge_reply_correlation_cannot_consume_an_unrelated_result() {
        let mut replies = ActionReplies::default();
        let id = Ulid::new();
        replies.sent(&GuiAction::Knowledge {
            id,
            method: "list".into(),
            args: serde_json::json!({}),
        });
        for op in [Ulid::nil(), Ulid::new()] {
            assert_eq!(
                replies.take(&GuiReply::Knowledge {
                    op,
                    response: serde_json::json!({})
                }),
                None
            );
            assert_eq!(
                replies.take(&GuiReply::Error {
                    op,
                    reason: "unrelated".into()
                }),
                None
            );
        }
        assert_eq!(
            replies.take(&GuiReply::Knowledge {
                op: id,
                response: serde_json::json!({})
            }),
            Some(ActionReply::Knowledge(id))
        );
    }

    // keeps: spec c1-live-seats §2a
    #[test]
    fn held_and_reordered_knowledge_replies_do_not_block_control_or_history_results() {
        let mut replies = ActionReplies::default();
        let a = Ulid::new();
        let b = Ulid::new();
        let seat = SeatId::from_u128(42);
        for action in [
            GuiAction::Knowledge {
                id: a,
                method: "list".into(),
                args: serde_json::json!({}),
            },
            GuiAction::GetHistory {
                seat,
                from: 0,
                to: 1,
            },
            GuiAction::Input {
                seat,
                generation: 1,
                id: Ulid::new(),
                bytes: vec![b'x'],
            },
            GuiAction::Knowledge {
                id: b,
                method: "fetch".into(),
                args: serde_json::json!({}),
            },
            GuiAction::Resize {
                seat,
                generation: 1,
                cols: 80,
                rows: 24,
            },
        ] {
            replies.sent(&action);
        }
        assert_eq!(
            replies.take(&GuiReply::Error {
                op: Ulid::new(),
                reason: "history refused".into()
            }),
            Some(ActionReply::History(seat))
        );
        assert_eq!(
            replies.take(&GuiReply::Knowledge {
                op: b,
                response: serde_json::json!({"Err":{"code":"provider_busy"}})
            }),
            Some(ActionReply::Knowledge(b))
        );
        for _ in 0..2 {
            assert_eq!(
                replies.take(&GuiReply::Ok { op: Ulid::new() }),
                Some(ActionReply::Operation)
            );
        }
        assert_eq!(
            replies.take(&GuiReply::Knowledge {
                op: a,
                response: serde_json::json!({})
            }),
            Some(ActionReply::Knowledge(a))
        );
    }

    // keeps: spec c1-live-seats §2a
    #[test]
    fn event_terminal_replies_release_fifo_while_status_and_gap_pushes_never_do() {
        use bbt_proto::events::{GuiEventCursor, GuiEventPage};
        let cursor = GuiEventCursor {
            stream: "account".into(),
            admitted: 7,
            acknowledged: 6,
            read: 3,
            high: 7,
            unread: 2,
        };
        let cases = [
            (
                GuiAction::GetEvents {
                    stream: "account".into(),
                    after: 0,
                    limit: 32,
                },
                GuiReply::Events {
                    page: GuiEventPage {
                        records: Vec::new(),
                        next: None,
                        cursor: cursor.clone(),
                    },
                },
                ActionReply::Events {
                    stream: "account".into(),
                },
            ),
            (
                GuiAction::GetEventCursor {
                    stream: "account".into(),
                },
                GuiReply::EventCursor {
                    cursor: cursor.clone(),
                },
                ActionReply::EventCursor,
            ),
            (
                GuiAction::MarkEventsRead {
                    stream: "account".into(),
                    seq: 3,
                },
                GuiReply::EventCursor {
                    cursor: cursor.clone(),
                },
                ActionReply::EventCursor,
            ),
            (
                GuiAction::AttachEvents { id: Ulid::new() },
                GuiReply::Ok { op: Ulid::new() },
                ActionReply::Operation,
            ),
            (
                GuiAction::EventAck {
                    stream: "account".into(),
                    seq: 6,
                },
                GuiReply::Ok { op: Ulid::new() },
                ActionReply::Operation,
            ),
        ];
        let mut replies = ActionReplies::default();
        for (action, _, _) in &cases {
            replies.sent(action);
        }
        for (_, terminal, expected) in &cases {
            assert_eq!(
                replies.take(&GuiReply::UiAsk {
                    op: Ulid::new(),
                    request: bbt_proto::gui::UiRequest::Layout
                }),
                None
            );
            for attached in [false, true] {
                assert_eq!(
                    replies.take(&GuiReply::EventStatus {
                        attached,
                        reason: (!attached).then(|| "custody held".into()),
                    }),
                    None
                );
            }
            assert_eq!(
                replies.take(&GuiReply::Gap {
                    stream: "account".into(),
                    first: 1,
                    last: 2,
                    reason: "projection bound".into(),
                }),
                None
            );
            assert_eq!(replies.take(terminal), Some(expected.clone()));
        }
        // Refusals must release the same typed slots, including read queries.
        for (action, _, expected) in &cases {
            replies.sent(action);
            assert_eq!(
                replies.take(&GuiReply::EventStatus {
                    attached: false,
                    reason: Some("held".into())
                }),
                None
            );
            assert_eq!(
                replies.take(&GuiReply::Error {
                    op: Ulid::new(),
                    reason: "Busy: fixture".into()
                }),
                Some(expected.clone())
            );
        }
        for _ in 0..128 {
            replies.sent(&GuiAction::GetEventCursor {
                stream: "account".into(),
            });
        }
        assert!(replies.full());
        for _ in 0..128 {
            assert_eq!(
                replies.take(&GuiReply::EventCursor {
                    cursor: cursor.clone()
                }),
                Some(ActionReply::EventCursor)
            );
        }
    }

    // keeps: spec c1-live-seats §2a
    #[test]
    fn disconnected_mutations_wait_for_explicit_retry_while_pane_controls_survive() {
        let seat = SeatId::from_u128(1);
        let mut outgoing = VecDeque::from([
            GuiAction::Messaging {
                id: Ulid::new(),
                swarm_id: SwarmId::from_u128(2),
                request: bbt_proto::messaging::Request::Unsubscribe {
                    path: "channels/exact/member".into(),
                },
            },
            GuiAction::Task {
                id: Ulid::new(),
                request: bbt_proto::messaging::TaskRequest::Accept {
                    task_path: ".bboxy/tasks/exact.md".into(),
                },
            },
            GuiAction::Resize {
                seat,
                generation: 1,
                cols: 80,
                rows: 24,
            },
        ]);
        discard_uncertain_actions(&mut outgoing);
        assert_eq!(outgoing.len(), 1);
        assert!(
            matches!(outgoing.front(), Some(GuiAction::Resize { seat: actual, cols: 80, rows: 24, .. }) if *actual == seat)
        );
    }

    // keeps: spec c1-live-seats §2a
    #[test]
    fn messaging_and_task_terminal_wrappers_release_only_their_own_fifo_slots() {
        use bbt_proto::messaging::{Request, Response, TaskRequest, TaskResponse};
        let messaging = Ulid::new();
        let task = Ulid::new();
        let actions = [
            GuiAction::Messaging {
                id: messaging,
                swarm_id: SwarmId::from_u128(1),
                request: Request::Unsubscribe {
                    path: "fixture/channel/link".into(),
                },
            },
            GuiAction::Task {
                id: task,
                request: TaskRequest::Accept {
                    task_path: ".bboxy/tasks/fixture.md".into(),
                },
            },
        ];
        let mut replies = ActionReplies::default();
        for action in &actions {
            replies.sent(action);
        }
        assert_eq!(
            replies.take(&GuiReply::Messaging {
                op: task,
                response: Response::Applied
            }),
            None
        );
        assert_eq!(
            replies.take(&GuiReply::Task {
                op: messaging,
                response: TaskResponse::Applied
            }),
            None
        );
        assert_eq!(
            replies.take(&GuiReply::Error {
                op: Ulid::new(),
                reason: "unrelated".into()
            }),
            None
        );
        assert_eq!(
            replies.take(&GuiReply::Ok { op: messaging }),
            None,
            "inner Applied is not an outer Ok"
        );
        assert_eq!(
            replies.take(&GuiReply::Messaging {
                op: messaging,
                response: Response::Applied
            }),
            Some(ActionReply::Messaging(messaging))
        );
        assert_eq!(
            replies.take(&GuiReply::Messaging {
                op: messaging,
                response: Response::Applied
            }),
            None,
            "the same terminal reply cannot release another slot"
        );
        assert_eq!(
            replies.take(&GuiReply::Task {
                op: task,
                response: TaskResponse::Applied
            }),
            Some(ActionReply::Task(task))
        );
        for action in &actions {
            replies.sent(action);
        }
        for (op, expected) in [
            (messaging, ActionReply::Messaging(messaging)),
            (task, ActionReply::Task(task)),
        ] {
            assert_eq!(
                replies.take(&GuiReply::Error {
                    op,
                    reason: "Busy: fixture".into()
                }),
                Some(expected)
            );
        }
    }

    // keeps: spec e2e-acceptance M6 (authoritative mute metadata must not release a request slot).
    #[test]
    fn mute_snapshot_preserves_its_receipt_and_the_next_operation() {
        let op = Ulid::new();
        let input = Ulid::new();
        let seat = SeatId::from_u128(42);
        let mut replies = ActionReplies::default();
        replies.sent(&GuiAction::Messaging {
            id: op,
            swarm_id: SwarmId::from_u128(1),
            request: bbt_proto::messaging::Request::Mute {
                seat: Ulid(42),
                muted: true,
            },
        });
        replies.sent(&GuiAction::Input {
            seat,
            generation: 1,
            id: input,
            bytes: vec![b'x'],
        });
        let mut record = bbt_proto::registry::SeatRecord::new(Ulid(42), Ulid(1), "muted", "zsh");
        record.muted = true;
        let snapshot = GuiReply::Seat(GuiSeat {
            record,
            attention: None,
            state: bbt_proto::SeatState {
                generation: 1,
                process: ProcessStatus::Running { pid: 1 },
                activity: bbt_ui_model::Activity::Idle,
                grade: bbt_ui_model::Grade::Reported,
                title: String::new(),
                detail: None,
                turn_open: false,
                observed_ms: 0,
                subagents: vec![],
            },
        });
        assert_eq!(replies.take(&snapshot), None);
        assert_eq!(
            replies.take(&GuiReply::Messaging {
                op,
                response: bbt_proto::messaging::Response::Muted { notice: None },
            }),
            Some(ActionReply::Messaging(op))
        );
        assert_eq!(
            replies.take(&GuiReply::Ok { op: input }),
            Some(ActionReply::Operation)
        );
        assert_eq!(replies.take(&GuiReply::Ok { op: input }), None);
    }

    // keeps: spec c1-live-seats §2a
    #[test]
    fn scroll_history_errors_are_correlated_behind_operations_and_async_pushes() {
        let seat = SeatId::from_u128(9);
        let mut replies = ActionReplies::default();
        replies.sent(&GuiAction::Input {
            seat,
            generation: 1,
            id: Ulid::new(),
            bytes: b"x".to_vec(),
        });
        replies.sent(&GuiAction::GetHistory {
            seat,
            from: 1,
            to: 10,
        });
        assert_eq!(
            replies.take(&GuiReply::Identity(GuiIdentity {
                state: bbt_proto::GuiIdentityState::LocalDev,
                reason: None
            })),
            None
        );
        assert_eq!(
            replies.take(&GuiReply::Error {
                op: Ulid::nil(),
                reason: "async snapshot error".into()
            }),
            None
        );
        assert_eq!(
            replies.take(&GuiReply::Error {
                op: Ulid::new(),
                reason: "Busy: input refused".into()
            }),
            Some(ActionReply::Operation)
        );
        assert_eq!(
            replies.take(&GuiReply::Error {
                op: Ulid::new(),
                reason: "Busy: history refused".into()
            }),
            Some(ActionReply::History(seat))
        );
        for _ in 0..128 {
            replies.sent(&GuiAction::ListSeats);
        }
        assert!(replies.full());
        assert_eq!(replies.take(&GuiReply::Hosts(vec![])), None);
        for _ in 0..128 {
            assert_eq!(
                replies.take(&GuiReply::Seats(vec![])),
                Some(ActionReply::Seats)
            );
        }
        assert!(!replies.full());
    }

    // keeps: spec c1-live-seats §2a
    #[test]
    fn scroll_history_reads_yield_to_already_published_applied_actions() {
        let seat = SeatId::from_u128(9);
        let mut outgoing = VecDeque::from([GuiAction::GetHistory {
            seat,
            from: 1,
            to: 10,
        }]);
        let view = control_view(seat, 1);
        queue_action(
            &mut outgoing,
            GuiAction::Applied {
                seat,
                view,
                sequence: 10,
            },
        );
        assert!(matches!(
            outgoing.pop_front(),
            Some(GuiAction::Applied { sequence: 10, .. })
        ));
        assert!(matches!(
            outgoing.pop_front(),
            Some(GuiAction::GetHistory { from: 1, .. })
        ));
    }

    fn control_view(seat: SeatId, generation: u64) -> View {
        View {
            seat: Ulid(seat.0.0),
            generation,
            view_generation: Ulid::nil(),
        }
    }

    fn offer_screen(
        grids: &mut Grids,
        seat: SeatId,
        view: View,
        sequence: u64,
        term: &bbt_core::Term,
    ) {
        grids
            .apply(
                seat,
                &Update::Snapshot {
                    snapshot: Box::new(bbt_proto::Snapshot {
                        view,
                        sequence,
                        screen: bbt_proto::Screen::from_term(term, String::new(), None, None),
                    }),
                },
            )
            .unwrap();
    }

    // keeps: spec c1-live-seats §2a
    #[test]
    fn first_refused_resize_converges_without_another_ui_resize() {
        let seat = SeatId::from_u128(42);
        let view = control_view(seat, 7);
        let now = Instant::now();
        let mut controls = PaneControls::default();
        let mut outgoing = VecDeque::new();
        let mut grids = Grids::new();
        grids.set_watched(&[(seat, view)]);
        controls.set_watched(&[(seat, view)], &mut outgoing, now);
        controls.resize(seat, 7, 37, 57, &grids, &mut outgoing);
        controls.queue_due(&grids, &mut outgoing, now);
        let watch = outgoing.pop_front().unwrap();
        assert!(matches!(watch, GuiAction::Watch { .. }));
        controls.sent(&watch, now);
        let mut term = bbt_core::Term::new(80, 24, 0);
        offer_screen(&mut grids, seat, view, 1, &term);
        controls.observed(seat, &grids);

        let sent = now + CONTROL_RETRY;
        controls.queue_due(&grids, &mut outgoing, sent);
        let first = outgoing.pop_front().unwrap();
        assert!(matches!(
            first,
            GuiAction::Resize {
                cols: 37,
                rows: 57,
                ..
            }
        ));
        controls.sent(&first, sent);
        // The fixture refuses the first Resize without changing the terminal.
        // Its server-only op is deliberately unrelated to any client input ID.
        let refusal = GuiReply::Error {
            op: Ulid::new(),
            reason: "Busy: capture admission paused".into(),
        };
        let GuiReply::Error { reason, .. } = refusal else {
            unreachable!()
        };
        controls.refused(&reason, sent);
        // A different pane can keep reporting Busy. That must not postpone an
        // already scheduled reconciliation indefinitely.
        controls.refused(&reason, sent + CONTROL_RETRY / 2);
        assert_eq!(
            (
                grids.screen(seat).unwrap().cols,
                grids.screen(seat).unwrap().rows
            ),
            (80, 24)
        );
        controls.queue_due(
            &grids,
            &mut outgoing,
            sent + CONTROL_RETRY - Duration::from_nanos(1),
        );
        assert!(outgoing.is_empty(), "admission refusal must back off");

        // Admission resumes. There is no second UI Resize or resumed event.
        controls.queue_due(&grids, &mut outgoing, sent + CONTROL_RETRY);
        let retry = outgoing.pop_front().unwrap();
        let GuiAction::Resize {
            generation,
            cols,
            rows,
            ..
        } = retry
        else {
            panic!("latest desired Resize was lost")
        };
        assert_eq!((generation, cols, rows), (7, 37, 57));
        term.resize(usize::from(cols), usize::from(rows));
        offer_screen(&mut grids, seat, view, 2, &term);
        let record = bbt_proto::registry::SeatRecord::new(view.seat, Ulid::new(), "zsh", "fixture");
        let model = model::build(std::slice::from_ref(&record), &grids, &Model::default());
        assert_eq!(
            (model.screens[&seat].cols, model.screens[&seat].rows),
            (37, 57)
        );
        assert_eq!(
            controls.deadline(&grids, &outgoing),
            None,
            "converged panes must sleep indefinitely"
        );
    }

    // keeps: spec c1-live-seats §2a
    #[test]
    fn refused_watch_retries_stay_bounded_and_coalesce_the_latest_size() {
        let now = Instant::now();
        let seat = SeatId::from_u128(42);
        let view = control_view(seat, 7);
        let mut controls = PaneControls::default();
        let mut outgoing = VecDeque::new();
        let mut grids = Grids::new();
        grids.set_watched(&[(seat, view)]);
        controls.set_watched(&[(seat, view)], &mut outgoing, now);
        // Ten virtual minutes of a paused capture path with repeated layout
        // changes. A blocked writer still owns only one queued pane control.
        for attempt in 0..6000 {
            let at = now + CONTROL_RETRY * attempt;
            controls.resize(
                seat,
                7,
                30 + (attempt % 40) as u16,
                57,
                &grids,
                &mut outgoing,
            );
            controls.queue_due(&grids, &mut outgoing, at);
            controls.queue_due(&grids, &mut outgoing, at);
            assert_eq!(outgoing.len(), 1);
            let watch = outgoing.pop_front().unwrap();
            assert!(matches!(watch, GuiAction::Watch { .. }));
            controls.sent(&watch, at);
            controls.refused("Busy: capture admission paused", at);
        }
        offer_screen(&mut grids, seat, view, 1, &bbt_core::Term::new(80, 24, 0));
        controls.queue_due(&grids, &mut outgoing, now + Duration::from_secs(600));
        assert!(matches!(
            outgoing.front(),
            Some(GuiAction::Resize {
                cols: 69,
                rows: 57,
                ..
            })
        ));
        controls.set_watched(&[], &mut outgoing, now + Duration::from_secs(600));
        assert!(outgoing.is_empty(), "Unwatch must cancel queued retries");
        assert_eq!(controls.deadline(&grids, &outgoing), None);
    }

    // keeps: spec c1-live-seats §2a
    #[test]
    fn controls_obey_logic_size_limits_and_discard_old_generations() {
        let now = Instant::now();
        let seat = SeatId::from_u128(42);
        let view = control_view(seat, 7);
        let mut controls = PaneControls::default();
        let mut outgoing = VecDeque::new();
        let mut grids = Grids::new();
        grids.set_watched(&[(seat, view)]);
        controls.set_watched(&[(seat, view)], &mut outgoing, now);
        offer_screen(&mut grids, seat, view, 1, &bbt_core::Term::new(512, 64, 0));
        controls.resize(seat, 7, u16::MAX, u16::MAX, &grids, &mut outgoing);
        assert_eq!(controls.deadline(&grids, &outgoing), None);
        controls.resize(seat, 7, 0, 0, &grids, &mut outgoing);
        controls.queue_due(&grids, &mut outgoing, now);
        assert!(matches!(
            outgoing.front(),
            Some(GuiAction::Resize {
                cols: 2,
                rows: 1,
                ..
            })
        ));
        let id = Ulid::new();
        outgoing.push_back(GuiAction::Input {
            seat,
            generation: 7,
            id,
            bytes: b"typed once\r".to_vec(),
        });
        controls.set_watched(&[(seat, control_view(seat, 8))], &mut outgoing, now);
        controls.resize(seat, 7, 40, 12, &grids, &mut outgoing);
        let next = control_view(seat, 8);
        grids.set_watched(&[(seat, next)]);
        offer_screen(&mut grids, seat, next, 1, &bbt_core::Term::new(80, 24, 0));
        let mut next_actions = VecDeque::new();
        controls.queue_due(&grids, &mut next_actions, now);
        assert!(
            next_actions.is_empty(),
            "old resize must not become new-process intent"
        );
        assert_eq!(outgoing.len(), 1);
        assert!(
            matches!(outgoing.front(), Some(GuiAction::Input {id:original,bytes,..}) if *original==id && bytes==b"typed once\r")
        );
    }

    // keeps: spec c1-live-seats §2a
    #[test]
    fn sent_control_waits_for_an_observation_or_refusal_before_retrying() {
        let now = Instant::now();
        let seat = SeatId::from_u128(42);
        let view = control_view(seat, 7);
        let mut controls = PaneControls::default();
        let mut outgoing = VecDeque::new();
        let mut grids = Grids::new();
        grids.set_watched(&[(seat, view)]);
        controls.set_watched(&[(seat, view)], &mut outgoing, now);
        controls.queue_due(&grids, &mut outgoing, now);
        let watch = outgoing.pop_front().unwrap();
        controls.sent(&watch, now);
        controls.resize(seat, 7, 37, 57, &grids, &mut outgoing);
        controls.queue_due(&grids, &mut outgoing, now + Duration::from_secs(3600));
        assert!(outgoing.is_empty(), "do not resend an unanswered Watch");
        assert_eq!(controls.deadline(&grids, &outgoing), None);
        controls.refused("StaleGeneration: process restarted", now);
        assert_eq!(
            controls.deadline(&grids, &outgoing),
            None,
            "permanent rejections must not become a busy loop"
        );
        controls.refused("Busy: capture admission paused", now);
        controls.queue_due(&grids, &mut outgoing, now + CONTROL_RETRY);
        assert!(matches!(outgoing.front(), Some(GuiAction::Watch { .. })));
    }

    // keeps: spec c1-live-seats §2a
    #[test]
    fn resize_before_a_coalesced_watch_keeps_its_desired_grid() {
        let now = Instant::now();
        let seat = SeatId::from_u128(42);
        let view = control_view(seat, 7);
        let mut controls = PaneControls::default();
        let mut outgoing = VecDeque::new();
        let mut grids = Grids::new();
        controls.resize(seat, 7, 37, 57, &grids, &mut outgoing);
        controls.queue_due(&grids, &mut outgoing, now);
        assert!(
            outgoing.is_empty(),
            "a size alone must not subscribe a hidden pane"
        );
        assert_eq!(controls.deadline(&grids, &outgoing), None);
        grids.set_watched(&[(seat, view)]);
        controls.set_watched(&[(seat, view)], &mut outgoing, now);
        controls.queue_due(&grids, &mut outgoing, now);
        controls.sent(&outgoing.pop_front().unwrap(), now);
        offer_screen(&mut grids, seat, view, 1, &bbt_core::Term::new(80, 24, 0));
        controls.observed(seat, &grids);
        controls.queue_due(&grids, &mut outgoing, now + CONTROL_RETRY);
        assert!(matches!(
            outgoing.front(),
            Some(GuiAction::Resize {
                cols: 37,
                rows: 57,
                ..
            })
        ));

        let rewatch = View {
            view_generation: Ulid::new(),
            ..view
        };
        grids.set_watched(&[(seat, rewatch)]);
        controls.set_watched(&[(seat, rewatch)], &mut outgoing, now + CONTROL_RETRY);
        assert!(
            outgoing.is_empty(),
            "a new view must cancel its queued old Resize"
        );
        controls.queue_due(&grids, &mut outgoing, now + CONTROL_RETRY);
        assert!(
            matches!(outgoing.front(), Some(GuiAction::Watch {view: token,..}) if *token==rewatch)
        );

        let next = control_view(seat, 8);
        controls.resize(seat, 8, 40, 12, &grids, &mut outgoing);
        grids.set_watched(&[(seat, next)]);
        controls.set_watched(&[(seat, next)], &mut outgoing, now + CONTROL_RETRY);
        controls.queue_due(&grids, &mut outgoing, now + CONTROL_RETRY);
        let watch = outgoing.pop_front().unwrap();
        controls.sent(&watch, now + CONTROL_RETRY);
        offer_screen(&mut grids, seat, next, 1, &bbt_core::Term::new(80, 24, 0));
        controls.observed(seat, &grids);
        controls.queue_due(&grids, &mut outgoing, now + CONTROL_RETRY * 2);
        assert!(matches!(
            outgoing.pop_front(),
            Some(GuiAction::Resize {
                generation: 8,
                cols: 40,
                rows: 12,
                ..
            })
        ));
    }

    // keeps: spec c1-live-seats §2a
    #[test]
    fn generic_spawn_preserves_inputs_and_uses_io_defaults() {
        let swarm = Ulid::from(42_u128);
        let cwd = Path::new("/fixture/repository with spaces");
        for location in [
            Where::Local,
            Where::Ssh { host: "rig".into() },
            Where::Cloud {
                provider: "provider".into(),
                host: "worker".into(),
            },
        ] {
            let message = UiMsg::Spawn {
                id: bbt_mux::Ulid::new(),
                swarm: None,
                placement: None,
                harness_kind: "codex".into(),
                name: Some("named seat".into()),
                role: Some("executor".into()),
                location: location.clone(),
            };
            let spec = spawn_spec(&message, swarm, cwd).unwrap();
            assert_eq!(spec.swarm, SwarmId::from_u128(swarm.0));
            assert_eq!(spec.harness_kind, "codex");
            assert_eq!(spec.name.as_deref(), Some("named seat"));
            assert_eq!(spec.location, location);
            assert_eq!(spec.cwd, "/fixture/repository with spaces");
            assert_eq!(spec.role.as_deref(), Some("executor"));
            assert_eq!(spec.resume, None);
            let wire = serde_json::to_value(&spec).unwrap();
            assert_eq!(serde_json::from_value::<SpawnSpec>(wire).unwrap(), spec);
        }
        let generic = UiMsg::Spawn {
            id: bbt_mux::Ulid::new(),
            swarm: None,
            placement: None,
            harness_kind: "zsh".into(),
            name: None,
            role: None,
            location: Where::Local,
        };
        assert_eq!(
            spawn_spec(&generic, swarm, cwd),
            spawn_spec(&UiMsg::SpawnZsh, swarm, cwd)
        );
        assert!(spawn_spec(&UiMsg::Watched(vec![]), swarm, cwd).is_none());
        // Unsupported kinds reach the existing backend rejection, never a
        // shell fallback or a command guessed from a label.
        let unsupported = UiMsg::Spawn {
            id: bbt_mux::Ulid::new(),
            swarm: None,
            placement: None,
            harness_kind: "pi".into(),
            name: None,
            role: None,
            location: Where::Local,
        };
        assert_eq!(
            spawn_spec(&unsupported, swarm, cwd).unwrap().harness_kind,
            "pi"
        );
    }
    use std::{os::unix::fs::PermissionsExt, time::Duration};

    // keeps: spec c1-live-seats §2a
    #[test]
    fn queued_unwatch_after_a_rewatch_is_not_lost_to_deduplication() {
        let seat = SeatId::from_u128(42);
        let view = bbt_proto::View {
            seat: Ulid(seat.0.0),
            generation: 2,
            view_generation: Ulid::new(),
        };
        let mut outgoing = VecDeque::new();
        queue_action(&mut outgoing, GuiAction::Unwatch { seat });
        queue_action(&mut outgoing, GuiAction::Watch { seat, view });
        queue_action(&mut outgoing, GuiAction::Unwatch { seat });
        queue_action(&mut outgoing, GuiAction::Unwatch { seat });
        assert_eq!(outgoing.len(), 3);
        assert!(matches!(
            outgoing.pop_front(),
            Some(GuiAction::Unwatch { .. })
        ));
        assert!(
            matches!(outgoing.pop_front(), Some(GuiAction::Watch { view: token, .. }) if token == view)
        );
        assert!(matches!(
            outgoing.pop_front(),
            Some(GuiAction::Unwatch { .. })
        ));
    }

    struct NoWake;
    impl bbt_rt::Waker for NoWake {
        fn wake(&self) {}
    }

    // keeps: spec c1-live-seats §2a
    #[test]
    fn empty_page_completions_publish_and_refusals_complete_only_the_matched_stream() {
        use bbt_proto::events::{GuiEventCursor, GuiEventPage};
        let dirty = bbt_rt::DirtySet::new(1, Arc::new(NoWake));
        let updates = Latest::new(dirty.source(0));
        let mut model = Arc::new(Model::default());
        let page = GuiEventPage {
            records: vec![],
            next: None,
            cursor: GuiEventCursor {
                stream: "swarm-exact".into(),
                admitted: 0,
                acknowledged: 0,
                read: 0,
                high: 0,
                unread: 0,
            },
        };
        for revision in 1..=2 {
            publish_page(&mut model, &updates, page.clone()).unwrap();
            assert_eq!(
                updates
                    .take()
                    .expect("empty completion published")
                    .events
                    .page_snapshots()["swarm-exact"]
                    .revision,
                revision
            );
        }
        let mut replies = ActionReplies::default();
        replies.sent(&GuiAction::GetEvents {
            stream: "other".into(),
            after: 42,
            limit: 128,
        });
        for push in [
            GuiReply::EventStatus {
                attached: true,
                reason: None,
            },
            GuiReply::Gap {
                stream: "other".into(),
                first: 1,
                last: 2,
                reason: "not a read reply".into(),
            },
        ] {
            let terminal = replies.take(&push);
            publish_page_refusal(&mut model, &updates, terminal.as_ref(), "unrelated").unwrap();
            assert!(updates.take().is_none());
        }
        let terminal = replies.take(&GuiReply::Error {
            op: Ulid::new(),
            reason: "HistoryDenied".into(),
        });
        publish_page_refusal(&mut model, &updates, terminal.as_ref(), "HistoryDenied").unwrap();
        let published = updates.take().unwrap();
        assert_eq!(
            published.events.page_errors()["other"].reason,
            "HistoryDenied"
        );
        assert_eq!(published.events.page_snapshots()["swarm-exact"].revision, 2);
        assert!(!published.events.page_errors().contains_key("swarm-exact"));
    }

    // keeps: spec c1-live-seats §2a
    #[test]
    fn waiting_attention_is_published_before_ack_but_history_pages_are_passive() {
        use bbt_proto::events::*;
        let make_row = |sequence, activity| {
            let seat = GuiSeat {
                attention: None,
                record: bbt_proto::SeatRecord::new(Ulid(3), Ulid(1), "question fixture", "claude"),
                state: bbt_proto::SeatState {
                    generation: 1,
                    process: ProcessStatus::Running { pid: 9 },
                    activity,
                    grade: bbt_proto::Grade::Shim,
                    title: String::new(),
                    detail: Some("May I continue?".into()),
                    turn_open: true,
                    observed_ms: sequence,
                    subagents: vec![],
                },
            };
            GuiEventRecord::Event(Box::new(GuiEventEnvelope {
                metadata: EventMetadata {
                    reference: EventReference {
                        position: EventPosition {
                            stream: "account".into(),
                            sequence,
                        },
                        hash: format!("{sequence:064x}"),
                    },
                    id: Ulid(u128::from(sequence)),
                    ts: sequence,
                    subject: "seat/fixture/state".into(),
                    source: "fixture".into(),
                    seat: Some(Ulid(3)),
                    generation: Some(1),
                    proof: EventProof {
                        scope: EventSignatureScope::OriginalRetainedEnvelope,
                        public_key: "fixture reference".into(),
                        signature: "not independently verified by display cache".into(),
                        prev: String::new(),
                    },
                },
                projected_at: sequence,
                cursor: None,
                payload: GuiEventPayload::Seat {
                    seat: Box::new(seat),
                },
            }))
        };
        let dirty = bbt_rt::DirtySet::new(1, Arc::new(NoWake));
        let updates = Latest::new(dirty.source(0));
        let mut model = Arc::new(Model::default());
        let waiting = make_row(
            1,
            bbt_proto::Activity::Waiting(bbt_proto::WaitKind::Question),
        );
        let ack = publish_delivery(&mut model, &updates, waiting.clone()).unwrap();
        assert!(matches!(ack, Some(GuiAction::EventAck { seq: 1, .. })));
        let published = updates.take().unwrap();
        assert!(published.attention.contains_key(&SeatId::from_u128(3)));
        assert!(published.ui.window.stage.focused.is_none());
        let episode = published.attention_key(SeatId::from_u128(3));
        publish_delivery(&mut model, &updates, waiting.clone()).unwrap();
        assert!(updates.take().is_none());
        assert_eq!(model.attention_key(SeatId::from_u128(3)), episode);
        publish_delivery(&mut model, &updates, make_row(2, bbt_proto::Activity::Idle)).unwrap();
        assert!(updates.take().unwrap().attention.is_empty());
        let mut history = Arc::new(Model::default());
        event_model(&mut history)
            .apply_page(GuiEventPage {
                records: vec![waiting],
                next: None,
                cursor: GuiEventCursor {
                    stream: "account".into(),
                    admitted: 1,
                    acknowledged: 1,
                    read: 0,
                    high: 1,
                    unread: 0,
                },
            })
            .unwrap();
        assert!(
            history.attention.is_empty(),
            "reading a historical wait must never notify"
        );
    }

    // keeps: spec c1-live-seats §2a
    #[test]
    fn event_ack_follows_shared_model_publication_and_replays_do_not_publish_twice() {
        let dirty = bbt_rt::DirtySet::new(1, Arc::new(NoWake));
        let updates = Latest::new(dirty.source(0));
        let mut model = Arc::new(Model::default());
        let gap = GuiEventRecord::Gap(EventGap {
            stream: "index".into(),
            first: 7,
            last: 7,
            reason: "projection bound; retained".into(),
        });
        let ack = publish_delivery(&mut model, &updates, gap.clone()).unwrap();
        assert!(matches!(ack, Some(GuiAction::EventAck { seq: 7, .. })));
        let published = updates
            .take()
            .expect("publication precedes returning the ACK");
        assert_eq!(published.events.records().len(), 1);
        assert!(Arc::ptr_eq(&model, &published));
        assert!(published.ui.window.stage.focused.is_none());
        assert!(published.frames.is_empty());
        assert!(
            publish_delivery(&mut model, &updates, gap)
                .unwrap()
                .is_some()
        );
        assert!(
            updates.take().is_none(),
            "published duplicate is ACKed without another dirty frame"
        );
        assert!(
            publish_delivery(
                &mut model,
                &updates,
                GuiEventRecord::Gap(EventGap {
                    stream: "index".into(),
                    first: 8,
                    last: 7,
                    reason: "invalid".into()
                })
            )
            .is_err()
        );
        assert!(updates.take().is_none());
    }

    #[allow(
        clippy::disallowed_methods,
        reason = "test/fixture IO, not the UI thread"
    )]
    #[ignore = "night: real IO/process/PTY boundary, test-policy #327 B2"]
    // keeps: spec c1-live-seats §2a
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn live_e2e_and_ctl_share_one_corpus_and_refresh_from_document_events() {
        use bbt_ui::modules::e2e_checklist::Tone;
        use serde_json::{Value, json};
        let promises = knowledge::promises().unwrap();
        let terminal = promises.iter().filter(|p| p.group == "terminal").count();
        let root = tempfile::Builder::new()
            .prefix("a3-live-")
            .tempdir()
            .unwrap();
        let repository = root.path().join("repository");
        let run_dir = root.path().join("run");
        for name in ["e2e", "tasks"] {
            tokio::fs::create_dir_all(repository.join(".bboxy").join(name))
                .await
                .unwrap();
        }
        let task = repository.join(".bboxy/tasks/tracked.md");
        let task_text =
            "---\nnumber: 1\ntype: task\ntitle: A3 fixture task\nstatus: running\n---\n";
        tokio::fs::write(&task, task_text).await.unwrap();
        let promises = knowledge::promises().unwrap();
        for (i, promise) in promises.iter().enumerate() {
            let row = json!({"number":i+2,"type":"e2e","function":promise.id,
                "status":if promise.id=="T1" {"fail"} else if promise.id=="S3" {"manual"} else {"pass"},"coverage":"full",
                "last_run":"2026-10-24T00:00:00Z","run_by":"fixture-seat",
                "links":if promise.id=="T1" {json!([{"rel":"tracked-by","to":"#1"}])} else {json!([])}});
            let text = format!(
                "---\n{}\n---\nFixture only.\n",
                serde_json::to_string_pretty(&row).unwrap()
            );
            tokio::fs::write(
                repository.join(format!(".bboxy/e2e/{}.md", promise.id)),
                text,
            )
            .await
            .unwrap();
        }
        let binary = std::env::current_exe()
            .unwrap()
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("bboxy450-term");
        assert!(
            binary.is_file(),
            "build the app before its real-process test"
        );
        let executable = root.path().join("test-app");
        tokio::fs::write(
            &executable,
            format!(
                "#!/bin/sh\nBBOXY_TEST_CUSTODY=memory exec '{}' \"$@\"\n",
                binary.to_string_lossy().replace('\'', "'\\''")
            ),
        )
        .await
        .unwrap();
        tokio::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700))
            .await
            .unwrap();
        let session = Session::start(cockpit::Config {
            executable,
            socket: run_dir.join("mux.sock"),
            run_dir: run_dir.clone(),
            repository: repository.clone(),
            shell_config: None,
            ctl: true,
            test_account: None,
        })
        .await
        .unwrap();
        let owned = (
            session.supervisor.id().unwrap(),
            session.logic.id().unwrap(),
        );
        tokio::fs::write(
            root.path().join("owned-pids.log"),
            format!("supervisor={} logic={}\n", owned.0, owned.1),
        )
        .await
        .unwrap();
        eprintln!("A3 owned supervisor={} logic={}", owned.0, owned.1);
        let context = Arc::new(DeviceContext::new().await.unwrap());
        let (updates, publication) = publications();
        let (sender, commands) = command_channel();
        let (render, rendering) = oneshot::channel();
        render
            .send(RenderContext {
                context,
                scale: 1.,
                theme: bbt_ui_model::tokens::Theme::Dark,
            })
            .ok()
            .unwrap();
        let task_updates = updates.clone();
        let io = tokio::spawn(async move {
            run(
                task_updates,
                commands,
                Logger::new(),
                session,
                repository,
                rendering,
                |_| {},
                |_| {},
            )
            .await
        });
        let mut current = Arc::new(Model::default());
        sender.observe_knowledge(true);
        until(&updates, &publication, &mut current, |m| {
            m.e2e
                .as_ref()
                .is_some_and(|e| e.records.len() == promises.len() && e.faults.is_empty())
        })
        .await;
        assert_eq!(
            current.e2e.as_ref().unwrap().project().summary.tone,
            Tone::Amber,
            "{:?}",
            current.e2e.as_ref().unwrap().project().summary
        );
        let stage = current.ui.window.stage.clone();
        let ctl = |group: Option<&str>| {
            let binary = binary.clone();
            let run_dir = run_dir.clone();
            let group = group.map(str::to_owned);
            tokio::task::spawn_blocking(move || {
                let mut command = std::process::Command::new(binary);
                command.args(["ctl", "e2e"]).env("BBOXY_RUN_DIR", run_dir);
                if let Some(group) = group {
                    command.args(["--group", &group]);
                }
                let output = command.output().unwrap();
                let lines = String::from_utf8(output.stdout)
                    .unwrap()
                    .lines()
                    .map(|l| serde_json::from_str::<Value>(l).unwrap())
                    .collect::<Vec<_>>();
                (output.status.success(), lines)
            })
        };
        let (ok, lines) = ctl(None).await.unwrap();
        assert!(ok, "{lines:?}");
        assert_eq!(
            lines,
            knowledge::e2e_lines(current.e2e.as_ref().unwrap(), None).unwrap()
        );
        assert_eq!(lines[0]["last_run"], "2026-10-24T00:00:00Z");
        assert_eq!(lines[0]["run_by"], "fixture-seat");
        let (ok, group) = ctl(Some("terminal")).await.unwrap();
        assert!(ok);
        assert_eq!(group.len(), terminal + 1);
        assert_eq!(
            group.last(),
            lines.last(),
            "group filters rows, never the global summary"
        );
        assert!(!ctl(Some("missing-group")).await.unwrap().0);

        tokio::fs::write(&task, task_text.replace("running", "accepted"))
            .await
            .unwrap();
        until(&updates, &publication, &mut current, |m| {
            m.e2e
                .as_ref()
                .is_some_and(|e| e.faults.is_empty() && e.project().summary.untracked == 1)
        })
        .await;
        assert_eq!(
            current.e2e.as_ref().unwrap().project().summary.tone,
            Tone::Red
        );
        let failed = root.path().join("repository/.bboxy/e2e/T1.md");
        let text = tokio::fs::read_to_string(&failed).await.unwrap();
        let passed = text.replace("\"status\": \"fail\"", "\"status\": \"pass\"");
        tokio::fs::write(&failed, &passed).await.unwrap();
        until(&updates, &publication, &mut current, |m| {
            m.e2e
                .as_ref()
                .is_some_and(|e| e.project().summary.tone == Tone::Green)
        })
        .await;
        let (_, lines) = ctl(None).await.unwrap();
        assert_eq!(
            lines,
            knowledge::e2e_lines(current.e2e.as_ref().unwrap(), None).unwrap()
        );
        assert_eq!(
            stage, current.ui.window.stage,
            "knowledge cannot place or focus a pane"
        );

        tokio::fs::write(
            &failed,
            passed.replace("\"type\": \"e2e\"", "\"type\": \"task\""),
        )
        .await
        .unwrap();
        until(&updates, &publication, &mut current, |m| {
            m.e2e.as_ref().is_some_and(|e| !e.faults.is_empty())
        })
        .await;
        assert_eq!(
            current.e2e.as_ref().unwrap().project().summary.tone,
            Tone::Red
        );
        let (_, lines) = ctl(None).await.unwrap();
        assert_eq!(
            lines.last().unwrap()["state"],
            "red",
            "bad provider rows never leave stale green"
        );
        sender.observe_knowledge(false);
        tokio::fs::write(&failed, &passed).await.unwrap();
        sender.observe_knowledge(true);
        until(&updates, &publication, &mut current, |m| {
            m.e2e
                .as_ref()
                .is_some_and(|e| e.project().summary.tone == Tone::Green)
        })
        .await;
        drop(sender);
        tokio::time::timeout(Duration::from_secs(30), io)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        eprintln!(
            "PASS A3: one live record per promise; screen == ctl JSON; terminal group + global summary; doc/task wakes amber -> red -> green; provider fault red; close/reopen recovers; children reaped"
        );
    }

    fn lines(screen: &bbt_proto::Screen) -> Vec<String> {
        screen
            .cells
            .iter()
            .map(|row| {
                row.iter()
                    .map(|cell| cell.c)
                    .collect::<String>()
                    .trim_end()
                    .to_owned()
            })
            .collect()
    }
    struct Publications {
        dirty: bbt_rt::DirtySet,
        ready: Arc<tokio::sync::Notify>,
    }
    struct PublicationWake(Arc<tokio::sync::Notify>);
    impl bbt_rt::Waker for PublicationWake {
        fn wake(&self) {
            self.0.notify_one();
        }
    }
    fn publications() -> (Arc<Latest<Arc<Model>>>, Publications) {
        let ready = Arc::new(tokio::sync::Notify::new());
        let dirty = bbt_rt::DirtySet::new(1, Arc::new(PublicationWake(ready.clone())));
        (
            Arc::new(Latest::new(dirty.source(0))),
            Publications { dirty, ready },
        )
    }
    async fn until(
        updates: &Latest<Arc<Model>>,
        publication: &Publications,
        current: &mut Arc<Model>,
        check: impl Fn(&Model) -> bool,
    ) {
        tokio::time::timeout(Duration::from_secs(30), async {
            loop {
                let ready = publication.ready.notified();
                publication.dirty.drain();
                if let Some(next) = updates.take() {
                    *current = next;
                }
                if check(current) {
                    break;
                }
                ready.await;
            }
        })
        .await
        .unwrap_or_else(|_| {
            panic!(
                "model condition timed out; e2e: {:?}; screens: {:?}",
                current
                    .e2e
                    .as_ref()
                    .map(|e| (&e.faults, e.records.len(), e.project().summary)),
                current
                    .screens
                    .iter()
                    .map(|(id, screen)| (id, lines(screen)))
                    .collect::<Vec<_>>()
            )
        });
    }
    async fn input(sender: &CommandSender, seat: SeatId, generation: u64, bytes: &[u8]) {
        sender
            .send(UiMsg::Input {
                seat,
                generation,
                id: Ulid::new(),
                bytes: bytes.into(),
            })
            .await
            .unwrap();
    }

    async fn messaging_through_the_one_io_owner(
        sender: &CommandSender,
        updates: &Latest<Arc<Model>>,
        publication: &Publications,
        current: &mut Arc<Model>,
        seat: SeatId,
        task_id: Ulid,
        task_path: &str,
    ) {
        use bboxy450_term::messaging::{Client, History, Records};
        use bbt_ui::modules::{composer, stream_dock::Entry};
        let scope = current.ui.seats[&seat].swarm;
        let mut editor = composer::Controller::new(composer::Composer {
            focus: true,
            kind: "decision".into(),
            to: vec![composer::Address {
                kind: "channel".into(),
                destination: format!("swarms/{scope}"),
                label: "Actual selected swarm".into(),
                ..Default::default()
            }],
            ..Default::default()
        });
        editor.handle(composer::Command::Edit(bbt_ui::text_edit::Command::Insert(
            "Decision through the production GUI IO owner".into(),
        )));
        let card = bbt_proto::messaging::TaskCard {
            id: task_id,
            path: task_path.into(),
            title: "Actual task attachment".into(),
            status: "open".into(),
            assignee: None,
            delegated_by: None,
            updated: None,
            verify: None,
        };
        editor.handle(composer::Command::Attach(
            bboxy450_term::messaging::task_attachment(&card),
        ));
        editor.handle(composer::Command::Attach(composer::Attachment {
            kind: "record".into(),
            path: "[[#1]]".into(),
            title: "#1".into(),
            ..Default::default()
        }));

        let composer::Outcome::Submit(draft) = editor.handle(composer::Command::Send) else {
            panic!("committed decision");
        };
        let mut client = Client::default();
        let batch = client.prepare(Some(scope), draft).unwrap();
        let command = client.admitted(batch).pop().unwrap();
        let UiMsg::Messaging { id, .. } = &command else {
            unreachable!()
        };
        let id = *id;
        let stage = current.ui.window.stage.clone();
        sender.send(command).await.unwrap();
        until(updates, publication, current, |model| model.operation_results.rows().any(|result| result.op == id)
            && model.events.records().iter().any(|record| matches!(record.as_ref(), GuiEventRecord::Event(event) if matches!(&event.payload, GuiEventPayload::Message {row} if row.id == id)))).await;
        let committed = client
            .replies(&current.operation_results)
            .0
            .expect("actual Stored result");
        editor.stored(&committed);
        assert!(editor.view.text.is_empty());
        assert_eq!(
            stage, current.ui.window.stage,
            "message pushes cannot place or focus a pane"
        );
        let mut history = History::default();
        history.select(scope);
        history.observe(&current.events);
        let stream = history.stream().unwrap();
        let revision = current
            .events
            .page_snapshots()
            .get(&stream)
            .map_or(0, |page| page.revision);
        for action in history.actions() {
            sender.send(action.into()).await.unwrap();
        }
        until(updates, publication, current, |model| {
            model
                .events
                .page_snapshots()
                .get(&stream)
                .is_some_and(|page| page.revision > revision)
        })
        .await;
        history.observe(&current.events);
        let mut list_ops = vec![];
        for command in history.commands() {
            if let UiMsg::Messaging { id, .. } = &command {
                list_ops.push(*id);
            }
            sender.send(command).await.unwrap();
        }
        until(updates, publication, current, |model| {
            list_ops
                .iter()
                .all(|op| model.operation_results.rows().any(|r| r.op == *op))
        })
        .await;
        history.replies(&current.operation_results);
        let entries = history.entries(current, client.groups(), &[]);
        assert!(entries.iter().any(|entry|matches!(entry,Entry::Message {data,..} if data.attach.iter().any(|a|a.path=="[[#1]]"))));
        let conversation = bbt_ui::modules::composer::Conversation {
            stream: bbt_ui::modules::stream_dock::StreamDock {
                entries: entries.clone(),
                dock: true,
                ..Default::default()
            },
            composer: editor.view.clone(),
            scroll: 0.,
        };
        let layout = crate::scene::Scene::new()
            .conversation_layout([1400., 1000.], &conversation)
            .unwrap();
        fn attachment(
            n: &bbt_ui::components::Layout,
            index: usize,
        ) -> Option<bbt_ui_model::layout::Rect> {
            if matches!(&n.kind,bbt_ui::components::LayoutKind::Named(key) if key.ends_with(&format!("/body/attach/a{index}")))
            {
                Some(n.rect)
            } else {
                n.children.iter().find_map(|child| attachment(child, index))
            }
        }
        let mut document = documents::Host::default();
        for (index, expected) in [format!("[[{task_path}]]"), "[[#1]]".into()]
            .iter()
            .enumerate()
        {
            let r = attachment(&layout.layout, index).expect("live attachment card");
            let bbt_ui::modules::stream_dock::Action::OpenRecord(target) = layout
                .actions
                .hit([r.x + r.w / 2., r.y + r.h / 2.])
                .expect("live attachment action")
            else {
                panic!("attachment must open a record");
            };
            assert_eq!(
                &target, expected,
                "plain task paths wrap once; bracketed #N stays intact"
            );
            document.show(target);
            for _ in 0..2 {
                let (id, read) = document.next_read().expect("resolve then fetch");
                sender.send(UiMsg::DocumentRead { id, read }).await.unwrap();
                until(updates, publication, current, |model| {
                    model.document_reply.as_ref().is_some_and(|r| r.id == id)
                })
                .await;
                assert!(document.apply(current.document_reply.as_ref().unwrap()));
                sender.send(UiMsg::ConsumeDocument { id }).await.unwrap();
            }
            assert!(document.open);
            assert_eq!(document.prepared.as_ref().unwrap().document.number, Some(1));
        }
        eprintln!(
            "PASS composed task attachment and retained #N -> retained hit -> document modal -> Knowledge resolve/fetch"
        );
        assert!(history.entries(current, client.groups(), &[]).iter().any(|entry| matches!(entry, Entry::Message {data,..} if data.text == "Decision through the production GUI IO owner" && data.kind.as_deref() == Some("decision") && data.issuer_kind.as_deref() == Some("local-dev") && data.unread)));
        assert!(
            history.actions().is_empty(),
            "receive, publication and transport ACK do not mark read"
        );
        document.close();
        history.observe(&current.events);
        let action = history
            .viewed_all()
            .expect("explicit view advances the durable read-mark");
        let bboxy450_term::messaging::HistoryAction::Read { sequence: read, .. } = &action else {
            panic!("explicit view sends a read mark");
        };
        let read = *read;
        sender.send(action.into()).await.unwrap();
        until(updates, publication, current, |model| {
            model
                .events
                .cursors()
                .get(&stream)
                .is_some_and(|cursor| cursor.read >= read)
        })
        .await;
        history.observe(&current.events);
        assert!(history.entries(current, client.groups(), &[]).iter().any(|entry| matches!(entry, Entry::Message {data,..} if data.text == "Decision through the production GUI IO owner" && !data.unread)));
        eprintln!(
            "PASS compose/send -> one IO owner -> published MessageRow; only confirmed operator-read clears its dot"
        );

        let op = Ulid::new();
        sender
            .send(UiMsg::Task {
                id: op,
                request: bbt_proto::messaging::TaskRequest::Delegate {
                    task_path: task_path.into(),
                    seat_id: Ulid::from(seat.0.0),
                    role: None,
                },
            })
            .await
            .unwrap();
        until(updates, publication, current, |model| model.operation_results.rows().any(|result| result.op == op)
            && model.events.records().iter().any(|record| matches!(record.as_ref(), GuiEventRecord::Event(event) if matches!(&event.payload, GuiEventPayload::Activity {summary} if event.metadata.subject.starts_with("task/") && summary.id == Some(task_id) && summary.status.as_deref() == Some("claimed"))))).await;
        let notices = history.observe(&current.events);
        assert!(
            notices.contains(&Some(task_id)),
            "the one lane carries canonical task status"
        );
        let mut records = Records::default();
        records.select(scope);
        for id in notices {
            records.task_notice(id);
        }
        let mut pending = Vec::new();
        for command in records.commands() {
            if let UiMsg::Task { id, .. } | UiMsg::Messaging { id, .. } = &command {
                pending.push(*id);
            }
            sender.send(command).await.unwrap();
        }
        until(updates, publication, current, |model| {
            pending.iter().all(|op| {
                model
                    .operation_results
                    .rows()
                    .any(|result| result.op == *op)
            })
        })
        .await;
        records.replies(&current.operation_results);
        let card = records
            .task(task_id)
            .expect("TaskCard refreshed from the existing TaskRequest API");
        assert_eq!(card.status, "claimed");
        assert!(records.entries(current, None).iter().any(|entry| matches!(entry, Entry::Task {data,..} if data.id == task_id.to_string() && data.state == "claimed")));
        eprintln!(
            "PASS one task-status event -> existing TaskCard read -> canonical claimed task row"
        );
    }

    /// Scroll through the real admitted socket, IO replay and prepared model.
    /// The fixture starts with numbered output, so the capture's first row is
    /// line 1 rather than a shell prompt or an echoed test command.
    #[ignore = "night: real IO/process/PTY boundary, test-policy #327 B2"]
    // keeps: spec c1-live-seats §2a
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[allow(
        clippy::disallowed_methods,
        reason = "headless scroll integration owns its runtime, Metal device and child processes"
    )]
    async fn real_io_scrollback_reaches_first_line_joins_live_and_releases_on_input() {
        let root = tempfile::Builder::new()
            .prefix("c1c3-io-")
            .tempdir()
            .unwrap();
        let repository = root.path().join("repository");
        let shell_config = root.path().join("zsh-config");
        tokio::fs::create_dir(&repository).await.unwrap();
        tokio::fs::create_dir(&shell_config).await.unwrap();
        tokio::fs::write(
            shell_config.join(".zshrc"),
            "for ((i=1;i<=20000;i++)); do printf 'line %05d\\r\\n' \"$i\"; done\nexec /bin/cat\n",
        )
        .await
        .unwrap();
        let binary = std::env::current_exe()
            .unwrap()
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("bboxy450-term");
        assert!(
            binary.is_file(),
            "build the app before its real-process scroll test"
        );
        let executable = root.path().join("test-app");
        tokio::fs::write(
            &executable,
            format!(
                "#!/bin/sh\nBBOXY_TEST_CUSTODY=memory exec '{}' \"$@\"\n",
                binary.to_string_lossy().replace('\'', "'\\''")
            ),
        )
        .await
        .unwrap();
        tokio::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700))
            .await
            .unwrap();
        let session = Session::start(cockpit::Config {
            executable,
            socket: root.path().join("run/mux.sock"),
            run_dir: root.path().join("run"),
            repository: repository.clone(),
            shell_config: Some(shell_config),
            ctl: false,
            test_account: None,
        })
        .await
        .unwrap();
        let pids = format!(
            "owned scroll supervisor={} logic={}\n",
            session.supervisor.id().unwrap(),
            session.logic.id().unwrap()
        );
        tokio::fs::write(root.path().join("owned-pids.log"), &pids)
            .await
            .unwrap();
        eprint!("{pids}");
        let context = Arc::new(DeviceContext::new().await.unwrap());
        let (updates, publication) = publications();
        let (sender, commands) = command_channel();
        let (render, rendering) = oneshot::channel();
        render
            .send(RenderContext {
                context,
                scale: 1.,
                theme: bbt_ui_model::tokens::Theme::Dark,
            })
            .ok()
            .unwrap();
        let task_updates = updates.clone();
        let task = tokio::spawn(async move {
            run(
                task_updates,
                commands,
                Logger::new(),
                session,
                repository,
                rendering,
                |reason| {
                    assert!(reason.is_none(), "unexpected backend loss: {reason:?}");
                },
                |_| {},
            )
            .await
        });
        sender.send(UiMsg::SpawnZsh).await.unwrap();
        let mut current = Arc::new(Model::default());
        until(&updates, &publication, &mut current, |m| {
            m.ui.seats.len() == 1
        })
        .await;
        let (&seat, metadata) = current.ui.seats.first_key_value().unwrap();
        let view = View {
            seat: Ulid(seat.0.0),
            generation: metadata.process_generation,
            view_generation: Ulid::new(),
        };
        sender
            .send(UiMsg::Watched(vec![(seat, view)]))
            .await
            .unwrap();
        until(&updates, &publication, &mut current, |m| {
            m.screens
                .get(&seat)
                .is_some_and(|s| lines(s).iter().any(|line| line == "line 20000"))
        })
        .await;
        let live = current.screens[&seat].clone();
        let live_frame = current.frames[&seat].clone();
        let started = Instant::now();
        sender
            .send(UiMsg::Scroll {
                seat,
                generation: view.generation,
                request: bboxy450_term::scrollback::Request::top(),
            })
            .await
            .unwrap();
        until(&updates, &publication, &mut current, |m| {
            m.history
                .get(&seat)
                .is_some_and(|h| lines(&h.screen)[0] == "line 00001")
        })
        .await;
        assert_eq!(
            current.screens[&seat].cells, live.cells,
            "historical publication must preserve the live Replica"
        );
        assert!(
            !Arc::ptr_eq(&live_frame, &current.frames[&seat]),
            "the published frame must prepare the historical cells"
        );
        eprintln!(
            "PASS actual GetHistory/IO/Metal preparation: first row line1 after {:?}",
            started.elapsed()
        );
        sender
            .send(UiMsg::Scroll {
                seat,
                generation: view.generation,
                request: bboxy450_term::scrollback::Request {
                    anchor: bboxy450_term::scrollback::Anchor::Live,
                    lines: 1,
                    op: None,
                },
            })
            .await
            .unwrap();
        until(&updates, &publication, &mut current, |m| {
            m.history
                .get(&seat)
                .is_some_and(|h| h.screen.cells[1..] == live.cells[..live.rows - 1])
        })
        .await;
        assert_eq!(current.history[&seat].screen.modes, live.modes);
        eprintln!(
            "PASS actual capture-prefix seek: historical/live rows join without a duplicated or missing row"
        );
        input(&sender, seat, view.generation, b"after scroll\r").await;
        until(&updates, &publication, &mut current, |m| {
            !m.history.contains_key(&seat)
                && lines(&m.screens[&seat])
                    .iter()
                    .any(|line| line == "after scroll")
        })
        .await;
        // A fresh historical viewport is also discarded when the pane is no
        // longer watched; the screen and parser are never retained by UI input.
        sender
            .send(UiMsg::Scroll {
                seat,
                generation: view.generation,
                request: bboxy450_term::scrollback::Request::top(),
            })
            .await
            .unwrap();
        until(&updates, &publication, &mut current, |m| {
            m.history.contains_key(&seat)
        })
        .await;
        sender.send(UiMsg::Watched(vec![])).await.unwrap();
        until(&updates, &publication, &mut current, |m| {
            m.history.is_empty() && m.screens.is_empty()
        })
        .await;
        eprintln!("PASS input returns live and unwatch releases historical display");
        drop(sender);
        tokio::time::timeout(Duration::from_secs(30), task)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        eprintln!("PASS scroll integration shutdown: owned children reaped");
    }

    /// Exercise the same IO owner the native app starts, with real children,
    /// a real PTY, the Replica and worker preparation (no synthetic Updates).
    #[ignore = "night: real IO/process/PTY boundary, test-policy #327 B2"]
    // keeps: spec c1-live-seats §2a
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[allow(
        clippy::disallowed_methods,
        reason = "headless integration test owns its runtime and child processes"
    )]
    async fn real_io_echo_resize_reflow_and_logic_loss_retains_last_frame() {
        let root = tempfile::Builder::new()
            .prefix("c1i-io-")
            .tempdir()
            .unwrap();
        let repository = root.path().join("repository");
        let shell_config = root.path().join("zsh-config");
        tokio::fs::create_dir(&repository).await.unwrap();
        let task_id = Ulid::new();
        let task_path = format!(".bboxy/tasks/{task_id}.md");
        tokio::fs::create_dir_all(repository.join(".bboxy/tasks"))
            .await
            .unwrap();
        tokio::fs::write(repository.join(&task_path), format!("---\nid: {task_id}\ntype: task\nnumber: 1\ntitle: Production GUI pump fixture\nstatus: open\nverify: 'true'\n---\nTask delegation through the same event lane.\n")).await.unwrap();
        tokio::fs::create_dir(&shell_config).await.unwrap();
        tokio::fs::write(shell_config.join(".zshrc"), "PROMPT='> '\nRPROMPT=''\n")
            .await
            .unwrap();
        let binary = std::env::current_exe()
            .unwrap()
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("bboxy450-term");
        assert!(
            binary.is_file(),
            "build the app binary before this real-process test"
        );
        let executable = root.path().join("test-app");
        tokio::fs::write(
            &executable,
            format!(
                "#!/bin/sh\nif [ \"$1\" = logic ]; then\n  if [ -e '{marker}' ]; then\n    echo $$ > '{held_pid}'\n    exec /bin/cat >/dev/null\n  fi\n  : > '{marker}'\nfi\nBBOXY_TEST_CUSTODY=memory exec '{binary}' \"$@\"\n",
                marker = root.path().join("logic-started").display(),
                held_pid = root.path().join("held-replacement-pid").display(),
                binary = binary.to_string_lossy().replace('\'', "'\\''")
            ),
        )
        .await
        .unwrap();
        tokio::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700))
            .await
            .unwrap();
        let session = Session::start(cockpit::Config {
            executable,
            socket: root.path().join("run/mux.sock"),
            run_dir: root.path().join("run"),
            repository: repository.clone(),
            shell_config: Some(shell_config),
            ctl: false,
            test_account: None,
        })
        .await
        .unwrap();
        let supervisor_pid = session.supervisor.id().unwrap();
        let logic_pid = session.logic.id().unwrap();
        let pids = format!("owned supervisor={supervisor_pid} logic={logic_pid}\n");
        tokio::fs::write(root.path().join("owned-pids.log"), &pids)
            .await
            .unwrap();
        eprint!("{pids}");
        let context = Arc::new(DeviceContext::new().await.unwrap());
        let (updates, publication) = publications();
        let (sender, commands) = command_channel();
        let (render, rendering) = oneshot::channel();
        render
            .send(RenderContext {
                context,
                scale: 1.,
                theme: bbt_ui_model::tokens::Theme::Dark,
            })
            .ok()
            .unwrap();
        let (lost_sender, mut lost_receiver) = mpsc::unbounded_channel();
        let task_updates = updates.clone();
        let task = tokio::spawn(async move {
            run(
                task_updates,
                commands,
                Logger::new(),
                session,
                repository,
                rendering,
                |reason| {
                    let _ = lost_sender.send(reason);
                },
                |_| {},
            )
            .await
        });
        let mut current = Arc::new(Model::default());
        sender
            .send(UiMsg::Spawn {
                id: bbt_mux::Ulid::new(),
                swarm: None,
                placement: None,
                harness_kind: "pi".into(),
                name: None,
                role: None,
                location: Where::Local,
            })
            .await
            .unwrap();
        sender
            .send(UiMsg::Spawn {
                id: bbt_mux::Ulid::new(),
                swarm: None,
                placement: None,
                harness_kind: "zsh".into(),
                name: Some("generic shell".into()),
                role: None,
                location: Where::Local,
            })
            .await
            .unwrap();
        until(&updates, &publication, &mut current, |model| {
            model.ui.seats.len() == 1
        })
        .await;
        let (&seat, metadata) = current.ui.seats.first_key_value().unwrap();
        assert_eq!(metadata.name, "generic shell");
        assert_eq!(metadata.harness.0, "zsh");
        let view = bbt_proto::View {
            seat: Ulid(seat.0.0),
            generation: metadata.process_generation,
            view_generation: Ulid::nil(),
        };
        sender
            .send(UiMsg::Watched(vec![(seat, view)]))
            .await
            .unwrap();
        until(&updates, &publication, &mut current, |model| {
            model.screens.contains_key(&seat) && model.frames.contains_key(&seat)
        })
        .await;
        input(&sender, seat, view.generation, b"echo hi\r").await;
        until(&updates, &publication, &mut current, |model| {
            lines(&model.screens[&seat])
                .iter()
                .any(|line| line.trim() == "hi")
        })
        .await;
        eprintln!("PASS real IO: standalone hi row in Replica, prepared frame published");
        // Document resolve/fetch uses the same admitted session as terminal
        // controls; its result survives later grid publications until consumed.
        let original = (current.screens[&seat].cols, current.screens[&seat].rows);
        let document_op = Ulid::new();
        sender
            .send(UiMsg::DocumentRead {
                id: document_op,
                read: documents::Read::Resolve("#1".into()),
            })
            .await
            .unwrap();
        sender
            .send(UiMsg::Resize {
                seat,
                generation: view.generation,
                cols: 81,
                rows: 17,
            })
            .await
            .unwrap();
        input(&sender, seat, view.generation, b"echo DOCUMENT_IO_LIVE\r").await;
        until(&updates, &publication, &mut current, |model| {
            model
                .document_reply
                .as_ref()
                .is_some_and(|r| r.id == document_op)
                && model.screens.get(&seat).is_some_and(|screen| {
                    screen.cols == 81
                        && screen.rows == 17
                        && lines(screen)
                            .iter()
                            .any(|line| line.trim() == "DOCUMENT_IO_LIVE")
                })
        })
        .await;
        let documents::Response::Resolved(reference) =
            &current.document_reply.as_ref().unwrap().response
        else {
            panic!("document did not resolve: {:?}", current.document_reply);
        };
        let reference = reference.clone();
        sender
            .send(UiMsg::ConsumeDocument { id: document_op })
            .await
            .unwrap();
        let fetch_op = Ulid::new();
        sender
            .send(UiMsg::DocumentRead {
                id: fetch_op,
                read: documents::Read::Fetch(reference),
            })
            .await
            .unwrap();
        until(&updates, &publication, &mut current, |model| {
            model
                .document_reply
                .as_ref()
                .is_some_and(|r| r.id == fetch_op)
        })
        .await;
        assert!(
            matches!(&current.document_reply.as_ref().unwrap().response,documents::Response::Fetched(record) if record.document.number==Some(1))
        );
        sender
            .send(UiMsg::ConsumeDocument { id: fetch_op })
            .await
            .unwrap();
        sender
            .send(UiMsg::Resize {
                seat,
                generation: view.generation,
                cols: u16::try_from(original.0).unwrap(),
                rows: u16::try_from(original.1).unwrap(),
            })
            .await
            .unwrap();
        until(&updates, &publication, &mut current, |model| {
            model
                .screens
                .get(&seat)
                .is_some_and(|screen| (screen.cols, screen.rows) == original)
        })
        .await;
        eprintln!(
            "PASS document Knowledge resolve/fetch and concurrent Input/Resize through one IO owner"
        );

        // T7 exercises the production command pump, registry and real PTY.
        // A queued close does not invent a lifecycle; the metadata lane confirms it.
        for index in 0..2 {
            sender
                .send(UiMsg::Spawn {
                    id: Ulid::new(),
                    swarm: None,
                    placement: None,
                    harness_kind: "zsh".into(),
                    name: Some(format!("T7 neighbour {index}")),
                    role: None,
                    location: Where::Local,
                })
                .await
                .unwrap();
        }
        until(&updates, &publication, &mut current, |model| {
            model.ui.seats.len() == 3
                && model.ui.tab_of(seat).is_some_and(|(si, ti)| {
                    model.ui.window.sidebar.swarms[si].tabs[ti]
                        .tree
                        .leaves()
                        .len()
                        == 3
                })
        })
        .await;
        let original_tab = current.ui.tab_of(seat).unwrap();
        let tab = current.ui.window.sidebar.swarms[original_tab.0].tabs[original_tab.1].clone();
        assert_eq!(tab.tree.leaves().len(), 3);
        let process = current.states[&seat].process.clone();
        let ProcessStatus::Running { pid } = process else {
            panic!("real fixture PTY must be running")
        };
        let close = bboxy450_term::close::Intent::prepare(
            &current.ui,
            tab.id,
            &tab.tree,
            seat,
            1,
            Ulid::new(),
            Instant::now(),
        )
        .unwrap();
        sender.send(UiMsg::Close(close)).await.unwrap();
        until(&updates, &publication, &mut current, |model| {
            matches!(
                model.ui.seats[&seat].lifecycle,
                bbt_ui_model::Lifecycle::ClosedGrace { .. }
            ) && model.ui.tab_of(seat).is_none()
        })
        .await;
        assert!(matches!(
            current.states[&seat].process,
            ProcessStatus::ClosedGrace { .. }
        ));
        assert!(
            std::process::Command::new("/bin/kill")
                .args(["-0", &pid.to_string()])
                .status()
                .unwrap()
                .success(),
            "same owned PTY is alive during grace"
        );
        assert_eq!(current.ui.seats[&seat].process_generation, view.generation);
        sender.send(UiMsg::Watched(vec![])).await.unwrap();
        until(&updates, &publication, &mut current, |m| {
            !m.screens.contains_key(&seat)
        })
        .await;
        sender.send(UiMsg::UndoClose { seat }).await.unwrap();
        until(&updates, &publication, &mut current, |model| {
            model.ui.seats[&seat].lifecycle == bbt_ui_model::Lifecycle::Open
                && model.ui.tab_of(seat).is_some()
        })
        .await;
        let (si, ti) = current.ui.tab_of(seat).unwrap();
        assert_eq!(current.ui.window.sidebar.swarms[si].tabs[ti], tab);
        assert_eq!(
            current.states[&seat].process, process,
            "undo must not spawn a replacement process"
        );
        assert_eq!(current.ui.seats[&seat].process_generation, view.generation);
        sender
            .send(UiMsg::Watched(vec![(seat, view)]))
            .await
            .unwrap();
        until(&updates, &publication, &mut current, |m| {
            m.screens.contains_key(&seat) && m.frames.contains_key(&seat)
        })
        .await;
        assert!(
            lines(&current.screens[&seat])
                .iter()
                .any(|line| line.trim() == "hi")
        );
        eprintln!(
            "PASS T7 real IO: same PID/generation, tab/split and terminal screen after close/undo"
        );
        messaging_through_the_one_io_owner(
            &sender,
            &updates,
            &publication,
            &mut current,
            seat,
            task_id,
            &task_path,
        )
        .await;
        let expected: String = (0..256)
            .map(|i| "0123456789abcdef".as_bytes()[i % 16] as char)
            .collect();
        let probe = format!(
            "printf 'BURST_READY\\n'; IFS= read -r payload; if [ \"$payload\" = '{expected}' ]; then printf 'BURST_ORDER_OK\\n'; else printf 'BURST_ORDER_BAD\\n'; fi\r"
        );
        input(&sender, seat, view.generation, probe.as_bytes()).await;
        until(&updates, &publication, &mut current, |model| {
            lines(&model.screens[&seat])
                .iter()
                .any(|line| line.trim() == "BURST_READY")
        })
        .await;
        tokio::time::timeout(Duration::from_secs(30), async {
            for byte in expected.bytes() {
                input(&sender, seat, view.generation, &[byte]).await;
            }
            input(&sender, seat, view.generation, b"\r").await;
        })
        .await
        .expect("full-duplex input burst remained blocked");
        until(&updates, &publication, &mut current, |model| {
            lines(&model.screens[&seat])
                .iter()
                .any(|line| line.trim() == "BURST_ORDER_OK")
        })
        .await;
        eprintln!(
            "PASS full duplex: 256 distinct input IDs delivered all bytes in exact order while frames/replies were drained"
        );
        sender
            .send(UiMsg::Resize {
                seat,
                generation: view.generation,
                cols: 40,
                rows: 12,
            })
            .await
            .unwrap();
        until(&updates, &publication, &mut current, |model| {
            (model.screens[&seat].cols, model.screens[&seat].rows) == (40, 12)
        })
        .await;
        input(&sender, seat, view.generation, b"stty size\r").await;
        until(&updates, &publication, &mut current, |model| {
            lines(&model.screens[&seat])
                .iter()
                .any(|line| line.trim() == "12 40")
        })
        .await;
        input(
            &sender,
            seat,
            view.generation,
            b"printf 'REFLOW_abcdefghijklmnopqrstuvwxyz0123456789\\n'\r",
        )
        .await;
        until(&updates, &publication, &mut current, |model| {
            lines(&model.screens[&seat])
                .concat()
                .contains("REFLOW_abcdefghijklmnopqrstuvwxyz0123456789")
        })
        .await;
        sender
            .send(UiMsg::Resize {
                seat,
                generation: view.generation,
                cols: 20,
                rows: 12,
            })
            .await
            .unwrap();
        until(&updates, &publication, &mut current, |model| {
            model.screens[&seat].cols == 20
                && lines(&model.screens[&seat])
                    .concat()
                    .contains("REFLOW_abcdefghijklmnopqrstuvwxyz0123456789")
        })
        .await;
        assert!(
            lines(&current.screens[&seat])
                .iter()
                .all(|line| line.len() <= 20)
        );
        input(&sender, seat, view.generation, b"stty size\r").await;
        until(&updates, &publication, &mut current, |model| {
            lines(&model.screens[&seat])
                .iter()
                .any(|line| line.trim() == "12 20")
        })
        .await;
        eprintln!("PASS resize: PTY stty reports 12x40 then 12x20; prior output reflows");
        assert!(
            tokio::process::Command::new("/bin/kill")
                .args(["-KILL", &logic_pid.to_string()])
                .status()
                .await
                .unwrap()
                .success()
        );
        let reason = tokio::time::timeout(Duration::from_secs(30), lost_receiver.recv())
            .await
            .unwrap()
            .unwrap();
        if let Some(next) = updates.take() {
            current = next;
        }
        let last_screen = current.screens[&seat].clone();
        let last_frame = current.frames[&seat].clone();
        input(
            &sender,
            seat,
            view.generation,
            b"queued after backend loss\r",
        )
        .await;
        assert!(
            !task.is_finished(),
            "backend loss must not stop the app's IO owner"
        );
        if let Some(next) = updates.take() {
            assert!(matches!(
                next.events.state(),
                model::EventLaneState::Held(_)
            ));
            assert_ne!(
                next.events.state(),
                current.events.state(),
                "a duplicate hold must not dirty the frame"
            );
            assert_eq!(next.ui.window.stage.swarm, current.ui.window.stage.swarm);
            assert_eq!(next.ui.window.stage.tab, current.ui.window.stage.tab);
            assert_eq!(
                next.ui.window.stage.focused,
                current.ui.window.stage.focused
            );
            current = next;
        }
        assert!(Arc::ptr_eq(&last_screen, &current.screens[&seat]));
        assert!(Arc::ptr_eq(&last_frame, &current.frames[&seat]));
        // Fill both bounded queues while disconnected. Dropping the UI owner
        // must still wake shutdown when command receive is backpressured.
        for _ in 1..(COMMAND_CAPACITY * 2) {
            input(&sender, seat, view.generation, b"queued while lost\r").await;
        }
        assert!(reason.is_some());
        eprintln!("PASS logic SIGKILL: {reason:?}; retained Screen/frame; no restoration claimed");
        drop(sender);
        tokio::time::timeout(Duration::from_secs(30), task)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        eprintln!(
            "PASS orderly shutdown at full retained/channel capacity after backend loss, supervisor {supervisor_pid} reaped"
        );
    }
}

#[cfg(test)]
mod plugin_reply_tests {
    use super::*;
    #[test]
    fn plugin_completion_does_not_consume_an_ordinary_control_slot() {
        let mut replies = ActionReplies::default();
        let op = Ulid::new();
        replies.push(&GuiAction::ListSeats);
        replies.push(&GuiAction::Plugin {
            id: op,
            request: bbt_proto::gui::PluginRequest::List,
        });
        assert_eq!(
            replies.take(&GuiReply::Plugin {
                op,
                entries: vec![]
            }),
            Some(ActionReply::Plugin(op, true))
        );
        assert_eq!(
            replies.take(&GuiReply::Seats(vec![])),
            Some(ActionReply::Seats)
        );
    }
}
