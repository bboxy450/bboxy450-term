//! Seat controls are explicit, generation-bound and independent of GUI focus.
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use ulid::Ulid;

use super::{Call, SeatActor as Actor, tools};
use crate::{
    adapters::{Evidence, WakeOutcome},
    registry::{Class, SeatRecord},
};

pub(crate) fn handles(name: &str) -> bool {
    mutation(name) || matches!(name, "screen_read" | "history_read")
}
pub(crate) fn mutation(name: &str) -> bool {
    matches!(
        name,
        "seat_input_text" | "seat_spawn" | "seat_mute" | "seat_unmute" | "seat_interrupt"
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
            ("seat_spawn","Launch an approved builtin in your swarm through the per-seat wiring kit. No arbitrary commands, environment, privilege class, resume or layout.",json!({"swarm_id":id,"approved_profile":{"enum":["zsh","claude","codex"]},"cwd":{"type":"string","minLength":1,"maxLength":4096},"role":{"type":"string","minLength":1,"maxLength":128}}),vec!["swarm_id","approved_profile","cwd"]),
            ("seat_mute","Set current target mute policy once under the expected policy epoch and process generation.",json!({"expected_policy_epoch":revision}),vec!["expected_policy_epoch"]),
            ("seat_unmute","Clear mute under the current policy epoch; existing eligible held traffic resumes without replaying unknown attempts.",json!({"expected_policy_epoch":revision}),vec!["expected_policy_epoch"]),
            ("seat_interrupt","Send text through a verified native Now/steer endpoint. Unavailable harnesses are named; this never sends a PTY control key.",json!({"text":text}),vec!["text"]),
        ]);
    }
    defs.into_iter().map(|(name,description,mut properties,mut required)| {
        if name!="seat_spawn" {
            properties["seat_id"]=id.clone();properties["expected_process_generation"]=revision.clone();
            required.extend(["seat_id","expected_process_generation"]);
        }
        if mutation(name) { properties["op_id"]=id.clone();required.insert(0,"op_id"); }
        json!({"name":name,"description":description,
            "inputSchema":{"type":"object","properties":properties,"required":required,"additionalProperties":false},
            "outputSchema":{"type":"object","additionalProperties":true},
            "annotations":{"readOnlyHint":!mutation(name),"destructiveHint":mutation(name),"idempotentHint":true,"openWorldHint":false}})
    }).collect()
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
            Self::Screen { target, .. }
            | Self::History { target, .. }
            | Self::Input { target, .. }
            | Self::Mute { target, .. }
            | Self::Interrupt { target, .. } => Some(*target),
            Self::Spawn(_) => None,
        }
    }
    pub fn is_read(&self) -> bool {
        matches!(self, Self::Screen { .. } | Self::History { .. })
    }
}
pub(crate) fn request(actor: &Actor, call: &Call) -> Result<Request, String> {
    if mutation(&call.tool) && actor.role.class != Class::Oracle {
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
        "seat_spawn" => {
            let a: Spawn = tools::parse(&call.args)?;
            if a.swarm_id != actor.swarm_id || a.swarm_id == Ulid::nil() {
                return Err("spawn outside caller swarm".into());
            }
            if !matches!(a.approved_profile.as_str(), "zsh" | "claude" | "codex") {
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
    Ulid::from_bytes(hash.finalize()[..16].try_into().unwrap())
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
    match outcome.evidence {
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
                if outcome.note == "no positive composer-ready indication" {
                    "no positive readiness"
                } else {
                    &outcome.note
                }
            ),
            json!({"input_id":id}),
        ),
    }
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
        || record.muted
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
