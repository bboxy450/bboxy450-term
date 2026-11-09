// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! `bboxy450-term-remote attach`: the local seat's ACP agent command. A local
//! bboxy450-harness on an `acp-agent` profile speaks ACP on this process's
//! stdio; each line goes to the seat's remote `bboxy450-harness acp` as one
//! exactly-once op, and each remote line comes back unchanged. Only
//! `session/new` is rewritten: the local `cwd` becomes the remote one, and
//! each local-socket MCP server (`nc -U <sock>`, this machine only) becomes
//! `bboxy450-term-remote mcp` on the remote, relayed back here, where the
//! original local command runs. Nothing is answered here.
use crate::client::{Client, Received, Transport};
use crate::frame::Frame;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::io::{self, BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Sender, TryRecvError};
use std::time::{Duration, Instant};

/// Exit code when the remote supervisor restarted and the session is gone;
/// the harness reports "agent gone" and starts fresh on the next turn.
pub const SESSION_LOST: i32 = 3;

/// Local MCP servers that `session/new` named, by name, for the relay.
pub type Servers = BTreeMap<String, Value>;

/// Rewrite one client→agent line for the remote side.
pub fn rewrite(
    line: &str,
    remote_cwd: &str,
    state: &str,
    session: &str,
    servers: &mut Servers,
) -> String {
    let Ok(mut v) = serde_json::from_str::<Value>(line) else {
        return line.to_owned();
    };
    if v["method"] != "session/new" {
        return line.to_owned();
    }
    v["params"]["cwd"] = Value::String(remote_cwd.to_owned());
    if let Some(list) = v["params"]["mcpServers"].as_array_mut() {
        for s in list.iter_mut() {
            let local_socket = s["args"]
                .as_array()
                .is_some_and(|a| a.iter().any(|x| x == "-U"));
            let Some(name) = s["name"].as_str().map(str::to_owned) else {
                continue;
            };
            if local_socket {
                servers.insert(name.clone(), s.clone());
                s["command"] = json!("bboxy450-term-remote");
                s["args"] = json!([
                    "mcp",
                    "--state",
                    state,
                    "--session",
                    session,
                    "--name",
                    name
                ]);
            }
        }
    }
    v.to_string()
}

fn message_id(n: u64) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    format!("{}-{nanos:x}-{n}", std::process::id())
}

enum Input {
    Line(String),
    StdinClosed,
    Mcp(u64, Option<String>),
}

struct Local {
    child: Child,
    stdin: ChildStdin,
}

/// Run the local MCP server a relayed channel names; its lines come back as Input::Mcp.
fn open_local(server: &Value, channel: u64, tx: &Sender<Input>) -> io::Result<Local> {
    let command = server["command"].as_str().unwrap_or_default();
    let args: Vec<&str> = server["args"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    let mut c = Command::new(command);
    c.args(&args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    if let Some(env) = server["env"].as_array() {
        for e in env {
            if let (Some(k), Some(v)) = (e["name"].as_str(), e["value"].as_str()) {
                c.env(k, v);
            }
        }
    }
    let mut child = c.spawn()?;
    let stdin = child.stdin.take().expect("piped");
    let stdout = child.stdout.take().expect("piped");
    let tx = tx.clone();
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines() {
            let Ok(line) = line else { break };
            if tx.send(Input::Mcp(channel, Some(line))).is_err() {
                return;
            }
        }
        let _ = tx.send(Input::Mcp(channel, None));
    });
    Ok(Local { child, stdin })
}

