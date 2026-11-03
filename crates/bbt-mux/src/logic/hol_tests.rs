// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Owned-process witness: retained history must never own the request worker.
#![allow(clippy::disallowed_methods)] // Private process/SQLite fixture.
use super::*;
use crate::store::events::{Envelope, EventLog};
use std::process::{Command, Stdio};

const ROWS: u64 = 4096;
// Existing cockpit STARTUP_TIMEOUT, bridge read timeout, and refresh child wait.
const HELLO: Duration = Duration::from_secs(30);
const MCP: Duration = Duration::from_secs(10);
const SHUTDOWN: Duration = Duration::from_secs(5);

fn history(directory: &Path) {
    let events = EventLog::open(directory).unwrap();
    let key = SigningKey::from_bytes(&[64; 32]);
    let mut seq = events.last_sequence().unwrap();
    let mut prev = events
        .at(seq)
        .unwrap()
        .map(|e| e.hash().unwrap())
        .unwrap_or_default();
    let mut db = rusqlite::Connection::open(directory.join("events.sqlite")).unwrap();
    let tx = db.transaction().unwrap();
    {
        let mut insert = tx
            .prepare("INSERT INTO events VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)")
            .unwrap();
        for _ in 0..ROWS {
            seq += 1;
            let row = Envelope::signed(
                Draft::new(
                    "fixture/retained",
                    "fixture",
                    serde_json::json!({"bytes":"x".repeat(512)}),
                ),
                seq,
                prev,
                &key,
            )
            .unwrap();
            prev = row.hash().unwrap();
            insert
                .execute(rusqlite::params![
                    seq as i64,
                    row.event.id.to_string(),
                    row.event.ts as i64,
                    row.event.subject,
                    row.event.source,
                    serde_json::to_vec(&row).unwrap(),
                    row.provenance.signer.as_slice(),
                    row.provenance.sig,
                    row.prev,
                    prev
                ])
                .unwrap();
        }
    }
    tx.execute(
        "UPDATE verification_checkpoint SET seq=?1,hash=?2 WHERE id=1",
        rusqlite::params![seq as i64, prev],
    )
    .unwrap();
    tx.commit().unwrap();
}

fn in_flight(root: &Path) -> bool {
    std::fs::read_dir(root.join("log")).is_ok_and(|entries| {
        entries
            .flatten()
            .any(|e| e.file_name().to_string_lossy().starts_with(".projection-"))
    })
}

