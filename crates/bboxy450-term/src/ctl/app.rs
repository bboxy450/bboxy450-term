//! Lifecycle commands launch locally and otherwise use the existing admitted door.
use super::*;
use std::{
    fs::OpenOptions,
    os::unix::fs::{DirBuilderExt, OpenOptionsExt},
    process::{Command as Process, Stdio},
    sync::mpsc,
    time::Instant,
};

const STARTUP: Duration = Duration::from_secs(45);

pub(super) fn start(
    endpoint: &Path,
    headless: bool,
    ctl: bool,
    unattended: bool,
    test_account: Option<&Path>,
) -> Result<Value, String> {
    let root = endpoint
        .parent()
        .ok_or("app start: run directory is unavailable")?;
    if endpoint.file_name() != Some(std::ffi::OsStr::new(bbt_mux::ctl_door::ENDPOINT)) {
        return Err(format!(
            "{USAGE_PREFIX}app start uses --run-dir, not a custom --endpoint filename"
        ));
    }
    if endpoint.exists() {
        let mut client = connect(&[], endpoint)?;
        let status = client.app(bbt_proto::gui::AppRequest::Status)?;
        let mut data = serde_json::to_value(status).map_err(|e| e.to_string())?;
        data["run_dir"] = json!(root);
        return Ok(data);
    }
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(root)
        .map_err(|e| format!("app start run directory: {e}"))?;
    let log = root.join("app.log");
    let stderr = OpenOptions::new()
        .create(true)
        .append(true)
        .mode(0o600)
        .open(&log)
        .map_err(|e| format!("app start log: {e}"))?;
    let mut process = Process::new(std::env::current_exe().map_err(|e| e.to_string())?);
    process
        .arg("--app-ready")
        .arg("--run-dir")
        .arg(root)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(stderr);
    if headless {
        process.arg("--headless");
    }
    if ctl {
        process.arg("--ctl");
    }
    if unattended {
        process.arg("--unattended");
    }
    if let Some(account) = test_account {
        process.arg("--test-account").arg(account);
    }
    let mut child = process.spawn().map_err(|e| format!("app start: {e}"))?;
    let pid = child.id();
    let stdout = child
        .stdout
        .take()
        .ok_or("app start readiness pipe unavailable")?;
    let (ready, received) = mpsc::sync_channel(1);
    std::thread::spawn(move || {
        let mut line = String::new();
        let result = BufReader::new(stdout.take(64 * 1024))
            .read_line(&mut line)
            .map(|_| line);
        let _ = ready.send(result);
    });
    let line=received.recv_timeout(STARTUP)
        .map_err(|_|format!("app endpoint unavailable: render readiness timed out; inspect {} and debug ps (app {pid})",log.display()))?
        .map_err(|e|format!("app endpoint unavailable: readiness pipe: {e}"))?;
    let status:bbt_proto::gui::AppStatus=serde_json::from_str(&line).map_err(|_| {
        let code=child.try_wait().ok().flatten().and_then(|s|s.code());
        format!("app endpoint unavailable: app exited before render readiness (exit {code:?}); inspect {} and debug ps",log.display())
    })?;
    if !status.backend_ready || !status.gui_ready || status.generation == 0 {
        return Err("app endpoint unavailable: app did not report backend and GUI render readiness; inspect debug ps".into());
    }
    let mut data = serde_json::to_value(status).map_err(|e| e.to_string())?;
    data["run_dir"] = json!(root);
    data["pid"] = json!(pid);
    Ok(data)
}

pub(super) fn stop(
    client: &mut Client,
    endpoint: &Path,
) -> Result<bbt_proto::gui::AppStatus, String> {
    let mut status = client
        .app(bbt_proto::gui::AppRequest::Stop)
        .map_err(|e| format!("app endpoint unavailable: {e}"))?;
    let start = Instant::now();
    // Endpoint removal belongs to the existing orderly logic shutdown. Never
    // signal a PID from metadata when the admitted app cannot answer.
    while endpoint.exists() {
        if start.elapsed() > Duration::from_secs(30) {
            return Err(
                "app endpoint unavailable: orderly shutdown did not finish; inspect debug ps"
                    .into(),
            );
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    status.running = false;
    status.backend_ready = false;
    status.gui_ready = false;
    status.events_ready = false;
    Ok(status)
}
