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
