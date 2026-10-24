//! `bboxy450-term e2e night [--group G]`: the unattended e2e runner (#282, e2e-acceptance
//! §4a). It starts its own app with no window in a throwaway run directory, so no operator
//! account, Keychain or display is touched, and runs one deterministic ctl script per
//! promise (`scripts/e2e/<group>/<ID>.sh`); a script's exit code is the verdict, never an
//! agent's self-report. Exit 0 only when every promise passed, 1 with the red rows named.
#![allow(
    clippy::disallowed_methods,
    reason = "the runner is its own CLI process; it has no UI thread"
)]
use crate::cockpit::{Config, Event, Session};
use bbt_proto::{GuiAction, GuiReply};
use serde_json::json;
use std::{
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

/// One promise's script may run this long before it is stopped and counted as failed.
const SCRIPT_TIMEOUT: Duration = Duration::from_secs(180);

pub const USAGE: &str = "usage: bboxy450-term e2e night [--group GROUP] [--scripts DIR]
Starts a windowless app in a throwaway run directory with ctl on and, for every promise of
the group in e2e-acceptance §1 (default terminal), runs scripts/e2e/<ID>.sh against it (BBT,
BBOXY_RUN_DIR, E2E_WORK and E2E_PIDS set). Prints one JSON line per promise (pass, fail,
or untested when it has no script) and a summary; exits 0 only when every promise passed.";

pub fn main(args: &[String]) -> Result<bool, String> {
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("{USAGE}");
        return Ok(true);
    }
    if args.first().map(String::as_str) != Some("night") {
        return Err(USAGE.into());
    }
    let flag = |name: &str| args.windows(2).find(|w| w[0] == name).map(|w| w[1].clone());
    // A release build would sign with the login Keychain; the throwaway automation account
    // in file custody (#281) is what lets it run unattended, and it is not built yet.
    if !cfg!(debug_assertions) {
        return Err(
            "e2e night runs the debug build until the automation identity (#281) lands".into(),
        );
    }
    let group = flag("--group").unwrap_or_else(|| "terminal".into());
    let dir = flag("--scripts")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("scripts/e2e"));
    // The canonical list (§1, parsed) names the promises; a script per promise is the test.
    let ids: Vec<String> = crate::knowledge::promises()?
        .into_iter()
        .filter(|p| p.group == group)
        .map(|p| p.id)
        .collect();
    if ids.is_empty() {
        return Err(format!("e2e night: no promises in group {group}"));
    }
    let scripts: Vec<(String, PathBuf)> = ids
        .into_iter()
        .map(|id| {
            let path = dir.join(format!("{id}.sh"));
            (id, path)
        })
        .collect();
    bbt_rt::handle().block_on(night(&group, &scripts))
}

async fn night(group: &str, scripts: &[(String, PathBuf)]) -> Result<bool, String> {
    let root = tempfile::Builder::new()
        .prefix("e2en")
        .tempdir_in(std::env::var_os("TMPDIR").map_or_else(std::env::temp_dir, PathBuf::from))
        .map_err(|e| e.to_string())?;
    let root = root.path().canonicalize().map_err(|e| e.to_string())?;
    // The throwaway account: memory custody in this debug build, nothing persisted.
    // SAFETY: set before the runner starts any thread that reads the environment.
    unsafe { std::env::set_var("BBOXY_TEST_CUSTODY", "memory") };
    let executable = std::env::current_exe().map_err(|e| e.to_string())?;
    let mut red = Vec::new();
    let mut total = 0;
    for (id, script) in scripts {
        total += 1;
        // Each promise gets its own app and run directory: one script's damage (a killed
        // logic, a lost test identity) never decides another's verdict.
        let (verdict, exit, tail) = if script.is_file() {
            match promise(&root.join(id), script, &executable).await {
                Ok((exit, tail)) => (if exit == Some(0) { "pass" } else { "fail" }, exit, tail),
                Err(reason) => ("fail", None, reason),
            }
        } else {
            (
                "untested",
                None,
                format!("no script at {}", script.display()),
            )
        };
        if verdict != "pass" {
            red.push(id.clone());
        }
        println!(
            "{}",
            json!({ "kind": "e2e", "group": group, "function": id, "verdict": verdict,
                "exit": exit, "evidence": tail })
        );
    }
    let green = red.is_empty();
    println!(
        "{}",
        json!({ "kind": "summary", "group": group, "state": if green { "green" } else { "red" },
            "passed": total - red.len(), "total": total, "red": red })
    );
    Ok(green)
}

