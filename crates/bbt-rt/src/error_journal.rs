// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Best-effort failure reporting. Producers only send bounded nonblocking
//! datagrams; one run-owned background thread performs every file operation.
use serde_json::{Value, json};
use std::fs::{File, OpenOptions};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{
    Arc, OnceLock,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Maximum retained stderr tail in bytes, before and after redaction.
pub const MAX_STDERR: usize = 4096;
/// Maximum encoded datagram/record size, including its terminating newline.
pub const MAX_RECORD: usize = 16 * 1024;
/// Rotation size of the active diagnostics file in bytes.
pub const MAX_FILE: u64 = 1024 * 1024;
/// Number of bounded older files retained beside the active file.
pub const ARCHIVES: usize = 2;
/// Maximum number of rows returned by one filtered read.
pub const MAX_ROWS: usize = 256;
/// Encoded read-page byte bound, including room for response metadata.
pub const MAX_PAGE: usize = 128 * 1024;
const SOCKET: &str = "diagnostics.sock";
const ACTIVE: &str = "diagnostics.jsonl";
const READ: &str = ".diagnostics-read";
static DIRECTORY: OnceLock<PathBuf> = OnceLock::new();
static SERIAL: AtomicU64 = AtomicU64::new(0);

/// Configure once at process bootstrap, before handling a seat or UI request.
pub fn configure(root: &Path) {
    let _ = DIRECTORY.set(root.to_path_buf());
}

/// Current Unix milliseconds, or zero if the process clock precedes the epoch.
pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(u64::MAX as u128) as u64
}

#[derive(Clone, Debug)]
/// One failure observation. Strings are bounded and redacted before transmission.
pub struct Failure {
    /// Component reporting the failure, such as hook, MCP, ctl or supervisor.
    pub source: String,
    /// Named operation or hook; never its raw command line or arguments.
    pub operation: String,
    /// Diagnostic reason; request bodies and environment maps must not be supplied.
    pub message: String,
    /// Exact seat identity when known.
    pub seat: Option<String>,
    /// Native harness kind when known.
    pub harness: Option<String>,
    /// Actual child exit code; absent when not observed, including signal-only exits.
    pub exit_code: Option<i32>,
    /// Measured duration of the failed operation in milliseconds when available.
    pub duration_ms: Option<u64>,
    /// Stderr tail, scrubbed again before transmission and before persistence.
    pub stderr: String,
}
impl Failure {
    /// Build a component/operation observation without guessing seat or exit metadata.
    pub fn new(source: &str, operation: &str, message: impl ToString) -> Self {
        Self {
            source: source.into(),
            operation: operation.into(),
            message: message.to_string(),
            seat: None,
            harness: None,
            exit_code: None,
            duration_ms: None,
            stderr: String::new(),
        }
    }
    /// Associate an observed seat identity.
    pub fn seat(mut self, seat: impl ToString) -> Self {
        self.seat = Some(seat.to_string());
        self
    }
    /// Associate the native harness kind.
    pub fn harness(mut self, harness: impl ToString) -> Self {
        self.harness = Some(harness.to_string());
        self
    }
    /// Record only an actually observed exit code, retaining unknown as absent.
    pub fn exit(mut self, exit: Option<i32>) -> Self {
        self.exit_code = exit;
        self
    }
    /// Associate a measured duration, saturating at the numeric schema bound.
    pub fn duration(mut self, elapsed: Duration) -> Self {
        self.duration_ms = Some(elapsed.as_millis().min(u64::MAX as u128) as u64);
        self
    }
    /// Retain only the last four KiB; invalid UTF-8 is replaced and secrets scrubbed on send.
    pub fn stderr(mut self, bytes: &[u8]) -> Self {
        self.stderr =
            String::from_utf8_lossy(&bytes[bytes.len().saturating_sub(MAX_STDERR)..]).into();
        self
    }
    fn value(self) -> Value {
        let at_ms = now_ms();
        json!({"version":1,"id":format!("{at_ms}-{}-{}",std::process::id(),SERIAL.fetch_add(1,Ordering::Relaxed)),
            "at_ms":at_ms,"pid":std::process::id(),"source":scrub(&self.source,64),
            "operation":scrub(&self.operation,128),"message":scrub(&self.message,1024),
            "seat":self.seat.map(|s|scrub(&s,64)),"harness":self.harness.map(|s|scrub(&s,64)),
            "exit_code":self.exit_code,"duration_ms":self.duration_ms,"stderr":scrub(&self.stderr,MAX_STDERR)})
    }
}

