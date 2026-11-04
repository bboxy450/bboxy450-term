// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
// Inspired by Loom's cfg shim and Shuttle's seeded scheduler approach:
// https://github.com/tokio-rs/loom and https://github.com/awslabs/shuttle
// Independently implemented test adapters; no upstream synchronisation source copied.
//! Test-only scheduler backends for the actual source adapters and reactor macro.
#![allow(missing_docs)]

use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll, Waker};
use std::sync::Arc as WakeArc;
use std::task::Wake;

#[cfg(loom)]
use loom::{sync, thread};
#[cfg(shuttle)]
use shuttle::{sync, thread};
use sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use sync::{Arc, Mutex};

use crate::source::{self, ReactorSource};

pub(crate) fn broken(_kind: &str) -> bool {
    #[cfg(all(loom, yarn_broken_controls))]
    return std::env::var("YARN_BROKEN").is_ok_and(|kind| kind == _kind);
    #[cfg(not(all(loom, yarn_broken_controls)))]
    false
}

struct WakeCounter(AtomicUsize);

impl Wake for WakeCounter {
    fn wake(self: WakeArc<Self>) {
        self.wake_by_ref();
    }

    fn wake_by_ref(self: &WakeArc<Self>) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

fn counter() -> (WakeArc<WakeCounter>, Waker) {
    let count = WakeArc::new(WakeCounter(AtomicUsize::new(0)));
    let waker = Waker::from(WakeArc::clone(&count));
    (count, waker)
}

// Boundary assumption for retained arbitrary futures: register, then recheck.
// This is test producer code, not a replacement for Tokio's AtomicWaker.
struct Signal {
    ready: AtomicBool,
    value: AtomicUsize,
    waker: Mutex<Option<Waker>>,
}

impl Signal {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            ready: AtomicBool::new(false),
            value: AtomicUsize::new(0),
            waker: Mutex::new(None),
        })
    }

    fn publish(&self, value: usize) {
        self.value.store(value, Ordering::Relaxed);
        self.ready.store(true, Ordering::Release);
        let waker = self.waker.lock().unwrap().take();
        if let Some(waker) = waker {
            waker.wake();
        }
    }
}

struct SignalFuture(Arc<Signal>);

impl Future for SignalFuture {
    type Output = usize;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<usize> {
        if self.0.ready.load(Ordering::Acquire) {
            return Poll::Ready(self.0.value.load(Ordering::Relaxed));
        }
        {
            let mut waker = self.0.waker.lock().unwrap();
            if !broken("waker") || waker.is_none() {
                *waker = Some(cx.waker().clone());
            }
        }
        if !broken("wake") && self.0.ready.load(Ordering::Acquire) {
            return Poll::Ready(self.0.value.load(Ordering::Relaxed));
        }
        Poll::Pending
    }
}

// Only the public poll/try/close/backlog boundary is simulated. Queue, close
// and waker registration are one mutex transaction; this does NOT explore
// Tokio channel internals, cooperative budgets, timers or OS executors.
pub(crate) mod mpsc {
    use super::{Arc, Context, Mutex, Poll, Waker};
    use std::collections::VecDeque;

    pub mod error {
        #[derive(Debug)]
        pub enum TryRecvError {
            Empty,
            Disconnected,
        }
    }
    use error::TryRecvError;

    #[derive(Debug)]
    struct State<T> {
        queue: VecDeque<T>,
        closed: bool,
        waker: Option<Waker>,
    }

    #[derive(Debug)]
    pub struct Receiver<T>(Arc<Mutex<State<T>>>);
    #[derive(Debug)]
    pub struct UnboundedReceiver<T>(Receiver<T>);
    pub struct Sender<T>(Arc<Mutex<State<T>>>);

    pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
        let state = Arc::new(Mutex::new(State {
            queue: VecDeque::new(),
            closed: false,
            waker: None,
        }));
        (Sender(Arc::clone(&state)), Receiver(state))
    }

    #[cfg(shuttle)]
    pub fn unbounded_channel<T>() -> (Sender<T>, UnboundedReceiver<T>) {
        let (sender, receiver) = channel();
        (sender, UnboundedReceiver(receiver))
    }

    impl<T> Sender<T> {
        pub fn send(&self, item: T) -> Result<(), T> {
            let wake = {
                let mut state = self.0.lock().unwrap();
                if state.closed {
                    return Err(item);
                }
                state.queue.push_back(item);
                state.waker.take()
            };
            if let Some(wake) = wake {
                wake.wake();
            }
            Ok(())
        }
    }

    impl<T> Drop for Sender<T> {
        fn drop(&mut self) {
            let wake = {
                let mut state = self.0.lock().unwrap();
                state.closed = true;
                state.waker.take()
            };
            if let Some(wake) = wake {
                wake.wake();
            }
        }
    }

    impl<T> Drop for Receiver<T> {
        fn drop(&mut self) {
            self.0.lock().unwrap().closed = true;
        }
    }

    impl<T> Receiver<T> {
        pub fn poll_recv(&mut self, cx: &mut Context<'_>) -> Poll<Option<T>> {
            let mut state = self.0.lock().unwrap();
            if let Some(item) = state.queue.pop_front() {
                Poll::Ready(Some(item))
            } else if state.closed {
                Poll::Ready(None)
            } else {
                state.waker = Some(cx.waker().clone());
                Poll::Pending
            }
        }

        pub fn try_recv(&mut self) -> Result<T, TryRecvError> {
            let mut state = self.0.lock().unwrap();
            state.queue.pop_front().ok_or(if state.closed {
                TryRecvError::Disconnected
            } else {
                TryRecvError::Empty
            })
        }

        pub fn is_empty(&self) -> bool {
            self.0.lock().unwrap().queue.is_empty()
        }

        pub fn is_closed(&self) -> bool {
            self.0.lock().unwrap().closed
        }
    }

    impl<T> UnboundedReceiver<T> {
        pub fn poll_recv(&mut self, cx: &mut Context<'_>) -> Poll<Option<T>> {
            self.0.poll_recv(cx)
        }

        pub fn try_recv(&mut self) -> Result<T, TryRecvError> {
            self.0.try_recv()
        }

        pub fn is_empty(&self) -> bool {
            self.0.is_empty()
        }

        pub fn is_closed(&self) -> bool {
            self.0.is_closed()
        }
    }
}

#[cfg(loom)]
mod loom_checks {
    use super::*;
    use source::{ChannelEvent, close};
    use std::{eprintln, vec::Vec};

    fn explore(f: impl Fn() + Send + Sync + 'static) {
        let count = WakeArc::new(std::sync::atomic::AtomicUsize::new(0));
        let runs = WakeArc::clone(&count);
        let mut builder = loom::model::Builder::new();
        builder.max_threads = 3;
        builder.max_branches = 1_000;
        builder.preemption_bound = Some(2);
        // Environment settings must never silently truncate the checked bound.
        builder.max_permutations = None;
        builder.max_duration = None;
        builder.check(move || {
            runs.fetch_add(1, Ordering::Relaxed);
            f();
        });
        eprintln!(
            "Loom: {} schedules, preemption_bound=2",
            count.load(Ordering::Relaxed)
        );
    }

    #[test]
    fn lost_wake() {
        explore(|| {
            let signal = Signal::new();
            let mut source = source::one_shot(SignalFuture(Arc::clone(&signal)));
            let producer = thread::spawn(move || signal.publish(7));
            let (count, waker) = counter();
            let first = source.poll_next(&mut Context::from_waker(&waker));
            producer.join().unwrap();
            if first.is_pending() {
                assert!(count.0.load(Ordering::SeqCst) > 0, "WAKE-001 lost wake");
                assert_eq!(
                    source.poll_next(&mut Context::from_waker(&waker)),
                    Poll::Ready(7)
                );
            } else {
                assert_eq!(first, Poll::Ready(7));
            }
            assert!(
                source
                    .poll_next(&mut Context::from_waker(&waker))
                    .is_pending()
            );
        });
    }

    #[test]
    fn close_racing_ready() {
        explore(|| {
            let (sender, receiver) = mpsc::channel();
            let mut source = source::mpsc(receiver, close::Emit);
            let producer = thread::spawn(move || {
                sender.send(7).unwrap();
                drop(sender);
            });
            let (count, waker) = counter();
            let mut cx = Context::from_waker(&waker);
            let mut events = Vec::new();
            let first = source.poll_next(&mut cx);
            let pending = first.is_pending();
            if let Poll::Ready(event) = first {
                events.push(event);
            }
            producer.join().unwrap();
            if pending {
                assert!(
                    count.0.load(Ordering::SeqCst) > 0,
                    "CLOSE-001 lost close wake"
                );
            }
            while events.len() < 2 {
                let Poll::Ready(event) = source.poll_next(&mut cx) else {
                    panic!("CLOSE-001 item or close lost");
                };
                events.push(event);
            }
            assert_eq!(events, [ChannelEvent::Item(7), ChannelEvent::Closed]);
            assert!(source.is_dormant(), "CLOSE-001 receiver not retired");
            assert!(
                source.poll_next(&mut cx).is_pending(),
                "CLOSE-001 duplicate close"
            );
        });
    }

