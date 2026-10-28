// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
#![allow(clippy::disallowed_methods)] // Private sockets, no operator daemon or UI.

use bboxy450_harness_wake_up::{launch::CodexLaunch, native};
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use std::time::Duration;
use tokio_tungstenite::tungstenite::{Message, protocol::WebSocketConfig};

// A real installed catalog was 22,787,296 bytes, above Tungstenite's default
// 16 MiB frame limit. Both directions must preserve large frames and keep the
// connection alive for subsequent requests, rather than make the TUI reconnect.
#[tokio::test]
async fn large_catalog_frames_survive_the_proxy_without_reconnecting() {
    tokio::time::timeout(Duration::from_secs(20), async {
        let root = tempfile::tempdir().unwrap();
        let frontend = root.path().join("front");
        let backend = root.path().join("back");
        let listener = tokio::net::UnixListener::bind(&backend).unwrap();
        let config = || {
            WebSocketConfig::default()
                .max_message_size(Some(128 << 20))
                .max_frame_size(Some(128 << 20))
        };
        let catalog = json!({"id":42,"result":{"catalog":"x".repeat(23 << 20)}}).to_string();
        let expected = catalog.clone();
        let server = tokio::spawn(async move {
            // The proxy's readiness probe opens a plain connection first.
            let mut ws = loop {
                let (stream, _) = listener.accept().await.unwrap();
                if let Ok(ws) =
                    tokio_tungstenite::accept_async_with_config(stream, Some(config())).await
                {
                    break ws;
                }
            };
            assert_eq!(
                ws.next().await.unwrap().unwrap().into_text().unwrap(),
                expected
            );
            ws.send(Message::Text(expected.into())).await.unwrap();
            assert_eq!(
                ws.next().await.unwrap().unwrap().into_text().unwrap(),
                "still connected"
            );
            ws.send(Message::Text("confirmed".into())).await.unwrap();
        });
        let launch = CodexLaunch {
            name: "private fixture".into(),
            cwd: root.path().into(),
            config: Default::default(),
            resume: None,
            host: "/unused".into(),
        };
        let proxy = tokio::spawn(async move {
            native::codex_proxy(&frontend, &backend, &launch, "/must-not-start-a-daemon")
                .await
                .unwrap();
        });
        let path = root.path().join("front");
        while !path.exists() {
            tokio::task::yield_now().await;
        }
        let stream = tokio::net::UnixStream::connect(path).await.unwrap();
        let (mut client, _) =
            tokio_tungstenite::client_async_with_config("ws://localhost/", stream, Some(config()))
                .await
                .unwrap();
        client
            .send(Message::Text(catalog.clone().into()))
            .await
            .unwrap();
        assert_eq!(
            client.next().await.unwrap().unwrap().into_text().unwrap(),
            catalog
        );
        client
            .send(Message::Text("still connected".into()))
            .await
            .unwrap();
        assert_eq!(
            client.next().await.unwrap().unwrap().into_text().unwrap(),
            "confirmed"
        );
        server.await.unwrap();
        proxy.abort();
        let _ = proxy.await;
    })
    .await
    .expect("large catalog relay stalled");
}

// keeps: interface app-harness — policy is scoped to this seat's first launch request
#[tokio::test]
async fn shared_start_and_resume_enforce_seat_permission_policy_without_changing_later_requests() {
    tokio::time::timeout(Duration::from_secs(10), async {
        for method in ["thread/start", "thread/resume"] {
            let root = tempfile::tempdir().unwrap();
            let frontend = root.path().join("front");
            let backend = root.path().join("back");
            let listener = tokio::net::UnixListener::bind(&backend).unwrap();
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
                config: [("approval_policy".into(), json!("never")), ("sandbox_mode".into(), json!("danger-full-access"))].into(),
                resume: (method == "thread/resume").then(|| "fixture-thread".into()), host: "/usr/bin/true".into(),
            };
            let proxy = tokio::spawn(async move {
                native::codex_proxy(&frontend, &backend, &launch, "/must-not-start-a-daemon").await.unwrap();
            });
            let path = root.path().join("front");
            while !path.exists() { tokio::task::yield_now().await; }
            let (mut client, _) = tokio_tungstenite::client_async("ws://localhost/", tokio::net::UnixStream::connect(path).await.unwrap()).await.unwrap();
            let mut params = json!({"approvalPolicy":"on-request","sandbox":"read-only","config":{"approval_policy":"on-request","sandbox_mode":"read-only"}});
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
