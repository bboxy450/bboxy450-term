#![allow(clippy::disallowed_methods)] // Disposable broker/database/socket fixtures.
mod common;
use bbt_mux::Ulid;
use bbt_mux::messaging::{
    keys::{LocalSeatKeys, SeatKeys},
    tree::{Actor, Filter, Node, Rate, Right, Tree},
    wire::{Body, Envelope, EnvelopeExt, MessageKind, SendRequest, SendRequestExt, public},
};
use bbt_mux::registry::{Class, Role};
use bbt_mux::store::{digest, sqlite::SqliteStore};
use ed25519_dalek::SigningKey;
use std::collections::BTreeMap;

fn envelope(keys: &LocalSeatKeys, seat: Ulid, swarm: Ulid) -> Envelope {
    let issuer = keys.issuer();
    Envelope {
        version: 1,
        id: Ulid::new(),
        account: issuer.key.clone(),
        device: issuer.key.clone(),
        issuer_kind: issuer.kind,
        issuer: issuer.key,
        seat,
        author: public(&keys.leaf(seat, 1).unwrap().verifying_key().to_bytes()),
        swarm_id: swarm,
        process_generation: 1,
        context: format!("seats/{seat}/inbox"),
        role: Role {
            name: "executor".into(),
            class: Class::Worker,
        },
        role_grant_id: digest(b"fixture-grant"),
        role_epoch: 1,
        priority_class: Class::Worker,
        sender_sequence: 1,
        thread_id: Ulid::new(),
        reply_to: None,
        timestamp: 1_792_694_400,
        priority: 0,
        kind: MessageKind::Text,
        body: Body::Text {
            text: "signed message".into(),
        },
        attachments: vec!["[[tasks/fixture]]".into()],
        door: None,
        issued_by: None,
        signature: String::new(),
    }
}
#[test]
fn signed_envelope_covers_role_routing_thread_and_rejects_untrusted_issuer_and_noncanonical_counters()
 {
    let keys = LocalSeatKeys::new(SigningKey::from_bytes(&[42; 32]));
    let seat = Ulid::new();
    let swarm = Ulid::new();
    let mut signed = envelope(&keys, seat, swarm);
    signed.sign(&keys.leaf(seat, 1).unwrap()).unwrap();
    keys.verify(&signed).unwrap();
    for which in 0..6 {
        let mut forged = signed.clone();
        match which {
            0 => forged.role.class = Class::Oracle,
            1 => forged.context.push_str("/other"),
            2 => forged.thread_id = Ulid::new(),
            3 => forged.attachments.push("[[other]]".into()),
            4 => {
                forged.body = Body::Text {
                    text: "forged".into(),
                }
            }
            _ => forged.process_generation += 1,
        };
        assert!(keys.verify(&forged).is_err());
    }
    let foreign = LocalSeatKeys::new(SigningKey::from_bytes(&[43; 32]));
    assert!(foreign.verify(&signed).is_err());
    let mut json = serde_json::to_value(&signed).unwrap();
    assert_eq!(json["process_generation"], "1");
    json["sender_sequence"] = serde_json::json!("01");
    assert!(serde_json::from_value::<Envelope>(json).is_err());
    assert_eq!(
        keys.leaf(seat, 1).unwrap().to_bytes(),
        keys.leaf(seat, 1).unwrap().to_bytes()
    );
    assert_ne!(
        keys.leaf(seat, 1).unwrap().to_bytes(),
        keys.leaf(seat, 2).unwrap().to_bytes()
    );
    assert_ne!(
        keys.leaf(seat, 1).unwrap().to_bytes(),
        keys.leaf(Ulid::new(), 1).unwrap().to_bytes()
    );
    let mut bad = SendRequest::text("#bboxy/*", "text");
    bad.attachments = vec!["[[".into()];
    assert!(bad.validate().is_err());
}
fn add_sink(tree: &mut Tree, seat: Ulid) {
    tree.put(Actor::Operator, Node::folder(format!("seats/{seat}")))
        .unwrap();
    tree.put(Actor::Operator, Node::sink(seat)).unwrap();
}

#[test]
fn nil_seat_requires_the_local_operator_device_and_reserved_role() {
    use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
    use ed25519_dalek::Signer;
    let device = SigningKey::from_bytes(&[42; 32]);
    let keys = LocalSeatKeys::new(device.clone());
    let mut signed = envelope(&keys, Ulid::new(), Ulid::new());
    signed.seat = Ulid::nil();
    signed.process_generation = 0;
    signed.author = public(&device.verifying_key().to_bytes());
    signed.role = Role {
        name: "operator".into(),
        class: Class::Oracle,
    };
    signed.priority_class = Class::Oracle;
    signed.sign(&device).unwrap();
    keys.verify(&signed).unwrap();
    assert!(
        LocalSeatKeys::new(SigningKey::from_bytes(&[43; 32]))
            .verify(&signed)
            .is_err()
    );
    for which in 0..4 {
        let mut forged = signed.clone();
        match which {
            0 => forged.role.name = "executor".into(),
            1 => forged.role.class = Class::Worker,
            2 => forged.process_generation = 1,
            _ => forged.device = "11".repeat(32),
        }
        // A valid cryptographic signature must not bypass operator policy.
        forged.signature =
            URL_SAFE_NO_PAD.encode(device.sign(&forged.signing_bytes().unwrap()).to_bytes());
        assert!(keys.verify(&forged).is_err(), "forgery {which}");
    }
}

