//! `bboxy450-term e2e night [--group G]`: the unattended e2e runner (#282, e2e-acceptance
//! §4a). It starts the headless app (`ctl app start --headless`, on a throwaway test account)
//! in a throwaway run directory, so no operator account, Keychain or display is touched,
//! and runs one deterministic ctl script per
//! promise (`scripts/e2e/<group>/<ID>.sh`); a script's exit code is the verdict, never an
//! agent's self-report. Exit 0 only when every promise passed, 1 with the red rows named.
#![allow(
    clippy::disallowed_methods,
    reason = "the runner is its own CLI process; it has no UI thread"
)]
use serde_json::{Value, json};
use std::{
    io::Write,
    os::unix::fs::PermissionsExt,
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
BBOXY_RUN_DIR, E2E_WORK and E2E_APP, the app's pid, set). Then one more app on REPO (default a throwaway
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
    let temporary = tempfile::Builder::new()
        .prefix("e2en")
        .permissions(std::fs::Permissions::from_mode(0o700))
        .tempdir_in(std::env::var_os("TMPDIR").map_or_else(std::env::temp_dir, PathBuf::from))
        .map_err(|e| e.to_string())?;
    let root = temporary.path().canonicalize().map_err(|e| e.to_string())?;
    // First line: the night's private root, so a caller that stops the night at a bound can
    // stop every app under it by its own endpoint (invariant #283).
    println!(
        "{}",
        json!({ "kind": "night", "group": group, "root": root })
    );
    let executable = std::env::current_exe().map_err(|e| e.to_string())?;
    let mut red = Vec::new();
    let mut total = 0;
    let mut verdicts = Vec::new();
    let mut partial = Vec::new();
    let mut failed = false;
    for (id, script) in scripts {
        total += 1;
        // Each promise gets its own app and run directory: one script's damage (a killed
        // logic, a lost test identity) never decides another's verdict.
        let cover = if script.is_file() {
            coverage(script)
        } else {
            (false, vec![])
        };
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
        } else if !cover.0 {
            partial.push(id.clone());
        }
        failed |= verdict == "fail";
        println!(
            "{}",
            json!({ "kind": "e2e", "group": group, "function": id, "verdict": verdict,
                "exit": exit,
                "coverage": script.is_file().then_some(if cover.0 { "full" } else { "partial" }),
                "missing": cover.1, "evidence": tail })
        );
        verdicts.push((id.clone(), verdict, tail, cover));
    }
    // The records app: its own throwaway identity, after every promise's damage is done, so
    // a script that kills logic never costs another promise its record.
    let repository = records.unwrap_or_else(|| root.join("records"));
    let (unwritten, tracked) =
        write_records(&root.join("records-app"), &repository, group, verdicts).await;
    // Green only when every promise passed in full (#316); amber when every other row is a
    // failure or a partial pass an open task tracks; an untested row, an untracked failure or
    // partial, or an unwritten record is red.
    let state = if !unwritten.is_empty() {
        "red"
    } else if red.is_empty() && partial.is_empty() {
        "green"
    } else if red.iter().chain(&partial).all(|id| tracked.contains(id)) {
        "amber"
    } else {
        "red"
    };
    // Preserve failed evidence at its original private paths. App shutdown and
    // the unattended wrapper still stop owners; retention keeps files only.
    // Missing scripts/partial coverage alone do not retain these private accounts.
    let kept = (failed || !unwritten.is_empty()).then(|| temporary.keep());
    println!(
        "{}",
        json!({ "kind": "summary", "group": group, "state": state,
            "passed": total - red.len(), "total": total, "red": red, "partial": partial,
            "tracked": tracked, "unwritten": unwritten, "kept": kept })
    );
    Ok(state != "red")
}

