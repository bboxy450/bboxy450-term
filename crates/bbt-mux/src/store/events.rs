// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Immutable, signed, hash-chained events. Root/grant trust belongs to the caller;
//! this layer verifies bytes and chain continuity, not account authorization.

use std::path::{Path, PathBuf};

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use ulid::Ulid;

use super::{Events, Result, digest, invalid, now_ms, sqlite};

pub const MAX_EVENT_BYTES: usize = 1024 * 1024;
/// First-upgrade work is bounded independently of retained history. The prefix
/// is audited after logic publishes readiness; this anchor is not a full audit.
const STARTUP_TAIL_ROWS: u64 = 128;

#[derive(Debug, PartialEq)]
struct Checkpoint {
    seq: u64,
    hash: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Draft {
    pub id: Ulid,
    pub ts: u64,
    pub subject: String,
    pub source: String,
    pub seat: Option<Ulid>,
    pub generation: Option<u64>,
    pub reply_to: Option<String>,
    /// Derived from the verified account root; absent on ordinary account rows.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub issued_by: Option<String>,
    pub payload: Value,
}

impl Draft {
    pub fn new(subject: impl Into<String>, source: impl Into<String>, payload: Value) -> Self {
        Self {
            id: Ulid::new(),
            ts: now_ms(),
            subject: subject.into(),
            source: source.into(),
            seat: None,
            generation: None,
            reply_to: None,
            issued_by: None,
            payload,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Provenance {
    pub signer: [u8; 32],
    pub sig: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Envelope {
    pub seq: u64,
    #[serde(flatten)]
    pub event: Draft,
    pub prev: String,
    pub provenance: Provenance,
}

impl Envelope {
    pub fn signed(event: Draft, seq: u64, prev: String, key: &SigningKey) -> Result<Self> {
        let mut envelope = Self {
            seq,
            event,
            prev,
            provenance: Provenance {
                signer: key.verifying_key().to_bytes(),
                sig: Vec::new(),
            },
        };
        envelope.provenance.sig = key.sign(&envelope.signing_bytes()?).to_bytes().to_vec();
        if serde_json::to_vec(&envelope)?.len() > MAX_EVENT_BYTES {
            return Err(invalid("event exceeds 1 MiB"));
        }
        Ok(envelope)
    }
    fn signing_bytes(&self) -> Result<Vec<u8>> {
        // Length-prefixed domain and canonical struct fields; the key and all
        // routing/generation fields are covered, not an unsigned JSON adjunct.
        let payload =
            serde_json::to_vec(&(self.seq, &self.event, &self.prev, self.provenance.signer))?;
        let mut bytes = b"bboxy/event/v1\0".to_vec();
        bytes.extend_from_slice(&(payload.len() as u64).to_le_bytes());
        bytes.extend(payload);
        Ok(bytes)
    }
    pub fn verify(&self) -> Result<()> {
        let key = VerifyingKey::from_bytes(&self.provenance.signer)?;
        let signature = Signature::try_from(self.provenance.sig.as_slice())?;
        key.verify(&self.signing_bytes()?, &signature)?;
        Ok(())
    }
    pub fn hash(&self) -> Result<String> {
        Ok(digest(&serde_json::to_vec(self)?))
    }
}

#[derive(Debug, Clone)]
pub struct EventLog {
    pub(crate) path: PathBuf,
}

impl EventLog {
    pub fn open(directory: &Path) -> Result<Self> {
        Ok(Self::open_counted(directory)?.0)
    }

    /// Startup verification is bounded by the checkpoint, so progress reports
    /// the verified suffix once rather than streaming retained history.
    pub(crate) fn open_with_progress(
        directory: &Path,
        progress: &mut dyn FnMut(u64, u64),
    ) -> Result<Self> {
        let (log, verified) = Self::open_counted(directory)?;
        progress(verified, verified);
        Ok(log)
    }

    fn open_counted(directory: &Path) -> Result<(Self, u64)> {
        bbt_rt::assert_not_ui_thread();
        std::fs::create_dir_all(directory)?;
        let log = Self {
            path: directory.join("events.sqlite"),
        };
        let mut db = sqlite::open(&log.path)?;
        sqlite::ensure_schema(&mut db, "CREATE TABLE IF NOT EXISTS version(version INTEGER NOT NULL);
          INSERT INTO version SELECT 1 WHERE NOT EXISTS(SELECT 1 FROM version);
          CREATE TABLE IF NOT EXISTS events(seq INTEGER PRIMARY KEY, id TEXT NOT NULL UNIQUE, ts INTEGER NOT NULL,
            subject TEXT NOT NULL, source TEXT NOT NULL, payload BLOB NOT NULL, signer BLOB NOT NULL, sig BLOB NOT NULL, prev TEXT NOT NULL, hash TEXT NOT NULL);
          CREATE INDEX IF NOT EXISTS events_subject ON events(subject,seq);
          CREATE TRIGGER IF NOT EXISTS immutable_update BEFORE UPDATE ON events BEGIN SELECT RAISE(ABORT,'append only'); END;
          CREATE TRIGGER IF NOT EXISTS immutable_delete BEFORE DELETE ON events BEGIN SELECT RAISE(ABORT,'append only'); END;
          CREATE TABLE IF NOT EXISTS consumers(name TEXT PRIMARY KEY, filter TEXT NOT NULL, cursor INTEGER NOT NULL, acked INTEGER NOT NULL, deliver TEXT NOT NULL, policy TEXT NOT NULL);
          CREATE TABLE IF NOT EXISTS pending(consumer TEXT NOT NULL REFERENCES consumers(name), seq INTEGER NOT NULL REFERENCES events(seq), delivered_at INTEGER NOT NULL, deliveries INTEGER NOT NULL, PRIMARY KEY(consumer,seq));
          CREATE TABLE IF NOT EXISTS stream(first_seq INTEGER NOT NULL,cold TEXT);
          INSERT INTO stream SELECT 1,NULL WHERE NOT EXISTS(SELECT 1 FROM stream);", "unsupported event schema")?;
        // ensure_schema deliberately skips DDL for user_version=1. Migrate
        // separately, additively, without taking a writer on subsequent opens.
        let migrated: bool = db.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='verification_checkpoint')",
            [], |row| row.get(0),
        )?;
        if !migrated {
            db.execute_batch("CREATE TABLE IF NOT EXISTS verification_checkpoint(
                id INTEGER PRIMARY KEY CHECK(id=1), seq INTEGER NOT NULL CHECK(seq>=0), hash TEXT NOT NULL)")?;
        }
        let tx = db.transaction()?;
        let checkpoint = tx
            .query_row(
                "SELECT seq,hash FROM verification_checkpoint WHERE id=1",
                [],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()?
            .map(|(seq, hash)| -> Result<_> {
                Ok(Checkpoint {
                    seq: seq.try_into()?,
                    hash,
                })
            })
            .transpose()?;
        // A range starting at zero must not silently exclude nonpositive rows.
        let first: Option<i64> = tx
            .query_row("SELECT seq FROM events ORDER BY seq LIMIT 1", [], |row| {
                row.get(0)
            })
            .optional()?;
        if first.is_some_and(|seq| seq != 1) {
            return Err(invalid("broken event chain: missing genesis"));
        }
        let anchor = if let Some(checkpoint) = &checkpoint {
            Self::check_checkpoint(&tx, checkpoint)?;
            Checkpoint {
                seq: checkpoint.seq,
                hash: checkpoint.hash.clone(),
            }
        } else {
            // Indexed reverse seek: at most 129 rows, never COUNT/OFFSET(history).
            let anchor = tx
                .query_row(
                    "SELECT seq,hash FROM events ORDER BY seq DESC LIMIT 1 OFFSET ?1",
                    [super::sqlint(STARTUP_TAIL_ROWS)?],
                    |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
                )
                .optional()?;
            let anchor = match anchor {
                Some((seq, hash)) => Checkpoint {
                    seq: seq.try_into()?,
                    hash,
                },
                None => Checkpoint {
                    seq: 0,
                    hash: String::new(),
                },
            };
            Self::check_checkpoint(&tx, &anchor)?;
            anchor
        };
        let (verified, count) = Self::verify_after(&tx, anchor)?;
        if checkpoint.as_ref() != Some(&verified) {
            tx.execute(
                "INSERT INTO verification_checkpoint(id,seq,hash) VALUES(1,?1,?2)
                 ON CONFLICT(id) DO UPDATE SET seq=excluded.seq,hash=excluded.hash",
                params![super::sqlint(verified.seq)?, verified.hash],
            )?;
        }
        tx.commit()?;
        Ok((log, count))
    }

    fn check_checkpoint(db: &rusqlite::Connection, checkpoint: &Checkpoint) -> Result<()> {
        if checkpoint.seq == 0 {
            return if checkpoint.hash.is_empty() {
                Ok(())
            } else {
                Err(invalid("invalid event checkpoint genesis hash"))
            };
        }
        let row = db
            .query_row(
                "SELECT payload,hash,prev FROM events WHERE seq=?1",
                [super::sqlint(checkpoint.seq)?],
                |row| {
                    Ok((
                        row.get::<_, Vec<u8>>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )
            .optional()?
            .ok_or_else(|| invalid("event checkpoint row missing"))?;
        if row.0.len() > MAX_EVENT_BYTES {
            return Err(invalid("oversized stored event"));
        }
        let record: Envelope = serde_json::from_slice(&row.0)?;
        // Rehash the payload too: comparing only the SQL hash would miss a
        // payload changed while its cached SQL hash stayed unchanged. The
        // checkpoint hash binds the already-verified signature without another
        // Ed25519 verification of the checkpoint row.
        if row.1 != checkpoint.hash
            || record.hash()? != checkpoint.hash
            || record.seq != checkpoint.seq
            || record.prev != row.2
        {
            return Err(invalid("event checkpoint hash mismatch"));
        }
        Ok(())
    }

    fn verify_after(db: &rusqlite::Connection, mut head: Checkpoint) -> Result<(Checkpoint, u64)> {
        let mut stmt =
            db.prepare("SELECT seq,payload,hash,prev FROM events WHERE seq>?1 ORDER BY seq")?;
        let mut rows = stmt.query([super::sqlint(head.seq)?])?;
        let mut count = 0;
        while let Some(row) = rows.next()? {
            let seq: u64 = row.get::<_, i64>(0)?.try_into()?;
            let bytes: Vec<u8> = row.get(1)?;
            if bytes.len() > MAX_EVENT_BYTES {
                return Err(invalid("oversized stored event"));
            }
            let record: Envelope = serde_json::from_slice(&bytes)?;
            let hash: String = row.get(2)?;
            let prev: String = row.get(3)?;
            if seq != head.seq + 1
                || record.seq != seq
                || prev != head.hash
                || record.prev != head.hash
                || hash != record.hash()?
            {
                return Err(invalid(format!("broken event chain at sequence {seq}")));
            }
            record.verify()?;
            count += 1;
            head = Checkpoint { seq, hash };
        }
        Ok((head, count))
    }
    pub fn last_sequence(&self) -> Result<u64> {
        Ok(sqlite::open(&self.path)?
            .query_row("SELECT COALESCE(MAX(seq),0) FROM events", [], |r| {
                r.get::<_, i64>(0)
            })?
            .try_into()?)
    }
    pub fn at(&self, sequence: u64) -> Result<Option<Envelope>> {
        let bytes: Option<Vec<u8>> = sqlite::open(&self.path)?
            .query_row(
                "SELECT payload FROM events WHERE seq=?1",
                [super::sqlint(sequence)?],
                |r| r.get(0),
            )
            .optional()?;
        bytes.map(|b| Ok(serde_json::from_slice(&b)?)).transpose()
    }
}

impl Events for EventLog {
    fn append(&self, event: Draft, key: &SigningKey) -> Result<Envelope> {
        if serde_json::to_vec(&event)?.len() > MAX_EVENT_BYTES - 1024 {
            return Err(invalid("event too large"));
        }
        let mut db = sqlite::open(&self.path)?;
        let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let existing: Option<Vec<u8>> = tx
            .query_row(
                "SELECT payload FROM events WHERE id=?1",
                [event.id.to_string()],
                |r| r.get(0),
            )
            .optional()?;
        if let Some(bytes) = existing {
            let existing: Envelope = serde_json::from_slice(&bytes)?;
            existing.verify()?;
            if existing.event != event
                || existing.provenance.signer != key.verifying_key().to_bytes()
            {
                return Err(invalid("event id conflicts with retained event"));
            }
            return Ok(existing);
        }
        let (last, prev): (i64, String) = tx
            .query_row(
                "SELECT seq,hash FROM events ORDER BY seq DESC LIMIT 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?
            .unwrap_or((0, String::new()));
        let envelope = Envelope::signed(
            event,
            u64::try_from(
                last.checked_add(1)
                    .ok_or_else(|| invalid("event sequence exhausted"))?,
            )?,
            prev,
            key,
        )?;
        let hash = envelope.hash()?;
        tx.execute(
            "INSERT INTO events VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
            params![
                super::sqlint(envelope.seq)?,
                envelope.event.id.to_string(),
                super::sqlint(envelope.event.ts)?,
                envelope.event.subject,
                envelope.event.source,
                serde_json::to_vec(&envelope)?,
                envelope.provenance.signer.as_slice(),
                envelope.provenance.sig,
                envelope.prev,
                hash
            ],
        )?;
        // A locally signed append extends a verified head atomically. Never
        // jump over rows imported by an older writer: open must verify those.
        tx.execute(
            "UPDATE verification_checkpoint SET seq=?1,hash=?2 WHERE id=1 AND seq=?3 AND hash=?4",
            params![super::sqlint(envelope.seq)?, hash, last, envelope.prev],
        )?;
        tx.commit()?;
        Ok(envelope)
    }
    fn after(&self, sequence: u64, limit: usize) -> Result<Vec<Envelope>> {
        let db = sqlite::open(&self.path)?;
        let mut stmt =
            db.prepare("SELECT payload FROM events WHERE seq>?1 ORDER BY seq LIMIT ?2")?;
        let records = stmt
            .query_map(
                params![super::sqlint(sequence)?, limit.min(1024) as i64],
                |r| r.get::<_, Vec<u8>>(0),
            )?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        records
            .into_iter()
            .map(|bytes| {
                let event: Envelope = serde_json::from_slice(&bytes)?;
                event.verify()?;
                Ok(event)
            })
            .collect()
    }
    fn verify_chain(&self) -> Result<u64> {
        let db = sqlite::open(&self.path)?;
        let mut stmt = db.prepare("SELECT seq,payload,hash,prev FROM events ORDER BY seq")?;
        let mut rows = stmt.query([])?;
        let (mut sequence, mut previous) = (0, String::new());
        while let Some(row) = rows.next()? {
            let seq: u64 = row.get::<_, i64>(0)?.try_into()?;
            let bytes: Vec<u8> = row.get(1)?;
            if bytes.len() > MAX_EVENT_BYTES {
                return Err(invalid("oversized stored event"));
            }
            let record: Envelope = serde_json::from_slice(&bytes)?;
            let hash: String = row.get(2)?;
            let prev: String = row.get(3)?;
            if seq != sequence + 1
                || record.seq != seq
                || prev != previous
                || record.prev != previous
                || hash != record.hash()?
            {
                return Err(invalid("broken event chain"));
            }
            record.verify()?;
            sequence = seq;
            previous = hash;
        }
        Ok(sequence)
    }
}

fn history_failure(root: &Path, path: &Path, error: impl std::fmt::Display) {
    let message = format!("events.sqlite at {}: {error}", path.display());
    // stderr remains visible if the bounded diagnostic datagram cannot be sent.
    eprintln!("event history verification failed: {message}");
    bbt_rt::error_journal::report_to(
        root,
        bbt_rt::error_journal::Failure::new("event_log", "verify_history", message),
    );
}

/// Start an independent historical audit for every retained stream. Call only
/// after writing the logic readiness line. No task is joined by the bus/logic
/// owner; a failed stream cannot stop other streams' audits or message traffic.
/// Each process start audits from genesis, even when a checkpoint already exists.
pub fn verify_history_after_ready(root: &Path, runtime: &tokio::runtime::Handle) {
    let root = root.to_owned();
    runtime.spawn(async move {
        let scan_root = root.clone();
        let paths = tokio::task::spawn_blocking(move || -> std::io::Result<Vec<PathBuf>> {
            std::fs::read_dir(scan_root)?
                .map(|entry| entry.map(|e| e.path().join("events.sqlite")))
                .filter(|path| match path {
                    Ok(path) => path.is_file(),
                    Err(_) => true,
                })
                .collect()
        })
        .await;
        let paths = match paths {
            Ok(Ok(paths)) => paths,
            Ok(Err(error)) => {
                history_failure(&root, &root, error);
                return;
            }
            Err(error) => {
                history_failure(&root, &root, error);
                return;
            }
        };
        for path in paths {
            let root = root.clone();
            tokio::spawn(async move {
                let audit_path = path.clone();
                let result = tokio::task::spawn_blocking(move || {
                    EventLog { path: audit_path }.verify_chain()
                })
                .await;
                match result {
                    Ok(Ok(_)) => {}
                    Ok(Err(error)) => history_failure(&root, &path, error),
                    Err(error) => history_failure(&root, &path, error),
                }
            });
        }
    });
}

#[cfg(test)]
mod checkpoint_tests {
    use super::*;

    fn populated(directory: &Path, count: u64) -> EventLog {
        let log = EventLog::open(directory).unwrap();
        let key = SigningKey::from_bytes(&[27; 32]); // public fixture, no custody
        for n in 0..count {
            log.append(Draft::new("fixture/event", "fixture", Value::from(n)), &key)
                .unwrap();
        }
        log
    }

    fn checkpoint_at(log: &EventLog, seq: u64) {
        sqlite::open(&log.path).unwrap().execute(
            "UPDATE verification_checkpoint SET seq=?1,hash=(SELECT hash FROM events WHERE seq=?1) WHERE id=1",
            [super::super::sqlint(seq).unwrap()],
        ).unwrap();
    }

    fn corrupt_payload(log: &EventLog, seq: u64) {
        let mut record = log.at(seq).unwrap().unwrap();
        record.event.source = "tampered".into();
        let db = sqlite::open(&log.path).unwrap();
        db.execute_batch("DROP TRIGGER immutable_update").unwrap();
        db.execute(
            "UPDATE events SET payload=?1 WHERE seq=?2",
            params![
                serde_json::to_vec(&record).unwrap(),
                super::super::sqlint(seq).unwrap()
            ],
        )
        .unwrap();
    }

    // keeps: contract EventLogCheckpoint
    #[test]
    fn opening_n_rows_with_checkpoint_n_minus_one_verifies_exactly_one() {
        let temp = tempfile::tempdir().unwrap();
        let log = populated(temp.path(), 512);
        checkpoint_at(&log, 511);
        let (reopened, verified) = EventLog::open_counted(temp.path()).unwrap();
        assert_eq!(
            verified, 1,
            "counts completed Envelope::verify calls in open"
        );
        assert_eq!(reopened.last_sequence().unwrap(), 512);
        assert_eq!(EventLog::open_counted(temp.path()).unwrap().1, 0);
    }

    // keeps: contract EventLogCheckpoint
    #[test]
    fn checkpoint_hash_and_checkpoint_payload_tampering_are_rejected_on_open() {
        for payload in [false, true] {
            let temp = tempfile::tempdir().unwrap();
            let log = populated(temp.path(), 8);
            checkpoint_at(&log, 7);
            if payload {
                corrupt_payload(&log, 7); // unchanged SQL hash must not conceal this
            } else {
                let db = sqlite::open(&log.path).unwrap();
                db.execute_batch(
                    "DROP TRIGGER immutable_update; UPDATE events SET hash='tampered' WHERE seq=7",
                )
                .unwrap();
            }
            assert!(
                EventLog::open(temp.path())
                    .unwrap_err()
                    .to_string()
                    .contains("checkpoint hash mismatch")
            );
        }
    }

    // keeps: contract EventLogCheckpoint
    #[test]
    fn suffix_tampering_linkage_gaps_and_invalid_signatures_are_rejected_on_open() {
        for fault in ["payload", "prev", "gap", "signature", "missing_checkpoint"] {
            let temp = tempfile::tempdir().unwrap();
            let log = populated(temp.path(), 8);
            checkpoint_at(&log, 6);
            let db = sqlite::open(&log.path).unwrap();
            db.execute_batch("DROP TRIGGER immutable_update; DROP TRIGGER immutable_delete")
                .unwrap();
            match fault {
                "payload" => corrupt_payload_without_trigger(&log, &db, 7),
                "prev" => {
                    db.execute("UPDATE events SET prev='wrong' WHERE seq=7", [])
                        .unwrap();
                }
                "gap" => {
                    db.execute("DELETE FROM events WHERE seq=7", []).unwrap();
                }
                "missing_checkpoint" => {
                    db.execute("DELETE FROM events WHERE seq=6", []).unwrap();
                }
                "signature" => {
                    let mut row = log.at(8).unwrap().unwrap();
                    row.provenance.sig[0] ^= 1;
                    // Keep every chain/hash check valid so Envelope::verify is
                    // the only check that can reject this forged last row.
                    db.execute(
                        "UPDATE events SET payload=?1,hash=?2 WHERE seq=8",
                        params![serde_json::to_vec(&row).unwrap(), row.hash().unwrap()],
                    )
                    .unwrap();
                }
                _ => unreachable!(),
            }
            assert!(EventLog::open(temp.path()).is_err(), "fault={fault}");
            let retained: i64 = db
                .query_row(
                    "SELECT seq FROM verification_checkpoint WHERE id=1",
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(
                retained, 6,
                "a refused open must not advance its checkpoint"
            );
        }
    }

    fn corrupt_payload_without_trigger(log: &EventLog, db: &rusqlite::Connection, seq: u64) {
        let mut row = log.at(seq).unwrap().unwrap();
        row.event.source = "tampered".into();
        db.execute(
            "UPDATE events SET payload=?1 WHERE seq=?2",
            params![
                serde_json::to_vec(&row).unwrap(),
                super::super::sqlint(seq).unwrap()
            ],
        )
        .unwrap();
    }

    // keeps: contract EventLogCheckpoint
    #[test]
    fn old_format_without_checkpoint_table_has_bounded_additive_upgrade() {
        let temp = tempfile::tempdir().unwrap();
        let log = populated(temp.path(), 512);
        let db = sqlite::open(&log.path).unwrap();
        db.execute_batch("DROP TABLE verification_checkpoint")
            .unwrap();
        assert_eq!(
            db.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
                .unwrap(),
            1
        );
        let old_schema: Vec<(String, String)> = db
            .prepare("SELECT name,sql FROM sqlite_master WHERE sql IS NOT NULL ORDER BY name")
            .unwrap()
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap()
            .collect::<std::result::Result<_, _>>()
            .unwrap();
        drop(db);
        assert_eq!(
            EventLog::open_counted(temp.path()).unwrap().1,
            STARTUP_TAIL_ROWS
        );
        let db = sqlite::open(&log.path).unwrap();
        for (name, sql) in old_schema {
            assert_eq!(
                db.query_row(
                    "SELECT sql FROM sqlite_master WHERE name=?1",
                    [&name],
                    |r| r.get::<_, String>(0)
                )
                .unwrap(),
                sql
            );
        }
        assert!(
            db.execute("UPDATE events SET subject='forbidden'", [])
                .is_err()
        );
        assert!(db.execute("DELETE FROM events", []).is_err());
        assert_eq!(log.verify_chain().unwrap(), 512);
    }

    // keeps: contract EventLogCheckpoint
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn historical_tampering_is_reported_after_ready_with_and_without_a_checkpoint() {
        for legacy in [false, true] {
            let root = tempfile::tempdir().unwrap();
            let stream = root.path().join("retained");
            let log = populated(&stream, 256);
            corrupt_payload(&log, 1);
            if legacy {
                sqlite::open(&log.path)
                    .unwrap()
                    .execute_batch("DROP TABLE verification_checkpoint")
                    .unwrap();
            }
            let (_, verified) = EventLog::open_counted(&stream).unwrap();
            assert_eq!(verified, if legacy { STARTUP_TAIL_ROWS } else { 0 });
            // A private real error-journal transport observes the detached audit.
            let sink = std::os::unix::net::UnixDatagram::bind(root.path().join("diagnostics.sock"))
                .unwrap();
            sink.set_read_timeout(Some(std::time::Duration::from_secs(10)))
                .unwrap();
            verify_history_after_ready(root.path(), &tokio::runtime::Handle::current());
            let mut bytes = vec![0; bbt_rt::error_journal::MAX_RECORD];
            let len = sink.recv(&mut bytes).unwrap();
            let failure: Value = serde_json::from_slice(&bytes[..len]).unwrap();
            assert_eq!(failure["source"], "event_log");
            assert_eq!(failure["operation"], "verify_history");
            assert!(
                failure["message"]
                    .as_str()
                    .unwrap()
                    .contains("events.sqlite")
            );
            assert!(
                failure["message"]
                    .as_str()
                    .unwrap()
                    .contains("broken event chain")
            );
        }
    }
}

#[cfg(test)]
mod attribution_tests {
    use super::*;
    #[derive(Serialize)]
    struct LegacyDraft<'a> {
        id: Ulid,
        ts: u64,
        subject: &'a str,
        source: &'a str,
        seat: Option<Ulid>,
        generation: Option<u64>,
        reply_to: Option<String>,
        payload: &'a Value,
    }
    #[derive(Serialize)]
    struct LegacyEnvelope<'a> {
        seq: u64,
        #[serde(flatten)]
        event: LegacyDraft<'a>,
        prev: &'a str,
        provenance: &'a Provenance,
    }
    // keeps: decision #313
    #[test]
    fn real_account_rows_keep_the_original_serialization_and_signing_bytes() {
        let mut event = Draft::new("account/control", "fixture", serde_json::json!({"a":1}));
        event.id = Ulid(9);
        event.ts = 7;
        let legacy = LegacyDraft {
            id: event.id,
            ts: event.ts,
            subject: &event.subject,
            source: &event.source,
            seat: event.seat,
            generation: event.generation,
            reply_to: None,
            payload: &event.payload,
        };
        assert_eq!(
            serde_json::to_vec(&event).unwrap(),
            serde_json::to_vec(&legacy).unwrap()
        );
        let key = SigningKey::from_bytes(&[42; 32]); // public fixture, no native key
        let payload =
            serde_json::to_vec(&(1u64, &legacy, "prev", key.verifying_key().to_bytes())).unwrap();
        let mut previous_signing = b"bboxy/event/v1\0".to_vec();
        previous_signing.extend_from_slice(&(payload.len() as u64).to_le_bytes());
        previous_signing.extend_from_slice(&payload);
        let envelope = Envelope::signed(event.clone(), 1, "prev".into(), &key).unwrap();
        assert_eq!(envelope.signing_bytes().unwrap(), previous_signing);
        assert_eq!(
            envelope.provenance.sig,
            key.sign(&previous_signing).to_bytes()
        );
        let old = LegacyEnvelope {
            seq: 1,
            event: legacy,
            prev: "prev",
            provenance: &envelope.provenance,
        };
        assert_eq!(
            serde_json::to_vec(&envelope).unwrap(),
            serde_json::to_vec(&old).unwrap()
        );
    }
    // keeps: decision #313
    #[test]
    fn test_account_rows_sign_the_host_derived_label_without_changing_payloads() {
        let temp = tempfile::tempdir().unwrap();
        let mut broker =
            crate::bus::Broker::open(temp.path(), SigningKey::from_bytes(&[43; 32])).unwrap();
        let label = "test-account:01ARZ3NDEKTSV4RRFFQ69G5FAV";
        broker.set_issued_by(Some(label.into()));
        let payload = serde_json::json!(["arbitrary", "payload", 7]);
        let row = broker
            .publish(
                "account",
                Draft::new("account/control", "fixture", payload.clone()),
            )
            .unwrap();
        assert_eq!(row.event.issued_by.as_deref(), Some(label));
        assert_eq!(row.event.payload, payload);
        row.verify().unwrap();
        assert_eq!(serde_json::to_value(&row).unwrap()["issued_by"], label);
        let mut forged = Draft::new("account/control", "fixture", Value::Null);
        forged.issued_by = Some("test-account:another-run".into());
        assert!(
            broker
                .publish("account", forged)
                .unwrap_err()
                .to_string()
                .contains("test_account_label_mismatch")
        );
    }
}
