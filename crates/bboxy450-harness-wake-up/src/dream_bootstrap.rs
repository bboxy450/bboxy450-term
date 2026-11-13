// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Narrow producer-owned dream-bootstrap/1 mirror and exact parent pipe custody.
//! Adapted from bboxy450-harness src/dream/bootstrap.rs at signed bab543e2116d.
//! https://github.com/bboxy450/bboxy450-harness/blob/main/src/dream/bootstrap.rs
//! Producer tests/fixtures/dream-bootstrap is the sole byte/vector authority.
#![allow(missing_docs)]
use crate::dream_c1::wire::{self, Code};
use std::{
    fs::File,
    os::{
        fd::{AsRawFd, OwnedFd, RawFd},
        unix::fs::{FileTypeExt, MetadataExt},
    },
    time::{Duration, Instant},
};

pub const PROFILE: &str = "dream-bootstrap/1";
pub const ENV: &str = "BBOXY_DREAM_BOOTSTRAP";
pub const LENGTH: usize = 128;
pub const BOOTSTRAP_CONTROL_BYTES: usize = 1024;
/// Spawn-to-Hello readiness. A debug child on a loaded machine misses 250 ms;
/// the 30 s job deadline bounds wall time (as in d1794292861627496).
pub const WINDOW: Duration = Duration::from_secs(2);
const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    Format,
    Identity,
    Clock,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Purpose {
    InspectC1 = 1,
    ReadPrepare = 2,
}
/// Parent ordinary-work binding. Field order is the producer's sorted canonical
/// JSON order, including the borrowed typed Context; no intermediate JSON tree
/// or context clone is required. This data grants no execution authority.
#[derive(serde::Serialize)]
pub struct Binding<'a> {
    context: Option<CanonicalContext<'a>>,
    incarnation: u64,
    nonce: &'a str,
    op_id: &'a str,
    probe: bool,
    profile: &'static str,
    purpose: &'static str,
    run_id: &'a str,
    worker_sha256: &'a str,
}
#[derive(serde::Serialize)]
struct CanonicalContext<'a> {
    account: &'a str,
    account_epoch: u64,
    generation: u64,
    grant_epoch: u64,
    provider: &'a str,
    provider_epoch: u64,
    root: &'a str,
    root_revision: u64,
    seat: &'a str,
    swarm: &'a str,
    swarm_revision: u64,
    workspace: &'a str,
    workspace_revision: u64,
}
impl<'a> Binding<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        purpose: Purpose,
        probe: bool,
        op_id: &'a wire::Ulid,
        run_id: &'a wire::Ulid,
        nonce: &'a wire::Ulid,
        incarnation: u64,
        context: Option<&'a wire::Context>,
        worker_sha256: &'a wire::Hex,
    ) -> Result<Self, Error> {
        if incarnation == 0 || probe != context.is_none() {
            return Err(Error::Identity);
        }
        let context = context.map(|c| CanonicalContext {
            account: &c.account.0,
            account_epoch: c.account_epoch.0,
            generation: c.generation.0,
            grant_epoch: c.grant_epoch.0,
            provider: &c.provider.0,
            provider_epoch: c.provider_epoch.0,
            root: &c.root.0,
            root_revision: c.root_revision.0,
            seat: &c.seat.0,
            swarm: &c.swarm.0,
            swarm_revision: c.swarm_revision.0,
            workspace: &c.workspace.0,
            workspace_revision: c.workspace_revision.0,
        });
        Ok(Self {
            context,
            incarnation,
            nonce: &nonce.0,
            op_id: &op_id.0,
            probe,
            profile: PROFILE,
            purpose: match purpose {
                Purpose::InspectC1 => "inspect-c1",
                Purpose::ReadPrepare => "read-prepare",
            },
            run_id: &run_id.0,
            worker_sha256: &worker_sha256.0,
        })
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Token {
    pub fd: RawFd,
    pub birth: [u8; 16],
    pub binding: [u8; 32],
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Permit {
    pub purpose: Purpose,
    pub probe: bool,
    pub birth: [u8; 16],
    pub run_id: [u8; 16],
    pub nonce: [u8; 16],
    pub binding: [u8; 32],
    pub incarnation: u64,
    pub sequence: u64,
    pub deadline_ns: u64,
}
pub fn ulid(bytes: &[u8]) -> Result<[u8; 16], Error> {
    if bytes.len() != 26 {
        return Err(Error::Format);
    }
    let mut n = 0u128;
    for c in bytes {
        let v = ALPHABET.iter().position(|a| a == c).ok_or(Error::Format)? as u128;
        n = n
            .checked_mul(32)
            .and_then(|n| n.checked_add(v))
            .ok_or(Error::Format)?;
    }
    Ok(n.to_be_bytes())
}
pub fn binding(bytes: &[u8]) -> Result<[u8; 32], Error> {
    if bytes.len() != 64 {
        return Err(Error::Format);
    }
    let digit = |b| match b {
        b'0'..=b'9' => Ok(b - b'0'),
        b'a'..=b'f' => Ok(b - b'a' + 10),
        _ => Err(Error::Format),
    };
    let mut out = [0; 32];
    for (o, pair) in out.iter_mut().zip(bytes.as_chunks::<2>().0) {
        *o = digit(pair[0])? * 16 + digit(pair[1])?;
    }
    Ok(out)
}
pub fn token(bytes: &[u8]) -> Result<Token, Error> {
    if bytes.len() > 128 || !bytes.is_ascii() {
        return Err(Error::Format);
    }
    let mut fields = bytes.split(|c| *c == b':');
    if fields.next() != Some(b"1".as_slice()) {
        return Err(Error::Format);
    }
    let fd = fields.next().ok_or(Error::Format)?;
    if fd.is_empty() {
        return Err(Error::Format);
    }
    let mut n = 0u32;
    for c in fd {
        if !c.is_ascii_digit() {
            return Err(Error::Format);
        }
        n = n
            .checked_mul(10)
            .and_then(|n| n.checked_add(u32::from(c - b'0')))
            .ok_or(Error::Format)?;
        if n > 65535 {
            return Err(Error::Format);
        }
    }
    if !(3..=65535).contains(&n) {
        return Err(Error::Format);
    }
    let birth = ulid(fields.next().ok_or(Error::Format)?)?;
    let binding = binding(fields.next().ok_or(Error::Format)?)?;
    if fields.next().is_some() {
        return Err(Error::Format);
    }
    Ok(Token {
        fd: n as RawFd,
        birth,
        binding,
    })
}
pub fn decode(bytes: &[u8]) -> Result<Permit, Error> {
    if bytes.len() != LENGTH
        || &bytes[..8] != b"BBDBOOT1"
        || bytes[9] > 1
        || bytes[10..16].iter().chain(&bytes[120..]).any(|b| *b != 0)
    {
        return Err(Error::Format);
    }
    let purpose = match bytes[8] {
        1 => Purpose::InspectC1,
        2 => Purpose::ReadPrepare,
        _ => return Err(Error::Format),
    };
    let number = |at| u64::from_le_bytes(bytes[at..at + 8].try_into().unwrap());
    let p = Permit {
        purpose,
        probe: bytes[9] == 1,
        birth: bytes[16..32].try_into().unwrap(),
        run_id: bytes[32..48].try_into().unwrap(),
        nonce: bytes[48..64].try_into().unwrap(),
        binding: bytes[64..96].try_into().unwrap(),
        incarnation: number(96),
        sequence: number(104),
        deadline_ns: number(112),
    };
    if p.incarnation == 0 || p.sequence != 1 || p.deadline_ns == 0 {
        return Err(Error::Format);
    }
    Ok(p)
}
pub fn encode(p: &Permit) -> [u8; LENGTH] {
    let mut out = [0; LENGTH];
    out[..8].copy_from_slice(b"BBDBOOT1");
    out[8] = p.purpose as u8;
    out[9] = u8::from(p.probe);
    out[16..32].copy_from_slice(&p.birth);
    out[32..48].copy_from_slice(&p.run_id);
    out[48..64].copy_from_slice(&p.nonce);
    out[64..96].copy_from_slice(&p.binding);
    out[96..104].copy_from_slice(&p.incarnation.to_le_bytes());
    out[104..112].copy_from_slice(&p.sequence.to_le_bytes());
    out[112..120].copy_from_slice(&p.deadline_ns.to_le_bytes());
    out
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Frozen {
    pub purpose: Purpose,
    pub probe: bool,
    pub birth: [u8; 16],
    pub run_id: [u8; 16],
    pub nonce: [u8; 16],
    pub binding: [u8; 32],
    pub incarnation: u64,
}
impl Frozen {
    pub fn inspect(
        probe: bool,
        birth: [u8; 16],
        run_id: [u8; 16],
        nonce: [u8; 16],
        binding: [u8; 32],
        incarnation: u64,
    ) -> Self {
        Self {
            purpose: Purpose::InspectC1,
            probe,
            birth,
            run_id,
            nonce,
            binding,
            incarnation,
        }
    }
    pub fn permit(&self, deadline_ns: u64) -> Result<Permit, Error> {
        if self.incarnation == 0 || deadline_ns == 0 {
            return Err(Error::Format);
        }
        Ok(Permit {
            purpose: self.purpose,
            probe: self.probe,
            birth: self.birth,
            run_id: self.run_id,
            nonce: self.nonce,
            binding: self.binding,
            incarnation: self.incarnation,
            sequence: 1,
            deadline_ns,
        })
    }
    pub fn matches(&self, p: &Permit, t: &Token) -> bool {
        p.purpose == self.purpose
            && p.probe == self.probe
            && p.birth == self.birth
            && p.run_id == self.run_id
            && p.nonce == self.nonce
            && p.binding == self.binding
            && p.incarnation == self.incarnation
            && p.sequence == 1
            && p.deadline_ns != 0
            && t.birth == self.birth
            && t.binding == self.binding
    }
}
pub fn monotonic_ns() -> Result<u64, Error> {
    let mut t = std::mem::MaybeUninit::<libc::timespec>::uninit();
    // SAFETY: one valid stack timespec, no borrowed data or callbacks.
    if unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, t.as_mut_ptr()) } != 0 {
        return Err(Error::Clock);
    }
    let t = unsafe { t.assume_init() };
    let sec = u64::try_from(t.tv_sec).map_err(|_| Error::Clock)?;
    let ns = u64::try_from(t.tv_nsec).map_err(|_| Error::Clock)?;
    if ns >= 1_000_000_000 {
        return Err(Error::Clock);
    }
    sec.checked_mul(1_000_000_000)
        .and_then(|v| v.checked_add(ns))
        .ok_or(Error::Clock)
}

pub struct Environment {
    bytes: [u8; 128],
    len: usize,
}
impl Environment {
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.len]
    }
    pub fn as_str(&self) -> &str {
        std::str::from_utf8(self.as_bytes()).expect("constructed ASCII bootstrap environment")
    }
    fn push(&mut self, bytes: &[u8]) -> Result<(), Code> {
        let end = self
            .len
            .checked_add(bytes.len())
            .filter(|n| *n <= 128)
            .ok_or(Code::ResourceLimit)?;
        self.bytes[self.len..end].copy_from_slice(bytes);
        self.len = end;
        Ok(())
    }
}
fn environment(fd: RawFd, frozen: &Frozen) -> Result<Environment, Code> {
    let mut out = Environment {
        bytes: [0; 128],
        len: 0,
    };
    out.push(b"1:")?;
    let mut digits = [0u8; 5];
    let mut at = digits.len();
    let mut n = fd as u32;
    loop {
        at -= 1;
        digits[at] = b'0' + (n % 10) as u8;
        n /= 10;
        if n == 0 {
            break;
        }
    }
    out.push(&digits[at..])?;
    out.push(b":")?;
    let mut text = [0; 26];
    let mut n = u128::from_be_bytes(frozen.birth);
    for b in text.iter_mut().rev() {
        *b = ALPHABET[(n & 31) as usize];
        n >>= 5;
    }
    out.push(&text)?;
    out.push(b":")?;
    let mut hex = [0; 64];
    for (i, b) in frozen.binding.iter().enumerate() {
        hex[i * 2] = b"0123456789abcdef"[(b >> 4) as usize];
        hex[i * 2 + 1] = b"0123456789abcdef"[(b & 15) as usize];
    }
    out.push(&hex)?;
    Ok(out)
}
#[derive(Debug, Clone, Copy, Default)]
pub struct Observation {
    pub consumed: bool,
    pub write_attempted: bool,
    pub bytes_written: Option<u16>,
    pub errno: Option<i32>,
    pub granted: bool,
    pub revoked: bool,
    pub qualified_hello: bool,
    pub deadline_ns: Option<u64>,
}
/// Scheduling custody only. The caller must verify the packaged source/binary/
/// compiler/features bootstrap receipt BEFORE constructing/executing a command.
pub struct Gate {
    read: Option<File>,
    writer: Option<File>,
    frozen: Frozen,
    fd: RawFd,
    dev: u64,
    ino: u64,
    writer_dev: u64,
    writer_ino: u64,
    uid: u32,
    readiness: Option<Instant>,
    last_monotonic: u64,
    observation: Observation,
}
const _: () = assert!(
    std::mem::size_of::<Gate>() + std::mem::size_of::<Environment>() + LENGTH + 128
        <= BOOTSTRAP_CONTROL_BYTES
);
impl Gate {
    pub fn prepare(frozen: Frozen) -> Result<Self, Code> {
        crate::assert_not_ui_thread();
        if frozen.incarnation == 0 {
            return Err(Code::ProtocolError);
        }
        let (read, write) = std::io::pipe().map_err(|_| Code::RunnerUnavailable)?;
        let read = File::from(OwnedFd::from(read));
        let writer = File::from(OwnedFd::from(write));
        let fd = read.as_raw_fd();
        if !(3..=65535).contains(&fd) {
            return Err(Code::RunnerUnavailable);
        }
        let a = read.metadata().map_err(|_| Code::RunnerUnavailable)?;
        let b = writer.metadata().map_err(|_| Code::RunnerUnavailable)?;
        let read_flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
        let write_flags = unsafe { libc::fcntl(writer.as_raw_fd(), libc::F_GETFL) };
        if !a.file_type().is_fifo()
            || !b.file_type().is_fifo()
            || a.uid() != unsafe { libc::geteuid() }
            || b.uid() != unsafe { libc::geteuid() }
            || read_flags < 0
            || read_flags & libc::O_ACCMODE != libc::O_RDONLY
            || write_flags < 0
            || write_flags & libc::O_ACCMODE != libc::O_WRONLY
        {
            return Err(Code::RunnerUnavailable);
        }
        if unsafe {
            libc::fcntl(
                writer.as_raw_fd(),
                libc::F_SETFL,
                write_flags | libc::O_NONBLOCK,
            )
        } != 0
        {
            return Err(Code::RunnerUnavailable);
        }
        let last_monotonic = monotonic_ns().map_err(|_| Code::ResourceLimit)?;
        Ok(Self {
            read: Some(read),
            writer: Some(writer),
            frozen,
            fd,
            dev: a.dev(),
            ino: a.ino(),
            writer_dev: b.dev(),
            writer_ino: b.ino(),
            uid: a.uid(),
            readiness: None,
            last_monotonic,
            observation: Observation::default(),
        })
    }
    pub fn identity(&self) -> (u32, u64, u64) {
        (self.fd as u32, self.dev, self.ino)
    }
    pub fn writer_identity(&self) -> Option<(u32, u64, u64)> {
        self.writer
            .as_ref()
            .map(|file| (file.as_raw_fd() as u32, self.writer_dev, self.writer_ino))
    }
    pub fn inherited_fd(&self) -> RawFd {
        self.fd
    }
    pub fn frozen(&self) -> Frozen {
        self.frozen
    }
    pub fn env_value(&self) -> Result<Environment, Code> {
        if self.read.is_none()
            || self.writer.is_none()
            || self.readiness.is_some()
            || self.observation.consumed
            || self.observation.revoked
        {
            return Err(Code::RunnerUnavailable);
        }
        let read = self.read.as_ref().ok_or(Code::RunnerUnavailable)?;
        let metadata = read.metadata().map_err(|_| Code::RunnerUnavailable)?;
        let flags = unsafe { libc::fcntl(read.as_raw_fd(), libc::F_GETFL) };
        if read.as_raw_fd() != self.fd
            || !metadata.file_type().is_fifo()
            || (metadata.dev(), metadata.ino(), metadata.uid()) != (self.dev, self.ino, self.uid)
            || flags < 0
            || flags & libc::O_ACCMODE != libc::O_RDONLY
        {
            return Err(Code::RunnerUnavailable);
        }
        environment(self.fd, &self.frozen)
    }
    pub fn observation(&self) -> Observation {
        self.observation
    }
    pub fn spawn_returned(
        &mut self,
        returned_at: Instant,
        job_deadline: Instant,
    ) -> Result<(), Code> {
        if self.read.is_none() || self.readiness.is_some() {
            self.revoke();
            return Err(Code::ProtocolError);
        }
        self.read.take();
        let ready = returned_at
            .checked_add(WINDOW)
            .ok_or_else(|| {
                self.revoke();
                Code::ResourceLimit
            })?
            .min(job_deadline);
        self.readiness = Some(ready);
        if Instant::now() >= ready {
            self.revoke();
            return Err(Code::Deadline);
        }
        let now = monotonic_ns().map_err(|_| {
            self.revoke();
            Code::ResourceLimit
        })?;
        if now < self.last_monotonic {
            self.revoke();
            return Err(Code::ResourceLimit);
        }
        self.last_monotonic = now;
        Ok(())
    }
    pub fn grant(&mut self) -> Result<(), Code> {
        if self.observation.consumed {
            self.revoke();
            return Err(Code::ProtocolError);
        }
        self.observation.consumed = true;
        let result = (|| {
            let ready = self.readiness.ok_or(Code::ProtocolError)?;
            if self.observation.revoked || self.writer.is_none() {
                return Err(Code::Cancelled);
            }
            // Darwin assigns distinct inodes to the two endpoints. The OS pipe
            // constructor establishes their pair; each owned endpoint retains
            // its own original stamp rather than borrowing its peer's inode.
            let writer = self.writer.as_ref().ok_or(Code::Cancelled)?;
            let metadata = writer.metadata().map_err(|_| Code::RunnerUnavailable)?;
            let flags = unsafe { libc::fcntl(writer.as_raw_fd(), libc::F_GETFL) };
            if !metadata.file_type().is_fifo()
                || (metadata.dev(), metadata.ino(), metadata.uid())
                    != (self.writer_dev, self.writer_ino, self.uid)
                || flags < 0
                || flags & libc::O_ACCMODE != libc::O_WRONLY
                || flags & libc::O_NONBLOCK == 0
            {
                return Err(Code::RunnerUnavailable);
            }
            let now = monotonic_ns().map_err(|_| Code::ResourceLimit)?;
            if now < self.last_monotonic {
                return Err(Code::ResourceLimit);
            }
            self.last_monotonic = now;
            let left = ready
                .checked_duration_since(Instant::now())
                .ok_or(Code::Deadline)?;
            let left = u64::try_from(left.as_nanos()).map_err(|_| Code::ResourceLimit)?;
            let deadline = now.checked_add(left).ok_or(Code::ResourceLimit)?;
            let permit = self
                .frozen
                .permit(deadline)
                .map_err(|_| Code::ProtocolError)?;
            let bytes = encode(&permit);
            self.observation.deadline_ns = Some(deadline);
            self.observation.write_attempted = true;
            // Exactly one write. EINTR/short/error consumes this original attempt.
            let written = unsafe {
                libc::write(
                    self.writer.as_ref().unwrap().as_raw_fd(),
                    bytes.as_ptr().cast(),
                    LENGTH,
                )
            };
            if written >= 0 {
                self.observation.bytes_written = u16::try_from(written).ok();
            } else {
                self.observation.errno = std::io::Error::last_os_error().raw_os_error();
            }
            if written != LENGTH as isize {
                return Err(Code::WorkerLost);
            }
            self.observation.granted = true;
            Ok(())
        })();
        if result.is_err() {
            self.revoke();
        }
        result
    }
    pub fn readiness_deadline(&self) -> Result<Instant, Code> {
        self.readiness.ok_or(Code::RunnerUnavailable)
    }
    pub fn check_readiness(&self) -> Result<(), Code> {
        if self.observation.revoked || !self.observation.granted || self.writer.is_none() {
            Err(Code::RunnerUnavailable)
        } else if Instant::now() >= self.readiness_deadline()? {
            Err(Code::Deadline)
        } else {
            Ok(())
        }
    }
    pub fn qualify_hello(&mut self) -> Result<(), Code> {
        self.check_readiness()?;
        self.observation.qualified_hello = true;
        self.writer.take();
        Ok(())
    }
    pub fn revoke(&mut self) {
        if !self.observation.qualified_hello {
            self.observation.revoked = true;
        }
        self.writer.take();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    fn gate() -> Gate {
        // Ephemeral custody data only, not a canonical binding/byte fixture or
        // approved worker identity. Canonical format vectors live in producer.
        Gate::prepare(Frozen::inspect(
            false,
            ulid::Ulid::new().to_bytes(),
            ulid::Ulid::new().to_bytes(),
            ulid::Ulid::new().to_bytes(),
            [0; 32],
            1,
        ))
        .unwrap()
    }
    #[test]
    fn one_atomic_grant_consumes_attempt_and_second_grant_revokes_actual_pipe() {
        let mut gate = gate();
        let mut reader = gate.read.as_ref().unwrap().try_clone().unwrap();
        let (fd, dev, ino) = gate.identity();
        assert!((3..=65535).contains(&fd));
        let m = reader.metadata().unwrap();
        assert_eq!((m.dev(), m.ino()), (dev, ino));
        let (_, writer_dev, writer_ino) = gate.writer_identity().unwrap();
        let writer_metadata = gate.writer.as_ref().unwrap().metadata().unwrap();
        assert_eq!(
            (writer_metadata.dev(), writer_metadata.ino()),
            (writer_dev, writer_ino)
        );
        let env = gate.env_value().unwrap();
        assert!(env.as_bytes().len() <= 128);
        assert!(env.as_bytes().is_ascii());
        assert_eq!(token(env.as_bytes()).unwrap().fd, fd as i32);
        let returned = Instant::now();
        gate.spawn_returned(returned, returned + Duration::from_secs(30))
            .unwrap();
        gate.grant().unwrap();
        let mut bytes = [0; LENGTH];
        assert_eq!(reader.read(&mut bytes).unwrap(), LENGTH);
        let observed = gate.observation();
        assert!(observed.consumed && observed.write_attempted && observed.granted);
        assert_eq!(observed.bytes_written, Some(LENGTH as u16));
        assert_eq!(gate.grant(), Err(Code::ProtocolError));
        assert!(gate.observation().revoked);
        assert_eq!(reader.read(&mut [0; 1]).unwrap(), 0);
    }
    #[test]
    fn original_readiness_cannot_restart_and_expiry_closes_lease() {
        let mut gate = gate();
        let mut reader = gate.read.as_ref().unwrap().try_clone().unwrap();
        let returned = Instant::now() - WINDOW;
        assert_eq!(
            gate.spawn_returned(returned, Instant::now() + Duration::from_secs(30)),
            Err(Code::Deadline)
        );
        assert_eq!(
            gate.spawn_returned(Instant::now(), Instant::now() + Duration::from_secs(30)),
            Err(Code::ProtocolError)
        );
        assert_eq!(reader.read(&mut [0; 1]).unwrap(), 0);
        assert!(!gate.observation().write_attempted);
    }
    #[test]
    fn reversed_clock_or_prior_revoke_never_writes_or_reissues_permit() {
        for reversed in [false, true] {
            let mut gate = gate();
            let mut reader = gate.read.as_ref().unwrap().try_clone().unwrap();
            let now = Instant::now();
            gate.spawn_returned(now, now + Duration::from_secs(30))
                .unwrap();
            if reversed {
                gate.last_monotonic = u64::MAX;
            } else {
                gate.revoke();
            }
            assert_eq!(
                gate.grant(),
                Err(if reversed {
                    Code::ResourceLimit
                } else {
                    Code::Cancelled
                })
            );
            assert!(gate.observation().consumed && gate.observation().revoked);
            assert!(!gate.observation().write_attempted);
            assert_eq!(reader.read(&mut [0; 1]).unwrap(), 0);
            assert_eq!(gate.grant(), Err(Code::ProtocolError));
        }
    }
    #[test]
    fn replacing_either_endpoint_refuses_without_granting_a_foreign_pipe() {
        let mut replaced_read = gate();
        let (foreign_read, _foreign_write) = std::io::pipe().unwrap();
        replaced_read.read = Some(File::from(OwnedFd::from(foreign_read)));
        assert!(matches!(
            replaced_read.env_value(),
            Err(Code::RunnerUnavailable)
        ));

        let mut gate = gate();
        let mut original = gate.read.as_ref().unwrap().try_clone().unwrap();
        let now = Instant::now();
        gate.spawn_returned(now, now + Duration::from_secs(30))
            .unwrap();
        let (foreign_read, foreign_write) = std::io::pipe().unwrap();
        let mut foreign_read = File::from(OwnedFd::from(foreign_read));
        let foreign_write = File::from(OwnedFd::from(foreign_write));
        let flags = unsafe { libc::fcntl(foreign_write.as_raw_fd(), libc::F_GETFL) };
        assert!(flags >= 0);
        assert_eq!(
            unsafe {
                libc::fcntl(
                    foreign_write.as_raw_fd(),
                    libc::F_SETFL,
                    flags | libc::O_NONBLOCK,
                )
            },
            0
        );
        gate.writer = Some(foreign_write);
        assert_eq!(gate.grant(), Err(Code::RunnerUnavailable));
        assert!(gate.observation().consumed && gate.observation().revoked);
        assert!(!gate.observation().write_attempted);
        assert_eq!(original.read(&mut [0; 1]).unwrap(), 0);
        assert_eq!(foreign_read.read(&mut [0; 1]).unwrap(), 0);
    }
}
