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
    #[allow(
        clippy::large_enum_variant,
        reason = "keep the canonical bounded DTO inline instead of allocating an extra box"
    )]
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
            if b {
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
            if !b {
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

use crate::dream_bootstrap::{self as bootstrap, Gate};
#[cfg(target_os = "linux")]
use std::os::unix::fs::OpenOptionsExt;
use std::{
    cell::Cell,
    fs::File,
    io::{Read, Write},
    os::{
        fd::{AsRawFd, OwnedFd},
        unix::{
            fs::MetadataExt,
            process::{CommandExt, ExitStatusExt},
        },
    },
    path::Path,
    process::{Child, Command, ExitStatus, Stdio},
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
const COMPUTE_METRIC_COUNT: usize = 21;
pub const NATIVE_METRICS_PROFILE: &str = "c1-native-debt/1";
pub const NATIVE_TERMINAL_CUTOFF: &str = "joined_final_control_pre_telemetry";
pub const PRODUCER_METRIC_KEYS: [&str; 69] = [
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
    "cleanup_cpu_ns",
    "max_cleanup_unit_cpu_ns",
    "reactor_max_cpu_ns",
    "control_max_cpu_ns",
    "compute_stack_reserved_bytes",
    "cost_known",
    "native_compute_owner_create_count",
    "native_compute_owner_create_cpu_ns",
    "native_compute_owner_create_wall_ns",
    "native_compute_owner_create_max_cpu_ns",
    "native_compute_owner_create_max_wall_ns",
    "native_compute_owner_create_inclusive_max_cpu_ns",
    "native_compute_owner_create_inclusive_max_wall_ns",
    "native_compute_owner_create_app_max_cpu_ns",
    "native_compute_owner_create_effects",
    "native_compute_owner_create_failures_with_effect",
    "native_publish_noreplace_count",
    "native_publish_noreplace_cpu_ns",
    "native_publish_noreplace_wall_ns",
    "native_publish_noreplace_max_cpu_ns",
    "native_publish_noreplace_max_wall_ns",
    "native_publish_noreplace_inclusive_max_cpu_ns",
    "native_publish_noreplace_inclusive_max_wall_ns",
    "native_publish_noreplace_app_max_cpu_ns",
    "native_publish_noreplace_effects",
    "native_publish_noreplace_failures_with_effect",
    "terminal_captured_max_cpu_ns",
    "terminal_inclusive_max_cpu_ns",
    "terminal_app_max_cpu_ns",
    "terminal_control_max_cpu_ns",
    "terminal_inclusive_max_wall_ns",
    "terminal_span_cpu_ns",
    "terminal_owner_thread_cpu_ns",
    "terminal_process_user_cpu_ns",
    "terminal_process_system_cpu_ns",
    "terminal_wall_ns",
    "terminal_debt_total_ns",
    "terminal_rest_observed_ns",
    "terminal_remaining_debt_ns",
    "control_parts_input_max_cpu_ns",
    "control_parts_protocol_max_cpu_ns",
    "control_parts_settlement_max_cpu_ns",
    "control_parts_output_max_cpu_ns",
    "control_parts_total_cpu_ns",
    "publication_operation",
    "publication_dev",
    "publication_ino",
    "publication_effect",
    "publication_errno",
    "publication_cleanup_uncertain",
];
#[derive(Clone, Copy, Debug)]
pub struct NativeMetricsProfile {
    run: [u8; 26],
    nonce: [u8; 26],
}
#[derive(Clone, Copy, Debug)]
pub struct ProducerMetrics {
    pub values: [Option<u64>; PRODUCER_METRIC_KEYS.len()],
    native_profile: Option<NativeMetricsProfile>,
}
impl ProducerMetrics {
    pub fn get(&self, key: &str) -> Option<u64> {
        PRODUCER_METRIC_KEYS
            .iter()
            .position(|v| *v == key)
            .and_then(|i| self.values[i])
    }
    pub fn has_native_profile(&self) -> bool {
        self.native_profile.is_some()
    }
    pub fn work_debt_paid_for(&self, run: &str, nonce: &str) -> bool {
        self.native_profile.is_some_and(|p| {
            p.run.as_slice() == run.as_bytes() && p.nonce.as_slice() == nonce.as_bytes()
        }) && self.get("cost_known") == Some(1)
            && self.get("terminal_remaining_debt_ns") == Some(0)
            && (45..63).all(|i| self.values[i].is_some_and(|v| v <= wire::MAX_SAFE))
    }
    /// Numeric/profile consistency only; approved executable/source/compiler/
    /// feature binding remains a separate host-provisioned authority fence.
    pub fn valid_native_completion(&self, run: &str, nonce: &str, pages: u32) -> bool {
        if !self.work_debt_paid_for(run, nonce)
            || !(1..=4096).contains(&pages)
            || self.values.iter().any(Option::is_none)
        {
            return false;
        }
        let v = |i: usize| self.values[i].unwrap();
        let native = |base: usize, count: u64| {
            v(base) == count
                && v(base + 8) == count
                && v(base + 9) == 0
                && v(base + 3) > 0
                && v(base + 3) <= 2_000_000
                && v(base + 4) > 0
                && v(base + 4) <= 25_000_000
                && v(base + 5) >= v(base + 3)
                && v(base + 5) <= 3_000_000
                && v(base + 6) >= v(base + 4)
                && v(base + 6) <= wire::MAX_SAFE
                && v(base + 7) <= 1_000_000
                && v(base + 5) <= v(base + 3).saturating_add(v(base + 7))
                && v(base + 1) >= v(base + 3)
                && v(base + 1) <= count.saturating_mul(v(base + 3))
                && v(base + 2) >= v(base + 4)
                && v(base + 2) <= count.saturating_mul(v(base + 4))
                && (base + 1..=base + 7).all(|i| v(i) <= wire::MAX_SAFE)
        };
        let terminal = v(46);
        native(25, 1)
            && native(35, u64::from(pages) + 1)
            && v(4) > 0
            && v(4) <= wire::MAX_SAFE
            && v(19) <= wire::MAX_SAFE
            && terminal > 0
            && terminal <= 3_000_000
            && terminal == [v(30), v(40), v(47), v(48)].into_iter().max().unwrap()
            && v(45) <= terminal
            && [v(5), v(30), v(40), v(47), v(48), v(20)]
                .into_iter()
                .all(|n| n <= terminal)
            && [6usize, 9, 44, 67, 68].into_iter().all(|i| v(i) == 0)
            && [15usize, 16, 17, 18, 20, 21, 22, 47, 48, 58, 59, 60, 61]
                .into_iter()
                .all(|i| v(i) <= 1_000_000)
            && v(0) <= v(1)
            && v(1) > 0
            && v(1) <= 96 * 1024 * 1024
            && v(2) > 0
            && v(2) <= 96 * 1024 * 1024
            && v(10) <= 78 * 1024 * 1024
            && v(11) >= v(10)
            && v(11) <= wire::MAX_SAFE
            && v(12) == 2 * 1024 * 1024
            && v(13) == 16 * 1024 * 1024
            && v(14) == 96 * 1024 * 1024
            && v(23) == 2 * 1024 * 1024
            && v(7) <= 65_536
            && v(8) <= 64 * 1024 * 1024
            && v(50) > 0
            && v(51) > 0
            && v(52)
                .checked_add(v(53))
                .is_some_and(|n| n > 0 && n <= wire::MAX_SAFE)
            && v(54) > 0
            && v(55) > 0
            && v(56) > 0
            && v(63) == 4096
            && v(64) > 0
            && v(65) > 0
            && v(66) == 1
    }
    pub fn validates_descriptor_capture(&self, captured: u64) -> bool {
        captured <= 3_000_000
            && self.get("terminal_captured_max_cpu_ns") == Some(captured)
            && self
                .get("terminal_inclusive_max_cpu_ns")
                .is_some_and(|max| captured <= max)
    }
}

// Private telemetry adapter for the producer-owned profile, not wire/corpus authority.
// Adapted from bboxy450-harness src/dream/c1.rs::TerminalMetrics (ffa09e52).
// https://github.com/bboxy450/bboxy450-harness/blob/main/src/dream/c1.rs
const NATIVE_FIELDS: [&str; 10] = [
    "count",
    "cpu_ns",
    "wall_ns",
    "max_cpu_ns",
    "max_wall_ns",
    "inclusive_max_cpu_ns",
    "inclusive_max_wall_ns",
    "app_max_cpu_ns",
    "effects",
    "failures_with_effect",
];
const TERMINAL_FIELDS: [&str; 13] = [
    "captured_max_cpu_ns",
    "inclusive_max_cpu_ns",
    "app_max_cpu_ns",
    "control_max_cpu_ns",
    "inclusive_max_wall_ns",
    "span_cpu_ns",
    "owner_thread_cpu_ns",
    "process_user_cpu_ns",
    "process_system_cpu_ns",
    "wall_ns",
    "debt_total_ns",
    "rest_observed_ns",
    "remaining_debt_ns",
];
const CONTROL_FIELDS: [&str; 5] = [
    "input_max_cpu_ns",
    "protocol_max_cpu_ns",
    "settlement_max_cpu_ns",
    "output_max_cpu_ns",
    "total_cpu_ns",
];
fn metric_number(text: &str) -> Option<u64> {
    if text.is_empty() || text.len() > 20 || !text.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}
fn metric_bool(text: &str) -> Option<u64> {
    match text {
        "true" => Some(1),
        "false" => Some(0),
        _ => None,
    }
}
fn metric_id(text: &str) -> Option<[u8; 26]> {
    let bytes: [u8; 26] = text.as_bytes().try_into().ok()?;
    if bytes[0] > b'7'
        || !bytes
            .iter()
            .all(|b| b"0123456789ABCDEFGHJKMNPQRSTVWXYZ".contains(b))
    {
        return None;
    }
    Some(bytes)
}
fn metric_fields(
    fields: &str,
    names: &[&str],
    base: usize,
    values: &mut [Option<u64>; PRODUCER_METRIC_KEYS.len()],
) -> Option<()> {
    for field in fields.split_ascii_whitespace() {
        let (key, value) = field.split_once('=')?;
        let index = base + names.iter().position(|name| *name == key)?;
        if values[index].is_some() {
            return None;
        }
        values[index] = Some(metric_number(value)?);
    }
    values[base..base + names.len()]
        .iter()
        .all(Option::is_some)
        .then_some(())
}
fn parse_producer_metrics(stderr: &[u8]) -> Option<ProducerMetrics> {
    if stderr.len() > 16 * 1024 || !stderr.ends_with(b"\n") {
        return None;
    }
    let text = std::str::from_utf8(stderr).ok()?;
    let mut values = [None; PRODUCER_METRIC_KEYS.len()];
    let mut compute = false;
    let mut mask = 0u8;
    let mut run = None;
    let mut nonce = None;
    for line in text.lines() {
        if !line.starts_with("dream-c1 ") {
            continue;
        }
        if line.len() > 2048 {
            return None;
        }
        if let Some(fields) = line.strip_prefix("dream-c1 metrics ") {
            if compute {
                return None;
            }
            compute = true;
            let mut native = false;
            for field in fields.split_ascii_whitespace() {
                let (key, value) = field.split_once('=')?;
                if key == "native_cumulative_bytes" {
                    if native || value != "unmeasured" {
                        return None;
                    }
                    native = true;
                    continue;
                }
                let index = if key == "compute_stack_reserved_bytes" {
                    Some(23)
                } else {
                    PRODUCER_METRIC_KEYS[..COMPUTE_METRIC_COUNT]
                        .iter()
                        .position(|v| *v == key)
                };
                let Some(index) = index else {
                    continue;
                };
                if values[index].is_some() {
                    return None;
                }
                values[index] = Some(metric_number(value)?);
            }
        } else if let Some(fields) = line.strip_prefix("dream-c1 native tag=") {
            let (tag, fields) = fields.split_once(' ')?;
            let (base, bit) = match tag {
                "compute_owner_create" => (25, 1),
                "publish_noreplace" => (35, 2),
                _ => return None,
            };
            if mask & bit != 0 {
                return None;
            }
            metric_fields(fields, &NATIVE_FIELDS, base, &mut values)?;
            mask |= bit;
        } else if let Some(value) = line.strip_prefix("dream-c1 cost_known=") {
            if values[24].is_some() {
                return None;
            }
            values[24] = Some(metric_bool(value)?);
            mask |= 4;
        } else if let Some(fields) = line.strip_prefix("dream-c1 terminal_metrics ") {
            if mask & 8 != 0 {
                return None;
            }
            let (profile, fields) = fields.split_once(' ')?;
            let (cutoff, fields) = fields.split_once(' ')?;
            if profile != "profile=c1-native-debt/1"
                || cutoff != "cutoff=joined_final_control_pre_telemetry"
            {
                return None;
            }
            metric_fields(fields, &TERMINAL_FIELDS, 45, &mut values)?;
            mask |= 8;
        } else if let Some(fields) = line.strip_prefix("dream-c1 control_parts ") {
            if mask & 16 != 0 {
                return None;
            }
            metric_fields(fields, &CONTROL_FIELDS, 58, &mut values)?;
            mask |= 16;
        } else if let Some(fields) = line.strip_prefix("dream-c1 publication_custody ") {
            if mask & 32 != 0 {
                return None;
            }
            let mut path = false;
            let mut temp = false;
            for field in fields.split_ascii_whitespace() {
                let (key, value) = field.split_once('=')?;
                match key {
                    "run_id" => {
                        if run.is_some() {
                            return None;
                        }
                        run = Some(metric_id(value)?);
                    }
                    "nonce" => {
                        if nonce.is_some() {
                            return None;
                        }
                        nonce = Some(metric_id(value)?);
                    }
                    "path_mapping" => {
                        if path || value != "page_index_or_4096_result" {
                            return None;
                        }
                        path = true;
                    }
                    "temp_mapping" => {
                        if temp || value != "dot_name_original_Start_nonce_tmp" {
                            return None;
                        }
                        temp = true;
                    }
                    _ => {
                        let index = 63
                            + [
                                "operation",
                                "dev",
                                "ino",
                                "effect",
                                "errno",
                                "cleanup_uncertain",
                            ]
                            .iter()
                            .position(|v| *v == key)?;
                        if values[index].is_some() {
                            return None;
                        }
                        values[index] = Some(if key == "cleanup_uncertain" {
                            metric_bool(value)?
                        } else {
                            metric_number(value)?
                        });
                    }
                }
            }
            if run.is_none()
                || nonce.is_none()
                || !path
                || !temp
                || !values[63..69].iter().all(Option::is_some)
            {
                return None;
            }
            mask |= 32;
        } else {
            let (index, receipt, reactor) =
                if let Some(value) = line.strip_prefix("dream-c1 reactor_max_cpu_ns=") {
                    (21, value, true)
                } else if let Some(value) = line.strip_prefix("dream-c1 control_max_cpu_ns=") {
                    (22, value, false)
                } else {
                    continue;
                };
            if values[index].is_some() {
                return None;
            }
            let numeric = if reactor {
                match receipt.split_once(' ') {
                    Some((value, "startup_runtime=none")) => value,
                    Some(_) => return None,
                    None => receipt,
                }
            } else {
                receipt
            };
            values[index] = Some(metric_number(numeric)?);
        }
    }
    if !compute || values[0]? > values[1]? {
        return None;
    }
    let native_profile = if mask == 63 {
        Some(NativeMetricsProfile {
            run: run?,
            nonce: nonce?,
        })
    } else {
        None
    };
    Some(ProducerMetrics {
        values,
        native_profile,
    })
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

/// Frozen ICD78219b913 entry line. Auxiliary s/g/u/k/r/p are raw, unverified
/// counters: no clock conversion, subtraction or CPU/loader classification.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub struct BootstrapEntryRecord {
    pub s: u64,
    pub g: u64,
    pub e: u64,
    pub m: u64,
    pub u: u64,
    pub k: u64,
    pub r: u64,
    pub p: u64,
    pub ok: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum BootstrapEntryClock {
    ClockGettimeMonotonic,
    Unknown,
}

/// The original return Instant lies within the explicit same-clock bracket.
/// A conservative lower bound may be the sample before the native attempt.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub struct BootstrapEntryClockBounds {
    pub clock: BootstrapEntryClock,
    pub before_spawn_ns: u64,
    pub return_lower_ns: u64,
    pub return_upper_ns: u64,
}
impl BootstrapEntryClockBounds {
    pub fn new(
        clock: BootstrapEntryClock,
        before_spawn_ns: u64,
        return_lower_ns: u64,
        return_upper_ns: u64,
    ) -> Result<Self, Code> {
        if clock != BootstrapEntryClock::ClockGettimeMonotonic
            || before_spawn_ns > return_lower_ns
            || return_lower_ns > return_upper_ns
        {
            return Err(Code::ResourceLimit);
        }
        Ok(Self {
            clock,
            before_spawn_ns,
            return_lower_ns,
            return_upper_ns,
        })
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BootstrapEntryEvidenceState {
    #[default]
    Pending,
    Known,
    Unknown,
}

#[derive(Clone, Copy, Debug, serde::Serialize)]
pub struct BootstrapEntryEvidence {
    pub status: BootstrapEntryEvidenceState,
    pub record: Option<BootstrapEntryRecord>,
    pub observed_ns: Option<u64>,
    pub bounds: Option<BootstrapEntryClockBounds>,
    pub return_clock_uncertainty_ns: Option<u64>,
    pub signed_spawn_return_to_entry_ns: Option<[i128; 2]>,
    pub recognition_ns: Option<u64>,
    /// m is PRE-write. The actual write lies in [m,observed_ns]; this is
    /// only its observation-lag upper bound, never an emission timestamp.
    pub write_observation_upper_ns: Option<u64>,
}

pub fn parse_bootstrap_entry(line: &[u8]) -> Result<BootstrapEntryRecord, Code> {
    if line.len() > 256 {
        return Err(Code::ResourceLimit);
    }
    let body = line
        .strip_prefix(b"dream-bootstrap entry ")
        .and_then(|line| line.strip_suffix(b"\n"))
        .ok_or(Code::ProtocolError)?;
    let mut fields = body.split(|byte| *byte == b' ');
    let mut values = [0u64; 9];
    for (index, key) in [
        b"s=".as_slice(),
        b"g=",
        b"e=",
        b"m=",
        b"u=",
        b"k=",
        b"r=",
        b"p=",
        b"ok=",
    ]
    .into_iter()
    .enumerate()
    {
        let value = fields
            .next()
            .and_then(|field| field.strip_prefix(key))
            .ok_or(Code::ProtocolError)?;
        if value.is_empty() || value.len() > 20 || value.len() > 1 && value[0] == b'0' {
            return Err(Code::ProtocolError);
        }
        for &byte in value {
            if !byte.is_ascii_digit() {
                return Err(Code::ProtocolError);
            }
            values[index] = values[index]
                .checked_mul(10)
                .and_then(|n| n.checked_add(u64::from(byte - b'0')))
                .ok_or(Code::ProtocolError)?;
        }
    }
    if fields.next().is_some()
        || values[8] > 1
        || values[2] > values[3]
        || values[8] == 0 && [0, 4, 5, 6, 7].into_iter().any(|i| values[i] != 0)
    {
        return Err(Code::ProtocolError);
    }
    Ok(BootstrapEntryRecord {
        s: values[0],
        g: values[1],
        e: values[2],
        m: values[3],
        u: values[4],
        k: values[5],
        r: values[6],
        p: values[7],
        ok: values[8] == 1,
    })
}

#[derive(Clone, Copy, Debug, Default)]
pub struct BootstrapMarkers {
    pub entry: Option<BootstrapEntryRecord>,
    pub waiting: bool,
    pub waiting_admissions: [u64; 3],
    pub paid_hello: bool,
    /// argv, channel, compute-owner and frame-buffer admissions, in that order.
    pub paid_admissions: [u64; 4],
    pub permit_reads: u64,
    pub control_cpu_ns: u64,
    pub control_max_cpu_ns: u64,
}

/// Fixed retained control state, charged out of existing codec scratch.
const ENTRY_CONTROL_BYTES: usize = 256;
#[derive(Default)]
struct BootstrapEntryTracking {
    record: Option<BootstrapEntryRecord>,
    observed_ns: Option<u64>,
    bounds: Option<BootstrapEntryClockBounds>,
    last_observation_ns: Option<u64>,
    invalid: bool,
    stderr_eof: bool,
}
const _: () = assert!(std::mem::size_of::<BootstrapEntryTracking>() <= ENTRY_CONTROL_BYTES);
impl BootstrapEntryTracking {
    fn observe(&mut self, bytes: &[u8], clock: Result<u64, bootstrap::Error>) {
        let observed = match clock {
            Ok(now) => {
                if self.last_observation_ns.is_some_and(|prior| now < prior) {
                    self.invalid = true;
                }
                self.last_observation_ns = Some(now);
                Some(now)
            }
            Err(_) => {
                self.invalid = true;
                None
            }
        };
        if self.observed_ns.is_none()
            && let Some(line) = bytes
                .split_inclusive(|b| *b == b'\n')
                .find(|line| line.starts_with(b"dream-bootstrap entry") && line.ends_with(b"\n"))
        {
            self.record = parse_bootstrap_entry(line).ok();
            self.observed_ns = observed;
        }
        if parse_bootstrap_markers(bytes).is_err() {
            self.invalid = true;
        }
    }
    fn evidence(&self, bytes: &[u8], reaped: bool) -> BootstrapEntryEvidence {
        let mut evidence = BootstrapEntryEvidence {
            status: BootstrapEntryEvidenceState::Pending,
            record: self.record,
            observed_ns: self.observed_ns,
            bounds: self.bounds,
            return_clock_uncertainty_ns: None,
            signed_spawn_return_to_entry_ns: None,
            recognition_ns: None,
            write_observation_upper_ns: None,
        };
        let markers = parse_bootstrap_markers(bytes);
        if self.invalid || markers.is_err() {
            evidence.status = BootstrapEntryEvidenceState::Unknown;
            return evidence;
        }
        let markers = markers.unwrap();
        if (self.stderr_eof || reaped)
            && bytes
                .rsplit(|b| *b == b'\n')
                .next()
                .is_some_and(|tail| tail.starts_with(b"dream-bootstrap"))
        {
            evidence.status = BootstrapEntryEvidenceState::Unknown;
            return evidence;
        }
        let (Some(record), Some(observed), Some(bounds)) =
            (markers.entry, self.observed_ns, self.bounds)
        else {
            if self.stderr_eof || reaped {
                evidence.status = BootstrapEntryEvidenceState::Unknown;
            }
            return evidence;
        };
        if bounds.clock != BootstrapEntryClock::ClockGettimeMonotonic
            || bounds.before_spawn_ns > bounds.return_lower_ns
            || bounds.return_lower_ns > bounds.return_upper_ns
            || record.e < bounds.before_spawn_ns
            || record.e > record.m
            || record.m > observed
            || bounds.return_upper_ns > observed
        {
            evidence.status = BootstrapEntryEvidenceState::Unknown;
            return evidence;
        }
        if !markers.waiting {
            if self.stderr_eof || reaped {
                evidence.status = BootstrapEntryEvidenceState::Unknown;
            }
            return evidence;
        }
        evidence.status = BootstrapEntryEvidenceState::Known;
        evidence.return_clock_uncertainty_ns =
            Some(bounds.return_upper_ns - bounds.return_lower_ns);
        evidence.signed_spawn_return_to_entry_ns = Some([
            i128::from(record.e) - i128::from(bounds.return_upper_ns),
            i128::from(record.e) - i128::from(bounds.return_lower_ns),
        ]);
        evidence.recognition_ns = Some(record.m - record.e);
        evidence.write_observation_upper_ns = Some(observed - record.m);
        evidence
    }
}
fn capture_stderr(
    stderr: &mut File,
    capture: &mut Vec<u8>,
    entry: &mut BootstrapEntryTracking,
) -> Result<(), Code> {
    let mut bytes = [0; 256];
    match stderr.read(&mut bytes) {
        Ok(n) => {
            let observed = (n > 0).then(bootstrap::monotonic_ns);
            if capture.len() + n > 16 * 1024 {
                return Err(Code::ResourceLimit);
            }
            capture.extend_from_slice(&bytes[..n]);
            if let Some(observed) = observed {
                entry.observe(capture, observed);
            }
            if n == 0 {
                entry.stderr_eof = true;
            }
            Ok(())
        }
        Err(e)
            if matches!(
                e.kind(),
                std::io::ErrorKind::WouldBlock | std::io::ErrorKind::Interrupted
            ) =>
        {
            Ok(())
        }
        Err(_) => Err(Code::WorkerLost),
    }
}
fn parse_bootstrap_markers(bytes: &[u8]) -> Result<BootstrapMarkers, Code> {
    if bytes.len() > 16 * 1024 {
        return Err(Code::ResourceLimit);
    }
    let text = std::str::from_utf8(bytes).map_err(|_| Code::ProtocolError)?;
    let mut markers = BootstrapMarkers::default();
    for line in text.split_inclusive('\n') {
        if line.starts_with("dream-bootstrap entry") && line.len() > 256 {
            return Err(Code::ResourceLimit);
        }
        if !line.ends_with('\n') {
            if line.starts_with("dream-bootstrap entry") && markers.entry.is_some() {
                return Err(Code::ProtocolError);
            }
            continue;
        }
        let Some(fields) = line
            .trim_end_matches(['\n', '\r'])
            .strip_prefix("dream-bootstrap ")
        else {
            continue;
        };
        if fields.starts_with("entry") {
            if markers.entry.is_some() || markers.waiting || markers.paid_hello {
                return Err(Code::ProtocolError);
            }
            markers.entry = Some(parse_bootstrap_entry(line.as_bytes())?);
            continue;
        }
        if line.len() > 256 {
            return Err(Code::ResourceLimit);
        }
        let (state, fields) = fields.split_once(' ').ok_or(Code::ProtocolError)?;
        let (profile, fields) = fields.split_once(' ').ok_or(Code::ProtocolError)?;
        if profile != "profile=dream-bootstrap/1" {
            return Err(Code::VersionMismatch);
        }
        let (target, expected): (&mut [u64], usize) = match state {
            "state=waiting"
                if markers.entry.is_some() && !markers.waiting && !markers.paid_hello =>
            {
                markers.waiting = true;
                (&mut markers.waiting_admissions, 3)
            }
            "state=paid_hello" if markers.waiting && !markers.paid_hello => {
                markers.paid_hello = true;
                (&mut markers.paid_admissions, 7)
            }
            _ => return Err(Code::ProtocolError),
        };
        let mut seen = 0u8;
        for field in fields.split_ascii_whitespace() {
            let (key, value) = field.split_once('=').ok_or(Code::ProtocolError)?;
            let i = [
                "argv_admissions",
                "channel_admissions",
                "owner_admissions",
                "frame_admissions",
                "permit_reads",
                "control_cpu_ns",
                "control_max_cpu_ns",
            ]
            .iter()
            .position(|name| *name == key)
            .filter(|i| *i < expected)
            .ok_or(Code::ProtocolError)?;
            if seen & (1 << i) != 0 {
                return Err(Code::ProtocolError);
            }
            seen |= 1 << i;
            let value = metric_number(value)
                .filter(|v| *v <= wire::MAX_SAFE)
                .ok_or(Code::ProtocolError)?;
            match i {
                0..=3 => target[i] = value,
                4 => markers.permit_reads = value,
                5 => markers.control_cpu_ns = value,
                _ => markers.control_max_cpu_ns = value,
            }
        }
        if seen != (1 << expected) - 1 {
            return Err(Code::ProtocolError);
        }
    }
    Ok(markers)
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
    // Adapted from bboxy450-harness src/dream/c1_wire.rs at f848c03f:
    // https://github.com/bboxy450/bboxy450-harness/blob/f848c03ff0462e55dfa0b17fcfd550626a52db2b/src/dream/c1_wire.rs
    // Keep producer canonical bytes stable under ACP's preserve_order feature.
    // Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
    let mut value = serde_json::to_value(value)?;
    value.sort_all_objects();
    serde_json::to_vec(&value)
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
/// Fixed Start frame and its write observation share the existing control reserve.
pub const START_CONTROL_BYTES: usize = 640;
/// A Start is below POSIX's minimum atomic pipe-write size. Encoding and scalar
/// validation happen during ordinary work, before the final paid handoff.
pub struct PreparedStart {
    bytes: [u8; 512],
    len: usize,
    run_id: [u8; 16],
    nonce: [u8; 16],
}
impl PreparedStart {
    pub fn new(start: &wire::Start) -> Result<Self, Code> {
        Self::from_parts(
            &start.run_id.0,
            &start.nonce.0,
            &start.manifest_sha256.0,
            start.timeout_ms,
            start.activity_revision.0,
            start.foreground,
        )
    }
    /// Borrow the retained original scalar fields; preparing a fixed frame
    /// needs no intermediate owned Start or String clones.
    pub fn from_parts(
        run: &str,
        nonce: &str,
        manifest: &str,
        timeout_ms: u32,
        activity_revision: u64,
        foreground: bool,
    ) -> Result<Self, Code> {
        let run_id = bootstrap::ulid(run.as_bytes()).map_err(|_| Code::ProtocolError)?;
        let nonce_id = bootstrap::ulid(nonce.as_bytes()).map_err(|_| Code::ProtocolError)?;
        if manifest.len() != 64
            || !manifest
                .bytes()
                .all(|v| v.is_ascii_digit() || (b'a'..=b'f').contains(&v))
            || timeout_ms == 0
            || timeout_ms > 30_000
            || activity_revision > wire::MAX_SAFE
        {
            return Err(Code::ResourceLimit);
        }
        #[derive(serde::Serialize)]
        struct Envelope<'a> {
            // Producer canonical key order, without an intermediate JSON tree.
            activity_revision: u64,
            foreground: bool,
            manifest_sha256: &'a str,
            nonce: &'a str,
            protocol: wire::Protocol,
            run_id: &'a str,
            timeout_ms: u32,
            #[serde(rename = "type")]
            kind: &'static str,
        }
        let mut bytes = [0; 512];
        let len = {
            let mut cursor = std::io::Cursor::new(&mut bytes[..511]);
            serde_json::to_writer(
                &mut cursor,
                &Envelope {
                    kind: "start",
                    protocol: wire::Protocol::V1,
                    run_id: run,
                    nonce,
                    manifest_sha256: manifest,
                    timeout_ms,
                    activity_revision,
                    foreground,
                },
            )
            .map_err(|_| Code::ResourceLimit)?;
            cursor.position() as usize
        };
        preflight_json(&bytes[..len], JsonShape::WorkerIn)?;
        bytes[len] = b'\n';
        Ok(Self {
            bytes,
            len: len + 1,
            run_id,
            nonce: nonce_id,
        })
    }
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.len]
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StartWriteObservation {
    pub attempted: bool,
    pub bytes: usize,
    pub errno: i32,
}
const _: () = assert!(
    std::mem::size_of::<PreparedStart>() + std::mem::size_of::<StartWriteObservation>()
        <= START_CONTROL_BYTES
);
/// Fallibly allocated codec backing, owned independently of native spawn.
pub struct PreparedBuffers {
    pending: Vec<u8>,
    write: Vec<u8>,
    stderr_capture: Vec<u8>,
    frame_scratch: Vec<u8>,
}
/// One explicit pipe pair, created before native process admission.
pub struct PipePair {
    read: File,
    write: File,
}
/// A frozen command and parent pipe endpoints. No source/manifest access.
pub struct PreparedCommand {
    command: Option<Command>,
    input: File,
    input_nonblocking: Cell<bool>,
    output: File,
    stderr: File,
    buffers: PreparedBuffers,
    bootstrap: Gate,
    entry: BootstrapEntryTracking,
}
impl PreparedCommand {
    /// One bounded nonblocking control read; raw stderr never leaves its owner.
    pub fn poll_bootstrap_markers(&mut self) -> Result<BootstrapMarkers, Code> {
        capture_stderr(
            &mut self.stderr,
            &mut self.buffers.stderr_capture,
            &mut self.entry,
        )?;
        parse_bootstrap_markers(&self.buffers.stderr_capture)
    }
    pub fn configure_input(&self) -> Result<(), Code> {
        nonblocking(&self.input)?;
        self.input_nonblocking.set(true);
        Ok(())
    }
    pub fn configure_output(&self) -> Result<(), Code> {
        nonblocking(&self.output)
    }
    pub fn configure_stderr(&self) -> Result<(), Code> {
        nonblocking(&self.stderr)
    }
    /// One original attempt; taking this command again refuses before effects.
    pub fn take_command(&mut self) -> Result<Command, Code> {
        self.command.take().ok_or(Code::BadRequest)
    }
    /// Capture a returned child before any timing/cancel verdict. Only moves
    /// ownership; taking handles/fcntl and Command retirement are separate work.
    pub fn attach_child(self, child: Child) -> (Worker, Option<Command>) {
        let PreparedCommand {
            command,
            input,
            input_nonblocking,
            output,
            stderr,
            buffers,
            bootstrap,
            entry,
        } = self;
        let PreparedBuffers {
            pending,
            write,
            stderr_capture,
            frame_scratch,
        } = buffers;
        let stderr_bytes = stderr_capture.len();
        (
            Worker {
                child,
                input: Some(input),
                input_nonblocking,
                output,
                stderr,
                pending,
                write,
                written: 0,
                start_write: StartWriteObservation::default(),
                stderr_bytes,
                stderr_capture,
                frame_scratch,
                eof: false,
                stderr_eof: false,
                reaped: None,
                usage: None,
                bootstrap,
                entry,
            },
            command,
        )
    }
}

fn validate_runner(binary: &Path, job: &Path) -> Result<(), Code> {
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
    Ok(())
}

/// An exact child/group with bounded nonblocking pipes. Never pass this to a hot owner.
pub struct Worker {
    child: Child,
    input: Option<File>,
    input_nonblocking: Cell<bool>,
    output: File,
    stderr: File,
    pending: Vec<u8>,
    write: Vec<u8>,
    written: usize,
    start_write: StartWriteObservation,
    stderr_bytes: usize,
    stderr_capture: Vec<u8>,
    /// Preprovisioned protected codec scratch, reused for every frame without growth.
    frame_scratch: Vec<u8>,
    eof: bool,
    stderr_eof: bool,
    reaped: Option<ExitStatus>,
    usage: Option<ChildUsage>,
    bootstrap: Gate,
    entry: BootstrapEntryTracking,
}

impl Worker {
    pub fn prepare_buffers() -> Result<PreparedBuffers, Code> {
        crate::assert_not_ui_thread();
        Ok(PreparedBuffers {
            pending: frame_buffer(wire::MAX_FRAME)?,
            write: frame_buffer(wire::MAX_FRAME)?,
            stderr_capture: frame_buffer(16 * 1024)?,
            frame_scratch: frame_buffer(
                PROTECTED_BYTES
                    - 2 * wire::MAX_FRAME
                    - 16 * 1024
                    - bootstrap::BOOTSTRAP_CONTROL_BYTES
                    - START_CONTROL_BYTES
                    - ENTRY_CONTROL_BYTES,
            )?,
        })
    }
    pub fn prepare_pipe() -> Result<PipePair, Code> {
        crate::assert_not_ui_thread();
        let (read, write) = std::io::pipe().map_err(|_| Code::RunnerUnavailable)?;
        let read = File::from(OwnedFd::from(read));
        let write = File::from(OwnedFd::from(write));
        Ok(PipePair { read, write })
    }
    pub fn prepare_command(
        binary: &Path,
        job: &Path,
        buffers: PreparedBuffers,
        input: PipePair,
        output: PipePair,
        stderr: PipePair,
        bootstrap: Gate,
    ) -> Result<PreparedCommand, Code> {
        crate::assert_not_ui_thread();
        validate_runner(binary, job)?;
        if bootstrap.frozen().purpose != bootstrap::Purpose::InspectC1 {
            return Err(Code::ProtocolError);
        }
        let bootstrap_environment = bootstrap.env_value()?;
        let bootstrap_fd = bootstrap.inherited_fd();
        let mut command = Command::new(binary);
        command
            .args(["dream", "inspect-c1", "--job-dir"])
            .arg(job)
            .current_dir(job)
            .env_clear()
            .env("LANG", "C.UTF-8")
            .env("TMPDIR", job.join("tmp"))
            .env(bootstrap::ENV, bootstrap_environment.as_str())
            .stdin(Stdio::from(input.read))
            .stdout(Stdio::from(output.write))
            .stderr(Stdio::from(stderr.write));
        // SAFETY: child-only async-signal-safe calls; no allocation or inherited lock.
        unsafe {
            command.pre_exec(move || {
                let flags = libc::fcntl(bootstrap_fd, libc::F_GETFD);
                if flags < 0
                    || libc::fcntl(bootstrap_fd, libc::F_SETFD, flags & !libc::FD_CLOEXEC) != 0
                {
                    return Err(std::io::Error::last_os_error());
                }
                if libc::setpgid(0, 0) != 0 || libc::setpriority(libc::PRIO_PROCESS, 0, 10) != 0 {
                    return Err(std::io::Error::last_os_error());
                }
                if libc::getpriority(libc::PRIO_PROCESS, 0) < 10 {
                    return Err(std::io::Error::from_raw_os_error(libc::EPERM));
                }
                Ok(())
            });
        }
        Ok(PreparedCommand {
            command: Some(command),
            input: input.write,
            input_nonblocking: Cell::new(false),
            output: output.read,
            stderr: stderr.read,
            buffers,
            bootstrap,
            entry: BootstrapEntryTracking::default(),
        })
    }
    /// Spawn the provisioned absolute sibling directly, without inherited authority.
    pub fn spawn(_binary: &Path, _job: &Path) -> Result<Self, Code> {
        // No ungated capability detection/compatibility path may execute a
        // candidate. Bootstrap support must be approved before its birth.
        Err(Code::RunnerUnavailable)
    }
    pub fn spawn_prepared(
        _binary: &Path,
        _job: &Path,
        _buffers: PreparedBuffers,
    ) -> Result<Self, Code> {
        Err(Code::RunnerUnavailable)
    }

    pub fn configure_input(&self) -> Result<(), Code> {
        nonblocking(self.input.as_ref().ok_or(Code::RunnerUnavailable)?)?;
        self.input_nonblocking.set(true);
        Ok(())
    }
    pub fn configure_output(&self) -> Result<(), Code> {
        nonblocking(&self.output)
    }
    pub fn configure_stderr(&self) -> Result<(), Code> {
        nonblocking(&self.stderr)
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
            + bootstrap::BOOTSTRAP_CONTROL_BYTES
            + START_CONTROL_BYTES
            + ENTRY_CONTROL_BYTES
    }
    pub fn bootstrap_identity(&self) -> (u32, u64, u64) {
        self.bootstrap.identity()
    }
    pub fn bootstrap_markers(&self) -> Result<BootstrapMarkers, Code> {
        parse_bootstrap_markers(&self.stderr_capture)
    }
    /// Refresh only the bounded private stderr cache. Stdout and any queued
    /// Hello remain untouched; incomplete diagnostic lines remain unobserved.
    pub fn poll_bootstrap_markers(&mut self) -> Result<BootstrapMarkers, Code> {
        capture_stderr(&mut self.stderr, &mut self.stderr_capture, &mut self.entry)?;
        self.stderr_bytes = self.stderr_capture.len();
        self.stderr_eof |= self.entry.stderr_eof;
        self.bootstrap_markers()
    }
    pub fn install_bootstrap_entry_clock_bounds(
        &mut self,
        bounds: BootstrapEntryClockBounds,
    ) -> Result<(), Code> {
        if self.entry.bounds.is_some() {
            self.entry.invalid = true;
            return Err(Code::ProtocolError);
        }
        let bounds = BootstrapEntryClockBounds::new(
            bounds.clock,
            bounds.before_spawn_ns,
            bounds.return_lower_ns,
            bounds.return_upper_ns,
        )
        .inspect_err(|_| self.entry.invalid = true)?;
        self.entry.bounds = Some(bounds);
        Ok(())
    }
    pub fn bootstrap_entry_evidence(&self) -> BootstrapEntryEvidence {
        self.entry
            .evidence(&self.stderr_capture, self.reaped.is_some())
    }
    pub fn bootstrap_frozen(&self) -> bootstrap::Frozen {
        self.bootstrap.frozen()
    }
    pub fn bootstrap_observation(&self) -> bootstrap::Observation {
        self.bootstrap.observation()
    }
    pub fn bootstrap_spawn_returned(
        &mut self,
        returned_at: Instant,
        job_deadline: Instant,
    ) -> Result<(), Code> {
        self.bootstrap.spawn_returned(returned_at, job_deadline)
    }
    pub fn bootstrap_grant(&mut self) -> Result<(), Code> {
        self.bootstrap.grant()
    }
    pub fn bootstrap_revoke(&mut self) {
        self.bootstrap.revoke();
    }
    pub fn bootstrap_deadline(&self) -> Result<Instant, Code> {
        self.bootstrap.readiness_deadline()
    }
    pub fn hello_step(&mut self) -> Result<Option<wire::Hello>, Code> {
        let result = (|| {
            self.bootstrap.check_readiness().map_err(|code| {
                if code == Code::Deadline {
                    Code::RunnerUnavailable
                } else {
                    code
                }
            })?;
            match self.receive()? {
                Some(WorkerOut::Hello(h))
                    if h.schema_sha256.0 == SCHEMA_SHA256
                        && h.ir_major == 1
                        && h.ir_minor_max == 3 =>
                {
                    self.bootstrap.qualify_hello()?;
                    Ok(Some(h))
                }
                Some(_) => Err(Code::VersionMismatch),
                None if self.eof => Err(Code::RunnerUnavailable),
                None => Ok(None),
            }
        })();
        if result.is_err() {
            self.bootstrap.revoke();
        }
        result
    }
    /// Actual C1 readiness/probing requires entry+waiting and same-clock bounds.
    /// Protocol-only fake peers use hello_step; that method is NOT entry proof.
    /// Stdout can arrive before the stderr read: retain it in the existing
    /// bounded byte buffer without accepting/discarding its queued Hello.
    pub fn hello_entry_step(&mut self) -> Result<Option<wire::Hello>, Code> {
        let result = (|| {
            self.bootstrap
                .check_readiness()
                .map_err(|_| Code::RunnerUnavailable)?;
            self.poll_bootstrap_markers()
                .map_err(|_| Code::RunnerUnavailable)?;
            match self.bootstrap_entry_evidence().status {
                BootstrapEntryEvidenceState::Unknown => Err(Code::RunnerUnavailable),
                BootstrapEntryEvidenceState::Pending => {
                    self.buffer_stdout()?;
                    Ok(None)
                }
                BootstrapEntryEvidenceState::Known => self.hello_step(),
            }
        })();
        if result.is_err() {
            self.bootstrap.revoke();
        }
        result
    }
    pub fn is_eof(&self) -> bool {
        self.eof && self.stderr_eof
    }
    /// Failed/cancelled runs grant pending frames no meaning. Clearing their
    /// bounded prefix lets EOF drain even after a framing/flood error.
    pub fn discard_pending_output(&mut self) {
        self.pending.clear();
        self.frame_scratch.clear();
    }

    /// Hello precedes every manifest/source read, including launch capability probing.
    pub fn hello(&mut self) -> Result<wire::Hello, Code> {
        loop {
            if let Some(hello) = self.hello_step()? {
                return Ok(hello);
            }
            self.wait_io(
                self.bootstrap_deadline()?
                    .saturating_duration_since(Instant::now())
                    .min(Duration::from_millis(9)),
            );
        }
    }

    /// At most one pending frame; callers coalesce Activity before numbering.
    pub fn send(&mut self, frame: &WorkerIn) -> Result<bool, Code> {
        if let WorkerIn::Start(start) = frame {
            let prepared = PreparedStart::new(start)?;
            return self.send_prepared_start(&prepared).map(|()| true);
        }
        let (run, nonce) = match frame {
            WorkerIn::Start(v) => (&v.run_id, &v.nonce),
            WorkerIn::Activity(v) => (&v.run_id, &v.nonce),
            WorkerIn::Cancel(v) => (&v.run_id, &v.nonce),
        };
        let frozen = self.bootstrap.frozen();
        if !self.input_nonblocking.get()
            || bootstrap::ulid(run.0.as_bytes()).ok() != Some(frozen.run_id)
            || bootstrap::ulid(nonce.0.as_bytes()).ok() != Some(frozen.nonce)
        {
            return Err(Code::ProtocolError);
        }
        if let WorkerIn::Activity(v) = frame
            && v.revision.0 > wire::MAX_SAFE
        {
            return Err(Code::ResourceLimit);
        }
        if !self.flush()? {
            return Ok(false);
        }
        self.write.clear();
        // Fixed control framing only while the child owns work. The validated
        // IDs are canonical ASCII, numbers have the wire's53-bit bound, and the
        // sole boolean has a fixed spelling. No owned JSON graph, generic
        // payload serialization or parser runs on this independent control lane.
        {
            let mut output = FrameWriter(&mut self.write);
            match frame {
                WorkerIn::Activity(v) => write!(output,
                    r#"{{"type":"activity","protocol":"bboxy.dream.c1/1","run_id":"{}","nonce":"{}","revision":{},"foreground":{}}}"#,
                    v.run_id.0, v.nonce.0, v.revision.0, v.foreground),
                WorkerIn::Cancel(v) => write!(output,
                    r#"{{"type":"cancel","protocol":"bboxy.dream.c1/1","run_id":"{}","nonce":"{}"}}"#,
                    v.run_id.0, v.nonce.0),
                WorkerIn::Start(_) => return Err(Code::ProtocolError),
            }.map_err(|_| Code::ResourceLimit)?;
        }
        self.write.push(b'\n');
        if self.write.len() > 512 {
            return Err(Code::ResourceLimit);
        }
        self.written = 0;
        self.flush()
    }

    /// Final Start admission performs no encoding, parsing, allocation or queued
    /// retry. A <=512B nonblocking pipe write is atomic; every refusal retires
    /// stdin so a later Cancel/Activity flush cannot complete a rejected Start.
    pub fn send_prepared_start(&mut self, frame: &PreparedStart) -> Result<(), Code> {
        let frozen = self.bootstrap.frozen();
        if !self.input_nonblocking.get()
            || self.start_write.attempted
            || !self.bootstrap.observation().qualified_hello
            || frozen.probe
            || frame.run_id != frozen.run_id
            || frame.nonce != frozen.nonce
            || !self.write.is_empty()
            || self.written != 0
        {
            self.close_input();
            return Err(Code::ProtocolError);
        }
        let Some(input) = self.input.as_ref() else {
            return Err(Code::WorkerLost);
        };
        self.start_write.attempted = true;
        // SAFETY: the fixed prepared bytes and this owned nonblocking pipe
        // remain live through one syscall. Capture errno before other effects.
        let written =
            unsafe { libc::write(input.as_raw_fd(), frame.bytes.as_ptr().cast(), frame.len) };
        let errno = if written < 0 {
            std::io::Error::last_os_error().raw_os_error().unwrap_or(0)
        } else {
            0
        };
        self.start_write.bytes = if written > 0 { written as usize } else { 0 };
        self.start_write.errno = errno;
        if written != frame.len as isize {
            self.close_input();
            return Err(Code::WorkerLost);
        }
        Ok(())
    }

    pub fn start_write_observation(&self) -> StartWriteObservation {
        self.start_write
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

    /// Preserve an early Hello until its independent stderr evidence arrives.
    /// Read at most 16 KiB, within the already reserved frame capacity.
    fn buffer_stdout(&mut self) -> Result<(), Code> {
        if self.pending.contains(&b'\n') {
            return Ok(());
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
        Ok(())
    }

    /// Read no more than 16 KiB stdout and 256 B stderr per call. Every stderr
    /// read uses the same entry tracker, including reads after Hello.
    pub fn receive(&mut self) -> Result<Option<WorkerOut>, Code> {
        self.buffer_stdout()?;
        capture_stderr(&mut self.stderr, &mut self.stderr_capture, &mut self.entry)?;
        self.stderr_bytes = self.stderr_capture.len();
        self.stderr_eof |= self.entry.stderr_eof;
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
        parse_producer_metrics(&self.stderr_capture)
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
        self.bootstrap.revoke();
        self.input.take();
    }

    /// Only an unreaped child group created by this owner can receive escalation.
    pub fn terminate(&mut self, kill: bool) -> Result<(), Code> {
        self.bootstrap.revoke();
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

#[cfg(test)]
mod entry_tests {
    use super::*;

    const ENTRY: &str = "dream-bootstrap entry s=7 g=8 e=110 m=125 u=9 k=10 r=11 p=12 ok=1\n";
    const WAITING: &str = "dream-bootstrap state=waiting profile=dream-bootstrap/1 argv_admissions=0 channel_admissions=0 owner_admissions=0\n";

    fn tracking() -> BootstrapEntryTracking {
        BootstrapEntryTracking {
            bounds: Some(
                BootstrapEntryClockBounds::new(
                    BootstrapEntryClock::ClockGettimeMonotonic,
                    100,
                    115,
                    120,
                )
                .unwrap(),
            ),
            ..Default::default()
        }
    }

    #[test]
    fn entry_preserves_signed_return_uncertainty_and_raw_counters() {
        let bytes = format!("{ENTRY}{WAITING}");
        let mut tracker = tracking();
        tracker.observe(bytes.as_bytes(), Ok(130));
        let evidence = tracker.evidence(bytes.as_bytes(), false);
        assert_eq!(evidence.status, BootstrapEntryEvidenceState::Known);
        assert_eq!(evidence.signed_spawn_return_to_entry_ns, Some([-10, -5]));
        assert_eq!(evidence.return_clock_uncertainty_ns, Some(5));
        assert_eq!(evidence.recognition_ns, Some(15));
        assert_eq!(evidence.write_observation_upper_ns, Some(5));
        assert_eq!(
            evidence.record.unwrap(),
            parse_bootstrap_entry(ENTRY.as_bytes()).unwrap()
        );
        let maximum = u64::MAX;
        let max_line = format!(
            "dream-bootstrap entry s={maximum} g={maximum} e={maximum} m={maximum} u={maximum} k={maximum} r={maximum} p={maximum} ok=1\n"
        );
        assert!(max_line.len() <= 256);
        assert_eq!(
            parse_bootstrap_entry(max_line.as_bytes()).unwrap().p,
            maximum
        );
        let failed = parse_bootstrap_entry(
            b"dream-bootstrap entry s=0 g=8 e=110 m=125 u=0 k=0 r=0 p=0 ok=0\n",
        )
        .unwrap();
        assert!(!failed.ok);
        assert_eq!(failed.g, 8); // Raw counter, never a derived zero cost.
    }

    #[test]
    fn entry_parser_refuses_malformed_decimal_order_bounds_and_usage_failure() {
        for bad in [
            ENTRY.trim_end().to_owned(),
            ENTRY.replace('\n', "\r\n"),
            ENTRY.replace("s=7 g=8", "g=8 s=7"),
            ENTRY.replace("s=7", "s=07"),
            ENTRY.replace("g=8", "g=8 g=9"),
            ENTRY.replace("s=7", "s=+7"),
            ENTRY.replace("s=7", "s=18446744073709551616"),
            ENTRY.replace("s=7", "s=x"),
            ENTRY.replace("ok=1", "ok=2"),
            ENTRY.replace("e=110", "e=126"),
            ENTRY.replace("ok=1", "ok=0"),
            format!("{ENTRY}{}", " ".repeat(256)),
        ] {
            assert!(parse_bootstrap_entry(bad.as_bytes()).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn entry_missing_duplicate_out_of_order_partial_and_reversed_clocks_are_unknown() {
        for bad in [
            String::new(),
            WAITING.to_owned(),
            ENTRY.to_owned(),
            format!("{ENTRY}{ENTRY}{WAITING}"),
            format!("{WAITING}{ENTRY}"),
            format!("{ENTRY}{WAITING}{ENTRY}"),
            format!("{ENTRY}{WAITING}dream-bootstrap entry s="),
        ] {
            let mut tracker = tracking();
            tracker.observe(bad.as_bytes(), Ok(130));
            assert_ne!(
                tracker.evidence(bad.as_bytes(), false).status,
                BootstrapEntryEvidenceState::Known,
                "{bad:?}"
            );
            assert_eq!(
                tracker.evidence(bad.as_bytes(), true).status,
                BootstrapEntryEvidenceState::Unknown,
                "{bad:?}"
            );
        }
        let bytes = format!("{ENTRY}{WAITING}");
        for clock in [Ok(124), Err(bootstrap::Error::Clock)] {
            let mut tracker = tracking();
            tracker.observe(bytes.as_bytes(), clock);
            assert_eq!(
                tracker.evidence(bytes.as_bytes(), false).status,
                BootstrapEntryEvidenceState::Unknown
            );
        }
        let mut tracker = tracking();
        tracker.observe(bytes.as_bytes(), Ok(130));
        tracker.observe(bytes.as_bytes(), Ok(129));
        assert_eq!(
            tracker.evidence(bytes.as_bytes(), false).status,
            BootstrapEntryEvidenceState::Unknown
        );
        assert!(
            BootstrapEntryClockBounds::new(BootstrapEntryClock::Unknown, 100, 115, 120).is_err()
        );
        assert!(
            BootstrapEntryClockBounds::new(
                BootstrapEntryClock::ClockGettimeMonotonic,
                100,
                120,
                115
            )
            .is_err()
        );
    }

    #[test]
    fn entry_fragmented_pipe_observation_keeps_first_complete_line_timestamp() {
        let (read, mut write) = std::io::pipe().unwrap();
        let mut read = File::from(OwnedFd::from(read));
        nonblocking(&read).unwrap();
        let mut captured = Vec::with_capacity(16 * 1024);
        let mut tracker = BootstrapEntryTracking::default();
        write.write_all(&ENTRY.as_bytes()[..20]).unwrap();
        capture_stderr(&mut read, &mut captured, &mut tracker).unwrap();
        assert!(tracker.observed_ns.is_none());
        write.write_all(&ENTRY.as_bytes()[20..]).unwrap();
        capture_stderr(&mut read, &mut captured, &mut tracker).unwrap();
        let first = tracker.observed_ns.unwrap();
        write.write_all(WAITING.as_bytes()).unwrap();
        capture_stderr(&mut read, &mut captured, &mut tracker).unwrap();
        assert_eq!(tracker.observed_ns, Some(first));
        assert_eq!(
            tracker.record,
            Some(parse_bootstrap_entry(ENTRY.as_bytes()).unwrap())
        );
        assert!(parse_bootstrap_markers(&captured).unwrap().waiting);
        drop(write);
        capture_stderr(&mut read, &mut captured, &mut tracker).unwrap();
        assert!(tracker.stderr_eof);
    }
}

#[cfg(test)]
mod diagnostic_tests {
    use super::*;

    const COMPUTE: &str = "dream-c1 metrics heap_live_bytes=12 heap_peak_bytes=24 max_unit_cpu_ns=50 cleanup_cpu_ns=60 max_cleanup_unit_cpu_ns=70 native_cumulative_bytes=unmeasured\n";
    // Grammar/limit witness for the producer-owned private ABI; this is not a
    // canonical worker fixture or a real-process conformance claim.
    const PROFILE: &str = concat!(
        "dream-c1 metrics heap_live_bytes=1024 heap_peak_bytes=8388608 startup_heap_peak_bytes=2097152 compute_stack_reserved_bytes=2097152 rss_peak_bytes=12582912 compute_cpu_ns=8000000 max_unit_cpu_ns=2000000 overruns=0 envelopes=71 bytes=7359851 storage_declared_live_bytes=0 storage_reserved_peak_bytes=4096 storage_cumulative_credits_bytes=8192 control_reserved_bytes=2097152 helper_reserved_bytes=16777216 credit_ceiling_bytes=100663296 native_cumulative_bytes=unmeasured normalization_unit_0_cpu_ns=100 normalization_unit_1_cpu_ns=100 normalization_unit_2_cpu_ns=100 normalization_unit_3_cpu_ns=100 cleanup_cpu_ns=500 max_cleanup_unit_cpu_ns=100\n",
        "dream-c1 native tag=compute_owner_create count=1 cpu_ns=1200000 wall_ns=2000000 max_cpu_ns=1200000 max_wall_ns=2000000 inclusive_max_cpu_ns=1300000 inclusive_max_wall_ns=2100000 app_max_cpu_ns=100000 effects=1 failures_with_effect=0\n",
        "dream-c1 native tag=publish_noreplace count=2 cpu_ns=2800000 wall_ns=4000000 max_cpu_ns=1500000 max_wall_ns=2000000 inclusive_max_cpu_ns=2000000 inclusive_max_wall_ns=2500000 app_max_cpu_ns=500000 effects=2 failures_with_effect=0\n",
        "dream-c1 cost_known=true\n",
        "dream-c1 control_parts input_max_cpu_ns=100 protocol_max_cpu_ns=100 settlement_max_cpu_ns=100 output_max_cpu_ns=100 total_cpu_ns=1000000\n",
        "dream-c1 terminal_metrics profile=c1-native-debt/1 cutoff=joined_final_control_pre_telemetry captured_max_cpu_ns=1500000 inclusive_max_cpu_ns=2000000 app_max_cpu_ns=500000 control_max_cpu_ns=100000 inclusive_max_wall_ns=2500000 span_cpu_ns=10000000 owner_thread_cpu_ns=8500000 process_user_cpu_ns=10000000 process_system_cpu_ns=2000000 wall_ns=1000000000 debt_total_ns=900000000 rest_observed_ns=900000000 remaining_debt_ns=0\n",
        "dream-c1 publication_custody run_id=01ARZ3NDEKTSV4RRFFQ69G5FAV nonce=01ARZ3NDEKTSV4RRFFQ69G5FAW operation=4096 dev=1 ino=2 effect=1 errno=0 cleanup_uncertain=false path_mapping=page_index_or_4096_result temp_mapping=dot_name_original_Start_nonce_tmp\n",
        "dream-c1 control_max_cpu_ns=100000\n",
        "dream-c1 reactor_max_cpu_ns=100000 startup_runtime=none\n",
    );
    const RUN: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
    const NONCE: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAW";

    #[test]
    fn bootstrap_numeric_markers_require_all_four_paid_admissions_and_complete_unique_fields() {
        // Numeric receipt parser cases, not an alternative permit/vector corpus.
        // Adapted from signed harness bab543e2116d src/dream/bootstrap.rs::Gate::retire.
        const WAITING: &str = "dream-bootstrap state=waiting profile=dream-bootstrap/1 argv_admissions=0 channel_admissions=0 owner_admissions=0\n";
        const PAID: &str = "dream-bootstrap state=paid_hello profile=dream-bootstrap/1 argv_admissions=1 channel_admissions=1 owner_admissions=1 frame_admissions=1 permit_reads=1 control_cpu_ns=12000 control_max_cpu_ns=7000\n";
        const ENTRY: &str = "dream-bootstrap entry s=7 g=8 e=110 m=125 u=9 k=10 r=11 p=12 ok=1\n";
        let receipt = format!("{ENTRY}{WAITING}{PAID}");
        let markers = parse_bootstrap_markers(receipt.as_bytes()).unwrap();
        assert!(markers.waiting && markers.paid_hello);
        assert_eq!(markers.waiting_admissions, [0; 3]);
        assert_eq!(markers.paid_admissions, [1; 4]);
        assert_eq!(markers.permit_reads, 1);
        assert_eq!(markers.control_cpu_ns, 12000);
        assert_eq!(markers.control_max_cpu_ns, 7000);
        for bad in [
            PAID.replace("frame_admissions=1 ", ""),
            PAID.replace(
                "frame_admissions=1",
                "frame_admissions=1 frame_admissions=1",
            ),
            PAID.replace("frame_admissions=1", "unknown_admissions=1"),
            PAID.replace(
                "control_cpu_ns=12000",
                "control_cpu_ns=18446744073709551615",
            ),
            WAITING.replace(
                "owner_admissions=0",
                "owner_admissions=0 frame_admissions=0",
            ),
            format!("{PAID}{PAID}"),
        ] {
            let receipt = format!("{ENTRY}{WAITING}{bad}");
            assert!(
                parse_bootstrap_markers(receipt.as_bytes()).is_err(),
                "{receipt}"
            );
        }
        let partial = receipt.trim_end_matches('\n');
        let markers = parse_bootstrap_markers(partial.as_bytes()).unwrap();
        assert!(markers.waiting && !markers.paid_hello);
    }

    #[test]
    fn native_profile_requires_disaggregated_bounds_debt_identity_and_capture_cutoff() {
        let metrics = parse_producer_metrics(PROFILE.as_bytes()).unwrap();
        assert!(metrics.has_native_profile());
        assert!(metrics.work_debt_paid_for(RUN, NONCE));
        assert!(metrics.valid_native_completion(RUN, NONCE, 1));
        assert!(!metrics.valid_native_completion(NONCE, NONCE, 1));
        assert!(!metrics.valid_native_completion(RUN, RUN, 1));
        assert!(!metrics.valid_native_completion(RUN, NONCE, 2));
        assert!(metrics.validates_descriptor_capture(1_500_000));
        assert!(!metrics.validates_descriptor_capture(1_400_000));
        assert!(!metrics.validates_descriptor_capture(3_000_001));
        assert_eq!(metrics.values[21], Some(100_000));
        assert_eq!(metrics.values[22], Some(100_000));
        assert_eq!(metrics.values[23], Some(2 * 1024 * 1024));
        assert_eq!(metrics.values[68], Some(0));
    }

    #[test]
    fn native_profile_refuses_missing_or_false_proof_and_ordinary_overruns() {
        for (from, to) in [
            ("cost_known=true", "cost_known=false"),
            ("remaining_debt_ns=0", "remaining_debt_ns=1"),
            (
                "process_user_cpu_ns=10000000",
                "process_user_cpu_ns=18446744073709551615",
            ),
            (
                "owner_thread_cpu_ns=8500000",
                "owner_thread_cpu_ns=9007199254740992",
            ),
            ("captured_max_cpu_ns=1500000", "captured_max_cpu_ns=3000001"),
            (
                "inclusive_max_cpu_ns=2000000 app_max_cpu_ns=500000 control_max_cpu_ns=100000",
                "inclusive_max_cpu_ns=3000000 app_max_cpu_ns=500000 control_max_cpu_ns=100000",
            ),
            (
                "max_cleanup_unit_cpu_ns=100",
                "max_cleanup_unit_cpu_ns=1000001",
            ),
            ("protocol_max_cpu_ns=100", "protocol_max_cpu_ns=1000001"),
            ("control_max_cpu_ns=100000", "control_max_cpu_ns=1000001"),
            ("app_max_cpu_ns=500000", "app_max_cpu_ns=1000001"),
            (
                "tag=compute_owner_create count=1",
                "tag=compute_owner_create count=2",
            ),
            (
                "tag=publish_noreplace count=2",
                "tag=publish_noreplace count=4098",
            ),
            ("cleanup_uncertain=false", "cleanup_uncertain=true"),
            ("effect=1 errno=0", "effect=2 errno=5"),
        ] {
            let text = PROFILE.replace(from, to);
            let metrics = parse_producer_metrics(text.as_bytes()).unwrap();
            assert!(
                !metrics.valid_native_completion(RUN, NONCE, 1),
                "{from} -> {to}"
            );
        }
        let missing = PROFILE
            .lines()
            .filter(|line| !line.starts_with("dream-c1 control_parts "))
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        let metrics = parse_producer_metrics(missing.as_bytes()).unwrap();
        assert!(!metrics.has_native_profile());
        assert!(!metrics.valid_native_completion(RUN, NONCE, 1));
    }

    #[test]
    fn native_profile_rejects_incompatible_duplicate_unknown_or_truncated_records() {
        for (from, to) in [
            ("profile=c1-native-debt/1", "profile=c1-native-debt/2"),
            (
                "cutoff=joined_final_control_pre_telemetry",
                "cutoff=before_descriptor",
            ),
            ("tag=compute_owner_create", "tag=parser"),
            ("cost_known=true", "cost_known=1"),
            ("cleanup_uncertain=false", "cleanup_uncertain=0"),
            ("operation=4096", "operation=4096 unknown=1"),
        ] {
            assert!(parse_producer_metrics(PROFILE.replace(from, to).as_bytes()).is_none());
        }
        let duplicate = PROFILE.to_string() + "dream-c1 cost_known=true\n";
        assert!(parse_producer_metrics(duplicate.as_bytes()).is_none());
        assert!(parse_producer_metrics(PROFILE.trim_end().as_bytes()).is_none());
    }

    #[test]
    fn numeric_receipts_merge_unique_compute_reactor_and_control_without_reindexing() {
        let text = format!(
            "dream-c1 reactor_max_cpu_ns=80 startup_runtime=none\n{COMPUTE}dream-c1 control_max_cpu_ns=90\n"
        );
        let metrics = parse_producer_metrics(text.as_bytes()).unwrap();
        assert_eq!(PRODUCER_METRIC_KEYS[0], "heap_live_bytes");
        assert_eq!(PRODUCER_METRIC_KEYS[20], "max_cleanup_unit_cpu_ns");
        assert_eq!(metrics.values[0], Some(12));
        assert_eq!(metrics.values[20], Some(70));
        assert_eq!(metrics.get("reactor_max_cpu_ns"), Some(80));
        assert_eq!(metrics.get("control_max_cpu_ns"), Some(90));
        let alternate =
            format!("{COMPUTE}dream-c1 reactor_max_cpu_ns=81\ndream-c1 control_max_cpu_ns=91\n");
        assert_eq!(
            parse_producer_metrics(alternate.as_bytes())
                .unwrap()
                .get("reactor_max_cpu_ns"),
            Some(81)
        );
        let partial = parse_producer_metrics(COMPUTE.as_bytes()).unwrap();
        assert_eq!(partial.get("reactor_max_cpu_ns"), None);
        assert_eq!(partial.get("control_max_cpu_ns"), None);
    }

    #[test]
    fn duplicate_malformed_truncated_and_overlimit_numeric_receipts_are_unknown() {
        for suffix in [
            "dream-c1 reactor_max_cpu_ns=1\ndream-c1 reactor_max_cpu_ns=2\n",
            "dream-c1 control_max_cpu_ns=1\ndream-c1 control_max_cpu_ns=2\n",
            "dream-c1 reactor_max_cpu_ns=1 startup_runtime=some\n",
            "dream-c1 reactor_max_cpu_ns=1 startup_runtime=none extra=1\n",
            "dream-c1 reactor_max_cpu_ns=+1\n",
            "dream-c1 reactor_max_cpu_ns=-1\n",
            "dream-c1 reactor_max_cpu_ns=18446744073709551616\n",
            "dream-c1 control_max_cpu_ns=1.0\n",
            "dream-c1 control_max_cpu_ns=1 extra=2\n",
            "dream-c1 control_max_cpu_ns=1",
            COMPUTE,
        ] {
            let text = format!("{COMPUTE}{suffix}");
            assert!(
                parse_producer_metrics(text.as_bytes()).is_none(),
                "{suffix:?}"
            );
        }
        assert!(parse_producer_metrics(&[b'x'; 16 * 1024 + 1]).is_none());
        assert!(
            parse_producer_metrics(
                b"dream-c1 metrics heap_live_bytes=1 heap_live_bytes=1 heap_peak_bytes=2\n"
            )
            .is_none()
        );
    }

    #[test]
    fn codec_preparation_owns_exact_protected_capacity_without_spawning() {
        let buffers = Worker::prepare_buffers().unwrap();
        assert_eq!(buffers.pending.capacity(), wire::MAX_FRAME);
        assert_eq!(buffers.write.capacity(), wire::MAX_FRAME);
        assert_eq!(buffers.stderr_capture.capacity(), 16 * 1024);
        assert_eq!(
            buffers.frame_scratch.capacity()
                + buffers.pending.capacity()
                + buffers.write.capacity()
                + buffers.stderr_capture.capacity()
                + bootstrap::BOOTSTRAP_CONTROL_BYTES
                + START_CONTROL_BYTES
                + ENTRY_CONTROL_BYTES,
            PROTECTED_BYTES
        );
        assert!(buffers.frame_scratch.capacity() >= wire::MAX_FRAME);
    }

    #[test]
    fn command_and_three_cloexec_pipes_are_frozen_before_the_single_attempt() {
        let job = tempfile::tempdir().unwrap();
        let input = Worker::prepare_pipe().unwrap();
        let output = Worker::prepare_pipe().unwrap();
        let stderr = Worker::prepare_pipe().unwrap();
        for pair in [&input, &output, &stderr] {
            for file in [&pair.read, &pair.write] {
                // SAFETY: reads flags on the exact still-owned descriptor.
                let flags = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETFD) };
                assert!(flags >= 0 && flags & libc::FD_CLOEXEC != 0);
            }
        }
        let mut prepared = Worker::prepare_command(
            Path::new("/bin/cat"),
            job.path(),
            Worker::prepare_buffers().unwrap(),
            input,
            output,
            stderr,
            Gate::prepare(bootstrap::Frozen::inspect(
                false,
                ulid::Ulid::new().to_bytes(),
                ulid::Ulid::new().to_bytes(),
                ulid::Ulid::new().to_bytes(),
                [0; 32],
                1,
            ))
            .unwrap(),
        )
        .unwrap();
        let command = prepared.take_command().unwrap();
        assert_eq!(command.get_program(), "/bin/cat");
        assert_eq!(
            command.get_args().collect::<Vec<_>>(),
            [
                "dream".as_ref(),
                "inspect-c1".as_ref(),
                "--job-dir".as_ref(),
                job.path().as_os_str()
            ]
        );
        assert_eq!(command.get_current_dir(), Some(job.path()));
        let env = command
            .get_envs()
            .collect::<std::collections::BTreeMap<_, _>>();
        assert_eq!(env.len(), 3);
        let bootstrap = env
            .get(std::ffi::OsStr::new(bootstrap::ENV))
            .unwrap()
            .unwrap();
        assert!(bootstrap.len() <= 128);
        assert!(crate::dream_bootstrap::token(bootstrap.as_encoded_bytes()).is_ok());
        assert_eq!(
            env.get(std::ffi::OsStr::new("LANG")),
            Some(&Some(std::ffi::OsStr::new("C.UTF-8")))
        );
        assert_eq!(
            env.get(std::ffi::OsStr::new("TMPDIR")),
            Some(&Some(job.path().join("tmp").as_os_str()))
        );
        assert!(matches!(prepared.take_command(), Err(Code::BadRequest)));
        assert!(!job.path().join("input/manifest.json").exists());
    }
    #[test]
    fn unknown_ungated_candidate_is_never_executed_for_feature_detection() {
        use std::os::unix::fs::PermissionsExt;
        let root = tempfile::tempdir().unwrap();
        let script = root.path().join("candidate");
        std::fs::write(&script,b"#!/usr/bin/python3\nimport pathlib,sys\npathlib.Path(sys.argv[4]).joinpath('born').write_text('bad')\n").unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o700)).unwrap();
        assert!(matches!(
            Worker::spawn(&script, root.path()),
            Err(Code::RunnerUnavailable)
        ));
        assert!(matches!(
            Worker::spawn_prepared(&script, root.path(), Worker::prepare_buffers().unwrap()),
            Err(Code::RunnerUnavailable)
        ));
        assert!(!root.path().join("born").exists());
    }
}

/// Deliberately filled stdin is a private transport fault, not evidence that a
/// fresh canonical worker's first Start can encounter backpressure. This module
/// has private pipe access so production needs no descriptor bypass/test hook.
// Origin: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
#[cfg(test)]
mod start_write_tests {
    use super::*;
    use std::{
        ffi::CString,
        fs::OpenOptions,
        os::unix::fs::{OpenOptionsExt, PermissionsExt},
    };

    struct HeldInput {
        root: tempfile::TempDir,
        release: File,
        worker: Worker,
        start: wire::Start,
    }

    impl HeldInput {
        fn new() -> Self {
            let root = tempfile::tempdir().unwrap();
            let job = root.path().canonicalize().unwrap();
            std::fs::create_dir(job.join("tmp")).unwrap();
            let gate_path = job.join("drain-input.fifo");
            let gate_c = CString::new(gate_path.as_os_str().as_encoded_bytes()).unwrap();
            // SAFETY: creates one new FIFO only in this fixture's owned directory.
            assert_eq!(unsafe { libc::mkfifo(gate_c.as_ptr(), 0o600) }, 0);
            let release = OpenOptions::new()
                .read(true)
                .write(true)
                .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK)
                .open(&gate_path)
                .unwrap();
            let start = wire::Start {
                protocol: wire::Protocol::V1,
                run_id: wire::Ulid(ulid::Ulid::new().to_string()),
                nonce: wire::Ulid(ulid::Ulid::new().to_string()),
                manifest_sha256: wire::Hex("0".repeat(64)),
                timeout_ms: 30_000,
                activity_revision: wire::Number(1),
                foreground: false,
            };
            // The shape is generated from the local mirror ONLY to admit this
            // protocol-fault echo process. No second fixture/conformance authority.
            let hello = canonical(&wire::WorkerOut::Hello(wire::Hello {
                protocol: wire::Protocol::V1,
                algorithm: wire::Algorithm::D0C1,
                schema_sha256: wire::Hex(SCHEMA_SHA256.to_owned()),
                ir_major: 1,
                ir_minor_max: 3,
            }))
            .unwrap();
            let hello = String::from_utf8(hello).unwrap();
            let script = job.join("held-input.sh");
            // This transport-fault child needs no interpreter runtime startup.
            // Use the system shell, as the other private pipe fixtures do; the
            // canonical worker's original 250ms readiness gate stays unchanged.
            let hello = hello.replace('\'', "'\\''");
            std::fs::write(
                &script,
                format!(
                    "#!/bin/sh\nset -eu\ngate=${{BBOXY_DREAM_BOOTSTRAP#*:}}\ngate=${{gate%%:*}}\npermit_bytes=$(/bin/dd bs=128 count=1 <&\"$gate\" 2>/dev/null | /usr/bin/wc -c)\ntest \"$permit_bytes\" -eq 128\neval \"exec ${{gate}}<&-\"\nprintf '%s\\n' '{hello}'\nIFS= read -r release <\"$4/drain-input.fifo\"\ntest \"$release\" = x\nexec /bin/cat >\"$4/captured-input\"\n"
                ),
            )
            .unwrap();
            std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o700)).unwrap();
            let gate = Gate::prepare(bootstrap::Frozen::inspect(
                false,
                ulid::Ulid::new().to_bytes(),
                bootstrap::ulid(start.run_id.0.as_bytes()).unwrap(),
                bootstrap::ulid(start.nonce.0.as_bytes()).unwrap(),
                [0; 32],
                1,
            ))
            .unwrap();
            let mut prepared = Worker::prepare_command(
                &script,
                &job,
                Worker::prepare_buffers().unwrap(),
                Worker::prepare_pipe().unwrap(),
                Worker::prepare_pipe().unwrap(),
                Worker::prepare_pipe().unwrap(),
                gate,
            )
            .unwrap();
            prepared.configure_input().unwrap();
            prepared.configure_output().unwrap();
            prepared.configure_stderr().unwrap();
            let mut command = prepared.take_command().unwrap();
            let child = command.spawn().unwrap();
            let returned = Instant::now();
            let (mut worker, unused) = prepared.attach_child(child);
            // Retire every parent's duplicate child endpoint before EOF can be
            // observed; Worker alone retains the exact Child and stdin writer.
            drop(command);
            drop(unused);
            worker
                .bootstrap_spawn_returned(returned, returned + Duration::from_secs(30))
                .unwrap();
            worker.bootstrap_grant().unwrap();
            worker.hello().unwrap();
            Self {
                root,
                release,
                worker,
                start,
            }
        }

        fn fill_owned_pipe(&mut self) -> usize {
            // A bounded raw fault setup, not Worker::send: leave its queued
            // output empty so the next refusal records the actual Start syscall.
            const CHUNK: [u8; 512] = [b'x'; 512];
            const MAX_FILL: usize = 1024 * 1024;
            let input = self.worker.input.as_ref().unwrap();
            let mut filled = 0;
            for _ in 0..=MAX_FILL / CHUNK.len() {
                // SAFETY: writes only to this live fixture-owned nonblocking pipe.
                let n =
                    unsafe { libc::write(input.as_raw_fd(), CHUNK.as_ptr().cast(), CHUNK.len()) };
                if n < 0 {
                    assert_eq!(
                        std::io::Error::last_os_error().raw_os_error(),
                        Some(libc::EAGAIN),
                        "fault setup must reach real pipe backpressure"
                    );
                    assert!(filled > 0 && filled <= MAX_FILL);
                    return filled;
                }
                assert_eq!(n as usize, CHUNK.len(), "fixed atomic fill write");
                filled += n as usize;
            }
            panic!("owned pipe did not fill inside the fixed fault-setup bound");
        }

        fn release_and_reap(&mut self) {
            self.release.write_all(b"x\n").unwrap();
            let until = Instant::now() + Duration::from_secs(2);
            loop {
                assert!(self.worker.receive().unwrap().is_none());
                if let Some(status) = self.worker.try_wait().unwrap()
                    && self.worker.is_eof()
                {
                    assert!(status.success(), "private echo child failed: {status}");
                    break;
                }
                assert!(
                    Instant::now() < until,
                    "exact child did not reap and drain EOF"
                );
                self.worker.wait_io(Duration::from_millis(2));
            }
            let mut status = 0;
            // SAFETY: queries only the exact child already reaped by this Worker.
            assert_eq!(
                unsafe { libc::waitpid(self.worker.pid() as i32, &mut status, libc::WNOHANG) },
                -1
            );
            assert_eq!(
                std::io::Error::last_os_error().raw_os_error(),
                Some(libc::ECHILD)
            );
        }
    }

    #[test]
    fn prepared_start_raw_eagain_closes_input_and_cannot_flush_after_cancel_or_foreground() {
        let mut fixture = HeldInput::new();
        let prepared = PreparedStart::new(&fixture.start).unwrap();
        let filled = fixture.fill_owned_pipe();
        assert!(fixture.worker.write.is_empty());
        assert_eq!(fixture.worker.written, 0);
        assert_eq!(
            fixture.worker.send_prepared_start(&prepared),
            Err(Code::WorkerLost)
        );
        let refusal = fixture.worker.start_write_observation();
        assert!(refusal.attempted);
        assert_eq!(refusal.bytes, 0);
        assert_eq!(refusal.errno, libc::EAGAIN);
        assert!(fixture.worker.input.is_none());
        assert!(fixture.worker.write.is_empty());
        assert_eq!(fixture.worker.written, 0);
        // Exercise precisely the later leaf operations formerly able to flush
        // a queued Start. They cannot reopen its closed writer or repeat it.
        assert_eq!(fixture.worker.flush(), Err(Code::WorkerLost));
        assert!(
            fixture
                .worker
                .send(&WorkerIn::Activity(wire::Activity {
                    protocol: wire::Protocol::V1,
                    run_id: fixture.start.run_id.clone(),
                    nonce: fixture.start.nonce.clone(),
                    revision: wire::Number(2),
                    foreground: true,
                }))
                .is_err()
        );
        assert!(
            fixture
                .worker
                .send(&WorkerIn::Cancel(wire::WorkerCancel {
                    protocol: wire::Protocol::V1,
                    run_id: fixture.start.run_id.clone(),
                    nonce: fixture.start.nonce.clone(),
                }))
                .is_err()
        );
        fixture.worker.bootstrap_revoke();
        fixture.worker.close_input();
        assert_eq!(
            fixture.worker.send_prepared_start(&prepared),
            Err(Code::ProtocolError)
        );
        assert_eq!(fixture.worker.start_write_observation(), refusal);
        assert!(fixture.worker.try_wait().unwrap().is_none());
        fixture.release_and_reap();
        let received = std::fs::read(fixture.root.path().join("captured-input")).unwrap();
        assert_eq!(received.len(), filled);
        assert!(received.iter().all(|byte| *byte == b'x'));
        assert!(
            !received.contains(&b'\n'),
            "no Start/Cancel/Activity line crossed"
        );
    }
}
