// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Cancel pending wakes in the inbox transaction; retained content is immutable.
use super::*;
use bbt_proto::messaging::{Body, MessageKind, WakeReason};
use std::collections::BTreeSet;

pub(super) fn install(db: &rusqlite::Connection) -> Result<()> {
    db.execute_batch("CREATE TABLE IF NOT EXISTS msg_wake_cancelled(message INTEGER NOT NULL REFERENCES msg_messages(seq),recipient TEXT NOT NULL,reason TEXT NOT NULL,PRIMARY KEY(message,recipient));
        CREATE TABLE IF NOT EXISTS msg_task_state(task TEXT PRIMARY KEY,status TEXT NOT NULL);
        CREATE INDEX IF NOT EXISTS msg_task_delegations ON msg_messages(COALESCE(json_extract(CAST(envelope AS TEXT),'$.body.task_id'),json_extract(CAST(envelope AS TEXT),'$.intent.body.task_id'))) WHERE COALESCE(json_extract(CAST(envelope AS TEXT),'$.body.event'),json_extract(CAST(envelope AS TEXT),'$.intent.body.event'))='delegate';")?;
    Ok(())
}

fn cancelled(
    db: &rusqlite::Connection,
    sequence: u64,
    recipient: Recipient,
) -> Result<Option<WakeReason>> {
    let reason: Option<String> = db.query_row(
        "SELECT reason FROM msg_wake_cancelled WHERE message=?1 AND recipient IN ('',?2) ORDER BY reason='superseded' DESC LIMIT 1",
        params![sqlint(sequence)?, recipient.to_string()], |row| row.get(0),
    ).optional()?;
    reason
        .map(|reason| serde_json::from_value(serde_json::Value::String(reason)).map_err(Into::into))
        .transpose()
}

pub(super) fn reason(
    db: &rusqlite::Connection,
    message: &StoredMessage,
    recipient: Recipient,
) -> Result<Option<WakeReason>> {
    if let Some(reason) = cancelled(db, message.sequence, recipient)? {
        return Ok(Some(reason));
    }
    if let Body::Task { task_id, event, .. } = message.view().body
        && event == "delegate"
    {
        let terminal: bool = db.query_row("SELECT EXISTS(SELECT 1 FROM msg_task_state WHERE task=?1 AND status IN ('accepted','rejected'))", [task_id.to_string()], |row| row.get(0))?;
        if terminal {
            return Ok(Some(WakeReason::Done));
        }
    }
    Ok(None)
}

fn cancel(
    db: &rusqlite::Transaction<'_>,
    sequence: u64,
    recipient: Option<Recipient>,
    reason: WakeReason,
    now: u64,
) -> Result<()> {
    let recipient_key = recipient.map(String::from).unwrap_or_default();
    let encoded = serde_json::to_value(reason)?;
    db.execute("INSERT INTO msg_wake_cancelled VALUES(?1,?2,?3) ON CONFLICT(message,recipient) DO UPDATE SET reason=CASE WHEN msg_wake_cancelled.reason='superseded' THEN msg_wake_cancelled.reason ELSE excluded.reason END",
        params![sqlint(sequence)?, recipient_key, encoded.as_str().unwrap()])?;
    let receipts: Vec<Receipt> = {
        let mut query=db.prepare("SELECT data FROM msg_deliveries WHERE message=?1 AND (?2='' OR recipient=?2) AND evidence='recipient-stored' AND attempt IS NULL")?;
        query
            .query_map(params![sqlint(sequence)?, recipient_key], |row| {
                row.get::<_, Vec<u8>>(0)
            })?
            .map(|row| Ok(serde_json::from_slice(&row?)?))
            .collect::<Result<_>>()?
    };
    for mut receipt in receipts {
        let wake = Some(ReceiptWake::None {
            reason: cancelled(db, sequence, receipt.recipient)?,
        });
        if receipt.wake == wake {
            continue;
        }
        receipt.wake = wake;
        receipt.note = format!("pending wake cancelled: {}", encoded.as_str().unwrap());
        save_receipt(db, &receipt)?;
        receipt_event(db, &receipt, now)?;
        retire_digests(
            db,
            receipt.recipient,
            receipt.agent.as_deref(),
            std::iter::once(sequence),
        )?;
    }
    Ok(())
}

pub(super) fn admitted(
    db: &rusqlite::Transaction<'_>,
    message: &StoredMessage,
    sender: Recipient,
    now: u64,
) -> Result<()> {
    let view = message.view();
    let mut supersedes = view.supersedes;
    let mut seen = BTreeSet::new();
    while let Some(id) = supersedes {
        if id == view.id || !seen.insert(id) {
            return Err(invalid("cyclic supersedes chain"));
        }
        let (sequence, owner, request): (u64, String, Vec<u8>) = db.query_row(
            "SELECT seq,sender,request FROM msg_messages WHERE id=?1",
            [id.to_string()],
            |row| Ok((u64_at(row, 0)?, row.get(1)?, row.get(2)?)),
        )?;
        if owner != sender.to_string() {
            return Err(invalid("supersedes requires the original sender"));
        }
        cancel(db, sequence, None, WakeReason::Superseded, now)?;
        supersedes = serde_json::from_slice::<SendRequest>(&request)?.supersedes;
    }
    if view.kind.class() != MessageClass::Conversation {
        return Ok(());
    }
    // Only the addressed recipient can answer; a third party's branch cannot.
    let mut parent = view.reply_to;
    seen.clear();
    while let Some(id) = parent {
        if !seen.insert(id) {
            return Err(invalid("cyclic reply chain"));
        }
        let (sequence, request): (u64, Vec<u8>) = db.query_row(
            "SELECT seq,request FROM msg_messages WHERE id=?1",
            [id.to_string()],
            |row| Ok((u64_at(row, 0)?, row.get(1)?)),
        )?;
        let request: SendRequest = serde_json::from_slice(&request)?;
        let addressed:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM msg_deliveries WHERE message=?1 AND recipient=?2 AND evidence!='sender-stored')",params![sqlint(sequence)?,sender.to_string()],|row|row.get(0))?;
        if addressed
            && request.kind.class() == MessageClass::Conversation
            && request.kind != MessageKind::Task
        {
            cancel(db, sequence, Some(sender), WakeReason::Done, now)?;
        }
        parent = request.reply_to;
    }
    Ok(())
}

