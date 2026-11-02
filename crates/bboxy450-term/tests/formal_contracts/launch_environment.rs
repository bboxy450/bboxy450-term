// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Exec snapshots, retained dirty owners and three independent seat births.
use super::{action, all, eq, implies, model, ne, safe};
use bboxy450_tlc::{Expr::*, Model};

fn environment(backend: bool, broken: bool) -> Model {
    let mut m = model(
        if backend {
            "BackendLaunchEnvironment"
        } else {
            "SeatLaunchEnvironment"
        },
        broken,
        &[
            ("caller_marker", 1, 1),
            ("owner_marker", 1, 0),
            ("owner_source", 2, 0),
            ("seat_marker", 1, 0),
            ("birth", 3, 0),
        ],
    );
    for marker in 0..=1 {
        action(
            &mut m,
            &format!("CallerMarker{marker}"),
            ne(0, marker),
            &[(0, Int(marker))],
        );
    }
    action(
        &mut m,
        "StartFreshBackend",
        eq(2, 0),
        &[
            (1, if backend && broken { Var(0) } else { Int(0) }),
            (2, Int(1)),
        ],
    );
    action(
        &mut m,
        "AttachRetainedBackend",
        eq(2, 0),
        &[(1, Var(0)), (2, Int(2))],
    );
    action(
        &mut m,
        "SpawnSeat",
        all([ne(2, 0), Var(4).lt(Int(3))]),
        &[
            (3, if !backend && broken { Var(1) } else { Int(0) }),
            (4, Var(4).plus(Int(1))),
        ],
    );
    action(
        &mut m,
        "RestartBackend",
        ne(2, 0),
        &[(1, Int(0)), (2, Int(0))],
    );
    safe(
        &mut m,
        "FreshBackendHasNoCallerMarker",
        implies(eq(2, 1), eq(1, 0)),
    );
    safe(&mut m, "NoCallerMarkerCrossesSeatExec", eq(3, 0));
    m
}

pub fn cases() -> Vec<(Model, &'static str)> {
    vec![
        (environment(false, false), "pass"),
        (environment(false, true), "safety"),
        (environment(true, false), "pass"),
        (environment(true, true), "safety"),
    ]
}
