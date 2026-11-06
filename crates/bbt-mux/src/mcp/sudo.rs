// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Sudo: the operator's authority carried into MCP by one seat, across every swarm.
//!
//! A sudo seat is an Oracle in its own swarm. These tools add what only the operator
//! doors had: every swarm at once, spawning anywhere (a new swarm included), messaging
//! any swarm as the operator, the operator inbox, and `sudo_as`, which runs any MCP
//! tool exactly as a named live seat would. Logic resolves sudo from the admitted
//! seat record; the role is assigned only by the operator or another sudo seat.
use std::path::PathBuf;

use serde::Deserialize;
use serde_json::{Value, json};
use ulid::Ulid;

use super::{Call, SeatActor as Actor, tools};
use crate::registry::Class;
pub use bbt_proto::gui::SUDO_ROLE;

const PROFILES: [&str; 6] = ["claude", "codex", "pi", "omp", "bboxy450-harness", "zsh"];

pub(crate) fn is_sudo(actor: &Actor) -> bool {
    actor.role.name == SUDO_ROLE && actor.role.class == Class::Oracle
}
pub(crate) fn handles(name: &str) -> bool {
    name.starts_with("sudo_")
}

pub(crate) fn catalogue(actor: &Actor) -> Vec<Value> {
    if !is_sudo(actor) {
        return vec![];
    }
    let id = json!({"type":"string","pattern":"^[0-7][0-9A-HJKMNP-TV-Z]{25}$"});
    let revision = json!({"type":"string","pattern":"^[1-9][0-9]{0,19}$"});
    let short = json!({"type":"string","minLength":1,"maxLength":128});
    let path = json!({"type":"string","minLength":1,"maxLength":4096});
    let text = json!({"type":"string","minLength":1,"maxLength":16384});
    [
        ("sudo_swarms", "Every swarm on this machine: id, name, working folder and its open seats (id, name, role, harness, process generation). Use the ids with the other sudo_* tools.", json!({}), json!([]), false),
        ("sudo_spawn", "Spawn a seat in any swarm as the operator, in a new tab. Omit swarm_id to create a new swarm (optionally named swarm_name); the result names it. role may be any configured role, oracle and sudo included. cwd defaults to the swarm's working folder. resume continues a native Claude session, Codex thread or bboxy450-harness session.", json!({"swarm_id":id,"swarm_name":short,"approved_profile":{"enum":PROFILES},"cwd":path,"role":short,"name":short,"resume":short}), json!(["approved_profile"]), true),
        ("sudo_swarm", "Rename a swarm or set its working folder, as the operator.", json!({"swarm_id":id,"name":short,"workdir":path}), json!(["swarm_id"]), true),
        ("sudo_message", "Send a message into any swarm as the operator. to is a seat id or name (its inbox) or a destination path such as channels/general; reply_to answers a message. Replies arrive in sudo_inbox.", json!({"swarm_id":id,"to":short,"text":text,"reply_to":id}), json!(["swarm_id","to","text"]), true),
        ("sudo_inbox", "Read the operator inbox: messages addressed to the operator, from every swarm or one swarm, after a ledger sequence. Poll it; it does not wake you.", json!({"swarm_id":id,"since":{"type":"integer","minimum":0},"limit":{"type":"integer","minimum":1,"maximum":128}}), json!([]), false),
        ("sudo_as", "Run any MCP tool exactly as another live seat: its identity, role, swarm and grants. Use it to act as a swarm's oracle (task_delegate, seat_spawn, message, knowledge_write) without joining that swarm. tool and arguments follow that tool's own schema as the seat sees it; sudo_* tools cannot be nested.", json!({"seat_id":id,"expected_process_generation":revision,"tool":{"type":"string","minLength":1,"maxLength":64},"arguments":{"type":"object"}}), json!(["seat_id","expected_process_generation","tool","arguments"]), true),
    ]
    .into_iter()
    .map(|(name, description, properties, required, mutation)| json!({
        "name":name,"description":description,
        "inputSchema":{"type":"object","properties":properties,"required":required,"additionalProperties":false},
        "outputSchema":{"type":"object","additionalProperties":true},
        "annotations":{"readOnlyHint":!mutation,"destructiveHint":mutation,"idempotentHint":true,"openWorldHint":false}
    }))
    .collect()
}

#[derive(Debug)]
pub(crate) enum Request {
    Swarms,
    Spawn {
        swarm: Option<Ulid>,
        swarm_name: Option<String>,
        profile: String,
        cwd: Option<PathBuf>,
        role: Option<String>,
        name: Option<String>,
        resume: Option<String>,
    },
    Swarm {
        swarm: Ulid,
        name: Option<String>,
        workdir: Option<String>,
    },
    Message {
        swarm: Ulid,
        to: String,
        text: String,
        reply_to: Option<Ulid>,
    },
    Inbox {
        swarm: Option<Ulid>,
        since: u64,
        limit: usize,
    },
    As {
        seat: Ulid,
        generation: u64,
        call: Call,
    },
}

