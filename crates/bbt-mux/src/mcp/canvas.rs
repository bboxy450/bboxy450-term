// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Canvas tools own no second picture. MCP and ctl share these store operations.
#![allow(
    clippy::disallowed_methods,
    reason = "canvas work executes on the blocking IO pool"
)]
use super::{Actor, Call, tools};
use crate::{knowledge::data_source as d, registry::Class};
use bboxy450_canvas::{
    Action, CanvasRecord, Content,
    record::Record,
    store::{Snapshot, Store},
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    path::Path,
    time::{Duration, Instant},
};

pub(crate) const PAGE_SIZE: usize = 32;
pub(crate) const MAX_OPS_PER_MINUTE: usize = 2400;
const RATE_WINDOW: Duration = Duration::from_secs(60);
#[derive(Default)]
pub(crate) struct Rates(BTreeMap<String, (Instant, usize)>);
impl Rates {
    pub(crate) fn admit(&mut self, actor: &str, count: usize, now: Instant) -> Result<(), String> {
        self.0
            .retain(|_, (start, _)| now.saturating_duration_since(*start) < RATE_WINDOW);
        let (_, used) = self.0.entry(actor.into()).or_insert((now, 0));
        if count > MAX_OPS_PER_MINUTE.saturating_sub(*used) {
            return Err(format!(
                "MAX_OPS_PER_MINUTE={MAX_OPS_PER_MINUTE}; per-seat 60-second cap"
            ));
        }
        *used += count;
        Ok(())
    }
}
pub(crate) fn handles(tool: &str) -> bool {
    matches!(
        tool,
        "canvas_open" | "canvas_list" | "canvas_draw" | "canvas_read"
    )
}
pub(crate) fn op_count(call: &Call) -> usize {
    if call.tool == "canvas_draw" {
        call.args["ops"]
            .as_array()
            .map_or(0, Vec::len)
            .min(bboxy450_canvas::store::MAX_BATCH_OPS)
    } else {
        0
    }
}
fn object(properties: Value, required: Value) -> Value {
    json!({"type":"object","properties":properties,"required":required,"additionalProperties":false})
}
fn action_schema() -> Value {
    let string = json!({"type":"string"});
    let tags = json!({"type":"array","items":{"type":"string","minLength":1}});
    let point = json!({"type":"array","items":{"type":"number"},"minItems":2,"maxItems":2});
    let bounds = object(
        json!({"origin":point,"size":point}),
        json!(["origin", "size"]),
    );
    let content = json!({"oneOf":[
        object(json!({"type":{"const":"shape"},"bounds":bounds,"shape":{"enum":["box","rounded","ellipse","diamond","cylinder","actor"]},"label":string}),json!(["type","bounds","shape","label"])),
        object(json!({"type":{"const":"text"},"bounds":bounds,"text":string,"role":{"enum":["heading","paragraph","note"]}}),json!(["type","bounds","text","role"])),
        object(json!({"type":{"const":"frame"},"bounds":bounds,"name":string,"members":{"type":"array","items":string}}),json!(["type","bounds","name","members"])),
        object(json!({"type":{"const":"connector"},"from":string,"to":string,"label":{"type":["string","null"]},"arrowheads":{"type":"array","items":{"type":"boolean"},"minItems":2,"maxItems":2},"dashed":{"type":"boolean"}}),json!(["type","from","to","arrowheads","dashed"]))
    ]});
    let element = object(
        json!({"id":{"type":"string","minLength":1},"tags":tags,"kind":{"enum":[null,"service","store","queue","person","external"]},"emphasis":{"type":"boolean"},"status":{"enum":["normal","waiting","error","done"]},"content":content}),
        json!(["id", "tags", "content"]),
    );
    json!({"oneOf":[
        object(json!({"op":{"const":"add"},"element":element}),json!(["op","element"])),
        object(json!({"op":{"const":"change"},"element":element}),json!(["op","element"])),
        object(json!({"op":{"const":"remove"},"id":string}),json!(["op","id"])),
        object(json!({"op":{"const":"tag"},"id":string,"tag":{"type":"string","minLength":1}}),json!(["op","id","tag"]))
    ]})
}
pub(crate) fn catalogue() -> Vec<Value> {
    let id = json!({"type":"string","minLength":1,"maxLength":64});
    let mut open = object(
        json!({"title":{"type":"string","minLength":1,"maxLength":256},"number":{"type":"integer","minimum":0},"id":id,"tags":{"type":"array","items":{"type":"string","minLength":1,"maxLength":64},"maxItems":32}}),
        json!([]),
    );
    open["oneOf"] = json!([{"required":["title"]},{"required":["number"]},{"required":["id"]}]);
    [
        ("canvas_open", "Create or open a swarm canvas by title, number or id. C1a returns shown:false. Default drawing authority is creator plus Oracle; explicit grants are a later step.", open, false),
        ("canvas_list", "List this swarm's canvases, newest activity first, in bounded pages. Filter by tag; pass next as cursor.", object(json!({"tag":{"type":"string","minLength":1},"cursor":{"type":"string","maxLength":256}}),json!([])), true),
        ("canvas_draw", "Append a small semantic batch. Store assigns seq; op_id dedupes delivery across restart. Stale expect_seq is recorded, not refused. Colours, fonts and weights are forbidden. MAX_ELEMENTS, MAX_BATCH_OPS and MAX_OPS_PER_MINUTE bound drawing.", object(json!({"id":id,"ops":{"type":"array","items":action_schema(),"minItems":1,"maxItems":bboxy450_canvas::store::MAX_BATCH_OPS},"expect_seq":{"type":"integer","minimum":0}}),json!(["id","ops"])), false),
        ("canvas_read", "Read semantic elements, seq and digest; image:true also returns a headless GPU-rendered PNG image block. Optional frame fits that region.", object(json!({"id":id,"frame":{"type":"string","minLength":1},"image":{"type":"boolean"}}),json!(["id"])), true)
    ].into_iter().map(|(name, description, input, read)|json!({"name":name,"description":description,"inputSchema":input,
        "outputSchema":{"type":"object","additionalProperties":true},"annotations":{"readOnlyHint":read,"destructiveHint":false,"idempotentHint":true,"openWorldHint":false}})).collect()
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Open {
    title: Option<String>,
    number: Option<u64>,
    id: Option<String>,
    tags: Option<Vec<String>>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct List {
    tag: Option<String>,
    cursor: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Draw {
    id: String,
    ops: Vec<Action>,
    expect_seq: Option<u64>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Read {
    id: String,
    frame: Option<String>,
    #[serde(default)]
    image: bool,
}
fn error(code: &str, reason: impl Into<String>) -> d::CommandError {
    d::CommandError::new(code, reason)
}
fn parse<T: serde::de::DeserializeOwned>(args: &Value) -> d::Result<T> {
    tools::parse(args).map_err(|s| error("invalid_arguments", s))
}
fn io(error: std::io::Error) -> d::CommandError {
    let text = error.to_string();
    let code = text
        .split_once(':')
        .map(|(code, _)| code)
        .filter(|code| code.starts_with("canvas_") || *code == "op_id_reused")
        .unwrap_or("canvas_io");
    d::CommandError::new(code, text.clone())
}
fn find(store: &Store, swarm: &str, id: &str) -> d::Result<Record> {
    store
        .records()
        .map_err(io)?
        .into_iter()
        .find(|r| r.swarm == swarm && r.id == id)
        .ok_or_else(|| error("canvas_not_found", "canvas is not in the current swarm"))
}
fn summary(s: &Snapshot) -> Value {
    json!({"id":s.record.id,"number":s.record.number,"slug":s.record.slug,"title":s.record.title,"tags":s.record.tags,
    "seq":s.seq,"digest":s.canvas.digest(),"dropped_tail_bytes":s.dropped_tail_bytes})
}
fn activity(s: &Snapshot) -> (u64, String) {
    (
        if s.batches.is_empty() {
            ulid::Ulid::from_string(&s.record.id).map_or(0, |id| id.timestamp_ms())
        } else {
            s.last_change()
        },
        s.record.id.clone(),
    )
}
/// Root and swarm come from admission, never from a caller-controlled path.
/// Image bytes are kept outside structuredContent and appended as MCP content.
pub(crate) fn execute(
    root: &Path,
    swarm: &str,
    actor: &Actor,
    task: Option<&str>,
    call: &Call,
) -> d::Result<Value> {
    let directory = if root.join(".bboxy").is_dir() {
        root.join(".bboxy")
    } else {
        root.to_owned()
    }
    .join("canvases");
    let store = Store::new(directory);
    let mut image = None;
    let value = match call.tool.as_str() {
        "canvas_open" => {
            let args: Open = parse(&call.args)?;
            if [
                args.title.is_some(),
                args.number.is_some(),
                args.id.is_some(),
            ]
            .into_iter()
            .filter(|b| *b)
            .count()
                != 1
            {
                return Err(error(
                    "invalid_arguments",
                    "canvas_open requires exactly one of title, number or id",
                ));
            }
            let records = store.records().map_err(io)?;
            // A create whose journal result was lost owns its operation ID.
            // Resume Store::create (including durability custody), even if a
            // concurrent creator also published the same title.
            let recovering_create = args.title.is_some()
                && records
                    .iter()
                    .any(|r| r.swarm == swarm && r.id == call.op_id.to_string());
            let existing = records.into_iter().find(|r| {
                !recovering_create
                    && r.swarm == swarm
                    && (args.id.as_ref().is_some_and(|id| *id == r.id)
                        || args.number == Some(r.number)
                        || args.title.as_ref().is_some_and(|t| *t == r.title))
            });
            let s = if let Some(record) = existing {
                store.load(&record).map_err(io)?
            } else if let Some(title) = args.title {
                store
                    .create(
                        &call.op_id.to_string(),
                        CanvasRecord {
                            title,
                            swarm: swarm.into(),
                            tags: args.tags.unwrap_or_default(),
                            creator: actor.reference(),
                            task: task.map(str::to_owned),
                        },
                    )
                    .map_err(io)?
            } else {
                return Err(error(
                    "canvas_not_found",
                    "canvas is not in the current swarm",
                ));
            };
            let mut value = summary(&s);
            value["shown"] = json!(false);
            value
        }
        "canvas_list" => {
            let args: List = parse(&call.args)?;
            // A cursor is a scoped last activity key, not an unstable row offset.
            let before: Option<(String, Option<String>, u64, String)> = args
                .cursor
                .as_deref()
                .map(serde_json::from_str)
                .transpose()
                .map_err(|_| error("invalid_canvas_cursor", "cursor is not a canvas page key"))?;
            if before
                .as_ref()
                .is_some_and(|(scope, tag, _, _)| scope != swarm || tag != &args.tag)
            {
                return Err(error(
                    "invalid_canvas_cursor",
                    "cursor belongs to another swarm or tag",
                ));
            }
            let mut rows = vec![];
            for record in store.records().map_err(io)?.into_iter().filter(|r| {
                r.swarm == swarm && args.tag.as_ref().is_none_or(|t| r.tags.contains(t))
            }) {
                let s = store.load(&record).map_err(io)?;
                let key = activity(&s);
                if before
                    .as_ref()
                    .is_none_or(|(_, _, time, id)| key < (*time, id.clone()))
                {
                    let mut row = summary(&s);
                    row["last_change"] = json!(key.0);
                    row["last_author"] = json!(s.last_author());
                    rows.push((key, row));
                }
            }
            rows.sort_by(|a, b| b.0.cmp(&a.0));
            let next = if rows.len() > PAGE_SIZE {
                let key = &rows[PAGE_SIZE - 1].0;
                Some(serde_json::to_string(&(swarm, &args.tag, key.0, &key.1)).unwrap())
            } else {
                None
            };
            json!({"canvases":rows.into_iter().take(PAGE_SIZE).map(|(_, row)| row).collect::<Vec<_>>(),"next":next,"page_size":PAGE_SIZE})
        }
        "canvas_draw" => {
            let args: Draw = parse(&call.args)?;
            let record = find(&store, swarm, &args.id)?;
            if actor.class() != Class::Oracle && actor.reference() != record.creator {
                return Err(error(
                    "canvas_draw_denied",
                    "drawing is granted to the creator and Oracle class; explicit grants are a later step",
                ));
            }
            let appended = store
                .append(
                    &record,
                    &call.op_id.to_string(),
                    &actor.reference(),
                    args.expect_seq,
                    &args.ops,
                )
                .map_err(io)?;
            let mut value = summary(&appended.snapshot);
            value["stale"] = json!(appended.stale);
            value["duplicate"] = json!(appended.duplicate);
            value
        }
        "canvas_read" => {
            let args: Read = parse(&call.args)?;
            let s = store.load(&find(&store, swarm, &args.id)?).map_err(io)?;
            let frame = args
                .frame
                .as_ref()
                .map(|id| match s.canvas.get(id).map(|e| &e.content) {
                    Some(Content::Frame {
                        bounds, members, ..
                    }) => Ok((*bounds, members)),
                    _ => Err(error(
                        "canvas_frame_not_found",
                        "frame id is not a frame on this canvas",
                    )),
                })
                .transpose()?;
            let mut value = summary(&s);
            value["elements"] = json!(
                s.canvas
                    .elements()
                    .filter(|e| frame.is_none_or(|(_, members)| members.contains(&e.id)
                        || args.frame.as_ref() == Some(&e.id)))
                    .collect::<Vec<_>>()
            );
            if args.image {
                image = Some(render::png(&s.canvas, frame.map(|(bounds, _)| bounds))?);
                value["image"] =
                    json!({"mimeType":"image/png","width":render::WIDTH,"height":render::HEIGHT});
            }
            value
        }
        _ => return Err(error("unknown_tool", "not a canvas tool")),
    };
    let mut result = super::success(call.op_id, value);
    if let Some(png) = image {
        use base64::Engine;
        result["content"].as_array_mut().unwrap().push(json!({"type":"image","mimeType":"image/png","data":base64::engine::general_purpose::STANDARD.encode(png)}));
    }
    if serde_json::to_vec(&result)
        .map_err(|e| error("canvas_result", e.to_string()))?
        .len()
        > super::MAX_RESULT
    {
        return Err(error(
            "canvas_result_cap",
            "canvas_read exceeds MAX_RESULT; read a frame",
        ));
    }
    Ok(result)
}
mod render;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn canvas_rate_cap_names_per_seat_cap_and_recovers() {
        let mut rates = Rates::default();
        let now = Instant::now();
        rates.admit("a", MAX_OPS_PER_MINUTE, now).unwrap();
        assert!(
            rates
                .admit("a", 1, now)
                .unwrap_err()
                .contains("MAX_OPS_PER_MINUTE")
        );
        rates.admit("b", 1, now).unwrap();
        rates.admit("a", 1, now + RATE_WINDOW).unwrap();
    }
    #[test]
    fn canvas_schema_refuses_style_fields_at_every_depth() {
        let catalogue = catalogue();
        let schema = jsonschema::validator_for(
            &catalogue
                .iter()
                .find(|t| t["name"] == "canvas_draw")
                .unwrap()["inputSchema"],
        )
        .unwrap();
        let base = json!({"id":"canvas","ops":[{"op":"add","element":{"id":"a","tags":[],"content":{"type":"shape","bounds":{"origin":[0,0],"size":[100,50]},"shape":"box","label":"A"}}}]});
        assert!(schema.is_valid(&base));
        for field in ["color", "colour", "font", "weight"] {
            for level in 0..3 {
                let mut value = base.clone();
                let target = match level {
                    0 => &mut value["ops"][0],
                    1 => &mut value["ops"][0]["element"],
                    _ => &mut value["ops"][0]["element"]["content"],
                };
                target[field] = json!("injected");
                assert!(!schema.is_valid(&value), "{field} level {level}");
                assert!(parse::<Draw>(&value).is_err());
            }
        }
        let mut seq = base;
        seq["ops"][0]["seq"] = json!(1);
        assert!(!schema.is_valid(&seq));
    }
}
