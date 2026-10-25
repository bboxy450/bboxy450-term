use bbt_proto::gui::{UiClockCommand, UiLayout, UiRequest};
use serde_json::json;

// keeps: spec ctl-cli §1
#[test]
fn clock_actions_round_trip_inside_the_existing_ui_request() {
    for (command, body) in [
        (UiClockCommand::Freeze, json!({"action":"freeze"})),
        (
            UiClockCommand::Advance { ms: 450 },
            json!({"action":"advance","ms":450}),
        ),
        (UiClockCommand::Resume, json!({"action":"resume"})),
    ] {
        let request = UiRequest::Clock(command);
        let wire = json!({"kind":"clock","body":body});
        assert_eq!(serde_json::to_value(&request).unwrap(), wire);
        assert_eq!(serde_json::from_value::<UiRequest>(wire).unwrap(), request);
    }
    assert!(
        serde_json::from_value::<UiRequest>(
            json!({"kind":"clock","body":{"action":"advance","ms":-1}})
        )
        .is_err()
    );
    assert!(
        serde_json::from_value::<UiRequest>(
            json!({"kind":"clock","body":{"action":"advance","ms":true}})
        )
        .is_err()
    );
}

// keeps: spec ctl-cli §1
#[test]
fn an_older_layout_does_not_invent_an_animation_clock() {
    let wire = json!({"frame_seq":7,"size":[800.,600.],"swarm":null,"tab":null,"focused":null,"launcher_open":false,"page":null,"document":null,"panes":[],"tree":[]});
    let layout: UiLayout = serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(layout.clock, None);
    assert!(serde_json::to_value(layout).unwrap().get("clock").is_none());
}
