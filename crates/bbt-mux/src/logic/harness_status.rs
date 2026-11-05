// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Legacy activity projection and an independently owned bounded C1 source follower.
use super::*;
use bboxy450_harness_wake_up::harness_log::{self, Log};

use crate::registry::dream_c1::{Authority as SourceAuthority, Bookmark, Journal};
use bbt_proto::dream::{Code, Context};
use std::collections::BTreeSet;

/// Closing a view asks this exact source owner to stop; a held read retains its
/// own thread/fd until it returns. No join or source IO runs in the async owner.
struct SourceFollower {
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl Drop for SourceFollower {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            thread.thread().unpark();
        }
    }
}
struct WatchedSource {
    reader: super::dream_c1::Reader,
    proof: Bookmark,
    current: bool,
    saved: u64,
}
fn proof(mut observed: Bookmark, original: &Bookmark) -> Bookmark {
    observed.pid = original.pid;
    observed.process_start = original.process_start.clone();
    observed.socket = original.socket.clone();
    observed.root = original.root.clone();
    observed.root_revision = original.root_revision;
    observed.workspace_revision = original.workspace_revision;
    observed.owner_context = original.owner_context.clone();
    observed.owner_fence = original.owner_fence.clone();
    observed
}
fn initial_proof(
    mut observed: Bookmark,
    binding: &Binding,
    socket: &std::path::Path,
    context: &Context,
) -> Bookmark {
    observed.pid = binding.pid;
    observed.process_start = binding.process_start.clone();
    observed.socket = socket.to_path_buf();
    observed.root = context.root.0.clone();
    observed.root_revision = context.root_revision.0;
    observed.workspace_revision = context.workspace_revision.0;
    observed.owner_context = Some(context.clone());
    observed
}
fn publish_activity(
    active: Option<bool>,
    captured: u64,
    current: bool,
    foreground: &AtomicBool,
    revision: &AtomicU64,
    latch: &AtomicU64,
) -> bool {
    let active = active.unwrap_or(true);
    if !active && !current {
        return false;
    }
    if captured & 1 == u64::from(active) {
        return false;
    }
    let Some(next) = (captured >> 1)
        .checked_add(1)
        .and_then(|v| v.checked_mul(2))
    else {
        return false;
    };
    if latch
        .compare_exchange(
            captured,
            next | u64::from(active),
            Ordering::AcqRel,
            Ordering::Acquire,
        )
        .is_ok()
    {
        // Compatibility projections never decide production work admission.
        foreground.store(active, Ordering::Release);
        revision.store((next | u64::from(active)) >> 1, Ordering::Release);
        true
    } else {
        false
    }
}

