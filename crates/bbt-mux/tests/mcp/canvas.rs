// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use super::*;
fn add(i: usize) -> Value {
    json!({"op":"add","element":{"id":format!("node-{i}"),"tags":["plan"],"kind":"service","content":{"type":"shape","bounds":{"origin":[(i%5)*150,(i/5)*100],"size":[120,64]},"shape":"rounded","label":format!("Service {i}")}}})
}
#[test]
fn canvas_mcp_script_ten_elements_three_batches_restart_list_and_gpu_image() {
    let mut f = Fixture::new();
    let mut bridge = f.bridge(f.own.id);
    initialize(&mut bridge);
    let catalogue = rpc(&mut bridge, 2, "tools/list", json!({}));
    for tool in ["canvas_open", "canvas_list", "canvas_draw", "canvas_read"] {
        assert!(
            catalogue["result"]["tools"]
                .as_array()
                .unwrap()
                .iter()
                .any(|t| t["name"] == tool),
            "{tool}"
        );
    }
    let created = rpc(
        &mut bridge,
        3,
        "tools/call",
        json!({"name":"canvas_open","arguments":{"title":"Ten services","tags":["plan"]}}),
    );
    let opened = data(created["result"].clone());
    let id = opened["id"].clone();
    assert_eq!(opened["shown"], false);
    assert_eq!(opened["presentation"], "detached");
    assert!(opened["reason"].as_str().unwrap().contains("ui_detached"));
    let mut seq = 0;
    for (index, range) in [0..3, 3..7, 7..10].into_iter().enumerate() {
        let ops: Vec<_> = range.map(add).collect();
        let result = rpc(
            &mut bridge,
            4 + index as u64,
            "tools/call",
            json!({"name":"canvas_draw","arguments":{"id":id,"ops":ops,"expect_seq":seq}}),
        );
        seq = data(result["result"].clone())["seq"].as_u64().unwrap();
    }
    let before = data(f.call(f.own.id, Ulid::new(), "canvas_read", json!({"id":id})));
    assert_eq!(before["elements"].as_array().unwrap().len(), 10);
    assert_eq!(before["seq"], 10);
    drop(bridge);
    f.restart();
    let result = f.call(
        f.own.id,
        Ulid::new(),
        "canvas_read",
        json!({"id":id,"image":true}),
    );
    let after = data(result.clone());
    assert_eq!(before["digest"], after["digest"]);
    assert_eq!(before["elements"], after["elements"]);
    let image = result["content"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["type"] == "image")
        .unwrap();
    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(image["data"].as_str().unwrap())
        .unwrap();
    assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes))
        .read_info()
        .unwrap();
    let mut pixels = vec![0; decoder.output_buffer_size().unwrap()];
    let info = decoder.next_frame(&mut pixels).unwrap();
    assert_eq!((info.width, info.height), (960, 640));
    assert!(
        pixels.as_chunks::<4>().0.iter().any(|p| p != &pixels[..4]),
        "actual drawing has ink"
    );
    let list = data(f.call(f.own.id, Ulid::new(), "canvas_list", json!({"tag":"plan"})));
    assert_eq!(list["canvases"][0]["id"], id);
    let log = f
        .config
        .repository
        .join(".bboxy/canvases")
        .join(format!("{}.log.jsonl", opened["slug"].as_str().unwrap()));
    assert_eq!(std::fs::read_to_string(log).unwrap().lines().count(), 3);
}
#[test]
fn canvas_mcp_creator_oracle_swarm_fences_and_duplicate_restart() {
    let mut f = Fixture::new();
    let opened = data(f.call(
        f.own.id,
        Ulid::new(),
        "canvas_open",
        json!({"title":"Shared"}),
    ));
    let id = opened["id"].clone();
    let args = json!({"id":id,"ops":[add(0)],"expect_seq":0});
    let op = Ulid::new();
    assert_eq!(
        f.call(f.peer.id, Ulid::new(), "canvas_draw", args.clone())["structuredContent"]["error"]["code"],
        "canvas_draw_denied"
    );
    let once = data(f.call(f.own.id, op, "canvas_draw", args.clone()));
    f.restart();
    assert_eq!(
        once,
        data(f.call(f.own.id, op, "canvas_draw", args.clone()))
    );
    assert_eq!(
        f.call(f.own.id, op, "canvas_draw", json!({"id":id,"ops":[add(1)]}))["structuredContent"]["error"]
            ["code"],
        "op_id_reused"
    );
    let late = data(f.call(
        f.oracle.id,
        Ulid::new(),
        "canvas_draw",
        json!({"id":id,"ops":[{"op":"remove","id":"node-0"}],"expect_seq":0}),
    ));
    assert_eq!(late["stale"], true);
    assert_eq!(late["seq"], 2);
    data(f.call(f.own.id, Ulid::new(), "canvas_draw", args)); // same stable id stays removed
    assert_eq!(
        data(f.call(f.peer.id, Ulid::new(), "canvas_read", json!({"id":id})))["elements"],
        json!([])
    );
    assert_eq!(
        f.call(f.outside.id, Ulid::new(), "canvas_read", json!({"id":id}))["structuredContent"]["error"]
            ["code"],
        "canvas_not_found"
    );
    assert!(
        data(f.call(f.outside.id, Ulid::new(), "canvas_list", json!({})))["canvases"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    for query in [json!({"number":opened["number"]}), json!({"id":id})] {
        assert_eq!(
            data(f.call(f.peer.id, Ulid::new(), "canvas_open", query))["id"],
            id
        );
    }
}
#[test]
fn canvas_mcp_strict_semantics_caps_and_shared_ctl_path() {
    let f = Fixture::new();
    f.prepare_operator();
    let unscoped = f
        .runtime
        .block_on(f.client().gui_knowledge_scoped(
            "canvas_list".into(),
            json!({"op_id":Ulid::new()}),
            None,
        ))
        .unwrap()
        .unwrap_err();
    assert_eq!(unscoped.code, "canvas_swarm_required");
    // This is the same admitted GUI transport that ctl canvas calls.
    let scope = Some(bbt_proto::gui::SwarmScope {
        swarm: f.own.swarm,
        revision: 0,
    });
    let opened = f
        .runtime
        .block_on(f.client().gui_knowledge_scoped(
            "canvas_open".into(),
            json!({"op_id":Ulid::new(),"title":"ctl canvas"}),
            scope,
        ))
        .unwrap()
        .unwrap();
    let id = opened["id"].clone();
    let mut styled = add(0);
    styled["element"]["color"] = json!("red");
    assert_eq!(
        f.call(
            f.own.id,
            Ulid::new(),
            "canvas_draw",
            json!({"id":id,"ops":[styled]})
        )["structuredContent"]["error"]["code"],
        "invalid_arguments"
    );
    let too_big = vec![add(0); 257];
    let refused = f
        .runtime
        .block_on(f.client().gui_knowledge_scoped(
            "canvas_draw".into(),
            json!({"op_id":Ulid::new(),"id":id,"ops":too_big}),
            scope,
        ))
        .unwrap()
        .unwrap_err();
    assert!(refused.message.contains("MAX_BATCH_OPS"));
    let drawn = f
        .runtime
        .block_on(f.client().gui_knowledge_scoped(
            "canvas_draw".into(),
            json!({"op_id":Ulid::new(),"id":id,"ops":[add(0)]}),
            scope,
        ))
        .unwrap()
        .unwrap();
    assert_eq!(drawn["seq"], 1);
    let read = data(f.call(f.own.id, Ulid::new(), "canvas_read", json!({"id":id})));
    assert_eq!(read["elements"].as_array().unwrap().len(), 1);
}
#[test]
fn canvas_list_is_bounded_scoped_and_activity_ordered() {
    let f = Fixture::new();
    let mut first = Value::Null;
    for i in 0..33 {
        let record = data(f.call(
            f.own.id,
            Ulid::new(),
            "canvas_open",
            json!({"title":format!("Canvas {i}"),"tags":["page"]}),
        ));
        if i == 0 {
            first = record;
        }
    }
    data(f.call(
        f.own.id,
        Ulid::new(),
        "canvas_draw",
        json!({"id":first["id"],"ops":[add(0)]}),
    ));
    let page = data(f.call(f.own.id, Ulid::new(), "canvas_list", json!({"tag":"page"})));
    assert_eq!(page["canvases"].as_array().unwrap().len(), 32);
    assert_eq!(page["canvases"][0]["id"], first["id"]);
    let next = data(f.call(
        f.own.id,
        Ulid::new(),
        "canvas_list",
        json!({"tag":"page","cursor":page["next"]}),
    ));
    assert_eq!(next["canvases"].as_array().unwrap().len(), 1);
    assert!(next["next"].is_null());
    assert_eq!(
        f.call(
            f.peer.id,
            Ulid::new(),
            "canvas_list",
            json!({"tag":"other","cursor":page["next"]})
        )["structuredContent"]["error"]["code"],
        "invalid_canvas_cursor"
    );
}

#[test]
fn canvas_busy_retry_recovers_same_operation_after_restart() {
    let mut f = Fixture::new();
    let opened = data(f.call(
        f.own.id,
        Ulid::new(),
        "canvas_open",
        json!({"title":"Retry"}),
    ));
    let path = f
        .config
        .repository
        .join(".bboxy/canvases")
        .join(format!("{}.log.jsonl", opened["slug"].as_str().unwrap()));
    let locked = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .unwrap();
    locked.try_lock().unwrap();
    let op = Ulid::new();
    let args = json!({"id":opened["id"],"ops":[add(0)]});
    assert_eq!(
        f.call(f.own.id, op, "canvas_draw", args.clone())["structuredContent"]["error"]["code"],
        "canvas_busy"
    );
    assert_eq!(f.original_operation(f.own.id, op)["status"], "unknown");
    drop(locked);
    f.restart();
    assert_eq!(data(f.call(f.own.id, op, "canvas_draw", args))["seq"], 1);
}

#[test]
fn canvas_concurrent_changed_operation_id_cannot_modify_two_canvases() {
    let f = Fixture::new();
    let left = data(f.call(
        f.own.id,
        Ulid::new(),
        "canvas_open",
        json!({"title":"Left"}),
    ));
    let right = data(f.call(
        f.own.id,
        Ulid::new(),
        "canvas_open",
        json!({"title":"Right"}),
    ));
    let op = Ulid::new();
    let call = |id: Value| Call {
        op_id: op,
        tool: "canvas_draw".into(),
        args: json!({"id":id,"ops":[add(0)]}),
    };
    let (a, b) = f.runtime.block_on(async {
        tokio::join!(
            f.client()
                .control_call(f.own.id, 1, call(left["id"].clone())),
            f.client()
                .control_call(f.own.id, 1, call(right["id"].clone()))
        )
    });
    let results = [a.unwrap(), b.unwrap()];
    assert_eq!(
        results.iter().filter(|r| r["isError"] == false).count(),
        1,
        "{results:?}"
    );
    assert_eq!(
        results
            .iter()
            .filter(|r| r["structuredContent"]["error"]["code"] == "op_id_reused")
            .count(),
        1
    );
    let total: usize = [left, right]
        .into_iter()
        .map(|opened| {
            data(f.call(
                f.own.id,
                Ulid::new(),
                "canvas_read",
                json!({"id":opened["id"]}),
            ))["elements"]
                .as_array()
                .unwrap()
                .len()
        })
        .sum();
    assert_eq!(total, 1);
}

#[test]
fn canvas_open_lost_create_result_recovers_its_id_among_equal_titles() {
    let op = Ulid::new();
    let f = Fixture::open_seeded(false, false, |root, seat| {
        let store = bboxy450_canvas::store::Store::new(root.join(".bboxy/canvases"));
        let metadata = bboxy450_canvas::CanvasRecord {
            title: "Same title".into(),
            swarm: seat.swarm.to_string(),
            tags: vec![],
            creator: format!("seat:{}", seat.id),
            task: seat.tasks.first().map(ToString::to_string),
        };
        store
            .create(&Ulid::new().to_string(), metadata.clone())
            .unwrap();
        store.create(&op.to_string(), metadata).unwrap();
    });
    let result = data(f.call(f.own.id, op, "canvas_open", json!({"title":"Same title"})));
    assert_eq!(result["id"], op.to_string());
}

#[test]
fn canvas_close_retires_the_canvas_for_creator_or_oracle_and_retries_safely() {
    let mut f = Fixture::new();
    let opened = data(f.call(f.own.id, Ulid::new(), "canvas_open", json!({"title":"Gone"})));
    let id = opened["id"].clone();
    assert_eq!(
        f.call(f.peer.id, Ulid::new(), "canvas_close", json!({"id":id}))["structuredContent"]["error"]["code"],
        "canvas_close_denied"
    );
    let op = Ulid::new();
    let closed = data(f.call(f.own.id, op, "canvas_close", json!({"id":id})));
    assert_eq!(closed["closed"], true);
    assert!(
        data(f.call(f.own.id, Ulid::new(), "canvas_list", json!({})))["canvases"]
            .as_array()
            .unwrap()
            .iter()
            .all(|c| c["id"] != id)
    );
    assert_eq!(
        f.call(f.own.id, Ulid::new(), "canvas_read", json!({"id":id}))["structuredContent"]["error"]["code"],
        "canvas_not_found"
    );
    // The same operation replays its result; a fresh close of a closed canvas is a no-op.
    f.restart();
    assert_eq!(closed, data(f.call(f.own.id, op, "canvas_close", json!({"id":id}))));
    assert_eq!(
        data(f.call(f.oracle.id, Ulid::new(), "canvas_close", json!({"id":id})))["already"],
        true
    );
    // The archive lists it; restore brings it back for the creator or Oracle.
    let archived = data(f.call(f.own.id, Ulid::new(), "canvas_archived_list", json!({})));
    assert_eq!(archived["canvases"][0]["id"], id);
    assert_eq!(
        f.call(f.peer.id, Ulid::new(), "canvas_restore", json!({"id":id}))["structuredContent"]["error"]["code"],
        "canvas_restore_denied"
    );
    let restored = data(f.call(f.oracle.id, Ulid::new(), "canvas_restore", json!({"id":id})));
    assert_eq!(restored["restored"], true);
    assert_eq!(
        data(f.call(f.own.id, Ulid::new(), "canvas_read", json!({"id":id})))["id"],
        id
    );
    assert!(
        data(f.call(f.own.id, Ulid::new(), "canvas_archived_list", json!({})))["canvases"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}
