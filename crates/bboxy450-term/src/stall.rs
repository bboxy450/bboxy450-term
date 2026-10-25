//! R5 verification only. Synthetic UI fixtures and real PTYs are reported
//! separately; neither a submitted frame nor a GPU completion is a photon.
mod headless;
mod load;

use bboxy450_term::model::Model;
use bbt_proto::SeatId;
use bbt_rt::{Latest, Phases, UiSender};
use serde::{Serialize, Serializer};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU8, AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};
use tokio::sync::{mpsc, oneshot};

pub const HELP: &str = "  --stall-test          R5 functional fixture (debug guards required)\n  --stall-headless      render without an NSWindow\n  --stall-real          real PTYs; otherwise explicitly synthetic state fixtures\n  --stall-frame-gate    also enforce timing limits; requires starting loadavg <=8\n  --stall-seats N       fixture rows / real PTYs (default 1000 synthetic)\n  --stall-watched N     encoded watched panes (default 15)\n  --stall-seconds N     measured overlap (default 5, range 3..60)\n  --stall-idle N        settled idle seconds (default 2)\n  --stall-report PATH   JSON result (required)\n  --stall-run-dir PATH  fresh private directory (default /tmp/r5-<id>)";

#[derive(Clone, Debug)]
pub struct Config {
    pub real: bool,
    pub headless: bool,
    pub frame_gate: bool,
    pub seats: usize,
    pub watched: usize,
    pub seconds: f64,
    pub idle: f64,
    pub report: PathBuf,
    pub run_dir: PathBuf,
    pub budget: Option<f64>,
}
impl Config {
    pub fn parse(args: &[String]) -> Result<Option<Self>, String> {
        if !args.iter().any(|a| a == "--stall-test") {
            return Ok(None);
        }
        if !cfg!(debug_assertions) || std::env::var("BBOXY_TEST_CUSTODY").as_deref() != Ok("memory")
        {
            return Err("r5-memory-custody-required: build with debug assertions and set BBOXY_TEST_CUSTODY=memory".into());
        }
        let value = |name: &str| -> Result<Option<&str>, String> {
            let Some(at) = args.iter().position(|a| a == name) else {
                return Ok(None);
            };
            args.get(at + 1)
                .filter(|a| !a.starts_with("--"))
                .map(|s| Some(s.as_str()))
                .ok_or_else(|| format!("r5-missing-value: {name}"))
        };
        let number = |name: &str, default: usize| -> Result<usize, String> {
            value(name)?.map_or(Ok(default), |s| {
                s.parse().map_err(|_| format!("r5-invalid-number: {name}"))
            })
        };
        let seats = number("--stall-seats", 1000)?;
        let watched = number("--stall-watched", 15)?;
        let seconds = number("--stall-seconds", 5)? as f64;
        let idle = number("--stall-idle", 2)? as f64;
        if seats == 0
            || seats > 1000
            || watched == 0
            || watched > 50
            || watched > seats
            || !(3. ..=60.).contains(&seconds)
            || !(1. ..=30.).contains(&idle)
        {
            return Err("r5-invalid-load-bounds".into());
        }
        let budget = std::env::var("GATE_BUDGET")
            .ok()
            .map(|s| s.parse::<f64>().map_err(|_| "r5-invalid-budget"))
            .transpose()
            .map_err(str::to_owned)?;
        if budget.is_some_and(|b| !b.is_finite() || b <= 0.) {
            return Err("r5-invalid-budget".into());
        }
        Ok(Some(Self {
            real: args.iter().any(|a| a == "--stall-real"),
            headless: args.iter().any(|a| a == "--stall-headless"),
            frame_gate: args.iter().any(|a| a == "--stall-frame-gate"),
            seats,
            watched,
            seconds,
            idle,
            report: value("--stall-report")?.ok_or("r5-report-required")?.into(),
            run_dir: value("--stall-run-dir")?.map_or_else(
                || std::env::temp_dir().join(format!("r5-{}", bbt_mux::Ulid::new())),
                PathBuf::from,
            ),
            budget,
        }))
    }
}

pub const SETUP: u8 = 0;
pub const WARM: u8 = 1;
pub const MEASURE: u8 = 2;
pub const SETTLE: u8 = 3;
pub const IDLE: u8 = 4;
pub const FINISHED: u8 = 5;

const WAKE_SLOTS: usize = 16;
/// Redraw requests outside a DirtySet Wake use 1024, outside its source range.
pub const DIRECT_REDRAW_SOURCE: usize = 1024;

#[derive(Debug, Default, Clone, Copy, Serialize)]
struct WakeSourceCounts {
    source: usize,
    wakes: u64,
    frame_drains: u64,
    redraws: u64,
    frames: u64,
}
impl WakeSourceCounts {
    fn used(&self) -> bool {
        self.wakes | self.frame_drains | self.redraws | self.frames != 0
    }
}

