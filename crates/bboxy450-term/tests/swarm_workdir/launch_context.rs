// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Real launch children and admitted root MCP; only loopback fixture inference.
use super::rich_detach::{row, wait};
use super::*;
use serde_json::json;
use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::{Arc, Mutex},
};

fn provider(
    task_id: Arc<Mutex<Option<String>>>,
) -> (u16, Arc<Mutex<Vec<Value>>>, std::thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    listener.set_nonblocking(true).unwrap();
    let requests = Arc::new(Mutex::new(Vec::new()));
    let captured = requests.clone();
    let task = std::thread::spawn(move || {
        let until = Instant::now() + Duration::from_secs(50);
        while Instant::now() < until && captured.lock().unwrap().len() < 3 {
            let Ok((mut stream, _)) = listener.accept() else {
                std::thread::sleep(Duration::from_millis(10));
                continue;
            };
            // Darwin accepted sockets can inherit the listener's nonblocking
            // mode. Keep the existing bounded blocking HTTP-read contract.
            stream.set_nonblocking(false).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let mut data = Vec::new();
            let mut byte = [0];
            while !data.ends_with(b"\r\n\r\n") {
                stream.read_exact(&mut byte).unwrap();
                data.push(byte[0]);
                assert!(data.len() < 16384);
            }
            let headers = String::from_utf8(data).unwrap();
            let size: usize = headers
                .lines()
                .find_map(|line| {
                    line.to_ascii_lowercase()
                        .strip_prefix("content-length:")
                        .map(|s| s.trim().parse().unwrap())
                })
                .unwrap();
            assert!(size < 2 * 1024 * 1024);
            let mut body = vec![0; size];
            stream.read_exact(&mut body).unwrap();
            let mut requests = captured.lock().unwrap();
            requests.push(serde_json::from_slice(&body).unwrap());
            let (delta, finish) = match requests.len() {
                1 => (
                    json!({"tool_calls":[{"index":0,"id":"real-admitted-orient","type":"function","function":{"name":"mcp__bboxy__orient","arguments":"{}"}}]}),
                    "tool_calls",
                ),
                2 => (
                    json!({"tool_calls":[{"index":0,"id":"real-admitted-task","type":"function","function":{"name":"mcp__bboxy__task_status","arguments":json!({"task_id":task_id.lock().unwrap().as_ref().unwrap()}).to_string()}}]}),
                    "tool_calls",
                ),
                _ => (json!({"content":"PRIVATE CONTEXT COMPLETE"}), "stop"),
            };
            let response = format!(
                "data: {}\n\ndata: {}\n\ndata: [DONE]\n\n",
                json!({"choices":[{"index":0,"delta":delta,"finish_reason":null}]}),
                json!({"choices":[{"index":0,"delta":{},"finish_reason":finish}]})
            );
            write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",response.len(),response).unwrap();
        }
    });
    (port, requests, task)
}
fn assigned(f: &Fixture, seat: &str) -> Value {
    let title = format!("Private launch context {seat}");
    let task = f.ctl(&[
        "task",
        "create",
        &title,
        "--verify",
        "true",
        "--brief",
        "PRIVATE_TASK_CONTEXT_20261103: inspect the assigned repository; no paid inference.",
    ]);
    let locator = format!(
        "#{}",
        task["record"]["number"]
            .as_u64()
            .unwrap_or_else(|| panic!("task locator: {task}"))
    );
    f.ctl(&["task", "delegate", &locator, seat]);
    task["record"].clone()
}
#[test]
#[ignore = "private native App, vendor compatibility children and real admitted MCP; make test-rich-launch-context"]
fn launch_children_receive_workflow_and_call_their_own_admitted_mcp() {
    let mut f = Fixture::with_directory(
        tempfile::Builder::new()
            .prefix("rich-context-")
            .tempdir_in("/tmp")
            .unwrap(),
    );
    fs::copy(BIN, f.root.join("bin/bboxy450-term")).unwrap();
    fs::copy(
        std::env::current_exe().unwrap(),
        f.root.join("fixture-test"),
    )
    .unwrap();
    for kind in ["claude", "codex"] {
        let path = f.root.join(format!("bin/{kind}"));
        fs::write(&path, include_str!("agent_context.py")).unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
    }
    let task_id = Arc::new(Mutex::new(None));
    let (port, requests, server) = provider(task_id.clone());
    let profiles = f.root.join("home/.bboxy450-harness/profiles");
    fs::create_dir_all(&profiles).unwrap();
    fs::write(profiles.join("private-context.md"),format!("---\nname: private-context\nmodel: private-context-model\nendpoint: http://127.0.0.1:{port}/v1\napi_key_env: PRIVATE_FIXTURE_MODEL_KEY\non_unattended: deny\nauto_compact: false\n---\n")).unwrap();
    let binary = PathBuf::from(std::env::var_os("BBOXY_HARNESS_TEST_BIN").unwrap());
    let shim = f.root.join("bin/bboxy450-harness");
    let approval_receipt = f.root.join("managed-approval-parity.json");
    fs::write(&shim,format!("#!/usr/bin/python3\nimport json,os,sys\nfrom pathlib import Path\nargs=sys.argv[1:]\nPath({}).write_text(json.dumps({{\"yolo\": \"--yolo\" in args}}))\nos.environ[\"PRIVATE_FIXTURE_MODEL_KEY\"]=\"private-local-fixture-only\"\nos.execv({},[{},*args,\"--profile\",\"private-context\"])\n",serde_json::to_string(&approval_receipt).unwrap(),serde_json::to_string(&binary).unwrap(),serde_json::to_string(&binary).unwrap())).unwrap();
    fs::set_permissions(shim, fs::Permissions::from_mode(0o700)).unwrap();
    f.start();
    let swarm = f.ctl(&["ui", "layout"])["swarm"]
        .as_str()
        .unwrap()
        .to_owned();
    for (kind, role) in [("codex", "executor"), ("claude", "oracle")] {
        let cwd = f.root.join("project with spaces");
        let seat = f.ctl(&[
            "seat",
            "spawn",
            kind,
            "--place",
            "tab",
            "--swarm",
            &swarm,
            "--name",
            "literal \"quoted\" 'name'",
            "--cwd",
            cwd.to_str().unwrap(),
        ])["seat"]
            .as_str()
            .unwrap()
            .to_owned();
        let task = assigned(&f, &seat);
        let path = f.root.join(format!("context-{seat}.json"));
        fs::write(
            f.root.join(format!("context-{seat}.refresh")),
            task["id"].as_str().unwrap(),
        )
        .unwrap();
        let context: Value = wait(|| {
            fs::read(&path)
                .ok()
                .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
                .filter(|c| {
                    c["task"]
                        .to_string()
                        .contains("PRIVATE_TASK_CONTEXT_20261103")
                })
        });
        let orient = &context["orient"]["structuredContent"];
        assert_eq!(orient["actor"]["seat_id"], seat);
        assert_eq!(orient["actor"]["role"]["name"], role);
        assert_eq!(
            orient["actor"]["role"]["class"],
            if role == "executor" { "worker" } else { role }
        );
        assert_eq!(context["cwd"], cwd.to_str().unwrap());
        assert!(
            context["prompt"]
                .as_str()
                .unwrap()
                .contains(&serde_json::to_string(&row(&f, &seat)["record"]["name"]).unwrap())
        );
        for marker in ["orient", "task_report"] {
            assert!(context["skill"].as_str().unwrap().contains(marker));
        }
        assert!(
            context["tools"]["tools"]
                .as_array()
                .unwrap()
                .iter()
                .any(|t| t["name"] == "self_status")
        );
        assert_eq!(
            context["generation"],
            row(&f, &seat)["record"]["generation"].to_string()
        );
        assert_eq!(
            orient["actor"]["process_generation"],
            row(&f, &seat)["record"]["generation"]
        );
        wait(|| {
            (row(&f, &seat)["record"]["harness_session_id"] == context["native_session"])
                .then_some(())
        });
        f.ctl(&["seat", "restart", &seat]);
        wait(|| {
            fs::read(&path)
                .ok()
                .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
                .filter(|c| c["generation"] == "2")
        });
        fs::write(
            f.root.join(format!("context-{seat}.refresh")),
            task["id"].as_str().unwrap(),
        )
        .unwrap();
        let restarted: Value = wait(|| {
            fs::read(&path)
                .ok()
                .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
                .filter(|c| {
                    c["generation"] == "2"
                        && c["task"]
                            .to_string()
                            .contains("PRIVATE_TASK_CONTEXT_20261103")
                })
        });
        assert_eq!(
            restarted["orient"]["structuredContent"]["actor"]["process_generation"],
            2
        );
        assert_eq!(
            restarted["orient"]["structuredContent"]["actor"]["seat_id"],
            seat
        );
        assert_ne!(restarted["pid"], context["pid"]);
        assert_eq!(restarted["cwd"], context["cwd"]);
        assert_eq!(restarted["prompt"], context["prompt"]);
        assert_eq!(restarted["native_session"], context["native_session"]);
        println!(
            "launch-context: {kind} seat={seat} role={role} task={} MCP=real-own-seat generations=1,2 quoted-name/spaced-cwd preserved",
            task["id"]
        );
    }
    let cwd = f.root.join("project with spaces");
    let seat = f.ctl(&[
        "seat",
        "spawn",
        "bboxy450-harness",
        "--place",
        "tab",
        "--swarm",
        &swarm,
        "--name",
        "native \"quoted\" 'name'",
        "--cwd",
        cwd.to_str().unwrap(),
    ])["seat"]
        .as_str()
        .unwrap()
        .to_owned();
    let approval: Value = wait(|| {
        fs::read(&approval_receipt)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
    });
    assert_eq!(
        approval["yolo"], true,
        "managed approval parity: harness --yolo missing from production launch"
    );
    let task = assigned(&f, &seat);
    *task_id.lock().unwrap() = Some(task["id"].as_str().unwrap().to_owned());
    super::rich_detach::rich(&f, &seat, 1);
    f.ctl(&[
        "seat",
        "type",
        &seat,
        "Validate my assigned private launch context.\n",
    ]);
    wait(|| (requests.lock().unwrap().len() == 3).then_some(()));
    server.join().unwrap();
    let requests = requests.lock().unwrap();
    let first = &requests[0];
    assert_eq!(first["model"], "private-context-model");
    let system = first["messages"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|m| m["role"] == "system")
        .map(|m| m["content"].as_str().unwrap())
        .collect::<Vec<_>>()
        .join("\n");
    let name = serde_json::to_string(&row(&f, &seat)["record"]["name"]).unwrap();
    for marker in [
        "ROLE: EXECUTOR",
        "# bboxy450 seat workflow",
        "orient",
        "task_report",
        cwd.to_str().unwrap(),
        name.as_str(),
    ] {
        assert!(
            system.contains(marker),
            "native system context lost {marker}"
        );
    }
    assert!(
        first["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|t| t["function"]["name"] == "mcp__bboxy__orient")
    );
    let tool = requests[1]["messages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["role"] == "tool")
        .expect("actual downstream MCP result")["content"]
        .as_str()
        .unwrap();
    assert!(
        tool.contains(&seat),
        "native MCP did not authenticate its own seat"
    );
    assert!(tool.contains("executor"));
    assert!(
        requests[2]["messages"]
            .to_string()
            .contains("PRIVATE_TASK_CONTEXT_20261103"),
        "native MCP lost assigned task body"
    );
    println!(
        "launch-context: native seat={seat} task={} MCP=real-own-seat requests={} model=private-context-model; no vendor/provider traffic",
        task["id"],
        requests.len()
    );
}
