//! Bounded operator diagnostics. No arbitrary file, SQL, environment or command line reads.
use crate::{
    Ulid,
    store::{Result, invalid},
};
use bbt_proto::gui::{DebugProcess, DebugRequest};
use rusqlite::{Connection, OpenFlags, OptionalExtension};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::Duration,
};

pub const ROWS: usize = 128;
const LOG_BYTES: usize = 32 * 1024;
const JOURNAL_BYTES: usize = 128 * 1024;
pub const REPLY_BYTES: usize = 256 * 1024;

/// An immutable cut of non-secret worker state, handed to a blocking read task.
pub(crate) struct Context {
    pub root: PathBuf,
    pub peers: Option<crate::ctl_door::Peers>,
    pub seats: Vec<Value>,
    pub seat_count: usize,
    pub watched: usize,
    pub stale_gui_requests: u64,
    pub processes: BTreeMap<u32, &'static str>,
}

impl Context {
    pub fn read(&self, request: &DebugRequest, capture: Value) -> Result<Value> {
        bbt_rt::assert_not_ui_thread();
        let data = match request {
            DebugRequest::Logs { process, since } => logs(&self.root, *process, *since)?,
            DebugRequest::Journal { query } => journal(&self.root, query)?,
            DebugRequest::Ps => process_tree(self.peers, &self.processes)?,
            DebugRequest::Metrics => json!({
                "sampled_at_ms": crate::store::now_ms(),
                "basis": "logic cached state; capture counters are completion-accounted inclusive wall time",
                "logic_pid": std::process::id(), "views": self.seat_count,
                "watched": self.watched, "stale_gui_requests": self.stale_gui_requests,
                "capture": capture,
                "ui": {"available":false,"reason":"use ctl app status for the GUI owner's frame_seq"},
                "supervisor": {"available":false,"reason":"numeric recorder is process-local; no cross-process export"},
            }),
            DebugRequest::Snapshot { .. } => json!({
                "sampled_at_ms": crate::store::now_ms(),
                "reason": "operator_debug_snapshot", "seats": self.seats,
                "omitted_seats": self.seat_count.saturating_sub(self.seats.len()),
                "capture_at_debug": {"logic":capture,
                    "supervisor":{"available":false,"reason":"numeric recorder is process-local; no cross-process export"}},
                "notes": [
                    "cached logic cursors, not fresh ring reads or GUI offered/applied acknowledgements",
                    "disabled capture means unobserved, never zero work; this read does not enable instrumentation",
                    "rows and recorder copied sequentially; compare clocks only within one process_id",
                    "completed inclusive spans overlap; inspect in_flight, missed, untracked and omitted counts",
                ],
            }),
        };
        let data = sanitize(data);
        if serde_json::to_vec(&data)?.len() > REPLY_BYTES {
            return Err(invalid(
                "debug reply exceeds its byte bound; narrow the query",
            ));
        }
        Ok(data)
    }
}

/// Keep the recorder's numeric schema, bounding the copied tail before wire serialization.
pub(crate) fn capture(mut value: Value, spans: bool) -> Value {
    value["state"] = json!(if value["enabled"] == true {
        "enabled"
    } else {
        "disabled"
    });
    for (key, limit) in [("seats", ROWS), ("spans", if spans { 128 } else { 0 })] {
        if let Some(rows) = value[key].as_array_mut() {
            let omitted = rows.len().saturating_sub(limit);
            if key == "spans" {
                rows.drain(..omitted);
            } else {
                rows.truncate(limit);
            }
            value[format!("omitted_{key}")] = json!(omitted);
        }
    }
    value
}

fn private_source(root: &Path, object: Option<&str>, file: &str) -> Result<PathBuf> {
    let root = root.canonicalize()?;
    let directory = object.map_or_else(|| root.clone(), |o| root.join(o));
    // Opening a live Store or Ring would repair/create data. Only existing regular
    // files under the selected run are eligible, with no symlink indirection.
    if std::fs::symlink_metadata(&directory)?
        .file_type()
        .is_symlink()
    {
        return Err(invalid("debug source directory must not be a symlink"));
    }
    let path = directory.join(file);
    let metadata = std::fs::symlink_metadata(&path)?;
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || !path.canonicalize()?.starts_with(&root)
    {
        return Err(invalid(
            "debug source must be a regular file within the app run",
        ));
    }
    Ok(path)
}

