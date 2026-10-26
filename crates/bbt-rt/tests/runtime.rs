#![allow(
    clippy::disallowed_methods,
    reason = "test driver waits and probes child state off the app UI thread"
)]

use std::ffi::OsString;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use std::time::{Duration, Instant};

use bbt_rt::*;

#[derive(Default)]
struct CountWakes(AtomicUsize);
impl Waker for CountWakes {
    fn wake(&self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

struct Signal(std::sync::mpsc::SyncSender<()>);
impl Waker for Signal {
    fn wake(&self) {
        let _ = self.0.try_send(());
    }
}
struct Changes {
    dirty: DirtySet,
    ready: std::sync::mpsc::Receiver<()>,
}
impl Changes {
    fn wait<T>(&self, mut result: impl FnMut() -> Option<T>) -> T {
        loop {
            self.dirty.drain();
            if let Some(value) = result() {
                return value;
            }
            self.ready
                .recv_timeout(Duration::from_secs(120))
                .expect("command progress hang guard");
        }
    }
}
fn bridge(capacity: usize) -> (CommandRunner, UiReceiver, ViewRing<OutputChunk>, Changes) {
    let (signal, ready) = std::sync::mpsc::sync_channel(1);
    let dirty = DirtySet::new(2, Arc::new(Signal(signal)));
    let (sender, receiver) = ui_channel(8, dirty.source(0));
    let output = ViewRing::new(capacity, dirty.source(1));
    (
        CommandRunner::new(sender, output.clone()),
        receiver,
        output,
        Changes { dirty, ready },
    )
}

fn command(runner: &CommandRunner, script: &str) -> TaskId {
    runner.spawn_command(
        vec!["/bin/sh".into(), "-c".into(), script.into()],
        std::env::temp_dir(),
        vec![],
    )
}

struct CancelOnDrop(TaskId);
impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        if self.0.status().is_none() {
            cancel(&self.0);
            if let Some(pid) = self.0.pid() {
                // SAFETY: the TaskId was created by this fixture's CommandRunner;
                // retire its dedicated group even when an assertion unwinds.
                unsafe {
                    libc::kill(-(pid as i32), libc::SIGKILL);
                }
            }
        }
    }
}

fn finished(task: &TaskId, changes: &Changes) -> ExitStatus {
    let _cleanup = CancelOnDrop(task.clone());
    changes.wait(|| task.status().cloned())
}

// keeps: spec runtime §3 — bounded handoff, coalesced wake and caller-visible progress
#[test]
#[ignore = "#327 B4: throughput is a reported night measurement; scripts/night-test-boundaries.sh"]
fn million_messages_per_second_never_wait_for_a_consumer() {
    let dirty = DirtySet::new(1, Arc::new(CountWakes::default()));
    let (sender, receiver) = ui_channel(8, dirty.source(0));
    let start = Instant::now();
    let mut rejected = 0;
    for _ in 0..1_000_000 {
        rejected += usize::from(sender.try_send(UiMsg::Changed { source: 0 }).is_err());
    }
    let elapsed = start.elapsed();
    println!(
        "1M try_send: {elapsed:?}, {:.0} msgs/s, rejected={rejected}",
        1_000_000.0 / elapsed.as_secs_f64()
    );
    assert_eq!(rejected, 1_000_000 - 8);
    assert_eq!(receiver.len(), 8);
}

// keeps: spec runtime §3 — bounded handoff, coalesced wake and caller-visible progress
#[test]
fn ten_thousand_dirty_sets_wake_once_until_drained() {
    let waker = Arc::new(CountWakes::default());
    let dirty = DirtySet::new(1000, waker.clone());
    for index in 0..10_000 {
        dirty.source(index % 1000).mark();
    }
    assert_eq!(waker.0.load(Ordering::SeqCst), 1);
    assert_eq!(dirty.drain(), (0..1000).collect::<Vec<_>>());
    assert!(dirty.drain().is_empty());
    dirty.source(999).mark();
    assert_eq!(waker.0.load(Ordering::SeqCst), 2);
    assert_eq!(dirty.drain(), vec![999]);
}

