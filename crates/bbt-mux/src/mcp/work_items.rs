// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Typed work-item transitions. Markdown is authority; MCP journals attempts.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path};
use std::time::{Duration, Instant};

use bbt_proto::messaging::{
    Recipient, TaskCard, TaskDoc, TaskRequest, TaskResponse, TaskVerifySummary,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use ulid::Ulid;

use super::{Actor, Call, tools};
use crate::index::{Document, Index, Update};
use crate::knowledge::{DataSource, data_source as d};
use crate::registry::{Class, Registry, SeatRecord};

type Checked<T> = std::result::Result<T, String>;

/// One task inventory read answers well inside the seat bridge's 10 s socket
/// timeout (`mcp/bridge.rs`), across every graph, list and fetch call it makes.
pub(crate) const TASK_LIST_BUDGET: Duration = Duration::from_secs(8);
/// Code of the named refusal when [`TASK_LIST_BUDGET`] runs out.
pub(crate) const DEADLINE_EXCEEDED: &str = "deadline_exceeded";

/// Refusal code for a task read failure message: the named deadline or `rejected`.
pub(crate) fn failure_code(reason: &str) -> &'static str {
    if reason.starts_with(DEADLINE_EXCEEDED) {
        DEADLINE_EXCEEDED
    } else {
        "rejected"
    }
}

#[cfg(test)]
#[path = "work_items_inventory_tests.rs"]
mod inventory_tests;

/// Task policy consumes the selected document source; the host still owns transitions.
pub(crate) trait TaskDocuments {
    fn import_root(&self) -> Checked<std::path::PathBuf>;
    fn locate(&self, locator: &str) -> Checked<Document>;
    /// Return an authorized page plus one lookahead, with no unrelated body reads.
    fn documents(&self, registry: &Registry, actor: &Actor, args: &List) -> Checked<Vec<Document>>;
}
impl TaskDocuments for Index {
    fn import_root(&self) -> Checked<std::path::PathBuf> {
        Ok(self.knowledge_root().to_owned())
    }
    fn locate(&self, locator: &str) -> Checked<Document> {
        locate_index(self, locator)
    }
    fn documents(&self, registry: &Registry, actor: &Actor, args: &List) -> Checked<Vec<Document>> {
        let source = crate::knowledge::BboxySource::new(self);
        ProviderTasks(&source, None).documents(registry, actor, args)
    }
}
pub(crate) struct ProviderTasks<'a>(pub &'a dyn DataSource, pub Option<Ulid>);
impl TaskDocuments for ProviderTasks<'_> {
    fn import_root(&self) -> Checked<std::path::PathBuf> {
        self.0.import_root().map_err(|error| error.to_string())
    }
    fn locate(&self, locator: &str) -> Checked<Document> {
        let locator = locator.strip_prefix(".bboxy/").unwrap_or(locator);
        let reference = self
            .0
            .resolve(d::ResolveRequest {
                locator: locator.into(),
            })
            .map_err(|e| e.to_string())?;
        self.fetch(reference, None)
    }
    fn documents(&self, registry: &Registry, actor: &Actor, args: &List) -> Checked<Vec<Document>> {
        if self
            .1
            .is_some_and(|scope| args.swarm_id.is_some_and(|swarm| swarm != scope))
        {
            return Err("task list outside captured swarm".into());
        }
        let scope = self
            .1
            .or(args.swarm_id)
            .or_else(|| actor.seat().map(|seat| seat.swarm_id));
        let assignee = args.assignee.as_deref().map(seat_reference).transpose()?;
        let limit = tools::page_limit(args.limit)?;
        let deadline = Instant::now() + TASK_LIST_BUDGET;
        let mut candidates = BTreeMap::new();
        // Native imports retain unassigned tasks with a scoped delegator. Their
        // metadata is already in the bundled Index; external sources retain the
        // assignee queries below rather than scanning unrelated documents.
        let native_inventory =
            assignee.is_none() && actor.class() == Class::Oracle && self.0.import_root().is_ok();
        if args.unassigned
            || assignee == Some(None)
            || scope.is_none() && assignee.is_none()
            || native_inventory
        {
            // Classify wide or unassigned inventory from typed metadata,
            // then read only the authorized output page, never every body.
            for kind in ["task", "bug", "epic"] {
                let mut cursor = None;
                let mut seen = BTreeSet::new();
                for page_number in 0..64 {
                    let request = d::GraphRequest {
                        scope: d::Scope::Type {
                            graph: d::DocumentGraph::Markdown,
                            record_type: kind.into(),
                        },
                        cursor,
                        limit: Some(d::MAX_PAGE),
                    };
                    let page = within(deadline, || self.0.graph_until(request, deadline))?;
                    for node in page.nodes {
                        let Some(reference) = node.reference else {
                            continue;
                        };
                        if node.id != reference.id {
                            return Err("inconsistent task graph identity".into());
                        }
                        let metadata = Document {
                            id: reference.id.clone(),
                            path: reference.path.clone(),
                            metadata: node.frontmatter,
                            body: String::new(),
                            digest: reference.digest.clone(),
                            error: None,
                            links: Vec::new(),
                        };
                        if metadata.metadata["type"].as_str().is_some() && !is_task(&metadata) {
                            continue;
                        }
                        // Older graph providers may omit metadata; retain their
                        // candidates and apply the same checks after fetching.
                        let complete = metadata.metadata["id"].as_str()
                            == Some(reference.id.as_str())
                            && is_task(&metadata)
                            && metadata.metadata["status"].as_str().is_some();
                        if !complete || listed(&metadata, registry, actor, args, assignee, scope) {
                            task_candidate(&mut candidates, reference, args.after)?;
                        }
                    }
                    cursor = page.next;
                    let Some(next) = &cursor else { break };
                    if !seen.insert(next.clone()) {
                        return Err("provider repeated a task-list cursor".into());
                    }
                    if page_number == 63 {
                        return Err("task inventory exceeds its page bound".into());
                    }
                }
            }
        } else {
            // Registry membership is authoritative even for legacy tasks with
            // no swarm frontmatter. Do not scan other assignees' documents.
            let seats = registry.list().map_err(|e| e.to_string())?;
            let seats = seats.into_iter().filter(|seat| {
                scope.is_none_or(|swarm| seat.swarm == swarm)
                    && assignee.is_none_or(|id| id == Some(seat.id))
                    && (actor.class() == Class::Oracle
                        || actor.seat().is_some_and(|actor| actor.seat_id == seat.id))
            });
            for seat in seats {
                // Bundled providers normalize these spellings, while external
                // providers can expose legacy bare IDs as exact native values.
                for selector in [format!("seat:{}", seat.id), seat.id.to_string()] {
                    for kind in ["task", "bug", "epic"] {
                        let mut cursor = None;
                        let mut seen = BTreeSet::new();
                        for page_number in 0..64 {
                            let request = d::ListRequest {
                                filter: d::Filter {
                                    record_type: Some(kind.into()),
                                    status: args.status.clone(),
                                    assignee: Some(selector.clone()),
                                    ..Default::default()
                                },
                                cursor,
                                limit: Some(d::MAX_PAGE),
                            };
                            let page = within(deadline, || self.0.list_until(request, deadline))?;
                            for reference in page.items {
                                task_candidate(&mut candidates, reference, args.after)?;
                            }
                            cursor = page.next;
                            let Some(next) = &cursor else { break };
                            if !seen.insert(next.clone()) {
                                return Err("provider repeated a task-list cursor".into());
                            }
                            if page_number == 63 {
                                return Err("task inventory exceeds its page bound".into());
                            }
                        }
                    }
                }
            }
        }
        let mut result = Vec::new();
        for reference in candidates.into_values() {
            let document = self.fetch_record(reference, Some(deadline))?;
            if listed(&document, registry, actor, args, assignee, scope) {
                result.push(document);
                if result.len() > limit {
                    break;
                }
            }
        }
        Ok(result)
    }
}
impl ProviderTasks<'_> {
    fn fetch(&self, reference: d::RecordRef, deadline: Option<Instant>) -> Checked<Document> {
        let doc = self.fetch_record(reference, deadline)?;
        task_state(doc.metadata["status"].as_str().unwrap_or_default())?;
        Ok(doc)
    }
    fn fetch_record(
        &self,
        reference: d::RecordRef,
        deadline: Option<Instant>,
    ) -> Checked<Document> {
        let request = d::FetchRequest {
            reference: reference.clone(),
        };
        let record = match deadline {
            Some(deadline) => within(deadline, || self.0.fetch_until(request, deadline))?,
            None => self.0.fetch(request).map_err(|e| e.to_string())?,
        };
        if record.reference != reference
            || record.parse_error.is_some()
            || reference.id.parse::<Ulid>().is_err()
            || record.frontmatter["id"].as_str() != Some(reference.id.as_str())
            || reference.number == Some(0)
            || record.frontmatter["number"].as_u64() != reference.number
            || Path::new(&reference.path).is_absolute()
            || Path::new(&reference.path)
                .components()
                .any(|c| !matches!(c, Component::Normal(_)))
        {
            return Err("task requires a consistent confined record reference and valid id/type/number frontmatter".into());
        }
        let doc = Document {
            id: reference.id,
            path: reference.path,
            metadata: record.frontmatter,
            body: record.body,
            digest: reference.digest,
            error: record.parse_error,
            links: Vec::new(),
        };
        if !is_task(&doc) {
            return Err("record is not a task".into());
        }
        Ok(doc)
    }
}

