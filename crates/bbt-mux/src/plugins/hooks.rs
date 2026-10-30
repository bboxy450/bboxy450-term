// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Hook delivery to plugins (G3).
//!
//! Every enabled, checked plugin whose trusted shape declares `hooks`
//! subscribes to harness events (by name) and, with event `bus`, to bus
//! subjects (by pattern). The host calls the plugin's MCP tool `hook` with
//! `{event, phase, subject?, seat?, budget_ms, payload?}` on the plugin's
//! hook connection, within `min(budget_ms, ceiling_ms)`, and reads
//! `{decision: "allow" | "deny", reason?}`.
//!
//! - [`PluginHost::notify`] never blocks: it offers each matching
//!   subscription to a bounded queue ([`HOOK_QUEUE`]) and drops and counts on
//!   overflow. One worker thread per host drains the queue.
//! - [`PluginHost::decide`] is the blocking form for a `before` event: only a
//!   `before` subscription whose trust record holds a `block` grant is called
//!   synchronously, within its ceiling; its `deny` vetoes, and a timeout or
//!   failure follows the grant's [`FailurePolicy`]. Every other matching
//!   subscription is queued and its reply ignored.
//! - A queued item carries the activation generation it was offered under;
//!   it is dropped as `stale` unless that generation is still current, and a
//!   disabled plugin receives nothing.
//! - The payload is sent only when the trust record grants it
//!   (`payload_events` contains the hook's event; `bus` for bus hooks).
//! - Each outcome is published as `plugin/<candidate>/hook/<outcome>` on the
//!   account stream when a bus is attached, and kept in a short ring.
//!
//! Modelled by `PluginHook` in verification/contracts.toml.
use super::*;
use crate::adapters::hook::{Gate, GateEntry, GatePolicy};
use crate::bus::{Ack, BusHandle, Consumer, Delivery, Start};
use crate::store::events::Draft;
use bbt_plugin::policy::{FailurePolicy, Hook, Phase, subject_matches};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, SyncSender, TrySendError, sync_channel};

/// Hook deliveries queued per root before new ones are dropped and counted.
pub const HOOK_QUEUE: usize = 64;
/// Outcomes kept in memory per root.
const OUTCOMES: usize = 64;
/// Bus subscription poll interval.
const BUS_POLL: Duration = Duration::from_millis(100);
/// Bus patterns one root subscribes to (the bus consumer limit).
const BUS_PATTERNS: usize = 64;

/// One event offered to plugin hooks.
#[derive(Debug, Clone, PartialEq)]
pub struct HookEvent {
    /// The harness event as the harness spelled it, or `bus`.
    pub event: String,
    /// Bus events: the published subject.
    pub subject: Option<String>,
    pub seat: Option<crate::Ulid>,
    /// Sent only to subscriptions holding the payload grant.
    pub payload: Value,
}

impl HookEvent {
    /// A harness hook event (`Command::Hook`).
    pub fn harness(event: &str, seat: Option<crate::Ulid>, payload: Value) -> Self {
        Self {
            event: event.into(),
            subject: None,
            seat,
            payload,
        }
    }
    /// A published bus envelope.
    pub fn bus(subject: &str, seat: Option<crate::Ulid>, payload: Value) -> Self {
        Self {
            event: bbt_plugin::policy::BUS_EVENT.into(),
            subject: Some(subject.into()),
            seat,
            payload,
        }
    }
}

/// The result of [`PluginHost::decide`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    Allow,
    Deny { candidate: String, reason: String },
}

/// One recorded delivery outcome.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct HookOutcome {
    pub candidate: String,
    pub event: String,
    /// `allowed`, `denied`, `ignored` (a reply that cannot decide),
    /// `failed_open`, `failed_closed`, `failed`, `stale` or `dropped`.
    pub outcome: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

/// One plugin's declared hook under the authority it was snapshotted with.
#[derive(Debug, Clone)]
pub(super) struct Subscription {
    candidate: String,
    generation: u64,
    hook: Hook,
    payload: bool,
    block: Option<FailurePolicy>,
}

