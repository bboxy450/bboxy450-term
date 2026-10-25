#![allow(
    clippy::disallowed_methods,
    reason = "test/fixture IO, not the UI thread"
)]
use std::collections::{BTreeMap, VecDeque};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU64, Ordering},
};
use std::time::{Duration, Instant};

use bbt_mux::logic::{Logic, LogicConfig, SupervisorConnection};
use bbt_mux::provider::fake::{
    BUNDLED_FIXTURE, Clock, FakeProvider, Fixture, STATE_SUBJECT, is_link_down,
};
use bbt_mux::provider::{HostId, LinkState, LocalProvider, Provider, ProviderEvent};
use bbt_mux::{Action, Attachment, Config, Event, LaunchSpec, Reply, Supervisor, Ulid};

#[derive(Default)]
struct ManualClock(AtomicU64);
impl Clock for ManualClock {
    fn now_ms(&self) -> u64 {
        self.0.load(Ordering::SeqCst)
    }
}
impl ManualClock {
    fn set(&self, ms: u64) {
        assert!(ms >= self.now_ms());
        self.0.store(ms, Ordering::SeqCst);
    }
    fn advance(&self, ms: u64) {
        self.0.fetch_add(ms, Ordering::SeqCst);
    }
}
#[derive(Default)]
struct Spy {
    actions: Vec<Action>,
    events: VecDeque<Event>,
}
struct SpyLink(Arc<Mutex<Spy>>);
impl SupervisorConnection for SpyLink {
    fn request(&mut self, action: Action) -> bbt_mux::store::Result<Reply> {
        self.0.lock().unwrap().actions.push(action);
        Ok(Reply::CheckpointCommitted { sequence: 999 })
    }
    fn receive(&mut self, wait: Duration) -> bbt_mux::store::Result<Option<Event>> {
        assert_eq!(wait, Duration::ZERO);
        Ok(self.0.lock().unwrap().events.pop_front())
    }
}
fn ssh() -> HostId {
    HostId::Ssh {
        host: "build-box".into(),
    }
}

// keeps: spec d1-providers
#[test]
fn a_ring_batch_checks_every_logical_host_before_forwarding_any_member() {
    use bbt_supervisor::protocol::{MAX_RING_BATCH_BYTES, RingRead};
    let local = Ulid::new();
    let remote = Ulid::new();
    let unknown = Ulid::new();
    let spy = Arc::new(Mutex::new(Spy::default()));
    let links = bbt_mux::provider::shared::LocalLinks::new(
        SpyLink(spy.clone()),
        [(local, HostId::Local), (remote, ssh())],
    );
    let read = |seat| RingRead {
        attachment: Attachment {
            seat,
            generation: 1,
            writer_lease: Ulid::new(),
        },
        from: 1,
        to: 2,
    };
    let batch = |reads| Action::ReadRingBatch {
        reads,
        max_bytes: MAX_RING_BATCH_BYTES as u32,
    };
    let mut link = links.link(HostId::Local);
    let error = link
        .request(batch(vec![read(local), read(remote)]))
        .unwrap_err();
    assert_eq!(
        error.to_string(),
        "ring batch crosses supervisor host boundary"
    );
    assert!(
        spy.lock().unwrap().actions.is_empty(),
        "valid prefix was forwarded"
    );
    let mut remote_link = links.link(ssh());
    assert!(
        remote_link
            .request(batch(vec![read(remote), read(unknown)]))
            .is_err()
    );
    assert!(
        spy.lock().unwrap().actions.is_empty(),
        "unknown remote member was forwarded"
    );
    remote_link.request(batch(vec![read(remote)])).unwrap();
    assert_eq!(spy.lock().unwrap().actions.len(), 1);
}

