#![allow(
    clippy::disallowed_methods,
    reason = "test/fixture IO, not the UI thread"
)]
//! A-1 acceptance (e2e-acceptance §3, decision #260): the app runs headless with memory
//! custody and `--ctl`; from outside it, `bboxy450-term ctl` spawns a Claude seat, lists it,
//! DMs it, reads the seat's reply in the operator inbox, sees the seat's read mark,
//! delegates a task to it and reads the seat's report with its verify_result.
//!
//! The "claude" here is a stand-in first on PATH: it answers through the real seat MCP
//! bridge (`bboxy450-term mcp`) like a harness would, so no model and no network run.
use bboxy450_term::cockpit::{Config, Event, Session};
use bbt_proto::{GuiAction, GuiReply};
use serde_json::Value;
use std::{
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
    time::{Duration, Instant},
};

const AGENT: &str = r#"import json, os, re, subprocess, time
BIN = os.environ["BBT_BIN"]
ME = os.environ.get("BBOXY_SEAT_ID", "")
LOG = os.environ.get("BBT_AGENT_LOG", "/dev/null")
seen = set()
def ulid():
    n = (int(time.time() * 1000) << 80) | int.from_bytes(os.urandom(10), "big")
    return "".join("0123456789ABCDEFGHJKMNPQRSTVWXYZ"[(n >> (5 * i)) & 31] for i in range(25, -1, -1))
def mcp(calls):
    lines = [{"jsonrpc": "2.0", "id": 0, "method": "initialize", "params": {"protocolVersion": "2025-06-18",
             "capabilities": {}, "clientInfo": {"name": "fake-claude", "version": "0"}}},
             {"jsonrpc": "2.0", "method": "notifications/initialized"}]
    for i, (name, args) in enumerate(calls, 1):
        lines.append({"jsonrpc": "2.0", "id": i, "method": "tools/call",
                      "params": {"name": name, "arguments": args, "_meta": {"bboxy450/op_id": ulid()}}})
    p = subprocess.run([BIN, "mcp"], input="".join(json.dumps(l) + "\n" for l in lines),
                       capture_output=True, text=True, timeout=20)
    out = {}
    for line in p.stdout.splitlines():
        try:
            m = json.loads(line)
        except ValueError:
            continue
        if "id" in m:
            out[m["id"]] = m
    return [out.get(i, {"stderr": p.stderr}) for i in range(1, len(calls) + 1)]
def text_of(env):
    body = env.get("body") or {}
    if isinstance(body, dict):
        if isinstance(body.get("text"), str):
            return body["text"]
        for value in body.values():
            if isinstance(value, dict) and isinstance(value.get("text"), str):
                return value["text"]
    return ""
while True:
    try:
        (listed,) = mcp([("message_list", {"limit": 32})])
        sc = (listed.get("result") or {}).get("structuredContent") or {}
        items = sc.get("items") or (sc.get("page") or {}).get("items") or []
        for m in items:
            env = m.get("envelope", m)
            mid = env.get("id")
            if not mid or mid in seen or env.get("seat") == ME:
                continue
            seen.add(mid)
            text = text_of(env)
            calls = [("message_ack", {"message": mid}),
                     ("message_send", {"destination": "operator/inbox", "text": "pong: " + text, "reply_to": mid})]
            if text == "knock":
                # A real seat process trying the operator's door (decision #260).
                env = dict(os.environ, BBOXY_RUN_DIR=os.environ["BBT_RUN"])
                knocked = subprocess.run([BIN, "ctl", "seats"], env=env, capture_output=True, text=True, timeout=20)
                calls[1] = ("message_send", {"destination": "operator/inbox", "reply_to": mid,
                            "text": "knock: " + knocked.stdout.strip() + " exit=" + str(knocked.returncode)})
            report = re.match(r"report (\S+)", text)
            if report:
                calls.append(("task_report", {"task_path": report.group(1), "summary": "stand-in claude reported",
                              "verify_result": {"command": "true", "exit_code": 0, "evidence": []}}))
            with open(LOG, "a") as f:
                f.write(json.dumps({"message": mid, "text": text, "results": mcp(calls)}) + "\n")
    except Exception as error:
        with open(LOG, "a") as f:
            f.write(json.dumps({"error": repr(error)}) + "\n")
    time.sleep(0.3)
