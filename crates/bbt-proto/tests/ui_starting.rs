use bbt_proto::gui::{UiRequest, UiStartingFrame, UiStartingPane, UiStartingState};
use bbt_proto::{TabId, seat_state::Placement};
use serde_json::{Value, json};
use ulid::Ulid;

// keeps: decision #340 (a starting pane is an operation, never a seat identity)
#[test]
fn starting_and_named_refusal_have_an_operation_but_no_seat_record_or_authority() {
    let mut pane = UiStartingPane {
        op: Ulid(1),
        name: "Vesper Greywind".into(),
        harness_kind: "claude".into(),
        swarm: Ulid(2),
        placement: Placement::NewTab {
            tab: TabId::from_u128(3),
        },
        state: UiStartingState::Starting,
        rect: [300., 40., 900., 600.],
        focused: true,
        admitted_frame_seq: 8,
        first_frame_seq: Some(9),
    };
    for state in [
        UiStartingState::Starting,
        UiStartingState::Refused {
            reason: "spawn_timeout: no reply before the launch deadline".into(),
        },
    ] {
        pane.state = state;
        let value = serde_json::to_value(&pane).unwrap();
        assert_eq!(value["op"], json!(Ulid(1)));
        for forbidden in ["seat", "seat_id", "record", "key", "grant", "generation"] {
            assert!(
                value.get(forbidden).is_none(),
                "starting pane has {forbidden}"
            );
        }
        assert_eq!(
            serde_json::from_value::<UiStartingPane>(value).unwrap(),
            pane
        );
    }
}

// keeps: decision #340 (the real-seat reply retains the observed first starting frame)
#[test]
fn starting_frame_and_gui_owned_listing_have_explicit_wire_shapes() {
    let frame = UiStartingFrame {
        admitted_frame_seq: 8,
        frame_seq: 9,
        rect: [300., 40., 900., 600.],
        name: "Vesper Greywind".into(),
    };
    let encoded = serde_json::to_value(&frame).unwrap();
    assert_eq!(
        encoded,
        json!({"admitted_frame_seq":8,"frame_seq":9,"rect":[300.,40.,900.,600.],
               "name":"Vesper Greywind"})
    );
    assert_eq!(
        serde_json::from_value::<UiStartingFrame>(encoded).unwrap(),
        frame
    );
    assert!(UiRequest::Seats.validate().is_ok());
    let request: Value = serde_json::to_value(UiRequest::Seats).unwrap();
    assert_eq!(request, json!({"kind":"seats"}));
}