/// Never enqueue raw argv, request bodies, environment maps or credentials.
/// A suspicious diagnostic line is withheld in full; long opaque tokens and
/// control sequences are also withheld, including when no field label exists.
pub fn scrub(text: &str, limit: usize) -> String {
    let tail = &text[text
        .char_indices()
        .map(|(i, _)| i)
        .find(|i| text.len() - i <= limit)
        .unwrap_or(text.len())..];
    let mut result = String::new();
    for line in tail.lines() {
        if !result.is_empty() {
            result.push('\n');
        }
        let lower = line.to_ascii_lowercase();
        let assignment = line.split_whitespace().any(|word| {
            word.split_once(['=', ':']).is_some_and(|(key, _)| {
                let key = key.trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '_');
                key.len() >= 2
                    && key
                        .bytes()
                        .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_')
            })
        });
        if assignment
            || [
                "token",
                "secret",
                "password",
                "capability",
                "authorization",
                "bearer ",
                "private_key",
                "signing_key",
                "api_key",
                "api-key",
                "credential",
                "seed=",
                "seed:",
                "sk-",
                "ghp_",
                "github_pat_",
            ]
            .iter()
            .any(|s| lower.contains(s))
        {
            result.push_str("[redacted sensitive diagnostic line]");
            continue;
        }
        for (index, word) in line.split_whitespace().enumerate() {
            if index > 0 {
                result.push(' ');
            }
            let opaque = word
                .split(|c: char| !c.is_ascii_alphanumeric() && !"_-+/=.".contains(c))
                .any(|part| part.len() >= 32);
            if opaque || word.contains('\u{1b}') {
                result.push_str("[redacted token]");
            } else {
                result.extend(word.chars().filter(|c| !c.is_control()));
            }
        }
    }
    if result.len() > limit {
        result.truncate(result.floor_char_boundary(limit));
    }
    result
}

/// No file IO, lock wait, retry, runtime job or dependency on a live sink.
pub fn report(failure: Failure) -> bool {
    DIRECTORY.get().is_some_and(|root| report_to(root, failure))
}
/// Send to an explicit run without filesystem writes, waiting, retry or sink startup.
/// Returns false when the receiver is absent/full or the platform has no transport.
pub fn report_to(root: &Path, failure: Failure) -> bool {
    let Ok(bytes) = serde_json::to_vec(&failure.value()) else {
        return false;
    };
    if bytes.len() > MAX_RECORD {
        return false;
    }
    #[cfg(unix)]
    {
        let Ok(sender) = std::os::unix::net::UnixDatagram::unbound() else {
            return false;
        };
        sender.set_nonblocking(true).is_ok() && sender.send_to(&bytes, root.join(SOCKET)).is_ok()
    }
    #[cfg(not(unix))]
    {
        let _ = (root, bytes);
        false
    }
}