// keeps: spec d1-providers
#[test]
fn fixture_is_the_host_source_and_invalid_scripts_are_rejected() {
    let clock = Arc::new(ManualClock::default());
    let mut hosts = Vec::new();
    for config in Fixture::bundled().unwrap().into_providers() {
        let provider = FakeProvider::new(config, clock.clone(), |_| unreachable!()).unwrap();
        hosts.extend(provider.hosts());
        if provider.id() == "fixture-ssh" {
            assert_eq!(provider.address(&ssh()), Some("agent@build-box.invalid"));
        }
    }
    assert_eq!(hosts.len(), 3);
    assert!(hosts.iter().all(|h| h.state == LinkState::Connecting));
    assert_eq!(hosts.iter().filter(|h| h.region.is_some()).count(), 2);
    for edit in [0, 1, 2] {
        let mut bad: serde_json::Value = serde_json::from_str(BUNDLED_FIXTURE).unwrap();
        match edit {
            0 => bad["providers"][0]["hosts"][0]["id"] = serde_json::json!({"kind":"local"}),
            1 => bad["providers"][0]["hosts"][0]["script"][1]["at_ms"] = 0.into(),
            _ => bad["providers"][0]["hosts"][0]["script"] = serde_json::json!([]),
        }
        assert!(Fixture::parse(&bad.to_string()).is_err());
    }
}
fn actions() -> Vec<Action> {
    let seat = Ulid::new();
    let attachment = Attachment {
        seat,
        generation: 1,
        writer_lease: Ulid::new(),
    };
    vec![
        Action::AcquireLogic {
            capability: [0; 32],
        },
        Action::ListOwnedSeats,
        Action::Spawn {
            spec: LaunchSpec::new("/bin/zsh", "/tmp"),
        },
        Action::Attach {
            seat,
            generation: 1,
        },
        Action::AttachRing {
            seat,
            generation: 1,
            directory: "unused".into(),
            settings: Default::default(),
        },
        Action::RingRecovery {
            attachment: attachment.clone(),
        },
        Action::ReadRing {
            attachment: attachment.clone(),
            from: 1,
            to: 2,
        },
        Action::PublishCheckpoint {
            attachment: attachment.clone(),
            checkpoint: vec![1],
        },
        Action::Input {
            attachment: attachment.clone(),
            input_id: Ulid::new(),
            bytes: vec![1],
        },
        Action::InputContext {
            attachment: attachment.clone(),
        },
        Action::GuardedInput {
            attachment: attachment.clone(),
            input_id: Ulid::new(),
            bytes: vec![1],
            expected: bbt_supervisor::protocol::InputContext {
                epoch: 1,
                last_input_ms: 0,
                sequence: 1,
                draft: false,
                foreground_pid: None,
                canonical: true,
                echo: true,
                settled: true,
            },
        },
        Action::InputStatus {
            seat,
            generation: 1,
            input_id: Ulid::new(),
        },
        Action::Resize {
            attachment: attachment.clone(),
            size: Default::default(),
        },
        Action::Kill {
            attachment: attachment.clone(),
        },
        Action::ScheduleClose {
            attachment: attachment.clone(),
            close_id: Ulid::new(),
            deadline_ms: 100,
        },
        Action::UndoClose {
            attachment,
            close_id: Ulid::new(),
        },
    ]
}
// keeps: spec d1-providers
#[test]
fn all_requests_keep_their_reply_and_down_requests_never_execute_while_events_wait_for_rtt() {
    let clock = Arc::new(ManualClock::default());
    let spy = Arc::new(Mutex::new(Spy::default()));
    let config = Fixture::bundled().unwrap().into_providers().remove(0);
    let shared = spy.clone();
    let mut provider = FakeProvider::new(config, clock.clone(), move |_| {
        Ok(Box::new(SpyLink(shared.clone())))
    })
    .unwrap();
    let mut link = provider.link(ssh()).unwrap();
    for action in actions() {
        assert!(is_link_down(&link.request(action).unwrap_err()));
    }
    assert!(spy.lock().unwrap().actions.is_empty());
    assert!(link.receive(Duration::from_secs(10)).unwrap().is_none());
    clock.set(120);
    let calls = actions();
    for action in &calls {
        assert_eq!(
            link.request(action.clone()).unwrap(),
            Reply::CheckpointCommitted { sequence: 999 }
        );
    }
    assert_eq!(spy.lock().unwrap().actions, calls);
    let event = Event::Output {
        seat: Ulid::new(),
        generation: 1,
        seq: 1,
        bytes: b"latency".to_vec(),
    };
    spy.lock().unwrap().events.push_back(event.clone());
    assert!(link.receive(Duration::ZERO).unwrap().is_none());
    clock.set(159);
    assert!(link.receive(Duration::ZERO).unwrap().is_none());
    clock.set(160);
    assert_eq!(link.receive(Duration::ZERO).unwrap(), Some(event.clone()));
    spy.lock().unwrap().events.push_back(event.clone());
    assert!(link.receive(Duration::ZERO).unwrap().is_none());
    clock.set(12000);
    for action in actions() {
        assert!(is_link_down(&link.request(action).unwrap_err()));
    }
    assert_eq!(spy.lock().unwrap().actions, calls);
    assert!(link.receive(Duration::ZERO).unwrap().is_none());
    clock.set(18000);
    assert_eq!(link.receive(Duration::ZERO).unwrap(), Some(event));
    let states: Vec<_> = provider
        .poll()
        .into_iter()
        .map(|ProviderEvent::HostState { state, .. }| state)
        .collect();
    assert_eq!(
        states,
        vec![
            LinkState::Connecting,
            LinkState::Connected { rtt_ms: 40 },
            LinkState::Disconnected {
                reason: "fixture link dropped".into()
            },
            LinkState::Connecting,
            LinkState::Connected { rtt_ms: 20 }
        ]
    );
    assert!(provider.poll().is_empty());
}

