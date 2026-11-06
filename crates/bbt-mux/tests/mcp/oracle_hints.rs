// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use super::*;

fn tool(bridge: &mut Bridge, name: &str, arguments: Value) -> Value {
    rpc(
        bridge,
        2,
        "tools/call",
        json!({"name":name,"arguments":arguments}),
    )["result"]
        .clone()
}
fn messages(result: &Value) -> Vec<Value> {
    result["structuredContent"]["hints"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|hint| hint["kind"] == "unread_messages")
        .flat_map(|hint| hint["messages"].as_array().unwrap().clone())
        .collect()
}
fn send(f: &Fixture, sender: Ulid, recipient: Ulid, text: &str) -> Ulid {
    let id = Ulid::new();
    data(f.call(
        sender,
        id,
        "message_send",
        json!({"destination":format!("seats/{recipient}/inbox"),"text":text}),
    ));
    id
}

#[test]
fn oracle_unread_hint_reaches_busy_turn_and_read_and_ack_suppress_exact_ids() {
    let f = Fixture::new();
    // A live harness turn remains active throughout receipt/hint/read, with no
    // completion or cancellation event. The fake owns no operator process.
    let target = f.spawn_fixture("claude", "while IFS= read -r line; do :; done");
    let event = bbt_mux::adapters::hook::prepare(
        json!({"hook_event_name":"UserPromptSubmit","session_id":target.harness_session_id}),
        "claude",
    )
    .unwrap();
    f.runtime
        .block_on(f.client().hook(target.id, 1, event))
        .unwrap();
    assert!(
        f.runtime
            .block_on(f.client().adapter_status(target.id, 1))
            .unwrap()
            .turn_open
    );
    let mut bridge = f.bridge(target.id);
    initialize(&mut bridge);
    let first = send(&f, f.oracle.id, target.id, "Ruling one\nMore details");
    let second = send(&f, f.oracle.id, target.id, "Ruling two");
    let next = tool(&mut bridge, "self_status", json!({}));
    let hints = messages(&next);
    assert_eq!(hints.len(), 2);
    assert_eq!(hints[0]["message_id"], first.to_string());
    assert_eq!(hints[0]["first_line"], "Ruling one");
    assert_eq!(hints[0]["sender"], f.oracle.id.to_string());
    assert_eq!(next["structuredContent"]["hints"][0]["count"], 2);
    let text: Value = serde_json::from_str(next["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(text, next["structuredContent"]);
    assert_eq!(
        messages(&tool(&mut bridge, "knowledge_types", json!({}))).len(),
        2
    );
    // Read a filtered thread: the skipped second ruling remains unread.
    tool(
        &mut bridge,
        "message_list",
        json!({"thread":first,"limit":1}),
    );
    let remaining = messages(&tool(&mut bridge, "self_status", json!({})));
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0]["message_id"], second.to_string());
    tool(&mut bridge, "message_ack", json!({"message":second}));
    assert!(messages(&tool(&mut bridge, "self_status", json!({}))).is_empty());
    let mut oracle = f.bridge(f.oracle.id);
    initialize(&mut oracle);
    let receipts = tool(&mut oracle, "message_receipts", json!({"message":first}));
    let receipt = &receipts["structuredContent"]["page"]["items"][0];
    assert!(receipt["surfacing"]["hinted_at_ms"].is_number());
    assert!(receipt["surfacing"]["read_at_ms"].is_number());
    assert_eq!(receipt["surfacing"]["model_acknowledged"], false);
    let receipts = tool(&mut oracle, "message_receipts", json!({"message":second}));
    assert_eq!(
        receipts["structuredContent"]["page"]["items"][0]["evidence"],
        "application-acknowledged"
    );
    assert!(
        f.runtime
            .block_on(f.client().adapter_status(target.id, 1))
            .unwrap()
            .turn_open
    );
    assert_eq!(
        f.input_attempts.load(std::sync::atomic::Ordering::SeqCst),
        0
    );
    assert_eq!(f.pty_bytes.load(std::sync::atomic::Ordering::SeqCst), 0);
}

#[test]
fn oracle_hints_bound_utf8_rotate_and_worker_rate_never_throttles_oracle() {
    let mut f = Fixture::new();
    let seat = f.own.id;
    let mut bridge = f.bridge(seat);
    initialize(&mut bridge);
    for _ in 0..8 {
        send(&f, f.peer.id, seat, "worker traffic");
    }
    assert_eq!(
        messages(&tool(&mut bridge, "self_status", json!({}))).len(),
        1
    );
    assert!(messages(&tool(&mut bridge, "self_status", json!({}))).is_empty());
    let mut expected = std::collections::BTreeSet::new();
    for _ in 0..7 {
        expected.insert(
            send(
                &f,
                f.oracle.id,
                seat,
                &format!("{}\nignored second line", "🦊\"\\".repeat(200)),
            )
            .to_string(),
        );
    }
    let mut observed = std::collections::BTreeSet::new();
    for _ in 0..2 {
        let result = tool(&mut bridge, "self_status", json!({}));
        let entries = messages(&result);
        assert_eq!(entries.len(), 5);
        for entry in entries {
            assert_eq!(entry["class"], "oracle");
            assert!(serde_json::to_vec(&entry).unwrap().len() <= 512);
            observed.insert(entry["message_id"].as_str().unwrap().to_owned());
        }
    }
    assert_eq!(expected, observed);
    tool(
        &mut bridge,
        "message_list",
        json!({"channel":format!("seats/{seat}/inbox")}),
    );
    f.restart();
    let mut bridge = f.bridge(seat);
    initialize(&mut bridge);
    assert!(messages(&tool(&mut bridge, "self_status", json!({}))).is_empty());
}