#[allow(
    clippy::disallowed_methods,
    reason = "private file helper called only by the sink or off-UI reader"
)]
fn private(root: &Path, name: &str, create: bool, append: bool) -> io::Result<File> {
    #[cfg(unix)]
    use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
    let mut opts = OpenOptions::new();
    opts.read(true).write(create).create(create).append(append);
    #[cfg(unix)]
    opts.mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC);
    let file = opts.open(root.join(name))?;
    let m = file.metadata()?;
    if !m.is_file() {
        return Err(io::Error::other("diagnostics source is not a regular file"));
    }
    #[cfg(unix)]
    {
        // SAFETY: geteuid takes no pointer and mutates no process state.
        if m.uid() != unsafe { libc::geteuid() } || m.mode() & 0o077 != 0 || m.nlink() != 1 {
            return Err(io::Error::other(
                "diagnostics source must be owned and private",
            ));
        }
    }
    Ok(file)
}
struct Lock(File);
impl Drop for Lock {
    fn drop(&mut self) {
        let _ = self.0.unlock();
    }
}
fn lock(root: &Path) -> io::Result<Lock> {
    let file = private(root, ".diagnostics-lock", true, false)?;
    file.try_lock().map_err(io::Error::other)?;
    Ok(Lock(file))
}
#[allow(
    clippy::disallowed_methods,
    reason = "rotation belongs exclusively to the owned sink thread"
)]
fn append(root: &Path, bytes: &[u8], max: u64) -> io::Result<()> {
    if bytes.len() + 1 > MAX_RECORD || bytes.len() as u64 + 1 > max {
        return Err(io::Error::other("diagnostic record exceeds bound"));
    }
    let _lock = lock(root)?;
    let mut file = private(root, ACTIVE, true, true)?;
    if file.metadata()?.len() + bytes.len() as u64 + 1 > max {
        for n in (1..=ARCHIVES).rev() {
            let from = if n == 1 {
                ACTIVE.into()
            } else {
                format!("diagnostics.{}.jsonl", n - 1)
            };
            let to = format!("diagnostics.{n}.jsonl");
            if let Ok(meta) = std::fs::symlink_metadata(root.join(&from)) {
                if !meta.is_file() || meta.file_type().is_symlink() {
                    return Err(io::Error::other("invalid diagnostic archive"));
                }
                std::fs::rename(root.join(from), root.join(to))?;
            }
        }
        file = private(root, ACTIVE, true, true)?;
    }
    let mut line = Vec::with_capacity(bytes.len() + 1);
    line.extend_from_slice(bytes);
    line.push(b'\n');
    file.write_all(&line)
}

fn normalized(row: Value) -> Option<Value> {
    if row["version"] != 1 {
        return None;
    }
    let id = row["id"].as_str()?;
    if id.len() > 96
        || id.split('-').count() != 3
        || id
            .split('-')
            .any(|p| p.is_empty() || !p.bytes().all(|b| b.is_ascii_digit()))
    {
        return None;
    }
    let optional = |key: &str| match &row[key] {
        Value::Null => Some(Value::Null),
        Value::String(text) => Some(scrub(text, 64).into()),
        _ => None,
    };
    let exit = match &row["exit_code"] {
        Value::Null => None,
        _ => Some(i32::try_from(row["exit_code"].as_i64()?).ok()?),
    };
    let duration = match &row["duration_ms"] {
        Value::Null => None,
        _ => Some(row["duration_ms"].as_u64()?),
    };
    Some(
        json!({"version":1,"id":id,"at_ms":row["at_ms"].as_u64()?,"pid":u32::try_from(row["pid"].as_u64()?).ok()?,
        "source":scrub(row["source"].as_str()?,64),"operation":scrub(row["operation"].as_str()?,128),
        "message":scrub(row["message"].as_str()?,1024),"stderr":scrub(row["stderr"].as_str().unwrap_or(""),MAX_STDERR),
        "seat":optional("seat")?,"harness":optional("harness")?,"exit_code":exit,"duration_ms":duration}),
    )
    .map(|mut out| {
        if let Some(n) = row["repeats"].as_u64().filter(|n| *n > 0) {
            out["repeats"] = n.into();
        }
        out
    })
}

