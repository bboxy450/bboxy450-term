// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
#![allow(clippy::disallowed_methods)] // Disposable IO/process test harness.
use std::time::Duration;

use bbt_mux::bus::{Ack, Broker, Consumer, Delivery, Start, matches, validate_subject};
use bbt_mux::index::{Document, FileWatch, GraphKind, Index, wikilinks};
use bbt_mux::registry::{Lifecycle, Position, Registry, SeatRecord};
use bbt_mux::store::{
    Blobs, Events, Store,
    blobs::LocalBlobs,
    events::{Draft, EventLog},
    sqlite::SqliteStore,
};
use ed25519_dalek::SigningKey;
use serde_json::json;
use ulid::Ulid;

// keeps: spec storage
#[test]
fn sqlite_objects_blobs_and_alarms_round_trip_across_reopen() {
    let temp = tempfile::tempdir().unwrap();
    let store = SqliteStore::open(temp.path()).unwrap();
    store.put("literal%_key", b"value").unwrap();
    store.alarm("wake", 1000).unwrap();
    drop(store);
    let store = SqliteStore::open(temp.path()).unwrap();
    assert_eq!(store.get("literal%_key").unwrap().unwrap(), b"value");
    assert_eq!(store.list("literal%_").unwrap().len(), 1);
    assert!(store.due_alarms(999).unwrap().is_empty());
    assert_eq!(store.due_alarms(1000).unwrap(), vec!["wake"]);
    let blobs = LocalBlobs::open(temp.path()).unwrap();
    let bytes = vec![b'R'; 1024 * 1024];
    let hash = blobs.put(&bytes).unwrap();
    assert_eq!(blobs.get(&hash).unwrap(), bytes);
    assert_eq!(blobs.put(&bytes).unwrap(), hash);
    assert_eq!(blobs.list().unwrap(), vec![hash.clone()]);
    assert!(blobs.get("../../escape").is_err());
    std::fs::write(
        temp.path().join("blobs").join(format!("{hash}.zst")),
        b"torn",
    )
    .unwrap();
    assert!(blobs.get(&hash).is_err());
}

// keeps: spec storage
#[test]
fn signed_events_dedupe_forever_reject_conflicts_tampering_and_torn_database() {
    let temp = tempfile::tempdir().unwrap();
    let key = SigningKey::from_bytes(&[7; 32]);
    let log = EventLog::open(temp.path()).unwrap();
    let event = Draft::new("seat/one/state", "fixture", json!({"state":"idle"}));
    let first = log.append(event.clone(), &key).unwrap();
    assert_eq!(log.append(event.clone(), &key).unwrap(), first);
    let mut conflict = event;
    conflict.payload = json!({"state":"working"});
    assert!(log.append(conflict, &key).is_err());
    let second = log
        .append(Draft::new("seat/one/title", "fixture", json!("two")), &key)
        .unwrap();
    assert_eq!(second.seq, 2);
    assert_eq!(second.prev, first.hash().unwrap());
    assert_eq!(log.verify_chain().unwrap(), 2);
    drop(log);
    let log = EventLog::open(temp.path()).unwrap();
    assert_eq!(log.after(0, 128).unwrap().len(), 2);
    let mut forged = first.clone();
    forged.event.source = "other".into();
    assert!(forged.verify().is_err());
    let db = rusqlite::Connection::open(temp.path().join("events.sqlite")).unwrap();
    assert!(db.execute("UPDATE events SET subject='x'", []).is_err());
    db.execute_batch("DROP TRIGGER immutable_update").unwrap();
    db.execute(
        "UPDATE events SET payload=?1 WHERE seq=1",
        [serde_json::to_vec(&forged).unwrap()],
    )
    .unwrap();
    drop(db);
    assert!(EventLog::open(temp.path()).is_err());
    let torn = tempfile::tempdir().unwrap();
    std::fs::write(
        torn.path().join("events.sqlite"),
        b"SQLite format 3\0partial",
    )
    .unwrap();
    assert!(EventLog::open(torn.path()).is_err());
}

