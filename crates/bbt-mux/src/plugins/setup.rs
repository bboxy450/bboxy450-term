// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Plugin setup (#57): the trusted pinned copy's own auth status command,
//! then, when not logged in, its login command in a bare login terminal.
//! Setup never trusts or enables anything; its result is recorded per pin.
//!
//! The bare login terminal is a private PTY owned by this job: no harness
//! adapter, no MCP, hooks or skills, no OSC observation (its output is kept
//! only as bounded text with escapes removed), a scrubbed environment and the
//! plugin's own data directory as cwd. The login's whole process group is
//! killed when it exits or exceeds its budget, so no child outlives setup.

use std::io::Read;
use std::os::fd::{FromRawFd, OwnedFd};
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use bbt_plugin::policy::allowed_env_name;
use bbt_plugin::registry::{Activation, Entry, SetupResult, SetupState};
use bbt_plugin::shape::Auth;
use bbt_plugin::trust::TrustRecord;
use rusqlite::TransactionBehavior;

use super::{Refusal, Registry, activation_key, failure, load, put, trust_key};

/// The status command's fixed budget (tests may raise it like the other
/// fixed plugin budgets).
const STATUS: Duration = Duration::from_secs(2);
/// A login may wait for the operator, e.g. a browser device flow.
pub const LOGIN: Duration = Duration::from_secs(120);
/// Bounded captured output of either command.
const OUTPUT: usize = 4096;

pub(super) fn key(candidate: &str) -> String {
    format!("plugins/setup/{candidate}")
}

/// One admitted setup run for one trusted pin.
pub struct SetupJob {
    activation: Activation,
    record: TrustRecord,
    copy: PathBuf,
    auth: Auth,
    /// How long the login may run; [`LOGIN`] unless a test shortens it.
    pub login_budget: Duration,
    /// PIDs this job started (status and login), for cleanup assertions.
    pub pids: Vec<u32>,
}

/// What a finished job observed, before it is recorded.
pub struct SetupRun {
    activation: Activation,
    record: TrustRecord,
    pub result: SetupResult,
    pub pids: Vec<u32>,
    /// The login terminal's bounded output, escapes removed.
    pub login_output: String,
}

struct Ran {
    exit: Option<i32>,
    output: String,
    timed_out: bool,
}

// Remove escape sequences and control bytes: a login terminal is never
// observed for OSC, titles, notifications or clipboard writes.
fn plain(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes);
    let mut out = String::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            // CSI/OSC/other: skip to a terminator.
            match chars.next() {
                Some('[') => {
                    for c in chars.by_ref() {
                        if ('@'..='~').contains(&c) {
                            break;
                        }
                    }
                }
                Some(']') => {
                    while let Some(c) = chars.next() {
                        if c == '\u{7}' || c == '\u{1b}' && chars.next_if_eq(&'\\').is_some() {
                            break;
                        }
                    }
                }
                _ => (),
            }
        } else if c == '\n' || c == '\t' || !c.is_control() {
            out.push(c);
        }
    }
    out
}

fn first_line(text: &str) -> String {
    text.lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("")
        .chars()
        .take(200)
        .collect()
}

impl SetupJob {
    fn command(&self, word: &str) -> Command {
        let data_dir = PathBuf::from(&self.record.data_dir);
        let home = data_dir.join("home");
        let _ = std::fs::create_dir_all(home.join("tmp"));
        let mut command = Command::new(&self.copy);
        command
            .arg(word)
            .current_dir(&data_dir)
            .env_clear()
            .envs(std::env::vars_os().filter(|(key, _)| key.to_str().is_some_and(allowed_env_name)))
            .env("HOME", &home)
            .env("TMPDIR", home.join("tmp"));
        command
    }