#[test]
fn message_bus_position_survives_reopen_without_reusing_the_ledger_sequence() {
    use bbt_mux::messaging::ledger::{BusPosition, Ledger};
    let tmp = tempfile::tempdir().unwrap();
    let store = SqliteStore::open(tmp.path()).unwrap();
    let swarm = Ulid::new();
    let seat = Ulid::new();
    let keys = LocalSeatKeys::new(SigningKey::from_bytes(&[42; 32]));
    let mut signed = envelope(&keys, seat, swarm);
    signed.sign(&keys.leaf(seat, 1).unwrap()).unwrap();
    let mut request = SendRequest::text(&signed.context, signed.body.text().unwrap());
    request.id = signed.id;
    request.attachments = signed.attachments.clone();
    let ledger = Ledger::open(store.clone(), swarm).unwrap();
    let stored = ledger.accept(&request, &signed, &[], None, 1).unwrap();
    assert!(stored.bus.is_none());
    let bus = BusPosition {
        stream: format!("swarm-{swarm}"),
        seq: 9_007_199_254_740_993,
    };
    assert_ne!(stored.sequence, bus.seq);
    ledger.record_publication(signed.id, &bus).unwrap();
    ledger.record_publication(signed.id, &bus).unwrap();
    let mut conflict = bus.clone();
    conflict.seq += 1;
    assert!(ledger.record_publication(signed.id, &conflict).is_err());
    drop(ledger);
    let reopened = Ledger::open(store, swarm).unwrap();
    let stored = reopened.by_id(signed.id).unwrap();
    assert_eq!(stored.bus, Some(bus));
    assert_eq!(
        serde_json::to_value(&stored).unwrap()["bus"]["seq"],
        "9007199254740993"
    );
    assert_eq!(
        reopened.existing(&request, seat).unwrap().unwrap().bus,
        stored.bus
    );
    assert_eq!(
        reopened.list(Some(seat), None, None, 0, 128).unwrap().items[0].bus,
        stored.bus
    );
}
#[test]
fn nested_fanout_deduplicates_bridges_cycles_filters_and_preserves_arrival_paths_after_reopen() {
    let tmp = tempfile::tempdir().unwrap();
    let store = SqliteStore::open(tmp.path()).unwrap();
    let swarm = Ulid::new();
    let mut tree = Tree::open(store.clone(), swarm).unwrap();
    let a = Ulid::new();
    let b = Ulid::new();
    add_sink(&mut tree, a);
    add_sink(&mut tree, b);
    for p in [
        "channels/bboxy",
        "channels/bboxy/ui",
        "channels/bboxy/render",
    ] {
        tree.put(Actor::Operator, Node::folder(p)).unwrap();
    }
    tree.put(
        Actor::Operator,
        Node::link(format!("channels/bboxy/{a}"), format!("seats/{a}/inbox")),
    )
    .unwrap();
    tree.put(
        Actor::Operator,
        Node::link(
            format!("channels/bboxy/render/{b}"),
            format!("seats/{b}/inbox"),
        ),
    )
    .unwrap();
    tree.put(
        Actor::Operator,
        Node::link("channels/bboxy/ui/bridge", "channels/bboxy/render"),
    )
    .unwrap();
    tree.put(
        Actor::Operator,
        Node::link("channels/bboxy/render/cycle", "channels/bboxy/ui"),
    )
    .unwrap();
    let routes = tree
        .resolve(
            Actor::Operator,
            "#bboxy/**",
            MessageKind::Text,
            Class::Worker,
            0,
        )
        .unwrap();
    assert_eq!(routes.len(), 2);
    assert!(
        routes
            .iter()
            .find(|r| r.recipient == b)
            .unwrap()
            .arrivals
            .len()
            >= 2
    );
    let direct = tree
        .resolve(
            Actor::Operator,
            "#bboxy/*",
            MessageKind::Text,
            Class::Worker,
            0,
        )
        .unwrap();
    assert_eq!(direct.len(), 1);
    assert_eq!(direct[0].recipient, a);
    let narrow = tree
        .resolve(
            Actor::Operator,
            "#bboxy/ui/**",
            MessageKind::Text,
            Class::Worker,
            0,
        )
        .unwrap();
    assert_eq!(narrow.len(), 1);
    assert_eq!(narrow[0].recipient, b);
    assert!(
        narrow[0].arrivals[0]
            .path
            .starts_with("channels/bboxy/ui/bridge/")
    );
    drop(tree);
    let tree = Tree::open(store, swarm).unwrap();
    assert_eq!(
        tree.resolve(
            Actor::Operator,
            "#bboxy/**",
            MessageKind::Text,
            Class::Worker,
            0
        )
        .unwrap(),
        routes
    );
}
#[test]
fn filter_rate_pause_and_worker_self_subscription_are_bounded_by_folder_grants() {
    let tmp = tempfile::tempdir().unwrap();
    let mut tree = Tree::open(SqliteStore::open(tmp.path()).unwrap(), Ulid::new()).unwrap();
    let seat = Ulid::new();
    add_sink(&mut tree, seat);
    let worker = Actor::Seat {
        seat,
        class: Class::Worker,
    };
    let mut channel = Node::folder("channels/review");
    channel.grants = BTreeMap::from([(seat, vec![Right::Read, Right::Link])]);
    tree.put(Actor::Operator, channel).unwrap();
    let err = tree
        .resolve(
            worker,
            "channels/review/**",
            MessageKind::Text,
            Class::Worker,
            0,
        )
        .unwrap_err()
        .to_string();
    assert!(err.contains("channels/review") && err.contains("write"));
    let mut link = Node::link("channels/review/me", format!("seats/{seat}/inbox"));
    link.filter = Filter {
        kinds: vec![MessageKind::Chunk],
        classes: vec![Class::Worker],
        priorities: Vec::new(),
    };
    link.rate = Rate {
        batch_ms: 1000,
        coalesce: true,
    };
    tree.put(worker, link).unwrap();
    assert!(
        tree.resolve(
            Actor::Operator,
            "channels/review/**",
            MessageKind::Text,
            Class::Worker,
            0
        )
        .unwrap()
        .is_empty()
    );
    let routes = tree
        .resolve(
            Actor::Operator,
            "channels/review/**",
            MessageKind::Chunk,
            Class::Worker,
            0,
        )
        .unwrap();
    assert_eq!(routes.len(), 1);
    assert_eq!(routes[0].arrivals[0].batch_ms, 1000);
    assert!(routes[0].arrivals[0].coalesce);
    tree.pause(worker, "channels/review/me", true).unwrap();
    assert!(
        tree.resolve(
            Actor::Operator,
            "channels/review/**",
            MessageKind::Chunk,
            Class::Worker,
            0
        )
        .unwrap()[0]
            .arrivals[0]
            .paused
    );
    assert!(
        tree.put(worker, Node::link("channels/review/other", "channels"))
            .is_err()
    );
    tree.remove(worker, "channels/review/me").unwrap();
}

