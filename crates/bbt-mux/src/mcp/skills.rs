// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Canonical instruction bytes through legacy MCP resources and read-only tools.
//! This compatibility surface does not advertise the 2026 Skills extension.
use bbt_plugin::skills::Skill;
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use super::{Call, SeatActor, tools};
use crate::knowledge::{data_source as d, roots::Host};

const KB_SKILL: &str = include_str!("../knowledge/skills/knowledge-base/SKILL.md");
pub(crate) const MAX_URI_BYTES: usize = 512;

pub(crate) fn handles(tool: &str) -> bool {
    matches!(tool, "knowledge_orient" | "skill_list" | "skill_read")
}

pub(crate) fn catalogue() -> Vec<Value> {
    [
        ("knowledge_orient", "Read the selected knowledge provider's workflow and pinned skill instructions, built-in document types, task/report practices and human-readable Markdown guidance before knowledge work. No arguments; refresh after changing workspace or provider. Instructions grant no authority.", json!({}), json!([])),
        ("skill_list", "Discover host workflow skills and the skill instructions of every enabled, checked plugin in your workspace. Read a listed URI with skill_read. provider_error lists plugins whose guidance is unavailable. Instructions do not grant permissions.", json!({}), json!([])),
        ("skill_read", "Read exact Markdown for a currently available skill URI. The host rechecks your seat, workspace and the plugin's enablement and pin; a skill never grants tool or filesystem authority.", json!({"uri":{"type":"string","minLength":1,"maxLength":MAX_URI_BYTES}}), json!(["uri"])),
    ].into_iter().map(|(name, description, properties, required)| json!({
        "name":name,"description":description,
        "inputSchema":{"type":"object","properties":properties,"required":required,"additionalProperties":false},
        "outputSchema":{"type":"object","additionalProperties":true},
        "annotations":{"readOnlyHint":true,"destructiveHint":false,"idempotentHint":true,"openWorldHint":false}
    })).collect()
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Read {
    uri: String,
}

struct Resolved {
    metadata: Value,
    content: String,
}

fn error(code: &str, message: impl Into<String>) -> d::CommandError {
    d::CommandError::new(code, message)
}

fn builtin(content: String) -> d::Result<Skill> {
    // These are our embedded files, not a parser for arbitrary provider YAML.
    let header = content
        .strip_prefix("---\n")
        .and_then(|rest| rest.split_once("\n---\n"))
        .map(|(header, _)| header)
        .ok_or_else(|| error("invalid_builtin_skill", "missing host skill frontmatter"))?;
    let metadata: Value =
        serde_yaml::from_str(header).map_err(|e| error("invalid_builtin_skill", e.to_string()))?;
    let field = |key: &str| {
        metadata[key]
            .as_str()
            .map(str::to_owned)
            .ok_or_else(|| error("invalid_builtin_skill", format!("missing host skill {key}")))
    };
    Ok(Skill {
        name: field("name")?,
        description: field("description")?,
        content,
    })
}

fn resolved(root: &str, origin: &str, pin: Option<&str>, skill: Skill) -> Resolved {
    let digest = format!("{:x}", Sha256::digest(skill.content.as_bytes()));
    // Origin and root are part of identity. A name is never an authority key.
    // Inspired by MCP Skills' server+URI identity:
    // https://github.com/modelcontextprotocol/ext-skills/blob/ff224337b01f0c9b18b81dcf99eefe66e9a52b5c/specification/beta/skills.mdx
    let source = format!(
        "{:x}",
        Sha256::digest(format!("{origin}\0{}", pin.unwrap_or_default()).as_bytes())
    );
    let uri = format!(
        "bboxy://skills/{root}/{source}/{digest}/{}/SKILL.md",
        skill.name
    );
    Resolved {
        metadata: json!({"uri":uri,"name":skill.name,"description":skill.description,
            "origin":origin,"root_id":root,"plugin_sha256":pin,
            "digest":format!("sha256:{digest}"),"size":skill.content.len()}),
        content: skill.content,
    }
}

fn resolve(host: &Host, actor: &SeatActor) -> d::Result<(Vec<Resolved>, Option<Value>)> {
    let workflow = crate::adapters::launch::workflow_skill(&actor.role)
        .map_err(|e| error("skill_unavailable", e.to_string()))?;
    let mut entries = vec![
        resolved(
            host.root_id(),
            "bboxy450-term/workflow",
            None,
            builtin(workflow)?,
        ),
        resolved(
            host.root_id(),
            "bboxy450-term/knowledge",
            None,
            builtin(KB_SKILL.into())?,
        ),
    ];
    // One entry per failed plugin; null when every enabled plugin resolved.
    let mut errors = Vec::new();
    match host.plugin_skills() {
        Ok(plugins) => {
            for plugin in plugins {
                let skills = plugin
                    .skills
                    .and_then(|skills| bbt_plugin::skills::validate(&skills).map(|()| skills));
                match skills {
                    Ok(skills) => entries.extend(skills.into_iter().map(|skill| {
                        resolved(host.root_id(), &plugin.candidate, Some(&plugin.sha256), skill)
                    })),
                    Err(reason) => errors.push(json!({"origin":plugin.candidate,
                        "plugin_sha256":plugin.sha256,"code":reason.code,
                        "message":"Plugin guidance is unavailable; other guidance remains available."})),
                }
            }
        }
        Err(reason) => errors.push(
            json!({"origin":null,"plugin_sha256":null,"code":reason.code,
            "message":"Plugin guidance is unavailable; host guidance remains available."}),
        ),
    }
    let provider_error = (!errors.is_empty()).then_some(Value::Array(errors));
    Ok((entries, provider_error))
}

fn knowledge_orient(host: &Host, actor: &SeatActor) -> d::Result<Value> {
    let (provider, pin) = host.guidance_provider()?;
    let (entries, errors) = resolve(host, actor)?;
    let mut remaining = 48 * 1024;
    let mut provider_loaded = provider == crate::knowledge::roots::BUNDLED;
    let mut skills = Vec::new();
    for entry in entries {
        let origin = entry.metadata["origin"].as_str().unwrap_or_default();
        let selected = origin == provider && entry.metadata["plugin_sha256"] == json!(pin);
        if !origin.starts_with("bboxy450-term/") && !selected { continue; }
        let mut value = entry.metadata;
        value["content"] = json!(entry.content);
        let size = serde_json::to_vec(&value).map_err(|e|error("invalid_skill",e.to_string()))?.len();
        let inline = size <= remaining;
        if inline { remaining -= size; } else { value.as_object_mut().unwrap().remove("content"); }
        value["requires_skill_read"] = json!(!inline);
        provider_loaded |= selected && inline;
        skills.push(value);
    }
    let mut provider_errors: Vec<Value> = errors.and_then(|v|v.as_array().cloned()).unwrap_or_default()
        .into_iter().filter(|e|e["origin"] == provider || e["origin"].is_null()).collect();
    if !provider_loaded && !skills.iter().any(|s|s["origin"] == provider) {
        provider_errors.push(json!({"origin":provider,"code":"selected_provider_guidance_unavailable",
            "message":"Selected provider guidance is unavailable; inspect knowledge_status before provider-specific work."}));
    }
    let types = super::knowledge::type_info(&json!({})).map_err(|e|error("invalid_builtin_guidance",e))?;
    Ok(json!({"provider":{"id":provider,"sha256":pin,"guidance_loaded":provider_loaded},
        "skills":skills,"document_types":types,
        "provider_error":if provider_errors.is_empty(){Value::Null}else{json!(provider_errors)}}))
}

pub(crate) fn read(host: &Host, actor: &SeatActor, call: &Call) -> d::Result<Value> {
    if call.tool == "knowledge_orient" {
        tools::parse::<tools::Empty>(&call.args).map_err(|e|error("invalid_arguments",e))?;
        return knowledge_orient(host, actor);
    }
    let uri = match call.tool.as_str() {
        "skill_list" => {
            tools::parse::<tools::Empty>(&call.args).map_err(|e| error("invalid_arguments", e))?;
            None
        }
        "skill_read" => {
            let args: Read = tools::parse(&call.args).map_err(|e| error("invalid_arguments", e))?;
            if args.uri.is_empty() || args.uri.len() > MAX_URI_BYTES {
                return Err(error("invalid_skill_uri", "skill URI exceeds its bound"));
            }
            Some(args.uri)
        }
        _ => return Err(error("unknown_tool", "not a skill read")),
    };
    let (entries, provider_error) = resolve(host, actor)?;
    if let Some(uri) = uri {
        let Some(entry) = entries
            .into_iter()
            .find(|entry| entry.metadata["uri"] == uri)
        else {
            return Err(error(
                "skill_not_found",
                "Skill URI is not available in this seat's current workspace and enabled plugins.",
            ));
        };
        let mut value = entry.metadata;
        value["content"] = json!(entry.content);
        Ok(json!({"skill":value}))
    } else {
        Ok(
            json!({"skills":entries.into_iter().map(|entry| entry.metadata).collect::<Vec<_>>(),"provider_error":provider_error}),
        )
    }
}

pub(crate) fn retained(host: &Host, actor: &SeatActor, call: &Call, old: &Value) -> bool {
    // Re-resolve even a journal hit: trust, enablement and plugin pins may
    // have changed without changing the request's operation ID.
    let Ok(current) = read(host, actor, call) else {
        return false;
    };
    let saved = &old["structuredContent"];
    if call.tool == "knowledge_orient" {
        ["provider", "skills", "document_types", "provider_error"].iter().all(|key| saved[*key] == current[*key])
    } else if call.tool == "skill_read" {
        saved["skill"] == current["skill"]
    } else {
        saved["skills"] == current["skills"] && saved["provider_error"] == current["provider_error"]
    }
}
