//! One actor owns the PTY masters, child handles, sequence counters and writers.

use std::collections::{BTreeMap, HashMap};
use std::io::{self, Read};
use std::os::fd::RawFd;
use std::path::PathBuf;
use std::sync::{Arc, mpsc};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use mio::unix::SourceFd;
use mio::{Events, Interest, Poll, Token, Waker};
use portable_pty::{Child, CommandBuilder, MasterPty, PtySize, native_pty_system};
use sha2::{Digest, Sha256};
use ulid::Ulid;

use crate::protocol::*;
use crate::ring::{CaptureHandle, CaptureQuery, CaptureResponse, CaptureWriter};
use crate::transport::{BoundSocket, SocketPeer};

#[cfg(target_os = "macos")]
mod child_events;
mod companion;

const WAKE: Token = Token(0);
const LISTENER: Token = Token(1);
const CHANNEL_CAPACITY: usize = 128;
const MAX_CLIENTS: usize = 8;
const MAX_INPUT_IDS: usize = 4096;
const TICK: Duration = Duration::from_millis(20);
const CALL_TIMEOUT: Duration = Duration::from_secs(10);
const OUTPUT_DELAY: Duration = Duration::from_millis(2);

/// Opt-in, bounded numeric diagnostics shared by the capture owners. Recording
/// never waits for a reader: contention drops a sample and increments `missed`.
/// No payload, capability or signing material enters these buffers.
pub mod capture_diagnostics {
    use std::collections::{HashMap, VecDeque};
    use std::sync::{
        Arc, Mutex, OnceLock, TryLockError,
        atomic::{AtomicBool, AtomicU64, Ordering},
    };
    use std::time::Instant;

    use serde::Serialize;
    use ulid::Ulid;

    const SPANS: usize = 8192;
    const SEATS: usize = 2048;

    /// Spans are inclusive, not a flat partition. In particular RPC waits
    /// overlap supervisor query/sweep time, and replay contains parse/checkpoint.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
    #[serde(rename_all = "snake_case")]
    #[repr(usize)]
    pub enum Phase {
        /// Caller logical-owner turn.
        LogicTurn,
        /// Caller command admission and dispatch.
        Commands,
        /// Receiving host events.
        Receive,
        /// Applying one observed host event.
        Event,
        /// Replaying a bounded retained-capture page.
        ReplayPage,
        /// Submitting a retained-capture request.
        CaptureRequest,
        /// Waiting for a retained-ring read reply.
        ReadRingRpc,
        /// Waiting for capture recovery state.
        RecoveryRpc,
        /// Waiting for checkpoint publication.
        CheckpointRpc,
        /// Parsing captured terminal bytes.
        Parse,
        /// Preparing a terminal checkpoint.
        Checkpoint,
        /// Caller-labelled maintenance work.
        Maintenance,
        /// One supervisor actor turn.
        SupervisorTurn,
        /// Supervisor control dispatch.
        SupervisorDispatch,
        /// Supervisor capture-query service.
        SupervisorQuery,
        /// Supervisor maintenance sweep.
        SupervisorSweep,
        /// Caller-labelled provider maintenance.
        MaintenanceProviders,
        /// Caller-labelled adapter maintenance.
        MaintenanceAdapters,
        /// Caller-labelled identity maintenance.
        MaintenanceIdentity,
        /// Caller-labelled message maintenance.
        MaintenanceMessages,
        /// Caller-labelled provenance maintenance.
        MaintenanceProvenance,
        /// Serving queued checkpoint work.
        CheckpointService,
    }
    const PHASES: [Phase; 22] = [
        Phase::LogicTurn,
        Phase::Commands,
        Phase::Receive,
        Phase::Event,
        Phase::ReplayPage,
        Phase::CaptureRequest,
        Phase::ReadRingRpc,
        Phase::RecoveryRpc,
        Phase::CheckpointRpc,
        Phase::Parse,
        Phase::Checkpoint,
        Phase::Maintenance,
        Phase::SupervisorTurn,
        Phase::SupervisorDispatch,
        Phase::SupervisorQuery,
        Phase::SupervisorSweep,
        Phase::MaintenanceProviders,
        Phase::MaintenanceAdapters,
        Phase::MaintenanceIdentity,
        Phase::MaintenanceMessages,
        Phase::MaintenanceProvenance,
        Phase::CheckpointService,
    ];

    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
    #[serde(rename_all = "snake_case")]
    /// Observed diagnostic span outcome, without scheduling or retry authority.
    pub enum Outcome {
        /// The measured operation completed successfully.
        Ok,
        /// The measured operation met a bounded admission refusal.
        Busy,
        /// The measured dependency was unavailable.
        Unavailable,
        #[default]
        /// The measured operation failed or the guard was dropped without a success outcome.
        Error,
    }

    #[derive(Clone, Copy, Debug, Serialize)]
    /// One inclusive timing span with caller-supplied numeric capture context.
    pub struct Span {
        /// Stable diagnostic phase assigned by the caller.
        pub phase: Phase,
        /// Process-local owner-turn counter associated with this observation.
        pub turn: u64,
        /// Start time in process-relative monotonic nanoseconds.
        pub start_ns: u64,
        /// Completed inclusive wall duration in nanoseconds.
        pub elapsed_ns: u64,
        /// Opaque process identity associated with this observation.
        pub seat: Ulid,
        /// Process-generation fence associated with this observation.
        pub generation: u64,
        /// First capture sequence associated with this measured operation.
        pub from: u64,
        /// Last capture sequence associated with this measured operation.
        pub to: u64,
        /// Correlated request identifier, or nil when none is attached.
        pub request_id: Ulid,
        /// Explicit diagnostic completion outcome.
        pub outcome: Outcome,
        /// Pages for LogicTurn; events for ReplayPage/Parse; seats for Sweep;
        /// commands for Commands. Other phases leave this zero.
        pub work: u64,
        /// Caller-supplied completed byte count.
        pub bytes: u64,
        /// Pending seat slots (logic) or pending capture requests (supervisor).
        pub pending: u64,
        /// Ready deque length, including entries whose host is parked.
        pub ready: u64,
        /// Parked host count, not a number of parked seats.
        pub parked: u64,
    }

    /// Completed spans only; a missing sample makes these a lower bound. Read
    /// `in_flight` as well when a phase has not returned at snapshot time.
    #[derive(Clone, Debug, Default, Serialize)]
    pub struct Totals {
        /// Number of completed diagnostic spans.
        pub calls: u64,
        /// Completed inclusive wall duration in nanoseconds.
        pub elapsed_ns: u64,
        /// Largest completed inclusive duration in nanoseconds.
        pub max_ns: u64,
        /// Caller-defined completed work units.
        pub work: u64,
        /// Caller-supplied completed byte count.
        pub bytes: u64,
        /// Completed non-success outcomes, including busy/unavailable observations.
        pub errors: u64,
        /// Completed outcomes explicitly marked Busy.
        pub busy: u64,
        /// Largest pending-work count observed at span completion.
        pub pending_peak: u64,
    }

    #[derive(Clone, Debug, Serialize)]
    /// Sampled active-span count and the current overlapping group start.
    pub struct InFlight {
        /// Number of currently active spans in this phase.
        pub count: u64,
        /// First start in the current overlapping group; it may precede the
        /// oldest still-active span. Counts/timestamps are sampled separately.
        pub group_started_ns: u64,
    }

    /// A last-observed head is not a fresh ring query. Compare only within the
    /// same generation, using the timestamps to bound observation age.
    #[derive(Clone, Debug, Default, Serialize)]
    pub struct Seat {
        /// Opaque process identity associated with this observation.
        pub seat: Ulid,
        /// Process-generation fence associated with this observation.
        pub generation: u64,
        /// Most recent row observation in process-relative monotonic nanoseconds.
        pub observed_ns: u64,
        /// Latest capture sequence observed through a notification.
        pub notified_sequence: u64,
        /// Latest sequence selected as a replay target.
        pub target_sequence: u64,
        /// Time of the latest recorded ring-head observation.
        pub ring_observed_ns: u64,
        /// Latest ordered capture head observed for this generation.
        pub captured_sequence: u64,
        /// Latest committed capture head observed for this generation.
        pub committed_sequence: u64,
        /// Time of the latest applied-sequence observation.
        pub applied_observed_ns: u64,
        /// Latest capture sequence recorded as applied.
        pub applied_sequence: u64,
        /// Time when the applied sequence last advanced.
        pub last_progress_ns: u64,
        /// Number of replay pages recorded for this generation.
        pub pages: u64,
        /// Number of events recorded as parsed.
        pub parsed_events: u64,
        /// Original output bytes recorded as parsed.
        pub parsed_bytes: u64,
        /// Latest checkpoint capture sequence observed.
        pub checkpoint_sequence: u64,
        /// Original-output byte cursor associated with the observed checkpoint.
        pub checkpoint_byte_end: u64,
        /// Original-output byte cursor recorded as parsed.
        pub parsed_byte_end: u64,
        /// Number of observed checkpoint-credit pauses.
        pub checkpoint_pauses: u64,
        /// Whether checkpoint admission was last observed paused.
        pub checkpoint_paused: bool,
        /// Number of checkpoints attributed to idle maintenance.
        pub idle_checkpoints: u64,
        /// Number of forced checkpoints recorded.
        pub forced_checkpoints: u64,
        /// Admission to the actor's notification queue, not receipt by logic.
        pub notification_attempts: u64,
        /// Number of notifications admitted by the actor queue.
        pub notification_accepted: u64,
        /// Capture sequence of the latest notification admission attempt.
        pub notification_last_attempt: u64,
        /// Capture sequence of the latest accepted notification.
        pub notification_last_accepted: u64,
    }
    impl Seat {
        /// Record an applied-sequence observation and update progress time only when it advances.
        pub fn progress(&mut self, sequence: u64, now: u64) {
            if sequence > self.applied_sequence {
                self.last_progress_ns = now;
            }
            self.applied_sequence = sequence;
            self.applied_observed_ns = now;
        }
    }

    #[derive(Clone, Debug, Serialize)]
    /// Bounded diagnostic copy; timestamps and rows are observations, not an atomic cut.
    pub struct Snapshot {
        /// Whether the recorder buffers were explicitly enabled.
        pub enabled: bool,
        /// Opt-in recovery-fixture audit; live traces leave this false.
        pub fixture_lossless: bool,
        /// All timestamps use one process-relative monotonic clock. Compare
        /// logic and supervisor directly only when their process_id matches.
        pub process_id: u32,
        /// Completion of the copy. Rows are earlier observations, not a single
        /// atomic cut; none may appear to come from after this timestamp.
        pub sampled_ns: u64,
        /// Recorder-buffer initialization time in process-relative nanoseconds.
        pub started_ns: u64,
        /// Process-local owner-turn counter associated with this observation.
        pub turn: u64,
        /// Maximum number of detailed retained span records.
        pub span_capacity: usize,
        /// Maximum number of distinct retained process observation rows.
        pub seat_capacity: usize,
        /// Detailed span records evicted by the fixed retention bound.
        pub overwritten_spans: u64,
        /// Ordinary diagnostic updates dropped due to buffer contention.
        pub missed: u64,
        /// Updates omitted because the process-row capacity was exhausted.
        pub untracked_seat_samples: u64,
        /// Cumulative completed-span counters in fixed phase order.
        pub totals: Vec<(Phase, Totals)>,
        /// Active-span observations in fixed phase order.
        pub in_flight: Vec<(Phase, InFlight)>,
        /// Last-observed process rows sorted by opaque identity.
        pub seats: Vec<Seat>,
        /// Retained completed spans in completion order.
        pub spans: Vec<Span>,
    }

    struct Buffer {
        started_ns: u64,
        overwritten: u64,
        untracked: u64,
        totals: [Totals; PHASES.len()],
        seats: HashMap<Ulid, Seat>,
        spans: VecDeque<Span>,
    }

    #[derive(Default)]
    /// Opt-in bounded capture recorder with non-waiting ordinary sample admission.
    pub struct Recorder {
        buffer: OnceLock<Mutex<Buffer>>,
        fixture_lossless: AtomicBool,
        turn: AtomicU64,
        pages: AtomicU64,
        missed: AtomicU64,
        active: [AtomicU64; PHASES.len()],
        active_since: [AtomicU64; PHASES.len()],
    }
    impl Recorder {
        /// Allocate the fixed capacity once, on IO/bootstrap, before the run.
        pub fn enable(&self) {
            bbt_rt::assert_not_ui_thread();
            self.buffer.get_or_init(|| {
                Mutex::new(Buffer {
                    started_ns: now_ns(),
                    overwritten: 0,
                    untracked: 0,
                    totals: std::array::from_fn(|_| Totals::default()),
                    seats: HashMap::with_capacity(SEATS),
                    spans: VecDeque::with_capacity(SPANS),
                })
            });
        }

        /// Recovery-fixture audit only, enabled before the controlled kill.
        /// Updates serialize in the bounded buffer; ordinary traces keep try_lock.
        pub fn enable_fixture_lossless(&self) {
            self.fixture_lossless.store(true, Ordering::Relaxed);
            self.enable();
        }

        /// Begin a bounded diagnostic span; a disabled recorder returns an inactive guard.
        pub fn start(
            self: &Arc<Self>,
            phase: Phase,
            seat: Ulid,
            generation: u64,
            from: u64,
            to: u64,
        ) -> Guard {
            if self.buffer.get().is_none() {
                return Guard(None);
            }
            let turn = if matches!(phase, Phase::LogicTurn | Phase::SupervisorTurn) {
                self.pages.store(0, Ordering::Relaxed);
                self.turn.fetch_add(1, Ordering::Relaxed) + 1
            } else {
                self.turn.load(Ordering::Relaxed)
            };
            let start_ns = now_ns();
            if self.active[phase as usize].fetch_add(1, Ordering::Relaxed) == 0 {
                self.active_since[phase as usize].store(start_ns, Ordering::Relaxed);
            }
            Guard(Some((
                self.clone(),
                Span {
                    phase,
                    turn,
                    start_ns,
                    elapsed_ns: 0,
                    seat,
                    generation,
                    from,
                    to,
                    request_id: Ulid::nil(),
                    outcome: Outcome::Error,
                    work: 0,
                    bytes: 0,
                    pending: 0,
                    ready: 0,
                    parked: 0,
                },
            )))
        }

