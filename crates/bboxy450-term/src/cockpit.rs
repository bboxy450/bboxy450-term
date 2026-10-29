// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! App-owned child processes and the single, admitted GUI connection.
//!
//! Everything in this module runs on bbt-io or in a private non-UI process.
//! Capabilities travel over inherited pipes and private run owner records, never argv or logs.

use std::{
    collections::{BTreeMap, VecDeque},
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};

use bbt_mux::{
    logic::{Logic, LogicConfig, SocketSupervisor},
    seat_socket::read_line,
    store::{Result, invalid},
};
use bbt_proto::{GuiAction, GuiIdentity, GuiReply, GuiRequest, GuiSeat, SeatId, View};
use serde::{Deserialize, Serialize};
use tokio::{
    io::{AsyncWrite, AsyncWriteExt, BufReader},
    net::{UnixStream, unix::OwnedReadHalf, unix::OwnedWriteHalf},
    process::Command,
    sync::mpsc,
};

pub mod desktop;
mod fixture_hosts;
mod owners;
mod shell_path;
use owners::ManagedChild as Child;
pub use owners::{Exit as ProcessExit, Process};

/// A read-only lock observation for ctl's local launcher, not backend discovery.
pub fn frontend_running(run_dir: &std::path::Path) -> Result<Option<bool>> {
    bbt_rt::assert_not_ui_thread();
    owners::frontend_running(run_dir)
}

const STARTUP_TIMEOUT: Duration = Duration::from_secs(30);
const GUI_WRITE_CAPACITY: usize = 64;

/// App-defined, bounded UI-to-IO messages. A full sender returns ownership to
/// the UI for retry; no socket or process operation runs in the window handler.
pub enum UiMsg {
    ConfigureRoles {
        id: bbt_mux::Ulid,
        catalog: bbt_proto::gui::RoleCatalog,
    },
    NewSwarm(bbt_proto::SwarmId),
    SwarmRename {
        id: bbt_mux::Ulid,
        swarm: bbt_proto::SwarmId,
        name: String,
    },
    SwarmWorkdir {
        swarm: bbt_proto::SwarmId,
        path: Option<String>,
    },
    SwarmLaunchDefaults {
        swarm: bbt_proto::SwarmId,
        directory: String,
        environment: Vec<(String, String)>,
    },
    /// Latest durable layout intent from a produced GUI frame.
    WindowState(bbt_proto::gui::WindowState),
    UiReply {
        id: bbt_mux::Ulid,
        result: Box<std::result::Result<bbt_proto::gui::UiLayout, String>>,
    },
    GetAttention {
        id: bbt_mux::Ulid,
        seat: bbt_mux::Ulid,
        generation: u64,
    },
    SetAttention {
        id: bbt_mux::Ulid,
        seat: bbt_mux::Ulid,
        edit: bbt_proto::gui::AttentionEdit,
    },
    PluginControl {
        id: bbt_mux::Ulid,
        request: bbt_proto::gui::PluginRequest,
    },
    KnowledgeControl {
        id: bbt_mux::Ulid,
        request: bbt_proto::gui::KnowledgeRequest,
    },
    IdentityControl {
        id: bbt_mux::Ulid,
        request: bbt_proto::identity::IdentityRequest,
    },
    /// Declarative launch inputs only. IO supplies cwd, swarm and backend role
    /// defaults before using the existing GUI Spawn action.
    Spawn {
        id: bbt_mux::Ulid,
        swarm: Option<bbt_proto::SwarmId>,
        placement: Option<bbt_proto::seat_state::Placement>,
        harness_kind: String,
        name: Option<String>,
        role: Option<String>,
        location: bbt_proto::Where,
    },
    /// An operator's full inputs and placement captured by the GUI owner.
    SpawnSpec {
        id: bbt_mux::Ulid,
        spec: Box<bbt_proto::SpawnSpec>,
    },
    /// Compatibility for the existing verification and default-shell callers.
    SpawnZsh,
    Close(crate::close::Intent),
    UndoClose {
        seat: bbt_proto::SeatId,
    },
    /// UI consumed this retained result; no additional GUI socket RPC.
    ConsumeSpawn {
        id: bbt_mux::Ulid,
    },
    Messaging {
        id: bbt_mux::Ulid,
        swarm_id: bbt_proto::SwarmId,
        request: bbt_proto::messaging::Request,
    },
    Task {
        id: bbt_mux::Ulid,
        scope: Option<bbt_proto::gui::SwarmScope>,
        request: bbt_proto::messaging::TaskRequest,
    },
    SeatControl {
        id: bbt_mux::Ulid,
        seat: bbt_mux::Ulid,
        generation: u64,
        command: bbt_proto::gui::SeatControl,
    },
    Input {
        seat: SeatId,
        generation: u64,
        id: bbt_mux::Ulid,
        bytes: Vec<u8>,
    },
    Resize {
        seat: SeatId,
        generation: u64,
        cols: u16,
        rows: u16,
    },
    /// Local history navigation; IO fetches capture pages without changing Watch.
    Scroll {
        seat: SeatId,
        generation: u64,
        request: crate::scrollback::Request,
    },
    /// Explicit history hydration; transport delivery does not mark it read.
    EventPage {
        stream: String,
        after: u64,
        limit: usize,
    },
    EventCursor {
        stream: String,
    },
    /// View/mark-read intent, separate from a transport acknowledgement.
    MarkEventsRead {
        stream: String,
        sequence: u64,
    },
    /// Local attention handling, not a GUI socket RPC or a read watermark.
    AttentionFocus {
        seat: SeatId,
        generation: u64,
        episode: Option<crate::attention::EpisodeKey>,
        landed: Option<crate::model::FocusSwell>,
    },
    DocumentRead {
        id: bbt_mux::Ulid,
        scope: Option<bbt_proto::gui::SwarmScope>,
        read: crate::documents::Read,
    },
    ConsumeDocument {
        id: bbt_mux::Ulid,
    },
    Watched(Vec<(SeatId, View)>),
    /// The window's answer to a capture asked on the operator's behalf.
    Captured {
        id: bbt_mux::Ulid,
        error: Option<String>,
        frame: bbt_proto::gui::CaptureFrame,
    },
    Focused {
        id: bbt_mux::Ulid,
        result: std::result::Result<bbt_proto::gui::FocusFrame, String>,
    },
    /// The GUI owner's public lifecycle answer; never process credentials.
    AppReply {
        id: bbt_mux::Ulid,
        status: bbt_proto::gui::AppStatus,
    },
    HistoryMetrics {
        id: bbt_mux::Ulid,
        status: bbt_proto::gui::AppStatus,
    },
}
impl UiMsg {
    /// All launches share one admission budget, including legacy callers.
    pub fn is_spawn(&self) -> bool {
        matches!(
            self,
            Self::Spawn { .. } | Self::SpawnSpec { .. } | Self::SpawnZsh
        )
    }
}

/// IO bootstrap settings shared with this app's private child processes.
#[derive(Clone, Serialize, Deserialize)]
pub struct Config {
    pub executable: PathBuf,
    pub socket: PathBuf,
    pub run_dir: PathBuf,
    pub repository: PathBuf,
    /// Optional inherited ZDOTDIR; tests use an empty directory.
    pub shell_config: Option<PathBuf>,
    /// Open the operator's `ctl` door (decision #260): off unless asked for with `--ctl`.
    #[serde(default)]
    pub ctl: bool,
    /// Explicit throwaway root/device custody, persisted across logic restarts.
    #[serde(default)]
    pub test_account: Option<PathBuf>,
}
impl Config {
    /// An explicit repository is never silently replaced with another workspace.
    pub fn repository_from_args(args: &[String]) -> Result<Option<PathBuf>> {
        let mut selected = None;
        for (index, arg) in args.iter().enumerate() {
            let path = if arg == "--repository" {
                args.get(index + 1)
                    .filter(|path| !path.starts_with('-'))
                    .map(String::as_str)
                    .ok_or_else(|| invalid("--repository needs a directory"))?
            } else if let Some(path) = arg.strip_prefix("--repository=") {
                path
            } else {
                continue;
            };
            if selected.is_some() || path.is_empty() {
                return Err(invalid("--repository needs one directory"));
            }
            selected = Some(PathBuf::from(path));
        }
        Ok(selected)
    }

