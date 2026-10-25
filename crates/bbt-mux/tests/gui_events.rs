#![allow(
    clippy::disallowed_methods,
    reason = "test/fixture IO, not the UI thread"
)]

use bbt_mux::{
    Config, LaunchSpec, Supervisor, Ulid,
    logic::{Logic, LogicConfig, SocketSupervisor},
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
    collections::{BTreeMap, VecDeque},
    io::{BufRead, Write},
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
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

struct Process(Child);
impl Process {
    fn start(root: &Path, capability: [u8; 32]) -> Self {
        let ready = root.join("ready.json");
        if ready.exists() {
            std::fs::remove_file(&ready).unwrap();
        }
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
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();
        eprintln!("gui-events owned logic fixture pid={}", child.id());
        let mut process = Self(child);
        let started = Instant::now();
        while !ready.exists() {
            assert!(
                process.0.try_wait().unwrap().is_none(),
                "fixture exited before Ready"
            );
            assert!(
                started.elapsed() < Duration::from_secs(10),
                "fixture Ready timed out"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
        process
    }
    fn emit(&mut self, count: usize) {
        writeln!(self.0.stdin.as_mut().unwrap(), "emit {count}").unwrap();
    }
    fn kill(&mut self) {
        self.0.kill().unwrap();
        self.0.wait().unwrap();
    }
    fn stop(&mut self) {
        writeln!(self.0.stdin.as_mut().unwrap(), "stop").unwrap();
        let start = Instant::now();
        loop {
            if let Some(status) = self.0.try_wait().unwrap() {
                assert!(status.success());
                return;
            }
            assert!(
                start.elapsed() < Duration::from_secs(10),
                "fixture shutdown timed out"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}
impl Drop for Process {
    fn drop(&mut self) {
        if self.0.try_wait().ok().flatten().is_none() {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
}

struct Gui {
    read: BufReader<tokio::net::unix::OwnedReadHalf>,
    write: tokio::net::unix::OwnedWriteHalf,
    capability: [u8; 32],
    deferred: VecDeque<GuiReply>,
}
impl Gui {
    async fn open(root: &Path) -> Self {
        let ready: Ready =
            serde_json::from_slice(&std::fs::read(root.join("ready.json")).unwrap()).unwrap();
        let (read, write) = tokio::net::UnixStream::connect(ready.path)
            .await
            .unwrap()
            .into_split();
        Self {
            read: BufReader::new(read),
            write,
            capability: ready.capability,
            deferred: VecDeque::new(),
        }
    }
    async fn send(&mut self, action: GuiAction) {
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
                let line = tokio::time::timeout(
                    Duration::from_secs(5),
                    bbt_mux::seat_socket::read_line(&mut self.read),
                )
                .await
                .unwrap()
                .unwrap()
                .unwrap();
                serde_json::from_slice(&line).unwrap()
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
    let root = tempfile::Builder::new()
        .prefix("gui-events-")
        .tempdir_in("/tmp")
        .unwrap();
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
        let mut gui=Gui::open(root.path()).await;gui.attach().await;
        // One durable consumer, and a second connection cannot drain it.
        let mut second=Gui::open(root.path()).await;let id=Ulid::new();second.send(GuiAction::AttachEvents {id}).await;
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
        assert_eq!(first_message.envelope().expect("signed fixture envelope").id,message_id);
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
        assert_eq!(page.items.len(),1);assert_eq!(page.items[0].envelope().expect("signed fixture envelope").id,second_id);
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
        let mut gui=Gui::open(root.path()).await;gui.attach().await;
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