    // Wait at most `budget`, then end the whole process group; always reap.
    fn wait(child: &mut Child, budget: Duration) -> (Option<i32>, bool) {
        let deadline = Instant::now() + budget;
        let group = child.id() as i32;
        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break Some(status),
                Ok(None) if Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(10));
                }
                _ => break None,
            }
        };
        let timed_out = status.is_none();
        // SAFETY: signals only this job's own process group (setsid in the child).
        unsafe { libc::kill(-group, libc::SIGTERM) };
        let status = status.or_else(|| {
            let grace = Instant::now() + Duration::from_secs(1);
            while Instant::now() < grace {
                if let Ok(Some(status)) = child.try_wait() {
                    return Some(status);
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            None
        });
        // SAFETY: as above; ESRCH when the group is already gone.
        unsafe { libc::kill(-group, libc::SIGKILL) };
        let reaped = child.wait().ok();
        (status.or(reaped).and_then(|s| s.code()), timed_out)
    }

    fn status(&mut self) -> Result<Ran, Refusal> {
        let mut command = self.command(&self.auth.status_command.clone());
        command
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .process_group(0);
        let mut child = command
            .spawn()
            .map_err(|e| failure("setup_spawn_failed", e))?;
        self.pids.push(child.id());
        let mut stdout = child.stdout.take().expect("piped stdout");
        let reader = std::thread::spawn(move || {
            let mut bytes = Vec::new();
            let _ = stdout.by_ref().take(OUTPUT as u64).read_to_end(&mut bytes);
            bytes
        });
        let (exit, timed_out) = Self::wait(&mut child, crate::plugins::client::fixed(STATUS));
        let output = plain(&reader.join().unwrap_or_default());
        Ok(Ran {
            exit,
            output,
            timed_out,
        })
    }

    // The bare login terminal: a private PTY with the plugin's login command
    // as its session leader and controlling terminal.
    fn login(&mut self) -> Result<Ran, Refusal> {
        let (mut master, mut slave) = (-1, -1);
        // SAFETY: openpty fills two fresh descriptors; null name/termios/size.
        if unsafe {
            libc::openpty(
                &mut master,
                &mut slave,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        } != 0
        {
            return Err(failure(
                "setup_pty_unavailable",
                std::io::Error::last_os_error(),
            ));
        }
        // SAFETY: both descriptors were just returned by openpty and are owned here.
        let (master, slave) =
            unsafe { (OwnedFd::from_raw_fd(master), OwnedFd::from_raw_fd(slave)) };
        let terminal = |fd: &OwnedFd| {
            fd.try_clone()
                .map(Stdio::from)
                .map_err(|e| failure("setup_pty_unavailable", e))
        };
        let mut command = self.command(&self.auth.login_command.clone());
        command
            .env("TERM", "dumb")
            .stdin(terminal(&slave)?)
            .stdout(terminal(&slave)?)
            .stderr(terminal(&slave)?);
        // SAFETY: async-signal-safe calls only: a new session whose controlling
        // terminal is the PTY on stdin.
        unsafe {
            command.pre_exec(|| {
                if libc::setsid() < 0 || libc::ioctl(0, libc::TIOCSCTTY as _, 0) < 0 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
        let mut child = command
            .spawn()
            .map_err(|e| failure("setup_spawn_failed", e))?;
        drop(command);
        drop(slave);
        self.pids.push(child.id());
        let reader = std::thread::spawn(move || {
            let mut file = std::fs::File::from(master);
            let mut bytes = Vec::new();
            let mut part = [0; 1024];
            // EIO once every slave descriptor is closed: the session ended.
            while let Ok(count) = file.read(&mut part) {
                if count == 0 {
                    break;
                }
                let take = count.min(OUTPUT.saturating_sub(bytes.len()));
                bytes.extend_from_slice(&part[..take]);
            }
            bytes
        });
        let (exit, timed_out) = Self::wait(&mut child, self.login_budget);
        let output = plain(&reader.join().unwrap_or_default());
        Ok(Ran {
            exit,
            output,
            timed_out,
        })
    }
}

impl Registry {
    /// Admit setup for a trusted, current pin whose shape declares `auth`.
    /// Trust and enable are untouched; a disabled plugin can be set up.
    pub fn setup(&self, candidate: &str) -> Result<SetupJob, Refusal> {
        bbt_rt::assert_not_ui_thread();
        let entry = self.entry(candidate)?;
        let (activation, record) = self.authority(&entry)?;
        let auth = record
            .shape
            .as_ref()
            .and_then(|shape| shape.auth.clone())
            .ok_or_else(|| failure("setup_unavailable", "the trusted shape declares no auth"))?;
        auth.validate()?;
        let copy = self.verify_copy(record)?;
        Ok(SetupJob {
            activation: activation.clone(),
            record: record.clone(),
            copy,
            auth,
            login_budget: LOGIN,
            pids: Vec::new(),
        })
    }

    /// Record a finished run if the pin it ran against is still current.
    pub fn finish_setup(&self, run: SetupRun) -> Result<Entry, Refusal> {
        bbt_rt::assert_not_ui_thread();
        let mut db = self
            .account
            .connection()
            .map_err(|e| failure("plugin_store_unavailable", e))?;
        let tx = db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| failure("plugin_store_unavailable", e))?;
        let current: Option<Activation> = load(&tx, &activation_key(&run.activation.candidate))?;
        let trust: Option<TrustRecord> = load(&tx, &trust_key(&run.record.sha256))?;
        if current.as_ref() != Some(&run.activation) || trust.as_ref() != Some(&run.record) {
            return Err(failure(
                "stale_plugin_completion",
                "plugin authority changed during setup",
            ));
        }
        put(&tx, &key(&run.activation.candidate), &run.result)?;
        tx.commit()
            .map_err(|e| failure("plugin_store_unavailable", e))?;
        self.entry(&run.activation.candidate)
    }
}

impl SetupJob {
    /// Status, then login when not logged in, then status again. Blocking.
    pub fn run(mut self) -> SetupRun {
        let started = Instant::now();
        let mut login_exit = None;
        let mut login_output = String::new();
        let result = (|| -> Result<(SetupState, Option<String>, String), Refusal> {
            let first = self.status()?;
            if first.timed_out {
                return Ok((
                    SetupState::Failed,
                    Some("status_deadline_exceeded".into()),
                    String::new(),
                ));
            }
            match first.exit {
                Some(0) => return Ok((SetupState::LoggedIn, None, first_line(&first.output))),
                Some(1) => (),
                other => {
                    return Ok((
                        SetupState::Failed,
                        Some(format!(
                            "status_exit_{}",
                            other.map_or("signal".into(), |c| c.to_string())
                        )),
                        first_line(&first.output),
                    ));
                }
            }
            let login = self.login()?;
            login_output = login.output.chars().take(OUTPUT).collect();
            login_exit = login.exit;
            if login.timed_out {
                return Ok((
                    SetupState::Failed,
                    Some("login_deadline_exceeded".into()),
                    String::new(),
                ));
            }
            if login.exit != Some(0) {
                return Ok((SetupState::LoginRefused, None, first_line(&login_output)));
            }
            let after = self.status()?;
            Ok(match after.exit {
                Some(0) => (SetupState::LoggedIn, None, first_line(&after.output)),
                Some(1) => (SetupState::LoggedOut, None, first_line(&after.output)),
                _ => (
                    SetupState::Failed,
                    Some("status_after_login_failed".into()),
                    String::new(),
                ),
            })
        })();
        let (state, code, detail) =
            result.unwrap_or_else(|reason| (SetupState::Failed, Some(reason.code), String::new()));
        SetupRun {
            result: SetupResult {
                sha256: self.activation.sha256.clone(),
                state,
                code,
                login_exit,
                detail,
                elapsed_ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
            },
            activation: self.activation,
            record: self.record,
            pids: self.pids,
            login_output,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn login_output_never_carries_escapes() {
        let text = plain(b"\x1b]52;c;c2VjcmV0\x07Open https://example.invalid\x1b[1m code\x1b[0m\r\n\x1b]0;title\x1b\\done");
        assert_eq!(text, "Open https://example.invalid code\ndone");
        assert_eq!(first_line("\n  hello  \nworld"), "hello");
    }
}