    /// Parse the selector before any backend opens. Invalid input never falls
    /// through to the real account's native custody.
    pub fn test_account_from_args(args: &[String]) -> Result<Option<PathBuf>> {
        let mut selected = None;
        for (index, arg) in args.iter().enumerate() {
            let path = if arg == "--test-account" {
                args.get(index + 1)
                    .map(String::as_str)
                    .ok_or_else(|| invalid("--test-account needs an absolute private directory"))?
            } else if let Some(path) = arg.strip_prefix("--test-account=") {
                path
            } else {
                if arg.starts_with("--test-account")
                    || (arg.starts_with("--unattended") && arg != "--unattended")
                {
                    return Err(invalid(
                        "invalid custody selector; use --test-account <absolute private directory>",
                    ));
                }
                continue;
            };
            let path = PathBuf::from(path);
            if selected.is_some()
                || !path.is_absolute()
                || path
                    .components()
                    .any(|part| matches!(part, std::path::Component::ParentDir))
            {
                return Err(invalid(
                    "--test-account needs one absolute private directory",
                ));
            }
            selected = Some(path);
        }
        if args.iter().any(|arg| arg == "--unattended") && selected.is_none() {
            return Err(invalid(
                "unattended requires --test-account; no production automation identity is provisioned",
            ));
        }
        Ok(selected)
    }
    /// Resolve the executable, repository and local control endpoint on IO.
    pub fn discover(run_dir: Option<PathBuf>) -> Result<Self> {
        bbt_rt::assert_not_ui_thread();
        let args: Vec<_> = std::env::args().skip(1).collect();
        let test_account = Self::test_account_from_args(&args)?;
        let selected = Self::repository_from_args(&args)?;
        let home = std::env::var_os("HOME").map(PathBuf::from);
        let repository = resolve_repository(
            selected.as_deref(),
            std::env::current_dir(),
            home.as_deref(),
        )?;
        let run_dir = run_dir.or_else(|| test_account.as_ref().map(|root| root.join("run")));
        let socket = match run_dir.as_ref() {
            Some(root) => root.join("mux.sock"),
            None => bbt_mux::transport::default_socket_path()?,
        };
        let run_dir = socket
            .parent()
            .ok_or_else(|| invalid("supervisor socket has no parent"))?
            .into();
        Ok(Self {
            executable: std::env::current_exe()?,
            socket,
            run_dir,
            repository,
            shell_config: std::env::var_os("ZDOTDIR").map(PathBuf::from),
            ctl: false,
            test_account,
        })
    }
}

fn checked_repository(path: &Path) -> Result<PathBuf> {
    let check = || -> std::io::Result<PathBuf> {
        let path = path.canonicalize()?;
        if path.parent().is_none() || !path.is_dir() {
            return Err(std::io::Error::other(
                "choose a directory other than the filesystem root",
            ));
        }
        // Probe real filesystem permissions (including ACLs), without leaving a
        // file behind. An existing read-only .bboxy is unusable even if cwd is writable.
        let _probe = tempfile::Builder::new()
            .prefix(".bboxy-write-")
            .tempfile_in(&path)?;
        let records = path.join(".bboxy");
        if records.try_exists()? {
            if !records.is_dir() {
                return Err(std::io::Error::other(".bboxy is not a directory"));
            }
            let _probe = tempfile::Builder::new()
                .prefix(".bboxy-write-")
                .tempfile_in(records)?;
        }
        Ok(path)
    };
    check().map_err(|error| invalid(format!("repository_unavailable: {}: {error}; choose a writable directory with --repository PATH", path.display())))
}

fn resolve_repository(
    selected: Option<&Path>,
    cwd: std::io::Result<PathBuf>,
    home: Option<&Path>,
) -> Result<PathBuf> {
    if let Some(path) = selected {
        return checked_repository(path);
    }
    if let Ok(cwd) = cwd
        && let Ok(repository) = checked_repository(&cwd)
    {
        return Ok(repository);
    }
    let home = home.ok_or_else(|| invalid("repository_unavailable: HOME is unavailable for the default workspace; use --repository PATH"))?;
    let path = home.join("Library/Application Support/bboxy450-term/workspace");
    use std::os::unix::fs::DirBuilderExt;
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(&path)
        .map_err(|error| {
            invalid(format!(
                "repository_unavailable: {}: {error}; use --repository PATH",
                path.display()
            ))
        })?;
    checked_repository(&path)
}

#[cfg(test)]
#[allow(
    clippy::disallowed_methods,
    reason = "private filesystem-permission fixtures run outside the UI owner"
)]
mod repository_tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    // keeps: release #54 (explicit repository parsing never silently falls back).
    #[test]
    fn repository_arguments_accept_both_forms_and_reject_missing_or_duplicate_paths() {
        for args in [
            vec!["--repository", "somewhere"],
            vec!["--repository=somewhere"],
        ] {
            let args = args.into_iter().map(str::to_owned).collect::<Vec<_>>();
            assert_eq!(
                Config::repository_from_args(&args).unwrap(),
                Some("somewhere".into())
            );
        }
        for args in [
            vec!["--repository"],
            vec!["--repository", "--headless"],
            vec!["--repository="],
            vec!["--repository=a", "--repository=b"],
        ] {
            let args = args.into_iter().map(str::to_owned).collect::<Vec<_>>();
            assert!(Config::repository_from_args(&args).is_err());
        }
    }

    // keeps: release #54 (Finder cwd and unavailable cwd use a private writable workspace).
    #[test]
    fn finder_and_unavailable_working_directories_use_the_app_workspace() {
        let home = tempfile::tempdir().unwrap();
        let expected = home
            .path()
            .join("Library/Application Support/bboxy450-term/workspace");
        for cwd in [
            Ok(PathBuf::from("/")),
            Err(std::io::Error::from(std::io::ErrorKind::NotFound)),
        ] {
            let selected = resolve_repository(None, cwd, Some(home.path())).unwrap();
            assert_eq!(selected, expected.canonicalize().unwrap());
            assert!(selected.is_dir());
        }
        let cwd = tempfile::tempdir().unwrap();
        assert_eq!(
            resolve_repository(None, Ok(cwd.path().into()), Some(home.path())).unwrap(),
            cwd.path().canonicalize().unwrap()
        );
    }

    // keeps: release #54 (implicit read-only roots fall back; explicit roots are refused).
    #[test]
    fn readonly_repository_or_existing_records_are_never_passed_to_logic() {
        let home = tempfile::tempdir().unwrap();
        let root = tempfile::tempdir().unwrap();
        let records = root.path().join(".bboxy");
        std::fs::create_dir(&records).unwrap();
        for readonly in [root.path(), records.as_path()] {
            std::fs::set_permissions(readonly, std::fs::Permissions::from_mode(0o500)).unwrap();
            let implicit = resolve_repository(None, Ok(root.path().into()), Some(home.path()));
            let explicit =
                resolve_repository(Some(root.path()), Ok(home.path().into()), Some(home.path()));
            std::fs::set_permissions(readonly, std::fs::Permissions::from_mode(0o700)).unwrap();
            assert_eq!(
                implicit.unwrap(),
                home.path()
                    .join("Library/Application Support/bboxy450-term/workspace")
                    .canonicalize()
                    .unwrap()
            );
            assert!(
                explicit
                    .unwrap_err()
                    .to_string()
                    .starts_with("repository_unavailable:")
            );
        }
        assert_eq!(
            resolve_repository(Some(root.path()), Ok(PathBuf::from("/")), Some(home.path()))
                .unwrap(),
            root.path().canonicalize().unwrap()
        );
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct SupervisorReady {
    socket: PathBuf,
    capability: [u8; 32],
    /// Every seat descends from this process; `ctl` refuses its descendants (decision #260).
    #[serde(default)]
    pid: u32,
}
#[derive(Serialize, Deserialize)]
struct LogicBootstrap {
    config: Config,
    supervisor: SupervisorReady,
    #[serde(default)]
    recover_seats: bool,
    /// The idle smoke has finite local inputs, without timed remote-link faults.
    #[serde(default = "fixture_hosts_default")]
    fixture_hosts: bool,
}
fn fixture_hosts_default() -> bool {
    true
}
// MCP control schema/grants; logical UI input is versioned independently.
const MCP_CONTROL_VERSION: u16 = 7;
/// The one-line private startup reply. Never log its capability.
#[derive(Clone, Serialize, Deserialize)]
pub struct LogicReady {
    /// Refresh an older control service on UI reattach without replacing PTYs.
    #[serde(default)]
    pub mcp_control_version: u16,
    #[serde(default)]
    pub gui_state_version: u16,
    /// New metadata verbs need a fresh logic owner even with the same crate version.
    #[serde(default)]
    pub swarm_metadata_version: u16,
    #[serde(default)]
    pub ui_control_version: u16,

    /// Scoped records/tasks need a root-aware owner even on the same layout wire.
    #[serde(default)]
    pub swarm_workspace_version: u16,
    /// Private environment compatibility hint; never exports the environment.
    #[serde(default)]
    pub launch_path_fingerprint: Option<String>,
    pub root: PathBuf,
    pub capability: [u8; 32],
    pub identity: GuiIdentity,
    #[serde(default)]
    pub generation: u64,
    #[serde(default = "unknown_custody")]
    pub custody_backend: String,
    #[serde(default)]
    pub account_root: Option<String>,
    #[serde(skip)]
    pub window: Option<bbt_proto::gui::WindowSnapshot>,
}
fn unknown_custody() -> String {
    "unknown".into()
}

fn json_frame(value: &impl Serialize) -> Result<Vec<u8>> {
    let mut bytes = serde_json::to_vec(value)?;
    if bytes.len() >= 1024 * 1024 {
        return Err(invalid("GUI request exceeds 1 MiB framing bound"));
    }
    bytes.push(b'\n');
    Ok(bytes)
}
async fn write_frame(writer: &mut (impl AsyncWrite + Unpin), bytes: &[u8]) -> Result<()> {
    tokio::time::timeout(Duration::from_secs(2), writer.write_all(bytes)).await??;
    writer.flush().await?;
    Ok(())
}
async fn write_json(writer: &mut (impl AsyncWrite + Unpin), value: &impl Serialize) -> Result<()> {
    write_frame(writer, &json_frame(value)?).await
}

/// The bounded GUI writer is full. The caller retains the original action and
/// retries it while continuing to drain replies; no socket write was attempted.
#[derive(Debug)]
pub struct Busy;
impl std::fmt::Display for Busy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Busy: GUI writer queue is full")
    }
}
impl std::error::Error for Busy {}

