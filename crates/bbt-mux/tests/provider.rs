#![allow(
    clippy::disallowed_methods,
    reason = "test/fixture IO, not the UI thread"
)]
use std::sync::{Arc, Mutex};
use std::time::Duration;

use bbt_mux::logic::SupervisorConnection;
use bbt_mux::provider::{
    Connections, Host, HostId, LinkState, LocalProvider, Provider, ProviderEvent, ProviderKind,
};
use bbt_mux::registry::Location;
use bbt_mux::{Action, Event, Reply, SeatInfo};

#[derive(Default)]
struct Calls {
    requests: Vec<HostId>,
    waits: Vec<(HostId, Duration)>,
}
struct ProbeLink {
    host: HostId,
    calls: Arc<Mutex<Calls>>,
}
impl SupervisorConnection for ProbeLink {
    fn request(&mut self, _: Action) -> bbt_mux::store::Result<Reply> {
        self.calls.lock().unwrap().requests.push(self.host.clone());
        Ok(Reply::Seats {
            seats: Vec::<SeatInfo>::new(),
        })
    }
    fn receive(&mut self, wait: Duration) -> bbt_mux::store::Result<Option<Event>> {
        self.calls
            .lock()
            .unwrap()
            .waits
            .push((self.host.clone(), wait));
        Ok(None)
    }
}
struct ProbeProvider {
    host: HostId,
    id: String,
    calls: Arc<Mutex<Calls>>,
}
impl Provider for ProbeProvider {
    fn id(&self) -> String {
        self.id.clone()
    }
    fn kind(&self) -> ProviderKind {
        ProviderKind::Ssh
    }
    fn hosts(&self) -> Vec<Host> {
        vec![Host {
            id: self.host.clone(),
            provider: self.id(),
            label: "probe".into(),
            region: None,
            state: LinkState::Connected { rtt_ms: 0 },
        }]
    }
    fn link(&mut self, host: HostId) -> bbt_mux::store::Result<Box<dyn SupervisorConnection>> {
        assert_eq!(host, self.host);
        Ok(Box::new(ProbeLink {
            host,
            calls: self.calls.clone(),
        }))
    }
    fn poll(&mut self) -> Vec<ProviderEvent> {
        vec![]
    }
}
#[test]
fn location_namespaces_select_one_link_and_only_the_local_link_waits() {
    let calls = Arc::new(Mutex::new(Calls::default()));
    let ssh = HostId::from(&Location::Ssh {
        host: "same-host".into(),
    });
    let cloud = HostId::from(&Location::Cloud {
        provider: "fixture-cloud".into(),
        host: "same-host".into(),
    });
    assert_ne!(ssh, cloud);
    let mut connections = Connections::open(vec![
        Box::new(LocalProvider::new(ProbeLink {
            host: HostId::Local,
            calls: calls.clone(),
        })),
        Box::new(ProbeProvider {
            host: ssh.clone(),
            id: "ssh".into(),
            calls: calls.clone(),
        }),
        Box::new(ProbeProvider {
            host: cloud.clone(),
            id: "cloud".into(),
            calls: calls.clone(),
        }),
    ])
    .unwrap();
    for host in [&HostId::Local, &ssh, &cloud] {
        connections.request(host, Action::ListOwnedSeats).unwrap();
    }
    assert_eq!(
        calls.lock().unwrap().requests,
        vec![HostId::Local, ssh.clone(), cloud.clone()]
    );
    assert!(connections.receive().unwrap().is_empty());
    let waits = &calls.lock().unwrap().waits;
    assert_eq!(waits[0], (HostId::Local, Duration::from_millis(10)));
    assert_eq!(waits.len(), 3);
    assert!(waits[1..].iter().all(|(_, wait)| *wait == Duration::ZERO));
}
#[test]
fn local_provider_preserves_the_existing_link_and_rejects_unknown_hosts() {
    let calls = Arc::new(Mutex::new(Calls::default()));
    let mut provider = LocalProvider::new(ProbeLink {
        host: HostId::Local,
        calls: calls.clone(),
    });
    assert_eq!(provider.kind(), ProviderKind::Local);
    assert_eq!(provider.hosts()[0].label, "this Mac");
    assert!(provider.poll().is_empty());
    assert!(
        provider
            .link(HostId::Ssh {
                host: "missing".into()
            })
            .is_err()
    );
    let mut link = provider.link(HostId::Local).unwrap();
    link.request(Action::ListOwnedSeats).unwrap();
    assert_eq!(calls.lock().unwrap().requests, vec![HostId::Local]);
    assert!(provider.link(HostId::Local).is_err());
}