fn serialize_sources<S: Serializer>(
    sources: &[WakeSourceCounts; WAKE_SLOTS],
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.collect_seq(sources.iter().filter(|source| source.used()))
}

/// Bounded numerical UI observations. A frame may have several source causes;
/// these attribution counts are not additive frame or event totals.
#[derive(Debug, Default, Clone, Copy, Serialize)]
pub struct WakeDiagnostics {
    #[serde(serialize_with = "serialize_sources")]
    sources: [WakeSourceCounts; WAKE_SLOTS],
    wake_events: u64,
    empty_wakes: u64,
    ignored_ticks: u64,
    overflow: u64,
    pending_redraw: bool,
    pending_messages: bool,
    pending_output: bool,
}
impl WakeDiagnostics {
    /// Snapshot the three independent pending predicates without a queue read on IO.
    pub fn with_pending(mut self, redraw: bool, messages: bool, output: bool) -> Self {
        self.pending_redraw = redraw;
        self.pending_messages = messages;
        self.pending_output = output;
        self
    }
    fn pending(&self) -> bool {
        self.pending_redraw || self.pending_messages || self.pending_output
    }
}

/// UI-owned counters, with no IO or locks. The app currently uses six dirty
/// sources; overflow is explicit if a future producer exceeds the 16 slots.
#[derive(Default)]
pub struct WakeCounters {
    values: WakeDiagnostics,
    pending: [bool; WAKE_SLOTS],
}
impl WakeCounters {
    fn slot(&mut self, source: usize) -> Option<usize> {
        let slot = self
            .values
            .sources
            .iter()
            .position(|row| row.used() && row.source == source)
            .or_else(|| self.values.sources.iter().position(|row| !row.used()));
        let Some(slot) = slot else {
            self.values.overflow = self.values.overflow.saturating_add(1);
            return None;
        };
        self.values.sources[slot].source = source;
        Some(slot)
    }
    /// Record one delivered Wake and the source flags it consumed.
    pub fn wake(&mut self, sources: &[usize]) {
        self.values.wake_events = self.values.wake_events.saturating_add(1);
        if sources.is_empty() {
            self.values.empty_wakes = self.values.empty_wakes.saturating_add(1);
        }
        for &source in sources {
            if let Some(slot) = self.slot(source) {
                let row = &mut self.values.sources[slot];
                row.wakes = row.wakes.saturating_add(1);
            }
        }
    }
    /// Attribute flags consumed by a frame before their queued Wake is delivered.
    pub fn drained(&mut self, sources: &[usize]) {
        for &source in sources {
            if let Some(slot) = self.slot(source) {
                let row = &mut self.values.sources[slot];
                row.frame_drains = row.frame_drains.saturating_add(1);
                self.pending[slot] = true;
            }
        }
    }
    /// Attribute a redraw request to each of its coalesced causes.
    pub fn redraw(&mut self, sources: &[usize]) {
        for &source in sources {
            if let Some(slot) = self.slot(source) {
                let row = &mut self.values.sources[slot];
                row.redraws = row.redraws.saturating_add(1);
                self.pending[slot] = true;
            }
        }
    }
    /// Attribute a submitted frame, then retire its pending causes.
    pub fn frame(&mut self) {
        for (row, pending) in self.values.sources.iter_mut().zip(&mut self.pending) {
            if std::mem::take(pending) {
                row.frames = row.frames.saturating_add(1);
            }
        }
    }
    /// Count a delivered tick that could not request new display work.
    pub fn ignored_tick(&mut self) {
        self.values.ignored_ticks = self.values.ignored_ticks.saturating_add(1);
    }
    /// Copy counters; no reset, queue drain or IO occurs.
    pub fn snapshot(&self) -> WakeDiagnostics {
        self.values
    }
}

#[derive(Default)]
struct WakeSourceProbe {
    source: AtomicU64,
    wakes: AtomicU64,
    frame_drains: AtomicU64,
    redraws: AtomicU64,
    frames: AtomicU64,
}

