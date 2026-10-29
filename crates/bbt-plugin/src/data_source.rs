// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! The version 1.0 knowledge-provider contract (knowledge-base §2).
//!
//! Unknown object fields are additive and ignored. The host supplies authority
//! outside these DTOs; neither a principal nor a signing key is a call argument.
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Contract identity, shared by in-process and future stdio providers.
pub const CONTRACT: &str = "data_source";
/// Major must match; minor changes are additive.
pub const VERSION: &str = "1.0";
/// Highest method tier: write can send content outside the machine.
pub const TIER: &str = "export";
/// Default page size; callers can request at most [`MAX_PAGE`].
pub const DEFAULT_PAGE: usize = 32;
/// Maximum number of records admitted in one page.
pub const MAX_PAGE: usize = 128;
/// Serialized page bound, independent of a GUI transport's own frame bound.
pub const MAX_PAGE_BYTES: usize = 256 * 1024;
/// A resource is read only through the host's confined, bounded IO path.
pub const MAX_RESOURCE_BYTES: u64 = 64 * 1024 * 1024;
/// One schema asset, with request/result definitions for all eight methods.
pub const SCHEMA: &str = include_str!("../schemas/data_source-1.0.schema.json");

/// Address of a record within one knowledge root. A missing number is honest
/// for providers without local record handles (and deleted change references).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordRef {
    pub id: String,
    pub number: Option<u64>,
    pub slug: String,
    pub path: String,
    pub digest: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub external_id: Option<String>,
}

/// MCP-compatible resource metadata. The bundled provider's data directory is
/// its root; URI validation and reading remain host IO, never UI work.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceLink {
    #[serde(rename = "type", default)]
    pub kind: ResourceKind,
    pub uri: String,
    pub name: String,
    #[serde(rename = "mimeType")]
    pub mime_type: String,
    pub size: u64,
}

/// MCP resource-link discriminator; it is not an embedded resource or a tool.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceKind {
    #[default]
    ResourceLink,
}

/// Lossless document model: JSON frontmatter plus unrendered Markdown.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Record {
    #[serde(flatten)]
    pub reference: RecordRef,
    pub frontmatter: Value,
    pub body: String,
    pub etag: String,
    #[serde(default)]
    pub resources: Vec<ResourceLink>,
    /// Broken source remains addressable and visible, never silently omitted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parse_error: Option<String>,
}

/// Named command error; transport/protocol errors are not command results.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandError {
    pub code: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub candidates: Vec<RecordRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retry_after_ms: Option<u64>,
}
impl CommandError {
    /// Construct a non-retryable refusal with no ambiguous candidates.
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            candidates: Vec::new(),
            retry_after_ms: None,
        }
    }
}
impl std::fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}
impl std::error::Error for CommandError {}

/// Provider method result; authority is enforced by its host.
pub type Result<T> = std::result::Result<T, CommandError>;

