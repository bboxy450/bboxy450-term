//! Bounded, versioned JSON frames: u32 big-endian length, then UTF-8 JSON.
//!
//! These are private supervisor control frames, not the later public seat bus.

use std::collections::VecDeque;
use std::fs::{self, DirBuilder, File, OpenOptions};
use std::io::{self, Read, Write};
use std::marker::PhantomData;
use std::os::fd::AsRawFd;
use std::os::unix::fs::{DirBuilderExt, FileTypeExt, MetadataExt, OpenOptionsExt, PermissionsExt};
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};

use mio::net::{UnixListener as MioListener, UnixStream};
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::protocol::{Frame, Message, Request, VERSION};

pub const MAX_FRAME_BYTES: usize = 1024 * 1024;
pub(crate) const MAX_SOCKET_QUEUE_BYTES: usize = 4 * MAX_FRAME_BYTES;

fn invalid(message: impl std::fmt::Display) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.to_string())
}

pub fn encode<T: Serialize>(frame: &Frame<T>) -> io::Result<Vec<u8>> {
    if frame.version != VERSION {
        return Err(invalid("unsupported supervisor protocol version"));
    }
    let body = serde_json::to_vec(frame).map_err(invalid)?;
    if body.is_empty() || body.len() > MAX_FRAME_BYTES {
        return Err(invalid("supervisor frame exceeds 1 MiB"));
    }
    let mut bytes = Vec::with_capacity(4 + body.len());
    bytes.extend_from_slice(&(body.len() as u32).to_be_bytes());
    bytes.extend_from_slice(&body);
    Ok(bytes)
}

fn decode_body<T: DeserializeOwned>(body: &[u8]) -> io::Result<Frame<T>> {
    let frame: Frame<T> = serde_json::from_slice(body).map_err(invalid)?;
    if frame.version != VERSION {
        return Err(invalid("unsupported supervisor protocol version"));
    }
    Ok(frame)
}

fn checked_length(bytes: [u8; 4]) -> io::Result<usize> {
    let len = u32::from_be_bytes(bytes) as usize;
    if len == 0 || len > MAX_FRAME_BYTES {
        return Err(invalid("invalid supervisor frame length"));
    }
    Ok(len)
}

/// Blocking helper for a future split-process client. Length checked before allocation.
pub fn read_frame<T: DeserializeOwned>(reader: &mut impl Read) -> io::Result<Frame<T>> {
    let mut header = [0; 4];
    reader.read_exact(&mut header)?;
    let len = checked_length(header)?;
    let mut body = vec![0; len];
    reader.read_exact(&mut body)?;
    decode_body(&body)
}

pub fn write_frame<T: Serialize>(writer: &mut impl Write, frame: &Frame<T>) -> io::Result<()> {
    writer.write_all(&encode(frame)?)
}

/// Incremental decoder used by the nonblocking socket in the supervisor's poller.
pub struct Decoder<T> {
    buffer: Vec<u8>,
    body_len: Option<usize>,
    marker: PhantomData<T>,
}

impl<T> Default for Decoder<T> {
    fn default() -> Self {
        Self {
            buffer: Vec::new(),
            body_len: None,
            marker: PhantomData,
        }
    }
}

impl<T: DeserializeOwned> Decoder<T> {
    pub fn push(&mut self, mut bytes: &[u8]) -> io::Result<Vec<Frame<T>>> {
        let mut frames = Vec::new();
        while !bytes.is_empty() {
            let needed = self.body_len.unwrap_or(4) - self.buffer.len();
            let count = needed.min(bytes.len());
            self.buffer.extend_from_slice(&bytes[..count]);
            bytes = &bytes[count..];
            if self.buffer.len() != self.body_len.unwrap_or(4) {
                continue;
            }
            if self.body_len.is_none() {
                // Reject a forged length before reserving its payload.
                let len = checked_length(self.buffer[..4].try_into().unwrap())?;
                self.buffer.clear();
                self.body_len = Some(len);
            } else {
                frames.push(decode_body(&self.buffer)?);
                self.buffer.clear();
                self.body_len = None;
            }
        }
        Ok(frames)
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty() && self.body_len.is_none()
    }
}

