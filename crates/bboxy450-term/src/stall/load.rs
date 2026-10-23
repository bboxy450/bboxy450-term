use super::*;
use bboxy450_term::{
    grids::Grids,
    model::{self, PanePreparer},
};
use bbt_mux::{
    Action, Attachment, Event, LaunchSpec, Reply, Supervisor, Ulid,
    bus::{Consumer, Delivery},
    logic::{Logic, LogicConfig, SupervisorConnection},
    store::events::Draft,
};
use bbt_proto::{GuiAction, GuiReply, GuiRequest, SeatRecord, Snapshot, Update, View};
use std::{collections::VecDeque, ffi::OsString, path::Path, sync::Mutex};
use tokio::{
    io::{AsyncWriteExt, BufReader},
    net::UnixStream,
    sync::watch,
};

struct Core {
    logic: Logic,
    supervisor: Supervisor,
    ring_probe: RingProbe,
}
impl Core {
    #[allow(
        clippy::disallowed_methods,
        reason = "R5 bootstrap file IO runs on the blocking pool, never the UI"
    )]
    async fn open(config: &Config) -> Result<Self, String> {
        let root = config.run_dir.clone();
        bbt_rt::handle()
            .spawn_blocking(move || {
                bbt_rt::assert_not_ui_thread();
                std::fs::create_dir(&root).map_err(|e| format!("r5-fresh-run-dir: {e}"))?;
                let repository = root.join("repository");
                std::fs::create_dir_all(repository.join(".bboxy")).map_err(|e| e.to_string())?;
                let supervisor = Supervisor::start(bbt_mux::Config {
                    journal_directory: Some(root.join("journal")),
                    ..Default::default()
                })
                .map_err(|e| e.to_string())?;
                let connection = supervisor.connect().map_err(|e| e.to_string())?;
                let ring_probe = RingProbe::new(connection);
                let logic = Logic::start_custodied(
                    ObservedConnection(ring_probe.clone()),
                    LogicConfig {
                        run_dir: root.join("logic"),
                        repository,
                        ring: Default::default(),
                    },
                    bbt_rt::handle(),
                )
                .map_err(|e| e.to_string())?;
                Ok(Self {
                    logic,
                    supervisor,
                    ring_probe,
                })
            })
            .await
            .map_err(|e| e.to_string())?
    }
    async fn close(self) -> Result<(), String> {
        bbt_rt::handle()
            .spawn_blocking(move || {
                let a = self.logic.shutdown().map_err(|e| e.to_string());
                let b = self.supervisor.shutdown().map_err(|e| e.to_string());
                a.and(b)
            })
            .await
            .map_err(|e| e.to_string())?
    }
}

// This wrapper shares the already admitted connection. A second connect()
// would acquire a new logic lease and invalidate the run being diagnosed.
#[derive(Clone)]
struct RingProbe {
    client: Arc<Mutex<Box<dyn SupervisorConnection>>>,
    attachments: Arc<Mutex<BTreeMap<Ulid, Attachment>>>,
}
struct ObservedConnection(RingProbe);

#[allow(
    clippy::disallowed_methods,
    reason = "R5 diagnostic state and blocking client are owned by IO tasks, never the UI"
)]
fn io_lock<T>(value: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    bbt_rt::assert_not_ui_thread();
    value
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[allow(
    clippy::disallowed_methods,
    reason = "bounded diagnostic lock wait on the blocking pool after the run has already failed"
)]
fn diagnostic_lock<T>(
    value: &Mutex<T>,
    deadline: Instant,
) -> Result<std::sync::MutexGuard<'_, T>, String> {
    bbt_rt::assert_not_ui_thread();
    loop {
        match value.try_lock() {
            Ok(lock) => return Ok(lock),
            Err(std::sync::TryLockError::Poisoned(error)) => return Ok(error.into_inner()),
            Err(std::sync::TryLockError::WouldBlock) => {
                if Instant::now() >= deadline {
                    return Err("ring diagnostic connection lock deadline reached".into());
                }
                std::thread::sleep(Duration::from_millis(1));
            }
        }
    }
}

impl SupervisorConnection for ObservedConnection {
    fn request(&mut self, action: Action) -> bbt_mux::store::Result<Reply> {
        let reply = io_lock(&self.0.client).request(action)?;
        if let Reply::Attached { attachment, .. } = &reply {
            io_lock(&self.0.attachments).insert(attachment.seat, attachment.clone());
        }
        Ok(reply)
    }

    fn receive(&mut self, timeout: Duration) -> bbt_mux::store::Result<Option<Event>> {
        io_lock(&self.0.client).receive(timeout)
    }
}

#[derive(Debug, Default, serde::Serialize)]
struct RingWitness {
    captured_sequence: Option<u64>,
    committed_sequence: Option<u64>,
    first_sequence: Option<u64>,
    examined_from: Option<u64>,
    examined_to: Option<u64>,
    paused_in_examined_tail: Option<bool>,
    paused_marker_sequence: Option<u64>,
    tail_complete: bool,
    gap: Option<bbt_mux::ring::HistoryGap>,
    sampled_after_timeout_ms: f64,
    error: Option<String>,
}

#[derive(Default)]
struct PauseBytes {
    suffix: Vec<u8>,
    found_at: Option<u64>,
}
impl PauseBytes {
    fn output(&mut self, sequence: u64, bytes: &[u8]) {
        self.suffix.extend_from_slice(bytes);
        if self
            .suffix
            .windows(b"R5PAUSED".len())
            .any(|w| w == b"R5PAUSED")
        {
            self.found_at = Some(sequence);
        }
        let keep = self.suffix.len().saturating_sub(b"R5PAUSED".len() - 1);
        self.suffix.drain(..keep);
    }
}

impl RingProbe {
    fn new(client: impl SupervisorConnection + 'static) -> Self {
        Self {
            client: Arc::new(Mutex::new(Box::new(client))),
            attachments: Arc::new(Mutex::new(BTreeMap::new())),
        }
    }

    fn tail(&self, seat: Ulid, deadline: Instant, row: &mut RingWitness) -> Result<(), String> {
        let attachment = io_lock(&self.attachments)
            .get(&seat)
            .cloned()
            .ok_or("no attachment observed on the admitted connection")?;
        let reply = diagnostic_lock(&self.client, deadline)?
            .request(Action::RingRecovery {
                attachment: attachment.clone(),
            })
            .map_err(|e| e.to_string())?;
        let Reply::RingRecovery { status, .. } = reply else {
            return Err("unexpected ring status reply".into());
        };
        row.captured_sequence = Some(status.captured_sequence);
        row.committed_sequence = Some(status.committed_sequence);
        row.first_sequence = Some(status.first_sequence);
        if status.captured_sequence <= status.first_sequence {
            row.tail_complete = true;
            row.paused_in_examined_tail = Some(false);
            return Ok(());
        }
        // A bounded final-output probe, not a claim about all retained history.
        // Keep sequence ranges and gaps so absence is never inferred from a
        // failed or truncated read. Final PAUSED may span two Output records.
        let from = status
            .captured_sequence
            .saturating_sub(1)
            .max(status.first_sequence + 1);
        let to = status.captured_sequence;
        row.examined_from = Some(from);
        row.examined_to = Some(to);
        let mut next = from;
        let mut marker = PauseBytes::default();
        for _ in 0..4 {
            if Instant::now() >= deadline {
                return Err("ring diagnostic deadline reached".into());
            }
            let reply = diagnostic_lock(&self.client, deadline)?
                .request(Action::ReadRing {
                    attachment: attachment.clone(),
                    from: next,
                    to,
                })
                .map_err(|e| e.to_string())?;
            let Reply::RingHistory { history } = reply else {
                return Err("unexpected ring history reply".into());
            };
            let bbt_mux::ring::History {
                records,
                gap,
                next: after,
                ..
            } = *history;
            if gap.is_some() {
                row.gap = gap;
                marker.suffix.clear();
            }
            for record in records {
                if let Event::Output { seq, bytes, .. } = record.event {
                    marker.output(seq, &bytes);
                }
            }
            row.paused_marker_sequence = marker.found_at;
            if marker.found_at.is_some() {
                row.paused_in_examined_tail = Some(true);
            }
            match after {
                None => {
                    row.tail_complete = true;
                    if row.gap.is_none() && row.paused_in_examined_tail.is_none() {
                        row.paused_in_examined_tail = Some(false);
                    }
                    return Ok(());
                }
                Some(after) if after > next && after <= to => next = after,
                _ => {
                    return Err(
                        "ring history cursor did not advance within the requested range".into(),
                    );
                }
            }
        }
        Err("ring diagnostic page limit reached".into())
    }

