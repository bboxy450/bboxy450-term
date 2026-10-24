#![allow(clippy::disallowed_methods)] // Controlled blocking IO/process fixtures.

use bbt_mux::messaging::wire::EnvelopeExt;
mod common;
use std::io::{Cursor, Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use bbt_mux::logic::{Logic, LogicConfig};
use bbt_mux::mcp::{
    Call,
    bridge::{Bridge, Config as BridgeConfig, read_frame},
};
use bbt_mux::registry::{Class, Registry, SeatRecord};
use bbt_mux::store::{Events, Store, events::EventLog, sqlite::SqliteStore};
use bbt_mux::{LaunchSpec, Supervisor, Ulid};
use ed25519_dalek::SigningKey;
use serde_json::{Value, json};

struct Fixture {
    runtime: tokio::runtime::Runtime,
    logic: Option<Logic>,
    supervisor: Option<Supervisor>,
    config: LogicConfig,
    own: SeatRecord,
    peer: SeatRecord,
    oracle: SeatRecord,
    outside: SeatRecord,
    task: Ulid,
    foreign_task: Ulid,
    input_attempts: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    history_gap: std::sync::Arc<std::sync::atomic::AtomicBool>,
    _temp: tempfile::TempDir,
}
impl Fixture {
    fn new() -> Self {
        Self::with_lost_input_receipt(false)
    }
    fn with_lost_input_receipt(lose: bool) -> Self {
        let temp = tempfile::tempdir().unwrap();
        let repository = temp.path().join("repository");
        std::fs::create_dir_all(repository.join(".bboxy/tasks")).unwrap();
        let swarm = Ulid::new();
        let task = Ulid::new();
        let foreign_task = Ulid::new();
        let mut own = SeatRecord::new(Ulid::new(), swarm, "Own", "zsh");
        own.tasks.push(task);
        let peer = SeatRecord::new(Ulid::new(), swarm, "Peer", "zsh");
        let mut oracle = SeatRecord::new(Ulid::new(), swarm, "Oracle", "zsh");
        oracle.role.name = "oracle".into();
        oracle.role.class = Class::Oracle;
        let outside = SeatRecord::new(Ulid::new(), Ulid::new(), "Outside", "zsh");
        for (number, (id, seat, title)) in [
            (task, own.id, "Own fixture task"),
            (foreign_task, outside.id, "Foreign fixture task"),
        ]
        .into_iter()
        .enumerate()
        {
            let number = number + 1;
            std::fs::write(repository.join(format!(".bboxy/tasks/{id}.md")),format!("---\nid: {id}\ntype: task\nnumber: {number}\ntitle: {title}\nstatus: running\nassignee: {seat}\nverify: 'true'\n---\nSynthetic fixture brief only.\n")).unwrap();
        }
        let config = LogicConfig {
            run_dir: temp.path().join("run"),
            repository,
            ring: Default::default(),
        };
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .unwrap();
        let supervisor = Supervisor::start(common::capture_config(temp.path())).unwrap();
        let input_attempts = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let history_gap = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let logic = Logic::start(
            LostInputReceipt {
                client: supervisor.connect().unwrap(),
                lose,
                attempts: input_attempts.clone(),
                history_gap: history_gap.clone(),
            },
            config.clone(),
            key(),
            runtime.handle(),
        )
        .unwrap();
        for record in [&own, &peer, &oracle, &outside] {
            let mut spec = LaunchSpec::new("/bin/zsh", temp.path());
            spec.seat = record.id;
            spec.generation = record.generation;
            spec.args = vec![
                "-f".into(),
                "-c".into(),
                "while IFS= read -r line; do :; done".into(),
            ];
            runtime
                .block_on(logic.client.spawn(record.clone(), spec))
                .unwrap();
        }
        Self {
            runtime,
            logic: Some(logic),
            supervisor: Some(supervisor),
            config,
            own,
            peer,
            oracle,
            outside,
            task,
            foreign_task,
            input_attempts,
            history_gap,
            _temp: temp,
        }
    }
    fn client(&self) -> &bbt_mux::logic::LogicClient {
        &self.logic.as_ref().unwrap().client
    }
    fn bridge(&self, seat: Ulid) -> Bridge {
        let info = self.runtime.block_on(self.client().endpoint(seat)).unwrap();
        Bridge::new(BridgeConfig {
            socket: info.path,
            seat,
            generation: info.generation,
            capability: info.capability,
        })
    }
    fn call(&self, seat: Ulid, op_id: Ulid, tool: &str, args: Value) -> Value {
        self.runtime
            .block_on(self.client().control_call(
                seat,
                1,
                Call {
                    op_id,
                    tool: tool.into(),
                    args,
                },
            ))
            .unwrap()
    }
    fn registry(&self) -> Registry {
        Registry::new(SqliteStore::open(&self.config.run_dir.join("account")).unwrap())
    }
    fn mutate(&self, seat: Ulid, op_id: Ulid, tool: &str, mut args: Value) -> Value {
        args["op_id"] = json!(op_id);
        let mut bridge = self.bridge(seat);
        initialize(&mut bridge);
        let reply = rpc(
            &mut bridge,
            2,
            "tools/call",
            json!({"name":tool,"arguments":args}),
        );
        assert!(reply.get("error").is_none(), "{reply}");
        reply["result"].clone()
    }
    fn audits(&self, seat: Ulid, count: usize) -> Vec<bbt_mux::store::events::Envelope> {
        let until = Instant::now() + Duration::from_secs(5);
        loop {
            let log = EventLog::open(&self.config.run_dir.join(format!("seat-{seat}"))).unwrap();
            let events: Vec<_> = log
                .after(0, 128)
                .unwrap()
                .into_iter()
                .filter(|event| event.event.subject == format!("seat/{seat}/control"))
                .collect();
            if events.len() >= count {
                return events;
            }
            assert!(Instant::now() < until, "audit not published");
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    fn restart(&mut self) {
        self.logic.take().unwrap().shutdown().unwrap();
        self.logic = Some(
            Logic::start(
                self.supervisor.as_ref().unwrap().connect().unwrap(),
                self.config.clone(),
                key(),
                self.runtime.handle(),
            )
            .unwrap(),
        );
    }
}
struct LostInputReceipt {
    client: bbt_mux::Client,
    lose: bool,
    attempts: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    history_gap: std::sync::Arc<std::sync::atomic::AtomicBool>,
}
impl bbt_mux::logic::SupervisorConnection for LostInputReceipt {
    fn request(&mut self, action: bbt_mux::Action) -> bbt_mux::store::Result<bbt_mux::Reply> {
        let input = matches!(action, bbt_mux::Action::GuardedInput { .. });
        let mut result = self.client.request(action)?;
        if let bbt_mux::Reply::RingHistory { history } = &mut result
            && self
                .history_gap
                .swap(false, std::sync::atomic::Ordering::SeqCst)
        {
            history.records.clear();
            history.next = None;
            history.gap = Some(bbt_mux::ring::HistoryGap {
                requested_from: 1,
                requested_to: 10,
                retained_from: None,
                retained_to: None,
                reason: "fixture capture unavailable".into(),
            });
        }
        if input {
            self.attempts
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if std::mem::take(&mut self.lose) {
                return Err(bbt_mux::store::invalid("fixture lost the write receipt"));
            }
        }
        Ok(result)
    }
    fn receive(&mut self, timeout: Duration) -> bbt_mux::store::Result<Option<bbt_mux::Event>> {
        bbt_mux::logic::SupervisorConnection::receive(&mut self.client, timeout)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if let Some(logic) = self.logic.take() {
            let _ = logic.shutdown();
        }
        if let Some(supervisor) = self.supervisor.take() {
            let _ = supervisor.shutdown();
        }
    }
}
fn key() -> SigningKey {
    SigningKey::from_bytes(&[53; 32])
}

impl Fixture {
    fn prepare_operator(&self) -> Ulid {
        struct Provided(SigningKey);
        impl bbt_mux::identity::DeviceKeyStore for Provided {
            fn custody(&self) -> bbt_mux::identity::Custody {
                bbt_mux::identity::Custody::Memory
            }
            fn load(&self, _: Ulid) -> bbt_mux::identity::Result<Option<SigningKey>> {
                Ok(Some(self.0.clone()))
            }
            fn create(&self, _: Ulid, _: &SigningKey) -> bbt_mux::identity::Result<()> {
                Err(bbt_mux::identity::PendingReason::JournalConflict)
            }
        }
        assert!(matches!(
            bbt_mux::identity::local_dev_issuer_with_store(&self.config.run_dir, &Provided(key())),
            bbt_mux::identity::LocalIssuerOutcome::Ready(_)
        ));
        self.runtime
            .block_on(self.client().identity_status())
            .unwrap()
            .device_id
            .unwrap()
    }
    fn gui(&self, id: Ulid, action: bbt_proto::GuiAction) -> bbt_proto::GuiReply {
        let endpoint = self.runtime.block_on(self.client().gui_endpoint()).unwrap();
        let mut socket = UnixStream::connect(endpoint.path).unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut bytes = serde_json::to_vec(&bbt_proto::GuiRequest {
            version: bbt_proto::gui::VERSION,
            capability: endpoint.capability,
            action,
        })
        .unwrap();
        bytes.push(b'\n');
        socket.write_all(&bytes).unwrap();
        let mut read = std::io::BufReader::new(socket);
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            assert!(Instant::now() < deadline, "GUI operation {id} timed out");
            let bytes = read_frame(&mut read)
                .unwrap()
                .expect("admitted GUI stays open");
            let reply: bbt_proto::GuiReply = serde_json::from_slice(&bytes).unwrap();
            let matched = match &reply {
                bbt_proto::GuiReply::Messaging { op, .. }
                | bbt_proto::GuiReply::Task { op, .. }
                | bbt_proto::GuiReply::Error { op, .. } => *op == id,
                _ => false,
            };
            if matched {
                return reply;
            }
        }
    }
    fn gui_task(&self, request: bbt_proto::messaging::TaskRequest) -> bbt_proto::GuiReply {
        let id = Ulid::new();
        self.gui(id, bbt_proto::GuiAction::Task { id, request })
    }
}

#[test]
fn gui_operator_dm_replays_once_and_tasks_share_mcp_verification() {
    use bbt_proto::messaging::{AuthorKind, Request, Response, TaskRequest, TaskResponse};
    use bbt_proto::{GuiAction, GuiReply, SwarmId};
    let mut f = Fixture::new();
    let device = f.prepare_operator();
    let id = Ulid::new();
    let mut message = bbt_mux::messaging::SendRequest::text(
        format!("seats/{}/inbox", f.own.id),
        "GUI operator fixture DM",
    );
    message.id = id;
    let action = GuiAction::Messaging {
        id,
        swarm_id: SwarmId::from_u128(f.own.swarm.0),
        request: Request::Send { message },
    };
    let first = f.gui(id, action.clone());
    let GuiReply::Messaging {
        response: Response::Stored { message },
        ..
    } = &first
    else {
        panic!("{first:?}")
    };
    assert_eq!(message.envelope.seat, Ulid::nil());
    assert_eq!(message.envelope.process_generation, 0);
    assert_eq!(message.envelope.role.name, "operator");
    assert_eq!(message.envelope.role.class, Class::Oracle);
    assert_eq!(message.envelope.author, message.envelope.device);
    message.envelope.verify_signature().unwrap();
    let account = SqliteStore::open(&f.config.run_dir.join("account")).unwrap();
    let local_issuer = account.get("identity/local-issuer").unwrap().unwrap();
    assert_eq!(
        message.envelope.role_grant_id,
        bbt_mux::store::digest(&local_issuer)
    );
    let local_issuer: Value = serde_json::from_slice(&local_issuer).unwrap();
    assert_eq!(
        json!(message.envelope.role_epoch.to_string()),
        local_issuer["epoch"]
    );
    assert_eq!(
        serde_json::to_value(f.gui(id, action.clone())).unwrap(),
        serde_json::to_value(&first).unwrap()
    );
    f.restart();
    assert_eq!(
        serde_json::to_value(f.gui(id, action.clone())).unwrap(),
        serde_json::to_value(&first).unwrap()
    );
    let mut different = action;
    if let GuiAction::Messaging {
        request: Request::Send { message },
        ..
    } = &mut different
    {
        message.text = "reused id must not send".into();
    }
    assert!(
        matches!(f.gui(id,different), GuiReply::Error {reason,..} if reason.contains("different arguments"))
    );
    let message_count: i64 =
        SqliteStore::open(&f.config.run_dir.join(format!("swarm-{}", f.own.swarm)))
            .unwrap()
            .connection()
            .unwrap()
            .query_row(
                "SELECT COUNT(*) FROM msg_messages WHERE id=?1",
                [id.to_string()],
                |row| row.get(0),
            )
            .unwrap();
    assert_eq!(message_count, 1);
    let entry: Value =
        serde_json::from_slice(&account.get(&format!("control/op/{id}")).unwrap().unwrap())
            .unwrap();
    assert_eq!(entry["actor"]["device_id"], json!(device));
    assert_eq!(entry["tool"], "gui_messaging");
    let inbox = f.call(f.own.id, Ulid::new(), "message_list", json!({"limit":128}));
    assert!(!inbox["isError"].as_bool().unwrap(), "{inbox}");
    assert!(inbox.to_string().contains("GUI operator fixture DM"));
    let ack = f.mutate(f.own.id, Ulid::new(), "message_ack", json!({"message":id}));
    assert_eq!(ack["isError"], false, "{ack}");
    let receipt_op = Ulid::new();
    let receipts = f.gui(
        receipt_op,
        GuiAction::Messaging {
            id: receipt_op,
            swarm_id: SwarmId::from_u128(f.own.swarm.0),
            request: Request::Receipts {
                message: id,
                after: 0,
                limit: 128,
            },
        },
    );
    let GuiReply::Messaging {
        response: Response::Receipts { page },
        ..
    } = receipts
    else {
        panic!("{receipts:?}")
    };
    assert!(page.items.iter().any(|receipt| receipt.evidence == bbt_mux::adapters::Evidence::ApplicationAcknowledged));
    let published = Instant::now() + Duration::from_secs(5);
    let row = loop {
        let row = f
            .runtime
            .block_on(f.client().gui_message_row(f.own.swarm, id))
            .unwrap();
        let log = EventLog::open(&f.config.run_dir.join(format!("swarm-{}", f.own.swarm))).unwrap();
        let ack_published = log.after(0, 128).unwrap().iter().any(|event| {
            event.event.subject == "operator/inbox/receipt"
                && event.event.payload["message"] == json!(id)
                && event.event.payload["evidence"] == "application-acknowledged"
        });
        if row.bus.is_some() && ack_published {
            break row;
        }
        assert!(
            Instant::now() < published,
            "operator receipt or message bus position not published"
        );
        std::thread::sleep(Duration::from_millis(10));
    };
    assert_eq!(row.author.kind, AuthorKind::Operator);
    assert_eq!(row.author.id, device.to_string());
    assert_eq!(
        row.issuer_kind,
        bbt_mux::messaging::wire::IssuerKind::LocalDev
    );
    assert_eq!(row.receipts.acked, 1);
    assert_eq!(
        row.bus.as_ref().unwrap().stream,
        format!("swarm-{}", f.own.swarm)
    );
    let wrong = Ulid::new();
    let mut bad =
        bbt_mux::messaging::SendRequest::text(format!("seats/{}/inbox", f.own.id), "wrong swarm");
    bad.id = wrong;
    assert!(
        matches!(f.gui(wrong,GuiAction::Messaging {id:wrong,swarm_id:SwarmId::from_u128(f.outside.swarm.0),
        request:Request::Send {message:bad}}),GuiReply::Error {reason,..} if reason.contains("swarm mismatch"))
    );
    let wrong = Ulid::new();
    let mut bad = bbt_mux::messaging::SendRequest::text(
        format!("swarms/{}/**", f.outside.swarm),
        "wrong swarm path",
    );
    bad.id = wrong;
    bad.kind = bbt_mux::messaging::MessageKind::Decision;
    assert!(matches!(f.gui(wrong,GuiAction::Messaging {
        id:wrong,swarm_id:SwarmId::from_u128(f.own.swarm.0),request:Request::Send {message:bad}
    }),GuiReply::Error {reason,..} if reason.contains("swarm mismatch")));
    let wrong = Ulid::new();
    assert!(matches!(f.gui(wrong,GuiAction::Messaging {
        id:wrong,swarm_id:SwarmId::from_u128(f.outside.swarm.0),request:Request::Receipts {message:id,after:0,limit:128}
    }),GuiReply::Error {reason,..} if reason.contains("requested swarm")));

    assert!(matches!(
        f.gui_task(TaskRequest::Delegate {
            task_path: f.task_path(),
            seat_id: f.own.id,
            role: None
        }),
        GuiReply::Task {
            response: TaskResponse::Applied,
            ..
        }
    ));
    assert_eq!(
        f.task_document().metadata["delegated_by"],
        format!("operator:{device}")
    );
    let report = f.mutate(
        f.own.id,
        Ulid::new(),
        "task_report",
        json!({"task_path":f.task_path(),"summary":"reported without verification"}),
    );
    assert_eq!(report["isError"], false, "{report}");
    let refused = f.gui_task(TaskRequest::Accept {
        task_path: f.task_path(),
    });
    assert!(
        matches!(refused,GuiReply::Error {reason,..} if reason.contains("recorded verify_result"))
    );
    assert_eq!(f.task_document().metadata["status"], "reported");
    let report = f.report(verification("true", 0, json!([])));
    assert_eq!(report["isError"], false, "{report}");
    let accept_id = Ulid::new();
    let accept = GuiAction::Task {
        id: accept_id,
        request: TaskRequest::Accept {
            task_path: f.task_path(),
        },
    };
    let accepted = f.gui(accept_id, accept.clone());
    assert!(matches!(
        accepted,
        GuiReply::Task {
            response: TaskResponse::Applied,
            ..
        }
    ));
    assert_eq!(
        serde_json::to_value(f.gui(accept_id, accept)).unwrap(),
        serde_json::to_value(accepted).unwrap()
    );
    let document = f.task_document();
    assert_eq!(document.metadata["status"], "accepted");
    assert_eq!(
        document.metadata["authorship"]["signer"],
        format!("device:{device}")
    );
    assert_eq!(document.metadata["authorship"]["kind"], "local-dev");
    let listed = f.gui_task(TaskRequest::List {
        unassigned: false,
        swarm_id: Some(f.own.swarm),
        status: Some("accepted".into()),
        assignee: None,
        after: None,
        limit: Some(128),
    });
    let GuiReply::Task {
        response: TaskResponse::Items { items, .. },
        ..
    } = listed
    else {
        panic!("{listed:?}")
    };
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].id, f.task);
    assert_eq!(items[0].delegated_by, Some(format!("operator:{device}")));
    assert_eq!(items[0].verify.as_ref().unwrap().exit_code, 0);
    let status = f.gui_task(TaskRequest::Status {
        task_path: None,
        task_id: Some(f.task),
    });
    assert!(
        matches!(status,GuiReply::Task {response:TaskResponse::Item {item},..} if item.metadata["status"] == "accepted")
    );
    let subjects = f
        .runtime
        .block_on(f.client().gui_messaging_subjects())
        .unwrap();
    assert!(subjects.contains(&"operator/inbox/receipt".to_string()));
    assert!(subjects.contains(&format!("swarms/{}/message", f.own.swarm)));
    assert!(subjects.iter().all(|subject| !subject.contains(['*', '>'])));
}

