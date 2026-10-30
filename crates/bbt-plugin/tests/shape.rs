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
    assert!(
        shape.skills.is_empty(),
        "old descriptors need no skill field"
    );
    shape
        .validate(
            &PluginName::parse("bboxy450-term", "bboxy450-term-data_source-my-provider").unwrap(),
        )
        .unwrap();
}

#[test]
fn descriptor_skills_are_additive_and_use_shared_validation() {
    let name = PluginName::parse("bboxy450-term", "bboxy450-term-data_source-reference").unwrap();
    let mut value = fixture();
    let content = "---\r\nname: knowledge\r\n---\r\n\nRead é with the current tools.\n";
    value["skills"] = json!([{"name":"knowledge","description":"Knowledge workflow",
        "content":content,"future_minor_field":true}]);
    let validator =
        jsonschema::validator_for(&serde_json::from_str(bbt_plugin::shape::SCHEMA).unwrap())
            .unwrap();
    validator.validate(&value).unwrap();
    let shape: Shape = serde_json::from_value(value.clone()).unwrap();
    shape.validate(&name).unwrap();
    assert_eq!(shape.skills[0].content.as_bytes(), content.as_bytes());
    let duplicate = value["skills"][0].clone();
    value["skills"].as_array_mut().unwrap().push(duplicate);
    assert_eq!(
        serde_json::from_value::<Shape>(value.clone())
            .unwrap()
            .validate(&name)
            .unwrap_err()
            .code,
        "duplicate_skill"
    );
    value["skills"] = json!([]);
    validator.validate(&value).unwrap();
    serde_json::from_value::<Shape>(value.clone())
        .unwrap()
        .validate(&name)
        .unwrap();
    value["skills"] = Value::Null;
    assert!(validator.validate(&value).is_err());
    assert!(serde_json::from_value::<Shape>(value).is_err());
}

// keeps: G2 plugin seat tools — additive, validated, bounded names.
#[test]
fn declared_tools_are_additive_and_validated() {
    let name = PluginName::parse("bboxy450-term", "bboxy450-term-data_source-my-provider").unwrap();
    let validator =
        jsonschema::validator_for(&serde_json::from_str(bbt_plugin::shape::SCHEMA).unwrap())
            .unwrap();
    let tool = json!({"name":"summarize_open","description":"Summarize open work",
        "input_schema":{"type":"object","properties":{"limit":{"type":"integer"}}},
        "output_schema":{"type":"object"},"role":"oracle","timeout_ms":3000,"future_minor_field":1});
    let mut value = fixture();
    value["tools"] = json!([tool.clone()]);
    validator.validate(&value).unwrap();
    let shape: Shape = serde_json::from_value(value.clone()).unwrap();
    shape.validate(&name).unwrap();
    assert_eq!(
        shape.tools[0].seat_name(&name).unwrap(),
        "plugin_tool_my-provider_summarize_open"
    );
    assert_eq!(shape.tools[0].wire_name(), "tool_summarize_open");
    assert!(
        !serde_json::to_value(serde_json::from_value::<Shape>(fixture()).unwrap())
            .unwrap()
            .as_object()
            .unwrap()
            .contains_key("tools"),
        "descriptors without tools serialize unchanged"
    );
    for (field, bad, code, schema_ok) in [
        ("name", json!("Upper"), "invalid_tool", false),
        ("name", json!("x".repeat(41)), "tool_name_too_long", true),
        ("role", json!("host_only"), "invalid_tool", false),
        ("timeout_ms", json!(0), "invalid_timeout", false),
        (
            "input_schema",
            json!({"type":"string"}),
            "invalid_tool",
            true,
        ),
        ("output_schema", json!({"type":7}), "invalid_tool", true),
    ] {
        let mut broken = tool.clone();
        broken[field] = bad;
        let mut value = fixture();
        value["tools"] = json!([broken]);
        assert_eq!(validator.validate(&value).is_ok(), schema_ok, "{field}");
        let shape: Shape = serde_json::from_value(value).unwrap();
        assert_eq!(shape.validate(&name).unwrap_err().code, code, "{field}");
    }
    let mut value = fixture();
    value["tools"] = json!([tool.clone(), tool]);
    let shape: Shape = serde_json::from_value(value).unwrap();
    assert_eq!(shape.validate(&name).unwrap_err().code, "duplicate_tool");
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

    // A bus hook needs no grant to subscribe; its payload grant needs export.
    let mut value = serde_json::to_value(&shape).unwrap();
    value["hooks"] =
        json!([{"event":"bus","phase":"after","budget_ms":500,"subject":"task/*/status"}]);
    let bus: Shape = serde_json::from_value(value).unwrap();
    record.grants = Grants::default();
    record.approved_tier = Tier::Read;
    record.validate_shape(&bus).unwrap();
    record.grants.payload_events.insert("bus".into());
    assert_eq!(
        record.validate_shape(&bus).unwrap_err().code,
        "tier_not_approved"
    );
    record.approved_tier = Tier::Export;
    record.validate_shape(&bus).unwrap();
    assert!(record.payload_granted(&bus.hooks[0]));
}

