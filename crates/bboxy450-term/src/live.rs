//! GUI socket IO, immutable model publication and worker-only preparation.
use std::{
    collections::{BTreeMap, VecDeque},
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

use crate::{UiEvent, log::Logger};
use bboxy450_term::{
    cockpit::{self, Session},
    grids::Grids,
    model::{self, Model, PanePreparer},
};
use bbt_mux::{
    Ulid,
    store::{Result, invalid},
};
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
    let (harness_kind, name, location) = match message {
        UiMsg::Spawn {
            harness_kind,
            name,
            location,
        } => (harness_kind.as_str(), name.clone(), location.clone()),
        UiMsg::SpawnZsh => ("zsh", None, Where::Local),
        _ => return None,
    };
    Some(SpawnSpec {
        swarm: SwarmId::from_u128(swarm.0),
        harness_kind: harness_kind.into(),
        name,
        role: None,
        cwd: cwd.to_string_lossy().into_owned(),
        location,
        resume: None,
    })
}

pub struct CommandSender {
    sender: mpsc::Sender<UiMsg>,
    // Dropping the UI owner wakes shutdown even when the queue and retained
    // retry slot are full. No polling timer and no UI wait are needed.
    _stop: oneshot::Sender<()>,
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
}
fn command_channel() -> (CommandSender, Commands) {
    let (sender, messages) = mpsc::channel(COMMAND_CAPACITY);
    let (stop, stopped) = oneshot::channel();
    (
        CommandSender {
            sender,
            _stop: stop,
        },
        Commands {
            messages,
            stop: stopped,
        },
    )
}

pub fn start(
    updates: Arc<Latest<Arc<Model>>>,
    proxy: EventLoopProxy<UiEvent>,
    logger: Logger,
    run_dir: Option<PathBuf>,
    rendering: oneshot::Receiver<RenderContext>,
) -> CommandSender {
    let (sender, commands) = command_channel();
    bbt_rt::handle().spawn(async move {
        let result = async {
            let config = cockpit::Config::discover(run_dir)?;
            let cwd = config.repository.clone();
            let session = Session::start(config).await?;
            run(
                updates,
                commands,
                logger.clone(),
                session,
                cwd,
                rendering,
                |reason| {
                    let event = reason.map_or(UiEvent::BackendReady, UiEvent::BackendLost);
                    let _ = proxy.send_event(event);
                },
            )
            .await
        }
        .await;
        if let Err(error) = &result {
            logger.write(format!("cockpit stopped: {error}"));
        }
        let _ = proxy.send_event(UiEvent::BackendStopped(result.map_err(|e| e.to_string())));
    });
    sender
}

