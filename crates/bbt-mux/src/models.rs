// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Host-side plugin contract models (feature `model`), shipped with this
//! package as `models/*.tla` and `models/*.cfg`: activation, live
//! connections, the hook gate, setup and the hook queue many plugins share.
//! Each family is the model and its
//! `Broken` control (one guard removed), checked by `make test-formal` with
//! the native `bboxy450-tlc` checker and Java TLC.
use bboxy450_tlc::Expr::{Int, Var};
#[allow(unused_imports)]
use bboxy450_tlc::build::{action, all, eq, implies, model, ne, no_bad, safe};
#[allow(unused_imports)]
use bboxy450_tlc::{Expr, Model, Response};

/// Every shipped family of this crate.
mod hook_budget;
mod managed_workspace;
mod transport_launch;
mod workspace_ready;

pub fn families() -> [fn(bool) -> Model; 19] {
    [
        plugin_activation,
        plugin_connection,
        plugin_hook_gate,
        plugin_setup,
        plugin_hook_queue,
        hook_budget::plugin_hook_budget,
        transport_launch::plugin_transport_launch,
        transport_launch::plugin_transport_progress,
        workspace_ready::workspace_ready,
        crate::mcp::live_models::live_catalogue,
        crate::mcp::live_models::delegation_pointer,
        crate::mcp::live_models::absent_seat_close,
        crate::mcp::oracle_hint_model::oracle_in_turn,
        managed_workspace::managed_workspace_create,
        managed_workspace::managed_workspace_writer,
        managed_workspace::managed_workspace_fence,
        managed_workspace::managed_workspace_release,
        managed_workspace::managed_workspace_ownership,
        managed_workspace::managed_workspace_provider_lease,
    ]
}

// The per-root hook queue (plugins/hooks.rs, HOOK_QUEUE) shared by many
// plugins: two plugins here, a FIFO of two slots, three offers. Each queued
// delivery carries its plugin and that plugin's activation generation; a
// disable/re-enable supersedes a generation at any time; a full queue drops
// and counts the offer. The head is delivered only while its generation is
// current, else skipped as `stale`. The broken control delivers a stale head.
pub fn plugin_hook_queue(b: bool) -> Model {
    use bboxy450_tlc::Expr::Bound;
    const PLUGINS: usize = 2;
    const SLOTS: usize = 2;
    let (generation, len, plugin, slot_generation, offers, bad) = (0, 2, 3, 5, 7, 8);
    let mut m = model(
        "PluginHookQueue",
        b,
        &[
            ("generation_a", 1, 0),
            ("generation_b", 1, 0),
            ("len", 2, 0),
            ("slot0_plugin", 1, 0),
            ("slot1_plugin", 1, 0),
            ("slot0_generation", 1, 0),
            ("slot1_generation", 1, 0),
            ("offers", 3, 0),
            ("bad", 1, 0),
        ],
    );
    let current =
        |slot: Expr| Expr::pick(generation, PLUGINS, Expr::pick(plugin, SLOTS, slot.clone()));
    for p in 0..PLUGINS {
        // Enqueue at index `len`: every slot keeps its value unless it is that index.
        let mut updates = vec![
            (len, Var(len).plus(Int(1))),
            (offers, Var(offers).plus(Int(1))),
        ];
        for slot in 0..SLOTS {
            let here = Var(len).eq(Int(slot as i64));
            updates.push((
                plugin + slot,
                here.clone().ite(Int(p as i64), Var(plugin + slot)),
            ));
            updates.push((
                slot_generation + slot,
                here.ite(Var(generation + p), Var(slot_generation + slot)),
            ));
        }
        action(
            &mut m,
            &format!("Offer{p}"),
            all([Var(len).lt(Int(SLOTS as i64)), Var(offers).lt(Int(3))]),
            &updates,
        );
        action(
            &mut m,
            &format!("DropWhenFull{p}"),
            all([eq(len, SLOTS as i64), Var(offers).lt(Int(3))]),
            &[(offers, Var(offers).plus(Int(1)))],
        );
        action(
            &mut m,
            &format!("Supersede{p}"),
            eq(generation + p, 0),
            &[(generation + p, Int(1))],
        );
    }
    // Pop the head: the second slot moves up and the vacated slot clears.
    let pop = |extra: Option<(usize, Expr)>| {
        let mut updates = vec![
            (len, Var(len).minus(Int(1))),
            (plugin, Var(plugin + 1)),
            (slot_generation, Var(slot_generation + 1)),
            (plugin + 1, Int(0)),
            (slot_generation + 1, Int(0)),
        ];
        updates.extend(extra);
        updates
    };
    let head_current = Var(slot_generation).eq(current(Int(0)));
    let queued = Int(0).lt(Var(len));
    action(
        &mut m,
        "DeliverCurrentHead",
        all([queued.clone(), head_current.clone()]),
        &pop(None),
    );
    action(
        &mut m,
        "SkipStaleHead",
        all([queued.clone(), head_current.clone().negate()]),
        &pop(None),
    );
    if b {
        action(
            &mut m,
            "DeliverStaleHead",
            all([queued, head_current.negate()]),
            &pop(Some((bad, Int(1)))),
        );
    }
    no_bad(&mut m, bad);
    // Every queued delivery's generation is at most its plugin's current one.
    safe(
        &mut m,
        "QueuedGenerationsNeverAhead",
        Expr::forall(
            0,
            SLOTS as i64 - 1,
            implies(
                Bound(0).lt(Var(len)),
                Expr::pick(slot_generation, SLOTS, Bound(0)).le(current(Bound(0))),
            ),
        ),
    );
    m
}

