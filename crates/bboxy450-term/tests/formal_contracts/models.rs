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

#[path = "notification_models.rs"]
mod notification_models;

#[path = "seat_models.rs"]
mod seat_models;

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
            ("job", 3, 0),
            ("authorized", 1, 1),
            ("bad", 1, 0),
            ("provider_revision", 3, 1),
            ("job_provider_revision", 3, 0),
        ],
    );
    action(
        &mut m,
        "CaptureRead",
        eq(4, 0),
        &[(2, Var(0)), (3, Var(1)), (4, Int(1)), (8, Var(7))],
    );
    // Record and decoded bitmap bytes stay one captured job across worker preparation.
    action(&mut m, "PrepareCapturedImages", eq(4, 1), &[(4, Int(2))]);
    action(
        &mut m,
        "SwitchRoot",
        Var(0).lt(Int(3)),
        &[(0, Var(0).plus(Int(1))), (1, Int(1).minus(Var(1)))],
    );
    action(
        &mut m,
        "SwitchProvider",
        Var(7).lt(Int(3)),
        &[(7, Var(7).plus(Int(1)))],
    );
    action(&mut m, "RevokeAuthority", eq(5, 1), &[(5, Int(0))]);
    let workspace_valid = all([Var(2).eq(Var(0)), Var(3).eq(Var(1)), eq(5, 1)]);
    let valid = all([workspace_valid.clone(), Var(8).eq(Var(7))]);
    action(
        &mut m,
        "PublishCurrentRead",
        all([eq(4, 2), valid.clone()]),
        &[(4, Int(3))],
    );
    action(
        &mut m,
        "RefuseStaleRead",
        all([eq(4, 2), valid.clone().negate()]),
        &[(4, Int(3))],
    );
    // A root can stay unchanged while its provider changes, including A -> B -> A.
    // Checking the directory ID alone also misses workspace revisions and revocation.
    if b {
        action(
            &mut m,
            "DecodeImagesFromCurrentRoot",
            all([eq(4, 1), Var(3).eq(Var(1)).negate()]),
            &[(6, Int(1))],
        );
        action(
            &mut m,
            "PublishByRootIdOnly",
            all([eq(4, 2), Var(3).eq(Var(1)), valid.negate()]),
            &[(6, Int(1))],
        );
        action(
            &mut m,
            "PublishByWorkspaceOnly",
            all([eq(4, 2), workspace_valid, Var(8).eq(Var(7)).negate()]),
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
    // Logic closes a GUI that misses its reply-write deadline (#79): only the
    // attachment changes, and the app reattaches to the same live owner.
    action(&mut m, "GuiConnectionLost", eq(0, 1), &[(0, Int(0))]);
    action(
        &mut m,
        "ReattachUi",
        all([eq(0, 0), eq(1, 1)]),
        &[(0, Int(1))],
    );
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
            "ConnectionLossKillsOwner",
            all([eq(0, 1), eq(2, 0)]),
            &[(0, Int(0)), (1, Int(0)), (4, Int(1))],
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

// Check/test evidence is tied to a commit, inputs and current remote base.
fn landing_verification(b: bool) -> Model {
    let mut m = model(
        "LandingVerification",
        b,
        &[
            ("candidate", 3, 1),
            ("tests", 3, 0),
            ("formal", 3, 0),
            ("inputs", 1, 0),
            ("formal_inputs", 1, 0),
            ("remote", 2, 1),
            ("base", 2, 1),
            ("clean", 1, 1),
            ("bad", 1, 0),
            ("evidence_intact", 1, 0),
        ],
    );
    action(
        &mut m,
        "CommitFeature",
        eq(0, 1),
        &[(0, Int(2)), (7, Int(1))],
    );
    action(&mut m, "RustChecksPass", eq(7, 1), &[(1, Var(0))]);
    action(
        &mut m,
        "NativeAndTlcAgree",
        eq(7, 1),
        &[(2, Var(0)), (4, Var(3)), (9, Int(1))],
    );
    action(
        &mut m,
        "SourceChangesWithoutCommit",
        eq(3, 0),
        &[(3, Int(1)), (7, Int(0))],
    );
    action(&mut m, "RemoteMainMoves", eq(5, 1), &[(5, Int(2))]);
    action(
        &mut m,
        "RebaseChangesCandidate",
        all([Var(6).eq(Var(5)).negate(), eq(7, 1)]),
        &[(0, Int(3)), (6, Var(5))],
    );
    action(
        &mut m,
        "EvidenceRemovedOrTampered",
        eq(9, 1),
        &[(9, Int(0))],
    );
    let valid = all([
        Var(1).eq(Var(0)),
        Var(2).eq(Var(0)),
        Var(4).eq(Var(3)),
        Var(6).eq(Var(5)),
        eq(7, 1),
        eq(9, 1),
    ]);
    action(&mut m, "PublishVerifiedCandidate", valid.clone(), &[]);
    if b {
        action(
            &mut m,
            "PublishUsingOldOrPartialEvidence",
            all([
                Var(1).eq(Var(0)),
                Var(6).eq(Var(5)),
                eq(7, 1),
                valid.negate(),
            ]),
            &[(8, Int(1))],
        );
    }
    no_bad(&mut m, 8);
    m
}

// Atomic host actions: publish, press, move, release, copy; no IO completion.
// Two source changes cover A -> B -> A while capture from A must remain retired.
fn selection_gesture(b: bool) -> Model {
    let mut m = model(
        "SelectionGesture",
        b,
        &[
            ("source", 2, 0),
            ("captured", 2, 0),
            ("gesture", 1, 0),
            ("range", 1, 0),
            ("bad_copy", 1, 0),
        ],
    );
    action(
        &mut m,
        "Press",
        Bool(true),
        &[(1, Var(0)), (2, Int(1)), (3, Int(0))],
    );
    action(
        &mut m,
        "Drag",
        all([eq(2, 1), Var(1).eq(Var(0))]),
        &[(3, Int(1))],
    );
    action(&mut m, "Release", eq(2, 1), &[(2, Int(0))]);
    action(
        &mut m,
        "ReplaceSource",
        eq(0, 2).negate(),
        &[
            (0, Var(0).plus(Int(1))),
            (2, if b { Var(2) } else { Int(0) }),
            (3, if b { Var(3) } else { Int(0) }),
        ],
    );
    action(&mut m, "Copy", all([eq(3, 1), Var(1).eq(Var(0))]), &[]);
    if b {
        action(
            &mut m,
            "CopyStale",
            all([eq(3, 1), Var(1).eq(Var(0)).negate()]),
            &[(4, Int(1))],
        );
    }
    no_bad(&mut m, 4);
    m
}

// Four independent fold bits; save/restore is an atomic durable snapshot here.
// WindowDurability separately checks asynchronous write/reply interleavings.
fn sidebar_accordions(b: bool) -> Model {
    let mut m = model(
        "SidebarAccordions",
        b,
        &[
            ("actual", 15, 0),
            ("expected", 15, 0),
            ("saved", 15, 0),
            ("focus", 1, 0),
            ("expected_focus", 1, 0),
        ],
    );
    for mask in 0..16 {
        for bit in 0..4 {
            let expected = mask ^ (1 << bit);
            let actual = if b && bit == 2 {
                expected ^ 8
            } else {
                expected
            };
            action(
                &mut m,
                &format!("Toggle_{bit}_{mask}"),
                all([eq(0, mask), eq(1, mask)]),
                &[(0, Int(actual)), (1, Int(expected))],
            );
        }
    }
    action(&mut m, "Save", Bool(true), &[(2, Var(0))]);
    action(&mut m, "Restore", Bool(true), &[(0, Var(2)), (1, Var(2))]);
    for focus in 0..2 {
        action(
            &mut m,
            &format!("Focus_{focus}"),
            Bool(true),
            &[(3, Int(focus)), (4, Int(focus))],
        );
    }
    safe(&mut m, "IndependentFolds", Var(0).eq(Var(1)));
    safe(&mut m, "FoldDoesNotFocus", Var(3).eq(Var(4)));
    m
}

// One cached plan and two monotonically advancing font/mask incarnations.
// A repack changes coordinates, not mask identity; plans contain no coordinates.
fn render_line_plan(b: bool) -> Model {
    let mut m = model(
        "RenderLinePlan",
        b,
        &[
            ("font", 2, 0),
            ("mask", 2, 0),
            ("plan_font", 2, 0),
            ("plan_mask", 2, 0),
            ("cached", 1, 0),
            ("bad", 1, 0),
        ],
    );
    action(
        &mut m,
        "BuildPlan",
        Bool(true),
        &[(2, Var(0)), (3, Var(1)), (4, Int(1))],
    );
    action(
        &mut m,
        "EvictInk",
        Var(1).lt(Int(2)),
        &[(1, Var(1).plus(Int(1)))],
    );
    action(
        &mut m,
        "ChangeFonts",
        Var(0).lt(Int(2)),
        &[(0, Var(0).plus(Int(1))), (4, Int(0))],
    );
    action(&mut m, "RepackAtlas", Bool(true), &[]);
    action(&mut m, "TrimPlans", Bool(true), &[(4, Int(0))]);
    action(
        &mut m,
        "ReuseCurrentPlan",
        all([eq(4, 1), Var(2).eq(Var(0)), Var(3).eq(Var(1))]),
        &[],
    );
    if b {
        // Removing the mask-identity guard admits reuse of an evicted incarnation.
        action(
            &mut m,
            "ReuseEvictedInk",
            all([eq(4, 1), Var(2).eq(Var(0)), Var(3).eq(Var(1)).negate()]),
            &[(5, Int(1))],
        );
    }
    no_bad(&mut m, 5);
    m
}

// An awaited worker owns only its prepared value. Dropping the waiter cannot
// install a later result into a dead UI owner; completion retires it instead.
fn async_render_preparation(b: bool) -> Model {
    let mut m = model(
        "AsyncRenderPreparation",
        b,
        &[("owner", 1, 1), ("worker", 2, 0), ("installed", 1, 0)],
    );
    action(&mut m, "Start", all([eq(0, 1), eq(1, 0)]), &[(1, Int(1))]);
    action(&mut m, "DropWaiter", eq(0, 1), &[(0, Int(0)), (2, Int(0))]);
    action(
        &mut m,
        "Deliver",
        all([eq(0, 1), eq(1, 1)]),
        &[(1, Int(2)), (2, Int(1))],
    );
    action(
        &mut m,
        "CompleteAfterDrop",
        all([eq(0, 0), eq(1, 1)]),
        &[(1, Int(2)), (2, Int(i64::from(b)))],
    );
    safe(&mut m, "NoLateInstallation", implies(eq(0, 0), eq(2, 0)));
    m
}

// Target and three displayed disclosure steps; one owner ticks or retargets.
// This safety projection does not assume a clock or fair OS frame delivery.
fn sidebar_reveal(b: bool) -> Model {
    let mut m = model(
        "SidebarReveal",
        b,
        &[("target", 1, 0), ("display", 2, 0), ("frames", 1, 0)],
    );
    for target in 0..=1 {
        for display in 0..=2 {
            let next = 1 - target;
            action(
                &mut m,
                &format!("Toggle_{target}_{display}"),
                all([eq(0, target), eq(1, display)]),
                &[(0, Int(next)), (2, Int(i64::from(display != next * 2)))],
            );
            if display != target * 2 {
                let step = display + if target == 1 { 1 } else { -1 };
                action(
                    &mut m,
                    &format!("Tick_{target}_{display}"),
                    all([eq(0, target), eq(1, display)]),
                    &[(1, Int(step)), (2, Int(i64::from(b || step != target * 2)))],
                );
            }
        }
    }
    safe(
        &mut m,
        "NoFramesAfterSettlement",
        implies(
            all([eq(0, 0), eq(1, 0)]).or(all([eq(0, 1), eq(1, 2)])),
            eq(2, 0),
        ),
    );
    m
}

// Import owns staging/reservations; a separate observer and one ordinary writer
// may interleave before publication. Real fsync/signature/notify behavior is
// witnessed by the private active-observer migration, not abstracted as Rust.
fn import_staging(b: bool) -> Model {
    let mut m = model(
        "ImportStaging",
        b,
        &[
            ("staged", 1, 0),
            ("private_observed", 1, 0),
            ("epoch", 2, 0),
            ("reserved", 2, 0),
            ("ordinary_written", 1, 0),
            ("published", 1, 0),
        ],
    );
    action(
        &mut m,
        "Stage",
        all([eq(0, 0), eq(5, 0)]),
        &[(0, Int(1)), (3, Var(2))],
    );
    action(
        &mut m,
        "ObservePrivate",
        all([eq(0, 1), eq(1, 0), eq(5, 0)]),
        &[
            (1, Int(if b { 1 } else { 0 })),
            (2, if b { Var(2).plus(Int(1)) } else { Var(2) }),
        ],
    );
    action(
        &mut m,
        "OrdinaryWrite",
        eq(4, 0),
        &[(2, Var(2).plus(Int(1))), (4, Int(1))],
    );
    action(
        &mut m,
        "RetryStaleAllocation",
        all([eq(0, 1), eq(5, 0), Var(2).eq(Var(3)).negate()]),
        &[(3, Var(2))],
    );
    action(
        &mut m,
        "Publish",
        all([eq(0, 1), eq(5, 0), Var(2).eq(Var(3))]),
        &[(0, Int(0)), (5, Int(1))],
    );
    safe(&mut m, "StagingNeverConsumesARecordNumber", eq(1, 0));
    m.responses.push(Response {
        name: "StagedImportEventuallyPublishes".into(),
        request: eq(0, 1),
        response: eq(5, 1),
    });
    // The importer gets weakly fair retry/publication turns. One legitimate
    // writer is bounded; OS notification latency and arbitrary churn are not.
    m.weak_fair.extend([vec![3], vec![4]]);
    m
}

// The authenticated operator may archive old records in the captured native
// root without manufacturing live seat ownership. Other callers retain the
// normal ownership check; accepted verification cannot become historical done.
fn historical_reconcile(b: bool) -> Model {
    let mut m = model(
        "HistoricalReconcile",
        b,
        &[
            ("chosen", 1, 0),
            ("actor", 2, 0),
            ("native_root", 1, 0),
            ("owner_current", 1, 0),
            ("accepted", 1, 0),
            ("attempted", 1, 0),
            ("done", 1, 0),
        ],
    );
    for actor in 0..=2 {
        for root in 0..=1 {
            for owner in 0..=1 {
                for accepted in 0..=1 {
                    action(
                        &mut m,
                        &format!("Choose{actor}{root}{owner}{accepted}"),
                        eq(0, 0),
                        &[
                            (0, Int(1)),
                            (1, Int(actor)),
                            (2, Int(root)),
                            (3, Int(owner)),
                            (4, Int(accepted)),
                        ],
                    );
                }
            }
        }
    }
    let authorized = all([
        eq(2, 1),
        eq(4, 0),
        eq(1, 2).or(all([eq(1, 1), if b { Bool(true) } else { eq(3, 1) }])),
    ]);
    let pending = all([eq(0, 1), eq(5, 0)]);
    action(
        &mut m,
        "Reconcile",
        pending.clone().and(authorized.clone()),
        &[(5, Int(1)), (6, Int(1))],
    );
    action(
        &mut m,
        "Refuse",
        pending.and(authorized.negate()),
        &[(5, Int(1))],
    );
    safe(
        &mut m,
        "OnlyScopedAuthorityArchivesHistoricalRecords",
        implies(
            eq(6, 1),
            all([eq(2, 1), eq(4, 0), eq(1, 2).or(all([eq(1, 1), eq(3, 1)]))]),
        ),
    );
    m
}

// A captured transport launch survives later A -> B -> A Settings changes.
// No network fairness or remote availability is assumed; composition is pure.
fn transport_capture(b: bool) -> Model {
    let mut m = model(
        "TransportCapture",
        b,
        &[
            ("settings", 2, 1),
            ("captured", 2, 0),
            ("spawned", 2, 0),
            ("phase", 5, 0),
            ("remembered", 2, 0),
        ],
    );
    for value in 1..=2 {
        action(
            &mut m,
            &format!("Edit_{value}"),
            Bool(true),
            &[(0, Int(value))],
        );
    }
    action(&mut m, "Capture", eq(3, 0), &[(1, Var(0)), (3, Int(1))]);
    action(
        &mut m,
        "Spawn",
        eq(3, 1),
        &[(2, if b { Var(0) } else { Var(1) }), (3, Int(2))],
    );
    action(
        &mut m,
        "RememberSuccess",
        eq(3, 3),
        &[(4, Var(1)), (3, Int(5))],
    );
    action(&mut m, "CompleteSuccess", eq(3, 2), &[(3, Int(3))]);
    action(&mut m, "CompleteFailure", eq(3, 2), &[(3, Int(4))]);
    action(&mut m, "RejectedOrCancelled", eq(3, 1), &[(3, Int(4))]);
    action(&mut m, "RememberFailure", if b { eq(3, 4) } else { Bool(false) }, &[(4, Var(1))]);
    safe(&mut m, "FailureNeverRemembers", implies(eq(3, 4), eq(4, 0)));
    action(
        &mut m,
        "RestartKnownExited",
        all([eq(3, 3).or(eq(3, 4)).or(eq(3, 5)), ne(2, 0)]),
        &[(2, if b { Var(0) } else { Var(1) })],
    );
    safe(&mut m, "CapturedArgv", implies(ne(2, 0), Var(2).eq(Var(1))));
    safe(
        &mut m,
        "RememberOnlyCapturedSuccess",
        implies(ne(4, 0), Var(4).eq(Var(1))),
    );
    m
}

// One live query in a stable generation. Parsing/capture is atomic here;
// reply delivery is weakly fair and uses InputReceipt's existing stable retry ID.
// Queue saturation, terminal byte semantics, OS timing and owner restart are
// implementation witnesses/explicit assumptions, not inferred from this model.
fn terminal_reply_progress(b: bool) -> Model {
    let mut m = model(
        "TerminalReplyProgress",
        b,
        &[("captured", 1, 0), ("queued", 1, 0), ("delivered", 1, 0)],
    );
    action(
        &mut m,
        "LiveQuery",
        eq(0, 0),
        &[(0, Int(1)), (1, Int(i64::from(!b)))],
    );
    action(&mut m, "Deliver", eq(1, 1), &[(1, Int(0)), (2, Int(1))]);
    m.responses.push(Response {
        name: "LiveQueryGetsReply".into(),
        request: eq(0, 1),
        response: eq(2, 1),
    });
    m.weak_fair.push(vec![1]);
    m
}

// Replay reconstructs cells and parser state, never terminal output effects.
// The broken control routes a historical query to the live effect consumer.
fn terminal_replay_quiet(b: bool) -> Model {
    let mut m = model(
        "TerminalReplayQuiet",
        b,
        &[("replayed", 1, 0), ("effects", 1, 0)],
    );
    action(
        &mut m,
        "ReplayQuery",
        eq(0, 0),
        &[(0, Int(1)), (1, Int(i64::from(b)))],
    );
    safe(&mut m, "NoHistoricalTerminalWrite", eq(1, 0));
    m
}

// One physical key; UI turns are atomic. Command may disappear before repeat/release.
fn native_input(b: bool) -> Model {
    let mut m = model(
        "NativeInputOwnership",
        b,
        &[
            ("held_local", 1, 0),
            ("command", 1, 0),
            ("event_local", 1, 0),
            ("emitted", 1, 0),
        ],
    );
    action(
        &mut m,
        "LocalPress",
        Bool(true),
        &[(0, Int(1)), (1, Int(1)), (2, Int(1)), (3, Int(0))],
    );
    action(&mut m, "DropCommand", Bool(true), &[(1, Int(0))]);
    action(
        &mut m,
        "TerminalPress",
        Bool(true),
        &[(0, Int(0)), (1, Int(0)), (2, Int(0)), (3, Int(1))],
    );
    for (name, release) in [("Repeat", false), ("Release", true)] {
        for local in 0..=1 {
            for command in 0..=1 {
                let emits = local == 0 || (b && command == 0);
                action(
                    &mut m,
                    &format!("{name}{local}{command}"),
                    all([eq(0, local), eq(1, command)]),
                    &[
                        (0, Int(if release { 0 } else { local })),
                        (2, Int(local)),
                        (3, Int(i64::from(emits))),
                    ],
                );
            }
        }
    }
    safe(
        &mut m,
        "LocalLifecycleCannotEmit",
        implies(eq(2, 1), eq(3, 0)),
    );
    m
}

fn document_feed(b: bool) -> Model {
    let mut m = model(
        "DocumentFeed",
        b,
        &[
            ("open", 1, 0),
            ("ready", 1, 0),
            ("generation", 3, 1),
            ("captured", 3, 0),
            ("emissions", 2, 0),
            ("queue_ready", 1, 1),
            ("bad", 1, 0),
        ],
    );
    action(
        &mut m,
        "Open",
        eq(0, 0),
        &[(0, Int(1)), (1, Int(0)), (3, Var(2)), (4, Int(0))],
    );
    action(&mut m, "LoadBody", eq(0, 1), &[(1, Int(1))]);
    action(
        &mut m,
        "Restart",
        Var(2).lt(Int(3)),
        &[(2, Var(2).plus(Int(1)))],
    );
    action(&mut m, "QueueFull", Bool(true), &[(5, Int(0))]);
    action(&mut m, "QueueReady", Bool(true), &[(5, Int(1))]);
    action(&mut m, "Copy", all([eq(0, 1), eq(1, 1)]), &[]);
    action(&mut m, "Close", eq(0, 1), &[(0, Int(0))]);
    let current = all([eq(0, 1), eq(1, 1), Var(2).eq(Var(3)), eq(5, 1)]);
    action(
        &mut m,
        "Feed",
        current,
        &[(0, Int(0)), (4, Var(4).plus(Int(1)))],
    );
    if b {
        action(
            &mut m,
            "FeedReplacement",
            all([eq(0, 1), eq(1, 1), Var(2).eq(Var(3)).negate()]),
            &[(6, Int(1))],
        );
        action(
            &mut m,
            "RepeatAfterClose",
            all([eq(0, 0), eq(4, 1)]),
            &[(4, Int(2))],
        );
    }
    safe(&mut m, "AtMostOneFeedPerOpen", Var(4).lt(Int(2)));
    no_bad(&mut m, 6);
    m
}

fn graph_query_cursors(b: bool) -> Model {
    let mut m = model(
        "GraphQueryCursors",
        b,
        &[("e2e_seen", 1, 0), ("decision_seen", 1, 0), ("bad", 1, 0)],
    );
    action(&mut m, "E2ePage", eq(0, 0), &[(0, Int(1))]);
    action(&mut m, "DecisionPage", eq(1, 0), &[(1, Int(1))]);
    if b {
        action(
            &mut m,
            "SharedCursorSet",
            all([eq(0, 1), eq(1, 0)]),
            &[(2, Int(1))],
        );
    }
    no_bad(&mut m, 2);
    m
}

fn pane_raster_publication(b: bool) -> Model {
    let mut m = model(
        "PaneRasterPublication",
        b,
        &[
            ("source", 2, 1),
            ("stage", 2, 0),
            ("built_source", 2, 0),
            ("published_source", 2, 0),
            ("published_pixels", 2, 0),
            ("published", 1, 0),
        ],
    );
    for source in [1, 2] {
        action(
            &mut m,
            if source == 1 { "SourceA" } else { "SourceB" },
            ne(0, source),
            &[(0, Int(source))],
        );
    }
    action(
        &mut m,
        "PrepareNewTexture",
        eq(1, 0),
        &[(1, Int(1)), (2, Var(0))],
    );
    action(&mut m, "SubmitRaster", eq(1, 1), &[(1, Int(2))]);
    action(
        &mut m,
        "PublishSubmittedRaster",
        all([eq(1, 2), Var(0).eq(Var(2))]),
        &[(1, Int(0)), (3, Var(2)), (4, Var(2)), (5, Int(1))],
    );
    action(
        &mut m,
        "DiscardStalePreparation",
        all([ne(1, 0), Var(0).eq(Var(2)).negate()]),
        &[(1, Int(0))],
    );
    if b {
        action(
            &mut m,
            "RepaintPublishedTextureInPlace",
            all([eq(5, 1), Var(0).eq(Var(3)).negate()]),
            &[(4, Var(0))],
        );
    }
    safe(
        &mut m,
        "PublishedPixelsStayWithTheirSource",
        implies(eq(5, 1), Var(3).eq(Var(4))),
    );
    m
}

pub fn cases() -> Vec<(Model, &'static str)> {
    let families: [fn(bool) -> Model; 30] = [
        pane_raster_publication,
        terminal_reply_progress,
        terminal_replay_quiet,
        seat_models::diagnostics,
        transport_capture,
        seat_models::recovery,
        graph_query_cursors,
        document_feed,
        native_input,
        sidebar_reveal,
        async_render_preparation,
        render_line_plan,
        sidebar_accordions,
        selection_gesture,
        landing_verification,
        dirty_wake,
        notification,
        notification_models::recovery,
        notification_models::lifecycle,
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
    let mut cases = families
        .into_iter()
        .flat_map(|family| {
            let good = family(false);
            let temporal = !good.responses.is_empty();
            [
                (good, "pass"),
                (family(true), if temporal { "liveness" } else { "safety" }),
            ]
        })
        .collect::<Vec<_>>();
    cases.extend([
        (import_staging(false), "pass"),
        (import_staging(true), "safety"),
        (historical_reconcile(false), "pass"),
        (historical_reconcile(true), "safety"),
    ]);
    cases
}
