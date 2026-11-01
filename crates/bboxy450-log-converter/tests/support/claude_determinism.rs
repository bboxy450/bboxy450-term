// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md

#[path = "claude.rs"]
mod support;
use bboxy450_log_converter::ir::cbor;
#[test]
fn claude_deterministic_canonical_ir() {
    let bytes = support::fixture();
    let (a, ae) = support::parse(&bytes);
    let (b, be) = support::parse(&bytes);
    assert_eq!(a, b);
    assert_eq!(ae, be);
    let encode = |session, events| {
        let mut out = cbor::encode(session).unwrap();
        for event in events {
            out.extend(cbor::encode(event).unwrap());
        }
        out
    };
    assert_eq!(encode(&a, &ae), encode(&b, &be), "ClaudeDeterminism");
}
