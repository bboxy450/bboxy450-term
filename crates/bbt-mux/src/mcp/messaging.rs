// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Thin typed mappings to W2.4; sender, scope and delivery policy stay in logic.

use std::collections::BTreeMap;

use serde::Deserialize;
use serde_json::{Value, json};
use ulid::Ulid;

use super::{Actor, Call, tools};
use crate::{
    messaging::{
        api::{Request, Response},
        tree::{EntryKind, Filter, Node, Rate, Right, normalize},
        wire::{MessageKind, SendRequest, SendRequestExt},
    },
    registry::Class,
};

pub(crate) fn mutation(tool: &str) -> bool {
    matches!(
        tool,
        "message_send"
            | "message_ack"
            | "ch_subscribe"
            | "ch_unsubscribe"
            | "ch_link"
            | "ch_pause"
            | "ch_grant"
    )
}

pub(crate) fn catalogue(actor: &Actor) -> Vec<Value> {
    let id = json!({"type":"string","pattern":"^[0-7][0-9A-HJKMNP-TV-Z]{25}$"});
    let path = json!({"type":"string","maxLength":230});
    let limit = json!({"type":"integer","minimum":1,"maximum":128});
    let after = json!({"type":"integer","minimum":0});
    let rights =
        json!({"type":"array","maxItems":4,"items":{"enum":["read","write","link","admin"]}});
    let filter = json!({"type":"object","additionalProperties":false,"properties":{
        "kinds":{"type":"array","maxItems":6,"items":{"enum":["text","chunk","decision","presence","task","withdrawn"]}},
        "classes":{"type":"array","maxItems":2,"items":{"enum":["oracle","worker"]}},
        "priorities":{"type":"array","maxItems":10,"items":{"type":"integer","minimum":0,"maximum":9}}
    }});
    let rate = json!({"type":"object","additionalProperties":false,"properties":{
        "batch_ms":{"type":"integer","minimum":0,"maximum":86400000},"coalesce":{"type":"boolean"}
    }});
    let mut definitions = vec![
        (
            "message_send",
            "Send within your current W2.4 destination grants as this seat. To answer a message, pass reply_to and omit destination: the reply goes to its sender (the operator is operator/inbox). A DM is for questions and feedback; work arrives only as a delegated task. Set supersedes to one of your own message ids to replace it; kind withdrawn requires empty text/attachments and supersedes. Stored/held means durable acceptance, not model receipt; use message_receipts for delivery evidence.",
            json!({"destination":path,"text":{"type":"string","maxLength":16384},"thread_id":id,"reply_to":id,"supersedes":id,
             "priority":{"type":"integer","minimum":0,"maximum":9},"kind":{"enum":["text","chunk","decision","withdrawn"]},
             "attachments":{"type":"array","maxItems":32,"items":{"type":"string","maxLength":512}}}),
            vec!["text"],
        ),
        (
            "message_list",
            "Page your inbox/outbox; an explicit channel filter uses existing W2.4 read grants. Oracle channel reads remain own-swarm scoped.",
            json!({"thread":id,"channel":path,"after":after,"limit":limit}),
            vec![],
        ),
        (
            "message_receipts",
            "Page W2.4 delivery evidence. Senders and same-swarm Oracle use existing visibility; other recipients see only their own receipts. Native submission and application acknowledgement differ.",
            json!({"message":id,"after":after,"limit":limit}),
            vec!["message"],
        ),
        (
            "message_ack",
            "Explicitly acknowledge an addressed message as this admitted recipient (and optional addressed agent). This is separate from unread state.",
            json!({"message":id,"agent":{"type":"string","minLength":1,"maxLength":128}}),
            vec!["message"],
        ),
        (
            "ch_ls",
            "Page the current swarm's channel tree under your W2.4 read grants.",
            json!({"path":path,"after":path,"limit":limit}),
            vec!["path"],
        ),
        (
            "ch_subscribe",
            "Link your own inbox into a granted folder with optional filter and rate. This cannot subscribe another seat or grant permission.",
            json!({"folder":path,"name":{"type":"string","minLength":1,"maxLength":230},"filter":filter,"rate":rate}),
            vec!["folder", "name"],
        ),
        (
            "ch_unsubscribe",
            "Remove your own inbox link where W2.4 grants permit; Oracle retains its existing tree authority.",
            json!({"path":path}),
            vec!["path"],
        ),
    ];
    if actor.class() == Class::Oracle {
        definitions.extend([
            ("ch_link", "Create or replace a W2.4 folder, sink or routing link in your own swarm. Requires Oracle class; grants remain W2.4 policy.",
             json!({"node":{"type":"object","additionalProperties":false,"required":["path","kind"],"properties":{
                 "path":path,"kind":{"enum":["folder","sink","link"]},"target":path,"filter":filter,"rate":rate,
                 "paused":{"type":"boolean"},"grants":{"type":"object","maxProperties":4096,"propertyNames":id,"additionalProperties":rights}
             }}}),vec!["node"]),
            ("ch_pause", "Pause or resume a W2.4 tree node in your own swarm. Requires Oracle class; retained traffic is not deleted.",
             json!({"path":path,"paused":{"type":"boolean"}}),vec!["path","paused"]),
            ("ch_grant", "Replace one seat's rights on a path in your own swarm. An empty rights list is an explicit revocation. Requires Oracle class.",
             json!({"path":path,"seat":id,"rights":rights}),vec!["path","seat","rights"]),
        ]);
    }
    definitions.into_iter().map(|(name,description,mut properties,mut required)| {
        let effect = mutation(name);
        if effect {
            properties["op_id"] = id.clone();
            required.push("op_id");
        }
        json!({"name":name,"description":description,
            "inputSchema":{"type":"object","properties":properties,"required":required,"additionalProperties":false},
            "outputSchema":{"type":"object","additionalProperties":true},
            "annotations":{"readOnlyHint":!effect,"destructiveHint":matches!(name,"ch_unsubscribe"|"ch_link"|"ch_grant"),"idempotentHint":true,"openWorldHint":false}})
    }).collect()
}

