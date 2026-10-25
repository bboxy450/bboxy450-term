//! The invariant crosses the actual admitted GUI door and durable events lane.
#![allow(clippy::disallowed_methods, reason = "private unattended IO fixture")]

use bbt_mux::{
    Config, LaunchSpec, Supervisor, Ulid,
    identity::{Custody, DeviceKeyStore, PendingReason},
    logic::{Logic, LogicConfig},
    messaging::wire::EnvelopeExt,
    registry::SeatRecord,
    store::{Store as _, events::EventLog, object_dir, sqlite::SqliteStore},
};
use bbt_proto::{
    GuiAction, GuiReply, GuiRequest,
    events::{GuiEventEnvelope, GuiEventPayload},
    messaging::{
        MessageRow, OperatorReference, OperatorReferenceKind, Request, Response, SendRequest,
        SignatureState,
    },
};
use ed25519_dalek::SigningKey;
use std::{
    collections::BTreeMap,
    os::unix::fs::PermissionsExt,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tokio::{
    io::{AsyncWriteExt, BufReader},
    net::{
        UnixStream,
        unix::{OwnedReadHalf, OwnedWriteHalf},
    },
};

struct FixtureCustody {
    held: AtomicBool,
    keys: Mutex<BTreeMap<Ulid, SigningKey>>,
}
impl DeviceKeyStore for FixtureCustody {
    fn custody(&self) -> Custody {
        Custody::Memory
    }
    fn load(&self, id: Ulid) -> bbt_mux::identity::Result<Option<SigningKey>> {
        if self.held.load(Ordering::Acquire) {
            return Err(PendingReason::KeychainUnavailable(-25308));
        }
        Ok(self.keys.lock().unwrap().get(&id).cloned())
    }
    fn create(&self, id: Ulid, key: &SigningKey) -> bbt_mux::identity::Result<()> {
        assert!(
            !self.held.load(Ordering::Acquire),
            "a held store cannot mint an author"
        );
        assert!(self.keys.lock().unwrap().insert(id, key.clone()).is_none());
        Ok(())
    }
}

struct Gui {
    reader: BufReader<OwnedReadHalf>,
    writer: OwnedWriteHalf,
    capability: [u8; 32],
    events: Vec<GuiEventEnvelope>,
}
impl Gui {
    async fn send(&mut self, action: GuiAction) {
        let mut bytes = serde_json::to_vec(&GuiRequest {
            version: bbt_proto::gui::VERSION,
            capability: self.capability,
            action,
        })
        .unwrap();
        bytes.push(b'\n');
        self.writer.write_all(&bytes).await.unwrap();
    }
    async fn until(&mut self, accept: impl Fn(&GuiReply) -> bool) -> GuiReply {
        tokio::time::timeout(Duration::from_secs(90), async {
            loop {
                let bytes = bbt_mux::seat_socket::read_line(&mut self.reader)
                    .await
                    .unwrap()
                    .expect("GUI disconnected");
                let reply: GuiReply = serde_json::from_slice(&bytes).unwrap();
                if let GuiReply::Event {
                    stream,
                    seq,
                    envelope,
                    ..
                } = &reply
                {
                    assert!(self.events.len() < 2048, "fixture event bound");
                    self.events.push((**envelope).clone());
                    // Publication to this fixture's owned projection precedes ACK.
                    self.send(GuiAction::EventAck {
                        stream: stream.clone(),
                        seq: *seq,
                    })
                    .await;
                }
                if accept(&reply) {
                    return reply;
                }
                if let GuiReply::Error { reason, .. } = &reply {
                    panic!("unexpected GUI refusal: {reason}");
                }
            }
        })
        .await
        .unwrap_or_else(|_| {
            let rows: Vec<_> = self
                .events
                .iter()
                .filter_map(|event| match &event.payload {
                    GuiEventPayload::Message { row } => Some((row.id, row.signature)),
                    _ => None,
                })
                .collect();
            panic!("GUI made no matching progress; retained message rows: {rows:?}")
        })
    }
    async fn request(&mut self, swarm: Ulid, request: Request) -> Response {
        let id = match &request {
            Request::Send { message } => message.id,
            _ => Ulid::new(),
        };
        self.send(GuiAction::Messaging {
            id,
            swarm_id: bbt_proto::SwarmId::from_u128(swarm.0),
            request,
        })
        .await;
        match self.until(|r| matches!(r, GuiReply::Messaging {op, ..} | GuiReply::Error {op, ..} if *op == id)).await {
            GuiReply::Messaging {response, ..} => response,
            other => panic!("messaging failed: {other:?}"),
        }
    }
    fn row(&self, id: Ulid, signature: SignatureState) -> Option<&MessageRow> {
        self.events
            .iter()
            .rev()
            .find_map(|event| match &event.payload {
                GuiEventPayload::Message { row } if row.id == id && row.signature == signature => {
                    Some(row)
                }
                _ => None,
            })
    }
}

pub(super) fn exercise_held_startup() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(exercise());
}

