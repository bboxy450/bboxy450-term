//! Bounded, read-only calls for the admitted GUI/ctl transport. These are the
//! existing portable data_source requests, not another document reader.
use super::{DataSource, data_source as d};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;

fn invalid(message: impl Into<String>) -> d::CommandError {
    d::CommandError::new("invalid_provider_result", message)
}
fn decode<T: DeserializeOwned>(value: Value) -> d::Result<T> {
    serde_json::from_value(value)
        .map_err(|error| d::CommandError::new("invalid_request", error.to_string()))
}
fn encode<T: Serialize>(value: T) -> d::Result<Value> {
    serde_json::to_value(value).map_err(|error| invalid(error.to_string()))
}

/// No call can write, acknowledge a delivery, or advance an operator read mark.
/// The caller owns GUI admission; this function owns the portable call bounds.
pub fn read(source: &dyn DataSource, method: &str, args: Value) -> d::Result<Value> {
    bbt_rt::assert_not_ui_thread();
    if serde_json::to_vec(&args)
        .map_err(|e| invalid(e.to_string()))?
        .len()
        > d::MAX_PAGE_BYTES
    {
        return Err(d::CommandError::new(
            "request_too_large",
            "knowledge read exceeds the provider page bound",
        ));
    }
    let result = (|| match method {
        "resolve" => encode(source.resolve(decode(args)?)?),
        "fetch" => encode(source.fetch(decode(args)?)?),
        "list" => {
            let page = source.list(decode(args)?)?;
            if page.items.len() > d::MAX_PAGE {
                return Err(invalid("knowledge list exceeds the provider item bound"));
            }
            encode(page)
        }
        "changes_since" => {
            let page = source.changes_since(decode(args)?)?;
            if page.changed.len() > d::MAX_PAGE {
                return Err(invalid("knowledge changes exceed the provider item bound"));
            }
            encode(page)
        }
        _ => Err(d::CommandError::new(
            "read_only_method",
            "knowledge GUI reads allow resolve, fetch, list and changes_since only",
        )),
    })();
    // Errors have the same wire bound as successful values. A provider cannot
    // turn an error message or ambiguity candidates into an oversized frame.
    if serde_json::to_vec(&result)
        .map_err(|e| invalid(e.to_string()))?
        .len()
        > d::MAX_PAGE_BYTES
    {
        return Err(d::CommandError::new(
            "response_too_large",
            "knowledge read response exceeds the provider page bound",
        ));
    }
    result
}
