#![allow(
    clippy::disallowed_methods,
    reason = "private fixture IO, never a UI thread"
)]
//! C-2: the real CLI reads records and retained events through the admitted door.
use bboxy450_term::cockpit::{Config, Event, Session};
use bbt_mux::Ulid;
use bbt_proto::{GuiAction, GuiReply};
use serde_json::{Value, json};
use std::{
    io::{BufRead, BufReader},
    os::unix::net::UnixListener,
    path::{Path, PathBuf},
    process::Command,
    time::{Duration, Instant},
};

#[test]
fn ctl_record_read_keeps_the_endpoint_exit_when_the_socket_closes_mid_request() {
    let root = tempfile::tempdir().unwrap();
    let socket = root.path().join("gui.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let endpoint = root.path().join("gui.endpoint");
    let capability = [0x63; 32];
    std::fs::write(
        &endpoint,
        bboxy450_term::ctl::Endpoint {
            path: socket,
            capability,
        }
        .to_json(),
    )
    .unwrap();
    let server = std::thread::spawn(move || {
        let (socket, _) = listener.accept().unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(60)))
            .unwrap();
        let mut request = String::new();
        BufReader::new(socket).read_line(&mut request).unwrap();
        let request: bbt_proto::GuiRequest = serde_json::from_str(&request).unwrap();
        assert!(
            matches!(request.action, GuiAction::Knowledge { method, .. } if method == "resolve")
        );
        // The owner disappears after receiving the request, before returning its result.
    });
    let output = Command::new(env!("CARGO_BIN_EXE_bboxy450-term"))
        .args(["ctl", "record", "fetch", "#1", "--endpoint"])
        .arg(endpoint)
        .env_remove("BBOXY_CTL_ISSUER")
        .current_dir(root.path())
        .output()
        .unwrap();
    server.join().unwrap();
    assert_eq!(output.status.code(), Some(4), "{output:?}");
    let lines = String::from_utf8(output.stdout).unwrap();
    let reply: Value = serde_json::from_str(lines.trim()).unwrap();
    assert_eq!(reply["error"]["kind"], "endpoint_unavailable");
    let secret = "63".repeat(32);
    assert!(!lines.contains(&secret));
    assert!(!String::from_utf8_lossy(&output.stderr).contains(&secret));
}

