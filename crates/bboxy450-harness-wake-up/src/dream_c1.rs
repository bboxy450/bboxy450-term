// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Bounded C1 subprocess ownership; every method runs on its dedicated IO owner.
#![allow(missing_docs)]
pub mod wire {
    // Copyright 2026 Boxy (bboxy450) and contributors.
    // SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
    // Project: bboxy450-harness — https://github.com/bboxy450/bboxy450-harness
    // License: https://github.com/bboxy450/bboxy450-harness/blob/main/LICENSE
    // Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
    // Policy: https://github.com/bboxy450/bboxy450-harness/blob/main/ATTRIBUTION.md
    //! Narrow C1 worker mirror and bounded, independently owned subprocess transport.
    //! Adapted from bboxy450-harness src/dream/c1_wire.rs, boundary a3425ac0.
    //! https://github.com/bboxy450/bboxy450-harness/blob/main/src/dream/c1_wire.rs
    //! Producer owns schema/fixtures; preserve that origin with extracts.
    //! Consumer policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
    #![allow(missing_docs)]
    use serde::{Deserialize, Serialize};
    pub const MAX_FRAME: usize = 256 * 1024;
    pub const MAX_SAFE: u64 = (1u64 << 53) - 1;

    macro_rules! text_scalar {
        ($name:ident, $check:expr) => {
            #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
            #[serde(try_from = "String", into = "String")]
            pub struct $name(pub String);
            impl TryFrom<String> for $name {
                type Error = &'static str;
                fn try_from(v: String) -> Result<Self, Self::Error> {
                    if ($check)(&v) {
                        Ok(Self(v))
                    } else {
                        Err(stringify!($name))
                    }
                }
            }
            impl From<$name> for String {
                fn from(v: $name) -> Self {
                    v.0
                }
            }
        };
    }

