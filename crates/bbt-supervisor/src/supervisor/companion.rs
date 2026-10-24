//! Generic seat-owned services. Startup never waits for service readiness;
//! running children use the actor's existing process-exit notifications.

use std::io;
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use crate::protocol::{CompanionInfo, Failure, FailureCode, LaunchSpec};

pub(super) struct Companion {
    child: Child,
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
        let mut command = Command::new(&service.program);
        command
            .args(&service.args)
            .current_dir(&spec.cwd)
            .envs(&spec.env)
            .envs(&service.env)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        // A side service has no controlling tty. Hooks must use their caller's
        // authenticated socket sink, never inherit the launching terminal.
        // SAFETY: setsid is async-signal-safe; this runs only in the new child
        // between fork and exec, without allocation or shared Rust locks.
        unsafe {
            command.pre_exec(|| {
                if libc::setsid() == -1 {
                    return Err(io::Error::last_os_error());
                }
                Ok(())
            });
        }
        let child = command.spawn().map_err(super::pty_error)?;
        Ok(Some(Self {
            info: CompanionInfo {
                pid: child.id(),
                ready_socket: service.ready_socket.clone(),
            },
            child,
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

impl Drop for Companion {
    fn drop(&mut self) {
        #[cfg(target_os = "macos")]
        drop(self.watch.take());
        if !self.reaped {
            self.signal(libc::SIGKILL);
            let _ = self.child.wait();
            self.reaped = true;
        }
    }
}
