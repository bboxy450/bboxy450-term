// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Bounded contract projections of the sources mapped in verification/contracts.toml.
//! These are specifications, not translations of arbitrary Rust or OS semantics.
use Expr::{Bool, Int, Var};
use bboxy450_tlc::{Action, Expr, Invariant, Model, Response, Variable};

fn eq(i: usize, n: i64) -> Expr {
    Var(i).eq(Int(n))
}
fn ne(i: usize, n: i64) -> Expr {
    eq(i, n).negate()
}
fn all(xs: impl IntoIterator<Item = Expr>) -> Expr {
    xs.into_iter().fold(Bool(true), Expr::and)
}
fn implies(a: Expr, b: Expr) -> Expr {
    a.negate().or(b)
}
fn model(name: &str, broken: bool, vars: &[(&str, i64, i64)]) -> Model {
    Model {
        name: format!("{}{name}", if broken { "Broken" } else { "" }),
        variables: vars
            .iter()
            .map(|(n, max, initial)| Variable::new(n, 0, *max, *initial))
            .collect(),
        actions: vec![],
        invariants: vec![],
        responses: vec![],
        weak_fair: vec![],
        // Completed operations may quiesce; TLA stuttering is still explored for liveness.
        check_deadlock: false,
    }
}
fn action(m: &mut Model, name: &str, guard: Expr, updates: &[(usize, Expr)]) {
    m.actions.push(Action::new(name, guard, updates.to_vec()));
}
fn safe(m: &mut Model, name: &str, condition: Expr) {
    m.invariants.push(Invariant {
        name: name.into(),
        condition,
    });
}
fn no_bad(m: &mut Model, index: usize) {
    safe(m, "NoInvalidEffect", eq(index, 0));
}

