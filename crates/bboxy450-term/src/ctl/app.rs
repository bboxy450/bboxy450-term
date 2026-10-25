//! Lifecycle commands launch locally and otherwise use the existing admitted door.
use super::*;
use std::{
    fs::OpenOptions,
    os::unix::{
        fs::{DirBuilderExt, OpenOptionsExt},
        process::ExitStatusExt,
    },
    process::{Command as Process, ExitStatus, Stdio},
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
    let status: bbt_proto::gui::AppStatus =
        serde_json::from_str(&line).map_err(|_| startup_failure(pid, child.try_wait(), &log))?;
    if !status.backend_ready || !status.gui_ready || status.generation == 0 {
        return Err("app endpoint unavailable: app did not report backend and GUI render readiness; inspect debug ps".into());
    }
    let mut data = serde_json::to_value(status).map_err(|e| e.to_string())?;
    data["run_dir"] = json!(root);
    data["pid"] = json!(pid);
    Ok(data)
}

fn startup_failure(pid: u32, status: std::io::Result<Option<ExitStatus>>, log: &Path) -> String {
    let termination = match status {
        Ok(Some(status)) => {
            if let Some(signal) = status.signal() {
                format!("signal {signal}, core dumped: {}", status.core_dumped())
            } else if let Some(code) = status.code() {
                format!("exit code {code}")
            } else {
                status.to_string()
            }
        }
        Ok(None) => "still running when readiness closed".into(),
        Err(error) => format!("exit status unavailable: {error}"),
    };
    let crash_reports = if cfg!(target_os = "macos") {
        "; macOS crash reports, if generated: ~/Library/Logs/DiagnosticReports/ (match the app PID)"
    } else {
        ""
    };
    format!(
        "app endpoint unavailable: app {pid} did not report valid render readiness ({termination}); inspect {} and debug ps{crash_reports}",
        log.display()
    )
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn startup_diagnostic_preserves_a_real_terminating_signal() {
        let status = Process::new("/bin/sh")
            .args(["-c", "kill -TERM $$"])
            .status()
            .unwrap();
        assert_eq!(status.signal(), Some(libc::SIGTERM));
        let message = startup_failure(12345, Ok(Some(status)), Path::new("/private/run/app.log"));
        assert!(message.contains("app 12345"), "{message}");
        assert!(message.contains("signal 15"), "{message}");
        assert!(message.contains("/private/run/app.log"), "{message}");
        if cfg!(target_os = "macos") {
            assert!(
                message.contains("~/Library/Logs/DiagnosticReports/"),
                "{message}"
            );
        }
        assert!(!message.contains("exit None"), "{message}");
    }

    #[test]
    fn startup_diagnostic_preserves_exit_codes_and_core_dump_status() {
        let log = Path::new("/private/run/app.log");
        let exited = startup_failure(1, Ok(Some(ExitStatus::from_raw(7 << 8))), log);
        assert!(exited.contains("exit code 7"), "{exited}");
        let crashed = startup_failure(2, Ok(Some(ExitStatus::from_raw(6 | 0x80))), log);
        assert!(crashed.contains("signal 6, core dumped: true"), "{crashed}");
    }

    #[test]
    fn startup_diagnostic_does_not_claim_an_unobserved_exit() {
        let log = Path::new("/private/run/app.log");
        let running = startup_failure(3, Ok(None), log);
        assert!(
            running.contains("still running when readiness closed"),
            "{running}"
        );
        let unknown = startup_failure(4, Err(std::io::Error::other("status probe failed")), log);
        assert!(
            unknown.contains("exit status unavailable: status probe failed"),
            "{unknown}"
        );
    }
}
