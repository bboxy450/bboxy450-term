// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Exact ownership of backend processes started by a private app fixture.
#![allow(
    clippy::disallowed_methods,
    reason = "private integration fixture cleanup, off the UI thread"
)]

use bboxy450_term::cockpit::Process;
use serde::Deserialize;
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Child,
    time::{Duration, Instant},
};

pub(crate) fn backend_owners(run: &Path) -> Vec<Process> {
    #[derive(Deserialize)]
    struct Owner {
        run: PathBuf,
        process: Value,
    }
    ["logic", "supervisor", "desktop"]
        .into_iter()
        .filter_map(|name| {
            let record = run.join(format!("{name}.owner.json"));
            let owner: Owner = serde_json::from_slice(&fs::read(&record).ok()?).ok()?;
            if owner.run != run {
                return None;
            }
            // Reuse the app's PID and kernel birth verification. Never use a
            // numeric PID from a stale record as the authority to signal it.
            let process = Process::Adopted {
                identity: serde_json::from_value(owner.process).ok()?,
                record,
            };
            process
                .id()
                .is_some_and(|pid| (2..=i32::MAX as u32).contains(&pid))
                .then_some(process)
        })
        .collect()
}

fn wait_for_exit(mut exited: impl FnMut() -> bool) -> bool {
    let deadline = Instant::now() + Duration::from_secs(2);
    while !exited() {
        if Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    true
}

pub(crate) fn retire_backends(mut owners: Vec<Process>) {
    for owner in &mut owners {
        if matches!(owner.try_wait(), Ok(None))
            && let Some(pid) = owner.id()
        {
            // SAFETY: the private run record and the current kernel birth
            // identified this exact backend before requesting orderly exit.
            unsafe { libc::kill(pid as i32, libc::SIGTERM) };
        }
    }
    let departed = |owner: &mut Process| owner.try_wait().is_ok_and(|exit| exit.is_some());
    if !wait_for_exit(|| owners.iter_mut().all(departed)) {
        for owner in &mut owners {
            // The existing adopted-process primitive revalidates birth.
            let _ = owner.start_kill();
        }
        if !wait_for_exit(|| owners.iter_mut().all(departed)) {
            eprintln!("private backend cleanup did not observe process exit");
        }
    }
}

/// The directly spawned app is the leader of a fixture-owned process group.
/// Signal the group while its leader is still an unreaped child.
pub(crate) fn retire_app_group(mut child: Child) {
    let group = -(child.id() as i32);
    if matches!(child.try_wait(), Ok(None)) {
        // SAFETY: the fixture set process_group(0) before spawning this child.
        unsafe { libc::kill(group, libc::SIGKILL) };
    }
    let _ = child.wait();
}

/// Cleanup for tests that start cockpit directly and exercise production detach.
pub(crate) struct BackendGuard(pub PathBuf);

impl Drop for BackendGuard {
    fn drop(&mut self) {
        retire_backends(backend_owners(&self.0));
    }
}
