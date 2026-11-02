// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//
// The GitKB plugin's contract models, exported as models/*.tla and *.cfg
// (bboxy450_tlc::build::artifacts). This file sits outside src/ and has no
// effect on the plugin binary: the formal gate (`crates/bboxy450-term/tests/
// formal_contracts/gitkb_models.rs`) includes it, checks every family with
// the native checker and Java TLC, and fails when a shipped file drifts.
// `BBOXY_UPDATE_MODELS=1 cargo test -p bboxy450-term --test formal_contracts
// shipped_gitkb` rewrites them.
//
// Each model describes what the code does today, cited file:line at the
// commit these models were written against; the broken control removes the
// one guard its property depends on.
use bboxy450_tlc::Expr::{Int, Var};
use bboxy450_tlc::Model;
use bboxy450_tlc::Response;
use bboxy450_tlc::build::{action, all, eq, implies, model, ne, safe};

/// Every shipped family of the GitKB plugin.
pub fn families() -> [fn(bool) -> Model; 3] {
    [index_freshness, paging, attribution]
}

/// `GitKbIndexFreshness` (or its broken control): the committed index is
/// labelled by the store signal its export was taken at, the persisted
/// snapshot carries the same label, and a call is served only the layer whose
/// label equals the signal it read.
///
/// Store generations stand for store signals (`index.rs:64` `store_signal`,
/// stat identity of the manifest, documents, commits and tips: a native
/// commit changes it, reads and workspace writes do not). Signal `g + 1`
/// names generation `g`; 0 is none. A label is what the code keys a layer by;
/// a content is the generation its export really read.
///
/// - A call reads the signal (`store.rs:517`) and is served the committed
///   layer only when its label equals it (`index.rs:807`, `store.rs:521`);
///   otherwise it falls back to the backend path and starts one background
///   build when none runs (`index.rs:810-814`).
/// - A build reads the signal before the export (`index.rs:1066`), exports
///   (`index.rs:1067-1069`) and publishes only when the signal is unchanged
///   (`index.rs:1070`), otherwise discards and retries (`index.rs:1071-1072`;
///   ATTEMPTS bounds retries, abstracted here). It saves the snapshot before
///   publishing (`index.rs:826-830`, `index.rs:849`).
/// - A restarted process keeps the snapshot and loads it only for exactly the
///   current signal (`index.rs:672`, `index.rs:1019-1021`).
///
/// The broken control publishes without the post-export re-check: a commit
/// during the export labels older content with a newer signal, or newer
/// content with an older one, and fails safety.
pub fn index_freshness(broken: bool) -> Model {
    let mut m = model(
        "GitKbIndexFreshness",
        broken,
        &[
            ("store", 2, 0),
            ("committed", 3, 0),
            ("committed_content", 3, 0),
            ("snapshot", 3, 0),
            ("snapshot_content", 3, 0),
            ("phase", 3, 0),
            ("before", 3, 0),
            ("exported", 3, 0),
            ("call", 3, 0),
            ("served", 3, 0),
            ("served_for", 3, 0),
        ],
    );
    let signal = || Var(0).plus(Int(1));
    action(
        &mut m,
        "StoreChange",
        Var(0).lt(Int(2)),
        &[(0, Var(0).plus(Int(1)))],
    );
    action(
        &mut m,
        "Restart",
        eq(1, 0).negate().or(ne(5, 0)).or(ne(8, 0)),
        &[
            (1, Int(0)),
            (2, Int(0)),
            (5, Int(0)),
            (6, Int(0)),
            (7, Int(0)),
            (8, Int(0)),
        ],
    );
    action(
        &mut m,
        "LoadSnapshot",
        all([eq(1, 0), eq(5, 0), Var(3).eq(signal())]),
        &[(1, Var(3)), (2, Var(4))],
    );
    action(&mut m, "CallReadsSignal", eq(8, 0), &[(8, signal())]);
    action(
        &mut m,
        "CallServed",
        all([ne(8, 0), Var(1).eq(Var(8))]),
        &[(9, Var(2)), (10, Var(8)), (8, Int(0))],
    );
    action(
        &mut m,
        "CallFallsBack",
        all([ne(8, 0), Var(1).eq(Var(8)).negate()]),
        &[(8, Int(0)), (5, eq(5, 0).ite(Int(1), Var(5)))],
    );
    action(
        &mut m,
        "BuildReadsSignal",
        eq(5, 1),
        &[(5, Int(2)), (6, signal())],
    );
    action(
        &mut m,
        "BuildExports",
        eq(5, 2),
        &[(5, Int(3)), (7, signal())],
    );
    let unchanged = signal().eq(Var(6));
    action(
        &mut m,
        "BuildSavesAndPublishes",
        if broken {
            eq(5, 3)
        } else {
            all([eq(5, 3), unchanged.clone()])
        },
        &[
            (3, Var(6)),
            (4, Var(7)),
            (1, Var(6)),
            (2, Var(7)),
            (5, Int(0)),
            (6, Int(0)),
            (7, Int(0)),
        ],
    );
    action(
        &mut m,
        "BuildDiscards",
        all([eq(5, 3), unchanged.negate()]),
        &[(5, Int(1)), (6, Int(0)), (7, Int(0))],
    );
    safe(&mut m, "CommittedLabelIsItsContent", Var(1).eq(Var(2)));
    safe(&mut m, "SnapshotLabelIsItsContent", Var(3).eq(Var(4)));
    safe(
        &mut m,
        "ServedContentIsTheSignalTheCallRead",
        Var(9).eq(Var(10)),
    );
    m
}

