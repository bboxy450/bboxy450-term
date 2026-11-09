// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! The remote seat seam end to end, unattended: a local "harness" (this test)
//! speaks ACP to `attach`, which reaches a scripted remote ACP agent through
//! a banner-printing carrier, serve and the daemon. Also install into a
//! private HOME and dead-line detection. Kills only daemon pids it recorded.
#![allow(clippy::disallowed_methods)]
use bboxy450_term_remote::client::{Client, Received, Transport};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

/// Answers every JSON-RPC request with its id and the params it saw.
const AGENT: &str = "#!/usr/bin/env python3\nimport sys, json\nfor line in sys.stdin:\n    m = json.loads(line)\n    print(json.dumps({'jsonrpc': '2.0', 'id': m.get('id'), 'result': {'saw': m.get('params')}}))\n    sys.stdout.flush()\n";

struct Scratch(PathBuf);
impl Drop for Scratch {
    fn drop(&mut self) {
        if let Ok(pid) = std::fs::read_to_string(self.0.join("s/daemon.pid")) {
            let _ = Command::new("kill").args(["-CONT", pid.trim()]).status();
            let _ = Command::new("kill")
                .args(["-TERM", "--", &format!("-{}", pid.trim())])
                .status();
        }
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn scratch(tag: &str) -> (Scratch, PathBuf) {
    let dir = PathBuf::from(format!("/tmp/bba{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let exe = |name: &str, body: &str| {
        let p = dir.join(name);
        std::fs::write(&p, body).unwrap();
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o700)).unwrap();
        p
    };
    let agent = exe("agent.py", AGENT);
    unsafe { std::env::set_var("BBOXY450_REMOTE_AGENT", &agent) };
    let bin = Path::new(env!("CARGO_BIN_EXE_bboxy450-term-remote"))
        .parent()
        .unwrap()
        .to_owned();
    exe(
        "carrier.sh",
        &format!(
            "#!/bin/sh\necho 'Last login: yesterday (motd)'\nPATH='{}':$PATH HOME='{}' eval \"$1\"\n",
            bin.display(),
            dir.join("home").display()
        ),
    );
    std::fs::create_dir_all(dir.join("home")).unwrap();
    (Scratch(dir.clone()), dir)
}

#[test]
fn attach_bridges_acp_lines_to_the_remote_harness_and_rewrites_session_new() {
    let (_guard, dir) = scratch("a");
    let remote_cwd = dir.join("remote-app");
    std::fs::create_dir_all(&remote_cwd).unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_bboxy450-term-remote"))
        .args(["attach", "--via"])
        .arg(dir.join("carrier.sh"))
        .arg("--state")
        .arg(dir.join("s"))
        .args(["--swarm", "sw", "--session", "seat1", "--cwd"])
        .arg(&remote_cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let mut lines = BufReader::new(child.stdout.take().unwrap()).lines();
    writeln!(
        stdin,
        r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"protocolVersion":1}}}}"#
    )
    .unwrap();
    writeln!(stdin, r#"{{"jsonrpc":"2.0","id":2,"method":"session/new","params":{{"cwd":"/Users/me/local","mcpServers":[{{"name":"bboxy","command":"/usr/bin/nc","args":["-U","/tmp/local.sock"]}}]}}}}"#).unwrap();
    stdin.flush().unwrap();
    let first: serde_json::Value = serde_json::from_str(&lines.next().unwrap().unwrap()).unwrap();
    assert_eq!(first["id"], 1);
    assert_eq!(first["result"]["saw"]["protocolVersion"], 1);
    let second: serde_json::Value = serde_json::from_str(&lines.next().unwrap().unwrap()).unwrap();
    assert_eq!(second["id"], 2);
    assert_eq!(
        second["result"]["saw"]["cwd"],
        remote_cwd.to_str().unwrap(),
        "remote cwd"
    );
    assert_eq!(
        second["result"]["saw"]["mcpServers"][0]["command"], "bboxy450-term-remote",
        "local socket MCP becomes a relay"
    );
    drop(stdin);
    let status = child.wait().unwrap();
    assert!(
        status.success(),
        "attach exits 0 once every op is applied: {status}"
    );
}

#[test]
fn install_copies_this_build_into_the_remote_local_bin() {
    let (_guard, dir) = scratch("i");
    let via = Transport::Via {
        command: dir.join("carrier.sh").to_string_lossy().into(),
        state: "unused".into(),
    };
    let me = Path::new(env!("CARGO_BIN_EXE_bboxy450-term-remote"));
    let there =
        bboxy450_term_remote::install::install(&via, &[(me, "bboxy450-term-remote")]).unwrap();
    assert!(!there.is_empty());
    let landed = dir.join("home/.local/bin/bboxy450-term-remote");
    assert_eq!(
        std::fs::read(&landed).unwrap(),
        std::fs::read(me).unwrap(),
        "identical bytes"
    );
    assert_eq!(
        std::fs::metadata(&landed).unwrap().permissions().mode() & 0o777,
        0o755
    );
    let out = Command::new(&landed).arg("version").output().unwrap();
    assert!(String::from_utf8_lossy(&out.stdout).starts_with("bboxy450-term-remote "));
}

