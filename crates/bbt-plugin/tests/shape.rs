// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use bbt_plugin::{
    contracts::Contract,
    shape::{PluginName, Shape, Version},
};
use serde_json::{Value, json};

fn fixture() -> Value {
    json!({"name":"reference", "version":"0.1", "shape_version":"1.0", "health_command":"health",
        "systems":[{"contract":"data_source", "contract_version":"1.0",
            "commands":Contract::DataSource.methods().iter().map(|name|json!({"name":name,
                "args":Contract::DataSource.method_schema(name,false).unwrap(),
                "result":Contract::DataSource.method_schema(name,true).unwrap(),"timeout_ms":1000})).collect::<Vec<_>>()}]})
}

// keeps: decision #278 — concrete contracts, additive wire and fixed host approval
#[test]
fn versions_are_major_minor_and_unknown_fields_are_additive() {
    for invalid in ["1", "1.0.0", "-1.0", "01.0", "1.00", "1.4294967296"] {
        assert!(Version::parse(invalid).is_err(), "{invalid}");
    }
    Version::compatible("1.99", "1.0").unwrap();
    assert_eq!(
        Version::compatible("2.0", "1.0").unwrap_err().code,
        "major_mismatch"
    );
    let mut value = fixture();
    value["future_minor_field"] = json!({"kept_by_newer_plugins":true});
    value["systems"][0]["commands"][0]["future_minor_field"] = json!(true);
    let validator =
        jsonschema::validator_for(&serde_json::from_str(bbt_plugin::shape::SCHEMA).unwrap())
            .unwrap();
    validator.validate(&value).unwrap();
    let shape: Shape = serde_json::from_value(value).unwrap();
    shape
        .validate(
            &PluginName::parse("bboxy450-term", "bboxy450-term-data_source-my-provider").unwrap(),
        )
        .unwrap();
}

// keeps: decision #278 — concrete contracts, additive wire and fixed host approval
#[test]
fn app_prefix_provider_hyphens_and_exact_name_boundary() {
    let name = PluginName::parse("another-app", "another-app-data_source-git-kb").unwrap();
    assert_eq!(name.provider, "git-kb");
    assert_eq!(
        name.tool(Contract::DataSource, "fetch").unwrap(),
        "plugin_data_source_git-kb_fetch"
    );
    for bad in [
        "bboxy450-term-data_source-good",
        "another-app-data_source-",
        "another-app-data_source-Upper",
        "another-app-data_source-two_words",
    ] {
        assert!(PluginName::parse("another-app", bad).is_err(), "{bad}");
    }
    let prefix = "plugin_data_source__changes_since".len();
    let name = PluginName {
        system: Contract::DataSource,
        provider: "x".repeat(64 - prefix),
    };
    assert_eq!(
        name.tool(Contract::DataSource, "changes_since")
            .unwrap()
            .len(),
        64
    );
    let longer = PluginName {
        provider: format!("{}x", name.provider),
        ..name
    };
    assert_eq!(
        longer
            .tool(Contract::DataSource, "changes_since")
            .unwrap_err()
            .code,
        "tool_name_too_long"
    );
}

// keeps: decision #278 — concrete contracts, additive wire and fixed host approval
#[test]
fn unknown_contract_major_missing_method_and_budget_are_named_refusals() {
    let name = PluginName::parse("bboxy450-term", "bboxy450-term-data_source-reference").unwrap();
    for (patch, code) in [
        ("unknown", "unknown_contract"),
        ("major", "major_mismatch"),
        ("missing", "missing_method"),
        ("budget", "hook_budget_exceeded"),
    ] {
        let mut value = fixture();
        match patch {
            "unknown" => value["systems"][0]["contract"] = json!("mystery"),
            "major" => value["shape_version"] = json!("2.0"),
            "missing" => {
                value["systems"][0]["commands"]
                    .as_array_mut()
                    .unwrap()
                    .pop();
            }
            "budget" => {
                value["hooks"] = json!([{"event":"PreToolUse","phase":"before","budget_ms":2001}])
            }
            _ => unreachable!(),
        }
        let shape: Shape = serde_json::from_value(value).unwrap();
        assert_eq!(shape.validate(&name).unwrap_err().code, code);
    }
    assert!(Contract::Session.methods().contains(&"describe"));
    assert!(Contract::Session.role("open").is_err());
    assert!(Contract::Provider.role("attach").is_err());
    assert!(Version::compatible("1.0", Contract::Provider.version()).is_err());
}