impl Subscription {
    fn matches(&self, event: &HookEvent) -> bool {
        match (&event.subject, &self.hook.subject) {
            (Some(subject), Some(pattern)) => {
                self.hook.is_bus() && subject_matches(pattern, subject)
            }
            (None, None) => self.hook.matches_event(&event.event),
            _ => false,
        }
    }
    fn blocks(&self) -> Option<FailurePolicy> {
        self.block.filter(|_| self.hook.phase == Phase::Before)
    }
    fn budget(&self) -> Duration {
        let ceiling = self.hook.ceiling_ms().unwrap_or(1_000);
        Duration::from_millis(self.hook.budget_ms.min(ceiling))
    }
}

/// The subscriptions of one trusted record at `generation`.
pub(super) fn subscriptions(
    candidate: &str,
    generation: u64,
    record: &TrustRecord,
) -> Vec<Subscription> {
    let Some(shape) = &record.shape else {
        return Vec::new();
    };
    shape
        .hooks
        .iter()
        .filter(|hook| hook.validate().is_ok())
        .map(|hook| Subscription {
            candidate: candidate.to_owned(),
            generation,
            payload: record.payload_granted(hook)
                && (hook.is_bus() || hook.carries_payload().unwrap_or(false)),
            block: record.grants.block,
            hook: hook.clone(),
        })
        .collect()
}

pub(super) struct Item {
    subscription: Subscription,
    event: HookEvent,
}

pub(super) struct Hooks {
    subscriptions: StdMutex<Arc<Vec<Subscription>>>,
    queue: StdMutex<Option<SyncSender<Item>>>,
    outcomes: StdMutex<VecDeque<HookOutcome>>,
    dropped: AtomicU64,
    bus: StdMutex<Option<BusHandle>>,
    /// Advanced on every resubscription; an older bus task then exits.
    bus_epoch: AtomicU64,
    /// Seat gate files this root keeps current, and the gate last written.
    gates: StdMutex<(BTreeSet<std::path::PathBuf>, Gate)>,
}

impl Hooks {
    pub(super) fn new() -> (Self, Receiver<Item>) {
        let (sender, receiver) = sync_channel(HOOK_QUEUE);
        (
            Self {
                subscriptions: StdMutex::default(),
                queue: StdMutex::new(Some(sender)),
                outcomes: StdMutex::default(),
                dropped: AtomicU64::new(0),
                bus: StdMutex::default(),
                bus_epoch: AtomicU64::new(0),
                gates: StdMutex::default(),
            },
            receiver,
        )
    }
}

fn result_schema() -> Value {
    json!({"type":"object","required":["decision"],"properties":{
        "decision":{"enum":["allow","deny"]},
        "reason":{"type":"string","maxLength":4096}}})
}

impl PluginHost {
    /// One worker thread drains the queue; it holds the host weakly and
    /// stops when the queue's sender is dropped (close or host drop).
    pub(super) fn start_hook_worker(&self, queue: Receiver<Item>) {
        let host = Arc::downgrade(&self.0);
        let spawned = std::thread::Builder::new()
            .name("plugin-hooks".into())
            .spawn(move || {
                while let Ok(item) = queue.recv() {
                    let Some(inner) = host.upgrade() else { break };
                    PluginHost(inner).deliver(item);
                }
            });
        if let Err(reason) = spawned {
            failure("hook_worker_unavailable", reason);
        }
    }

    pub(super) fn set_subscriptions(&self, subscriptions: Vec<Subscription>) {
        let gate = gate(&subscriptions);
        if let Ok(mut held) = self.0.hooks.subscriptions.lock() {
            *held = Arc::new(subscriptions);
        }
        self.resubscribe_bus();
        if let Ok(mut gates) = self.0.hooks.gates.lock()
            && gates.1 != gate
        {
            for path in &gates.0 {
                write_gate(path, &gate);
            }
            gates.1 = gate;
        }
    }

