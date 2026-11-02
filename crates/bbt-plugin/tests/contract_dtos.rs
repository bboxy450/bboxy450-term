// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! The typed contract DTOs round-trip through their JSON Schemas: every
//! request and result of every method of every contract serializes to a
//! value its `schemas/*.json` definition accepts, and decodes back equal.
use std::collections::BTreeMap;

use bbt_plugin::contracts::Contract;
use bbt_plugin::{
    data_source as ds, decision_model as dm, event_sink as es, provider as pv, repo_graph as rg,
    session as ss, task_system as ts,
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Map, Value, json};

const ULID: &str = "01M66NR73B287NMCJCN34MR5Q1";
const DIGEST: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

fn check<T: Serialize + DeserializeOwned + PartialEq + std::fmt::Debug>(
    contract: Contract,
    method: &str,
    result: bool,
    value: &T,
    seen: &mut Vec<(Contract, String, bool)>,
) {
    let schema = contract
        .method_schema(method, result)
        .unwrap_or_else(|| panic!("{} {method} has no schema", contract.name()));
    let encoded = serde_json::to_value(value).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    if let Err(error) = validator.validate(&encoded) {
        panic!(
            "{} {method} {}: {error}\n{encoded}",
            contract.name(),
            if result { "result" } else { "request" }
        );
    }
    let decoded: T = serde_json::from_value(encoded.clone()).unwrap();
    assert_eq!(&decoded, value, "{} {method}", contract.name());
    // Additive fields are ignored on decode.
    if let Value::Object(mut fields) = encoded {
        fields.insert("future_minor_field".into(), json!(true));
        let _: T = serde_json::from_value(Value::Object(fields))
            .unwrap_or_else(|e| panic!("{} {method}: unknown field refused: {e}", contract.name()));
    }
    seen.push((contract, method.into(), result));
}

fn task() -> ts::TaskRecord {
    ts::TaskRecord {
        id: ULID.into(),
        number: Some(7),
        external_id: Some("native-7".into()),
        path: "tasks/seven.md".into(),
        metadata: Map::from_iter([("status".into(), json!("open"))]),
        body: "Do it.\n".into(),
        body_truncated: Some(false),
        etag: "e1".into(),
        signed_record: Some("signed bytes".into()),
    }
}

fn envelope() -> es::Envelope {
    es::Envelope {
        version: 1,
        id: ULID.into(),
        issuer_kind: es::IssuerKind::LocalDev,
        issuer: "dev".into(),
        account: "account".into(),
        device: "device".into(),
        seat: ULID.into(),
        author: "author".into(),
        swarm_id: ULID.into(),
        process_generation: "1".into(),
        context: "ctx".into(),
        role: es::Role {
            name: "executor".into(),
            class: es::Class::Worker,
        },
        role_grant_id: "grant".into(),
        role_epoch: "0".into(),
        priority_class: es::Class::Worker,
        sender_sequence: "3".into(),
        thread_id: ULID.into(),
        reply_to: None,
        timestamp: 1,
        priority: 2,
        kind: es::Kind::Text,
        body: es::Body::Text {
            text: "hello".into(),
        },
        attachments: Vec::new(),
        door: Some(es::Door::Ctl),
        issued_by: None,
        signature: "sig".into(),
    }
}

