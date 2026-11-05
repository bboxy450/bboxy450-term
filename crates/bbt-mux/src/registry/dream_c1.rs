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
    os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt},
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

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Record {
    pub trigger: bool,
    pub started: bool,
    pub owner_incarnation: w::Ulid,
    pub worker_pid: Option<u32>,
    pub worker_start: Option<String>,
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

pub(crate) struct Journal {
    db: Connection,
}
impl Journal {
    pub fn open(runtime: &Path, seat: &w::Ulid) -> Result<Self, Code> {
        bbt_rt::assert_not_ui_thread();
        private_dir(runtime)?;
        let journals = runtime.join("journals");
        private_dir(&journals)?;
        let path = journals.join(format!("{}.sqlite", seat.0));
        use std::os::unix::fs::OpenOptionsExt;
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(&path)
            .map_err(|_| Code::StorageUnavailable)?;
        let m = file.metadata().map_err(|_| Code::StorageUnavailable)?;
        if !m.is_file()
            || m.nlink() != 1
            || m.uid() != unsafe { libc::geteuid() }
            || m.mode() & 0o077 != 0
        {
            return Err(Code::StorageUnavailable);
        }
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))
            .map_err(|_| Code::StorageUnavailable)?;
        let db = Connection::open(&path).map_err(|_| Code::StorageUnavailable)?;
        db.busy_timeout(std::time::Duration::from_millis(100))
            .map_err(|_| Code::StorageUnavailable)?;
        db.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL; PRAGMA cache_size=-1024; PRAGMA mmap_size=0;
            CREATE TABLE IF NOT EXISTS source_errors(session TEXT NOT NULL,generation INTEGER NOT NULL,error BLOB NOT NULL,PRIMARY KEY(session,generation));
            CREATE TABLE IF NOT EXISTS sources(session TEXT NOT NULL,generation INTEGER NOT NULL,bookmark BLOB NOT NULL,PRIMARY KEY(session,generation));
            CREATE TABLE IF NOT EXISTS runs(id TEXT PRIMARY KEY,source_key TEXT UNIQUE NOT NULL,record BLOB NOT NULL);
            CREATE TABLE IF NOT EXISTS ops(actor TEXT NOT NULL,op TEXT NOT NULL,request BLOB NOT NULL,run TEXT NOT NULL,PRIMARY KEY(actor,op));
            CREATE TABLE IF NOT EXISTS close_cursor(session TEXT NOT NULL,generation INTEGER NOT NULL,cursor INTEGER NOT NULL,PRIMARY KEY(session,generation));")
            .map_err(|_| Code::StorageUnavailable)?;
        Ok(Self { db })
    }
    pub fn observe_error(
        &self,
        context: &w::Context,
        fence: &crate::identity::PublicFence,
        selection: &w::Selection,
        code: Code,
    ) -> Result<(), Code> {
        let mut context = context.clone();
        context.generation = selection.source_generation;
        let row = w::canonical(&serde_json::json!({"context":context,"fence":fence,"code":code}))
            .map_err(|_| Code::StorageUnavailable)?;
        if row.len() > 16 * 1024 {
            return Err(Code::ResourceLimit);
        }
        // One SQLite write statement arbitrates capacity and dedup atomically;
        // overlapping predecessor/current owners cannot overfill the retained index.
        let changed=self.db.execute(
            "INSERT OR IGNORE INTO source_errors(session,generation,error) SELECT ?1,?2,?3 WHERE (SELECT count(*) FROM source_errors)<32",
            params![selection.session.0,selection.source_generation.0 as i64,row],
        ).map_err(|_|Code::StorageUnavailable)?;
        if changed == 0 {
            let existing: bool = self
                .db
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM source_errors WHERE session=?1 AND generation=?2)",
                    params![selection.session.0, selection.source_generation.0 as i64],
                    |r| r.get(0),
                )
                .map_err(|_| Code::StorageUnavailable)?;
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
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct ErrorRecord {
            context: w::Context,
            fence: crate::identity::PublicFence,
            code: Code,
        }
        let bytes=self.db.query_row("SELECT CASE WHEN length(error)<=16384 AND typeof(error)='blob' THEN error ELSE NULL END FROM source_errors WHERE session=?1 AND generation=?2",params![selection.session.0,selection.source_generation.0 as i64],|r|bounded_blob(r,0)).optional().map_err(|_|Code::StorageUnavailable)?;
        let Some(bytes) = bytes else {
            return Ok(None);
        };
        let record: ErrorRecord =
            serde_json::from_slice(&bytes).map_err(|_| Code::StorageUnavailable)?;
        let mut expected = context.clone();
        expected.generation = selection.source_generation;
        if record.context != expected || &record.fence != fence {
            return Err(Code::StaleContext);
        }
        Ok(Some(record.code))
    }
    fn source_set(&self, key: &str) -> Result<Option<Record>, Code> {
        let bytes=self.db.query_row("SELECT CASE WHEN typeof(record)='blob' AND length(record)<=16384 THEN record ELSE NULL END FROM runs WHERE source_key=?1",[key],|r|bounded_blob(r,0)).optional().map_err(|_|Code::StorageUnavailable)?;
        bytes
            .map(|bytes| Self::decode_record(&self.db, &bytes))
            .transpose()
    }
    pub fn source(&self, context: &w::Context, source: &w::Selection) -> Result<Bookmark, Code> {
        let bytes: Option<Vec<u8>> = self
            .db
            .query_row(
                "SELECT CASE WHEN typeof(bookmark)='blob' AND length(bookmark)<=16384 THEN bookmark ELSE NULL END FROM sources WHERE session=?1 AND generation=?2",
                params![source.session.0, source.source_generation.0 as i64],
                |r| bounded_blob(r, 0),
            )
            .optional()
            .map_err(|_| Code::StorageUnavailable)?;
        let source: Bookmark = serde_json::from_slice(&bytes.ok_or(Code::SourceUnavailable)?)
            .map_err(|_| Code::StorageUnavailable)?;
        if !source_current(&source, context) {
            return Err(Code::SourceMismatch);
        }
        if source.closed_seq == 0 {
            return Err(Code::SourceOpen);
        }
        Ok(source)
    }
    pub fn observe_source(&mut self, context: &w::Context, source: &Bookmark) -> Result<(), Code> {
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
        let bytes = serde_json::to_vec(source).map_err(|_| Code::StorageUnavailable)?;
        if bytes.len() > 16 * 1024 {
            return Err(Code::ResourceLimit);
        }
        let tx = self
            .db
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|_| Code::StorageUnavailable)?;
        let old: Option<Vec<u8>> = tx
            .query_row(
                "SELECT CASE WHEN typeof(bookmark)='blob' AND length(bookmark)<=16384 THEN bookmark ELSE NULL END FROM sources WHERE session=?1 AND generation=?2",
                params![source.session.0, source.generation as i64],
                |r| bounded_blob(r, 0),
            )
            .optional()
            .map_err(|_| Code::StorageUnavailable)?;
        if let Some(old) = old {
            let prior: Bookmark =
                serde_json::from_slice(&old).map_err(|_| Code::StorageUnavailable)?;
            if prior.root != source.root
                || prior.workspace_revision != source.workspace_revision
                || prior.root_revision != source.root_revision
            {
                return Err(Code::StaleContext);
            }
            if prior.observed_seq >= source.observed_seq || prior.closed_seq > 0 {
                return Ok(());
            }
            tx.execute(
                "UPDATE sources SET bookmark=?3 WHERE session=?1 AND generation=?2",
                params![source.session.0, source.generation as i64, bytes],
            )
            .map_err(|_| Code::StorageUnavailable)?;
        } else {
            let count: i64 = tx
                .query_row("SELECT count(*) FROM sources", [], |r| r.get(0))
                .map_err(|_| Code::StorageUnavailable)?;
            if count >= 32 {
                return Err(Code::Busy);
            }
            tx.execute(
                "INSERT INTO sources(session,generation,bookmark) VALUES(?1,?2,?3)",
                params![source.session.0, source.generation as i64, bytes],
            )
            .map_err(|_| Code::StorageUnavailable)?;
        }
        tx.commit().map_err(|_| Code::StorageUnavailable)
    }
    pub fn retained_sources(&self, context: &w::Context) -> Result<Vec<Bookmark>, Code> {
        let mut query = self
            .db
            .prepare("SELECT CASE WHEN typeof(bookmark)='blob' AND length(bookmark)<=16384 THEN bookmark ELSE NULL END FROM sources ORDER BY generation,session LIMIT 33")
            .map_err(|_| Code::StorageUnavailable)?;
        let bytes = query
            .query_map([], |r| bounded_blob(r, 0))
            .map_err(|_| Code::StorageUnavailable)?;
        let mut sources = Vec::new();
        for row in bytes {
            let source: Bookmark =
                serde_json::from_slice(&row.map_err(|_| Code::StorageUnavailable)?)
                    .map_err(|_| Code::StorageUnavailable)?;
            if source_current(&source, context) {
                sources.push(source);
            }
        }
        if sources.len() > 32 {
            return Err(Code::StorageUnavailable);
        }
        Ok(sources)
    }
    fn decode_record(_db: &Connection, bytes: &[u8]) -> Result<Record, Code> {
        let record: Record = serde_json::from_slice(bytes).map_err(|_| Code::StorageUnavailable)?;
        if record.sources.is_empty() || record.sources.len() > 32 {
            return Err(Code::StorageUnavailable);
        }
        Ok(record)
    }
    pub fn run(&self, id: &w::Ulid) -> Result<Option<Record>, Code> {
        let bytes = self
            .db
            .query_row("SELECT CASE WHEN typeof(record)='blob' AND length(record)<=16384 THEN record ELSE NULL END FROM runs WHERE id=?1", [&id.0], |r| {
                bounded_blob(r, 0)
            })
            .optional()
            .map_err(|_| Code::StorageUnavailable)?;
        bytes.map(|b| Self::decode_record(&self.db, &b)).transpose()
    }
    /// Lookup original operation before any source lookup/hash/copy/process work.
    pub fn operation(
        &self,
        actor: &str,
        op: &w::Ulid,
        request: &[u8],
    ) -> Result<Option<Record>, Code> {
        let old: Option<(Vec<u8>, String)> = self
            .db
            .query_row(
                "SELECT CASE WHEN typeof(request)='blob' AND length(request)<=16384 THEN request ELSE NULL END,CASE WHEN typeof(run)='text' AND length(run)=26 THEN run ELSE NULL END FROM ops WHERE actor=?1 AND op=?2",
                params![actor, op.0],
                |r| Ok((bounded_blob(r, 0)?, r.get(1)?)),
            )
            .optional()
            .map_err(|_| Code::StorageUnavailable)?;
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
        let tx = self
            .db
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|_| Code::StorageUnavailable)?;
        let prior: Option<(Vec<u8>, String)> = tx
            .query_row(
                "SELECT CASE WHEN typeof(request)='blob' AND length(request)<=16384 THEN request ELSE NULL END,CASE WHEN typeof(run)='text' AND length(run)=26 THEN run ELSE NULL END FROM ops WHERE actor=?1 AND op=?2",
                params![actor, op.0],
                |r| Ok((bounded_blob(r, 0)?, r.get(1)?)),
            )
            .optional()
            .map_err(|_| Code::StorageUnavailable)?;
        if let Some((prior, prior_run)) = prior {
            if prior != request || prior_run != run.0 {
                return Err(Code::OpIdReused);
            }
        } else {
            let count: i64 = tx
                .query_row("SELECT count(*) FROM ops", [], |r| r.get(0))
                .map_err(|_| Code::StorageUnavailable)?;
            if count >= 4096 {
                return Err(Code::StorageUnavailable);
            }
            tx.execute(
                "INSERT INTO ops(actor,op,request,run) VALUES(?1,?2,?3,?4)",
                params![actor, op.0, request, run.0],
            )
            .map_err(|_| Code::StorageUnavailable)?;
        }
        tx.commit().map_err(|_| Code::StorageUnavailable)
    }
    /// Cursor, close custody, unique source set and original op share one durable transaction.
    pub fn register(&mut self, actor: &str, key: &str, record: &Record) -> Result<Record, Code> {
        let tx = self
            .db
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|_| Code::StorageUnavailable)?;
        let prior_op:Option<(Vec<u8>,String)>=tx.query_row(
            "SELECT CASE WHEN length(request)<=16384 AND typeof(request)='blob' THEN request ELSE NULL END,CASE WHEN length(run)=26 THEN run ELSE NULL END FROM ops WHERE actor=?1 AND op=?2",
            params![actor,record.op_id.0],|r|Ok((bounded_blob(r,0)?,r.get(1)?))).optional().map_err(|_|Code::StorageUnavailable)?;
        if let Some((request, id)) = prior_op {
            if request != record.request {
                return Err(Code::OpIdReused);
            }
            let bytes=tx.query_row("SELECT CASE WHEN length(record)<=16384 AND typeof(record)='blob' THEN record ELSE NULL END FROM runs WHERE id=?1",[id],|r|bounded_blob(r,0)).map_err(|_|Code::StorageUnavailable)?;
            let retained = Self::decode_record(&tx, &bytes)?;
            tx.commit().map_err(|_| Code::StorageUnavailable)?;
            return Ok(retained);
        }
        let prior: Option<Vec<u8>> = tx
            .query_row("SELECT CASE WHEN typeof(record)='blob' AND length(record)<=16384 THEN record ELSE NULL END FROM runs WHERE source_key=?1", [key], |r| {
                bounded_blob(r, 0)
            })
            .optional()
            .map_err(|_| Code::StorageUnavailable)?;
        let aliases: i64 = tx
            .query_row("SELECT count(*) FROM ops", [], |r| r.get(0))
            .map_err(|_| Code::StorageUnavailable)?;
        if aliases >= 4096 && !(record.trigger && prior.is_some()) {
            return Err(Code::StorageUnavailable);
        }
        let actual: Record = if let Some(bytes) = prior {
            Self::decode_record(&tx, &bytes)?
        } else {
            let count: i64 = tx
                .query_row("SELECT count(*) FROM runs", [], |r| r.get(0))
                .map_err(|_| Code::StorageUnavailable)?;
            // Explicit retention admission; never delete old results or evidence implicitly.
            if count >= 1024 {
                return Err(Code::StorageUnavailable);
            }
            let bytes = record_bytes(record)?;
            if bytes.len() > 16 * 1024 {
                return Err(Code::ResourceLimit);
            }
            tx.execute(
                "INSERT INTO runs(id,source_key,record) VALUES(?1,?2,?3)",
                params![record.view.run_id.0, key, bytes],
            )
            .map_err(|_| Code::StorageUnavailable)?;
            record.clone()
        };
        for source in &record.sources {
            let bytes = serde_json::to_vec(source).map_err(|_| Code::StorageUnavailable)?;
            if bytes.len() > 16 * 1024 {
                return Err(Code::ResourceLimit);
            }
            tx.execute("INSERT INTO sources(session,generation,bookmark) VALUES(?1,?2,?3) ON CONFLICT(session,generation) DO UPDATE SET bookmark=excluded.bookmark WHERE json_extract(CAST(sources.bookmark AS TEXT),'$.observed_seq')<=json_extract(CAST(excluded.bookmark AS TEXT),'$.observed_seq')",
                params![source.session.0,source.generation as i64,bytes]).map_err(|_| Code::StorageUnavailable)?;
            tx.execute("INSERT INTO close_cursor(session,generation,cursor) VALUES(?1,?2,?3) ON CONFLICT(session,generation) DO UPDATE SET cursor=max(cursor,excluded.cursor)",
                params![source.session.0,source.generation as i64,source.cursor as i64]).map_err(|_| Code::StorageUnavailable)?;
        }
        // Duplicate close notifications retain the unique row's first operation;
        // explicit manual aliases alone consume another operation entry.
        if !record.trigger || actual.view.run_id == record.view.run_id {
            tx.execute(
                "INSERT INTO ops(actor,op,request,run) VALUES(?1,?2,?3,?4)",
                params![actor, record.op_id.0, record.request, actual.view.run_id.0],
            )
            .map_err(|_| Code::StorageUnavailable)?;
        }
        tx.commit().map_err(|_| Code::StorageUnavailable)?;
        Ok(actual)
    }
    /// Durable lifecycle arbitration survives eviction of an in-memory Run.
    pub fn compare_and_save(&self, expected: &Record, next: &Record) -> Result<bool, Code> {
        if expected.view.run_id != next.view.run_id
            || expected.op_id != next.op_id
            || expected.nonce != next.nonce
        {
            return Err(Code::ProtocolError);
        }
        let before = record_bytes(expected)?;
        let after = record_bytes(next)?;
        self.db
            .execute(
                "UPDATE runs SET record=?2 WHERE id=?1 AND record=?3",
                params![expected.view.run_id.0, after, before],
            )
            .map(|n| n == 1)
            .map_err(|_| Code::StorageUnavailable)
    }
    fn cancel_unstarted(&self, previous: &Record) -> Result<Record, Code> {
        if previous.started || previous.view.settled {
            return Ok(previous.clone());
        }
        let mut next = previous.clone();
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
    }
    pub fn save(&self, record: &Record) -> Result<(), Code> {
        let bytes = record_bytes(record)?;
        if bytes.len() > 16 * 1024 {
            return Err(Code::ResourceLimit);
        }
        self.db
            .execute(
                "UPDATE runs SET record=?2 WHERE id=?1",
                params![record.view.run_id.0, bytes],
            )
            .map_err(|_| Code::StorageUnavailable)?;
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
impl Authority {
    pub fn capture(&self) -> Result<w::Context, Code> {
        self.capture_all().map(|(context, _)| context)
    }
    pub fn capture_all(&self) -> Result<(w::Context, crate::identity::PublicFence), Code> {
        // No SeatKeys or identity status/admission call is reachable here. The
        // host publication carries public facts only; each effect checks it anew.
        if self.fence.cached_revision() & 1 != 0 {
            return Err(Code::StaleContext);
        }
        let fence = self.fence.current().map_err(|_| Code::StaleContext)?;
        let context = self.capture_context(&fence)?;
        Ok((context, fence))
    }
    fn capture_context(&self, fence: &crate::identity::PublicFence) -> Result<w::Context, Code> {
        // Identity and caller metadata share ONE consistent read-only snapshot;
        // no repeated key parsing or separate connection sits inside a work unit.
        let (record,grant,revision)=self.fence.with_current(fence,|db| {
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
        self.matches_with_session(context, fence, None)
    }
    fn matches_with_session(
        &self,
        context: &w::Context,
        fence: &Option<crate::identity::PublicFence>,
        session: Option<&crate::identity::PublicFenceSession>,
    ) -> bool {
        let Some(fence) = fence.as_ref() else {
            return false;
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
            return false;
        }
        let Ok(before) = self.port.host.dream_metadata() else {
            return false;
        };
        if before.0 != context.provider_epoch.0 || before.1 != context.provider.0 {
            return false;
        }
        let read = |db: &Connection| Self::rows_match(db, context);
        let matches = match session {
            Some(session) => self.fence.with_current_session(fence, session, read),
            None => self.fence.with_current(fence, read),
        };
        if !matches.is_ok_and(|current| current) {
            return false;
        }
        self.port
            .host
            .dream_metadata()
            .is_ok_and(|after| after == before)
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
        let cpu = bbt_rt::fault::thread_cpu_time().map_err(|_| Code::ResourceLimit)?;
        let prepared = (|| -> Result<Arc<engine::AuthorityGuard>, Code> {
            let session = self
                .fence
                .prepare_session(|db| {
                    drop(db.prepare_cached(AUTHORITY_MATCH_SQL)?);
                    Ok(())
                })
                .map_err(|_| Code::StaleContext)?;
            let authority = self.clone();
            let context = context.clone();
            let fence = fence.clone();
            Ok(Arc::new(move || {
                if authority.matches_with_session(&context, &fence, Some(&session)) {
                    Ok(())
                } else {
                    Err(Code::StaleContext)
                }
            }))
        })();
        let used = bbt_rt::fault::thread_cpu_time()
            .map_err(|_| Code::ResourceLimit)?
            .saturating_sub(cpu);
        let setup_cpu_ns = used.as_nanos().min(u64::MAX as u128) as u64;
        if let Some(observed) = setup_observed {
            observed.fetch_max(setup_cpu_ns, Ordering::Release);
        }
        if used > std::time::Duration::from_millis(1) {
            eprintln!("dream_c1_authority_setup_refused cpu_ns={setup_cpu_ns} limit_ns=1000000");
            return Err(Code::ResourceLimit);
        }
        Ok(engine::PreparedGuard {
            callback: prepared?,
            setup_cpu_ns,
        })
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
/// No queue or shared compute pool: each admitted run owns its own process/IO lanes.
#[derive(Clone)]
pub(crate) struct Runtime {
    incarnation: w::Ulid,
    path: PathBuf,
    binary: PathBuf,
    supported: Arc<AtomicBool>,
    retired: Arc<AtomicBool>,
    requests: Arc<AtomicUsize>,
    controls: Arc<AtomicUsize>,
    roots: Arc<Mutex<BTreeMap<String, Arc<RootState>>>>,
}
impl Runtime {
    pub fn new(path: PathBuf, binary: PathBuf, handle: &tokio::runtime::Handle) -> Self {
        let runtime = Self {
            incarnation: crate::Ulid::new().into(),
            path,
            binary,
            supported: Arc::new(AtomicBool::new(false)),
            retired: Arc::new(AtomicBool::new(false)),
            requests: Default::default(),
            controls: Default::default(),
            roots: Default::default(),
        };
        let probe = runtime.clone();
        // Only the short owner creation uses the blocking executor; no C1 IO runs there.
        handle.spawn_blocking(move || {
            let _ = std::thread::Builder::new()
                .name("dream-c1-capability".into())
                .spawn(move || {
                    if engine::utility().is_err() {
                        return;
                    }
                    if private_dir(&probe.path).is_err() {
                        return;
                    }
                    let job = probe.path.join(format!("probe-{}", crate::Ulid::new()));
                    if private_dir(&job).is_err() {
                        return;
                    }
                    if private_dir(&job.join("tmp")).is_err() {
                        return;
                    }
                    let supported =
                        bboxy450_harness_wake_up::dream_c1::probe(&probe.binary, &job).is_ok();
                    probe.supported.store(supported, Ordering::Release);
                    // Only this newly created empty probe tree, after exact child reap.
                    let _ = std::fs::remove_dir(job.join("tmp"));
                    let _ = std::fs::remove_dir(job);
                });
        });
        runtime
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
                let mut done=Some(done);
                let mut presentation=None;
                let result=(|| {
                    engine::utility()?;
                    if runtime.retired.load(Ordering::Acquire) {return Err(Code::WorkerLost);}
                    let request=parse()?;
                    request.validate()?;
                    let (context,identity_fence)=authority.capture_all()?;
                    presentation=Some((context.clone(),Some(identity_fence.clone())));
                    let root_key=crate::store::digest(&w::canonical(&serde_json::json!([context.account,context.root])).map_err(|_|Code::BadRequest)?);
                    let root={
                        let mut roots=runtime.roots.try_lock().map_err(|_|Code::Busy)?;
                        if roots.len()>=1024 && !roots.contains_key(&root_key) {return Err(Code::StorageUnavailable);}
                        roots.entry(root_key.clone()).or_default().clone()
                    };
                    let mut journal=Journal::open(&runtime.path,&context.seat)?;
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
                                if let Some(mut predecessor)=journal.source_set(&key)? {
                                    if predecessor.started && !predecessor.view.settled && predecessor.owner_incarnation!=runtime.incarnation {
                                        predecessor.view.state=w::State::Quarantined;
                                        predecessor.view.error=Some(w::Error {code:Code::WorkerLost,source:None});
                                        predecessor.view.revision.0+=1;
                                        journal.save(&predecessor)?;
                                    }
                                }
                                return Err(Code::StaleContext);
                            }
                            let record=Record {trigger:automatic,started:false,owner_incarnation:runtime.incarnation.clone(),worker_pid:None,worker_start:None,
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
                                let run=Arc::new(Run {initial:record.clone(),started:AtomicBool::new(false),final_record:OnceLock::new(),control});
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
                            // Deferred controls retain no SQLite connection. Prepare the
                            // one compute session only after this owner holds both caps;
                            // its measured setup is independent of per-unit guard CPU.
                            let preparation=authority.guard(&context,&record.identity_fence,Some(&run.control.authority_setup_cpu_ns))
                                .and_then(|guard|run.control.install_authority_guard(guard));
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
                                        if let Ok(mut runs)=root.runs.try_lock() {
                                            if runs.get(&record.view.run_id.0).is_some_and(|cached|Arc::ptr_eq(cached,&run)) {runs.remove(&record.view.run_id.0);}
                                        }
                                        return Err(code);
                                    },
                                    Ok(false)=>{
                                        if let Ok(mut runs)=root.runs.try_lock() {
                                            if runs.get(&record.view.run_id.0).is_some_and(|cached|Arc::ptr_eq(cached,&run)) {runs.remove(&record.view.run_id.0);}
                                        }
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
                                    if let Ok(mut runs)=root.runs.try_lock() {
                                        if runs.get(&record.view.run_id.0).is_some_and(|cached|Arc::ptr_eq(cached,&run)) {runs.remove(&record.view.run_id.0);}
                                    }
                                    if let Err(code)=outcome {return Err(code);}
                                    let current=journal.run(&record.view.run_id)?.ok_or(Code::NotFound)?;
                                    return runtime.status(&root,&authority,&context,current,None);
                                }
                            }
                            run.control.state.store(engine::PREPARING,Ordering::Release);
                            run.control.revision.fetch_add(1,Ordering::AcqRel);
                            let admission=if authority.matches(&context,&record.identity_fence) {
                                reply(&record)
                            } else {
                                // A claimed durable owner still settles even if its
                                // presentation authority changes before the reply.
                                run.control.cancelled.store(true,Ordering::Release);
                                failure(Code::StaleContext)
                            };
                            (done.take().unwrap())(admission);
                            // Admission reply is independent of snapshot/process ownership.
                            drop(permit);
                            drop(journal);
                            let job=runtime.path.join(&record.view.run_id.0);
                            let monitor_done=Arc::new(AtomicBool::new(false));
                            let monitor_stop=monitor_done.clone();
                            let monitor_authority=authority.clone();
                            let monitor_context=context.clone();
                            let monitor_fence=record.identity_fence.clone();
                            let monitor_control=run.control.clone();
                            let monitor_failure=Arc::new(OnceLock::new());
                            let monitor_error=monitor_failure.clone();
                            let retired=runtime.retired.clone();
                            let monitor=std::thread::Builder::new().name("dream-c1-authority".into()).spawn(move || {
                                if engine::utility().is_err() {let _=monitor_error.set(Code::ResourceLimit);monitor_control.cancelled.store(true,Ordering::Release);return;}
                                // One independent monitor session plus the sequential
                                // compute session is the job's entire two-connection
                                // authority allocation. Each check still reads a fresh
                                // transaction; prepared SQL never caches permission.
                                let monitor_guard=match monitor_authority.guard(&monitor_context,&monitor_fence,Some(&monitor_control.authority_monitor_setup_cpu_ns)) {
                                    Ok(guard)=>guard,
                                    Err(code)=>{
                                        let _=monitor_error.set(code);
                                        if code==Code::ResourceLimit {monitor_control.authority_monitor_overruns.fetch_add(1,Ordering::Relaxed);}
                                        monitor_control.cancelled.store(true,Ordering::Release);
                                        return;
                                    }
                                };
                                std::thread::park_timeout(std::time::Duration::from_millis(9));
                                while !monitor_stop.load(Ordering::Acquire) {
                                    let before=match bbt_rt::fault::thread_cpu_time() {
                                        Ok(cpu)=>cpu,
                                        Err(_)=>{let _=monitor_error.set(Code::ResourceLimit);monitor_control.authority_monitor_overruns.fetch_add(1,Ordering::Relaxed);monitor_control.cancelled.store(true,Ordering::Release);return;}
                                    };
                                    let current=(monitor_guard.callback)();
                                    let used=bbt_rt::fault::thread_cpu_time().map(|cpu|cpu.saturating_sub(before));
                                    let overrun=match used {
                                        Ok(cpu)=>{
                                            monitor_control.authority_monitor_max_cpu_ns.fetch_max(cpu.as_nanos().min(u64::MAX as u128) as u64,Ordering::Relaxed);
                                            cpu>std::time::Duration::from_millis(1)
                                        },
                                        Err(_)=>true,
                                    };
                                    if overrun {monitor_control.authority_monitor_overruns.fetch_add(1,Ordering::Relaxed);}
                                    if overrun||retired.load(Ordering::Acquire)||current.is_err() {
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
                                match monitor {
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
                                        monitor.join().map_err(|_|Code::WorkerLost)?;
                                    },
                                    Err(_)=>return Err(Code::WorkerLost),
                                }
                                monitor_failure.get().copied().map_or(Ok(()),Err)
                            });
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
                            let persisted=Journal::open(&runtime.path,&context.seat).and_then(|journal|journal.save(&record));
                            if persisted.is_err() {record.view.state=w::State::Quarantined;record.view.settled=false;record.view.error=Some(w::Error {code:Code::StorageUnavailable,source:None});}
                            let settled=record.view.settled;
                            if !settled {std::mem::forget(account);std::mem::forget(root_permit);}
                            let _=run.final_record.set(record);
                            if settled {if let Ok(mut runs)=root.runs.try_lock() {runs.remove(&run.initial.view.run_id.0);}}
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
                            let previous=journal.operation(&actor,&cancel.op_id,&request)?;
                            let record=match previous {Some(r)=>r,None=>journal.run(&cancel.run_id)?.ok_or(Code::NotFound)?};
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
                                    if let Ok(mut runs)=root.runs.try_lock() {
                                        if runs.get(&cancel.run_id.0).is_some_and(|cached|Arc::ptr_eq(cached,&run)) {runs.remove(&cancel.run_id.0);}
                                    }
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
                if let Some(done)=done {
                    let response=match result {
                        Ok(response @ w::ControlReply::Success(_))=> {
                            if presentation.as_ref().is_some_and(|(context,fence)|authority.matches(context,fence)) {response}
                            else {failure(Code::StaleContext)}
                        }
                        Ok(response)=>response,
                        Err(code)=>failure(code),
                    };
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
        mut record: Record,
        page: Option<u32>,
    ) -> Result<w::ControlReply, Code> {
        if &record.context != context || !authority.matches(context, &record.identity_fence) {
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
        if !authority.matches(context, &record.identity_fence) {
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
            [a.join().unwrap().unwrap(), b.join().unwrap().unwrap()]
        });
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
