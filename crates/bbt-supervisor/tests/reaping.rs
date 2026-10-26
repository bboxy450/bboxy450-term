#![allow(clippy::disallowed_methods)] // Opt-in owned-child reaping/latency fixture.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use bbt_supervisor::{Config, Event, LaunchSpec, Supervisor, Ulid};

fn monotonic_ns() -> u64 {
    let mut value = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // SAFETY: one valid output timespec; Python uses the same CLOCK_MONOTONIC.
    assert_eq!(
        unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut value) },
        0
    );
    value.tv_sec as u64 * 1_000_000_000 + value.tv_nsec as u64
}

fn percentile(values: &[f64], p: f64) -> f64 {
    values[((values.len() as f64 * p).ceil() as usize).clamp(1, values.len()) - 1]
}

/// A producer records its marker immediately before _exit; the measured span
/// includes process teardown, supervisor observation and client event delivery.
/// It is not a kernel-only scheduling measurement.
fn exit_latencies(n: usize, verbose: bool) -> Vec<f64> {
    let mut limit = libc::rlimit {
        rlim_cur: 0,
        rlim_max: 0,
    };
    // SAFETY: valid rlimit; only this fixture process's soft fd limit changes.
    unsafe {
        assert_eq!(libc::getrlimit(libc::RLIMIT_NOFILE, &mut limit), 0);
        limit.rlim_cur = limit.rlim_max.min(65_536);
        assert_eq!(libc::setrlimit(libc::RLIMIT_NOFILE, &limit), 0);
    }
    let root = tempfile::Builder::new()
        .prefix("w12e-exit-")
        .tempdir_in("/tmp")
        .unwrap();
    let release = root.path().join("release");
    let supervisor = Supervisor::start(Config {
        max_seats: n,
        ..Default::default()
    })
    .unwrap();
    let client = supervisor.connect().unwrap();
    let mut pids = BTreeMap::new();
    for _ in 0..n {
        let mut spec = LaunchSpec::new("python3", root.path());
        spec.args = vec![
            "-c".into(),
            r#"
import os, sys, time, tty
tty.setraw(1)
os.write(1, b'READY\n')
while not os.path.exists(sys.argv[1]): time.sleep(.01)
stamp = time.clock_gettime_ns(time.CLOCK_MONOTONIC)
os.write(1, ('EXIT %d\n' % stamp).encode())
os._exit(0)
"#
            .into(),
            release.to_string_lossy().into(),
        ];
        let info = client.spawn(spec).unwrap();
        client.attach(info.seat, info.generation).unwrap();
        if verbose {
            println!("EXIT_FIXTURE owned_pid={} seat={}", info.pid, info.seat);
        }
        pids.insert(info.seat, info.pid);
    }
    let mut buffers: BTreeMap<Ulid, Vec<u8>> = BTreeMap::new();
    let mut ready = BTreeSet::new();
    let mut markers = BTreeMap::new();
    let mut observed = BTreeMap::new();
    let until = Instant::now() + Duration::from_secs(90);
    let mut released = false;
    while observed.len() < n {
        assert!(
            Instant::now() < until,
            "exit fixture timed out: ready={} exited={}/{}",
            ready.len(),
            observed.len(),
            n
        );
        if !released && ready.len() == n {
            println!("EXIT_FIXTURE ready={n}; releasing all producers");
            std::fs::write(&release, b"go").unwrap();
            released = true;
        }
        let event = match client.recv_timeout(Duration::from_millis(100)) {
            Ok(event) => event,
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
            Err(error) => panic!("exit fixture disconnected: {error}"),
        };
        match event {
            Event::Output {
                seat,
                generation,
                bytes,
                ..
            } => {
                assert_eq!(generation, 1);
                let buffer = buffers.entry(seat).or_default();
                buffer.extend(bytes);
                while let Some(end) = buffer.iter().position(|byte| *byte == b'\n') {
                    let line = String::from_utf8(buffer.drain(..=end).collect()).unwrap();
                    if line.trim() == "READY" {
                        ready.insert(seat);
                    } else if let Some(value) = line.trim().strip_prefix("EXIT ") {
                        markers.insert(seat, value.parse::<u64>().unwrap());
                    } else {
                        panic!("unexpected synthetic line: {line:?}");
                    }
                }
            }
            Event::Exited {
                seat,
                generation,
                status,
                ..
            } => {
                let received = monotonic_ns();
                assert_eq!(generation, 1);
                assert_eq!(status.code, 0);
                let marker = *markers
                    .get(&seat)
                    .expect("exit arrived before final output");
                assert!(received >= marker, "different monotonic clocks");
                assert!(
                    observed.insert(seat, (marker, received)).is_none(),
                    "duplicate exit"
                );
            }
            event => panic!("unexpected fixture event: {event:?}"),
        }
    }
    let mut latencies = Vec::with_capacity(n);
    for (seat, (marker, received)) in observed {
        let latency = (received - marker) as f64 / 1e6;
        latencies.push(latency);
        if verbose {
            println!(
                "EXIT_EVENT seat={seat} pid={} marker_ns={marker} received_ns={received} latency_ms={latency:.6}",
                pids[&seat]
            );
        }
    }
    latencies.sort_by(f64::total_cmp);
    println!(
        "EXIT_LATENCY seats={n} p50_ms={:.6} p99_ms={:.6} max_ms={:.6} all_exit_zero=true clock=CLOCK_MONOTONIC span=producer_marker_to_client_exit",
        percentile(&latencies, 0.50),
        percentile(&latencies, 0.99),
        latencies[n - 1]
    );
    supervisor.shutdown().unwrap();
    latencies
}