#[test]
fn every_contract_dto_round_trips_through_its_schema() {
    let mut seen = Vec::new();
    let s = &mut seen;
    let reference = ds::RecordRef {
        id: ULID.into(),
        number: Some(7),
        slug: "seven".into(),
        path: "seven.md".into(),
        digest: DIGEST.into(),
        external_id: None,
        updated_at: ds::timestamp_millis("2026-11-01T02:00:00.123Z"),
    };
    let record = ds::Record {
        reference: reference.clone(),
        frontmatter: json!({"title": "Seven"}),
        body: "Body\n".into(),
        etag: "e1".into(),
        resources: Vec::new(),
        parse_error: None,
        relationships: None,
    };
    let page = |items| ds::Page { items, next: None };
    let changes = ds::ChangesRequest {
        cursor: "0".into(),
        limit: Some(1),
    };
    let fetch = ds::FetchRequest {
        reference: reference.clone(),
    };
    let d = Contract::DataSource;
    check(
        d,
        "resolve",
        false,
        &ds::ResolveRequest {
            locator: "#7".into(),
        },
        s,
    );
    check(d, "resolve", true, &reference, s);
    check(d, "fetch", false, &fetch, s);
    check(d, "fetch", true, &record, s);
    check(
        d,
        "search",
        false,
        &ds::SearchRequest {
            query: "seven".into(),
            cursor: None,
            limit: Some(1),
        },
        s,
    );
    let hit = ds::SearchHit {
        reference: reference.clone(),
        title: "Seven".into(),
        record_type: "note".into(),
        number: Some(7),
        snippet: "Body".into(),
        score: 1.,
    };
    check(d, "search", true, &page(vec![hit]), s);
    check(
        d,
        "list",
        false,
        &ds::ListRequest {
            filter: ds::Filter::default(),
            cursor: None,
            limit: Some(1),
        },
        s,
    );
    check(
        d,
        "list",
        true,
        &ds::Page {
            items: vec![reference.clone()],
            next: Some("1".into()),
        },
        s,
    );
    check(d, "links", false, &fetch, s);
    let edge = ds::Edge {
        from: "a".into(),
        to: "b".into(),
        rel: "link".into(),
        weight: 1.,
    };
    check(
        d,
        "links",
        true,
        &ds::Links {
            outgoing: vec![edge.clone()],
            incoming: Vec::new(),
        },
        s,
    );
    let graph = ds::GraphRequest {
        scope: ds::Scope::All {
            graph: ds::DocumentGraph::Markdown,
        },
        cursor: None,
        limit: Some(1),
    };
    check(d, "graph", false, &graph, s);
    let node = ds::Node {
        id: ULID.into(),
        reference: Some(reference.clone()),
        title: Some("Seven".into()),
        record_type: Some("note".into()),
        frontmatter: json!({}),
    };
    check(
        d,
        "graph",
        true,
        &ds::GraphPage {
            nodes: vec![node],
            edges: vec![edge],
            next: None,
            edges_truncated: false,
        },
        s,
    );
    let write = ds::WriteRequest {
        reference: None,
        path: Some("new.md".into()),
        frontmatter: json!({"title": "New"}),
        body: "New\n".into(),
        etag: None,
    };
    check(d, "write", false, &write, s);
    check(
        d,
        "write",
        true,
        &ds::WriteResult {
            record: record.clone(),
            lossy: false,
        },
        s,
    );
    check(
        d,
        "commit",
        false,
        &ds::CommitRequest {
            refs: vec![reference.clone()],
            message: "m".into(),
        },
        s,
    );
    check(
        d,
        "commit",
        true,
        &ds::CommitResult {
            commit: "abc".into(),
            committed: vec![reference.clone()],
        },
        s,
    );
    check(d, "changes_since", false, &changes, s);
    check(
        d,
        "changes_since",
        true,
        &ds::Changes {
            changed: vec![reference],
            next: "1".into(),
        },
        s,
    );

    let t = Contract::TaskSystem;
    let signed = "signed bytes".to_owned();
    check(
        t,
        "list",
        false,
        &ts::ListRequest {
            status: Some("open".into()),
            limit: Some(1),
            ..Default::default()
        },
        s,
    );
    check(
        t,
        "list",
        true,
        &ts::TaskPage {
            items: vec![task()],
            next: Some(format!("1793498400000:{ULID}")),
        },
        s,
    );
    check(
        t,
        "status",
        false,
        &ts::StatusRequest::Path {
            task_path: "tasks/seven.md".into(),
        },
        s,
    );
    check(
        t,
        "status",
        false,
        &ts::StatusRequest::Id {
            task_id: "#7".into(),
        },
        s,
    );
    check(t, "status", true, &task(), s);
    check(t, "changes_since", false, &changes, s);
    check(
        t,
        "changes_since",
        true,
        &ts::Changes {
            changed: vec![ULID.into()],
            next: "2".into(),
        },
        s,
    );
    check(
        t,
        "delegate",
        false,
        &ts::DelegateRequest {
            task_path: "t.md".into(),
            etag: "e".into(),
            signed_record: signed.clone(),
            seat_id: ULID.into(),
            role: Some("executor".into()),
        },
        s,
    );
    check(
        t,
        "reassign",
        false,
        &ts::ReassignRequest {
            task_path: "t.md".into(),
            etag: "e".into(),
            signed_record: signed.clone(),
            seat_id: ULID.into(),
        },
        s,
    );
    let verification = ts::TaskVerification {
        command: "make test".into(),
        exit_code: 0,
        evidence: vec![ts::Evidence {
            criterion: "green".into(),
            reference: "log".into(),
        }],
    };
    check(
        t,
        "report",
        false,
        &ts::ReportRequest {
            task_path: "t.md".into(),
            etag: "e".into(),
            signed_record: signed.clone(),
            summary: "done".into(),
            commit: Some("abc".into()),
            report: None,
            verify_result: Some(verification),
        },
        s,
    );
    check(
        t,
        "accept",
        false,
        &ts::AcceptRequest {
            task_path: "t.md".into(),
            etag: "e".into(),
            signed_record: signed.clone(),
        },
        s,
    );
    check(
        t,
        "reject",
        false,
        &ts::RejectRequest {
            task_path: "t.md".into(),
            etag: "e".into(),
            signed_record: signed,
            reason: "no".into(),
        },
        s,
    );
    for method in ["delegate", "reassign", "report", "accept", "reject"] {
        check(t, method, true, &task(), s);
    }

    let e = Contract::EventSink;
    check(
        e,
        "deliver",
        false,
        &es::DeliverRequest {
            subject: "a/b".into(),
            envelope: envelope(),
        },
        s,
    );
    let mut presence = envelope();
    presence.kind = es::Kind::Presence;
    presence.body = es::Body::Presence {
        event: "joined".into(),
        name: "Seat".into(),
        harness: "claude".into(),
        state: es::PresenceState::Idle,
        muted: false,
        policy_epoch: "0".into(),
    };
    check(
        e,
        "deliver",
        false,
        &es::DeliverRequest {
            subject: "a/b".into(),
            envelope: presence,
        },
        s,
    );
    check(
        e,
        "deliver",
        true,
        &es::DeliverResult {
            id: ULID.into(),
            accepted: true,
            reason: None,
        },
        s,
    );

    let m = Contract::DecisionModel;
    let criteria = BTreeMap::from([
        ("a".to_owned(), "first".to_owned()),
        ("b".to_owned(), "second".to_owned()),
    ]);
    check(
        m,
        "choose",
        false,
        &dm::ChooseRequest {
            state: json!({"x": 1}),
            instructions: "pick".into(),
            criteria,
        },
        s,
    );
    check(
        m,
        "choose",
        true,
        &dm::ChooseResult {
            choice: "a".into(),
            confidence: 0.75,
            probabilities: BTreeMap::from([("a".into(), 0.75), ("b".into(), 0.25)]),
            model: "m-1".into(),
        },
        s,
    );

    let r = Contract::RepoGraph;
    check(
        r,
        "describe",
        false,
        &rg::DescribeRequest {
            root: "/repo".into(),
        },
        s,
    );
    check(
        r,
        "describe",
        true,
        &rg::DescribeResult {
            repos: vec![rg::Repo {
                path: "/repo".into(),
                kind: rg::RepoKind::Cargo,
                name: "repo".into(),
            }],
            edges: vec![rg::Edge {
                from: "repo".into(),
                to: "sub".into(),
                rel: rg::Rel::Contains,
            }],
        },
        s,
    );
    check(r, "changes_since", false, &changes, s);
    check(
        r,
        "changes_since",
        true,
        &rg::Changes {
            changed: vec!["/repo/sub".into()],
            next: "1".into(),
        },
        s,
    );

    check(
        Contract::Session,
        "describe",
        false,
        &ss::DescribeRequest {},
        s,
    );
    check(
        Contract::Session,
        "describe",
        true,
        &ss::DescribeResult {
            sessions: vec![ss::Session {
                id: "1".into(),
                name: "one".into(),
                harness: "codex".into(),
                cwd: Some("/w".into()),
            }],
        },
        s,
    );
    check(
        Contract::Provider,
        "describe",
        false,
        &pv::DescribeRequest {},
        s,
    );
    check(
        Contract::Provider,
        "describe",
        true,
        &pv::DescribeResult {
            hosts: vec![pv::Host {
                id: "h".into(),
                name: "box".into(),
                kind: pv::HostKind::Ssh,
                address: "box.local".into(),
            }],
        },
        s,
    );

    check(
        Contract::Transport,
        "wrap",
        false,
        &bbt_plugin::transport::WrapRequest {
            fields: Default::default(),
            command: vec!["/bin/echo".into(), "literal argument".into()],
        },
        s,
    );
    check(
        Contract::Transport,
        "wrap",
        true,
        &bbt_plugin::transport::WrapResult {
            argv: vec!["/bin/echo".into(), "literal argument".into()],
        },
        s,
    );

    // Every method of every contract, both directions, has a typed DTO.
    for contract in Contract::ALL {
        let methods = contract
            .methods()
            .iter()
            .copied()
            .chain((contract == Contract::DataSource).then_some("commit"));
        for method in methods {
            for result in [false, true] {
                assert!(
                    seen.iter()
                        .any(|(c, m, r)| *c == contract && m == method && *r == result),
                    "{} {method} {result}: no typed DTO checked",
                    contract.name()
                );
            }
        }
    }
    assert_eq!(ts::METHODS.as_slice(), Contract::TaskSystem.methods());
    assert_eq!(es::METHODS.as_slice(), Contract::EventSink.methods());
    assert_eq!(dm::METHODS.as_slice(), Contract::DecisionModel.methods());
    assert_eq!(rg::METHODS.as_slice(), Contract::RepoGraph.methods());
    assert_eq!(ss::METHODS.as_slice(), Contract::Session.methods());
    assert_eq!(pv::METHODS.as_slice(), Contract::Provider.methods());
}

