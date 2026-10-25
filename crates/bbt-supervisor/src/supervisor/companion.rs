//! Generic seat-owned services. Startup never waits for service readiness;
//! running children use the actor's existing process-exit notifications.

use std::io;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use crate::protocol::{CompanionInfo, Failure, FailureCode, LaunchSpec};

pub(super) struct Companion {
    child: Child,
    lifetime: Option<OwnedFd>,
    info: CompanionInfo,
    reaped: bool,
    pub(super) notified: bool,
    stopping: Option<Instant>,
    #[cfg(target_os = "macos")]
    pub(super) watch: Option<super::child_events::Watch>,
}

impl Companion {
    pub(super) fn start(spec: &LaunchSpec) -> Result<Option<Self>, Failure> {
        let Some(service) = &spec.companion else {
            return Ok(None);
        };
        if service.ready_socket.exists() {
            return Err(Failure::new(
                FailureCode::Io,
                "companion socket already exists",
            ));
        }
        let (lifetime, slave) = lifetime_pty().map_err(super::pty_error)?;
        let slave_fd = slave.as_raw_fd();
        let mut command = Command::new(&service.program);
        command
            .args(&service.args)
            .current_dir(&spec.cwd)
            .envs(&spec.env)
            .envs(&service.env)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        // The private tty is a lifetime guard, never input or output. Closing its
        // sole master on supervisor death hangs up this service's own group.
        // Keep the slave across exec: closing it loses the hangup association on
        // macOS. All stdio stays null; the operator's tty is never inherited.
        // SAFETY: these calls operate on the new child's session and owned FD
        // between fork and exec, without allocation or shared Rust locks.
        unsafe {
            command.pre_exec(move || {
                if libc::setsid() == -1
                    || libc::ioctl(slave_fd, libc::TIOCSCTTY as _, 0) == -1
                    || libc::tcsetpgrp(slave_fd, libc::getpgrp()) == -1
                    || libc::signal(libc::SIGHUP, libc::SIG_DFL) == libc::SIG_ERR
                    || libc::fcntl(slave_fd, libc::F_SETFD, 0) == -1
                {
                    return Err(io::Error::last_os_error());
                }
                Ok(())
            });
        }
        let child = command.spawn().map_err(super::pty_error)?;
        drop(slave);
        Ok(Some(Self {
            info: CompanionInfo {
                pid: child.id(),
                ready_socket: service.ready_socket.clone(),
            },
            child,
            lifetime: Some(lifetime),
            reaped: false,
            notified: false,
            stopping: None,
            #[cfg(target_os = "macos")]
            watch: None,
        }))
    }

    pub(super) fn info(&self) -> CompanionInfo {
        self.info.clone()
    }
    pub(super) fn reaped(&self) -> bool {
        self.reaped
    }

    pub(super) fn stop(&mut self) {
        if !self.reaped && self.stopping.is_none() {
            // Release tty output before waiting for child shutdown; a session
            // leader can otherwise remain in tty close with buffered output.
            self.lifetime.take();
            self.signal(libc::SIGTERM);
            self.stopping = Some(Instant::now());
        }
    }

    fn signal(&self, signal: libc::c_int) {
        // SAFETY: the child is owned/unreaped and was made its own process group.
        // No PID lookup, pattern kill, or user-global service is involved.
        unsafe {
            libc::kill(-(self.info.pid as libc::pid_t), signal);
        }
    }

    pub(super) fn poll_exit(&mut self) -> io::Result<bool> {
        if self.reaped {
            return Ok(true);
        }
        if self
            .stopping
            .is_some_and(|at| at.elapsed() >= Duration::from_millis(250))
        {
            self.signal(libc::SIGKILL);
            #[cfg(not(target_os = "macos"))]
            {
                self.notified = true;
            }
        }
        if self.notified && self.child.try_wait()?.is_some() {
            self.reaped = true;
            self.lifetime.take();
        }
        Ok(self.reaped)
    }

    #[cfg(target_os = "macos")]
    pub(super) fn watch(
        &mut self,
        registry: &mio::Registry,
        token: mio::Token,
    ) -> io::Result<bool> {
        self.watch = super::child_events::Watch::register(registry, self.info.pid, token)?;
        if self.watch.is_none() {
            self.notified = true;
            self.poll_exit()?;
        }
        Ok(self.watch.is_some())
    }

    #[cfg(target_os = "macos")]
    pub(super) fn complete_watch(&mut self) {
        // The watched child is still unreaped, so its process group is ours.
        if !self.reaped {
            self.signal(libc::SIGKILL);
        }
        if let Some(watch) = self.watch.take() {
            watch.completed();
        }
        self.notified = true;
    }
}

fn lifetime_pty() -> io::Result<(OwnedFd, OwnedFd)> {
    let (mut master, mut slave) = (-1, -1);
    // SAFETY: openpty initializes both descriptor out-parameters; no optional
    // name, termios or window-size memory is supplied.
    if unsafe {
        libc::openpty(
            &mut master,
            &mut slave,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    } == -1
    {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: successful openpty returned two distinct owned descriptors.
    let pair = unsafe { (OwnedFd::from_raw_fd(master), OwnedFd::from_raw_fd(slave)) };
    for fd in [&pair.0, &pair.1] {
        // SAFETY: fcntl changes only our newly created descriptor. The child
        // clears CLOEXEC for its slave after installing the controlling tty.
        if unsafe { libc::fcntl(fd.as_raw_fd(), libc::F_SETFD, libc::FD_CLOEXEC) } == -1 {
            return Err(io::Error::last_os_error());
        }
    }
    Ok(pair)
}

impl Drop for Companion {
    fn drop(&mut self) {
        #[cfg(target_os = "macos")]
        drop(self.watch.take());
        self.lifetime.take();
        if !self.reaped {
            self.signal(libc::SIGKILL);
            let _ = self.child.wait();
            self.reaped = true;
        }
    }
}
