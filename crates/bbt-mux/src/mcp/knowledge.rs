// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Seat knowledge tools use the shared provider and the decision #266 table.
use std::path::{Component, Path};

use serde::Deserialize;
use serde_json::{Value, json};

use super::{Call, SeatActor};
use crate::knowledge::{DataSource, data_source as d, types::DOCUMENT_TYPES};
use crate::registry::Class;

/// What every knowledge result's identifiers mean (review 5004 finding 4).
const IDENTIFIERS: &str = "Identifiers: id is the bboxy ULID, external_id the provider's native ID (the GitKB UUID), slug the root-relative path without .md.";
/// The locator forms that address one document.
const LOCATOR: &str =
    "A locator is a slug, a path ending .md, #N, the id (ULID) or the external_id.";

pub(crate) fn handles(tool: &str) -> bool {
    matches!(
        tool,
        "knowledge_fetch" | "knowledge_search" | "knowledge_write" | "knowledge_commit"
    )
}
pub(crate) fn catalogue(actor: &SeatActor) -> Vec<Value> {
    let contract = d::method_schema("write", false).expect("contract write schema");
    let mut write = contract["$defs"]["WriteRequest"].clone();
    // Advertise only this tool's fields and referenced type, like the other
    // MCP catalogues. Create/update conditions are enforced by prepare/Index.
    write.as_object_mut().unwrap().remove("allOf");
    write["$defs"] = json!({"RecordRef":contract["$defs"]["RecordRef"]});
    let definition = &mut write;
    definition["additionalProperties"] = json!(false);
    definition["properties"]
        .as_object_mut()
        .unwrap()
        .remove("signed_record");
    definition["properties"]["op_id"] =
        json!({"type":"string","pattern":"^[0-7][0-9A-HJKMNP-TV-Z]{25}$"});
    definition["required"]
        .as_array_mut()
        .unwrap()
        .push(json!("op_id"));
    let type_schema = if actor.role.class == Class::Oracle {
        json!({"type":"string","minLength":1,"maxLength":128,"pattern":"^\\S+$"})
    } else {
        json!({"type":"string","enum":WORKER_TYPES})
    };
    definition["properties"]["frontmatter"]["properties"] = json!({"type":type_schema});
    definition["properties"]["frontmatter"]["required"] = json!(["type"]);
    definition["properties"]["frontmatter"]["description"] = json!(format!(
        "Type-specific metadata; call knowledge_types for guidance. Reports require task ([[locator]] or #N), author (seat:{} for this authenticated seat), created (timestamp), commits (full SHA list, possibly empty), and verify (command plus integer exit_code).",
        actor.seat_id
    ));
    let write_help = if actor.role.class == Class::Oracle {
        "Write root-relative Markdown as your seat. Protected document types require your Oracle class. Create a decision at decisions/<slug>.md; the Index returns its numbered final path. Reports are append-only at reports/YYYY-MM-DD-slug.md; corrections create a new report with supersedes. Tasks are created open; their delegating seat may amend only brief/verify/allowed_paths while nonterminal. Amendments wake the assignee in the task thread; changed verify requires a fresh report. Lifecycle and assignment use task_*. Artifact claim/observation/lineage fields use their existing tools. Updates retain type, ref and etag."
    } else {
        "Create/update e2e, comment, research, note and artifact records as your seat. Create append-only reports at reports/YYYY-MM-DD-slug.md for your assigned task; corrections create a new report with supersedes. For your assigned task preserve every other field and the body, changing only report.summary, report.commit, report.report and verify_result; this runs task_report. Artifact claims/observations/lineage use their existing tools. Updates retain type, ref and etag."
    };
    let write_help = format!(
        "{write_help} First call knowledge_orient to load the selected provider's skill and writing workflow. Call knowledge_types for the built-in type dictionary, purposes, paths and writing guidance; concern records use concerns/<slug>.md. Research is a supported type: use research/<topic>.md with parent/children for its document tree, derived_from for origins, and links with rel used/evidence for cited sources. Specs and tasks retain derived_from links to their research or spec; these relations feed the provenance graph. Reports require task ([[locator]] or #N), author (your authenticated seat:ULID), created (a timestamp), commits (full SHA list, possibly empty), and verify (command plus integer exit_code). The host stamps authorship and a signed mutation audit. Writes are drafts, visible to other harnesses but not artifacts. Call knowledge_commit only for cross-machine sharing preparation, a substantial checkpoint, or final accepted/done work; it never pushes. Later edits become draft again. Create takes a document-type path (tasks/slug.md, specs/slug.md, concerns/slug.md, etc.); update takes the ref a fetch or search returned. Top-level slugs are refused. {IDENTIFIERS}"
    );
    let kinds: Vec<_> = DOCUMENT_TYPES.iter().map(|entry| entry.kind).collect();
    let mut tools = vec![
        json!({"name":"knowledge_types","description":"Read the built-in document type dictionary: short purposes, recommended paths and instructions for writing each type. Omit type to list all, or supply a type such as concern, spec or report. This host help is available independently of the selected knowledge provider; it grants no write authority.",
            "inputSchema":{"type":"object","properties":{"type":{"type":"string","enum":kinds}},"required":[],"additionalProperties":false},
            "outputSchema":{"type":"object","additionalProperties":true},
            "annotations":{"readOnlyHint":true,"destructiveHint":false,"idempotentHint":true,"openWorldHint":false}}),
        json!({"name":"knowledge_status","description":"Inspect this swarm's selected knowledge provider, root and revision, available providers, plugin activation/check state and failures. Read-only; trust, check, enable and selection use operator controls.",
            "inputSchema":{"type":"object","properties":{},"required":[],"additionalProperties":false},
            "outputSchema":{"type":"object","additionalProperties":true},
            "annotations":{"readOnlyHint":true,"destructiveHint":false,"idempotentHint":true,"openWorldHint":false}}),
        json!({"name":"knowledge_fetch","description":format!("Read a knowledge-root document. {LOCATOR} The body comes in pages of at most 16 KiB: while truncated is true, call again with next as body_offset and the returned etag. Other truncated projections are flagged. GitKB records also carry relationships (references, depends_on, blocks, parent) as git-kb show reports them. {IDENTIFIERS}"),
            "inputSchema":{"type":"object","properties":{"locator":{"type":"string","minLength":1,"maxLength":4096},"body_offset":{"type":"integer","minimum":0},"etag":{"type":"string","maxLength":4096}},"required":["locator"],"additionalProperties":false},
            "outputSchema":{"type":"object","additionalProperties":true},
            "annotations":{"readOnlyHint":true,"destructiveHint":false,"idempotentHint":true,"openWorldHint":false}}),
        json!({"name":"knowledge_search","description":format!("Search the current knowledge root using plain text and type:/status:/since: terms. Results are bounded pages; next is an opaque cursor for this query/root snapshot. Each item's ref addresses its document. {IDENTIFIERS}"),
            "inputSchema":{"type":"object","properties":{"query":{"type":"string","maxLength":4096},"cursor":{"type":"string","maxLength":16384},"limit":{"type":"integer","minimum":1,"maximum":128}},"required":["query"],"additionalProperties":false},
            "outputSchema":{"type":"object","additionalProperties":true},
            "annotations":{"readOnlyHint":true,"destructiveHint":false,"idempotentHint":true,"openWorldHint":false}}),
        json!({"name":"knowledge_write","description":write_help,
            "inputSchema":write,"outputSchema":{"type":"object","additionalProperties":true},
            "annotations":{"readOnlyHint":false,"destructiveHint":false,"idempotentHint":true,"openWorldHint":false}}),
    ];
    tools.push(json!({"name":"knowledge_commit","description":format!("Create an explicit committed artifact checkpoint for named documents. Commit to prepare cross-machine sharing, preserve a substantial chunk of changes, or record finally accepted/done work; do not commit each edit. Draft edits stay visible without artifact processing. Later edits become draft again. Built-in knowledge uses a private local Git repository; GitKB uses its native commit operation. Workers commit their own records/assigned tasks; Oracle may commit shared records. Never pushes or shares remotely by itself. Each of locators: {LOCATOR} {IDENTIFIERS}"),
            "inputSchema":{"type":"object","additionalProperties":false,"properties":{"op_id":{"type":"string"},"locators":{"type":"array","minItems":1,"maxItems":32,"items":{"type":"string","minLength":1,"maxLength":4096}},"message":{"type":"string","minLength":1,"maxLength":4096}},"required":["op_id","locators","message"]},
            "outputSchema":{"type":"object","additionalProperties":true},"annotations":{"readOnlyHint":false,"idempotentHint":true,"destructiveHint":false,"openWorldHint":false}}));
    tools
}