/// Private per-user control endpoint. Nothing binds here merely by using the crate.
pub fn default_socket_path() -> io::Result<PathBuf> {
    #[cfg(target_os = "macos")]
    let base = std::env::var_os("HOME")
        .map(|home| PathBuf::from(home).join("Library/Application Support/bboxy450-term/run"));
    #[cfg(not(target_os = "macos"))]
    let base = std::env::var_os("XDG_RUNTIME_DIR")
        .map(|dir| PathBuf::from(dir).join("bboxy450-term"))
        .or_else(|| {
            std::env::var_os("HOME")
                .map(|home| PathBuf::from(home).join(".local/state/bboxy450-term/run"))
        });
    base.map(|dir| dir.join("mux.sock"))
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no private runtime directory"))
}

// Keep this inode forever: unlinking/recreating the lock could split owners
// across two independent flocks. Only closing the last descriptor releases it.
fn socket_ownership(path: &Path, uid: libc::uid_t) -> io::Result<File> {
    let mut lock_name = path.as_os_str().to_os_string();
    lock_name.push(".lock");
    let lock_path = PathBuf::from(lock_name);
    let previous = match fs::symlink_metadata(&lock_path) {
        Ok(metadata) => Some(metadata),
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => return Err(error),
    };
    if previous.is_none() {
        match fs::symlink_metadata(path) {
            Ok(_) => {
                return Err(io::Error::new(
                    io::ErrorKind::AddrInUse,
                    "ControlSocketOwnershipUnknown: existing endpoint has no ownership lock",
                ));
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
    }
    let validate = |metadata: &fs::Metadata| {
        if !metadata.is_file() || metadata.uid() != uid || metadata.mode() & 0o7777 != 0o600 {
            Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "ControlSocketLockInvalid: lock must be an owned regular file of mode 0600",
            ))
        } else {
            Ok(())
        }
    };
    if let Some(previous) = &previous {
        validate(previous)?;
    }
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK)
        .open(&lock_path)?;
    let metadata = file.metadata()?;
    validate(&metadata)?;
    if previous.as_ref().is_some_and(|previous| {
        (previous.dev(), previous.ino()) != (metadata.dev(), metadata.ino())
    }) {
        return Err(io::Error::new(
            io::ErrorKind::AddrInUse,
            "ControlSocketLockChanged: ownership lock changed while opening",
        ));
    }
    loop {
        // SAFETY: file owns this valid descriptor; flock borrows it without
        // taking ownership. LOCK_NB never waits for another owner to exit.
        if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0 {
            break;
        }
        let error = io::Error::last_os_error();
        match error.kind() {
            io::ErrorKind::Interrupted => continue,
            io::ErrorKind::WouldBlock => {
                return Err(io::Error::new(
                    io::ErrorKind::AddrInUse,
                    format!(
                        "another live supervisor owns the control socket: ownership lock {error:?}"
                    ),
                ));
            }
            _ => return Err(error),
        }
    }
    let current = fs::symlink_metadata(&lock_path)?;
    if (current.dev(), current.ino()) != (metadata.dev(), metadata.ino()) {
        return Err(io::Error::new(
            io::ErrorKind::AddrInUse,
            "ControlSocketLockChanged: ownership lock changed while locking",
        ));
    }
    Ok(file)
}

pub(crate) struct BoundSocket {
    pub listener: MioListener,
    path: PathBuf,
    dev: u64,
    ino: u64,
    _ownership: File,
}

