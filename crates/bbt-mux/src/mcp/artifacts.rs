// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use crate::{
    Ulid,
    provenance::stored::{Address, MAX_FILE, MAX_INLINE, MAX_PINS, Revision},
    store::{Result, digest, invalid},
};
use serde::Deserialize;
use serde_json::{Value, json};

pub(crate) mod staging;

pub(crate) struct CanvasReads<'a> {
    pub(crate) store: crate::provenance::stored::Store<'a>,
    pub(crate) caller: Option<Ulid>,
    pub(crate) runtime: &'a std::path::Path,
}

const PAGE_BYTES: usize = 64 * 1024;
const URI_PREFIX: &str = "bboxy://artifact/";

pub(crate) fn mutation(tool: &str) -> bool {
    matches!(tool, "artifact_store" | "artifact_update")
}

pub(crate) fn read_tool(tool: &str) -> bool {
    matches!(tool, "artifact_read" | "artifact_resource_read")
}
pub(crate) fn resource_uri(uri: &str) -> bool {
    uri.starts_with(URI_PREFIX)
}

pub(crate) fn failure(op: Ulid, error: &dyn std::error::Error) -> Value {
    let text = error.to_string();
    let (code, message) = text
        .split_once(": ")
        .unwrap_or(("invalid_arguments", &text));
    let stable = match code {
        "not_stored"
        | "op_id_reused"
        | "execution_source_unavailable"
        | "execution_source_changed"
        | "artifact_unavailable"
        | "artifact_forbidden"
        | "artifact_input_limit"
        | "artifact_conflict"
        | "artifact_revision_not_found"
        | "artifact_immutable"
        | "artifact_custody_conflict"
        | "artifact_too_large"
        | "artifact_image_refused"
        | "artifact_source_unavailable"
        | "artifact_source_forbidden"
        | "artifact_source_changed"
        | "unknown" => Some(code),
        _ => None,
    };
    super::error(
        op,
        stable.unwrap_or("rejected"),
        if stable.is_some() { message } else { &text },
    )
}

pub(crate) fn read(
    store: &crate::provenance::stored::Store<'_>,
    caller: Option<Ulid>,
    call: &super::Call,
) -> Value {
    let result = (|| -> Result<_> {
        if call.tool == "artifact_resource_read" {
            let page = Page::parse(&call.args)?;
            let (revision, bytes) =
                store.read(page.address.id, Some(page.address.revision), caller)?;
            return resource_result(call.op_id, revision, &bytes, page.offset);
        }
        let request: Read = serde_json::from_value(call.args.clone())?;
        Address {
            id: request.id,
            revision: request.revision.unwrap_or(1),
        }
        .validate()?;
        let (revision, bytes) = store.read(request.id, request.revision, caller)?;
        Ok(read_result(call.op_id, revision, &bytes))
    })();
    result.unwrap_or_else(|error| failure(call.op_id, error.as_ref()))
}

pub(super) fn argument_limit(tool: &str) -> usize {
    if mutation(tool) {
        MAX_INLINE + 16 * 1024
    } else {
        super::MAX_ARGS
    }
}