    fn collect(&self, seats: Vec<Ulid>, started: Instant) -> BTreeMap<Ulid, RingWitness> {
        let deadline = Instant::now() + Duration::from_secs(20);
        seats
            .into_iter()
            .map(|seat| {
                let mut row = RingWitness::default();
                // Existing Client RPCs have a 5s bound; one already-started RPC can
                // outlast this collection budget. No request is started after it.
                let result = if Instant::now() < deadline {
                    self.tail(seat, deadline, &mut row)
                } else {
                    Err("ring diagnostic collection budget exhausted".into())
                };
                row.error = result.err();
                row.sampled_after_timeout_ms = started.elapsed().as_secs_f64() * 1000.;
                (seat, row)
            })
            .collect()
    }
}

pub async fn run(
    config: Config,
    updates: Arc<Latest<Arc<Model>>>,
    rendering: oneshot::Receiver<crate::live::RenderContext>,
    messages: UiSender,
    progress: Arc<Progress>,
    commands: mpsc::Receiver<Control>,
    mut stopped: oneshot::Receiver<()>,
) -> Result<Report, String> {
    let host_load_before = bbt_rt::fault::host_load_average().map_err(|e| e.to_string())?;
    eprintln!("R5 host load before: {host_load_before:?}");
    if config.frame_gate && host_load_before[0] > 8. {
        let reason = format!(
            "r5-timing-run-discarded: starting loadavg {} exceeds8",
            host_load_before[0]
        );
        tokio::fs::write(
            &config.report,
            serde_json::to_vec_pretty(&serde_json::json!({
                "discarded": true, "reason": reason, "host_load_before": host_load_before,
            }))
            .map_err(|e| e.to_string())?,
        )
        .await
        .map_err(|e| e.to_string())?;
        return Err(reason);
    }
    let mut result=async {
        disk_guard(&config.run_dir, config.real).await?;
        if config.real {
            let output=tokio::process::Command::new("sysctl").args(["-n","kern.tty.ptmx_max"]).output().await.map_err(|e|e.to_string())?;
            let maximum=String::from_utf8_lossy(&output.stdout).trim().parse::<usize>().map_err(|_|"r5-pty-ceiling-unavailable")?.saturating_sub(32);
            if config.seats>maximum {return Err(format!("r5-pty-capacity: requested{} exceeds host ceiling{maximum} with32margin",config.seats));}
        }
        let core=Core::open(&config).await?;
        let identity=core.logic.client.identity_status().await.map_err(|e|e.to_string())?;
        if identity.custody!=Some(bbt_mux::identity::Custody::Memory) {core.close().await?;return Err("r5-custody-witness-not-memory".into());}
        eprintln!("R5 identity witness: custody={:?} label={:?}",identity.custody,identity.custody_label);
        let outcome=tokio::select! {
            result=exercise(&config,&core,updates,rendering,messages,progress.clone(),commands)=>result,
            _=&mut stopped=>Err("r5-cancelled".into()),
        };
        let mut background_reaped=true;
        if let Some(task)=progress.background.get() {
            bbt_rt::cancel(task);
            let end=Instant::now()+Duration::from_secs(5);
            while task.status().is_none() && Instant::now()<end {tokio::time::sleep(Duration::from_millis(10)).await;}
            background_reaped=task.status().is_some();
        }
        let cleanup=core.close().await;
        let _=tokio::fs::remove_dir_all(&config.run_dir).await;
        cleanup?;
        if !background_reaped {return Err("r5-background-command-not-reaped".into());}
        outcome
    }.await;
    if let Ok(report) = &mut result {
        report.host_load_before = host_load_before;
        report.host_load_after = bbt_rt::fault::host_load_average().map_err(|e| e.to_string())?;
        let json = serde_json::to_vec_pretty(report).map_err(|e| e.to_string())?;
        tokio::fs::write(&config.report, json)
            .await
            .map_err(|e| e.to_string())?;
        println!("{}", report.gate_line(&config));
    }
    result
}