/// One provider read under the task inventory deadline. A call that fails once
/// the deadline has passed reports the named deadline, never its transport timeout.
fn within<T>(deadline: Instant, read: impl FnOnce() -> d::Result<T>) -> Checked<T> {
    let exhausted = || {
        format!(
            "{DEADLINE_EXCEEDED}: task_list exceeded its {} s budget; narrow it with assignee, status or swarm_id, or page with after",
            TASK_LIST_BUDGET.as_secs()
        )
    };
    if Instant::now() >= deadline {
        return Err(exhausted());
    }
    read().map_err(|error| {
        if Instant::now() >= deadline {
            exhausted()
        } else {
            error.to_string()
        }
    })
}

fn task_candidate(
    candidates: &mut BTreeMap<String, d::RecordRef>,
    reference: d::RecordRef,
    after: Option<Ulid>,
) -> Checked<()> {
    let id = reference
        .id
        .parse::<Ulid>()
        .map_err(|_| "invalid task reference ID")?;
    if after.is_some_and(|after| id <= after) {
        return Ok(());
    }
    if let Some(old) = candidates.insert(reference.id.clone(), reference.clone())
        && old != reference
    {
        return Err("provider returned conflicting task references".into());
    }
    if candidates.len() > 4096 {
        return Err("task inventory exceeds bounded scope; narrow the source".into());
    }
    Ok(())
}

fn listed(
    doc: &Document,
    registry: &Registry,
    actor: &Actor,
    args: &List,
    assignee: Option<Option<Ulid>>,
    scope: Option<Ulid>,
) -> bool {
    is_task(doc)
        // Historical task/bug/epic Markdown can predate the native lifecycle.
        // Keep it addressable as knowledge, without poisoning the entire task page
        // or inventing an actionable state. Direct reads/transitions stay strict.
        // Origin: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
        && task_state(doc.metadata["status"].as_str().unwrap_or_default()).is_ok()
        && !scope.is_some_and(|scope| {
            doc.metadata["swarm"]
                .as_str()
                .is_some_and(|value| value != scope.to_string())
        })
        && if args.unassigned {
            doc.error.is_none()
                && doc.metadata["status"] == "open"
                && matches!(reference(doc, "assignee"), Ok(None))
        } else {
            authorize(doc, registry, actor).is_ok()
        }
        && (args.unassigned
            || scope.is_none_or(|swarm| {
                reference(doc, "assignee")
                    .ok()
                    .flatten()
                    .and_then(|seat| registry.get(seat).ok())
                    .is_some_and(|seat| seat.swarm == swarm)
                    || assignee == Some(None) && actor.seat().is_some()
                    || actor.class() == Class::Oracle
                        && reference(doc, "assignee").ok() == Some(None)
                        && (doc.metadata["swarm"] == json!(swarm)
                            || reference(doc, "delegated_by")
                                .ok()
                                .flatten()
                                .and_then(|seat| registry.get(seat).ok())
                                .is_some_and(|seat| seat.swarm == swarm))
            }))
        && args
            .status
            .as_ref()
            .is_none_or(|status| doc.metadata["status"] == *status)
        && assignee.is_none_or(|id| reference(doc, "assignee").ok() == Some(id))
        && args.after.is_none_or(|id| doc.id > id.to_string())
}

