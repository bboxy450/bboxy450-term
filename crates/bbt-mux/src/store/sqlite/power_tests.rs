// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use super::*;
use rusqlite::StatementStatus;
use std::{collections::BTreeMap, time::Instant};

const BEFORE: &str = "SELECT key,value FROM kv WHERE substr(key,1,length(?1))=?1 ORDER BY key";
const PREFIX: &str = "identity/issuer-history/";

#[test]
fn literal_prefix_ranges_match_rust_for_unicode_and_pattern_characters() {
    let root = tempfile::tempdir().unwrap();
    let store = SqliteStore::open(root.path()).unwrap();
    let mut expected = BTreeMap::new();
    let parts = [
        "",
        "a",
        "a/",
        "%",
        "_",
        "[",
        "*",
        "?",
        "\\",
        "\0",
        "a\0",
        "é",
        "🦊",
        "\u{d7ff}",
        "\u{e000}",
        "\u{10ffff}",
        "a\u{10ffff}",
        "\u{10ffff}\u{10ffff}",
    ];
    for prefix in parts {
        for suffix in parts {
            let key = format!("{prefix}{suffix}");
            let value = key.as_bytes().to_vec();
            store.put(&key, &value).unwrap();
            expected.insert(key, value);
        }
    }
    for prefix in parts
        .into_iter()
        .chain(["missing/", "A", "b", "a\u{10ffff}\0"])
    {
        let wanted: Vec<_> = expected
            .iter()
            .filter(|(key, _)| key.starts_with(prefix))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        assert_eq!(store.list(prefix).unwrap(), wanted, "prefix={prefix:?}");
    }
    assert_eq!(prefix_end("\u{d7ff}"), Some("\u{e000}".into()));
    assert_eq!(prefix_end("a\u{10ffff}"), Some("b".into()));
    assert_eq!(prefix_end("\u{10ffff}"), None);
    assert_eq!(prefix_end(""), None);
}

fn aged_store(root: &Path, unrelated: usize) -> SqliteStore {
    let store = SqliteStore::open(root).unwrap();
    let mut db = store.connection().unwrap();
    let tx = db.transaction().unwrap();
    tx.execute("WITH RECURSIVE rows(n) AS (SELECT 1 UNION ALL SELECT n+1 FROM rows WHERE n<?1) INSERT INTO kv SELECT 'history/' || n, zeroblob(64) FROM rows WHERE n<=?1", [unrelated as i64]).unwrap();
    tx.commit().unwrap();
    drop(db);
    store.put(&format!("{PREFIX}one"), b"first").unwrap();
    store.put(&format!("{PREFIX}two"), b"second").unwrap();
    store
}

fn cost(db: &Connection, indexed: bool) -> (usize, i32, i32) {
    let mut q = db
        .prepare(if indexed { PREFIX_RANGE } else { BEFORE })
        .unwrap();
    let mut rows = if indexed {
        q.query(params![PREFIX, prefix_end(PREFIX).unwrap()])
            .unwrap()
    } else {
        q.query([PREFIX]).unwrap()
    };
    let mut count = 0;
    while rows.next().unwrap().is_some() {
        count += 1;
    }
    drop(rows);
    (
        count,
        q.get_status(StatementStatus::VmStep),
        q.get_status(StatementStatus::FullscanStep),
    )
}

#[test]
fn namespace_reads_do_not_scan_unrelated_history() {
    let mut costs = Vec::new();
    for unrelated in [0, 10_000] {
        let root = tempfile::tempdir().unwrap();
        let store = aged_store(root.path(), unrelated);
        assert_eq!(
            store.list(PREFIX).unwrap(),
            vec![
                (format!("{PREFIX}one"), b"first".to_vec()),
                (format!("{PREFIX}two"), b"second".to_vec())
            ]
        );
        let db = store.connection().unwrap();
        let good = cost(&db, true);
        assert_eq!(good.0, 2);
        assert_eq!(good.2, 0);
        assert!(good.1 < 40, "small namespace needs bounded work: {good:?}");
        if unrelated > 0 {
            let broken = cost(&db, false);
            assert_eq!(broken.0, good.0);
            assert!(broken.2 >= unrelated as i32);
            assert!(broken.1 > good.1 * 1000);
        }
        costs.push(good);
    }
    assert_eq!(costs[0], costs[1]);
}

#[test]
#[ignore = "repeatable CPU/query-work profile; make profile-power"]
fn namespace_cpu_profile() {
    let root = tempfile::tempdir().unwrap();
    let store = aged_store(root.path(), 100_000);
    let db = store.connection().unwrap();
    for indexed in [false, true] {
        let mut samples = Vec::new();
        for _ in 0..50 {
            let start = Instant::now();
            assert_eq!(cost(&db, indexed).0, 2);
            samples.push(start.elapsed().as_secs_f64() * 1_000_000.);
        }
        samples.sort_by(f64::total_cmp);
        let work = cost(&db, indexed);
        println!(
            "power namespace {}: history=100000 samples=50 query_us_p50={:.3} query_us_p99={:.3} vm_steps={} fullscan_steps={}",
            if indexed { "after" } else { "before" },
            samples[25],
            samples[49],
            work.1,
            work.2
        );
    }
}
