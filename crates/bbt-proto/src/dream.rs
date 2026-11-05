// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
// Adapted from bboxy450-harness src/dream/c1_wire.rs; sole schema/corpus authority:
// https://github.com/bboxy450/bboxy450-harness/blob/main/src/dream/c1_wire.rs
// Producer policy: https://github.com/bboxy450/bboxy450-harness/blob/main/ATTRIBUTION.md
//! Narrow public Dream C1 DTO mirror; the harness owns the schema and fixtures.
use serde::{Deserialize, Serialize};
/// Approved producer boundary a3425ac0a08e7184dc0a9198ddfa71f94a15ad54.
pub const SCHEMA_SHA256: &str = "5e3d8a602eeaa1da5cb97d0f6af0b4f331b216a02255d3dff990be84c30d6a30";
pub const CORPUS_LOCK_SHA256: &str =
    "31113dd0968ace9daf43b1e610b53a3fc9b586e7ff9a707136ec25ab5e9f2c31";
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
pub enum Algorithm {
    #[serde(rename = "d0-c1/1")]
    D0C1,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PageSchema {
    #[serde(rename = "bboxy.dream.c1.page/1")]
    V1,
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
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Row {
    Episode(Episode),
    Cluster(Cluster),
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

/// Canonical UTF-8 bytes used by the one harness-owned protocol.
pub fn canonical<T: Serialize>(value: &T) -> Result<Vec<u8>, serde_json::Error> {
    serde_json::to_vec(&serde_json::to_value(value)?)
}

/// Decode one bounded public message without accepting unknown fields.
pub fn decode<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T, Code> {
    if bytes.is_empty() || bytes.len() > MAX_FRAME {
        return Err(Code::ResourceLimit);
    }
    serde_json::from_slice(bytes).map_err(|_| Code::ProtocolError)
}

impl Control {
    /// Public input bounds beyond scalar JSON representation. Authority is host-owned.
    pub fn validate(&self) -> Result<(), Code> {
        if let Self::Inspect(request) = self {
            if !(1..=32).contains(&request.sources.len()) {
                return Err(Code::BadRequest);
            }
            let mut sessions = std::collections::BTreeSet::new();
            for source in &request.sources {
                if source.source_generation.0 == 0 || !sessions.insert(&source.session) {
                    return Err(Code::BadRequest);
                }
            }
        }
        Ok(())
    }
}

impl From<ulid::Ulid> for Ulid {
    fn from(value: ulid::Ulid) -> Self {
        Self(value.to_string())
    }
}
impl std::fmt::Display for Ulid {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}