        #[allow(
            clippy::disallowed_methods,
            reason = "explicit recovery-fixture audit serializes bounded diagnostic updates"
        )]
        fn update(&self, f: impl FnOnce(&mut Buffer)) {
            let Some(buffer) = self.buffer.get() else {
                return;
            };
            if self.fixture_lossless.load(Ordering::Relaxed) {
                let mut buffer = buffer.lock().unwrap_or_else(|error| error.into_inner());
                f(&mut buffer);
                return;
            }
            match buffer.try_lock() {
                Ok(mut buffer) => f(&mut buffer),
                Err(TryLockError::Poisoned(error)) => f(&mut error.into_inner()),
                Err(TryLockError::WouldBlock) => {
                    self.missed.fetch_add(1, Ordering::Relaxed);
                }
            }
        }

        /// Update one bounded process observation row; a new generation resets its prior counters.
        pub fn seat(&self, seat: Ulid, generation: u64, f: impl FnOnce(&mut Seat, u64)) {
            self.update(|buffer| {
                if !buffer.seats.contains_key(&seat) && buffer.seats.len() == SEATS {
                    buffer.untracked += 1;
                    return;
                }
                let row = buffer.seats.entry(seat).or_default();
                if row.generation != generation {
                    *row = Seat {
                        seat,
                        generation,
                        ..Seat::default()
                    };
                }
                let now = now_ns();
                row.observed_ns = now;
                f(row, now);
            });
        }

        #[allow(
            clippy::disallowed_methods,
            reason = "bounded diagnostic snapshot on IO, never the UI"
        )]
        /// Copy bounded recorder data on IO without draining or resetting it.
        ///
        /// # Panics
        /// In debug builds, panics on the registered UI thread.
        pub fn snapshot(&self) -> Snapshot {
            bbt_rt::assert_not_ui_thread();
            let mut snapshot = Snapshot {
                enabled: self.buffer.get().is_some(),
                fixture_lossless: self.fixture_lossless.load(Ordering::Relaxed),
                process_id: std::process::id(),
                sampled_ns: now_ns(),
                started_ns: 0,
                turn: self.turn.load(Ordering::Relaxed),
                span_capacity: SPANS,
                seat_capacity: SEATS,
                overwritten_spans: 0,
                missed: self.missed.load(Ordering::Relaxed),
                untracked_seat_samples: 0,
                totals: Vec::new(),
                in_flight: PHASES
                    .into_iter()
                    .map(|phase| {
                        (
                            phase,
                            InFlight {
                                count: self.active[phase as usize].load(Ordering::Relaxed),
                                group_started_ns: self.active_since[phase as usize]
                                    .load(Ordering::Relaxed),
                            },
                        )
                    })
                    .collect(),
                seats: Vec::new(),
                spans: Vec::new(),
            };
            if let Some(buffer) = self.buffer.get() {
                let buffer = buffer
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                snapshot.started_ns = buffer.started_ns;
                snapshot.overwritten_spans = buffer.overwritten;
                snapshot.untracked_seat_samples = buffer.untracked;
                snapshot.totals = PHASES
                    .into_iter()
                    .zip(buffer.totals.iter().cloned())
                    .collect();
                snapshot.seats = buffer.seats.values().cloned().collect();
                snapshot.spans = buffer.spans.iter().copied().collect();
            }
            snapshot.seats.sort_by_key(|seat| seat.seat);
            snapshot.sampled_ns = now_ns();
            snapshot
        }
    }

    /// RAII diagnostic span; dropping it records completion once.
    pub struct Guard(Option<(Arc<Recorder>, Span)>);
    impl Guard {
        /// Attach a request correlation identifier to this span.
        pub fn request(&mut self, request_id: Ulid) {
            if let Some((_, span)) = &mut self.0 {
                span.request_id = request_id;
            }
        }
        /// Attach pending-work, ready-queue and parked-host counts to this span.
        pub fn pending(&mut self, pending: usize, ready: usize, parked: usize) {
            if let Some((_, span)) = &mut self.0 {
                span.pending = pending as u64;
                span.ready = ready as u64;
                span.parked = parked as u64;
            }
        }
        /// Set the final outcome and work counts, then record completion by consuming the guard.
        pub fn finish(mut self, outcome: Outcome, work: u64, bytes: u64) {
            if let Some((_, span)) = &mut self.0 {
                span.outcome = outcome;
                span.work = work;
                span.bytes = bytes;
            }
        }
    }
    impl Drop for Guard {
        fn drop(&mut self) {
            let Some((recorder, mut span)) = self.0.take() else {
                return;
            };
            span.elapsed_ns = now_ns().saturating_sub(span.start_ns);
            recorder.active[span.phase as usize].fetch_sub(1, Ordering::Relaxed);
            if span.phase == Phase::ReplayPage && span.work > 0 {
                recorder.pages.fetch_add(1, Ordering::Relaxed);
            } else if span.phase == Phase::LogicTurn {
                span.work = recorder.pages.load(Ordering::Relaxed);
            }
            recorder.update(|buffer| {
                let total = &mut buffer.totals[span.phase as usize];
                total.calls += 1;
                total.elapsed_ns += span.elapsed_ns;
                total.max_ns = total.max_ns.max(span.elapsed_ns);
                total.work += span.work;
                total.bytes += span.bytes;
                total.errors += u64::from(span.outcome != Outcome::Ok);
                total.busy += u64::from(span.outcome == Outcome::Busy);
                total.pending_peak = total.pending_peak.max(span.pending);
                if buffer.spans.len() == SPANS {
                    buffer.spans.pop_front();
                    buffer.overwritten += 1;
                }
                buffer.spans.push_back(span);
            });
        }
    }

    fn now_ns() -> u64 {
        static ORIGIN: OnceLock<Instant> = OnceLock::new();
        ORIGIN
            .get_or_init(Instant::now)
            .elapsed()
            .as_nanos()
            .min(u64::MAX as u128) as u64
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        // keeps: spec mux-daemon.md — Ownership and failure boundaries
        #[test]
        fn bounded_trace_retains_totals_and_reports_overwrite_and_contention() {
            let recorder = Arc::new(Recorder::default());
            recorder
                .start(Phase::Parse, Ulid::nil(), 0, 0, 0)
                .finish(Outcome::Ok, 9, 10);
            assert!(!recorder.snapshot().enabled);
            assert!(recorder.snapshot().spans.is_empty());
            recorder.enable();
            assert!(!recorder.snapshot().fixture_lossless);
            for _ in 0..SPANS + 3 {
                recorder
                    .start(Phase::ReadRingRpc, Ulid::nil(), 1, 2, 3)
                    .finish(Outcome::Busy, 1, 4);
            }
            let snapshot = recorder.snapshot();
            assert_eq!(snapshot.spans.len(), SPANS);
            assert_eq!(snapshot.overwritten_spans, 3);
            let total = &snapshot.totals[Phase::ReadRingRpc as usize].1;
            assert_eq!(total.calls, SPANS as u64 + 3);
            assert_eq!(total.busy, total.calls);
            assert_eq!(total.bytes, 4 * total.calls);
            let lock = recorder.buffer.get().unwrap().try_lock().unwrap();
            recorder
                .start(Phase::Parse, Ulid::nil(), 1, 0, 0)
                .finish(Outcome::Ok, 1, 1);
            drop(lock);
            assert_eq!(
                recorder.snapshot().missed,
                1,
                "recording waited for a reader"
            );
        }

        // keeps: spec mux-daemon.md — Ownership and failure boundaries
        #[test]
        fn fixture_lossless_counts_every_concurrent_writer_update() {
            let recorder = Arc::new(Recorder::default());
            recorder.enable_fixture_lossless();
            let start = Arc::new(std::sync::Barrier::new(5));
            let writers: Vec<_> = (0..4)
                .map(|_| {
                    let recorder = recorder.clone();
                    let start = start.clone();
                    std::thread::spawn(move || {
                        start.wait();
                        for _ in 0..4096 {
                            recorder.start(Phase::Parse, Ulid::nil(), 0, 0, 0).finish(
                                Outcome::Ok,
                                1,
                                1,
                            );
                        }
                    })
                })
                .collect();
            start.wait();
            for writer in writers {
                writer.join().unwrap();
            }
            let snapshot = recorder.snapshot();
            assert!(snapshot.fixture_lossless);
            assert_eq!(snapshot.missed, 0);
            assert_eq!(snapshot.totals[Phase::Parse as usize].1.calls, 4 * 4096);
            assert_eq!(snapshot.totals[Phase::Parse as usize].1.work, 4 * 4096);
            assert_eq!(snapshot.spans.len(), SPANS);
            assert_eq!(snapshot.overwritten_spans, 4 * 4096 - SPANS as u64);
        }

        // keeps: spec mux-daemon.md — Ownership and failure boundaries
        #[test]
        fn cursor_generations_and_turn_pages_remain_distinct() {
            let recorder = Arc::new(Recorder::default());
            recorder.enable();
            let seat = Ulid::from(1);
            let mut turn = recorder.start(Phase::LogicTurn, Ulid::nil(), 0, 0, 0);
            let active = recorder.snapshot();
            assert_eq!(active.in_flight[Phase::LogicTurn as usize].1.count, 1);
            recorder.seat(seat, 1, |row, now| {
                row.progress(5, now);
                row.notified_sequence = 10;
                row.captured_sequence = 20;
                row.ring_observed_ns = now;
            });
            recorder
                .start(Phase::ReplayPage, seat, 1, 6, 9)
                .finish(Outcome::Ok, 4, 100);
            turn.pending(7, 6, 1);
            turn.finish(Outcome::Ok, 0, 0);
            let snapshot = recorder.snapshot();
            assert_eq!(snapshot.in_flight[Phase::LogicTurn as usize].1.count, 0);
            assert_eq!(snapshot.spans.last().unwrap().work, 1);
            assert_eq!(snapshot.spans.last().unwrap().pending, 7);
            assert_eq!(
                snapshot.seats[0].captured_sequence - snapshot.seats[0].applied_sequence,
                15
            );
            assert!(snapshot.seats[0].observed_ns <= snapshot.sampled_ns);
            recorder.seat(seat, 2, |row, now| row.progress(1, now));
            let snapshot = recorder.snapshot();
            assert_eq!(snapshot.seats[0].generation, 2);
            assert_eq!(snapshot.seats[0].applied_sequence, 1);
            assert_eq!(snapshot.seats[0].captured_sequence, 0);
            for id in 2..=SEATS + 1 {
                recorder.seat(Ulid::from(id as u128), 1, |_, _| {});
            }
            let snapshot = recorder.snapshot();
            assert_eq!(snapshot.seats.len(), SEATS);
            assert_eq!(snapshot.untracked_seat_samples, 1);
            assert!(serde_json::to_vec(&snapshot).is_ok());
        }
    }
}
fn diagnostic_outcome<T>(result: &Result<T, Failure>) -> capture_diagnostics::Outcome {
    match result {
        Ok(_) => capture_diagnostics::Outcome::Ok,
        Err(error) if error.code == FailureCode::Busy => capture_diagnostics::Outcome::Busy,
        Err(_) => capture_diagnostics::Outcome::Error,
    }
}
// Non-macOS fallback is unchanged; this pass validates the macOS cockpit only.
#[cfg(not(target_os = "macos"))]
const CHILD_POLL_INTERVAL: Duration = Duration::from_millis(100);

// Pure close policy; the actor supplies its production clocks and liveness.
// Tests advance those clock samples in-process without a PTY or a real wait.
#[derive(Default)]
struct CloseState {
    grace: Option<(Ulid, Instant)>,
    ids: HashMap<Ulid, (u64, bool)>,
}
impl CloseState {
    fn schedule(
        &mut self,
        close_id: Ulid,
        deadline_ms: u64,
        live: Result<(), Failure>,
        now_ms: u64,
        now: Instant,
    ) -> Result<Reply, Failure> {
        if let Some((deadline, canceled)) = self.ids.get(&close_id) {
            if *canceled || *deadline != deadline_ms {
                return Err(Failure::new(
                    FailureCode::Invalid,
                    "close id was canceled or conflicts",
                ));
            }
            return Ok(Reply::CloseScheduled {
                close_id,
                deadline_ms: *deadline,
            });
        }
        live?;
        if close_id == Ulid::nil()
            || deadline_ms.saturating_sub(now_ms) > 86_400_000
            || self.ids.len() >= 4096
            || self.grace.is_some()
        {
            return Err(Failure::new(
                FailureCode::Invalid,
                "invalid or conflicting close",
            ));
        }
        self.grace = Some((
            close_id,
            now + Duration::from_millis(deadline_ms.saturating_sub(now_ms)),
        ));
        self.ids.insert(close_id, (deadline_ms, false));
        Ok(Reply::CloseScheduled {
            close_id,
            deadline_ms,
        })
    }
    fn undo(
        &mut self,
        close_id: Ulid,
        live: Result<(), Failure>,
        now: Instant,
    ) -> Result<Reply, Failure> {
        if self
            .ids
            .get(&close_id)
            .is_some_and(|(_, canceled)| *canceled)
        {
            return Ok(Reply::CloseUndone { close_id });
        }
        live?;
        match self.grace {
            Some((id, deadline)) if id == close_id && now < deadline => {
                self.grace = None;
                if let Some((_, canceled)) = self.ids.get_mut(&close_id) {
                    *canceled = true;
                }
                Ok(Reply::CloseUndone { close_id })
            }
            _ => Err(Failure::new(
                FailureCode::Exited,
                "close expired or revision changed",
            )),
        }
    }
    fn expired(&self, now: Instant) -> bool {
        self.grace.is_some_and(|(_, deadline)| now >= deadline)
    }
}

/// Explicit supervisor endpoints, process limits, input deadline and caller probes.
pub struct Config {
    /// None is the v0.1 in-process transport; Some enables the same protocol on Unix IPC.
    pub socket_path: Option<PathBuf>,
    /// Required for AttachRing. Supervisor-wide, outside all seat storage units.
    pub journal_directory: Option<PathBuf>,
    /// Maximum retained processes; admission accepts 1..=1024.
    pub max_seats: usize,
    /// A partially written input times out as unknown; it is never resubmitted.
    pub write_timeout: Duration,
    /// Exact caller-owned probes that preserve the input epoch, time and draft.
    pub input_probes: Vec<Vec<u8>>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            socket_path: None,
            journal_directory: None,
            max_seats: 1024,
            write_timeout: Duration::from_secs(5),
            input_probes: Vec::new(),
        }
    }
}

/// Owning this value keeps the single supervisor thread alive. Dropping clients
/// does not kill seats. Dropping the supervisor explicitly terminates/reaps them.
pub struct Supervisor {
    tx: mpsc::SyncSender<Control>,
    waker: Arc<Waker>,
    thread: Option<JoinHandle<io::Result<()>>>,
    capability: [u8; 32],
    capture_diagnostics: Arc<capture_diagnostics::Recorder>,
}

impl Supervisor {
    /// Start the owning supervisor thread and optional explicit Unix endpoint.
    ///
    /// # Errors
    /// Refuses invalid limits or returns poller, endpoint, entropy, capture-writer or thread-start failures.
    pub fn start(config: Config) -> io::Result<Self> {
        if config.max_seats == 0 || config.max_seats > 1024 || config.write_timeout.is_zero() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid supervisor limits",
            ));
        }
        let poll = Poll::new()?;
        let waker = Arc::new(Waker::new(poll.registry(), WAKE)?);
        let mut socket = config
            .socket_path
            .as_deref()
            .map(BoundSocket::bind)
            .transpose()?;
        if let Some(socket) = socket.as_mut() {
            poll.registry()
                .register(&mut socket.listener, LISTENER, Interest::READABLE)?;
        }
        let mut capability = [0; 32];
        getrandom::fill(&mut capability).map_err(io::Error::other)?;
        let (tx, rx) = mpsc::sync_channel(CHANNEL_CAPACITY);
        let capture_diagnostics = Arc::new(capture_diagnostics::Recorder::default());
        let mut actor = Actor {
            capture_writer: CaptureWriter::start(
                waker.clone(),
                config.journal_directory.as_deref(),
            )?,
            poll,
            socket,
            config,
            capability,
            boot: Ulid::new(),
            rx,
            clients: HashMap::new(),
            routes: HashMap::new(),
            seats: BTreeMap::new(),
            last_notified_seat: None,
            notification_waiter: None,
            active: None,
            next_token: 2,
            pending_io: Vec::new(),
            pending_batches: Vec::new(),
            capture_diagnostics: capture_diagnostics.clone(),
        };
        let thread = thread::Builder::new()
            .name("bbt-supervisor".into())
            .spawn(move || actor.run())?;
        Ok(Self {
            tx,
            waker,
            thread: Some(thread),
            capability,
            capture_diagnostics,
        })
    }

    /// Enable bounded capture spans on IO before a diagnostic run. No RPCs or
    /// file writes are added, and recording is disabled by default.
    pub fn enable_capture_diagnostics(&self) {
        self.capture_diagnostics.enable();
    }

    /// Copy the bounded buffers on IO; does not drain or reset them.
    pub fn capture_diagnostics(&self) -> capture_diagnostics::Snapshot {
        self.capture_diagnostics.snapshot()
    }

    /// The startup owner passes this capability out of band, never via argv/logs.
    pub fn socket_capability(&self) -> [u8; 32] {
        self.capability
    }

    /// Register an in-process client and acquire its logic lease.
    ///
    /// # Errors
    /// Returns registration, admission, wake or disconnected-supervisor failures.
    pub fn connect(&self) -> Result<Client, Failure> {
        let id = Ulid::new();
        let (events_tx, events_rx) = mpsc::sync_channel(CHANNEL_CAPACITY);
        let (reply_tx, reply_rx) = mpsc::sync_channel(1);
        self.tx
            .send(Control::Register {
                id,
                events: events_tx,
                reply: reply_tx,
            })
            .map_err(|_| disconnected())?;
        self.waker.wake()?;
        reply_rx
            .recv_timeout(CALL_TIMEOUT)
            .map_err(|_| disconnected())??;
        let mut client = Client {
            id,
            lease: Ulid::nil(),
            tx: self.tx.clone(),
            waker: self.waker.clone(),
            events: events_rx,
        };
        match client.request(Action::AcquireLogic {
            capability: self.capability,
        })? {
            Reply::Acquired { logic_lease, .. } => client.lease = logic_lease,
            _ => return Err(disconnected()),
        }
        Ok(client)
    }

    /// Request owner shutdown and join after owned children and capture work are stopped.
    ///
    /// # Errors
    /// Returns supervisor thread, persistence or cleanup failure.
    pub fn shutdown(mut self) -> io::Result<()> {
        self.stop()
    }

    fn stop(&mut self) -> io::Result<()> {
        if let Some(thread) = self.thread.take() {
            let _ = self.tx.send(Control::Shutdown);
            let _ = self.waker.wake();
            thread
                .join()
                .map_err(|_| io::Error::other("supervisor thread panicked"))??;
        }
        Ok(())
    }
}

