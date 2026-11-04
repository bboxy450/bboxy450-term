// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Typed requests and results of the `task_system` 1.0 contract
//! (`schemas/task_system-1.0.schema.json`). Unknown fields are additive and
//! ignored; `signed_record` is opaque host-signed bytes, returned exactly.
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

pub use crate::data_source::ChangesRequest;

/// `task_system` 1.0 method names, in schema order.
pub const METHODS: [&str; 8] = [
    "list",
    "status",
    "delegate",
    "reassign",
    "report",
    "accept",
    "reject",
    "changes_since",
];

/// One task as the provider stores it. `metadata` is the task's portable
/// frontmatter (status, title, assignee, …).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskRecord {
    /// Portable ULID.
    pub id: String,
    /// Local `#N` handle, when the provider has one.
    pub number: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub external_id: Option<String>,
    pub path: String,
    pub metadata: Map<String, Value>,
    pub body: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body_truncated: Option<bool>,
    pub etag: String,
    /// The host-signed record of the last transition, byte for byte.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signed_record: Option<String>,
}

/// `list`: absent filters impose no restriction.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unassigned: Option<bool>,
    /// Operator only: merge the open unassigned tasks into the swarm's page.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub include_unassigned: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub swarm_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assignee: Option<String>,
    /// Exclusive cursor from the previous page's `next`: a task ULID, or
    /// `<updated ms>:<ULID>` for most-recently-changed-first pages.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

/// One page of tasks; `next` is the cursor to pass as `after`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskPage {
    pub items: Vec<TaskRecord>,
    pub next: Option<String>,
}

/// `status`: exactly one of `task_path` or `task_id` (`<ULID>` or `#N`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum StatusRequest {
    Path { task_path: String },
    Id { task_id: String },
}

/// `changes_since` result: task ULIDs changed after the cursor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Changes {
    pub changed: Vec<String>,
    pub next: String,
}

/// `delegate` a task to a seat (an oracle transition, host-signed).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DelegateRequest {
    pub task_path: String,
    pub etag: String,
    pub signed_record: String,
    pub seat_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
}

/// `reassign` a task to another seat (oracle, host-signed).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReassignRequest {
    pub task_path: String,
    pub etag: String,
    pub signed_record: String,
    pub seat_id: String,
}

/// The command and exit code that verified a report, with evidence rows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskVerification {
    pub command: String,
    pub exit_code: i32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence: Vec<Evidence>,
}

/// One acceptance criterion and where its evidence is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Evidence {
    pub criterion: String,
    pub reference: String,
}

/// `report` a task's result (worker, host-signed).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportRequest {
    pub task_path: String,
    pub etag: String,
    pub signed_record: String,
    pub summary: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commit: Option<String>,
    /// Locator of the committed immutable report document.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub report: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verify_result: Option<TaskVerification>,
}

/// `accept` a reported task (oracle, host-signed).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcceptRequest {
    pub task_path: String,
    pub etag: String,
    pub signed_record: String,
}

/// `reject` a reported task with a reason (oracle, host-signed).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RejectRequest {
    pub task_path: String,
    pub etag: String,
    pub signed_record: String,
    pub reason: String,
}

/// Every write returns the stored task with the request's `signed_record`
/// unchanged (`TaskWriteResult`).
pub type TaskWriteResult = TaskRecord;
