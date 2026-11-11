// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Real native canvas/seat/MCP witness using the existing private headless owner.
#![allow(
    clippy::disallowed_methods,
    reason = "private unattended canvas process witness"
)]
use super::{CrashOwners, Fixture, await_value, crate_process_absent};
use bbt_mux::{
    mcp::bridge::{Bridge, Config},
    store::{Store as _, object_dir, sqlite::SqliteStore},
};
use serde_json::{Value, json};
use std::{
    cell::Cell,
    fs,
    path::PathBuf,
    time::{Duration, Instant},
};

/// Retain only this fixture's bounded stderr logs before its normal owner
/// cleanup runs. Admission and identity files are never copied.
struct FailureLogs {
    run: PathBuf,
    phase: Cell<&'static str>,
}
impl FailureLogs {
    fn phase(&self, phase: &'static str) {
        self.phase.set(phase);
        eprintln!("canvas process phase: {phase}");
    }
}
impl Drop for FailureLogs {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            return;
        }
        let directory =
            PathBuf::from("/tmp/canvas-c1b-diagnostic").join(bbt_mux::Ulid::new().to_string());
        let _ = fs::create_dir_all(&directory);
        let _ = fs::write(directory.join("phase.txt"), self.phase.get());
        if let Ok(entries) = fs::read_dir(&self.run) {
            for entry in entries.flatten() {
                if !entry.file_type().is_ok_and(|kind| kind.is_file()) {
                    continue;
                }
                if !entry
                    .path()
                    .extension()
                    .is_some_and(|extension| extension == "log" || extension == "stderr")
                {
                    continue;
                }
                if let Ok(bytes) = fs::read(entry.path()) {
                    let tail = &bytes[bytes.len().saturating_sub(128 * 1024)..];
                    let _ = fs::write(directory.join(entry.file_name()), tail);
                }
            }
        }
        eprintln!("canvas process failure logs: {}", directory.display());
    }
}

fn rpc(peer: &mut Bridge, id: u64, method: &str, params: Value) -> Value {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let value = peer
            .handle(
                &serde_json::to_vec(&json!({
                    "jsonrpc":"2.0", "id":id, "method":method, "params":params
                }))
                .unwrap(),
            )
            .unwrap();
        let code = &value["result"]["structuredContent"]["error"]["code"];
        if code != "workspace_pending" && code != "canvas_busy" {
            return value;
        }
        // Both refusals have no canvas effect and ask for the same request
        // again: the private root's admission, or another append, completes.
        assert!(
            Instant::now() < deadline,
            "canvas request still refused after 10 s: {value}"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}
fn data(reply: Value) -> Value {
    assert!(reply.get("error").is_none(), "{reply}");
    assert_eq!(reply["result"]["isError"], false, "{reply}");
    reply["result"]["structuredContent"].clone()
}
fn canvas<'a>(layout: &'a Value, id: &str) -> Option<&'a Value> {
    layout["canvases"]
        .as_object()?
        .values()
        .find(|view| view["source"]["id"] == id)
}
fn named<'a>(node: &'a Value, suffix: &str) -> Option<&'a Value> {
    if node["kind"]
        .as_str()
        .is_some_and(|kind| kind.ends_with(suffix))
    {
        return Some(node);
    }
    node["children"]
        .as_array()?
        .iter()
        .find_map(|child| named(child, suffix))
}
fn row<'a>(layout: &'a Value, key: &str) -> Option<&'a Value> {
    layout["tree"]
        .as_array()?
        .iter()
        .find_map(|node| named(node, key))
}