    #[test]
    fn double_service() {
        explore(|| {
            let signal = Signal::new();
            signal.publish(7);
            let source = Arc::new(Mutex::new(source::one_shot(SignalFuture(signal))));
            let served = Arc::new(AtomicUsize::new(0));
            let poll = |source: Arc<Mutex<source::OneShot<SignalFuture>>>,
                        served: Arc<AtomicUsize>| {
                if source
                    .lock()
                    .unwrap()
                    .poll_next(&mut Context::from_waker(Waker::noop()))
                    .is_ready()
                {
                    served.fetch_add(1, Ordering::SeqCst);
                }
            };
            let source2 = Arc::clone(&source);
            let served2 = Arc::clone(&served);
            let consumer = thread::spawn(move || poll(source2, served2));
            poll(source, Arc::clone(&served));
            consumer.join().unwrap();
            assert_eq!(
                served.load(Ordering::SeqCst),
                1,
                "SERVICE-001 duplicate delivery"
            );
        });
    }

    #[test]
    fn waker_replacement() {
        explore(|| {
            let signal = Signal::new();
            let mut source = source::one_shot(SignalFuture(Arc::clone(&signal)));
            let (_, old) = counter();
            assert!(
                source
                    .poll_next(&mut Context::from_waker(&old))
                    .is_pending()
            );
            let producer = thread::spawn(move || signal.publish(7));
            let (count, new) = counter();
            let second = source.poll_next(&mut Context::from_waker(&new));
            producer.join().unwrap();
            if second.is_pending() {
                assert!(count.0.load(Ordering::SeqCst) > 0, "WAKER-001 stale waker");
                assert_eq!(
                    source.poll_next(&mut Context::from_waker(&new)),
                    Poll::Ready(7)
                );
            } else {
                assert_eq!(second, Poll::Ready(7));
            }
        });
    }
}

#[cfg(shuttle)]
mod shuttle_checks {
    use super::*;
    use crate::reactor::Control;
    use source::close;
    use std::{convert::Infallible, eprintln, vec, vec::Vec};

    const SEEDS: [u64; 2] = [0x5941_524e, 0x4c4f_4f4d];
    const ITERATIONS: usize = 128;

    fn explore(f: fn()) {
        if let Ok(schedule) = std::env::var("YARN_SHUTTLE_REPLAY") {
            shuttle::replay(f, &schedule);
            return;
        }
        for seed in SEEDS {
            for pct in [false, true] {
                let mut config = shuttle::Config::new();
                config.max_steps = shuttle::MaxSteps::FailAfter(10_000);
                config.max_time = None;
                config.failure_persistence = shuttle::FailurePersistence::Print;
                let runs = if pct {
                    shuttle::Runner::new(
                        shuttle::scheduler::PctScheduler::new_from_seed(seed, 3, ITERATIONS),
                        config,
                    )
                    .run(f)
                } else {
                    shuttle::Runner::new(
                        shuttle::scheduler::RandomScheduler::new_from_seed(seed, ITERATIONS),
                        config,
                    )
                    .run(f)
                };
                assert_eq!(runs, ITERATIONS, "scheduler truncated its iteration count");
                eprintln!(
                    "Shuttle: seed={seed:#x}, scheduler={}, iterations={ITERATIONS}, depth=3 for PCT",
                    if pct { "PCT" } else { "random" }
                );
            }
        }
    }

