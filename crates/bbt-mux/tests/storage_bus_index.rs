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
        "---\ntype: note\n---\n[[#47\\|decision]]{evidence:2}\n",
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
        format!("---\nid: {id}\ntype: spec\n---\nA spec"),
    )
    .unwrap();
    std::fs::write(root.join("task.md"),"---\ntype: task\nlinks:\n  - {to: '[[spec]]', rel: depends}\n---\n[[spec|Display name]]{evidence:2} [[spec]]").unwrap();
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
