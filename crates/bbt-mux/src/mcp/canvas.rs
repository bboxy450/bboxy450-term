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
        "canvas_open"
            | "canvas_list"
            | "canvas_draw"
            | "canvas_read"
            | "canvas_close"
            | "canvas_archived_list"
            | "canvas_restore"
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
    let points = json!({"type":"array","items":point,"maxItems":bboxy450_canvas::MAX_POINTS});
    let arrowheads = json!({"type":"array","items":{"type":"boolean"},"minItems":2,"maxItems":2});
    let side = json!({"enum":["top","right","bottom","left"]});
    // A colour is a theme token, a named hue or a hex literal; see NAMED_COLORS.
    let color = json!({"type":"string","pattern":"^(#[0-9a-fA-F]{3}|#[0-9a-fA-F]{6}|#[0-9a-fA-F]{8}|accent|fg|dim|surface|bg|waiting|error|done|red|orange|yellow|green|teal|blue|indigo|violet|pink|white|black)$"});
    let style = object(
        json!({"fill":color,"stroke":color,"stroke_width":{"type":"number","minimum":0,"maximum":64},"opacity":{"type":"number","minimum":0,"maximum":1},"glow":color,"shadow":{"type":"boolean"},"radius":{"type":"number","minimum":0,"maximum":1000},"text":color,"font_size":{"type":"number","minimum":0,"maximum":400},"weight":{"type":"integer","minimum":100,"maximum":900},"italic":{"type":"boolean"},"mono":{"type":"boolean"},"align":{"enum":["left","center","right"]}}),
        json!([]),
    );
    let content = json!({"oneOf":[
        object(json!({"type":{"const":"shape"},"bounds":bounds,"shape":{"enum":["box","rounded","ellipse","diamond","cylinder","actor","hexagon","triangle","parallelogram","star","cloud"]},"label":string}),json!(["type","bounds","shape","label"])),
        object(json!({"type":{"const":"text"},"bounds":bounds,"text":string,"role":{"enum":["heading","paragraph","note"]}}),json!(["type","bounds","text","role"])),
        object(json!({"type":{"const":"frame"},"bounds":bounds,"name":string,"members":{"type":"array","items":string}}),json!(["type","bounds","name","members"])),
        object(json!({"type":{"const":"connector"},"from":string,"to":string,"label":{"type":["string","null"]},"arrowheads":arrowheads,"dashed":{"type":"boolean"},"route":{"enum":["straight","orthogonal","curved"]},"from_side":side,"to_side":side,"waypoints":points,"label_at":{"type":"number","minimum":0,"maximum":1}}),json!(["type","from","to","arrowheads","dashed"])),
        object(json!({"type":{"const":"path"},"points":{"type":"array","items":point,"minItems":2,"maxItems":bboxy450_canvas::MAX_POINTS},"closed":{"type":"boolean"},"smooth":{"type":"boolean"},"arrowheads":arrowheads}),json!(["type","points"]))
    ]});
    let element = object(
        json!({"id":{"type":"string","minLength":1},"tags":tags,"kind":{"enum":[null,"service","store","queue","person","external"]},"emphasis":{"type":"boolean"},"status":{"enum":["normal","waiting","error","done"]},"style":style,"content":content}),
        json!(["id", "tags", "content"]),
    );
    json!({"oneOf":[
        object(json!({"op":{"const":"add"},"element":element}),json!(["op","element"])),
        object(json!({"op":{"const":"change"},"element":element}),json!(["op","element"])),
        object(json!({"op":{"const":"patch"},"id":string,"bounds":bounds,"label":string,"style":style,"status":{"enum":["normal","waiting","error","done"]},"emphasis":{"type":"boolean"}}),json!(["op","id"])),
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
        ("canvas_open", "Create or open a swarm canvas by title, number or id. Enqueues presentation beside the calling seat when a GUI is attached; shown:false names a detached or busy reason. Retry a busy presentation with the same operation ID. Never waits for the GUI. Default drawing authority is creator plus Oracle; explicit grants are a later step.", open, false),
        ("canvas_list", "List this swarm's canvases, newest activity first, in bounded pages. Filter by tag; pass next as cursor.", object(json!({"tag":{"type":"string","minLength":1},"cursor":{"type":"string","maxLength":256}}),json!([])), true),
        ("canvas_draw", "Append a batch of ops: add, change, patch (bounds, label, style, status or emphasis of one element), remove, tag. A batch is one set: a frame may list members added later in the same batch. Elements: shape (box, rounded, ellipse, diamond, cylinder, actor, hexagon, triangle, parallelogram, star, cloud), text, frame, connector (route straight, orthogonal or curved; from_side/to_side ports; waypoints; label_at 0..1) and path (free world points, closed, smooth, arrowheads). Meaning (kind, emphasis, status) sets the house look; an optional style overrides it: fill, stroke, stroke_width, opacity, glow, shadow, radius, text, font_size, weight, italic, mono, align, with colours as theme tokens, named hues or hex. Store assigns seq; op_id dedupes delivery across restart. Stale expect_seq is recorded, not refused. MAX_ELEMENTS, MAX_BATCH_OPS, MAX_POINTS and MAX_OPS_PER_MINUTE bound drawing. Read the canvas skill for the full reference.", object(json!({"id":id,"ops":{"type":"array","items":action_schema(),"minItems":1,"maxItems":bboxy450_canvas::store::MAX_BATCH_OPS},"expect_seq":{"type":"integer","minimum":0}}),json!(["id","ops"])), false),
        ("canvas_close", "Retire a canvas: it leaves canvas_list and the GUI; its record and log are kept under closed/, nothing is deleted. Granted to the creator and Oracle class. Closing an already closed canvas is a no-op.", object(json!({"id":id}),json!(["id"])), false),
        ("canvas_archived_list", "List this swarm's closed canvases, newest first, so one can be restored.", object(json!({}),json!([])), true),
        ("canvas_restore", "Bring a closed canvas back: it returns to canvas_list and the GUI with its drawing. Granted to the creator and Oracle class. Restoring an open canvas is a no-op.", object(json!({"id":id}),json!(["id"])), false),
        ("canvas_read", "Read seq, digest, the fitted extent of the canvas and of every frame's members, and the elements; elements:false leaves the element list out for a cheap look. image:true also returns a headless GPU-rendered PNG image block fitted to the same extent the GUI fits, with FIT_PADDING around it. Optional frame restricts both to that frame.", object(json!({"id":id,"frame":{"type":"string","minLength":1},"image":{"type":"boolean"},"elements":{"type":"boolean"}}),json!(["id"])), true)
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
struct Close {
    id: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Read {
    id: String,
    frame: Option<String>,
    #[serde(default)]
    image: bool,
    #[serde(default = "yes")]
    elements: bool,
}
fn yes() -> bool {
    true
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
    (s.last_change(), s.record.id.clone())
}
/// Presentation is volatile, not part of the durable drawing result. Replaying
/// an operation reuses its semantic result and checks the current GUI owner.
pub(crate) fn present(
    value: &mut Value,
    call: &Call,
    captures: &crate::gui_capture::Captures,
    root: &Path,
    swarm: crate::Ulid,
    actor: &Actor,
) {
    if value["isError"] != false {
        return;
    }
    match call.tool.as_str() {
        "canvas_open" => {
            let (Some(id), Some(title)) = (
                value["structuredContent"]["id"].as_str(),
                value["structuredContent"]["title"].as_str(),
            ) else {
                return;
            };
            let presentation = captures.canvas(
                call.op_id,
                bbt_proto::gui::UiRequest::Canvas {
                    source: bbt_proto::gui::UiCanvasSource {
                        root: root.to_string_lossy().into_owned(),
                        swarm,
                        id: id.into(),
                        title: title.into(),
                    },
                    beside: actor.seat().map(|seat| seat.seat_id),
                },
            );
            let data = &mut value["structuredContent"];
            data["shown"] = json!(presentation.is_ok());
            match presentation {
                Ok(()) => {
                    data["presentation"] = json!("enqueued");
                    data.as_object_mut().unwrap().remove("reason");
                }
                Err(reason) => {
                    data["presentation"] = json!(if reason.starts_with("ui_detached:") {
                        "detached"
                    } else {
                        "busy"
                    });
                    data["reason"] = json!(reason);
                }
            }
        }
        "canvas_draw" | "canvas_close" | "canvas_restore" => {
            let _ = captures.canvas(
                call.op_id,
                bbt_proto::gui::UiRequest::CanvasRefresh { swarm },
            );
        }
        _ => return,
    }
    value["content"][0]["text"] = json!(value["structuredContent"].to_string());
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
            summary(&s)
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
        "canvas_close" => {
            let args: Close = parse(&call.args)?;
            match find(&store, swarm, &args.id) {
                Ok(record) => {
                    if actor.class() != Class::Oracle && actor.reference() != record.creator {
                        return Err(error(
                            "canvas_close_denied",
                            "closing is granted to the creator and Oracle class",
                        ));
                    }
                    store.close(&record).map_err(io)?;
                    json!({"id":record.id,"number":record.number,"slug":record.slug,"closed":true})
                }
                // Already closed: the retry of a lost reply.
                Err(e)
                    if e.code == "canvas_not_found"
                        && store
                            .closed_records()
                            .map_err(io)?
                            .iter()
                            .any(|r| r.swarm == swarm && r.id == args.id) =>
                {
                    json!({"id":args.id,"closed":true,"already":true})
                }
                Err(e) => return Err(e),
            }
        }
        "canvas_archived_list" => {
            let mut records: Vec<Record> = store
                .closed_records()
                .map_err(io)?
                .into_iter()
                .filter(|r| r.swarm == swarm)
                .collect();
            records.sort_by_key(|r| std::cmp::Reverse(r.number));
            json!({"canvases": records.iter().map(|r| json!({"id":r.id,"number":r.number,"slug":r.slug,"title":r.title,"tags":r.tags,"creator":r.creator})).collect::<Vec<_>>()})
        }
        "canvas_restore" => {
            let args: Close = parse(&call.args)?;
            if let Ok(record) = find(&store, swarm, &args.id) {
                json!({"id":record.id,"number":record.number,"slug":record.slug,"restored":true,"already":true})
            } else {
                let record = store
                    .closed_records()
                    .map_err(io)?
                    .into_iter()
                    .find(|r| r.swarm == swarm && r.id == args.id)
                    .ok_or_else(|| {
                        error(
                            "canvas_not_found",
                            "no closed canvas with this id in the current swarm",
                        )
                    })?;
                if actor.class() != Class::Oracle && actor.reference() != record.creator {
                    return Err(error(
                        "canvas_restore_denied",
                        "restoring is granted to the creator and Oracle class",
                    ));
                }
                store.restore(&record).map_err(io)?;
                json!({"id":record.id,"number":record.number,"slug":record.slug,"restored":true})
            }
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
            let shown = |e: &&bboxy450_canvas::Element| {
                frame.is_none_or(|(_, members)| {
                    members.contains(&e.id) || args.frame.as_ref() == Some(&e.id)
                })
            };
            // Layout help: the fitted extent, and each frame's members' box,
            // so a harness places the next element without a trial render.
            value["extent"] = json!(frame.map_or_else(|| s.canvas.extent(), |(b, _)| Some(b)));
            value["frames"] = json!(
                s.canvas
                    .elements()
                    .filter(shown)
                    .filter_map(|e| match &e.content {
                        Content::Frame {
                            bounds, members, ..
                        } => Some(json!({
                            "id": e.id,
                            "bounds": bounds,
                            "members": s.canvas.extent_of(members.iter().map(String::as_str)),
                        })),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
            );
            if args.elements {
                value["elements"] = json!(s.canvas.elements().filter(shown).collect::<Vec<_>>());
            }
            if args.image {
                let (png, [width, height]) =
                    render::png(&s.canvas, frame.map(|(bounds, _)| bounds))?;
                image = Some(png);
                value["image"] = json!({"mimeType":"image/png","width":width,"height":height});
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
    fn canvas_presentation_rechecks_gui_attachment_on_replay_without_waiting() {
        let captures = crate::gui_capture::Captures::default();
        let actor = Actor::Operator {
            device_id: crate::Ulid(1),
            door: super::super::Door::Gui,
            issued_by: None,
        };
        let call = Call {
            op_id: crate::Ulid(2),
            tool: "canvas_open".into(),
            args: json!({"title":"Plan"}),
        };
        // This is the semantic result retained in the operation journal. The
        // GUI result is derived each time, rather than replaying a stale flag.
        let retained = super::super::success(call.op_id, json!({"id":"drawing","title":"Plan"}));
        let mut result = retained.clone();
        present(
            &mut result,
            &call,
            &captures,
            Path::new("/private/project/.bboxy"),
            crate::Ulid(3),
            &actor,
        );
        assert_eq!(result["structuredContent"]["shown"], false);
        assert!(
            result["structuredContent"]["reason"]
                .as_str()
                .unwrap()
                .contains("ui_detached")
        );
        let mut owner = captures.subscribe();
        result = retained.clone();
        present(
            &mut result,
            &call,
            &captures,
            Path::new("/private/project/.bboxy"),
            crate::Ulid(3),
            &actor,
        );
        assert_eq!(result["structuredContent"]["shown"], true);
        assert_eq!(result["structuredContent"]["presentation"], "enqueued");
        assert!(result["structuredContent"].get("reason").is_none());
        assert_eq!(
            result["content"][0]["text"],
            result["structuredContent"].to_string()
        );
        assert!(
            matches!(owner.try_recv().unwrap(), crate::gui_capture::Notice::Ui {
            id: crate::Ulid(2), request: bbt_proto::gui::UiRequest::Canvas { source, beside: None }
        } if source.id == "drawing" && source.root == "/private/project/.bboxy" && source.swarm == crate::Ulid(3))
        );
        let draw = Call {
            tool: "canvas_draw".into(),
            ..call.clone()
        };
        present(
            &mut result,
            &draw,
            &captures,
            Path::new("/private/project/.bboxy"),
            crate::Ulid(3),
            &actor,
        );
        assert!(matches!(
            owner.try_recv().unwrap(),
            crate::gui_capture::Notice::Ui {
                request: bbt_proto::gui::UiRequest::CanvasRefresh {
                    swarm: crate::Ulid(3)
                },
                ..
            }
        ));
        for id in 10..18 {
            captures
                .canvas(
                    crate::Ulid(id),
                    bbt_proto::gui::UiRequest::CanvasRefresh {
                        swarm: crate::Ulid(3),
                    },
                )
                .unwrap();
        }
        present(
            &mut result,
            &call,
            &captures,
            Path::new("/private/project/.bboxy"),
            crate::Ulid(3),
            &actor,
        );
        assert_eq!(result["structuredContent"]["shown"], false);
        assert_eq!(result["structuredContent"]["presentation"], "busy");
        assert!(
            result["structuredContent"]["reason"]
                .as_str()
                .unwrap()
                .contains("ui_busy: GUI relay is full; retry")
        );
        for _ in 0..8 {
            owner.try_recv().unwrap();
        }
        present(
            &mut result,
            &call,
            &captures,
            Path::new("/private/project/.bboxy"),
            crate::Ulid(3),
            &actor,
        );
        assert_eq!(
            result["structuredContent"]["shown"], true,
            "retry can enqueue after the GUI drains"
        );
        assert!(matches!(
            owner.try_recv().unwrap(),
            crate::gui_capture::Notice::Ui {
                request: bbt_proto::gui::UiRequest::Canvas { .. },
                ..
            }
        ));
        drop(owner);
        present(
            &mut result,
            &call,
            &captures,
            Path::new("/private/project/.bboxy"),
            crate::Ulid(3),
            &actor,
        );
        assert_eq!(result["structuredContent"]["shown"], false);
        assert!(retained["structuredContent"].get("shown").is_none());
    }
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
    fn canvas_schema_keeps_look_under_style_and_refuses_stray_fields() {
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
        // The look lives in one place, the style object; loose colour or font
        // fields anywhere else are refused at every depth.
        for field in ["color", "colour", "font", "weight", "fill"] {
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
        let mut styled = base.clone();
        styled["ops"][0]["element"]["style"] = json!({"fill":"#1b2a4a","stroke":"violet","glow":"accent","stroke_width":2,"opacity":0.9,"font_size":18,"weight":700,"italic":true,"align":"left"});
        assert!(schema.is_valid(&styled));
        parse::<Draw>(&styled).unwrap();
        let mut bad_color = base.clone();
        bad_color["ops"][0]["element"]["style"] = json!({"fill":"not a colour"});
        assert!(!schema.is_valid(&bad_color));
        let mut unknown_style = base.clone();
        unknown_style["ops"][0]["element"]["style"] = json!({"gradient":"#000"});
        assert!(!schema.is_valid(&unknown_style));
        assert!(parse::<Draw>(&unknown_style).is_err());
        let routed = json!({"id":"canvas","ops":[
            {"op":"add","element":{"id":"b","tags":[],"content":{"type":"shape","bounds":{"origin":[300,0],"size":[100,50]},"shape":"cloud","label":"B"}}},
            {"op":"add","element":{"id":"ab","tags":[],"content":{"type":"connector","from":"a","to":"b","label":"x","arrowheads":[false,true],"dashed":false,"route":"orthogonal","from_side":"right","to_side":"left","waypoints":[[200,80]],"label_at":0.25}}},
            {"op":"add","element":{"id":"mark","tags":[],"content":{"type":"path","points":[[0,0],[10,10],[20,0]],"smooth":true}}},
            {"op":"patch","id":"a","bounds":{"origin":[5,5],"size":[100,50]},"style":{"glow":"pink"}}
        ]});
        assert!(schema.is_valid(&routed));
        parse::<Draw>(&routed).unwrap();
        let mut seq = base;
        seq["ops"][0]["seq"] = json!(1);
        assert!(!schema.is_valid(&seq));
    }
}
