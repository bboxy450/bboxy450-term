// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! MCP hint graph: actionable next reads, projected without executing or altering receipts.
//! Keep advice in structuredContent AND its text mirror. Codex may prefer the
//! structured result exclusively; _meta and an extra text block are insufficient.
//! https://developers.openai.com/plugins/build/mcp-server#return-useful-results-without-ui
//! https://github.com/openai/codex/blob/bbe8cde1c362b090cd1c683db2d9139047d8aede/codex-rs/protocol/src/models.rs#L2048
//! Advice is not authorization or an automatic tool invocation. Attach it only
//! at response boundaries, after durable journal/signature work has completed.
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
    if matches!(
        call.tool.as_str(),
        "python_execute"
            | "python_execute_orient"
            | "python_execution_get"
            | "python_execution_cancel"
    ) {
        // Runtime faults carry precise recovery data; preserve their guidance
        // instead of replacing it with the native journal's unrelated op ID.
        if let Some(hints) = data["hints"].as_array().filter(|hints| !hints.is_empty()) {
            return hints.clone();
        }
        if let Some(execution) = data["execution_id"].as_str()
            && (matches!(
                data["status"].as_str(),
                Some("starting" | "running" | "cancelling")
            ) || data["next_cursor"].is_string())
        {
            let mut args = json!({"execution_id":execution});
            if data["next_cursor"].is_string() {
                args["cursor"] = data["next_cursor"].clone();
            }
            return vec![tool(
                "python_execution_get",
                args,
                "Read the retained execution/output cursor; do not rerun the script or replay earlier effects.",
            )];
        }
        return vec![];
    }
    if data["status"] == "unknown"
        || matches!(
            data["error"]["code"].as_str(),
            Some("unknown" | "op_id_reused" | "commit_not_confirmed" | "write_not_confirmed")
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
        if matches!(code, "invalid_report" | "report_author_mismatch") {
            return vec![tool(
                "knowledge_types",
                json!({"type":"report"}),
                "Call knowledge_types for report and correct all listed fields in one new request: task, author (your authenticated seat:ULID), created, commits and verify. No report was published by this rejection.",
            )];
        }
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
        // tools/list is a client protocol method, not a tool the model can call.
        return vec![tool(
            "orient",
            json!({}),
            "Refresh your current seat and workflow tools, then correct the arguments using the tool's input schema.",
        )];
    }
    let hint = match call.tool.as_str() {
        "workspace_get" if data["workspace"]["state"] == "unknown" => {
            let mut hints = vec![tool(
                "workspace_orient",
                json!({}),
                "Reconcile the original recorded resource; do not prepare a replacement or retry an unknown release.",
            )];
            if let Some(op) = data["workspace"]["operation_id"]
                .as_str()
                .and_then(|id| id.parse().ok())
            {
                hints.extend(recovery(op));
            }
            return hints;
        }
        "workspace_prepare" | "workspace_release" => {
            return data["workspace"]["workspace_id"].as_str().map(|id|vec![tool("workspace_get",json!({"workspace_id":id}),"Inspect the durable workspace state. Preparing/releasing is not readiness; never replace an unknown operation.")]).unwrap_or_default();
        }
        "orient" => {
            let mut hints = vec![
                tool(
                    "knowledge_orient",
                    json!({}),
                    "Load the selected provider's skill, document-writing workflow and task guidance before knowledge work.",
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
            // Tool availability is the authority projection. Custom Oracle
            // display names and future grants must not change this edge.
            if data["tools"].as_array().is_some_and(|tools| {
                tools
                    .iter()
                    .any(|entry| entry["name"] == "workspace_prepare")
            }) {
                hints.push(tool("workspace_orient",json!({}),"Load the configured providers and managed worker provisioning/launch/delegation workflow before creating a worker workspace."));
            }
            if data["tools"].as_array().is_some_and(|tools| {
                tools
                    .iter()
                    .any(|tool| tool["name"] == "python_execute_orient")
            }) {
                hints.push(tool("python_execute_orient",json!({}),"Read runtime readiness, active workspace and KB binding, all configured MCP discovery, exact-argv/jq examples and recovery before running Python."));
            }
            return hints;
        }
        "knowledge_orient" => {
            if let Some(uri) = data["skills"]
                .as_array()
                .and_then(|skills| skills.iter().find(|s| s["requires_skill_read"] == true))
                .and_then(|s| s["uri"].as_str())
            {
                tool(
                    "skill_read",
                    json!({"uri":uri}),
                    "Load the remaining provider guidance before writing.",
                )
            } else if !data["provider_error"].is_null() {
                tool(
                    "knowledge_status",
                    json!({}),
                    "Inspect why the selected provider's guidance is unavailable.",
                )
            } else {
                tool(
                    "knowledge_search",
                    json!({"query":"type:spec"}),
                    "Read existing intent and linked tasks before writing or creating duplicates.",
                )
            }
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
                "Read back this draft. Use knowledge_commit deliberately for a substantial checkpoint, before cross-machine sharing, or after final acceptance; do not commit each edit.",
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

/// Optional host-envelope field; plugin result schemas remain nested/unchanged.
pub(crate) fn schema() -> Value {
    json!({"type":"array","description":"Advisory next reads; params.name identifies an exposed tool, not an automatic action or authorization grant.",
        "items":{"type":"object","properties":{"reason":{"type":"string"},"method":{"type":"string"},"params":{"type":"object"}},"required":["reason","method","params"],"additionalProperties":true}})
}

/// Project advice onto a wire copy, never into canonical journaled receipts.
/// Idempotent: the server socket and a newer bridge can both apply this. Socket
/// projection also reaches old, still-running bridges after a logic refresh.
pub(crate) fn attach(call: &Call, result: Value) -> Value {
    let mut hints = next(call, &result);
    for hint in result["structuredContent"]["hints"]
        .as_array()
        .into_iter()
        .flatten()
    {
        if hint["kind"] == "unread_messages" && !hints.contains(hint) {
            hints.insert(0, hint.clone());
        }
    }
    project(result, hints)
}

pub(crate) fn with_message_hint(call: &Call, result: Value, hint: Value) -> Value {
    let mut hints = next(call, &result);
    hints.retain(|h| h["kind"] != "unread_messages");
    hints.insert(0, hint);
    project(result, hints)
}

/// Refresh a host JSON text mirror after adding receipt-only metadata.
pub(crate) fn remirror(mut result: Value) -> Value {
    let data = result["structuredContent"].clone();
    if !data["op_id"].is_null() {
        for block in result["content"].as_array_mut().into_iter().flatten() {
            if block["text"]
                .as_str()
                .and_then(|s| serde_json::from_str::<Value>(s).ok())
                .is_some_and(|old| old["op_id"] == data["op_id"])
            {
                block["text"] = data.to_string().into();
            }
        }
    }
    result
}

fn project(result: Value, hints: Vec<Value>) -> Value {
    let mut projected = result.clone();
    if let Some(data) = projected["structuredContent"].as_object_mut() {
        data.insert("hints".into(), json!(hints));
        let text = projected["structuredContent"].to_string();
        let mirror = result["content"].as_array().and_then(|content| {
            content.iter().position(|block| {
                block["text"]
                    .as_str()
                    .and_then(|text| serde_json::from_str::<Value>(text).ok())
                    .as_ref()
                    == Some(&result["structuredContent"])
            })
        });
        if let Some(index) = mirror {
            projected["content"][index]["text"] = text.into();
        } else if let Some(content) = projected["content"].as_array_mut() {
            content.push(json!({"type":"text","text":text}));
        }
    } else if let Some(content) = projected["content"].as_array_mut() {
        let block = json!({"type":"text","text":json!({"hints":hints}).to_string()});
        if !content.contains(&block) {
            content.push(block);
        }
    }
    projected["_meta"]["bboxy450/hints"] = json!(hints);
    // Account for all copies and leave room for the enclosing JSON-RPC frame.
    if serde_json::to_vec(&projected)
        .is_ok_and(|bytes| bytes.len().saturating_add(2048) < super::bridge::MAX_FRAME)
    {
        projected
    } else {
        result
    }
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
        let mut core = hinted["structuredContent"].clone();
        core.as_object_mut().unwrap().remove("hints");
        assert_eq!(
            core, receipt["structuredContent"],
            "canonical receipt changed"
        );
        let hints = hinted["_meta"]["bboxy450/hints"].as_array().unwrap();
        assert_eq!(hints.len(), 1);
        assert_eq!(hints[0]["params"]["name"], "operation_status");
        assert_eq!(hints[0]["params"]["arguments"]["original_op_id"], json!(op));
        assert_eq!(
            serde_json::from_str::<Value>(hinted["content"][0]["text"].as_str().unwrap()).unwrap()
                ["hints"],
            json!(hints)
        );
        // Codex's structured-only path and a legacy text client both see advice.
        assert_eq!(hinted["structuredContent"]["hints"], json!(hints));
        assert_eq!(
            attach(&call, hinted.clone()),
            hinted,
            "double projection changed the reply"
        );
        assert_eq!(hinted["isError"], true);
    }

    #[test]
    fn report_feedback_is_actionable_in_structured_and_text_clients() {
        let call = Call {
            op_id: Ulid::from(9_u128),
            tool: "knowledge_write".into(),
            args: json!({"frontmatter":{"type":"report"}}),
        };
        let error =
            bbt_doc::reports::validate("reports/2026-11-05-work.md", &call.args["frontmatter"])
                .unwrap_err();
        let reply = attach(
            &call,
            super::super::error(call.op_id, "invalid_report", &error),
        );
        let structured = &reply["structuredContent"];
        for field in ["task:", "author:", "created:", "commits:", "verify:"] {
            assert!(
                structured["error"]["message"]
                    .as_str()
                    .unwrap()
                    .contains(field)
            );
        }
        assert_eq!(structured["hints"][0]["params"]["name"], "knowledge_types");
        assert_eq!(
            structured["hints"][0]["params"]["arguments"],
            json!({"type":"report"})
        );
        let text: Value =
            serde_json::from_str(reply["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(&text, structured);
    }

    #[test]
    fn projection_preserves_non_json_content_and_is_idempotent() {
        let call = Call {
            op_id: Ulid::from(9_u128),
            tool: "knowledge_status".into(),
            args: json!({}),
        };
        let result = json!({"structuredContent":{"result":{"signature":"opaque signed bytes"}}, "content":[{"type":"text","text":"Provider narrative"}]});
        let projected = attach(&call, result.clone());
        assert_eq!(projected["content"][0], result["content"][0]);
        assert_eq!(
            projected["structuredContent"]["result"],
            result["structuredContent"]["result"]
        );
        assert_eq!(attach(&call, projected.clone()), projected);
        let text_only = attach(
            &call,
            json!({"content":[{"type":"text","text":"Narrative"}]}),
        );
        assert_eq!(attach(&call, text_only.clone()), text_only);
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