// keeps: spec storage
#[test]
fn grammar_matches_tokens_and_rejects_mid_path_tail_wildcards() {
    assert!(matches("seat/*/state", "seat/abc/state"));
    assert!(!matches("seat/*/state", "seat/a/b/state"));
    assert!(matches("doc/>", "doc/a/modified"));
    assert!(!matches("doc/>", "doc"));
    for bad in ["doc//x", "doc/>/x", "doc/a*", "doc/\n"] {
        assert!(validate_subject(bad, true).is_err());
    }
    assert!(validate_subject("doc/*", false).is_err());
}

// keeps: spec storage
#[test]
fn independent_streams_latest_snapshots_queue_gaps_and_nonretained_output() {
    let temp = tempfile::tempdir().unwrap();
    let key = SigningKey::from_bytes(&[9; 32]);
    let mut broker = Broker::open(temp.path(), key.clone()).unwrap();
    let mut config = Consumer::gui("seat/>");
    config.queue_max = 2;
    let id = broker.subscribe(config).unwrap();
    for n in 0..5 {
        broker
            .publish("seat-a", Draft::new("seat/a/state", "test", json!(n)))
            .unwrap();
    }
    let latest = broker.poll(&id, 8, 100).unwrap();
    assert_eq!(latest.len(), 1);
    assert!(matches!(&latest[0],Delivery::Event{envelope,..} if envelope.event.payload==json!(4)));
    let b = broker
        .publish(
            "seat-b",
            Draft::new("seat/b/title", "test", json!("independent")),
        )
        .unwrap();
    assert_eq!(b.seq, 1);
    for n in 0..5 {
        broker
            .publish("seat-a", Draft::new("seat/a/output", "test", json!(n)))
            .unwrap();
    }
    let queue = broker.poll(&id, 8, 100).unwrap();
    assert!(
        queue
            .iter()
            .any(|d| matches!(d, Delivery::Gap { first: 0, .. }))
    );
    assert_eq!(
        EventLog::open(&temp.path().join("seat-a"))
            .unwrap()
            .last_sequence()
            .unwrap(),
        5
    );
    let mut snapshot = Consumer::gui("seat/*/state");
    snapshot.name = Some("latest".into());
    snapshot.start = Start::LastPerSubject;
    let snap = broker.subscribe(snapshot).unwrap();
    assert_eq!(broker.poll(&snap, 8, 100).unwrap().len(), 1);
    drop(broker);
    let mut broker = Broker::open(temp.path(), key).unwrap();
    assert_eq!(broker.poll(&snap, 8, 100).unwrap().len(), 1);
}

// keeps: spec storage
#[test]
fn explicit_ack_redelivers_original_ids_survives_restart_and_never_redelivers_acked_work() {
    let temp = tempfile::tempdir().unwrap();
    let key = SigningKey::from_bytes(&[11; 32]);
    let mut broker = Broker::open(temp.path(), key.clone()).unwrap();
    let mut config = Consumer::gui("task/>");
    config.name = Some("worker".into());
    config.ack = Ack::Explicit;
    config.ack_wait_ms = 10;
    config.max_deliver = 2;
    let id = broker.subscribe(config).unwrap();
    let draft = Draft::new("task/a/delegated", "test", json!({"task":"a"}));
    let event = broker.publish("swarm-a", draft.clone()).unwrap();
    broker.publish("swarm-a", draft).unwrap();
    assert!(
        matches!(&broker.poll(&id,1,100).unwrap()[0],Delivery::Event{deliveries:1,envelope,..} if envelope.event.id==event.event.id)
    );
    assert!(broker.poll(&id, 1, 105).unwrap().is_empty());
    drop(broker);
    let mut broker = Broker::open(temp.path(), key).unwrap();
    assert!(
        matches!(&broker.poll(&id,1,111).unwrap()[0],Delivery::Event{deliveries:2,envelope,..} if envelope.event.id==event.event.id)
    );
    broker.ack(&id, "swarm-a", event.seq).unwrap();
    assert!(broker.poll(&id, 8, 1000).unwrap().is_empty());
    assert!(broker.ack(&id, "swarm-a", event.seq + 100).is_err());
}

