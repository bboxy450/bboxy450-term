//! Owner-only, capability-bound seat observation and semantic-control socket.
//! Raw input is unavailable here; messaging uses the common adapter/guard broker.

use std::collections::{BTreeMap, BTreeSet};
use std::os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncWriteExt, BufReader};
use ulid::Ulid;

use crate::bus::Consumer;
use crate::logic::LogicClient;
use crate::store::{Result, invalid};
use crate::streams::{View, ViewStream};

const MAX_LINE: usize = 1024 * 1024;

pub use bbt_proto::{GuiAction, GuiReply, GuiRequest, GuiSeat};

#[derive(Debug, Serialize, Deserialize)]
pub struct Request {
    pub version: u16,
    pub capability: [u8; 32],
    pub seat: Ulid,
    pub generation: u64,
    pub action: Action,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum Action {
    Catalogue,
    Call {
        #[serde(flatten)]
        call: crate::mcp::Call,
    },
    Messaging {
        request: crate::messaging::api::Request,
    },
    Hook {
        event: crate::adapters::hook::ShimEvent,
    },
    Attach,
    Snapshot,
    Subscribe,
    Applied {
        view: View,
        sequence: u64,
    },
    GetHistory {
        from: u64,
        to: u64,
    },
    BusSubscribe {
        consumer: Consumer,
    },
    PollEvents {
        consumer: String,
        limit: usize,
    },
    Ack {
        consumer: String,
        stream: String,
        sequence: u64,
    },
}

struct SocketFile {
    path: PathBuf,
    dev: u64,
    ino: u64,
}
impl Drop for SocketFile {
    fn drop(&mut self) {
        if let Ok(meta) = std::fs::symlink_metadata(&self.path)
            && meta.dev() == self.dev
            && meta.ino() == self.ino
        {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}
pub struct SeatEndpoint {
    task: tokio::task::JoinHandle<()>,
    _file: SocketFile,
}
impl Drop for SeatEndpoint {
    fn drop(&mut self) {
        self.task.abort();
    }
}

impl SeatEndpoint {
    /// One app connection multiplexes only sidebar state for every seat.
    pub fn bind_gui(
        handle: &tokio::runtime::Handle,
        path: &Path,
        client: LogicClient,
        capability: [u8; 32],
    ) -> Result<Self> {
        bbt_rt::assert_not_ui_thread();
        let (listener, file) = bind_listener(handle, path)?;
        let task = handle.spawn(async move {
            let slots = Arc::new(tokio::sync::Semaphore::new(32));
            let mut peers = tokio::task::JoinSet::new();
            loop {
                tokio::select! {
                    accepted=listener.accept()=> {
                        let Ok((stream,_))=accepted else {break;};
                        let Ok(permit)=slots.clone().try_acquire_owned() else {continue;};
                        let client=client.clone();
                        peers.spawn(async move {
                            let _permit=permit;
                            let _=gui_session(stream,client,capability).await;
                        });
                    }
                    _=peers.join_next(),if !peers.is_empty()=>{}
                }
            }
        });
        Ok(Self { task, _file: file })
    }
    /// Bind during IO bootstrap. Each endpoint's capability grants observations
    /// of one generation only, not a grant to another seat or an input lease.
    pub fn bind(
        handle: &tokio::runtime::Handle,
        path: &Path,
        client: LogicClient,
        seat: Ulid,
        generation: u64,
        capability: [u8; 32],
    ) -> Result<Self> {
        bbt_rt::assert_not_ui_thread();
        let (listener, file) = bind_listener(handle, path)?;
        let task=handle.spawn(async move {
            let slots=Arc::new(tokio::sync::Semaphore::new(32));
            let mut peers=tokio::task::JoinSet::new();
            loop {
                tokio::select! {
                    accepted=listener.accept()=> {
                        let Ok((stream,_))=accepted else {break;};
                        let Ok(permit)=slots.clone().try_acquire_owned() else {continue;};
                        let client=client.clone();
                        peers.spawn(async move {let _permit=permit;let _=session(stream,client,seat,generation,capability).await;});
                    }
                    _=peers.join_next(),if !peers.is_empty()=>{}
                }
            }
        });
        Ok(Self { task, _file: file })
    }
}

fn bind_listener(
    handle: &tokio::runtime::Handle,
    path: &Path,
) -> Result<(tokio::net::UnixListener, SocketFile)> {
    let parent = path
        .parent()
        .ok_or_else(|| invalid("socket requires a parent"))?;
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(parent)?;
    let meta = std::fs::symlink_metadata(parent)?;
    // SAFETY: geteuid takes no pointers and changes no state.
    if !meta.is_dir() || meta.uid() != unsafe { libc::geteuid() } || meta.mode() & 0o077 != 0 {
        return Err(invalid("seat socket parent must be private and owned"));
    }
    let listener = std::os::unix::net::UnixListener::bind(path)?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    let meta = std::fs::symlink_metadata(path)?;
    let file = SocketFile {
        path: path.into(),
        dev: meta.dev(),
        ino: meta.ino(),
    };
    listener.set_nonblocking(true)?;
    let _entered = handle.enter();
    Ok((tokio::net::UnixListener::from_std(listener)?, file))
}

struct GuiView {
    stream: ViewStream,
    generation: u64,
    last: Option<(View, u64)>,
}
async fn gui_session(
    stream: tokio::net::UnixStream,
    client: LogicClient,
    capability: [u8; 32],
) -> Result<()> {
    let (read, mut write) = stream.into_split();
    let (lines, mut incoming) = tokio::sync::mpsc::channel(2);
    let task = tokio::spawn(async move {
        let mut reader = BufReader::new(read);
        loop {
            let line = read_line(&mut reader).await;
            let done = !matches!(line, Ok(Some(_)));
            if lines.send(line).await.is_err() || done {
                break;
            }
        }
    });
    struct Reader(tokio::task::JoinHandle<()>);
    impl Drop for Reader {
        fn drop(&mut self) {
            self.0.abort();
        }
    }
    let _reader = Reader(task);
    let mut consumer = None;
    let mut views: BTreeMap<bbt_proto::SeatId, GuiView> = BTreeMap::new();
    let mut tick = tokio::time::interval(Duration::from_millis(20));
    let result:Result<()>=async {
        loop {
            tokio::select! {
                line=incoming.recv()=> {
                    let Some(Ok(Some(line)))=line else {break;};
                    let request:GuiRequest=serde_json::from_slice(&line)?;
                    if request.version!=1||request.capability!=capability {
                        return Err(invalid("GUI socket admission rejected"));
                    }
                    if consumer.is_none() {
                        // Subscribe before ListSeats: no state transition can
                        // disappear between the initial list and live events.
                        let mut subscription=Consumer::gui("seat/*/state");
                        subscription.filter.push("account/identity/state".into());
                        consumer=Some(client.bus.subscribe(subscription).await?);
                        send(&mut write,&GuiReply::Identity(client.gui_identity().await?)).await?;
                    }
                    let op=match &request.action {GuiAction::Input {id,..}=>*id,_=>Ulid::new()};
                    let reply:Result<GuiReply>=async {
                        match request.action {
                            GuiAction::ListSeats=>Ok(GuiReply::Seats(client.gui_seats().await?)),
                            GuiAction::Spawn {spec}=>Ok(GuiReply::Seat(client.gui_spawn(spec).await?)),
                            GuiAction::Watch {seat,view}=> {
                                if view.seat!=Ulid::from(seat.0.0) {return Err(invalid("watch seat and view disagree"));}
                                if views.len()>=1024&&!views.contains_key(&seat) {return Err(invalid("watch limit reached"));}
                                let snapshot=client.snapshot(view.seat,view.generation).await?;
                                let last=Some((snapshot.view,snapshot.sequence));
                                let mut stream=ViewStream::default();stream.offer(snapshot)?;
                                views.insert(seat,GuiView {stream,generation:view.generation,last});
                                Ok(GuiReply::Ok {op})
                            }
                            GuiAction::Unwatch {seat}=>{views.remove(&seat);Ok(GuiReply::Ok {op})}
                            GuiAction::Applied {seat,view,sequence}=> {
                                if view.seat!=Ulid::from(seat.0.0) {return Err(invalid("ack seat and view disagree"));}
                                views.get_mut(&seat).ok_or_else(||invalid("seat is not watched"))?.stream.applied(view,sequence)?;
                                Ok(GuiReply::Ok {op})
                            }
                            GuiAction::Input {seat,id,bytes}=> {
                                let receipt=client.gui_input(Ulid::from(seat.0.0),id,bytes).await?;
                                if receipt.state==crate::InputState::PartialUnknown {return Err(invalid("input result is partial/unknown; keep the original id"));}
                                Ok(GuiReply::Ok {op:id})
                            }
                            GuiAction::Resize {seat,cols,rows}=> {
                                client.gui_resize(Ulid::from(seat.0.0),cols,rows).await?;
                                Ok(GuiReply::Ok {op})
                            }
                            GuiAction::ScheduleClose {..}|GuiAction::UndoClose {..}|GuiAction::GetHistory {..}=>Err(invalid("close, undo and history are not connected until C1c")),
                        }
                    }.await;
                    send(&mut write,&reply.unwrap_or_else(|e|GuiReply::Error {op,reason:e.to_string()})).await?;
                }
                _=tick.tick(),if consumer.is_some()=> {
                    for event in client.bus.poll(consumer.as_deref().unwrap(),128).await? {
                        match event {
                            crate::bus::Delivery::Event {envelope,..}=> {
                                if envelope.event.subject=="account/identity/state" {
                                    send(&mut write,&GuiReply::Identity(client.gui_identity().await?)).await?;
                                    continue;
                                }
                                let id=envelope.event.subject.split('/').nth(1).ok_or_else(||invalid("state event has no seat"))?.parse()?;
                                if let Ok(seat)=client.gui_seat(id).await {send(&mut write,&GuiReply::Seat(seat)).await?;}
                            }
                            crate::bus::Delivery::Gap {..}=> {
                                send(&mut write,&GuiReply::Identity(client.gui_identity().await?)).await?;
                                send(&mut write,&GuiReply::Seats(client.gui_seats().await?)).await?;
                            }
                        }
                    }
                    let mut stale=Vec::new();
                    for (&seat,watched) in &mut views {
                        match client.snapshot(Ulid::from(seat.0.0),watched.generation).await {
                            Ok(snapshot)=> {
                                let identity=Some((snapshot.view,snapshot.sequence));
                                if identity!=watched.last {watched.stream.offer(snapshot)?;watched.last=identity;}
                                if let Some(update)=watched.stream.next_update() {
                                    send(&mut write,&GuiReply::Update {seat,update}).await?;
                                }
                            }
                            Err(error)=> {
                                stale.push(seat);
                                send(&mut write,&GuiReply::Error {op:Ulid::nil(),reason:error.to_string()}).await?;
                            }
                        }
                    }
                    for seat in stale {views.remove(&seat);}
                }
            }
        }
        Ok(())
    }.await;
    if let Some(consumer) = consumer {
        let _ = client.bus.unsubscribe(&consumer).await;
    }
    result
}

pub async fn read_line(reader: &mut (impl AsyncBufRead + Unpin)) -> Result<Option<Vec<u8>>> {
    let mut line = Vec::new();
    loop {
        let part = reader.fill_buf().await?;
        if part.is_empty() {
            return if line.is_empty() {
                Ok(None)
            } else {
                Err(invalid("truncated NDJSON frame"))
            };
        }
        let count = part
            .iter()
            .position(|b| *b == b'\n')
            .map_or(part.len(), |n| n + 1);
        if line.len() + count > MAX_LINE {
            return Err(invalid("NDJSON frame exceeds 1 MiB"));
        }
        let done = part[count - 1] == b'\n';
        line.extend_from_slice(&part[..count]);
        reader.consume(count);
        if done {
            line.pop();
            return Ok(Some(line));
        }
    }
}

async fn send(writer: &mut tokio::net::unix::OwnedWriteHalf, value: &impl Serialize) -> Result<()> {
    let mut bytes = serde_json::to_vec(value)?;
    if bytes.len() >= MAX_LINE {
        return Err(invalid("response exceeds negotiated frame bound"));
    }
    bytes.push(b'\n');
    tokio::time::timeout(Duration::from_secs(2), writer.write_all(&bytes)).await??;
    Ok(())
}

async fn session_inner(
    stream: tokio::net::UnixStream,
    client: LogicClient,
    seat: Ulid,
    generation: u64,
    capability: [u8; 32],
    consumers: &mut BTreeSet<String>,
) -> Result<()> {
    let (read, mut write) = stream.into_split();
    let (lines, mut incoming) = tokio::sync::mpsc::channel(2);
    let reader_task = tokio::spawn(async move {
        let mut reader = BufReader::new(read);
        loop {
            let line = read_line(&mut reader).await;
            let done = !matches!(line, Ok(Some(_)));
            if lines.send(line).await.is_err() || done {
                break;
            }
        }
    });
    struct ReaderTask(tokio::task::JoinHandle<()>);
    impl Drop for ReaderTask {
        fn drop(&mut self) {
            self.0.abort();
        }
    }
    let _reader_task = ReaderTask(reader_task);
    let mut view = ViewStream::default();
    let mut subscribed = false;
    let mut last = None;
    let mut tick = tokio::time::interval(Duration::from_millis(20));
    loop {
        tokio::select! {
            line=incoming.recv()=> {
                let Some(Ok(Some(line)))=line else {break;};
                let request:Request=serde_json::from_slice(&line)?;
                if request.version!=1||request.capability!=capability||request.seat!=seat||request.generation!=generation {return Err(invalid("seat socket admission/generation rejected"));}
                // Revalidate against the logic's supervisor-confirmed generation
                // on every request, not merely against this listener's pathname.
                let snapshot=client.snapshot(seat,generation).await?;
                let result:Result<serde_json::Value>=match request.action {
                    Action::Catalogue=>client.control_catalogue(seat,generation).await,
                    Action::Call{call}=>client.control_call(seat,generation,call).await,
                    Action::Messaging{request}=>client.messaging(seat,generation,request).await.and_then(|v|Ok(serde_json::to_value(v)?)),
                    Action::Hook{event}=>Ok(serde_json::json!({"kind":"HookObserved","status":client.hook(seat,generation,event).await?})),
                    Action::Attach|Action::Snapshot=>Ok(serde_json::json!({"kind":"Snapshot","snapshot":snapshot})),
                    Action::Subscribe=>{subscribed=true;view.reset();view.offer(snapshot)?;Ok(serde_json::json!({"kind":"Subscribed"}))}
                    Action::Applied{view:identity,sequence}=>view.applied(identity,sequence).map(|()|serde_json::json!({"kind":"Applied"})),
                    Action::GetHistory{from,to}=>Ok(serde_json::json!({"kind":"History","history":client.history(seat,generation,from,to).await?})),
                    Action::BusSubscribe{consumer}=> {
                        if consumers.len()>=8||consumer.name.is_some()||consumer.filter.iter().any(|p|!(p.starts_with(&format!("seat/{seat}/"))||p.starts_with(&format!("inbox/{seat}/")))) {Err(invalid("seat socket consumer is limited to its own ephemeral topics"))}else {let id=client.bus.subscribe(consumer).await?;consumers.insert(id.clone());Ok(serde_json::json!({"kind":"BusSubscribed","consumer":id}))}
                    }
                    Action::PollEvents{consumer,limit}=> {if !consumers.contains(&consumer){Err(invalid("consumer belongs to another connection"))}else{Ok(serde_json::json!({"kind":"Events","events":client.bus.poll(&consumer,limit.min(128)).await?}))}}
                    Action::Ack{consumer,stream,sequence}=>{if !consumers.contains(&consumer){Err(invalid("consumer belongs to another connection"))}else{client.bus.ack(&consumer,&stream,sequence).await.map(|()|serde_json::json!({"kind":"Acked"}))}}
                };
                match result {Ok(value)=>send(&mut write,&value).await?,Err(error)=>send(&mut write,&serde_json::json!({"kind":"Rejected","reason":error.to_string()})).await?,}
            }
            _=tick.tick(),if subscribed=> {
                let snapshot=client.snapshot(seat,generation).await?;
                let identity=(snapshot.view,snapshot.sequence);
                if last!=Some(identity){view.offer(snapshot)?;last=Some(identity);}
                if let Some(update)=view.next_update(){send(&mut write,&update).await?;}
            }
        }
    }
    Ok(())
}

async fn session(
    stream: tokio::net::UnixStream,
    client: LogicClient,
    seat: Ulid,
    generation: u64,
    capability: [u8; 32],
) -> Result<()> {
    let mut consumers = BTreeSet::new();
    let result = session_inner(
        stream,
        client.clone(),
        seat,
        generation,
        capability,
        &mut consumers,
    )
    .await;
    for consumer in consumers {
        let _ = client.bus.unsubscribe(&consumer).await;
    }
    result
}

#[cfg(test)]
mod gui_tests {
    use super::*;
    use crate::{
        LaunchSpec, Supervisor,
        logic::{Logic, LogicConfig},
        registry::SeatRecord,
    };

    async fn update(rx: &mut tokio::sync::mpsc::Receiver<GuiReply>) -> (View, u64, bool) {
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                match rx.recv().await.expect("GUI reader closed") {
                    GuiReply::Update { update, .. } => {
                        return match update {
                            bbt_proto::Update::Snapshot { snapshot } => {
                                (snapshot.view, snapshot.sequence, true)
                            }
                            bbt_proto::Update::Delta { delta } => {
                                (delta.view, delta.sequence, false)
                            }
                        };
                    }
                    GuiReply::Error { reason, .. } => panic!("GUI rejected operation: {reason}"),
                    _ => {}
                }
            }
        })
        .await
        .expect("no view update")
    }
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn gui_view_waits_for_applied_and_resize_clamps_without_reading_cells() {
        let temp = tempfile::tempdir().unwrap();
        let repository = temp.path().join("repository");
        std::fs::create_dir(&repository).unwrap();
        let supervisor = Supervisor::start(Default::default()).unwrap();
        let logic = Logic::start(
            supervisor.connect().unwrap(),
            LogicConfig {
                run_dir: temp.path().join("run"),
                repository,
                ring: Default::default(),
            },
            ed25519_dalek::SigningKey::from_bytes(&[91; 32]),
            &tokio::runtime::Handle::current(),
        )
        .unwrap();
        let spec = LaunchSpec::new("/bin/cat", temp.path());
        let id = spec.seat;
        logic
            .client
            .spawn(SeatRecord::new(id, Ulid::new(), "cat", "cat"), spec)
            .await
            .unwrap();
        let endpoint = logic.client.gui_endpoint().await.unwrap();
        let stream = tokio::net::UnixStream::connect(&endpoint.path)
            .await
            .unwrap();
        let (read, mut write) = stream.into_split();
        let (tx, mut rx) = tokio::sync::mpsc::channel(8);
        let reader = tokio::spawn(async move {
            let mut read = BufReader::new(read);
            while let Ok(Some(line)) = read_line(&mut read).await {
                if tx
                    .send(serde_json::from_slice(&line).unwrap())
                    .await
                    .is_err()
                {
                    break;
                }
            }
        });
        let seat = bbt_proto::SeatId::from_u128(id.0);
        send(
            &mut write,
            &GuiRequest {
                version: 1,
                capability: endpoint.capability,
                action: GuiAction::Watch {
                    seat,
                    view: View {
                        seat: id,
                        generation: 1,
                        view_generation: Ulid::nil(),
                    },
                },
            },
        )
        .await
        .unwrap();
        let (view, first, snapshot) = update(&mut rx).await;
        assert!(snapshot);
        send(
            &mut write,
            &GuiRequest {
                version: 1,
                capability: endpoint.capability,
                action: GuiAction::Input {
                    seat,
                    id: Ulid::new(),
                    bytes: b"probe\n".to_vec(),
                },
            },
        )
        .await
        .unwrap();
        let _ = tokio::time::timeout(Duration::from_millis(200), async {
            while let Some(reply) = rx.recv().await {
                assert!(
                    !matches!(reply, GuiReply::Update { .. }),
                    "sent a second view before Applied"
                );
                if let GuiReply::Error { reason, .. } = reply {
                    panic!("input rejected: {reason}");
                }
            }
        })
        .await;
        send(
            &mut write,
            &GuiRequest {
                version: 1,
                capability: endpoint.capability,
                action: GuiAction::Applied {
                    seat,
                    view,
                    sequence: first,
                },
            },
        )
        .await
        .unwrap();
        let (next, sequence, _) = update(&mut rx).await;
        assert!(sequence > first);
        send(
            &mut write,
            &GuiRequest {
                version: 1,
                capability: endpoint.capability,
                action: GuiAction::Resize {
                    seat,
                    cols: 0,
                    rows: 1024,
                },
            },
        )
        .await
        .unwrap();
        send(
            &mut write,
            &GuiRequest {
                version: 1,
                capability: endpoint.capability,
                action: GuiAction::Applied {
                    seat,
                    view: next,
                    sequence,
                },
            },
        )
        .await
        .unwrap();
        let (resized, _, snapshot) = update(&mut rx).await;
        assert!(snapshot);
        assert_ne!(resized.view_generation, next.view_generation);
        drop(write);
        reader.abort();
        logic.shutdown().unwrap();
        supervisor.shutdown().unwrap();
    }
}
