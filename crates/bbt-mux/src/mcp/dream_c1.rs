// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Own-seat Dream C1 controls. Agent arguments never supply authority or paths.
use super::Call;
use bbt_proto::dream::{self as w, Control, ControlReply};
use serde_json::{Value, json};

pub(crate) fn handles(name: &str) -> bool {
    name == "dream_c1"
}

pub(crate) fn catalogue() -> Vec<Value> {
    let id = json!({"type":"string","pattern":"^[0-7][0-9A-HJKMNP-TV-Z]{25}$"});
    vec![json!({
        "name":"dream_c1",
        "description":"Inspect exact retained closed sessions of your own seat, read an immutable run/status page, or cancel its owned worker. Inspect returns admission/deferred/error promptly. Open, idle, turn-end and native EOF never authorize inspection. Status/Cancel remain scoped to the current admitted actor; Oracle role does not expose peer runs. Retain the original op_id after an unknown outcome; do not start a replacement operation.",
        "inputSchema":{
            "type":"object",
            "properties":{
                "v":{"type":"integer","const":1},
                "method":{"type":"string","enum":["inspect","status","cancel"]},
                "sources":{"type":"array","minItems":1,"maxItems":32,"uniqueItems":true,
                    "items":{"type":"object","properties":{
                        "session":id,
                        "source_generation":{"type":"integer","minimum":1,"maximum":w::MAX_SAFE}
                    },"required":["session","source_generation"],"additionalProperties":false}},
                "run_id":id,
                "page":{"anyOf":[{"type":"integer","minimum":0,"maximum":u32::MAX},{"type":"null"}]}
            },
            "required":["v","method"],"additionalProperties":false,
            "oneOf":[
                {"properties":{"method":{"const":"inspect"}},"required":["sources"],"not":{"anyOf":[{"required":["run_id"]},{"required":["page"]}]}},
                {"properties":{"method":{"const":"status"}},"required":["run_id"],"not":{"required":["sources"]}},
                {"properties":{"method":{"const":"cancel"}},"required":["run_id"],"not":{"anyOf":[{"required":["sources"]},{"required":["page"]}]}}
            ]
        },
        "outputSchema":{"type":"object","additionalProperties":true},
        "annotations":{"readOnlyHint":false,"destructiveHint":false,"idempotentHint":true,"openWorldHint":false}
    })]
}

/// The bridge removes retry metadata from args. Inject its original Call ID only
/// into the two public controls which own an operation ID; Status stays exact.
pub(crate) fn parse(call: &Call) -> Result<Control, String> {
    if !handles(&call.tool) || call.op_id.is_nil() {
        return Err("bad_request".into());
    }
    let mut args = call.args.clone();
    let object = args.as_object_mut().ok_or("bad_request")?;
    match object.get("method").and_then(Value::as_str) {
        Some("inspect" | "cancel") => {
            if let Some(provided) = object.get("op_id")
                && provided != &json!(call.op_id)
            {
                return Err("op_id_reused".into());
            }
            object.insert("op_id".into(), json!(call.op_id));
        }
        Some("status") => {}
        _ => return Err("bad_request".into()),
    }
    let control: Control = serde_json::from_value(args).map_err(|_| "bad_request")?;
    control.validate().map_err(|_| "bad_request")?;
    Ok(control)
}

/// Call only with the already bounded immutable public response. This wrapper
/// adds the normal MCP operation metadata, never a second Dream result shape.
pub(crate) fn success(op_id: ulid::Ulid, reply: ControlReply) -> Value {
    super::success(
        op_id,
        serde_json::to_value(reply).expect("serializable Dream C1 reply"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn call(args: Value) -> Call {
        Call {
            op_id: ulid::Ulid::from(1_u128),
            tool: "dream_c1".into(),
            args,
        }
    }
    #[test]
    fn dream_control_preserves_host_operation_and_exact_status_shape() {
        let parsed = parse(&call(json!({"v":1,"method":"inspect","sources":[{"session":"00000000000000000000000002","source_generation":1}]}))).unwrap();
        let Control::Inspect(request) = parsed else {
            panic!("inspect changed method")
        };
        assert_eq!(request.op_id.0, "00000000000000000000000001");
        let parsed = parse(&call(
            json!({"v":1,"method":"status","run_id":"00000000000000000000000003"}),
        ))
        .unwrap();
        assert!(serde_json::to_value(parsed).unwrap().get("op_id").is_none());
        let parsed = parse(&call(
            json!({"v":1,"method":"cancel","run_id":"00000000000000000000000003"}),
        ))
        .unwrap();
        assert_eq!(
            serde_json::to_value(parsed).unwrap()["op_id"],
            "00000000000000000000000001"
        );
    }
    #[test]
    fn dream_control_refuses_injected_authority_paths_versions_and_sources() {
        let valid = json!({"v":1,"method":"inspect","sources":[{"session":"00000000000000000000000002","source_generation":1}]});
        for field in ["seat", "root", "context", "path", "provider"] {
            let mut args = valid.clone();
            args[field] = json!("caller input");
            assert!(parse(&call(args)).is_err());
        }
        for sources in [
            json!([]),
            json!([{"session":"00000000000000000000000002","source_generation":0}]),
            json!([{"session":"00000000000000000000000002","source_generation":1},{"session":"00000000000000000000000002","source_generation":2}]),
        ] {
            let mut args = valid.clone();
            args["sources"] = sources;
            assert!(parse(&call(args)).is_err());
        }
        for version in [json!(0), json!(2), json!(1.0)] {
            let mut args = valid.clone();
            args["v"] = version;
            assert!(parse(&call(args)).is_err());
        }
        let mut conflict = valid;
        conflict["op_id"] = json!("00000000000000000000000003");
        assert_eq!(parse(&call(conflict)).unwrap_err(), "op_id_reused");
        assert!(parse(&call(json!({"v":1,"method":"status","run_id":"00000000000000000000000003","op_id":"00000000000000000000000001"}))).is_err());
    }
}