#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct FilterArgs {
    kinds: Vec<MessageKind>,
    classes: Vec<Class>,
    priorities: Vec<u8>,
}
impl From<FilterArgs> for Filter {
    fn from(value: FilterArgs) -> Self {
        Self {
            kinds: value.kinds,
            classes: value.classes,
            priorities: value.priorities,
        }
    }
}
#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct RateArgs {
    batch_ms: u64,
    coalesce: bool,
}
impl From<RateArgs> for Rate {
    fn from(value: RateArgs) -> Self {
        Self {
            batch_ms: value.batch_ms,
            coalesce: value.coalesce,
        }
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Send {
    destination: Option<String>,
    text: String,
    thread_id: Option<Ulid>,
    reply_to: Option<Ulid>,
    supersedes: Option<Ulid>,
    priority: Option<u8>,
    kind: Option<MessageKind>,
    attachments: Option<Vec<String>>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct List {
    thread: Option<Ulid>,
    channel: Option<String>,
    after: Option<u64>,
    limit: Option<usize>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Receipts {
    message: Ulid,
    after: Option<u64>,
    limit: Option<usize>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Ack {
    message: Ulid,
    agent: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Ls {
    path: String,
    after: Option<String>,
    limit: Option<usize>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Subscribe {
    folder: String,
    name: String,
    #[serde(default)]
    filter: FilterArgs,
    #[serde(default)]
    rate: RateArgs,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PathArgs {
    path: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Link {
    node: NodeArgs,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NodeArgs {
    path: String,
    kind: EntryKind,
    target: Option<String>,
    #[serde(default)]
    filter: FilterArgs,
    #[serde(default)]
    rate: RateArgs,
    #[serde(default)]
    paused: bool,
    #[serde(default)]
    grants: BTreeMap<Ulid, Vec<Right>>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Pause {
    path: String,
    paused: bool,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Grant {
    path: String,
    seat: Ulid,
    rights: Vec<Right>,
}

fn path(value: String, pattern: bool) -> Result<String, String> {
    normalize(&value, pattern).map_err(|e| e.to_string())
}
fn id(value: Ulid) -> Result<Ulid, String> {
    (value != Ulid::nil())
        .then_some(value)
        .ok_or_else(|| "nil message or seat ID".into())
}
/// Adapt the shared GUI record through the same parsers as the MCP tools.
pub(crate) fn gui_request(actor: &Actor, op_id: Ulid, value: Request) -> Result<Request, String> {
    if let Request::Mute { seat, .. } = &value {
        if actor.class() != Class::Oracle {
            return Err("tool requires oracle-class authority".into());
        }
        id(*seat)?;
        return Ok(value);
    }
    let tool = match &value {
        Request::Send { .. } => "message_send",
        Request::List { .. } => "message_list",
        Request::Receipts { .. } => "message_receipts",
        Request::Acknowledge { .. } => "message_ack",
        Request::Ls { .. } => "ch_ls",
        Request::Subscribe { .. } => "ch_subscribe",
        Request::Unsubscribe { .. } => "ch_unsubscribe",
        Request::Link { .. } => "ch_link",
        Request::Pause { .. } => "ch_pause",
        Request::Grant { .. } => "ch_grant",
        Request::Mute { .. } => unreachable!(),
    };
    let mut args = match value {
        Request::Send { message } => {
            if message.id != op_id {
                return Err("message ID must equal the correlated operation ID".into());
            }
            let mut args = serde_json::to_value(message).map_err(|e| e.to_string())?;
            args.as_object_mut()
                .ok_or("message request is not an object")?
                .remove("id");
            args
        }
        other => serde_json::to_value(other).map_err(|e| e.to_string())?,
    };
    args.as_object_mut()
        .ok_or("messaging request is not an object")?
        .remove("op");
    request(
        actor,
        &Call {
            op_id,
            tool: tool.into(),
            args,
        },
    )?
    .ok_or_else(|| "unsupported messaging operation".into())
}
pub(crate) fn request(actor: &Actor, call: &Call) -> Result<Option<Request>, String> {
    if matches!(call.tool.as_str(), "ch_link" | "ch_pause" | "ch_grant")
        && actor.class() != Class::Oracle
    {
        return Err("tool requires oracle-class authority".into());
    }
    let req = match call.tool.as_str() {
        "message_send" => {
            let a: Send = tools::parse(&call.args)?;
            let message = SendRequest {
                id: call.op_id,
                // No destination: logic routes the reply to its sender (decision #264).
                destination: match (a.destination, a.reply_to.or(a.supersedes)) {
                    (Some(destination), _) => path(destination, true)?,
                    (None, Some(_)) => String::new(),
                    (None, None) => {
                        return Err(
                            "message_send needs a destination, or reply_to to answer a message's sender"
                                .into(),
                        );
                    }
                },
                thread_id: a.thread_id,
                reply_to: a.reply_to,
                supersedes: a.supersedes,
                priority: a.priority.unwrap_or(0),
                kind: a.kind.unwrap_or(MessageKind::Text),
                text: a.text,
                attachments: a.attachments.unwrap_or_default(),
            };
            message.validate().map_err(|e| e.to_string())?;
            if matches!(message.kind, MessageKind::Presence | MessageKind::Task) {
                return Err("typed presence/task messages require their producer API".into());
            }
            Request::Send { message }
        }
        "message_list" => {
            let a: List = tools::parse(&call.args)?;
            Request::List {
                thread: a.thread,
                channel: a.channel.map(|p| path(p, true)).transpose()?,
                after: a.after.unwrap_or(0),
                limit: tools::page_limit(a.limit)?,
            }
        }
        "message_receipts" => {
            let a: Receipts = tools::parse(&call.args)?;
            Request::Receipts {
                message: id(a.message)?,
                after: a.after.unwrap_or(0),
                limit: tools::page_limit(a.limit)?,
            }
        }
        "message_ack" => {
            let a: Ack = tools::parse(&call.args)?;
            if a.agent
                .as_ref()
                .is_some_and(|s| s.is_empty() || s.len() > 128 || s.chars().any(char::is_control))
            {
                return Err("invalid addressed agent".into());
            }
            Request::Acknowledge {
                message: id(a.message)?,
                agent: a.agent,
            }
        }
        "ch_ls" => {
            let a: Ls = tools::parse(&call.args)?;
            Request::Ls {
                path: path(a.path, false)?,
                after: a.after.map(|p| path(p, false)).transpose()?,
                limit: tools::page_limit(a.limit)?,
            }
        }
        "ch_subscribe" => {
            let a: Subscribe = tools::parse(&call.args)?;
            if a.name.is_empty() || a.name.contains('/') {
                return Err("subscription name must be one path token".into());
            }
            path(
                format!("{}/{}", path(a.folder.clone(), false)?, a.name),
                false,
            )?;
            Request::Subscribe {
                folder: a.folder,
                name: a.name,
                filter: a.filter.into(),
                rate: a.rate.into(),
            }
        }
        "ch_unsubscribe" => {
            let a: PathArgs = tools::parse(&call.args)?;
            Request::Unsubscribe {
                path: path(a.path, false)?,
            }
        }
        "ch_link" => {
            let a: Link = tools::parse(&call.args)?;
            Request::Link {
                node: Node {
                    path: path(a.node.path, false)?,
                    kind: a.node.kind,
                    target: a.node.target.map(|p| path(p, false)).transpose()?,
                    filter: a.node.filter.into(),
                    rate: a.node.rate.into(),
                    paused: a.node.paused,
                    grants: a.node.grants,
                },
            }
        }
        "ch_pause" => {
            let a: Pause = tools::parse(&call.args)?;
            Request::Pause {
                path: path(a.path, false)?,
                paused: a.paused,
            }
        }
        "ch_grant" => {
            let a: Grant = tools::parse(&call.args)?;
            Request::Grant {
                path: path(a.path, false)?,
                seat: id(a.seat)?,
                rights: a.rights,
            }
        }
        _ => return Ok(None),
    };
    Ok(Some(req))
}

pub(crate) fn destination_list(rows: Vec<crate::messaging::tree::Destination>) -> Value {
    json!(
        rows.into_iter()
            .map(|row| {
                let rights = match (row.read, row.post) {
                    (true, true) => vec!["read", "post"],
                    (true, false) => vec!["read"],
                    (false, true) => vec!["write-only"],
                    (false, false) => unreachable!("destination has no granted right"),
                };
                json!({"path":row.path,"rights":rights})
            })
            .collect::<Vec<_>>()
    )
}

pub(crate) fn with_counts(
    mut value: Value,
    summary: crate::messaging::ledger::InboxSummary,
) -> Value {
    // No persisted read cursor exists yet. Application acknowledgement is
    // separate evidence and never substitutes for a read cursor.
    value["unread"] = json!({"count":summary.received,"basis":"no_cursor_yet"});
    let truncated = summary.pending_ack > summary.ids.len() as u64;
    value["pending_ack"] = json!({
        "count":summary.pending_ack,"ids":summary.ids,
        "action":"call message_ack for each id listed","ids_truncated":truncated
    });
    value
}

pub(crate) fn result_for_seat(actor: &super::SeatActor, response: Response) -> Value {
    let empty = match &response {
        Response::Messages { page } => Some(page.items.is_empty()),
        Response::Receipts { page } => Some(page.items.is_empty()),
        Response::Tree { nodes, .. } => Some(nodes.is_empty()),
        _ => None,
    };
    let value = result(response);
    match empty {
        Some(empty) => tools::scoped_list(actor, value, empty.then_some("nothing_yet")),
        None => value,
    }
}

pub(crate) fn refused(op: Ulid, error: bbt_proto::messaging::MessageRefusal) -> Value {
    let mut result = super::error(op, &error.kind, &error.message);
    result["structuredContent"]["error"]["kind"] = json!(error.kind);
    result["structuredContent"]["error"]["hint"] = json!(error.hint);
    result["content"][0]["text"] = json!(result["structuredContent"].to_string());
    result
}

pub(crate) fn result(response: Response) -> Value {
    let status = match &response {
        Response::Stored { .. } | Response::Held { .. } => Some("accepted"),
        Response::Applied => Some("applied"),
        Response::Refused { .. } => Some("rejected"),
        _ => None,
    };
    let mut value = serde_json::to_value(response).expect("W2.4 response is serializable");
    if let Some(status) = status {
        value["status"] = json!(status);
    }
    value
}
