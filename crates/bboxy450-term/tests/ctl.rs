// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
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
use serde_json::{Value, json};
use std::{
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
    time::{Duration, Instant},
};

const AGENT: &str = r#"import base64, json, os, re, socket, subprocess, sys, time
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
native = os.environ.get("BBT_NATIVE_SOCKET")
listener = None
if native:
    listener = socket.socket(socket.AF_UNIX)
    listener.bind(native)
    os.chmod(native, 0o600)
    listener.listen(8)
    listener.settimeout(45)
    session = sys.argv[sys.argv.index("--session-id") + 1]
    raw = {"hook_event_name": "SessionStart", "session_id": session,
           "harness": "claude", "bboxy_native_socket": native}
    payload = base64.b64encode(json.dumps({"raw": raw}).encode()).decode()
    print("\033]7502;v=1:seat=" + ME + ":ev=SessionStart:d=" + payload + "\033\\", flush=True)
    with open(LOG, "a") as f:
        f.write(json.dumps({"native": "osc-only", "session": session}) + "\n")
    while not os.path.exists(os.environ["BBT_RELEASE_HOOK"]):
        time.sleep(0.01)
    env = dict(os.environ, CLAUDE_CODE_MESSAGING_SOCKET=native,
               CLAUDE_CODE_MESSAGING_TOKEN="fixture-token-never-forward")
    hook = subprocess.run([BIN, "hook", "--agent", "claude"],
                          input=json.dumps(raw), env=env, text=True, capture_output=True, timeout=10)
    assert hook.returncode == 0 and not hook.stdout, (hook.returncode, hook.stdout, hook.stderr)
    with open(LOG, "a") as f:
        f.write(json.dumps({"native": "hook-written"}) + "\n")
while True:
    if listener:
        connection, _ = listener.accept()
        with connection, connection.makefile("rb") as stream:
            line = stream.readline(65537)
        if not line:
            continue
        assert len(line) <= 65536
        frame = json.loads(line)
        with open(LOG, "a") as f:
            f.write(json.dumps({"native": "received", "id": frame["msg_id"],
                                "text": frame["message"]["content"]}) + "\n")
        if "RESTORE-ONLY" in frame["message"]["content"]:
            continue
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
    if not listener:
        time.sleep(0.3)
"#;

fn quote(path: &Path) -> String {
    format!("'{}'", path.to_string_lossy().replace('\'', "'\\''"))
}
fn executable(path: &Path, body: &str) {
    std::fs::write(path, body).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).unwrap();
}