#[derive(Default)]
struct WakeProbe {
    sources: [WakeSourceProbe; WAKE_SLOTS],
    wake_events: AtomicU64,
    empty_wakes: AtomicU64,
    ignored_ticks: AtomicU64,
    overflow: AtomicU64,
    pending: AtomicU8,
}
impl WakeProbe {
    fn record(&self, value: WakeDiagnostics) {
        for (target, row) in self.sources.iter().zip(value.sources) {
            target.source.store(row.source as u64, Ordering::Relaxed);
            target.wakes.store(row.wakes, Ordering::Relaxed);
            target
                .frame_drains
                .store(row.frame_drains, Ordering::Relaxed);
            target.redraws.store(row.redraws, Ordering::Relaxed);
            target.frames.store(row.frames, Ordering::Relaxed);
        }
        self.wake_events.store(value.wake_events, Ordering::Relaxed);
        self.empty_wakes.store(value.empty_wakes, Ordering::Relaxed);
        self.ignored_ticks
            .store(value.ignored_ticks, Ordering::Relaxed);
        self.overflow.store(value.overflow, Ordering::Relaxed);
        self.pending.store(
            u8::from(value.pending_redraw)
                | (u8::from(value.pending_messages) << 1)
                | (u8::from(value.pending_output) << 2),
            Ordering::Relaxed,
        );
    }
    fn snapshot(&self) -> WakeDiagnostics {
        let pending = self.pending.load(Ordering::Relaxed);
        WakeDiagnostics {
            sources: std::array::from_fn(|i| {
                let row = &self.sources[i];
                WakeSourceCounts {
                    source: row.source.load(Ordering::Relaxed) as usize,
                    wakes: row.wakes.load(Ordering::Relaxed),
                    frame_drains: row.frame_drains.load(Ordering::Relaxed),
                    redraws: row.redraws.load(Ordering::Relaxed),
                    frames: row.frames.load(Ordering::Relaxed),
                }
            }),
            wake_events: self.wake_events.load(Ordering::Relaxed),
            empty_wakes: self.empty_wakes.load(Ordering::Relaxed),
            ignored_ticks: self.ignored_ticks.load(Ordering::Relaxed),
            overflow: self.overflow.load(Ordering::Relaxed),
            pending_redraw: pending & 1 != 0,
            pending_messages: pending & 2 != 0,
            pending_output: pending & 4 != 0,
        }
    }
}

#[derive(Default)]
pub struct Progress {
    phase: AtomicU8,
    pub poll_pending: AtomicBool,
    pub probe: AtomicU64,
    pub rows: AtomicU64,
    pub updates: AtomicU64,
    pub bus_events: AtomicU64,
    pub bus_delivered: AtomicU64,
    pub stdout_bytes: AtomicU64,
    pub fault_ns: AtomicU64,
    pub synthetic: std::sync::OnceLock<SyntheticStates>,
    pub background: std::sync::OnceLock<bbt_rt::TaskId>,
    native_settle: NativeSettleProbe,
}

/// UI writes only atomics. The timeout diagnostic reads the last observed
/// predicates without asking the UI to take an IO-owned lock.
#[derive(Default)]
struct NativeSettleProbe {
    flags: AtomicU64,
    frames: AtomicU64,
    callbacks: AtomicU64,
    paused_frame: AtomicU64,
    completed_frame: AtomicU64,
    wakes: WakeProbe,
    observed_elapsed_ns: AtomicU64,
}

#[derive(Debug, Default, Clone, Copy, Serialize)]
struct NativeSettleWitness {
    observed: bool,
    phase: u8,
    pending: bool,
    has_paused_frame: bool,
    gpu_complete: bool,
    stable_counts: bool,
    frames: u64,
    callbacks: u64,
    paused_frame: Option<u64>,
    completed_frame: u64,
    wakes: WakeDiagnostics,
    observed_elapsed_ns: u64,
}
impl NativeSettleProbe {
    fn record(&self, witness: NativeSettleWitness) {
        self.wakes.record(witness.wakes);
        self.observed_elapsed_ns
            .store(witness.observed_elapsed_ns, Ordering::Relaxed);
        self.frames.store(witness.frames, Ordering::Relaxed);
        self.callbacks.store(witness.callbacks, Ordering::Relaxed);
        self.paused_frame
            .store(witness.paused_frame.unwrap_or(0), Ordering::Relaxed);
        self.completed_frame
            .store(witness.completed_frame, Ordering::Relaxed);
        let flags = u64::from(witness.observed)
            | (u64::from(witness.pending) << 1)
            | (u64::from(witness.has_paused_frame) << 2)
            | (u64::from(witness.gpu_complete) << 3)
            | (u64::from(witness.stable_counts) << 4)
            | (u64::from(witness.phase) << 8);
        self.flags.store(flags, Ordering::Release);
    }

