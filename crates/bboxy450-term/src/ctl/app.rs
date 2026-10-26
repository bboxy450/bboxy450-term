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
    time::{Instant, SystemTime},
};

const STARTUP: Duration = Duration::from_secs(45);

pub(super) fn start(
    endpoint: &Path,
    headless: bool,
    ctl: bool,
    unattended: bool,
    test_account: Option<&Path>,
    record_notifications: bool,
) -> Result<Value, String> {
    if record_notifications && (test_account.is_none() || !cfg!(debug_assertions)) {
        return Err(format!(
            "{USAGE_PREFIX}notification recording requires a debug build and --test-account DIR"
        ));
    }
    let root = endpoint
        .parent()
        .ok_or("app start: run directory is unavailable")?;
    if endpoint.file_name() != Some(std::ffi::OsStr::new(bbt_mux::ctl_door::ENDPOINT)) {
        return Err(format!(
            "{USAGE_PREFIX}app start uses --run-dir, not a custom --endpoint filename"
        ));
    }
    if endpoint.exists() && crate::cockpit::frontend_running(root).map_err(|e|e.to_string())?.unwrap_or(true) {
        let mut client = connect(&[], endpoint)?;
        if record_notifications
            && client
                .app(bbt_proto::gui::AppRequest::Notifications)?
                .notifications["backend"]
                != "recording"
        {
            return Err("app already runs without recording; stop it and start with --test-account DIR --notifications record".into());
        }
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
    if record_notifications {
        process.arg("--notifications").arg("record");
    }
    let launched = SystemTime::now();
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
    let status: bbt_proto::gui::AppStatus = serde_json::from_str(&line).map_err(|_| {
        let status = child.try_wait();
        let report = crash_report(pid, launched, &status);
        startup_failure(pid, status, &log, report.as_deref())
    })?;
    if !status.backend_ready || !status.gui_ready || status.generation == 0 {
        return Err("app endpoint unavailable: app did not report backend and GUI render readiness; inspect debug ps".into());
    }
    let mut data = serde_json::to_value(status).map_err(|e| e.to_string())?;
    data["run_dir"] = json!(root);
    data["pid"] = json!(pid);
    Ok(data)
}

fn startup_failure(
    pid: u32,
    status: std::io::Result<Option<ExitStatus>>,
    log: &Path,
    report: Option<&Path>,
) -> String {
    let termination = match status {
        Ok(Some(status)) => {
            if let Some(signal) = status.signal() {
                let name = signal_name(signal).unwrap_or("unknown signal");
                format!(
                    "{name} (signal {signal}), core dumped: {}",
                    status.core_dumped()
                )
            } else if let Some(code) = status.code() {
                format!("exit code {code}")
            } else {
                status.to_string()
            }
        }
        Ok(None) => "still running when readiness closed".into(),
        Err(error) => format!("exit status unavailable: {error}"),
    };
    let crash_reports = if let Some(report) = report {
        format!("; matching crash report: {}", report.display())
    } else if cfg!(target_os = "macos") {
        "; macOS crash reports, if generated: ~/Library/Logs/DiagnosticReports/ (match the app PID)"
            .into()
    } else {
        String::new()
    };
    format!(
        "app endpoint unavailable: app {pid} did not report valid render readiness ({termination}); inspect {} and debug ps{crash_reports}",
        log.display()
    )
}

fn signal_name(signal: i32) -> Option<&'static str> {
    Some(match signal {
        libc::SIGBUS => "SIGBUS",
        libc::SIGSEGV => "SIGSEGV",
        libc::SIGABRT => "SIGABRT",
        libc::SIGILL => "SIGILL",
        libc::SIGFPE => "SIGFPE",
        libc::SIGKILL => "SIGKILL",
        libc::SIGTERM => "SIGTERM",
        libc::SIGINT => "SIGINT",
        libc::SIGQUIT => "SIGQUIT",
        libc::SIGHUP => "SIGHUP",
        libc::SIGPIPE => "SIGPIPE",
        libc::SIGTRAP => "SIGTRAP",
        libc::SIGUSR1 => "SIGUSR1",
        libc::SIGUSR2 => "SIGUSR2",
        libc::SIGALRM => "SIGALRM",
        libc::SIGXCPU => "SIGXCPU",
        libc::SIGXFSZ => "SIGXFSZ",
        libc::SIGVTALRM => "SIGVTALRM",
        libc::SIGPROF => "SIGPROF",
        libc::SIGSYS => "SIGSYS",
        #[cfg(target_os = "macos")]
        libc::SIGEMT => "SIGEMT",
        _ => return None,
    })
}