fn fixture_login(root: &Path, bin: &Path) {
    let home = root.join("home");
    std::fs::create_dir_all(&home).unwrap();
    let path = format!("{}:{}", bin.display(), std::env::var("PATH").unwrap());
    std::fs::write(
        root.join("zsh-config/.zprofile"),
        format!(
            "export HOME={}\nexport PATH='{}'\n",
            quote(&home),
            path.replace('\'', "'\\''")
        ),
    )
    .unwrap();
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
    fixture_login(&root, &bin);
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
            "#!/bin/sh\nHOME={} CLAUDE_CONFIG_DIR={} BBOXY_TEST_CUSTODY=memory PATH={}:\"$PATH\" exec {} \"$@\"\n",
            quote(&root.join("home")),
            quote(&root.join("home/.claude")),
            quote(&bin),
            quote(&binary)
        ),
    );
    let mut session = Session::start(Config {
        executable: app,
        socket: run.join("mux.sock"),
        run_dir: run.clone(),
        repository: repository.clone(),
        shell_config: Some(root.join("zsh-config")),
        ctl: true,
        test_account: None,
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

    // MA-6: the operator reads the host's policy without borrowing the receiver's identity.
    let policy = ctl(&run, &["seat", "attention", &seat]);
    assert_eq!(policy[0]["noun"], "seat");
    assert_eq!(policy[0]["verb"], "attention");
    assert_eq!(policy[0]["data"]["seat_id"], seat);
    assert_eq!(policy[0]["data"]["policy"], json!({"rules":[]}));
    assert_eq!(policy[0]["data"]["defaults"]["event"], "inbox");
    assert_eq!(policy[0]["data"]["muted"], false);

    // --role takes the launcher's preset table and guard (#262): an executor Claude is a
    // worker, Codex is never an oracle, and an unknown name is refused by that name.
    let repo = repository.to_str().unwrap();
    let tester = ctl(
        &run,
        &[
            "spawn", "claude", "--role", "executor", "--cwd", repo, "--name", "tester",
        ],
    );
    let tester = tester[0]["seat"].as_str().unwrap().to_owned();
    let tester = until("the executor seat in ctl seats", &log, || {
        ctl(&run, &["seats"])
            .into_iter()
            .find(|s| s["seat"] == tester.as_str())
    });
    assert_eq!(tester["role"]["name"], "executor", "{tester}");
    assert_eq!(tester["role"]["class"], "worker", "{tester}");
    for (harness, role, reason) in [
        ("codex", "oracle", "codex_oracle_role_forbidden"),
        ("claude", "wizard", "unknown_role_preset"),
    ] {
        let (ok, lines) = ctl_in(
            &run,
            &root,
            &["spawn", harness, "--role", role, "--cwd", repo],
        );
        assert!(!ok, "{harness} --role {role}: {lines:?}");
        assert!(
            lines[0]["reason"]
                .as_str()
                .unwrap_or_default()
                .contains(reason),
            "{harness} --role {role}: {lines:?}"
        );
    }

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
    // Issued as 'Vesper (Oracle)' (decision #269): the Delegate message and the DM carry
    // door ctl and that issuer in their signed envelopes.
    let issuer = "Vesper (Oracle)";
    let delegated = ctl(&run, &["--as", issuer, "delegate", "#1", &seat]);
    assert_eq!(delegated[0]["kind"], "delegated", "{delegated:?}");
    ctl(
        &run,
        &["--as", issuer, "dm", &seat, &format!("report {task_path}")],
    );
    let swarm: bbt_mux::Ulid = spawned[0]["swarm"].as_str().unwrap().parse().unwrap();
    let list = bbt_mux::Ulid::new();
    session
        .send(GuiAction::Messaging {
            id: list,
            swarm_id: bbt_proto::SwarmId::from_u128(swarm.0),
            request: bbt_proto::messaging::Request::List {
                thread: None,
                channel: Some(format!("seats/{seat}/inbox")),
                after: 0,
                limit: 128,
            },
        })
        .await
        .unwrap();
    let page = loop {
        let event = tokio::time::timeout(Duration::from_secs(20), session.event())
            .await
            .expect("the seat's inbox page")
            .unwrap();
        if let Event::Reply(reply) = event
            && let GuiReply::Messaging {
                op,
                response: bbt_proto::messaging::Response::Messages { page },
            } = *reply
            && op == list
        {
            break page;
        }
    };
    let issued: Vec<_> = page
        .items
        .iter()
        .filter(|m| {
            m.envelope()
                .expect("signed fixture envelope")
                .issued_by
                .as_deref()
                == Some(issuer)
        })
        .collect();
    assert!(
        issued
            .iter()
            .any(|m| m.envelope().expect("signed fixture envelope").kind
                == bbt_proto::messaging::MessageKind::Task),
        "the Delegate message names its issuer: {:?}",
        page.items
            .iter()
            .map(|m| (
                &m.envelope().expect("signed fixture envelope").kind,
                &m.envelope().expect("signed fixture envelope").issued_by
            ))
            .collect::<Vec<_>>()
    );
    assert!(
        issued.iter().any(
            |m| m.envelope().expect("signed fixture envelope").body.text()
                == Some(&format!("report {task_path}"))
        ),
        "the DM names its issuer"
    );
    assert!(
        issued
            .iter()
            .all(|m| m.envelope().expect("signed fixture envelope").door
                == Some(bbt_proto::messaging::Door::Ctl))
    );
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
async fn first_contact_uses_the_authenticated_hook_socket_and_restores_it_after_logic_restart() {
    use bboxy450_term::cockpit::Event;
    use bbt_mux::store::{Events, Store, events::EventLog, sqlite::SqliteStore};
    use bbt_proto::GuiReply;

    let root = tempfile::tempdir().unwrap();
    let root = root.path().canonicalize().unwrap();
    let run = root.join("run");
    let repository = root.join("repository");
    let bin = root.join("bin");
    let log = root.join("agent.log");
    let socket = root.join("native.sock");
    let release = root.join("release-hook");
    let client_config = root.join("client-config");
    for dir in [
        repository.join(".bboxy"),
        bin.clone(),
        root.join("zsh-config"),
        client_config.clone(),
    ] {
        std::fs::create_dir_all(dir).unwrap();
    }
    fixture_login(&root, &bin);
    let binary = PathBuf::from(env!("CARGO_BIN_EXE_bboxy450-term"));
    std::fs::write(bin.join("agent.py"), AGENT).unwrap();
    executable(
        &bin.join("claude"),
        &format!(
            "#!/bin/sh\nBBT_BIN={} BBT_AGENT_LOG={} BBT_NATIVE_SOCKET={} BBT_RELEASE_HOOK={} exec python3 -I {} \"$@\"\n",
            quote(&binary),
            quote(&log),
            quote(&socket),
            quote(&release),
            quote(&bin.join("agent.py"))
        ),
    );
    let app = root.join("test-app");
    executable(
        &app,
        &format!(
            "#!/bin/sh\nHOME={} BBOXY_TEST_CUSTODY={} CLAUDE_CONFIG_DIR={} PATH={}:\"$PATH\" exec {} \"$@\"\n",
            quote(&root.join("home")),
            quote(&PathBuf::from(format!(
                "fixture:{}",
                root.join("custody").display()
            ))),
            quote(&client_config),
            quote(&bin),
            quote(&binary)
        ),
    );
    let mut session = Session::start(Config {
        executable: app,
        socket: run.join("mux.sock"),
        run_dir: run.clone(),
        repository: repository.clone(),
        shell_config: Some(root.join("zsh-config")),
        ctl: true,
        test_account: None,
    })
    .await
    .unwrap();
    let spawned = ctl(
        &run,
        &["spawn", "claude", "--cwd", repository.to_str().unwrap()],
    );
    let seat = spawned[0]["seat"].as_str().unwrap().to_owned();
    let object = run.join(format!("seat-{seat}"));
    until("the forged OSC being observed", &log, || {
        EventLog::open(&object)
            .unwrap()
            .after(0, 128)
            .unwrap()
            .iter()
            .any(|event| {
                event.event.subject == format!("seat/{seat}/state")
                    && event.event.payload["source"] == "7502"
            })
            .then_some(())
    });
    let store = SqliteStore::open(&object).unwrap();
    assert!(
        store.get("claude_socket_registration").unwrap().is_none(),
        "OSC must not bind a native endpoint"
    );
    let sent = ctl(&run, &["dm", &seat, "first-contact"]);
    let message = sent[0]["message"].as_str().unwrap().to_owned();
    let stored = until("the missing session-file discovery cause", &log, || {
        ctl(&run, &["receipts", &message]).into_iter().find(|r| {
            r["receipt"]["note"].as_str().is_some_and(|s| {
                s.contains("session-file discovery failed") && s.contains("no mutation attempted")
            })
        })
    });
    assert_eq!(
        stored["receipt"]["evidence"], "recipient-stored",
        "{stored}"
    );
    assert!(
        stored["receipt"]["note"]
            .as_str()
            .unwrap()
            .contains("discovery failed"),
        "{stored}"
    );
    assert!(
        !std::fs::read_to_string(&log)
            .unwrap()
            .contains("\"message\":"),
        "the idle stand-in must not poll MCP"
    );
    std::fs::write(&release, b"").unwrap();
    let reply = until("an automatic native-woken MCP reply", &log, || {
        ctl(&run, &["inbox"])
            .into_iter()
            .find(|m| m["message"]["envelope"]["reply_to"] == message)
    });
    assert_eq!(
        reply["message"]["envelope"]["body"]["text"],
        "pong: first-contact"
    );
    until("application acknowledgement", &log, || {
        ctl(&run, &["receipts", &message])
            .into_iter()
            .any(|r| r["receipt"]["evidence"] == "application-acknowledged")
            .then_some(())
    });
    let registration = store.get("claude_socket_registration").unwrap().unwrap();
    let raw = std::str::from_utf8(&registration).unwrap();
    assert!(!raw.contains("fixture-token-never-forward"));
    let registration: Value = serde_json::from_slice(&registration).unwrap();
    assert_eq!(registration["binding"]["seat"], seat);
    assert_eq!(registration["binding"]["generation"], 1);
    assert_eq!(registration["path"], socket.to_str().unwrap());
    assert!(
        !client_config.join("sessions").exists(),
        "no PID session file is supplied"
    );

    let previous_logic = session.logic.id();
    session.logic.start_kill().unwrap();
    tokio::time::timeout(Duration::from_secs(30), async {
        loop {
            if let Event::Reply(reply) = session.event().await.unwrap()
                && let GuiReply::Seats(seats) = *reply
                && seats.iter().any(|s| s.record.id.to_string() == seat)
                && session.logic.id() != previous_logic
            {
                break;
            }
        }
    })
    .await
    .unwrap();
    let restored = ctl(&run, &["dm", &seat, "RESTORE-ONLY"]);
    let restored = restored[0]["message"].as_str().unwrap();
    until("the restored native transport", &log, || {
        ctl(&run, &["receipts", restored])
            .into_iter()
            .any(|r| r["receipt"]["evidence"] == "native-written")
            .then_some(())
    });
    until(
        "the retained stand-in receiving the restored wake",
        &log,
        || {
            std::fs::read_to_string(&log)
                .ok()
                .filter(|s| s.contains("RESTORE-ONLY"))
                .map(|_| ())
        },
    );
    println!(
        "first-contact: OSC ignored for native binding; authenticated hook wakes an idle stand-in; real MCP list/ack/reply; no PID file or operator input; native binding restored after logic death (transport only)"
    );
    session.shutdown().await.unwrap();
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
        test_account: None,
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
        test_account: None,
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
    async fn answer(
        session: &mut Session,
        error: Option<&str>,
        frame: bbt_proto::gui::CaptureFrame,
    ) -> (String, Option<String>) {
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
                        frame,
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
    let current = bbt_proto::gui::CaptureFrame {
        stale: false,
        occluded: false,
        presented_at_ms: Some(1_792_900_000_000),
        frame_seq: 7,
        ..Default::default()
    };
    let (path, seat) = answer(&mut session, None, current).await;
    assert_eq!((path.as_str(), seat), (out.to_str().unwrap(), None));
    let (ok, lines) = asked.await.unwrap();
    assert!(ok, "{lines:?}");
    assert_eq!(lines[0]["kind"], "captured", "{lines:?}");
    assert_eq!(lines[0]["path"], out.to_str().unwrap());
    assert_eq!(
        (
            &lines[0]["stale"],
            &lines[0]["occluded"],
            &lines[0]["frame_presented_at"]
        ),
        (
            &Value::Bool(false),
            &Value::Bool(false),
            &serde_json::json!(1_792_900_000_000u64)
        ),
        "{lines:?}"
    );
    assert!(std::fs::read(&out).unwrap().starts_with(b"\x89PNG"));

    // An occluded window answers with its last frame, and says it is stale, so a judge
    // never takes it for the current one.
    let asked = tokio::task::spawn_blocking({
        let (run, root) = (run.clone(), root.clone());
        move || ctl_in(&run, &root, &["capture", "occluded.png"])
    });
    let stale = bbt_proto::gui::CaptureFrame {
        stale: true,
        occluded: true,
        presented_at_ms: Some(1_792_800_000_000),
        frame_seq: 7,
        ..Default::default()
    };
    answer(&mut session, None, stale).await;
    let (ok, lines) = asked.await.unwrap();
    assert!(ok, "{lines:?}");
    assert_eq!(lines[0]["stale"], true, "{lines:?}");
    assert_eq!(lines[0]["occluded"], true, "{lines:?}");
    assert_eq!(lines[0]["frame_presented_at"], 1_792_800_000_000u64);
    assert_eq!(lines[0]["frame_seq"], 7);

    // The window's refusal reaches ctl as its error and exit 1.
    let asked = tokio::task::spawn_blocking({
        let (run, root) = (run.clone(), root.clone());
        move || ctl_in(&run, &root, &["capture", "/nonexistent/x.png"])
    });
    answer(
        &mut session,
        Some("the window is not presenting"),
        Default::default(),
    )
    .await;
    let (ok, lines) = asked.await.unwrap();
    assert!(!ok);
    assert_eq!(lines[0]["kind"], "error", "{lines:?}");
    assert_eq!(lines[0]["reason"], "the window is not presenting");

    // An unknown seat is refused by ctl before anything is asked of the window.
    let (ok, lines) = ctl_in(&run, &root, &["capture", "x.png", "--seat", "nobody"]);
    assert!(!ok && lines[0]["kind"] == "error", "{lines:?}");

    // A seat ctl spawns is handed to the window to place on stage, as its own pick is.
    let (ok, lines) = ctl_in(
        &run,
        &root,
        &[
            "spawn",
            "zsh",
            "--placement",
            "new-tab",
            "--cwd",
            root.to_str().unwrap(),
        ],
    );
    assert!(ok, "{lines:?}");
    let spawned = lines[0]["seat"].as_str().unwrap().to_owned();
    loop {
        let event = tokio::time::timeout(Duration::from_secs(20), session.event())
            .await
            .expect("the window hears of the ctl spawn")
            .unwrap();
        if let Event::Reply(reply) = event
            && let GuiReply::OperatorSpawned { seat, spec, .. } = *reply
        {
            assert_eq!(seat.to_string(), spawned);
            assert!(matches!(
                spec.placement,
                Some(bbt_proto::seat_state::Placement::NewTab { .. })
            ));
            break;
        }
    }

    // seat focus is relayed to the window (ctl-cli §1a), which focuses as a row click does.
    let focus = tokio::task::spawn_blocking({
        let (run, root, spawned) = (run.clone(), root.clone(), spawned.clone());
        move || ctl_in(&run, &root, &["seat", "focus", &spawned])
    });
    loop {
        let event = tokio::time::timeout(Duration::from_secs(20), session.event())
            .await
            .expect("the window is asked to focus")
            .unwrap();
        if let Event::Reply(reply) = event
            && let GuiReply::OperatorFocus { op, seat } = *reply
        {
            assert_eq!(seat.to_string(), spawned);
            assert!(
                !focus.is_finished(),
                "focus must wait for the window's frame acknowledgment"
            );
            session
                .send(GuiAction::Focused {
                    id: op,
                    result: Ok(bbt_proto::gui::FocusFrame {
                        seat,
                        generation: 1,
                        frame_seq: 7,
                    }),
                })
                .await
                .unwrap();
            break;
        }
    }
    let (ok, lines) = focus.await.unwrap();
    assert!(ok && lines[0]["verb"] == "focus", "{lines:?}");
    for undo in [false, true] {
        let close = tokio::task::spawn_blocking({
            let (run, root, spawned) = (run.clone(), root.clone(), spawned.clone());
            move || {
                ctl_in(
                    &run,
                    &root,
                    &["seat", if undo { "undo-close" } else { "close" }, &spawned],
                )
            }
        });
        loop {
            let event = tokio::time::timeout(Duration::from_secs(20), session.event())
                .await
                .expect("window receives T7 relay")
                .unwrap();
            if let Event::Reply(reply) = event
                && let notice
                @ (GuiReply::OperatorClose { .. } | GuiReply::OperatorUndoClose { .. }) = *reply
            {
                assert_eq!(matches!(notice, GuiReply::OperatorUndoClose { .. }), undo);
                let (op, seat) = match notice {
                    GuiReply::OperatorClose { op, seat }
                    | GuiReply::OperatorUndoClose { op, seat } => (op, seat),
                    _ => unreachable!(),
                };
                assert_eq!(seat.to_string(), spawned);
                assert!(
                    !close.is_finished(),
                    "close must wait for the window's produced frame"
                );
                let frame_seq = if undo { 9 } else { 8 };
                session
                    .send(GuiAction::UiReply {
                        id: op,
                        result: Box::new(Ok(bbt_proto::gui::UiLayout {
                            regions: Default::default(),
                            swarm_workdirs: Default::default(),
                            sidebar_open: Default::default(),
                            clock: None,
                            frame_seq,
                            layout_revision: None,
                            spawned: None,
                            composed: None,
                            starting: Vec::new(),
                            seats: None,
                            size: [1600., 900.],
                            swarm: None,
                            tab: None,
                            focused: None,
                            launcher_open: false,
                            launcher_params: None,
                            page: None,
                            document: None,
                            panes: Vec::new(),
                            tree: Vec::new(),
                        })),
                    })
                    .await
                    .unwrap();
                break;
            }
        }
        let (ok, lines) = close.await.unwrap();
        assert!(ok && lines[0]["data"]["relayed"] == true, "{lines:?}");
        assert_eq!(lines[0]["data"]["applied"], true);
        assert_eq!(lines[0]["data"]["frame_seq"], if undo { 9 } else { 8 });
    }

    // SIGTERM to logic closes the door: gui.endpoint is removed (invariant #261). The
    // restarted logic writes a fresh one, with a new capability.
    let before = std::fs::read_to_string(&endpoint).unwrap();
    let logic = session.logic.id().unwrap().to_string();
    assert!(
        Command::new("kill")
            .args(["-TERM", &logic])
            .status()
            .unwrap()
            .success()
    );
    loop {
        let event = tokio::time::timeout(Duration::from_secs(20), session.event())
            .await
            .expect("logic exits on SIGTERM")
            .unwrap();
        if let Event::Exited {
            process: "logic", ..
        } = event
        {
            break;
        }
    }
    assert!(!endpoint.exists(), "SIGTERM removes gui.endpoint");
    let deadline = Instant::now() + Duration::from_secs(30);
    while !endpoint.exists() {
        assert!(
            Instant::now() < deadline,
            "the restarted logic reopens the door"
        );
        let _ = tokio::time::timeout(Duration::from_millis(250), session.event()).await;
    }
    let after = until("the new endpoint", &root.join("none"), || {
        std::fs::read_to_string(&endpoint).ok()
    });
    assert_ne!(before, after, "a restart writes a fresh endpoint");

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

#[tokio::test]
async fn real_ctl_keeps_a_refusal_that_interrupts_its_request_write() {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

    struct ResumeOnDrop(Option<libc::pid_t>);
    impl Drop for ResumeOnDrop {
        fn drop(&mut self) {
            if let Some(pid) = self.0 {
                // SAFETY: this is only the fixture's stopped, unreaped child.
                unsafe { libc::kill(pid, libc::SIGCONT) };
            }
        }
    }

    tokio::time::timeout(Duration::from_secs(10), async {
        let root = tempfile::Builder::new()
            .prefix("ctl-refusal-")
            .tempdir_in("/tmp")
            .unwrap();
        let path = root.path().join("door.sock");
        let listener = tokio::net::UnixListener::bind(&path).unwrap();
        let endpoint = root.path().join("endpoint.json");
        std::fs::write(
            &endpoint,
            json!({"path":path,"capability":"00".repeat(32)}).to_string(),
        )
        .unwrap();
        std::fs::set_permissions(&endpoint, std::fs::Permissions::from_mode(0o600)).unwrap();

        let child = tokio::process::Command::new(env!("CARGO_BIN_EXE_bboxy450-term"))
            .args(["ctl", "--endpoint"])
            .arg(&endpoint)
            .args(["--as", "refusal fixture", "seat", "ls"])
            .env_remove("BBOXY_CTL_ISSUER")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        let pid = child.id().unwrap() as libc::pid_t;
        let (stream, _) = listener.accept().await.unwrap();
        let mut stream = BufReader::new(stream);
        let mut line = String::new();
        stream.read_line(&mut line).await.unwrap();
        let request: bbt_proto::GuiRequest = serde_json::from_str(&line).unwrap();
        assert!(
            matches!(request.action, GuiAction::Issuer { label } if label == "refusal fixture")
        );

        // Issuer must be acknowledged before ctl can send ListSeats. Stop our
        // own child at that protocol barrier, close the peer, then resume it:
        // the next request is guaranteed to encounter an already-closed peer.
        // SAFETY: pid came directly from our still-owned, unreaped Child.
        assert_eq!(unsafe { libc::kill(pid, libc::SIGSTOP) }, 0);
        let mut resume = ResumeOnDrop(Some(pid));
        loop {
            let mut status = 0;
            // SAFETY: status is writable and waitpid targets only our child.
            let result =
                unsafe { libc::waitpid(pid, &mut status, libc::WUNTRACED | libc::WNOHANG) };
            if result == pid {
                assert!(
                    libc::WIFSTOPPED(status),
                    "ctl exited before the barrier: {status}"
                );
                break;
            }
            if result < 0 {
                let error = std::io::Error::last_os_error();
                assert_eq!(error.kind(), std::io::ErrorKind::Interrupted, "{error}");
            }
            tokio::task::yield_now().await;
        }
        let mut replies = Vec::new();
        for reply in [
            GuiReply::Ok {
                op: bbt_mux::Ulid::nil(),
            },
            GuiReply::Error {
                op: bbt_mux::Ulid::nil(),
                reason: bbt_mux::ctl_door::REFUSAL.into(),
            },
        ] {
            serde_json::to_writer(&mut replies, &reply).unwrap();
            replies.push(b'\n');
        }
        stream.write_all(&replies).await.unwrap();
        stream.flush().await.unwrap();
        stream.shutdown().await.unwrap();
        drop(stream);
        // SAFETY: the fixture still owns this stopped, unreaped child.
        assert_eq!(unsafe { libc::kill(pid, libc::SIGCONT) }, 0);
        resume.0 = None;
        let output = child.wait_with_output().await.unwrap();
        assert_eq!(
            output.status.code(),
            Some(1),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let reply: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(reply["error"]["kind"], "ancestry_refused", "{reply}");
        assert_eq!(reply["error"]["message"], bbt_mux::ctl_door::REFUSAL);
        assert!(!String::from_utf8_lossy(&output.stdout).contains("Broken pipe"));
        assert!(!String::from_utf8_lossy(&output.stderr).contains("Broken pipe"));
    })
    .await
    .expect("real ctl must finish with the named refusal, not a write error");
}

/// A wrong capability is refused with the reason, which ctl prints, not a closed socket.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_wrong_capability_is_refused_with_its_reason() {
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
        ctl: true,
        test_account: None,
    })
    .await
    .unwrap();
    let endpoint = run.join("gui.endpoint");
    until("gui.endpoint", &root.join("none"), || {
        endpoint.exists().then_some(())
    });
    let mut forged: Value = serde_json::from_slice(&std::fs::read(&endpoint).unwrap()).unwrap();
    forged["capability"] = Value::String("00".repeat(32));
    let forged_file = root.join("forged.endpoint");
    std::fs::write(&forged_file, forged.to_string()).unwrap();
    let (ok, lines) = ctl_in(
        &run,
        &root,
        &["--endpoint", forged_file.to_str().unwrap(), "seats"],
    );
    assert!(!ok, "{lines:?}");
    assert_eq!(lines[0]["kind"], "error", "{lines:?}");
    assert_eq!(lines[0]["reason"], "GUI socket admission rejected");
    session.shutdown().await.unwrap();
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
    assert!(
        text.contains("seat capture <seat|window> <file.png>"),
        "{text}"
    );
}

/// A real pipe whose reader is gone before the CLI starts: no scheduler race.
fn stdout_with_closed_reader() -> std::process::Stdio {
    use std::os::fd::FromRawFd;
    let mut fds = [-1; 2];
    // SAFETY: pipe initializes two distinct descriptors on success; each is
    // transferred exactly once to an owning File below.
    assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0);
    let reader = unsafe { std::fs::File::from_raw_fd(fds[0]) };
    let writer = unsafe { std::fs::File::from_raw_fd(fds[1]) };
    drop(reader);
    writer.into()
}

fn ctl_stdout(args: &[&str], stdout: std::process::Stdio) -> std::process::Output {
    // These malformed commands may fail before --run-dir can be parsed. Even
    // their default diagnostics destination must belong to this fixture.
    let root = tempfile::tempdir().unwrap();
    Command::new(env!("CARGO_BIN_EXE_bboxy450-term"))
        .arg("ctl")
        .args(args)
        .env("HOME", root.path())
        .env("BBOXY_RUN_DIR", root.path())
        .stdin(std::process::Stdio::null())
        .stdout(stdout)
        .stderr(std::process::Stdio::piped())
        .output()
        .unwrap()
}

// keeps: release #42/task332; Unix filters may close stdout after one match.
#[test]
fn ctl_closed_stdout_help_and_usage_exit_quietly() {
    for args in [&["--help"][..], &["seat", "fly", "x"][..], &["--as"][..]] {
        let open = ctl_stdout(args, std::process::Stdio::piped());
        if args[0] == "--help" {
            assert!(open.status.success());
            assert!(!open.stdout.is_empty());
        } else if args[0] == "seat" {
            assert_eq!(open.status.code(), Some(2));
            let error: Value = serde_json::from_slice(&open.stdout).unwrap();
            assert_eq!(error["ok"], false);
            assert_eq!(error["error"]["kind"], "usage");
        } else {
            assert_eq!(open.status.code(), Some(1));
            let error: Value = serde_json::from_slice(&open.stdout).unwrap();
            assert_eq!(
                error,
                json!({"kind": "error", "reason": "--as needs a value"})
            );
        }
        let closed = ctl_stdout(args, stdout_with_closed_reader());
        assert_eq!(
            closed.status.code(),
            Some(0),
            "{args:?}: {}",
            String::from_utf8_lossy(&closed.stderr)
        );
        assert!(
            closed.stderr.is_empty(),
            "{args:?}: {}",
            String::from_utf8_lossy(&closed.stderr)
        );
    }
}

// keeps: release #42/task332; partial writes must not turn a large reply into a panic.
#[test]
fn ctl_closed_stdout_larger_than_pipe_buffer_exits_quietly() {
    let run = tempfile::tempdir().unwrap();
    let path = "x".repeat(2 * 1024 * 1024);
    // app endpoint reads this public metadata without opening a backend or identity.
    std::fs::write(
        run.path().join("gui.endpoint"),
        json!({"path": path, "generation": 7}).to_string(),
    )
    .unwrap();
    let args = ["--run-dir", run.path().to_str().unwrap(), "app", "endpoint"];
    let open = ctl_stdout(&args, std::process::Stdio::piped());
    assert!(
        open.status.success(),
        "{}",
        String::from_utf8_lossy(&open.stderr)
    );
    assert!(open.stdout.len() > 2 * 1024 * 1024);
    let reply: Value = serde_json::from_slice(&open.stdout).unwrap();
    assert_eq!(reply["ok"], true);
    assert_eq!(reply["data"]["path"], path);
    assert_eq!(reply["data"]["generation"], 7);
    let closed = ctl_stdout(&args, stdout_with_closed_reader());
    assert_eq!(
        closed.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&closed.stderr)
    );
    assert!(
        closed.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&closed.stderr)
    );
}

