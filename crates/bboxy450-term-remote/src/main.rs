// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! `bboxy450-term-remote serve --state DIR` is what the SSH provider runs: a
//! stdio↔socket pump to the swarm's daemon, starting it when absent.
//! `bboxy450-term-remote daemon --state DIR` owns the sessions on
//! DIR/link.sock (0600), so seats outlive any one SSH connection.
// A backend daemon binary: the UI-only bbt-rt IO rule does not apply.
#![allow(clippy::disallowed_methods)]
use bboxy450_term_remote::client::Transport;
use bboxy450_term_remote::serve::Server;
use std::io::{self, Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::process::{ExitCode, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};

fn daemon(state: &Path) -> io::Result<()> {
    let server = Arc::new(Server::open(state)?);
    let sock = state.join("link.sock");
    if UnixStream::connect(&sock).is_ok() {
        return Err(io::Error::new(
            io::ErrorKind::AddrInUse,
            "a daemon already serves this state",
        ));
    }
    let _ = std::fs::remove_file(&sock); // stale socket from a dead daemon
    let listener = UnixListener::bind(&sock)?;
    std::fs::set_permissions(&sock, std::fs::Permissions::from_mode(0o600))?;
    std::fs::write(
        state.join("daemon.pid"),
        format!("{}\n", std::process::id()),
    )?;
    for stream in listener.incoming() {
        let stream = stream?;
        let server = Arc::clone(&server);
        std::thread::spawn(move || {
            if let Ok(writer) = stream.try_clone() {
                let _ = server.serve(stream, writer);
            }
        });
    }
    Ok(())
}

fn pump(from: &mut impl Read, to: &mut impl Write) -> io::Result<()> {
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = from.read(&mut buf)?;
        if n == 0 {
            return Ok(());
        }
        to.write_all(&buf[..n])?;
        to.flush()?;
    }
}

fn serve(state: &Path) -> io::Result<()> {
    std::fs::create_dir_all(state)?;
    std::fs::set_permissions(state, std::fs::Permissions::from_mode(0o700))?;
    let sock = state.join("link.sock");
    let stream = match UnixStream::connect(&sock) {
        Ok(s) => s,
        Err(_) => {
            // Its own process group: harness children share it, a closing
            // SSH session's signals do not reach it, and `stop` ends them all.
            use std::os::unix::process::CommandExt;
            std::process::Command::new(std::env::current_exe()?)
                .process_group(0)
                .args(["daemon", "--state"])
                .arg(state)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()?;
            let deadline = Instant::now() + Duration::from_secs(10);
            loop {
                match UnixStream::connect(&sock) {
                    Ok(s) => break s,
                    Err(e) if Instant::now() > deadline => return Err(e),
                    Err(_) => std::thread::sleep(Duration::from_millis(20)),
                }
            }
        }
    };
    {
        let mut out = io::stdout().lock();
        out.write_all(bboxy450_term_remote::frame::MAGIC)?;
        out.flush()?;
    }
    let mut from = stream.try_clone()?;
    // Stdout is line-buffered; frames are binary, so flush every chunk.
    let up = std::thread::spawn(move || pump(&mut from, &mut io::stdout().lock()));
    let mut to = stream.try_clone()?;
    pump(&mut io::stdin().lock(), &mut to)?;
    let _ = stream.shutdown(std::net::Shutdown::Both);
    let _ = up.join();
    Ok(())
}

/// `--ssh HOST`, `--via CMD` or `--local PROGRAM`, plus `--state DIR`.
fn transport(args: &[String]) -> Result<Transport, String> {
    let value = |flag: &str| {
        args.iter()
            .position(|a| a == flag)
            .and_then(|i| args.get(i + 1))
            .cloned()
    };
    let state = value("--state").unwrap_or_else(|| "~/.bboxy450-remote".into());
    if let Some(host) = value("--ssh") {
        Ok(Transport::Ssh { host, state })
    } else if let Some(command) = value("--via") {
        Ok(Transport::Via { command, state })
    } else if let Some(program) = value("--local") {
        Ok(Transport::Local { program, state })
    } else {
        Err("one of --ssh HOST, --via CMD or --local PROGRAM is required".into())
    }
}