pub(crate) fn catalogue() -> Vec<Value> {
    let id = json!({"type":"string","pattern":"^[0-7][0-9A-HJKMNP-TV-Z]{25}$"});
    let revision = json!({"type":"integer","minimum":1});
    let payload = json!({
        "content":{"description":"Inline JSON or UTF-8 text, up to 262144 bytes. Supply this or source_path."},
        "source_path":{"type":"string","minLength":1,"maxLength":4096,"description":"Regular staged file inside your admitted workspace, up to 16777216 bytes. Supply this or content."},
        "inputs":{"type":"array","maxItems":MAX_PINS,"items":{"type":"object","properties":{"id":id,"revision":revision},"required":["id","revision"],"additionalProperties":false}}
    });
    let properties = |base: Value| {
        let mut fields = payload.as_object().expect("constant schema").clone();
        fields.extend(base.as_object().expect("constant schema").clone());
        Value::Object(fields)
    };
    [
        ("artifact_store", "Store bytes in your current native root as an artifact at revision 1. Declare exact revision input pins.", properties(json!({"media_type":{"type":"string","minLength":3,"maxLength":127}})), json!(["media_type"])),
        ("artifact_update", "Update a stored SVG at its expected current revision. Supply the full set of declared input pins for the new revision.", properties(json!({"id":id,"expected_revision":revision})), json!(["id","expected_revision"])),
        ("artifact_read", "Read the latest or exact retained stored artifact revision with its input pins. Binary and large results return a resource link.", json!({"id":id,"revision":revision}), json!(["id"])),
    ].into_iter().map(|(name, description, properties, required)| {
        let read = !mutation(name);
        let mut schema = json!({"type":"object","properties":properties,"required":required,"additionalProperties":false});
        if !read {
            schema["oneOf"] = json!([{"required":["content"]},{"required":["source_path"]}]);
        }
        json!({"name":name,"description":description,"inputSchema":schema,
            "outputSchema":{"type":"object","additionalProperties":true},
            "annotations":{"readOnlyHint":read,"destructiveHint":false,"idempotentHint":true,"openWorldHint":false}})
    }).collect()
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Read {
    pub(crate) id: Ulid,
    pub(crate) revision: Option<u64>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Resource {
    uri: String,
}

pub(crate) struct Page {
    pub(crate) address: Address,
    pub(crate) offset: usize,
}

impl Page {
    pub(crate) fn parse(args: &Value) -> Result<Self> {
        let Resource { uri } = serde_json::from_value(args.clone())?;
        let path = uri
            .strip_prefix(URI_PREFIX)
            .ok_or_else(|| invalid("invalid artifact resource"))?;
        let (address, offset) = path.split_once("?offset=").unwrap_or((path, "0"));
        let (id, revision) = address
            .split_once('/')
            .ok_or_else(|| invalid("artifact resource requires an exact revision"))?;
        let address = Address {
            id: id.parse()?,
            revision: revision.parse()?,
        };
        address.validate()?;
        let offset = offset.parse()?;
        if offset > MAX_FILE || offset % PAGE_BYTES != 0 {
            return Err(invalid("invalid artifact resource page offset"));
        }
        Ok(Self { address, offset })
    }
}

pub(crate) enum Payload {
    Inline(Value),
    Staged(String),
}

pub(crate) struct Prepared {
    pub(crate) write: Write,
    pub(crate) bytes: Option<Vec<u8>>,
    pub(crate) source: Option<crate::mcp::artifact_source::Source>,
}

pub(crate) struct Write {
    pub(crate) expected: Option<Address>,
    pub(crate) media_type: String,
    pub(crate) inputs: Vec<Address>,
    pub(crate) payload: Payload,
    pub(crate) fingerprint: String,
}

fn media_type(value: &str) -> Result<String> {
    let token = |value: &str| {
        !value.is_empty()
            && value
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"!#$&^_.+-".contains(&b))
    };
    if value.len() > 127
        || !value
            .split_once('/')
            .is_some_and(|(kind, subtype)| token(kind) && token(subtype))
    {
        return Err(invalid("invalid artifact media type"));
    }
    Ok(value.to_ascii_lowercase())
}

impl Write {
    pub(crate) fn parse(call: &super::Call) -> Result<Self> {
        let mut args = call
            .args
            .as_object()
            .cloned()
            .ok_or_else(|| invalid("arguments must be an object"))?;
        let payload = match (args.remove("content"), args.remove("source_path")) {
            (Some(value), None) => Payload::Inline(value),
            (None, Some(Value::String(path))) if !path.is_empty() && path.len() <= 4096 => {
                Payload::Staged(path)
            }
            _ => return Err(invalid("supply exactly one of content or source_path")),
        };
        let inputs: Vec<Address> = args
            .remove("inputs")
            .map(serde_json::from_value)
            .transpose()?
            .unwrap_or_default();
        if inputs.len() > MAX_PINS {
            return Err(invalid("artifact_input_limit: at most 32 exact input pins"));
        }
        for input in &inputs {
            input.validate()?;
        }
        let (expected, media_type) = if call.tool == "artifact_update" {
            let id = args.remove("id").ok_or_else(|| invalid("id is required"))?;
            let revision = args
                .remove("expected_revision")
                .ok_or_else(|| invalid("expected_revision is required"))?;
            let address = Address {
                id: serde_json::from_value(id)?,
                revision: serde_json::from_value(revision)?,
            };
            address.validate()?;
            (Some(address), crate::provenance::stored::SVG.to_owned())
        } else {
            let name = args
                .remove("media_type")
                .ok_or_else(|| invalid("media_type is required"))?;
            (
                None,
                media_type(
                    name.as_str()
                        .ok_or_else(|| invalid("media_type must be text"))?,
                )?,
            )
        };
        if !args.is_empty() {
            return Err(invalid("unknown artifact arguments"));
        }
        let mut canonical = json!({"tool":call.tool,"arguments":call.args});
        canonical.sort_all_objects();
        Ok(Self {
            expected,
            media_type,
            inputs,
            payload,
            fingerprint: digest(&serde_json::to_vec(&canonical)?),
        })
    }
}

fn json_media(media: &str) -> bool {
    media == "application/json" || media.ends_with("+json")
}

fn text_media(media: &str) -> bool {
    media.starts_with("text/") || media == crate::provenance::stored::SVG
}

pub(crate) fn inline_bytes(media: &str, value: Value) -> Result<Vec<u8>> {
    let bytes = if json_media(media) {
        serde_json::to_vec(&value)?
    } else if text_media(media) {
        match value {
            Value::String(text) => text.into_bytes(),
            _ => return Err(invalid("artifact media type requires UTF-8 text")),
        }
    } else {
        return Err(invalid("use source_path for this media type"));
    };
    if bytes.len() > MAX_INLINE {
        return Err(invalid(
            "artifact_too_large: inline content exceeds 262144 bytes",
        ));
    }
    Ok(bytes)
}

pub(super) fn uri(address: Address, offset: usize) -> String {
    let Address { id, revision } = address;
    let mut uri = format!("{URI_PREFIX}{id}/{revision}");
    if offset != 0 {
        use std::fmt::Write as _;
        write!(uri, "?offset={offset}").expect("write to String");
    }
    uri
}

pub(crate) fn read_result(op: Ulid, revision: Revision, bytes: &[u8]) -> Value {
    let mut data = revision.value(false);
    if bytes.len() <= PAGE_BYTES {
        let content = if json_media(&revision.media_type) {
            serde_json::from_slice(bytes).ok()
        } else if text_media(&revision.media_type) {
            std::str::from_utf8(bytes).ok().map(|text| json!(text))
        } else {
            None
        };
        if let Some(content) = content {
            data["content"] = content;
            let result = super::success(op, data.clone());
            if serde_json::to_vec(&result).is_ok_and(|bytes| bytes.len() <= super::MAX_RESULT) {
                return result;
            }
            data.as_object_mut()
                .expect("revision data")
                .remove("content");
        }
    }
    let link = json!({"type":"resource_link","uri":uri(revision.address, 0),
        "name":revision.address.id.to_string(),"mimeType":revision.media_type,"size":bytes.len()});
    data["resource"] = link.clone();
    let mut result = super::success(op, data);
    result["content"]
        .as_array_mut()
        .expect("MCP content")
        .push(link);
    result
}

pub(crate) fn resource_result(
    op: Ulid,
    revision: Revision,
    bytes: &[u8],
    offset: usize,
) -> Result<Value> {
    use base64::{Engine, engine::general_purpose::STANDARD};
    let end = (offset + PAGE_BYTES).min(bytes.len());
    let chunk = bytes
        .get(offset..end)
        .ok_or_else(|| invalid("artifact resource offset exceeds retained bytes"))?;
    let address = revision.address;
    let mut part = json!({"uri":uri(address, offset),"mimeType":revision.media_type,"blob":STANDARD.encode(chunk),
        "_meta":{"bboxy450/digest":revision.digest,"bboxy450/offset":offset,"bboxy450/bytes":bytes.len()}});
    if end < bytes.len() {
        part["_meta"]["bboxy450/next"] = json!(uri(address, end));
    }
    Ok(super::success(op, json!({"contents":[part]})))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inline_json_null_and_utf8_limits_are_exact() {
        assert_eq!(
            inline_bytes("application/json", Value::Null).unwrap(),
            b"null"
        );
        let exact = "é".repeat(MAX_INLINE / 2);
        assert_eq!(
            inline_bytes("text/plain", json!(exact)).unwrap().len(),
            MAX_INLINE
        );
        assert!(inline_bytes("text/plain", json!(exact + "x")).is_err());
        assert!(inline_bytes("image/png", json!("not binary custody")).is_err());
    }

    #[test]
    fn revision_resources_are_pinned_and_page_bounded() {
        let id = Ulid(17);
        let address = Address { id, revision: 2 };
        let args = json!({"uri":uri(address, PAGE_BYTES)});
        let parsed = Page::parse(&args).unwrap();
        assert_eq!(parsed.address.id, id);
        assert_eq!(parsed.address.revision, 2);
        assert_eq!(parsed.offset, PAGE_BYTES);
        assert!(Page::parse(&json!({"uri":uri(address, 1)})).is_err());
        assert!(Page::parse(&json!({"uri":format!("{URI_PREFIX}{id}")})).is_err());
    }
}
