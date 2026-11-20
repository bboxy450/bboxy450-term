// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md

//! The verifier sees one replica. Issue, expiry, delivery and admission are
//! atomic separately; publishing a notice doesn't imply that it was seen.
use super::*;

pub(super) fn cases() -> Vec<(Model, &'static str)> {
    vec![
        (seat_key_chain(Control::None), "pass"),
        (seat_key_chain(Control::SkipParentRevocation), "safety"),
        (seat_key_chain(Control::SkipScopeSubset), "safety"),
    ]
}

#[derive(PartialEq, Eq)]
enum Control {
    None,
    SkipParentRevocation,
    SkipScopeSubset,
}

const ROTATION: usize = 0;
const NODE: usize = 1;
const NODE_ROOT: usize = 2;
const NODE_EXPIRED: usize = 3;
const REVOCATION: usize = 4;
const S1: usize = 5;
const S2: usize = 6;
const CLIENT: usize = 7;
const VIOLATION: usize = 8;
const EXPIRED_SEAT: i64 = 4;
const ESCALATION_OFFSET: i64 = 4;

fn seat_key_chain(control: Control) -> Model {
    let name = match control {
        Control::None => "SeatKeyChain",
        Control::SkipParentRevocation => "SeatKeyChainParentRevocation",
        Control::SkipScopeSubset => "SeatKeyChainScopeSubset",
    };
    let mut m = model(
        name,
        control != Control::None,
        &[
            ("rotation", 2, 0),         // unpublished, delayed, seen
            ("node_incarnation", 3, 0), // absent, epoch 1, epoch 2, epoch 1 again
            ("node_issuer", 1, 0),      // pinned root or successor
            ("node_expired", 1, 0),
            ("node_revocation", 2, 0), // unpublished, delayed, seen
            ("seat_s1", 7, 0),
            ("seat_s2", 4, 0),
            ("client_p", 3, 0), // absent, pinned-root grant, successor grant, expired
            ("violation", 3, 0), // none, expiry, revocation, scope
        ],
    );

    let issuing_root = ne(ROTATION, 0).ite(Int(1), Int(0));
    let trusted = |issuer: Expr| issuer.eq(Int(0)).or(eq(ROTATION, 2));
    let current_node = ne(NODE, 0);
    let node_live = eq(NODE_EXPIRED, 0);
    // A subject's atomic max-update is abstracted to floor 2; a delayed,
    // duplicate or stale notice has no power to reset that floor.
    let node_revoked = all([eq(REVOCATION, 2), ne(NODE, 2)]);

    for (name, index) in [("Rotation", ROTATION), ("Revocation", REVOCATION)] {
        let can_publish = if index == REVOCATION {
            current_node.clone()
        } else {
            Bool(true)
        };
        action(
            &mut m,
            &format!("Publish{name}"),
            eq(index, 0).and(can_publish),
            &[(index, Int(1))],
        );
        action(
            &mut m,
            &format!("See{name}"),
            eq(index, 1),
            &[(index, Int(2))],
        );
    }
    action(
        &mut m,
        "IssueNode",
        eq(NODE, 0),
        &[(NODE, Int(1)), (NODE_ROOT, issuing_root.clone())],
    );
    action(
        &mut m,
        "RenewNode",
        all([current_node.clone(), ne(NODE, 3)]),
        &[
            (NODE, Var(NODE).plus(Int(1))),
            (NODE_ROOT, issuing_root.clone()),
            (NODE_EXPIRED, Int(0)),
        ],
    );
    action(
        &mut m,
        "ExpireNode",
        current_node.clone().and(node_live.clone()),
        &[(NODE_EXPIRED, Int(1))],
    );

    // Each seat binds one exact parent digest, represented by the issuance's
    // incarnation even when the node key and epoch repeat. The scope bit
    // models whether S1 requests an extra scope absent from its parent.
    let may_issue_seat = current_node.clone().and(node_live.clone());
    for (name, seat) in [("S1", S1), ("S2", S2)] {
        action(
            &mut m,
            &format!("IssueOrRenew{name}"),
            may_issue_seat.clone(),
            &[(seat, Var(NODE))],
        );
        let seat_live = all([ne(seat, 0), ne(seat, EXPIRED_SEAT)]);
        action(
            &mut m,
            &format!("Expire{name}"),
            seat_live.clone(),
            &[(seat, Int(EXPIRED_SEAT))],
        );
        let fits = Var(seat).eq(Var(NODE));
        let same_parent = if seat == S1 {
            fits.clone()
                .or(Var(seat).eq(Var(NODE).plus(Int(ESCALATION_OFFSET))))
        } else {
            fits.clone()
        };
        let chain_live = seat_live.and(node_live.clone());
        let mut guards = vec![
            current_node.clone(),
            trusted(Var(NODE_ROOT)),
            same_parent,
            chain_live.clone(),
        ];
        if control != Control::SkipParentRevocation {
            guards.push(node_revoked.clone().negate());
        }
        if control != Control::SkipScopeSubset {
            guards.push(fits.clone());
        }
        let invalid = chain_live.negate().ite(
            Int(1),
            node_revoked
                .clone()
                .ite(Int(2), fits.negate().ite(Int(3), Var(VIOLATION))),
        );
        action(
            &mut m,
            &format!("Request{name}"),
            all(guards),
            &[(VIOLATION, invalid)],
        );
    }
    action(
        &mut m,
        "IssueS1WithIssueNodeScope",
        may_issue_seat,
        &[(S1, Var(NODE).plus(Int(ESCALATION_OFFSET)))],
    );
    action(
        &mut m,
        "RequestNode",
        all([
            current_node,
            trusted(Var(NODE_ROOT)),
            node_live,
            node_revoked.negate(),
        ]),
        &[],
    );
    action(
        &mut m,
        "IssueOrRenewClient",
        Bool(true),
        &[(CLIENT, issuing_root.plus(Int(1)))],
    );
    let client_live = eq(CLIENT, 1).or(eq(CLIENT, 2));
    action(
        &mut m,
        "ExpireClient",
        client_live.clone(),
        &[(CLIENT, Int(3))],
    );
    action(
        &mut m,
        "RequestClient",
        client_live.and(trusted(Var(CLIENT).minus(Int(1)))),
        &[],
    );

    safe(&mut m, "NoExpiredChainAccepted", ne(VIOLATION, 1));
    safe(&mut m, "NoSeenRevokedEpochAccepted", ne(VIOLATION, 2));
    safe(&mut m, "NoScopeEscalationAccepted", ne(VIOLATION, 3));
    m
}
