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
use serde_json::{Value, json};
use std::{
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

/// One promise's script may run this long before it is stopped and counted as failed.
const SCRIPT_TIMEOUT: Duration = Duration::from_secs(180);

pub const USAGE: &str =
    "usage: bboxy450-term e2e night [--group GROUP] [--scripts DIR] [--records REPO]
Starts a windowless app in a throwaway run directory with ctl on and, for every promise of
the group in e2e-acceptance §1 (default terminal), runs scripts/e2e/<ID>.sh against it (BBT,
BBOXY_RUN_DIR, E2E_WORK and E2E_PIDS set). Then one more app on REPO (default a throwaway
repository) writes each promise's e2e record, .bboxy/e2e/<ID>.md, through `record write`:
created when missing, its status, last_run, run_by and result updated. Prints one JSON line
per promise (pass, fail, or untested when it has no script), one per record write and a
summary: green when every promise passed, amber when every other row is a failure an open
task tracks (the runner opens one, `task create`, for a failure nothing tracks, linked
tracked-by), red otherwise. Exits 0 on green or amber, 1 on red.";

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
    let records = flag("--records").map(PathBuf::from);
    let scripts: Vec<(String, PathBuf)> = ids
        .into_iter()
        .map(|id| {
            let path = dir.join(format!("{id}.sh"));
            (id, path)
        })
        .collect();
    bbt_rt::handle().block_on(night(&group, &scripts, records))
}

async fn night(
    group: &str,
    scripts: &[(String, PathBuf)],
    records: Option<PathBuf>,
) -> Result<bool, String> {
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
    let mut verdicts = Vec::new();
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
        verdicts.push((id.clone(), verdict, tail));
    }
    // The records app: its own throwaway identity, after every promise's damage is done, so
    // a script that kills logic never costs another promise its record.
    let repository = records.unwrap_or_else(|| root.join("records"));
    let (unwritten, tracked) =
        write_records(&root.join("records-app"), &repository, group, verdicts).await;
    // Amber (e2e acceptance §4a): every row that did not pass is a failure an open task
    // tracks. An untested row or an unwritten record stays red.
    let state = if !unwritten.is_empty() {
        "red"
    } else if red.is_empty() {
        "green"
    } else if red.iter().all(|id| tracked.contains(id)) {
        "amber"
    } else {
        "red"
    };
    println!(
        "{}",
        json!({ "kind": "summary", "group": group, "state": state,
            "passed": total - red.len(), "total": total, "red": red, "tracked": tracked,
            "unwritten": unwritten })
    );
    Ok(state != "red")
}

