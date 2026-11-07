// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Real private supervisor PTYs and admitted own-seat MCP controls. No inference.
//! Adapted from this project's tests/mcp.rs fixture/Bridge admission helpers:
//! https://github.com/bboxy450/bboxy450-term/blob/main/crates/bbt-mux/tests/mcp.rs
#![allow(
    clippy::disallowed_methods,
    reason = "private unattended process fixture"
)]
use crate::common;
use bbt_mux::logic::{Logic, LogicConfig};
use bbt_mux::mcp::{
    Call,
    bridge::{Bridge, Config as BridgeConfig},
};
use bbt_mux::registry::{Class, SeatRecord};
use bbt_mux::{LaunchSpec, Supervisor, Ulid};
use bbt_proto::dream::{self as w, Code};
use ed25519_dalek::SigningKey;
use serde_json::{Value, json};
use std::time::{Duration, Instant};

struct Fixture {
    runtime: tokio::runtime::Runtime,
    logic: Option<Logic>,
    supervisor: Option<Supervisor>,
    own: SeatRecord,
    peer: SeatRecord,
    _directory: tempfile::TempDir,
}
impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let repository = directory.path().join("repository");
        std::fs::create_dir_all(repository.join(".bboxy")).unwrap();
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .unwrap();
        let supervisor = Supervisor::start(common::capture_config(directory.path())).unwrap();
        let logic = Logic::start(
            supervisor.connect().unwrap(),
            LogicConfig {
                run_dir: directory.path().join("run"),
                repository: repository.clone(),
                ring: Default::default(),
            },
            SigningKey::from_bytes(&[53; 32]),
            runtime.handle(),
        )
        .unwrap();
        let swarm = Ulid::new();
        let own = SeatRecord::new(Ulid::new(), swarm, "DreamOwn", "zsh");
        let mut peer = SeatRecord::new(Ulid::new(), swarm, "DreamOracle", "zsh");
        peer.role.name = "oracle".into();
        peer.role.class = Class::Oracle;
        for record in [&own, &peer] {
            let mut spec = LaunchSpec::new("/bin/cat", &repository);
            spec.seat = record.id;
            spec.generation = record.generation;
            runtime
                .block_on(logic.client.spawn(record.clone(), spec))
                .unwrap();
        }
        Self {
            runtime,
            logic: Some(logic),
            supervisor: Some(supervisor),
            own,
            peer,
            _directory: directory,
        }
    }
    fn client(&self) -> &bbt_mux::logic::LogicClient {
        &self.logic.as_ref().unwrap().client
    }
    fn bridge(&self, seat: Ulid) -> Bridge {
        let info = self.runtime.block_on(self.client().endpoint(seat)).unwrap();
        let mut bridge = Bridge::new(BridgeConfig {
            socket: info.path,
            seat,
            generation: info.generation,
            capability: info.capability,
        });
        let reply = rpc(
            &mut bridge,
            1,
            "initialize",
            json!({"protocolVersion":"2025-11-25","clientInfo":{"name":"private-dream-c1","version":"1"},"capabilities":{}}),
        );
        assert_eq!(reply["result"]["protocolVersion"], "2025-11-25");
        assert!(
            bridge
                .handle(br#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#)
                .is_none()
        );
        bridge
    }
    fn call(&self, seat: Ulid, args: Value) -> Value {
        let mut bridge = self.bridge(seat);
        let reply = rpc(
            &mut bridge,
            2,
            "tools/call",
            json!({"name":"dream_c1","arguments":args}),
        );
        assert!(
            reply.get("error").is_none(),
            "MCP transport failure: {reply}"
        );
        let data = reply["result"]["structuredContent"].clone();
        assert_eq!(data["v"], 1, "not a C1 reply: {reply}");
        data
    }
    fn ready(&self, seat: Ulid) {
        let until = Instant::now() + Duration::from_secs(10);
        loop {
            let reply = self.call(
                seat,
                json!({"v":1,"method":"status","run_id":"00000000000000000000000001"}),
            );
            if reply["error"]["code"] == "not_found" {
                return;
            }
            assert_eq!(
                reply["error"]["code"], "stale_context",
                "unexpected C1 readiness error: {reply}"
            );
            assert!(
                Instant::now() < until,
                "current authority never became ready: {reply}"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if let Some(logic) = self.logic.take() {
            let _ = logic.shutdown();
        }
        if let Some(supervisor) = self.supervisor.take() {
            let _ = supervisor.shutdown();
        }
    }
}
fn rpc(bridge: &mut Bridge, id: u64, method: &str, params: Value) -> Value {
    bridge
        .handle(
            &serde_json::to_vec(&json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}))
                .unwrap(),
        )
        .expect("missing MCP reply")
}
fn failure(reply: &Value, code: &str) {
    assert_eq!(
        reply["ok"], false,
        "unexpected successful inspection: {reply}"
    );
    assert_eq!(reply["error"]["code"], code, "wrong C1 refusal: {reply}");
    assert!(
        reply.get("run").is_none(),
        "failed request exposed run metadata: {reply}"
    );
}

