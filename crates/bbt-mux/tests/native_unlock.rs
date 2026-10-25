#![allow(
    clippy::disallowed_methods,
    reason = "throwaway account and injected custody; no native credential service"
)]
mod common;

use bbt_mux::{
    LaunchSpec, Supervisor, Ulid,
    identity::{
        Custody, DeviceKeyStore, LocalIssuerOutcome, PendingReason, UnlockPhase, UnlockProgress,
        UnlockProgressRow, UnlockStep,
    },
    logic::{Logic, LogicConfig},
    mcp::{Door, Origin},
    store::{Events, Store, events::EventLog, sqlite::SqliteStore},
};
use bbt_proto::identity::{CustodyAcl, IdentityControl, IdentityRequest, UnlockOutcome};
use ed25519_dalek::SigningKey;
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};

struct NativeFixture {
    root: PathBuf,
    keys: Mutex<BTreeMap<Ulid, SigningKey>>,
    held: AtomicBool,
    attempts: AtomicUsize,
    loads: AtomicUsize,
    audit_seen: AtomicBool,
    blocked: Mutex<bool>,
    entered: Condvar,
    released: Condvar,
    outcome: Mutex<UnlockOutcome>,
}
impl NativeFixture {
    fn release(&self) {
        *self.blocked.lock().unwrap() = false;
        self.released.notify_all();
    }
    fn wait_entered(&self) {
        let until = Instant::now() + Duration::from_secs(3);
        let mut locked = self.blocked.lock().unwrap();
        while self.attempts.load(Ordering::Acquire) == 0 {
            assert!(Instant::now() < until, "native attempt never entered");
            locked = self
                .entered
                .wait_timeout(locked, Duration::from_millis(20))
                .unwrap()
                .0;
        }
    }
}
impl DeviceKeyStore for NativeFixture {
    fn custody(&self) -> Custody {
        Custody::LoginKeychain
    }
    fn load(&self, id: Ulid) -> bbt_mux::identity::Result<Option<SigningKey>> {
        self.loads.fetch_add(1, Ordering::Relaxed);
        if self.held.load(Ordering::Acquire) {
            return Err(PendingReason::KeychainInteractionRequired);
        }
        Ok(self.keys.lock().unwrap().get(&id).cloned())
    }
    fn create(&self, id: Ulid, key: &SigningKey) -> bbt_mux::identity::Result<()> {
        self.keys.lock().unwrap().insert(id, key.clone());
        Ok(())
    }
    fn native_acl(&self, _: Ulid) -> bbt_mux::identity::Result<CustodyAcl> {
        Ok(CustodyAcl::TeamSigned)
    }
    fn unlock(
        &self,
        id: Ulid,
        expected: [u8; 32],
        progress: UnlockProgress,
    ) -> bbt_mux::identity::Result<UnlockOutcome> {
        let key = self
            .keys
            .lock()
            .unwrap()
            .get(&id)
            .cloned()
            .ok_or(PendingReason::LocalIssuerMissing)?;
        assert_eq!(key.verifying_key().to_bytes(), expected);
        let rows = EventLog::open(&self.root.join("account"))
            .unwrap()
            .after(0, 128)
            .unwrap();
        let pending = rows
            .iter()
            .find(|row| {
                row.event.payload["tool"] == "identity_unlock"
                    && row.event.payload["signature"] == "pending"
            })
            .expect("audit must be published before any native access");
        pending.verify().unwrap();
        assert_ne!(
            pending.provenance.signer, expected,
            "hold signer must be labelled honestly"
        );
        let op: Ulid = serde_json::from_value(pending.event.payload["op_id"].clone()).unwrap();
        let entry: serde_json::Value = serde_json::from_slice(
            &SqliteStore::open(&self.root.join("account"))
                .unwrap()
                .get(&format!("control/op/{op}"))
                .unwrap()
                .unwrap(),
        )
        .unwrap();
        assert_eq!(
            entry["native_unlock"]["position"]["digest"],
            pending.hash().unwrap()
        );
        self.audit_seen.store(true, Ordering::Release);
        progress(UnlockProgressRow {
            step: UnlockStep::ReadLegacy,
            phase: UnlockPhase::Started,
            status: None,
        })?;
        self.attempts.fetch_add(1, Ordering::Release);
        self.entered.notify_all();
        let guard = self.blocked.lock().unwrap();
        let _guard = self.released.wait_while(guard, |blocked| *blocked).unwrap();
        let outcome = *self.outcome.lock().unwrap();
        if outcome == UnlockOutcome::Allowed {
            self.held.store(false, Ordering::Release);
        }
        progress(UnlockProgressRow {
            step: UnlockStep::ReadLegacy,
            phase: UnlockPhase::Applied,
            status: None,
        })?;
        Ok(outcome)
    }
}
struct Fixture {
    root: tempfile::TempDir,
    runtime: tokio::runtime::Runtime,
    store: Arc<NativeFixture>,
    logic: Option<Logic>,
    supervisor: Option<Supervisor>,
    config: LogicConfig,
}
impl Fixture {
    fn new(held: bool, blocked: bool, outcome: UnlockOutcome) -> Self {
        let root = tempfile::tempdir().unwrap();
        let store = Arc::new(NativeFixture {
            root: root.path().into(),
            keys: Default::default(),
            held: AtomicBool::new(false),
            attempts: AtomicUsize::new(0),
            loads: AtomicUsize::new(0),
            audit_seen: AtomicBool::new(false),
            blocked: Mutex::new(blocked),
            entered: Condvar::new(),
            released: Condvar::new(),
            outcome: Mutex::new(outcome),
        });
        assert!(matches!(
            bbt_mux::identity::local_dev_issuer_with_store(root.path(), store.as_ref()),
            LocalIssuerOutcome::Ready(_)
        ));
        store.held.store(held, Ordering::Release);
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .unwrap();
        let supervisor = Supervisor::start(common::capture_config(root.path())).unwrap();
        let repository = root.path().join("repository");
        std::fs::create_dir_all(repository.join(".bboxy")).unwrap();
        let config = LogicConfig {
            run_dir: root.path().into(),
            repository,
            ring: Default::default(),
        };
        let logic = Logic::start_custodied_with_store(
            supervisor.connect().unwrap(),
            config.clone(),
            store.clone(),
            runtime.handle(),
        )
        .unwrap();
        Self {
            root,
            runtime,
            store,
            logic: Some(logic),
            supervisor: Some(supervisor),
            config,
        }
    }
    fn control(
        &self,
        op: Ulid,
        request: IdentityRequest,
        door: Door,
    ) -> bbt_mux::store::Result<IdentityControl> {
        self.runtime
            .block_on(self.logic.as_ref().unwrap().client.gui_identity_control(
                op,
                request,
                Origin {
                    door,
                    issued_by: None,
                },
            ))
    }
    fn final_result(&self, op: Ulid) -> IdentityControl {
        let until = Instant::now() + Duration::from_secs(5);
        loop {
            let response = self
                .control(op, IdentityRequest::Unlock, Door::Gui)
                .unwrap();
            if response.outcome != Some(UnlockOutcome::Unknown)
                || response
                    .pending
                    .iter()
                    .all(|row| row.op != op || !row.in_flight)
            {
                return response;
            }
            assert!(Instant::now() < until, "native completion did not publish");
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    fn restart(&mut self) {
        self.logic.take().unwrap().shutdown().unwrap();
        self.logic = Some(
            Logic::start_custodied_with_store(
                self.supervisor.as_ref().unwrap().connect().unwrap(),
                self.config.clone(),
                self.store.clone(),
                self.runtime.handle(),
            )
            .unwrap(),
        );
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.store.release();
        if let Some(logic) = self.logic.take() {
            logic.shutdown().unwrap();
        }
        if let Some(supervisor) = self.supervisor.take() {
            supervisor.shutdown().unwrap();
        }
    }
}

#[test]
fn native_hold_unlock_is_audited_before_access_and_does_not_block_panes() {
    let f = Fixture::new(true, true, UnlockOutcome::Allowed);
    let public = f
        .runtime
        .block_on(f.logic.as_ref().unwrap().client.identity_status())
        .unwrap();
    assert_eq!(public.device_id, None);
    let before = f.store.loads.load(Ordering::Relaxed);
    let status = f
        .control(Ulid::new(), IdentityRequest::Status, Door::Ctl)
        .unwrap();
    assert!(status.hold.is_some());
    assert_eq!(status.clear_actions.len(), 2);
    assert_eq!(
        f.store.loads.load(Ordering::Relaxed),
        before,
        "status must not query native secrets"
    );
    let mut spec = LaunchSpec::new("/bin/sh", f.root.path());
    spec.args = vec![
        "-c".into(),
        "printf 'NATIVE-READY\\n'; read -r line; printf 'ECHO-%s\\n' \"$line\"; read -r done"
            .into(),
    ];
    let seat = bbt_mux::registry::SeatRecord::new(spec.seat, Ulid::new(), "native fixture", "zsh");
    f.runtime
        .block_on(f.logic.as_ref().unwrap().client.spawn(seat.clone(), spec))
        .unwrap();
    let op = Ulid::new();
    let initial = f.control(op, IdentityRequest::Unlock, Door::Ctl).unwrap();
    assert_eq!(initial.outcome, Some(UnlockOutcome::Unknown));
    f.store.wait_entered();
    let started = Instant::now();
    f.runtime.block_on(async {
        tokio::time::timeout(Duration::from_millis(500), async {
            f.logic
                .as_ref()
                .unwrap()
                .client
                .gui_input(seat.id, 1, Ulid::new(), b"ALIVE\n".to_vec())
                .await
                .unwrap();
            f.logic
                .as_ref()
                .unwrap()
                .client
                .snapshot(seat.id, 1)
                .await
                .unwrap();
            f.logic.as_ref().unwrap().client.gui_seats().await.unwrap();
        })
        .await
        .expect("native dialog must not stall terminal command service");
    });
    assert!(started.elapsed() < Duration::from_millis(500));
    assert!(
        f.control(Ulid::new(), IdentityRequest::Unlock, Door::Gui)
            .unwrap_err()
            .to_string()
            .contains("Busy")
    );
    let spawn = bbt_proto::SpawnSpec {
        swarm: bbt_proto::SwarmId::from_u128(Ulid::new().0),
        harness_kind: "zsh".into(),
        name: None,
        role: None,
        cwd: f.root.path().to_string_lossy().into(),
        location: bbt_proto::Where::Local,
        resume: None,
        placement: None,
    };
    let refused = f
        .runtime
        .block_on(
            f.logic
                .as_ref()
                .unwrap()
                .client
                .gui_spawn(Ulid::new(), spawn, Door::Ctl),
        )
        .unwrap_err()
        .to_string();
    assert!(
        refused.contains("held")
            && refused.contains("ctl identity unlock")
            && refused.contains("Settings"),
        "{refused}"
    );
    assert_eq!(f.store.attempts.load(Ordering::Acquire), 1);
    assert!(f.store.audit_seen.load(Ordering::Acquire));
    f.store.release();
    let result = f.final_result(op);
    assert_eq!(result.outcome, Some(UnlockOutcome::Allowed));
    let until = Instant::now() + Duration::from_secs(3);
    let rows = loop {
        let rows = EventLog::open(&f.root.path().join("account"))
            .unwrap()
            .after(0, 128)
            .unwrap();
        if rows.iter().any(|row| {
            row.event.payload["op_id"] == serde_json::json!(op)
                && row.event.payload["signature"] == "device"
        }) {
            break rows;
        }
        assert!(Instant::now() < until);
        std::thread::sleep(Duration::from_millis(10));
    };
    let pending = rows
        .iter()
        .find(|row| {
            row.event.payload["op_id"] == serde_json::json!(op)
                && row.event.payload["signature"] == "pending"
        })
        .unwrap();
    let done: Vec<_> = rows
        .iter()
        .filter(|row| {
            row.event.payload["op_id"] == serde_json::json!(op)
                && row.event.payload["signature"] == "device"
        })
        .collect();
    assert_eq!(done.len(), 1);
    assert_eq!(
        done[0].event.payload["pending"]["digest"],
        pending.hash().unwrap()
    );
    assert!(done[0].seq > pending.seq);
    done[0].verify().unwrap();
    assert_eq!(
        f.control(op, IdentityRequest::Unlock, Door::Gui)
            .unwrap()
            .outcome,
        Some(UnlockOutcome::Allowed)
    );
    assert_eq!(f.store.attempts.load(Ordering::Acquire), 1);
}

#[test]
fn native_audit_failure_executes_nothing_and_no_hold_is_a_native_noop() {
    let f = Fixture::new(true, false, UnlockOutcome::Allowed);
    let store = SqliteStore::open(&f.root.path().join("account")).unwrap();
    store.connection().unwrap().execute_batch("CREATE TRIGGER unlock_audit_failure BEFORE INSERT ON kv WHEN substr(NEW.key,1,14)='control/audit/' BEGIN SELECT RAISE(ABORT,'fixture audit failure'); END;").unwrap();
    assert!(
        f.control(Ulid::new(), IdentityRequest::Unlock, Door::Ctl)
            .is_err()
    );
    assert_eq!(f.store.attempts.load(Ordering::Acquire), 0);
    drop(f);
    let f = Fixture::new(false, false, UnlockOutcome::Allowed);
    let before = f.store.loads.load(Ordering::Relaxed);
    let response = f
        .control(Ulid::new(), IdentityRequest::Unlock, Door::Gui)
        .unwrap();
    assert!(response.hold.is_none());
    assert_eq!(f.store.attempts.load(Ordering::Acquire), 0);
    assert_eq!(f.store.loads.load(Ordering::Relaxed), before);
}

#[test]
fn native_cancelled_and_unknown_outcomes_survive_logic_restart_without_replay() {
    for outcome in [UnlockOutcome::Cancelled, UnlockOutcome::Unknown] {
        let mut f = Fixture::new(true, false, outcome);
        let op = Ulid::new();
        f.control(op, IdentityRequest::Unlock, Door::Gui).unwrap();
        f.store.wait_entered();
        let result = f.final_result(op);
        assert_eq!(result.outcome, Some(outcome));
        f.restart();
        assert_eq!(
            f.control(op, IdentityRequest::Unlock, Door::Ctl)
                .unwrap()
                .outcome,
            Some(outcome)
        );
        assert_eq!(f.store.attempts.load(Ordering::Acquire), 1);
        let status = f
            .control(Ulid::new(), IdentityRequest::Status, Door::Gui)
            .unwrap();
        assert!(
            status
                .pending
                .iter()
                .any(|row| row.op == op && row.outcome == outcome && row.signature == "pending")
        );
    }
}

#[test]
fn native_held_gui_requires_the_current_capability_and_refuses_a_supplied_device() {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    let f = Fixture::new(true, false, UnlockOutcome::Cancelled);
    let endpoint = f
        .runtime
        .block_on(f.logic.as_ref().unwrap().client.gui_endpoint())
        .unwrap();
    let request = |capability, op, action| {
        serde_json::to_value(bbt_proto::GuiRequest {
            version: bbt_proto::gui::VERSION,
            capability,
            action: bbt_proto::GuiAction::Identity(bbt_proto::identity::IdentityCall {
                id: op,
                request: action,
            }),
        })
        .unwrap()
    };
    let ask = |value: serde_json::Value| {
        f.runtime.block_on(async {
            tokio::time::timeout(Duration::from_secs(3), async {
                let mut socket = tokio::net::UnixStream::connect(&endpoint.path)
                    .await
                    .unwrap();
                let mut bytes = serde_json::to_vec(&value).unwrap();
                bytes.push(b'\n');
                socket.write_all(&bytes).await.unwrap();
                let mut reader = BufReader::new(socket);
                let mut line = String::new();
                while reader.read_line(&mut line).await.unwrap() != 0 {
                    if let Ok(bbt_proto::GuiReply::IdentityControl { response, .. }) =
                        serde_json::from_str(&line)
                    {
                        return Some(response);
                    }
                    line.clear();
                }
                None
            })
            .await
            .expect("identity door must answer or close within its deadline")
        })
    };
    let mut wrong = endpoint.capability;
    wrong[0] ^= 1;
    assert!(ask(request(wrong, Ulid::new(), IdentityRequest::Unlock)).is_none());
    let mut forged = request(endpoint.capability, Ulid::new(), IdentityRequest::Unlock);
    forged["action"]["body"]["device_id"] = serde_json::json!(Ulid::new());
    assert!(ask(forged).is_none());
    assert_eq!(f.store.attempts.load(Ordering::Acquire), 0);
    let status = ask(request(
        endpoint.capability,
        Ulid::new(),
        IdentityRequest::Status,
    ))
    .unwrap();
    assert!(status.hold.is_some());
    let op = Ulid::new();
    assert_eq!(
        ask(request(endpoint.capability, op, IdentityRequest::Unlock))
            .unwrap()
            .outcome,
        Some(UnlockOutcome::Unknown)
    );
    f.store.wait_entered();
    assert_eq!(f.final_result(op).outcome, Some(UnlockOutcome::Cancelled));
    assert_eq!(f.store.attempts.load(Ordering::Acquire), 1);
}

#[test]
fn native_hold_refuses_an_unvalidated_persisted_device_before_admission() {
    let f = Fixture::new(true, false, UnlockOutcome::Allowed);
    let store = SqliteStore::open(&f.root.path().join("account")).unwrap();
    let original = store.get("identity/local-issuer").unwrap().unwrap();
    let mut changed: serde_json::Value = serde_json::from_slice(&original).unwrap();
    changed["device_id"] = serde_json::json!(Ulid::new());
    store
        .put(
            "identity/local-issuer",
            &serde_json::to_vec(&changed).unwrap(),
        )
        .unwrap();
    let op = Ulid::new();
    let result = f.control(op, IdentityRequest::Unlock, Door::Ctl);
    store.put("identity/local-issuer", &original).unwrap();
    assert!(result.is_err());
    assert_eq!(f.store.attempts.load(Ordering::Acquire), 0);
    assert!(
        store.get(&format!("control/op/{op}")).unwrap().is_none(),
        "unvalidated identity cannot admit an act"
    );
}