/// Writes every promise's e2e record through one windowless app on `repository`; returns
/// the functions whose record could not be written and the failures an open task tracks.
async fn write_records(
    root: &Path,
    repository: &Path,
    group: &str,
    verdicts: Vec<(String, &'static str, String)>,
) -> (Vec<String>, Vec<String>) {
    let functions: Vec<String> = verdicts.iter().map(|(id, ..)| id.clone()).collect();
    let app = match App::start(root, repository).await {
        Ok(app) => app,
        Err(reason) => {
            println!("{}", json!({ "kind": "record", "error": reason }));
            return (functions, vec![]);
        }
    };
    let run = app.run.clone();
    let group = group.to_owned();
    let written = tokio::task::spawn_blocking(move || {
        let endpoint = crate::ctl::Endpoint::read(&run.join(bbt_mux::ctl_door::ENDPOINT))?;
        let mut client = crate::ctl::Client::connect(&endpoint)?;
        let now = crate::ctl::utc_now();
        let mut unwritten = Vec::new();
        let mut tracked = Vec::new();
        for (id, verdict, tail) in verdicts {
            let row = match upsert(&mut client, &group, &id, verdict, &tail, &now) {
                Ok((record, task)) => {
                    if task.is_some() {
                        tracked.push(id.clone());
                    }
                    json!({ "kind": "record", "function": id,
                        "number": record["record"]["number"],
                        "path": record["record"]["path"], "tracked_by": task })
                }
                Err(error) => {
                    unwritten.push(id.clone());
                    json!({ "kind": "record", "function": id, "error": error })
                }
            };
            println!("{row}");
        }
        Ok::<_, String>((unwritten, tracked))
    })
    .await;
    if let Err(error) = app.stop().await {
        println!(
            "{}",
            json!({ "kind": "record", "error": format!("app shutdown: {error}") })
        );
    }
    match written {
        Ok(Ok(written)) => written,
        Ok(Err(reason)) => {
            println!("{}", json!({ "kind": "record", "error": reason }));
            (functions, vec![])
        }
        Err(error) => {
            println!(
                "{}",
                json!({ "kind": "record", "error": error.to_string() })
            );
            (functions, vec![])
        }
    }
}

/// One promise's record: `.bboxy/e2e/<ID>.md`, created when missing, else its run fields
/// updated under the fetched etag. The result is the verdict and the evidence's last line.
/// A failure no open task tracks gets one (`task create`), linked `tracked-by`; returns the
/// write result and the open task that tracks a failure.
fn upsert(
    client: &mut crate::ctl::Client,
    group: &str,
    id: &str,
    verdict: &str,
    tail: &str,
    now: &str,
) -> Result<(Value, Option<String>), String> {
    let path = format!("e2e/{id}.md");
    let last = tail.lines().last().unwrap_or_default();
    let result: String = format!("{verdict}: {last}").chars().take(240).collect();
    let read = |client: &mut crate::ctl::Client, method, args| {
        client
            .knowledge(&crate::knowledge::ReadCall { method, args })
            .map_err(|e| format!("{}: {}", e.code, e.message))
    };
    let (mut args, mut frontmatter) = match read(client, "resolve", json!({ "locator": path })) {
        Ok(reference) => {
            let record = read(client, "fetch", json!({ "ref": reference }))?;
            let args = json!({ "ref": reference, "body": record["body"], "etag": record["etag"] });
            (args, record["frontmatter"].clone())
        }
        Err(error) if error.starts_with("not_found") => (
            json!({ "path": path,
                "body": format!("The e2e record for promise {id}; scripts/e2e/{id}.sh is its test.\n") }),
            json!({ "type": "e2e", "title": format!("{id} e2e promise"), "function": id,
                "group": group, "links": [] }),
        ),
        Err(error) => return Err(error),
    };
    for (key, value) in [
        ("status", json!(verdict)),
        ("last_run", json!(now)),
        ("run_by", json!("automation, e2e night")),
        ("result", json!(result)),
    ] {
        frontmatter[key] = value;
    }
    let mut tracked = None;
    if verdict == "fail" {
        let links = frontmatter["links"].as_array().cloned().unwrap_or_default();
        for link in links.iter().filter(|l| l["rel"] == "tracked-by") {
            let target = link["to"].as_str().unwrap_or_default();
            let target = target.trim_start_matches("[[").trim_end_matches("]]");
            let open = read(client, "resolve", json!({ "locator": target }))
                .and_then(|reference| read(client, "fetch", json!({ "ref": reference })))
                .is_ok_and(|task| {
                    bbt_ui::modules::e2e_checklist::task_open(
                        task["frontmatter"]["status"].as_str().unwrap_or_default(),
                    )
                });
            if open {
                tracked = Some(target.to_owned());
                break;
            }
        }
        if tracked.is_none() {
            let task = client.record_write(crate::ctl::task_create(
                &format!("Fix e2e {id}: the night failed it"),
                &format!("sh scripts/e2e/{id}.sh"),
                Some(&format!(
                    "ROLE: EXECUTOR. The e2e night failed promise {id} \
                     ([[e2e-acceptance]] §1); its record is [[{path}]].\n\nEvidence:\n\n```\n{tail}\n```"
                )),
            ))?;
            let task = task["record"]["id"].as_str().unwrap_or_default().to_owned();
            let mut links = links;
            links.push(json!({ "to": format!("[[{task}]]"), "rel": "tracked-by" }));
            frontmatter["links"] = json!(links);
            tracked = Some(task);
        }
    }
    args["frontmatter"] = frontmatter;
    client.record_write(args).map(|record| (record, tracked))
}

/// One promise: a fresh windowless app with ctl on, the script against it, then an orderly
/// shutdown. Returns the script's exit code and the tail of its output.
async fn promise(
    root: &Path,
    script: &Path,
    executable: &Path,
) -> Result<(Option<i32>, String), String> {
    let app = App::start(root, &root.join("repository")).await?;
    let result = run_script(script, executable, &app.run, &root.join("work"), &app.pids).await;
    let (exit, mut tail) = result;
    if let Err(error) = app.stop().await {
        tail.push_str(&format!("\napp shutdown: {error}"));
    }
    Ok((exit, tail))
}

/// A windowless app with ctl on, its run directory under `root`, kept alive by `pump`.
struct App {
    run: PathBuf,
    pids: PathBuf,
    stop: tokio::sync::oneshot::Sender<()>,
    pump: tokio::task::JoinHandle<Result<(), String>>,
}

impl App {
    async fn start(root: &Path, repository: &Path) -> Result<Self, String> {
        let run = root.join("run");
        for dir in [&root.join("zsh-config"), &root.join("work"), repository] {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let session = Session::start(Config {
            executable: std::env::current_exe().map_err(|e| e.to_string())?,
            socket: run.join("mux.sock"),
            run_dir: run.clone(),
            repository: repository.to_path_buf(),
            shell_config: Some(root.join("zsh-config")),
            ctl: true,
        })
        .await
        .map_err(|e| format!("the app did not start: {e}"))?;
        let pids = root.join("pids.json");
        let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
        let app = Self {
            pump: tokio::spawn(pump(session, pids.clone(), stopped)),
            run,
            pids,
            stop,
        };
        let deadline = Instant::now() + Duration::from_secs(30);
        while !app.run.join(bbt_mux::ctl_door::ENDPOINT).exists() {
            if Instant::now() > deadline {
                let _ = app.stop().await;
                return Err("the app never opened ctl".into());
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        Ok(app)
    }

    async fn stop(self) -> Result<(), String> {
        let _ = self.stop.send(());
        self.pump.await.map_err(|e| e.to_string())?
    }
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

/// The summary's exit code: 0 green or amber, 1 red.
pub fn exit_code(ok: bool) -> i32 {
    if ok { 0 } else { 1 }
}
