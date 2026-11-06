// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
#![allow(clippy::disallowed_methods)] // Private source/SQLite fixtures.
use super::*;
use crate::{
    Ulid,
    messaging::wire::IssuerKind,
    registry::Role,
    store::{Events, events::Draft, sqlite::SqliteStore},
};
use ed25519_dalek::SigningKey;

struct Fixture {
    _temp: tempfile::TempDir,
    index: Index,
    actor: SeatActor,
    seat: SeatRecord,
    binding: RootBinding,
}
impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let index = Index::open(
            &temp.path().join(".bboxy"),
            SqliteStore::open(&temp.path().join("index")).unwrap(),
        )
        .unwrap();
        let seat = SeatRecord::new(Ulid(10), Ulid(11), "provenance queries", "zsh");
        let actor = SeatActor {
            seat_id: seat.id,
            process_generation: 1,
            swarm_id: seat.swarm,
            role: Role {
                name: "executor".into(),
                class: Class::Worker,
            },
            grant_epoch: 1,
            issuer_kind: IssuerKind::LocalDev,
        };
        let binding = RootBinding {
            root_id: "fixture-root".into(),
            swarm: seat.swarm,
            revision: 1,
        };
        Self {
            _temp: temp,
            index,
            actor,
            seat,
            binding,
        }
    }
    fn write(&mut self, n: u128, owner: Ulid, metadata: Value) -> String {
        let id = Ulid(n);
        let mut metadata = metadata;
        metadata["id"] = json!(id);
        if metadata["type"].is_null() {
            metadata["type"] = json!("note");
        }
        self.index
            .write(
                &format!("notes/{n:04}.md"),
                metadata,
                "signed fixture",
                owner,
                &SigningKey::from_bytes(&[91; 32]),
            )
            .unwrap();
        self.index.scan().unwrap();
        self.index.rebuild_graphs();
        id.to_string()
    }
    fn call(&self, tool: &str, args: Value) -> Result<Value> {
        read(
            &self.index,
            &self.actor,
            &self.seat,
            &self.binding,
            3,
            &Call {
                op_id: Ulid::new(),
                tool: tool.into(),
                args,
            },
        )
    }
}
#[test]
fn provenance_queries_page_owned_records_and_bind_cursor_to_scope_query_and_snapshot() {
    let mut f = Fixture::new();
    let own = f.actor.seat_id;
    let a = f.write(1, own, json!({"title":"own alpha"}));
    f.write(2, Ulid(55), json!({"title":"foreign secret title"}));
    let b = f.write(3, own, json!({"title":"own beta"}));
    let first = f
        .call("provenance_search", json!({"query":"own","limit":1}))
        .unwrap();
    assert_eq!(first["items"][0]["id"], a);
    let cursor = first["next"].as_str().unwrap();
    let decoded = String::from_utf8(URL_SAFE_NO_PAD.decode(cursor).unwrap()).unwrap();
    assert!(!decoded.contains("foreign"));
    let second = f
        .call(
            "provenance_search",
            json!({"query":"own","limit":1,"cursor":cursor}),
        )
        .unwrap();
    assert_eq!(second["items"][0]["id"], b);
    assert!(second["complete"].as_bool().unwrap());
    assert!(
        f.call(
            "provenance_search",
            json!({"query":"other","limit":1,"cursor":cursor})
        )
        .is_err()
    );
    for changed in ["root", "workspace", "grant", "provider"] {
        let mut actor = f.actor.clone();
        let mut binding = f.binding.clone();
        let mut revision = 3;
        match changed {
            "root" => binding.root_id = "other".into(),
            "workspace" => binding.revision += 1,
            "grant" => actor.grant_epoch += 1,
            _ => revision += 1,
        }
        assert!(
            read(
                &f.index,
                &actor,
                &f.seat,
                &binding,
                revision,
                &Call {
                    op_id: Ulid::new(),
                    tool: "provenance_search".into(),
                    args: json!({"query":"own","limit":1,"cursor":cursor})
                }
            )
            .is_err()
        );
    }
    f.write(4, own, json!({"title":"own gamma"}));
    assert!(
        f.call(
            "provenance_search",
            json!({"query":"own","limit":1,"cursor":cursor})
        )
        .is_err()
    );
    assert!(
        f.call("provenance_get", json!({"locator":Ulid(2)}))
            .is_err()
    );
}
#[test]
fn provenance_queries_lineage_is_directional_cycle_safe_bounded_and_role_filtered() {
    let mut f = Fixture::new();
    let own = f.actor.seat_id;
    let a = f.write(1, own, json!({"derived_from":[format!("[[{}]]",Ulid(2))]}));
    let b = f.write(2, own, json!({"derived_from":[format!("[[{}]]",Ulid(3))]}));
    let c = f.write(
        3,
        Ulid(55),
        json!({"derived_from":[format!("[[{}]]",Ulid(1))]}),
    );
    let worker = f
        .call("provenance_lineage", json!({"locator":a,"depth":8}))
        .unwrap();
    assert_eq!(worker["nodes"].as_array().unwrap().len(), 2);
    assert!(!worker.to_string().contains(&c));
    f.actor.role.class = Class::Oracle;
    let upstream = f
        .call(
            "provenance_lineage",
            json!({"locator":a,"depth":8,"limit":2}),
        )
        .unwrap();
    assert_eq!(upstream["nodes"].as_array().unwrap().len(), 2);
    assert_eq!(upstream["truncated"], true);
    assert!(
        upstream["edges"]
            .as_array()
            .unwrap()
            .iter()
            .all(|e| e["causation_verified"] == false)
    );
    let downstream = f
        .call(
            "provenance_lineage",
            json!({"locator":b,"direction":"downstream","depth":1}),
        )
        .unwrap();
    assert!(
        downstream["edges"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["input"] == b && e["output"] == a)
    );
    let cycle = f
        .call("provenance_lineage", json!({"locator":a,"depth":8}))
        .unwrap();
    assert_eq!(cycle["nodes"].as_array().unwrap().len(), 3);
    assert_eq!(cycle["truncated"], false);
}
#[test]
fn provenance_queries_history_pages_beyond_inline_history_and_pins_the_head() {
    let mut f = Fixture::new();
    let own = f.actor.seat_id;
    let id = f.write(
        1,
        own,
        json!({"history":(0..32).map(|n|json!({"id":format!("inline-{n}")})).collect::<Vec<_>>()}),
    );
    let key = SigningKey::from_bytes(&[92; 32]);
    for n in 0..70 {
        f.index
            .activity()
            .append(
                Draft::new(format!("doc/{id}/written"), "fixture", json!({"n":n})),
                &key,
            )
            .unwrap();
        f.index
            .activity()
            .append(
                Draft::new(
                    format!("doc/{}/written", Ulid(2)),
                    "fixture",
                    json!({"foreign":n}),
                ),
                &key,
            )
            .unwrap();
    }
    let before = f.index.activity().last_sequence().unwrap();
    let mut cursor = None;
    let mut count = 0;
    let mut pages = 0;
    loop {
        let page = f
            .call(
                "provenance_history",
                json!({"locator":id,"limit":16,"cursor":cursor}),
            )
            .unwrap();
        assert_eq!(page["event_head"], before.to_string());
        let items = page["items"].as_array().unwrap();
        assert!(items.len() <= 16);
        assert!(items.iter().all(|row| {
            row["subject"]
                .as_str()
                .unwrap()
                .starts_with(&format!("doc/{id}/"))
        }));
        assert!(
            items
                .iter()
                .all(|row| row["sequence"].as_str().unwrap().parse::<u64>().unwrap() <= before)
        );
        count += items.len();
        pages += 1;
        if pages == 1 {
            f.index
                .activity()
                .append(
                    Draft::new(format!("doc/{id}/written"), "later", json!({"later":true})),
                    &key,
                )
                .unwrap();
        }
        cursor = page["next"].as_str().map(str::to_owned);
        if cursor.is_none() {
            break;
        }
        assert!(pages < 10);
    }
    assert_eq!(count, 72); // Signed prepare/written plus 70 canonical rows.
    assert!(pages > 2);
    assert_eq!(
        f.index.activity().last_sequence().unwrap(),
        before + 1,
        "queries appended to the log"
    );
}
#[test]
fn provenance_queries_verify_separates_cached_signature_event_integrity_and_trust() {
    let mut f = Fixture::new();
    let own = f.actor.seat_id;
    let id = f.write(1, own, json!({"title":"evidence"}));
    let before = f.index.activity().last_sequence().unwrap();
    let verified = f.call("provenance_verify", json!({"locator":id})).unwrap();
    assert_eq!(verified["document_signature"]["status"], "valid");
    assert_eq!(verified["event"]["status"], "valid");
    assert_eq!(verified["event"]["recorded_state_matches"], true);
    assert_eq!(verified["limits"]["account_key_trust"], "not_checked");
    assert_eq!(verified["limits"]["external_artifact_bytes"], "not_checked");
    assert_eq!(verified["limits"]["historical_chain"], "not_audited");
    std::sync::Arc::make_mut(&mut f.index.documents).get_mut("notes/0001.md").unwrap().body = "tampered cached bytes".into();
    let changed = f.call("provenance_verify", json!({"locator":id})).unwrap();
    assert_eq!(changed["document_signature"]["status"], "invalid");
    assert_eq!(changed["event"]["recorded_state_matches"], false);
    assert_eq!(f.index.activity().last_sequence().unwrap(), before);
}
#[test]
fn provenance_queries_entity_page_rejects_tampered_columns_payload_and_predecessor() {
    for column in ["hash", "prev", "source", "signer", "sig", "payload"] {
        let mut f = Fixture::new();
        let own = f.actor.seat_id;
        let id = f.write(1, own, json!({}));
        let events = f.index.activity();
        let rows = events
            .entity_page(&id, 0, events.last_sequence().unwrap(), 64)
            .unwrap();
        assert_eq!(rows.events.len(), 2);
        let db = rusqlite::Connection::open(&events.path).unwrap();
        db.execute_batch("DROP TRIGGER immutable_update").unwrap();
        let sql = match column {
            "payload" => "UPDATE events SET payload=x'7b7d' WHERE seq=2",
            "signer" => "UPDATE events SET signer=zeroblob(32) WHERE seq=2",
            "sig" => "UPDATE events SET sig=zeroblob(64) WHERE seq=2",
            "hash" => "UPDATE events SET hash='forged' WHERE seq=2",
            "prev" => "UPDATE events SET prev='forged' WHERE seq=2",
            _ => "UPDATE events SET source='forged' WHERE seq=2",
        };
        db.execute_batch(sql).unwrap();
        assert!(events.entity_page(&id, 1, 2, 1).is_err(), "{column}");
        let verified = f
            .call("provenance_verify", json!({"locator":id,"sequence":2}))
            .unwrap();
        assert_eq!(verified["event"]["status"], "invalid", "{column}");
    }
    let mut f = Fixture::new();
    let own = f.actor.seat_id;
    let id = f.write(1, own, json!({}));
    let events = f.index.activity();
    let db = rusqlite::Connection::open(&events.path).unwrap();
    db.execute_batch("DROP TRIGGER immutable_delete; DELETE FROM events WHERE seq=1")
        .unwrap();
    assert!(events.entity_page(&id, 1, 2, 1).is_err());
}
#[test]
fn provenance_queries_schema_bounds_and_annotations_are_enforced() {
    let f = Fixture::new();
    for tool in crate::mcp::provenance::catalogue() {
        assert_eq!(tool["annotations"]["readOnlyHint"], true);
        assert_eq!(tool["annotations"]["openWorldHint"], false);
    }
    for args in [
        json!({"limit":0}),
        json!({"limit":65}),
        json!({"query":"x".repeat(1025)}),
        json!({"unexpected":true}),
    ] {
        assert!(f.call("provenance_search", args).is_err());
    }
}

