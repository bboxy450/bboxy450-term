//! Own-seat attention tools; receiver identity comes only from admission.

use bbt_proto::registry::AttentionRule;
use serde_json::{Value, json};

use super::{Call, tools};
use crate::{messaging::attention, registry::SeatRecord};

pub(crate) enum Request {
    Get,
    Set(AttentionRule),
}

pub(crate) fn handles(name: &str) -> bool {
    matches!(name, "attention_get" | "attention_set")
}

pub(crate) fn catalogue() -> Vec<Value> {
    [
        (
            "attention_get",
            "Read your receiver-owned wake overrides and role defaults. Inbox storage and operator mute are separate from waking a model.",
            json!({}),
            json!([]),
            true,
        ),
        (
            "attention_set",
            "Set one destination and class to wake, inbox or digest for your own seat only. Conversation from the operator, delegated work and replies in your own thread always wake. This never mutes traffic or changes channel grants.",
            json!({
                "destination":{"type":"string","minLength":1,"maxLength":230},
                "class":{"enum":["conversation","event"]},
                "mode":{"enum":["wake","inbox","digest"]}
            }),
            json!(["destination","class","mode"]),
            false,
        ),
    ]
    .into_iter()
    .map(|(name, description, properties, required, read)| {
        json!({"name":name,"description":description,
            "inputSchema":{"type":"object","properties":properties,"required":required,"additionalProperties":false},
            "outputSchema":{"type":"object","additionalProperties":true},
            "annotations":{"readOnlyHint":read,"destructiveHint":false,"idempotentHint":true,"openWorldHint":false}})
    })
    .collect()
}

pub(crate) fn prepare(call: &Call) -> Result<Request, String> {
    match call.tool.as_str() {
        "attention_get" => {
            let _: tools::Empty = tools::parse(&call.args)?;
            Ok(Request::Get)
        }
        "attention_set" => {
            let mut rule: AttentionRule = tools::parse(&call.args)?;
            rule.destination =
                attention::destination(&rule.destination).map_err(|e| e.to_string())?;
            Ok(Request::Set(rule))
        }
        _ => Err("unknown attention tool".into()),
    }
}

pub(crate) fn view(record: &SeatRecord) -> Result<Value, String> {
    if let Some(policy) = &record.attention {
        attention::validate(policy).map_err(|e| e.to_string())?;
    }
    Ok(json!({
        "seat_id":record.id,"role":record.role,
        "policy":record.attention.clone().unwrap_or_default(),
        "defaults":attention::defaults(&record.role),
        "always_wake":["operator_conversation","delegated_task","own_thread_reply"],
        "muted":record.muted
    }))
}