/// Identical failures closer than this collapse into one later `repeats` row.
const REPEAT_GAP_MS: u64 = 60_000;
/// A continuing burst still records its count at least this often.
const REPEAT_SPAN_MS: u64 = 600_000;
const REPEAT_KEYS: usize = 256;
/// One hook process per failure means an outage would otherwise write a row
/// per call and bury distinct failures (#81). The first row is written at
/// once; later identical rows are held and the last is written with `repeats`,
/// the number of occurrences since the previous row, itself included.
#[derive(Default)]
struct Coalescer {
    pending: std::collections::HashMap<String, Burst>,
}
struct Burst {
    first_ms: u64,
    last_ms: u64,
    repeats: u64,
    last: Option<Value>,
}
impl Coalescer {
    fn key(row: &Value) -> String {
        ["source", "operation", "harness", "seat", "message"]
            .map(|k| row[k].to_string())
            .join("\u{1f}")
    }
    /// Rows to append for one admitted row, oldest first.
    fn admit(&mut self, row: Value, now_ms: u64) -> Vec<Value> {
        // Only per-process hook reports burst. Other sources stay one row per
        // failure: scrubbed messages can make distinct failures look identical.
        if row["source"] != "hook" {
            return vec![row];
        }
        let key = Self::key(&row);
        if let Some(burst) = self.pending.get_mut(&key)
            && now_ms.saturating_sub(burst.last_ms) < REPEAT_GAP_MS
            && now_ms.saturating_sub(burst.first_ms) < REPEAT_SPAN_MS
        {
            burst.last_ms = now_ms;
            burst.repeats += 1;
            burst.last = Some(row);
            return Vec::new();
        }
        let mut out: Vec<Value> = self
            .pending
            .remove(&key)
            .and_then(Burst::close)
            .into_iter()
            .collect();
        if self.pending.len() >= REPEAT_KEYS {
            out.extend(self.pending.drain().filter_map(|(_, b)| b.close()));
        }
        self.pending.insert(
            key,
            Burst {
                first_ms: now_ms,
                last_ms: now_ms,
                repeats: 0,
                last: None,
            },
        );
        out.push(row);
        out
    }
    /// Close bursts that went quiet or ran past their span; `None` closes all.
    fn expire(&mut self, now_ms: Option<u64>) -> Vec<Value> {
        let done: Vec<_> = self
            .pending
            .iter()
            .filter(|(_, b)| {
                now_ms.is_none_or(|now| {
                    now.saturating_sub(b.last_ms) >= REPEAT_GAP_MS
                        || now.saturating_sub(b.first_ms) >= REPEAT_SPAN_MS
                })
            })
            .map(|(k, _)| k.clone())
            .collect();
        let mut out: Vec<_> = done
            .into_iter()
            .filter_map(|k| self.pending.remove(&k)?.close())
            .collect();
        out.sort_by_key(|row| row["at_ms"].as_u64());
        out
    }
}
impl Burst {
    fn close(self) -> Option<Value> {
        let mut row = self.last?;
        row["repeats"] = self.repeats.into();
        Some(row)
    }
}

/// Bounded local file view, also usable when a backend has already stopped.
/// An unfiltered read acknowledges through the last returned row. Filtered
/// reads leave the global unread cursor unchanged.
///
/// # Errors
/// Returns file ownership/type/permission, cursor-write or underlying IO failures.
#[allow(
    clippy::disallowed_methods,
    reason = "bounded diagnostic IO reader asserts it is not the UI thread"
)]
pub fn read(
    root: &Path,
    since: Option<u64>,
    seat: Option<&str>,
    acknowledge: bool,
) -> io::Result<Value> {
    #[cfg(feature = "runtime")]
    crate::assert_not_ui_thread();
    let mut all = Vec::new();
    let mut malformed = 0;
    for name in (1..=ARCHIVES)
        .rev()
        .map(|n| format!("diagnostics.{n}.jsonl"))
        .chain([ACTIVE.into()])
    {
        let file = match private(root, &name, false, false) {
            Ok(file) => file,
            Err(e) if e.kind() == io::ErrorKind::NotFound => continue,
            Err(e) => return Err(e),
        };
        // Never allocate an unbounded line from a damaged/external file.
        let mut reader = BufReader::new(file).take(MAX_FILE + MAX_RECORD as u64);
        loop {
            let mut line = Vec::new();
            let n = (&mut reader)
                .take(MAX_RECORD as u64 + 1)
                .read_until(b'\n', &mut line)?;
            if n == 0 {
                break;
            }
            if line.last() != Some(&b'\n') || line.len() > MAX_RECORD {
                malformed += 1;
                break;
            }
            match serde_json::from_slice::<Value>(&line)
                .ok()
                .and_then(normalized)
            {
                Some(row) => all.push(row),
                _ => malformed += 1,
            }
        }
    }
    let cursor = private(root, READ, false, false).ok().and_then(|mut f| {
        let mut text = String::new();
        Read::by_ref(&mut f)
            .take(128)
            .read_to_string(&mut text)
            .ok()?;
        Some(text)
    });
    let seen = cursor
        .as_ref()
        .and_then(|id| {
            all.iter()
                .position(|row| row["id"].as_str() == Some(id.as_str()))
        })
        .map_or(0, |n| n + 1);
    let unread = all.len().saturating_sub(seen);
    let selected: Vec<_> = all
        .iter()
        .filter(|row| {
            since.is_none_or(|t| row["at_ms"].as_u64().is_some_and(|n| n >= t))
                && seat.is_none_or(|seat| row["seat"].as_str() == Some(seat))
        })
        .collect();
    let total = selected.len();
    let mut bytes = 1024;
    let mut page = std::collections::VecDeque::new();
    for row in selected.into_iter().rev().take(MAX_ROWS) {
        let size = serde_json::to_vec(row)?.len() + 1;
        if bytes + size > MAX_PAGE {
            break;
        }
        bytes += size;
        page.push_front(row.clone());
    }
    let selected: Vec<_> = page.into();
    let omitted = total.saturating_sub(selected.len());
    if acknowledge
        && since.is_none()
        && seat.is_none()
        && let Some(id) = selected.last().and_then(|r| r["id"].as_str())
    {
        let temp = format!(
            ".diagnostics-read-{}-{}-{}",
            std::process::id(),
            now_ms(),
            SERIAL.fetch_add(1, Ordering::Relaxed)
        );
        let mut f = private(root, &temp, true, false)?;
        f.set_len(0)?;
        f.write_all(id.as_bytes())?;
        if let Err(error) = std::fs::rename(root.join(&temp), root.join(READ)) {
            let _ = std::fs::remove_file(root.join(temp));
            return Err(error);
        }
    }
    Ok(
        json!({"available":true,"rows":selected,"unread_before":unread,"unread":if acknowledge&&since.is_none()&&seat.is_none(){0}else{unread},"omitted":omitted,"malformed":malformed,
        "cursor_expired":cursor.is_some()&&seen==0,"snapshot":"best_effort across rotation; malformed or incomplete rows are not returned",
        "retention_bytes":MAX_FILE*(ARCHIVES as u64+1),"acknowledged":acknowledge&&since.is_none()&&seat.is_none()}),
    )
}

