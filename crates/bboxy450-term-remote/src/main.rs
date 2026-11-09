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
            std::process::Command::new(std::env::current_exe()?)
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

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (cmd, state) = match args.as_slice() {
        [cmd, flag, dir] if flag == "--state" && (cmd == "serve" || cmd == "daemon") => {
            (cmd.as_str(), PathBuf::from(dir))
        }
        _ => {
            eprintln!("usage: bboxy450-term-remote (serve|daemon) --state DIR");
            return ExitCode::from(2);
        }
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