// keeps: invariant no-chokepoints; spec mux-daemon
#[test]
#[ignore = "owned child of logic_worker_recovery_yields_to_requests_and_shutdown"]
fn recovery_child() {
    let root = PathBuf::from(std::env::var_os("BBOXY_HOL_ROOT").unwrap());
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    let config = LogicConfig {
        run_dir: root.join("run"),
        repository: root.join("repository"),
        ring: Default::default(),
    };
    std::fs::create_dir_all(config.repository.join(".bboxy")).unwrap();
    let supervisor = crate::Supervisor::start(crate::Config {
        journal_directory: Some(root.join("supervisor")),
        ..Default::default()
    })
    .unwrap();
    let initial = Logic::start(
        supervisor.connect().unwrap(),
        config.clone(),
        SigningKey::from_bytes(&[64; 32]),
        rt.handle(),
    )
    .unwrap();
    let spec = crate::LaunchSpec::new("/bin/cat", &config.repository);
    let seat = spec.seat;
    let record = SeatRecord::new(seat, Ulid::new(), "HOL fixture", "zsh");
    rt.block_on(initial.client.spawn(record, spec)).unwrap();
    initial.shutdown().unwrap();
    let directory = object_dir(&config.run_dir, "index").unwrap();
    history(&directory);
    let store = SqliteStore::open(&directory).unwrap();
    store
        .connection()
        .unwrap()
        .execute("DELETE FROM kv WHERE key='provenance/jsonl/v1'", [])
        .unwrap();
    let knowledge = config.repository.join(".bboxy");
    let start = Instant::now();
    let logic = Logic::start(
        supervisor.connect().unwrap(),
        config.clone(),
        SigningKey::from_bytes(&[64; 32]),
        rt.handle(),
    )
    .unwrap();
    // Match the post-readiness startup audit while the same retained index is
    // rebuilding. These independent readers must not occupy the request owner.
    crate::store::events::verify_history_after_ready(&config.run_dir, rt.handle());
    // The temporary directory is created only by real recovery, after chain
    // verification on the unfixed implementation. No artificial delay/fault.
    while !in_flight(&knowledge) {
        assert!(
            start.elapsed() < Duration::from_secs(120),
            "recovery never started"
        );
        std::thread::sleep(Duration::from_millis(2));
    }
    let client = logic.client.clone();
    let hello_at = Instant::now();
    let (hello, mcp) = rt.block_on(async {
        tokio::join!(
            tokio::time::timeout(HELLO, client.gui_hello()),
            tokio::time::timeout(
                MCP,
                client.control_call(
                    seat,
                    1,
                    crate::mcp::Call {
                        op_id: Ulid::new(),
                        tool: "self_status".into(),
                        args: serde_json::json!({})
                    }
                )
            )
        )
    });
    println!(
        "HOL replies {:?}: hello={} mcp={}",
        hello_at.elapsed(),
        hello.is_ok(),
        mcp.is_ok()
    );
    // This also measures the old synchronous recovery if a deadline fired;
    // joining it is bounded externally by the parent, never by the operator.
    let shutdown_at = Instant::now();
    logic.shutdown().unwrap();
    let shutdown_elapsed = shutdown_at.elapsed();
    println!(
        "HOL recovery/shutdown total={:?} shutdown={shutdown_elapsed:?}",
        start.elapsed()
    );
    assert!(
        hello.is_ok_and(|v| v.is_ok()),
        "GUI hello exceeded its existing deadline"
    );
    assert!(
        mcp.is_ok_and(|v| v.is_ok_and(|v| v["isError"] == false
            && v["structuredContent"]["actor"]["seat_id"] == seat.to_string())),
        "MCP exceeded its existing deadline"
    );
    assert!(
        shutdown_elapsed < SHUTDOWN,
        "shutdown exceeded refresh deadline"
    );
    assert!(
        in_flight(&knowledge),
        "fixture must still be rebuilding when stopped"
    );
    let cursor = || -> u64 {
        let bytes = store
            .get("provenance/jsonl/rebuild/v1")
            .unwrap()
            .expect("durable recovery cursor");
        serde_json::from_slice::<serde_json::Value>(&bytes).unwrap()["projection"]["cursor"]
            .as_u64()
            .unwrap()
    };
    let before = cursor();
    assert!(before > 0 && before < ROWS);
    let restarted = Logic::start(
        supervisor.connect().unwrap(),
        config,
        SigningKey::from_bytes(&[64; 32]),
        rt.handle(),
    )
    .unwrap();
    let resumed_at = Instant::now();
    while cursor() <= before {
        assert!(
            resumed_at.elapsed() < SHUTDOWN,
            "restart did not resume recovery"
        );
        std::thread::sleep(Duration::from_millis(2));
    }
    let shutdown_at = Instant::now();
    restarted.shutdown().unwrap();
    assert!(shutdown_at.elapsed() < SHUTDOWN);
    println!(
        "HOL durable cursor resumed from {before}; second shutdown={:?}",
        shutdown_at.elapsed()
    );
    supervisor.shutdown().unwrap();
}

// keeps: invariant no-chokepoints; spec mux-daemon
#[test]
fn logic_worker_recovery_yields_to_requests_and_shutdown() {
    let root = tempfile::Builder::new()
        .prefix("bbt-hol-")
        .tempdir_in("/tmp")
        .unwrap();
    let log_path = root.path().join("child.log");
    let log = std::fs::File::create(&log_path).unwrap();
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "logic::hol_tests::recovery_child",
            "--ignored",
            "--nocapture",
        ])
        .env("BBOXY_HOL_ROOT", root.path())
        .stdin(Stdio::null())
        .stdout(log.try_clone().unwrap())
        .stderr(log)
        .spawn()
        .unwrap();
    let start = Instant::now();
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            let log = std::fs::read_to_string(log_path).unwrap();
            println!("{log}");
            assert!(status.success(), "HOL child failed: {log}");
            break;
        }
        if start.elapsed() > Duration::from_secs(240) {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!(
                "HOL child watchdog: {}",
                std::fs::read_to_string(log_path).unwrap()
            );
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}
