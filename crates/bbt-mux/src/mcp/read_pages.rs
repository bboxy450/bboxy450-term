// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Read projections count both MCP JSON copies and reserve space for host scope.
use crate::{Ulid, knowledge::data_source as d};
use serde_json::{Value, json};

pub(crate) const BUDGET: usize = super::MAX_RESULT - 16 * 1024;

pub(crate) fn fits(value: &Value) -> bool {
    serde_json::to_vec(&super::success(Ulid::nil(), value.clone()))
        .is_ok_and(|bytes| bytes.len() <= BUDGET)
}

/// The ledger retains complete receipts; only an individually oversized display
/// projection loses arrival expansion. Signed message envelopes stay untouched.
pub(crate) fn message_page(value: &mut Value) {
    if fits(value) {
        return;
    }
    let receipts = value["kind"] == "receipts";
    if let Some(items) = value["page"]["items"].as_array_mut() {
        for item in items {
            if receipts {
                let required = [
                    "message_sequence",
                    "message",
                    "sender",
                    "recipient",
                    "agent",
                    "evidence",
                    "note",
                    "held",
                    "received_ms",
                    "eligible_ms",
                    "batch_key",
                    "class",
                    "attempt",
                    "native_id",
                    "relayed_via_main",
                    "signature",
                    "signature_reason",
                    "sender_device",
                    "recipient_device",
                    "sender_operator",
                    "recipient_operator",
                    "wake",
                    "until",
                    "reason",
                ];
                item.as_object_mut()
                    .unwrap()
                    .retain(|key, _| required.contains(&key.as_str()));
                for key in [
                    "agent",
                    "note",
                    "held",
                    "batch_key",
                    "native_id",
                    "signature_reason",
                    "until",
                ] {
                    if let Some(s) = item[key].as_str() {
                        item[key] = json!(super::tools::clip(s, 2048));
                    }
                }
                item["arrivals"] = json!([]);
                item["receipt_truncated"] = json!(true);
            } else if !item["held_out"].is_null() {
                item["held_out"] = Value::Null;
                item["held_out_truncated"] = json!(true);
            }
        }
    }
}

pub(crate) fn lean_frontmatter(value: &Value) -> Value {
    let mut result = serde_json::Map::new();
    for key in [
        "id",
        "type",
        "title",
        "status",
        "number",
        "role",
        "assignee",
        "delegated_by",
        "swarm",
        "updated",
        "created",
    ] {
        if let Some(value) = value.get(key) {
            let value = match value {
                Value::String(s) => json!(super::tools::clip(s, 2048)),
                Value::Number(_) | Value::Bool(_) | Value::Null => value.clone(),
                _ => Value::Null,
            };
            result.insert(key.into(), value);
        }
    }
    // Keep bounded type-specific fields (artifact refs, evidence, provenance,
    // etc.) ahead of optional metadata. Large history never consumes the page.
    let priority = [
        "authorship",
        "provenance",
        "report",
        "verify_result",
        "ref",
        "produced_by",
        "grade",
        "for",
    ];
    let keys = priority.into_iter().chain(
        value
            .as_object()
            .into_iter()
            .flat_map(|m| m.keys().map(String::as_str)),
    );
    for key in keys {
        if key == "history" || key.len() > 128 || result.contains_key(key) {
            continue;
        }
        let Some(field) = value.get(key) else {
            continue;
        };
        if serde_json::to_vec(field).map_or(true, |b| b.len() > 64 * 1024) {
            continue;
        }
        let mut candidate = result.clone();
        candidate.insert(key.into(), field.clone());
        if serde_json::to_vec(&super::success(
            Ulid::nil(),
            json!({"frontmatter":candidate}),
        ))
        .is_ok_and(|b| b.len() <= BUDGET / 3)
        {
            result = candidate;
        }
    }
    result.insert("frontmatter_truncated".into(), json!(true));
    Value::Object(result)
}

pub(crate) fn task_detail(mut value: Value) -> Value {
    if fits(&value) {
        return value;
    }
    let original = value["metadata"].clone();
    if let Some(metadata) = value["metadata"].as_object_mut() {
        metadata.remove("history");
    }
    value["metadata_truncated"] = json!(true);
    value["body_truncated"] = json!(true);
    if !fits(&value) {
        value["metadata"] = lean_frontmatter(&original);
        for key in ["report", "verify_result"] {
            if let Some(field) = original.get(key) {
                value["metadata"][key] = field.clone();
            }
        }
        if !fits(&value) {
            value["metadata"] = lean_frontmatter(&original);
        }
    }
    value
}

