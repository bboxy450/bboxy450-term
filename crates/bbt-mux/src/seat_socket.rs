// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Owner-only, capability-bound seat observation and semantic-control socket.
//! Raw input is unavailable here; messaging uses the common adapter/guard broker.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::os::unix::fs::{DirBuilderExt, FileTypeExt, MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncWrite, AsyncWriteExt, BufReader};
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
                                        let (read,mut write)=stream.into_split();
                                        let mut read=BufReader::new(read);
                                        if let Err(error)=refuse_gui(&mut read,&mut write,reason).await {
                                            eprintln!("GUI refusal ended: {}",gui_session_error_reason(&error));
                                        }
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
        private_parent(path)?;
        reclaim_stale_seat_socket(path)?;
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

/// Finish the bounded request before closing its read side, so a refused peer
/// can finish writing and read the explanation instead of losing it to EPIPE.
async fn refuse_gui(
    read: &mut (impl AsyncBufRead + Unpin),
    write: &mut (impl AsyncWrite + Unpin),
    reason: &'static str,
) -> Result<()> {
    // This is a hang watchdog, not a delay: a complete NDJSON frame drives progress.
    let frame = tokio::time::timeout(Duration::from_secs(10), read_line(read))
        .await??
        .ok_or_else(|| invalid("refused GUI peer closed before its request"))?;
    drop(frame);
    send(
        write,
        &GuiReply::Error {
            op: Ulid::nil(),
            reason: reason.into(),
        },
    )
    .await?;
    write.flush().await?;
    write.shutdown().await?;
    Ok(())
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
    } else if let Some(error) = error.downcast_ref::<GuiStorePersistent>() {
        error.to_string()
    } else if let Some(error) = error.downcast_ref::<rusqlite::Error>() {
        format!("GUI store error ({:?})", error.sqlite_error_code())
    } else {
        "GUI session failed".into()
    }
}

#[derive(Clone, Copy, Debug)]
enum GuiStoreOperation {
    Poll,
    Cursor,
    Refresh,
}
impl GuiStoreOperation {
    fn name(self) -> &'static str {
        match self {
            Self::Poll => "gui_poll",
            Self::Cursor => "gui_cursor",
            Self::Refresh => "gui_refresh",
        }
    }
}

#[derive(Debug)]
struct GuiStorePersistent {
    operation: GuiStoreOperation,
    extended_code: Option<i32>,
}
impl std::fmt::Display for GuiStorePersistent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "GuiStorePersistent: operation={} extended_code={:?}",
            self.operation.name(),
            self.extended_code
        )
    }
}
impl std::error::Error for GuiStorePersistent {}

/// One flag per read operation, never an accumulating retry queue. The session's
/// existing ticks decide when to retry; no request/effect is replayed here.
#[derive(Default)]
struct GuiStoreReads {
    busy: [bool; 3],
}
impl GuiStoreReads {
    fn read<T>(&mut self, operation: GuiStoreOperation, result: Result<T>) -> Result<Option<T>> {
        let busy = &mut self.busy[operation as usize];
        match result {
            Ok(value) => {
                *busy = false;
                Ok(Some(value))
            }
            Err(error) => {
                let Some(sqlite) = error.downcast_ref::<rusqlite::Error>() else {
                    return Err(error);
                };
                let (code, extended_code) = match sqlite {
                    rusqlite::Error::SqliteFailure(code, _) => {
                        (Some(code.code), Some(code.extended_code))
                    }
                    _ => (None, None),
                };
                if matches!(
                    code,
                    Some(rusqlite::ErrorCode::DatabaseBusy | rusqlite::ErrorCode::DatabaseLocked)
                ) {
                    if !*busy {
                        eprintln!(
                            "GUI store transient: operation={} extended_code={extended_code:?}; retry on next eligible tick",
                            operation.name()
                        );
                    }
                    *busy = true;
                    Ok(None)
                } else {
                    Err(GuiStorePersistent {
                        operation,
                        extended_code,
                    }
                    .into())
                }
            }
        }
    }
}

fn private_parent(path: &Path) -> Result<()> {
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
    Ok(())
}

/// Logic has exclusive run custody before recovery. An abrupt exit can leave
/// its socket inode behind; never replace a live listener or another file.
#[derive(Debug)]
pub(crate) struct SeatListenerPending;
impl std::fmt::Display for SeatListenerPending {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SeatListenerPending: seat endpoint still has a live or unresponsive listener; retry later")
    }
}
impl std::error::Error for SeatListenerPending {}

fn reclaim_stale_seat_socket(path: &Path) -> Result<()> {
    let meta = match std::fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    // SAFETY: geteuid has no pointer arguments or side effects.
    if !meta.file_type().is_socket() || meta.uid() != unsafe { libc::geteuid() } {
        return Err(invalid("seat endpoint is not an owned socket"));
    }
    match probe_seat_socket(path) {
        Err(error)
            if matches!(
                error.kind(),
                std::io::ErrorKind::ConnectionRefused | std::io::ErrorKind::NotFound
            ) => {}
        _ => {
            return Err(SeatListenerPending.into());
        }
    }
    let current = std::fs::symlink_metadata(path)?;
    if current.dev() != meta.dev() || current.ino() != meta.ino() {
        return Err(invalid("seat endpoint changed during recovery"));
    }
    std::fs::remove_file(path)?;
    Ok(())
}