/// Writes every promise's e2e record through one windowless app on `repository`; returns
/// the functions whose record could not be written and the failures an open task tracks.
async fn write_records(
    root: &Path,
    repository: &Path,
    group: &str,
    verdicts: Vec<Verdict>,
) -> (Vec<String>, Vec<String>) {
    let functions: Vec<String> = verdicts.iter().map(|(id, ..)| id.clone()).collect();
    let app = match App::start(root, repository, None).await {
        Ok(app) => app,
        Err(reason) => {
            println!("{}", json!({ "kind": "record", "error": reason }));
            return (functions, vec![]);
        }
    };
    println!("{}", app.line("records", None));
    let run = app.run.clone();
    let issued_by = app.issued_by.clone();
    let group = group.to_owned();
    let written = tokio::task::spawn_blocking(move || {
        let endpoint = crate::ctl::Endpoint::read(&run.join(bbt_mux::ctl_door::ENDPOINT))?;
        let mut client = crate::ctl::Client::connect(&endpoint)?;
        let now = crate::ctl::utc_now();
        let mut unwritten = Vec::new();
        let mut tracked = Vec::new();
        for (id, verdict, tail, cover) in verdicts {
            let run = Run {
                verdict,
                tail: &tail,
                now: &now,
                issued_by: &issued_by,
                cover: &cover,
            };
            let row = match upsert(&mut client, &group, &id, &run) {
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
/// A promise's id, verdict, evidence tail and coverage (full, else the missing assertions).
type Verdict = (String, &'static str, String, (bool, Vec<String>));

/// One promise's night, as its record is written.
struct Run<'a> {
    verdict: &'a str,
    tail: &'a str,
    now: &'a str,
    issued_by: &'a str,
    /// The script's coverage header: full, else the assertions it leaves out (#316).
    cover: &'a (bool, Vec<String>),
}

/// A script's `# coverage:` header (#316): `full`, or `partial; missing: a; b`. No header,
/// or a partial one that names no gap, is partial with the gap `unstated`: fail closed.
fn coverage(script: &Path) -> (bool, Vec<String>) {
    let text = std::fs::read_to_string(script).unwrap_or_default();
    let header = text
        .lines()
        .find_map(|line| line.strip_prefix("# coverage:"))
        .map(str::trim);
    if header.is_some_and(|h| h.split_whitespace().next() == Some("full")) {
        return (true, vec![]);
    }
    let missing: Vec<String> = header
        .and_then(|h| h.split_once("missing:"))
        .map(|(_, list)| {
            list.split(';')
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default();
    if missing.is_empty() {
        (false, vec!["unstated".into()])
    } else {
        (false, missing)
    }
}

fn upsert(
    client: &mut crate::ctl::Client,
    group: &str,
    id: &str,
    run: &Run<'_>,
) -> Result<(Value, Option<String>), String> {
    let Run {
        verdict,
        tail,
        now,
        issued_by,
        cover: (full, missing),
    } = *run;
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
        ("run_by", json!(issued_by)),
        ("result", json!(result)),
        ("coverage", json!(if *full { "full" } else { "partial" })),
        ("missing", json!(missing)),
    ] {
        frontmatter[key] = value;
    }
    let mut tracked = None;
    // A failure, and a pass that proves only part of its promise, each need an open task.
    let partial = verdict == "pass" && !*full;
    if verdict == "fail" || partial {
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
            let (title, brief) = if partial {
                (
                    format!("Cover e2e {id}: missing {}", missing.join("; ")),
                    format!(
                        "ROLE: EXECUTOR. scripts/e2e/{id}.sh passes but proves only part of \
                         promise {id} ([[e2e-acceptance]] §1, decision #316); its record is \
                         [[{path}]]. Assert the rest, then set its header to `# coverage: full`. \
                         Missing:\n\n- {}\n",
                        missing.join("\n- ")
                    ),
                )
            } else {
                (
                    format!("Fix e2e {id}: the night failed it"),
                    format!(
                        "ROLE: EXECUTOR. The e2e night failed promise {id} \
                         ([[e2e-acceptance]] §1); its record is [[{path}]].\n\nEvidence:\n\n```\n{tail}\n```"
                    ),
                )
            };
            let task = client.record_write(crate::ctl::task_create(
                &title,
                &format!("sh scripts/e2e/{id}.sh"),
                Some(&brief),
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
/// shutdown. Adjacent `<ID>.fixture-bin` and `<ID>.fixture-home` directories supply
/// deterministic harnesses and a private harness configuration to this app only.
/// Returns the script's exit code and the tail of its output.
async fn promise(
    root: &Path,
    script: &Path,
    executable: &Path,
) -> Result<(Option<i32>, String), String> {
    let fixture = script.with_extension("fixture-bin");
    let fixture = fixture.is_dir().then_some(fixture);
    let home_seed = script.with_extension("fixture-home");
    if home_seed.is_dir() {
        copy_fixture_home(&home_seed, &root.join("home"))?;
    }
    let app = App::start(root, &root.join("repository"), fixture.as_deref()).await?;
    println!(
        "{}",
        app.line("promise", root.file_name().and_then(|name| name.to_str()))
    );
    let result = run_script(
        script,
        executable,
        &app.run,
        &root.join("work"),
        app.pid,
        &app.issued_by,
    )
    .await;
    let (exit, mut tail) = result;
    if let Err(error) = app.stop().await {
        tail.push_str(&format!("\napp shutdown: {error}"));
    }
    Ok((exit, tail))
}

/// Copy a committed configuration seed without following links outside the fixture.
/// Each promise can mutate its copy; it never shares the seed or the runner's HOME.
fn copy_fixture_home(source: &Path, target: &Path) -> Result<(), String> {
    let kind = std::fs::symlink_metadata(source)
        .map_err(|e| e.to_string())?
        .file_type();
    if !kind.is_dir() {
        return Err("fixture home must be a directory, never a symlink".into());
    }
    std::fs::create_dir_all(target).map_err(|e| e.to_string())?;
    std::fs::set_permissions(target, std::fs::Permissions::from_mode(0o700))
        .map_err(|e| e.to_string())?;
    for entry in std::fs::read_dir(source).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let destination = target.join(entry.file_name());
        let kind = entry.file_type().map_err(|e| e.to_string())?;
        if kind.is_dir() {
            copy_fixture_home(&entry.path(), &destination)?;
        } else if kind.is_file() {
            std::fs::copy(entry.path(), &destination).map_err(|e| e.to_string())?;
            std::fs::set_permissions(&destination, std::fs::Permissions::from_mode(0o600))
                .map_err(|e| e.to_string())?;
        } else {
            return Err("fixture home contains a symlink or a special file".into());
        }
    }
    Ok(())
}

/// A windowless app in its own run directory, started as the operator would start one:
/// `ctl app start --headless --ctl --unattended --test-account` (U1 on U0), so captures,
/// status and recovery are the real owner's, and stopped through `ctl app stop`, never by
/// pid. `pid` is the app's own process, the parent of its logic and supervisor.
struct App {
    run: PathBuf,
    pid: u32,
    issued_by: String,
    /// The owner's readiness at start (`ctl app start`'s reply): mode, custody, windows.
    status: Value,
}

impl App {
    /// One line per app the night starts (invariant #283): the custody it ran on and its
    /// windows, so the unattended check reads them rather than trusting the runner.
    fn line(&self, role: &str, function: Option<&str>) -> Value {
        json!({ "kind": "app", "role": role, "function": function, "pid": self.pid,
            "run_dir": self.run,
            "mode": self.status["mode"], "custody_backend": self.status["custody_backend"],
            "windows": self.status["windows"], "issued_by": self.issued_by })
    }
    async fn start(
        root: &Path,
        repository: &Path,
        fixture_bin: Option<&Path>,
    ) -> Result<Self, String> {
        let run = root.join("run");
        for dir in [&root.join("zsh-config"), &root.join("work"), repository] {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let account = root.join("account");
        let mut launch = ctl(&run, repository, &root.join("zsh-config"));
        let home = root.join("home");
        if home.is_dir() {
            launch
                .env("HOME", &home)
                .env("CLAUDE_CONFIG_DIR", home.join(".claude"))
                .env("CODEX_HOME", home.join(".codex"));
        }
        // A deterministic harness is local to this promise's app and its real seats.
        // Do not mutate the runner's PATH or the separate records app's environment.
        if let Some(bin) = fixture_bin {
            let bin = bin.canonicalize().map_err(|e| e.to_string())?;
            let inherited = std::env::var_os("PATH").unwrap_or_default();
            let path =
                std::env::join_paths(std::iter::once(bin).chain(std::env::split_paths(&inherited)))
                    .map_err(|e| e.to_string())?;
            launch
                .env("PATH", path)
                .env("BBT", std::env::current_exe().map_err(|e| e.to_string())?);
        }
        let started = launch
            .args([
                "app",
                "start",
                "--headless",
                "--ctl",
                "--unattended",
                "--notifications",
                "record",
                "--test-account",
            ])
            .arg(&account)
            .output()
            .await
            .map_err(|e| format!("ctl app start: {e}"))?;
        let reply: Value = String::from_utf8_lossy(&started.stdout)
            .lines()
            .find_map(|line| serde_json::from_str(line).ok())
            .unwrap_or_default();
        let Some(pid) = reply["data"]["pid"].as_u64().map(|pid| pid as u32) else {
            return Err(format!(
                "the app did not start: {}{}",
                String::from_utf8_lossy(&started.stdout).trim(),
                String::from_utf8_lossy(&started.stderr).trim()
            ));
        };
        // The verified label logic announces on admission names the run's test account.
        let endpoint = run.join(bbt_mux::ctl_door::ENDPOINT);
        let label = tokio::task::spawn_blocking(move || {
            let mut client = crate::ctl::Client::connect(&crate::ctl::Endpoint::read(&endpoint)?)?;
            client.seats()?;
            Ok::<_, String>(client.identity.and_then(|identity| identity.reason))
        })
        .await
        .map_err(|e| e.to_string())?;
        let app = Self {
            run,
            pid,
            issued_by: String::new(),
            status: reply["data"].clone(),
        };
        match label {
            Ok(Some(label)) if label.starts_with("test-account:") => Ok(Self {
                issued_by: label,
                ..app
            }),
            other => {
                let _ = app.stop().await;
                Err(format!(
                    "the night app has no verified test-account label: {other:?}"
                ))
            }
        }
    }

    async fn stop(self) -> Result<(), String> {
        let stopped = ctl(&self.run, &self.run, &self.run)
            .args(["app", "stop"])
            .output()
            .await
            .map_err(|e| format!("ctl app stop: {e}"))?;
        if stopped.status.success() {
            Ok(())
        } else {
            Err(format!(
                "ctl app stop: {}{}",
                String::from_utf8_lossy(&stopped.stdout).trim(),
                String::from_utf8_lossy(&stopped.stderr).trim()
            ))
        }
    }
}

/// `bboxy450-term ctl --run-dir RUN …` from `cwd` (the app's repository on start), stdin
/// closed, with the throwaway zsh configuration the app's seats inherit.
fn ctl(run: &Path, cwd: &Path, zdotdir: &Path) -> tokio::process::Command {
    let mut command = tokio::process::Command::new(
        std::env::current_exe().unwrap_or_else(|_| "bboxy450-term".into()),
    );
    command
        .arg("ctl")
        .arg("--run-dir")
        .arg(run)
        .current_dir(cwd)
        .env("ZDOTDIR", zdotdir)
        .stdin(Stdio::null());
    command
}

/// Runs one promise's script with stdin closed; returns its exit code (None when it was
/// stopped at the timeout) and the last lines of its output as evidence.
async fn run_script(
    script: &Path,
    executable: &Path,
    run: &Path,
    work: &Path,
    app: u32,
    issued_by: &str,
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
        .env("E2E_APP", app.to_string())
        .env("BBOXY_CTL_ISSUER", issued_by)
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

#[cfg(test)]
mod fixture_home_tests {
    use super::*;

    // keeps: spec e2e-script-map §L3
    #[test]
    fn fixture_home_copies_private_configs_without_sharing_the_seed() {
        let temp = tempfile::tempdir().unwrap();
        let seed = temp.path().join("seed");
        let copied = temp.path().join("promise-home");
        std::fs::create_dir_all(seed.join(".claude")).unwrap();
        std::fs::write(seed.join(".claude/settings.json"), b"{\"fixture\":1}\n").unwrap();
        copy_fixture_home(&seed, &copied).unwrap();
        std::fs::write(seed.join(".claude/settings.json"), b"{\"fixture\":2}\n").unwrap();
        assert_eq!(
            std::fs::read(copied.join(".claude/settings.json")).unwrap(),
            b"{\"fixture\":1}\n"
        );
        assert_eq!(
            std::fs::metadata(&copied).unwrap().permissions().mode() & 0o777,
            0o700
        );
        assert_eq!(
            std::fs::metadata(copied.join(".claude/settings.json"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }

    // keeps: spec e2e-script-map §L3
    #[test]
    fn fixture_home_refuses_a_link_outside_its_seed() {
        let temp = tempfile::tempdir().unwrap();
        let seed = temp.path().join("seed");
        let copied = temp.path().join("promise-home");
        let outside = temp.path().join("outside");
        std::fs::create_dir(&seed).unwrap();
        std::fs::write(&outside, b"private outside value").unwrap();
        std::os::unix::fs::symlink(&outside, seed.join("escape")).unwrap();
        assert!(copy_fixture_home(&seed, &copied).is_err());
        assert!(!copied.join("escape").exists());
        assert_eq!(std::fs::read(&outside).unwrap(), b"private outside value");
    }
}