async fn fixture(root: &Path) -> (Session, PathBuf) {
    let repository = root.join("repository");
    std::fs::create_dir_all(repository.join(".bboxy")).unwrap();
    std::fs::create_dir_all(root.join("zsh-config")).unwrap();
    let run = root.join("account/run");
    let session = Session::start(Config {
        executable: env!("CARGO_BIN_EXE_bboxy450-term").into(),
        socket: run.join("mux.sock"),
        run_dir: run.clone(),
        repository,
        shell_config: Some(root.join("zsh-config")),
        ctl: true,
        test_account: Some(root.join("account")),
    })
    .await
    .unwrap();
    let deadline = Instant::now() + Duration::from_secs(60);
    while !run.join("gui.endpoint").exists() {
        assert!(Instant::now() < deadline, "ctl endpoint never appeared");
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    (session, run)
}

fn call(root: &Path, run: &Path, args: &[&str]) -> (i32, Vec<Value>) {
    let output = Command::new(env!("CARGO_BIN_EXE_bboxy450-term"))
        .arg("ctl")
        .args(args)
        .env("BBOXY_RUN_DIR", run)
        .current_dir(root)
        .output()
        .unwrap();
    let lines = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect::<Vec<_>>();
    assert!(
        !lines.is_empty(),
        "ctl {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    (output.status.code().unwrap(), lines)
}
fn ok(root: &Path, run: &Path, args: &[&str]) -> Vec<Value> {
    let (code, lines) = call(root, run, args);
    assert_eq!(code, 0, "ctl {args:?}: {lines:#?}");
    lines
}
fn data(root: &Path, run: &Path, args: &[&str]) -> Value {
    let lines = ok(root, run, args);
    assert_eq!(lines.len(), 1, "{lines:#?}");
    lines[0]["data"].clone()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ctl_records_fetch_search_pages_and_changes_use_the_private_provider() {
    let root = tempfile::tempdir().unwrap();
    let root = root.path().canonicalize().unwrap();
    let notes = root.join("repository/.bboxy/notes");
    std::fs::create_dir_all(&notes).unwrap();
    for n in 0..36 {
        std::fs::write(
            notes.join(format!("n{n:02}.md")),
            format!(
                "---\nid: {}\nnumber: {}\ntype: {}\ntitle: needle {n:02}\n---\nneedle body {n}\n",
                Ulid::new(),
                n + 1,
                if n == 35 { "spec" } else { "note" }
            ),
        )
        .unwrap();
    }
    let (session, run) = fixture(&root).await;
    let fetched = data(&root, &run, &["record", "fetch", "#1"]);
    assert_eq!(fetched["path"], "notes/n00.md");
    assert_eq!(fetched["body"], "needle body 0\n");
    assert_eq!(fetched["frontmatter"]["type"], "note");
    assert_eq!(data(&root, &run, &["record", "fetch", "n00"]), fetched);
    assert_eq!(
        data(&root, &run, &["record", "fetch", "notes/n00.md"]),
        fetched
    );

    let mut ids = std::collections::BTreeSet::new();
    let mut cursor = None;
    let mut first_cursor = None;
    for _ in 0..10 {
        let mut args = vec!["record", "search", "needle", "--type", "note"];
        if let Some(c) = cursor.as_deref() {
            args.extend(["--cursor", c]);
        }
        let page = data(&root, &run, &args);
        for hit in page["items"].as_array().unwrap() {
            assert_eq!(hit["type"], "note");
            assert!(
                ids.insert(hit["ref"]["id"].as_str().unwrap().to_owned()),
                "duplicate page row"
            );
        }
        cursor = page["next"].as_str().map(str::to_owned);
        if first_cursor.is_none() {
            first_cursor = cursor.clone();
        }
        if cursor.is_none() {
            break;
        }
    }
    assert!(cursor.is_none(), "search never reached its head");
    assert_eq!(ids.len(), 35);
    let first_cursor = first_cursor.expect("fixture exceeds the default provider page");
    let (code, lines) = call(
        &root,
        &run,
        &[
            "record",
            "search",
            "needle",
            "--type",
            "spec",
            "--cursor",
            &first_cursor,
        ],
    );
    assert_eq!(code, 1);
    assert!(
        lines[0]["error"]["message"]
            .as_str()
            .unwrap()
            .contains("cursor")
    );

    let mut since = "0".to_owned();
    for _ in 0..20 {
        let page = data(&root, &run, &["record", "changes", "--since", &since]);
        let next = page["next"].as_str().unwrap().to_owned();
        if next == since {
            break;
        }
        since = next;
    }
    let write = root.join("write.json");
    std::fs::write(&write,json!({"path":"e2e/c2.md","body":"C2 provider write\n",
        "frontmatter":{"type":"e2e","title":"C2 change","function":"T9","group":"terminal","status":"untested"}}).to_string()).unwrap();
    let written = data(&root, &run, &["record", "write", write.to_str().unwrap()]);
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        let page = data(&root, &run, &["record", "changes", "--since", &since]);
        since = page["next"].as_str().unwrap().to_owned();
        if page["changed"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["id"] == written["record"]["id"])
        {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "signed write absent from provider changes: {page}"
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    session.shutdown().await.unwrap();
}

fn consumers(run: &Path, stream: &str) -> Vec<(String, i64)> {
    let path = bbt_mux::store::object_dir(run, stream)
        .unwrap()
        .join("events.sqlite");
    let output = Command::new("python3")
        .args([
            "-c",
            r#"
import json,pathlib,sqlite3,sys
with sqlite3.connect(pathlib.Path(sys.argv[1]).as_uri()+'?mode=ro',uri=True) as db:
    print(json.dumps(db.execute('SELECT name,acked FROM consumers ORDER BY name').fetchall()))
"#,
        ])
        .arg(path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}
fn tail(root: &Path, run: &Path, stream: &str, since: u64, subject: &str) -> Vec<Value> {
    ok(
        root,
        run,
        &[
            "events",
            "tail",
            "--stream",
            stream,
            "--since",
            &since.to_string(),
            "--subject",
            subject,
        ],
    )
}
fn has(lines: &[Value], text: &str) -> bool {
    lines
        .iter()
        .any(|r| r.get("event").is_some_and(|e| e.to_string().contains(text)))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ctl_events_discover_filter_and_resume_without_a_second_consumer_or_ack() {
    let root = tempfile::tempdir().unwrap();
    let root = root.path().canonicalize().unwrap();
    let (mut session, run) = fixture(&root).await;
    let swarm = Ulid::new().to_string();
    ok(
        &root,
        &run,
        &[
            "seat",
            "spawn",
            "zsh",
            "--swarm",
            &swarm,
            "--cwd",
            root.to_str().unwrap(),
        ],
    );
    let stream = format!("swarm-{swarm}");
    let attach = Ulid::new();
    session
        .send(GuiAction::AttachEvents { id: attach })
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(60), async {
        loop {
            if let Event::Reply(reply) = session.event().await.unwrap() {
                match *reply {
                    GuiReply::Ok { op } if op == attach => break,
                    GuiReply::Error { op, reason } if op == attach || op.is_nil() => {
                        panic!("events attachment: {reason}")
                    }
                    _ => {}
                }
            }
        }
    })
    .await
    .expect("fixture GUI lane never attached");
    let before = consumers(&run, &stream);
    assert!(!before.is_empty(), "fixture must already own a GUI lane");
    let streams = data(&root, &run, &["events", "streams"]);
    assert!(
        streams["streams"]
            .as_array()
            .unwrap()
            .contains(&json!(stream))
    );
    ok(
        &root,
        &run,
        &[
            "message",
            "post",
            "operator/inbox",
            "C2-event-one",
            "--swarm",
            &swarm,
        ],
    );
    let deadline = Instant::now() + Duration::from_secs(60);
    let first = loop {
        let rows = tail(&root, &run, &stream, 0, "operator/inbox/message");
        if has(&rows, "C2-event-one") {
            break rows;
        }
        assert!(
            Instant::now() < deadline,
            "posted message never reached retained events"
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    };
    for row in &first {
        assert_eq!(row["stream"], stream);
        assert!(row["seq"].is_u64());
    }
    let after = first.last().unwrap()["next_since"].as_u64().unwrap();
    assert!(first.last().unwrap()["done"].as_bool().unwrap());
    assert!(after > 0);
    ok(
        &root,
        &run,
        &[
            "message",
            "post",
            "operator/inbox",
            "C2-event-two",
            "--swarm",
            &swarm,
        ],
    );
    let deadline = Instant::now() + Duration::from_secs(60);
    let second = loop {
        let rows = tail(&root, &run, &stream, after, "operator/inbox/message");
        if has(&rows, "C2-event-two") {
            break rows;
        }
        assert!(
            Instant::now() < deadline,
            "second message never reached retained events"
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    };
    assert!(!has(&second, "C2-event-one"));
    assert!(
        second
            .iter()
            .filter(|r| r.get("event").is_some())
            .all(|r| r["seq"].as_u64().unwrap() > after)
    );
    let filtered = tail(&root, &run, &stream, 0, "does/not/match");
    assert_eq!(filtered.len(), 1);
    let scanned = filtered[0]["next_since"].as_u64().unwrap();
    assert!(scanned >= second.last().unwrap()["next_since"].as_u64().unwrap());
    assert_eq!(
        consumers(&run, &stream),
        before,
        "tail changed the GUI lane/ACK or made another consumer"
    );

    let (code, error) = call(
        &root,
        &run,
        &[
            "events",
            "tail",
            "--stream",
            "missing-stream",
            "--since",
            "7",
        ],
    );
    assert_eq!(code, 1);
    assert_eq!(error.last().unwrap()["next_since"], 7);
    assert_eq!(error.last().unwrap()["stream"], "missing-stream");
    assert!(
        error.last().unwrap()["error"]["hint"]
            .as_str()
            .unwrap()
            .contains("events streams")
    );
    for args in [
        vec!["events", "tail"],
        vec![
            "events",
            "tail",
            "--stream",
            "account",
            "--since",
            "not-a-sequence",
        ],
        vec!["record", "changes"],
        vec!["record", "fetch", "#1", "--cursor", "not-applicable"],
    ] {
        let (code, lines) = call(&root, &run, &args);
        assert_eq!(code, 2, "{args:?}: {lines:?}");
        assert_eq!(lines[0]["error"]["kind"], "usage");
    }
    session.shutdown().await.unwrap();
}
