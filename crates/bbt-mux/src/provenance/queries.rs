// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Bounded native snapshot queries. No provider call, filesystem repair or Git.
use crate::{
    index::{Document, GraphKind, Index},
    knowledge::{data_source::CommandError, workspaces::RootBinding},
    mcp::{Call, SeatActor, tools},
    registry::{Class, SeatRecord},
    store::{Store, events::Envelope},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use petgraph::visit::EdgeRef;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

type Result<T> = std::result::Result<T, CommandError>;
const PAGE: usize = 32;
const LIMIT: usize = 64;
const SCAN: usize = 4096;
const BYTES: usize = 32 * 1024;
const RELATIONS: &[&str] = &["derived_from", "used", "evidence", "revision", "generated"];

fn fail(code: &str, message: impl Into<String>) -> CommandError {
    CommandError::new(code, message)
}
fn internal(error: impl std::fmt::Display) -> CommandError {
    fail("provenance_unavailable", error.to_string())
}
fn text(value: &Value) -> Value {
    value
        .as_str()
        .map(|v| json!(tools::clip(v, 512)))
        .unwrap_or(Value::Null)
}
fn limit(value: Option<usize>) -> Result<usize> {
    let value = value.unwrap_or(PAGE);
    if (1..=LIMIT).contains(&value) {
        Ok(value)
    } else {
        Err(fail("invalid_arguments", "limit must be 1..64"))
    }
}
fn hash(value: &[u8]) -> String {
    format!("{:x}", Sha256::digest(value))
}
fn visible(doc: &Document, actor: &SeatActor, seat: &SeatRecord) -> bool {
    let own = format!("seat:{}", actor.seat_id);
    let owned = if doc.metadata["type"] == "task" {
        tools::own_document(doc, seat)
    } else {
        doc.metadata["created_by"] == own
            || doc.metadata["updated_by"] == own
            || doc.metadata["authorship"]["signer"] == own
            || doc.metadata["type"] == "artifact" && doc.metadata["produced_by"] == own
    };
    doc.error.is_none() && (actor.role.class == Class::Oracle || owned)
}
fn catalogue<'a>(index: &'a Index, actor: &SeatActor, seat: &SeatRecord) -> Vec<&'a Document> {
    index
        .documents
        .values()
        .filter(|doc| visible(doc, actor, seat))
        .collect()
}
/// Bounded witnesses used by the control owner only for current authorization.
/// The payload and cryptographic work remain on the read worker.
pub(crate) fn authority_paths(
    index: &Index,
    call: &Call,
    value: &Value,
) -> Result<Vec<(String, String)>> {
    if value["isError"] == true {
        return Ok(Vec::new());
    }
    let data = &value["structuredContent"];
    let ids: Vec<&str> = match call.tool.as_str() {
        "provenance_search" => data["items"]
            .as_array()
            .ok_or_else(|| fail("invalid_result", "missing provenance items"))?
            .iter()
            .map(|v| {
                v["id"]
                    .as_str()
                    .ok_or_else(|| fail("invalid_result", "missing provenance identity"))
            })
            .collect::<Result<_>>()?,
        "provenance_lineage" => data["nodes"]
            .as_array()
            .ok_or_else(|| fail("invalid_result", "missing provenance nodes"))?
            .iter()
            .map(|v| {
                v["id"]
                    .as_str()
                    .ok_or_else(|| fail("invalid_result", "missing provenance identity"))
            })
            .collect::<Result<_>>()?,
        "provenance_get" => vec![
            data["entity"]["id"]
                .as_str()
                .ok_or_else(|| fail("invalid_result", "missing provenance entity"))?,
        ],
        _ => vec![
            data["entity"]
                .as_str()
                .ok_or_else(|| fail("invalid_result", "missing provenance entity"))?,
        ],
    };
    if ids.len() > LIMIT {
        return Err(fail(
            "invalid_result",
            "too many provenance authority witnesses",
        ));
    }
    let mut paths = Vec::new();
    for id in ids {
        // Unresolved lineage nodes exist only in the Oracle view. They have no
        // native record and therefore convey no owned-record read authority.
        if let Some(doc) = index.documents.values().find(|doc| doc.id == id) {
            paths.push((doc.path.clone(), doc.id.clone()));
        }
    }
    Ok(paths)
}
pub(crate) fn current_authority(
    index: &Index,
    actor: &SeatActor,
    seat: &SeatRecord,
    paths: &[(String, String)],
) -> bool {
    paths.len() <= LIMIT
        && (actor.role.class == Class::Oracle
            || paths.iter().all(|(path, id)| {
                index
                    .documents
                    .get(path)
                    .is_some_and(|doc| doc.id == *id && visible(doc, actor, seat))
            }))
}
pub(crate) fn retained(
    index: &Index,
    actor: &SeatActor,
    seat: &SeatRecord,
    call: &Call,
    value: &Value,
) -> bool {
    if actor.role.class == Class::Oracle || value["isError"] == true {
        return true;
    }
    let Ok(paths) = authority_paths(index, call, value) else {
        return false;
    };
    // A worker response can never contain unresolved nodes. Missing records
    // must not turn a revoked page into an empty, authorized witness set.
    let data = &value["structuredContent"];
    let expected = match call.tool.as_str() {
        "provenance_search" => data["items"].as_array().map_or(LIMIT + 1, Vec::len),
        "provenance_lineage" => data["nodes"].as_array().map_or(LIMIT + 1, Vec::len),
        _ => 1,
    };
    paths.len() == expected && current_authority(index, actor, seat, &paths)
}