// keeps: spec runtime §3 — bounded handoff, coalesced wake and caller-visible progress
#[test]
fn hundred_thousand_concurrent_notices_keep_all_data_and_the_latest_value() {
    const COUNT: usize = 100_000;
    let waker = Arc::new(CountWakes::default());
    let dirty = DirtySet::new(3, waker.clone());
    let (sender, receiver) = ui_channel(COUNT, dirty.source(0));
    let latest = Latest::new(dirty.source(1));
    assert!(dirty.drain().is_empty());
    std::thread::scope(|scope| {
        for producer in 0..4 {
            let sender = sender.clone();
            scope.spawn(move || {
                for sequence in 0..COUNT / 4 {
                    sender
                        .try_send(UiMsg::Changed {
                            source: producer * (COUNT / 4) + sequence,
                        })
                        .unwrap();
                }
            });
        }
    });
    latest.publish(COUNT);
    assert_eq!(waker.0.load(Ordering::SeqCst), 1);
    assert_eq!(dirty.drain(), vec![0, 1]);
    let mut seen = vec![false; COUNT];
    while let Some(message) = receiver.try_recv() {
        let UiMsg::Changed { source } = message else {
            panic!("unexpected message");
        };
        assert!(!std::mem::replace(&mut seen[source], true));
    }
    assert!(seen.into_iter().all(|value| value));
    assert_eq!(latest.take(), Some(COUNT));
    assert!(latest.take().is_none());
    dirty.source(2).mark();
    assert_eq!(waker.0.load(Ordering::SeqCst), 2);
    assert_eq!(dirty.drain(), vec![2]);
}

// keeps: spec runtime §3 — bounded handoff, coalesced wake and caller-visible progress
#[test]
fn paint_publications_stay_latched_until_a_draw_and_recheck_its_tail() {
    let waker = Arc::new(CountWakes::default());
    let dirty = DirtySet::new(66, waker.clone());
    let latest = Latest::new(dirty.paint_source(65));
    for revision in 0..100_000 {
        latest.publish(revision);
        assert_eq!(dirty.drain(), vec![65]);
    }
    assert_eq!(waker.0.load(Ordering::SeqCst), 1);
    assert_eq!(latest.take(), Some(99_999));
    // Publication between taking the drawn value and retiring the draw must
    // retain the newest value and arrange a new wake, even without another writer.
    latest.publish(100_000);
    assert_eq!(waker.0.load(Ordering::SeqCst), 1);
    latest.complete_drain();
    assert_eq!(waker.0.load(Ordering::SeqCst), 2);
    assert_eq!(dirty.drain(), vec![65]);
    assert_eq!(latest.take(), Some(100_000));
    latest.complete_drain();
    assert!(dirty.drain().is_empty());
}

// keeps: spec runtime §3 — bounded handoff, coalesced wake and caller-visible progress
#[test]
fn a_tick_can_wake_while_paint_is_latched() {
    let waker = Arc::new(CountWakes::default());
    let dirty = DirtySet::new(6, waker.clone());
    let latest = Latest::new(dirty.paint_source(5));
    latest.publish(7);
    assert_eq!(dirty.drain(), vec![5]);
    dirty.source(4).mark();
    assert_eq!(waker.0.load(Ordering::SeqCst), 2);
    assert_eq!(dirty.drain(), vec![4, 5]);
    assert_eq!(latest.take(), Some(7));
    latest.complete_drain();
    assert!(dirty.drain().is_empty());
}

