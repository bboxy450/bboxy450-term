// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Seat controls are explicit, generation-bound and independent of GUI focus.
//! Origin/credit: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use ulid::Ulid;

use super::{Call, SeatActor as Actor, tools};
use crate::{
    adapters::{Evidence, WakeCause, WakeOutcome},
    registry::{Class, SeatRecord},
};

const APPROVED_PROFILES: [&str; 5] = ["zsh", "claude", "codex", "pi", "omp"];

pub(crate) fn handles(name: &str) -> bool {
    mutation(name) || matches!(name, "screen_read" | "history_read" | "layout_read" | "ctl")
}
pub(crate) fn mutation(name: &str) -> bool {
    matches!(
        name,
        "seat_input_text"
            | "seat_spawn"
            | "seat_split"
            | "seat_focus"
            | "seat_mute"
            | "seat_unmute"
            | "seat_interrupt"
    )
}
pub(crate) fn catalogue(actor: &Actor) -> Vec<Value> {
    let id = json!({"type":"string","pattern":"^[0-7][0-9A-HJKMNP-TV-Z]{25}$"});
    let revision = json!({"type":"string","pattern":"^[1-9][0-9]{0,19}$"});
    let text = json!({"type":"string","minLength":1,"maxLength":16384});
    let mut defs = vec![
        (
            "screen_read",
            "Read bounded viewport text from this process generation. Workers can read only their own seat.",
            json!({"view_cursor":{"type":"string","maxLength":512},"limit_rows":{"type":"integer","minimum":1,"maximum":128}}),
            vec![],
        ),
        (
            "history_read",
            "Page captured records, not semantic terminal rows. Output payloads are base64; gaps and capture cursors are explicit. Workers read only their own seat.",
            json!({"cursor":{"type":"string","maxLength":512},"limit_records":{"type":"integer","minimum":1,"maximum":128}}),
            vec![],
        ),
    ];
    if actor.role.class == Class::Oracle {
        defs.extend([
            ("seat_input_text","Paste literal text only through the live positive-readiness guard. submit explicitly chooses Enter; rejected input is not queued.",json!({"text":text,"submit":{"type":"boolean"}}),vec!["text","submit"]),
            ("seat_spawn","Launch an approved builtin in your swarm through the per-seat wiring kit. Defaults to a new tab; optional tab_id names that new tab. No arbitrary commands, environment, privilege class or resume.",json!({"swarm_id":id,"approved_profile":{"enum":APPROVED_PROFILES},"cwd":{"type":"string","minLength":1,"maxLength":4096},"role":{"type":"string","minLength":1,"maxLength":128},"name":{"type":"string","minLength":1,"maxLength":128},"tab_id":id}),vec!["swarm_id","approved_profile","cwd"]),
            ("seat_split","Launch an approved builtin beside an exact live seat in your swarm, even while the UI is detached. side is right or bottom (equal halves). tab_id optionally asserts the anchor tab.",json!({"swarm_id":id,"approved_profile":{"enum":APPROVED_PROFILES},"cwd":{"type":"string","minLength":1,"maxLength":4096},"role":{"type":"string","minLength":1,"maxLength":128},"name":{"type":"string","minLength":1,"maxLength":128},"tab_id":id,"side":{"enum":["right","bottom"]}}),vec!["swarm_id","approved_profile","cwd","side"]),
            ("seat_focus","Focus a seat in your swarm, acknowledging a completed frame. Unavailable while the UI is detached.",json!({}),vec![]),
            ("layout_read","Read your swarm's saved tab membership and durable seat placements, with exact process generations for orchestration. Saved membership can lag the attached UI.",json!({"after":id,"limit":{"type":"integer","minimum":1,"maximum":128}}),vec![]),
            ("seat_mute","Set current target mute policy once under the expected policy epoch and process generation.",json!({"expected_policy_epoch":revision}),vec!["expected_policy_epoch"]),
            ("seat_unmute","Clear mute under the current policy epoch; existing eligible held traffic resumes without replaying unknown attempts.",json!({"expected_policy_epoch":revision}),vec!["expected_policy_epoch"]),
            ("seat_interrupt","Send text through a verified native Now/steer endpoint. Unavailable harnesses are named; this never sends a PTY control key.",json!({"text":text}),vec!["text"]),
        ]);
    }
    defs.into_iter().map(|(name,description,mut properties,mut required)| {
        if !matches!(name,"seat_spawn" | "layout_read") {
            properties["seat_id"]=id.clone();properties["expected_process_generation"]=revision.clone();
            required.extend(["seat_id","expected_process_generation"]);
        }
        if mutation(name) { properties["op_id"]=id.clone();required.insert(0,"op_id"); }
        json!({"name":name,"description":description,
            "inputSchema":{"type":"object","properties":properties,"required":required,"additionalProperties":false},
            "outputSchema":{"type":"object","additionalProperties":true},
            "annotations":{"readOnlyHint":!mutation(name),"destructiveHint":mutation(name),"idempotentHint":true,"openWorldHint":false}})
    }).chain((actor.role.class == Class::Oracle).then(|| json!({
        "name":"ctl","description":"Oracle seat control through MCP, under your authenticated seat identity and swarm grants. command is a tool name; arguments use that tool's schema. This never executes a shell or grants device-operator authority.",
        "inputSchema":{"type":"object","properties":{"command":{"enum":CTL_COMMANDS},"arguments":{"type":"object"}},"required":["command","arguments"],"additionalProperties":false},
        "annotations":{"readOnlyHint":false,"destructiveHint":true,"idempotentHint":true,"openWorldHint":false}
    }))).collect()
}
const CTL_COMMANDS: &[&str] = &[
    "orient",
    "self_status",
    "seat_list",
    "seat_get",
    "layout_read",
    "screen_read",
    "history_read",
    "seat_spawn",
    "seat_split",
    "seat_focus",
    "seat_input_text",
    "seat_interrupt",
    "seat_mute",
    "seat_unmute",
];
/// A typed alias, never a process launch or an operator-principal conversion.
pub(crate) fn canonical(actor: &Actor, call: &Call) -> Result<Call, String> {
    if call.tool != "ctl" {
        return Ok(call.clone());
    }
    if actor.role.class != Class::Oracle {
        return Err("tool requires oracle-class authority".into());
    }
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Control {
        command: String,
        arguments: Value,
    }
    let control: Control = tools::parse(&call.args)?;
    if !CTL_COMMANDS.contains(&control.command.as_str()) || !control.arguments.is_object() {
        return Err("unsupported ctl command or arguments".into());
    }
    Ok(Call {
        op_id: call.op_id,
        tool: control.command,
        args: control.arguments,
    })
}