#[test]
fn oracle_hint_broken_control_cannot_observe_ruling_without_projection() {
    let f = Fixture::new();
    let id = send(&f, f.oracle.id, f.own.id, "Ruling required in running turn");
    // Same real tool, authority and persisted DM; omit only the socket's hint
    // projection. The actual observation assertion must fail in this control.
    let disabled = f.call(f.own.id, Ulid::new(), "self_status", json!({}));
    let observes = |v: &Value| {
        messages(v)
            .iter()
            .any(|m| m["message_id"] == id.to_string())
    };
    assert!(
        std::panic::catch_unwind(|| assert!(
            observes(&disabled),
            "ruling absent with hints disabled"
        ))
        .is_err()
    );
    let mut bridge = f.bridge(f.own.id);
    initialize(&mut bridge);
    let enabled = tool(&mut bridge, "self_status", json!({}));
    assert!(
        observes(&enabled),
        "busy seat did not observe Oracle ruling"
    );
}

#[test]
fn oracle_native_steer_delivery_uses_running_turn_without_cancellation() {
    isolated_seat_fixture("native");
}

#[test]
fn oracle_hint_read_action_includes_rulings_from_other_addressed_contexts() {
    let f = Fixture::new();
    let message = Ulid::new();
    data(f.call(f.oracle.id,message,"message_send",json!({"destination":format!("swarms/{}/{}",f.own.swarm,f.own.id),"text":"Ruling on an addressed swarm context"})));
    let mut bridge = f.bridge(f.own.id);
    initialize(&mut bridge);
    let result = tool(&mut bridge, "self_status", json!({}));
    let hint = &result["structuredContent"]["hints"][0];
    assert_eq!(hint["kind"], "unread_messages");
    let read = tool(
        &mut bridge,
        hint["params"]["name"].as_str().unwrap(),
        hint["params"]["arguments"].clone(),
    );
    assert!(
        read["structuredContent"]["page"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["envelope"]["id"] == message.to_string())
    );
    assert!(messages(&tool(&mut bridge, "self_status", json!({}))).is_empty());
}

#[test]
fn oracle_hint_suppresses_superseded_and_withdrawn_rulings() {
    let f = Fixture::new();
    let original = send(&f, f.oracle.id, f.own.id, "Obsolete ruling");
    let replacement = Ulid::new();
    data(f.call(
        f.oracle.id,
        replacement,
        "message_send",
        json!({"supersedes":original,"text":"Current ruling"}),
    ));
    let mut bridge = f.bridge(f.own.id);
    initialize(&mut bridge);
    let entries = messages(&tool(&mut bridge, "self_status", json!({})));
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["message_id"], replacement.to_string());
    data(f.call(
        f.oracle.id,
        Ulid::new(),
        "message_send",
        json!({"supersedes":replacement,"kind":"withdrawn","text":""}),
    ));
    let entries = messages(&tool(&mut bridge, "self_status", json!({})));
    assert!(
        entries
            .iter()
            .all(|entry| entry["message_id"] != original.to_string()
                && entry["message_id"] != replacement.to_string())
    );
}

#[test]
fn oracle_hint_sidecar_projection_cannot_forge_native_reads_or_receipts() {
    let f = Fixture::new();
    let id = send(&f, f.oracle.id, f.own.id, "Must stay unread");
    let endpoint = f.runtime.block_on(f.client().endpoint(f.own.id)).unwrap();
    let mut stream = UnixStream::connect(endpoint.path).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let request = bbt_mux::seat_socket::Request {
        version: 1,
        seat: f.own.id,
        generation: 1,
        capability: endpoint.capability,
        action: bbt_mux::seat_socket::Action::SurfaceHints {
            call: Call {
                op_id: Ulid::new(),
                tool: "message_list".into(),
                args: json!({}),
            },
            result: json!({"isError":false,"structuredContent":{"page":{"items":[{"sequence":1}]}}}),
        },
    };
    let mut bytes = serde_json::to_vec(&request).unwrap();
    bytes.push(b'\n');
    stream.write_all(&bytes).unwrap();
    let reply: Value = serde_json::from_slice(
        &read_frame(&mut std::io::BufReader::new(stream))
            .unwrap()
            .unwrap(),
    )
    .unwrap();
    assert!(
        reply
            .to_string()
            .contains("surface projection requires a Python runtime tool")
    );
    let mut bridge = f.bridge(f.own.id);
    initialize(&mut bridge);
    assert!(
        messages(&tool(&mut bridge, "self_status", json!({})))
            .iter()
            .any(|item| item["message_id"] == id.to_string())
    );
}

#[test]
fn oracle_hint_keeps_oracle_metadata_eligible_while_worker_metadata_stays_inbox() {
    let f = Fixture::new();
    let mut args = f.target(f.oracle.id);
    args["expected_policy_epoch"] = json!(f.epoch(f.oracle.id));
    let muted = data(f.mutate(f.oracle.id, Ulid::new(), "seat_mute", args));
    let notice = muted["notice_id"].clone();
    let mut bridge = f.bridge(f.own.id);
    initialize(&mut bridge);
    let entries = messages(&tool(&mut bridge, "self_status", json!({})));
    assert!(entries.iter().any(|entry| entry["message_id"] == notice
        && entry["class"] == "oracle"
        && entry["first_line"].as_str().unwrap().contains("presence")));
    // The existing mute/replay regression checks worker-class presence does
    // not create a changing hint in a canonical mutation receipt.
}