// keeps: release #42/task332; only a closed reader is a successful output outcome.
#[test]
fn ctl_other_stdout_errors_remain_visible() {
    use std::os::fd::FromRawFd;
    let (mut master, mut slave) = (-1, -1);
    // SAFETY: openpty initializes two distinct descriptors on success; each
    // transfers once to File. No terminal settings or name buffer are supplied.
    assert_eq!(
        unsafe {
            libc::openpty(
                &mut master,
                &mut slave,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        },
        0
    );
    let master = unsafe { std::fs::File::from_raw_fd(master) };
    let slave = unsafe { std::fs::File::from_raw_fd(slave) };
    // A disconnected terminal returns EIO, rather than a pipe's EPIPE.
    drop(master);
    let result = ctl_stdout(&["--help"], slave.into());
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains(&format!("os error {}", libc::EIO)));
}

/// Decision #270: the operator reads and types into a seat from outside the app. A real zsh
/// seat echoes typed text, a named key reaches the PTY as its exact bytes, the screen shows
/// both, a wait that never matches names what the screen showed, and each input is journaled
/// as a ctl operation.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ctl_screen_input_and_wait_drive_a_seat() {
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
        test_account: None,
    })
    .await
    .unwrap();
    let endpoint = run.join("gui.endpoint");
    until("gui.endpoint", &root.join("none"), || {
        endpoint.exists().then_some(())
    });
    let ctl = |args: &[&str]| ctl_in(&run, &root, args);
    let (ok, lines) = ctl(&["spawn", "zsh", "--cwd", root.to_str().unwrap()]);
    assert!(ok, "{lines:?}");
    let seat = lines[0]["seat"].as_str().unwrap().to_owned();

    // Typed text runs and its output shows on the screen (ctl-cli rev. 2 seat verbs, §2 shape).
    let (ok, lines) = ctl(&["seat", "type", &seat, "echo hello-$((40+2))"]);
    assert!(ok, "{lines:?}");
    assert_eq!(
        (&lines[0]["ok"], &lines[0]["noun"], &lines[0]["verb"]),
        (&Value::Bool(true), &json!("seat"), &json!("type")),
        "{lines:?}"
    );
    assert!(ctl(&["seat", "key", &seat, "enter"]).0);
    let (ok, lines) = ctl(&[
        "seat",
        "wait",
        &seat,
        "--text",
        "(?m)^hello-42$",
        "--timeout",
        "20000",
    ]);
    assert!(ok && lines[0]["done"] == true, "{lines:?}");

    // Named keys reach the PTY byte for byte: cat -v shows the up arrow as ^[[A and Ctrl-A
    // as ^A; typed text is literal, a backslash stays a backslash.
    assert!(ctl(&["seat", "type", &seat, "cat -v"]).0);
    assert!(ctl(&["seat", "key", &seat, "enter"]).0);
    std::thread::sleep(Duration::from_millis(500));
    assert!(ctl(&["seat", "key", &seat, "up"]).0);
    assert!(ctl(&["seat", "type", &seat, "x"]).0);
    assert!(ctl(&["seat", "key", &seat, "ctrl-a"]).0);
    assert!(ctl(&["seat", "type", &seat, "y"]).0);
    assert!(ctl(&["seat", "key", &seat, "enter"]).0);
    assert!(ctl(&["seat", "type", &seat, "lit\\e"]).0);
    assert!(ctl(&["seat", "key", &seat, "enter"]).0);
    let (ok, lines) = ctl(&[
        "seat",
        "wait",
        &seat,
        "--text",
        r"(?m)^\^\[\[Ax\^Ay$",
        "--timeout",
        "20000",
    ]);
    assert!(ok, "{lines:?}");
    let (ok, lines) = ctl(&[
        "seat",
        "wait",
        &seat,
        "--text",
        r"(?m)^lit\\e$",
        "--timeout",
        "20000",
    ]);
    assert!(ok, "typed text arrives as written: {lines:?}");
    assert!(ctl(&["seat", "key", &seat, "ctrl-c"]).0);

    // A paste from a file arrives whole at the prompt.
    let pasted = root.join("paste.txt");
    std::fs::write(&pasted, "echo pasted-$((6*7))").unwrap();
    let (ok, lines) = ctl(&["seat", "paste", &seat, pasted.to_str().unwrap()]);
    assert!(ok && lines[0]["verb"] == "paste", "{lines:?}");
    assert!(ctl(&["seat", "key", &seat, "enter"]).0);
    let (ok, lines) = ctl(&[
        "seat",
        "wait",
        &seat,
        "--text",
        "(?m)^pasted-42$",
        "--timeout",
        "20000",
    ]);
    assert!(ok, "{lines:?}");

    // The screen: the data is the window action's reply; --human renders it as text.
    let (ok, lines) = ctl(&["seat", "screen", &seat]);
    assert!(ok, "{lines:?}");
    let data = &lines[0]["data"];
    let rows: Vec<&str> = data["rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r.as_str().unwrap())
        .collect();
    assert!(rows.contains(&"hello-42"), "{rows:?}");
    assert!(data["generation"].as_u64().unwrap() > 0);
    let plain = Command::new(env!("CARGO_BIN_EXE_bboxy450-term"))
        .args(["ctl", "seat", "screen", &seat, "--human"])
        .env("BBOXY_RUN_DIR", &run)
        .output()
        .unwrap();
    let plain = String::from_utf8_lossy(&plain.stdout);
    assert!(plain.lines().any(|l| l == "hello-42"), "{plain}");
    assert!(
        plain.lines().last().unwrap().starts_with("-- cursor "),
        "{plain}"
    );

    // --cells gives each row as runs of equal style; --scrollback replays the capture range
    // into the seat's size and gives its rows, history included (#27).
    let (ok, lines) = ctl(&["seat", "screen", &seat, "--cells"]);
    assert!(ok, "{lines:?}");
    let runs = lines[0]["data"]["cells"].as_array().unwrap();
    assert_eq!(
        runs.len(),
        lines[0]["data"]["rows"].as_array().unwrap().len()
    );
    assert!(
        runs.iter().any(|row| row
            .as_array()
            .unwrap()
            .iter()
            .any(|run| run["text"].as_str().unwrap().contains("hello-42"))),
        "{runs:?}"
    );
    let sequence = lines[0]["data"]["sequence"].as_u64().unwrap();
    assert!(sequence > 0);
    let range = format!("1..{sequence}");
    let (ok, lines) = ctl(&["seat", "screen", &seat, "--scrollback", &range]);
    assert!(ok, "{lines:?}");
    let history: Vec<&str> = lines[0]["data"]["rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r.as_str().unwrap())
        .collect();
    assert!(history.contains(&"hello-42"), "{history:?}");
    let (ok, lines) = ctl(&["seat", "screen", &seat, "--scrollback", "9..1"]);
    assert!(!ok && lines[0]["error"]["kind"] == "usage", "{lines:?}");

    // resize goes through the window's resize path and the seat's screen follows.
    let (ok, lines) = ctl(&["seat", "resize", &seat, "100x30"]);
    assert!(ok && lines[0]["verb"] == "resize", "{lines:?}");
    let resized = until("the seat at 100 columns", &root.join("none"), || {
        let (_, lines) = ctl(&["seat", "screen", &seat]);
        (lines[0]["data"]["cols"] == 100).then_some(lines)
    });
    assert_eq!(resized[0]["data"]["rows"].as_array().unwrap().len(), 30);

    // watch: event lines (state from the seat's retained stream, the screen when its
    // sequence moves), then one done line counting them.
    let watched = Command::new(env!("CARGO_BIN_EXE_bboxy450-term"))
        .args(["ctl", "seat", "watch", &seat, "--timeout", "1500"])
        .env("BBOXY_RUN_DIR", &run)
        .output()
        .unwrap();
    assert_eq!(watched.status.code(), Some(0));
    let events: Vec<Value> = String::from_utf8_lossy(&watched.stdout)
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    let done = events.last().unwrap();
    assert_eq!(done["done"], true, "{events:?}");
    assert_eq!(
        done["data"]["events"].as_u64().unwrap() as usize,
        events.len() - 1
    );
    assert!(
        events.iter().any(|e| e["event"]["kind"] == "screen"
            && e["event"]["rows"]
                .as_array()
                .unwrap()
                .iter()
                .any(|r| r == "hello-42")),
        "{events:?}"
    );
    assert!(
        events.iter().any(|e| e["event"]["kind"] == "state"),
        "{events:?}"
    );

    // rename writes the registry name; the seat list shows it; a blank name is refused.
    let (ok, lines) = ctl(&["seat", "rename", &seat, "night-shell"]);
    assert!(ok && lines[0]["data"]["name"] == "night-shell", "{lines:?}");
    let (code, out, _) = ctl_gui_seats(&mut session, &run, &root).await;
    assert_eq!(code, Some(0), "{out}");
    let lines: Vec<Value> = out
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert!(
        lines[0]["data"]
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s["record"]["name"] == "night-shell"),
        "{lines:?}"
    );
    let (ok, lines) = ctl(&["seat", "rename", &seat, " "]);
    assert!(!ok && lines[0]["error"]["kind"] == "refused", "{lines:?}");

    // A wait that never matches is a timeout (exit 3) whose message ends with the screen.
    let timed = Command::new(env!("CARGO_BIN_EXE_bboxy450-term"))
        .args([
            "ctl",
            "seat",
            "wait",
            &seat,
            "--text",
            "never-there",
            "--timeout",
            "800",
        ])
        .env("BBOXY_RUN_DIR", &run)
        .output()
        .unwrap();
    assert_eq!(timed.status.code(), Some(3));
    let timed: Value = serde_json::from_slice(&timed.stdout).unwrap();
    assert_eq!(timed["ok"], false);
    assert_eq!(timed["error"]["kind"], "timeout");
    assert!(
        timed["error"]["message"]
            .as_str()
            .unwrap()
            .contains("hello-42"),
        "{timed}"
    );
    assert!(timed["error"]["hint"].is_string());
    let (ok, lines) = ctl(&["seat", "wait", &seat, "--state", "sideways"]);
    assert!(!ok && lines[0]["error"]["kind"] == "usage", "{lines:?}");

    // Each ctl input is journaled under the operator principal through the ctl door.
    let account = bbt_mux::store::sqlite::SqliteStore::open(
        &bbt_mux::store::object_dir(&run, "account").unwrap(),
    )
    .unwrap();
    let inputs: Vec<Value> = account
        .connection()
        .unwrap()
        .prepare("SELECT value FROM kv WHERE key LIKE 'control/op/%'")
        .unwrap()
        .query_map([], |row| row.get::<_, Vec<u8>>(0))
        .unwrap()
        .map(|v| serde_json::from_slice::<Value>(&v.unwrap()).unwrap())
        .filter(|entry| entry.to_string().contains("gui_input"))
        .collect();
    assert!(inputs.len() >= 13, "{inputs:?}");
    assert!(
        inputs.iter().all(|e| e["actor"]["door"] == "ctl"),
        "{inputs:?}"
    );
    session.shutdown().await.unwrap();
}

