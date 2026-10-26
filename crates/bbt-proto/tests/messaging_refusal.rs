//! MA-5: a refusal crosses the existing Messaging reply as typed data.
use bbt_proto::{
    GuiReply,
    messaging::{MessageRefusal, Response},
};

#[test]
fn unknown_thread_retains_its_kind_message_and_hint() {
    let json = r#"{"kind":"Messaging","body":{"op":"00000000000000000000000001","response":{"kind":"refused","error":{"kind":"unknown_thread","message":"reply target is not visible","hint":"Use reply_to 00000000000000000000000002 from a visible thread."}}}}"#;
    let GuiReply::Messaging {
        response: Response::Refused { error },
        ..
    } = serde_json::from_str(json).unwrap()
    else {
        panic!("typed messaging refusal")
    };
    assert_eq!(error.kind, "unknown_thread");
    assert_eq!(error.message, "reply target is not visible");
    assert_eq!(
        error.hint,
        "Use reply_to 00000000000000000000000002 from a visible thread."
    );
}

#[test]
fn refusal_requires_all_three_fields_and_rejects_unrelated_data() {
    for malformed in [
        r#"{"message":"unknown reply","hint":"list messages"}"#,
        r#"{"kind":"unknown_thread","hint":"list messages"}"#,
        r#"{"kind":"unknown_thread","message":"unknown reply"}"#,
        r#"{"kind":"unknown_thread","message":"unknown reply","hint":null}"#,
        r#"{"kind":"unknown_thread","message":"unknown reply","hint":"list messages","extra":true}"#,
    ] {
        assert!(
            serde_json::from_str::<MessageRefusal>(malformed).is_err(),
            "{malformed}"
        );
    }
}
