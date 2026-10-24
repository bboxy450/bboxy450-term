#![allow(
    clippy::disallowed_methods,
    reason = "test/fixture IO, not the UI thread"
)]
//! The e2e night runner (#282): every promise of the group gets its own app and script; the
//! script's exit code is the verdict; a promise without a script is untested; the summary
//! is green only when all passed, and the exit code follows it.
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
    let output = Command::new(env!("CARGO_BIN_EXE_bboxy450-term"))
        .args(["e2e", "night", "--group", "terminal", "--scripts"])
        .arg(scripts.path())
        .stdin(std::process::Stdio::null())
        .output()
        .unwrap();
    let lines: Vec<Value> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    let verdict = |id: &str| {
        lines
            .iter()
            .find(|l| l["function"] == id)
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
    assert_eq!(output.status.code(), Some(1), "red exits 1");
}
