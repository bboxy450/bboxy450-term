// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
#![allow(clippy::disallowed_methods, reason = "private filesystem witnesses")]
use bboxy450_canvas::{store::Store, *};
use std::{fs::OpenOptions, io::Write};
fn element(id: &str) -> Element {
    Element {
        id: id.into(),
        tags: vec![],
        kind: Some(Kind::Service),
        emphasis: false,
        status: Status::Normal,
        style: None,
        content: Content::Shape {
            bounds: Bounds {
                origin: [0.; 2],
                size: [100., 50.],
            },
            shape: Shape::Box,
            label: id.into(),
        },
    }
}
fn setup() -> (tempfile::TempDir, Store, record::Record) {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::new(dir.path().join("canvases"));
    let snapshot = store
        .create(
            &ulid::Ulid::new().to_string(),
            CanvasRecord {
                title: "Kept".into(),
                swarm: "swarm".into(),
                tags: vec!["design".into()],
                creator: "seat:a".into(),
                task: Some("task".into()),
            },
        )
        .unwrap();
    (dir, store, snapshot.record)
}
#[test]
fn canvas_store_restart_and_three_batches() {
    let (dir, store, record) = setup();
    for (batch, range) in [("first", 0..3), ("second", 3..7), ("third", 7..10)] {
        let ops: Vec<_> = range
            .map(|i| Action::Add {
                element: element(&format!("element-{i}")),
            })
            .collect();
        store.append(&record, batch, "seat:a", None, &ops).unwrap();
    }
    let before = store.load(&record).unwrap();
    drop(store);
    let restarted = Store::new(dir.path().join("canvases"));
    let listed = restarted.records().unwrap();
    assert_eq!(listed.as_slice(), std::slice::from_ref(&record));
    let after = restarted.load(&listed[0]).unwrap();
    assert_eq!(after.canvas.elements().count(), 10);
    assert_eq!(after.seq, 10);
    assert_eq!(before.canvas.digest(), after.canvas.digest());
    assert_eq!(after.batches.len(), 3);
    assert_eq!(after.record.tags, ["design"]);
    assert!(
        std::fs::read_to_string(restarted.directory().join(format!("{}.md", record.slug)))
            .unwrap()
            .contains("type: canvas")
    );
}
#[test]
fn canvas_store_duplicate_and_late_batches() {
    let (dir, store, record) = setup();
    let a = [Action::Add {
        element: element("a"),
    }];
    let added = store.append(&record, "add", "seat:a", Some(0), &a).unwrap();
    assert!(!added.stale);
    assert!(
        store
            .append(&record, "add", "seat:a", Some(0), &a)
            .unwrap()
            .duplicate
    );
    let removed = store
        .append(
            &record,
            "remove",
            "operator",
            Some(0),
            &[Action::Remove { id: "a".into() }],
        )
        .unwrap();
    assert!(removed.stale);
    let late = [
        Action::Change {
            element: element("a"),
        },
        Action::Remove { id: "a".into() },
    ];
    let changed = store
        .append(&record, "late", "seat:b", Some(1), &late)
        .unwrap();
    assert_eq!(changed.snapshot.seq, 4);
    assert_eq!(
        removed.snapshot.canvas.digest(),
        changed.snapshot.canvas.digest()
    );
    let restarted = Store::new(dir.path().join("canvases"));
    let duplicate = restarted
        .append(&record, "late", "seat:b", Some(1), &late)
        .unwrap();
    assert!(duplicate.duplicate);
    assert_eq!(duplicate.snapshot.seq, 4);
    assert_eq!(duplicate.snapshot.batches.len(), 3);
    assert!(duplicate.snapshot.canvas.get("a").is_none());
    assert!(
        restarted
            .append(&record, "late", "seat:b", None, &late)
            .unwrap_err()
            .to_string()
            .contains("op_id_reused")
    );
}
#[test]
fn canvas_store_torn_tail_dropped_reported_and_repaired_without_partial_batch() {
    let (_dir, store, record) = setup();
    store
        .append(
            &record,
            "first",
            "seat:a",
            None,
            &[Action::Add {
                element: element("a"),
            }],
        )
        .unwrap();
    let path = store.directory().join(format!("{}.log.jsonl", record.slug));
    let mut file = OpenOptions::new().append(true).open(&path).unwrap();
    let tail = b"{\"batch_id\":\"torn\",\"ops\":[";
    file.write_all(tail).unwrap();
    file.sync_all().unwrap();
    let loaded = store.load(&record).unwrap();
    assert_eq!(loaded.dropped_tail_bytes, tail.len());
    assert_eq!(loaded.seq, 1);
    let result = store
        .append(
            &record,
            "next",
            "seat:a",
            None,
            &[Action::Add {
                element: element("b"),
            }],
        )
        .unwrap();
    assert_eq!(result.snapshot.dropped_tail_bytes, tail.len());
    assert_eq!(store.load(&record).unwrap().dropped_tail_bytes, 0);
    assert_eq!(store.load(&record).unwrap().seq, 2);
    // Interior corruption is never treated as a torn tail.
    OpenOptions::new()
        .append(true)
        .open(path)
        .unwrap()
        .write_all(b"bad-json\n")
        .unwrap();
    assert!(
        store
            .load(&record)
            .unwrap_err()
            .to_string()
            .contains("canvas batch line 3")
    );
}
#[test]
fn canvas_store_batch_atomic_validation_and_per_canvas_lock() {
    let (_dir, store, record) = setup();
    let bad = [
        Action::Add {
            element: element("a"),
        },
        Action::Remove {
            id: "never-added".into(),
        },
    ];
    assert!(store.append(&record, "bad", "seat:a", None, &bad).is_err());
    assert_eq!(store.load(&record).unwrap().seq, 0);
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(store.directory().join(format!("{}.log.jsonl", record.slug)))
        .unwrap();
    file.try_lock().unwrap();
    assert!(
        store
            .append(&record, "locked", "seat:a", None, &bad[..1])
            .unwrap_err()
            .to_string()
            .contains("canvas_busy")
    );
    file.unlock().unwrap();
    assert_eq!(
        store
            .append(&record, "locked", "seat:a", None, &bad[..1])
            .unwrap()
            .snapshot
            .seq,
        1
    );
}
#[test]
fn canvas_store_caps_name_cap_and_keep_log_unchanged() {
    let (_dir, store, record) = setup();
    let too_many = vec![Action::Remove { id: "a".into() }; store::MAX_BATCH_OPS + 1];
    assert!(
        store
            .append(&record, "big", "seat:a", None, &too_many)
            .unwrap_err()
            .to_string()
            .contains("MAX_BATCH_OPS")
    );
    for i in (0..MAX_ELEMENTS).step_by(store::MAX_BATCH_OPS) {
        let batch: Vec<_> = (i..(i + store::MAX_BATCH_OPS).min(MAX_ELEMENTS))
            .map(|j| Action::Add {
                element: element(&j.to_string()),
            })
            .collect();
        store
            .append(&record, &i.to_string(), "seat:a", None, &batch)
            .unwrap();
    }
    assert!(
        store
            .append(
                &record,
                "over",
                "seat:a",
                None,
                &[Action::Add {
                    element: element("over")
                }]
            )
            .unwrap_err()
            .to_string()
            .contains("MAX_ELEMENTS")
    );
    assert_eq!(store.load(&record).unwrap().seq, MAX_ELEMENTS as u64);
}
#[test]
fn canvas_store_create_retry_and_strict_semantic_codec() {
    let (_dir, store, record) = setup();
    let metadata = CanvasRecord {
        title: record.title.clone(),
        swarm: record.swarm.clone(),
        tags: record.tags.clone(),
        creator: record.creator.clone(),
        task: record.task.clone(),
    };
    assert_eq!(store.create(&record.id, metadata).unwrap().record, record);
    assert_eq!(
        record::decode(&record::encode(&record).unwrap()).unwrap(),
        record
    );
    let mut json = serde_json::to_value(Action::Add {
        element: element("a"),
    })
    .unwrap();
    json["element"]["colour"] = "red".into();
    assert!(serde_json::from_value::<Action>(json).is_err());
}