type Trace = Arc<Mutex<Vec<(HostId, Option<bbt_mux::Ulid>, &'static str)>>>;
struct RecordingLink {
    host: HostId,
    inner: Box<dyn SupervisorConnection>,
    trace: Trace,
}
impl SupervisorConnection for RecordingLink {
    fn request(&mut self, action: Action) -> bbt_mux::store::Result<Reply> {
        if let Action::ReadRingBatch { reads, .. } = &action {
            self.trace.lock().unwrap().extend(
                reads
                    .iter()
                    .map(|read| (self.host.clone(), Some(read.attachment.seat), "history")),
            );
            return self.inner.request(action);
        }
        let (seat, kind) = match &action {
            Action::Spawn { spec } => (Some(spec.seat), "spawn"),
            Action::AttachRing { seat, .. } => (Some(*seat), "attach"),
            Action::RingRecovery { attachment } => (Some(attachment.seat), "recovery"),
            Action::ReadRing { attachment, .. } => (Some(attachment.seat), "history"),
            Action::PublishCheckpoint { attachment, .. } => (Some(attachment.seat), "checkpoint"),
            Action::Input { attachment, .. } => (Some(attachment.seat), "input"),
            Action::Resize { attachment, .. } => (Some(attachment.seat), "resize"),
            Action::InputContext { attachment } => (Some(attachment.seat), "input-context"),
            Action::GuardedInput { attachment, .. } => (Some(attachment.seat), "guarded-input"),
            Action::ScheduleClose { attachment, .. } => (Some(attachment.seat), "close"),
            Action::UndoClose { attachment, .. } => (Some(attachment.seat), "undo"),
            Action::ListOwnedSeats => (None, "owned"),
            _ => (None, "other"),
        };
        self.trace
            .lock()
            .unwrap()
            .push((self.host.clone(), seat, kind));
        self.inner.request(action)
    }
    fn receive(&mut self, wait: Duration) -> bbt_mux::store::Result<Option<Event>> {
        self.inner.receive(wait)
    }
}
struct LinkedProvider {
    host: Host,
    link: Option<Box<dyn SupervisorConnection>>,
}
impl Provider for LinkedProvider {
    fn id(&self) -> String {
        self.host.provider.clone()
    }
    fn kind(&self) -> ProviderKind {
        ProviderKind::Ssh
    }
    fn hosts(&self) -> Vec<Host> {
        vec![self.host.clone()]
    }
    fn link(&mut self, host: HostId) -> bbt_mux::store::Result<Box<dyn SupervisorConnection>> {
        assert_eq!(host, self.host.id);
        Ok(self.link.take().unwrap())
    }
    fn poll(&mut self) -> Vec<ProviderEvent> {
        vec![]
    }
}

#[test]
fn logic_routes_real_seats_recovery_history_ownership_and_close_by_location() {
    use bbt_mux::logic::{Logic, LogicConfig};
    use bbt_mux::registry::{Position, SeatRecord};
    use bbt_mux::{Config, LaunchSpec, Supervisor, Ulid};
    let temp = tempfile::tempdir().unwrap();
    let repository = temp.path().join(".bboxy");
    std::fs::create_dir(&repository).unwrap();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    let local = Supervisor::start(Config {
        journal_directory: Some(temp.path().join("local-journal")),
        ..Config::default()
    })
    .unwrap();
    let remote = Supervisor::start(Config {
        journal_directory: Some(temp.path().join("remote-journal")),
        ..Config::default()
    })
    .unwrap();
    let trace: Trace = Default::default();
    let host = HostId::Ssh {
        host: "build-box".into(),
    };
    let providers: Vec<Box<dyn Provider>> = vec![
        Box::new(LocalProvider::new(RecordingLink {
            host: HostId::Local,
            inner: Box::new(local.connect().unwrap()),
            trace: trace.clone(),
        })),
        Box::new(LinkedProvider {
            host: Host {
                id: host.clone(),
                provider: "test-ssh".into(),
                label: "build-box".into(),
                region: None,
                state: LinkState::Connected { rtt_ms: 0 },
            },
            link: Some(Box::new(RecordingLink {
                host: host.clone(),
                inner: Box::new(remote.connect().unwrap()),
                trace: trace.clone(),
            })),
        }),
    ];
    let logic = Logic::start_with_providers(
        providers,
        LogicConfig {
            run_dir: temp.path().join("run"),
            repository,
            ring: Default::default(),
        },
        ed25519_dalek::SigningKey::from_bytes(&[42; 32]),
        runtime.handle(),
    )
    .unwrap();
    let swarm = Ulid::new();
    let mut seats = Vec::new();
    for (name, location) in [
        ("LOCAL", Location::Local),
        (
            "REMOTE",
            Location::Ssh {
                host: "build-box".into(),
            },
        ),
    ] {
        let mut spec = LaunchSpec::new("/bin/zsh", temp.path());
        spec.args = vec![
            "-f".into(),
            "-c".into(),
            format!("printf '{name}\\n'; while read -r line; do printf 'R:%s\\n' \"$line\"; done"),
        ];
        let id = spec.seat;
        let mut record = SeatRecord::new(id, swarm, name, "zsh");
        record.location = location.clone();
        runtime.block_on(logic.client.spawn(record, spec)).unwrap();
        runtime
            .block_on(logic.client.gui_resize(id, 1, 94, 27))
            .unwrap();
        let receipt = runtime
            .block_on(
                logic
                    .client
                    .gui_input(id, 1, Ulid::new(), b"host-routed\n".to_vec()),
            )
            .unwrap();
        assert_eq!(receipt.state, bbt_mux::InputState::PtyWritten);
        let until = std::time::Instant::now() + Duration::from_secs(5);
        let snapshot = loop {
            let snapshot = runtime.block_on(logic.client.snapshot(id, 1)).unwrap();
            let text: String = snapshot
                .screen
                .cells
                .iter()
                .flatten()
                .map(|cell| cell.c)
                .collect();
            if text.contains(name) && text.contains("R:host-routed") {
                break snapshot;
            }
            assert!(
                std::time::Instant::now() < until,
                "output did not reach its host's viewport"
            );
            std::thread::sleep(Duration::from_millis(10));
        };
        assert!(
            runtime
                .block_on(logic.client.history(id, 1, 1, snapshot.sequence))
                .unwrap()
                .gap
                .is_none()
        );
        runtime
            .block_on(logic.client.control_catalogue(id, 1))
            .unwrap();
        assert_eq!(
            runtime.block_on(logic.client.record(id)).unwrap().location,
            location
        );
        seats.push(id);
    }
    let remote_id = seats[1];
    let position = Position {
        swarm,
        tab: Ulid::new(),
        split_path: vec![],
        ratios: vec![],
        layout_revision: 1,
    };
    runtime
        .block_on(
            logic
                .client
                .close(remote_id, 1, Ulid::new(), position, 5000),
        )
        .unwrap();
    let closed = runtime.block_on(logic.client.record(remote_id)).unwrap();
    runtime
        .block_on(
            logic
                .client
                .undo(remote_id, 1, closed.close.unwrap().revision, 1),
        )
        .unwrap();
    logic.shutdown().unwrap();
    let trace = trace.lock().unwrap();
    for (link, seat, _) in trace.iter() {
        if let Some(seat) = seat {
            assert_eq!(
                link,
                if *seat == remote_id {
                    &host
                } else {
                    &HostId::Local
                }
            );
        }
    }
    for required in [
        "spawn",
        "attach",
        "recovery",
        "history",
        "checkpoint",
        "input",
        "resize",
        "close",
        "undo",
    ] {
        assert!(
            trace
                .iter()
                .any(|(link, _, kind)| link == &host && *kind == required),
            "remote {required} did not use its link"
        );
    }
    assert!(
        trace
            .iter()
            .any(|(link, _, kind)| link == &host && *kind == "owned")
    );
    println!(
        "D1a: real local and SSH-labelled PTYs route spawn/input/resize/capture/history/ownership/close through distinct links and private host journals; records retain location"
    );
}
