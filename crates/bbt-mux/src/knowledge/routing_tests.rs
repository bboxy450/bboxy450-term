// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
#![allow(
    clippy::disallowed_methods,
    reason = "private IO fixtures, never the operator or UI thread"
)]
use super::*;
use crate::store::Events;
use crate::{
    identity::IdentityRuntime,
    mcp::{Actor, Call, SeatActor},
};
use serde_json::{Value, json};

struct Link;
impl SupervisorConnection for Link {
    fn request(&mut self, action: Action) -> Result<Reply> {
        match action {
            Action::ListOwnedSeats => Ok(Reply::Seats { seats: vec![] }),
            _ => panic!("no terminal effects in workspace fixture"),
        }
    }
    fn receive(&mut self, _: Duration) -> Result<Option<Event>> {
        Ok(None)
    }
}
struct Fixture {
    worker: Worker,
    runtime: tokio::runtime::Runtime,
    root: tempfile::TempDir,
    first: PathBuf,
    second: PathBuf,
    a: SeatRecord,
    b: SeatRecord,
}
impl Fixture {
    fn open() -> Self {
        let root = tempfile::tempdir().unwrap();
        let startup = root.path().join("startup");
        let first = root.path().join("first");
        let second = root.path().join("second");
        let mut a = SeatRecord::new(Ulid::new(), Ulid::new(), "First seat", "zsh");
        let mut b = SeatRecord::new(Ulid::new(), Ulid::new(), "Second seat", "zsh");
        a.tasks.push(Ulid(2));
        b.tasks.push(Ulid(2));
        seed(&startup, "startup", &a);
        seed(&first, "first", &a);
        seed(&second, "second", &b);
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .unwrap();
        let run = root.path().join("run");
        let key = SigningKey::from_bytes(&[51; 32]);
        let identity = IdentityRuntime::open(&run, key.clone(), runtime.handle().clone()).unwrap();
        let bus =
            BusHandle::start(runtime.handle(), Broker::open(&run, key).unwrap(), 128).unwrap();
        let (tx, _rx) = mpsc::channel(128);
        let client = LogicClient {
            tx,
            bus: bus.clone(),
            exit_cause: Arc::new(OnceLock::new()),
            capture_diagnostics: Arc::new(Recorder::default()),
            gui_peers: Arc::default(),
            attention_observer_guard: Arc::default(),
            current_attention: Arc::default(),
            captures: Arc::default(),
            gui_watch_epoch: Arc::default(),
            window_journal: None,
        };
        let worker = Worker::open(
            vec![Box::new(LocalProvider::new(Link))],
            LogicConfig {
                run_dir: run,
                repository: startup,
                ring: Default::default(),
            },
            bus,
            runtime.handle().clone(),
            client,
            identity,
            None,
        )
        .unwrap();
        for record in [&a, &b] {
            worker.registry.insert(record).unwrap();
        }
        worker
            .registry
            .update_swarm_workdir(a.swarm, Some(first.to_str().unwrap()))
            .unwrap();
        worker
            .registry
            .update_swarm_workdir(b.swarm, Some(second.to_str().unwrap()))
            .unwrap();
        Self {
            worker,
            runtime,
            root,
            first,
            second,
            a,
            b,
        }
    }
    fn actor(&mut self, record: &SeatRecord) -> SeatActor {
        self.worker.messaging.sync_seat(record).unwrap();
        let (_, epoch, issuer) = self
            .worker
            .messaging
            .authority(record.id, record.generation)
            .unwrap();
        SeatActor {
            seat_id: record.id,
            process_generation: record.generation,
            swarm_id: record.swarm,
            role: record.role.clone(),
            grant_epoch: epoch,
            issuer_kind: issuer,
        }
    }
    fn ready(&mut self, actor: &SeatActor) -> WorkspacePort {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            self.worker.workspaces.poll(&self.worker.registry);
            match self.worker.seat_workspace(actor) {
                Ok(port) => match self.worker.scan_port(&port.binding) {
                    Ok(port) => return port,
                    Err(error)
                        if error
                            .downcast_ref::<crate::knowledge::workspaces::Error>()
                            .is_some_and(|e| {
                                matches!(e, crate::knowledge::workspaces::Error::Pending(_))
                            }) => {}
                    Err(error) => panic!("{error}"),
                },
                Err(error)
                    if error
                        .downcast_ref::<crate::knowledge::workspaces::Error>()
                        .is_some_and(|e| {
                            matches!(e, crate::knowledge::workspaces::Error::Pending(_))
                        }) => {}
                Err(error) => panic!("{error}"),
            }
            assert!(Instant::now() < deadline, "workspace never completed");
            std::thread::sleep(Duration::from_millis(2));
        }
    }
    fn knowledge(&mut self, actor: &SeatActor, tool: &str, args: Value) -> Value {
        let call = Call {
            op_id: Ulid::new(),
            tool: tool.into(),
            args,
        };
        let journal = self.worker.control_journal(actor.seat_id).unwrap();
        self.worker.knowledge_call(actor, &call, &journal).unwrap()
    }
    // This routing fixture has no live process owner. Exercise the same captured
    // source IO and root journal; harmony_host covers real owner admission/jobs.
    fn write(&mut self, actor: &SeatActor, call: &Call) -> Value {
        let port = self.ready(actor);
        let journal = self.worker.control_journal(actor.seat_id).unwrap();
        let journal = self.worker.root_journal(&journal, &port);
        let authority = Actor::from(actor);
        journal
            .effect(&authority, call, || {
                let source = port.host.source(&port.index, &self.worker.registry);
                let mut prepared = crate::knowledge::effects::prepare(
                    &source,
                    &self.worker.registry,
                    actor.swarm_id,
                    &authority,
                    call,
                )
                .unwrap();
                let key = self
                    .worker
                    .identity
                    .keys()
                    .leaf(actor.seat_id, actor.process_generation)
                    .unwrap();
                crate::knowledge::effects::apply(
                    &source,
                    &mut prepared,
                    call,
                    &crate::knowledge::WriteContext::seat(
                        actor.seat_id,
                        Some(actor.process_generation),
                        &key,
                    ),
                )
                .value
            })
            .unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.worker.workspaces.cache.close();
    }
}
fn seed(path: &std::path::Path, label: &str, seat: &SeatRecord) {
    std::fs::create_dir_all(path.join(".bboxy/notes")).unwrap();
    std::fs::create_dir_all(path.join(".bboxy/tasks")).unwrap();
    std::fs::write(
        path.join(".bboxy/notes/same.md"),
        format!(
            "---\nid: {}\nnumber: 1\ntype: note\ntitle: Same\n---\n{label} content\n",
            Ulid(1)
        ),
    )
    .unwrap();
    std::fs::write(path.join(".bboxy/tasks/task.md"),format!("---\nid: {}\nnumber: 2\ntype: task\ntitle: {label} task\nstatus: running\nassignee: {}\nverify: 'true'\n---\n{label} brief\n",Ulid(2),seat.id)).unwrap();
}