/// Capture records already have a 128 KiB encoded-JSON budget. Bound the only
/// variable-length diagnostics beside that payload, preserving every record.
pub(crate) fn history_result(mut result: Value) -> Value {
    if result["isError"] != false {
        return result;
    }
    fn clip_field(value: &mut Value, field: &str, flag: &str) -> bool {
        let Some(text) = value[field].as_str() else {
            return false;
        };
        if text.len() <= 2048 {
            return false;
        }
        let text = super::tools::clip(text, 2048).to_owned();
        value[field] = json!(text);
        value[flag] = json!(true);
        true
    }
    let mut data = result["structuredContent"].clone();
    let mut changed = clip_field(
        &mut data["status"],
        "recovery_notice",
        "recovery_notice_truncated",
    );
    if let Some(gaps) = data["gaps"].as_array_mut() {
        for gap in gaps {
            changed |= clip_field(gap, "reason", "reason_truncated");
        }
    }
    if changed {
        result["content"][0]["text"] = json!(data.to_string());
        result["structuredContent"] = data;
    }
    result
}

pub(crate) fn graph_page(mut value: Value) -> Value {
    while !fits(&value) {
        if value["nodes"].as_array().is_some_and(|n| n.len() > 1) {
            value["nodes"].as_array_mut().unwrap().pop();
            value["next"] = value["nodes"].as_array().unwrap().last().unwrap()["id"].clone();
            let ids: std::collections::BTreeSet<_> = value["nodes"]
                .as_array()
                .unwrap()
                .iter()
                .map(|n| n["id"].clone().to_string())
                .collect();
            if let Some(edges) = value["edges"].as_array_mut() {
                edges.retain(|e| {
                    ids.contains(&e["from"].to_string()) && ids.contains(&e["to"].to_string())
                });
            }
        } else if let Some(edges) = value["edges"].as_array_mut().filter(|e| !e.is_empty()) {
            edges.pop();
            value["edges_truncated"] = json!(true);
        } else {
            break;
        }
    }
    value
}

/// Body bytes one fetch page returns by default; `next` continues the body.
pub(crate) const BODY_PAGE: usize = 16 * 1024;

/// One body page: at most [`BODY_PAGE`] bytes from `offset`, cut on a UTF-8
/// boundary, and less when the record's other fields leave less room.
/// `truncated` says more body follows at `next`; `body_truncated` says this
/// page is not the whole body.
pub(crate) fn body_page(value: Value, offset: usize, etag: Option<&str>) -> d::Result<Value> {
    let mut page = envelope_page(value, offset, etag)?;
    page["truncated"] = json!(!page["next"].is_null());
    Ok(page)
}

