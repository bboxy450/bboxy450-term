// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Ordered task prefixes and etag-bound UTF-8 body continuation.
use super::*;

pub(super) fn cases() -> Vec<(Model, &'static str)> {
    vec![
        (task_byte_page(false), "pass"),
        (task_byte_page(true), "safety"),
        (fetch_body_page(false), "pass"),
        (fetch_body_page(true), "safety"),
    ]
}

fn task_byte_page(broken: bool) -> Model {
    let mut m = model(
        "TaskBytePage",
        broken,
        &[
            ("cursor", 3, 0),
            ("end", 3, 0),
            ("bytes", 4, 0),
            ("published", 1, 0),
        ],
    );
    // The third source item exceeds a page alone; its lean projection costs 1.
    // Exact JSON escaping, margins and 128-item limits use the Rust API witness.
    for (index, cost) in [1, 2, 1].into_iter().enumerate() {
        let fits = Var(2).plus(Int(cost)).le(Int(2));
        action(
            &mut m,
            &format!("Append{index}"),
            all([
                eq(3, 0),
                eq(1, index as i64),
                if broken { Bool(true) } else { fits },
            ]),
            &[(1, Int(index as i64 + 1)), (2, Var(2).plus(Int(cost)))],
        );
    }
    action(
        &mut m,
        "PublishPrefix",
        all([eq(3, 0), Var(0).lt(Var(1))]),
        &[(0, Var(1)), (3, Int(1))],
    );
    action(
        &mut m,
        "FollowNext",
        all([eq(3, 1), Var(0).lt(Int(3))]),
        &[(2, Int(0)), (3, Int(0))],
    );
    safe(&mut m, "BoundedEncodedPage", Var(2).le(Int(2)));
    safe(&mut m, "CursorNeverSkipsAnItem", Var(0).le(Var(1)));
    safe(
        &mut m,
        "NextIsLastEmittedId",
        implies(eq(3, 1), Var(0).eq(Var(1))),
    );
    m
}

fn fetch_body_page(broken: bool) -> Model {
    let mut m = model(
        "FetchBodyPage",
        broken,
        &[("offset", 7, 0), ("delivered", 7, 0)],
    );
    // UTF-8 boundaries of a three-character body with lengths 2, 1 and 4.
    for (from, to) in [(0, 2), (2, 3), (3, 7)] {
        action(
            &mut m,
            &format!("Emit{from}to{to}"),
            eq(0, from),
            &[
                (0, Int(if broken && from == 0 { to + 1 } else { to })),
                (1, Var(1).plus(Int(to - from))),
            ],
        );
    }
    // A caller that keeps following next schedules each enabled Emit fairly.
    m.weak_fair.extend([vec![0], vec![1], vec![2]]);
    m.responses.push(Response {
        name: "BodyEventuallyComplete".into(),
        request: Bool(true),
        response: eq(0, 7),
    });
    safe(&mut m, "NoGapOrOverlap", Var(0).eq(Var(1)));
    safe(
        &mut m,
        "Utf8Boundary",
        eq(0, 0).or(eq(0, 2)).or(eq(0, 3)).or(eq(0, 7)),
    );
    m
}
