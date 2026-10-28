// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
#![allow(
    clippy::disallowed_methods,
    reason = "desktop process bootstrap and socket/file operations execute on IO, never the window thread"
)]
//! A native desktop daemon follows the PTY supervisor, independently of the UI.
//! Its private UI socket admits only the recorded desktop process, never a seat.
use super::*;
use bboxy450_term_desktop::Action;
use std::{
    fs::OpenOptions,
    os::{
        fd::{AsRawFd, FromRawFd, OwnedFd},
        unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt},
    },
    sync::Arc,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt},
    net::UnixListener,
    task::JoinHandle,
};

const SOCKET: &str = "desktop-ui.sock";
const VERSION: u16 = 1;

#[derive(Serialize, Deserialize)]
struct Ready {
    version: u16,
    supervisor_pid: u32,
}

/// Retained by the GUI's IO task. Dropping a UI never stops the desktop daemon.
pub struct UiPort {
    task: JoinHandle<()>,
    path: PathBuf,
    inode: u64,
}
impl Drop for UiPort {
    fn drop(&mut self) {
        self.task.abort();
        if std::fs::symlink_metadata(&self.path).is_ok_and(|m| m.ino() == self.inode) {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

fn enabled(config: &Config) -> bool {
    if cfg!(debug_assertions)
        && config.test_account.is_some()
        && std::env::args().any(|a| a == "--desktop-fixture")
    {
        return true;
    }
    cfg!(target_os = "macos")
        && config.test_account.is_none()
        && !std::env::args()
            .any(|a| matches!(a.as_str(), "--headless" | "--smoke" | "--unattended"))
}

/// Install the private callback socket without waiting on native initialization.
/// Ordinary fixtures and headless applications never create an OS status item.
pub fn attach(
    config: Config,
    supervisor: Arc<std::sync::atomic::AtomicU32>,
    callback: impl Fn(Action) + Send + Sync + 'static,
) -> Option<UiPort> {
    if !enabled(&config) {
        return None;
    }
    match bind(config.clone(), supervisor, Arc::new(callback)) {
        Ok(port) => {
            refresh(config);
            Some(port)
        }
        Err(error) => {
            eprintln!("desktop callback unavailable: {error}");
            None
        }
    }
}

fn bind(
    config: Config,
    supervisor: Arc<std::sync::atomic::AtomicU32>,
    callback: Arc<dyn Fn(Action) + Send + Sync>,
) -> Result<UiPort> {
    let path = config.run_dir.join(SOCKET);
    // The caller holds this run's FrontendLock; only a stale socket can remain.
    if let Ok(meta) = std::fs::symlink_metadata(&path) {
        use std::os::unix::fs::FileTypeExt;
        if !meta.file_type().is_socket() {
            return Err(invalid("desktop callback is not a socket"));
        }
        std::fs::remove_file(&path)?;
    }
    let listener = UnixListener::bind(&path)?;
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    let inode = std::fs::symlink_metadata(&path)?.ino();
    let budget = Arc::new(tokio::sync::Semaphore::new(16));
    let task = tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            let Ok(permit) = budget.clone().try_acquire_owned() else {
                continue;
            };
            let config = config.clone();
            let callback = callback.clone();
            let supervisor_pid = supervisor.load(std::sync::atomic::Ordering::Acquire);
            // A slow or malicious peer never holds the accept loop.
            tokio::spawn(async move {
                let _permit = permit;
                let _ = tokio::time::timeout(
                    Duration::from_secs(1),
                    receive(&config, supervisor_pid, stream, &*callback),
                )
                .await;
            });
        }
    });
    Ok(UiPort { task, path, inode })
}

async fn receive(
    config: &Config,
    supervisor_pid: u32,
    stream: UnixStream,
    callback: &(dyn Fn(Action) + Send + Sync),
) -> Result<()> {
    let peer = stream
        .peer_cred()?
        .pid()
        .and_then(|pid| u32::try_from(pid).ok());
    let owner = owners::read::<Ready>(config, "desktop")?
        .ok_or_else(|| invalid("desktop owner unavailable"))?;
    if peer != Some(owner.process.pid())
        || owner.ready.version != VERSION
        || owner.ready.supervisor_pid != supervisor_pid
        || !peer.is_some_and(|pid| outside_supervisor(pid, supervisor_pid))
    {
        return Err(invalid("desktop callback peer refused"));
    }
    let (read, mut write) = stream.into_split();
    let mut input = BufReader::new(read.take(128));
    let mut line = String::new();
    input.read_line(&mut line).await?;
    if line.len() > 64 || !line.ends_with('\n') {
        return Err(invalid("desktop action framing refused"));
    }
    let action: Action = serde_json::from_str(&line)?;
    callback(action);
    write.write_all(b"ok\n").await?;
    Ok(())
}