impl BoundSocket {
    pub fn bind(path: &Path) -> io::Result<Self> {
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .ok_or_else(|| invalid("socket path must have a private parent directory"))?;
        DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(parent)?;
        let metadata = fs::symlink_metadata(parent)?;
        // SAFETY: geteuid has no arguments and no memory-safety preconditions.
        let uid = unsafe { libc::geteuid() };
        if !metadata.is_dir() || metadata.uid() != uid || metadata.mode() & 0o077 != 0 {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "runtime parent must be owned and mode 0700",
            ));
        }
        let ownership = socket_ownership(path, uid)?;
        let listener = match UnixListener::bind(path) {
            Ok(listener) => listener,
            Err(error) if error.kind() == io::ErrorKind::AddrInUse => {
                let previous = fs::symlink_metadata(path)?;
                if !previous.file_type().is_socket() || previous.uid() != uid {
                    return Err(io::Error::new(
                        io::ErrorKind::AddrInUse,
                        "control endpoint exists and is not an owned socket",
                    ));
                }
                // Exclusive ownership, rather than connect errno, authorizes
                // reclamation: a full live backlog can also return refused.
                let current = fs::symlink_metadata(path)?;
                if current.dev() != previous.dev() || current.ino() != previous.ino() {
                    return Err(io::Error::new(
                        io::ErrorKind::AddrInUse,
                        "control socket changed during stale-path check",
                    ));
                }
                fs::remove_file(path)?;
                UnixListener::bind(path)?
            }
            Err(error) => return Err(error),
        };
        let metadata = fs::symlink_metadata(path)?;
        let bound = Self {
            listener: MioListener::from_std(listener),
            path: path.to_owned(),
            dev: metadata.dev(),
            ino: metadata.ino(),
            _ownership: ownership,
        };
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
        // mio requires nonblocking descriptors, including those passed from_std.
        crate::supervisor::set_nonblocking(std::os::fd::AsRawFd::as_raw_fd(&bound.listener))?;
        Ok(bound)
    }
}

impl Drop for BoundSocket {
    fn drop(&mut self) {
        if let Ok(metadata) = fs::symlink_metadata(&self.path)
            && metadata.dev() == self.dev
            && metadata.ino() == self.ino
        {
            let _ = fs::remove_file(&self.path);
        }
    }
}

pub(crate) struct SocketPeer {
    pub stream: UnixStream,
    pub decoder: Decoder<Request>,
    queue: VecDeque<Vec<u8>>,
    offset: usize,
    queued_bytes: usize,
}

impl SocketPeer {
    pub fn new(stream: UnixStream) -> Self {
        Self {
            stream,
            decoder: Decoder::default(),
            queue: VecDeque::new(),
            offset: 0,
            queued_bytes: 0,
        }
    }

    pub fn enqueue(&mut self, message: Message) -> io::Result<bool> {
        let bytes = encode(&Frame::new(message))?;
        if self.queued_bytes + bytes.len() > MAX_SOCKET_QUEUE_BYTES {
            return Ok(false);
        }
        self.queued_bytes += bytes.len();
        self.queue.push_back(bytes);
        Ok(true)
    }

    pub fn wants_write(&self) -> bool {
        !self.queue.is_empty()
    }

