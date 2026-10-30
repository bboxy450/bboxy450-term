// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Live plugin connections (`PluginHost`). A separate test binary: each
//! fixture execs several freshly copied plugins, and that burst would share
//! the conformance checker's fixed 2 s budgets with the library unit tests.
#![allow(
    clippy::disallowed_methods,
    reason = "owned private account and plugin fixtures on IO, never operator identity"
)]
use bbt_mux::{
    Ulid,
    bus::{Ack, Broker, BusHandle, Consumer, Delivery, Start},
    knowledge::data_source as d,
    mcp::{Actor, Door},
    plugins::{
        DiscoveryRoots, HOOK_QUEUE, HookEvent, PluginHost, ProviderConfig, Registry,
        RegistryConfig, Verdict,
    },
    store::{events::Draft, sqlite::SqliteStore},
};
use bbt_plugin::{
    contracts::Contract,
    policy::{FailurePolicy, Tier},
    registry::Approval,
    trust::Grants,
};
use serde_json::{Value, json};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    time::{Duration, Instant},
};

fn operator() -> Actor {
    Actor::Operator {
        device_id: Ulid::from(91_u128),
        door: Door::Ctl,
        issued_by: None,
    }
}

/// One fixture at a time keeps the exec burst, and its timing, small.
static SERIAL: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

struct Fixture {
    _serial: tokio::sync::MutexGuard<'static, ()>,
    temp: tempfile::TempDir,
    registry: Registry,
    host: PluginHost,
    candidate: String,
    delay: PathBuf,
    hook_log: PathBuf,
}

/// Fixture knobs: Python lines of the reference plugin to replace, and the
/// operator's approval at trust (the contract minimum when absent).
#[derive(Default)]
struct Variant {
    concurrent: bool,
    knobs: Vec<(&'static str, String)>,
    approval: Option<Approval>,
}

impl Variant {
    /// A plugin declaring `hooks` that answers every hook with `decision`.
    fn hooks(hooks: Value, decision: &str, approval: Approval) -> Self {
        Self {
            knobs: vec![
                ("HOOKS = []", format!("HOOKS = {hooks}")),
                ("DECISION = \"allow\"", format!("DECISION = {decision:?}")),
            ],
            approval: Some(approval),
            ..Self::default()
        }
    }
}

/// Export tier with the given payload and block grants.
fn approval(payload: &[&str], block: Option<FailurePolicy>) -> Approval {
    Approval {
        tier: Tier::Export,
        grants: Grants {
            payload_events: payload.iter().map(|event| (*event).to_owned()).collect(),
            block,
        },
    }
}

impl Fixture {
    /// A checked, enabled repo_graph reference plugin on a private account.
    async fn new(concurrent: bool) -> Self {
        Self::with(Variant {
            concurrent,
            ..Variant::default()
        })
        .await
    }

    async fn with(variant: Variant) -> Self {
        let serial = SERIAL.lock().await;
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("repo/.bboxy");
        fs::create_dir_all(root.join("plugins")).unwrap();
        let delay = temp.path().join("delay");
        let hook_log = temp.path().join("hooks.jsonl");
        let mut source = include_str!("fixtures/plugins/bboxy450-term-data_source-reference")
            .replacen("SYSTEM = \"data_source\"", "SYSTEM = \"repo_graph\"", 1)
            .replacen(
                "CONCURRENT = False",
                if variant.concurrent {
                    "CONCURRENT = True"
                } else {
                    "CONCURRENT = False"
                },
                1,
            )
            .replacen(
                "DELAY_FILE = None",
                &format!("DELAY_FILE = {:?}", delay.to_str().unwrap()),
                1,
            )
            .replacen(
                "HOOK_LOG = None",
                &format!("HOOK_LOG = {:?}", hook_log.to_str().unwrap()),
                1,
            );
        for (line, replacement) in &variant.knobs {
            assert!(source.contains(line), "fixture knob {line}");
            source = source.replacen(line, replacement, 1);
        }
        let path = root.join("plugins/bboxy450-term-repo_graph-reference");
        fs::write(&path, source).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        let account = SqliteStore::open(&temp.path().join("account")).unwrap();
        let registry = Registry::with_config(
            account,
            DiscoveryRoots::new(&temp.path().join("app"), &temp.path().join("repo")),
            RegistryConfig {
                descriptor_timeout: Duration::from_secs(30),
                conformance_timeout: Duration::from_secs(30),
            },
        )
        .unwrap();
        let entry = registry.refresh().unwrap().remove(0);
        let candidate = entry.candidate.id.clone();
        registry
            .trust(
                &operator(),
                &candidate,
                entry.candidate.sha256.as_deref().unwrap(),
                variant.approval,
            )
            .unwrap();
        let inspected = registry
            .inspection(&candidate)
            .unwrap()
            .unwrap()
            .run()
            .await;
        registry.finish_inspection(inspected).unwrap();
        let checked = registry
            .conformance(&operator(), &candidate)
            .unwrap()
            .unwrap()
            .run()
            .await;
        let checked = registry.finish_conformance(checked).unwrap();
        assert!(checked.conformance.as_ref().unwrap().ok, "{checked:?}");
        registry.set_enabled(&operator(), &candidate, true).unwrap();
        let host = PluginHost::new(
            registry.clone(),
            &root,
            tokio::runtime::Handle::current(),
            ProviderConfig {
                timeout: Duration::from_secs(30),
            },
        )
        .unwrap();
        Self {
            _serial: serial,
            temp,
            registry,
            host,
            candidate,
            delay,
            hook_log,
        }
    }

