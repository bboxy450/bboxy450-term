// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! A recipient-local, durable bound on automatic native wake admissions.
use super::*;

pub(crate) const LIMIT: u64 = 8;
pub(crate) const WINDOW_MS: u64 = 60_000;
const SELF_NOTE: &str = "self-authored message retained in inbox; no automatic wake";
const OPEN_NOTE: &str = "automatic wake circuit open: eight admissions within 60 seconds; messages remain in inbox; a new direct operator message resets the circuit";

#[derive(Debug, Serialize)]
pub(crate) struct Status {
    pub open: bool,
    pub admissions: u64,
    pub window_started_ms: u64,
    pub retained_through_sequence: u64,
    pub limit: u64,
    pub window_ms: u64,
}

pub(super) fn install(db: &rusqlite::Connection) -> Result<()> {
    db.execute_batch("CREATE TABLE IF NOT EXISTS msg_wake_circuits(recipient TEXT PRIMARY KEY,epoch TEXT NOT NULL,started_ms INTEGER NOT NULL,admissions INTEGER NOT NULL,open INTEGER NOT NULL,retained_through INTEGER NOT NULL);
        CREATE TABLE IF NOT EXISTS msg_wake_reservations(attempt TEXT PRIMARY KEY,recipient TEXT NOT NULL,epoch TEXT NOT NULL);")?;
    Ok(())
}

pub(super) fn status(db: &rusqlite::Connection, recipient: Recipient) -> Result<Status> {
    let (open, admissions, window_started_ms, retained_through_sequence) = db
        .query_row(
            "SELECT open,admissions,started_ms,retained_through FROM msg_wake_circuits WHERE recipient=?1",
            [recipient.to_string()],
            |row| Ok((row.get(0)?, u64_at(row, 1)?, u64_at(row, 2)?, u64_at(row, 3)?)),
        )
        .optional()?
        .unwrap_or((false, 0, 0, 0));
    Ok(Status {
        open,
        admissions,
        window_started_ms,
        retained_through_sequence,
        limit: LIMIT,
        window_ms: WINDOW_MS,
    })
}

/// Only a newly admitted, direct operator conversation resets a latched circuit.
/// Replaying an existing message never reaches insert_deliveries again.
pub(super) fn admission(
    db: &rusqlite::Connection,
    sender: Recipient,
    recipient: Recipient,
    destination: &str,
    class: MessageClass,
) -> Result<Option<&'static str>> {
    if sender == recipient {
        return Ok(Some(SELF_NOTE));
    }
    if sender.is_operator() {
        if class == MessageClass::Conversation
            && recipient
                .seat()
                .is_some_and(|seat| destination == format!("seats/{seat}/inbox"))
        {
            db.execute(
                "UPDATE msg_wake_circuits SET epoch=?2,started_ms=0,admissions=0,open=0 WHERE recipient=?1",
                params![recipient.to_string(), Ulid::new().to_string()],
            )?;
        }
        return Ok(None);
    }
    Ok(status(db, recipient)?.open.then_some(OPEN_NOTE))
}