// keeps: spec storage
#[test]
fn request_reply_has_no_responders_and_fanout_is_not_replayed() {
    let temp = tempfile::tempdir().unwrap();
    let mut broker = Broker::open(temp.path(), SigningKey::from_bytes(&[12; 32])).unwrap();
    let id = broker.subscribe(Consumer::gui("inbox/seat-a/>")).unwrap();
    let mut draft = Draft::new("service/missing/query", "test", json!({}));
    draft.reply_to = Some("inbox/seat-a/nonce".into());
    broker.publish("account", draft).unwrap();
    assert!(
        matches!(&broker.poll(&id,4,1).unwrap()[0],Delivery::Event{envelope,..} if envelope.seq==0&&envelope.event.payload["status"]=="no_responders")
    );
    let log = EventLog::open(&temp.path().join("account")).unwrap();
    assert_eq!(log.last_sequence().unwrap(), 1);
}

#[test]
fn registry_keeps_session_mapping_mute_and_exact_close_revision_across_restart() {
    let temp = tempfile::tempdir().unwrap();
    let registry = Registry::new(SqliteStore::open(temp.path()).unwrap());
    let mut record = SeatRecord::new(Ulid::new(), Ulid::new(), "fixture", "zsh");
    record.harness_session_id = Some("test-session".into());
    record.muted = true;
    registry.insert(&record).unwrap();
    let position = Position {
        swarm: record.swarm,
        tab: Ulid::new(),
        split_path: vec![1, 0],
        ratios: vec![0.4, 0.5],
        layout_revision: 7,
    };
    let operation = Ulid::new();
    let close = registry
        .close(record.id, 1, operation, position.clone(), 100, 60_000)
        .unwrap();
    assert_eq!(
        registry
            .close(record.id, 1, operation, position, 500, 60_000)
            .unwrap(),
        close
    );
    let registry = Registry::new(SqliteStore::open(temp.path()).unwrap());
    assert!(registry.get(record.id).unwrap().muted);
    assert!(registry.undo(record.id, 1, close.revision, 8, 500).is_err());
    assert!(registry.undo(record.id, 2, close.revision, 7, 500).is_err());
    registry.undo(record.id, 1, close.revision, 7, 500).unwrap();
    assert_eq!(registry.get(record.id).unwrap().lifecycle, Lifecycle::Open);
    assert_eq!(
        registry.get(record.id).unwrap().harness_session_id,
        Some("test-session".into())
    );
}

#[test]
fn numbered_links_bind_graph_edges_only_when_the_number_is_unique() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join(".bboxy");
    std::fs::create_dir(&root).unwrap();
    let id = Ulid::new();
    std::fs::write(
        root.join("decision.md"),
        format!("---\nid: {id}\ntype: decision\nnumber: 47\n---\nDecision\n"),
    )
    .unwrap();
    std::fs::write(
        root.join("source.md"),
        "---\ntype: note\nnumber: 48\n---\n[[#47\\|decision]]{evidence:2}\n",
    )
    .unwrap();
    let mut index = Index::open(
        &root,
        SqliteStore::open(&temp.path().join("state")).unwrap(),
    )
    .unwrap();
    index.scan().unwrap();
    for kind in [GraphKind::Markdown, GraphKind::Provenance] {
        let graph = &index.graphs[&kind];
        let edge = graph.edge_indices().next().unwrap();
        assert_eq!(graph[graph.edge_endpoints(edge).unwrap().1], id.to_string());
        assert!(!graph.node_weights().any(|node| node == "#47"));
    }
    std::fs::write(
        root.join("duplicate.md"),
        format!(
            "---\nid: {}\ntype: note\nnumber: 47\n---\nDuplicate\n",
            Ulid::new()
        ),
    )
    .unwrap();
    index.scan().unwrap();
    let graph = &index.graphs[&GraphKind::Markdown];
    let edge = graph.edge_indices().next().unwrap();
    assert_eq!(
        graph[graph.edge_endpoints(edge).unwrap().1],
        "#47",
        "an ambiguous number stays unresolved"
    );
}

