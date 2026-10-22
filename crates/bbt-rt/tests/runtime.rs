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