    // mode 0 drains every item; 1 begins shut down; 2 races shutdown against work.
    fn reactor_scenario(mode: u8) {
        let (high_tx, high_rx) = mpsc::channel();
        let (low_tx, low_rx) = mpsc::unbounded_channel();
        let stop = Signal::new();
        if mode == 1 {
            stop.publish(9);
        }
        let high = thread::spawn(move || {
            for item in 0..3 {
                let result = high_tx.send(item);
                assert!(mode != 0 || result.is_ok());
                thread::yield_now();
            }
        });
        let low = thread::spawn(move || {
            for item in 0..3 {
                let result = low_tx.send(item);
                assert!(mode != 0 || result.is_ok());
                thread::yield_now();
            }
        });
        let racing_stop = (mode == 2).then(|| {
            let stop = Arc::clone(&stop);
            thread::spawn(move || stop.publish(9))
        });
        let mut shutdown = source::one_shot(SignalFuture(Arc::clone(&stop)));
        let mut high_source = source::mpsc(high_rx, close::Dormant);
        let mut low_source = source::mpsc(low_rx, close::Dormant);
        let mut handled = [Vec::new(), Vec::new()];
        let mut window = 0;
        let mut stop_was_ready = false;
        let result: Result<usize, Infallible> = shuttle::future::block_on(async {
            crate::reactor! {
                policy { selection: biased; required_phases: [before_poll, after_event]; }
                before_poll {
                    window = 0;
                    if mode == 0 && handled.iter().map(Vec::len).sum::<usize>() == 6 {
                        stop.publish(9);
                    }
                    stop_was_ready = stop.ready.load(Ordering::Acquire);
                    Ok(())
                }
                #[source(shutdown)]
                #[readiness(quiescent)]
                #[shutdown]
                #[terminal]
                value = shutdown => { Ok(value) }
                #[source(high)]
                #[readiness(may_remain_ready)]
                #[yields_to(low, when = buffered)]
                #[drain(max = 2)]
                value = high_source => {
                    assert!(!stop_was_ready, "SHUTDOWN-001 ready shutdown lost priority");
                    handled[0].push(value);
                    window += 1;
                    Ok(Control::Continue)
                }
                #[source(low)]
                #[readiness(may_remain_ready)]
                #[drain(max = 2)]
                value = low_source => {
                    assert!(!stop_was_ready, "SHUTDOWN-001 ready shutdown lost priority");
                    handled[1].push(value);
                    window += 1;
                    Ok(Control::Continue)
                }
                after_event {
                    assert!((1..=2).contains(&window), "DRAIN-001 service exceeded bound");
                    thread::yield_now();
                    Ok(())
                }
            }
        });
        assert_eq!(result, Ok(9));
        high.join().unwrap();
        low.join().unwrap();
        if let Some(stop) = racing_stop {
            stop.join().unwrap();
        }
        if mode == 0 {
            assert_eq!(handled, [vec![0, 1, 2], vec![0, 1, 2]]);
        } else if mode == 1 {
            assert!(handled.iter().all(Vec::is_empty));
        }
        for stream in handled {
            assert!(
                stream.windows(2).all(|pair| pair[0] < pair[1]),
                "SERVICE-001 duplicate or reordered item"
            );
        }
    }

    #[test]
    fn several_sources_bounded_drain() {
        explore(|| reactor_scenario(0));
    }

    #[test]
    fn shutdown_priority() {
        explore(|| reactor_scenario(1));
    }

    #[test]
    fn shutdown_races_ready_and_close() {
        explore(|| reactor_scenario(2));
    }

    #[test]
    fn replacement_a_b_a_retires_old_completions() {
        explore(|| {
            let old_a = Signal::new();
            let b = Signal::new();
            let new_a = Signal::new();
            let mut source = source::OptionalOneShot::from_future(SignalFuture(Arc::clone(&old_a)));
            let (_, old_waker) = counter();
            assert!(
                source
                    .poll_next(&mut Context::from_waker(&old_waker))
                    .is_pending()
            );
            let a_completion = thread::spawn(move || old_a.publish(1));
            assert!(source.cancel_and_replace(SignalFuture(Arc::clone(&b))));
            let b_completion = thread::spawn(move || b.publish(2));
            assert!(source.cancel_and_replace(SignalFuture(Arc::clone(&new_a))));
            let new_completion = thread::spawn(move || new_a.publish(3));
            let result =
                shuttle::future::block_on(core::future::poll_fn(|cx| source.poll_next(cx)));
            assert_eq!(result, 3, "GENERATION-001 stale A/B completion");
            a_completion.join().unwrap();
            b_completion.join().unwrap();
            new_completion.join().unwrap();
            assert!(source.is_dormant());
            assert!(
                source
                    .poll_next(&mut Context::from_waker(Waker::noop()))
                    .is_pending()
            );

            let (old_tx, old_rx) = mpsc::channel();
            let (b_tx, b_rx) = mpsc::channel();
            let (current_tx, current_rx) = mpsc::channel();
            let mut source = source::OptionalMpsc::new(close::Emit);
            source.arm(old_rx).unwrap();
            assert!(
                source
                    .poll_next(&mut Context::from_waker(&old_waker))
                    .is_pending()
            );
            let old_completion = thread::spawn(move || {
                let _ = old_tx.send(1);
            });
            drop(source.replace(b_rx));
            let b_completion = thread::spawn(move || {
                let _ = b_tx.send(2);
            });
            drop(source.replace(current_rx));
            let current_completion = thread::spawn(move || current_tx.send(3).unwrap());
            let item = shuttle::future::block_on(core::future::poll_fn(|cx| source.poll_next(cx)));
            assert_eq!(
                item,
                source::ChannelEvent::Item(3),
                "GENERATION-001 stale receiver"
            );
            old_completion.join().unwrap();
            b_completion.join().unwrap();
            current_completion.join().unwrap();
            assert_eq!(
                source.poll_next(&mut Context::from_waker(Waker::noop())),
                Poll::Ready(source::ChannelEvent::Closed)
            );
            assert!(source.is_dormant());
            assert!(
                source
                    .poll_next(&mut Context::from_waker(Waker::noop()))
                    .is_pending()
            );
        });
    }
}
