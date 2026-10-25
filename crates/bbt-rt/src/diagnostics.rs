//! Opt-in R5 measurements. Disabled hooks do not read a clock or allocate.
//! Counters are inclusive, process-local observations, never scheduling inputs.
use std::sync::{
    OnceLock,
    atomic::{AtomicU8, AtomicU64, Ordering::Relaxed},
};
use std::time::{Duration, Instant};

macro_rules! phases {
    ($($name:ident),+ $(,)?) => {
        #[derive(Clone, Copy, Debug)]
        #[repr(usize)]
        pub enum Phase { $($name),+ }
        const PHASES: &[Phase] = &[$(Phase::$name),+];
        impl Phase {
            pub fn name(self) -> &'static str { match self { $(Self::$name => stringify!($name)),+ } }
        }
    };
}
phases! {
    AboutToWait, Resumed, UserWake, UserStallPoll, UserOther,
    WindowRedraw, WindowPointer, WindowOther,
    UiDrain, UiLatestTake, UiProjection, UiSeatRows, UiOldModelDrop,
    TicklessRedraw, ModelBuild, PanePrepare, GuiReceive, GuiApply,
    ModelPublish, LatestPublish, IoOldModelDrop,
    ReadRingRequest, ReadRingTerminalDecode, LogicViewPublish,
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
    ($($name:ident),+ $(,)?) => {
        #[derive(Clone, Copy, Debug)]
        #[repr(usize)]
        pub enum ProbeStage { $($name),+ }
        const STAMPS: &[ProbeStage] = &[$(ProbeStage::$name),+];
        impl ProbeStage {
            pub fn name(self) -> &'static str { match self { $(Self::$name => stringify!($name)),+ } }
        }
    };
}
stamps! {
    Admitted, ControlReceived, GuiEnqueued, WireEnqueued, WriterDequeued, WriteStarted, Written,
    LogicOutputRead, LogicRingRead, LogicLiveEvent, GuiReceived, GuiApplyStarted, GuiApplied,
    ModelBuilt, ModelPrepared, ModelPublished, LatestTaken,
    ProjectionStarted, ProjectionFinished, ModelApplied, Encoded, GpuComplete,
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
#[derive(Default)]
pub struct Recorder(OnceLock<State>);
static RECORDER: Recorder = Recorder(OnceLock::new());

#[derive(Debug)]
pub struct Sample {
    pub period: u8,
    pub phase: &'static str,
    pub calls: u64,
    pub active: u64,
    pub work: u64,
    pub bytes: u64,
    pub wall_ns: u64,
    pub wall_max_ns: u64,
    pub cpu_ns: u64,
    pub cpu_max_ns: u64,
    pub cpu_samples: u64,
    pub cpu_errors: u64,
}
#[derive(Debug)]
pub struct Snapshot {
    pub enabled: bool,
    pub sampled_ns: u64,
    pub samples: Vec<Sample>,
    pub probe_id: u64,
    pub probe_seat: u128,
    pub probe_generation: u64,
    pub probe_sequence: u64,
    pub probe: Vec<(&'static str, Option<u64>)>,
}
impl Recorder {
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
            work: 0,
            bytes: 0,
        }))
    }
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
    work: u64,
    bytes: u64,
}
pub struct Span<'a>(Option<Measurement<'a>>);
impl Span<'_> {
    pub fn enabled(&self) -> bool {
        self.0.is_some()
    }
    pub fn work(mut self, work: u64, bytes: u64) -> Self {
        if let Some(m) = &mut self.0 {
            m.work = work;
            m.bytes = bytes;
        }
        self
    }
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
        if let Some(start) = m.cpu {
            match crate::fault::thread_cpu_time() {
                Ok(end) => {
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
    }
}
fn ns(value: Duration) -> u64 {
    value.as_nanos().min(u128::from(u64::MAX)) as u64
}
fn elapsed_ns(state: &State) -> u64 {
    ns(state.origin.elapsed()).saturating_add(1)
}

pub fn enable() {
    RECORDER.enable();
}
pub fn span(phase: Phase) -> Span<'static> {
    RECORDER.span_with(phase, || None)
}
pub fn callback(phase: Phase) -> Span<'static> {
    RECORDER.span_with(phase, || crate::fault::thread_cpu_time().ok())
}
pub fn now() -> Option<u64> {
    RECORDER.0.get().map(elapsed_ns)
}
pub fn at(instant: Instant) -> Option<u64> {
    RECORDER
        .0
        .get()
        .map(|state| ns(instant.saturating_duration_since(state.origin)).saturating_add(1))
}
pub fn calls(phase: Phase) -> Option<u64> {
    RECORDER.0.get().map(|state| {
        state.counters[usize::from(state.period.load(Relaxed))][phase as usize]
            .calls
            .load(Relaxed)
    })
}
pub fn set_period(period: u8) {
    if let Some(state) = RECORDER.0.get() {
        state.period.store(period.min(PERIODS as u8 - 1), Relaxed);
    }
}
pub fn snapshot() -> Snapshot {
    RECORDER.snapshot()
}
pub fn probe_target(seat: u128, generation: u64) {
    if let Some(state) = RECORDER.0.get() {
        state.probe.seat_hi.store((seat >> 64) as u64, Relaxed);
        state.probe.seat_lo.store(seat as u64, Relaxed);
        state.probe.generation.store(generation, Relaxed);
    }
}
pub fn arm_probe(id: u64) {
    if let Some(state) = RECORDER.0.get() {
        state.probe.id.store(id, Relaxed);
    }
}
pub fn probe_mark(stage: ProbeStage) {
    if probe_pending(stage) {
        probe_mark_at(stage, now());
    }
}
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
pub fn probe_pending(stage: ProbeStage) -> bool {
    RECORDER.0.get().is_some_and(|state| {
        state.probe.id.load(Relaxed) != 0 && state.probe.stamps[stage as usize].load(Relaxed) == 0
    })
}
pub fn probe_after(required: ProbeStage, stage: ProbeStage) {
    if let Some(state) = RECORDER.0.get()
        && state.probe.stamps[required as usize].load(Relaxed) != 0
    {
        probe_mark(stage);
    }
}
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
