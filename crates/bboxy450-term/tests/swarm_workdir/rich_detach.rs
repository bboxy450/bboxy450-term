// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Actual App, native rich actor, independent backend and real scripted harness turn.
use super::*;
use serde_json::json;

fn wait<T>(mut read: impl FnMut() -> Option<T>) -> T {
    let until = Instant::now() + Duration::from_secs(35);
    loop {
        if let Some(value) = read() {
            return value;
        }
        assert!(Instant::now() < until, "rich App witness did not progress");
        std::thread::sleep(Duration::from_millis(20));
    }
}
fn row(f: &Fixture, seat: &str) -> Value {
    f.ctl(&["seat", "ls"])
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["record"]["id"] == seat)
        .unwrap()
        .clone()
}
fn identity(row: &Value) -> Value {
    json!({"process":row["state"]["process"],"generation":row["record"]["generation"],"session":row["record"]["harness_session_id"]})
}
fn rich(f: &Fixture, seat: &str, min_seq: u64) -> Value {
    wait(|| {
        let frame = f.ctl(&["ui", "layout"]);
        let rich = &frame["rich_observation"][seat];
        (rich["ready"] == true
            && rich["paint_count"].as_u64().unwrap_or(0) > 0
            && rich["cursor"]["seq"].as_u64().unwrap_or(0) >= min_seq)
            .then(|| rich.clone())
    })
}
fn events(path: &Path) -> Vec<Value> {
    fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}