async fn exercise() {
    let root = tempfile::Builder::new()
        .prefix("ng1-gui-")
        .tempdir_in("/tmp")
        .unwrap();
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let repository = root.path().join("repository");
    std::fs::create_dir(&repository).unwrap();
    let store = Arc::new(FixtureCustody {
        held: AtomicBool::new(true),
        keys: Mutex::new(BTreeMap::new()),
    });
    let supervisor = Supervisor::start(Config {
        journal_directory: Some(
            object_dir(root.path(), "supervisor")
                .unwrap()
                .join("journal"),
        ),
        ..Default::default()
    })
    .unwrap();
    let logic = Logic::start_custodied_with_store(
        supervisor.connect().unwrap(),
        LogicConfig {
            run_dir: root.path().into(),
            repository,
            ring: Default::default(),
        },
        store.clone(),
        &tokio::runtime::Handle::current(),
    )
    .unwrap();
    let status = logic.client.identity_status().await.unwrap();
    assert!(
        status.device_id.is_none(),
        "first startup must have no public device"
    );
    let swarm = Ulid::new();
    let sender = SeatRecord::new(Ulid::new(), swarm, "sender", "zsh");
    let receiver = SeatRecord::new(Ulid::new(), swarm, "receiver", "zsh");
    for record in [&sender, &receiver] {
        let mut launch = LaunchSpec::new("/bin/zsh", root.path());
        launch.seat = record.id;
        launch.args = vec![
            "-f".into(),
            "-c".into(),
            "while IFS= read -r line; do print -r -- NG1:$line; done".into(),
        ];
        logic.client.spawn(record.clone(), launch).await.unwrap();
    }
    let endpoint = logic.client.gui_endpoint().await.unwrap();
    let (read, write) = UnixStream::connect(&endpoint.path)
        .await
        .unwrap()
        .into_split();
    let mut gui = Gui {
        reader: BufReader::new(read),
        writer: write,
        capability: endpoint.capability,
        events: vec![],
    };
    let attach = Ulid::new();
    gui.send(GuiAction::AttachEvents { id: attach }).await;
    assert!(matches!(
        gui.until(|r| matches!(r, GuiReply::Ok {op} | GuiReply::Error {op, ..} if *op == attach))
            .await,
        GuiReply::Ok { .. }
    ));
    let account = SqliteStore::open(&object_dir(root.path(), "account").unwrap()).unwrap();
    let reference_bytes = account
        .get("messaging/operator-reference")
        .unwrap()
        .unwrap();
    let reference: OperatorReference = serde_json::from_slice(&reference_bytes).unwrap();
    assert_eq!(reference.kind, OperatorReferenceKind::OperatorLocal);
    assert!(store.keys.lock().unwrap().is_empty());
    let mut operator_ids = vec![];
    for text in ["operator first", "operator second"] {
        let message = SendRequest::text(format!("seats/{}/inbox", receiver.id), text);
        operator_ids.push(message.id);
        let Response::Stored { message } = gui.request(swarm, Request::Send { message }).await
        else {
            panic!("operator delivery held")
        };
        assert!(message.envelope().is_none());
        assert_eq!(message.view().signature, SignatureState::Pending);
    }
    let mut seat_ids = vec![];
    for path in [
        format!("seats/{}/inbox", receiver.id),
        "operator/inbox".into(),
    ] {
        let message = SendRequest::text(path, "seat content while held");
        seat_ids.push(message.id);
        let Response::Stored { message } = logic
            .client
            .messaging(sender.id, 1, Request::Send { message })
            .await
            .unwrap()
        else {
            panic!("seat delivery held")
        };
        assert!(message.envelope().is_none());
    }
    let ids: Vec<_> = operator_ids.iter().chain(&seat_ids).copied().collect();
    // The operator lane contains its own sends/receipts and inbox, not another
    // seat's private inbox. Confirm that separate delivery through its owner.
    let Response::Messages { page } = logic
        .client
        .messaging(
            receiver.id,
            1,
            Request::List {
                thread: None,
                channel: None,
                after: 0,
                limit: 16,
            },
        )
        .await
        .unwrap()
    else {
        panic!("pending recipient inbox missing")
    };
    assert_eq!(
        page.items
            .iter()
            .map(|message| message.view().id)
            .collect::<Vec<_>>(),
        vec![operator_ids[0], operator_ids[1], seat_ids[0]]
    );
    assert!(page.items.iter().all(|message| message.envelope().is_none()
        && message.view().signature == SignatureState::Pending));
    let visible_ids: Vec<_> = operator_ids.iter().chain(&seat_ids[1..]).copied().collect();
    for id in &visible_ids {
        if gui.row(*id, SignatureState::Pending).is_none() {
            gui.until(|r| matches!(r, GuiReply::Event {envelope, ..} if matches!(&envelope.payload, GuiEventPayload::Message {row} if row.id == *id && row.signature == SignatureState::Pending))).await;
        }
        let row = gui.row(*id, SignatureState::Pending).unwrap();
        assert!(row.issuer_kind.is_none());
        assert!(
            row.signature_reason
                .as_deref()
                .unwrap()
                .contains("Keychain")
        );
    }
    let ledger_dir = object_dir(root.path(), &format!("swarm-{swarm}")).unwrap();
    let ledger = SqliteStore::open(&ledger_dir).unwrap();
    let originals: Vec<Vec<u8>> = ids
        .iter()
        .map(|id| {
            ledger
                .connection()
                .unwrap()
                .query_row(
                    "SELECT envelope FROM msg_messages WHERE id=?1",
                    [id.to_string()],
                    |r| r.get(0),
                )
                .unwrap()
        })
        .collect();
    let event_log = EventLog::open(&ledger_dir).unwrap();
    let original_events: Vec<_> = gui.events.iter().filter(|event| matches!(&event.payload, GuiEventPayload::Message {row} if ids.contains(&row.id))).map(|event| {
        let seq = event.metadata.reference.position.sequence;
        (seq, serde_json::to_vec(&event_log.at(seq).unwrap().unwrap()).unwrap())
    }).collect();
    let consumer_names = || {
        let db = rusqlite::Connection::open_with_flags(
            ledger_dir.join("events.sqlite"),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        db.prepare("SELECT name FROM consumers WHERE name LIKE 'gui-%' ORDER BY name")
            .unwrap()
            .query_map([], |row| row.get::<_, String>(0))
            .unwrap()
            .collect::<std::result::Result<Vec<_>, _>>()
            .unwrap()
    };
    let consumers = consumer_names();
    assert_eq!(consumers, vec![format!("gui-{}", reference.id)]);
    let Response::Receipts { page } = gui
        .request(
            swarm,
            Request::Receipts {
                message: operator_ids[0],
                after: 0,
                limit: 16,
            },
        )
        .await
    else {
        panic!("operator receipt missing")
    };
    assert_eq!(page.items.len(), 1);
    assert_eq!(page.items[0].signature, SignatureState::Pending);
    assert!(page.items[0].sender_device.is_none());
    assert_eq!(page.items[0].sender_operator, Some(reference));
    store.held.store(false, Ordering::Release);
    for id in &visible_ids {
        if gui.row(*id, SignatureState::Signed).is_none() {
            gui.until(|r| matches!(r, GuiReply::Event {envelope, ..} if matches!(&envelope.payload, GuiEventPayload::Message {row} if row.id == *id && row.signature == SignatureState::Signed))).await;
        }
        assert!(
            gui.row(*id, SignatureState::Signed)
                .unwrap()
                .upgrade_of
                .is_some()
        );
    }
    let actual_device = logic
        .client
        .identity_status()
        .await
        .unwrap()
        .device_id
        .unwrap();
    assert_ne!(actual_device, reference.id);
    assert_eq!(
        account
            .get("messaging/operator-reference")
            .unwrap()
            .unwrap(),
        reference_bytes
    );
    assert_eq!(
        consumer_names(),
        consumers,
        "custody recovery must not move the consumer"
    );
    for (id, original) in ids.iter().zip(originals) {
        let retained: Vec<u8> = ledger
            .connection()
            .unwrap()
            .query_row(
                "SELECT envelope FROM msg_messages WHERE id=?1",
                [id.to_string()],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(retained, original, "intent bytes changed after delivery");
    }
    for (seq, original) in original_events {
        assert_eq!(
            serde_json::to_vec(&event_log.at(seq).unwrap().unwrap()).unwrap(),
            original,
            "retained event bytes changed"
        );
    }
    let Response::Messages { page } = gui
        .request(
            swarm,
            Request::List {
                thread: None,
                channel: None,
                after: 0,
                limit: 16,
            },
        )
        .await
    else {
        panic!("operator inbox missing")
    };
    assert!(
        page.items
            .iter()
            .any(|message| message.view().id == seat_ids[1])
    );
    let Response::Messages { page } = logic
        .client
        .messaging(
            receiver.id,
            1,
            Request::List {
                thread: None,
                channel: None,
                after: 0,
                limit: 16,
            },
        )
        .await
        .unwrap()
    else {
        panic!("receiver inbox missing")
    };
    assert_eq!(
        page.items
            .iter()
            .map(|message| message.view().id)
            .collect::<Vec<_>>(),
        vec![operator_ids[0], operator_ids[1], seat_ids[0]]
    );
    for message in &page.items {
        message.envelope().unwrap().verify_signature().unwrap();
    }
    assert_eq!(
        page.items[..2]
            .iter()
            .map(|message| message.envelope().unwrap().sender_sequence)
            .collect::<Vec<_>>(),
        vec![1, 2]
    );
    assert_eq!(
        ledger
            .connection()
            .unwrap()
            .query_row("SELECT count(*) FROM msg_signatures", [], |row| row
                .get::<_, i64>(0))
            .unwrap(),
        ids.len() as i64
    );
    let devices: Vec<Option<String>> = ledger
        .connection()
        .unwrap()
        .prepare("SELECT actual_device FROM msg_signatures ORDER BY message")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<std::result::Result<_, _>>()
        .unwrap();
    assert_eq!(devices, vec![Some(actual_device.to_string()); ids.len()]);
    drop(gui);
    logic.shutdown().unwrap();
    supervisor.shutdown().unwrap();
}