fn envelope_page(mut value: Value, offset: usize, etag: Option<&str>) -> d::Result<Value> {
    let mut body = value["body"].as_str().unwrap_or_default().to_owned();
    if offset > body.len() || !body.is_char_boundary(offset) {
        return Err(d::CommandError::new(
            "invalid_body_offset",
            "offset must be a UTF-8 boundary within the body",
        ));
    }
    if (offset > 0 && etag.is_none()) || etag.is_some_and(|e| value["etag"].as_str() != Some(e)) {
        return Err(d::CommandError::new(
            "stale_body",
            "continuation requires the unchanged record etag",
        ));
    }
    let mut end = body.len().min(offset + BODY_PAGE);
    while !body.is_char_boundary(end) {
        end -= 1;
    }
    let rest = end < body.len();
    body.truncate(end);
    value["body_offset"] = json!(offset);
    value["body_truncated"] = json!(offset > 0 || rest);
    value["next"] = if rest { json!(end) } else { Value::Null };
    value["body"] = json!(&body[offset..]);
    if fits(&value) {
        return Ok(value);
    }
    // Reserve the longest cursor and at least one UTF-8 character before
    // allocating space to optional resources. Terminal flags then also fit.
    let probe_end = offset + body[offset..].chars().next().map_or(0, char::len_utf8);
    value["body"] = json!(&body[offset..probe_end]);
    value["body_truncated"] = json!(true);
    value["next"] = json!(u64::MAX);
    let mut metadata_probe = value.clone();
    metadata_probe["resources"] = json!([]);
    if !fits(&metadata_probe) {
        value["frontmatter"] = lean_frontmatter(&value["frontmatter"]);
        value["frontmatter_truncated"] = json!(true);
    }
    while !fits(&value) && value["resources"].as_array().is_some_and(|r| !r.is_empty()) {
        value["resources"].as_array_mut().unwrap().pop();
        value["resources_truncated"] = json!(true);
    }
    if !fits(&value) {
        return Err(d::CommandError::new(
            "invalid_record",
            "record identity exceeds the read envelope",
        ));
    }
    let mut low = offset;
    let mut high = body.len();
    while low < high {
        let mut end = low + (high - low).div_ceil(2);
        while !body.is_char_boundary(end) {
            end -= 1;
        }
        if end == low {
            end += body[low..].chars().next().unwrap().len_utf8();
        }
        if end > high {
            break;
        }
        value["body"] = json!(&body[offset..end]);
        if fits(&value) {
            low = end;
        } else {
            high = end - 1;
        }
    }
    value["body"] = json!(&body[offset..low]);
    let more = low < body.len() || rest;
    value["next"] = if more { json!(low) } else { Value::Null };
    value["body_truncated"] = json!(offset > 0 || more);
    if low == offset && low < body.len() {
        return Err(d::CommandError::new(
            "invalid_record",
            "record leaves no body-page capacity",
        ));
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tiny_utf8_bodies_and_resource_expansion_keep_terminal_frames_bounded() {
        for title_bytes in [250_000, 253_000, 254_000, 255_000] {
            let page = body_page(json!({"etag":"a","frontmatter":{"title":"x".repeat(title_bytes)},"body":"😀","resources":[]}),0,None).unwrap();
            assert!(fits(&page));
            assert_eq!(page["body"], "😀");
            assert!(page["next"].is_null());
        }
        let resources: Vec<_> = (0..128).map(|n| json!({"type":"resource_link","uri":format!("file:///t/{}/{}/{}/{}/{n}.png","%20".repeat(200),"%20".repeat(200),"%20".repeat(200),"%20".repeat(200)),"name":"image","mime_type":"image/png","size":1})).collect();
        let metadata = json!({"id":"one","unknown":{"keep":true}});
        let page = body_page(
            json!({"etag":"a","frontmatter":metadata,"body":"😀","resources":resources}),
            0,
            None,
        )
        .unwrap();
        assert!(fits(&page));
        assert_eq!(page["body"], "😀");
        assert!(page["next"].is_null());
        assert_eq!(page["frontmatter"], metadata);
        assert_eq!(page["resources_truncated"], true);
    }

    #[test]
    fn task_detail_projects_oversized_history_and_keeps_current_verification() {
        let small = json!({"id":Ulid::new(),"path":".bboxy/tasks/a.md","metadata":{"id":Ulid::new(),"title":"a","report":{"summary":"done"},"verify_result":{"exit_code":0}},"body":"brief","body_truncated":false});
        assert_eq!(task_detail(small.clone()), small);
        let mut large = small.clone();
        large["metadata"]["history"] = json!("x".repeat(2 * 1024 * 1024));
        assert!(!fits(&large));
        let page = task_detail(large);
        assert!(fits(&page));
        assert_eq!(page["metadata_truncated"], true);
        assert_eq!(
            page["metadata"]["verify_result"],
            small["metadata"]["verify_result"]
        );
        assert_eq!(page["metadata"]["report"], small["metadata"]["report"]);
        assert_eq!(page["body"], small["body"]);
    }

    #[test]
    fn history_diagnostics_cannot_poison_the_bounded_capture_payload() {
        let data = json!({"format":"capture_records","records":[{"kind":"output","encoding":"base64","bytes":"A".repeat(120*1024)}],"status":{"recovery_notice":"x".repeat(2*1024*1024)},"gaps":[{"reason":"y".repeat(2*1024*1024)}],"next":null});
        let original = super::super::success(Ulid::new(), data.clone());
        assert_eq!(
            super::super::knowledge::bounded(Ulid::new(), original.clone(), false)["structuredContent"]
                ["error"]["code"],
            "response_bound"
        );
        let reply = history_result(original);
        assert!(fits(&reply["structuredContent"]));
        assert_eq!(reply["structuredContent"]["records"], data["records"]);
        assert_eq!(
            reply["structuredContent"]["status"]["recovery_notice_truncated"],
            true
        );
        assert_eq!(
            reply["structuredContent"]["gaps"][0]["reason_truncated"],
            true
        );
        assert_eq!(
            serde_json::from_str::<Value>(reply["content"][0]["text"].as_str().unwrap()).unwrap(),
            reply["structuredContent"]
        );
    }

    #[test]
    fn fetch_byte_pages_reconstruct_two_mib_with_utf8_and_heavy_frontmatter() {
        let body = "é\n\\\"".repeat(420_000);
        assert!(body.len() >= 2 * 1024 * 1024);
        let record = json!({"id":Ulid::new(),"path":"notes/large.md","etag":"revision-a","frontmatter":{"title":"large","history":"x".repeat(2*1024*1024)},"body":body,"resources":[]});
        let broken = super::super::knowledge::bounded(
            Ulid::new(),
            super::super::success(Ulid::new(), record.clone()),
            false,
        );
        assert_eq!(
            broken["structuredContent"]["error"]["code"],
            "response_bound"
        );
        let mut offset = 0;
        let mut restored = String::new();
        loop {
            let page =
                body_page(record.clone(), offset, (offset > 0).then_some("revision-a")).unwrap();
            assert!(fits(&page));
            assert_eq!(page["frontmatter_truncated"], true);
            assert_eq!(page["body_offset"], offset);
            let part = page["body"].as_str().unwrap();
            restored.push_str(part);
            if page["next"].is_null() {
                break;
            }
            let next = page["next"].as_u64().unwrap() as usize;
            assert_eq!(next, offset + part.len());
            assert!(next > offset && body.is_char_boundary(next));
            offset = next;
        }
        assert_eq!(restored, body);
        assert_eq!(
            body_page(record.clone(), 1, Some("revision-a"))
                .unwrap_err()
                .code,
            "invalid_body_offset"
        );
        assert_eq!(
            body_page(record, 2, Some("revision-b")).unwrap_err().code,
            "stale_body"
        );
    }

    // keeps: review 5004 finding 5 — fetch pages a 50 KB body by default.
    #[test]
    fn fetch_pages_bodies_by_default_with_an_explicit_truncated_flag() {
        let body = "ab😀\n".repeat(50 * 1024 / 7 + 1);
        assert!(body.len() >= 50 * 1024);
        let record = json!({"id":Ulid::new(),"path":"notes/fifty.md","etag":"e1","frontmatter":{"title":"fifty"},"body":body,"resources":[]});
        let (mut offset, mut restored, mut pages) = (0, String::new(), 0);
        loop {
            let page = body_page(record.clone(), offset, (offset > 0).then_some("e1")).unwrap();
            pages += 1;
            let part = page["body"].as_str().unwrap();
            assert!(part.len() <= 16 * 1024, "page of {} bytes", part.len());
            restored.push_str(part);
            assert_eq!(page["truncated"], !page["next"].is_null());
            assert_eq!(page["body_truncated"], pages > 1 || !page["next"].is_null());
            let Some(next) = page["next"].as_u64() else {
                break;
            };
            assert_eq!(next as usize, offset + part.len());
            offset = next as usize;
        }
        assert_eq!(restored, body);
        assert!(pages >= body.len().div_ceil(16 * 1024), "{pages} pages");
        let small = body_page(
            json!({"etag":"e","frontmatter":{},"body":"short","resources":[]}),
            0,
            None,
        )
        .unwrap();
        assert_eq!(
            (small["truncated"].clone(), small["next"].clone()),
            (json!(false), Value::Null)
        );
    }

    #[test]
    fn graph_byte_pages_preserve_all_ids_and_reject_uncut_control() {
        let rows: Vec<_> = (0..768)
            .map(|n| json!({"id":format!("{n:026}"),"title":"\0".repeat(512)}))
            .collect();
        let uncut = json!({"graph":"execution","nodes":rows,"edges":[],"next":null});
        assert!(serde_json::to_vec(&uncut).unwrap().len() > 2 * 1024 * 1024);
        assert!(!fits(&uncut));
        let broken = super::super::knowledge::bounded(
            Ulid::new(),
            super::super::success(Ulid::new(), uncut),
            false,
        );
        assert_eq!(
            broken["structuredContent"]["error"]["code"],
            "response_bound"
        );
        let mut offset = 0;
        while offset < rows.len() {
            let page = graph_page(
                json!({"graph":"execution","nodes":rows[offset..].iter().take(128).collect::<Vec<_>>(),"edges":[],"next":null}),
            );
            assert!(fits(&page));
            let nodes = page["nodes"].as_array().unwrap();
            assert!(!nodes.is_empty());
            assert_eq!(nodes.as_slice(), &rows[offset..offset + nodes.len()]);
            offset += nodes.len();
            if !page["next"].is_null() {
                assert_eq!(page["next"], rows[offset - 1]["id"]);
            }
        }
    }
}