fn short(value: &Option<String>, what: &str) -> Result<(), String> {
    match value {
        Some(s) if s.trim().is_empty() || s.len() > 128 || s.chars().any(char::is_control) => {
            Err(format!("invalid {what}"))
        }
        _ => Ok(()),
    }
}
fn swarm_id(id: Ulid) -> Result<Ulid, String> {
    if id == Ulid::nil() {
        return Err("swarm_id must be nonnil".into());
    }
    Ok(id)
}

/// Authority is the caller's admitted record, never tool input.
pub(crate) fn request(actor: &Actor, call: &Call) -> Result<Request, String> {
    if !is_sudo(actor) {
        return Err("tool requires the sudo role".into());
    }
    Ok(match call.tool.as_str() {
        "sudo_swarms" => {
            tools::parse::<tools::Empty>(&call.args)?;
            Request::Swarms
        }
        "sudo_spawn" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct Args {
                swarm_id: Option<Ulid>,
                swarm_name: Option<String>,
                approved_profile: String,
                cwd: Option<PathBuf>,
                role: Option<String>,
                name: Option<String>,
                resume: Option<String>,
            }
            let a: Args = tools::parse(&call.args)?;
            if !PROFILES.contains(&a.approved_profile.as_str()) {
                return Err("unapproved spawn profile".into());
            }
            if a.swarm_id.is_some() && a.swarm_name.is_some() {
                return Err("swarm_name names a new swarm; omit swarm_id or use sudo_swarm".into());
            }
            if a.cwd.as_ref().is_some_and(|cwd| !cwd.is_absolute()) {
                return Err("cwd must be an explicit absolute directory".into());
            }
            short(&a.swarm_name, "swarm name")?;
            short(&a.role, "role name")?;
            short(&a.name, "seat name")?;
            short(&a.resume, "resume id")?;
            Request::Spawn {
                swarm: a.swarm_id.map(swarm_id).transpose()?,
                swarm_name: a.swarm_name,
                profile: a.approved_profile,
                cwd: a.cwd,
                role: a.role,
                name: a.name,
                resume: a.resume,
            }
        }
        "sudo_swarm" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct Args {
                swarm_id: Ulid,
                name: Option<String>,
                workdir: Option<String>,
            }
            let a: Args = tools::parse(&call.args)?;
            if a.name.is_none() && a.workdir.is_none() {
                return Err("supply name, workdir or both".into());
            }
            short(&a.name, "swarm name")?;
            if a.workdir
                .as_ref()
                .is_some_and(|p| !PathBuf::from(p).is_absolute())
            {
                return Err("workdir must be an absolute directory".into());
            }
            Request::Swarm {
                swarm: swarm_id(a.swarm_id)?,
                name: a.name,
                workdir: a.workdir,
            }
        }
        "sudo_message" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct Args {
                swarm_id: Ulid,
                to: String,
                text: String,
                reply_to: Option<Ulid>,
            }
            let a: Args = tools::parse(&call.args)?;
            if a.to.trim().is_empty() || a.to.len() > 256 || a.to.chars().any(char::is_control) {
                return Err("invalid destination".into());
            }
            if a.text.is_empty() || a.text.len() > 16384 {
                return Err("text must be 1..16384 bytes".into());
            }
            Request::Message {
                swarm: swarm_id(a.swarm_id)?,
                to: a.to,
                text: a.text,
                reply_to: a.reply_to,
            }
        }
        "sudo_inbox" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct Args {
                swarm_id: Option<Ulid>,
                since: Option<u64>,
                limit: Option<usize>,
            }
            let a: Args = tools::parse(&call.args)?;
            Request::Inbox {
                swarm: a.swarm_id.map(swarm_id).transpose()?,
                since: a.since.unwrap_or(0),
                limit: tools::page_limit(a.limit)?,
            }
        }
        "sudo_as" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct Args {
                seat_id: Ulid,
                expected_process_generation: String,
                tool: String,
                arguments: Value,
            }
            let a: Args = tools::parse(&call.args)?;
            let generation = a
                .expected_process_generation
                .parse::<u64>()
                .ok()
                .filter(|g| *g > 0 && !a.expected_process_generation.starts_with('0'))
                .ok_or("expected_process_generation must be a positive decimal string")?;
            if a.seat_id == Ulid::nil() {
                return Err("seat_id must be nonnil".into());
            }
            // One level only: sudo is the caller's, never the target's.
            if handles(&a.tool) {
                return Err("sudo_* tools cannot run through sudo_as".into());
            }
            let call = Call {
                op_id: call.op_id,
                tool: a.tool,
                args: a.arguments,
            };
            call.validate().map_err(|e| e.to_string())?;
            Request::As {
                seat: a.seat_id,
                generation,
                call,
            }
        }
        _ => return Err("unknown sudo tool".into()),
    })
}

