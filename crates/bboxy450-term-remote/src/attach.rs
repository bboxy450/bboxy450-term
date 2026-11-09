// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! `bboxy450-term-remote attach`: the local seat's ACP agent command. A local
//! bboxy450-harness on an `acp-agent` profile speaks ACP on this process's
//! stdio; each line goes to the seat's remote `bboxy450-harness acp` as one
//! exactly-once op, and each remote line comes back unchanged. Only two things
//! are rewritten in `session/new`: the local `cwd` becomes the remote one,
//! and local-only MCP relay entries are dropped (they name this machine's
//! sockets; the MCP relay over the link is a later cut). Nothing is answered
//! here: requests, responses and permission prompts pass through.
use crate::client::{Client, Received, Transport};
use serde_json::Value;
use std::io::{self, BufRead, Write};
use std::sync::mpsc::{self, TryRecvError};
use std::time::{Duration, Instant};

/// Exit code when the remote supervisor restarted and the session is gone;
/// the harness reports "agent gone" and starts fresh on the next turn.
pub const SESSION_LOST: i32 = 3;

/// Rewrite one client→agent line for the remote side.
pub fn rewrite(line: &str, remote_cwd: &str) -> String {
    let Ok(mut v) = serde_json::from_str::<Value>(line) else {
        return line.to_owned();
    };
    if v["method"] != "session/new" {
        return line.to_owned();
    }
    v["params"]["cwd"] = Value::String(remote_cwd.to_owned());
    if let Some(servers) = v["params"]["mcpServers"].as_array_mut() {
        servers.retain(|s| {
            let local_socket = s["args"]
                .as_array()
                .is_some_and(|a| a.iter().any(|x| x == "-U"));
            !local_socket
        });
    }
    v.to_string()
}

fn message_id(n: u64) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    format!("{}-{nanos:x}-{n}", std::process::id())
}

/// Bridge stdin/stdout to one remote session until stdin closes.
pub fn run(transport: Transport, swarm: &str, session: &str, remote_cwd: &str) -> io::Result<i32> {
    let mut client = Client::new(transport, swarm);
    client.connect()?;
    client.spawn_live(session, remote_cwd)?;
    let (tx, rx) = mpsc::channel::<String>();
    std::thread::spawn(move || {
        for line in io::stdin().lock().lines() {
            let Ok(line) = line else { break };
            if tx.send(line).is_err() {
                break;
            }
        }
    });
    let mut out = io::stdout().lock();
    let (mut sent, mut cleared, mut stdin_open) = (0u64, 0u32, true);
    loop {
        loop {
            match rx.try_recv() {
                Ok(line) if !line.trim().is_empty() => {
                    sent += 1;
                    client.op(session, &message_id(sent), &rewrite(&line, remote_cwd))?;
                }
                Ok(_) => {}
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    stdin_open = false;
                    break;
                }
            }
        }
        if !stdin_open && !client.has_pending() {
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
            Some(Received::Down) => {
                client.reconnect(Instant::now() + Duration::from_secs(600))?;
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn session_new_gets_the_remote_cwd_and_loses_local_socket_mcp() {
        let line = r#"{"jsonrpc":"2.0","id":2,"method":"session/new","params":{"cwd":"/Users/me/app","mcpServers":[{"name":"bboxy","command":"/usr/bin/nc","args":["-U","/tmp/x.sock"]},{"name":"other","command":"npx","args":["srv"]}]}}"#;
        let v: Value = serde_json::from_str(&rewrite(line, "/srv/app")).unwrap();
        assert_eq!(v["params"]["cwd"], "/srv/app");
        assert_eq!(v["params"]["mcpServers"].as_array().unwrap().len(), 1);
        assert_eq!(v["params"]["mcpServers"][0]["name"], "other");
        assert_eq!(v["id"], 2);
        let prompt = r#"{"jsonrpc":"2.0","id":3,"method":"session/prompt","params":{"sessionId":"s","prompt":[]}}"#;
        assert_eq!(
            rewrite(prompt, "/srv/app"),
            prompt,
            "other messages pass byte-for-byte"
        );
        assert_eq!(rewrite("not json", "/x"), "not json");
    }
}