/// One promise: a fresh windowless app with ctl on, the script against it, then an orderly
/// shutdown. Returns the script's exit code and the tail of its output.
async fn promise(
    root: &Path,
    script: &Path,
    executable: &Path,
) -> Result<(Option<i32>, String), String> {
    let run = root.join("run");
    for dir in ["repository", "zsh-config", "work"] {
        std::fs::create_dir_all(root.join(dir)).map_err(|e| e.to_string())?;
    }
    let session = Session::start(Config {
        executable: executable.to_path_buf(),
        socket: run.join("mux.sock"),
        run_dir: run.clone(),
        repository: root.join("repository"),
        shell_config: Some(root.join("zsh-config")),
        ctl: true,
    })
    .await
    .map_err(|e| format!("the app did not start: {e}"))?;
    let pids = root.join("pids.json");
    let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
    let pump = tokio::spawn(pump(session, pids.clone(), stopped));
    let deadline = Instant::now() + Duration::from_secs(30);
    while !run.join(bbt_mux::ctl_door::ENDPOINT).exists() {
        if Instant::now() > deadline {
            let _ = stop.send(());
            let _ = pump.await;
            return Err("the app never opened ctl".into());
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let result = run_script(script, executable, &run, &root.join("work"), &pids).await;
    let _ = stop.send(());
    let shutdown = pump.await.map_err(|e| e.to_string())?;
    let (exit, mut tail) = result;
    if let Err(error) = shutdown {
        tail.push_str(&format!("\napp shutdown: {error}"));
    }
    Ok((exit, tail))
}

/// Keeps the windowless app alive: drains its events (so logic and supervisor recovery
/// run), answers a capture with "no window" at once, and keeps the current process ids in
/// `pids` for the scripts that kill them (T5, T6).
async fn pump(
    mut session: Session,
    pids: PathBuf,
    mut stopped: tokio::sync::oneshot::Receiver<()>,
) -> Result<(), String> {
    loop {
        let ids = json!({ "logic": session.logic.id(), "supervisor": session.supervisor.id() });
        let _ = std::fs::write(&pids, ids.to_string());
        tokio::select! {
            _ = &mut stopped => break,
            event = tokio::time::timeout(Duration::from_millis(500), session.event()) => {
                if let Ok(Ok(Event::Reply(reply))) = event
                    && let GuiReply::CaptureAsk { op, .. } = *reply
                {
                    let _ = session
                        .send(GuiAction::Captured {
                            id: op,
                            error: Some("the e2e runner is headless: no window to capture".into()),
                            frame: Default::default(),
                        })
                        .await;
                }
            }
        }
    }
    session.shutdown().await.map_err(|e| e.to_string())
}

/// Runs one promise's script with stdin closed; returns its exit code (None when it was
/// stopped at the timeout) and the last lines of its output as evidence.
async fn run_script(
    script: &Path,
    executable: &Path,
    run: &Path,
    work: &Path,
    pids: &Path,
) -> (Option<i32>, String) {
    let log = work.join("output.log");
    let Ok(file) = std::fs::File::create(&log) else {
        return (None, "cannot create the script log".into());
    };
    let Ok(err) = file.try_clone() else {
        return (None, "cannot create the script log".into());
    };
    let child = Command::new("sh")
        .arg(script)
        .env("BBT", executable)
        .env("BBOXY_RUN_DIR", run)
        .env("E2E_WORK", work)
        .env("E2E_PIDS", pids)
        .env("BBOXY_CTL_ISSUER", "automation, e2e night")
        .stdin(Stdio::null())
        .stdout(file)
        .stderr(err)
        .spawn();
    let Ok(mut child) = child else {
        return (None, "the script did not start".into());
    };
    let started = Instant::now();
    let exit = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status.code(),
            Ok(None) if started.elapsed() > SCRIPT_TIMEOUT => {
                // Our own child, by its handle: never a kill by name.
                let _ = child.kill();
                let _ = child.wait();
                break None;
            }
            Ok(None) => tokio::time::sleep(Duration::from_millis(100)).await,
            Err(_) => break None,
        }
    };
    let text = std::fs::read_to_string(&log).unwrap_or_default();
    let tail: Vec<&str> = text.lines().rev().take(6).collect();
    let mut tail: Vec<&str> = tail.into_iter().rev().collect();
    if exit.is_none() {
        tail.push("stopped at the script timeout");
    }
    let _ = std::io::stdout().flush();
    (exit, tail.join("\n"))
}

/// The summary's exit code: 0 green, 1 otherwise.
pub fn exit_code(green: bool) -> i32 {
    if green { 0 } else { 1 }
}
