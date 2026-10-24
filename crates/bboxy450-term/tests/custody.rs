//! The GUI remains useful while its signing provider is unavailable.
use std::{
    collections::{BTreeMap, btree_map::Entry},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use bboxy450_term::{grids::Grids, model};
use bbt_mux::{
    LaunchSpec, Supervisor, Ulid,
    identity::{Custody, DeviceKeyStore, PendingReason},
    logic::{Logic, LogicConfig},
    messaging::{
        api::{Request, Response},
        wire::SendRequest,
    },
    registry::SeatRecord,
    seat_socket::read_line,
};
use bbt_proto::{
    GuiAction, GuiIdentity, GuiIdentityState, GuiReply, GuiRequest, SeatId, Update, View,
};
use ed25519_dalek::SigningKey;
use tokio::{
    io::{AsyncWriteExt, BufReader},
    net::{
        UnixStream,
        unix::{OwnedReadHalf, OwnedWriteHalf},
    },
};

struct Store {
    unavailable: AtomicBool,
    keys: Mutex<BTreeMap<Ulid, SigningKey>>,
}
#[allow(
    clippy::disallowed_methods,
    reason = "memory-only fault store used by headless IO fixtures"
)]
impl DeviceKeyStore for Store {
    fn custody(&self) -> Custody {
        Custody::Memory
    }
    fn load(&self, id: Ulid) -> bbt_mux::identity::Result<Option<SigningKey>> {
        if self.unavailable.load(Ordering::Acquire) {
            return Err(PendingReason::KeychainUnavailable(-25308));
        }
        Ok(self.keys.lock().unwrap().get(&id).cloned())
    }
    fn create(&self, id: Ulid, key: &SigningKey) -> bbt_mux::identity::Result<()> {
        match self.keys.lock().unwrap().entry(id) {
            Entry::Vacant(entry) => {
                entry.insert(key.clone());
                Ok(())
            }
            Entry::Occupied(_) => Err(PendingReason::JournalConflict),
        }
    }
}

