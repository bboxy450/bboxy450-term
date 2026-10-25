#![allow(clippy::disallowed_methods)] // Owned read-only protocol fixtures, never UI.

use bboxy450_harness_wake_up::{Binding, HarnessRecord, RuntimeBinding, Session, Ulid, native};
use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use std::{path::Path, time::Duration};
use tokio_tungstenite::tungstenite::Message;

fn binding(path: &Path) -> Binding {
    let pid = std::process::id();
    let start = native::process_start(pid).unwrap();
    Binding {
        seat: Ulid::new(),
        pid,
        process_start: start.clone(),
        session: format!("process-{pid}"),
        generation: 1,
        attachment_generation: 1,
        runtime: Some(RuntimeBinding {
            pid,
            process_start: start,
            endpoint: path.into(),
        }),
    }
}

// keeps: decision #279
#[tokio::test]
async fn bootstrap_reads_only_the_selected_loaded_thread_and_never_guesses_or_mutates() {
    for (ids, returned, state, expected) in [
        (json!([]), "native-thread", "idle", "empty"),
        (json!(["native-thread"]), "native-thread", "idle", "bound"),
        (json!(["native-thread"]), "native-thread", "active", "bound"),
        (json!(["one", "two"]), "one", "idle", "error"),
        (json!(["native-thread"]), "other-thread", "idle", "error"),
        (
            json!(["native-thread"]),
            "native-thread",
            "notLoaded",
            "error",
        ),
    ] {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("s");
        let listener = tokio::net::UnixListener::bind(&path).unwrap();
        let binding = binding(&path);
        let ids = ids.clone();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut ws = tokio_tungstenite::accept_async(stream).await.unwrap();
            let mut methods = Vec::new();
            while let Some(frame) = ws.next().await {
                let Ok(Message::Text(text)) = frame else {
                    break;
                };
                let value: Value = serde_json::from_str(&text).unwrap();
                let method = value["method"].as_str().unwrap();
                methods.push(method.to_owned());
                let result = match method {
                    "initialize" => json!({}),
                    "initialized" => continue,
                    "thread/loaded/list" => json!({"data":ids,"nextCursor":null}),
                    "thread/read" => {
                        assert_eq!(value["params"]["threadId"], "native-thread");
                        json!({"thread":{"id":returned,"status":{"type":state}}})
                    }
                    other => panic!("bootstrap attempted a non-read method: {other}"),
                };
                ws.send(Message::Text(
                    json!({"id":value["id"],"result":result}).to_string().into(),
                ))
                .await
                .unwrap();
            }
            methods
        });
        let found = native::discover_codex_thread(&binding).await;
        match expected {
            "bound" => {
                let (thread, status) = found.unwrap().unwrap();
                assert_eq!(thread, "native-thread");
                assert_eq!(status["type"], state);
            }
            "empty" => assert!(found.unwrap().is_none()),
            _ => assert!(found.is_err()),
        }
        let methods = tokio::time::timeout(Duration::from_secs(30), server)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            &methods[..3],
            ["initialize", "initialized", "thread/loaded/list"]
        );
        assert!(methods.len() <= 4, "bootstrap retried an unknown result");
    }
}

// keeps: decision #279
#[test]
fn discovery_cannot_replace_a_hook_generation_runtime_or_exited_session() {
    let root = tempfile::tempdir().unwrap();
    let original = binding(&root.path().join("s"));
    let record = HarnessRecord::builtin("codex").unwrap();
    for change in [
        "none",
        "generation",
        "runtime",
        "attachment",
        "hook",
        "exit",
        "unknown",
    ] {
        let mut session = Session::new(record.clone(), original.clone()).unwrap();
        match change {
            "generation" => session.binding.generation += 1,
            "runtime" => session.binding.runtime.as_mut().unwrap().pid += 1,
            "attachment" => session.binding.attachment_generation += 1,
            "hook" => session.binding.session = "hook-bound-thread".into(),
            "exit" => {
                session.exit(1, 1, 0);
            }
            _ => (),
        }
        let before = session.binding.clone();
        let status = json!({"type":if change=="unknown" {"notLoaded"} else {"idle"}});
        let result =
            native::bind_discovered_codex(&mut session, &original, "native-thread", status, 2, 3);
        if change == "none" {
            assert!(result.unwrap());
            assert_eq!(session.binding.session, "native-thread");
            assert_eq!(session.binding.runtime, original.runtime);
            assert_eq!(session.binding.attachment_generation, 2);
        } else {
            if change == "unknown" {
                assert!(result.is_err())
            } else {
                assert!(!result.unwrap())
            }
            assert_eq!(
                session.binding, before,
                "late or unknown proof changed identity"
            );
        }
    }
}

// keeps: decision #279
#[tokio::test]
async fn a_locator_owned_by_another_process_receives_no_protocol_bytes() {
    use tokio::io::AsyncReadExt;
    struct Owned(std::process::Child);
    impl Drop for Owned {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("s");
    let listener = tokio::net::UnixListener::bind(&path).unwrap();
    let child = Owned(
        std::process::Command::new("/bin/sleep")
            .arg("600")
            .spawn()
            .unwrap(),
    );
    eprintln!("owned different-runtime fixture PID {}", child.0.id());
    let mut binding = binding(&path);
    let runtime = binding.runtime.as_mut().unwrap();
    runtime.pid = child.0.id();
    runtime.process_start = native::process_start(runtime.pid).unwrap();
    let peer = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut byte = [0];
        stream.read(&mut byte).await.unwrap()
    });
    assert!(native::discover_codex_thread(&binding).await.is_err());
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(30), peer)
            .await
            .unwrap()
            .unwrap(),
        0
    );
}