#[test]
fn workspace_routing_actor_roots_isolate_colliding_records_tasks_writes_and_replays() {
    let mut fixture = Fixture::open();
    let a = fixture.actor(&fixture.a.clone());
    let b = fixture.actor(&fixture.b.clone());
    let first = fixture.ready(&a);
    let second = fixture.ready(&b);
    assert_ne!(first.binding.root_id, second.binding.root_id);
    let legacy = fixture.worker.gui_workspace(None).unwrap();
    let reference = legacy
        .host
        .with_source(&legacy.index, &fixture.worker.registry, |source| {
            source.resolve(crate::knowledge::data_source::ResolveRequest {
                locator: "#1".into(),
            })
        })
        .unwrap();
    let record = legacy
        .host
        .with_source(&legacy.index, &fixture.worker.registry, |source| {
            source.fetch(crate::knowledge::data_source::FetchRequest { reference })
        })
        .unwrap();
    assert_eq!(record.body, "startup content\n");

    for (actor, label, port) in [(&a, "first", &first), (&b, "second", &second)] {
        for locator in ["#1", "same", &Ulid(1).to_string()] {
            let value = fixture.knowledge(actor, "knowledge_fetch", json!({"locator":locator}));
            assert_ne!(value["isError"], true, "{value}");
            assert_eq!(
                value["structuredContent"]["body"],
                format!("{label} content\n")
            );
        }
        let search = fixture.knowledge(actor, "knowledge_search", json!({"query":"content"}));
        assert!(
            search.to_string().contains(&format!("{label} content")),
            "{search}"
        );
        let call = Call {
            op_id: Ulid::new(),
            tool: "task_status".into(),
            args: json!({"task_id":"#2"}),
        };
        let journal = fixture.worker.control_journal(actor.seat_id).unwrap();
        let journal = fixture.worker.root_journal(&journal, port);
        let value = fixture
            .worker
            .work_item_call(port, &Actor::from(actor), &call, &journal)
            .unwrap();
        assert_ne!(value["isError"], true, "{value}");
        assert!(
            value.to_string().contains(&format!("{label} task")),
            "{value}"
        );
        let list = port
            .host
            .with_source(&port.index, &fixture.worker.registry, |source| {
                crate::knowledge::read(source, "list", json!({"filter":{}}))
            })
            .unwrap();
        assert_eq!(list["items"].as_array().unwrap().len(), 2);
    }
    let call = Call {
        op_id: Ulid::new(),
        tool: "knowledge_write".into(),
        args: json!({"path":"notes/created.md","frontmatter":{"type":"note","title":"Only first"},"body":"created in first"}),
    };
    let written = fixture.write(&a, &call);
    assert_ne!(written["isError"], true, "{written}");
    assert!(fixture.first.join(".bboxy/notes/created.md").is_file());
    assert!(!fixture.second.join(".bboxy/notes/created.md").exists());
    let intent = first
        .index
        .activity()
        .after(0, 128)
        .unwrap()
        .into_iter()
        .find(|row| row.event.subject.ends_with("/write_prepared"))
        .expect("signed intent retained in source root activity");
    intent.verify().unwrap();
    let signer = fixture
        .worker
        .identity
        .keys()
        .leaf(a.seat_id, a.process_generation)
        .unwrap()
        .verifying_key()
        .to_bytes();
    assert_eq!(intent.provenance.signer, signer);
    assert!(
        !second
            .index
            .activity()
            .after(0, 128)
            .unwrap()
            .iter()
            .any(|row| row.event.id == intent.event.id)
    );
    assert!(
        written["structuredContent"]["record"]["frontmatter"]["provenance"]["log"]
            .as_str()
            .is_some_and(|path| path.starts_with(".bboxy/log/"))
    );

    fixture
        .worker
        .registry
        .update_swarm_workdir(a.swarm_id, Some(fixture.second.to_str().unwrap()))
        .unwrap();
    assert!(
        fixture
            .worker
            .workspaces
            .current(&fixture.worker.registry, &first.binding)
            .is_err(),
        "old read survived metadata change before next admission"
    );
    let moved = fixture.ready(&a);
    assert_eq!(moved.binding.root_id, second.binding.root_id);
    assert!(Arc::ptr_eq(
        &moved.index,
        &fixture
            .worker
            .workspaces
            .port(second.binding.clone())
            .unwrap()
            .index
    ));
    let replay = fixture.write(&a, &call);
    assert_eq!(
        replay["isError"], true,
        "operation replayed into a different root: {replay}"
    );
    assert!(!fixture.second.join(".bboxy/notes/created.md").exists());
    let stale = bbt_proto::gui::SwarmScope {
        swarm: a.swarm_id,
        revision: first.binding.revision,
    };
    assert!(fixture.worker.gui_workspace(Some(stale)).is_err());
    let _ = &fixture.runtime;
}

