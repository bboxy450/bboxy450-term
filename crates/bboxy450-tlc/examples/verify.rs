// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Export the bounded differential corpus. Java TLC remains the reference.
// Standalone development tool, outside the UI; depending on bbt-rt would invert layering.
#![allow(clippy::disallowed_methods)]
use Expr::{Bool as B, Int as I, Var as V};
use bboxy450_tlc::{Action, Expr, Invariant, Model, Response, Variable};
use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::path::Path;

fn counter(name: &str, bad: bool, deadlock: bool) -> Model {
    let mut m = Model::new(name, vec![Variable::new("counter", 0, 2, 0)]);
    m.actions.push(Action::new(
        "increment",
        V(0).lt(I(2)),
        vec![(0, V(0).plus(I(1)))],
    ));
    if !deadlock {
        m.actions.push(Action::new(
            "decrement",
            I(0).lt(V(0)),
            vec![(0, V(0).minus(I(1)))],
        ));
    }
    m.invariants.push(Invariant {
        name: "counter_bound".into(),
        condition: V(0).le(I(if bad { 1 } else { 2 })),
    });
    m
}
fn response(name: &str, fair: bool, flicker: bool) -> Model {
    let mut m = Model::new(
        name,
        vec![
            Variable::new("done", 0, 1, 0),
            Variable::new("tick", 0, 1, 0),
        ],
    );
    let mut guard = V(0).eq(I(0));
    if flicker {
        guard = guard.and(V(1).eq(I(0)));
    }
    m.actions.push(Action::new("serve", guard, vec![(0, I(1))]));
    m.actions
        .push(Action::new("busy", B(true), vec![(1, I(1).minus(V(1)))]));
    if fair {
        m.weak_fair.push(vec![0]);
    }
    m.responses.push(Response {
        name: "request_is_served".into(),
        request: B(true),
        response: V(0).eq(I(1)),
    });
    m
}
fn checkpoint(name: &str, bad: bool) -> Model {
    // Contract: bbt-mux/src/checkpoint.rs, Saves::start/complete. Two generations,
    // one save and two cursor steps; no disk durability or parser-byte claim.
    let mut m = Model::new(
        name,
        vec![
            Variable::new("generation", 0, 1, 0),
            Variable::new("parsed", 0, 2, 0),
            Variable::new("confirmed", 0, 2, 0),
            Variable::new("inflight", 0, 1, 0),
            Variable::new("save_generation", 0, 1, 0),
            Variable::new("save_cursor", 0, 2, 0),
        ],
    );
    m.actions.push(Action::new(
        "parse",
        V(1).lt(I(2)),
        vec![(1, V(1).plus(I(1)))],
    ));
    m.actions.push(Action::new(
        "submit",
        V(3).eq(I(0)),
        vec![(3, I(1)), (4, V(0)), (5, V(1))],
    ));
    m.actions.push(Action::new(
        "replace_generation",
        V(0).eq(I(0)),
        vec![(0, I(1)), (1, I(0)), (2, I(0))],
    ));
    let guard = if bad {
        V(3).eq(I(1))
    } else {
        V(3).eq(I(1)).and(V(4).eq(V(0)))
    };
    m.actions
        .push(Action::new("confirm", guard, vec![(2, V(5)), (3, I(0))]));
    m.actions.push(Action::new(
        "discard_stale",
        V(3).eq(I(1)).and(V(4).eq(V(0)).negate()),
        vec![(3, I(0))],
    ));
    m.invariants.push(Invariant {
        name: "confirmed_never_ahead_of_current_parser".into(),
        condition: V(2).le(V(1)),
    });
    m
}
fn input(name: &str, bad: bool) -> Model {
    // Contract: supervisor lease fencing + accept_input stable-id deduplication.
    // One operation, two leases. Unknown outcome does not become a new input id.
    let mut m = Model::new(
        name,
        vec![
            Variable::new("lease", 0, 1, 0),
            Variable::new("sender_lease", 0, 1, 0),
            Variable::new("pending", 0, 1, 0),
            Variable::new("applied", 0, 1, 0),
            Variable::new("writes", 0, 2, 0),
            Variable::new("ack_lost", 0, 1, 0),
        ],
    );
    m.actions.push(Action::new(
        "send",
        V(2).eq(I(0)).and(V(3).eq(I(0))),
        vec![(2, I(1)), (1, V(0))],
    ));
    m.actions
        .push(Action::new("replace_lease", V(0).eq(I(0)), vec![(0, I(1))]));
    m.actions.push(Action::new(
        "write",
        V(2).eq(I(1)).and(V(1).eq(V(0))).and(V(3).eq(I(0))),
        vec![(2, I(0)), (3, I(1)), (4, V(4).plus(I(1)))],
    ));
    m.actions.push(Action::new(
        "fence_stale",
        V(2).eq(I(1)).and(V(1).eq(V(0)).negate()),
        vec![(2, I(0))],
    ));
    m.actions
        .push(Action::new("lose_ack", V(3).eq(I(1)), vec![(5, I(1))]));
    m.actions.push(Action::new(
        "query_original_id",
        V(5).eq(I(1)),
        vec![(5, I(0))],
    ));
    if bad {
        m.actions.push(Action::new(
            "unsafe_retry",
            V(5).eq(I(1)).and(V(4).lt(I(2))),
            vec![(4, V(4).plus(I(1)))],
        ));
    }
    m.check_deadlock = false;
    m.invariants.push(Invariant {
        name: "input_applied_at_most_once".into(),
        condition: V(4).le(I(1)),
    });
    m
}
fn snapshot() -> Model {
    let mut m = Model::new(
        "SimultaneousUpdates",
        vec![
            Variable::new("left", 0, 1, 0),
            Variable::new("right", 0, 1, 1),
        ],
    );
    m.actions
        .push(Action::new("swap", B(true), vec![(0, V(1)), (1, V(0))]));
    m.invariants.push(Invariant {
        name: "swap_uses_prestate".into(),
        condition: V(0).plus(V(1)).eq(I(1)),
    });
    m
}
fn stutter() -> Model {
    let mut m = Model::new(
        "StutteringIsNotFairProgress",
        vec![Variable::new("done", 0, 1, 0)],
    );
    m.actions.push(Action::new("noop", B(true), vec![]));
    m.weak_fair.push(vec![0]);
    m.responses.push(Response {
        name: "eventually_done".into(),
        request: B(true),
        response: V(0).eq(I(1)),
    });
    m
}
fn export(
    m: Model,
    expected: &str,
    root: &Path,
    manifest: &mut String,
) -> Result<(), Box<dyn std::error::Error>> {
    let r = m.check(100_000)?;
    let actual = if r.violations.is_empty() {
        "pass"
    } else if r.violations.iter().any(|v| !v.cycle.is_empty()) {
        "liveness"
    } else if r.violations.iter().any(|v| v.property == "Deadlock") {
        "deadlock"
    } else {
        "safety"
    };
    if actual != expected {
        return Err(format!("{}: native expected {expected}, got {actual}", m.name).into());
    }
    std::fs::write(root.join(format!("{}.tla", m.name)), m.tla()?)?;
    std::fs::write(root.join(format!("{}.cfg", m.name)), m.config(false))?;
    std::fs::write(root.join(format!("{}Graph.cfg", m.name)), m.config(true))?;
    let states: BTreeSet<_> = r.states.iter().cloned().collect();
    let mut rows = String::new();
    for state in states {
        writeln!(
            &mut rows,
            "{}",
            state
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(",")
        )?;
    }
    std::fs::write(root.join(format!("{}.states", m.name)), rows)?;
    let mut transitions = BTreeSet::new();
    for (i, edges) in r.edges.iter().enumerate() {
        for edge in edges {
            transitions.insert(format!(
                "{:?}\t{}\t{:?}",
                r.states[i], edge.action, r.states[edge.to]
            ));
        }
    }
    std::fs::write(
        root.join(format!("{}.edges", m.name)),
        transitions.into_iter().collect::<Vec<_>>().join("\n") + "\n",
    )?;
    let mut trace = String::new();
    for v in &r.violations {
        writeln!(
            &mut trace,
            "{} prefix={:?} cycle={:?}",
            v.property,
            v.prefix.iter().map(|i| &r.states[*i]).collect::<Vec<_>>(),
            v.cycle.iter().map(|i| &r.states[*i]).collect::<Vec<_>>()
        )?;
    }
    std::fs::write(root.join(format!("{}.native-trace", m.name)), trace)?;
    writeln!(
        manifest,
        "{}\t{}\t{}\t{}",
        m.name,
        expected,
        r.states.len(),
        m.variables.len()
    )?;
    Ok(())
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = std::env::args()
        .nth(1)
        .ok_or("usage: verify OUTPUT_DIRECTORY")?;
    let root = Path::new(&output);
    std::fs::create_dir_all(root)?;
    let mut manifest = String::new();
    for (m, expected) in [
        (counter("Counter", false, false), "pass"),
        (counter("BrokenSafety", true, false), "safety"),
        (counter("Deadlock", false, true), "deadlock"),
        (response("FairResponse", true, false), "pass"),
        (response("UnfairResponse", false, false), "liveness"),
        (response("WeakFairFlicker", true, true), "liveness"),
        (checkpoint("CheckpointFence", false), "pass"),
        (checkpoint("BrokenCheckpointFence", true), "safety"),
        (input("InputReceipt", false), "pass"),
        (input("BrokenInputRetry", true), "safety"),
        (snapshot(), "pass"),
        (stutter(), "liveness"),
    ] {
        export(m, expected, root, &mut manifest)?;
    }
    std::fs::write(root.join("manifest.tsv"), manifest)?;
    Ok(())
}
