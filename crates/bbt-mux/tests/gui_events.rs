// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
#![allow(
    clippy::disallowed_methods,
    reason = "test/fixture IO, not the UI thread"
)]

use bbt_mux::{
    Config, LaunchSpec, Supervisor, Ulid,
    logic::{Logic, LogicConfig, SocketSupervisor},
    messaging::wire::EnvelopeExt,
    registry::SeatRecord,
    store::{Store, events::Draft, sqlite::SqliteStore},
};
use bbt_proto::{
    GuiAction, GuiReply, GuiRequest, SeatId, View,
    events::GuiEventPayload,
    messaging::{IssuerKind, Request, Response, SendRequest},
};
use ed25519_dalek::SigningKey;
use serde::{Deserialize, Serialize};
use std::{
    cell::RefCell,
    collections::{BTreeMap, VecDeque},
    io::{BufRead, Write},
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    rc::Rc,
    time::{Duration, Instant},
};
use tokio::io::{AsyncWriteExt, BufReader};

// These are throwaway fixture addresses/keys. No production custody, secret
// file, real primary or process-global environment mutation is involved.
const SEAT: Ulid = Ulid(731);
const SWARM: Ulid = Ulid(732);
struct Provided;
impl bbt_mux::identity::DeviceKeyStore for Provided {
    fn custody(&self) -> bbt_mux::identity::Custody {
        bbt_mux::identity::Custody::Memory
    }
    fn load(&self, _: Ulid) -> bbt_mux::identity::Result<Option<SigningKey>> {
        Ok(Some(SigningKey::from_bytes(&[73; 32])))
    }
    fn create(&self, _: Ulid, _: &SigningKey) -> bbt_mux::identity::Result<()> {
        Err(bbt_mux::identity::PendingReason::JournalConflict)
    }
}

#[derive(Serialize, Deserialize)]
struct Ready {
    path: PathBuf,
    capability: [u8; 32],
}

#[test]
#[ignore = "spawned only by the parent with its private fixture paths"]
fn gui_event_fixture_child() {
    let root = PathBuf::from(std::env::var_os("BBT_GUI_EVENTS_FIXTURE").expect("parent fixture"));
    let capability: [u8; 32] =
        serde_json::from_str(&std::env::var("BBT_GUI_EVENTS_CAP").unwrap()).unwrap();
    let run = root.join("run");
    assert!(matches!(
        bbt_mux::identity::local_dev_issuer_with_store(&run, &Provided),
        bbt_mux::identity::LocalIssuerOutcome::Ready(_)
    ));
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    let logic = Logic::start(
        SocketSupervisor::connect(&root.join("supervisor.sock"), capability).unwrap(),
        LogicConfig {
            run_dir: run,
            repository: root.join("repository"),
            ring: Default::default(),
        },
        SigningKey::from_bytes(&[73; 32]),
        rt.handle(),
    )
    .unwrap();
    if rt.block_on(logic.client.record(SEAT)).is_err() {
        let mut spec = LaunchSpec::new("/bin/zsh", &root);
        spec.seat = SEAT;
        spec.generation = 1;
        spec.args = vec![
            "-f".into(),
            "-c".into(),
            "while IFS= read -r line; do print -r -- GUI-ECHO:$line; done".into(),
        ];
        rt.block_on(
            logic
                .client
                .spawn(SeatRecord::new(SEAT, SWARM, "GUI fixture", "zsh"), spec),
        )
        .unwrap();
        rt.block_on(logic.client.bus.publish(
            format!("swarm-{SWARM}"),
            Draft::new(
                "activity/fixture",
                "fixture/observer",
                serde_json::json!({"prefill":true}),
            ),
        ))
        .unwrap();
    }
    let endpoint = rt.block_on(logic.client.gui_endpoint()).unwrap();
    std::fs::write(
        root.join("ready.tmp"),
        serde_json::to_vec(&Ready {
            path: endpoint.path,
            capability: endpoint.capability,
        })
        .unwrap(),
    )
    .unwrap();
    std::fs::rename(root.join("ready.tmp"), root.join("ready.json")).unwrap();
    for command in std::io::stdin().lock().lines() {
        let command = command.unwrap();
        if command == "stop" {
            break;
        }
        let count: usize = command.strip_prefix("emit ").unwrap().parse().unwrap();
        for _ in 0..count {
            rt.block_on(logic.client.bus.publish("index",Draft::new("artifact/fixture/created","fixture/observer",
                serde_json::json!({"artifact_id":Ulid::new(),"reference":"result.txt","digest":"a".repeat(64)})))).unwrap();
        }
    }
    logic.shutdown().unwrap();
}

