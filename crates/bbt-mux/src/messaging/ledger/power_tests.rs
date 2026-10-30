// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! The idle pump must not reread all retained publications ten times a second.
use super::*;
use rusqlite::StatementStatus;
use std::time::Instant;

fn historical_store(root: &std::path::Path, count: usize) -> SqliteStore {
    let store = SqliteStore::open(root).unwrap();
    // Install the pre-index schema: opening a real old ledger must migrate it.
    let mut db = store.connection().unwrap();
    db.execute_batch("CREATE TABLE msg_bus_outbox(id TEXT PRIMARY KEY,draft BLOB NOT NULL,sent INTEGER NOT NULL DEFAULT 0)").unwrap();
    let tx = db.transaction().unwrap();
    let draft = Draft::new(
        "power/fixture",
        "test",
        serde_json::json!({"body":"x".repeat(1024)}),
    );
    tx.execute(
        "WITH RECURSIVE rows(n) AS (SELECT 1 UNION ALL SELECT n+1 FROM rows WHERE n<?1)
         INSERT INTO msg_bus_outbox(id,draft,sent) SELECT 'sent-' || n,?2,1 FROM rows WHERE n<=?1",
        params![count as i64, serde_json::to_vec(&draft).unwrap()],
    )
    .unwrap();
    tx.commit().unwrap();
    store
}

fn query_cost(db: &rusqlite::Connection, query: &str) -> (i32, i32) {
    let mut q = db.prepare(query).unwrap();
    let mut rows = q.query([]).unwrap();
    while rows.next().unwrap().is_some() {}
    drop(rows);
    (
        q.get_status(StatementStatus::VmStep),
        q.get_status(StatementStatus::FullscanStep),
    )
}

#[test]
fn recipient_busy_lookup_does_not_scan_other_recipients_or_finished_attempts() {
    let root = tempfile::tempdir().unwrap();
    let ledger = Ledger::open(SqliteStore::open(root.path()).unwrap(), Ulid::new()).unwrap();
    let db = ledger.store.connection().unwrap();
    db.execute_batch(
        "WITH RECURSIVE rows(n) AS (SELECT 1 UNION ALL SELECT n+1 FROM rows WHERE n<10000)
        INSERT INTO msg_attempts(id,recipient,data,outcome)
        SELECT 'attempt-' || n,CASE WHEN n%2=0 THEN 'target' ELSE 'other' END,x'7b7d',
            CASE WHEN n%2=0 THEN x'7b7d' ELSE NULL END FROM rows;",
    )
    .unwrap();
    let query =
        "SELECT EXISTS(SELECT 1 FROM msg_attempts WHERE recipient='target' AND outcome IS NULL)";
    let cost = query_cost(&db, query);
    assert_eq!(cost.1, 0, "busy lookup scanned retained attempts: {cost:?}");
    assert!(
        cost.0 < 32,
        "absent recipient must be an indexed lookup: {cost:?}"
    );
    let broken = query_cost(&db, &query.replace(" WHERE", " NOT INDEXED WHERE"));
    assert!(
        broken.1 >= 9999,
        "broken control did not scan history: {broken:?}"
    );
    db.execute(
        "INSERT INTO msg_attempts VALUES('active','target',x'7b7d',NULL)",
        [],
    )
    .unwrap();
    assert!(
        db.query_row(query, [], |row| row.get::<_, bool>(0))
            .unwrap()
    );
    assert_eq!(query_cost(&db, query).1, 0);
}

#[test]
fn pending_outbox_work_is_independent_of_sent_history() {
    let mut costs = Vec::new();
    for count in [0, 10_000] {
        let root = tempfile::tempdir().unwrap();
        let store = historical_store(root.path(), count);
        let ledger = Ledger::open(store, Ulid::new()).unwrap();
        assert!(ledger.bus_outbox().unwrap().is_empty());
        let db = ledger.store.connection().unwrap();
        let cost = query_cost(&db, BUS_OUTBOX_QUERY);
        assert_eq!(cost.1, 0, "idle must not scan archived rows");
        assert!(cost.0 < 32, "empty indexed poll is bounded: {cost:?}");
        costs.push(cost);
        if count > 0 {
            // Deliberately broken performance control: the exact old access
            // path must demonstrably read the history this gate protects.
            let unindexed = BUS_OUTBOX_QUERY.replace(" WHERE", " NOT INDEXED WHERE");
            let broken = query_cost(&db, &unindexed);
            assert!(broken.1 >= count as i32 - 1, "negative control: {broken:?}");
            assert!(broken.0 > cost.0 * 1000);
        }
    }
    assert_eq!(costs[0], costs[1]);
}

