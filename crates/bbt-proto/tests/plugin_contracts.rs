use bbt_plugin::contracts::Contract;
use bbt_proto::messaging::{Envelope, TaskEvidence, TaskVerification};
use serde_json::{Value, json};

#[test]
fn current_envelope_serializes_into_event_sink_without_relaxing_the_native_parser() {
    let id = "00000000000000000000000001";
    let value = json!({"version":1,"id":id,"issuer_kind":"local-dev","issuer":"fixture-issuer",
        "account":"fixture-account","device":"fixture-device","seat":id,"author":"fixture-author",
        "swarm_id":id,"process_generation":"9007199254740993","context":"fixture/topic",
        "role":{"name":"executor","class":"worker"},"role_grant_id":"fixture-grant",
        "role_epoch":"1","priority_class":"worker","sender_sequence":"18446744073709551615",
        "thread_id":id,"reply_to":null,"timestamp":1,"priority":7,"kind":"text",
        "body":{"type":"text","text":"opaque fixture payload"},"attachments":[],"signature":"fixture-signature"});
    let envelope: Envelope = serde_json::from_value(value).unwrap();
    let serialized = serde_json::to_value(&envelope).unwrap();
    let mut request = json!({"subject":"fixture/topic","envelope":serialized});
    request["future_minor_field"] = json!(true);
    let validator =
        jsonschema::validator_for(&Contract::EventSink.method_schema("deliver", false).unwrap())
            .unwrap();
    validator.validate(&request).unwrap();
    request["envelope"]["injected_authority"] = json!(true);
    assert!(!validator.is_valid(&request));
    assert!(serde_json::from_value::<Envelope>(request["envelope"].clone()).is_err());
}

#[test]
fn task_verification_uses_the_existing_wire_shape_and_rejects_authority_fields() {
    let verification = TaskVerification {
        command: "cargo test --locked".into(),
        exit_code: 0,
        evidence: vec![TaskEvidence {
            criterion: "exit zero".into(),
            reference: "fixture.log:1".into(),
        }],
    };
    let serialized = serde_json::to_value(&verification).unwrap();
    let schema: Value = serde_json::from_str(Contract::TaskSystem.schema()).unwrap();
    let verification_schema = json!({"$schema":schema["$schema"],"$defs":schema["$defs"],"$ref":"#/$defs/TaskVerification"});
    let validator = jsonschema::validator_for(&verification_schema).unwrap();
    validator.validate(&serialized).unwrap();
    let mut injected = serialized.clone();
    injected["accepted"] = json!(true);
    assert!(!validator.is_valid(&injected));
    assert!(serde_json::from_value::<TaskVerification>(injected).is_err());
    let args = json!({"task_path":"tasks/fixture.md","etag":"old","signed_record":"opaque fixture","summary":"reported","verify_result":serialized,"future_minor_field":true});
    jsonschema::validator_for(&Contract::TaskSystem.method_schema("report", false).unwrap())
        .unwrap()
        .validate(&args)
        .unwrap();
}