// Keep raw subprocess/session diagnostics local, including a libtest panic on
// stdout. A failed boundary must survive TempDir unwinding for investigation.
struct FixtureRoot(Option<tempfile::TempDir>);
impl FixtureRoot {
    fn new() -> Self {
        let parent = std::env::var_os("BBOXY_GUI_EVENTS_EVIDENCE")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/tmp"));
        std::fs::create_dir_all(&parent).unwrap();
        Self(Some(
            tempfile::Builder::new()
                .prefix("gui-events-")
                .tempdir_in(parent)
                .unwrap(),
        ))
    }
    fn path(&self) -> &Path {
        self.0.as_ref().unwrap().path()
    }
}
impl Drop for FixtureRoot {
    fn drop(&mut self) {
        if std::thread::panicking() || std::env::var_os("BBOXY_GUI_EVENTS_EVIDENCE").is_some() {
            eprintln!(
                "gui-events retained private diagnostics: {}",
                self.0.take().unwrap().keep().display()
            );
        }
    }
}

struct Process {
    child: Rc<RefCell<Child>>,
    diagnostics: PathBuf,
    started: Instant,
}
impl Process {
    fn start(root: &Path, capability: [u8; 32]) -> Self {
        let ready = root.join("ready.json");
        if ready.exists() {
            std::fs::remove_file(&ready).unwrap();
        }
        let diagnostics = root.join(format!("logic-{}", Ulid::new()));
        let stdout = std::fs::File::create(diagnostics.with_extension("stdout")).unwrap();
        let stderr = std::fs::File::create(diagnostics.with_extension("stderr")).unwrap();
        let child = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "gui_event_fixture_child",
                "--ignored",
                "--nocapture",
            ])
            .env("BBT_GUI_EVENTS_FIXTURE", root)
            .env(
                "BBT_GUI_EVENTS_CAP",
                serde_json::to_string(&capability).unwrap(),
            )
            .stdin(Stdio::piped())
            .stdout(stdout)
            .stderr(stderr)
            .spawn()
            .unwrap();
        let process = Self {
            child: Rc::new(RefCell::new(child)),
            diagnostics,
            started: Instant::now(),
        };
        process.record("spawned");
        let started = Instant::now();
        while !ready.exists() {
            let status = process.child.borrow_mut().try_wait().unwrap();
            if let Some(status) = status {
                eprintln!(
                    "gui-events fixture exited before Ready: pid={} status={status:?}; diagnostics={}",
                    process.child.borrow().id(),
                    process.diagnostics.display()
                );
                panic!("fixture exited before Ready: {status:?}");
            }
            assert!(
                started.elapsed() < Duration::from_secs(10),
                "fixture Ready timed out"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
        process.record("ready");
        process
    }
    fn record(&self, phase: &str) {
        let mut log = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.diagnostics.with_extension("lifecycle"))
            .unwrap();
        let mut child = self.child.borrow_mut();
        let pid = child.id();
        writeln!(
            log,
            "elapsed_us={} phase={phase} pid={pid} status={:?}",
            self.started.elapsed().as_micros(),
            child.try_wait().unwrap()
        )
        .unwrap();
    }
    fn emit(&mut self, count: usize) {
        writeln!(
            self.child.borrow_mut().stdin.as_mut().unwrap(),
            "emit {count}"
        )
        .unwrap();
    }
    fn kill(&mut self) {
        self.record("before-kill");
        self.child.borrow_mut().kill().unwrap();
        let status = self.child.borrow_mut().wait().unwrap();
        self.record(&format!("killed exit={status:?}"));
    }
    fn stop(&mut self) {
        writeln!(self.child.borrow_mut().stdin.as_mut().unwrap(), "stop").unwrap();
        let start = Instant::now();
        loop {
            if let Some(status) = self.child.borrow_mut().try_wait().unwrap() {
                assert!(status.success());
                break;
            }
            assert!(
                start.elapsed() < Duration::from_secs(10),
                "fixture shutdown timed out"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
        self.record("stopped");
    }
}
impl Drop for Process {
    fn drop(&mut self) {
        let mut child = self.child.borrow_mut();
        if child.try_wait().ok().flatten().is_none() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

struct Gui {
    connection: Ulid,
    started: Instant,
    child: Rc<RefCell<Child>>,
    diagnostics: PathBuf,
    last_action: String,
    received: usize,
    read: BufReader<tokio::net::unix::OwnedReadHalf>,
    write: tokio::net::unix::OwnedWriteHalf,
    capability: [u8; 32],
    deferred: VecDeque<GuiReply>,
}
impl Gui {
    async fn open(root: &Path, process: &Process) -> Self {
        let ready: Ready =
            serde_json::from_slice(&std::fs::read(root.join("ready.json")).unwrap()).unwrap();
        let (read, write) = tokio::net::UnixStream::connect(ready.path)
            .await
            .unwrap()
            .into_split();
        let gui = Self {
            connection: Ulid::new(),
            started: process.started,
            child: process.child.clone(),
            diagnostics: process.diagnostics.clone(),
            last_action: String::new(),
            received: 0,
            read: BufReader::new(read),
            write,
            capability: ready.capability,
            deferred: VecDeque::new(),
        };
        gui.record("connected");
        gui
    }
    fn record(&self, phase: &str) {
        let mut child = self.child.borrow_mut();
        let pid = child.id();
        let status = child.try_wait().unwrap();
        let mut log = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.diagnostics.with_extension("gui"))
            .unwrap();
        writeln!(
            log,
            "elapsed_us={} connection={} phase={phase} pid={pid} status={status:?} received={} last_action={}",
            self.started.elapsed().as_micros(), self.connection, self.received, self.last_action
        )
        .unwrap();
    }
    async fn send(&mut self, action: GuiAction) {
        // Only operation kind/cursor coordinates, never capability or payload.
        self.last_action = match &action {
            GuiAction::EventAck { stream, seq } => format!("event_ack stream={stream} seq={seq}"),
            GuiAction::MarkEventsRead { stream, seq } => {
                format!("mark_read stream={stream} seq={seq}")
            }
            GuiAction::GetEventCursor { stream } => format!("cursor stream={stream}"),
            _ => serde_json::to_value(&action).unwrap()["kind"]
                .as_str()
                .unwrap_or("control")
                .into(),
        };
        self.record("send");
        let mut bytes = serde_json::to_vec(&GuiRequest {
            version: bbt_proto::gui::VERSION,
            capability: self.capability,
            action,
        })
        .unwrap();
        bytes.push(b'\n');
        self.write.write_all(&bytes).await.unwrap();
    }
    async fn until(&mut self, accept: impl Fn(&GuiReply) -> bool) -> GuiReply {
        let mut staged = VecDeque::new();
        let mut bytes = 0;
        let start = Instant::now();
        loop {
            assert!(
                start.elapsed() < Duration::from_secs(10),
                "GUI fixture reply deadline"
            );
            let reply = if let Some(reply) = self.deferred.pop_front() {
                reply
            } else {
                let result = tokio::time::timeout(
                    Duration::from_secs(5),
                    bbt_mux::seat_socket::read_line(&mut self.read),
                )
                .await;
                let line = match result {
                    Ok(Ok(Some(line))) => line,
                    other => {
                        self.record(&format!("read-failed result={other:?}"));
                        panic!(
                            "GUI boundary read failed; diagnostics={}; result={other:?}",
                            self.diagnostics.display()
                        );
                    }
                };
                self.received += 1;
                let reply: GuiReply = serde_json::from_slice(&line).unwrap();
                match &reply {
                    GuiReply::Identity(identity) => {
                        self.record(&format!("identity state={:?}", identity.state))
                    }
                    GuiReply::EventCursor { cursor } => self.record(&format!("cursor {cursor:?}")),
                    GuiReply::Event { stream, seq, .. } => {
                        self.record(&format!("event stream={stream} seq={seq}"))
                    }
                    GuiReply::EventStatus { attached, .. } => {
                        self.record(&format!("event-status attached={attached}"))
                    }
                    GuiReply::Update { .. } => self.record("pty-update"),
                    _ => {}
                }
                reply
            };
            if accept(&reply) {
                for old in staged.into_iter().rev() {
                    self.deferred.push_front(old);
                }
                return reply;
            }
            // Admission metadata and completed transport OKs need no fixture cache.
            if matches!(reply, GuiReply::Identity(_) | GuiReply::Ok { .. }) {
                continue;
            }
            bytes += serde_json::to_vec(&reply).unwrap().len();
            assert!(
                staged.len() < 128 && bytes <= 256 * 1024,
                "fixture staging cap"
            );
            staged.push_back(reply);
        }
    }
    async fn attach(&mut self) {
        let id = Ulid::new();
        self.send(GuiAction::AttachEvents { id }).await;
        match self
            .until(|r| matches!(r,GuiReply::Ok {op}|GuiReply::Error {op,..} if *op==id))
            .await
        {
            GuiReply::Ok { .. } => (),
            GuiReply::Error { reason, .. } => panic!("attach refused: {reason}"),
            _ => unreachable!(),
        }
    }
    async fn cursor(&mut self, stream: &str) -> bbt_proto::events::GuiEventCursor {
        self.send(GuiAction::GetEventCursor {
            stream: stream.into(),
        })
        .await;
        match self
            .until(|r| matches!(r,GuiReply::EventCursor {cursor} if cursor.stream==stream))
            .await
        {
            GuiReply::EventCursor { cursor } => cursor,
            _ => unreachable!(),
        }
    }
    async fn messaging(&mut self, request: Request) -> Response {
        let id = match &request {
            Request::Send { message } => message.id,
            _ => Ulid::new(),
        };
        self.send(GuiAction::Messaging {
            id,
            swarm_id: bbt_proto::SwarmId::from_u128(SWARM.0),
            request,
        })
        .await;
        match self
            .until(|r| matches!(r,GuiReply::Messaging {op,..}|GuiReply::Error {op,..} if *op==id))
            .await
        {
            GuiReply::Messaging { response, .. } => response,
            GuiReply::Error { reason, .. } => panic!("message fixture refused: {reason}"),
            _ => unreachable!(),
        }
    }
    async fn artifact(&mut self) -> (String, u64, String) {
        match self.until(|r|matches!(r,GuiReply::Event {stream,subject,..} if stream=="index"&&subject=="artifact/fixture/created")).await {
            GuiReply::Event {stream,seq,envelope,..}=>(stream,seq,envelope.metadata.reference.hash),_=>unreachable!(),
        }
    }
}

#[test]
fn durable_gui_survives_real_logic_kill_keeps_unread_and_services_pty_controls() {
    let root = FixtureRoot::new();
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    std::fs::create_dir_all(root.path().join("repository/.bboxy")).unwrap();
    let supervisor = Supervisor::start(Config {
        socket_path: Some(root.path().join("supervisor.sock")),
        journal_directory: Some(root.path().join("journal")),
        ..Default::default()
    })
    .unwrap();
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    let mut process = Process::start(root.path(), supervisor.socket_capability());
    rt.block_on(async {
        let mut gui=Gui::open(root.path(), &process).await;gui.attach().await;
        // One durable consumer, and a second connection cannot drain it.
        let mut second=Gui::open(root.path(), &process).await;let id=Ulid::new();second.send(GuiAction::AttachEvents {id}).await;
        assert!(matches!(second.until(|r|matches!(r,GuiReply::Error {op,..} if *op==id)).await,
            GuiReply::Error {reason,..} if reason.starts_with("GuiConsumerAttached:")));
        drop(second);
        let message=SendRequest::text("operator/inbox","retained fixture message");let message_id=message.id;
        assert!(matches!(gui.messaging(Request::Send {message}).await,Response::Stored {..}));
        let (stream,seq)=match gui.until(|r|matches!(r,GuiReply::Event {subject,envelope,..} if subject.ends_with("/message")&&matches!(&envelope.payload,GuiEventPayload::Message {row} if row.id==message_id))).await {
            GuiReply::Event {stream,seq,envelope,..}=> {
                let GuiEventPayload::Message {row}=envelope.payload else {unreachable!()};
                assert_eq!(row.issuer_kind,Some(IssuerKind::LocalDev));
                let position=row.bus.expect("canonical bus publication");assert_eq!(position.stream,stream);assert_eq!(position.seq,seq);
                (stream,seq)
            },_=>unreachable!(),
        };
        gui.send(GuiAction::EventAck {stream:stream.clone(),seq}).await;
        assert_eq!(gui.cursor(&stream).await.unread,1,"transport publication is not an operator read");
        gui.send(GuiAction::MarkEventsRead {stream:stream.clone(),seq}).await;
        let mark=gui.until(|r|matches!(r,GuiReply::EventCursor {cursor} if cursor.stream==stream&&cursor.read==seq)).await;
        assert!(matches!(mark,GuiReply::EventCursor {cursor} if cursor.unread==0));
        let Response::Messages {page}=gui.messaging(Request::List {thread:None,channel:None,after:0,limit:1}).await else {panic!("expected inbox page")};
        assert_eq!(page.items.len(),1);let first_message=&page.items[0];
        let signed=first_message.envelope().expect("signed fixture envelope");
        signed.verify_signature().unwrap();assert_eq!(signed.id,message_id);
        let position=first_message.bus.as_ref().unwrap();assert_eq!(position.stream,stream);assert_eq!(position.seq,seq);
        assert_ne!(first_message.sequence,seq,"fixture exercises separate ledger and bus domains");
        let ledger_cursor=first_message.sequence;
        let message=SendRequest::text("operator/inbox","unread across restart");let second_id=message.id;
        assert!(matches!(gui.messaging(Request::Send {message}).await,Response::Stored {..}));
        let second_sequence=match gui.until(|r|matches!(r,GuiReply::Event {subject,envelope,..} if subject.ends_with("/message")&&matches!(&envelope.payload,GuiEventPayload::Message {row} if row.id==second_id))).await {
            GuiReply::Event {seq,..}=>seq,_=>unreachable!(),
        };
        gui.send(GuiAction::EventAck {stream:stream.clone(),seq:second_sequence}).await;
        let Response::Messages {page}=gui.messaging(Request::List {thread:None,channel:None,after:ledger_cursor,limit:1}).await else {panic!("expected next inbox page")};
        assert_eq!(page.items.len(),1);let signed=page.items[0].envelope().expect("signed fixture envelope");
        signed.verify_signature().unwrap();assert_eq!(signed.id,second_id);
        assert_eq!(page.items[0].bus.as_ref().unwrap().seq,second_sequence);
        assert_eq!(gui.cursor(&stream).await.unread,1);
        process.emit(2);
        let first=gui.artifact().await;let second=gui.artifact().await;
        let mut published=BTreeMap::from([((first.0.clone(),first.1),first.2),((second.0.clone(),second.1),second.2.clone())]);
        gui.send(GuiAction::EventAck {stream:first.0,seq:first.1}).await;
        gui.until(|r|matches!(r,GuiReply::Ok {..})).await;
        // The second row was published, but its ACK is lost with the process.
        process.kill();drop(gui);
        process=Process::start(root.path(),supervisor.socket_capability());
        let mut gui=Gui::open(root.path(), &process).await;gui.attach().await;
        let replay=gui.artifact().await;assert_eq!((replay.0.as_str(),replay.1), (second.0.as_str(),second.1));
        assert_eq!(published.insert((replay.0.clone(),replay.1),replay.2),Some(second.2));
        assert_eq!(published.len(),2,"at-least-once replay does not add a model row");
        assert_eq!(gui.cursor(&stream).await.read,seq,"operator read survives real logic restart");
        assert_eq!(gui.cursor(&stream).await.unread,1,"receiving/ACKing the second message never marked it viewed");
        let catalog=SqliteStore::open(&root.path().join("run/bus")).unwrap();
        assert_eq!(catalog.list("consumer/gui-").unwrap().len(),1);

        process.emit(40);
        let seat=SeatId::from_u128(SEAT.0);let view=View {seat:SEAT,generation:1,view_generation:Ulid::new()};
        gui.send(GuiAction::Watch {seat,view}).await;
        let reply=gui.until(|r|matches!(r,GuiReply::Update {..})).await;
        let mut replica=bbt_mux::streams::Replica::default();
        let GuiReply::Update {update,..}=reply else {unreachable!()};
        replica.apply(update).unwrap();
        let baseline=replica.snapshot.as_ref().unwrap();let (view,sequence)=(baseline.view,baseline.sequence);
        gui.send(GuiAction::Applied {seat,view,sequence}).await;
        let input=Ulid::new();let started=Instant::now();
        gui.send(GuiAction::Input {seat,generation:1,id:input,bytes:b"alive\n".to_vec()}).await;
        assert!(matches!(gui.until(|r|matches!(r,GuiReply::Ok {op}|GuiReply::Error {op,..} if *op==input)).await,GuiReply::Ok {..}));
        gui.send(GuiAction::Resize {seat,generation:1,cols:66,rows:22}).await;
        loop {
            let GuiReply::Update {update,..}=gui.until(|r|matches!(r,GuiReply::Update {..})).await else {unreachable!()};
            replica.apply(update).unwrap();let current=replica.snapshot.as_ref().unwrap();
            gui.send(GuiAction::Applied {seat,view:current.view,sequence:current.sequence}).await;
            let text:String=current.screen.cells.iter().flatten().map(|cell|cell.c).collect();
            if current.screen.cols==66&&current.screen.rows==22&&text.contains("GUI-ECHO:alive") {break;}
        }
        for _ in 0..40 {let row=gui.artifact().await;assert!(published.insert((row.0,row.1),row.2).is_none());}
        assert_eq!(published.len(),42);
        eprintln!("gui-events real kill/restart: 1 consumer; 1 exact replay; 42 unique artifact rows; read watermark={seq}; input/resize serviced during events in {:?}",started.elapsed());
        gui.send(GuiAction::GetEvents {stream:"../foreign".into(),after:0,limit:128}).await;
        assert!(matches!(gui.until(|r|matches!(r,GuiReply::Error {..})).await,GuiReply::Error {reason,..} if reason.contains("unknown GUI stream")));
        drop(gui);
    });
    process.stop();
    supervisor.shutdown().unwrap();
}

// keeps: task 183 — retain failed GUI/session diagnostics at the restart boundary.
// A read failure used to take the clean-disconnect branch in gui_session,
// destroying the only explanation for the peer's EOF. Exercise the real owner
// after a kill/reopen, without timing assumptions, retries or operator access.
#[test]
fn restarted_gui_retains_truncated_request_diagnostics_before_eof() {
    let root = FixtureRoot::new();
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    std::fs::create_dir_all(root.path().join("repository/.bboxy")).unwrap();
    let supervisor = Supervisor::start(Config {
        socket_path: Some(root.path().join("supervisor.sock")),
        journal_directory: Some(root.path().join("journal")),
        ..Default::default()
    })
    .unwrap();
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    let mut process = Process::start(root.path(), supervisor.socket_capability());
    process.kill();
    process = Process::start(root.path(), supervisor.socket_capability());
    rt.block_on(async {
        let mut gui = Gui::open(root.path(), &process).await;
        // EOF is intentional here: the request ends halfway through a frame.
        gui.write.write_all(b"{").await.unwrap();
        gui.write.shutdown().await.unwrap();
        let end = tokio::time::timeout(
            Duration::from_secs(5),
            bbt_mux::seat_socket::read_line(&mut gui.read),
        )
        .await
        .unwrap()
        .unwrap();
        assert!(
            end.is_none(),
            "truncated frame must close its own connection"
        );
        let stderr = std::fs::read_to_string(process.diagnostics.with_extension("stderr")).unwrap();
        assert!(
            stderr.contains("GUI boundary failed:"),
            "missing failed boundary diagnostics: {stderr}"
        );
        assert!(
            stderr.contains("phase=request-read"),
            "missing failing owner phase: {stderr}"
        );
        assert!(
            stderr.contains("admitted=false events_attached=false"),
            "incorrect admission diagnostic: {stderr}"
        );
        assert!(
            stderr.contains("I/O error (InvalidData, os=None)"),
            "missing frame error category: {stderr}"
        );
        assert!(!stderr.contains("capability"));
        assert!(
            process.child.borrow_mut().try_wait().unwrap().is_none(),
            "session failure must leave the logic process alive"
        );
        let mut next = Gui::open(root.path(), &process).await;
        next.attach().await;
        drop(next);
    });
    process.stop();
    supervisor.shutdown().unwrap();
}

#[test]
fn retained_message_and_receipt_pages_hydrate_rows_without_replaying_seat_state() {
    let root = FixtureRoot::new();
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    std::fs::create_dir_all(root.path().join("repository/.bboxy")).unwrap();
    // Adapted from bboxy450-term's retained GUI fixture: preserve historical
    // memberships while one real seat keeps canonical message/receipt delivery.
    // Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
    let registry = bbt_mux::registry::Registry::new(
        SqliteStore::open(&root.path().join("run/account")).unwrap(),
    );
    for i in 0..42_u128 {
        let mut seat = SeatRecord::new(
            Ulid::from(10_000 + i),
            Ulid::from(SWARM.0 + i % 7),
            "retained",
            "zsh",
        );
        seat.lifecycle = bbt_mux::registry::Lifecycle::ClosedRetained;
        registry.insert(&seat).unwrap();
    }
    let supervisor = Supervisor::start(Config {
        socket_path: Some(root.path().join("supervisor.sock")),
        journal_directory: Some(root.path().join("journal")),
        ..Default::default()
    })
    .unwrap();
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    let mut process = Process::start(root.path(), supervisor.socket_capability());
    rt.block_on(async {
        let mut gui = Gui::open(root.path(), &process).await;
        gui.attach().await;
        assert_eq!(registry.list().unwrap().len(), 43, "108 exact GUI subjects across seven swarms, including canonical inboxes");
        let message = SendRequest::text("operator/inbox", "latest retained message body");
        let message_id = message.id;
        assert!(matches!(gui.messaging(Request::Send { message }).await, Response::Stored { .. }));
        let message = match gui.until(|reply| matches!(reply,
            GuiReply::Event {subject,envelope,..} if subject.ends_with("/message")
                && matches!(&envelope.payload, GuiEventPayload::Message {row} if row.id == message_id)
        )).await {
            GuiReply::Event {envelope,..} => *envelope,
            _ => unreachable!(),
        };
        let message_position = message.metadata.reference.position.clone();
        gui.send(GuiAction::EventAck {stream:message_position.stream.clone(), seq:message_position.sequence}).await;
        assert!(matches!(gui.messaging(Request::Acknowledge {message:message_id,agent:None}).await, Response::Applied));
        let receipt = match gui.until(|reply| matches!(reply,
            GuiReply::Event {subject,envelope,..} if subject.ends_with("/receipt")
                && matches!(&envelope.payload, GuiEventPayload::Message {row} if row.id == message_id)
        )).await {
            GuiReply::Event {envelope,..} => *envelope,
            _ => unreachable!(),
        };
        assert_ne!(receipt.metadata.reference.position, message_position);
        gui.send(GuiAction::EventAck {stream:receipt.metadata.reference.position.stream.clone(), seq:receipt.metadata.reference.position.sequence}).await;
        for expected in [message, receipt] {
            let position = &expected.metadata.reference.position;
            let before = gui.cursor(&position.stream).await;
            gui.send(GuiAction::GetEvents {
                stream: position.stream.clone(),
                after: position.sequence.saturating_sub(1),
                limit: 1,
            }).await;
            let GuiReply::Events {page} = gui.until(|reply| matches!(reply,
                GuiReply::Events {page} if page.cursor.stream == position.stream
            )).await else { unreachable!() };
            assert_eq!(page.records.len(), 1);
            assert!(serde_json::to_vec(&page).unwrap().len() <= 256 * 1024);
            assert_eq!(page.cursor.read, before.read, "retained projection never marks read");
            let bbt_proto::events::GuiEventRecord::Event(actual) = &page.records[0] else {
                panic!("bounded fixture record unexpectedly became a gap");
            };
            assert_eq!(actual.metadata, expected.metadata, "canonical position and proof unchanged");
            assert!(actual.cursor.is_none());
                let GuiEventPayload::Message {row} = &actual.payload else {
                    panic!("retained message/receipt did not hydrate MessageRow");
                };
                assert_eq!(row.id, message_id);
                assert_eq!(row.text, "latest retained message body");
                assert_eq!(row.issuer_kind, Some(IssuerKind::LocalDev));
                assert_eq!(row.signature, bbt_proto::messaging::SignatureState::Signed);
                let bus = row.bus.as_ref().unwrap();
                assert_eq!(bus.stream, message_position.stream);
                assert_eq!(bus.seq, message_position.sequence, "receipt is not the message position");
            assert_eq!(gui.cursor(&position.stream).await.read, before.read);
        }
        let read = Ulid::new();
        gui.send(GuiAction::EventsRead {
            id: read,
            stream: message_position.stream.clone(),
            after: message_position.sequence - 1,
            limit: 1,
            wait_ms: None,
        }).await;
        let GuiReply::EventsRead {events, through, ..} = gui.until(|reply| matches!(reply,
            GuiReply::EventsRead {op,..} if *op == read
        )).await else {unreachable!()};
        assert_eq!(events.len(), 1, "ctl retained reads use the same >64 authorized subjects");
        assert_eq!(events[0].seq, message_position.sequence);
        assert_eq!(through, message_position.sequence);
        assert_eq!(serde_json::from_value::<Ulid>(events[0].payload["id"].clone()).unwrap(), message_id);

        // The seat started before attachment, so its original state is retained
        // history rather than a promised new live transition.
        let stream = format!("seat-{SEAT}");
        let before = gui.cursor(&stream).await;
        gui.send(GuiAction::GetEvents {stream:stream.clone(),after:0,limit:128}).await;
        let GuiReply::Events {page} = gui.until(|reply| matches!(reply,
            GuiReply::Events {page} if page.cursor.stream == stream
        )).await else { unreachable!() };
        assert_eq!(page.cursor.read,before.read);
        assert!(page.records.iter().any(|record| matches!(record,
            bbt_proto::events::GuiEventRecord::Event(event) if event.metadata.subject.ends_with("/state")
                && matches!(event.payload,GuiEventPayload::Activity {..})
        )), "retained seat state remains activity");
        assert!(page.records.iter().all(|record| !matches!(record,
            bbt_proto::events::GuiEventRecord::Event(event) if matches!(event.payload,
                GuiEventPayload::Seat {..}|GuiEventPayload::Identity {..}|GuiEventPayload::Host {..})
        )));
        // #73: a receipt can hydrate a row even when its direct publication is
        // missing from the catalogue. Require the canonical message subject,
        // not merely any event carrying this message ID.
        let destination = format!("seats/{SEAT}/inbox");
        let message = SendRequest::text(&destination, "direct canonical retained message");
        let direct_id = message.id;
        assert!(matches!(gui.messaging(Request::Send {message}).await, Response::Stored {..}));
        let (stream, seq, row) = match gui.until(|reply| matches!(reply,
            GuiReply::Event {subject,envelope,..} if subject == &format!("{destination}/message")
                && matches!(&envelope.payload, GuiEventPayload::Message {row} if row.id == direct_id)
        )).await {
            GuiReply::Event {stream,seq,envelope,..} => {
                let GuiEventPayload::Message {row} = envelope.payload else {unreachable!()};
                (stream, seq, row)
            },
            _ => unreachable!(),
        };
        assert_eq!(row.text, "direct canonical retained message");
        assert!(row.receipts.delivered > 0, "canonical projection includes actual recipient storage");
        assert_eq!(row.receipts.acked, 0, "transport is not model consumption");
        assert_eq!(row.bus.as_ref().unwrap().seq, seq);
        let read = gui.cursor(&stream).await.read;
        gui.send(GuiAction::EventAck {stream:stream.clone(), seq}).await;
        assert_eq!(gui.cursor(&stream).await.read, read, "publication ACK never marks read");
        drop(gui);
    });
    process.stop();
    supervisor.shutdown().unwrap();
}
