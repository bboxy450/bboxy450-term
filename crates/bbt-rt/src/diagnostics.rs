// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Opt-in R5 measurements. Disabled hooks do not read a clock or allocate.
//! Counters are inclusive, process-local observations, never scheduling inputs.
mod callbacks;
pub use callbacks::{CALLBACK_KINDS, CallbackCpuSlice, CallbackFrameCpu, CallbackKindCpu};
use std::sync::{
    OnceLock,
    atomic::{AtomicU8, AtomicU64, Ordering::Relaxed},
};
use std::time::{Duration, Instant};

macro_rules! phases {
    ($($(#[$doc:meta])* $name:ident),+ $(,)?) => {
        #[derive(Clone, Copy, Debug)]
        #[repr(usize)]
        /// Fixed measurement phase; its discriminant indexes the bounded counters.
        pub enum Phase { $($(#[$doc])* $name),+ }
        const PHASES: &[Phase] = &[$(Phase::$name),+];
        impl Phase {
            /// Return the stable case-sensitive label exported in diagnostic snapshots.
            pub fn name(self) -> &'static str { match self { $(Self::$name => stringify!($name)),+ } }
        }
    };
}
phases! {
    /// Frontend about-to-wait callback.
    AboutToWait,
    /// Frontend resume callback.
    Resumed,
    /// User-event callback carrying a wake notice.
    UserWake,
    /// User-event callback polling a diagnostic run.
    UserStallPoll,
    /// Other application user-event callbacks.
    UserOther,
    /// Window redraw callback, including early returns.
    WindowRedraw,
    /// Window pointer-input callback.
    WindowPointer,
    /// Other window-event callbacks.
    WindowOther,
    /// Draining incoming frontend publications.
    UiDrain,
    /// Taking a replaceable latest-value publication.
    UiLatestTake,
    /// Projecting the immutable model into frontend state.
    UiProjection,
    /// Projecting the frontend row list.
    UiSeatRows,
    /// Dropping a superseded model on the frontend owner.
    UiOldModelDrop,
    /// Redraw callback that returns without consuming a display tick.
    TicklessRedraw,
    /// Building an immutable model snapshot.
    ModelBuild,
    /// Preparing pane data for a frontend snapshot.
    PanePrepare,
    /// Receiving a host-to-frontend update.
    GuiReceive,
    /// Applying a received update to the frontend model.
    GuiApply,
    /// Publishing a prepared model snapshot.
    ModelPublish,
    /// Replacing the unread frontend model slot.
    LatestPublish,
    /// Dropping a superseded model on an IO owner.
    IoOldModelDrop,
    /// Requesting retained output from a host ring.
    ReadRingRequest,
    /// Decoding retained output into a terminal model.
    ReadRingTerminalDecode,
    /// Publishing a logical terminal view.
    LogicViewPublish,
    /// Fetching or preparing a retained-history page.
    HistoryPage,
    /// Advancing bounded history replay.
    HistoryStep,
}
const PERIODS: usize = 6;

#[derive(Default)]
struct Counter {
    active: AtomicU64,
    calls: AtomicU64,
    work: AtomicU64,
    bytes: AtomicU64,
    wall_ns: AtomicU64,
    wall_max_ns: AtomicU64,
    cpu_ns: AtomicU64,
    cpu_max_ns: AtomicU64,
    cpu_samples: AtomicU64,
    cpu_errors: AtomicU64,
}

macro_rules! stamps {
    ($($(#[$doc:meta])* $name:ident),+ $(,)?) => {
        #[derive(Clone, Copy, Debug)]
        #[repr(usize)]
        /// A first-observation point in the opt-in probe timeline.
        pub enum ProbeStage { $($(#[$doc])* $name),+ }
        const STAMPS: &[ProbeStage] = &[$(ProbeStage::$name),+];
        impl ProbeStage {
            /// Return the stable case-sensitive label exported in diagnostic snapshots.
            pub fn name(self) -> &'static str { match self { $(Self::$name => stringify!($name)),+ } }
        }
    };
}
stamps! {
    /// Input probe admitted by its caller.
    Admitted,
    /// Probe request received at the control surface.
    ControlReceived,
    /// Probe input enqueued for the frontend owner.
    GuiEnqueued,
    /// Probe input enqueued for the host transport.
    WireEnqueued,
    /// Probe input dequeued by its writer.
    WriterDequeued,
    /// Probe write began.
    WriteStarted,
    /// Probe write completed.
    Written,
    /// The matching probe marker first observed in logical output.
    LogicOutputRead,
    /// Probe marker observed in retained-ring output.
    LogicRingRead,
    /// Probe marker observed in a live output event.
    LogicLiveEvent,
    /// Frontend received the probe-bearing publication.
    GuiReceived,
    /// Frontend began applying the probe-bearing update.
    GuiApplyStarted,
    /// Frontend completed applying the update.
    GuiApplied,
    /// Probe-bearing immutable model built.
    ModelBuilt,
    /// Probe-bearing model prepared for the frontend.
    ModelPrepared,
    /// Probe-bearing model published to the frontend.
    ModelPublished,
    /// Probe-bearing publication taken from its latest-value slot.
    LatestTaken,
    /// Frontend projection of the probe-bearing model began.
    ProjectionStarted,
    /// Frontend projection completed.
    ProjectionFinished,
    /// Probe-bearing model installed in frontend state.
    ModelApplied,
    /// Probe-bearing frame encoded.
    Encoded,
    /// GPU completion for the probe-bearing frame observed.
    GpuComplete,
}
#[derive(Default)]
struct Probe {
    id: AtomicU64,
    seat_hi: AtomicU64,
    seat_lo: AtomicU64,
    generation: AtomicU64,
    matched: AtomicU8,
    previous_sequence: AtomicU64,
    sequence: AtomicU64,
    stamps: [AtomicU64; STAMPS.len()],
}
struct State {
    origin: Instant,
    period: AtomicU8,
    counters: [[Counter; PHASES.len()]; PERIODS],
    probe: Probe,
}
/// Lazy diagnostics state; global convenience functions use the process-wide recorder.
#[derive(Default)]
pub struct Recorder(OnceLock<State>);
static RECORDER: Recorder = Recorder(OnceLock::new());

/// Inclusive counters for one phase and period; concurrent updates are not atomically snapshotted.
#[derive(Debug)]
pub struct Sample {
    /// Zero-based measurement period, clamped to the six retained periods.
    pub period: u8,
    /// Stable phase label for this row.
    pub phase: &'static str,
    /// Number of completed spans.
    pub calls: u64,
    /// Number of spans still in flight at the sampled instant.
    pub active: u64,
    /// Sum of caller-defined work units supplied by completed spans.
    pub work: u64,
    /// Sum of caller-defined byte counts supplied by completed spans.
    pub bytes: u64,
    /// Sum of completed span wall durations in nanoseconds.
    pub wall_ns: u64,
    /// Largest completed span wall duration in nanoseconds.
    pub wall_max_ns: u64,
    /// Sum of available completed thread CPU measurements in nanoseconds.
    pub cpu_ns: u64,
    /// Largest completed thread CPU measurement in nanoseconds.
    pub cpu_max_ns: u64,
    /// Number of completed spans with successful CPU measurements.
    pub cpu_samples: u64,
    /// Failed CPU measurements, including unavailable callback-entry readings.
    pub cpu_errors: u64,
}
/// On-demand diagnostic counters and probe timestamps, without scheduling authority.
#[derive(Debug)]
pub struct Snapshot {
    /// Whether this recorder has been initialized explicitly.
    pub enabled: bool,
    /// Positive elapsed nanoseconds from recorder enablement, or zero when disabled.
    pub sampled_ns: u64,
    /// Rows in period-then-phase order; empty when diagnostics are disabled.
    pub samples: Vec<Sample>,
    /// Caller-selected probe identifier; zero means no probe is armed.
    pub probe_id: u64,
    /// Caller-supplied opaque source identifier bound to the probe.
    pub probe_seat: u128,
    /// Caller-supplied source generation bound to the probe.
    pub probe_generation: u64,
    /// Capture sequence where the probe marker completed; zero before a match.
    pub probe_sequence: u64,
    /// Stage labels paired with positive recorder-relative nanoseconds, or `None` if unobserved.
    pub probe: Vec<(&'static str, Option<u64>)>,
}
impl Recorder {
    /// Initialize counters and a monotonic origin once; later calls preserve prior observations.
    pub fn enable(&self) {
        self.0.get_or_init(|| State {
            origin: Instant::now(),
            period: AtomicU8::new(0),
            counters: std::array::from_fn(|_| std::array::from_fn(|_| Counter::default())),
            probe: Probe::default(),
        });
    }
    fn span_with(&self, phase: Phase, cpu: impl FnOnce() -> Option<Duration>) -> Span<'_> {
        let Some(state) = self.0.get() else {
            return Span(None);
        };
        let counter = &state.counters[usize::from(state.period.load(Relaxed)).min(PERIODS - 1)]
            [phase as usize];
        counter.active.fetch_add(1, Relaxed);
        let cpu = cpu();
        if cpu.is_none() && (phase as usize) <= Phase::WindowOther as usize {
            counter.cpu_errors.fetch_add(1, Relaxed);
        }
        Span(Some(Measurement {
            counter,
            start: Instant::now(),
            cpu,
            callback: None,
            work: 0,
            bytes: 0,
        }))
    }
    /// Read the current bounded counters; disabled state returns an empty snapshot without a clock read.
    pub fn snapshot(&self) -> Snapshot {
        let mut snapshot = Snapshot {
            enabled: false,
            sampled_ns: 0,
            samples: vec![],
            probe_id: 0,
            probe_seat: 0,
            probe_generation: 0,
            probe_sequence: 0,
            probe: vec![],
        };
        let Some(state) = self.0.get() else {
            return snapshot;
        };
        snapshot.enabled = true;
        snapshot.sampled_ns = elapsed_ns(state);
        for (period, counters) in state.counters.iter().enumerate() {
            for (&phase, counter) in PHASES.iter().zip(counters) {
                snapshot.samples.push(Sample {
                    period: period as u8,
                    phase: phase.name(),
                    calls: counter.calls.load(Relaxed),
                    active: counter.active.load(Relaxed),
                    work: counter.work.load(Relaxed),
                    bytes: counter.bytes.load(Relaxed),
                    wall_ns: counter.wall_ns.load(Relaxed),
                    wall_max_ns: counter.wall_max_ns.load(Relaxed),
                    cpu_ns: counter.cpu_ns.load(Relaxed),
                    cpu_max_ns: counter.cpu_max_ns.load(Relaxed),
                    cpu_samples: counter.cpu_samples.load(Relaxed),
                    cpu_errors: counter.cpu_errors.load(Relaxed),
                });
            }
        }
        let probe = &state.probe;
        snapshot.probe_id = probe.id.load(Relaxed);
        snapshot.probe_seat =
            u128::from(probe.seat_hi.load(Relaxed)) << 64 | u128::from(probe.seat_lo.load(Relaxed));
        snapshot.probe_generation = probe.generation.load(Relaxed);
        snapshot.probe_sequence = probe.sequence.load(Relaxed);
        snapshot.probe = STAMPS
            .iter()
            .map(|&stage| {
                let value = probe.stamps[stage as usize].load(Relaxed);
                (stage.name(), (value != 0).then_some(value))
            })
            .collect();
        snapshot
    }
}
struct Measurement<'a> {
    counter: &'a Counter,
    start: Instant,
    cpu: Option<Duration>,
    callback: Option<(u64, u64)>,
    work: u64,
    bytes: u64,
}
/// RAII wall/CPU measurement that records once on drop; disabled guards do no sampling.
pub struct Span<'a>(Option<Measurement<'a>>);
impl Span<'_> {
    /// Return whether this guard captured an enabled recorder at its creation.
    pub fn enabled(&self) -> bool {
        self.0.is_some()
    }
    /// Set the final work and byte counts, replacing any previous values on this guard.
    pub fn work(mut self, work: u64, bytes: u64) -> Self {
        if let Some(m) = &mut self.0 {
            m.work = work;
            m.bytes = bytes;
        }
        self
    }
    /// Set final counts and consume the guard, recording completion through its drop.
    pub fn finish(mut self, work: u64, bytes: u64) {
        if let Some(m) = &mut self.0 {
            m.work = work;
            m.bytes = bytes;
        }
    }
}
impl Drop for Span<'_> {
    fn drop(&mut self) {
        let Some(m) = self.0.take() else { return };
        let elapsed = ns(m.start.elapsed());
        m.counter.calls.fetch_add(1, Relaxed);
        m.counter.work.fetch_add(m.work, Relaxed);
        m.counter.bytes.fetch_add(m.bytes, Relaxed);
        m.counter.wall_ns.fetch_add(elapsed, Relaxed);
        m.counter.wall_max_ns.fetch_max(elapsed, Relaxed);
        m.counter.active.fetch_sub(1, Relaxed);
        let mut callback_end = None;
        if let Some(start) = m.cpu {
            match crate::fault::thread_cpu_time() {
                Ok(end) => {
                    callback_end = Some(ns(end));
                    let elapsed = ns(end.saturating_sub(start));
                    m.counter.cpu_samples.fetch_add(1, Relaxed);
                    m.counter.cpu_ns.fetch_add(elapsed, Relaxed);
                    m.counter.cpu_max_ns.fetch_max(elapsed, Relaxed);
                }
                Err(_) => {
                    m.counter.cpu_errors.fetch_add(1, Relaxed);
                }
            }
        }
        if let Some((id, started_ns)) = m.callback {
            callbacks::leave(id, started_ns.saturating_add(elapsed), callback_end);
        }
    }
}
fn ns(value: Duration) -> u64 {
    value.as_nanos().min(u128::from(u64::MAX)) as u64
}
fn elapsed_ns(state: &State) -> u64 {
    ns(state.origin.elapsed()).saturating_add(1)
}

/// Enable the process recorder once; there is no disable or reset operation.
pub fn enable() {
    RECORDER.enable();
}
/// Begin an inclusive wall-time span; disabled diagnostics do not read a clock.
pub fn span(phase: Phase) -> Span<'static> {
    RECORDER.span_with(phase, || None)
}
/// Begin callback wall and thread-CPU attribution; keep this guard on the creating thread.
pub fn callback(phase: Phase) -> Span<'static> {
    let mut span = RECORDER.span_with(phase, || crate::fault::thread_cpu_time().ok());
    if let Some(measurement) = &mut span.0 {
        if let Some(cpu) = measurement.cpu
            && let Some(started_ns) = at(measurement.start)
        {
            measurement.callback =
                callbacks::enter(phase, started_ns, ns(cpu)).map(|id| (id, started_ns));
        } else {
            callbacks::failed();
        }
    }
    span
}
/// Cut callback attribution at the supplied UI-thread CPU endpoint. Disabled
/// diagnostics read no clock and do not initialize the thread-local tracker.
/// Wall timestamps use the same origin as `now`; they are not capture clocks.
pub fn callback_frame_cpu(cpu: Duration) -> Option<CallbackFrameCpu> {
    RECORDER
        .0
        .get()
        .and_then(|state| callbacks::cut(elapsed_ns(state), ns(cpu)))
}
/// Read positive nanoseconds since enablement, or `None` without reading a clock when disabled.
pub fn now() -> Option<u64> {
    RECORDER.0.get().map(elapsed_ns)
}
/// Express an instant in the recorder clock domain, saturating instants before its origin.
pub fn at(instant: Instant) -> Option<u64> {
    RECORDER
        .0
        .get()
        .map(|state| ns(instant.saturating_duration_since(state.origin)).saturating_add(1))
}
/// Observe completed calls in the currently selected period, or `None` while disabled.
pub fn calls(phase: Phase) -> Option<u64> {
    RECORDER.0.get().map(|state| {
        state.counters[usize::from(state.period.load(Relaxed))][phase as usize]
            .calls
            .load(Relaxed)
    })
}
/// Select the period for newly started spans, clamping to 0..=5; disabled calls do nothing.
pub fn set_period(period: u8) {
    if let Some(state) = RECORDER.0.get() {
        state.period.store(period.min(PERIODS as u8 - 1), Relaxed);
    }
}
/// Read the process recorder on demand; concurrent row values may reflect different instants.
pub fn snapshot() -> Snapshot {
    RECORDER.snapshot()
}
/// Bind the enabled probe to a caller-owned source identifier and generation without clearing stamps.
pub fn probe_target(seat: u128, generation: u64) {
    if let Some(state) = RECORDER.0.get() {
        state.probe.seat_hi.store((seat >> 64) as u64, Relaxed);
        state.probe.seat_lo.store(seat as u64, Relaxed);
        state.probe.generation.store(generation, Relaxed);
    }
}
/// Set the probe identifier; zero disarms it, and existing timestamps are retained.
pub fn arm_probe(id: u64) {
    if let Some(state) = RECORDER.0.get() {
        state.probe.id.store(id, Relaxed);
    }
}
/// Record the first observation of an armed stage using the recorder clock.
pub fn probe_mark(stage: ProbeStage) {
    if probe_pending(stage) {
        probe_mark_at(stage, now());
    }
}
/// Return the nonzero probe ID, opaque source ID and generation when enabled and armed.
pub fn probe_info() -> Option<(u64, u128, u64)> {
    let state = RECORDER.0.get()?;
    let id = state.probe.id.load(Relaxed);
    (id != 0).then(|| {
        (
            id,
            u128::from(state.probe.seat_hi.load(Relaxed)) << 64
                | u128::from(state.probe.seat_lo.load(Relaxed)),
            state.probe.generation.load(Relaxed),
        )
    })
}
/// Return whether an enabled, armed probe has not recorded this stage.
pub fn probe_pending(stage: ProbeStage) -> bool {
    RECORDER.0.get().is_some_and(|state| {
        state.probe.id.load(Relaxed) != 0 && state.probe.stamps[stage as usize].load(Relaxed) == 0
    })
}
/// Record a stage only after its required predecessor has a stored observation.
pub fn probe_after(required: ProbeStage, stage: ProbeStage) {
    if let Some(state) = RECORDER.0.get()
        && state.probe.stamps[required as usize].load(Relaxed) != 0
    {
        probe_mark(stage);
    }
}
/// Store the first supplied stage timestamp for an armed probe; use positive `now`/`at` values.
pub fn probe_mark_at(stage: ProbeStage, at: Option<u64>) {
    if let Some(state) = RECORDER.0.get()
        && state.probe.id.load(Relaxed) != 0
        && let Some(at) = at
    {
        let _ = state.probe.stamps[stage as usize].compare_exchange(0, at, Relaxed, Relaxed);
    }
}
/// Only the one armed seat is scanned; matching survives ring-record boundaries.
pub fn probe_ring_output(seat: u128, generation: u64, sequence: u64, bytes: &[u8]) {
    let Some(state) = RECORDER.0.get() else {
        return;
    };
    record_probe_output(
        state,
        seat,
        generation,
        sequence,
        bytes,
        ProbeStage::LogicRingRead,
    );
}
/// Scan live output for the armed marker, matching its bound source, generation and contiguous sequences.
pub fn probe_live_output(seat: u128, generation: u64, sequence: u64, bytes: &[u8]) {
    let Some(state) = RECORDER.0.get() else {
        return;
    };
    record_probe_output(
        state,
        seat,
        generation,
        sequence,
        bytes,
        ProbeStage::LogicLiveEvent,
    );
}
fn record_probe_output(
    state: &State,
    seat: u128,
    generation: u64,
    sequence: u64,
    bytes: &[u8],
    source: ProbeStage,
) {
    let probe = &state.probe;
    let id = probe.id.load(Relaxed);
    if id == 0
        || probe.stamps[ProbeStage::LogicOutputRead as usize].load(Relaxed) != 0
        || probe.seat_hi.load(Relaxed) != (seat >> 64) as u64
        || probe.seat_lo.load(Relaxed) != seat as u64
        || probe.generation.load(Relaxed) != generation
    {
        return;
    }
    let mut marker = *b"R5P00000000";
    for (index, digit) in marker[3..].iter_mut().enumerate() {
        *digit = b"0123456789ABCDEF"[((id >> ((7 - index) * 4)) & 15) as usize];
    }
    let previous = probe.previous_sequence.swap(sequence, Relaxed);
    let mut matched = if sequence == previous.saturating_add(1) {
        usize::from(probe.matched.load(Relaxed))
    } else {
        0
    };
    for &byte in bytes {
        if matched == marker.len() && matches!(byte, b'|' | b'/') {
            probe.sequence.store(sequence, Relaxed);
            let at = elapsed_ns(state);
            let _ = probe.stamps[ProbeStage::LogicOutputRead as usize]
                .compare_exchange(0, at, Relaxed, Relaxed);
            probe.stamps[source as usize].store(at, Relaxed);
            matched = 0;
            break;
        }
        matched = if matched < marker.len() && byte == marker[matched] {
            matched + 1
        } else {
            usize::from(byte == marker[0])
        };
    }
    probe.matched.store(matched as u8, Relaxed);
}

#[cfg(test)]
mod tests {
    use super::*;
    // keeps: spec runtime §5 and §6 — bounded opt-in measurements and callback/probe attribution
    #[test]
    fn disabled_hooks_never_sample_clocks_or_allocate_state() {
        let recorder = Recorder::default();
        recorder
            .span_with(Phase::WindowRedraw, || panic!("disabled CPU clock sampled"))
            .finish(1, 2);
        assert!(recorder.0.get().is_none());
        let snapshot = recorder.snapshot();
        assert!(!snapshot.enabled);
        assert!(snapshot.samples.is_empty());
    }
    // keeps: spec runtime §5 and §6 — bounded opt-in measurements and callback/probe attribution
    #[test]
    fn enabled_counters_are_partitioned_by_start_period_and_bounded() {
        let recorder = Recorder::default();
        recorder.enable();
        let state = recorder.0.get().unwrap();
        state.period.store(2, Relaxed);
        let span = recorder.span_with(Phase::ReadRingRequest, || None);
        state.period.store(3, Relaxed);
        span.finish(1, 4096);
        let snapshot = recorder.snapshot();
        let row = snapshot
            .samples
            .iter()
            .find(|row| row.period == 2 && row.phase == "ReadRingRequest")
            .unwrap();
        assert_eq!((row.calls, row.work, row.bytes), (1, 1, 4096));
        assert_eq!(snapshot.samples.len(), PERIODS * PHASES.len());
        assert!(snapshot.probe.iter().all(|(_, stamp)| stamp.is_none()));
    }
    // keeps: spec runtime §5 and §6 — bounded opt-in measurements and callback/probe attribution
    #[test]
    fn probe_requires_the_pinned_ack_and_does_not_stitch_across_gaps() {
        let recorder = Recorder::default();
        recorder.enable();
        let state = recorder.0.get().unwrap();
        state.probe.id.store(1, Relaxed);
        state.probe.seat_lo.store(7, Relaxed);
        state.probe.generation.store(2, Relaxed);
        record_probe_output(
            state,
            7,
            2,
            1,
            b"echo R5P00000001\r\n",
            ProbeStage::LogicRingRead,
        );
        assert!(recorder.snapshot().probe.iter().all(|(_, at)| at.is_none()));
        record_probe_output(state, 7, 2, 2, b"R5P000", ProbeStage::LogicRingRead);
        record_probe_output(state, 7, 2, 4, b"00001|", ProbeStage::LogicRingRead);
        assert_eq!(recorder.snapshot().probe_sequence, 0);
        record_probe_output(state, 8, 2, 5, b"R5P00000001|", ProbeStage::LogicRingRead);
        assert_eq!(recorder.snapshot().probe_sequence, 0);
        record_probe_output(state, 7, 2, 5, b"R5P000", ProbeStage::LogicRingRead);
        record_probe_output(state, 7, 2, 6, b"00001/", ProbeStage::LogicRingRead);
        let first = recorder.snapshot();
        assert_eq!(first.probe_sequence, 6);
        assert!(
            first
                .probe
                .iter()
                .any(|(stage, at)| *stage == "LogicRingRead" && at.is_some())
        );
        record_probe_output(state, 7, 2, 7, b"R5P00000001|", ProbeStage::LogicRingRead);
        assert_eq!(recorder.snapshot().probe, first.probe);
    }
    // keeps: spec runtime §5 and §6 — bounded opt-in measurements and callback/probe attribution
    #[test]
    fn callback_guard_accounts_for_early_return_and_in_flight() {
        let recorder = Recorder::default();
        recorder.enable();
        let run = || {
            let _guard = recorder.span_with(Phase::WindowRedraw, || Some(Duration::ZERO));
            assert!(
                recorder
                    .snapshot()
                    .samples
                    .iter()
                    .any(|row| row.phase == "WindowRedraw" && row.active == 1)
            );
        };
        run();
        let snapshot = recorder.snapshot();
        let row = snapshot
            .samples
            .iter()
            .find(|row| row.period == 0 && row.phase == "WindowRedraw")
            .unwrap();
        assert_eq!((row.calls, row.active, row.cpu_samples), (1, 0, 1));
    }
}