#[test]
fn canvas_store_colliding_numbers_keep_separate_logs_and_create_retries() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::new(dir.path().join("canvases"));
    let metadata = CanvasRecord {
        title: "Collision".into(),
        swarm: "swarm".into(),
        tags: vec![],
        creator: "seat:a".into(),
        task: None,
    };
    let first = store
        .create(&ulid::Ulid::from(1_u128).to_string(), metadata.clone())
        .unwrap();
    let id = ulid::Ulid::from(1_u128 + (1_u128 << 53)).to_string();
    let second = store.create(&id, metadata.clone()).unwrap();
    assert_ne!(first.record.number, second.record.number);
    store
        .append(
            &second.record,
            "only-second",
            "seat:a",
            None,
            &[Action::Add {
                element: element("a"),
            }],
        )
        .unwrap();
    assert_eq!(store.load(&first.record).unwrap().seq, 0);
    assert_eq!(store.create(&id, metadata).unwrap().seq, 1);
}

#[cfg(unix)]
#[test]
fn canvas_store_refuses_symlink_log_and_temporary_record_writes() {
    let dir = tempfile::tempdir().unwrap();
    let directory = dir.path().join("canvases");
    std::fs::create_dir(&directory).unwrap();
    let store = Store::new(&directory);
    let target = dir.path().join("unrelated");
    std::fs::write(&target, b"preserve").unwrap();
    let metadata = CanvasRecord {
        title: "Safe".into(),
        swarm: "swarm".into(),
        tags: vec![],
        creator: "seat:a".into(),
        task: None,
    };
    let id = ulid::Ulid::from(1_u128).to_string();
    let log = directory.join("canvas-1.log.jsonl");
    std::os::unix::fs::symlink(&target, &log).unwrap();
    assert!(store.create(&id, metadata.clone()).is_err());
    std::fs::remove_file(log).unwrap();
    std::os::unix::fs::symlink(&target, directory.join(format!(".canvas-1-{id}.tmp"))).unwrap();
    assert!(store.create(&id, metadata).is_err());
    assert_eq!(std::fs::read(target).unwrap(), b"preserve");
}

