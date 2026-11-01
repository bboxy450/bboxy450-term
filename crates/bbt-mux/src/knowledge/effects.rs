// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Captured source IO. Admission, signing authority and task wakes stay with logic.
// Adapted from bboxy450-term, crates/bbt-mux/src/logic.rs:
// https://github.com/bboxy450/bboxy450-term/blob/main/crates/bbt-mux/src/logic.rs
// Changes: express existing signed document/task effects through the selected source.
use super::{DataSource, WriteContext, data_source as d};
use crate::{
    mcp::{self, Actor, Call, knowledge as tool, work_items as tasks},
    registry::Registry,
};
use serde_json::Value;
use ulid::Ulid;

pub(crate) enum Prepared {
    Record {
        request: Box<d::WriteRequest>,
        report_task: Option<d::RecordRef>,
    },
    Task {
        plan: tasks::Plan,
        record: Option<String>,
    },
    Commit(d::CommitRequest),
}
pub(crate) struct Applied {
    pub value: Value,
    pub statuses: Vec<(Ulid, String)>,
    pub applied: bool,
}
fn rejected(reason: impl Into<String>) -> d::CommandError {
    d::CommandError::new("rejected", reason)
}
fn task_plan(
    source: &dyn DataSource,
    registry: &Registry,
    swarm: Ulid,
    actor: &Actor,
    call: &Call,
) -> d::Result<tasks::Plan> {
    match tasks::prepare(
        &tasks::ProviderTasks(source, (!swarm.is_nil()).then_some(swarm)),
        registry,
        actor,
        call,
    )
    .map_err(rejected)?
    {
        tasks::Prepared::Effect(effect) => tasks::plan(*effect, actor, call).map_err(rejected),
        tasks::Prepared::Read(_) => Err(rejected("effect job cannot run a task read")),
    }
}
pub(crate) fn prepare(
    source: &dyn DataSource,
    registry: &Registry,
    swarm: Ulid,
    actor: &Actor,
    call: &Call,
) -> d::Result<Prepared> {
    if tasks::mutation(&call.tool) {
        return Ok(Prepared::Task {
            plan: task_plan(source, registry, swarm, actor, call)?,
            record: None,
        });
    }
    match tool::prepare(call).map_err(rejected)? {
        tool::Request::Commit { locators, message } => {
            if actor.class() != crate::registry::Class::Oracle {
                return Err(d::CommandError::new(
                    "knowledge_commit_denied",
                    "document commit requires Oracle authority",
                ));
            }
            let refs = locators
                .into_iter()
                .map(|locator| source.resolve(d::ResolveRequest { locator }))
                .collect::<d::Result<Vec<_>>>()?;
            let request = d::CommitRequest { refs, message };
            super::commit::validate(&request)?;
            Ok(Prepared::Commit(request))
        }
        tool::Request::Write(mut request) => {
            let mut report_task = None;
            let current = request
                .reference
                .clone()
                .map(|reference| source.fetch(d::FetchRequest { reference }))
                .transpose()?;
            if current.as_ref().is_some_and(|record| {
                request
                    .etag
                    .as_ref()
                    .is_some_and(|etag| etag != &record.etag)
            }) {
                return Err(d::CommandError::new(
                    "etag_mismatch",
                    "etag does not match the current record",
                ));
            }
            if let Some(seat) = actor.seat() {
                match tool::write_plan(seat, call, *request, current.as_ref())? {
                    tool::WritePlan::Record(prepared) => request = prepared,
                    tool::WritePlan::Report(report) => {
                        let plan = task_plan(source, registry, swarm, actor, &report).map_err(
                            |error| d::CommandError::new("task_field_protected", error.message),
                        )?;
                        if !current.as_ref().is_some_and(|old| {
                            plan.writes.first().is_some_and(|update| {
                                update.path == old.reference.path
                                    && update.expected.as_ref() == Some(&old.reference.digest)
                            })
                        }) {
                            return Err(d::CommandError::new(
                                "stale_reference",
                                "task changed before Report; fetch it again",
                            ));
                        }
                        return Ok(Prepared::Task {
                            plan,
                            record: Some(current.expect("checked report reference").reference.id),
                        });
                    }
                }
                if matches!(
                    request.frontmatter["type"].as_str(),
                    Some("task" | "bug" | "epic")
                ) {
                    let document = crate::index::Document {
                        id: request.frontmatter["id"]
                            .as_str()
                            .unwrap_or_default()
                            .into(),
                        path: request
                            .reference
                            .as_ref()
                            .map(|r| r.path.as_str())
                            .or(request.path.as_deref())
                            .unwrap_or_default()
                            .into(),
                        metadata: request.frontmatter.clone(),
                        body: request.body.clone(),
                        digest: current
                            .as_ref()
                            .map(|old| old.reference.digest.clone())
                            .unwrap_or_default(),
                        error: current.as_ref().and_then(|old| old.parse_error.clone()),
                        links: Vec::new(),
                    };
                    tasks::authorize(&document, registry, actor)
                        .map_err(|reason| d::CommandError::new("task_field_protected", reason))?;
                    if let Some(old) = &current
                        && let Some(plan) =
                            tasks::amendment(document, old, call).map_err(rejected)?
                    {
                        return Ok(Prepared::Task {
                            plan,
                            record: Some(old.reference.id.clone()),
                        });
                    }
                }
            } else {
                let Actor::Operator { device_id, .. } = actor else {
                    return Err(d::CommandError::new(
                        "held",
                        "document signing requires custody",
                    ));
                };
                let kind = request.frontmatter["type"].as_str().unwrap_or_default();
                if kind.is_empty()
                    || kind.len() > 128
                    || kind.chars().any(|c| c.is_whitespace() || c.is_control())
                    || kind == "artifact"
                {
                    return Err(d::CommandError::new(
                        "knowledge_write_denied",
                        "invalid document type or artifact lineage required",
                    ));
                }
                if current
                    .as_ref()
                    .is_some_and(|record| record.frontmatter["type"] != request.frontmatter["type"])
                {
                    return Err(d::CommandError::new(
                        "type_immutable",
                        "an update must retain the current record type",
                    ));
                }
                if matches!(kind, "task" | "bug" | "epic") {
                    tool::operator_task(&mut request, *device_id)?;
                    if !swarm.is_nil() {
                        request.frontmatter["swarm"] = serde_json::json!(swarm);
                    }
                }
            }
            if request.frontmatter["type"] == "report" {
                let path = request
                    .reference
                    .as_ref()
                    .map(|r| r.path.as_str())
                    .or(request.path.as_deref())
                    .unwrap_or_default();
                if current.is_some() {
                    return Err(d::CommandError::new(
                        "report_append_only",
                        "reports are append-only; create a new report with supersedes",
                    ));
                }
                bbt_doc::reports::validate(path, &request.frontmatter)
                    .map_err(|e| d::CommandError::new("invalid_report", e))?;
                let seat = actor
                    .seat()
                    .ok_or_else(|| rejected("report author requires a seat"))?;
                if request.frontmatter["author"] != format!("seat:{}", seat.seat_id) {
                    return Err(d::CommandError::new(
                        "report_author_mismatch",
                        "report author must be the authenticated seat",
                    ));
                }
                let task = tool::fetch(
                    source,
                    request.frontmatter["task"]
                        .as_str()
                        .unwrap_or_default()
                        .into(),
                )?;
                report_task = Some(task.reference.clone());
                let document = crate::index::Document {
                    id: task.reference.id,
                    path: task.reference.path,
                    metadata: task.frontmatter,
                    body: task.body,
                    digest: task.reference.digest,
                    error: task.parse_error,
                    links: Vec::new(),
                };
                tasks::authorize(&document, registry, actor)
                    .map_err(|e| d::CommandError::new("report_task_denied", e))?;
                if let Some(previous) = request.frontmatter["supersedes"].as_str() {
                    let previous = tool::fetch(source, previous.into())?;
                    if previous.frontmatter["type"] != "report" {
                        return Err(rejected("supersedes must name a report"));
                    }
                    let previous_task = tool::fetch(
                        source,
                        previous.frontmatter["task"]
                            .as_str()
                            .unwrap_or_default()
                            .into(),
                    )?;
                    if previous_task.reference.id != document.id
                        || previous_task.reference.path != document.path
                    {
                        return Err(rejected("superseded report names a different task"));
                    }
                }
            }
            if request.reference.is_none() {
                match source.resolve(d::ResolveRequest {
                    locator: call.op_id.to_string(),
                }) {
                    Err(error) if error.code == "not_found" => (),
                    _ => {
                        return Err(rejected(
                            "knowledge create id already exists or cannot be resolved uniquely",
                        ));
                    }
                }
            }
            Ok(Prepared::Record {
                request,
                report_task,
            })
        }
        _ => Err(rejected("effect job cannot run a knowledge read")),
    }
}

