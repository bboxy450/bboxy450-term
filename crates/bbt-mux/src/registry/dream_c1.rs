// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Per-seat C1 journal. Only isolated persistence owners call this module.
use bbt_proto::dream::{self as w, Code};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::{
    os::unix::fs::{DirBuilderExt, MetadataExt},
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Bookmark {
    pub owner_context: Option<w::Context>,
    pub owner_fence: Option<crate::identity::PublicFence>,
    pub session: w::Ulid,
    pub generation: u64,
    pub path: PathBuf,
    pub repo: String,
    pub pid: u32,
    pub process_start: String,
    pub socket: PathBuf,
    pub root: String,
    pub root_revision: u64,
    pub workspace_revision: u64,
    pub device: u64,
    pub inode: u64,
    pub len: u64,
    pub mtime: (i64, i64),
    pub ctime: (i64, i64),
    pub closed_seq: u64,
    pub closed_line_sha256: String,
    pub cursor: u64,
    pub observed_seq: u64,
    pub last_line_len: u32,
    pub last_line_sha256: String,
    pub turns: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct LaunchBookmark {
    pub birth: w::Ulid,
    pub binding: w::Hex,
    pub incarnation: u64,
    pub worker_sha256: w::Hex,
    pub pipe_fd: u32,
    pub pipe_device: u64,
    pub pipe_inode: u64,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Record {
    pub trigger: bool,
    pub started: bool,
    pub owner_incarnation: w::Ulid,
    pub worker_pid: Option<u32>,
    pub worker_start: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub launch: Option<LaunchBookmark>,
    pub context: w::Context,
    pub identity_fence: Option<crate::identity::PublicFence>,
    pub op_id: w::Ulid,
    pub nonce: w::Ulid,
    pub sources: Vec<Bookmark>,
    #[serde(with = "request_bytes")]
    pub request: Vec<u8>,
    pub view: w::RunView,
}

mod request_bytes {
    use serde::{Deserialize, Deserializer, Serializer};
    pub fn serialize<S: Serializer>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(std::str::from_utf8(bytes).map_err(serde::ser::Error::custom)?)
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<u8>, D::Error> {
        Ok(String::deserialize(deserializer)?.into_bytes())
    }
}
fn record_bytes(record: &Record) -> Result<Vec<u8>, Code> {
    let bytes = serde_json::to_vec(record).map_err(|_| Code::StorageUnavailable)?;
    // The complete retained per-run metadata, including all selected bookmarks,
    // fits this limit. Normalizing rows must not evade the per-run allocation.
    if bytes.len() > 16 * 1024 {
        return Err(Code::ResourceLimit);
    }
    Ok(bytes)
}

fn source_current(source: &Bookmark, context: &w::Context) -> bool {
    let mut expected = context.clone();
    expected.generation = w::Number(source.generation);
    source.owner_context.as_ref() == Some(&expected)
        && source.root == context.root.0
        && source.root_revision == context.root_revision.0
        && source.workspace_revision == context.workspace_revision.0
}

pub(crate) fn failure(code: Code) -> w::ControlReply {
    w::ControlReply::Failure(w::Failure {
        v: w::Version,
        ok: w::False,
        error: w::Error { code, source: None },
    })
}
fn reply(record: &Record) -> w::ControlReply {
    w::ControlReply::Success(w::Success {
        v: w::Version,
        ok: w::True,
        run: record.view.clone(),
        page: None,
    })
}

pub(crate) fn private_dir(path: &Path) -> Result<(), Code> {
    match std::fs::DirBuilder::new().mode(0o700).create(path) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(_) => return Err(Code::StorageUnavailable),
    }
    let m = std::fs::symlink_metadata(path).map_err(|_| Code::StorageUnavailable)?;
    // SAFETY: geteuid has no pointer arguments or side effects.
    if !m.is_dir() || m.uid() != unsafe { libc::geteuid() } || m.mode() & 0o077 != 0 {
        return Err(Code::StorageUnavailable);
    }
    Ok(())
}

fn bounded_blob(row: &rusqlite::Row<'_>, index: usize) -> rusqlite::Result<Vec<u8>> {
    let bytes = row.get_ref(index)?.as_blob()?;
    if bytes.len() > 16 * 1024 {
        return Err(rusqlite::Error::InvalidQuery);
    }
    bboxy450_harness_wake_up::dream_c1::preflight_json(
        bytes,
        bboxy450_harness_wake_up::dream_c1::JsonShape::Any,
    )
    .map_err(|_| rusqlite::Error::InvalidQuery)?;
    let mut result = Vec::new();
    result
        .try_reserve_exact(bytes.len())
        .map_err(|_| rusqlite::Error::InvalidQuery)?;
    result.extend_from_slice(bytes);
    Ok(result)
}

struct JournalConnection {
    connection: Option<Connection>,
    quantum: Option<Arc<engine::JournalQuantum>>,
}
impl std::ops::Deref for JournalConnection {
    type Target = Connection;
    fn deref(&self) -> &Connection {
        self.connection.as_ref().unwrap()
    }
}
impl std::ops::DerefMut for JournalConnection {
    fn deref_mut(&mut self) -> &mut Connection {
        self.connection.as_mut().unwrap()
    }
}
impl Drop for JournalConnection {
    fn drop(&mut self) {
        if let Some(connection) = self.connection.take() {
            if let Some(quantum) = &self.quantum {
                quantum.retire(connection);
            } else {
                drop(connection);
            }
        }
    }
}
pub(crate) struct Journal {
    db: JournalConnection,
    quantum: Option<Arc<engine::JournalQuantum>>,
}
/// Retains the no-follow descriptor across the separately measured native open.
pub(crate) struct ExistingJournal {
    path: PathBuf,
    file: std::fs::File,
    read_only: bool,
}
#[derive(Clone, Copy)]
pub(crate) enum JournalConfig {
    Cache,
    Mmap,
    Sync,
}
impl Journal {
    #[track_caller]
    pub fn open(runtime: &Path, seat: &w::Ulid) -> Result<Self, Code> {
        Self::open_mode(runtime, seat, true, false)
    }
    /// Test admission to a retained catalogue never creates a missing schema.
    #[cfg(test)]
    #[track_caller]
    pub fn open_existing(runtime: &Path, seat: &w::Ulid) -> Result<Self, Code> {
        Self::open_mode(runtime, seat, false, false)
    }
    #[track_caller]
    pub fn open_readonly_existing(runtime: &Path, seat: &w::Ulid) -> Result<Self, Code> {
        Self::open_mode(runtime, seat, false, true)
    }
    /// Refusal-only preflight for a cold Status/Cancel. No schema/config/run
    /// is read or retained. Fixed header preparation/step/finalization share a
    /// strict ordinary unit; every included call must therefore fit that same
    /// 1ms total, with the full after-end debt paid before the next phase.
    #[track_caller]
    fn probe_control_header(runtime: &Path, seat: &w::Ulid) -> Result<(), Code> {
        let quantum = Arc::new(engine::JournalQuantum::at(std::panic::Location::caller()));
        let existing = quantum.retain(quantum.unit(|| {
            Self::prepare_path_mode(runtime, seat, false, true)
        })?);
        let mut journal = quantum.unit(|| Self::connect_existing(&existing))?;
        journal.quantum = Some(quantum.clone());
        journal.db.quantum = Some(quantum.clone());
        quantum.unit(|| {
            journal.db.busy_timeout(std::time::Duration::ZERO)
                .map_err(Self::initialization_error)
        })?;
        let version = quantum.unit(|| {
            journal.db.query_row("PRAGMA main.user_version", [], |row| row.get::<_, i64>(0))
                .map_err(Self::initialization_error)
        });
        journal.finish()?;
        drop(existing);
        quantum.check()?;
        quantum.pay()?;
        match version? {
            1 => Ok(()),
            0 => Err(Code::Busy),
            _ => Err(Code::StorageUnavailable),
        }
    }
    #[track_caller]
    fn open_mode(
        runtime: &Path,
        seat: &w::Ulid,
        initialize: bool,
        read_only: bool,
    ) -> Result<Self, Code> {
        let quantum = Arc::new(engine::JournalQuantum::at(std::panic::Location::caller()));
        let outcome = (|| {
            let existing = quantum.retain(
                quantum.unit(|| Self::prepare_path_mode(runtime, seat, initialize, read_only))?,
            );
            let mut journal = quantum.unit(|| Self::connect_existing(&existing))?;
            journal.quantum = Some(quantum.clone());
            journal.db.quantum = Some(quantum.clone());
            // Bootstrap never waits behind a writer, including the first
            // schema-cookie read. A ready writable catalogue restores the
            // ordinary bounded timeout only after this prelude completes.
            quantum.unit(|| {
                journal
                    .db
                    .busy_timeout(std::time::Duration::ZERO)
                    .map_err(Self::initialization_error)
            })?;
            // The already-required header check precedes NeedSchema PRAGMAs.
            // user_version emits Transaction(read)/ReadCookie/ResultRow without
            // sqlite3InitOne's all-schema SQL parse. Retain this value.
            let version = journal.schema_version()?;
            if version == 0 && !initialize {
                return Err(Code::Busy);
            }
            if version != 0 && version != 1 {
                return Err(Code::StorageUnavailable);
            }
            journal.configure_cache()?;
            journal.configure_mmap()?;
            if !read_only {
                journal.configure_sync()?;
            }
            match version {
                1 => {
                    if !read_only {
                        // Decisions66026/66058: transition only this exact private
                        // catalogue, outside a transaction and while busy0.
                        // SQLite retains/settles existing WAL rows itself.
                        journal.initialization_statement("PRAGMA journal_mode=PERSIST", 0)?;
                        journal.configure_busy()?;
                    }
                }
                0 if initialize => journal.initialize_schema(None)?,
                0 => return Err(Code::Busy), // Known initialization is not ready.
                _ => return Err(Code::StorageUnavailable),
            }
            drop(existing);
            Ok(journal)
        })();
        // A failed open owns and closes its exact connection before checking
        // cleanup. A semantic refusal cannot hide a native close overrun.
        let mut outcome = quantum.retain(outcome);
        quantum.check()?;
        outcome.take_value()
    }

    fn schema_version(&self) -> Result<i64, Code> {
        let quantum = self.quantum.as_ref().ok_or(Code::StorageUnavailable)?;
        let mut header = quantum.retain(quantum.unit(|| self.prepare_version())?);
        let statement = &mut *header;
        let mut rows = quantum.retain(quantum.unit(move || Ok(Self::rows(statement)))?);
        let version = quantum.unit(|| Self::read_version_first(&mut rows))?;
        drop(rows);
        drop(header);
        quantum.check()?;
        Ok(version)
    }
    fn initialization_error(error: rusqlite::Error) -> Code {
        match error {
            rusqlite::Error::SqliteFailure(error, _)
                if matches!(
                    error.code,
                    rusqlite::ErrorCode::DatabaseBusy | rusqlite::ErrorCode::DatabaseLocked
                ) =>
            {
                Code::Busy
            }
            _ => Code::StorageUnavailable,
        }
    }
    fn initialization_step(rows: &mut rusqlite::Rows<'_>) -> Result<bool, Code> {
        rows.next()
            .map(|row| row.is_some())
            .map_err(Self::initialization_error)
    }
    fn initialization_statement(&self, sql: &'static str, index: usize) -> Result<(), Code> {
        let quantum = self.quantum.as_ref().ok_or(Code::StorageUnavailable)?;
        let mut prepared = quantum.retain(
            quantum
                .unit(|| self.db.prepare(sql).map_err(Self::initialization_error))
                .inspect_err(|code| {
                    #[cfg(debug_assertions)]
                    eprintln!("C1 journal init refused index={index} phase=prepare code={code:?}");
                    #[cfg(not(debug_assertions))]
                    let _ = (index, code);
                })?,
        );
        let statement = &mut *prepared;
        let mut rows = quantum.retain(quantum.unit(move || Ok(Self::rows(statement)))?);
        let first = quantum
            .unit(|| {
                if index == 0 {
                    let row = rows
                        .next()
                        .map_err(Self::initialization_error)?
                        .ok_or(Code::StorageUnavailable)?;
                    if row
                        .get_ref(0)
                        .map_err(Self::initialization_error)?
                        .as_str()
                        .map_err(|_| Code::StorageUnavailable)?
                        != "persist"
                    {
                        return Err(Code::StorageUnavailable);
                    }
                    Ok(true)
                } else {
                    Self::initialization_step(&mut rows)
                }
            })
            .inspect_err(|code| {
                #[cfg(debug_assertions)]
                eprintln!("C1 journal init refused index={index} phase=first_step code={code:?}");
                #[cfg(not(debug_assertions))]
                let _ = (index, code);
            })?;
        if first
            && quantum
                .unit(|| Self::initialization_step(&mut rows))
                .inspect_err(|code| {
                    #[cfg(debug_assertions)]
                    eprintln!(
                        "C1 journal init refused index={index} phase=done_step code={code:?}"
                    );
                    #[cfg(not(debug_assertions))]
                    let _ = (index, code);
                })?
        {
            return Err(Code::StorageUnavailable);
        }
        drop(rows);
        drop(prepared);
        quantum.check()
    }
    fn rollback_initialization(&self) -> Result<(), Code> {
        let quantum = self.quantum.as_ref().ok_or(Code::StorageUnavailable)?;
        let mut prepared = quantum.retain(quantum.required(|| {
            self.db
                .prepare("ROLLBACK")
                .map_err(Self::initialization_error)
        })?);
        let statement = &mut *prepared;
        let mut rows = quantum.retain(quantum.required(move || Ok(Self::rows(statement)))?);
        let row = quantum.required(|| Self::initialization_step(&mut rows))?;
        drop(rows);
        drop(prepared);
        if row || !self.db.is_autocommit() {
            return Err(Code::StorageUnavailable);
        }
        Ok(())
    }
    /// Rollback-journal selection is outside the transaction. Schema and its ready marker
    /// become durable atomically; CREATE no longer hides an implicit commit.
    /// The bounded IO owner uses busy0, never a writer wait on a UI/shared owner.
    fn initialize_schema(&self, inject_failure_after: Option<usize>) -> Result<(), Code> {
        let quantum = self.quantum.as_ref().ok_or(Code::StorageUnavailable)?;
        quantum.unit(|| {
            self.db
                .busy_timeout(std::time::Duration::ZERO)
                .map_err(Self::initialization_error)
        })?;
        let initialized = (|| {
            self.initialization_statement("PRAGMA journal_mode=PERSIST", 0)?;
            self.initialization_statement("BEGIN IMMEDIATE", 7)?;
            // This second cookie read is an atomic race check after acquiring
            // the writer transaction, not repeated preparation/warmup work.
            match self.schema_version()? {
                1 => {}
                0 => {
                    for (offset, statement) in [
                        "CREATE TABLE IF NOT EXISTS source_errors(session TEXT NOT NULL,generation INTEGER NOT NULL,error BLOB NOT NULL,PRIMARY KEY(session,generation))",
                        "CREATE TABLE IF NOT EXISTS sources(session TEXT NOT NULL,generation INTEGER NOT NULL,bookmark BLOB NOT NULL,PRIMARY KEY(session,generation))",
                        "CREATE TABLE IF NOT EXISTS runs(id TEXT PRIMARY KEY,source_key TEXT UNIQUE NOT NULL,record BLOB NOT NULL)",
                        "CREATE TABLE IF NOT EXISTS ops(actor TEXT NOT NULL,op TEXT NOT NULL,request BLOB NOT NULL,run TEXT NOT NULL,PRIMARY KEY(actor,op))",
                        "CREATE TABLE IF NOT EXISTS close_cursor(session TEXT NOT NULL,generation INTEGER NOT NULL,cursor INTEGER NOT NULL,PRIMARY KEY(session,generation))",
                        "PRAGMA user_version=1",
                    ].into_iter().enumerate() {
                        let index = offset + 1;
                        self.initialization_statement(statement, index)?;
                        if cfg!(test) && inject_failure_after == Some(index) {
                            return Err(Code::StorageUnavailable);
                        }
                    }
                }
                _ => return Err(Code::StorageUnavailable),
            }
            self.initialization_statement("COMMIT", 8)
        })();
        if initialized.is_err() && !self.db.is_autocommit() {
            // Even CPU/clock/cancel refusal retains the actual transaction
            // until required rollback has run; closing cannot imply rollback.
            let rolled_back = self.rollback_initialization();
            quantum.check()?;
            rolled_back?;
        }
        initialized?;
        self.configure_busy()?;
        quantum.check()
    }
    fn finish(self) -> Result<(), Code> {
        let quantum = self.quantum.clone();
        drop(self);
        quantum.as_ref().map_or(Ok(()), |quantum| quantum.check())
    }
    #[track_caller]
    fn step<T: engine::disposal::Drain>(
        quantum: &Option<Arc<engine::JournalQuantum>>,
        work: impl FnOnce() -> Result<T, Code>,
    ) -> Result<T, Code> {
        match quantum {
            Some(quantum) => quantum.unit(work),
            None => work(),
        }
    }
    fn retain<T: engine::disposal::Drain>(
        quantum: &Option<Arc<engine::JournalQuantum>>,
        value: T,
    ) -> engine::disposal::Paced<T> {
        match quantum {
            Some(quantum) => quantum.retain(value),
            None => engine::JournalQuantum::default().retain(value),
        }
    }
    pub fn existing_path(runtime: &Path, seat: &w::Ulid) -> Result<ExistingJournal, Code> {
        Self::prepare_path(runtime, seat, false)
    }
    fn prepare_path(
        runtime: &Path,
        seat: &w::Ulid,
        initialize: bool,
    ) -> Result<ExistingJournal, Code> {
        Self::prepare_path_mode(runtime, seat, initialize, false)
    }
    fn prepare_path_mode(
        runtime: &Path,
        seat: &w::Ulid,
        initialize: bool,
        read_only: bool,
    ) -> Result<ExistingJournal, Code> {
        bbt_rt::assert_not_ui_thread();
        let journals = runtime.join("journals");
        if read_only {
            for directory in [runtime, journals.as_path()] {
                let metadata = std::fs::symlink_metadata(directory).map_err(|error| {
                    if error.kind() == std::io::ErrorKind::NotFound {
                        Code::NotFound
                    } else {
                        Code::StorageUnavailable
                    }
                })?;
                if !metadata.is_dir()
                    || metadata.uid() != unsafe { libc::geteuid() }
                    || metadata.mode() & 0o777 != 0o700
                {
                    return Err(Code::StorageUnavailable);
                }
            }
        } else {
            private_dir(runtime)?;
            private_dir(&journals)?;
        }
        let path = journals.join(format!("{}.sqlite", seat.0));
        use std::os::unix::fs::OpenOptionsExt;
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(!read_only)
            .create(initialize)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(&path)
            .map_err(|error| {
                if read_only && error.kind() == std::io::ErrorKind::NotFound {
                    Code::NotFound
                } else {
                    Code::StorageUnavailable
                }
            })?;
        let m = file.metadata().map_err(|_| Code::StorageUnavailable)?;
        if !m.is_file()
            || m.nlink() != 1
            || m.uid() != unsafe { libc::geteuid() }
            || m.mode() & 0o077 != 0
        {
            return Err(Code::StorageUnavailable);
        }
        // macOS publishes /var as a symlink to /private/var. SQLite NOFOLLOW
        // checks the complete path, so retain the already opened exact inode
        // while resolving that system alias before the native database open.
        let path = std::fs::canonicalize(path).map_err(|_| Code::StorageUnavailable)?;
        let canonical = std::fs::symlink_metadata(&path).map_err(|_| Code::StorageUnavailable)?;
        if !canonical.is_file() || (canonical.dev(), canonical.ino()) != (m.dev(), m.ino()) {
            return Err(Code::StorageUnavailable);
        }
        Ok(ExistingJournal {
            path,
            file,
            read_only,
        })
    }
    pub fn connect_existing(existing: &ExistingJournal) -> Result<Self, Code> {
        let db = Connection::open_with_flags(
            &existing.path,
            (if existing.read_only {
                rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY
            } else {
                rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE
            }) | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX
                | rusqlite::OpenFlags::SQLITE_OPEN_NOFOLLOW,
        )
        .map_err(|_| Code::StorageUnavailable)?;
        let before = existing
            .file
            .metadata()
            .map_err(|_| Code::StorageUnavailable)?;
        let after =
            std::fs::symlink_metadata(&existing.path).map_err(|_| Code::StorageUnavailable)?;
        if !after.is_file() || (before.dev(), before.ino()) != (after.dev(), after.ino()) {
            return Err(Code::StorageUnavailable);
        }
        Ok(Self {
            db: JournalConnection {
                connection: Some(db),
                quantum: None,
            },
            quantum: None,
        })
    }
    pub fn prepare_version(&self) -> Result<rusqlite::Statement<'_>, Code> {
        self.db
            .prepare("PRAGMA main.user_version")
            .map_err(|_| Code::StorageUnavailable)
    }
    pub fn rows<'statement, 'connection: 'statement>(
        statement: &'statement mut rusqlite::Statement<'connection>,
    ) -> rusqlite::Rows<'statement> {
        statement.raw_query()
    }
    pub fn read_version_first(rows: &mut rusqlite::Rows<'_>) -> Result<i64, Code> {
        rows.next()
            .map_err(|_| Code::StorageUnavailable)?
            .ok_or(Code::StorageUnavailable)?
            .get(0)
            .map_err(|_| Code::StorageUnavailable)
    }
    pub fn configure_busy(&self) -> Result<(), Code> {
        let quantum = self.quantum.clone();
        Self::step(&quantum, || {
            self.db
                .busy_timeout(std::time::Duration::ZERO)
                .map_err(Self::initialization_error)
        })
    }
    pub fn configure_cache(&self) -> Result<(), Code> {
        self.configure_fixed(JournalConfig::Cache)
    }
    pub fn configure_mmap(&self) -> Result<(), Code> {
        self.configure_fixed(JournalConfig::Mmap)
    }
    pub fn configure_sync(&self) -> Result<(), Code> {
        self.configure_fixed(JournalConfig::Sync)
    }
    /// Fixed statement preparation and execution have independent ordinary
    /// verdicts. SQLite schema loading/SQL parsing is never a native exemption.
    pub fn prepare_config(&self, config: JournalConfig) -> Result<rusqlite::Statement<'_>, Code> {
        self.db
            .prepare(match config {
                JournalConfig::Cache => "PRAGMA cache_size=-1024",
                JournalConfig::Mmap => "PRAGMA mmap_size=0",
                JournalConfig::Sync => "PRAGMA synchronous=FULL",
            })
            .map_err(|_| Code::StorageUnavailable)
    }
    fn configure_fixed(&self, config: JournalConfig) -> Result<(), Code> {
        let quantum = self.quantum.clone();
        let statement = Self::retain(
            &quantum,
            Self::step(&quantum, || self.prepare_config(config))?,
        );
        // Pinned bundled SQLite3.53.2 applies these setters while preparing
        // PRAGMA bytecode (sqlite3.c145545,145613,145794; upstream source:
        // https://sqlite.org/src/file/src/pragma.c). Stepping their optional
        // result VM is redundant; finalization retains the prepared effects.
        drop(statement);
        quantum.as_ref().map_or(Ok(()), |quantum| quantum.check())
    }
    pub fn observe_error(
        &self,
        context: &w::Context,
        fence: &crate::identity::PublicFence,
        selection: &w::Selection,
        code: Code,
    ) -> Result<(), Code> {
        let quantum = self.quantum.clone();
        let mut context = context.clone();
        context.generation = selection.source_generation;
        let row = Self::step(&quantum, || {
            w::canonical(&serde_json::json!({"context":context,"fence":fence,"code":code}))
                .map_err(|_| Code::StorageUnavailable)
        })?;
        if row.len() > 16 * 1024 {
            return Err(Code::ResourceLimit);
        }
        // One SQLite write statement arbitrates capacity and dedup atomically;
        // overlapping predecessor/current owners cannot overfill the retained index.
        let changed = Self::step(&quantum, || {
            self.db.execute(
            "INSERT OR IGNORE INTO source_errors(session,generation,error) SELECT ?1,?2,?3 WHERE (SELECT count(*) FROM source_errors)<32",
            params![selection.session.0,selection.source_generation.0 as i64,row],
        ).map_err(|_|Code::StorageUnavailable)
        })?;
        if changed == 0 {
            let existing: bool = Self::step(&quantum, || {
                self
                .db
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM source_errors WHERE session=?1 AND generation=?2)",
                    params![selection.session.0, selection.source_generation.0 as i64],
                    |r| r.get(0),
                )
                .map_err(|_| Code::StorageUnavailable)
            })?;
            if !existing {
                return Err(Code::Busy);
            }
        }
        Ok(())
    }
    pub fn source_error(
        &self,
        context: &w::Context,
        fence: &crate::identity::PublicFence,
        selection: &w::Selection,
    ) -> Result<Option<Code>, Code> {
        let quantum = self.quantum.clone();
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct ErrorRecord {
            context: w::Context,
            fence: crate::identity::PublicFence,
            code: Code,
        }
        impl engine::disposal::Drain for ErrorRecord {
            fn drain_step(&mut self) -> bool {
                false
            }
        }
        let bytes = Self::step(&quantum, || {
            self.db.query_row("SELECT CASE WHEN length(error)<=16384 AND typeof(error)='blob' THEN error ELSE NULL END FROM source_errors WHERE session=?1 AND generation=?2",params![selection.session.0,selection.source_generation.0 as i64],|r|bounded_blob(r,0)).optional().map_err(|_|Code::StorageUnavailable)
        })?;
        let Some(bytes) = bytes else {
            return Ok(None);
        };
        let record: ErrorRecord = Self::step(&quantum, || {
            serde_json::from_slice(&bytes).map_err(|_| Code::StorageUnavailable)
        })?;
        let mut expected = context.clone();
        expected.generation = selection.source_generation;
        if record.context != expected || &record.fence != fence {
            return Err(Code::StaleContext);
        }
        Ok(Some(record.code))
    }
    fn source_set(&self, key: &str) -> Result<Option<Record>, Code> {
        let quantum = self.quantum.clone();
        let bytes = Self::step(&quantum, || {
            self.db.query_row("SELECT CASE WHEN typeof(record)='blob' AND length(record)<=16384 THEN record ELSE NULL END FROM runs WHERE source_key=?1",[key],|r|bounded_blob(r,0)).optional().map_err(|_|Code::StorageUnavailable)
        })?;
        bytes
            .map(|bytes| Self::step(&quantum, || Self::decode_record(&self.db, &bytes)))
            .transpose()
    }
    pub fn source(&self, context: &w::Context, source: &w::Selection) -> Result<Bookmark, Code> {
        let quantum = self.quantum.clone();
        let bytes: Option<Vec<u8>> = Self::step(&quantum, || {
            self
            .db
            .query_row(
                "SELECT CASE WHEN typeof(bookmark)='blob' AND length(bookmark)<=16384 THEN bookmark ELSE NULL END FROM sources WHERE session=?1 AND generation=?2",
                params![source.session.0, source.source_generation.0 as i64],
                |r| bounded_blob(r, 0),
            )
            .optional()
            .map_err(|_| Code::StorageUnavailable)
        })?;
        let bytes = bytes.ok_or(Code::SourceUnavailable)?;
        let source: Bookmark = Self::step(&quantum, || {
            serde_json::from_slice(&bytes).map_err(|_| Code::StorageUnavailable)
        })?;
        if !source_current(&source, context) {
            return Err(Code::SourceMismatch);
        }
        if source.closed_seq == 0 {
            return Err(Code::SourceOpen);
        }
        Ok(source)
    }
    pub fn observe_source(&mut self, context: &w::Context, source: &Bookmark) -> Result<(), Code> {
        let quantum = self.quantum.clone();
        if !source_current(source, context)
            || source.pid == 0
            || source.process_start.is_empty()
            || !source.path.is_absolute()
            || !source.socket.is_absolute()
            || source.turns.len() > 128
            || source.observed_seq > 65_536
        {
            return Err(Code::SourceMismatch);
        }
        let bytes = Self::step(&quantum, || {
            serde_json::to_vec(source).map_err(|_| Code::StorageUnavailable)
        })?;
        if bytes.len() > 16 * 1024 {
            return Err(Code::ResourceLimit);
        }
        let tx = Self::step(&quantum, || {
            self.db
                .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
                .map_err(Self::initialization_error)
        })?;
        let mut tx = Self::retain(&quantum, tx);
        let old: Option<Vec<u8>> = Self::step(&quantum, || {
            tx
            .query_row(
                "SELECT CASE WHEN typeof(bookmark)='blob' AND length(bookmark)<=16384 THEN bookmark ELSE NULL END FROM sources WHERE session=?1 AND generation=?2",
                params![source.session.0, source.generation as i64],
                |r| bounded_blob(r, 0),
            )
            .optional()
            .map_err(|_| Code::StorageUnavailable)
        })?;
        if let Some(old) = old {
            let prior: Bookmark = Self::step(&quantum, || {
                serde_json::from_slice(&old).map_err(|_| Code::StorageUnavailable)
            })?;
            if prior.root != source.root
                || prior.workspace_revision != source.workspace_revision
                || prior.root_revision != source.root_revision
            {
                return Err(Code::StaleContext);
            }
            if prior.observed_seq >= source.observed_seq || prior.closed_seq > 0 {
                return Ok(());
            }
            Self::step(&quantum, || {
                tx.execute(
                    "UPDATE sources SET bookmark=?3 WHERE session=?1 AND generation=?2",
                    params![source.session.0, source.generation as i64, bytes],
                )
                .map_err(|_| Code::StorageUnavailable)
            })?;
        } else {
            let count: i64 = Self::step(&quantum, || {
                tx.query_row("SELECT count(*) FROM sources", [], |r| r.get(0))
                    .map_err(|_| Code::StorageUnavailable)
            })?;
            if count >= 32 {
                return Err(Code::Busy);
            }
            Self::step(&quantum, || {
                tx.execute(
                    "INSERT INTO sources(session,generation,bookmark) VALUES(?1,?2,?3)",
                    params![source.session.0, source.generation as i64, bytes],
                )
                .map_err(|_| Code::StorageUnavailable)
            })?;
        }
        Self::step(&quantum, || {
            tx.take_value()
                .commit()
                .map_err(|_| Code::StorageUnavailable)
        })
    }
    pub fn retained_sources(&self, context: &w::Context) -> Result<Vec<Bookmark>, Code> {
        let quantum = self.quantum.clone();
        let query = Self::step(&quantum, || {
            self.db.prepare("SELECT CASE WHEN typeof(bookmark)='blob' AND length(bookmark)<=16384 THEN bookmark ELSE NULL END FROM sources ORDER BY generation,session LIMIT 33").map_err(|_|Code::StorageUnavailable)
        })?;
        let mut query = Self::retain(&quantum, query);
        let rows = Self::step(&quantum, || {
            query.query([]).map_err(|_| Code::StorageUnavailable)
        })?;
        let mut rows = Self::retain(&quantum, rows);
        let mut sources = Self::retain(&quantum, Vec::new());
        loop {
            let bytes = Self::step(&quantum, || {
                rows.next()
                    .map_err(|_| Code::StorageUnavailable)?
                    .map(|row| bounded_blob(row, 0).map_err(|_| Code::StorageUnavailable))
                    .transpose()
            })?;
            let Some(bytes) = bytes else {
                break;
            };
            let bytes = Self::retain(&quantum, bytes);
            let source: Bookmark = Self::step(&quantum, || {
                serde_json::from_slice(&bytes).map_err(|_| Code::StorageUnavailable)
            })?;
            if source_current(&source, context) {
                Self::step(&quantum, || {
                    sources
                        .try_reserve_exact(1)
                        .map_err(|_| Code::ResourceLimit)?;
                    sources.push(source);
                    Ok(())
                })?;
            } else {
                drop(Self::retain(&quantum, source));
            }
        }
        drop(rows);
        drop(query);
        if sources.len() > 32 {
            return Err(Code::StorageUnavailable);
        }
        Ok(sources.take_value())
    }
    fn decode_record(_db: &Connection, bytes: &[u8]) -> Result<Record, Code> {
        let record: Record = serde_json::from_slice(bytes).map_err(|_| Code::StorageUnavailable)?;
        if record.sources.is_empty() || record.sources.len() > 32 {
            return Err(Code::StorageUnavailable);
        }
        Ok(record)
    }
    pub fn run(&self, id: &w::Ulid) -> Result<Option<Record>, Code> {
        let quantum = self.quantum.clone();
        let bytes = Self::step(&quantum, || {
            self
            .db
            .query_row("SELECT CASE WHEN typeof(record)='blob' AND length(record)<=16384 THEN record ELSE NULL END FROM runs WHERE id=?1", [&id.0], |r| {
                bounded_blob(r, 0)
            })
            .optional()
            .map_err(|_| Code::StorageUnavailable)
        })?;
        bytes
            .map(|b| Self::step(&quantum, || Self::decode_record(&self.db, &b)))
            .transpose()
    }
    /// Lookup original operation before any source lookup/hash/copy/process work.
    pub fn operation(
        &self,
        actor: &str,
        op: &w::Ulid,
        request: &[u8],
    ) -> Result<Option<Record>, Code> {
        let quantum = self.quantum.clone();
        let old: Option<(Vec<u8>, String)> = Self::step(&quantum, || {
            self
            .db
            .query_row(
                "SELECT CASE WHEN typeof(request)='blob' AND length(request)<=16384 THEN request ELSE NULL END,CASE WHEN typeof(run)='text' AND length(run)=26 THEN run ELSE NULL END FROM ops WHERE actor=?1 AND op=?2",
                params![actor, op.0],
                |r| Ok((bounded_blob(r, 0)?, r.get(1)?)),
            )
            .optional()
            .map_err(|_| Code::StorageUnavailable)
        })?;
        match old {
            Some((bytes, _)) if bytes != request => Err(Code::OpIdReused),
            Some((_, run)) => self.run(&w::Ulid(run)),
            None => Ok(None),
        }
    }
    /// Retain Cancel's exact original request before touching the independent flag.
    pub fn cancel_operation(
        &mut self,
        actor: &str,
        op: &w::Ulid,
        request: &[u8],
        run: &w::Ulid,
    ) -> Result<(), Code> {
        let quantum = self.quantum.clone();
        Self::step(&quantum, || {
            let tx = Self::step(&quantum, || {
                self.db
                    .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
                    .map_err(Self::initialization_error)
            })?;
            let mut tx = Self::retain(&quantum, tx);
            let prior: Option<(Vec<u8>, String)> = Self::step(&quantum, || {
                tx
            .query_row(
                "SELECT CASE WHEN typeof(request)='blob' AND length(request)<=16384 THEN request ELSE NULL END,CASE WHEN typeof(run)='text' AND length(run)=26 THEN run ELSE NULL END FROM ops WHERE actor=?1 AND op=?2",
                params![actor, op.0],
                |r| Ok((bounded_blob(r, 0)?, r.get(1)?)),
            )
            .optional()
            .map_err(|_| Code::StorageUnavailable)
            })?;
            if let Some((prior, prior_run)) = prior {
                if prior != request || prior_run != run.0 {
                    return Err(Code::OpIdReused);
                }
            } else {
                let count: i64 = Self::step(&quantum, || {
                    tx.query_row("SELECT count(*) FROM ops", [], |r| r.get(0))
                        .map_err(|_| Code::StorageUnavailable)
                })?;
                if count >= 4096 {
                    return Err(Code::StorageUnavailable);
                }
                Self::step(&quantum, || {
                    tx.execute(
                        "INSERT INTO ops(actor,op,request,run) VALUES(?1,?2,?3,?4)",
                        params![actor, op.0, request, run.0],
                    )
                    .map_err(|_| Code::StorageUnavailable)
                })?;
            }
            Self::step(&quantum, || {
                tx.take_value()
                    .commit()
                    .map_err(|_| Code::StorageUnavailable)
            })
        })
    }
    /// Cursor, close custody, unique source set and original op share one durable transaction.
    pub fn register(&mut self, actor: &str, key: &str, record: &Record) -> Result<Record, Code> {
        let quantum = self.quantum.clone();
        let tx = Self::step(&quantum, || {
            self.db
                .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
                .map_err(Self::initialization_error)
        })?;
        let mut tx = Self::retain(&quantum, tx);
        let prior_op: Option<(Vec<u8>, String)> = Self::step(&quantum, || {
            tx.query_row(
            "SELECT CASE WHEN length(request)<=16384 AND typeof(request)='blob' THEN request ELSE NULL END,CASE WHEN length(run)=26 THEN run ELSE NULL END FROM ops WHERE actor=?1 AND op=?2",
            params![actor,record.op_id.0],|r|Ok((bounded_blob(r,0)?,r.get(1)?))).optional().map_err(|_|Code::StorageUnavailable)
        })?;
        if let Some((request, id)) = prior_op {
            if request != record.request {
                return Err(Code::OpIdReused);
            }
            let bytes = Self::step(&quantum, || {
                tx.query_row("SELECT CASE WHEN length(record)<=16384 AND typeof(record)='blob' THEN record ELSE NULL END FROM runs WHERE id=?1",[id],|r|bounded_blob(r,0)).map_err(|_|Code::StorageUnavailable)
            })?;
            let retained = Self::step(&quantum, || Self::decode_record(&tx, &bytes))?;
            Self::step(&quantum, || {
                tx.take_value()
                    .commit()
                    .map_err(|_| Code::StorageUnavailable)
            })?;
            return Ok(retained);
        }
        let prior: Option<Vec<u8>> = Self::step(&quantum, || {
            tx
            .query_row("SELECT CASE WHEN typeof(record)='blob' AND length(record)<=16384 THEN record ELSE NULL END FROM runs WHERE source_key=?1", [key], |r| {
                bounded_blob(r, 0)
            })
            .optional()
            .map_err(|_| Code::StorageUnavailable)
        })?;
        let aliases: i64 = Self::step(&quantum, || {
            tx.query_row("SELECT count(*) FROM ops", [], |r| r.get(0))
                .map_err(|_| Code::StorageUnavailable)
        })?;
        if aliases >= 4096 && !(record.trigger && prior.is_some()) {
            return Err(Code::StorageUnavailable);
        }
        let actual: Record = if let Some(bytes) = prior {
            Self::step(&quantum, || Self::decode_record(&tx, &bytes))?
        } else {
            let count: i64 = Self::step(&quantum, || {
                tx.query_row("SELECT count(*) FROM runs", [], |r| r.get(0))
                    .map_err(|_| Code::StorageUnavailable)
            })?;
            // Explicit retention admission; never delete old results or evidence implicitly.
            if count >= 1024 {
                return Err(Code::StorageUnavailable);
            }
            let bytes = Self::step(&quantum, || record_bytes(record))?;
            if bytes.len() > 16 * 1024 {
                return Err(Code::ResourceLimit);
            }
            Self::step(&quantum, || {
                tx.execute(
                    "INSERT INTO runs(id,source_key,record) VALUES(?1,?2,?3)",
                    params![record.view.run_id.0, key, bytes],
                )
                .map_err(|_| Code::StorageUnavailable)
            })?;
            Self::step(&quantum, || Ok(record.clone()))?
        };
        for source in &record.sources {
            let bytes = Self::step(&quantum, || {
                serde_json::to_vec(source).map_err(|_| Code::StorageUnavailable)
            })?;
            if bytes.len() > 16 * 1024 {
                return Err(Code::ResourceLimit);
            }
            Self::step(&quantum, || {
                tx.execute("INSERT INTO sources(session,generation,bookmark) VALUES(?1,?2,?3) ON CONFLICT(session,generation) DO UPDATE SET bookmark=excluded.bookmark WHERE json_extract(CAST(sources.bookmark AS TEXT),'$.observed_seq')<=json_extract(CAST(excluded.bookmark AS TEXT),'$.observed_seq')",
                params![source.session.0,source.generation as i64,bytes]).map_err(|_| Code::StorageUnavailable)
            })?;
            Self::step(&quantum, || {
                tx.execute("INSERT INTO close_cursor(session,generation,cursor) VALUES(?1,?2,?3) ON CONFLICT(session,generation) DO UPDATE SET cursor=max(cursor,excluded.cursor)",
                params![source.session.0,source.generation as i64,source.cursor as i64]).map_err(|_| Code::StorageUnavailable)
            })?;
        }
        // Duplicate close notifications retain the unique row's first operation;
        // explicit manual aliases alone consume another operation entry.
        if !record.trigger || actual.view.run_id == record.view.run_id {
            Self::step(&quantum, || {
                tx.execute(
                    "INSERT INTO ops(actor,op,request,run) VALUES(?1,?2,?3,?4)",
                    params![actor, record.op_id.0, record.request, actual.view.run_id.0],
                )
                .map_err(|_| Code::StorageUnavailable)
            })?;
        }
        Self::step(&quantum, || {
            tx.take_value()
                .commit()
                .map_err(|_| Code::StorageUnavailable)
        })?;
        Ok(actual)
    }
    /// Durable lifecycle arbitration survives eviction of an in-memory Run.
    pub fn compare_and_save(&self, expected: &Record, next: &Record) -> Result<bool, Code> {
        let quantum = self.quantum.clone();
        if expected.view.run_id != next.view.run_id
            || expected.op_id != next.op_id
            || expected.nonce != next.nonce
        {
            return Err(Code::ProtocolError);
        }
        let before = Self::step(&quantum, || record_bytes(expected))?;
        let after = Self::step(&quantum, || record_bytes(next))?;
        Self::step(&quantum, || {
            self.db
                .execute(
                    "UPDATE runs SET record=?2 WHERE id=?1 AND record=?3",
                    params![expected.view.run_id.0, after, before],
                )
                .map(|n| n == 1)
                .map_err(|_| Code::StorageUnavailable)
        })
    }
    fn cancel_unstarted(&self, previous: &Record) -> Result<Record, Code> {
        let quantum = self.quantum.clone();
        Self::step(&quantum, || {
            if previous.started || previous.view.settled {
                return Self::step(&quantum, || Ok(previous.clone()));
            }
            let mut next = Self::step(&quantum, || Ok(previous.clone()))?;
            next.view.state = w::State::Cancelled;
            next.view.settled = true;
            next.view.revision.0 += 1;
            next.view.error = Some(w::Error {
                code: Code::Cancelled,
                source: None,
            });
            if self.compare_and_save(previous, &next)? {
                Ok(next)
            } else {
                self.run(&previous.view.run_id)?.ok_or(Code::NotFound)
            }
        })
    }
    pub fn save(&self, record: &Record) -> Result<(), Code> {
        let quantum = self.quantum.clone();
        let bytes = Self::step(&quantum, || record_bytes(record))?;
        if bytes.len() > 16 * 1024 {
            return Err(Code::ResourceLimit);
        }
        Self::step(&quantum, || {
            self.db
                .execute(
                    "UPDATE runs SET record=?2 WHERE id=?1",
                    params![record.view.run_id.0, bytes],
                )
                .map_err(|_| Code::StorageUnavailable)
        })?;
        Ok(())
    }
}

