// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Private headless term/native shell/admitted MCP process witness.
//! Adapted from tests/dream_c1.rs; own throwaway identity and exact PID cleanup.
#![allow(
    clippy::disallowed_methods,
    reason = "private unattended process fixture"
)]
use serde_json::{Value, json};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};
const BIN: &str = env!("CARGO_BIN_EXE_bboxy450-term");
struct Process {
    root: tempfile::TempDir,
    app: Option<Child>,
}
impl Process {
    fn command(&self) -> Command {
        let mut c = Command::new(BIN);
        c.env_clear()
            .env("HOME", self.root.path().join("home"))
            .env("ZDOTDIR", self.root.path().join("home"))
            .env("PATH", "/usr/bin:/bin:/usr/sbin:/sbin")
            .env("TERM", "xterm-256color")
            .env("LANG", "en_US.UTF-8")
            .current_dir(self.root.path())
            .stdin(Stdio::null());
        c
    }
    fn ctl(&self, args: &[&str]) -> Option<Value> {
        let out = self
            .command()
            .arg("ctl")
            .arg("--run-dir")
            .arg(self.root.path().join("run"))
            .args(args)
            .output()
            .unwrap();
        if !out.status.success() {
            return None;
        }
        serde_json::from_slice::<Value>(&out.stdout)
            .ok()
            .map(|v| v["data"].clone())
    }
    fn open() -> Self {
        let root = tempfile::Builder::new()
            .prefix("dq1-")
            .tempdir_in(std::path::Path::new("/tmp").canonicalize().unwrap())
            .unwrap();
        let mut f = Self { root, app: None };
        for name in ["home", "repository", "repository/.bboxy"] {
            fs::create_dir(f.root.path().join(name)).unwrap();
            fs::set_permissions(f.root.path().join(name), fs::Permissions::from_mode(0o700))
                .unwrap();
        }
        // A native shell fixture records only paths/IDs to its OWN admission,
        // never operator environment or capability bytes. EOF remains non-close.
        let script = format!(
            "/usr/bin/python3 -c 'import json,os; p={:?}; f=os.open(p,os.O_WRONLY|os.O_CREAT|os.O_TRUNC,0o600); os.write(f,json.dumps({{k:os.environ[k] for k in [\"BBOXY_SOCK\",\"BBOXY_SEAT_ID\",\"BBOXY_GENERATION\",\"BBOXY_CAPABILITY_FILE\"]}}).encode()); os.close(f)'\nexec /bin/cat\n",
            f.root.path().join("admission.json").to_str().unwrap()
        );
        fs::write(f.root.path().join("home/.zshrc"), script).unwrap();
        let git = Command::new("/usr/bin/git")
            .args(["init", "-q"])
            .arg(f.root.path().join("repository"))
            .env_clear()
            .env("HOME", f.root.path().join("home"))
            .env("PATH", "/usr/bin:/bin")
            .output()
            .unwrap();
        assert!(
            git.status.success(),
            "private repository initialization failed"
        );
        let log = fs::File::create(f.root.path().join("app.log")).unwrap();
        f.app = Some(
            f.command()
                .args(["--headless", "--ctl", "--unattended", "--test-account"])
                .arg(f.root.path().join("account"))
                .arg("--run-dir")
                .arg(f.root.path().join("run"))
                .arg("--repository")
                .arg(f.root.path().join("repository"))
                .stdout(log.try_clone().unwrap())
                .stderr(log)
                .spawn()
                .unwrap(),
        );
        let until = Instant::now() + Duration::from_secs(30);
        loop {
            assert!(
                f.app.as_mut().unwrap().try_wait().unwrap().is_none(),
                "{}",
                fs::read_to_string(f.root.path().join("app.log")).unwrap()
            );
            if f.ctl(&["app", "status"])
                .is_some_and(|v| v["backend_ready"] == true && v["gui_ready"] == true)
            {
                break;
            }
            assert!(
                Instant::now() < until,
                "headless private app startup timeout"
            );
            std::thread::sleep(Duration::from_millis(20));
        }
        f
    }
}
impl Drop for Process {
    fn drop(&mut self) {
        if std::thread::panicking() {
            let log = fs::read_to_string(self.root.path().join("app.log")).unwrap_or_default();
            let tail: String = log
                .chars()
                .rev()
                .take(4096)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect();
            eprintln!("private headless app diagnostics: {tail}");
        }
        let _ = self.ctl(&["app", "stop"]);
        if let Some(mut c) = self.app.take() {
            if c.try_wait().ok().flatten().is_none() {
                let _ = c.kill();
            }
            let _ = c.wait();
        }
    }
}
fn rpc(peer: &mut bbt_mux::mcp::bridge::Bridge, id: u64, method: &str, params: Value) -> Value {
    peer.handle(
        &serde_json::to_vec(&json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}))
            .unwrap(),
    )
    .unwrap()
}
#[test]
fn dream_read_process_native_admitted_discovery_and_cold_reads_start_no_job() {
    let f = Process::open();
    let layout = f.ctl(&["ui", "layout"]).unwrap();
    let swarm = layout["swarm"].as_str().unwrap();
    f.ctl(&[
        "ui",
        "swarm-workdir",
        swarm,
        f.root.path().join("repository").to_str().unwrap(),
    ])
    .expect("private root binding");

    let spawned = f
        .ctl(&[
            "seat",
            "spawn",
            "zsh",
            "--place",
            "tab",
            "--name",
            "DreamReadPrivate",
            "--swarm",
            swarm,
            "--cwd",
            f.root.path().join("repository").to_str().unwrap(),
        ])
        .unwrap();
    let seat = spawned["seat"].as_str().unwrap();
    let until = Instant::now() + Duration::from_secs(10);
    let admission = loop {
        if let Ok(bytes) = fs::read(f.root.path().join("admission.json"))
            && let Ok(v) = serde_json::from_slice::<Value>(&bytes)
            && v["BBOXY_SEAT_ID"] == seat
        {
            break v;
        }
        assert!(Instant::now() < until, "private shell admission missing");
        std::thread::sleep(Duration::from_millis(20));
    };
    let path = PathBuf::from(admission["BBOXY_CAPABILITY_FILE"].as_str().unwrap());
    assert!(
        path.canonicalize()
            .unwrap()
            .starts_with(f.root.path().canonicalize().unwrap())
    );
    let raw = fs::read(path).unwrap();
    assert_eq!(raw.len(), 65);
    let mut capability = [0; 32];
    for (i, b) in capability.iter_mut().enumerate() {
        *b = u8::from_str_radix(std::str::from_utf8(&raw[i * 2..i * 2 + 2]).unwrap(), 16).unwrap();
    }
    let mut peer = bbt_mux::mcp::bridge::Bridge::new(bbt_mux::mcp::bridge::Config {
        socket: PathBuf::from(admission["BBOXY_SOCK"].as_str().unwrap()),
        seat: seat.parse().unwrap(),
        generation: admission["BBOXY_GENERATION"]
            .as_str()
            .unwrap()
            .parse()
            .unwrap(),
        capability,
    });
    let initialized = rpc(
        &mut peer,
        1,
        "initialize",
        json!({"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"dream-read-private","version":"1"}}),
    );
    assert!(initialized.get("error").is_none());
    let _ = peer.handle(br#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#);
    let orientation = rpc(
        &mut peer,
        20,
        "tools/call",
        json!({"name":"orient","arguments":{}}),
    );
    assert_eq!(
        orientation["result"]["structuredContent"]["actor"]["seat_id"], seat,
        "{orientation}"
    );
    let catalogue = rpc(&mut peer, 2, "tools/list", json!({}));
    for tool in ["dream_search", "dream_read"] {
        assert!(
            catalogue["result"]["tools"]
                .as_array()
                .unwrap()
                .iter()
                .any(|t| t["name"] == tool)
        );
    }
    for (i, kind) in ["observation", "lesson"].into_iter().enumerate() {
        let ready = Instant::now() + Duration::from_secs(10);
        let response = loop {
            let response = rpc(
                &mut peer,
                3 + i as u64,
                "tools/call",
                json!({"name":"dream_search","arguments":{"v":1,"kind":kind,"budget_tokens":4096}}),
            );
            // tools/list admission can precede the asynchronous root binding.
            // Only setup's explicit stale_context is retried; no read starts work.
            if response["result"]["structuredContent"]["error"]["code"] == "stale_context" {
                if Instant::now() >= ready {
                    let diagnostic = rpc(
                        &mut peer,
                        21,
                        "tools/call",
                        json!({"name":"orient","arguments":{}}),
                    );
                    let data = &diagnostic["result"]["structuredContent"];
                    panic!(
                        "private root binding never became current: {response}; orientation={}; app_log={}",
                        json!({"actor":data["actor"],"availability":data["tasks"]["availability"],"reason":data["tasks"]["reason"],"root":data["knowledge_root_id"],"error":data["error"]}),
                        fs::read_to_string(f.root.path().join("app.log"))
                            .unwrap_or_default()
                            .chars()
                            .take(4096)
                            .collect::<String>()
                    );
                }
                std::thread::sleep(Duration::from_millis(20));
                continue;
            }
            break response;
        };
        let data = &response["result"]["structuredContent"];
        assert_eq!(data["status"], "unavailable", "{response}");
        assert_eq!(
            data["reason"],
            if kind == "lesson" {
                "no_published_input"
            } else {
                "not_prepared"
            }
        );
        assert!(data["hints"].is_array());
    }
    assert!(
        !f.root
            .path()
            .join("run/dream-c1")
            .read_dir()
            .is_ok_and(|mut paths| paths.any(|p| p
                .unwrap()
                .path()
                .join("output/result.json")
                .exists()))
    );
    fs::set_permissions(f.root.path(), fs::Permissions::from_mode(0o700)).unwrap();
}
