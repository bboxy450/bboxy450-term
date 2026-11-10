// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! The checked declaration owns the card and its Mermaid corpus fixture.
#![allow(clippy::disallowed_methods)]
use bboxy450_term_remote::delivery::RemoteDelivery as M;
use bbt_yarn::card::{View, generate};
use std::path::Path;

#[test]
fn remote_delivery_card_is_current() {
    let model = M::model();
    let index = |name| model.variables.iter().position(|v| v.name == name).unwrap();
    let (acked, applied, inflight, unacked) = (
        index("acked::A"),
        index("applied::A"),
        index("inflight::A"),
        index("unacked::A"),
    );
    let view: &[View<'_>] = &[
        ("acknowledged", &|s| s[acked] == 1),
        ("applied", &|s| s[applied] == 1),
        ("in flight", &|s| s[inflight] == 1),
        ("queued", &|s| s[unacked] == 1),
        ("new", &|_| true),
    ];
    // The registry uses one single-line, JSON-compatible TOML basic string.
    // Parse that string with the crate's existing serde_json primitive; fail
    // loudly if the registry format changes rather than add another TOML parser.
    let contracts = include_str!("../../../verification/contracts.toml");
    let entry = contracts
        .split("[[contract]]")
        .find(|s| s.lines().any(|l| l == "name = \"RemoteDelivery\""))
        .unwrap();
    let encoded = entry
        .lines()
        .find_map(|l| l.strip_prefix("assumptions = "))
        .unwrap();
    let assumptions: String = serde_json::from_str(encoded).unwrap();
    let card = generate(&M::verification_metadata(), &model, view,
        "Apply each message at most once across duplicate frames, link drops and remote crashes; retire acknowledged messages from the outbox. The diagram follows message A, collapsing message B and connection state.",
        &assumptions, 100_000).unwrap();
    let diagram = card
        .split_once("```mermaid\n")
        .unwrap()
        .1
        .split_once("```\n")
        .unwrap()
        .0;
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    for (path, contents) in [
        (root.join("cards/RemoteDelivery.md"), card.as_str()),
        (
            root.join("../bbt-mermaid/tests/fixtures/cards/RemoteDelivery.mmd"),
            diagram,
        ),
    ] {
        if std::env::var("BBOXY_UPDATE_CARDS").as_deref() == Ok("1") {
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, contents).unwrap();
        }
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            contents,
            "{} is stale; regenerate with BBOXY_UPDATE_CARDS=1",
            path.display()
        );
    }
}
