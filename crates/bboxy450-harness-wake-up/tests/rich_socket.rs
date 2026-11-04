// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Private Unix boundary witnesses. No credentials, desktop, or operator input.
#![allow(clippy::disallowed_methods, reason="private scripted socket fixture, never the UI owner")]
use bboxy450_harness_wake_up::{
    Evidence, WakeCause,
    rich::{Client, Connection, socket_path, valid_session},
};
use serde_json::{Value, json};
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::UnixListener,
};
use ulid::Ulid;
async fn ready(client: &Client, session: Ulid) {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if client.hello().is_some_and(|h| h.session == session) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
}
#[test]
fn launch_path_and_resume_are_bounded() {
    let id = Ulid::new();
    assert_eq!(
        socket_path(std::path::Path::new("/tmp/test/control.sock"), id).unwrap(),
        std::path::PathBuf::from(format!("/tmp/test/h-{id}.sock"))
    );
    assert!(
        socket_path(
            std::path::Path::new(&format!("/tmp/{}/ctl", "a".repeat(96))),
            id
        )
        .is_err()
    );
    assert!(socket_path(std::path::Path::new("relative/ctl"), id).is_err());
    assert!(valid_session(&id.to_string()));
    for bad in [
        "../escape",
        "profile-a",
        "00000000000000000000000000",
        "01m6hcyvge7cj8kk6pb20spf3d",
    ] {
        assert!(!valid_session(bad));
    }
}
#[tokio::test]
async fn private_socket_duplicate_busy_closed_reconnect_and_generation() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("h.sock");
    let listener = UnixListener::bind(&path).unwrap();
    let session = Arc::new(Mutex::new(Ulid::new()));
    let current = session.clone();
    let turns = Arc::new(AtomicUsize::new(0));
    let count = turns.clone();
    let seen = Arc::new(Mutex::new(std::collections::BTreeSet::new()));
    let known = seen.clone();
    let server = tokio::spawn(async move {
        loop {
            let (socket, _) = listener.accept().await.unwrap();
            let s = *current.lock().unwrap();
            let count = count.clone();
            let known = known.clone();
            tokio::spawn(async move {
                let (r, mut w) = socket.into_split();
                let mut r = BufReader::new(r);
                let mut line = String::new();
                r.read_line(&mut line).await.unwrap();
                let sub: Value = serde_json::from_str(&line).unwrap();
                assert!(sub["subscribe"].get("session").is_none());
                // Hello is deliberately fragmented around command/timer opportunities.
                let hello = format!(
                    "{}\n",
                    json!({"v":{"major":1,"minor":1},"meta":{"session":s},"epoch":1,"head":0})
                );
                w.write_all(&hello.as_bytes()[..10]).await.unwrap();
                tokio::time::sleep(Duration::from_millis(130)).await;
                w.write_all(&hello.as_bytes()[10..]).await.unwrap();
                // A fork/resume can replay durable bytes from its ancestry.
                w.write_all(format!("{}\n",json!({"v":{"major":1,"minor":1},"session":Ulid::new(),"epoch":0,"seq":1,"event":{"type":"session_closed"}})).as_bytes()).await.unwrap();
                loop {
                    line.clear();
                    if r.read_line(&mut line).await.unwrap() == 0 {
                        return;
                    }
                    let op: Value = serde_json::from_str(&line).unwrap();
                    let id = op["op_id"].clone();
                    if op["op"]["test"] == "disconnect" {
                        return;
                    }
                    if op["op"]["test"] == "oversize" {
                        let _ = w.write_all(&vec![b'x'; 256 * 1024 + 1]).await;
                        return;
                    }
                    let answer = if op["session"] != json!(s) {
                        json!({"op_id":id,"refused":{"reason":"unknown_session"}})
                    } else if op["op"]["test"] == "busy" {
                        json!({"op_id":id,"refused":{"reason":"busy"}})
                    } else if op["op"]["test"] == "closed" {
                        json!({"op_id":id,"refused":{"reason":"closed"}})
                    } else {
                        let duplicate = !known.lock().unwrap().insert((s, id.to_string()));
                        if !duplicate {
                            count.fetch_add(1, Ordering::SeqCst);
                        }
                        json!({"op_id":id,"ack":{"op_id":id,"duplicate":duplicate}})
                    };
                    let wire = format!("{answer}\n");
                    if w.write_all(wire.as_bytes()).await.is_err() {
                        return;
                    }
                }
            });
        }
    });
    let owner = Connection::start(path.clone(), &tokio::runtime::Handle::current());
    let client = owner.client.clone();
    let a = *session.lock().unwrap();
    ready(&client, a).await;
    let id = Ulid::new();
    let text =
        json!({"type":"user_turn","blocks":[{"type":"text","text":"fixture"}],"mode":"queue"});
    for attempt in 0..2 {
        let id = if attempt == 1
            && std::env::var("BBOXY_RICH_TEST_CONTROL").as_deref() == Ok("fresh-id")
        {
            Ulid::new()
        } else {
            id
        };
        let result = client.send(id, a, text.clone()).await;
        assert_eq!(result.evidence, Evidence::NativeQueued);
        assert_eq!(result.native_id, Some(id.to_string()));
    }
    assert_eq!(
        turns.load(Ordering::SeqCst),
        1,
        "one turn per message; duplicate control must produce two turns"
    );
    assert_eq!(
        client
            .send(Ulid::new(), a, json!({"test":"busy"}))
            .await
            .cause,
        Some(WakeCause::HarnessBusy)
    );
    assert_eq!(
        client
            .send(Ulid::new(), a, json!({"test":"closed"}))
            .await
            .cause,
        Some(WakeCause::HarnessSessionClosed)
    );
    let result = client
        .send(Ulid::new(), a, json!({"test":"disconnect"}))
        .await;
    assert_eq!(result.evidence, Evidence::NativeUnknown);
    let b = Ulid::new();
    *session.lock().unwrap() = b;
    ready(&client, b).await;
    assert_eq!(
        client.send(id, a, text.clone()).await.cause,
        Some(WakeCause::HarnessSessionClosed)
    );
    let result = client
        .send(Ulid::new(), b, json!({"test":"disconnect"}))
        .await;
    assert_eq!(result.evidence, Evidence::NativeUnknown);
    let a2 = Ulid::new();
    *session.lock().unwrap() = a2;
    ready(&client, a2).await;
    assert_eq!(
        client.send(id, a, text).await.cause,
        Some(WakeCause::HarnessSessionClosed)
    );
    assert_eq!(turns.load(Ordering::SeqCst), 1);
    drop(owner);
    assert_eq!(
        client.send(Ulid::new(), a2, json!({})).await.cause,
        Some(WakeCause::HarnessSocketUnavailable)
    );
    server.abort();
}
#[tokio::test]
async fn unknown_on_timeout_and_oversize_never_admits_another_turn() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("h.sock");
    let s = Ulid::new();
    let listener = UnixListener::bind(&path).unwrap();
    let server = tokio::spawn(async move {
        let (socket, _) = listener.accept().await.unwrap();
        let (r, mut w) = socket.into_split();
        let mut r = BufReader::new(r);
        let mut l = String::new();
        r.read_line(&mut l).await.unwrap();
        w.write_all(
            format!(
                "{}\n",
                json!({"v":{"major":0,"minor":1},"meta":{"session":s},"epoch":4,"head":7})
            )
            .as_bytes(),
        )
        .await
        .unwrap();
        l.clear();
        r.read_line(&mut l).await.unwrap();
        tokio::time::sleep(Duration::from_secs(6)).await;
        let _ = w.write_all(&vec![b'x'; 256 * 1024 + 1]).await;
    });
    let owner = Connection::start(path, &tokio::runtime::Handle::current());
    ready(&owner.client, s).await;
    let result = owner
        .client
        .send(Ulid::new(), s, json!({"type":"interrupt","turn":null}))
        .await;
    assert_eq!(result.evidence, Evidence::NativeUnknown);
    assert_eq!(result.cause, Some(WakeCause::HarnessAckUnknown));
    tokio::time::sleep(Duration::from_millis(1200)).await;
    assert!(owner.client.hello().is_none());
    server.await.unwrap();
}
#[tokio::test]
async fn future_major_and_absent_socket_have_no_admission() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("h.sock");
    let owner = Connection::start(path.clone(), &tokio::runtime::Handle::current());
    assert_eq!(
        owner
            .client
            .send(Ulid::new(), Ulid::new(), json!({}))
            .await
            .cause,
        Some(WakeCause::HarnessSocketUnavailable)
    );
    let listener = UnixListener::bind(&path).unwrap();
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        socket.write_all(format!("{}\n",json!({"v":{"major":2,"minor":0},"meta":{"session":Ulid::new()},"epoch":1,"head":0})).as_bytes()).await.unwrap();
    });
    tokio::time::sleep(Duration::from_millis(2300)).await;
    assert!(owner.client.hello().is_none());
    server.await.unwrap();
}
