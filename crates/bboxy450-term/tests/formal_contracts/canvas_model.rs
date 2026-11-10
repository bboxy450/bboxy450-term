// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Three immutable batches, two seats and the operator, one durable canvas log.
//! Conflicting operations follow appended order, never network delivery order.
use super::*;

const LOG: usize = 0;
const DELIVERIES: usize = 3;
const ACK: usize = 6;
const SEQ: usize = 9;
const PICTURE_A: usize = 10;
const PICTURE_B: usize = 11;
const READY: usize = 12;
const RESTARTS: usize = 13;

fn seen(batch: i64) -> Expr {
    eq(LOG, batch).or(eq(LOG + 1, batch)).or(eq(LOG + 2, batch))
}

// Replay is computed independently from the complete durable log. A=0 means
// absent, A=1 original, A=2 changed; B is the second element's presence bit.
fn replay_a() -> Expr {
    (LOG..LOG + 3).fold(Int(0), |picture, slot| {
        eq(slot, 1).ite(
            Int(1),
            eq(slot, 2).ite(
                picture.clone().eq(Int(0)).ite(Int(0), Int(2)),
                eq(slot, 3).ite(Int(0), picture),
            ),
        )
    })
}

fn convergence(broken: bool) -> Model {
    let mut m = model(
        "CanvasConvergence",
        broken,
        &[
            ("log_first", 3, 0),
            ("log_second", 3, 0),
            ("log_third", 3, 0),
            ("seat_a_deliveries", 2, 0),
            ("seat_b_deliveries", 2, 0),
            ("operator_deliveries", 2, 0),
            ("seat_a_acknowledged", 1, 0),
            ("seat_b_acknowledged", 1, 0),
            ("operator_acknowledged", 1, 0),
            ("seq", 6, 0),
            ("picture_a", 2, 0),
            ("picture_b", 1, 0),
            ("ready", 1, 1),
            ("restarts", 2, 0),
        ],
    );
    // A: add A (expect_seq 0). B: add B + change A (expect_seq 0).
    // Operator: remove A (expect_seq 1). B's stale expectation deliberately
    // does not gate append: its add must survive after A advanced the log.
    // Unknown-id changes/removes refuse before append; after removal, the
    // retained identity makes a late Change/Remove a durable no-op.
    for batch in 1..=3 {
        let index = batch as usize - 1;
        let pending = all([eq(READY, 1), Var(DELIVERIES + index).lt(Int(2))]);
        let known = if batch == 1 { Bool(true) } else { seen(1) };
        let absent = seen(batch).negate();
        for slot in 0..3 {
            let free_prefix = all((0..slot).map(|i| eq(LOG + i, 0).negate()));
            let guard = all([
                pending.clone(),
                known.clone(),
                if broken { Bool(true) } else { absent.clone() },
                eq(LOG + slot, 0),
                free_prefix,
            ]);
            let a = match batch {
                1 => Int(1),
                2 => eq(PICTURE_A, 0).ite(Int(0), Int(2)),
                _ => Int(0),
            };
            action(
                &mut m,
                &format!("AppendBatch{batch}At{slot}"),
                guard,
                &[
                    (LOG + slot, Int(batch)),
                    (DELIVERIES + index, Var(DELIVERIES + index).plus(Int(1))),
                    (SEQ, Var(SEQ).plus(Int(if batch == 2 { 2 } else { 1 }))),
                    (PICTURE_A, a),
                    (PICTURE_B, if batch == 2 { Int(1) } else { Var(PICTURE_B) }),
                ],
            );
        }
        if !broken {
            action(
                &mut m,
                &format!("DeduplicateBatch{batch}"),
                pending.clone().and(seen(batch)),
                &[(DELIVERIES + index, Var(DELIVERIES + index).plus(Int(1)))],
            );
        }
        action(
            &mut m,
            &format!("RefuseUnknownBatch{batch}"),
            pending.and(known.negate()),
            &[(DELIVERIES + index, Var(DELIVERIES + index).plus(Int(1)))],
        );
        let ack_action = m.actions.len();
        action(
            &mut m,
            &format!("AcknowledgeBatch{batch}"),
            all([eq(READY, 1), seen(batch), eq(ACK + index, 0)]),
            &[(ACK + index, Int(1))],
        );
        m.weak_fair.push(vec![ack_action]);
        let count = (LOG..LOG + 3).fold(Int(0), |sum, slot| {
            sum.plus(eq(slot, batch).ite(Int(1), Int(0)))
        });
        safe(
            &mut m,
            &format!("Batch{batch}AppliedAtMostOnce"),
            count.le(Int(1)),
        );
        safe(
            &mut m,
            &format!("AcknowledgedBatch{batch}SurvivesRestart"),
            implies(eq(ACK + index, 1), seen(batch)),
        );
        m.responses.push(Response {
            name: format!("DurableBatch{batch}EventuallyAcknowledged"),
            request: seen(batch),
            response: eq(ACK + index, 1),
        });
    }
    action(
        &mut m,
        "Restart",
        all([eq(READY, 1), Var(RESTARTS).lt(Int(2))]),
        &[
            (READY, Int(0)),
            (RESTARTS, Var(RESTARTS).plus(Int(1))),
            (PICTURE_A, Int(0)),
            (PICTURE_B, Int(0)),
        ],
    );
    let recover = m.actions.len();
    action(
        &mut m,
        "ReplayDurableLog",
        eq(READY, 0),
        &[
            (READY, Int(1)),
            (PICTURE_A, replay_a()),
            (PICTURE_B, seen(2).ite(Int(1), Int(0))),
        ],
    );
    m.weak_fair.push(vec![recover]);
    safe(
        &mut m,
        "PictureIsFunctionOfAppendedLog",
        implies(
            eq(READY, 1),
            Var(PICTURE_A)
                .eq(replay_a())
                .and(Var(PICTURE_B).eq(seen(2).ite(Int(1), Int(0)))),
        ),
    );
    safe(
        &mut m,
        "StoreSequenceCountsAppendedOperations",
        Var(SEQ).eq((LOG..LOG + 3).fold(Int(0), |sum, slot| {
            sum.plus(eq(slot, 0).ite(Int(0), eq(slot, 2).ite(Int(2), Int(1))))
        })),
    );
    safe(
        &mut m,
        "LateChangeNeverResurrectsRemovedElement",
        implies(eq(READY, 1).and(seen(3)), eq(PICTURE_A, 0)),
    );
    // No delivery fairness is assumed: a dead seat may leave an unsent batch.
    // Only recovery and replies for already durable batches are weakly fair.
    m
}