fn logs(root: &Path, process: DebugProcess, since: u64) -> Result<Value> {
    let name = match process {
        DebugProcess::App => "app",
        DebugProcess::Logic => "logic",
        DebugProcess::Supervisor => "supervisor",
    };
    let path = match private_source(root, None, &format!("{name}.log")) {
        Ok(path) => path,
        Err(error)
            if error
                .downcast_ref::<std::io::Error>()
                .is_some_and(|e| e.kind() == std::io::ErrorKind::NotFound) =>
        {
            return Ok(
                json!({"process":name,"available":false,"reason":"log not recorded in this run","lines":[],"next_since":since}),
            );
        }
        Err(error) => return Err(error),
    };
    use std::os::unix::fs::OpenOptionsExt;
    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)?;
    let length = file.metadata()?.len();
    if since > length {
        return Err(invalid(
            "debug logs cursor exceeds file length; log rotated, restart with --since 0",
        ));
    }
    if since > 0 && since < length {
        file.seek(SeekFrom::Start(since - 1))?;
        let mut previous = [0];
        file.read_exact(&mut previous)?;
        if previous[0] != b'\n' {
            return Err(invalid(
                "debug logs --since must be a next_since line boundary",
            ));
        }
    }
    file.seek(SeekFrom::Start(since))?;
    let mut bytes = Vec::new();
    file.take(LOG_BYTES as u64).read_to_end(&mut bytes)?;
    let mut lines = Vec::new();
    let mut consumed = 0;
    for line in bytes.split_inclusive(|b| *b == b'\n').take(256) {
        if line.last() != Some(&b'\n') {
            break;
        }
        consumed += line.len();
        let text = String::from_utf8_lossy(line);
        let lower = text.to_ascii_lowercase();
        lines.push(
            if [
                "capability",
                "secret",
                "private_key",
                "signing_key",
                "password",
                "seed",
                "token",
                "key=",
                "key:",
            ]
            .iter()
            .any(|label| lower.contains(label))
            {
                "[redacted diagnostic line]\n".into()
            } else {
                scrub(&text)
            },
        );
    }
    // Never return a fragment of an encoded secret. A single oversized/unfinished
    // line is omitted, including the unread remainder, rather than split across pages.
    let omitted_bytes = if consumed == 0 && !bytes.is_empty() {
        length - since
    } else {
        0
    };
    let next = since + consumed as u64 + omitted_bytes;
    Ok(
        json!({"process":name,"available":true,"since":since,"next_since":next,
        "lines":lines,"omitted_bytes":omitted_bytes,"more":next<length,"redacted":true}),
    )
}

/// A single operation only. Bind the key; no caller-supplied SQL or other KV namespace.
pub fn journal_query(query: &str) -> std::result::Result<(Option<Ulid>, Ulid), &'static str> {
    let parts: Vec<_> = query.split('/').collect();
    match parts.as_slice() {
        ["account", op] => op
            .parse()
            .map(|op| (None, op))
            .map_err(|_| "invalid operation ID"),
        ["seat", seat, op] => Ok((
            Some(seat.parse().map_err(|_| "invalid seat ID")?),
            op.parse().map_err(|_| "invalid operation ID")?,
        )),
        _ => Err("journal query: account/<op-id> or seat/<seat-id>/<op-id>; SQL is not accepted"),
    }
}