// keeps: spec mux-daemon.md — Ownership and failure boundaries
#[ignore = "night native boundary: exact selector in scripts/night-test-boundaries.sh"]
#[test]
fn final_output_and_exit_reach_the_owner() {
    let mut foreign = std::process::Command::new("/bin/sh")
        .args(["-c", "exit 17"])
        .spawn()
        .unwrap();
    let result = exit_latencies(8, false);
    assert_eq!(result.len(), 8);
    // A supervisor must never consume a different owner's child status.
    assert_eq!(foreign.wait().unwrap().code(), Some(17));
}

// keeps: spec mux-daemon.md — Ownership and failure boundaries
#[ignore = "night native boundary: exact selector in scripts/night-test-boundaries.sh"]
#[test]
fn stdio_eof_does_not_wait_for_a_still_running_child() {
    let root = tempfile::tempdir().unwrap();
    let closed = root.path().join("closed");
    let release = root.path().join("release");
    let supervisor = Supervisor::start(Config::default()).unwrap();
    let client = supervisor.connect().unwrap();
    struct Release(PathBuf);
    impl Drop for Release {
        fn drop(&mut self) {
            let _ = std::fs::write(&self.0, b"go");
        }
    }
    // On a failed query, release before Supervisor::drop joins the actor.
    let release_on_drop = Release(release.clone());
    let mut spec = LaunchSpec::new("python3", root.path());
    spec.args = vec![
        "-c".into(),
        r#"
import os, sys, time
os.close(0); os.close(1); os.close(2)
open(sys.argv[1], 'w').close()
while not os.path.exists(sys.argv[2]): time.sleep(.01)
os._exit(0)
"#
        .into(),
        closed.to_string_lossy().into(),
        release.to_string_lossy().into(),
    ];
    let info = client.spawn(spec).unwrap();
    client.attach(info.seat, 1).unwrap();
    let until = Instant::now() + Duration::from_secs(30);
    while !closed.exists() {
        assert!(Instant::now() < until, "child never closed its stdio");
        std::thread::sleep(Duration::from_millis(1));
    }
    for _ in 0..10 {
        let bbt_supervisor::Reply::Seats { seats } = client
            .request(bbt_supervisor::Action::ListOwnedSeats)
            .unwrap()
        else {
            panic!()
        };
        assert!(seats[0].exited.is_none(), "EOF invented process exit");
    }
    drop(release_on_drop);
    loop {
        assert!(Instant::now() < until, "exit after closed stdio was lost");
        if let Ok(Event::Exited { seat, status, .. }) =
            client.recv_timeout(Duration::from_millis(100))
        {
            assert_eq!(seat, info.seat);
            assert_eq!(status.code, 0);
            break;
        }
    }
    supervisor.shutdown().unwrap();
}

// keeps: spec mux-daemon.md — Ownership and failure boundaries
#[test]
#[ignore = "W1.2e before/after: GATE_SEATS=967 cargo test --release -p bboxy450-harness-host --test reaping -- --ignored --nocapture"]
fn exit_events_at_machine_scale() {
    let n = std::env::var("GATE_SEATS")
        .map(|n| n.parse().unwrap())
        .unwrap_or(967);
    assert!((1..=1024).contains(&n));
    exit_latencies(n, true);
}