    pub fn flush(&mut self) -> io::Result<()> {
        while let Some(front) = self.queue.front() {
            match self.stream.write(&front[self.offset..]) {
                Ok(0) => return Err(io::ErrorKind::WriteZero.into()),
                Ok(count) => {
                    self.offset += count;
                    self.queued_bytes -= count;
                    if self.offset == front.len() {
                        self.queue.pop_front();
                        self.offset = 0;
                    }
                }
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => return Ok(()),
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(error),
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::Action;

    fn bind_orphan_fixture(path: &Path) -> BoundSocket {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        loop {
            match BoundSocket::bind(path) {
                Ok(bound) => return bound,
                Err(error)
                    if error.kind() == io::ErrorKind::AddrInUse
                        && error.to_string().contains("live supervisor") =>
                {
                    assert!(
                        std::time::Instant::now() < deadline,
                        "fixture still owned: {error:?}"
                    );
                    std::thread::yield_now();
                }
                Err(error) => panic!("bind orphan fixture: {error:?}"),
            }
        }
    }

    fn fixture_ownership(path: &Path) -> File {
        // SAFETY: geteuid has no arguments or memory-safety preconditions.
        socket_ownership(path, unsafe { libc::geteuid() }).unwrap()
    }

    #[test]
    fn refused_socket_rebinds_without_replacing_a_live_socket_or_file() {
        let root = tempfile::tempdir().unwrap();
        let run = root.path().join("run");
        DirBuilder::new().mode(0o700).create(&run).unwrap();
        let path = run.join("mux.sock");
        let ownership = fixture_ownership(&path);
        drop(UnixListener::bind(&path).unwrap());
        drop(ownership);
        // A pre-exec child can inherit the listener and its lifetime lock.
        let bound = bind_orphan_fixture(&path);
        let before = fs::symlink_metadata(&path).unwrap();
        let error = BoundSocket::bind(&path).err().unwrap();
        assert_eq!(error.kind(), io::ErrorKind::AddrInUse);
        assert!(error.to_string().contains("live supervisor"));
        let after = fs::symlink_metadata(&path).unwrap();
        assert_eq!((before.dev(), before.ino()), (after.dev(), after.ino()));
        assert!(std::os::unix::net::UnixStream::connect(&path).is_ok());
        drop(bound);
        assert!(!path.exists());

        fs::write(&path, b"owned fixture").unwrap();
        assert_eq!(
            BoundSocket::bind(&path).err().unwrap().kind(),
            io::ErrorKind::AddrInUse
        );
        assert_eq!(fs::read(&path).unwrap(), b"owned fixture");
    }

    #[test]
    fn ownership_lock_persists_and_legacy_without_one_fails_closed() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("run/mux.sock");
        let lock_path = root.path().join("run/mux.sock.lock");
        let bound = BoundSocket::bind(&path).unwrap();
        let before = fs::symlink_metadata(&lock_path).unwrap();
        assert!(before.is_file());
        assert_eq!(before.mode() & 0o7777, 0o600);
        // SAFETY: the BoundSocket owns this valid descriptor; read flags only.
        let flags = unsafe { libc::fcntl(bound._ownership.as_raw_fd(), libc::F_GETFD) };
        assert!(flags >= 0);
        assert_ne!(flags & libc::FD_CLOEXEC, 0);
        drop(bound);
        assert!(!path.exists());
        fs::write(&lock_path, b"persistent fixture").unwrap();
        let bound = bind_orphan_fixture(&path);
        let after = fs::symlink_metadata(&lock_path).unwrap();
        assert_eq!((before.dev(), before.ino()), (after.dev(), after.ino()));
        assert_eq!(fs::read(&lock_path).unwrap(), b"persistent fixture");
        drop(bound);

        let legacy = root.path().join("run/legacy.sock");
        let legacy_lock = root.path().join("run/legacy.sock.lock");
        let listener = UnixListener::bind(&legacy).unwrap();
        let before = fs::symlink_metadata(&legacy).unwrap();
        for _ in 0..2 {
            let error = BoundSocket::bind(&legacy).err().unwrap();
            assert_eq!(error.kind(), io::ErrorKind::AddrInUse);
            assert!(error.to_string().contains("ControlSocketOwnershipUnknown"));
            assert!(
                !legacy_lock.exists(),
                "refusal must not mint legacy authority"
            );
            let after = fs::symlink_metadata(&legacy).unwrap();
            assert_eq!((before.dev(), before.ino()), (after.dev(), after.ino()));
        }
        drop(listener);
        let error = BoundSocket::bind(&legacy).err().unwrap();
        assert!(error.to_string().contains("ControlSocketOwnershipUnknown"));
        assert!(!legacy_lock.exists());
    }

    #[test]
    fn invalid_lock_types_permissions_and_owner_are_rejected_without_changes() {
        use std::os::unix::fs::symlink;
        for kind in ["mode", "symlink", "directory", "socket", "uid"] {
            let root = tempfile::tempdir().unwrap();
            let run = root.path().join("run");
            DirBuilder::new().mode(0o700).create(&run).unwrap();
            let path = run.join("mux.sock");
            let lock_path = run.join("mux.sock.lock");
            let target = run.join("untouched");
            fs::write(&target, b"sentinel").unwrap();
            let mut listener = None;
            match kind {
                "symlink" => symlink(&target, &lock_path).unwrap(),
                "directory" => fs::create_dir(&lock_path).unwrap(),
                "socket" => listener = Some(UnixListener::bind(&lock_path).unwrap()),
                _ => {
                    fs::write(&lock_path, b"lock fixture").unwrap();
                    fs::set_permissions(
                        &lock_path,
                        fs::Permissions::from_mode(if kind == "uid" { 0o600 } else { 0o644 }),
                    )
                    .unwrap();
                }
            }
            let before = fs::symlink_metadata(&lock_path).unwrap();
            let error = if kind == "uid" {
                // Exercise the owner check without chown or privileged state.
                socket_ownership(&path, before.uid().wrapping_add(1))
                    .err()
                    .unwrap()
            } else {
                BoundSocket::bind(&path).err().unwrap()
            };
            assert_eq!(error.kind(), io::ErrorKind::PermissionDenied, "{kind}");
            assert!(
                error.to_string().contains("ControlSocketLockInvalid"),
                "{kind}: {error:?}"
            );
            let after = fs::symlink_metadata(&lock_path).unwrap();
            assert_eq!(
                (before.dev(), before.ino(), before.mode()),
                (after.dev(), after.ino(), after.mode())
            );
            assert!(!path.exists());
            assert_eq!(fs::read(&target).unwrap(), b"sentinel");
            drop(listener);
        }
    }

    #[test]
    fn full_listener_backlog_is_live_and_never_unlinked() {
        use std::os::fd::AsRawFd;
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("run/mux.sock");
        let bound = BoundSocket::bind(&path).unwrap();
        // SAFETY: the test owns this listening descriptor. A small backlog
        // makes the real nonblocking WouldBlock path deterministic.
        assert_eq!(unsafe { libc::listen(bound.listener.as_raw_fd(), 1) }, 0);
        let before = fs::symlink_metadata(&path).unwrap();
        let mut clients = Vec::new();
        let mut full = false;
        for _ in 0..32 {
            match UnixStream::connect(&path) {
                Ok(client) => clients.push(client),
                Err(error)
                    if matches!(
                        error.kind(),
                        io::ErrorKind::WouldBlock | io::ErrorKind::ConnectionRefused
                    ) =>
                {
                    eprintln!(
                        "saturated live listener: {} queued clients; connect error {error:?}",
                        clients.len()
                    );
                    full = true;
                    break;
                }
                Err(error) => panic!("fill backlog: {error:?}"),
            }
        }
        assert!(full, "test did not saturate its listener backlog");
        let probe = BoundSocket::bind(&path);
        let after = fs::symlink_metadata(&path).unwrap();
        let same_inode = (before.dev(), before.ino()) == (after.dev(), after.ino());
        let accepted = bound.listener.accept().is_ok();
        eprintln!(
            "full backlog: replacement_bound={} same_inode={same_inode} original_listener_accepts={accepted}",
            probe.is_ok()
        );
        assert!(accepted, "original listener must still be live");
        assert!(same_inode, "must not replace a saturated live listener");
        let error = probe.err().unwrap();
        assert_eq!(error.kind(), io::ErrorKind::AddrInUse);
        assert!(error.to_string().contains("live supervisor"));
    }

    #[test]
    fn fork_inherited_listener_is_live_until_the_child_releases_it() {
        use std::os::fd::AsRawFd;
        let root = tempfile::tempdir().unwrap();
        let run = root.path().join("run");
        DirBuilder::new().mode(0o700).create(&run).unwrap();
        let path = run.join("mux.sock");
        let ownership = fixture_ownership(&path);
        let listener = UnixListener::bind(&path).unwrap();
        let before = fs::symlink_metadata(&path).unwrap();
        // SAFETY: listener owns this valid descriptor; F_GETFD only reads flags.
        let flags = unsafe { libc::fcntl(listener.as_raw_fd(), libc::F_GETFD) };
        assert!(flags >= 0);
        assert_ne!(flags & libc::FD_CLOEXEC, 0);
        let (mut parent_gate, child_gate) = std::os::unix::net::UnixStream::pair().unwrap();
        let parent_fd = parent_gate.as_raw_fd();
        let child_fd = child_gate.as_raw_fd();
        // SAFETY: the child only closes/reads owned descriptors and _exits;
        // it never allocates, locks, or runs Rust destructors after fork.
        let pid = unsafe { libc::fork() };
        assert!(pid >= 0, "fork: {}", io::Error::last_os_error());
        if pid == 0 {
            // SAFETY: both descriptors were opened before fork and are valid.
            unsafe {
                libc::close(parent_fd);
                let mut byte = 0u8;
                let count = libc::read(child_fd, (&mut byte as *mut u8).cast(), 1);
                libc::_exit(if count >= 0 { 0 } else { 2 });
            }
        }
        eprintln!("socket probe owned_fork_child_pid={pid}");
        drop(child_gate);
        drop(listener);
        drop(ownership);
        // The parent closed its listener, but the pre-exec child still owns
        // the same open file description. A successful connect is truly live.
        let probe = BoundSocket::bind(&path);
        let after = fs::symlink_metadata(&path);
        // Release and reap before asserting, even if the probe unexpectedly
        // succeeded. Closing the gate also releases the child on write error.
        let released = parent_gate.write_all(&[1]);
        drop(parent_gate);
        let mut status = 0;
        let reaped = loop {
            // SAFETY: pid is this test's fork child; status is writable.
            let result = unsafe { libc::waitpid(pid, &mut status, 0) };
            if result < 0 && io::Error::last_os_error().kind() == io::ErrorKind::Interrupted {
                continue;
            }
            break result;
        };
        released.unwrap();
        assert_eq!(reaped, pid);
        assert_eq!(status, 0);
        let error = probe.err().expect("must preserve inherited live listener");
        eprintln!("inherited listener probe: {error}");
        assert_eq!(error.kind(), io::ErrorKind::AddrInUse);
        let after = after.unwrap();
        assert_eq!((before.dev(), before.ino()), (after.dev(), after.ino()));
        let bound = bind_orphan_fixture(&path);
        drop(bound);
        assert!(!path.exists());
    }

    #[test]
    #[ignore = "manual 2000-iteration parallel stale/live socket stress"]
    fn stale_and_live_socket_paths_survive_2000_parallel_iterations() {
        let finished = std::sync::atomic::AtomicBool::new(false);
        let (failures, forks) = std::thread::scope(|scope| {
            let fork_load = scope.spawn(|| {
                let mut forks = 0;
                while !finished.load(std::sync::atomic::Ordering::Acquire) {
                    fork_inherited_listener_is_live_until_the_child_releases_it();
                    forks += 1;
                }
                forks
            });
            let workers: Vec<_> = (0..8)
                .map(|worker| {
                    scope.spawn(move || {
                        let mut failures = Vec::new();
                        for iteration in 0..250 {
                            if std::panic::catch_unwind(
                                refused_socket_rebinds_without_replacing_a_live_socket_or_file,
                            )
                            .is_err()
                            {
                                failures.push((worker, iteration));
                            }
                        }
                        failures
                    })
                })
                .collect();
            let failures = workers
                .into_iter()
                .flat_map(|worker| worker.join().unwrap())
                .collect::<Vec<_>>();
            finished.store(true, std::sync::atomic::Ordering::Release);
            (failures, fork_load.join().unwrap())
        });
        eprintln!(
            "socket stress: 2000 iterations, 8 workers, {forks} concurrent fork/exit cycles, {} failures",
            failures.len()
        );
        assert!(
            failures.is_empty(),
            "failed worker/iteration pairs: {failures:?}"
        );
    }

    #[test]
    fn fragmented_and_coalesced_frames_round_trip() {
        let frame = Frame::new(Request::new(None, Action::ListOwnedSeats));
        let bytes = encode(&frame).unwrap();
        let mut decoder = Decoder::<Request>::default();
        let mut result = Vec::new();
        for byte in &bytes {
            result.extend(decoder.push(&[*byte]).unwrap());
        }
        assert_eq!(result, vec![frame.clone()]);
        assert!(decoder.is_empty());
        let joined = [bytes.as_slice(), bytes.as_slice()].concat();
        assert_eq!(decoder.push(&joined).unwrap(), vec![frame.clone(), frame]);
    }

    #[test]
    fn rejects_lengths_versions_and_truncated_payloads() {
        for len in [0, MAX_FRAME_BYTES as u32 + 1, u32::MAX] {
            let mut decoder = Decoder::<Request>::default();
            assert!(decoder.push(&len.to_be_bytes()).is_err());
            assert!(read_frame::<Request>(&mut len.to_be_bytes().as_slice()).is_err());
        }
        let mut frame = Frame::new(Request::new(None, Action::ListOwnedSeats));
        frame.version += 1;
        assert!(encode(&frame).is_err());
        let body = serde_json::to_vec(&frame).unwrap();
        let raw = [(body.len() as u32).to_be_bytes().as_slice(), &body].concat();
        assert!(read_frame::<Request>(&mut raw.as_slice()).is_err());
        let bytes = encode(&Frame::new(Request::new(None, Action::ListOwnedSeats))).unwrap();
        assert!(read_frame::<Request>(&mut &bytes[..bytes.len() - 1]).is_err());
    }
}