/// Reservation and attempt journal share one transaction. An unknown native
/// outcome spends credit; only a definite pre-effect failure can refund it.
pub(super) fn reserve(db: &rusqlite::Connection, work: &Work, now: u64) -> Result<bool> {
    if work
        .messages
        .iter()
        .any(|message| message.view().seat == work.recipient)
    {
        return Ok(false);
    }
    let automated: bool = work
        .messages
        .iter()
        .any(|message| message.view().seat != Ulid::nil());
    if !automated {
        return Ok(true);
    }
    let recipient = Recipient::Seat(work.recipient);
    let s = status(db, recipient)?;
    if s.open
        || work
            .messages
            .iter()
            .any(|m| m.view().seat != Ulid::nil() && m.sequence <= s.retained_through_sequence)
    {
        return Ok(false);
    }
    if (s.window_started_ms == 0 && s.admissions == 0)
        || now.saturating_sub(s.window_started_ms) >= WINDOW_MS
    {
        db.execute("INSERT INTO msg_wake_circuits VALUES(?1,?2,?3,0,0,0) ON CONFLICT(recipient) DO UPDATE SET epoch=excluded.epoch,started_ms=excluded.started_ms,admissions=0,open=0",
            params![recipient.to_string(),Ulid::new().to_string(),sqlint(now)?])?;
    }
    let count: u64 = db.query_row(
        "SELECT admissions FROM msg_wake_circuits WHERE recipient=?1",
        [recipient.to_string()],
        |row| u64_at(row, 0),
    )?;
    if count >= LIMIT {
        db.execute(
            "UPDATE msg_wake_circuits SET open=1,retained_through=(SELECT COALESCE(MAX(message),0) FROM msg_deliveries WHERE recipient=?1 AND evidence!='sender-stored') WHERE recipient=?1",
            [recipient.to_string()],
        )?;
        return Ok(false);
    }
    db.execute(
        "UPDATE msg_wake_circuits SET admissions=admissions+1 WHERE recipient=?1",
        [recipient.to_string()],
    )?;
    db.execute("INSERT INTO msg_wake_reservations SELECT ?1,recipient,epoch FROM msg_wake_circuits WHERE recipient=?2", params![work.id.to_string(),recipient.to_string()])?;
    Ok(true)
}

pub(super) fn finish(db: &rusqlite::Connection, attempt: Ulid, no_effect: bool) -> Result<()> {
    if no_effect {
        db.execute("UPDATE msg_wake_circuits SET admissions=admissions-1 WHERE admissions>0 AND EXISTS(SELECT 1 FROM msg_wake_reservations r WHERE r.attempt=?1 AND r.recipient=msg_wake_circuits.recipient AND r.epoch=msg_wake_circuits.epoch)", [attempt.to_string()])?;
    }
    db.execute(
        "DELETE FROM msg_wake_reservations WHERE attempt=?1",
        [attempt.to_string()],
    )?;
    Ok(())
}

/// Also catches pending rows written by an older binary. Bounded recipient-local
/// pages keep storage/readback intact and never touch another seat's credit.
pub(super) fn suppress(db: &rusqlite::Transaction<'_>, recipient: Ulid, now: u64) -> Result<()> {
    let state = status(db, Recipient::Seat(recipient))?;
    let rows: Vec<(Vec<u8>, bool)> = {
        let mut q=db.prepare("SELECT d.data,m.sender=d.recipient FROM msg_deliveries d INDEXED BY msg_native_ready JOIN msg_messages m ON m.seq=d.message WHERE d.recipient=?1 AND d.evidence='recipient-stored' AND json_extract(CAST(d.data AS TEXT),'$.wake') IS NOT 'none' AND d.attempt IS NULL AND (m.sender=d.recipient OR ((?2 OR d.message<=?3) AND m.sender NOT LIKE 'operator:%' AND m.sender NOT LIKE 'operator-local:%')) ORDER BY d.message,d.agent LIMIT 256")?;
        q.query_map(
            params![
                recipient.to_string(),
                state.open,
                sqlint(state.retained_through_sequence)?
            ],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?
        .collect::<std::result::Result<_, _>>()?
    };
    for (raw, own) in rows {
        let mut receipt: Receipt = serde_json::from_slice(&raw)?;
        receipt.wake = Some(ReceiptWake::None {
            reason: Some(bbt_proto::messaging::WakeReason::Policy),
        });
        receipt.note = if own { SELF_NOTE } else { OPEN_NOTE }.into();
        save_receipt(db, &receipt)?;
        retire_digests(
            db,
            receipt.recipient,
            receipt.agent.as_deref(),
            [receipt.message_sequence],
        )?;
        receipt_event(db, &receipt, now)?;
    }
    Ok(())
}
