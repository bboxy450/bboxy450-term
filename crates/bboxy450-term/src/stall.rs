//! R5 verification only. Synthetic UI fixtures and real PTYs are reported
//! separately; neither a submitted frame nor a GPU completion is a photon.
mod headless;
mod load;

use bboxy450_term::model::Model;
use bbt_proto::SeatId;
use bbt_rt::{Latest, Phases, UiSender};
use serde::Serialize;
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
}
impl Native {
    pub fn start(
        config: Config,
        updates: Arc<Latest<Arc<Model>>>,
        rendering: oneshot::Receiver<crate::live::RenderContext>,
        messages: UiSender,
        wake: impl Fn() + Send + Sync + 'static,
    ) -> Self {
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
        if self.model_ms.is_none()
            && let Some((id, start)) = self.probe_started
            && model
                .screens
                .values()
                .any(|screen| contains(screen, &format!("R5P{id:08X}")))
        {
            self.model_ms = Some(start.elapsed().as_secs_f64() * 1000.);
        }
    }
    pub fn poll(
        &mut self,
        frames: u64,
        callbacks: u64,
        pending: bool,
        hz: f64,
    ) -> Result<Option<Report>, String> {
        self.progress.poll_pending.store(false, Ordering::Release);
        self.hz = hz;
        match self.result.try_recv() {
            Ok(result) => return result.map(Some),
            Err(oneshot::error::TryRecvError::Closed) => {
                return Err("r5-worker-stopped-without-report".into());
            }
            Err(oneshot::error::TryRecvError::Empty) => {}
        }
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
        if phase == MEASURE && self.probe_started.is_none() && self.cpu.len() >= 8 {
            let id = 1;
            let admitted = Instant::now();
            if self.commands.try_send(Control::Probe(id)).is_ok() {
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
            };
            // Pressure leaves finalization pending. Only the result channel
            // can report an error, after the IO owner has completed teardown.
            if self.commands.try_send(Control::Finish(metrics)).is_ok() {
                self.finish_sent = true;
            }
        }
        Ok(None)
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
                && contains(screen, &format!("R5P{id:08X}"))
            {
                let prepared = &model.frames[seat];
                let cell_width = prepared.width as f32 / screen.cols as f32;
                let line_height = prepared.height as f32 / screen.rows as f32;
                if rect.w * scale >= 12. * cell_width && rect.h * scale >= line_height {
                    self.probe_frame = Some(frame);
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