impl Ledger {
    pub(crate) fn task_state(&self, task: Ulid, status: &str, now: u64) -> Result<()> {
        let mut db = self.store.connection()?;
        let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        tx.execute("INSERT INTO msg_task_state VALUES(?1,?2) ON CONFLICT(task) DO UPDATE SET status=excluded.status",params![task.to_string(),status])?;
        if matches!(status, "accepted" | "rejected") {
            let sequences: Vec<u64> = {
                let mut q=tx.prepare("SELECT seq FROM msg_messages INDEXED BY msg_task_delegations WHERE COALESCE(json_extract(CAST(envelope AS TEXT),'$.body.event'),json_extract(CAST(envelope AS TEXT),'$.intent.body.event'))='delegate' AND COALESCE(json_extract(CAST(envelope AS TEXT),'$.body.task_id'),json_extract(CAST(envelope AS TEXT),'$.intent.body.task_id'))=?1")?;
                q.query_map([task.to_string()], |row| u64_at(row, 0))?
                    .collect::<std::result::Result<_, _>>()?
            };
            for sequence in sequences {
                cancel(&tx, sequence, None, WakeReason::Done, now)?;
            }
        }
        tx.commit()?;
        Ok(())
    }
    pub(crate) fn pending_tasks(&self) -> Result<Vec<Ulid>> {
        let db = self.store.connection()?;
        let mut q=db.prepare("SELECT DISTINCT COALESCE(json_extract(CAST(m.envelope AS TEXT),'$.body.task_id'),json_extract(CAST(m.envelope AS TEXT),'$.intent.body.task_id')) FROM msg_deliveries d INDEXED BY msg_native_ready JOIN msg_messages m ON m.seq=d.message WHERE d.evidence='recipient-stored' AND json_extract(CAST(d.data AS TEXT),'$.wake') IS NOT 'none' AND COALESCE(json_extract(CAST(m.envelope AS TEXT),'$.body.event'),json_extract(CAST(m.envelope AS TEXT),'$.intent.body.event'))='delegate'")?;
        q.query_map([], |row| row.get::<_, String>(0))?
            .map(|id| Ok(id?.parse()?))
            .collect()
    }
}