/// `GitKbPaging` (or its broken control): `list`/`search`/`graph` pages over
/// one catalog snapshot. The cursor stores the query key, the snapshot and an
/// offset (`methods.rs:308-314`); a later page continues only for the same
/// key and snapshot, else refuses `stale_cursor` (`methods.rs:187-192`).
/// `fill` examines at least one record before the time reserve may stop it
/// (`methods.rs:224`) and advances the offset past every examined record
/// (`methods.rs:239`; a page cut by the byte budget returns before advancing
/// past the item it could not admit, `methods.rs:234`). Every page mints a
/// fresh token (`store.rs:412-421`).
///
/// Three records a, b, c; one source change inserts record n before them
/// (positions shift by one). Pages examine one or two records (any limit of
/// at least that). Properties: no record twice and none skipped by a walk
/// that ends (`next` null), and every walk ends or is refused.
///
/// The broken control drops the at-least-one-record guard: a page cut by the
/// reserve before its first record returns the same offset under a new token
/// forever, and fails liveness.
pub fn paging(broken: bool) -> Model {
    let mut m = model(
        "GitKbPaging",
        broken,
        &[
            ("snapshot", 1, 0),
            ("cursor_snapshot", 1, 0),
            ("offset", 4, 0),
            ("walk", 3, 0),
            ("seen_a", 2, 0),
            ("seen_b", 2, 0),
            ("seen_c", 2, 0),
            ("seen_n", 2, 0),
            ("token", 1, 0),
        ],
    );
    let len = || Var(1).plus(Int(3));
    let flip = || (8, Int(1).minus(Var(8)));
    action(
        &mut m,
        "Begin",
        eq(3, 0),
        &[(3, Int(1)), (1, Var(0)), (2, Int(0))],
    );
    action(&mut m, "SourceChange", eq(0, 0), &[(0, Int(1))]);
    let current = || all([eq(3, 1), Var(1).eq(Var(0))]);
    let mut pages = Vec::new();
    for k in [1, 2] {
        let end = Var(2).plus(Int(k));
        let covers = |position: bboxy450_tlc::Expr| {
            Var(2)
                .le(position.clone())
                .and(position.lt(Var(2).plus(Int(k))))
        };
        let seen =
            |i: usize, covered: bboxy450_tlc::Expr| (i, covered.ite(Var(i).plus(Int(1)), Var(i)));
        let mut updates = vec![
            (2, end.clone()),
            (3, end.clone().eq(len()).ite(Int(2), Int(1))),
            flip(),
        ];
        for (record, index) in [(0, 4), (1, 5), (2, 6)] {
            updates.push(seen(index, covers(Var(1).plus(Int(record)))));
        }
        updates.push(seen(7, eq(1, 1).and(eq(2, 0))));
        pages.push(m.actions.len());
        action(
            &mut m,
            &format!("Page{k}"),
            all([current(), end.le(len())]),
            &updates,
        );
    }
    pages.push(m.actions.len());
    action(
        &mut m,
        "RefuseStaleCursor",
        all([eq(3, 1), Var(1).eq(Var(0)).negate()]),
        &[(3, Int(3))],
    );
    if broken {
        pages.push(m.actions.len());
        action(
            &mut m,
            "PageWithoutARecord",
            all([current(), Var(2).lt(len())]),
            &[flip()],
        );
    }
    safe(
        &mut m,
        "NoRecordTwice",
        all([
            Var(4).le(Int(1)),
            Var(5).le(Int(1)),
            Var(6).le(Int(1)),
            Var(7).le(Int(1)),
        ]),
    );
    safe(
        &mut m,
        "NoRecordSkipped",
        implies(
            eq(3, 2),
            all([eq(4, 1), eq(5, 1), eq(6, 1), implies(eq(1, 1), eq(7, 1))]),
        ),
    );
    m.responses.push(Response {
        name: "EveryWalkEnds".into(),
        request: eq(3, 1),
        response: eq(3, 2).or(eq(3, 3)),
    });
    // The host keeps calling with `next` until it is null or refused.
    m.weak_fair = vec![pages];
    m
}