pub(crate) fn apply(
    source: &dyn DataSource,
    prepared: &mut Prepared,
    call: &Call,
    context: &WriteContext<'_>,
) -> Applied {
    let mut applied = false;
    let mut statuses = Vec::new();
    let result = (|| -> d::Result<Value> {
        match prepared {
            Prepared::Record {
                request,
                report_task,
            } => {
                if let Some(reference) = report_task {
                    source
                        .fetch(d::FetchRequest {
                            reference: reference.clone(),
                        })
                        .map_err(|e| {
                            d::CommandError::new(
                                "report_task_changed",
                                format!("assigned task changed before report publication: {e}"),
                            )
                        })?;
                }
                let result = source.write(*request.clone(), context)?;
                applied = true;
                serde_json::to_value(result)
                    .map_err(|e| d::CommandError::new("unknown", e.to_string()))
            }
            Prepared::Commit(request) => {
                let result = source.commit(request.clone(), context)?;
                applied = true;
                serde_json::to_value(result)
                    .map_err(|e| d::CommandError::new("unknown", e.to_string()))
            }
            Prepared::Task { plan, record } => {
                let references = plan
                    .writes
                    .iter()
                    .map(|update| {
                        match source.resolve(d::ResolveRequest {
                            locator: update.path.clone(),
                        }) {
                            Ok(reference)
                                if update.expected.as_ref() == Some(&reference.digest) =>
                            {
                                Ok(Some(reference))
                            }
                            Err(error)
                                if error.code == "not_found" && update.expected.is_none() =>
                            {
                                Ok(None)
                            }
                            _ => Err(d::CommandError::new(
                                "conflict",
                                "document changed before write",
                            )),
                        }
                    })
                    .collect::<d::Result<Vec<_>>>()?;
                for (update, reference) in plan.writes.iter().zip(references) {
                    if update.metadata["status"] == "accepted" {
                        let document = crate::index::Document {
                            id: update.metadata["id"].as_str().unwrap_or_default().into(),
                            path: update.path.clone(),
                            metadata: update.metadata.clone(),
                            body: update.body.clone(),
                            digest: String::new(),
                            error: None,
                            links: Vec::new(),
                        };
                        tasks::linked_report(&tasks::ProviderTasks(source, None), &document)
                            .map_err(rejected)?;
                    }
                    let request = d::WriteRequest {
                        path: reference.is_none().then(|| update.path.clone()),
                        reference,
                        frontmatter: update.metadata.clone(),
                        body: update.body.clone(),
                        etag: None,
                    };
                    source.write(request, context)?;
                    applied = true;
                    if matches!(
                        update.metadata["type"].as_str(),
                        Some("task" | "bug" | "epic")
                    ) && let (Some(id), Some(status)) = (
                        update.metadata["id"]
                            .as_str()
                            .and_then(|id| id.parse().ok()),
                        update.metadata["status"].as_str(),
                    ) {
                        statuses.push((id, status.to_owned()));
                    }
                }
                if let Some(id) = record {
                    let reference = source.resolve(d::ResolveRequest {
                        locator: id.clone(),
                    })?;
                    let record = source.fetch(d::FetchRequest { reference })?;
                    serde_json::to_value(d::WriteResult {
                        record,
                        lossy: false,
                    })
                    .map_err(|e| d::CommandError::new("unknown", e.to_string()))
                } else {
                    Ok(plan.data.clone())
                }
            }
        }
    })();
    let value = match result {
        Ok(value) => mcp::knowledge::bounded(call.op_id, mcp::success(call.op_id, value), true),
        Err(error)
            if applied
                || matches!(
                    error.code.as_str(),
                    "write_not_confirmed"
                        | "write_applied_result_unavailable"
                        | "commit_not_confirmed"
                ) =>
        {
            applied = true;
            mcp::error(
                call.op_id,
                "unknown",
                &format!(
                    "{}: {}; inspect the original operation; never automatically redispatch it",
                    error.code, error.message
                ),
            )
        }
        Err(error) => mcp::error(call.op_id, &error.code, &error.message),
    };
    Applied {
        value,
        statuses,
        applied,
    }
}