// Both directions must progress independently. A peer may write a burst before
// reading requests; awaiting a write on the reply-draining task deadlocks it.
struct GuiIo {
    capability: [u8; 32],
    outgoing: mpsc::Sender<Vec<u8>>,
    incoming: mpsc::Receiver<Result<GuiReply>>,
    reader_task: tokio::task::JoinHandle<()>,
    writer_task: tokio::task::JoinHandle<Result<()>>,
    disconnected: bool,
}
impl GuiIo {
    fn new(stream: UnixStream, capability: [u8; 32]) -> Self {
        let (read, writer) = stream.into_split();
        let (sender, incoming) = mpsc::channel(8);
        let reader_task = tokio::spawn(read_replies(read, sender));
        let (outgoing, requests) = mpsc::channel(GUI_WRITE_CAPACITY);
        let writer_task = tokio::spawn(write_requests(writer, requests));
        Self {
            capability,
            outgoing,
            incoming,
            reader_task,
            writer_task,
            disconnected: false,
        }
    }
    fn try_send(&self, action: GuiAction) -> Result<()> {
        if self.disconnected {
            return Err(invalid("GUI connection closed"));
        }
        let bytes = json_frame(&GuiRequest {
            version: bbt_proto::gui::VERSION,
            capability: self.capability,
            action,
        })?;
        match self.outgoing.try_send(bytes) {
            Ok(()) => Ok(()),
            Err(mpsc::error::TrySendError::Full(_)) => Err(Box::new(Busy)),
            Err(mpsc::error::TrySendError::Closed(_)) => Err(invalid("GUI writer stopped")),
        }
    }
    fn writer_ready(&self) -> impl std::future::Future<Output = Result<()>> + Send + use<> {
        let outgoing = self.outgoing.clone();
        async move {
            drop(
                outgoing
                    .reserve_owned()
                    .await
                    .map_err(|_| invalid("GUI writer stopped"))?,
            );
            Ok(())
        }
    }
    async fn event(&mut self) -> Event {
        tokio::select! {
            reply=self.incoming.recv(), if !self.disconnected => {
                match reply {
                    Some(Ok(reply)) => Event::Reply(Box::new(reply)),
                    other => {
                        self.disconnected = true;
                        self.writer_task.abort();
                        Event::Disconnected(match other {
                            Some(Err(error)) => error.to_string(),
                            _ => "GUI connection closed".into(),
                        })
                    }
                }
            }
            result=&mut self.writer_task, if !self.disconnected => {
                self.disconnected = true;
                self.reader_task.abort();
                Event::Disconnected(match result {
                    Ok(Err(error)) => format!("GUI writer stopped: {error}"),
                    Err(error) => format!("GUI writer task stopped: {error}"),
                    Ok(Ok(())) => "GUI writer stopped".into(),
                })
            }
            else => std::future::pending().await,
        }
    }
}
impl Drop for GuiIo {
    fn drop(&mut self) {
        self.reader_task.abort();
        self.writer_task.abort();
    }
}
async fn write_requests(
    mut writer: OwnedWriteHalf,
    mut requests: mpsc::Receiver<Vec<u8>>,
) -> Result<()> {
    while let Some(bytes) = requests.recv().await {
        write_frame(&mut writer, &bytes).await?;
    }
    Ok(())
}

/// Dispatch before creating winit or registering a UI thread. Each private
/// process initializes its own process-wide bbt-rt runtime.
#[allow(
    clippy::disallowed_methods,
    reason = "private non-UI process entry point"
)]
pub fn private_entry(args: &[String]) -> Option<Result<()>> {
    match args.first().map(String::as_str) {
        Some("desktop") => Some(desktop::run()),
        Some("supervisor") => Some(bbt_rt::handle().block_on(supervisor_main())),
        Some("logic") => Some(bbt_rt::handle().block_on(logic_main())),
        _ => None,
    }
}

// EOF is not an instruction to kill seats after a GUI crash. An explicit stop
// is used on orderly app shutdown; Session owns child restart/reconnection.
async fn wait_for_stop(reader: &mut BufReader<tokio::io::Stdin>) -> Result<()> {
    loop {
        match read_line(reader).await? {
            Some(line) if line == b"stop" => return Ok(()),
            Some(_) => return Err(invalid("unknown private child control")),
            None => std::future::pending().await,
        }
    }
}

