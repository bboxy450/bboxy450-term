// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Actual App, native rich actor, independent backend and real scripted harness turn.
use super::*;
use serde_json::json;

pub(super) fn wait<T>(mut read: impl FnMut() -> Option<T>) -> T {
    let until = Instant::now() + Duration::from_secs(35);
    loop {
        if let Some(value) = read() {
            return value;
        }
        assert!(Instant::now() < until, "rich App witness did not progress");
        std::thread::sleep(Duration::from_millis(20));
    }
}
pub(super) fn row(f: &Fixture, seat: &str) -> Value {
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
pub(super) fn rich(f: &Fixture, seat: &str, min_seq: u64) -> Value {
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
fn submit(f: &Fixture, text: &str) {
    f.ctl(&[
        "ui",
        "input",
        &json!({"kind":"commit","text":text}).to_string(),
    ]);
    f.ctl(&["ui","input",&json!({"kind":"key","logical":"Enter","physical":"Enter","unmodified":"Enter","text":"\r","event":"press"}).to_string()]);
}

#[test]
#[ignore = "real App native composer, approval, wheel and focus; make test-rich-app-input"]
fn rich_app_routes_composer_approval_scroll_and_focus() {
    let binary = PathBuf::from(std::env::var_os("BBOXY_HARNESS_TEST_BIN").unwrap());
    let mut f = Fixture::with_directory(
        tempfile::Builder::new()
            .prefix("rich-input-")
            .tempdir_in("/tmp")
            .unwrap(),
    );
    fs::copy(BIN, f.root.join("bin/bboxy450-term")).unwrap();
    let quote = |p: &Path| format!("'{}'", p.to_str().unwrap().replace('\'', "'\\''"));
    let response = |delta: Value, finish: &str| {
        format!(
            "data: {}\n\ndata: {}\n\ndata: [DONE]\n\n",
            json!({"choices":[{"index":0,"delta":delta,"finish_reason":null}]}),
            json!({"choices":[{"index":0,"delta":{},"finish_reason":finish}]})
        )
    };
    let long = f.root.join("long.sse");
    fs::write(&long,response(json!({"content":(0..75).map(|i|format!("Native shared row {i}: retained transcript evidence.\n\n")).collect::<String>()}),"stop")).unwrap();
    let marker = f.root.join("approved-tool");
    let tool = f.root.join("tool.sse");
    fs::write(&tool,response(json!({"tool_calls":[{"index":0,"id":"actual-app-approval","type":"function","function":{"name":"bash","arguments":json!({"command":format!("printf approved > {}",quote(&marker))}).to_string()}}]}),"tool_calls")).unwrap();
    let text = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../bboxy450-harness/tests/fixtures/sse/text.sse")
        .canonicalize()
        .unwrap();
    let shim = f.root.join("bin/bboxy450-harness");
    // This UI interaction fixture explicitly requests attended approvals. The
    // separate launch-context fixture forwards production approval argv intact.
    fs::write(
        &shim,
        format!(
            "#!/usr/bin/python3\nimport os,sys\nargs=[a for a in sys.argv[1:] if a!='--yolo']\nos.execv({0},[{0},*args,'--scripted',{1},'--scripted',{2},'--scripted',{3}])\n",
            serde_json::to_string(&binary).unwrap(),
            serde_json::to_string(&long).unwrap(),
            serde_json::to_string(&tool).unwrap(),
            serde_json::to_string(&text).unwrap()
        ),
    )
    .unwrap();
    fs::set_permissions(shim, fs::Permissions::from_mode(0o700)).unwrap();
    f.start();
    let swarm = f.ctl(&["ui", "layout"])["swarm"]
        .as_str()
        .unwrap()
        .to_owned();
    let seat = f.spawn("bboxy450-harness", &swarm, None);
    let initial = wait(|| {
        let r = rich(&f, &seat, 1);
        (r["focused"] == true && r["ime_caret"].is_array()).then_some(r)
    });
    assert_eq!(
        initial["ime_allowed"], true,
        "focused native composer has IME disabled"
    );
    assert!(
        initial["ime_caret"].is_array(),
        "native composer has no shared candidate area"
    );
    f.ctl(&[
        "ui",
        "input",
        &json!({"kind":"preedit","text":"漢","cursor":[3,3]}).to_string(),
    ]);
    f.ctl(&[
        "ui",
        "input",
        &json!({"kind":"cancel_composition"}).to_string(),
    ]);
    let identity_before = identity(&row(&f, &seat));
    submit(&f, "Show the native retained transcript");
    let full = wait(|| {
        let r = rich(&f, &seat, 1);
        (r["scroll"][1].as_f64().unwrap() > 100.).then_some(r)
    });
    let layout = f.ctl(&["ui", "layout"]);
    let pane = layout["panes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["seat"] == seat)
        .unwrap();
    let rect = pane["rect"].as_array().unwrap();
    let x = (rect[0].as_f64().unwrap() + rect[2].as_f64().unwrap() / 2.) as u32;
    let y = (rect[1].as_f64().unwrap() + rect[3].as_f64().unwrap() / 2.) as u32;
    f.ctl(&[
        "ui",
        "input",
        &json!({"kind":"wheel","x":x,"y":y,"dy":500}).to_string(),
    ]);
    let scrolled = wait(|| {
        let r = rich(&f, &seat, 1);
        (r["scroll"][0].as_f64().unwrap() + 10. < full["scroll"][0].as_f64().unwrap()).then_some(r)
    });
    let other = f.spawn("claude", &swarm, None);
    f.ctl(&["ui", "focus", &other]);
    assert!(
        f.ctl(&["ui", "layout"])["rich_observation"]
            .get(&seat)
            .is_none()
    );
    f.ctl(&["ui", "focus", &seat]);
    wait(|| {
        let r = rich(&f, &seat, 1);
        (r["focused"] == true).then_some(())
    });
    submit(&f, "Run the tool after explicit native approval");
    let approval = wait(|| {
        let r = rich(&f, &seat, 1);
        (!r["approval_choices"].as_array().unwrap().is_empty()).then_some(r)
    });
    assert!(!marker.exists(), "tool ran before the App approval");
    let choice = approval["approval_choices"][0].as_array().unwrap();
    let ax = (rect[0].as_f64().unwrap()
        + choice[0].as_f64().unwrap()
        + choice[2].as_f64().unwrap() / 2.) as u32;
    let ay = (rect[1].as_f64().unwrap()
        + choice[1].as_f64().unwrap()
        + choice[3].as_f64().unwrap() / 2.) as u32;
    f.ctl(&[
        "ui",
        "input",
        &json!({"kind":"click","x":ax,"y":ay}).to_string(),
    ]);
    wait(|| marker.exists().then_some(()));
    assert_eq!(fs::read_to_string(marker).unwrap(), "approved");
    assert_eq!(identity(&row(&f, &seat)), identity_before);
    let session = identity_before["session"].as_str().unwrap();
    let log = f
        .root
        .join(format!("home/.bboxy450-harness/sessions/{session}.jsonl"));
    wait(|| {
        (events(&log)
            .iter()
            .filter(|e| e["event"]["type"] == "turn_ended")
            .count()
            == 2)
            .then_some(())
    });
    let all = events(&log);
    assert_eq!(
        all.iter()
            .filter(|e| e["event"]["type"] == "turn_queued")
            .count(),
        2
    );
    println!(
        "rich-app-input: seat={seat} composer_turns=2 wheel_before={} wheel_after={} approval_click=[{ax},{ay}] tool=approved identity={identity_before}",
        full["scroll"], scrolled["scroll"]
    );
}
#[test]
#[ignore = "real private headless App, native harness and 30-second tool; make test-rich-app-detach"]
fn rich_app_detach_during_native_turn_retains_owner_and_replays() {
    app_detach(false);
}

#[test]
#[ignore = "canonical prepared ad-hoc bundle, GUI-only replacement and rollback; make test-rich-gui-handoff"]
fn rich_gui_handoff_preserves_turn_and_rolls_back_failed_activation() {
    app_detach(true);
}

fn handoff(f: &Fixture, failure: bool) -> Value {
    let prepared =
        std::env::var_os("BBOXY_PREPARED_GUI_BUNDLE").expect("canonical prepared bundle");
    let mut command = Command::new("/usr/bin/python3");
    f.environment(&mut command);
    command
        .arg(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../scripts/reopen-gui.py"))
        .args(["--private", "--headless", "--prepared"])
        .arg(prepared)
        .arg("--destination")
        .arg(f.root.join("installed"))
        .arg("--run-dir")
        .arg(f.root.join("run"))
        .arg("--test-account")
        .arg(f.root.join("account"))
        .arg("--repository")
        .arg(f.root.join("repository"));
    if failure {
        command.arg("--verify-failure-after-detach");
    }
    let output = command.output().unwrap();
    assert_eq!(
        output.status.code(),
        Some(if failure { 2 } else { 0 }),
        "handoff output={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let receipt: Value = serde_json::from_slice(&output.stdout).unwrap();
    println!("rich-gui-handoff: {receipt}");
    assert_eq!(receipt["ok"], !failure);
    receipt
}

fn app_detach(replace: bool) {
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
    let mut owners = f.owners();
    // Exercise both unexpected UI loss and the canonical normal GUI replacement.
    let mut ui = f.app.take().unwrap();
    let ui_pid = ui.id();
    let before_gui = f.ctl(&["app", "status"])["gui"].clone();
    if replace {
        let receipt = handoff(&f, false);
        assert_eq!(receipt["before"]["gui"], before_gui);
        assert_ne!(receipt["after"]["gui"]["instance"], before_gui["instance"]);
        assert_eq!(
            receipt["ready"]["gui"]["source_revision"],
            receipt["installed_revision"]
        );
        assert_eq!(receipt["ready"]["gui"], f.ctl(&["app", "status"])["gui"]);
        assert_eq!(f.owners(), owners, "GUI handoff stopped a backend owner");
        assert_eq!(
            identity(&row(&f, &seat)),
            before,
            "GUI handoff replaced the seat"
        );
    } else {
        ui.kill().unwrap();
    }
    let exit = ui.wait().unwrap();
    let detached = Instant::now();
    while detached.elapsed() < Duration::from_secs(21) {
        assert_eq!(identity(&row(&f, &seat)), before);
        assert_eq!(unsafe { libc::kill(tool_pid, 0) }, 0, "turn died with UI");
        std::thread::sleep(Duration::from_millis(200));
    }
    if !replace {
        f.start();
    }
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
    for (index, event) in complete.iter().enumerate() {
        assert_eq!(
            event["seq"].as_u64(),
            Some(index as u64 + 1),
            "native durable prefix has a gap or duplicate"
        );
        assert_ne!(
            event["event"]["type"], "session_closed",
            "GUI detach closed its session"
        );
    }
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
    if replace {
        // Canonical reopen is now a hard UI-only operation. An incompatible
        // retained Logic must refuse before changing the GUI or any owner.
        let owner_path = f.root.join("run/logic.owner.json");
        let original_owner = fs::read(&owner_path).unwrap();
        let mut owner: Value = serde_json::from_slice(&original_owner).unwrap();
        let backend_births: Vec<Value> = ["supervisor", "logic"]
            .into_iter()
            .map(|name| {
                serde_json::from_slice::<Value>(
                    &fs::read(f.root.join(format!("run/{name}.owner.json"))).unwrap(),
                )
                .unwrap()["process"]
                    .clone()
            })
            .collect();
        let prior_gui = f.ctl(&["app", "status"])["gui"].clone();
        owner["ready"]["ui_control_version"] = json!(bbt_proto::gui::UI_CONTROL_VERSION - 1);
        fs::write(&owner_path, serde_json::to_vec(&owner).unwrap()).unwrap();
        let refused = handoff(&f, true);
        assert!(
            refused["error"].as_str().unwrap().contains("incompatible"),
            "{refused}"
        );
        assert_eq!(
            f.owners(),
            owners,
            "incompatible UI-only handoff replaced backend"
        );
        let after_births: Vec<Value> = ["supervisor", "logic"]
            .into_iter()
            .map(|name| {
                serde_json::from_slice::<Value>(
                    &fs::read(f.root.join(format!("run/{name}.owner.json"))).unwrap(),
                )
                .unwrap()["process"]
                    .clone()
            })
            .collect();
        assert_eq!(after_births, backend_births);
        assert_eq!(
            f.ctl(&["app", "status"])["gui"],
            prior_gui,
            "refusal detached the old GUI"
        );
        assert_eq!(
            identity(&row(&f, &seat)),
            before,
            "refusal replaced native session"
        );
        fs::write(&owner_path, original_owner).unwrap();
        assert_eq!(
            rich(&f, &seat, tail)["retained_items"],
            final_frame["retained_items"]
        );
        owners = f.owners();
        println!(
            "rich-gui-handoff: incompatible preserved owners refused before GUI disruption; exact PID/birth/session retained"
        );
        let active_gui = f.ctl(&["app", "status"])["gui"].clone();
        let contents = f.root.join("installed/bboxy450-term.app/Contents");
        let inode = std::os::unix::fs::MetadataExt::ino(&fs::metadata(&contents).unwrap());
        let failed = handoff(&f, true);
        assert_eq!(
            failed["restored"], true,
            "old installed contents were not restored: {failed}"
        );
        assert_eq!(
            std::os::unix::fs::MetadataExt::ino(&fs::metadata(&contents).unwrap()),
            inode,
            "rollback did not restore the original installed Contents inode"
        );
        assert!(
            failed["rollback"]["ready"]["gui_ready"] == true,
            "failed activation did not reopen the prior GUI: {failed}"
        );
        let rolled_back = f.ctl(&["app", "status"])["gui"].clone();
        assert_ne!(rolled_back["instance"], active_gui["instance"]);
        assert_eq!(
            rolled_back["source_revision"],
            active_gui["source_revision"]
        );
        assert_eq!(
            f.owners(),
            owners,
            "failed activation stopped a backend owner"
        );
        assert_eq!(
            identity(&row(&f, &seat)),
            before,
            "failed activation replaced the seat"
        );
        assert_eq!(
            rich(&f, &seat, tail)["retained_items"],
            final_frame["retained_items"]
        );
    }
    // Terminal is not the default for this announced session: reattachment must
    // retain an explicit Terminal preference as well as the rich replay cursor.
    f.ctl(&["ui", "click", &x, &y]);
    assert!(
        f.ctl(&["ui", "layout"])["rich_observation"]
            .get(&seat)
            .is_none()
    );
    if replace {
        handoff(&f, false);
    } else {
        let mut terminal_ui = f.app.take().unwrap();
        terminal_ui.kill().unwrap();
        terminal_ui.wait().unwrap();
        f.start();
    }
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
    assert_eq!(
        events(&log),
        complete,
        "GUI replacement/face toggles changed the durable session"
    );
    println!(
        "rich-app-detach: ui_pid={ui_pid} exit={exit} detached_ms={} owners={owners:?} identity={before} before={} after={} head={tail}",
        detached.elapsed().as_millis(),
        active["cursor"],
        final_frame["cursor"]
    );
}
