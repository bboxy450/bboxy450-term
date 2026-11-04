// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! MCP hint graph: actionable next reads, projected without executing or altering receipts.
use serde_json::{Value, json};

use super::Call;
use crate::knowledge::types::DOCUMENT_TYPES;

fn tool(name: &str, arguments: Value, reason: &str) -> Value {
    json!({"reason":reason,"method":"tools/call","params":{"name":name,"arguments":arguments}})
}

fn catalogue() -> Value {
    json!({"reason":"Read your permitted tools and exact input schemas, then correct the arguments.","method":"tools/list","params":{}})
}

pub(crate) fn recovery(op: crate::Ulid) -> Vec<Value> {
    vec![tool(
        "operation_status",
        json!({"original_op_id":op}),
        "Read the original operation outcome before retrying; never repeat an unknown effect.",
    )]
}

/// All tool edges target reads available to both worker and Oracle roles.
fn next(call: &Call, result: &Value) -> Vec<Value> {
    let data = &result["structuredContent"];
    if data["status"] == "unknown"
        || matches!(
            data["error"]["code"].as_str(),
            Some("unknown" | "op_id_reused")
        )
    {
        let original = call.args["original_op_id"]
            .as_str()
            .and_then(|id| id.parse().ok())
            .unwrap_or(call.op_id);
        return recovery(original);
    }
    if result["isError"] == true {
        let code = data["error"]["code"].as_str().unwrap_or_default();
        let message = data["error"]["message"].as_str().unwrap_or_default();
        if code == "document_type_prefix_required"
            || message.contains("document_type_prefix_required")
        {
            return vec![tool(
                "knowledge_types",
                json!({}),
                "Choose a supported document type and use its directory before the slug.",
            )];
        }
        if matches!(
            code,
            "swarm_root_changed" | "root_changed" | "workspace_pending" | "forbidden"
        ) {
            return vec![tool(
                "orient",
                json!({}),
                "Refresh your current workspace, role and permitted tools.",
            )];
        }
        return vec![catalogue()];
    }
    let hint = match call.tool.as_str() {
        "orient" => {
            return vec![
                tool(
                    "skill_list",
                    json!({}),
                    "Discover the workflow and selected provider's guidance.",
                ),
                tool(
                    "knowledge_types",
                    json!({}),
                    "Choose a document type and learn how to write it.",
                ),
                tool(
                    "task_list",
                    json!({}),
                    "Read the tasks visible to your seat.",
                ),
            ];
        }
        "knowledge_types" | "graph_read" => {
            let query = call.args["type"]
                .as_str()
                .map(|kind| format!("type:{kind}"))
                .unwrap_or_default();
            tool(
                "knowledge_search",
                json!({"query":query}),
                "Find existing documents before creating or changing one.",
            )
        }
        "knowledge_search" => match data["items"][0]["ref"]["path"].as_str() {
            Some(path) => tool(
                "knowledge_fetch",
                json!({"locator":path}),
                "Read the first matching document and retain its reference and etag.",
            ),
            None => tool(
                "knowledge_types",
                json!({}),
                "Review document purposes, paths and writing instructions.",
            ),
        },
        "knowledge_write" => match data["record"]["path"].as_str() {
            Some(path) => tool(
                "knowledge_fetch",
                json!({"locator":path}),
                "Read back the document that was written.",
            ),
            None => tool(
                "knowledge_types",
                json!({}),
                "Review the supported document types and write conventions.",
            ),
        },
        "knowledge_fetch" => {
            let kind = data["frontmatter"]["type"].as_str();
            let args =
                match kind.filter(|kind| DOCUMENT_TYPES.iter().any(|entry| entry.kind == *kind)) {
                    Some(kind) => json!({"type":kind}),
                    None => json!({}),
                };
            tool(
                "knowledge_types",
                args,
                "Read the document type's purpose and update guidance before editing.",
            )
        }
        "knowledge_status" => tool(
            "skill_list",
            json!({}),
            "Read the selected provider's current guidance.",
        ),
        "skill_list" => match data["skills"][0]["uri"].as_str() {
            Some(uri) => tool(
                "skill_read",
                json!({"uri":uri}),
                "Read a discovered skill using its exact current URI.",
            ),
            None => tool(
                "knowledge_types",
                json!({}),
                "Read the host's built-in document guidance.",
            ),
        },
        "skill_read" => tool(
            "knowledge_types",
            json!({}),
            "Choose the appropriate document type for this workflow.",
        ),
        "self_status" => tool(
            "task_list",
            json!({}),
            "Read the work currently visible to your seat.",
        ),
        "task_status" => tool(
            "graph_read",
            json!({"graph":"task"}),
            "Inspect the task's surrounding work and dependencies.",
        ),
        name if name.starts_with("task_") => match call.args["task_path"].as_str() {
            Some(path) => tool(
                "task_status",
                json!({"task_path":path}),
                "Read the current task state and verification requirements.",
            ),
            None => tool(
                "orient",
                json!({}),
                "Read your current tasks and permitted next actions.",
            ),
        },
        name if name.starts_with("artifact_") => tool(
            "knowledge_search",
            json!({"query":"type:artifact"}),
            "Find artifact records and their provenance.",
        ),
        _ => tool(
            "orient",
            json!({}),
            "Read your current context and permitted next actions.",
        ),
    };
    vec![hint]
}

