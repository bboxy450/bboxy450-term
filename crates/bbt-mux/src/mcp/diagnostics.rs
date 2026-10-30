// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Own-seat diagnostic projection. Never opens the operator's acknowledgement file.
use super::{Call, SeatActor};
use crate::{Ulid, knowledge::data_source as d, store::digest};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use bbt_rt::error_journal::{self as journal, ARCHIVES, MAX_FILE, MAX_RECORD};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    fs::OpenOptions,
    io::{BufRead, BufReader, Read},
    path::Path,
};

const MAX_ROWS: usize = 64;
const PAGE_BYTES: usize = 64 * 1024;
const CURSOR_AGE_MS: u64 = 5 * 60 * 1000;

#[derive(Clone, Serialize)]
pub(crate) struct Context {
    pub actor: SeatActor,
    pub workspace_revision: u64,
}
impl Context {
    pub(crate) fn current(&self, actor: &SeatActor, revision: u64) -> bool {
        &self.actor == actor && self.workspace_revision == revision
    }
}

pub(crate) fn catalogue() -> Value {
    json!({"name":"diagnostic_errors",
        "description":"Read bounded redacted error-journal pages for your own seat. Never acknowledges the operator's journal. No swarm/account read grant exists on this surface. Missing rows do not prove cause or recovery. Cursors expire after five minutes or a scoped snapshot/authority change.",
        "inputSchema":{"type":"object","properties":{"seat_id":{"type":"string"},"scope":{"type":"string","enum":["seat"]},"since_ms":{"type":"integer","minimum":0},"limit":{"type":"integer","minimum":1,"maximum":MAX_ROWS},"cursor":{"type":"string","maxLength":4096}},"required":[],"additionalProperties":false},
        "outputSchema":{"type":"object","additionalProperties":true},
        "annotations":{"readOnlyHint":true,"destructiveHint":false,"idempotentHint":true,"openWorldHint":false}})
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    seat_id: Option<Ulid>,
    scope: Option<String>,
    since_ms: Option<u64>,
    limit: Option<usize>,
    cursor: Option<String>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Cursor {
    context: String,
    snapshot: String,
    since_ms: Option<u64>,
    after: String,
    issued_ms: u64,
}
fn error(code: &str, message: &str) -> d::CommandError {
    d::CommandError::new(code, message)
}
fn scrub(text: &str, bound: usize) -> String {
    // Classify the complete bounded line before truncation can remove its label.
    journal::scrub(&journal::scrub(text, MAX_RECORD), bound)
}

// Adapted from bboxy450-term crates/bbt-rt/src/error_journal.rs::private/read/normalized.
// https://github.com/bboxy450/bboxy450-term — preserve private-file admission,
// bound every read, and whitelist fields; this projection adds scoped paging and
// deliberately has no operator read-mark access.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
fn rows(root: &Path, seat: Ulid, since: Option<u64>) -> d::Result<(Vec<Value>, bool)> {
    use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
    let mut found = false;
    let mut incomplete = false;
    let mut rows = Vec::new();
    for name in (1..=ARCHIVES)
        .rev()
        .map(|n| format!("diagnostics.{n}.jsonl"))
        .chain(["diagnostics.jsonl".into()])
    {
        let file = match OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK)
            .open(root.join(name))
        {
            Ok(file) => file,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(_) => {
                return Err(error(
                    "diagnostics_unavailable",
                    "diagnostic store unavailable or unsafe",
                ));
            }
        };
        let m = file
            .metadata()
            .map_err(|_| error("diagnostics_unavailable", "diagnostic metadata unavailable"))?;
        // SAFETY: geteuid takes no pointers and does not mutate process state.
        if !m.is_file()
            || m.uid() != unsafe { libc::geteuid() }
            || m.mode() & 0o077 != 0
            || m.nlink() != 1
        {
            return Err(error(
                "diagnostics_unavailable",
                "diagnostic store must be an owned private regular file",
            ));
        }
        found = true;
        incomplete |= m.len() > MAX_FILE + MAX_RECORD as u64;
        let mut reader = BufReader::new(file).take(MAX_FILE + MAX_RECORD as u64);
        loop {
            let mut line = Vec::new();
            let n = (&mut reader)
                .take(MAX_RECORD as u64 + 1)
                .read_until(b'\n', &mut line)
                .map_err(|_| error("diagnostics_unavailable", "diagnostic read unavailable"))?;
            if n == 0 {
                break;
            }
            if line.len() > MAX_RECORD || line.last() != Some(&b'\n') {
                incomplete = true;
                break;
            }
            let Ok(row) = serde_json::from_slice::<Value>(&line) else {
                incomplete = true;
                continue;
            };
            if row["seat"].as_str().and_then(|s| s.parse::<Ulid>().ok()) != Some(seat) {
                continue;
            }
            let Some(at_ms) = row["at_ms"].as_u64() else {
                incomplete = true;
                continue;
            };
            if since.is_some_and(|since| at_ms < since) {
                continue;
            }
            let Some(id) = row["id"].as_str().filter(|id| {
                id.len() <= 96
                    && id.split('-').count() == 3
                    && id
                        .split('-')
                        .all(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))
            }) else {
                incomplete = true;
                continue;
            };
            if row["version"] != 1 {
                incomplete = true;
                continue;
            }
            let Some(source) = row["source"].as_str() else {
                incomplete = true;
                continue;
            };
            let Some(operation) = row["operation"].as_str() else {
                incomplete = true;
                continue;
            };
            let Some(message) = row["message"].as_str() else {
                incomplete = true;
                continue;
            };
            let original_op_id = operation
                .rsplit_once([' ', '/'])
                .and_then(|(_, op)| op.parse::<Ulid>().ok())
                .filter(|op| *op != Ulid::nil());
            rows.push(json!({"id":id,"error_ref":format!("diagnostic:{id}"),"at_ms":at_ms,
                "seat":seat,"source":scrub(source,64),"operation":scrub(operation,128),
                "original_op_id":original_op_id,"message":scrub(message,1024),
                "stderr":scrub(row["stderr"].as_str().unwrap_or(""),4096),
                "harness":row["harness"].as_str().map(|s|scrub(s,64)),
                "exit_code":row["exit_code"].as_i64().and_then(|n|i32::try_from(n).ok()),
                "duration_ms":row["duration_ms"].as_u64(),"repeats":row["repeats"].as_u64().unwrap_or(1)}));
        }
    }
    if !found {
        return Err(error(
            "diagnostics_unavailable",
            "no retained diagnostic store is available",
        ));
    }
    rows.sort_by(|a, b| {
        b["at_ms"]
            .as_u64()
            .cmp(&a["at_ms"].as_u64())
            .then_with(|| b["id"].as_str().cmp(&a["id"].as_str()))
    });
    rows.dedup_by(|a, b| a["id"] == b["id"]);
    Ok((rows, incomplete))
}