/// Owns the run's background diagnostics sink; dropping requests stop without joining it.
pub struct Server {
    stop: Arc<AtomicBool>,
    #[cfg(unix)]
    root: PathBuf,
    ready: std::sync::mpsc::Receiver<io::Result<()>>,
}
impl Server {
    /// Spawning is memory-only for the caller. Socket/filesystem startup is
    /// entirely on this owned thread; failure never prevents app startup.
    ///
    /// # Errors
    /// Returns an OS thread-spawn failure. Asynchronous setup errors arrive on `ready`.
    pub fn start(root: PathBuf, notify: impl Fn(u64) + Send + 'static) -> io::Result<Self> {
        let stop = Arc::new(AtomicBool::new(false));
        let stopping = stop.clone();
        let (ready_tx, ready) = std::sync::mpsc::sync_channel(1);
        #[cfg(unix)]
        let path = root.clone();
        std::thread::Builder::new()
            .name("bbt-error-journal".into())
            .stack_size(256 * 1024)
            .spawn(move || {
                #[cfg(unix)]
                {
                    let outcome = serve(&root, &stopping, notify, &ready_tx);
                    if let Err(error) = outcome {
                        let _ = ready_tx.try_send(Err(error));
                    }
                }
                #[cfg(not(unix))]
                {
                    let _ = (root, stopping, notify);
                    let _ = ready_tx
                        .try_send(Err(io::Error::other("diagnostics transport unavailable")));
                }
            })?;
        Ok(Self {
            stop,
            #[cfg(unix)]
            root: path,
            ready,
        })
    }
    /// Setup observation for IO/tests. UI callers must never wait on this receiver.
    pub fn ready(&self) -> &std::sync::mpsc::Receiver<io::Result<()>> {
        &self.ready
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        #[cfg(unix)]
        if let Ok(s) = std::os::unix::net::UnixDatagram::unbound() {
            let _ = s.set_nonblocking(true);
            let _ = s.send_to(&[], self.root.join(SOCKET));
        }
    }
}
#[cfg(unix)]
#[allow(
    clippy::disallowed_methods,
    reason = "owned diagnostics sink performs its filesystem work on its background thread"
)]
fn serve(
    root: &Path,
    stop: &AtomicBool,
    notify: impl Fn(u64),
    ready: &std::sync::mpsc::SyncSender<io::Result<()>>,
) -> io::Result<()> {
    use std::os::unix::fs::{DirBuilderExt, FileTypeExt, MetadataExt, PermissionsExt};
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(root)?;
    let directory = std::fs::symlink_metadata(root)?;
    // SAFETY: geteuid takes no arguments and mutates no process state.
    let uid = unsafe { libc::geteuid() };
    if !directory.is_dir() || directory.mode() & 0o077 != 0 || directory.uid() != uid {
        return Err(io::Error::other(
            "diagnostics run directory must be owned and private",
        ));
    }
    let lease = private(root, ".diagnostics-owner", true, false)?;
    lease.try_lock().map_err(io::Error::other)?;
    let _lease = Lock(lease);
    let socket = root.join(SOCKET);
    if let Ok(m) = std::fs::symlink_metadata(&socket) {
        if !m.file_type().is_socket() {
            return Err(io::Error::other("invalid diagnostics socket"));
        }
        std::fs::remove_file(&socket)?;
    }
    let receiver = std::os::unix::net::UnixDatagram::bind(&socket)?;
    std::fs::set_permissions(&socket, std::fs::Permissions::from_mode(0o600))?;
    receiver.set_read_timeout(Some(Duration::from_secs(1)))?;
    let _ = ready.try_send(Ok(()));
    // Zero is already the UI's initial value. Never wake it for an empty log.
    let mut prior = Some(0);
    let mut next_count = std::time::Instant::now();
    let mut bytes = [0u8; MAX_RECORD + 1];
    let mut coalescer = Coalescer::default();
    let write = |rows: Vec<Value>| {
        for row in rows {
            if let Ok(data) = serde_json::to_vec(&row) {
                let _ = append(root, &data, MAX_FILE);
            }
        }
    };
    while !stop.load(Ordering::Relaxed) {
        match receiver.recv(&mut bytes) {
            Ok(n) if n > 0 && n <= MAX_RECORD => {
                if let Some(row) = serde_json::from_slice::<Value>(&bytes[..n])
                    .ok()
                    .and_then(normalized)
                {
                    write(coalescer.admit(row, now_ms()));
                }
            }
            Ok(_) => {}
            Err(e)
                if [
                    io::ErrorKind::WouldBlock,
                    io::ErrorKind::TimedOut,
                    io::ErrorKind::Interrupted,
                ]
                .contains(&e.kind()) => {}
            Err(e) => {
                write(coalescer.expire(None));
                return Err(e);
            }
        }
        write(coalescer.expire(Some(now_ms())));
        // Accepted rows still append individually. Bound rescans and UI count
        // updates during an error burst; idle polls cause no UI callback.
        if std::time::Instant::now() < next_count {
            continue;
        }
        next_count = std::time::Instant::now() + Duration::from_millis(250);
        if let Ok(data) = read(root, None, None, false) {
            let n = data["unread"].as_u64().unwrap_or(0);
            if prior != Some(n) {
                prior = Some(n);
                notify(n);
            }
        }
    }
    write(coalescer.expire(None));
    let _ = std::fs::remove_file(socket);
    Ok(())
}

