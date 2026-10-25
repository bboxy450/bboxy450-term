#![allow(
    clippy::disallowed_methods,
    reason = "R5 integration test owns this child and its temporary artifacts"
)]
use std::{
    process::{Command, Stdio},
    time::{Duration, Instant},
};

#[test]
fn synthetic_runtime_contract_keeps_fifteen_encoded_panes_live_under_mixed_io() {
    let root = tempfile::Builder::new()
        .prefix("r5-contract-")
        .tempdir()
        .unwrap();
    let report = root.path().join("report.json");
    let stdout = root.path().join("stdout.log");
    let stderr = root.path().join("stderr.log");
    let mut child = Command::new(env!("CARGO_BIN_EXE_bboxy450-term"))
        .args([
            "--stall-test",
            "--stall-headless",
            "--stall-seats",
            "1000",
            "--stall-watched",
            "15",
            "--stall-seconds",
            "3",
            "--stall-idle",
            "1",
            "--stall-report",
        ])
        .arg(&report)
        .arg("--stall-run-dir")
        .arg(root.path().join("fixture"))
        .env("BBOXY_TEST_CUSTODY", "memory")
        .stdout(Stdio::from(std::fs::File::create(&stdout).unwrap()))
        .stderr(Stdio::from(std::fs::File::create(&stderr).unwrap()))
        .spawn()
        .unwrap();
    let pid = child.id();
    std::fs::write(root.path().join("owned-pid"), pid.to_string()).unwrap();
    let start = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if start.elapsed() > Duration::from_secs(90) {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!(
                "R5 child{pid} timed out; stderr:{}",
                std::fs::read_to_string(&stderr).unwrap()
            );
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    let output = std::fs::read_to_string(&stdout).unwrap();
    let errors = std::fs::read_to_string(&stderr).unwrap();
    assert!(
        status.success(),
        "{status}; stdout:{output}; stderr:{errors}"
    );
    let value: serde_json::Value = serde_json::from_slice(&std::fs::read(report).unwrap()).unwrap();
    assert_eq!(value["kind"], "synthetic-render-state");
    assert_eq!(value["rows"], 1000);
    assert_eq!(value["real_ptys"], 0);
    assert_eq!(value["watched"], 15);
    assert_eq!(value["stdout_bytes"], 1_000_000_000u64);
    assert!(value["fault_read_ms"].as_f64().unwrap() >= 2000.);
    assert!(value["bus_published"].as_u64().unwrap() > 0);
    assert!(value["bus_delivered"].as_u64().unwrap() > 0);
    assert_eq!(value["timing_gated"], false);
    assert_eq!(value["metrics"]["probe_clipped"], false);
    assert!(value["synthetic_states_consumed"].as_u64().unwrap() >= 1000);
    assert_eq!(
        value["metrics"]["encoded_changes"]
            .as_object()
            .unwrap()
            .len(),
        15
    );
    assert_eq!(value["metrics"]["idle_frames"], 0);
    assert_eq!(value["metrics"]["guard_violations"], 0);
    assert_eq!(value["failures"], serde_json::json!([]));
    let diagnostic = &value["metrics"]["diagnostics"];
    assert_eq!(diagnostic["enabled"], true);
    assert!(diagnostic["read_ring_syscall_ns"].is_null());
    let counters = diagnostic["counters"].as_array().unwrap();
    for phase in [
        "ReadRingRequest",
        "ReadRingTerminalDecode",
        "LogicViewPublish",
        "ModelBuild",
        "PanePrepare",
        "UiProjection",
        "UiSeatRows",
        "UiOldModelDrop",
        "WindowRedraw",
    ] {
        assert!(
            counters
                .iter()
                .any(|row| row["period"] == 2 && row["phase"] == phase),
            "missing {phase}"
        );
    }
    assert!(counters.iter().any(|row| row["phase"] == "ModelBuild"
        && row["calls"].as_u64().unwrap() > 0
        && row["work"].as_u64().unwrap() >= 1000));
    assert!(
        counters
            .iter()
            .filter(|row| row["phase"] == "WindowRedraw")
            .all(|row| row["calls"] == 0)
    );
    let stamps = &diagnostic["probe"]["stamps"];
    for stage in [
        "Admitted",
        "ControlReceived",
        "ModelBuilt",
        "ModelPrepared",
        "ModelPublished",
        "LatestTaken",
        "ProjectionStarted",
        "ProjectionFinished",
        "ModelApplied",
        "Encoded",
        "GpuComplete",
    ] {
        assert!(
            stamps.as_object().unwrap().contains_key(stage),
            "missing {stage}"
        );
        assert!(stamps[stage].is_null() || stamps[stage].as_u64().is_some());
    }
    // Synthetic state is not a ring read or GUI socket observation.
    assert!(stamps["LogicOutputRead"].is_null());
    assert!(stamps["GuiReceived"].is_null());
}