pub(super) fn cases() -> Vec<(Model, &'static str)> {
    vec![
        (convergence(false), "pass"),
        (convergence(true), "safety"),
        (refresh(false), "pass"),
        (refresh(true), "safety"),
    ]
}

/// Two roots in one swarm retain independent results. Each root's three reads
/// can complete twice and in any order; generation identifies an incarnation.
fn refresh(broken: bool) -> Model {
    const BEGUN: usize = 0;
    const COMPLETIONS: usize = 2;
    const PUBLISHED: usize = 8;
    const AT_PUBLICATION: usize = 10;
    let mut m = model(
        "CanvasRefreshGeneration",
        broken,
        &[
            ("root_a_generation", 3, 0),
            ("root_b_generation", 3, 0),
            ("a_first_completions", 2, 0),
            ("a_second_completions", 2, 0),
            ("a_third_completions", 2, 0),
            ("b_first_completions", 2, 0),
            ("b_second_completions", 2, 0),
            ("b_third_completions", 2, 0),
            ("a_published_generation", 3, 0),
            ("b_published_generation", 3, 0),
            ("a_generation_at_publication", 3, 0),
            ("b_generation_at_publication", 3, 0),
        ],
    );
    for root in 0..2 {
        for generation in 1..=3 {
            let completed = COMPLETIONS + root * 3 + generation as usize - 1;
            action(
                &mut m,
                &format!("BeginRoot{root}Read{generation}"),
                eq(BEGUN + root, generation - 1),
                &[(BEGUN + root, Int(generation))],
            );
            let eligible = all([
                Int(generation).le(Var(BEGUN + root)),
                Var(completed).lt(Int(2)),
            ]);
            let current = eq(BEGUN + root, generation);
            action(
                &mut m,
                &format!("PublishRoot{root}Read{generation}"),
                eligible
                    .clone()
                    .and(if broken { Bool(true) } else { current.clone() }),
                &[
                    (completed, Var(completed).plus(Int(1))),
                    (PUBLISHED + root, Int(generation)),
                    (AT_PUBLICATION + root, Var(BEGUN + root)),
                ],
            );
            if !broken {
                action(
                    &mut m,
                    &format!("DiscardRoot{root}Read{generation}"),
                    eligible.and(current.negate()),
                    &[(completed, Var(completed).plus(Int(1)))],
                );
            }
        }
        safe(
            &mut m,
            &format!("Root{root}OnlyLatestReadPublishes"),
            Var(PUBLISHED + root).eq(Var(AT_PUBLICATION + root)),
        );
    }
    // Completion arrival and filesystem success have no fairness assumption.
    // A successful current completion publishes in that same atomic action;
    // no deferred UI frame or renderer progress is inferred by this model.
    m
}