fn publish(
    seats: &BTreeMap<Ulid, GuiSeat>,
    identity: &GuiIdentity,
    grids: &Grids,
    previous: &mut Arc<Model>,
    preparer: &mut PanePreparer,
    updates: &Latest<Arc<Model>>,
) -> Result<()> {
    let seats: Vec<_> = seats.values().cloned().collect();
    let records: Vec<_> = seats.iter().map(|seat| seat.record.clone()).collect();
    let mut next = model::build(&records, grids, previous);
    next.apply_seat_states(&seats);
    next.identity = Some(identity.clone());
    preparer.prepare(&mut next)?;
    *previous = Arc::new(next);
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
    outgoing.push_back(action);
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

async fn run(
    updates: Arc<Latest<Arc<Model>>>,
    mut commands: Commands,
    logger: Logger,
    mut session: Session,
    cwd: PathBuf,
    rendering: oneshot::Receiver<RenderContext>,
    backend_lost: impl Fn(Option<String>),
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
    let mut in_flight = None;
    let mut closing = false;
    // Accept one UI command's actions at a time. Watch diffs are finite;
    // the stream allows one unapplied update per watch. Deduplicate Unwatch
    // notifications so a full writer cannot create an unbounded control queue.
    let mut outgoing = VecDeque::from([GuiAction::ListHosts]);
    while !closing {
        let mut writer_blocked = false;
        if !lost {
            controls.queue_due(&grids, &mut outgoing, Instant::now());
            while let Some(action) = outgoing.front() {
                match session.send(action.clone()).await {
                    Ok(()) => {
                        controls.sent(action, Instant::now());
                        outgoing.pop_front();
                    }
                    Err(error) if error.is::<cockpit::Busy>() => {
                        writer_blocked = true;
                        break;
                    }
                    Err(error) => {
                        logger.write(format!("GUI writer stopped: {error}"));
                        backend_lost(Some(error.to_string()));
                        lost = true;
                        break;
                    }
                }
            }
        }
        let retry_at = controls.deadline(&grids, &outgoing);
        let writer_ready = session.writer_ready();
        let result:Result<()>=async {
            tokio::select! {
                command=async {
                    if !lost && let Some(message)=retained.pop_front() { return Some(message); }
                    commands.messages.recv().await
                }, if (lost && retained.len() < COMMAND_CAPACITY) || (!lost && outgoing.is_empty())=>match command {
                    Some(message) if lost=>retained.push_back(message),
                    Some(message)=> {
                    in_flight=Some(message);
                    match in_flight.as_ref().unwrap() {
                    message @ (UiMsg::SpawnZsh | UiMsg::Spawn {..})=>queue_action(&mut outgoing, GuiAction::Spawn {
                        spec:spawn_spec(message,swarm,&cwd).expect("spawn intent branch"),
                    }),
                    UiMsg::Input {seat,generation,id,bytes}=>queue_action(&mut outgoing, GuiAction::Input {seat:*seat,generation:*generation,id:*id,bytes:bytes.clone()}),
                    UiMsg::Resize {seat,generation,cols,rows}=> {
                        if seats.get(&Ulid(seat.0.0)).is_some_and(|metadata| metadata.state.generation==*generation) {
                            controls.resize(*seat,*generation,*cols,*rows,&grids,&mut outgoing);
                        }
                    }
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
                event=session.event()=>match event.unwrap_or_else(|error| cockpit::Event::Disconnected(error.to_string())) {
                    cockpit::Event::Reply(reply)=>match *reply {
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
                                logger.write(format!("identity status: {:?}",identity.state));
                                publish(&seats,&identity,&grids,&mut model,&mut preparer,&updates)?;
                            }
                        }
                        GuiReply::Seats(next)=> {
                            seats=next.into_iter().map(|seat|(seat.record.id,seat)).collect();
                            publish(&seats,&identity,&grids,&mut model,&mut preparer,&updates)?;
                            if lost { lost=false; backend_lost(None); }
                        }
                        GuiReply::Seat(seat)=> {
                            if seats.get(&seat.record.id).is_some_and(|old|old==&seat||old.state.generation>seat.state.generation) {return Ok(());}
                            let id=SeatId::from_u128(seat.record.id.0);
                            if matches!(seat.state.process,ProcessStatus::Exited {..}|ProcessStatus::Retained) {
                                grids.forget(id);
                                controls.forget(id,&mut outgoing);
                                queue_action(&mut outgoing, GuiAction::Unwatch {seat:id});
                            }
                            logger.write(format!("seat {} activity={:?} process={:?}",seat.record.id,seat.state.activity,seat.state.process));
                            seats.insert(seat.record.id,seat);
                            publish(&seats,&identity,&grids,&mut model,&mut preparer,&updates)?;
                        }
                        GuiReply::Update {seat,update}=> {
                            let (view,sequence)=match &update {
                                Update::Snapshot {snapshot}=>(snapshot.view,snapshot.sequence),
                                Update::Delta {delta}=>(delta.view,delta.sequence),
                            };
                            match grids.apply(seat,&update) {
                                Ok(changed)=> {
                                    if changed.is_some() {publish(&seats,&identity,&grids,&mut model,&mut preparer,&updates)?;}
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
                        GuiReply::Ok {op}=>logger.write(format!("GUI operation {op} accepted")),
                        GuiReply::Error {op,reason}=> {
                            controls.refused(&reason,Instant::now());
                            logger.write(format!("GUI operation {op} rejected: {reason}"));
                        }
                        GuiReply::History {..}=>logger.write("history reply deferred to C1c"),
                    },
                    cockpit::Event::Exited {process,status}=> {
                        let reason=format!("{process} exited: {status}");
                        logger.write(&reason);
                        if !lost { backend_lost(Some(reason)); }
                        lost=true;
                    }
                    cockpit::Event::Disconnected(reason)=> {
                        logger.write(format!("GUI disconnected: {reason}"));
                        if !lost { backend_lost(Some(reason)); }
                        lost=true;
                    }
                }
            }
        Ok(())
        }.await;
        if let Err(error) = result {
            logger.write(format!("live IO stopped: {error}"));
            if !lost {
                backend_lost(Some(error.to_string()));
            }
            lost = true;
            if let Some(message) = in_flight.take() {
                if let UiMsg::Watched(panes) = &message {
                    for (seat, _) in panes {
                        grids.forget(*seat);
                    }
                }
                retained.push_front(message);
            }
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
            assert_eq!(controls.0.len(), 1);
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
        assert!(controls.0.is_empty());
        assert_eq!(controls.deadline(&grids, &outgoing), None);
    }

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
        assert_eq!(
            controls.0[&seat].size, None,
            "old resize must not become new-process intent"
        );
        assert_eq!(outgoing.len(), 1);
        assert!(
            matches!(outgoing.front(), Some(GuiAction::Input {id:original,bytes,..}) if *original==id && bytes==b"typed once\r")
        );
    }

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
        assert_eq!(controls.0[&seat].size, Some((7, 37, 57)));
        controls.queue_due(&grids, &mut outgoing, now + CONTROL_RETRY);
        assert!(
            matches!(outgoing.front(), Some(GuiAction::Watch {view: token,..}) if *token==rewatch)
        );

        let next = control_view(seat, 8);
        controls.resize(seat, 8, 40, 12, &grids, &mut outgoing);
        grids.set_watched(&[(seat, next)]);
        controls.set_watched(&[(seat, next)], &mut outgoing, now + CONTROL_RETRY);
        assert_eq!(controls.0[&seat].size, Some((8, 40, 12)));
    }

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
                harness_kind: "codex".into(),
                name: Some("named seat".into()),
                location: location.clone(),
            };
            let spec = spawn_spec(&message, swarm, cwd).unwrap();
            assert_eq!(spec.swarm, SwarmId::from_u128(swarm.0));
            assert_eq!(spec.harness_kind, "codex");
            assert_eq!(spec.name.as_deref(), Some("named seat"));
            assert_eq!(spec.location, location);
            assert_eq!(spec.cwd, "/fixture/repository with spaces");
            assert_eq!(spec.role, None);
            assert_eq!(spec.resume, None);
            let wire = serde_json::to_value(&spec).unwrap();
            assert_eq!(serde_json::from_value::<SpawnSpec>(wire).unwrap(), spec);
        }
        let generic = UiMsg::Spawn {
            harness_kind: "zsh".into(),
            name: None,
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
            harness_kind: "pi".into(),
            name: None,
            location: Where::Local,
        };
        assert_eq!(
            spawn_spec(&unsupported, swarm, cwd).unwrap().harness_kind,
            "pi"
        );
    }
    use std::{os::unix::fs::PermissionsExt, time::Duration};

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
    async fn until(
        updates: &Latest<Arc<Model>>,
        current: &mut Arc<Model>,
        check: impl Fn(&Model) -> bool,
    ) {
        tokio::time::timeout(Duration::from_secs(15), async {
            loop {
                if let Some(next) = updates.take() {
                    *current = next;
                }
                if check(current) {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap_or_else(|_| {
            panic!(
                "model condition timed out; screens: {:?}",
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

    /// Exercise the same IO owner the native app starts, with real children,
    /// a real PTY, the Replica and worker preparation (no synthetic Updates).
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
        let dirty = bbt_rt::DirtySet::new(1, Arc::new(NoWake));
        let updates = Arc::new(Latest::new(dirty.source(0)));
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
            )
            .await
        });
        let mut current = Arc::new(Model::default());
        sender
            .send(UiMsg::Spawn {
                harness_kind: "pi".into(),
                name: None,
                location: Where::Local,
            })
            .await
            .unwrap();
        sender
            .send(UiMsg::Spawn {
                harness_kind: "zsh".into(),
                name: Some("generic shell".into()),
                location: Where::Local,
            })
            .await
            .unwrap();
        until(&updates, &mut current, |model| model.ui.seats.len() == 1).await;
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
        until(&updates, &mut current, |model| {
            model.screens.contains_key(&seat) && model.frames.contains_key(&seat)
        })
        .await;
        input(&sender, seat, view.generation, b"echo hi\r").await;
        until(&updates, &mut current, |model| {
            lines(&model.screens[&seat])
                .iter()
                .any(|line| line.trim() == "hi")
        })
        .await;
        eprintln!("PASS real IO: standalone hi row in Replica, prepared frame published");
        let expected: String = (0..256)
            .map(|i| "0123456789abcdef".as_bytes()[i % 16] as char)
            .collect();
        let probe = format!(
            "printf 'BURST_READY\\n'; IFS= read -r payload; if [ \"$payload\" = '{expected}' ]; then printf 'BURST_ORDER_OK\\n'; else printf 'BURST_ORDER_BAD\\n'; fi\r"
        );
        input(&sender, seat, view.generation, probe.as_bytes()).await;
        until(&updates, &mut current, |model| {
            lines(&model.screens[&seat])
                .iter()
                .any(|line| line.trim() == "BURST_READY")
        })
        .await;
        tokio::time::timeout(Duration::from_secs(15), async {
            for byte in expected.bytes() {
                input(&sender, seat, view.generation, &[byte]).await;
            }
            input(&sender, seat, view.generation, b"\r").await;
        })
        .await
        .expect("full-duplex input burst remained blocked");
        until(&updates, &mut current, |model| {
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
        until(&updates, &mut current, |model| {
            (model.screens[&seat].cols, model.screens[&seat].rows) == (40, 12)
        })
        .await;
        input(&sender, seat, view.generation, b"stty size\r").await;
        until(&updates, &mut current, |model| {
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
        until(&updates, &mut current, |model| {
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
        until(&updates, &mut current, |model| {
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
        until(&updates, &mut current, |model| {
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
        let reason = tokio::time::timeout(Duration::from_secs(10), lost_receiver.recv())
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
        tokio::time::sleep(Duration::from_millis(150)).await;
        assert!(
            !task.is_finished(),
            "backend loss must not stop the app's IO owner"
        );
        assert!(updates.take().is_none());
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
        tokio::time::timeout(Duration::from_secs(10), task)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        eprintln!(
            "PASS orderly shutdown at full retained/channel capacity after backend loss, supervisor {supervisor_pid} reaped"
        );
    }
}
