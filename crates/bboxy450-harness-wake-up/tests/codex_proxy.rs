// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
#![allow(clippy::disallowed_methods)] // Private sockets, no operator daemon or UI.

use bboxy450_harness_wake_up::{launch::CodexLaunch, native};
use futures_util::{FutureExt, SinkExt, StreamExt};
use serde_json::json;
use std::{
    panic::AssertUnwindSafe,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio_tungstenite::tungstenite::{Message, protocol::WebSocketConfig};

// Private fixture observations contain static stage labels and elapsed times,
// never frames, panic payloads, capabilities or environment values.
#[derive(Clone)]
struct ProxyProbe {
    started: Instant,
    data: Arc<Mutex<ProxyProbeData>>,
}
#[derive(Debug, Default)]
struct ProxyProbeData {
    stages: Vec<(&'static str, u128)>,
    dropped: usize,
    tasks: Vec<(&'static str, &'static str)>,
}
impl ProxyProbe {
    fn new() -> Self {
        Self { started: Instant::now(), data: Default::default() }
    }
    fn stage(&self, stage: &'static str) {
        let mut data = self.data.lock().unwrap();
        if data.stages.len() < 64 {
            data.stages.push((stage, self.started.elapsed().as_millis()));
        } else {
            data.dropped += 1;
        }
    }
    fn task(&self, name: &'static str, outcome: &'static str) {
        let mut data = self.data.lock().unwrap();
        if let Some(task) = data.tasks.iter_mut().find(|task| task.0 == name) {
            task.1 = outcome;
        } else {
            assert!(data.tasks.len() < 2, "private fixture child bound");
            data.tasks.push((name, outcome));
        }
    }
    fn emit(&self) {
        use std::io::Write;
        // Write directly so successful focused runs retain observations too.
        let _ = writeln!(std::io::stderr().lock(), "proxy fixture elapsed_ms={} {:?}",
                         self.started.elapsed().as_millis(), *self.data.lock().unwrap());
    }
}
struct ProxyReport(ProxyProbe);
impl Drop for ProxyReport {
    fn drop(&mut self) { self.0.emit(); }
}
struct ProxyChild {
    probe: ProxyProbe,
    name: &'static str,
    finished: bool,
}
impl Drop for ProxyChild {
    fn drop(&mut self) {
        if !self.finished {
            self.probe.task(self.name, "cancelled");
            self.probe.emit();
        }
    }
}
fn proxy_child(
    tasks: &mut tokio::task::JoinSet<(&'static str, bool)>,
    probe: &ProxyProbe,
    name: &'static str,
    future: impl std::future::Future<Output = ()> + Send + 'static,
) {
    probe.task(name, "spawned");
    let child = ProxyChild { probe: probe.clone(), name, finished: false };
    tasks.spawn(async move {
        let mut child = child;
        let success = AssertUnwindSafe(future).catch_unwind().await.is_ok();
        child.probe.task(name, if success { "completed" } else { "panicked" });
        child.finished = true;
        (name, success)
    });
}
async fn stop_proxy_children(
    tasks: &mut tokio::task::JoinSet<(&'static str, bool)>,
    probe: &ProxyProbe,
) -> bool {
    let mut no_panics = true;
    tasks.abort_all();
    while let Some(result) = tasks.join_next().await {
        match result {
            Ok((_, success)) => no_panics &= success,
            Err(error) => {
                no_panics &= error.is_cancelled();
                probe.stage(if error.is_cancelled() { "child cancellation joined" } else { "child panic joined" });
            }
        }
    }
    probe.stage("all children joined");
    no_panics
}

// keeps: interface app-harness — a composed launcher can deliver natively beside its live TUI
#[tokio::test]
async fn a_native_client_reaches_the_backend_while_the_tui_stays_connected() {
    tokio::time::timeout(Duration::from_secs(10), async {
        let root = tempfile::tempdir().unwrap();
        let frontend = root.path().join("front");
        let backend = root.path().join("back");
        let listener = tokio::net::UnixListener::bind(&backend).unwrap();
        async fn accept(listener: &tokio::net::UnixListener) -> tokio_tungstenite::WebSocketStream<tokio::net::UnixStream> {
            loop {
                let (stream, _) = listener.accept().await.unwrap();
                if let Ok(ws) = tokio_tungstenite::accept_async(stream).await { return ws; }
            }
        }
        let server = tokio::spawn(async move {
            let mut tui = accept(&listener).await;
            let start: serde_json::Value = serde_json::from_str(tui.next().await.unwrap().unwrap().to_text().unwrap()).unwrap();
            assert_eq!(start["method"], "thread/start");
            tui.send(Message::Text(json!({"id":1,"result":{"thread":{"id":"private-thread"}}}).to_string().into())).await.unwrap();
            let title: serde_json::Value = serde_json::from_str(tui.next().await.unwrap().unwrap().to_text().unwrap()).unwrap();
            assert_eq!(title["method"], "thread/name/set");
            tui.send(Message::Text(json!({"id":title["id"],"result":{}}).to_string().into())).await.unwrap();
            let mut native = accept(&listener).await;
            let initialize = native.next().await.unwrap().unwrap();
            native.send(initialize).await.unwrap();
            let turn = native.next().await.unwrap().unwrap();
            assert_eq!(serde_json::from_str::<serde_json::Value>(turn.to_text().unwrap()).unwrap()["method"], "turn/start");
            native.send(turn).await.unwrap();
            let mut refused = accept(&listener).await;
            assert!(!matches!(refused.next().await, Some(Ok(frame)) if !frame.is_close()), "wrong resume reached the backend");
            let still_connected = tui.next().await.unwrap().unwrap();
            tui.send(still_connected).await.unwrap();
        });
        let launch = CodexLaunch {
            name: "private fixture".into(), cwd: root.path().into(),
            config: Default::default(), resume: None, host: "/usr/bin/true".into(),
        };
        let proxy = tokio::spawn(async move {
            native::codex_proxy(&frontend, &backend, &launch, "/must-not-start-a-daemon").await.unwrap();
        });
        let path = root.path().join("front");
        while !path.exists() { tokio::task::yield_now().await; }
        let (mut tui, _) = tokio_tungstenite::client_async("ws://localhost/", tokio::net::UnixStream::connect(&path).await.unwrap()).await.unwrap();
        let start = json!({"id":1,"method":"thread/start","params":{}}).to_string();
        tui.send(Message::Text(start.into())).await.unwrap();
        let started: serde_json::Value = serde_json::from_str(tui.next().await.unwrap().unwrap().to_text().unwrap()).unwrap();
        assert_eq!(started["result"]["thread"]["id"], "private-thread");
        let (mut control, _) = tokio_tungstenite::client_async("ws://localhost/", tokio::net::UnixStream::connect(path).await.unwrap()).await.unwrap();
        let initialize = json!({"id":2,"method":"initialize","params":{}}).to_string();
        control.send(Message::Text(initialize.clone().into())).await.unwrap();
        assert_eq!(control.next().await.unwrap().unwrap().to_text().unwrap(), initialize);
        let turn = json!({"id":3,"method":"turn/start","params":{"threadId":"private-thread","input":[]}}).to_string();
        control.send(Message::Text(turn.clone().into())).await.unwrap();
        assert_eq!(control.next().await.unwrap().unwrap().to_text().unwrap(), turn);
        let (mut refused, _) = tokio_tungstenite::client_async("ws://localhost/", tokio::net::UnixStream::connect(root.path().join("front")).await.unwrap()).await.unwrap();
        refused.send(Message::Text(json!({"id":4,"method":"thread/resume","params":{"threadId":"another-thread"}}).to_string().into())).await.unwrap();
        assert!(!matches!(refused.next().await, Some(Ok(frame)) if !frame.is_close()), "wrong native resume was accepted");
        tui.send(Message::Text("still connected".into())).await.unwrap();
        assert_eq!(tui.next().await.unwrap().unwrap().to_text().unwrap(), "still connected");
        server.await.unwrap();
        proxy.abort();
        let _ = proxy.await;
    }).await.expect("native client was blocked behind the live TUI");
}

// A real installed catalog was 22,787,296 bytes, above Tungstenite's default
// 16 MiB frame limit. Both directions must preserve large frames and keep the
// connection alive for subsequent requests, rather than make the TUI reconnect.
#[tokio::test]
async fn large_catalog_frames_survive_the_proxy_without_reconnecting() {
    let probe = ProxyProbe::new();
    let _report = ProxyReport(probe.clone());
    let mut tasks = tokio::task::JoinSet::new();
    let mut directory = None;
    let result = AssertUnwindSafe(tokio::time::timeout(Duration::from_secs(20), async {
        probe.stage("fixture construction");
        directory = Some(tempfile::tempdir().unwrap());
        let root = directory.as_ref().unwrap();
        let frontend = root.path().join("front");
        let backend = root.path().join("back");
        let listener = tokio::net::UnixListener::bind(&backend).unwrap();
        let config = || {
            WebSocketConfig::default()
                .max_message_size(Some(128 << 20))
                .max_frame_size(Some(128 << 20))
        };
        probe.stage("payload construction");
        let catalog = json!({"id":42,"result":{"catalog":"x".repeat(23 << 20)}}).to_string();
        let expected = catalog.clone();
        probe.stage("payload constructed");
        let server_probe = probe.clone();
        proxy_child(&mut tasks, &probe, "server", async move {
            // The proxy's readiness probe opens a plain connection first.
            let mut ws = loop {
                server_probe.stage("server accept");
                let (stream, _) = listener.accept().await.unwrap();
                server_probe.stage("server handshake");
                if let Ok(ws) =
                    tokio_tungstenite::accept_async_with_config(stream, Some(config())).await
                {
                    break ws;
                }
                server_probe.stage("server plain readiness probe closed");
            };
            server_probe.stage("server read catalog");
            let received = ws.next().await.unwrap().unwrap().into_text().unwrap();
            assert!(received.as_bytes() == expected.as_bytes(), "server catalog bytes differ");
            drop(received);
            server_probe.stage("server echo catalog");
            ws.send(Message::Text(expected.into())).await.unwrap();
            server_probe.stage("server read final exchange");
            assert_eq!(
                ws.next().await.unwrap().unwrap().into_text().unwrap(),
                "still connected"
            );
            server_probe.stage("server send confirmation");
            ws.send(Message::Text("confirmed".into())).await.unwrap();
            server_probe.stage("server complete");
        });
        let launch = CodexLaunch {
            name: "private fixture".into(),
            cwd: root.path().into(),
            config: Default::default(),
            resume: None,
            host: "/unused".into(),
        };
        let proxy_probe = probe.clone();
        proxy_child(&mut tasks, &probe, "proxy", async move {
            proxy_probe.stage("proxy serving");
            native::codex_proxy(&frontend, &backend, &launch, "/must-not-start-a-daemon")
                .await
                .unwrap();
        });
        let path = root.path().join("front");
        probe.stage("client wait for frontend");
        while !path.exists() {
            tokio::task::yield_now().await;
        }
        probe.stage("client connect");
        let stream = tokio::net::UnixStream::connect(path).await.unwrap();
        probe.stage("client handshake");
        let (mut client, _) =
            tokio_tungstenite::client_async_with_config("ws://localhost/", stream, Some(config()))
                .await
                .unwrap();
        probe.stage("client send catalog");
        client
            .send(Message::Text(catalog.clone().into()))
            .await
            .unwrap();
        probe.stage("client read catalog echo");
        let received = client.next().await.unwrap().unwrap().into_text().unwrap();
        assert!(received.as_bytes() == catalog.as_bytes(), "client catalog bytes differ");
        drop(received);
        probe.stage("client send final exchange");
        client
            .send(Message::Text("still connected".into()))
            .await
            .unwrap();
        probe.stage("client read confirmation");
        assert_eq!(
            client.next().await.unwrap().unwrap().into_text().unwrap(),
            "confirmed"
        );
        probe.stage("client exchange complete");
        loop {
            let (name, success) = tasks.join_next().await.unwrap().unwrap();
            assert!(success, "private {name} task panicked");
            if name == "server" { break; }
        }
        assert!(stop_proxy_children(&mut tasks, &probe).await, "private proxy task panicked");
        drop(directory.take());
    })).catch_unwind().await;
    probe.stage(match &result {
        Ok(Ok(())) => "relay complete",
        Ok(Err(_)) => "outer deadline elapsed",
        Err(_) => "driver panicked",
    });
    if !tasks.is_empty() {
        stop_proxy_children(&mut tasks, &probe).await;
    }
    match result {
        Ok(Ok(())) => {},
        Ok(Err(error)) => panic!("large catalog relay stalled: {error}"),
        Err(error) => std::panic::resume_unwind(error),
    }
}

// keeps: task 278 — cancellation and panic observations are executable controls.
#[tokio::test]
async fn proxy_diagnostics_retain_child_panic_and_join_cancelled_tasks() {
    let probe = ProxyProbe::new();
    let mut tasks = tokio::task::JoinSet::new();
    proxy_child(&mut tasks, &probe, "server", async { panic!("private diagnostic control"); });
    assert_eq!(tasks.join_next().await.unwrap().unwrap(), ("server", false));
    proxy_child(&mut tasks, &probe, "proxy", std::future::pending());
    probe.stage("control waiting for native read");
    assert!(tokio::time::timeout(Duration::ZERO, std::future::pending::<()>()).await.is_err());
    assert!(stop_proxy_children(&mut tasks, &probe).await);
    let data = probe.data.lock().unwrap();
    assert_eq!(data.tasks, [("server", "panicked"), ("proxy", "cancelled")]);
    assert!(data.stages.iter().any(|(stage, _)| *stage == "control waiting for native read"));
    assert!(tasks.is_empty());
}

// keeps: interface app-harness — compose shim hooks; seat policy applies only to the first launch
#[tokio::test]
async fn shared_start_and_resume_compose_shim_hooks_and_keep_seat_policy() {
    tokio::time::timeout(Duration::from_secs(10), async {
        for method in ["thread/start", "thread/resume"] {
            let root = tempfile::tempdir().unwrap();
            let frontend = root.path().join("front");
            let backend = root.path().join("back");
            let listener = tokio::net::UnixListener::bind(&backend).unwrap();
            let shim_hook = json!({"hooks":[{"type":"command","command":"/private/shim-hook"}]});
            let seat_hook = json!({"hooks":[{"type":"command","command":"/private/seat-hook"}]});
            let common_hook = json!({"hooks":[{"type":"command","command":"/private/common-hook"}]});
            let expected_hooks = json!([shim_hook, common_hook, seat_hook]);
            let unchanged = json!({"id":2,"method":"thread/resume","params":{"threadId":"unrelated-thread","approvalPolicy":"on-request","sandbox":"read-only","config":{"approval_policy":"on-request"}}});
            let expected_unchanged = unchanged.clone();
            let server = tokio::spawn(async move {
                let mut ws = loop {
                    let (stream, _) = listener.accept().await.unwrap();
                    if let Ok(ws) = tokio_tungstenite::accept_async(stream).await { break ws; }
                };
                let call: serde_json::Value = serde_json::from_str(&ws.next().await.unwrap().unwrap().into_text().unwrap()).unwrap();
                assert_eq!(call["method"], method);
                assert_eq!(call["params"]["approvalPolicy"], "never");
                assert_eq!(call["params"]["sandbox"], "danger-full-access");
                assert_eq!(call["params"]["config"]["approval_policy"], "never");
                assert_eq!(call["params"]["config"]["sandbox_mode"], "danger-full-access");
                assert_eq!(call["params"]["config"]["hooks"]["SessionStart"], expected_hooks);
                assert_eq!(call["params"]["config"]["hooks"]["Stop"], json!([{"hooks":[{"type":"command","command":"/private/shim-stop"}]}]));
                assert_eq!(call["params"]["config"]["hooks"]["PreToolUse"], json!([{"hooks":[{"type":"command","command":"/private/seat-tool"}]}]));
                assert_eq!(call["params"]["config"]["mcp_servers"]["shim"]["command"], "/private/shim-mcp");
                assert_eq!(call["params"]["config"]["mcp_servers"]["bboxy"]["command"], "/private/seat-mcp");
                assert_eq!(call["params"]["config"]["shell_environment_policy"]["set"]["CALLER_SESSION_ID"], "private-session");
                assert_eq!(call["params"]["config"]["notify"], json!(["/private/seat-notify"]), "ordinary arrays still use the seat overlay");
                ws.send(Message::Text(json!({"id":1,"result":{"thread":{"id":"fixture-thread"}}}).to_string().into())).await.unwrap();
                let title: serde_json::Value = serde_json::from_str(&ws.next().await.unwrap().unwrap().into_text().unwrap()).unwrap();
                assert_eq!(title["method"], "thread/name/set");
                ws.send(Message::Text(json!({"id":title["id"],"result":{}}).to_string().into())).await.unwrap();
                let other: serde_json::Value = serde_json::from_str(&ws.next().await.unwrap().unwrap().into_text().unwrap()).unwrap();
                assert_eq!(other, expected_unchanged);
                ws.send(Message::Text(json!({"id":2,"result":{}}).to_string().into())).await.unwrap();
            });
            let launch = CodexLaunch {
                name: "private fixture".into(), cwd: root.path().into(),
                config: [
                    ("approval_policy".into(), json!("never")),
                    ("sandbox_mode".into(), json!("danger-full-access")),
                    ("hooks".into(), json!({"SessionStart":[seat_hook,common_hook],"PreToolUse":[{"hooks":[{"type":"command","command":"/private/seat-tool"}]}]})),
                    ("mcp_servers".into(), json!({"bboxy":{"command":"/private/seat-mcp"}})),
                    ("notify".into(), json!(["/private/seat-notify"])),
                ].into(),
                resume: (method == "thread/resume").then(|| "fixture-thread".into()), host: "/usr/bin/true".into(),
            };
            let proxy = tokio::spawn(async move {
                native::codex_proxy(&frontend, &backend, &launch, "/must-not-start-a-daemon").await.unwrap();
            });
            let path = root.path().join("front");
            while !path.exists() { tokio::task::yield_now().await; }
            let (mut client, _) = tokio_tungstenite::client_async("ws://localhost/", tokio::net::UnixStream::connect(path).await.unwrap()).await.unwrap();
            let mut params = json!({"approvalPolicy":"on-request","sandbox":"read-only","config":{"approval_policy":"on-request","sandbox_mode":"read-only"}});
            params["config"]["hooks"] = json!({"SessionStart":[shim_hook,common_hook],"Stop":[{"hooks":[{"type":"command","command":"/private/shim-stop"}]}]});
            params["config"]["mcp_servers"] = json!({"shim":{"command":"/private/shim-mcp"}});
            params["config"]["shell_environment_policy"] = json!({"set":{"CALLER_SESSION_ID":"private-session"}});
            params["config"]["notify"] = json!(["/private/shim-notify"]);
            if method == "thread/resume" { params["threadId"] = json!("fixture-thread"); }
            client.send(Message::Text(json!({"id":1,"method":method,"params":params}).to_string().into())).await.unwrap();
            let reply: serde_json::Value = serde_json::from_str(&client.next().await.unwrap().unwrap().into_text().unwrap()).unwrap();
            assert_eq!(reply["id"], 1);
            client.send(Message::Text(unchanged.to_string().into())).await.unwrap();
            let reply: serde_json::Value = serde_json::from_str(&client.next().await.unwrap().unwrap().into_text().unwrap()).unwrap();
            assert_eq!(reply["id"], 2);
            server.await.unwrap();
            proxy.abort();
            let _ = proxy.await;
        }
    }).await.expect("private policy proxy stalled");
}