/// `GitKbAttribution` (or its broken control): what reaches a `git-kb`
/// child's environment. Every child is spawned with `env_clear()` and the
/// backend's own list (`backend.rs:222-225`), which starts scrubbed
/// (`backend.rs:55-65`, `backend.rs:79`) and is extended only while binding
/// the root, before any call (`main.rs:93`, `main.rs:152` via `set_env`,
/// `backend.rs:84-87`). Index builds and workspace preparation run on clones
/// of that backend with detached contexts (`index.rs:680`, `index.rs:813`,
/// `index.rs:732`, `index.rs:1062`).
///
/// No call writes its seat attribution into that list: a call's attribution
/// reaches only its own child, added per spawn from its context
/// (`backend.rs:226`), and a detached context carries none. The model's
/// properties are the ones that per-call delivery must keep: a call's child
/// carries no attribution or its own call's, and an index child carries none.
/// Two seats A and B (1, 2).
///
/// The broken control has a call write its attribution into the shared
/// backend environment (a `set_env` while serving it): the next call's child
/// and a later index build inherit it, and it fails safety.
pub fn attribution(broken: bool) -> Model {
    let mut m = model(
        "GitKbAttribution",
        broken,
        &[
            ("backend_env", 2, 0),
            ("call", 2, 0),
            ("child", 1, 0),
            ("child_attribution", 2, 0),
            ("child_call", 2, 0),
            ("index_child", 1, 0),
            ("index_attribution", 2, 0),
        ],
    );
    for seat in [1, 2] {
        let mut updates = vec![(1, Int(seat))];
        if broken {
            updates.push((0, Int(seat)));
        }
        action(&mut m, &format!("Call{seat}"), eq(1, 0), &updates);
    }
    action(
        &mut m,
        "CallStartsChild",
        all([ne(1, 0), eq(2, 0)]),
        &[(2, Int(1)), (3, Var(0)), (4, Var(1))],
    );
    action(
        &mut m,
        "ChildExits",
        eq(2, 1),
        &[(2, Int(0)), (3, Int(0)), (4, Int(0))],
    );
    action(
        &mut m,
        "CallEnds",
        all([ne(1, 0), eq(2, 0)]),
        &[(1, Int(0))],
    );
    action(
        &mut m,
        "IndexStartsChild",
        eq(5, 0),
        &[(5, Int(1)), (6, Var(0))],
    );
    action(
        &mut m,
        "IndexChildExits",
        eq(5, 1),
        &[(5, Int(0)), (6, Int(0))],
    );
    safe(
        &mut m,
        "ChildCarriesOnlyItsOwnCallsAttribution",
        implies(eq(2, 1), eq(3, 0).or(Var(3).eq(Var(4)))),
    );
    safe(
        &mut m,
        "IndexChildrenAreUnattributed",
        implies(eq(5, 1), eq(6, 0)),
    );
    m
}