    text_scalar!(Ulid, |s: &str| s.len() == 26
        && s.parse::<ulid::Ulid>().is_ok_and(|id| id.to_string() == s));
    text_scalar!(Hex, |s: &str| s.len() == 64
        && s.bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c)));
    text_scalar!(Id, |s: &str| !s.is_empty()
        && s.len() <= 256
        && !s.contains('\0'));
    text_scalar!(Repo, |s: &str| !s.is_empty()
        && s.len() <= 4096
        && !s.contains('\0'));
    text_scalar!(Mime, |s: &str| s.len() <= 256 && !s.contains('\0'));
    #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
    #[serde(try_from = "u64", into = "u64")]
    pub struct Number(pub u64);
    impl TryFrom<u64> for Number {
        type Error = &'static str;
        fn try_from(n: u64) -> Result<Self, Self::Error> {
            if n <= MAX_SAFE {
                Ok(Self(n))
            } else {
                Err("unsafe integer")
            }
        }
    }
    impl From<Number> for u64 {
        fn from(n: Number) -> u64 {
            n.0
        }
    }
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(try_from = "u64", into = "u64")]
    pub struct Version;
    impl TryFrom<u64> for Version {
        type Error = &'static str;
        fn try_from(n: u64) -> Result<Self, Self::Error> {
            if n == 1 { Ok(Self) } else { Err("version") }
        }
    }
    impl From<Version> for u64 {
        fn from(_: Version) -> u64 {
            1
        }
    }
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "snake_case")]
    pub enum Protocol {
        #[serde(rename = "bboxy.dream.c1/1")]
        V1,
    }
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "snake_case")]
    pub enum Algorithm {
        #[serde(rename = "d0-c1/1")]
        D0C1,
    }
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "snake_case")]
    pub enum ManifestSchema {
        #[serde(rename = "bboxy.dream.c1.manifest/1")]
        V1,
    }
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "snake_case")]
    pub enum ResultSchema {
        #[serde(rename = "bboxy.dream.c1.result/1")]
        V1,
    }
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "snake_case")]
    pub enum PageSchema {
        #[serde(rename = "bboxy.dream.c1.page/1")]
        V1,
    }
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "snake_case")]
    pub enum Trigger {
        Manual,
        SessionClosed,
    }
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "snake_case")]
    pub enum State {
        Deferred,
        Preparing,
        Running,
        Paused,
        Cancelling,
        Settling,
        Completed,
        Failed,
        Cancelled,
        Interrupted,
        Quarantined,
    }
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "snake_case")]
    pub enum Code {
        BadRequest,
        VersionMismatch,
        NotFound,
        StaleContext,
        OpIdReused,
        Busy,
        RunnerUnavailable,
        SourceUnavailable,
        SourceOpen,
        SourceChanged,
        MalformedIr,
        UnsupportedIr,
        SourceMismatch,
        BlobMismatch,
        ResourceLimit,
        StorageUnavailable,
        WorkerLost,
        ProtocolError,
        Cancelled,
        Deadline,
    }
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "snake_case")]
    pub enum Phase {
        Validate,
        Replay,
        Publish,
        Paused,
        Settling,
    }
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "snake_case")]
    pub enum Outcome {
        Completed,
        Failed,
        Cancelled,
    }
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "snake_case")]
    pub enum Knowledge {
        NotRequested,
    }
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "snake_case")]
    pub enum Trust {
        NotEvaluated,
    }
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct Context {
        pub account: Id,
        pub account_epoch: Number,
        pub seat: Ulid,
        pub generation: Number,
        pub grant_epoch: Number,
        pub swarm: Ulid,
        pub swarm_revision: Number,
        pub workspace: Id,
        pub workspace_revision: Number,
        pub root: Id,
        pub root_revision: Number,
        pub provider: Id,
        pub provider_epoch: Number,
    }
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct Selection {
        pub session: Ulid,
        pub source_generation: Number,
    }
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct Error {
        pub code: Code,
        pub source: Option<u32>,
    }
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct RunView {
        pub run_id: Ulid,
        pub revision: Number,
        pub state: State,
        pub settled: bool,
        pub request_sha256: Hex,
        pub manifest_sha256: Option<Hex>,
        pub result_sha256: Option<Hex>,
        pub pages: u32,
        pub error: Option<Error>,
    }
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct Blob {
        pub sha256: Hex,
        pub len: Number,
        pub mime: Mime,
    }
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct Source {
        pub session: Ulid,
        pub source_generation: Number,
        pub repo: Repo,
        pub len: Number,
        pub sha256: Hex,
        pub closed_seq: Number,
        pub closed_line_sha256: Hex,
        pub blobs: Vec<Blob>,
    }
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct Manifest {
        pub schema: ManifestSchema,
        pub run_id: Ulid,
        pub nonce: Ulid,
        pub context: Context,
        pub trigger: Trigger,
        pub algorithm: Algorithm,
        pub sources: Vec<Source>,
    }
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct Start {
        pub protocol: Protocol,
        pub run_id: Ulid,
        pub nonce: Ulid,
        pub manifest_sha256: Hex,
        pub timeout_ms: u32,
        pub activity_revision: Number,
        pub foreground: bool,
    }
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct Activity {
        pub protocol: Protocol,
        pub run_id: Ulid,
        pub nonce: Ulid,
        pub revision: Number,
        pub foreground: bool,
    }
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct WorkerCancel {
        pub protocol: Protocol,
        pub run_id: Ulid,
        pub nonce: Ulid,
    }
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct Hello {
        pub protocol: Protocol,
        pub algorithm: Algorithm,
        pub schema_sha256: Hex,
        pub ir_major: u32,
        pub ir_minor_max: u32,
    }
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct Progress {
        pub protocol: Protocol,
        pub run_id: Ulid,
        pub nonce: Ulid,
        pub seq: Number,
        pub phase: Phase,
        pub bytes: Number,
        pub envelopes: Number,
        pub cpu_ns: Number,
    }
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct Done {
        pub protocol: Protocol,
        pub run_id: Ulid,
        pub nonce: Ulid,
        pub seq: Number,
        pub outcome: Outcome,
        pub result_sha256: Option<Hex>,
        pub pages: u32,
        pub error: Option<Error>,
    }
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct PageRef {
        pub index: u32,
        pub len: Number,
        pub sha256: Hex,
    }
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct ResultDescriptor {
        pub schema: ResultSchema,
        pub run_id: Ulid,
        pub nonce: Ulid,
        pub manifest_sha256: Hex,
        pub algorithm: Algorithm,
        pub source_count: u32,
        pub episode_count: u32,
        pub cluster_count: u32,
        pub max_poll_cpu_ns: Number,
        pub knowledge: Knowledge,
        pub trust: Trust,
        pub pages: Vec<PageRef>,
    }
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct Page {
        pub schema: PageSchema,
        pub run_id: Ulid,
        pub manifest_sha256: Hex,
        pub index: u32,
        pub rows: Vec<Row>,
        pub next: Option<u32>,
    }
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct Origin {
        pub session: Ulid,
        pub seq: Number,
        pub digest: String,
    }
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct Episode {
        pub session: Ulid,
        pub turn: Ulid,
        pub origin: Origin,
        pub fingerprints: Vec<Hex>,
        pub first_ms: Number,
        pub last_ms: Number,
    }
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct Cluster {
        pub repo: Repo,
        pub fingerprint: Hex,
        pub count: u32,
        pub first_ms: Number,
        pub last_ms: Number,
    }
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct Inspect {
        pub v: Version,
        pub op_id: Ulid,
        pub sources: Vec<Selection>,
    }
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct Status {
        pub v: Version,
        pub run_id: Ulid,
        pub page: Option<u32>,
    }
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct Cancel {
        pub v: Version,
        pub op_id: Ulid,
        pub run_id: Ulid,
    }
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
    pub enum WorkerIn {
        Start(Start),
        Activity(Activity),
        Cancel(WorkerCancel),
    }
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
    pub enum WorkerOut {
        Hello(Hello),
        Progress(Progress),
        Done(Done),
    }
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
    pub enum Row {
        Episode(Episode),
        Cluster(Cluster),
    }
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(tag = "method", rename_all = "snake_case", deny_unknown_fields)]
    pub enum Control {
        Inspect(Inspect),
        Status(Status),
        Cancel(Cancel),
    }
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(untagged)]
    pub enum ControlReply {
        Success(Success),
        Failure(Failure),
    }
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct Success {
        pub v: Version,
        pub ok: True,
        pub run: RunView,
        pub page: Option<Page>,
    }
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct Failure {
        pub v: Version,
        pub ok: False,
        pub error: Error,
    }
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(try_from = "bool", into = "bool")]
    pub struct True;
    impl TryFrom<bool> for True {
        type Error = &'static str;
        fn try_from(b: bool) -> Result<Self, Self::Error> {
            if b == true {
                Ok(Self)
            } else {
                Err("reply discriminator")
            }
        }
    }
    impl From<True> for bool {
        fn from(_: True) -> bool {
            true
        }
    }
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(try_from = "bool", into = "bool")]
    pub struct False;
    impl TryFrom<bool> for False {
        type Error = &'static str;
        fn try_from(b: bool) -> Result<Self, Self::Error> {
            if b == false {
                Ok(Self)
            } else {
                Err("reply discriminator")
            }
        }
    }
    impl From<False> for bool {
        fn from(_: False) -> bool {
            false
        }
    }
}