"#;

fn quote(path: &Path) -> String {
    format!("'{}'", path.to_string_lossy().replace('\'', "'\\''"))
}
fn executable(path: &Path, body: &str) {
    std::fs::write(path, body).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).unwrap();
}

/// Runs `bboxy450-term ctl ARGS` as an outside process; returns its JSON lines.
fn ctl(run: &Path, args: &[&str]) -> Vec<Value> {
    let output = Command::new(env!("CARGO_BIN_EXE_bboxy450-term"))
        .arg("ctl")
        .args(args)
        .env("BBOXY_RUN_DIR", run)
        .output()
        .unwrap();
    let lines: Vec<Value> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|line| serde_json::from_str(line).expect("ctl prints JSON lines"))
        .collect();
    assert!(
        output.status.success(),
        "ctl {args:?}: {lines:?} {}",
        String::from_utf8_lossy(&output.stderr)
    );
    lines
}
fn until<T>(what: &str, log: &Path, mut probe: impl FnMut() -> Option<T>) -> T {
    let deadline = Instant::now() + Duration::from_secs(45);
    loop {
        if let Some(found) = probe() {
            return found;
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for {what}; agent log:\n{}",
            std::fs::read_to_string(log).unwrap_or_default()
        );
        std::thread::sleep(Duration::from_millis(250));
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ctl_drives_a_claude_seat_from_outside_the_app() {
    let root = tempfile::tempdir().unwrap();
    let root = root.path().canonicalize().unwrap();
    let run = root.join("run");
    let repository = root.join("repository");
    let bin = root.join("bin");
    let log = root.join("agent.log");
    for dir in [
        &repository.join(".bboxy/tasks"),
        &bin,
        &root.join("zsh-config"),
    ] {
        std::fs::create_dir_all(dir).unwrap();
    }
    let task = "01K00000000000000000000A1C";
    let task_path = format!(".bboxy/tasks/{task}.md");
    std::fs::write(
        repository.join(&task_path),
        format!("---\nid: {task}\nnumber: 1\ntype: task\ntitle: ctl acceptance\nstatus: open\nverify: 'true'\n---\nReport through ctl.\n"),
    )
    .unwrap();
    let binary = PathBuf::from(env!("CARGO_BIN_EXE_bboxy450-term"));
    std::fs::write(bin.join("agent.py"), AGENT).unwrap();
    executable(
        &bin.join("claude"),
        &format!(
            "#!/bin/sh\nBBT_BIN={} BBT_AGENT_LOG={} BBT_RUN={} exec python3 -I {}\n",
            quote(&binary),
            quote(&log),
            quote(&run),
            quote(&bin.join("agent.py"))
        ),
    );
    // Memory custody, and the stand-in claude first on PATH, for the app's children only.
    let app = root.join("test-app");
    executable(
        &app,
        &format!(
            "#!/bin/sh\nBBOXY_TEST_CUSTODY=memory PATH={}:\"$PATH\" exec {} \"$@\"\n",
            quote(&bin),
            quote(&binary)
        ),
    );
    let session = Session::start(Config {
        executable: app,
        socket: run.join("mux.sock"),
        run_dir: run.clone(),
        repository: repository.clone(),
        shell_config: Some(root.join("zsh-config")),
        ctl: true,
    })
    .await
    .unwrap();
    let endpoint = run.join("gui.endpoint");
    until("gui.endpoint", &log, || endpoint.exists().then_some(()));

    // spawn, then seats lists it with harness=claude.
    let spawned = ctl(
        &run,
        &[
            "spawn",
            "claude",
            "--placement",
            "new-tab",
            "--cwd",
            repository.to_str().unwrap(),
        ],
    );
    assert_eq!(spawned[0]["kind"], "spawned", "{spawned:?}");
    let seat = spawned[0]["seat"].as_str().unwrap().to_owned();
    let listed = until("the seat in ctl seats", &log, || {
        ctl(&run, &["seats"])
            .into_iter()
            .find(|s| s["seat"] == seat.as_str())
    });
    assert_eq!(listed["harness"], "claude", "{listed}");

    // dm, then inbox shows the seat's reply, then receipts show its read mark.
    let sent = ctl(&run, &["dm", &seat, "ping"]);
    assert_eq!(sent[0]["kind"], "sent", "{sent:?}");
    let message = sent[0]["message"].as_str().unwrap().to_owned();
    let reply = until("the seat's reply in ctl inbox", &log, || {
        ctl(&run, &["inbox"])
            .into_iter()
            .find(|m| m["message"]["envelope"]["reply_to"] == message.as_str())
    });
    assert_eq!(reply["kind"], "message", "{reply}");
    let acked = until("the seat's read mark in ctl receipts", &log, || {
        ctl(&run, &["receipts", &message])
            .into_iter()
            .find(|r| r["receipt"]["evidence"] == "application-acknowledged")
    });
    assert_eq!(acked["kind"], "receipt", "{acked}");

    // A real seat process (a child of the app's own supervisor) is refused at ctl's door,
    // by name, and nothing it asked for ran (decision #260, invariant #261).
    let knock = ctl(&run, &["dm", &seat, "knock"]);
    let knock = knock[0]["message"].as_str().unwrap().to_owned();
    let answer = until("the seat's knock result", &log, || {
        ctl(&run, &["inbox"])
            .into_iter()
            .find(|m| m["message"]["envelope"]["reply_to"] == knock.as_str())
    });
    let text = answer["message"]["envelope"]["body"]["text"]
        .as_str()
        .unwrap_or_default();
    assert!(
        text.contains(bbt_mux::ctl_door::REFUSAL) && text.ends_with("exit=1"),
        "a seat must be refused at the door: {text}"
    );
    assert!(
        !text.contains("\"kind\":\"seat\""),
        "the refused seat saw no roster: {text}"
    );

    // delegate, then the seat reports, then reports returns its verify_result.
    // By #N, the selector the spec promises (NUM2); the report names the path.
    let delegated = ctl(&run, &["delegate", "#1", &seat]);
    assert_eq!(delegated[0]["kind"], "delegated", "{delegated:?}");
    ctl(&run, &["dm", &seat, &format!("report {task_path}")]);
    let report = until("the task's report in ctl reports", &log, || {
        ctl(&run, &["reports"])
            .into_iter()
            .find(|r| r["path"] == task_path.as_str())
    });
    assert_eq!(report["verify_result"]["exit_code"], 0, "{report}");
    assert_eq!(report["verify_result"]["command"], "true", "{report}");

    session.shutdown().await.unwrap();
    assert!(
        !endpoint.exists(),
        "gui.endpoint is removed when the app stops"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn without_ctl_the_app_writes_no_endpoint() {
    let root = tempfile::tempdir().unwrap();
    let root = root.path().canonicalize().unwrap();
    let run = root.join("run");
    let repository = root.join("repository");
    std::fs::create_dir_all(&repository).unwrap();
    std::fs::create_dir_all(root.join("zsh-config")).unwrap();
    let app = root.join("test-app");
    executable(
        &app,
        &format!(
            "#!/bin/sh\nBBOXY_TEST_CUSTODY=memory exec {} \"$@\"\n",
            quote(&PathBuf::from(env!("CARGO_BIN_EXE_bboxy450-term")))
        ),
    );
    let session = Session::start(Config {
        executable: app,
        socket: run.join("mux.sock"),
        run_dir: run.clone(),
        repository,
        shell_config: Some(root.join("zsh-config")),
        ctl: false,
    })
    .await
    .unwrap();
    // The logic child is ready once Session::start returns; no door, no file.
    assert!(
        !run.join("gui.endpoint").exists(),
        "no gui.endpoint without --ctl"
    );
    let refused = Command::new(env!("CARGO_BIN_EXE_bboxy450-term"))
        .args(["ctl", "seats"])
        .env("BBOXY_RUN_DIR", &run)
        .output()
        .unwrap();
    assert!(!refused.status.success(), "ctl has no door to knock on");
    session.shutdown().await.unwrap();
}

/// `ctl capture` is relayed by logic to the window's own session and answered by it; this
/// test process is the window (logic's parent, so the window's door). A real window writes
/// the presented frame; here the test writes the file and answers, which is the protocol.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ctl_capture_is_answered_by_the_window() {
    let root = tempfile::tempdir().unwrap();
    let root = root.path().canonicalize().unwrap();
    let run = root.join("run");
    let repository = root.join("repository");
    std::fs::create_dir_all(&repository).unwrap();
    std::fs::create_dir_all(root.join("zsh-config")).unwrap();
    let app = root.join("test-app");
    executable(
        &app,
        &format!(
            "#!/bin/sh\nBBOXY_TEST_CUSTODY=memory exec {} \"$@\"\n",
            quote(&PathBuf::from(env!("CARGO_BIN_EXE_bboxy450-term")))
        ),
    );
    let mut session = Session::start(Config {
        executable: app,
        socket: run.join("mux.sock"),
        run_dir: run.clone(),
        repository,
        shell_config: Some(root.join("zsh-config")),
        ctl: true,
    })
    .await
    .unwrap();
    let endpoint = run.join("gui.endpoint");
    until("gui.endpoint", &root.join("none"), || {
        endpoint.exists().then_some(())
    });
    // The window's session is admitted by its first request, as the real window's is.
    session.list().await.unwrap();

    // Answers the next CaptureAsk as the window does: written, or why not.
    async fn answer(session: &mut Session, error: Option<&str>) -> (String, Option<String>) {
        loop {
            let event = tokio::time::timeout(Duration::from_secs(20), session.event())
                .await
                .expect("the window is asked to capture")
                .unwrap();
            if let Event::Reply(reply) = event
                && let GuiReply::CaptureAsk { op, path, seat } = *reply
            {
                if error.is_none() {
                    std::fs::write(&path, b"\x89PNG stand-in").unwrap();
                }
                session
                    .send(GuiAction::Captured {
                        id: op,
                        error: error.map(Into::into),
                    })
                    .await
                    .unwrap();
                return (path, seat.map(|seat| seat.to_string()));
            }
        }
    }

    // A relative path is made absolute by ctl, written by the window, and printed.
    let out = root.join("window.png");
    let asked = tokio::task::spawn_blocking({
        let (run, root) = (run.clone(), root.clone());
        move || ctl_in(&run, &root, &["capture", "window.png"])
    });
    let (path, seat) = answer(&mut session, None).await;
    assert_eq!((path.as_str(), seat), (out.to_str().unwrap(), None));
    let (ok, lines) = asked.await.unwrap();
    assert!(ok, "{lines:?}");
    assert_eq!(lines[0]["kind"], "captured", "{lines:?}");
    assert_eq!(lines[0]["path"], out.to_str().unwrap());
    assert!(std::fs::read(&out).unwrap().starts_with(b"\x89PNG"));

    // The window's refusal reaches ctl as its error and exit 1.
    let asked = tokio::task::spawn_blocking({
        let (run, root) = (run.clone(), root.clone());
        move || ctl_in(&run, &root, &["capture", "/nonexistent/x.png"])
    });
    answer(&mut session, Some("the window is not presenting")).await;
    let (ok, lines) = asked.await.unwrap();
    assert!(!ok);
    assert_eq!(lines[0]["kind"], "error", "{lines:?}");
    assert_eq!(lines[0]["reason"], "the window is not presenting");

    // An unknown seat is refused by ctl before anything is asked of the window.
    let (ok, lines) = ctl_in(&run, &root, &["capture", "x.png", "--seat", "nobody"]);
    assert!(!ok && lines[0]["kind"] == "error", "{lines:?}");

    session.shutdown().await.unwrap();
}

/// Runs `bboxy450-term ctl ARGS` from `cwd`; returns success and its JSON lines.
fn ctl_in(run: &Path, cwd: &Path, args: &[&str]) -> (bool, Vec<Value>) {
    let output = Command::new(env!("CARGO_BIN_EXE_bboxy450-term"))
        .arg("ctl")
        .args(args)
        .env("BBOXY_RUN_DIR", run)
        .current_dir(cwd)
        .output()
        .unwrap();
    let lines = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|line| serde_json::from_str(line).expect("ctl prints JSON lines"))
        .collect();
    (output.status.success(), lines)
}

#[test]
fn ctl_help_prints_its_own_usage() {
    let output = Command::new(env!("CARGO_BIN_EXE_bboxy450-term"))
        .args(["ctl", "--help"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.starts_with("usage: bboxy450-term ctl"), "{text}");
    assert!(text.contains("capture <file.png>"), "{text}");
}
