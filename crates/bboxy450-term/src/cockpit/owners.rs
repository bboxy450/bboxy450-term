#![allow(
    clippy::disallowed_methods,
    reason = "private run discovery and process observations execute in the cockpit IO owner"
)]
//! Private run discovery. Capabilities stay in same-user mode-0600 owner records.
use super::*;
use std::{
    fs::{File, OpenOptions},
    io::{Read, Write},
    os::{
        fd::AsRawFd,
        unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt},
    },
    path::Path,
};

#[derive(Clone, Serialize, Deserialize)]
pub struct ProcessIdentity {
    pid: u32,
    birth: (u64, u64),
}
impl ProcessIdentity {
    pub(super) fn read(pid: u32) -> Result<Self> {
        birth(pid)
            .map(|birth| Self { pid, birth })
            .ok_or_else(|| invalid("backend process identity unavailable"))
    }
    pub(super) fn alive(&self) -> bool {
        birth(self.pid) == Some(self.birth)
    }
    fn signal(&self, signal: i32) -> std::io::Result<()> {
        if !self.alive() {
            return Ok(());
        }
        // SAFETY: the recorded PID and process birth still match this run's owner.
        if unsafe { libc::kill(self.pid as i32, signal) } == 0 {
            Ok(())
        } else {
            Err(std::io::Error::last_os_error())
        }
    }
}
#[cfg(target_os = "macos")]
fn birth(pid: u32) -> Option<(u64, u64)> {
    // SAFETY: proc_pidinfo fills a plain C struct, bounded by its exact size.
    let mut info: libc::proc_bsdinfo = unsafe { std::mem::zeroed() };
    let size = std::mem::size_of_val(&info) as i32;
    let read = unsafe {
        libc::proc_pidinfo(
            pid as i32,
            libc::PROC_PIDTBSDINFO,
            0,
            (&mut info as *mut libc::proc_bsdinfo).cast(),
            size,
        )
    };
    (read == size && info.pbi_status != 5).then_some((info.pbi_start_tvsec, info.pbi_start_tvusec))
}
#[cfg(target_os = "linux")]
fn birth(pid: u32) -> Option<(u64, u64)> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let fields: Vec<_> = stat.rsplit_once(')')?.1.split_whitespace().collect();
    if *fields.first()? == "Z" {
        return None;
    }
    Some((fields.get(19)?.parse().ok()?, 0))
}
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn birth(_pid: u32) -> Option<(u64, u64)> {
    None
}

#[derive(Serialize, Deserialize)]
pub(super) struct Owner<T> {
    pub(super) process: ProcessIdentity,
    run: PathBuf,
    account: Option<PathBuf>,
    pub(super) ready: T,
}
fn private_file(path: &Path, create: bool) -> Result<File> {
    let file = OpenOptions::new()
        .read(true)
        .write(create)
        .create(create)
        .truncate(false)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(path)?;
    let m = file.metadata()?;
    // SAFETY: geteuid has no pointer arguments.
    if !m.is_file()
        || m.uid() != unsafe { libc::geteuid() }
        || m.mode() & 0o077 != 0
        || m.nlink() != 1
    {
        return Err(invalid(
            "run owner record must be an owned private regular file",
        ));
    }
    Ok(file)
}
fn private_run(config: &Config) -> Result<()> {
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(&config.run_dir)?;
    let m = std::fs::symlink_metadata(&config.run_dir)?;
    // SAFETY: geteuid has no pointer arguments.
    if !m.is_dir() || m.uid() != unsafe { libc::geteuid() } || m.mode() & 0o077 != 0 {
        return Err(invalid("run directory must be owned and mode 0700"));
    }
    Ok(())
}
pub(super) fn write<T: Serialize>(config: &Config, name: &str, ready: T) -> Result<()> {
    private_run(config)?;
    let record = Owner {
        process: ProcessIdentity::read(std::process::id())?,
        run: config.run_dir.clone(),
        account: config.test_account.clone(),
        ready,
    };
    let mut temp = tempfile::NamedTempFile::new_in(&config.run_dir)?;
    serde_json::to_writer(temp.as_file_mut(), &record)?;
    temp.as_file_mut().flush()?;
    temp.persist(config.run_dir.join(format!("{name}.owner.json")))
        .map_err(|e| e.error)?;
    Ok(())
}
pub(super) fn read<T: serde::de::DeserializeOwned>(
    config: &Config,
    name: &str,
) -> Result<Option<Owner<T>>> {
    let path = config.run_dir.join(format!("{name}.owner.json"));
    let file = match private_file(&path, false) {
        Ok(f) => f,
        Err(e)
            if e.downcast_ref::<std::io::Error>()
                .is_some_and(|e| e.kind() == std::io::ErrorKind::NotFound) =>
        {
            return Ok(None);
        }
        Err(e) => return Err(e),
    };
    let record: Owner<T> = serde_json::from_reader(file.take(64 * 1024))?;
    if record.run != config.run_dir || record.account != config.test_account {
        return Err(invalid("run owner account/run mismatch"));
    }
    Ok(record.process.alive().then_some(record))
}
pub(super) fn retire(config: &Config, name: &str) -> Result<()> {
    std::fs::remove_file(config.run_dir.join(format!("{name}.owner.json")))?;
    Ok(())
}
pub(super) struct FrontendLock {
    _file: File,
}
impl Drop for FrontendLock {
    fn drop(&mut self) {
        // A concurrent fork can retain this open file description until exec,
        // even with CLOEXEC. Release ownership before closing our descriptor.
        // SAFETY: the borrowed FD is still owned and open; LOCK_UN never waits.
        let _ = unsafe { libc::flock(self._file.as_raw_fd(), libc::LOCK_UN) };
    }
}
impl FrontendLock {
    pub(super) fn claim(config: &Config) -> Result<Option<Self>> {
        private_run(config)?;
        let file = private_file(&config.run_dir.join("app.owner.lock"), true)?;
        // SAFETY: flock borrows an owned FD; LOCK_NB never waits for a peer.
        if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0 {
            return Ok(Some(Self { _file: file }));
        }
        let error = std::io::Error::last_os_error();
        if error.kind() == std::io::ErrorKind::WouldBlock {
            Ok(None)
        } else {
            Err(error.into())
        }
    }
}
pub(super) fn supervisor_held(config: &Config) -> Result<bool> {
    let mut path = config.socket.as_os_str().to_os_string();
    path.push(".lock");
    let file = match private_file(Path::new(&path), false) {
        Ok(f) => f,
        Err(e)
            if e.downcast_ref::<std::io::Error>()
                .is_some_and(|e| e.kind() == std::io::ErrorKind::NotFound) =>
        {
            return Ok(false);
        }
        Err(e) => return Err(e),
    };
    // SAFETY: a nonblocking probe on this run's existing lock; no listener is opened.
    if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0 {
        return Ok(false);
    }
    let error = std::io::Error::last_os_error();
    if error.kind() == std::io::ErrorKind::WouldBlock {
        Ok(true)
    } else {
        Err(error.into())
    }
}