async fn send(writer: &mut OwnedWriteHalf, capability: [u8; 32], action: GuiAction) {
    let mut line = serde_json::to_vec(&GuiRequest {
        version: bbt_proto::gui::VERSION,
        capability,
        action,
    })
    .unwrap();
    line.push(b'\n');
    writer.write_all(&line).await.unwrap();
}
async fn receive(reader: &mut BufReader<OwnedReadHalf>) -> GuiReply {
    serde_json::from_slice(&read_line(reader).await.unwrap().expect("GUI closed")).unwrap()
}
async fn apply(
    reply: GuiReply,
    grids: &mut Grids,
    writer: &mut OwnedWriteHalf,
    capability: [u8; 32],
) -> Option<GuiIdentity> {
    let reply = match reply {
        GuiReply::Event { envelope, .. } => match envelope.payload {
            bbt_proto::events::GuiEventPayload::Identity { identity } => {
                GuiReply::Identity(identity)
            }
            _ => return None,
        },
        GuiReply::Error { reason, .. } if reason.starts_with("GuiEventsHeld:") => return None,
        reply => reply,
    };
    match reply {
        GuiReply::Identity(identity) => Some(identity),
        GuiReply::Update { seat, update } => {
            let (view, sequence) = match &update {
                Update::Snapshot { snapshot } => (snapshot.view, snapshot.sequence),
                Update::Delta { delta } => (delta.view, delta.sequence),
            };
            grids.apply(seat, &update).unwrap();
            send(
                writer,
                capability,
                GuiAction::Applied {
                    seat,
                    view,
                    sequence,
                },
            )
            .await;
            None
        }
        GuiReply::Error { reason, .. } => panic!("GUI operation rejected: {reason}"),
        _ => None,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[allow(
    clippy::disallowed_methods,
    reason = "headless test entry point creates its own Tokio runtime"
)]
async fn custody_hold_keeps_gui_echo_live_and_releases_held_sends_on_retry() {
    let root = tempfile::tempdir().unwrap();
    let repository = root.path().join("repository");
    tokio::fs::create_dir(&repository).await.unwrap();
    let store = Arc::new(Store {
        unavailable: AtomicBool::new(true),
        keys: Mutex::new(BTreeMap::new()),
    });
    let supervisor = Supervisor::start(bbt_mux::Config {
        journal_directory: Some(
            bbt_mux::store::object_dir(root.path(), "supervisor")
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
    let endpoint = logic.client.gui_endpoint().await.unwrap();
    let (read, mut write) = UnixStream::connect(&endpoint.path)
        .await
        .unwrap()
        .into_split();
    let mut read = BufReader::new(read);
    let mut grids = Grids::new();
    send(&mut write, endpoint.capability, GuiAction::ListSeats).await;
    let mut initial = None;
    loop {
        let reply = receive(&mut read).await;
        if matches!(reply, GuiReply::Seats(_)) {
            break;
        }
        initial = apply(reply, &mut grids, &mut write, endpoint.capability)
            .await
            .or(initial);
    }
    let initial = initial.expect("initial identity missing");
    assert_eq!(initial.state, GuiIdentityState::Pending);
    assert!(initial.reason.unwrap().contains("Keychain"));
    // A custody hold keeps this opt-in attachment pending; the same connection
    // receives the signed identity notice when custody becomes available.
    send(
        &mut write,
        endpoint.capability,
        GuiAction::AttachEvents { id: Ulid::new() },
    )
    .await;

    let spawn_id = Ulid::new();
    let spawn_spec = bbt_proto::SpawnSpec {
        swarm: bbt_proto::SwarmId::from_u128(71),
        harness_kind: "zsh".into(),
        name: Some("held operator spawn".into()),
        role: None,
        cwd: root.path().to_string_lossy().into_owned(),
        location: bbt_proto::Where::Local,
        resume: None,
        placement: None,
    };
    send(
        &mut write,
        endpoint.capability,
        GuiAction::Spawn {
            id: spawn_id,
            spec: spawn_spec.clone(),
        },
    )
    .await;
    loop {
        let reply = receive(&mut read).await;
        if let GuiReply::Error { op, reason } = &reply
            && *op == spawn_id
        {
            assert!(reason.starts_with("held:"));
            break;
        }
        apply(reply, &mut grids, &mut write, endpoint.capability).await;
    }
    assert!(logic.client.gui_seats().await.unwrap().is_empty());
    let account = bbt_mux::store::sqlite::SqliteStore::open(
        &bbt_mux::store::object_dir(root.path(), "account").unwrap(),
    )
    .unwrap();
    use bbt_mux::store::Store as _;
    assert!(
        account
            .get(&format!("control/op/{spawn_id}"))
            .unwrap()
            .is_none()
    );

    let mut launch = LaunchSpec::new("/bin/zsh", root.path());
    launch.args = vec![
        "-f".into(),
        "-c".into(),
        "printf 'GUI-READY\\n'; while read -r line; do printf 'GUI-ECHO:%s\\n' \"$line\"; done"
            .into(),
    ];
    let record = SeatRecord::new(launch.seat, Ulid::new(), "held GUI", "zsh");
    logic.client.spawn(record.clone(), launch).await.unwrap();
    let seat = SeatId::from_u128(record.id.0);
    for action in grids.set_watched(&[(
        seat,
        View {
            seat: record.id,
            generation: 1,
            view_generation: Ulid::nil(),
        },
    )]) {
        send(&mut write, endpoint.capability, action).await;
    }
    send(
        &mut write,
        endpoint.capability,
        GuiAction::Input {
            seat,
            generation: record.generation,
            id: Ulid::new(),
            bytes: b"held-probe\n".to_vec(),
        },
    )
    .await;
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            apply(
                receive(&mut read).await,
                &mut grids,
                &mut write,
                endpoint.capability,
            )
            .await;
            if let Some(screen) = grids.screen(seat) {
                // Project through C1b's API; the socket owner never implements
                // cell conversion or edits the Replica's screen contents.
                let grid =
                    model::screen_grid(&screen, bbt_ui_model::tokens::Theme::Dark, false).unwrap();
                let text: String = grid.cells.iter().map(|cell| cell.text.as_str()).collect();
                if text.contains("GUI-ECHO:held-probe") {
                    break;
                }
            }
        }
    })
    .await
    .expect("live GUI echo missing during custody hold");

    let mut ids = Vec::new();
    for text in ["first held", "second held"] {
        let message = SendRequest::text(format!("seats/{}/inbox", record.id), text);
        ids.push(message.id);
        let response = logic
            .client
            .messaging(record.id, 1, Request::Send { message })
            .await
            .unwrap();
        assert!(matches!(response, Response::Held {id, ..} if Some(&id)==ids.last()));
    }
    store.unavailable.store(false, Ordering::Release);
    tokio::time::timeout(Duration::from_secs(35), async {
        loop {
            if apply(
                receive(&mut read).await,
                &mut grids,
                &mut write,
                endpoint.capability,
            )
            .await
            .is_some_and(|identity| identity.state == GuiIdentityState::LocalDev)
            {
                break;
            }
        }
    })
    .await
    .expect("30s identity recovery did not reach the GUI connection");
    // This was definitely held before admission, so releasing the same request
    // is safe. Unknown admitted attempts, tested separately, never relaunch.
    send(
        &mut write,
        endpoint.capability,
        GuiAction::Spawn {
            id: spawn_id,
            spec: spawn_spec,
        },
    )
    .await;
    let spawned = loop {
        let reply = receive(&mut read).await;
        if let GuiReply::Spawned { op, seat } = &reply {
            assert_eq!(*op, spawn_id);
            break *seat;
        }
        apply(reply, &mut grids, &mut write, endpoint.capability).await;
    };
    let spawned = logic.client.gui_seat(Ulid(spawned.0.0)).await.unwrap();
    assert_eq!(spawned.record.role.class, bbt_mux::registry::Class::Worker);
    assert_eq!(spawned.record.role.name, "shell");
    let Response::Messages { page } = logic
        .client
        .messaging(
            record.id,
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
        panic!("message list missing")
    };
    assert_eq!(
        page.items
            .iter()
            .map(|message| message.envelope.id)
            .collect::<Vec<_>>(),
        ids
    );
    assert_eq!(
        page.items
            .iter()
            .map(|message| message.envelope.sender_sequence)
            .collect::<Vec<_>>(),
        vec![1, 2]
    );
    logic.shutdown().unwrap();
    supervisor.shutdown().unwrap();
}