impl Drop for Supervisor {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

/// Admitted in-process control client with a bounded event receiver.
pub struct Client {
    id: Ulid,
    lease: Ulid,
    tx: mpsc::SyncSender<Control>,
    waker: Arc<Waker>,
    events: mpsc::Receiver<Event>,
}

impl Client {
    /// Return this client's admitted logic-lease identity.
    pub fn logic_lease(&self) -> Ulid {
        self.lease
    }

    /// Submit a correlated action under this client's current logic lease.
    ///
    /// # Errors
    /// Returns named host refusals or IO/disconnection; an unavailable reply leaves outcome unknown.
    pub fn request(&self, action: Action) -> Result<Reply, Failure> {
        self.request_frame(Frame::new(Request::new(Some(self.lease), action)))
    }

    /// Submit an explicit versioned request and wait for its correlated reply on IO.
    ///
    /// # Errors
    /// Returns version, lease, host admission or transport failures; query uncertain outcomes before retry.
    pub fn request_frame(&self, frame: Frame<Request>) -> Result<Reply, Failure> {
        let (reply, rx) = mpsc::sync_channel(1);
        self.tx
            .send(Control::Request {
                client: self.id,
                frame: Box::new(frame),
                reply,
            })
            .map_err(|_| disconnected())?;
        self.waker.wake()?;
        rx.recv_timeout(CALL_TIMEOUT).map_err(|_| {
            Failure::new(
                FailureCode::Io,
                "reply unavailable; operation outcome unknown, query before retry",
            )
        })?
    }

    /// Start the explicit process generation and return its host metadata.
    ///
    /// # Errors
    /// Returns launch validation, capacity, process-start, lease or request failures.
    pub fn spawn(&self, spec: LaunchSpec) -> Result<SeatInfo, Failure> {
        match self.request(Action::Spawn { spec })? {
            Reply::Spawned { info } => Ok(info),
            _ => Err(disconnected()),
        }
    }

    /// Acquire a writer attachment for an exact retained process generation.
    ///
    /// # Errors
    /// Returns stale/missing identity, writer-lease or request failures.
    pub fn attach(&self, seat: Ulid, generation: u64) -> Result<Attachment, Failure> {
        match self.request(Action::Attach { seat, generation })? {
            Reply::Attached { attachment, .. } => Ok(attachment),
            _ => Err(disconnected()),
        }
    }

    /// Submit one idempotently identified byte sequence under the attachment.
    ///
    /// # Errors
    /// Returns stale leases, invalid/oversized/conflicting input or request failures; inspect unknown writes.
    pub fn input(
        &self,
        attachment: &Attachment,
        input_id: Ulid,
        bytes: &[u8],
    ) -> Result<InputReceipt, Failure> {
        match self.request(Action::Input {
            attachment: attachment.clone(),
            input_id,
            bytes: bytes.into(),
        })? {
            Reply::Input { receipt } => Ok(receipt),
            _ => Err(disconnected()),
        }
    }

    /// Request an ordered resize under the attached generation.
    ///
    /// # Errors
    /// Returns invalid dimensions, stale leases/generation or request failures.
    pub fn resize(&self, attachment: &Attachment, size: SeatSize) -> Result<Reply, Failure> {
        self.request(Action::Resize {
            attachment: attachment.clone(),
            size,
        })
    }

    /// Asynchronously requests SIGHUP, escalating to SIGKILL after 250 ms.
    ///
    /// # Errors
    /// Returns stale attachment/generation or control-request failures.
    pub fn kill(&self, attachment: &Attachment) -> Result<Reply, Failure> {
        self.request(Action::Kill {
            attachment: attachment.clone(),
        })
    }

    /// Block on IO for one observed event up to the supplied timeout.
    ///
    /// # Errors
    /// Returns Timeout when no event arrives, or Disconnected after the event channel closes.
    pub fn recv_timeout(&self, timeout: Duration) -> Result<Event, mpsc::RecvTimeoutError> {
        self.events.recv_timeout(timeout)
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        // Never block a caller's drop. A disconnected event receiver is also
        // detected by the next attempted publication if this queue is full.
        let _ = self.tx.try_send(Control::Disconnect(self.id));
        let _ = self.waker.wake();
    }
}

enum Control {
    Register {
        id: Ulid,
        events: mpsc::SyncSender<Event>,
        reply: mpsc::SyncSender<Result<(), Failure>>,
    },
    Request {
        client: Ulid,
        frame: Box<Frame<Request>>,
        reply: mpsc::SyncSender<Result<Reply, Failure>>,
    },
    Disconnect(Ulid),
    Shutdown,
}

enum Target {
    Local(mpsc::SyncSender<Event>),
    Socket { token: Token, peer: SocketPeer },
}

#[derive(Clone, Copy)]
enum Route {
    Seat(Ulid),
    Child(Ulid),
    Companion(Ulid),
    Socket(Ulid),
}

struct Actor {
    poll: Poll,
    socket: Option<BoundSocket>,
    config: Config,
    capability: [u8; 32],
    boot: Ulid,
    rx: mpsc::Receiver<Control>,
    clients: HashMap<Ulid, Target>,
    routes: HashMap<Token, Route>,
    seats: BTreeMap<Ulid, Seat>,
    last_notified_seat: Option<Ulid>,
    notification_waiter: Option<Ulid>,
    active: Option<(Ulid, Ulid)>,
    next_token: usize,
    pending_io: Vec<PendingIo>,
    pending_batches: Vec<PendingRingBatch>,
    capture_diagnostics: Arc<capture_diagnostics::Recorder>,
    // Dropped after seats, so their queued close/flush jobs are drained first.
    capture_writer: CaptureWriter,
}

enum Dispatch {
    Ready(Result<Reply, Failure>),
    Pending(CaptureResponse, capture_diagnostics::Guard),
    Batch(PendingRingBatch),
}
struct PendingIo {
    client: Ulid,
    request_id: Ulid,
    reply: Option<mpsc::SyncSender<Result<Reply, Failure>>>,
    response: CaptureResponse,
    diagnostic: capture_diagnostics::Guard,
}

struct PendingRingRead {
    // Refused overflow metadata uses the existing reply reserve, never an IO slot.
    charged: bool,
    result: RingReadResult,
    response: Option<CaptureResponse>,
}

struct PendingRingBatch {
    client: Ulid,
    request_id: Ulid,
    reply: Option<mpsc::SyncSender<Result<Reply, Failure>>>,
    members: Vec<PendingRingRead>,
    max_bytes: usize,
    diagnostic: capture_diagnostics::Guard,
}

fn batch_limit() -> Failure {
    Failure::new(FailureCode::Limit, "ring history exceeds batch byte budget")
}

fn batch_wire_bytes(request_id: Ulid, results: &[RingReadResult]) -> io::Result<usize> {
    crate::transport::encode(&Frame::new(Message::Reply {
        request_id,
        result: Ok(Reply::RingHistoryBatch {
            results: results.to_vec(),
        }),
    }))
    .map(|bytes| bytes.len())
}

fn bounded_batch_reply(
    request_id: Ulid,
    results: Vec<RingReadResult>,
    max_bytes: usize,
) -> Result<Reply, Failure> {
    let encoding_failed = || Failure::new(FailureCode::Invalid, "ring batch reply encoding failed");
    let mut bounded: Vec<_> = results
        .iter()
        .map(|row| RingReadResult {
            seat: row.seat,
            generation: row.generation,
            from: row.from,
            to: row.to,
            result: Err(batch_limit()),
        })
        .collect();
    let mut used = batch_wire_bytes(request_id, &bounded).map_err(|_| encoding_failed())?;
    if used > max_bytes {
        return Err(Failure::new(
            FailureCode::Invalid,
            "batch budget cannot contain member results",
        ));
    }
    // Replacing a JSON array member changes only its own encoded length. Reserve
    // named errors first, so one oversized member cannot erase a later small one.
    for (index, candidate) in results.into_iter().enumerate() {
        let reserved = serde_json::to_vec(&bounded[index])
            .map_err(|_| encoding_failed())?
            .len();
        let actual = serde_json::to_vec(&candidate)
            .map_err(|_| encoding_failed())?
            .len();
        if let Some(next) = used
            .checked_sub(reserved)
            .and_then(|n| n.checked_add(actual))
            && next <= max_bytes
        {
            used = next;
            bounded[index] = candidate;
        }
    }
    if !batch_wire_bytes(request_id, &bounded).is_ok_and(|bytes| bytes <= max_bytes) {
        return Err(encoding_failed());
    }
    Ok(Reply::RingHistoryBatch { results: bounded })
}

impl Drop for Actor {
    fn drop(&mut self) {
        // Drop seat handles while the capture writer is still alive. Their
        // pending output/Close jobs must precede the writer's shutdown flush.
        self.seats.clear();
    }
}

impl Actor {
    fn tick_seats(&mut self, read_buffer: &mut [u8; OUTPUT_CHUNK_BYTES]) -> io::Result<()> {
        if self.notification_waiter.is_some_and(|id| {
            !self.seats.get(&id).is_some_and(|seat| {
                seat.pending_event.is_some()
                    && self.active.is_some_and(|(client, _)| {
                        seat.writer.is_some_and(|(writer, _)| writer == client)
                    })
            })
        }) {
            self.notification_waiter = None;
        }
        let mut ids: Vec<_> = self.seats.keys().copied().collect();
        if let Some(waiting) = self.notification_waiter {
            let start = ids.partition_point(|id| *id < waiting);
            ids.rotate_left(start);
        } else if let Some(last) = self.last_notified_seat {
            // Rotate on admissions, not sweeps: a full channel must not let
            // the same hot seat reclaim every credit the receiver returns.
            let start = ids.partition_point(|id| *id <= last);
            ids.rotate_left(start);
        }
        for id in ids {
            let mut seat = self.seats.remove(&id).unwrap();
            seat.tick(self, read_buffer)?;
            self.seats.insert(id, seat);
        }
        Ok(())
    }

    fn token(&mut self) -> Token {
        let token = Token(self.next_token);
        self.next_token = self
            .next_token
            .checked_add(1)
            .expect("poll token exhausted");
        token
    }

    fn run(&mut self) -> io::Result<()> {
        let mut events = Events::with_capacity(256);
        // Reads replace only the returned prefix; keep one initialized buffer
        // across seats and turns instead of clearing it for every output chunk.
        let mut read_buffer = [0; OUTPUT_CHUNK_BYTES];
        loop {
            let mut turn = self.capture_diagnostics.start(
                capture_diagnostics::Phase::SupervisorTurn,
                Ulid::nil(),
                0,
                0,
                0,
            );
            self.complete_io();
            for _ in 0..64 {
                match self.rx.try_recv() {
                    Ok(Control::Shutdown) => return Ok(()),
                    Ok(Control::Disconnect(id)) => self.disconnect(id),
                    Ok(Control::Register { id, events, reply }) => {
                        let result = if self.clients.len() < MAX_CLIENTS {
                            self.clients.insert(id, Target::Local(events));
                            Ok(())
                        } else {
                            Err(Failure::new(
                                FailureCode::Limit,
                                "too many logic connections",
                            ))
                        };
                        let _ = reply.send(result);
                    }
                    Ok(Control::Request {
                        client,
                        frame,
                        reply,
                    }) => {
                        let request_id = frame.body.request_id;
                        match self.dispatch(client, *frame) {
                            Dispatch::Ready(result) => {
                                let _ = reply.try_send(result);
                            }
                            Dispatch::Pending(response, diagnostic) => {
                                self.pending_io.push(PendingIo {
                                    client,
                                    request_id,
                                    reply: Some(reply),
                                    response,
                                    diagnostic,
                                })
                            }
                            Dispatch::Batch(mut batch) => {
                                batch.reply = Some(reply);
                                self.pending_batches.push(batch);
                            }
                        }
                    }
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => return Ok(()),
                }
            }
            // Every seat is driven here; no reader/writer/wait thread per seat.
            let sweep = self.capture_diagnostics.start(
                capture_diagnostics::Phase::SupervisorSweep,
                Ulid::nil(),
                0,
                0,
                0,
            );
            self.tick_seats(&mut read_buffer)?;
            sweep.finish(capture_diagnostics::Outcome::Ok, self.seats.len() as u64, 0);
            self.flush_sockets();
            let ready = self.seats.values().any(|seat| {
                seat.read_ready
                    && (seat.ring.is_some() || seat.pending_event.is_none())
                    && seat
                        .ring
                        .as_ref()
                        .map_or(seat.writer.is_some(), CaptureHandle::can_capture)
                    && !seat.exit_published
            });
            let timeout = if ready {
                Duration::ZERO
            } else {
                self.seats
                    .values()
                    .filter_map(|seat| {
                        seat.output_since
                            .filter(|_| seat.ring.as_ref().is_none_or(CaptureHandle::can_capture))
                            .map(|at| (at + OUTPUT_DELAY).saturating_duration_since(Instant::now()))
                    })
                    .min()
                    .unwrap_or(TICK)
                    .min(TICK)
            };
            match self.poll.poll(&mut events, Some(timeout)) {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(error),
            }
            for event in &events {
                match event.token() {
                    WAKE => {}
                    LISTENER => self.accept_sockets()?,
                    token => match self.routes.get(&token).copied() {
                        Some(Route::Seat(id)) => {
                            if let Some(seat) = self.seats.get_mut(&id) {
                                if event.is_readable() || event.is_read_closed() || event.is_error()
                                {
                                    seat.read_ready = true;
                                }
                                if event.is_writable() {
                                    seat.write_ready = true;
                                }
                            }
                        }
                        Some(Route::Child(id)) => {
                            self.routes.remove(&token);
                            if let Some(seat) = self.seats.get_mut(&id) {
                                seat.complete_child_watch();
                                seat.reap_child()?;
                            }
                        }
                        Some(Route::Companion(id)) => {
                            self.routes.remove(&token);
                            if let Some(companion) =
                                self.seats.get_mut(&id).and_then(|s| s.companion.as_mut())
                            {
                                #[cfg(target_os = "macos")]
                                companion.complete_watch();
                                companion.poll_exit()?;
                            }
                        }
                        Some(Route::Socket(id)) => self.read_socket(id),
                        None => {}
                    },
                }
            }
            turn.pending(self.pending_capture_slots(), 0, 0);
            turn.finish(capture_diagnostics::Outcome::Ok, 0, 0);
        }
    }

    fn dispatch(&mut self, client: Ulid, frame: Frame<Request>) -> Dispatch {
        if matches!(&frame.body.action, Action::ReadRingBatch { .. }) {
            let mut span = self.capture_diagnostics.start(
                capture_diagnostics::Phase::SupervisorDispatch,
                Ulid::nil(),
                0,
                0,
                0,
            );
            span.request(frame.body.request_id);
            span.pending(self.pending_capture_slots(), 0, 0);
            let result = self.dispatch_ring_batch(client, frame);
            span.finish(diagnostic_outcome(&result), 0, 0);
            return match result {
                Ok(batch) => Dispatch::Batch(batch),
                Err(error) => Dispatch::Ready(Err(error)),
            };
        }
        if !matches!(
            &frame.body.action,
            Action::RingRecovery { .. }
                | Action::ReadRing { .. }
                | Action::PublishCheckpoint { .. }
        ) {
            return Dispatch::Ready(self.dispatch_now(client, frame));
        }
        let (seat, generation, from, to) = match &frame.body.action {
            Action::ReadRing {
                attachment,
                from,
                to,
            } => (attachment.seat, attachment.generation, *from, *to),
            Action::RingRecovery { attachment } | Action::PublishCheckpoint { attachment, .. } => {
                (attachment.seat, attachment.generation, 0, 0)
            }
            _ => unreachable!(),
        };
        let mut dispatch = self.capture_diagnostics.start(
            capture_diagnostics::Phase::SupervisorDispatch,
            seat,
            generation,
            from,
            to,
        );
        dispatch.request(frame.body.request_id);
        dispatch.pending(self.pending_capture_slots(), 0, 0);
        let mut query = self.capture_diagnostics.start(
            capture_diagnostics::Phase::SupervisorQuery,
            seat,
            generation,
            from,
            to,
        );
        query.request(frame.body.request_id);
        let result = (|| {
            if frame.version != VERSION {
                return Err(Failure::new(FailureCode::Version, "unsupported version"));
            }
            if !self.clients.contains_key(&client) {
                return Err(disconnected());
            }
            if frame.body.request_id == Ulid::nil() {
                return Err(Failure::new(FailureCode::Invalid, "nil request id"));
            }
            if self.active.is_none()
                || self.active != frame.body.logic_lease.map(|lease| (client, lease))
            {
                return Err(Failure::new(
                    FailureCode::StaleLease,
                    "logic lease was replaced or disconnected",
                ));
            }
            if self.pending_capture_slots() >= CHANNEL_CAPACITY {
                return Err(Failure::new(
                    FailureCode::Busy,
                    "too many pending ring requests",
                ));
            }
            let (attachment, query) = match frame.body.action {
                Action::RingRecovery { attachment } => (attachment, CaptureQuery::Recovery),
                Action::ReadRing {
                    attachment,
                    from,
                    to,
                } => (attachment, CaptureQuery::History { from, to }),
                Action::PublishCheckpoint {
                    attachment,
                    checkpoint,
                } => (attachment, CaptureQuery::Checkpoint(checkpoint)),
                _ => unreachable!(),
            };
            let seat = self.seat_mut(attachment.seat, attachment.generation)?;
            seat.check_writer(client, &attachment)?;
            let ring = seat
                .ring
                .as_ref()
                .ok_or_else(|| Failure::new(FailureCode::NotFound, "no ring configured"))?;
            ring.request(query)
        })();
        let outcome = diagnostic_outcome(&result);
        dispatch.finish(outcome, 0, 0);
        match result {
            Ok(response) => Dispatch::Pending(response, query),
            Err(error) => {
                query.finish(outcome, 0, 0);
                Dispatch::Ready(Err(error))
            }
        }
    }