#[cfg(test)]
#[allow(
    clippy::disallowed_methods,
    reason = "private report publication fixtures"
)]
mod report_tests {
    use super::*;
    use crate::{
        index::{Index, Update},
        knowledge::BboxySource,
        provenance::author::Author,
        store::sqlite::SqliteStore,
    };
    use serde_json::json;

    #[test]
    fn publication_rechecks_assigned_task_and_linked_report() {
        for change in ["none", "owner", "aba", "missing"] {
            let temp = tempfile::tempdir().unwrap();
            let root = temp.path().join(".bboxy");
            std::fs::create_dir_all(root.join("tasks")).unwrap();
            let seat = Ulid::from(10_u128);
            let task_id = Ulid::from(11_u128).to_string();
            let mut task = json!({"id":task_id,"number":1,"type":"task","status":"reported","assignee":format!("seat:{seat}")});
            let write_task = |task: &Value| {
                std::fs::write(
                    root.join("tasks/work.md"),
                    format!("---\n{task}\n---\nBrief"),
                )
                .unwrap()
            };
            write_task(&task);
            let mut index = Index::open(
                &root,
                SqliteStore::open(&temp.path().join("store")).unwrap(),
            )
            .unwrap();
            index.scan().unwrap();
            let source = BboxySource::new(&index);
            let guard = source
                .resolve(d::ResolveRequest {
                    locator: "#1".into(),
                })
                .unwrap();
            let path = "reports/2026-10-31-work.md";
            let metadata = json!({"id":Ulid::from(12_u128).to_string(),"type":"report","task":"#1","commits":[],"verify":{"command":"true","exit_code":0},"author":format!("seat:{seat}"),"created":"2026-10-31T00:00:00Z"});
            let mut prepared = Prepared::Record {
                request: Box::new(d::WriteRequest {
                    reference: None,
                    path: Some(path.into()),
                    frontmatter: metadata,
                    body: "Full evidence".into(),
                    etag: None,
                }),
                report_task: Some(guard.clone()),
            };
            match change {
                "owner" => {
                    task["assignee"] = json!(format!("seat:{}", Ulid::from(13_u128)));
                    write_task(&task);
                }
                "aba" => {
                    task["updated"] = json!("second assignment back to A");
                    write_task(&task);
                }
                "missing" => std::fs::remove_file(root.join("tasks/work.md")).unwrap(),
                _ => (),
            }
            let key = ed25519_dalek::SigningKey::from_bytes(&[7; 32]);
            let context = WriteContext {
                author: Author::seat(seat, None, &key),
            };
            let call = Call {
                op_id: Ulid::new(),
                tool: "knowledge_write".into(),
                args: json!({}),
            };
            let result = apply(&source, &mut prepared, &call, &context);
            assert_eq!(
                result.applied,
                change == "none",
                "{change}: {}",
                result.value
            );
            assert_eq!(root.join(path).exists(), change == "none");
            if change == "none" {
                // A link valid at preparation must still exist at publication.
                let mut accepted = task.clone();
                accepted["status"] = json!("accepted");
                accepted["report"] = json!({"report":"[[reports/2026-10-31-work]]"});
                let mut acceptance = Prepared::Task {
                    plan: tasks::Plan {
                        writes: vec![Update {
                            path: "tasks/work.md".into(),
                            metadata: accepted,
                            body: "Brief".into(),
                            expected: Some(guard.digest),
                        }],
                        wakes: vec![],
                        data: json!({}),
                    },
                    record: None,
                };
                std::fs::remove_file(root.join(path)).unwrap();
                let result = apply(&source, &mut acceptance, &call, &context);
                assert!(!result.applied, "{}", result.value);
                assert_eq!(
                    index.current("tasks/work.md").unwrap().unwrap().metadata["status"],
                    "reported"
                );
            }
        }
    }
}