async fn supervisor_main() -> Result<()> {
    let mut input = BufReader::new(tokio::io::stdin());
    let line = read_line(&mut input)
        .await?
        .ok_or_else(|| invalid("missing supervisor bootstrap"))?;
    let config: Config = serde_json::from_slice(&line)?;
    bbt_rt::error_journal::configure(&config.run_dir);
    let supervisor = bbt_mux::Supervisor::start(bbt_mux::Config {
        socket_path: Some(config.socket.clone()),
        input_probes: vec![b"\x1b]7501;?\x07".to_vec(), b"\x1b]7501;?\x1b\\".to_vec()],
        journal_directory: Some({
            use std::os::unix::fs::DirBuilderExt;
            // Prepare the private socket parent before object_dir can create
            // it with the default mode. Existing parents are still validated
            // by the supervisor's socket admission check.
            std::fs::DirBuilder::new()
                .recursive(true)
                .mode(0o700)
                .create(&config.run_dir)?;
            bbt_mux::store::object_dir(&config.run_dir, "supervisor")?.join("journal")
        }),
        ..Default::default()
    })?;
    let ready = SupervisorReady {
        socket: config.socket.clone(),
        capability: supervisor.socket_capability(),
        pid: std::process::id(),
    };
    owners::write(&config, "supervisor", &ready)?;
    write_json(&mut tokio::io::stdout(), &ready).await?;
    use tokio::signal::unix::{SignalKind, signal};
    let mut terminate = signal(SignalKind::terminate())?;
    tokio::select! {
        stopped = wait_for_stop(&mut input) => stopped?,
        _ = terminate.recv() => eprintln!("supervisor: SIGTERM"),
    }
    eprintln!("supervisor: orderly shutdown requested");
    supervisor.shutdown()?;
    owners::retire(&config, "supervisor")?;
    eprintln!("supervisor: shutdown completed");
    Ok(())
}

async fn logic_main() -> Result<()> {
    use tokio::signal::unix::{SignalKind, signal};
    let mut input = BufReader::new(tokio::io::stdin());
    let line = read_line(&mut input)
        .await?
        .ok_or_else(|| invalid("missing logic bootstrap"))?;
    let bootstrap: LogicBootstrap = serde_json::from_slice(&line)?;
    let config = bootstrap.config.clone();
    bbt_rt::error_journal::configure(&config.run_dir);
    let connection = SocketSupervisor::connect(
        &bootstrap.supervisor.socket,
        bootstrap.supervisor.capability,
    )?;
    let run_dir = bootstrap.config.run_dir.clone();
    let store = bootstrap
        .config
        .test_account
        .as_ref()
        .map(|directory| {
            bbt_mux::identity::TestAccount::open(directory, &bootstrap.config.run_dir)
                .map(|account| account.key_store())
        })
        .transpose()?;
    let logic_config = LogicConfig {
        run_dir: bootstrap.config.run_dir,
        repository: bootstrap.config.repository,
        ring: Default::default(),
    };
    let logic = if bootstrap.fixture_hosts {
        Logic::start_custodied_with_fixture_store(
            connection,
            logic_config,
            store,
            bbt_rt::handle(),
        )?
    } else if let Some(store) = store {
        Logic::start_custodied_with_store(connection, logic_config, store, bbt_rt::handle())?
    } else {
        Logic::start_custodied(connection, logic_config, bbt_rt::handle())?
    };
    if bootstrap.recover_seats {
        logic.client.recover_seats().await?;
    }
    // The operator's ctl door, only when asked for (decision #260, invariant #261): the
    // window is logic's parent, and the supervisor names whose descendants are seats.
    let door = if bootstrap.config.ctl {
        let peers = bbt_mux::ctl_door::Peers {
            supervisor: bootstrap.supervisor.pid,
            gui: std::os::unix::process::parent_id(),
        };
        Some(bbt_mux::ctl_door::open(&logic.client, &run_dir, peers).await?)
    } else {
        None
    };
    let endpoint = logic.client.gui_endpoint().await?;
    let status = logic.client.identity_status().await?;
    let ready = LogicReady {
        mcp_control_version: MCP_CONTROL_VERSION,
        gui_state_version: bbt_proto::gui::WINDOW_STATE_VERSION,
        swarm_metadata_version: bbt_proto::gui::SWARM_METADATA_VERSION,
        ui_control_version: bbt_proto::gui::UI_CONTROL_VERSION,

        swarm_workspace_version: bbt_proto::gui::SWARM_WORKSPACE_VERSION,
        launch_path_fingerprint: std::env::var_os("PATH")
            .as_deref()
            .map(shell_path::fingerprint),
        root: endpoint
            .path
            .parent()
            .ok_or_else(|| invalid("GUI endpoint has no root"))?
            .into(),
        capability: endpoint.capability,
        identity: logic.client.gui_identity().await?,
        generation: endpoint.generation,
        custody_backend: serde_json::to_value(status.custody)?
            .as_str()
            .unwrap_or("unknown")
            .into(),
        account_root: status.root_fingerprint,
        window: None,
    };
    owners::write(&config, "logic", &ready)?;
    write_json(&mut tokio::io::stdout(), &ready).await?;
    // SIGTERM and SIGINT stop logic in order too, so the ctl door closes and its
    // gui.endpoint is removed (invariant #261), as on the window's stop.
    let mut terminate = signal(SignalKind::terminate())?;
    let mut interrupt = signal(SignalKind::interrupt())?;
    tokio::select! {
        stopped = wait_for_stop(&mut input) => stopped?,
        _ = terminate.recv() => eprintln!("logic: SIGTERM"),
        _ = interrupt.recv() => eprintln!("logic: SIGINT"),
    }
    eprintln!("logic: orderly shutdown requested");
    drop(door);
    logic.shutdown()?;
    owners::retire(&config, "logic")?;
    eprintln!("logic: shutdown completed");
    Ok(())
}