/// Bridge stdin/stdout to one remote session until stdin closes.
pub fn run(transport: Transport, swarm: &str, session: &str, remote_cwd: &str) -> io::Result<i32> {
    let state = transport.state().to_owned();
    let mut client = Client::new(transport, swarm);
    client.connect()?;
    client.spawn_live(session, remote_cwd)?;
    let (tx, rx) = mpsc::channel::<Input>();
    let stdin_tx = tx.clone();
    std::thread::spawn(move || {
        for line in io::stdin().lock().lines() {
            let Ok(line) = line else { break };
            if stdin_tx.send(Input::Line(line)).is_err() {
                return;
            }
        }
        let _ = stdin_tx.send(Input::StdinClosed);
    });
    let mut out = io::stdout().lock();
    let mut servers = Servers::new();
    let mut locals: BTreeMap<u64, Local> = BTreeMap::new();
    let (mut sent, mut cleared, mut stdin_open) = (0u64, 0u32, true);
    loop {
        loop {
            match rx.try_recv() {
                Ok(Input::Line(line)) if !line.trim().is_empty() => {
                    sent += 1;
                    let line = rewrite(&line, remote_cwd, &state, session, &mut servers);
                    client.op(session, &message_id(sent), &line)?;
                }
                Ok(Input::Line(_)) => {}
                Ok(Input::StdinClosed) => stdin_open = false,
                Ok(Input::Mcp(channel, Some(line))) => {
                    let _ = client.mcp(&Frame::McpData { channel, line });
                }
                Ok(Input::Mcp(channel, None)) => {
                    locals.remove(&channel);
                    let _ = client.mcp(&Frame::McpClose { channel });
                }
                Err(TryRecvError::Empty) | Err(TryRecvError::Disconnected) => break,
            }
        }
        if !stdin_open && !client.has_pending() {
            for (_, mut l) in locals {
                let _ = l.child.kill();
                let _ = l.child.wait();
            }
            return Ok(0);
        }
        match client.recv_timeout(Duration::from_millis(20))? {
            Some(Received::Show { envelope, .. }) => {
                writeln!(out, "{envelope}")?;
                out.flush()?;
            }
            Some(Received::Cleared { .. }) => {
                cleared += 1;
                if cleared > 1 {
                    eprintln!(
                        "bboxy450-term-remote attach: remote supervisor restarted; session lost"
                    );
                    return Ok(SESSION_LOST);
                }
            }
            Some(Received::Refused { reason, .. }) => {
                eprintln!("bboxy450-term-remote attach: refused: {reason}");
                return Ok(SESSION_LOST);
            }
            Some(Received::McpOpen { channel, name, .. }) => {
                match servers.get(&name).map(|s| open_local(s, channel, &tx)) {
                    Some(Ok(local)) => {
                        locals.insert(channel, local);
                    }
                    _ => {
                        let _ = client.mcp(&Frame::McpClose { channel });
                    }
                }
            }
            Some(Received::McpData { channel, line }) => {
                if let Some(l) = locals.get_mut(&channel)
                    && (writeln!(l.stdin, "{line}").is_err() || l.stdin.flush().is_err())
                {
                    locals.remove(&channel);
                }
            }
            Some(Received::McpClose { channel }) => {
                if let Some(mut l) = locals.remove(&channel) {
                    let _ = l.child.kill();
                    let _ = l.child.wait();
                }
            }
            Some(Received::Down) => {
                // MCP channels do not survive a drop: the remote side closed them.
                for (_, mut l) in std::mem::take(&mut locals) {
                    let _ = l.child.kill();
                    let _ = l.child.wait();
                }
                client.reconnect(Instant::now() + Duration::from_secs(600))?;
            }
            _ => {}
        }
    }
}

/// The remote end of one relayed MCP server: stdio ↔ the daemon socket.
pub fn mcp(state: &std::path::Path, session: &str, name: &str) -> io::Result<()> {
    let sock = std::os::unix::net::UnixStream::connect(state.join("link.sock"))?;
    let mut writer = sock.try_clone()?;
    crate::frame::write(
        &mut writer,
        &Frame::McpOpen {
            session: session.into(),
            channel: 0,
            name: name.into(),
        },
    )?;
    let mut reader = sock.try_clone()?;
    std::thread::spawn(move || {
        let mut out = io::stdout().lock();
        while let Ok(Some(f)) = crate::frame::read(&mut reader) {
            match f {
                Frame::McpData { line, .. } => {
                    if writeln!(out, "{line}").and_then(|()| out.flush()).is_err() {
                        break;
                    }
                }
                Frame::McpClose { .. } => break,
                _ => {}
            }
        }
        std::process::exit(0); // the relay closed: this MCP server ends
    });
    for line in io::stdin().lock().lines() {
        crate::frame::write(
            &mut writer,
            &Frame::McpData {
                channel: 0,
                line: line?,
            },
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn session_new_gets_the_remote_cwd_and_local_socket_mcp_becomes_a_relay() {
        let line = r#"{"jsonrpc":"2.0","id":2,"method":"session/new","params":{"cwd":"/Users/me/app","mcpServers":[{"name":"bboxy","command":"/usr/bin/nc","args":["-U","/tmp/x.sock"]},{"name":"other","command":"npx","args":["srv"]}]}}"#;
        let mut servers = Servers::new();
        let v: Value =
            serde_json::from_str(&rewrite(line, "/srv/app", "/st", "seat", &mut servers)).unwrap();
        assert_eq!(v["params"]["cwd"], "/srv/app");
        let list = v["params"]["mcpServers"].as_array().unwrap();
        assert_eq!(list[0]["command"], "bboxy450-term-remote");
        assert_eq!(
            list[0]["args"],
            json!([
                "mcp",
                "--state",
                "/st",
                "--session",
                "seat",
                "--name",
                "bboxy"
            ])
        );
        assert_eq!(
            list[1]["command"], "npx",
            "remote-runnable servers pass unchanged"
        );
        assert_eq!(
            servers["bboxy"]["command"], "/usr/bin/nc",
            "original kept for the local end"
        );
        let prompt = r#"{"jsonrpc":"2.0","id":3,"method":"session/prompt","params":{"sessionId":"s","prompt":[]}}"#;
        assert_eq!(
            rewrite(prompt, "/srv/app", "/st", "seat", &mut servers),
            prompt
        );
        assert_eq!(
            rewrite("not json", "/x", "/st", "seat", &mut servers),
            "not json"
        );
    }
}