#[test]
fn canvas_refresh_generation_model_and_broken_control() {
    for (model, expected) in [(refresh(false), "pass"), (refresh(true), "safety")] {
        let output = tempfile::tempdir().unwrap();
        super::super::export(model, expected, output.path(), &mut String::new()).unwrap();
    }
}

#[test]
fn canvas_refresh_reads_api_rejects_late_aba_and_keeps_swarms_independent() {
    use bboxy450_term::canvas_host::Reads;
    use bbt_mux::Ulid;
    let (one, two) = (Ulid(1), Ulid(2));
    let mut reads = Reads::default();
    assert!(
        !reads.current(one, "/a", 0),
        "an unknown swarm has no admitted read"
    );
    let first_a = reads.begin(one, "/a/.bboxy");
    let other = reads.begin(two, "/a");
    assert!(reads.current(one, "/a", first_a));
    assert!(reads.current(two, "/a/.bboxy", other));
    let b = reads.begin(one, "/b");
    let returned_a = reads.begin(one, "/a");
    let latest_a = reads.begin(one, "/a/.bboxy");
    assert_eq!([first_a, b, returned_a, latest_a], [1, 1, 2, 3]);
    // All six possible completion orders, followed by duplicate stale/current
    // completions. Root A's first incarnation never passes for its third.
    for order in [
        [1, 2, 3],
        [1, 3, 2],
        [2, 1, 3],
        [2, 3, 1],
        [3, 1, 2],
        [3, 2, 1],
    ] {
        let mut published = None;
        for generation in order.into_iter().chain([first_a, returned_a, latest_a]) {
            if reads.current(one, "/a", generation) {
                published = Some(generation);
            }
            assert!(published.is_none_or(|generation| generation == latest_a));
            assert!(
                reads.current(two, "/a", other),
                "another swarm's refresh stays current"
            );
        }
        assert_eq!(published, Some(latest_a));
        assert!(
            reads.current(one, "/b", b),
            "A's return never cancels B's retained read"
        );
    }
    let next_other = reads.begin(two, "/a");
    assert!(!reads.current(two, "/a", other));
    assert!(reads.current(two, "/a", next_other));
    assert!(reads.current(one, "/a", latest_a));
}