fn journal(root: &Path, query: &str) -> Result<Value> {
    let (seat, op) = journal_query(query).map_err(invalid)?;
    let object = seat.map_or_else(|| "account".into(), |id| format!("seat-{id}"));
    let path = private_source(root, Some(&object), "object.sqlite")?;
    let mut uri = String::from("file:");
    use std::os::unix::ffi::OsStrExt;
    for b in path.as_os_str().as_bytes() {
        if b.is_ascii_alphanumeric() || b"/-_.".contains(b) {
            uri.push(char::from(*b));
        } else {
            use std::fmt::Write;
            write!(&mut uri, "%{b:02X}").unwrap();
        }
    }
    uri.push_str("?mode=ro");
    let db = Connection::open_with_flags(
        uri,
        OpenFlags::SQLITE_OPEN_READ_ONLY
            | OpenFlags::SQLITE_OPEN_URI
            | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    db.busy_timeout(Duration::from_millis(100))?;
    db.pragma_update(None, "query_only", true)?;
    let row: Option<(i64, Vec<u8>)> = db
        .query_row(
            "SELECT length(value), substr(value,1,?2) FROM kv WHERE key=?1",
            rusqlite::params![format!("control/op/{op}"), (JOURNAL_BYTES + 1) as i64],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    let entry = row.map(|(length, bytes)| -> Result<Value> {
        if length > JOURNAL_BYTES as i64 { return Err(invalid("debug journal entry exceeds byte bound")); }
        let entry: Value = serde_json::from_slice(&bytes)?;
        // Project only the journal schema; never neighboring KV/key material.
        Ok(json!({"actor":entry["actor"],"tool":entry["tool"],"args_digest":entry["args_digest"],"result":entry["result"]}))
    }).transpose()?;
    Ok(json!({"query":query,"mode":"ro","query_only":true,"entry":entry,"redacted":true}))
}

fn process_tree(
    peers: Option<crate::ctl_door::Peers>,
    known: &BTreeMap<u32, &'static str>,
) -> Result<Value> {
    let mut known = known.clone();
    known.insert(std::process::id(), "logic");
    if let Some(peers) = peers {
        known.insert(peers.gui, "app");
        known.insert(peers.supervisor, "supervisor");
    }
    // Only numeric pid/ppid columns are read, never argv or environment. The
    // bounded result is filtered to this app before any bytes cross the wire.
    let mut child = Command::new("/bin/ps")
        .args(["-axo", "pid=,ppid="])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    let mut bytes = Vec::new();
    child
        .stdout
        .take()
        .unwrap()
        .take(1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 1024 * 1024 {
        let _ = child.kill();
        let _ = child.wait();
        return Err(invalid("debug process scan exceeds byte bound"));
    }
    if !child.wait()?.success() {
        return Err(invalid("debug process snapshot unavailable"));
    }
    let parents: BTreeMap<u32, u32> = String::from_utf8_lossy(&bytes)
        .lines()
        .filter_map(|line| {
            let mut words = line.split_whitespace();
            Some((words.next()?.parse().ok()?, words.next()?.parse().ok()?))
        })
        .collect();
    let mut owned = BTreeSet::from([std::process::id()]);
    if let Some(peers) = peers {
        if parents.get(&std::process::id()) == Some(&peers.gui) {
            owned.insert(peers.gui);
        }
        if parents
            .get(&peers.supervisor)
            .is_some_and(|parent| owned.contains(parent))
        {
            owned.insert(peers.supervisor);
        }
    }
    // A finite depth and row budget; a vanished or reparented process is not
    // invented. Seat/companion IDs only label proven descendants, never seed
    // the tree (an old seat PID may already have been reused by another app).
    for _ in 0..64 {
        let additions: Vec<_> = parents
            .iter()
            .filter(|(pid, parent)| !owned.contains(pid) && owned.contains(parent))
            .map(|(pid, _)| *pid)
            .take(2048 - owned.len().min(2048))
            .collect();
        if additions.is_empty() {
            break;
        }
        owned.extend(additions);
    }
    let count = owned.len();
    let processes: Vec<_> = owned.into_iter().take(ROWS).map(|pid| json!({"pid":pid,"parent":parents[&pid],"role":known.get(&pid).copied().unwrap_or("descendant")})).collect();
    Ok(
        json!({"sampled_at_ms":crate::store::now_ms(),"processes":processes,
        "omitted":count.saturating_sub(ROWS),"truncated":count>ROWS || count>=2048,
        "basis":"app-owned roots and current numeric parent links; no argv or environment"}),
    )
}

fn sensitive(name: &str) -> bool {
    let name: String = name
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .flat_map(char::to_lowercase)
        .collect();
    [
        "capability",
        "secret",
        "privatekey",
        "signingkey",
        "seed",
        "token",
        "password",
    ]
    .iter()
    .any(|s| name.contains(s))
}
/// Diagnostic text is untrusted, including persisted tool results and log errors.
/// Suppress encoded 256-bit values (hex/base64/byte arrays), secret-labelled
/// fields; numeric recorder fields retain their types.
pub fn sanitize(value: Value) -> Value {
    match value {
        Value::String(text) => Value::String(scrub(&text)),
        Value::Array(rows)
            if rows.len() == 32 && rows.iter().all(|v| v.as_u64().is_some_and(|n| n <= 255)) =>
        {
            json!("[redacted]")
        }
        Value::Array(rows) => Value::Array(rows.into_iter().map(sanitize).collect()),
        Value::Object(fields) => Value::Object(
            fields
                .into_iter()
                .map(|(key, value)| {
                    let value = if sensitive(&key) {
                        json!("[redacted]")
                    } else {
                        sanitize(value)
                    };
                    (scrub(&key), value)
                })
                .collect(),
        ),
        value => value,
    }
}

fn scrub(text: &str) -> String {
    let mut result = String::new();
    let mut rest = text;
    while !rest.is_empty() {
        if rest.starts_with('[')
            && let Some(end) = rest.find(']')
        {
            let candidate = &rest[..=end];
            if serde_json::from_str::<Vec<u8>>(candidate).is_ok_and(|v| v.len() == 32) {
                result.push_str("[redacted]");
                rest = &rest[end + 1..];
                continue;
            }
        }
        let token = rest
            .bytes()
            .take_while(|b| b.is_ascii_alphanumeric() || b"+/_=-".contains(b))
            .count();
        if token >= 40 {
            result.push_str("[redacted]");
            rest = &rest[token..];
        } else if token > 0 {
            result.push_str(&rest[..token]);
            rest = &rest[token..];
        } else {
            let ch = rest.chars().next().unwrap();
            if !ch.is_control() || ch == '\n' || ch == '\t' {
                result.push(ch);
            }
            rest = &rest[ch.len_utf8()..];
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn journal_selects_one_operation_read_only_without_opening_other_namespaces() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::create_dir(directory.path().join("account")).unwrap();
        let db_path = directory.path().join("account/object.sqlite");
        let db = Connection::open(&db_path).unwrap();
        db.execute_batch("CREATE TABLE kv(key TEXT PRIMARY KEY,value BLOB NOT NULL)")
            .unwrap();
        let op = Ulid::new();
        let value = json!({"actor":{"device_id":Ulid::new(),"door":"ctl"},"tool":"message_send","args_digest":"digest","result":{"text":"receipt","private_key":vec![7;32]},"extra":"not journal schema"});
        db.execute(
            "INSERT INTO kv VALUES(?1,?2)",
            rusqlite::params![
                format!("control/op/{op}"),
                serde_json::to_vec(&value).unwrap()
            ],
        )
        .unwrap();
        db.execute(
            "INSERT INTO kv VALUES('identity/key',?1)",
            [b"private-store-sentinel".to_vec()],
        )
        .unwrap();
        drop(db);
        let before = std::fs::read(&db_path).unwrap();
        let result = journal(directory.path(), &format!("account/{op}")).unwrap();
        assert_eq!(result["mode"], "ro");
        assert_eq!(result["query_only"], true);
        assert_eq!(result["entry"]["tool"], "message_send");
        assert!(result["entry"].get("extra").is_none());
        assert_eq!(before, std::fs::read(&db_path).unwrap());
        for query in [
            "SELECT * FROM kv",
            "account/../identity/key",
            "identity/key",
            "control/op/0",
        ] {
            assert!(journal(directory.path(), query).is_err());
        }
        assert!(
            !serde_json::to_string(&result)
                .unwrap()
                .contains("private-store-sentinel")
        );
        let unknown = Ulid::new();
        assert!(journal(directory.path(), &format!("seat/{unknown}/{op}")).is_err());
        assert!(!directory.path().join(format!("seat-{unknown}")).exists());
    }

    #[test]
    fn log_cursors_are_bounded_complete_lines_and_cannot_split_secrets() {
        let directory = tempfile::tempdir().unwrap();
        let text = format!("first\n{}\nlast\n", "ab".repeat(32));
        std::fs::write(directory.path().join("app.log"), &text).unwrap();
        let value = logs(directory.path(), DebugProcess::App, 0).unwrap();
        assert_eq!(value["lines"][0], "first\n");
        assert!(!value.to_string().contains(&"ab".repeat(32)));
        assert_eq!(value["next_since"], text.len());
        assert!(logs(directory.path(), DebugProcess::App, 12).is_err());
        assert!(logs(directory.path(), DebugProcess::App, text.len() as u64 + 1).is_err());
        std::fs::write(directory.path().join("app.log"), "x\n".repeat(1000)).unwrap();
        let first = logs(directory.path(), DebugProcess::App, 0).unwrap();
        assert_eq!(first["lines"].as_array().unwrap().len(), 256);
        assert_eq!(first["next_since"], 512);
        assert_eq!(first["more"], true);
        let next = logs(directory.path(), DebugProcess::App, 512).unwrap();
        assert_eq!(next["since"], 512);
        std::fs::write(directory.path().join("app.log"), "a".repeat(LOG_BYTES * 2)).unwrap();
        let oversized = logs(directory.path(), DebugProcess::App, 0).unwrap();
        assert_eq!(oversized["lines"], json!([]));
        assert_eq!(oversized["omitted_bytes"], LOG_BYTES * 2);
    }

    #[test]
    fn debug_sources_refuse_symlinks_and_encoded_key_material() {
        let root = tempfile::tempdir().unwrap();
        let other = tempfile::NamedTempFile::new().unwrap();
        std::os::unix::fs::symlink(other.path(), root.path().join("app.log")).unwrap();
        assert!(logs(root.path(), DebugProcess::App, 0).is_err());
        let seed = vec![197u8; 32];
        let encodings = [
            "c5".repeat(32),
            "C5".repeat(32),
            "xcXFxcXFxcXFxcXFxcXFxcXFxcXFxcXFxcXFxcXFxcU=".into(),
            serde_json::to_string(&seed).unwrap(),
        ];
        let value = json!({"capability":seed,"nested":{"secret":"short-value"},"texts":encodings});
        let redacted = sanitize(value).to_string();
        for encoding in encodings {
            assert!(!redacted.contains(&encoding));
        }
        assert!(!redacted.contains("short-value"));
    }
}
