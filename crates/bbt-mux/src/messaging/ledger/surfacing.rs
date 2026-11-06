// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Recipient-local unread set, independent of native delivery and explicit ACK.
use super::*;
use serde_json::{Value, json};

pub(super) fn install(db: &rusqlite::Connection) -> Result<()> {
    db.execute_batch("CREATE TABLE IF NOT EXISTS msg_surface(message INTEGER NOT NULL,recipient TEXT NOT NULL,hinted_ms INTEGER,read_ms INTEGER,PRIMARY KEY(message,recipient));
        CREATE INDEX IF NOT EXISTS msg_surface_unread ON msg_surface(recipient,message) WHERE read_ms IS NULL;
        CREATE TABLE IF NOT EXISTS msg_surface_migration(version INTEGER PRIMARY KEY);
        CREATE TABLE IF NOT EXISTS msg_surface_worker(recipient TEXT PRIMARY KEY,at_ms INTEGER NOT NULL);")?;
    // Legacy messages become unread once. Never infer a read from native admission.
    db.execute_batch("INSERT OR IGNORE INTO msg_surface SELECT message,recipient,NULL,NULL FROM msg_deliveries WHERE agent='' AND evidence NOT IN ('sender-stored','application-acknowledged') AND recipient!=(SELECT sender FROM msg_messages m WHERE m.seq=message) AND (class=1 OR (SELECT COALESCE(json_extract(CAST(envelope AS TEXT),'$.kind'),json_extract(CAST(envelope AS TEXT),'$.intent.request.kind')) FROM msg_messages m WHERE m.seq=message) IN ('text','chunk','decision','task')) AND NOT EXISTS(SELECT 1 FROM msg_surface_migration WHERE version=1);
        INSERT OR IGNORE INTO msg_surface_migration VALUES(1);")?;
    Ok(())
}

impl Ledger {
    /// Project only after the canonical tool outcome has been journaled. A fresh
    /// projection also covers retries without changing their original receipt.
    pub(crate) fn surface_result(
        &self,
        seat: Ulid,
        call: &crate::mcp::Call,
        mut value: Value,
        now: u64,
    ) -> Result<Value> {
        let mut db = self.store.connection()?;
        let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        if call.tool == "message_list" && value["isError"] != true {
            for item in value["structuredContent"]["page"]["items"]
                .as_array()
                .into_iter()
                .flatten()
            {
                if let Some(sequence) = item["sequence"].as_u64() {
                    tx.execute("UPDATE msg_surface SET read_ms=?3 WHERE message=?1 AND recipient=?2 AND read_ms IS NULL",
                        params![sqlint(sequence)?,seat.to_string(),sqlint(now)?])?;
                }
            }
        }
        if call.tool == "message_receipts" {
            for item in value["structuredContent"]["page"]["items"]
                .as_array_mut()
                .into_iter()
                .flatten()
            {
                let Some(sequence) = item["message_sequence"].as_u64() else {
                    continue;
                };
                let Some(recipient) = item["recipient"].as_str() else {
                    continue;
                };
                let state: Option<(Option<i64>,Option<i64>)> = tx.query_row(
                    "SELECT hinted_ms,read_ms FROM msg_surface WHERE message=?1 AND recipient=?2",
                    params![sqlint(sequence)?,recipient], |r| Ok((r.get(0)?,r.get(1)?))).optional()?;
                if let Some((hinted, read)) = state {
                    item["surfacing"] = json!({"hinted_at_ms":hinted,"read_at_ms":read,"model_acknowledged":item["evidence"]=="application-acknowledged"});
                }
            }
        }
        let worker_due: bool = tx.query_row(
            "SELECT NOT EXISTS(SELECT 1 FROM msg_surface_worker WHERE recipient=?1 AND (at_ms>?2 OR ?3<60000))",
            params![seat.to_string(), sqlint(now.saturating_sub(60_000))?,sqlint(now)?],
            |r| r.get(0),
        )?;
        let count: i64 = tx.query_row("SELECT COUNT(*) FROM msg_surface s JOIN msg_deliveries d ON d.message=s.message AND d.recipient=s.recipient AND d.agent='' WHERE s.recipient=?1 AND s.read_ms IS NULL AND NOT EXISTS(SELECT 1 FROM msg_wake_cancelled c WHERE c.message=s.message AND c.recipient IN ('',s.recipient)) AND (d.class=1 OR ?2)",
            params![seat.to_string(),worker_due],|r|r.get(0))?;
        let candidates = {
            // Oracle traffic always precedes workers. Oldest unhinted first,
            // then rotate unread messages, so batches larger than five progress.
            let mut query = tx.prepare_cached("SELECT m.seq,m.envelope,m.held_out,d.class,s.hinted_ms FROM msg_surface s INDEXED BY msg_surface_unread JOIN msg_messages m ON m.seq=s.message JOIN msg_deliveries d ON d.message=s.message AND d.recipient=s.recipient AND d.agent='' WHERE s.recipient=?1 AND s.read_ms IS NULL AND NOT EXISTS(SELECT 1 FROM msg_wake_cancelled c WHERE c.message=s.message AND c.recipient IN ('',s.recipient)) AND d.evidence!='application-acknowledged' AND (d.class=1 OR ?2) ORDER BY d.class DESC,s.hinted_ms,m.seq LIMIT 5")?;
            query
                .query_map(params![seat.to_string(), worker_due], |r| {
                    Ok((
                        u64_at(r, 0)?,
                        r.get::<_, Vec<u8>>(1)?,
                        r.get::<_, Option<String>>(2)?,
                        r.get::<_, i32>(3)?,
                        r.get::<_, Option<i64>>(4)?,
                    ))
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?
        };
        let mut entries = Vec::new();
        let mut selected = Vec::new();
        let mut worker = false;
        for (seq, raw, held, class, hinted) in candidates {
            if class == 0 && worker {
                continue;
            }
            let message = self.decode_message(seq, &raw, held)?;
            let view = message.view();
            if view.seat == seat {
                continue;
            }
            let preview = match view.body.text() {
                Some(text) => text.to_owned(),
                None => serde_json::to_string(view.body)?,
            };
            let mut entry = json!({"message_id":view.id,"sender":view.seat,"class":if class==1 {"oracle"} else {"worker"},"first_line":preview.lines().next().unwrap_or_default().chars().take(350).collect::<String>()});
            while serde_json::to_vec(&entry)?.len() > 512 {
                let line = entry["first_line"].as_str().unwrap();
                let end = line.char_indices().next_back().map_or(0, |(i, _)| i);
                entry["first_line"] = line[..end].into();
            }
            entries.push(entry);
            selected.push((seq, hinted));
            worker |= class == 0;
        }
        // Receipt metadata changes only the host's own JSON mirror. Leave
        // arbitrary plugin content intact on all other tool results.
        if call.tool == "message_receipts" {
            value = crate::mcp::hints::remirror(value);
        }
        if !entries.is_empty() {
            let hint = json!({"reason":"Unread peer messages are available during this turn; read them before continuing.","method":"tools/call","params":{"name":"message_list","arguments":{"after":selected.iter().map(|(seq,_)|*seq).min().unwrap().saturating_sub(1),"limit":128}},"kind":"unread_messages","count":count,"messages":entries});
            let projected = crate::mcp::hints::with_message_hint(call, value.clone(), hint);
            // Record only hints that survived the final wire-size projection.
            if projected["structuredContent"]["hints"]
                .as_array()
                .is_some_and(|h| h.iter().any(|h| h["kind"] == "unread_messages"))
            {
                for (seq, first) in selected {
                    tx.execute(
                        "UPDATE msg_surface SET hinted_ms=?3 WHERE message=?1 AND recipient=?2",
                        params![sqlint(seq)?, seat.to_string(), sqlint(now)?],
                    )?;
                    if first.is_none() {
                        let data:Vec<u8> = tx.query_row("SELECT data FROM msg_deliveries WHERE message=?1 AND recipient=?2 AND agent=''",params![sqlint(seq)?,seat.to_string()],|r|r.get(0))?;
                        let mut receipt: Receipt = serde_json::from_slice(&data)?;
                        receipt.evidence = Evidence::Hinted;
                        receipt.note =
                            "bounded MCP hint returned; model consumption unverified".into();
                        // Event only: preserve the current native/ACK evidence.
                        receipt_event(&tx, &receipt, now)?;
                    }
                }
                if worker {
                    tx.execute("INSERT INTO msg_surface_worker VALUES(?1,?2) ON CONFLICT(recipient) DO UPDATE SET at_ms=excluded.at_ms",params![seat.to_string(),sqlint(now)?])?;
                }
                value = projected;
            }
        }
        tx.commit()?;
        Ok(value)
    }
}
