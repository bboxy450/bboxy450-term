// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
#![allow(clippy::disallowed_methods)] // Controlled blocking IO/process fixtures.

use bbt_mux::messaging::wire::EnvelopeExt;
mod common;
#[path = "common/record_supervisor.rs"]
mod record_supervisor;
use record_supervisor::RecordLink;
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

type NativeRuntimes = std::sync::Arc<
    std::sync::Mutex<std::collections::BTreeMap<Ulid, bbt_supervisor::protocol::CompanionInfo>>,
>;

struct Fixture {
    runtime: tokio::runtime::Runtime,
    logic: Option<Logic>,
    supervisor: Option<Supervisor>,
    record_link: Option<RecordLink>,
    config: LogicConfig,
    own: SeatRecord,
    peer: SeatRecord,
    oracle: SeatRecord,
    outside: SeatRecord,
    task: Ulid,
    foreign_task: Ulid,
    input_attempts: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    history_gap: std::sync::Arc<std::sync::atomic::AtomicBool>,
    native_runtime: NativeRuntimes,
    _temp: tempfile::TempDir,
}
impl Fixture {
    fn new() -> Self {
        Self::open(false, false)
    }
    fn real() -> Self {
        Self::open(true, false)
    }
    fn with_lost_input_receipt(lose: bool) -> Self {
        Self::open(true, lose)
    }
    fn open(real: bool, lose: bool) -> Self {
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
        let supervisor =
            real.then(|| Supervisor::start(common::capture_config(temp.path())).unwrap());
        let record_link = (!real).then(RecordLink::default);
        let input_attempts = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let history_gap = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let native_runtime =
            std::sync::Arc::new(std::sync::Mutex::new(std::collections::BTreeMap::new()));
        let logic = Logic::start(
            LostInputReceipt {
                client: match &record_link {
                    Some(link) => Connection::Records(link.clone()),
                    None => Connection::Real(supervisor.as_ref().unwrap().connect().unwrap()),
                },
                lose,
                attempts: input_attempts.clone(),
                history_gap: history_gap.clone(),
                native_runtime: native_runtime.clone(),
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
            supervisor,
            record_link,
            config,
            own,
            peer,
            oracle,
            outside,
            task,
            foreign_task,
            input_attempts,
            history_gap,
            native_runtime,
            _temp: temp,
        }
    }
    fn connection(&self) -> Connection {
        match &self.record_link {
            Some(link) => Connection::Records(link.clone()),
            None => Connection::Real(self.supervisor.as_ref().unwrap().connect().unwrap()),
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
        let call = Call {
            op_id,
            tool: tool.into(),
            args,
        };
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let value = self
                .runtime
                .block_on(self.client().control_call(seat, 1, call.clone()))
                .unwrap();
            if value["structuredContent"]["error"]["code"] != "workspace_pending"
                && value["structuredContent"]["tasks"]["availability"] != "workspace_pending"
            {
                return value;
            }
            assert!(
                Instant::now() < deadline,
                "workspace never settled: {value}"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    fn registry(&self) -> Registry {
        Registry::new(SqliteStore::open(&self.config.run_dir.join("account")).unwrap())
    }
    fn mutate(&self, seat: Ulid, op_id: Ulid, tool: &str, args: Value) -> Value {
        self.mutate_at("mutation", seat, op_id, tool, args)
    }
    fn mutate_at(
        &self,
        phase: &str,
        seat: Ulid,
        op_id: Ulid,
        tool: &str,
        mut args: Value,
    ) -> Value {
        let started = Instant::now();
        if tool == "task_import" {
            eprintln!("import call started: tool={tool}, phase={phase}, op={op_id}");
        }
        args["op_id"] = json!(op_id);
        let mut bridge = self.bridge(seat);
        initialize(&mut bridge);
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let reply = rpc(
                &mut bridge,
                2,
                "tools/call",
                json!({"name":tool,"arguments":args}),
            );
            if reply.get("error").is_some() {
                // Inspect this exact operation without calling it again. Opening the
                // existing private database read-only cannot create/admit an effect.
                let retained = self.original_operation(seat, op_id);
                panic!(
                    "MCP mutation failed: tool={tool}, phase={phase}, op={op_id}, elapsed_ms={}, original_reply={reply}, read_only_original_operation={retained}",
                    started.elapsed().as_millis()
                );
            }
            let value = reply["result"].clone();
            if value["structuredContent"]["error"]["code"] != "workspace_pending" {
                if tool == "task_import" {
                    eprintln!(
                        "import call: tool={tool}, phase={phase}, op={op_id}, elapsed_ms={}, status={}, error_code={}",
                        started.elapsed().as_millis(),
                        value["structuredContent"]["status"],
                        value["structuredContent"]["error"]["code"]
                    );
                }
                return value;
            }
            assert!(
                Instant::now() < deadline,
                "workspace never settled: {value}"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    fn original_operation(&self, seat: Ulid, op_id: Ulid) -> Value {
        let journal = self
            .config
            .run_dir
            .join(format!("seat-{seat}/object.sqlite"));
        rusqlite::Connection::open_with_flags(&journal, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .and_then(|db| {
                db.query_row(
                    "SELECT value FROM kv WHERE key=?1",
                    [format!("control/op/{op_id}")],
                    |row| row.get::<_, Vec<u8>>(0),
                )
            })
            .map(|bytes| {
                let entry: Value = serde_json::from_slice(&bytes).unwrap();
                json!({"tool":entry["tool"],"status":entry["result"]["structuredContent"]["status"],
                    "error_code":entry["result"]["structuredContent"]["error"]["code"],
                    "source_records":entry["result"]["structuredContent"]["source_records"]})
            })
            .unwrap_or_else(|error| json!({"read_only_query":format!("{error:?}")}))
    }
    fn audits(&self, seat: Ulid, count: usize) -> Vec<bbt_mux::store::events::Envelope> {
        let until = Instant::now() + Duration::from_secs(120);
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
                LostInputReceipt {
                    client: self.connection(),
                    lose: false,
                    attempts: self.input_attempts.clone(),
                    history_gap: self.history_gap.clone(),
                    native_runtime: self.native_runtime.clone(),
                },
                self.config.clone(),
                key(),
                self.runtime.handle(),
            )
            .unwrap(),
        );
    }
}
enum Connection {
    Real(bbt_mux::Client),
    Records(RecordLink),
}
impl bbt_mux::logic::SupervisorConnection for Connection {
    fn request(&mut self, action: bbt_mux::Action) -> bbt_mux::store::Result<bbt_mux::Reply> {
        match self {
            Self::Real(client) => client.request(action),
            Self::Records(client) => client.request(action),
        }
    }
    fn receive(&mut self, timeout: Duration) -> bbt_mux::store::Result<Option<bbt_mux::Event>> {
        match self {
            Self::Real(client) => client.receive(timeout),
            Self::Records(client) => client.receive(timeout),
        }
    }
}
struct LostInputReceipt {
    client: Connection,
    lose: bool,
    attempts: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    history_gap: std::sync::Arc<std::sync::atomic::AtomicBool>,
    native_runtime: NativeRuntimes,
}
impl bbt_mux::logic::SupervisorConnection for LostInputReceipt {
    fn request(&mut self, action: bbt_mux::Action) -> bbt_mux::store::Result<bbt_mux::Reply> {
        let input = matches!(action, bbt_mux::Action::GuardedInput { .. });
        let mut result = self.client.request(action)?;
        // This RPC fixture hosts its native server in the test process. Supply
        // its explicit host facts at the existing fake transport boundary; real
        // companion subprocess ownership/reaping is checked in supervisor tests.
        let runtimes = self.native_runtime.lock().unwrap();
        match &mut result {
            bbt_mux::Reply::Spawned { info } => {
                info.companion = runtimes.get(&info.seat).cloned().or(info.companion.take());
            }
            bbt_mux::Reply::Seats { seats } => {
                for info in seats {
                    info.companion = runtimes.get(&info.seat).cloned().or(info.companion.take());
                }
            }
            _ => (),
        }
        drop(runtimes);

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
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let reply = self.gui_once(id, action.clone());
            if !matches!(&reply,bbt_proto::GuiReply::Error{reason,..} if reason.starts_with("workspace_pending:"))
            {
                return reply;
            }
            assert!(
                Instant::now() < deadline,
                "workspace never settled: {reply:?}"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    fn gui_once(&self, id: Ulid, action: bbt_proto::GuiAction) -> bbt_proto::GuiReply {
        let endpoint = self.runtime.block_on(self.client().gui_endpoint()).unwrap();
        let mut socket = UnixStream::connect(endpoint.path).unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(120)))
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
        let deadline = Instant::now() + Duration::from_secs(120);
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
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let reply = self.gui(
                id,
                bbt_proto::GuiAction::Task {
                    scope: None,
                    id,
                    request: request.clone(),
                },
            );
            if !matches!(&reply,bbt_proto::GuiReply::Error{reason,..} if reason.starts_with("workspace_pending:"))
            {
                return reply;
            }
            assert!(
                Instant::now() < deadline,
                "workspace never settled: {reply:?}"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}

// keeps: decision #28
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
    assert_eq!(
        message.envelope().expect("signed fixture envelope").seat,
        Ulid::nil()
    );
    assert_eq!(
        message
            .envelope()
            .expect("signed fixture envelope")
            .process_generation,
        0
    );
    assert_eq!(
        message
            .envelope()
            .expect("signed fixture envelope")
            .role
            .name,
        "operator"
    );
    assert_eq!(
        message
            .envelope()
            .expect("signed fixture envelope")
            .role
            .class,
        Class::Oracle
    );
    assert_eq!(
        message.envelope().expect("signed fixture envelope").author,
        message.envelope().expect("signed fixture envelope").device
    );
    message
        .envelope()
        .expect("signed fixture envelope")
        .verify_signature()
        .unwrap();
    let account = SqliteStore::open(&f.config.run_dir.join("account")).unwrap();
    let local_issuer = account.get("identity/local-issuer").unwrap().unwrap();
    assert_eq!(
        message
            .envelope()
            .expect("signed fixture envelope")
            .role_grant_id,
        bbt_mux::store::digest(&local_issuer)
    );
    let local_issuer: Value = serde_json::from_slice(&local_issuer).unwrap();
    assert_eq!(
        json!(
            message
                .envelope()
                .expect("signed fixture envelope")
                .role_epoch
                .to_string()
        ),
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
    // This fixture installs the device after Logic::start. Its already-minted
    // account-local messaging principal survives that later enrolment.
    let reference: bbt_proto::messaging::OperatorReference = serde_json::from_slice(
        &account
            .get("messaging/operator-reference")
            .unwrap()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        reference.kind,
        bbt_proto::messaging::OperatorReferenceKind::OperatorLocal
    );
    assert_ne!(reference.id, device);
    assert_eq!(entry["actor"]["reference"], json!(reference));
    assert!(entry["actor"].get("device_id").is_none());
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
    let published = Instant::now() + Duration::from_secs(120);
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
    assert_eq!(row.author.kind, AuthorKind::OperatorLocal);
    assert_eq!(row.author.id, reference.id.to_string());
    assert_eq!(
        row.issuer_kind,
        Some(bbt_mux::messaging::wire::IssuerKind::LocalDev)
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
        scope: None,
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
        self.spawn_native_fixture(kind, script, None)
    }
    fn spawn_native_fixture(
        &self,
        kind: &str,
        script: &str,
        runtime: Option<bbt_supervisor::protocol::CompanionInfo>,
    ) -> SeatRecord {
        let mut record = SeatRecord::new(Ulid::new(), self.own.swarm, "Controlled target", kind);
        record.harness_session_id = Some(format!("fixture-{}", record.id));
        if let Some(runtime) = runtime {
            self.native_runtime
                .lock()
                .unwrap()
                .insert(record.id, runtime);
        }
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
        let until = Instant::now() + Duration::from_secs(120);
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
            tokio::time::timeout(Duration::from_secs(120), async {
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

// keeps: spec mcp-control-plane
#[test]
fn mcp_seat_controls_enforce_catalogue_scope_generation_and_strict_arguments() {
    let f = Fixture::new();
    let controls = [
        "seat_input_text",
        "seat_spawn",
        "seat_split",
        "seat_focus",
        "layout_read",
        "ctl",
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
        controls.len()
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
    assert_eq!(no_proof["cause"], "composer-readiness-missing");
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

const ONE_INPUT_SHELL: &str = "stty icanon echo; printf 'FIXTURE-READY\\n'; printf '\\033]133;B\\007'; if IFS= read -r line; then printf '\\033]133;C\\007'; printf 'OBSERVED:%s\\n' \"$line\"; fi; while IFS= read -r line; do :; done";

// keeps: spec mcp-control-plane
#[test]
#[ignore = "#327 B2/B4: real terminal/native boundary; scripts/night-test-boundaries.sh"]
fn mcp_seat_input_consumes_readiness_and_enter_is_explicit() {
    // A single prompt keeps consumed readiness consumed, and the completed
    // read stays active until teardown rather than rearming after a sleep.
    let mut f = Fixture::real();
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

// keeps: spec mcp-control-plane
#[test]
#[ignore = "#327 B2/B4: real terminal/native boundary; scripts/night-test-boundaries.sh"]
fn mcp_seat_input_lost_receipt_is_unknown_and_never_repeated_after_restart() {
    let mut f = Fixture::with_lost_input_receipt(true);
    let target = f.spawn_fixture("zsh", ONE_INPUT_SHELL);
    f.wait_screen(target.id, "FIXTURE-READY");
    f.wait_input_ready(target.id);
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

// keeps: spec mcp-control-plane
#[test]
#[ignore = "#327 B2/B4: real terminal/native boundary; scripts/night-test-boundaries.sh"]
fn mcp_seat_reads_page_real_capture_and_refuse_stale_view_cursors() {
    use base64::Engine;
    let f = Fixture::real();
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

// keeps: spec mcp-control-plane
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
            message.envelope().expect("signed fixture envelope").seat == f.peer.id
                && matches!(
                    message.envelope().expect("signed fixture envelope").body,
                    bbt_mux::messaging::wire::Body::Presence { .. }
                )
        })
        .collect();
    assert_eq!(notices.len(), 2);
    for notice in notices {
        notice
            .envelope()
            .expect("signed fixture envelope")
            .verify_signature()
            .unwrap();
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
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let value = bridge
            .handle(
                &serde_json::to_vec(
                    &json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}),
                )
                .unwrap(),
            )
            .unwrap();
        if value["result"]["structuredContent"]["error"]["code"] != "workspace_pending"
            && value["result"]["structuredContent"]["tasks"]["availability"] != "workspace_pending"
        {
            return value;
        }
        assert!(
            Instant::now() < deadline,
            "workspace never settled: {value}"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
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

// keeps: spec mcp-control-plane
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
    assert_eq!(tools.len(), 23);
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
    assert_eq!(data["unread"], json!({"count":0,"basis":"no_cursor_yet"}));
    assert_eq!(data["pending_ack"]["count"], 0);
    assert_eq!(data["pending_ack"]["ids"], json!([]));
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

// keeps: spec mcp-control-plane
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

// keeps: spec mcp-control-plane
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

// keeps: spec mcp-control-plane
#[test]
fn running_mcp_bridge_survives_logic_restart_with_the_same_generation_admission() {
    let mut fixture = Fixture::new();
    let seat = fixture.own.id;
    let old = fixture
        .runtime
        .block_on(fixture.client().endpoint(seat))
        .unwrap();
    let mut bridge = fixture.bridge(seat);
    initialize(&mut bridge);
    let call = |bridge: &mut Bridge, id| {
        rpc(
            bridge,
            id,
            "tools/call",
            json!({"name":"orient","arguments":{}}),
        )
    };
    assert!(call(&mut bridge, 2).get("error").is_none());
    fixture.restart();
    let after = call(&mut bridge, 3);
    assert!(
        after.get("error").is_none(),
        "surviving MCP bridge: {after}"
    );
    assert_eq!(after["result"]["isError"], false, "{after}");
    let current = fixture
        .runtime
        .block_on(fixture.client().endpoint(seat))
        .unwrap();
    assert_eq!(current.path, old.path);
    assert_eq!(current.generation, old.generation);
    assert!(
        current.capability == old.capability,
        "same generation admission changed"
    );

    for (claimed_seat, generation, capability) in [
        (fixture.peer.id, old.generation, old.capability),
        (seat, old.generation + 1, old.capability),
        (seat, old.generation, [0; 32]),
    ] {
        let mut foreign = Bridge::new(BridgeConfig {
            socket: old.path.clone(),
            seat: claimed_seat,
            generation,
            capability,
        });
        initialize(&mut foreign);
        assert!(call(&mut foreign, 4).get("error").is_some());
    }
}

// keeps: spec mcp-control-plane
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
                .set_read_timeout(Some(Duration::from_secs(120)))
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

// keeps: decision #266
#[test]
fn mcp_orient_unread_has_a_basis_and_acknowledgement_remains_separate() {
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
    assert_eq!(
        first["structuredContent"]["unread"],
        json!({"count":1,"basis":"no_cursor_yet"})
    );
    assert_eq!(first["structuredContent"]["pending_ack"]["count"], 1);
    assert_eq!(
        first["structuredContent"]["pending_ack"]["ids"],
        json!([message])
    );
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
    assert_eq!(
        second["structuredContent"]["unread"],
        json!({"count":1,"basis":"no_cursor_yet"})
    );
    assert_eq!(second["structuredContent"]["pending_ack"]["count"], 0);
    assert_eq!(second["structuredContent"]["pending_ack"]["ids"], json!([]));
}

// keeps: spec mcp-control-plane
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
    assert_eq!(
        unavailable["error"]["data"]["transport"]["phase"],
        "connect"
    );
    assert_eq!(
        unavailable["error"]["data"]["transport"]["kind"],
        "NotFound"
    );
    assert!(unavailable["error"]["data"]["transport"]["elapsed_ms"].is_u64());
    assert!(!unavailable.to_string().contains("/does/not/exist"));
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

/// Real installed clients under a private HOME need a test credential; unproven.
/// This driver never inherits the operator's client account or copies a credential.
// keeps: spec mcp-control-plane
#[test]
#[ignore = "real installed clients need a private test credential; unproven; never inherit operator HOME"]
fn mcp_installed_claude_and_codex_call_orient() {
    use std::process::{Command, Stdio};
    let private_home = tempfile::Builder::new()
        .prefix("bbt-mcp-private-home-")
        .tempdir()
        .unwrap();
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
    let before = configuration_digests(private_home.path());
    let prompt = "This is a synthetic MCP interop fixture. Call the bboxy orient tool exactly once, then say the role name and class it returns. Use no other tools, read no files, and change no settings.";
    for (program, seat) in [("claude", fixture.own.id), ("codex", fixture.peer.id)] {
        let endpoint = fixture
            .runtime
            .block_on(fixture.client().endpoint(seat))
            .unwrap();
        let mut command = Command::new(program);
        command
            .env_clear()
            .env("HOME", private_home.path())
            .env("CLAUDE_CONFIG_DIR", private_home.path().join(".claude"))
            .env("CODEX_HOME", private_home.path().join(".codex"))
            .env("PATH", std::env::var_os("PATH").unwrap_or_default())
            .current_dir(&cwd)
            .stdin(Stdio::null());
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
        let (status, out, err) = bounded_client(command, Duration::from_secs(120), &private_home);
        let diagnostics = String::from_utf8_lossy(&err).to_ascii_lowercase();
        assert!(
            status.success(),
            "real installed clients under a private HOME need a test credential; unproven: {program} exit={status}; stdout_bytes={} stderr_bytes={} trust_text={} config_text={}",
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
        configuration_digests(private_home.path()),
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
    private_home: &tempfile::TempDir,
) -> (std::process::ExitStatus, Vec<u8>, Vec<u8>) {
    use std::os::unix::process::CommandExt;
    use std::process::Stdio;
    use tokio::io::AsyncReadExt;
    require_private_client_home(&command, private_home).expect(
        "NEVER BLOCK SWARM: refuse inherited HOME; use only a throwaway HOME created by this test",
    );
    command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0);
    tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
        let mut child = tokio::process::Command::from(command).kill_on_drop(true).spawn().expect("start private client/fixture");
        let pid = child.id().unwrap();
        eprintln!("owned private child PID {pid}, process group {pid}");
        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();
        let out = tokio::spawn(async move {
            let mut bytes = Vec::new();
            stdout.take(1024 * 1024).read_to_end(&mut bytes).await.unwrap();
            bytes
        });
        let err = tokio::spawn(async move {
            let mut bytes = Vec::new();
            stderr.take(256 * 1024).read_to_end(&mut bytes).await.unwrap();
            bytes
        });
        let status = tokio::time::timeout(timeout, child.wait()).await;
        // Only the new disposable child's own group. End a pipe-holding helper
        // before waiting for its stdout/stderr readers, on both result paths.
        // SAFETY: process_group(0) created this owned group immediately above.
        unsafe { libc::kill(-(pid as i32), libc::SIGKILL); }
        match status {
            Ok(status) => (status.unwrap(), out.await.unwrap(), err.await.unwrap()),
            Err(_) => {
                let status = child.wait().await.unwrap();
                let out = out.await.unwrap();
                let err = err.await.unwrap();
                panic!("client hang guard, status={status}, stdout_bytes={}, stderr_bytes={}; raw outputs discarded", out.len(), err.len());
            }
        }
    })
}

fn require_private_client_home(
    command: &std::process::Command,
    private_home: &tempfile::TempDir,
) -> std::io::Result<()> {
    let explicit_home = command
        .get_envs()
        .find(|(name, _)| *name == std::ffi::OsStr::new("HOME"))
        .and_then(|(_, value)| value);
    if explicit_home != Some(private_home.path().as_os_str()) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "inherited or foreign HOME is forbidden for installed-client interop",
        ));
    }
    Ok(())
}

// keeps: invariant never-block-swarm, Oracle b4760d4a requires a pre-spawn private-HOME guard.
#[test]
fn installed_client_guard_refuses_inherited_or_foreign_home_before_spawn() {
    let private_home = tempfile::tempdir().unwrap();
    let foreign_home = tempfile::tempdir().unwrap();
    let mut command = std::process::Command::new("never-executed-client-fixture");
    for home in [None, Some(foreign_home.path())] {
        if let Some(home) = home {
            command.env("HOME", home);
        }
        assert_eq!(
            require_private_client_home(&command, &private_home)
                .unwrap_err()
                .kind(),
            std::io::ErrorKind::PermissionDenied
        );
    }
    command.env_clear().env("HOME", private_home.path());
    require_private_client_home(&command, &private_home).unwrap();
}

fn configuration_digests(
    home: &std::path::Path,
) -> std::collections::BTreeMap<PathBuf, Option<String>> {
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

// keeps: spec mcp-control-plane
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

// keeps: spec mcp-control-plane
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
    assert_eq!(worker["tools"].as_array().unwrap().len(), 23);
    assert_eq!(admin["tools"].as_array().unwrap().len(), 43);
    for tool in [
        "ch_link",
        "ch_pause",
        "ch_grant",
        "seat_input_text",
        "task_delegate",
        "task_import_context",
        "task_import",
        "task_reconcile",
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

// keeps: spec mcp-control-plane
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

// keeps: decision #266
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

// keeps: decision #266
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

// keeps: decision #266
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

// keeps: decision #266
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

// keeps: decision #266
#[test]
fn mcp_destinations_explain_rights_and_write_only_reads_without_expanding_grants() {
    let f = Fixture::new();
    f.prepare_operator();
    operator_dm(&f, f.peer.id, "operator fixture asks");
    data(f.mutate(
        f.peer.id,
        Ulid::new(),
        "message_send",
        json!({"destination":"operator/inbox","text":"operator-only fixture"}),
    ));
    for seat in [f.oracle.id, f.own.id] {
        let mut bridge = f.bridge(seat);
        initialize(&mut bridge);
        let orient = rpc(&mut bridge, 2, "tools/call", json!({"name":"orient"}));
        assert_eq!(orient["result"]["isError"], false, "{orient}");
        let rows = orient["result"]["structuredContent"]["destinations"]
            .as_array()
            .unwrap();
        let rights =
            |path: &str| rows.iter().find(|row| row["path"] == path).unwrap()["rights"].clone();
        assert_eq!(rights("operator/inbox"), json!(["write-only"]));
        assert_eq!(
            rights(&format!("seats/{seat}/inbox")),
            json!(["read", "post"])
        );
        assert!(!rows.iter().any(|row| {
            row["path"]
                .as_str()
                .unwrap()
                .contains(&f.outside.id.to_string())
        }));
        for path in ["operator/inbox", "operator"] {
            for (tool, args) in [
                ("ch_ls", json!({"path":path})),
                ("message_list", json!({"channel":path})),
            ] {
                let denied = rpc(
                    &mut bridge,
                    3,
                    "tools/call",
                    json!({"name":tool,"arguments":args}),
                );
                assert_eq!(denied["result"]["isError"], true, "{tool}: {denied}");
                assert!(
                    denied["result"]["structuredContent"]["error"]["message"]
                        .as_str()
                        .unwrap()
                        .contains("write-only; it is in your destinations"),
                    "{denied}"
                );
            }
        }
    }
    assert!(
        operator_inbox(&f)
            .iter()
            .any(|text| text.contains("operator-only fixture"))
    );
    let mut worker = f.bridge(f.own.id);
    initialize(&mut worker);
    let peer = format!("seats/{}/inbox", f.peer.id);
    let first = rpc(&mut worker, 4, "tools/call", json!({"name":"orient"}));
    let rows = first["result"]["structuredContent"]["destinations"]
        .as_array()
        .unwrap();
    assert_eq!(
        rows.iter().find(|row| row["path"] == peer).unwrap()["rights"],
        json!(["write-only"])
    );
    let denied = rpc(
        &mut worker,
        5,
        "tools/call",
        json!({"name":"ch_ls","arguments":{"path":peer}}),
    );
    assert_eq!(denied["result"]["isError"], true);
    assert!(
        denied["result"]["structuredContent"]["error"]["message"]
            .as_str()
            .unwrap()
            .contains("write-only; it is in your destinations")
    );
    data(f.mutate(
        f.oracle.id,
        Ulid::new(),
        "ch_grant",
        json!({"path":peer,"seat":f.own.id,"rights":[]}),
    ));
    let changed = rpc(&mut worker, 6, "tools/call", json!({"name":"orient"}));
    assert_eq!(changed["result"]["isError"], false);
    assert!(
        !changed["result"]["structuredContent"]["destinations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["path"] == peer)
    );
    let send = rpc(
        &mut worker,
        7,
        "tools/call",
        json!({"name":"message_send","arguments":{"destination":peer,"text":"refused after revocation"}}),
    );
    assert_eq!(send["result"]["isError"], true);
    let page = data(f.call(f.peer.id, Ulid::new(), "message_list", json!({})));
    assert!(!texts(&page).contains("refused after revocation"));
    // The hint must not advertise a posting grant that was explicitly revoked.
    data(f.mutate(
        f.oracle.id,
        Ulid::new(),
        "ch_grant",
        json!({"path":"operator","seat":f.own.id,"rights":[]}),
    ));
    let changed = rpc(&mut worker, 8, "tools/call", json!({"name":"orient"}));
    assert_eq!(changed["result"]["isError"], false);
    assert!(
        changed["result"]["structuredContent"]["destinations"]
            .as_array()
            .unwrap()
            .iter()
            .all(|row| row["path"] != "operator/inbox")
    );
    let denied = rpc(
        &mut worker,
        9,
        "tools/call",
        json!({"name":"ch_ls","arguments":{"path":"operator/inbox"}}),
    );
    assert_eq!(denied["result"]["isError"], true);
    assert!(
        denied["result"]["structuredContent"]["error"]["message"]
            .as_str()
            .unwrap()
            .contains("no posting grant")
    );
}

// keeps: decision #266
#[test]
fn mcp_orient_cold_start_names_identity_allowed_tools_tasks_counts_and_actions() {
    let f = Fixture::new();
    for record in [&f.own, &f.peer, &f.oracle] {
        let mut bridge = f.bridge(record.id);
        initialize(&mut bridge);
        let catalogue = rpc(&mut bridge, 2, "tools/list", json!({}));
        let response = rpc(&mut bridge, 3, "tools/call", json!({"name":"orient"}));
        assert_eq!(response["result"]["isError"], false, "{response}");
        let value = &response["result"]["structuredContent"];
        assert_eq!(value["actor"]["seat_id"], json!(record.id));
        assert_eq!(value["actor"]["swarm_id"], json!(record.swarm));
        assert_eq!(value["actor"]["process_generation"], record.generation);
        assert_eq!(value["actor"]["role"], json!(record.role));
        assert_eq!(value["seat"]["record"]["name"], record.name);
        assert_eq!(
            value["scope"],
            json!({"seat":record.id,"role":record.role.name,"class":record.role.class})
        );
        let expected = catalogue["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .map(|tool| json!({"name":tool["name"],"description":tool["description"]}))
            .collect::<Vec<_>>();
        assert_eq!(value["tools"], json!(expected));
        let tools = value["tools"].as_array().unwrap();
        assert_eq!(
            tools.iter().any(|tool| tool["name"] == "ch_grant"),
            record.role.class == Class::Oracle
        );
        for tool in tools {
            let line = tool["description"].as_str().unwrap();
            assert!(!line.is_empty() && !line.contains('\n'));
        }
        assert_eq!(value["tasks"]["scope"], value["scope"]);
        if record.id == f.own.id {
            assert_eq!(value["tasks"]["items"][0]["id"], json!(f.task));
            assert!(value["tasks"].get("empty_reason").is_none());
        } else {
            assert_eq!(value["tasks"]["items"], json!([]));
            assert_eq!(value["tasks"]["empty_reason"], "nothing_assigned");
            assert!(!value["tasks"]["empty_message"].as_str().unwrap().is_empty());
        }
        assert_eq!(value["execution"]["scope"], value["scope"]);
        assert_eq!(value["channels"]["scope"], value["scope"]);
        assert_eq!(value["unread"], json!({"count":0,"basis":"no_cursor_yet"}));
        assert_eq!(value["pending_ack"]["count"], 0);
        assert_eq!(value["pending_ack"]["ids"], json!([]));
        assert_eq!(
            value["pending_ack"]["action"],
            "call message_ack for each id listed"
        );
        assert_eq!(
            value["operator"],
            "to reach the operator, reply with reply_to, or send to operator/inbox"
        );
        assert_eq!(
            value["op_id_guidance"],
            "ids are minted for you unless you retry"
        );
        assert!(value.get("unread_count").is_none());
        assert!(value.get("pending_application_ack").is_none());
    }
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

#[test]
fn task_migration_reconciliation_preserves_acceptance_and_refuses_workers() {
    let f = Fixture::new();
    let mut metadata = f.task_metadata();
    metadata["acceptance"] = json!(["actual evidence"]);
    f.task_file(metadata);
    let args = json!({"task_path":f.task_path(),"disposition":"implemented","reason":"Historical audited implementation",
        "evidence":["source.rs:12 — retained mechanism"]});
    let unauthorized = f.call(f.own.id, Ulid::new(), "task_reconcile", args.clone());
    assert_eq!(unauthorized["isError"], true);
    assert_eq!(f.task_document().metadata["status"], "running");
    let inbox = data(f.call(f.own.id, Ulid::new(), "message_list", json!({})));
    data(f.mutate(f.oracle.id, Ulid::new(), "task_reconcile", args.clone()));
    let doc = f.task_document();
    assert_eq!(doc.metadata["status"], "done");
    assert_eq!(doc.metadata["reconciliation"][0]["fresh_accepted"], false);
    assert!(doc.metadata.get("verify_result").is_none());
    assert_eq!(
        inbox["page"],
        data(f.call(f.own.id, Ulid::new(), "message_list", json!({})))["page"]
    );
    assert_eq!(
        f.mutate(
            f.oracle.id,
            Ulid::new(),
            "task_accept",
            json!({"task_path":f.task_path()})
        )["isError"],
        true
    );

    let mut metadata = f.task_metadata();
    metadata["acceptance"] = json!(["actual evidence"]);
    f.task_file(metadata);
    for result in [
        verification("wrong", 0, json!([])),
        verification("true", 1, json!([])),
        verification("true", 0, json!([])),
    ] {
        data(f.report(result));
        assert_eq!(
            f.mutate(
                f.oracle.id,
                Ulid::new(),
                "task_accept",
                json!({"task_path":f.task_path()})
            )["isError"],
            true
        );
    }
    data(f.report(verification(
        "true",
        0,
        json!([{"criterion":"actual evidence","reference":"test output: exit 0"}]),
    )));
    data(f.mutate(
        f.oracle.id,
        Ulid::new(),
        "task_accept",
        json!({"task_path":f.task_path()}),
    ));
    assert_eq!(f.task_document().metadata["status"], "accepted");
    assert_eq!(
        f.mutate(f.oracle.id, Ulid::new(), "task_reconcile", args)["isError"],
        true
    );
}

fn migration_files(root: &std::path::Path) -> std::collections::BTreeMap<String, String> {
    fn visit(
        root: &std::path::Path,
        path: &std::path::Path,
        out: &mut std::collections::BTreeMap<String, String>,
    ) {
        if !path.exists() {
            return;
        }
        for entry in std::fs::read_dir(path).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_dir() {
                visit(root, &entry.path(), out);
            } else {
                out.insert(
                    entry
                        .path()
                        .strip_prefix(root)
                        .unwrap()
                        .to_string_lossy()
                        .into_owned(),
                    bbt_mux::store::digest(&std::fs::read(entry.path()).unwrap()),
                );
            }
        }
    }
    let mut out = std::collections::BTreeMap::new();
    visit(root, root, &mut out);
    out
}

// A provisional range may legitimately lose to ordinary work. Only these
// explicitly no-effect results permit a fresh operation; unknown never does.
fn migration_import(f: &Fixture, phase: &str, args: &Value) -> (Ulid, Value, usize) {
    for attempt in 0..8 {
        let op = Ulid::new();
        let result = f.mutate_at(phase, f.oracle.id, op, "task_import", args.clone());
        let code = result["structuredContent"]["error"]["code"].as_str();
        if matches!(
            code,
            Some("import_allocation_stale" | "index_busy" | "task_import_busy")
        ) {
            if code == Some("import_allocation_stale") {
                assert!(
                    !PathBuf::from(args["expected_root"].as_str().unwrap())
                        .join("imports")
                        .join(args["bundle_sha256"].as_str().unwrap())
                        .exists(),
                    "stale allocation exposed an active import"
                );
            }
            eprintln!(
                "private import known no-effect attempt {}: {}",
                attempt + 1,
                code.unwrap()
            );
            continue;
        }
        return (op, data(result), attempt);
    }
    panic!(
        "private importer did not obtain a stable provisional range in eight explicit no-effect operations"
    );
}

// Adapted from this file's real seat-socket bridge fixtures. The relay changes only
// whether the completed reply is delivered; it never interprets or retries an effect.
// https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
fn migration_transport_controls(f: &Fixture, args: &Value) -> Vec<Value> {
    let mut evidence = Vec::new();
    for lose_reply in [false, true] {
        let info = f
            .runtime
            .block_on(f.client().endpoint(f.oracle.id))
            .unwrap();
        let proxy = f
            ._temp
            .path()
            .join(format!("import-relay-{lose_reply}.sock"));
        let listener = std::os::unix::net::UnixListener::bind(&proxy).unwrap();
        let thread = std::thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(10);
            loop {
                use std::os::fd::AsRawFd;
                let mut readiness = libc::pollfd {
                    fd: listener.as_raw_fd(),
                    events: libc::POLLIN,
                    revents: 0,
                };
                let remaining = deadline
                    .saturating_duration_since(Instant::now())
                    .as_millis();
                assert!(
                    remaining > 0,
                    "import relay did not receive its completed operation"
                );
                // SAFETY: this thread owns the live listener and the one-element poll array.
                assert_eq!(
                    unsafe { libc::poll(&mut readiness, 1, remaining.try_into().unwrap()) },
                    1
                );
                let (mut caller, _) = listener.accept().unwrap();
                caller
                    .set_read_timeout(Some(Duration::from_secs(10)))
                    .unwrap();
                caller
                    .set_write_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                let request = read_frame(&mut std::io::BufReader::new(caller.try_clone().unwrap()))
                    .unwrap()
                    .unwrap();
                let mut upstream = UnixStream::connect(&info.path).unwrap();
                upstream
                    .set_read_timeout(Some(Duration::from_secs(10)))
                    .unwrap();
                upstream
                    .set_write_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                upstream.write_all(&request).unwrap();
                upstream.write_all(b"\n").unwrap();
                let reply = read_frame(&mut std::io::BufReader::new(upstream))
                    .unwrap()
                    .unwrap();
                let actual = serde_json::from_slice::<Value>(&reply).unwrap();
                let opening = actual["structuredContent"]["error"]["code"] == "workspace_pending";
                // Preserve the existing RPC helper's explicit pre-effect readiness
                // replies unchanged. Only a completed operation can lose its receipt.
                if !lose_reply || opening {
                    caller.write_all(&reply).unwrap();
                    caller.write_all(b"\n").unwrap();
                }
                if opening {
                    continue;
                }
                // Both owned sockets close before the thread joins. Payload/capability
                // stays in memory; only the actual tool result returns to the test.
                return actual;
            }
        });
        let admission = f
            .runtime
            .block_on(f.client().endpoint(f.oracle.id))
            .unwrap();
        let mut bridge = Bridge::new(BridgeConfig {
            socket: proxy,
            seat: f.oracle.id,
            generation: admission.generation,
            capability: admission.capability,
        });
        initialize(&mut bridge);
        let op = Ulid::new();
        let mut arguments = args.clone();
        arguments["op_id"] = json!(op);
        let started = Instant::now();
        let observed = rpc(
            &mut bridge,
            2,
            "tools/call",
            json!({"name":"task_import","arguments":arguments}),
        );
        let actual = thread.join().unwrap();
        let retained = f.original_operation(f.oracle.id, op);
        let row = json!({"phase":if lose_reply{"completed_reply_lost"}else{"completed_reply_delivered"},
            "op_id":op,"elapsed_ms":started.elapsed().as_millis(),"observed":observed,
            "actual":actual,"read_only_original":retained,"unknown_retried":false});
        eprintln!("import transport control: {row}");
        assert_eq!(data(actual.clone())["status"], "imported");
        assert_eq!(data(actual.clone())["source_records"], 250);
        assert_eq!(retained["tool"], "task_import");
        assert_eq!(retained["status"], "imported");
        assert_eq!(retained["source_records"], 250);
        if lose_reply {
            assert_eq!(observed["error"]["code"], -32001);
            assert_eq!(observed["error"]["data"]["op_id"], op.to_string());
            assert_eq!(observed["error"]["data"]["status"], "unknown");
            assert_eq!(
                observed["error"]["data"]["transport"]["phase"],
                "response_eof"
            );
            assert_eq!(observed["error"]["data"]["transport"]["kind"], "Other");
        } else {
            assert_eq!(observed["result"], actual);
        }
        evidence.push(row);
    }
    evidence
}

#[test]
fn task_migration_250_records_authenticated_collision_restart_and_replay() {
    let private = tempfile::tempdir().unwrap();
    let input = std::env::var_os("BBOXY_TASK_MIGRATION_INPUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let script = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../scripts/test-task-migration.py");
            assert!(
                std::process::Command::new("python3")
                    .arg(script)
                    .arg("prepare")
                    .arg(private.path())
                    .status()
                    .unwrap()
                    .success()
            );
            private.path().join("task-migration/input")
        });
    let mut f = Fixture::new();
    let mut metadata = f.task_metadata();
    metadata["number"] = json!(171);
    f.task_file(metadata);
    let root = f.config.repository.join(".bboxy").canonicalize().unwrap();
    let context = data(f.call(f.oracle.id, Ulid::new(), "task_import_context", json!({})));
    assert_eq!(context["expected_root"], json!(root));
    assert_eq!(
        f.call(f.own.id, Ulid::new(), "task_import_context", json!({}))["isError"],
        true
    );
    assert_eq!(
        f.call(
            f.oracle.id,
            Ulid::new(),
            "task_import_context",
            json!({"expected_root":"/another/root"})
        )["isError"],
        true
    );
    std::fs::write(
        root.join("collision999.md"),
        format!(
            "---\nid: {}\nnumber: 999\ntype: note\n---\nUnrelated collision control.\n",
            Ulid::new()
        ),
    )
    .unwrap();
    let task171_path = format!("tasks/{}.md", f.task);
    let task171_before = std::fs::read(root.join(&task171_path)).unwrap();
    let unrelated = root.join(format!("tasks/{}.md", f.foreign_task));
    let unrelated_before = std::fs::read(&unrelated).unwrap();
    let manifest = input.join("lossless-import-manifest.json");
    let audit = input.join("checked-audit.json");
    let sources = input.join("dependency-source");
    let digest = bbt_mux::index::migration::bundle_digest(&manifest, &audit, &sources).unwrap();
    let args = json!({"expected_root":context["expected_root"],"manifest_path":manifest,"audit_path":audit,"source_dir":sources,"bundle_sha256":digest,"owner_seat":f.oracle.id});
    let mut denied = args.clone();
    denied["expected_root"] = json!(private.path());
    assert_eq!(
        f.mutate_at(
            "wrong_root",
            f.oracle.id,
            Ulid::new(),
            "task_import",
            denied
        )["isError"],
        true
    );
    assert!(!root.join("imports").exists());
    assert_eq!(
        f.call(f.own.id, Ulid::new(), "task_import", args.clone())["isError"],
        true
    );
    let mut denied = args.clone();
    denied["owner_seat"] = json!(f.own.id);
    assert_eq!(
        f.mutate_at(
            "worker_custodian",
            f.oracle.id,
            Ulid::new(),
            "task_import",
            denied
        )["isError"],
        true
    );
    let mut denied = args.clone();
    denied["bundle_sha256"] = json!("0".repeat(64));
    assert_eq!(
        f.mutate_at(
            "wrong_digest",
            f.oracle.id,
            Ulid::new(),
            "task_import",
            denied
        )["isError"],
        true
    );
    let inboxes: Vec<_> = [&f.own, &f.peer, &f.oracle, &f.outside]
        .iter()
        .map(|s| data(f.call(s.id, Ulid::new(), "message_list", json!({})))["page"].clone())
        .collect();
    let (op, first, no_effect_attempts) = migration_import(&f, "first_publication", &args);
    assert_eq!(first["source_records"], 250);
    assert_eq!(first["published_records"], 251);
    let published = migration_files(&root.join("imports"));
    let mut changed = args.clone();
    changed["expected_root"] = json!(format!("{}/.", root.display()));
    assert_eq!(
        f.mutate_at("changed_arguments", f.oracle.id, op, "task_import", changed)["structuredContent"]
            ["error"]["code"],
        "op_id_reused"
    );
    data(f.mutate_at(
        "same_operation_replay",
        f.oracle.id,
        op,
        "task_import",
        args.clone(),
    ));
    assert_eq!(published, migration_files(&root.join("imports")));
    let transport_controls = migration_transport_controls(&f, &args);
    assert_eq!(published, migration_files(&root.join("imports")));
    f.restart();
    let (_, _, restart_no_effect_attempts) = migration_import(&f, "restart_bundle_replay", &args);
    f.prepare_operator(); // This fixture's existing memory-only custody; no real account or prompt.
    let bbt_proto::GuiReply::Task {
        response: context_response,
        ..
    } = f.gui_task(bbt_proto::messaging::TaskRequest::ImportContext {})
    else {
        panic!("operator import context unavailable");
    };
    assert!(
        matches!(context_response,bbt_proto::messaging::TaskResponse::ImportContext{expected_root} if expected_root==root.to_string_lossy())
    );
    let mut gui_args = args;
    gui_args["op"] = json!("import");
    let request = serde_json::from_value(gui_args).unwrap();
    let bbt_proto::GuiReply::Task { response, .. } = f.gui_task(request) else {
        panic!("operator import replay unavailable");
    };
    assert!(matches!(
        response,
        bbt_proto::messaging::TaskResponse::Imported {
            source_records: 250,
            published_records: 251,
            ..
        }
    ));
    assert_eq!(published, migration_files(&root.join("imports")));
    for (seat, before) in [&f.own, &f.peer, &f.oracle, &f.outside].iter().zip(inboxes) {
        assert_eq!(
            before,
            data(f.call(seat.id, Ulid::new(), "message_list", json!({})))["page"]
        );
    }
    assert_eq!(
        task171_before,
        std::fs::read(root.join(&task171_path)).unwrap()
    );
    assert_eq!(unrelated_before, std::fs::read(&unrelated).unwrap());
    // Task reads consume the published root, not a migration-only result packet.
    let mut total = Vec::new();
    let mut after = Value::Null;
    loop {
        let page = data(f.call(
            f.oracle.id,
            Ulid::new(),
            "task_list",
            json!({"limit":128,"after":after}),
        ));
        total.extend(page["items"].as_array().unwrap().iter().cloned());
        after = page["next"].clone();
        if after.is_null() {
            break;
        }
    }
    assert_eq!(
        total
            .iter()
            .filter(|d| d["metadata"]["status"] == "open")
            .count(),
        7
    );
    if let Some(output) = std::env::var_os("BBOXY_TASK_MIGRATION_OUTPUT").map(PathBuf::from) {
        fn copy(source: &std::path::Path, dest: &std::path::Path) {
            std::fs::create_dir_all(dest).unwrap();
            for entry in std::fs::read_dir(source).unwrap() {
                let entry = entry.unwrap();
                let target = dest.join(entry.file_name());
                if entry.file_type().unwrap().is_dir() {
                    copy(&entry.path(), &target);
                } else {
                    std::fs::copy(entry.path(), target).unwrap();
                }
            }
        }
        let dest = output.join("published");
        std::fs::create_dir_all(dest.join("tasks")).unwrap();
        copy(&root.join("imports"), &dest.join("imports"));
        std::fs::copy(root.join(&task171_path), dest.join(&task171_path)).unwrap();
        let proof = json!({"task171_path":task171_path,"task171_before":bbt_mux::store::digest(&task171_before),"task171_after":bbt_mux::store::digest(&std::fs::read(root.join(&task171_path)).unwrap()),
            "unrelated_before":bbt_mux::store::digest(&unrelated_before),"unrelated_after":bbt_mux::store::digest(&std::fs::read(unrelated).unwrap()),"historical_wakes":0,"replay_unchanged":true,"known_no_effect_attempts":no_effect_attempts+restart_no_effect_attempts,"transport_controls":transport_controls});
        std::fs::write(
            output.join("proof.json"),
            serde_json::to_vec_pretty(&proof).unwrap(),
        )
        .unwrap();
    }
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

// keeps: spec mcp-control-plane
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

// keeps: decision #28
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

// keeps: spec mcp-control-plane
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

fn assert_claim_number_matches_actual_root(
    repository: &std::path::Path,
    claimed: &bbt_mux::index::Document,
) {
    fn records(path: &std::path::Path, out: &mut Vec<bbt_mux::index::Document>) {
        for entry in std::fs::read_dir(path).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                records(&path, out);
            } else if path.extension().is_some_and(|extension| extension == "md") {
                out.push(bbt_mux::index::Document::parse(
                    path.to_str().unwrap(),
                    &std::fs::read_to_string(&path).unwrap(),
                ));
            }
        }
    }
    let mut docs = Vec::new();
    records(&repository.join(".bboxy"), &mut docs);
    let number = claimed.metadata["number"]
        .as_u64()
        .filter(|number| *number > 0)
        .expect("positive claim number");
    let mut numbers = std::collections::BTreeSet::new();
    for doc in &docs {
        assert!(
            numbers.insert(
                doc.metadata["number"]
                    .as_u64()
                    .filter(|number| *number > 0)
                    .expect("positive actual root record number")
            ),
            "duplicate record number in {}",
            doc.path
        );
    }
    if number > 3 {
        assert!(
            docs.iter().any(|doc| doc.id != claimed.id
                && doc.metadata["type"] == "artifact"
                && doc.metadata["observed"] == true
                && doc.metadata["last_observation_sequence"].as_str().is_some()
                && doc.metadata["number"]
                    .as_u64()
                    .is_some_and(|prior| prior < number)),
            "extra preceding record was not an actual provenance observation"
        );
    }
}

// keeps: spec mcp-control-plane
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
    assert_claim_number_matches_actual_root(&f.config.repository, &artifact);
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

// keeps: spec mcp-control-plane
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

// keeps: spec mcp-control-plane
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
    let decision = knowledge_create(
        &f,
        f.oracle.id,
        "decision",
        "decisions/writer-corpus.md",
        "A decision created through the seat's knowledge tool.\n",
    );
    assert_eq!(
        decision["path"],
        format!(
            "decisions/d{}-writer-corpus.md",
            decision["number"].as_u64().unwrap()
        )
    );
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

// keeps: spec mcp-control-plane
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
    assert_claim_number_matches_actual_root(
        &f.config.repository,
        &bbt_mux::index::Document::parse("artifact.md", &written),
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

// keeps: spec mcp-control-plane
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
    let call = Call {
        op_id: Ulid::new(),
        tool: "task_report".into(),
        args: json!({"task_path":f.task_path(),"summary":"must not write"}),
    };
    let deadline = Instant::now() + Duration::from_secs(10);
    let result = loop {
        let result = f
            .runtime
            .block_on(f.client().control_call(f.own.id, 1, call.clone()));
        if !matches!(&result,Ok(value) if value["structuredContent"]["error"]["code"]=="workspace_pending")
        {
            break result;
        }
        assert!(
            Instant::now() < deadline,
            "workspace never settled before audit fault admission"
        );
        std::thread::sleep(Duration::from_millis(10));
    };
    assert!(result.is_err());
    assert_eq!(f.task_document().digest, original.digest);
    db.execute_batch("DROP TRIGGER fixture_work_audit_failure")
        .unwrap();
}

// keeps: spec mcp-control-plane
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
            f.connection(),
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

// keeps: spec mcp-control-plane
#[test]
fn mcp_oracle_controls_remain_seat_bound_through_the_ctl_alias() {
    let f = Fixture::new();
    let layout = data(f.mutate(
        f.oracle.id,
        Ulid::new(),
        "ctl",
        json!({"command":"layout_read","arguments":{}}),
    ));
    assert_eq!(layout["ui_attached"], false);
    let ids: Vec<_> = layout["seats"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["seat_id"].clone())
        .collect();
    assert_eq!(ids.len(), 3);
    assert!(!ids.contains(&json!(f.outside.id)));
    for (caller, tool, args, reason) in [
        (
            f.own.id,
            "ctl",
            json!({"command":"seat_list","arguments":{}}),
            "oracle-class",
        ),
        (
            f.oracle.id,
            "ctl",
            json!({"command":"app_stop","arguments":{}}),
            "unsupported ctl",
        ),
        (
            f.oracle.id,
            "ctl",
            json!({"command":"seat_focus","arguments":f.target(f.outside.id)}),
            "outside caller swarm",
        ),
        (
            f.oracle.id,
            "seat_focus",
            json!({"seat_id":f.peer.id,"expected_process_generation":"2"}),
            "stale target",
        ),
    ] {
        refused(f.mutate(caller, Ulid::new(), tool, args), reason);
    }
    let no_window = f.mutate(f.oracle.id, Ulid::new(), "seat_focus", f.target(f.peer.id));
    assert_eq!(
        no_window["structuredContent"]["error"]["code"],
        "unavailable"
    );
    // Architect is an Oracle-class preset; the authenticated class, not the
    // role string provided by a caller, determines control access.
    f.registry()
        .update(f.oracle.id, |r| {
            r.role.name = "architect".into();
            Ok(())
        })
        .unwrap();
    assert_eq!(
        f.mutate(
            f.oracle.id,
            Ulid::new(),
            "ctl",
            json!({"command":"layout_read","arguments":{}})
        )["isError"],
        false
    );
}

// keeps: spec mcp-control-plane
#[test]
#[ignore = "#327 B2: real terminal boundary; make test-oracle-control"]
fn mcp_oracle_split_runs_without_a_ui_and_replays_after_logic_restart() {
    isolated_seat_fixture("orchestrate");
}

fn seat_orchestration_fixture() {
    use bbt_proto::seat_state::{Placement, Side};
    let home = PathBuf::from(std::env::var_os("HOME").unwrap());
    std::fs::write(home.join(".zshrc"), "printf 'ORCHESTRATED\\n'\n").unwrap();
    let mut f = Fixture::real();
    let baseline = f.registry().list().unwrap().len();
    let first = data(f.mutate(f.oracle.id,Ulid::new(),"ctl",json!({"command":"seat_spawn","arguments":{"swarm_id":f.own.swarm,"approved_profile":"zsh","cwd":f._temp.path(),"name":"Oracle helper"}})));
    let anchor: Ulid = serde_json::from_value(first["seat_id"].clone()).unwrap();
    f.wait_screen(anchor, "ORCHESTRATED");
    let tab = match f.registry().get(anchor).unwrap().launch_placement.unwrap() {
        Placement::NewTab { tab } => tab,
        _ => panic!("expected tab"),
    };
    refused(f.mutate(f.oracle.id,Ulid::new(),"seat_spawn",json!({"swarm_id":f.own.swarm,"approved_profile":"zsh","cwd":f._temp.path(),"tab_id":Ulid::from(tab.0.0)})),"already occupied");
    let page = data(f.mutate(f.oracle.id, Ulid::new(), "layout_read", json!({"limit":1})));
    assert_eq!(page["seats"].as_array().unwrap().len(), 1);
    let next = data(f.mutate(
        f.oracle.id,
        Ulid::new(),
        "layout_read",
        json!({"after":page["next"],"limit":1}),
    ));
    assert_ne!(page["seats"][0]["seat_id"], next["seats"][0]["seat_id"]);

    let mut receipts = vec![];
    for (side, label) in [(Side::Right, "right"), (Side::Bottom, "bottom")] {
        let op = Ulid::new();
        let args = json!({"swarm_id":f.own.swarm,"approved_profile":"zsh","cwd":f._temp.path(),"seat_id":anchor,"expected_process_generation":"1","side":label,"tab_id":Ulid::from(tab.0.0)});
        let result = f.mutate(
            f.oracle.id,
            op,
            "ctl",
            json!({"command":"seat_split","arguments":args}),
        );
        let value = data(result.clone());
        let seat: Ulid = serde_json::from_value(value["seat_id"].clone()).unwrap();
        assert_eq!(seat.timestamp_ms(), op.timestamp_ms());
        assert_eq!(
            f.registry().get(seat).unwrap().launch_placement,
            Some(Placement::Split {
                tab,
                target: bbt_proto::SeatId::from_u128(anchor.0),
                target_generation: 1,
                side
            })
        );
        f.wait_screen(seat, "ORCHESTRATED");
        assert_eq!(
            f.mutate(f.oracle.id, op, "seat_split", args.clone()),
            result
        );
        receipts.push((op, args, result));
    }
    let mut invalid = receipts[0].1.clone();
    invalid["seat_id"] = json!(f.outside.id);
    refused(
        f.mutate(f.oracle.id, Ulid::new(), "seat_split", invalid),
        "outside caller swarm",
    );
    let mut invalid = receipts[0].1.clone();
    invalid["expected_process_generation"] = json!("2");
    refused(
        f.mutate(f.oracle.id, Ulid::new(), "seat_split", invalid),
        "stale target",
    );
    let mut invalid = receipts[0].1.clone();
    invalid["tab_id"] = json!(Ulid::new());
    refused(
        f.mutate(f.oracle.id, Ulid::new(), "seat_split", invalid),
        "tab changed",
    );
    assert_eq!(f.registry().list().unwrap().len(), baseline + 3);
    f.restart();
    for (op, args, result) in receipts {
        assert_eq!(f.mutate(f.oracle.id, op, "seat_split", args), result);
    }
    assert_eq!(f.registry().list().unwrap().len(), baseline + 3);
}

fn isolated_seat_fixture(mode: &str) {
    let temp = tempfile::tempdir().unwrap();
    let mut command = std::process::Command::new(std::env::current_exe().unwrap());
    command
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
        .env("BBOXY_MCP_SEAT_FIXTURE", mode);
    // Leave the child time to report its own 120-second guard and clean up.
    let (status, stdout, stderr) = bounded_client(command, Duration::from_secs(240), &temp);
    assert!(
        status.success(),
        "isolated {mode}: {status}\n{}\n{}",
        String::from_utf8_lossy(&stdout),
        String::from_utf8_lossy(&stderr)
    );
}

// keeps: spec mcp-control-plane
#[test]
#[ignore = "#327 B2/B4: real terminal/native boundary; scripts/night-test-boundaries.sh"]
fn mcp_seat_spawn_runs_once_with_isolated_home_and_existing_wiring() {
    isolated_seat_fixture("spawn");
}

// keeps: decision #262
#[test]
fn mcp_spawn_refuses_unknown_roles_before_admitting_a_seat() {
    let fixture = Fixture::new();
    let count = fixture.registry().list().unwrap().len();
    for (kind, role, reason) in [
        ("codex", "unknown", "unknown_role_preset"),
        ("pi", "missing", "unknown_role_preset"),
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

// keeps: spec mcp-control-plane
#[test]
#[ignore = "#327 B2/B4: real terminal/native boundary; scripts/night-test-boundaries.sh"]
fn mcp_seat_native_interrupt_is_bounded_audited_and_never_replayed() {
    isolated_seat_fixture("native");
}

// keeps: spec mcp-control-plane
#[test]
#[ignore = "subprocess fixture; parent supplies a disposable HOME and no credential environment"]
fn mcp_seat_isolated_process() {
    match std::env::var("BBOXY_MCP_SEAT_FIXTURE").unwrap().as_str() {
        "spawn" => seat_spawn_fixture(),
        "orchestrate" => seat_orchestration_fixture(),
        "native" => seat_native_fixture(),
        _ => panic!("unknown fixture mode"),
    }
}

fn seat_spawn_fixture() {
    let home = PathBuf::from(std::env::var_os("HOME").unwrap());
    std::fs::write(home.join(".zshrc"), "printf 'ISOLATED-SPAWN\\n'\n").unwrap();
    let mut f = Fixture::real();
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
    assert!(bbt_proto::seat_names::POOL.contains(&record.name.as_str()));
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
    assert_eq!(f.registry().get(seat).unwrap().name, record.name);
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
    let mut f = Fixture::real();
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
                    Duration::from_secs(120),
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
        tokio::time::timeout(Duration::from_secs(120), frames.recv())
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

    let path = std::env::temp_dir().join(format!("mcp-codex-{}.sock", Ulid::new()));
    let codex = f.spawn_native_fixture(
        "codex",
        "printf 'CODEX-READY\\n'; while IFS= read -r line; do :; done",
        Some(bbt_supervisor::protocol::CompanionInfo {
            pid: std::process::id(),
            ready_socket: path.clone(),
        }),
    );
    let thread = codex.harness_session_id.clone().unwrap();
    let hold = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let flag = hold.clone();
    let (server, mut frames, reader_ready) = f.runtime.block_on(async {
        let listener = tokio::net::UnixListener::bind(&path).unwrap();
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
        let ready = std::sync::Arc::new(std::sync::Mutex::new(Some(ready_tx)));
        let task = tokio::spawn(async move {
            let mut connections = tokio::task::JoinSet::new();
            loop {
                let (stream, _) = listener.accept().await.unwrap();
                let tx = tx.clone();
                let thread = thread.clone();
                let flag = flag.clone();
                let ready = ready.clone();
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
                                if let Some(ready) = ready.lock().unwrap().take() {
                                    let _ = ready.send(());
                                }
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
        (task, rx, ready_rx)
    });
    // Its first read-only request proves the Worker installed this reader.
    // Bind the test turn after that causal boundary, without a scheduler delay.
    f.runtime.block_on(async {
        tokio::time::timeout(Duration::from_secs(120), reader_ready)
            .await
            .expect("native reader did not reach its owned server")
            .unwrap();
    });
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
        tokio::time::timeout(Duration::from_secs(120), frames.recv())
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
                tokio::time::timeout(Duration::from_secs(120), frames.recv()).await,
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
    assert!(
        jobs.try_join_next().is_none(),
        "a held native call completed before control service"
    );
    let busy = f.call(actor, Ulid::new(), "seat_interrupt", args.clone());
    refused(busy, "capacity is full");
    assert_eq!(
        data(f.call(actor, Ulid::new(), "self_status", json!({})))["actor"]["seat_id"],
        json!(actor)
    );
    assert!(
        jobs.try_join_next().is_none(),
        "controls waited for held native calls to finish"
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
        tokio::time::timeout(Duration::from_secs(120), frames.recv())
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

// keeps: spec mcp-control-plane
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

// keeps: spec mcp-control-plane
#[test]
fn mcp_artifact_number_uses_unindexed_records_of_every_type_and_keeps_retry_bytes() {
    let mut f = Fixture::new();
    // The provenance observer legitimately numbers newly observed files. Keep
    // this allocator input in its excluded artifact subtree, so that observer
    // cannot allocate 9002 before this claim. Index allocation still scans every
    // nested Markdown record and type, including this BOM/CRLF spec.
    let nested = f.config.repository.join(".bboxy/artifacts/number-inputs");
    std::fs::create_dir_all(&nested).unwrap();
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

// keeps: spec mcp-control-plane
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

// keeps: spec mcp-control-plane
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

// keeps: spec mcp-control-plane
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
        "/tmp/forbidden.md",
        "../escape.md",
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
            "relative Markdown path",
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
        "task creation requires Oracle-class",
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
    assert_eq!(
        data(knowledge_fetch(&f, f.own.id, "#1"))["id"],
        json!(f.task)
    );
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
        "stale_reference",
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

// keeps: spec mcp-control-plane
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
        .map(|item| serde_json::to_value(&item.envelope().expect("signed fixture envelope").body).unwrap()["text"].to_string())
        .collect()
}
fn texts(page: &Value) -> String {
    page["page"]["items"].to_string()
}

// keeps: decision #264
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

// keeps: decision #264
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

// keeps: decision #264
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

// keeps: decision #264
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
fn knowledge_record_args(path: &str, kind: &str, body: &str) -> Value {
    json!({"path":path,"frontmatter":{"type":kind,"title":format!("{kind} knowledge fixture"),"custom":{"preserved":true}},"body":body})
}
fn knowledge_update_args(record: &Value) -> Value {
    let reference: bbt_mux::knowledge::data_source::RecordRef =
        serde_json::from_value(record.clone()).unwrap();
    json!({"ref":reference,"frontmatter":record["frontmatter"],"body":record["body"],"etag":record["etag"]})
}
fn knowledge_create(f: &Fixture, actor: Ulid, kind: &str, path: &str, body: &str) -> Value {
    data(f.mutate(
        actor,
        Ulid::new(),
        "knowledge_write",
        knowledge_record_args(path, kind, body),
    ))["record"]
        .clone()
}
fn knowledge_denied(value: Value, code: &str) {
    assert_eq!(value["isError"], true, "{value}");
    assert_eq!(value["structuredContent"]["error"]["code"], code, "{value}");
}
fn knowledge_search(f: &Fixture, actor: Ulid, args: Value) -> Value {
    let mut bridge = f.bridge(actor);
    initialize(&mut bridge);
    let response = rpc(
        &mut bridge,
        2,
        "tools/call",
        json!({"name":"knowledge_search","arguments":args}),
    );
    assert!(response.get("error").is_none(), "{response}");
    response["result"].clone()
}

// keeps: decision #266
#[test]
fn mcp_knowledge266_class_rows_use_actual_signers_and_cannot_relabel_protected_types() {
    let f = Fixture::new();
    for kind in ["e2e", "comment", "research", "note", "artifact"] {
        let path = format!("records/worker-{kind}.md");
        let record = knowledge_create(&f, f.own.id, kind, &path, "original body");
        let mut update = knowledge_update_args(&record);
        update["body"] = json!("updated body");
        let stored =
            data(f.mutate(f.own.id, Ulid::new(), "knowledge_write", update))["record"].clone();
        assert_eq!(stored["id"], record["id"]);
        assert_eq!(stored["number"], record["number"]);
        assert_eq!(stored["frontmatter"]["custom"]["preserved"], true);
        assert_eq!(
            stored["frontmatter"]["authorship"]["signer"],
            format!("seat:{}", f.own.id)
        );
        verify_knowledge_record(&stored);
        let doc = bbt_mux::index::Document::parse(
            &path,
            &std::fs::read_to_string(f.config.repository.join(".bboxy").join(&path)).unwrap(),
        );
        assert_item_signature(&doc, f.own.id);
    }
    let note = data(knowledge_fetch(&f, f.own.id, "records/worker-note.md"));
    for actor in [f.own.id, f.oracle.id] {
        for kind in ["research", "spec", "unlisted-kind"] {
            let mut relabel = knowledge_update_args(&note);
            relabel["frontmatter"]["type"] = json!(kind);
            knowledge_denied(
                f.mutate(actor, Ulid::new(), "knowledge_write", relabel),
                "type_immutable",
            );
        }
    }
    for kind in ["spec", "decision", "invariant", "proposal"] {
        let path = if kind == "decision" {
            "decisions/oracle-decision.md".to_owned()
        } else {
            format!("records/oracle-{kind}.md")
        };
        knowledge_denied(
            f.mutate(
                f.own.id,
                Ulid::new(),
                "knowledge_write",
                knowledge_record_args(&path, kind, "denied"),
            ),
            "knowledge_write_denied",
        );
        assert!(!f.config.repository.join(".bboxy").join(&path).exists());
        let record = knowledge_create(&f, f.oracle.id, kind, &path, "Oracle original");
        if kind == "decision" {
            assert_eq!(
                record["path"],
                format!(
                    "decisions/d{}-oracle-decision.md",
                    record["number"].as_u64().unwrap()
                )
            );
        }
        let file = f
            .config
            .repository
            .join(".bboxy")
            .join(record["path"].as_str().unwrap());
        let before = std::fs::read(&file).unwrap();
        let mut update = knowledge_update_args(&record);
        update["body"] = json!("worker must not edit");
        knowledge_denied(
            f.mutate(f.own.id, Ulid::new(), "knowledge_write", update.clone()),
            "knowledge_write_denied",
        );
        update["frontmatter"]["type"] = json!("note");
        knowledge_denied(
            f.mutate(f.own.id, Ulid::new(), "knowledge_write", update),
            "type_immutable",
        );
        assert_eq!(std::fs::read(&file).unwrap(), before);
        let mut update = knowledge_update_args(&record);
        update["body"] = json!("Oracle edit");
        let stored =
            data(f.mutate(f.oracle.id, Ulid::new(), "knowledge_write", update))["record"].clone();
        assert_eq!(
            stored["frontmatter"]["authorship"]["signer"],
            format!("seat:{}", f.oracle.id)
        );
        verify_knowledge_record(&stored);
    }
    let guessed = f.mutate(
        f.oracle.id,
        Ulid::new(),
        "knowledge_write",
        knowledge_record_args("decisions/d999999-wrong.md", "decision", "no effect"),
    );
    knowledge_denied(guessed.clone(), "decision_filename_required");
    assert!(
        guessed["structuredContent"]["error"]["message"]
            .as_str()
            .unwrap()
            .contains("-wrong.md")
    );
    assert!(
        !f.config
            .repository
            .join(".bboxy/decisions/d999999-wrong.md")
            .exists()
    );
    knowledge_denied(
        f.mutate(
            f.own.id,
            Ulid::new(),
            "knowledge_write",
            knowledge_record_args("records/unknown-worker.md", "unlisted-kind", "denied"),
        ),
        "knowledge_write_denied",
    );
    // Harmony addendum: Oracle may create typed native GitKB documents; workers
    // keep the existing allow-list and every update retains its original type.
    let custom = knowledge_create(
        &f,
        f.oracle.id,
        "unlisted-kind",
        "records/unknown.md",
        "Oracle native custom type",
    );
    assert_eq!(custom["frontmatter"]["type"], "unlisted-kind");
    verify_knowledge_record(&custom);
    let catalogue = f
        .runtime
        .block_on(f.client().control_catalogue(f.own.id, 1))
        .unwrap();
    let claim = catalogue["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|tool| tool["name"] == "artifact_claim")
        .unwrap();
    assert!(
        claim["description"]
            .as_str()
            .unwrap()
            .contains("responsibility only")
    );
    assert!(
        claim["description"]
            .as_str()
            .unwrap()
            .contains("knowledge_write")
    );
}

// keeps: decision #266
#[test]
fn mcp_knowledge266_worker_task_results_use_report_notice_and_acceptance_guards() {
    let mut f = Fixture::new();
    f.task_file(f.task_metadata());
    let initial = data(knowledge_fetch(&f, f.own.id, "#1"));
    let original_body = initial["body"].as_str().unwrap().to_owned();
    let mut args = knowledge_update_args(&initial);
    args["frontmatter"]["report"] =
        json!({"summary":"knowledge Report via canonical transition","commit":"abc123"});
    args["frontmatter"]["verify_result"] = verification("false", 0, json!([]));
    let op = Ulid::new();
    let first = f.mutate(f.own.id, op, "knowledge_write", args.clone());
    let record = data(first.clone())["record"].clone();
    assert_eq!(record["frontmatter"]["status"], "reported");
    assert_eq!(
        record["frontmatter"]["report"]["reported_by"],
        format!("seat:{}", f.own.id)
    );
    assert_eq!(record["frontmatter"]["report"]["op_id"], json!(op));
    assert!(record["body"].as_str().unwrap().starts_with(&original_body));
    assert_item_signature(&f.task_document(), f.own.id);
    assert_eq!(
        f.mutate(f.own.id, op, "knowledge_write", args.clone()),
        first
    );
    f.restart();
    assert_eq!(f.mutate(f.own.id, op, "knowledge_write", args), first);
    let inbox = data(f.call(f.oracle.id, Ulid::new(), "message_list", json!({})));
    let notices: Vec<_> = inbox["page"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| {
            row["envelope"]["kind"] == "task"
                && row["envelope"]["body"]["task_id"] == json!(f.task)
                && row["envelope"]["body"]["event"] == "report"
        })
        .collect();
    assert_eq!(notices.len(), 1, "{inbox}");
    refused(
        f.mutate(
            f.oracle.id,
            Ulid::new(),
            "task_accept",
            json!({"task_path":f.task_path()}),
        ),
        "does not match",
    );
    for (field, value) in [
        ("status", json!("accepted")),
        ("assignee", json!(format!("seat:{}", f.peer.id))),
        ("verify", json!("forged")),
        ("allowed_paths", json!(["**"])),
        ("custom", json!({"changed":true})),
    ] {
        let mut forged = knowledge_update_args(&record);
        forged["frontmatter"][field] = value;
        knowledge_denied(
            f.mutate(f.own.id, Ulid::new(), "knowledge_write", forged),
            "task_field_protected",
        );
    }
    let file = f
        .config
        .repository
        .join(format!(".bboxy/tasks/{}.md", f.task));
    let before = std::fs::read(&file).unwrap();
    let mut forged = knowledge_update_args(&record);
    forged["body"] = json!("changed brief");
    knowledge_denied(
        f.mutate(f.own.id, Ulid::new(), "knowledge_write", forged),
        "task_field_protected",
    );
    let mut forged = knowledge_update_args(&record);
    forged["frontmatter"]["report"]["reported_by"] = json!(format!("seat:{}", f.oracle.id));
    knowledge_denied(
        f.mutate(f.own.id, Ulid::new(), "knowledge_write", forged),
        "task_field_protected",
    );
    for actor in [f.peer.id, f.outside.id] {
        let mut other = knowledge_update_args(&record);
        other["frontmatter"]["report"]["summary"] = json!("not my task");
        knowledge_denied(
            f.mutate(actor, Ulid::new(), "knowledge_write", other),
            "task_field_protected",
        );
    }
    assert_eq!(std::fs::read(&file).unwrap(), before);
    let mut next = knowledge_update_args(&record);
    next["frontmatter"]["verify_result"] = verification("true", 1, json!([]));
    let next = data(f.mutate(f.own.id, Ulid::new(), "knowledge_write", next))["record"].clone();
    refused(
        f.mutate(
            f.oracle.id,
            Ulid::new(),
            "task_accept",
            json!({"task_path":f.task_path()}),
        ),
        "exit_code is 1",
    );
    let mut valid = knowledge_update_args(&next);
    valid["frontmatter"]["verify_result"] = verification("true", 0, json!([]));
    data(f.mutate(f.own.id, Ulid::new(), "knowledge_write", valid));
    data(f.mutate(
        f.oracle.id,
        Ulid::new(),
        "task_accept",
        json!({"task_path":f.task_path()}),
    ));
    let closed = data(knowledge_fetch(&f, f.own.id, "#1"));
    let mut blocked = knowledge_update_args(&closed);
    blocked["frontmatter"]["report"]["summary"] = json!("cannot rewrite closed results");
    refused(
        f.mutate(f.own.id, Ulid::new(), "knowledge_write", blocked),
        "closed task",
    );
    let foreign = data(knowledge_fetch(&f, f.own.id, "#2"));
    let mut blocked = knowledge_update_args(&foreign);
    blocked["frontmatter"]["report"] = json!({"summary":"foreign task"});
    knowledge_denied(
        f.mutate(f.own.id, Ulid::new(), "knowledge_write", blocked),
        "task_field_protected",
    );
}

// keeps: decision #266
#[test]
fn mcp_knowledge266_oracle_task_creation_and_open_brief_edits_do_not_delegate_or_accept() {
    let f = Fixture::new();
    knowledge_denied(
        f.mutate(
            f.own.id,
            Ulid::new(),
            "knowledge_write",
            knowledge_record_args("tasks/new.md", "task", "brief"),
        ),
        "task_field_protected",
    );
    let mut create = knowledge_record_args("tasks/new.md", "task", "new brief");
    create["frontmatter"]["verify"] = json!("true");
    create["frontmatter"]["allowed_paths"] = json!(["fixture/**"]);
    let mut forged = create.clone();
    forged["frontmatter"]["status"] = json!("accepted");
    knowledge_denied(
        f.mutate(f.oracle.id, Ulid::new(), "knowledge_write", forged),
        "task_field_protected",
    );
    let mut assigned = create.clone();
    assigned["frontmatter"]["assignee"] = json!(format!("seat:{}", f.own.id));
    knowledge_denied(
        f.mutate(f.oracle.id, Ulid::new(), "knowledge_write", assigned),
        "task_field_protected",
    );
    let first =
        data(f.mutate(f.oracle.id, Ulid::new(), "knowledge_write", create))["record"].clone();
    assert_eq!(first["frontmatter"]["status"], "open");
    assert_eq!(
        first["frontmatter"]["delegated_by"],
        format!("seat:{}", f.oracle.id)
    );
    let mut update = knowledge_update_args(&first);
    update["body"] = json!("open revised brief");
    update["frontmatter"]["verify"] = json!("true && true");
    update["frontmatter"]["allowed_paths"] = json!(["fixture/narrow/**"]);
    let edited =
        data(f.mutate(f.oracle.id, Ulid::new(), "knowledge_write", update))["record"].clone();
    let file = f.config.repository.join(".bboxy/tasks/new.md");
    let before = std::fs::read(&file).unwrap();
    for (field, value) in [
        ("status", json!("reported")),
        ("assignee", json!(format!("seat:{}", f.own.id))),
        ("delegated_by", json!(format!("seat:{}", f.peer.id))),
    ] {
        let mut bad = knowledge_update_args(&edited);
        bad["frontmatter"][field] = value;
        knowledge_denied(
            f.mutate(f.oracle.id, Ulid::new(), "knowledge_write", bad),
            "task_field_protected",
        );
    }
    assert_eq!(std::fs::read(&file).unwrap(), before);
    data(f.mutate(
        f.oracle.id,
        Ulid::new(),
        "task_delegate",
        json!({"task_path":".bboxy/tasks/new.md","seat_id":f.own.id}),
    ));
    let claimed = data(knowledge_fetch(&f, f.oracle.id, "tasks/new.md"));
    let mut bad = knowledge_update_args(&claimed);
    bad["body"] = json!("cannot edit claimed brief");
    knowledge_denied(
        f.mutate(f.oracle.id, Ulid::new(), "knowledge_write", bad),
        "task_field_protected",
    );
    assert_eq!(
        data(knowledge_fetch(&f, f.oracle.id, "tasks/new.md"))["frontmatter"]["status"],
        "claimed"
    );
}

// keeps: decision #266
#[test]
fn mcp_knowledge266_artifacts_keep_claims_observations_lineage_and_creator_corrections() {
    let f = Fixture::new();
    let mut create = knowledge_record_args("artifacts/generic.md", "artifact", "description");
    create["frontmatter"]["ref"] = json!("fixture-output-a");
    create["frontmatter"]["digest"] = json!("unverified-a");
    let first =
        data(f.mutate(f.own.id, Ulid::new(), "knowledge_write", create.clone()))["record"].clone();
    let metadata = &first["frontmatter"];
    assert_eq!(metadata["claimed"], false);
    assert_eq!(metadata["produced_by"], format!("seat:{}", f.own.id));
    assert_eq!(metadata["grade"], "inferred");
    assert_eq!(metadata["history"], json!([]));
    for (field, value) in [
        ("claimed", json!(true)),
        ("produced_by", json!(format!("seat:{}", f.peer.id))),
        ("grade", json!("reported")),
        ("history", json!([{"kind":"claimed"}])),
        ("for", json!(["[[#1]]"])),
        ("derived_from", json!(["foreign"])),
        ("observed", json!(true)),
    ] {
        let mut bad = knowledge_update_args(&first);
        bad["frontmatter"][field] = value;
        knowledge_denied(
            f.mutate(f.own.id, Ulid::new(), "knowledge_write", bad),
            "artifact_field_protected",
        );
    }
    let mut update = knowledge_update_args(&first);
    update["frontmatter"]["ref"] = json!("fixture-output-b");
    update["frontmatter"]["digest"] = json!("unverified-b");
    let corrected =
        data(f.mutate(f.own.id, Ulid::new(), "knowledge_write", update))["record"].clone();
    let mut steal = knowledge_update_args(&corrected);
    steal["frontmatter"]["ref"] = json!("peer-change");
    knowledge_denied(
        f.mutate(f.peer.id, Ulid::new(), "knowledge_write", steal),
        "artifact_field_protected",
    );
    let mut body = knowledge_update_args(&corrected);
    body["body"] = json!("peer description, same responsibility");
    let described =
        data(f.mutate(f.peer.id, Ulid::new(), "knowledge_write", body))["record"].clone();
    assert_eq!(
        described["frontmatter"]["produced_by"],
        format!("seat:{}", f.own.id)
    );
    assert_eq!(
        described["frontmatter"]["authorship"]["signer"],
        format!("seat:{}", f.peer.id)
    );
    let mut forged = create;
    forged["path"] = json!("artifacts/forged.md");
    forged["frontmatter"]["claimed"] = json!(true);
    knowledge_denied(
        f.mutate(f.own.id, Ulid::new(), "knowledge_write", forged),
        "artifact_field_protected",
    );
    for (number, claimed, observed) in [(4, false, true), (5, true, false)] {
        let id = Ulid::new();
        let path = format!("artifacts/protected-{number}.md");
        let metadata = json!({"id":id,"number":number,"type":"artifact","title":"trusted detector/claim fixture","ref":"observed-output","digest":"observed-digest","claimed":claimed,"produced_by":format!("seat:{}",f.own.id),"grade":"inferred","history":[],"for":[],"derived_from":[],"observed":observed});
        let file = f.config.repository.join(".bboxy").join(&path);
        std::fs::write(
            &file,
            format!(
                "---\n{}---\nExisting detector/claim fixture.\n",
                serde_yaml::to_string(&metadata).unwrap()
            ),
        )
        .unwrap();
        let record = data(knowledge_fetch(&f, f.own.id, &path));
        let before = std::fs::read(&file).unwrap();
        for actor in [f.own.id, f.oracle.id] {
            for (field, value) in [
                ("ref", json!("forged-ref")),
                ("digest", json!("forged-digest")),
                ("observed", json!(!observed)),
                ("claimed", json!(!claimed)),
            ] {
                let mut bad = knowledge_update_args(&record);
                bad["frontmatter"][field] = value;
                knowledge_denied(
                    f.mutate(actor, Ulid::new(), "knowledge_write", bad),
                    "artifact_field_protected",
                );
            }
            let mut relabel = knowledge_update_args(&record);
            relabel["frontmatter"]["type"] = json!("note");
            knowledge_denied(
                f.mutate(actor, Ulid::new(), "knowledge_write", relabel),
                "type_immutable",
            );
        }
        assert_eq!(std::fs::read(&file).unwrap(), before);
    }
}

// keeps: decision #266
#[test]
fn mcp_knowledge266_search_pages_use_shared_root_resolution_and_refuse_stale_cursors() {
    let f = Fixture::new();
    let mut records = vec![];
    for n in 0..3 {
        records.push(knowledge_create(
            &f,
            f.own.id,
            "note",
            &format!("notes/needle-{n}.md"),
            "unique-knowledge-needle\n",
        ));
    }
    let spec = knowledge_create(
        &f,
        f.oracle.id,
        "spec",
        "specs/readable.md",
        "Oracle knowledge body\n",
    );
    assert_eq!(
        data(knowledge_fetch(
            &f,
            f.own.id,
            &format!("#{}", spec["number"].as_u64().unwrap())
        ))["body"],
        "Oracle knowledge body\n"
    );
    for record in &records {
        assert_eq!(
            data(knowledge_fetch(
                &f,
                f.own.id,
                record["id"].as_str().unwrap()
            ))["id"],
            record["id"]
        );
    }
    let query = "type:note unique-knowledge-needle";
    // The live fixture's detector may materialize artifact records between
    // calls. A snapshot-bound cursor must reject that, so restart the whole
    // walk on that one named response; never combine different snapshots.
    let deadline = Instant::now() + Duration::from_secs(120);
    let (seen, old_cursor) = 'walk: loop {
        assert!(
            Instant::now() < deadline,
            "knowledge root never yielded a stable bounded page walk"
        );
        let first = data(knowledge_search(
            &f,
            f.own.id,
            json!({"query":query,"limit":1}),
        ));
        assert_eq!(
            first["scope"],
            json!({"seat":f.own.id,"role":f.own.role.name,"class":f.own.role.class})
        );
        assert!(first.get("empty_reason").is_none());
        assert_eq!(first["items"].as_array().unwrap().len(), 1);
        let old_cursor = first["next"].as_str().unwrap().to_owned();
        let mut seen = std::collections::BTreeSet::new();
        seen.insert(first["items"][0]["ref"]["id"].as_str().unwrap().to_owned());
        let mut next = Some(old_cursor.clone());
        while let Some(cursor) = next {
            let response = knowledge_search(
                &f,
                f.own.id,
                json!({"query":query,"limit":1,"cursor":cursor}),
            );
            if response["isError"] == true
                && response["structuredContent"]["error"]["code"] == "stale_cursor"
            {
                eprintln!("knowledge page walk restarted on an explicit snapshot change");
                continue 'walk;
            }
            let page = data(response);
            assert!(
                serde_json::to_vec(&page).unwrap().len()
                    <= bbt_mux::knowledge::data_source::MAX_PAGE_BYTES + 1024
            );
            for item in page["items"].as_array().unwrap() {
                assert!(seen.insert(item["ref"]["id"].as_str().unwrap().to_owned()));
            }
            next = page["next"].as_str().map(str::to_owned);
        }
        break (seen, old_cursor);
    };
    assert_eq!(seen.len(), 3);
    knowledge_denied(
        knowledge_search(
            &f,
            f.own.id,
            json!({"query":"changed query","cursor":old_cursor}),
        ),
        "invalid_cursor",
    );
    let mut update = knowledge_update_args(&records[0]);
    update["body"] = json!("unique-knowledge-needle revised\n");
    data(f.mutate(f.own.id, Ulid::new(), "knowledge_write", update));
    knowledge_denied(
        knowledge_search(&f, f.own.id, json!({"query":query,"cursor":old_cursor})),
        "stale_cursor",
    );
    let empty = data(knowledge_search(
        &f,
        f.own.id,
        json!({"query":"no-such-knowledge-fixture"}),
    ));
    assert_eq!(empty["items"], json!([]));
    assert!(empty["next"].is_null());
    assert_eq!(
        empty["scope"],
        json!({"seat":f.own.id,"role":f.own.role.name,"class":f.own.role.class})
    );
    assert_eq!(empty["empty_reason"], "nothing_yet");
    assert!(empty["empty_message"].as_str().unwrap().len() > 10);
}

/// Decision #269: an operator envelope sent through ctl carries its door and issuer in the
/// signed body, the native frame names both, and the window's sends carry door gui.
// keeps: decision #269
#[test]
fn d269_ctl_envelopes_carry_door_and_issuer_signed_and_framed() {
    use bbt_proto::messaging::{Door, Request, Response};
    let f = Fixture::new();
    f.prepare_operator();
    let own = f.own.id;
    let send = |text: &str, origin: bbt_mux::mcp::Origin| {
        let id = Ulid::new();
        let mut message = bbt_mux::messaging::SendRequest::text(format!("seats/{own}/inbox"), text);
        message.id = id;
        let response = f
            .runtime
            .block_on(
                f.client()
                    .gui_messaging(id, f.own.swarm, Request::Send { message }, origin),
            )
            .unwrap();
        let Response::Stored { message } = response else {
            panic!("{response:?}")
        };
        *message
    };
    let ctl = send(
        "a question through ctl",
        bbt_mux::mcp::Origin {
            door: Door::Ctl,
            issued_by: Some("Vesper (Oracle)".into()),
        },
    );
    // In the envelope bytes, and signed: changing or stripping either breaks the signature.
    let wire = serde_json::to_value(ctl.envelope().expect("signed fixture envelope")).unwrap();
    assert_eq!(wire["door"], "ctl", "{wire}");
    assert_eq!(wire["issued_by"], "Vesper (Oracle)", "{wire}");
    ctl.envelope()
        .expect("signed fixture envelope")
        .verify_signature()
        .unwrap();
    let mut forged = ctl.envelope().expect("signed fixture envelope").clone();
    forged.issued_by = Some("Mallory (Oracle)".into());
    assert!(forged.verify_signature().is_err(), "the issuer is signed");
    let mut stripped = ctl.envelope().expect("signed fixture envelope").clone();
    stripped.door = None;
    stripped.issued_by = None;
    assert!(stripped.verify_signature().is_err(), "the door is signed");
    // The frame a native Claude or Codex seat reads names the principal, door and issuer.
    let frame = bbt_mux::messaging::ledger::Work {
        id: Ulid::new(),
        recipient: own,
        agent: None,
        messages: vec![ctl.clone()],
        class: Class::Oracle,
        digest: false,
        wake_members: vec![],
    }
    .text();
    assert!(
        frame.contains("from: the operator principal, via ctl, issued by Vesper (Oracle)\n"),
        "{frame}"
    );
    assert!(!frame.contains(&Ulid::nil().to_string()), "{frame}");
    // The window's own send records door gui and no issuer.
    let id = Ulid::new();
    let mut message =
        bbt_mux::messaging::SendRequest::text(format!("seats/{own}/inbox"), "from the window");
    message.id = id;
    let reply = f.gui(
        id,
        bbt_proto::GuiAction::Messaging {
            id,
            swarm_id: bbt_proto::SwarmId::from_u128(f.own.swarm.0),
            request: Request::Send { message },
        },
    );
    let bbt_proto::GuiReply::Messaging {
        response: Response::Stored { message },
        ..
    } = reply
    else {
        panic!("{reply:?}")
    };
    assert_eq!(
        message.envelope().expect("signed fixture envelope").door,
        Some(Door::Gui)
    );
    assert_eq!(
        message
            .envelope()
            .expect("signed fixture envelope")
            .issued_by,
        None
    );
    message
        .envelope()
        .expect("signed fixture envelope")
        .verify_signature()
        .unwrap();
    // A seat's own envelope never carries either.
    let seat = data(f.mutate(
        own,
        Ulid::new(),
        "message_send",
        json!({"destination":format!("seats/{}/inbox", f.peer.id),"text":"seat says"}),
    ));
    assert!(seat["message"]["envelope"].get("door").is_none(), "{seat}");
    assert!(
        seat["message"]["envelope"].get("issued_by").is_none(),
        "{seat}"
    );
}

// keeps: spec messaging-attention MA-2 (own-seat tools, no borrowed Oracle authority)
#[test]
fn mcp_attention_is_owned_by_the_admitted_receiver_and_never_grants_other_seats() {
    let f = Fixture::new();
    let own = f.own.id;
    let before = data(f.call(own, Ulid::new(), "attention_get", json!({})));
    assert_eq!(before["seat_id"], json!(own));
    assert_eq!(before["policy"], json!({"rules":[]}));
    assert_eq!(before["defaults"]["conversation"], "digest");
    assert_eq!(before["defaults"]["direct_conversation"], "wake");
    assert_eq!(before["defaults"]["event"], "inbox");
    let epoch = f.epoch(own);
    let set = data(f.mutate(
        own,
        Ulid::new(),
        "attention_set",
        json!({
            "destination":"#status", "class":"event", "mode":"wake"
        }),
    ));
    assert_eq!(
        set["policy"]["rules"],
        json!([
            {"destination":"channels/status","class":"event","mode":"wake"}
        ])
    );
    assert_eq!(f.epoch(own), epoch);
    assert!(!f.registry().get(own).unwrap().muted);
    for caller in [own, f.oracle.id] {
        let catalogue = f
            .runtime
            .block_on(f.client().control_catalogue(caller, 1))
            .unwrap();
        for name in ["attention_get", "attention_set"] {
            let tool = catalogue["tools"]
                .as_array()
                .unwrap()
                .iter()
                .find(|tool| tool["name"] == name)
                .unwrap();
            assert_eq!(tool["annotations"]["readOnlyHint"], name == "attention_get");
            assert!(tool["inputSchema"]["properties"].get("seat_id").is_none());
            let result = f.call(caller, Ulid::new(), name, if name == "attention_get" {
                json!({"seat_id":f.peer.id})
            } else {
                json!({"seat_id":f.peer.id,"destination":"channels/status","class":"event","mode":"wake"})
            });
            assert_eq!(result["isError"], true, "{result}");
        }
    }
    for args in [
        json!({"destination":"channels/status","class":"event","mode":"mute"}),
        json!({"destination":"channels/status","class":"unknown","mode":"wake"}),
        json!({"destination":"channels/*","class":"event","mode":"wake"}),
    ] {
        assert_eq!(
            f.mutate(own, Ulid::new(), "attention_set", args)["isError"],
            true
        );
    }
    assert_eq!(
        data(f.call(own, Ulid::new(), "attention_get", json!({})))["policy"],
        set["policy"]
    );
    assert_eq!(f.registry().get(f.peer.id).unwrap().attention, None);
    assert_eq!(f.registry().get(f.oracle.id).unwrap().attention, None);
}

// keeps: spec messaging-attention MA-2 (durability and existing operation replay)
#[test]
fn mcp_attention_replay_survives_restart_without_rolling_back_a_later_policy() {
    let mut f = Fixture::new();
    let own = f.own.id;
    let op = Ulid::new();
    let args = json!({"destination":"channels/work","class":"conversation","mode":"inbox"});
    let first = f.mutate(own, op, "attention_set", args.clone());
    assert_eq!(first["isError"], false);
    data(f.mutate(
        own,
        Ulid::new(),
        "attention_set",
        json!({
            "destination":"channels/work","class":"conversation","mode":"digest"
        }),
    ));
    assert_eq!(f.mutate(own, op, "attention_set", args.clone()), first);
    let reused = f.mutate(
        own,
        op,
        "attention_set",
        json!({
            "destination":"channels/work","class":"conversation","mode":"wake"
        }),
    );
    assert_eq!(reused["structuredContent"]["error"]["code"], "op_id_reused");
    f.registry()
        .update(own, |record| {
            record.name = "renamed receiver".into();
            Ok(())
        })
        .unwrap();
    f.restart();
    assert_eq!(f.mutate(own, op, "attention_set", args), first);
    let current = data(f.call(own, Ulid::new(), "attention_get", json!({})));
    assert_eq!(
        current["policy"]["rules"],
        json!([
            {"destination":"channels/work","class":"conversation","mode":"digest"}
        ])
    );
    assert_eq!(f.registry().get(own).unwrap().name, "renamed receiver");
}

// keeps: spec messaging-attention §7 MA-6 — cold Orient includes effective policy inputs
#[test]
fn mcp_orient_reports_role_defaults_overrides_and_always_wake_exceptions() {
    let f = Fixture::new();
    for record in [&f.own, &f.oracle] {
        let orient = data(f.call(record.id, Ulid::new(), "orient", json!({})));
        let attention = &orient["attention"];
        assert_eq!(attention["seat_id"], json!(record.id));
        assert_eq!(attention["role"], json!(record.role));
        assert_eq!(attention["policy"], json!({"rules":[]}));
        assert_eq!(attention["defaults"]["event"], "inbox");
        assert_eq!(attention["defaults"]["direct_conversation"], "wake");
        assert_eq!(attention["defaults"]["named_conversation"], "wake");
        assert_eq!(
            attention["defaults"]["conversation"],
            if record.role.class == Class::Oracle {
                "wake"
            } else {
                "digest"
            }
        );
        assert_eq!(
            attention["always_wake"],
            json!([
                "operator_conversation",
                "delegated_task",
                "own_thread_reply"
            ])
        );
        assert_eq!(attention["muted"], false);
    }
    for name in ["architect", "creativity"] {
        f.registry()
            .update(f.oracle.id, |record| {
                record.role.name = name.into();
                Ok(())
            })
            .unwrap();
        let orient = data(f.call(f.oracle.id, Ulid::new(), "orient", json!({})));
        assert_eq!(orient["attention"]["defaults"]["conversation"], "inbox");
        assert_eq!(
            orient["attention"]["defaults"]["direct_conversation"],
            "wake"
        );
        assert_eq!(orient["attention"]["defaults"]["event"], "inbox");
    }
    data(f.mutate(
        f.own.id,
        Ulid::new(),
        "attention_set",
        json!({"destination":"#status","class":"event","mode":"digest"}),
    ));
    let orient = data(f.call(f.own.id, Ulid::new(), "orient", json!({})));
    assert_eq!(
        orient["attention"]["policy"]["rules"],
        json!([
            {"destination":"channels/status","class":"event","mode":"digest"}
        ])
    );
    assert!(
        orient["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool["name"] == "message_list")
    );
}

// keeps: MA-5 named thread refusals preserve structured fields and operation replay.
#[test]
fn unknown_thread_mcp_refusal_keeps_the_visible_hint_and_replays_without_a_send() {
    let f = Fixture::new();
    let visible = Ulid::new();
    data(f.mutate(
        f.peer.id,
        visible,
        "message_send",
        json!({
            "destination":format!("seats/{}/inbox",f.own.id),"text":"visible thread tip"
        }),
    ));
    let op = Ulid::new();
    let args = json!({"reply_to":Ulid::new(),"text":"cannot answer an unknown id"});
    let refused = f.mutate(f.own.id, op, "message_send", args.clone());
    assert_eq!(refused["isError"], true, "{refused}");
    let result = &refused["structuredContent"];
    assert_eq!(result["op_id"], json!(op));
    assert_eq!(result["error"]["code"], "unknown_thread");
    assert_eq!(result["error"]["kind"], "unknown_thread");
    assert!(
        result["error"]["hint"]
            .as_str()
            .unwrap()
            .contains(&visible.to_string())
    );
    let text: Value =
        serde_json::from_str(refused["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(text, *result);
    let replay = f.mutate(f.own.id, op, "message_send", args);
    assert_eq!(replay, refused);
}

// keeps: MA-10 real MCP tool signs resolved IDs; only named channel members wake; replay never resolves again.
#[test]
fn mcp_channel_mentions_sign_ids_and_pin_each_receivers_wake() {
    let mut f = Fixture::new();
    let channel = "channels/mention-proof";
    data(f.mutate(
        f.oracle.id,
        Ulid::new(),
        "ch_link",
        json!({"node":{"path":channel,"kind":"folder"}}),
    ));
    for seat in [f.own.id, f.peer.id] {
        data(f.mutate(f.oracle.id, Ulid::new(), "ch_link", json!({"node":{
            "path":format!("{channel}/{seat}"),"kind":"link","target":format!("seats/{seat}/inbox")
        }})));
    }
    let op = Ulid::new();
    let args = json!({"destination":format!("{channel}/*"),"text":"@pEeR please review this; mail user@outside.example"});
    let result = f.mutate(f.oracle.id, op, "message_send", args.clone());
    let sent = data(result.clone());
    assert_eq!(sent["message"]["envelope"]["mentions"], json!([f.peer.id]));
    let envelope: bbt_proto::messaging::Envelope =
        serde_json::from_value(sent["message"]["envelope"].clone()).unwrap();
    envelope.verify_signature().unwrap();
    let receipts = data(f.call(
        f.oracle.id,
        Ulid::new(),
        "message_receipts",
        json!({"message":op}),
    ));
    let rows = receipts["page"]["items"].as_array().unwrap();
    assert_eq!(rows.len(), 2);
    let peer = rows
        .iter()
        .find(|row| row["recipient"] == json!(f.peer.id))
        .unwrap();
    let own = rows
        .iter()
        .find(|row| row["recipient"] == json!(f.own.id))
        .unwrap();
    assert_eq!(peer["wake"], "now");
    assert_eq!(own["wake"], "deferred");
    f.registry()
        .update(f.peer.id, |seat| {
            seat.name = "Renamed Peer".into();
            Ok(())
        })
        .unwrap();
    f.restart();
    assert_eq!(
        f.mutate(f.oracle.id, op, "message_send", args.clone()),
        result
    );
    let refused = f.mutate(f.oracle.id, Ulid::new(), "message_send", args);
    assert_eq!(
        refused["structuredContent"]["error"]["kind"],
        "unknown_mention"
    );
    assert!(
        refused["structuredContent"]["error"]["hint"]
            .as_str()
            .unwrap()
            .contains("Renamed Peer")
    );
}

// keeps: caller IDs grant no mention authority; GUI journal replay pins both signed and refused results.
#[test]
fn composer_mentions_replace_proposed_ids_and_replay_original_identity_after_rename() {
    use bbt_proto::{
        GuiAction, GuiReply,
        messaging::{Request, Response, SendRequest},
    };
    let mut f = Fixture::new();
    f.prepare_operator();
    let send = |f: &Fixture, message: SendRequest| {
        let id = message.id;
        match f.gui_once(
            id,
            GuiAction::Messaging {
                id,
                swarm_id: bbt_proto::SwarmId::from_u128(f.own.swarm.0),
                request: Request::Send { message },
            },
        ) {
            GuiReply::Messaging { response, .. } => response,
            reply => panic!("{reply:?}"),
        }
    };
    let destination = format!("seats/{}/inbox", f.own.id);
    let mut ordinary = SendRequest::text(&destination, "ordinary prose");
    ordinary.mentions = vec![f.outside.id];
    let Response::Stored { message } = send(&f, ordinary) else {
        panic!("ordinary stored")
    };
    assert!(
        message.view().mentions.is_empty(),
        "unsigned caller IDs must be cleared"
    );

    let mut request = SendRequest::text(&destination, "@pEeR review user@example.org");
    request.mentions = vec![f.outside.id];
    request.attachments = vec!["[[images/custody-fixture]]".into()];
    let first = send(&f, request.clone());
    let Response::Stored { message } = &first else {
        panic!("{first:?}")
    };
    assert_eq!(message.view().mentions, [f.peer.id]);
    assert_eq!(message.view().attachments, request.attachments);
    message.envelope().unwrap().verify_signature().unwrap();
    f.registry()
        .update(f.peer.id, |seat| {
            seat.name = "Renamed peer".into();
            Ok(())
        })
        .unwrap();
    f.restart();
    assert_eq!(
        serde_json::to_value(send(&f, request.clone())).unwrap(),
        serde_json::to_value(first).unwrap()
    );

    request.id = Ulid::new();
    let refused = send(&f, request.clone());
    assert!(matches!(&refused, Response::Refused { error } if error.kind == "unknown_mention"));
    f.registry()
        .update(f.peer.id, |seat| {
            seat.name = "Peer".into();
            Ok(())
        })
        .unwrap();
    f.restart();
    assert_eq!(
        serde_json::to_value(send(&f, request)).unwrap(),
        serde_json::to_value(refused).unwrap()
    );

    let supplied = f.mutate(
        f.own.id,
        Ulid::new(),
        "message_send",
        json!({
            "destination": destination, "text":"ordinary prose", "mentions":[f.peer.id]
        }),
    );
    assert_eq!(
        supplied["isError"], true,
        "MCP accepts names, never caller-resolved IDs"
    );
}

// keeps: scoped mention reads isolate foreign corruption; ordinary prose has no roster dependency.
#[test]
fn mentions_ignore_foreign_corruption_and_email_only_messages_do_not_read_the_roster() {
    use bbt_proto::messaging::{Request, Response, SendRequest};
    let f = Fixture::new();
    f.prepare_operator();
    let destination = format!("seats/{}/inbox", f.own.id);
    let send_mcp = |text: &str| {
        f.mutate(
            f.own.id,
            Ulid::new(),
            "message_send",
            json!({"destination":destination,"text":text}),
        )
    };
    let send_gui = |text: &str| {
        let message = SendRequest::text(&destination, text);
        let id = message.id;
        match f.gui_once(
            id,
            bbt_proto::GuiAction::Messaging {
                id,
                swarm_id: bbt_proto::SwarmId::from_u128(f.own.swarm.0),
                request: Request::Send { message },
            },
        ) {
            bbt_proto::GuiReply::Messaging { response, .. } => Ok(response),
            reply => Err(format!("{reply:?}")),
        }
    };
    data(send_mcp("prime private recipient route"));
    assert!(matches!(
        send_gui("prime private operator route"),
        Ok(Response::Stored { .. })
    ));
    let account = SqliteStore::open(&f.config.run_dir.join("account")).unwrap();
    let mut seeded = Vec::new();
    {
        let mut db = account.connection().unwrap();
        let tx = db.transaction().unwrap();
        for id in 100u128..1124 {
            let seat = SeatRecord::new(Ulid::from(id), f.outside.swarm, "Foreign", "zsh");
            let key = format!("seat/{}", seat.id);
            tx.execute(
                "INSERT INTO kv VALUES(?1,?2)",
                rusqlite::params![key, serde_json::to_vec(&seat).unwrap()],
            )
            .unwrap();
            seeded.push(key);
        }
        for (key, bytes) in [
            ("seat/foreign-invalid-json", b"{invalid-json".to_vec()),
            (
                "seat/foreign-invalid-record",
                serde_json::to_vec(
                    &json!({"swarm":f.outside.swarm,"lifecycle":"open","role":false}),
                )
                .unwrap(),
            ),
        ] {
            tx.execute(
                "INSERT INTO kv VALUES(?1,?2)",
                rusqlite::params![key, bytes],
            )
            .unwrap();
            seeded.push(key.into());
        }
        tx.commit().unwrap();
    }
    let sent = send_mcp("@Peer local mention with many corrupt foreign records");
    let composed = send_gui("@Peer local GUI mention with corrupt foreign records");
    let foreign_card = f.runtime.block_on(f.client().identity_status());
    {
        let mut db = account.connection().unwrap();
        let tx = db.transaction().unwrap();
        for key in seeded {
            tx.execute("DELETE FROM kv WHERE key=?1", [key]).unwrap();
        }
        tx.commit().unwrap();
    }
    assert_eq!(
        sent["isError"], false,
        "local mention must ignore foreign records: {sent}"
    );
    let envelope: bbt_proto::messaging::Envelope =
        serde_json::from_value(data(sent)["message"]["envelope"].clone()).unwrap();
    assert_eq!(envelope.mentions, [f.peer.id]);
    envelope.verify_signature().unwrap();
    // An unreadable open local non-recipient makes genuine resolution unavailable.
    // Emails and lone @ must still send their original text with no signed IDs.
    let key = "seat/local-invalid-record";
    account
        .put(
            key,
            &serde_json::to_vec(&json!({"swarm":f.own.swarm,"lifecycle":"open","role":false}))
                .unwrap(),
        )
        .unwrap();
    let text = "mail user@example.org; path/a@host; lone @";
    let email = send_mcp(text);
    let gui_email = send_gui(text);
    let genuine = send_mcp("@Peer genuine mention needs its roster");
    let local_card = f.runtime.block_on(f.client().identity_status());
    account
        .connection()
        .unwrap()
        .execute("DELETE FROM kv WHERE key=?1", [key])
        .unwrap();
    assert_eq!(
        email["isError"], false,
        "email-only message must not read the roster: {email}"
    );
    let envelope: bbt_proto::messaging::Envelope =
        serde_json::from_value(data(email)["message"]["envelope"].clone()).unwrap();
    assert!(envelope.mentions.is_empty());
    assert_eq!(envelope.body.text(), Some(text));
    envelope.verify_signature().unwrap();
    let Ok(Response::Stored { message }) = composed else {
        panic!("local GUI mention must ignore foreign records: {composed:?}")
    };
    assert_eq!(message.view().mentions, [f.peer.id]);
    message.envelope().unwrap().verify_signature().unwrap();

    let Ok(Response::Stored { message }) = gui_email else {
        panic!("email-only GUI message must not read the roster: {gui_email:?}")
    };
    assert!(message.view().mentions.is_empty());
    assert_eq!(message.envelope().unwrap().body.text(), Some(text));
    assert_eq!(genuine["structuredContent"]["error"]["code"], "unavailable");
    assert!(
        foreign_card.is_err() && local_card.is_err(),
        "full count-bearing status must preserve count errors"
    );
}

// keeps: MA-10 unknown/ambiguous names refuse before effects and reveal only open same-swarm names.
#[test]
fn mcp_and_composer_mentions_share_typed_refusals_before_message_or_signing_effects() {
    use bbt_proto::messaging::{Request, Response, SendRequest};
    let f = Fixture::new();
    f.prepare_operator();
    let counters = || {
        let store = SqliteStore::open(&f.config.run_dir.join("account")).unwrap();
        let db = store.connection().unwrap();
        let sequences: i64 = db
            .query_row(
                "SELECT coalesce(sum(sequence),0) FROM msg_counters",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let held: i64 = db
            .query_row("SELECT count(*) FROM msg_held", [], |row| row.get(0))
            .unwrap();
        (sequences, held)
    };
    let before = counters();
    let args = json!({"destination":format!("seats/{}/inbox", f.own.id),"text":"@Outside cannot address another swarm"});
    let unknown = Ulid::new();
    let rejected = f.mutate(f.own.id, unknown, "message_send", args.clone());
    let error = &rejected["structuredContent"]["error"];
    assert_eq!(error["kind"], "unknown_mention");
    let hint = error["hint"].as_str().unwrap();
    assert!(hint.contains("Peer"));
    assert!(!hint.contains("Outside"));
    f.registry()
        .update(f.own.id, |seat| {
            seat.name = "Outside".into();
            Ok(())
        })
        .unwrap();
    assert_eq!(f.mutate(f.own.id, unknown, "message_send", args), rejected);
    f.registry()
        .update(f.own.id, |seat| {
            seat.name = "Own".into();
            Ok(())
        })
        .unwrap();
    let mut message = SendRequest::text(
        format!("seats/{}/inbox", f.own.id),
        "@Outside same composer rule",
    );
    let id = message.id;
    let response = f
        .runtime
        .block_on(f.client().gui_messaging(
            id,
            f.own.swarm,
            Request::Send {
                message: message.clone(),
            },
            Default::default(),
        ))
        .unwrap();
    let Response::Refused { error } = response else {
        panic!("{response:?}");
    };
    assert_eq!(error.kind, "unknown_mention");
    assert!(error.hint.contains("Peer"));
    f.registry()
        .update(f.own.id, |seat| {
            seat.name = "Peer Second".into();
            Ok(())
        })
        .unwrap();
    message.id = Ulid::new();
    message.text = "@Peer ambiguous first word".into();
    let response = f
        .runtime
        .block_on(f.client().gui_messaging(
            message.id,
            f.own.swarm,
            Request::Send { message },
            Default::default(),
        ))
        .unwrap();
    assert!(matches!(response, Response::Refused { error } if error.kind == "ambiguous_mention"));
    f.registry()
        .update(f.peer.id, |seat| {
            seat.lifecycle = bbt_mux::registry::Lifecycle::ClosedRetained;
            Ok(())
        })
        .unwrap();
    let message = SendRequest::text(
        format!("seats/{}/inbox", f.own.id),
        "@{Peer} is no longer open",
    );
    let response = f
        .runtime
        .block_on(f.client().gui_messaging(
            message.id,
            f.own.swarm,
            Request::Send { message },
            Default::default(),
        ))
        .unwrap();
    assert!(matches!(response, Response::Refused { error } if error.kind == "unknown_mention"));
    assert_eq!(counters(), before);
}

// keeps: #334 a real admitted sender can withdraw through message_send, never another seat.
#[test]
fn mcp_withdrawal_uses_the_signed_sender_and_preserves_its_cancelled_receipt() {
    let f = Fixture::new();
    let old = Ulid::new();
    data(f.mutate(
        f.peer.id,
        old,
        "message_send",
        json!({
            "destination":format!("seats/{}/inbox",f.own.id),"text":"replace this"
        }),
    ));
    let id = Ulid::new();
    let args = json!({"kind":"withdrawn","supersedes":old,"text":""});
    let response = f.mutate(f.peer.id, id, "message_send", args.clone());
    let message = data(response.clone());
    assert_eq!(message["message"]["envelope"]["kind"], "withdrawn");
    assert_eq!(
        message["message"]["envelope"]["body"],
        json!({"type":"withdrawn"})
    );
    assert_eq!(message["message"]["envelope"]["supersedes"], json!(old));
    assert_eq!(f.mutate(f.peer.id, id, "message_send", args), response);
    let receipts = data(f.call(
        f.peer.id,
        Ulid::new(),
        "message_receipts",
        json!({"message":old}),
    ));
    assert_eq!(receipts["page"]["items"][0]["wake"], "none");
    assert_eq!(receipts["page"]["items"][0]["reason"], "superseded");
    let denied = f.mutate(
        f.own.id,
        Ulid::new(),
        "message_send",
        json!({"kind":"withdrawn","supersedes":old,"text":""}),
    );
    assert_eq!(denied["isError"], true);
    assert_eq!(
        denied["structuredContent"]["error"]["kind"],
        "invalid_supersedes"
    );
}

// keeps: #334 actual task frontmatter transitions cancel a pending delegation across restart.
#[test]
fn indexed_task_acceptance_cancels_the_pending_delegation_with_a_done_reason() {
    let mut f = Fixture::new();
    f.registry()
        .update(f.own.id, |own| {
            own.muted = true;
            Ok(())
        })
        .unwrap();
    let mut metadata = f.task_metadata();
    metadata["assignee"] = json!("none");
    metadata["status"] = json!("open");
    f.task_file(metadata);
    let delegated = data(f.mutate(
        f.oracle.id,
        Ulid::new(),
        "task_delegate",
        json!({
            "task_path":f.task_path(),"seat_id":f.own.id,"role":"executor"
        }),
    ));
    let id = delegated["message_ids"][0].as_str().unwrap().to_owned();
    data(f.report(verification("true", 0, json!([]))));
    data(f.mutate(
        f.oracle.id,
        Ulid::new(),
        "task_accept",
        json!({"task_path":f.task_path()}),
    ));
    assert_eq!(f.task_document().metadata["status"], "accepted");
    for restart in [false, true] {
        if restart {
            f.restart();
        }
        let receipts = data(f.call(
            f.oracle.id,
            Ulid::new(),
            "message_receipts",
            json!({"message":id}),
        ));
        let receipt = &receipts["page"]["items"][0];
        assert_eq!(receipt["evidence"], "recipient-stored");
        assert_eq!(receipt["wake"], "none");
        assert_eq!(receipt["reason"], "done");
    }
}