/// The IO owner, never stored on or dropped from the UI thread.
pub struct Session {
    pub supervisor: Process,
    pub desktop_supervisor: std::sync::Arc<std::sync::atomic::AtomicU32>,
    pub logic: Process,
    _frontend: owners::FrontendLock,
    pub endpoint: LogicReady,
    config: Config,
    launch_path: std::ffi::OsString,
    supervisor_ready: SupervisorReady,
    // Child::wait closes Child.stdin on its first poll, even when cancelled.
    supervisor_control: Option<tokio::process::ChildStdin>,
    logic_control: Option<tokio::process::ChildStdin>,
    io: GuiIo,
    watches: BTreeMap<SeatId, View>,
    restoring: VecDeque<GuiAction>,
    deferred: VecDeque<Event>,
    // Startup belongs to this task, never to a cancellable event() future.
    recovery: Option<tokio::task::JoinHandle<Result<Recovered>>>,
    recovery_failed: bool,
    supervisor_exited: bool,
    logic_exited: bool,
    disconnected: bool,
}
struct StartedSupervisor {
    child: Child,
    control: Option<tokio::process::ChildStdin>,
    ready: SupervisorReady,
}
struct StartedLogic {
    child: Child,
    control: Option<tokio::process::ChildStdin>,
    endpoint: LogicReady,
    io: GuiIo,
}
struct Recovered {
    supervisor: Option<StartedSupervisor>,
    logic: StartedLogic,
    seats: Vec<GuiSeat>,
    watches: BTreeMap<SeatId, View>,
}
/// Socket and process notifications consumed by the app's IO owner.
pub enum Event {
    Reply(Box<GuiReply>),
    Exited {
        process: &'static str,
        status: ProcessExit,
    },
    Disconnected(String),
}
impl Drop for Session {
    fn drop(&mut self) {
        if let Some(task) = &self.recovery {
            task.abort();
        }
    }
}
impl Session {
    /// Spawn both private processes and connect their admitted GUI endpoint.
    pub async fn start(mut config: Config) -> Result<Self> {
        bbt_rt::assert_not_ui_thread();
        // Refuse before any custody or child process opens, including direct callers.
        config.repository = checked_repository(&config.repository)?;
        if let Some(directory) = &config.test_account {
            bbt_mux::identity::TestAccount::prepare_directory(directory)?;
        }
        let frontend = match owners::FrontendLock::claim(&config)? {
            Some(lock) => lock,
            None => {
                if let Some(record) = owners::read::<LogicReady>(&config, "logic")? {
                    let mut endpoint = record.ready.clone();
                    if let Ok(mut io) = attach_gui(&config, &mut endpoint, false).await {
                        let id = bbt_mux::Ulid::new();
                        io.try_send(GuiAction::App {
                            id,
                            request: bbt_proto::gui::AppRequest::Activate,
                        })?;
                        let _ = tokio::time::timeout(Duration::from_secs(2), async {
                            loop {if matches!(io.event().await,Event::Reply(reply) if matches!(*reply,GuiReply::App {op,..} if op==id)) {break;}}
                        }).await;
                    }
                }
                return Err(invalid(
                    "app_already_running: another GUI owns this run; activation attempted",
                ));
            }
        };
        let launch_path = shell_path::load(&config).await;
        let (mut supervisor, supervisor_control, supervisor_ready, attached) =
            if owners::supervisor_held(&config)? {
                let record = owners::read::<SupervisorReady>(&config, "supervisor")?
                .ok_or_else(|| invalid("live supervisor has no matching private run owner record; no second supervisor started"))?;
                let mut ready = record.ready.clone();
                ready.pid = record.process.pid();
                (
                    Process::adopted(&config, "supervisor", record),
                    None,
                    ready,
                    true,
                )
            } else {
                let owner = start_supervisor(&config, &launch_path).await?;
                (
                    Process::Child(owner.child),
                    owner.control,
                    owner.ready,
                    false,
                )
            };
        let mut existing = owners::read::<LogicReady>(&config, "logic")?.filter(|_| attached);
        if existing.as_ref().is_some_and(|record| {
            record.ready.mcp_control_version != MCP_CONTROL_VERSION
                || record.ready.gui_state_version != bbt_proto::gui::WINDOW_STATE_VERSION
                || record.ready.swarm_metadata_version != bbt_proto::gui::SWARM_METADATA_VERSION
                || record.ready.ui_control_version != bbt_proto::gui::UI_CONTROL_VERSION
                || record.ready.swarm_workspace_version != bbt_proto::gui::SWARM_WORKSPACE_VERSION
                || record.ready.launch_path_fingerprint.as_deref()
                    != Some(shell_path::fingerprint(&launch_path).as_str())
        }) {
            let record = existing.take().unwrap();
            let mut old_logic = Process::adopted(&config, "logic", record);
            // Refresh logic only: the supervisor retains every live PTY and process.
            stop_logic_for_refresh(&mut old_logic, &mut None).await?;
            eprintln!(
                "control service, launch environment or GUI schema changed; refreshed logic, retained supervisor and seats"
            );
        }
        let (mut logic, logic_control, endpoint, io) = if let Some(record) = existing {
            let mut endpoint = record.ready.clone();
            let mut process = Process::adopted(&config, "logic", record);
            match attach_gui(&config, &mut endpoint, true).await {
                Ok(io) => (process, None, endpoint, io),
                Err(error) => {
                    eprintln!(
                        "logic endpoint unavailable: {error}; refreshing logic, retaining supervisor and seats"
                    );
                    stop_logic_for_refresh(&mut process, &mut None).await?;
                    let owner =
                        start_logic(&config, &supervisor_ready, false, &launch_path).await?;
                    (
                        Process::Child(owner.child),
                        owner.control,
                        owner.endpoint,
                        owner.io,
                    )
                }
            }
        } else {
            let owner = start_logic(&config, &supervisor_ready, false, &launch_path).await?;
            (
                Process::Child(owner.child),
                owner.control,
                owner.endpoint,
                owner.io,
            )
        };
        supervisor.preserve_on_drop();
        logic.preserve_on_drop();
        Ok(Self {
            config,
            launch_path,
            _frontend: frontend,
            desktop_supervisor: std::sync::Arc::new(std::sync::atomic::AtomicU32::new(
                supervisor_ready.pid,
            )),
            supervisor_ready,
            supervisor,
            supervisor_control,
            logic,
            logic_control,
            endpoint,
            io,
            watches: BTreeMap::new(),
            restoring: VecDeque::new(),
            deferred: VecDeque::new(),
            recovery: None,
            recovery_failed: false,
            supervisor_exited: false,
            logic_exited: false,
            disconnected: false,
        })
    }

