//! Read-only control results and their durable audit outbox share one transaction.

use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use ulid::Ulid;

use super::{Actor, Call, MAX_RESULT, error};
use crate::store::{Result, Store, digest, events::Draft, invalid, sqlite::SqliteStore};

#[derive(Clone)]
pub(crate) struct Journal {
    store: SqliteStore,
}

#[derive(Serialize, Deserialize)]
struct Entry {
    actor: Actor,
    tool: String,
    args_digest: String,
    result: Value,
}

impl Journal {
    pub fn new(store: SqliteStore) -> Self {
        Self { store }
    }

    pub fn call(
        &self,
        actor: &Actor,
        call: &Call,
        dispatch: impl FnOnce() -> Value,
    ) -> Result<Value> {
        let hash = digest(&serde_json::to_vec(&call.args)?);
        let key = format!("control/op/{}", call.op_id);
        let retained = self.store.get(&key)?;
        let (result, replay) = if let Some(bytes) = retained {
            let old: Entry = serde_json::from_slice(&bytes)?;
            if old.actor != *actor || old.tool != call.tool || old.args_digest != hash {
                (
                    error(
                        call.op_id,
                        "conflict",
                        "operation ID belongs to different arguments or authority",
                    ),
                    false,
                )
            } else {
                (old.result, true)
            }
        } else {
            let value = dispatch();
            let value = if serde_json::to_vec(&value)?.len() > MAX_RESULT {
                error(
                    call.op_id,
                    "response_bound",
                    "result exceeds the bounded response size",
                )
            } else {
                value
            };
            (value, false)
        };
        let mut event = Draft::new(
            format!("seat/{}/control", actor.seat_id),
            "mcp",
            json!({"op_id":call.op_id,"tool":call.tool,"actor":actor,
                "outcome":if replay {"replay"} else if result["isError"]==true {"rejected"} else {"applied"},
                "args_digest":hash,"replay":replay}),
        );
        event.seat = Some(actor.seat_id);
        event.generation = Some(actor.process_generation);
        let mut db = self.store.connection()?;
        let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        // This pass has reads only. Later effectful tools must journal admission
        // before their effects rather than running an effect in this closure.
        if retained_is_absent(&tx, &key)? {
            tx.execute(
                "INSERT INTO kv VALUES(?1,?2)",
                params![
                    key,
                    serde_json::to_vec(&Entry {
                        actor: actor.clone(),
                        tool: call.tool.clone(),
                        args_digest: hash,
                        result: result.clone(),
                    })?
                ],
            )?;
        }
        tx.execute(
            "INSERT INTO kv VALUES(?1,?2)",
            params![
                format!("control/audit/{}", event.id),
                serde_json::to_vec(&event)?
            ],
        )?;
        tx.commit()?;
        Ok(result)
    }

    pub fn pending(&self, limit: usize) -> Result<Vec<Draft>> {
        let db = self.store.connection()?;
        let mut q = db.prepare(
            "SELECT value FROM kv WHERE substr(key,1,14)='control/audit/' ORDER BY key LIMIT ?1",
        )?;
        q.query_map([limit.min(128) as i64], |row| row.get::<_, Vec<u8>>(0))?
            .map(|bytes| Ok(serde_json::from_slice(&bytes?)?))
            .collect()
    }

    pub fn published(&self, event: Ulid) -> Result<()> {
        self.store.connection()?.execute(
            "DELETE FROM kv WHERE key=?1",
            [format!("control/audit/{event}")],
        )?;
        Ok(())
    }
}

fn retained_is_absent(tx: &rusqlite::Transaction<'_>, key: &str) -> Result<bool> {
    let old: Option<Vec<u8>> = tx
        .query_row("SELECT value FROM kv WHERE key=?1", [key], |r| r.get(0))
        .optional()?;
    if let Some(bytes) = old {
        let _: Entry =
            serde_json::from_slice(&bytes).map_err(|_| invalid("invalid control journal"))?;
        Ok(false)
    } else {
        Ok(true)
    }
}
