//! Record reads use the same bounded provider DTOs as the window's document loader.
use super::{Client, USAGE_PREFIX, Value};
use bbt_mux::knowledge::data_source as d;
use serde::Serialize;

fn read(client: &mut Client, method: &'static str, args: impl Serialize) -> Result<Value, String> {
    client
        .knowledge(&crate::knowledge::ReadCall {
            method,
            args: serde_json::to_value(args).map_err(|e| e.to_string())?,
        })
        .map_err(|e| {
            if e.code == "transport" {
                e.message
            } else {
                e.to_string()
            }
        })
}

pub(super) fn fetch(client: &mut Client, locator: String) -> Result<Value, String> {
    let reference = read(client, "resolve", d::ResolveRequest { locator })?;
    let reference =
        serde_json::from_value(reference).map_err(|e| format!("record reference: {e}"))?;
    read(client, "fetch", d::FetchRequest { reference })
}

pub(super) fn search_query(query: &str, record_type: Option<&str>) -> Result<String, String> {
    let mut query = query.to_owned();
    if let Some(record_type) = record_type {
        if record_type.is_empty() || record_type.chars().any(char::is_whitespace) {
            return Err(format!(
                "{USAGE_PREFIX}--type requires one nonempty record type"
            ));
        }
        query.push_str(" type:");
        query.push_str(record_type);
    }
    Ok(query)
}

pub(super) fn search(
    client: &mut Client,
    query: String,
    cursor: Option<String>,
) -> Result<Value, String> {
    read(
        client,
        "search",
        d::SearchRequest {
            query,
            cursor,
            limit: None,
        },
    )
}

pub(super) fn changes(client: &mut Client, cursor: String) -> Result<Value, String> {
    read(
        client,
        "changes_since",
        d::ChangesRequest {
            cursor,
            limit: None,
        },
    )
}

/// This noun's rows of [`super::VERBS`], in parse order.
pub(super) const VERBS: &[super::Verb] = &[
    ("record", "write", "RecordWrite"),
    ("record", "fetch", "Knowledge"),
    ("record", "search", "Knowledge"),
    ("record", "changes", "Knowledge"),
];