/// The Codex hook-review recipe's expected list (#270): generated from the launch overlay.
#[test]
fn hook_list_names_the_codex_overlay_from_launch_rs() {
    let output = Command::new(env!("CARGO_BIN_EXE_bboxy450-term"))
        .args(["hook", "--list", "codex"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let lines: Vec<Value> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    let events: Vec<&str> = lines.iter().map(|l| l["event"].as_str().unwrap()).collect();
    assert_eq!(events, bbt_mux::adapters::launch::CODEX_HOOK_EVENTS);
    assert!(lines.iter().all(|l| {
        l["command"]
            .as_str()
            .unwrap()
            .ends_with("bboxy450-term' hook --agent codex")
    }));
}

/// Starts a --ctl fixture app with a zsh seat; returns its session, run dir, root and seat.
async fn noun_verb_fixture(root: &Path) -> (Session, PathBuf, String) {
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
        ctl: true,
        test_account: None,
    })
    .await
    .unwrap();
    let endpoint = run.join("gui.endpoint");
    until("gui.endpoint", &root.join("none"), || {
        endpoint.exists().then_some(())
    });
    let (ok, lines) = ctl_in(
        &run,
        root,
        &["seat", "spawn", "zsh", "--cwd", root.to_str().unwrap()],
    );
    assert!(ok, "{lines:?}");
    let seat = lines[0]["data"]["seat"].as_str().unwrap().to_owned();
    (session, run, seat)
}