use bbt_mux::{
    adapters::{Evidence, State, WakeOutcome},
    messaging::Messaging,
    registry::{Registry, SeatRecord},
    store::object_dir,
};
use std::sync::Arc;
struct Fixture {
    tmp: tempfile::TempDir,
    registry: Registry,
    messaging: Messaging,
    a: SeatRecord,
    b: SeatRecord,
    o: SeatRecord,
}
impl Fixture {
    fn new() -> Self {
        let tmp = tempfile::tempdir().unwrap();
        let registry =
            Registry::new(SqliteStore::open(&object_dir(tmp.path(), "account").unwrap()).unwrap());
        let swarm = Ulid::new();
        let a = SeatRecord::new(Ulid::new(), swarm, "a", "claude");
        let b = SeatRecord::new(Ulid::new(), swarm, "b", "claude");
        let mut o = SeatRecord::new(Ulid::new(), swarm, "oracle", "claude");
        o.role = Role {
            name: "a custom name".into(),
            class: Class::Oracle,
        };
        for r in [&a, &b, &o] {
            registry.insert(r).unwrap();
        }
        let messaging = Messaging::open(
            tmp.path(),
            Arc::new(LocalSeatKeys::new(SigningKey::from_bytes(&[42; 32]))),
        )
        .unwrap();
        Self {
            tmp,
            registry,
            messaging,
            a,
            b,
            o,
        }
    }
    fn reopen(&mut self) {
        self.messaging = Messaging::open(
            self.tmp.path(),
            Arc::new(LocalSeatKeys::new(SigningKey::from_bytes(&[42; 32]))),
        )
        .unwrap();
    }
    fn send(
        &mut self,
        from: Ulid,
        text: &str,
        now: u64,
    ) -> bbt_mux::messaging::ledger::StoredMessage {
        self.messaging
            .send(
                from,
                1,
                SendRequest::text(format!("seats/{}/inbox", self.b.id), text),
                now,
            )
            .unwrap()
    }
    fn take(&mut self, now: u64) -> bbt_mux::messaging::ledger::Work {
        self.messaging.ready(now).unwrap();
        self.messaging
            .prepare(self.b.swarm, self.b.id, now)
            .unwrap()
            .unwrap()
    }
    fn finish(&mut self, work: &bbt_mux::messaging::ledger::Work, evidence: Evidence, now: u64) {
        self.messaging
            .finish(
                self.b.swarm,
                work,
                &WakeOutcome {
                    evidence,
                    note: "fixture".into(),
                    native_id: None,
                },
                now,
                false,
            )
            .unwrap();
    }
}
#[test]
fn persist_before_dispatch_thread_retry_ack_race_and_unknown_survive_restart() {
    let mut f = Fixture::new();
    let first = f.send(f.a.id, "parent", 1000);
    let r = f.messaging.receipts(f.a.id, first.envelope.id).unwrap();
    assert_eq!(r[0].evidence, Evidence::RecipientStored);
    assert!(r[0].attempt.is_none());
    // One wake carries everything pending (decision #23); the reply below is
    // sent while this one is in flight, so it gets its own attempt.
    let work = f.take(1010);
    assert_eq!(work.messages, vec![first.clone()]);
    let mut reply = SendRequest::text(format!("seats/{}/inbox", f.b.id), "reply");
    reply.reply_to = Some(first.envelope.id);
    let stored = f.messaging.send(f.a.id, 1, reply.clone(), 1001).unwrap();
    assert_eq!(stored.envelope.thread_id, first.envelope.thread_id);
    assert_eq!(
        f.messaging.send(f.a.id, 1, reply.clone(), 1002).unwrap(),
        stored
    );
    reply.text.push_str(" changed");
    assert!(f.messaging.send(f.a.id, 1, reply, 1003).is_err());
    f.messaging
        .acknowledge(f.b.id, first.envelope.id, None, 1011)
        .unwrap();
    f.finish(&work, Evidence::NativeWritten, 1012);
    assert_eq!(
        f.messaging.receipts(f.a.id, first.envelope.id).unwrap()[0].evidence,
        Evidence::ApplicationAcknowledged
    );
    let abandoned = f.take(1020);
    assert_eq!(abandoned.messages[0].envelope.id, stored.envelope.id);
    f.reopen();
    assert_eq!(
        f.messaging.receipts(f.a.id, stored.envelope.id).unwrap()[0].evidence,
        Evidence::NativeUnknown
    );
    assert!(
        f.messaging
            .prepare(f.b.swarm, f.b.id, 2000)
            .unwrap()
            .is_none()
    );
    f.send(f.a.id, "later independent message", 2001);
    let next = f.take(2002);
    assert_ne!(next.id, abandoned.id);
    f.finish(&next, Evidence::NativeUnknown, 2003);
    f.reopen();
    assert!(
        f.messaging
            .prepare(f.b.swarm, f.b.id, 5000)
            .unwrap()
            .is_none()
    );
}
#[test]
fn mute_holds_both_directions_reopens_and_releases_only_under_current_grants() {
    let mut f = Fixture::new();
    let notice = f
        .messaging
        .set_mute(Actor::Operator, f.b.id, true, State::Active, 1000)
        .unwrap()
        .unwrap();
    let body = f
        .messaging
        .list(
            f.a.id,
            None,
            Some(&format!("swarms/{}/*", f.a.swarm)),
            0,
            128,
        )
        .unwrap();
    assert!(body.items.iter().any(|m| m.envelope.id == notice
        && matches!(
            m.envelope.body,
            Body::Presence {
                state: State::Active,
                ..
            }
        )));
    let inbound = f.send(f.a.id, "held in", 1001);
    let outbound = f
        .messaging
        .send(
            f.b.id,
            1,
            SendRequest::text(format!("seats/{}/inbox", f.a.id), "held out"),
            1002,
        )
        .unwrap();
    assert!(outbound.held_out.is_some());
    assert!(f.messaging.ready(2000).unwrap().is_empty());
    f.reopen();
    let blocked = f.send(f.a.id, "grant will be removed", 2001);
    f.messaging
        .tree(f.a.swarm)
        .unwrap()
        .grant(
            Actor::Operator,
            &format!("seats/{}/inbox", f.b.id),
            f.a.id,
            vec![],
        )
        .unwrap();
    f.messaging
        .set_mute(Actor::Operator, f.b.id, false, State::Active, 3000)
        .unwrap();
    let held = f.messaging.receipts(f.a.id, blocked.envelope.id).unwrap();
    assert!(held[0].held.as_ref().unwrap().contains("write"));
    assert!(
        f.messaging.receipts(f.a.id, inbound.envelope.id).unwrap()[0]
            .held
            .is_some()
    );
    let pending = f.messaging.ready(3001).unwrap();
    assert!(pending.iter().any(|(_, s)| *s == f.a.id));
    let work = f
        .messaging
        .prepare(f.a.swarm, f.a.id, 3001)
        .unwrap()
        .unwrap();
    assert_eq!(work.messages[0].envelope.id, outbound.envelope.id);
    f.messaging
        .tree(f.a.swarm)
        .unwrap()
        .grant(
            Actor::Operator,
            &format!("seats/{}/inbox", f.b.id),
            f.a.id,
            vec![Right::Write],
        )
        .unwrap();
    // Both released messages travel in one FIFO wake (decision #23).
    let first = f.take(3002);
    let ids: Vec<_> = first.messages.iter().map(|m| m.envelope.id).collect();
    assert_eq!(ids, vec![inbound.envelope.id, blocked.envelope.id]);
    f.finish(&first, Evidence::NativeWritten, 3003);
    assert!(
        f.messaging
            .prepare(f.b.swarm, f.b.id, 3004)
            .unwrap()
            .is_none()
    );
    assert!(!f.registry.get(f.b.id).unwrap().muted);
}
#[test]
fn priority_fifo_four_to_one_and_burst_overflow_are_receiver_local() {
    let mut f = Fixture::new();
    let worker = f.send(f.a.id, "old worker", 1000);
    let mut ids = Vec::new();
    for _ in 0..32 {
        ids.push(f.send(f.o.id, "oracle", 1000).envelope.id);
    }
    // Per-recipient wakes (decision #23), still under the fair queue: four
    // Oracle messages, then the waiting worker, then the rest of the Oracle
    // credit (30 at 1000 each), then the overflow demoted to worker class.
    let take = |f: &mut Fixture| {
        let work = f.take(1000);
        f.finish(&work, Evidence::NativeWritten, 1000);
        (
            work.class,
            work.messages
                .iter()
                .map(|m| m.envelope.id)
                .collect::<Vec<_>>(),
        )
    };
    assert_eq!(take(&mut f), (Class::Oracle, ids[..4].to_vec()));
    assert_eq!(take(&mut f), (Class::Worker, vec![worker.envelope.id]));
    assert_eq!(take(&mut f), (Class::Oracle, ids[4..30].to_vec()));
    assert_eq!(take(&mut f), (Class::Worker, ids[30..].to_vec()));
}
#[test]
fn oracle_sink_filter_and_rate_batching_do_not_reorder_or_repeat_effects() {
    let mut f = Fixture::new();
    let channel = "channels/updates";
    let mut folder = Node::folder(channel);
    folder
        .grants
        .insert(f.a.id, vec![Right::Write, Right::Read]);
    f.messaging
        .tree(f.a.swarm)
        .unwrap()
        .put(Actor::Operator, folder)
        .unwrap();
    let mut link = Node::link("channels/updates/oracle", format!("seats/{}/inbox", f.o.id));
    link.filter.kinds = vec![MessageKind::Chunk];
    link.rate = Rate {
        batch_ms: 1000,
        coalesce: true,
    };
    f.messaging
        .tree(f.a.swarm)
        .unwrap()
        .put(Actor::Operator, link)
        .unwrap();
    let mut ids = Vec::new();
    for t in [1000, 1001, 1002] {
        let mut request = SendRequest::text("#updates/*", "chunk");
        request.kind = MessageKind::Chunk;
        ids.push(f.messaging.send(f.a.id, 1, request, t).unwrap().envelope.id);
    }
    f.messaging
        .send(
            f.a.id,
            1,
            SendRequest::text("#updates/*", "filtered out"),
            1003,
        )
        .unwrap();
    assert_eq!(
        f.messaging
            .list(f.a.id, None, Some("#updates"), 0, 128)
            .unwrap()
            .items
            .len(),
        4
    );
    assert!(f.messaging.ready(1999).unwrap().is_empty());
    f.messaging.ready(2010).unwrap();
    let work = f
        .messaging
        .prepare(f.o.swarm, f.o.id, 2010)
        .unwrap()
        .unwrap();
    assert_eq!(
        work.messages
            .iter()
            .map(|m| m.envelope.id)
            .collect::<Vec<_>>(),
        ids
    );
    f.messaging
        .finish(
            f.o.swarm,
            &work,
            &WakeOutcome::unknown("partial PTY write; no retry"),
            2011,
            false,
        )
        .unwrap();
    f.reopen();
    assert!(
        f.messaging
            .prepare(f.o.swarm, f.o.id, 9000)
            .unwrap()
            .is_none()
    );
    for id in ids {
        assert_eq!(
            f.messaging.receipts(f.a.id, id).unwrap()[0].evidence,
            Evidence::NativeUnknown
        );
    }
}

