//! App-owned child processes and the single, admitted GUI connection.
//!
//! Everything in this module runs on bbt-io or in a private non-UI process.
//! Capabilities travel over inherited pipes and are never put in argv or logs.

use std::{path::PathBuf, process::Stdio, time::Duration};

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
    process::{Child, Command},
    sync::mpsc,
};

const STARTUP_TIMEOUT: Duration = Duration::from_secs(30);

/// App-defined, bounded UI-to-IO messages. A full sender returns ownership to
/// the UI for retry; no socket or process operation runs in the window handler.
pub enum UiMsg {
    SpawnZsh,
    Input {
        seat: SeatId,
        id: bbt_mux::Ulid,
        bytes: Vec<u8>,
    },
    Resize {
        seat: SeatId,
        cols: u16,
        rows: u16,
    },
    Watched(Vec<(SeatId, View)>),
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
}
impl Config {
    /// Resolve the executable, repository and local control endpoint on IO.
    pub fn discover(run_dir: Option<PathBuf>) -> Result<Self> {
        bbt_rt::assert_not_ui_thread();
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
            repository: std::env::current_dir()?,
            shell_config: std::env::var_os("ZDOTDIR").map(PathBuf::from),
        })
    }
}

#[derive(Serialize, Deserialize)]
struct SupervisorReady {
    socket: PathBuf,
    capability: [u8; 32],
}
#[derive(Serialize, Deserialize)]
struct LogicBootstrap {
    config: Config,
    supervisor: SupervisorReady,
}
/// The one-line private startup reply. Never log its capability.
#[derive(Clone, Serialize, Deserialize)]
pub struct LogicReady {
    pub root: PathBuf,
    pub capability: [u8; 32],
    pub identity: GuiIdentity,
}

async fn write_json(writer: &mut (impl AsyncWrite + Unpin), value: &impl Serialize) -> Result<()> {
    let mut bytes = serde_json::to_vec(value)?;
    if bytes.len() >= 1024 * 1024 {
        return Err(invalid("GUI request exceeds 1 MiB framing bound"));
    }
    bytes.push(b'\n');
    tokio::time::timeout(Duration::from_secs(2), writer.write_all(&bytes)).await??;
    writer.flush().await?;
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
        Some("supervisor") => Some(bbt_rt::handle().block_on(supervisor_main())),
        Some("logic") => Some(bbt_rt::handle().block_on(logic_main())),
        _ => None,
    }
}

// EOF is not an instruction to kill seats after a GUI crash. An explicit stop
// is used on orderly app shutdown; restart/reconnection belongs to C1c.
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
    let supervisor = bbt_mux::Supervisor::start(bbt_mux::Config {
        socket_path: Some(config.socket.clone()),
        journal_directory: Some(
            bbt_mux::store::object_dir(&config.run_dir, "supervisor")?.join("journal"),
        ),
        ..Default::default()
    })?;
    write_json(
        &mut tokio::io::stdout(),
        &SupervisorReady {
            socket: config.socket,
            capability: supervisor.socket_capability(),
        },
    )
    .await?;
    wait_for_stop(&mut input).await?;
    eprintln!("supervisor: orderly shutdown requested");
    supervisor.shutdown()?;
    eprintln!("supervisor: shutdown completed");
    Ok(())
}

async fn logic_main() -> Result<()> {
    let mut input = BufReader::new(tokio::io::stdin());
    let line = read_line(&mut input)
        .await?
        .ok_or_else(|| invalid("missing logic bootstrap"))?;
    let bootstrap: LogicBootstrap = serde_json::from_slice(&line)?;
    let connection = SocketSupervisor::connect(
        &bootstrap.supervisor.socket,
        bootstrap.supervisor.capability,
    )?;
    let logic = Logic::start_custodied(
        connection,
        LogicConfig {
            run_dir: bootstrap.config.run_dir,
            repository: bootstrap.config.repository,
            ring: Default::default(),
        },
        bbt_rt::handle(),
    )?;
    let endpoint = logic.client.gui_endpoint().await?;
    write_json(
        &mut tokio::io::stdout(),
        &LogicReady {
            root: endpoint
                .path
                .parent()
                .ok_or_else(|| invalid("GUI endpoint has no root"))?
                .into(),
            capability: endpoint.capability,
            identity: logic.client.gui_identity().await?,
        },
    )
    .await?;
    wait_for_stop(&mut input).await?;
    eprintln!("logic: orderly shutdown requested");
    logic.shutdown()?;
    eprintln!("logic: shutdown completed");
    Ok(())
}

