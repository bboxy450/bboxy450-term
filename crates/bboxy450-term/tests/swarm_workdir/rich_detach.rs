// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Actual App, native rich actor, independent backend and real scripted harness turn.
use super::*;
use serde_json::json;

#[track_caller]
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
    rich_frame(f, seat, min_seq)["rich_observation"][seat].clone()
}
fn rich_frame(f: &Fixture, seat: &str, min_seq: u64) -> Value {
    wait(|| {
        let frame = f.ctl(&["ui", "layout"]);
        let rich = &frame["rich_observation"][seat];
        (rich["ready"] == true
            && rich["paint_count"].as_u64().unwrap_or(0) > 0
            && rich["cursor"]["seq"].as_u64().unwrap_or(0) >= min_seq)
            .then_some(frame)
    })
}
fn at_rich_tail(rich: &Value) -> bool {
    rich["scroll"][0] == rich["scroll"][1]
}
fn rich_pane_rect(frame: &Value, seat: &str) -> [f64; 4] {
    let pane = frame["panes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["seat"] == seat)
        .unwrap();
    std::array::from_fn(|index| pane["rect"][index].as_f64().unwrap())
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
#[ignore = "private App --capture witness needs BBOXY_HARNESS_TEST_BIN"]
fn rich_app_images_use_the_swarm_root_and_paint_the_png() {
    let binary = PathBuf::from(std::env::var_os("BBOXY_HARNESS_TEST_BIN").unwrap());
    assert!(binary.is_absolute() && binary.is_file());
    let mut f = Fixture::with_directory(
        tempfile::Builder::new()
            .prefix("rich-images-")
            .tempdir_in("/tmp")
            .unwrap(),
    );
    fs::copy(BIN, f.root.join("bin/bboxy450-term")).unwrap();
    let repository = f.root.join("repository").canonicalize().unwrap();
    let images = repository.join(".bboxy/images");
    fs::create_dir_all(&images).unwrap();
    let signature = [17_u8, 241, 131, 255];
    let mut encoder =
        png::Encoder::new(fs::File::create(images.join("probe.png")).unwrap(), 96, 64);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .unwrap()
        .write_image_data(&signature.repeat(96 * 64))
        .unwrap();

    let response = f.root.join("image.sse");
    let delta = json!({"choices":[{"index":0,"delta":{"content":"Before image.

![PNG witness](images/probe.png)

After image."},"finish_reason":"stop"}]});
    fs::write(&response, format!("data: {delta}\n\ndata: [DONE]\n\n")).unwrap();
    let shim = f.root.join("bin/bboxy450-harness");
    let quote = |path: &Path| format!("'{}'", path.to_str().unwrap().replace('\'', "'\\''"));
    fs::write(
        &shim,
        format!(
            "#!/bin/sh\nexec {} \"$@\" --scripted {} --yolo\n",
            quote(&binary),
            quote(&response)
        ),
    )
    .unwrap();
    fs::set_permissions(&shim, fs::Permissions::from_mode(0o700)).unwrap();

    let capture = f.root.join("transcript-images.png");
    let log = fs::File::create(f.root.join("app.log")).unwrap();
    f.app = Some(
        f.command()
            .args(["--headless", "--ctl", "--unattended", "--test-account"])
            .arg(f.root.join("account"))
            .arg("--run-dir")
            .arg(f.root.join("run"))
            .arg("--repository")
            .arg(&repository)
            .arg("--capture")
            .arg(&capture)
            .args(["--capture-after", "15"])
            .stdout(log.try_clone().unwrap())
            .stderr(log)
            .spawn()
            .unwrap(),
    );
    wait(|| {
        assert!(
            f.app.as_mut().unwrap().try_wait().unwrap().is_none(),
            "{}",
            f.log()
        );
        if !f.root.join("run/gui.endpoint").exists() {
            return None;
        }
        let output = f
            .command()
            .args(["ctl", "app", "status", "--run-dir"])
            .arg(f.root.join("run"))
            .output()
            .ok()?;
        let status: Value = serde_json::from_slice(&output.stdout).ok()?;
        (status["data"]["gui_ready"] == true && status["data"]["backend_ready"] == true)
            .then_some(())
    });
    let swarm = f.ctl(&["ui", "layout"])["swarm"]
        .as_str()
        .unwrap()
        .to_owned();
    f.ctl(&["ui", "swarm-workdir", &swarm, repository.to_str().unwrap()]);
    let seat = f.spawn("bboxy450-harness", &swarm, None);
    let initial = rich(&f, &seat, 1);
    submit(&f, "Show the PNG from this workspace.");
    let final_frame = rich(&f, &seat, initial["cursor"]["seq"].as_u64().unwrap() + 3);
    assert!(final_frame["error"].is_null(), "{final_frame}");

    let pixels = wait(|| {
        let file = fs::File::open(&capture).ok()?;
        let mut reader = png::Decoder::new(std::io::BufReader::new(file))
            .read_info()
            .ok()?;
        let mut rgba = vec![0; reader.output_buffer_size()?];
        let image = reader.next_frame(&mut rgba).ok()?;
        if image.color_type != png::ColorType::Rgba {
            return None;
        }
        let (mut left, mut top, mut right, mut bottom) = (u32::MAX, u32::MAX, 0, 0);
        let mut pixels = 0;
        for (index, pixel) in rgba[..image.buffer_size()]
            .as_chunks::<4>()
            .0
            .iter()
            .enumerate()
        {
            if pixel
                .iter()
                .zip(signature)
                .all(|(actual, expected)| actual.abs_diff(expected) <= 3)
            {
                let index = index as u32;
                let (x, y) = (index % image.width, index / image.width);
                left = left.min(x);
                top = top.min(y);
                right = right.max(x);
                bottom = bottom.max(y);
                pixels += 1;
            }
        }
        if pixels >= 1024 {
            let (width, height) = (right - left + 1, bottom - top + 1);
            assert!(
                width.abs_diff(96) <= 2 && height.abs_diff(64) <= 2,
                "inline image stretched to {width}x{height}"
            );
            Some(pixels)
        } else {
            None
        }
    });
    println!("captured {pixels} probe-colour pixels from the actual transcript");
    if let Some(directory) = std::env::var_os("BBOXY_TRANSCRIPT_IMAGE_EVIDENCE") {
        let directory = PathBuf::from(directory);
        fs::create_dir_all(&directory).unwrap();
        fs::copy(&capture, directory.join("transcript-images.png")).unwrap();
        fs::write(
            directory.join("transcript-images.json"),
            serde_json::to_vec_pretty(&final_frame).unwrap(),
        )
        .unwrap();
    }
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
    let full_frame = wait(|| {
        let frame = rich_frame(&f, &seat, 1);
        let r = &frame["rich_observation"][&seat];
        // Scroll from a settled baseline so tail-follow cannot outrun the wheel.
        (at_rich_tail(r) && r["scroll"][1].as_f64().unwrap() > 100.).then_some(frame)
    });
    let full = &full_frame["rich_observation"][&seat];
    let [x, y, width, height] = rich_pane_rect(&full_frame, &seat);
    let x = (x + width / 2.) as u32;
    let y = (y + height / 2.) as u32;
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
    let approval_frame = wait(|| {
        let frame = rich_frame(&f, &seat, 1);
        let r = &frame["rich_observation"][&seat];
        // Tail-follow can expose a clipped button while it is still moving.
        (at_rich_tail(r) && r["approval_choices"][0].is_array()).then_some(frame)
    });
    assert!(!marker.exists(), "tool ran before the App approval");
    let approval = &approval_frame["rich_observation"][&seat];
    let [x, y, ..] = rich_pane_rect(&approval_frame, &seat);
    let choice = approval["approval_choices"][0].as_array().unwrap();
    let ax = (x + choice[0].as_f64().unwrap() + choice[2].as_f64().unwrap() / 2.) as u32;
    let ay = (y + choice[1].as_f64().unwrap() + choice[3].as_f64().unwrap() / 2.) as u32;
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
        "rich-app-input: seat={seat} composer_turns=2 wheel_before={} wheel_after={} approval_scroll={} approval_click=[{ax},{ay}] tool=approved identity={identity_before}",
        full["scroll"], scrolled["scroll"], approval["scroll"]
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
    // The view switch ends 70 px from the pane's right edge: the automation
    // bolt (13 px and an 8 px gap) sits between it and mute.
    let x = (rect[0].as_f64().unwrap() + rect[2].as_f64().unwrap() - 73.)
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

#[test]
#[ignore = "make test-chart-artifacts supplies the private unsigned install"]
fn chart_artifact_live_and_pinned_images_paint_in_private_app() {
    let install = PathBuf::from(std::env::var_os("BBOXY_CHART_TEST_INSTALL").unwrap());
    let contents = install.join("bboxy450-term.app/Contents");
    let harness = contents.join("MacOS/bboxy450-harness");
    let mut f = Fixture::with_directory(
        tempfile::Builder::new()
            .prefix("chart-app-")
            .tempdir_in("/tmp")
            .unwrap(),
    );
    // Keep the installed bundle intact. The isolated copy replaces only the
    // bundled harness entry point, to feed scripts to the installed harness.
    let private_contents = f.root.join("bboxy450-term.app/Contents");
    let bin = private_contents.join("MacOS");
    fs::create_dir_all(&bin).unwrap();
    let app = bin.join("bboxy450-term");
    fs::copy(contents.join("MacOS/bboxy450-term"), &app).unwrap();
    std::os::unix::fs::symlink(
        contents.join("Resources"),
        private_contents.join("Resources"),
    )
    .unwrap();
    f.binary = Some(app);
    let support = Path::new("Library/Application Support/bboxy450-term");
    let private_support = f.root.join("home").join(support);
    fs::create_dir_all(&private_support).unwrap();
    std::os::unix::fs::symlink(
        install.join("home").join(support).join("chart-runtime"),
        private_support.join("chart-runtime"),
    )
    .unwrap();
    let root = f.root.join("repository");
    fs::create_dir_all(root.join(".bboxy/notes")).unwrap();
    let shared = r#"
import altair as alt
import json, time
from pathlib import Path
def chart(colour):
    return alt.Chart(alt.Data(values=[{'label':'A','value':3}, {'label':'B','value':5}])).mark_bar(color=colour).encode(x='label:N',y='value:Q').properties(title='Native chart',width=160,height=74)
"#;
    let first = r#"
import sys
from matplotlib.figure import Figure
assert 'chart-runtime-' in sys.prefix, sys.executable
figure = Figure(figsize=(3.5, 2))
figure.subplots().plot([1, 2, 3], [1, 3, 2], color='#175ccb', linewidth=4)
line = display(figure)
saved = display(chart('#11f183'))
bboxy.knowledge_write(path='notes/live-chart.md',
    frontmatter={'type':'note','title':'Private chart revisions'},
    body='# Chart revisions\n\n' + saved.reference + '\n\n' + saved.snapshot_reference + '\n')
Path('chart-1.json').write_text(json.dumps(saved))
Path('matplotlib.json').write_text(json.dumps(line))
"#;
    let update = |revision: u64, colour: &str| {
        format!(
            r#"
previous = json.loads(Path('chart-{}.json').read_text())
for _ in range(500):
    if Path('publish-{revision}').is_file():
        break
    time.sleep(.02)
else:
    raise RuntimeError('fixture update did not receive its trigger')
saved = display(chart('{colour}'), update=previous)
Path('chart-{revision}.json').write_text(json.dumps(saved))
"#,
            revision - 1
        )
    };
    let turns = [first.to_owned(), update(2, "#bb2484"), update(3, "#11f183")];
    let mut scripts = Vec::new();
    for (index, source) in turns.iter().enumerate() {
        let args = json!({"source":format!("{shared}{source}"),"wait_ms":25000}).to_string();
        let invocation = json!({"choices":[{"index":0,"delta":{"tool_calls":[{
            "index":0,"id":format!("chart-{index}"),"type":"function",
            "function":{"name":"mcp__bboxy_python__python_execute","arguments":args}
        }]},"finish_reason":"tool_calls"}]});
        let ready = json!({"choices":[{"index":0,"delta":{"content":"Chart saved."},"finish_reason":"stop"}]});
        for (name, event) in [("call", invocation), ("ready", ready)] {
            let path = f.root.join(format!("chart-{index}-{name}.sse"));
            fs::write(&path, format!("data: {event}\n\ndata: [DONE]\n\n")).unwrap();
            scripts.extend(["--scripted".to_owned(), path.to_string_lossy().into_owned()]);
        }
    }
    let shim = bin.join("bboxy450-harness");
    fs::write(&shim, format!(
        "#!/usr/bin/python3\nimport os,sys\nos.environ['MPLCONFIGDIR']={0}\nos.execv({1},[{1},*sys.argv[1:],'--yolo',*{2}])\n",
        json!(f.root.join("mpl-cache")), json!(harness), json!(scripts)
    )).unwrap();
    fs::set_permissions(shim, fs::Permissions::from_mode(0o700)).unwrap();
    f.start();
    let swarm = f.ctl(&["ui", "layout"])["swarm"]
        .as_str()
        .unwrap()
        .to_owned();
    f.ctl(&["ui", "swarm-workdir", &swarm, root.to_str().unwrap()]);
    let seat = f.spawn("bboxy450-harness", &swarm, None);
    rich(&f, &seat, 1);
    let seat_identity = identity(&row(&f, &seat));
    let session = seat_identity["session"].as_str().unwrap();
    let transcript = f
        .root
        .join(format!("home/.bboxy450-harness/sessions/{session}.jsonl"));
    let submit_chart = |previous_turns, prompt: &str| {
        f.ctl(&["ui", "focus", &seat]);
        wait(|| {
            let pane = rich(&f, &seat, 1);
            let ended = events(&transcript)
                .iter()
                .filter(|row| row["event"]["type"] == "turn_ended")
                .count();
            (pane["focused"] == true && pane["ime_caret"].is_array() && ended == previous_turns)
                .then_some(())
        });
        submit(&f, prompt);
        wait(|| {
            events(&transcript)
                .iter()
                .any(|row| {
                    let body = &row["event"]["body"];
                    body["kind"] == "user_message"
                        && body["blocks"].as_array().is_some_and(|blocks| {
                            blocks.iter().any(|block| block["text"] == prompt)
                        })
                })
                .then_some(())
        });
    };
    let saved = |revision| {
        let path = root.join(format!("chart-{revision}.json"));
        let deadline = Instant::now() + Duration::from_secs(35);
        loop {
            if let Some(value) = fs::read(&path)
                .ok()
                .and_then(|raw| serde_json::from_slice::<Value>(&raw).ok())
            {
                break value;
            }
            assert!(
                Instant::now() < deadline,
                "chart revision {revision} was not stored: {}",
                f.log()
            );
            std::thread::sleep(Duration::from_millis(20));
        }
    };
    let evidence = std::env::var_os("BBOXY_CHART_EVIDENCE").map(PathBuf::from);
    let capture_regions = |name: &str, regions: &[([u8; 3], std::ops::Range<usize>)]| {
        let image = f.root.join(format!("view-{name}.png"));
        wait(|| {
            f.ctl(&["seat", "capture", "window", image.to_str().unwrap()]);
            let file = fs::File::open(&image).ok()?;
            let mut reader = png::Decoder::new(std::io::BufReader::new(file))
                .read_info()
                .ok()?;
            let mut rgb = vec![0; reader.output_buffer_size()?];
            let info = reader.next_frame(&mut rgb).ok()?;
            let channels = match info.color_type {
                png::ColorType::Rgb => 3,
                png::ColorType::Rgba => 4,
                _ => return None,
            };
            let width = info.width as usize;
            let counts: Vec<_> = regions
                .iter()
                .map(|(colour, columns)| {
                    rgb[..info.buffer_size()]
                        .chunks_exact(channels)
                        .enumerate()
                        .filter(|(index, pixel)| {
                            columns.contains(&(index % width * 100 / width))
                                && pixel[..3]
                                    .iter()
                                    .zip(colour)
                                    .all(|(a, b)| a.abs_diff(*b) <= 3)
                        })
                        .count()
                })
                .collect();
            if !counts.iter().all(|count| *count > 250) {
                return None;
            }
            if let Some(dir) = &evidence {
                fs::create_dir_all(dir).unwrap();
                fs::copy(&image, dir.join(format!("{name}.png"))).unwrap();
            }
            println!("chart {name}: {counts:?} paint pixels");
            Some(counts)
        })
    };
    let capture = |name: &str, colours: &[[u8; 3]]| {
        let regions: Vec<_> = colours.iter().map(|colour| (*colour, 0..100)).collect();
        capture_regions(name, &regions)
    };
    let a = [17, 241, 131];
    let b = [187, 36, 132];
    submit_chart(0, "Save the first chart.");
    let one = saved(1);
    let line: Value = wait(|| {
        let bytes = fs::read(root.join("matplotlib.json")).ok()?;
        serde_json::from_slice(&bytes).ok()
    });
    assert_eq!(line["media_type"], "image/svg+xml");
    assert_eq!(one["media_type"], "image/svg+xml");
    let id = one["id"].as_str().unwrap();
    let line_id = line["id"].as_str().unwrap();
    assert_ne!(line_id, id);
    wait(|| {
        let transcript = fs::read_to_string(&transcript).ok()?;
        (transcript.contains(one["reference"].as_str().unwrap())
            && transcript.contains(line["reference"].as_str().unwrap()))
        .then_some(())
    });
    println!("installed charts: matplotlib={line_id} altair={id}");
    capture("transcript-installed-charts", &[a, [23, 92, 203]]);
    let pinned = format!("{id}:1");
    let document_label = |revision: u64| {
        let pin = format!("artifact:{id}:{revision}");
        wait(|| {
            let frame = f.ctl(&["ui", "layout"]);
            let observed = &frame["document_observation"];
            (observed["source"]["path"] == "notes/live-chart.md"
                && observed["frame_seq"] == frame["frame_seq"]
                && observed["texts"]
                    .as_array()?
                    .iter()
                    .any(|run| run["text"] == pin))
            .then_some(())
        });
    };
    capture("transcript-one", &[a]);
    f.ctl(&["ui", "open", "docs", "notes/live-chart.md"]);
    document_label(1);
    capture("document-one", &[a]);
    f.ctl(&["ui", "key", "esc"]);
    let canvas = |verb: &str, arguments: &Value| {
        f.ctl(&["canvas", verb, &arguments.to_string(), "--swarm", &swarm])
    };
    let drawing = canvas("open", &json!({"title":"Private image placements"}));
    let drawing_id = &drawing["id"];
    let placement = |id, artifact, x| {
        json!({"op":"add","element":{"id":id,"tags":[],"kind":null,
        "content":{"type":"image","bounds":{"origin":[x,0],"size":[300,200]},"artifact":artifact}}})
    };
    canvas(
        "draw",
        &json!({"id":drawing_id,"ops":[placement("live",id,0),placement("pinned",&pinned,330)]}),
    );
    capture("canvas-one", &[a]);

    submit_chart(1, "Update the chart to B.");
    f.ctl(&["ui", "open", "docs", "notes/live-chart.md"]);
    capture("document-before-update", &[a]);
    fs::write(root.join("publish-2"), b"publish").unwrap();
    let two = saved(2);
    assert_eq!(two["id"], id);
    assert_eq!(two["revision"], 2);
    document_label(2);
    document_label(1);
    capture("document-two-unchanged-text", &[a, b]);
    f.ctl(&["ui", "key", "esc"]);
    canvas("open", &json!({"id":drawing_id}));
    capture("canvas-two", &[a, b]);
    let zoom = || {
        let frame = f.ctl(&["ui", "layout"]);
        frame["canvases"]
            .as_object()?
            .values()
            .find(|row| row["source"]["id"] == *drawing_id)?["camera"][2]
            .as_f64()
    };
    let fitted = wait(&zoom);
    f.ctl(&["ui", "key", "=", "="]);
    wait(|| zoom().filter(|scale| *scale > fitted));
    capture("canvas-two-zoomed", &[a, b]);
    f.ctl(&["ui", "key", "f"]);
    let exported = canvas("read", &json!({"id":drawing_id,"image":true}));
    assert_eq!(
        exported["image"]["artifacts"][id]["pin"],
        format!("artifact:{id}:2")
    );
    assert_eq!(
        exported["image"]["artifacts"][&pinned]["pin"],
        format!("artifact:{id}:1")
    );
    assert!(
        exported["image"]["artifacts"][id]["resource"]["uri"]
            .as_str()
            .unwrap()
            .ends_with("/2")
    );
    assert!(
        exported["image"]["font_set"]
            .as_str()
            .unwrap()
            .starts_with("sha256:")
    );
    assert!(exported["image"]["renderer"].is_string());

    submit_chart(2, "Return to the first chart pixels.");
    fs::write(root.join("publish-3"), b"publish").unwrap();
    let three = saved(3);
    assert_eq!(three["revision"], 3);
    assert_eq!(three["digest"], one["digest"]);
    canvas("open", &json!({"id":drawing_id}));
    // The pinned green chart is visible even while the live chart still shows B.
    // Require green on both sides of the fixed private window before capturing A again.
    capture_regions("canvas-three", &[(a, 25..50), (a, 75..100)]);
    let finished = canvas("read", &json!({"id":drawing_id,"image":true}));
    assert_eq!(
        finished["image"]["artifacts"][id]["pin"],
        format!("artifact:{id}:3")
    );
    assert_eq!(finished["image"]["artifacts"][id]["digest"], one["digest"]);
    assert_eq!(
        finished["image"]["artifacts"][&pinned]["pin"],
        format!("artifact:{id}:1")
    );
    if let Some(dir) = &evidence {
        fs::write(
            dir.join("chart-canvas-revisions.json"),
            serde_json::to_vec_pretty(
                &json!({"before":exported["image"],"after":finished["image"]}),
            )
            .unwrap(),
        )
        .unwrap();
    }
}
