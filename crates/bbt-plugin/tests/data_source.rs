// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use bbt_plugin::data_source as d;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::collections::BTreeSet;

fn roundtrip<T: DeserializeOwned + Serialize>(value: &Value) -> Value {
    serde_json::to_value(serde_json::from_value::<T>(value.clone()).unwrap()).unwrap()
}
fn typed(method: &str, value: &Value, result: bool) -> Value {
    match (method, result) {
        ("resolve", false) => roundtrip::<d::ResolveRequest>(value),
        ("resolve", true) => roundtrip::<d::RecordRef>(value),
        ("fetch" | "links", false) => roundtrip::<d::FetchRequest>(value),
        ("fetch", true) => roundtrip::<d::Record>(value),
        ("search", false) => roundtrip::<d::SearchRequest>(value),
        ("search", true) => roundtrip::<d::Page<d::SearchHit>>(value),
        ("list", false) => roundtrip::<d::ListRequest>(value),
        ("list", true) => roundtrip::<d::Page<d::RecordRef>>(value),
        ("links", true) => roundtrip::<d::Links>(value),
        ("graph", false) => roundtrip::<d::GraphRequest>(value),
        ("graph", true) => roundtrip::<d::GraphPage>(value),
        ("write", false) => roundtrip::<d::WriteRequest>(value),
        ("write", true) => roundtrip::<d::WriteResult>(value),
        ("changes_since", false) => roundtrip::<d::ChangesRequest>(value),
        ("changes_since", true) => roundtrip::<d::Changes>(value),
        _ => panic!("fixture names a non-contract method"),
    }
}

// keeps: decision #278 — concrete contracts, additive wire and fixed host approval
#[test]
fn independent_eight_method_fixtures_match_schema_and_portable_types() {
    let fixtures: Vec<Value> =
        serde_json::from_str(include_str!("fixtures/data_source/conformance.json")).unwrap();
    let mut seen = BTreeSet::new();
    for fixture in fixtures {
        let method = fixture["method"].as_str().unwrap();
        assert!(seen.insert(method.to_owned()));
        for (field, result) in [("request", false), ("result", true)] {
            let schema = d::method_schema(method, result).unwrap();
            let validator = jsonschema::validator_for(&schema).unwrap();
            let mut value = fixture[field].clone();
            validator.validate(&value).unwrap();
            // Minor extensions are accepted at both boundaries, not merely
            // by the schema while a Rust deserializer refuses them.
            value["future_minor_field"] = json!({"ignored": true});
            validator.validate(&value).unwrap();
            let normalized = typed(method, &value, result);
            assert!(normalized.get("future_minor_field").is_none());
            validator.validate(&normalized).unwrap();
        }
    }
    assert_eq!(
        seen,
        [
            "resolve",
            "fetch",
            "search",
            "list",
            "links",
            "graph",
            "write",
            "changes_since"
        ]
        .into_iter()
        .map(str::to_owned)
        .collect()
    );
    assert!(d::method_schema("execute", false).is_none());
}

// keeps: decision #278 — concrete contracts, additive wire and fixed host approval
#[test]
fn malformed_pages_references_and_create_without_destination_fail_the_schema() {
    for (method, value) in [
        ("resolve", json!({"locator":""})),
        ("search", json!({"query":"x","limit":0})),
        ("list", json!({"limit":129})),
        (
            "fetch",
            json!({"ref":{"id":"x","number":0,"path":"a.md","slug":"a","digest":"not-a-digest"}}),
        ),
        ("graph", json!({"scope":{"kind":"all","graph":"execution"}})),
        (
            "write",
            json!({"frontmatter":{"type":"note"},"body":"new file without a path"}),
        ),
        ("changes_since", json!({"cursor":""})),
        (
            "write",
            json!({"ref":{"id":"record","number":7,"path":"a.md","slug":"a","digest":"a".repeat(64)},"path":"a.md","frontmatter":{},"body":"an update cannot supply a destination"}),
        ),
    ] {
        let validator =
            jsonschema::validator_for(&d::method_schema(method, false).unwrap()).unwrap();
        assert!(!validator.is_valid(&value), "{method}: {value}");
    }
}

// keeps: decision #278 — concrete contracts, additive wire and fixed host approval
#[test]
fn document_unknown_metadata_and_markdown_are_preserved() {
    let fixtures: Vec<Value> =
        serde_json::from_str(include_str!("fixtures/data_source/conformance.json")).unwrap();
    let fixture = fixtures.iter().find(|f| f["method"] == "fetch").unwrap();
    let record: d::Record = serde_json::from_value(fixture["result"].clone()).unwrap();
    assert_eq!(record.frontmatter["unknown_metadata"], json!({"kept":true}));
    assert_eq!(record.body, "# Fixture\n\n![[images/pixel.png]]\n");
    let encoded = serde_json::to_value(&record).unwrap();
    assert_eq!(encoded["id"], record.reference.id);
    assert_eq!(encoded["number"], 7);
    assert_eq!(encoded["resources"][0]["mimeType"], "image/png");
    assert_eq!(encoded["resources"][0]["type"], "resource_link");
}