    fn pending_capture_slots(&self) -> usize {
        self.pending_io.len()
            + self
                .pending_batches
                .iter()
                .map(|batch| batch.members.iter().filter(|member| member.charged).count())
                .sum::<usize>()
    }

    fn dispatch_ring_batch(
        &mut self,
        client: Ulid,
        frame: Frame<Request>,
    ) -> Result<PendingRingBatch, Failure> {
        if frame.version != VERSION {
            return Err(Failure::new(FailureCode::Version, "unsupported version"));
        }
        if !self.clients.contains_key(&client) {
            return Err(disconnected());
        }
        if frame.body.request_id.is_nil() {
            return Err(Failure::new(FailureCode::Invalid, "nil request id"));
        }
        if self.active.is_none()
            || self.active != frame.body.logic_lease.map(|lease| (client, lease))
        {
            return Err(Failure::new(
                FailureCode::StaleLease,
                "logic lease was replaced or disconnected",
            ));
        }
        let Action::ReadRingBatch { reads, max_bytes } = frame.body.action else {
            unreachable!()
        };
        if reads.is_empty()
            || reads.len() > MAX_RING_BATCH_MEMBERS
            || max_bytes as usize > MAX_RING_BATCH_BYTES
        {
            return Err(Failure::new(
                FailureCode::Invalid,
                "invalid ring batch count or byte budget",
            ));
        }
        let mut identities = std::collections::BTreeSet::new();
        if reads
            .iter()
            .any(|read| !identities.insert((read.attachment.seat, read.attachment.generation)))
        {
            return Err(Failure::new(
                FailureCode::Invalid,
                "duplicate ring batch member",
            ));
        }
        let limits: Vec<_> = reads
            .iter()
            .map(|read| RingReadResult {
                seat: read.attachment.seat,
                generation: read.attachment.generation,
                from: read.from,
                to: read.to,
                result: Err(batch_limit()),
            })
            .collect();
        if !batch_wire_bytes(frame.body.request_id, &limits)
            .is_ok_and(|bytes| bytes <= max_bytes as usize)
        {
            return Err(Failure::new(
                FailureCode::Invalid,
                "batch budget cannot contain member results",
            ));
        }
        let mut credits = CHANNEL_CAPACITY.saturating_sub(self.pending_capture_slots());
        let mut members = Vec::with_capacity(reads.len());
        let mut diagnostic = self.capture_diagnostics.start(
            capture_diagnostics::Phase::SupervisorQuery,
            Ulid::nil(),
            0,
            0,
            0,
        );
        diagnostic.request(frame.body.request_id);
        // request() only admits a query and returns its receiver. No member is
        // awaited until every member has been validated and admitted/refused.
        for (read, mut result) in reads.into_iter().zip(limits) {
            let charged = credits > 0;
            if charged {
                credits -= 1;
            }
            let response = (|| {
                if !charged {
                    return Err(Failure::new(
                        FailureCode::Busy,
                        "too many pending ring requests",
                    ));
                }
                if read.from > read.to {
                    return Err(Failure::new(FailureCode::Invalid, "invalid history range"));
                }
                let seat = self.seat_mut(read.attachment.seat, read.attachment.generation)?;
                seat.check_writer(client, &read.attachment)?;
                seat.ring
                    .as_ref()
                    .ok_or_else(|| Failure::new(FailureCode::NotFound, "no ring configured"))?
                    .request(CaptureQuery::History {
                        from: read.from,
                        to: read.to,
                    })
            })();
            let response = match response {
                Ok(response) => Some(response),
                Err(error) => {
                    result.result = Err(error);
                    None
                }
            };
            members.push(PendingRingRead {
                charged,
                result,
                response,
            });
        }
        Ok(PendingRingBatch {
            client,
            request_id: frame.body.request_id,
            reply: None,
            members,
            max_bytes: max_bytes as usize,
            diagnostic,
        })
    }

    fn complete_io(&mut self) {
        let mut ready = Vec::new();
        for i in (0..self.pending_io.len()).rev() {
            let result = match self.pending_io[i].response.try_recv() {
                Ok(result) => result,
                Err(mpsc::TryRecvError::Empty) => continue,
                Err(mpsc::TryRecvError::Disconnected) => Err(Failure::new(
                    FailureCode::Io,
                    "capture reply unavailable; outcome unknown",
                )),
            };
            ready.push((self.pending_io.swap_remove(i), result));
        }
        for (pending, result) in ready {
            pending.diagnostic.finish(diagnostic_outcome(&result), 0, 0);
            if let Some(reply) = pending.reply {
                let _ = reply.try_send(result);
            } else {
                self.socket_reply(pending.client, pending.request_id, result);
            }
        }
        for index in (0..self.pending_batches.len()).rev() {
            let batch = &mut self.pending_batches[index];
            for member in &mut batch.members {
                let Some(response) = &member.response else {
                    continue;
                };
                let result = match response.try_recv() {
                    Ok(Ok(Reply::RingHistory { history })) => Ok(history),
                    Ok(Ok(_)) => Err(Failure::new(
                        FailureCode::Invalid,
                        "ring batch reply mismatch",
                    )),
                    Ok(Err(error)) => Err(error),
                    Err(mpsc::TryRecvError::Empty) => continue,
                    Err(mpsc::TryRecvError::Disconnected) => Err(Failure::new(
                        FailureCode::Io,
                        "capture reply unavailable; outcome unknown",
                    )),
                };
                member.response = None;
                member.result.result = result;
            }
            if batch.members.iter().any(|member| member.response.is_some()) {
                continue;
            }
            let batch = self.pending_batches.swap_remove(index);
            let result = bounded_batch_reply(
                batch.request_id,
                batch
                    .members
                    .into_iter()
                    .map(|member| member.result)
                    .collect(),
                batch.max_bytes,
            );
            batch.diagnostic.finish(diagnostic_outcome(&result), 0, 0);
            if let Some(reply) = batch.reply {
                let _ = reply.try_send(result);
            } else {
                self.socket_reply(batch.client, batch.request_id, result);
            }
        }
    }

    fn socket_reply(&mut self, id: Ulid, request_id: Ulid, result: Result<Reply, Failure>) {
        let queued = match self.clients.get_mut(&id) {
            Some(Target::Socket { peer, .. }) => {
                match peer.enqueue(Message::Reply { request_id, result }) {
                    Ok(true) => true,
                    Ok(false) => {
                        eprintln!("supervisor reply queue full: request={request_id}");
                        false
                    }
                    Err(error) => {
                        eprintln!(
                            "supervisor reply encoding failed: request={request_id} error={error}"
                        );
                        false
                    }
                }
            }
            _ => false,
        };
        if !queued {
            self.disconnect(id);
        }
    }

    fn dispatch_now(&mut self, client: Ulid, frame: Frame<Request>) -> Result<Reply, Failure> {
        if frame.version != VERSION {
            return Err(Failure::new(FailureCode::Version, "unsupported version"));
        }
        if !self.clients.contains_key(&client) {
            return Err(disconnected());
        }
        let request = frame.body;
        if request.request_id == Ulid::nil() {
            return Err(Failure::new(FailureCode::Invalid, "nil request id"));
        }
        if let Action::AcquireLogic { capability } = request.action {
            // Constant-time comparison of the fixed-width startup capability.
            let difference = capability
                .iter()
                .zip(self.capability)
                .fold(0, |acc, (a, b)| acc | (a ^ b));
            if difference != 0 {
                return Err(Failure::new(
                    FailureCode::Unauthorized,
                    "invalid startup capability",
                ));
            }
            let lease = Ulid::new();
            self.active = Some((client, lease));
            for seat in self.seats.values_mut() {
                seat.writer = None;
            }
            return Ok(Reply::Acquired {
                logic_lease: lease,
                supervisor_boot: self.boot,
            });
        }
        if self.active.is_none() || self.active != request.logic_lease.map(|lease| (client, lease))
        {
            return Err(Failure::new(
                FailureCode::StaleLease,
                "logic lease was replaced or disconnected",
            ));
        }
        match request.action {
            Action::AcquireLogic { .. } => unreachable!(),
            Action::ListOwnedSeats => Ok(Reply::Seats {
                seats: self.seats.values().map(Seat::info).collect(),
            }),
            Action::Spawn { spec } => self.spawn_seat(spec),
            Action::Attach { seat, generation } => {
                let seat = self.seat_mut(seat, generation)?;
                let attachment = Attachment {
                    seat: seat.spec.seat,
                    generation,
                    writer_lease: Ulid::new(),
                };
                seat.writer = Some((client, attachment.writer_lease));
                Ok(Reply::Attached {
                    attachment,
                    info: seat.info(),
                    first_available_sequence: seat
                        .pending_event
                        .as_ref()
                        .and_then(Event::sequence)
                        .unwrap_or(seat.next_seq),
                })
            }
            Action::AttachRing {
                seat,
                generation,
                directory,
                settings,
            } => {
                if self.config.journal_directory.is_none() {
                    return Err(Failure::new(
                        FailureCode::Invalid,
                        "JournalDirectoryRequired: capture requires Config.journal_directory",
                    ));
                }
                let seat_id = seat;
                let seat = self.seat_mut(seat_id, generation)?;
                if !directory.is_absolute() {
                    return Err(Failure::new(
                        FailureCode::Invalid,
                        "ring directory must be absolute",
                    ));
                }
                if let Some(ring) = &seat.ring {
                    if ring.directory() != directory || ring.settings() != &settings {
                        return Err(Failure::new(
                            FailureCode::Invalid,
                            "ring owner/configuration conflicts",
                        ));
                    }
                } else {
                    let ring = crate::ring::Ring::open(
                        &directory,
                        seat.spec.seat,
                        generation,
                        seat.next_seq - 1,
                        settings,
                    )
                    .map_err(pty_error)?;
                    if ring.status().captured_sequence != seat.next_seq - 1 {
                        return Err(Failure::new(
                            FailureCode::StaleGeneration,
                            "retained ring is from an earlier supervisor/process; resume needs a new generation",
                        ));
                    }
                    let handle = self.capture_writer.register(ring);
                    self.seat_mut(seat_id, generation)?.ring = Some(handle);
                }
                let seat = self.seat_mut(seat_id, generation)?;
                let attachment = Attachment {
                    seat: seat.spec.seat,
                    generation,
                    writer_lease: Ulid::new(),
                };
                seat.writer = Some((client, attachment.writer_lease));
                Ok(Reply::Attached {
                    attachment,
                    info: seat.info(),
                    first_available_sequence: seat.ring.as_ref().unwrap().first_sequence(),
                })
            }
            Action::RingRecovery { .. }
            | Action::ReadRing { .. }
            | Action::ReadRingBatch { .. }
            | Action::PublishCheckpoint { .. } => unreachable!("ring IO is deferred"),
            Action::Input {
                attachment,
                input_id,
                bytes,
            } => {
                let timeout = self.config.write_timeout;
                let probe = self.config.input_probes.contains(&bytes);
                let seat = self.seat_mut(attachment.seat, attachment.generation)?;
                seat.check_writer(client, &attachment)?;
                let receipt = seat.accept_input(input_id, bytes, timeout, false, probe)?;
                Ok(Reply::Input { receipt })
            }
            Action::InputContext { attachment } => {
                let seat = self.seat_mut(attachment.seat, attachment.generation)?;
                seat.check_writer(client, &attachment)?;
                seat.check_live()?;
                seat.flush_output().map_err(capture_error)?;
                Ok(Reply::InputContext {
                    context: seat.input_context(),
                })
            }
            Action::GuardedInput {
                attachment,
                input_id,
                bytes,
                expected,
            } => {
                let timeout = self.config.write_timeout;
                let probe = self.config.input_probes.contains(&bytes);
                let seat = self.seat_mut(attachment.seat, attachment.generation)?;
                seat.check_writer(client, &attachment)?;
                seat.check_live()?;
                seat.flush_output().map_err(capture_error)?;
                if !seat.inputs.contains_key(&input_id)
                    && (seat.input_context() != expected
                        || !expected.settled
                        || expected.draft
                        || expected.foreground_pid != Some(seat.pid))
                {
                    return Err(Failure::new(
                        FailureCode::Busy,
                        "guard changed before PTY write; no mutation",
                    ));
                }
                let receipt = seat.accept_input(input_id, bytes, timeout, true, probe)?;
                Ok(Reply::Input { receipt })
            }
            Action::InputStatus {
                seat,
                generation,
                input_id,
            } => {
                let seat = self.seat_mut(seat, generation)?;
                let record = seat
                    .inputs
                    .get(&input_id)
                    .ok_or_else(|| Failure::new(FailureCode::NotFound, "input id not accepted"))?;
                Ok(Reply::Input {
                    receipt: record.receipt.clone(),
                })
            }
            Action::Resize { attachment, size } => {
                size.validate()?;
                let seat = self.seat_mut(attachment.seat, attachment.generation)?;
                seat.check_writer(client, &attachment)?;
                seat.check_live()?;
                if seat.pending_event.is_some() {
                    return Err(Failure::new(
                        FailureCode::Busy,
                        "drain pending records before resize",
                    ));
                }
                if seat.ring.as_ref().is_some_and(|ring| !ring.can_control()) {
                    return Err(Failure::new(
                        FailureCode::Busy,
                        "ring full: checkpoint required before resize",
                    ));
                }
                seat.flush_output().map_err(capture_error)?;
                seat.master
                    .as_ref()
                    .ok_or_else(|| Failure::new(FailureCode::Io, "PTY is closed"))?
                    .resize(pty_size(size))
                    .map_err(pty_error)?;
                seat.size = size;
                let sequence = seat.next_seq;
                let event = Event::Resized {
                    seat: attachment.seat,
                    generation: attachment.generation,
                    seq: sequence,
                    size,
                };
                seat.capture(event).map_err(pty_error)?;
                Ok(Reply::Resized { size, sequence })
            }
            Action::Kill { attachment } => {
                let seat = self.seat_mut(attachment.seat, attachment.generation)?;
                seat.check_writer(client, &attachment)?;
                seat.check_live()?;
                if seat.terminate_at.is_none() {
                    signal(seat.pid, libc::SIGHUP)?;
                    seat.terminate_at = Some(Instant::now() + Duration::from_millis(250));
                }
                Ok(Reply::Terminating)
            }
            Action::ScheduleClose {
                attachment,
                close_id,
                deadline_ms,
            } => {
                let seat = self.seat_mut(attachment.seat, attachment.generation)?;
                seat.check_writer(client, &attachment)?;
                let live = seat.check_live();
                seat.closing.schedule(
                    close_id,
                    deadline_ms,
                    live,
                    crate::store::now_ms(),
                    Instant::now(),
                )
            }
            Action::UndoClose {
                attachment,
                close_id,
            } => {
                let seat = self.seat_mut(attachment.seat, attachment.generation)?;
                seat.check_writer(client, &attachment)?;
                let live = seat.check_live();
                seat.closing.undo(close_id, live, Instant::now())
            }
        }
    }

    fn seat_mut(&mut self, id: Ulid, generation: u64) -> Result<&mut Seat, Failure> {
        let seat = self
            .seats
            .get_mut(&id)
            .ok_or_else(|| Failure::new(FailureCode::NotFound, "seat not owned"))?;
        if seat.spec.generation != generation {
            return Err(Failure::new(
                FailureCode::StaleGeneration,
                "process generation changed",
            ));
        }
        Ok(seat)
    }