#[test]
fn pending_outbox_migration_retains_fifo_batch_and_recovery() {
    let root = tempfile::tempdir().unwrap();
    let swarm = Ulid::new();
    let ledger = Ledger::open(historical_store(root.path(), 1000), swarm).unwrap();
    let expected: Vec<_> = (1..=130).rev().map(Ulid::from).collect();
    {
        let mut db = ledger.store.connection().unwrap();
        let tx = db.transaction().unwrap();
        for (i, id) in expected.iter().enumerate() {
            let mut draft = Draft::new("power/fixture", "test", serde_json::json!({"n":i}));
            draft.id = *id;
            tree::queue_event(&tx, &draft).unwrap();
        }
        tx.commit().unwrap();
    }
    let page = ledger.bus_outbox().unwrap();
    assert_eq!(
        page.iter().map(|draft| draft.id).collect::<Vec<_>>(),
        expected[..128]
    );
    // A lost publication acknowledgement is redelivered across owner restart.
    drop(ledger);
    let ledger = Ledger::open(SqliteStore::open(root.path()).unwrap(), swarm).unwrap();
    assert_eq!(ledger.bus_outbox().unwrap(), page);
    for draft in page {
        ledger.bus_sent(draft.id).unwrap();
        ledger.bus_sent(draft.id).unwrap();
    }
    assert_eq!(
        ledger
            .bus_outbox()
            .unwrap()
            .iter()
            .map(|draft| draft.id)
            .collect::<Vec<_>>(),
        expected[128..]
    );
    for id in &expected[128..] {
        ledger.bus_sent(*id).unwrap();
    }
    drop(ledger);
    let ledger = Ledger::open(SqliteStore::open(root.path()).unwrap(), swarm).unwrap();
    assert!(ledger.bus_outbox().unwrap().is_empty());
    let db = ledger.store.connection().unwrap();
    let retained: i64 = db
        .query_row("SELECT COUNT(*) FROM msg_bus_outbox", [], |row| row.get(0))
        .unwrap();
    assert_eq!(retained, 1130, "optimization never prunes durable history");
    let pending_index: i64 = db
        .query_row(
            "SELECT COUNT(*) FROM msg_bus_outbox INDEXED BY msg_bus_pending WHERE sent=0",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(pending_index, 0);
}

#[test]
#[ignore = "repeatable CPU/query-work profile; make profile-power"]
fn pending_outbox_cpu_profile() {
    let root = tempfile::tempdir().unwrap();
    let ledger = Ledger::open(historical_store(root.path(), 100_000), Ulid::new()).unwrap();
    let db = ledger.store.connection().unwrap();
    let unindexed = BUS_OUTBOX_QUERY.replace(" WHERE", " NOT INDEXED WHERE");
    for (label, query) in [("before", unindexed.as_str()), ("after", BUS_OUTBOX_QUERY)] {
        let mut samples = Vec::new();
        for _ in 0..50 {
            let start = Instant::now();
            let cost = query_cost(&db, query);
            samples.push(start.elapsed().as_secs_f64() * 1_000_000.);
            if label == "after" {
                assert_eq!(cost.1, 0);
            }
        }
        samples.sort_by(f64::total_cmp);
        println!(
            "power outbox {label}: history=100000 samples=50 query_us_p50={:.3} query_us_p99={:.3} vm_steps={} fullscan_steps={}",
            samples[25],
            samples[49],
            query_cost(&db, query).0,
            query_cost(&db, query).1
        );
    }
}
