//! Logic's filtered catalogue and bounded read projections.

use std::collections::BTreeSet;

use petgraph::visit::EdgeRef;
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::{Value, json};
use ulid::Ulid;

use super::SeatActor as Actor;
use crate::index::{Document, GraphKind, Index};
use crate::registry::{Class, SeatRecord};

pub(crate) fn catalogue(actor: &Actor) -> Value {
    let oracle = actor.role.class == Class::Oracle;
    let id = json!({"type":"string","pattern":"^[0-7][0-9A-HJKMNP-TV-Z]{25}$"});
    let scope = if oracle {
        json!(["swarm", "account"])
    } else {
        json!(["swarm"])
    };
    let graphs = if oracle {
        json!(["execution", "task", "markdown"])
    } else {
        json!(["task", "markdown"])
    };
    let definitions = [
        (
            "orient",
            "Read your authenticated role, own task briefs, own channel tree and execution context. Delivery evidence is separate from unread state.",
            json!({}),
            json!([]),
        ),
        (
            "self_status",
            "Read your seat record, current grant epoch, adapter state/grade and lifecycle.",
            json!({}),
            json!([]),
        ),
        (
            "seat_list",
            "List owned seat metadata in your swarm; oracle-class may request another swarm or account scope.",
            json!({"scope":{"type":"string","enum":scope},"swarm_id":id,"after":id,"limit":{"type":"integer","minimum":1,"maximum":128}}),
            json!([]),
        ),
        (
            "seat_get",
            "Read authorized seat metadata and activity; this tool never returns terminal output or a capability.",
            json!({"seat_id":id}),
            json!(["seat_id"]),
        ),
        (
            "graph_read",
            "Read the in-memory execution, task or markdown projection; workers see only their own items in task/markdown.",
            json!({"graph":{"type":"string","enum":graphs},"after":{"type":"string","maxLength":256},"limit":{"type":"integer","minimum":1,"maximum":128}}),
            json!(["graph"]),
        ),
    ];
    let mut tools: Vec<Value> = definitions.into_iter().map(|(name,description,properties,required)|json!({
        "name":name,"description":description,
        "inputSchema":{"type":"object","properties":properties,"required":required,"additionalProperties":false},
        "outputSchema":{"type":"object","additionalProperties":true},
        "annotations":{"readOnlyHint":true,"destructiveHint":false,"idempotentHint":true,"openWorldHint":false}
    })).collect();
    tools.extend(super::messaging::catalogue(&super::Actor::from(actor)));
    tools.extend(super::work_items::catalogue(&super::Actor::from(actor)));
    tools.extend(super::seats::catalogue(actor));
    json!({"tools":tools})
}