#[test]
fn index_resolves_annotated_links_retains_broken_frontmatter_and_rebuilds_from_sqlite() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join(".bboxy");
    std::fs::create_dir(&root).unwrap();
    let state = temp.path().join("state");
    let mut index = Index::open(&root, SqliteStore::open(&state).unwrap()).unwrap();
    let id = Ulid::new();
    std::fs::write(
        root.join("spec.md"),
        format!("---\nid: {id}\ntype: spec\nnumber: 1\n---\nA spec"),
    )
    .unwrap();
    std::fs::write(root.join("task.md"),"---\ntype: task\nnumber: 2\nlinks:\n  - {to: '[[spec]]', rel: depends}\n---\n[[spec|Display name]]{evidence:2} [[spec]]").unwrap();
    std::fs::write(root.join("broken.md"), "---\nx: [\n---\nbroken").unwrap();
    let events = index.scan().unwrap();
    assert_eq!(events.len(), 6);
    assert!(index.documents["broken.md"].error.is_some());
    assert!(
        index.graphs[&GraphKind::Markdown]
            .node_weights()
            .any(|n| n == &id.to_string())
    );
    assert!(index.graphs[&GraphKind::Task].edge_count() > 0);
    assert!(index.graphs[&GraphKind::Provenance].edge_count() > 0);
    // A restart before bus acknowledgment must retain the exact draft IDs.
    drop(index);
    let mut index = Index::open(&root, SqliteStore::open(&state).unwrap()).unwrap();
    assert_eq!(index.scan().unwrap(), events);
    for event in events {
        index.acknowledge(event.id).unwrap();
    }
    assert!(index.scan().unwrap().is_empty());
    drop(index);
    let index = Index::open(&root, SqliteStore::open(&state).unwrap()).unwrap();
    assert_eq!(index.documents.len(), 3);
    let links = wikilinks("[[spec|not-a-relation]] [[spec]]{evidence:2}");
    assert_eq!(links[0].relation, "ref");
    assert_eq!(links[1].weight, 2.);
    let escaped = wikilinks(r"[[spec\|Display]]{evidence:2} [[#47\|decision]]");
    assert_eq!(escaped[0].to, "spec");
    assert_eq!(escaped[0].weight, 2.);
    assert_eq!(escaped[1].to, "#47");
    let invalid = Document::parse("bad.md", "---\nid: not-a-ulid\n---\nx");
    assert!(invalid.error.is_some());
}