#[test]
fn workspace_routing_unavailable_root_keeps_basic_orientation_and_chat_available() {
    let mut fixture = Fixture::open();
    let record = fixture.a.clone();
    let actor = fixture.actor(&record);
    let missing = fixture.root.path().join("removed");
    std::fs::create_dir(&missing).unwrap();
    fixture
        .worker
        .registry
        .update_swarm_workdir(record.swarm, Some(missing.to_str().unwrap()))
        .unwrap();
    std::fs::remove_dir(&missing).unwrap();
    assert!(fixture.worker.seat_workspace(&actor).is_err());
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        fixture.worker.workspaces.poll(&fixture.worker.registry);
        let error = fixture.worker.seat_workspace(&actor).err().unwrap();
        if error
            .downcast_ref::<crate::knowledge::workspaces::Error>()
            .is_some_and(|e| matches!(e, crate::knowledge::workspaces::Error::Unavailable { .. }))
        {
            break;
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(2));
    }
    let orient = fixture
        .worker
        .control_read(
            &actor,
            &record,
            &Call {
                op_id: Ulid::new(),
                tool: "orient".into(),
                args: json!({}),
            },
            None,
            None,
        )
        .unwrap();
    assert_eq!(orient["actor"]["seat_id"], json!(actor.seat_id));
    assert_eq!(orient["tasks"]["items"], json!([]));
    assert!(!orient.to_string().contains("startup brief"));
    let request = crate::messaging::api::Request::Send {
        message: crate::messaging::wire::SendRequest::text(
            format!("seats/{}/inbox", record.id),
            "chat still works",
        ),
    };
    assert!(
        fixture
            .worker
            .messaging
            .request(record.id, record.generation, request, false, None, now_ms())
            .is_ok()
    );
    let b = fixture.actor(&fixture.b.clone());
    fixture.ready(&b);
    assert!(
        fixture
            .knowledge(&b, "knowledge_fetch", json!({"locator":"#1"}))
            .to_string()
            .contains("second content")
    );
}