/// A local child has a wait status. An adopted process has only observed liveness.
pub enum Exit {
    Child(std::process::ExitStatus),
    Adopted { orderly: bool },
}
impl std::fmt::Display for Exit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Child(s) => write!(f, "{s}"),
            Self::Adopted { orderly } => write!(
                f,
                "adopted owner departed (orderly={orderly}; wait status unavailable)"
            ),
        }
    }
}
impl Exit {
    pub(super) fn success(&self) -> bool {
        match self {
            Self::Child(s) => s.success(),
            Self::Adopted { orderly } => *orderly,
        }
    }
}
pub enum Process {
    Child(Child),
    Adopted {
        identity: ProcessIdentity,
        record: PathBuf,
    },
}
impl Process {
    pub fn id(&self) -> Option<u32> {
        match self {
            Self::Child(c) => c.id(),
            Self::Adopted { identity, .. } => Some(identity.pid),
        }
    }
    pub fn try_wait(&mut self) -> std::io::Result<Option<Exit>> {
        match self {
            Self::Child(c) => c.try_wait().map(|s| s.map(Exit::Child)),
            Self::Adopted { identity, record } => Ok((!identity.alive()).then(|| Exit::Adopted {
                orderly: !record.exists(),
            })),
        }
    }
    pub async fn wait(&mut self) -> std::io::Result<Exit> {
        if let Self::Child(c) = self {
            return c.wait().await.map(Exit::Child);
        }
        loop {
            if let Some(exit) = self.try_wait()? {
                return Ok(exit);
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
    pub fn start_kill(&mut self) -> std::io::Result<()> {
        match self {
            Self::Child(c) => c.start_kill(),
            Self::Adopted { identity, .. } => identity.signal(libc::SIGKILL),
        }
    }
    pub async fn kill(&mut self) -> std::io::Result<()> {
        self.start_kill()?;
        self.wait().await?;
        Ok(())
    }
    pub(super) fn request_stop(&self) -> std::io::Result<()> {
        if let Self::Adopted { identity, .. } = self {
            identity.signal(libc::SIGTERM)?;
        }
        Ok(())
    }
    pub(super) fn adopted<T>(config: &Config, name: &str, record: Owner<T>) -> Self {
        Self::Adopted {
            identity: record.process,
            record: config.run_dir.join(format!("{name}.owner.json")),
        }
    }
}

pub(super) fn frontend_running(run: &Path) -> Result<Option<bool>> {
    let file = match private_file(&run.join("app.owner.lock"), false) {
        Ok(file) => file,
        Err(error)
            if error
                .downcast_ref::<std::io::Error>()
                .is_some_and(|e| e.kind() == std::io::ErrorKind::NotFound) =>
        {
            return Ok(None);
        }
        Err(error) => return Err(error),
    };
    // SAFETY: a bounded probe of the exact recorded run lock, not a process scan.
    if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0 {
        return Ok(Some(false));
    }
    let error = std::io::Error::last_os_error();
    if error.kind() == std::io::ErrorKind::WouldBlock {
        Ok(Some(true))
    } else {
        Err(error.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    fn config(root: &Path) -> Config {
        std::fs::set_permissions(root, std::fs::Permissions::from_mode(0o700)).unwrap();
        Config {
            executable: std::env::current_exe().unwrap(),
            socket: root.join("mux.sock"),
            run_dir: root.into(),
            repository: root.into(),
            shell_config: None,
            ctl: false,
            test_account: None,
        }
    }
    // keeps: decision #344 rules 1 and 3
    #[test]
    fn frontend_lock_refuses_a_live_owner_and_releases_on_departure() {
        let root = tempfile::tempdir().unwrap();
        let config = config(root.path());
        let lock = FrontendLock::claim(&config).unwrap().unwrap();
        assert_eq!(frontend_running(root.path()).unwrap(), Some(true));
        assert!(FrontendLock::claim(&config).unwrap().is_none());
        drop(lock);
        assert_eq!(frontend_running(root.path()).unwrap(), Some(false));
        assert!(FrontendLock::claim(&config).unwrap().is_some());
    }

    // keeps: decision #344 rule 3; a forked CLOEXEC alias is not a live owner
    #[test]
    fn frontend_lock_release_does_not_wait_for_a_forked_descriptor() {
        use std::net::Shutdown;
        use std::os::unix::net::UnixStream;

        struct ForkedDescriptor {
            pid: libc::pid_t,
            control: UnixStream,
        }
        impl Drop for ForkedDescriptor {
            fn drop(&mut self) {
                // Also runs on assertion failure: EOF releases the child and
                // waitpid reaps only the exact child this test forked.
                let _ = self.control.shutdown(Shutdown::Both);
                loop {
                    // SAFETY: pid is our unreaped child; no status is needed.
                    let result = unsafe { libc::waitpid(self.pid, std::ptr::null_mut(), 0) };
                    if result >= 0
                        || std::io::Error::last_os_error().kind() != std::io::ErrorKind::Interrupted
                    {
                        break;
                    }
                }
            }
        }

        let root = tempfile::tempdir().unwrap();
        let config = config(root.path());
        let lock = FrontendLock::claim(&config).unwrap().unwrap();
        let (parent, child) = UnixStream::pair().unwrap();
        let (parent_fd, child_fd, lock_fd) = (
            parent.as_raw_fd(),
            child.as_raw_fd(),
            lock._file.as_raw_fd(),
        );
        // SAFETY: after fork the child calls only async-signal-safe libc
        // functions, with stack data, then _exit; it never enters Rust cleanup.
        let pid = unsafe { libc::fork() };
        assert!(pid >= 0, "fork: {}", std::io::Error::last_os_error());
        if pid == 0 {
            // SAFETY: inherited descriptors and a one-byte stack buffer; see
            // the fork contract above. The control EOF is the only release.
            unsafe {
                libc::close(parent_fd);
                let flags = libc::fcntl(lock_fd, libc::F_GETFD);
                let ready = u8::from(flags >= 0 && flags & libc::FD_CLOEXEC != 0);
                if libc::write(child_fd, (&ready as *const u8).cast(), 1) != 1 {
                    libc::_exit(1);
                }
                let mut release = 0u8;
                while libc::read(child_fd, (&mut release as *mut u8).cast(), 1) < 0 {}
                libc::_exit(0);
            }
        }
        drop(child);
        let mut child = ForkedDescriptor {
            pid,
            control: parent,
        };
        let mut ready = [0u8];
        child.control.read_exact(&mut ready).unwrap();
        assert_eq!(ready, [1], "child retains the CLOEXEC lock descriptor");
        drop(lock);
        assert_eq!(frontend_running(root.path()).unwrap(), Some(false));
        assert!(FrontendLock::claim(&config).unwrap().is_some());
        drop(child);
    }
    // keeps: decision #344 rule 1, private same-run/account discovery
    #[test]
    fn discovery_refuses_other_accounts_public_records_and_symlinks() {
        let root = tempfile::tempdir().unwrap();
        let mut config = config(root.path());
        write(&config, "logic", 17u32).unwrap();
        assert_eq!(read::<u32>(&config, "logic").unwrap().unwrap().ready, 17);
        config.test_account = Some(root.path().join("other-account"));
        assert!(read::<u32>(&config, "logic").is_err());
        config.test_account = None;
        let path = root.path().join("logic.owner.json");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert!(read::<u32>(&config, "logic").is_err());
        std::fs::remove_file(&path).unwrap();
        std::os::unix::fs::symlink("foreign", &path).unwrap();
        assert!(read::<u32>(&config, "logic").is_err());
    }
    // keeps: kill-only-own-pids; a reused PID cannot name an adopted owner
    #[test]
    fn mismatched_birth_never_receives_a_signal() {
        let mut identity = ProcessIdentity::read(std::process::id()).unwrap();
        assert!(identity.alive());
        identity.birth.1 = identity.birth.1.wrapping_add(1);
        assert!(!identity.alive());
        identity.signal(libc::SIGKILL).unwrap();
    }
}