/// One source IO owner per process binding, with bounded retained-source rotation.
/// It never shares the legacy reader's potentially held syscall or blocking pool.
#[allow(clippy::too_many_arguments)]
fn follow_sources(
    handle: tokio::runtime::Handle,
    client: LogicClient,
    binding: Binding,
    root: PathBuf,
    socket: PathBuf,
    authority: SourceAuthority,
    runtime: PathBuf,
    foreground: Arc<AtomicBool>,
    revision: Arc<AtomicU64>,
    latch: Arc<AtomicU64>,
    stop: Arc<AtomicBool>,
) -> std::result::Result<(), Code> {
    super::dream_c1::utility()?;
    let (context, identity_fence) = authority.capture_all()?;
    let mut journal = Journal::open(&runtime, &context.seat)?;
    let mut watched = Vec::new();
    let mut known = BTreeSet::new();
    // A daemon-loss recovery is this exact retained index, never a home/corpus scan.
    for source in journal.retained_sources(&context)? {
        if source.owner_fence.as_ref() != Some(&identity_fence) {
            continue;
        }
        let selected = bbt_proto::dream::Selection {
            session: source.session.clone(),
            source_generation: bbt_proto::dream::Number(source.generation),
        };
        if journal
            .source_error(&context, &identity_fence, &selected)?
            .is_some()
        {
            continue;
        }
        known.insert((source.generation, source.session.0.clone()));
        if source.closed_seq > 0 {
            if stop.load(Ordering::Acquire) {
                return Ok(());
            }
            // Dedupe registration precedes any new source hash/copy/replay.
            let _ = handle.block_on(client.dream_closed(binding.seat, binding.generation, source));
        } else {
            match super::dream_c1::Reader::resume(&source) {
                Ok(reader) => watched.push(WatchedSource {
                    reader,
                    saved: source.cursor,
                    proof: source,
                    current: false,
                }),
                Err(code) => journal.observe_error(&context, &identity_fence, &selected, code)?,
            }
        }
    }
    let mut discover_at = Instant::now();
    let mut current_session = None;
    let mut peer_seen = false;
    let mut peer_lost = false;
    let mut current_closed = false;
    let mut notified_activity = 0;
    loop {
        if stop.load(Ordering::Acquire) {
            return Ok(());
        }
        // Every source unit retains the full actor/root/provider incarnation, including ABA.
        if !authority.matches(&context, &Some(identity_fence.clone())) {
            return Err(Code::StaleContext);
        }
        if Instant::now() >= discover_at {
            let discovered = bboxy450_harness_wake_up::native::verify_process(&binding)
                .and_then(|_| {
                    handle.block_on(harness_log::discover_socket(&socket, None, &binding))
                })
                .and_then(|session| {
                    bboxy450_harness_wake_up::native::verify_process(&binding)?;
                    Ok(session)
                });
            match discovered {
                Ok(session) => {
                    peer_seen = true;
                    peer_lost = false;
                    if current_session.as_ref() != Some(&session) {
                        current_closed = false;
                    }
                    current_session = Some(session.clone());
                    for source in &mut watched {
                        source.current = source.proof.generation == binding.generation
                            && source.proof.session.0 == session
                            && source.proof.pid == binding.pid
                            && source.proof.process_start == binding.process_start;
                    }
                    if !known.contains(&(binding.generation, session.clone())) && known.len() < 32 {
                        let selection = bbt_proto::dream::Selection {
                            session: bbt_proto::dream::Ulid(session.clone()),
                            source_generation: bbt_proto::dream::Number(binding.generation),
                        };
                        if journal
                            .source_error(&context, &identity_fence, &selection)?
                            .is_some()
                        {
                            known.insert((binding.generation, session));
                            continue;
                        }
                        let reader = match super::dream_c1::Reader::open(
                            &root,
                            &session,
                            binding.generation,
                        ) {
                            Ok(reader) => reader,
                            Err(code) => {
                                let selection = bbt_proto::dream::Selection {
                                    session: bbt_proto::dream::Ulid(session.clone()),
                                    source_generation: bbt_proto::dream::Number(binding.generation),
                                };
                                journal.observe_error(
                                    &context,
                                    &identity_fence,
                                    &selection,
                                    code,
                                )?;
                                known.insert((binding.generation, session));
                                continue;
                            }
                        };
                        let mut initial =
                            initial_proof(reader.bookmark()?, &binding, &socket, &context);
                        initial.owner_fence = Some(identity_fence.clone());
                        journal.observe_source(&context, &initial)?;
                        known.insert((binding.generation, session));
                        watched.push(WatchedSource {
                            reader,
                            saved: initial.cursor,
                            proof: initial,
                            current: true,
                        });
                    }
                }
                Err(_) => {
                    peer_lost = peer_seen;
                    for source in &mut watched {
                        source.current = false;
                    }
                    let captured = latch.load(Ordering::Acquire);
                    if !current_closed
                        && publish_activity(None, captured, false, &foreground, &revision, &latch)
                    {
                        let _ = handle
                            .block_on(client.dream_activity(binding.seat, binding.generation));
                    }
                }
            }
            discover_at =
                Instant::now() + Duration::from_millis(if peer_seen { 500 } else { 2000 });
        }
        let mut index = 0;
        let mut all_at_head = true;
        while index < watched.len() {
            if stop.load(Ordering::Acquire) {
                return Ok(());
            }
            let captured = latch.load(Ordering::Acquire);
            let result = watched[index].reader.poll();
            match result {
                Ok((closed, active)) => {
                    all_at_head &= watched[index].reader.at_head();
                    let observed = proof(watched[index].reader.bookmark()?, &watched[index].proof);
                    if observed.cursor != watched[index].saved || closed.is_some() {
                        journal.observe_source(&context, &observed)?;
                        watched[index].saved = observed.cursor;
                    }
                    let mut current = watched[index].current
                        && !peer_lost
                        && current_session.as_deref()
                            == Some(watched[index].proof.session.0.as_str());
                    // A fresh exact Hello after the read plus the packed CAS prevents an old
                    // A reader from clearing a newer B turn/session foreground latch.
                    if active == Some(false) && captured & 1 != 0 && current {
                        current = handle
                            .block_on(harness_log::hello(
                                &socket,
                                &watched[index].proof.session.0,
                                &binding,
                            ))
                            .is_ok();
                    }
                    if current
                        && publish_activity(active, captured, true, &foreground, &revision, &latch)
                    {
                        let _ = handle
                            .block_on(client.dream_activity(binding.seat, binding.generation));
                    }
                    if let Some(closed) = closed {
                        if !authority.matches(&context, &Some(identity_fence.clone())) {
                            return Err(Code::StaleContext);
                        }
                        let closed = proof(closed, &watched[index].proof);
                        if current_session.as_deref() == Some(closed.session.0.as_str())
                            && closed.generation == binding.generation
                        {
                            current_closed = true;
                        }
                        if stop.load(Ordering::Acquire) {
                            return Ok(());
                        }
                        let _ = handle.block_on(client.dream_closed(
                            binding.seat,
                            binding.generation,
                            closed,
                        ));
                        watched.remove(index);
                        continue;
                    }
                    // Peer loss/native EOF is not closure. Retain the bounded exact
                    // reader until closure or proven producer death, including partial LF.
                }
                Err(code) => {
                    let source = &watched[index].proof;
                    let selection = bbt_proto::dream::Selection {
                        session: source.session.clone(),
                        source_generation: bbt_proto::dream::Number(source.generation),
                    };
                    journal.observe_error(&context, &identity_fence, &selection, code)?;
                    if watched[index].current && !peer_lost {
                        let _ =
                            publish_activity(None, captured, false, &foreground, &revision, &latch);
                    }
                    watched.remove(index);
                    continue;
                }
            }
            index += 1;
        }
        let activity = latch.load(Ordering::Acquire);
        if activity & 1 == 0 && activity != notified_activity {
            // Rich admission may have already published quiescence. The follower
            // still emits its trusted refresh so retained proposals can resume.
            let _ = handle.block_on(client.dream_activity(binding.seat, binding.generation));
            notified_activity = activity;
        }
        if peer_lost && all_at_head {
            // SAFETY: signal zero queries only the exact captured PID. ESRCH proves
            // absence; timeout, EPERM and a failed ps/Hello alone never prove exit.
            if unsafe { libc::kill(binding.pid as i32, 0) } == -1
                && std::io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH)
            {
                return Ok(());
            }
        }
        if all_at_head {
            std::thread::park_timeout(Duration::from_millis(200));
        }
    }
}