// keeps: spec storage
#[test]
fn mux_document_writes_stamp_and_sign_digests_and_file_watch_observes_real_changes() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join(".bboxy");
    std::fs::create_dir(&root).unwrap();
    let index = Index::open(
        &root,
        SqliteStore::open(&temp.path().join("index")).unwrap(),
    )
    .unwrap();
    let watch = FileWatch::start(&root).unwrap();
    watch.changed();
    index
        .write(
            "note.md",
            json!({"id":Ulid::new(),"type":"note"}),
            "body\n",
            Ulid::new(),
            &SigningKey::from_bytes(&[13; 32]),
        )
        .unwrap();
    let source = std::fs::read_to_string(root.join("note.md")).unwrap();
    let doc = Document::parse("note.md", &source);
    assert!(doc.error.is_none());
    assert_eq!(doc.metadata["number"], 1);
    assert!(
        doc.metadata["updated_by"]
            .as_str()
            .unwrap()
            .starts_with("seat:")
    );
    assert!(doc.metadata["provenance"]["sig"].is_string());
    use base64::Engine;
    use ed25519_dalek::{Signature, Verifier};
    let provenance = doc.metadata["provenance"].clone();
    let mut front = doc.metadata.clone();
    front.as_object_mut().unwrap().remove("provenance");
    let front_hash = bbt_mux::store::digest(serde_yaml::to_string(&front).unwrap().as_bytes());
    let body_hash = bbt_mux::store::digest(doc.body.as_bytes());
    assert_eq!(provenance["digest"], format!("sha256:{body_hash}"));
    assert_eq!(
        provenance["frontmatter_digest"],
        format!("sha256:{front_hash}")
    );
    let mut signed = b"DSSEv1 13 bboxy/item/v1 128 ".to_vec();
    signed.extend(format!("{front_hash}{body_hash}").as_bytes());
    let signature = Signature::try_from(
        base64::engine::general_purpose::STANDARD
            .decode(provenance["sig"].as_str().unwrap())
            .unwrap()
            .as_slice(),
    )
    .unwrap();
    SigningKey::from_bytes(&[13; 32])
        .verifying_key()
        .verify(&signed, &signature)
        .unwrap();
    assert!(
        provenance["log"]
            .as_str()
            .unwrap()
            .starts_with(".bboxy/log/")
    );
    let projection = std::fs::read_dir(root.join("log"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let projection = std::fs::read_to_string(projection).unwrap();
    assert_eq!(projection.lines().count(), 2);
    for line in projection.lines() {
        serde_json::from_str::<bbt_mux::store::events::Envelope>(line)
            .unwrap()
            .verify()
            .unwrap();
    }
    assert!(
        index
            .write(
                "../escape.md",
                json!({}),
                "x",
                Ulid::new(),
                &SigningKey::from_bytes(&[13; 32])
            )
            .is_err()
    );
    let until = std::time::Instant::now() + Duration::from_secs(5);
    while !watch.changed() {
        assert!(std::time::Instant::now() < until);
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(!watch.failed());
}

// keeps: spec storage
#[test]
fn record_numbers_use_fresh_nested_frontmatter_and_are_immutable() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join(".bboxy");
    let index = Index::open(
        &root,
        SqliteStore::open(&temp.path().join("state")).unwrap(),
    )
    .unwrap();
    let seat = Ulid::new();
    let key = SigningKey::from_bytes(&[71; 32]);
    index
        .write(
            "first.md",
            json!({"id":Ulid::new(),"type":"note"}),
            "First\n",
            seat,
            &key,
        )
        .unwrap();
    let first = Document::parse(
        "first.md",
        &std::fs::read_to_string(root.join("first.md")).unwrap(),
    );
    assert_eq!(first.metadata["number"], 1);
    std::fs::create_dir(root.join("nested")).unwrap();
    std::fs::write(root.join("nested/spec.md"), format!("\u{feff}---\r\nid: {}\r\ntype: spec\r\nnumber: 90 # fresh external record\r\n---\r\nnumber: 999999\r\n", Ulid::new())).unwrap();
    // The Index mirror was never rescanned; allocation must read the files.
    assert!(index.documents.is_empty());
    index
        .write(
            "second.md",
            json!({"id":Ulid::new(),"type":"note"}),
            "Second\n",
            seat,
            &key,
        )
        .unwrap();
    let source = std::fs::read_to_string(root.join("second.md")).unwrap();
    let second = Document::parse("second.md", &source);
    assert_eq!(second.metadata["number"], 91);
    assert!(
        index
            .write(
                "duplicate.md",
                json!({"id":Ulid::new(),"type":"note","number":90}),
                "Rejected",
                seat,
                &key
            )
            .is_err()
    );
    assert!(!root.join("duplicate.md").exists());
    let mut metadata = second.metadata.clone();
    metadata["number"] = json!(92);
    assert!(
        index
            .write("second.md", metadata, "No renumber", seat, &key)
            .is_err()
    );
    assert_eq!(
        std::fs::read_to_string(root.join("second.md")).unwrap(),
        source
    );
    let mut metadata = second.metadata;
    metadata.as_object_mut().unwrap().remove("number");
    index
        .write("second.md", metadata, "Updated\n", seat, &key)
        .unwrap();
    let updated = Document::parse(
        "second.md",
        &std::fs::read_to_string(root.join("second.md")).unwrap(),
    );
    assert_eq!(updated.metadata["number"], 91);
}

// keeps: spec storage
#[test]
fn invalid_legacy_numbers_remain_visible_and_block_new_writes() {
    for number in [
        "",
        "number: 0\n",
        "number: -1\n",
        "number: '4'\n",
        "number: 1.5\n",
    ] {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join(".bboxy");
        let mut index = Index::open(
            &root,
            SqliteStore::open(&temp.path().join("state")).unwrap(),
        )
        .unwrap();
        let source = format!(
            "---\nid: {}\ntype: note\n{number}---\nRetained broken body\n",
            Ulid::new()
        );
        std::fs::write(root.join("legacy.md"), &source).unwrap();
        index.scan().unwrap();
        let legacy = &index.documents["legacy.md"];
        assert!(legacy.error.as_deref().unwrap().contains("number"));
        assert!(legacy.body.contains("Retained broken body"));
        let key = SigningKey::from_bytes(&[72; 32]);
        assert!(
            index
                .write(
                    "new.md",
                    json!({"id":Ulid::new(),"type":"artifact"}),
                    "No write",
                    Ulid::new(),
                    &key
                )
                .is_err()
        );
        assert!(!root.join("new.md").exists());
        assert!(
            index
                .write(
                    "legacy.md",
                    legacy.metadata.clone(),
                    "No repair",
                    Ulid::new(),
                    &key
                )
                .is_err()
        );
        assert_eq!(
            std::fs::read_to_string(root.join("legacy.md")).unwrap(),
            source
        );
    }
}

// keeps: spec storage
#[test]
fn duplicate_numbers_in_different_types_preserve_conflicts_and_allocate_above_them() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join(".bboxy");
    let index = Index::open(
        &root,
        SqliteStore::open(&temp.path().join("state")).unwrap(),
    )
    .unwrap();
    for (name, kind) in [("spec", "spec"), ("task", "task")] {
        std::fs::write(
            root.join(format!("{name}.md")),
            format!(
                "---\nid: {}\ntype: {kind}\nnumber: 7\n---\nBody\n",
                Ulid::new()
            ),
        )
        .unwrap();
    }
    let original = std::fs::read_to_string(root.join("spec.md")).unwrap();
    index
        .write(
            "new.md",
            json!({"id":Ulid::new(),"type":"note"}),
            "Unrelated new record",
            Ulid::new(),
            &SigningKey::from_bytes(&[73; 32]),
        )
        .unwrap();
    let created = bbt_mux::index::Document::parse(
        "new.md",
        &std::fs::read_to_string(root.join("new.md")).unwrap(),
    );
    assert_eq!(created.metadata["number"], 8);
    let current = bbt_mux::index::Document::parse("spec.md", &original);
    let error = index
        .write(
            "spec.md",
            current.metadata,
            "Do not replace a conflicting record",
            Ulid::new(),
            &SigningKey::from_bytes(&[73; 32]),
        )
        .unwrap_err();
    assert!(error.to_string().contains("duplicate record number 7"));
    assert_eq!(
        std::fs::read_to_string(root.join("spec.md")).unwrap(),
        original
    );
}

