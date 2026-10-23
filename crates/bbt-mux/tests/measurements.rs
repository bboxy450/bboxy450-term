//! Opt-in, finite W2.2 measurements; no provider or user harness is touched.
#![allow(clippy::disallowed_methods)] // Disposable IO/process test harness.
mod common;
use std::time::Instant;

use bbt_mux::bus::{Broker, Consumer};
use bbt_mux::logic::{Logic, LogicConfig};
use bbt_mux::registry::SeatRecord;
use bbt_mux::store::events::Draft;
use bbt_mux::{LaunchSpec, Supervisor, Ulid};
use ed25519_dalek::SigningKey;
use petgraph::{
    graph::{DiGraph, NodeIndex},
    visit::Dfs,
};
use serde_json::json;

#[test]
#[ignore = "finite synthetic 10k-document / 100k-link measurement"]
fn recursive_cte_vs_petgraph_and_thousand_subscriber_fanout() {
    let temp = tempfile::tempdir().unwrap();
    let mut db = rusqlite::Connection::open(temp.path().join("graph.sqlite")).unwrap();
    db.execute_batch("PRAGMA journal_mode=WAL; CREATE TABLE docs(id INTEGER PRIMARY KEY,title TEXT); CREATE TABLE links(src INTEGER,dst INTEGER); CREATE INDEX links_src ON links(src);").unwrap();
    let mut graph = DiGraph::<(), ()>::new();
    for _ in 0..10_000 {
        graph.add_node(());
    }
    let tx = db.transaction().unwrap();
    {
        let mut docs = tx.prepare("INSERT INTO docs VALUES(?1,?2)").unwrap();
        let mut edge = tx.prepare("INSERT INTO links VALUES(?1,?2)").unwrap();
        for src in 0..10_000 {
            docs.execute(rusqlite::params![src, format!("synthetic document {src}")])
                .unwrap();
            for offset in 1..=10 {
                let dst = (src + offset) % 10_000;
                edge.execute([src, dst]).unwrap();
                graph.add_edge(
                    NodeIndex::new(src as usize),
                    NodeIndex::new(dst as usize),
                    (),
                );
            }
        }
    }
    tx.commit().unwrap();
    println!(
        "fixture: {} documents, {} directed links; SQLite {}",
        graph.node_count(),
        graph.edge_count(),
        rusqlite::version()
    );
    for trial in 1..=3 {
        let start = Instant::now();
        let count:i64=db.query_row("WITH RECURSIVE reachable(id) AS (VALUES(0) UNION SELECT dst FROM links JOIN reachable ON links.src=reachable.id) SELECT count(*) FROM reachable",[],|r|r.get(0)).unwrap();
        let cte = start.elapsed();
        let start = Instant::now();
        let mut dfs = Dfs::new(&graph, NodeIndex::new(0));
        let mut visited = 0;
        while dfs.next(&graph).is_some() {
            visited += 1;
        }
        let memory = start.elapsed();
        assert_eq!(count, 10_000);
        assert_eq!(visited, 10_000);
        println!(
            "trial {trial}: recursive CTE {:?}; petgraph DFS {:?}; both reach {visited}",
            cte, memory
        );
    }
    let mut broker =
        Broker::open(&temp.path().join("bus"), SigningKey::from_bytes(&[30; 32])).unwrap();
    broker
        .publish(
            "index",
            Draft::new("doc/spec/created", "fixture", json!({})),
        )
        .unwrap();
    let mut subscribers = Vec::new();
    for _ in 0..1000 {
        subscribers.push(
            broker
                .subscribe(Consumer::gui("doc/spec/modified"))
                .unwrap(),
        );
    }
    let start = Instant::now();
    broker
        .publish(
            "index",
            Draft::new("doc/spec/modified", "fixture", json!({"revision":1})),
        )
        .unwrap();
    let fanout = start.elapsed();
    for id in subscribers {
        assert_eq!(broker.poll(&id, 1, 1).unwrap().len(), 1);
    }
    println!(
        "one durable doc edit -> 1000 live consumers: {:?}; no harness wake/load claim",
        fanout
    );
}

fn file_count() -> usize {
    // SAFETY: F_GETFD inspects validity only; it does not close or modify an fd.
    (0..65_536)
        .filter(|fd| unsafe { libc::fcntl(*fd, libc::F_GETFD) } != -1)
        .count()
}

#[test]
#[ignore = "finite 200-live-seat descriptor measurement"]
fn file_descriptors_at_200_live_seats_and_sqlite_objects() {
    let temp = tempfile::tempdir().unwrap();
    let mut limit = std::mem::MaybeUninit::<libc::rlimit>::uninit();
    // SAFETY: getrlimit writes one valid rlimit to the supplied pointer.
    assert_eq!(
        unsafe { libc::getrlimit(libc::RLIMIT_NOFILE, limit.as_mut_ptr()) },
        0
    );
    // SAFETY: successful getrlimit initialized the value.
    let limit = unsafe { limit.assume_init() };
    println!(
        "RLIMIT_NOFILE before: soft={} hard={}",
        limit.rlim_cur, limit.rlim_max
    );
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    let supervisor = Supervisor::start(common::capture_config(temp.path())).unwrap();
    let repo = temp.path().join(".bboxy");
    std::fs::create_dir(&repo).unwrap();
    let logic = Logic::start(
        supervisor.connect().unwrap(),
        LogicConfig {
            ring: Default::default(),
            run_dir: temp.path().join("run"),
            repository: repo,
        },
        SigningKey::from_bytes(&[31; 32]),
        runtime.handle(),
    )
    .unwrap();
    let baseline = file_count();
    let start = Instant::now();
    let swarm = Ulid::new();
    for i in 0..200 {
        let mut launch = LaunchSpec::new("/bin/sleep", std::env::temp_dir());
        launch.args = vec!["120".into()];
        let record = SeatRecord::new(launch.seat, swarm, format!("fixture-{i}"), "sleep-fixture");
        runtime
            .block_on(logic.client.spawn(record, launch))
            .unwrap();
    }
    let count = file_count();
    println!(
        "200 live seats + 200 Unix listeners + per-object SQLite with operation-scoped connections: baseline={baseline}, current={count}, delta={}, soft={}, hard={}, bootstrap={:?}",
        count - baseline,
        limit.rlim_cur,
        limit.rlim_max,
        start.elapsed()
    );
    assert!(count >= baseline + 400);
    assert!((count as u64) < limit.rlim_cur);
    logic.shutdown().unwrap();
    supervisor.shutdown().unwrap();
}