    fn spawn_seat(&mut self, spec: LaunchSpec) -> Result<Reply, Failure> {
        spec.validate()?;
        if let Some(old) = self.seats.get(&spec.seat) {
            // The exact retry returns the original child, even after its exit.
            if old.spec == spec {
                return Ok(Reply::Spawned { info: old.info() });
            }
            if spec.generation != old.spec.generation.checked_add(1).unwrap_or(0) {
                return Err(Failure::new(
                    FailureCode::StaleGeneration,
                    "replacement needs the next generation",
                ));
            }
            if !old.exit_published || old.pending_event.is_some() {
                return Err(Failure::new(
                    FailureCode::Busy,
                    "old process or its final records are still live",
                ));
            }
        } else {
            admit_recovered_generation(&self.config, &spec)?;
            if self.seats.len() >= self.config.max_seats {
                return Err(Failure::new(FailureCode::Limit, "machine seat cap reached"));
            }
        }
        let token = self.token();
        let mut seat = Seat::spawn(spec, token, self.poll.registry())?;
        let child_token = self.token();
        let watching = seat
            .watch_child(self.poll.registry(), child_token)
            .map_err(|error| {
                spawn_error(
                    &seat.spec,
                    &format!("watch_child pid={}", seat.pid),
                    seat.master
                        .as_ref()
                        .and_then(|master| master.tty_name())
                        .as_deref(),
                    error,
                )
            })?;
        if watching {
            self.routes
                .insert(child_token, Route::Child(seat.spec.seat));
        }
        #[cfg(target_os = "macos")]
        if let Some(companion) = &mut seat.companion {
            let companion_token = self.token();
            if companion.watch(self.poll.registry(), companion_token)? {
                self.routes
                    .insert(companion_token, Route::Companion(seat.spec.seat));
            }
        }
        let info = seat.info();
        if let Some(old) = self.seats.insert(info.seat, seat) {
            self.routes.remove(&old.token);
            #[cfg(target_os = "macos")]
            if let Some(watch) = &old.child_watch {
                self.routes.remove(&watch.token());
            }
            #[cfg(target_os = "macos")]
            if let Some(watch) = old.companion.as_ref().and_then(|c| c.watch.as_ref()) {
                self.routes.remove(&watch.token());
            }
        }
        self.routes.insert(token, Route::Seat(info.seat));
        Ok(Reply::Spawned { info })
    }

    fn notify(&mut self, client: Ulid, event: &Event) -> bool {
        // A credit may return halfway through a sweep. Keep the first refused
        // seat's turn until it is serviced; later hot seats cannot jump it.
        let eligible = self
            .notification_waiter
            .is_none_or(|waiting| waiting == event.seat());
        let accepted = eligible
            && match self.clients.get_mut(&client) {
                Some(Target::Local(tx)) => match tx.try_send(event.clone()) {
                    Ok(()) => true,
                    Err(mpsc::TrySendError::Full(_)) => false,
                    Err(mpsc::TrySendError::Disconnected(_)) => {
                        self.disconnect(client);
                        false
                    }
                },
                Some(Target::Socket { peer, .. }) => peer
                    .enqueue(Message::Event {
                        event: event.clone(),
                    })
                    .unwrap_or(false),
                None => false,
            };
        if accepted {
            self.last_notified_seat = Some(event.seat());
            self.notification_waiter = None;
        } else if eligible && self.last_notified_seat != Some(event.seat()) {
            // A seat that just used a credit cannot reserve the next one for
            // its own newly captured output ahead of the other pending seats.
            self.notification_waiter = Some(event.seat());
        }
        let generation = match event {
            Event::Output { generation, .. }
            | Event::Resized { generation, .. }
            | Event::Exited { generation, .. } => *generation,
        };
        self.capture_diagnostics
            .seat(event.seat(), generation, |row, _| {
                row.notification_attempts = row.notification_attempts.saturating_add(1);
                row.notification_last_attempt = event.sequence().unwrap();
                if accepted {
                    row.notification_accepted = row.notification_accepted.saturating_add(1);
                    row.notification_last_accepted = event.sequence().unwrap();
                }
            });
        accepted
    }

    fn disconnect(&mut self, id: Ulid) {
        self.pending_batches.retain(|batch| batch.client != id);
        if let Some(Target::Socket { token, mut peer }) = self.clients.remove(&id) {
            let _ = self.poll.registry().deregister(&mut peer.stream);
            self.routes.remove(&token);
        }
        if self.active.is_some_and(|(client, _)| client == id) {
            self.active = None;
            for seat in self.seats.values_mut() {
                seat.writer = None;
            }
        }
    }

    fn accept_sockets(&mut self) -> io::Result<()> {
        loop {
            let accepted = self.socket.as_mut().unwrap().listener.accept();
            let (mut stream, _) = match accepted {
                Ok(pair) => pair,
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => return Ok(()),
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(error),
            };
            if self.clients.len() >= MAX_CLIENTS {
                continue;
            }
            let id = Ulid::new();
            let token = self.token();
            self.poll
                .registry()
                .register(&mut stream, token, Interest::READABLE)?;
            self.clients.insert(
                id,
                Target::Socket {
                    token,
                    peer: SocketPeer::new(stream),
                },
            );
            self.routes.insert(token, Route::Socket(id));
        }
    }

    fn read_socket(&mut self, id: Ulid) {
        // Drain to WouldBlock; decoder retains partial headers/payloads.
        loop {
            let mut bytes = [0; 8192];
            let frames = match self.clients.get_mut(&id) {
                Some(Target::Socket { peer, .. }) => match peer.stream.read(&mut bytes) {
                    Ok(0) => {
                        self.disconnect(id);
                        return;
                    }
                    Ok(count) => peer.decoder.push(&bytes[..count]),
                    Err(error) if error.kind() == io::ErrorKind::WouldBlock => return,
                    Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                    Err(error) => Err(error),
                },
                _ => return,
            };
            let frames = match frames {
                Ok(frames) => frames,
                Err(_) => {
                    self.disconnect(id);
                    return;
                }
            };
            for frame in frames {
                let request_id = frame.body.request_id;
                match self.dispatch(id, frame) {
                    Dispatch::Ready(result) => self.socket_reply(id, request_id, result),
                    Dispatch::Pending(response, diagnostic) => self.pending_io.push(PendingIo {
                        client: id,
                        request_id,
                        reply: None,
                        response,
                        diagnostic,
                    }),
                    Dispatch::Batch(batch) => self.pending_batches.push(batch),
                }
                if !self.clients.contains_key(&id) {
                    return;
                }
            }
        }
    }

    fn flush_sockets(&mut self) {
        let mut failed = Vec::new();
        for (&id, target) in &mut self.clients {
            if let Target::Socket { token, peer } = target {
                let result = peer.flush().and_then(|()| {
                    let interest = if peer.wants_write() {
                        Interest::READABLE | Interest::WRITABLE
                    } else {
                        Interest::READABLE
                    };
                    self.poll
                        .registry()
                        .reregister(&mut peer.stream, *token, interest)
                });
                if result.is_err() {
                    failed.push(id);
                }
            }
        }
        for id in failed {
            self.disconnect(id);
        }
    }
}

struct StoredInput {
    digest: [u8; 32],
    receipt: InputReceipt,
}

struct PendingWrite {
    id: Ulid,
    bytes: Vec<u8>,
    deadline: Instant,
    guard: Option<crate::protocol::InputContext>,
}

struct Seat {
    spec: LaunchSpec,
    size: SeatSize,
    master: Option<Box<dyn MasterPty + Send>>,
    child: Box<dyn Child + Send + Sync>,
    companion: Option<companion::Companion>,
    #[cfg(target_os = "macos")]
    child_watch: Option<child_events::Watch>,
    #[cfg(target_os = "macos")]
    reap_pending: bool,
    pid: u32,
    fd: RawFd,
    token: Token,
    registered: bool,
    writable_interest: bool,
    read_ready: bool,
    write_ready: bool,
    eof: bool,
    next_seq: u64,
    writer: Option<(Ulid, Ulid)>,
    pending_event: Option<Event>,
    output: Vec<u8>,
    output_since: Option<Instant>,
    pending_write: Option<PendingWrite>,
    inputs: HashMap<Ulid, StoredInput>,
    input_epoch: u64,
    last_input_ms: u64,
    operator_draft: bool,
    exited: Option<ExitInfo>,
    #[cfg(not(target_os = "macos"))]
    next_child_check: Instant,
    exit_published: bool,
    terminate_at: Option<Instant>,
    termination_escalated: bool,
    closing: CloseState,
    ring: Option<CaptureHandle>,
}

impl Seat {
    fn spawn(spec: LaunchSpec, token: Token, registry: &mio::Registry) -> Result<Self, Failure> {
        let companion = companion::Companion::start(&spec)?;
        let pair = native_pty_system()
            .openpty(pty_size(spec.size))
            .map_err(|error| spawn_error(&spec, "openpty", None, error))?;
        let tty = pair.master.tty_name();
        let fd = pair
            .master
            .as_raw_fd()
            .ok_or_else(|| Failure::new(FailureCode::Io, "PTY has no raw fd"))?;
        set_nonblocking(fd)
            .map_err(|error| spawn_error(&spec, "nonblocking", tty.as_deref(), error))?;
        registry
            .register(&mut SourceFd(&fd), token, Interest::READABLE)
            .map_err(|error| spawn_error(&spec, "poll register", tty.as_deref(), error))?;
        let mut command = CommandBuilder::new(&spec.program);
        command.args(&spec.args);
        command.cwd(&spec.cwd);
        command.env("TERM", "xterm-256color");
        for (key, value) in &spec.env {
            command.env(key, value);
        }
        command.env("BBOXY_SEAT_ID", spec.seat.to_string());
        command.env("BBOXY_PROCESS_GENERATION", spec.generation.to_string());
        let child = pair
            .slave
            .spawn_command(command)
            .map_err(|error| spawn_error(&spec, "spawn_command", tty.as_deref(), error))?;
        drop(pair.slave);
        let pid = child
            .process_id()
            .ok_or_else(|| Failure::new(FailureCode::Io, "child has no pid"))?;
        Ok(Self {
            size: spec.size,
            spec,
            master: Some(pair.master),
            child,
            companion,
            #[cfg(target_os = "macos")]
            child_watch: None,
            #[cfg(target_os = "macos")]
            reap_pending: false,
            pid,
            fd,
            token,
            registered: true,
            writable_interest: false,
            read_ready: false,
            write_ready: true,
            eof: false,
            next_seq: 1,
            writer: None,
            pending_event: None,
            output: Vec::with_capacity(OUTPUT_CHUNK_BYTES),
            output_since: None,
            pending_write: None,
            inputs: HashMap::new(),
            input_epoch: 0,
            last_input_ms: 0,
            operator_draft: false,
            exited: None,
            #[cfg(not(target_os = "macos"))]
            next_child_check: Instant::now(),
            exit_published: false,
            terminate_at: None,
            termination_escalated: false,
            closing: CloseState::default(),
            ring: None,
        })
    }

    fn info(&self) -> SeatInfo {
        SeatInfo {
            seat: self.spec.seat,
            generation: self.spec.generation,
            pid: self.pid,
            size: self.size,
            next_sequence: self.next_seq,
            exited: self.exited.clone(),
            companion: self.companion.as_ref().map(companion::Companion::info),
        }
    }

    #[cfg(target_os = "macos")]
    fn watch_child(&mut self, registry: &mio::Registry, token: Token) -> io::Result<bool> {
        self.child_watch = child_events::Watch::register(registry, self.pid, token)?;
        if self.child_watch.is_none() {
            // The exited child remains unreaped, so its PID cannot be reused.
            // It follows the same output-drain/one-Exit path as NOTE_EXIT.
            self.reap_child()?;
        }
        Ok(self.child_watch.is_some())
    }

    #[cfg(not(target_os = "macos"))]
    fn watch_child(&mut self, _registry: &mio::Registry, _token: Token) -> io::Result<bool> {
        Ok(false)
    }

    fn complete_child_watch(&mut self) {
        #[cfg(target_os = "macos")]
        if let Some(watch) = self.child_watch.take() {
            watch.completed();
        }
    }

    fn reap_child(&mut self) -> io::Result<()> {
        if self.exited.is_none() {
            // Called only after NOTE_EXIT or EV_ADD's ESRCH, never on PTY EOF.
            // An exiting process can be unattachable before waitpid is ready,
            // e.g. while tty output drains. Never block that drain in wait().
            #[cfg(target_os = "macos")]
            {
                self.reap_pending = true;
                if let Some(status) = self.child.try_wait()? {
                    self.exited = Some(ExitInfo {
                        code: status.exit_code(),
                        signal: status.signal().map(str::to_owned),
                    });
                    self.reap_pending = false;
                }
            }
            self.read_ready = true;
        }
        Ok(())
    }

    fn sequence(&mut self) -> u64 {
        let seq = self.next_seq;
        self.next_seq = self
            .next_seq
            .checked_add(1)
            .expect("seat sequence exhausted");
        seq
    }

    fn check_live(&self) -> Result<(), Failure> {
        if self.exited.is_some() || self.eof || self.terminate_at.is_some() {
            return Err(Failure::new(
                FailureCode::Exited,
                "child exited or termination is pending",
            ));
        }
        Ok(())
    }

    fn check_writer(&self, client: Ulid, attachment: &Attachment) -> Result<(), Failure> {
        if self.writer != Some((client, attachment.writer_lease)) {
            return Err(Failure::new(
                FailureCode::StaleWriter,
                "writer attachment was replaced",
            ));
        }
        Ok(())
    }

    fn accept_input(
        &mut self,
        id: Ulid,
        bytes: Vec<u8>,
        timeout: Duration,
        guarded: bool,
        probe: bool,
    ) -> Result<InputReceipt, Failure> {
        if id == Ulid::nil() || bytes.is_empty() || bytes.len() > MAX_INPUT_BYTES {
            return Err(Failure::new(
                FailureCode::Invalid,
                "input needs a nonnil id and 1..65536 bytes",
            ));
        }
        let digest: [u8; 32] = Sha256::digest(&bytes).into();
        if let Some(stored) = self.inputs.get(&id) {
            if stored.digest != digest {
                return Err(Failure::new(
                    FailureCode::InputConflict,
                    "input id reused for different bytes",
                ));
            }
            return Ok(stored.receipt.clone());
        }
        self.check_live()?;
        if self.pending_write.is_some() {
            return Err(Failure::new(
                FailureCode::Busy,
                "a previous input is still in flight",
            ));
        }
        if self.inputs.len() >= MAX_INPUT_IDS {
            return Err(Failure::new(
                FailureCode::Limit,
                "input deduplication capacity reached; ids are never silently evicted",
            ));
        }
        let receipt = InputReceipt {
            input_id: id,
            state: InputState::Pending,
            bytes_written: 0,
            bytes_total: bytes.len(),
        };
        // Only exact caller-configured probes leave the observed input context
        // intact. Receipt, deduplication and guarded-write rules still apply.
        if !probe {
            self.input_epoch = self
                .input_epoch
                .checked_add(1)
                .ok_or_else(|| Failure::new(FailureCode::Limit, "input epoch exhausted"))?;
            self.last_input_ms = crate::store::now_ms();
            self.operator_draft = !matches!(bytes.last(), Some(b'\r' | b'\n' | 3));
        }
        self.inputs.insert(id, StoredInput { digest, receipt });
        let guard = guarded.then(|| self.input_context());
        self.pending_write = Some(PendingWrite {
            id,
            bytes,
            deadline: Instant::now() + timeout,
            guard,
        });
        self.write_ready = true;
        self.write_pending();
        Ok(self.inputs[&id].receipt.clone())
    }

    fn input_context(&self) -> crate::protocol::InputContext {
        let mut attrs = std::mem::MaybeUninit::<libc::termios>::uninit();
        // SAFETY: this actor owns the PTY master; tcgetattr initializes attrs
        // only on success and tcgetpgrp takes no pointer.
        let valid = unsafe { libc::tcgetattr(self.fd, attrs.as_mut_ptr()) } == 0;
        let flags = if valid {
            unsafe { attrs.assume_init() }.c_lflag
        } else {
            0
        };
        let group = unsafe { libc::tcgetpgrp(self.fd) };
        crate::protocol::InputContext {
            epoch: self.input_epoch,
            last_input_ms: self.last_input_ms,
            sequence: self.next_seq - 1,
            draft: self.operator_draft,
            foreground_pid: u32::try_from(group).ok().filter(|p| *p > 0),
            canonical: valid && flags & libc::ICANON != 0,
            echo: valid && flags & libc::ECHO != 0,
            settled: self.output.is_empty() && !self.read_ready && self.pending_event.is_none(),
        }
    }

