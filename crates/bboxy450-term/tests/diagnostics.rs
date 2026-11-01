// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
#![allow(
    clippy::disallowed_methods,
    reason = "private CLI and journal fixtures, never UI IO"
)]
use bbt_rt::error_journal::{self, Server};
use serde_json::{Value, json};
use std::os::unix::fs::PermissionsExt;
use std::{
    io::Write,
    path::Path,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

fn capability_file(root: &Path) -> std::path::PathBuf {
    let dir = tempfile::tempdir_in(root).unwrap().keep();
    bbt_mux::adapters::capability::create(&dir, &[0x11; 32]).unwrap()
}
fn command(root: &Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_bboxy450-term"));
    cmd.env_clear()
        .env("HOME", root)
        .env("PATH", "/usr/bin:/bin")
        .env("BBOXY_RUN_DIR", root);
    cmd
}
fn errors(root: &Path, extra: &[&str]) -> Value {
    let output = command(root)
        .args(["ctl", "debug", "errors"])
        .args(extra)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "exit={} stdout={} stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["ok"], true);
    value["data"].clone()
}

fn wait_rows(root: &Path, ready: impl Fn(&[Value]) -> bool) -> Vec<Value> {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let data = error_journal::read(root, None, None, false).unwrap();
        let rows = data["rows"].as_array().unwrap();
        if ready(rows) {
            return rows.clone();
        }
        assert!(
            Instant::now() < deadline,
            "journal did not converge: {data}"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn private_backend_bootstrap_owns_its_journal_before_reporting_failure() {
    let root = tempfile::Builder::new()
        .prefix("bbt-bootstrap-journal-")
        .tempdir_in("/tmp")
        .unwrap();
    let operator = root.path().join("operator-run");
    let private = root.path().join("private-run");
    let servers: Vec<_> = [&operator, &private]
        .into_iter()
        .map(|path| {
            let server = Server::start(path.clone(), |_| {}).unwrap();
            server
                .ready()
                .recv_timeout(Duration::from_secs(30))
                .unwrap()
                .unwrap();
            server
        })
        .collect();
    let config = bboxy450_term::cockpit::Config {
        executable: env!("CARGO_BIN_EXE_bboxy450-term").into(),
        // Force an actual startup error after the private config is decoded.
        socket: private.join("overlong-socket-".repeat(20)),
        run_dir: private.clone(),
        repository: root.path().into(),
        shell_config: None,
        ctl: false,
        test_account: None,
    };
    let mut backend = command(root.path())
        .arg("supervisor")
        .env("BBOXY_RUN_DIR", &operator)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    writeln!(
        backend.stdin.take().unwrap(),
        "{}",
        serde_json::to_string(&config).unwrap()
    )
    .unwrap();
    let result = backend.wait_with_output().unwrap();
    assert!(
        !result.status.success(),
        "overlong private socket unexpectedly started"
    );
    let rows = wait_rows(&private, |rows| !rows.is_empty());
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["source"], "supervisor");
    assert_eq!(rows[0]["operation"], "process_entry");
    assert!(
        !operator.join("diagnostics.jsonl").exists(),
        "private bootstrap leaked to inherited run"
    );
    drop(servers);
}

// Pure probes retain JSON-RPC's method-not-found response, including before
// initialization. Malformed requests and real tool errors remain diagnostics.
#[test]
fn optional_method_probes_do_not_journal_but_tool_errors_do() {
    use std::io::{BufRead, BufReader};
    use std::os::unix::net::UnixListener;
    let root = tempfile::tempdir().unwrap();
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let server = Server::start(root.path().into(), |_| {}).unwrap();
    server
        .ready()
        .recv_timeout(Duration::from_secs(30))
        .unwrap()
        .unwrap();
    let socket = root.path().join("seat.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let owner = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(30)))
            .unwrap();
        let mut line = String::new();
        BufReader::new(&stream).read_line(&mut line).unwrap();
        let call: bbt_mux::seat_socket::Request = serde_json::from_str(&line).unwrap();
        let bbt_mux::seat_socket::Action::Call { call } = call.action else {
            panic!("a pure probe reached the seat transport");
        };
        assert_eq!(call.tool, "knowledge_fetch");
        writeln!(stream, "{}", json!({"isError":true,"content":[{"type":"text","text":"fixture tool failed"}],"structuredContent":{"op_id":call.op_id,"error":{"code":"fixture_failure","message":"fixture tool failed"}}})).unwrap();
    });
    let mut bridge = command(root.path())
        .arg("mcp")
        .env("BBOXY_SOCK", socket)
        .env("BBOXY_CAPABILITY_FILE", capability_file(root.path()))
        .env("BBOXY_SEAT_ID", "01M5WZKA3MZC8BRJ601TFF7FH1")
        .env("BBOXY_GENERATION", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut input = bridge.stdin.take().unwrap();
    let requests = [
        json!({"jsonrpc":"2.0","id":1,"method":"server/discover"}),
        json!({"jsonrpc":"2.0","id":2,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"fixture","version":"1"}}}),
        json!({"jsonrpc":"2.0","id":3,"method":"server/discover"}),
        json!({"jsonrpc":"2.0","id":4,"method":"prompts/list"}),
        json!({"jsonrpc":"2.0","id":5,"method":"future/optional"}),
        json!({"jsonrpc":"2.0","method":"future/notification"}),
        json!({"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"knowledge_fetch","arguments":{"locator":"private-request-sentinel"}}}),
        json!({"jsonrpc":"2.0","id":7,"method":"future/optional","params":[]}),
    ];
    for request in requests {
        writeln!(input, "{request}").unwrap();
    }
    drop(input);
    let output = bridge.wait_with_output().unwrap();
    assert!(output.status.success(), "{output:?}");
    owner.join().unwrap();
    let replies: Vec<Value> = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(replies.len(), 7, "notifications must have no response");
    for id in [1, 3, 4, 5] {
        assert_eq!(replies[id - 1]["id"], id);
        assert_eq!(replies[id - 1]["error"]["code"], -32601);
        assert_eq!(replies[id - 1]["error"]["message"], "Method not found");
    }
    assert_eq!(replies[5]["result"]["isError"], true);
    assert_eq!(replies[6]["error"]["code"], -32602);
    let rows = wait_rows(root.path(), |rows| rows.len() >= 2);
    assert_eq!(rows.len(), 2, "{rows:?}");
    assert!(rows.iter().any(|r| {
        r["operation"]
            .as_str()
            .unwrap()
            .starts_with("knowledge_fetch ")
            && r["message"] == "fixture_failure: fixture tool failed"
    }));
    assert!(
        rows.iter()
            .any(|r| r["message"] == "Params must be an object")
    );
    assert!(
        !serde_json::to_string(&rows)
            .unwrap()
            .contains("private-request-sentinel")
    );
}

// The decoy is an owned stand-in for an operator journal. No test reads or
// writes the actual operator's directory, even when running the broken control.
#[test]
#[ignore = "night: real headless app/provider boundary, run by make test-error-journal"]
fn fixture_provider_and_ctl_status_keep_the_operator_journal_untouched() {
    struct App {
        child: std::process::Child,
        run: std::path::PathBuf,
    }
    impl Drop for App {
        fn drop(&mut self) {
            let _ = command(&self.run).args(["ctl", "app", "stop"]).output();
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
    let root = tempfile::Builder::new()
        .prefix("bbt-journal-guard-")
        .tempdir_in("/tmp")
        .unwrap();
    let operator = root.path().join("operator-run");
    let decoy = Server::start(operator.clone(), |_| {}).unwrap();
    decoy
        .ready()
        .recv_timeout(Duration::from_secs(30))
        .unwrap()
        .unwrap();
    let home = root.path().join("home");
    let account = home.join("account");
    let run = account.join("run");
    let repository = home.join("repository");
    std::fs::create_dir_all(&repository).unwrap();
    let child = command(&home)
        .args(["--headless", "--ctl", "--unattended", "--test-account"])
        .arg(&account)
        .arg("--repository")
        .arg(repository)
        // Recreate a test launched inside a live seat; private CLI custody wins.
        .env("BBOXY_RUN_DIR", &operator)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(std::fs::File::create(root.path().join("app.log")).unwrap())
        .spawn()
        .unwrap();
    let mut app = App {
        child,
        run: run.clone(),
    };
    let deadline = Instant::now() + Duration::from_secs(30);
    while !run.join("gui.endpoint").exists() {
        assert!(
            app.child.try_wait().unwrap().is_none(),
            "{}",
            std::fs::read_to_string(root.path().join("app.log")).unwrap()
        );
        assert!(Instant::now() < deadline, "private app did not start");
        std::thread::sleep(Duration::from_millis(10));
    }
    // Reproduce the redacted app/status category: Endpoint::read rejects an
    // endpoint missing its capability; scrub withholds the entire reason.
    let forged = run.join("broken.endpoint");
    std::fs::write(&forged, r#"{"path":"unused.sock"}"#).unwrap();
    let status = command(&home)
        .args(["ctl", "app", "status", "--endpoint"])
        .arg(&forged)
        .env("BBOXY_RUN_DIR", &operator)
        .output()
        .unwrap();
    assert_eq!(status.status.code(), Some(1), "{status:?}");
    assert!(String::from_utf8_lossy(&status.stdout).contains("missing capability"));
    let rows = wait_rows(&run, |rows| {
        rows.iter().any(|r| r["source"] == "provider")
            && rows.iter().any(|r| r["operation"] == "app/status")
    });
    assert!(
        rows.iter().any(|r| r["source"] == "provider"
            && r["operation"] == "request"
            && r["message"].as_str().unwrap().contains("LinkDown:")),
        "{rows:?}"
    );
    assert!(
        rows.iter().any(|r| r["operation"] == "app/status"
            && r["message"] == "[redacted sensitive diagnostic line]"),
        "{rows:?}"
    );
    drop(app);
    assert!(
        error_journal::read(&operator, None, None, false).unwrap()["rows"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    for name in [
        "diagnostics.jsonl",
        "diagnostics.1.jsonl",
        "diagnostics.2.jsonl",
        ".diagnostics-read",
    ] {
        assert!(
            !operator.join(name).exists(),
            "operator journal touched: {name}"
        );
    }
    drop(decoy);
}

// keeps: #56 — malformed private commands must not report to another run's
// journal, including failures before the endpoint-bearing command can be parsed.
#[test]
fn malformed_cli_keeps_its_explicit_journal_and_default_errors_remain_visible() {
    let root = tempfile::Builder::new()
        .prefix("bbt-diagnostic-routing-")
        .tempdir_in("/tmp")
        .unwrap();
    let fallback = root.path().join("fallback");
    let selected = root.path().join("selected");
    std::fs::create_dir(&fallback).unwrap();
    std::fs::create_dir(&selected).unwrap();
    for path in [&fallback, &selected] {
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).unwrap();
    }
    let servers: Vec<_> = [&fallback, &selected]
        .into_iter()
        .map(|path| {
            let server = Server::start(path.clone(), |_| {}).unwrap();
            server
                .ready()
                .recv_timeout(Duration::from_secs(60))
                .unwrap()
                .unwrap();
            server
        })
        .collect();
    let endpoint = selected.join("gui.endpoint");
    let cases = [
        vec![
            "--human",
            "seat",
            "fly",
            "x",
            "--run-dir",
            selected.to_str().unwrap(),
        ],
        vec!["--run-dir", selected.to_str().unwrap(), "--as"],
        vec!["seat", "fly", "x", "--endpoint", endpoint.to_str().unwrap()],
    ];
    for args in cases {
        let output = command(&fallback).arg("ctl").args(&args).output().unwrap();
        assert!(
            !output.status.success(),
            "{args:?} should remain a CLI refusal"
        );
    }
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        let selected_rows = error_journal::read(&selected, None, None, false).unwrap();
        if selected_rows["rows"].as_array().unwrap().len() == 3 {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "private parse failures were not retained: {selected_rows}"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    let untouched = error_journal::read(&fallback, None, None, false).unwrap();
    assert!(
        untouched["rows"].as_array().unwrap().is_empty(),
        "{untouched}"
    );

    // An actual bad command without a routing override still belongs in the
    // caller's default journal; isolation must never suppress that diagnostic.
    for args in [
        vec!["ctl", "seat", "fly", "x"],
        vec!["ctl", "--as", "--run-dir", "seat", "fly", "x"],
    ] {
        let output = command(&fallback).args(args).output().unwrap();
        assert_eq!(output.status.code(), Some(2));
    }
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        let rows = error_journal::read(&fallback, None, None, false).unwrap();
        if rows["rows"].as_array().unwrap().len() == 2 {
            assert_eq!(rows["rows"][0]["source"], "ctl");
            assert!(
                rows["rows"][0]["message"]
                    .as_str()
                    .unwrap()
                    .contains("seat verbs")
            );
            break;
        }
        assert!(
            Instant::now() < deadline,
            "default diagnostic was suppressed: {rows}"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    drop(servers);
}

// keeps: #56 — actual child CLI failures enter one bounded journal without an app,
// endpoint, account, network or operator; reading it never depends on logic.
#[test]
fn cli_mcp_and_shim_failures_are_retained_redacted_filterable_and_acknowledged_offline() {
    let root = tempfile::tempdir().unwrap();
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let server = Server::start(root.path().into(), |_| {}).unwrap();
    server
        .ready()
        .recv_timeout(Duration::from_secs(60))
        .unwrap()
        .unwrap();
    let seat = "01M5WZKA3MZC8BRJ601TFF7FH1";
    let ctl = command(root.path())
        .args(["ctl", "seat", "ls"])
        .output()
        .unwrap();
    assert_eq!(ctl.status.code(), Some(4));
    let mut mcp = command(root.path())
        .arg("mcp")
        .env("BBOXY_SOCK", root.path().join("not-an-endpoint.sock"))
        .env("BBOXY_CAPABILITY_FILE", capability_file(root.path()))
        .env("BBOXY_SEAT_ID", seat)
        .env("BBOXY_GENERATION", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let request = json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"graph_read","arguments":{"token":"fixture-private-request-never-log"}}});
    writeln!(mcp.stdin.take().unwrap(), "{request}").unwrap();
    let result = mcp.wait_with_output().unwrap();
    assert!(result.status.success());
    let reply: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(reply["error"]["code"], -32002);
    let shim = command(root.path())
        .args(["hook", "--agent", "codex", "{"])
        .env("BBOXY_SEAT_ID", seat)
        .output()
        .unwrap();
    assert!(
        shim.status.success(),
        "a failed best-effort shim must remain quiet-success"
    );
    assert!(shim.stdout.is_empty() && shim.stderr.is_empty());

    let guard = Instant::now() + Duration::from_secs(60);
    let data = loop {
        let data = error_journal::read(root.path(), None, None, false).unwrap();
        if data["rows"].as_array().unwrap().len() == 3 {
            break data;
        }
        assert!(
            Instant::now() < guard,
            "diagnostic sink did not retain all child failures: {data}"
        );
        std::thread::sleep(Duration::from_millis(5));
    };
    let text = data.to_string();
    assert!(!text.contains("fixture-private-request-never-log"));
    assert!(!text.contains(&"11".repeat(32)));
    for source in ["ctl", "mcp", "hook"] {
        assert_eq!(
            data["rows"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|r| r["source"] == source)
                .count(),
            1
        );
    }
    let filtered = errors(root.path(), &["--seat", seat, "--since", "0"]);
    assert_eq!(filtered["rows"].as_array().unwrap().len(), 2);
    assert_eq!(filtered["acknowledged"], false);
    assert_eq!(filtered["unread"], 3);
    let all = errors(root.path(), &[]);
    assert_eq!(all["rows"].as_array().unwrap().len(), 3);
    assert_eq!(all["unread_before"], 3);
    assert_eq!(all["unread"], 0);
    assert_eq!(all["acknowledged"], true);
    assert_eq!(errors(root.path(), &[])["unread_before"], 0);
}