/// Slug, path, ID or positive `#N` handle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolveRequest {
    pub locator: String,
}
/// Fetch one exact record reference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FetchRequest {
    #[serde(rename = "ref")]
    pub reference: RecordRef,
}
/// Plain text with optional `type:`, `status:` and `since:` terms.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchRequest {
    pub query: String,
    #[serde(default)]
    pub cursor: Option<String>,
    #[serde(default)]
    pub limit: Option<usize>,
}
/// Frontmatter filters; absent fields impose no restriction.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Filter {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub swarm: Option<String>,
    #[serde(default, rename = "type")]
    pub record_type: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub assignee: Option<String>,
    #[serde(default)]
    pub since: Option<String>,
}
/// Page through references using frontmatter filters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListRequest {
    #[serde(default)]
    pub filter: Filter,
    #[serde(default)]
    pub cursor: Option<String>,
    #[serde(default)]
    pub limit: Option<usize>,
}
/// Query-local page. `next` is opaque and belongs to this root and query.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub next: Option<String>,
}
/// Ranked result; score is ordering data, not confidence or authority.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchHit {
    #[serde(rename = "ref")]
    pub reference: RecordRef,
    pub title: String,
    #[serde(rename = "type")]
    pub record_type: String,
    pub number: Option<u64>,
    pub snippet: String,
    pub score: f64,
}
/// Weighted edge from the existing graph. Unresolved targets retain their
/// locator; they are never fabricated as records.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Edge {
    pub from: String,
    pub to: String,
    pub rel: String,
    pub weight: f64,
}
/// Both directions of a record's links.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Links {
    pub outgoing: Vec<Edge>,
    pub incoming: Vec<Edge>,
}
/// The document-owned graph families; execution and messaging are host-owned.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentGraph {
    Markdown,
    Task,
    Artifact,
}
/// Scope is explicit and never derived from a filesystem directory name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Scope {
    All {
        graph: DocumentGraph,
    },
    Type {
        graph: DocumentGraph,
        #[serde(rename = "type")]
        record_type: String,
    },
    Swarm {
        graph: DocumentGraph,
        swarm: String,
    },
    Neighborhood {
        graph: DocumentGraph,
        #[serde(rename = "ref")]
        reference: RecordRef,
        depth: u16,
    },
}
/// A bounded graph page request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphRequest {
    pub scope: Scope,
    #[serde(default)]
    pub cursor: Option<String>,
    #[serde(default)]
    pub limit: Option<usize>,
}
/// A node may name an unresolved target without pretending it is a record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Node {
    pub id: String,
    #[serde(rename = "ref")]
    pub reference: Option<RecordRef>,
    pub title: Option<String>,
    #[serde(rename = "type")]
    pub record_type: Option<String>,
    /// Indexed frontmatter lets the host retain its existing read policy
    /// without reading every Markdown body just to classify graph nodes.
    #[serde(default = "empty_frontmatter")]
    pub frontmatter: Value,
}
fn empty_frontmatter() -> Value {
    serde_json::json!({})
}
/// Outgoing edges travel with their source node, including edges whose target
/// appears on another page. Any edge truncation is explicit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GraphPage {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub next: Option<String>,
    pub edges_truncated: bool,
}
/// Creation requires `path` and no reference. Updates use the resolved path.
/// `etag` is checked before a write can be applied; it is never an authority.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WriteRequest {
    #[serde(default, rename = "ref")]
    pub reference: Option<RecordRef>,
    #[serde(default)]
    pub path: Option<String>,
    pub frontmatter: Value,
    pub body: String,
    #[serde(default)]
    pub etag: Option<String>,
}
/// The actual stored record, with any conversion loss made explicit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WriteResult {
    pub record: Record,
    pub lossy: bool,
}

/// Explicit, bounded publication of current records, never a whole workspace.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommitRequest {
    pub refs: Vec<RecordRef>,
    pub message: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommitResult {
    pub commit: String,
    pub committed: Vec<RecordRef>,
}
/// Read through the existing activity log, without acknowledging delivery.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangesRequest {
    pub cursor: String,
    #[serde(default)]
    pub limit: Option<usize>,
}
/// Position to use on the next poll, including when no record changed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Changes {
    pub changed: Vec<RecordRef>,
    pub next: String,
}

/// Select a request/result schema without defining the contract twice.
pub fn method_schema(method: &str, result: bool) -> Option<Value> {
    let name = match method {
        "resolve" => {
            if result {
                "RecordRef"
            } else {
                "ResolveRequest"
            }
        }
        "fetch" => {
            if result {
                "Record"
            } else {
                "FetchRequest"
            }
        }
        "search" => {
            if result {
                "SearchPage"
            } else {
                "SearchRequest"
            }
        }
        "list" => {
            if result {
                "RefPage"
            } else {
                "ListRequest"
            }
        }
        "links" => {
            if result {
                "Links"
            } else {
                "FetchRequest"
            }
        }
        "graph" => {
            if result {
                "GraphPage"
            } else {
                "GraphRequest"
            }
        }
        "write" => {
            if result {
                "WriteResult"
            } else {
                "WriteRequest"
            }
        }
        "commit" => {
            if result {
                "CommitResult"
            } else {
                "CommitRequest"
            }
        }
        "changes_since" => {
            if result {
                "Changes"
            } else {
                "ChangesRequest"
            }
        }
        _ => return None,
    };
    let schema: Value = serde_json::from_str(SCHEMA).expect("bundled data_source schema");
    crate::contracts::standalone_schema(&schema, &format!("#/$defs/{name}"))
}
