#![allow(
    clippy::disallowed_methods,
    reason = "disposable fixture IO, never the operator's custody store"
)]

use super::*;
use crate::identity::{
    Custody, DeviceKeyStore, IdentityRuntime, LocalIssuerOutcome, PendingReason, UnlockPhase,
    UnlockProgress, UnlockProgressRow, UnlockStep,
};
use crate::store::{Events, events::EventLog};
use bbt_proto::identity::{CustodyAcl, IdentityRequest, UnlockOutcome};
use std::sync::{Mutex, atomic::AtomicUsize};

#[derive(Default)]
struct Keys {
    keys: Mutex<BTreeMap<Ulid, SigningKey>>,
    held: AtomicBool,
    attempts: AtomicUsize,
}
impl DeviceKeyStore for Keys {
    fn custody(&self) -> Custody {
        Custody::LoginKeychain
    }
    fn load(&self, id: Ulid) -> crate::identity::Result<Option<SigningKey>> {
        if self.held.load(Ordering::Acquire) {
            return Err(PendingReason::KeychainInteractionRequired);
        }
        Ok(self.keys.lock().unwrap().get(&id).cloned())
    }
    fn create(&self, id: Ulid, key: &SigningKey) -> crate::identity::Result<()> {
        self.keys.lock().unwrap().insert(id, key.clone());
        Ok(())
    }
    fn native_acl(&self, _: Ulid) -> crate::identity::Result<CustodyAcl> {
        Ok(CustodyAcl::TeamSigned)
    }
    fn unlock(
        &self,
        id: Ulid,
        expected: [u8; 32],
        progress: UnlockProgress,
    ) -> crate::identity::Result<UnlockOutcome> {
        assert_eq!(
            self.keys.lock().unwrap()[&id].verifying_key().to_bytes(),
            expected
        );
        self.attempts.fetch_add(1, Ordering::AcqRel);
        progress(UnlockProgressRow {
            step: UnlockStep::ReadCurrent,
            phase: UnlockPhase::Started,
            status: None,
        })?;
        self.held.store(false, Ordering::Release);
        progress(UnlockProgressRow {
            step: UnlockStep::ReadCurrent,
            phase: UnlockPhase::Applied,
            status: None,
        })?;
        Ok(UnlockOutcome::Allowed)
    }
}

#[test]
fn allowed_native_unlock_reports_reopen_busy_then_refresh_recovers_without_another_attempt() {
    let root = tempfile::tempdir().unwrap();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    let keys = Arc::new(Keys::default());
    let LocalIssuerOutcome::Ready(issuer) =
        crate::identity::local_dev_issuer_with_store(root.path(), keys.as_ref())
    else {
        panic!("disposable local issuer unavailable");
    };
    let expected_signer = issuer.verifying_key().to_bytes();
    drop(issuer);
    // Opening the log initializes its schema. The observer must not repeat
    // those writes while the actor publishes the native completion.
    let events = EventLog::open(&root.path().join("account")).unwrap();
    keys.held.store(true, Ordering::Release);
    let identity = IdentityRuntime::open_custodied_with_store(
        root.path(),
        keys.clone(),
        runtime.handle().clone(),
    )
    .unwrap();
    identity.fail_next_native_reopen();
    let supervisor = crate::Supervisor::start(crate::Config {
        journal_directory: Some(root.path().join("supervisor-journal")),
        ..Default::default()
    })
    .unwrap();
    let repository = root.path().join("repository");
    std::fs::create_dir_all(repository.join(".bboxy")).unwrap();
    let logic = Logic::start_inner(
        vec![Box::new(LocalProvider::new(supervisor.connect().unwrap()))],
        LogicConfig {
            run_dir: root.path().into(),
            repository,
            ring: Default::default(),
        },
        SigningKey::from_bytes(&[61; 32]),
        IdentityMode::Prepared(Box::new(identity)),
        runtime.handle(),
    )
    .unwrap();
    let origin = || crate::mcp::Origin {
        door: crate::mcp::Door::Ctl,
        issued_by: None,
    };
    let op = Ulid::new();
    let result = runtime.block_on(async {
        logic
            .client
            .gui_identity_control(op, IdentityRequest::Unlock, origin())
            .await
            .unwrap();
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                let response = logic
                    .client
                    .gui_identity_control(op, IdentityRequest::Status, origin())
                    .await
                    .unwrap();
                if response.outcome == Some(UnlockOutcome::Allowed) {
                    break response;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("native fixture outcome missing")
    });
    assert_eq!(result.outcome, Some(UnlockOutcome::Allowed));
    assert_eq!(
        result.reopen_failed.as_ref().unwrap().reason,
        "SQLite DatabaseBusy (5)"
    );
    assert!(result.hold.as_deref().unwrap().contains("reopen_failed"));
    assert_eq!(keys.attempts.load(Ordering::Acquire), 1);
    let status = runtime
        .block_on(
            logic
                .client
                .gui_identity_control(Ulid::new(), IdentityRequest::Status, origin()),
        )
        .unwrap();
    assert_eq!(status.reopen_failed, result.reopen_failed);
    let held = runtime.block_on(logic.client.identity_status()).unwrap();
    assert_eq!(held.pending_reason, Some(PendingReason::ReopenFailed));
    assert_eq!(held.reopen_failed, result.reopen_failed);
    assert!(held.device_id.is_none());
    let refused = runtime
        .block_on(
            logic
                .client
                .gui_identity_control(Ulid::new(), IdentityRequest::Unlock, origin()),
        )
        .unwrap_err();
    assert!(refused.to_string().starts_with("reopen_failed:"));
    // Exercise the same refresh as maintenance without waiting for its 30s clock.
    runtime.block_on(async {
        let (tx, rx) = oneshot::channel();
        logic
            .client
            .tx
            .send(Command::RefreshIdentity(tx))
            .await
            .unwrap();
        rx.await.unwrap().unwrap();
    });
    let refreshed = runtime.block_on(logic.client.identity_status()).unwrap();
    assert!(refreshed.reopen_failed.is_none());
    assert!(refreshed.pending_reason.is_none());
    assert!(refreshed.device_id.is_some());
    let current = runtime
        .block_on(
            logic
                .client
                .gui_identity_control(Ulid::new(), IdentityRequest::Status, origin()),
        )
        .unwrap();
    assert!(current.hold.is_none());
    assert!(current.reopen_failed.is_none());

    // Recovery still publishes one legitimate completion for the immutable act.
    let until = Instant::now() + Duration::from_secs(3);
    loop {
        let rows = events.after(0, 256).unwrap();
        let completions: Vec<_> = rows
            .iter()
            .filter(|row| {
                row.event.payload["tool"] == "identity_unlock"
                    && row.event.payload["op_id"] == serde_json::json!(op)
                    && row.event.payload["signature"] == "device"
            })
            .collect();
        if let [completion] = completions.as_slice() {
            completion.verify().unwrap();
            assert_eq!(completion.provenance.signer, expected_signer);
            break;
        }
        assert!(completions.is_empty(), "duplicate native completion");
        if Instant::now() >= until {
            let status = runtime.block_on(logic.client.identity_status());
            let retained = runtime.block_on(logic.client.gui_identity_control(
                op,
                IdentityRequest::Status,
                origin(),
            ));
            panic!(
                "native completion missing: status={status:?}, retained={retained:?}, rows={}, last={:?}, attempts={}",
                rows.len(),
                rows.last().map(|row| row.seq),
                keys.attempts.load(Ordering::Acquire)
            );
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(keys.attempts.load(Ordering::Acquire), 1);
    logic.shutdown().unwrap();
    supervisor.shutdown().unwrap();
}
