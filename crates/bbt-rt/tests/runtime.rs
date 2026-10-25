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

fn bridge(capacity: usize) -> (CommandRunner, UiReceiver, ViewRing<OutputChunk>) {
    let dirty = DirtySet::new(2, Arc::new(CountWakes::default()));
    let (sender, receiver) = ui_channel(8, dirty.source(0));
    let output = ViewRing::new(capacity, dirty.source(1));
    (CommandRunner::new(sender, output.clone()), receiver, output)
}

fn command(runner: &CommandRunner, script: &str) -> TaskId {
    runner.spawn_command(
        vec!["/bin/sh".into(), "-c".into(), script.into()],
        std::env::temp_dir(),
        vec![],
    )
}

fn finished(task: &TaskId) -> ExitStatus {
    let deadline = Instant::now() + Duration::from_secs(15);
    while task.status().is_none() {
        assert!(Instant::now() < deadline, "task {} timed out", task.id());
        std::thread::sleep(Duration::from_millis(2));
    }
    task.status().unwrap().clone()
}

#[test]
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
    assert!(elapsed < Duration::from_secs(1), "producer below 1M msgs/s");
}

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

#[test]
fn hundred_megabyte_command_finishes_with_a_bounded_view() {
    let (runner, messages, output) = bridge(16);
    let task = command(&runner, "dd if=/dev/zero bs=1048576 count=100 2>/dev/null");
    let status = finished(&task);
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

#[test]
fn command_argv_cwd_env_stderr_and_exit_are_observable() {
    let (runner, messages, output) = bridge(32);
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
    let status = finished(&task);
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
    // Status is published before Exited is enqueued; allow the async producer
    // to finish that last memory-only publication before checking its message.
    let deadline = Instant::now() + Duration::from_secs(1);
    loop {
        if matches!(messages.try_recv(), Some(UiMsg::Exited { task: id, status: s }) if id == task && s.code == Some(7))
        {
            break;
        }
        assert!(Instant::now() < deadline, "missing Exited notification");
        std::thread::yield_now();
    }
}

#[test]
fn cancel_kills_the_process_group_including_a_grandchild() {
    let (runner, _, output) = bridge(8);
    let task = command(&runner, "sleep 60 & printf '%s\\n' \"$!\"; wait");
    let deadline = Instant::now() + Duration::from_secs(5);
    let child = loop {
        if let Some(chunk) = output.pop() {
            break std::str::from_utf8(&chunk.bytes)
                .unwrap()
                .trim()
                .parse::<i32>()
                .unwrap();
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(2));
    };
    // SAFETY: kill(pid, 0) observes existence without delivering a signal.
    assert_eq!(unsafe { libc::kill(child, 0) }, 0);
    cancel(&task);
    let status = finished(&task);
    assert!(status.cancelled);
    assert_eq!(status.signal, Some(libc::SIGKILL));
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        // A dead orphan can briefly remain a zombie before init reaps it.
        let state = std::process::Command::new("/bin/ps")
            .args(["-o", "stat=", "-p", &child.to_string()])
            .output()
            .unwrap();
        let state = String::from_utf8_lossy(&state.stdout);
        if state.trim().is_empty() || state.trim().starts_with('Z') {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "grandchild {child} survived: {state}"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    println!(
        "cancelled process group {:?}, grandchild {child} no longer running",
        task.pid()
    );
}

#[test]
fn spawn_failure_and_early_cancel_always_complete() {
    let (runner, _, _) = bridge(8);
    let missing = runner.spawn_command(vec!["/does/not/exist".into()], "/".into(), vec![]);
    assert!(finished(&missing).error.is_some());
    let empty = runner.spawn_command(vec![], "/".into(), vec![]);
    assert!(finished(&empty).error.is_some());
    let early = command(&runner, "sleep 60");
    cancel(&early);
    assert!(finished(&early).cancelled);
}

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
    receiver.recv_timeout(Duration::from_secs(2)).unwrap();
}