#[cfg(target_os = "macos")]
fn crash_report(
    pid: u32,
    launched: SystemTime,
    status: &std::io::Result<Option<ExitStatus>>,
) -> Option<PathBuf> {
    if !matches!(status, Ok(Some(_))) {
        return None;
    }
    let directory = PathBuf::from(std::env::var_os("HOME")?).join("Library/Logs/DiagnosticReports");
    newest_crash_report(&directory, pid, launched)
}

#[cfg(not(target_os = "macos"))]
fn crash_report(_: u32, _: SystemTime, _: &std::io::Result<Option<ExitStatus>>) -> Option<PathBuf> {
    None
}

#[cfg(any(target_os = "macos", test))]
fn newest_crash_report(directory: &Path, pid: u32, launched: SystemTime) -> Option<PathBuf> {
    const REPORT_LIMIT: u64 = 1024 * 1024;
    std::fs::read_dir(directory)
        .ok()?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name();
            let name = name.to_str()?;
            if !name.starts_with("bboxy450-term-")
                || !name.ends_with(".ips")
                || !entry.file_type().ok()?.is_file()
            {
                return None;
            }
            let metadata = entry.metadata().ok()?;
            let modified = metadata.modified().ok()?;
            if modified < launched || metadata.len() > REPORT_LIMIT {
                return None;
            }
            let path = entry.path();
            let mut bytes = Vec::new();
            std::fs::File::open(&path)
                .ok()?
                .take(REPORT_LIMIT + 1)
                .read_to_end(&mut bytes)
                .ok()?;
            if bytes.len() as u64 > REPORT_LIMIT {
                return None;
            }
            // .ips contains a metadata JSON value followed by the report JSON.
            let matches = serde_json::Deserializer::from_slice(&bytes)
                .into_iter::<Value>()
                .filter_map(Result::ok)
                .any(|report| {
                    report["pid"].as_u64() == Some(u64::from(pid))
                        && report["procName"].as_str() == Some("bboxy450-term")
                });
            matches.then_some((modified, path))
        })
        .max_by_key(|(modified, _)| *modified)
        .map(|(_, path)| path)
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

/// This noun's rows of [`super::VERBS`], in parse order.
pub(super) const VERBS: &[super::Verb] = &[
    ("app", "endpoint", "-"),
    ("app", "start", "-"),
    ("app", "status", "App"),
    ("app", "resize", "Ui"),
    ("app", "stop", "App"),
];

/// This noun's `Command` variants, spliced into the flat enum by `commands` in `ctl.rs`.
macro_rules! app_commands {
    ($cb:ident, $rest:tt [$($acc:tt)*] $($head:tt)*) => {
        $cb!(@next $rest [$($acc)* "app": [
            AppEndpoint,
            /// Start the same app binary and wait for its GUI render-readiness line.
            AppStart {
                headless: bool,
                ctl: bool,
                unattended: bool,
                test_account: Option<PathBuf>,
                record_notifications: bool,
            },
            /// Read liveness through the attached GUI owner.
            AppStatus,
            /// `app resize <cols>x<rows>`: the window to that grid of cells; the layout sizes panes.
            AppResize {
                cols: u16,
                rows: u16,
            },
            /// Stop through the authenticated app relay, never by PID.
            AppStop,
        ]] $($head)*);
    };
}
pub(super) use app_commands;

