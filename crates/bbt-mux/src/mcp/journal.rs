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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    native_unlock: Option<NativeAttempt>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct PendingPosition {
    pub stream: String,
    pub sequence: String,
    pub digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct NativeAttempt {
    pub reference: crate::identity::unlock::Reference,
    pub hold: String,
    pub pending_event: Ulid,
    pub position: Option<PendingPosition>,
    pub outcome: bbt_proto::identity::UnlockOutcome,
    pub finished: bool,
    #[serde(default)]
    pub started: bool,
    pub steps: Vec<crate::identity::unlock::ProgressRow>,
    pub completion: Option<Draft>,
}

pub(crate) struct NativePending {
    pub op: Ulid,
    pub actor: Principal,
    pub attempt: NativeAttempt,
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
    /// Probe a retained operator operation before applying an in-flight admission cap.
    pub(crate) fn retained(&self, actor: &Principal, call: &Call) -> Result<Option<Value>> {
        let bytes: Option<Vec<u8>> = self
            .store
            .connection()?
            .query_row(
                "SELECT value FROM kv WHERE key=?1",
                [format!("control/op/{}", call.op_id)],
                |row| row.get(0),
            )
            .optional()?;
        let Some(bytes) = bytes else {
            return Ok(None);
        };
        let old: Entry = serde_json::from_slice(&bytes)?;
        Ok(Some(
            if old.actor.same_authority(actor)
                && old.tool == call.tool
                && old.args_digest == digest(&serde_json::to_vec(&call.args)?)
            {
                old.result
            } else {
                error(
                    call.op_id,
                    "op_id_reused",
                    "operation ID belongs to different arguments or authority",
                )
            },
        ))
    }

    pub fn call(
        &self,
        actor: impl Into<Principal>,
        call: &Call,
        dispatch: impl FnOnce() -> Value,
    ) -> Result<Value> {
        self.record(&actor.into(), call, dispatch, Mode::Read, |_| true, None)
            .map(|(value, _)| value)
    }

    /// Read a native result by its operation ID, without creating or admitting an effect.
    pub(crate) fn native_status(
        &self,
        actor: &Principal,
        call: &Call,
        current: Value,
    ) -> Result<Value> {
        if !matches!(actor, Principal::Operator { .. }) || call.tool != "identity_status" {
            return Err(crate::store::invalid(
                "native status requires the admitted operator",
            ));
        }
        let mut db = self.store.connection()?;
        let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let old: Option<Vec<u8>> = tx
            .query_row(
                "SELECT value FROM kv WHERE key=?1",
                [format!("control/op/{}", call.op_id)],
                |row| row.get(0),
            )
            .optional()?;
        let old = old
            .map(|bytes| serde_json::from_slice::<Entry>(&bytes))
            .transpose()?;
        let (result, replay) = match old {
            Some(entry) if entry.tool == "identity_unlock" => {
                if entry.actor.same_authority(actor) {
                    (entry.result, true)
                } else {
                    (
                        error(
                            call.op_id,
                            "forbidden",
                            "native result belongs to another operator identity",
                        ),
                        false,
                    )
                }
            }
            _ => (current, false),
        };
        let event = audit(
            actor,
            call,
            &digest(&serde_json::to_vec(&call.args)?),
            &result,
            replay,
            false,
        );
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

    /// Reauthorize reads before consulting an old result. A current denial is
    /// audited and returned without revealing or replacing an old success.
    pub fn checked(&self, actor: impl Into<Principal>, call: &Call, value: Value) -> Result<Value> {
        let actor = actor.into();
        if value["isError"] == true {
            self.record(&actor, call, || value, Mode::Rejected, |_| true, None)
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
        self.record(&actor, call, || value, Mode::Read, retained_allowed, None)
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
        ),Mode::Effect,|_|true,None)
    }

    /// The existing operator journal admits the native act and pending audit atomically.
    pub(crate) fn admit_native_unlock(
        &self,
        actor: &Principal,
        call: &Call,
        hold: String,
        reference: crate::identity::unlock::Reference,
        initial: Value,
    ) -> Result<(Value, bool)> {
        if !matches!(actor, Principal::Operator { device_id, .. } if *device_id == reference.device)
            || call.tool != "identity_unlock"
        {
            return Err(crate::store::invalid(
                "native unlock requires the persisted operator identity",
            ));
        }
        self.record(
            actor,
            call,
            || initial,
            Mode::Effect,
            |_| true,
            Some((hold, reference)),
        )
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
            native_unlock: admitted.native_unlock,
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

    #[allow(
        clippy::too_many_arguments,
        reason = "one optional native admission shares the existing journal transaction"
    )]
    fn record(
        &self,
        actor: &Principal,
        call: &Call,
        dispatch: impl FnOnce() -> Value,
        mode: Mode,
        retained_allowed: impl FnOnce(&Value) -> bool,
        native: Option<(String, crate::identity::unlock::Reference)>,
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
                        "op_id_reused",
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
        let mut event = audit(
            actor,
            call,
            &hash,
            &result,
            replay,
            fresh && mode == Mode::Effect,
        );
        let native_unlock = if fresh {
            native.map(|(hold, reference)| {
                event.payload["signature"] = json!("pending");
                event.payload["hold_reason"] = json!(hold);
                event.payload["outcome"] = json!("unknown");
                NativeAttempt {
                    reference,
                    hold,
                    pending_event: event.id,
                    position: None,
                    outcome: bbt_proto::identity::UnlockOutcome::Unknown,
                    finished: false,
                    started: false,
                    steps: Vec::new(),
                    completion: None,
                }
            })
        } else {
            None
        };
        if fresh {
            if native_unlock.is_some() {
                let pending: i64 = tx.query_row("SELECT count(*) FROM (SELECT 1 FROM kv WHERE key >= 'control/unlock/' AND key < 'control/unlock0' LIMIT 128)", [], |row| row.get(0))?;
                if pending >= 128 {
                    return Err(crate::store::invalid(
                        "Busy: 128 native unlock completions remain pending",
                    ));
                }
                // This is a bounded outbox index, not another operation ledger.
                tx.execute(
                    "INSERT INTO kv VALUES(?1,?2)",
                    params![format!("control/unlock/{}", call.op_id), b"1".as_slice()],
                )?;
            }
            tx.execute(
                "INSERT INTO kv VALUES(?1,?2)",
                params![
                    key,
                    serde_json::to_vec(&Entry {
                        actor: actor.clone(),
                        tool: call.tool.clone(),
                        args_digest: hash,
                        result: result.clone(),
                        native_unlock,
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

    /// Remember the actual immutable chain position before starting native I/O.
    pub(crate) fn bind_native_pending(
        &self,
        op: Ulid,
        envelope: &crate::store::events::Envelope,
    ) -> Result<()> {
        envelope.verify()?;
        self.update_native(op, None, |entry| {
            let native = entry
                .native_unlock
                .as_mut()
                .ok_or_else(|| crate::store::invalid("missing native admission"))?;
            if native.pending_event != envelope.event.id
                || envelope.event.subject != "account/control"
                || envelope.event.payload["op_id"] != json!(op)
                || envelope.event.payload["signature"] != "pending"
            {
                return Err(crate::store::invalid("native pending publication mismatch"));
            }
            let position = PendingPosition {
                stream: "account".into(),
                sequence: envelope.seq.to_string(),
                digest: envelope.hash()?,
            };
            if native.position.as_ref().is_some_and(|old| {
                old.digest != position.digest || old.sequence != position.sequence
            }) {
                return Err(crate::store::invalid("native pending position changed"));
            }
            native.position = Some(position);
            Ok(())
        })
    }

    /// Read only the bounded completion outbox; never scan the operation history.
    pub(crate) fn native_pending(&self, limit: usize) -> Result<Vec<NativePending>> {
        let db = self.store.connection()?;
        let mut query = db.prepare("SELECT key FROM kv WHERE key >= 'control/unlock/' AND key < 'control/unlock0' ORDER BY rowid LIMIT ?1")?;
        let keys = query
            .query_map([limit.min(128) as i64], |row| row.get::<_, String>(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        keys.into_iter()
            .map(|key| {
                let op: Ulid = key.trim_start_matches("control/unlock/").parse()?;
                let bytes: Vec<u8> = db.query_row(
                    "SELECT value FROM kv WHERE key=?1",
                    [format!("control/op/{op}")],
                    |row| row.get(0),
                )?;
                let entry: Entry = serde_json::from_slice(&bytes)?;
                let attempt = entry.native_unlock.ok_or_else(|| {
                    crate::store::invalid("native outbox index has no admitted operation")
                })?;
                Ok(NativePending {
                    op,
                    actor: entry.actor,
                    attempt,
                })
            })
            .collect()
    }

    pub(crate) fn native_progress(
        &self,
        op: Ulid,
        actor: &Principal,
        row: crate::identity::unlock::ProgressRow,
    ) -> Result<()> {
        self.update_native(op, Some(actor), |entry| {
            let native = entry
                .native_unlock
                .as_mut()
                .ok_or_else(|| crate::store::invalid("missing native admission"))?;
            if native.finished {
                return Err(crate::store::invalid(
                    "native outcome already retained; stop late effects",
                ));
            }
            if !native.started {
                return Err(crate::store::invalid("native attempt was not submitted"));
            }
            if native.steps.len() >= 32 {
                return Err(crate::store::invalid("native migration progress bound"));
            }
            native.steps.push(row);
            Ok(())
        })
    }
    pub(crate) fn start_native_unlock(&self, op: Ulid, actor: &Principal) -> Result<()> {
        self.update_native(op, Some(actor), |entry| {
            let native = entry
                .native_unlock
                .as_mut()
                .ok_or_else(|| crate::store::invalid("missing native admission"))?;
            if native.started || native.finished || native.position.is_none() {
                return Err(crate::store::invalid(
                    "native attempt already submitted or audit not published",
                ));
            }
            native.started = true;
            Ok(())
        })
    }
    /// Retained polling sees the published admission, without admitting another act.
    pub(crate) fn native_snapshot(&self, op: Ulid, actor: &Principal, result: Value) -> Result<()> {
        self.update_native(op, Some(actor), |entry| {
            if entry
                .native_unlock
                .as_ref()
                .is_some_and(|native| !native.finished)
            {
                entry.result = result;
            }
            Ok(())
        })
    }

    /// Retain only the actual result; it is not yet a device-signed completion.
    pub(crate) fn finish_native_unlock(
        &self,
        op: Ulid,
        actor: &Principal,
        outcome: bbt_proto::identity::UnlockOutcome,
        result: Value,
    ) -> Result<()> {
        self.update_native(op, Some(actor), |entry| {
            let native = entry
                .native_unlock
                .as_mut()
                .ok_or_else(|| crate::store::invalid("missing native admission"))?;
            if native.finished {
                return Ok(());
            }
            native.finished = true;
            native.outcome = outcome;
            entry.result = result;
            Ok(())
        })
    }

    /// Allocate one immutable completion draft, only for the originally expected key.
    pub(crate) fn native_completion(&self, op: Ulid, signer: [u8; 32]) -> Result<Option<Draft>> {
        let mut prepared = None;
        self.update_native(op, None, |entry| {
            let native = entry.native_unlock.as_mut().ok_or_else(|| crate::store::invalid("missing native admission"))?;
            if !native.finished || native.reference.signer != signer { return Ok(()); }
            let Some(position) = &native.position else { return Ok(()); };
            if native.completion.is_none() {
                native.completion = Some(Draft::new("account/control", "identity-unlock/device", json!({
                    "op_id":op,"tool":"identity_unlock","actor":entry.actor,
                    "signature":"device","pending":position,"outcome":native.outcome,"steps":native.steps,
                })));
            }
            prepared = native.completion.clone();
            Ok(())
        })?;
        Ok(prepared)
    }

    /// Publication is idempotent; mark the retained reply, never the signed pending row.
    pub(crate) fn native_completion_published(&self, op: Ulid, event: Ulid) -> Result<()> {
        let mut db = self.store.connection()?;
        let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let bytes: Vec<u8> = tx.query_row(
            "SELECT value FROM kv WHERE key=?1",
            [format!("control/op/{op}")],
            |row| row.get(0),
        )?;
        let mut entry: Entry = serde_json::from_slice(&bytes)?;
        if entry
            .native_unlock
            .as_ref()
            .and_then(|native| native.completion.as_ref())
            .is_none_or(|draft| draft.id != event)
        {
            return Err(crate::store::invalid(
                "native completion publication mismatch",
            ));
        }
        if let Some(rows) = entry.result["structuredContent"]["pending"].as_array_mut() {
            for row in rows {
                if row["op"] == json!(op) {
                    row["signature"] = json!("device");
                    row["in_flight"] = json!(false);
                }
            }
            entry.result = super::success(op, entry.result["structuredContent"].clone());
            tx.execute(
                "UPDATE kv SET value=?2 WHERE key=?1",
                params![format!("control/op/{op}"), serde_json::to_vec(&entry)?],
            )?;
        }
        tx.execute(
            "DELETE FROM kv WHERE key=?1",
            [format!("control/unlock/{op}")],
        )?;
        tx.commit()?;
        Ok(())
    }

    fn update_native(
        &self,
        op: Ulid,
        actor: Option<&Principal>,
        update: impl FnOnce(&mut Entry) -> Result<()>,
    ) -> Result<()> {
        let mut db = self.store.connection()?;
        let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let key = format!("control/op/{op}");
        let bytes: Vec<u8> = tx.query_row("SELECT value FROM kv WHERE key=?1", [&key], |row| {
            row.get(0)
        })?;
        let mut entry: Entry = serde_json::from_slice(&bytes)?;
        if actor.is_some_and(|actor| actor != &entry.actor) {
            return Err(crate::store::invalid("native completion authority changed"));
        }
        update(&mut entry)?;
        tx.execute(
            "UPDATE kv SET value=?2 WHERE key=?1",
            params![key, serde_json::to_vec(&entry)?],
        )?;
        tx.commit()?;
        Ok(())
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
        Principal::OperatorLocal { door, .. } => (
            "account/control".into(),
            match door { super::Door::Gui => "gui", super::Door::Ctl => "ctl" },
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

    fn native_fixture() -> (
        tempfile::TempDir,
        SqliteStore,
        Principal,
        Call,
        crate::identity::unlock::Reference,
        ed25519_dalek::SigningKey,
    ) {
        let temp = tempfile::tempdir().unwrap();
        let store = SqliteStore::open(temp.path()).unwrap();
        let device = Ulid::new();
        let actor = Principal::Operator {
            device_id: device,
            door: super::super::Door::Gui,
            issued_by: None,
        };
        let call = Call {
            op_id: Ulid::new(),
            tool: "identity_unlock".into(),
            args: json!({"request":"unlock"}),
        };
        let key = ed25519_dalek::SigningKey::from_bytes(&[91; 32]);
        let reference = serde_json::from_value(json!({"device":device,"key":Ulid::new(),"expected":ed25519_dalek::SigningKey::from_bytes(&[90;32]).verifying_key().to_bytes(),"signer":key.verifying_key().to_bytes(),"custody":"login-keychain","kind":"Local"})).unwrap();
        (temp, store, actor, call, reference, key)
    }

    // keeps: spec mcp-control-plane
    #[test]
    fn native_admission_failure_rolls_back_every_intent_and_authority_is_not_supplied() {
        let (_temp, store, actor, call, reference, _) = native_fixture();
        let journal = Journal::new(store.clone());
        let other = Principal::Operator {
            device_id: Ulid::new(),
            door: super::super::Door::Gui,
            issued_by: None,
        };
        assert!(
            journal
                .admit_native_unlock(&other, &call, "held".into(), reference.clone(), json!({}))
                .is_err()
        );
        store.connection().unwrap().execute_batch("CREATE TRIGGER native_audit_failure BEFORE INSERT ON kv WHEN substr(NEW.key,1,14)='control/audit/' BEGIN SELECT RAISE(ABORT,'fixture audit failure'); END;").unwrap();
        assert!(
            journal
                .admit_native_unlock(&actor, &call, "held".into(), reference, json!({}))
                .is_err()
        );
        assert!(journal.native_pending(128).unwrap().is_empty());
        assert!(journal.retained(&actor, &call).unwrap().is_none());
        assert!(journal.pending(128).unwrap().is_empty());
    }

    // keeps: spec mcp-control-plane
    #[test]
    fn native_status_of_a_missing_id_never_admits_or_poisons_an_unlock() {
        let (_temp, store, actor, call, reference, _) = native_fixture();
        let journal = Journal::new(store.clone());
        let status = Call {
            op_id: call.op_id,
            tool: "identity_status".into(),
            args: json!({"request":"status"}),
        };
        let current = super::super::success(call.op_id, json!({"hold":"fixture","outcome":null}));
        assert_eq!(
            journal
                .native_status(&actor, &status, current.clone())
                .unwrap(),
            current
        );
        assert!(
            store
                .get(&format!("control/op/{}", call.op_id))
                .unwrap()
                .is_none()
        );
        let admitted = super::super::success(call.op_id, json!({"outcome":"unknown"}));
        assert!(
            journal
                .admit_native_unlock(&actor, &call, "held".into(), reference, admitted.clone())
                .unwrap()
                .1
        );
        assert_eq!(
            journal
                .native_status(&actor, &status, current.clone())
                .unwrap(),
            admitted
        );
        let other = Principal::Operator {
            device_id: Ulid::new(),
            door: super::super::Door::Gui,
            issued_by: None,
        };
        let refused = journal.native_status(&other, &status, current).unwrap();
        assert_eq!(refused["structuredContent"]["error"]["code"], "forbidden");
    }

    // keeps: spec mcp-control-plane
    #[test]
    fn native_unknown_survives_restart_and_gets_one_immutable_device_completion() {
        use crate::store::{Events, events::EventLog};
        use bbt_proto::identity::UnlockOutcome;
        let (temp, store, actor, call, reference, key) = native_fixture();
        let journal = Journal::new(store.clone());
        let unknown = super::super::success(call.op_id, json!({"outcome":"unknown"}));
        assert!(
            journal
                .admit_native_unlock(
                    &actor,
                    &call,
                    "custody interaction required".into(),
                    reference,
                    unknown.clone()
                )
                .unwrap()
                .1
        );
        assert!(
            journal.start_native_unlock(call.op_id, &actor).is_err(),
            "no native attempt before publication"
        );
        let draft = journal
            .pending(128)
            .unwrap()
            .into_iter()
            .find(|draft| draft.payload["signature"] == "pending")
            .unwrap();
        assert_eq!(draft.payload["outcome"], "unknown");
        let log = EventLog::open(&temp.path().join("events")).unwrap();
        let hold_key = ed25519_dalek::SigningKey::from_bytes(&[92; 32]);
        let pending = log.append(draft, &hold_key).unwrap();
        let original = serde_json::to_vec(&pending).unwrap();
        journal.bind_native_pending(call.op_id, &pending).unwrap();
        journal.start_native_unlock(call.op_id, &actor).unwrap();
        let reopened = Journal::new(SqliteStore::open(temp.path()).unwrap());
        assert!(
            !reopened
                .admit_native_unlock(
                    &actor,
                    &call,
                    "held".into(),
                    reopened.native_pending(1).unwrap()[0]
                        .attempt
                        .reference
                        .clone(),
                    unknown.clone()
                )
                .unwrap()
                .1
        );
        assert!(
            reopened.start_native_unlock(call.op_id, &actor).is_err(),
            "unknown must never authorize a second attempt"
        );
        assert_eq!(
            reopened
                .effect(&actor, &call, || panic!("unknown redispatched"))
                .unwrap(),
            unknown
        );
        reopened
            .finish_native_unlock(call.op_id, &actor, UnlockOutcome::Unknown, unknown.clone())
            .unwrap();
        assert!(
            reopened
                .native_completion(call.op_id, hold_key.verifying_key().to_bytes())
                .unwrap()
                .is_none(),
            "hold key is never a device signer"
        );
        let completion = reopened
            .native_completion(call.op_id, key.verifying_key().to_bytes())
            .unwrap()
            .unwrap();
        assert_eq!(
            completion,
            reopened
                .native_completion(call.op_id, key.verifying_key().to_bytes())
                .unwrap()
                .unwrap()
        );
        assert_eq!(
            completion.payload["pending"]["digest"],
            pending.hash().unwrap()
        );
        assert_eq!(
            completion.payload["pending"]["sequence"],
            pending.seq.to_string()
        );
        assert_eq!(completion.payload["op_id"], json!(call.op_id));
        assert_eq!(completion.payload["outcome"], "unknown");
        let signed = log.append(completion.clone(), &key).unwrap();
        assert_eq!(
            log.append(completion.clone(), &key).unwrap(),
            signed,
            "publication is deduplicated by the retained draft ID"
        );
        reopened
            .native_completion_published(call.op_id, completion.id)
            .unwrap();
        assert!(reopened.native_pending(128).unwrap().is_empty());
        assert_eq!(
            serde_json::to_vec(&log.at(pending.seq).unwrap().unwrap()).unwrap(),
            original,
            "signed pending bytes never change"
        );
        assert_eq!(log.verify_chain().unwrap(), 2);
        assert_eq!(reopened.retained(&actor, &call).unwrap(), Some(unknown));
    }

    // keeps: spec mcp-control-plane
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

    // keeps: spec mcp-control-plane
    #[test]
    fn local_operator_replays_across_doors_without_claiming_native_authority() {
        let (_temp, store, _, mut call) = effect_fixture();
        let reference = bbt_proto::messaging::OperatorReference {
            id: Ulid::new(),
            kind: bbt_proto::messaging::OperatorReferenceKind::OperatorLocal,
        };
        let local = Principal::OperatorLocal {
            reference,
            door: super::super::Door::Gui,
            issued_by: None,
        };
        let encoded = serde_json::to_value(&local).unwrap();
        assert!(encoded.get("device_id").is_none());
        assert!(encoded["reference"].get("device").is_none());
        assert_eq!(serde_json::from_value::<Principal>(encoded).unwrap(), local);
        call.tool = "gui_messaging".into();
        let journal = Journal::new(store.clone());
        let result = super::super::success(call.op_id, json!({"signature":"pending"}));
        assert_eq!(
            journal.effect(&local, &call, || result.clone()).unwrap(),
            result
        );
        let ctl = Principal::OperatorLocal {
            reference,
            door: super::super::Door::Ctl,
            issued_by: Some("fixture".into()),
        };
        assert_eq!(
            Journal::new(store)
                .effect(&ctl, &call, || panic!("local intent repeated"))
                .unwrap(),
            result
        );
        let audits = journal.pending(128).unwrap();
        assert!(audits.iter().all(|row| row.subject == "account/control"));
        let (_native_temp, _, _, native_call, native_ref, _) = native_fixture();
        assert!(
            journal
                .admit_native_unlock(&local, &native_call, "held".into(), native_ref, json!({}))
                .is_err()
        );
        let status = Call {
            tool: "identity_status".into(),
            ..native_call
        };
        assert!(journal.native_status(&local, &status, json!({})).is_err());
        assert!(journal.native_pending(128).unwrap().is_empty());
    }
    // keeps: spec mcp-control-plane
    #[test]
    fn operator_reconnect_replays_the_account_result_and_rejects_another_device() {
        let (temp, _, _, mut call) = effect_fixture();
        let directory = crate::store::object_dir(temp.path(), "account").unwrap();
        let store = SqliteStore::open(&directory).unwrap();
        let operator = Principal::Operator {
            device_id: Ulid::new(),
            door: super::super::Door::Gui,
            issued_by: None,
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
            issued_by: None,
        };
        let denied = journal
            .effect(&other, &call, || panic!("different device dispatched"))
            .unwrap();
        assert_eq!(denied["structuredContent"]["error"]["code"], "op_id_reused");
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

    // keeps: spec mcp-control-plane
    #[test]
    fn admitted_operator_without_a_result_stays_unknown_after_reopen() {
        let (temp, store, _, mut call) = effect_fixture();
        let operator = Principal::Operator {
            device_id: Ulid::new(),
            door: super::super::Door::Gui,
            issued_by: None,
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

    // keeps: spec mcp-control-plane
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
            "op_id_reused"
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

    // keeps: spec mcp-control-plane
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

    // keeps: spec mcp-control-plane
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

    // keeps: spec mcp-control-plane
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

    // keeps: spec mcp-control-plane
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

    // keeps: spec mcp-control-plane
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