pub(crate) fn handles(tool: &str) -> bool {
    mutation(tool) || matches!(tool, "task_status" | "task_list" | "task_import_context")
}
pub(crate) fn mutation(tool: &str) -> bool {
    matches!(
        tool,
        "task_report"
            | "task_delegate"
            | "task_reassign"
            | "task_accept"
            | "task_reject"
            | "task_reconcile"
            | "task_import"
            | "artifact_claim"
    )
}
pub(crate) fn catalogue(actor: &Actor) -> Vec<Value> {
    let id = json!({"type":"string","pattern":"^[0-7][0-9A-HJKMNP-TV-Z]{25}$"});
    let record_id =
        json!({"type":"string","pattern":"^(?:[0-7][0-9A-HJKMNP-TV-Z]{25}|#[1-9][0-9]*)$"});
    let path = json!({"type":"string","minLength":1,"maxLength":4096});
    let text = json!({"type":"string","minLength":1,"maxLength":16384});
    let evidence = json!({"type":"array","maxItems":128,"items":{"type":"object","additionalProperties":false,"properties":{
        "criterion":{"type":"string","minLength":1,"maxLength":512},"reference":{"type":"string","minLength":1,"maxLength":2048}},"required":["criterion","reference"]}});
    let verification = json!({"type":"object","additionalProperties":false,"properties":{
        "command":text,"exit_code":{"type":"integer","minimum":-2147483648_i64,"maximum":2147483647_i64},"evidence":evidence},"required":["command","exit_code"]});
    let mut definitions = vec![
        (
            "task_report",
            "Report your assigned task; records evidence and wakes its delegator. Reporting never accepts a task.",
            json!({"task_path":path,"summary":text,"commit":{"type":"string","maxLength":128},"verify_result":verification}),
            vec!["task_path", "summary"],
        ),
        (
            "task_status",
            "Read an authorized task by path, ID or #N; workers see only the current assignment.",
            json!({"task_path":path,"task_id":record_id}),
            vec![],
        ),
        (
            "task_list",
            "Page tasks assigned to seats in the captured swarm; workers see only their current assignments. Only the account operator may request the read-only open unassigned inventory.",
            json!({"unassigned":{"type":"boolean"},"swarm_id":id,"status":{"type":"string","maxLength":32},"assignee":{"type":"string","maxLength":64},"after":id,"limit":{"type":"integer","minimum":1,"maximum":128}}),
            vec![],
        ),
        (
            "artifact_claim",
            "Claim artifact responsibility only, optionally linking an authorized task; this is not a document reader or editor. Use knowledge_fetch, knowledge_search and knowledge_write for document content. A claim does not verify external content.",
            json!({"kind":{"enum":["markdown","pr","binary","file","image","release","other"]},"ref":{"type":"string","minLength":1,"maxLength":4096},"digest":{"type":"string","maxLength":71},"task":path}),
            vec!["kind", "ref"],
        ),
    ];
    if actor.class() == Class::Oracle {
        definitions.extend([
            ("task_import_context", "Read the exact selected bundled knowledge root for an authenticated import; Oracle-class only, no other root is exposed.", json!({}), vec![]),
            ("task_delegate", "Assign a scoped task and send its signed brief. Omit role to use the target’s current role; an explicit role must be that exact role or an authorized legacy executor/oracle/creativity override.", json!({"task_path":path,"seat_id":id,"role":{"type":"string","minLength":1,"maxLength":128,"description":"The target's exact current role, or an authorized legacy executor/oracle/creativity override."}}), vec!["task_path","seat_id"]),
            ("task_reassign", "Move responsibility to an existing seat, releasing the old assignee and waking the new one.", json!({"task_path":path,"seat_id":id}), vec!["task_path","seat_id"]),
            ("task_accept", "Accept only a reported task with matching verify command, recorded exit zero and evidence for every declared criterion.", json!({"task_path":path}), vec!["task_path"]),
            ("task_reject", "Reject a scoped task with a recorded reason and wake its assignee.", json!({"task_path":path,"reason":text}), vec!["task_path","reason"]),
            ("task_reconcile", "Record an evidenced historical disposition as done, without fresh acceptance or historical wakes.", json!({"task_path":path,"disposition":{"enum":["implemented","superseded","cancelled"]},"reason":text,"evidence":{"type":"array","minItems":1,"maxItems":128,"items":text}}), vec!["task_path","disposition","reason","evidence"]),
            ("task_import", "Atomically import pinned local source records and audited dispositions into the exact selected knowledge root; no historical wakes.", json!({"expected_root":path,"manifest_path":path,"audit_path":path,"source_dir":path,"bundle_sha256":{"type":"string","pattern":"^[a-f0-9]{64}$"},"owner_seat":id}), vec!["expected_root","manifest_path","audit_path","source_dir","bundle_sha256","owner_seat"]),
        ]);
    }
    definitions.into_iter().map(|(name,description,mut properties,mut required)| {
        let mutating = mutation(name);
        if mutating {
            properties["op_id"] = id.clone();
            required.insert(0,"op_id");
        }
        json!({"name":name,"description":description,
            "inputSchema":{"type":"object","properties":properties,"required":required,"additionalProperties":false},
            "outputSchema":{"type":"object","additionalProperties":true},
            "annotations":{"readOnlyHint":!mutating,"destructiveHint":mutating,"idempotentHint":true,"openWorldHint":false}})
    }).collect()
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Status {
    task_path: Option<String>,
    task_id: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct List {
    #[serde(default)]
    unassigned: bool,
    swarm_id: Option<Ulid>,
    status: Option<String>,
    assignee: Option<String>,
    after: Option<Ulid>,
    limit: Option<usize>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TaskPath {
    task_path: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Delegate {
    task_path: String,
    seat_id: Ulid,
    role: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Reassign {
    task_path: String,
    seat_id: Ulid,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Report {
    task_path: String,
    summary: String,
    commit: Option<String>,
    verify_result: Option<Verification>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Reject {
    task_path: String,
    reason: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Reconcile {
    task_path: String,
    disposition: String,
    reason: String,
    evidence: Vec<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Import {
    expected_root: String,
    manifest_path: String,
    audit_path: String,
    source_dir: String,
    bundle_sha256: String,
    owner_seat: Ulid,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Verification {
    command: String,
    exit_code: i32,
    #[serde(default)]
    evidence: Vec<Evidence>,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Evidence {
    criterion: String,
    reference: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Claim {
    kind: String,
    #[serde(rename = "ref")]
    reference: String,
    digest: Option<String>,
    task: Option<String>,
}

pub(crate) enum Prepared {
    Read(Value),
    Effect(Box<Effect>),
}
pub(crate) enum Effect {
    Import {
        bundle: Box<crate::index::migration::Bundle>,
        owner: Ulid,
    },
    Task {
        document: Document,
        action: Box<Action>,
    },
    Artifact {
        claim: Claim,
        task: Option<Document>,
    },
}
pub(crate) enum Action {
    Report(Report),
    Delegate { target: SeatRecord, role: String },
    Reassign(SeatRecord),
    Accept,
    Reject(String),
    Reconcile(Reconcile),
}
pub(crate) struct Wake {
    pub id: Ulid,
    pub to: Recipient,
    pub task_id: Ulid,
    pub event: String,
    pub path: String,
    pub summary: String,
}
pub(crate) struct Plan {
    pub writes: Vec<Update>,
    pub wakes: Vec<Wake>,
    pub data: Value,
}

pub(crate) fn call(op_id: Ulid, request: TaskRequest) -> Checked<Call> {
    let tool = match &request {
        TaskRequest::ImportContext {} => "task_import_context",
        TaskRequest::List { .. } => "task_list",
        TaskRequest::Status { .. } => "task_status",
        TaskRequest::Delegate { .. } => "task_delegate",
        TaskRequest::Reassign { .. } => "task_reassign",
        TaskRequest::Accept { .. } => "task_accept",
        TaskRequest::Reject { .. } => "task_reject",
        TaskRequest::Report { .. } => "task_report",
        TaskRequest::Reconcile { .. } => "task_reconcile",
        TaskRequest::Import { .. } => "task_import",
    };
    let mut args = serde_json::to_value(request).map_err(|e| e.to_string())?;
    args.as_object_mut()
        .ok_or("task request is not an object")?
        .remove("op");
    Ok(Call {
        op_id,
        tool: tool.into(),
        args,
    })
}

pub(crate) fn response(tool: &str, result: Value) -> Checked<TaskResponse> {
    if result["isError"] == true {
        let error = &result["structuredContent"]["error"];
        return Err(format!(
            "{}: {}",
            error["code"].as_str().unwrap_or("unknown"),
            error["message"]
                .as_str()
                .unwrap_or("task outcome unavailable")
        ));
    }
    let data = &result["structuredContent"];
    match tool {
        "task_import_context" => Ok(TaskResponse::ImportContext {
            expected_root: data["expected_root"]
                .as_str()
                .ok_or("import context root missing")?
                .into(),
        }),
        "task_import" => Ok(TaskResponse::Imported {
            receipt: data["receipt"]
                .as_str()
                .ok_or("import receipt missing")?
                .into(),
            bundle_sha256: data["bundle_sha256"]
                .as_str()
                .ok_or("import digest missing")?
                .into(),
            source_records: data["source_records"]
                .as_u64()
                .ok_or("import count missing")? as usize,
            published_records: data["published_records"]
                .as_u64()
                .ok_or("published count missing")? as usize,
        }),
        "task_status" => Ok(TaskResponse::Item {
            item: serde_json::from_value::<TaskDoc>(data.clone()).map_err(|e| e.to_string())?,
        }),
        "task_list" => {
            let items = data["items"]
                .as_array()
                .ok_or("task page missing items")?
                .iter()
                .map(|item| {
                    let doc: TaskDoc =
                        serde_json::from_value(item.clone()).map_err(|e| e.to_string())?;
                    let m = &doc.metadata;
                    Ok(TaskCard {
                        id: doc.id,
                        path: doc.path,
                        title: m["title"].as_str().unwrap_or_default().into(),
                        status: m["status"].as_str().unwrap_or_default().into(),
                        assignee: m["assignee"]
                            .as_str()
                            .filter(|s| *s != "none")
                            .map(str::to_owned),
                        delegated_by: m["delegated_by"]
                            .as_str()
                            .filter(|s| *s != "none")
                            .map(str::to_owned),
                        updated: m["updated"].as_str().map(str::to_owned),
                        verify: m["verify_result"]["exit_code"]
                            .as_i64()
                            .and_then(|code| i32::try_from(code).ok())
                            .map(|exit_code| TaskVerifySummary { exit_code }),
                    })
                })
                .collect::<Checked<Vec<_>>>()?;
            let next = serde_json::from_value(data["next"].clone()).map_err(|e| e.to_string())?;
            Ok(TaskResponse::Items { items, next })
        }
        _ => Ok(TaskResponse::Applied),
    }
}

pub(crate) fn prepare(
    index: &impl TaskDocuments,
    registry: &Registry,
    actor: &Actor,
    call: &Call,
) -> Checked<Prepared> {
    let oracle = actor.class() == Class::Oracle;
    if matches!(
        call.tool.as_str(),
        "task_delegate"
            | "task_reassign"
            | "task_accept"
            | "task_reject"
            | "task_reconcile"
            | "task_import"
            | "task_import_context"
    ) && !oracle
    {
        return Err("task control requires oracle-class authority".into());
    }
    let task = |path: &str| -> Checked<Document> {
        let doc = index.locate(path)?;
        if call.tool == "task_reconcile"
            && matches!(actor, Actor::Operator { .. })
            && index.import_root().is_ok()
        {
            // The account operator archives history in the captured native root.
            // Historical owners may be absent; never invent a live assignment.
            // Seat callers retain authorize(), and transition() preserves accepted.
            reference(&doc, "assignee")?;
            delegator(&doc)?;
        } else if oracle && call.tool == "task_delegate" && reference(&doc, "assignee")?.is_none() {
            let args: Delegate = tools::parse(&call.args)?;
            let target = target(registry, actor, args.seat_id)?;
            let mut scoped = doc.clone();
            scoped.metadata["assignee"] = json!(format!("seat:{}", target.id));
            authorize(&scoped, registry, actor)?;
        } else {
            authorize(&doc, registry, actor)?;
        }
        Ok(doc)
    };
    let effect = match call.tool.as_str() {
        "task_import_context" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct Context {}
            let _: Context = tools::parse(&call.args)?;
            return Ok(Prepared::Read(
                json!({"expected_root":index.import_root()?}),
            ));
        }
        "task_import" => {
            let args: Import = tools::parse(&call.args)?;
            if !Path::new(&args.expected_root).is_absolute()
                || Path::new(&args.expected_root)
                    .canonicalize()
                    .map_err(|e| e.to_string())?
                    != index.import_root()?
            {
                return Err("import expected_root differs from selected knowledge root".into());
            }
            let owner = target(registry, actor, args.owner_seat)?;
            if owner.role.class != Class::Oracle {
                return Err("import owner must be an oracle-class seat".into());
            }
            let bundle = crate::index::migration::Bundle::load(
                Path::new(&args.manifest_path),
                Path::new(&args.audit_path),
                Path::new(&args.source_dir),
            )
            .map_err(|e| e.to_string())?;
            if bundle.digest != args.bundle_sha256 {
                return Err("import bundle digest changed".into());
            }
            Effect::Import {
                bundle: Box::new(bundle),
                owner: owner.id,
            }
        }
        "task_reconcile" => {
            let args: Reconcile = tools::parse(&call.args)?;
            bounded(&args.reason, 16384, "reconciliation reason")?;
            if !matches!(
                args.disposition.as_str(),
                "implemented" | "superseded" | "cancelled"
            ) || args.evidence.is_empty()
                || args.evidence.len() > 128
            {
                return Err("historical disposition and evidence required".into());
            }
            for reference in &args.evidence {
                bounded(reference, 2048, "reconciliation evidence")?;
            }
            Effect::Task {
                document: task(&args.task_path)?,
                action: Box::new(Action::Reconcile(args)),
            }
        }
        "task_status" => {
            let args: Status = tools::parse(&call.args)?;
            let locator = match (args.task_path, args.task_id) {
                (Some(path), None) => path,
                (None, Some(id))
                    if number_handle(&id)?.is_some()
                        || id.parse::<Ulid>().is_ok_and(|id| id != Ulid::nil()) =>
                {
                    id
                }
                _ => return Err("supply exactly one task_path or task_id".into()),
            };
            return Ok(Prepared::Read(projection(&task(&locator)?)));
        }
        "task_list" => {
            let args: List = tools::parse(&call.args)?;
            if args.unassigned {
                if !matches!(actor, Actor::Operator { .. }) {
                    return Err("unassigned task inventory requires the account operator".into());
                }
                if args.swarm_id.is_some()
                    || args.assignee.is_some()
                    || args
                        .status
                        .as_deref()
                        .is_some_and(|status| status != "open")
                {
                    return Err(
                        "unassigned inventory is open and account-wide; omit swarm_id and assignee"
                            .into(),
                    );
                }
            }
            if args
                .swarm_id
                .is_some_and(|id| actor.seat().is_some_and(|seat| id != seat.swarm_id))
            {
                return Err("task list outside caller swarm".into());
            }
            if let Some(status) = &args.status {
                task_state(status)?;
            }
            let assignee = args.assignee.as_deref().map(seat_reference).transpose()?;
            if !oracle && assignee.is_some_and(|id| id != actor.seat().map(|seat| seat.seat_id)) {
                return Err("workers list only their assigned tasks".into());
            }
            let limit = tools::page_limit(args.limit)?;
            let mut docs = index.documents(registry, actor, &args)?;
            // Provider selection is an IO optimization, never read authority.
            docs.retain(|doc| {
                listed(
                    doc,
                    registry,
                    actor,
                    &args,
                    assignee,
                    args.swarm_id
                        .or_else(|| actor.seat().map(|seat| seat.swarm_id)),
                )
            });
            docs.sort_by(|a, b| a.id.cmp(&b.id));
            let more = docs.len() > limit;
            docs.truncate(limit);
            let items: Vec<_> = docs.iter().map(projection).collect();
            return Ok(Prepared::Read(
                json!({"items":items,"next":if more { docs.last().map(|d|&d.id) } else { None }}),
            ));
        }
        "task_report" => {
            let args: Report = tools::parse(&call.args)?;
            bounded(&args.summary, 16384, "summary")?;
            if let Some(commit) = &args.commit {
                single_line(commit, 128, "commit")?;
            }
            if let Some(result) = &args.verify_result {
                result.validate()?;
            }
            Effect::Task {
                document: task(&args.task_path)?,
                action: Box::new(Action::Report(args)),
            }
        }
        "task_delegate" => {
            let args: Delegate = tools::parse(&call.args)?;
            let target = target(registry, actor, args.seat_id)?;
            let role = args.role.unwrap_or_else(|| target.role.name.clone());
            validate_task_role(&target, &role)?;
            Effect::Task {
                document: task(&args.task_path)?,
                action: Box::new(Action::Delegate { target, role }),
            }
        }
        "task_reassign" => {
            let args: Reassign = tools::parse(&call.args)?;
            Effect::Task {
                document: task(&args.task_path)?,
                action: Box::new(Action::Reassign(target(registry, actor, args.seat_id)?)),
            }
        }
        "task_accept" => {
            let args: TaskPath = tools::parse(&call.args)?;
            Effect::Task {
                document: task(&args.task_path)?,
                action: Box::new(Action::Accept),
            }
        }
        "task_reject" => {
            let args: Reject = tools::parse(&call.args)?;
            bounded(&args.reason, 16384, "reason")?;
            Effect::Task {
                document: task(&args.task_path)?,
                action: Box::new(Action::Reject(args.reason)),
            }
        }
        "artifact_claim" => {
            let claim: Claim = tools::parse(&call.args)?;
            if !matches!(
                claim.kind.as_str(),
                "markdown" | "pr" | "binary" | "file" | "image" | "release" | "other"
            ) {
                return Err("unknown artifact kind".into());
            }
            single_line(&claim.reference, 4096, "artifact reference")?;
            if claim.digest.as_deref().is_some_and(|d| {
                !d.strip_prefix("sha256:")
                    .is_some_and(|s| s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit()))
            }) {
                return Err("artifact digest must be sha256:<64 hex digits>".into());
            }
            let task = claim.task.as_deref().map(task).transpose()?;
            Effect::Artifact { claim, task }
        }
        _ => return Err("unknown work-item tool".into()),
    };
    Ok(Prepared::Effect(Box::new(effect)))
}

pub(crate) fn retained_allowed(
    index: &impl TaskDocuments,
    registry: &Registry,
    actor: &Actor,
    call: &Call,
    value: &Value,
) -> bool {
    if value["isError"] == true {
        return true;
    }
    let allowed = |item: &Value| {
        item["path"]
            .as_str()
            .and_then(|path| index.locate(path).ok())
            .is_some_and(|doc| authorize(&doc, registry, actor).is_ok())
    };
    match call.tool.as_str() {
        "task_status" => allowed(&value["structuredContent"]),
        "task_list" => value["structuredContent"]["items"]
            .as_array()
            .is_some_and(|items| items.iter().all(allowed)),
        _ => true,
    }
}

fn is_task(doc: &Document) -> bool {
    matches!(doc.metadata["type"].as_str(), Some("task" | "bug" | "epic"))
}
fn number_handle(locator: &str) -> Checked<Option<u64>> {
    let Some(digits) = locator.strip_prefix('#') else {
        return Ok(None);
    };
    digits
        .parse::<u64>()
        .ok()
        .filter(|number| *number > 0 && number.to_string() == digits)
        .map(Some)
        .ok_or_else(|| "invalid task number handle".into())
}
fn locate_index(index: &Index, locator: &str) -> Checked<Document> {
    let doc = if let Some(number) = number_handle(locator)? {
        index
            .by_number(number)
            .map_err(|_| "task number unavailable or ambiguous".to_owned())?
            .ok_or("task number unavailable")?
    } else {
        let path = if let Ok(id) = locator.parse::<Ulid>() {
            let docs: Vec<_> = index
                .documents
                .values()
                .filter(|d| d.id == id.to_string())
                .collect();
            if docs.len() != 1 {
                return Err("task ID missing or ambiguous".into());
            }
            docs[0].path.clone()
        } else {
            let path = locator.strip_prefix(".bboxy/").unwrap_or(locator);
            if path.len() > 4096
                || Path::new(path).is_absolute()
                || Path::new(path)
                    .components()
                    .any(|c| !matches!(c, Component::Normal(_)))
            {
                return Err("task path must stay inside .bboxy".into());
            }
            path.into()
        };
        index
            .current(&path)
            .map_err(|e| e.to_string())?
            .ok_or("task unavailable")?
    };
    if doc.error.is_some()
        || !is_task(&doc)
        || doc.metadata["id"]
            .as_str()
            .and_then(|id| id.parse::<Ulid>().ok())
            .is_none()
    {
        return Err("task requires valid id/type/number frontmatter".into());
    }
    if index
        .documents
        .values()
        .any(|d| d.path != doc.path && d.id == doc.id)
    {
        return Err("task ID is ambiguous".into());
    }
    task_state(doc.metadata["status"].as_str().unwrap_or_default())?;
    Ok(doc)
}
fn seat_reference(value: &str) -> Checked<Option<Ulid>> {
    if value == "none" {
        return Ok(None);
    }
    value
        .strip_prefix("seat:")
        .unwrap_or(value)
        .parse::<Ulid>()
        .ok()
        .filter(|id| *id != Ulid::nil())
        .map(Some)
        .ok_or_else(|| "task seat reference is ambiguous".into())
}
/// Shared interpretation of a present seat reference; never infer from a path.
pub(crate) fn reference(doc: &Document, field: &str) -> Checked<Option<Ulid>> {
    match doc.metadata.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(value)) => seat_reference(value),
        _ => Err(format!("task {field} must name a seat")),
    }
}
/// Keep operator and seat delegation references distinct for host read scopes.
pub(crate) fn delegator(doc: &Document) -> Checked<Option<Recipient>> {
    match doc.metadata.get("delegated_by") {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(value)) if value == "none" => Ok(None),
        Some(Value::String(value)) if value.starts_with("operator:") => {
            Recipient::try_from(value.clone())
                .map(Some)
                .map_err(str::to_owned)
        }
        Some(Value::String(value)) => seat_reference(value).map(|id| id.map(Recipient::Seat)),
        _ => Err("task delegated_by must name a seat or operator".into()),
    }
}
/// Reuse canonical task ownership and swarm checks for trusted host writes.
pub(crate) fn authorize(doc: &Document, registry: &Registry, actor: &Actor) -> Checked<()> {
    // Every present registry reference, never the directory, establishes scope.
    if doc.error.is_some() || !is_task(doc) {
        return Err("task frontmatter is invalid".into());
    }
    let assignee = reference(doc, "assignee")?;
    let delegator = delegator(doc)?.and_then(Recipient::seat);
    if assignee.is_none() && delegator.is_none() {
        return Err("task swarm is ambiguous".into());
    }
    let swarm = match actor {
        Actor::OperatorLocal { .. } => return Err("task authority requires signing custody".into()),
        Actor::Seat(actor) => actor.swarm_id,
        Actor::Operator { .. } => {
            registry
                .get(assignee.ok_or("task swarm requires an assignee")?)
                .map_err(|_| "task assignee unavailable")?
                .swarm
        }
    };
    for id in [assignee, delegator].into_iter().flatten() {
        if registry.get(id).map_err(|_| "task seat unavailable")?.swarm != swarm {
            return Err("task outside caller swarm".into());
        }
    }
    if actor.class() != Class::Oracle && assignee != actor.seat().map(|seat| seat.seat_id) {
        return Err("task is not assigned to caller".into());
    }
    Ok(())
}
fn validate_task_role(target: &SeatRecord, role: &str) -> Checked<()> {
    if (!matches!(role, "executor" | "oracle" | "creativity") && role != target.role.name)
        || (role == "oracle" && target.role.class != Class::Oracle)
    {
        return Err("requested task role exceeds target authority".into());
    }
    Ok(())
}
fn target(registry: &Registry, actor: &Actor, id: Ulid) -> Checked<SeatRecord> {
    let target = registry.get(id).map_err(|_| "target seat unavailable")?;
    if actor
        .seat()
        .is_some_and(|actor| target.swarm != actor.swarm_id)
    {
        return Err("target outside caller swarm".into());
    }
    Ok(target)
}
fn projection(doc: &Document) -> Value {
    json!({"id":doc.id,"path":format!(".bboxy/{}",doc.path),"metadata":doc.metadata,"body":tools::clip(&doc.body,16384),"body_truncated":doc.body.len()>16384})
}
fn task_state(state: &str) -> Checked<()> {
    if matches!(
        state,
        "open" | "claimed" | "running" | "reported" | "accepted" | "rejected" | "blocked" | "done"
    ) {
        Ok(())
    } else {
        Err("invalid task status".into())
    }
}
fn bounded(value: &str, bound: usize, field: &str) -> Checked<()> {
    if value.trim().is_empty()
        || value.len() > bound
        || value
            .chars()
            .any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t'))
    {
        Err(format!("invalid or oversized {field}"))
    } else {
        Ok(())
    }
}
fn single_line(value: &str, bound: usize, field: &str) -> Checked<()> {
    bounded(value, bound, field)?;
    if value.chars().any(char::is_control) {
        Err(format!("{field} must be one line"))
    } else {
        Ok(())
    }
}
impl Verification {
    fn validate(&self) -> Checked<()> {
        bounded(&self.command, 16384, "verify command")?;
        if self.evidence.len() > 128 {
            return Err("too many evidence entries".into());
        }
        let mut seen = BTreeSet::new();
        for item in &self.evidence {
            single_line(&item.criterion, 512, "criterion")?;
            bounded(&item.reference, 2048, "evidence reference")?;
            if !seen.insert(&item.criterion) {
                return Err("duplicate evidence criterion".into());
            }
        }
        Ok(())
    }
}

pub(crate) fn plan(effect: Effect, actor: &Actor, call: &Call) -> Checked<Plan> {
    match effect {
        Effect::Import { .. } => Err("import requires atomic Index publication".into()),
        Effect::Artifact { claim, task } => artifact(
            claim,
            task,
            actor.seat().ok_or("artifact claims require a seat")?,
            call,
        ),
        Effect::Task { document, action } => transition(document, *action, actor, call),
    }
}
fn transition(mut doc: Document, action: Action, actor: &Actor, call: &Call) -> Checked<Plan> {
    let assignee = reference(&doc, "assignee")?;
    let task_id: Ulid = doc.id.parse().map_err(|_| "invalid task ID")?;
    let path = format!(".bboxy/{}", doc.path);
    let mut notices = vec![];
    let current = doc.metadata["status"].as_str().unwrap_or_default();
    match action {
        Action::Reconcile(reconciliation) => {
            if current == "accepted" {
                return Err(
                    "fresh accepted verification cannot be replaced by historical reconciliation"
                        .into(),
                );
            }
            let entry = json!({"kind":"historical","fresh_accepted":false,"previous_status":current,"disposition":reconciliation.disposition,
                "reason":reconciliation.reason,"evidence":reconciliation.evidence,"by":actor.reference(),"op_id":call.op_id});
            let previous = doc
                .metadata
                .as_object_mut()
                .unwrap()
                .remove("reconciliation");
            let mut history = match previous {
                Some(Value::Array(xs)) => xs,
                Some(value) => vec![value],
                None => vec![],
            };
            history.push(entry);
            doc.metadata["reconciliation"] = json!(history);
            doc.metadata["status"] = json!("done");
        }
        Action::Report(report) => {
            let delegator = delegator(&doc)?.ok_or("task has no delegator")?;
            if matches!(current, "accepted" | "done") {
                return Err("closed task cannot be reported".into());
            }
            doc.metadata["status"] = json!("reported");
            doc.metadata["report"] = json!({"summary":report.summary,"commit":report.commit,"reported_by":actor.reference(),"op_id":call.op_id});
            if let Some(result) = &report.verify_result {
                doc.metadata["verify_result"] = json!(result);
            } else {
                doc.metadata
                    .as_object_mut()
                    .unwrap()
                    .remove("verify_result");
            }
            doc.body.push_str(&format!("\n\n## Report from {}\n\n{}\n\n```json\n{}\n```\n",actor.reference(),report.summary,serde_json::to_string_pretty(&json!({"op_id":call.op_id,"commit":report.commit,"verify_result":report.verify_result})).unwrap()));
            notices.push((
                delegator,
                "report",
                format!(
                    "Task {task_id} reported by {}: {}",
                    actor
                        .seat()
                        .map_or_else(|| actor.reference(), |seat| seat.seat_id.to_string()),
                    report.summary
                ),
            ));
        }
        Action::Delegate { target, role } => {
            if matches!(current, "accepted" | "done") {
                return Err("closed task cannot be delegated".into());
            }
            if assignee.is_some_and(|id| {
                id != target.id && actor.seat().is_none_or(|seat| id != seat.seat_id)
            }) {
                return Err("already assigned; use task_reassign".into());
            }
            doc.metadata["assignee"] = json!(format!("seat:{}", target.id));
            doc.metadata["delegated_by"] = json!(actor.reference());
            doc.metadata["role"] = json!(role);
            doc.metadata["status"] = json!("claimed");
            clear_report(&mut doc.metadata);
            notices.push((
                Recipient::Seat(target.id),
                "delegate",
                format!(
                    "ROLE: {}. Execute this: {path}\n\n{}",
                    role.to_uppercase(),
                    doc.body
                ),
            ));
        }
        Action::Reassign(target) => {
            let assignee = assignee.ok_or("task has no assignee; use task_delegate")?;
            if matches!(current, "accepted" | "done") {
                return Err("closed task cannot be reassigned".into());
            }
            if assignee == target.id {
                return Err("task is already assigned to target".into());
            }
            let role = doc.metadata["role"].as_str().unwrap_or("executor");
            validate_task_role(&target, role)?;
            notices.push((
                Recipient::Seat(assignee),
                "release",
                format!("Released from task {task_id}: {path}"),
            ));
            notices.push((
                Recipient::Seat(target.id),
                "delegate",
                format!(
                    "ROLE: {}. Execute this: {path}\n\n{}",
                    role.to_uppercase(),
                    doc.body
                ),
            ));
            doc.metadata["assignee"] = json!(format!("seat:{}", target.id));
            doc.metadata["delegated_by"] = json!(actor.reference());
            doc.metadata["status"] = json!("claimed");
            clear_report(&mut doc.metadata);
        }
        Action::Accept => {
            let assignee = assignee.ok_or("task has no assignee")?;
            if current != "reported" {
                return Err("task must be reported before acceptance".into());
            }
            acceptance(&doc)?;
            doc.metadata["status"] = json!("accepted");
            notices.push((
                Recipient::Seat(assignee),
                "accept",
                format!("Task {task_id} accepted: {path}"),
            ));
        }
        Action::Reject(reason) => {
            let assignee = assignee.ok_or("task has no assignee")?;
            doc.metadata["status"] = json!("rejected");
            doc.metadata["rejection"] = json!({"reason":reason,"by":actor.reference()});
            notices.push((
                Recipient::Seat(assignee),
                "reject",
                format!("Task {task_id} rejected: {reason}"),
            ));
        }
    }
    let wakes = notices
        .into_iter()
        .map(|(to, event, summary)| {
            bounded(&summary, 16384, "Task wake")?;
            Ok(Wake {
                id: wake_id(call.op_id, to, event),
                to,
                task_id,
                event: event.into(),
                path: path.clone(),
                summary,
            })
        })
        .collect::<Checked<Vec<_>>>()?;
    Ok(Plan {
        data: json!({"status":"accepted","task_id":task_id,"path":path,"task_status":doc.metadata["status"],"message_ids":wakes.iter().map(|w|w.id).collect::<Vec<_>>()}),
        writes: vec![Update {
            path: doc.path,
            metadata: doc.metadata,
            body: doc.body,
            expected: Some(doc.digest),
        }],
        wakes,
    })
}
fn clear_report(metadata: &mut Value) {
    let map = metadata.as_object_mut().unwrap();
    for key in ["report", "verify_result", "rejection"] {
        map.remove(key);
    }
}
fn acceptance(doc: &Document) -> Checked<()> {
    let result: Verification = serde_json::from_value(doc.metadata["verify_result"].clone())
        .map_err(|_| "acceptance requires a recorded verify_result")?;
    result.validate()?;
    let command = doc.metadata["verify"]
        .as_str()
        .filter(|s| !s.trim().is_empty())
        .ok_or("task has no verify command")?;
    if result.command != command {
        return Err("verify_result command does not match task verify".into());
    }
    if result.exit_code != 0 {
        return Err(format!(
            "verify_result exit_code is {}; acceptance requires 0",
            result.exit_code
        ));
    }
    if let Some(criteria) = doc.metadata.get("acceptance") {
        let criteria = criteria
            .as_array()
            .ok_or("acceptance criteria must be a list")?;
        let mut seen = BTreeSet::new();
        for entry in criteria {
            let name = entry
                .as_str()
                .or_else(|| entry["criterion"].as_str())
                .filter(|s| !s.trim().is_empty())
                .ok_or("invalid acceptance criterion")?;
            if !seen.insert(name) {
                return Err(format!("duplicate acceptance criterion: {name}"));
            }
            if !result.evidence.iter().any(|e| e.criterion == name) {
                return Err(format!("missing acceptance evidence: {name}"));
            }
        }
    }
    Ok(())
}
fn artifact(
    claim: Claim,
    mut task: Option<Document>,
    actor: &super::SeatActor,
    call: &Call,
) -> Checked<Plan> {
    let id = call.op_id;
    let path = format!("artifacts/{id}.md");
    let mut metadata = json!({"id":id,"type":"artifact","kind":claim.kind,"ref":claim.reference,"produced_by":format!("seat:{}", actor.seat_id),"claimed":true,"grade":"reported","for":[],"derived_from":[]});
    if let Some(digest) = claim.digest {
        metadata["digest"] = json!(digest);
    }
    if let Some(doc) = &task {
        metadata["for"] = json!([format!("[[{}]]", doc.id)]);
        metadata["derived_from"] = json!([format!("[[{}]]", doc.id)]);
    }
    let mut writes = vec![Update {
        path: path.clone(),
        metadata,
        body: format!("# Reported artifact\n\n{}\n", claim.reference),
        expected: None,
    }];
    if let Some(doc) = &mut task {
        let links = doc
            .metadata
            .as_object_mut()
            .unwrap()
            .entry("artifacts")
            .or_insert(json!([]))
            .as_array_mut()
            .ok_or("task artifacts must be a list")?;
        let link = json!(format!("[[{id}]]"));
        if !links.contains(&link) {
            links.push(link);
        }
        writes.push(Update {
            path: doc.path.clone(),
            metadata: doc.metadata.clone(),
            body: doc.body.clone(),
            expected: Some(doc.digest.clone()),
        });
    }
    Ok(Plan {
        writes,
        wakes: vec![],
        data: json!({"status":"accepted","artifact_id":id,"path":format!(".bboxy/{path}"),"grade":"reported"}),
    })
}
fn wake_id(op: Ulid, recipient: Recipient, event: &str) -> Ulid {
    let mut hash = Sha256::new();
    hash.update(b"bboxy450/task-wake/v1");
    hash.update(op.to_bytes());
    match recipient {
        Recipient::Seat(seat) => hash.update(seat.to_bytes()),
        Recipient::Operator(device) => {
            hash.update(b"operator:");
            hash.update(device.to_bytes());
        }
        Recipient::OperatorLocal(reference) => {
            hash.update(b"operator-local:");
            hash.update(reference.to_bytes());
        }
        Recipient::Plugin(plugin) => {
            hash.update(b"plugin:");
            hash.update(plugin.to_bytes());
        }
    }
    hash.update(event.as_bytes());
    Ulid::from_bytes(hash.finalize()[..16].try_into().unwrap())
}

#[cfg(test)]
mod task_role_tests {
    use super::*;
    use crate::store::sqlite::SqliteStore;
    #[test]
    #[allow(clippy::disallowed_methods, reason = "private task planning fixture")]
    fn default_and_explicit_task_briefs_keep_the_exact_architect_and_custom_role() {
        let root = tempfile::tempdir().unwrap();
        let registry = Registry::new(SqliteStore::open(&root.path().join("account")).unwrap());
        let mut target = SeatRecord::new(Ulid(2), Ulid(7), "target", "claude");
        target.role = crate::registry::Role {
            name: "architect".into(),
            class: Class::Oracle,
        };
        registry.insert(&target).unwrap();
        let mut index = Index::open(
            &root.path().join("docs"),
            SqliteStore::open(&root.path().join("index")).unwrap(),
        )
        .unwrap();
        let source = format!(
            "---\nid: {}\nnumber: 1\ntype: task\nstatus: open\nassignee: none\n---\nPreserve the assigned role.\n",
            Ulid(3)
        );
        std::fs::create_dir_all(root.path().join("docs/tasks")).unwrap();
        std::fs::write(root.path().join("docs/tasks/brief.md"), &source).unwrap();
        let document = Document::parse("tasks/brief.md", &source);
        index.documents.insert(document.path.clone(), document);
        let actor = Actor::Operator {
            device_id: Ulid(4),
            door: super::super::Door::Gui,
            issued_by: None,
        };
        for (name, class) in [
            ("architect", Class::Oracle),
            ("Code reviewer", Class::Worker),
        ] {
            registry
                .update(target.id, |record| {
                    record.role.name = name.into();
                    record.role.class = class;
                    Ok(())
                })
                .unwrap();
            for role in [None, Some(name.to_owned())] {
                let call = call(
                    Ulid::new(),
                    TaskRequest::Delegate {
                        task_path: ".bboxy/tasks/brief.md".into(),
                        seat_id: target.id,
                        role,
                    },
                )
                .unwrap();
                let Prepared::Effect(effect) = prepare(&index, &registry, &actor, &call).unwrap()
                else {
                    panic!("delegation must prepare an effect");
                };
                let plan = plan(*effect, &actor, &call).unwrap();
                assert_eq!(plan.writes[0].metadata["role"], name);
                assert!(
                    plan.wakes[0]
                        .summary
                        .starts_with(&format!("ROLE: {}.", name.to_uppercase()))
                );
            }
        }
        target.role = crate::registry::Role {
            name: "Code reviewer".into(),
            class: Class::Worker,
        };
        for role in ["oracle", "architect", "invented"] {
            assert!(validate_task_role(&target, role).is_err());
        }
        for role in ["executor", "creativity", "Code reviewer"] {
            validate_task_role(&target, role).unwrap();
        }
        let tool = catalogue(&actor)
            .into_iter()
            .find(|tool| tool["name"] == "task_delegate")
            .unwrap();
        assert_eq!(tool["inputSchema"]["properties"]["role"]["type"], "string");
        assert!(tool["inputSchema"]["properties"]["role"]["enum"].is_null());
    }
}