    fn snapshot(&self) -> NativeSettleWitness {
        let flags = self.flags.load(Ordering::Acquire);
        let has_paused_frame = flags & 4 != 0;
        NativeSettleWitness {
            observed: flags & 1 != 0,
            phase: (flags >> 8) as u8,
            pending: flags & 2 != 0,
            has_paused_frame,
            gpu_complete: flags & 8 != 0,
            stable_counts: flags & 16 != 0,
            frames: self.frames.load(Ordering::Relaxed),
            callbacks: self.callbacks.load(Ordering::Relaxed),
            paused_frame: has_paused_frame.then(|| self.paused_frame.load(Ordering::Relaxed)),
            completed_frame: self.completed_frame.load(Ordering::Relaxed),
            wakes: self.wakes.snapshot(),
            observed_elapsed_ns: self.observed_elapsed_ns.load(Ordering::Relaxed),
        }
    }
}
pub struct SyntheticStates {
    pub dirty: bbt_rt::DirtySet,
    pub values: Vec<Latest<u64>>,
    pub consumed: AtomicU64,
}
impl Progress {
    pub fn phase(&self) -> u8 {
        self.phase.load(Ordering::Acquire)
    }
    pub fn set_phase(&self, phase: u8) {
        bbt_rt::diagnostics::set_period(phase);
        self.phase.store(phase, Ordering::Release);
    }
}

pub enum Control {
    Probe(u64),
    Finish(Measurements),
}

#[derive(Serialize, Default)]
pub struct Measurements {
    pub cpu_p50_ms: f64,
    pub cpu_p99_ms: f64,
    pub frame_work_p50_ms: f64,
    pub frame_work_p99_ms: f64,
    pub frames: usize,
    pub hz: f64,
    pub input_to_gpu_complete_ms: Option<f64>,
    pub input_to_prepared_model_ms: Option<f64>,
    pub input_to_encoded_frame_ms: Option<f64>,
    pub input_budget_ms: f64,
    pub encoded_changes: BTreeMap<String, u64>,
    pub idle_frames: u64,
    pub idle_callbacks: u64,
    pub guard_violations: u64,
    pub cpu_samples_ms: Vec<f64>,
    pub frame_work_samples_ms: Vec<f64>,
    pub probe_clipped: bool,
    pub diagnostics: serde_json::Value,
}

#[derive(Serialize)]
pub struct Report {
    pub kind: &'static str,
    pub rows: usize,
    pub real_ptys: usize,
    pub watched: usize,
    pub seconds: f64,
    pub observed_seconds: f64,
    pub metrics: Measurements,
    pub stdout_bytes: u64,
    pub bus_published: u64,
    pub bus_delivered: u64,
    pub bus_hz: f64,
    pub bus_seconds: f64,
    pub fault_read_ms: f64,
    pub synthetic_states_consumed: u64,
    pub owned_producer_pids: Vec<u32>,
    pub background_pid: Option<u32>,
    pub host_load_before: [f64; 3],
    pub host_load_after: [f64; 3],
    pub timing_gated: bool,
    pub timing_failures: Vec<String>,
    pub failures: Vec<String>,
    pub unproven: Vec<&'static str>,
}
impl Report {
    pub fn gate_line(&self, config: &Config) -> String {
        format!(
            "GATE frame value={:.6} unit=ms budget={} {} rows={} real_ptys={} watched={} frames={} cpu_p99_ms={:.6} input_gpu_ms={:?} idle_frames={} idle_callbacks={} timing_gated={} timing_failures={:?} failures={:?}",
            self.metrics.frame_work_p99_ms,
            config.budget.map_or("none".into(), |b| b.to_string()),
            if self.failures.is_empty() {
                "PASS"
            } else {
                "FAIL"
            },
            self.rows,
            self.real_ptys,
            self.watched,
            self.metrics.frames,
            self.metrics.cpu_p99_ms,
            self.metrics.input_to_gpu_complete_ms,
            self.metrics.idle_frames,
            self.metrics.idle_callbacks,
            self.timing_gated,
            self.timing_failures,
            self.failures
        )
    }
}