async fn disk_guard(path: &Path, real: bool) -> Result<(), String> {
    let parent = if path.exists() {
        path
    } else {
        path.parent().ok_or("r5-run-parent")?
    };
    let output = tokio::process::Command::new("df")
        .arg("-Pk")
        .arg(parent)
        .env("LC_ALL", "C")
        .output()
        .await
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err("r5-disk-probe-failed".into());
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let free = text
        .lines()
        .last()
        .and_then(|l| l.split_whitespace().nth(3))
        .and_then(|s| s.parse::<u64>().ok())
        .ok_or("r5-disk-probe-invalid")?
        * 1024;
    // Real output is journaled; the synthetic fixture writes no PTY history.
    let minimum = if real {
        10 * 1024 * 1024 * 1024
    } else {
        512 * 1024 * 1024
    };
    if free < minimum {
        return Err(format!(
            "r5-disk-guard: {free}bytes below required{minimum}"
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
struct UpdateCursor {
    view: View,
    sequence: u64,
}
impl UpdateCursor {
    fn of(update: &Update) -> Self {
        match update {
            Update::Snapshot { snapshot } => Self {
                view: snapshot.view,
                sequence: snapshot.sequence,
            },
            Update::Delta { delta } => Self {
                view: delta.view,
                sequence: delta.sequence,
            },
        }
    }
}

#[derive(Debug, Clone, Default, serde::Serialize)]
struct GuiSeatTrace {
    received: Option<UpdateCursor>,
    applied: Option<UpdateCursor>,
    applied_has_paused: Option<bool>,
    applied_queued: Option<UpdateCursor>,
    applied_written: Option<UpdateCursor>,
}

#[derive(Default)]
struct GuiTrace(Mutex<BTreeMap<SeatId, GuiSeatTrace>>);
impl GuiTrace {
    fn new(seats: impl IntoIterator<Item = SeatId>) -> Self {
        Self(Mutex::new(
            seats
                .into_iter()
                .map(|seat| (seat, GuiSeatTrace::default()))
                .collect(),
        ))
    }

    fn with_rows<R>(&self, f: impl FnOnce(&mut BTreeMap<SeatId, GuiSeatTrace>) -> R) -> R {
        f(&mut io_lock(&self.0))
    }

    fn record(&self, seat: SeatId, f: impl FnOnce(&mut GuiSeatTrace)) {
        self.with_rows(|rows| {
            if let Some(row) = rows.get_mut(&seat) {
                f(row);
            }
        });
    }

    fn snapshot(&self) -> BTreeMap<SeatId, GuiSeatTrace> {
        self.with_rows(|rows| rows.clone())
    }
}

struct Wire {
    out: mpsc::Sender<GuiAction>,
    incoming: mpsc::Receiver<Result<GuiReply, String>>,
    reader: tokio::task::JoinHandle<()>,
    writer: tokio::task::JoinHandle<()>,
    trace: Arc<GuiTrace>,
}
impl Drop for Wire {
    fn drop(&mut self) {
        self.reader.abort();
        self.writer.abort();
    }
}
impl Wire {
    async fn open(core: &Core, trace: Arc<GuiTrace>) -> Result<Self, String> {
        let endpoint = core
            .logic
            .client
            .gui_endpoint()
            .await
            .map_err(|e| e.to_string())?;
        let stream = UnixStream::connect(endpoint.path)
            .await
            .map_err(|e| e.to_string())?;
        Ok(Self::from_stream(stream, endpoint.capability, trace))
    }

    fn from_stream(stream: UnixStream, capability: [u8; 32], trace: Arc<GuiTrace>) -> Self {
        let (read, mut write) = stream.into_split();
        let (out, mut requests) = mpsc::channel(64);
        let (reply, incoming) = mpsc::channel(8);
        let received_trace = trace.clone();
        let reader = tokio::spawn(async move {
            let mut read = BufReader::new(read);
            loop {
                let parsed = match bbt_mux::seat_socket::read_line(&mut read).await {
                    Ok(Some(line)) => {
                        serde_json::from_slice::<GuiReply>(&line).map_err(|e| e.to_string())
                    }
                    Ok(None) => Err("r5-gui-eof".into()),
                    Err(e) => Err(e.to_string()),
                };
                if let Ok(GuiReply::Update { seat, update }) = &parsed {
                    received_trace
                        .record(*seat, |row| row.received = Some(UpdateCursor::of(update)));
                }
                let end = parsed.is_err();
                if reply.send(parsed).await.is_err() || end {
                    break;
                }
            }
        });
        let written_trace = trace.clone();
        let writer = tokio::spawn(async move {
            while let Some(action) = requests.recv().await {
                let applied = match &action {
                    GuiAction::Applied {
                        seat,
                        view,
                        sequence,
                    } => Some((
                        *seat,
                        UpdateCursor {
                            view: *view,
                            sequence: *sequence,
                        },
                    )),
                    _ => None,
                };
                let request = GuiRequest {
                    version: bbt_proto::gui::VERSION,
                    capability,
                    action,
                };
                let Ok(mut bytes) = serde_json::to_vec(&request) else {
                    break;
                };
                bytes.push(b'\n');
                if write.write_all(&bytes).await.is_err() {
                    break;
                }
                if let Some((seat, cursor)) = applied {
                    written_trace.record(seat, |row| row.applied_written = Some(cursor));
                }
            }
        });
        Self {
            out,
            incoming,
            reader,
            writer,
            trace,
        }
    }
}

/// Own the GUI drain separately from the load coordinator. In particular,
/// waiting for side_loads must never stop applying and acknowledging Updates.
struct GuiTask {
    actions: mpsc::Sender<GuiAction>,
    task: tokio::task::JoinHandle<Result<(), String>>,
}
impl Drop for GuiTask {
    fn drop(&mut self) {
        self.task.abort();
    }
}
impl GuiTask {
    fn start(
        wire: Wire,
        outgoing: VecDeque<GuiAction>,
        apply: impl FnMut(SeatId, &Update) -> Result<bool, String> + Send + 'static,
    ) -> Self {
        let (actions, incoming) = mpsc::channel(64);
        let task = tokio::spawn(drain_gui(wire, outgoing, incoming, apply));
        Self { actions, task }
    }
}

#[track_caller]
fn gui_error(op: Ulid, reason: &str, input: Option<Ulid>) -> String {
    let path = if reason == "GUI socket version mismatch" {
        "server admission"
    } else if op.is_nil() {
        "server periodic view check"
    } else if Some(op) == input {
        "server Input action"
    } else {
        // Watch/Applied operation IDs are generated by the server. Do not
        // guess their action from FIFO order or unrelated Seat notifications.
        "server control action (uncorrelated operation)"
    };
    format!(
        "r5-gui-error at {}: op={op} path={path}: {reason}",
        std::panic::Location::caller()
    )
}

async fn drain_gui(
    mut wire: Wire,
    mut outgoing: VecDeque<GuiAction>,
    mut actions: mpsc::Receiver<GuiAction>,
    mut apply: impl FnMut(SeatId, &Update) -> Result<bool, String>,
) -> Result<(), String> {
    let mut input = None;
    loop {
        let mut writer_blocked = false;
        while let Some(action) = outgoing.pop_front() {
            let id = match &action {
                GuiAction::Input { id, .. } => Some(*id),
                _ => None,
            };
            match wire.out.try_send(action) {
                Ok(()) => input = id.or(input),
                Err(mpsc::error::TrySendError::Full(action)) => {
                    outgoing.push_front(action);
                    writer_blocked = true;
                    break;
                }
                Err(_) => return Err("r5-gui-writer-closed".into()),
            }
        }
        let capacity = wire.out.clone();
        tokio::select! {
            action=actions.recv()=>match action {
                Some(action)=>outgoing.push_back(action),
                None=>return Err("r5-gui-actions-closed".into()),
            },
            reply=wire.incoming.recv()=>match reply.ok_or("r5-gui-reader-closed")?? {
                GuiReply::Update {seat,update}=> {
                    let cursor=UpdateCursor::of(&update);
                    let paused=apply(seat,&update)?;
                    outgoing.push_back(GuiAction::Applied {seat,view:cursor.view,sequence:cursor.sequence});
                    wire.trace.record(seat, |row| {
                        row.applied=Some(cursor);
                        row.applied_has_paused=Some(paused);
                        row.applied_queued=Some(cursor);
                    });
                }
                GuiReply::Error {op,reason}=> {
                    let error=gui_error(op,&reason,input);
                    eprintln!("{error}");
                    return Err(error);
                }
                _=>{},
            },
            _=async {drop(capacity.reserve_owned().await);},if writer_blocked=>{},
        }
    }
}

struct PaneWork {
    grids: Grids,
    previous: Arc<Model>,
    preparer: PanePreparer,
}

#[derive(Clone, Copy, Default, serde::Serialize)]
struct PaneState {
    ready: bool,
    paused: bool,
}
impl PaneState {
    fn from_model(model: &Model, watched: usize) -> Self {
        Self {
            ready: model.frames.len() == watched,
            paused: model.screens.len() == watched
                && model
                    .screens
                    .values()
                    .all(|screen| contains(screen, "R5PAUSED")),
        }
    }
}

#[derive(Default, serde::Serialize)]
struct LogicWitness {
    cursor: Option<UpdateCursor>,
    has_paused: Option<bool>,
    sampled_after_timeout_ms: f64,
    error: Option<String>,
}

#[derive(serde::Serialize)]
struct SettleSeat {
    watched_index: usize,
    name: String,
    seat: Ulid,
    generation: u64,
    producer_pid: Option<u32>,
    gui_at_timeout: GuiSeatTrace,
    applied_written_to_socket: Option<bool>,
    logic_current_snapshot: LogicWitness,
    supervisor_ring_tail: RingWitness,
}

fn false_settle_predicates(
    phase: u8,
    panes: PaneState,
    native: NativeSettleWitness,
) -> Vec<&'static str> {
    let mut predicates = Vec::new();
    if !panes.paused {
        predicates.push("IO: every watched published Screen contains R5PAUSED");
    }
    if phase < SETTLE {
        predicates.push("IO: phase reached SETTLE");
    }
    if !native.observed {
        predicates.push("UI: Native::poll has been observed");
    } else if phase < IDLE && native.phase != SETTLE {
        predicates.push(
            "UI: latest Native::poll observed phase SETTLE (other UI settle checks inactive)",
        );
    } else if native.phase == SETTLE {
        if native.pending {
            predicates.push("UI: no pending work");
        }
        if !native.has_paused_frame {
            predicates.push("UI: a frame containing all paused panes was encoded");
        }
        if !native.gpu_complete {
            predicates.push("UI: paused frame reached GPU completion");
        }
        if !native.stable_counts {
            predicates.push("UI: frame and callback counts match the previous settle poll");
        }
    }
    predicates
}

// Diagnostic queries happen only AFTER a timeout. They cannot turn a failed
// run into a pass. GUI and native predicates are frozen before any queries;
// backend snapshots are explicitly later observations, not a consistent cut.
async fn settle_timeout(
    reason: &str,
    config: &Config,
    core: &Core,
    (records, pids): (&[SeatRecord], &[u32]),
    progress: &Progress,
    panes: PaneState,
    gui: &GuiTrace,
) -> String {
    let started = Instant::now();
    let phase = progress.phase();
    let native = progress.native_settle.snapshot();
    let mut received = gui.snapshot();
    let mut rows: Vec<_> = records
        .iter()
        .take(config.watched)
        .enumerate()
        .map(|(index, record)| {
            let trace = received
                .remove(&SeatId::from_u128(record.id.0))
                .unwrap_or_default();
            let written = trace
                .applied
                .map(|applied| trace.applied_written == Some(applied));
            SettleSeat {
                watched_index: index,
                name: record.name.clone(),
                seat: record.id,
                generation: record.generation,
                producer_pid: pids.get(index).copied(),
                gui_at_timeout: trace,
                applied_written_to_socket: written,
                logic_current_snapshot: LogicWitness::default(),
                supervisor_ring_tail: RingWitness::default(),
            }
        })
        .collect();
    // Query logic first, without holding its shared supervisor connection.
    // These are current parser Snapshots, NOT the GUI server's offered Update.
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    for row in &mut rows {
        let result = if config.real && tokio::time::Instant::now() < deadline {
            tokio::time::timeout_at(
                deadline.min(tokio::time::Instant::now() + Duration::from_secs(1)),
                core.logic.client.snapshot(row.seat, row.generation),
            )
            .await
            .map_err(|_| "logic current snapshot deadline reached".to_owned())
            .and_then(|reply| reply.map_err(|e| e.to_string()))
        } else {
            Err(
                "logic current snapshot unavailable (synthetic or collection budget exhausted)"
                    .into(),
            )
        };
        match result {
            Ok(snapshot) => {
                row.logic_current_snapshot.cursor = Some(UpdateCursor {
                    view: snapshot.view,
                    sequence: snapshot.sequence,
                });
                row.logic_current_snapshot.has_paused =
                    Some(contains(&snapshot.screen, "R5PAUSED"));
            }
            Err(error) => row.logic_current_snapshot.error = Some(error),
        }
        row.logic_current_snapshot.sampled_after_timeout_ms =
            started.elapsed().as_secs_f64() * 1000.;
    }
    if config.real {
        let probe = core.ring_probe.clone();
        let seats = rows.iter().map(|row| row.seat).collect();
        match bbt_rt::handle()
            .spawn_blocking(move || probe.collect(seats, started))
            .await
        {
            Ok(mut rings) => {
                for row in &mut rows {
                    row.supervisor_ring_tail = rings.remove(&row.seat).unwrap_or_default();
                }
            }
            Err(error) => {
                for row in &mut rows {
                    row.supervisor_ring_tail.error = Some(format!("ring diagnostic task: {error}"));
                }
            }
        }
    }
    let table = serde_json::json!({
        "reason": reason,
        "phase_at_timeout": phase,
        "panes_at_timeout": panes,
        "native_last_poll_at_timeout": native,
        "false_settle_predicates": false_settle_predicates(phase, panes, native),
        "notes": [
            "GUI checkpoints frozen at timeout; native flags are from its last poll and counters may be newer",
            "Applied written means socket write_all completed, not server acceptance",
            "logic current snapshot is a later parser observation, NOT GUI offered/published",
            "ring query examines the final two captured sequences, up to four pages, not all historical output",
            "null means unknown; a gap, failed read or truncated tail never proves marker absence",
            "R5PAUSED is the existing marker; an initial pause is not distinguished from the final pause",
        ],
        "seats": rows,
    });
    let json = serde_json::to_string_pretty(&table).expect("diagnostic fields serialize");
    let path = config.report.with_extension("diagnostics.json");
    let saved = tokio::fs::write(&path, &json).await;
    let location = match saved {
        Ok(()) => format!("table={}", path.display()),
        Err(error) => format!("table-write-failed={} error={error}", path.display()),
    };
    // Keep the complete per-seat table in stderr even if report IO fails.
    format!("{reason}; {location}\nR5 settle diagnostic table:\n{json}")
}

fn publish(
    records: &[SeatRecord],
    grids: &Grids,
    previous: &mut Arc<Model>,
    preparer: &mut PanePreparer,
    updates: &Latest<Arc<Model>>,
) -> Result<(), String> {
    let mut next = model::build(records, grids, previous);
    // A fixture grid keeps the probe's entire row visible. The production
    // membership builder's one-row fallback would make 50 sliver panes.
    for swarm in &mut next.ui.window.sidebar.swarms {
        if swarm.id == bbt_ui_model::model::SwarmId::from_u128(1) {
            for tab in &mut swarm.tabs {
                tab.tree = fixture_grid(&tab.seats);
            }
        }
    }
    preparer.prepare(&mut next).map_err(|e| e.to_string())?;
    *previous = Arc::new(next);
    updates.publish(previous.clone());
    Ok(())
}

fn fixture_grid(seats: &[SeatId]) -> bbt_ui_model::model::SplitNode {
    use bbt_ui_model::model::{Axis, SplitNode};
    fn join(nodes: &[SplitNode], axis: Axis) -> SplitNode {
        if nodes.len() == 1 {
            return nodes[0].clone();
        }
        let half = nodes.len() / 2;
        SplitNode::split(
            axis,
            half as f32 / nodes.len() as f32,
            join(&nodes[..half], axis),
            join(&nodes[half..], axis),
        )
    }
    let columns = ((seats.len() as f64 * 1.5).sqrt().ceil() as usize).max(1);
    let rows: Vec<_> = seats
        .chunks(columns)
        .map(|row| {
            join(
                &row.iter().copied().map(SplitNode::Leaf).collect::<Vec<_>>(),
                Axis::Row,
            )
        })
        .collect();
    join(&rows, Axis::Column)
}

async fn exercise(
    config: &Config,
    core: &Core,
    updates: Arc<Latest<Arc<Model>>>,
    rendering: oneshot::Receiver<crate::live::RenderContext>,
    messages: UiSender,
    progress: Arc<Progress>,
    mut commands: mpsc::Receiver<Control>,
) -> Result<Report, String> {
    let rendering = rendering.await.map_err(|_| "r5-renderer-stopped")?;
    let mut options = bbt_render::cells::terminal::Options::default();
    options.font.scale = rendering.scale.round().max(1.) as u32;
    let preparer =
        PanePreparer::with_options(rendering.context, options).map_err(|e| e.to_string())?;
    let mut records = Vec::with_capacity(config.seats);
    let mut pids = Vec::new();
    let executable = std::env::current_exe().map_err(|e| e.to_string())?;
    for i in 0..config.seats {
        if config.real && i % 25 == 0 {
            disk_guard(&config.run_dir, true).await?;
        }
        let mut spec = LaunchSpec::new(executable.to_string_lossy(), &config.run_dir);
        spec.args = vec![
            "--r5-producer".into(),
            config.run_dir.to_string_lossy().into_owned(),
        ];
        spec.size.cols = 24;
        spec.size.rows = 8;
        let id = if config.real {
            spec.seat
        } else {
            Ulid::from(i as u128 + 1)
        };
        let record = SeatRecord::new(
            id,
            Ulid::from(if i < config.watched { 1 } else { 2 }),
            format!(
                "{}-{i}",
                if config.real {
                    "r5-pty"
                } else {
                    "synthetic-state"
                }
            ),
            "zsh",
        );
        if config.real {
            let info = core
                .logic
                .client
                .spawn(record.clone(), spec)
                .await
                .map_err(|e| format!("r5-spawn-{i}: {e}"))?;
            pids.push(info.pid);
            eprintln!("R5 owned producer seat={} pid={}", record.id, info.pid);
            tokio::fs::write(
                config.run_dir.join("owned-pids.json"),
                serde_json::to_vec(&pids).map_err(|e| e.to_string())?,
            )
            .await
            .map_err(|e| e.to_string())?;
        }
        records.push(record);
    }
    progress.rows.store(records.len() as u64, Ordering::Release);
    if !config.real {
        struct FixtureWake(UiSender);
        impl bbt_rt::Waker for FixtureWake {
            fn wake(&self) {
                let _ = self.0.try_send(bbt_rt::UiMsg::Changed { source: 7 });
            }
        }
        let dirty = bbt_rt::DirtySet::new(config.seats, Arc::new(FixtureWake(messages.clone())));
        let values = (0..config.seats)
            .map(|id| Latest::new(dirty.source(id)))
            .collect();
        let _ = progress.synthetic.set(SyntheticStates {
            dirty,
            values,
            consumed: AtomicU64::new(0),
        });
    }
    let panes: Vec<_> = records
        .iter()
        .take(config.watched)
        .map(|r| {
            (
                SeatId::from_u128(r.id.0),
                View {
                    seat: r.id,
                    generation: 1,
                    view_generation: Ulid::new(),
                },
            )
        })
        .collect();
    let mut grids = Grids::new();
    let watches = grids.set_watched(&panes);
    let mut terms: Vec<_> = (0..config.watched)
        .map(|_| bbt_core::Term::new(24, 8, 0))
        .collect();
    let mut previous = Arc::new(Model::default());
    Arc::make_mut(&mut previous).ui.window.theme = rendering.theme;
    let mut work = Some(PaneWork {
        grids,
        previous,
        preparer,
    });
    let (state, latest_state) = watch::channel(PaneState::default());
    let gui_trace = Arc::new(GuiTrace::new(panes.iter().map(|(seat, _)| *seat)));
    let mut gui = if config.real {
        let wire = Wire::open(core, gui_trace.clone()).await?;
        let mut work = work.take().unwrap();
        let records = records.clone();
        let updates = updates.clone();
        let progress = progress.clone();
        let watched = config.watched;
        Some(GuiTask::start(wire, watches.into(), move |seat, update| {
            if work
                .grids
                .apply(seat, update)
                .map_err(|e| e.to_string())?
                .is_some()
            {
                publish(
                    &records,
                    &work.grids,
                    &mut work.previous,
                    &mut work.preparer,
                    &updates,
                )?;
            }
            progress.updates.fetch_add(1, Ordering::Relaxed);
            state.send_replace(PaneState::from_model(&work.previous, watched));
            Ok(work
                .grids
                .screen(seat)
                .is_some_and(|screen| contains(&screen, "R5PAUSED")))
        }))
    } else {
        None
    };
    let mut sequence = 0;
    let mut warm = None;
    let mut measured = None;
    let mut observed_seconds = 0.;
    let mut side = None;
    let mut pending_probe = None;
    let mut stop_sent = false;
    let mut stopped_at = None;
    let mut last_disk = Instant::now();
    let start = Instant::now();
    let mut tick = tokio::time::interval(Duration::from_millis(5));
    let mut sides = SideStats::default();
    loop {
        if start.elapsed() > Duration::from_secs(240) {
            let panes = work.as_ref().map_or_else(
                || *latest_state.borrow(),
                |work| PaneState::from_model(&work.previous, config.watched),
            );
            return Err(settle_timeout(
                "r5-setup-or-settle-timeout",
                config,
                core,
                (&records, &pids),
                &progress,
                panes,
                &gui_trace,
            )
            .await);
        }
        if stopped_at.is_some_and(|at: Instant| at.elapsed() > Duration::from_secs(20))
            && progress.phase() < IDLE
        {
            let panes = work.as_ref().map_or_else(
                || *latest_state.borrow(),
                |work| PaneState::from_model(&work.previous, config.watched),
            );
            return Err(settle_timeout(
                "r5-unsettled-after-producers-stopped",
                config,
                core,
                (&records, &pids),
                &progress,
                panes,
                &gui_trace,
            )
            .await);
        }
        if config.real && last_disk.elapsed() >= Duration::from_secs(1) {
            disk_guard(&config.run_dir, true).await?;
            last_disk = Instant::now();
        }
        let pane_state = work.as_ref().map_or_else(
            || *latest_state.borrow(),
            |work| PaneState::from_model(&work.previous, config.watched),
        );
        if pane_state.ready && warm.is_none() {
            tokio::fs::write(config.run_dir.join("go"), b"run")
                .await
                .map_err(|e| e.to_string())?;
            progress.set_phase(WARM);
            warm = Some(Instant::now());
        }
        if warm.is_some_and(|t| t.elapsed() >= Duration::from_secs(2)) && measured.is_none() {
            progress.set_phase(MEASURE);
            measured = Some(Instant::now());
            let client = core.logic.client.clone();
            let p = progress.clone();
            let sender = messages.clone();
            let exe = executable.clone();
            let secs = config.seconds;
            side = Some(AbortTask(tokio::spawn(async move {
                side_loads(client, p, sender, exe, secs).await
            })));
        }
        if measured.is_some_and(|t| t.elapsed().as_secs_f64() >= config.seconds) && !stop_sent {
            stop_sent = true;
            stopped_at = Some(Instant::now());
            tokio::fs::remove_file(config.run_dir.join("go"))
                .await
                .map_err(|e| e.to_string())?;
            // Producers stop before the idle witness. Their last snapshot is
            // an explicit PAUSED row, so a quiet interval cannot hide backlog.
            if let Some(task) = side.take() {
                sides = task.finish().await?;
            }
        }
        if stop_sent && pane_state.paused && progress.phase() == MEASURE {
            observed_seconds = measured.unwrap().elapsed().as_secs_f64();
            progress.set_phase(SETTLE);
        }
        tokio::select! {
            biased;
            command=commands.recv()=>match command {
                Some(Control::Probe(id))=>{
                    pending_probe=Some(id);progress.probe.store(id,Ordering::Release);
                    if let Some(gui)=&gui {
                        gui.actions.send(GuiAction::Input {seat:panes[0].0,generation:1,id:Ulid::new(),bytes:format!("R5P{id:08X}\n").into_bytes()}).await.map_err(|_|"r5-gui-actions-closed")?;
                    }
                }
                Some(Control::Finish(metrics))=>{
                    let (failures,timing_failures)=assess(config,&metrics,&sides);
                    return Ok(Report {
                        kind:if config.real {"real-pty"} else {"synthetic-render-state"},
                        rows:records.len(), real_ptys:pids.len(), watched:config.watched,
                        seconds:config.seconds, observed_seconds, metrics,
                        stdout_bytes:sides.stdout_bytes,
                        bus_published:sides.bus_published, bus_delivered:sides.bus_delivered,
                        bus_hz:sides.bus_hz, bus_seconds:sides.bus_seconds,
                        fault_read_ms:sides.fault_ms,
                        synthetic_states_consumed:progress.synthetic.get().map_or(0,|s|s.consumed.load(Ordering::Relaxed)),
                        owned_producer_pids:pids.clone(),
                        background_pid:progress.background.get().and_then(bbt_rt::TaskId::pid),
                        host_load_before:[0.;3], host_load_after:[0.;3],
                        timing_gated:config.frame_gate, timing_failures, failures,
                        unproven:vec!["1000 real PTYs on this host","120Hz native panel",
                            "physical keystroke/scanout/photon",
                            "exhaustive framework/OS lock and IO tracing","quinn non-answering peer"],
                    });
                }
                None=>return Err("r5-ui-commands-closed".into()),
            },
            result=async {(&mut gui.as_mut().unwrap().task).await},if gui.is_some()=>{
                return result.map_err(|e|e.to_string())?.and(Err("r5-gui-task-stopped".into()));
            },
            _=tick.tick()=>{
                if let Some(work)=&mut work && progress.phase()<SETTLE {
                    sequence+=1;
                    if let Some(states)=progress.synthetic.get(){for value in &states.values{value.publish(sequence);}}
                    for (i,((seat,view),term)) in panes.iter().zip(&mut terms).enumerate() {
                        let marker=if stop_sent{"R5PAUSED".to_owned()}else if i==0 && pending_probe.is_some(){format!("R5P{:08X}{}",pending_probe.unwrap(),if sequence.is_multiple_of(2){'|'}else{'/'})}else{format!("R5 {:04X}",sequence & 0xffff)};
                        term.advance(format!("\x1b[H\x1b[2J{marker}\r\nsynthetic {i} revision {sequence}\r\n").as_bytes());
                        work.grids.apply(*seat,&Update::Snapshot {snapshot:Box::new(Snapshot {view:*view,sequence,screen:bbt_proto::Screen::from_term(term,String::new(),None,None)})}).map_err(|e|e.to_string())?;
                    }
                    progress.updates.fetch_add(config.seats as u64,Ordering::Relaxed);
                    publish(&records,&work.grids,&mut work.previous,&mut work.preparer,&updates)?;
                }
            },
        }
    }
}

#[derive(Default)]
struct SideStats {
    stdout_bytes: u64,
    bus_published: u64,
    bus_delivered: u64,
    bus_hz: f64,
    bus_seconds: f64,
    fault_ms: f64,
}

fn assess(
    config: &Config,
    metrics: &Measurements,
    sides: &SideStats,
) -> (Vec<String>, Vec<String>) {
    let mut failures = Vec::new();
    if metrics.encoded_changes.len() != config.watched
        || metrics.encoded_changes.values().any(|n| *n < 2)
    {
        failures.push("non-advancing-watched-pane".into());
    }
    if metrics.idle_frames != 0 || metrics.idle_callbacks != 0 {
        failures.push("idle-frame-or-callback".into());
    }
    if metrics.guard_violations != 0 {
        failures.push("ui-blocking-guard-violation".into());
    }
    if sides.stdout_bytes != 1_000_000_000 {
        failures.push("stdout-not1GB".into());
    }
    if sides.fault_ms < 2000. {
        failures.push("file-read-not-stalled2s".into());
    }
    if metrics.probe_clipped {
        failures.push("probe-glyphs-clipped-by-pane".into());
    }
    let mut timing = Vec::new();
    if metrics.frames < 30 {
        timing.push("insufficient-measured-frames".into());
    }
    if metrics.cpu_p99_ms > 4. {
        timing.push("ui-cpu-p99-over4ms".into());
    }
    if metrics
        .input_to_gpu_complete_ms
        .is_none_or(|ms| ms > metrics.input_budget_ms)
    {
        timing.push("input-to-gpu-over-two-display-intervals".into());
    }
    if sides.bus_hz < 10_000. {
        timing.push("bus-below10k-events-per-second".into());
    }
    if config.budget.is_some_and(|b| metrics.frame_work_p99_ms > b) {
        timing.push("frame-budget-exceeded".into());
    }
    if config.frame_gate {
        failures.extend(timing.clone());
    }
    (failures, timing)
}
struct AbortTask(tokio::task::JoinHandle<Result<SideStats, String>>);
impl Drop for AbortTask {
    fn drop(&mut self) {
        self.0.abort();
    }
}
impl AbortTask {
    async fn finish(mut self) -> Result<SideStats, String> {
        (&mut self.0).await.map_err(|e| e.to_string())?
    }
}
async fn side_loads(
    client: bbt_mux::logic::LogicClient,
    progress: Arc<Progress>,
    messages: UiSender,
    executable: PathBuf,
    seconds: f64,
) -> Result<SideStats, String> {
    let task = bbt_rt::spawn_command(
        vec![executable.into_os_string(), OsString::from("--r5-stdout")],
        std::env::temp_dir(),
        vec![],
    );
    let _ = progress.background.set(task.clone());
    let spawn_deadline = Instant::now() + Duration::from_secs(5);
    while task.pid().is_none() && task.status().is_none() {
        if Instant::now() > spawn_deadline {
            return Err("r5-background-spawn-timeout".into());
        }
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
    if let Some(pid) = task.pid() {
        eprintln!("R5 owned background stdout pid={pid}");
    }
    let read = tokio::spawn(bbt_rt::fault::stalled_file_read(Duration::from_secs(2)));
    let consumer = client
        .bus
        .subscribe(Consumer::gui("seat/r5/output"))
        .await
        .map_err(|e| e.to_string())?;
    let start = Instant::now();
    let mut published = 0;
    let mut delivered = 0;
    while start.elapsed().as_secs_f64() < seconds {
        let target = (start.elapsed().as_secs_f64() * 10_000.) as u64 + 128;
        while published < target && start.elapsed().as_secs_f64() < seconds {
            client
                .bus
                .publish(
                    "r5",
                    Draft::new(
                        "seat/r5/output",
                        "r5-fixture",
                        serde_json::json!({"revision":published}),
                    ),
                )
                .await
                .map_err(|e| e.to_string())?;
            published += 1;
        }
        for item in client
            .bus
            .poll(&consumer, 256)
            .await
            .map_err(|e| e.to_string())?
        {
            if matches!(item, Delivery::Event { .. }) {
                delivered += 1;
            }
        }
        progress.bus_events.store(published, Ordering::Release);
        progress.bus_delivered.store(delivered, Ordering::Release);
        let _ = messages.try_send(bbt_rt::UiMsg::Changed { source: 6 });
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
    let bus_seconds = start.elapsed().as_secs_f64();
    let bus_hz = published as f64 / bus_seconds;
    let fault = read
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;
    progress
        .fault_ns
        .store(fault.elapsed.as_nanos() as u64, Ordering::Release);
    let deadline = Instant::now() + Duration::from_secs(10);
    while task.status().is_none() {
        if Instant::now() > deadline {
            bbt_rt::cancel(&task);
            return Err("r5-stdout-timeout".into());
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    let status = task.status().unwrap();
    if status.code != Some(0) || status.error.is_some() {
        return Err(format!("r5-stdout-failed: {status:?}"));
    }
    progress
        .stdout_bytes
        .store(status.stdout_bytes, Ordering::Release);
    client
        .bus
        .unsubscribe(&consumer)
        .await
        .map_err(|e| e.to_string())?;
    Ok(SideStats {
        stdout_bytes: status.stdout_bytes,
        bus_published: published,
        bus_delivered: delivered,
        bus_hz,
        bus_seconds,
        fault_ms: fault.elapsed.as_secs_f64() * 1000.,
    })
}

#[allow(
    clippy::disallowed_methods,
    reason = "R5 helper child processes do blocking fixture IO; they never register a UI thread"
)]
pub fn private_entry(args: &[String]) -> Option<Result<(), String>> {
    use std::io::{BufRead, Write};
    if args.first().is_some_and(|s| s == "--r5-stdout") {
        return Some((|| {
            let mut out = std::io::stdout().lock();
            let block = vec![b'x'; 64 * 1024];
            let mut left = 1_000_000_000;
            while left > 0 {
                let n = left.min(block.len());
                out.write_all(&block[..n]).map_err(|e| e.to_string())?;
                left -= n;
            }
            Ok(())
        })());
    }
    if !args.first().is_some_and(|s| s == "--r5-producer") {
        return None;
    }
    Some((|| {
        let root = PathBuf::from(args.get(1).ok_or("r5-producer-root-required")?);
        let probe = Arc::new(AtomicU64::new(0));
        let input = probe.clone();
        std::thread::spawn(move || {
            for line in std::io::stdin().lock().lines().map_while(Result::ok) {
                if let Some(id) = line
                    .strip_prefix("R5P")
                    .and_then(|s| u64::from_str_radix(s.trim(), 16).ok())
                {
                    input.store(id, Ordering::Release);
                }
            }
        });
        let mut stdout = std::io::stdout().lock();
        let block = "y\r\n".repeat(2048);
        let mut sequence = 0u64;
        let mut last = Instant::now() - Duration::from_secs(1);
        let mut running = false;
        let mut bytes = 0u64;
        stdout
            .write_all(b"\x1b[H\x1b[2J\x1b[2;rR5READY\r\n")
            .map_err(|e| e.to_string())?;
        loop {
            if last.elapsed() >= Duration::from_millis(100) {
                running = root.join("go").exists();
                last = Instant::now();
                if !running {
                    stdout
                        .write_all(b"\x1b7\x1b[1;1HR5PAUSED\x1b[K\x1b8")
                        .map_err(|e| e.to_string())?;
                }
            }
            if running {
                sequence += 1;
                let id = probe.load(Ordering::Acquire);
                let header = if id == 0 {
                    format!("R5 {:04X}", sequence & 0xffff)
                } else {
                    format!(
                        "R5P{id:08X}{}",
                        if sequence.is_multiple_of(2) { '|' } else { '/' }
                    )
                };
                let prefix = format!("\x1b7\x1b[1;1H{header}\x1b[K\x1b8");
                stdout
                    .write_all(prefix.as_bytes())
                    .and_then(|_| stdout.write_all(block.as_bytes()))
                    .map_err(|e| e.to_string())?;
                bytes += (prefix.len() + block.len()) as u64;
            } else {
                stdout.flush().map_err(|e| e.to_string())?;
                std::thread::sleep(Duration::from_millis(20));
            }
            if sequence.is_multiple_of(4096) && sequence > 0 {
                let _ = std::fs::write(
                    root.join(format!("producer-{}.json", std::process::id())),
                    format!(
                        "{{\"pid\":{},\"written_bytes\":{bytes}}}",
                        std::process::id()
                    ),
                );
            }
        }
    })())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct ScriptedConnection {
        calls: Arc<Mutex<Vec<Action>>>,
        replies: VecDeque<Reply>,
    }
    impl SupervisorConnection for ScriptedConnection {
        fn request(&mut self, action: Action) -> bbt_mux::store::Result<Reply> {
            io_lock(&self.calls).push(action);
            Ok(self
                .replies
                .pop_front()
                .expect("unexpected diagnostic request"))
        }
        fn receive(&mut self, _: Duration) -> bbt_mux::store::Result<Option<Event>> {
            Ok(None)
        }
    }

    fn ring_status() -> bbt_mux::ring::Status {
        bbt_mux::ring::Status {
            generation: 1,
            first_sequence: 0,
            captured_sequence: 2,
            committed_sequence: 1,
            byte_start: 0,
            byte_end: 8,
            hot_bytes: 8,
            paused: false,
            recovery_notice: None,
            capture: Default::default(),
        }
    }

    fn output_page(sequence: u64, bytes: &[u8], next: Option<u64>, gap: bool) -> Reply {
        Reply::RingHistory {
            history: Box::new(bbt_mux::ring::History {
                status: ring_status(),
                records: vec![bbt_mux::ring::IndexedEvent {
                    event: Event::Output {
                        seat: Ulid::from(1),
                        generation: 1,
                        seq: sequence,
                        bytes: bytes.into(),
                    },
                    row: None,
                }],
                gap: gap.then(|| bbt_mux::ring::HistoryGap {
                    requested_from: 1,
                    requested_to: 2,
                    retained_from: Some(2),
                    retained_to: Some(2),
                    reason: "fixture gap".into(),
                }),
                next,
            }),
        }
    }

    fn scripted_probe(pages: Vec<Reply>) -> (RingProbe, Arc<Mutex<Vec<Action>>>) {
        let attachment = Attachment {
            seat: Ulid::from(1),
            generation: 1,
            writer_lease: Ulid::from(99),
        };
        let mut replies = VecDeque::from([
            Reply::Attached {
                attachment,
                info: bbt_mux::SeatInfo {
                    seat: Ulid::from(1),
                    generation: 1,
                    pid: 1,
                    size: Default::default(),
                    next_sequence: 1,
                    exited: None,
                },
                first_available_sequence: 1,
            },
            Reply::RingRecovery {
                status: Box::new(ring_status()),
                checkpoint: None,
            },
        ]);
        replies.extend(pages);
        let calls = Arc::new(Mutex::new(Vec::new()));
        let probe = RingProbe::new(ScriptedConnection {
            calls: calls.clone(),
            replies,
        });
        // The normal logic's original Attach is observed, never re-issued by
        // the diagnostic. No AcquireLogic or second connection exists here.
        ObservedConnection(probe.clone())
            .request(Action::Attach {
                seat: Ulid::from(1),
                generation: 1,
            })
            .unwrap();
        (probe, calls)
    }

    #[test]
    fn ring_probe_follows_pages_and_never_combines_marker_bytes_across_a_gap() {
        for gap in [false, true] {
            let (probe, calls) = scripted_probe(vec![
                output_page(1, b"R5PA", Some(2), false),
                output_page(2, b"USED", None, gap),
            ]);
            let mut row = RingWitness::default();
            probe
                .tail(
                    Ulid::from(1),
                    Instant::now() + Duration::from_secs(60),
                    &mut row,
                )
                .unwrap();
            assert_eq!(
                row.paused_in_examined_tail,
                if gap { None } else { Some(true) }
            );
            assert_eq!(row.paused_marker_sequence, if gap { None } else { Some(2) });
            assert_eq!(row.captured_sequence, Some(2));
            assert_eq!(row.committed_sequence, Some(1));
            assert!(row.tail_complete);
            let calls = io_lock(&calls);
            assert_eq!(calls.len(), 4);
            assert!(matches!(&calls[1], Action::RingRecovery { .. }));
            assert!(matches!(&calls[2], Action::ReadRing { from: 1, to: 2, .. }));
            assert!(matches!(&calls[3], Action::ReadRing { from: 2, to: 2, .. }));
            assert!(
                !serde_json::to_string(&row)
                    .unwrap()
                    .contains("writer_lease")
            );
        }
    }

    #[test]
    fn failed_tail_keeps_known_ring_cursor_and_does_not_report_marker_absence() {
        let (probe, _) = scripted_probe(vec![output_page(1, b"R5PA", Some(1), false)]);
        let mut row = RingWitness::default();
        row.error = probe
            .tail(
                Ulid::from(1),
                Instant::now() + Duration::from_secs(60),
                &mut row,
            )
            .err();
        assert!(row.error.as_ref().unwrap().contains("did not advance"));
        assert_eq!(row.captured_sequence, Some(2));
        assert_eq!(row.paused_in_examined_tail, None);
        assert!(!row.tail_complete);
        let (probe, _) = scripted_probe(vec![output_page(2, b"no marker", None, false)]);
        let rows = probe.collect(vec![Ulid::from(1), Ulid::from(2)], Instant::now());
        assert_eq!(rows[&Ulid::from(1)].paused_in_examined_tail, Some(false));
        assert!(rows[&Ulid::from(2)].error.is_some());
        assert_eq!(rows[&Ulid::from(2)].paused_in_examined_tail, None);
    }

    #[test]
    fn settle_diagnostic_names_the_active_predicates_without_inventing_ui_results() {
        let probe = NativeSettleProbe::default();
        let predicates = false_settle_predicates(MEASURE, PaneState::default(), probe.snapshot());
        assert!(predicates.contains(&"IO: every watched published Screen contains R5PAUSED"));
        assert!(predicates.contains(&"UI: Native::poll has been observed"));
        assert!(!predicates.iter().any(|p| p.contains("GPU")));
        let native = NativeSettleWitness {
            observed: true,
            phase: SETTLE,
            pending: true,
            has_paused_frame: true,
            gpu_complete: false,
            stable_counts: false,
            frames: 12,
            callbacks: 13,
            paused_frame: Some(12),
            completed_frame: 11,
        };
        probe.record(native);
        let snapshot = probe.snapshot();
        assert_eq!(snapshot.paused_frame, Some(12));
        assert_eq!(snapshot.completed_frame, 11);
        let predicates = false_settle_predicates(
            SETTLE,
            PaneState {
                ready: true,
                paused: true,
            },
            snapshot,
        );
        assert_eq!(
            predicates,
            [
                "UI: no pending work",
                "UI: paused frame reached GPU completion",
                "UI: frame and callback counts match the previous settle poll"
            ]
        );
    }

    #[tokio::test]
    #[allow(
        clippy::disallowed_methods,
        reason = "non-UI diagnostic test owns its memory-custody fixture and temporary report"
    )]
    async fn timeout_prints_and_saves_each_seat_without_inventing_unobserved_backend_results() {
        let root = tempfile::tempdir().unwrap();
        let config = Config {
            real: false,
            headless: true,
            frame_gate: false,
            seats: 2,
            watched: 2,
            seconds: 3.,
            idle: 1.,
            report: root.path().join("report.json"),
            run_dir: root.path().join("fixture"),
            budget: None,
        };
        // An explicit fixture issuer works even without a memory-selector
        // environment variable; this unit test never invokes real custody.
        let fixture = config.run_dir.clone();
        let core = bbt_rt::handle()
            .spawn_blocking(move || {
                let repository = fixture.join("repository");
                std::fs::create_dir_all(repository.join(".bboxy")).unwrap();
                let supervisor = Supervisor::start(bbt_mux::Config {
                    journal_directory: Some(fixture.join("journal")),
                    ..Default::default()
                })
                .unwrap();
                let ring_probe = RingProbe::new(supervisor.connect().unwrap());
                let logic = Logic::start(
                    ObservedConnection(ring_probe.clone()),
                    LogicConfig {
                        run_dir: fixture.join("logic"),
                        repository,
                        ring: Default::default(),
                    },
                    ed25519_dalek::SigningKey::from_bytes(&[73; 32]),
                    bbt_rt::handle(),
                )
                .unwrap();
                Core {
                    logic,
                    supervisor,
                    ring_probe,
                }
            })
            .await
            .unwrap();
        let records: Vec<_> = (1..=2)
            .map(|id| SeatRecord::new(Ulid::from(id), Ulid::from(1), format!("pane-{id}"), "zsh"))
            .collect();
        let progress = Progress::default();
        progress.set_phase(MEASURE);
        let gui = GuiTrace::new([SeatId::from_u128(1), SeatId::from_u128(2)]);
        let cursor = UpdateCursor {
            view: View {
                seat: Ulid::from(1),
                generation: 1,
                view_generation: Ulid::from(9),
            },
            sequence: 7,
        };
        gui.record(SeatId::from_u128(1), |row| {
            row.received = Some(cursor);
            row.applied = Some(cursor);
            row.applied_queued = Some(cursor);
            row.applied_has_paused = Some(false);
        });
        let error = settle_timeout(
            "forced diagnostic fixture timeout",
            &config,
            &core,
            (&records, &[]),
            &progress,
            PaneState {
                ready: true,
                paused: false,
            },
            &gui,
        )
        .await;
        core.close().await.unwrap();
        let bytes = tokio::fs::read(config.report.with_extension("diagnostics.json"))
            .await
            .unwrap();
        let table: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert!(error.starts_with("forced diagnostic fixture timeout; table="));
        assert!(error.contains("pane-1") && error.contains("pane-2"));
        assert_eq!(table["seats"].as_array().unwrap().len(), 2);
        assert_eq!(
            table["seats"][0]["gui_at_timeout"]["received"]["sequence"],
            7
        );
        assert_eq!(table["seats"][0]["applied_written_to_socket"], false);
        assert!(table["seats"][1]["applied_written_to_socket"].is_null());
        assert!(table["seats"][0]["logic_current_snapshot"]["cursor"].is_null());
        assert!(table["seats"][0]["supervisor_ring_tail"]["paused_in_examined_tail"].is_null());
        assert!(
            !table["false_settle_predicates"]
                .as_array()
                .unwrap()
                .is_empty()
        );
    }

    async fn write_reply(stream: &mut tokio::net::unix::OwnedWriteHalf, reply: &GuiReply) {
        let mut bytes = serde_json::to_vec(reply).unwrap();
        bytes.push(b'\n');
        stream.write_all(&bytes).await.unwrap();
    }

    #[tokio::test]
    #[allow(
        clippy::disallowed_methods,
        reason = "tokio::test owns a non-UI runtime for this socket regression"
    )]
    async fn gui_keeps_applying_and_acknowledging_while_side_finish_is_held() {
        const COUNT: u64 = 128;
        let (client, server) = UnixStream::pair().unwrap();
        let trace = Arc::new(GuiTrace::new([SeatId::from_u128(1)]));
        let published = Arc::new(AtomicU64::new(0));
        let applied = published.clone();
        let gui = GuiTask::start(
            Wire::from_stream(client, [0; 32], trace.clone()),
            VecDeque::new(),
            move |_, update| {
                let Update::Snapshot { snapshot } = update else {
                    panic!("fixture sends snapshots")
                };
                applied.store(snapshot.sequence, Ordering::Release);
                Ok(false)
            },
        );
        let (release, held) = oneshot::channel();
        let side = AbortTask(tokio::spawn(async move {
            held.await.map_err(|e| e.to_string())?;
            Ok(SideStats::default())
        }));
        let waiting = tokio::spawn(side.finish());
        let peer = tokio::spawn(async move {
            let (read, mut write) = server.into_split();
            let mut read = BufReader::new(read);
            let seat = SeatId::from_u128(1);
            let view = View {
                seat: Ulid::from(1),
                generation: 1,
                view_generation: Ulid::new(),
            };
            let screen = bbt_proto::Screen::from_term(
                &bbt_core::Term::new(2, 1, 0),
                String::new(),
                None,
                None,
            );
            // More than the bounded8 reader and bounded64 writer combined.
            for sequence in 1..=COUNT {
                write_reply(
                    &mut write,
                    &GuiReply::Update {
                        seat,
                        update: Update::Snapshot {
                            snapshot: Box::new(Snapshot {
                                view,
                                sequence,
                                screen: screen.clone(),
                            }),
                        },
                    },
                )
                .await;
            }
            for sequence in 1..=COUNT {
                let bytes = bbt_mux::seat_socket::read_line(&mut read)
                    .await
                    .unwrap()
                    .unwrap();
                let request: GuiRequest = serde_json::from_slice(&bytes).unwrap();
                assert!(
                    matches!(request.action, GuiAction::Applied {sequence: applied, ..} if applied == sequence)
                );
                assert!(published.load(Ordering::Acquire) >= sequence);
            }
        });
        // A deadlock watchdog, not a latency/performance assertion. Completion
        // is driven by ACK counts while the side future remains unreleased.
        tokio::time::timeout(Duration::from_secs(60), peer)
            .await
            .unwrap()
            .unwrap();
        assert!(!waiting.is_finished());
        let row = trace.snapshot().remove(&SeatId::from_u128(1)).unwrap();
        assert_eq!(row.received.unwrap().sequence, COUNT);
        assert_eq!(row.applied.unwrap().sequence, COUNT);
        assert_eq!(row.applied_written, row.applied);
        assert_eq!(row.applied_has_paused, Some(false));
        release.send(()).unwrap();
        waiting.await.unwrap().unwrap();
        drop(gui);
    }

    #[tokio::test]
    #[allow(
        clippy::disallowed_methods,
        reason = "tokio::test owns a non-UI runtime for this socket regression"
    )]
    async fn gui_errors_keep_the_operation_and_identify_the_consumer_path() {
        for (op, expected) in [
            (Ulid::nil(), "server periodic view check"),
            (
                Ulid::from(7),
                "server control action (uncorrelated operation)",
            ),
        ] {
            let (client, server) = UnixStream::pair().unwrap();
            let mut gui = GuiTask::start(
                Wire::from_stream(client, [0; 32], Arc::new(GuiTrace::default())),
                VecDeque::new(),
                |_, _| Ok(false),
            );
            let (_, mut write) = server.into_split();
            write_reply(
                &mut write,
                &GuiReply::Error {
                    op,
                    reason: "deadline has elapsed".into(),
                },
            )
            .await;
            let error = tokio::time::timeout(Duration::from_secs(60), &mut gui.task)
                .await
                .unwrap()
                .unwrap()
                .unwrap_err();
            assert!(error.contains(&format!("op={op}")), "{error}");
            assert!(error.contains(expected), "{error}");
            assert!(error.contains("stall/load.rs:"), "{error}");
            assert!(error.contains("deadline has elapsed"), "{error}");
        }
    }

    #[test]
    fn probe_row_fits_every_fifteen_and_fifty_pane_fixture() {
        for count in [15, 50] {
            let seats: Vec<_> = (1..=count).map(SeatId::from_u128).collect();
            let tree = fixture_grid(&seats);
            let solved = bbt_ui_model::layout::solve_split(
                &tree,
                bbt_ui_model::layout::Rect::new(0., 0., 1280., 860.),
                4.,
            );
            assert_eq!(solved.panes.len(), count as usize);
            for (_, rect) in solved.panes {
                assert!(rect.w >= 120. && rect.h >= 100., "{count}: {rect:?}");
            }
        }
    }

    #[test]
    fn functional_ci_reports_timing_failures_and_the_slow_switch_enforces_them() {
        let mut config = Config {
            real: false,
            headless: true,
            frame_gate: false,
            seats: 1000,
            watched: 15,
            seconds: 3.,
            idle: 1.,
            report: PathBuf::new(),
            run_dir: PathBuf::new(),
            budget: Some(4.),
        };
        let metrics = Measurements {
            frames: 30,
            cpu_p99_ms: 5.,
            frame_work_p99_ms: 5.,
            input_to_gpu_complete_ms: Some(30.),
            input_budget_ms: 2000. / 120.,
            encoded_changes: (0..15).map(|id| (id.to_string(), 2)).collect(),
            ..Default::default()
        };
        let sides = SideStats {
            stdout_bytes: 1_000_000_000,
            fault_ms: 2000.,
            bus_hz: 9999.,
            ..Default::default()
        };
        let (functional, timing) = assess(&config, &metrics, &sides);
        assert!(functional.is_empty());
        assert_eq!(timing.len(), 4);
        config.frame_gate = true;
        let (gated, same_timing) = assess(&config, &metrics, &sides);
        assert_eq!(same_timing, timing);
        assert_eq!(gated, timing);
    }
}
