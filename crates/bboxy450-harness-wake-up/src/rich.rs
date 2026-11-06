// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Generation-owned, bounded Unix delivery. This transport has no PTY writer.
use crate::{Evidence, WakeCause, WakeOutcome};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    os::unix::fs::{FileTypeExt, MetadataExt},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use tokio::{
    io::{AsyncBufRead, AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::UnixStream,
    sync::{Notify, mpsc, oneshot, watch},
};
use ulid::Ulid;
const LIMIT: usize = 256 * 1024;
const ACK: Duration = Duration::from_secs(5);
const RECONNECT: Duration = Duration::from_secs(2);
/// Negotiated wire version. Major zero and one are readable.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Version {
    /// Wire compatibility major.
    pub major: u16,
    /// Additive wire revision.
    pub minor: u16,
}
/// A complete announcement from the current socket incarnation.
#[derive(Clone, Debug)]
pub struct Hello {
    /// Announced session incarnation, never a profile label.
    pub session: Ulid,
    /// Session restart epoch.
    pub epoch: u32,
    /// Highest durable sequence observed.
    pub head: u64,
    /// Negotiated envelope version.
    pub version: Version,
    /// Last validated transport frame received.
    pub last_frame: Instant,
}
/// Read-only latest transport state; no socket calls on the consumer thread.
#[derive(Clone, Debug)]
pub enum State {
    /// No announcement yet.
    Unbound,
    /// Current validated announcement and age.
    Connected(Hello),
    /// Connection ended or could not be established.
    Closed(String),
}
struct Request {
    id: Ulid,
    session: Ulid,
    op: Value,
    reply: oneshot::Sender<WakeOutcome>,
}
struct Stop {
    retired: AtomicBool,
    notify: Notify,
}
/// Cloneable submission port, always scoped to one process generation.
#[derive(Clone)]
pub struct Client {
    tx: mpsc::Sender<Request>,
    state: watch::Receiver<State>,
    stop: Arc<Stop>,
}
/// The process-generation owner; dropping it retires even retained client clones.
pub struct Connection {
    /// Submission and observation port.
    pub client: Client,
    task: tokio::task::JoinHandle<()>,
}
impl Drop for Connection {
    fn drop(&mut self) {
        self.client.stop.retired.store(true, Ordering::Release);
        self.client.stop.notify.notify_waiters();
        self.task.abort();
    }
}
impl Connection {
    /// Schedule the generation-owned IO worker.
    pub fn start(path: PathBuf, handle: &tokio::runtime::Handle) -> Self {
        Self::start_notifying(path, handle, Arc::new(|| {}))
    }
    /// Wake the logic owner on connection changes, independently of PTY output.
    pub fn start_notifying(
        path: PathBuf,
        handle: &tokio::runtime::Handle,
        wake: Arc<dyn Fn() + Send + Sync>,
    ) -> Self {
        Self::start_observing(path, handle, wake, None, None)
    }
    /// Optional tiny current-incarnation activity latch. This never establishes source closure.
    /// The callback must perform bounded atomics only: no IO, parsing, locks or waiting.
    pub fn start_observing(
        path: PathBuf,
        handle: &tokio::runtime::Handle,
        wake: Arc<dyn Fn() + Send + Sync>,
        expected_pid: Option<u32>,
        activity: Option<Arc<dyn Fn(Option<bool>) + Send + Sync>>,
    ) -> Self {
        let (tx, rx) = mpsc::channel(32);
        let (state, latest) = watch::channel(State::Unbound);
        let stop = Arc::new(Stop {
            retired: AtomicBool::new(false),
            notify: Notify::new(),
        });
        let client = Client {
            tx,
            state: latest,
            stop: stop.clone(),
        };
        let task = handle.spawn(run(path, rx, state, stop, wake, expected_pid, activity));
        Self { client, task }
    }
}
fn refused(cause: WakeCause, note: &str) -> WakeOutcome {
    WakeOutcome {
        evidence: Evidence::RecipientStored,
        native_id: None,
        note: note.into(),
        cause: Some(cause),
    }
}
/// Typed refusal without a write.
pub fn unavailable() -> WakeOutcome {
    refused(
        WakeCause::HarnessSocketUnavailable,
        "harness socket unavailable; no PTY fallback",
    )
}
impl Client {
    /// Latest immutable connection snapshot.
    pub fn state(&self) -> State {
        self.state.borrow().clone()
    }
    /// Recent announcement, absent after retirement or twenty seconds of silence.
    pub fn hello(&self) -> Option<Hello> {
        if self.stop.retired.load(Ordering::Acquire) {
            return None;
        }
        match self.state() {
            State::Connected(h) if h.last_frame.elapsed() < Duration::from_secs(20) => Some(h),
            _ => None,
        }
    }
    /// `session` is the message's retained original admission, never the new Hello on retry.
    pub async fn send(&self, id: Ulid, session: Ulid, op: Value) -> WakeOutcome {
        if self.hello().is_none() {
            return unavailable();
        }
        let (reply, response) = oneshot::channel();
        if self
            .tx
            .try_send(Request {
                id,
                session,
                op,
                reply,
            })
            .is_err()
        {
            return refused(WakeCause::HarnessBusy, "harness delivery lane full");
        }
        match tokio::time::timeout(ACK + Duration::from_millis(250), response).await {
            Ok(Ok(outcome)) => outcome,
            _ => WakeOutcome::unknown_cause(
                WakeCause::HarnessAckUnknown,
                "harness ack unknown; retain the same op_id",
            ),
        }
    }
}
/// Term chooses the path beside its supervisor control socket. Never truncate.
pub fn socket_path(control: &Path, seat: Ulid) -> crate::Result<PathBuf> {
    let parent = control
        .parent()
        .ok_or_else(|| crate::invalid("harness-socket-path-invalid"))?;
    let path = parent.join(format!("h-{seat}.sock"));
    if !path.is_absolute() || path.as_os_str().as_encoded_bytes().len() > 96 {
        return Err(crate::invalid(
            "harness-socket-path-too-long: maximum 96 bytes",
        ));
    }
    Ok(path)
}
/// Canonical nonzero harness session ULID.
pub fn valid_session(id: &str) -> bool {
    id.len() == 26
        && id
            .parse::<Ulid>()
            .is_ok_and(|u| u.to_string() == id && u != Ulid::nil())
}
// Adapted from bboxy450-harness src/gateway/unix.rs bounded NDJSON framing:
// https://github.com/bboxy450/bboxy450-harness (d5e7249). Changes: parse/retire
// payloads on a blocking worker and retain only delivery identity/reply fields.
// Policy: https://github.com/bboxy450/bboxy450-harness/blob/main/ATTRIBUTION.md
async fn frame(r: &mut (impl AsyncBufRead + Unpin)) -> std::io::Result<Option<Value>> {
    let mut b = Vec::new();
    loop {
        let buf = r.fill_buf().await?;
        if buf.is_empty() {
            return if b.is_empty() {
                Ok(None)
            } else {
                Err(std::io::Error::other("truncated frame"))
            };
        }
        let n = buf
            .iter()
            .position(|c| *c == b'\n')
            .map_or(buf.len(), |i| i + 1);
        if b.len() + n > LIMIT {
            return Err(std::io::Error::other("frame exceeds 256 KiB"));
        }
        b.extend_from_slice(&buf[..n]);
        r.consume(n);
        if b.ends_with(b"\n") {
            // Delivery needs only identity/liveness and correlated replies. Parse
            // and retire arbitrary IR payloads on the blocking pool, so a 256KiB
            // transcript frame cannot monopolize a shared IO worker.
            return tokio::task::spawn_blocking(move || -> std::io::Result<Option<Value>> {
                let value:Value=serde_json::from_slice(&b)?;
                let minimal=if value.get("meta").is_some() {
                    json!({"v":value["v"],"epoch":value["epoch"],"head":value["head"],"meta":{"session":value["meta"]["session"]}})
                } else if value.get("ack").is_some()||value.get("refused").is_some() {
                    json!({"op_id":value["op_id"],"ack":value.get("ack"),"refused":value.get("refused")})
                } else {
                    {
                        let turn=value["event"]["turn"].as_str().filter(|s|valid_session(s));
                        let fork=if value["event"]["meta"]["fork"].is_null(){Value::Null}else{json!(true)};
                        let source=if value["event"]["meta"]["source"].is_null(){Value::Null}else{json!(true)};
                        json!({"v":value["v"],"session":value["session"],"epoch":value["epoch"],"seq":value["seq"],"event":{"type":value["event"]["type"],"turn":turn,"meta":{"fork":fork,"source":source}}})
                    }
                };
                Ok(Some(minimal))
            }).await.map_err(std::io::Error::other)?;
        }
    }
}
async fn write(w: &mut (impl tokio::io::AsyncWrite + Unpin), v: &Value) -> std::io::Result<()> {
    let mut b = serde_json::to_vec(v)?;
    b.push(b'\n');
    if b.len() > LIMIT {
        return Err(std::io::Error::other("frame exceeds 256 KiB"));
    }
    w.write_all(&b).await
}
fn announcement(v: Value) -> std::io::Result<Hello> {
    let version: Version = serde_json::from_value(v["v"].clone())?;
    if version.major > 1 {
        return Err(std::io::Error::other(
            "harness-protocol-version-unsupported",
        ));
    }
    let session: Ulid = serde_json::from_value(v["meta"]["session"].clone())?;
    if !valid_session(&session.to_string()) {
        return Err(std::io::Error::other("invalid session"));
    }
    Ok(Hello {
        session,
        epoch: serde_json::from_value(v["epoch"].clone())?,
        head: serde_json::from_value(v["head"].clone())?,
        version,
        last_frame: Instant::now(),
    })
}
// Current-session typed activity only. A gap/fork/unknown state can pause but
// never clear foreground. Source closure remains the retained IR follower's job.
struct Foreground {
    next: u64,
    head: u64,
    known: bool,
    closed: bool,
    turns: std::collections::BTreeSet<Ulid>,
}
impl Foreground {
    fn new(head: u64) -> Self {
        Self {
            next: 1,
            head,
            known: true,
            closed: false,
            turns: Default::default(),
        }
    }
    fn observe(&mut self, value: &Value) -> Option<Option<bool>> {
        let seq = value["seq"].as_u64()?;
        if seq != self.next {
            self.known = false;
        } else {
            self.next = self.next.saturating_add(1);
        }
        let event = &value["event"];
        let kind = event["type"].as_str();
        if kind == Some("session_started")
            && (!event["meta"]["fork"].is_null() || !event["meta"]["source"].is_null())
        {
            self.known = false;
        }
        match kind {
            Some("turn_queued" | "turn_started" | "turn_ended") => {
                let Some(turn) = event["turn"]
                    .as_str()
                    .filter(|s| valid_session(s))
                    .and_then(|s| s.parse::<Ulid>().ok())
                else {
                    self.known = false;
                    return Some(None);
                };
                if kind == Some("turn_ended") {
                    self.turns.remove(&turn);
                } else {
                    if self.turns.len() >= 128 && !self.turns.contains(&turn) {
                        self.known = false;
                        return Some(None);
                    }
                    self.turns.insert(turn);
                    return Some(Some(true));
                }
            }
            Some("session_closed") if self.known && seq >= self.head && self.turns.is_empty() => {
                self.closed = true;
                return Some(Some(false));
            }
            Some("session_closed" | "session_recovered" | "lagged" | "reset") => {
                self.known = false;
                return Some(None);
            }
            _ => {}
        }
        if self.known && seq >= self.head {
            Some(Some(!self.turns.is_empty()))
        } else {
            None
        }
    }
}

