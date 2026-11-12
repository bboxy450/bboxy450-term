// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
// Adapted from this project's logic::dream_public_api_tests admitted MCP fixture.
fn dream_read_client(f: &Fixture, seat: Ulid, name: &str, args: Value) -> Value {
    let reply = f.mcp_tool_reply(seat, name, args);
    assert!(reply.get("error").is_none(), "{reply}");
    let result = reply["result"].clone();
    let schema = advertised_output_schema(f, name);
    jsonschema::validator_for(&schema)
        .unwrap()
        .validate(&result["structuredContent"])
        .unwrap();
    let text: Value = serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(text, result["structuredContent"]);
    let mut domain = result["structuredContent"].clone();
    let object = domain.as_object_mut().unwrap();
    assert!(object.remove("op_id").is_some());
    assert!(
        !object
            .remove("hints")
            .unwrap()
            .as_array()
            .unwrap()
            .is_empty()
    );
    serde_json::from_value::<bbt_proto::dream_read::Reply>(domain).unwrap();
    assert!(
        serde_json::to_vec(&result).unwrap().len() <= 4096,
        "{result}"
    );
    result["structuredContent"].clone()
}
#[test]
fn dream_read_actual_admitted_empty_cold_lesson_foreign_and_bad_requests() {
    let f = Fixture::open_with_control(Some(vec![]), Some("unprepared"));
    let before = f.counts();
    for name in ["dream_search", "dream_read"] {
        let schema = advertised_output_schema(&f, name);
        assert_eq!(schema["type"], "object");
    }
    let reply = dream_read_client(
        &f,
        f.own.id,
        "dream_search",
        json!({"v":1,"kind":"observation"}),
    );
    assert_eq!(reply["reason"], "not_prepared");
    let reply = dream_read_client(&f, f.own.id, "dream_search", json!({"v":1,"kind":"lesson"}));
    assert_eq!(reply["reason"], "no_published_input");
    let reply = dream_read_client(
        &f,
        f.peer.id,
        "dream_read",
        json!({"v":1,"handle":"foreign","view":"summary"}),
    );
    assert_eq!(reply["error"]["code"], "not_found");
    let reply = dream_read_client(
        &f,
        f.own.id,
        "dream_search",
        json!({"v":1,"kind":"observation","root":"injected"}),
    );
    assert_eq!(reply["error"]["code"], "bad_request");
    let reply = dream_read_client(
        &f,
        f.own.id,
        "dream_search",
        json!({"v":2,"kind":"observation"}),
    );
    assert_eq!(reply["error"]["code"], "version_mismatch");
    f.unchanged(before, "Q1 admitted cold/foreign/lesson/schema reads");
    drop(f);
    let f = Fixture::open_with_rows(Some(vec![]));
    let reply = dream_read_client(
        &f,
        f.own.id,
        "dream_search",
        json!({"v":1,"kind":"observation","budget_tokens":4096}),
    );
    assert_eq!(reply["status"], "available");
    assert_eq!(reply["items"], json!([]));
    assert_eq!(reply["coverage"]["complete"], true);
}

#[test]
fn dream_read_two_session_cluster_episode_origins_and_honest_pagination() {
    let mut rows = vec![w::Row::Cluster(w::Cluster {
        repo: w::Repo("private.invalid/public-api".into()),
        fingerprint: w::Hex("a".repeat(64)),
        count: 2,
        first_ms: w::Number(1),
        last_ms: w::Number(2),
    })];
    for n in 1..=2 {
        let session = w::Ulid(Ulid::from(n as u128).to_string());
        rows.push(w::Row::Episode(w::Episode {
            session: session.clone(),
            turn: w::Ulid(Ulid::from(10 + n as u128).to_string()),
            origin: w::Origin {
                session,
                seq: w::Number(n),
                digest: format!("sha256:{}", "b".repeat(64)),
            },
            fingerprints: vec![w::Hex("a".repeat(64))],
            first_ms: w::Number(n),
            last_ms: w::Number(n),
        }));
    }
    let f = Fixture::open_with_rows(Some(rows));
    let before = f.counts();
    let mut page = dream_read_client(
        &f,
        f.own.id,
        "dream_search",
        json!({"v":1,"kind":"observation","fingerprint":"a".repeat(64),"limit":1,"budget_tokens":4096}),
    );
    let mut hits = vec![];
    loop {
        hits.push(page["items"][0].clone());
        let Some(cursor) = page["next"].as_str().map(str::to_owned) else {
            break;
        };
        let mixed = dream_read_client(
            &f,
            f.own.id,
            "dream_search",
            json!({"v":1,"kind":"observation","cursor":cursor,"limit":2,"budget_tokens":4096}),
        );
        assert_eq!(mixed["error"]["code"], "stale_cursor");
        page = dream_read_client(
            &f,
            f.own.id,
            "dream_search",
            json!({"v":1,"kind":"observation","cursor":cursor,"limit":1,"budget_tokens":4096}),
        );
    }
    assert_eq!(hits.len(), 3);
    assert_eq!(
        hits.iter()
            .filter(|h| h["proof"]["kind"] == "closed_ir_cluster")
            .count(),
        1
    );
    let origins: std::collections::BTreeSet<_> = hits
        .iter()
        .filter_map(|h| h["proof"]["origin"]["session"].as_str())
        .collect();
    assert_eq!(origins.len(), 2);
    for hit in hits {
        let read = dream_read_client(
            &f,
            f.own.id,
            "dream_read",
            json!({"v":1,"handle":hit["handle"],"view":"summary","budget_tokens":4096}),
        );
        assert_eq!(read["item"], hit);
    }
    f.unchanged(before, "two-session immutable row/cursor reads");
}

#[test]
fn dream_read_actual_row_binding_kind_fabricated_origin_and_member_controls() {
    for corrupt in ["row", "kind", "members", "field"] {
        let row = w::Row::Cluster(w::Cluster {
            repo: w::Repo("private.invalid/public-api".into()),
            fingerprint: w::Hex("a".repeat(64)),
            count: 2,
            first_ms: w::Number(1),
            last_ms: w::Number(2),
        });
        let f = Fixture::open_with_control(Some(vec![row]), Some(corrupt));
        let reply = dream_read_client(
            &f,
            f.own.id,
            "dream_search",
            json!({"v":1,"kind":"observation","budget_tokens":4096}),
        );
        assert_eq!(
            reply["error"]["code"], "protocol_error",
            "control {corrupt}: {reply}"
        );
    }
}

#[test]
fn dream_read_actual_inflight_snapshot_invalidation_refuses_old_page() {
    let row = w::Row::Cluster(w::Cluster {
        repo: w::Repo("private.invalid/public-api".into()),
        fingerprint: w::Hex("a".repeat(64)),
        count: 2,
        first_ms: w::Number(1),
        last_ms: w::Number(2),
    });
    let f = Fixture::open_with_rows(Some(vec![row]));
    let search = dream_read_client(
        &f,
        f.own.id,
        "dream_search",
        json!({"v":1,"kind":"observation","budget_tokens":4096}),
    );
    let handle = search["items"][0]["handle"].clone();
    f.read_cache.invalidate_on_presentation();
    let before = f.counts();
    let reply = dream_read_client(
        &f,
        f.own.id,
        "dream_read",
        json!({"v":1,"handle":handle,"view":"summary","budget_tokens":4096}),
    );
    assert_eq!(reply["error"]["code"], "stale_cursor");
    assert!(reply.get("item").is_none() && reply.get("snapshot").is_none());
    f.unchanged(before, "inflight disposable snapshot invalidation");
}