    /// Non-blocking enqueue; Busy retains the caller's action and input ID.
    /// Recovery never writes on the task which drains incoming replies.
    pub async fn send(&mut self, action: GuiAction) -> Result<()> {
        if self.disconnected || self.recovery.is_some() {
            return Err(invalid("backend reconnecting; retain request"));
        }
        if self.flush_restoring()? {
            return Err(Box::new(Busy));
        }
        let result = self.io.try_send(action.clone());
        if result.is_ok() {
            match action {
                GuiAction::Watch { seat, view } => {
                    self.watches.insert(seat, view);
                }
                GuiAction::Unwatch { seat } => {
                    self.watches.remove(&seat);
                }
                _ => {}
            }
        } else if self.io.disconnected || self.io.outgoing.is_closed() {
            self.disconnect()?;
        }
        result
    }
    /// Owned capacity notification; event() and close remain polled alongside it.
    pub fn writer_ready(&self) -> impl std::future::Future<Output = Result<()>> + Send + use<> {
        self.io.writer_ready()
    }
    // One finite restoration diff, bounded by the last watched pane set. Drain
    // it through the existing bounded writer before admitting later UI actions.
    fn flush_restoring(&mut self) -> Result<bool> {
        while let Some(action) = self.restoring.front() {
            match self.io.try_send(action.clone()) {
                Ok(()) => {
                    self.restoring.pop_front();
                }
                Err(error) if error.is::<Busy>() => return Ok(true),
                Err(error) => {
                    self.disconnect()?;
                    return Err(error);
                }
            }
        }
        Ok(false)
    }
    pub async fn receive(&mut self) -> Result<GuiReply> {
        loop {
            match self.event().await? {
                Event::Reply(reply) => return Ok(*reply),
                Event::Disconnected(reason) if self.recovery_failed => return Err(invalid(reason)),
                _ => {}
            }
        }
    }
    /// Cancellation leaves the stored recovery task and its handshake alive.
    pub async fn event(&mut self) -> Result<Event> {
        if let Some(event) = self.deferred.pop_front() {
            return Ok(event);
        }
        self.next_event().await
    }
    fn disconnect(&mut self) -> Result<()> {
        self.disconnected = true;
        self.io.disconnected = true;
        self.io.reader_task.abort();
        self.io.writer_task.abort();
        if !self.logic_exited {
            self.logic.start_kill()?;
        }
        Ok(())
    }
    fn restart_if_ready(&mut self) {
        if !self.logic_exited || self.recovery.is_some() || self.recovery_failed {
            return;
        }
        let config = self.config.clone();
        let launch_path = self.launch_path.clone();
        let ready = self.supervisor_ready.clone();
        let replace_supervisor = self.supervisor_exited;
        let watches = self.watches.clone();
        self.recovery = Some(tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            tokio::time::timeout(
                STARTUP_TIMEOUT,
                recover(config, ready, replace_supervisor, watches, launch_path),
            )
            .await
            .map_err(|_| invalid("recovery startup timed out"))?
        }));
    }
    async fn next_event(&mut self) -> Result<Event> {
        loop {
            self.restart_if_ready();
            let writer_blocked = if self.disconnected {
                false
            } else {
                self.flush_restoring()?
            };
            let writer_ready = self.io.writer_ready();
            return tokio::select! {
                biased;
                status=self.supervisor.wait(), if !self.supervisor_exited => {
                    self.supervisor_exited = true;
                    if let Some(task) = self.recovery.take() { task.abort(); }
                    self.disconnect()?;
                    self.restart_if_ready();
                    Ok(Event::Exited {process:"supervisor",status:status?})
                }
                status=self.logic.wait(), if !self.logic_exited => {
                    self.logic_exited = true;
                    self.disconnected = true;
                    self.io.disconnected = true;
                    self.io.reader_task.abort();
                    self.io.writer_task.abort();
                    self.restart_if_ready();
                    Ok(Event::Exited {process:"logic",status:status?})
                }
                result=async { self.recovery.as_mut().unwrap().await }, if self.recovery.is_some() => {
                    self.recovery = None;
                    match result.map_err(|error| invalid(format!("recovery task stopped: {error}")))? {
                        Ok(recovered) => {
                            if let Some(supervisor) = recovered.supervisor {
                                self.supervisor = Process::Child(supervisor.child);
                                self.supervisor.preserve_on_drop();
                                self.supervisor_control = supervisor.control;
                                self.supervisor_ready = supervisor.ready;
                                self.desktop_supervisor.store(self.supervisor_ready.pid, std::sync::atomic::Ordering::Release);
                                desktop::refresh(self.config.clone());
                            }
                            let logic = recovered.logic;
                            self.logic = Process::Child(logic.child);
                            self.logic.preserve_on_drop();
                            self.logic_control = logic.control;
                            self.endpoint = logic.endpoint;
                            self.io = logic.io;
                            self.watches = recovered.watches;
                            self.restoring = self.watches.iter().map(|(seat,view)| GuiAction::Watch {seat:*seat,view:*view}).collect();
                            self.supervisor_exited = false;
                            self.logic_exited = false;
                            self.disconnected = false;
                            self.deferred.push_back(Event::Reply(Box::new(GuiReply::Identity(self.endpoint.identity.clone()))));
                            Ok(Event::Reply(Box::new(GuiReply::Seats(recovered.seats))))
                        }
                        Err(error) => {
                            self.recovery_failed = true;
                            Ok(Event::Disconnected(format!("recovery failed: {error}")))
                        }
                    }
                }
                ready=writer_ready, if writer_blocked && !self.disconnected => { ready?; continue; }
                event=self.io.event(), if !self.disconnected => {
                    if matches!(event,Event::Disconnected(_)) { self.disconnect()?; }
                    Ok(event)
                }
                else => std::future::pending().await,
            };
        }
    }
    /// Obtain the catalogue before any views are watched.
    pub async fn list(&mut self) -> Result<Vec<GuiSeat>> {
        self.send(GuiAction::ListSeats).await?;
        let mut events = VecDeque::new();
        let mut bytes = 0;
        loop {
            match self.receive().await? {
                GuiReply::Seats(seats) => {
                    self.deferred.extend(events);
                    return Ok(seats);
                }
                reply @ (GuiReply::Event { .. }
                | GuiReply::Gap { .. }
                | GuiReply::Events { .. }
                | GuiReply::EventCursor { .. }
                | GuiReply::EventStatus { .. }
                | GuiReply::UiAsk { .. }
                | GuiReply::AppAsk { .. }
                | GuiReply::AppStopping { .. }) => {
                    bytes += serde_json::to_vec(&reply)?.len();
                    if events.len() == 128 || bytes > 256 * 1024 {
                        return Err(invalid(
                            "startup event staging bound reached; events remain unacknowledged",
                        ));
                    }
                    events.push_back(Event::Reply(Box::new(reply)));
                }
                GuiReply::Identity(identity) => self.endpoint.identity = identity,
                GuiReply::Error { reason, .. } => return Err(invalid(reason)),
                GuiReply::WindowSaved { .. }
                | GuiReply::Swarm { .. }
                | GuiReply::Swarms { .. }
                | GuiReply::GuiHello { .. }
                | GuiReply::Seat(_)
                | GuiReply::Spawned { .. }
                | GuiReply::Update { .. }
                | GuiReply::Ok { .. }
                | GuiReply::CaptureAsk { .. }
                | GuiReply::OperatorSpawned { .. }
                | GuiReply::Screen { .. }
                | GuiReply::EventsRead { .. }
                | GuiReply::OperatorClose { .. }
                | GuiReply::OperatorUndoClose { .. }
                | GuiReply::EventsStreams { .. }
                | GuiReply::Plugin { .. }
                | GuiReply::Debug { .. }
                | GuiReply::OperatorFocus { .. }
                | GuiReply::Focused { .. }
                | GuiReply::Hosts(_)
                | GuiReply::HostState { .. } => {}
                _ => return Err(invalid("unexpected GUI startup reply")),
            }
        }
    }
    /// Save the last window frame and release only the GUI attachment.
    /// Backend owners and their PTYs remain available to the next UI.
    pub async fn detach(mut self, state: Option<bbt_proto::gui::WindowState>) -> Result<()> {
        self.supervisor.preserve_on_drop();
        self.logic.preserve_on_drop();
        let result = tokio::time::timeout(STARTUP_TIMEOUT, async {
            while self.recovery.is_some() {
                self.event().await?;
                self.supervisor.preserve_on_drop();
                self.logic.preserve_on_drop();
            }
            if let Some(state) = state {
                let epoch = self
                    .endpoint
                    .window
                    .as_ref()
                    .ok_or_else(|| invalid("GUI detach has no window epoch"))?
                    .epoch;
                let id = bbt_mux::Ulid::new();
                let action = GuiAction::WindowSave { id, epoch, state };
                loop {
                    match self.send(action.clone()).await {
                        Ok(()) => break,
                        Err(error) if error.is::<Busy>() => self.writer_ready().await?,
                        Err(error) => return Err(error),
                    }
                }
                loop {
                    if let GuiReply::WindowSaved { op, error } = self.receive().await?
                        && op == id
                    {
                        if let Some(error) = error {
                            return Err(invalid(error));
                        }
                        break;
                    }
                }
            }
            Ok(())
        })
        .await
        .map_err(|_| invalid("GUI detach state-save timed out"))?;
        self.supervisor.preserve_on_drop();
        self.logic.preserve_on_drop();
        result
    }

    /// Explicit hard quit: stop and reap both backend owners and their seats.
    pub async fn shutdown(mut self) -> Result<()> {
        if let Some(task) = self.recovery.take() {
            task.abort();
            if let Ok(Ok(mut recovered)) = task.await {
                let _ = stop_child(
                    &mut Process::Child(recovered.logic.child),
                    &mut recovered.logic.control,
                    "logic",
                )
                .await;
                if let Some(mut supervisor) = recovered.supervisor {
                    let _ = stop_child(
                        &mut Process::Child(supervisor.child),
                        &mut supervisor.control,
                        "supervisor",
                    )
                    .await;
                }
            }
        }
        let logic = stop_child(&mut self.logic, &mut self.logic_control, "logic").await;
        let supervisor = stop_child(
            &mut self.supervisor,
            &mut self.supervisor_control,
            "supervisor",
        )
        .await;
        logic.and(supervisor)
    }
}

async fn attach_gui(config: &Config, endpoint: &mut LogicReady, claim: bool) -> Result<GuiIo> {
    let stream = UnixStream::connect(endpoint.root.join("gui.sock")).await?;
    let mut io = GuiIo::new(stream, endpoint.capability);
    let id = bbt_mux::Ulid::new();
    io.try_send(GuiAction::GuiHello { id, claim })?;
    tokio::time::timeout(STARTUP_TIMEOUT, async {
        loop {
            match io.event().await {
                Event::Reply(reply) => match *reply {
                    GuiReply::GuiHello { op, hello } if op == id => {
                        if hello.run_dir != config.run_dir
                            || hello.account_root != endpoint.account_root
                        {
                            return Err(invalid("GUI hello account/run mismatch"));
                        }
                        endpoint.window = hello.window;
                        return Ok(());
                    }
                    GuiReply::Identity(identity) => endpoint.identity = identity,
                    GuiReply::Error { reason, .. } => return Err(invalid(reason)),
                    _ => {}
                },
                Event::Disconnected(reason) => return Err(invalid(reason)),
                _ => {}
            }
        }
    })
    .await
    .map_err(|_| invalid("GUI hello deadline elapsed"))??;
    Ok(io)
}