    /// Keep `path` (a seat's `hook-gate.json`) listing the harness events a
    /// blocking `before` subscription of this root decides, so that seat's
    /// hook shim waits for [`PluginHost::decide`] only for those events.
    /// Idempotent; writes only on first registration and on change.
    pub fn gate_seat(&self, path: std::path::PathBuf) {
        if let Ok(mut gates) = self.0.hooks.gates.lock()
            && !gates.0.contains(&path)
        {
            write_gate(&path, &gates.1);
            gates.0.insert(path);
        }
    }

    /// The events this root's blocking subscriptions decide.
    pub fn gate(&self) -> Gate {
        self.0
            .hooks
            .gates
            .lock()
            .map(|gates| gates.1.clone())
            .unwrap_or_default()
    }

    fn matching(&self, event: &HookEvent) -> Vec<Subscription> {
        self.0
            .hooks
            .subscriptions
            .lock()
            .map(|held| held.iter().filter(|s| s.matches(event)).cloned().collect())
            .unwrap_or_default()
    }

    /// Offer `event` to every matching subscription without blocking: the
    /// bounded queue drops and counts on overflow. Safe on the logic thread.
    pub fn notify(&self, event: &HookEvent) {
        for subscription in self.matching(event) {
            self.enqueue(subscription, event);
        }
    }

    fn enqueue(&self, subscription: Subscription, event: &HookEvent) {
        let item = Item {
            subscription,
            event: event.clone(),
        };
        let sent = match self.0.hooks.queue.lock() {
            Ok(queue) => match queue.as_ref() {
                Some(queue) => queue.try_send(item),
                None => Err(TrySendError::Disconnected(item)),
            },
            Err(_) => return,
        };
        if let Err(TrySendError::Full(item) | TrySendError::Disconnected(item)) = sent {
            self.0.hooks.dropped.fetch_add(1, Ordering::Relaxed);
            self.record(&item.subscription, &item.event, "dropped", None);
        }
    }

    /// Decide a `before` event: call each matching `before` subscription
    /// that holds a block grant, within its ceiling and `deadline`; queue
    /// every other match. The first veto wins. Blocking IO; never call it
    /// on the logic or UI thread.
    pub fn decide(&self, event: &HookEvent, deadline: Instant) -> Verdict {
        bbt_rt::assert_not_ui_thread();
        let mut verdict = Verdict::Allow;
        for subscription in self.matching(event) {
            let Some(policy) = subscription.blocks().filter(|_| verdict == Verdict::Allow) else {
                self.enqueue(subscription, event);
                continue;
            };
            let at = deadline.min(Instant::now() + subscription.budget());
            let (outcome, code, deny) = match self.hook_call(&subscription, event, at) {
                Ok(value) if value["decision"] == "deny" => (
                    "denied",
                    None,
                    Some(value["reason"].as_str().unwrap_or("denied").to_owned()),
                ),
                Ok(_) => ("allowed", None, None),
                Err(reason) => match policy {
                    FailurePolicy::FailOpen => ("failed_open", Some(reason.code), None),
                    FailurePolicy::FailClosed => (
                        "failed_closed",
                        Some(reason.code.clone()),
                        Some(format!("hook failed closed: {}", reason.code)),
                    ),
                },
            };
            self.record(&subscription, event, outcome, code);
            if let Some(reason) = deny {
                verdict = Verdict::Deny {
                    candidate: subscription.candidate.clone(),
                    reason,
                };
            }
        }
        verdict
    }

    /// One queued delivery on the worker thread; its reply decides nothing.
    fn deliver(&self, item: Item) {
        let Item {
            subscription,
            event,
        } = item;
        let deadline = Instant::now() + subscription.budget();
        let (outcome, code) = match self.hook_call(&subscription, &event, deadline) {
            // A queued reply is recorded; it never vetoes anything.
            Ok(value) if value["decision"] == "deny" => ("ignored", None),
            Ok(_) => ("allowed", None),
            Err(reason) if reason.code == "stale_generation" => ("stale", Some(reason.code)),
            Err(reason) => ("failed", Some(reason.code)),
        };
        self.record(&subscription, &event, outcome, code);
    }

