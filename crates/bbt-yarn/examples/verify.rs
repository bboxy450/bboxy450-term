// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Export the bounded differential corpus. Java TLC remains the reference.
// Standalone development tool, outside the UI; depending on bbt-rt would invert layering.
#![allow(clippy::disallowed_methods)]
use bbt_yarn::model::Model;
use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::path::Path;

// Copied from bboxy450-tlc examples/verify.rs, within the same source project.
// https://github.com/bboxy450/bboxy450-tlc — artifact format shared by both frontends.
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
    let mut manifest = std::fs::read_to_string(root.join("manifest.tsv"))?;
    let reactor = bbt_yarn::reactor_model! {
        policy {selection: biased;required_phases: [];}
        #[source(stop)] #[readiness(quiescent)] #[shutdown]
        _ = sources.stop => { Ok(()) }
        #[source(input)] #[readiness(may_remain_ready)]
        #[starvation(allowed, reason="test corpus explores source scheduling, not handler termination")]
        _ = sources.input => { Ok(Control::Continue) }
        #[source(output)] #[readiness(may_remain_ready)] #[last] #[drain(max=2)]
        #[starvation(allowed, reason="background output may wait behind operator input")]
        _ = sources.output => { Ok(Control::Continue) }
    };
    let mut yield_reactor = bbt_yarn::reactor_model! {
        policy {selection: biased;required_phases: [];}
        #[source(stop)] #[readiness(quiescent)] #[shutdown]
        _ = sources.stop => { Ok(()) }
        #[source(output)] #[readiness(may_remain_ready)] #[yields_to(input,when=buffered)] #[drain(max=2)]
        _ = sources.output => { Ok(Control::Continue) }
        #[source(input)] #[readiness(may_remain_ready)] #[last] #[when(enabled)]
        _ = sources.input => { Ok(Control::Continue) }
    };
    yield_reactor.name = "YarnBufferedYield".into();
    for model in [reactor, yield_reactor] {
        export(model, "pass", root, &mut manifest)?;
    }
    std::fs::write(root.join("manifest.tsv"), manifest)?;
    Ok(())
}