// Plugin activation (plugins/registry.rs): an inspection result is accepted
// only for the source digest, approval and activation generation it ran for.
pub fn plugin_activation(b: bool) -> Model {
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

pub fn plugin_connection(b: bool) -> Model {
    let mut m = model(
        "PluginConnection",
        b,
        &[
            ("generation", 4, 1),
            ("enabled", 1, 1),
            ("connection", 1, 0),
            ("connection_generation", 4, 0),
            ("call", 1, 0),
            ("call_generation", 4, 0),
            ("reconcile_pending", 1, 0),
            ("bad", 1, 0),
        ],
    );
    let current = all([eq(1, 1), Var(3).eq(Var(0))]);
    action(
        &mut m,
        "OpenConnection",
        all([eq(2, 0), eq(1, 1)]),
        &[(2, Int(1)), (3, Var(0))],
    );
    action(
        &mut m,
        "ReplaceStaleConnection",
        all([eq(2, 1), eq(1, 1), Var(3).eq(Var(0)).negate()]),
        &[(3, Var(0))],
    );
    action(
        &mut m,
        "StartCall",
        all([eq(4, 0), eq(2, 1), current.clone()]),
        &[(4, Int(1)), (5, Var(3))],
    );
    let bump = Var(0).plus(Int(1));
    action(
        &mut m,
        "Disable",
        all([eq(1, 1), Var(0).lt(Int(4))]),
        &[(1, Int(0)), (0, bump.clone()), (6, Int(1))],
    );
    action(
        &mut m,
        "Enable",
        all([eq(1, 0), Var(0).lt(Int(4))]),
        &[(1, Int(1)), (0, bump), (6, Int(1))],
    );
    action(
        &mut m,
        "ReconcileKeep",
        all([eq(6, 1), current.clone().or(eq(2, 0))]),
        &[(6, Int(0))],
    );
    action(
        &mut m,
        "ReconcileClose",
        all([eq(6, 1), eq(2, 1), current.clone().negate()]),
        &[(6, Int(0)), (2, Int(0))],
    );
    action(
        &mut m,
        "CallFailsOnClosedConnection",
        all([eq(4, 1), eq(2, 0)]),
        &[(4, Int(0))],
    );
    let call_current = all([eq(1, 1), Var(5).eq(Var(0))]);
    action(
        &mut m,
        "DeliverCurrentResult",
        all([eq(4, 1), eq(2, 1), call_current.clone()]),
        &[(4, Int(0))],
    );
    action(
        &mut m,
        "RefuseSupersededResult",
        all([eq(4, 1), eq(2, 1), call_current.clone().negate()]),
        &[(4, Int(0)), (2, Int(0))],
    );
    if b {
        // Without the completion recheck a superseded generation's result is delivered.
        action(
            &mut m,
            "DeliverWithoutGenerationCheck",
            all([eq(4, 1), eq(2, 1), call_current.negate()]),
            &[(4, Int(0)), (7, Int(1))],
        );
    }
    no_bad(&mut m, 7);
    safe(
        &mut m,
        "NoConnectionSurvivesDisable",
        implies(all([eq(1, 0), eq(6, 0)]), eq(2, 0)),
    );
    m
}

// The hook shim's wait for a plugin decision (adapters/hook.rs, seat_socket
// HookDecide). The host's gate file lags the real blocking subscriptions; the
// shim waits only when the gate lists the event, at most three ticks (the
// ceiling minus a margin), while the host decides by tick two. The host may
// be lost at any point. One tick advances the shim's and the host's clocks.
pub fn plugin_hook_gate(b: bool) -> Model {
    let mut m = model(
        "PluginHookGate",
        b,
        &[
            ("blocking", 1, 0),
            ("gate", 1, 0),
            ("shim", 3, 0),
            ("clock", 3, 0),
            ("host", 3, 0),
            ("reply", 2, 0),
            ("plugin", 2, 0),
            ("fail_closed", 1, 0),
            ("exit", 2, 0),
        ],
    );
    // Enable/disable or re-trust changes the blocking subscriptions; the
    // host rewrites the gate file afterwards.
    action(&mut m, "GrantBlock", eq(0, 0), &[(0, Int(1))]);
    action(&mut m, "RevokeBlock", eq(0, 1), &[(0, Int(0))]);
    action(
        &mut m,
        "WriteGate",
        Var(1).eq(Var(0)).negate(),
        &[(1, Var(0))],
    );
    action(
        &mut m,
        "ChooseFailClosed",
        all([eq(2, 0), eq(7, 0)]),
        &[(7, Int(1))],
    );
    // Not listed: the unchanged fire-and-forget path, never a veto.
    action(
        &mut m,
        "ShimFastPath",
        all([eq(2, 0), eq(1, 0)]),
        &[(2, Int(3)), (8, Int(1))],
    );
    action(
        &mut m,
        "ShimAsks",
        all([eq(2, 0), eq(1, 1)]),
        &[(2, Int(2)), (4, Int(1))],
    );
    action(
        &mut m,
        "HostNoBlocker",
        all([eq(4, 1), eq(0, 0)]),
        &[(4, Int(2)), (5, Int(1))],
    );
    action(
        &mut m,
        "PluginAllows",
        all([eq(4, 1), eq(0, 1), eq(6, 0)]),
        &[(6, Int(1))],
    );
    action(
        &mut m,
        "PluginDenies",
        all([eq(4, 1), eq(0, 1), eq(6, 0)]),
        &[(6, Int(2))],
    );
    action(
        &mut m,
        "HostAccepts",
        all([eq(4, 1), ne(6, 0), Var(3).lt(Int(2))]),
        &[(4, Int(2)), (5, Var(6))],
    );
    let host_late = all([eq(4, 1), Var(3).lt(Int(2)).negate()]);
    action(
        &mut m,
        "HostTimeoutFailsOpen",
        all([host_late.clone(), eq(7, 0)]),
        &[(4, Int(2)), (5, Int(1))],
    );
    action(
        &mut m,
        "HostTimeoutFailsClosed",
        all([host_late, eq(7, 1)]),
        &[(4, Int(2)), (5, Int(2))],
    );
    action(
        &mut m,
        "HostLost",
        all([eq(2, 2), eq(4, 1).or(eq(4, 2))]),
        &[(4, Int(3))],
    );
    action(
        &mut m,
        "Tick",
        all([eq(2, 2), Var(3).lt(Int(3))]),
        &[(3, Var(3).plus(Int(1)))],
    );
    action(
        &mut m,
        "ShimReceives",
        all([eq(2, 2), eq(4, 2)]),
        &[(2, Int(3)), (8, Var(5))],
    );
    // The broken control waits for the reply without a bound.
    if !b {
        let expired = all([eq(2, 2), eq(3, 3), ne(4, 2)]);
        action(
            &mut m,
            "ShimTimeoutFailsOpen",
            all([expired.clone(), eq(7, 0)]),
            &[(2, Int(3)), (8, Int(1))],
        );
        action(
            &mut m,
            "ShimTimeoutFailsClosed",
            all([expired, eq(7, 1)]),
            &[(2, Int(3)), (8, Int(2))],
        );
    }
    safe(
        &mut m,
        "DenyOnlyByPluginOrPolicy",
        implies(eq(8, 2), eq(6, 2).or(eq(7, 1))),
    );
    safe(&mut m, "FastPathNeverVetoes", implies(eq(4, 0), ne(8, 2)));
    safe(&mut m, "FastPathNeverWaits", implies(eq(4, 0), eq(3, 0)));
    m.responses.push(Response {
        name: "ShimAlwaysAnswers".into(),
        request: eq(2, 2),
        response: eq(2, 3),
    });
    // Time, the host and the shim progress; grants, loss and plugin replies do not.
    let mut fair = vec![];
    for (index, action) in m.actions.iter().enumerate() {
        if [
            "WriteGate",
            "HostNoBlocker",
            "HostAccepts",
            "HostTimeoutFailsOpen",
            "HostTimeoutFailsClosed",
            "Tick",
            "ShimReceives",
            "ShimTimeoutFailsOpen",
            "ShimTimeoutFailsClosed",
        ]
        .contains(&action.name.as_str())
        {
            fair.push(index);
        }
    }
    for index in fair {
        m.weak_fair.push(vec![index]);
    }
    m
}

// Plugin setup (plugins/setup.rs): status, then a login in a bare login
// terminal when not logged in, then status again. The login may start a
// child and may never end; its budget is three ticks. Ending the login ends
// its whole process group, and a result is recorded only for an unchanged pin.
// The login runs in a visible login row (logic.rs plugin_login): the row is
// open exactly while the login runs. Its setup job (the owner) can be lost
// with a stopped logic owner; the next owner's attach then ends the orphaned
// login's process group and closes its row (phase 5), and nothing is recorded.
pub fn plugin_setup(b: bool) -> Model {
    let mut m = model(
        "PluginSetup",
        b,
        &[
            ("phase", 5, 0),
            ("login", 1, 0),
            ("child", 1, 0),
            ("clock", 3, 0),
            ("pin_changed", 1, 0),
            ("recorded", 1, 0),
            ("bad", 1, 0),
            ("row", 1, 0),
            ("owner", 1, 1),
        ],
    );
    action(&mut m, "StartStatus", eq(0, 0), &[(0, Int(1))]);
    action(&mut m, "StatusLoggedIn", eq(0, 1), &[(0, Int(4))]);
    action(
        &mut m,
        "StatusLoggedOut",
        eq(0, 1),
        &[(0, Int(2)), (1, Int(1)), (3, Int(0)), (7, Int(1))],
    );
    action(
        &mut m,
        "LoginStartsChild",
        all([eq(0, 2), eq(1, 1)]),
        &[(2, Int(1))],
    );
    action(
        &mut m,
        "LoginExits",
        all([eq(0, 2), eq(1, 1)]),
        &[(1, Int(0))],
    );
    action(
        &mut m,
        "Tick",
        all([eq(0, 2), eq(1, 1), Var(3).lt(Int(3))]),
        &[(3, Var(3).plus(Int(1)))],
    );
    // The broken control ends only the login, not its process group (the
    // supervisor's bare-terminal group cleanup), on exit, at the budget and
    // when an orphaned login is cleaned up on attach.
    let end = |phase: i64| -> Vec<(usize, Expr)> {
        let mut updates = vec![(1, Int(0)), (7, Int(0)), (0, Int(phase))];
        if !b {
            updates.push((2, Int(0)));
        }
        updates
    };
    let ended = all([eq(0, 2), eq(8, 1), eq(1, 0).or(eq(3, 3))]);
    action(&mut m, "EndProcessGroup", ended, &end(3));
    action(
        &mut m,
        "LoseOwner",
        all([eq(0, 2), eq(8, 1)]),
        &[(8, Int(0))],
    );
    action(
        &mut m,
        "CleanUpOrphanOnAttach",
        all([eq(0, 2), eq(8, 0)]),
        &end(5),
    );
    action(&mut m, "StatusAgain", eq(0, 3), &[(0, Int(4))]);
    action(&mut m, "Retrust", eq(4, 0), &[(4, Int(1))]);
    action(
        &mut m,
        "RecordCurrentPin",
        all([eq(0, 4), eq(5, 0), eq(4, 0), eq(8, 1)]),
        &[(5, Int(1))],
    );
    no_bad(&mut m, 6);
    safe(
        &mut m,
        "NoProcessOutlivesTheLogin",
        implies(Var(0).lt(Int(3)).negate(), all([eq(1, 0), eq(2, 0)])),
    );
    safe(&mut m, "LoginWithinBudget", Var(3).le(Int(3)));
    safe(
        &mut m,
        "RowOpenOnlyWhileLoginRuns",
        implies(eq(7, 1), eq(0, 2)),
    );
    safe(
        &mut m,
        "NothingRecordedWithoutOwner",
        implies(eq(5, 1), eq(8, 1)),
    );
    m
}