/// `ctl app …` and its pre-grammar aliases, parsed into this noun's commands; `None`
/// when the words are not this noun's.
pub(super) fn parse(words: &[&str], cx: &super::ParseCx) -> Result<Option<Command>, String> {
    let flags = cx.flags;
    let flag = |name: &str| flags.get(name).cloned();
    let headless = cx.headless;
    let ctl = cx.ctl;
    let unattended = cx.unattended;
    Ok(Some(match *words {
        ["app", "endpoint"] => Command::AppEndpoint,
        ["app", "start"] => Command::AppStart {
            headless,
            ctl,
            unattended,
            test_account: flag("test-account").map(PathBuf::from),
            record_notifications: match flag("notifications").as_deref() {
                None => false,
                Some("record") if flags.contains_key("test-account") => true,
                _ => {
                    return Err(format!(
                        "{USAGE_PREFIX}--notifications record requires --test-account DIR"
                    ));
                }
            },
        },
        ["app", "status"] => Command::AppStatus,
        ["app", "resize", size] => {
            let (cols, rows) = size
                .split_once('x')
                .and_then(|(c, r)| Some((c.parse::<u16>().ok()?, r.parse::<u16>().ok()?)))
                .filter(|&(c, r)| (2..=512).contains(&c) && (1..=256).contains(&r))
                .ok_or(format!(
                    "{USAGE_PREFIX}app resize: COLSxROWS, 2-512 by 1-256, not {size}"
                ))?;
            Command::AppResize { cols, rows }
        }
        ["app", "stop"] => Command::AppStop,
        _ => return Ok(None),
    }))
}

/// This noun's noun-verb commands, run on the admitted connection; prints the one reply.
pub(super) fn run_verb(cx: super::VerbCx<'_>, command: Command) -> Result<(), String> {
    let super::VerbCx {
        mut client,
        endpoint_file,
        out,
        noun,
        verb,
        id,
        human,
        ..
    } = cx;
    let data = match command {
        Command::AppResize { cols, rows } => {
            serde_json::to_value(client.ui(bbt_proto::gui::UiRequest::Resize { cols, rows })?)
                .map_err(|e| e.to_string())?
        }
        Command::AppStatus => serde_json::to_value(client.app(bbt_proto::gui::AppRequest::Status)?)
            .map_err(|e| e.to_string())?,
        Command::AppStop => serde_json::to_value(app::stop(&mut client, &endpoint_file)?)
            .map_err(|e| e.to_string())?,
        command => super::flat_reply(&mut client, command, verb)?,
    };
    super::print_reply(out, noun, verb, id, data, human)
}

/// One parse sample per verb of this noun's table (the verb gate in ctl.rs's tests).
#[cfg(test)]
pub(super) const PARSE_SAMPLES: &[&str] = &[
    "app endpoint",
    "app start --headless --ctl",
    "app status",
    "app resize 100x30",
    "app stop",
];

#[cfg(test)]
mod tests {
    use super::*;

    fn report_fixture(directory: &Path, name: &str, pid: u32, modified: SystemTime) -> PathBuf {
        let path = directory.join(name);
        std::fs::write(&path, format!(
            "{{\"app_name\":\"bboxy450-term\"}}\n{{\"pid\":{pid},\"procName\":\"bboxy450-term\"}}\n"
        )).unwrap();
        std::fs::File::open(&path)
            .unwrap()
            .set_times(std::fs::FileTimes::new().set_modified(modified))
            .unwrap();
        path
    }

