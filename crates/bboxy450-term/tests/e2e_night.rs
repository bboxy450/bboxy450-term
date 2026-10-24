#![allow(
    clippy::disallowed_methods,
    reason = "test/fixture IO, not the UI thread"
)]
//! The e2e night runner (#282): every promise of the group gets its own app and script; the
//! script's exit code is the verdict; a promise without a script is untested; the summary
//! is green only when all passed, and the exit code follows it. Every promise's e2e record
//! is written through `record write`: created on the first night, updated on the next.
use serde_json::Value;
use std::process::Command;

#[test]
fn the_runner_turns_script_exit_codes_into_verdicts_and_a_summary() {
    let scripts = tempfile::tempdir().unwrap();
    // T1 passes only if ctl reaches this promise's own app; T2 fails with its reason.
    std::fs::write(
        scripts.path().join("T1.sh"),
        "\"$BBT\" ctl seat ls >/dev/null && [ -d \"$E2E_WORK\" ] && [ -f \"$E2E_PIDS\" ]\n",
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
        l["function"] == "T2"
            && l["evidence"]
                .as_str()
                .unwrap()
                .contains("the fixture says no")
    }));
    for id in ["T3", "T4", "T5", "T6", "T7", "T8"] {
        assert_eq!(verdict(id), "untested", "{id}");
    }
    let summary = lines.last().unwrap();
    assert_eq!(summary["kind"], "summary");
    assert_eq!(summary["state"], "red");
    assert_eq!(summary["passed"], 1);
    assert_eq!(summary["total"], 8);
    assert_eq!(summary["unwritten"], serde_json::json!([]), "{lines:?}");
    assert_eq!(output.status.code(), Some(1), "red exits 1");
    let written: Vec<&Value> = lines.iter().filter(|l| l["kind"] == "record").collect();
    assert_eq!(written.len(), 8, "{written:?}");
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
    // The failure got one fix task, open and unassigned, linked tracked-by; untested rows
    // keep the night red.
    let task = written.iter().find(|l| l["function"] == "T2").unwrap()["tracked_by"].clone();
    assert!(task.is_string(), "{written:?}");
    assert!(
        first.contains(&format!("to: '[[{}]]'", task.as_str().unwrap())),
        "{first}"
    );
    assert_eq!(summary["tracked"], serde_json::json!(["T2"]));
    assert!(
        written
            .iter()
            .filter(|l| l["function"] != "T2")
            .all(|l| l["tracked_by"].is_null())
    );
    // The next night updates the same records under their etag: same numbers, new run.
    let again = night();
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
    assert_eq!(tasks, 1, "one fix task across both nights");
    assert!(record("T2").contains("updated_by:"), "{}", record("T2"));
}
