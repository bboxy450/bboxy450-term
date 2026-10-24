//! Owner-only, capability-bound seat observation and semantic-control socket.
//! Raw input is unavailable here; messaging uses the common adapter/guard broker.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

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
                        // With ctl closed the window is the only client; with it open the
                        // door is chosen by the peer's process ancestry (decision #260).
                        let door=match client.gui_peers() {
                            None=>crate::mcp::Door::Gui,
                            Some(policy)=>match crate::ctl_door::classify(stream.peer_cred().ok().and_then(|c|c.pid()),policy) {
                                crate::ctl_door::Admit::Door(door)=>door,
                                crate::ctl_door::Admit::Refused(reason)=> {
                                    peers.spawn(async move {
                                        let _permit=permit;
                                        let (_read,mut write)=stream.into_split();
                                        let _=send(&mut write,&GuiReply::Error {op:Ulid::nil(),reason:reason.into()}).await;
                                    });
                                    continue;
                                }
                            },
                        };
                        let client=client.clone();
                        peers.spawn(async move {
                            let _permit=permit;
                            if let Err(error)=gui_session(stream,client,capability,door).await {
                                eprintln!("GUI session ended: door={door:?}, reason={}",gui_session_error_reason(&error));
                            }
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

fn gui_session_error_reason(error: &crate::store::Error) -> String {
    // Error Display implementations can include rejected input values. Log
    // only fixed categories and numeric metadata, never frame payload bytes.
    if error.is::<tokio::time::error::Elapsed>() {
        "deadline has elapsed".into()
    } else if let Some(error) = error.downcast_ref::<std::io::Error>() {
        format!(
            "I/O error ({:?}, os={:?})",
            error.kind(),
            error.raw_os_error()
        )
    } else if let Some(error) = error.downcast_ref::<serde_json::Error>() {
        format!(
            "invalid GUI frame ({:?}, line={}, column={})",
            error.classify(),
            error.line(),
            error.column(),
        )
    } else if let Some(error) = error.downcast_ref::<rusqlite::Error>() {
        format!("GUI store error ({:?})", error.sqlite_error_code())
    } else {
        "GUI session failed".into()
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
    in_flight: Option<(View, u64)>,
}
impl GuiView {
    fn next_update(&mut self) -> Option<crate::streams::Update> {
        let update = self.stream.next_update()?;
        self.in_flight = Some(match &update {
            crate::streams::Update::Snapshot { snapshot } => (snapshot.view, snapshot.sequence),
            crate::streams::Update::Delta { delta } => (delta.view, delta.sequence),
        });
        Some(update)
    }
    fn applied(&mut self, view: View, sequence: u64) -> Result<()> {
        self.stream.applied(view, sequence)?;
        // A duplicate baseline ACK is valid, but must not clear a newer send.
        if self.in_flight == Some((view, sequence)) {
            self.in_flight = None;
        }
        Ok(())
    }
}
#[derive(Default)]
struct GuiChecks {
    queued: BTreeSet<bbt_proto::SeatId>,
    order: VecDeque<bbt_proto::SeatId>,
}
impl GuiChecks {
    fn schedule(&mut self, seat: bbt_proto::SeatId) {
        if self.queued.insert(seat) {
            self.order.push_back(seat);
        }
    }
    fn remove(&mut self, seat: bbt_proto::SeatId) {
        if self.queued.remove(&seat) {
            self.order.retain(|id| *id != seat);
        }
    }
    fn next(&mut self) -> Option<bbt_proto::SeatId> {
        let seat = self.order.pop_front()?;
        self.queued.remove(&seat);
        Some(seat)
    }
    fn is_empty(&self) -> bool {
        self.order.is_empty()
    }
}
async fn gui_check_view(
    client: &LogicClient,
    write: &mut tokio::net::unix::OwnedWriteHalf,
    seat: bbt_proto::SeatId,
    watched: &mut GuiView,
) -> Result<()> {
    let snapshot = client
        .snapshot(Ulid::from(seat.0.0), watched.generation)
        .await?;
    let identity = Some((snapshot.view, snapshot.sequence));
    if identity != watched.last {
        watched.stream.offer(snapshot)?;
        watched.last = identity;
    }
    if let Some(update) = watched.next_update() {
        send(write, &GuiReply::Update { seat, update }).await?;
    }
    Ok(())
}
async fn gui_fallback_check(
    client: &LogicClient,
    write: &mut tokio::net::unix::OwnedWriteHalf,
    seat: bbt_proto::SeatId,
    watched: &mut GuiView,
) -> Result<()> {
    // Applied immediately checks for a fresh snapshot. Until then, another
    // Snapshot RPC cannot produce an Update for this view.
    if watched.in_flight.is_some() {
        return Ok(());
    }
    gui_check_view(client, write, seat, watched).await
}
struct GuiLane {
    bus: crate::bus::BusHandle,
    lease: crate::bus::GuiLease,
    detached: bool,
}
impl Drop for GuiLane {
    fn drop(&mut self) {
        if self.detached {
            return;
        }
        let bus = self.bus.clone();
        let lease = self.lease.clone();
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(async move {
                let _ = bus.gui_detach(&lease).await;
            });
        }
    }
}
impl GuiLane {
    async fn close(mut self) {
        if self.bus.gui_detach(&self.lease).await.is_ok() {
            self.detached = true;
        }
    }
}
async fn gui_attach_lane(client: &LogicClient) -> Result<(GuiLane, Option<crate::bus::Delivery>)> {
    let device = client.gui_device().await?;
    let filter = crate::gui_events::filters(client.gui_messaging_subjects().await?);
    let lease = client.bus.gui_attach(device, filter).await?;
    let lane = GuiLane {
        bus: client.bus.clone(),
        lease,
        detached: false,
    };
    let latest = client
        .bus
        .gui_latest(&lane.lease, "account", "account/identity/state")
        .await?
        .map(|envelope| crate::bus::Delivery::Event {
            stream: "account".into(),
            envelope: Box::new(envelope),
            deliveries: 0,
        });
    Ok((lane, latest))
}
fn gui_gap(stream: String, first: u64, last: u64, reason: String) -> GuiReply {
    GuiReply::Gap {
        stream,
        first,
        last,
        reason,
    }
}
async fn gui_event_reply(
    delivery: crate::bus::Delivery,
    client: &LogicClient,
    cursor: Option<bbt_proto::events::GuiEventCursor>,
) -> Result<GuiReply> {
    match delivery {
        crate::bus::Delivery::Event {
            stream, envelope, ..
        } => {
            let seq = envelope.seq;
            let subject = envelope.event.subject.clone();
            let projected = crate::gui_events::project(client, &stream, *envelope, cursor).await?;
            let reply = GuiReply::Event {
                stream: stream.clone(),
                seq,
                subject,
                envelope: Box::new(projected),
            };
            if serde_json::to_vec(&reply)?.len() > crate::gui_events::LIVE_BYTES {
                return Ok(gui_gap(
                    stream,
                    seq,
                    seq,
                    "GUI projection exceeds live byte bound; original retained".into(),
                ));
            }
            Ok(reply)
        }
        crate::bus::Delivery::Gap {
            stream,
            first,
            last,
            reason,
        } => Ok(gui_gap(stream, first, last, reason)),
    }
}
fn gui_events_page(page: crate::bus::GuiPage) -> Result<GuiReply> {
    use bbt_proto::events::{
        EventGap, GuiEventEnvelope, GuiEventPage, GuiEventPayload, GuiEventRecord,
    };
    let mut records = Vec::new();
    let mut used = 0;
    let mut next = page.next;
    for delivery in page.records {
        let (record, before) = match delivery {
            crate::bus::Delivery::Event {
                stream, envelope, ..
            } => {
                let sequence = envelope.seq;
                // A retained history page is activity data, never a request to
                // replace a live pane or historical identity with old state.
                let record = GuiEventRecord::Event(Box::new(GuiEventEnvelope {
                    metadata: crate::gui_events::metadata(&stream, &envelope)?,
                    projected_at: crate::store::now_ms(),
                    cursor: None,
                    payload: GuiEventPayload::Activity {
                        summary: crate::gui_events::summary(&envelope)?,
                    },
                }));
                (record, sequence.saturating_sub(1))
            }
            crate::bus::Delivery::Gap {
                stream,
                first,
                last,
                reason,
            } => (
                GuiEventRecord::Gap(EventGap {
                    stream,
                    first,
                    last,
                    reason,
                }),
                first.saturating_sub(1),
            ),
        };
        let bytes = serde_json::to_vec(&record)?.len();
        if used + bytes > crate::gui_events::PAGE_BYTES - 2048 {
            next = Some(before);
            break;
        }
        used += bytes;
        records.push(record);
    }
    Ok(GuiReply::Events {
        page: GuiEventPage {
            records,
            next,
            cursor: crate::gui_events::cursor(page.cursor),
        },
    })
}
async fn gui_session(
    stream: tokio::net::UnixStream,
    client: LogicClient,
    capability: [u8; 32],
    door: crate::mcp::Door,
) -> Result<()> {
    let (read, mut write) = stream.into_split();
    // Absorb simultaneous ACKs, but a waiting pane gets a work turn after
    // one action. Otherwise one fast ACK loop can delay the other panes.
    const ACTION_BURST: usize = 4;
    let (lines, mut incoming) = tokio::sync::mpsc::channel(16);
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
    let mut admitted = false;
    let mut lane: Option<GuiLane> = None;
    let mut want_events = None;
    let mut lane_at: Option<Instant> = None;
    let mut events_at: Option<Instant> = None;
    let mut lane_error: Option<String> = None;
    let mut filters_at = Instant::now();
    let mut event_cursors = BTreeMap::new();
    let mut views: BTreeMap<bbt_proto::SeatId, GuiView> = BTreeMap::new();
    let mut checks = GuiChecks::default();
    let mut state_events = VecDeque::new();
    let mut actions = 0;
    let mut prefer_view = true;
    let mut tick = tokio::time::interval(Duration::from_millis(20));
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    // Only the window captures: its session relays the operator's asks.
    let mut capture_asks = (door == crate::mcp::Door::Gui).then(|| client.captures.subscribe());
    // Who issues this connection's operator requests, once it says (decision #269).
    let mut issued_by: Option<String> = None;
    let result:Result<()>=async {
        loop {
            tokio::select! {
                biased;
                line=incoming.recv(),if actions<ACTION_BURST&&(actions==0||checks.is_empty())=> {
                    actions+=1;
                    let Some(Ok(Some(line)))=line else {break;};
                    #[derive(Deserialize)]
                    struct GuiAdmission { version:u16, capability:[u8;32] }
                    // Inspect admission before the action schema, so an old
                    // Spawn without an id gets the named version refusal.
                    let admission:GuiAdmission=serde_json::from_slice(&line)?;
                    if admission.capability!=capability {
                        // Say why before closing, as the version check does: a caller reads
                        // the reason, not a closed socket.
                        send(&mut write,&GuiReply::Error {op:Ulid::nil(),reason:"GUI socket admission rejected".into()}).await?;
                        return Err(invalid("GUI socket admission rejected"));
                    }
                    if admission.version!=bbt_proto::gui::VERSION {
                        send(&mut write,&GuiReply::Error {op:Ulid::nil(),reason:"GUI socket version mismatch".into()}).await?;
                        return Err(invalid("GUI socket version mismatch"));
                    }
                    let request:GuiRequest=serde_json::from_slice(&line)?;
                    if !admitted {
                        admitted=true;
                        send(&mut write,&GuiReply::Identity(client.gui_identity().await?)).await?;
                    }
                    let op=match &request.action {
                        GuiAction::AttachEvents {id}|GuiAction::Spawn {id,..}|GuiAction::Messaging {id,..}|GuiAction::Task {id,..}|GuiAction::Input {id,..}|GuiAction::ScheduleClose {id,..}|GuiAction::Capture {id,..}|GuiAction::Captured {id,..}|GuiAction::Knowledge {id,..}=>*id,
                        GuiAction::UndoClose {close_id,..}=>*close_id,
                        _=>Ulid::new(),
                    };
                    let reply:Result<GuiReply>=async {
                        match request.action {
                            GuiAction::ListSeats=>Ok(GuiReply::Seats(client.gui_seats().await?)),
                            GuiAction::ListHosts=>Ok(GuiReply::Hosts(client.gui_hosts().await?)),
                            GuiAction::AttachEvents {id}=> {
                                want_events=Some(id);
                                if lane.is_none() {
                                    lane_at=Some(Instant::now());
                                    match gui_attach_lane(&client).await {
                                        Ok((attached,latest))=> {
                                            lane=Some(attached); lane_error=None;
                                            state_events.extend(latest);
                                        }
                                        Err(error)=> {
                                            let reason=error.to_string();
                                            if !reason.starts_with("GuiEventsHeld:") {want_events=None;}
                                            lane_error=Some(reason);
                                            return Err(error);
                                        }
                                    }
                                }
                                Ok(GuiReply::Ok {op:id})
                            }
                            GuiAction::EventAck {stream,seq}=> {
                                let active=lane.as_ref().ok_or_else(||invalid("GuiEventsHeld: events lane is not attached"))?;
                                client.bus.gui_ack(&active.lease,&stream,seq).await?;
                                Ok(GuiReply::Ok {op})
                            }
                            GuiAction::GetEventCursor {stream}=> {
                                let active=lane.as_ref().ok_or_else(||invalid("GuiEventsHeld: events lane is not attached"))?;
                                Ok(GuiReply::EventCursor {cursor:crate::gui_events::cursor(client.bus.gui_cursor(&active.lease,&stream).await?)})
                            }
                            GuiAction::MarkEventsRead {stream,seq}=> {
                                let active=lane.as_ref().ok_or_else(||invalid("GuiEventsHeld: events lane is not attached"))?;
                                let cursor=crate::gui_events::cursor(client.bus.gui_read(&active.lease,&stream,seq).await?);
                                event_cursors.remove(&stream);
                                Ok(GuiReply::EventCursor {cursor})
                            }
                            GuiAction::GetEvents {stream,after,limit}=> {
                                let active=lane.as_ref().ok_or_else(||invalid("GuiEventsHeld: events lane is not attached"))?;
                                gui_events_page(client.bus.gui_page(&active.lease,&stream,after,limit,crate::gui_events::PAGE_BYTES).await?)
                            }
                            GuiAction::Spawn {id,spec}=> {
                                let seat=client.gui_spawn(id,spec.clone(),door).await?;
                                // The window places a seat another door spawned, as its own pick.
                                if door==crate::mcp::Door::Ctl {client.captures.spawned(id,seat,spec);}
                                Ok(GuiReply::Spawned {op:id,seat})
                            }
                            GuiAction::Messaging {id,swarm_id,request}=>Ok(GuiReply::Messaging {op:id,response:client.gui_messaging(id,Ulid::from(swarm_id.0.0),request,crate::mcp::Origin {door,issued_by:issued_by.clone()}).await?}),
                            GuiAction::Task {id,request}=>Ok(GuiReply::Task {op:id,response:client.gui_task(id,request,crate::mcp::Origin {door,issued_by:issued_by.clone()}).await?}),
                            GuiAction::Knowledge {id,method,args}=>Ok(GuiReply::Knowledge {op:id,response:serde_json::to_value(client.gui_knowledge(method,args).await?)?}),
                            GuiAction::Watch {seat,view}=> {
                                if view.seat!=Ulid::from(seat.0.0) {return Err(invalid("watch seat and view disagree"));}
                                if views.len()>=1024&&!views.contains_key(&seat) {return Err(invalid("watch limit reached"));}
                                let snapshot=client.snapshot(view.seat,view.generation).await?;
                                let last=Some((snapshot.view,snapshot.sequence));
                                let mut stream=ViewStream::default();stream.offer(snapshot)?;
                                views.insert(seat,GuiView {stream,generation:view.generation,last,in_flight:None});
                                checks.schedule(seat);
                                Ok(GuiReply::Ok {op})
                            }
                            GuiAction::Unwatch {seat}=>{views.remove(&seat);checks.remove(seat);Ok(GuiReply::Ok {op})}
                            GuiAction::Applied {seat,view,sequence}=> {
                                if view.seat!=Ulid::from(seat.0.0) {return Err(invalid("ack seat and view disagree"));}
                                let watched=views.get_mut(&seat).ok_or_else(||invalid("seat is not watched"))?;
                                watched.applied(view,sequence)?;
                                checks.remove(seat);
                                let result=gui_check_view(&client,&mut write,seat,watched).await;
                                if result.is_err() {views.remove(&seat);}
                                result?;
                                Ok(GuiReply::Ok {op})
                            }
                            GuiAction::Input {seat,generation,id,bytes}=> {
                                let receipt=client.gui_input(Ulid::from(seat.0.0),generation,id,bytes).await?;
                                if receipt.state==crate::InputState::PartialUnknown {return Err(invalid("input result is partial/unknown; keep the original id"));}
                                Ok(GuiReply::Ok {op:id})
                            }
                            GuiAction::Resize {seat,generation,cols,rows}=> {
                                client.gui_resize(Ulid::from(seat.0.0),generation,cols,rows).await?;
                                Ok(GuiReply::Ok {op})
                            }
                            GuiAction::ScheduleClose {seat,id,position}=> {
                                let seat=Ulid::from(seat.0.0);
                                let record=client.record(seat).await?;
                                client.close(seat,record.generation,id,position,60_000).await?;
                                Ok(GuiReply::Seat(client.gui_seat(seat).await?))
                            }
                            GuiAction::UndoClose {seat,close_id,layout_revision}=> {
                                let seat=Ulid::from(seat.0.0);
                                let record=client.record(seat).await?;
                                let close=record.close.as_ref().filter(|close|close.operation==close_id)
                                    .ok_or_else(||invalid("stale close operation"))?;
                                client.undo(seat,record.generation,close.revision,layout_revision).await?;
                                Ok(GuiReply::Seat(client.gui_seat(seat).await?))
                            }
                            GuiAction::Capture {id,path,seat}=> {
                                let frame=client.captures.ask(crate::gui_capture::Ask {id,path:path.clone(),seat}).await.map_err(invalid)?;
                                Ok(GuiReply::Captured {op:id,path,frame})
                            }
                            GuiAction::Issuer {label}=> {
                                if label.trim().is_empty()||label.len()>bbt_proto::messaging::MAX_ISSUER||label.chars().any(char::is_control) {
                                    return Err(invalid(format!("issuer label must be 1-{} printable characters",bbt_proto::messaging::MAX_ISSUER)));
                                }
                                issued_by=Some(label);
                                Ok(GuiReply::Ok {op})
                            }
                            GuiAction::Captured {id,error,frame}=> {
                                if capture_asks.is_none() {return Err(invalid("only the window answers a capture"));}
                                client.captures.answer(id,error.map_or(Ok(frame),Err));
                                Ok(GuiReply::Ok {op:id})
                            }
                            GuiAction::GetHistory {seat,from,to}=> {
                                let id=Ulid::from(seat.0.0);
                                let record=client.record(id).await?;
                                let history=client.history(id,record.generation,from,to).await?;
                                gui_history_reply(seat,record.generation,from,to,history)
                            }
                        }
                    }.await;
                    send(&mut write,&reply.unwrap_or_else(|e|GuiReply::Error {op,reason:e.to_string()})).await?;
                }
                ask=async {capture_asks.as_mut().unwrap().recv().await},if admitted&&capture_asks.is_some()=> {
                    match ask {
                        Ok(notice)=>send(&mut write,&notice.reply()).await?,
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(_))=>{}
                        Err(tokio::sync::broadcast::error::RecvError::Closed)=>capture_asks=None,
                    }
                }
                _=tick.tick(),if admitted=> {
                    if filters_at.elapsed()>=Duration::from_secs(1) && let Some(active)=&mut lane {
                        filters_at=Instant::now();
                        let filters=async {
                            if client.gui_device().await?!=active.lease.device {
                                return Err(invalid("GuiDeviceChanged: reattaching current custody device"));
                            }
                            client.gui_messaging_subjects().await
                        }.await;
                        match filters {
                            Ok(filters)=> {
                                let next=client.bus.gui_refresh(&active.lease,crate::gui_events::filters(filters)).await?;
                                if next!=active.lease {
                                    active.lease=next;state_events.clear();event_cursors.clear();events_at=None;
                                }
                            }
                            Err(error)=> {
                                let reason=format!("GuiEventsHeld: {error}");
                                if let Some(active)=lane.take() {active.close().await;}
                                state_events.clear();event_cursors.clear();
                                lane_error=Some(reason.clone());lane_at=Some(Instant::now());
                                send(&mut write,&GuiReply::EventStatus {attached:false,reason:Some(reason)}).await?;
                            }
                        }
                    }
                    if want_events.is_some() && lane.is_none() && lane_at.is_none_or(|at|at.elapsed()>=Duration::from_secs(1)) {
                        lane_at=Some(Instant::now());
                        match gui_attach_lane(&client).await {
                            Ok((attached,latest))=> {
                                lane=Some(attached);lane_error=None;state_events.extend(latest);
                                send(&mut write,&GuiReply::EventStatus {attached:true,reason:None}).await?;
                            }
                            Err(error)=> {
                                let reason=error.to_string();
                                if lane_error.as_deref()!=Some(&reason) {
                                    send(&mut write,&GuiReply::EventStatus {attached:false,reason:Some(reason.clone())}).await?;
                                    lane_error=Some(reason.clone());
                                }
                                if !reason.starts_with("GuiEventsHeld:") {want_events=None;}
                            }
                        }
                    }
                    if state_events.is_empty() && events_at.is_none_or(|at|at.elapsed()>=Duration::from_millis(100)) && let Some(active)=&lane {
                        state_events.extend(client.bus.gui_poll(&active.lease,crate::gui_events::PAGE_RECORDS,crate::gui_events::PAGE_BYTES).await?);
                        event_cursors.clear();events_at=Some(Instant::now());
                    }
                    for &seat in views.keys() {checks.schedule(seat);}
                }
                _=async {},if !checks.is_empty()||!state_events.is_empty()=> {
                    // One awaited unit, then incoming actions are polled again.
                    // Alternate state and pane work so neither can monopolize.
                    if !checks.is_empty()&&(prefer_view||state_events.is_empty()) {
                        let seat=checks.next().unwrap();
                        if let Some(watched)=views.get_mut(&seat)
                            && let Err(error)=gui_fallback_check(&client,&mut write,seat,watched).await {
                            views.remove(&seat);
                            send(&mut write,&GuiReply::Error {op:Ulid::nil(),reason:error.to_string()}).await?;
                        }
                        prefer_view=false;
                    } else if let Some(event)=state_events.pop_front() {
                        let cursor=if let crate::bus::Delivery::Event {stream,envelope,..}=&event {
                            if envelope.event.subject.ends_with("/message")||envelope.event.subject.ends_with("/receipt") {
                                if !event_cursors.contains_key(stream) && let Some(active)=&lane {
                                    event_cursors.insert(stream.clone(),crate::gui_events::cursor(client.bus.gui_cursor(&active.lease,stream).await?));
                                }
                                event_cursors.get(stream).cloned()
                            } else {None}
                        } else {None};
                        match gui_event_reply(event,&client,cursor).await {
                            Ok(reply)=>send(&mut write,&reply).await?,
                            Err(error)=>send(&mut write,&GuiReply::Error {op:Ulid::nil(),reason:error.to_string()}).await?,
                        }
                        prefer_view=true;
                    }
                    actions=0;
                }
                _=async {},if actions>=ACTION_BURST=> {
                    actions=0;
                    tokio::task::yield_now().await;
                }
            }
        }
        Ok(())
    }.await;
    if let Some(active) = lane.take() {
        active.close().await;
    }
    result
}

fn gui_history_reply(
    seat: bbt_proto::SeatId,
    generation: u64,
    from: u64,
    to: u64,
    history: crate::ring::History,
) -> Result<GuiReply> {
    use bbt_proto::gui::HistoryRecord;
    if history.status.generation != generation {
        return Err(invalid("history belongs to another seat or generation"));
    }
    let mut records = Vec::new();
    let gap = |from, to, source: &crate::ring::HistoryGap| HistoryRecord::Gap {
        from,
        to,
        reason: source.reason.clone(),
    };
    if let Some(missing) = &history.gap {
        match missing.retained_from {
            Some(first) if from < first => records.push(gap(from, to.min(first - 1), missing)),
            None => records.push(gap(from, to, missing)),
            _ => {}
        }
    }
    for record in history.records {
        let record_generation = match &record.event {
            crate::Event::Output { generation, .. }
            | crate::Event::Resized { generation, .. }
            | crate::Event::Exited { generation, .. } => *generation,
        };
        if record.event.seat() != Ulid::from(seat.0.0) || record_generation != generation {
            return Err(invalid(
                "history record belongs to another seat or generation",
            ));
        }
        records.push(match record.event {
            crate::Event::Output { seq, bytes, .. } => HistoryRecord::Bytes {
                sequence: seq,
                bytes,
            },
            crate::Event::Resized { seq, .. } | crate::Event::Exited { seq, .. } => {
                HistoryRecord::Bytes {
                    sequence: seq,
                    bytes: Vec::new(),
                }
            }
        });
    }
    // A suffix gap follows all available records, including later pages. Do
    // not put it before data that the caller will fetch through next.
    if history.next.is_none()
        && let Some(missing) = &history.gap
        && let Some(last) = missing.retained_to
        && to > last
    {
        records.push(gap(from.max(last.saturating_add(1)), to, missing));
    }
    let reply = GuiReply::History {
        seat,
        records,
        next: history.next,
    };
    if serde_json::to_vec(&reply)?.len() >= MAX_LINE {
        return Err(invalid(
            "HistoryFrameTooLarge: capture record exceeds GUI framing",
        ));
    }
    Ok(reply)
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

    #[tokio::test]
    async fn session_error_reasons_name_deadlines_without_logging_payloads() {
        let elapsed = tokio::time::timeout(Duration::ZERO, std::future::pending::<()>())
            .await
            .unwrap_err();
        let elapsed: crate::store::Error = elapsed.into();
        assert_eq!(gui_session_error_reason(&elapsed), "deadline has elapsed");

        let io: crate::store::Error = std::io::Error::other("private-payload-value").into();
        let reason = gui_session_error_reason(&io);
        assert!(reason.starts_with("I/O error"));
        assert!(!reason.contains("private-payload-value"));

        let json: crate::store::Error =
            serde_json::from_str::<GuiReply>(r#"{"kind":"private-payload-value"}"#)
                .unwrap_err()
                .into();
        let reason = gui_session_error_reason(&json);
        assert!(reason.starts_with("invalid GUI frame"));
        assert!(!reason.contains("private-payload-value"));
    }

    #[test]
    fn repeated_ticks_keep_one_check_per_seat_and_hot_seats_stay_at_the_back() {
        let mut checks = GuiChecks::default();
        let seats: Vec<_> = (1..=1024).map(bbt_proto::SeatId::from_u128).collect();
        for _ in 0..10 {
            for &seat in &seats {
                checks.schedule(seat);
            }
        }
        assert_eq!(checks.order.len(), seats.len());
        assert_eq!(checks.queued.len(), seats.len());
        let first = checks.next().unwrap();
        checks.schedule(first);
        for &seat in &seats[1..] {
            assert_eq!(checks.next(), Some(seat));
        }
        assert_eq!(checks.next(), Some(first));
        assert!(checks.is_empty());
    }

    #[test]
    fn unwatch_removes_pending_work_and_rewatch_schedules_only_once() {
        let mut checks = GuiChecks::default();
        let a = bbt_proto::SeatId::from_u128(1);
        let b = bbt_proto::SeatId::from_u128(2);
        checks.schedule(a);
        checks.schedule(b);
        checks.remove(a);
        checks.schedule(a);
        checks.schedule(a);
        assert_eq!(checks.next(), Some(b));
        assert_eq!(checks.next(), Some(a));
        assert!(checks.is_empty());
        assert!(checks.queued.is_empty());
    }

    fn offered_view() -> (bbt_proto::SeatId, View, GuiView) {
        let id = Ulid::new();
        let view = View {
            seat: id,
            generation: 1,
            view_generation: Ulid::new(),
        };
        let snapshot = crate::streams::Snapshot {
            view,
            sequence: 1,
            screen: crate::streams::Screen::from_term(
                &bbt_core::Term::new(2, 2, 0),
                String::new(),
                None,
                None,
            ),
        };
        let mut stream = ViewStream::default();
        stream.offer(snapshot).unwrap();
        (
            bbt_proto::SeatId::from_u128(id.0),
            view,
            GuiView {
                stream,
                generation: 1,
                last: Some((view, 1)),
                in_flight: None,
            },
        )
    }

    #[test]
    fn duplicate_baseline_ack_preserves_the_newer_in_flight_update() {
        let (_, view, mut watched) = offered_view();
        let crate::streams::Update::Snapshot { snapshot } = watched.next_update().unwrap() else {
            panic!("first update must be a snapshot");
        };
        watched.applied(view, 1).unwrap();
        let mut newer = *snapshot;
        newer.sequence = 2;
        watched.stream.offer(newer).unwrap();
        assert!(watched.next_update().is_some());
        watched.applied(view, 1).unwrap();
        assert_eq!(watched.in_flight, Some((view, 2)));
        assert!(watched.next_update().is_none());
        assert!(watched.applied(view, 3).is_err());
        assert_eq!(watched.in_flight, Some((view, 2)));
        watched.applied(view, 2).unwrap();
        assert_eq!(watched.in_flight, None);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn fallback_never_requests_a_snapshot_while_an_update_is_in_flight() {
        let temp = tempfile::tempdir().unwrap();
        let repository = temp.path().join("repository");
        std::fs::create_dir(&repository).unwrap();
        let supervisor = Supervisor::start(crate::Config {
            journal_directory: Some(temp.path().join("supervisor-journal")),
            ..Default::default()
        })
        .unwrap();
        let logic = Logic::start(
            supervisor.connect().unwrap(),
            LogicConfig {
                run_dir: temp.path().join("run"),
                repository,
                ring: Default::default(),
            },
            ed25519_dalek::SigningKey::from_bytes(&[92; 32]),
            &tokio::runtime::Handle::current(),
        )
        .unwrap();
        let client = logic.client.clone();
        logic.shutdown().unwrap();
        let (writer, _reader) = tokio::net::UnixStream::pair().unwrap();
        let (_, mut write) = writer.into_split();
        let (seat, view, mut watched) = offered_view();
        // Positive control: an RPC against this stopped client always fails.
        let error = gui_fallback_check(&client, &mut write, seat, &mut watched)
            .await
            .unwrap_err();
        assert!(error.to_string().contains("logic stopped"), "{error}");
        assert!(watched.next_update().is_some());
        for _ in 0..100 {
            gui_fallback_check(&client, &mut write, seat, &mut watched)
                .await
                .unwrap();
        }
        assert_eq!(watched.in_flight, Some((view, 1)));
        watched.applied(view, 1).unwrap();
        assert!(
            gui_fallback_check(&client, &mut write, seat, &mut watched)
                .await
                .is_err()
        );
        supervisor.shutdown().unwrap();
    }

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
        let supervisor = Supervisor::start(crate::Config {
            journal_directory: Some(temp.path().join("supervisor-journal")),
            ..Default::default()
        })
        .unwrap();
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
                version: bbt_proto::gui::VERSION,
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
                version: bbt_proto::gui::VERSION,
                capability: endpoint.capability,
                action: GuiAction::Input {
                    seat,
                    generation: view.generation,
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
                version: bbt_proto::gui::VERSION,
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
                version: bbt_proto::gui::VERSION,
                capability: endpoint.capability,
                action: GuiAction::Resize {
                    seat,
                    generation: view.generation,
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
                version: bbt_proto::gui::VERSION,
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