#[test]
#[ignore = "real private headless App, native harness and 30-second tool; make test-rich-app-detach"]
fn rich_app_detach_during_native_turn_retains_owner_and_replays() {
    let binary = PathBuf::from(std::env::var_os("BBOXY_HARNESS_TEST_BIN").unwrap());
    assert!(binary.is_absolute() && binary.is_file());
    // Both the term alias and the standalone session socket must fit sockaddr_un.
    let mut f = Fixture::with_directory(
        tempfile::Builder::new()
            .prefix("rich-app-")
            .tempdir_in("/tmp")
            .unwrap(),
    );
    fs::copy(BIN, f.root.join("bin/bboxy450-term")).unwrap();
    println!("rich-app-detach: private_root={}", f.root.display());
    let quote = |p: &Path| format!("'{}'", p.to_str().unwrap().replace('\'', "'\\''"));
    let pid_path = f.root.join("tool.pid");
    let command = format!("printf '%s\\n' \"$$\" > {}; sleep 30", quote(&pid_path));
    let delta = json!({"choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"id":"detach-tool","type":"function","function":{"name":"bash","arguments":serde_json::to_string(&json!({"command":command})).unwrap()}}]},"finish_reason":null}]});
    let tool = f.root.join("tool.sse");
    fs::write(&tool,format!("data: {delta}\n\ndata: {{\"choices\":[{{\"index\":0,\"delta\":{{}},\"finish_reason\":\"tool_calls\"}}]}}\n\ndata: [DONE]\n\n")).unwrap();
    let text = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../bboxy450-harness/tests/fixtures/sse/text.sse")
        .canonicalize()
        .unwrap();
    let shim = f.root.join("bin/bboxy450-harness");
    fs::write(
        &shim,
        format!(
            "#!/bin/sh\nexec {} \"$@\" --scripted {} --scripted {} --yolo\n",
            quote(&binary),
            quote(&tool),
            quote(&text)
        ),
    )
    .unwrap();
    fs::set_permissions(&shim, fs::Permissions::from_mode(0o700)).unwrap();
    f.start();
    let swarm = f.ctl(&["ui", "layout"])["swarm"]
        .as_str()
        .unwrap()
        .to_owned();
    let seat = f.spawn("bboxy450-harness", &swarm, None);
    let initial = rich(&f, &seat, 1);
    println!("rich-app-detach: initial={initial}");
    let before = identity(&row(&f, &seat));
    let session = before["session"].as_str().unwrap();
    let log = f
        .root
        .join(format!("home/.bboxy450-harness/sessions/{session}.jsonl"));
    // Exercise the actual App face toggle, then restore the rich preference before detach.
    let frame = f.ctl(&["ui", "layout"]);
    let pane = frame["panes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["seat"] == seat)
        .unwrap();
    let rect = pane["rect"].as_array().unwrap();
    let x = (rect[0].as_f64().unwrap() + rect[2].as_f64().unwrap() - 52.)
        .round()
        .to_string();
    let y = (rect[1].as_f64().unwrap() - 13.).round().to_string();
    f.ctl(&["ui", "click", &x, &y]);
    assert!(
        f.ctl(&["ui", "layout"])["rich_observation"]
            .get(&seat)
            .is_none()
    );
    f.ctl(&["ui", "click", &x, &y]);
    rich(&f, &seat, initial["cursor"]["seq"].as_u64().unwrap());
    f.ctl(&[
        "ui",
        "input",
        &json!({"kind":"commit","text":"run the detached tool"}).to_string(),
    ]);
    f.ctl(&["ui", "input", &json!({"kind":"key","logical":"Enter","physical":"Enter","unmodified":"Enter","text":"\r","event":"press"}).to_string()]);
    let tool_pid: i32 = wait(|| {
        fs::read_to_string(&pid_path)
            .ok()
            .and_then(|s| s.trim().parse().ok())
    });
    // SAFETY: exact child PID written by this fixture's own tool.
    assert_eq!(unsafe { libc::kill(tool_pid, 0) }, 0);
    let active = rich(&f, &seat, initial["cursor"]["seq"].as_u64().unwrap() + 1);
    let owners = f.owners();
    // Kill only the UI process we started: backend/session custodians stay attached.
    let mut ui = f.app.take().unwrap();
    let ui_pid = ui.id();
    ui.kill().unwrap();
    let exit = ui.wait().unwrap();
    let detached = Instant::now();
    while detached.elapsed() < Duration::from_secs(21) {
        assert_eq!(identity(&row(&f, &seat)), before);
        assert_eq!(unsafe { libc::kill(tool_pid, 0) }, 0, "turn died with UI");
        std::thread::sleep(Duration::from_millis(200));
    }
    f.start();
    assert_eq!(f.owners(), owners);
    assert_eq!(identity(&row(&f, &seat)), before);
    let resumed = rich(&f, &seat, active["cursor"]["seq"].as_u64().unwrap());
    assert_eq!(resumed["face"], "harness_rich");
    assert_eq!(resumed["cursor"]["session"], active["cursor"]["session"]);
    assert_eq!(
        resumed["cursor"]["generation"],
        active["cursor"]["generation"]
    );
    assert_eq!(resumed["cursor"]["epoch"], active["cursor"]["epoch"]);
    wait(|| {
        events(&log)
            .iter()
            .any(|e| e["event"]["type"] == "turn_ended")
            .then_some(())
    });
    let complete = events(&log);
    assert_eq!(
        complete
            .iter()
            .filter(|e| e["event"]["type"] == "turn_queued")
            .count(),
        1
    );
    let tail = complete.last().unwrap()["seq"].as_u64().unwrap();
    let final_frame = rich(&f, &seat, tail);
    let mut transcript = bboxy450_harness::frontends::gui::model::Transcript::default();
    for event in &complete {
        transcript.apply(
            &bboxy450_harness::protocol::Envelope::from_line(&format!("{event}\n")).unwrap(),
        );
    }
    assert_eq!(
        final_frame["retained_items"].as_u64().unwrap(),
        transcript.items.len() as u64,
        "cold reopen must rebuild the whole prefix, not only the cursor suffix"
    );
    // Terminal is not the default for this announced session: reattachment must
    // retain an explicit Terminal preference as well as the rich replay cursor.
    f.ctl(&["ui", "click", &x, &y]);
    assert!(
        f.ctl(&["ui", "layout"])["rich_observation"]
            .get(&seat)
            .is_none()
    );
    let mut terminal_ui = f.app.take().unwrap();
    terminal_ui.kill().unwrap();
    terminal_ui.wait().unwrap();
    f.start();
    assert_eq!(f.owners(), owners);
    assert_eq!(identity(&row(&f, &seat)), before);
    assert!(
        f.ctl(&["ui", "layout"])["rich_observation"]
            .get(&seat)
            .is_none(),
        "Terminal face preference was not persisted"
    );
    f.ctl(&["ui", "click", &x, &y]);
    let reopened = rich(&f, &seat, tail);
    assert_eq!(reopened["retained_items"], final_frame["retained_items"]);
    println!(
        "rich-app-detach: ui_pid={ui_pid} exit={exit} detached_ms={} owners={owners:?} identity={before} before={} after={} head={tail}",
        detached.elapsed().as_millis(),
        active["cursor"],
        final_frame["cursor"]
    );
}
