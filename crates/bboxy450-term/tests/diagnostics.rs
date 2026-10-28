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
        .env("BBOXY_CAPABILITY", "11".repeat(32))
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