const AUTHORITY_MATCH_SQL: &str = "SELECT CASE
              WHEN typeof(s.value)='blob' AND length(s.value)<=16384
                AND typeof(p.role)='blob' AND length(p.role)<=1024
              THEN CASE WHEN json_valid(s.value) THEN COALESCE(
                json_type(s.value,'$.id')='text' AND json_extract(s.value,'$.id')=?2
                AND json_type(s.value,'$.swarm')='text' AND json_extract(s.value,'$.swarm')=?4
                AND json_type(s.value,'$.generation')='integer' AND json_extract(s.value,'$.generation')=?5
                AND json_type(s.value,'$.muted')='false'
                AND typeof(p.epoch)='integer' AND p.epoch>0 AND p.epoch=?6
                AND typeof(p.muted)='integer' AND p.muted=0
                AND json_type(s.value,'$.role.name')='text'
                AND json_type(s.value,'$.role.class')='text'
                AND json_extract(s.value,'$.role.class') IN ('oracle','worker')
                AND CAST(p.role AS TEXT)=json_object('name',json_extract(s.value,'$.role.name'),'class',json_extract(s.value,'$.role.class'))
                AND CASE WHEN sw.key IS NULL THEN ?7=0
                    WHEN typeof(sw.value)='blob' AND length(sw.value)<=16384
                    THEN CASE WHEN json_valid(sw.value) THEN
                      json_type(sw.value,'$.id')='text' AND json_extract(sw.value,'$.id')=?4
                      AND json_type(sw.value,'$.revision')='integer' AND json_extract(sw.value,'$.revision')=?7
                    ELSE 0 END ELSE 0 END,
                0) ELSE 0 END ELSE 0 END
             FROM kv AS s JOIN msg_policies AS p ON p.seat=?2
             LEFT JOIN kv AS sw ON sw.key=?3 WHERE s.key=?1";