/// Static host guidance; never route this read through a workspace or provider.
pub(crate) fn type_info(args: &Value) -> Result<Value, String> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Query {
        #[serde(rename = "type")]
        kind: Option<String>,
    }
    let query: Query = super::tools::parse(args)?;
    let entries: Vec<_> = DOCUMENT_TYPES
        .iter()
        .filter(|entry| query.kind.as_deref().is_none_or(|kind| kind == entry.kind))
        .collect();
    if entries.is_empty() {
        return Err("unknown built-in document type; omit type to list the dictionary".into());
    }
    Ok(json!({
        "types":entries,
        "writing":{
            "create":"Call knowledge_write with a root-relative Markdown path under a document-type directory (such as concerns/session-continuity.md), frontmatter containing type and title, and a Markdown body. The host assigns identity, number, timestamps, authorship and signatures; do not invent them.",
            "update":"Fetch the current record with knowledge_fetch, then pass its ref and etag to knowledge_write, preserving type and other frontmatter. Omit path on update.",
            "publication":"Draft -> committed checkpoint -> draft on later edits. Use knowledge_write freely for work in progress. Use knowledge_commit deliberately to prepare cross-machine sharing, preserve substantial document changes, or record finally accepted/done work. Commit creates artifact provenance; it does not push. knowledge_fetch exposes publication_state and a matching checkpoint.",
            "authority":"Follow your role-filtered knowledge_write schema and task_* or artifact_* controls. This dictionary describes types and conventions; it grants no permissions and a selected provider may refuse unsupported writes.",
            "custom_types":"Oracle-class writes may also use custom types; these entries describe the built-in vocabulary rather than a closed storage schema."
        }
    }))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Fetch {
    locator: String,
    #[serde(default)]
    body_offset: usize,
    etag: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Search {
    query: String,
    cursor: Option<String>,
    limit: Option<usize>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Write {
    #[serde(default, rename = "ref")]
    reference: Option<d::RecordRef>,
    #[serde(default)]
    path: Option<String>,
    frontmatter: Value,
    body: String,
    #[serde(default)]
    etag: Option<String>,
}
pub(crate) enum Request {
    Fetch(String),
    Search(d::SearchRequest),
    Write(Box<d::WriteRequest>),
    Commit {
        locators: Vec<String>,
        message: String,
    },
}

fn permitted(path: &str) -> bool {
    let path = Path::new(path);
    !path.is_absolute()
        && !path.as_os_str().is_empty()
        && path.as_os_str().len() <= 4096
        && path.extension().is_some_and(|ext| ext == "md")
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

pub(crate) fn prepare(call: &Call) -> Result<Request, String> {
    match call.tool.as_str() {
        "knowledge_commit" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct Commit {
                locators: Vec<String>,
                message: String,
            }
            let args: Commit = super::tools::parse(&call.args)?;
            if args.locators.is_empty()
                || args.locators.len() > 32
                || args
                    .locators
                    .iter()
                    .any(|s| s.trim().is_empty() || s.len() > 4096)
                || args.message.trim().is_empty()
                || args.message.len() > 4096
            {
                return Err("commit requires explicit bounded locators and message".into());
            }
            Ok(Request::Commit {
                locators: args.locators,
                message: args.message,
            })
        }
        "knowledge_fetch" => {
            let args: Fetch = super::tools::parse(&call.args)?;
            if args.locator.is_empty() || args.locator.len() > 4096 {
                return Err("knowledge locator is empty or exceeds its bound".into());
            }
            Ok(Request::Fetch(args.locator))
        }
        "knowledge_search" => {
            let args: Search = super::tools::parse(&call.args)?;
            if args.query.len() > 4096
                || args
                    .cursor
                    .as_ref()
                    .is_some_and(|cursor| cursor.len() > 16384)
                || args
                    .limit
                    .is_some_and(|limit| !(1..=d::MAX_PAGE).contains(&limit))
            {
                return Err("knowledge search query, cursor or limit exceeds its bound".into());
            }
            Ok(Request::Search(d::SearchRequest {
                query: args.query,
                cursor: args.cursor,
                limit: args.limit,
            }))
        }
        "knowledge_write" => {
            let mut args: Write = super::tools::parse(&call.args)?;
            let path = args
                .reference
                .as_ref()
                .map(|r| r.path.as_str())
                .or(args.path.as_deref())
                .ok_or("knowledge create requires a root-relative Markdown path")?;
            if !permitted(path) {
                return Err(
                    "knowledge write requires a relative Markdown path inside the root".into(),
                );
            }
            if args.reference.is_some() && args.path.is_some() {
                return Err("knowledge update uses ref, without a path".into());
            }
            let metadata = args
                .frontmatter
                .as_object_mut()
                .ok_or("frontmatter must be an object")?;
            if metadata
                .get("type")
                .and_then(Value::as_str)
                .is_none_or(str::is_empty)
            {
                return Err("knowledge write requires a record type".into());
            }
            crate::knowledge::validate_document_path(
                path,
                metadata["type"].as_str().unwrap_or_default(),
            )
            .map_err(|error| format!("{}: {}", error.code, error.message))?;
            let id = args
                .reference
                .as_ref()
                .map_or_else(|| call.op_id.to_string(), |r| r.id.clone());
            if id.parse::<crate::Ulid>().is_err() || id == crate::Ulid::nil().to_string() {
                return Err("knowledge record reference requires a non-nil record id".into());
            }
            if metadata
                .get("id")
                .is_some_and(|value| value.as_str() != Some(id.as_str()))
            {
                return Err("knowledge create id must equal op_id; update id cannot change".into());
            }
            metadata.insert("id".into(), json!(id));
            Ok(Request::Write(Box::new(d::WriteRequest {
                reference: args.reference,
                path: args.path,
                frontmatter: args.frontmatter,
                body: args.body,
                etag: args.etag,
            })))
        }
        _ => Err("knowledge tool is unavailable".into()),
    }
}

pub(crate) fn fetch_page_args(call: &Call) -> Result<(usize, Option<String>), String> {
    let args: Fetch = super::tools::parse(&call.args)?;
    if args.etag.as_ref().is_some_and(|e| e.len() > 4096) {
        return Err("etag exceeds bound".into());
    }
    Ok((args.body_offset, args.etag))
}

pub(crate) fn fetch_page(
    source: &dyn DataSource,
    locator: String,
    call: &Call,
) -> d::Result<Value> {
    let (offset, etag) =
        fetch_page_args(call).map_err(|e| d::CommandError::new("invalid_arguments", e))?;
    let value = serde_json::to_value(fetch(source, locator)?)
        .map_err(|e| d::CommandError::new("invalid_record", e.to_string()))?;
    super::read_pages::body_page(value, offset, etag.as_deref())
}

pub(crate) fn fetch(source: &dyn DataSource, locator: String) -> d::Result<d::Record> {
    // Do not reflect outside-root candidates or parse failures into a scoped
    // tool response. The lower provider still retains its full command error.
    let target = bbt_doc::links::locator(&locator);
    let target = if target.starts_with('#') {
        target
    } else {
        target.split_once('#').map_or(target, |(path, _)| path)
    };
    let locator = target.strip_prefix(".bboxy/").unwrap_or(target).to_owned();
    let reference = source
        .resolve(d::ResolveRequest { locator })
        .map_err(|error| {
            d::CommandError::new(
                &error.code,
                format!("knowledge lookup refused: {}", error.code),
            )
        })?;
    if !permitted(&reference.path) {
        return Err(d::CommandError::new(
            "invalid_path",
            "knowledge reads require a relative Markdown path inside the root",
        ));
    }
    let record = source
        .fetch(d::FetchRequest { reference })
        .map_err(|error| {
            d::CommandError::new(
                &error.code,
                format!("knowledge fetch refused: {}", error.code),
            )
        })?;
    Ok(record)
}

const WORKER_TYPES: &[&str] = &[
    "e2e", "comment", "research", "note", "artifact", "report", "task", "bug", "epic",
];

pub(crate) enum WritePlan {
    Record(Box<d::WriteRequest>),
    Report(Call),
}

fn denied(code: &str, reason: &str) -> d::CommandError {
    d::CommandError::new(code, reason)
}

/// Authorize the actual current and proposed record, never its directory name.
/// Task reports remain canonical task transitions; artifacts retain their lineage.
pub(crate) fn write_plan(
    actor: &SeatActor,
    call: &Call,
    mut request: d::WriteRequest,
    current: Option<&d::Record>,
) -> d::Result<WritePlan> {
    let kind = request.frontmatter["type"].as_str().unwrap_or_default();
    if current.is_some_and(|old| old.frontmatter["type"] != kind) {
        return Err(denied(
            "type_immutable",
            "an update must retain the current record type",
        ));
    }
    if kind.is_empty()
        || kind.len() > 128
        || kind.chars().any(|c| c.is_whitespace() || c.is_control())
        || (actor.role.class == Class::Worker && !WORKER_TYPES.contains(&kind))
    {
        return Err(denied(
            "knowledge_write_denied",
            "record type is outside the caller's write table",
        ));
    }
    if kind == "report" && current.is_some() {
        return Err(denied(
            "report_append_only",
            "reports are append-only; create a new report with supersedes",
        ));
    }
    if matches!(kind, "task" | "bug" | "epic") {
        if actor.role.class == Class::Worker {
            let old = current.ok_or_else(|| {
                denied(
                    "task_field_protected",
                    "task creation requires Oracle-class authority",
                )
            })?;
            if request.body != old.body
                || task_report_remainder(&request.frontmatter)?
                    != task_report_remainder(&old.frontmatter)?
            {
                return Err(denied(
                    "task_field_protected",
                    "only report.summary, report.commit, report.report and verify_result may change; the task brief is protected",
                ));
            }
            let mut args = json!({"task_path":old.reference.path,"summary":request.frontmatter["report"]["summary"]});
            if let Some(commit) = request.frontmatter["report"].get("commit") {
                args["commit"] = commit.clone();
            }
            if let Some(report) = request.frontmatter["report"].get("report") {
                args["report"] = report.clone();
            }
            if let Some(verification) = request.frontmatter.get("verify_result") {
                args["verify_result"] = verification.clone();
            }
            return Ok(WritePlan::Report(Call {
                op_id: call.op_id,
                tool: "task_report".into(),
                args,
            }));
        }
        if let Some(old) = current {
            let mut before = old.frontmatter.clone();
            let mut after = request.frontmatter.clone();
            for field in ["verify", "allowed_paths"] {
                before
                    .as_object_mut()
                    .expect("record frontmatter")
                    .remove(field);
                after
                    .as_object_mut()
                    .expect("validated frontmatter")
                    .remove(field);
            }
            if !matches!(
                old.frontmatter["status"].as_str(),
                Some("open" | "claimed" | "running" | "reported" | "rejected" | "blocked")
            ) || old.frontmatter["delegated_by"] != format!("seat:{}", actor.seat_id)
                || before != after
            {
                return Err(denied(
                    "task_field_protected",
                    "only the delegating Oracle seat may amend a nonterminal task's brief, verify and allowed_paths; use task_* for lifecycle and assignment",
                ));
            }
        } else {
            let metadata = request
                .frontmatter
                .as_object_mut()
                .expect("validated frontmatter");
            if metadata.get("status").is_none() {
                metadata.insert("status".into(), json!("open"));
            }
            if metadata["status"] != "open"
                || metadata
                    .get("assignee")
                    .is_some_and(|value| !value.is_null() && value != "none")
                || metadata
                    .get("delegated_by")
                    .is_some_and(|value| value != &json!(format!("seat:{}", actor.seat_id)))
            {
                return Err(denied(
                    "task_field_protected",
                    "create tasks open and unassigned; delegation uses task_delegate",
                ));
            }
            metadata.insert(
                "delegated_by".into(),
                json!(format!("seat:{}", actor.seat_id)),
            );
            metadata.insert("swarm".into(), json!(actor.swarm_id));
        }
    } else if kind == "artifact" {
        artifact_fields(
            actor,
            &mut request.frontmatter,
            current.map(|old| &old.frontmatter),
        )?;
    }
    Ok(WritePlan::Record(Box::new(request)))
}

/// The operator's task create (ctl `task create`, #266 addendum): open and unassigned under
/// `tasks/`, with a title and a verify command, delegated by the operator device. Edits and
/// lifecycle keep the task_* tools; delegation assigns it later.
pub(crate) fn operator_task(request: &mut d::WriteRequest, device: crate::Ulid) -> d::Result<()> {
    let protected = |message: &str| Err(denied("task_field_protected", message));
    if request.reference.is_some() {
        return protected("operator task edits use the task_* tools");
    }
    if !request
        .path
        .as_deref()
        .is_some_and(|path| path.starts_with("tasks/"))
    {
        return protected("an operator task is created under tasks/");
    }
    let delegator = json!(format!("operator:{device}"));
    let metadata = request
        .frontmatter
        .as_object_mut()
        .expect("validated frontmatter");
    for field in ["title", "verify"] {
        if metadata
            .get(field)
            .and_then(Value::as_str)
            .is_none_or(|v| v.trim().is_empty())
        {
            return protected("a task needs a title and a verify command");
        }
    }
    metadata.entry("status").or_insert_with(|| json!("open"));
    metadata.entry("assignee").or_insert_with(|| json!("none"));
    if metadata["status"] != "open"
        || !(metadata["assignee"].is_null() || metadata["assignee"] == "none")
    {
        return protected("create tasks open and unassigned; delegation uses task delegate");
    }
    if metadata
        .get("delegated_by")
        .is_some_and(|value| *value != delegator)
    {
        return protected("an operator task is delegated by the operator device");
    }
    metadata.insert("delegated_by".into(), delegator);
    Ok(())
}

fn task_report_remainder(metadata: &Value) -> d::Result<Value> {
    let mut value = metadata.clone();
    let object = value
        .as_object_mut()
        .ok_or_else(|| denied("task_field_protected", "task frontmatter must be an object"))?;
    object.remove("verify_result");
    if let Some(report) = object.get_mut("report") {
        let report = report
            .as_object_mut()
            .ok_or_else(|| denied("task_field_protected", "report must be an object"))?;
        report.remove("summary");
        report.remove("commit");
        report.remove("report");
        if report.is_empty() {
            object.remove("report");
        }
    }
    Ok(value)
}

fn artifact_fields(actor: &SeatActor, metadata: &mut Value, old: Option<&Value>) -> d::Result<()> {
    // The observation marker cannot be cleared to make ref/digest mutable again.
    let protected = [
        "claimed",
        "produced_by",
        "grade",
        "history",
        "for",
        "derived_from",
        "observed",
    ];
    if let Some(old) = old {
        if protected
            .iter()
            .any(|field| metadata.get(*field) != old.get(*field))
        {
            return Err(denied(
                "artifact_field_protected",
                "claim, observation and lineage fields require their existing artifact/detector path",
            ));
        }
        if ["ref", "digest"]
            .iter()
            .any(|field| metadata.get(*field) != old.get(*field))
            && (old["claimed"] == true
                || old["observed"] == true
                || old["produced_by"] != format!("seat:{}", actor.seat_id))
        {
            return Err(denied(
                "artifact_field_protected",
                "only the creator may correct ref/digest while unclaimed and unobserved",
            ));
        }
    } else {
        let defaults = json!({"claimed":false,"produced_by":format!("seat:{}",actor.seat_id),"grade":"inferred","history":[],"for":[],"derived_from":[],"observed":false});
        let object = metadata.as_object_mut().expect("validated frontmatter");
        for field in protected {
            let expected = defaults.get(field).expect("all protected defaults");
            if object.get(field).is_some_and(|value| value != expected) {
                return Err(denied(
                    "artifact_field_protected",
                    "a generic artifact create is unclaimed; claim and lineage fields are host-owned",
                ));
            }
            object.insert(field.into(), expected.clone());
        }
    }
    Ok(())
}

pub(crate) fn bounded(op: crate::Ulid, value: Value, applied: bool) -> Value {
    match serde_json::to_vec(&value) {
        Ok(bytes) if bytes.len() <= super::MAX_RESULT => value,
        _ if applied => super::error(
            op,
            "unknown",
            "knowledge write applied but its result is unavailable; inspect the original operation, never redispatch it",
        ),
        _ => super::error(
            op,
            "response_bound",
            "knowledge result exceeds the MCP response bound",
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messaging::wire::IssuerKind;
    use crate::registry::Role;

    #[test]
    fn document_type_prefix_is_required_for_create_and_update() {
        for update in [false, true] {
            let mut args = json!({"frontmatter":{"type":"note"},"body":"body"});
            if update {
                args["ref"] = json!({"id":crate::Ulid::from(4_u128).to_string(),"number":1,"slug":"a","path":"a.md","digest":"0".repeat(64)});
            } else {
                args["path"] = json!("a.md");
            }
            let call = Call {
                op_id: crate::Ulid::from(5_u128),
                tool: "knowledge_write".into(),
                args,
            };
            assert!(
                prepare(&call)
                    .err()
                    .unwrap()
                    .contains("document_type_prefix_required")
            );
        }
    }

    // keeps: review 5004 finding 4 — every knowledge tool says which identifiers it takes.
    #[test]
    fn every_knowledge_tool_names_the_identifiers_it_accepts() {
        let actor = SeatActor {
            seat_id: crate::Ulid::from(1_u128),
            process_generation: 1,
            swarm_id: crate::Ulid::from(2_u128),
            role: Role {
                name: "oracle".into(),
                class: Class::Oracle,
            },
            grant_epoch: 1,
            issuer_kind: IssuerKind::LocalDev,
        };
        let tools = catalogue(&actor);
        for name in [
            "knowledge_fetch",
            "knowledge_search",
            "knowledge_write",
            "knowledge_commit",
        ] {
            let tool = tools.iter().find(|t| t["name"] == name).unwrap();
            let text = tool["description"].as_str().unwrap();
            for term in ["ULID", "external_id", "slug"] {
                assert!(
                    text.contains(term),
                    "{name} description lacks {term}: {text}"
                );
            }
        }
        let fetch = tools
            .iter()
            .find(|t| t["name"] == "knowledge_fetch")
            .unwrap();
        assert!(fetch["description"].as_str().unwrap().contains("truncated"));
    }

    #[test]
    #[allow(clippy::disallowed_methods, reason = "private strict-provider fixture")]
    fn wiki_report_links_reach_a_strict_provider_as_record_locators() {
        use crate::knowledge::{BboxySource, WriteContext};
        struct Strict<'a>(BboxySource<'a>);
        impl DataSource for Strict<'_> {
            fn resolve(&self, r: d::ResolveRequest) -> d::Result<d::RecordRef> {
                if !["tasks/work", "tasks/work.md", "#3096"].contains(&r.locator.as_str()) {
                    return Err(d::CommandError::new(
                        "invalid_path",
                        "strict provider requires an unwrapped locator",
                    ));
                }
                self.0.resolve(r)
            }
            fn fetch(&self, r: d::FetchRequest) -> d::Result<d::Record> {
                self.0.fetch(r)
            }
            fn search(&self, r: d::SearchRequest) -> d::Result<d::Page<d::SearchHit>> {
                self.0.search(r)
            }
            fn list(&self, r: d::ListRequest) -> d::Result<d::Page<d::RecordRef>> {
                self.0.list(r)
            }
            fn links(&self, r: d::FetchRequest) -> d::Result<d::Links> {
                self.0.links(r)
            }
            fn graph(&self, r: d::GraphRequest) -> d::Result<d::GraphPage> {
                self.0.graph(r)
            }
            fn write(&self, r: d::WriteRequest, c: &WriteContext<'_>) -> d::Result<d::WriteResult> {
                self.0.write(r, c)
            }
            fn changes_since(&self, r: d::ChangesRequest) -> d::Result<d::Changes> {
                self.0.changes_since(r)
            }
        }
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join(".bboxy");
        std::fs::create_dir_all(root.join("tasks")).unwrap();
        let id = crate::Ulid::from(11_u128).to_string();
        let metadata = json!({"id":id,"number":3096,"type":"task","status":"open","verify":"true"});
        std::fs::write(
            root.join("tasks/work.md"),
            format!("---\n{metadata}\n---\nTask body"),
        )
        .unwrap();
        let mut index = crate::index::Index::open(
            &root,
            crate::store::sqlite::SqliteStore::open(&temp.path().join("store")).unwrap(),
        )
        .unwrap();
        index.scan().unwrap();
        let source = Strict(BboxySource::new(&index));
        for locator in [
            "[[tasks/work]]",
            "[[tasks/work|Task]]",
            "[[.bboxy/tasks/work.md#Evidence|Task]]",
            "[[#3096]]",
            "#3096",
        ] {
            let record =
                fetch(&source, locator.into()).unwrap_or_else(|e| panic!("{locator}: {e}"));
            assert_eq!(record.reference.id, id);
        }
        assert!(fetch(&source, "[[../outside]]".into()).is_err());
    }
}