#[test]
fn a_silent_link_is_declared_down_and_pings_keep_a_live_one_up() {
    let (_guard, dir) = scratch("h");
    unsafe {
        std::env::set_var("BBOXY450_REMOTE_PING_MS", "100");
        std::env::set_var("BBOXY450_REMOTE_DEAD_MS", "1000");
    }
    let via = Transport::Via {
        command: dir.join("carrier.sh").to_string_lossy().into(),
        state: dir.join("s").to_string_lossy().into(),
    };
    let mut c = Client::new(via, "sw");
    c.connect().unwrap();
    // Pings every 100 ms: three dead-line spans pass without a Down.
    let start = std::time::Instant::now();
    while start.elapsed() < Duration::from_secs(3) {
        assert_ne!(
            c.recv_timeout(Duration::from_millis(200)).unwrap(),
            Some(Received::Down)
        );
    }
    // Freeze the daemon (SIGSTOP): no bytes at all, so the line is declared down.
    let pid = std::fs::read_to_string(dir.join("s/daemon.pid")).unwrap();
    Command::new("kill")
        .args(["-STOP", pid.trim()])
        .status()
        .unwrap();
    let start = std::time::Instant::now();
    let down = loop {
        match c.recv_timeout(Duration::from_millis(200)).unwrap() {
            Some(Received::Down) => break start.elapsed(),
            _ if start.elapsed() > Duration::from_secs(10) => panic!("never declared down"),
            _ => {}
        }
    };
    assert!(down < Duration::from_secs(5), "{down:?}");
    Command::new("kill")
        .args(["-CONT", pid.trim()])
        .status()
        .unwrap();
}