// keeps: spec storage
#[test]
#[ignore = "subprocess fixture used by concurrent_processes_allocate_under_one_root_lock"]
fn number_allocation_child() {
    let fixture = std::path::PathBuf::from(std::env::var_os("BBOXY_NUM2_FIXTURE").unwrap());
    let name = std::env::var("BBOXY_NUM2_WRITER").unwrap();
    let index = Index::open(
        &fixture.join(".bboxy"),
        SqliteStore::open(&fixture.join(&name)).unwrap(),
    )
    .unwrap();
    std::fs::write(fixture.join(format!("{name}.ready")), b"ready").unwrap();
    let seat = Ulid::new();
    let key = SigningKey::from_bytes(&[74; 32]);
    for n in 0..4 {
        index
            .write(
                &format!("{name}-{n}.md"),
                json!({"id":Ulid::new(),"type":"note"}),
                "Child record\n",
                seat,
                &key,
            )
            .unwrap();
    }
}

// keeps: spec storage
#[test]
fn concurrent_processes_allocate_under_one_root_lock() {
    struct OwnedChild(std::process::Child);
    impl Drop for OwnedChild {
        fn drop(&mut self) {
            if self.0.try_wait().ok().flatten().is_none() {
                let _ = self.0.kill();
                let _ = self.0.wait();
            }
        }
    }
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join(".bboxy");
    std::fs::create_dir(&root).unwrap();
    let lock = std::fs::File::open(&root).unwrap();
    lock.lock().unwrap();
    let mut children = Vec::new();
    for name in ["writer-a", "writer-b"] {
        let log = std::fs::File::create(temp.path().join(format!("{name}.log"))).unwrap();
        let child = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "number_allocation_child",
                "--ignored",
                "--nocapture",
            ])
            .env("BBOXY_NUM2_FIXTURE", temp.path())
            .env("BBOXY_NUM2_WRITER", name)
            .stdout(log.try_clone().unwrap())
            .stderr(log)
            .spawn()
            .unwrap();
        eprintln!("NUM2 owned fixture child {name}: PID {}", child.id());
        children.push((name, OwnedChild(child)));
    }
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    while children
        .iter()
        .any(|(name, _)| !temp.path().join(format!("{name}.ready")).exists())
    {
        assert!(
            std::time::Instant::now() < deadline,
            "number writer did not become ready"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    std::thread::sleep(Duration::from_millis(100));
    for (name, child) in &mut children {
        assert!(child.0.try_wait().unwrap().is_none());
        assert!(
            !root.join(format!("{name}-0.md")).exists(),
            "writer bypassed the store lock"
        );
    }
    // Both independent Index instances opened before this fresh maximum.
    std::fs::write(
        root.join("anchor.md"),
        format!(
            "---\nid: {}\ntype: spec\nnumber: 41\n---\nAnchor\n",
            Ulid::new()
        ),
    )
    .unwrap();
    drop(lock);
    for (name, child) in &mut children {
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        let status = loop {
            if let Some(status) = child.0.try_wait().unwrap() {
                break status;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "number writer timed out"
            );
            std::thread::sleep(Duration::from_millis(10));
        };
        assert!(
            status.success(),
            "{}",
            std::fs::read_to_string(temp.path().join(format!("{name}.log"))).unwrap()
        );
    }
    let mut numbers = std::collections::BTreeSet::new();
    for name in ["writer-a", "writer-b"] {
        for n in 0..4 {
            let path = format!("{name}-{n}.md");
            let doc = Document::parse(&path, &std::fs::read_to_string(root.join(&path)).unwrap());
            assert!(doc.error.is_none(), "{:?}", doc.error);
            assert!(numbers.insert(doc.metadata["number"].as_u64().unwrap()));
            assert!(doc.metadata["provenance"]["sig"].is_string());
        }
    }
    assert_eq!(numbers, (42..=49).collect());
    eprintln!("NUM2 independent-process numbers: {numbers:?}");
}
