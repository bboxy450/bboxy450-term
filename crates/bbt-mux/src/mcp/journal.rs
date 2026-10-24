//! Durable control results and audit attempts, including admission before effects.

use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use ulid::Ulid;

use super::{Call, MAX_RESULT, Principal, error};
use crate::store::{Result, digest, events::Draft, sqlite::SqliteStore};

#[derive(Clone)]
pub(crate) struct Journal {
    store: SqliteStore,
}

#[derive(Serialize, Deserialize)]
struct Entry {
    actor: Principal,
    tool: String,
    args_digest: String,
    result: Value,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Read,
    Rejected,
    Effect,
}

impl Journal {
    pub fn new(store: SqliteStore) -> Self {
        Self { store }
    }

    pub fn call(
        &self,
        actor: impl Into<Principal>,
        call: &Call,
        dispatch: impl FnOnce() -> Value,
    ) -> Result<Value> {
        self.record(&actor.into(), call, dispatch, Mode::Read, |_| true)
            .map(|(value, _)| value)
    }

    /// Reauthorize reads before consulting an old result. A current denial is
    /// audited and returned without revealing or replacing an old success.
    pub fn checked(&self, actor: impl Into<Principal>, call: &Call, value: Value) -> Result<Value> {
        let actor = actor.into();
        if value["isError"] == true {
            self.record(&actor, call, || value, Mode::Rejected, |_| true)
                .map(|(value, _)| value)
        } else {
            self.call(&actor, call, || value)
        }
    }

    /// A retained page can contain items removed from the caller's current
    /// authority. Deny that replay without destroying the original evidence.
    pub fn checked_authorized(
        &self,
        actor: impl Into<Principal>,
        call: &Call,
        value: Value,
        retained_allowed: impl FnOnce(&Value) -> bool,
    ) -> Result<Value> {
        let actor = actor.into();
        if value["isError"] == true {
            return self.checked(&actor, call, value);
        }
        self.record(&actor, call, || value, Mode::Read, retained_allowed)
            .map(|(value, _)| value)
    }

    /// Commit an intent and audit before dispatch, with no SQLite transaction
    /// held across W2.4. Missing outcomes remain unknown and never redispatch.
    pub fn effect(
        &self,
        actor: impl Into<Principal>,
        call: &Call,
        dispatch: impl FnOnce() -> Value,
    ) -> Result<Value> {
        let actor = actor.into();
        let (retained, fresh) = self.admit(&actor, call)?;
        if !fresh {
            return Ok(retained);
        }
        self.complete(&actor, call, dispatch())
    }

    /// Commit one durable attempt before asynchronous I/O; retained attempts
    /// never authorize another native submission, including unknown outcomes.
    pub fn admit(&self, actor: impl Into<Principal>, call: &Call) -> Result<(Value, bool)> {
        self.record(&actor.into(),call,||error(
            call.op_id,"unknown","durable attempt has no recorded outcome; retain the operation ID and check its receipts"
        ),Mode::Effect,|_|true)
    }

