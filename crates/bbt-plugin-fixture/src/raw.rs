// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Protocol-level servers for host tests: a shape-only descriptor, an
//! out-of-order demultiplexing peer and a host-callback peer. Translated
//! from the former inline Python fixtures in bboxy450-term
//! crates/bbt-mux/tests/plugin_registry.rs, src/plugins/control_tests.rs and
//! src/plugins/client.rs: https://github.com/bboxy450/bboxy450-term

use std::io::{BufRead, Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixStream;
use std::path::Path;

use bbt_plugin_fixture::Knobs;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::reference::append;

/// Block until the test's Unix socket listener sends one byte.
pub fn wait_gate(path: &str) {
    let mut gate = UnixStream::connect(path).expect("fixture gate");
    let mut byte = [0; 1];
    let _ = gate.read(&mut byte);
}

fn emit(value: &Value) {
    let mut out = std::io::stdout().lock();
    let _ = writeln!(out, "{value}");
    let _ = out.flush();
}

/// `--shape` only. With `account_db`, first prove the operator's approval is
/// durable and the pinned copy read-only; otherwise mark `shape` once.
pub fn inspect(k: &Knobs) {
    let marker = k.marker.as_deref().expect("inspect fixture marker");
    // A failed proof is visible to the test, not only to the host's stderr capture.
    let log = marker.to_owned();
    std::panic::set_hook(Box::new(move |info| {
        append(
            &log,
            &json!({"phase":"panicked","message":info.to_string()}),
        );
    }));
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Some(db) = &k.account_db {
        append(marker, &json!({"phase":"entered","pid":std::process::id()}));
        assert_eq!(
            args,
            ["--shape"],
            "no business or setup method in P-1 cut 1"
        );
        let copy = std::fs::canonicalize(std::env::current_exe().unwrap()).unwrap();
        let digest: String = Sha256::digest(std::fs::read(&copy).unwrap())
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        let db = rusqlite::Connection::open_with_flags(
            db,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_URI,
        )
        .unwrap();
        let row: Vec<u8> = db
            .query_row(
                "SELECT value FROM kv WHERE key=?",
                [format!("plugins/trust/{digest}")],
                |row| row.get(0),
            )
            .expect("account approval must be durable before --shape");
        let record: Value = serde_json::from_slice(&row).unwrap();
        let source = std::fs::canonicalize(k.source.as_deref().unwrap()).unwrap();
        assert_eq!(record["copy"], copy.to_string_lossy().as_ref());
        assert_eq!(record["source"], source.to_string_lossy().as_ref());
        let mode = std::fs::metadata(&copy).unwrap().permissions().mode();
        assert_eq!(mode & 0o222, 0, "the pinned copy is read-only");
        let mut env: Vec<String> = std::env::vars_os()
            .map(|(name, _)| name.to_string_lossy().into_owned())
            .collect();
        env.sort();
        append(
            marker,
            &json!({"phase":"verified","pid":std::process::id(),
                "cwd":std::env::current_dir().unwrap().to_string_lossy(),
                "copy":copy.to_string_lossy(),"env":env}),
        );
    } else {
        assert_eq!(args, ["--shape"]);
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(Path::new(marker))
            .unwrap();
        let _ = file.write_all(b"shape\n");
    }
    if let Some(gate) = &k.initialize_gate {
        wait_gate(gate);
    }
    if k.hang {
        // SAFETY: setting a signal disposition to SIG_IGN has no handler code.
        unsafe { libc::signal(libc::SIGTERM, libc::SIG_IGN) };
        loop {
            std::thread::park();
        }
    }
    emit(k.descriptor.as_ref().expect("inspect fixture descriptor"));
}

/// Answers id 2 before id 1, asks the host one question in between and
/// emits notifications; then replies with an ID it was never sent.
pub fn demux() {
    let mut held: Option<Value> = None;
    for line in std::io::stdin().lock().lines() {
        let Ok(req) = serde_json::from_str::<Value>(&line.unwrap_or_default()) else {
            continue;
        };
        if req.get("id").is_none() {
            continue;
        }
        if req["id"] == "plugin-1" {
            eprintln!("host answered {}", req["error"]);
            emit(
                &json!({"jsonrpc":"2.0","method":"notifications/message","params":{"level":"info","data":"x"}}),
            );
            emit(&json!({"jsonrpc":"2.0","id":2,"result":{"order":"second request first"}}));
            emit(
                &json!({"jsonrpc":"2.0","id":held.clone().unwrap_or(Value::Null),"result":{"order":"first request last"}}),
            );
            continue;
        }
        if req["params"]["name"] == "stray" {
            emit(&json!({"jsonrpc":"2.0","id":req["id"].as_i64().unwrap_or(0) + 40,"result":{}}));
            continue;
        }
        if held.is_none() {
            held = Some(req["id"].clone());
            emit(
                &json!({"jsonrpc":"2.0","method":"notifications/progress","params":{"progressToken":"t","progress":1}}),
            );
            continue;
        }
        emit(&json!({"jsonrpc":"2.0","id":"plugin-1","method":"host/records.search","params":{}}));
    }
}

/// While serving a call, asks the host `host/<name>` and answers the call
/// only with the host's answer.
pub fn callback() {
    let stdin = std::io::stdin();
    let mut input = stdin.lock();
    let mut line = String::new();
    loop {
        line.clear();
        if input.read_line(&mut line).unwrap_or(0) == 0 {
            return;
        }
        let Ok(req) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        if req.get("method").is_none() || req.get("id").is_none() {
            continue;
        }
        let asked = format!("cb-{}", req["id"]);
        emit(
            &json!({"jsonrpc":"2.0","id":asked,"method":format!("host/{}", req["params"]["name"].as_str().unwrap_or("")),"params":{"n":req["id"]}}),
        );
        let answer = loop {
            line.clear();
            if input.read_line(&mut line).unwrap_or(0) == 0 {
                return;
            }
            if let Ok(answer) = serde_json::from_str::<Value>(&line)
                && answer["id"] == asked
            {
                break answer;
            }
        };
        emit(&json!({"jsonrpc":"2.0","id":req["id"],"result":answer}));
    }
}