impl Fixture {
    fn target(&self, seat: Ulid) -> Value {
        json!({"seat_id":seat,"expected_process_generation":"1"})
    }
    fn spawn_fixture(&self, kind: &str, script: &str) -> SeatRecord {
        let mut record = SeatRecord::new(Ulid::new(), self.own.swarm, "Controlled target", kind);
        record.harness_session_id = Some(format!("fixture-{}", record.id));
        let mut spec = LaunchSpec::new("/bin/zsh", self._temp.path());
        spec.seat = record.id;
        spec.generation = record.generation;
        spec.args = vec!["-f".into(), "-c".into(), script.into()];
        self.runtime
            .block_on(self.client().spawn(record.clone(), spec))
            .unwrap();
        record
    }
    fn screen(&self, seat: Ulid) -> String {
        let snapshot = self
            .runtime
            .block_on(self.client().snapshot(seat, 1))
            .unwrap();
        snapshot
            .screen
            .cells
            .iter()
            .map(|row| row.iter().map(|c| c.c).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }
    fn wait_screen(&self, seat: Ulid, text: &str) {
        let until = Instant::now() + Duration::from_secs(5);
        loop {
            if self.screen(seat).contains(text) {
                return;
            }
            assert!(
                Instant::now() < until,
                "missing {text:?}: {}",
                self.screen(seat)
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    fn wait_input_ready(&self, seat: Ulid) {
        let mut observed = None;
        let ready = self.runtime.block_on(async {
            tokio::time::timeout(Duration::from_secs(5), async {
                let mut quiet = None::<(u64, Instant)>;
                loop {
                    let status = self.client().adapter_status(seat, 1).await.unwrap();
                    let snapshot = self.client().snapshot(seat, 1).await.unwrap();
                    let prompt = status.state == bbt_mux::adapters::State::Idle
                        && status.source == "133"
                        && status.sequence == snapshot.sequence;
                    observed = Some((status, snapshot.sequence));
                    // The banner precedes OSC 133 B. Wait for that actual
                    // prompt and the guard's 50 ms settled-view condition.
                    if prompt {
                        match quiet {
                            Some((sequence, since))
                                if sequence == snapshot.sequence
                                    && since.elapsed() >= Duration::from_millis(50) =>
                            {
                                return;
                            }
                            Some((sequence, _)) if sequence == snapshot.sequence => {}
                            _ => quiet = Some((snapshot.sequence, Instant::now())),
                        }
                    } else {
                        quiet = None;
                    }
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            })
            .await
        });
        assert!(ready.is_ok(), "input readiness not observed: {observed:?}");
    }
    fn input_args(&self, seat: Ulid, text: &str, submit: bool) -> Value {
        let mut args = self.target(seat);
        args["text"] = json!(text);
        args["submit"] = json!(submit);
        args
    }
    fn epoch(&self, seat: Ulid) -> String {
        // Resolve policy through the production authority path before reading its version.
        let _ = self.call(seat, Ulid::new(), "self_status", json!({}));
        let store = SqliteStore::open(&self.config.run_dir.join("account")).unwrap();
        let epoch: i64 = store
            .connection()
            .unwrap()
            .query_row(
                "SELECT epoch FROM msg_policies WHERE seat=?1",
                [seat.to_string()],
                |row| row.get(0),
            )
            .unwrap();
        epoch.to_string()
    }
}

fn refused(value: Value, fragment: &str) -> Value {
    assert_eq!(value["isError"], true, "{value}");
    assert!(
        value.to_string().contains(fragment),
        "expected {fragment:?}: {value}"
    );
    value["structuredContent"].clone()
}

#[test]
fn mcp_seat_controls_enforce_catalogue_scope_generation_and_strict_arguments() {
    let f = Fixture::new();
    let controls = [
        "seat_input_text",
        "seat_spawn",
        "seat_mute",
        "seat_unmute",
        "seat_interrupt",
    ];
    for (caller, visible) in [(f.own.id, false), (f.oracle.id, true)] {
        let catalogue = f
            .runtime
            .block_on(f.client().control_catalogue(caller, 1))
            .unwrap();
        for name in controls {
            assert_eq!(
                catalogue["tools"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|t| t["name"] == name),
                visible
            );
        }
    }
    for tool in controls {
        refused(
            f.call(f.own.id, Ulid::new(), tool, f.target(f.peer.id)),
            "oracle-class",
        );
    }
    assert_eq!(
        f.audits(f.own.id, controls.len())
            .iter()
            .filter(|event| controls.contains(&event.event.payload["tool"].as_str().unwrap_or("")))
            .count(),
        5
    );
    for tool in ["screen_read", "history_read"] {
        assert_eq!(
            data(f.call(f.own.id, Ulid::new(), tool, f.target(f.own.id)))["seat_id"],
            json!(f.own.id)
        );
        refused(
            f.call(f.own.id, Ulid::new(), tool, f.target(f.peer.id)),
            "own seat",
        );
        refused(
            f.call(f.oracle.id, Ulid::new(), tool, f.target(f.outside.id)),
            "outside caller swarm",
        );
        let mut stale = f.target(f.peer.id);
        stale["expected_process_generation"] = json!("2");
        refused(
            f.call(f.oracle.id, Ulid::new(), tool, stale),
            "stale target process generation",
        );
        for revision in [
            json!(1),
            json!("0"),
            json!("01"),
            json!("18446744073709551616"),
        ] {
            let mut args = f.target(f.own.id);
            args["expected_process_generation"] = revision;
            refused(f.call(f.own.id, Ulid::new(), tool, args), "tool schema");
        }
        let mut spoof = f.target(f.own.id);
        spoof["operator"] = json!(true);
        refused(f.call(f.own.id, Ulid::new(), tool, spoof), "tool schema");
    }
    let no_proof = refused(
        f.mutate(
            f.oracle.id,
            Ulid::new(),
            "seat_input_text",
            f.input_args(f.peer.id, "DO-NOT-WRITE", true),
        ),
        "not ready: no positive readiness",
    );
    assert_eq!(no_proof["error"]["code"], "not_submitted");
    assert!(!f.screen(f.peer.id).contains("DO-NOT-WRITE"));
    let mut interrupt = f.target(f.peer.id);
    interrupt["text"] = json!("native only");
    refused(
        f.mutate(f.oracle.id, Ulid::new(), "seat_interrupt", interrupt),
        "interrupt unavailable for zsh",
    );
    assert!(
        f.runtime
            .block_on(f.client().snapshot(f.peer.id, 1))
            .is_ok()
    );
    let mut control_bytes = f.input_args(f.peer.id, "\u{3}", true);
    refused(
        f.mutate(
            f.oracle.id,
            Ulid::new(),
            "seat_input_text",
            control_bytes.clone(),
        ),
        "terminal control bytes",
    );
    control_bytes["text"] = json!("safe");
    control_bytes["caller"] = json!(f.oracle.id);
    refused(
        f.mutate(f.oracle.id, Ulid::new(), "seat_input_text", control_bytes),
        "tool schema",
    );
}

const READY_SHELL: &str = "stty icanon echo; printf 'FIXTURE-READY\\n'; while true; do printf '\\033]133;B\\007'; if IFS= read -r -t 1 line; then printf '\\033]133;C\\007'; printf 'OBSERVED:%s\\n' \"$line\"; sleep 0.6; fi; done";

#[test]
fn mcp_seat_input_consumes_readiness_and_enter_is_explicit() {
    // A single prompt keeps consumed readiness consumed, and the completed
    // read stays active until teardown rather than rearming after a sleep.
    const ONE_INPUT_SHELL: &str = "stty icanon echo; printf 'FIXTURE-READY\\n'; printf '\\033]133;B\\007'; if IFS= read -r line; then printf '\\033]133;C\\007'; printf 'OBSERVED:%s\\n' \"$line\"; fi; while IFS= read -r line; do :; done";
    let mut f = Fixture::new();
    let target = f.spawn_fixture("zsh", ONE_INPUT_SHELL);
    f.wait_screen(target.id, "FIXTURE-READY");
    f.wait_input_ready(target.id);
    let op = Ulid::new();
    let args = f.input_args(target.id, "ONE-PASTE", true);
    let first = f.mutate(f.oracle.id, op, "seat_input_text", args.clone());
    assert_eq!(data(first.clone())["evidence"], "native-written");
    f.wait_screen(target.id, "OBSERVED:ONE-PASTE");
    assert_eq!(
        f.mutate(f.oracle.id, op, "seat_input_text", args.clone()),
        first
    );
    refused(
        f.mutate(
            f.oracle.id,
            op,
            "seat_input_text",
            f.input_args(target.id, "DIFFERENT", true),
        ),
        "op_id_reused",
    );
    refused(
        f.mutate(
            f.oracle.id,
            Ulid::new(),
            "seat_input_text",
            f.input_args(target.id, "BUSY-PASTE", true),
        ),
        "not ready",
    );
    f.restart();
    assert_eq!(f.mutate(f.oracle.id, op, "seat_input_text", args), first);
    assert_eq!(f.screen(target.id).matches("OBSERVED:ONE-PASTE").count(), 1);
    assert!(!f.screen(target.id).contains("BUSY-PASTE"));

    let draft = f.spawn_fixture("zsh", ONE_INPUT_SHELL);
    f.wait_screen(draft.id, "FIXTURE-READY");
    f.wait_input_ready(draft.id);
    assert_eq!(
        data(f.mutate(
            f.oracle.id,
            Ulid::new(),
            "seat_input_text",
            f.input_args(draft.id, "DRAFT-ONLY", false)
        ))["status"],
        "applied"
    );
    f.wait_screen(draft.id, "DRAFT-ONLY");
    assert!(!f.screen(draft.id).contains("OBSERVED:DRAFT-ONLY"));
    refused(
        f.mutate(
            f.oracle.id,
            Ulid::new(),
            "seat_input_text",
            f.input_args(draft.id, "SECOND", true),
        ),
        "not ready",
    );
    // The fixture's operator, rather than another automatic paste, submits the draft.
    f.runtime
        .block_on(f.client().gui_input(draft.id, 1, Ulid::new(), vec![b'\r']))
        .unwrap();
    f.wait_screen(draft.id, "OBSERVED:DRAFT-ONLY");
    assert!(!f.screen(draft.id).contains("SECOND"));
}

#[test]
fn mcp_seat_input_lost_receipt_is_unknown_and_never_repeated_after_restart() {
    let mut f = Fixture::with_lost_input_receipt(true);
    let target = f.spawn_fixture("zsh", READY_SHELL);
    f.wait_screen(target.id, "FIXTURE-READY");
    std::thread::sleep(Duration::from_millis(80));
    let op = Ulid::new();
    let args = f.input_args(target.id, "UNKNOWN-PASTE", true);
    let first = f.mutate(f.oracle.id, op, "seat_input_text", args.clone());
    refused(first.clone(), "unknown");
    f.wait_screen(target.id, "OBSERVED:UNKNOWN-PASTE");
    assert_eq!(
        f.mutate(f.oracle.id, op, "seat_input_text", args.clone()),
        first
    );
    assert_eq!(
        f.input_attempts.load(std::sync::atomic::Ordering::SeqCst),
        1
    );
    f.restart();
    assert_eq!(f.mutate(f.oracle.id, op, "seat_input_text", args), first);
    assert_eq!(
        f.screen(target.id)
            .matches("OBSERVED:UNKNOWN-PASTE")
            .count(),
        1
    );
}

#[test]
fn mcp_seat_reads_page_real_capture_and_refuse_stale_view_cursors() {
    use base64::Engine;
    let f = Fixture::new();
    for text in ["CAPTURE-ONE\r", "CAPTURE-TWO\r", "CAPTURE-THREE\r"] {
        f.runtime
            .block_on(
                f.client()
                    .gui_input(f.own.id, 1, Ulid::new(), text.as_bytes().to_vec()),
            )
            .unwrap();
        f.wait_screen(f.own.id, text.trim_end_matches('\r'));
    }
    let mut args = f.target(f.own.id);
    args["limit_rows"] = json!(2);
    let page = data(f.call(f.own.id, Ulid::new(), "screen_read", args.clone()));
    assert_eq!(page["rows"].as_array().unwrap().len(), 2);
    assert!(page["next"].is_string());
    args["view_cursor"] = page["next"].clone();
    assert_eq!(
        data(f.call(f.own.id, Ulid::new(), "screen_read", args.clone()))["row_start"],
        2
    );
    f.runtime
        .block_on(
            f.client()
                .gui_input(f.own.id, 1, Ulid::new(), b"CHANGED\r".to_vec()),
        )
        .unwrap();
    f.wait_screen(f.own.id, "CHANGED");
    refused(
        f.call(f.own.id, Ulid::new(), "screen_read", args),
        "stale viewport cursor",
    );
    for (tool, limit) in [
        ("screen_read", "limit_rows"),
        ("history_read", "limit_records"),
    ] {
        for value in [0, 129] {
            let mut args = f.target(f.own.id);
            args[limit] = json!(value);
            refused(f.call(f.own.id, Ulid::new(), tool, args), "limit");
        }
    }
    let mut args = f.target(f.own.id);
    args["limit_records"] = json!(1);
    let mut output = Vec::new();
    let mut last = 0;
    let mut pages = 0;
    loop {
        let page = data(f.call(f.own.id, Ulid::new(), "history_read", args.clone()));
        assert_eq!(page["format"], "capture_records");
        assert!(page["gaps"].as_array().unwrap().is_empty());
        assert!(page["records"].as_array().unwrap().len() <= 1);
        for record in page["records"].as_array().unwrap() {
            let seq: u64 = record["sequence"].as_str().unwrap().parse().unwrap();
            assert!(seq > last);
            last = seq;
            if record["kind"] == "output" {
                assert_eq!(record["encoding"], "base64");
                let bytes = base64::engine::general_purpose::STANDARD
                    .decode(record["bytes"].as_str().unwrap())
                    .unwrap();
                assert_eq!(
                    bytes.len(),
                    record["byte_length"].as_u64().unwrap() as usize
                );
                output.extend(bytes);
            }
        }
        pages += 1;
        assert!(pages < 64, "capture cursor did not advance");
        if page["next"].is_null() {
            break;
        }
        args["cursor"] = page["next"].clone();
    }
    assert!(String::from_utf8(output).unwrap().contains("CAPTURE-THREE"));
    f.history_gap
        .store(true, std::sync::atomic::Ordering::SeqCst);
    let gap = data(f.call(f.own.id, Ulid::new(), "history_read", f.target(f.own.id)));
    assert!(gap["records"].as_array().unwrap().is_empty());
    assert_eq!(gap["gaps"][0]["reason"], "fixture capture unavailable");
    assert!(gap["next"].is_null());
}

#[test]
fn mcp_seat_mute_is_epoch_checked_signed_and_preserves_read_access() {
    let f = Fixture::new();
    let mut args = f.target(f.peer.id);
    let old_epoch = f.epoch(f.peer.id);
    args["expected_policy_epoch"] = json!(old_epoch);
    let op = Ulid::new();
    let first = f.mutate(f.oracle.id, op, "seat_mute", args.clone());
    let muted = data(first.clone());
    assert_eq!(muted["muted"], true);
    assert_ne!(muted["policy_epoch"], args["expected_policy_epoch"]);
    assert_eq!(f.mutate(f.oracle.id, op, "seat_mute", args.clone()), first);
    refused(
        f.mutate(f.oracle.id, Ulid::new(), "seat_unmute", args.clone()),
        "stale target policy epoch",
    );
    assert!(f.registry().get(f.peer.id).unwrap().muted);
    refused(
        f.mutate(
            f.oracle.id,
            Ulid::new(),
            "seat_input_text",
            f.input_args(f.peer.id, "NOT-WRITTEN", true),
        ),
        "muted target",
    );
    assert_eq!(
        data(f.call(f.peer.id, Ulid::new(), "screen_read", f.target(f.peer.id)))["seat_id"],
        json!(f.peer.id)
    );
    args["expected_policy_epoch"] = muted["policy_epoch"].clone();
    let already = data(f.mutate(f.oracle.id, Ulid::new(), "seat_mute", args.clone()));
    assert_eq!(already["notice_id"], muted["notice_id"]);
    let clear = data(f.mutate(f.oracle.id, Ulid::new(), "seat_unmute", args));
    assert_eq!(clear["muted"], false);
    assert_ne!(clear["notice_id"], muted["notice_id"]);

    let response = f
        .runtime
        .block_on(f.client().messaging(
            f.oracle.id,
            1,
            bbt_mux::messaging::api::Request::List {
                thread: None,
                channel: None,
                after: 0,
                limit: 128,
            },
        ))
        .unwrap();
    let bbt_mux::messaging::api::Response::Messages { page } = response else {
        panic!("messages expected");
    };
    let notices: Vec<_> = page
        .items
        .iter()
        .filter(|message| {
            message.envelope.seat == f.peer.id
                && matches!(
                    message.envelope.body,
                    bbt_mux::messaging::wire::Body::Presence { .. }
                )
        })
        .collect();
    assert_eq!(notices.len(), 2);
    for notice in notices {
        notice.envelope.verify_signature().unwrap();
    }

    let mut self_mute = f.target(f.oracle.id);
    self_mute["expected_policy_epoch"] = json!(f.epoch(f.oracle.id));
    let result = data(f.mutate(f.oracle.id, Ulid::new(), "seat_mute", self_mute.clone()));
    refused(
        f.mutate(
            f.oracle.id,
            Ulid::new(),
            "seat_input_text",
            f.input_args(f.peer.id, "DENIED", true),
        ),
        "muted caller",
    );
    refused(
        f.mutate(
            f.oracle.id,
            Ulid::new(),
            "seat_spawn",
            json!({"swarm_id":f.own.swarm,"approved_profile":"zsh","cwd":f._temp.path()}),
        ),
        "muted caller",
    );
    assert_eq!(
        data(f.call(f.oracle.id, Ulid::new(), "screen_read", f.target(f.peer.id)))["seat_id"],
        json!(f.peer.id)
    );
    self_mute["expected_policy_epoch"] = result["policy_epoch"].clone();
    assert_eq!(
        data(f.mutate(f.oracle.id, Ulid::new(), "seat_unmute", self_mute))["muted"],
        false
    );
}
fn data(value: Value) -> Value {
    assert_eq!(value["isError"], false, "{value}");
    value["structuredContent"].clone()
}
fn rpc(bridge: &mut Bridge, id: u64, method: &str, params: Value) -> Value {
    bridge
        .handle(
            &serde_json::to_vec(&json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}))
                .unwrap(),
        )
        .unwrap()
}
fn initialize(bridge: &mut Bridge) {
    let value = rpc(
        bridge,
        1,
        "initialize",
        json!({"protocolVersion":"2025-11-25","clientInfo":{"name":"fixture","version":"1"},"capabilities":{}}),
    );
    assert_eq!(value["result"]["protocolVersion"], "2025-11-25");
    assert_eq!(
        value["result"]["capabilities"]["tools"]["listChanged"],
        false
    );
    assert!(
        bridge
            .handle(br#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#)
            .is_none()
    );
}

#[test]
fn mcp_four_methods_forward_to_logic_and_explicit_metadata_replays_the_original_result() {
    let fixture = Fixture::new();
    let mut bridge = fixture.bridge(fixture.own.id);
    assert_eq!(
        rpc(&mut bridge, 0, "tools/list", json!({}))["error"]["code"],
        -32002
    );
    assert_eq!(rpc(&mut bridge, 0, "ping", json!({}))["result"], json!({}));
    initialize(&mut bridge);
    let list = rpc(&mut bridge, 2, "tools/list", json!({}));
    let tools = list["result"]["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 20);
    assert!(
        tools
            .iter()
            .filter(|tool| matches!(
                tool["name"].as_str(),
                Some("orient" | "self_status" | "seat_list" | "seat_get" | "graph_read")
            ))
            .all(|tool| tool["annotations"]["readOnlyHint"] == true)
    );
    assert_eq!(
        rpc(&mut bridge, 3, "resources/list", json!({}))["error"]["code"],
        -32601
    );
    let op_id = Ulid::new();
    let params = json!({"name":"orient","arguments":{},"_meta":{"bboxy450/op_id":op_id}});
    let first = rpc(&mut bridge, 4, "tools/call", params.clone());
    assert_eq!(first["result"]["isError"], false);
    let data = &first["result"]["structuredContent"];
    assert_eq!(data["actor"]["seat_id"], json!(fixture.own.id));
    assert_eq!(data["actor"]["process_generation"], 1);
    assert_eq!(data["actor"]["role"]["class"], "worker");
    assert_eq!(data["actor"]["role"]["name"], "executor");
    assert_eq!(data["tasks"]["items"][0]["id"], json!(fixture.task));
    assert_eq!(data["tasks"]["items"][0]["verify"], "true");
    assert!(data["unread_count"].is_null());
    assert_eq!(data["pending_application_ack"], 0);
    let second = rpc(&mut bridge, 5, "tools/call", params);
    assert_eq!(first["result"], second["result"]);
    let audit = fixture.audits(fixture.own.id, 2);
    assert_eq!(audit[0].event.payload["tool"], "orient");
    assert_eq!(audit[0].event.generation, Some(1));
    assert_eq!(audit[1].event.payload["outcome"], "replay");
    let minted = rpc(&mut bridge, 6, "tools/call", json!({"name":"self_status"}));
    assert!(
        minted["result"]["structuredContent"]["op_id"]
            .as_str()
            .unwrap()
            .parse::<Ulid>()
            .is_ok()
    );
}

#[test]
fn mcp_role_scope_and_schema_cannot_be_spoofed_and_graph_reads_hide_foreign_items() {
    let fixture = Fixture::new();
    let own = fixture.own.id;
    let list = fixture
        .runtime
        .block_on(fixture.client().control_catalogue(own, 1))
        .unwrap();
    let graph = list["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|tool| tool["name"] == "graph_read")
        .unwrap();
    assert_eq!(
        graph["inputSchema"]["properties"]["graph"]["enum"],
        json!(["task", "markdown"])
    );
    for (tool, args) in [
        ("seat_list", json!({"scope":"account"})),
        ("seat_get", json!({"seat_id":fixture.outside.id})),
        ("graph_read", json!({"graph":"execution"})),
        ("self_status", json!({"caller_role":"oracle"})),
        ("seat_input_text", json!({"text":"must never be typed"})),
        ("task_delegate", json!({})),
    ] {
        assert_eq!(fixture.call(own, Ulid::new(), tool, args)["isError"], true);
    }
    let peers = fixture.call(own, Ulid::new(), "seat_list", json!({}));
    assert_eq!(
        peers["structuredContent"]["items"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    let peer = fixture.call(
        own,
        Ulid::new(),
        "seat_get",
        json!({"seat_id":fixture.peer.id}),
    );
    assert_eq!(peer["isError"], false);
    for graph in ["task", "markdown"] {
        let data = fixture.call(own, Ulid::new(), "graph_read", json!({"graph":graph}));
        let text = data.to_string();
        assert!(text.contains(&fixture.task.to_string()));
        assert!(!text.contains(&fixture.foreign_task.to_string()));
    }
    let all = fixture.call(
        fixture.oracle.id,
        Ulid::new(),
        "seat_list",
        json!({"scope":"account"}),
    );
    assert_eq!(
        all["structuredContent"]["items"].as_array().unwrap().len(),
        4
    );
    let execution = fixture.call(
        fixture.oracle.id,
        Ulid::new(),
        "graph_read",
        json!({"graph":"execution"}),
    );
    assert_eq!(execution["isError"], false);
    assert!(
        execution
            .to_string()
            .contains(&fixture.foreign_task.to_string())
    );
    let audit = fixture.audits(own, 6);
    assert_eq!(
        audit
            .iter()
            .filter(|e| e.event.payload["outcome"] == "rejected")
            .count(),
        6
    );
}

#[test]
fn mcp_journal_survives_logic_restart_and_never_replays_a_result_under_changed_authority() {
    let mut fixture = Fixture::new();
    let own = fixture.own.id;
    let op = Ulid::new();
    let first = fixture.call(own, op, "self_status", json!({}));
    fixture
        .registry()
        .update(own, |record| {
            record.name = "Changed".into();
            Ok(())
        })
        .unwrap();
    let repeated = fixture.call(own, op, "self_status", json!({}));
    assert_eq!(first, repeated);
    let store = SqliteStore::open(&fixture.config.run_dir.join(format!("seat-{own}"))).unwrap();
    let bytes = store.get(&format!("control/op/{op}")).unwrap().unwrap();
    let conflict = fixture.call(own, op, "seat_get", json!({"seat_id":own}));
    assert_eq!(
        conflict["structuredContent"]["error"]["code"],
        "op_id_reused"
    );
    assert_eq!(
        store.get(&format!("control/op/{op}")).unwrap().unwrap(),
        bytes
    );
    fixture.audits(own, 3);
    fixture.restart();
    assert_eq!(fixture.call(own, op, "self_status", json!({})), first);
    let oracle = fixture.oracle.id;
    let privileged = Ulid::new();
    assert_eq!(
        fixture.call(
            oracle,
            privileged,
            "graph_read",
            json!({"graph":"execution"})
        )["isError"],
        false
    );
    fixture
        .registry()
        .update(oracle, |record| {
            record.role.class = Class::Worker;
            record.role.name = "oracle".into();
            Ok(())
        })
        .unwrap();
    let old = fixture.call(
        oracle,
        privileged,
        "graph_read",
        json!({"graph":"execution"}),
    );
    assert_eq!(old["isError"], true);
    assert_eq!(old["structuredContent"]["error"]["code"], "op_id_reused");
    assert_eq!(
        fixture.call(
            oracle,
            Ulid::new(),
            "graph_read",
            json!({"graph":"execution"})
        )["isError"],
        true
    );
}

#[test]
fn mcp_socket_refuses_bad_capabilities_and_generations_and_audit_does_not_store_arguments() {
    let fixture = Fixture::new();
    let own = fixture.own.id;
    let endpoint = fixture
        .runtime
        .block_on(fixture.client().endpoint(own))
        .unwrap();
    for (capability, generation) in [([0; 32], 1), (endpoint.capability, 2)] {
        for action in [
            bbt_mux::seat_socket::Action::Catalogue,
            bbt_mux::seat_socket::Action::Call {
                call: Call {
                    op_id: Ulid::new(),
                    tool: "message_send".into(),
                    args: json!({"destination":format!("seats/{}/inbox",fixture.peer.id),"text":"must not execute"}),
                },
            },
        ] {
            let mut stream = UnixStream::connect(&endpoint.path).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let request = bbt_mux::seat_socket::Request {
                version: 1,
                capability,
                seat: own,
                generation,
                action,
            };
            let mut bytes = serde_json::to_vec(&request).unwrap();
            bytes.push(b'\n');
            stream.write_all(&bytes).unwrap();
            let mut out = Vec::new();
            let result = stream.read_to_end(&mut out);
            assert!(out.is_empty(), "unauthorized response bytes");
            assert!(
                matches!(result, Ok(0))
                    || result
                        .as_ref()
                        .is_err_and(|e| e.kind() == std::io::ErrorKind::ConnectionReset),
                "admission did not close the connection: {result:?}"
            );
        }
    }
    assert!(
        fixture
            .runtime
            .block_on(fixture.client().control_call(
                own,
                2,
                Call {
                    op_id: Ulid::new(),
                    tool: "orient".into(),
                    args: json!({})
                }
            ))
            .is_err()
    );
    for (tool, args) in [("bad\nname", json!({})), ("orient", json!([]))] {
        assert!(
            fixture
                .runtime
                .block_on(fixture.client().control_call(
                    own,
                    1,
                    Call {
                        op_id: Ulid::new(),
                        tool: tool.into(),
                        args
                    }
                ))
                .is_err()
        );
    }
    let marker = "DO_NOT_PERSIST_CONTROL_ARGUMENT_PAYLOAD";
    assert_eq!(
        fixture.call(own, Ulid::new(), "self_status", json!({"extra":marker}))["isError"],
        true
    );
    let audit = fixture.audits(own, 1);
    let encoded = serde_json::to_string(&audit).unwrap();
    assert!(!encoded.contains(marker));
    assert!(
        audit[0].event.payload["args_digest"]
            .as_str()
            .unwrap()
            .len()
            == 64
    );
    audit[0].verify().unwrap();
}

#[test]
fn mcp_unread_is_unknown_and_application_ack_is_a_separate_count() {
    use bbt_mux::messaging::{api, wire::SendRequest};
    let fixture = Fixture::new();
    let own = fixture.own.id;
    let send = SendRequest::text(format!("seats/{own}/inbox"), "Synthetic MCP count fixture");
    let message = send.id;
    fixture
        .runtime
        .block_on(fixture.client().messaging(
            fixture.oracle.id,
            1,
            api::Request::Send { message: send },
        ))
        .unwrap();
    let first = fixture.call(own, Ulid::new(), "orient", json!({}));
    assert!(first["structuredContent"]["unread_count"].is_null());
    assert_eq!(first["structuredContent"]["pending_application_ack"], 1);
    fixture
        .runtime
        .block_on(fixture.client().messaging(
            own,
            1,
            api::Request::Acknowledge {
                message,
                agent: None,
            },
        ))
        .unwrap();
    let second = fixture.call(own, Ulid::new(), "orient", json!({}));
    assert!(second["structuredContent"]["unread_count"].is_null());
    assert_eq!(second["structuredContent"]["pending_application_ack"], 0);
}

#[test]
fn mcp_framing_is_bounded_and_protocol_errors_never_echo_input() {
    assert_eq!(
        read_frame(&mut Cursor::new(b"{}\nnext\n")).unwrap(),
        Some(b"{}".to_vec())
    );
    assert!(read_frame(&mut Cursor::new(b"truncated")).is_err());
    assert!(
        read_frame(&mut Cursor::new(vec![
            b'x';
            bbt_mux::mcp::bridge::MAX_FRAME + 1
        ]))
        .is_err()
    );
    let mut bridge = Bridge::new(BridgeConfig {
        socket: PathBuf::from("/does/not/exist"),
        seat: Ulid::new(),
        generation: 1,
        capability: [0; 32],
    });
    let secret = b"secret input that is not JSON";
    let error = bridge.handle(secret).unwrap();
    assert_eq!(error["error"]["code"], -32700);
    assert!(!error.to_string().contains("secret input"));
    assert_eq!(bridge.handle(b"[]").unwrap()["error"]["code"], -32600);
    initialize(&mut bridge);
    assert_eq!(
        rpc(
            &mut bridge,
            4,
            "tools/call",
            json!({"name":"orient","_meta":{"bboxy450/op_id":"invalid"}})
        )["error"]["code"],
        -32602
    );
    let unavailable = rpc(&mut bridge, 5, "tools/call", json!({"name":"orient"}));
    assert_eq!(unavailable["error"]["code"], -32001);
    assert_eq!(unavailable["error"]["data"]["status"], "unknown");
    assert_ne!(
        unavailable["error"]["data"]["op_id"]
            .as_str()
            .unwrap()
            .parse::<Ulid>()
            .unwrap(),
        Ulid::nil()
    );
    let original = Ulid::new();
    assert_eq!(
        rpc(
            &mut bridge,
            6,
            "tools/call",
            json!({"name":"orient", "_meta":{"bboxy450/op_id":original}})
        )["error"]["data"]["op_id"],
        original.to_string()
    );
    for args in [
        json!({"name":"bad\nname"}),
        json!({"name":"orient","arguments":[]}),
    ] {
        assert_eq!(
            rpc(&mut bridge, 7, "tools/call", args)["error"]["code"],
            -32602
        );
    }
}

/// Runs only by explicit request: calls the installed, authenticated clients
/// against synthetic seat state. Output is parsed in memory, never exported.
#[test]
#[ignore = "live installed-client interop; BBOXY_TEST_EXE must name the built app"]
fn mcp_installed_claude_and_codex_call_orient() {
    use std::process::{Command, Stdio};
    let fixture = Fixture::new();
    let exe = std::env::var("BBOXY_TEST_EXE").expect("set BBOXY_TEST_EXE to the built app");
    let cwd = fixture._temp.path().join("client-repo");
    std::fs::create_dir(&cwd).unwrap();
    assert!(
        Command::new("git")
            .args(["init", "-q"])
            .current_dir(&cwd)
            .status()
            .unwrap()
            .success()
    );
    let mcp_config = cwd.join("mcp-fixture.json");
    std::fs::write(
        &mcp_config,
        json!({"mcpServers":{"bboxy":{"command":exe,"args":["mcp"]}}}).to_string(),
    )
    .unwrap();
    let before = configuration_digests();
    let prompt = "This is a synthetic MCP interop fixture. Call the bboxy orient tool exactly once, then say the role name and class it returns. Use no other tools, read no files, and change no settings.";
    for (program, seat) in [("claude", fixture.own.id), ("codex", fixture.peer.id)] {
        let endpoint = fixture
            .runtime
            .block_on(fixture.client().endpoint(seat))
            .unwrap();
        let mut command = Command::new(program);
        command.current_dir(&cwd).stdin(Stdio::null());
        for name in [
            "CLAUDECODE",
            "CLAUDE_CODE_SESSION_ID",
            "CLAUDE_SESSION_ID",
            "CODEX_THREAD_ID",
            "CODEX_SESSION_ID",
            "CODEX_TURN_ID",
            "ICQ_SESSION_ID",
            "ICQ_SESSION_KEY",
        ] {
            command.env_remove(name);
        }
        command
            .env("BBOXY_SOCK", &endpoint.path)
            .env("BBOXY_SEAT_ID", seat.to_string())
            .env("BBOXY_GENERATION", endpoint.generation.to_string())
            .env(
                "BBOXY_CAPABILITY",
                endpoint
                    .capability
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>(),
            );
        if program == "claude" {
            command
                .args([
                    "-p",
                    prompt,
                    "--no-session-persistence",
                    "--tools",
                    "",
                    "--allowedTools",
                    "mcp__bboxy__orient",
                    "--permission-prompts",
                    "none",
                    "--output-format",
                    "json",
                    "--mcp-config",
                ])
                .arg(&mcp_config);
        } else {
            command.args(["exec","--ephemeral","--json","-s","read-only","-c","approval_policy=\"never\"","-c"])
                .arg(format!("mcp_servers.bboxy.command={}",json!(exe)))
                .args(["-c","mcp_servers.bboxy.args=[\"mcp\"]","-c","mcp_servers.bboxy.env_vars=[\"BBOXY_SOCK\",\"BBOXY_CAPABILITY\",\"BBOXY_SEAT_ID\",\"BBOXY_GENERATION\"]","-c","mcp_servers.bboxy.enabled_tools=[\"orient\"]"])
                .arg(prompt);
        }
        let (status, out, err) = bounded_client(command, Duration::from_secs(120));
        let diagnostics = String::from_utf8_lossy(&err).to_ascii_lowercase();
        assert!(
            status.success(),
            "{program} exit={status}; stdout_bytes={} stderr_bytes={} trust_text={} config_text={}",
            out.len(),
            err.len(),
            diagnostics.contains("trust"),
            diagnostics.contains("config")
        );
        let role_spoken = if program == "claude" {
            let result: Value = serde_json::from_slice(&out).expect("Claude JSON result");
            result["result"]
                .as_str()
                .is_some_and(|s| s.to_ascii_lowercase().contains("executor"))
        } else {
            out.split(|b| *b == b'\n')
                .filter_map(|line| serde_json::from_slice::<Value>(line).ok())
                .any(|event| {
                    event["type"] == "item.completed"
                        && event["item"]["type"] == "agent_message"
                        && event["item"]["text"]
                            .as_str()
                            .is_some_and(|s| s.to_ascii_lowercase().contains("executor"))
                })
        };
        assert!(
            role_spoken,
            "{program} final output did not report the returned role"
        );
        let audits = fixture.audits(seat, 1);
        let orient = audits
            .iter()
            .find(|event| event.event.payload["tool"] == "orient")
            .unwrap();
        assert_eq!(orient.event.seat, Some(seat));
        assert_eq!(orient.event.generation, Some(1));
        assert_eq!(orient.event.payload["actor"]["role"]["class"], "worker");
        orient.verify().unwrap();
        println!(
            "client={program} exit=0 tool=orient seat={seat} generation=1 role=executor class=worker audit={} seq={} final_role_verified=true stdout_bytes={} stderr_bytes={} (raw outputs discarded)",
            orient.event.id,
            orient.seq,
            out.len(),
            err.len()
        );
    }
    assert_eq!(
        configuration_digests(),
        before,
        "named user config files changed during interop"
    );
    println!(
        "named Claude/Codex settings/config/hooks hashes unchanged; no secret/config values exported"
    );
}

fn bounded_client(
    mut command: std::process::Command,
    timeout: Duration,
) -> (std::process::ExitStatus, Vec<u8>, Vec<u8>) {
    use std::os::unix::process::CommandExt;
    use std::process::Stdio;
    command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0);
    let mut child = command.spawn().expect("start installed client");
    let pid = child.id();
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    let out = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        stdout.take(1024 * 1024).read_to_end(&mut bytes).unwrap();
        bytes
    });
    let err = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        stderr.take(256 * 1024).read_to_end(&mut bytes).unwrap();
        bytes
    });
    let start = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if start.elapsed() > timeout {
            // SAFETY: this is the newly-created disposable client's own group.
            unsafe {
                libc::kill(-(pid as i32), libc::SIGKILL);
            }
            let status = child.wait().unwrap();
            let out = out.join().unwrap();
            let err = err.join().unwrap();
            panic!(
                "client timeout, status={status}, stdout_bytes={}, stderr_bytes={}; raw outputs discarded",
                out.len(),
                err.len()
            );
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    // End only descendants of the disposable process group before joining its
    // pipe readers; an MCP helper must not keep a completed fixture open.
    // SAFETY: this is the newly-created disposable client's own group.
    unsafe {
        libc::kill(-(pid as i32), libc::SIGKILL);
    }
    (status, out.join().unwrap(), err.join().unwrap())
}

fn configuration_digests() -> std::collections::BTreeMap<PathBuf, Option<String>> {
    let home = PathBuf::from(std::env::var_os("HOME").unwrap());
    [
        ".claude/settings.json",
        ".claude/settings.local.json",
        ".codex/config.toml",
        ".codex/hooks.json",
    ]
    .into_iter()
    .map(|path| {
        let path = home.join(path);
        let digest = match std::fs::read(&path) {
            Ok(bytes) => Some(bbt_mux::store::digest(&bytes)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => panic!("cannot hash named config: {error}"),
        };
        (path, digest)
    })
    .collect()
}

#[test]
fn mcp_messages_bind_the_sender_retry_once_reply_list_and_ack_through_w24() {
    let mut f = Fixture::new();
    let own = f.own.id;
    let peer = f.peer.id;
    let destination = format!("seats/{peer}/inbox");
    let op = Ulid::new();
    let args = json!({"destination":destination,"text":"private fixture payload","priority":9,"attachments":["[[fixture-artifact]]"]});
    let first = f.mutate(own, op, "message_send", args.clone());
    let sent = data(first.clone());
    assert_eq!(sent["status"], "accepted");
    assert_eq!(sent["kind"], "stored");
    let envelope = &sent["message"]["envelope"];
    assert_eq!(envelope["id"], op.to_string());
    assert_eq!(envelope["seat"], own.to_string());
    assert_eq!(envelope["role"]["class"], "worker");
    assert_eq!(envelope["priority_class"], "worker");
    assert_eq!(f.mutate(own, op, "message_send", args.clone()), first);
    let changed = f.mutate(
        own,
        op,
        "message_send",
        json!({"destination":destination,"text":"changed"}),
    );
    assert_eq!(
        changed["structuredContent"]["error"]["code"],
        "op_id_reused"
    );
    f.restart();
    assert_eq!(f.mutate(own, op, "message_send", args), first);
    let inbox = data(f.call(peer, Ulid::new(), "message_list", json!({})));
    assert_eq!(inbox["page"]["items"].as_array().unwrap().len(), 1);
    assert_eq!(inbox["page"]["items"][0]["envelope"]["id"], op.to_string());
    let receipts = data(f.call(own, Ulid::new(), "message_receipts", json!({"message":op})));
    assert_ne!(
        receipts["page"]["items"][0]["evidence"],
        json!(bbt_mux::adapters::Evidence::ApplicationAcknowledged)
    );
    assert_eq!(
        f.mutate(
            f.oracle.id,
            Ulid::new(),
            "message_ack",
            json!({"message":op})
        )["isError"],
        true
    );
    let acknowledged = data(f.mutate(peer, Ulid::new(), "message_ack", json!({"message":op})));
    assert_eq!(acknowledged["status"], "applied");
    let receipts = data(f.call(own, Ulid::new(), "message_receipts", json!({"message":op})));
    assert_eq!(
        receipts["page"]["items"][0]["evidence"],
        json!(bbt_mux::adapters::Evidence::ApplicationAcknowledged)
    );
    let reply = data(f.mutate(
        peer,
        Ulid::new(),
        "message_send",
        json!({"destination":format!("seats/{own}/inbox"),"text":"reply","reply_to":op}),
    ));
    assert_eq!(
        reply["message"]["envelope"]["thread_id"],
        envelope["thread_id"]
    );
    let thread = data(f.call(
        own,
        Ulid::new(),
        "message_list",
        json!({"thread":envelope["thread_id"],"limit":1}),
    ));
    assert_eq!(thread["page"]["items"].as_array().unwrap().len(), 1);
    let next = thread["page"]["next"].as_u64().unwrap();
    let second = data(f.call(
        own,
        Ulid::new(),
        "message_list",
        json!({"thread":envelope["thread_id"],"after":next,"limit":1}),
    ));
    assert_eq!(second["page"]["items"].as_array().unwrap().len(), 1);
    assert_eq!(
        second["page"]["items"][0]["envelope"]["body"]["text"],
        "reply"
    );
    let audits = f.audits(own, 7);
    let serialized = serde_json::to_string(&audits).unwrap();
    assert!(!serialized.contains("private fixture payload"));
    assert!(!serialized.contains("fixture-artifact"));
    assert!(
        audits
            .iter()
            .any(|event| event.event.payload["outcome"] == "admitted")
    );
    assert!(
        audits
            .iter()
            .any(|event| event.event.payload["outcome"] == "replay")
    );
}

#[test]
fn mcp_channel_tools_keep_worker_links_and_recheck_revoked_reads_before_cached_results() {
    let f = Fixture::new();
    let own = f.own.id;
    let oracle = f.oracle.id;
    let folder = "channels/fixture";
    data(f.mutate(
        oracle,
        Ulid::new(),
        "ch_link",
        json!({"node":{"path":folder,"kind":"folder"}}),
    ));
    data(f.mutate(
        oracle,
        Ulid::new(),
        "ch_grant",
        json!({"path":folder,"seat":own,"rights":["read","write","link"]}),
    ));
    let path = format!("{folder}/private-node-marker");
    data(f.mutate(own,Ulid::new(),"ch_subscribe",json!({"folder":folder,"name":"private-node-marker","filter":{"kinds":["chunk"]},"rate":{"batch_ms":100,"coalesce":true}})));
    let read_op = Ulid::new();
    let before = data(f.call(own, read_op, "ch_ls", json!({"path":folder})));
    let node = &before["nodes"][0];
    assert_eq!(node["path"], path);
    assert_eq!(node["target"], format!("seats/{own}/inbox"));
    assert_eq!(node["filter"]["kinds"], json!(["chunk"]));
    assert_eq!(node["filter"]["classes"], json!([]));
    assert_eq!(node["rate"]["batch_ms"], 100);
    data(f.mutate(
        oracle,
        Ulid::new(),
        "ch_pause",
        json!({"path":path,"paused":true}),
    ));
    assert_eq!(
        data(f.call(own, Ulid::new(), "ch_ls", json!({"path":folder})))["nodes"][0]["paused"],
        true
    );
    data(f.mutate(
        oracle,
        Ulid::new(),
        "ch_pause",
        json!({"path":path,"paused":false}),
    ));
    let worker = f
        .runtime
        .block_on(f.client().control_catalogue(own, 1))
        .unwrap();
    let admin = f
        .runtime
        .block_on(f.client().control_catalogue(oracle, 1))
        .unwrap();
    assert_eq!(worker["tools"].as_array().unwrap().len(), 20);
    assert_eq!(admin["tools"].as_array().unwrap().len(), 32);
    for tool in [
        "ch_link",
        "ch_pause",
        "ch_grant",
        "seat_input_text",
        "task_delegate",
        "seat_mute",
        "seat_unmute",
    ] {
        assert!(
            !worker["tools"]
                .as_array()
                .unwrap()
                .iter()
                .any(|item| item["name"] == tool)
        );
        assert_eq!(f.call(own, Ulid::new(), tool, json!({}))["isError"], true);
    }
    let others = format!("swarms/{}/{}", f.own.swarm, f.peer.id);
    assert_eq!(
        f.mutate(own, Ulid::new(), "ch_unsubscribe", json!({"path":others}))["isError"],
        true
    );
    let invalid = f.mutate(
        own,
        Ulid::new(),
        "ch_subscribe",
        json!({"folder":folder,"name":"../escape"}),
    );
    assert_eq!(invalid["isError"], true);
    let epoch =
        data(f.call(own, Ulid::new(), "self_status", json!({})))["actor"]["grant_epoch"].clone();
    data(f.mutate(
        oracle,
        Ulid::new(),
        "ch_grant",
        json!({"path":folder,"seat":own,"rights":[]}),
    ));
    assert_eq!(
        data(f.call(own, Ulid::new(), "self_status", json!({})))["actor"]["grant_epoch"],
        epoch
    );
    let denied = f.call(own, read_op, "ch_ls", json!({"path":folder}));
    assert_eq!(denied["isError"], true);
    assert!(
        !denied.to_string().contains("private-node-marker"),
        "cached authorized read escaped current grants"
    );
    assert_eq!(
        f.mutate(
            own,
            Ulid::new(),
            "message_send",
            json!({"destination":format!("{folder}/*"),"text":"revoked","kind":"chunk"})
        )["isError"],
        true
    );
    data(f.mutate(
        oracle,
        Ulid::new(),
        "ch_grant",
        json!({"path":folder,"seat":own,"rights":["read","write","link"]}),
    ));
    data(f.mutate(own, Ulid::new(), "ch_unsubscribe", json!({"path":path})));
    assert!(
        data(f.call(own, Ulid::new(), "ch_ls", json!({"path":folder})))["nodes"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn mcp_message_visibility_and_receipt_pages_preserve_current_w24_scope() {
    let f = Fixture::new();
    let own = f.own.id;
    let peer = f.peer.id;
    let oracle = f.oracle.id;
    let private = data(f.mutate(
        oracle,
        Ulid::new(),
        "message_send",
        json!({"destination":format!("seats/{peer}/inbox"),"text":"third party private payload"}),
    ));
    let private_id = private["message"]["envelope"]["id"].clone();
    assert!(
        data(f.call(own, Ulid::new(), "message_list", json!({})))["page"]["items"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        f.call(
            own,
            Ulid::new(),
            "message_receipts",
            json!({"message":private_id})
        )["isError"],
        true
    );
    assert_eq!(
        f.call(
            f.outside.id,
            Ulid::new(),
            "message_receipts",
            json!({"message":private_id})
        )["isError"],
        true
    );
    let admin_page = data(f.call(
        oracle,
        Ulid::new(),
        "message_list",
        json!({"channel":format!("seats/{peer}/inbox")}),
    ));
    assert_eq!(admin_page["page"]["items"].as_array().unwrap().len(), 1);
    let broadcast = data(f.mutate(
        own,
        Ulid::new(),
        "message_send",
        json!({"destination":format!("swarms/{}/*",f.own.swarm),"kind":"chunk","text":"chunk"}),
    ));
    let broadcast_id = broadcast["message"]["envelope"]["id"].clone();
    let recipient = data(f.call(
        peer,
        Ulid::new(),
        "message_receipts",
        json!({"message":broadcast_id}),
    ));
    assert_eq!(recipient["page"]["items"].as_array().unwrap().len(), 1);
    assert_eq!(recipient["page"]["items"][0]["recipient"], peer.to_string());
    let mut cursor = 0;
    let mut count = 0;
    loop {
        let page = data(f.call(
            own,
            Ulid::new(),
            "message_receipts",
            json!({"message":broadcast_id,"after":cursor,"limit":1}),
        ));
        count += page["page"]["items"].as_array().unwrap().len();
        if let Some(next) = page["page"]["next"].as_u64() {
            assert!(next > cursor);
            cursor = next;
        } else {
            break;
        }
    }
    assert_eq!(count, 3);
    assert_eq!(
        data(f.call(
            oracle,
            Ulid::new(),
            "message_receipts",
            json!({"message":broadcast_id})
        ))["page"]["items"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    data(f.mutate(
        oracle,
        Ulid::new(),
        "ch_grant",
        json!({"path":format!("seats/{peer}/inbox"),"seat":own,"rights":[]}),
    ));
    let denied = f.mutate(
        own,
        Ulid::new(),
        "message_send",
        json!({"destination":format!("seats/{peer}/inbox"),"text":"explicitly revoked DM"}),
    );
    assert_eq!(denied["isError"], true);
    assert!(denied.to_string().contains("write"));
    // Even a controller-created foreign sink in this tree cannot bypass the
    // registry's swarm fence at the W2.4 send boundary.
    let foreign_folder = format!("seats/{}", f.outside.id);
    data(f.mutate(
        oracle,
        Ulid::new(),
        "ch_link",
        json!({"node":{"path":foreign_folder,"kind":"folder"}}),
    ));
    let foreign_sink = format!("{foreign_folder}/inbox");
    data(f.mutate(
        oracle,
        Ulid::new(),
        "ch_link",
        json!({"node":{"path":foreign_sink,"kind":"sink"}}),
    ));
    let foreign = f.mutate(
        own,
        Ulid::new(),
        "message_send",
        json!({"destination":foreign_sink,"text":"no cross-swarm delivery"}),
    );
    assert_eq!(foreign["isError"], true);
    assert!(foreign.to_string().contains("swarm"));
    assert!(
        data(f.call(f.outside.id, Ulid::new(), "message_list", json!({})))["page"]["items"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        f.mutate(
            oracle,
            Ulid::new(),
            "ch_grant",
            json!({"path":format!("swarms/{}",f.own.swarm),"seat":f.outside.id,"rights":["write"]})
        )["isError"],
        true
    );
}

#[test]
fn mcp_mutation_argument_ids_are_visible_retryable_and_cannot_spoof_authority() {
    let f = Fixture::new();
    let own = f.own.id;
    let mut bridge = f.bridge(own);
    initialize(&mut bridge);
    let args = json!({"destination":format!("seats/{}/inbox",f.peer.id),"text":"fixture"});
    let minted = rpc(
        &mut bridge,
        2,
        "tools/call",
        json!({"name":"message_send","arguments":args}),
    );
    assert_eq!(minted["result"]["isError"], false);
    let op = minted["result"]["structuredContent"]["op_id"]
        .as_str()
        .unwrap()
        .parse::<Ulid>()
        .unwrap();
    assert_ne!(op, Ulid::nil());
    for invalid_id in [json!("not-a-ulid"), json!(Ulid::nil()), json!("#1")] {
        let mut invalid = args.clone();
        invalid["op_id"] = invalid_id;
        assert_eq!(
            rpc(
                &mut bridge,
                2,
                "tools/call",
                json!({"name":"message_send","arguments":invalid})
            )["error"]["code"],
            -32602
        );
    }
    let mut original = args.clone();
    original["op_id"] = json!(op);
    let first = rpc(
        &mut bridge,
        3,
        "tools/call",
        json!({"name":"message_send","arguments":original}),
    );
    assert_eq!(first["result"]["isError"], false);
    assert_eq!(first["result"], minted["result"]);
    let compatible = rpc(
        &mut bridge,
        4,
        "tools/call",
        json!({"name":"message_send","arguments":args,"_meta":{"bboxy450/op_id":op}}),
    );
    assert_eq!(compatible["result"], first["result"]);
    let reordered = rpc(
        &mut bridge,
        4,
        "tools/call",
        json!({"name":"message_send","arguments":{
            "text":"fixture","op_id":op,"destination":args["destination"]
        }}),
    );
    assert_eq!(reordered["result"], first["result"]);
    let changed = rpc(
        &mut bridge,
        4,
        "tools/call",
        json!({"name":"message_send","arguments":{
            "op_id":op,"destination":args["destination"],"text":"different"
        }}),
    );
    assert_eq!(changed["result"]["isError"], true);
    assert_eq!(
        changed["result"]["structuredContent"]["error"]["code"],
        "op_id_reused"
    );
    let mismatch = rpc(
        &mut bridge,
        5,
        "tools/call",
        json!({"name":"message_send","arguments":original,"_meta":{"bboxy450/op_id":Ulid::new()}}),
    );
    assert_eq!(mismatch["error"]["code"], -32602);
    assert!(
        mismatch["error"]["message"]
            .as_str()
            .unwrap()
            .contains("mismatch")
    );
    for (field, value) in [
        ("sender", json!(f.oracle.id)),
        ("role", json!("oracle")),
        ("operator", json!(true)),
        ("swarm_id", json!(f.outside.swarm)),
        ("process_generation", json!(2)),
    ] {
        let mut forged = args.clone();
        forged[field] = value;
        assert_eq!(
            f.mutate(own, Ulid::new(), "message_send", forged)["isError"],
            true
        );
    }
    for extra in [
        json!({"kind":"task"}),
        json!({"kind":"presence"}),
        json!({"text":"x".repeat(16385)}),
        json!({"text":"\u{1b}[escape"}),
    ] {
        let mut invalid = args.clone();
        invalid
            .as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        assert_eq!(
            f.mutate(own, Ulid::new(), "message_send", invalid)["isError"],
            true
        );
    }
    assert_eq!(
        f.mutate(
            own,
            Ulid::new(),
            "message_send",
            json!({"destination":format!("swarms/{}/*",f.own.swarm),"text":"plain progress"})
        )["isError"],
        true
    );
    for (tool, args) in [
        ("message_list", json!({"limit":129})),
        ("message_receipts", json!({"message":op,"limit":0})),
        ("ch_ls", json!({"path":"../escape"})),
    ] {
        assert_eq!(f.call(own, Ulid::new(), tool, args)["isError"], true);
    }
    assert_eq!(
        data(f.call(own, Ulid::new(), "message_list", json!({})))["page"]["items"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn mcp_minted_read_ids_are_retryable_and_every_catalogue_explains_optional_ids() {
    let f = Fixture::new();
    for seat in [f.own.id, f.oracle.id] {
        let mut bridge = f.bridge(seat);
        initialize(&mut bridge);
        let catalogue = rpc(&mut bridge, 2, "tools/list", json!({}));
        for tool in catalogue["result"]["tools"].as_array().unwrap() {
            let schema = &tool["inputSchema"];
            assert!(
                schema["properties"]["op_id"]["description"]
                    .as_str()
                    .unwrap()
                    .contains("retry")
            );
            assert!(
                schema["required"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(|field| field != "op_id")
            );
        }
        let first = rpc(
            &mut bridge,
            3,
            "tools/call",
            json!({"name":"self_status","arguments":{}}),
        );
        assert_eq!(first["result"]["isError"], false);
        let id = first["result"]["structuredContent"]["op_id"]
            .as_str()
            .unwrap()
            .parse::<Ulid>()
            .unwrap();
        assert_ne!(id, Ulid::nil());
        let retry = rpc(
            &mut bridge,
            4,
            "tools/call",
            json!({"name":"self_status","arguments":{"op_id":id}}),
        );
        assert_eq!(retry["result"], first["result"]);
        let different_tool = rpc(
            &mut bridge,
            5,
            "tools/call",
            json!({"name":"seat_list","arguments":{"op_id":id}}),
        );
        assert_eq!(different_tool["result"]["isError"], true);
        assert_eq!(
            different_tool["result"]["structuredContent"]["error"]["code"],
            "op_id_reused"
        );
    }
}

#[test]
fn mcp_list_and_graph_pages_explain_authenticated_scope_and_empty_views() {
    let f = Fixture::new();
    let mut bridge = f.bridge(f.peer.id);
    initialize(&mut bridge);
    let expected = json!({
        "seat":f.peer.id,"role":f.peer.role.name,"class":f.peer.role.class
    });
    for (tool, args, reason) in [
        ("task_list", json!({}), "nothing_assigned"),
        ("graph_read", json!({"graph":"task"}), "nothing_assigned"),
        (
            "graph_read",
            json!({"graph":"markdown"}),
            "not_visible_to_role",
        ),
        ("message_list", json!({}), "nothing_yet"),
        (
            "seat_list",
            json!({"after":Ulid::from(u128::MAX)}),
            "nothing_yet",
        ),
    ] {
        let response = rpc(
            &mut bridge,
            2,
            "tools/call",
            json!({"name":tool,"arguments":args}),
        );
        assert_eq!(response["result"]["isError"], false, "{tool}: {response}");
        let value = &response["result"]["structuredContent"];
        assert_eq!(value["scope"], expected, "{tool}");
        assert_eq!(value["empty_reason"], reason, "{tool}");
        let line = value["empty_message"].as_str().unwrap();
        assert!(!line.is_empty() && !line.contains('\n'));
    }
    for (tool, args, collection) in [
        ("seat_list", json!({"limit":1}), "items"),
        (
            "ch_ls",
            json!({"path":format!("seats/{}/inbox",f.peer.id)}),
            "nodes",
        ),
        (
            "screen_read",
            json!({"seat_id":f.peer.id,"expected_process_generation":"1","limit_rows":1}),
            "rows",
        ),
        (
            "history_read",
            json!({"seat_id":f.peer.id,"expected_process_generation":"1","limit_records":1}),
            "records",
        ),
    ] {
        let response = rpc(
            &mut bridge,
            3,
            "tools/call",
            json!({"name":tool,"arguments":args}),
        );
        assert_eq!(response["result"]["isError"], false, "{tool}: {response}");
        let value = &response["result"]["structuredContent"];
        assert_eq!(value["scope"], expected, "{tool}");
        if value[collection].as_array().unwrap().is_empty() {
            assert_eq!(value["empty_reason"], "nothing_yet");
        } else {
            assert!(value.get("empty_reason").is_none());
        }
    }
    let denied = rpc(
        &mut bridge,
        4,
        "tools/call",
        json!({"name":"graph_read","arguments":{"graph":"execution"}}),
    );
    assert_eq!(denied["result"]["isError"], true);
    let mut oracle = f.bridge(f.oracle.id);
    initialize(&mut oracle);
    let folder = rpc(
        &mut oracle,
        5,
        "tools/call",
        json!({"name":"ch_ls","arguments":{"path":"channels"}}),
    );
    assert_eq!(folder["result"]["isError"], false);
    let folder = &folder["result"]["structuredContent"];
    assert_eq!(folder["nodes"], json!([]));
    assert_eq!(
        folder["scope"],
        json!({"seat":f.oracle.id,"role":"oracle","class":Class::Oracle})
    );
    assert_eq!(folder["empty_reason"], "nothing_yet");
}

#[test]
fn mcp_counts_have_a_basis_and_ack_ids_without_equating_acknowledgement_with_reading() {
    let f = Fixture::new();
    let mut sender = f.bridge(f.peer.id);
    let mut receiver = f.bridge(f.own.id);
    initialize(&mut sender);
    initialize(&mut receiver);
    let mut ids = Vec::new();
    for text in ["first", "second"] {
        let response = rpc(
            &mut sender,
            2,
            "tools/call",
            json!({"name":"message_send","arguments":{
                "destination":format!("seats/{}/inbox",f.own.id),"text":text
            }}),
        );
        assert_eq!(response["result"]["isError"], false);
        ids.push(response["result"]["structuredContent"]["message"]["envelope"]["id"].clone());
    }
    let first = rpc(
        &mut receiver,
        3,
        "tools/call",
        json!({"name":"message_list","arguments":{}}),
    );
    assert_eq!(first["result"]["isError"], false);
    let first = &first["result"]["structuredContent"];
    assert_eq!(first["unread"], json!({"count":2,"basis":"no_cursor_yet"}));
    assert_eq!(first["pending_ack"]["count"], 2);
    assert_eq!(first["pending_ack"]["ids"], json!(ids));
    assert_eq!(first["pending_ack"]["ids_truncated"], false);
    assert_eq!(
        first["pending_ack"]["action"],
        "call message_ack for each id listed"
    );
    let ack = rpc(
        &mut receiver,
        4,
        "tools/call",
        json!({"name":"message_ack","arguments":{"message":ids[0]}}),
    );
    assert_eq!(ack["result"]["isError"], false);
    let after = rpc(
        &mut receiver,
        5,
        "tools/call",
        json!({"name":"message_list","arguments":{}}),
    );
    let after = &after["result"]["structuredContent"];
    assert_eq!(after["unread"], json!({"count":2,"basis":"no_cursor_yet"}));
    assert_eq!(after["pending_ack"]["count"], 1);
    assert_eq!(after["pending_ack"]["ids"], json!([ids[1]]));
    let outbox = rpc(
        &mut sender,
        6,
        "tools/call",
        json!({"name":"message_list","arguments":{}}),
    );
    let outbox = &outbox["result"]["structuredContent"];
    assert_eq!(outbox["unread"], json!({"count":0,"basis":"no_cursor_yet"}));
    assert_eq!(outbox["pending_ack"]["count"], 0);
    let receipts = rpc(
        &mut sender,
        7,
        "tools/call",
        json!({"name":"message_receipts","arguments":{"message":ids[1]}}),
    );
    let receipts = &receipts["result"]["structuredContent"];
    assert_eq!(receipts["scope"]["seat"], json!(f.peer.id));
    assert!(receipts.get("empty_reason").is_none());
}

// Work-item fixtures deliberately use local zsh PTYs and fixture MCP clients.
// They prove the protocol/terminal path, not a real model's first-turn behavior.
impl Fixture {
    fn task_file(&self, metadata: Value) -> bbt_mux::index::Document {
        let path = self
            .config
            .repository
            .join(format!(".bboxy/tasks/{}.md", self.task));
        std::fs::write(
            &path,
            format!(
                "---\n{}---\nOriginal brief: keep this body.\n",
                serde_yaml::to_string(&metadata).unwrap()
            ),
        )
        .unwrap();
        self.task_document()
    }
    fn task_document(&self) -> bbt_mux::index::Document {
        let path = format!("tasks/{}.md", self.task);
        bbt_mux::index::Document::parse(
            &path,
            &std::fs::read_to_string(self.config.repository.join(".bboxy").join(&path)).unwrap(),
        )
    }
    fn task_metadata(&self) -> Value {
        json!({"id":self.task,"type":"task","number":1,"title":"Fixture: quotes ' and colon: retained","status":"running",
            "assignee":format!("seat:{}",self.own.id),"delegated_by":format!("seat:{}",self.oracle.id),
            "verify":"true","allowed_paths":["fixture/**"],"custom":{"nested":["untouched",42]}})
    }
    fn task_path(&self) -> String {
        format!(".bboxy/tasks/{}.md", self.task)
    }
    fn report(&self, result: Value) -> Value {
        self.mutate(self.own.id,Ulid::new(),"task_report",json!({"task_path":self.task_path(),"summary":"Fixture report: verified","verify_result":result}))
    }
}
fn verification(command: &str, exit: i32, evidence: Value) -> Value {
    json!({"command":command,"exit_code":exit,"evidence":evidence})
}
fn assert_item_signature(doc: &bbt_mux::index::Document, seat: Ulid) {
    use base64::Engine;
    use bbt_mux::messaging::keys::{LocalSeatKeys, SeatKeys};
    use ed25519_dalek::{Signature, Verifier};
    let mut front = doc.metadata.clone();
    let provenance = front.as_object_mut().unwrap().remove("provenance").unwrap();
    let front_hash = bbt_mux::store::digest(serde_yaml::to_string(&front).unwrap().as_bytes());
    let body_hash = bbt_mux::store::digest(doc.body.as_bytes());
    assert_eq!(
        provenance["frontmatter_digest"],
        format!("sha256:{front_hash}")
    );
    assert_eq!(provenance["digest"], format!("sha256:{body_hash}"));
    assert_eq!(provenance["signer"], format!("seat:{seat}"));
    let mut signed = b"DSSEv1 13 bboxy/item/v1 128 ".to_vec();
    signed.extend(format!("{front_hash}{body_hash}").as_bytes());
    let signature = Signature::try_from(
        base64::engine::general_purpose::STANDARD
            .decode(provenance["sig"].as_str().unwrap())
            .unwrap()
            .as_slice(),
    )
    .unwrap();
    LocalSeatKeys::new(key())
        .leaf(seat, 1)
        .unwrap()
        .verifying_key()
        .verify(&signed, &signature)
        .unwrap();
}

#[test]
fn mcp_fixture_first_turn_orients_reports_and_retains_signed_task_receipts() {
    for harness in ["fixture-Claude", "fixture-Codex"] {
        let mut f = Fixture::new();
        let mut metadata = f.task_metadata();
        metadata["assignee"] = json!("none");
        metadata["status"] = json!("open");
        let original = f.task_file(metadata);
        let delegate = data(f.mutate(
            f.oracle.id,
            Ulid::new(),
            "task_delegate",
            json!({"task_path":f.task_path(),"seat_id":f.own.id,"role":"executor"}),
        ));
        assert_eq!(delegate["task_status"], "claimed");
        let mut bridge = f.bridge(f.own.id);
        assert!(rpc(&mut bridge,1,"initialize",json!({"protocolVersion":"2025-11-25","clientInfo":{"name":harness,"version":"1"},"capabilities":{}})).get("error").is_none());
        let orient = rpc(
            &mut bridge,
            2,
            "tools/call",
            json!({"name":"orient","arguments":{}}),
        );
        assert!(
            orient["result"]["structuredContent"]["tasks"]
                .to_string()
                .contains(&f.task.to_string())
        );
        let op = Ulid::new();
        let args = json!({"op_id":op,"task_path":f.task_path(),"summary":format!("{harness}: fixture report"),"commit":"0123456789012345678901234567890123456789","verify_result":verification("true",0,json!([]))});
        let first = rpc(
            &mut bridge,
            3,
            "tools/call",
            json!({"name":"task_report","arguments":args}),
        );
        assert_eq!(data(first["result"].clone())["task_status"], "reported");
        let source = f.task_document();
        assert!(source.body.starts_with(&original.body));
        assert_eq!(source.metadata["custom"], original.metadata["custom"]);
        assert_eq!(source.metadata["updated_by"], format!("seat:{}", f.own.id));
        assert_eq!(source.metadata["verify_result"]["exit_code"], 0);
        assert_item_signature(&source, f.own.id);
        assert_eq!(
            rpc(
                &mut bridge,
                4,
                "tools/call",
                json!({"name":"task_report","arguments":args})
            )["result"],
            first["result"]
        );
        let message: Ulid = first["result"]["structuredContent"]["message_ids"][0]
            .as_str()
            .unwrap()
            .parse()
            .unwrap();
        let inbox = data(f.call(f.oracle.id, Ulid::new(), "message_list", json!({})));
        let notice = inbox["page"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|m| m["envelope"]["id"] == message.to_string())
            .unwrap();
        assert_eq!(notice["envelope"]["kind"], "task");
        assert_eq!(notice["envelope"]["body"]["event"], "report");
        assert_eq!(notice["envelope"]["body"]["task_id"], f.task.to_string());
        assert_eq!(notice["envelope"]["priority_class"], "worker");
        let receipts = data(f.call(
            f.own.id,
            Ulid::new(),
            "message_receipts",
            json!({"message":message}),
        ));
        assert!(!receipts["page"]["items"].as_array().unwrap().is_empty());
        data(f.mutate(
            f.oracle.id,
            Ulid::new(),
            "message_ack",
            json!({"message":message}),
        ));
        let receipts = data(f.call(
            f.own.id,
            Ulid::new(),
            "message_receipts",
            json!({"message":message}),
        ));
        assert_eq!(
            receipts["page"]["items"][0]["evidence"],
            json!(bbt_mux::adapters::Evidence::ApplicationAcknowledged)
        );
        let audits = f.audits(f.own.id, 2);
        assert_eq!(audits[0].event.payload["tool"], "orient");
        assert_eq!(audits[1].event.payload["tool"], "task_report");
        f.restart();
        assert_eq!(
            f.mutate(f.own.id, op, "task_report", {
                let mut a = args.clone();
                a.as_object_mut().unwrap().remove("op_id");
                a
            }),
            first["result"]
        );
        assert_eq!(f.task_document().digest, source.digest);
        let changed = f.mutate(
            f.own.id,
            op,
            "task_report",
            json!({"task_path":f.task_path(),"summary":"different"}),
        );
        assert_eq!(
            changed["structuredContent"]["error"]["code"],
            "op_id_reused"
        );
    }
}

#[test]
fn mcp_task_accept_requires_matching_zero_verification_and_declared_criteria_only() {
    let f = Fixture::new();
    let mut metadata = f.task_metadata();
    metadata["acceptance"] = json!(["criterion 1",{"criterion":"specs/fixture.md:7","spec":"specs/fixture.md","line":7}]);
    f.task_file(metadata);
    let accept = || {
        f.mutate(
            f.oracle.id,
            Ulid::new(),
            "task_accept",
            json!({"task_path":f.task_path()}),
        )
    };
    assert_eq!(accept()["isError"], true);
    data(f.report(verification("true", 9, json!([]))));
    assert!(accept().to_string().contains("exit_code"));
    data(f.report(verification("different command", 0, json!([]))));
    assert!(accept().to_string().contains("does not match"));
    data(f.report(verification(
        "true",
        0,
        json!([{"criterion":"criterion 1","reference":"fixture.log:1 — exit 0"}]),
    )));
    assert!(accept().to_string().contains("specs/fixture.md:7"));
    assert_eq!(f.task_document().metadata["status"], "reported");
    data(f.report(verification(
        "true",
        0,
        json!([
            {"criterion":"criterion 1","reference":"fixture.rs:1"},
            {"criterion":"specs/fixture.md:7","reference":"fixture.log:2"}
        ]),
    )));
    assert_eq!(
        f.mutate(
            f.own.id,
            Ulid::new(),
            "task_accept",
            json!({"task_path":f.task_path()})
        )["isError"],
        true
    );
    assert_eq!(data(accept())["task_status"], "accepted");
    assert_eq!(f.task_document().metadata["status"], "accepted");
    // Legacy tasks without a criteria list still need the exact command and zero.
    f.task_file(f.task_metadata());
    data(f.report(verification("true", 0, json!([]))));
    assert_eq!(data(accept())["task_status"], "accepted");
    f.task_file(f.task_metadata());
    data(f.report(verification("true", 0, json!([]))));
    data(f.mutate(
        f.oracle.id,
        Ulid::new(),
        "task_reject",
        json!({"task_path":f.task_path(),"reason":"fixture evidence incomplete"}),
    ));
    assert_eq!(
        f.task_document().metadata["rejection"]["reason"],
        "fixture evidence incomplete"
    );
    let mut args = json!({"task_path":f.task_path(),"summary":"missing result"});
    let op = Ulid::new();
    args["op_id"] = json!(op);
    let mut bridge = f.bridge(f.own.id);
    initialize(&mut bridge);
    data(
        rpc(
            &mut bridge,
            2,
            "tools/call",
            json!({"name":"task_report","arguments":args}),
        )["result"]
            .clone(),
    );
    assert!(f.task_document().metadata.get("verify_result").is_none());
    assert!(accept().to_string().contains("recorded verify_result"));
}

#[test]
fn mcp_task_reassignment_reauthorizes_history_and_refuses_foreign_or_ambiguous_scope() {
    let f = Fixture::new();
    f.task_file(f.task_metadata());
    let old_read = Ulid::new();
    let old_list = Ulid::new();
    let old_orient = Ulid::new();
    let old_graph = Ulid::new();
    data(f.call(f.own.id, old_list, "task_list", json!({})));
    data(f.call(f.own.id, old_orient, "orient", json!({})));
    data(f.call(f.own.id, old_graph, "graph_read", json!({"graph":"task"})));
    data(f.call(f.own.id, old_read, "task_status", json!({"task_id":f.task})));
    let changed = data(f.mutate(
        f.oracle.id,
        Ulid::new(),
        "task_reassign",
        json!({"task_path":f.task_path(),"seat_id":f.peer.id}),
    ));
    assert_eq!(changed["message_ids"].as_array().unwrap().len(), 2);
    assert_eq!(
        f.task_document().metadata["assignee"],
        format!("seat:{}", f.peer.id)
    );
    assert_eq!(
        f.call(f.own.id, old_read, "task_status", json!({"task_id":f.task}))["isError"],
        true
    );
    for (op, tool, args) in [
        (old_list, "task_list", json!({})),
        (old_orient, "orient", json!({})),
        (old_graph, "graph_read", json!({"graph":"task"})),
    ] {
        assert_eq!(
            f.call(f.own.id, op, tool, args)["isError"],
            true,
            "retained {tool} must not disclose a released task"
        );
    }
    assert_eq!(
        f.report(verification("true", 0, json!([])))["isError"],
        true
    );
    let orient = data(f.call(f.own.id, Ulid::new(), "orient", json!({})));
    assert_eq!(orient["tasks"]["items"].as_array().unwrap().len(), 0);
    data(f.call(
        f.peer.id,
        Ulid::new(),
        "task_status",
        json!({"task_path":f.task_path()}),
    ));
    let tasks = data(f.call(f.peer.id, Ulid::new(), "task_list", json!({})));
    assert_eq!(tasks["items"].as_array().unwrap().len(), 1);
    assert_eq!(
        f.call(
            f.peer.id,
            Ulid::new(),
            "task_list",
            json!({"assignee":f.own.id})
        )["isError"],
        true
    );
    for id in [f.own.id, f.peer.id] {
        let inbox = data(f.call(id, Ulid::new(), "message_list", json!({})));
        assert!(
            inbox["page"]["items"]
                .as_array()
                .unwrap()
                .iter()
                .any(|m| m["envelope"]["kind"] == "task")
        );
    }
    for (assignee, delegator) in [
        (
            json!(format!("seat:{}", f.outside.id)),
            json!(format!("seat:{}", f.oracle.id)),
        ),
        (json!("none"), Value::Null),
    ] {
        let mut meta = f.task_metadata();
        meta["assignee"] = assignee;
        meta["delegated_by"] = delegator;
        f.task_file(meta);
        assert_eq!(
            f.call(
                f.oracle.id,
                Ulid::new(),
                "task_status",
                json!({"task_path":f.task_path()})
            )["isError"],
            true
        );
    }
    let mut meta = f.task_metadata();
    meta["assignee"] = json!("none");
    f.task_file(meta);
    data(f.mutate(
        f.oracle.id,
        Ulid::new(),
        "task_delegate",
        json!({"task_path":f.task_path(),"seat_id":f.own.id}),
    ));
    let before = f.task_document().digest;
    for (tool, args) in [
        (
            "task_report",
            json!({"task_path":f.task_path(),"summary":"spoof","caller":f.oracle.id}),
        ),
        (
            "task_report",
            json!({"task_path":"../escape.md","summary":"escape"}),
        ),
        (
            "task_delegate",
            json!({"task_path":f.task_path(),"seat_id":f.outside.id}),
        ),
        (
            "task_delegate",
            json!({"task_path":f.task_path(),"seat_id":f.own.id,"role":"oracle"}),
        ),
    ] {
        assert_eq!(
            f.mutate(
                if tool == "task_report" {
                    f.own.id
                } else {
                    f.oracle.id
                },
                Ulid::new(),
                tool,
                args
            )["isError"],
            true
        );
    }
    assert_eq!(f.task_document().digest, before);
    assert!(
        f.runtime
            .block_on(f.client().control_call(
                f.own.id,
                2,
                Call {
                    op_id: Ulid::new(),
                    tool: "task_report".into(),
                    args: json!({"task_path":f.task_path(),"summary":"stale"})
                }
            ))
            .is_err()
    );
}

#[test]
fn mcp_artifact_claim_is_signed_linked_idempotent_and_never_reads_external_content() {
    let mut f = Fixture::new();
    let original = f.task_file(f.task_metadata());
    let op = Ulid::new();
    let args = json!({"kind":"pr","ref":"https://invalid.example/review/123?literal=':","digest":format!("sha256:{}","a".repeat(64)),"task":f.task_path()});
    let first = f.mutate(f.own.id, op, "artifact_claim", args.clone());
    let claimed = data(first.clone());
    assert_eq!(claimed["artifact_id"], op.to_string());
    let path = f
        .config
        .repository
        .join(format!(".bboxy/artifacts/{op}.md"));
    let source = std::fs::read_to_string(&path).unwrap();
    let artifact = bbt_mux::index::Document::parse(&format!("artifacts/{op}.md"), &source);
    assert_eq!(artifact.metadata["number"], 3);
    assert_eq!(
        artifact.metadata["produced_by"],
        format!("seat:{}", f.own.id)
    );
    assert_eq!(artifact.metadata["for"], json!([format!("[[{}]]", f.task)]));
    assert_item_signature(&artifact, f.own.id);
    let task = f.task_document();
    assert_eq!(task.body, original.body);
    assert_eq!(task.metadata["custom"], original.metadata["custom"]);
    assert_eq!(task.metadata["artifacts"], json!([format!("[[{op}]]")]));
    assert_item_signature(&task, f.own.id);
    assert_eq!(
        f.mutate(f.own.id, op, "artifact_claim", args.clone()),
        first
    );
    f.restart();
    assert_eq!(
        f.mutate(f.own.id, op, "artifact_claim", args.clone()),
        first
    );
    assert_eq!(std::fs::read_to_string(&path).unwrap(), source);
    assert_eq!(f.task_document().digest, task.digest);
    assert_eq!(
        f.mutate(
            f.own.id,
            op,
            "artifact_claim",
            json!({"kind":"file","ref":"changed"})
        )["structuredContent"]["error"]["code"],
        "op_id_reused"
    );
    assert_eq!(
        f.mutate(
            f.own.id,
            Ulid::new(),
            "artifact_claim",
            json!({"kind":"file","ref":"fixture","produced_by":f.oracle.id})
        )["isError"],
        true
    );
    let index_log = EventLog::open(&f.config.run_dir.join("index")).unwrap();
    assert!(index_log.verify_chain().unwrap() >= 4);
}

#[test]
fn mcp_task_partial_wake_stays_unknown_after_restart_and_never_rewrites_the_file() {
    let mut f = Fixture::new();
    f.task_file(f.task_metadata());
    data(f.mutate(
        f.oracle.id,
        Ulid::new(),
        "ch_grant",
        json!({"path":format!("seats/{}/inbox",f.oracle.id),"seat":f.own.id,"rights":[]}),
    ));
    let op = Ulid::new();
    let args = json!({"task_path":f.task_path(),"summary":"written but wake denied","verify_result":verification("true",0,json!([]))});
    let unknown = f.mutate(f.own.id, op, "task_report", args.clone());
    assert_eq!(unknown["structuredContent"]["status"], "unknown");
    assert!(unknown.to_string().contains("frontmatter written"));
    let written = f.task_document();
    assert_eq!(written.metadata["status"], "reported");
    data(f.mutate(
        f.oracle.id,
        Ulid::new(),
        "ch_grant",
        json!({"path":format!("seats/{}/inbox",f.oracle.id),"seat":f.own.id,"rights":["write"]}),
    ));
    f.restart();
    assert_eq!(f.mutate(f.own.id, op, "task_report", args), unknown);
    assert_eq!(f.task_document().digest, written.digest);
    assert!(
        data(f.call(f.oracle.id, Ulid::new(), "message_list", json!({})))["page"]["items"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[test]
#[ignore = "acceptance driver supplies the prebuilt bbt-doc corpus test binary"]
fn mcp_work_item_writer_outputs_pass_the_actual_bbt_doc_corpus_check() {
    let f = Fixture::new();
    f.task_file(f.task_metadata());
    data(f.report(verification("true", 0, json!([]))));
    data(f.mutate(
        f.own.id,
        Ulid::new(),
        "artifact_claim",
        json!({"kind":"file","ref":"fixture: artifact","task":f.task_path()}),
    ));
    let binary =
        std::env::var_os("BBOXY_DOC_CORPUS_TEST_BINARY").expect("prebuild the bbt-doc corpus test");
    let output = std::process::Command::new(binary)
        .args(["--exact", "real_bboxy_frontmatter_parses", "--nocapture"])
        .env("BBOXY_DOC_CORPUS_ROOT", &f.config.repository)
        .output()
        .unwrap();
    println!(
        "writer corpus child exit {:?}\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("positive unique numbers, 0 errors"));
}

#[test]
fn mcp_artifact_partial_write_remains_unknown_and_is_not_reexecuted() {
    let mut f = Fixture::new();
    let original = f.task_file(f.task_metadata());
    let db = rusqlite::Connection::open(f.config.run_dir.join("index/events.sqlite")).unwrap();
    db.execute_batch(&format!("CREATE TRIGGER fixture_task_write_failure BEFORE INSERT ON events WHEN NEW.subject='doc/{}/write_prepared' BEGIN SELECT RAISE(ABORT,'fixture task write failure'); END;",f.task)).unwrap();
    let op = Ulid::new();
    let args = json!({"kind":"file","ref":"fixture artifact","task":f.task_path()});
    let unknown = f.mutate(f.own.id, op, "artifact_claim", args.clone());
    assert_eq!(unknown["structuredContent"]["status"], "unknown");
    let artifact_path = f
        .config
        .repository
        .join(format!(".bboxy/artifacts/{op}.md"));
    let written = std::fs::read_to_string(&artifact_path).unwrap();
    assert_eq!(
        bbt_mux::index::Document::parse("artifact.md", &written).metadata["number"],
        3
    );
    assert_eq!(f.task_document().digest, original.digest);
    db.execute_batch("DROP TRIGGER fixture_task_write_failure")
        .unwrap();
    drop(db);
    f.restart();
    assert_eq!(f.mutate(f.own.id, op, "artifact_claim", args), unknown);
    assert_eq!(std::fs::read_to_string(artifact_path).unwrap(), written);
    assert_eq!(f.task_document().digest, original.digest);
}

#[test]
fn mcp_work_item_audit_admission_failure_executes_no_frontmatter_write() {
    let f = Fixture::new();
    let original = f.task_file(f.task_metadata());
    // Open the actual per-seat journal through a read before installing its
    // controlled fault. Its failed admission transaction must roll back.
    data(f.call(
        f.own.id,
        Ulid::new(),
        "task_status",
        json!({"task_path":f.task_path()}),
    ));
    let db = rusqlite::Connection::open(
        f.config
            .run_dir
            .join(format!("seat-{}/object.sqlite", f.own.id)),
    )
    .unwrap();
    db.execute_batch("CREATE TRIGGER fixture_work_audit_failure BEFORE INSERT ON kv WHEN substr(NEW.key,1,14)='control/audit/' BEGIN SELECT RAISE(ABORT,'fixture audit failure'); END;").unwrap();
    let result = f.runtime.block_on(f.client().control_call(
        f.own.id,
        1,
        Call {
            op_id: Ulid::new(),
            tool: "task_report".into(),
            args: json!({"task_path":f.task_path(),"summary":"must not write"}),
        },
    ));
    assert!(result.is_err());
    assert_eq!(f.task_document().digest, original.digest);
    db.execute_batch("DROP TRIGGER fixture_work_audit_failure")
        .unwrap();
}

#[test]
fn mcp_work_item_signing_hold_keeps_the_original_file_and_terminals() {
    struct Denied;
    impl bbt_mux::identity::DeviceKeyStore for Denied {
        fn custody(&self) -> bbt_mux::identity::Custody {
            bbt_mux::identity::Custody::Memory
        }
        fn load(&self, _: Ulid) -> bbt_mux::identity::Result<Option<SigningKey>> {
            Err(bbt_mux::identity::PendingReason::KeychainInteractionRequired)
        }
        fn create(&self, _: Ulid, _: &SigningKey) -> bbt_mux::identity::Result<()> {
            Err(bbt_mux::identity::PendingReason::KeychainInteractionRequired)
        }
    }
    let mut f = Fixture::new();
    let original = f.task_file(f.task_metadata());
    f.logic.take().unwrap().shutdown().unwrap();
    f.logic = Some(
        Logic::start_custodied_with_store(
            f.supervisor.as_ref().unwrap().connect().unwrap(),
            f.config.clone(),
            std::sync::Arc::new(Denied),
            f.runtime.handle(),
        )
        .unwrap(),
    );
    let held = f.report(verification("true", 0, json!([])));
    assert_eq!(held["structuredContent"]["error"]["code"], "held");
    assert!(held.to_string().contains("signing held"));
    assert_eq!(f.task_document().digest, original.digest);
    assert_eq!(f.runtime.block_on(f.client().gui_seats()).unwrap().len(), 4);
}

fn isolated_seat_fixture(mode: &str) {
    let temp = tempfile::tempdir().unwrap();
    let stdout = temp.path().join("stdout");
    let stderr = temp.path().join("stderr");
    let mut child = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "mcp_seat_isolated_process",
            "--ignored",
            "--nocapture",
        ])
        .env_clear()
        .env("HOME", temp.path())
        .env("PATH", "/usr/bin:/bin")
        .env("CLAUDE_CONFIG_DIR", temp.path().join("claude"))
        .env("CODEX_HOME", temp.path().join("codex"))
        .env("BBOXY_TEST_CUSTODY", "memory")
        .env("BBOXY_MCP_SEAT_FIXTURE", mode)
        .stdout(std::fs::File::create(&stdout).unwrap())
        .stderr(std::fs::File::create(&stderr).unwrap())
        .spawn()
        .unwrap();
    eprintln!("isolated {mode} fixture PID {}", child.id());
    let until = Instant::now() + Duration::from_secs(30);
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if Instant::now() > until {
            child.kill().unwrap(); // Only the exact child PID started above.
            let status = child.wait().unwrap();
            panic!("isolated {mode} fixture timed out: {status}");
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    assert!(
        status.success(),
        "isolated {mode}: {status}\n{}\n{}",
        std::fs::read_to_string(stdout).unwrap(),
        std::fs::read_to_string(stderr).unwrap()
    );
}

#[test]
fn mcp_seat_spawn_runs_once_with_isolated_home_and_existing_wiring() {
    isolated_seat_fixture("spawn");
}

#[test]
fn mcp_spawn_refuses_forbidden_and_unknown_presets_before_admitting_a_seat() {
    let fixture = Fixture::new();
    let count = fixture.registry().list().unwrap().len();
    for (kind, role, reason) in [
        ("codex", "oracle", "codex_oracle_role_forbidden"),
        ("codex", "architect", "codex_oracle_role_forbidden"),
        ("zsh", "director", "unknown_role_preset"),
    ] {
        refused(
            fixture.mutate(
                fixture.oracle.id,
                Ulid::new(),
                "seat_spawn",
                json!({"swarm_id":fixture.own.swarm,"approved_profile":kind,"cwd":fixture._temp.path(),"role":role}),
            ),
            reason,
        );
        assert_eq!(fixture.registry().list().unwrap().len(), count);
    }
}

#[test]
fn mcp_seat_native_interrupt_is_bounded_audited_and_never_replayed() {
    isolated_seat_fixture("native");
}

#[test]
#[ignore = "subprocess fixture; parent supplies a disposable HOME and no credential environment"]
fn mcp_seat_isolated_process() {
    match std::env::var("BBOXY_MCP_SEAT_FIXTURE").unwrap().as_str() {
        "spawn" => seat_spawn_fixture(),
        "native" => seat_native_fixture(),
        _ => panic!("unknown fixture mode"),
    }
}

fn seat_spawn_fixture() {
    let home = PathBuf::from(std::env::var_os("HOME").unwrap());
    std::fs::write(home.join(".zshrc"), "printf 'ISOLATED-SPAWN\\n'\n").unwrap();
    let mut f = Fixture::new();
    let baseline = f.registry().list().unwrap().len();
    let args = json!({"swarm_id":f.own.swarm,"approved_profile":"zsh","cwd":f._temp.path(),"role":"oracle"});
    let op = Ulid::new();
    let first = f.mutate(f.oracle.id, op, "seat_spawn", args.clone());
    let result = data(first.clone());
    assert_eq!(result["status"], "applied");
    let seat: Ulid = serde_json::from_value(result["seat_id"].clone()).unwrap();
    assert!(result["pid"].as_u64().unwrap() > 0);
    f.wait_screen(seat, "ISOLATED-SPAWN");
    let record = f.registry().get(seat).unwrap();
    assert_eq!(record.role.name, "oracle");
    assert_eq!(record.role.class, Class::Oracle);
    assert_eq!(record.swarm, f.own.swarm);
    assert_eq!(f.registry().list().unwrap().len(), baseline + 1);
    assert_eq!(f.mutate(f.oracle.id, op, "seat_spawn", args.clone()), first);
    let overlays = f.config.run_dir.join(format!("seat-{seat}/overlays"));
    let dirs: Vec<_> = std::fs::read_dir(&overlays)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    assert_eq!(dirs.len(), 1);
    assert!(
        dirs[0].join(".zshrc").exists(),
        "missing wiring at {overlays:?}"
    );
    f.restart();
    assert_eq!(f.mutate(f.oracle.id, op, "seat_spawn", args.clone()), first);
    assert_eq!(f.registry().list().unwrap().len(), baseline + 1);
    assert!(f.screen(seat).contains("ISOLATED-SPAWN"));
    for (field, value) in [
        ("class", json!("oracle")),
        ("env", json!({"HOME":"/untrusted"})),
        ("program", json!("/bin/sh")),
    ] {
        let mut invalid = args.clone();
        invalid[field] = value;
        refused(
            f.mutate(f.oracle.id, Ulid::new(), "seat_spawn", invalid),
            "tool schema",
        );
    }
    let mut invalid = args;
    invalid["approved_profile"] = json!("arbitrary");
    refused(
        f.mutate(f.oracle.id, Ulid::new(), "seat_spawn", invalid),
        "unapproved spawn profile",
    );
    assert_eq!(f.registry().list().unwrap().len(), baseline + 1);
    assert!(!home.join(".claude").exists());
    assert!(!home.join(".codex").exists());
}

fn seat_native_fixture() {
    use bbt_mux::adapters::Endpoint;
    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::tungstenite::Message;
    let mut f = Fixture::new();
    let path = std::env::temp_dir().join(format!("mcp-native-{}.sock", Ulid::new()));
    let claude = f.spawn_fixture(
        "claude",
        "printf 'NATIVE-READY\\n'; while IFS= read -r line; do :; done",
    );
    let (server, mut frames) = f.runtime.block_on(async {
        let listener = tokio::net::UnixListener::bind(&path).unwrap();
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let task = tokio::spawn(async move {
            loop {
                let (stream, _) = listener.accept().await.unwrap();
                let mut reader = tokio::io::BufReader::new(stream);
                if let Ok(Ok(Some(bytes))) = tokio::time::timeout(
                    Duration::from_secs(1),
                    bbt_mux::seat_socket::read_line(&mut reader),
                )
                .await
                {
                    tx.send(serde_json::from_slice::<Value>(&bytes).unwrap())
                        .unwrap();
                }
            }
        });
        (task, rx)
    });
    f.runtime
        .block_on(f.client().bind_delivery_endpoint(
            claude.id,
            1,
            Endpoint::Claude { path: path.clone() },
        ))
        .unwrap();
    let mut args = f.target(claude.id);
    args["text"] = json!("literal native fixture");
    let op = Ulid::new();
    let first = f.mutate(f.oracle.id, op, "seat_interrupt", args.clone());
    let result = data(first.clone());
    assert_eq!(result["evidence"], "native-written");
    assert!(
        result["note"]
            .as_str()
            .unwrap()
            .contains("acceptance unverified")
    );
    let frame = f.runtime.block_on(async {
        tokio::time::timeout(Duration::from_secs(1), frames.recv())
            .await
            .unwrap()
            .unwrap()
    });
    assert_eq!(frame["priority"], "now");
    assert_eq!(frame["message"]["content"], args["text"]);
    assert_eq!(frame["uuid"], result["input_id"]);
    assert_eq!(
        f.mutate(f.oracle.id, op, "seat_interrupt", args.clone()),
        first
    );
    f.restart();
    assert_eq!(
        f.mutate(f.oracle.id, op, "seat_interrupt", args.clone()),
        first
    );
    args["text"] = json!("different");
    refused(
        f.mutate(f.oracle.id, op, "seat_interrupt", args),
        "op_id_reused",
    );
    assert!(frames.try_recv().is_err());
    server.abort();
    std::fs::remove_file(path).unwrap();

    let codex = f.spawn_fixture(
        "codex",
        "printf 'CODEX-READY\\n'; while IFS= read -r line; do :; done",
    );
    let path = std::env::temp_dir().join(format!("mcp-codex-{}.sock", Ulid::new()));
    let thread = codex.harness_session_id.clone().unwrap();
    let hold = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let flag = hold.clone();
    let (server, mut frames) = f.runtime.block_on(async {
        let listener = tokio::net::UnixListener::bind(&path).unwrap();
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let task = tokio::spawn(async move {
            let mut connections = tokio::task::JoinSet::new();
            loop {
                let (stream, _) = listener.accept().await.unwrap();
                let tx = tx.clone();
                let thread = thread.clone();
                let flag = flag.clone();
                connections.spawn(async move {
                    let mut ws = tokio_tungstenite::accept_async(stream).await.unwrap();
                    while let Some(Ok(frame)) = ws.next().await {
                        let Ok(text) = frame.to_text() else {
                            continue;
                        };
                        let value: Value = serde_json::from_str(text).unwrap();
                        let result = match value["method"].as_str().unwrap() {
                            "initialize" => json!({}),
                            "initialized" => continue,
                            "thread/read" => {
                                json!({"thread":{"id":thread,"status":{"type":"idle"}}})
                            }
                            "turn/steer" => {
                                assert_eq!(value["params"]["expectedTurnId"], "turn-fixture");
                                tx.send(value.clone()).unwrap();
                                if flag.load(std::sync::atomic::Ordering::SeqCst) {
                                    // No acknowledgement: production's own 2 s native bound decides UNKNOWN.
                                    let _ = ws.next().await;
                                    break;
                                }
                                json!({"turnId":"turn-fixture"})
                            }
                            other => panic!("unexpected native method {other}"),
                        };
                        if ws
                            .send(Message::Text(
                                json!({"id":value["id"],"result":result}).to_string().into(),
                            ))
                            .await
                            .is_err()
                        {
                            break;
                        }
                    }
                });
                while connections.try_join_next().is_some() {}
            }
        });
        (task, rx)
    });
    // Let automatic endpoint discovery install its reader before binding the fixture endpoint.
    std::thread::sleep(Duration::from_millis(350));
    f.runtime
        .block_on(f.client().bind_delivery_endpoint(
            codex.id,
            1,
            Endpoint::Codex {
                path: path.clone(),
                thread: codex.harness_session_id.clone().unwrap(),
                turn: Some("turn-fixture".into()),
            },
        ))
        .unwrap();
    let mut args = f.target(codex.id);
    args["text"] = json!("steer fixture");
    let first = data(f.mutate(f.oracle.id, Ulid::new(), "seat_interrupt", args.clone()));
    assert_eq!(first["evidence"], "native-queued");
    assert_eq!(first["native_id"], "turn-fixture");
    f.runtime.block_on(async {
        tokio::time::timeout(Duration::from_secs(1), frames.recv())
            .await
            .unwrap()
            .unwrap();
    });
    hold.store(true, std::sync::atomic::Ordering::SeqCst);
    let actor = f.oracle.id;
    let mut ops = Vec::new();
    let mut jobs = tokio::task::JoinSet::new();
    f.runtime.block_on(async {
        for _ in 0..8 {
            let client = f.client().clone();
            let op_id = Ulid::new();
            ops.push(op_id);
            let args = args.clone();
            jobs.spawn(async move {
                client
                    .control_call(
                        actor,
                        1,
                        Call {
                            op_id,
                            tool: "seat_interrupt".into(),
                            args,
                        },
                    )
                    .await
                    .unwrap()
            });
        }
        for received in 0..8 {
            if !matches!(
                tokio::time::timeout(Duration::from_secs(2), frames.recv()).await,
                Ok(Some(_))
            ) {
                let mut completed = Vec::new();
                while let Some(value) = jobs.try_join_next() {
                    completed.push(value.unwrap());
                }
                panic!("only {received}/8 native attempts; completed={completed:?}");
            }
        }
    });
    let start = Instant::now();
    let busy = f.call(actor, Ulid::new(), "seat_interrupt", args.clone());
    refused(busy, "capacity is full");
    assert_eq!(
        data(f.call(actor, Ulid::new(), "self_status", json!({})))["actor"]["seat_id"],
        json!(actor)
    );
    assert!(
        start.elapsed() < Duration::from_millis(1500),
        "Worker blocked on native I/O"
    );
    f.runtime.block_on(async {
        while let Some(result) = jobs.join_next().await {
            refused(result.unwrap(), "unknown");
        }
    });
    for op in ops {
        refused(
            f.mutate(actor, op, "seat_interrupt", args.clone()),
            "unknown",
        );
    }
    assert!(
        frames.try_recv().is_err(),
        "an unknown operation was redispatched"
    );
    // Cancel the Worker after a native mutation but before its acknowledgement.
    // The durable admission survives and the next Worker cannot redispatch it.
    let crash_op = Ulid::new();
    let client = f.client().clone();
    let crash_args = args.clone();
    let pending = f.runtime.spawn(async move {
        client
            .control_call(
                actor,
                1,
                Call {
                    op_id: crash_op,
                    tool: "seat_interrupt".into(),
                    args: crash_args,
                },
            )
            .await
    });
    f.runtime.block_on(async {
        tokio::time::timeout(Duration::from_secs(2), frames.recv())
            .await
            .unwrap()
            .unwrap();
    });
    f.restart();
    assert!(f.runtime.block_on(pending).unwrap().is_err());
    refused(f.mutate(actor, crash_op, "seat_interrupt", args), "unknown");
    assert!(frames.try_recv().is_err());
    let audit = f.audits(actor, 30);
    assert!(
        audit
            .iter()
            .any(|event| event.event.payload["tool"] == "seat_interrupt")
    );
    for event in audit {
        event.verify().unwrap();
    }
    server.abort();
    std::fs::remove_file(path).unwrap();
}

#[test]
fn mcp_record_numbers_resolve_before_the_existing_task_authority_fences() {
    let f = Fixture::new();
    f.task_file(f.task_metadata());
    for args in [json!({"task_path":"#1"}), json!({"task_id":"#1"})] {
        let task = data(f.call(f.own.id, Ulid::new(), "task_status", args));
        assert_eq!(task["id"], f.task.to_string());
        assert_eq!(task["metadata"]["number"], 1);
        assert_eq!(task["path"], f.task_path());
    }
    for caller in [f.own.id, f.oracle.id] {
        let denied = f.call(
            caller,
            Ulid::new(),
            "task_status",
            json!({"task_path":"#2"}),
        );
        assert_eq!(denied["isError"], true);
        assert!(!denied.to_string().contains("Foreign fixture task"));
    }
    assert_eq!(
        f.mutate(
            f.own.id,
            Ulid::new(),
            "task_accept",
            json!({"task_path":"#1"})
        )["isError"],
        true
    );
    data(f.mutate(f.own.id, Ulid::new(), "task_report", json!({"task_path":"#1","summary":"Numbered report","verify_result":verification("true",0,json!([]))})));
    assert_eq!(
        data(f.mutate(
            f.oracle.id,
            Ulid::new(),
            "task_accept",
            json!({"task_path":"#1"})
        ))["task_status"],
        "accepted"
    );
    assert_eq!(f.task_document().metadata["number"], 1);
    assert_eq!(
        f.call(f.own.id, Ulid::new(), "task_list", json!({"after":"#1"}))["isError"],
        true
    );
}

#[test]
fn mcp_artifact_number_uses_unindexed_records_of_every_type_and_keeps_retry_bytes() {
    let mut f = Fixture::new();
    f.task_file(f.task_metadata());
    let nested = f.config.repository.join(".bboxy/nested");
    std::fs::create_dir(&nested).unwrap();
    std::fs::write(
        nested.join("spec.md"),
        format!(
            "\u{feff}---\r\nid: {}\r\ntype: spec\r\nnumber: 9001\r\n---\r\nnumber: 999999\r\n",
            Ulid::new()
        ),
    )
    .unwrap();
    let op = Ulid::new();
    let args = json!({"kind":"file","ref":"fixture: numbered artifact","task":"#1"});
    let first = f.mutate(f.own.id, op, "artifact_claim", args.clone());
    data(first.clone());
    let path = f
        .config
        .repository
        .join(format!(".bboxy/artifacts/{op}.md"));
    let source = std::fs::read_to_string(&path).unwrap();
    let doc = bbt_mux::index::Document::parse("artifact.md", &source);
    assert_eq!(doc.metadata["number"], 9002);
    assert_item_signature(&doc, f.own.id);
    assert_eq!(f.task_document().metadata["number"], 1);
    f.restart();
    assert_eq!(f.mutate(f.own.id, op, "artifact_claim", args), first);
    assert_eq!(std::fs::read_to_string(path).unwrap(), source);
}

#[test]
fn mcp_numeric_selectors_refuse_invalid_missing_and_ambiguous_handles() {
    let f = Fixture::new();
    for locator in [
        "#0",
        "#01",
        "#-1",
        "#1.5",
        "#1x",
        "#18446744073709551616",
        "#999",
    ] {
        for field in ["task_path", "task_id"] {
            assert_eq!(
                f.call(f.own.id, Ulid::new(), "task_status", json!({field:locator}))["isError"],
                true,
                "{field} {locator}"
            );
        }
    }
    let path = f.config.repository.join(".bboxy/duplicate.md");
    std::fs::write(
        path,
        format!(
            "---\nid: {}\ntype: spec\nnumber: 1\n---\nPrivate duplicate content\n",
            Ulid::new()
        ),
    )
    .unwrap();
    let result = f.call(
        f.own.id,
        Ulid::new(),
        "task_status",
        json!({"task_path":"#1"}),
    );
    assert_eq!(result["isError"], true);
    assert!(result.to_string().contains("ambiguous"));
    assert!(!result.to_string().contains("Private duplicate content"));
    let op = Ulid::new();
    assert_eq!(
        f.mutate(
            f.own.id,
            op,
            "artifact_claim",
            json!({"kind":"file","ref":"must not write"})
        )["isError"],
        true
    );
    assert!(
        !f.config
            .repository
            .join(format!(".bboxy/artifacts/{op}.md"))
            .exists()
    );
}

fn knowledge_args(path: &str, body: &str) -> Value {
    json!({"path":path,"frontmatter":{"type":"e2e","title":"T1 fixture","function":"T1","group":"terminal","status":"untested","expected":"fixture expectation","unknown":{"kept":true}},"body":body})
}
fn knowledge_fetch(f: &Fixture, seat: Ulid, locator: &str) -> Value {
    let mut bridge = f.bridge(seat);
    initialize(&mut bridge);
    let reply = rpc(
        &mut bridge,
        2,
        "tools/call",
        json!({"name":"knowledge_fetch","arguments":{"locator":locator}}),
    );
    assert!(reply.get("error").is_none(), "{reply}");
    reply["result"].clone()
}
fn verify_knowledge_record(record: &Value) -> [u8; 32] {
    use base64::Engine;
    let mut metadata = record["frontmatter"].clone();
    let proof = metadata
        .as_object_mut()
        .unwrap()
        .remove("provenance")
        .unwrap();
    let body = record["body"].as_str().unwrap();
    let front_hash = bbt_mux::store::digest(serde_yaml::to_string(&metadata).unwrap().as_bytes());
    let body_hash = bbt_mux::store::digest(body.as_bytes());
    assert_eq!(proof["frontmatter_digest"], format!("sha256:{front_hash}"));
    assert_eq!(proof["digest"], format!("sha256:{body_hash}"));
    let payload = format!("{front_hash}{body_hash}");
    let domain = b"bboxy/item/v1";
    let mut pae = format!("DSSEv1 {} ", domain.len()).into_bytes();
    pae.extend(domain);
    pae.extend(format!(" {} ", payload.len()).as_bytes());
    pae.extend(payload.as_bytes());
    let public: [u8; 32] = base64::engine::general_purpose::STANDARD
        .decode(metadata["authorship"]["public_key"].as_str().unwrap())
        .unwrap()
        .try_into()
        .unwrap();
    let sig = base64::engine::general_purpose::STANDARD
        .decode(proof["sig"].as_str().unwrap())
        .unwrap();
    ed25519_dalek::VerifyingKey::from_bytes(&public)
        .unwrap()
        .verify_strict(&pae, &ed25519_dalek::Signature::from_slice(&sig).unwrap())
        .unwrap();
    public
}

#[test]
fn mcp_knowledge_real_bridge_writes_with_the_seat_key_and_replays_without_reallocation() {
    let mut f = Fixture::new();
    let mut args = knowledge_args("e2e/t1.md", "# Untested T1\n");
    args["frontmatter"]["authorship"] =
        json!({"signer":"device:forged","kind":"device","public_key":"forged"});
    let mut bridge = f.bridge(f.own.id);
    initialize(&mut bridge);
    let first = rpc(
        &mut bridge,
        2,
        "tools/call",
        json!({"name":"knowledge_write","arguments":args}),
    )["result"]
        .clone();
    let op = first["structuredContent"]["op_id"]
        .as_str()
        .unwrap()
        .parse::<Ulid>()
        .unwrap();
    let value = data(first.clone());
    assert_eq!(value["record"]["id"], json!(op));
    assert_eq!(value["record"]["number"], 3);
    assert_eq!(
        value["record"]["frontmatter"]["unknown"],
        json!({"kept":true})
    );
    assert_eq!(
        value["record"]["frontmatter"]["authorship"]["signer"],
        format!("seat:{}", f.own.id)
    );
    assert_eq!(value["record"]["frontmatter"]["authorship"]["kind"], "seat");
    assert_eq!(
        value["record"]["frontmatter"]["authorship"]["process_generation"],
        "1"
    );
    let public = verify_knowledge_record(&value["record"]);
    let path = f.config.repository.join(".bboxy/e2e/t1.md");
    let bytes = std::fs::read(&path).unwrap();
    assert_eq!(
        f.mutate(f.own.id, op, "knowledge_write", args.clone()),
        first
    );
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    let mut conflicting = args.clone();
    conflicting["body"] = json!("changed arguments");
    refused(
        f.mutate(f.own.id, op, "knowledge_write", conflicting),
        "op_id_reused",
    );
    let fetched = data(knowledge_fetch(&f, f.own.id, "#3"));
    assert_eq!(fetched["id"], json!(op));
    assert_eq!(fetched["body"], "# Untested T1\n");
    verify_knowledge_record(&fetched);
    let directory = bbt_mux::store::object_dir(&f.config.run_dir, "index").unwrap();
    let log = EventLog::open(&directory).unwrap();
    assert!(log.verify_chain().unwrap() > 0);
    let written: Vec<_> = log
        .after(0, 1024)
        .unwrap()
        .into_iter()
        .filter(|row| row.event.subject == format!("doc/{op}/written"))
        .collect();
    assert_eq!(written.len(), 1);
    written[0].verify().unwrap();
    assert_eq!(written[0].provenance.signer, public);
    assert_eq!(written[0].event.seat, Some(f.own.id));
    assert_eq!(written[0].event.generation, Some(1));
    f.restart();
    assert_eq!(f.mutate(f.own.id, op, "knowledge_write", args), first);
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    let reference: bbt_mux::knowledge::data_source::RecordRef =
        serde_json::from_value(value["record"].clone()).unwrap();
    let update = json!({"ref":reference,"frontmatter":value["record"]["frontmatter"],"body":"updated by the other admitted fixture seat\n","etag":value["record"]["etag"]});
    let peer = data(f.mutate(f.peer.id, Ulid::new(), "knowledge_write", update));
    assert_eq!(peer["record"]["number"], 3);
    assert_eq!(peer["record"]["id"], json!(op));
    assert_eq!(
        peer["record"]["frontmatter"]["authorship"]["signer"],
        format!("seat:{}", f.peer.id)
    );
    assert_ne!(verify_knowledge_record(&peer["record"]), public);
    let catalogue = f
        .runtime
        .block_on(f.client().control_catalogue(f.own.id, 1))
        .unwrap();
    let write = catalogue["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|tool| tool["name"] == "knowledge_write")
        .unwrap();
    assert_eq!(
        write["inputSchema"]["properties"]["op_id"]["type"],
        "string"
    );
    assert!(
        !write["inputSchema"]["required"]
            .as_array()
            .unwrap()
            .contains(&json!("op_id"))
    );
}

#[test]
fn mcp_knowledge_refuses_outside_paths_forged_authority_type_ids_and_stale_etags() {
    use std::os::unix::fs::symlink;
    let f = Fixture::new();
    let outside = f
        .config
        .repository
        .join(format!(".bboxy/tasks/{}.md", f.task));
    let original = std::fs::read(&outside).unwrap();
    for path in [
        "tasks/forbidden.md",
        "../escape.md",
        "e2evil/no.md",
        "e2e/../../escape.md",
        "e2e/not-markdown.txt",
    ] {
        refused(
            f.mutate(
                f.own.id,
                Ulid::new(),
                "knowledge_write",
                knowledge_args(path, "denied"),
            ),
            "restricted to e2e/",
        );
    }
    let mut args = knowledge_args("e2e/forged.md", "denied");
    args["seat_id"] = json!(f.oracle.id);
    refused(
        f.mutate(f.own.id, Ulid::new(), "knowledge_write", args),
        "tool schema",
    );
    let mut args = knowledge_args("e2e/wrong-type.md", "denied");
    args["frontmatter"]["type"] = json!("task");
    refused(
        f.mutate(f.own.id, Ulid::new(), "knowledge_write", args),
        "record type e2e",
    );
    let mut args = knowledge_args("e2e/wrong-id.md", "denied");
    args["frontmatter"]["id"] = json!(f.task);
    refused(
        f.mutate(f.own.id, Ulid::new(), "knowledge_write", args),
        "id must equal op_id",
    );
    refused(
        f.mutate(
            f.own.id,
            f.task,
            "knowledge_write",
            knowledge_args("e2e/alias-task.md", "denied"),
        ),
        "already exists",
    );
    refused(knowledge_fetch(&f, f.own.id, "#1"), "restricted to e2e/");
    assert_eq!(std::fs::read(&outside).unwrap(), original);
    std::fs::create_dir_all(f.config.repository.join(".bboxy/e2e")).unwrap();
    let link = f.config.repository.join(".bboxy/e2e/link.md");
    symlink(&outside, &link).unwrap();
    let guarded = f.mutate(
        f.own.id,
        Ulid::new(),
        "knowledge_write",
        knowledge_args("e2e/link.md", "denied"),
    );
    assert_eq!(guarded["isError"], true);
    assert_eq!(std::fs::read(&outside).unwrap(), original);
    let created = data(f.mutate(
        f.own.id,
        Ulid::new(),
        "knowledge_write",
        knowledge_args("e2e/valid.md", "original"),
    ));
    let reference: bbt_mux::knowledge::data_source::RecordRef =
        serde_json::from_value(created["record"].clone()).unwrap();
    let metadata = created["record"]["frontmatter"].clone();
    let path = f.config.repository.join(".bboxy/e2e/valid.md");
    let before = std::fs::read(&path).unwrap();
    refused(
        f.mutate(
            f.own.id,
            Ulid::new(),
            "knowledge_write",
            json!({"ref":reference,"frontmatter":metadata,"body":"stale","etag":"wrong"}),
        ),
        "etag_mismatch",
    );
    let mut forged = reference.clone();
    forged.path = format!("tasks/{}.md", f.task);
    refused(
        f.mutate(
            f.own.id,
            Ulid::new(),
            "knowledge_write",
            json!({"ref":forged,"frontmatter":metadata,"body":"denied"}),
        ),
        "restricted to e2e/",
    );
    assert_eq!(std::fs::read(&path).unwrap(), before);
    assert_eq!(std::fs::read(&outside).unwrap(), original);
    assert!(!f.config.repository.join("escape.md").exists());
    assert!(
        !f.config
            .repository
            .join(".bboxy/e2e/alias-task.md")
            .exists()
    );
    let mut bridge = f.bridge(f.own.id);
    initialize(&mut bridge);
    let mut invalid_id = knowledge_args("e2e/no-op.md", "denied");
    invalid_id["op_id"] = json!("not-a-ulid");
    assert_eq!(
        rpc(
            &mut bridge,
            2,
            "tools/call",
            json!({"name":"knowledge_write","arguments":invalid_id})
        )["error"]["code"],
        -32602
    );
    assert!(!f.config.repository.join(".bboxy/e2e/no-op.md").exists());
}

#[test]
fn mcp_knowledge_post_commit_failure_is_unknown_and_never_recreates_a_deleted_file() {
    let mut f = Fixture::new();
    let directory = bbt_mux::store::object_dir(&f.config.run_dir, "index").unwrap();
    let db = rusqlite::Connection::open(directory.join("events.sqlite")).unwrap();
    db.execute_batch("CREATE TRIGGER fixture_lost_write_outcome BEFORE INSERT ON events WHEN NEW.subject LIKE 'doc/%/written' BEGIN SELECT RAISE(ABORT,'fixture loss after file commit'); END;").unwrap();
    let id = Ulid::new();
    let args = knowledge_args("e2e/uncertain.md", "actually committed\n");
    let first = f.mutate(f.own.id, id, "knowledge_write", args.clone());
    refused(first.clone(), "unknown");
    let path = f.config.repository.join(".bboxy/e2e/uncertain.md");
    assert!(path.exists(), "fault is after the atomic file effect");
    db.execute_batch("DROP TRIGGER fixture_lost_write_outcome")
        .unwrap();
    drop(db);
    std::fs::remove_file(&path).unwrap();
    assert_eq!(
        f.mutate(f.own.id, id, "knowledge_write", args.clone()),
        first
    );
    assert!(!path.exists());
    f.restart();
    assert_eq!(f.mutate(f.own.id, id, "knowledge_write", args), first);
    assert!(!path.exists());
    let log = EventLog::open(&directory).unwrap();
    assert!(log.verify_chain().unwrap() > 0);
    assert_eq!(
        log.after(0, 1024)
            .unwrap()
            .iter()
            .filter(|row| row.event.subject == format!("doc/{id}/write_prepared"))
            .count(),
        1
    );
}

/// Decision #264 over the real MCP bridge: the operator's DM to a seat.
fn operator_dm(f: &Fixture, seat: Ulid, text: &str) -> Ulid {
    use bbt_proto::messaging::Request;
    let id = Ulid::new();
    let mut message = bbt_mux::messaging::SendRequest::text(format!("seats/{seat}/inbox"), text);
    message.id = id;
    let reply = f.gui(
        id,
        bbt_proto::GuiAction::Messaging {
            id,
            swarm_id: bbt_proto::SwarmId::from_u128(f.own.swarm.0),
            request: Request::Send { message },
        },
    );
    assert!(
        matches!(reply, bbt_proto::GuiReply::Messaging { .. }),
        "{reply:?}"
    );
    id
}
/// Every message the operator's inbox holds, by text.
fn operator_inbox(f: &Fixture) -> Vec<String> {
    use bbt_proto::messaging::{Request, Response};
    let id = Ulid::new();
    let reply = f.gui(
        id,
        bbt_proto::GuiAction::Messaging {
            id,
            swarm_id: bbt_proto::SwarmId::from_u128(f.own.swarm.0),
            request: Request::List {
                thread: None,
                channel: None,
                after: 0,
                limit: 128,
            },
        },
    );
    let bbt_proto::GuiReply::Messaging {
        response: Response::Messages { page },
        ..
    } = reply
    else {
        panic!("{reply:?}")
    };
    page.items
        .iter()
        .map(|item| serde_json::to_value(&item.envelope.body).unwrap()["text"].to_string())
        .collect()
}
fn texts(page: &Value) -> String {
    page["page"]["items"].to_string()
}

#[test]
fn d264_a_reply_without_destination_goes_to_the_sender() {
    let f = Fixture::new();
    f.prepare_operator();
    let (own, peer) = (f.own.id, f.peer.id);
    // To the operator: the reply to an operator DM lands in operator/inbox.
    let dm = operator_dm(&f, own, "operator asks");
    let sent = data(f.mutate(
        own,
        Ulid::new(),
        "message_send",
        json!({"text":"answer to the operator","reply_to":dm}),
    ));
    assert_eq!(
        sent["message"]["envelope"]["context"], "operator/inbox",
        "{sent}"
    );
    assert!(
        operator_inbox(&f)
            .iter()
            .any(|text| text.contains("answer to the operator"))
    );
    // To a seat: the reply to a peer's message lands in the peer's inbox.
    let asked = data(f.mutate(
        peer,
        Ulid::new(),
        "message_send",
        json!({"destination":format!("seats/{own}/inbox"),"text":"peer asks"}),
    ));
    let asked = asked["message"]["envelope"]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    data(f.mutate(
        own,
        Ulid::new(),
        "message_send",
        json!({"text":"answer to the peer","reply_to":asked}),
    ));
    let inbox = data(f.call(peer, Ulid::new(), "message_list", json!({})));
    assert!(texts(&inbox).contains("answer to the peer"), "{inbox}");
    // Neither destination nor reply_to: refused, naming both ways.
    refused(
        f.mutate(
            own,
            Ulid::new(),
            "message_send",
            json!({"text":"to nobody"}),
        ),
        "reply_to",
    );
}

#[test]
fn d264_orient_and_the_skill_say_how_to_reach_the_operator() {
    let f = Fixture::new();
    let line = "to reach the operator, reply with reply_to, or send to operator/inbox";
    let orient = data(f.call(f.own.id, Ulid::new(), "orient", json!({})));
    assert_eq!(orient["operator"], line, "{orient}");
    let skill = include_str!("../src/adapters/skills/bboxy450/SKILL.md");
    assert!(
        skill
            .contains("To reach the operator, reply with `reply_to`, or send to `operator/inbox`."),
        "the skill names the operator's inbox"
    );
}

#[test]
fn d264_an_unknown_destination_is_refused_with_the_valid_one() {
    let f = Fixture::new();
    f.prepare_operator();
    let own = f.own.id;
    // The seat that guessed the operator's inbox from the envelope's nil seat id.
    let dm = operator_dm(&f, own, "operator asks again");
    let nil = format!("seats/{}/inbox", Ulid::nil());
    refused(
        f.mutate(
            own,
            Ulid::new(),
            "message_send",
            json!({"destination":nil,"text":"guessed","reply_to":dm}),
        ),
        "came from operator/inbox",
    );
    // Without a message to infer from, the refusal lists what the seat may address.
    let listed = refused(
        f.mutate(
            own,
            Ulid::new(),
            "message_send",
            json!({"destination":"seats/nowhere/inbox","text":"lost"}),
        ),
        "you may address: ",
    );
    let listed = listed.to_string();
    assert!(listed.contains("operator/inbox"), "{listed}");
    assert!(
        listed.contains(&format!("seats/{}/inbox", f.peer.id)),
        "{listed}"
    );
}

#[test]
fn d264_a_refused_send_is_stored_nowhere() {
    let f = Fixture::new();
    f.prepare_operator();
    let own = f.own.id;
    let dm = operator_dm(&f, own, "operator asks a third time");
    let nil = format!("seats/{}/inbox", Ulid::nil());
    for args in [
        json!({"destination":nil,"text":"refused payload one","reply_to":dm}),
        json!({"destination":"seats/nowhere/inbox","text":"refused payload two"}),
        json!({"text":"refused payload three"}),
    ] {
        // A tool error, and the refusal does not echo the payload back.
        let error = refused(f.mutate(own, Ulid::new(), "message_send", args), "rejected");
        assert!(!error.to_string().contains("refused payload"), "{error}");
    }
    for seat in [own, f.peer.id, f.oracle.id] {
        let page = data(f.call(seat, Ulid::new(), "message_list", json!({"limit":128})));
        assert!(!texts(&page).contains("refused payload"), "{seat}: {page}");
    }
    assert!(
        !operator_inbox(&f)
            .iter()
            .any(|text| text.contains("refused payload"))
    );
}