// Adapted from tests/dream_read.rs: read only a throwaway shell's delivered
// admission. Capability bytes stay in memory and never enter logs or reports.
fn bridge(f: &Fixture, seat: &str) -> Bridge {
    let admission = await_value(f, || {
        let value: Value =
            serde_json::from_slice(&fs::read(f.root.path().join("canvas-admission.json")).ok()?)
                .ok()?;
        (value["BBOXY_SEAT_ID"] == seat).then_some(value)
    });
    let path = PathBuf::from(admission["BBOXY_CAPABILITY_FILE"].as_str().unwrap());
    assert!(
        path.canonicalize()
            .unwrap()
            .starts_with(f.root.path().canonicalize().unwrap())
    );
    let raw = fs::read(path).unwrap();
    assert_eq!(raw.len(), 65, "private admission format changed");
    let capability = std::array::from_fn(|i| {
        u8::from_str_radix(std::str::from_utf8(&raw[i * 2..i * 2 + 2]).unwrap(), 16).unwrap()
    });
    let mut peer = Bridge::new(Config {
        socket: admission["BBOXY_SOCK"].as_str().unwrap().into(),
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
        json!({
            "protocolVersion":"2025-11-25", "capabilities":{},
            "clientInfo":{"name":"private-canvas-native","version":"1"}
        }),
    );
    assert_eq!(initialized["result"]["protocolVersion"], "2025-11-25");
    assert!(
        peer.handle(br#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#)
            .is_none()
    );
    let oriented = data(rpc(
        &mut peer,
        2,
        "tools/call",
        json!({"name":"orient","arguments":{}}),
    ));
    assert_eq!(oriented["actor"]["seat_id"], seat);
    peer
}

#[test]
fn canvas_native_mcp_three_batches_restart_sidebar_and_headless_capture() {
    let mut f = Fixture::new();
    // Declared after the owner, so unwinding retains logs before Fixture drops.
    let diagnostic = FailureLogs {
        run: f.run.clone(),
        phase: Cell::new("setup"),
    };
    let repository = f.root.path().join("repository");
    fs::create_dir_all(repository.join(".bboxy")).unwrap();
    assert!(
        std::process::Command::new("/usr/bin/git")
            .args(["init", "-q"])
            .arg(&repository)
            .env_clear()
            .env("HOME", f.root.path())
            .env("PATH", "/usr/bin:/bin")
            .status()
            .unwrap()
            .success(),
        "private canvas repository initialization"
    );
    // Write only this private native shell's delivered paths/ids, never its key.
    fs::write(f.root.path().join("shell/.zshrc"), format!(
        "/usr/bin/python3 -c 'import json,os; p={:?}; f=os.open(p,os.O_WRONLY|os.O_CREAT|os.O_TRUNC,0o600); os.write(f,json.dumps({{k:os.environ[k] for k in [\"BBOXY_SOCK\",\"BBOXY_SEAT_ID\",\"BBOXY_GENERATION\",\"BBOXY_CAPABILITY_FILE\"]}}).encode()); os.close(f)'\n",
        f.root.path().join("canvas-admission.json").to_str().unwrap()
    )).unwrap();
    let account = f.root.path().join("canvas-account");
    diagnostic.phase("start private GUI");
    let start = f.ok(&[
        "app",
        "start",
        "--headless",
        "--ctl",
        "--unattended",
        "--test-account",
        account.to_str().unwrap(),
    ]);
    f.pid = Some(start["data"]["pid"].as_u64().unwrap() as u32);
    assert_eq!(start["data"]["windows"], 0);
    assert_eq!(start["data"]["custody_backend"], "file");
    diagnostic.phase("initial layout");
    let initial = f.ok(&["ui", "layout"]);
    let swarm = initial["data"]["swarm"].as_str().unwrap().to_owned();
    f.ok(&["ui", "swarm-workdir", &swarm, repository.to_str().unwrap()]);
    diagnostic.phase("spawn native seat");
    let spawned = f.ok(&[
        "seat",
        "spawn",
        "zsh",
        "--name",
        "Canvas native author",
        "--place",
        "tab",
        "--swarm",
        &swarm,
        "--cwd",
        repository.to_str().unwrap(),
    ]);
    let seat = spawned["data"]["seat"].as_str().unwrap().to_owned();
    diagnostic.phase("admit native MCP");
    let mut peer = bridge(&f, &seat);
    diagnostic.phase("canvas_open");
    let opened = data(rpc(
        &mut peer,
        3,
        "tools/call",
        json!({
            "name":"canvas_open", "arguments":{
                "op_id": bbt_mux::Ulid::new(), "title":"Ten kept services",
                "tags":["canvas","plan","services","restart","kept"]
            }
        }),
    ));
    assert_eq!(opened["shown"], true, "{opened}");
    let id = opened["id"].as_str().unwrap().to_owned();
    let key = format!("canvas/{swarm}/open/{id}");
    diagnostic.phase("first canvas layout");
    let shown = await_value(&f, || {
        let layout = f.ok(&["ui", "layout"])["data"].clone();
        (canvas(&layout, &id).is_some() && row(&layout, &key).is_some()).then_some(layout)
    });
    assert!(shown["canvases"].as_object().unwrap().len() == 1);
    diagnostic.phase("three canvas_draw batches");
    for (batch, range) in [0..3, 3..7, 7..10].into_iter().enumerate() {
        let ops: Vec<_> = range.map(|i| json!({
            "op":"add", "element":{
                "id":format!("service-{i}"), "tags":["service"], "kind":"service",
                "content":{"type":"shape","shape":"rounded","bounds":{"origin":[(i%5)*160,(i/5)*120],"size":[130,72]},"label":format!("Service {i}")}
            }
        })).collect();
        data(rpc(
            &mut peer,
            4 + batch as u64,
            "tools/call",
            json!({
                "name":"canvas_draw", "arguments":{"op_id":bbt_mux::Ulid::new(),"id":id,"ops":ops}
            }),
        ));
    }
    let query = json!({"id":id}).to_string();
    diagnostic.phase("read and observe digest");
    let before = f.ok(&["canvas", "read", &query, "--swarm", &swarm])["data"].clone();
    assert_eq!(before["elements"].as_array().unwrap().len(), 10);
    assert_eq!(before["seq"], 10);
    let digest = before["digest"].clone();
    let drawn = await_value(&f, || {
        let layout = f.ok(&["ui", "layout"])["data"].clone();
        canvas(&layout, &id)
            .filter(|view| view["seq"] == 10 && view["digest"] == digest)
            .map(|_| layout.clone())
    });
    assert!(row(&drawn, &key).is_some());
    assert_eq!(
        fs::read_to_string(
            repository
                .join(".bboxy/canvases")
                .join(format!("{}.log.jsonl", opened["slug"].as_str().unwrap()))
        )
        .unwrap()
        .lines()
        .count(),
        3
    );

    // The sidebar's actual retained hit opens the ordinary canvas tab again.
    diagnostic.phase("click actual sidebar row");
    f.ok(&["seat", "focus", &seat]);
    let layout = f.ok(&["ui", "layout"])["data"].clone();
    let rect = row(&layout, &key).unwrap()["rect"].as_array().unwrap();
    let x = (rect[0].as_f64().unwrap() + rect[2].as_f64().unwrap() / 2.) as u32;
    let y = (rect[1].as_f64().unwrap() + rect[3].as_f64().unwrap() / 2.) as u32;
    f.ok(&["ui", "click", &x.to_string(), &y.to_string()]);
    await_value(&f, || {
        let layout = f.ok(&["ui", "layout"])["data"].clone();
        let focused = layout["canvases"].as_object()?.iter().any(|(pane, view)| {
            view["source"]["id"] == id && layout["focused"].as_str() == Some(pane.as_str())
        });
        focused.then_some(layout)
    });
    f.ok(&["ui", "key", "f"]);

    // Wait for the actual durable layout checkpoint before killing only our GUI.
    diagnostic.phase("wait durable window checkpoint");
    let store = SqliteStore::open(&object_dir(&f.run, "account").unwrap()).unwrap();
    await_value(&f, || {
        let saved: bbt_proto::gui::WindowState =
            serde_json::from_slice(&store.get("gui/window/state").ok()??).ok()?;
        saved
            .swarms
            .iter()
            .flat_map(|swarm| &swarm.tabs)
            .any(|tab| tab.canvases.values().any(|source| source.id == id))
            .then_some(Value::Bool(true))
    });
    let owner = |name: &str| -> u32 {
        serde_json::from_slice::<Value>(
            &fs::read(f.run.join(format!("{name}.owner.json"))).unwrap(),
        )
        .unwrap()["process"]["pid"]
            .as_u64()
            .unwrap() as u32
    };
    let owners = [owner("logic"), owner("supervisor")];
    let mut guard = CrashOwners::new(&owners);
    diagnostic.phase("restart GUI");
    let old_app = f.pid.take().unwrap();
    // SAFETY: this is the exact GUI PID started and recorded above in our TempDir.
    assert_eq!(unsafe { libc::kill(old_app as i32, libc::SIGKILL) }, 0);
    await_value(&f, || {
        crate_process_absent(old_app).then_some(Value::Bool(true))
    });
    let restarted = f.ok(&[
        "app",
        "start",
        "--headless",
        "--ctl",
        "--unattended",
        "--test-account",
        account.to_str().unwrap(),
    ]);
    f.pid = Some(restarted["data"]["pid"].as_u64().unwrap() as u32);
    assert_eq!([owner("logic"), owner("supervisor")], owners);
    guard.armed = false;
    diagnostic.phase("restored digest and sidebar");
    let after = f.ok(&["canvas", "read", &query, "--swarm", &swarm])["data"].clone();
    assert_eq!(after["digest"], digest);
    assert_eq!(after["elements"], before["elements"]);
    let restored = await_value(&f, || {
        let layout = f.ok(&["ui", "layout"])["data"].clone();
        (canvas(&layout, &id).is_some_and(|view| view["digest"] == digest)
            && row(&layout, &key).is_some())
        .then_some(layout)
    });
    assert!(
        restored["canvases"]
            .as_object()
            .unwrap()
            .values()
            .any(|view| view["source"]["title"] == "Ten kept services")
    );
    let listed = f.ok(&["canvas", "list", "--swarm", &swarm]);
    assert_eq!(listed["data"]["canvases"][0]["id"], id);
    f.ok(&["ui", "key", "f"]);
    diagnostic.phase("capture canvas");
    let png = f.root.path().join("canvas-c1b-headless.png");
    let capture = f.ok(&["seat", "capture", "window", png.to_str().unwrap()]);
    assert_eq!(capture["data"]["offscreen"], true);
    assert!(fs::read(&png).unwrap().starts_with(b"\x89PNG\r\n\x1a\n"));
    if let Some(directory) = std::env::var_os("BBOXY_CANVAS_CAPTURE") {
        let directory = PathBuf::from(directory);
        fs::create_dir_all(&directory).unwrap();
        let path = directory.join("canvas-c1b-headless.png");
        fs::copy(png, &path).unwrap();
        println!("canvas headless capture: {}", path.display());
    }
    diagnostic.phase("close canvas restores legacy layout");
    f.ok(&["ui", "key", "cmd-w"]);
    let closed = await_value(&f, || {
        let layout = f.ok(&["ui", "layout"])["data"].clone();
        canvas(&layout, &id).is_none().then_some(layout)
    });
    assert_eq!(closed["focused"], seat);
    let old_format = await_value(&f, || {
        let raw = store.get("gui/window/state").ok()??;
        let saved: Value = serde_json::from_slice(&raw).ok()?;
        let tabs: Vec<_> = saved["swarms"]
            .as_array()?
            .iter()
            .flat_map(|s| s["tabs"].as_array().into_iter().flatten())
            .collect();
        if tabs.iter().any(|t| t.get("canvases").is_some()) {
            return None;
        }
        // v5's strict tab reader knows exactly these unchanged fields.
        for tab in tabs {
            assert!(tab.as_object().unwrap().keys().all(|k| {
                ["id", "name", "shortcut", "expanded", "seats", "tree"].contains(&k.as_str())
            }));
        }
        let decoded: bbt_proto::gui::WindowState = serde_json::from_slice(&raw).unwrap();
        assert_eq!(serde_json::to_vec(&decoded).unwrap(), raw);
        Some(saved)
    });
    assert_eq!(old_format["stage"]["focused"], seat);
    assert_eq!(
        [owner("logic"), owner("supervisor")],
        owners,
        "closing content never stops backend owners"
    );
    let seats = f.ok(&["seat", "ls"]);
    assert_eq!(seats["data"].as_array().unwrap().len(), 1);
    assert_eq!(seats["data"][0]["record"]["id"], seat);
    drop(peer);
    diagnostic.phase("stop owned processes");
    f.ok(&["app", "stop"]);
    f.wait_stopped();
}