/// Numeric snapshots are not atomic cuts. Nested wall times are inclusive.
pub fn diagnostic_report() -> serde_json::Value {
    let snapshot = bbt_rt::diagnostics::snapshot();
    let samples: Vec<_> = snapshot.samples.iter().map(|row| serde_json::json!({
        "period":row.period,"phase":row.phase,"calls":row.calls,"active":row.active,"work":row.work,"bytes":row.bytes,
        "wall_ns":row.wall_ns,"wall_max_ns":row.wall_max_ns,
        "cpu_ns":row.cpu_ns,"cpu_max_ns":row.cpu_max_ns,"cpu_samples":row.cpu_samples,"cpu_errors":row.cpu_errors,
    })).collect();
    serde_json::json!({
        "enabled":snapshot.enabled,"process_id":std::process::id(),"sampled_ns":snapshot.sampled_ns,
        "clock":"nanoseconds since R5i diagnostics enable, independent of capture.json clock; null = not observed",
        "periods":["setup","warm","measure","settle","idle","finished"],
        "scopes":"inclusive: callback > drain > projection; publish > build/prepare/drop/Latest; request = inclusive RPC wait, overlaps supervisor work",
        "read_ring_bytes":"request=output payload; work=request seats (one per call); terminal decode work=records",
        "read_ring_request_scope":"SupervisorConnection::request including RPC wait/transport decode; syscall-only timing unavailable at the logic boundary",
        "read_ring_syscall_ns":serde_json::Value::Null,
        "tickless_redraw_work":"models projected before a redraw returned with no Metal tick",
        "counters":samples,
        "probe":{"id":snapshot.probe_id,"seat_u128":snapshot.probe_seat.to_string(),"generation":snapshot.probe_generation,
            "ring_sequence":snapshot.probe_sequence,"stamps":snapshot.probe.into_iter().collect::<BTreeMap<_,_>>()},
    })
}
fn model_has_probe(model: &Model) -> bool {
    if !bbt_rt::diagnostics::probe_pending(bbt_rt::diagnostics::ProbeStage::ModelPublished) {
        return false;
    }
    let Some((id, seat, _)) = bbt_rt::diagnostics::probe_info() else {
        return false;
    };
    model
        .screens
        .get(&SeatId::from_u128(seat))
        .is_some_and(|screen| probe_acknowledged(screen, &format!("R5P{id:08X}")))
}
pub struct Native {
    pub config: Config,
    pub progress: Arc<Progress>,
    commands: mpsc::Sender<Control>,
    stop: Option<oneshot::Sender<()>>,
    result: oneshot::Receiver<Result<Report, String>>,
    phase: u8,
    cpu: Vec<f64>,
    wall: Vec<f64>,
    previous: BTreeMap<SeatId, Arc<bbt_proto::Screen>>,
    changes: BTreeMap<String, u64>,
    probe_started: Option<(u64, Instant)>,
    probe_frame: Option<u64>,
    model_ms: Option<f64>,
    encoded_ms: Option<f64>,
    input_ms: Option<f64>,
    settled: Option<(u64, u64)>,
    idle: Option<(Instant, u64, u64)>,
    guards: u64,
    hz: f64,
    finish_sent: bool,
    last_cpu: Option<Duration>,
    paused_frame: Option<u64>,
    completed_frame: u64,
    probe_clipped: bool,
    started: Instant,
    settle_logged_phase: Option<u8>,
}
impl Native {
    pub fn start(
        config: Config,
        updates: Arc<Latest<Arc<Model>>>,
        rendering: oneshot::Receiver<crate::live::RenderContext>,
        messages: UiSender,
        wake: impl Fn() + Send + Sync + 'static,
    ) -> Self {
        bbt_rt::diagnostics::enable();
        let progress = Arc::new(Progress::default());
        let (commands, input) = mpsc::channel(8);
        let (stop, stopped) = oneshot::channel();
        let (complete, result) = oneshot::channel();
        let task_progress = progress.clone();
        let task_config = config.clone();
        bbt_rt::handle().spawn(async move {
            let result = load::run(
                task_config,
                updates,
                rendering,
                messages,
                task_progress.clone(),
                input,
                stopped,
            )
            .await;
            let _ = complete.send(result);
            task_progress.set_phase(FINISHED);
        });
        let ticks = progress.clone();
        let wake = Arc::new(wake);
        bbt_rt::handle().spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(20)).await;
                if !ticks.poll_pending.swap(true, Ordering::AcqRel) {
                    wake();
                }
                if ticks.phase() == FINISHED {
                    break;
                }
            }
        });
        Self {
            config,
            progress,
            commands,
            stop: Some(stop),
            result,
            phase: SETUP,
            cpu: vec![],
            wall: vec![],
            previous: BTreeMap::new(),
            changes: BTreeMap::new(),
            probe_started: None,
            probe_frame: None,
            model_ms: None,
            encoded_ms: None,
            input_ms: None,
            settled: None,
            idle: None,
            guards: bbt_rt::ui_blocking_violations(),
            hz: 120.,
            finish_sent: false,
            last_cpu: None,
            paused_frame: None,
            completed_frame: 0,
            probe_clipped: false,
            started: Instant::now(),
            settle_logged_phase: None,
        }
    }
    pub fn cancel(&mut self) {
        self.stop.take();
    }
    pub fn drain(&self) {
        if let Some(states) = self.progress.synthetic.get() {
            for id in states.dirty.drain() {
                if states.values[id].take().is_some() {
                    states.consumed.fetch_add(1, Ordering::Relaxed);
                }
            }
        }
    }
    pub fn model_received(&mut self, model: &Model) {
        self.model_received_at(model, bbt_rt::diagnostics::now());
    }
    pub fn model_received_at(&mut self, model: &Model, received_at: Option<u64>) {
        if self.model_ms.is_none()
            && let Some((id, start)) = self.probe_started
            && model
                .screens
                .values()
                .any(|screen| probe_acknowledged(screen, &format!("R5P{id:08X}")))
        {
            self.model_ms = Some(start.elapsed().as_secs_f64() * 1000.);
            bbt_rt::diagnostics::probe_mark_at(
                bbt_rt::diagnostics::ProbeStage::LatestTaken,
                received_at,
            );
        }
    }
    pub fn model_applied(&self) {
        bbt_rt::diagnostics::probe_after(
            bbt_rt::diagnostics::ProbeStage::LatestTaken,
            bbt_rt::diagnostics::ProbeStage::ModelApplied,
        );
    }
    pub fn poll(
        &mut self,
        frames: u64,
        callbacks: u64,
        pending: bool,
        hz: f64,
    ) -> Result<Option<Report>, String> {
        // Headless has no native dirty-source producer or UI output queues.
        self.poll_with_wakes(
            frames,
            callbacks,
            WakeDiagnostics::default().with_pending(pending, false, false),
            hz,
        )
    }
    /// Native poll with bounded source attribution and separate pending flags.
    pub fn poll_with_wakes(
        &mut self,
        frames: u64,
        callbacks: u64,
        wakes: WakeDiagnostics,
        hz: f64,
    ) -> Result<Option<Report>, String> {
        self.progress.poll_pending.store(false, Ordering::Release);
        self.hz = hz;
        let pending = wakes.pending();
        let phase = self.progress.phase();
        if phase != self.phase {
            self.phase = phase;
            if phase == MEASURE {
                self.cpu.clear();
                self.wall.clear();
                self.changes.clear();
                self.previous.clear();
            }
        }
        self.progress.native_settle.record(NativeSettleWitness {
            observed: true,
            phase,
            pending,
            has_paused_frame: self.paused_frame.is_some(),
            gpu_complete: self
                .paused_frame
                .is_some_and(|frame| self.completed_frame >= frame),
            stable_counts: self.settled == Some((frames, callbacks)),
            frames,
            callbacks,
            paused_frame: self.paused_frame,
            completed_frame: self.completed_frame,
            wakes,
            observed_elapsed_ns: self.started.elapsed().as_nanos().min(u64::MAX as u128) as u64,
        });
        match self.result.try_recv() {
            Ok(result) => return result.map(Some),
            Err(oneshot::error::TryRecvError::Closed) => {
                return Err("r5-worker-stopped-without-report".into());
            }
            Err(oneshot::error::TryRecvError::Empty) => {}
        }
        if phase == MEASURE && self.probe_started.is_none() && self.cpu.len() >= 8 {
            let id = 1;
            let admitted = Instant::now();
            bbt_rt::diagnostics::arm_probe(id);
            let admitted_at = bbt_rt::diagnostics::now();
            if self.commands.try_send(Control::Probe(id)).is_ok() {
                bbt_rt::diagnostics::probe_mark_at(
                    bbt_rt::diagnostics::ProbeStage::Admitted,
                    admitted_at,
                );
                self.probe_started = Some((id, admitted));
            }
        }
        if phase == SETTLE
            && !pending
            && self
                .paused_frame
                .is_some_and(|frame| self.completed_frame >= frame)
        {
            let counts = (frames, callbacks);
            if self.settled == Some(counts) {
                self.idle = Some((Instant::now(), frames, callbacks));
                self.progress.set_phase(IDLE);
                self.phase = IDLE;
            } else {
                self.settled = Some(counts);
            }
        }
        if phase == IDLE
            && !self.finish_sent
            && let Some((start, old_frames, old_callbacks)) = self.idle
            && start.elapsed().as_secs_f64() >= self.config.idle
        {
            let metrics = Measurements {
                cpu_p50_ms: percentile(&self.cpu, 0.5),
                cpu_p99_ms: percentile(&self.cpu, 0.99),
                frame_work_p50_ms: percentile(&self.wall, 0.5),
                frame_work_p99_ms: percentile(&self.wall, 0.99),
                frames: self.cpu.len(),
                hz,
                input_to_gpu_complete_ms: self.input_ms,
                input_to_prepared_model_ms: self.model_ms,
                input_to_encoded_frame_ms: self.encoded_ms,
                input_budget_ms: 2000. / hz,
                encoded_changes: self.changes.clone(),
                idle_frames: frames - old_frames,
                idle_callbacks: callbacks - old_callbacks,
                guard_violations: bbt_rt::ui_blocking_violations().saturating_sub(self.guards),
                cpu_samples_ms: self.cpu.clone(),
                frame_work_samples_ms: self.wall.clone(),
                probe_clipped: self.probe_clipped,
                diagnostics: diagnostic_report(),
            };
            // Pressure leaves finalization pending. Only the result channel
            // can report an error, after the IO owner has completed teardown.
            if self.commands.try_send(Control::Finish(metrics)).is_ok() {
                self.finish_sent = true;
            }
        }
        Ok(None)
    }
    /// Memory-only formatting; the app sends this to its existing IO logger.
    pub fn settle_line(&mut self, finishing: bool) -> Option<String> {
        let witness = self.progress.native_settle.snapshot();
        if !finishing && (witness.phase < SETTLE || self.settle_logged_phase == Some(witness.phase))
        {
            return None;
        }
        self.settle_logged_phase = Some(witness.phase);
        let mut value = serde_json::to_value(witness).expect("numeric settle witness serializes");
        value["diagnostics"] = diagnostic_report();
        Some(format!(
            "R5 settle sources=0:messages,1:output,2:prepared,3:demo,4:display_tick,5:live_model,1024:direct {}",
            serde_json::to_string(
                &serde_json::json!({"settle":witness,"diagnostics":diagnostic_report()})
            )
            .expect("numeric settle witness serializes")
        ))
    }
    pub fn frame(
        &mut self,
        frame: u64,
        model: &Model,
        bounds: &[(SeatId, bbt_ui_model::layout::Rect)],
        phases: Phases,
        cpu: Duration,
        scale: f32,
    ) {
        let cpu = self
            .last_cpu
            .replace(cpu)
            .map_or(Duration::ZERO, |last| cpu.saturating_sub(last));
        if bounds.len() == self.config.watched
            && bounds.iter().all(|(seat, _)| {
                model
                    .screens
                    .get(seat)
                    .is_some_and(|screen| contains(screen, "R5PAUSED"))
            })
        {
            self.paused_frame = Some(frame);
        } else {
            self.paused_frame = None;
        }
        if self.progress.phase() != MEASURE {
            return;
        }
        if self.cpu.len() < 8192 {
            self.cpu.push(cpu.as_secs_f64() * 1000.);
            self.wall
                .push((phases.cpu() + phases.present_wait).as_secs_f64() * 1000.);
        }
        for (seat, rect) in bounds {
            if rect.w <= 0. || rect.h <= 0. {
                continue;
            }
            let Some(screen) = model.screens.get(seat) else {
                continue;
            };
            if self
                .previous
                .get(seat)
                .is_none_or(|old| !Arc::ptr_eq(old, screen))
            {
                *self.changes.entry(seat.to_string()).or_default() += 1;
                self.previous.insert(*seat, screen.clone());
            }
            if self.probe_frame.is_none()
                && let Some((id, _)) = self.probe_started
                && probe_acknowledged(screen, &format!("R5P{id:08X}"))
            {
                let prepared = &model.frames[seat];
                let cell_width = prepared.width as f32 / screen.cols as f32;
                let line_height = prepared.height as f32 / screen.rows as f32;
                if probe_visible(
                    screen,
                    &format!("R5P{id:08X}"),
                    [rect.w * scale, rect.h * scale],
                    [cell_width, line_height],
                ) {
                    self.probe_frame = Some(frame);
                    bbt_rt::diagnostics::probe_mark(bbt_rt::diagnostics::ProbeStage::Encoded);
                    self.probe_clipped = false;
                    self.encoded_ms = self
                        .probe_started
                        .map(|(_, start)| start.elapsed().as_secs_f64() * 1000.);
                } else {
                    self.probe_clipped = true;
                }
            }
        }
    }
    pub fn gpu_complete(&mut self, frame: u64, at: Instant) {
        self.completed_frame = self.completed_frame.max(frame);
        if self.input_ms.is_none()
            && self.probe_frame.is_some_and(|target| frame >= target)
            && let Some((_, start)) = self.probe_started
        {
            self.input_ms = Some(at.saturating_duration_since(start).as_secs_f64() * 1000.);
            bbt_rt::diagnostics::probe_mark_at(
                bbt_rt::diagnostics::ProbeStage::GpuComplete,
                bbt_rt::diagnostics::at(at),
            );
        }
    }
}
impl Drop for Native {
    fn drop(&mut self) {
        self.cancel();
    }
}
fn contains(screen: &bbt_proto::Screen, needle: &str) -> bool {
    screen
        .cells
        .iter()
        .any(|row| row.iter().map(|c| c.c).collect::<String>().contains(needle))
}

