// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
#![allow(
    clippy::disallowed_methods,
    reason = "test/fixture IO, not the UI thread"
)]
//! The e2e night runner (#282): every promise of the group gets its own app and script; the
//! script's exit code is the verdict; a promise without a script is untested; the summary
//! is green only when all passed, and the exit code follows it. Every promise's e2e record
//! is written through `record write`: created on the first night, updated on the next.
use serde_json::Value;
use std::{os::unix::fs::PermissionsExt, process::Command};

fn assert_night_apps_stopped(output: &std::process::Output) {
    for app in String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter(|line| line["kind"] == "app")
    {
        let run = std::path::Path::new(app["run_dir"].as_str().unwrap());
        assert!(
            !run.join(bbt_mux::ctl_door::ENDPOINT).exists(),
            "night left its control endpoint: {app}"
        );
        let pid: libc::pid_t = app["pid"].as_u64().unwrap().try_into().unwrap();
        // Hang guard only; successful PID absence determines completion.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
        loop {
            // SAFETY: signal zero only queries this night's recorded app PID;
            // it neither signals the process nor supplies a PID-kill fallback.
            if unsafe { libc::kill(pid, 0) } < 0 {
                assert_eq!(
                    std::io::Error::last_os_error().raw_os_error(),
                    Some(libc::ESRCH)
                );
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "retained evidence kept an app alive: {app}"
            );
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
}

// keeps: spec e2e-acceptance §3 (night verdict, signed record and custody boundary)
#[test]
#[ignore = "night: real process boundary, test-policy #327 B2"]
fn the_runner_turns_script_exit_codes_into_verdicts_and_a_summary() {
    let scripts = tempfile::tempdir().unwrap();
    let bin = scripts.path().join("T1.fixture-bin");
    std::fs::create_dir(&bin).unwrap();
    for (name, body) in [
        (
            "claude",
            "#!/bin/sh\ne2e_private_probe\nwhile read -r line; do :; done\n",
        ),
        ("e2e_private_probe", "#!/bin/sh\necho E2E_FIXTURE_ONLY\n"),
    ] {
        let path = bin.join(name);
        std::fs::write(&path, body).unwrap();
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).unwrap();
    }
    // T1 passes in full only if ctl reaches this promise's own app; T2 fails with its reason;
    // T3 passes but says it proves only part of its promise (#316).
    std::fs::write(
        scripts.path().join("T1.sh"),
        r#"# coverage: full
set -eu
"$BBT" ctl seat ls >/dev/null
[ -d "$E2E_WORK" ] && kill -0 "$E2E_APP"
seat=$("$BBT" ctl seat spawn claude --role executor --place tab --cwd "$E2E_WORK" | python3 -I -c 'import json,sys; print(json.load(sys.stdin)["data"]["seat"])')
"$BBT" ctl seat wait "$seat" --text '(?m)^E2E_FIXTURE_ONLY$' --timeout 20000
"#,
    )
    .unwrap();
    std::fs::write(
        scripts.path().join("T3.sh"),
        r#"# coverage: partial; missing: resizing a pane; no stale cells
set -eu
seat=$("$BBT" ctl seat spawn zsh --place tab --cwd "$E2E_WORK" | python3 -I -c 'import json,sys; print(json.load(sys.stdin)["data"]["seat"])')
"$BBT" ctl seat type "$seat" 'command -v e2e_private_probe >/dev/null && echo E2E_FIXTURE_LEAKED || echo E2E_FIXTURE_ABSENT'
"$BBT" ctl seat key "$seat" enter
"$BBT" ctl seat wait "$seat" --text '(?m)^E2E_FIXTURE_ABSENT$' --timeout 20000
"#,
    )
    .unwrap();
    std::fs::write(
        scripts.path().join("T2.sh"),
        "echo 'FAIL: the fixture says no' >&2; exit 1\n",
    )
    .unwrap();
    let records = tempfile::tempdir().unwrap();
    let night = || {
        Command::new(env!("CARGO_BIN_EXE_bboxy450-term"))
            .args(["e2e", "night", "--group", "terminal", "--scripts"])
            .arg(scripts.path())
            .arg("--records")
            .arg(records.path())
            .stdin(std::process::Stdio::null())
            .output()
            .unwrap()
    };
    let output = night();
    assert_night_apps_stopped(&output);
    let lines: Vec<Value> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    let verdict = |id: &str| {
        lines
            .iter()
            .find(|l| l["kind"] == "e2e" && l["function"] == id)
            .map(|l| l["verdict"].as_str().unwrap().to_owned())
            .unwrap()
    };
    assert_eq!(verdict("T1"), "pass", "{lines:?}");
    assert_eq!(verdict("T2"), "fail");
    assert!(lines.iter().any(|l| {
        l["kind"] == "e2e"
            && l["function"] == "T2"
            && l["evidence"]
                .as_str()
                .unwrap()
                .contains("the fixture says no")
    }));
    assert_eq!(verdict("T3"), "pass");
    for id in ["T4", "T5", "T6", "T7", "T8", "T9"] {
        assert_eq!(verdict(id), "untested", "{id}");
    }
    let summary = lines.last().unwrap();
    let kept = std::path::PathBuf::from(summary["kept"].as_str().expect("failed run retained"));
    assert_eq!(
        std::fs::metadata(&kept).unwrap().permissions().mode() & 0o077,
        0,
        "retained root is private: {kept:?}"
    );
    assert!(
        std::fs::read_to_string(kept.join("T2/work/output.log"))
            .unwrap()
            .contains("FAIL: the fixture says no")
    );
    assert_eq!(summary["kind"], "summary");
    assert_eq!(summary["state"], "red");
    assert_eq!(summary["passed"], 2);
    assert_eq!(summary["partial"], serde_json::json!(["T3"]));
    assert_eq!(summary["total"], 9);
    assert_eq!(summary["unwritten"], serde_json::json!([]), "{lines:?}");
    // One line per app the night started (invariant #283): each promise with a script, and
    // the records app; every one on file custody with no window.
    let apps: Vec<&Value> = lines.iter().filter(|l| l["kind"] == "app").collect();
    assert_eq!(
        apps.iter().filter(|a| a["role"] == "promise").count(),
        3,
        "{apps:?}"
    );
    assert_eq!(
        apps.iter().filter(|a| a["role"] == "records").count(),
        1,
        "{apps:?}"
    );
    for app in &apps {
        assert_eq!(
            (&app["custody_backend"], &app["windows"]),
            (&serde_json::json!("file"), &serde_json::json!(0)),
            "{app}"
        );
        assert!(
            app["issued_by"]
                .as_str()
                .unwrap()
                .starts_with("test-account:"),
            "{app}"
        );
    }
    assert_eq!(output.status.code(), Some(1), "red exits 1");
    let written: Vec<&Value> = lines.iter().filter(|l| l["kind"] == "record").collect();
    assert_eq!(written.len(), 9, "{written:?}");
    let record = |id: &str| {
        std::fs::read_to_string(records.path().join(format!(".bboxy/e2e/{id}.md"))).unwrap()
    };
    let first = record("T2");
    for field in ["type: e2e", "function: T2", "status: fail"] {
        assert!(first.contains(field), "{field}: {first}");
    }
    let label = first
        .lines()
        .find_map(|line| line.strip_prefix("run_by: "))
        .unwrap();
    assert!(label.starts_with("test-account:"), "{first}");
    assert!(
        first.matches(&format!("issued_by: {label}")).count() >= 2,
        "{first}"
    );
    assert!(
        first.contains("authorship:") && first.contains("sig:"),
        "signed: {first}"
    );
    assert!(record("T7").contains("status: untested"));
    let full = record("T1");
    assert!(full.contains("coverage: full"), "{full}");
    let partial = record("T3");
    for field in [
        "status: pass",
        "coverage: partial",
        "- resizing a pane",
        "- no stale cells",
    ] {
        assert!(partial.contains(field), "{field}: {partial}");
    }
    // The failure got one fix task, open and unassigned, linked tracked-by; untested rows
    // keep the night red.
    let task = written.iter().find(|l| l["function"] == "T2").unwrap()["tracked_by"].clone();
    assert!(task.is_string(), "{written:?}");
    assert!(
        first.contains(&format!("to: '[[{}]]'", task.as_str().unwrap())),
        "{first}"
    );
    // The partial pass got its own task naming the missing assertions.
    assert_eq!(summary["tracked"], serde_json::json!(["T2", "T3"]));
    assert!(
        written
            .iter()
            .filter(|l| l["function"] != "T2" && l["function"] != "T3")
            .all(|l| l["tracked_by"].is_null())
    );
    // The next night updates the same records under their etag: same numbers, new run.
    let again = night();
    assert_night_apps_stopped(&again);
    let second: Vec<Value> = String::from_utf8_lossy(&again.stdout)
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .filter(|l: &Value| l["kind"] == "record")
        .collect();
    for (a, b) in written.iter().zip(&second) {
        assert_eq!(a["number"], b["number"], "{second:?}");
        assert!(b.get("error").is_none(), "{b}");
        // A failure an open task already tracks gets no second task.
        assert_eq!(a["tracked_by"], b["tracked_by"], "{second:?}");
    }
    let tasks = std::fs::read_dir(records.path().join(".bboxy/tasks"))
        .unwrap()
        .count();
    assert_eq!(
        tasks, 2,
        "one fix task and one cover task across both nights"
    );
    assert!(record("T2").contains("updated_by:"), "{}", record("T2"));
    let kept_again = String::from_utf8_lossy(&again.stdout)
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .find(|line| line["kind"] == "summary")
        .unwrap()["kept"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_ne!(kept, std::path::Path::new(&kept_again));
    assert!(
        std::path::Path::new(&kept_again)
            .join("T2/work/output.log")
            .is_file()
    );
    // These are the two roots the test's own child nights retained, not shared state.
    std::fs::remove_dir_all(kept).unwrap();
    std::fs::remove_dir_all(kept_again).unwrap();
}
