//! Read-only control results and their durable audit outbox share one transaction.

use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use ulid::Ulid;

use super::{Actor, Call, MAX_RESULT, error};
use crate::store::{Result, digest, events::Draft, sqlite::SqliteStore};

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
        // The lookup and result publication use the same write transaction, so
        // overlapping logic owners cannot return two results for one op_id.
        let mut db = self.store.connection()?;
        let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let retained: Option<Vec<u8>> = tx
            .query_row("SELECT value FROM kv WHERE key=?1", [&key], |row| {
                row.get(0)
            })
            .optional()?;
        let fresh = retained.is_none();
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
        // This pass has reads only. Later effectful tools must journal admission
        // before their effects rather than running an effect in this closure.
        if fresh {
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
            "SELECT value FROM kv WHERE substr(key,1,14)='control/audit/' ORDER BY rowid LIMIT ?1",
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messaging::wire::IssuerKind;
    use crate::registry::{Class, Role};
    use crate::store::Store;

    #[test]
    fn control_result_and_audit_commit_atomically_and_reopen_without_redispatch() {
        let temp = tempfile::tempdir().unwrap();
        let store = SqliteStore::open(temp.path()).unwrap();
        let actor = Actor {
            seat_id: Ulid::new(),
            process_generation: 1,
            swarm_id: Ulid::new(),
            role: Role {
                name: "executor".into(),
                class: Class::Worker,
            },
            grant_epoch: 1,
            issuer_kind: IssuerKind::LocalDev,
        };
        let call = Call {
            op_id: Ulid::new(),
            tool: "self_status".into(),
            args: json!({}),
        };
        store.connection().unwrap().execute_batch("CREATE TRIGGER fail_audit BEFORE INSERT ON kv WHEN substr(NEW.key,1,14)='control/audit/' BEGIN SELECT RAISE(ABORT,'fixture audit failure'); END;").unwrap();
        let journal = Journal::new(store.clone());
        assert!(
            journal
                .call(&actor, &call, || super::super::success(
                    call.op_id,
                    json!({"fixture":true})
                ))
                .is_err()
        );
        assert!(
            store
                .get(&format!("control/op/{}", call.op_id))
                .unwrap()
                .is_none()
        );
        assert!(journal.pending(128).unwrap().is_empty());
        store
            .connection()
            .unwrap()
            .execute_batch("DROP TRIGGER fail_audit")
            .unwrap();
        let result = journal
            .call(&actor, &call, || {
                super::super::success(call.op_id, json!({"fixture":true}))
            })
            .unwrap();
        assert_eq!(journal.pending(128).unwrap().len(), 1);
        drop(journal);
        let reopened = Journal::new(SqliteStore::open(temp.path()).unwrap());
        assert_eq!(
            reopened
                .call(&actor, &call, || panic!("cached read dispatched again"))
                .unwrap(),
            result
        );
        let events = reopened.pending(128).unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[1].payload["outcome"], "replay");
        reopened.published(events[0].id).unwrap();
        assert_eq!(reopened.pending(128).unwrap().len(), 1);
    }

    #[test]
    fn audit_outbox_preserves_call_order_instead_of_random_ulid_order() {
        let temp = tempfile::tempdir().unwrap();
        let store = SqliteStore::open(temp.path()).unwrap();
        let mut first = Draft::new("seat/fixture/control", "mcp", json!({"call":1}));
        first.id = Ulid::from(2_u128);
        let mut second = Draft::new("seat/fixture/control", "mcp", json!({"call":2}));
        second.id = Ulid::from(1_u128);
        for event in [&first, &second] {
            store
                .put(
                    &format!("control/audit/{}", event.id),
                    &serde_json::to_vec(event).unwrap(),
                )
                .unwrap();
        }
        let pending = Journal::new(store).pending(128).unwrap();
        assert_eq!(
            pending.iter().map(|event| event.id).collect::<Vec<_>>(),
            vec![first.id, second.id]
        );
    }

    #[test]
    fn concurrent_operation_retries_return_one_committed_result() {
        use std::sync::{
            Arc, Barrier,
            atomic::{AtomicUsize, Ordering},
        };
        let temp = tempfile::tempdir().unwrap();
        let journal = Journal::new(SqliteStore::open(temp.path()).unwrap());
        let actor = Actor {
            seat_id: Ulid::new(),
            process_generation: 1,
            swarm_id: Ulid::new(),
            role: Role {
                name: "executor".into(),
                class: Class::Worker,
            },
            grant_epoch: 1,
            issuer_kind: IssuerKind::LocalDev,
        };
        let call = Call {
            op_id: Ulid::new(),
            tool: "self_status".into(),
            args: json!({}),
        };
        let barrier = Arc::new(Barrier::new(2));
        let dispatched = Arc::new(AtomicUsize::new(0));
        let threads: Vec<_> = (0..2)
            .map(|_| {
                let (journal, actor, call, barrier, dispatched) = (
                    journal.clone(),
                    actor.clone(),
                    call.clone(),
                    barrier.clone(),
                    dispatched.clone(),
                );
                std::thread::spawn(move || {
                    barrier.wait();
                    journal
                        .call(&actor, &call, || {
                            let ordinal = dispatched.fetch_add(1, Ordering::SeqCst);
                            std::thread::sleep(std::time::Duration::from_millis(30));
                            super::super::success(call.op_id, json!({"ordinal":ordinal}))
                        })
                        .unwrap()
                })
            })
            .collect();
        let values: Vec<_> = threads
            .into_iter()
            .map(|thread| thread.join().unwrap())
            .collect();
        assert_eq!(values[0], values[1]);
        assert_eq!(dispatched.load(Ordering::SeqCst), 1);
        assert_eq!(journal.pending(128).unwrap().len(), 2);
    }
}