    /// Call the plugin's `hook` tool on its hook connection, only while the
    /// subscription's activation generation is current.
    fn hook_call(
        &self,
        subscription: &Subscription,
        event: &HookEvent,
        deadline: Instant,
    ) -> d::Result<Value> {
        let candidate = &subscription.candidate;
        let connection = self.connection(candidate, Lane::Hooks)?;
        if connection.authority.activation.generation != subscription.generation
            || !connection.shape().hooks.contains(&subscription.hook)
        {
            return Err(error(
                "stale_generation",
                "the hook was offered under an activation that is no longer current",
            ));
        }
        let budget = subscription.budget();
        let mut args = json!({
            "event": event.event,
            "phase": subscription.hook.phase,
            "budget_ms": budget.as_millis() as u64,
        });
        if let Some(subject) = &event.subject {
            args["subject"] = subject.clone().into();
        }
        if let Some(seat) = event.seat {
            args["seat"] = seat.to_string().into();
        }
        if subscription.payload {
            args["payload"] = event.payload.clone();
        }
        self.dispatch(
            candidate,
            &connection,
            "hook",
            args,
            &[],
            &[(&result_schema(), "invalid_provider_result")],
            budget.as_millis() as u64,
            deadline,
        )
    }

    fn record(
        &self,
        subscription: &Subscription,
        event: &HookEvent,
        outcome: &str,
        code: Option<String>,
    ) {
        let entry = HookOutcome {
            candidate: subscription.candidate.clone(),
            event: event.subject.clone().unwrap_or_else(|| event.event.clone()),
            outcome: outcome.into(),
            code,
        };
        let bus = self.0.hooks.bus.lock().ok().and_then(|bus| bus.clone());
        if let Some(bus) = bus {
            let mut detail = json!(entry);
            detail["dropped_total"] = self.hook_dropped().into();
            let draft = Draft::new(
                format!("plugin/{}/hook/{outcome}", subscription.candidate),
                "plugin-host",
                detail,
            );
            // Activity is best effort; a full router never stalls delivery.
            let _ = bus.try_publish("account", draft);
        }
        if let Ok(mut outcomes) = self.0.hooks.outcomes.lock() {
            if outcomes.len() == OUTCOMES {
                outcomes.pop_front();
            }
            outcomes.push_back(entry);
        }
    }

