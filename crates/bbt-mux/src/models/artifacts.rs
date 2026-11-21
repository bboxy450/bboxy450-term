// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Finite cases for the signed artifact stream and retained view labels.
use super::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ArtifactFault {
    None,
    Owner,
    Cas,
    Durability,
    Binding,
    Custody,
    Immutable,
}

pub(super) fn artifact_publication(fault: ArtifactFault) -> Model {
    let mut m = model(
        "ArtifactPublication",
        fault != ArtifactFault::None,
        &[
            ("owner", 2, 0),
            ("latest", 3, 0),
            ("custody", 2, 0),
            ("executionSource", 2, 0),
            ("expectedA", 3, 0),
            ("expectedB", 3, 0),
            ("blobA", 1, 0),
            ("blobB", 1, 0),
            ("publishedA", 1, 0),
            ("publishedB", 1, 0),
            ("atPublishOwner", 2, 0),
            ("expectedOwner", 2, 0),
            ("atPublishRev", 3, 0),
            ("atPublishExpected", 3, 0),
            ("atPublishBlob", 1, 0),
            ("atPublishSource", 1, 0),
            ("reference", 1, 0),
        ],
    );
    m.name.push_str(match fault {
        ArtifactFault::None => "",
        ArtifactFault::Owner => "Owner",
        ArtifactFault::Cas => "Cas",
        ArtifactFault::Durability => "Durability",
        ArtifactFault::Binding => "Binding",
        ArtifactFault::Custody => "Custody",
        ArtifactFault::Immutable => "Immutable",
    });
    let allow = |removed, expr| {
        if fault == removed {
            Expr::Bool(true)
        } else {
            expr
        }
    };
    action(
        &mut m,
        "ReferenceClaim",
        eq(2, 0),
        &[(2, Int(1)), (16, Int(1))],
    );
    action(
        &mut m,
        "RetainBaseRevision",
        allow(ArtifactFault::Custody, eq(2, 0)),
        &[(1, Int(1)), (2, Int(2))],
    );
    action(
        &mut m,
        "CaptureHostObservedSource",
        eq(3, 0),
        &[(3, Int(1))],
    );
    if fault == ArtifactFault::Immutable {
        action(&mut m, "RewriteExecutionSource", eq(3, 1), &[(3, Int(2))]);
    }
    for (i, owner_name) in [(0, "A"), (1, "B")] {
        let owner = (i + 1) as i64;
        let expected = 4 + i;
        let blob = 6 + i;
        let complete = 8 + i;
        action(
            &mut m,
            &format!("ReadExpected{owner_name}"),
            all([eq(2, 2), eq(expected, 0), Var(1).lt(Int(3))]),
            &[(expected, Var(1))],
        );
        action(
            &mut m,
            &format!("Claim{owner_name}"),
            all([eq(0, 0), ne(expected, 0), eq(complete, 0)]),
            &[(0, Int(owner))],
        );
        action(
            &mut m,
            &format!("DurableBytes{owner_name}"),
            all([eq(0, owner), eq(blob, 0)]),
            &[(blob, Int(1))],
        );
        action(
            &mut m,
            &format!("CrashAfterCapture{owner_name}"),
            eq(0, owner),
            &[(0, Int(0))],
        );
        action(
            &mut m,
            &format!("CanonicalPublish{owner_name}"),
            all([
                eq(2, 2),
                Var(1).lt(Int(3)),
                eq(complete, 0),
                ne(expected, 0),
                allow(ArtifactFault::Owner, eq(0, owner)),
                allow(ArtifactFault::Cas, Var(1).eq(Var(expected))),
                allow(ArtifactFault::Durability, eq(blob, 1)),
                allow(ArtifactFault::Binding, eq(3, 1)),
            ]),
            &[
                (0, Int(0)),
                (1, Var(1).plus(Int(1))),
                (complete, Int(1)),
                (10, Var(0)),
                (11, Int(owner)),
                (12, Var(1)),
                (13, Var(expected)),
                (14, Var(blob)),
                (15, Var(3)),
            ],
        );
    }
    let scoped = |property| eq(11, 0).or(property);
    safe(&mut m, "OneCanonicalOwner", scoped(Var(10).eq(Var(11))));
    safe(
        &mut m,
        "OriginalExpectedRevision",
        scoped(Var(12).eq(Var(13))),
    );
    safe(&mut m, "PayloadBeforePublish", scoped(eq(14, 1)));
    safe(&mut m, "HostSourceBeforePublish", scoped(eq(15, 1)));
    safe(&mut m, "ExecutionSourceIsWriteOnce", Var(3).le(Int(1)));
    safe(
        &mut m,
        "DenseDisplayedRevisions",
        eq(2, 2)
            .negate()
            .or(Var(1).eq(Int(1).plus(Var(8)).plus(Var(9)))),
    );
    safe(
        &mut m,
        "ReferenceCustodyNeverConverts",
        eq(16, 0).or(eq(2, 1)),
    );
    m
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum RetryFault {
    None,
    Duplicate,
    RereadStaging,
    ChangedArguments,
}
pub(super) fn artifact_recovery(fault: RetryFault) -> Model {
    let mut m = model(
        "ArtifactRecovery",
        fault != RetryFault::None,
        &[
            ("admitted", 1, 0),
            ("captured", 2, 0),
            ("staged", 2, 1),
            ("responseLost", 1, 0),
            ("writes", 2, 0),
            ("returned", 2, 0),
            ("acceptedChangedArguments", 1, 0),
            ("finished", 1, 0),
        ],
    );
    m.name.push_str(match fault {
        RetryFault::None => "",
        RetryFault::Duplicate => "Duplicate",
        RetryFault::RereadStaging => "RereadStaging",
        RetryFault::ChangedArguments => "ChangedArguments",
    });
    action(&mut m, "AdmitOriginal", eq(0, 0), &[(0, Int(1))]);
    action(
        &mut m,
        "CaptureOneSetOfBytes",
        all([eq(0, 1), eq(1, 0)]),
        &[(1, Var(2))],
    );
    action(
        &mut m,
        "StagePathReplaced",
        all([eq(1, 1), eq(2, 1)]),
        &[(2, Int(2))],
    );
    action(
        &mut m,
        "RetainPublicationAndLoseReceipt",
        all([eq(1, 1), eq(4, 0)]),
        &[(3, Int(1)), (4, Int(1))],
    );
    action(
        &mut m,
        "OriginalOperationRetry",
        all([eq(3, 1), eq(7, 0)]),
        &[
            (4, Int(if fault == RetryFault::Duplicate { 2 } else { 1 })),
            (
                5,
                if fault == RetryFault::RereadStaging {
                    Var(2)
                } else {
                    Var(1)
                },
            ),
            (7, Int(1)),
        ],
    );
    action(
        &mut m,
        "ChangedArgumentsRetry",
        all([eq(0, 1), eq(7, 0)]),
        &[(6, Int(i64::from(fault == RetryFault::ChangedArguments)))],
    );
    safe(&mut m, "OnePublicationPerOperation", Var(4).le(Int(1)));
    safe(
        &mut m,
        "RetryResolvesCapturedBytes",
        eq(7, 0).or(Var(5).eq(Var(1))),
    );
    safe(&mut m, "ChangedOperationArgumentsRefused", eq(6, 0));
    m.weak_fair.push(vec![3, 4]);
    m.responses.push(Response {
        name: "PublishedRetryCanFinish".into(),
        request: eq(3, 1),
        response: eq(7, 1),
    });
    m
}

pub(super) fn artifact_view_revision(broken: bool) -> Model {
    let mut m = model(
        "ArtifactViewRevision",
        broken,
        &[
            ("latest", 3, 1),
            ("livePending", 3, 0),
            ("pinPending", 3, 0),
            ("livePainted", 3, 0),
            ("pinnedPainted", 3, 0),
            ("liveCurrentAtApply", 3, 0),
            ("pinnedCurrentAtApply", 3, 0),
            ("viewGeneration", 3, 1),
            ("pendingGeneration", 3, 0),
            ("presentedGeneration", 3, 0),
            ("currentGenerationAtApply", 3, 0),
        ],
    );
    action(
        &mut m,
        "UpdateLatest",
        Var(0).lt(Int(3)),
        &[(0, Var(0).plus(Int(1)))],
    );
    action(&mut m, "ResolveLive", eq(1, 0), &[(1, Var(0)), (8, Var(7))]);
    action(&mut m, "ResolvePinnedOne", eq(2, 0), &[(2, Int(1))]);
    let current = all([Var(1).eq(Var(0)), Var(8).eq(Var(7))]);
    let aba = all([eq(1, 1), eq(0, 3)]);
    action(
        &mut m,
        "PresentLive",
        all([
            ne(1, 0),
            if broken {
                current.clone().or(aba)
            } else {
                current.clone()
            },
        ]),
        &[
            (1, Int(0)),
            (3, Var(1)),
            (5, Var(0)),
            (9, Var(8)),
            (10, Var(7)),
        ],
    );
    action(
        &mut m,
        "DiscardLateLive",
        all([ne(1, 0), current.negate()]),
        &[(1, Int(0))],
    );
    action(
        &mut m,
        "PresentPinned",
        eq(2, 1),
        &[(2, Int(0)), (4, Int(1)), (6, Int(1))],
    );
    action(
        &mut m,
        "CloseAndReopen",
        Var(7).lt(Int(3)),
        &[(7, Var(7).plus(Int(1)))],
    );
    safe(&mut m, "CompletionBelongsToCurrentView", Var(9).eq(Var(10)));
    safe(
        &mut m,
        "LiveLabelMatchesResolvedRevision",
        Var(3).eq(Var(5)),
    );
    safe(&mut m, "ExactHistoricalPin", Var(4).eq(Var(6)));
    m.weak_fair.push(vec![3, 4]);
    m.weak_fair.push(vec![5]);
    m.responses.push(Response {
        name: "ResolvedLiveFinishesOrIsSuperseded".into(),
        request: ne(1, 0),
        response: eq(1, 0),
    });
    m.responses.push(Response {
        name: "PinnedHistoryFinishes".into(),
        request: eq(2, 1),
        response: eq(4, 1),
    });
    m
}

pub(super) fn cases() -> Vec<(Model, &'static str)> {
    use ArtifactFault::*;
    use RetryFault::{ChangedArguments, Duplicate, RereadStaging};
    [None, Owner, Cas, Durability, Binding, Custody, Immutable]
        .into_iter()
        .map(|fault| {
            (
                artifact_publication(fault),
                if fault == None { "pass" } else { "safety" },
            )
        })
        .chain(
            [RetryFault::None, Duplicate, RereadStaging, ChangedArguments]
                .into_iter()
                .map(|fault| {
                    (
                        artifact_recovery(fault),
                        if fault == RetryFault::None {
                            "pass"
                        } else {
                            "safety"
                        },
                    )
                }),
        )
        .chain([
            (artifact_view_revision(false), "pass"),
            (artifact_view_revision(true), "safety"),
        ])
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn artifact_models_reach_recovery_history_two_views_and_controls() {
        for (model, expected) in cases() {
            let report = model.check(100_000).unwrap();
            assert_eq!(
                report.violations.is_empty(),
                expected == "pass",
                "{}",
                model.name
            );
            match model.name.as_str() {
                "ArtifactPublication" => assert!(report.states.iter().any(|state| state[1] == 3)),
                "ArtifactRecovery" => assert!(
                    report
                        .states
                        .iter()
                        .any(|state| state[2] == 2 && state[5] == 1)
                ),
                "ArtifactViewRevision" => assert!(
                    report
                        .states
                        .iter()
                        .any(|state| state[3] == 3 && state[4] == 1)
                ),
                _ => assert!(
                    report
                        .violations
                        .iter()
                        .any(|v| v.cycle.is_empty() && v.property != "TypeOK")
                ),
            }
        }
    }
}