/// Runs `ctl ARGS` and returns the exit code, stdout and stderr.
fn ctl_raw(run: &Path, cwd: &Path, args: &[&str]) -> (Option<i32>, String, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_bboxy450-term"))
        .arg("ctl")
        .args(args)
        .env("BBOXY_RUN_DIR", run)
        .current_dir(cwd)
        .output()
        .unwrap();
    (
        output.status.code(),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

/// A bare Session has no rendering host. Answer this one GUI metadata request
/// with its real public rows, without inventing a rendered-frame receipt.
async fn ctl_gui_seats(
    session: &mut Session,
    run: &Path,
    cwd: &Path,
) -> (Option<i32>, String, String) {
    // As at real GUI bootstrap, the first request authenticates the owner
    // before the server forwards any operator asks to this connection.
    let seats = session.list().await.unwrap();
    let (run, cwd) = (run.to_path_buf(), cwd.to_path_buf());
    let mut cli = tokio::task::spawn_blocking(move || ctl_raw(&run, &cwd, &["seat", "ls"]));
    loop {
        let event = tokio::select! {
            result = &mut cli => return result.unwrap(),
            event = tokio::time::timeout(Duration::from_secs(20), session.event()) =>
                event.expect("GUI metadata request watchdog").unwrap(),
        };
        if let Event::Reply(reply) = event
            && let GuiReply::UiAsk {
                op,
                request: bbt_proto::gui::UiRequest::Seats,
            } = *reply
        {
            session
                .send(GuiAction::UiReply {
                    id: op,
                    result: Box::new(Ok(bbt_proto::gui::UiLayout {
                        regions: Default::default(),
                        swarm_workdirs: Default::default(),
                        sidebar_open: Default::default(),
                        frame_seq: 0,
                        layout_revision: None,
                        clock: None,
                        size: [0., 0.],
                        swarm: None,
                        tab: None,
                        focused: None,
                        launcher_open: false,
                        launcher_params: None,
                        page: None,
                        document: None,
                        panes: Vec::new(),
                        tree: Vec::new(),
                        spawned: None,
                        composed: None,
                        starting: Vec::new(),
                        seats: Some(seats),
                    })),
                })
                .await
                .unwrap();
            return cli.await.unwrap();
        }
    }
}

/// ctl-cli rev. 2, C-0: the noun-verb grammar answers in the §2 shape, fails in the §3 shape
/// with a hint and the named exit code, honours --op, and the flat names still work.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ctl_noun_verb_grammar_replies_and_errors() {
    let root = tempfile::tempdir().unwrap();
    let root = root.path().canonicalize().unwrap();
    let (mut session, run, seat) = noun_verb_fixture(&root).await;
    let json =
        |text: &str| -> Value { serde_json::from_str(text.lines().next().unwrap()).unwrap() };

    let (code, out, _) = ctl_gui_seats(&mut session, &run, &root).await;
    let reply = json(&out);
    assert_eq!(code, Some(0), "{out}");
    assert_eq!(
        (&reply["ok"], &reply["noun"], &reply["verb"]),
        (&json!(true), &json!("seat"), &json!("ls"))
    );
    assert!(reply["op"].is_string());
    assert_eq!(reply["data"][0]["record"]["id"], seat.as_str(), "{reply}");

    // app endpoint: the run directory, socket path and generation, never the capability.
    let (code, out, _) = ctl_raw(&run, &root, &["app", "endpoint"]);
    let reply = json(&out);
    assert_eq!(code, Some(0));
    let keys: Vec<&String> = reply["data"].as_object().unwrap().keys().collect();
    assert_eq!(keys, ["generation", "path", "run_dir"], "{reply}");

    // message send to a seat by id; --op makes a retry the same message.
    let op = bbt_mux::Ulid::new().to_string();
    let first = json(
        &ctl_raw(
            &run,
            &root,
            &["message", "send", &seat, "hello", "--op", &op],
        )
        .1,
    );
    assert_eq!(first["op"], op.as_str());
    assert_eq!(first["data"]["kind"], "stored", "{first}");
    let again = json(
        &ctl_raw(
            &run,
            &root,
            &["message", "send", &seat, "hello", "--op", &op],
        )
        .1,
    );
    assert_eq!(
        again["data"]["message"]["envelope"]["id"],
        first["data"]["message"]["envelope"]["id"]
    );
    let (code, out, _) = ctl_raw(&run, &root, &["message", "receipts", &op]);
    assert_eq!(code, Some(0), "{out}");
    assert!(json(&out)["data"].is_array());
    let (code, out, _) = ctl_raw(&run, &root, &["task", "list"]);
    assert_eq!(code, Some(0), "{out}");
    assert!(json(&out)["data"].is_array());

    // §3: usage (2), refused (1), endpoint unavailable (4), each with a kind and a hint.
    for (args, kind, exit) in [
        (vec!["seat", "fly", "x"], "usage", 2),
        (vec!["seat", "ls", "--bogus", "1"], "usage", 2),
        (
            vec!["message", "ack", "01M5Q8A7DQ7EQFKBZH4NXBW3QQ"],
            "refused",
            1,
        ),
        (vec!["seat", "screen", "nobody"], "refused", 1),
        (
            vec!["--run-dir", "/nonexistent-run", "seat", "ls"],
            "endpoint_unavailable",
            4,
        ),
    ] {
        let (code, out, _) = ctl_raw(&run, &root, &args);
        let reply = json(&out);
        assert_eq!(code, Some(exit), "{args:?}: {out}");
        assert_eq!(reply["ok"], false, "{args:?}");
        assert_eq!(reply["error"]["kind"], kind, "{args:?}: {reply}");
        assert!(
            !reply["error"]["hint"].as_str().unwrap().is_empty(),
            "{args:?}: {reply}"
        );
    }

    // A flat name still answers in its old shape, and says what replaces it.
    let (code, out, err) = ctl_raw(&run, &root, &["seats"]);
    assert_eq!(code, Some(0));
    assert_eq!(json(&out)["kind"], "seat");
    assert!(err.contains("deprecated, use: ctl seat ls"), "{err}");
    session.shutdown().await.unwrap();
}

/// Invariant #275: no ctl output, reply, error or hint carries the GUI capability, in any
/// encoding, for any verb, flat or noun-verb, success or failure.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ctl_never_prints_secrets() {
    let root = tempfile::tempdir().unwrap();
    let root = root.path().canonicalize().unwrap();
    let (session, run, seat) = noun_verb_fixture(&root).await;
    let endpoint: Value =
        serde_json::from_slice(&std::fs::read(run.join("gui.endpoint")).unwrap()).unwrap();
    let hex = endpoint["capability"].as_str().unwrap().to_owned();
    let bytes: Vec<u8> = (0..32)
        .map(|i| u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).unwrap())
        .collect();
    let b64 = |alphabet: &[u8], pad: bool| {
        let mut out = String::new();
        for chunk in bytes.chunks(3) {
            let n = chunk
                .iter()
                .enumerate()
                .fold(0u32, |n, (i, b)| n | (u32::from(*b) << (16 - 8 * i)));
            for i in 0..=chunk.len() {
                out.push(alphabet[((n >> (18 - 6 * i)) & 63) as usize] as char);
            }
            if pad {
                out.push_str(&"=".repeat(3 - chunk.len()));
            }
        }
        out
    };
    let standard = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let url = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut secrets = vec![
        hex.clone(),
        hex.to_uppercase(),
        b64(standard, true),
        b64(standard, false),
        b64(url, true),
        b64(url, false),
        serde_json::to_string(&bytes).unwrap(),
    ];
    // Throwaway key material, deliberately placed both in the selected journal
    // result and in a neighboring identity KV row that debug must never read.
    let fixture_key = "e7".repeat(32);
    secrets.extend([
        fixture_key.clone(),
        serde_json::to_string(&vec![231u8; 32]).unwrap(),
    ]);
    use bbt_mux::store::{Store, sqlite::SqliteStore};
    let store = SqliteStore::open(&run.join("account")).unwrap();
    let retained = bbt_mux::Ulid::new();
    let journal_entry = serde_json::to_vec(
        &json!({"actor":{"device_id":bbt_mux::Ulid::new(),"door":"ctl"},
        "tool":"fixture_debug","args_digest":"fixture-digest",
        "result":{"private_key":fixture_key,"capability":bytes,"text":secrets.join(" ")}}),
    )
    .unwrap();
    store
        .put(&format!("control/op/{retained}"), &journal_entry)
        .unwrap();
    store
        .put("identity/private-key-fixture", fixture_key.as_bytes())
        .unwrap();
    let journal_query = format!("account/{retained}");
    std::fs::write(
        run.join("app.log"),
        format!("fixture log\n{}\n", secrets.join("\n")),
    )
    .unwrap();
    let snapshot = root.join("debug-snapshot.json");
    let pasted = root.join("paste.txt");
    std::fs::write(&pasted, "echo pasted").unwrap();
    let message = bbt_mux::Ulid::new().to_string();
    let paste = pasted.to_str().unwrap();
    let cwd = root.to_str().unwrap();
    let runs: Vec<Vec<&str>> = vec![
        vec!["app", "endpoint"],
        vec!["app", "endpoint", "--human"],
        vec!["seat", "ls"],
        vec!["seat", "ls", "--human"],
        vec![
            "seat",
            "spawn",
            "zsh",
            "--cwd",
            cwd,
            "--as",
            "Fixture, Test",
        ],
        vec!["seat", "screen", &seat],
        vec!["seat", "screen", &seat, "--human"],
        vec!["seat", "screen", &seat, "--cells"],
        vec!["seat", "screen", &seat, "--scrollback", "1..4000"],
        vec!["seat", "watch", &seat, "--timeout", "500"],
        vec!["seat", "resize", &seat, "90x25"],
        vec!["seat", "rename", &seat, "renamed-in-secrets-test"],
        vec!["seat", "focus", &seat],
        vec!["seat", "type", &seat, "echo x"],
        vec!["seat", "key", &seat, "enter"],
        vec!["seat", "paste", &seat, paste],
        vec!["seat", "wait", &seat, "--text", "x", "--timeout", "5000"],
        vec![
            "seat",
            "wait",
            &seat,
            "--text",
            "never-there",
            "--timeout",
            "300",
        ],
        vec!["message", "send", &seat, "hello", "--op", &message],
        vec!["message", "post", "swarms/nowhere", "hi"],
        vec!["message", "inbox"],
        vec!["message", "receipts", &message],
        vec!["message", "ack", &message],
        vec!["task", "list"],
        vec!["task", "get", "#9999"],
        vec!["task", "delegate", "#9999", &seat],
        vec!["e2e", "list"],
        vec!["seat", "fly"],
        vec!["--run-dir", "/nonexistent-run", "seat", "ls"],
        vec![
            "--endpoint",
            "/nonexistent-run/gui.endpoint",
            "app",
            "endpoint",
        ],
        // The flat names, for their one remaining release.
        vec!["seats"],
        vec!["dm", &seat, "hi"],
        vec!["inbox"],
        vec!["receipts", &message],
        vec!["reports"],
        vec!["e2e"],
        vec!["record", "fetch", "#1"],
        vec!["record", "search", "not-a-fixture-secret"],
        vec!["record", "changes", "--since", "0"],
        vec!["events", "streams"],
        vec!["events", "tail", "--stream", "account", "--since", "0"],
        vec!["debug", "logs", "app"],
        vec!["debug", "logs", "app", "--since", "1"],
        vec!["debug", "logs", "logic"],
        vec!["debug", "logs", "supervisor"],
        vec!["debug", "journal", &journal_query],
        vec!["debug", "journal", "SELECT * FROM kv"],
        vec!["debug", "journal", "identity/private-key-fixture"],
        vec!["debug", "metrics"],
        vec!["debug", "notifications"],
        vec!["debug", "snapshot", snapshot.to_str().unwrap()],
        vec!["debug", "ps"],
        vec!["--help"],
    ];
    for args in runs {
        let (_, out, err) = ctl_raw(&run, &root, &args);
        for secret in &secrets {
            assert!(
                !out.contains(secret.as_str()) && !err.contains(secret.as_str()),
                "ctl {args:?} printed the capability ({secret}): {out}{err}"
            );
        }
    }
    let snapshot_bytes = std::fs::read_to_string(snapshot).unwrap();
    for secret in &secrets {
        assert!(!snapshot_bytes.contains(secret));
    }
    assert_eq!(
        store.get(&format!("control/op/{retained}")).unwrap(),
        Some(journal_entry)
    );
    assert_eq!(
        store.get("identity/private-key-fixture").unwrap(),
        Some(fixture_key.into_bytes())
    );
    session.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn debug_reads_real_process_logs_journal_cursors_and_disabled_capture_without_a_window() {
    let root = tempfile::tempdir().unwrap();
    let root = root.path().canonicalize().unwrap();
    let (session, run, seat) = noun_verb_fixture(&root).await;
    let supervisor = session.supervisor.id().unwrap();
    let logic = session.logic.id().unwrap();
    let (code, out, err) = ctl_raw(&run, &root, &["debug", "ps"]);
    assert_eq!(code, Some(0), "{out}{err}");
    let reply: Value = serde_json::from_str(out.trim()).unwrap();
    let processes = reply["data"]["processes"].as_array().unwrap();
    assert!(processes.iter().any(|p| p["pid"] == supervisor));
    assert!(processes.iter().any(|p| p["pid"] == logic));
    assert!(processes.iter().any(|p| p["role"] == "seat"));
    assert!(
        processes
            .iter()
            .all(|p| p.get("argv").is_none() && p.get("env").is_none())
    );
    let read = |args: &[&str]| {
        let (code, out, err) = ctl_raw(&run, &root, args);
        assert_eq!(code, Some(0), "{args:?}: {out}{err}");
        serde_json::from_str::<Value>(out.trim()).unwrap()["data"].clone()
    };
    let operation = bbt_mux::Ulid::new().to_string();
    let metrics = read(&[
        "debug",
        "metrics",
        "--op",
        &operation,
        "--as",
        "Debug Fixture",
    ]);
    assert_eq!(metrics["views"], 1);
    assert_eq!(metrics["capture"]["state"], "disabled");
    assert_eq!(metrics, read(&["debug", "metrics", "--op", &operation]));
    let (code, out, _) = ctl_raw(&run, &root, &["debug", "ps", "--op", &operation]);
    assert_eq!(code, Some(1));
    assert!(out.contains("op_id_reused"));
    let journal = read(&["debug", "journal", &format!("account/{operation}")]);
    assert_eq!(journal["mode"], "ro");
    assert_eq!(journal["query_only"], true);
    assert_eq!(journal["entry"]["actor"]["door"], "ctl");
    assert_eq!(journal["entry"]["actor"]["issued_by"], "Debug Fixture");
    assert_eq!(journal["entry"]["tool"], "debug_read");
    std::fs::write(run.join("app.log"), "first\n").unwrap();
    let first = read(&["debug", "logs", "app"]);
    assert_eq!(first["lines"], json!(["first\n"]));
    use std::io::Write;
    std::fs::OpenOptions::new()
        .append(true)
        .open(run.join("app.log"))
        .unwrap()
        .write_all(b"second\n")
        .unwrap();
    let second = read(&[
        "debug",
        "logs",
        "app",
        "--since",
        &first["next_since"].to_string(),
    ]);
    assert_eq!(second["lines"], json!(["second\n"]));
    for source in ["logic", "supervisor"] {
        assert_eq!(read(&["debug", "logs", source])["available"], true);
    }
    let destination = root.join("snapshot.json");
    let snapshot_op = bbt_mux::Ulid::new().to_string();
    let snapshot_args = [
        "debug",
        "snapshot",
        destination.to_str().unwrap(),
        "--op",
        &snapshot_op,
    ];
    let saved = read(&snapshot_args);
    assert_eq!(saved, read(&snapshot_args));
    let table: Value = serde_json::from_slice(&std::fs::read(&destination).unwrap()).unwrap();
    assert_eq!(table["seats"][0]["seat"], seat);
    assert_eq!(table["capture_at_debug"]["logic"]["state"], "disabled");
    assert!(table["seats"][0]["supervisor_ring_tail"].is_null());
    assert_eq!(
        std::fs::metadata(destination).unwrap().permissions().mode() & 0o777,
        0o600
    );
    session.shutdown().await.unwrap();
}