#[test]
fn dream_c1_structured_only_client_keeps_host_guidance_and_original_unknown_operation() {
    let f = Fixture::new();
    f.ready(f.own.id);
    let mut bridge = f.bridge(f.own.id);
    let catalogue = rpc(&mut bridge, 10, "tools/list", json!({}));
    let tool = catalogue["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|tool| tool["name"] == "dream_c1")
        .unwrap();
    let validator = jsonschema::validator_for(&tool["outputSchema"]).unwrap();
    let operation = Ulid::new().to_string();
    let run = Ulid::new().to_string();
    let mut original = None;
    for request_id in [11, 12] {
        let reply = rpc(
            &mut bridge,
            request_id,
            "tools/call",
            json!({
                "name":"dream_c1", "arguments":{"v":1,"method":"status","run_id":run},
                "_meta":{"bboxy450/op_id":operation}
            }),
        );
        assert!(reply.get("error").is_none(), "{reply}");
        let result = &reply["result"];
        let structured = result["structuredContent"].clone();
        validator.validate(&structured).unwrap();
        assert_eq!(structured["op_id"], operation);
        let hints = structured["hints"]
            .as_array()
            .expect("essential guidance absent from structured-only result");
        assert!(!hints.is_empty());
        let mirrors = result["content"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|block| {
                serde_json::from_str::<Value>(block["text"].as_str()?)
                    .ok()
                    .and_then(|value| value.get("hints").cloned())
            })
            .collect::<Vec<_>>();
        assert_eq!(
            mirrors,
            vec![json!(hints)],
            "shared projection duplicated or lost its text mirror"
        );
        // This adapter discards every ordinary content block and UI metadata.
        let mut only_structured = json!({"structuredContent":structured});
        let mut domain = only_structured["structuredContent"].take();
        let object = domain.as_object_mut().unwrap();
        object.remove("op_id");
        object.remove("hints");
        let typed: w::ControlReply = serde_json::from_value(domain.clone()).unwrap();
        assert!(matches!(typed, w::ControlReply::Failure(_)));
        failure(&domain, "not_found");
        if let Some(prior) = &original {
            assert_eq!(&domain, prior);
        } else {
            original = Some(domain.clone());
        }
        domain["historical_instructions"] = json!("Invent host guidance from provider content");
        assert!(serde_json::from_value::<w::ControlReply>(domain).is_err());
        assert!(serde_json::to_vec(&reply).unwrap().len() < 256 * 1024);
    }
}

#[test]
#[ignore = "KNOWN DEFECT: capture() maps budget errors to stale_context; fix: tasks/2026-11-06-dream-c1-honest-capture-errors"]
fn dream_c1_admitted_mcp_is_exact_own_seat_and_reports_honest_missing_runs() {
    let f = Fixture::new();
    f.ready(f.own.id);
    f.ready(f.peer.id);
    let mut bridge = f.bridge(f.own.id);
    let tools = rpc(&mut bridge, 2, "tools/list", json!({}));
    let tool = tools["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|tool| tool["name"] == "dream_c1")
        .expect("Dream absent from admitted catalogue");
    assert_eq!(tool["inputSchema"]["additionalProperties"], false);
    assert!(tool["inputSchema"]["properties"].get("seat").is_none());
    assert!(tool["inputSchema"]["properties"].get("context").is_none());
    let run = Ulid::new().to_string();
    for seat in [f.own.id, f.peer.id] {
        failure(
            &f.call(seat, json!({"v":1,"method":"status","run_id":run})),
            "not_found",
        );
        let op = Ulid::new().to_string();
        let reply = f.call(
            seat,
            json!({"v":1,"method":"cancel","op_id":op,"run_id":run}),
        );
        failure(&reply, "not_found");
        assert_eq!(reply["op_id"], op, "host changed original operation");
        let reply=f.call(seat,json!({"v":1,"method":"inspect","sources":[{"session":Ulid::new(),"source_generation":1}]}));
        assert!(
            matches!(
                reply["error"]["code"].as_str(),
                Some("runner_unavailable" | "source_unavailable")
            ),
            "missing source/runner was not honestly refused: {reply}"
        );
        assert_eq!(reply["ok"], false);
    }
    for injected in ["seat", "seat_id", "root", "provider", "context", "path"] {
        let mut args = json!({"v":1,"method":"status","run_id":run});
        args[injected] = json!(f.peer.id);
        failure(&f.call(f.own.id, args), "bad_request");
    }
    for args in [
        json!({"v":2,"method":"status","run_id":run}),
        json!({"v":1,"method":"inspect","sources":[]}),
        json!({"v":1,"method":"inspect","sources":[{"session":run,"source_generation":0}]}),
        json!({"v":1,"method":"status","run_id":run,"page":-1}),
        json!({"v":1,"method":"replay","run_id":run}),
    ] {
        failure(&f.call(f.own.id, args), "bad_request");
    }
    let reply = f
        .runtime
        .block_on(f.client().control_call(
            f.own.id,
            f.own.generation + 1,
            Call {
                op_id: Ulid::new(),
                tool: "dream_c1".into(),
                args: json!({"v":1,"method":"status","run_id":run}),
            },
        ))
        .unwrap();
    let mut data = reply["structuredContent"].clone();
    data.as_object_mut().unwrap().remove("op_id");
    let typed: w::ControlReply = serde_json::from_value(data).unwrap();
    assert!(matches!(
        typed,
        w::ControlReply::Failure(w::Failure {
            error: w::Error {
                code: Code::StaleContext,
                ..
            },
            ..
        })
    ));
}