    fn write_pending(&mut self) {
        let Some(pending) = self.pending_write.as_ref() else {
            return;
        };
        if pending
            .guard
            .as_ref()
            .is_some_and(|g| g != &self.input_context())
            && self.inputs[&pending.id].receipt.bytes_written == 0
        {
            self.inputs.get_mut(&pending.id).unwrap().receipt.state = InputState::PartialUnknown;
            self.pending_write = None;
            return;
        }
        let stored = self.inputs.get_mut(&pending.id).unwrap();
        if self.exited.is_some() || self.eof || Instant::now() >= pending.deadline {
            stored.receipt.state = InputState::PartialUnknown;
            self.pending_write = None;
            return;
        }
        if !self.write_ready {
            return;
        }
        while stored.receipt.bytes_written < pending.bytes.len() {
            let remaining = &pending.bytes[stored.receipt.bytes_written..];
            // SAFETY: master owns fd for this call; remaining is a valid slice.
            let count = unsafe { libc::write(self.fd, remaining.as_ptr().cast(), remaining.len()) };
            if count > 0 {
                stored.receipt.bytes_written += count as usize;
                continue;
            }
            let error = io::Error::last_os_error();
            if count < 0 && error.kind() == io::ErrorKind::Interrupted {
                continue;
            }
            if count < 0 && error.kind() == io::ErrorKind::WouldBlock {
                self.write_ready = false;
                return;
            }
            stored.receipt.state = InputState::PartialUnknown;
            self.pending_write = None;
            return;
        }
        stored.receipt.state = InputState::PtyWritten;
        self.pending_write = None;
    }

    fn publish_pending(&mut self, actor: &mut Actor) {
        if let Some((client, _)) = self.writer
            && actor.active.is_some_and(|(active, _)| active == client)
            && let Some(event) = self.pending_event.as_ref()
            && actor.notify(client, event)
        {
            self.pending_event = None;
        }
        if self.ring.is_some() && self.writer.is_none() {
            // Already committed; the replacement logic replays from the ring.
            self.pending_event = None;
        }
    }

    fn capture(&mut self, event: Event) -> io::Result<()> {
        if let Some(ring) = self.ring.as_mut() {
            ring.append_event(&event)?;
        }
        self.sequence();
        self.pending_event = Some(event);
        Ok(())
    }

    fn flush_output(&mut self) -> io::Result<()> {
        if self.output.is_empty() {
            return Ok(());
        }
        let bytes = std::mem::replace(&mut self.output, Vec::with_capacity(OUTPUT_CHUNK_BYTES));
        let event = Event::Output {
            seat: self.spec.seat,
            generation: self.spec.generation,
            seq: self.next_seq,
            bytes,
        };
        if let Some(ring) = self.ring.as_mut()
            && let Err(error) = ring.append_event(&event)
        {
            let Event::Output { bytes, .. } = event else {
                unreachable!()
            };
            self.output = bytes;
            return Err(error);
        }
        self.output_since = None;
        self.sequence();
        self.pending_event = Some(event);
        Ok(())
    }

