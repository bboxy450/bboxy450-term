// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
#![allow(
    clippy::disallowed_methods,
    reason = "bounded GUI startup/lifecycle IO, before the UI loop exists"
)]
//! Replace only the GUI. No operator ctl door, capability file, or backend stop.
use super::*;
use std::{io::Read, process::Command as Process, time::Instant};
const TIMEOUT: Duration = Duration::from_secs(35);
fn lock_owner(run: &Path) -> Result<u32> {
    let mut child = Process::new("/usr/sbin/lsof")
        .args(["-t", "--"])
        .arg(run.join("app.owner.lock"))
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()?;
    let started = Instant::now();
    while child.try_wait()?.is_none() {
        if started.elapsed() >= Duration::from_secs(3) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(invalid("GUI lock observation timed out"));
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let mut bytes = Vec::new();
    child
        .stdout
        .take()
        .ok_or_else(|| invalid("GUI lock observation unavailable"))?
        .take(4096)
        .read_to_end(&mut bytes)?;
    let mut pids: Vec<u32> = String::from_utf8_lossy(&bytes)
        .lines()
        .filter_map(|s| s.parse().ok())
        .filter(|pid| *pid != std::process::id())
        .collect();
    pids.sort_unstable();
    pids.dedup();
    if pids.len() != 1 {
        return Err(invalid("GUI replacement requires one exact lock owner"));
    }
    Ok(pids[0])
}
fn current(run: &Path) -> Result<Option<owners::ProcessIdentity>> {
    if super::frontend_running(run)? != Some(true) {
        return Ok(None);
    }
    // Identify the holder of the GUI-only lease, not an agent/backend PID from
    // an arbitrary file. This also supports legacy GUIs without the receipt.
    let pid = lock_owner(run)?;
    let path = run.join("gui.owner.json");
    match owners::public_gui_file(&path) {
        Ok(mut file) => {
            let mut bytes = Vec::new();
            file.by_ref().take(4096).read_to_end(&mut bytes)?;
            let value: serde_json::Value = serde_json::from_slice(&bytes)?;
            let identity: owners::ProcessIdentity =
                serde_json::from_value(value["process"].clone())?;
            // A retained receipt can precede a legacy GUI which does not write
            // one. Only the actual lease holder and its kernel birth authorize
            // replacement; the receipt is evidence, never signal authority.
            Ok(Some(if identity.pid() == pid && identity.alive() {
                identity
            } else {
                owners::ProcessIdentity::read(pid)?
            }))
        }
        Err(error)
            if error
                .downcast_ref::<std::io::Error>()
                .is_some_and(|e| e.kind() == std::io::ErrorKind::NotFound) =>
        {
            Ok(Some(owners::ProcessIdentity::read(pid)?))
        }
        Err(error) => Err(error),
    }
}
fn run_dir(args: &[String]) -> Result<PathBuf> {
    args.windows(2)
        .find(|p| p[0] == "--run-dir")
        .map(|p| Ok(PathBuf::from(&p[1])))
        .unwrap_or_else(|| {
            Ok(bbt_mux::transport::default_socket_path()?
                .parent()
                .ok_or_else(|| invalid("run directory unavailable"))?
                .to_owned())
        })
}
/// Credential-free process/lock observation for the canonical installer.
pub fn probe(args: &[String]) -> Result<Option<serde_json::Value>> {
    if !args.iter().any(|s| s == "--gui-handoff-probe") {
        return Ok(None);
    }
    let run = run_dir(args)?;
    let owner = current(&run)?;
    let gui = if owner.is_some() {
        owners::public_gui_file(&run.join("gui.owner.json"))
            .ok()
            .and_then(|mut f| {
                let mut b = Vec::new();
                f.by_ref().take(4096).read_to_end(&mut b).ok()?;
                serde_json::from_slice::<serde_json::Value>(&b)
                    .ok()
                    .filter(|v| {
                        serde_json::from_value::<owners::ProcessIdentity>(v["process"].clone())
                            .ok()
                            .as_ref()
                            == owner.as_ref()
                    })
                    .map(|v| serde_json::json!({"gui":v["gui"],"repository":v["repository"]}))
            })
    } else {
        None
    };
    Ok(Some(
        serde_json::json!({"running":owner.is_some(),"process":owner,"gui":gui.as_ref().map(|v|&v["gui"]),"repository":gui.as_ref().map(|v|&v["repository"])}),
    ))
}
/// Explicit normal GUI startup action, before claiming its lock. No ctl/capability door.
/// The installer passes its captured public owner identity to fence concurrent replacement.
pub fn prepare(args: &[String]) -> Result<()> {
    if !args.iter().any(|s| s == "--replace-gui") {
        return Ok(());
    }
    let root = run_dir(args)?;
    let owner = current(&root)?;
    if let Some(expected) = args.windows(2).find(|p| p[0] == "--replace-gui-owner") {
        let expected: Option<owners::ProcessIdentity> = serde_json::from_str(&expected[1])?;
        if owner != expected {
            return Err(invalid(
                "GUI owner changed after install preflight; replacement refused",
            ));
        }
    }
    let Some(owner) = owner else {
        return Ok(());
    };
    // SIGHUP closes ONLY the exact GUI lock holder. Current GUIs use CloseWindow
    // and save/detach; legacy default HUP exits only this PID. Never a process group.
    if !owner.alive() {
        return Ok(());
    }
    owner.signal(libc::SIGHUP)?;
    let started = Instant::now();
    while owner.alive() || super::frontend_running(&root)? == Some(true) {
        if started.elapsed() >= TIMEOUT {
            return Err(invalid("GUI detach did not complete; no force termination"));
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    Ok(())
}
