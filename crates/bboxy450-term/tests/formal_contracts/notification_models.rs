// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Notification IO progress and retained desktop/current-episode contracts.
use super::*;

// One seat, enabled policy, one question/cancel and at most one transient failure
// across authorization/post/remove. Each native call start and completion can
// interleave with cancellation. No OS latency or permanent permission guarantee.
pub(super) fn recovery(broken: bool) -> Model {
    let mut m = model(
        "NotificationRecovery",
        broken,
        &[
            ("question_phase", 2, 0), // no question / waiting / cancelled
            ("permission_checked", 1, 0),
            ("native_notice", 1, 0),
            ("call", 3, 0), // idle / authorization / post / remove
            ("retry_pending", 1, 0),
            ("transient_failures_left", 1, 1),
        ],
    );
    action(
        &mut m,
        "BeginInitialPermissionCheck",
        all([eq(1, 0), eq(3, 0), eq(4, 0)]),
        &[(3, Int(1))],
    );
    action(
        &mut m,
        "PermissionAvailable",
        eq(3, 1),
        &[(1, Int(1)), (3, Int(0))],
    );
    action(
        &mut m,
        "QuestionArrives",
        eq(0, 0),
        &[(0, Int(1))],
    );
    action(
        &mut m,
        "BeginPost",
        all([eq(0, 1), eq(1, 1), eq(2, 0), eq(3, 0), eq(4, 0)]),
        &[(3, Int(2))],
    );
    action(
        &mut m,
        "PostCompletes",
        eq(3, 2),
        &[(2, Int(1)), (3, Int(0))],
    );
    // Cancellation cannot retract a native call already in flight; its eventual
    // completion is followed by withdrawal. A failed obsolete post is not retried.
    action(
        &mut m,
        "CancelQuestion",
        eq(0, 1),
        &[(0, Int(2)), (4, Int(0))],
    );
    action(
        &mut m,
        "BeginWithdrawal",
        all([eq(0, 2), eq(2, 1), eq(3, 0), eq(4, 0)]),
        &[(3, Int(3))],
    );
    action(
        &mut m,
        "WithdrawalCompletes",
        eq(3, 3),
        &[(2, Int(0)), (3, Int(0))],
    );
    for (call, name) in [
        (1, "PermissionTemporarilyUnavailable"),
        (2, "PostTemporarilyUnavailable"),
        (3, "WithdrawalTemporarilyUnavailable"),
    ] {
        action(
            &mut m,
            name,
            all([eq(3, call), eq(5, 1)]),
            &[(3, Int(0)), (4, Int(1)), (5, Int(0))],
        );
    }
    // Each enabled worker stage and call completion is weakly fair. Failure is
    // finite; permission/service availability eventually allows completion.
    m.weak_fair = vec![
        vec![0],
        vec![1, 8],
        vec![3],
        vec![4, 9],
        vec![6],
        vec![7, 10],
    ];
    if !broken {
        let timer = m.actions.len();
        action(
            &mut m,
            "RetryTimerFires",
            all([eq(3, 0), eq(4, 1)]),
            &[(4, Int(0))],
        );
        m.weak_fair.push(vec![timer]);
    }
    safe(
        &mut m,
        "NoPostWithoutPermission",
        implies(eq(2, 1), eq(1, 1)),
    );
    m.responses.extend([
        Response {
            name: "EnabledPolicyChecksBeforeQuestion".into(),
            request: Bool(true),
            response: eq(1, 1),
        },
        Response {
            name: "WaitingEventuallyPostsOrCancels".into(),
            request: eq(0, 1),
            response: eq(2, 1).or(eq(0, 2)),
        },
        Response {
            name: "CancelledEventuallyWithdraws".into(),
            request: eq(0, 2),
            response: all([eq(2, 0), eq(3, 0)]),
        },
    ]);
    m
}

