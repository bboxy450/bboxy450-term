// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use bbt_plugin::{contracts::Contract, policy::RoleClass};
use serde_json::{Value, json};

fn valid(contract: Contract, method: &str, result: bool, value: Value) {
    let schema = contract.method_schema(method, result).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    validator.validate(&value).unwrap();
    let mut extended = value;
    extended["future_minor_field"] = json!({"ignore":true});
    validator.validate(&extended).unwrap();
}

// keeps: decision #278 — concrete contracts, additive wire and fixed host approval
#[test]
fn every_versioned_method_has_two_valid_schemas_and_no_unbriefed_operations() {
    for contract in Contract::ALL {
        let document: Value = serde_json::from_str(contract.schema()).unwrap();
        jsonschema::validator_for(&document).unwrap();
        for method in contract
            .methods()
            .iter()
            .copied()
            .chain((contract == Contract::DataSource).then_some("commit"))
        {
            for result in [false, true] {
                jsonschema::validator_for(&contract.method_schema(method, result).unwrap())
                    .unwrap();
            }
        }
        assert!(contract.method_schema("execute", false).is_none());
    }
    assert_eq!(
        Contract::TaskSystem.methods(),
        [
            "list",
            "status",
            "delegate",
            "reassign",
            "report",
            "accept",
            "reject",
            "changes_since"
        ]
    );
    assert_eq!(Contract::Session.version(), "0.1");
    assert_eq!(Contract::Provider.version(), "0.1");
    assert_eq!(
        Contract::EventSink.role("deliver").unwrap(),
        RoleClass::HostOnly
    );
    assert_eq!(
        Contract::DecisionModel.role("choose").unwrap(),
        RoleClass::Oracle
    );
}

#[test]
fn method_schemas_keep_canonical_definitions_without_unreachable_tables() {
    for contract in Contract::ALL {
        let canonical: Value = serde_json::from_str(contract.schema()).unwrap();
        let mut bytes = 0;
        for method in contract
            .methods()
            .iter()
            .copied()
            .chain((contract == Contract::DataSource).then_some("commit"))
        {
            for result in [false, true] {
                let schema = contract.method_schema(method, result).unwrap();
                for (name, definition) in schema["$defs"].as_object().unwrap() {
                    assert_eq!(Some(definition), canonical["$defs"].get(name));
                }
                bytes += serde_json::to_vec(&schema).unwrap().len();
            }
        }
        eprintln!("canonical {contract:?} method schemas: {bytes} bytes");
    }
    for (contract, method) in [
        (Contract::DataSource, "resolve"),
        (Contract::Session, "describe"),
    ] {
        assert_eq!(
            contract.method_schema(method, false).unwrap()["$defs"]
                .as_object()
                .unwrap()
                .len(),
            1,
            "unrelated methods must not inflate a standalone request"
        );
    }
    let graph = Contract::DataSource.method_schema("graph", true).unwrap();
    for dependency in ["GraphPage", "Node", "Edge", "RecordRef"] {
        assert!(graph["$defs"].get(dependency).is_some());
    }
    assert!(graph["$defs"].get("WriteRequest").is_none());
    assert!(graph["$defs"].get("CommitRequest").is_none());
}

// keeps: decision #278 — concrete contracts, additive wire and fixed host approval
#[test]
fn the_four_pinned_tables_are_concrete_and_additive() {
    valid(Contract::Session, "describe", false, json!({}));
    valid(
        Contract::Session,
        "describe",
        true,
        json!({"sessions":[{"id":"local","name":"fixture","harness":"zsh","cwd":"/fixture"}]}),
    );
    valid(Contract::Provider, "describe", false, json!({}));
    valid(
        Contract::Provider,
        "describe",
        true,
        json!({"hosts":[{"id":"rig","name":"Rig","kind":"ssh","address":"rig.local"}]}),
    );
    valid(
        Contract::RepoGraph,
        "describe",
        false,
        json!({"root":"/fixture"}),
    );
    valid(
        Contract::RepoGraph,
        "describe",
        true,
        json!({"repos":[{"path":".","name":"root","kind":"cargo"}],"edges":[{"from":"root","to":"member","rel":"workspace"}]}),
    );
    valid(
        Contract::RepoGraph,
        "changes_since",
        false,
        json!({"cursor":"0"}),
    );
    valid(
        Contract::RepoGraph,
        "changes_since",
        true,
        json!({"changed":["Cargo.toml"],"next":"1"}),
    );
    valid(
        Contract::DecisionModel,
        "choose",
        false,
        json!({"state":{"ready":true},"instructions":"Pick an assignee","criteria":{"fox":"Owns the seam"}}),
    );
    valid(
        Contract::DecisionModel,
        "choose",
        true,
        json!({"choice":"fox","confidence":0.9,"probabilities":{"fox":0.9},"model":"fixture-1"}),
    );
    for (contract, method, value) in [
        (
            Contract::Session,
            "describe",
            json!({"sessions":[{"id":"x"}]}),
        ),
        (
            Contract::Provider,
            "describe",
            json!({"hosts":[{"id":"x","name":"x","kind":"launch","address":"x"}]}),
        ),
        (
            Contract::RepoGraph,
            "describe",
            json!({"repos":[{"path":".","name":"x","kind":"unknown"}],"edges":[]}),
        ),
        (
            Contract::DecisionModel,
            "choose",
            json!({"choice":"x","confidence":1.1,"probabilities":{"x":1.1},"model":"fixture"}),
        ),
    ] {
        assert!(
            !jsonschema::validator_for(&contract.method_schema(method, true).unwrap())
                .unwrap()
                .is_valid(&value)
        );
    }
}

// keeps: decision #278 — concrete contracts, additive wire and fixed host approval
#[test]
fn task_methods_keep_native_arguments_and_require_an_etag_on_every_write() {
    let id = "00000000000000000000000001";
    valid(Contract::TaskSystem, "list", false, json!({"limit":128}));
    valid(
        Contract::TaskSystem,
        "status",
        false,
        json!({"task_id":"#258"}),
    );
    for (method, extra) in [
        ("delegate", json!({"seat_id":id})),
        ("reassign", json!({"seat_id":id})),
        (
            "report",
            json!({"summary":"done","verify_result":{"command":"true","exit_code":0}}),
        ),
        ("accept", json!({})),
        ("reject", json!({"reason":"still missing evidence"})),
    ] {
        let mut args = json!({"task_path":"tasks/fixture.md","etag":"old-digest","signed_record":"opaque-fixture"});
        args.as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        valid(Contract::TaskSystem, method, false, args.clone());
        let mut no_signature = args.clone();
        no_signature
            .as_object_mut()
            .unwrap()
            .remove("signed_record");
        assert!(
            !jsonschema::validator_for(&Contract::TaskSystem.method_schema(method, false).unwrap())
                .unwrap()
                .is_valid(&no_signature)
        );
        args.as_object_mut().unwrap().remove("etag");
        assert!(
            !jsonschema::validator_for(&Contract::TaskSystem.method_schema(method, false).unwrap())
                .unwrap()
                .is_valid(&args)
        );
    }
}