// keeps: spec runtime §3 — bounded handoff, coalesced wake and caller-visible progress
#[test]
fn a_budgeted_deferred_drain_rearms_its_tail_without_another_producer() {
    let waker = Arc::new(CountWakes::default());
    let dirty = DirtySet::new(2, waker.clone());
    let (sender, receiver) = ui_channel(4, dirty.deferred_source(0));
    let ring = ViewRing::new(4, dirty.deferred_source(1));
    for source in 0..4 {
        sender.try_send(UiMsg::Changed { source }).unwrap();
        ring.push(source);
    }
    assert_eq!(dirty.drain(), vec![0, 1]);
    assert!(receiver.try_recv().is_some());
    assert_eq!(ring.pop(), Some(0));
    let before = waker.0.load(Ordering::SeqCst);
    receiver.complete_drain();
    ring.complete_drain();
    assert_eq!(waker.0.load(Ordering::SeqCst), before + 2);
    assert_eq!(dirty.drain(), vec![0, 1]);
    for expected in 1..4 {
        assert!(
            matches!(receiver.try_recv(), Some(UiMsg::Changed { source }) if source == expected)
        );
        assert_eq!(ring.pop(), Some(expected));
    }
    receiver.complete_drain();
    ring.complete_drain();
    assert!(dirty.drain().is_empty());
}

// keeps: spec runtime §3 — bounded handoff, coalesced wake and caller-visible progress
#[test]
fn latest_and_view_ring_replace_old_values() {
    let dirty = DirtySet::new(2, Arc::new(CountWakes::default()));
    let latest = Latest::new(dirty.source(0));
    let ring = ViewRing::new(3, dirty.source(1));
    for value in 0..100 {
        latest.publish(value);
        ring.push(value);
    }
    assert_eq!(latest.take(), Some(99));
    assert_eq!(latest.take(), None);
    assert_eq!(ring.dropped(), 97);
    assert_eq!(
        (ring.pop(), ring.pop(), ring.pop(), ring.pop()),
        (Some(97), Some(98), Some(99), None)
    );
}

fn hundred_megabyte_command_finishes_with_a_bounded_view() {
    let (runner, messages, output, changes) = bridge(16);
    let task = command(&runner, "dd if=/dev/zero bs=1048576 count=100 2>/dev/null");
    let status = finished(&task, &changes);
    assert_eq!(status.code, Some(0), "{status:?}");
    assert_eq!(status.error, None);
    assert_eq!(status.stdout_bytes, 100 * 1024 * 1024);
    assert_eq!(status.stderr_bytes, 0);
    assert!(!status.cancelled);
    assert_eq!(output.len(), output.capacity());
    assert!(output.dropped() > 0);
    assert_eq!(
        messages.len(),
        8,
        "deliberately saturated notification queue"
    );
    let mut retained = 0;
    while let Some(chunk) = output.pop() {
        assert!(chunk.bytes.len() <= 16 * 1024);
        retained += chunk.bytes.len();
    }
    assert!(retained <= 16 * 16 * 1024);
    println!(
        "stdout={} bytes, retained={retained} bytes, dropped={} chunks; Exited status survived full queue",
        status.stdout_bytes,
        output.dropped()
    );
}

fn command_argv_cwd_env_stderr_and_exit_are_observable() {
    let (runner, messages, output, changes) = bridge(32);
    let task = runner.spawn_command(
        vec![
            "/bin/sh".into(),
            "-c".into(),
            "printf '%s:%s' \"$PWD\" \"$BBT_VALUE\"; printf 'problem\\n' >&2; exit 7".into(),
        ],
        "/".into(),
        vec![(
            OsString::from("BBT_VALUE"),
            OsString::from("literal $(no shell expansion)"),
        )],
    );
    let status = finished(&task, &changes);
    assert_eq!(status.code, Some(7));
    let mut out = Vec::new();
    let mut err = Vec::new();
    while let Some(chunk) = output.pop() {
        match chunk.stream {
            Stream::Stdout => out.extend_from_slice(&chunk.bytes),
            Stream::Stderr => err.extend_from_slice(&chunk.bytes),
        }
    }
    assert_eq!(out, b"/:literal $(no shell expansion)");
    assert_eq!(err, b"problem\n");
    changes.wait(|| {
        while let Some(message) = messages.try_recv() {
            if matches!(message, UiMsg::Exited { task: id, status: s } if id == task && s.code == Some(7)) { return Some(()); }
        }
        None
    });
}