/// One nonblocking connect: a busy listener is live, never a reason to wait or
/// unlink. This probe needs no running async executor during Logic bootstrap.
fn probe_seat_socket(path: &Path) -> std::io::Result<()> {
    use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};

    // SAFETY: sockaddr_un is a plain C address; all bytes are initialized.
    let mut address: libc::sockaddr_un = unsafe { std::mem::zeroed() };
    let bytes = path.as_os_str().as_encoded_bytes();
    if bytes.len() >= address.sun_path.len() || bytes.contains(&0) {
        return Err(std::io::Error::other("invalid seat endpoint path"));
    }
    address.sun_family = libc::AF_UNIX as libc::sa_family_t;
    for (slot, byte) in address.sun_path.iter_mut().zip(bytes) {
        *slot = *byte as libc::c_char;
    }
    let length = std::mem::size_of_val(&address) as libc::socklen_t;
    #[cfg(any(
        target_os = "macos",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd",
        target_os = "dragonfly"
    ))]
    {
        address.sun_len = length as u8;
    }
    // SAFETY: socket creates an unconnected owned descriptor, no borrowed pointers.
    let raw = unsafe { libc::socket(libc::AF_UNIX, libc::SOCK_STREAM, 0) };
    if raw < 0 {
        return Err(std::io::Error::last_os_error());
    }
    // SAFETY: raw is a fresh valid descriptor, transferred exactly once.
    let fd = unsafe { OwnedFd::from_raw_fd(raw) };
    // SAFETY: fcntl acts only on our live owned descriptor.
    if unsafe { libc::fcntl(fd.as_raw_fd(), libc::F_SETFL, libc::O_NONBLOCK) } < 0
        // SAFETY: same private owned descriptor; do not pass the probe to a child.
        || unsafe { libc::fcntl(fd.as_raw_fd(), libc::F_SETFD, libc::FD_CLOEXEC) } < 0
    {
        return Err(std::io::Error::last_os_error());
    }
    // SAFETY: address has the initialized sockaddr_un size and remains live;
    // fd stays owned until this nonblocking connect returns.
    if unsafe {
        libc::connect(
            fd.as_raw_fd(),
            (&address as *const libc::sockaddr_un).cast(),
            length,
        )
    } < 0
    {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

fn bind_listener(
    handle: &tokio::runtime::Handle,
    path: &Path,
) -> Result<(tokio::net::UnixListener, SocketFile)> {
    private_parent(path)?;
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

const INPUT_CHECK_INTERVAL: Duration = Duration::from_millis(2);
const INPUT_CHECK_WINDOW: Duration = Duration::from_millis(40);

struct InputCheck {
    generation: u64,
    baseline: Option<(View, u64)>,
    next: Instant,
    until: Instant,
}

/// Only an admitted input to an exact existing watch starts fast checks. The
/// ordinary 20 ms fallback remains unchanged once its first fresh snapshot lands.
/// Origin/credit: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
#[derive(Default)]
struct InputChecks(BTreeMap<bbt_proto::SeatId, InputCheck>);
impl InputChecks {
    fn admitted(
        &mut self,
        seat: bbt_proto::SeatId,
        generation: u64,
        watched: Option<&GuiView>,
        now: Instant,
    ) {
        let Some(watched) = watched.filter(|view| view.generation == generation) else {
            return;
        };
        self.0.insert(
            seat,
            InputCheck {
                generation,
                baseline: watched.last,
                next: now + INPUT_CHECK_INTERVAL,
                until: now + INPUT_CHECK_WINDOW,
            },
        );
    }

    fn queue_due(
        &mut self,
        views: &BTreeMap<bbt_proto::SeatId, GuiView>,
        checks: &mut GuiChecks,
        now: Instant,
    ) {
        self.0.retain(|seat, input| {
            let Some(watched) = views.get(seat) else {
                return false;
            };
            if watched.generation != input.generation
                || watched.last != input.baseline
                || now >= input.until
            {
                return false;
            }
            if watched.in_flight.is_none() && now >= input.next {
                checks.schedule(*seat);
                input.next = now + INPUT_CHECK_INTERVAL;
            }
            true
        });
    }

    fn deadline(&self, views: &BTreeMap<bbt_proto::SeatId, GuiView>) -> Option<Instant> {
        self.0
            .iter()
            .filter_map(|(seat, input)| {
                // Applied already checks immediately. Wait for that ACK instead of
                // spending fast timer wakes on an in-flight view.
                views
                    .get(seat)
                    .filter(|view| view.in_flight.is_none())
                    .map(|_| input.next.min(input.until))
            })
            .min()
    }
}

#[cfg(test)]
mod input_check_tests {
    use super::*;

    fn watched() -> GuiView {
        GuiView {
            stream: ViewStream::default(),
            generation: 7,
            last: Some((
                View {
                    seat: Ulid(1),
                    generation: 7,
                    view_generation: Ulid(2),
                },
                11,
            )),
            in_flight: None,
        }
    }

    #[test]
    fn fast_checks_are_exact_generation_bounded_and_stop_on_fresh_snapshot() {
        let now = Instant::now();
        let seat = bbt_proto::SeatId::from_u128(1);
        let mut views = BTreeMap::from([(seat, watched())]);
        let mut input = InputChecks::default();
        let mut checks = GuiChecks::default();
        input.admitted(seat, 6, views.get(&seat), now);
        input.admitted(bbt_proto::SeatId::from_u128(9), 7, None, now);
        assert!(input.deadline(&views).is_none());
        input.admitted(seat, 7, views.get(&seat), now);
        assert_eq!(input.deadline(&views), Some(now + INPUT_CHECK_INTERVAL));
        input.queue_due(&views, &mut checks, now);
        assert!(checks.is_empty());
        input.queue_due(&views, &mut checks, now + INPUT_CHECK_INTERVAL);
        assert_eq!(checks.next(), Some(seat));
        assert!(checks.is_empty());
        views.get_mut(&seat).unwrap().last.as_mut().unwrap().1 = 12;
        input.queue_due(&views, &mut checks, now + INPUT_CHECK_INTERVAL * 2);
        assert!(input.deadline(&views).is_none());
        assert!(checks.is_empty());
        input.admitted(seat, 7, views.get(&seat), now);
        input.queue_due(&views, &mut checks, now + INPUT_CHECK_WINDOW);
        assert!(input.deadline(&views).is_none());
        assert!(checks.is_empty());
    }

    #[test]
    fn fast_checks_wait_for_ack_and_cannot_follow_a_replaced_or_removed_watch() {
        let now = Instant::now();
        let seat = bbt_proto::SeatId::from_u128(1);
        let mut views = BTreeMap::from([(seat, watched())]);
        let mut input = InputChecks::default();
        let mut checks = GuiChecks::default();
        input.admitted(seat, 7, views.get(&seat), now);
        let baseline = views[&seat].last;
        views.get_mut(&seat).unwrap().in_flight = baseline;
        input.queue_due(&views, &mut checks, now + INPUT_CHECK_INTERVAL);
        assert!(input.deadline(&views).is_none());
        assert!(checks.is_empty());
        views.get_mut(&seat).unwrap().in_flight = None;
        input.queue_due(&views, &mut checks, now + INPUT_CHECK_INTERVAL);
        assert_eq!(checks.next(), Some(seat));
        views.get_mut(&seat).unwrap().generation = 8;
        input.queue_due(&views, &mut checks, now + INPUT_CHECK_INTERVAL * 2);
        assert!(input.deadline(&views).is_none());
        input.admitted(seat, 8, views.get(&seat), now);
        views.remove(&seat);
        input.queue_due(&views, &mut checks, now + INPUT_CHECK_INTERVAL);
        assert!(input.deadline(&views).is_none());
        assert!(input.0.is_empty());
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
async fn gui_read_events(
    client: &LogicClient,
    id: Ulid,
    stream: String,
    after: u64,
    limit: usize,
    wait_ms: Option<u64>,
) -> Result<GuiReply> {
    let filter = crate::gui_events::filters(client.gui_messaging_subjects().await?);
    let (envelopes, next, through) = client
        .bus
        .read_page_wait(&stream, filter, after, limit, wait_ms)
        .await?;
    let events = envelopes
        .into_iter()
        .map(|e| bbt_proto::gui::RetainedEvent {
            seq: e.seq,
            subject: e.event.subject,
            source: e.event.source,
            ts: e.event.ts,
            payload: e.event.payload,
        })
        .collect();
    Ok(GuiReply::EventsRead {
        op: id,
        stream,
        events,
        next,
        through,
    })
}

async fn gui_attach_lane(client: &LogicClient) -> Result<(GuiLane, Option<crate::bus::Delivery>)> {
    let reference = client.gui_device().await?;
    let filter = crate::gui_events::filters(client.gui_messaging_subjects().await?);
    let lease = client.bus.gui_attach(reference.id, filter).await?;
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
async fn gui_events_page(page: crate::bus::GuiPage, client: &LogicClient) -> Result<GuiReply> {
    use bbt_proto::events::{EventGap, GuiEventPage, GuiEventRecord};
    let mut records = Vec::new();
    let mut used = 0;
    let mut next = page.next;
    for delivery in page.records {
        let (record, before) = match delivery {
            crate::bus::Delivery::Event {
                stream, envelope, ..
            } => {
                let sequence = envelope.seq;
                let record = GuiEventRecord::Event(Box::new(
                    crate::gui_events::project_retained(client, &stream, *envelope).await?,
                ));
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
fn gui_knowledge_reply(
    op: Ulid,
    result: Result<crate::knowledge::data_source::Result<serde_json::Value>>,
) -> Result<GuiReply> {
    let response = result.unwrap_or_else(|_| {
        Err(crate::knowledge::data_source::CommandError::new(
            "provider_unavailable",
            "knowledge read interrupted; retry the read",
        ))
    });
    Ok(GuiReply::Knowledge {
        op,
        response: serde_json::to_value(response)?,
    })
}

type GuiKnowledgeScope = Option<(Ulid, u64)>;
/// Connection memory is bounded independently of each captured workspace.
/// Canonical-root provider permits remain the logic owner's authority.
#[derive(Default)]
struct GuiKnowledgeBudget {
    active: usize,
    scopes: BTreeMap<GuiKnowledgeScope, usize>,
}
impl GuiKnowledgeBudget {
    const CONNECTION: usize = 32;
    const SCOPE: usize = 2;
    fn key(scope: Option<bbt_proto::gui::SwarmScope>) -> GuiKnowledgeScope {
        scope.map(|scope| (scope.swarm, scope.revision))
    }
    fn admit(&mut self, scope: GuiKnowledgeScope) -> bool {
        if self.active >= Self::CONNECTION
            || self.scopes.get(&scope).copied().unwrap_or(0) >= Self::SCOPE
        {
            return false;
        }
        self.active += 1;
        *self.scopes.entry(scope).or_default() += 1;
        true
    }
    fn release(&mut self, scope: GuiKnowledgeScope) -> Result<()> {
        let Some(count) = self.scopes.get_mut(&scope) else {
            return Err(invalid("knowledge budget completion has no admission"));
        };
        *count -= 1;
        self.active -= 1;
        if *count == 0 {
            self.scopes.remove(&scope);
        }
        Ok(())
    }
}

async fn gui_session(
    stream: tokio::net::UnixStream,
    client: LogicClient,
    capability: [u8; 32],
    mut door: crate::mcp::Door,
) -> Result<()> {
    let peer_pid = stream
        .peer_cred()
        .ok()
        .and_then(|cred| cred.pid())
        .and_then(|pid| u32::try_from(pid).ok());
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
    // The first request selects a legacy/window connection or a permanently
    // restricted observer. An observer never subscribes to capture relays.
    let mut observer_mode: Option<bool> = None;
    let mut observer_reference: Option<bbt_proto::messaging::OperatorReference> = None;
    let mut attention_observer: Option<crate::attention_observer::AttentionSubscription> = None;
    let mut phase = "admission";
    let mut requests = 0_u64;
    let mut last_action = "none";
    let mut lane: Option<GuiLane> = None;
    let mut want_events = None;
    let mut lane_at: Option<Instant> = None;
    let mut events_at: Option<Instant> = None;
    let mut lane_error: Option<String> = None;
    let mut filters_at = Instant::now();
    let mut refresh_retry = false;
    let mut cursor_waits_for_tick = false;
    let mut store_reads = GuiStoreReads::default();
    let mut event_cursors = BTreeMap::new();
    let mut views: BTreeMap<bbt_proto::SeatId, GuiView> = BTreeMap::new();
    let watch_session = client.gui_watch_session();
    let mut checks = GuiChecks::default();
    let mut input_checks = InputChecks::default();
    let mut state_events = VecDeque::new();
    let mut actions = 0;
    let mut prefer_view = true;
    let mut tick = tokio::time::interval(Duration::from_millis(20));
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    // Only the window captures: its session relays the operator's asks.
    let mut capture_asks: Option<tokio::sync::broadcast::Receiver<crate::gui_capture::Notice>> =
        None;
    // Who issues this connection's operator requests, once it says (decision #269).
    let mut issued_by: Option<String> = None;
    // Knowledge is operation-correlated side work; ordinary controls keep their FIFO.
    // Dropping the session aborts these waiters, not a running provider's permit.
    let mut window_jobs: tokio::task::JoinSet<(Ulid, crate::store::Result<()>)> =
        tokio::task::JoinSet::new();
    let mut knowledge_reads = tokio::task::JoinSet::new();
    let mut knowledge_budget = GuiKnowledgeBudget::default();
    let mut debug_reads = tokio::task::JoinSet::new();
    let mut plugin_jobs = tokio::task::JoinSet::new();
    let mut root_jobs = tokio::task::JoinSet::new();
    let mut document_jobs = tokio::task::JoinSet::new();
    let mut swarm_jobs = tokio::task::JoinSet::new();
    let mut event_reads = tokio::task::JoinSet::new();
    let result:Result<()>=async {
        loop {
            input_checks.queue_due(&views,&mut checks,Instant::now());
            let input_at=input_checks.deadline(&views);
            tokio::select! {
                biased;
                Some(done)=window_jobs.join_next(),if !window_jobs.is_empty()=> {
                    let (op,result)=done.map_err(|e|invalid(format!("window state task stopped: {e}")))?;
                    send(&mut write,&GuiReply::WindowSaved {op,error:result.err().map(|e:crate::store::Error|e.to_string())}).await?;
                }
                line=incoming.recv(),if actions<ACTION_BURST&&(actions==0||checks.is_empty())=> {
                    actions+=1;
                    phase="request-read";
                    let line=match line {
                        Some(Ok(Some(line)))=>line,
                        Some(Err(error))=>return Err(error),
                        _=>break,
                    };
                    requests+=1;
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
                    let wants_observer=matches!(&request.action,GuiAction::GuiHello {observer:true,..});
                    if wants_observer {
                        let GuiAction::GuiHello {id,claim,..}=&request.action else {unreachable!()};
                        let allowed=if *claim || observer_mode==Some(false) {
                            Err(invalid("attention observer cannot claim or replace a GUI connection"))
                        } else {
                            peer_pid.ok_or_else(||invalid("attention observer peer PID unavailable"))
                                .and_then(|pid|client.admit_attention_observer(pid))
                        };
                        if let Err(error)=allowed {
                            send(&mut write,&GuiReply::Error {op:*id,reason:error.to_string()}).await?;
                            return Err(error);
                        }
                        observer_mode=Some(true);
                    } else if observer_mode==Some(true) {
                        if !matches!(&request.action,GuiAction::ObserveAttention {..}) {
                            send(&mut write,&GuiReply::Error {op:Ulid::nil(),reason:"attention observer is read-only".into()}).await?;
                            continue;
                        }
                    } else if matches!(&request.action,GuiAction::ObserveAttention {..}) {
                        send(&mut write,&GuiReply::Error {op:Ulid::nil(),reason:"ObserveAttention requires an observer hello".into()}).await?;
                        continue;
                    } else if observer_mode.is_none() {
                        observer_mode=Some(false);
                        if door==crate::mcp::Door::Gui {capture_asks=Some(client.captures.subscribe());}
                    }
                    last_action=match &request.action {
                        GuiAction::AttachEvents {..}=>"attach-events",
                        GuiAction::EventAck {..}=>"event-ack",
                        GuiAction::GetEventCursor {..}=>"event-cursor",
                        GuiAction::MarkEventsRead {..}=>"mark-events-read",
                        GuiAction::Watch {..}=>"watch",
                        GuiAction::Applied {..}=>"applied",
                        GuiAction::Input {..}=>"input",
                        GuiAction::Resize {..}=>"resize",
                        _=>"other-control",
                    };
                    phase="request-action";
                    if !admitted {
                        admitted=true;
                        send(&mut write,&GuiReply::Identity(client.gui_identity().await?)).await?;
                    }
                    if matches!(&request.action, GuiAction::SwarmList {..}|GuiAction::SwarmGet {..}|GuiAction::SwarmRename {..}|GuiAction::SwarmWorkdir {..}) {
                        let op = match &request.action {
                            GuiAction::SwarmList {id}|GuiAction::SwarmGet {id,..}|GuiAction::SwarmRename {id,..}|GuiAction::SwarmWorkdir {id,..} => *id,
                            _ => unreachable!(),
                        };
                        if swarm_jobs.len() >= 8 {
                            send(&mut write,&GuiReply::Error {op,reason:"swarm_busy: at most eight metadata requests per connection".into()}).await?;
                        } else {
                            let client=client.clone();
                            let origin=crate::mcp::Origin {door,issued_by:issued_by.clone()};
                            swarm_jobs.spawn(async move { (op,client.gui_swarm(request.action,origin).await) });
                        }
                        continue;
                    }
                    if let GuiAction::Knowledge {id,scope,method,args} = request.action {
                        let key=GuiKnowledgeBudget::key(scope);
                        if !knowledge_budget.admit(key) {
                            let mut error = crate::knowledge::data_source::CommandError::new(
                                "provider_busy", "this workspace already has two reads or the connection has 32 active reads",
                            );
                            error.retry_after_ms=Some(100);
                            send(&mut write,&gui_knowledge_reply(id,Ok(Err(error)))?).await?;
                        } else {
                            let client = client.clone();
                            knowledge_reads.spawn(async move { (id,key,client.gui_knowledge_scoped(method,args,scope).await) });
                        }
                        continue;
                    }
                    if matches!(&request.action, GuiAction::RecordWrite {..} | GuiAction::KnowledgeCommit {..} | GuiAction::Task {..}) {
                        let id = match &request.action {
                            GuiAction::RecordWrite {id,..} | GuiAction::KnowledgeCommit {id,..} | GuiAction::Task {id,..} => *id,
                            _ => unreachable!(),
                        };
                        if document_jobs.len() >= 8 {
                            send(&mut write, &GuiReply::Error {op:id,reason:"provider_busy: at most eight active document operations per connection; no effect admitted".into()}).await?;
                        } else {
                            let client=client.clone();
                            let origin=crate::mcp::Origin {door,issued_by:issued_by.clone()};
                            document_jobs.spawn(async move {
                                let result: Result<GuiReply> = async {
                                    match request.action {
                                        GuiAction::RecordWrite {id,scope,args} => Ok(gui_knowledge_reply(id,client.gui_record_write_scoped(id,args,scope,origin).await)?),
                                        GuiAction::KnowledgeCommit {id,scope,locators,message} => Ok(gui_knowledge_reply(id,client.gui_knowledge_commit(id,locators,message,scope,origin).await)?),
                                        GuiAction::Task {id,scope,request} => Ok(GuiReply::Task {op:id,response:client.gui_task_scoped(id,request,scope,origin).await?}),
                                        _ => unreachable!(),
                                    }
                                }.await;
                                (id,result)
                            });
                        }
                        continue;
                    }
                    if let GuiAction::Plugin {id,scope,request:query} = request.action {
                        if plugin_jobs.len() >= 8 {
                            send(&mut write,&GuiReply::Error {op:id,reason:"plugin_busy: at most eight active plugin controls per connection".into()}).await?;
                        } else {
                            let client = client.clone();
                            let origin = crate::mcp::Origin {door,issued_by:issued_by.clone()};
                            plugin_jobs.spawn(async move { (id,client.gui_plugin_scoped(id,query,scope,origin).await) });
                        }
                        continue;
                    }
                    if let GuiAction::KnowledgeControl {id,scope,request:query} = request.action {
                        if root_jobs.len() >= 8 {
                            send(&mut write,&GuiReply::Error {op:id,reason:"knowledge_busy: at most eight root controls per connection".into()}).await?;
                        } else {
                            let client=client.clone();
                            let origin=crate::mcp::Origin {door,issued_by:issued_by.clone()};
                            root_jobs.spawn(async move { (id,client.gui_knowledge_control_scoped(id,query,scope,origin).await) });
                        }
                        continue;
                    }
                    if let GuiAction::Debug {id,request:query} = request.action {
                        if door != crate::mcp::Door::Ctl || debug_reads.len() >= 2 {
                            send(&mut write,&GuiReply::Error {op:id,reason:"debug requires the ctl operator door and at most two active reads".into()}).await?;
                        } else {
                            let client = client.clone();
                            let origin = crate::mcp::Origin {door,issued_by:issued_by.clone()};
                            debug_reads.spawn(async move { (id,client.gui_debug(id,query,origin).await) });
                        }
                        continue;
                    }
                    if let GuiAction::EventsRead {id,stream,after,limit,wait_ms:Some(wait_ms)} = &request.action {
                        let id=*id;
                        if *wait_ms>60_000 || event_reads.len()>=2 {
                            send(&mut write,&GuiReply::Error {op:id,reason:"EventsRead allows wait_ms <= 60000 and at most two waiting reads per connection".into()}).await?;
                        } else {
                            let client=client.clone();
                            let (stream,after,limit,wait_ms)=(stream.clone(),*after,*limit,*wait_ms);
                            event_reads.spawn(async move {(id,gui_read_events(&client,id,stream,after,limit,Some(wait_ms)).await)});
                        }
                        continue;
                    }
                    if let GuiAction::WindowSave {id,epoch,state}=request.action {
                        if capture_asks.is_none() || !window_jobs.is_empty() {
                            send(&mut write,&GuiReply::WindowSaved {op:id,error:Some("window_state_busy_or_not_gui".into())}).await?;
                        } else {
                            let journal=client.window_journal.clone();
                            window_jobs.spawn(async move {
                                let result=match journal {Some(j)=>j.save(epoch,state).await,None=>Err(invalid("window journal unavailable"))};
                                (id,result)
                            });
                        }
                        continue;
                    }
                    let stopping = matches!(&request.action, GuiAction::App {request:bbt_proto::gui::AppRequest::Stop,..});
                    let op=match &request.action {
                        GuiAction::ConfigureRoles {id,..}|GuiAction::WindowSave {id,..}|GuiAction::GuiHello {id,..}|GuiAction::ObserveAttention {id}|GuiAction::Ui {id,..}|GuiAction::UiReply {id,..}|GuiAction::App {id,..}|GuiAction::AppReply {id,..}|GuiAction::Focused {id,..}=>*id,
                        GuiAction::Identity(bbt_proto::identity::IdentityCall {id,..})|GuiAction::AttachEvents {id}|GuiAction::Spawn {id,..}|GuiAction::Messaging {id,..}|GuiAction::Task {id,..}|GuiAction::Input {id,..}|GuiAction::ScheduleClose {id,..}|GuiAction::Capture {id,..}|GuiAction::Captured {id,..}|GuiAction::Knowledge {id,..}|GuiAction::Screen {id,..}|GuiAction::EventsRead {id,..}|GuiAction::EventsStreams {id}|GuiAction::GetAttention {id,..}|GuiAction::SetAttention {id,..}|GuiAction::RenameSeat {id,..}|GuiAction::SeatControl {id,..}|GuiAction::FocusSeat {id,..}|GuiAction::RecordWrite {id,..}|GuiAction::KnowledgeCommit {id,..}|GuiAction::RestartSeat {id,..}|GuiAction::CloseSeat {id,..}|GuiAction::UndoCloseSeat {id,..}=>*id,
                        GuiAction::UndoClose {close_id,..}=>*close_id,
                        _=>Ulid::new(),
                    };
                    let reply:Result<GuiReply>=async {
                        match request.action {
                            GuiAction::SwarmList {..}|GuiAction::SwarmGet {..}|GuiAction::SwarmRename {..}|GuiAction::SwarmWorkdir {..}=>unreachable!("swarm metadata dispatched independently"),
                            GuiAction::WindowSave {..}=>unreachable!("window saves handled before state dispatch"),
                            GuiAction::GuiHello {id,claim,observer}=> {
                                if observer {observer_reference=Some(client.gui_device().await?);}
                                if claim {
                                    client.claim_gui(peer_pid.ok_or_else(||invalid("native GUI peer PID unavailable"))?)?;
                                    door=crate::mcp::Door::Gui;
                                    if capture_asks.is_none() {capture_asks=Some(client.captures.subscribe());}
                                }
                                let mut hello=client.gui_hello().await?;
                                if claim {
                                    hello.window=Some(client.window_journal.as_ref().ok_or_else(||invalid("window journal unavailable"))?.claim().await?);
                                }
                                Ok(GuiReply::GuiHello {op:id,hello})
                            }
                            GuiAction::ObserveAttention {id}=> {
                                client.admit_attention_observer(peer_pid.ok_or_else(||invalid("attention observer peer PID unavailable"))?)?;
                                if observer_reference.as_ref().map(|reference|reference.id)!=Some(client.gui_device().await?.id) {
                                    return Err(invalid("attention observer device changed; reconnect"));
                                }
                                let mut subscription=client.observe_attention();
                                let update=subscription.next().await?;
                                attention_observer=Some(subscription);
                                Ok(GuiReply::CurrentAttention {op:Some(id),update})
                            }
                            GuiAction::Ui {id,request}=> {
                                if door!=crate::mcp::Door::Ctl {return Err(invalid("UI requests use the ctl operator door"));}
                                let layout=client.ctl_ui(id,request,crate::mcp::Origin {door,issued_by:issued_by.clone()}).await?;
                                Ok(GuiReply::Ui {op:id,layout})
                            }
                            GuiAction::UiReply {id,result}=> {
                                if capture_asks.is_none() {return Err(invalid("only the GUI owner answers UI requests"));}
                                client.captures.ui_answer(id,*result);
                                Ok(GuiReply::Ok {op:id})
                            }
                            GuiAction::App {id,request}=> {
                                if door!=crate::mcp::Door::Ctl && request!=bbt_proto::gui::AppRequest::Activate {return Err(invalid("app lifecycle requests use the ctl operator door"));}
                                let status=client.captures.app(id,request).await.map_err(invalid)?;
                                Ok(GuiReply::App {op:id,status})
                            }
                            GuiAction::AppReply {id,status}=> {
                                if capture_asks.is_none() {return Err(invalid("only the GUI client answers app lifecycle requests"));}
                                client.captures.app_answer(id,status);
                                Ok(GuiReply::Ok {op:id})
                            }
                            GuiAction::Focused {id,result}=> {
                                if capture_asks.is_none() {return Err(invalid("only the GUI client acknowledges focus"));}
                                client.captures.focused(id,result);
                                Ok(GuiReply::Ok {op:id})
                            }
                            GuiAction::ListSeats=> {
                                let mut seats=client.gui_seats().await?;
                                if door==crate::mcp::Door::Ctl {
                                    for seat in &mut seats {
                                        let observed=seat.attention.take();
                                        let mut view=crate::mcp::attention::view(&seat.record).map_err(invalid)?;
                                        if let Some(observed)=observed { for (key,value) in observed.as_object().into_iter().flatten() { view[key]=value.clone(); } }
                                        seat.attention=Some(view);
                                    }
                                }
                                Ok(GuiReply::Seats(seats))
                            },
                            GuiAction::ListHosts=>Ok(GuiReply::Hosts(client.gui_hosts().await?)),
                            GuiAction::Identity(bbt_proto::identity::IdentityCall {id,request})=>Ok(GuiReply::IdentityControl {op:id,response:client.gui_identity_control(id,request,crate::mcp::Origin {door,issued_by:issued_by.clone()}).await?}),
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
                                gui_events_page(client.bus.gui_page(&active.lease,&stream,after,limit,crate::gui_events::PAGE_BYTES).await?, &client).await
                            }
                            GuiAction::ConfigureRoles {id,catalog}=> {
                                client.configure_roles(id,catalog,crate::mcp::Origin {door,issued_by:issued_by.clone()}).await?;
                                Ok(GuiReply::RolesConfigured {id})
                            }
                            GuiAction::Spawn {id,spec}=> {
                                let seat=client.gui_spawn(id,spec.clone(),door).await?;
                                // The window places a seat another door spawned, as its own pick.
                                if door==crate::mcp::Door::Ctl {client.captures.spawned(id,seat,spec);}
                                Ok(GuiReply::Spawned {op:id,seat})
                            }
                            GuiAction::Messaging {id,swarm_id,request}=> {
                                let mute_seat=match &request {
                                    bbt_proto::messaging::Request::Mute {seat,..} if door==crate::mcp::Door::Gui=>Some(*seat),
                                    _=>None,
                                };
                                let response=client.gui_messaging(id,Ulid::from(swarm_id.0.0),request,crate::mcp::Origin {door,issued_by:issued_by.clone()}).await?;
                                if matches!(&response,bbt_proto::messaging::Response::Muted {..}) && let Some(seat)=mute_seat {
                                    // Read the committed metadata before the receipt. The
                                    // window's own write must not wait for its event backlog.
                                    let current=client.gui_seat(seat).await.map_err(|error|invalid(format!("unknown: mute committed; current seat state unavailable: {error}")))?;
                                    send(&mut write,&GuiReply::Seat(current)).await?;
                                }
                                Ok(GuiReply::Messaging {op:id,response})
                            },
                            GuiAction::Task {..}=>unreachable!("tasks dispatched independently"),
                            GuiAction::Knowledge {..}=>unreachable!("knowledge was dispatched independently"),
                            GuiAction::Plugin {..}=>unreachable!("plugins were dispatched independently"),
                            GuiAction::KnowledgeControl {..}=>unreachable!("root controls were dispatched independently"),
                            GuiAction::Debug {..}=>unreachable!("debug was dispatched independently"),
                            GuiAction::RecordWrite {..}|GuiAction::KnowledgeCommit {..}=>unreachable!("document effects dispatched independently"),
                            GuiAction::Watch {seat,view}=> {
                                if view.seat!=Ulid::from(seat.0.0) {return Err(invalid("watch seat and view disagree"));}
                                if views.len()>=1024&&!views.contains_key(&seat) {return Err(invalid("watch limit reached"));}
                                let snapshot=client.snapshot(view.seat,view.generation).await?;
                                let last=Some((snapshot.view,snapshot.sequence));
                                let mut stream=ViewStream::default();stream.offer(snapshot)?;
                                client.gui_watch_interest(&watch_session,view.seat,Some(view.generation)).await?;
                                views.insert(seat,GuiView {stream,generation:view.generation,last,in_flight:None});
                                input_checks.0.remove(&seat);
                                checks.schedule(seat);
                                Ok(GuiReply::Ok {op})
                            }
                            GuiAction::Unwatch {seat}=>{
                                views.remove(&seat);checks.remove(seat);input_checks.0.remove(&seat);
                                client.gui_watch_interest(&watch_session,Ulid::from(seat.0.0),None).await?;
                                Ok(GuiReply::Ok {op})
                            }
                            GuiAction::Applied {seat,view,sequence}=> {
                                if view.seat!=Ulid::from(seat.0.0) {return Err(invalid("ack seat and view disagree"));}
                                let watched=views.get_mut(&seat).ok_or_else(||invalid("seat is not watched"))?;
                                watched.applied(view,sequence)?;
                                checks.remove(seat);
                                let result=gui_check_view(&client,&mut write,seat,watched).await;
                                if result.is_err() {
                                    views.remove(&seat);
                                    let _=client.gui_watch_interest(&watch_session,Ulid::from(seat.0.0),None).await;
                                }
                                result?;
                                Ok(GuiReply::Ok {op})
                            }
                            GuiAction::Input {seat,generation,id,bytes}=> {
                                if door==crate::mcp::Door::Ctl {
                                    client.ctl_input(Ulid::from(seat.0.0),generation,id,bytes,crate::mcp::Origin {door,issued_by:issued_by.clone()}).await?;
                                    input_checks.admitted(seat,generation,views.get(&seat),Instant::now());
                                    return Ok(GuiReply::Ok {op:id});
                                }
                                let receipt=client.gui_input(Ulid::from(seat.0.0),generation,id,bytes).await?;
                                if receipt.state==crate::InputState::PartialUnknown {return Err(invalid("input result is partial/unknown; keep the original id"));}
                                input_checks.admitted(seat,generation,views.get(&seat),Instant::now());
                                Ok(GuiReply::Ok {op:id})
                            }
                            GuiAction::GetAttention {id,seat,generation}=> {
                                Ok(GuiReply::Attention {op:id,seat:client.gui_attention(seat,Some(generation)).await?})
                            }
                            GuiAction::SetAttention {id,seat,edit}=> {
                                client.ctl_attention(seat,edit,id,crate::mcp::Origin {door,issued_by:issued_by.clone()}).await?;
                                let seat=client.gui_attention(seat,None).await?;
                                Ok(GuiReply::Attention {op:id,seat})
                            }
                            GuiAction::RenameSeat {id,seat,name}=> {
                                client.ctl_rename(seat,name,id,crate::mcp::Origin {door,issued_by:issued_by.clone()}).await?;
                                Ok(GuiReply::Ok {op:id})
                            }
                            GuiAction::RestartSeat {id,seat,generation}=> {
                                client.ctl_restart(seat,generation,id,crate::mcp::Origin {door,issued_by:issued_by.clone()}).await?;
                                Ok(GuiReply::Ok {op:id})
                            }
                            GuiAction::SeatControl {id,seat,generation,command}=> {
                                client.seat_control(seat,generation,id,command,crate::mcp::Origin {door,issued_by:issued_by.clone()}).await?;
                                if door==crate::mcp::Door::Gui {
                                    // The GUI's cached listing must observe its own
                                    // committed metadata before the correlated receipt.
                                    let current=client.gui_seat(seat).await.map_err(|error|invalid(format!("unknown: seat control committed; current seat state unavailable: {error}; retain operation ID {id}")))?;
                                    send(&mut write,&GuiReply::Seat(current)).await?;
                                }
                                Ok(GuiReply::SeatControlled {op:id})
                            }
                            action @ (GuiAction::CloseSeat {..}|GuiAction::UndoCloseSeat {..})=> {
                                let (id,seat,undo)=match action { GuiAction::CloseSeat {id,seat}=>(id,seat,false),GuiAction::UndoCloseSeat {id,seat}=>(id,seat,true),_=>unreachable!() };
                                client.record(seat).await?;
                                if !client.captures.attached() {return Err(invalid("no window is attached to close or undo"));}
                                client.ctl_record(id,if undo {"seat_undo_close"} else {"seat_close"},serde_json::json!({"seat":seat}),crate::mcp::Origin {door,issued_by:issued_by.clone()}).await?;
                                let layout=client.captures.close(id,bbt_proto::SeatId::from_u128(seat.0),undo).await.map_err(invalid)?;
                                Ok(GuiReply::Ui {op:id,layout})
                            }
                            GuiAction::FocusSeat {id,seat}=> {
                                client.record(seat).await?;
                                if !client.captures.attached() {return Err(invalid("no window is attached to focus"));}
                                client.ctl_record(id,"seat_focus",serde_json::json!({"seat":seat}),crate::mcp::Origin {door,issued_by:issued_by.clone()}).await?;
                                let frame=client.captures.focus(id,bbt_proto::SeatId::from_u128(seat.0)).await.map_err(invalid)?;
                                Ok(GuiReply::Focused {op:id,frame})
                            }
                            GuiAction::EventsRead {id,stream,after,limit,wait_ms}=>gui_read_events(&client,id,stream,after,limit,wait_ms).await,
                            GuiAction::EventsStreams {id}=> {
                                // The operator can browse every registered stream; the
                                // current subject grants still filter each EventsRead.
                                client.gui_messaging_subjects().await?;
                                let streams=client.bus.read_streams().await?;
                                if serde_json::to_vec(&streams)?.len()>crate::gui_events::PAGE_BYTES {
                                    return Err(invalid("retained stream names exceed the page bound"));
                                }
                                Ok(GuiReply::EventsStreams {op:id,streams})
                            }
                            GuiAction::Screen {id,seat,cells}=>Ok(GuiReply::Screen {op:id,screen:client.gui_screen(seat,cells).await?}),
                            GuiAction::Resize {seat,generation,cols,rows}=> {
                                if door==crate::mcp::Door::Ctl {
                                    // Journaled under the op this reply returns; a resize to the
                                    // same size is idempotent, so a retry needs no --op.
                                    client.ctl_resize(Ulid::from(seat.0.0),generation,cols,rows,op,crate::mcp::Origin {door,issued_by:issued_by.clone()}).await?;
                                } else {
                                    client.gui_resize(Ulid::from(seat.0.0),generation,cols,rows).await?;
                                }
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
                                let (history,masks)=client.history_for_gui(id,record.generation,from,to).await?;
                                gui_history_reply(seat,record.generation,from,to,history,&masks)
                            }
                        }
                    }.await;
                    let reply=reply.unwrap_or_else(|e|GuiReply::Error {op,reason:e.to_string()});
                    let stop=stopping&&matches!(&reply,GuiReply::App {..});
                    phase="request-reply";
                    send(&mut write,&reply).await?;
                    // The ctl response is on the socket before stopping its server.
                    if stop {client.captures.app_stopping(op);}
                }
                Some(completed)=knowledge_reads.join_next(),if !knowledge_reads.is_empty()=> {
                    let (op,key,response)=completed.map_err(|_|invalid("knowledge reply task interrupted"))?;
                    knowledge_budget.release(key)?;
                    send(&mut write,&gui_knowledge_reply(op,response)?).await?;
                }
                _=tokio::time::sleep_until(tokio::time::Instant::from_std(input_at.unwrap_or_else(Instant::now))),if input_at.is_some()=> {}
                Some(completed)=document_jobs.join_next(),if !document_jobs.is_empty()=> {
                    let (op,result)=completed.map_err(|e|invalid(format!("document task stopped: {e}; admitted effects remain unknown")))?;
                    let reply=result.unwrap_or_else(|error|GuiReply::Error {op,reason:error.to_string()});
                    send(&mut write,&reply).await?;
                }
                Some(completed)=plugin_jobs.join_next(),if !plugin_jobs.is_empty()=> {
                    let (op,result)=completed.map_err(|_|invalid("plugin reply task interrupted"))?;
                    let reply=match result {
                        Ok(entries)=>GuiReply::Plugin {op,entries},
                        Err(error)=>GuiReply::Error {op,reason:error.to_string()},
                    };
                    send(&mut write,&reply).await?;
                }
                Some(completed)=root_jobs.join_next(),if !root_jobs.is_empty()=> {
                    let (op,result)=completed.map_err(|_|invalid("root control task interrupted"))?;
                    let reply=match result {
                        Ok(status)=>GuiReply::KnowledgeControl {op,status},
                        Err(error)=>GuiReply::Error {op,reason:error.to_string()},
                    };
                    send(&mut write,&reply).await?;
                }
                Some(completed)=swarm_jobs.join_next(),if !swarm_jobs.is_empty()=> {
                    let (op,result)=completed.map_err(|_|invalid("swarm metadata task interrupted"))?;
                    let reply=result.unwrap_or_else(|error|GuiReply::Error {op,reason:error.to_string()});
                    send(&mut write,&reply).await?;
                }
                Some(completed)=debug_reads.join_next(),if !debug_reads.is_empty()=> {
                    let (op,result)=completed.map_err(|_|invalid("debug reply task interrupted"))?;
                    let reply=match result {
                        Ok(data)=>GuiReply::Debug {op,data},
                        Err(error)=>GuiReply::Error {op,reason:crate::debug::sanitize(serde_json::json!(error.to_string())).as_str().unwrap_or("debug read refused").into()},
                    };
                    send(&mut write,&reply).await?;
                }
                Some(completed)=event_reads.join_next(),if !event_reads.is_empty()=> {
                    let (op,result)=completed.map_err(|_|invalid("waiting event read interrupted"))?;
                    let reply=result.unwrap_or_else(|error|GuiReply::Error {op,reason:error.to_string()});
                    send(&mut write,&reply).await?;
                }
                ask=async {capture_asks.as_mut().unwrap().recv().await},if admitted&&capture_asks.is_some()=> {
                    match ask {
                        Ok(notice)=>send(&mut write,&notice.reply()).await?,
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(_))=>{}
                        Err(tokio::sync::broadcast::error::RecvError::Closed)=>capture_asks=None,
                    }
                }
                update=async {attention_observer.as_mut().unwrap().next().await},if admitted&&attention_observer.is_some()=> {
                    let update=update?;
                    client.admit_attention_observer(peer_pid.ok_or_else(||invalid("attention observer peer PID unavailable"))?)?;
                    if observer_reference.as_ref().map(|reference|reference.id)!=Some(client.gui_device().await?.id) {
                        return Err(invalid("attention observer device changed; reconnect"));
                    }
                    send(&mut write,&GuiReply::CurrentAttention {op:None,update}).await?;
                }
                _=tick.tick(),if admitted && observer_mode!=Some(true)=> {
                    cursor_waits_for_tick=false;
                    if (refresh_retry||filters_at.elapsed()>=Duration::from_secs(1)) && let Some(active)=&mut lane {
                        refresh_retry=false;
                        filters_at=Instant::now();
                        phase="events-refresh";
                        let filters=async {
                            if client.gui_device().await?.id!=active.lease.reference {
                                return Err(invalid("GuiOperatorChanged: reattaching current local principal"));
                            }
                            client.gui_messaging_subjects().await
                        }.await;
                        match filters {
                            Ok(filters)=> {
                                if let Some(next)=store_reads.read(GuiStoreOperation::Refresh,client.bus.gui_refresh(&active.lease,crate::gui_events::filters(filters)).await)? {
                                    if next!=active.lease {
                                        active.lease=next;state_events.clear();event_cursors.clear();events_at=None;
                                    }
                                } else {
                                    refresh_retry=true;
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
                        phase="events-poll";
                        if let Some(events)=store_reads.read(GuiStoreOperation::Poll,client.bus.gui_poll(&active.lease,crate::gui_events::PAGE_RECORDS,crate::gui_events::PAGE_BYTES).await)? {
                            state_events.extend(events);
                            event_cursors.clear();
                        }
                        events_at=Some(Instant::now());
                    }
                    for &seat in views.keys() {checks.schedule(seat);}
                }
                _=async {},if !checks.is_empty()||(!state_events.is_empty()&&!cursor_waits_for_tick)=> {
                    // One awaited unit, then incoming actions are polled again.
                    // Alternate state and pane work so neither can monopolize.
                    if !checks.is_empty()&&(prefer_view||state_events.is_empty()||cursor_waits_for_tick) {
                        let seat=checks.next().unwrap();
                        if let Some(watched)=views.get_mut(&seat)
                            && let Err(error)=gui_fallback_check(&client,&mut write,seat,watched).await {
                            views.remove(&seat);
                            let _=client.gui_watch_interest(&watch_session,Ulid::from(seat.0.0),None).await;
                            send(&mut write,&GuiReply::Error {op:Ulid::nil(),reason:error.to_string()}).await?;
                        }
                        prefer_view=false;
                    } else if let Some(event)=state_events.front() {
                        let cursor=if let crate::bus::Delivery::Event {stream,envelope,..}=event {
                            if envelope.event.subject.ends_with("/message")||envelope.event.subject.ends_with("/receipt") {
                                if !event_cursors.contains_key(stream) && let Some(active)=&lane {
                                    phase="events-cursor";
                                    if let Some(cursor)=store_reads.read(GuiStoreOperation::Cursor,client.bus.gui_cursor(&active.lease,stream).await)? {
                                        event_cursors.insert(stream.clone(),crate::gui_events::cursor(cursor));
                                    } else {
                                        cursor_waits_for_tick=true;prefer_view=true;actions=0;
                                        continue;
                                    }
                                }
                                event_cursors.get(stream).cloned()
                            } else {None}
                        } else {None};
                        let event=state_events.pop_front().expect("front event retained until its cursor is ready");
                        phase="event-project-write";
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
    if let Err(error) = &result {
        eprintln!(
            "GUI boundary failed: owner_pid={} peer_pid={peer_pid:?} phase={phase} requests={requests} last_action={last_action} admitted={admitted} events_attached={} reason={}",
            std::process::id(),
            lane.is_some(),
            gui_session_error_reason(error),
        );
    }
    if let Err(error) = &result
        && error.is::<GuiStorePersistent>()
    {
        let _ = send(
            &mut write,
            &GuiReply::EventStatus {
                attached: false,
                reason: Some(gui_session_error_reason(error)),
            },
        )
        .await;
    }
    drop(watch_session);
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
    masks: &[crate::history_fault::Missing],
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
    let mut next = history.next;
    crate::history_fault::project(&mut records, &mut next, to, masks);
    let reply = GuiReply::History {
        seat,
        records,
        next,
    };
    if serde_json::to_vec(&reply)?.len() >= MAX_LINE {
        return Err(invalid(
            "HistoryFrameTooLarge: capture record exceeds GUI framing",
        ));
    }
    Ok(reply)
}

pub async fn read_line(reader: &mut (impl AsyncBufRead + Unpin)) -> Result<Option<Vec<u8>>> {
    read_bounded_line(reader, MAX_LINE).await
}
/// GUI snapshots use the GUI reply budget; seat/MCP requests retain 1 MiB.
pub async fn read_gui_line(reader: &mut (impl AsyncBufRead + Unpin)) -> Result<Option<Vec<u8>>> {
    read_bounded_line(reader, bbt_proto::gui::MAX_REPLY_BYTES).await
}
async fn read_bounded_line(
    reader: &mut (impl AsyncBufRead + Unpin),
    limit: usize,
) -> Result<Option<Vec<u8>>> {
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
        if line.len() + count > limit {
            return Err(invalid(if limit == MAX_LINE {
                "NDJSON frame exceeds 1 MiB"
            } else {
                "GUI reply exceeds 64 MiB"
            }));
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
async fn send(writer: &mut (impl AsyncWrite + Unpin), value: &impl Serialize) -> Result<()> {
    send_bounded(writer, value, bbt_proto::gui::MAX_REPLY_BYTES).await
}
async fn send_seat(writer: &mut (impl AsyncWrite + Unpin), value: &impl Serialize) -> Result<()> {
    send_bounded(writer, value, MAX_LINE).await
}
async fn send_bounded(
    writer: &mut (impl AsyncWrite + Unpin),
    value: &impl Serialize,
    limit: usize,
) -> Result<()> {
    let mut bytes = serde_json::to_vec(value)?;
    if bytes.len() >= limit {
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
                match result {Ok(value)=>send_seat(&mut write,&value).await?,Err(error)=>send_seat(&mut write,&serde_json::json!({"kind":"Rejected","reason":error.to_string()})).await?,}
            }
            _=tick.tick(),if subscribed=> {
                let snapshot=client.snapshot(seat,generation).await?;
                let identity=(snapshot.view,snapshot.sequence);
                if last!=Some(identity){view.offer(snapshot)?;last=Some(identity);}
                if let Some(update)=view.next_update(){send_seat(&mut write,&update).await?;}
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
mod attention_tests;
#[cfg(test)]
mod gui_tests {
    use super::*;
    use crate::{
        LaunchSpec, Supervisor,
        logic::{Logic, LogicConfig},
        registry::SeatRecord,
    };

    // keeps: spec c1-live-seats; large GUI replies do not widen seat/MCP requests
    #[tokio::test]
    async fn gui_reply_budget_accepts_large_frames_without_widening_seat_requests() {
        let mut bytes = vec![b'x'; (1 << 20) + 16];
        bytes.push(b'\n');
        let mut seat_reader = bytes.as_slice();
        assert!(read_line(&mut seat_reader).await.is_err());
        let mut gui_reader = bytes.as_slice();
        assert_eq!(
            read_gui_line(&mut gui_reader).await.unwrap().unwrap().len(),
            bytes.len() - 1
        );
    }

    // keeps: SeatControl metadata is authoritative before receipt without event/model publication.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ignore = "private real supervisor/GUI role boundary; make test-seat-actions-backend"]
    async fn seat_control_role_snapshot_precedes_receipt_without_events() {
        struct Keys;
        impl crate::identity::DeviceKeyStore for Keys {
            fn custody(&self) -> crate::identity::Custody {
                crate::identity::Custody::Memory
            }
            fn load(&self, _: Ulid) -> crate::identity::Result<Option<ed25519_dalek::SigningKey>> {
                Ok(Some(ed25519_dalek::SigningKey::from_bytes(&[92; 32])))
            }
            fn create(
                &self,
                _: Ulid,
                _: &ed25519_dalek::SigningKey,
            ) -> crate::identity::Result<()> {
                Err(crate::identity::PendingReason::JournalConflict)
            }
        }
        let root = tempfile::tempdir().unwrap();
        let repository = root.path().join("repository");
        let home = root.path().join("home");
        std::fs::create_dir(&repository).unwrap();
        std::fs::create_dir(&home).unwrap();
        let run = root.path().join("run");
        assert!(matches!(
            crate::identity::local_dev_issuer_with_store(&run, &Keys),
            crate::identity::LocalIssuerOutcome::Ready(_)
        ));
        let supervisor = Supervisor::start(crate::Config {
            journal_directory: Some(root.path().join("supervisor-journal")),
            ..Default::default()
        })
        .unwrap();
        let logic = Logic::start(
            supervisor.connect().unwrap(),
            LogicConfig {
                run_dir: run,
                repository: repository.clone(),
                ring: Default::default(),
            },
            ed25519_dalek::SigningKey::from_bytes(&[92; 32]),
            &tokio::runtime::Handle::current(),
        )
        .unwrap();
        let mut catalog = bbt_proto::gui::RoleCatalog::builtin();
        let prompt = catalog.roles["executor"].prompt.clone();
        catalog.harnesses.insert(
            "cat".into(),
            bbt_proto::gui::HarnessRoles {
                default_role: "executor".into(),
                available_roles: vec!["executor".into(), "architect".into()],
            },
        );
        logic
            .client
            .configure_roles(Ulid::new(), catalog, crate::mcp::Origin::default())
            .await
            .unwrap();
        let seat = Ulid::new();
        let swarm = Ulid::new();
        let harness = crate::adapters::HarnessRecord {
            kind: "cat".into(),
            name: "role fixture cat".into(),
            command: vec!["/bin/cat".into()],
            tile: "#7397d4".into(),
            icon: "cat".into(),
            default_role: crate::adapters::role_preset("executor").unwrap(),
            wake_method: crate::adapters::WakeMethod::Pty,
            state_source: crate::adapters::StateSource::Cadence,
            hooks: false,
        };
        harness.validate().unwrap();
        let info = logic
            .client
            .spawn_harness(
                SeatRecord::new(seat, swarm, "role fixture", "cat"),
                harness,
                crate::adapters::launch::LaunchOptions {
                    via: None,
                    env: vec![("PATH".into(), "/usr/bin:/bin".into())],
                    cwd: repository,
                    role_prompt: prompt,
                    executable: std::env::current_exe().unwrap(),
                    user_home: home.clone(),
                    user_zdotdir: Some(home),
                    resume: false,
                },
            )
            .await
            .unwrap();
        let result = tokio::time::timeout(Duration::from_secs(30), async {
            for door in [crate::mcp::Door::Gui, crate::mcp::Door::Ctl] {
                let capability = [75; 32];
                let (peer, server) = tokio::net::UnixStream::pair()?;
                let session = tokio::spawn(gui_session(server, logic.client.clone(), capability, door));
                let (read, mut write) = peer.into_split();
                let mut read = BufReader::new(read);
                let checked: Result<()> = async {
                    // No AttachEvents and no GUI Model exist in this fixture.
                    for role in ["architect", "executor"] {
                        let id = Ulid::new();
                        let next = Ulid::new();
                        send(&mut write, &GuiRequest { version: bbt_proto::gui::VERSION, capability, action: GuiAction::SeatControl { id, seat, generation: 1, command: bbt_proto::gui::SeatControl::Role { name: role.into() } } }).await?;
                        send(&mut write, &GuiRequest { version: bbt_proto::gui::VERSION, capability, action: GuiAction::RenameSeat { id: next, seat, name: "role fixture".into() } }).await?;
                        let mut snapshot = false;
                        let mut receipt = false;
                        loop {
                            let line = read_line(&mut read).await?.ok_or_else(|| invalid("role connection closed"))?;
                            match serde_json::from_slice::<GuiReply>(&line)? {
                                GuiReply::Identity(_) => {},
                                GuiReply::Seat(current) => {
                                    if door != crate::mcp::Door::Gui || receipt || current.record.id != seat || current.record.role.name != role || current.record.generation != 1 || current.state.process != (bbt_proto::ProcessStatus::Running { pid: info.pid }) {
                                        return Err(invalid("role metadata prelude was absent, stale, late or sent to ctl"));
                                    }
                                    snapshot = true;
                                },
                                GuiReply::SeatControlled { op } if op == id => {
                                    if (door == crate::mcp::Door::Gui) != snapshot { return Err(invalid("role receipt preceded GUI metadata, or ctl gained a prelude")); }
                                    receipt = true;
                                },
                                GuiReply::Ok { op } if op == next => {
                                    if !receipt { return Err(invalid("following operation overtook the role receipt")); }
                                    break;
                                },
                                other => return Err(invalid(format!("unexpected role boundary reply: {other:?}"))),
                            }
                        }
                        if logic.client.record(seat).await?.role.name != role { return Err(invalid("role prelude was not committed registry authority")); }
                    }
                    Ok(())
                }.await;
                drop(read); drop(write);
                let stopped = session.await.map_err(|_| invalid("role session interrupted"))?;
                checked?; stopped?;
            }
            Ok::<_, crate::store::Error>(())
        }).await;
        logic.shutdown().unwrap();
        supervisor.shutdown().unwrap();
        result.expect("role receipt hang guard").unwrap();
    }

    // keeps: spec e2e-acceptance M6 (a committed mute is visible without draining the event lane).
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ignore = "night: real supervisor, PTY and GUI socket boundary"]
    async fn mute_reply_refreshes_authoritative_seat_before_receipt_without_events() {
        use bbt_proto::messaging::{Request, Response};
        let root = tempfile::tempdir().unwrap();
        let repository = root.path().join("repository");
        std::fs::create_dir(&repository).unwrap();
        let run = root.path().join("run");
        let account = crate::identity::TestAccount::open(&root.path().join("keys"), &run).unwrap();
        let key_store = account.key_store();
        let supervisor = Supervisor::start(crate::Config {
            journal_directory: Some(root.path().join("supervisor-journal")),
            ..Default::default()
        })
        .unwrap();
        let logic = Logic::start_custodied_with_store(
            supervisor.connect().unwrap(),
            LogicConfig {
                run_dir: run,
                repository,
                ring: Default::default(),
            },
            key_store.clone(),
            &tokio::runtime::Handle::current(),
        )
        .unwrap();
        let device = logic.client.gui_device().await.unwrap().id;
        let device_public = key_store.load(device).unwrap().unwrap().verifying_key().to_bytes();
        let spec = LaunchSpec::new("/bin/cat", root.path());
        let seat = spec.seat;
        let swarm = Ulid::new();
        logic
            .client
            .spawn(SeatRecord::new(seat, swarm, "mute fixture", "cat"), spec)
            .await
            .unwrap();
        let result = tokio::time::timeout(Duration::from_secs(30), async {
            for door in [crate::mcp::Door::Gui, crate::mcp::Door::Ctl] {
                let capability = [74; 32];
                let (peer, server) = tokio::net::UnixStream::pair()?;
                let session = tokio::spawn(gui_session(
                    server,
                    logic.client.clone(),
                    capability,
                    door,
                ));
                let (read, mut write) = peer.into_split();
                let mut read = BufReader::new(read);
                let checked: Result<()> = async {
                    // Never attach events: a state backlog must not be a
                    // prerequisite for observing the operator's own write.
                    for muted in [true, false, false] {
                        let id = Ulid::new();
                        send(&mut write, &GuiRequest {
                            version: bbt_proto::gui::VERSION,
                            capability,
                            action: GuiAction::Messaging {
                                id,
                                swarm_id: bbt_proto::SwarmId::from_u128(swarm.0),
                                request: Request::Mute {seat, muted},
                            },
                        }).await?;
                        let mut snapshot = None;
                        loop {
                            let line = read_line(&mut read).await?
                                .ok_or_else(|| invalid("mute connection closed"))?;
                            match serde_json::from_slice::<GuiReply>(&line)? {
                                GuiReply::Identity(_) => {},
                                GuiReply::Seat(current) => {
                                    if current.record.id != seat || current.record.muted != muted {
                                        return Err(invalid("mute snapshot differs from the committed write"));
                                    }
                                    snapshot = Some(current);
                                },
                                GuiReply::Messaging {op, response: Response::Muted {..}} if op == id => {
                                    if (door == crate::mcp::Door::Gui) != snapshot.is_some() {
                                        return Err(invalid("GUI mute receipt preceded authoritative state, or ctl gained an unsolicited state reply"));
                                    }
                                    break;
                                },
                                other => return Err(invalid(format!("unexpected mute reply: {other:?}"))),
                            }
                        }
                        let retained = logic.client.gui_seat(seat).await?;
                        if retained.record.muted != muted || snapshot.as_ref().is_some_and(|s| s.record != retained.record) {
                            return Err(invalid("mute snapshot was not authoritative"));
                        }
                    }
                    Ok(())
                }.await;
                drop(read);
                drop(write);
                let stopped = session.await.map_err(|_| invalid("mute session interrupted"))?;
                checked?;
                stopped?;
            }
            // Read original retained envelopes, not the unsigned GUI projection.
            // File custody supplies the certified device key; the temporary broker
            // starts with a separate random fallback key.
            let mut after = 0;
            let mut muted_states = vec![];
            while muted_states.len() < 4 {
                let (rows, _, through) = logic.client.bus.read_page_wait(
                    &format!("seat-{seat}"), vec![format!("seat/{seat}/state")],
                    after, 128, Some(5000),
                ).await?;
                assert!(through > after, "mute journal publication did not progress");
                after = through;
                for row in rows.into_iter().filter(|row| row.event.source == "mute") {
                    row.verify()?;
                    assert_eq!(row.provenance.signer, device_public);
                    muted_states.push(row.event.payload["muted"].as_bool().unwrap());
                    let mut tampered = row.clone();
                    tampered.event.payload["muted"] = serde_json::json!("forged");
                    assert!(tampered.verify().is_err(), "journal signature must cover mute state");
                    let mut foreign = row;
                    foreign.provenance.signer = ed25519_dalek::SigningKey::from_bytes(&[92; 32]).verifying_key().to_bytes();
                    assert!(foreign.verify().is_err(), "another device must not verify this journal entry");
                }
            }
            assert_eq!(muted_states, [true, false, true, false]);
            Ok::<_, crate::store::Error>(())
        }).await;
        logic.shutdown().unwrap();
        supervisor.shutdown().unwrap();
        result.expect("mute response hang guard").unwrap();
    }

    // keeps: decision #328 (a parked page read cannot hold up the connection's controls)
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_waiting_page_does_not_block_the_next_control_request() {
        tokio::time::timeout(Duration::from_secs(30), async {
            let root = tempfile::tempdir().unwrap();
            let repository = root.path().join("repository");
            std::fs::create_dir(&repository).unwrap();
            let supervisor = Supervisor::start(crate::Config {
                journal_directory: Some(root.path().join("supervisor-journal")),
                ..Default::default()
            })
            .unwrap();
            let logic = Logic::start(
                supervisor.connect().unwrap(),
                LogicConfig {
                    run_dir: root.path().join("run"),
                    repository,
                    ring: Default::default(),
                },
                ed25519_dalek::SigningKey::from_bytes(&[92; 32]),
                &tokio::runtime::Handle::current(),
            )
            .unwrap();
            let capability = [74; 32];
            let (peer, server) = tokio::net::UnixStream::pair().unwrap();
            let session = tokio::spawn(gui_session(
                server,
                logic.client.clone(),
                capability,
                crate::mcp::Door::Ctl,
            ));
            let (read, mut write) = peer.into_split();
            let mut read = BufReader::new(read);
            let waiting = Ulid::new();
            for action in [
                GuiAction::EventsRead {
                    id: waiting,
                    stream: "index".into(),
                    after: i64::MAX as u64,
                    limit: 1,
                    wait_ms: Some(60_000),
                },
                GuiAction::ListSeats,
            ] {
                send(
                    &mut write,
                    &GuiRequest {
                        version: bbt_proto::gui::VERSION,
                        capability,
                        action,
                    },
                )
                .await
                .unwrap();
            }
            loop {
                let line = read_line(&mut read)
                    .await
                    .unwrap()
                    .expect("GUI closed before the control reply");
                match serde_json::from_slice::<GuiReply>(&line).unwrap() {
                    GuiReply::Identity(_) => {}
                    GuiReply::Seats(seats) => {
                        assert!(seats.is_empty());
                        break;
                    }
                    other => panic!("waiting read overtook the ready control: {other:?}"),
                }
            }
            drop(read);
            drop(write);
            session.await.unwrap().unwrap();
            logic.shutdown().unwrap();
            supervisor.shutdown().unwrap();
        })
        .await
        .expect("waiting read blocked the GUI control path");
    }
    // keeps: spec c1-live-seats
    #[tokio::test]
    async fn refusal_drains_the_complete_request_before_half_closing() {
        tokio::time::timeout(Duration::from_secs(10), async {
            // The request cannot fit before the server reads. The refusal can fit.
            let (client, server) = tokio::io::duplex(512);
            let (read, mut write) = tokio::io::split(server);
            let refused = tokio::spawn(async move {
                refuse_gui(
                    &mut BufReader::new(read),
                    &mut write,
                    crate::ctl_door::REFUSAL,
                )
                .await
            });
            let (read, mut write) = tokio::io::split(client);
            let mut request = vec![b' '; 4096];
            request.extend_from_slice(b"{}\n");
            write.write_all(&request).await.unwrap();
            let mut read = BufReader::new(read);
            let bytes = read_line(&mut read).await.unwrap().unwrap();
            let reply: GuiReply = serde_json::from_slice(&bytes).unwrap();
            assert!(matches!(reply, GuiReply::Error {op,reason}
                if op.is_nil() && reason == crate::ctl_door::REFUSAL));
            assert!(read_line(&mut read).await.unwrap().is_none());
            refused.await.unwrap().unwrap();
        })
        .await
        .expect("a complete bounded request must receive its refusal and EOF");
    }

    // keeps: spec c1-live-seats
    #[tokio::test]
    async fn busy_gui_poll_and_locked_cursor_keep_the_session_and_queued_events() {
        use bbt_proto::messaging::{Request, Response, SendRequest};
        use std::sync::atomic::Ordering;
        struct Keys;
        impl crate::identity::DeviceKeyStore for Keys {
            fn custody(&self) -> crate::identity::Custody {
                crate::identity::Custody::Memory
            }
            fn load(&self, _: Ulid) -> crate::identity::Result<Option<ed25519_dalek::SigningKey>> {
                Ok(Some(ed25519_dalek::SigningKey::from_bytes(&[91; 32])))
            }
            fn create(
                &self,
                _: Ulid,
                _: &ed25519_dalek::SigningKey,
            ) -> crate::identity::Result<()> {
                Err(crate::identity::PendingReason::JournalConflict)
            }
        }
        let root = tempfile::tempdir().unwrap();
        let repository = root.path().join("repository");
        std::fs::create_dir(&repository).unwrap();
        let run = root.path().join("run");
        assert!(matches!(
            crate::identity::local_dev_issuer_with_store(&run, &Keys),
            crate::identity::LocalIssuerOutcome::Ready(_)
        ));
        let supervisor = Supervisor::start(crate::Config {
            journal_directory: Some(root.path().join("supervisor-journal")),
            ..Default::default()
        })
        .unwrap();
        let logic = Logic::start(
            supervisor.connect().unwrap(),
            LogicConfig {
                run_dir: run,
                repository,
                ring: Default::default(),
            },
            ed25519_dalek::SigningKey::from_bytes(&[91; 32]),
            &tokio::runtime::Handle::current(),
        )
        .unwrap();
        let spec = LaunchSpec::new("/bin/cat", root.path());
        let seat = spec.seat;
        let swarm = Ulid::new();
        logic
            .client
            .spawn(
                SeatRecord::new(seat, swarm, "GUI transient fixture", "cat"),
                spec,
            )
            .await
            .unwrap();
        let endpoint = logic.client.gui_endpoint().await.unwrap();
        let stream = tokio::net::UnixStream::connect(&endpoint.path)
            .await
            .unwrap();
        let (read, mut write) = stream.into_split();
        let mut read = BufReader::new(read);
        // This is per-handle, one-shot injection before attachment can start polling.
        logic
            .client
            .bus
            .gui_read_faults
            .poll
            .store(rusqlite::ffi::SQLITE_BUSY_SNAPSHOT, Ordering::SeqCst);
        let attached = Ulid::new();
        send(
            &mut write,
            &GuiRequest {
                version: bbt_proto::gui::VERSION,
                capability: endpoint.capability,
                action: GuiAction::AttachEvents { id: attached },
            },
        )
        .await
        .unwrap();
        // Live replay begins at the attachment cut: publish after its acknowledgement.
        tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                let line = read_line(&mut read)
                    .await
                    .unwrap()
                    .expect("GUI closed before attachment");
                let reply: GuiReply = serde_json::from_slice(&line).unwrap();
                if matches!(reply,GuiReply::Ok {op} if op==attached) {
                    break;
                }
                assert!(
                    !matches!(reply, GuiReply::Error { .. }),
                    "attach refused: {reply:?}"
                );
            }
        })
        .await
        .unwrap();
        let published = logic
            .client
            .bus
            .publish(
                "index",
                crate::store::events::Draft::new(
                    "activity/transient-fixture",
                    "fixture",
                    serde_json::json!({"marker":"after-busy"}),
                ),
            )
            .await
            .unwrap();
        let result = tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                let line = read_line(&mut read)
                    .await
                    .unwrap()
                    .expect("GUI session closed on transient poll");
                let reply: GuiReply = serde_json::from_slice(&line).unwrap();
                assert!(
                    !matches!(reply, GuiReply::Error { .. }),
                    "event refused after busy: {reply:?}"
                );
                if let GuiReply::Event { stream, seq, .. } = reply {
                    send(
                        &mut write,
                        &GuiRequest {
                            version: bbt_proto::gui::VERSION,
                            capability: endpoint.capability,
                            action: GuiAction::EventAck {
                                stream: stream.clone(),
                                seq,
                            },
                        },
                    )
                    .await
                    .unwrap();
                    if stream == "index" && seq == published.seq {
                        break;
                    }
                }
            }
            assert_eq!(
                logic.client.bus.gui_read_faults.poll.load(Ordering::SeqCst),
                0
            );
            logic
                .client
                .bus
                .gui_read_faults
                .cursor
                .store(rusqlite::ffi::SQLITE_LOCKED, Ordering::SeqCst);
            let message = SendRequest::text("operator/inbox", "after-locked-cursor");
            let response = logic
                .client
                .gui_messaging(
                    message.id,
                    swarm,
                    Request::Send { message },
                    crate::mcp::Origin::default(),
                )
                .await
                .unwrap();
            let Response::Stored { message } = response else {
                panic!("message was not stored");
            };
            let message_id = message
                .envelope()
                .expect("ready-custody fixture must produce a signed envelope")
                .id;
            loop {
                let line = read_line(&mut read)
                    .await
                    .unwrap()
                    .expect("GUI session closed on transient cursor");
                let reply: GuiReply = serde_json::from_slice(&line).unwrap();
                assert!(
                    !matches!(reply, GuiReply::Error { .. }),
                    "event refused after locked: {reply:?}"
                );
                if let GuiReply::Event { stream, seq, envelope, .. } = reply {
                    send(
                        &mut write,
                        &GuiRequest {
                            version: bbt_proto::gui::VERSION,
                            capability: endpoint.capability,
                            action: GuiAction::EventAck {
                                stream: stream.clone(),
                                seq,
                            },
                        },
                    )
                    .await
                    .unwrap();
                    if matches!(&envelope.payload,bbt_proto::events::GuiEventPayload::Message {row} if row.id==message_id) {
                        break;
                    }
                }
            }
            assert_eq!(
                logic
                    .client
                    .bus
                    .gui_read_faults
                    .cursor
                    .load(Ordering::SeqCst),
                0
            );
            send(
                &mut write,
                &GuiRequest {
                    version: bbt_proto::gui::VERSION,
                    capability: endpoint.capability,
                    action: GuiAction::ListSeats,
                },
            )
            .await
            .unwrap();
            loop {
                let line = read_line(&mut read)
                    .await
                    .unwrap()
                    .expect("original GUI session did not survive");
                if let GuiReply::Seats(seats) = serde_json::from_slice::<GuiReply>(&line).unwrap() {
                    assert!(seats.iter().any(|s| s.record.id == seat));
                    break;
                }
            }
            logic
                .client
                .bus
                .gui_read_faults
                .poll
                .store(rusqlite::ffi::SQLITE_CORRUPT, Ordering::SeqCst);
            loop {
                let line = read_line(&mut read)
                    .await
                    .unwrap()
                    .expect("persistent error was not reported before close");
                if let GuiReply::EventStatus {
                    attached: false,
                    reason: Some(reason),
                } = serde_json::from_slice::<GuiReply>(&line).unwrap()
                {
                    assert!(
                        reason.starts_with("GuiStorePersistent: operation=gui_poll"),
                        "{reason}"
                    );
                    assert!(reason.contains("Some(11)"), "{reason}");
                    break;
                }
            }
            while let Some(line) = read_line(&mut read).await.unwrap() {
                assert!(
                    !matches!(
                        serde_json::from_slice::<GuiReply>(&line).unwrap(),
                        GuiReply::Seats(_)
                    ),
                    "persistent store failure kept serving the session"
                );
            }
        })
        .await;
        drop(write);
        drop(read);
        logic.shutdown().unwrap();
        supervisor.shutdown().unwrap();
        result.expect("GUI event recovery watchdog elapsed");
    }

    // keeps: spec c1-live-seats
    #[test]
    fn seat_listener_recovery_reclaims_only_a_dead_owned_socket() {
        use std::os::unix::{fs::symlink, net::UnixListener};

        let root = tempfile::tempdir().unwrap();
        std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        let path = root.path().join("seat.sock");
        private_parent(&path).unwrap();
        let listener = UnixListener::bind(&path).unwrap();
        let inode = std::fs::symlink_metadata(&path).unwrap().ino();
        assert!(reclaim_stale_seat_socket(&path).is_err());
        assert_eq!(std::fs::symlink_metadata(&path).unwrap().ino(), inode);
        drop(listener);
        assert!(path.exists(), "crash fixture must leave the socket inode");
        // Another test can fork while this listener is open. CLOEXEC closes
        // the child's copy at exec, not at the parent's drop above.
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            match probe_seat_socket(&path) {
                Err(error) if error.kind() == std::io::ErrorKind::ConnectionRefused => break,
                observed => {
                    assert!(
                        Instant::now() < deadline,
                        "listener did not refuse after drop: {observed:?}"
                    );
                    std::thread::yield_now();
                }
            }
        }
        reclaim_stale_seat_socket(&path).unwrap();
        assert!(!path.exists());

        std::fs::write(&path, b"unrelated file").unwrap();
        assert!(reclaim_stale_seat_socket(&path).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"unrelated file");
        let link = root.path().join("link.sock");
        symlink(&path, &link).unwrap();
        assert!(reclaim_stale_seat_socket(&link).is_err());
        assert!(
            std::fs::symlink_metadata(link)
                .unwrap()
                .file_type()
                .is_symlink()
        );
    }

    // keeps: spec c1-live-seats
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

        let error = GuiStoreReads::default()
            .read::<()>(
                GuiStoreOperation::Poll,
                Err(rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CORRUPT),
                    Some("private-payload-value".into()),
                )
                .into()),
            )
            .unwrap_err();
        let reason = gui_session_error_reason(&error);
        assert!(reason.starts_with("GuiStorePersistent: operation=gui_poll"));
        assert!(reason.contains("Some(11)"));
        assert!(!reason.contains("private-payload-value"));

        let json: crate::store::Error =
            serde_json::from_str::<GuiReply>(r#"{"kind":"private-payload-value"}"#)
                .unwrap_err()
                .into();
        let reason = gui_session_error_reason(&json);
        assert!(reason.starts_with("invalid GUI frame"));
        assert!(!reason.contains("private-payload-value"));
    }

    // keeps: spec c1-live-seats §2, bounded watched work and exact Applied gating.
    #[test]
    fn repeated_ticks_keep_one_check_per_seat_and_hot_seats_stay_at_the_back() {
        let mut checks = GuiChecks::default();
        let seats: Vec<_> = (1..=1024).map(bbt_proto::SeatId::from_u128).collect();
        for _ in 0..10 {
            for &seat in &seats {
                checks.schedule(seat);
            }
        }
        let first = checks.next().unwrap();
        assert_eq!(first, seats[0]);
        checks.schedule(first);
        for &seat in &seats[1..] {
            assert_eq!(checks.next(), Some(seat));
        }
        assert_eq!(checks.next(), Some(first));
        assert!(checks.is_empty());
    }

    // keeps: spec c1-live-seats §2, bounded watched work and exact Applied gating.
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

    // keeps: spec c1-live-seats §2, bounded watched work and exact Applied gating.
    #[test]
    fn duplicate_baseline_ack_preserves_the_newer_in_flight_update() {
        let (_, view, mut watched) = offered_view();
        let crate::streams::Update::Snapshot { snapshot } = watched.next_update().unwrap() else {
            panic!("first update must be a snapshot");
        };
        watched.applied(view, 1).unwrap();
        let sequence = |update| match update {
            crate::streams::Update::Snapshot { snapshot } => snapshot.sequence,
            crate::streams::Update::Delta { delta } => delta.sequence,
        };
        let mut newer = *snapshot;
        newer.sequence = 2;
        watched.stream.offer(newer.clone()).unwrap();
        assert_eq!(sequence(watched.next_update().unwrap()), 2);
        watched.applied(view, 1).unwrap();
        assert!(watched.next_update().is_none());
        assert!(watched.applied(view, 3).is_err());
        newer.sequence = 3;
        watched.stream.offer(newer).unwrap();
        assert!(watched.next_update().is_none());
        watched.applied(view, 2).unwrap();
        assert_eq!(sequence(watched.next_update().unwrap()), 3);
        watched.applied(view, 3).unwrap();
        assert!(watched.next_update().is_none());
    }

    // keeps: spec c1-live-seats §2, no fallback RPC before Applied.
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
        assert_eq!(
            error.downcast_ref::<std::io::Error>().unwrap().kind(),
            std::io::ErrorKind::InvalidData
        );
        assert!(watched.next_update().is_some());
        for _ in 0..100 {
            gui_fallback_check(&client, &mut write, seat, &mut watched)
                .await
                .unwrap();
        }
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
    // keeps: spec c1-live-seats
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

#[cfg(test)]
mod knowledge_budget_tests {
    use super::*;
    #[test]
    fn workspace_connection_budget_caps_and_releases_every_scope_key() {
        let mut budget = GuiKnowledgeBudget::default();
        let first = Some((Ulid(1), 1));
        let next_revision = Some((Ulid(1), 2));
        assert!(budget.admit(first));
        assert!(budget.admit(first));
        assert!(!budget.admit(first));
        assert!(budget.admit(next_revision));
        assert!(budget.admit(None));
        budget.release(first).unwrap();
        assert!(budget.admit(first));
        budget.release(first).unwrap();
        budget.release(first).unwrap();
        budget.release(next_revision).unwrap();
        budget.release(None).unwrap();
        assert_eq!(budget.active, 0);
        assert!(budget.scopes.is_empty());
        for id in 1..=32 {
            assert!(budget.admit(Some((Ulid(id), 0))));
        }
        assert!(!budget.admit(None));
        assert_eq!(budget.active, 32);
        for id in 1..=32 {
            budget.release(Some((Ulid(id), 0))).unwrap();
        }
        assert!(budget.scopes.is_empty());
        assert!(budget.admit(None));
        budget.release(None).unwrap();
        assert_eq!(budget.active, 0);
    }
}
