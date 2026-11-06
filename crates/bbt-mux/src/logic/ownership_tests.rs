// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Deterministic real-owner interleaving, with private blocking IO held off-owner.
#![allow(clippy::disallowed_methods)]
use super::*;
use crate::{index::Index, mcp::Call, provenance::projection::WriterLease, store::Events};
use std::sync::{Mutex, Weak, mpsc as channel};

const DEADLINE: Duration = Duration::from_secs(10);
fn write(path: &str, body: &str) -> Call {
    Call {
        op_id: Ulid::new(),
        tool: "knowledge_write".into(),
        args: serde_json::json!({"path":path,"frontmatter":{"type":"note","title":path},"body":body}),
    }
}
async fn call(client: &LogicClient, seat: Ulid, request: Call) -> serde_json::Value {
    let detail = format!("{} {} seat={seat}", request.tool, request.args);
    tokio::time::timeout(DEADLINE, async {
        loop {
            let value = client.control_call(seat, 1, request.clone()).await.unwrap();
            if value["structuredContent"]["error"]["code"] != "workspace_pending" {
                return value;
            }
            tokio::time::sleep(Duration::from_millis(2)).await;
        }
    })
    .await
    .unwrap_or_else(|_| panic!("private API deadline: {detail}"))
}
async fn turn(client: &LogicClient, root: &Path) {
    let (tx, rx) = oneshot::channel();
    client
        .tx
        .send(Command::ProjectionTurn(root.to_owned(), tx))
        .await
        .unwrap();
    tokio::time::timeout(DEADLINE, rx)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
}
fn cursor(store: &SqliteStore) -> u64 {
    for key in ["provenance/jsonl/rebuild/v1", "provenance/jsonl/v1"] {
        if let Some(bytes) = store.get(key).unwrap() {
            let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            return value["projection"]["cursor"]
                .as_u64()
                .or(value["cursor"].as_u64())
                .unwrap();
        }
    }
    0
}
fn acquire(root: &Path) -> Arc<WriterLease> {
    let at = Instant::now();
    loop {
        if let Some(writer) = WriterLease::acquire(root).unwrap() {
            return writer;
        }
        assert!(at.elapsed() < DEADLINE, "writer claim leaked");
        std::thread::sleep(Duration::from_millis(2));
    }
}

// keeps: invariant no-chokepoints; spec provenance
#[test]
fn projection_writer_descriptor_lifetime_and_stale_completion() {
    let temp = tempfile::tempdir().unwrap();
    let a = temp.path().join("a");
    let b = temp.path().join("b");
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();
    let old = acquire(&a);
    let stale = Arc::downgrade(&old);
    let alias = temp.path().join("alias");
    std::os::unix::fs::symlink(&a, &alias).unwrap();
    assert!(WriterLease::acquire(&alias).unwrap().is_none());
    let other = acquire(&b);
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    let (started, arrived) = channel::channel();
    let (release, held) = channel::channel();
    let io_owner = old.clone();
    let task = rt.spawn_blocking(move || {
        let _claim = io_owner;
        started.send(()).unwrap();
        held.recv_timeout(DEADLINE).unwrap();
    });
    arrived.recv_timeout(DEADLINE).unwrap();
    task.abort();
    drop(task);
    drop(old);
    assert!(
        WriterLease::acquire(&a).unwrap().is_none(),
        "cancelling an awaiter released live IO"
    );
    release.send(()).unwrap();
    let current = acquire(&a);
    assert!(stale.upgrade().is_none());
    let mut claims = BTreeMap::from([("a".into(), Arc::downgrade(&current))]);
    knowledge_effects::release_claim(&mut claims, "a", &stale);
    assert!(
        claims["a"].ptr_eq(&Arc::downgrade(&current)),
        "A-B-A stale completion cleared the new owner"
    );
    assert!(WriterLease::acquire(&a).unwrap().is_none());
    assert!(
        WriterLease::acquire(&b).unwrap().is_none(),
        "old A completion released B"
    );
    drop(current);
    drop(other);
    let index = Index::open(&a, SqliteStore::open(&temp.path().join("index")).unwrap()).unwrap();
    assert!(
        index
            .write(
                "../refused.md",
                serde_json::json!({"id":Ulid::new(),"type":"note"}),
                "no",
                Ulid::new(),
                &SigningKey::from_bytes(&[17; 32])
            )
            .is_err()
    );
    drop(acquire(&a));
    let mut index = index;
    let writer = acquire(&a);
    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _scope = index.projection_scope(writer).unwrap();
        panic!("private projection scope unwind");
    }));
    assert!(panic.is_err());
    drop(acquire(&a));
}

