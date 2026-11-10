// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Exhaustive refinement of runtime operations, with a volatile-journal control.
#![allow(clippy::disallowed_methods)]
use bboxy450_term_remote::delivery::{BrokenRemoteDelivery, RemoteDelivery as M};
use bboxy450_term_remote::link::{AppliedJournal, Arrival, FileJournal, Inbox, Outbox};
use bbt_yarn::model::{Model, Report};
use std::collections::{BTreeMap, BTreeSet};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

const MESSAGES: [(&str, M::Msg); 2] = [("a", M::Msg::A), ("b", M::Msg::B)];

#[derive(Clone, Copy, Debug)]
enum Op {
    Submit(usize),
    Flush,
    Deliver(usize),
    DeliverApplied(usize),
    Drop,
    Reconnect,
    Crash,
}
const OPS: [Op; 10] = [
    Op::Submit(0),
    Op::Submit(1),
    Op::Flush,
    Op::Deliver(0),
    Op::Deliver(1),
    Op::DeliverApplied(0),
    Op::DeliverApplied(1),
    Op::Drop,
    Op::Reconnect,
    Op::Crash,
];

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Observation {
    unacked: [bool; 2],
    acked: [bool; 2],
    applied: [bool; 2],
    frames: [bool; 2],
    receipts: [bool; 2],
    up: bool,
}

trait Journal: AppliedJournal + Sized {
    fn open(path: &Path) -> io::Result<Self>;
}
impl Journal for FileJournal {
    fn open(path: &Path) -> io::Result<Self> {
        FileJournal::open(path)
    }
}
#[derive(Default)]
struct ForgetfulJournal(BTreeSet<String>);
impl Journal for ForgetfulJournal {
    fn open(_: &Path) -> io::Result<Self> {
        Ok(Self::default())
    }
}
impl AppliedJournal for ForgetfulJournal {
    fn contains(&self, id: &str) -> bool {
        self.0.contains(id)
    }
    fn record(&mut self, id: &str) -> io::Result<()> {
        self.0.insert(id.into());
        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq)]
enum FailureKind {
    Io,
    Decision,
    Apply,
    Applied,
    State,
    Invariant,
}
#[derive(Debug)]
struct Failure {
    kind: FailureKind,
    path: Vec<Op>,
    detail: String,
}
type Checked<T> = Result<T, (FailureKind, String)>;
fn require(ok: bool, kind: FailureKind, detail: impl Into<String>) -> Checked<()> {
    if ok {
        Ok(())
    } else {
        Err((kind, detail.into()))
    }
}
fn io_result<T>(r: io::Result<T>) -> Checked<T> {
    r.map_err(|e| (FailureKind::Io, e.to_string()))
}