#[cfg(all(test, unix))]
#[allow(
    clippy::disallowed_methods,
    reason = "private IO fixtures and bounded synchronization, never UI code"
)]
mod tests {
    use super::*;
    use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
    struct Root(PathBuf);
    impl Root {
        fn new() -> Self {
            let p = std::env::temp_dir().join(format!(
                "bbt-errors-{}-{}-{}",
                std::process::id(),
                now_ms(),
                SERIAL.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::DirBuilder::new().mode(0o700).create(&p).unwrap();
            Self(p)
        }
    }
    impl Drop for Root {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    // keeps: operator error journal; secret/env values never reach disk or readers
    #[test]
    fn diagnostic_redaction_tail_and_file_boundary_remove_secret_values() {
        let raw = format!(
            "old-prefix{}\nCODEX_THREAD_ID=parent-private\nAuthorization: Bearer topsecret\n{{\"MY_ENV\":\"hidden\"}}\nlast failure",
            "x".repeat(5000)
        );
        let value = Failure::new("hook", "SessionStart", "failed")
            .harness("codex")
            .seat("01M5WZKA3MZC8BRJ601TFF7FH1")
            .exit(Some(1))
            .duration(Duration::from_millis(3))
            .stderr(raw.as_bytes())
            .value();
        let text = value.to_string();
        for secret in ["old-prefix", "parent-private", "topsecret", "hidden"] {
            assert!(!text.contains(secret), "leaked {secret}");
        }
        assert!(text.contains("last failure"));
        assert_eq!(value["exit_code"], 1);
        assert_eq!(value["duration_ms"], 3);
        let mut forged = value;
        forged["stderr"] = "password=unsafe".into();
        forged["env"] = json!({"PRIVATE":"unsafe"});
        let normalized = normalized(forged).unwrap().to_string();
        assert!(!normalized.contains("unsafe"));
        assert!(!normalized.contains("PRIVATE"));
        assert!(!scrub("opaque aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", 100).contains("aaaa"));
    }

    // keeps: bounded diagnostics retention; filters never acknowledge other seats
    #[test]
    fn diagnostic_rotation_filters_and_acknowledgement_keep_complete_records() {
        let root = Root::new();
        for i in 1..=20 {
            let mut value = Failure::new("ctl", "screen", format!("failure {i}"))
                .seat(if i % 2 == 0 { "alpha" } else { "beta" })
                .value();
            value["at_ms"] = i.into();
            let bytes = serde_json::to_vec(&value).unwrap();
            append(&root.0, &bytes, 768).unwrap();
        }
        for name in [ACTIVE, "diagnostics.1.jsonl", "diagnostics.2.jsonl"] {
            let metadata = std::fs::metadata(root.0.join(name)).unwrap();
            assert!(metadata.len() <= 768);
            assert_eq!(metadata.permissions().mode() & 0o777, 0o600);
        }
        let before = read(&root.0, None, None, false).unwrap();
        assert!(before["unread"].as_u64().unwrap() > 0);
        assert_eq!(
            before["rows"].as_array().unwrap().last().unwrap()["at_ms"],
            20
        );
        let filtered = read(&root.0, Some(19), Some("alpha"), true).unwrap();
        assert!(!filtered["acknowledged"].as_bool().unwrap());
        assert!(
            filtered["rows"]
                .as_array()
                .unwrap()
                .iter()
                .all(|r| r["seat"] == "alpha" && r["at_ms"].as_u64().unwrap() >= 19)
        );
        assert_eq!(
            read(&root.0, None, None, false).unwrap()["unread"],
            before["unread"]
        );
        read(&root.0, None, None, true).unwrap();
        assert_eq!(read(&root.0, None, None, false).unwrap()["unread"], 0);
    }

    // keeps: diagnostics are best effort; a full/missing sink never waits on IO
    #[test]
    fn diagnostic_read_pages_keep_newest_rows_inside_the_wire_byte_bound() {
        let root = Root::new();
        for _ in 0..80 {
            let row = Failure::new("provider", "read", "failed")
                .stderr("io error ".repeat(450).as_bytes())
                .value();
            append(&root.0, &serde_json::to_vec(&row).unwrap(), MAX_FILE).unwrap();
        }
        let page = read(&root.0, None, None, false).unwrap();
        assert!(page["omitted"].as_u64().unwrap() > 0);
        assert!(!page["rows"].as_array().unwrap().is_empty());
        assert!(serde_json::to_vec(&page).unwrap().len() <= MAX_PAGE);
        let read = read(&root.0, None, None, true).unwrap();
        assert_eq!(read["unread"], 0);
    }

    // keeps: diagnostics are best effort; a full/missing sink never waits on IO
    #[test]
    fn diagnostic_sender_reports_a_missing_or_full_sink_without_waiting_for_a_reader() {
        let root = Root::new();
        assert!(!report_to(
            &root.0,
            Failure::new("hook", "SessionStart", "missing")
        ));
        let _receiver = std::os::unix::net::UnixDatagram::bind(root.0.join(SOCKET)).unwrap();
        let mut full = false;
        for _ in 0..4096 {
            if !report_to(&root.0, Failure::new("hook", "SessionStart", "full")) {
                full = true;
                break;
            }
        }
        assert!(full, "bounded kernel datagram queue did not fill");
        assert!(!report_to(
            &root.0,
            Failure::new("hook", "SessionStart", "still full")
        ));
        assert!(!root.0.join(ACTIVE).exists());
    }

    // keeps: nonblocking cross-process-compatible sink; unread count follows durable rows
    #[test]
    fn diagnostic_sink_captures_failure_fields_and_notifies_after_read_acknowledgement() {
        let root = Root::new();
        let (tx, rx) = std::sync::mpsc::channel();
        let server = Server::start(root.0.clone(), move |n| {
            let _ = tx.send(n);
        })
        .unwrap();
        server
            .ready()
            .recv_timeout(Duration::from_secs(60))
            .unwrap()
            .unwrap();
        assert!(rx.try_recv().is_err(), "empty journal woke its UI consumer");
        assert!(report_to(
            &root.0,
            Failure::new("hook", "UserPromptSubmit", "nonzero exit")
                .seat("alpha")
                .harness("codex")
                .exit(Some(1))
                .stderr(b"hook stderr")
        ));
        while rx.recv_timeout(Duration::from_secs(60)).unwrap() != 1 {}
        let data = read(&root.0, None, None, true).unwrap();
        let rows = data["rows"].as_array().unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["operation"], "UserPromptSubmit");
        assert_eq!(rows[0]["seat"], "alpha");
        assert_eq!(rows[0]["harness"], "codex");
        assert_eq!(rows[0]["exit_code"], 1);
        assert_eq!(rows[0]["stderr"], "hook stderr");
        while rx.recv_timeout(Duration::from_secs(60)).unwrap() != 0 {}
        drop(server);
    }

    // keeps: an outage of identical hook failures costs one row plus one count (#81)
    #[test]
    fn diagnostic_identical_failures_coalesce_into_a_repeat_count() {
        let row = |message: &str, seat: &str| {
            normalized(
                Failure::new("hook", "bboxy_shim", message)
                    .seat(seat)
                    .harness("claude")
                    .value(),
            )
            .unwrap()
        };
        let mut c = Coalescer::default();
        let start = 1_000_000;
        // 104 refusals over ~11 minutes, as observed on 2026-10-30.
        let mut written = Vec::new();
        for i in 0..104 {
            written.extend(c.admit(
                row("Connection refused (os error 61)", "a"),
                start + i * 6_000,
            ));
        }
        // Another seat and another message are never merged into that burst.
        written.extend(c.admit(row("Connection refused (os error 61)", "b"), start));
        written.extend(c.admit(row("hook input bound", "a"), start));
        written.extend(c.expire(Some(start + 103 * 6_000 + REPEAT_GAP_MS)));
        let counts: Vec<_> = written
            .iter()
            .map(|r| {
                (
                    r["seat"].as_str().unwrap().to_owned(),
                    r["message"].as_str().unwrap().to_owned(),
                    r["repeats"].as_u64(),
                )
            })
            .collect();
        let a = "Connection refused (os error 61)".to_owned();
        // A closing row is the burst's last occurrence; `repeats` counts every
        // occurrence since the previous row, itself included.
        let burst: Vec<_> = counts.iter().filter(|c| c.0 == "a" && c.1 == a).collect();
        assert_eq!(
            burst.len(),
            4,
            "the 10-minute span splits the burst once: {counts:?}"
        );
        let plain = burst.iter().filter(|c| c.2.is_none()).count() as u64;
        let repeats: u64 = burst.iter().filter_map(|c| c.2).sum();
        assert_eq!(
            plain + repeats,
            104,
            "every refusal is written or counted: {counts:?}"
        );
        assert!(counts.contains(&("b".into(), a, None)));
        assert!(counts.contains(&("a".into(), "hook input bound".into(), None)));
        // Distinct operator failures can scrub to the same text; never merge them.
        let ctl = || normalized(Failure::new("ctl", "parse", "[redacted]").value()).unwrap();
        assert_eq!(c.admit(ctl(), start).len() + c.admit(ctl(), start).len(), 2);
        // Rows survive the reader's validation with their count.
        assert!(
            written
                .iter()
                .all(|r| normalized(r.clone()).as_ref() == Some(r))
        );
        assert!(c.expire(None).is_empty());
    }

    // keeps: the journal cannot follow a run-file symlink into credentials
    #[test]
    fn diagnostic_reader_and_writer_refuse_symlink_sources() {
        let root = Root::new();
        let secret = root.0.join("private-data");
        std::fs::write(&secret, "do not read").unwrap();
        std::os::unix::fs::symlink(&secret, root.0.join(ACTIVE)).unwrap();
        assert!(append(&root.0, b"{}", MAX_FILE).is_err());
        assert!(read(&root.0, None, None, false).is_err());
        assert_eq!(std::fs::read_to_string(secret).unwrap(), "do not read");
    }
}