fn flag(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

fn usage() -> ExitCode {
    eprintln!(
        "usage:\n  bboxy450-term-remote serve|daemon --state DIR\n  \
         bboxy450-term-remote attach (--ssh HOST|--via CMD|--local PROG) [--state DIR] --swarm ID [--session NAME] --cwd REMOTE_DIR\n  \
         bboxy450-term-remote profile (--ssh HOST|--via CMD) [--state DIR] --name NAME --swarm ID --cwd REMOTE_DIR\n  \
         bboxy450-term-remote install (--ssh HOST|--via CMD) [--harness PATH]\n  bboxy450-term-remote stop --state DIR\n  bboxy450-term-remote version"
    );
    ExitCode::from(2)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("mcp") => {
            // Run by the remote harness as a relayed MCP server (see attach).
            let (Some(state), Some(session), Some(name)) = (
                flag(&args, "--state"),
                flag(&args, "--session"),
                flag(&args, "--name"),
            ) else {
                return usage();
            };
            return match bboxy450_term_remote::attach::mcp(Path::new(&state), &session, &name) {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => {
                    eprintln!("bboxy450-term-remote mcp: {e}");
                    ExitCode::FAILURE
                }
            };
        }
        Some("stop") => {
            // End the daemon and every harness it started (one process group).
            let Some(state) = flag(&args, "--state") else {
                return usage();
            };
            let pid =
                std::fs::read_to_string(Path::new(&state).join("daemon.pid")).unwrap_or_default();
            let pid = pid.trim();
            if pid.is_empty() {
                eprintln!("bboxy450-term-remote stop: no daemon.pid in {state}");
                return ExitCode::FAILURE;
            }
            let ok = std::process::Command::new("kill")
                .args(["-TERM", "--", &format!("-{pid}")])
                .status()
                .is_ok_and(|s| s.success());
            return if ok {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            };
        }
        Some("version") => {
            println!("bboxy450-term-remote {}", env!("CARGO_PKG_VERSION"));
            return ExitCode::SUCCESS;
        }
        Some("attach") => {
            let (Ok(t), Some(swarm), Some(session), Some(cwd)) = (
                transport(&args),
                flag(&args, "--swarm"),
                flag(&args, "--session").or_else(|| std::env::var("BBOXY_SEAT_ID").ok()),
                flag(&args, "--cwd"),
            ) else {
                return usage();
            };
            return match bboxy450_term_remote::attach::run(t, &swarm, &session, &cwd) {
                Ok(code) => ExitCode::from(code as u8),
                Err(e) => {
                    eprintln!("bboxy450-term-remote attach: {e}");
                    ExitCode::FAILURE
                }
            };
        }
        Some("profile") => {
            // Print a bboxy450-harness acp-agent profile whose agent is this
            // bridge; save it as ~/.bboxy450-harness/profiles/<name>.md and
            // spawn seats with harness_profile <name>. Session defaults to the
            // seat id (BBOXY_SEAT_ID), so one profile serves every seat.
            let (Ok(_), Some(name), Some(swarm), Some(cwd)) = (
                transport(&args),
                flag(&args, "--name"),
                flag(&args, "--swarm"),
                flag(&args, "--cwd"),
            ) else {
                return usage();
            };
            let me = std::env::current_exe()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| "bboxy450-term-remote".into());
            let mut command = vec![me, "attach".into()];
            for f in ["--ssh", "--via", "--local", "--state"] {
                if let Some(v) = flag(&args, f) {
                    command.push(f.into());
                    command.push(v);
                }
            }
            command.extend(["--swarm".into(), swarm, "--cwd".into(), cwd]);
            let yaml: Vec<String> = command
                .iter()
                .map(|a| serde_json::to_string(a).unwrap())
                .collect();
            println!(
                "---\nname: {name}\nprovider: acp-agent\npolicy: yolo\nagent:\n  command: [{}]\n---\nRemote seat: bboxy450-harness acp on the swarm's remote supervisor.",
                yaml.join(", ")
            );
            return ExitCode::SUCCESS;
        }
        Some("install") => {
            let Ok(t) = transport(&args) else {
                return usage();
            };
            let me = match std::env::current_exe() {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("bboxy450-term-remote install: {e}");
                    return ExitCode::FAILURE;
                }
            };
            let harness = flag(&args, "--harness").map(PathBuf::from);
            let mut files: Vec<(&Path, &str)> = vec![(me.as_path(), "bboxy450-term-remote")];
            if let Some(h) = harness.as_deref() {
                files.push((h, "bboxy450-harness"));
            }
            return match bboxy450_term_remote::install::install(&t, &files) {
                Ok(there) => {
                    println!("installed on {there}");
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("bboxy450-term-remote install: {e}");
                    ExitCode::FAILURE
                }
            };
        }
        _ => {}
    }
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (cmd, state) = match args.as_slice() {
        [cmd, flag, dir] if flag == "--state" && (cmd == "serve" || cmd == "daemon") => {
            (cmd.as_str(), PathBuf::from(dir))
        }
        _ => return usage(),
    };
    let result = if cmd == "daemon" {
        daemon(&state)
    } else {
        serve(&state)
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("bboxy450-term-remote {cmd}: {e}");
            ExitCode::FAILURE
        }
    }
}