    /// Recent delivery outcomes, oldest first.
    pub fn hook_outcomes(&self) -> Vec<HookOutcome> {
        self.0
            .hooks
            .outcomes
            .lock()
            .map(|outcomes| outcomes.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Deliveries dropped because the queue was full or closed, plus bus gaps.
    pub fn hook_dropped(&self) -> u64 {
        self.0.hooks.dropped.load(Ordering::Relaxed)
    }

    /// The attached bus, if any.
    pub(super) fn hooks_bus(&self) -> Option<BusHandle> {
        self.0.hooks.bus.lock().ok().and_then(|bus| bus.clone())
    }

    /// Record outcomes on `bus` and deliver its events to bus hooks.
    pub fn attach_bus(&self, bus: BusHandle) {
        if let Ok(mut held) = self.0.hooks.bus.lock() {
            *held = Some(bus);
        }
        self.resubscribe_bus();
    }

    /// Replace the bus consumer with one for the current bus patterns.
    fn resubscribe_bus(&self) {
        let epoch = self.0.hooks.bus_epoch.fetch_add(1, Ordering::SeqCst) + 1;
        let Some(bus) = self.0.hooks.bus.lock().ok().and_then(|bus| bus.clone()) else {
            return;
        };
        let patterns: BTreeSet<String> = self
            .0
            .hooks
            .subscriptions
            .lock()
            .map(|held| {
                held.iter()
                    .filter_map(|s| s.hook.subject.clone())
                    .take(BUS_PATTERNS)
                    .collect()
            })
            .unwrap_or_default();
        if patterns.is_empty() {
            return;
        }
        let host = Arc::downgrade(&self.0);
        drop(self.0.handle.spawn(async move {
            let consumer = Consumer {
                name: None,
                filter: patterns.into_iter().collect(),
                start: Start::New,
                ack: Ack::None,
                deliver: "plugin-hooks".into(),
                queue_max: 256,
                ack_wait_ms: 30_000,
                max_deliver: 1,
            };
            let Ok(id) = bus.subscribe(consumer).await else {
                return;
            };
            loop {
                let current = host
                    .upgrade()
                    .filter(|inner| inner.hooks.bus_epoch.load(Ordering::SeqCst) == epoch);
                let Some(inner) = current else { break };
                let Ok(deliveries) = bus.poll(&id, 64).await else {
                    break;
                };
                let host = PluginHost(inner);
                for delivery in deliveries {
                    match delivery {
                        Delivery::Event { envelope, .. } => {
                            let event = &envelope.event;
                            host.notify(&HookEvent::bus(
                                &event.subject,
                                event.seat,
                                event.payload.clone(),
                            ));
                        }
                        Delivery::Gap { first, last, .. } => {
                            let lost = last.saturating_sub(first) + 1;
                            host.0.hooks.dropped.fetch_add(lost, Ordering::Relaxed);
                        }
                    }
                }
                drop(host);
                tokio::time::sleep(BUS_POLL).await;
            }
            let _ = bus.unsubscribe(&id).await;
        }));
    }

    /// Stop hook delivery: new events are dropped, items already queued are
    /// refused by the closed host, and the worker and bus consumer exit.
    pub(super) fn stop_hooks(&self) {
        if let Ok(mut queue) = self.0.hooks.queue.lock() {
            queue.take();
        }
        if let Ok(mut bus) = self.0.hooks.bus.lock() {
            bus.take();
        }
        self.0.hooks.bus_epoch.fetch_add(1, Ordering::SeqCst);
        // A closed root decides nothing: its seats return to the fast path.
        if let Ok(mut gates) = self.0.hooks.gates.lock() {
            let (paths, _) = std::mem::take(&mut *gates);
            for path in paths {
                write_gate(&path, &Gate::default());
            }
        }
    }
}

/// Every harness spelling of each event a blocking `before` subscription
/// decides, with the strictest failure policy and the smallest ceiling.
fn gate(subscriptions: &[Subscription]) -> Gate {
    let mut gate = Gate::default();
    for subscription in subscriptions {
        let (Some(policy), Ok(ceiling_ms)) =
            (subscription.blocks(), subscription.hook.ceiling_ms())
        else {
            continue;
        };
        let policy = match policy {
            FailurePolicy::FailOpen => GatePolicy::FailOpen,
            FailurePolicy::FailClosed => GatePolicy::FailClosed,
        };
        for name in crate::adapters::hook::EVENTS {
            if subscription.hook.matches_event(name) {
                let entry = gate
                    .events
                    .entry(name.into())
                    .or_insert(GateEntry { policy, ceiling_ms });
                if policy == GatePolicy::FailClosed {
                    entry.policy = policy;
                }
                entry.ceiling_ms = entry.ceiling_ms.min(ceiling_ms);
            }
        }
    }
    gate
}

/// Replace `path` atomically with `gate`, or remove it when nothing blocks.
/// A missing seat directory is skipped: that seat has no shim to gate.
fn write_gate(path: &std::path::Path, gate: &Gate) {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    if gate.events.is_empty() {
        if let Err(error) = std::fs::remove_file(path)
            && error.kind() != std::io::ErrorKind::NotFound
        {
            failure("hook_gate_unwritable", error);
        }
        return;
    }
    if !path.parent().is_some_and(std::path::Path::is_dir) {
        return;
    }
    let temporary = path.with_extension("json.tmp");
    let written = serde_json::to_vec(gate)
        .map_err(std::io::Error::other)
        .and_then(|bytes| {
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .mode(0o600)
                .open(&temporary)?;
            file.write_all(&bytes)?;
            std::fs::rename(&temporary, path)
        });
    if let Err(error) = written {
        failure("hook_gate_unwritable", error);
    }
}