#[derive(Deserialize)]
#[serde(try_from = "String")]
struct Revision(u64);
impl TryFrom<String> for Revision {
    type Error = &'static str;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.is_empty()
            || value.len() > 20
            || value.starts_with('0')
            || !value.bytes().all(|b| b.is_ascii_digit())
        {
            return Err("expected a positive decimal-string revision");
        }
        value.parse().map(Self).map_err(|_| "revision exceeds u64")
    }
}
#[derive(Clone, Copy)]
pub(crate) struct Target {
    pub seat: Ulid,
    pub generation: u64,
}
impl Target {
    fn new(seat: Ulid, generation: Revision) -> Result<Self, String> {
        if seat == Ulid::nil() {
            return Err("target seat must be nonnil".into());
        }
        Ok(Self {
            seat,
            generation: generation.0,
        })
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Screen {
    seat_id: Ulid,
    expected_process_generation: Revision,
    view_cursor: Option<String>,
    limit_rows: Option<usize>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct History {
    seat_id: Ulid,
    expected_process_generation: Revision,
    cursor: Option<String>,
    limit_records: Option<usize>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    seat_id: Ulid,
    expected_process_generation: Revision,
    text: String,
    submit: bool,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Spawn {
    pub swarm_id: Ulid,
    pub approved_profile: String,
    pub cwd: PathBuf,
    pub role: Option<String>,
    pub name: Option<String>,
    pub tab_id: Option<Ulid>,
    seat_id: Option<Ulid>,
    expected_process_generation: Option<Revision>,
    pub side: Option<bbt_proto::seat_state::Side>,
}
impl Spawn {
    pub fn target(&self) -> Option<Target> {
        self.seat_id
            .zip(self.expected_process_generation.as_ref())
            .map(|(seat, generation)| Target {
                seat,
                generation: generation.0,
            })
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Focus {
    seat_id: Ulid,
    expected_process_generation: Revision,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Mute {
    seat_id: Ulid,
    expected_process_generation: Revision,
    expected_policy_epoch: Revision,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Interrupt {
    seat_id: Ulid,
    expected_process_generation: Revision,
    text: String,
}

pub(crate) enum Request {
    Layout {
        swarm: Ulid,
        after: Option<Ulid>,
        limit: usize,
    },
    Focus {
        target: Target,
    },
    Screen {
        target: Target,
        cursor: Option<String>,
        limit: usize,
    },
    History {
        target: Target,
        cursor: Option<String>,
        limit: usize,
    },
    Input {
        target: Target,
        text: String,
        submit: bool,
    },
    Spawn(Spawn),
    Mute {
        target: Target,
        epoch: u64,
        muted: bool,
    },
    Interrupt {
        target: Target,
        text: String,
    },
}
impl Request {
    pub fn target(&self) -> Option<Target> {
        match self {
            Self::Focus { target }
            | Self::Screen { target, .. }
            | Self::History { target, .. }
            | Self::Input { target, .. }
            | Self::Mute { target, .. }
            | Self::Interrupt { target, .. } => Some(*target),
            Self::Spawn(args) => args.target(),
            Self::Layout { .. } => None,
        }
    }
    pub fn is_read(&self) -> bool {
        matches!(
            self,
            Self::Layout { .. } | Self::Screen { .. } | Self::History { .. }
        )
    }
}
pub(crate) fn request(actor: &Actor, call: &Call) -> Result<Request, String> {
    if (mutation(&call.tool) || matches!(call.tool.as_str(), "layout_read" | "ctl"))
        && actor.role.class != Class::Oracle
    {
        return Err("tool requires oracle-class authority".into());
    }
    Ok(match call.tool.as_str() {
        "screen_read" => {
            let a: Screen = tools::parse(&call.args)?;
            cursor_bound(&a.view_cursor)?;
            Request::Screen {
                target: Target::new(a.seat_id, a.expected_process_generation)?,
                cursor: a.view_cursor,
                limit: tools::page_limit(a.limit_rows)?,
            }
        }
        "history_read" => {
            let a: History = tools::parse(&call.args)?;
            cursor_bound(&a.cursor)?;
            Request::History {
                target: Target::new(a.seat_id, a.expected_process_generation)?,
                cursor: a.cursor,
                limit: tools::page_limit(a.limit_records)?,
            }
        }
        "seat_input_text" => {
            let a: Input = tools::parse(&call.args)?;
            text(&a.text)?;
            Request::Input {
                target: Target::new(a.seat_id, a.expected_process_generation)?,
                text: a.text,
                submit: a.submit,
            }
        }
        "layout_read" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct Layout {
                after: Option<Ulid>,
                limit: Option<usize>,
            }
            let a: Layout = tools::parse(&call.args)?;
            Request::Layout {
                swarm: actor.swarm_id,
                after: a.after,
                limit: tools::page_limit(a.limit)?,
            }
        }
        "seat_focus" => {
            let a: Focus = tools::parse(&call.args)?;
            Request::Focus {
                target: Target::new(a.seat_id, a.expected_process_generation)?,
            }
        }
        "seat_spawn" | "seat_split" => {
            let a: Spawn = tools::parse(&call.args)?;
            if a.swarm_id != actor.swarm_id || a.swarm_id == Ulid::nil() {
                return Err("spawn outside caller swarm".into());
            }
            if !APPROVED_PROFILES.contains(&a.approved_profile.as_str()) {
                return Err("unapproved spawn profile".into());
            }
            if !a.cwd.is_absolute() || a.cwd.as_os_str().len() > 4096 {
                return Err("cwd must be an explicit absolute directory".into());
            }
            if a.role.as_ref().is_some_and(|s| {
                s.trim().is_empty() || s.len() > 128 || s.chars().any(char::is_control)
            }) {
                return Err("invalid role name".into());
            }
            if a.name.as_ref().is_some_and(|name| {
                name.trim().is_empty() || name.len() > 128 || name.chars().any(char::is_control)
            }) {
                return Err("invalid seat name".into());
            }
            if a.tab_id == Some(Ulid::nil()) {
                return Err("tab_id must be nonnil".into());
            }
            if call.tool == "seat_split" {
                if a.target().is_none() || a.seat_id == Some(Ulid::nil()) || a.side.is_none() {
                    return Err(
                        "split requires seat_id, expected_process_generation and side".into(),
                    );
                }
            } else if a.seat_id.is_some()
                || a.expected_process_generation.is_some()
                || a.side.is_some()
            {
                return Err("use seat_split for anchor placement".into());
            }
            Request::Spawn(a)
        }
        "seat_mute" | "seat_unmute" => {
            let a: Mute = tools::parse(&call.args)?;
            Request::Mute {
                target: Target::new(a.seat_id, a.expected_process_generation)?,
                epoch: a.expected_policy_epoch.0,
                muted: call.tool == "seat_mute",
            }
        }
        "seat_interrupt" => {
            let a: Interrupt = tools::parse(&call.args)?;
            text(&a.text)?;
            Request::Interrupt {
                target: Target::new(a.seat_id, a.expected_process_generation)?,
                text: a.text,
            }
        }
        "ctl" => {
            return Err(canonical(actor, call)
                .err()
                .unwrap_or_else(|| "invalid ctl dispatch".into()));
        }
        _ => return Err("unknown seat tool".into()),
    })
}
fn cursor_bound(cursor: &Option<String>) -> Result<(), String> {
    if cursor
        .as_ref()
        .is_some_and(|s| s.is_empty() || s.len() > 512)
    {
        Err("invalid cursor bound".into())
    } else {
        Ok(())
    }
}
fn text(value: &str) -> Result<(), String> {
    if value.is_empty()
        || value.len() > 16384
        || value
            .chars()
            .any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t'))
    {
        Err("invalid literal text or terminal control bytes".into())
    } else {
        Ok(())
    }
}
pub(crate) fn authorize(
    actor: &Actor,
    caller: &SeatRecord,
    request: &Request,
    target: Option<&SeatRecord>,
) -> Result<(), String> {
    if matches!(
        request,
        Request::Input { .. } | Request::Spawn(_) | Request::Interrupt { .. }
    ) && caller.muted
    {
        return Err("muted caller cannot input, interrupt or spawn".into());
    }
    if let Some(expected) = request.target() {
        let record = target.ok_or("target unavailable")?;
        if record.id != expected.seat || record.generation != expected.generation {
            return Err("stale target process generation".into());
        }
        if record.swarm != actor.swarm_id {
            return Err("target outside caller swarm".into());
        }
        if actor.role.class != Class::Oracle && (!request.is_read() || record.id != actor.seat_id) {
            return Err("workers read only their own seat".into());
        }
        if record.muted && matches!(request, Request::Input { .. } | Request::Interrupt { .. }) {
            return Err("muted target receives no MCP input or interrupt".into());
        }
    }
    Ok(())
}
pub(crate) fn effect_id(actor: &Actor, call: &Call, target: Option<Target>) -> Ulid {
    let mut hash = Sha256::new();
    hash.update(b"bboxy450/mcp-seat-effect/v1");
    hash.update(call.tool.as_bytes());
    hash.update(actor.seat_id.to_bytes());
    hash.update(actor.process_generation.to_be_bytes());
    hash.update(call.op_id.to_bytes());
    if let Some(target) = target {
        hash.update(target.seat.to_bytes());
        hash.update(target.generation.to_be_bytes());
    }
    // Keep the operation's ULID time, hashing only its entropy. Hashing all 16
    // bytes produced seats dated centuries away from their actual launch.
    let mut bytes = call.op_id.to_bytes();
    bytes[6..].copy_from_slice(&hash.finalize()[6..16]);
    Ulid::from_bytes(bytes)
}
pub(crate) fn failure(op: Ulid, code: &str, reason: &str, ids: Value) -> Value {
    let mut result = super::error(op, code, reason);
    if let Some(ids) = ids.as_object() {
        for (key, value) in ids {
            result["structuredContent"][key] = value.clone();
        }
    }
    result["content"][0]["text"] =
        json!(serde_json::to_string(&result["structuredContent"]).unwrap());
    result
}
pub(crate) fn input_result(call: &Call, id: Ulid, outcome: WakeOutcome) -> Value {
    let cause = outcome.cause;
    let mut result = match outcome.evidence {
        Evidence::NativeWritten | Evidence::NativeQueued => super::success(
            call.op_id,
            json!({"status":if outcome.evidence==Evidence::NativeWritten{"applied"}else{"accepted"},"input_id":id,"evidence":outcome.evidence,"native_id":outcome.native_id,"note":outcome.note}),
        ),
        Evidence::NativeUnknown => {
            failure(call.op_id, "unknown", &outcome.note, json!({"input_id":id}))
        }
        _ => failure(
            call.op_id,
            "not_submitted",
            &format!(
                "not ready: {}",
                if cause == Some(WakeCause::ComposerReadinessMissing) {
                    "no positive readiness"
                } else {
                    &outcome.note
                }
            ),
            json!({"input_id":id}),
        ),
    };
    if let Some(cause) = cause {
        result["structuredContent"]["cause"] = json!(cause);
        result["content"][0]["text"] =
            json!(serde_json::to_string(&result["structuredContent"]).unwrap());
    }
    result
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ViewCursor {
    pub seat: Ulid,
    pub generation: u64,
    pub view: Ulid,
    pub sequence: u64,
    pub row: usize,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CaptureCursor {
    pub seat: Ulid,
    pub generation: u64,
    pub next: u64,
    pub to: u64,
}
pub(crate) fn encode_cursor(value: &impl Serialize) -> Result<String, String> {
    use base64::Engine;
    serde_json::to_vec(value)
        .map(|b| base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(b))
        .map_err(|_| "cursor unavailable".into())
}
pub(crate) fn decode_cursor<T: serde::de::DeserializeOwned>(value: &str) -> Result<T, String> {
    use base64::Engine;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(value)
        .map_err(|_| "invalid cursor")?;
    serde_json::from_slice(&bytes).map_err(|_| "invalid cursor".into())
}

/// Recheck queued native authority on an IO task immediately before the one
/// native attempt. No connection/transaction survives into native I/O.
pub(crate) fn revalidate_native(
    root: &std::path::Path,
    actor: &Actor,
    target: Target,
    harness: &str,
) -> Result<(), String> {
    revalidate(root, actor, target, harness, false)
}
pub(crate) fn revalidate_focus(
    root: &std::path::Path,
    actor: &Actor,
    target: Target,
    harness: &str,
) -> Result<(), String> {
    revalidate(root, actor, target, harness, true)
}
fn revalidate(
    root: &std::path::Path,
    actor: &Actor,
    target: Target,
    harness: &str,
    focus: bool,
) -> Result<(), String> {
    // This account already exists. Authority checks do not initialize schemas
    // or acquire a write lease while several native attempts start together.
    let mut account = rusqlite::Connection::open_with_flags(
        root.join("account/object.sqlite"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .map_err(|_| "native control authority unavailable")?;
    let tx = account
        .transaction()
        .map_err(|_| "native authority snapshot unavailable")?;
    let record = |seat: Ulid| -> Result<SeatRecord, String> {
        let bytes: Vec<u8> = tx
            .query_row(
                "SELECT value FROM kv WHERE key=?1",
                [format!("seat/{seat}")],
                |row| row.get(0),
            )
            .map_err(|_| "native seat unavailable")?;
        serde_json::from_slice(&bytes).map_err(|_| "native seat metadata unavailable".into())
    };
    let caller = record(actor.seat_id)?;
    if caller.generation != actor.process_generation
        || caller.swarm != actor.swarm_id
        || caller.role != actor.role
        || caller.role.class != Class::Oracle
        || caller.muted
    {
        return Err("native caller authority changed before attempt".into());
    }
    let epoch: i64 = tx
        .query_row(
            "SELECT epoch FROM msg_policies WHERE seat=?1",
            [actor.seat_id.to_string()],
            |row| row.get(0),
        )
        .map_err(|_| "native policy unavailable")?;
    if u64::try_from(epoch).ok() != Some(actor.grant_epoch) {
        return Err("native caller policy epoch changed before attempt".into());
    }
    let record = record(target.seat)?;
    if record.generation != target.generation
        || record.swarm != actor.swarm_id
        || (record.muted && !focus)
        || (focus && record.lifecycle != crate::registry::Lifecycle::Open)
        || record.harness_kind != harness
    {
        return Err("native target authority changed before attempt".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{messaging::wire::IssuerKind, registry::Registry, store::sqlite::SqliteStore};

    // keeps: spec mcp-control-plane
    #[test]
    fn native_attempt_rechecks_canonical_caller_policy_and_target_binding() {
        let temp = tempfile::tempdir().unwrap();
        let store = SqliteStore::open(&temp.path().join("account")).unwrap();
        let registry = Registry::new(store.clone());
        let swarm = Ulid::new();
        let mut caller = SeatRecord::new(Ulid::new(), swarm, "Oracle fixture", "zsh");
        caller.role.class = Class::Oracle;
        let target = SeatRecord::new(Ulid::new(), swarm, "Native fixture", "codex");
        registry.insert(&caller).unwrap();
        registry.insert(&target).unwrap();
        store
            .connection()
            .unwrap()
            .execute_batch(
                "CREATE TABLE msg_policies(seat TEXT PRIMARY KEY,epoch INTEGER NOT NULL)",
            )
            .unwrap();
        store
            .connection()
            .unwrap()
            .execute(
                "INSERT INTO msg_policies VALUES(?1,1)",
                [caller.id.to_string()],
            )
            .unwrap();
        let actor = Actor {
            seat_id: caller.id,
            process_generation: 1,
            swarm_id: swarm,
            role: caller.role.clone(),
            grant_epoch: 1,
            issuer_kind: IssuerKind::LocalDev,
        };
        let binding = Target {
            seat: target.id,
            generation: 1,
        };
        for case in [
            "caller-generation",
            "caller-swarm",
            "caller-role",
            "caller-mute",
            "caller-epoch",
            "target-generation",
            "target-swarm",
            "target-mute",
            "target-harness",
        ] {
            assert!(revalidate_native(temp.path(), &actor, binding, "codex").is_ok());
            registry
                .update(caller.id, |record| {
                    match case {
                        "caller-generation" => record.generation += 1,
                        "caller-swarm" => record.swarm = Ulid::new(),
                        "caller-role" => record.role.class = Class::Worker,
                        "caller-mute" => record.muted = true,
                        _ => {}
                    }
                    Ok(())
                })
                .unwrap();
            registry
                .update(target.id, |record| {
                    match case {
                        "target-generation" => record.generation += 1,
                        "target-swarm" => record.swarm = Ulid::new(),
                        "target-mute" => record.muted = true,
                        "target-harness" => record.harness_kind = "zsh".into(),
                        _ => {}
                    }
                    Ok(())
                })
                .unwrap();
            if case == "caller-epoch" {
                store
                    .connection()
                    .unwrap()
                    .execute("UPDATE msg_policies SET epoch=2", [])
                    .unwrap();
            }
            assert!(
                revalidate_native(temp.path(), &actor, binding, "codex").is_err(),
                "{case} authorized a queued native attempt"
            );
            registry
                .update(caller.id, |record| {
                    *record = caller.clone();
                    Ok(())
                })
                .unwrap();
            registry
                .update(target.id, |record| {
                    *record = target.clone();
                    Ok(())
                })
                .unwrap();
            store
                .connection()
                .unwrap()
                .execute("UPDATE msg_policies SET epoch=1", [])
                .unwrap();
        }
    }
}

#[cfg(test)]
mod wake_cause_tests {
    use super::*;

    fn project(evidence: Evidence, cause: Option<WakeCause>, note: &str) -> Value {
        let call = Call {
            op_id: Ulid::new(),
            tool: "seat_input_text".into(),
            args: json!({}),
        };
        let result = input_result(
            &call,
            Ulid::new(),
            WakeOutcome {
                evidence,
                native_id: None,
                note: note.into(),
                cause,
            },
        );
        let text: Value =
            serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(text, result["structuredContent"]);
        result["structuredContent"].clone()
    }

    // keeps: interface app-harness — cause selects the probe; note is not a classifier.
    #[test]
    fn wake_causes_choose_readiness_without_parsing_notes_or_promoting_unknown() {
        let typed = project(
            Evidence::RecipientStored,
            Some(WakeCause::ComposerReadinessMissing),
            "different human wording",
        );
        assert_eq!(typed["error"]["code"], "not_submitted");
        assert_eq!(typed["cause"], "composer-readiness-missing");
        assert_eq!(
            typed["error"]["message"],
            "not ready: no positive readiness"
        );
        for cause in [None, Some(WakeCause::Unrecognized)] {
            let legacy_note = "no positive composer-ready indication";
            let legacy = project(Evidence::RecipientStored, cause, legacy_note);
            assert_eq!(legacy["error"]["code"], "not_submitted");
            assert_eq!(
                legacy["error"]["message"],
                format!("not ready: {legacy_note}")
            );
            if cause.is_none() {
                assert!(legacy.get("cause").is_none());
            }
        }
        for cause in [
            None,
            Some(WakeCause::Unrecognized),
            Some(WakeCause::ComposerReadinessMissing),
            Some(WakeCause::NativeQueueAdmitted),
        ] {
            let unknown = project(Evidence::NativeUnknown, cause, "mutation remains unknown");
            assert_eq!(unknown["error"]["code"], "unknown");
            assert_eq!(unknown["status"], "unknown");
        }
    }
}