/// `record write` (e2e acceptance §2): the operator signs an e2e record into the knowledge
/// base; another record type and a stale etag are refused with their reasons.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ctl_record_write_signs_e2e_records_only() {
    let root = tempfile::tempdir().unwrap();
    let root = root.path().canonicalize().unwrap();
    let (session, run, _) = noun_verb_fixture(&root).await;
    let json =
        |text: &str| -> Value { serde_json::from_str(text.lines().next().unwrap()).unwrap() };
    let write = |name: &str, args: Value| {
        let file = root.join(name);
        std::fs::write(&file, args.to_string()).unwrap();
        let (code, out, _) = ctl_raw(&run, &root, &["record", "write", file.to_str().unwrap()]);
        (code, json(&out))
    };

    let (code, reply) = write(
        "e2e.json",
        json!({ "path": "e2e/T9.md", "body": "fixture\n",
            "frontmatter": { "type": "e2e", "title": "T9 fixture", "function": "T9",
                "group": "terminal", "status": "untested" } }),
    );
    assert_eq!(code, Some(0), "{reply}");
    assert_eq!(
        (&reply["noun"], &reply["verb"]),
        (&json!("record"), &json!("write"))
    );
    let record = &reply["data"]["record"];
    assert!(record["number"].is_u64(), "{reply}");
    let text = std::fs::read_to_string(root.join("repository/.bboxy/e2e/T9.md")).unwrap();
    assert!(
        text.contains("status: untested") && text.contains("sig:"),
        "{text}"
    );

    let (code, reply) = write(
        "note.json",
        json!({ "path": "notes/x.md", "body": "x\n",
            "frontmatter": { "type": "note", "title": "x" } }),
    );
    assert_eq!(code, Some(1), "{reply}");
    assert!(
        reply["error"]["message"]
            .as_str()
            .unwrap()
            .contains("knowledge_write_denied"),
        "{reply}"
    );

    let reference = json!({ "id": record["id"], "number": record["number"], "slug": record["slug"],
        "path": record["path"], "digest": record["digest"] });
    let (code, reply) = write(
        "stale.json",
        json!({ "ref": reference, "etag": "stale", "body": "fixture\n",
            "frontmatter": { "type": "e2e", "title": "T9 fixture", "function": "T9",
                "group": "terminal", "status": "pass" } }),
    );
    assert_eq!(code, Some(1), "{reply}");
    assert!(
        reply["error"]["message"]
            .as_str()
            .unwrap()
            .contains("etag_mismatch"),
        "{reply}"
    );
    session.shutdown().await.unwrap();
}