/// A seat may hand out the sudo role only when it already holds it.
pub(crate) fn may_assign(actor: &Actor, role: Option<&str>) -> bool {
    role != Some(SUDO_ROLE) || is_sudo(actor)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messaging::wire::IssuerKind;
    use crate::registry::Role;

    fn actor(name: &str, class: Class) -> Actor {
        Actor {
            seat_id: Ulid::new(),
            process_generation: 1,
            swarm_id: Ulid::new(),
            role: Role {
                name: name.into(),
                class,
            },
            grant_epoch: 1,
            issuer_kind: IssuerKind::LocalDev,
        }
    }
    fn call(tool: &str, args: Value) -> Call {
        Call {
            op_id: Ulid::new(),
            tool: tool.into(),
            args,
        }
    }

    #[test]
    fn only_a_sudo_oracle_sees_or_runs_sudo_tools() {
        let sudo = actor(SUDO_ROLE, Class::Oracle);
        let names: Vec<_> = catalogue(&sudo)
            .iter()
            .map(|t| t["name"].as_str().unwrap().to_owned())
            .collect();
        assert_eq!(
            names,
            [
                "sudo_swarms",
                "sudo_spawn",
                "sudo_swarm",
                "sudo_message",
                "sudo_inbox",
                "sudo_as"
            ]
        );
        assert!(names.iter().all(|n| handles(n)));
        for other in [
            actor("oracle", Class::Oracle),
            actor("solo", Class::Oracle),
            // A name alone is not authority; the class must match too.
            actor(SUDO_ROLE, Class::Worker),
        ] {
            assert!(catalogue(&other).is_empty());
            assert_eq!(
                request(&other, &call("sudo_swarms", json!({}))).unwrap_err(),
                "tool requires the sudo role"
            );
        }
        assert!(matches!(
            request(&sudo, &call("sudo_swarms", json!({}))),
            Ok(Request::Swarms)
        ));
    }

    #[test]
    fn spawn_without_a_swarm_creates_one_and_validates_its_inputs() {
        let sudo = actor(SUDO_ROLE, Class::Oracle);
        let parsed = request(
            &sudo,
            &call("sudo_spawn", json!({"approved_profile":"claude","role":"oracle","swarm_name":"harmony","cwd":"/tmp"})),
        )
        .unwrap();
        assert!(
            matches!(parsed, Request::Spawn { swarm: None, ref role, ref swarm_name, .. }
            if role.as_deref() == Some("oracle") && swarm_name.as_deref() == Some("harmony"))
        );
        for bad in [
            json!({"approved_profile":"bash"}),
            json!({"approved_profile":"claude","cwd":"relative"}),
            json!({"approved_profile":"claude","swarm_id":Ulid::new(),"swarm_name":"x"}),
            json!({"approved_profile":"claude","role":" "}),
            json!({"approved_profile":"claude","command":"rm"}),
        ] {
            assert!(request(&sudo, &call("sudo_spawn", bad)).is_err());
        }
    }

    #[test]
    fn sudo_as_carries_the_call_but_never_nests_sudo() {
        let sudo = actor(SUDO_ROLE, Class::Oracle);
        let target = Ulid::new();
        let outer = call(
            "sudo_as",
            json!({"seat_id":target,"expected_process_generation":"3","tool":"message_send","arguments":{"to":"x"}}),
        );
        let Request::As {
            seat,
            generation,
            call: inner,
        } = request(&sudo, &outer).unwrap()
        else {
            panic!("expected sudo_as");
        };
        assert_eq!((seat, generation), (target, 3));
        assert_eq!(
            (inner.op_id, inner.tool.as_str()),
            (outer.op_id, "message_send")
        );
        assert_eq!(inner.args, json!({"to":"x"}));
        for (tool, generation) in [
            ("sudo_spawn", "3"),
            ("message_send", "0"),
            ("message_send", "03"),
        ] {
            assert!(
                request(&sudo, &call("sudo_as", json!({"seat_id":target,"expected_process_generation":generation,"tool":tool,"arguments":{}}))).is_err()
            );
        }
    }

    #[test]
    fn only_sudo_may_assign_sudo() {
        assert!(may_assign(
            &actor("oracle", Class::Oracle),
            Some("executor")
        ));
        assert!(may_assign(&actor("oracle", Class::Oracle), None));
        assert!(!may_assign(
            &actor("oracle", Class::Oracle),
            Some(SUDO_ROLE)
        ));
        assert!(may_assign(
            &actor(SUDO_ROLE, Class::Oracle),
            Some(SUDO_ROLE)
        ));
    }
}