#[test]
fn workspace_routing_panicked_prepare_refuses_only_its_root_and_can_retry() {
    let mut fixture = Fixture::open();
    let actor = fixture.actor(&fixture.a.clone());
    fixture.worker.workspaces.fail_next_prepare = true;
    assert!(fixture.worker.seat_workspace(&actor).is_err());
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        fixture.worker.workspaces.poll(&fixture.worker.registry);
        let error = fixture.worker.seat_workspace(&actor).err().unwrap();
        if error.to_string().contains("panicked") {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "panic left workspace permanently pending"
        );
        std::thread::sleep(Duration::from_millis(2));
    }
    let other = fixture.actor(&fixture.b.clone());
    fixture.ready(&other);
    assert!(
        fixture
            .knowledge(&other, "knowledge_fetch", json!({"locator":"#1"}))
            .to_string()
            .contains("second content")
    );
    std::thread::sleep(Duration::from_secs(1));
    fixture.ready(&actor);
}

#[test]
fn workspace_task_read_queue_is_bounded_while_its_scan_is_held() {
    use std::sync::{
        Condvar, Mutex,
        atomic::{AtomicBool, Ordering},
    };
    type Hold = Arc<(AtomicBool, Mutex<bool>, Condvar)>;
    struct Release(Hold);
    impl Drop for Release {
        fn drop(&mut self) {
            *self.0.1.lock().unwrap() = true;
            self.0.2.notify_all();
        }
    }
    let mut fixture = Fixture::open();
    let actor = fixture.actor(&fixture.a.clone());
    let port = fixture.ready(&actor);
    let hold: Hold = Arc::new((AtomicBool::new(false), Mutex::new(false), Condvar::new()));
    let release = Release(hold.clone());
    let wait = hold.clone();
    fixture.worker.workspaces.scan_hook = Some(Arc::new(move |_| {
        wait.0.store(true, Ordering::Release);
        let guard = wait.1.lock().unwrap();
        drop(wait.2.wait_while(guard, |released| !*released).unwrap());
    }));
    fixture
        .worker
        .workspaces
        .reconcile(&port.binding.root_id)
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while !hold.0.load(Ordering::Acquire) {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(2));
    }
    let mut receivers = Vec::new();
    for _ in 0..33 {
        let (reply, receiver) = oneshot::channel();
        fixture.worker.start_gui_task_read(
            Ulid::new(),
            bbt_proto::messaging::TaskRequest::Status {
                task_path: Some("tasks/task.md".into()),
                task_id: None,
            },
            Some(bbt_proto::gui::SwarmScope {
                swarm: actor.swarm_id,
                revision: port.binding.revision,
            }),
            crate::mcp::Origin::default(),
            reply,
            None,
        );
        receivers.push(receiver);
    }
    assert_eq!(
        fixture.worker.workspace_reads[&port.binding.root_id].len(),
        32
    );
    let refused = receivers
        .last_mut()
        .unwrap()
        .try_recv()
        .unwrap()
        .unwrap_err();
    assert!(refused.to_string().starts_with("provider_busy:"));
    drop(release);
}