#[cfg(target_os = "linux")]
use std::os::unix::fs::OpenOptionsExt;
use std::{
    io::{Read, Write},
    os::{
        fd::AsRawFd,
        unix::{
            fs::MetadataExt,
            process::{CommandExt, ExitStatusExt},
        },
    },
    path::Path,
    process::{Child, ChildStderr, ChildStdin, ChildStdout, Command, ExitStatus, Stdio},
    time::{Duration, Instant},
};
use wire::{Code, WorkerIn, WorkerOut};

/// Kernel birth identity, without an auxiliary process or unbounded stdout/stderr.
/// Adapted from bboxy450-term crates/bbt-mux/src/ctl_door.rs::parent.
/// https://github.com/bboxy450/bboxy450-term/blob/main/crates/bbt-mux/src/ctl_door.rs
pub fn process_stamp(pid: u32) -> Result<String, Code> {
    if pid == 0 || pid > i32::MAX as u32 {
        return Err(Code::WorkerLost);
    }
    #[cfg(target_os = "macos")]
    {
        // SAFETY: the kernel writes at most the supplied plain-C structure size.
        let mut info: libc::proc_bsdinfo = unsafe { std::mem::zeroed() };
        let size = std::mem::size_of::<libc::proc_bsdinfo>() as i32;
        let read = unsafe {
            libc::proc_pidinfo(
                pid as i32,
                libc::PROC_PIDTBSDINFO,
                0,
                (&mut info as *mut libc::proc_bsdinfo).cast(),
                size,
            )
        };
        if read != size || info.pbi_pid != pid || info.pbi_uid != unsafe { libc::geteuid() } {
            return Err(Code::WorkerLost);
        }
        Ok(format!(
            "kernel:{}:{}",
            info.pbi_start_tvsec, info.pbi_start_tvusec
        ))
    }
    #[cfg(target_os = "linux")]
    {
        let path = format!("/proc/{pid}/stat");
        let mut file = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(path)
            .map_err(|_| Code::WorkerLost)?;
        let mut bytes = [0u8; 4096];
        let n = file.read(&mut bytes).map_err(|_| Code::WorkerLost)?;
        if n == bytes.len() {
            return Err(Code::ResourceLimit);
        }
        let text = std::str::from_utf8(&bytes[..n]).map_err(|_| Code::WorkerLost)?;
        let ticks = text
            .rsplit_once(')')
            .and_then(|(_, s)| s.split_whitespace().nth(19))
            .ok_or(Code::WorkerLost)?;
        let ticks = ticks.parse::<u64>().map_err(|_| Code::WorkerLost)?;
        Ok(format!("kernel:{ticks}"))
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        Err(Code::RunnerUnavailable)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ChildUsage {
    pub peak_rss_bytes: u64,
    /// wait4 process usage, not thread CPU and not a heap measurement.
    pub user_cpu_ns: u64,
    pub system_cpu_ns: u64,
}
pub const PRODUCER_METRIC_KEYS: [&str; 19] = [
    "heap_live_bytes",
    "heap_peak_bytes",
    "startup_heap_peak_bytes",
    "rss_peak_bytes",
    "compute_cpu_ns",
    "max_unit_cpu_ns",
    "overruns",
    "envelopes",
    "bytes",
    "storage_declared_live_bytes",
    "storage_reserved_peak_bytes",
    "storage_cumulative_credits_bytes",
    "control_reserved_bytes",
    "helper_reserved_bytes",
    "credit_ceiling_bytes",
    "normalization_unit_0_cpu_ns",
    "normalization_unit_1_cpu_ns",
    "normalization_unit_2_cpu_ns",
    "normalization_unit_3_cpu_ns",
];
#[derive(Clone, Copy, Debug)]
pub struct ProducerMetrics {
    pub values: [Option<u64>; 19],
}
impl ProducerMetrics {
    pub fn get(&self, key: &str) -> Option<u64> {
        PRODUCER_METRIC_KEYS
            .iter()
            .position(|v| *v == key)
            .and_then(|i| self.values[i])
    }
}
fn child_usage(usage: libc::rusage) -> ChildUsage {
    let ns = |t: libc::timeval| {
        (t.tv_sec.max(0) as u64)
            .saturating_mul(1_000_000_000)
            .saturating_add((t.tv_usec.max(0) as u64).saturating_mul(1000))
    };
    let rss = usage.ru_maxrss.max(0) as u64;
    ChildUsage {
        peak_rss_bytes: if cfg!(target_os = "macos") {
            rss
        } else {
            rss.saturating_mul(1024)
        },
        user_cpu_ns: ns(usage.ru_utime),
        system_cpu_ns: ns(usage.ru_stime),
    }
}

pub const MAX_JSON_DEPTH: usize = 32;
pub const MAX_JSON_CONTAINER: usize = 4096;
pub const MAX_JSON_ITEMS: usize = 65_536;
#[derive(Clone, Copy)]
pub enum JsonShape {
    Any,
    /// Producer's canonical typed JSON: ASCII sorted object keys, unsigned
    /// integers, minimal serde_json string escapes and no outside whitespace.
    Canonical,
    WorkerIn,
    WorkerOut,
}
#[derive(Clone, Copy, Debug)]
pub struct JsonBounds {
    pub values: usize,
    pub keys: usize,
    pub string_bytes: usize,
    pub root_next: bool,
}
impl JsonBounds {
    /// Retained logical payload/entries, not a claim about opaque allocator internals.
    pub fn declared_payload_bytes(self) -> Result<usize, Code> {
        self.values
            .checked_mul(std::mem::size_of::<serde_json::Value>())
            .and_then(|n| {
                self.keys
                    .checked_mul(std::mem::size_of::<String>())
                    .and_then(|k| n.checked_add(k))
            })
            .and_then(|n| n.checked_add(self.string_bytes))
            .ok_or(Code::ResourceLimit)
    }
}

/// Allocation-free structural admission before general JSON/serde materialization.
pub fn preflight_json(bytes: &[u8], shape: JsonShape) -> Result<JsonBounds, Code> {
    preflight_json_cooperative(bytes, shape, || Ok(()))
}
/// Same structural scanner with bounded ownership checkpoints; callback pauses or
/// cancels without allocating/restarting the parser or rescanning a prefix.
pub fn preflight_json_cooperative(
    bytes: &[u8],
    shape: JsonShape,
    mut checkpoint: impl FnMut() -> Result<(), Code>,
) -> Result<JsonBounds, Code> {
    std::str::from_utf8(bytes).map_err(|_| Code::ProtocolError)?;
    struct Scan<'a> {
        bytes: &'a [u8],
        at: usize,
        bounds: JsonBounds,
        shape: JsonShape,
        checkpoint: &'a mut dyn FnMut() -> Result<(), Code>,
        next_checkpoint: usize,
    }
    impl Scan<'_> {
        fn checkpoint(&mut self) -> Result<(), Code> {
            if self.at >= self.next_checkpoint {
                (self.checkpoint)()?;
                self.next_checkpoint = self.at.saturating_add(4096);
            }
            Ok(())
        }
        fn ws(&mut self) -> Result<(), Code> {
            while self
                .bytes
                .get(self.at)
                .is_some_and(|b| b" \n\r\t".contains(b))
            {
                if matches!(self.shape, JsonShape::Canonical) {
                    return Err(Code::ProtocolError);
                }
                self.at += 1;
                self.checkpoint()?;
            }
            Ok(())
        }
        fn count(&mut self, key: bool) -> Result<(), Code> {
            self.checkpoint()?;
            if key {
                self.bounds.keys += 1;
            } else {
                self.bounds.values += 1;
            }
            if self.bounds.keys + self.bounds.values > MAX_JSON_ITEMS {
                Err(Code::ResourceLimit)
            } else {
                Ok(())
            }
        }
        fn string(&mut self) -> Result<(), Code> {
            if self.bytes.get(self.at) != Some(&b'"') {
                return Err(Code::ProtocolError);
            }
            self.at += 1;
            let start = self.at;
            loop {
                if !matches!(self.shape, JsonShape::Any | JsonShape::Canonical)
                    && self.at - start > 256
                {
                    return Err(Code::ResourceLimit);
                }
                self.checkpoint()?;
                let c = *self.bytes.get(self.at).ok_or(Code::ProtocolError)?;
                match c {
                    b'"' => {
                        let n = self.at - start;
                        self.bounds.string_bytes = self
                            .bounds
                            .string_bytes
                            .checked_add(n)
                            .ok_or(Code::ResourceLimit)?;
                        if !matches!(self.shape, JsonShape::Any | JsonShape::Canonical) && n > 256 {
                            return Err(Code::ResourceLimit);
                        }
                        self.at += 1;
                        return Ok(());
                    }
                    b'\\' => {
                        self.at += 1;
                        let escaped = *self.bytes.get(self.at).ok_or(Code::ProtocolError)?;
                        if matches!(self.shape, JsonShape::Canonical) && escaped == b'/' {
                            return Err(Code::ProtocolError);
                        }
                        if escaped == b'u' {
                            let start = self.at + 1;
                            for _ in 0..4 {
                                self.at += 1;
                                if !self.bytes.get(self.at).is_some_and(u8::is_ascii_hexdigit) {
                                    return Err(Code::ProtocolError);
                                }
                            }
                            if matches!(self.shape, JsonShape::Canonical) {
                                let digits = &self.bytes[start..self.at + 1];
                                if digits[..2] != *b"00"
                                    || digits.iter().any(|v| v.is_ascii_uppercase())
                                {
                                    return Err(Code::ProtocolError);
                                }
                                let digit = |v: u8| {
                                    if v.is_ascii_digit() {
                                        v - b'0'
                                    } else {
                                        v - b'a' + 10
                                    }
                                };
                                let value = digit(digits[2]) * 16 + digit(digits[3]);
                                if value >= 32 || matches!(value, 8 | 9 | 10 | 12 | 13) {
                                    return Err(Code::ProtocolError);
                                }
                            }
                        } else if !b"\"\\/bfnrt".contains(&escaped) {
                            return Err(Code::ProtocolError);
                        }
                        self.at += 1;
                    }
                    0..=31 => return Err(Code::ProtocolError),
                    _ => self.at += 1,
                }
            }
        }
        fn value(&mut self, depth: usize) -> Result<(), Code> {
            self.ws()?;
            self.count(false)?;
            match self
                .bytes
                .get(self.at)
                .copied()
                .ok_or(Code::ProtocolError)?
            {
                b'{' => self.object(depth + 1),
                b'[' => self.array(depth + 1),
                b'"' => self.string(),
                b't' => self.literal(b"true"),
                b'f' => self.literal(b"false"),
                b'n' => self.literal(b"null"),
                b'-' | b'0'..=b'9' => self.number(),
                _ => Err(Code::ProtocolError),
            }
        }
        fn literal(&mut self, word: &[u8]) -> Result<(), Code> {
            if self.bytes.get(self.at..self.at + word.len()) != Some(word) {
                return Err(Code::ProtocolError);
            }
            self.at += word.len();
            Ok(())
        }
        fn number(&mut self) -> Result<(), Code> {
            let start_number = self.at;
            if self.bytes.get(self.at) == Some(&b'-') {
                self.at += 1;
            }
            match self.bytes.get(self.at) {
                Some(b'0') => self.at += 1,
                Some(b'1'..=b'9') => {
                    self.at += 1;
                    while self.bytes.get(self.at).is_some_and(u8::is_ascii_digit) {
                        if matches!(self.shape, JsonShape::Canonical)
                            && self.at - start_number >= 16
                        {
                            return Err(Code::ProtocolError);
                        }
                        self.at += 1;
                        self.checkpoint()?;
                    }
                }
                _ => return Err(Code::ProtocolError),
            }
            if self.bytes.get(self.at) == Some(&b'.') {
                self.at += 1;
                let start = self.at;
                while self.bytes.get(self.at).is_some_and(u8::is_ascii_digit) {
                    if matches!(self.shape, JsonShape::Canonical) && self.at - start_number >= 16 {
                        return Err(Code::ProtocolError);
                    }
                    self.at += 1;
                    self.checkpoint()?;
                }
                if start == self.at {
                    return Err(Code::ProtocolError);
                }
            }
            if self
                .bytes
                .get(self.at)
                .is_some_and(|b| *b == b'e' || *b == b'E')
            {
                self.at += 1;
                if self
                    .bytes
                    .get(self.at)
                    .is_some_and(|b| *b == b'+' || *b == b'-')
                {
                    self.at += 1;
                }
                let start = self.at;
                while self.bytes.get(self.at).is_some_and(u8::is_ascii_digit) {
                    if matches!(self.shape, JsonShape::Canonical) && self.at - start_number >= 16 {
                        return Err(Code::ProtocolError);
                    }
                    self.at += 1;
                    self.checkpoint()?;
                }
                if start == self.at {
                    return Err(Code::ProtocolError);
                }
            }
            if matches!(self.shape, JsonShape::Canonical)
                && !self.bytes[start_number..self.at]
                    .iter()
                    .all(u8::is_ascii_digit)
            {
                return Err(Code::ProtocolError);
            }
            Ok(())
        }
        fn object(&mut self, depth: usize) -> Result<(), Code> {
            if depth > MAX_JSON_DEPTH
                || matches!(self.shape, JsonShape::WorkerIn) && depth > 1
                || matches!(self.shape, JsonShape::WorkerOut) && depth > 2
            {
                return Err(Code::ResourceLimit);
            }
            self.at += 1;
            self.ws()?;
            if self.bytes.get(self.at) == Some(&b'}') {
                self.at += 1;
                return Ok(());
            }
            let mut n = 0;
            let mut previous_key: Option<(usize, usize)> = None;
            loop {
                n += 1;
                let bound = match self.shape {
                    JsonShape::Any | JsonShape::Canonical => MAX_JSON_CONTAINER,
                    JsonShape::WorkerOut if depth == 2 => 2,
                    _ => 10,
                };
                if n > bound {
                    return Err(Code::ResourceLimit);
                }
                self.count(true)?;
                let key_start = self.at + 1;
                self.string()?;
                if matches!(self.shape, JsonShape::Canonical) {
                    let key_end = self.at - 1;
                    let key = &self.bytes[key_start..key_end];
                    if key.len() > 64
                        || !key.iter().all(|v| v.is_ascii_alphanumeric() || *v == b'_')
                        || previous_key.is_some_and(|(a, b)| &self.bytes[a..b] >= key)
                    {
                        return Err(Code::ProtocolError);
                    }
                    if depth == 1 && key == b"next" {
                        self.bounds.root_next = true;
                    }
                    previous_key = Some((key_start, key_end));
                }
                self.ws()?;
                if self.bytes.get(self.at) != Some(&b':') {
                    return Err(Code::ProtocolError);
                }
                self.at += 1;
                self.value(depth)?;
                self.ws()?;
                match self.bytes.get(self.at) {
                    Some(b'}') => {
                        self.at += 1;
                        return Ok(());
                    }
                    Some(b',') => {
                        self.at += 1;
                        self.ws()?;
                    }
                    _ => return Err(Code::ProtocolError),
                }
            }
        }
        fn array(&mut self, depth: usize) -> Result<(), Code> {
            if depth > MAX_JSON_DEPTH
                || !matches!(self.shape, JsonShape::Any | JsonShape::Canonical)
            {
                return Err(Code::ResourceLimit);
            }
            self.at += 1;
            self.ws()?;
            if self.bytes.get(self.at) == Some(&b']') {
                self.at += 1;
                return Ok(());
            }
            let mut n = 0;
            loop {
                n += 1;
                if n > MAX_JSON_CONTAINER {
                    return Err(Code::ResourceLimit);
                }
                self.value(depth)?;
                self.ws()?;
                match self.bytes.get(self.at) {
                    Some(b']') => {
                        self.at += 1;
                        return Ok(());
                    }
                    Some(b',') => self.at += 1,
                    _ => return Err(Code::ProtocolError),
                }
            }
        }
    }
    let mut scan = Scan {
        bytes,
        at: 0,
        bounds: JsonBounds {
            values: 0,
            keys: 0,
            string_bytes: 0,
            root_next: false,
        },
        shape,
        checkpoint: &mut checkpoint,
        next_checkpoint: 4096,
    };
    scan.value(0)?;
    scan.ws()?;
    if scan.at != bytes.len() {
        return Err(Code::ProtocolError);
    }
    if !matches!(shape, JsonShape::Any)
        && bytes.iter().copied().find(|b| !b" \n\r\t".contains(b)) != Some(b'{')
    {
        return Err(Code::ProtocolError);
    }
    Ok(scan.bounds)
}

