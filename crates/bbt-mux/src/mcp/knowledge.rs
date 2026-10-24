//! The first knowledge tools are seat-authenticated and confined to e2e/.
//! General document mutation and operator writes are outside this admission.
use std::path::{Component, Path};

use serde::Deserialize;
use serde_json::{Value, json};

use super::Call;
use crate::knowledge::{DataSource, data_source as d};

pub(crate) fn handles(tool: &str) -> bool {
    matches!(tool, "knowledge_fetch" | "knowledge_write")
}
pub(crate) fn catalogue() -> Vec<Value> {
    let contract = d::method_schema("write", false).expect("contract write schema");
    let mut write = contract["$defs"]["WriteRequest"].clone();
    // Advertise only this tool's fields and referenced type, like the other
    // MCP catalogues. Create/update conditions are enforced by prepare/Index.
    write.as_object_mut().unwrap().remove("allOf");
    write["$defs"] = json!({"RecordRef":contract["$defs"]["RecordRef"]});
    let definition = &mut write;
    definition["additionalProperties"] = json!(false);
    definition["properties"]["op_id"] =
        json!({"type":"string","pattern":"^[0-7][0-9A-HJKMNP-TV-Z]{25}$"});
    definition["required"]
        .as_array_mut()
        .unwrap()
        .push(json!("op_id"));
    definition["properties"]["frontmatter"]["properties"] = json!({"type":{"const":"e2e"}});
    definition["properties"]["frontmatter"]["required"] = json!(["type"]);
    vec![
        json!({"name":"knowledge_fetch","description":"Read an e2e record by path, #N or unambiguous locator. Other document paths are refused.",
            "inputSchema":{"type":"object","properties":{"locator":{"type":"string","minLength":1,"maxLength":4096}},"required":["locator"],"additionalProperties":false},
            "outputSchema":{"type":"object","additionalProperties":true},
            "annotations":{"readOnlyHint":true,"destructiveHint":false,"idempotentHint":true,"openWorldHint":false}}),
        json!({"name":"knowledge_write","description":"Create or update one e2e/ Markdown record with your own generation key. Omit id on create (the operation id is used); updates keep the returned ref and etag. UNKNOWN is never replayed.",
            "inputSchema":write,"outputSchema":{"type":"object","additionalProperties":true},
            "annotations":{"readOnlyHint":false,"destructiveHint":false,"idempotentHint":true,"openWorldHint":false}}),
    ]
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Fetch {
    locator: String,
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
    Write(Box<d::WriteRequest>),
}

fn permitted(path: &str) -> bool {
    let path = Path::new(path);
    let mut components = path.components();
    !path.is_absolute()
        && path.as_os_str().len() <= 4096
        && path.extension().is_some_and(|ext| ext == "md")
        && components.next() == Some(Component::Normal("e2e".as_ref()))
        && components.clone().next().is_some()
        && components.all(|component| matches!(component, Component::Normal(_)))
}

pub(crate) fn prepare(call: &Call) -> Result<Request, String> {
    match call.tool.as_str() {
        "knowledge_fetch" => {
            let args: Fetch = super::tools::parse(&call.args)?;
            if args.locator.is_empty() || args.locator.len() > 4096 {
                return Err("knowledge locator is empty or exceeds its bound".into());
            }
            Ok(Request::Fetch(args.locator))
        }
        "knowledge_write" => {
            let mut args: Write = super::tools::parse(&call.args)?;
            let path = args
                .reference
                .as_ref()
                .map(|r| r.path.as_str())
                .or(args.path.as_deref())
                .ok_or("knowledge create requires an e2e/ Markdown path")?;
            if !permitted(path) {
                return Err("knowledge writes are restricted to e2e/ Markdown paths".into());
            }
            if args.reference.is_some() && args.path.is_some() {
                return Err("knowledge update uses ref, without a path".into());
            }
            let metadata = args
                .frontmatter
                .as_object_mut()
                .ok_or("frontmatter must be an object")?;
            if metadata.get("type").and_then(Value::as_str) != Some("e2e") {
                return Err("knowledge writes require record type e2e".into());
            }
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

pub(crate) fn fetch(source: &dyn DataSource, locator: String) -> Result<d::Record, String> {
    // Do not reflect outside-root candidates or parse failures into a scoped
    // tool response. The lower provider still retains its full command error.
    let reference = source
        .resolve(d::ResolveRequest { locator })
        .map_err(|error| format!("knowledge lookup refused: {}", error.code))?;
    if !permitted(&reference.path) {
        return Err("knowledge reads are restricted to e2e/ Markdown paths".into());
    }
    let record = source
        .fetch(d::FetchRequest { reference })
        .map_err(|error| format!("knowledge fetch refused: {}", error.code))?;
    if record.frontmatter["type"] != "e2e" {
        return Err("knowledge record is not type e2e".into());
    }
    Ok(record)
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