// A strict envelope refuses fields it does not know.
#[test]
fn the_signed_envelope_stays_strict() {
    let mut value = serde_json::to_value(envelope()).unwrap();
    value["extra"] = json!(1);
    assert!(serde_json::from_value::<es::Envelope>(value).is_err());
}

#[test]
fn updated_at_is_optional_and_timestamps_parse_to_epoch_millis() {
    let old =
        json!({"id": ULID, "number": 7, "slug": "seven", "path": "seven.md", "digest": DIGEST});
    let reference: ds::RecordRef = serde_json::from_value(old.clone()).unwrap();
    assert_eq!(reference.updated_at, None);
    assert_eq!(
        serde_json::to_value(&reference).unwrap(),
        old,
        "absent stays absent"
    );
    for (text, millis) in [
        ("1970-01-01T00:00:00Z", Some(0)),
        ("2026-11-01T02:00:00Z", Some(1_793_498_400_000)),
        ("2026-11-01T02:00:00.123Z", Some(1_793_498_400_123)),
        ("2026-11-02T02:33:57.383724Z", Some(1_793_586_837_383)),
        ("2026-11-02T02:33:57.383724+00:00", Some(1_793_586_837_383)),
        ("2026-11-01T04:00:00+02:00", Some(1_793_498_400_000)),
        ("2024-02-29T00:00:00Z", Some(1_709_164_800_000)),
        ("1969-12-31T23:59:59Z", None),
        ("2026-13-01T00:00:00Z", None),
        ("2026-11-01", None),
        ("2026-11-01T02:00:00", None),
        ("2026-11-01T02:00:00.Z", None),
    ] {
        assert_eq!(ds::timestamp_millis(text), millis, "{text}");
    }
}