// keeps: G7 host API — requested by the shape, granted by the operator,
// raising the tier; unknown and duplicate names are refused.
#[test]
fn host_api_requests_are_named_raise_the_tier_and_need_a_grant() {
    use bbt_plugin::{policy::Tier, trust::TrustRecord};
    let name = PluginName::parse("bboxy450-term", "bboxy450-term-repo_graph-reference").unwrap();
    let repo = |host_api: Value| {
        let value = json!({"name":"repo","version":"1.0","shape_version":"1.0","health_command":"health",
            "systems":[{"contract":"repo_graph","contract_version":"1.0","commands":Contract::RepoGraph.methods().iter().map(|name|json!({
                "name":name,"args":{},"result":{},"timeout_ms":1000})).collect::<Vec<_>>()}],
            "host_api": host_api});
        let validator =
            jsonschema::validator_for(&serde_json::from_str(bbt_plugin::shape::SCHEMA).unwrap())
                .unwrap();
        (
            validator.validate(&value).is_ok(),
            serde_json::from_value::<Shape>(value).unwrap(),
        )
    };
    let (schema_ok, shape) = repo(json!(["events.publish"]));
    assert!(schema_ok);
    assert_eq!(shape.validate(&name).unwrap(), Tier::Write);
    let (_, shape) = repo(json!(["records.search", "events.publish"]));
    assert_eq!(shape.validate(&name).unwrap(), Tier::Export);
    let (schema_ok, shape) = repo(json!(["messages.send"]));
    assert!(schema_ok);
    assert_eq!(shape.validate(&name).unwrap(), Tier::Write);
    for (requested, code) in [
        (json!(["messages.delete"]), "unknown_host_api"),
        (
            json!(["records.fetch", "records.fetch"]),
            "duplicate_host_api",
        ),
    ] {
        let (schema_ok, shape) = repo(requested);
        assert!(!schema_ok);
        assert_eq!(shape.validate(&name).unwrap_err().code, code);
    }
    let (_, shape) = repo(json!(["records.search"]));
    let mut record = TrustRecord {
        sha256: "a".repeat(64),
        source: "fixture".into(),
        copy: "pinned".into(),
        data_dir: "data".into(),
        name,
        scope: "fixture".into(),
        approved_tier: Tier::Read,
        grants: Default::default(),
        shape: Some(shape.clone()),
    };
    assert_eq!(
        record.validate_shape(&shape).unwrap_err().code,
        "tier_not_approved"
    );
    record.approved_tier = Tier::Export;
    record.validate_shape(&shape).unwrap();
    assert!(!record.host_api_granted("records.search"), "requested only");
    record.grants.host_api.insert("records.search".into());
    record.grants.host_api.insert("events.publish".into());
    assert!(record.host_api_granted("records.search"));
    assert!(!record.host_api_granted("events.publish"), "granted only");
    assert!(
        !serde_json::to_value(&record.grants)
            .unwrap()
            .as_object()
            .unwrap()
            .is_empty()
    );
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
