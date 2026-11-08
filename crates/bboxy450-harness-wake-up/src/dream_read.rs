// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Narrow prepared-read codec; process custody and proof admission remain host-owned.
//! Adapted from bboxy450-harness src/dream/read_wire.rs at f848c03ff0462e55dfa0b17fcfd550626a52db2b.
//! https://github.com/bboxy450/bboxy450-harness/blob/f848c03ff0462e55dfa0b17fcfd550626a52db2b/src/dream/read_wire.rs
//! Producer license/policy: https://github.com/bboxy450/bboxy450-harness/blob/main/ATTRIBUTION.md
#![allow(missing_docs)]

/// Strict producer-owned prepared-read DTO mirror; no independent artifact schema.
pub mod wire {
    pub use crate::dream_c1::wire::{Context, Hex, Id, Number, PageRef, Source, Ulid};
    use serde::{Deserialize, Serialize};
    pub const MAX_FRAME: usize = 256 * 1024;
    pub const MAX_SAFE: u64 = crate::dream_c1::wire::MAX_SAFE;
    pub const MAX_INPUT: u64 = 64 * 1024 * 1024;
    pub const MAX_SNAPSHOT: usize = 4 * 1024 * 1024;
    pub const MAX_PAGE: usize = 16 * 1024;
    pub const MAX_ENTRY: usize = 12 * 1024;
    fn explicit_nullable<'de, D: serde::Deserializer<'de>, T: Deserialize<'de>>(
        d: D,
    ) -> Result<Option<T>, D::Error> {
        Option::<T>::deserialize(d)
    }
    macro_rules! bounded_text {
        ($name:ident, $max:expr) => {
            fn $name<'de, D: serde::Deserializer<'de>>(d: D) -> Result<String, D::Error> {
                let text = String::deserialize(d)?;
                if text.len() > $max {
                    Err(serde::de::Error::custom("prepared text byte limit"))
                } else {
                    Ok(text)
                }
            }
        };
    }
    bounded_text!(text160, 160);
    bounded_text!(text256, 256);
    bounded_text!(text2048, 2048);
    bounded_text!(text8192, 8192);
    macro_rules! bounded_vec {
        ($name:ident,$item:ty,$max:expr) => {
            fn $name<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<$item>, D::Error> {
                let values = Vec::<$item>::deserialize(d)?;
                if values.len() > $max {
                    Err(serde::de::Error::custom("prepared collection limit"))
                } else {
                    Ok(values)
                }
            }
        };
    }
    bounded_vec!(files2048, BundleFile, 2048);
    bounded_vec!(roots32, RootBinding, 32);
    bounded_vec!(records512, RecordPin, 512);
    bounded_vec!(sources32, Source, 32);
    bounded_vec!(nodes16, EvidenceNode, 16);
    bounded_vec!(edges32, EvidenceEdge, 32);

    macro_rules! scalar {
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
                fn from(v: $name) -> String {
                    v.0
                }
            }
        };
    }
    scalar!(PathText, |s: &str| !s.is_empty()
        && s.len() <= 4096
        && !s.contains('\0'));
    scalar!(RelativePath, |s: &str| !s.is_empty()
        && s.len() <= 4096
        && !s.contains(['\0', '\\'])
        && s.split('/').all(|c| !c.is_empty() && c != "." && c != ".."));
    scalar!(Commit, |s: &str| matches!(s.len(), 40 | 64)
        && s.bytes().all(|c| c.is_ascii_hexdigit()));
    scalar!(Digest, |s: &str| s
        .strip_prefix("sha256:")
        .is_some_and(|h| Hex::try_from(h.to_owned()).is_ok()));

    macro_rules! discriminator {
        ($name:ident, $variant:ident, $value:literal) => {
            #[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
            pub enum $name {
                #[serde(rename = $value)]
                $variant,
            }
        };
    }
    discriminator!(Protocol, V1, "bboxy.dream.read/1");
    discriminator!(Algorithm, ReadPrepare, "read-prepare/1");
    discriminator!(InputSchema, V1, "bboxy.dream.read-input/1");
    discriminator!(SnapshotSchema, V1, "bboxy.dream.read-snapshot/1");
    discriminator!(PageSchema, V1, "bboxy.dream.read-page/1");
    discriminator!(Kind, Lesson, "lesson");
    discriminator!(ProofKind, VerifiedLesson, "verified_lesson");
    discriminator!(Currentness, VerifiedAtRevision, "verified_at_revision");

    #[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "snake_case")]
    pub enum Code {
        BadRequest,
        SourceMismatch,
        SourceChanged,
        UnsupportedIr,
        BlobMismatch,
        ProducerUnapproved,
        VerificationUnproven,
        VerificationFailed,
        HarnessIncompatible,
        ResourceLimit,
        StorageUnavailable,
        ProtocolError,
        Cancelled,
        Deadline,
        WorkerLost,
    }
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "snake_case")]
    pub enum Phase {
        Validate,
        Prepare,
        Paused,
        Settling,
    }
    pub use crate::dream_c1::wire::Outcome;

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct RecordPin {
        pub root: Id,
        pub id: Id,
        pub path: PathText,
        pub commit: Commit,
        pub digest: Digest,
    }
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct EventPin {
        pub session: Ulid,
        pub seq: Number,
        pub digest: Digest,
    }
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
    pub enum Pin {
        Event(EventPin),
        Record(RecordPin),
    }

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct BundleFile {
        pub slot: u32,
        pub path: RelativePath,
        pub len: Number,
        pub sha256: Hex,
    }
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct RootBinding {
        pub root: Id,
        pub repo: Id,
        pub main: Commit,
        pub dir: RelativePath,
    }
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct KeyBinding {
        pub fingerprint: Id,
        pub file: u32,
    }
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct ArtifactBinding {
        pub logical: PathText,
        pub file: u32,
    }
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct EvidenceBundle {
        #[serde(deserialize_with = "files2048")]
        pub files: Vec<BundleFile>,
        #[serde(deserialize_with = "roots32")]
        pub roots: Vec<RootBinding>,
        pub policy: u32,
        pub public_keys: Vec<KeyBinding>,
        pub artifacts: Vec<ArtifactBinding>,
        #[serde(deserialize_with = "records512")]
        pub catalogue: Vec<RecordPin>,
        pub selected_at_ms: Number,
    }
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct Input {
        pub schema: InputSchema,
        pub nonce: Ulid,
        pub context: Context,
        #[serde(deserialize_with = "sources32")]
        pub closed_sources: Vec<Source>,
        #[serde(deserialize_with = "records512")]
        pub records: Vec<RecordPin>,
        pub publications: Vec<EventPin>,
        pub trust_policy: Hex,
        pub provider_revision: Id,
        pub evidence: EvidenceBundle,
    }
    pub type Manifest = Input;

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct Start {
        pub protocol: Protocol,
        pub run_id: Ulid,
        pub nonce: Ulid,
        pub input_sha256: Hex,
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
    pub struct Cancel {
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
    pub struct PrepareError {
        pub code: Code,
        #[serde(deserialize_with = "explicit_nullable")]
        pub source: Option<u32>,
    }
    pub type Error = PrepareError;
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct Done {
        pub protocol: Protocol,
        pub run_id: Ulid,
        pub nonce: Ulid,
        pub seq: Number,
        pub outcome: Outcome,
        #[serde(deserialize_with = "explicit_nullable")]
        pub result_sha256: Option<Hex>,
        pub pages: u32,
        #[serde(deserialize_with = "explicit_nullable")]
        pub error: Option<PrepareError>,
    }
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
    pub enum WorkerIn {
        Start(Start),
        Activity(Activity),
        Cancel(Cancel),
    }
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
    pub enum WorkerOut {
        Hello(Hello),
        Progress(Progress),
        Done(Done),
    }

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct Coverage {
        pub kinds: Vec<Kind>,
        pub indexed: u32,
        pub omitted: u32,
        pub complete: bool,
        pub currentness: Currentness,
    }
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct LessonProof {
        pub kind: ProofKind,
        pub entry: Id,
        pub revival: u32,
        pub record: RecordPin,
        pub trial: EventPin,
        pub publication: EventPin,
        pub outer: EventPin,
        pub projection: Hex,
        pub trust_policy: Hex,
        pub producer: Hex,
        pub currentness: Hex,
    }
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct Hit {
        pub handle: Id,
        pub id: Id,
        pub kind: Kind,
        #[serde(deserialize_with = "text160")]
        pub title: String,
        #[serde(deserialize_with = "text256")]
        pub preview: String,
        pub repo: Id,
        pub harness: Id,
        pub proof: LessonProof,
    }
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct EvidenceNode {
        pub id: Id,
        pub kind: Id,
        #[serde(deserialize_with = "explicit_nullable")]
        pub pin: Option<Pin>,
    }
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct EvidenceEdge {
        pub from: Id,
        pub to: Id,
        pub relation: Id,
    }
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct PreparedEntry {
        pub hit: Hit,
        #[serde(deserialize_with = "text2048")]
        pub why: String,
        #[serde(deserialize_with = "text2048")]
        pub how_to_apply: String,
        #[serde(deserialize_with = "text8192")]
        pub context: String,
        #[serde(deserialize_with = "text256")]
        pub citation: String,
        #[serde(deserialize_with = "nodes16")]
        pub nodes: Vec<EvidenceNode>,
        #[serde(deserialize_with = "edges32")]
        pub edges: Vec<EvidenceEdge>,
    }
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct Page {
        pub schema: PageSchema,
        pub run_id: Ulid,
        pub nonce: Ulid,
        pub index: u32,
        pub entries: Vec<PreparedEntry>,
        #[serde(deserialize_with = "explicit_nullable")]
        pub next: Option<u32>,
    }
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct Snapshot {
        pub schema: SnapshotSchema,
        pub run_id: Ulid,
        pub nonce: Ulid,
        pub context: Context,
        pub input_digest: Hex,
        pub source_revision: Hex,
        pub producer: Hex,
        pub trust_policy: Hex,
        pub entry_ids: Vec<Id>,
        pub coverage: Coverage,
        pub pages: Vec<PageRef>,
    }
    pub type ResultDescriptor = Snapshot;
}

use serde::Serialize;
use wire::{Code, Hex, Input, PreparedEntry};

/// Exact accepted producer revision from the assigned Q3 brief.
pub const PRODUCER_COMMIT: &str = "f848c03ff0462e55dfa0b17fcfd550626a52db2b";
/// SHA256 of the producer-owned closed read artifact schema.
pub const SCHEMA_SHA256: &str = "0afcbe084c959118cea8d3b49008e5cd9654d7ff5caa8565bf3691fe2ce7a08a";
/// SHA256 of the complete producer fixture lock at the accepted revision.
pub const CORPUS_LOCK_SHA256: &str =
    "00f0e61b07be56f1019227a50ad1ada55835bdfbaced9d1b3e5830c2e6247665";

pub fn canonical<T: Serialize>(value: &T) -> Result<Vec<u8>, serde_json::Error> {
    // Match the producer even when ACP unifies serde_json/preserve_order.
    let mut value = serde_json::to_value(value)?;
    value.sort_all_objects();
    serde_json::to_vec(&value)
}

pub fn canonical_limited<T: Serialize>(value: &T, limit: usize) -> Result<Vec<u8>, Code> {
    let bytes = canonical(value).map_err(|_| Code::ProtocolError)?;
    if bytes.len() > limit {
        Err(Code::ResourceLimit)
    } else {
        Ok(bytes)
    }
}

pub fn decode<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T, Code> {
    if bytes.is_empty() || bytes.len() > wire::MAX_FRAME {
        return Err(Code::ResourceLimit);
    }
    crate::dream_c1::preflight_json(bytes, crate::dream_c1::JsonShape::Any)
        .map_err(|_| Code::ProtocolError)?;
    serde_json::from_slice(bytes).map_err(|_| Code::ProtocolError)
}

/// Qualify only the fixed producer protocol/schema/IR capability, before inputs.
/// This check does not certify process bootstrap or actual settlement.
pub fn verify_hello(hello: &wire::Hello) -> Result<(), Code> {
    if hello.protocol == wire::Protocol::V1
        && hello.algorithm == wire::Algorithm::ReadPrepare
        && hello.schema_sha256.0 == SCHEMA_SHA256
        && hello.ir_major == 1
        && hello.ir_minor_max == 3
    {
        Ok(())
    } else {
        Err(Code::ProtocolError)
    }
}

/// Exact canonical producer revision material; hash using the caller's existing SHA256.
pub fn source_revision_bytes(input: &Input) -> Result<Vec<u8>, serde_json::Error> {
    #[derive(Serialize)]
    struct Revision<'a> {
        context: &'a wire::Context,
        provider_revision: &'a wire::Id,
        closed_sources: &'a [wire::Source],
        evidence: &'a wire::EvidenceBundle,
        trust_policy: &'a Hex,
    }
    canonical(&Revision {
        context: &input.context,
        provider_revision: &input.provider_revision,
        closed_sources: &input.closed_sources,
        evidence: &input.evidence,
        trust_policy: &input.trust_policy,
    })
}

/// Hash exact revision material without adding a second hashing dependency.
pub fn source_revision(input: &Input, digest: impl FnOnce(&[u8]) -> String) -> Result<Hex, Code> {
    let bytes = source_revision_bytes(input).map_err(|_| Code::ProtocolError)?;
    Hex::try_from(digest(&bytes)).map_err(|_| Code::ProtocolError)
}

/// Producer entry shape/bounds only; publication proof is not established here.
pub fn validate_entry(entry: &PreparedEntry) -> Result<(), Code> {
    if entry.hit.title.len() > 160
        || entry.hit.preview.len() > 256
        || entry.why.len() > 2048
        || entry.how_to_apply.len() > 2048
        || entry.context.len() > 8192
        || entry.citation.len() > 256
        || entry.nodes.len() > 16
        || entry.edges.len() > 32
    {
        return Err(Code::ResourceLimit);
    }
    if entry.hit.id != entry.hit.proof.entry {
        return Err(Code::VerificationFailed);
    }
    canonical_limited(entry, wire::MAX_ENTRY).map(|_| ())
}
