use bbt_proto::gui::{UiRequest, UiSpawnGateCommand};
use serde_json::json;
use ulid::Ulid;

// keeps: e2e-script-map §L2 — a fixture releases one exact captured operation.
#[test]
fn launch_gate_wire_is_explicit_and_release_requires_a_non_nil_operation() {
    for (command, value) in [
        (
            UiSpawnGateCommand::Arm,
            json!({"kind":"spawn_gate","body":{"kind":"arm"}}),
        ),
        (
            UiSpawnGateCommand::Release { op: Ulid(1) },
            json!({"kind":"spawn_gate","body":{"kind":"release","op":"00000000000000000000000001"}}),
        ),
    ] {
        let request = UiRequest::SpawnGate(command);
        request.validate().unwrap();
        assert_eq!(serde_json::to_value(&request).unwrap(), value);
        assert_eq!(serde_json::from_value::<UiRequest>(value).unwrap(), request);
    }
    assert!(
        UiRequest::SpawnGate(UiSpawnGateCommand::Release { op: Ulid::nil() })
            .validate()
            .is_err()
    );
}
