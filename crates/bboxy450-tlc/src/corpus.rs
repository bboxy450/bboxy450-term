// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! The differential corpus artifact format, shared by every frontend.
// A development-time exporter run by `make test-formal`; no UI thread exists here.
#![allow(clippy::disallowed_methods)]
use crate::Model;
use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::path::Path;
use std::string::{String, ToString};
use std::vec::Vec;
use std::{boxed::Box, format, writeln};

fn json_string(value: &str) -> String {
    let mut result = String::from("\"");
    for c in value.chars() {
        match c {
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            c if c < ' ' => write!(&mut result, "\\u{:04x}", u32::from(c)).unwrap(),
            c => result.push(c),
        }
    }
    result.push('"');
    result
}

fn metadata(model: &Model) -> String {
    let variables = model
        .variables
        .iter()
        .map(|v| {
            format!(
                "{{\"name\":{},\"min\":{},\"max\":{},\"initial\":{}}}",
                json_string(&v.name),
                v.min,
                v.max,
                v.initial,
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    let actions = model
        .actions
        .iter()
        .map(|a| json_string(&a.name))
        .collect::<Vec<_>>()
        .join(",");
    let invariants = model
        .invariants
        .iter()
        .map(|i| json_string(&i.name))
        .collect::<Vec<_>>()
        .join(",");
    let responses = model
        .responses
        .iter()
        .map(|r| json_string(&r.name))
        .collect::<Vec<_>>()
        .join(",");
    let fair = &model.weak_fair;
    let strong_fair = &model.strong_fair;
    let deadlock = model.check_deadlock;
    format!(
        "{{\"variables\":[{variables}],\"actions\":[{actions}],\"invariants\":[{invariants}],\"responses\":[{responses}],\"weak_fair_action_groups\":{fair:?},\"strong_fair_action_groups\":{strong_fair:?},\"check_deadlock\":{deadlock},\"implementation_refinement_proven\":false}}\n"
    )
}

/// Checks `m` natively, requires the `expected` verdict (`pass`, `safety`,
/// `deadlock` or `liveness`), and writes the artifacts the Java TLC parity gate
/// reads into `root`: TLA+ text, both configurations, exact states and edges,
/// the native counterexamples and the model's metadata. Appends one row to
/// `manifest`. Every frontend exports through this one function.
pub fn export(
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
    std::fs::write(root.join(format!("{}.metadata.json", m.name)), metadata(&m))?;
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