/// Preserve the tool's structured output/schema and signed receipt bytes.
/// Hints are visible content plus MCP metadata, outside the provider's output schema.
pub(crate) fn attach(call: &Call, mut result: Value) -> Value {
    let hints = next(call, &result);
    let text = json!({"hints":hints}).to_string();
    let block = json!({"type":"text","text":text});
    let size = serde_json::to_vec(&result).map_or(usize::MAX, |bytes| bytes.len());
    let extra = serde_json::to_vec(&block)
        .map_or(usize::MAX, |bytes| bytes.len())
        .saturating_add(serde_json::to_vec(&hints).map_or(usize::MAX, |bytes| bytes.len()));
    // Leave room for the enclosing JSON-RPC response; guidance never breaks a large result.
    if size.saturating_add(extra).saturating_add(2048) >= super::bridge::MAX_FRAME {
        return result;
    }
    if let Some(content) = result["content"].as_array_mut() {
        content.push(block);
        result["_meta"]["bboxy450/hints"] = json!(hints);
    }
    result
}

pub(crate) fn protocol(code: i64, message: &str) -> Vec<Value> {
    if code == -32002 && message == "Initialize first" {
        vec![
            json!({"reason":"Initialize the MCP connection before listing or calling tools.","method":"initialize","params":{
                "protocolVersion":super::bridge::VERSIONS[0],"clientInfo":{"name":"harness","version":"1"},"capabilities":{}
            }}),
        ]
    } else {
        vec![catalogue()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Ulid,
        mcp::SeatActor,
        registry::{Class, Role},
    };

    #[test]
    fn mcp_hint_graph_keeps_tool_data_and_unknown_operation_identity() {
        let op = Ulid::from(1_u128);
        let call = Call {
            op_id: op,
            tool: "knowledge_write".into(),
            args: json!({}),
        };
        let receipt = super::super::error(op, "unknown", "original outcome unknown");
        let hinted = attach(&call, receipt.clone());
        assert_eq!(hinted["structuredContent"], receipt["structuredContent"]);
        assert_eq!(hinted["content"][0], receipt["content"][0]);
        let hints = hinted["_meta"]["bboxy450/hints"].as_array().unwrap();
        assert_eq!(hints.len(), 1);
        assert_eq!(hints[0]["params"]["name"], "operation_status");
        assert_eq!(hints[0]["params"]["arguments"]["original_op_id"], json!(op));
        assert_eq!(
            serde_json::from_str::<Value>(hinted["content"][1]["text"].as_str().unwrap()).unwrap()
                ["hints"],
            json!(hints)
        );
    }

    #[test]
    fn mcp_hint_graph_never_makes_a_large_result_exceed_the_frame() {
        let call = Call {
            op_id: crate::Ulid::from(1_u128),
            tool: "orient".into(),
            args: json!({}),
        };
        let result = json!({"content":[{"type":"text","text":"x".repeat(super::super::bridge::MAX_FRAME - 1024)}]});
        assert_eq!(attach(&call, result.clone()), result);
    }

    #[test]
    fn mcp_hint_graph_edges_use_role_permitted_tools_and_valid_arguments() {
        for class in [Class::Worker, Class::Oracle] {
            let actor = SeatActor {
                seat_id: Ulid::from(1_u128),
                process_generation: 1,
                swarm_id: Ulid::from(2_u128),
                role: Role {
                    name: "fixture".into(),
                    class,
                },
                grant_epoch: 1,
                issuer_kind: crate::messaging::wire::IssuerKind::LocalDev,
            };
            let catalogue = super::super::tools::catalogue(&actor, vec![]);
            let tools = catalogue["tools"].as_array().unwrap();
            for source in tools {
                let call = Call {
                    op_id: Ulid::from(3_u128),
                    tool: source["name"].as_str().unwrap().into(),
                    args: json!({"type":"concern","task_path":"tasks/example.md"}),
                };
                for result in [
                    super::super::success(
                        call.op_id,
                        json!({
                            "items":[{"ref":{"path":"notes/example.md"}}],
                            "skills":[{"uri":"bboxy://skills/example"}],
                            "record":{"path":"notes/example.md"},"frontmatter":{"type":"note"}
                        }),
                    ),
                    super::super::error(call.op_id, "invalid_arguments", "correct the arguments"),
                    super::super::error(call.op_id, "unknown", "outcome unknown"),
                ] {
                    for hint in next(&call, &result) {
                        if hint["method"] == "tools/list" {
                            continue;
                        }
                        let target = tools
                            .iter()
                            .find(|t| t["name"] == hint["params"]["name"])
                            .unwrap_or_else(|| panic!("forbidden hint: {hint}"));
                        let validator = jsonschema::validator_for(&target["inputSchema"]).unwrap();
                        assert!(
                            validator.is_valid(&hint["params"]["arguments"]),
                            "invalid hint: {hint}"
                        );
                    }
                }
            }
        }
    }
}