/// The IO owner, never stored on or dropped from the UI thread.
pub struct Session {
    pub supervisor: Child,
    pub logic: Child,
    pub endpoint: LogicReady,
    // Child::wait closes Child.stdin on its first poll, even when cancelled.
    // Keep the control pipes outside Child while observing process exits.
    supervisor_control: Option<tokio::process::ChildStdin>,
    logic_control: Option<tokio::process::ChildStdin>,
    incoming: mpsc::Receiver<Result<GuiReply>>,
    reader_task: tokio::task::JoinHandle<()>,
    writer: OwnedWriteHalf,
    supervisor_exited: bool,
    logic_exited: bool,
    disconnected: bool,
}
/// Socket and process notifications consumed by the app's IO owner.
pub enum Event {
    Reply(Box<GuiReply>),
    Exited {
        process: &'static str,
        status: std::process::ExitStatus,
    },
    Disconnected(String),
}
impl Drop for Session {
    fn drop(&mut self) {
        self.reader_task.abort();
    }
}
impl Session {
    /// Spawn both private processes and connect their admitted GUI endpoint.
    pub async fn start(config: Config) -> Result<Self> {
        bbt_rt::assert_not_ui_thread();
        let mut supervisor = child(&config, "supervisor")?;
        write_json(supervisor.stdin.as_mut().unwrap(), &config).await?;
        let mut output = BufReader::new(supervisor.stdout.take().unwrap());
        let line = tokio::time::timeout(STARTUP_TIMEOUT, read_line(&mut output))
            .await
            .map_err(|_| invalid("supervisor startup timed out waiting for its readiness line"))??
            .ok_or_else(|| invalid("supervisor exited during startup"))?;
        let ready: SupervisorReady = serde_json::from_slice(&line)?;
        let mut logic = child(&config, "logic")?;
        write_json(
            logic.stdin.as_mut().unwrap(),
            &LogicBootstrap {
                config,
                supervisor: ready,
            },
        )
        .await?;
        let mut output = BufReader::new(logic.stdout.take().unwrap());
        let line = tokio::time::timeout(STARTUP_TIMEOUT, read_line(&mut output))
            .await
            .map_err(|_| invalid("logic startup timed out waiting for its readiness line"))??
            .ok_or_else(|| invalid("logic exited during startup"))?;
        let endpoint: LogicReady = serde_json::from_slice(&line)?;
        let stream = UnixStream::connect(endpoint.root.join("gui.sock")).await?;
        let (read, writer) = stream.into_split();
        let (sender, incoming) = mpsc::channel(8);
        let reader_task = tokio::spawn(read_replies(read, sender));
        let supervisor_control = supervisor.stdin.take();
        let logic_control = logic.stdin.take();
        Ok(Self {
            supervisor,
            logic,
            endpoint,
            supervisor_control,
            logic_control,
            incoming,
            reader_task,
            writer,
            supervisor_exited: false,
            logic_exited: false,
            disconnected: false,
        })
    }
    /// Send one bounded request; callers retain stable input IDs across retry.
    pub async fn send(&mut self, action: GuiAction) -> Result<()> {
        write_json(
            &mut self.writer,
            &GuiRequest {
                version: 1,
                capability: self.endpoint.capability,
                action,
            },
        )
        .await
    }
    pub async fn receive(&mut self) -> Result<GuiReply> {
        self.incoming
            .recv()
            .await
            .ok_or_else(|| invalid("GUI connection closed"))?
    }
    /// Channel receive and Child::wait are cancellation-safe. The dedicated
    /// reader retains partial NDJSON frames when a UI command wins the select.
    pub async fn event(&mut self) -> Result<Event> {
        tokio::select! {
            reply=self.incoming.recv(), if !self.disconnected => {
                match reply {
                    Some(Ok(reply)) => Ok(Event::Reply(Box::new(reply))),
                    other => {
                        self.disconnected = true;
                        Ok(Event::Disconnected(match other {
                            Some(Err(error)) => error.to_string(),
                            _ => "GUI connection closed".into(),
                        }))
                    }
                }
            }
            status=self.supervisor.wait(), if !self.supervisor_exited => {
                self.supervisor_exited = true;
                Ok(Event::Exited {process:"supervisor",status:status?})
            }
            status=self.logic.wait(), if !self.logic_exited => {
                self.logic_exited = true;
                Ok(Event::Exited {process:"logic",status:status?})
            }
            else => std::future::pending().await,
        }
    }
    /// Obtain the initial catalogue before any views are watched.
    pub async fn list(&mut self) -> Result<Vec<GuiSeat>> {
        self.send(GuiAction::ListSeats).await?;
        loop {
            match self.receive().await? {
                GuiReply::Seats(seats) => return Ok(seats),
                GuiReply::Identity(identity) => self.endpoint.identity = identity,
                GuiReply::Error { reason, .. } => return Err(invalid(reason)),
                GuiReply::Seat(_) | GuiReply::Update { .. } => {}
                _ => return Err(invalid("unexpected GUI startup reply")),
            }
        }
    }
    /// Stop and reap both children, attempting both even if the first fails.
    pub async fn shutdown(mut self) -> Result<()> {
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

async fn read_replies(read: OwnedReadHalf, sender: mpsc::Sender<Result<GuiReply>>) {
    let mut reader = BufReader::new(read);
    loop {
        let reply = match read_line(&mut reader).await {
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

fn child(config: &Config, entry: &str) -> Result<Child> {
    bbt_rt::assert_not_ui_thread();
    let mut command = Command::new(&config.executable);
    if let Some(root) = &config.shell_config {
        command.env("ZDOTDIR", root);
    }
    Ok(command
        .arg(entry)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .kill_on_drop(true)
        .spawn()?)
}
async fn stop_child(
    child: &mut Child,
    control: &mut Option<tokio::process::ChildStdin>,
    name: &str,
) -> Result<()> {
    // A previously observed death is not a failure of this shutdown request.
    if child.try_wait()?.is_some() {
        return Ok(());
    }
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