async fn start_supervisor(
    config: &Config,
    launch_path: &std::ffi::OsStr,
) -> Result<StartedSupervisor> {
    let mut child = child(config, "supervisor", launch_path)?;
    write_json(child.stdin.as_mut().unwrap(), config).await?;
    let mut output = BufReader::new(child.stdout.take().unwrap());
    let line = tokio::time::timeout(STARTUP_TIMEOUT, read_line(&mut output))
        .await
        .map_err(|_| invalid("supervisor startup timed out waiting for its readiness line"))??
        .ok_or_else(|| invalid("supervisor exited during startup"))?;
    let ready = serde_json::from_slice(&line)?;
    let control = child.stdin.take();
    Ok(StartedSupervisor {
        child,
        control,
        ready,
    })
}
async fn start_logic(
    config: &Config,
    ready: &SupervisorReady,
    recover_seats: bool,
    launch_path: &std::ffi::OsStr,
) -> Result<StartedLogic> {
    let mut child = child(config, "logic", launch_path)?;
    let fixture_hosts = fixture_hosts::enabled(
        std::env::args().any(|arg| arg == "--smoke"),
        config.test_account.is_some(),
    );
    write_json(
        child.stdin.as_mut().unwrap(),
        &LogicBootstrap {
            config: config.clone(),
            supervisor: ready.clone(),
            recover_seats,
            fixture_hosts,
        },
    )
    .await?;
    let mut output = BufReader::new(child.stdout.take().unwrap());
    let line = tokio::time::timeout(STARTUP_TIMEOUT, read_line(&mut output))
        .await
        .map_err(|_| invalid("logic startup timed out waiting for its readiness line"))??
        .ok_or_else(|| invalid("logic exited during startup"))?;
    let mut endpoint: LogicReady = serde_json::from_slice(&line)?;
    let io = attach_gui(config, &mut endpoint, true).await?;
    let control = child.stdin.take();
    Ok(StartedLogic {
        child,
        control,
        endpoint,
        io,
    })
}
async fn recover(
    config: Config,
    ready: SupervisorReady,
    replace_supervisor: bool,
    watches: BTreeMap<SeatId, View>,
    launch_path: std::ffi::OsString,
) -> Result<Recovered> {
    let supervisor = if replace_supervisor {
        Some(start_supervisor(&config, &launch_path).await?)
    } else {
        None
    };
    let ready = supervisor
        .as_ref()
        .map_or(&ready, |supervisor| &supervisor.ready);
    let mut logic = start_logic(&config, ready, replace_supervisor, &launch_path).await?;
    logic.io.try_send(GuiAction::ListSeats)?;
    let seats = loop {
        match logic.io.event().await {
            Event::Reply(reply) => match *reply {
                GuiReply::Seats(seats) => break seats,
                GuiReply::Identity(identity) => logic.endpoint.identity = identity,
                GuiReply::Error { reason, .. } => return Err(invalid(reason)),
                _ => {}
            },
            Event::Disconnected(reason) => return Err(invalid(reason)),
            Event::Exited { process, status } => {
                return Err(invalid(format!("{process} exited: {status}")));
            }
        }
    };
    let watches = watches
        .keys()
        .filter_map(|seat| {
            let record = &seats
                .iter()
                .find(|item| item.record.id.0 == seat.0.0)?
                .record;
            Some((
                *seat,
                View {
                    seat: record.id,
                    generation: record.generation,
                    view_generation: bbt_mux::Ulid::new(),
                },
            ))
        })
        .collect();
    Ok(Recovered {
        supervisor,
        logic,
        seats,
        watches,
    })
}

async fn read_replies(read: OwnedReadHalf, sender: mpsc::Sender<Result<GuiReply>>) {
    let mut reader = BufReader::new(read);
    loop {
        let reply = match bbt_mux::seat_socket::read_gui_line(&mut reader).await {
            Ok(Some(line)) => serde_json::from_slice(&line).map_err(Into::into),
            Ok(None) => return,
            Err(error) => Err(error),
        };
        let failed = reply.is_err();
        if sender.send(reply).await.is_err() || failed {
            return;
        }
    }
}

#[allow(
    clippy::disallowed_methods,
    reason = "child bootstrap opens its private stderr file on IO, guarded off the UI thread"
)]
fn child(config: &Config, entry: &str, launch_path: &std::ffi::OsStr) -> Result<Child> {
    bbt_rt::assert_not_ui_thread();
    use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(&config.run_dir)?;
    let stderr = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(config.run_dir.join(format!("{entry}.log")))?;
    let mut command = Command::new(&config.executable);
    command
        .env("BBOXY_RUN_DIR", &config.run_dir)
        .env("PATH", launch_path);
    if let Some(root) = &config.shell_config {
        command.env("ZDOTDIR", root);
    }
    Ok(command
        .arg(entry)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::from(stderr))
        .kill_on_drop(false)
        .spawn()
        .map(Child::new)
        .inspect_err(|e| {
            bbt_rt::error_journal::report(bbt_rt::error_journal::Failure::new("spawn", entry, e));
        })?)
}
/// A broken adopted logic owner may exit without retiring its record. Only
/// confirmed process departure permits replacement; live owners still refuse.
async fn stop_logic_for_refresh(
    child: &mut Process,
    control: &mut Option<tokio::process::ChildStdin>,
) -> Result<()> {
    match stop_child(child, control, "logic").await {
        Ok(()) => Ok(()),
        Err(error) => {
            if child.try_wait()?.is_none() {
                return Err(error);
            }
            eprintln!(
                "previous logic departed during refresh: {error}; retaining supervisor and seats"
            );
            Ok(())
        }
    }
}