#[test]
fn live_logic_and_capability_socket_deliver_dm_native_written_without_claiming_model_receipt() {
    use bbt_mux::{
        LaunchSpec, Supervisor,
        adapters::Endpoint,
        logic::{Logic, LogicConfig},
        messaging::api::{Request as Api, Response},
    };
    use tokio::io::{AsyncWriteExt, BufReader};
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path().join("repo");
    std::fs::create_dir_all(repo.join(".bboxy")).unwrap();
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    let supervisor = Supervisor::start(common::capture_config(tmp.path())).unwrap();
    let logic = Logic::start(
        supervisor.connect().unwrap(),
        LogicConfig {
            run_dir: tmp.path().join("run"),
            repository: repo,
            ring: Default::default(),
        },
        SigningKey::from_bytes(&[42; 32]),
        rt.handle(),
    )
    .unwrap();
    let swarm = Ulid::new();
    let mut specs = Vec::new();
    for name in ["sender", "receiver"] {
        let mut spec = LaunchSpec::new("/bin/zsh", tmp.path());
        spec.args = vec![
            "-f".into(),
            "-c".into(),
            "printf 'READY\\n'; read -r ignored".into(),
        ];
        let mut record = SeatRecord::new(spec.seat, swarm, name, "claude");
        record.harness_session_id = Some(format!("fixture-{name}"));
        rt.block_on(logic.client.spawn(record, spec.clone()))
            .unwrap();
        specs.push(spec);
    }
    let path = std::env::temp_dir().join(format!("w24-{}.sock", Ulid::new()));
    let (server, received) = rt.block_on(async {
        let listener = tokio::net::UnixListener::bind(&path).unwrap();
        let (tx, rx) = tokio::sync::oneshot::channel();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut r = BufReader::new(stream);
            let bytes = bbt_mux::seat_socket::read_line(&mut r)
                .await
                .unwrap()
                .unwrap();
            tx.send(serde_json::from_slice::<serde_json::Value>(&bytes).unwrap())
                .unwrap();
        });
        (server, rx)
    });
    rt.block_on(logic.client.bind_delivery_endpoint(
        specs[1].seat,
        1,
        Endpoint::Claude { path: path.clone() },
    ))
    .unwrap();
    let request = SendRequest::text(
        format!("seats/{}/inbox", specs[1].seat),
        "native fixture body",
    );
    let id = request.id;
    let endpoint = rt.block_on(logic.client.endpoint(specs[0].seat)).unwrap();
    let response = rt.block_on(async {
        let stream = tokio::net::UnixStream::connect(&endpoint.path)
            .await
            .unwrap();
        let (mut read, mut write) = stream.into_split();
        let request = bbt_mux::seat_socket::Request {
            version: 1,
            capability: endpoint.capability,
            seat: specs[0].seat,
            generation: 1,
            action: bbt_mux::seat_socket::Action::Messaging {
                request: Api::Send { message: request },
            },
        };
        let mut bytes = serde_json::to_vec(&request).unwrap();
        bytes.push(b'\n');
        write.write_all(&bytes).await.unwrap();
        let line = bbt_mux::seat_socket::read_line(&mut BufReader::new(&mut read))
            .await
            .unwrap()
            .unwrap();
        serde_json::from_slice::<Response>(&line).unwrap()
    });
    assert!(matches!(response, Response::Stored { .. }));
    let frame = rt.block_on(async {
        tokio::time::timeout(std::time::Duration::from_secs(5), received)
            .await
            .unwrap()
            .unwrap()
    });
    assert!(
        frame["message"]["content"]
            .as_str()
            .unwrap()
            .contains("native fixture body")
    );
    assert_eq!(frame["priority"], "next");
    assert_eq!(frame["type"], "user");
    let until = std::time::Instant::now() + std::time::Duration::from_secs(5);
    let receipts = loop {
        let response = rt
            .block_on(logic.client.messaging(
                specs[0].seat,
                1,
                Api::Receipts {
                    message: id,
                    after: 0,
                    limit: 128,
                },
            ))
            .unwrap();
        let Response::Receipts { page } = response else {
            panic!("wrong response")
        };
        if page.items[0].evidence == Evidence::NativeWritten {
            break page.items;
        }
        assert!(
            std::time::Instant::now() < until,
            "no native write: {:?}",
            page.items
        );
        std::thread::sleep(std::time::Duration::from_millis(25));
    };
    assert_ne!(receipts[0].evidence, Evidence::ApplicationAcknowledged);
    assert!(
        rt.block_on(logic.client.messaging(
            specs[0].seat,
            1,
            Api::Mute {
                seat: specs[1].seat,
                muted: true
            }
        ))
        .unwrap_err()
        .to_string()
        .contains("oracle")
    );
    assert!(
        rt.block_on(logic.client.messaging(
            specs[0].seat,
            2,
            Api::Ls {
                path: "seats".into(),
                after: None,
                limit: 10
            }
        ))
        .is_err()
    );
    rt.block_on(server).unwrap();
    let _ = std::fs::remove_file(path);
    logic.shutdown().unwrap();
    supervisor.shutdown().unwrap();
}

