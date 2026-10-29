// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! All project contract projections run during ordinary Rust tests; Make also exports them to TLC.
#![allow(
    clippy::disallowed_methods,
    reason = "unattended verification IO, never a UI turn"
)]
#[path = "formal_contracts/models.rs"]
mod models;
#[path = "formal_contracts/witnesses.rs"]
mod witnesses;
use bboxy450_tlc::Model;
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

#[test]
fn project_contracts_and_broken_controls() -> Result<(), Box<dyn std::error::Error>> {
    let output = std::env::var_os("BBOXY_FORMAL_OUTPUT");
    let mut manifest = if let Some(root) = &output {
        std::fs::read_to_string(Path::new(root).join("manifest.tsv"))?
    } else {
        String::new()
    };
    for (model, expected) in models::cases() {
        // Export into a private temporary directory even without a TLC export request,
        // so ordinary workspace tests exercise native assertions and the complete exporter.
        let temp = tempfile::tempdir()?;
        let root = output.as_ref().map_or(temp.path(), Path::new);
        export(model.clone(), expected, root, &mut manifest)?;
        if output.is_some() {
            let metadata = serde_json::json!({
                "variables": model.variables.iter().map(|v| serde_json::json!({
                    "name": v.name, "min": v.min, "max": v.max, "initial": v.initial
                })).collect::<Vec<_>>(),
                "actions": model.actions.iter().map(|a| &a.name).collect::<Vec<_>>(),
                "invariants": model.invariants.iter().map(|p| &p.name).collect::<Vec<_>>(),
                "responses": model.responses.iter().map(|p| &p.name).collect::<Vec<_>>(),
                "weak_fair_action_groups": model.weak_fair,
                "check_deadlock": model.check_deadlock,
                "implementation_refinement_proven": false
            });
            std::fs::write(
                root.join(format!("{}.metadata.json", model.name)),
                serde_json::to_vec_pretty(&metadata)?,
            )?;
        }
    }
    if let Some(root) = output {
        std::fs::write(Path::new(&root).join("manifest.tsv"), manifest)?;
    }
    Ok(())
}