/// Producer boundary a3425ac0: update only with reviewed canonical schema evidence.
pub const SCHEMA_SHA256: &str = "5e3d8a602eeaa1da5cb97d0f6af0b4f331b216a02255d3dff990be84c30d6a30";

/// Launch ownership is explicit and only constructed after a matching Hello.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DreamOwner {
    TermC1,
}

/// Off-owner capability probe. The child receives no source or manifest bytes.
pub fn probe(binary: &Path, job: &Path) -> Result<DreamOwner, Code> {
    let mut worker = Worker::spawn(binary, job)?;
    worker.hello()?;
    worker.close_input();
    let until = Instant::now() + Duration::from_millis(250);
    while worker.try_wait()?.is_none() && Instant::now() < until {
        worker.wait_io(Duration::from_millis(9));
    }
    // Drop reaps a still-running exact child, never an unrelated process.
    Ok(DreamOwner::TermC1)
}

pub fn canonical<T: serde::Serialize>(value: &T) -> Result<Vec<u8>, serde_json::Error> {
    serde_json::to_vec(&serde_json::to_value(value)?)
}

pub fn decode<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T, Code> {
    if bytes.is_empty() || bytes.len() > wire::MAX_FRAME {
        return Err(Code::ResourceLimit);
    }
    preflight_json(bytes, JsonShape::Any)?;
    serde_json::from_slice(bytes).map_err(|_| Code::ProtocolError)
}

