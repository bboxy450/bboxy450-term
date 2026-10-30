// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Plugin-provided seat tools.
//!
//! Every enabled, checked plugin of the seat's knowledge root contributes:
//! - its contract methods as `plugin_<contract>_<provider>_<method>`, and
//! - its declared `tools` as `plugin_tool_<provider>_<name>` (the plugin
//!   serves them as MCP tools `tool_<name>`).
//!
//! Rules:
//! - Role comes from the contract (`Contract::role`) or the declared tool.
//!   Worker seats see worker tools; oracle seats see worker and oracle tools.
//!   Host-only methods (event_sink) are never exposed.
//! - A method whose request requires a host-owned field (`plugins::HOST_OWNED`)
//!   stays with the host, as do data_source write/commit: signing belongs to
//!   the host, so seats use the native knowledge and task tools for those.
//!   Seat arguments never carry a host-owned field.
//! - inputSchema/outputSchema come from the plugin's declared command (or
//!   tool) schemas; calls are validated against them and, for contract
//!   methods, against the contract schema too.
//! - Contract reads (resolve, fetch, search, list, links, graph, status,
//!   describe, changes_since) are journaled reads. Every other method and
//!   every declared tool is an effect, admitted once per op_id and never
//!   re-dispatched; an interrupted effect stays `unknown`.
//! - A result is returned as `{"result": <plugin result>}`.
//! - The catalogue is a per-root snapshot, refreshed when the workspace mounts
//!   and after every plugin control mutation. `tools/list` reads it each time
//!   and `listChanged` stays false: a harness that caches its first list sees
//!   a newly enabled plugin in its next session. A call rereads the plugin's
//!   current authority, so a stale entry is refused, never served.
use crate::{
    knowledge::{seat_reads::Completion, workspaces::RootBinding},
    mcp::{self, Call, SeatActor, journal::Journal},
    plugins::{PluginHost, SeatTool},
    registry::Class,
};
use serde_json::{Map, Value, json};
use std::time::{Duration, Instant};
use tokio::sync::{OwnedSemaphorePermit, oneshot};

/// One overall budget under the seat bridge's 10 s socket timeout.
const BUDGET: Duration = Duration::from_secs(8);

pub(crate) fn handles(tool: &str) -> bool {
    tool.starts_with("plugin_") && tool != "plugin_control"
}

fn visible(tool: &SeatTool, actor: &SeatActor) -> bool {
    !tool.oracle || actor.role.class == Class::Oracle
}

/// Split a standalone schema into its definitions and a body that refers to
/// them from the document root.
fn hoist(schema: &Value) -> (Map<String, Value>, Value) {
    let mut body = schema.clone();
    let defs = body
        .as_object_mut()
        .and_then(|object| {
            object.remove("$schema");
            object.remove("$defs")
        })
        .and_then(|defs| defs.as_object().cloned())
        .unwrap_or_default();
    (defs, body)
}

fn input_schema(schema: &Value) -> Value {
    let (defs, body) = hoist(schema);
    let mut input = if body["type"] == "object" && body.get("$ref").is_none() {
        body
    } else {
        json!({"type":"object","allOf":[body]})
    };
    if !defs.is_empty() {
        input["$defs"] = Value::Object(defs);
    }
    if !input["properties"].is_object() {
        input["properties"] = json!({});
    }
    if !input["required"].is_array() {
        input["required"] = json!([]);
    }
    input
}

fn output_schema(schema: &Value) -> Value {
    let (defs, body) = hoist(schema);
    let mut output = json!({"type":"object","properties":{"result":body},"required":["result"]});
    if !defs.is_empty() {
        output["$defs"] = Value::Object(defs);
    }
    output
}

/// The actor's plugin tools, in the shape of the native catalogue entries.
pub(crate) fn catalogue(tools: &[SeatTool], actor: &SeatActor) -> Vec<Value> {
    tools
        .iter()
        .filter(|tool| visible(tool, actor))
        .map(|tool| {
            json!({
                "name": tool.name,
                "description": tool.description,
                "inputSchema": input_schema(&tool.input_schema),
                "outputSchema": output_schema(&tool.output_schema),
                "annotations": {"readOnlyHint": tool.read, "destructiveHint": false,
                    "idempotentHint": tool.read, "openWorldHint": true},
            })
        })
        .collect()
}

pub(crate) struct Job {
    pub plugins: PluginHost,
    pub actor: SeatActor,
    pub binding: RootBinding,
    pub revision: u64,
    pub call: Call,
    pub journal: Journal,
    pub reply: oneshot::Sender<crate::store::Result<Value>>,
    pub permit: OwnedSemaphorePermit,
}

impl Job {
    /// Blocking plugin IO; the logic owner revalidates seat and root on completion.
    pub(crate) fn run(self) -> Completion {
        let Self {
            plugins,
            actor,
            binding,
            revision,
            call,
            journal,
            reply,
            permit,
        } = self;
        let _permit = permit;
        let current = || {
            plugins
                .seat_tools()
                .iter()
                .find(|tool| tool.name == call.tool && visible(tool, &actor))
                .cloned()
        };
        let result = match current() {
            None => journal.checked(
                &actor,
                &call,
                mcp::error(
                    call.op_id,
                    "unknown_tool",
                    "no enabled plugin offers this tool to your role",
                ),
            ),
            Some(tool) => {
                let dispatch = || match plugins.call_seat_tool(
                    &tool,
                    call.args.clone(),
                    Instant::now() + BUDGET,
                ) {
                    Ok(value) => mcp::success(call.op_id, json!({"result": value})),
                    Err(reason) => mcp::error_retry(
                        call.op_id,
                        &reason.code,
                        &reason.message,
                        reason.retry_after_ms,
                    ),
                };
                if tool.read {
                    journal.checked_authorized(&actor, &call, dispatch(), |_| current().is_some())
                } else {
                    journal.effect(&actor, &call, dispatch)
                }
            }
        };
        Completion {
            actor,
            binding,
            revision,
            reply,
            result,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // keeps: plugin seat tools — a busy refusal keeps its retry hint in both
    // the structured and the text form; other refusals carry none.
    #[test]
    fn busy_plugin_tool_refusals_carry_retry_after_ms() {
        let op = crate::Ulid::from(7_u128);
        let busy = mcp::error_retry(op, "provider_busy", "busy", Some(100));
        assert_eq!(busy["isError"], true);
        assert_eq!(busy["structuredContent"]["error"]["retry_after_ms"], 100);
        let text: Value =
            serde_json::from_str(busy["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(text, busy["structuredContent"]);
        let plain = mcp::error_retry(op, "rejected", "no", None);
        assert_eq!(plain, mcp::error(op, "rejected", "no"));
    }
}