async fn run(
    path: PathBuf,
    mut requests: mpsc::Receiver<Request>,
    state: watch::Sender<State>,
    stop: Arc<Stop>,
    wake: Arc<dyn Fn() + Send + Sync>,
    expected_pid: Option<u32>,
    activity: Option<Arc<dyn Fn(Option<bool>) + Send + Sync>>,
) {
    loop {
        let attempted = tokio::time::Instant::now();
        let result = tokio::select! {biased;_ = stop.notify.notified()=>return,r=tokio::time::timeout(RECONNECT, async {
            let meta=tokio::fs::symlink_metadata(&path).await?;
            if !meta.file_type().is_socket() || meta.uid()!=unsafe{libc::geteuid()} {return Err(std::io::Error::other("invalid harness socket owner/type"));}
            let identity=(meta.dev(),meta.ino());
            let stream=UnixStream::connect(&path).await?;
            let peer=stream.peer_cred()?;
            if peer.uid()!=unsafe{libc::geteuid()} || expected_pid.is_some_and(|pid|peer.pid().map(|p|p as u32)!=Some(pid)) {return Err(std::io::Error::other("harness peer identity mismatch"));}
            let (r,mut w)=stream.into_split();let mut r=BufReader::new(r);
            write(&mut w,&json!({"v":{"major":1,"minor":1},"subscribe":{"after":null,"ephemeral":false}})).await?;
            let hello=announcement(frame(&mut r).await?.ok_or_else(||std::io::Error::other("no Hello"))?)?;
            Ok::<_,std::io::Error>((r,w,hello,identity))
        })=>r};
        if let Ok(Ok((mut reader, mut writer, mut hello, identity))) = result {
            state.send_replace(State::Connected(hello.clone()));
            wake();
            if let Some(callback) = &activity {
                callback(None);
            }
            let mut foreground = Foreground::new(hello.head);
            let mut pending: BTreeMap<Ulid, Vec<(Instant, oneshot::Sender<WakeOutcome>)>> =
                BTreeMap::new();
            let mut clock = tokio::time::interval(Duration::from_millis(100));
            // Keep the frame future alive while a command/timer is serviced: fill_buf
            // may have consumed a partial line before yielding.
            'connected: loop {
                let next = frame(&mut reader);
                tokio::pin!(next);
                loop {
                    tokio::select! {
                        _=stop.notify.notified()=>return,
                        f=&mut next=>{
                            let Ok(Some(v))=f else{break 'connected;};
                            if !v["ack"].is_null() || !v["refused"].is_null() {
                                let Ok(id)=serde_json::from_value::<Ulid>(v["op_id"].clone())else{break 'connected;};
                                let outcome=if let Some(ack)=v.get("ack").filter(|a|!a.is_null()) {
                                    if ack["op_id"]!=json!(id)||!ack["duplicate"].is_boolean(){break 'connected;}
                                    WakeOutcome{evidence:Evidence::NativeQueued,native_id:Some(id.to_string()),note:format!("harness Ack duplicate={}; model consumption unverified",ack["duplicate"]),cause:Some(WakeCause::NativeQueueAdmitted)}
                                }else{match v["refused"]["reason"].as_str(){
                                    Some("busy")=>refused(WakeCause::HarnessBusy,"harness busy; retryable"),
                                    Some("closed"|"unknown_session")=>refused(WakeCause::HarnessSessionClosed,"harness session closed"),
                                    _=>refused(WakeCause::NativeRejected,"harness refused op"),
                                }};
                                hello.last_frame=Instant::now();
                                if let Some(waiters)=pending.remove(&id){for (_,tx) in waiters{let _=tx.send(outcome.clone());}}
                            }else{
                                // Every event must name this incarnation; late old-session data
                                // cannot renew admission. Heartbeats obey the same envelope shape.
                                if v["v"]["major"].as_u64().is_none_or(|m|m>1) {break 'connected;}
                                // Fork/resume replay carries original ancestry bytes. The
                                // harness UnixClient projects these for views; delivery has
                                // no projection, so ignore them without renewing admission.
                                if v["session"]!=json!(hello.session) || v["epoch"]!=json!(hello.epoch) {break;}
                                hello.last_frame=Instant::now();
                                if let Some(seq)=v["seq"].as_u64(){hello.head=hello.head.max(seq);}
                                if let Some(callback)=&activity && let Some(value)=foreground.observe(&v) {callback(value);}
                                if matches!(v["event"]["type"].as_str(),Some("session_closed"|"lagged"|"reset")){break 'connected;}
                            }
                            state.send_replace(State::Connected(hello.clone()));break;
                        },
                        request=requests.recv()=>{
                            let Some(req)=request else{return;};
                            if stop.retired.load(Ordering::Acquire){return;}
                            if hello.last_frame.elapsed()>=Duration::from_secs(20){let _=req.reply.send(unavailable());continue;}
                            if tokio::fs::symlink_metadata(&path).await.map(|m|(m.dev(),m.ino())).ok()!=Some(identity){let _=req.reply.send(unavailable());break 'connected;}
                        if req.session!=hello.session {let _=req.reply.send(refused(WakeCause::HarnessSessionClosed,"retained message belongs to a closed session"));continue;}
                            if req.op["type"]=="user_turn" && let Some(callback)=&activity {callback(Some(true));}
                            let value=json!({"v":hello.version,"op_id":req.id,"session":req.session,"op":req.op});
                            let result=tokio::time::timeout(ACK,write(&mut writer,&value)).await;
                            if !matches!(result,Ok(Ok(()))){let _=req.reply.send(WakeOutcome::unknown_cause(WakeCause::HarnessAckUnknown,"harness write/ack unknown; retain op_id"));break 'connected;}
                            pending.entry(req.id).or_default().push((Instant::now(),req.reply));
                        },
                        _=clock.tick()=>{
                            for waiters in pending.values_mut(){let mut keep=Vec::new();for (at,tx) in waiters.drain(..){if at.elapsed()>=ACK{let _=tx.send(WakeOutcome::unknown_cause(WakeCause::HarnessAckUnknown,"harness ack timed out; retain op_id"));}else if !tx.is_closed(){keep.push((at,tx));}}*waiters=keep;}
                            pending.retain(|_,v|!v.is_empty());
                            if hello.last_frame.elapsed()>=Duration::from_secs(20) || tokio::fs::symlink_metadata(&path).await.map(|m|(m.dev(),m.ino())).ok()!=Some(identity){break 'connected;}
                        }
                    }
                }
            }
            if !foreground.closed
                && let Some(callback) = &activity
            {
                callback(None);
            }
            for (_, waiters) in pending {
                for (_, tx) in waiters {
                    let _ = tx.send(WakeOutcome::unknown_cause(
                        WakeCause::HarnessAckUnknown,
                        "harness disconnected after write; retain op_id",
                    ));
                }
            }
            state.send_replace(State::Closed("harness socket disconnected".into()));
        } else {
            state.send_replace(State::Closed(
                "harness socket unavailable or invalid Hello".into(),
            ));
        }
        // Refuse queued pre-Hello requests immediately, never bind them later.
        while let Ok(req) = requests.try_recv() {
            let _ = req.reply.send(unavailable());
        }
        tokio::select! {_=stop.notify.notified()=>return,_=tokio::time::sleep_until(attempted+RECONNECT)=>{}}
    }
}