#[test]
fn ambiguous_guarded_pty_stays_unknown_through_the_delivery_ledger() {
    use bbt_mux::adapters::{ArbiterView, Binding, HarnessRecord, Session, native};
    let mut f = Fixture::new();
    let message = f.send(f.a.id, "ambiguous", 1000);
    let work = f.take(1001);
    let binding = Binding {
        seat: f.b.id,
        generation: 1,
        pid: std::process::id(),
        process_start: native::process_start(std::process::id()).unwrap(),
        session: "fixture".into(),
        attachment_generation: 1,
    };
    let mut session =
        Session::new(HarnessRecord::builtin("claude").unwrap(), binding.clone()).unwrap();
    let file = native::ClaudeSessionFile {
        pid: binding.pid,
        session_id: binding.session.clone(),
        process_start: binding.process_start.clone(),
        status: "idle".into(),
        waiting_for: None,
        messaging_socket_path: None,
    };
    session.observe_claude_session(&file, 1, 1000).unwrap();
    let view = ArbiterView {
        binding,
        operator_epoch: 0,
        foreground_pid: Some(std::process::id()),
        operator_draft: false,
        active_tool: false,
        sync_pending: false,
        screen_switch_pending: false,
        canonical: true,
        echo: true,
        bracketed_paste: true,
    };
    let store = SqliteStore::open(&f.tmp.path().join("input-journal")).unwrap();
    let outcome = session
        .submit_pty(&view, 1600, work.id, &work.text(), &store, |bytes| {
            Ok(bbt_mux::InputReceipt {
                input_id: work.id,
                state: bbt_mux::InputState::PartialUnknown,
                bytes_written: 3,
                bytes_total: bytes.len(),
            })
        })
        .unwrap();
    assert_eq!(outcome.evidence, Evidence::NativeUnknown);
    f.messaging
        .finish(f.b.swarm, &work, &outcome, 1601, false)
        .unwrap();
    f.reopen();
    assert_eq!(
        f.messaging.receipts(f.a.id, message.envelope.id).unwrap()[0].evidence,
        Evidence::NativeUnknown
    );
    assert!(
        f.messaging
            .prepare(f.b.swarm, f.b.id, 9000)
            .unwrap()
            .is_none()
    );
}