    /// Publish the result and outcome audit together after the admitted I/O.
    pub fn complete(
        &self,
        actor: impl Into<Principal>,
        call: &Call,
        value: Value,
    ) -> Result<Value> {
        let actor = actor.into();
        let result = if serde_json::to_vec(&value)?.len() > MAX_RESULT {
            error(
                call.op_id,
                "unknown",
                "effect outcome exceeds the response bound; check the original operation's receipts",
            )
        } else {
            value
        };
        let mut db = self.store.connection()?;
        let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let admitted: Vec<u8> = tx.query_row(
            "SELECT value FROM kv WHERE key=?1",
            [format!("control/op/{}", call.op_id)],
            |row| row.get(0),
        )?;
        let admitted: Entry = serde_json::from_slice(&admitted)?;
        if admitted.actor != actor
            || admitted.tool != call.tool
            || admitted.args_digest != digest(&serde_json::to_vec(&call.args)?)
        {
            return Err(crate::store::invalid(
                "control completion admission mismatch",
            ));
        }
        let entry = Entry {
            actor: actor.clone(),
            tool: call.tool.clone(),
            args_digest: digest(&serde_json::to_vec(&call.args)?),
            result: result.clone(),
        };
        if tx.execute(
            "UPDATE kv SET value=?2 WHERE key=?1",
            params![
                format!("control/op/{}", call.op_id),
                serde_json::to_vec(&entry)?
            ],
        )? != 1
        {
            return Err(crate::store::invalid(
                "control admission disappeared; effect outcome is unknown",
            ));
        }
        let event = audit(&actor, call, &entry.args_digest, &result, false, false);
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

    fn record(
        &self,
        actor: &Principal,
        call: &Call,
        dispatch: impl FnOnce() -> Value,
        mode: Mode,
        retained_allowed: impl FnOnce(&Value) -> bool,
    ) -> Result<(Value, bool)> {
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
            if !old.actor.same_authority(actor) || old.tool != call.tool || old.args_digest != hash
            {
                (
                    error(
                        call.op_id,
                        "conflict",
                        "operation ID belongs to different arguments or authority",
                    ),
                    false,
                )
            } else if mode == Mode::Rejected {
                (dispatch(), false)
            } else if !retained_allowed(&old.result) {
                (
                    error(
                        call.op_id,
                        "rejected",
                        "retained read contains items outside current task authority; use a new operation ID",
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
        let event = audit(
            actor,
            call,
            &hash,
            &result,
            replay,
            fresh && mode == Mode::Effect,
        );
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
        Ok((result, fresh))
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

fn audit(
    actor: &Principal,
    call: &Call,
    hash: &str,
    result: &Value,
    replay: bool,
    admitted: bool,
) -> Draft {
    let (subject, kind) = match actor {
        Principal::Seat(actor) => (format!("seat/{}/control", actor.seat_id), "mcp"),
        Principal::Operator { door, .. } => (
            "account/control".into(),
            match door {
                super::Door::Gui => "gui",
                super::Door::Ctl => "ctl",
            },
        ),
    };
    let mut event = Draft::new(
        subject,
        kind,
        json!({"op_id":call.op_id,"tool":call.tool,"actor":actor,
            "outcome":if admitted {"admitted"} else if replay {"replay"} else if result["structuredContent"]["status"]=="unknown" {"unknown"} else if result["isError"]==true {"rejected"} else {"applied"},
            "args_digest":hash,"replay":replay}),
    );
    if let Principal::Seat(actor) = actor {
        event.seat = Some(actor.seat_id);
        event.generation = Some(actor.process_generation);
    }
    event
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::SeatActor as Actor;
    use crate::messaging::wire::IssuerKind;
    use crate::registry::{Class, Role};
    use crate::store::Store;

    fn effect_fixture() -> (tempfile::TempDir, SqliteStore, Actor, Call) {
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
            tool: "ch_subscribe".into(),
            args: json!({"folder":"channels/fixture","name":"own"}),
        };
        (temp, store, actor, call)
    }

    #[test]
    fn legacy_seat_actor_entries_replay_without_migration_or_dispatch() {
        let (_temp, store, actor, call) = effect_fixture();
        let result = super::super::success(call.op_id, json!({"status":"applied"}));
        let old = json!({
            "actor": actor,
            "tool": call.tool,
            "args_digest": digest(&serde_json::to_vec(&call.args).unwrap()),
            "result": result,
        });
        let key = format!("control/op/{}", call.op_id);
        store.put(&key, &serde_json::to_vec(&old).unwrap()).unwrap();
        let journal = Journal::new(store.clone());
        assert_eq!(
            journal
                .effect(&actor, &call, || panic!("legacy effect repeated"))
                .unwrap(),
            result
        );
        let stored: Value = serde_json::from_slice(&store.get(&key).unwrap().unwrap()).unwrap();
        assert_eq!(stored, old);
        let audits = journal.pending(128).unwrap();
        assert_eq!(audits[0].subject, format!("seat/{}/control", actor.seat_id));
        assert_eq!(audits[0].seat, Some(actor.seat_id));
    }

    #[test]
    fn operator_reconnect_replays_the_account_result_and_rejects_another_device() {
        let (temp, _, _, mut call) = effect_fixture();
        let directory = crate::store::object_dir(temp.path(), "account").unwrap();
        let store = SqliteStore::open(&directory).unwrap();
        let operator = Principal::Operator {
            device_id: Ulid::new(),
            door: super::super::Door::Gui,
        };
        call.tool = "gui_spawn".into();
        call.args = json!({"swarm":Ulid::new(),"harness":"zsh"});
        let seat = Ulid::new();
        let expected = super::super::success(call.op_id, json!({"seat":seat}));
        let journal = Journal::new(store.clone());
        assert_eq!(
            journal
                .effect(&operator, &call, || expected.clone())
                .unwrap(),
            expected
        );
        drop(journal);
        let journal = Journal::new(SqliteStore::open(&directory).unwrap());
        assert_eq!(
            journal
                .effect(&operator, &call, || panic!("operator launch repeated"))
                .unwrap(),
            expected
        );
        let other = Principal::Operator {
            device_id: Ulid::new(),
            door: super::super::Door::Gui,
        };
        let denied = journal
            .effect(&other, &call, || panic!("different device dispatched"))
            .unwrap();
        assert_eq!(denied["structuredContent"]["error"]["code"], "conflict");
        let audits = journal.pending(128).unwrap();
        assert_eq!(audits.len(), 4); // admission, result, replay, authority conflict
        assert!(audits.iter().all(|event| event.subject == "account/control"
            && event.seat.is_none()
            && event.generation.is_none()));
        assert!(audits[2].payload["replay"].as_bool().unwrap());
        assert_eq!(
            audits[0].payload["actor"],
            serde_json::to_value(&operator).unwrap()
        );
    }

    #[test]
    fn admitted_operator_without_a_result_stays_unknown_after_reopen() {
        let (temp, store, _, mut call) = effect_fixture();
        let operator = Principal::Operator {
            device_id: Ulid::new(),
            door: super::super::Door::Gui,
        };
        call.tool = "gui_spawn".into();
        let journal = Journal::new(store);
        let (unknown, fresh) = journal.admit(&operator, &call).unwrap();
        assert!(fresh);
        assert_eq!(unknown["structuredContent"]["status"], "unknown");
        drop(journal);
        let journal = Journal::new(SqliteStore::open(temp.path()).unwrap());
        assert_eq!(
            journal
                .effect(&operator, &call, || panic!("unknown launch repeated"))
                .unwrap(),
            unknown
        );
    }

    #[test]
    fn mutation_requires_audit_admission_and_releases_the_transaction_before_dispatch() {
        let (_temp, store, actor, call) = effect_fixture();
        let journal = Journal::new(store.clone());
        store.connection().unwrap().execute_batch("CREATE TRIGGER fail_effect_audit BEFORE INSERT ON kv WHEN substr(NEW.key,1,14)='control/audit/' BEGIN SELECT RAISE(ABORT,'fixture audit failure'); END;").unwrap();
        let count = std::cell::Cell::new(0);
        assert!(
            journal
                .effect(&actor, &call, || {
                    count.set(count.get() + 1);
                    super::super::success(call.op_id, json!({"status":"applied"}))
                })
                .is_err()
        );
        assert_eq!(count.get(), 0);
        assert!(
            store
                .get(&format!("control/op/{}", call.op_id))
                .unwrap()
                .is_none()
        );
        store
            .connection()
            .unwrap()
            .execute_batch("DROP TRIGGER fail_effect_audit")
            .unwrap();
        let first = journal
            .effect(&actor, &call, || {
                // This uses the very same SQLite object as the control journal.
                // An admission transaction still held here would block this write.
                store.put("fixture/effect", b"applied").unwrap();
                count.set(count.get() + 1);
                super::super::success(call.op_id, json!({"status":"applied"}))
            })
            .unwrap();
        assert_eq!(store.get("fixture/effect").unwrap().unwrap(), b"applied");
        assert_eq!(
            journal
                .effect(&actor, &call, || panic!(
                    "completed effect dispatched twice"
                ))
                .unwrap(),
            first
        );
        assert_eq!(count.get(), 1);
        let mut changed = call.clone();
        changed.args["name"] = json!("different");
        assert_eq!(
            journal
                .effect(&actor, &changed, || panic!("conflict dispatched"))
                .unwrap()["structuredContent"]["error"]["code"],
            "conflict"
        );
        let events = journal.pending(128).unwrap();
        assert_eq!(
            events
                .iter()
                .map(|e| e.payload["outcome"].clone())
                .collect::<Vec<_>>(),
            vec![
                json!("admitted"),
                json!("applied"),
                json!("replay"),
                json!("rejected")
            ]
        );
    }

    #[test]
    fn crashed_admitted_mutation_reopens_unknown_without_redispatch() {
        let (temp, store, actor, call) = effect_fixture();
        let journal = Journal::new(store.clone());
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let _ = journal.effect(&actor, &call, || {
                    store.put("fixture/effect", b"before crash").unwrap();
                    panic!("fixture crash after effect, before outcome");
                });
            }))
            .is_err()
        );
        drop(journal);
        let reopened = Journal::new(SqliteStore::open(temp.path()).unwrap());
        let value = reopened
            .effect(&actor, &call, || {
                panic!("unknown mutation dispatched again")
            })
            .unwrap();
        assert_eq!(value["structuredContent"]["status"], "unknown");
        assert_eq!(value["structuredContent"]["error"]["code"], "unknown");
        assert_eq!(
            store.get("fixture/effect").unwrap().unwrap(),
            b"before crash"
        );
    }

    #[test]
    fn outcome_audit_failure_keeps_the_original_unknown_intent() {
        let (_temp, store, actor, call) = effect_fixture();
        let journal = Journal::new(store.clone());
        store.connection().unwrap().execute_batch("CREATE TRIGGER fail_outcome_audit BEFORE INSERT ON kv WHEN substr(NEW.key,1,14)='control/audit/' AND (SELECT COUNT(*) FROM kv WHERE substr(key,1,14)='control/audit/')>0 BEGIN SELECT RAISE(ABORT,'fixture outcome audit failure'); END;").unwrap();
        assert!(
            journal
                .effect(&actor, &call, || {
                    store.put("fixture/effect", b"committed").unwrap();
                    super::super::success(call.op_id, json!({"status":"applied"}))
                })
                .is_err()
        );
        assert_eq!(store.get("fixture/effect").unwrap().unwrap(), b"committed");
        store
            .connection()
            .unwrap()
            .execute_batch("DROP TRIGGER fail_outcome_audit")
            .unwrap();
        let unknown = journal
            .effect(&actor, &call, || {
                panic!("uncertain effect dispatched again")
            })
            .unwrap();
        assert_eq!(unknown["structuredContent"]["status"], "unknown");
        assert_eq!(journal.pending(128).unwrap().len(), 2);
    }

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
