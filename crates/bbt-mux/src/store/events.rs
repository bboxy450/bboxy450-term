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
        drop(db);
        log.verify_chain()?;
        Ok(log)
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
                envelope.hash()?
            ],
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