/// A real blocking legacy-read witness, admitted only by the host's private
/// TestAccount fence. The FIFO and receipt are bounded, owned and never sources.
#[cfg(debug_assertions)]
fn hold_private_legacy_read(runtime: &Path, seat: crate::Ulid) -> std::io::Result<bool> {
    use std::io::{Read, Write};
    use std::os::fd::AsRawFd;
    use std::os::unix::fs::{FileTypeExt, MetadataExt, OpenOptionsExt};
    let path = runtime.join(format!("test-legacy-{seat}.fifo"));
    let expected = match std::fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error),
    };
    // SAFETY: geteuid has no arguments or side effects.
    if !expected.file_type().is_fifo()
        || expected.uid() != unsafe { libc::geteuid() }
        || expected.mode() & 0o777 != 0o600
        || expected.nlink() != 1
    {
        return Err(std::io::ErrorKind::PermissionDenied.into());
    }
    let mut input = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC)
        .open(&path)?;
    let writer = std::fs::OpenOptions::new()
        .write(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC)
        .open(&path)?;
    for file in [&input, &writer] {
        let opened = file.metadata()?;
        if (
            opened.dev(),
            opened.ino(),
            opened.uid(),
            opened.mode(),
            opened.nlink(),
        ) != (
            expected.dev(),
            expected.ino(),
            expected.uid(),
            expected.mode(),
            1,
        ) {
            return Err(std::io::ErrorKind::PermissionDenied.into());
        }
    }
    // SAFETY: this exact owned FIFO is isolated from the shared owner.
    let flags = unsafe { libc::fcntl(input.as_raw_fd(), libc::F_GETFL) };
    if flags < 0
        || unsafe { libc::fcntl(input.as_raw_fd(), libc::F_SETFL, flags & !libc::O_NONBLOCK) } != 0
    {
        return Err(std::io::Error::last_os_error());
    }
    let receipt = runtime.join(format!("test-legacy-{seat}.held.json"));
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(receipt)?;
    file.write_all(b"{\"owner\":\"legacy-log-read\",\"real_fifo_read\":true}")?;
    file.sync_all()?;
    let mut byte = [0u8; 1];
    input.read_exact(&mut byte)?;
    drop(writer);
    Ok(true)
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn run(
    client: LogicClient,
    binding: Binding,
    root: PathBuf,
    endpoint: Option<PathBuf>,
    dream_owned: bool,
    source_authority: Option<SourceAuthority>,
    foreground: Arc<AtomicBool>,
    activity_revision: Arc<AtomicU64>,
    activity_latch: Arc<AtomicU64>,
    runtime_path: PathBuf,
) {
    let handle = tokio::runtime::Handle::current();
    #[cfg(debug_assertions)]
    let legacy_fixture =
        dream_owned && source_authority.as_ref().is_some_and(|a| a.private_fixture);
    #[cfg(debug_assertions)]
    let legacy_runtime = runtime_path.clone();
    let mut legacy_held_once = false;
    let _source_follower = if dream_owned
        && let (Some(authority), Some(socket)) = (source_authority, endpoint.clone())
    {
        let stop = Arc::new(AtomicBool::new(false));
        let source_stop = stop.clone();
        let source_client = client.clone();
        let source_binding = binding.clone();
        let source_root = root.clone();
        let source_handle = handle.clone();
        // Only short owner creation uses the blocking executor; all subsequent source IO,
        // parsing, hashing and persistence belongs to the named utility thread.
        tokio::task::spawn_blocking(move || {
            std::thread::Builder::new()
                .name(format!("dream-c1-source-{}", source_binding.seat))
                .spawn(move || {
                    let result = follow_sources(
                        source_handle,
                        source_client,
                        source_binding,
                        source_root,
                        socket,
                        authority,
                        runtime_path,
                        foreground,
                        activity_revision,
                        activity_latch,
                        source_stop,
                    );
                    if let Err(code) = result {
                        eprintln!("Dream C1 source owner stopped: {code:?}");
                    }
                })
                .ok()
                .map(|thread| SourceFollower {
                    stop,
                    thread: Some(thread),
                })
        })
        .await
        .ok()
        .flatten()
    } else {
        None
    };
    let mut previous = binding.session.clone();
    loop {
        let session = match match &endpoint {
            Some(path) => harness_log::discover_socket(path, None, &binding).await,
            None => harness_log::discover(&root, &binding).await,
        } {
            Ok(session) => session,
            Err(_) => {
                tokio::time::sleep(Duration::from_secs(2)).await;
                continue;
            }
        };
        let log_root = root.clone();
        let log_session = session.clone();
        let Ok(Ok(mut log)) =
            tokio::task::spawn_blocking(move || Log::open(&log_root, &log_session)).await
        else {
            tokio::time::sleep(Duration::from_secs(2)).await;
            continue;
        };
        let socket = endpoint
            .clone()
            .unwrap_or_else(|| root.join("run").join(format!("{session}.sock")));
        let mut discovery_at = Instant::now() + Duration::from_secs(15);
        loop {
            #[cfg(debug_assertions)]
            let gate_runtime = legacy_runtime.clone();
            let Ok((returned, update, held)) = tokio::task::spawn_blocking(move || {
                let held = legacy_held_once;
                #[cfg(debug_assertions)]
                let held = if legacy_fixture && !legacy_held_once {
                    match hold_private_legacy_read(&gate_runtime, binding.seat) {
                        Ok(held) => held,
                        Err(error) => return (log, Err(error.into()), held),
                    }
                } else {
                    held
                };
                let update = log.poll();
                (log, update, held)
            })
            .await
            else {
                return;
            };
            log = returned;
            legacy_held_once = held;
            match update {
                Ok(Some(mut event)) => {
                    event["binding"] = serde_json::to_value(&binding).unwrap();
                    event["previous_session"] = serde_json::json!(previous);
                    if client
                        .native_event(binding.seat, binding.generation, event)
                        .await
                        .is_err()
                    {
                        return;
                    }
                    previous = session.clone();
                }
                Ok(None) => {}
                Err(_) => break,
            }
            if Instant::now() >= discovery_at {
                if harness_log::hello(&socket, &session, &binding)
                    .await
                    .is_err()
                {
                    break;
                }
                discovery_at = Instant::now() + Duration::from_secs(15);
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retained_or_stale_source_cannot_clear_new_current_foreground() {
        let foreground = AtomicBool::new(true);
        let revision = AtomicU64::new(2);
        let latch = AtomicU64::new(5); // current B2 foreground; old A1 captured 3
        assert!(!publish_activity(
            Some(false),
            3,
            true,
            &foreground,
            &revision,
            &latch
        ));
        assert_eq!(latch.load(Ordering::Acquire), 5);
        assert!(foreground.load(Ordering::Acquire));
        assert!(!publish_activity(
            Some(false),
            5,
            false,
            &foreground,
            &revision,
            &latch
        ));
        assert_eq!(
            latch.load(Ordering::Acquire),
            5,
            "historical source has no foreground authority"
        );
        assert!(publish_activity(
            Some(false),
            5,
            true,
            &foreground,
            &revision,
            &latch
        ));
        assert_eq!(latch.load(Ordering::Acquire), 6);
        assert!(!foreground.load(Ordering::Acquire));
        assert_eq!(revision.load(Ordering::Acquire), 3);
    }
}
