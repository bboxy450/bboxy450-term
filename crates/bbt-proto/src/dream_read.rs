// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Closed observation reads and reported-feedback DTOs; unavailable lessons fail closed.
use crate::dream::{self as c1, Hex, Id, Ulid, Version};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

pub const MAX_OUTPUT: usize = 16 * 1024;
pub const MAX_ENTRIES: usize = 128;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Observation,
    Lesson,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum View {
    Summary,
    Evidence,
    Context,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Search {
    pub v: Version,
    pub kind: Kind,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub query: Option<String>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub fingerprint: Option<Hex>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub limit: Option<u32>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub budget_tokens: Option<u32>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub cursor: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Read {
    pub v: Version,
    pub handle: String,
    pub view: View,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub budget_tokens: Option<u32>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub cursor: Option<String>,
}
fn present<'de, D: serde::Deserializer<'de>, T: Deserialize<'de>>(
    deserializer: D,
) -> Result<Option<T>, D::Error> {
    T::deserialize(deserializer).map(Some)
}
impl Search {
    pub fn validate(&self) -> Result<(), &'static str> {
        budget(self.budget_tokens)?;
        if !(1..=8).contains(&self.limit.unwrap_or(3))
            || self.query.as_ref().is_some_and(|q| {
                q.len() > 256 || q.split_whitespace().count() > 16 || q.contains('\0')
            })
            || self
                .cursor
                .as_ref()
                .is_some_and(|c| c.is_empty() || c.len() > 512)
            || (self.cursor.is_some() && (self.query.is_some() || self.fingerprint.is_some()))
            || (self.kind == Kind::Lesson && self.fingerprint.is_some())
        {
            return Err("bad_request");
        }
        Ok(())
    }
}
impl Read {
    pub fn validate(&self) -> Result<(), &'static str> {
        budget(self.budget_tokens)?;
        if self.handle.is_empty()
            || self.handle.len() > 256
            || self
                .cursor
                .as_ref()
                .is_some_and(|c| c.is_empty() || c.len() > 512)
        {
            return Err("bad_request");
        }
        Ok(())
    }
}
pub fn budget(b: Option<u32>) -> Result<usize, &'static str> {
    let n = b.unwrap_or(2048);
    if !(1024..=4096).contains(&n) {
        Err("bad_request")
    } else {
        Ok(n as usize)
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResultRowPin {
    pub run_id: Ulid,
    pub manifest: Hex,
    pub result: Hex,
    pub page_index: u32,
    pub page_sha256: Hex,
    pub row_index: u32,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClusterMembers {
    pub basis: MemberBasis,
    pub links: Vec<ResultRowPin>,
    pub total: u32,
    pub omitted: u32,
    pub complete: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemberBasis {
    #[serde(rename = "result_rows")]
    ResultRows,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Trust {
    #[serde(rename = "not_evaluated")]
    NotEvaluated,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ObservationProof {
    ClosedIrEpisode {
        row: ResultRowPin,
        origin: c1::Origin,
        trust: Trust,
    },
    ClosedIrCluster {
        row: ResultRowPin,
        members: ClusterMembers,
        trust: Trust,
    },
}
impl ObservationProof {
    pub fn row(&self) -> &ResultRowPin {
        match self {
            Self::ClosedIrEpisode { row, .. } | Self::ClosedIrCluster { row, .. } => row,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecordPin {
    pub root: Id,
    pub id: Id,
    pub path: String,
    pub commit: String,
    pub digest: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Pin {
    Event(c1::Origin),
    Record(RecordPin),
}
impl Pin {
    pub fn validate(&self) -> Result<(), &'static str> {
        let digest = match self {
            Self::Event(p) => &p.digest,
            Self::Record(p) => {
                if p.path.is_empty()
                    || p.path.len() > 4096
                    || p.path.contains('\0')
                    || !matches!(p.commit.len(), 40 | 64)
                    || !p.commit.bytes().all(|b| b.is_ascii_hexdigit())
                {
                    return Err("bad_request");
                }
                &p.digest
            }
        };
        digest
            .strip_prefix("sha256:")
            .and_then(|s| Hex::try_from(s.to_owned()).ok())
            .map(|_| ())
            .ok_or("bad_request")
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeedbackOutcome {
    Used,
    NotApplicable,
    Contradicted,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Feedback {
    pub v: Version,
    pub delivery: Hex,
    pub outcome: FeedbackOutcome,
    pub evidence: Vec<Pin>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub note: Option<String>,
}
impl Feedback {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.evidence.len() > 8
            || self
                .note
                .as_ref()
                .is_some_and(|s| s.len() > 512 || s.contains('\0'))
        {
            return Err("bad_request");
        }
        for pin in &self.evidence {
            pin.validate()?;
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FeedbackResult {
    pub v: Version,
    pub status: Recorded,
    pub report: Hex,
    pub assessment: Assessment,
    pub duplicate: bool,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum Recorded {
    #[serde(rename = "recorded")]
    Recorded,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum Assessment {
    #[serde(rename = "reported_only")]
    ReportedOnly,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Hit {
    pub handle: Id,
    pub id: Hex,
    pub kind: ObservationKind,
    pub title: String,
    pub preview: String,
    pub repo: Id,
    pub harness: Id,
    pub proof: ObservationProof,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObservationKind {
    #[serde(rename = "observation")]
    Observation,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Currentness {
    #[serde(rename = "historical_observation")]
    HistoricalObservation,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Coverage {
    pub kinds: Vec<Kind>,
    pub indexed: u32,
    pub omitted: u32,
    pub complete: bool,
    pub currentness: Currentness,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum Available {
    #[serde(rename = "available")]
    Available,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Page {
    pub v: Version,
    pub status: Available,
    pub snapshot: Hex,
    pub items: Vec<Hit>,
    pub next: Option<String>,
    pub truncated: bool,
    pub coverage: Coverage,
    pub offered: Option<Hex>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Payload {
    Observation {
        episode: Option<c1::Episode>,
        cluster: Option<c1::Cluster>,
    },
    ObservationEvidence {
        proof: ObservationProof,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadPage {
    pub v: Version,
    pub status: Available,
    pub snapshot: Hex,
    pub item: Hit,
    pub view: View,
    pub payload: Payload,
    pub next: Option<String>,
    pub truncated: bool,
    pub offered: Option<Hex>,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnavailableStatus {
    Unavailable,
    Pending,
    Inconclusive,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Reason {
    NotPrepared,
    NotALesson,
    NoPublishedInput,
    StorageUnavailable,
    Busy,
    ProviderProofUnavailable,
    ProducerUnapproved,
    HarnessIncompatible,
    StaleSource,
    VerificationInconclusive,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Unavailable {
    pub v: Version,
    pub status: UnavailableStatus,
    pub reason: Reason,
    pub run_id: Option<Ulid>,
    pub retry_after_ms: Option<u32>,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ErrorStatus {
    #[serde(rename = "error")]
    Error,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Code {
    BadRequest,
    VersionMismatch,
    NotFound,
    StaleContext,
    StaleCursor,
    BudgetExceeded,
    OpIdReused,
    ResourceLimit,
    ProtocolError,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Error {
    pub code: Code,
    pub retryable: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Failure {
    pub v: Version,
    pub status: ErrorStatus,
    pub error: Error,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Reply {
    Page(Page),
    Read(Box<ReadPage>),
    Unavailable(Unavailable),
    Failure(Failure),
    Feedback(FeedbackResult),
}
impl Reply {
    pub fn unavailable(reason: Reason) -> Self {
        Self::Unavailable(Unavailable {
            v: Version,
            status: UnavailableStatus::Unavailable,
            reason,
            run_id: None,
            retry_after_ms: None,
        })
    }
    pub fn failure(code: Code) -> Self {
        Self::Failure(Failure {
            v: Version,
            status: ErrorStatus::Error,
            error: Error {
                code,
                retryable: matches!(code, Code::ResourceLimit),
            },
        })
    }
}

fn obj(properties: Value, required: &[&str]) -> Value {
    json!({"type":"object","properties":properties,"required":required,"additionalProperties":false})
}
fn choice(names: &[&str]) -> Value {
    json!({"type":"string","enum":names})
}
fn string(max: usize) -> Value {
    json!({"type":"string","minLength":1,"maxLength":max})
}
fn number() -> Value {
    json!({"type":"integer","minimum":0,"maximum":c1::MAX_SAFE})
}
fn nullable(v: Value) -> Value {
    json!({"anyOf":[v,{"type":"null"}]})
}
fn array(v: Value, max: usize) -> Value {
    json!({"type":"array","items":v,"maxItems":max})
}
fn hex() -> Value {
    json!({"type":"string","pattern":"^[0-9a-f]{64}$"})
}
fn ulid() -> Value {
    json!({"type":"string","pattern":"^[0-7][0-9A-HJKMNP-TV-Z]{25}$"})
}
pub fn search_schema() -> Value {
    obj(
        json!({"v":{"const":1,"type":"integer"},"kind":choice(&["observation","lesson"]),"query":{"type":"string","maxLength":256},"fingerprint":hex(),"limit":{"type":"integer","minimum":1,"maximum":8},"budget_tokens":{"type":"integer","minimum":1024,"maximum":4096},"cursor":string(512)}),
        &["v", "kind"],
    )
}
pub fn read_schema() -> Value {
    obj(
        json!({"v":{"const":1,"type":"integer"},"handle":string(256),"view":choice(&["summary","evidence","context"]),"budget_tokens":{"type":"integer","minimum":1024,"maximum":4096},"cursor":string(512)}),
        &["v", "handle", "view"],
    )
}
pub fn feedback_schema() -> Value {
    obj(
        json!({"v":{"const":1,"type":"integer"},"delivery":hex(),"outcome":choice(&["used","not_applicable","contradicted"]),"evidence":array(pin_schema(),8),"note":{"type":"string","maxLength":512}}),
        &["v", "delivery", "outcome", "evidence"],
    )
}
fn row_schema() -> Value {
    obj(
        json!({"run_id":ulid(),"manifest":hex(),"result":hex(),"page_index":number(),"page_sha256":hex(),"row_index":number()}),
        &[
            "run_id",
            "manifest",
            "result",
            "page_index",
            "page_sha256",
            "row_index",
        ],
    )
}
fn origin_schema() -> Value {
    obj(
        json!({"session":ulid(),"seq":number(),"digest":{"type":"string","pattern":"^sha256:[0-9a-f]{64}$"}}),
        &["session", "seq", "digest"],
    )
}
fn record_schema() -> Value {
    obj(
        json!({"root":string(256),"id":string(256),"path":string(4096),"commit":{"type":"string","pattern":"^([0-9a-fA-F]{40}|[0-9a-fA-F]{64})$"},"digest":{"type":"string","pattern":"^sha256:[0-9a-f]{64}$"}}),
        &["root", "id", "path", "commit", "digest"],
    )
}
fn pin_schema() -> Value {
    let mut event = origin_schema();
    event["properties"]["kind"] = json!({"const":"event"});
    event["required"]
        .as_array_mut()
        .unwrap()
        .push(json!("kind"));
    let mut record = record_schema();
    record["properties"]["kind"] = json!({"const":"record"});
    record["required"]
        .as_array_mut()
        .unwrap()
        .push(json!("kind"));
    json!({"oneOf":[event,record]})
}
fn proof_schema() -> Value {
    json!({"oneOf":[
        obj(json!({"kind":{"const":"closed_ir_episode"},"row":row_schema(),"origin":origin_schema(),"trust":{"const":"not_evaluated"}}),&["kind","row","origin","trust"]),
        obj(json!({"kind":{"const":"closed_ir_cluster"},"row":row_schema(),"members":obj(json!({"basis":{"const":"result_rows"},"links":array(row_schema(),8),"total":number(),"omitted":number(),"complete":{"type":"boolean"}}),&["basis","links","total","omitted","complete"]),"trust":{"const":"not_evaluated"}}),&["kind","row","members","trust"])
    ]})
}
fn hit_schema() -> Value {
    obj(
        json!({"handle":string(256),"id":hex(),"kind":{"const":"observation"},"title":string(160),"preview":{"type":"string","maxLength":256},"repo":string(256),"harness":string(256),"proof":proof_schema()}),
        &[
            "handle", "id", "kind", "title", "preview", "repo", "harness", "proof",
        ],
    )
}
/// Extend EACH closed domain variant with precisely the two shared host fields.
pub fn output_schema(hints: Value) -> Value {
    let episode = obj(
        json!({"session":ulid(),"turn":ulid(),"origin":origin_schema(),"fingerprints":array(hex(),128),"first_ms":number(),"last_ms":number()}),
        &[
            "session",
            "turn",
            "origin",
            "fingerprints",
            "first_ms",
            "last_ms",
        ],
    );
    let cluster = obj(
        json!({"repo":string(4096),"fingerprint":hex(),"count":number(),"first_ms":number(),"last_ms":number()}),
        &["repo", "fingerprint", "count", "first_ms", "last_ms"],
    );
    let payload = json!({"oneOf":[obj(json!({"kind":{"const":"observation"},"episode":nullable(episode),"cluster":nullable(cluster)}),&["kind","episode","cluster"]),obj(json!({"kind":{"const":"observation_evidence"},"proof":proof_schema()}),&["kind","proof"])]});
    let mut variants = vec![
        obj(
            json!({"v":{"const":1},"status":{"const":"available"},"snapshot":hex(),"items":array(hit_schema(),8),"next":nullable(string(512)),"truncated":{"type":"boolean"},"coverage":obj(json!({"kinds":array(choice(&["observation"]),1),"indexed":number(),"omitted":number(),"complete":{"type":"boolean"},"currentness":{"const":"historical_observation"}}),&["kinds","indexed","omitted","complete","currentness"]),"offered":{"type":"null"}}),
            &[
                "v",
                "status",
                "snapshot",
                "items",
                "next",
                "truncated",
                "coverage",
                "offered",
            ],
        ),
        obj(
            json!({"v":{"const":1},"status":{"const":"available"},"snapshot":hex(),"item":hit_schema(),"view":choice(&["summary","evidence"]),"payload":payload,"next":nullable(string(512)),"truncated":{"type":"boolean"},"offered":{"type":"null"}}),
            &[
                "v",
                "status",
                "snapshot",
                "item",
                "view",
                "payload",
                "next",
                "truncated",
                "offered",
            ],
        ),
        obj(
            json!({"v":{"const":1},"status":choice(&["unavailable","pending","inconclusive"]),"reason":choice(&["not_prepared","not_a_lesson","no_published_input","storage_unavailable","busy","provider_proof_unavailable","producer_unapproved","harness_incompatible","stale_source","verification_inconclusive"]),"run_id":nullable(ulid()),"retry_after_ms":nullable(number())}),
            &["v", "status", "reason", "run_id", "retry_after_ms"],
        ),
        obj(
            json!({"v":{"const":1},"status":{"const":"error"},"error":obj(json!({"code":choice(&["bad_request","version_mismatch","not_found","stale_context","stale_cursor","budget_exceeded","op_id_reused","resource_limit","protocol_error"]),"retryable":{"type":"boolean"}}),&["code","retryable"])}),
            &["v", "status", "error"],
        ),
    ];
    variants.push(obj(json!({"v":{"const":1},"status":{"const":"recorded"},"report":hex(),"assessment":{"const":"reported_only"},"duplicate":{"type":"boolean"}}), &["v","status","report","assessment","duplicate"]));
    for v in &mut variants {
        v["properties"]["op_id"] = ulid();
        v["properties"]["hints"] = hints.clone();
        v["required"]
            .as_array_mut()
            .unwrap()
            .extend([json!("op_id"), json!("hints")]);
    }
    json!({"type":"object","oneOf":variants})
}