fn outside_supervisor(pid: u32, supervisor: u32) -> bool {
    outside_supervisor_with(
        pid,
        supervisor,
        std::process::id(),
        bbt_mux::ctl_door::parent,
    )
}
fn outside_supervisor_with(
    mut pid: u32,
    supervisor: u32,
    gui: u32,
    parent: impl Fn(u32) -> Option<u32>,
) -> bool {
    for _ in 0..128 {
        // A GUI launched from a seat can still own its private desktop helper,
        // just as ctl's GUI peer exception preserves its operator authority.
        if pid == gui {
            return true;
        }
        if pid == supervisor {
            return false;
        }
        match parent(pid) {
            Some(parent) if parent > 0 && parent != pid => pid = parent,
            _ => return true,
        }
    }
    false
}

async fn send(config: &Config, action: Action) -> Result<()> {
    tokio::time::timeout(Duration::from_secs(1), async {
        let mut socket = UnixStream::connect(config.run_dir.join(SOCKET)).await?;
        write_json(&mut socket, &action).await?;
        let mut reply = String::new();
        BufReader::new(socket.take(8)).read_line(&mut reply).await?;
        if reply != "ok\n" {
            return Err(invalid("desktop UI refused action"));
        }
        Ok(())
    })
    .await
    .map_err(|_| invalid("desktop UI timed out"))?
}

pub(super) fn refresh(config: Config) {
    if !enabled(&config) {
        return;
    }
    tokio::spawn(async move {
        if let Err(error) = ensure(&config).await {
            eprintln!("desktop unavailable: {error}");
        }
    });
}

async fn ensure(config: &Config) -> Result<()> {
    let supervisor = owners::read::<SupervisorReady>(config, "supervisor")?
        .ok_or_else(|| invalid("desktop supervisor unavailable"))?;
    if owners::read::<Ready>(config, "desktop")?.is_some_and(|owner| {
        owner.ready.version == VERSION && owner.ready.supervisor_pid == supervisor.process.pid()
    }) {
        return Ok(());
    }
    let path = std::env::var_os("PATH").unwrap_or_default();
    let mut child = super::child(config, "desktop", &path)?;
    write_json(child.stdin.as_mut().unwrap(), config).await?;
    let mut output = BufReader::new(child.stdout.take().unwrap());
    let line = tokio::time::timeout(STARTUP_TIMEOUT, read_line(&mut output))
        .await??
        .ok_or_else(|| invalid("desktop stopped during initialization"))?;
    let ready: Ready = serde_json::from_slice(&line)?;
    if ready.version != VERSION || ready.supervisor_pid != supervisor.process.pid() {
        return Err(invalid("desktop owner mismatch"));
    }
    let mut process = Process::Child(child);
    process.preserve_on_drop();
    Ok(())
}

fn claim(config: &Config) -> Result<std::fs::File> {
    let path = config.run_dir.join("desktop.owner.lock");
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(path)?;
    let meta = file.metadata()?;
    // SAFETY: geteuid has no pointer arguments; flock borrows the owned descriptor.
    if !meta.is_file() || meta.uid() != unsafe { libc::geteuid() } || meta.mode() & 0o077 != 0 {
        return Err(invalid("desktop lock must be private"));
    }
    if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
        return Err(invalid("desktop already running"));
    }
    Ok(file)
}