fn decode_output(bytes: &[u8]) -> Result<WorkerOut, Code> {
    preflight_json(bytes, JsonShape::WorkerOut)?;
    serde_json::from_slice(bytes).map_err(|_| Code::ProtocolError)
}

fn frame_buffer(capacity: usize) -> Result<Vec<u8>, Code> {
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(capacity)
        .map_err(|_| Code::ResourceLimit)?;
    Ok(bytes)
}
fn take_frame(pending: &mut Vec<u8>, scratch: &mut Vec<u8>, end: usize) -> Result<WorkerOut, Code> {
    if end + 1 > wire::MAX_FRAME || end + 1 > scratch.capacity() {
        return Err(Code::ResourceLimit);
    }
    scratch.clear();
    scratch.extend_from_slice(&pending[..=end]);
    pending.copy_within(end + 1.., 0);
    pending.truncate(pending.len() - end - 1);
    let result = if scratch.len() == 1 {
        Err(Code::ProtocolError)
    } else {
        decode_output(scratch)
    };
    scratch.clear();
    result
}
struct FrameWriter<'a>(&'a mut Vec<u8>);
impl Write for FrameWriter<'_> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if self
            .0
            .len()
            .checked_add(bytes.len())
            .is_none_or(|n| n > wire::MAX_FRAME - 1)
        {
            return Err(std::io::ErrorKind::OutOfMemory.into());
        }
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn nonblocking(fd: &impl AsRawFd) -> Result<(), Code> {
    // SAFETY: only status flags on the owned pipe descriptor are changed.
    let flags = unsafe { libc::fcntl(fd.as_raw_fd(), libc::F_GETFL) };
    if flags < 0
        || unsafe { libc::fcntl(fd.as_raw_fd(), libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0
    {
        Err(Code::RunnerUnavailable)
    } else {
        Ok(())
    }
}

/// Requested codec backing preallocated before child spawn; separate from native overhead.
pub const PROTECTED_BYTES: usize = 2 * 1024 * 1024;
/// An exact child/group with bounded nonblocking pipes. Never pass this to a hot owner.
pub struct Worker {
    child: Child,
    input: Option<ChildStdin>,
    output: ChildStdout,
    stderr: ChildStderr,
    pending: Vec<u8>,
    write: Vec<u8>,
    written: usize,
    stderr_bytes: usize,
    stderr_capture: Vec<u8>,
    /// Preprovisioned protected codec scratch, reused for every frame without growth.
    frame_scratch: Vec<u8>,
    eof: bool,
    reaped: Option<ExitStatus>,
    usage: Option<ChildUsage>,
}

impl Worker {
    /// Spawn the provisioned absolute sibling directly, without inherited authority.
    pub fn spawn(binary: &Path, job: &Path) -> Result<Self, Code> {
        crate::assert_not_ui_thread();
        if !binary.is_absolute() || !job.is_absolute() {
            return Err(Code::RunnerUnavailable);
        }
        let m = std::fs::symlink_metadata(binary).map_err(|_| Code::RunnerUnavailable)?;
        if !m.is_file()
            || m.mode() & 0o022 != 0
            || (m.uid() != unsafe { libc::geteuid() } && m.uid() != 0)
        {
            return Err(Code::RunnerUnavailable);
        }
        let pending = frame_buffer(wire::MAX_FRAME)?;
        let write = frame_buffer(wire::MAX_FRAME)?;
        let stderr_capture = frame_buffer(16 * 1024)?;
        let frame_scratch = frame_buffer(PROTECTED_BYTES - 2 * wire::MAX_FRAME - 16 * 1024)?;
        let mut command = Command::new(binary);
        command
            .args(["dream", "inspect-c1", "--job-dir"])
            .arg(job)
            .current_dir(job)
            .env_clear()
            .env("LANG", "C.UTF-8")
            .env("TMPDIR", job.join("tmp"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        // SAFETY: child-only async-signal-safe calls; no allocation or inherited lock.
        unsafe {
            command.pre_exec(|| {
                if libc::setpgid(0, 0) != 0 || libc::setpriority(libc::PRIO_PROCESS, 0, 10) != 0 {
                    return Err(std::io::Error::last_os_error());
                }
                if libc::getpriority(libc::PRIO_PROCESS, 0) < 10 {
                    return Err(std::io::Error::other("background priority unavailable"));
                }
                Ok(())
            });
        }
        let mut child = command.spawn().map_err(|_| Code::RunnerUnavailable)?;
        let input = child.stdin.take().ok_or(Code::RunnerUnavailable)?;
        let output = child.stdout.take().ok_or(Code::RunnerUnavailable)?;
        let stderr = child.stderr.take().ok_or(Code::RunnerUnavailable)?;
        let worker = Self {
            child,
            input: Some(input),
            output,
            stderr,
            pending,
            write,
            written: 0,
            stderr_bytes: 0,
            stderr_capture,
            frame_scratch,
            eof: false,
            reaped: None,
            usage: None,
        };
        nonblocking(worker.input.as_ref().unwrap())?;
        nonblocking(&worker.output)?;
        nonblocking(&worker.stderr)?;
        Ok(worker)
    }

    pub fn pid(&self) -> u32 {
        self.child.id()
    }
    /// Declared requested capacities, not RSS or opaque serde/allocator overhead.
    pub fn protected_capacity_bytes(&self) -> usize {
        self.pending.capacity()
            + self.write.capacity()
            + self.stderr_capture.capacity()
            + self.frame_scratch.capacity()
    }
    pub fn is_eof(&self) -> bool {
        self.eof
    }
    /// Failed/cancelled runs grant pending frames no meaning. Clearing their
    /// bounded prefix lets EOF drain even after a framing/flood error.
    pub fn discard_pending_output(&mut self) {
        self.pending.clear();
        self.frame_scratch.clear();
    }

    /// Hello precedes every manifest/source read, including launch capability probing.
    pub fn hello(&mut self) -> Result<wire::Hello, Code> {
        let until = Instant::now() + Duration::from_millis(250);
        loop {
            if let Some(frame) = self.receive()? {
                return match frame {
                    WorkerOut::Hello(h)
                        if h.schema_sha256.0 == SCHEMA_SHA256
                            && h.ir_major == 1
                            && h.ir_minor_max == 3 =>
                    {
                        Ok(h)
                    }
                    _ => Err(Code::VersionMismatch),
                };
            }
            if self.eof || Instant::now() >= until {
                return Err(Code::RunnerUnavailable);
            }
            self.wait_io(Duration::from_millis(9));
        }
    }

    /// At most one pending frame; callers coalesce Activity before numbering.
    pub fn send(&mut self, frame: &WorkerIn) -> Result<bool, Code> {
        if !self.flush()? {
            return Ok(false);
        }
        let (run, nonce) = match frame {
            WorkerIn::Start(v) => (&v.run_id, &v.nonce),
            WorkerIn::Activity(v) => (&v.run_id, &v.nonce),
            WorkerIn::Cancel(v) => (&v.run_id, &v.nonce),
        };
        if run.0.len() != 26 || nonce.0.len() != 26 {
            return Err(Code::ResourceLimit);
        }
        if let WorkerIn::Start(v) = frame
            && (v.manifest_sha256.0.len() != 64
                || v.timeout_ms == 0
                || v.timeout_ms > 30_000
                || v.activity_revision.0 > wire::MAX_SAFE)
        {
            return Err(Code::ResourceLimit);
        }
        if let WorkerIn::Activity(v) = frame
            && v.revision.0 > wire::MAX_SAFE
        {
            return Err(Code::ResourceLimit);
        }
        self.write.clear();
        let value = serde_json::to_value(frame).map_err(|_| Code::ProtocolError)?;
        serde_json::to_writer(FrameWriter(&mut self.write), &value)
            .map_err(|_| Code::ResourceLimit)?;
        preflight_json(&self.write, JsonShape::WorkerIn)?;
        self.write.push(b'\n');
        if self.write.len() > wire::MAX_FRAME {
            return Err(Code::ResourceLimit);
        }
        self.written = 0;
        self.flush()
    }

    pub fn flush(&mut self) -> Result<bool, Code> {
        let Some(input) = &mut self.input else {
            return Err(Code::WorkerLost);
        };
        while self.written < self.write.len() {
            match input.write(&self.write[self.written..]) {
                Ok(0) => return Err(Code::WorkerLost),
                Ok(n) => self.written += n,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => return Ok(false),
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => return Err(Code::WorkerLost),
            }
        }
        self.write.clear();
        self.written = 0;
        Ok(true)
    }

    /// Read no more than 16 KiB per call. Reject an overlong frame before growth.
    pub fn receive(&mut self) -> Result<Option<WorkerOut>, Code> {
        if let Some(end) = self.pending.iter().position(|b| *b == b'\n') {
            return take_frame(&mut self.pending, &mut self.frame_scratch, end).map(Some);
        }
        let mut bytes = [0u8; 16 * 1024];
        let room = wire::MAX_FRAME.saturating_sub(self.pending.len());
        if room == 0 {
            return Err(Code::ResourceLimit);
        }
        let read_bound = room.min(bytes.len());
        match self.output.read(&mut bytes[..read_bound]) {
            Ok(0) => {
                self.eof = true;
                if !self.pending.is_empty() {
                    return Err(Code::ProtocolError);
                }
            }
            Ok(n) => {
                if self.pending.len() + n > wire::MAX_FRAME {
                    return Err(Code::ResourceLimit);
                }
                self.pending.extend_from_slice(&bytes[..n]);
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            Err(_) => return Err(Code::WorkerLost),
        }
        match self.stderr.read(&mut bytes) {
            Ok(n) => {
                self.stderr_bytes += n;
                if self.stderr_bytes > 16 * 1024 {
                    return Err(Code::ResourceLimit);
                }
                self.stderr_capture.extend_from_slice(&bytes[..n]);
            }
            Err(e)
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::Interrupted
                ) => {}
            Err(_) => return Err(Code::WorkerLost),
        }
        if let Some(end) = self.pending.iter().position(|b| *b == b'\n') {
            take_frame(&mut self.pending, &mut self.frame_scratch, end).map(Some)
        } else {
            Ok(None)
        }
    }

    pub fn wait_io(&self, timeout: Duration) {
        let mut fds = [libc::pollfd {
            fd: self.output.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        }];
        // SAFETY: poll borrows the stack array for this call only.
        unsafe {
            libc::poll(fds.as_mut_ptr(), 1, timeout.as_millis().min(100) as i32);
        }
    }

    pub fn try_wait(&mut self) -> Result<Option<ExitStatus>, Code> {
        self.reap(libc::WNOHANG)
    }
    pub fn usage(&self) -> Option<ChildUsage> {
        self.usage
    }
    /// Only numeric allowlisted diagnostics; raw stderr is never returned or echoed.
    pub fn producer_metrics(&self) -> Option<ProducerMetrics> {
        let text = std::str::from_utf8(&self.stderr_capture).ok()?;
        let mut found = None;
        for line in text.lines() {
            let Some(fields) = line.strip_prefix("dream-c1 metrics ") else {
                continue;
            };
            if found.is_some() || line.len() > 2048 {
                return None;
            }
            let mut values = [None; 19];
            for field in fields.split_whitespace() {
                let (key, value) = field.split_once('=')?;
                if key == "native_cumulative_bytes" {
                    if value != "unmeasured" {
                        return None;
                    }
                    continue;
                }
                let Some(index) = PRODUCER_METRIC_KEYS.iter().position(|v| *v == key) else {
                    continue;
                };
                if values[index].is_some() {
                    return None;
                }
                values[index] = Some(value.parse::<u64>().ok()?);
            }
            if values[0]? > values[1]? {
                return None;
            }
            found = Some(ProducerMetrics { values });
        }
        found
    }
    fn reap(&mut self, flags: i32) -> Result<Option<ExitStatus>, Code> {
        if self.reaped.is_some() {
            return Ok(self.reaped);
        }
        loop {
            let mut status = 0;
            // SAFETY: wait4 writes only into the two valid stack structures and names
            // the exact child this Worker owns. This is the sole C1 reaper.
            let mut usage: libc::rusage = unsafe { std::mem::zeroed() };
            let pid =
                unsafe { libc::wait4(self.child.id() as i32, &mut status, flags, &mut usage) };
            if pid == 0 {
                return Ok(None);
            }
            if pid == self.child.id() as i32 {
                self.reaped = Some(ExitStatus::from_raw(status));
                self.usage = Some(child_usage(usage));
                return Ok(self.reaped);
            }
            if std::io::Error::last_os_error().raw_os_error() == Some(libc::EINTR) {
                continue;
            }
            return Err(Code::WorkerLost);
        }
    }

    /// EOF asks the producer to cancel. Ownership remains until a real reap.
    pub fn close_input(&mut self) {
        self.input.take();
    }

    /// Only an unreaped child group created by this owner can receive escalation.
    pub fn terminate(&mut self, kill: bool) -> Result<(), Code> {
        if self.try_wait()?.is_some() {
            return Ok(());
        }
        let signal = if kill { libc::SIGKILL } else { libc::SIGTERM };
        // SAFETY: child remains unreaped, so its PID/group cannot have been reused.
        if unsafe { libc::kill(-(self.child.id() as i32), signal) } == 0 {
            Ok(())
        } else {
            Err(Code::WorkerLost)
        }
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        // Every Worker lives on a dedicated process owner, including failure paths.
        if self.reaped.is_none() {
            self.close_input();
            let _ = self.terminate(true);
            let _ = self.reap(0);
        }
    }
}