// Each atomic action represents one atomic access in the immediate-wake protocol.
// Sequentially consistent interleavings only: weak-memory ordering needs Rust-specific tools.
fn dirty_wake(b: bool) -> Model {
    let mut m = model(
        "DirtyWake",
        b,
        &[
            ("bits", 1, 0),
            ("pending", 1, 0),
            ("producer_pc", 1, 0),
            ("drain_pc", 2, 0),
        ],
    );
    action(&mut m, "MarkBits", eq(2, 0), &[(0, Int(1)), (2, Int(1))]);
    action(&mut m, "PostWake", eq(2, 1), &[(1, Int(1)), (2, Int(0))]);
    action(&mut m, "TakeBits", eq(3, 0), &[(0, Int(0)), (3, Int(1))]);
    action(
        &mut m,
        "ClearPending",
        eq(3, 1),
        &[(1, Int(0)), (3, Int(2))],
    );
    action(
        &mut m,
        "RecheckEmpty",
        all([eq(3, 2), eq(0, 0)]),
        &[(3, Int(0))],
    );
    action(
        &mut m,
        "RecheckPublished",
        all([eq(3, 2), eq(0, 1)]),
        &[(3, Int(0)), (1, if b { Var(1) } else { Int(1) })],
    );
    safe(
        &mut m,
        "NoLostImmediateWake",
        implies(all([eq(2, 0), eq(3, 0), eq(0, 1)]), eq(1, 1)),
    );
    m
}
fn notification(b: bool) -> Model {
    let mut m = model(
        "NotificationReplace",
        b,
        &[
            ("submitted", 2, 0),
            ("queued", 2, 0),
            ("active", 2, 0),
            ("phase", 2, 0),
            ("processed", 2, 0),
            ("bad", 1, 0),
        ],
    );
    action(
        &mut m,
        "SubmitLatest",
        all([Var(0).lt(Int(2)), eq(1, 0)]),
        &[(0, Var(0).plus(Int(1))), (1, Var(0).plus(Int(1)))],
    );
    action(
        &mut m,
        "TakePlan",
        all([eq(3, 0), ne(1, 0)]),
        &[(2, Var(1)), (1, Int(0)), (3, Int(1))],
    );
    action(&mut m, "AuthorizationReturns", eq(3, 1), &[(3, Int(2))]);
    let newer = Var(2).lt(Var(1));
    action(
        &mut m,
        "ReplaceObsolete",
        all([eq(3, 2), newer.clone()]),
        &[(3, Int(0))],
    );
    action(
        &mut m,
        "FinishCurrent",
        all([eq(3, 2), newer.clone().negate()]),
        &[(3, Int(0)), (4, Var(2))],
    );
    if b {
        action(
            &mut m,
            "PostBeforeReplacementCheck",
            all([eq(3, 2), newer]),
            &[(5, Int(1))],
        );
    }
    no_bad(&mut m, 5);
    safe(&mut m, "ProcessedWasSubmitted", Var(4).le(Var(0)));
    m
}
fn workspace(b: bool) -> Model {
    let mut m = model(
        "WorkspaceFence",
        b,
        &[
            ("revision", 3, 1),
            ("root", 1, 0),
            ("job_revision", 3, 0),
            ("job_root", 1, 0),
            ("job", 2, 0),
            ("authorized", 1, 1),
            ("bad", 1, 0),
        ],
    );
    action(
        &mut m,
        "CaptureRead",
        eq(4, 0),
        &[(2, Var(0)), (3, Var(1)), (4, Int(1))],
    );
    action(
        &mut m,
        "SwitchRoot",
        Var(0).lt(Int(3)),
        &[(0, Var(0).plus(Int(1))), (1, Int(1).minus(Var(1)))],
    );
    action(&mut m, "RevokeAuthority", eq(5, 1), &[(5, Int(0))]);
    let valid = all([Var(2).eq(Var(0)), Var(3).eq(Var(1)), eq(5, 1)]);
    action(
        &mut m,
        "PublishCurrentRead",
        all([eq(4, 1), valid.clone()]),
        &[(4, Int(2))],
    );
    action(
        &mut m,
        "RefuseStaleRead",
        all([eq(4, 1), valid.clone().negate()]),
        &[(4, Int(2))],
    );
    // Checking the directory ID alone misses A -> B -> A and revoked authority.
    if b {
        action(
            &mut m,
            "PublishByRootIdOnly",
            all([eq(4, 1), Var(3).eq(Var(1)), valid.negate()]),
            &[(6, Int(1))],
        );
    }
    no_bad(&mut m, 6);
    m
}
fn plugin(b: bool) -> Model {
    let mut m = model(
        "PluginActivation",
        b,
        &[
            ("source_digest", 2, 1),
            ("approved_digest", 2, 1),
            ("enabled", 1, 1),
            ("generation", 2, 1),
            ("job_generation", 2, 0),
            ("job_digest", 2, 0),
            ("job", 2, 0),
            ("bad", 1, 0),
        ],
    );
    action(
        &mut m,
        "StartInspection",
        all([eq(6, 0), eq(2, 1)]),
        &[(4, Var(3)), (5, Var(0)), (6, Int(1))],
    );
    action(&mut m, "Disable", eq(2, 1), &[(2, Int(0)), (3, Int(2))]);
    action(&mut m, "SourceChanged", eq(0, 1), &[(0, Int(2))]);
    let valid = all([
        eq(2, 1),
        Var(4).eq(Var(3)),
        Var(5).eq(Var(0)),
        Var(5).eq(Var(1)),
    ]);
    action(
        &mut m,
        "AcceptCurrentInspection",
        all([eq(6, 1), valid.clone()]),
        &[(6, Int(2))],
    );
    action(
        &mut m,
        "RefuseOldInspection",
        all([eq(6, 1), valid.clone().negate()]),
        &[(6, Int(2))],
    );
    if b {
        action(
            &mut m,
            "AcceptOldInspection",
            all([eq(6, 1), valid.negate()]),
            &[(7, Int(1))],
        );
    }
    no_bad(&mut m, 7);
    m
}
fn token(b: bool) -> Model {
    let mut m = model(
        "HarnessToken",
        b,
        &[
            ("operator_epoch", 1, 0),
            ("proof", 1, 1),
            ("token", 1, 0),
            ("token_epoch", 1, 0),
            ("uncertain", 1, 0),
            ("sends", 2, 0),
            ("bad", 1, 0),
        ],
    );
    action(
        &mut m,
        "MintToken",
        all([eq(1, 1), eq(2, 0)]),
        &[(2, Int(1)), (3, Var(0))],
    );
    action(
        &mut m,
        "OperatorTakesInput",
        eq(0, 0),
        &[(0, Int(1)), (1, Int(0))],
    );
    action(
        &mut m,
        "NativeOutcomeUnknown",
        all([eq(5, 1), eq(4, 0)]),
        &[(4, Int(1)), (1, Int(0))],
    );
    // New readiness never clears the ambiguous outcome for the same delivery ID.
    action(
        &mut m,
        "FreshProof",
        all([eq(1, 0), eq(5, 1), eq(4, 1)]),
        &[(1, Int(1))],
    );
    let valid = all([eq(1, 1), Var(3).eq(Var(0)), eq(4, 0)]);
    action(
        &mut m,
        "ConsumeReadiness",
        all([eq(2, 1), Var(5).lt(Int(2)), valid.clone()]),
        &[(2, Int(0)), (1, Int(0)), (5, Var(5).plus(Int(1)))],
    );
    if b {
        action(
            &mut m,
            "UseStaleToken",
            all([eq(2, 1), valid.negate()]),
            &[(6, Int(1))],
        );
    }
    no_bad(&mut m, 6);
    m
}
fn custody(b: bool) -> Model {
    let mut m = model(
        "CustodyDelivery",
        b,
        &[("custody", 1, 0), ("delivered", 1, 0), ("signed", 1, 0)],
    );
    action(
        &mut m,
        "AdmitPendingIntent",
        all([eq(1, 0), if b { eq(0, 1) } else { Bool(true) }]),
        &[(1, Int(1))],
    );
    action(&mut m, "CustodyBecomesAvailable", eq(0, 0), &[(0, Int(1))]);
    action(
        &mut m,
        "UpgradeSignature",
        all([eq(0, 1), eq(1, 1), eq(2, 0)]),
        &[(2, Int(1))],
    );
    safe(
        &mut m,
        "NeverInventSignature",
        implies(eq(2, 1), all([eq(0, 1), eq(1, 1)])),
    );
    m.responses.push(Response {
        name: "LocalDeliveryProgress".into(),
        request: Bool(true),
        response: eq(1, 1),
    });
    // Delivery is scheduled fairly; custody recovery is intentionally NOT assumed.
    m.weak_fair.push(vec![0]);
    m
}
fn suppression(b: bool) -> Model {
    let mut m = model(
        "MessageSuppression",
        b,
        &[
            ("phase", 2, 0),
            ("conversation", 1, 0),
            ("muted", 1, 0),
            ("resolved", 1, 0),
            ("queued", 1, 0),
            ("bad", 1, 0),
        ],
    );
    action(
        &mut m,
        "AdmitEvent",
        eq(0, 0),
        &[(0, Int(1)), (1, Int(0)), (4, Int(1))],
    );
    action(
        &mut m,
        "AdmitConversation",
        eq(0, 0),
        &[(0, Int(1)), (1, Int(1)), (4, Int(1))],
    );
    action(&mut m, "MuteRecipient", eq(0, 1), &[(2, Int(1))]);
    action(&mut m, "ResolveOrSupersede", eq(0, 1), &[(3, Int(1))]);
    let eligible = all([eq(1, 1), eq(2, 0), eq(3, 0)]);
    action(
        &mut m,
        "WakeConversation",
        all([eq(4, 1), eligible.clone()]),
        &[(4, Int(0)), (0, Int(2))],
    );
    action(
        &mut m,
        "SuppressWake",
        all([eq(4, 1), eligible.clone().negate()]),
        &[(4, Int(0)), (0, Int(2))],
    );
    if b {
        action(
            &mut m,
            "WakeWithoutReconcile",
            all([eq(4, 1), eligible.negate()]),
            &[(5, Int(1))],
        );
    }
    no_bad(&mut m, 5);
    m
}
fn lifecycle(b: bool) -> Model {
    let mut m = model(
        "LifecycleOwnership",
        b,
        &[
            ("ui_attached", 1, 1),
            ("owner_live", 1, 1),
            ("stop_requested", 1, 0),
            ("birth_matches", 1, 1),
            ("bad", 1, 0),
        ],
    );
    action(&mut m, "DetachUi", eq(0, 1), &[(0, Int(0))]);
    action(&mut m, "ExplicitStop", eq(2, 0), &[(2, Int(1))]);
    action(&mut m, "PidReused", eq(3, 1), &[(3, Int(0))]);
    action(
        &mut m,
        "SignalExactOwner",
        all([eq(1, 1), eq(2, 1), eq(3, 1)]),
        &[(1, Int(0))],
    );
    if b {
        action(
            &mut m,
            "CloseKillsOwner",
            all([eq(0, 1), eq(2, 0)]),
            &[(1, Int(0)), (4, Int(1))],
        );
        action(
            &mut m,
            "SignalByPidAlone",
            all([eq(1, 1), eq(2, 1), eq(3, 0)]),
            &[(4, Int(1))],
        );
    }
    no_bad(&mut m, 4);
    safe(&mut m, "DetachPreservesOwner", implies(eq(2, 0), eq(1, 1)));
    m
}
fn window(b: bool) -> Model {
    let mut m = model(
        "WindowDurability",
        b,
        &[
            ("desired", 2, 1),
            ("saved", 2, 0),
            ("job_version", 2, 0),
            ("epoch", 1, 0),
            ("job_epoch", 1, 0),
            ("reply_version", 2, 0),
            ("reply_state", 2, 0),
            ("bad", 1, 0),
        ],
    );
    action(&mut m, "ObserveNewFrame", eq(0, 1), &[(0, Int(2))]);
    action(&mut m, "FenceReply", eq(6, 0), &[(5, Var(0)), (6, Int(1))]);
    action(
        &mut m,
        "StartSave",
        all([eq(2, 0), Var(1).lt(Var(0))]),
        &[(2, Var(0)), (4, Var(3))],
    );
    action(&mut m, "RestartGui", eq(3, 0), &[(3, Int(1)), (1, Int(0))]);
    action(
        &mut m,
        "CompleteCurrentSave",
        all([ne(2, 0), Var(4).eq(Var(3))]),
        &[(1, Var(2)), (2, Int(0))],
    );
    action(
        &mut m,
        "IgnoreOldSave",
        all([ne(2, 0), Var(4).eq(Var(3)).negate()]),
        &[(2, Int(0))],
    );
    action(
        &mut m,
        "ReleaseDurableReply",
        all([eq(6, 1), Var(5).le(Var(1))]),
        &[(6, Int(2))],
    );
    if b {
        action(
            &mut m,
            "AcceptOldSave",
            all([ne(2, 0), Var(4).eq(Var(3)).negate()]),
            &[(7, Int(1))],
        );
        action(
            &mut m,
            "ReleaseUnsavedReply",
            all([eq(6, 1), Var(1).lt(Var(5))]),
            &[(7, Int(1))],
        );
    }
    no_bad(&mut m, 7);
    safe(&mut m, "SavedNotFuture", Var(1).le(Var(0)));
    m
}
fn focus(b: bool) -> Model {
    let mut m = model(
        "FocusFrame",
        b,
        &[
            ("generation", 1, 0),
            ("captured_generation", 1, 0),
            ("completed", 1, 0),
            ("focused", 1, 1),
            ("visible", 1, 1),
            ("prior_settled", 1, 0),
            ("applied", 1, 0),
            ("answered", 1, 0),
            ("bad", 1, 0),
        ],
    );
    action(&mut m, "PriorPlacementSettles", eq(5, 0), &[(5, Int(1))]);
    action(
        &mut m,
        "ApplyFocus",
        all([eq(5, 1), eq(6, 0)]),
        &[(1, Var(0)), (6, Int(1)), (2, Int(0))],
    );
    action(&mut m, "FrameCompletes", eq(6, 1), &[(2, Int(1))]);
    action(&mut m, "ProcessRestarts", eq(0, 0), &[(0, Int(1))]);
    action(&mut m, "FocusMoves", eq(3, 1), &[(3, Int(0))]);
    action(&mut m, "PaneHidden", eq(4, 1), &[(4, Int(0))]);
    let valid = all([eq(2, 1), eq(3, 1), eq(4, 1), Var(1).eq(Var(0)), eq(5, 1)]);
    action(
        &mut m,
        "AnswerCompletedFrame",
        all([eq(6, 1), eq(7, 0), valid.clone()]),
        &[(7, Int(1))],
    );
    if b {
        action(
            &mut m,
            "AnswerBeforeFrameCheck",
            all([eq(6, 1), eq(7, 0), valid.negate()]),
            &[(8, Int(1))],
        );
    }
    no_bad(&mut m, 8);
    m
}
fn restore(b: bool) -> Model {
    let mut m = model(
        "CheckpointRestore",
        b,
        &[
            ("cursor", 2, 0),
            ("checkpoint", 2, 0),
            ("queued_effects", 2, 0),
            ("restored", 1, 0),
            ("bad", 1, 0),
        ],
    );
    action(
        &mut m,
        "ParseLiveBytes",
        all([eq(3, 0), Var(0).lt(Int(2))]),
        &[(0, Var(0).plus(Int(1))), (2, Var(2).plus(Int(1)))],
    );
    action(&mut m, "CaptureParserAndGrid", eq(3, 0), &[(1, Var(0))]);
    action(
        &mut m,
        "RestoreWithoutEffects",
        eq(3, 0),
        &[(0, Var(1)), (2, Int(0)), (3, Int(1))],
    );
    if b {
        action(
            &mut m,
            "RestoreQueuedEffects",
            all([eq(3, 0), ne(2, 0)]),
            &[(3, Int(1)), (4, Int(1))],
        );
    }
    no_bad(&mut m, 4);
    safe(&mut m, "RestoreNeverReemits", implies(eq(3, 1), eq(2, 0)));
    m
}
fn seats(b: bool) -> Model {
    let mut m = model(
        "SeatFairness",
        b,
        &[
            ("watched_streak", 3, 0),
            ("watched_paused", 1, 0),
            ("background_pending", 1, 1),
            ("watched_page", 1, 0),
        ],
    );
    action(
        &mut m,
        "IngestWatched",
        all([eq(1, 0), Var(0).lt(Int(3))]),
        &[(0, Var(0).plus(Int(1))), (3, Int(1).minus(Var(3)))],
    );
    if b {
        action(
            &mut m,
            "PreferWatchedForever",
            all([eq(1, 0), eq(0, 3)]),
            &[(3, Int(1).minus(Var(3)))],
        );
    }
    let bg = m.actions.len();
    action(
        &mut m,
        "IngestBackground",
        all([eq(2, 1), eq(0, 3).or(eq(1, 1))]),
        &[(2, Int(0)), (0, Int(0))],
    );
    action(
        &mut m,
        "PauseWatchedForCheckpoint",
        eq(1, 0),
        &[(1, Int(1))],
    );
    m.responses.push(Response {
        name: "BackgroundMakesProgress".into(),
        request: eq(2, 1),
        response: eq(2, 0),
    });
    // A fair scheduler turn, not fairness for each individual seat. Quota supplies isolation.
    let mut scheduler = vec![0, bg];
    if b {
        scheduler.push(1);
    }
    m.weak_fair.push(scheduler);
    m
}
fn arena(b: bool) -> Model {
    let mut m = model(
        "RenderSubmission",
        b,
        &[
            ("cursor", 2, 0),
            ("first_live", 1, 0),
            ("pending_upload", 1, 0),
            ("submitted", 1, 0),
            ("bad", 1, 0),
        ],
    );
    action(
        &mut m,
        "AllocateFirstSlice",
        all([eq(1, 0), eq(3, 0)]),
        &[(0, Int(1)), (1, Int(1)), (2, Int(1))],
    );
    action(
        &mut m,
        "AllocateDisjointSlice",
        all([eq(1, 1), eq(0, 1), eq(3, 0)]),
        &[(0, Int(2)), (2, Int(1))],
    );
    action(&mut m, "FlushStaging", eq(2, 1), &[(2, Int(0))]);
    action(
        &mut m,
        "SubmitFlushedCommands",
        all([eq(3, 0), eq(2, 0)]),
        &[(3, Int(1))],
    );
    action(
        &mut m,
        "BeginNextSubmission",
        eq(3, 1),
        &[(3, Int(0)), (0, Int(0)), (1, Int(0))],
    );
    if b {
        action(
            &mut m,
            "RewindInsideSubmission",
            all([eq(1, 1), eq(3, 0)]),
            &[(0, Int(0))],
        );
        action(
            &mut m,
            "AllocateOverlappingSlice",
            all([eq(0, 0), eq(1, 1)]),
            &[(4, Int(1))],
        );
        action(&mut m, "SubmitBeforeFlush", eq(2, 1), &[(4, Int(1))]);
    }
    no_bad(&mut m, 4);
    safe(&mut m, "LiveSlicesNotReused", implies(eq(1, 1), ne(0, 0)));
    m
}

pub fn cases() -> Vec<(Model, &'static str)> {
    let families: [fn(bool) -> Model; 13] = [
        dirty_wake,
        notification,
        workspace,
        plugin,
        token,
        custody,
        suppression,
        lifecycle,
        window,
        focus,
        restore,
        seats,
        arena,
    ];
    families
        .into_iter()
        .flat_map(|family| {
            let good = family(false);
            let temporal = !good.responses.is_empty();
            [
                (good, "pass"),
                (family(true), if temporal { "liveness" } else { "safety" }),
            ]
        })
        .collect()
}