async fn stop_child(
    child: &mut Process,
    control: &mut Option<tokio::process::ChildStdin>,
    name: &str,
) -> Result<()> {
    // A previously observed death is not a failure of this shutdown request.
    if child.try_wait()?.is_some() {
        return Ok(());
    }
    child.request_stop()?;
    if let Some(mut input) = control.take() {
        let _ = input.write_all(b"stop\n").await;
        let _ = input.shutdown().await;
    }
    match tokio::time::timeout(Duration::from_secs(5), child.wait()).await {
        Ok(status) => {
            let status = status?;
            if !status.success() {
                return Err(invalid(format!("{name} failed during shutdown: {status}")));
            }
        }
        Err(_) => {
            child.kill().await?;
            child.wait().await?;
            return Err(invalid(format!(
                "{name} shutdown timed out; child was killed"
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
#[allow(
    clippy::disallowed_methods,
    reason = "headless GUI transport fixtures own their Tokio runtime"
)]
mod gui_io_tests {
    use super::*;

    #[test]
    fn legacy_logic_ready_defaults_swarm_api_marker_and_accepts_additive_fields() {
        let mut value = serde_json::json!({
            "root":"/private/fixture",
            "capability":vec![0;32],
            "identity":{"state":"held","reason":"private fixture"},
            "mcp_control_version":MCP_CONTROL_VERSION,
            "gui_state_version":bbt_proto::gui::WINDOW_STATE_VERSION
        });
        let legacy: LogicReady = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(legacy.swarm_metadata_version, 0);
        assert_eq!(legacy.ui_control_version, 0);
        value["swarm_metadata_version"] = serde_json::json!(bbt_proto::gui::SWARM_METADATA_VERSION);
        value["ui_control_version"] = serde_json::json!(bbt_proto::gui::UI_CONTROL_VERSION);

        assert_eq!(legacy.swarm_workspace_version, 0);
        value["swarm_workspace_version"] =
            serde_json::json!(bbt_proto::gui::SWARM_WORKSPACE_VERSION);
        value["future_ready_hint"] = serde_json::json!(true);
        let current: LogicReady = serde_json::from_value(value).unwrap();
        assert_eq!(
            current.ui_control_version,
            bbt_proto::gui::UI_CONTROL_VERSION
        );
        assert_eq!(
            current.swarm_metadata_version,
            bbt_proto::gui::SWARM_METADATA_VERSION
        );
        assert_eq!(
            current.swarm_workspace_version,
            bbt_proto::gui::SWARM_WORKSPACE_VERSION
        );
    }

    // keeps: spec mux-daemon; a failed logic shutdown cannot require a second UI launch
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ignore = "real process shutdown boundary; make test-boundaries"]
    async fn refresh_replaces_a_confirmed_failed_process_without_requiring_clean_shutdown() {
        let native = Command::new("/bin/sh")
            .args(["-c", "read -r line; exit 7"])
            .env_clear()
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let mut child = Child::new(native);
        let mut control = child.stdin.take();
        let mut process = Process::Child(child);
        stop_logic_for_refresh(&mut process, &mut control)
            .await
            .unwrap();
        assert!(
            process
                .try_wait()
                .unwrap()
                .is_some_and(|exit| !exit.success())
        );
    }

    // keeps: spec c1-live-seats §2a (bounded GUI transport)
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn full_duplex_bursts_keep_all_replies_and_inputs_in_order_without_write_deadline() {
        const COUNT: usize = 1000;
        let (stream, peer) = UnixStream::pair().unwrap();
        let capability = [17; 32];
        let mut io = GuiIo::new(stream, capability);
        let started = std::time::Instant::now();
        let (release_reader, reader_ready) = tokio::sync::oneshot::channel();
        let mut release_reader = Some(release_reader);
        let peer = tokio::spawn(async move {
            let (read, mut write) = peer.into_split();
            // Exceed the socket buffers and bounded reply queue while both
            // fake-peer halves progress independently under backpressure.
            let replies = async {
                for index in 0..COUNT {
                    let frame = json_frame(&GuiReply::Error {
                        op: bbt_mux::Ulid::from(index as u128 + 1),
                        reason: format!("reply-{index}:{}", "x".repeat(4096)),
                    })
                    .unwrap();
                    // This peer measures ordering/backpressure, not the
                    // product startup writer's real-time two-second budget.
                    write.write_all(&frame).await.unwrap();
                }
            };
            let inputs = async {
                // Force queue pressure before allowing the concurrent drain.
                reader_ready.await.unwrap();
                let mut reader = BufReader::new(read);
                for index in 0..COUNT {
                    let bytes = read_line(&mut reader).await.unwrap().unwrap();
                    let request: GuiRequest = serde_json::from_slice(&bytes).unwrap();
                    assert_eq!(
                        (request.version, request.capability),
                        (bbt_proto::gui::VERSION, capability)
                    );
                    let GuiAction::Input {
                        seat,
                        generation,
                        id,
                        bytes,
                    } = request.action
                    else {
                        panic!("unexpected action")
                    };
                    assert_eq!(seat, SeatId::from_u128(7));
                    assert_eq!(generation, 7);
                    assert_eq!(id, bbt_mux::Ulid::from(index as u128 + 1));
                    assert_eq!(bytes.len(), 4096);
                    assert_eq!(&bytes[..8], &(index as u64).to_le_bytes());
                }
            };
            tokio::join!(replies, inputs);
        });
        let (mut sent, mut received, mut busy) = (0, 0, 0);
        {
            while sent < COUNT || received < COUNT {
                while sent < COUNT {
                    let mut bytes = vec![b'a'; 4096];
                    bytes[..8].copy_from_slice(&(sent as u64).to_le_bytes());
                    match io.try_send(GuiAction::Input {
                        seat: SeatId::from_u128(7),
                        generation: 7,
                        id: bbt_mux::Ulid::from(sent as u128 + 1),
                        bytes,
                    }) {
                        Ok(()) => sent += 1,
                        Err(error) if error.is::<Busy>() => {
                            busy += 1;
                            if let Some(release) = release_reader.take() {
                                release.send(()).unwrap();
                            }
                            break;
                        }
                        Err(error) => panic!("enqueue failed: {error}"),
                    }
                }
                if sent == COUNT && received == COUNT {
                    break;
                }
                // Retain the unsent action while still draining inbound data.
                // Capacity notification is only a test wake; production's IO
                // owner keeps the command in its existing bounded retry queue.
                let writable = io.writer_ready();
                tokio::select! {
                    event=io.event(), if received < COUNT => {
                        let Event::Reply(reply) = event else {panic!("transport disconnected before the burst completed")};
                        let GuiReply::Error {op, reason} = *reply else {panic!("reply kind changed")};
                        assert_eq!(op, bbt_mux::Ulid::from(received as u128 + 1));
                        assert!(reason.starts_with(&format!("reply-{received}:")));
                        received += 1;
                    }
                    ready=writable, if sent < COUNT => {ready.unwrap();}
                }
            }
            peer.await.unwrap();
        }
        assert!(busy > 0, "fixture did not fill the bounded writer queue");
        println!(
            "full-duplex: {sent} inputs / {received} replies, ordered; Busy {busy}; no write deadline; elapsed {:?}",
            started.elapsed()
        );
    }

    // keeps: spec c1-live-seats §2a (bounded GUI transport)
    #[tokio::test(start_paused = true)]
    async fn startup_write_deadline_is_two_seconds_of_controlled_time() {
        use std::future::{Future, poll_fn};
        use std::task::Poll;

        let (mut writer, _peer) = tokio::io::duplex(1);
        writer.write_all(b"x").await.unwrap();
        let started = tokio::time::Instant::now();
        let write = write_frame(&mut writer, b"blocked");
        tokio::pin!(write);
        // Poll once to arm the real product timeout against the paused clock.
        assert!(
            poll_fn(|cx| Poll::Ready(write.as_mut().poll(cx)))
                .await
                .is_pending()
        );
        tokio::time::sleep(Duration::from_secs(1)).await;
        assert!(
            poll_fn(|cx| Poll::Ready(write.as_mut().poll(cx)))
                .await
                .is_pending()
        );
        let error = tokio::time::timeout(Duration::from_millis(1100), write)
            .await
            .expect("product writer exceeded its controlled two-second deadline")
            .unwrap_err();
        assert!(error.is::<tokio::time::error::Elapsed>());
        // Tokio's timer wheel has millisecond granularity. This bounds virtual
        // time only; scheduler wall time is not part of the assertion.
        let elapsed = tokio::time::Instant::now() - started;
        assert!(
            (Duration::from_secs(2)..=Duration::from_millis(2002)).contains(&elapsed),
            "unexpected controlled deadline: {elapsed:?}"
        );
    }

    // keeps: spec c1-live-seats §2a (bounded GUI transport)
    #[tokio::test]
    async fn writer_task_death_is_disconnected_even_while_the_reader_stays_connected() {
        let (stream, _peer) = UnixStream::pair().unwrap();
        let mut io = GuiIo::new(stream, [18; 32]);
        io.writer_task.abort();
        // Notification determines success; this is only the policy's hang guard.
        let event = tokio::time::timeout(Duration::from_secs(30), io.event())
            .await
            .expect("writer death did not notify the IO owner");
        let Event::Disconnected(reason) = event else {
            panic!("writer death must disconnect the public transport");
        };
        assert!(!reason.is_empty(), "disconnection must explain itself");
        assert!(io.try_send(GuiAction::ListSeats).is_err());
    }

    // keeps: spec c1-live-seats §2a (bounded GUI transport)
    #[tokio::test]
    async fn oversized_request_is_rejected_before_enqueue_without_stopping_the_writer() {
        let (stream, peer) = UnixStream::pair().unwrap();
        let io = GuiIo::new(stream, [19; 32]);
        let error = io
            .try_send(GuiAction::Input {
                seat: SeatId::from_u128(7),
                generation: 7,
                id: bbt_mux::Ulid::new(),
                bytes: vec![255; 300_000],
            })
            .unwrap_err();
        assert!(error.to_string().contains("1 MiB"));
        let mut reader = BufReader::new(peer);
        for _ in 0..2 {
            io.try_send(GuiAction::ListSeats).unwrap();
            let line = read_line(&mut reader).await.unwrap().unwrap();
            let request: GuiRequest = serde_json::from_slice(&line).unwrap();
            assert!(matches!(request.action, GuiAction::ListSeats));
        }
    }
}