    #[test]
    fn startup_diagnostic_names_a_sigbus_child_and_its_matching_report() {
        let launched = SystemTime::now();
        let mut child = Process::new("/bin/sh")
            .args(["-c", "kill -BUS $$"])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let pid = child.id();
        let mut line = String::new();
        assert_eq!(
            BufReader::new(child.stdout.take().unwrap())
                .read_line(&mut line)
                .unwrap(),
            0
        );
        let status = child.wait().unwrap();
        assert_eq!(status.signal(), Some(libc::SIGBUS));
        let directory = tempfile::tempdir().unwrap();
        let report = report_fixture(
            directory.path(),
            "bboxy450-term-child.ips",
            pid,
            SystemTime::now(),
        );
        let found = newest_crash_report(directory.path(), pid, launched).unwrap();
        assert_eq!(found, report);
        let message = startup_failure(
            pid,
            child.try_wait(),
            Path::new("/private/run/app.log"),
            Some(&found),
        );
        assert!(message.contains("SIGBUS"), "{message}");
        assert!(
            message.contains(&format!("signal {}", libc::SIGBUS)),
            "{message}"
        );
        assert!(message.contains(report.to_str().unwrap()), "{message}");
        assert!(!message.contains("exit None"), "{message}");
    }

    #[test]
    fn startup_diagnostic_selects_newest_post_launch_report_for_its_pid_only() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path();
        let launched = std::time::UNIX_EPOCH + Duration::from_secs(1000);
        report_fixture(
            root,
            "bboxy450-term-old.ips",
            42,
            launched - Duration::from_secs(1),
        );
        assert_eq!(newest_crash_report(root, 42, launched), None);
        report_fixture(
            root,
            "bboxy450-term-first.ips",
            42,
            launched + Duration::from_secs(1),
        );
        let expected = report_fixture(
            root,
            "bboxy450-term-newest.ips",
            42,
            launched + Duration::from_secs(2),
        );
        report_fixture(
            root,
            "bboxy450-term-other-pid.ips",
            43,
            launched + Duration::from_secs(9),
        );
        report_fixture(
            root,
            "other-app-newer.ips",
            42,
            launched + Duration::from_secs(10),
        );
        std::fs::write(root.join("bboxy450-term-invalid.ips"), "not JSON").unwrap();
        let oversized = std::fs::File::create(root.join("bboxy450-term-oversize.ips")).unwrap();
        oversized.set_len(1024 * 1024 + 1).unwrap();
        std::os::unix::fs::symlink(&expected, root.join("bboxy450-term-link.ips")).unwrap();
        assert_eq!(newest_crash_report(root, 42, launched), Some(expected));
        assert_eq!(
            newest_crash_report(&root.join("missing"), 42, launched),
            None
        );
    }

    #[test]
    fn startup_diagnostic_preserves_a_real_terminating_signal() {
        let status = Process::new("/bin/sh")
            .args(["-c", "kill -TERM $$"])
            .status()
            .unwrap();
        assert_eq!(status.signal(), Some(libc::SIGTERM));
        let message = startup_failure(
            12345,
            Ok(Some(status)),
            Path::new("/private/run/app.log"),
            None,
        );
        assert!(message.contains("app 12345"), "{message}");
        assert!(message.contains("signal 15"), "{message}");
        assert!(message.contains("SIGTERM"), "{message}");
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
        let exited = startup_failure(1, Ok(Some(ExitStatus::from_raw(7 << 8))), log, None);
        assert!(exited.contains("exit code 7"), "{exited}");
        let crashed = startup_failure(2, Ok(Some(ExitStatus::from_raw(6 | 0x80))), log, None);
        assert!(crashed.contains("SIGABRT (signal 6)"), "{crashed}");
        assert!(crashed.contains("core dumped: true"), "{crashed}");
    }

    #[test]
    fn startup_diagnostic_does_not_claim_an_unobserved_exit() {
        let log = Path::new("/private/run/app.log");
        let running = startup_failure(3, Ok(None), log, None);
        assert!(
            running.contains("still running when readiness closed"),
            "{running}"
        );
        let unknown = startup_failure(
            4,
            Err(std::io::Error::other("status probe failed")),
            log,
            None,
        );
        assert!(
            unknown.contains("exit status unavailable: status probe failed"),
            "{unknown}"
        );
    }
}