// One stable seat ID, two questions, two UI streams. Source incarnations 1 and
// 3 both refer to apparent source A around source B (2), but exact keys differ.
// Applying a current projection/context frame and completing a captured click
// are separate atomic actions; arbitrary delay, duplicate frames and old EOF
// may interleave. Transport admission/OS execution remain real-API tests.
pub(super) fn lifecycle(broken: bool) -> Model {
    let mut m = model(
        "AttentionLifecycle",
        broken,
        &[
            ("source_incarnation", 3, 1),
            ("current_question", 2, 0),
            ("questions_seen", 2, 0),
            ("source_ready", 1, 1),
            ("latest_ui_stream", 2, 1),
            ("ui_attached", 1, 1),
            ("desktop_owner_live", 1, 1),
            ("captured_incarnation", 3, 0),
            ("captured_question", 2, 0),
            ("handled_incarnation", 3, 0),
            ("handled_question", 2, 0),
            ("bad", 1, 0),
        ],
    );
    action(
        &mut m,
        "FirstLiveQuestion",
        all([eq(0, 1), eq(2, 0), eq(3, 1)]),
        &[(1, Int(1)), (2, Int(1))],
    );
    for (from, name) in [(1, "ReconnectAtoB"), (2, "ReconnectBtoA")] {
        action(
            &mut m,
            name,
            eq(0, from),
            &[
                (0, Int(from + 1)),
                (1, Int(0)),
                (3, Int(0)),
                (9, Int(0)),
                (10, Int(0)),
            ],
        );
    }
    action(&mut m, "CurrentBootstrap", eq(3, 0), &[(3, Int(1))]);
    action(
        &mut m,
        "SecondLiveQuestionAtReturnedA",
        all([eq(0, 3), eq(2, 1), eq(3, 1)]),
        &[(1, Int(2)), (2, Int(2))],
    );
    action(
        &mut m,
        "ResolveQuestion",
        ne(1, 0),
        &[(1, Int(0)), (9, Int(0)), (10, Int(0))],
    );
    action(&mut m, "ObserverDisconnected", eq(3, 1), &[(3, Int(0))]);
    action(
        &mut m,
        "CaptureNativeClickBeforeShow",
        all([eq(3, 1), ne(1, 0), eq(8, 0)]),
        &[(7, Var(0)), (8, Var(1))],
    );
    let current = all([eq(3, 1), Var(7).eq(Var(0)), Var(8).eq(Var(1))]);
    action(
        &mut m,
        "CompleteExactCapturedClick",
        all([ne(8, 0), current.clone()]),
        &[(9, Var(7)), (10, Var(8)), (7, Int(0)), (8, Int(0))],
    );
    action(
        &mut m,
        "RefuseStaleCapturedClick",
        all([ne(8, 0), current.negate()]),
        &[(7, Int(0)), (8, Int(0))],
    );
    action(&mut m, "DetachCurrentUi", eq(5, 1), &[(5, Int(0))]);
    action(
        &mut m,
        "AttachNewerUiStream",
        eq(4, 1),
        &[(4, Int(2)), (5, Int(1))],
    );
    // Self-loops explicitly cover duplicate/current snapshots, old context/EOF
    // rejection and replay-only waiting. None mints an episode or handles it.
    action(&mut m, "RepeatCurrentSnapshot", eq(3, 1), &[]);
    action(&mut m, "IgnoreOldContextOrEof", eq(4, 2), &[]);
    action(&mut m, "ReplayOnlyWaitingCannotMint", eq(1, 0), &[]);
    if broken {
        action(
            &mut m,
            "CompleteBySeatIdAfterAtoBtoA",
            all([eq(0, 3), eq(1, 2), eq(7, 1), eq(8, 1)]),
            &[(9, Var(7)), (10, Var(8)), (11, Int(1))],
        );
        action(
            &mut m,
            "OldEofDetachesNewUi",
            all([eq(4, 2), eq(5, 1)]),
            &[(5, Int(0)), (11, Int(1))],
        );
        action(
            &mut m,
            "UiDetachStopsDesktopOwner",
            eq(5, 1),
            &[(5, Int(0)), (6, Int(0))],
        );
        action(
            &mut m,
            "UiDetachForgetsCurrentQuestion",
            all([eq(5, 1), ne(1, 0)]),
            &[(5, Int(0)), (1, Int(0)), (11, Int(1))],
        );
        action(
            &mut m,
            "ReplayMintsQuestion",
            all([eq(1, 0), eq(2, 0)]),
            &[(1, Int(1)), (11, Int(1))],
        );
    }
    no_bad(&mut m, 11);
    safe(&mut m, "DetachRetainsDesktopOwner", eq(6, 1));
    safe(
        &mut m,
        "OnlyExactCurrentEpisodeHandled",
        implies(ne(10, 0), all([Var(9).eq(Var(0)), Var(10).eq(Var(1))])),
    );
    m
}
