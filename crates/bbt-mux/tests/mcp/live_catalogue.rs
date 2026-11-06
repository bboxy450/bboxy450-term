// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use super::*;
use std::io::{BufRead, BufReader};

#[test]
fn live_catalogue_notifies_idle_bridge_on_upgrade_downgrade_and_epoch_changes() {
    let f = Fixture::new();
    let mut bridge = f.bridge(f.own.id);
    initialize(&mut bridge);
    // Establish a launch-time worker list before the live authority changes.
    let initial = rpc(&mut bridge, 0, "tools/list", json!({}));
    assert!(
        !initial["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|t| t["name"] == "seat_spawn")
    );
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    let (output, mut writer) = UnixStream::pair().unwrap();
    output
        .set_read_timeout(Some(Duration::from_secs(10)))
        .unwrap();
    let running = std::thread::spawn(move || bridge.run_frames(&receiver, &mut writer));
    let mut output = BufReader::new(output);
    let read = |output: &mut BufReader<UnixStream>| {
        let mut line = String::new();
        output.read_line(&mut line).unwrap();
        serde_json::from_str::<Value>(&line).unwrap()
    };
    let send = |id, method, params| {
        sender
            .send(Ok(serde_json::to_vec(&json!({
                "jsonrpc":"2.0","id":id,"method":method,"params":params
            }))
            .unwrap()))
            .unwrap()
    };
    for (id, class, name) in [
        (1, Class::Oracle, "oracle"),
        (2, Class::Worker, "executor"),
        (3, Class::Worker, "creativity"),
    ] {
        f.registry()
            .update(f.own.id, |r| {
                r.role.class = class;
                r.role.name = name.into();
                Ok(())
            })
            .unwrap();
        // No client request is pending: the production idle loop emits this.
        assert_eq!(
            read(&mut output),
            json!({"jsonrpc":"2.0","method":"notifications/tools/list_changed"})
        );
        send(id, "tools/list", json!({}));
        let catalogue = read(&mut output);
        assert_eq!(
            catalogue["result"]["tools"]
                .as_array()
                .unwrap()
                .iter()
                .any(|t| t["name"] == "seat_spawn"),
            class == Class::Oracle
        );
        send(
            10 + id,
            "tools/call",
            json!({"name":"layout_read","arguments":{}}),
        );
        let call = read(&mut output);
        assert_eq!(call["result"]["isError"], class == Class::Worker);
        if class == Class::Worker {
            assert!(call.to_string().contains("oracle-class"), "{call}");
        }
    }
    drop(sender);
    running.join().unwrap().unwrap();
}

#[test]
fn large_brief_delegate_and_reassign_keep_bounded_path_wakes() {
    let f = Fixture::new();
    let mut metadata = f.task_metadata();
    metadata["status"] = json!("open");
    metadata["assignee"] = json!("none");
    metadata["role"] = json!("executor");
    let document = f.task_file(metadata);
    let body = "🦊".repeat(10 * 1024);
    assert_eq!(body.len(), 40 * 1024);
    std::fs::write(
        f.config.repository.join(f.task_path()),
        format!(
            "---\n{}---\n{body}",
            serde_yaml::to_string(&document.metadata).unwrap()
        ),
    )
    .unwrap();
    for (tool, target) in [("task_delegate", f.own.id), ("task_reassign", f.peer.id)] {
        data(f.mutate(
            f.oracle.id,
            Ulid::new(),
            tool,
            json!({"task_path":f.task_path(),"seat_id":target}),
        ));
        let inbox = data(f.call(target, Ulid::new(), "message_list", json!({})));
        let notice = inbox["page"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|m| m["envelope"]["body"]["event"] == "delegate")
            .unwrap();
        let summary = notice["envelope"]["body"]["summary"].as_str().unwrap();
        assert!(summary.len() <= 16384);
        assert!(summary.starts_with("ROLE: EXECUTOR."));
        assert!(summary.contains(&f.task_path()));
        assert!(summary.contains("task_status") && summary.contains("knowledge_fetch"));
        assert_eq!(f.task_document().body, body);
    }
}

#[test]
fn dead_seat_close_retains_record_but_refuses_wrong_generation_and_live_owner() {
    use bbt_mux::logic::SupervisorConnection;
    let f = Fixture::new();
    let dead = SeatRecord::new(Ulid::new(), f.own.swarm, "Gone", "zsh");
    f.registry().insert(&dead).unwrap();
    let mut args = f.target(dead.id);
    args["expected_process_generation"] = json!("2");
    refused(
        f.mutate(f.oracle.id, Ulid::new(), "seat_close", args),
        "stale target process generation",
    );
    assert_eq!(
        f.registry().get(dead.id).unwrap().lifecycle,
        bbt_mux::registry::Lifecycle::Open
    );
    let op = Ulid::new();
    let closed = f.mutate(f.oracle.id, op, "seat_close", f.target(dead.id));
    let result = data(closed.clone());
    assert_eq!(result["status"], "applied");
    assert_eq!(result["lifecycle"], "closed_retained");
    assert_eq!(result["reason"], "process was gone");
    let record = f.registry().get(dead.id).unwrap();
    assert_eq!(
        record.lifecycle,
        bbt_mux::registry::Lifecycle::ClosedRetained
    );
    assert_eq!(record.lifecycle_revision, 1);
    assert_eq!(
        f.mutate(f.oracle.id, op, "seat_close", f.target(dead.id)),
        closed
    );
    assert_eq!(f.registry().get(dead.id).unwrap().lifecycle_revision, 1);

    // Broken-control witness: viewport absence alone cannot certify death.
    let live = SeatRecord::new(Ulid::new(), f.own.swarm, "No viewport", "zsh");
    f.registry().insert(&live).unwrap();
    let mut spec = LaunchSpec::new("/bin/zsh", f._temp.path());
    spec.seat = live.id;
    spec.generation = live.generation;
    f.connection()
        .request(bbt_mux::Action::Spawn { spec })
        .unwrap();
    refused(
        f.mutate(f.oracle.id, Ulid::new(), "seat_close", f.target(live.id)),
        "supervisor-owned",
    );
    assert_eq!(
        f.registry().get(live.id).unwrap().lifecycle,
        bbt_mux::registry::Lifecycle::Open
    );
    let live_result = f.mutate(f.oracle.id, Ulid::new(), "seat_close", f.target(f.peer.id));
    assert_eq!(
        live_result["structuredContent"]["error"]["code"],
        "unavailable"
    );
    assert_eq!(
        f.registry().get(f.peer.id).unwrap().lifecycle,
        bbt_mux::registry::Lifecycle::Open
    );
}

#[test]
fn task_amend_preserves_omitted_host_stamps_and_names_protected_differences() {
    let f = Fixture::new();
    f.task_file(f.task_metadata());
    let original = data(knowledge_fetch(&f, f.oracle.id, "#1"));
    let mut args = knowledge_update_args(&original);
    args["body"] = json!("Authored amendment without host stamps");
    let stamps = [
        "id",
        "number",
        "created",
        "updated",
        "created_by",
        "updated_by",
        "authorship",
        "provenance",
        "verify_revision",
    ];
    for field in stamps {
        args["frontmatter"].as_object_mut().unwrap().remove(field);
    }
    let amended =
        data(f.mutate(f.oracle.id, Ulid::new(), "knowledge_write", args))["record"].clone();
    for field in ["id", "number", "created"] {
        assert_eq!(
            amended["frontmatter"][field],
            original["frontmatter"][field]
        );
    }
    for (field, replacement) in [
        ("status", json!("accepted")),
        ("assignee", json!("none")),
        ("delegated_by", json!("operator")),
        ("role", json!("oracle")),
    ] {
        let mut args = knowledge_update_args(&amended);
        args["frontmatter"][field] = replacement;
        let refused = f.mutate(f.oracle.id, Ulid::new(), "knowledge_write", args);
        assert_eq!(refused["isError"], true, "{refused}");
        assert!(refused.to_string().contains(field), "{refused}");
    }
}