// keeps: event destinations follow authoritative defaults without requiring a prior read.
#[test]
fn workspace_event_routes_default_swarms_and_only_proven_explicit_roots() {
    let mut fixture = Fixture::open();
    let startup = fixture
        .worker
        .workspaces
        .cache
        .startup()
        .root_id()
        .to_owned();
    let legacy = SeatRecord::new(Ulid::new(), Ulid::new(), "legacy default", "zsh");
    fixture.worker.registry.insert(&legacy).unwrap();
    let named = fixture
        .worker
        .registry
        .rename_swarm(Ulid::new(), "default, no seat")
        .unwrap();
    let selected = fixture
        .worker
        .workspaces
        .bound_swarms(&fixture.worker.registry, &startup)
        .unwrap();
    assert!(selected.contains(&legacy.swarm));
    assert!(selected.contains(&named.id));
    assert!(!selected.contains(&fixture.a.swarm));
    assert!(!selected.contains(&fixture.b.swarm));
    let unregistered = Ulid::new();
    assert!(
        fixture
            .worker
            .workspaces
            .matches_root(&fixture.worker.registry, unregistered, &startup)
            .unwrap()
    );
    assert!(
        fixture
            .worker
            .registry
            .swarm(unregistered)
            .unwrap()
            .is_none(),
        "event routing created swarm authority"
    );
    assert_eq!(
        fixture.worker.workspaces.cache.roots().count(),
        1,
        "enumeration mounted unrelated roots"
    );
    let explicit = fixture
        .worker
        .registry
        .update_swarm_workdir(
            Ulid::new(),
            Some(fixture.worker.config.repository.to_str().unwrap()),
        )
        .unwrap();
    // Explicit metadata has to complete canonical IO, even when it will reuse startup.
    assert!(
        !fixture
            .worker
            .workspaces
            .matches_root(&fixture.worker.registry, explicit.id, &startup)
            .unwrap()
    );
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        fixture.worker.workspaces.poll(&fixture.worker.registry);
        if fixture
            .worker
            .workspaces
            .matches_root(&fixture.worker.registry, explicit.id, &startup)
            .unwrap()
        {
            break;
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(2));
    }
    assert_eq!(fixture.worker.workspaces.cache.roots().count(), 1);
    let other = fixture.a.swarm;
    assert!(
        !fixture
            .worker
            .workspaces
            .matches_root(&fixture.worker.registry, other, &startup)
            .unwrap()
    );
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        fixture.worker.workspaces.poll(&fixture.worker.registry);
        if let Ok(binding) = fixture.worker.workspaces.cache.current(other) {
            assert_ne!(binding.root_id, startup);
            break;
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(2));
    }
    assert!(
        !fixture
            .worker
            .workspaces
            .matches_root(&fixture.worker.registry, other, &startup)
            .unwrap()
    );
    assert!(
        fixture
            .worker
            .workspaces
            .cache
            .current(fixture.b.swarm)
            .is_err(),
        "explicit unrelated root was mounted by enumeration"
    );
    let removed = fixture.root.path().join("removed-route");
    std::fs::create_dir(&removed).unwrap();
    let missing = fixture
        .worker
        .registry
        .update_swarm_workdir(Ulid::new(), Some(removed.to_str().unwrap()))
        .unwrap();
    std::fs::remove_dir(&removed).unwrap();
    assert!(
        !fixture
            .worker
            .workspaces
            .matches_root(&fixture.worker.registry, missing.id, &startup)
            .unwrap()
    );
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        fixture.worker.workspaces.poll(&fixture.worker.registry);
        if matches!(
            fixture.worker.workspaces.cache.current(missing.id),
            Err(crate::knowledge::workspaces::Error::Unavailable { .. })
        ) {
            break;
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(2));
    }
    assert!(
        !fixture
            .worker
            .workspaces
            .matches_root(&fixture.worker.registry, missing.id, &startup)
            .unwrap()
    );
}