fn cancel_kills_the_process_group_including_a_grandchild() {
    let (runner, _, output, changes) = bridge(8);
    let task = command(&runner, "sleep 3600 & printf '%s\\n' \"$!\"; wait");
    let _cleanup = CancelOnDrop(task.clone());
    let child = changes.wait(|| {
        output.pop().map(|chunk| {
            std::str::from_utf8(&chunk.bytes)
                .unwrap()
                .trim()
                .parse::<i32>()
                .unwrap()
        })
    });
    // SAFETY: kill(pid, 0) observes existence without delivering a signal.
    assert_eq!(unsafe { libc::kill(child, 0) }, 0);
    cancel(&task);
    let status = finished(&task, &changes);
    assert!(status.cancelled);
    assert_eq!(status.signal, Some(libc::SIGKILL));
    // Completion joins the root wait AND both pipe readers. This fixture's
    // grandchild retains those descriptors until it exits, so EOF is its
    // termination signal. One observation suffices; no wall-clock polling.
    let state = std::process::Command::new("/bin/ps")
        .args(["-o", "stat=", "-p", &child.to_string()])
        .output()
        .unwrap();
    assert!(state.status.success() || state.status.code() == Some(1));
    let state = String::from_utf8_lossy(&state.stdout);
    assert!(
        state.trim().is_empty() || state.trim().starts_with('Z'),
        "grandchild {child} survived: {state}"
    );
    println!(
        "cancelled process group {:?}, grandchild {child} no longer running",
        task.pid()
    );
}

fn spawn_failure_and_early_cancel_always_complete() {
    let (runner, _, _, changes) = bridge(8);
    let missing = runner.spawn_command(vec!["/does/not/exist".into()], "/".into(), vec![]);
    assert!(finished(&missing, &changes).error.is_some());
    let empty = runner.spawn_command(vec![], "/".into(), vec![]);
    assert!(finished(&empty, &changes).error.is_some());
    let early = command(&runner, "sleep 3600");
    cancel(&early);
    assert!(finished(&early, &changes).cancelled);
}

// keeps: spec runtime §1 and §2 — one two-worker runtime and IO-thread guard
#[test]
fn singleton_runtime_and_ui_thread_guard() {
    let inbox = init_ui(CountWakes::default());
    assert!(is_ui_thread());
    assert!(std::panic::catch_unwind(assert_not_ui_thread).is_err());
    assert!(std::ptr::eq(handle(), handle()));
    assert_eq!(handle().metrics().num_workers(), 2);
    inbox.sender.try_send(UiMsg::Changed { source: 7 }).unwrap();
    assert!(matches!(
        inbox.messages.try_recv(),
        Some(UiMsg::Changed { source: 7 })
    ));
    let (sender, receiver) = std::sync::mpsc::channel();
    handle().spawn(async move {
        assert!(!is_ui_thread());
        assert_not_ui_thread();
        sender.send(()).unwrap();
    });
    receiver.recv_timeout(Duration::from_secs(120)).unwrap();
}

// keeps: spec runtime §3 — bounded handoff, coalesced wake and caller-visible progress
#[test]
fn a_full_queue_returns_the_original_message_without_consumption() {
    let waker = Arc::new(CountWakes::default());
    let dirty = DirtySet::new(1, waker.clone());
    let (sender, receiver) = ui_channel(1, dirty.source(0));
    sender.try_send(UiMsg::Changed { source: 7 }).unwrap();
    assert!(matches!(
        sender.try_send(UiMsg::Changed { source: 9 }),
        Err(UiMsg::Changed { source: 9 })
    ));
    assert_eq!(waker.0.load(Ordering::SeqCst), 1);
    assert!(matches!(
        receiver.try_recv(),
        Some(UiMsg::Changed { source: 7 })
    ));
    assert!(receiver.is_empty());
}

// keeps: spec runtime §3 — actual command pipes, cancellation and bounded output in the night
#[test]
#[ignore = "#327 B2: real command/pipe/process-group boundary; scripts/night-test-boundaries.sh"]
fn real_process_boundary() {
    hundred_megabyte_command_finishes_with_a_bounded_view();
    command_argv_cwd_env_stderr_and_exit_are_observable();
    cancel_kills_the_process_group_including_a_grandchild();
    spawn_failure_and_early_cancel_always_complete();
}
