// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Finite projections of workspace admission, publication, writer and cleanup.
use super::*;
pub(super) fn managed_workspace_create(b: bool) -> Model {
    let mut m = model(
        "ManagedWorkspaceCreate",
        b,
        &[("reserved", 1, 0), ("resources", 2, 0), ("requests", 2, 0)],
    );
    action(
        &mut m,
        "Prepare",
        all([
            Var(2).lt(Int(2)),
            if b { Expr::Bool(true) } else { eq(0, 0) },
        ]),
        &[
            (0, Int(1)),
            (1, Var(1).plus(Int(1))),
            (2, Var(2).plus(Int(1))),
        ],
    );
    action(
        &mut m,
        "Retry",
        all([eq(0, 1), Var(2).lt(Int(2))]),
        &[(2, Var(2).plus(Int(1)))],
    );
    safe(&mut m, "OneResourcePerAttempt", Var(1).le(Int(1)));
    m
}
pub(super) fn managed_workspace_writer(b: bool) -> Model {
    workspace_writer(b, false)
}
pub(super) fn broken_managed_workspace_writer_probe() -> Model {
    workspace_writer(false, true)
}
fn workspace_writer(retain_rejected_writer: bool, skip_probe: bool) -> Model {
    let mut m = model(
        "ManagedWorkspaceWriter",
        retain_rejected_writer || skip_probe,
        &[
            ("holders", 2, 0),
            ("launches", 2, 0),
            ("requested", 1, 0),
            ("outcome", 3, 0),
            ("fresh", 1, 1),
            ("probe", 2, 0),
        ],
    );
    if skip_probe {
        m.name.push_str("Probe");
    }
    // Outcome: pending (0), rejected (1), unknown (2), applied (3).
    // Probe: pending (0), success/non-LinkDown fallthrough (1), direct LinkDown (2).
    action(
        &mut m,
        "ReserveWriter",
        all([Var(1).lt(Int(2)), eq(0, 0)]),
        &[
            (0, Int(1)),
            (1, Var(1).plus(Int(1))),
            (3, Int(0)),
            (4, Int(1)),
            (5, Int(0)),
        ],
    );
    action(
        &mut m,
        "ProbeAllowsRequest",
        all([eq(0, 1), eq(2, 0), eq(4, 1), eq(5, 0)]),
        &[(5, Int(1))],
    );
    action(
        &mut m,
        "ProbeLinkDown",
        all([eq(0, 1), eq(2, 0), eq(4, 1), eq(5, 0)]),
        &[(5, Int(2))],
    );
    action(
        &mut m,
        "RequestLaunch",
        all([
            eq(0, 1),
            eq(2, 0),
            if skip_probe {
                Expr::Bool(true)
            } else {
                eq(5, 1)
            },
        ]),
        &[(2, Int(1)), (4, Int(0))],
    );
    action(
        &mut m,
        "RejectBeforeRequest",
        all([eq(0, 1), eq(2, 0)]),
        &[
            (0, Int(if retain_rejected_writer { 1 } else { 0 })),
            (3, Int(1)),
        ],
    );
    action(
        &mut m,
        "SupervisorRefuses",
        all([eq(2, 1), eq(3, 0)]),
        &[(0, Int(0)), (2, Int(0)), (3, Int(1))],
    );
    // Includes a direct LinkDown after a real Spawn applied: its type proves no phase.
    action(
        &mut m,
        "LoseReply",
        all([eq(2, 1), eq(3, 0)]),
        &[(3, Int(2))],
    );
    action(
        &mut m,
        "RetryRetainedWithLinkDown",
        all([eq(2, 1), eq(3, 2), eq(4, 0)]),
        &[(3, Int(2))],
    );
    action(
        &mut m,
        "ConfirmProcess",
        all([eq(2, 1), ne(3, 3)]),
        &[(3, Int(3))],
    );
    action(
        &mut m,
        "Detach",
        eq(3, 3),
        &[(0, Int(0)), (2, Int(0)), (3, Int(0))],
    );
    safe(&mut m, "SingleWriter", Var(0).le(Int(1)));
    safe(
        &mut m,
        "RejectedReleasesWriter",
        implies(eq(3, 1), all([eq(0, 0), eq(2, 0)])),
    );
    safe(
        &mut m,
        "RequestPassedProbe",
        implies(eq(2, 1), all([eq(5, 1), eq(4, 0)])),
    );
    safe(
        &mut m,
        "UnknownRequiresRequest",
        implies(eq(3, 2), all([eq(0, 1), eq(2, 1), eq(4, 0)])),
    );
    m
}
pub(super) fn managed_workspace_fence(b: bool) -> Model {
    let mut m = model(
        "ManagedWorkspaceFence",
        b,
        &[
            ("revision", 2, 0),
            ("captured", 2, 0),
            ("pending", 1, 0),
            ("started", 1, 0),
            ("bad", 1, 0),
        ],
    );
    action(
        &mut m,
        "Start",
        eq(3, 0),
        &[(1, Var(0)), (2, Int(1)), (3, Int(1))],
    );
    action(
        &mut m,
        "ChangeRootOrGeneration",
        Var(0).lt(Int(2)),
        &[(0, Var(0).plus(Int(1)))],
    );
    let stale = Expr::Not(Box::new(Var(0).eq(Var(1))));
    action(
        &mut m,
        "Complete",
        eq(2, 1),
        &[
            (2, Int(0)),
            (4, if b { stale.ite(Int(1), Var(4)) } else { Var(4) }),
        ],
    );
    no_bad(&mut m, 4);
    m.weak_fair.push(vec![2]);
    m.responses.push(Response {
        name: "ProviderEventuallyObserved".into(),
        request: eq(2, 1),
        response: eq(2, 0),
    });
    m
}
pub(super) fn managed_workspace_release(b: bool) -> Model {
    let mut m = model(
        "ManagedWorkspaceRelease",
        b,
        &[
            ("private", 1, 1),
            ("holder", 1, 1),
            ("deleted", 1, 0),
            ("bad", 1, 0),
            ("preserved", 1, 0),
            ("build", 1, 0),
            ("external_handle", 1, 0),
        ],
    );
    action(&mut m, "PreservePrivateData", eq(0, 1), &[(0, Int(0))]);
    action(&mut m, "PreserveSource", eq(4, 0), &[(4, Int(1))]);
    action(&mut m, "Build", all([eq(1, 1), eq(5, 0)]), &[(5, Int(1))]);
    action(&mut m, "CloseAndVerifyDead", eq(1, 1), &[(1, Int(0))]);
    action(
        &mut m,
        "OpenExternalHandle",
        all([eq(2, 0), eq(6, 0)]),
        &[(6, Int(1))],
    );
    action(&mut m, "CloseExternalHandle", eq(6, 1), &[(6, Int(0))]);
    let preserved = all([eq(0, 0), eq(4, 1)]);
    action(
        &mut m,
        "Release",
        all([
            eq(1, 0),
            eq(2, 0),
            eq(6, 0),
            if b {
                Expr::Bool(true)
            } else {
                preserved.clone()
            },
        ]),
        &[(2, Int(1)), (3, preserved.ite(Var(3), Int(1))), (5, Int(0))],
    );
    no_bad(&mut m, 3);
    safe(
        &mut m,
        "DeletedBuildHasNoOwner",
        implies(eq(2, 1), all([eq(5, 0), eq(6, 0)])),
    );
    m
}
pub(super) fn managed_workspace_ownership(b: bool) -> Model {
    let mut m = model(
        "ManagedWorkspaceOwnership",
        b,
        &[
            ("owned_identity", 1, 1),
            ("deleted", 1, 0),
            ("bad", 1, 0),
            ("historical_lease_current", 1, 1),
            ("history_restored", 1, 0),
            ("restore_failed", 1, 0),
        ],
    );
    action(&mut m, "ReplacePath", eq(0, 1), &[(0, Int(0))]);
    action(
        &mut m,
        "DeleteExactResource",
        all([eq(1, 0), if b { Expr::Bool(true) } else { eq(0, 1) }]),
        &[(1, Int(1)), (2, eq(0, 0).ite(Int(1), Var(2)))],
    );
    action(
        &mut m,
        "ReassignHistoricalWorkspace",
        eq(3, 1),
        &[(3, Int(0))],
    );
    // A retained exited process is observed as history, without confirming a
    // writer lease. The broken control attempts the stale launch confirmation.
    action(
        &mut m,
        "RestoreRetainedExit",
        eq(4, 0),
        &[
            (4, Int(1)),
            (
                5,
                if b {
                    eq(3, 0).ite(Int(1), Var(5))
                } else {
                    Var(5)
                },
            ),
        ],
    );
    no_bad(&mut m, 2);
    safe(&mut m, "RetainedExitCannotFailRestore", eq(5, 0));
    m
}
pub(super) fn managed_workspace_provider_lease(b: bool) -> Model {
    let mut m = model(
        "ManagedWorkspaceProviderLease",
        b,
        &[
            ("generation", 1, 0),
            ("provider_lease", 1, 0),
            ("started", 1, 0),
            ("released", 1, 0),
            ("resource", 1, 0),
            ("bad", 1, 0),
        ],
    );
    action(
        &mut m,
        "StartOriginalProvider",
        all([eq(0, 0), eq(2, 0)]),
        &[(1, Int(1)), (2, Int(1))],
    );
    action(&mut m, "RestartLogic", eq(0, 0), &[(0, Int(1))]);
    action(
        &mut m,
        "OriginalGitEffect",
        all([eq(1, 1), eq(4, 0)]),
        &[(4, Int(1))],
    );
    action(&mut m, "ProviderAndChildrenEnd", eq(1, 1), &[(1, Int(0))]);
    action(
        &mut m,
        "ReconcileAndRelease",
        all([
            eq(0, 1),
            eq(3, 0),
            if b { Expr::Bool(true) } else { eq(1, 0) },
        ]),
        &[(3, Int(1)), (5, eq(1, 1).ite(Int(1), Var(5)))],
    );
    no_bad(&mut m, 5);
    m
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn managed_workspace_models_and_broken_controls() {
        let cases = [
            managed_workspace_create,
            managed_workspace_writer,
            managed_workspace_fence,
            managed_workspace_release,
            managed_workspace_ownership,
            managed_workspace_provider_lease,
        ]
        .into_iter()
        .flat_map(bboxy450_tlc::build::family)
        .chain([(broken_managed_workspace_writer_probe(), "safety")]);
        for (model, expected) in cases {
            let checked = model.check(20000).unwrap();
            assert_eq!(
                checked.violations.is_empty(),
                expected == "pass",
                "{}",
                model.name
            );
            let expected_violation = match model.name.as_str() {
                "BrokenManagedWorkspaceWriter" => Some("RejectedReleasesWriter"),
                "BrokenManagedWorkspaceWriterProbe" => Some("RequestPassedProbe"),
                _ => None,
            };
            if let Some(property) = expected_violation {
                let properties = checked
                    .violations
                    .iter()
                    .map(|v| v.property.as_str())
                    .collect::<Vec<_>>();
                assert_eq!(properties, [property], "{}", model.name);
            }
            let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("models");
            for (name, text) in bboxy450_tlc::build::artifacts(&model).unwrap() {
                let path = dir.join(name);
                if std::env::var_os("BBOXY_UPDATE_MODELS").is_some() {
                    std::fs::write(&path, &text).unwrap();
                }
                assert_eq!(std::fs::read_to_string(path).unwrap(), text);
            }
        }
    }
}