#[test]
fn supervisor_final_fence_rejects_a_user_draft_without_accepting_any_guarded_bytes() {
    use bbt_mux::{Action, Config, LaunchSpec, Reply, Supervisor};
    let supervisor = Supervisor::start(Config::default()).unwrap();
    let client = supervisor.connect().unwrap();
    let mut spec = LaunchSpec::new("/bin/zsh", std::env::temp_dir());
    spec.args = vec!["-f".into(), "-c".into(), "read -r line".into()];
    client
        .request(Action::Spawn { spec: spec.clone() })
        .unwrap();
    let Reply::Attached { attachment, .. } = client
        .request(Action::Attach {
            seat: spec.seat,
            generation: 1,
        })
        .unwrap()
    else {
        panic!()
    };
    std::thread::sleep(std::time::Duration::from_millis(50));
    let Reply::InputContext { context: before } = client
        .request(Action::InputContext {
            attachment: attachment.clone(),
        })
        .unwrap()
    else {
        panic!()
    };
    client
        .input(&attachment, Ulid::new(), b"a user draft")
        .unwrap();
    let id = Ulid::new();
    let err = client
        .request(Action::GuardedInput {
            attachment: attachment.clone(),
            input_id: id,
            bytes: b"must not appear\r".to_vec(),
            expected: before,
        })
        .unwrap_err();
    assert_eq!(err.code, bbt_mux::FailureCode::Busy);
    assert!(err.message.contains("no mutation"));
    let Reply::InputContext { context: after } = client
        .request(Action::InputContext {
            attachment: attachment.clone(),
        })
        .unwrap()
    else {
        panic!()
    };
    assert!(after.draft);
    assert_eq!(after.epoch, 1);
    assert_eq!(
        client
            .request(Action::InputStatus {
                seat: spec.seat,
                generation: 1,
                input_id: id
            })
            .unwrap_err()
            .code,
        bbt_mux::FailureCode::NotFound
    );
    client.request(Action::Kill { attachment }).unwrap();
    supervisor.shutdown().unwrap();
}