#[test]
fn provenance_queries_revoked_task_assignment_blocks_old_replay_and_completion_witness() {
    let mut f = Fixture::new();
    let own = f.actor.seat_id;
    let id = f.write(1, own, json!({"type":"task", "assignee":format!("seat:{own}"), "status":"open", "title":"owned task", "verify":"true"}));
    f.seat.tasks.push(Ulid(1));
    let call = Call {
        op_id: Ulid::new(),
        tool: "provenance_get".into(),
        args: json!({"locator":id}),
    };
    let value = f.call(&call.tool, call.args.clone()).unwrap();
    let response = crate::mcp::success(call.op_id, value);
    let paths = authority_paths(&f.index, &call, &response).unwrap();
    assert!(retained(&f.index, &f.actor, &f.seat, &call, &response));
    assert!(current_authority(&f.index, &f.actor, &f.seat, &paths));
    std::sync::Arc::make_mut(&mut f.index.documents).get_mut("notes/0001.md").unwrap().metadata["assignee"] =
        json!(format!("seat:{}", Ulid(99)));
    assert!(!retained(&f.index, &f.actor, &f.seat, &call, &response));
    assert!(!current_authority(&f.index, &f.actor, &f.seat, &paths));
    assert!(f.call("provenance_get", json!({"locator":id})).is_err());
}