struct Harness<J: Journal> {
    out: Outbox<()>,
    inbox: Option<Inbox<J>>,
    dir: PathBuf,
    frames: [bool; 2],
    receipts: [bool; 2],
    up: bool,
    model: M::State,
}
impl<J: Journal> Drop for Harness<J> {
    fn drop(&mut self) {
        drop(self.inbox.take());
        std::fs::remove_dir_all(&self.dir).expect("remove owned journal directory");
    }
}
impl<J: Journal> Harness<J> {
    fn new() -> Checked<Self> {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "bboxy-refinement-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        io_result(std::fs::create_dir(&dir))?;
        let journal = io_result(J::open(&dir.join("applied.log")))?;
        Ok(Self {
            out: Outbox::new(),
            inbox: Some(Inbox::new(journal)),
            dir,
            frames: [false; 2],
            receipts: [false; 2],
            up: true,
            model: M::State::default(),
        })
    }
    fn observation(&self) -> Observation {
        Observation {
            unacked: MESSAGES.map(|(id, _)| self.out.is_unacked(id)),
            acked: MESSAGES.map(|(id, _)| self.out.is_acked(id)),
            applied: MESSAGES.map(|(id, _)| self.inbox.as_ref().unwrap().is_applied(id)),
            frames: self.frames,
            receipts: self.receipts,
            up: self.up,
        }
    }
    fn enabled(&self, op: Op) -> bool {
        match op {
            Op::Deliver(i) => self.frames[i],
            Op::DeliverApplied(i) => self.receipts[i],
            _ => true,
        }
    }
    fn compare(&self) -> Checked<()> {
        let obs = self.observation();
        require(
            obs.applied == MESSAGES.map(|(_, m)| self.model.applied().contains(m)),
            FailureKind::Applied,
            format!("applied: {obs:?} vs {:?}", self.model),
        )?;
        require(
            obs.unacked == MESSAGES.map(|(_, m)| self.model.unacked().contains(m))
                && obs.acked == MESSAGES.map(|(_, m)| self.model.acked().contains(m))
                && obs.frames == MESSAGES.map(|(_, m)| self.model.inflight().contains(m))
                && obs.up == self.model.link_up(),
            FailureKind::State,
            format!("{obs:?} vs {:?}", self.model),
        )?;
        require(
            M::applied_at_most_once(&self.model)
                && M::ack_implies_applied(&self.model)
                && M::acked_leaves_outbox(&self.model),
            FailureKind::Invariant,
            format!("{:?}", self.model),
        )
    }
    fn event(
        &mut self,
        e: M::Event,
        outcome: Outcome,
        apply: bool,
        coverage: &mut Coverage,
    ) -> Checked<()> {
        let before = self.model.model_values();
        let (actual, emitted) = match M::step(&self.model, e) {
            M::Decision::Accepted { next, effects } => {
                self.model = next;
                (Outcome::Accepted, !effects.is_empty())
            }
            M::Decision::Rejected { .. } => (Outcome::Rejected, false),
            M::Decision::Ignored { .. } => (Outcome::Ignored, false),
        };
        require(
            actual == outcome,
            FailureKind::Decision,
            format!("{e:?}: runtime {outcome:?}, model {actual:?}"),
        )?;
        require(
            apply == emitted,
            FailureKind::Apply,
            format!("{e:?}: Fresh={apply}, Apply={emitted}"),
        )?;
        coverage.edge(&before, e, &self.model.model_values());
        Ok(())
    }
    // This is the complete runtime-to-model mapping. The link never forges an
    // Applied frame: only a real Inbox arrival creates one. Frames may repeat.
    fn operate(&mut self, op: Op, coverage: &mut Coverage) -> Checked<()> {
        match op {
            Op::Submit(i) => {
                let outcome = if self.out.submit(MESSAGES[i].0, ()).is_ok() {
                    Outcome::Accepted
                } else {
                    Outcome::Rejected
                };
                self.event(M::Event::Submit(MESSAGES[i].1), outcome, false, coverage)?;
            }
            Op::Flush => {
                // One batch is a sequence of Send events in returned order.
                let sent = self.out.sendable();
                if sent.is_empty() {
                    // Empty flush stutters: every candidate Send is ignored.
                    for (_, m) in MESSAGES {
                        require(
                            matches!(
                                M::step(&self.model, M::Event::Send(m)),
                                M::Decision::Ignored { .. }
                            ),
                            FailureKind::Decision,
                            "empty flush has a model-sendable message",
                        )?;
                    }
                }
                for (id, ()) in sent {
                    let i = MESSAGES.iter().position(|(name, _)| *name == id).unwrap();
                    self.frames[i] = true;
                    self.event(
                        M::Event::Send(MESSAGES[i].1),
                        Outcome::Accepted,
                        false,
                        coverage,
                    )?;
                }
            }
            Op::Deliver(i) => {
                let fresh = io_result(self.inbox.as_mut().unwrap().arrive(MESSAGES[i].0))?
                    == Arrival::Fresh;
                self.receipts[i] = true;
                self.event(
                    M::Event::Arrive(MESSAGES[i].1),
                    if fresh {
                        Outcome::Accepted
                    } else {
                        Outcome::Ignored
                    },
                    fresh,
                    coverage,
                )?;
            }
            Op::DeliverApplied(i) => {
                let accepted = self.out.on_applied(MESSAGES[i].0);
                // Correction: retire the message frame when its ack retires
                // inflight. Retain the receipt so duplicate acks are exercised.
                // Late message frames after retirement are outside this link view.
                if accepted {
                    self.frames[i] = false;
                }
                self.event(
                    M::Event::AckArrive(MESSAGES[i].1),
                    if accepted {
                        Outcome::Accepted
                    } else {
                        Outcome::Ignored
                    },
                    false,
                    coverage,
                )?;
            }
            Op::Drop | Op::Crash => {
                let outcome = if self.up || matches!(op, Op::Crash) {
                    Outcome::Accepted
                } else {
                    Outcome::Ignored
                };
                self.out.drop_link();
                self.frames = [false; 2];
                self.receipts = [false; 2];
                self.up = false;
                if matches!(op, Op::Crash) {
                    drop(self.inbox.take());
                    self.inbox = Some(Inbox::new(io_result(J::open(
                        &self.dir.join("applied.log"),
                    ))?));
                }
                self.event(
                    if matches!(op, Op::Crash) {
                        M::Event::RemoteCrash
                    } else {
                        M::Event::Drop
                    },
                    outcome,
                    false,
                    coverage,
                )?;
            }
            Op::Reconnect => {
                let outcome = if self.up {
                    Outcome::Ignored
                } else {
                    Outcome::Accepted
                };
                self.out.reconnect();
                self.up = true;
                self.event(M::Event::Reconnect, outcome, false, coverage)?;
            }
        }
        self.compare()
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Outcome {
    Accepted,
    Rejected,
    Ignored,
}
struct Coverage {
    model: Model,
    report: Report,
    indices: BTreeMap<Vec<i64>, usize>,
    states: BTreeSet<usize>,
    edges: BTreeSet<(usize, usize, usize)>,
}
impl Coverage {
    fn new() -> Self {
        let model = M::model();
        let report = model.check(100_000).unwrap();
        assert!(report.violations.is_empty());
        let indices = report
            .states
            .iter()
            .cloned()
            .enumerate()
            .map(|(i, s)| (s, i))
            .collect();
        Self {
            model,
            report,
            indices,
            states: BTreeSet::new(),
            edges: BTreeSet::new(),
        }
    }
    fn edge(&mut self, before: &[i64], e: M::Event, after: &[i64]) {
        let from = self.indices[before];
        let to = self.indices[after];
        let edge = self.report.edges[from]
            .iter()
            .find(|edge| M::action_event(edge.action) == Some(e))
            .expect("generated step has a checked graph edge");
        assert_eq!(edge.to, to);
        self.states.extend([from, to]);
        self.edges.insert((from, edge.action, to));
    }
}
#[derive(Debug, PartialEq, Eq)]
struct Counts {
    harness_states: usize,
    harness_transitions: usize,
    model_states: usize,
    total_model_states: usize,
    model_edges: usize,
    total_model_edges: usize,
}
fn walk<J: Journal>() -> Result<Counts, Failure> {
    let mut coverage = Coverage::new();
    let initial = Harness::<J>::new().map_err(|(kind, detail)| Failure {
        kind,
        path: vec![],
        detail,
    })?;
    let mut seen = BTreeMap::from([(initial.observation(), initial.model.model_values())]);
    drop(initial);
    let mut paths = vec![Vec::new()];
    let mut transitions = 0;
    let mut cursor = 0;
    while cursor < paths.len() {
        for op in OPS {
            let mut path = paths[cursor].clone();
            let result = (|| {
                let mut h = Harness::<J>::new()?;
                for &previous in &path {
                    h.operate(previous, &mut coverage)?;
                }
                if !h.enabled(op) {
                    return Ok(None);
                }
                path.push(op);
                h.operate(op, &mut coverage)?;
                Ok(Some((h.observation(), h.model.model_values())))
            })();
            match result {
                Ok(Some((observation, model))) => {
                    transitions += 1;
                    if let Some(previous) = seen.get(&observation) {
                        assert_eq!(
                            previous, &model,
                            "observation quotient must preserve model state"
                        );
                    } else {
                        seen.insert(observation, model);
                        paths.push(path);
                    }
                }
                Ok(None) => (),
                Err((kind, detail)) => return Err(Failure { kind, path, detail }),
            }
        }
        cursor += 1;
    }
    assert_eq!(paths.len(), seen.len());
    let mut unused = BTreeMap::<String, usize>::new();
    for (from, edges) in coverage.report.edges.iter().enumerate() {
        for edge in edges {
            if !coverage.edges.contains(&(from, edge.action, edge.to)) {
                *unused
                    .entry(
                        coverage.model.actions[edge.action]
                            .name
                            .split('(')
                            .next()
                            .unwrap()
                            .split("::")
                            .next()
                            .unwrap()
                            .into(),
                    )
                    .or_default() += 1;
            }
        }
    }
    let counts = Counts {
        harness_states: seen.len(),
        harness_transitions: transitions,
        model_states: coverage.states.len(),
        total_model_states: coverage.report.states.len(),
        model_edges: coverage.edges.len(),
        total_model_edges: coverage.report.edges.iter().map(Vec::len).sum(),
    };
    println!("{counts:?}; unexercised model edges by event: {unused:?}");
    Ok(counts)
}

#[test]
fn runtime_operations_refine_the_checked_model() {
    let counts = walk::<FileJournal>()
        .unwrap_or_else(|e| panic!("{:?}: {:?}: {}", e.path, e.kind, e.detail));
    assert_eq!(
        counts,
        Counts {
            harness_states: 72,
            harness_transitions: 502,
            model_states: 50,
            total_model_states: 52,
            model_edges: 298,
            total_model_edges: 572
        }
    );
}
#[test]
fn volatile_runtime_journal_is_detected() {
    let failure = walk::<ForgetfulJournal>().unwrap_err();
    assert!(
        matches!(failure.kind, FailureKind::Apply | FailureKind::Applied),
        "{failure:?}"
    );
    assert!(
        failure.path.iter().any(|op| matches!(op, Op::Crash)),
        "{failure:?}"
    );
    println!("volatile runtime control: {failure:?}");
}

#[test]
fn exported_model_holds_and_volatile_dedup_control_fails() {
    let pass = M::model().check(100_000).expect("model check runs");
    assert!(pass.violations.is_empty(), "{:?}", pass.violations);
    let broken = BrokenRemoteDelivery::model()
        .check(100_000)
        .expect("control runs");
    assert!(
        broken
            .violations
            .iter()
            .any(|v| v.property.contains("applied_at_most_once")
                || v.property.contains("REMOTE-DELIVERY-001")),
        "volatile applied set must double-apply after crash replay: {:?}",
        broken.violations
    );
}

#[test]
fn resume_model_holds_and_resetless_restart_control_fails() {
    use bboxy450_term_remote::resume_model::{BrokenRemoteResume, RemoteResume};
    let pass = RemoteResume::model()
        .check(100_000)
        .expect("model check runs");
    assert!(pass.violations.is_empty(), "{:?}", pass.violations);
    let broken = BrokenRemoteResume::model()
        .check(100_000)
        .expect("control runs");
    assert!(
        !broken.violations.is_empty(),
        "a restart without reset must leave the view holding events the new log lacks"
    );
}