/// `task create` (#266 addendum): an open, unassigned task under tasks/, delegated by the
/// operator device; a create without --verify is a usage error.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ctl_task_create_opens_an_unassigned_operator_task() {
    let root = tempfile::tempdir().unwrap();
    let root = root.path().canonicalize().unwrap();
    let (session, run, _) = noun_verb_fixture(&root).await;
    let (code, out, _) = ctl_raw(
        &run,
        &root,
        &[
            "task",
            "create",
            "Fix the T9 fixture",
            "--verify",
            "true",
            "--brief",
            "why",
        ],
    );
    let reply: Value = serde_json::from_str(out.lines().next().unwrap()).unwrap();
    assert_eq!(code, Some(0), "{reply}");
    let record = &reply["data"]["record"];
    let path = record["path"].as_str().unwrap();
    assert!(
        path.starts_with("tasks/") && path.ends_with("-fix-the-t9-fixture.md"),
        "{path}"
    );
    let front = &record["frontmatter"];
    assert_eq!(
        (&front["status"], &front["assignee"]),
        (&json!("open"), &json!("none"))
    );
    assert!(
        front["delegated_by"]
            .as_str()
            .unwrap()
            .starts_with("operator:"),
        "{front}"
    );
    assert!(record["number"].is_u64(), "{reply}");
    let (code, _, _) = ctl_raw(&run, &root, &["task", "create", "no verify"]);
    assert_eq!(code, Some(2));
    session.shutdown().await.unwrap();
}