// keeps: decision #278 — concrete contracts, additive wire and fixed host approval
#[test]
fn descriptor_claims_cannot_reduce_tier_and_payload_hooks_need_a_separate_grant() {
    use bbt_plugin::{
        policy::Tier,
        trust::{Grants, TrustRecord},
    };
    let mut value = fixture();
    value["systems"][0]["tier"] = json!("read");
    value["systems"][0]["commands"][0]["role_class"] = json!("worker");
    let shape: Shape = serde_json::from_value(value).unwrap();
    let mut record = TrustRecord {
        sha256: "a".repeat(64),
        source: "fixture".into(),
        copy: "pinned".into(),
        data_dir: "data".into(),
        name: PluginName::parse("bboxy450-term", "bboxy450-term-data_source-reference").unwrap(),
        scope: "fixture".into(),
        approved_tier: Tier::Read,
        grants: Grants::default(),
        shape: None,
    };
    assert_eq!(
        record.validate_shape(&shape).unwrap_err().code,
        "tier_not_approved"
    );
    record.approved_tier = Tier::Export;
    record.validate_shape(&shape).unwrap();
    let value = json!({"name":"repo","version":"1.0","shape_version":"1.0","health_command":"health",
        "systems":[{"contract":"repo_graph","contract_version":"1.0","commands":Contract::RepoGraph.methods().iter().map(|name|json!({
            "name":name,"args":{},"result":{},"timeout_ms":1000})).collect::<Vec<_>>()}],
        "hooks":[{"event":"UserPromptSubmit","phase":"before","budget_ms":5000}]});
    let shape: Shape = serde_json::from_value(value).unwrap();
    record.name = PluginName::parse("bboxy450-term", "bboxy450-term-repo_graph-reference").unwrap();
    record.approved_tier = Tier::Read;
    assert_eq!(
        record.validate_shape(&shape).unwrap_err().code,
        "tier_not_approved"
    );
    record.approved_tier = Tier::Export;
    assert_eq!(
        record.validate_shape(&shape).unwrap_err().code,
        "payload_grant_required"
    );
    record
        .grants
        .payload_events
        .insert("UserPromptSubmit".into());
    record.validate_shape(&shape).unwrap();
}

// keeps: plugins — human identity survives descriptor caching without granting authority.
#[test]
fn presentation_metadata_round_trips_and_legacy_shapes_still_validate() {
    let name = PluginName::parse("bboxy450-term", "bboxy450-term-data_source-gitkb").unwrap();
    let legacy: Shape = serde_json::from_value(fixture()).unwrap();
    legacy.validate(&name).unwrap();
    assert!(legacy.description.is_empty());
    assert!(legacy.icon.is_none());
    assert!(legacy.identity.is_none());
    let mut value = fixture();
    value["name"] = json!("GitKB");
    value["description"] = json!("Documents and task records for this workspace");
    value["icon"] = json!("book");
    value["identity"] = json!({"homepage":"https://gitkb.dev", "provider":"gitkb"});
    let shape: Shape = serde_json::from_value(value.clone()).unwrap();
    shape.validate(&name).unwrap();
    let cached: Shape = serde_json::from_slice(&serde_json::to_vec(&shape).unwrap()).unwrap();
    assert_eq!(cached, shape);
    for field in ["name", "description", "icon", "identity"] {
        assert_eq!(serde_json::to_value(&cached).unwrap()[field], value[field]);
    }
}
