// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Pure filesystem custody. File::try_lock is the existing cross-process lock:
//! one lock per canvas log, no global writer or queue. Busy callers retry.
#![allow(
    clippy::disallowed_methods,
    reason = "canvas store IO runs off the UI/logic turn"
)]
use crate::{
    Action, Canvas, CanvasRecord, Op, ReplayError,
    log::{self, Batch},
    record::{self, Record},
};
use std::{
    collections::BTreeSet,
    fs::{self, File, OpenOptions},
    io::{self, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

pub const MAX_BATCH_OPS: usize = 256;
#[derive(Debug, Clone)]
pub struct Store {
    directory: PathBuf,
}
#[derive(Debug)]
pub struct Snapshot {
    pub record: Record,
    pub canvas: Canvas,
    pub batches: Vec<Batch>,
    pub seq: u64,
    pub dropped_tail_bytes: usize,
}
impl Snapshot {
    pub fn last_author(&self) -> &str {
        self.batches
            .last()
            .map_or(&self.record.creator, |b| &b.author)
    }
    pub fn last_change(&self) -> u64 {
        self.batches.last().map_or(0, |b| b.at)
    }
}
#[derive(Debug)]
pub struct Append {
    pub snapshot: Snapshot,
    pub duplicate: bool,
    pub stale: bool,
}

pub(crate) fn refused(code: &str, reason: impl std::fmt::Display) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, format!("{code}: {reason}"))
}
fn replay_error(error: ReplayError) -> io::Error {
    if let ReplayError::ElementCap { cap } = error {
        refused("canvas_element_cap", format!("MAX_ELEMENTS={cap}"))
    } else {
        refused("canvas_invalid_op", error)
    }
}
pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
impl Store {
    pub fn new(directory: impl Into<PathBuf>) -> Self {
        Self {
            directory: directory.into(),
        }
    }
    pub fn directory(&self) -> &Path {
        &self.directory
    }
    pub fn records(&self) -> io::Result<Vec<Record>> {
        let entries = match fs::read_dir(&self.directory) {
            Ok(entries) => entries,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(vec![]),
            Err(e) => return Err(e),
        };
        let mut records = vec![];
        for entry in entries {
            let entry = entry?;
            if entry.file_type()?.is_file()
                && entry.path().extension().is_some_and(|ext| ext == "md")
            {
                let record = record::decode(&fs::read(entry.path())?)?;
                if entry.file_name() != format!("{}.md", record.slug).as_str() {
                    return Err(refused(
                        "canvas_record_invalid",
                        "record slug differs from its file name",
                    ));
                }
                records.push(record);
            }
        }
        Ok(records)
    }
    pub fn create(&self, id: &str, metadata: CanvasRecord) -> io::Result<Snapshot> {
        let canvas_id = ulid::Ulid::from_string(id)
            .map_err(|_| refused("canvas_invalid_id", "id must be a ULID"))?;
        if metadata.title.trim().is_empty()
            || metadata.title.len() > 256
            || metadata.creator.is_empty()
            || metadata.swarm.is_empty()
            || metadata.tags.iter().any(|t| t.is_empty() || t.len() > 64)
            || metadata.tags.len() > 32
        {
            return Err(refused(
                "canvas_record_invalid",
                "title, swarm, creator and bounded nonempty tags required",
            ));
        }
        // The caller's create operation is its id, so a lost create reply is safe.
        for old in self.records()? {
            if old.id == id {
                let expected = Record::new(id.into(), old.number, metadata);
                if old != expected {
                    return Err(refused("op_id_reused", "canvas create arguments changed"));
                }
                File::open(&self.directory)?.sync_all()?;
                return self.load(&old);
            }
        }
        fs::create_dir_all(&self.directory)?;
        // A deterministic 53-bit number and the per-canvas OS lock also dedupe
        // simultaneous create retries without a global allocator lock.
        let first = u128::from(canvas_id) as u64 & ((1 << 53) - 1);
        for offset in 0..10_000 {
            let number = (first + offset) % ((1 << 53) - 1);
            let record = Record::new(id.into(), number, metadata.clone());
            let mut options = OpenOptions::new();
            options.read(true).write(true).create(true).truncate(false);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.custom_flags(libc::O_NOFOLLOW);
            }
            let log = options.open(self.log_path(&record))?;
            log.try_lock().map_err(|_| {
                refused(
                    "canvas_busy",
                    "canvas create in progress; retry same operation",
                )
            })?;
            let path = self.directory.join(format!("{}.md", record.slug));
            match fs::read(&path) {
                Ok(bytes) => {
                    let old = record::decode(&bytes)?;
                    if old.id != id {
                        continue;
                    }
                    if old != record {
                        return Err(refused("op_id_reused", "canvas create arguments changed"));
                    }
                    File::open(&self.directory)?.sync_all()?;
                    drop(log);
                    return self.load(&old);
                }
                Err(e) if e.kind() == io::ErrorKind::NotFound => {}
                Err(e) => return Err(e),
            }
            if log.metadata()?.len() != 0 {
                return Err(refused(
                    "canvas_record_missing",
                    "nonempty log has no record",
                ));
            }
            log.sync_all()?;
            // Publish the record atomically too. A crash before rename leaves
            // an empty reserved log that the same operation can finish.
            let temporary = self.directory.join(format!(".{}-{id}.tmp", record.slug));
            let mut file = options.truncate(true).open(&temporary)?;
            file.write_all(&record::encode(&record)?)?;
            file.sync_all()?;
            fs::rename(&temporary, &path)?;
            File::open(&self.directory)?.sync_all()?;
            drop(log);
            return self.load(&record);
        }
        Err(refused("canvas_busy", "canvas number allocation occupied"))
    }
    fn log_path(&self, record: &Record) -> PathBuf {
        self.directory
            .join(format!("canvas-{}.log.jsonl", record.number))
    }
    fn open_log(&self, record: &Record, write: bool) -> io::Result<File> {
        let mut options = OpenOptions::new();
        options.read(true).write(write);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.custom_flags(libc::O_NOFOLLOW);
        }
        options.open(self.log_path(record))
    }
    pub fn load(&self, record: &Record) -> io::Result<Snapshot> {
        let mut file = self.open_log(record, false)?;
        file.try_lock_shared()
            .map_err(|_| refused("canvas_busy", "canvas append in progress; retry"))?;
        self.read_locked(record, &mut file)
    }
    fn read_locked(&self, record: &Record, file: &mut File) -> io::Result<Snapshot> {
        let mut bytes = vec![];
        file.read_to_end(&mut bytes)?;
        let loaded = log::decode_batches(&bytes)?;
        let mut seen = BTreeSet::new();
        let mut ops = vec![];
        for batch in &loaded.batches {
            if batch.batch_id.is_empty()
                || batch.author.is_empty()
                || batch.ops.is_empty()
                || batch.ops.len() > MAX_BATCH_OPS
                || !seen.insert(&batch.batch_id)
                || batch.stale != batch.expect_seq.is_some_and(|seq| seq != ops.len() as u64)
                || batch
                    .ops
                    .iter()
                    .any(|op| op.author != batch.author || op.at != batch.at)
            {
                return Err(refused(
                    "canvas_log_invalid",
                    "invalid or duplicate batch metadata",
                ));
            }
            ops.extend(batch.ops.iter().cloned());
        }
        let canvas = Canvas::replay(&ops).map_err(replay_error)?;
        Ok(Snapshot {
            record: record.clone(),
            canvas,
            seq: ops.len() as u64,
            batches: loaded.batches,
            dropped_tail_bytes: loaded.dropped_tail_bytes,
        })
    }
    /// Sequence is assigned here, never accepted from a caller. A complete
    /// candidate is validated before write, then fsynced before acknowledgement.
    pub fn append(
        &self,
        record: &Record,
        batch_id: &str,
        author: &str,
        expect_seq: Option<u64>,
        actions: &[Action],
    ) -> io::Result<Append> {
        if batch_id.is_empty() || author.is_empty() {
            return Err(refused(
                "canvas_invalid_batch",
                "batch_id and author required",
            ));
        }
        if actions.is_empty() || actions.len() > MAX_BATCH_OPS {
            return Err(refused(
                "canvas_batch_cap",
                format!("MAX_BATCH_OPS={MAX_BATCH_OPS}; batch must be nonempty"),
            ));
        }
        let mut file = self.open_log(record, true)?;
        file.try_lock().map_err(|_| {
            refused(
                "canvas_busy",
                "canvas append in progress; retry same batch_id",
            )
        })?;
        let mut snapshot = self.read_locked(record, &mut file)?;
        if let Some(old) = snapshot.batches.iter().find(|b| b.batch_id == batch_id) {
            if !old.same_request(author, expect_seq, actions) {
                return Err(refused(
                    "op_id_reused",
                    "batch_id reused with different arguments",
                ));
            }
            // A prior writer may have died after its complete write but
            // before fsync. Retried acknowledgements take durability custody.
            file.sync_all()?;
            return Ok(Append {
                stale: old.stale,
                snapshot,
                duplicate: true,
            });
        }
        let mut candidate = snapshot.canvas.clone();
        for action in actions {
            candidate.apply(action).map_err(replay_error)?;
        }
        let at = now_ms();
        let stale = expect_seq.is_some_and(|seq| seq != snapshot.seq);
        let batch = Batch {
            batch_id: batch_id.into(),
            author: author.into(),
            at,
            expect_seq,
            stale,
            ops: actions
                .iter()
                .enumerate()
                .map(|(i, action)| Op {
                    author: author.into(),
                    seq: snapshot.seq + i as u64 + 1,
                    at,
                    action: action.clone(),
                })
                .collect(),
        };
        let bytes = log::encode_batch(&batch)?;
        let end = file.seek(SeekFrom::End(0))?;
        // A partial earlier attempt is not an operation. Drop it before the
        // next complete line, and carry its byte count in this response.
        let complete = end - snapshot.dropped_tail_bytes as u64;
        file.set_len(complete)?;
        file.seek(SeekFrom::Start(complete))?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        snapshot.seq += actions.len() as u64;
        snapshot.batches.push(batch);
        snapshot.canvas = candidate;
        Ok(Append {
            snapshot,
            stale,
            duplicate: false,
        })
    }
}