pub(crate) fn parse<T: DeserializeOwned>(value: &Value) -> std::result::Result<T, String> {
    serde_json::from_value(value.clone())
        .map_err(|_| "arguments do not match the tool schema".into())
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Empty {}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SeatList {
    pub scope: Option<String>,
    pub swarm_id: Option<Ulid>,
    pub after: Option<Ulid>,
    pub limit: Option<usize>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SeatGet {
    pub seat_id: Ulid,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct GraphRead {
    pub graph: String,
    pub after: Option<String>,
    pub limit: Option<usize>,
}
pub(crate) fn page_limit(limit: Option<usize>) -> std::result::Result<usize, String> {
    let limit = limit.unwrap_or(32);
    if !(1..=128).contains(&limit) {
        Err("limit must be 1..128".into())
    } else {
        Ok(limit)
    }
}
pub(crate) fn own_document(doc: &Document, seat: &SeatRecord) -> bool {
    // A current assignment overrides a stale historical registry task list.
    if let Some(assignee) = doc.metadata.get("assignee") {
        return assignee
            .as_str()
            .is_some_and(|id| id == seat.id.to_string() || id == format!("seat:{}", seat.id));
    }
    seat.tasks.iter().any(|id| id.to_string() == doc.id)
}

pub(crate) fn retained_documents(
    index: &Index,
    actor: &Actor,
    seat: &SeatRecord,
    call: &super::Call,
    value: &Value,
) -> bool {
    if value["isError"] == true {
        return true;
    }
    let document_allowed = |item: &Value| {
        item["path"]
            .as_str()
            .and_then(|path| index.documents.get(path))
            .is_some_and(|doc| doc.id == item["id"] && own_document(doc, seat))
    };
    let data = &value["structuredContent"];
    match call.tool.as_str() {
        "orient" => data["tasks"]["items"]
            .as_array()
            .is_some_and(|items| items.iter().all(document_allowed)),
        "graph_read" if actor.role.class != Class::Oracle => {
            data["nodes"].as_array().is_some_and(|items| {
                items.iter().all(|item| {
                    if item["path"].is_null() {
                        item["id"] == seat.id.to_string()
                            || item["id"] == format!("seat:{}", seat.id)
                    } else {
                        document_allowed(item)
                    }
                })
            })
        }
        _ => true,
    }
}
pub(crate) fn clip(text: &str, bound: usize) -> &str {
    let mut n = bound.min(text.len());
    while !text.is_char_boundary(n) {
        n -= 1;
    }
    &text[..n]
}

pub(crate) fn graph(
    index: &Index,
    actor: &Actor,
    seat: &SeatRecord,
    args: &GraphRead,
    own_only: bool,
) -> std::result::Result<Value, String> {
    let kind = match args.graph.as_str() {
        "execution" => GraphKind::Execution,
        "task" => GraphKind::Task,
        "markdown" => GraphKind::Markdown,
        _ => return Err("graph is unavailable in this catalogue".into()),
    };
    if kind == GraphKind::Execution && actor.role.class != Class::Oracle && !own_only {
        return Err("execution graph requires oracle-class authority".into());
    }
    let limit = page_limit(args.limit)?;
    if args.after.as_ref().is_some_and(|s| s.len() > 256) {
        return Err("graph cursor exceeds bound".into());
    }
    let restricted = own_only || actor.role.class != Class::Oracle;
    let mut allowed: BTreeSet<String> = index
        .documents
        .values()
        .filter(|doc| own_document(doc, seat))
        .map(|doc| doc.id.clone())
        .collect();
    allowed.extend([seat.id.to_string(), format!("seat:{}", seat.id)]);
    let graph = index.graphs.get(&kind).ok_or("graph unavailable")?;
    let names: BTreeSet<_> = graph
        .node_weights()
        .filter(|id| {
            (!restricted || allowed.contains(*id))
                && args.after.as_ref().is_none_or(|after| *id > after)
        })
        .cloned()
        .collect();
    let more = names.len() > limit;
    let selected: BTreeSet<_> = names.into_iter().take(limit).collect();
    let nodes:Vec<_>=selected.iter().map(|id|{
        let doc=index.documents.values().find(|doc|doc.id==*id);
        json!({"id":id,"path":doc.map(|d|&d.path),"title":doc.and_then(|d|d.metadata["title"].as_str()).map(|s|clip(s,512)),"type":doc.and_then(|d|d.metadata["type"].as_str())})
    }).collect();
    let mut edges = Vec::new();
    let mut edge_more = false;
    for edge in graph.edge_references() {
        let from = &graph[edge.source()];
        let to = &graph[edge.target()];
        if !selected.contains(from) || !selected.contains(to) {
            continue;
        }
        if edges.len() == 512 {
            edge_more = true;
            break;
        }
        edges.push(json!({"from":from,"to":to,"relation":edge.weight().relation,"weight":edge.weight().weight}));
    }
    Ok(
        json!({"graph":args.graph,"nodes":nodes,"edges":edges,"next":if more {selected.last()}else{None},"edges_truncated":edge_more}),
    )
}

pub(crate) fn orient_documents(index: &Index, seat: &SeatRecord) -> Value {
    let docs: Vec<_> = index
        .documents
        .values()
        .filter(|doc| own_document(doc, seat))
        .collect();
    json!({"items":docs.iter().take(8).map(|doc|{
        let verify=&doc.metadata["verify"];
        json!({"id":doc.id,"path":doc.path,"title":doc.metadata["title"].as_str().map(|s|clip(s,512)),
            "status":doc.metadata["status"],"verify":if verify.to_string().len()<=4096{verify.clone()}else{json!({"unavailable":"verify exceeds card bound; read the document"})},
            "body":clip(&doc.body,16*1024),"body_truncated":doc.body.len()>16*1024,"parse_error":doc.error})
    }).collect::<Vec<_>>(),"truncated":docs.len()>8})
}
