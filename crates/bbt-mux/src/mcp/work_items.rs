//! Typed work-item transitions. Markdown is authority; MCP journals attempts.

use std::collections::BTreeSet;
use std::path::{Component, Path};

use bbt_proto::messaging::{
    Recipient, TaskCard, TaskDoc, TaskRequest, TaskResponse, TaskVerifySummary,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use ulid::Ulid;

use super::{Actor, Call, tools};
use crate::index::{Document, Index, Update};
use crate::registry::{Class, Registry, SeatRecord};

type Checked<T> = std::result::Result<T, String>;

pub(crate) fn handles(tool: &str) -> bool {
    mutation(tool) || matches!(tool, "task_status" | "task_list")
}
pub(crate) fn mutation(tool: &str) -> bool {
    matches!(
        tool,
        "task_report"
            | "task_delegate"
            | "task_reassign"
            | "task_accept"
            | "task_reject"
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
            "Page authorized tasks in your swarm; worker results contain only their current assignments.",
            json!({"swarm_id":id,"status":{"type":"string","maxLength":32},"assignee":{"type":"string","maxLength":64},"after":id,"limit":{"type":"integer","minimum":1,"maximum":128}}),
            vec![],
        ),
        (
            "artifact_claim",
            "Record your reported responsibility for an artifact, optionally linking an authorized task. This does not verify external content.",
            json!({"kind":{"enum":["markdown","pr","binary","file","image","release","other"]},"ref":{"type":"string","minLength":1,"maxLength":4096},"digest":{"type":"string","maxLength":71},"task":path}),
            vec!["kind", "ref"],
        ),
    ];
    if actor.class() == Class::Oracle {
        definitions.extend([
            ("task_delegate", "Assign a scoped task to an existing seat and send its signed Task brief.", json!({"task_path":path,"seat_id":id,"role":{"enum":["executor","oracle","creativity"]}}), vec!["task_path","seat_id"]),
            ("task_reassign", "Move responsibility to an existing seat, releasing the old assignee and waking the new one.", json!({"task_path":path,"seat_id":id}), vec!["task_path","seat_id"]),
            ("task_accept", "Accept only a reported task with matching verify command, recorded exit zero and evidence for every declared criterion.", json!({"task_path":path}), vec!["task_path"]),
            ("task_reject", "Reject a scoped task with a recorded reason and wake its assignee.", json!({"task_path":path,"reason":text}), vec!["task_path","reason"]),
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
struct List {
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
        TaskRequest::List { .. } => "task_list",
        TaskRequest::Status { .. } => "task_status",
        TaskRequest::Delegate { .. } => "task_delegate",
        TaskRequest::Reassign { .. } => "task_reassign",
        TaskRequest::Accept { .. } => "task_accept",
        TaskRequest::Reject { .. } => "task_reject",
        TaskRequest::Report { .. } => "task_report",
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
    index: &Index,
    registry: &Registry,
    actor: &Actor,
    call: &Call,
) -> Checked<Prepared> {
    let oracle = actor.class() == Class::Oracle;
    if matches!(
        call.tool.as_str(),
        "task_delegate" | "task_reassign" | "task_accept" | "task_reject"
    ) && !oracle
    {
        return Err("task control requires oracle-class authority".into());
    }
    let task = |path: &str| -> Checked<Document> {
        let doc = locate(index, path)?;
        if actor.seat().is_none()
            && call.tool == "task_delegate"
            && reference(&doc, "assignee")?.is_none()
        {
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
            let mut docs: Vec<_> = index
                .documents
                .values()
                .filter(|d| {
                    is_task(d)
                        && authorize(d, registry, actor).is_ok()
                        && (actor.seat().is_some()
                            || args.swarm_id.is_none_or(|swarm| {
                                reference(d, "assignee")
                                    .ok()
                                    .flatten()
                                    .and_then(|seat| registry.get(seat).ok())
                                    .is_some_and(|seat| seat.swarm == swarm)
                            }))
                        && args
                            .status
                            .as_ref()
                            .is_none_or(|s| d.metadata["status"] == *s)
                        && assignee.is_none_or(|id| reference(d, "assignee").ok() == Some(id))
                        && args.after.is_none_or(|id| d.id > id.to_string())
                })
                .collect();
            docs.sort_by(|a, b| a.id.cmp(&b.id));
            let more = docs.len() > limit;
            docs.truncate(limit);
            return Ok(Prepared::Read(
                json!({"items":docs.iter().map(|d|projection(d)).collect::<Vec<_>>(),"next":if more { docs.last().map(|d|&d.id) } else { None }}),
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
            let role = args.role.unwrap_or_else(|| {
                if target.role.class == Class::Oracle {
                    "oracle"
                } else {
                    "executor"
                }
                .into()
            });
            if !matches!(role.as_str(), "executor" | "oracle" | "creativity")
                || (role == "oracle" && target.role.class != Class::Oracle)
            {
                return Err("requested task role exceeds target authority".into());
            }
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
    index: &Index,
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
            .and_then(|path| locate(index, path).ok())
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
    matches!(doc.metadata["type"].as_str(), Some("task" | "bug"))
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
fn locate(index: &Index, locator: &str) -> Checked<Document> {
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
fn reference(doc: &Document, field: &str) -> Checked<Option<Ulid>> {
    match doc.metadata.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(value)) => seat_reference(value),
        _ => Err(format!("task {field} must name a seat")),
    }
}
fn delegator(doc: &Document) -> Checked<Option<Recipient>> {
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
fn authorize(doc: &Document, registry: &Registry, actor: &Actor) -> Checked<()> {
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
            if role == "oracle" && target.role.class != Class::Oracle {
                return Err("requested task role exceeds target authority".into());
            }
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
    }
    hash.update(event.as_bytes());
    Ulid::from_bytes(hash.finalize()[..16].try_into().unwrap())
}