#[test]
fn canvas_refresh_catalog_retains_roots_and_sidebar_follows_workspace_aba() {
    use bboxy450_term::{canvas_host, model};
    use bbt_mux::Ulid;
    use bbt_proto::gui::UiCanvasSource;
    use bbt_ui_model::SwarmId;
    use std::sync::Arc;

    let directory = tempfile::tempdir().unwrap();
    let swarm = Ulid(33);
    let swarm_id = SwarmId::from_u128(swarm.0);
    let create = |name: &str| {
        let root = directory.path().join(name);
        let store = bboxy450_canvas::store::Store::new(root.join(".bboxy/canvases"));
        store
            .create(
                &Ulid::new().to_string(),
                bboxy450_canvas::CanvasRecord {
                    title: name.into(),
                    swarm: swarm.to_string(),
                    tags: vec![],
                    creator: "seat:test".into(),
                    task: None,
                },
            )
            .unwrap();
        root.to_string_lossy().into_owned()
    };
    let (a, b) = (create("a"), create("b"));
    let rows_a = Arc::new(canvas_host::load(&format!("{a}/.bboxy"), swarm).unwrap());
    let rows_b = Arc::new(canvas_host::load(&b, swarm).unwrap());
    assert_eq!(rows_a[0].source.root, a);
    let mut model = model::Model::default();
    let pane_a = model.open_canvas(
        UiCanvasSource {
            root: format!("{a}/.bboxy"),
            ..rows_a[0].source.clone()
        },
        None,
    );
    let pane_b = model.open_canvas(rows_b[0].source.clone(), None);
    assert_eq!(model.canvas_sources[&pane_a].root, a);
    assert_eq!(model.canvas_sources[&pane_b].root, b);
    let mut reads = canvas_host::Reads::default();
    let first_a = reads.begin(swarm, &a);
    let first_b = reads.begin(swarm, &b);
    let returned_a = reads.begin(swarm, &format!("{a}/.bboxy"));
    for (root, generation, rows) in [
        (&a, returned_a, rows_a.clone()),
        (&b, first_b, rows_b.clone()),
        (&a, first_a, Arc::new(vec![])),
    ] {
        if reads.current(swarm, root, generation) {
            model
                .canvas_catalog
                .insert((swarm_id, canvas_host::normalize_root(root)), rows);
        }
    }
    assert_eq!(model.canvas_catalog.len(), 2);
    assert_eq!(
        model.canvas_catalog[&(swarm_id, a.clone())][0].source.title,
        "a"
    );
    assert_eq!(
        model.canvas_catalog[&(swarm_id, b.clone())][0].source.title,
        "b"
    );
    for root in [&a, &b, &a] {
        model.ui.window.sidebar.swarms[0].workdir = Some(root.clone());
        assert_eq!(model.canvas_rows(swarm_id).unwrap()[0].source.root, *root);
    }
    // An explicit new workspace has no rows until its own read succeeds;
    // stale retained A/B rows never masquerade as the current workspace.
    model.ui.window.sidebar.swarms[0].workdir = Some("/unread-workspace".into());
    assert!(model.canvas_rows(swarm_id).is_none());
    assert_eq!(model.canvas_catalog.len(), 2);

    // Closing the last presentation must not remove the catalog's sidebar row,
    // including a swarm whose workspace comes solely from canvas admission.
    let mut no_workdir = model::Model::default();
    let closed = no_workdir.open_canvas(rows_a[0].source.clone(), None);
    no_workdir
        .canvas_catalog
        .insert((swarm_id, a.clone()), rows_a.clone());
    assert!(no_workdir.ui.window.sidebar.swarms[0].workdir.is_none());
    no_workdir.canvas_sources.remove(&closed);
    no_workdir.ui.seats.remove(&closed);
    no_workdir.ui.window.sidebar.swarms[0].tabs.clear();
    let retained = no_workdir.canvas_rows(swarm_id).unwrap();
    assert_eq!(retained[0].source, rows_a[0].source);
    let reopen = retained[0].source.clone();
    let reopened = no_workdir.open_canvas(reopen, None);
    assert_ne!(reopened, closed);
    assert!(no_workdir.ui.tab_of(reopened).is_some());
    assert_eq!(no_workdir.canvas_rows(swarm_id).unwrap()[0].source.root, a);
}