/// Private process entry. Native objects stay on its main thread, IO on bbt-rt.
pub(super) fn run() -> Result<()> {
    let mut input = BufReader::new(tokio::io::stdin());
    let config: Config = bbt_rt::handle().block_on(async {
        let line = read_line(&mut input)
            .await?
            .ok_or_else(|| invalid("missing desktop bootstrap"))?;
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(serde_json::from_slice(&line)?)
    })?;
    bbt_rt::error_journal::configure(&config.run_dir);
    let _lock = claim(&config)?;
    let (sender, mut actions) = mpsc::channel(32);
    let fixture_sender = sender.clone();
    let callback = Box::new(move |action| {
        if sender.try_send(action).is_err() {
            eprintln!("desktop action queue full");
        }
    });
    bboxy450_term_desktop::run_daemon(callback, move |stop| {
        let result = bbt_rt::handle().block_on(async {
            if let Some(error) = stop.native_error() { return Err(invalid(format!("desktop native backend: {error}"))); }
            let owner = owners::read::<SupervisorReady>(&config, "supervisor")?
                .ok_or_else(|| invalid("desktop supervisor unavailable"))?;
            let ready = Ready { version: VERSION, supervisor_pid: owner.process.pid() };
            owners::write(&config, "desktop", &ready)?;
            write_json(&mut tokio::io::stdout(), &ready).await?;
            use tokio::signal::unix::{signal, SignalKind};
            let mut terminate = signal(SignalKind::terminate())?;
            let ended = owner_exit(owner.process.pid());
            tokio::pin!(ended);
            let launching = Arc::new(std::sync::atomic::AtomicBool::new(false));
            let fixture = cfg!(debug_assertions) && config.test_account.is_some();
            loop {
                tokio::select! {
                    result = &mut ended => { result?; break; }
                    _ = terminate.recv() => break,
                    action = async {
                        loop {
                            match read_line(&mut input).await? {
                                Some(line) => return Ok::<_, Box<dyn std::error::Error + Send + Sync>>(serde_json::from_slice::<Action>(&line)?),
                                None => std::future::pending().await,
                            }
                        }
                    }, if fixture => {
                        fixture_sender.try_send(action?).map_err(|_| invalid("desktop fixture action queue full"))?;
                    }
                    Some(action) = actions.recv() => {
                        if action == Action::HardQuit {
                            if send(&config, action).await.is_ok() && matches!(tokio::time::timeout(Duration::from_secs(5), owner_exit(owner.process.pid())).await, Ok(Ok(()))) { break; }
                            stop_backends(&config).await?;
                            break;
                        }
                        let config = config.clone(); let launching = launching.clone();
                        tokio::spawn(async move {
                            if let Err(error) = show(&config, action, &launching).await { eprintln!("desktop action {}: {error}", action.as_str()); }
                        });
                    }
                }
            }
            owners::retire(&config, "desktop")?;
            Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
        });
        if let Err(error) = result { eprintln!("desktop stopped: {error}"); }
        stop.stop();
    }).map_err(|error| invalid(error.to_string()))
}

