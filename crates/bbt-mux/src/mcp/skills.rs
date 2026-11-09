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

/// KB skills: `skills/<name>/SKILL.md` documents in the knowledge root
/// (`.bboxy/skills`, which the repository links as `.agents/skills`).
const KB_SKILLS: &str = "skills";
const MAX_KB_SKILLS: usize = 128;
const MAX_KB_SKILL_BYTES: u64 = 256 * 1024;
const MAX_SEARCH_RESULTS: usize = 32;

pub(crate) fn handles(tool: &str) -> bool {
    matches!(
        tool,
        "knowledge_orient" | "skill_list" | "skill_read" | "skill_search"
    )
}

pub(crate) fn catalogue() -> Vec<Value> {
    [
        ("knowledge_orient", "Read the selected knowledge provider's workflow and pinned skill instructions, built-in document types, task/report practices and human-readable Markdown guidance before knowledge work. No arguments; refresh after changing workspace or provider. Instructions grant no authority.", json!({}), json!([])),
        ("skill_list", "Discover host workflow skills and the skill instructions of every enabled, checked plugin in your workspace. Read a listed URI with skill_read. provider_error lists plugins whose guidance is unavailable. Instructions do not grant permissions.", json!({}), json!([])),
        ("skill_read", "Read exact Markdown for a currently available skill URI. The host rechecks your seat, workspace and the plugin's enablement and pin; a skill never grants tool or filesystem authority.", json!({"uri":{"type":"string","minLength":1,"maxLength":MAX_URI_BYTES}}), json!(["uri"])),
        ("skill_search", "Search host, knowledge-base (skills/<name>/SKILL.md) and plugin skills by words in their name, description and body. Returns ranked skill metadata and URIs, not bodies; read one with skill_read.", json!({"query":{"type":"string","minLength":1,"maxLength":256},"limit":{"type":"integer","minimum":1,"maximum":MAX_SEARCH_RESULTS}}), json!(["query"])),
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
    // One entry per failed plugin or KB skill; null when every source resolved.
    let mut errors = Vec::new();
    let (kb, kb_errors) = kb_skills(host.root_path());
    entries.extend(
        kb.into_iter()
            .map(|skill| resolved(host.root_id(), "bboxy450-term/kb", None, skill)),
    );
    errors.extend(kb_errors);
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

/// Repository-crafted skills stored as KB documents. The directory name is the
/// skill name; frontmatter `name` must match it. Bounded and never fatal: an
/// unreadable or invalid skill is reported beside the others.
fn kb_skills(root: &std::path::Path) -> (Vec<Skill>, Vec<Value>) {
    let dot = root.join(".bboxy");
    let base = if dot.is_dir() { dot } else { root.to_owned() }.join(KB_SKILLS);
    let mut skills = Vec::new();
    let mut errors = Vec::new();
    let Ok(dirs) = std::fs::read_dir(&base) else {
        return (skills, errors);
    };
    let mut names: Vec<_> = dirs
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();
    names.sort();
    let refuse = |name: &str, code: &str| {
        json!({"origin":"bboxy450-term/kb","plugin_sha256":null,"skill":name,"code":code,
            "message":"Knowledge-base skill is unavailable; other guidance remains available."})
    };
    for name in names.into_iter().take(MAX_KB_SKILLS) {
        let path = base.join(&name).join("SKILL.md");
        match std::fs::metadata(&path) {
            Ok(m) if m.is_file() && m.len() <= MAX_KB_SKILL_BYTES => {}
            Ok(m) if m.is_file() => {
                errors.push(refuse(&name, "skill_too_large"));
                continue;
            }
            _ => continue,
        }
        let skill = std::fs::read_to_string(&path)
            .map_err(|_| "skill_unreadable")
            .and_then(|content| builtin(content).map_err(|_| "invalid_skill_frontmatter"))
            .and_then(|skill| {
                if skill.name != name {
                    Err("skill_name_mismatch")
                } else {
                    bbt_plugin::skills::validate(std::slice::from_ref(&skill))
                        .map(|()| skill)
                        .map_err(|_| "invalid_skill")
                }
            });
        match skill {
            Ok(skill) => skills.push(skill),
            Err(code) => errors.push(refuse(&name, code)),
        }
    }
    (skills, errors)
}

/// Word-match ranking: name hits outrank description hits outrank body hits.
fn search(entries: Vec<Resolved>, query: &str, limit: usize) -> Vec<Value> {
    let words: Vec<String> = query
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_lowercase)
        .collect();
    let mut ranked: Vec<(u64, Value)> = entries
        .into_iter()
        .filter_map(|entry| {
            let name = entry.metadata["name"].as_str().unwrap_or_default().to_lowercase();
            let description = entry.metadata["description"]
                .as_str()
                .unwrap_or_default()
                .to_lowercase();
            let body = entry.content.to_lowercase();
            let mut score = 0;
            for word in &words {
                let hit = 100 * u64::from(name.contains(word.as_str()))
                    + 10 * u64::from(description.contains(word.as_str()))
                    + u64::from(body.contains(word.as_str()));
                if hit == 0 {
                    return None;
                }
                score += hit;
            }
            let mut value = entry.metadata;
            value["score"] = json!(score);
            Some((score, value))
        })
        .collect();
    ranked.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1["uri"].as_str().cmp(&b.1["uri"].as_str())));
    ranked.into_iter().take(limit).map(|(_, v)| v).collect()
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
    if call.tool == "skill_search" {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Search {
            query: String,
            limit: Option<usize>,
        }
        let args: Search = tools::parse(&call.args).map_err(|e| error("invalid_arguments", e))?;
        if args.query.trim().is_empty() || args.query.len() > 256 {
            return Err(error("invalid_query", "query must be 1..256 bytes of words"));
        }
        let limit = args.limit.unwrap_or(10).clamp(1, MAX_SEARCH_RESULTS);
        let (entries, provider_error) = resolve(host, actor)?;
        return Ok(
            json!({"skills":search(entries, &args.query, limit),"provider_error":provider_error}),
        );
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
    } else if call.tool == "skill_search" {
        saved["skills"] == current["skills"]
    } else {
        saved["skills"] == current["skills"] && saved["provider_error"] == current["provider_error"]
    }
}