// keeps: invariant no-chokepoints; spec provenance; spec mux-daemon
#[test]
#[ignore = "owned child of projection_ownership_real_io_survives_cancel_restart_and_root_aba"]
fn ownership_child() {
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
    let other = root.join("other");
    std::fs::create_dir_all(config.repository.join(".bboxy")).unwrap();
    std::fs::create_dir_all(other.join(".bboxy")).unwrap();
    let a = config.repository.join(".bboxy").canonicalize().unwrap();
    let b = other.join(".bboxy").canonicalize().unwrap();
    let supervisor = crate::Supervisor::start(crate::Config {
        journal_directory: Some(root.join("supervisor")),
        ..Default::default()
    })
    .unwrap();
    let start = || {
        Logic::start(
            supervisor.connect().unwrap(),
            config.clone(),
            SigningKey::from_bytes(&[64; 32]),
            rt.handle(),
        )
        .unwrap()
    };
    let initial = start();
    let mut seats = Vec::new();
    for directory in [&config.repository, &other] {
        let spec = crate::LaunchSpec::new("/bin/cat", directory);
        let record = SeatRecord::new(spec.seat, Ulid::new(), "private writer", "zsh");
        seats.push((record.id, record.swarm));
        rt.block_on(initial.client.spawn(record, spec)).unwrap();
    }
    initial.shutdown().unwrap();
    let registry = Registry::new(SqliteStore::open(&config.run_dir.join("account")).unwrap());
    registry
        .update_swarm_workdir(seats[1].1, Some(other.to_str().unwrap()))
        .unwrap();
    let directory = object_dir(&config.run_dir, "index").unwrap();
    super::hol_tests::history(&directory);
    let store = SqliteStore::open(&directory).unwrap();
    store
        .connection()
        .unwrap()
        .execute("DELETE FROM kv WHERE key='provenance/jsonl/v1'", [])
        .unwrap();
    let logic = start();
    let client = logic.client.clone();
    let status = || Call {
        op_id: Ulid::new(),
        tool: "knowledge_status".into(),
        args: serde_json::json!({}),
    };
    assert_eq!(
        rt.block_on(call(&client, seats[1].0, status()))["isError"],
        false
    );
    let request = write("notes/held.md", "held A");
    let original = request.op_id;
    let (started, arrived) = channel::channel::<Weak<WriterLease>>();
    let (release, held) = channel::channel();
    let held = Mutex::new(held);
    let path = a.clone();
    let count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let counted = count.clone();
    *knowledge_effects::APPLY_HOOK.lock().unwrap() = Some(Arc::new(move |root, request, claim| {
        if root == path && request.op_id == original {
            counted.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            started.send(claim.clone()).unwrap();
            held.lock()
                .unwrap()
                .recv_timeout(Duration::from_secs(30))
                .unwrap();
        }
        if request.args["body"] == "private apply panic" {
            panic!("private apply panic");
        }
    }));
    let write_client = client.clone();
    let seat = seats[0].0;
    let task = rt.spawn(async move { call(&write_client, seat, request).await });
    let stale = arrived
        .recv_timeout(DEADLINE)
        .expect("actual admitted IO never reached its hold");
    let before = cursor(&store);
    assert!(before < 4096, "large recovery finished too early");
    rt.block_on(turn(&client, &a));
    assert_eq!(
        cursor(&store),
        before,
        "same-root provenance overlapped admitted IO"
    );
    let at = Instant::now();
    rt.block_on(async {
        let bwrite = call(
            &client,
            seats[1].0,
            write("notes/independent.md", "root B stayed writable"),
        );
        let hello = tokio::time::timeout(DEADLINE, client.gui_hello());
        let mcp = call(
            &client,
            seats[0].0,
            Call {
                op_id: Ulid::new(),
                tool: "self_status".into(),
                args: serde_json::json!({}),
            },
        );
        let (written, hello, status) = tokio::join!(bwrite, hello, mcp);
        assert_eq!(written["isError"], false, "{written}");
        assert!(hello.unwrap().is_ok());
        assert_eq!(status["isError"], false);
        let read = call(
            &client,
            seats[1].0,
            Call {
                op_id: Ulid::new(),
                tool: "knowledge_fetch".into(),
                args: serde_json::json!({"locator":"notes/independent.md"}),
            },
        )
        .await;
        assert_eq!(read["isError"], false, "{read}");
        assert!(read.to_string().contains("root B stayed writable"));
    });
    assert!(at.elapsed() < DEADLINE);
    assert!(b.join("notes/independent.md").is_file());
    task.abort();
    drop(task);
    // A contended writer now waits in the bounded root FIFO. Keep it live
    // while proving unrelated requests progress, then cancel it before the
    // restart so it cannot become an unintended second admitted write.
    let queued_client = client.clone();
    let request = write("notes/not-admitted.md", "queued then cancelled");
    let mut queued = rt.spawn(async move { queued_client.control_call(seat, 1, request).await });
    rt.block_on(async {
        assert!(
            tokio::time::timeout(Duration::from_millis(100), &mut queued)
                .await
                .is_err()
        );
        assert!(
            tokio::time::timeout(DEADLINE, client.gui_hello())
                .await
                .unwrap()
                .is_ok()
        );
        assert_eq!(call(&client, seats[1].0, status()).await["isError"], false);
    });
    assert!(!a.join("notes/not-admitted.md").exists());
    queued.abort();
    rt.block_on(async {
        assert!(queued.await.unwrap_err().is_cancelled());
    });
    rt.block_on(turn(&client, &a));
    assert_eq!(cursor(&store), before);
    let at = Instant::now();
    logic.shutdown().unwrap();
    assert!(
        at.elapsed() < Duration::from_secs(5),
        "shutdown waited for blocking IO"
    );
    registry
        .update_swarm_workdir(seats[0].1, Some(other.to_str().unwrap()))
        .unwrap();
    let restarted = start();
    let client = restarted.client.clone();
    assert_eq!(
        rt.block_on(call(&client, seats[0].0, status()))["isError"],
        false
    );
    let written = rt.block_on(call(
        &client,
        seats[0].0,
        write("notes/aba-b.md", "B during old A IO"),
    ));
    assert_eq!(written["isError"], false, "{written}");
    registry
        .update_swarm_workdir(seats[0].1, Some(config.repository.to_str().unwrap()))
        .unwrap();
    assert_eq!(
        rt.block_on(call(&client, seats[0].0, status()))["isError"],
        false
    );
    rt.block_on(turn(&client, &a));
    assert_eq!(
        cursor(&store),
        before,
        "restart/A-B-A released still-running IO"
    );
    release.send(()).unwrap();
    let current = acquire(&a);
    assert!(stale.upgrade().is_none());
    let mut claims = BTreeMap::from([("a".into(), Arc::downgrade(&current))]);
    knowledge_effects::release_claim(&mut claims, "a", &stale);
    assert!(claims["a"].ptr_eq(&Arc::downgrade(&current)));
    rt.block_on(turn(&client, &a));
    assert!(WriterLease::acquire(&a).unwrap().is_none());
    drop(current);
    // Retirement may reject the old provider's write after its IO returns;
    // its lease must still survive until then, and it must never be replayed.
    assert!(!a.join("notes/not-admitted.md").exists());
    assert_eq!(
        count.load(std::sync::atomic::Ordering::SeqCst),
        1,
        "cancel/restart duplicated the operation"
    );
    let events = crate::store::events::EventLog::open(&directory).unwrap();
    assert!(
        events
            .after(4096, 1024)
            .unwrap()
            .iter()
            .filter(|row| row.event.subject.ends_with("/write_prepared")
                && row.event.payload["path"] == "notes/held.md")
            .count()
            <= 1
    );
    let mut denied = write("notes/denied.md", "refused");
    denied.args["frontmatter"]["type"] = serde_json::json!("spec");
    assert_eq!(
        rt.block_on(call(&client, seats[1].0, denied))["isError"],
        true
    );
    let panic = rt.block_on(call(
        &client,
        seats[1].0,
        write("notes/panic.md", "private apply panic"),
    ));
    assert_eq!(panic["structuredContent"]["status"], "unknown", "{panic}");
    let recovered = rt.block_on(call(
        &client,
        seats[1].0,
        write("notes/after-error.md", "confirmed after refusal/panic"),
    ));
    assert_eq!(recovered["isError"], false, "{recovered}");
    assert!(b.join("notes/after-error.md").is_file());
    // The legacy synchronous allocator can wait on this external root lock;
    // an admitted effect and the request owner must still return promptly.
    let number_lock = std::fs::File::open(&b).unwrap();
    number_lock.lock().unwrap();
    let at = Instant::now();
    rt.block_on(async {
        let write = call(
            &client,
            seats[1].0,
            write("notes/number-busy.md", "held allocator"),
        );
        let hello = tokio::time::timeout(DEADLINE, client.gui_hello());
        let status = call(
            &client,
            seats[1].0,
            Call {
                op_id: Ulid::new(),
                tool: "self_status".into(),
                args: serde_json::json!({}),
            },
        );
        let (refused, hello, status) = tokio::join!(write, hello, status);
        assert_eq!(refused["isError"], true, "{refused}");
        assert_eq!(
            refused["structuredContent"]["status"], "unknown",
            "{refused}"
        );
        assert!(hello.unwrap().is_ok());
        assert_eq!(status["isError"], false);
    });
    assert!(at.elapsed() < DEADLINE);
    assert!(!b.join("notes/number-busy.md").exists());
    drop(number_lock);
    let recovered = rt.block_on(call(
        &client,
        seats[1].0,
        write("notes/after-number-lock.md", "confirmed allocator release"),
    ));
    assert_eq!(recovered["isError"], false, "{recovered}");
    assert!(b.join("notes/after-number-lock.md").is_file());
    *knowledge_effects::APPLY_HOOK.lock().unwrap() = None;
    let at = Instant::now();
    restarted.shutdown().unwrap();
    assert!(at.elapsed() < Duration::from_secs(5));
    supervisor.shutdown().unwrap();
    println!(
        "ownership: forced same-root exclusion, root B write/read+GUI/MCP, cancelled awaiter, live IO across shutdown/restart/A-B-A, stale completion, refusal/panic cleanup; original={original}; no duplicate operation"
    );
}

// keeps: invariant no-chokepoints; spec provenance; spec mux-daemon
#[test]
fn projection_ownership_real_io_survives_cancel_restart_and_root_aba() {
    super::hol_tests::run_process("logic::ownership_tests::ownership_child", false);
}