async fn show(
    config: &Config,
    action: Action,
    launching: &std::sync::atomic::AtomicBool,
) -> Result<()> {
    if send(config, Action::ShowWindow).await.is_ok() {
        if action != Action::ShowWindow {
            send(config, action).await?;
        }
        return Ok(());
    }
    use std::sync::atomic::Ordering;
    let launch = !launching.swap(true, Ordering::AcqRel);
    let result = async {
        if launch && !owners::frontend_running(&config.run_dir)?.unwrap_or(false) {
            let log = OpenOptions::new()
                .create(true)
                .append(true)
                .mode(0o600)
                .open(config.run_dir.join("app.log"))?;
            let mut command = Command::new(&config.executable);
            command
                .args(["--run-dir"])
                .arg(&config.run_dir)
                .arg("--repository")
                .arg(&config.repository)
                .current_dir(&config.repository)
                .stdin(Stdio::null())
                .stdout(log.try_clone()?)
                .stderr(log)
                .kill_on_drop(false);
            if config.ctl {
                command.arg("--ctl");
            }
            if let Some(account) = &config.test_account {
                command
                    .args([
                        "--headless",
                        "--unattended",
                        "--desktop-fixture",
                        "--test-account",
                    ])
                    .arg(account);
            }
            let child = command.spawn()?;
            if let Some(pid) = child.id() {
                let identity = owners::ProcessIdentity::read(pid)?;
                let file = OpenOptions::new()
                    .write(true)
                    .create(true)
                    .truncate(true)
                    .mode(0o600)
                    .open(config.run_dir.join("desktop-launched-ui.json"))?;
                serde_json::to_writer(file, &identity)?;
            }
        }
        tokio::time::timeout(STARTUP_TIMEOUT, async {
            loop {
                if send(config, Action::ShowWindow).await.is_ok() {
                    if action != Action::ShowWindow {
                        send(config, action).await?;
                    }
                    break Ok(());
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        })
        .await
        .map_err(|_| invalid("desktop UI did not become ready"))?
    }
    .await;
    if launch {
        launching.store(false, Ordering::Release);
    }
    result
}

async fn stop_backends(config: &Config) -> Result<()> {
    if let Some(owner) = owners::read::<LogicReady>(config, "logic")? {
        stop_owned(Process::adopted(config, "logic", owner)).await?;
    }
    if let Some(owner) = owners::read::<SupervisorReady>(config, "supervisor")? {
        stop_owned(Process::adopted(config, "supervisor", owner)).await?;
    }
    Ok(())
}
async fn stop_owned(mut process: Process) -> Result<()> {
    process.request_stop()?;
    match tokio::time::timeout(Duration::from_secs(5), process.wait()).await {
        Ok(result) => {
            result?;
        }
        Err(_) => process.kill().await?,
    }
    Ok(())
}

#[cfg(target_os = "macos")]
async fn owner_exit(pid: u32) -> Result<()> {
    // kqueue sleeps until process exit. There is no desktop lifetime polling timer.
    let raw = unsafe { libc::kqueue() };
    if raw < 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    let fd = unsafe { OwnedFd::from_raw_fd(raw) };
    unsafe {
        libc::fcntl(fd.as_raw_fd(), libc::F_SETFD, libc::FD_CLOEXEC);
    }
    let event = libc::kevent {
        ident: pid as _,
        filter: libc::EVFILT_PROC,
        flags: libc::EV_ADD | libc::EV_ENABLE | libc::EV_ONESHOT,
        fflags: libc::NOTE_EXIT,
        data: 0,
        udata: std::ptr::null_mut(),
    };
    if unsafe {
        libc::kevent(
            fd.as_raw_fd(),
            &event,
            1,
            std::ptr::null_mut(),
            0,
            std::ptr::null(),
        )
    } < 0
    {
        let error = std::io::Error::last_os_error();
        if error.raw_os_error() == Some(libc::ESRCH) {
            return Ok(());
        }
        return Err(error.into());
    }
    // SAFETY: AsyncFd owns this unique OwnedFd until deregistration; its descriptor
    // remains open and stable for the registration's entire lifetime.
    let watcher = unsafe {
        tokio::io::unix::AsyncFd::register_with_interest(fd, tokio::io::Interest::READABLE)
    }?;
    let _ready = watcher.readable().await?;
    Ok(())
}
#[cfg(not(target_os = "macos"))]
async fn owner_exit(_pid: u32) -> Result<()> {
    std::future::pending().await
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn desktop_helper_ancestry_keeps_the_gui_exception_and_refuses_seats() {
        for (pid, expected) in [(30, true), (40, false), (50, true), (10, true)] {
            assert_eq!(
                outside_supervisor_with(pid, 20, 10, |at| match at {
                    30 => Some(10),
                    10 => Some(20),
                    40 => Some(20),
                    20 => Some(1),
                    50 => Some(1),
                    _ => None,
                }),
                expected,
                "{pid}"
            );
        }
        assert!(!outside_supervisor_with(60, 20, 10, |at| Some(at + 1)));
    }
    #[tokio::test]
    async fn only_the_recorded_desktop_peer_can_deliver_an_action() {
        let root = tempfile::tempdir().unwrap();
        std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        let config = Config {
            executable: std::env::current_exe().unwrap(),
            socket: root.path().join("mux.sock"),
            run_dir: root.path().into(),
            repository: root.path().into(),
            shell_config: None,
            ctl: false,
            test_account: Some(root.path().join("account")),
        };
        owners::write(
            &config,
            "desktop",
            Ready {
                version: VERSION,
                supervisor_pid: 0,
            },
        )
        .unwrap();
        let (sender, mut received) = mpsc::unbounded_channel();
        let _port = bind(
            config.clone(),
            Arc::new(std::sync::atomic::AtomicU32::new(0)),
            Arc::new(move |action| {
                let _ = sender.send(action);
            }),
        )
        .unwrap();
        send(&config, Action::Settings).await.unwrap();
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(1), received.recv())
                .await
                .unwrap(),
            Some(Action::Settings)
        );
        // A separate real child has a different kernel peer PID despite the same uid.
        let output = tokio::process::Command::new("/usr/bin/python3")
            .args(["-c", "import socket,sys; s=socket.socket(socket.AF_UNIX); s.connect(sys.argv[1]); s.sendall(b'\\\"hard_quit\\\"\\n')", config.run_dir.join(SOCKET).to_str().unwrap()])
            .output().await.unwrap();
        assert!(output.status.success());
        assert!(
            tokio::time::timeout(Duration::from_millis(50), received.recv())
                .await
                .is_err()
        );
        assert!(!enabled(&config));
    }
}