#[test]
fn recipient_cursor_pages_do_not_skip_sinks_and_unsubscribing_survives_reopen() {
    use bbt_mux::messaging::{
        ledger::Ledger,
        tree::{Arrival, Route},
    };
    let tmp = tempfile::tempdir().unwrap();
    let swarm = Ulid::new();
    let keys = LocalSeatKeys::new(SigningKey::from_bytes(&[42; 32]));
    let sender = Ulid::new();
    let ledger = Ledger::open(SqliteStore::open(tmp.path()).unwrap(), swarm).unwrap();
    let mut message = envelope(&keys, sender, swarm);
    message.sign(&keys.leaf(sender, 1).unwrap()).unwrap();
    let mut req = SendRequest::text(message.context.clone(), "signed message");
    req.id = message.id;
    let routes = (0..3)
        .map(|_| Route {
            recipient: Ulid::new().into(),
            agent: None,
            arrivals: vec![Arrival {
                path: message.context.clone(),
                links: vec![],
                paused: false,
                batch_ms: 0,
                coalesce: false,
            }],
        })
        .collect::<Vec<_>>();
    ledger.accept(&req, &message, &routes, None, 1000).unwrap();
    let page = ledger.pending(0, 2).unwrap();
    assert_eq!(page.len(), 2);
    let next = ledger.pending(page[1].0, 2).unwrap();
    assert_eq!(next.len(), 1);
    assert_eq!(next[0].1.message_sequence, page[0].1.message_sequence);
    let mut f = Fixture::new();
    let membership = format!("swarms/{}/{}", f.a.swarm, f.a.id);
    f.messaging
        .tree(f.a.swarm)
        .unwrap()
        .remove(Actor::Operator, &membership)
        .unwrap();
    f.reopen();
    assert!(
        f.messaging
            .tree(f.a.swarm)
            .unwrap()
            .get(&membership)
            .is_none()
    );
}

#[test]
fn live_supervisor_admits_fresh_guard_and_rejects_replaced_foreground_evidence() {
    use bbt_mux::{Action, Config, LaunchSpec, Reply, Supervisor};
    let supervisor = Supervisor::start(Config::default()).unwrap();
    let client = supervisor.connect().unwrap();
    let mut spec = LaunchSpec::new("/bin/zsh", std::env::temp_dir());
    spec.args = vec![
        "-f".into(),
        "-c".into(),
        "read -r line; printf 'GOT:%s\\n' \"$line\"".into(),
    ];
    client
        .request(Action::Spawn { spec: spec.clone() })
        .unwrap();
    let Reply::Attached { attachment, .. } = client
        .request(Action::Attach {
            seat: spec.seat,
            generation: 1,
        })
        .unwrap()
    else {
        panic!()
    };
    let until = std::time::Instant::now() + std::time::Duration::from_secs(3);
    let context = loop {
        let Reply::InputContext { context } = client
            .request(Action::InputContext {
                attachment: attachment.clone(),
            })
            .unwrap()
        else {
            panic!()
        };
        if context.settled && context.canonical && context.echo && context.foreground_pid.is_some()
        {
            break context;
        }
        assert!(std::time::Instant::now() < until);
        std::thread::sleep(std::time::Duration::from_millis(10));
    };
    let mut wrong = context.clone();
    wrong.foreground_pid = Some(u32::MAX);
    assert!(
        client
            .request(Action::GuardedInput {
                attachment: attachment.clone(),
                input_id: Ulid::new(),
                bytes: b"wrong\r".to_vec(),
                expected: wrong
            })
            .is_err()
    );
    let id = Ulid::new();
    let Reply::Input { receipt } = client
        .request(Action::GuardedInput {
            attachment: attachment.clone(),
            input_id: id,
            bytes: b"guard fixture\r".to_vec(),
            expected: context,
        })
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(receipt.state, bbt_mux::InputState::PtyWritten);
    let mut output = Vec::new();
    while !String::from_utf8_lossy(&output).contains("GOT:guard fixture") {
        if let Ok(bbt_mux::Event::Output { bytes, .. }) =
            client.recv_timeout(std::time::Duration::from_millis(50))
        {
            output.extend(bytes);
        }
        assert!(
            std::time::Instant::now() < until,
            "guarded input did not reach fixture"
        );
    }
    supervisor.shutdown().unwrap();
}

