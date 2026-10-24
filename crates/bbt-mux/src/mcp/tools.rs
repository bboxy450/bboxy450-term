//! Logic's filtered catalogue and bounded read projections.

use std::collections::BTreeSet;

use petgraph::visit::EdgeRef;
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::{Value, json};
use ulid::Ulid;

use super::SeatActor as Actor;
use crate::index::{Document, GraphKind, Index};
use crate::knowledge::{DataSource, data_source, views};
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
    tools.extend(super::knowledge::catalogue());
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
    views::owned(&doc.metadata, &doc.id, seat)
}

pub(crate) fn retained_documents(
    source: &dyn DataSource,
    actor: &Actor,
    seat: &SeatRecord,
    call: &super::Call,
    value: &Value,
) -> bool {
    if value["isError"] == true {
        return true;
    }
    let data = &value["structuredContent"];
    match call.tool.as_str() {
        "orient" => views::retained(source, seat, &data["tasks"]["items"], false),
        "graph_read" if actor.role.class != Class::Oracle => {
            views::retained(source, seat, &data["nodes"], true)
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
    source: &dyn DataSource,
    actor: &Actor,
    seat: &SeatRecord,
    args: &GraphRead,
    own_only: bool,
) -> std::result::Result<Value, String> {
    match args.graph.as_str() {
        "execution" => execution_graph(index, actor, seat, args, own_only),
        "task" | "markdown" => views::graph(
            source,
            seat,
            if args.graph == "task" {
                data_source::DocumentGraph::Task
            } else {
                data_source::DocumentGraph::Markdown
            },
            args.after.as_deref(),
            page_limit(args.limit)?,
            own_only || actor.role.class != Class::Oracle,
        )
        .map_err(|e| e.to_string()),
        _ => Err("graph is unavailable in this catalogue".into()),
    }
}

// Execution is host-owned, outside the document data_source contract.
fn execution_graph(
    index: &Index,
    actor: &Actor,
    seat: &SeatRecord,
    args: &GraphRead,
    own_only: bool,
) -> std::result::Result<Value, String> {
    if actor.role.class != Class::Oracle && !own_only {
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
    let graph = index
        .graphs
        .get(&GraphKind::Execution)
        .ok_or("graph unavailable")?;
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

pub(crate) fn orient_documents(
    source: &dyn DataSource,
    seat: &SeatRecord,
) -> std::result::Result<Value, String> {
    views::orient(source, seat).map_err(|e| e.to_string())
}