type Trace = Arc<Mutex<Vec<(HostId, Action)>>>;
struct Recorded {
    host: HostId,
    inner: Box<dyn SupervisorConnection>,
    trace: Trace,
}
impl SupervisorConnection for Recorded {
    fn request(&mut self, action: Action) -> bbt_mux::store::Result<Reply> {
        self.trace
            .lock()
            .unwrap()
            .push((self.host.clone(), action.clone()));
        self.inner.request(action)
    }
    fn receive(&mut self, wait: Duration) -> bbt_mux::store::Result<Option<Event>> {
        self.inner.receive(wait)
    }
}
fn providers(
    servers: &[Supervisor],
    clock: Arc<ManualClock>,
    trace: Trace,
) -> Vec<Box<dyn Provider>> {
    let mut result: Vec<Box<dyn Provider>> = vec![Box::new(LocalProvider::new(Recorded {
        host: HostId::Local,
        inner: Box::new(servers[0].connect().unwrap()),
        trace: trace.clone(),
    }))];
    let links: VecDeque<Box<dyn SupervisorConnection>> = servers[1..]
        .iter()
        .map(|s| Box::new(s.connect().unwrap()) as Box<dyn SupervisorConnection>)
        .collect();
    let links = Arc::new(Mutex::new(links));
    for config in Fixture::bundled().unwrap().into_providers() {
        let links = links.clone();
        let trace = trace.clone();
        result.push(Box::new(
            FakeProvider::new(config, clock.clone(), move |host| {
                Ok(Box::new(Recorded {
                    host: host.id.clone(),
                    inner: links.lock().unwrap().pop_front().unwrap(),
                    trace: trace.clone(),
                }))
            })
            .unwrap(),
        ));
    }
    result
}
type States = BTreeMap<HostId, Vec<LinkState>>;
fn wait_states(
    rt: &tokio::runtime::Runtime,
    logic: &Logic,
    subscription: &str,
    states: &mut States,
    done: impl Fn(&States) -> bool,
) {
    use bbt_mux::bus::Delivery;
    let until = Instant::now() + Duration::from_secs(5);
    while !done(states) {
        for item in rt
            .block_on(logic.client.bus.poll(subscription, 32))
            .unwrap()
        {
            let Delivery::Event { envelope, .. } = item else {
                panic!("unexpected provider stream gap")
            };
            envelope.verify().unwrap();
            let ProviderEvent::HostState { host, state } =
                serde_json::from_value(envelope.event.payload).unwrap();
            states.entry(host).or_default().push(state);
        }
        assert!(Instant::now() < until, "provider state timeout: {states:?}");
        std::thread::sleep(Duration::from_millis(10));
    }
}
fn wait_text(
    rt: &tokio::runtime::Runtime,
    logic: &Logic,
    clock: &ManualClock,
    seat: Ulid,
    text: &str,
) -> bbt_mux::streams::Snapshot {
    let until = Instant::now() + Duration::from_secs(5);
    loop {
        clock.advance(20);
        if let Ok(snapshot) = rt.block_on(logic.client.snapshot(seat, 1)) {
            let content: String = snapshot
                .screen
                .cells
                .iter()
                .flatten()
                .map(|cell| cell.c)
                .collect();
            if content.contains(text) {
                return snapshot;
            }
        }
        assert!(Instant::now() < until, "missing {text}");
        std::thread::sleep(Duration::from_millis(10));
    }
}
// keeps: spec d1-providers
#[test]
fn headless_fixture_routes_real_ptys_survives_drop_and_recovers_a_down_host_on_restart() {
    use bbt_mux::bus::Consumer;
    use bbt_mux::registry::{Location, Position, SeatRecord};
    let temp = tempfile::tempdir().unwrap();
    let repository = temp.path().join(".bboxy");
    std::fs::create_dir(&repository).unwrap();
    let config = LogicConfig {
        run_dir: temp.path().join("run"),
        repository,
        ring: Default::default(),
    };
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    let servers: Vec<_> = (0..4)
        .map(|n| {
            Supervisor::start(Config {
                journal_directory: Some(temp.path().join(format!("journal-{n}"))),
                ..Default::default()
            })
            .unwrap()
        })
        .collect();
    let clock = Arc::new(ManualClock::default());
    let trace: Trace = Default::default();
    let key = ed25519_dalek::SigningKey::from_bytes(&[42; 32]);
    let logic = Logic::start_with_providers(
        providers(&servers, clock.clone(), trace.clone()),
        config.clone(),
        key.clone(),
        rt.handle(),
    )
    .unwrap();
    let subscription = rt
        .block_on(logic.client.bus.subscribe(Consumer {
            start: bbt_mux::bus::Start::All,
            ..Consumer::gui(STATE_SUBJECT)
        }))
        .unwrap();
    let mut states = States::new();
    wait_states(&rt, &logic, &subscription, &mut states, |s| s.len() == 3);
    let mut spec = LaunchSpec::new("/bin/zsh", temp.path());
    let gate = temp.path().join("release-output");
    let ack = temp.path().join("output-written");
    spec.args = vec![
        "-f".into(),
        "-c".into(),
        format!(
            "printf 'started\\n'; while read -r line; do if [[ $line == arm-drop ]]; then printf 'armed\\n'; while [[ ! -e '{}' ]]; do sleep 0.01; done; printf 'during-drop\\n'; : > '{}'; else printf 'echo:%s\\n' \"$line\"; fi; done",
            gate.display(),
            ack.display()
        ),
    ];
    let seat = spec.seat;
    let swarm = Ulid::new();
    let mut record = SeatRecord::new(seat, swarm, "remote", "zsh");
    record.location = Location::Ssh {
        host: "build-box".into(),
    };
    let error = rt
        .block_on(logic.client.spawn(record.clone(), spec.clone()))
        .unwrap_err();
    assert!(is_link_down(&error));
    assert!(
        !trace
            .lock()
            .unwrap()
            .iter()
            .any(|(_, a)| matches!(a, Action::Spawn { .. }))
    );
    clock.set(300);
    wait_states(&rt, &logic, &subscription, &mut states, |s| {
        s.values()
            .all(|v| matches!(v.last(), Some(LinkState::Connected { .. })))
    });
    rt.block_on(logic.client.spawn(record.clone(), spec))
        .unwrap();
    rt.block_on(logic.client.gui_resize(seat, 1, 94, 27))
        .unwrap();
    rt.block_on(
        logic
            .client
            .gui_input(seat, 1, Ulid::new(), b"before-drop\n".to_vec()),
    )
    .unwrap();
    wait_text(&rt, &logic, &clock, seat, "echo:before-drop");
    rt.block_on(logic.client.control_catalogue(seat, 1))
        .unwrap();
    rt.block_on(
        logic
            .client
            .gui_input(seat, 1, Ulid::new(), b"arm-drop\n".to_vec()),
    )
    .unwrap();
    wait_text(&rt, &logic, &clock, seat, "armed");
    clock.set(12000);
    wait_states(&rt, &logic, &subscription, &mut states, |s| {
        matches!(s[&ssh()].last(), Some(LinkState::Disconnected { .. }))
    });
    let count = trace.lock().unwrap().len();
    let error = rt
        .block_on(
            logic
                .client
                .gui_input(seat, 1, Ulid::new(), b"refused\n".to_vec()),
        )
        .unwrap_err();
    assert!(is_link_down(&error));
    assert_eq!(trace.lock().unwrap().len(), count);
    std::fs::write(&gate, b"go").unwrap();
    let until = Instant::now() + Duration::from_secs(5);
    while !ack.exists() {
        assert!(Instant::now() < until);
        std::thread::sleep(Duration::from_millis(10));
    }
    let held = rt.block_on(logic.client.snapshot(seat, 1)).unwrap();
    let held_text: String = held.screen.cells.iter().flatten().map(|c| c.c).collect();
    assert!(!held_text.contains("during-drop"));
    clock.set(16000);
    wait_states(&rt, &logic, &subscription, &mut states, |s| {
        s[&ssh()].last() == Some(&LinkState::Connecting)
    });
    clock.set(18000);
    wait_states(&rt, &logic, &subscription, &mut states, |s| {
        s[&ssh()].last() == Some(&LinkState::Connected { rtt_ms: 20 })
    });
    let snapshot = wait_text(&rt, &logic, &clock, seat, "during-drop");
    let history = rt
        .block_on(logic.client.history(seat, 1, 1, snapshot.sequence))
        .unwrap();
    assert!(history.gap.is_none());
    let position = Position {
        swarm,
        tab: Ulid::new(),
        split_path: vec![],
        ratios: vec![],
        layout_revision: 1,
    };
    rt.block_on(logic.client.close(seat, 1, Ulid::new(), position, 5000))
        .unwrap();
    let close = rt
        .block_on(logic.client.record(seat))
        .unwrap()
        .close
        .unwrap();
    rt.block_on(logic.client.undo(seat, 1, close.revision, 1))
        .unwrap();
    assert_eq!(
        states[&ssh()],
        vec![
            LinkState::Connecting,
            LinkState::Connected { rtt_ms: 40 },
            LinkState::Disconnected {
                reason: "fixture link dropped".into()
            },
            LinkState::Connecting,
            LinkState::Connected { rtt_ms: 20 }
        ]
    );
    logic.shutdown().unwrap();
    let spawn_count = trace
        .lock()
        .unwrap()
        .iter()
        .filter(|(_, a)| matches!(a, Action::Spawn { .. }))
        .count();
    let logic = Logic::start_with_providers(
        providers(&servers, clock.clone(), trace.clone()),
        config,
        key,
        rt.handle(),
    )
    .unwrap();
    clock.advance(400);
    wait_text(&rt, &logic, &clock, seat, "during-drop");
    assert_eq!(
        rt.block_on(logic.client.record(seat)).unwrap().location,
        record.location
    );
    logic.shutdown().unwrap();
    let trace = trace.lock().unwrap();
    assert_eq!(
        trace
            .iter()
            .filter(|(_, a)| matches!(a, Action::Spawn { .. }))
            .count(),
        spawn_count
    );
    for (host, action) in trace.iter() {
        if !matches!(action, Action::ListOwnedSeats) {
            assert_eq!(host, &ssh(), "{action:?}");
        }
    }
    for kind in [
        "Spawn",
        "AttachRing",
        "RingRecovery",
        "ReadRing",
        "PublishCheckpoint",
        "Input",
        "Resize",
        "ScheduleClose",
        "UndoClose",
    ] {
        assert!(
            trace.iter().any(|(host, action)| host == &ssh()
                && serde_json::to_value(action).unwrap()["kind"] == kind),
            "missing {kind}"
        );
    }
    println!(
        "D1b: fixture HostState sequence verified; disconnected effects rejected; RTT queued events; real PTY output retained through drop/reconnect and startup-down recovery, with no extra spawn"
    );
}