fn resolve<'a>(docs: &[&'a Document], locator: &str) -> Result<&'a Document> {
    if locator.is_empty() || locator.len() > 4096 || locator.chars().any(char::is_control) {
        return Err(fail(
            "invalid_arguments",
            "locator must be a bounded record identifier",
        ));
    }
    let path = locator.strip_prefix(".bboxy/").unwrap_or(locator);
    let exact: Vec<_> = docs
        .iter()
        .copied()
        .filter(|doc| {
            doc.id == locator
                || doc.path == path
                || doc.path.trim_end_matches(".md") == path
                || locator
                    .strip_prefix('#')
                    .and_then(|s| s.parse::<u64>().ok())
                    .is_some_and(|n| doc.metadata["number"] == n)
        })
        .collect();
    let matches = if exact.is_empty() {
        docs.iter()
            .copied()
            .filter(|doc| {
                doc.metadata["ref"] == locator
                    || doc.metadata["digest"] == locator
                    || doc.metadata["aliases"]
                        .as_array()
                        .is_some_and(|v| v.iter().any(|s| s == locator))
            })
            .collect::<Vec<_>>()
    } else {
        exact
    };
    match matches.as_slice() {
        [doc] => Ok(*doc),
        [] => Err(fail(
            "not_found",
            "No authorized native provenance entity matches this locator. For external-provider documents, use knowledge_fetch and its checkpoint artifact ID.",
        )),
        _ => Err(fail(
            "ambiguous_locator",
            "More than one authorized entity matches; search and use an exact record ID.",
        )),
    }
}
fn summary(doc: &Document) -> Value {
    let m = &doc.metadata;
    let last = m["history"]
        .as_array()
        .and_then(|v| v.last())
        .cloned()
        .unwrap_or(Value::Null);
    json!({"id":doc.id,"path":doc.path,"type":text(&m["type"]),"title":text(&m["title"]),
        "ref":text(&m["ref"]),"record_digest":doc.digest,"digest":text(&m["digest"]),
        "deleted":m["deleted"]==true,"created_by":text(&m["created_by"]),"updated_by":text(&m["updated_by"]),
        "responsibility":{"claimed":m["claimed"]==true,"produced_by":text(&m["produced_by"]),"grade":text(&m["grade"])},
        "last_observation":{"sequence":text(&m["last_observation_sequence"]),"kind":text(&last["kind"]),"commit":text(&last["commit"])},
        "document_signer":{"principal":text(&m["authorship"]["signer"]),"kind":text(&m["authorship"]["kind"]),"public_key":text(&m["authorship"]["public_key"])},
        "verification":"not_checked"})
}
fn snapshot(docs: &[&Document]) -> String {
    let mut hash = Sha256::new();
    for doc in docs {
        hash.update((doc.id.len() as u64).to_le_bytes());
        hash.update(doc.id.as_bytes());
        hash.update((doc.digest.len() as u64).to_le_bytes());
        hash.update(doc.digest.as_bytes());
    }
    format!("{:x}", hash.finalize())
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Cursor {
    version: u8,
    scope: String,
    query: String,
    snapshot: String,
    position: u64,
    head: u64,
}
fn encode(cursor: &Cursor) -> Result<String> {
    Ok(URL_SAFE_NO_PAD.encode(serde_json::to_vec(cursor).map_err(internal)?))
}
fn decode(
    value: &str,
    scope: &str,
    query: &str,
    snapshot: &str,
    current_head: u64,
) -> Result<Cursor> {
    if value.len() > 8192 {
        return Err(fail("invalid_cursor", "cursor exceeds bound"));
    }
    let error = || {
        fail(
            "stale_cursor",
            "Provenance cursor does not match the current actor, workspace, query or snapshot. Start a new read without it.",
        )
    };
    let bytes = URL_SAFE_NO_PAD.decode(value).map_err(|_| error())?;
    let cursor: Cursor = serde_json::from_slice(&bytes).map_err(|_| error())?;
    if cursor.version != 1
        || cursor.scope != scope
        || cursor.query != query
        || cursor.snapshot != snapshot
        || cursor.head > current_head
    {
        return Err(error());
    }
    Ok(cursor)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Search {
    query: Option<String>,
    digest: Option<String>,
    seat_id: Option<crate::Ulid>,
    commit: Option<String>,
    cursor: Option<String>,
    limit: Option<usize>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Get {
    locator: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct History {
    locator: String,
    cursor: Option<String>,
    limit: Option<usize>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Lineage {
    locator: String,
    direction: Option<String>,
    depth: Option<usize>,
    limit: Option<usize>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Verify {
    locator: String,
    sequence: Option<Value>,
}
fn parse<T: serde::de::DeserializeOwned>(call: &Call) -> Result<T> {
    tools::parse(&call.args).map_err(|s| fail("invalid_arguments", s))
}

pub(crate) fn read(
    index: &Index,
    actor: &SeatActor,
    seat: &SeatRecord,
    binding: &RootBinding,
    revision: u64,
    call: &Call,
) -> Result<Value> {
    bbt_rt::assert_not_ui_thread();
    let docs = catalogue(index, actor, seat);
    let head = index.activity().last_sequence().map_err(internal)?;
    let scope = hash(
        &serde_json::to_vec(&(
            actor,
            &binding.root_id,
            binding.swarm,
            binding.revision,
            revision,
        ))
        .map_err(internal)?,
    );
    let result = match call.tool.as_str() {
        "provenance_search" => search(&docs, call, &scope, head),
        "provenance_get" => {
            let args: Get = parse(call)?;
            let doc = resolve(&docs, &args.locator)?;
            let mut value = summary(doc);
            value["publication"] = if doc.metadata["type"] == "artifact" {
                json!({"state":if doc.metadata["history"].as_array().is_some_and(|v|v.iter().any(|row|row["commit"].is_string())) {"committed"} else {"unknown"},"basis":"recorded_observation"})
            } else {
                crate::knowledge::commit::state(index, &doc.id, &doc.digest).map_err(internal)?
            };
            value["links"]=json!(doc.links.iter().filter(|l| RELATIONS.contains(&l.relation.as_str()) || l.relation=="wasAttributedTo").take(LIMIT).map(|l|json!({"to":tools::clip(&l.to,512),"relation":l.relation,"basis":"declared_link","causation_verified":false})).collect::<Vec<_>>());
            value["links_truncated"] = json!(doc.links.len() > LIMIT);
            Ok(json!({"entity":value}))
        }
        "provenance_lineage" => lineage(index, &docs, actor, call),
        "provenance_history" => history(index, &docs, call, &scope, head),
        "provenance_verify" => verify(index, &docs, call, head),
        _ => Err(fail("unknown_tool", "unknown provenance tool")),
    }?;
    let cursor = index
        .mirror()
        .get("provenance/artifact-projection")
        .map_err(internal)?
        .and_then(|v| serde_json::from_slice::<u64>(&v).ok());
    let mut result = result;
    result["freshness"] = json!({"basis":"cached_native_index","event_head":head.to_string(),"artifact_projection_sequence":cursor.map(|v|v.to_string()),
        "projection_pending":cursor.is_none_or(|v|v<head),"repair_triggered":false});
    result["scope"] = json!({"seat":actor.seat_id,"class":actor.role.class,"visibility":if actor.role.class==Class::Oracle {"current_root"} else {"own_records"}});
    if serde_json::to_vec(&result).map_err(internal)?.len() > BYTES + 4096 {
        return Err(fail(
            "result_too_large",
            "Use a smaller limit for this provenance page.",
        ));
    }
    Ok(result)
}
fn search(docs: &[&Document], call: &Call, scope: &str, head: u64) -> Result<Value> {
    let args: Search = parse(call)?;
    let limit = limit(args.limit)?;
    if args.query.as_ref().is_some_and(|v| v.len() > 1024)
        || args.digest.as_ref().is_some_and(|v| v.len() > 128)
        || args.commit.as_ref().is_some_and(|v| v.len() > 256)
    {
        return Err(fail(
            "invalid_arguments",
            "provenance search filter exceeds bound",
        ));
    }
    let terms = args.query.as_deref().unwrap_or("").to_lowercase();
    let query = hash(
        &serde_json::to_vec(&(
            "search",
            &terms,
            &args.digest,
            args.seat_id,
            &args.commit,
            limit,
        ))
        .map_err(internal)?,
    );
    let snap = snapshot(docs);
    let start = args
        .cursor
        .as_deref()
        .map(|v| decode(v, scope, &query, &snap, head))
        .transpose()?
        .map_or(0, |v| v.position);
    let start = usize::try_from(start).map_err(internal)?;
    if start > docs.len() {
        return Err(fail("invalid_cursor", "cursor is outside this snapshot"));
    }
    let mut at = start;
    let mut items = Vec::new();
    while at < docs.len() && at - start < SCAN && items.len() < limit {
        let doc = docs[at];
        let m = &doc.metadata;
        let matches = (terms.is_empty()
            || [
                &doc.id,
                &doc.path,
                m["title"].as_str().unwrap_or(""),
                m["ref"].as_str().unwrap_or(""),
            ]
            .iter()
            .any(|v| v.to_lowercase().contains(&terms)))
            && args
                .digest
                .as_ref()
                .is_none_or(|v| m["digest"] == *v || doc.digest == *v)
            && args.seat_id.is_none_or(|v| {
                m["produced_by"] == format!("seat:{v}")
                    || m["created_by"] == format!("seat:{v}")
                    || m["updated_by"] == format!("seat:{v}")
            })
            && args.commit.as_ref().is_none_or(|v| {
                m["history"]
                    .as_array()
                    .is_some_and(|rows| rows.iter().any(|r| r["commit"] == *v))
            });
        if matches {
            items.push(summary(doc));
            if serde_json::to_vec(&items).map_err(internal)?.len() > BYTES - 2048 {
                items.pop();
                if items.is_empty() {
                    return Err(fail(
                        "result_too_large",
                        "Native entity summary exceeds the provenance page bound.",
                    ));
                }
                break;
            }
        }
        at += 1;
    }
    let next = if at < docs.len() {
        Some(encode(&Cursor {
            version: 1,
            scope: scope.into(),
            query,
            snapshot: snap.clone(),
            position: at as u64,
            head,
        })?)
    } else {
        None
    };
    Ok(
        json!({"items":items,"next":next,"snapshot":snap,"scanned":at-start,"complete":next.is_none()}),
    )
}
fn lineage(index: &Index, docs: &[&Document], actor: &SeatActor, call: &Call) -> Result<Value> {
    use petgraph::Direction::{Incoming, Outgoing};
    let args: Lineage = parse(call)?;
    let origin = resolve(docs, &args.locator)?;
    let depth = args.depth.unwrap_or(2);
    let limit = limit(args.limit)?;
    if !(1..=8).contains(&depth) {
        return Err(fail("invalid_arguments", "depth must be 1..8"));
    }
    let direction = args.direction.as_deref().unwrap_or("upstream");
    if !["upstream", "downstream"].contains(&direction) {
        return Err(fail(
            "invalid_arguments",
            "direction must be upstream or downstream",
        ));
    }
    let known: BTreeMap<_, _> = docs.iter().map(|doc| (doc.id.as_str(), *doc)).collect();
    let mut seen = BTreeSet::from([origin.id.clone()]);
    let mut edges = Vec::new();
    let mut edge_keys = BTreeSet::new();
    let mut frontier = BTreeSet::new();
    let mut bytes = serde_json::to_vec(&summary(origin))
        .map_err(internal)?
        .len()
        + 2048;
    let mut scanned = 0;
    if let Some(graph) = index.graphs.get(&GraphKind::Provenance)
        && let Some(root) = graph.node_indices().find(|n| graph[*n] == origin.id)
    {
        let mut queue = VecDeque::from([(root, 0)]);
        'walk: while let Some((node, level)) = queue.pop_front() {
            for side in [Outgoing, Incoming] {
                for edge in graph.edges_directed(node, side) {
                    if scanned == 1024 {
                        frontier.insert(graph[node].clone());
                        frontier.extend(queue.iter().map(|(node, _)| graph[*node].clone()));
                        break 'walk;
                    }
                    scanned += 1;
                    let link = edge.weight();
                    if !RELATIONS.contains(&link.relation.as_str()) {
                        continue;
                    }
                    let source = &graph[edge.source()];
                    let target = &graph[edge.target()];
                    if actor.role.class != Class::Oracle
                        && (!known.contains_key(source.as_str())
                            || !known.contains_key(target.as_str()))
                    {
                        continue;
                    }
                    let (input, output) = if link.relation == "generated" {
                        (source, target)
                    } else {
                        (target, source)
                    };
                    let (from, to) = if direction == "upstream" {
                        (output, input)
                    } else {
                        (input, output)
                    };
                    if *from != graph[node] {
                        continue;
                    }
                    let value = json!({"input":input,"output":output,"source":source,"target":target,
                        "relation":link.relation,"basis":"declared_link","causation_verified":false});
                    let key = serde_json::to_string(&value).map_err(internal)?;
                    if edge_keys.contains(&key) {
                        continue;
                    }
                    let node_bytes = if seen.contains(to) {
                        0
                    } else {
                        known.get(to.as_str()).map_or(to.len() + 100, |doc| {
                            serde_json::to_vec(&summary(doc)).map_or(BYTES, |v| v.len())
                        })
                    };
                    if level >= depth
                        || !seen.contains(to) && seen.len() >= limit
                        || edges.len() >= LIMIT * 2
                        || bytes + key.len() + node_bytes > BYTES - 2048
                    {
                        frontier.insert(graph[node].clone());
                        continue;
                    }
                    bytes += key.len() + node_bytes;
                    edge_keys.insert(key);
                    edges.push(value);
                    if seen.insert(to.clone()) {
                        let next = if graph[edge.source()] == *to {
                            edge.source()
                        } else {
                            edge.target()
                        };
                        queue.push_back((next, level + 1));
                    }
                }
            }
        }
    }
    let nodes: Vec<_> = seen
        .into_iter()
        .map(|id| {
            known.get(id.as_str()).map_or_else(
                || json!({"id":id,"type":"unresolved","verification":"not_checked"}),
                |doc| summary(doc),
            )
        })
        .collect();
    Ok(
        json!({"root":origin.id,"direction":direction,"depth":depth,"nodes":nodes,"edges":edges,
        "frontier":frontier,"truncated":!frontier.is_empty(),"scanned_edges":scanned,"snapshot":snapshot(docs)}),
    )
}
fn sequence(value: &Value) -> Option<u64> {
    value.as_u64().or_else(|| value.as_str()?.parse().ok())
}
fn event_summary(event: &Envelope) -> crate::store::Result<Value> {
    let payload = &event.event.payload;
    let mut facts = serde_json::Map::new();
    for key in [
        "activity",
        "kind",
        "status",
        "path",
        "task",
        "commit",
        "assignee",
        "body_digest",
        "frontmatter_digest",
        "prepared",
        "basis",
    ] {
        if payload[key].is_string() {
            facts.insert(key.into(), text(&payload[key]));
        } else if payload[key].is_boolean() || payload[key].is_number() {
            facts.insert(key.into(), payload[key].clone());
        }
    }
    if let Some(used) = payload["used"].as_array() {
        facts.insert(
            "used".into(),
            json!(
                used.iter()
                    .filter_map(Value::as_str)
                    .take(16)
                    .map(|v| tools::clip(v, 256))
                    .collect::<Vec<_>>()
            ),
        );
        facts.insert("used_truncated".into(), json!(used.len() > 16));
    }
    let change = &payload["observation"];
    Ok(
        json!({"id":event.event.id,"sequence":event.seq.to_string(),"time_ms":event.event.ts.to_string(),"subject":tools::clip(&event.event.subject,512),"source":tools::clip(&event.event.source,512),
        "actor_seat":event.event.seat,"actor_generation":event.event.generation.map(|v|v.to_string()),"signer_public_key":base64::engine::general_purpose::STANDARD.encode(event.provenance.signer),
        "event_hash":event.hash()?,"previous_hash":event.prev,"recorded_fields":facts,
        "observation":{"kind":text(&change["kind"]),"reference":text(&change["reference"]),"digest":text(&change["digest"]),"commit":text(&change["commit"]),"attribution":{"seat":text(&change["attribution"]["seat"]),"generation":change["attribution"]["generation"].as_u64(),"grade":text(&change["attribution"]["grade"]),"reason":text(&change["attribution"]["reason"]) }},
        "integrity":"signature_columns_and_local_predecessor_verified","account_trust":"not_checked","artifact_content":"not_checked"}),
    )
}
fn history(
    index: &Index,
    docs: &[&Document],
    call: &Call,
    scope: &str,
    head: u64,
) -> Result<Value> {
    let args: History = parse(call)?;
    let doc = resolve(docs, &args.locator)?;
    let limit = limit(args.limit)?;
    let query = hash(&serde_json::to_vec(&("history", &doc.id, limit)).map_err(internal)?);
    let cursor = args
        .cursor
        .as_deref()
        .map(|v| decode(v, scope, &query, &doc.id, head))
        .transpose()?;
    let after = cursor.as_ref().map_or(0, |v| v.position);
    let captured = cursor.as_ref().map_or(head, |v| v.head);
    if after > captured {
        return Err(fail(
            "invalid_cursor",
            "history cursor exceeds captured head",
        ));
    }
    let page = index
        .activity()
        .entity_page(&doc.id, after, captured, limit)
        .map_err(|e| fail("integrity_failed", e.to_string()))?;
    let mut items = Vec::new();
    let mut position = after;
    let mut more = page.more;
    for event in &page.events {
        items.push(event_summary(event).map_err(internal)?);
        if serde_json::to_vec(&items).map_err(internal)?.len() > BYTES - 2048 {
            items.pop();
            more = true;
            break;
        }
        position = event.seq;
    }
    let next = if more {
        Some(encode(&Cursor {
            version: 1,
            scope: scope.into(),
            query,
            snapshot: doc.id.clone(),
            position,
            head: captured,
        })?)
    } else {
        None
    };
    Ok(
        json!({"entity":doc.id,"items":items,"next":next,"event_head":captured.to_string(),"complete":next.is_none(),"full_history_audited":false}),
    )
}
fn document_signature(doc: &Document) -> Value {
    use ed25519_dalek::{Signature, Verifier, VerifyingKey};
    let outcome = (|| -> Option<bool> {
        let public = base64::engine::general_purpose::STANDARD
            .decode(doc.metadata["authorship"]["public_key"].as_str()?)
            .ok()?;
        let public: [u8; 32] = public.try_into().ok()?;
        let key = VerifyingKey::from_bytes(&public).ok()?;
        let sig = Signature::from_slice(
            &base64::engine::general_purpose::STANDARD
                .decode(doc.metadata["provenance"]["sig"].as_str()?)
                .ok()?,
        )
        .ok()?;
        let mut front = doc.metadata.clone();
        front.as_object_mut()?.remove("provenance");
        let front_hash = hash(serde_yaml::to_string(&front).ok()?.as_bytes());
        let body_hash = hash(doc.body.as_bytes());
        let payload = format!("{front_hash}{body_hash}");
        let domain = "bboxy/item/v1";
        let pae = format!(
            "DSSEv1 {} {domain} {} {payload}",
            domain.len(),
            payload.len()
        );
        Some(
            doc.metadata["provenance"]["digest"] == format!("sha256:{body_hash}")
                && doc.metadata["provenance"]["frontmatter_digest"]
                    == format!("sha256:{front_hash}")
                && key.verify(pae.as_bytes(), &sig).is_ok(),
        )
    })();
    let declared = doc.metadata["provenance"]["sig"].is_string()
        || doc.metadata["authorship"]["public_key"].is_string();
    json!({"status":match outcome {Some(true)=>"valid",Some(false)=>"invalid",None if declared=>"invalid",None=>"unavailable"},"basis":"cached_native_document","account_trust":"not_checked"})
}
fn verify(index: &Index, docs: &[&Document], call: &Call, head: u64) -> Result<Value> {
    let args: Verify = parse(call)?;
    let doc = resolve(docs, &args.locator)?;
    let explicit =
        args.sequence
            .as_ref()
            .map(|value| {
                if value.as_str().is_some_and(|text| {
                    text.len() > 20 || !text.bytes().all(|c| c.is_ascii_digit())
                }) {
                    return Err(fail(
                        "invalid_arguments",
                        "sequence must be a positive u64 decimal",
                    ));
                }
                sequence(value).filter(|n| *n > 0).ok_or_else(|| {
                    fail(
                        "invalid_arguments",
                        "sequence must be a positive u64 decimal",
                    )
                })
            })
            .transpose()?;
    let selected = explicit
        .or_else(|| sequence(&doc.metadata["last_observation_sequence"]))
        .or_else(|| sequence(&doc.metadata["provenance"]["event"]["sequence"]));
    if selected == Some(0) {
        return Err(fail("invalid_arguments", "sequence must be positive"));
    }
    let event = match selected {
        Some(seq) if seq <= head => match index.activity().entity_page(&doc.id, seq - 1, seq, 1) {
            Ok(page) => match page.events.first() {
                Some(event) => {
                    let observation = &event.event.payload["observation"];
                    let matches = if observation.is_object() {
                        observation["artifact_id"] == doc.id
                            && observation["digest"] == doc.metadata["digest"]
                    } else {
                        let mut front = doc.metadata.clone();
                        front
                            .as_object_mut()
                            .ok_or_else(|| fail("invalid_record", "frontmatter is not an object"))?
                            .remove("provenance");
                        let front_hash =
                            hash(serde_yaml::to_string(&front).map_err(internal)?.as_bytes());
                        let public = base64::engine::general_purpose::STANDARD
                            .decode(
                                doc.metadata["authorship"]["public_key"]
                                    .as_str()
                                    .unwrap_or(""),
                            )
                            .ok();
                        event.event.payload["body_digest"] == hash(doc.body.as_bytes())
                            && event.event.payload["frontmatter_digest"] == front_hash
                            && public.as_deref() == Some(event.provenance.signer.as_slice())
                            && doc.metadata["provenance"]["event"]["id"]
                                == event.event.id.to_string()
                            && doc.metadata["provenance"]["event"]["hash"]
                                == event.hash().map_err(internal)?
                    };
                    json!({"status":"valid","recorded_state_matches":matches,"comparison_basis":if observation.is_object() {"artifact_identity_and_digest"} else {"prepared_document_hashes_signer_and_anchor"},"evidence":event_summary(event).map_err(internal)?})
                }
                None => {
                    json!({"status":"unavailable","sequence":seq.to_string(),"reason":"no matching entity event"})
                }
            },
            Err(_) => {
                json!({"status":"invalid","sequence":seq.to_string(),"reason":"canonical event integrity check failed"})
            }
        },
        _ => json!({"status":"unavailable","reason":"no canonical event anchor in this snapshot"}),
    };
    Ok(
        json!({"entity":doc.id,"document_signature":document_signature(doc),"event":event,
        "limits":{"historical_chain":"not_audited","account_key_trust":"not_checked","external_artifact_bytes":"not_checked","lineage_causation":"not_proven","repair_triggered":false}}),
    )
}

#[cfg(test)]
#[path = "queries/tests.rs"]
mod tests;