#[test]
fn canvas_store_close_retires_the_record_and_its_log_without_deleting() {
    let (dir, store, record) = setup();
    store
        .append(
            &record,
            "op-1",
            "author",
            None,
            &[Action::Add {
                element: element("a"),
            }],
        )
        .unwrap();
    assert_eq!(store.records().unwrap().len(), 1);
    store.close(&record).unwrap();
    assert!(store.records().unwrap().is_empty());
    let closed = dir.path().join("canvases/closed");
    assert!(closed.join(format!("{}.md", record.slug)).is_file());
    assert!(
        closed
            .join(format!("canvas-{}.log.jsonl", record.number))
            .is_file()
    );
    // A retry after a lost reply changes nothing.
    store.close(&record).unwrap();
    assert!(store.records().unwrap().is_empty());
    // Restore brings it back with its drawing; restoring twice changes nothing.
    store.restore(&record).unwrap();
    assert_eq!(store.records().unwrap().len(), 1);
    assert_eq!(store.load(&record).unwrap().canvas.elements().count(), 1);
    store.restore(&record).unwrap();
    store.close(&record).unwrap();
    // The number is not reused: the next canvas is a new one.
    let next = store
        .create(
            &ulid::Ulid::new().to_string(),
            CanvasRecord {
                title: "Next".into(),
                swarm: record.swarm.clone(),
                tags: vec![],
                creator: record.creator.clone(),
                task: None,
            },
        )
        .unwrap();
    assert_ne!(next.record.id, record.id);
}
// State: one batch that lists a frame before the members it groups. Safety:
// the log keeps the applied order, so the same canvas loads again after a
// restart, and a retry of the listed order is the same request.
#[test]
fn canvas_store_frame_before_members_persists_and_retries_as_one_request() {
    let (dir, store, record) = setup();
    let frame = Element {
        content: Content::Frame {
            bounds: Bounds {
                origin: [-10.; 2],
                size: [200., 100.],
            },
            name: "Group".into(),
            members: vec!["a".into(), "b".into()],
        },
        ..element("group")
    };
    let ops = vec![
        Action::Add { element: frame },
        Action::Add {
            element: element("a"),
        },
        Action::Add {
            element: element("b"),
        },
    ];
    let first = store
        .append(&record, "batch", "seat:a", Some(0), &ops)
        .unwrap();
    assert!(!first.duplicate);
    assert_eq!(first.snapshot.seq, 3);
    let again = store
        .append(&record, "batch", "seat:a", Some(0), &ops)
        .unwrap();
    assert!(again.duplicate);
    assert!(
        store
            .append(&record, "batch", "seat:a", Some(0), &ops[1..])
            .is_err(),
        "a different set is a reused operation id"
    );
    drop(store);
    let restarted = Store::new(dir.path().join("canvases"));
    let loaded = restarted.load(&record).unwrap();
    assert_eq!(loaded.canvas.digest(), first.snapshot.canvas.digest());
    assert_eq!(loaded.seq, 3);
    assert!(
        matches!(loaded.batches[0].ops[0].action, Action::Add { ref element } if element.id == "a")
    );
    assert!(
        matches!(loaded.batches[0].ops[2].action, Action::Add { ref element } if element.id == "group")
    );
}