pub(crate) fn read(root: &Path, context: &Context, call: &Call) -> Value {
    let result = page(root, context, &call.args, journal::now_ms());
    match result {
        Ok(value) => {
            super::knowledge::bounded(call.op_id, super::success(call.op_id, value), false)
        }
        Err(error) => super::error(call.op_id, &error.code, &error.message),
    }
}
fn page(root: &Path, context: &Context, args: &Value, now: u64) -> d::Result<Value> {
    let request: Request = super::tools::parse(args).map_err(|_| {
        error(
            "invalid_arguments",
            "arguments do not match diagnostic schema",
        )
    })?;
    if request
        .seat_id
        .is_some_and(|id| id != context.actor.seat_id)
        || request
            .scope
            .as_deref()
            .is_some_and(|scope| scope != "seat")
    {
        return Err(error(
            "forbidden",
            "only your own seat's diagnostics are authorized",
        ));
    }
    let limit = request.limit.unwrap_or(32);
    if !(1..=MAX_ROWS).contains(&limit) {
        return Err(error("invalid_arguments", "limit must be 1..64"));
    }
    let context_hash = digest(&serde_json::to_vec(context).expect("context serializes"));
    let cursor = request
        .cursor
        .as_ref()
        .map(|text| {
            if text.len() > 4096 {
                return Err(error(
                    "invalid_cursor",
                    "diagnostic cursor exceeds its bound",
                ));
            }
            let bytes = URL_SAFE_NO_PAD
                .decode(text)
                .map_err(|_| error("invalid_cursor", "invalid diagnostic cursor"))?;
            let cursor: Cursor = serde_json::from_slice(&bytes)
                .map_err(|_| error("invalid_cursor", "invalid diagnostic cursor"))?;
            if cursor.context != context_hash || cursor.since_ms != request.since_ms {
                return Err(error(
                    "invalid_cursor",
                    "diagnostic cursor belongs to another scope or filter",
                ));
            }
            if now < cursor.issued_ms || now - cursor.issued_ms > CURSOR_AGE_MS {
                return Err(error(
                    "cursor_expired",
                    "diagnostic cursor expired; start a new page walk",
                ));
            }
            Ok(cursor)
        })
        .transpose()?;
    let (rows, incomplete) = rows(root, context.actor.seat_id, request.since_ms)?;
    let snapshot = digest(
        &serde_json::to_vec(&json!({"rows":rows,"incomplete":incomplete})).expect("rows serialize"),
    );
    let start = match &cursor {
        Some(cursor) if cursor.snapshot != snapshot => {
            return Err(error(
                "stale_cursor",
                "diagnostic snapshot changed or rotated; start a new page walk",
            ));
        }
        Some(cursor) => rows
            .iter()
            .position(|row| row["id"] == cursor.after)
            .map(|i| i + 1)
            .ok_or_else(|| error("invalid_cursor", "diagnostic cursor anchor is absent"))?,
        None => 0,
    };
    let mut selected = Vec::new();
    let mut bytes = 2048;
    for row in rows.iter().skip(start).take(limit) {
        let size = serde_json::to_vec(row).expect("row serializes").len() + 1;
        if bytes + size > PAGE_BYTES {
            break;
        }
        bytes += size;
        selected.push(row.clone());
    }
    let next = if start + selected.len() < rows.len() {
        let after = selected
            .last()
            .and_then(|row| row["id"].as_str())
            .ok_or_else(|| error("response_bound", "diagnostic row exceeds the page bound"))?;
        Some(
            URL_SAFE_NO_PAD.encode(
                serde_json::to_vec(&Cursor {
                    context: context_hash,
                    snapshot,
                    since_ms: request.since_ms,
                    after: after.into(),
                    issued_ms: cursor.map_or(now, |cursor| cursor.issued_ms),
                })
                .expect("cursor serializes"),
            ),
        )
    } else {
        None
    };
    Ok(
        json!({"rows":selected,"next":next,"scope":"seat","seat_id":context.actor.seat_id,
        "swarm_id":context.actor.swarm_id,"workspace_revision":context.workspace_revision,
        "root_scope":"not_recorded_by_journal_v1","acknowledged":false,"incomplete":incomplete,
        "snapshot":"best_effort; references identify observations, not causes or effect outcomes",
        "order":"newest_first","retention_bytes":MAX_FILE * (ARCHIVES as u64+1),"cursor_max_age_ms":CURSOR_AGE_MS}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        messaging::wire::IssuerKind,
        registry::{Class, Role},
    };
    use std::os::unix::fs::PermissionsExt;
    fn context() -> Context {
        Context {
            actor: SeatActor {
                seat_id: Ulid::from(1_u128),
                process_generation: 1,
                swarm_id: Ulid::from(2_u128),
                role: Role {
                    name: "executor".into(),
                    class: Class::Worker,
                },
                grant_epoch: 1,
                issuer_kind: IssuerKind::LocalDev,
            },
            workspace_revision: 1,
        }
    }
    fn fixture(count: usize) -> (tempfile::TempDir, Context) {
        let temp = tempfile::tempdir().unwrap();
        let context = context();
        let mut rows = String::new();
        for n in 1..=count {
            let row = json!({"version":1,"id":format!("100-{n}-0"),"at_ms":100,"pid":n,
                "source":"mcp","operation":"knowledge_fetch","seat":context.actor.seat_id,
                "message":format!("password=short-secret-sentinel {} trailing", "padding ".repeat(180)),
                "stderr":format!("Authorization: Bearer {}", "x".repeat(5000)),
                "unknown_field":"unknown-secret-sentinel"});
            rows.push_str(&format!("{row}\n"));
        }
        let path = temp.path().join("diagnostics.jsonl");
        std::fs::write(&path, rows).unwrap();
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).unwrap();
        (temp, context)
    }
    #[test]
    fn diagnostic_pages_bound_and_redact_before_truncation_and_never_acknowledge() {
        let (temp, context) = fixture(100);
        let first = page(temp.path(), &context, &json!({"limit":64}), 1000).unwrap();
        let bytes = serde_json::to_vec(&first).unwrap();
        assert!(bytes.len() <= PAGE_BYTES);
        let text = String::from_utf8(bytes).unwrap();
        assert!(!text.contains("short-secret-sentinel"));
        assert!(!text.contains("unknown-secret-sentinel"));
        assert!(!text.contains(&"x".repeat(32)));
        assert!(!temp.path().join(".diagnostics-read").exists());
        let mut ids = std::collections::BTreeSet::new();
        let mut current = first;
        loop {
            for row in current["rows"].as_array().unwrap() {
                assert!(ids.insert(row["id"].as_str().unwrap().to_owned()));
            }
            if current["next"].is_null() {
                break;
            }
            current = page(
                temp.path(),
                &context,
                &json!({"limit":64,"cursor":current["next"]}),
                1000,
            )
            .unwrap();
        }
        assert_eq!(ids.len(), 100);
    }
    #[test]
    fn diagnostic_cursor_rejects_age_scope_filter_rotation_and_unsafe_or_incomplete_store() {
        let (temp, context) = fixture(2);
        let first = page(temp.path(), &context, &json!({"limit":1}), 1000).unwrap();
        let args = json!({"cursor":first["next"]});
        assert_eq!(
            page(temp.path(), &context, &args, 1000 + CURSOR_AGE_MS + 1)
                .unwrap_err()
                .code,
            "cursor_expired"
        );
        let mut other = context.clone();
        other.workspace_revision += 2; // A -> B -> A still has another revision.
        assert!(!context.current(&other.actor, other.workspace_revision));
        assert_eq!(
            page(temp.path(), &other, &args, 1000).unwrap_err().code,
            "invalid_cursor"
        );
        other = context.clone();
        other.actor.grant_epoch += 1;
        assert!(!context.current(&other.actor, other.workspace_revision));
        other = context.clone();
        other.actor.process_generation += 1;
        assert!(!context.current(&other.actor, other.workspace_revision));
        assert_eq!(
            page(
                temp.path(),
                &context,
                &json!({"cursor":first["next"],"since_ms":1}),
                1000
            )
            .unwrap_err()
            .code,
            "invalid_cursor"
        );
        let path = temp.path().join("diagnostics.jsonl");
        std::fs::write(&path, b"").unwrap();
        assert_eq!(
            page(temp.path(), &context, &args, 1000).unwrap_err().code,
            "stale_cursor"
        );
        std::fs::write(&path, "x".repeat(MAX_RECORD + 1)).unwrap();
        let bounded = page(temp.path(), &context, &json!({}), 1000).unwrap();
        assert_eq!(bounded["incomplete"], true);
        assert!(bounded["rows"].as_array().unwrap().is_empty());
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert_eq!(
            page(temp.path(), &context, &json!({}), 1000)
                .unwrap_err()
                .code,
            "diagnostics_unavailable"
        );
        std::fs::remove_file(&path).unwrap();
        std::os::unix::fs::symlink("missing-private-source", &path).unwrap();
        assert_eq!(
            page(temp.path(), &context, &json!({}), 1000)
                .unwrap_err()
                .code,
            "diagnostics_unavailable"
        );
    }
}