#[test]
fn batching_never_jumps_over_an_earlier_eligible_route_in_worker_fifo() {
    let mut f = Fixture::new();
    for p in ["channels/one", "channels/two"] {
        let mut folder = Node::folder(p);
        folder.grants.insert(f.a.id, vec![Right::Write]);
        f.messaging
            .tree(f.a.swarm)
            .unwrap()
            .put(Actor::Operator, folder)
            .unwrap();
        let mut link = Node::link(format!("{p}/b"), format!("seats/{}/inbox", f.b.id));
        link.rate = Rate {
            batch_ms: 1,
            coalesce: true,
        };
        f.messaging
            .tree(f.a.swarm)
            .unwrap()
            .put(Actor::Operator, link)
            .unwrap();
    }
    let mut ids = Vec::new();
    for dest in ["#one/*", "#two/*", "#one/*"] {
        ids.push(
            f.messaging
                .send(f.a.id, 1, SendRequest::text(dest, "fifo"), 1000)
                .unwrap()
                .envelope
                .id,
        );
    }
    // One wake carries all three across both routes, in FIFO order.
    let work = f.take(1010);
    let got: Vec<_> = work.messages.iter().map(|m| m.envelope.id).collect();
    assert_eq!(got, ids);
    f.finish(&work, Evidence::NativeWritten, 1010);
    assert!(
        f.messaging
            .prepare(f.b.swarm, f.b.id, 1010)
            .unwrap()
            .is_none()
    );
}

#[test]
fn muted_peer_cannot_claim_the_authenticated_operator_direct_control_exception() {
    use bbt_mux::messaging::api::{Request, Response};
    let mut f = Fixture::new();
    f.messaging
        .set_mute(Actor::Operator, f.b.id, true, State::Idle, 1000)
        .unwrap();
    let ordinary = f.send(f.a.id, "ordinary peer", 1001);
    assert!(
        f.messaging.receipts(f.a.id, ordinary.envelope.id).unwrap()[0]
            .held
            .is_some()
    );
    let request = SendRequest::text(format!("seats/{}/inbox", f.b.id), "operator one to one");
    let id = request.id;
    f.messaging
        .request(
            f.a.id,
            1,
            Request::Send { message: request },
            true,
            None,
            1002,
        )
        .unwrap();
    f.reopen();
    let work = f.take(1003);
    assert_eq!(work.messages[0].envelope.id, id);
    f.finish(&work, Evidence::NativeWritten, 1004);
    let Response::Stored { message } = f
        .messaging
        .request(
            f.b.id,
            1,
            Request::Send {
                message: SendRequest::text(
                    format!("seats/{}/inbox", f.a.id),
                    "operator replies while seat muted",
                ),
            },
            true,
            None,
            1005,
        )
        .unwrap()
    else {
        panic!()
    };
    assert!(message.held_out.is_none());
    let Response::Stored { message } = f
        .messaging
        .request(
            f.b.id,
            1,
            Request::Send {
                message: SendRequest::text(
                    format!("swarms/{}/*", f.b.swarm),
                    "operator swarm post remains subject to mute",
                ),
            },
            true,
            None,
            1006,
        )
        .unwrap()
    else {
        panic!()
    };
    assert!(message.held_out.is_some());
    assert!(
        f.messaging.receipts(f.a.id, ordinary.envelope.id).unwrap()[0]
            .held
            .is_some()
    );
}

#[test]
fn arrival_path_limit_rejects_instead_of_silently_losing_an_unpaused_route() {
    let tmp = tempfile::tempdir().unwrap();
    let mut tree = Tree::open(SqliteStore::open(tmp.path()).unwrap(), Ulid::new()).unwrap();
    let seat = Ulid::new();
    add_sink(&mut tree, seat);
    tree.put(Actor::Operator, Node::folder("channels/many"))
        .unwrap();
    for i in 0..65 {
        let mut node = Node::link(
            format!("channels/many/{i:02}"),
            format!("seats/{seat}/inbox"),
        );
        node.paused = i < 64;
        tree.put(Actor::Operator, node).unwrap();
    }
    assert!(
        tree.resolve(
            Actor::Operator,
            "#many/*",
            MessageKind::Text,
            Class::Worker,
            0
        )
        .unwrap_err()
        .to_string()
        .contains("arrival paths")
    );
}
