// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Owned-child exit notifications on the actor's existing macOS kqueue.

use std::io;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};
use std::time::{Duration, Instant};

use mio::{Registry, Token};

pub(super) struct Watch {
    queue: RawFd,
    pid: u32,
    token: Token,
    armed: bool,
}

impl Watch {
    /// None means the owned child is already exiting/unattachable (ESRCH).
    /// Its status is still ours to reap nonblockingly as PTY output drains.
    pub(super) fn register(
        registry: &Registry,
        pid: u32,
        token: Token,
    ) -> io::Result<Option<Self>> {
        let queue = registry.as_raw_fd();
        match change(queue, pid, token, libc::EV_ADD | libc::EV_ONESHOT) {
            Ok(()) => Ok(Some(Self {
                queue,
                pid,
                token,
                armed: true,
            })),
            Err(error) if error.raw_os_error() == Some(libc::ESRCH) => Ok(None),
            Err(error) => Err(error),
        }
    }

    pub(super) fn token(&self) -> Token {
        self.token
    }

    /// EV_ONESHOT already removed this watch. Disarm before wait() releases the
    /// PID, so a retained old seat can never delete a reused PID's newer watch.
    pub(super) fn completed(mut self) {
        self.armed = false;
    }
}

impl Drop for Watch {
    fn drop(&mut self) {
        if self.armed {
            // Actor drops seat watches before their children and before Poll.
            // ENOENT/ESRCH is harmless if the kernel already removed a watch.
            let _ = change(self.queue, self.pid, self.token, libc::EV_DELETE);
        }
    }
}

/// Shutdown only: the actor's normal watches are already disarmed. A temporary
/// queue waits for this owned PID without consuming another seat's events.
/// The subsequent try_wait is nonblocking even if the kernel is still exiting.
pub(super) fn exited_within(pid: u32, timeout: Duration) -> io::Result<bool> {
    // SAFETY: kqueue returns a new owned descriptor, closed by OwnedFd.
    let fd = unsafe { libc::kqueue() };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    let queue = unsafe { OwnedFd::from_raw_fd(fd) };
    match change(
        queue.as_raw_fd(),
        pid,
        Token(0),
        libc::EV_ADD | libc::EV_ONESHOT,
    ) {
        Ok(()) => {}
        Err(error) if error.raw_os_error() == Some(libc::ESRCH) => return Ok(true),
        Err(error) => return Err(error),
    }
    let until = Instant::now() + timeout;
    loop {
        let left = until.saturating_duration_since(Instant::now());
        let timeout = libc::timespec {
            tv_sec: left.as_secs() as libc::time_t,
            tv_nsec: left.subsec_nanos() as libc::c_long,
        };
        // SAFETY: one valid output event and timeout on this private queue.
        let mut event: libc::kevent = unsafe { std::mem::zeroed() };
        let count = unsafe {
            libc::kevent(
                queue.as_raw_fd(),
                std::ptr::null(),
                0,
                &mut event,
                1,
                &timeout,
            )
        };
        if count >= 0 {
            return Ok(count == 1 && event.fflags & libc::NOTE_EXIT != 0);
        }
        let error = io::Error::last_os_error();
        if error.kind() != io::ErrorKind::Interrupted {
            return Err(error);
        }
        if Instant::now() >= until {
            return Ok(false);
        }
    }
}

fn change(queue: RawFd, pid: u32, token: Token, flags: u16) -> io::Result<()> {
    let event = libc::kevent {
        ident: pid as libc::uintptr_t,
        filter: libc::EVFILT_PROC,
        flags,
        fflags: libc::NOTE_EXIT,
        data: 0,
        udata: token.0 as *mut libc::c_void,
    };
    // SAFETY: queue is borrowed from the live actor's Registry; pid is its own
    // unreaped child. Token is opaque user data, never dereferenced. One valid
    // change is supplied; this call does not retrieve or consume poll events.
    if unsafe { libc::kevent(queue, &event, 1, std::ptr::null_mut(), 0, std::ptr::null()) } < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // keeps: spec mux-daemon.md — Ownership and failure boundaries
    #[ignore = "night native boundary: exact selector in scripts/night-test-boundaries.sh"]
    #[test]
    fn a_live_child_cannot_make_the_shutdown_wait_infinite() {
        let mut child = std::process::Command::new("/bin/cat")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap();
        let ready = exited_within(child.id(), Duration::ZERO).unwrap();
        child.kill().unwrap();
        child.wait().unwrap();
        assert!(!ready, "live child falsely reported as exited");
    }
}