    fn tick(
        &mut self,
        actor: &mut Actor,
        read_buffer: &mut [u8; OUTPUT_CHUNK_BYTES],
    ) -> io::Result<()> {
        if self.exited.is_none()
            && self.terminate_at.is_none()
            && self.closing.expired(Instant::now())
        {
            signal(self.pid, libc::SIGHUP).map_err(io::Error::other)?;
            self.terminate_at = Some(Instant::now() + Duration::from_millis(250));
        }
        if self
            .writer
            .is_some_and(|(client, _)| !actor.active.is_some_and(|(active, _)| active == client))
        {
            self.writer = None;
        }
        #[cfg(target_os = "macos")]
        if self.reap_pending {
            // Only an already-notified exit may need a later waitpid-ready
            // turn. Running seats never enter this path or poll child status.
            self.reap_child()?;
        }
        #[cfg(target_os = "macos")]
        if self.exited.is_none()
            && !self.termination_escalated
            && self.terminate_at.is_some_and(|at| Instant::now() >= at)
        {
            signal(self.pid, libc::SIGKILL)?;
            self.termination_escalated = true;
        }
        #[cfg(not(target_os = "macos"))]
        {
            let now = Instant::now();
            let escalate =
                !self.termination_escalated && self.terminate_at.is_some_and(|at| now >= at);
            if self.exited.is_none() && (now >= self.next_child_check || escalate) {
                self.next_child_check = now + CHILD_POLL_INTERVAL;
                if let Some(status) = self.child.try_wait()? {
                    self.exited = Some(ExitInfo {
                        code: status.exit_code(),
                        signal: status.signal().map(str::to_owned),
                    });
                    self.read_ready = true;
                } else if escalate {
                    signal(self.pid, libc::SIGKILL)?;
                    self.termination_escalated = true;
                }
            }
        }
        if let Some(companion) = &mut self.companion {
            if self.exited.is_some() {
                companion.stop();
            }
            #[cfg(not(target_os = "macos"))]
            {
                companion.notified = true;
            }
            if companion.poll_exit()? && self.exited.is_none() && self.terminate_at.is_none() {
                signal(self.pid, libc::SIGHUP)?;
                self.terminate_at = Some(Instant::now() + Duration::from_millis(250));
            }
        }
        self.write_pending();
        self.publish_pending(actor);
        // A configured ring keeps capturing independently of logic delivery.
        // One latest notification survives a full queue; skipped sequences are
        // hydrated by logic. The legacy ring-less path retains its old bound.
        for _ in 0..4 {
            if !self.read_ready
                || (self.ring.is_none() && self.pending_event.is_some())
                || self
                    .ring
                    .as_ref()
                    .map_or(self.writer.is_none(), |ring| !ring.can_capture())
                || self.exit_published
            {
                break;
            }
            let capacity = self
                .ring
                .as_ref()
                .map_or(OUTPUT_CHUNK_BYTES, CaptureHandle::capture_capacity)
                - self.output.len();
            if capacity == 0 {
                // A full frame can survive a WouldBlock admission race. A
                // read(fd, _, 0) would return zero without real PTY EOF.
                if let Err(error) = self.flush_output() {
                    if error.kind() == io::ErrorKind::WouldBlock {
                        return Ok(());
                    }
                    return Err(error);
                }
                self.publish_pending(actor);
                continue;
            }
            // Darwin commonly returns 1 KiB per PTY read. Coalesce currently
            // available bytes into one bounded record, without waiting for more.
            let mut count = 0;
            for _ in 0..64 {
                // SAFETY: master owns fd; the remaining slice is writable.
                let read = unsafe {
                    libc::read(
                        self.fd,
                        read_buffer[count..].as_mut_ptr().cast(),
                        capacity - count,
                    )
                };
                if read > 0 {
                    count += read as usize;
                    if count == capacity {
                        break;
                    }
                    continue;
                }
                let error = io::Error::last_os_error();
                if read < 0 && error.kind() == io::ErrorKind::Interrupted {
                    continue;
                }
                self.read_ready = false;
                if read == 0 || error.raw_os_error() == Some(libc::EIO) {
                    self.eof = true;
                    #[cfg(not(target_os = "macos"))]
                    {
                        self.next_child_check = Instant::now();
                    }
                } else if error.kind() != io::ErrorKind::WouldBlock {
                    return Err(error);
                }
                break;
            }
            if count > 0 {
                self.output.extend_from_slice(&read_buffer[..count]);
                self.output_since.get_or_insert_with(Instant::now);
                if count == capacity || self.ring.is_none() {
                    if let Err(error) = self.flush_output() {
                        if error.kind() == io::ErrorKind::WouldBlock {
                            return Ok(());
                        }
                        return Err(error);
                    }
                    self.publish_pending(actor);
                }
                continue;
            }
            break;
        }
        if self
            .output_since
            .is_some_and(|at| at.elapsed() >= OUTPUT_DELAY || self.eof || self.exited.is_some())
            && self.ring.as_ref().is_none_or(CaptureHandle::can_capture)
        {
            if let Err(error) = self.flush_output() {
                if error.kind() == io::ErrorKind::WouldBlock {
                    return Ok(());
                }
                return Err(error);
            }
            self.publish_pending(actor);
        }
        if let Some(status) = self.exited.clone()
            && self
                .companion
                .as_ref()
                .is_none_or(companion::Companion::reaped)
            && !self.read_ready
            && (self.ring.is_some() || self.pending_event.is_none())
            && !self.exit_published
            && self.output.is_empty()
            && self.ring.as_ref().is_none_or(CaptureHandle::can_control)
        {
            let seq = self.next_seq;
            self.capture(Event::Exited {
                seat: self.spec.seat,
                generation: self.spec.generation,
                seq,
                status,
            })?;
            self.exit_published = true;
            self.publish_pending(actor);
        }
        if self.eof || self.exit_published {
            if self.registered {
                actor.poll.registry().deregister(&mut SourceFd(&self.fd))?;
                self.registered = false;
            }
        } else if self.registered {
            update_pty_write_interest(
                actor.poll.registry(),
                self.fd,
                self.token,
                &mut self.writable_interest,
                self.pending_write.is_some(),
            )?;
        }
        Ok(())
    }
}

impl Drop for Seat {
    fn drop(&mut self) {
        #[cfg(target_os = "macos")]
        drop(self.child_watch.take());
        drop(self.companion.take());
        if self.ring.as_ref().is_none_or(CaptureHandle::can_capture) {
            let _ = self.flush_output();
        }
        #[cfg(target_os = "macos")]
        {
            if self.exited.is_none() {
                // portable-pty creates an owned session/process-group leader.
                // Signal its group as well as the exact unreaped child; CONT
                // also releases a staged/stopped child with KILL pending.
                // SAFETY: this unreaped PID cannot be reused by another owner.
                unsafe {
                    libc::kill(-(self.pid as libc::pid_t), libc::SIGKILL);
                    libc::kill(-(self.pid as libc::pid_t), libc::SIGCONT);
                }
                let _ = signal(self.pid, libc::SIGKILL);
                let _ = signal(self.pid, libc::SIGCONT);
            }
            // A child can be exiting while tty output drains. Close the one
            // owning master before waiting, after flushing captured bytes.
            drop(self.master.take());
            if self.exited.is_none() {
                let until = Instant::now() + Duration::from_millis(250);
                let result = child_events::exited_within(self.pid, Duration::from_millis(250))
                    .and_then(|ready| {
                        loop {
                            let status = self.child.try_wait()?;
                            if status.is_some() || !ready || Instant::now() >= until {
                                return Ok(status);
                            }
                            // NOTE_EXIT/ESRCH may precede a waitable zombie. This
                            // shutdown-only retry remains inside the same bound.
                            std::thread::sleep(Duration::from_millis(1));
                        }
                    });
                match result {
                    Ok(Some(status)) => {
                        self.exited = Some(ExitInfo {
                            code: status.exit_code(),
                            signal: status.signal().map(str::to_owned),
                        });
                    }
                    result => eprintln!(
                        "supervisor child shutdown unconfirmed: seat={} generation={} pid={} result={result:?}",
                        self.spec.seat, self.spec.generation, self.pid
                    ),
                }
            }
            if let Some(status) = self.exited.clone()
                && !self.exit_published
                && self.output.is_empty()
                && self.ring.as_ref().is_some_and(CaptureHandle::can_control)
            {
                let event = Event::Exited {
                    seat: self.spec.seat,
                    generation: self.spec.generation,
                    seq: self.next_seq,
                    status,
                };
                if let Err(error) = self.capture(event) {
                    eprintln!("supervisor shutdown exit capture: {error}");
                }
            }
        }
        #[cfg(not(target_os = "macos"))]
        if self.exited.is_none() {
            let _ = signal(self.pid, libc::SIGKILL);
            let _ = self.child.wait();
        }
    }
}

fn update_pty_write_interest(
    registry: &mio::Registry,
    fd: RawFd,
    token: Token,
    current: &mut bool,
    writable: bool,
) -> io::Result<()> {
    // Re-registering a kqueue filter re-arms unread bytes. A backpressured
    // seat retains read_ready until credit returns; it needs no new poll event.
    if *current == writable {
        return Ok(());
    }
    registry.reregister(
        &mut SourceFd(&fd),
        token,
        if writable {
            Interest::READABLE | Interest::WRITABLE
        } else {
            Interest::READABLE
        },
    )?;
    *current = writable;
    Ok(())
}

fn pty_size(size: SeatSize) -> PtySize {
    PtySize {
        rows: size.rows,
        cols: size.cols,
        pixel_width: size.pixel_width,
        pixel_height: size.pixel_height,
    }
}

fn capture_error(error: io::Error) -> Failure {
    if error.kind() == io::ErrorKind::WouldBlock {
        Failure::new(FailureCode::Busy, "capture admission paused")
    } else {
        pty_error(error)
    }
}
fn pty_error(error: impl std::fmt::Display) -> Failure {
    Failure::new(FailureCode::Io, error.to_string())
}

fn spawn_error(
    spec: &LaunchSpec,
    stage: &str,
    tty: Option<&std::path::Path>,
    error: impl std::fmt::Display,
) -> Failure {
    // These observations describe the failed attempt; they are not preflight
    // admission or a retry. Never include argv or the child environment.
    let path = |path: &std::path::Path| match std::fs::metadata(path) {
        Ok(_) => format!("{} [present]", path.display()),
        Err(error) => format!("{} [{error}]", path.display()),
    };
    let program = std::path::Path::new(&spec.program);
    let program = if program.is_absolute() {
        path(program)
    } else {
        format!("{} [child PATH lookup]", program.display())
    };
    Failure::new(
        FailureCode::Io,
        format!(
            "spawn {stage}: program={program}; cwd={}; pty={}: {error:#}",
            path(&spec.cwd),
            tty.map_or_else(|| "not allocated".into(), path),
        ),
    )
}

fn disconnected() -> Failure {
    Failure::new(FailureCode::Io, "supervisor connection unavailable")
}

pub(crate) fn set_nonblocking(fd: RawFd) -> io::Result<()> {
    // SAFETY: callers keep the owning PTY/socket alive. F_GETFL/F_SETFL operate
    // on this descriptor only; no pointers or borrowed ownership are created.
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if flags < 0 || unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

fn signal(pid: u32, signal: libc::c_int) -> io::Result<()> {
    // SAFETY: pid comes from this actor's unreaped child, never from a request.
    if unsafe { libc::kill(pid as libc::pid_t, signal) } < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

// A newly started supervisor has no in-memory generation history. Only an
// existing, validated predecessor ring can authorize a higher first generation.
// Do not use object_dir or open a ring before finding its durable marker:
// Ring::open initializes an empty directory, which cannot prove a predecessor.
fn admit_recovered_generation(config: &Config, spec: &LaunchSpec) -> Result<(), Failure> {
    let stale = |reason| Failure::new(FailureCode::StaleGeneration, reason);
    let Some(root) = config.socket_path.as_deref().and_then(|path| path.parent()) else {
        return if spec.generation == 1 {
            Ok(())
        } else {
            Err(stale("recovery requires a persistent supervisor root"))
        };
    };
    let directory = root.join(format!("seat-{}", spec.seat));
    match std::fs::symlink_metadata(directory.join(format!("ring-{}", spec.generation))) {
        Ok(_) => return Err(stale("requested process generation already has a ring")),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(_) => return Err(stale("requested process generation cannot be inspected")),
    }
    if spec.generation == 1 {
        return Ok(());
    }
    let previous = spec.generation - 1;
    let predecessor = directory.join(format!("ring-{previous}"));
    #[derive(serde::Deserialize)]
    struct MarkerSettings {
        settings: crate::ring::Settings,
    }
    for name in ["committed", "previous"] {
        let Ok(file) = std::fs::File::open(predecessor.join(name)) else {
            continue;
        };
        let mut bytes = Vec::new();
        if file.take(4097).read_to_end(&mut bytes).is_err() || bytes.len() < 4 || bytes.len() > 4096
        {
            continue;
        }
        let end = bytes.len() - 4;
        let crc = u32::from_le_bytes(bytes[end..].try_into().unwrap());
        if crate::ring::crc32c(&bytes[..end]) != crc {
            continue;
        }
        let Ok(marker) = serde_json::from_slice::<MarkerSettings>(&bytes[..end]) else {
            continue;
        };
        // The old supervisor has been reaped. The existing ring reader checks
        // identity, settings and record CRCs, and may repair its committed tail.
        if crate::ring::Ring::open(&predecessor, spec.seat, previous, 0, marker.settings).is_ok() {
            return Ok(());
        }
    }
    Err(stale(
        "no valid committed predecessor for process generation",
    ))
}

#[cfg(test)]
mod capture_read_tests {
    use super::*;

    // keeps: invariant seats-967-frame — held output must not manufacture poll turns
    #[test]
    fn unchanged_poll_interests_do_not_rearm_a_held_source() {
        use std::io::{Read, Write};
        use std::os::fd::AsRawFd;
        use std::os::unix::net::UnixStream;

        let (mut source, mut peer) = UnixStream::pair().unwrap();
        source.set_nonblocking(true).unwrap();
        peer.set_nonblocking(true).unwrap();
        let mut poll = Poll::new().unwrap();
        let mut events = Events::with_capacity(8);
        let token = Token(2);
        let fd = source.as_raw_fd();
        poll.registry()
            .register(&mut SourceFd(&fd), token, Interest::READABLE)
            .unwrap();
        let mut writable = false;
        let poll_now = |poll: &mut Poll, events: &mut Events| loop {
            match poll.poll(events, Some(Duration::ZERO)) {
                Ok(()) => break,
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) => panic!("poll failed: {error}"),
            }
        };

        peer.write_all(b"held").unwrap();
        poll_now(&mut poll, &mut events);
        assert!(
            events
                .iter()
                .any(|event| event.token() == token && event.is_readable())
        );
        // Model capture backpressure: leave the already observed byte unread.
        update_pty_write_interest(poll.registry(), fd, token, &mut writable, false).unwrap();
        poll_now(&mut poll, &mut events);
        assert!(
            events.iter().next().is_none(),
            "unchanged interest rearmed bytes held under backpressure"
        );
        let mut bytes = [0; 4];
        source.read_exact(&mut bytes).unwrap();
        assert_eq!(&bytes, b"held");

        peer.write_all(b"next").unwrap();
        poll_now(&mut poll, &mut events);
        assert!(events.iter().any(|event| event.is_readable()));
        source.read_exact(&mut bytes).unwrap();
        assert_eq!(&bytes, b"next");

        update_pty_write_interest(poll.registry(), fd, token, &mut writable, true).unwrap();
        poll_now(&mut poll, &mut events);
        assert!(events.iter().any(|event| event.is_writable()));
        update_pty_write_interest(poll.registry(), fd, token, &mut writable, true).unwrap();
        poll_now(&mut poll, &mut events);
        assert!(events.iter().next().is_none());

        update_pty_write_interest(poll.registry(), fd, token, &mut writable, false).unwrap();
        poll_now(&mut poll, &mut events);
        assert!(events.iter().next().is_none());
        peer.write_all(b"last").unwrap();
        poll_now(&mut poll, &mut events);
        assert!(
            events
                .iter()
                .all(|event| event.is_readable() && !event.is_writable())
        );
        assert!(events.iter().next().is_some());
        source.read_exact(&mut bytes).unwrap();
        assert_eq!(&bytes, b"last");
    }

    fn batch_row(id: u128, bytes: Vec<u8>) -> RingReadResult {
        let seat = Ulid::from(id);
        let end = bytes.len() as u64;
        RingReadResult {
            seat,
            generation: 1,
            from: 1,
            to: 1,
            result: Ok(Box::new(crate::ring::History {
                status: crate::ring::Status {
                    generation: 1,
                    first_sequence: 1,
                    committed_sequence: 1,
                    captured_sequence: 1,
                    byte_start: 0,
                    byte_end: end,
                    hot_bytes: end,
                    paused: false,
                    recovery_notice: None,
                    capture: Default::default(),
                },
                records: vec![crate::ring::IndexedEvent {
                    event: Event::Output {
                        seat,
                        generation: 1,
                        seq: 1,
                        bytes,
                    },
                    row: None,
                }],
                gap: None,
                next: None,
            })),
        }
    }

    // keeps: invariant ring-reads-batched
    #[test]
    fn ring_batch_byte_limit_refuses_only_the_member_that_does_not_fit() {
        let first = batch_row(1, b"first".to_vec());
        let oversized = batch_row(2, vec![255; OUTPUT_CHUNK_BYTES]);
        let last = batch_row(3, b"last".to_vec());
        let expected_first = first.clone();
        let expected_last = last.clone();
        let id = Ulid::new();
        let mut refused = oversized.clone();
        refused.result = Err(batch_limit());
        let budget = crate::transport::encode(&Frame::new(Message::Reply {
            request_id: id,
            result: Ok(Reply::RingHistoryBatch {
                results: vec![first.clone(), refused, last.clone()],
            }),
        }))
        .unwrap()
        .len();
        let Reply::RingHistoryBatch { results } =
            bounded_batch_reply(id, vec![first, oversized, last], budget).unwrap()
        else {
            panic!("batch reply required")
        };
        assert_eq!(results[0], expected_first);
        assert_eq!(results[2], expected_last);
        assert_eq!((results[1].from, results[1].to), (1, 1));
        assert_eq!(
            results[1].result.as_ref().unwrap_err().code,
            FailureCode::Limit
        );
        assert!(batch_wire_bytes(id, &results).unwrap() <= budget);
        assert!(bounded_batch_reply(id, results, 1).is_err());
    }

    // keeps: invariant ring-reads-batched
    #[test]
    fn ring_batch_collects_independent_results_and_reserves_every_leaf_until_done() {
        let root = tempfile::tempdir().unwrap();
        let mut actor = fixture_actor(root.path());
        let client = Ulid::new();
        let id = Ulid::new();
        let good = batch_row(1, b"good".to_vec());
        let expected = good.clone();
        let (good_tx, good_rx) = mpsc::sync_channel(1);
        good_tx
            .send(Ok(Reply::RingHistory {
                history: good.result.clone().unwrap(),
            }))
            .unwrap();
        let (last_tx, last_rx) = mpsc::sync_channel(1);
        let mut gone = batch_row(2, vec![]);
        gone.result = Err(Failure::new(FailureCode::NotFound, "seat gone"));
        let (tx, rx) = mpsc::sync_channel(1);
        actor.pending_batches.push(PendingRingBatch {
            client,
            request_id: id,
            reply: Some(tx),
            max_bytes: MAX_RING_BATCH_BYTES,
            diagnostic: actor.capture_diagnostics.start(
                capture_diagnostics::Phase::SupervisorQuery,
                Ulid::nil(),
                0,
                0,
                0,
            ),
            members: vec![
                PendingRingRead {
                    charged: true,
                    result: good,
                    response: Some(good_rx),
                },
                PendingRingRead {
                    charged: true,
                    result: gone,
                    response: None,
                },
                PendingRingRead {
                    charged: true,
                    result: batch_row(3, vec![]),
                    response: Some(last_rx),
                },
            ],
        });
        actor.complete_io();
        assert!(matches!(rx.try_recv(), Err(mpsc::TryRecvError::Empty)));
        last_tx
            .send(Err(Failure::new(FailureCode::Busy, "capture queue full")))
            .unwrap();
        actor.complete_io();
        let Reply::RingHistoryBatch { results } = rx.try_recv().unwrap().unwrap() else {
            panic!()
        };
        assert_eq!(results[0], expected);
        assert_eq!(
            results[1].result.as_ref().unwrap_err().code,
            FailureCode::NotFound
        );
        assert_eq!(
            results[2].result.as_ref().unwrap_err().code,
            FailureCode::Busy
        );
        assert!(rx.try_recv().is_err(), "one aggregate reply only");
    }

    // keeps: invariant ring-reads-batched
    #[test]
    fn ring_batch_admission_validates_structure_and_keeps_gone_members_separate() {
        let root = tempfile::tempdir().unwrap();
        let mut actor = fixture_actor(root.path());
        let client = Ulid::new();
        let lease = Ulid::new();
        let (tx, _rx) = mpsc::sync_channel(1);
        actor.clients.insert(client, Target::Local(tx));
        actor.active = Some((client, lease));
        let reads: Vec<_> = (1..=MAX_RING_BATCH_MEMBERS)
            .map(|n| RingRead {
                attachment: Attachment {
                    seat: Ulid::from(n as u128),
                    generation: 1,
                    writer_lease: Ulid::new(),
                },
                from: 1,
                to: 2,
            })
            .collect();
        let frame = |reads, max_bytes| {
            Frame::new(Request::new(
                Some(lease),
                Action::ReadRingBatch { reads, max_bytes },
            ))
        };
        assert!(
            actor
                .dispatch_ring_batch(client, frame(vec![], MAX_RING_BATCH_BYTES as u32))
                .is_err()
        );
        assert!(
            actor
                .dispatch_ring_batch(client, frame(reads.clone(), 1))
                .is_err()
        );
        let mut duplicate = reads.clone();
        duplicate[1] = duplicate[0].clone();
        assert!(
            actor
                .dispatch_ring_batch(client, frame(duplicate, MAX_RING_BATCH_BYTES as u32))
                .is_err()
        );
        let mut too_many = reads.clone();
        too_many.push(reads[0].clone());
        assert!(
            actor
                .dispatch_ring_batch(client, frame(too_many, MAX_RING_BATCH_BYTES as u32))
                .is_err()
        );
        assert!(
            actor
                .dispatch_ring_batch(
                    client,
                    frame(reads.clone(), MAX_RING_BATCH_BYTES as u32 + 1)
                )
                .is_err()
        );
        let batch = actor
            .dispatch_ring_batch(client, frame(reads.clone(), MAX_RING_BATCH_BYTES as u32))
            .unwrap();
        let reply = capture_batch(&mut actor, batch);
        actor.complete_io();
        let results = batch_reply(&reply);
        assert_eq!(results.len(), MAX_RING_BATCH_MEMBERS);
        assert!(
            results
                .iter()
                .all(|row| row.result.as_ref().unwrap_err().code == FailureCode::NotFound)
        );
        let batch = actor
            .dispatch_ring_batch(client, frame(reads, MAX_RING_BATCH_BYTES as u32))
            .unwrap();
        let canceled = capture_batch(&mut actor, batch);
        actor.disconnect(client);
        actor.complete_io();
        assert!(matches!(
            canceled.try_recv(),
            Err(mpsc::TryRecvError::Disconnected)
        ));
    }

    fn capture_batch(
        actor: &mut Actor,
        mut batch: PendingRingBatch,
    ) -> mpsc::Receiver<Result<Reply, Failure>> {
        let (reply, received) = mpsc::sync_channel(1);
        batch.reply = Some(reply);
        actor.pending_batches.push(batch);
        received
    }
    fn batch_reply(reply: &mpsc::Receiver<Result<Reply, Failure>>) -> Vec<RingReadResult> {
        let Reply::RingHistoryBatch { results } = reply.try_recv().unwrap().unwrap() else {
            panic!()
        };
        results
    }

    // keeps: invariant ring-reads-batched
    #[test]
    fn ring_batch_shares_the_existing_128_leaf_limit_with_singleton_io() {
        let root = tempfile::tempdir().unwrap();
        let mut actor = fixture_actor(root.path());
        let client = Ulid::new();
        let lease = Ulid::new();
        let (tx, _rx) = mpsc::sync_channel(1);
        actor.clients.insert(client, Target::Local(tx));
        actor.active = Some((client, lease));
        let (held, response) = mpsc::sync_channel(1);
        let (single_reply, single_result) = mpsc::sync_channel(1);
        actor.pending_io.push(PendingIo {
            client,
            request_id: Ulid::new(),
            reply: Some(single_reply),
            response,
            diagnostic: actor.capture_diagnostics.start(
                capture_diagnostics::Phase::SupervisorQuery,
                Ulid::nil(),
                0,
                0,
                0,
            ),
        });
        let frame = |count| {
            Frame::new(Request::new(
                Some(lease),
                Action::ReadRingBatch {
                    reads: (0..count)
                        .map(|n| RingRead {
                            attachment: Attachment {
                                seat: Ulid::from(n as u128 + 1),
                                generation: 1,
                                writer_lease: Ulid::new(),
                            },
                            from: 1,
                            to: 2,
                        })
                        .collect(),
                    max_bytes: MAX_RING_BATCH_BYTES as u32,
                },
            ))
        };
        let mut admitted = Vec::new();
        for _ in 0..15 {
            // 120 member credits plus the held ordinary request.
            let batch = actor
                .dispatch_ring_batch(client, frame(MAX_RING_BATCH_MEMBERS))
                .unwrap();
            admitted.push(capture_batch(&mut actor, batch));
        }
        let partial = actor
            .dispatch_ring_batch(client, frame(MAX_RING_BATCH_MEMBERS))
            .unwrap();
        let partial = capture_batch(&mut actor, partial);
        let full = actor
            .dispatch_ring_batch(client, frame(MAX_RING_BATCH_MEMBERS))
            .unwrap();
        let full = capture_batch(&mut actor, full);
        actor.complete_io();
        for reply in admitted {
            let rows = batch_reply(&reply);
            assert_eq!(rows.len(), 8);
            assert!(
                rows.iter()
                    .all(|row| row.result.as_ref().unwrap_err().code == FailureCode::NotFound)
            );
        }
        let rows = batch_reply(&partial);
        assert_eq!(rows.len(), 8);
        assert!(
            rows[..7]
                .iter()
                .all(|row| row.result.as_ref().unwrap_err().code == FailureCode::NotFound)
        );
        assert_eq!(rows[7].result.as_ref().unwrap_err().code, FailureCode::Busy);
        assert!(
            batch_reply(&full)
                .iter()
                .all(|row| row.result.as_ref().unwrap_err().code == FailureCode::Busy)
        );
        let recovered = actor.dispatch_ring_batch(client, frame(1)).unwrap();
        let recovered = capture_batch(&mut actor, recovered);
        actor.complete_io();
        assert_eq!(
            batch_reply(&recovered)[0].result.as_ref().unwrap_err().code,
            FailureCode::NotFound
        );
        actor.disconnect(client);
        actor.complete_io();
        assert!(matches!(
            single_result.try_recv(),
            Err(mpsc::TryRecvError::Empty)
        ));
        drop(held);
        actor.complete_io();
        assert_eq!(
            single_result.try_recv().unwrap().unwrap_err().code,
            FailureCode::Io
        );
    }

    fn fixture_actor(root: &std::path::Path) -> Actor {
        let journal = root.join("journal");
        let poll = Poll::new().unwrap();
        let wake = Arc::new(Waker::new(poll.registry(), WAKE).unwrap());
        let capture_writer = CaptureWriter::start(wake, Some(&journal)).unwrap();
        let (_tx, rx) = mpsc::sync_channel(CHANNEL_CAPACITY);
        Actor {
            poll,
            socket: None,
            config: Config {
                journal_directory: Some(journal),
                ..Default::default()
            },
            capability: [0; 32],
            boot: Ulid::new(),
            rx,
            clients: HashMap::new(),
            routes: HashMap::new(),
            seats: BTreeMap::new(),
            last_notified_seat: None,
            notification_waiter: None,
            active: None,
            next_token: 2,
            pending_io: Vec::new(),
            capture_diagnostics: Arc::new(capture_diagnostics::Recorder::default()),
            pending_batches: Vec::new(),
            capture_writer,
        }
    }

    // keeps: invariant no-seat-starves
    #[test]
    fn no_seat_starves_under_full_queue() {
        let root = tempfile::tempdir().unwrap();
        let mut actor = fixture_actor(root.path());
        let client = Ulid::new();
        let (tx, rx) = mpsc::sync_channel(1);
        actor.clients.insert(client, Target::Local(tx));
        actor.active = Some((client, Ulid::new()));
        let hot = Ulid::from(1u128);
        let waiting = Ulid::from(2u128);
        for id in [hot, waiting] {
            let mut spec = LaunchSpec::new("/bin/sleep", root.path());
            spec.seat = id;
            spec.args = vec!["60".into()];
            let token = actor.token();
            let mut seat = Seat::spawn(spec, token, actor.poll.registry()).unwrap();
            seat.writer = Some((client, Ulid::new()));
            seat.pending_event = Some(Event::Output {
                seat: id,
                generation: 1,
                seq: 1,
                bytes: vec![b'x'],
            });
            actor.seats.insert(id, seat);
        }

        let mut hot_sequence = 1;
        let mut delivered = Vec::new();
        for _ in 0..8 {
            let seat = actor.seats.get_mut(&hot).unwrap();
            if seat.pending_event.is_none() {
                hot_sequence += 1;
                seat.pending_event = Some(Event::Output {
                    seat: hot,
                    generation: 1,
                    seq: hot_sequence,
                    bytes: vec![b'x'],
                });
            }
            actor.tick_seats(&mut [0; OUTPUT_CHUNK_BYTES]).unwrap();
            // Exactly one credit is returned per sweep, independently of time.
            delivered.push(rx.try_recv().unwrap());
        }
        assert_eq!(delivered[0].seat(), hot);
        assert_eq!(
            delivered[1].seat(),
            waiting,
            "fixed sweep order starved the retained second-seat notification"
        );
        assert_eq!(
            delivered
                .iter()
                .filter(|event| event.seat() == hot)
                .map(|event| event.sequence().unwrap())
                .collect::<Vec<_>>(),
            (1..=7).collect::<Vec<_>>()
        );

        // A receiver can return a credit halfway through a sweep, after the
        // waiting seat was refused. The hot seat must not steal that credit.
        let publish = |actor: &mut Actor, id: Ulid, sequence: Option<u64>| {
            let mut seat = actor.seats.remove(&id).unwrap();
            if let Some(seq) = sequence {
                assert!(seat.pending_event.is_none());
                seat.pending_event = Some(Event::Output {
                    seat: id,
                    generation: 1,
                    seq,
                    bytes: vec![b'x'],
                });
            }
            seat.publish_pending(actor);
            actor.seats.insert(id, seat);
        };
        publish(&mut actor, hot, Some(8));
        publish(&mut actor, hot, Some(9));
        publish(&mut actor, waiting, Some(2));
        assert_eq!(rx.try_recv().unwrap().seat(), hot);
        publish(&mut actor, hot, None);
        assert!(
            actor.seats[&hot].pending_event.is_some(),
            "hot seat stole a credit returned after the waiting seat's turn"
        );
        publish(&mut actor, waiting, None);
        let event = rx.try_recv().unwrap();
        assert_eq!(event.seat(), waiting);
        assert_eq!(event.sequence(), Some(2));
        publish(&mut actor, hot, None);
        let event = rx.try_recv().unwrap();
        assert_eq!(event.seat(), hot);
        assert_eq!(event.sequence(), Some(9));

        // A detached writer cannot keep the retry turn and hold other seats.
        publish(&mut actor, hot, Some(10));
        publish(&mut actor, waiting, Some(3));
        actor.seats.get_mut(&waiting).unwrap().writer = None;
        assert_eq!(rx.try_recv().unwrap().sequence(), Some(10));
        publish(&mut actor, hot, Some(11));
        actor.tick_seats(&mut [0; OUTPUT_CHUNK_BYTES]).unwrap();
        let event = rx.try_recv().unwrap();
        assert_eq!(event.seat(), hot);
        assert_eq!(event.sequence(), Some(11));

        // An attached seat with no remaining notification also loses its turn.
        actor.seats.get_mut(&waiting).unwrap().writer = Some((client, Ulid::new()));
        actor.seats.get_mut(&waiting).unwrap().pending_event = None;
        publish(&mut actor, hot, Some(12));
        publish(&mut actor, waiting, Some(4));
        actor.seats.get_mut(&waiting).unwrap().pending_event = None;
        assert_eq!(rx.try_recv().unwrap().sequence(), Some(12));
        publish(&mut actor, hot, Some(13));
        actor.tick_seats(&mut [0; OUTPUT_CHUNK_BYTES]).unwrap();
        let event = rx.try_recv().unwrap();
        assert_eq!(event.seat(), hot);
        assert_eq!(event.sequence(), Some(13));
        assert!(actor.seats[&waiting].writer.is_some());
    }

    // keeps: spec mux-daemon.md — Ownership and failure boundaries
    #[ignore = "night native boundary: exact selector in scripts/night-test-boundaries.sh"]
    #[test]
    fn a_full_retained_capture_buffer_is_not_pty_eof() {
        let root = tempfile::tempdir().unwrap();
        let mut actor = fixture_actor(root.path());
        let mut spec = LaunchSpec::new("/bin/sleep", root.path());
        spec.args = vec!["60".into()];
        let seat_id = spec.seat;
        let token = actor.token();
        let mut seat = Seat::spawn(spec, token, actor.poll.registry()).unwrap();
        let ring =
            crate::ring::Ring::open(&root.path().join("ring"), seat_id, 1, 0, Default::default())
                .unwrap();
        let handle = actor.capture_writer.register(ring);
        // Model a full coalesced frame retained after append returned WouldBlock.
        seat.output.resize(handle.capture_capacity(), b'x');
        seat.output_since = Some(Instant::now());
        seat.ring = Some(handle);
        seat.read_ready = true;
        seat.tick(&mut actor, &mut [0; OUTPUT_CHUNK_BYTES]).unwrap();
        assert!(!seat.eof, "zero-length read invented EOF on a live PTY");
        assert!(
            seat.registered,
            "live PTY was deregistered after backpressure"
        );
        assert!(seat.output.is_empty(), "retained frame was not admitted");
    }

    #[cfg(target_os = "macos")]
    // keeps: spec mux-daemon.md — Ownership and failure boundaries
    #[ignore = "night native boundary: exact selector in scripts/night-test-boundaries.sh"]
    #[test]
    fn exited_between_spawn_and_registration_emits_exactly_one_exit() {
        let root = tempfile::tempdir().unwrap();
        let mut actor = fixture_actor(root.path());
        let id = Ulid::new();
        let lease = Ulid::new();
        let (tx, rx) = mpsc::sync_channel(16);
        actor.clients.insert(id, Target::Local(tx));
        actor.active = Some((id, lease));
        let token = actor.token();
        let mut seat = Seat::spawn(
            LaunchSpec::new("/usr/bin/true", root.path()),
            token,
            actor.poll.registry(),
        )
        .unwrap();
        seat.writer = Some((id, Ulid::new()));
        let until = Instant::now() + Duration::from_secs(30);
        loop {
            // SAFETY: observe this exact owned child without reaping it; its
            // PID remains reserved for the subsequent EV_ADD/ESRCH path.
            let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
            assert_eq!(
                unsafe {
                    libc::waitid(
                        libc::P_PID,
                        seat.pid as libc::id_t,
                        &mut info,
                        libc::WEXITED | libc::WNOHANG | libc::WNOWAIT,
                    )
                },
                0
            );
            if info.si_pid == seat.pid as libc::pid_t {
                break;
            }
            assert!(Instant::now() < until, "owned fast child did not exit");
            std::thread::sleep(Duration::from_millis(1));
        }
        let child_token = actor.token();
        assert!(
            !seat
                .watch_child(actor.poll.registry(), child_token)
                .unwrap(),
            "expected native ESRCH for the exited child"
        );
        assert_eq!(seat.exited.as_ref().unwrap().code, 0);
        for _ in 0..3 {
            seat.tick(&mut actor, &mut [0; OUTPUT_CHUNK_BYTES]).unwrap();
        }
        let events: Vec<_> = rx.try_iter().collect();
        assert_eq!(events.len(), 1, "missing or duplicate exit: {events:?}");
        assert!(
            matches!(&events[0], Event::Exited { seat: got, generation: 1, status, .. } if *got == seat.spec.seat && status.code == 0)
        );
    }

    #[cfg(target_os = "macos")]
    // keeps: spec mux-daemon.md — Ownership and failure boundaries
    #[ignore = "night native boundary: exact selector in scripts/night-test-boundaries.sh"]
    #[test]
    fn an_early_exiting_writer_is_drained_before_reaping_without_blocking_the_actor() {
        let root = tempfile::tempdir().unwrap();
        let mut actor = fixture_actor(root.path());
        let mut spec = LaunchSpec::new("python3", root.path());
        spec.args = vec![
            "-c".into(),
            "import os,tty; tty.setraw(1); os.write(1,b'x'*(64*1024*1024))".into(),
        ];
        let id = spec.seat;
        let token = actor.token();
        let mut seat = Seat::spawn(spec, token, actor.poll.registry()).unwrap();
        let ring_path = root.path().join("ring");
        let ring = crate::ring::Ring::open(&ring_path, id, 1, 0, Default::default()).unwrap();
        seat.ring = Some(actor.capture_writer.register(ring));
        let mut events = Events::with_capacity(8);
        actor
            .poll
            .poll(&mut events, Some(Duration::from_secs(30)))
            .unwrap();
        assert!(events.iter().any(|event| event.token() == token));
        signal(seat.pid, libc::SIGKILL).unwrap();
        // Leave the master open and undrained, exactly the old teardown cycle.
        // Observe the kernel's unattachable-exiting state without waiting for it.
        let until = Instant::now() + Duration::from_secs(30);
        let child_token = actor.token();
        loop {
            if child_events::Watch::register(actor.poll.registry(), seat.pid, child_token)
                .unwrap()
                .is_none()
            {
                break;
            }
            assert!(Instant::now() < until, "owned child never entered exit");
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(
            !seat
                .watch_child(actor.poll.registry(), child_token)
                .unwrap()
        );
        while !seat.exit_published {
            seat.tick(&mut actor, &mut [0; OUTPUT_CHUNK_BYTES]).unwrap();
            assert!(Instant::now() < until, "exiting PTY was not drained/reaped");
            std::thread::sleep(Duration::from_millis(1));
        }
        drop(seat);
        drop(actor);
        let ring = crate::ring::Ring::open(&ring_path, id, 1, 0, Default::default()).unwrap();
        let history = ring.history(1, ring.status().committed_sequence).unwrap();
        assert!(history.gap.is_none());
        assert!(history.records.iter().any(
            |record| matches!(&record.event, Event::Output { bytes, .. } if !bytes.is_empty())
        ));
        assert_eq!(
            history
                .records
                .iter()
                .filter(|record| matches!(record.event, Event::Exited { .. }))
                .count(),
            1
        );
    }

    #[cfg(target_os = "macos")]
    // keeps: spec mux-daemon.md — Ownership and failure boundaries
    #[ignore = "night native boundary: exact selector in scripts/night-test-boundaries.sh"]
    #[test]
    fn shutdown_closes_a_blocked_staged_pty_and_preserves_capture_and_one_exit() {
        for staged in [false, true] {
            let root = tempfile::tempdir().unwrap();
            let mut actor = fixture_actor(root.path());
            let mut spec = LaunchSpec::new("python3", root.path());
            spec.args = vec![
                "-c".into(),
                "import os,tty; tty.setraw(1); os.write(1,b'x'*(64*1024*1024))".into(),
            ];
            let id = spec.seat;
            let token = actor.token();
            let mut seat = Seat::spawn(spec, token, actor.poll.registry()).unwrap();
            let child_token = actor.token();
            seat.watch_child(actor.poll.registry(), child_token)
                .unwrap();
            let ring_path = root.path().join("ring");
            let ring = crate::ring::Ring::open(&ring_path, id, 1, 0, Default::default()).unwrap();
            seat.ring = Some(actor.capture_writer.register(ring));
            // Observe actual PTY output, but leave the writer blocked on a
            // 64 MiB write. Retain this read in the normal coalescing buffer.
            let mut bytes = [0; 256];
            let until = Instant::now() + Duration::from_secs(30);
            let count = loop {
                // SAFETY: the live master owns fd and bytes is writable.
                let count = unsafe { libc::read(seat.fd, bytes.as_mut_ptr().cast(), bytes.len()) };
                if count > 0 {
                    break count as usize;
                }
                assert!(Instant::now() < until, "writer never reached the PTY");
                std::thread::sleep(Duration::from_millis(1));
            };
            seat.output.extend_from_slice(&bytes[..count]);
            seat.output_since = Some(Instant::now());
            assert!(seat.child.try_wait().unwrap().is_none());
            if staged {
                signal(seat.pid, libc::SIGSTOP).unwrap();
                loop {
                    // SAFETY: observe the exact owned child's stop, without
                    // consuming the status its supervisor must reap.
                    let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
                    assert_eq!(
                        unsafe {
                            libc::waitid(
                                libc::P_PID,
                                seat.pid as libc::id_t,
                                &mut info,
                                libc::WSTOPPED | libc::WNOHANG | libc::WNOWAIT,
                            )
                        },
                        0
                    );
                    if info.si_pid == seat.pid as libc::pid_t {
                        break;
                    }
                    assert!(Instant::now() < until, "owned writer did not stop");
                    std::thread::sleep(Duration::from_millis(1));
                }
            }
            let pid = seat.pid;
            drop(seat);
            // SAFETY: signal zero only queries the original child's existence.
            assert_eq!(unsafe { libc::kill(pid as libc::pid_t, 0) }, -1);
            assert_eq!(io::Error::last_os_error().raw_os_error(), Some(libc::ESRCH));
            drop(actor); // drains the capture writer's shutdown barrier
            let ring = crate::ring::Ring::open(&ring_path, id, 1, 0, Default::default()).unwrap();
            assert_eq!(ring.status().committed_sequence, 2);
            let history = ring.history(1, 2).unwrap();
            assert!(history.gap.is_none());
            assert_eq!(
                history.records.len(),
                2,
                "missing/duplicate shutdown records"
            );
            assert!(
                matches!(&history.records[0].event, Event::Output { bytes: got, .. } if got == &bytes[..count])
            );
            assert!(
                matches!(&history.records[1].event, Event::Exited { status, .. } if status.signal.is_some())
            );
        }
    }
}

#[cfg(test)]
mod close_policy_tests {
    use super::*;

    fn gone() -> Result<(), Failure> {
        Err(Failure::new(FailureCode::Exited, "fixture process exited"))
    }

    // keeps: spec c1-live-seats §4 (real sixty-second grace, no real sleep)
    #[tokio::test(start_paused = true)]
    async fn grace_expires_at_sixty_seconds_and_undo_is_refused_at_the_boundary() {
        let mut close = CloseState::default();
        let id = Ulid::new();
        let origin = tokio::time::Instant::now();
        assert!(matches!(
            close
                .schedule(id, 61_000, Ok(()), 1_000, origin.into_std())
                .unwrap(),
            Reply::CloseScheduled {
                deadline_ms: 61_000,
                ..
            }
        ));
        tokio::time::advance(Duration::from_millis(59_999)).await;
        assert!(!close.expired(tokio::time::Instant::now().into_std()));
        assert_eq!(
            close
                .undo(Ulid::new(), Ok(()), tokio::time::Instant::now().into_std())
                .unwrap_err()
                .code,
            FailureCode::Exited
        );
        tokio::time::advance(Duration::from_millis(1)).await;
        assert!(close.expired(tokio::time::Instant::now().into_std()));
        assert_eq!(
            close
                .undo(id, Ok(()), tokio::time::Instant::now().into_std())
                .unwrap_err()
                .code,
            FailureCode::Exited
        );
        // Nothing in expiry consults a logic actor; its absence cannot reset it.
        assert!(matches!(
            close
                .schedule(
                    id,
                    61_000,
                    gone(),
                    61_000,
                    tokio::time::Instant::now().into_std()
                )
                .unwrap(),
            Reply::CloseScheduled {
                deadline_ms: 61_000,
                ..
            }
        ));
        assert!(close.expired(tokio::time::Instant::now().into_std()));
    }

    // keeps: spec c1-live-seats §4 (idempotent close/undo and conflicting IDs)
    #[tokio::test(start_paused = true)]
    async fn retry_keeps_the_original_deadline_and_undo_cancels_it_once() {
        let mut close = CloseState::default();
        let id = Ulid::new();
        close
            .schedule(
                id,
                60_000,
                Ok(()),
                0,
                tokio::time::Instant::now().into_std(),
            )
            .unwrap();
        tokio::time::advance(Duration::from_secs(30)).await;
        assert!(matches!(
            close
                .schedule(
                    id,
                    60_000,
                    Ok(()),
                    30_000,
                    tokio::time::Instant::now().into_std()
                )
                .unwrap(),
            Reply::CloseScheduled {
                deadline_ms: 60_000,
                ..
            }
        ));
        assert_eq!(
            close
                .schedule(
                    id,
                    90_000,
                    Ok(()),
                    30_000,
                    tokio::time::Instant::now().into_std()
                )
                .unwrap_err()
                .code,
            FailureCode::Invalid
        );
        assert_eq!(
            close
                .schedule(
                    Ulid::new(),
                    60_000,
                    Ok(()),
                    30_000,
                    tokio::time::Instant::now().into_std()
                )
                .unwrap_err()
                .code,
            FailureCode::Invalid
        );
        assert!(matches!(
            close
                .undo(id, Ok(()), tokio::time::Instant::now().into_std())
                .unwrap(),
            Reply::CloseUndone { .. }
        ));
        tokio::time::advance(Duration::from_secs(60)).await;
        assert!(!close.expired(tokio::time::Instant::now().into_std()));
        assert!(matches!(
            close
                .undo(id, gone(), tokio::time::Instant::now().into_std())
                .unwrap(),
            Reply::CloseUndone { .. }
        ));
        assert_eq!(
            close
                .schedule(
                    id,
                    60_000,
                    Ok(()),
                    90_000,
                    tokio::time::Instant::now().into_std()
                )
                .unwrap_err()
                .code,
            FailureCode::Invalid
        );
    }
}