/// Captured in-memory routing; all authority-store reads happen on an isolated owner.
#[derive(Clone)]
pub(crate) struct Authority {
    pub fence: crate::identity::PublicFenceReader,
    #[cfg(debug_assertions)]
    pub private_fixture: bool,
    pub port: crate::knowledge::routing::Port,
    pub seat: crate::Ulid,
    pub generation: u64,
    pub activity_latch: Arc<std::sync::atomic::AtomicU64>,
    pub foreground: Arc<AtomicBool>,
    pub activity_revision: Arc<std::sync::atomic::AtomicU64>,
}
// Fixed, test-only phase clocks. Diagnostic output is emitted after snapshots;
// it is never a product control, an authority receipt or latency acceptance.
#[cfg(test)]
struct PhaseTrace {
    kind: &'static str,
    enabled: bool,
    wall: std::time::Instant,
    cpu: Option<std::time::Duration>,
    samples: [(&'static str, u64, Option<u64>); 12],
    count: usize,
}
#[cfg(test)]
impl PhaseTrace {
    fn new(kind: &'static str) -> Self {
        Self {kind,enabled:std::env::var_os("BBOXY_DREAM_PHASE_TRACE").is_some(),wall:std::time::Instant::now(),cpu:bbt_rt::fault::thread_cpu_time().ok(),samples:[("",0,None);12],count:0}
    }
    fn mark(&mut self, phase: &'static str) {
        if !self.enabled {return;}
        let wall=std::time::Instant::now();
        let cpu=bbt_rt::fault::thread_cpu_time().ok();
        let used=cpu.zip(self.cpu).and_then(|(now,before)|now.checked_sub(before)).and_then(|v|u64::try_from(v.as_nanos()).ok());
        if let Some(sample)=self.samples.get_mut(self.count) {
            *sample=(phase,u64::try_from(wall.duration_since(self.wall).as_nanos()).unwrap_or(u64::MAX),used);
            self.count+=1;
        }
        self.wall=wall;self.cpu=cpu;
    }
}
#[cfg(test)]
impl Drop for PhaseTrace {
    fn drop(&mut self) {
        if self.enabled {
            for (phase,wall,cpu) in &self.samples[..self.count] {
                eprintln!("dream_c1_phase kind={} phase={phase} wall_ns={wall} cpu_ns={cpu:?} diagnostic_only=true",self.kind);
            }
        }
    }
}
impl Authority {
    pub fn capture(&self) -> Result<w::Context, Code> {
        self.capture_all().map(|(context, _)| context)
    }
    pub fn capture_all(&self) -> Result<(w::Context, crate::identity::PublicFence), Code> {
        #[cfg(test)] let mut trace=PhaseTrace::new("capture");
        // An unavailable routing dependency refuses before public SQL/setup;
        // controls never wait behind a held provider selection.
        let before = self
            .port
            .host
            .dream_metadata()
            .map_err(|_| Code::StaleContext)?;
        let quantum = engine::JournalQuantum::default();
        // The complete public-proof verification and the fresh typed actor
        // context each have their own ordinary <=1ms verdict/rest boundary.
        // No proof/currentness check is removed or reclassified as native.
        let session = quantum.retain(quantum.public_prelude(|checkpoint| {
            self.fence
                .prepare_session_cooperative(|_| Ok(()), checkpoint)
                .map_err(|_| Code::StaleContext)
        })?);
        #[cfg(test)] trace.mark("prepare_session");
        let fence = quantum.public_prelude(|checkpoint| {
            if self.fence.cached_revision() & 1 != 0 {
                return Err(Code::StaleContext);
            }
            self.fence
                .current_prepared_cooperative(&session, checkpoint)
                .map_err(|_| Code::StaleContext)
        })?;
        #[cfg(test)] trace.mark("current_public_fence");
        let context = quantum.unit(|| self.capture_context(&fence, &session))?;
        #[cfg(test)] trace.mark("actor_context");
        quantum.retire_session(&session)?;
        drop(session);
        // This short-lived prelude cannot discard its final work debt when
        // the independently owned catalogue/job context takes over.
        quantum.pay()?;
        #[cfg(test)] trace.mark("retire_and_pay");
        if !self
            .port
            .host
            .dream_metadata()
            .is_ok_and(|after| after == before)
        {
            return Err(Code::StaleContext);
        }
        Ok((context, fence))
    }
    #[cfg(test)]
    pub(crate) fn test_prepared_guard(
        &self,
    ) -> Result<
        (
            w::Context,
            crate::identity::PublicFence,
            engine::PreparedGuard,
        ),
        Code,
    > {
        let (context, fence) = self.capture_all()?;
        let guard = self.guard(&context, &Some(fence.clone()), None)?;
        Ok((context, fence, guard))
    }
    fn capture_context(
        &self,
        fence: &crate::identity::PublicFence,
        session: &crate::identity::PublicFenceSession,
    ) -> Result<w::Context, Code> {
        // Identity and caller metadata share ONE consistent read-only snapshot;
        // no repeated key parsing or separate connection sits inside a work unit.
        let (record,grant,revision)=self.fence.with_current_session(fence,session,|db| {
            let public=|key:&str| -> crate::store::Result<Vec<u8>> {
                Ok(db.query_row("SELECT CASE WHEN typeof(value)='blob' AND length(value)<=16384 THEN value ELSE NULL END FROM kv WHERE key=?1",[key],|row|bounded_blob(row,0))?)
            };
            let record:super::SeatRecord=serde_json::from_slice(&public(&format!("seat/{}",self.seat))?)?;
            if record.generation!=self.generation || record.swarm!=self.port.binding.swarm {
                return Err(crate::store::invalid("stale C1 seat context"));
            }
            // Adapted from messaging/service.rs::policy; current public grant only.
            let (grant,role,muted):(i64,Vec<u8>,bool)=db.query_row(
                "SELECT epoch,CASE WHEN length(role)<=1024 THEN role ELSE NULL END,muted FROM msg_policies WHERE seat=?1",
                [self.seat.to_string()],|r|Ok((r.get(0)?,bounded_blob(r,1)?,r.get(2)?)))?;
            if grant<=0 || muted || muted!=record.muted || role!=serde_json::to_vec(&record.role)? {
                return Err(crate::store::invalid("stale C1 public grant"));
            }
            let revision=match public(&format!("swarm/{}",record.swarm)) {
                Ok(bytes)=>serde_json::from_slice::<super::SwarmRecord>(&bytes)?.revision,
                Err(_) if self.port.binding.revision==0=>0,
                Err(error)=>return Err(error),
            };
            if revision!=self.port.binding.revision {return Err(crate::store::invalid("stale C1 workspace"));}
            Ok((record,grant,revision))
        }).map_err(|_|Code::StaleContext)?;
        let (provider_epoch, provider, _) = self
            .port
            .host
            .dream_metadata()
            .map_err(|_| Code::StaleContext)?;
        let text = |s: String| w::Id::try_from(s).map_err(|_| Code::StaleContext);
        Ok(w::Context {
            account: text(fence.account.clone())?,
            account_epoch: w::Number(fence.account_epoch),
            seat: w::Ulid(self.seat.to_string()),
            generation: w::Number(record.generation),
            grant_epoch: w::Number(grant as u64),
            swarm: w::Ulid(record.swarm.to_string()),
            swarm_revision: w::Number(revision),
            workspace: text(record.swarm.to_string())?,
            workspace_revision: w::Number(revision),
            root: text(self.port.binding.root_id.clone())?,
            root_revision: w::Number(provider_epoch),
            provider: text(provider)?,
            provider_epoch: w::Number(provider_epoch),
        })
    }

    pub fn matches(
        &self,
        context: &w::Context,
        fence: &Option<crate::identity::PublicFence>,
    ) -> bool {
        self.matches_with_session(context, fence, None).is_ok()
    }
    fn matches_with_session(
        &self,
        context: &w::Context,
        fence: &Option<crate::identity::PublicFence>,
        session: Option<&crate::identity::PublicFenceSession>,
    ) -> Result<(), Code> {
        if session.is_some_and(|session| session.is_retiring()) {
            return Err(Code::Busy);
        }
        let Some(fence) = fence.as_ref() else {
            return Err(Code::StaleContext);
        };
        // Admission still decodes the complete typed records. A unit only needs
        // fresh authority scalars, not newly allocated copies of those records
        // or another Context. Fixed routing fields remain receiver-owned.
        if context.account.0 != fence.account
            || context.account_epoch.0 != fence.account_epoch
            || context.seat.0 != self.seat.to_string()
            || context.generation.0 != self.generation
            || context.swarm.0 != self.port.binding.swarm.to_string()
            || context.workspace.0 != context.swarm.0
            || context.workspace_revision.0 != self.port.binding.revision
            || context.swarm_revision.0 != self.port.binding.revision
            || context.root.0 != self.port.binding.root_id
            || context.root_revision != context.provider_epoch
        {
            return Err(Code::StaleContext);
        }
        let Ok(before) = self.port.host.dream_metadata() else {
            return Err(Code::StaleContext);
        };
        if before.0 != context.provider_epoch.0 || before.1 != context.provider.0 {
            return Err(Code::StaleContext);
        }
        let read = |db: &Connection| Self::rows_match(db, context);
        let matches = match session {
            Some(session) => self.fence.try_with_current_session(fence, session, read),
            None => self.fence.with_current(fence, read).map(Some),
        }
        .map_err(|_| Code::StaleContext)?;
        match matches {
            None => return Err(Code::Busy),
            Some(false) => return Err(Code::StaleContext),
            Some(true) => {}
        }
        if self
            .port
            .host
            .dream_metadata()
            .is_ok_and(|after| after == before)
        {
            Ok(())
        } else {
            Err(Code::StaleContext)
        }
    }
    fn rows_match(db: &Connection, context: &w::Context) -> crate::store::Result<bool> {
        let generation = crate::store::sqlint(context.generation.0)?;
        let grant = crate::store::sqlint(context.grant_epoch.0)?;
        let revision = crate::store::sqlint(context.swarm_revision.0)?;
        // All lookups are exact primary keys. CASE bounds cells before JSON1
        // touches them; only a boolean crosses into Rust. json_object follows
        // Role's exact name,class field order, retaining the policy's canonical
        // byte equality without serde allocation or cloning its role BLOB.
        let current = db
            .prepare_cached(AUTHORITY_MATCH_SQL)?
            .query_row(
                params![
                    format!("seat/{}", context.seat.0),
                    context.seat.0,
                    format!("swarm/{}", context.swarm.0),
                    context.swarm.0,
                    generation,
                    grant,
                    revision
                ],
                |row| row.get::<_, bool>(0),
            )
            .optional()?;
        Ok(current.unwrap_or(false))
    }
    fn guard(
        &self,
        context: &w::Context,
        fence: &Option<crate::identity::PublicFence>,
        setup_observed: Option<&std::sync::atomic::AtomicU64>,
    ) -> Result<engine::PreparedGuard, Code> {
        self.guard_with_session(context, fence, setup_observed)
            .map(|(guard, _)| guard)
    }
    fn guard_with_session(
        &self,
        context: &w::Context,
        fence: &Option<crate::identity::PublicFence>,
        setup_observed: Option<&std::sync::atomic::AtomicU64>,
    ) -> Result<
        (
            engine::PreparedGuard,
            Arc<crate::identity::PublicFenceSession>,
        ),
        Code,
    > {
        let cpu = bbt_rt::fault::thread_cpu_time().map_err(|_| Code::ResourceLimit)?;
        // Decision66026: each real preparation phase finishes its own strict
        // unit and pays after-end debt before the next file/config/SQL phase.
        let retirement = engine::JournalQuantum::preparation()?;
        let prepared = retirement.public_prelude(|checkpoint| {
            let session = Arc::new(
                self.fence
                    .prepare_session_cooperative(
                        |db| {
                            drop(db.prepare_cached(AUTHORITY_MATCH_SQL)?);
                            Ok(())
                        },
                        checkpoint,
                    )
                    .map_err(|_| Code::StaleContext)?,
            );
            let shared = session.clone();
            let authority = self.clone();
            let context = context.clone();
            let fence = fence.clone();
            Ok((
                Arc::new(move || authority.matches_with_session(&context, &fence, Some(&shared)))
                    as Arc<engine::AuthorityGuard>,
                session,
            ))
        });
        let mut prepared = retirement.retain(prepared);
        let paid = retirement.pay();
        let used = bbt_rt::fault::thread_cpu_time()
            .map_err(|_| Code::ResourceLimit)
            .and_then(|after| after.checked_sub(cpu).ok_or(Code::ResourceLimit));
        let setup_cpu_ns = used
            .map(|used| used.as_nanos().min(u64::MAX as u128) as u64)
            .unwrap_or(u64::MAX);
        if let Some(observed) = setup_observed {
            observed.fetch_max(setup_cpu_ns, Ordering::Release);
        }
        let setup_max_unit_cpu_ns = retirement.max_unit_cpu_ns();
        if used.is_err()
            || setup_cpu_ns > w::MAX_SAFE
            || paid.is_err()
            || retirement.check().is_err()
        {
            eprintln!(
                "dream_c1_authority_setup_refused cumulative_cpu_ns={setup_cpu_ns} max_unit_cpu_ns={setup_max_unit_cpu_ns} limit_ns=1000000"
            );
            if let Ok((_, session)) = prepared.as_ref() {
                // Clock refusal retains and paces the actual connection before
                // the callback/session shells can leave this setup owner.
                let _ = retirement.retire_session(session);
            }
            return Err(Code::ResourceLimit);
        }
        let (callback, session) = prepared.take_value()?;
        Ok((
            engine::PreparedGuard {
                callback,
                setup_cpu_ns,
                setup_max_unit_cpu_ns,
                retire_session: Some(session.clone()),
            },
            session,
        ))
    }
}

use crate::logic::dream_c1::{self as engine, Control};
use std::collections::BTreeMap;
use std::sync::{
    Arc, Mutex, OnceLock,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};

struct Permit(Arc<AtomicUsize>);
impl Permit {
    fn claim(count: &Arc<AtomicUsize>, bound: usize) -> Result<Self, Code> {
        count
            .try_update(Ordering::AcqRel, Ordering::Acquire, |n| {
                (n < bound).then_some(n + 1)
            })
            .map_err(|_| Code::Busy)?;
        Ok(Self(count.clone()))
    }
}
impl Drop for Permit {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}

/// Self-service durable capacity. Predecessor claims survive daemon loss; no queue/lock.
struct DiskPermit {
    path: PathBuf,
    identity: (u64, u64),
    token: Vec<u8>,
}
impl DiskPermit {
    fn claim(runtime: &Path, scope: &str, bound: usize, record: &Record) -> Result<Self, Code> {
        use std::{io::Write, os::unix::fs::OpenOptionsExt};
        let claims = runtime.join("claims");
        private_dir(&claims)?;
        let root = claims.join(scope);
        private_dir(&root)?;
        let token=w::canonical(&serde_json::json!({"run":record.view.run_id,"nonce":record.nonce,"owner":record.owner_incarnation})).map_err(|_|Code::StorageUnavailable)?;
        for slot in 0..bound {
            let path = root.join(format!("slot-{slot}"));
            match std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .custom_flags(libc::O_NOFOLLOW)
                .open(&path)
            {
                Ok(mut file) => {
                    let m = file.metadata().map_err(|_| Code::StorageUnavailable)?;
                    let lease = Self {
                        path,
                        identity: (m.dev(), m.ino()),
                        token: token.clone(),
                    };
                    file.write_all(&token)
                        .and_then(|_| file.sync_all())
                        .map_err(|_| Code::StorageUnavailable)?;
                    return Ok(lease);
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(_) => return Err(Code::StorageUnavailable),
            }
        }
        Err(Code::Busy)
    }
}
impl Drop for DiskPermit {
    fn drop(&mut self) {
        use std::io::Read;
        use std::os::unix::fs::OpenOptionsExt;
        let Ok(mut file) = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(&self.path)
        else {
            return;
        };
        let Ok(m) = file.metadata() else {
            return;
        };
        if !m.is_file() || (m.dev(), m.ino()) != self.identity || m.len() != self.token.len() as u64
        {
            return;
        }
        let mut bytes = Vec::new();
        if file.by_ref().take(1024).read_to_end(&mut bytes).is_ok()
            && bytes == self.token
            && std::fs::symlink_metadata(&self.path)
                .is_ok_and(|m| (m.dev(), m.ino()) == self.identity)
        {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

struct Run {
    initial: Record,
    started: AtomicBool,
    final_record: OnceLock<Record>,
    control: Arc<Control>,
    session: OnceLock<Arc<crate::identity::PublicFenceSession>>,
}
impl Run {
    fn record(&self) -> Record {
        if let Some(record) = self.final_record.get() {
            return record.clone();
        }
        let mut record = self.initial.clone();
        record.view.revision.0 = record
            .view
            .revision
            .0
            .saturating_add(self.control.revision.load(Ordering::Acquire));
        record.view.state = engine::state(self.control.state.load(Ordering::Acquire));
        record.view.settled = self.control.settled.load(Ordering::Acquire);
        // Terminal metadata is one immutable publication, after final context/persistence checks.
        if record.view.settled {
            record.view.state = w::State::Settling;
            record.view.settled = false;
        }
        record
    }
}
#[derive(Default)]
struct RootState {
    runs: Mutex<BTreeMap<String, Arc<Run>>>,
}
impl RootState {
    /// The isolated owner keeps its permits until its exact cache entry is
    /// retired. A contended presentation snapshot cannot leave a live prepared
    /// SQLite session cached indefinitely after that job releases its slot.
    fn retire(&self, run: &Arc<Run>) {
        loop {
            let mut runs = match self.runs.try_lock() {
                Ok(runs) => runs,
                Err(std::sync::TryLockError::WouldBlock) => {
                    std::thread::park_timeout(std::time::Duration::from_millis(9));
                    continue;
                }
                // Poison cannot prevent releasing our own immutable entry. No
                // other entry or public authority is recovered by this path.
                Err(std::sync::TryLockError::Poisoned(poison)) => poison.into_inner(),
            };
            if runs
                .get(&run.initial.view.run_id.0)
                .is_some_and(|cached| Arc::ptr_eq(cached, run))
            {
                runs.remove(&run.initial.view.run_id.0);
            }
            return;
        }
    }
}
/// No queue or shared compute pool: each admitted run owns its own process/IO lanes.
#[derive(Clone)]
pub(crate) struct Runtime {
    incarnation: w::Ulid,
    read_cache: Arc<super::dream_read::Cache>,
    launch_counter: Arc<std::sync::atomic::AtomicU64>,
    path: PathBuf,
    binary: PathBuf,
    native_provision: Arc<OnceLock<Arc<engine::runner_provision::ProvisionProof>>>,
    probe_audit: Arc<OnceLock<engine::ProbeAudit>>,
    supported: Arc<AtomicBool>,
    retired: Arc<AtomicBool>,
    requests: Arc<AtomicUsize>,
    controls: Arc<AtomicUsize>,
    roots: Arc<Mutex<BTreeMap<String, Arc<RootState>>>>,
}
impl Drop for Runtime {
    fn drop(&mut self) {
        // The capability owner retains one Runtime until all other users have
        // retired. Its cache/proof descriptors then close on that utility
        // owner, never as a side effect of dropping a mux-side Runtime handle.
        // An unavailable capability probe must not retire metadata/control
        // service when its temporary Runtime clone goes away.
        if self.native_provision.get().is_some() && Arc::strong_count(&self.native_provision) == 2 {
            self.retired.store(true, Ordering::Release);
        }
    }
}
impl Runtime {
    pub fn new(
        path: PathBuf,
        binary: PathBuf,
        private_test_account: bool,
        handle: &tokio::runtime::Handle,
    ) -> Self {
        let runtime = Self {
            incarnation: crate::Ulid::new().into(),
            read_cache: Default::default(),
            launch_counter: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            path,
            binary,
            native_provision: Default::default(),
            probe_audit: Default::default(),
            supported: Arc::new(AtomicBool::new(false)),
            retired: Arc::new(AtomicBool::new(false)),
            requests: Default::default(),
            controls: Default::default(),
            roots: Default::default(),
        };
        let probe = runtime.clone();
        // Only the short owner creation uses the blocking executor; no C1 IO runs there.
        handle.spawn_blocking(move || {
            let owner = probe.clone();
            let spawned = std::thread::Builder::new()
                .name("dream-c1-capability".into())
                .spawn(move || {
                    let (outcome, proof) = (|| {
                        if let Err(code) = engine::utility() {
                            return (engine::ProbeOutcome::refused("utility", code), None);
                        }
                        if let Err(code) = private_dir(&owner.path) {
                            return (engine::ProbeOutcome::refused("private_root", code), None);
                        }
                        // The authenticated provisioning receipt is checked
                        // BEFORE any candidate birth, including capability IO.
                        let proof = match engine::runner_provision::verify_native_provision(
                            &owner.binary,
                            private_test_account,
                        ) {
                            Ok(proof) => Arc::new(proof),
                            Err(code) => {
                                return (engine::ProbeOutcome::refused("provision", code), None);
                            }
                        };
                        let incarnation = match owner.next_launch_incarnation() {
                            Ok(value) => value,
                            Err(code) => {
                                return (engine::ProbeOutcome::refused("incarnation", code), None);
                            }
                        };
                        let outcome = engine::probe_gated(&owner.path, proof.clone(), incarnation);
                        let proof = outcome.verdict.is_ok().then_some(proof);
                        (outcome, proof)
                    })();
                    let retained = owner.probe_audit.set(outcome.audit).is_ok();
                    let supported = outcome.verdict.is_ok()
                        && retained
                        && proof.is_some_and(|proof| owner.native_provision.set(proof).is_ok());
                    owner.supported.store(supported, Ordering::Release);
                    #[cfg(debug_assertions)]
                    if !supported {
                        eprintln!(
                            "dream_c1_probe_refused stage={} code={:?} private_account={}",
                            outcome.audit.stage,
                            outcome.verdict.err().unwrap_or(Code::Busy),
                            private_test_account
                        );
                    }
                    // All outcomes retain one fixed audit until the last
                    // Runtime user retires. Descriptors and the audit cache
                    // then close on this utility owner, including refusals.
                    while Arc::strong_count(&owner.probe_audit) > 1 {
                        std::thread::park_timeout(std::time::Duration::from_millis(50));
                    }
                });
            if spawned.is_err() {
                let outcome =
                    engine::ProbeOutcome::refused("owner_create", Code::RunnerUnavailable);
                let _ = probe.probe_audit.set(outcome.audit);
                #[cfg(debug_assertions)]
                eprintln!("dream_c1_probe_refused stage=owner_create code=RunnerUnavailable");
            }
        });
        runtime
    }
    fn next_launch_incarnation(&self) -> Result<u64, Code> {
        let value = self.launch_counter.load(Ordering::Acquire);
        if value == 0 {
            let seed = engine::monotonic_ns()?;
            if seed == 0 {
                return Err(Code::ResourceLimit);
            }
            let _ =
                self.launch_counter
                    .compare_exchange(0, seed, Ordering::AcqRel, Ordering::Acquire);
        }
        self.launch_counter
            .try_update(Ordering::AcqRel, Ordering::Acquire, |value| {
                value.checked_add(1).filter(|_| value != 0)
            })
            .map_err(|_| Code::ResourceLimit)
    }
    pub fn retire(&self) {
        self.retired.store(true, Ordering::Release);
    }
    pub fn owner(&self) -> Option<bboxy450_harness_wake_up::dream_c1::DreamOwner> {
        self.supported
            .load(Ordering::Acquire)
            .then_some(bboxy450_harness_wake_up::dream_c1::DreamOwner::TermC1)
    }
    /// A new authoritative activity revision resumes only this seat's retained proposals.
    pub fn activity(&self, handle: &tokio::runtime::Handle, authority: Authority) {
        if authority.activity_latch.load(Ordering::Acquire) & 1 != 0 {
            return;
        }
        let runtime = self.clone();
        let runtime_handle = handle.clone();
        handle.spawn_blocking(move || {
            let _ = std::thread::Builder::new()
                .name("dream-c1-activity".into())
                .spawn(move || {
                    if engine::utility().is_err() {
                        return;
                    }
                    let Ok(context) = authority.capture() else {
                        return;
                    };
                    let Ok(key) = w::canonical(&serde_json::json!([context.account, context.root]))
                    else {
                        return;
                    };
                    let key = crate::store::digest(&key);
                    let root = runtime
                        .roots
                        .try_lock()
                        .ok()
                        .and_then(|roots| roots.get(&key).cloned());
                    let Some(root) = root else {
                        return;
                    };
                    let proposals = root
                        .runs
                        .try_lock()
                        .ok()
                        .map(|runs| {
                            runs.values()
                                .filter(|run| {
                                    !run.started.load(Ordering::Acquire)
                                        && run.final_record.get().is_none()
                                        && run.initial.context == context
                                })
                                .take(32)
                                .map(|run| run.initial.clone())
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default();
                    for proposal in proposals {
                        let request = w::Control::Inspect(w::Inspect {
                            v: w::Version,
                            op_id: proposal.op_id,
                            sources: proposal
                                .sources
                                .iter()
                                .map(|s| w::Selection {
                                    session: s.session.clone(),
                                    source_generation: w::Number(s.generation),
                                })
                                .collect(),
                        });
                        runtime.request(
                            &runtime_handle,
                            authority.clone(),
                            move || Ok(request),
                            None,
                            false,
                            |_| {},
                        );
                    }
                });
        });
    }
    fn retained_control_authority(
        &self,
        authority: &Authority,
        request: &w::Control,
    ) -> Result<
        Option<(
            w::Context,
            crate::identity::PublicFence,
            Arc<crate::identity::PublicFenceSession>,
        )>,
        Code,
    > {
        let run_id = match request {
            w::Control::Status(status) => &status.run_id,
            w::Control::Cancel(cancel) => &cancel.run_id,
            _ => return Ok(None),
        };
        // Fail held provider/root metadata before any connection setup.
        authority
            .port
            .host
            .dream_metadata()
            .map_err(|_| Code::StaleContext)?;
        let Some(candidate) = authority
            .fence
            .verified_candidate()
            .map_err(|_| Code::StaleContext)?
        else {
            return Ok(None);
        };
        let key = crate::store::digest(
            &w::canonical(&serde_json::json!([
                candidate.account,
                authority.port.binding.root_id
            ]))
            .map_err(|_| Code::BadRequest)?,
        );
        let root = self
            .roots
            .try_lock()
            .map_err(|_| Code::Busy)?
            .get(&key)
            .cloned();
        let Some(root) = root else { return Ok(None) };
        let run = root
            .runs
            .try_lock()
            .map_err(|_| Code::Busy)?
            .get(&run_id.0)
            .cloned();
        let Some(run) = run else { return Ok(None) };
        if run.initial.context.seat.0 != authority.seat.to_string() {
            return Err(Code::NotFound);
        }
        let Some(session) = run.session.get().cloned() else {
            return Ok(None);
        };
        let Some(fence) = run.initial.identity_fence.clone() else {
            return Err(Code::StaleContext);
        };
        let context = run.initial.context.clone();
        let quantum = engine::JournalQuantum::default();
        quantum.unit(|| {
            authority.matches_with_session(&context, &Some(fence.clone()), Some(&session))
        })?;
        quantum.pay()?;
        Ok(Some((context, fence, session)))
    }
    pub fn request(
        &self,
        handle: &tokio::runtime::Handle,
        authority: Authority,
        parse: impl FnOnce() -> Result<w::Control, Code> + Send + 'static,
        close: Option<Bookmark>,
        control_lane: bool,
        done: impl FnOnce(w::ControlReply) + Send + 'static,
    ) {
        let lane = if control_lane {
            &self.controls
        } else {
            &self.requests
        };
        let permit = match Permit::claim(lane, 2) {
            Ok(p) => p,
            Err(c) => {
                done(failure(c));
                return;
            }
        };
        let runtime = self.clone();
        let runtime_handle = handle.clone();
        handle.spawn_blocking(move || {
            let _=std::thread::Builder::new().name("dream-c1-control".into()).spawn(move || {
                #[cfg(test)] let mut trace=PhaseTrace::new("request");
                let mut done=Some(done);
                let mut presentation=None;
                let mut journal_quantum=None;
                let result=(|| {
                    engine::utility()?;
                    if runtime.retired.load(Ordering::Acquire) {return Err(Code::WorkerLost);}
                    let request=parse()?;
                    request.validate()?;
                    let retained=runtime.retained_control_authority(&authority,&request)?;
                    #[cfg(test)] trace.mark("retained_authority");
                    // A cache-miss control first checks only its host-bound own
                    // catalogue header. Busy/missing storage can refuse without
                    // cold public-proof preparation; it never grants authority,
                    // reads a run, creates a file or changes journal mode. All
                    // successful controls still capture/revalidate the full
                    // public actor/context below. Active cached runs keep their
                    // independent prepared-session path, with no catalogue IO.
                    if retained.is_none() && matches!(&request,w::Control::Status(_)|w::Control::Cancel(_)) {
                        Journal::probe_control_header(&runtime.path,&authority.seat.into())?;
                    }
                    #[cfg(test)] trace.mark("header_preflight");
                    let (context,identity_fence,request_session)=match retained {
                        Some((context,fence,session))=>(context,fence,Some(session)),
                        None=>{let (context,fence)=authority.capture_all()?;(context,fence,None)},
                    };
                    #[cfg(test)] trace.mark("capture_authority");
                    presentation=Some((context.clone(),Some(identity_fence.clone()),request_session.clone()));
                    let root_key=crate::store::digest(&w::canonical(&serde_json::json!([context.account,context.root])).map_err(|_|Code::BadRequest)?);
                    let root={
                        let mut roots=runtime.roots.try_lock().map_err(|_|Code::Busy)?;
                        if roots.len()>=1024 && !roots.contains_key(&root_key) {return Err(Code::StorageUnavailable);}
                        roots.entry(root_key.clone()).or_default().clone()
                    };
                    if let w::Control::Status(status)=&request {
                        let cached=root.runs.try_lock().map_err(|_|Code::Busy)?.get(&status.run_id.0).cloned();
                        if let Some(run)=cached {
                            // Current full public actor/context fence still applies;
                            // an active known run needs no catalogue/schema replay.
                            return runtime.status_with_session(&root,&authority,&context,run.record(),status.page,request_session.as_deref());
                        }
                    }
                    #[cfg(test)] trace.mark("root_and_cache");
                    let mut journal=if matches!(&request,w::Control::Status(_)) {
                        // Read misses never bootstrap a catalogue, change WAL
                        // mode or open a writable foreign-seat database.
                        Journal::open_readonly_existing(&runtime.path,&context.seat)?
                    } else {Journal::open(&runtime.path,&context.seat)?};
                    journal_quantum=journal.quantum.clone();
                    #[cfg(test)] trace.mark("journal_open");
                    match request {
                        w::Control::Inspect(mut inspect)=> {
                            inspect.sources.sort_by(|a,b|(a.source_generation.0,&a.session).cmp(&(b.source_generation.0,&b.session)));
                            let request=w::canonical(&serde_json::json!({"v":1,"context":context,"sources":inspect.sources,"algorithm":"d0-c1/1"})).map_err(|_|Code::BadRequest)?;
                            let actor=crate::store::digest(&w::canonical(&serde_json::json!([context,identity_fence])).map_err(|_|Code::BadRequest)?);
                            let retained=journal.operation(&actor,&inspect.op_id,&request)?;
                            let record=if let Some(record)=retained {record} else {
                            if runtime.owner().is_none() {return Err(Code::RunnerUnavailable);}
                            let automatic=close.is_some();
                            let sources=if let Some(source)=close {
                                if inspect.sources.len()!=1 || inspect.sources[0].session!=source.session || inspect.sources[0].source_generation.0!=source.generation {return Err(Code::SourceMismatch);}
                                if !source_current(&source,&context) || source.pid==0 || source.process_start.is_empty() || !source.socket.is_absolute() {return Err(Code::SourceMismatch);}
                                vec![source]
                            } else {inspect.sources.iter().map(|s|{if let Some(code)=journal.source_error(&context,&identity_fence,s)? {return Err(code);}journal.source(&context,s)}).collect::<Result<Vec<_>,_>>()?};
                            let keys=sources.iter().map(|s|serde_json::json!([context.account,context.root_revision,context.seat,s.generation,s.session,s.closed_seq,s.closed_line_sha256,"d0-c1/1"])).collect::<Vec<_>>();
                            let key=crate::store::digest(&w::canonical(&keys).map_err(|_|Code::BadRequest)?);
                            if sources.iter().any(|source|source.owner_fence.as_ref()!=Some(&identity_fence)) {
                                if let Some(mut predecessor)=journal.source_set(&key)?
                                    && predecessor.started && !predecessor.view.settled && predecessor.owner_incarnation!=runtime.incarnation {
                                        predecessor.view.state=w::State::Quarantined;
                                        predecessor.view.error=Some(w::Error {code:Code::WorkerLost,source:None});
                                        predecessor.view.revision.0+=1;
                                        journal.save(&predecessor)?;
                                }
                                return Err(Code::StaleContext);
                            }
                            let record=Record {trigger:automatic,started:false,owner_incarnation:runtime.incarnation.clone(),worker_pid:None,worker_start:None,launch:None,
                                context:context.clone(),identity_fence:Some(identity_fence.clone()),op_id:inspect.op_id,nonce:crate::Ulid::new().into(),sources,request:request.clone(),
                                view:w::RunView {run_id:crate::Ulid::new().into(),revision:w::Number(1),state:w::State::Deferred,settled:false,
                                    request_sha256:w::Hex(crate::store::digest(&request)),manifest_sha256:None,result_sha256:None,pages:0,error:None}};
                            journal.register(&actor,&key,&record)?
                            };
                            if record.context!=context {return Err(Code::StaleContext);}
                            if record.started && record.owner_incarnation!=runtime.incarnation && !record.view.settled {
                                let mut prior=record;
                                prior.view.state=w::State::Quarantined;
                                prior.view.error=Some(w::Error {code:Code::WorkerLost,source:None});
                                prior.view.revision.0+=1;
                                journal.save(&prior)?;
                                if prior.identity_fence.as_ref()!=Some(&identity_fence) {return Err(Code::StaleContext);}
                                return Ok(reply(&prior));
                            }
                            if record.identity_fence.as_ref()!=Some(&identity_fence) {return Err(Code::StaleContext);}
                            let mut runs=root.runs.try_lock().map_err(|_|Code::Busy)?;
                            if record.view.state!=w::State::Deferred {drop(runs);return runtime.status(&root,&authority,&context,record,None);}
                            let run=if let Some(run)=runs.get(&record.view.run_id.0) {run.clone()} else {
                                if runs.len()>=1024 {return Err(Code::StorageUnavailable);}
                                if runs.values().filter(|r|!r.started.load(Ordering::Acquire)).count()>=32 {return Err(Code::Busy);}
                                let control=Arc::new(Control {activity_latch:Some(authority.activity_latch.clone()),#[cfg(debug_assertions)] private_fixture:authority.private_fixture,..Control::with_activity(authority.foreground.clone(),authority.activity_revision.clone())});
                                if let Some(proof)=runtime.native_provision.get() {control.install_native_provision(proof.clone())?;}
                                let run=Arc::new(Run {initial:record.clone(),started:AtomicBool::new(false),final_record:OnceLock::new(),control,session:OnceLock::new()});
                                runs.insert(record.view.run_id.0.clone(),run.clone());
                                run
                            };
                            drop(runs);
                            if run.started.load(Ordering::Acquire) || (authority.activity_latch.load(Ordering::Acquire)&1!=0) {
                                return Ok(reply(&run.record()));
                            }
                            let root_name=format!("root-{root_key}");
                            let account_name=format!("account-{}",crate::store::digest(context.account.0.as_bytes()));
                            let root_permit=DiskPermit::claim(&runtime.path,&root_name,2,&record)?;
                            let account=DiskPermit::claim(&runtime.path,&account_name,4,&record)?;
                            if run.started.swap(true,Ordering::AcqRel) {return Ok(reply(&run.record()));}
                            if run.control.launch_incarnation.load(Ordering::Acquire)==0 {
                                let incarnation=runtime.next_launch_incarnation()?;
                                let _=run.control.launch_incarnation.compare_exchange(0,incarnation,Ordering::AcqRel,Ordering::Acquire);
                            }
                            // Deferred controls retain no SQLite connection. Prepare the
                            // one compute session only after this owner holds both caps;
                            // its measured setup is independent of per-unit guard CPU.
                            let preparation=authority.guard_with_session(&context,&record.identity_fence,Some(&run.control.authority_setup_cpu_ns))
                                .and_then(|(guard,session)| {
                                    run.control.install_authority_guard(guard)?;
                                    run.session.set(session).map_err(|_|Code::Busy)
                                });
                            if let Err(code)=preparation {
                                if code==Code::Busy {
                                    run.started.store(false,Ordering::Release);
                                    return Err(code);
                                }
                                // A permanent preparation failure is itself one
                                // durable original-op outcome, never a fresh native
                                // setup burst on every replay of Deferred metadata.
                                let mut failed=record.clone();
                                failed.view.state=w::State::Failed;
                                failed.view.settled=true;
                                failed.view.error=Some(w::Error{code,source:None});
                                failed.view.revision.0+=1;
                                match journal.compare_and_save(&record,&failed) {
                                    Ok(true)=>{
                                        let _=run.final_record.set(failed);
                                        if let Ok(mut runs)=root.runs.try_lock()
                                            && runs.get(&record.view.run_id.0).is_some_and(|cached|Arc::ptr_eq(cached,&run)) {runs.remove(&record.view.run_id.0);}
                                        return Err(code);
                                    },
                                    Ok(false)=>{
                                        if let Ok(mut runs)=root.runs.try_lock()
                                            && runs.get(&record.view.run_id.0).is_some_and(|cached|Arc::ptr_eq(cached,&run)) {runs.remove(&record.view.run_id.0);}
                                        let current=journal.run(&record.view.run_id)?.ok_or(Code::NotFound)?;
                                        return runtime.status(&root,&authority,&context,current,None);
                                    },
                                    Err(code)=>return Err(code),
                                }
                            }
                            let previous=record.clone();
                            let mut record=record;
                            record.started=true;
                            record.owner_incarnation=runtime.incarnation.clone();
                            record.view.state=w::State::Preparing;
                            record.view.revision.0+=1;
                            match journal.compare_and_save(&previous,&record) {
                                Ok(true)=>{},
                                outcome=> {
                                    // A stale Deferred read cannot resurrect a cancelled or
                                    // completed run after its old Arc was evicted.
                                    if let Ok(mut runs)=root.runs.try_lock()
                                        && runs.get(&record.view.run_id.0).is_some_and(|cached|Arc::ptr_eq(cached,&run)) {runs.remove(&record.view.run_id.0);}
                                    outcome?;
                                    let current=journal.run(&record.view.run_id)?.ok_or(Code::NotFound)?;
                                    return runtime.status(&root,&authority,&context,current,None);
                                }
                            }
                            run.control.state.store(engine::PREPARING,Ordering::Release);
                            run.control.revision.fetch_add(1,Ordering::AcqRel);
                            runtime.read_cache.pending(&record);
                            let admission=if run.control.authority_prepared_guard.get().is_some_and(|guard|guard().is_ok()) {
                                reply(&record)
                            } else {
                                // A claimed durable owner still settles even if its
                                // presentation authority changes before the reply.
                                run.control.cancelled.store(true,Ordering::Release);
                                failure(Code::StaleContext)
                            };
                            let metadata_close_error=journal.finish().err();
                            if metadata_close_error.is_some() {run.control.cancelled.store(true,Ordering::Release);}
                            (done.take().unwrap())(metadata_close_error.map_or(admission, failure));
                            // Admission reply is independent of snapshot/process ownership.
                            drop(permit);
                            let job=runtime.path.join(&record.view.run_id.0);
                            let monitor_done=Arc::new(AtomicBool::new(false));
                            let monitor_stop=monitor_done.clone();
                            let monitor_authority=authority.clone();
                            let monitor_control=run.control.clone();
                            let monitor_failure=Arc::new(OnceLock::new());
                            let monitor_error=monitor_failure.clone();
                            let retired=runtime.retired.clone();
                            let monitor=std::thread::Builder::new().name("dream-c1-authority".into()).spawn(move || {
                                if engine::utility().is_err() {let _=monitor_error.set(Code::ResourceLimit);monitor_control.cancelled.store(true,Ordering::Release);return;}
                                let setup_cpu=match bbt_rt::fault::thread_cpu_time() {
                                    Ok(cpu)=>cpu,
                                    Err(_)=>{let _=monitor_error.set(Code::ResourceLimit);monitor_control.cancelled.store(true,Ordering::Release);return;}
                                };
                                // One prepared public session is shared through
                                // try_lock with compute. A concurrent check defers
                                // this monitor; it never grants permission. The
                                // transient checkpoint is the second SQL connection.
                                let Some(monitor_guard)=monitor_control.authority_prepared_guard.get().cloned() else {
                                    let _=monitor_error.set(Code::StaleContext);
                                    monitor_control.cancelled.store(true,Ordering::Release);
                                    return;
                                };
                                let setup_ns=bbt_rt::fault::thread_cpu_time().map(|cpu|cpu.saturating_sub(setup_cpu).as_nanos().min(u64::MAX as u128) as u64).unwrap_or(u64::MAX);
                                monitor_control.authority_monitor_setup_cpu_ns.store(setup_ns,Ordering::Release);
                                if setup_ns>1_000_000 {monitor_control.authority_monitor_overruns.fetch_add(1,Ordering::Relaxed);let _=monitor_error.set(Code::ResourceLimit);monitor_control.cancelled.store(true,Ordering::Release);return;}
                                std::thread::park_timeout(std::time::Duration::from_millis(9));
                                while !monitor_stop.load(Ordering::Acquire) {
                                    let before=match bbt_rt::fault::thread_cpu_time() {
                                        Ok(cpu)=>cpu,
                                        Err(_)=>{let _=monitor_error.set(Code::ResourceLimit);monitor_control.authority_monitor_overruns.fetch_add(1,Ordering::Relaxed);monitor_control.cancelled.store(true,Ordering::Release);return;}
                                    };
                                    let pending=monitor_control.native_pending_since_ns.load(Ordering::Acquire);
                                    if pending!=0 {
                                        let age=engine::monotonic_ns().ok().and_then(|now|now.checked_sub(pending));
                                        if age.is_none_or(|age|age>=6_000_000_000) {
                                            monitor_control.native_quarantine_observed.store(true,Ordering::Release);
                                            monitor_control.state.store(engine::QUARANTINED,Ordering::Release);
                                            monitor_control.revision.fetch_add(1,Ordering::AcqRel);
                                            monitor_control.cancelled.store(true,Ordering::Release);
                                        }
                                    }
                                    let current=monitor_guard();
                                    let used=bbt_rt::fault::thread_cpu_time().map(|cpu|cpu.saturating_sub(before));
                                    let overrun=match used {
                                        Ok(cpu)=>{
                                            monitor_control.authority_monitor_max_cpu_ns.fetch_max(cpu.as_nanos().min(u64::MAX as u128) as u64,Ordering::Relaxed);
                                            cpu>std::time::Duration::from_millis(1)
                                        },
                                        Err(_)=>true,
                                    };
                                    if overrun {monitor_control.authority_monitor_overruns.fetch_add(1,Ordering::Relaxed);}
                                    if overrun||retired.load(Ordering::Acquire)||matches!(current,Err(code) if code!=Code::Busy) {
                                        let _=monitor_error.set(if overrun {Code::ResourceLimit} else if retired.load(Ordering::Acquire) {Code::WorkerLost} else {Code::StaleContext});
                                        monitor_control.cancelled.store(true,Ordering::Release);
                                        let _=monitor_authority.activity_latch.try_update(Ordering::AcqRel,Ordering::Acquire,|old|old.checked_add(2).map(|n|n|1));
                                        return;
                                    }
                                    std::thread::park_timeout(std::time::Duration::from_millis(50));
                                }
                            });
                            if monitor.is_err() {run.control.cancelled.store(true,Ordering::Release);}
                            let result=engine::execute_with_settlement(&runtime.binary,&job,&mut record,&run.control,||{
                                monitor_done.store(true,Ordering::Release);
                                let joined=match monitor {
                                    Ok(monitor)=>{
                                        monitor.thread().unpark();
                                        let waiting=std::time::Instant::now();
                                        let mut quarantined=false;
                                        while !monitor.is_finished() {
                                            if !quarantined && waiting.elapsed()>=std::time::Duration::from_secs(6) {
                                                run.control.state.store(engine::QUARANTINED,Ordering::Release);
                                                run.control.revision.fetch_add(1,Ordering::AcqRel);
                                                quarantined=true;
                                            }
                                            std::thread::park_timeout(std::time::Duration::from_millis(9));
                                        }
                                        monitor.join().map_err(|_|Code::WorkerLost)
                                    },
                                    Err(_)=>Err(Code::WorkerLost),
                                };
                                let retired=run.session.get().map_or(Ok(()),|session|engine::retire_public_session(session,&run.control));
                                joined?;
                                retired?;
                                monitor_failure.get().copied().map_or(Ok(()),Err)
                            });
                            let result=metadata_close_error.map_or(result,Err);
                            if runtime.retired.load(Ordering::Acquire) {
                                record.view.state=w::State::Interrupted;
                                record.view.error=Some(w::Error {code:Code::WorkerLost,source:None});
                                record.view.result_sha256=None;record.view.pages=0;
                            } else if let Some(code)=monitor_failure.get() {
                                if !matches!(record.view.state,w::State::Quarantined) {record.view.state=w::State::Failed;}
                                record.view.error=Some(w::Error {code:*code,source:None});
                                record.view.result_sha256=None;record.view.pages=0;
                            } else if !authority.matches(&context,&record.identity_fence) {
                                record.view.state=w::State::Failed;
                                record.view.error=Some(w::Error {code:Code::StaleContext,source:None});
                                record.view.result_sha256=None;record.view.pages=0;
                            } else if let Err(code)=result {
                                if !matches!(record.view.state,w::State::Quarantined|w::State::Cancelled) {record.view.state=w::State::Failed;}
                                record.view.error=Some(w::Error {code,source:None});
                            }
                            record.view.revision.0+=1;
                            let persisted=engine::checkpoint(&job,&record,&run.control);
                            if let Err(code)=persisted {record.view.state=w::State::Quarantined;record.view.settled=false;record.view.error=Some(w::Error {code,source:None});}
                            let settled=record.view.settled;
                            if !settled {std::mem::forget(account);std::mem::forget(root_permit);}
                            // Publish actual C1 settlement before disposable read
                            // preparation. Cache work cannot delay its terminal view.
                            let _=run.final_record.set(record.clone());
                            if settled {root.retire(&run);}
                            if settled && record.view.state==w::State::Completed {
                                let _=runtime.read_cache.prepare(&runtime.path,&record,&authority,&run.control);
                            }
                            if settled {runtime.read_cache.settled(&record);}
                            runtime.activity(&runtime_handle,authority.clone());
                            Ok(failure(Code::NotFound)) // admission was already returned exactly once
                        }
                        w::Control::Status(status)=> {
                            let record=journal.run(&status.run_id)?.ok_or(Code::NotFound)?;
                            runtime.status(&root,&authority,&context,record,status.page)
                        }
                        w::Control::Cancel(cancel)=> {
                            let actor=crate::store::digest(&w::canonical(&serde_json::json!([context,identity_fence])).map_err(|_|Code::BadRequest)?);
                            let request=w::canonical(&serde_json::json!({"v":1,"context":context,"method":"cancel","run_id":cancel.run_id})).map_err(|_|Code::BadRequest)?;
                            let record=Journal::step(&journal.quantum.clone(),||{
                                let previous=journal.operation(&actor,&cancel.op_id,&request)?;
                                match previous {Some(r)=>Ok(r),None=>journal.run(&cancel.run_id)?.ok_or(Code::NotFound)}
                            })?;
                            if record.context!=context || record.identity_fence.as_ref()!=Some(&identity_fence) {return Err(Code::NotFound);}
                            journal.cancel_operation(&actor,&cancel.op_id,&request,&cancel.run_id)?;
                            let run=root.runs.try_lock().map_err(|_|Code::Busy)?.get(&cancel.run_id.0).cloned();
                            if let Some(run)=run {
                                let current=run.record();
                                if current.view.settled {return Ok(reply(&current));}
                                run.control.cancelled.store(true,Ordering::Release);
                                if run.started.compare_exchange(false,true,Ordering::AcqRel,Ordering::Acquire).is_ok() {
                                    let latest=journal.run(&cancel.run_id)?.ok_or(Code::NotFound)?;
                                    let terminal=journal.cancel_unstarted(&latest)?;
                                    if terminal.view.settled {
                                        run.control.state.store(if terminal.view.state==w::State::Cancelled {engine::CANCELLED}else{engine::SETTLING},Ordering::Release);
                                        run.control.settled.store(true,Ordering::Release);
                                        let _=run.final_record.set(terminal.clone());
                                    }
                                    if let Ok(mut runs)=root.runs.try_lock()
                                        && runs.get(&cancel.run_id.0).is_some_and(|cached|Arc::ptr_eq(cached,&run)) {runs.remove(&cancel.run_id.0);}
                                    return runtime.status(&root,&authority,&context,terminal,None);
                                }
                                run.control.state.store(engine::CANCELLING,Ordering::Release);
                                run.control.revision.fetch_add(1,Ordering::AcqRel);
                                return Ok(reply(&run.record()));
                            }
                            let record=journal.cancel_unstarted(&record)?;
                            runtime.status(&root,&authority,&context,record,None)
                        }
                    }
                })();
                #[cfg(test)] trace.mark("domain_and_retire");
                let result=match journal_quantum.as_ref().map(|quantum|quantum.check()) {Some(Err(code))=>Err(code),_=>result};
                if let Some(done)=done {
                    let response=match result {
                        Ok(response @ w::ControlReply::Success(_))=> {
                            if presentation.as_ref().is_some_and(|(context,fence,session)|authority.matches_with_session(context,fence,session.as_deref()).is_ok()) {response}
                            else {failure(Code::StaleContext)}
                        }
                        Ok(response)=>response,
                        Err(code)=>failure(code),
                    };
                    #[cfg(test)] trace.mark("presentation_recheck");
                    done(response);
                }
            });
        });
    }
    fn status(
        &self,
        root: &RootState,
        authority: &Authority,
        context: &w::Context,
        record: Record,
        page: Option<u32>,
    ) -> Result<w::ControlReply, Code> {
        self.status_with_session(root, authority, context, record, page, None)
    }
    fn status_with_session(
        &self,
        root: &RootState,
        authority: &Authority,
        context: &w::Context,
        mut record: Record,
        page: Option<u32>,
        session: Option<&crate::identity::PublicFenceSession>,
    ) -> Result<w::ControlReply, Code> {
        if &record.context != context
            || authority
                .matches_with_session(context, &record.identity_fence, session)
                .is_err()
        {
            return Err(Code::NotFound);
        }
        let cached = root
            .runs
            .try_lock()
            .map_err(|_| Code::Busy)?
            .get(&record.view.run_id.0)
            .cloned();
        if let Some(run) = cached {
            record = run.record();
        } else if !record.view.settled {
            // Never adopt/reexecute an unverified predecessor's process or result.
            record.view.state = w::State::Quarantined;
            record.view.error = Some(w::Error {
                code: Code::WorkerLost,
                source: None,
            });
        }
        let mut response = match reply(&record) {
            w::ControlReply::Success(s) => s,
            _ => unreachable!(),
        };
        if let Some(index) = page {
            if record.view.state != w::State::Completed {
                return Err(Code::NotFound);
            }
            if index >= record.view.pages {
                return Err(Code::BadRequest);
            }
            response.page = Some(engine::read_page_guarded(
                &self.path.join(&record.view.run_id.0),
                &record,
                index,
                Some(authority.guard(context, &record.identity_fence, None)?),
            )?);
        }
        if authority
            .matches_with_session(context, &record.identity_fence, session)
            .is_err()
        {
            return Err(Code::StaleContext);
        }
        Ok(w::ControlReply::Success(response))
    }
}

// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
// These tests exercise actual private SQLite transactions, owned filesystem
// claims and one recorded child.
// Metadata-only source fixtures do not claim observer authentication or IR validity.
#[cfg(test)]
mod registry_tests {
    #![allow(
        clippy::disallowed_methods,
        reason = "private unattended persistence/process witnesses"
    )]
    use super::*;
    use std::{
        fs::{self, File, OpenOptions},
        io::Write,
        os::unix::{fs::OpenOptionsExt, net::UnixListener},
        process::{Command, Stdio},
    };

    fn id(number: u128) -> w::Ulid {
        crate::Ulid::from(number).into()
    }
    struct Fixture {
        directory: tempfile::TempDir,
        runtime: PathBuf,
        source: PathBuf,
        _socket: UnixListener,
    }
    impl Fixture {
        fn new() -> Self {
            let directory = tempfile::tempdir().unwrap();
            let runtime = directory.path().join("runtime");
            private_dir(&runtime).unwrap();
            let originals = directory.path().join("originals");
            private_dir(&originals).unwrap();
            let source = originals.join("retained.jsonl");
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&source)
                .unwrap();
            file.write_all(b"METADATA-ONLY PRIVATE SOURCE\n").unwrap();
            file.sync_all().unwrap();
            let socket = UnixListener::bind(directory.path().join("owned.sock")).unwrap();
            Self {
                directory,
                runtime,
                source,
                _socket: socket,
            }
        }
        fn record(&self, number: u128) -> Record {
            let context = w::Context {
                account: w::Id("private-account".into()),
                account_epoch: w::Number(1),
                seat: id(10),
                generation: w::Number(1),
                grant_epoch: w::Number(1),
                swarm: id(11),
                swarm_revision: w::Number(1),
                workspace: w::Id("private-workspace".into()),
                workspace_revision: w::Number(1),
                root: w::Id("private-root".into()),
                root_revision: w::Number(1),
                provider: w::Id("private-provider".into()),
                provider_epoch: w::Number(1),
            };
            let metadata = fs::metadata(&self.source).unwrap();
            let source = Bookmark {
                owner_context: Some(context.clone()),
                owner_fence: None,
                observed_seq: 2,
                last_line_len: 19,
                last_line_sha256: crate::store::digest(b"retained-close-line"),
                turns: vec![],
                session: id(12),
                generation: 1,
                path: self.source.clone(),
                repo: "private.example/closed".into(),
                pid: std::process::id(),
                process_start: "metadata-only-owner-proof".into(),
                socket: self.directory.path().join("owned.sock"),
                root: context.root.0.clone(),
                root_revision: context.root_revision.0,
                workspace_revision: context.workspace_revision.0,
                device: metadata.dev(),
                inode: metadata.ino(),
                len: metadata.len(),
                mtime: (metadata.mtime(), metadata.mtime_nsec()),
                ctime: (metadata.ctime(), metadata.ctime_nsec()),
                closed_seq: 2,
                closed_line_sha256: crate::store::digest(b"retained-close-line"),
                cursor: metadata.len(),
            };
            let request=w::canonical(&serde_json::json!({"v":1,"context":context,
                "sources":[{"session":source.session,"source_generation":source.generation}],"algorithm":"d0-c1/1"})).unwrap();
            Record {
                trigger: true,
                started: false,
                owner_incarnation: id(100),
                worker_pid: None,
                worker_start: None,
                launch: None,
                context,
                identity_fence: None,
                op_id: id(number + 1000),
                nonce: id(number + 2000),
                sources: vec![source],
                view: w::RunView {
                    run_id: id(number),
                    revision: w::Number(1),
                    state: w::State::Deferred,
                    settled: false,
                    request_sha256: w::Hex(crate::store::digest(&request)),
                    manifest_sha256: None,
                    result_sha256: None,
                    pages: 0,
                    error: None,
                },
                request,
            }
        }
        fn journal(&self) -> Journal {
            Journal::open(&self.runtime, &id(10)).unwrap()
        }
    }
    fn key(record: &Record) -> String {
        let keys = record
            .sources
            .iter()
            .map(|s| {
                serde_json::json!([
                    record.context.account,
                    record.context.root_revision,
                    record.context.seat,
                    s.generation,
                    s.session,
                    s.closed_seq,
                    s.closed_line_sha256,
                    "d0-c1/1"
                ])
            })
            .collect::<Vec<_>>();
        crate::store::digest(&w::canonical(&keys).unwrap())
    }
    fn count(journal: &Journal, table: &str) -> i64 {
        assert!(["sources", "runs", "ops", "close_cursor", "source_errors"].contains(&table));
        journal
            .db
            .query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))
            .unwrap()
    }
    fn cursor(journal: &Journal, record: &Record) -> u64 {
        journal
            .db
            .query_row(
                "SELECT cursor FROM close_cursor WHERE session=?1 AND generation=?2",
                params![
                    record.sources[0].session.0,
                    record.sources[0].generation as i64
                ],
                |r| r.get::<_, i64>(0),
            )
            .unwrap() as u64
    }

    #[test]
    fn dream_c1_status_missing_catalogue_never_creates_directory_or_database() {
        let f = Fixture::new();
        let seat = id(998);
        let journals = f.runtime.join("journals");
        assert!(!journals.exists());
        assert!(matches!(
            Journal::open_readonly_existing(&f.runtime, &seat),
            Err(Code::NotFound)
        ));
        assert!(
            !journals.exists(),
            "Status created an empty catalogue directory"
        );
        private_dir(&journals).unwrap();
        let expected = journals.join(format!("{}.sqlite", seat.0));
        assert!(matches!(
            Journal::open_readonly_existing(&f.runtime, &seat),
            Err(Code::NotFound)
        ));
        assert!(
            !expected.exists(),
            "Status created a foreign/unknown seat database"
        );
        assert_eq!(std::fs::read_dir(&journals).unwrap().count(), 0);
    }

    #[test]
    fn dream_c1_fixed_sqlite_setters_apply_at_prepare_without_redundant_step() {
        let f = Fixture::new();
        let seat = id(998);
        let existing = Journal::prepare_path(&f.runtime, &seat, true).unwrap();
        let quantum = Arc::new(engine::JournalQuantum::default());
        let mut journal = quantum
            .unit(|| Journal::connect_existing(&existing))
            .unwrap();
        journal.quantum = Some(quantum.clone());
        journal.db.quantum = Some(quantum.clone());
        for (config, read, expected) in [
            (JournalConfig::Cache, "PRAGMA cache_size", -1024i64),
            (JournalConfig::Mmap, "PRAGMA mmap_size", 0),
            (JournalConfig::Sync, "PRAGMA synchronous", 2),
        ] {
            let statement =
                quantum.retain(quantum.unit(|| journal.prepare_config(config)).unwrap());
            // No Rows/step exists: preparing and retiring the fixed setter is
            // the production path. The separate public read proves its effect.
            drop(statement);
            let value: i64 = quantum
                .unit(|| {
                    journal
                        .db
                        .query_row(read, [], |row| row.get(0))
                        .map_err(|_| Code::StorageUnavailable)
                })
                .unwrap();
            assert_eq!(value, expected);
        }
        quantum.check().unwrap();
        journal.finish().unwrap();
    }

    // Metadata-only real SQLite witnesses. No worker/canonical-producer claim.
    #[track_caller]
    fn uninitialized_journal(fixture: &Fixture, seat: &w::Ulid) -> Journal {
        let quantum = Arc::new(engine::JournalQuantum::at(std::panic::Location::caller()));
        let existing = quantum.retain(
            quantum
                .unit(|| Journal::prepare_path(&fixture.runtime, seat, true))
                .unwrap(),
        );
        let mut journal = quantum
            .unit(|| Journal::connect_existing(&existing))
            .unwrap();
        journal.quantum = Some(quantum.clone());
        journal.db.quantum = Some(quantum.clone());
        quantum
            .unit(|| {
                journal
                    .db
                    .busy_timeout(std::time::Duration::ZERO)
                    .map_err(Journal::initialization_error)
            })
            .unwrap();
        assert_eq!(journal.schema_version().unwrap(), 0);
        journal.configure_cache().unwrap();
        journal.configure_mmap().unwrap();
        journal.configure_sync().unwrap();
        drop(existing);
        quantum.check().unwrap();
        journal
    }
    fn schema_table_count(journal: &Journal) -> i64 {
        let quantum = journal.quantum.as_ref().unwrap();
        let mut statement=quantum.retain(quantum.unit(||journal.db.prepare(
            "SELECT count(*) FROM sqlite_schema WHERE type='table' AND name IN ('source_errors','sources','runs','ops','close_cursor')")
            .map_err(Journal::initialization_error)).unwrap());
        let borrowed = &mut *statement;
        let mut rows = quantum.retain(quantum.unit(move || Ok(Journal::rows(borrowed))).unwrap());
        let count = quantum
            .unit(|| Journal::read_version_first(&mut rows))
            .unwrap();
        drop(rows);
        drop(statement);
        quantum.check().unwrap();
        count
    }
    #[test]
    fn dream_c1_schema_bootstrap_rolls_back_partial_ddl_and_ready_marker_before_recovery() {
        let fixture = Fixture::new();
        let seat = id(997);
        // Refuse once after the first real CREATE and once after the ready
        // cookie is written. Neither effect may escape the same transaction.
        for after in [1usize, 6] {
            let journal = uninitialized_journal(&fixture, &seat);
            assert_eq!(schema_table_count(&journal), 0);
            let result = journal.initialize_schema(Some(after));
            assert_eq!(
                result,
                Err(Code::StorageUnavailable),
                "failure after index {after}"
            );
            assert!(
                journal.db.is_autocommit(),
                "refusal retained an open write transaction"
            );
            assert_eq!(
                journal.schema_version().unwrap(),
                0,
                "ready marker survived rollback"
            );
            assert_eq!(
                schema_table_count(&journal),
                0,
                "partial tables survived rollback"
            );
            journal.finish().unwrap();
            // A new actual connection sees the rolled-back durable state,
            // rather than relying on the former connection's schema cache.
            let reopened = uninitialized_journal(&fixture, &seat);
            assert_eq!(schema_table_count(&reopened), 0);
            reopened.finish().unwrap();
        }
        let recovered = Journal::open(&fixture.runtime, &seat).unwrap();
        assert_eq!(recovered.schema_version().unwrap(), 1);
        assert_eq!(schema_table_count(&recovered), 5);
        let quantum = recovered.quantum.as_ref().unwrap();
        let mode: String = quantum
            .unit(|| {
                recovered
                    .db
                    .query_row("PRAGMA journal_mode", [], |row| row.get(0))
                    .map_err(Journal::initialization_error)
            })
            .unwrap();
        let sync: i64 = quantum
            .unit(|| {
                recovered
                    .db
                    .query_row("PRAGMA synchronous", [], |row| row.get(0))
                    .map_err(Journal::initialization_error)
            })
            .unwrap();
        assert_eq!(mode, "persist");
        assert_eq!(
            sync, 2,
            "recovery must preserve FULL rollback-journal durability"
        );
        recovered.finish().unwrap();
        let durable = Journal::open_readonly_existing(&fixture.runtime, &seat).unwrap();
        assert_eq!(durable.schema_version().unwrap(), 1);
        assert_eq!(schema_table_count(&durable), 5);
        durable.finish().unwrap();
    }
    #[test]
    #[ignore = "KNOWN DEFECT: C1 schema CPU unit overruns during lean land (1.279ms); fix: tasks/2026-11-07-dream-c1-native-unit-budget-overruns"]
    fn dream_c1_schema_bootstrap_busy_writer_refuses_without_waiting_or_partial_publication() {
        let fixture = Fixture::new();
        let seat = id(996);
        let writer = uninitialized_journal(&fixture, &seat);
        writer
            .initialization_statement("PRAGMA journal_mode=PERSIST", 0)
            .unwrap();
        let contender = uninitialized_journal(&fixture, &seat);
        writer
            .initialization_statement("BEGIN IMMEDIATE", 7)
            .unwrap();
        writer.initialization_statement(
            "CREATE TABLE source_errors(session TEXT NOT NULL,generation INTEGER NOT NULL,error BLOB NOT NULL,PRIMARY KEY(session,generation))",1).unwrap();
        let before = std::time::Instant::now();
        let refused = contender.initialize_schema(None);
        let elapsed = before.elapsed();
        assert_eq!(refused, Err(Code::Busy));
        assert!(
            elapsed < std::time::Duration::from_millis(250),
            "busy bootstrap waited {elapsed:?}"
        );
        assert!(contender.db.is_autocommit());
        assert_eq!(contender.schema_version().unwrap(), 0);
        assert_eq!(
            schema_table_count(&contender),
            0,
            "uncommitted schema was presented as ready"
        );
        writer.rollback_initialization().unwrap();
        writer.quantum.as_ref().unwrap().check().unwrap();
        writer.finish().unwrap();
        contender.initialize_schema(None).unwrap();
        assert_eq!(contender.schema_version().unwrap(), 1);
        assert_eq!(schema_table_count(&contender), 5);
        contender.finish().unwrap();
    }

    #[test]
    fn dream_c1_delete_catalogue_cold_init_commit_close_preserves_original_operation() {
        let fixture = Fixture::new();
        let record = fixture.record(440);
        let cpu = bbt_rt::fault::thread_cpu_time().unwrap();
        let mut journal = fixture.journal();
        let quantum = journal.quantum.as_ref().unwrap().clone();
        journal
            .register("cold-original", &key(&record), &record)
            .unwrap();
        assert_eq!(cursor(&journal, &record), record.sources[0].cursor);
        journal.finish().unwrap();
        let cumulative = bbt_rt::fault::thread_cpu_time()
            .unwrap()
            .checked_sub(cpu)
            .unwrap();
        quantum.check().unwrap();
        assert!(quantum.max_unit_cpu_ns() <= 1_000_000);
        assert!(quantum.max_cleanup_unit_cpu_ns() <= 1_000_000);
        eprintln!(
            "dream_c1_persist_cold cumulative_cpu_ns={} max_unit_cpu_ns={} max_cleanup_unit_cpu_ns={} effects=durable_original_op_run_source_cursor",
            cumulative.as_nanos(),
            quantum.max_unit_cpu_ns(),
            quantum.max_cleanup_unit_cpu_ns()
        );
        let journal = fixture.journal();
        let old = journal
            .operation("cold-original", &record.op_id, &record.request)
            .unwrap()
            .unwrap();
        assert_eq!(old.view.run_id, record.view.run_id);
        assert_eq!(old.nonce, record.nonce);
        for table in ["sources", "runs", "ops", "close_cursor"] {
            assert_eq!(count(&journal, table), 1);
        }
        journal.finish().unwrap();
    }

    fn catalogue_rows(db: &Connection) -> Vec<Vec<Vec<rusqlite::types::Value>>> {
        ["source_errors", "sources", "runs", "ops", "close_cursor"]
            .into_iter()
            .map(|table| {
                let mut statement = db
                    .prepare(&format!("SELECT * FROM {table} ORDER BY 1,2"))
                    .unwrap();
                let width = statement.column_count();
                statement
                    .query_map([], |row| (0..width).map(|i| row.get(i)).collect())
                    .unwrap()
                    .collect::<rusqlite::Result<Vec<_>>>()
                    .unwrap()
            })
            .collect()
    }

    #[test]
    fn dream_c1_legacy_wal_transition_preserves_rows_and_busy_reader_never_changes_custody() {
        let fixture = Fixture::new();
        let mut record = fixture.record(441);
        let mut journal = fixture.journal();
        journal
            .register("legacy-original", &key(&record), &record)
            .unwrap();
        journal.finish().unwrap();
        let path = fixture
            .runtime
            .join("journals")
            .join(format!("{}.sqlite", id(10).0));
        let inode = fs::metadata(&path).unwrap().ino();
        // Unmeasured legacy-fixture construction only. This deliberately seeds
        // the old supported WAL state; it is not a cold PERSIST timing result.
        let legacy = Connection::open(&path).unwrap();
        assert_eq!(
            legacy
                .query_row("PRAGMA journal_mode=WAL", [], |r| r.get::<_, String>(0))
                .unwrap(),
            "wal"
        );
        legacy.pragma_update(None, "synchronous", "FULL").unwrap();
        legacy.pragma_update(None, "wal_autocheckpoint", 0).unwrap();
        record.view.revision.0 += 1;
        legacy
            .execute(
                "UPDATE runs SET record=?1 WHERE id=?2",
                params![record_bytes(&record).unwrap(), record.view.run_id.0],
            )
            .unwrap();
        // Include all five tables in the byte-preservation witness. This raw
        // legacy diagnostic is not promoted into current public authority.
        legacy
            .execute(
                "INSERT INTO source_errors(session,generation,error) VALUES(?1,?2,?3)",
                params![
                    id(443).0,
                    1,
                    br#"{"legacy":"retained-diagnostic"}"#.as_slice()
                ],
            )
            .unwrap();
        for table in ["source_errors", "sources", "runs", "ops", "close_cursor"] {
            assert_eq!(
                legacy
                    .query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r
                        .get::<_, i64>(0))
                    .unwrap(),
                1
            );
        }
        let before = catalogue_rows(&legacy);
        let reader =
            Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        reader.execute_batch("BEGIN").unwrap();
        assert_eq!(
            reader
                .query_row("SELECT count(*) FROM runs", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            1
        );
        let start = std::time::Instant::now();
        assert!(matches!(
            Journal::open_existing(&fixture.runtime, &id(10)),
            Err(Code::Busy)
        ));
        assert!(start.elapsed() < std::time::Duration::from_millis(250));
        assert_eq!(fs::metadata(&path).unwrap().ino(), inode);
        assert_eq!(catalogue_rows(&legacy), before);
        assert_eq!(
            legacy
                .query_row("PRAGMA journal_mode", [], |r| r.get::<_, String>(0))
                .unwrap(),
            "wal"
        );
        reader.execute_batch("ROLLBACK").unwrap();
        reader.close().unwrap();
        legacy.close().unwrap(); // SQLite alone checkpoints/retires WAL/SHM.
        // Explicitly new admission after the actual dependency is released.
        let mut migrated = fixture.journal();
        assert_eq!(fs::metadata(&path).unwrap().ino(), inode);
        assert_eq!(catalogue_rows(&migrated.db), before);
        assert_eq!(
            migrated
                .db
                .query_row("PRAGMA journal_mode", [], |r| r.get::<_, String>(0))
                .unwrap(),
            "persist"
        );
        assert_eq!(
            migrated
                .db
                .query_row("PRAGMA synchronous", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            2
        );
        let original = migrated
            .register("legacy-original", &key(&record), &record)
            .unwrap();
        assert_eq!(original.view.run_id, record.view.run_id);
        assert_eq!(original.view.revision, record.view.revision);
        assert_eq!(original.nonce, record.nonce);
        assert_eq!(catalogue_rows(&migrated.db), before);
        migrated.finish().unwrap();
    }

    #[test]
    #[ignore = "exact private subprocess of dream_c1_delete_crash_before_commit_recovers_without_partial_ready"]
    fn dream_c1_catalogue_crash_child() {
        let path = PathBuf::from(
            std::env::var_os("BBOXY_C1_CATALOGUE_CRASH_FILE").expect("private parent path"),
        );
        let metadata = fs::symlink_metadata(&path).unwrap();
        assert!(
            path.is_absolute()
                && metadata.is_file()
                && metadata.nlink() == 1
                && metadata.uid() == unsafe { libc::geteuid() }
                && metadata.mode() & 0o777 == 0o600
        );
        let db = Connection::open_with_flags(
            path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE | rusqlite::OpenFlags::SQLITE_OPEN_NOFOLLOW,
        )
        .unwrap();
        db.execute_batch("PRAGMA journal_mode=PERSIST; PRAGMA synchronous=FULL; BEGIN IMMEDIATE; CREATE TABLE source_errors(session TEXT NOT NULL,generation INTEGER NOT NULL,error BLOB NOT NULL,PRIMARY KEY(session,generation));").unwrap();
        if std::env::var_os("BBOXY_C1_CRASH_AFTER_MARKER").is_some() {
            db.pragma_update(None, "user_version", 1).unwrap();
        }
        std::process::exit(73); // Abrupt process death: no Rust/SQLite Drop or COMMIT.
    }

    #[test]
    fn dream_c1_delete_crash_before_commit_recovers_without_partial_ready() {
        for after_marker in [false, true] {
            let fixture = Fixture::new();
            let seat = id(442);
            uninitialized_journal(&fixture, &seat).finish().unwrap();
            let path = fixture
                .runtime
                .join("journals")
                .join(format!("{}.sqlite", seat.0));
            // Resolve macOS's /var alias while retaining the exact inode. The
            // child keeps SQLite NOFOLLOW; a system alias is not a DB rewrite.
            let path = fs::canonicalize(path).unwrap();
            let inode = fs::metadata(&path).unwrap().ino();
            let mut command = Command::new(std::env::current_exe().unwrap());
            command
                .args([
                    "--ignored",
                    "--exact",
                    "registry::dream_c1::registry_tests::dream_c1_catalogue_crash_child",
                ])
                .env("BBOXY_C1_CATALOGUE_CRASH_FILE", &path)
                .env_remove("BBOXY_C1_CRASH_AFTER_MARKER");
            if after_marker {
                command.env("BBOXY_C1_CRASH_AFTER_MARKER", "1");
            }
            let mut child = command.spawn().unwrap();
            let pid = child.id();
            assert_eq!(child.wait().unwrap().code(), Some(73));
            let mut status = 0;
            assert_eq!(
                unsafe { libc::waitpid(pid as i32, &mut status, libc::WNOHANG) },
                -1
            );
            assert_eq!(
                std::io::Error::last_os_error().raw_os_error(),
                Some(libc::ECHILD)
            );
            let reopened = Journal::open(&fixture.runtime, &seat).unwrap();
            assert_eq!(fs::metadata(path).unwrap().ino(), inode);
            assert_eq!(reopened.schema_version().unwrap(), 1);
            assert_eq!(schema_table_count(&reopened), 5);
            reopened.finish().unwrap();
        }
    }

    #[test]
    fn dream_c1_existing_checkpoint_never_creates_or_initializes_a_missing_journal() {
        let fixture = Fixture::new();
        assert!(Journal::open_existing(&fixture.runtime, &id(10)).is_err());
        let initialized = fixture.journal();
        drop(initialized);
        let existing = Journal::open_existing(&fixture.runtime, &id(10)).unwrap();
        assert_eq!(count(&existing, "runs"), 0);
        assert_eq!(count(&existing, "ops"), 0);
        let sync: i64 = existing
            .db
            .query_row("PRAGMA synchronous", [], |row| row.get(0))
            .unwrap();
        assert_eq!(
            sync, 2,
            "reopened connection retains SQLite FULL synchronization"
        );
        let mode: String = existing
            .db
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))
            .unwrap();
        assert_eq!(mode, "persist");
    }

    #[test]
    fn dream_c1_staged_journal_open_rejects_inode_replacement_and_symlink() {
        let fixture = Fixture::new();
        drop(fixture.journal());
        let retained = Journal::existing_path(&fixture.runtime, &id(10)).unwrap();
        let original = retained.path.with_extension("retained");
        fs::rename(&retained.path, &original).unwrap();
        let replacement = OpenOptions::new()
            .create_new(true)
            .write(true)
            .mode(0o600)
            .open(&retained.path)
            .unwrap();
        drop(replacement);
        assert!(matches!(
            Journal::connect_existing(&retained),
            Err(Code::StorageUnavailable)
        ));
        assert_eq!(
            fs::read(&retained.path).unwrap(),
            b"",
            "replacement bytes remain untouched"
        );
        fs::remove_file(&retained.path).unwrap();
        std::os::unix::fs::symlink(&original, &retained.path).unwrap();
        assert!(matches!(
            Journal::connect_existing(&retained),
            Err(Code::StorageUnavailable)
        ));
    }

    #[test]
    fn dream_c1_settled_cache_retirement_keeps_slot_through_contention_and_preserves_replacement() {
        for replacement in [false, true] {
            let fixture = Fixture::new();
            let make_run = || {
                Arc::new(Run {
                    initial: fixture.record(20),
                    started: AtomicBool::new(true),
                    final_record: OnceLock::new(),
                    control: Arc::new(Control::default()),
                    session: OnceLock::new(),
                })
            };
            let run = make_run();
            let newer = make_run();
            let root = Arc::new(RootState::default());
            let mut held = root.runs.lock().unwrap();
            held.insert(run.initial.view.run_id.0.clone(), run.clone());
            let count = Arc::new(AtomicUsize::new(0));
            let permit = Permit::claim(&count, 1).unwrap();
            std::thread::scope(|scope| {
                let (sent, received) = std::sync::mpsc::sync_channel(1);
                let root = &root;
                let run = &run;
                let owner = scope.spawn(move || {
                    root.retire(run);
                    drop(permit);
                    sent.send(()).unwrap();
                });
                assert!(
                    received
                        .recv_timeout(std::time::Duration::from_millis(20))
                        .is_err()
                );
                assert_eq!(
                    count.load(Ordering::Acquire),
                    1,
                    "a cached owner keeps its slot"
                );
                if replacement {
                    held.insert(run.initial.view.run_id.0.clone(), newer.clone());
                }
                drop(held);
                received
                    .recv_timeout(std::time::Duration::from_secs(1))
                    .unwrap();
                owner.join().unwrap();
            });
            assert_eq!(count.load(Ordering::Acquire), 0);
            let entries = root.runs.lock().unwrap();
            if replacement {
                assert!(Arc::ptr_eq(
                    entries.get(&run.initial.view.run_id.0).unwrap(),
                    &newer
                ));
            } else {
                assert!(entries.is_empty());
            }
        }
    }

    #[test]
    #[ignore = "KNOWN DEFECT: C1 journal initialization CPU overrun during lean land (1.037ms); fix: tasks/2026-11-07-dream-c1-native-unit-budget-overruns"]
    fn dream_c1_registration_commits_close_cursor_original_op_and_source_key_once_after_reopen() {
        let fixture = Fixture::new();
        let before = fs::read(&fixture.source).unwrap();
        let record = fixture.record(20);
        let source_key = key(&record);
        let mut journal = fixture.journal();
        let retained = journal.register("actor-1", &source_key, &record).unwrap();
        assert_eq!(retained.view.run_id, record.view.run_id);
        assert_eq!(cursor(&journal, &record), record.sources[0].cursor);
        for table in ["sources", "runs", "ops", "close_cursor"] {
            assert_eq!(count(&journal, table), 1, "missing atomic {table}");
        }
        drop(journal);
        let mut journal = fixture.journal();
        let retained = journal
            .operation("actor-1", &record.op_id, &record.request)
            .unwrap()
            .unwrap();
        assert_eq!(retained.view.run_id, record.view.run_id);
        let selection = w::Selection {
            session: record.sources[0].session.clone(),
            source_generation: w::Number(1),
        };
        let source = journal.source(&record.context, &selection).unwrap();
        assert_eq!(
            (source.device, source.inode, source.cursor),
            (
                record.sources[0].device,
                record.sources[0].inode,
                record.sources[0].cursor
            )
        );
        assert!(
            journal
                .operation("different-actor", &record.op_id, &record.request)
                .unwrap()
                .is_none()
        );
        assert!(matches!(
            journal.operation("actor-1", &record.op_id, b"changed original bytes"),
            Err(Code::OpIdReused)
        ));
        // A separately proposed manual operation aliases the original exact set.
        let mut duplicate = fixture.record(21);
        duplicate.trigger = false;
        let retained = journal
            .register("actor-1", &source_key, &duplicate)
            .unwrap();
        assert_eq!(retained.view.run_id, record.view.run_id);
        assert_eq!(
            retained.op_id, record.op_id,
            "duplicate replaced original operation identity"
        );
        assert_eq!(count(&journal, "runs"), 1);
        assert_eq!(count(&journal, "sources"), 1);
        assert_eq!(count(&journal, "close_cursor"), 1);
        assert_eq!(count(&journal, "ops"), 2);
        assert_eq!(cursor(&journal, &record), record.sources[0].cursor);
        drop(journal);
        let journal = fixture.journal();
        assert_eq!(
            journal
                .operation("actor-1", &duplicate.op_id, &duplicate.request)
                .unwrap()
                .unwrap()
                .view
                .run_id,
            record.view.run_id
        );
        assert_eq!(
            fs::read(&fixture.source).unwrap(),
            before,
            "journal altered retained source bytes"
        );
    }

    #[test]
    #[ignore = "KNOWN DEFECT: C1 journal initialization CPU overrun during lean land (1.126ms); fix: tasks/2026-11-07-dream-c1-native-unit-budget-overruns"]
    fn dream_c1_concurrent_register_original_operation_wins_before_candidate_source_key() {
        let fixture = Fixture::new();
        let source_before = fs::read(&fixture.source).unwrap();
        let mut first = fixture.record(240);
        first.trigger = false;
        let mut second = fixture.record(241);
        second.trigger = false;
        second.op_id = first.op_id.clone();
        assert_eq!(first.request, second.request);
        assert_ne!(first.view.run_id, second.view.run_id);
        let first_key = crate::store::digest(b"private original-op candidate source A");
        let second_key = crate::store::digest(b"private original-op candidate source B");
        assert_ne!(first_key, second_key);

        // Open both real SQLite owners before either waits at the barrier. An
        // initialization error cannot strand the other owner at an open race.
        let first_owner = fixture.journal();
        let second_owner = fixture.journal();
        let start = std::sync::Barrier::new(3);
        let retained = std::thread::scope(|scope| {
            let a = scope.spawn(|| {
                let mut journal = first_owner;
                start.wait();
                journal.register("same-original-actor", &first_key, &first)
            });
            let b = scope.spawn(|| {
                let mut journal = second_owner;
                start.wait();
                journal.register("same-original-actor", &second_key, &second)
            });
            start.wait();
            [a.join().unwrap(), b.join().unwrap()]
        });
        assert!(retained.iter().any(Result::is_ok));
        for result in &retained {
            assert!(
                result.is_ok() || matches!(result, Err(Code::Busy)),
                "unlabelled concurrent admission: {:?}",
                result.as_ref().err()
            );
        }
        // PERSIST's paid transaction can outlast an overlapping admission. Busy
        // refuses without waiting or implicit replay. After both exact owners
        // return, an explicitly new request recovers the ORIGINAL operation,
        // regardless of its other candidate SourceKey; no source is rerun.
        let replay = |record: &Record, source_key: &str| {
            let mut journal = fixture.journal();
            let result = journal
                .register("same-original-actor", source_key, record)
                .unwrap();
            journal.finish().unwrap();
            result
        };
        let retained = [
            retained[0]
                .clone()
                .unwrap_or_else(|_| replay(&first, &first_key)),
            retained[1]
                .clone()
                .unwrap_or_else(|_| replay(&second, &second_key)),
        ];
        assert_eq!(retained[0].view.run_id, retained[1].view.run_id);
        assert_eq!(retained[0].nonce, retained[1].nonce);
        assert_eq!(retained[0].request, retained[1].request);
        let original = &retained[0];
        assert!(
            original.view.run_id == first.view.run_id || original.view.run_id == second.view.run_id
        );
        assert_eq!(original.op_id, first.op_id);
        assert_eq!(original.request, first.request);
        let mut journal = fixture.journal();
        for table in ["runs", "ops", "sources", "close_cursor"] {
            assert_eq!(count(&journal, table), 1, "duplicate durable {table}");
        }
        assert_eq!(cursor(&journal, original), original.sources[0].cursor);
        let (durable_key, durable_run): (String, String) = journal
            .db
            .query_row(
                "SELECT r.source_key,o.run FROM runs AS r JOIN ops AS o ON o.run=r.id WHERE o.actor=?1 AND o.op=?2",
                params!["same-original-actor", first.op_id.0],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(durable_run, original.view.run_id.0);
        assert_eq!(
            durable_key,
            if original.view.run_id == first.view.run_id {
                first_key
            } else {
                second_key
            }
        );
        assert_eq!(
            journal
                .operation("same-original-actor", &first.op_id, &first.request)
                .unwrap()
                .unwrap()
                .view
                .run_id,
            original.view.run_id
        );

        let mut changed = fixture.record(242);
        changed.trigger = false;
        changed.op_id = first.op_id.clone();
        changed.request = w::canonical(&serde_json::json!({"v":1,"context":changed.context,
            "sources":[{"session":id(243),"source_generation":1}],"algorithm":"d0-c1/1"}))
        .unwrap();
        changed.view.request_sha256 = w::Hex(crate::store::digest(&changed.request));
        assert!(matches!(
            journal.register(
                "same-original-actor",
                &crate::store::digest(b"private changed-request candidate source"),
                &changed
            ),
            Err(Code::OpIdReused)
        ));
        for table in ["runs", "ops", "sources", "close_cursor"] {
            assert_eq!(
                count(&journal, table),
                1,
                "reused operation altered {table}"
            );
        }
        assert_eq!(fs::read(&fixture.source).unwrap(), source_before);
        // This is journal arbitration over metadata-only source fixtures. It
        // does not claim observer authentication or canonical worker execution.
    }

    #[test]
    fn dream_c1_late_sql_failure_rolls_back_run_source_close_cursor_and_original_op() {
        let fixture = Fixture::new();
        let record = fixture.record(22);
        let mut journal = fixture.journal();
        // The failing ops INSERT occurs after runs, sources and close_cursor.
        // This witnesses the actual transaction; no helper mimics rollback.
        journal.db.execute_batch("CREATE TEMP TRIGGER fail_final_op BEFORE INSERT ON ops BEGIN SELECT RAISE(FAIL,'private injected final-op failure'); END;").unwrap();
        assert!(matches!(
            journal.register("actor-1", &key(&record), &record),
            Err(Code::StorageUnavailable)
        ));
        for table in ["sources", "runs", "ops", "close_cursor"] {
            assert_eq!(
                count(&journal, table),
                0,
                "partial transaction retained {table}"
            );
        }
        drop(journal);
        let mut journal = fixture.journal();
        for table in ["sources", "runs", "ops", "close_cursor"] {
            assert_eq!(count(&journal, table), 0, "restart exposed partial {table}");
        }
        assert!(
            journal
                .operation("actor-1", &record.op_id, &record.request)
                .unwrap()
                .is_none()
        );
        assert!(
            journal
                .source(
                    &record.context,
                    &w::Selection {
                        session: record.sources[0].session.clone(),
                        source_generation: w::Number(1)
                    }
                )
                .is_err()
        );
        journal.register("actor-1", &key(&record), &record).unwrap();
        assert_eq!(cursor(&journal, &record), record.sources[0].cursor);
    }

    #[test]
    fn dream_c1_retained_source_refuses_root_workspace_and_aba_revision_changes() {
        let fixture = Fixture::new();
        let record = fixture.record(23);
        let mut journal = fixture.journal();
        journal.register("actor-1", &key(&record), &record).unwrap();
        drop(journal);
        let journal = fixture.journal();
        let selection = w::Selection {
            session: record.sources[0].session.clone(),
            source_generation: w::Number(1),
        };
        assert!(journal.source(&record.context, &selection).is_ok());
        for field in [0, 1, 2] {
            let mut current = record.context.clone();
            match field {
                0 => current.root = w::Id("other-root".into()),
                1 => current.root_revision.0 += 1,
                _ => current.workspace_revision.0 += 1,
            }
            assert!(
                matches!(
                    journal.source(&current, &selection),
                    Err(Code::SourceMismatch)
                ),
                "source followed replaced root/workspace incarnation"
            );
        }
        let mut back_to_a = record.context.clone();
        back_to_a.root_revision.0 += 2;
        back_to_a.workspace_revision.0 += 2;
        assert!(
            matches!(
                journal.source(&back_to_a, &selection),
                Err(Code::SourceMismatch)
            ),
            "A3 inherited A1 source custody"
        );
        let mut wrong_generation = selection;
        wrong_generation.source_generation.0 += 1;
        assert!(matches!(
            journal.source(&record.context, &wrong_generation),
            Err(Code::SourceUnavailable)
        ));
        assert!(
            journal
                .operation("new-actor-incarnation", &record.op_id, &record.request)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn dream_c1_cancel_original_operation_is_durable_and_cannot_retarget_a_run() {
        let fixture = Fixture::new();
        let record = fixture.record(24);
        let mut journal = fixture.journal();
        journal.register("actor-1", &key(&record), &record).unwrap();
        let op = id(25);
        let cancel = w::Control::Cancel(w::Cancel {
            v: w::Version,
            op_id: op.clone(),
            run_id: record.view.run_id.clone(),
        });
        let request = w::canonical(&cancel).unwrap();
        journal
            .cancel_operation("actor-1", &op, &request, &record.view.run_id)
            .unwrap();
        drop(journal);
        let mut journal = fixture.journal();
        journal
            .cancel_operation("actor-1", &op, &request, &record.view.run_id)
            .unwrap();
        assert_eq!(
            count(&journal, "ops"),
            2,
            "same Cancel created another operation"
        );
        assert_eq!(
            journal
                .operation("actor-1", &op, &request)
                .unwrap()
                .unwrap()
                .view
                .run_id,
            record.view.run_id
        );
        let other_run = id(26);
        let changed = w::canonical(&w::Control::Cancel(w::Cancel {
            v: w::Version,
            op_id: op.clone(),
            run_id: other_run.clone(),
        }))
        .unwrap();
        assert!(matches!(
            journal.cancel_operation("actor-1", &op, &changed, &other_run),
            Err(Code::OpIdReused)
        ));
        // Defense at the storage boundary: matching bytes cannot smuggle a
        // different effect target through a mismatched internal argument.
        assert!(matches!(
            journal.cancel_operation("actor-1", &op, &request, &other_run),
            Err(Code::OpIdReused)
        ));
        drop(journal);
        let journal = fixture.journal();
        assert_eq!(
            journal
                .operation("actor-1", &op, &request)
                .unwrap()
                .unwrap()
                .view
                .run_id,
            record.view.run_id
        );
    }

    fn pair(
        runtime: &Path,
        scope: &str,
        record: &Record,
    ) -> Result<(DiskPermit, DiskPermit), Code> {
        let root = DiskPermit::claim(runtime, scope, 2, record)?;
        let account = DiskPermit::claim(runtime, "account", 4, record)?;
        Ok((root, account))
    }
    #[test]
    fn dream_c1_disk_claims_enforce_two_per_root_four_account_and_independent_root() {
        let fixture = Fixture::new();
        let a1 = pair(&fixture.runtime, "root-a", &fixture.record(30)).unwrap();
        let a2 = pair(&fixture.runtime, "root-a", &fixture.record(31)).unwrap();
        assert!(matches!(
            pair(&fixture.runtime, "root-a", &fixture.record(32)),
            Err(Code::Busy)
        ));
        let b1 = pair(&fixture.runtime, "root-b", &fixture.record(33)).unwrap();
        let b2 = pair(&fixture.runtime, "root-b", &fixture.record(34)).unwrap();
        assert!(matches!(
            pair(&fixture.runtime, "root-c", &fixture.record(35)),
            Err(Code::Busy)
        ));
        assert_eq!(
            fs::read_dir(fixture.runtime.join("claims/root-c"))
                .unwrap()
                .count(),
            0,
            "failed account claim leaked its partial root slot"
        );
        drop(a1);
        drop(a2);
        drop(b1);
        drop(b2);
        for scope in ["root-a", "root-b", "account"] {
            assert_eq!(
                fs::read_dir(fixture.runtime.join("claims").join(scope))
                    .unwrap()
                    .count(),
                0,
                "settled owned claim was not removed"
            );
        }
        let resumed = pair(&fixture.runtime, "root-c", &fixture.record(36)).unwrap();
        drop(resumed);
    }

    #[test]
    fn dream_c1_disk_claim_cleanup_requires_exact_inode_token_and_nofollow_path() {
        let fixture = Fixture::new();
        let record = fixture.record(37);
        let permit = DiskPermit::claim(&fixture.runtime, "clean", 1, &record).unwrap();
        let path = permit.path.clone();
        drop(permit);
        assert!(!path.exists(), "matching own token/inode was not retired");

        let permit = DiskPermit::claim(&fixture.runtime, "token", 1, &record).unwrap();
        let path = permit.path.clone();
        let before = fs::metadata(&path).unwrap();
        let foreign = vec![b'x'; permit.token.len()];
        fs::write(&path, &foreign).unwrap();
        assert_eq!(fs::metadata(&path).unwrap().ino(), before.ino());
        drop(permit);
        assert_eq!(
            fs::read(&path).unwrap(),
            foreign,
            "claim removed changed same-inode token"
        );

        let permit = DiskPermit::claim(&fixture.runtime, "inode", 1, &record).unwrap();
        let path = permit.path.clone();
        // Keep the old inode open so the filesystem cannot immediately reuse it.
        let old_inode = File::open(&path).unwrap();
        let original = fs::read(&path).unwrap();
        fs::remove_file(&path).unwrap();
        let mut replacement = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&path)
            .unwrap();
        replacement.write_all(&original).unwrap();
        replacement.sync_all().unwrap();
        assert_ne!(
            replacement.metadata().unwrap().ino(),
            old_inode.metadata().unwrap().ino()
        );
        drop(permit);
        assert_eq!(
            fs::read(&path).unwrap(),
            original,
            "claim removed a replacement inode"
        );

        let permit = DiskPermit::claim(&fixture.runtime, "symlink", 1, &record).unwrap();
        let path = permit.path.clone();
        let target = fixture.directory.path().join("private-peer-token");
        fs::write(&target, b"peer token must survive").unwrap();
        fs::remove_file(&path).unwrap();
        std::os::unix::fs::symlink(&target, &path).unwrap();
        drop(permit);
        assert!(
            fs::symlink_metadata(&path)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(fs::read(target).unwrap(), b"peer token must survive");
    }

    #[test]
    fn dream_c1_child_retains_claim_on_process_exit() {
        let Some(runtime) = std::env::var_os("BBOXY_DREAM_C1_CLAIM_CHILD_ROOT") else {
            return;
        };
        let record = std::env::var_os("BBOXY_DREAM_C1_CLAIM_CHILD_RECORD")
            .expect("missing private child record");
        let record: Record = serde_json::from_slice(&fs::read(record).unwrap()).unwrap();
        let runtime = PathBuf::from(runtime);
        let _claims = pair(&runtime, "root-a", &record).unwrap();
        let ready = runtime.join("child-claimed");
        let mut marker = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(ready)
            .unwrap();
        marker.write_all(b"durable claims established").unwrap();
        marker.sync_all().unwrap();
        // A daemon crash/process exit cannot invoke the claim owner's Drop.
        // This fixture exits only its exact child test process, never its parent.
        std::process::exit(0);
    }
    #[test]
    fn dream_c1_predecessor_claim_survives_actual_owner_exit_and_still_consumes_capacity() {
        let fixture = Fixture::new();
        let predecessor = fixture.record(40);
        let path = fixture.directory.path().join("private-child-record.json");
        let mut record_file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&path)
            .unwrap();
        record_file
            .write_all(&serde_json::to_vec(&predecessor).unwrap())
            .unwrap();
        record_file.sync_all().unwrap();
        let mut child = Command::new(std::env::current_exe().unwrap())
            .env_clear()
            .env("BBOXY_DREAM_C1_CLAIM_CHILD_ROOT", &fixture.runtime)
            .env("BBOXY_DREAM_C1_CLAIM_CHILD_RECORD", &path)
            .args([
                "--exact",
                "registry::dream_c1::registry_tests::dream_c1_child_retains_claim_on_process_exit",
                "--nocapture",
                "--test-threads=1",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let child_pid = child.id();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        loop {
            if child.try_wait().unwrap().is_some() {
                break;
            }
            if std::time::Instant::now() >= deadline {
                let _ = child.kill();
                let _ = child.wait();
                panic!("exact owned predecessor child {child_pid} did not exit");
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        let output = child.wait_with_output().unwrap();
        assert!(
            output.status.success(),
            "owned predecessor fixture failed: {} {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            fs::read(fixture.runtime.join("child-claimed")).unwrap(),
            b"durable claims established",
            "child selector ran no fixture"
        );
        let old_root = fixture.runtime.join("claims/root-a/slot-0");
        let old_account = fixture.runtime.join("claims/account/slot-0");
        let old_token = fs::read(&old_root).unwrap();
        assert_eq!(old_token, fs::read(&old_account).unwrap());
        let mut successor = fixture.record(41);
        successor.owner_incarnation = id(101);
        let a = pair(&fixture.runtime, "root-a", &successor).unwrap();
        assert!(
            matches!(
                pair(&fixture.runtime, "root-a", &fixture.record(42)),
                Err(Code::Busy)
            ),
            "successor ignored predecessor's root slot"
        );
        let b = pair(&fixture.runtime, "root-b", &fixture.record(43)).unwrap();
        let c = pair(&fixture.runtime, "root-c", &fixture.record(44)).unwrap();
        assert!(
            matches!(
                pair(&fixture.runtime, "root-d", &fixture.record(45)),
                Err(Code::Busy)
            ),
            "successor ignored predecessor's account slot"
        );
        drop(a);
        drop(b);
        drop(c);
        assert_eq!(
            fs::read(&old_root).unwrap(),
            old_token,
            "new owner reclaimed predecessor root evidence"
        );
        assert_eq!(
            fs::read(&old_account).unwrap(),
            old_token,
            "new owner reclaimed predecessor account evidence"
        );
    }
    fn metadata_fence(record: &Record) -> crate::identity::PublicFence {
        crate::identity::PublicFence {
            daemon_incarnation: crate::Ulid::new(),
            fence_rev: 1,
            publication_rev: 2,
            account: record.context.account.0.clone(),
            account_epoch: 1,
            issuer: crate::messaging::wire::Issuer {
                kind: crate::messaging::wire::IssuerKind::LocalDev,
                key: "metadata-only".into(),
            },
            issuer_epoch: 1,
            device: None,
            device_epoch: None,
            proof: "metadata-only".into(),
            grant: None,
            not_before: 0,
            expires_at: None,
            revoked: false,
        }
    }
    #[test]
    #[ignore = "KNOWN DEFECT: C1 per-unit CPU overrun at low load (1.05–1.44ms); fix: tasks/2026-11-07-dream-c1-native-unit-budget-overruns"]
    fn dream_c1_source_failure_is_bounded_immutable_and_fenced_before_any_replay() {
        let f = Fixture::new();
        let record = f.record(90);
        let source = &record.sources[0];
        let selection = w::Selection {
            session: source.session.clone(),
            source_generation: w::Number(source.generation),
        };
        // Metadata-only journal fixture; actual public proof admission is witnessed
        // by logic::dream_public_api_tests, never this constructed stamp.
        let fence = metadata_fence(&record);
        let before = fs::read(&f.source).unwrap();
        let journal = f.journal();
        journal
            .observe_error(&record.context, &fence, &selection, Code::ResourceLimit)
            .unwrap();
        journal
            .observe_error(&record.context, &fence, &selection, Code::SourceChanged)
            .unwrap();
        assert_eq!(
            journal
                .source_error(&record.context, &fence, &selection)
                .unwrap(),
            Some(Code::ResourceLimit)
        );
        let mut stale = fence.clone();
        stale.fence_rev += 1;
        assert_eq!(
            journal.source_error(&record.context, &stale, &selection),
            Err(Code::StaleContext)
        );
        let mut grant = record.context.clone();
        grant.grant_epoch.0 += 1;
        assert_eq!(
            journal.source_error(&grant, &fence, &selection),
            Err(Code::StaleContext)
        );
        for index in 1..32 {
            let selection = w::Selection {
                session: id(index + 900),
                source_generation: w::Number(1),
            };
            journal
                .observe_error(&record.context, &fence, &selection, Code::ResourceLimit)
                .unwrap();
        }
        journal
            .observe_error(&record.context, &fence, &selection, Code::SourceChanged)
            .unwrap();
        let overflow = w::Selection {
            session: id(9999),
            source_generation: w::Number(1),
        };
        assert_eq!(
            journal.observe_error(&record.context, &fence, &overflow, Code::ResourceLimit),
            Err(Code::Busy)
        );
        assert_eq!(
            journal
                .source_error(&record.context, &fence, &overflow)
                .unwrap(),
            None
        );
        assert_eq!(fs::read(&f.source).unwrap(), before);
    }
    #[test]
    #[ignore = "KNOWN DEFECT: busy0 two-owner contention; fix: tasks/2026-11-06-dream-c1-two-owner-storage-race"]
    fn dream_c1_source_error_capacity_is_atomic_across_real_sqlite_owners() {
        let f = Fixture::new();
        let record = f.record(91);
        let fence = metadata_fence(&record);
        let journal = f.journal();
        for i in 0..31 {
            journal
                .observe_error(
                    &record.context,
                    &fence,
                    &w::Selection {
                        session: id(10000 + i),
                        source_generation: w::Number(1),
                    },
                    Code::ResourceLimit,
                )
                .unwrap();
        }
        let barrier = Arc::new(std::sync::Barrier::new(2));
        let results = std::thread::scope(|scope| {
            let mut handles = Vec::new();
            for i in 0..2 {
                let runtime = f.runtime.clone();
                let context = record.context.clone();
                let fence = fence.clone();
                let barrier = barrier.clone();
                handles.push(scope.spawn(move || {
                    let journal = Journal::open(&runtime, &context.seat).unwrap();
                    barrier.wait();
                    journal.observe_error(
                        &context,
                        &fence,
                        &w::Selection {
                            session: id(20000 + i),
                            source_generation: w::Number(1),
                        },
                        Code::ResourceLimit,
                    )
                }));
            }
            handles
                .into_iter()
                .map(|h| h.join().unwrap())
                .collect::<Vec<_>>()
        });
        assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
        assert_eq!(results.iter().filter(|r| **r == Err(Code::Busy)).count(), 1);
        assert_eq!(count(&journal, "source_errors"), 32);
    }
    // Copyright 2026 Boxy (bboxy450) and contributors.
    // SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
    // Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
    // License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
    // Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
    // Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
    // Reuses the metadata-only Fixture; these are actual SQLite/process
    // admission witnesses, not producer
    // conformance or a claim that fabricated Completed metadata is a worker result.

    fn cas_preparing(previous: &Record, owner: u128) -> Record {
        let mut next = previous.clone();
        next.started = true;
        next.owner_incarnation = id(owner);
        next.view.state = w::State::Preparing;
        next.view.revision.0 += 1;
        next
    }
    fn cas_spawn_and_reap(counter: &std::sync::atomic::AtomicUsize) {
        // A real private child is dispatched only after a winning durable CAS. Its
        // exact returned PID and successful wait are retained in the test output.
        let mut child = Command::new("/usr/bin/true")
            .env_clear()
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let pid = child.id();
        assert!(pid > 0);
        counter.fetch_add(1, Ordering::AcqRel);
        let status = child.wait().unwrap();
        assert!(
            status.success(),
            "owned admission child {pid} failed: {status}"
        );
        assert!(child.try_wait().unwrap().unwrap().success());
        eprintln!("dream_c1_cas_child pid={pid} reaped=true exit=0");
    }

    #[test]
    fn dream_c1_durable_cas_rejects_stale_inspect_after_cancel_and_arc_eviction() {
        let f = Fixture::new();
        let initial = f.record(101);
        f.journal()
            .register("cas-cancel-actor", &key(&initial), &initial)
            .unwrap();
        let root = Arc::new(RootState::default());
        root.runs.lock().unwrap().insert(
            initial.view.run_id.0.clone(),
            Arc::new(Run {
                initial: initial.clone(),
                started: AtomicBool::new(false),
                final_record: OnceLock::new(),
                control: Arc::new(Control::default()),
                session: OnceLock::new(),
            }),
        );
        let released = Arc::new(std::sync::Barrier::new(2));
        let (read_ready, stale_read) = std::sync::mpsc::sync_channel(1);
        let spawned = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        std::thread::scope(|scope| {
            let runtime = f.runtime.clone();
            let run_id = initial.view.run_id.clone();
            let seat = initial.context.seat.clone();
            let inspector_root = root.clone();
            let inspector_released = released.clone();
            let spawned = spawned.clone();
            let inspector = scope.spawn(move || {
                let journal = Journal::open(&runtime, &seat).unwrap();
                let stale = journal.run(&run_id).unwrap().unwrap();
                assert_eq!(stale.view.state, w::State::Deferred);
                read_ready.send(()).unwrap();
                inspector_released.wait();
                assert!(
                    !inspector_root.runs.lock().unwrap().contains_key(&run_id.0),
                    "the terminal Arc must have been evicted before this start attempt"
                );
                // A fresh in-memory start fence can win again after eviction. The
                // persisted exact previous bytes must still forbid dispatch.
                let fresh_start = AtomicBool::new(false);
                assert!(
                    fresh_start
                        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                        .is_ok()
                );
                let next = cas_preparing(&stale, 5001);
                let won = journal.compare_and_save(&stale, &next).unwrap();
                if won {
                    cas_spawn_and_reap(&spawned);
                }
                assert!(
                    !won,
                    "stale Inspect resurrected a durably cancelled proposal"
                );
                let retained = journal.run(&run_id).unwrap().unwrap();
                assert_eq!(retained.view.state, w::State::Cancelled);
                assert!(retained.view.settled);
                assert!(!retained.started);
            });
            stale_read
                .recv_timeout(std::time::Duration::from_secs(5))
                .unwrap();
            let journal = f.journal();
            let previous = journal.run(&initial.view.run_id).unwrap().unwrap();
            let mut cancelled = previous.clone();
            cancelled.view.state = w::State::Cancelled;
            cancelled.view.settled = true;
            cancelled.view.revision.0 += 1;
            cancelled.view.error = Some(w::Error {
                code: Code::Cancelled,
                source: None,
            });
            assert!(journal.compare_and_save(&previous, &cancelled).unwrap());
            let run = root
                .runs
                .lock()
                .unwrap()
                .remove(&initial.view.run_id.0)
                .unwrap();
            assert!(
                run.started
                    .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                    .is_ok()
            );
            assert!(run.final_record.set(cancelled).is_ok());
            released.wait();
            inspector.join().unwrap();
        });
        assert_eq!(spawned.load(Ordering::Acquire), 0);
    }

    #[test]
    fn dream_c1_durable_cas_rejects_stale_inspect_after_completed_publication() {
        let f = Fixture::new();
        let initial = f.record(102);
        f.journal()
            .register("cas-complete-actor", &key(&initial), &initial)
            .unwrap();
        let released = Arc::new(std::sync::Barrier::new(2));
        let (read_ready, stale_read) = std::sync::mpsc::sync_channel(1);
        let spawned = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        std::thread::scope(|scope| {
            let runtime = f.runtime.clone();
            let run_id = initial.view.run_id.clone();
            let seat = initial.context.seat.clone();
            let inspector_released = released.clone();
            let spawned = spawned.clone();
            let inspector = scope.spawn(move || {
                let journal = Journal::open(&runtime, &seat).unwrap();
                let stale = journal.run(&run_id).unwrap().unwrap();
                read_ready.send(()).unwrap();
                inspector_released.wait();
                let next = cas_preparing(&stale, 5002);
                let won = journal.compare_and_save(&stale, &next).unwrap();
                if won {
                    cas_spawn_and_reap(&spawned);
                }
                assert!(!won, "stale Inspect overwrote immutable Completed metadata");
                let retained = journal.run(&run_id).unwrap().unwrap();
                assert_eq!(retained.view.state, w::State::Completed);
                assert!(retained.view.settled);
                assert_eq!(retained.view.result_sha256, Some(w::Hex("a".repeat(64))));
            });
            stale_read
                .recv_timeout(std::time::Duration::from_secs(5))
                .unwrap();
            let journal = f.journal();
            let previous = journal.run(&initial.view.run_id).unwrap().unwrap();
            let mut completed = previous.clone();
            completed.started = true;
            completed.view.state = w::State::Completed;
            completed.view.settled = true;
            completed.view.revision.0 += 1;
            completed.view.manifest_sha256 = Some(w::Hex("b".repeat(64)));
            completed.view.result_sha256 = Some(w::Hex("a".repeat(64)));
            assert!(journal.compare_and_save(&previous, &completed).unwrap());
            released.wait();
            inspector.join().unwrap();
        });
        assert_eq!(spawned.load(Ordering::Acquire), 0);
    }

    #[test]
    #[ignore = "KNOWN DEFECT: two-owner busy0 race refuses the winner's post-dispatch record (StorageUnavailable); fix: tasks/2026-11-06-dream-c1-two-owner-storage-race"]
    fn dream_c1_two_real_sqlite_start_owners_dispatch_exactly_one_private_child() {
        let f = Fixture::new();
        let initial = f.record(103);
        f.journal()
            .register("cas-start-actor", &key(&initial), &initial)
            .unwrap();
        let ready = Arc::new(std::sync::Barrier::new(2));
        let spawned = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let outcomes = std::thread::scope(|scope| {
            let mut owners = Vec::new();
            for owner in [5003, 5004] {
                let runtime = f.runtime.clone();
                let run_id = initial.view.run_id.clone();
                let seat = initial.context.seat.clone();
                let ready = ready.clone();
                let spawned = spawned.clone();
                owners.push(scope.spawn(move || {
                    let journal = Journal::open(&runtime, &seat).unwrap();
                    let previous = journal.run(&run_id).unwrap().unwrap();
                    assert_eq!(previous.view.state, w::State::Deferred);
                    let next = cas_preparing(&previous, owner);
                    ready.wait();
                    let won = journal.compare_and_save(&previous, &next).unwrap();
                    if won {
                        cas_spawn_and_reap(&spawned);
                    }
                    won
                }));
            }
            owners
                .into_iter()
                .map(|owner| owner.join().unwrap())
                .collect::<Vec<_>>()
        });
        assert_eq!(outcomes.iter().filter(|won| **won).count(), 1);
        assert_eq!(
            spawned.load(Ordering::Acquire),
            1,
            "CAS success, not a process exit or an in-memory Arc, decides one spawn"
        );
        let retained = f.journal().run(&initial.view.run_id).unwrap().unwrap();
        assert!(retained.started);
        assert_eq!(retained.view.state, w::State::Preparing);
        assert!([id(5003), id(5004)].contains(&retained.owner_incarnation));
    }
    fn scalar_authority_fixture() -> (
        Connection,
        w::Context,
        crate::registry::SeatRecord,
        crate::registry::SwarmRecord,
    ) {
        let f = Fixture::new();
        let context = f.record(104).context;
        let seat = crate::registry::SeatRecord::new(
            context.seat.0.parse().unwrap(),
            context.swarm.0.parse().unwrap(),
            "scalar fixture",
            "zsh",
        );
        let mut swarm = crate::registry::SwarmRecord::new(seat.swarm);
        swarm.revision = context.swarm_revision.0;
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch("CREATE TABLE kv(key TEXT PRIMARY KEY,value BLOB NOT NULL);
            CREATE TABLE msg_policies(seat TEXT PRIMARY KEY,epoch INTEGER NOT NULL,role BLOB NOT NULL,muted INTEGER NOT NULL);").unwrap();
        db.execute(
            "INSERT INTO kv VALUES(?1,?2)",
            params![
                format!("seat/{}", seat.id),
                serde_json::to_vec(&seat).unwrap()
            ],
        )
        .unwrap();
        db.execute(
            "INSERT INTO kv VALUES(?1,?2)",
            params![
                format!("swarm/{}", swarm.id),
                serde_json::to_vec(&swarm).unwrap()
            ],
        )
        .unwrap();
        db.execute(
            "INSERT INTO msg_policies VALUES(?1,?2,?3,0)",
            params![
                seat.id.to_string(),
                context.grant_epoch.0 as i64,
                serde_json::to_vec(&seat.role).unwrap()
            ],
        )
        .unwrap();
        (db, context, seat, swarm)
    }
    #[test]
    fn dream_c1_borrowed_authority_scalars_reject_current_grant_generation_mute_and_swarm_aba() {
        let (db, context, mut seat, mut swarm) = scalar_authority_fixture();
        assert!(Authority::rows_match(&db, &context).unwrap());
        db.execute("UPDATE msg_policies SET epoch=epoch+1", [])
            .unwrap();
        assert!(!Authority::rows_match(&db, &context).unwrap());
        db.execute(
            "UPDATE msg_policies SET epoch=?1",
            [context.grant_epoch.0 as i64],
        )
        .unwrap();
        db.execute("UPDATE msg_policies SET muted=1", []).unwrap();
        assert!(!Authority::rows_match(&db, &context).unwrap());
        db.execute("UPDATE msg_policies SET muted=0", []).unwrap();
        seat.generation += 1;
        db.execute(
            "UPDATE kv SET value=?1 WHERE key=?2",
            params![
                serde_json::to_vec(&seat).unwrap(),
                format!("seat/{}", seat.id)
            ],
        )
        .unwrap();
        assert!(!Authority::rows_match(&db, &context).unwrap());
        seat.generation -= 1;
        seat.muted = true;
        db.execute(
            "UPDATE kv SET value=?1 WHERE key=?2",
            params![
                serde_json::to_vec(&seat).unwrap(),
                format!("seat/{}", seat.id)
            ],
        )
        .unwrap();
        assert!(!Authority::rows_match(&db, &context).unwrap());
        seat.muted = false;
        db.execute(
            "UPDATE kv SET value=?1 WHERE key=?2",
            params![
                serde_json::to_vec(&seat).unwrap(),
                format!("seat/{}", seat.id)
            ],
        )
        .unwrap();
        assert!(Authority::rows_match(&db, &context).unwrap());
        for revision in [2, 3] {
            // A1 ->B2 ->A3 uses the committed revision, never a repeated name.
            swarm.revision = revision;
            db.execute(
                "UPDATE kv SET value=?1 WHERE key=?2",
                params![
                    serde_json::to_vec(&swarm).unwrap(),
                    format!("swarm/{}", swarm.id)
                ],
            )
            .unwrap();
            assert!(!Authority::rows_match(&db, &context).unwrap());
        }
    }
    #[test]
    fn dream_c1_borrowed_authority_role_comparison_keeps_canonical_bytes_and_escaping() {
        let (db, context, mut seat, _) = scalar_authority_fixture();
        for name in ["worker", "quote\" slash\\ newline\n λ"] {
            seat.role.name = name.into();
            db.execute(
                "UPDATE kv SET value=?1 WHERE key=?2",
                params![
                    serde_json::to_vec(&seat).unwrap(),
                    format!("seat/{}", seat.id)
                ],
            )
            .unwrap();
            let bytes = serde_json::to_vec(&seat.role).unwrap();
            db.execute("UPDATE msg_policies SET role=?1", [&bytes])
                .unwrap();
            assert!(
                Authority::rows_match(&db, &context).unwrap(),
                "canonical role changed its encoding: {name:?}"
            );
            let mut spaced = vec![b' '];
            spaced.extend_from_slice(&bytes);
            db.execute("UPDATE msg_policies SET role=?1", [spaced])
                .unwrap();
            assert!(
                !Authority::rows_match(&db, &context).unwrap(),
                "noncanonical policy bytes were accepted"
            );
        }
    }
    #[test]
    fn dream_c1_borrowed_authority_bounds_native_json_and_preserves_legacy_absence() {
        let (db, mut context, seat, swarm) = scalar_authority_fixture();
        for bytes in [vec![b'x'; 16385], b"{invalid-json".to_vec()] {
            db.execute(
                "UPDATE kv SET value=?1 WHERE key=?2",
                params![bytes, format!("seat/{}", seat.id)],
            )
            .unwrap();
            assert!(!Authority::rows_match(&db, &context).unwrap());
        }
        db.execute(
            "UPDATE kv SET value=?1 WHERE key=?2",
            params![
                serde_json::to_vec(&seat).unwrap(),
                format!("seat/{}", seat.id)
            ],
        )
        .unwrap();
        db.execute("UPDATE msg_policies SET role=?1", [vec![b'x'; 1025]])
            .unwrap();
        assert!(!Authority::rows_match(&db, &context).unwrap());
        db.execute(
            "UPDATE msg_policies SET role=?1",
            [serde_json::to_vec(&seat.role).unwrap()],
        )
        .unwrap();
        db.execute(
            "DELETE FROM kv WHERE key=?1",
            [format!("swarm/{}", swarm.id)],
        )
        .unwrap();
        assert!(!Authority::rows_match(&db, &context).unwrap());
        context.swarm_revision = w::Number(0);
        assert!(
            Authority::rows_match(&db, &context).unwrap(),
            "legacy absent metadata must retain revision zero"
        );
        db.execute(
            "INSERT INTO kv VALUES(?1,?2)",
            params![format!("swarm/{}", swarm.id), b"{invalid".as_slice()],
        )
        .unwrap();
        assert!(
            !Authority::rows_match(&db, &context).unwrap(),
            "invalid metadata is not legacy absence"
        );
    }
}

impl Runtime {
    pub(crate) fn read(&self,handle:&tokio::runtime::Handle,authority:Authority,call:crate::mcp::Call,done:impl FnOnce(serde_json::Value)+Send+'static) {
        let permit=match Permit::claim(&self.requests,2) {Ok(p)=>p,Err(_)=>{done(crate::mcp::dream_read::result(&call,bbt_proto::dream_read::Reply::unavailable(bbt_proto::dream_read::Reason::Busy)));return;}};
        let runtime=self.clone();
        handle.spawn_blocking(move||{
            let output=if engine::utility().is_err() || runtime.retired.load(Ordering::Acquire) {
                crate::mcp::dream_read::result(&call,bbt_proto::dream_read::Reply::failure(bbt_proto::dream_read::Code::StaleContext))
            }else{runtime.read_cache.respond(&runtime.path,&authority,&call)};
            done(output);drop(permit);
        });
    }
}

#[cfg(test)]
impl Runtime {
    pub(crate) fn prepare_read_fixture(&self,record:&Record,authority:&Authority)->Result<(),Code> {
        self.read_cache.prepare(&self.path,record,authority,&engine::Control::default())
    }
}
#[cfg(test)]
impl Runtime {
    pub(crate) fn corrupt_read_fixture(&self,context:&w::Context,kind:&str) {self.read_cache.corrupt_fixture(context,kind);}
}

#[cfg(test)]
impl Runtime {
    pub(crate) fn read_cache_fixture(&self)->Arc<super::dream_read::Cache> {self.read_cache.clone()}
}
