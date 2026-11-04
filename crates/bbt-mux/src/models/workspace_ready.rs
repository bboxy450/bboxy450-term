// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Document readiness under continuing internal activity output, not OS timing.
use super::*;

pub(super) fn workspace_ready(broken: bool) -> Model {
    let mut m = model(
        "WorkspaceReady",
        broken,
        &[
            ("scanning", 1, 1),
            ("dirty", 1, 0),
            ("document_edit", 1, 0),
            ("pending", 1, 1),
            ("written", 1, 0),
        ],
    );
    // Unbounded internal log writes can interleave with every owner/IO action.
    // Only the broken control classifies those bytes as a document edit.
    action(
        &mut m,
        "ActivityOutput",
        Expr::Bool(true),
        &[(1, if broken { Int(1) } else { Var(1) })],
    );
    action(
        &mut m,
        "DocumentEdit",
        eq(2, 0),
        &[(1, Int(1)), (2, Int(1))],
    );
    action(&mut m, "CompleteScan", eq(0, 1), &[(0, Int(0))]);
    action(
        &mut m,
        "ReconcileDirty",
        all([eq(0, 0), eq(1, 1)]),
        &[(0, Int(1)), (1, Int(0))],
    );
    action(
        &mut m,
        "AdmitWrite",
        all([eq(0, 0), eq(1, 0), eq(3, 1)]),
        &[(3, Int(0)), (4, Int(1))],
    );
    // IO completes and the owner services continuously enabled scan/admission
    // work. Neither activity output nor the one external edit is fair.
    m.weak_fair.extend([vec![2], vec![3], vec![4]]);
    m.responses.push(Response {
        name: "PendingWriteEventuallyAdmitted".into(),
        request: eq(3, 1),
        response: eq(4, 1),
    });
    m
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{collections::BTreeSet, path::Path};

    // Adapted from bboxy450-term tests/formal_contracts.rs, same project:
    // https://github.com/bboxy450/bboxy450-term/blob/main/crates/bboxy450-term/tests/formal_contracts.rs
    // Focused export uses the same graph/parity format without running the corpus.
    #[test]
    fn workspace_ready_model_and_broken_control() {
        let output = std::env::var_os("BBOXY_WORKSPACE_READY_MODELS");
        let mut manifest = String::new();
        for (m, expected) in bboxy450_tlc::build::family(workspace_ready) {
            let checked = m.check(1000).unwrap();
            assert_eq!(checked.violations.is_empty(), expected == "pass");
            if expected == "liveness" {
                assert!(checked.violations.iter().any(|v| !v.cycle.is_empty()));
            }
            let shipped = Path::new(env!("CARGO_MANIFEST_DIR")).join("models");
            for (name, bytes) in bboxy450_tlc::build::artifacts(&m).unwrap() {
                let path = shipped.join(name);
                if std::env::var_os("BBOXY_UPDATE_MODELS").is_some() {
                    std::fs::write(&path, &bytes).unwrap();
                }
                assert_eq!(std::fs::read_to_string(path).unwrap(), bytes);
            }
            let Some(root) = output.as_ref().map(Path::new) else {
                continue;
            };
            std::fs::create_dir_all(root).unwrap();
            for (name, bytes) in bboxy450_tlc::build::artifacts(&m).unwrap() {
                std::fs::write(root.join(name), bytes).unwrap();
            }
            std::fs::write(root.join(format!("{}Graph.cfg", m.name)), m.config(true)).unwrap();
            let states: BTreeSet<_> = checked
                .states
                .iter()
                .map(|state| {
                    state
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                        .join(",")
                })
                .collect();
            std::fs::write(
                root.join(format!("{}.states", m.name)),
                states.into_iter().collect::<Vec<_>>().join("\n") + "\n",
            )
            .unwrap();
            let mut edges = BTreeSet::new();
            for (i, outgoing) in checked.edges.iter().enumerate() {
                for edge in outgoing {
                    edges.insert(format!(
                        "{:?}\t{}\t{:?}",
                        checked.states[i], edge.action, checked.states[edge.to]
                    ));
                }
            }
            std::fs::write(
                root.join(format!("{}.edges", m.name)),
                edges.into_iter().collect::<Vec<_>>().join("\n") + "\n",
            )
            .unwrap();
            std::fs::write(
                root.join(format!("{}.native-trace", m.name)),
                format!("{:?}", checked.violations),
            )
            .unwrap();
            manifest += &format!(
                "{}\t{expected}\t{}\t{}\n",
                m.name,
                checked.states.len(),
                m.variables.len()
            );
        }
        if let Some(root) = output {
            std::fs::write(Path::new(&root).join("manifest.tsv"), manifest).unwrap();
        }
    }
}