#[test]
fn profile_names_this_bridge_as_the_acp_agent() {
    let out = Command::new(env!("CARGO_BIN_EXE_bboxy450-term-remote"))
        .args([
            "profile",
            "--ssh",
            "box",
            "--name",
            "remote-box",
            "--swarm",
            "sw",
            "--cwd",
            "/srv/app",
        ])
        .output()
        .unwrap();
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(
        text.starts_with("---\nname: remote-box\nprovider: acp-agent\n"),
        "{text}"
    );
    assert!(text.contains(r#""attach", "--ssh", "box""#), "{text}");
    assert!(
        text.contains(env!("CARGO_BIN_EXE_bboxy450-term-remote")),
        "absolute agent command"
    );
}

/// The real bboxy450-harness ACP server behind the link, one full turn with its
/// scripted provider (no network, no tokens). `make test-remote-live` builds the
/// harness with the `acp` feature and sets BBOXY450_REMOTE_LIVE_HARNESS and
/// BBOXY450_REMOTE_LIVE_SSE; without them this test reports a skip.
#[test]
fn real_harness_acp_turn_crosses_the_link() {
    let (Ok(harness), Ok(sse)) = (
        std::env::var("BBOXY450_REMOTE_LIVE_HARNESS"),
        std::env::var("BBOXY450_REMOTE_LIVE_SSE"),
    ) else {
        eprintln!("skip: set BBOXY450_REMOTE_LIVE_HARNESS/SSE (make test-remote-live)");
        return;
    };
    let (_guard, dir) = scratch("l");
    std::fs::create_dir_all(dir.join("sess")).unwrap();
    std::fs::create_dir_all(dir.join("app")).unwrap();
    let agent = dir.join("harness.sh");
    std::fs::write(
        &agent,
        format!(
            "#!/bin/sh\nexec '{harness}' --scripted '{sse}' --sessions '{}' acp\n",
            dir.join("sess").display()
        ),
    )
    .unwrap();
    std::fs::set_permissions(&agent, std::fs::Permissions::from_mode(0o700)).unwrap();
    unsafe { std::env::set_var("BBOXY450_REMOTE_AGENT", &agent) };
    let mut child = Command::new(env!("CARGO_BIN_EXE_bboxy450-term-remote"))
        .args(["attach", "--via"])
        .arg(dir.join("carrier.sh"))
        .arg("--state")
        .arg(dir.join("s"))
        .args(["--swarm", "live", "--session", "seat"])
        .arg("--cwd")
        .arg(dir.join("app"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let mut lines = BufReader::new(child.stdout.take().unwrap()).lines();
    let mut until = |id: u64| -> (serde_json::Value, usize) {
        let mut updates = 0;
        loop {
            let m: serde_json::Value =
                serde_json::from_str(&lines.next().expect("line").unwrap()).unwrap();
            if m["id"] == id {
                return (m, updates);
            }
            updates += usize::from(m["method"] == "session/update");
        }
    };
    writeln!(stdin, r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"protocolVersion":1,"clientCapabilities":{{}}}}}}"#).unwrap();
    assert_eq!(
        until(1).0["result"]["agentInfo"]["name"],
        "bboxy450-harness"
    );
    writeln!(stdin, r#"{{"jsonrpc":"2.0","id":2,"method":"session/new","params":{{"cwd":"/nowhere","mcpServers":[]}}}}"#).unwrap();
    let session = until(2).0["result"]["sessionId"]
        .as_str()
        .unwrap()
        .to_owned();
    writeln!(stdin, r#"{{"jsonrpc":"2.0","id":3,"method":"session/prompt","params":{{"sessionId":"{session}","prompt":[{{"type":"text","text":"hello"}}]}}}}"#).unwrap();
    let (done, updates) = until(3);
    assert_eq!(done["result"]["stopReason"], "end_turn");
    assert!(updates > 0, "streamed session/update notifications");
    drop(stdin);
    assert!(child.wait().unwrap().success());
}

/// Remote agent that, on session/new, starts its first MCP server the way a
/// harness would, asks it one question and answers with the reply.
const MCP_AGENT: &str = "#!/usr/bin/env python3\nimport sys, json, subprocess\nfor line in sys.stdin:\n    m = json.loads(line)\n    if m.get('method') == 'session/new':\n        s = m['params']['mcpServers'][0]\n        p = subprocess.Popen([s['command']] + s['args'], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)\n        p.stdin.write(json.dumps({'jsonrpc': '2.0', 'id': 9, 'method': 'tools/list'}) + '\\n'); p.stdin.flush()\n        reply = p.stdout.readline().strip()\n        p.stdin.close(); p.wait(timeout=10)\n        print(json.dumps({'jsonrpc': '2.0', 'id': m['id'], 'result': {'mcp_reply': reply, 'server': s}}))\n    else:\n        print(json.dumps({'jsonrpc': '2.0', 'id': m.get('id'), 'result': {}}))\n    sys.stdout.flush()\n";

/// Stands in for the local `nc -U <bboxy MCP socket>`: answers tools/list.
const LOCAL_MCP: &str = "#!/usr/bin/env python3\nimport sys, json\nfor line in sys.stdin:\n    m = json.loads(line)\n    print(json.dumps({'jsonrpc': '2.0', 'id': m['id'], 'result': {'tools': ['local-bboxy']}}))\n    sys.stdout.flush()\n";

#[test]
fn a_remote_harness_reaches_the_local_mcp_server_through_the_link() {
    let (_guard, dir) = scratch("m");
    let write = |name: &str, body: &str| {
        let p = dir.join(name);
        std::fs::write(&p, body).unwrap();
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o700)).unwrap();
        p
    };
    let agent = write("mcp-agent.py", MCP_AGENT);
    let local = write("local-mcp.py", LOCAL_MCP);
    unsafe { std::env::set_var("BBOXY450_REMOTE_AGENT", &agent) };
    std::fs::create_dir_all(dir.join("app")).unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_bboxy450-term-remote"))
        .args(["attach", "--via"])
        .arg(dir.join("carrier.sh"))
        .arg("--state")
        .arg(dir.join("s"))
        .args(["--swarm", "sw", "--session", "seat", "--cwd"])
        .arg(dir.join("app"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let mut lines = BufReader::new(child.stdout.take().unwrap()).lines();
    let new = serde_json::json!({"jsonrpc":"2.0","id":2,"method":"session/new","params":{"cwd":"/local","mcpServers":[
        {"name":"bboxy","command":local.to_str().unwrap(),"args":["-U","/tmp/not-used.sock"],"env":[]}]}});
    writeln!(stdin, "{new}").unwrap();
    stdin.flush().unwrap();
    let reply: serde_json::Value = serde_json::from_str(&lines.next().unwrap().unwrap()).unwrap();
    assert_eq!(reply["id"], 2);
    assert_eq!(
        reply["result"]["server"]["command"], "bboxy450-term-remote",
        "remote ran the relay"
    );
    let answer: serde_json::Value =
        serde_json::from_str(reply["result"]["mcp_reply"].as_str().unwrap()).unwrap();
    assert_eq!(answer["id"], 9);
    assert_eq!(
        answer["result"]["tools"][0], "local-bboxy",
        "the local MCP server answered across the link"
    );
    drop(stdin);
    assert!(child.wait().unwrap().success());
}