/// M4 through the real operator door: report evidence is retained, accept preserves the
/// existing verification refusals, and replaying an operation never repeats its effect.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ctl_task_transitions_preserve_verification_and_operation_identity() {
    let root = tempfile::tempdir().unwrap();
    let root = root.path().canonicalize().unwrap();
    let (session, run, seat) = noun_verb_fixture(&root).await;
    let call = |args: &[&str], exit: i32| -> Value {
        let (code, out, err) = ctl_raw(&run, &root, args);
        let reply: Value = serde_json::from_str(out.trim()).unwrap_or_else(|e| {
            panic!("ctl {args:?}: invalid JSON: {e}; stdout={out}; stderr={err}")
        });
        assert_eq!(code, Some(exit), "ctl {args:?}: {reply}; {err}");
        assert_eq!(reply["ok"], exit == 0, "{reply}");
        reply
    };
    let create = |title: &str| -> String {
        let created = call(&["task", "create", title, "--verify", "true"], 0);
        let number = created["data"]["record"]["number"].as_u64().unwrap();
        let handle = format!("#{number}");
        call(&["task", "delegate", &handle, &seat], 0);
        handle
    };
    let status = |handle: &str| -> Value {
        let reply = call(&["task", "status", handle], 0);
        assert_eq!(
            (&reply["noun"], &reply["verb"]),
            (&json!("task"), &json!("status"))
        );
        reply["data"]["item"].clone()
    };
    let handle = create("M4 verification fixture");
    let initial = status(&handle);
    assert_eq!(initial["metadata"]["status"], "claimed");
    for locator in [
        initial["id"].as_str().unwrap(),
        initial["path"].as_str().unwrap(),
    ] {
        assert_eq!(status(locator), initial);
    }
    for (command, exit, reason) in [
        (
            "different command",
            0,
            "verify_result command does not match task verify",
        ),
        (
            "true",
            9,
            "verify_result exit_code is 9; acceptance requires 0",
        ),
    ] {
        let verification = json!({"command":command,"exit_code":exit,"evidence":[]}).to_string();
        let reported = call(
            &[
                "task",
                "report",
                &handle,
                "--summary",
                "invalid proof",
                "--verify-result",
                &verification,
            ],
            0,
        );
        assert_eq!(reported["data"]["kind"], "applied");
        let before = status(&handle);
        assert_eq!(before["metadata"]["status"], "reported");
        let op = bbt_mux::Ulid::new().to_string();
        let args = ["task", "accept", &handle, "--op", &op];
        let refused = call(&args, 1);
        assert_eq!(refused["op"], op);
        assert_eq!(refused["error"]["kind"], "refused");
        assert_eq!(refused["error"]["message"], format!("rejected: {reason}"));
        assert!(!refused["error"]["hint"].as_str().unwrap().is_empty());
        assert_eq!(call(&args, 1), refused);
        assert_eq!(status(&handle), before, "a refused accept changed the task");
    }

    let verification = json!({"command":"true","exit_code":0,"evidence":[
        {"criterion":"M4","reference":"private fixture: exit 0"}
    ]})
    .to_string();
    let report_op = bbt_mux::Ulid::new().to_string();
    let report = [
        "task",
        "report",
        &handle,
        "--summary",
        "single final proof",
        "--commit",
        "0123456789012345678901234567890123456789",
        "--verify-result",
        &verification,
        "--op",
        &report_op,
    ];
    let reported = call(&report, 0);
    assert_eq!(reported["op"], report_op);
    let before = status(&handle);
    assert_eq!(call(&report, 0), reported);
    assert_eq!(status(&handle), before, "a replay duplicated the report");
    let metadata = &before["metadata"];
    assert_eq!(
        metadata["verify_result"],
        serde_json::from_str::<Value>(&verification).unwrap()
    );
    assert_eq!(metadata["report"]["op_id"], report_op);
    assert!(
        metadata["report"]["reported_by"]
            .as_str()
            .unwrap()
            .starts_with("operator:")
    );
    assert_eq!(
        before["body"]
            .as_str()
            .unwrap()
            .matches("single final proof")
            .count(),
        1
    );
    let conflict = call(
        &[
            "task",
            "report",
            &handle,
            "--summary",
            "changed retry",
            "--op",
            &report_op,
        ],
        1,
    );
    assert_eq!(conflict["error"]["kind"], "op_id_reused");
    assert_eq!(status(&handle), before);

    let accept_op = bbt_mux::Ulid::new().to_string();
    let accept = ["task", "accept", &handle, "--op", &accept_op];
    let accepted = call(&accept, 0);
    assert_eq!(accepted["data"]["kind"], "applied");
    let after = status(&handle);
    assert_eq!(after["metadata"]["status"], "accepted");
    assert_eq!(call(&accept, 0), accepted);
    assert_eq!(status(&handle), after);
    let refused = call(&["task", "accept", &handle], 1);
    assert_eq!(
        refused["error"]["message"],
        "rejected: task must be reported before acceptance"
    );
    assert_eq!(
        status(&handle),
        after,
        "accept must take effect exactly once"
    );

    let rejected_task = create("M4 rejection fixture");
    call(
        &[
            "task",
            "report",
            &rejected_task,
            "--summary",
            "no verification supplied",
        ],
        0,
    );
    assert!(
        status(&rejected_task)["metadata"]
            .get("verify_result")
            .is_none()
    );
    let reject_op = bbt_mux::Ulid::new().to_string();
    let reject = [
        "task",
        "reject",
        &rejected_task,
        "--reason",
        "missing evidence",
        "--op",
        &reject_op,
    ];
    let rejected = call(&reject, 0);
    let after = status(&rejected_task);
    assert_eq!(after["metadata"]["status"], "rejected");
    assert_eq!(after["metadata"]["rejection"]["reason"], "missing evidence");
    assert_eq!(call(&reject, 0), rejected);
    assert_eq!(status(&rejected_task), after);
    session.shutdown().await.unwrap();
}

/// `seat restart`: a live zsh seat's process stops and its next generation starts, the
/// committed screen restored (or its gap marked) and a fresh shell answering.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ctl_seat_restart_starts_the_next_generation() {
    let root = tempfile::tempdir().unwrap();
    let root = root.path().canonicalize().unwrap();
    let (session, run, seat) = noun_verb_fixture(&root).await;
    let json =
        |text: &str| -> Value { serde_json::from_str(text.lines().next().unwrap()).unwrap() };
    let screen = |human: bool| {
        let args: &[&str] = if human {
            &["seat", "screen", &seat, "--human"]
        } else {
            &["seat", "screen", &seat]
        };
        ctl_raw(&run, &root, args).1
    };
    let marker = |text: &str, tag: &str| {
        text.lines()
            .filter_map(|l| l.strip_prefix(&format!("{tag}-")))
            .rfind(|pid| pid.chars().all(|c| c.is_ascii_digit()) && !pid.is_empty())
            .map(str::to_owned)
    };
    for line in [
        vec!["seat", "type", &seat, "echo before-$$"],
        vec!["seat", "key", &seat, "enter"],
    ] {
        assert_eq!(ctl_raw(&run, &root, &line).0, Some(0));
    }
    let (code, out, _) = ctl_raw(
        &run,
        &root,
        &[
            "seat",
            "wait",
            &seat,
            "--text",
            "(?m)^before-[0-9]+$",
            "--timeout",
            "20000",
        ],
    );
    assert_eq!(code, Some(0), "{out}");
    let before = marker(&screen(true), "before").unwrap();
    let generation = json(&screen(false))["data"]["generation"].as_u64().unwrap();

    let (code, out, _) = ctl_raw(&run, &root, &["seat", "restart", &seat]);
    assert_eq!(code, Some(0), "{out}");
    assert_eq!(json(&out)["data"]["from_generation"], generation);
    let deadline = Instant::now() + Duration::from_secs(30);
    while json(&screen(false))["data"]["generation"].as_u64() != Some(generation + 1) {
        assert!(
            Instant::now() < deadline,
            "no generation {}",
            generation + 1
        );
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    let restored = screen(true);
    assert!(
        restored.contains(&format!("before-{before}")) || restored.contains("[gap "),
        "{restored}"
    );
    for line in [
        vec!["seat", "type", &seat, "echo after-$$"],
        vec!["seat", "key", &seat, "enter"],
    ] {
        assert_eq!(ctl_raw(&run, &root, &line).0, Some(0));
    }
    let (code, out, _) = ctl_raw(
        &run,
        &root,
        &[
            "seat",
            "wait",
            &seat,
            "--text",
            "(?m)^after-[0-9]+$",
            "--timeout",
            "20000",
        ],
    );
    assert_eq!(code, Some(0), "{out}");
    let after = marker(&screen(true), "after").unwrap();
    assert_ne!(after, before, "a fresh shell answers");
    session.shutdown().await.unwrap();
}

// keeps: ctl-cli §2 and MA-5 carry the backend's typed refusal, never classify its prose.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ctl_unknown_thread_keeps_the_typed_hint_and_exits_one() {
    let root = tempfile::tempdir().unwrap();
    let root = root.path().canonicalize().unwrap();
    let (session, run, seat) = noun_verb_fixture(&root).await;
    let visible = bbt_mux::Ulid::new().to_string();
    let (code, out, _) = ctl_raw(
        &run,
        &root,
        &[
            "message",
            "send",
            &seat,
            "a visible conversation",
            "--op",
            &visible,
        ],
    );
    assert_eq!(code, Some(0), "{out}");
    let op = bbt_mux::Ulid::new().to_string();
    let unknown = bbt_mux::Ulid::new().to_string();
    let (code, out, _) = ctl_raw(
        &run,
        &root,
        &[
            "message",
            "send",
            &seat,
            "cannot answer that id",
            "--reply-to",
            &unknown,
            "--op",
            &op,
        ],
    );
    let reply: Value = serde_json::from_str(out.trim()).unwrap();
    assert_eq!(code, Some(1), "{reply}");
    assert_eq!(reply["ok"], false);
    assert_eq!(reply["op"], op);
    assert_eq!(reply["error"]["kind"], "unknown_thread");
    assert!(
        reply["error"]["message"]
            .as_str()
            .is_some_and(|message| !message.is_empty())
    );
    let hint = reply["error"]["hint"].as_str().unwrap();
    assert!(hint.contains(&visible), "{reply}");
    assert!(!hint.contains(&unknown), "{reply}");
    session.shutdown().await.unwrap();
}
