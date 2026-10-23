#![allow(clippy::disallowed_methods)] // Owned fault-injection IO fixture, explicit signing key.

use std::future::Future;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc,
};
use std::task::{Context, Waker};
use std::time::Duration;

use bbt_mux::logic::{Logic, LogicConfig, SupervisorConnection};
use bbt_mux::{Action, Client, Config, Event, Reply, Supervisor};
use ed25519_dalek::SigningKey;

struct FaultLink {
    client: Client,
    armed: Arc<AtomicBool>,
    entered: mpsc::Sender<()>,
    release: mpsc::Receiver<()>,
    panic: bool,
}
impl SupervisorConnection for FaultLink {
    fn request(&mut self, action: Action) -> bbt_mux::store::Result<Reply> {
        SupervisorConnection::request(&mut self.client, action)
    }
    fn receive(&mut self, timeout: Duration) -> bbt_mux::store::Result<Option<Event>> {
        if self.armed.swap(false, Ordering::AcqRel) {
            self.entered.send(())?;
            self.release.recv_timeout(Duration::from_secs(10))?;
            assert!(!self.panic, "injected logic panic cause");
            return Err(std::io::Error::from_raw_os_error(libc::EINVAL).into());
        }
        SupervisorConnection::receive(&mut self.client, timeout)
    }
}
struct Release(mpsc::Sender<()>);
impl Drop for Release {
    fn drop(&mut self) {
        let _ = self.0.send(());
    }
}

#[test]
fn queued_and_later_callers_keep_the_worker_error_and_panic_cause() {
    for panic in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let repository = root.path().join("repository");
        std::fs::create_dir(&repository).unwrap();
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .unwrap();
        let supervisor = Supervisor::start(Config::default()).unwrap();
        let armed = Arc::new(AtomicBool::new(false));
        let (entered, receive_entered) = mpsc::channel();
        let (release, receive_release) = mpsc::channel();
        let release = Release(release);
        let logic = Logic::start(
            FaultLink {
                client: supervisor.connect().unwrap(),
                armed: armed.clone(),
                entered,
                release: receive_release,
                panic,
            },
            LogicConfig {
                run_dir: root.path().join("run"),
                repository,
                ring: Default::default(),
            },
            SigningKey::from_bytes(&[123; 32]),
            runtime.handle(),
        )
        .unwrap();
        armed.store(true, Ordering::Release);
        receive_entered
            .recv_timeout(Duration::from_secs(5))
            .unwrap();
        // The worker is paused in receive. Poll once to enqueue a real command
        // and wait for its oneshot before releasing the injected failure.
        let mut pending = Box::pin(logic.client.gui_seats());
        let mut context = Context::from_waker(Waker::noop());
        assert!(pending.as_mut().poll(&mut context).is_pending());
        release.0.send(()).unwrap();
        let queued = runtime.block_on(pending).unwrap_err().to_string();
        let later = runtime
            .block_on(logic.client.gui_seats())
            .unwrap_err()
            .to_string();
        assert_eq!(
            queued, later,
            "cause must precede channel closure for both callers"
        );
        assert!(queued.contains("logic stopped"));
        if panic {
            assert!(queued.contains("injected logic panic cause"), "{queued}");
            assert!(
                logic
                    .shutdown()
                    .unwrap_err()
                    .to_string()
                    .contains("logic panicked")
            );
        } else {
            let cause = std::io::Error::from_raw_os_error(libc::EINVAL).to_string();
            assert!(queued.contains(&cause), "lost underlying cause: {queued}");
            let original = logic.shutdown().unwrap_err();
            assert_eq!(
                original
                    .downcast_ref::<std::io::Error>()
                    .unwrap()
                    .raw_os_error(),
                Some(libc::EINVAL)
            );
        }
        supervisor.shutdown().unwrap();
    }
}