    /// Every `hook` call the plugin received, in order.
    fn hooks_received(&self) -> Vec<Value> {
        fs::read_to_string(&self.hook_log)
            .unwrap_or_default()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect()
    }

    async fn wait_for_hooks(&self, count: usize) -> Vec<Value> {
        let until = Instant::now() + Duration::from_secs(10);
        loop {
            let received = self.hooks_received();
            if received.len() >= count || Instant::now() > until {
                return received;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    }

    async fn wait_for_outcome(&self, outcome: &str) -> bbt_mux::plugins::HookOutcome {
        let until = Instant::now() + Duration::from_secs(10);
        loop {
            if let Some(found) = self
                .host
                .hook_outcomes()
                .into_iter()
                .find(|o| o.outcome == outcome)
            {
                return found;
            }
            assert!(
                Instant::now() < until,
                "no {outcome}: {:?}",
                self.host.hook_outcomes()
            );
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    }

    /// A private bus, attached to the host for bus hooks and outcomes.
    fn bus(&self) -> BusHandle {
        let directory = self.temp.path().join("bus");
        fs::create_dir_all(&directory).unwrap();
        let key = ed25519_dalek::SigningKey::from_bytes(&[5; 32]);
        let broker = Broker::open(&directory, key).unwrap();
        let bus = BusHandle::start(&tokio::runtime::Handle::current(), broker, 64).unwrap();
        self.host.attach_bus(bus.clone());
        bus
    }

    async fn decide(&self, event: HookEvent) -> (Verdict, Duration) {
        let host = self.host.clone();
        tokio::task::spawn_blocking(move || {
            let started = Instant::now();
            let verdict = host.decide(&event, Instant::now() + Duration::from_secs(10));
            (verdict, started.elapsed())
        })
        .await
        .unwrap()
    }

    async fn call_at(&self, deadline: Instant) -> d::Result<Value> {
        let host = self.host.clone();
        let candidate = self.candidate.clone();
        tokio::task::spawn_blocking(move || {
            host.call(
                &candidate,
                Contract::RepoGraph,
                "changes_since",
                json!({"cursor": "0"}),
                deadline,
            )
        })
        .await
        .unwrap()
    }

    async fn changes(&self) -> d::Result<Value> {
        self.call_at(Instant::now() + Duration::from_secs(10)).await
    }

    fn enable(&self, enabled: bool) {
        self.registry
            .set_enabled(&operator(), &self.candidate, enabled)
            .unwrap();
    }

    fn generation(&self) -> Option<u64> {
        self.host
            .connected()
            .into_iter()
            .find(|(candidate, _)| *candidate == self.candidate)
            .map(|(_, generation)| generation)
    }

    async fn reconcile(&self) {
        let host = self.host.clone();
        tokio::task::spawn_blocking(move || host.reconcile())
            .await
            .unwrap();
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let host = self.host.clone();
        let _ = std::thread::spawn(move || host.close()).join();
    }
}

// keeps: PluginConnection — one live connection per (candidate, sha, generation);
// disable closes it and the next call is refused; re-enable opens a new generation.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn enabled_plugins_serve_calls_and_disable_closes_their_connection() {
    let f = Fixture::new(false).await;
    assert!(f.host.connected().is_empty(), "connections open lazily");
    let changes = f.changes().await.unwrap();
    assert_eq!(changes["next"], "1");
    let opened = f.generation().expect("the first call opened a connection");
    f.changes().await.unwrap();
    assert_eq!(f.generation(), Some(opened), "the connection is reused");

    f.enable(false);
    f.reconcile().await;
    assert!(
        f.host.connected().is_empty(),
        "disable closes the connection"
    );
    assert_eq!(f.changes().await.unwrap_err().code, "plugin_disabled");

    // A -> B -> A: enabled again, at a newer generation, without a reconcile.
    f.enable(true);
    f.changes().await.unwrap();
    let reopened = f.generation().unwrap();
    assert!(reopened > opened, "{reopened} <= {opened}");
    f.enable(false);
    f.enable(true);
    f.changes().await.unwrap();
    assert!(
        f.generation().unwrap() > reopened,
        "a call never reuses a superseded generation's connection"
    );
    assert_eq!(
        f.call_at(Instant::now()).await.unwrap_err().code,
        "deadline_exceeded",
        "an exhausted deadline never dispatches"
    );
}

// keeps: PluginConnection — a result that completes after the activation
// generation changed is refused, and the stale connection is closed.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_result_from_a_superseded_generation_is_refused() {
    let f = Fixture::new(false).await;
    f.changes().await.unwrap();
    fs::write(&f.delay, b"").unwrap();
    let slow = {
        let host = f.host.clone();
        let candidate = f.candidate.clone();
        tokio::task::spawn_blocking(move || {
            host.call(
                &candidate,
                Contract::RepoGraph,
                "changes_since",
                json!({"cursor": "0"}),
                Instant::now() + Duration::from_secs(10),
            )
        })
    };
    tokio::time::sleep(Duration::from_millis(300)).await;
    // concurrent:false keeps one request in flight and refuses, never queues.
    let busy = f.changes().await.unwrap_err();
    assert_eq!(busy.code, "provider_busy");
    assert_eq!(busy.retry_after_ms, Some(100));
    f.enable(false);
    f.enable(true);
    assert_eq!(slow.await.unwrap().unwrap_err().code, "provider_changed");
    assert!(f.host.connected().is_empty(), "the stale connection closed");
    fs::remove_file(&f.delay).unwrap();
    f.changes().await.unwrap();
}

// keeps: G3 concurrent — a descriptor declaring concurrent admits parallel calls.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn concurrent_descriptors_admit_parallel_requests() {
    let f = Fixture::new(true).await;
    f.changes().await.unwrap();
    fs::write(&f.delay, b"").unwrap();
    let started = Instant::now();
    let (first, second) = tokio::join!(f.changes(), f.changes());
    first.unwrap();
    second.unwrap();
    // Each held call takes one second; serial service would need two.
    assert!(
        started.elapsed() < Duration::from_millis(1900),
        "{:?}",
        started.elapsed()
    );
    // A workspace drop closes the host; later calls are refused.
    let host = f.host.clone();
    tokio::task::spawn_blocking(move || host.close())
        .await
        .unwrap();
    assert_eq!(f.changes().await.unwrap_err().code, "provider_closed");
}

// keeps: G2 seat tools — which contract methods a seat may call, and at which role.
#[test]
fn seat_tools_exclude_host_only_and_host_owned_methods() {
    use bbt_plugin::policy::RoleClass::{Oracle, Worker};
    for (contract, method, role) in [
        (Contract::DataSource, "fetch", Some(Worker)),
        (Contract::DataSource, "changes_since", Some(Worker)),
        (Contract::DataSource, "write", None),
        (Contract::DataSource, "commit", None),
        (Contract::TaskSystem, "list", Some(Worker)),
        (Contract::TaskSystem, "delegate", None),
        (Contract::TaskSystem, "report", None),
        (Contract::RepoGraph, "changes_since", Some(Worker)),
        (Contract::RepoGraph, "describe", None),
        (Contract::DecisionModel, "choose", Some(Oracle)),
        (Contract::Session, "describe", Some(Worker)),
        (Contract::EventSink, "deliver", None),
    ] {
        assert_eq!(
            bbt_mux::plugins::seat_role(contract, method),
            role,
            "{} {method}",
            contract.name()
        );
    }
}

fn pull(filter: &str) -> Consumer {
    Consumer {
        name: None,
        filter: vec![filter.into()],
        start: Start::New,
        ack: Ack::None,
        deliver: "pull".into(),
        queue_max: 64,
        ack_wait_ms: 30_000,
        max_deliver: 1,
    }
}

// keeps: G3 hooks / PluginHook — a plugin receives harness and bus events by
// subscription; the payload only with its grant; outcomes become bus activity;
// a disabled or superseded activation receives nothing.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn hooks_receive_harness_and_bus_events_with_payload_only_when_granted() {
    let hooks = json!([
        {"event":"PreToolUse","phase":"before","budget_ms":1000},
        {"event":"bus","phase":"after","budget_ms":1000,"subject":"task/*/status"}
    ]);
    // PreToolUse carries its payload by grant; the bus payload is not granted.
    let f = Fixture::with(Variant::hooks(
        hooks,
        "deny",
        approval(&["PreToolUse"], None),
    ))
    .await;
    let bus = f.bus();
    let outcomes = bus.subscribe(pull("plugin/>")).await.unwrap();
    // The host's bus consumer subscribes asynchronously after attach.
    tokio::time::sleep(Duration::from_millis(300)).await;
    let seat = Ulid::from(7_u128);
    f.host.notify(&HookEvent::harness(
        "pre_tool_use",
        Some(seat),
        json!({"tool_name":"ls"}),
    ));
    for subject in ["seat/1/state", "task/7/status"] {
        bus.publish(
            "account",
            Draft::new(subject, "test", json!({"status":"done"})),
        )
        .await
        .unwrap();
    }
    let received = f.wait_for_hooks(2).await;
    assert_eq!(received.len(), 2, "{received:?}");
    let harness = received
        .iter()
        .find(|h| h["event"] == "pre_tool_use")
        .unwrap();
    assert_eq!(harness["phase"], "before");
    assert_eq!(harness["seat"], seat.to_string());
    assert_eq!(harness["budget_ms"], 1000);
    assert_eq!(harness["payload"], json!({"tool_name":"ls"}));
    let event = received.iter().find(|h| h["event"] == "bus").unwrap();
    assert_eq!(event["subject"], "task/7/status");
    assert_eq!(event["phase"], "after");
    assert!(event.get("payload").is_none(), "bus payload is not granted");
    // A queued deny is recorded, never enforced.
    f.wait_for_outcome("ignored").await;
    let mut published = Vec::new();
    let until = Instant::now() + Duration::from_secs(10);
    while published.len() < 2 && Instant::now() < until {
        for delivery in bus.poll(&outcomes, 16).await.unwrap() {
            if let Delivery::Event { envelope, .. } = delivery {
                published.push(envelope.event.subject.clone());
            }
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert_eq!(
        published,
        vec![format!("plugin/{}/hook/ignored", f.candidate); 2],
        "outcomes are bus activity under the plugin's prefix"
    );

    // Disabled without a reconcile: the stale snapshot still queues the
    // event, and the current authority refuses it.
    f.enable(false);
    f.host
        .notify(&HookEvent::harness("PreToolUse", Some(seat), json!({})));
    f.wait_for_outcome("failed").await;
    // Enabled again (A -> B -> A): the snapshot's generation is superseded.
    f.enable(true);
    f.host
        .notify(&HookEvent::harness("PreToolUse", Some(seat), json!({})));
    assert_eq!(
        f.wait_for_outcome("stale").await.code.as_deref(),
        Some("stale_generation")
    );
    f.enable(false);
    f.reconcile().await;
    assert!(f.host.hook_connections().is_empty());
    f.host
        .notify(&HookEvent::harness("PreToolUse", Some(seat), json!({})));
    bus.publish("account", Draft::new("task/8/status", "test", json!({})))
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(600)).await;
    assert_eq!(
        f.hooks_received().len(),
        2,
        "a disabled plugin receives nothing"
    );
}

// keeps: G3 hooks / PluginHook — a before hook vetoes only with the block
// grant, within its ceiling; timeout follows the grant's FailurePolicy;
// notify never stalls and drops and counts beyond the queue.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn blocking_hooks_veto_only_with_a_grant_and_time_out_by_policy() {
    let hooks = json!([{"event":"PreToolUse","phase":"before","budget_ms":800}]);
    let event = HookEvent::harness("PreToolUse", None, json!({"tool_name":"rm"}));
    for (decision, block, expected, outcome) in [
        ("deny", Some(FailurePolicy::FailOpen), false, "denied"),
        ("deny", None, true, "ignored"),
        (
            "slow",
            Some(FailurePolicy::FailClosed),
            false,
            "failed_closed",
        ),
        ("slow", Some(FailurePolicy::FailOpen), true, "failed_open"),
    ] {
        let f = Fixture::with(Variant::hooks(
            hooks.clone(),
            decision,
            approval(&["PreToolUse"], block),
        ))
        .await;
        let (verdict, elapsed) = f.decide(event.clone()).await;
        assert!(
            elapsed < Duration::from_millis(1_600),
            "{decision} {block:?}: {elapsed:?}"
        );
        assert_eq!(
            verdict == Verdict::Allow,
            expected,
            "{decision} {block:?}: {verdict:?}"
        );
        if let Verdict::Deny { candidate, reason } = &verdict {
            assert_eq!(candidate, &f.candidate);
            assert_eq!(
                reason,
                if decision == "deny" {
                    "fixture veto"
                } else {
                    "hook failed closed: deadline_exceeded"
                }
            );
        }
        let recorded = f.wait_for_outcome(outcome).await;
        if decision == "slow" {
            assert_eq!(recorded.code.as_deref(), Some("deadline_exceeded"));
        }
        if block == Some(FailurePolicy::FailOpen) && decision == "slow" {
            // The slow plugin holds the hook lane: offers beyond the queue
            // are dropped and counted, and notify returns at once.
            let started = Instant::now();
            for _ in 0..HOOK_QUEUE + 8 {
                f.host.notify(&event);
            }
            assert!(started.elapsed() < Duration::from_millis(200));
            assert!(f.host.hook_dropped() >= 7, "{}", f.host.hook_dropped());
        }
    }
}