/// Count the helper's pinned acknowledgement, never a PTY echo in a lower row.
fn probe_acknowledged(screen: &bbt_proto::Screen, needle: &str) -> bool {
    let columns = needle.chars().count();
    screen.cells.first().is_some_and(|row| {
        row.iter().take(columns).map(|c| c.c).eq(needle.chars())
            && row.get(columns).is_some_and(|c| matches!(c.c, '|' | '/'))
    })
}
fn probe_visible(screen: &bbt_proto::Screen, needle: &str, clip: [f32; 2], cell: [f32; 2]) -> bool {
    probe_acknowledged(screen, needle)
        && ((needle.chars().count() + 1) as f32 * cell[0]).ceil() <= clip[0].floor()
        && cell[1].ceil() <= clip[1].floor()
}
fn percentile(values: &[f64], p: f64) -> f64 {
    if values.is_empty() {
        return 0.;
    }
    let mut values = values.to_vec();
    values.sort_by(f64::total_cmp);
    values[((values.len() as f64 * p).ceil() as usize)
        .saturating_sub(1)
        .min(values.len() - 1)]
}

pub fn private_entry(args: &[String]) -> Option<Result<(), String>> {
    load::private_entry(args)
}
pub fn headless(config: Config) -> Result<(), String> {
    headless::run(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wake_sources_and_pending_queues_survive_settle_serialization() {
        let mut counters = WakeCounters::default();
        counters.wake(&[4]);
        counters.ignored_tick();
        counters.wake(&[4, 5]);
        counters.redraw(&[4, 5]);
        counters.frame();
        // A publication consumed by a frame can leave a later, empty Wake.
        counters.drained(&[0]);
        counters.frame();
        counters.wake(&[]);
        let wakes = counters.snapshot().with_pending(false, true, false);
        let witness = NativeSettleWitness {
            observed: true,
            phase: SETTLE,
            pending: wakes.pending(),
            has_paused_frame: true,
            frames: 12,
            callbacks: 13,
            paused_frame: Some(12),
            completed_frame: 11,
            observed_elapsed_ns: 1_000_000,
            wakes,
            ..Default::default()
        };
        let probe = NativeSettleProbe::default();
        probe.record(witness);
        let json = serde_json::to_value(probe.snapshot()).unwrap();
        assert_eq!(json["frames"], 12);
        assert_eq!(json["callbacks"], 13);
        assert_eq!(json["completed_frame"], 11);
        assert_eq!(json["observed_elapsed_ns"], 1_000_000);
        assert_eq!(json["wakes"]["wake_events"], 3);
        assert_eq!(json["wakes"]["empty_wakes"], 1);
        assert_eq!(json["wakes"]["ignored_ticks"], 1);
        assert_eq!(json["wakes"]["pending_redraw"], false);
        assert_eq!(json["wakes"]["pending_messages"], true);
        assert_eq!(json["wakes"]["pending_output"], false);
        let sources = json["wakes"]["sources"].as_array().unwrap();
        assert_eq!(sources.len(), 3);
        let tick = sources.iter().find(|row| row["source"] == 4).unwrap();
        assert_eq!(tick["wakes"], 2);
        assert_eq!(tick["redraws"], 1);
        assert_eq!(tick["frames"], 1);
        let messages = sources.iter().find(|row| row["source"] == 0).unwrap();
        assert_eq!(messages["frame_drains"], 1);
        assert_eq!(messages["wakes"], 0);
        assert_eq!(messages["frames"], 1);
    }

    #[test]
    fn extra_wake_sources_are_bounded_and_overflow_is_named() {
        let mut counters = WakeCounters::default();
        counters.wake(&(0..WAKE_SLOTS + 3).collect::<Vec<_>>());
        let json = serde_json::to_value(counters.snapshot()).unwrap();
        assert_eq!(json["sources"].as_array().unwrap().len(), WAKE_SLOTS);
        assert_eq!(json["overflow"], 3);
    }

    #[test]
    fn lower_row_echo_does_not_acknowledge_a_probe_or_bypass_the_clip() {
        let mut term = bbt_core::Term::new(24, 8, 0);
        term.advance(b"\x1b[8;1HR5P00000001");
        let screen = bbt_proto::Screen::from_term(&term, String::new(), None, None);
        assert!(contains(&screen, "R5P00000001"));
        assert!(!probe_visible(
            &screen,
            "R5P00000001",
            [192., 105.],
            [8., 15.]
        ));
        term.advance(b"\x1b[1;1HR5P00000001|");
        let screen = bbt_proto::Screen::from_term(&term, String::new(), None, None);
        assert!(probe_visible(
            &screen,
            "R5P00000001",
            [192., 105.],
            [8., 15.]
        ));
        assert!(!probe_visible(
            &screen,
            "R5P00000001",
            [95., 105.],
            [8., 15.]
        ));
        assert!(!probe_visible(
            &screen,
            "R5P00000001",
            [192., 14.],
            [8., 15.]
        ));
    }
}
