//! App-owned child processes and the single, admitted GUI connection.
//!
//! Everything in this module runs on bbt-io or in a private non-UI process.
//! Capabilities travel over inherited pipes and are never put in argv or logs.

use std::{
    collections::{BTreeMap, VecDeque},
    path::PathBuf,
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
    process::{Child, Command},
    sync::mpsc,
};

const STARTUP_TIMEOUT: Duration = Duration::from_secs(30);
const GUI_WRITE_CAPACITY: usize = 64;

/// App-defined, bounded UI-to-IO messages. A full sender returns ownership to
/// the UI for retry; no socket or process operation runs in the window handler.
pub enum UiMsg {
    SpawnZsh,
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

#[derive(Clone, Serialize, Deserialize)]
struct SupervisorReady {
    socket: PathBuf,
    capability: [u8; 32],
}
#[derive(Serialize, Deserialize)]
struct LogicBootstrap {
    config: Config,
    supervisor: SupervisorReady,
    #[serde(default)]
    recover_seats: bool,
}
/// The one-line private startup reply. Never log its capability.
#[derive(Clone, Serialize, Deserialize)]
pub struct LogicReady {
    pub root: PathBuf,
    pub capability: [u8; 32],
    pub identity: GuiIdentity,
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
            version: 1,
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
    let supervisor = bbt_mux::Supervisor::start(bbt_mux::Config {
        socket_path: Some(config.socket.clone()),
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
    let logic = Logic::start_custodied_with_fixtures(
        connection,
        LogicConfig {
            run_dir: bootstrap.config.run_dir,
            repository: bootstrap.config.repository,
            ring: Default::default(),
        },
        bbt_rt::handle(),
    )?;
    if bootstrap.recover_seats {
        logic.client.recover_seats().await?;
    }
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
    config: Config,
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
        status: std::process::ExitStatus,
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
    pub async fn start(config: Config) -> Result<Self> {
        bbt_rt::assert_not_ui_thread();
        let supervisor = start_supervisor(&config).await?;
        let logic = start_logic(&config, &supervisor.ready, false).await?;
        Ok(Self {
            config,
            supervisor_ready: supervisor.ready,
            supervisor: supervisor.child,
            supervisor_control: supervisor.control,
            logic: logic.child,
            logic_control: logic.control,
            endpoint: logic.endpoint,
            io: logic.io,
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
        let ready = self.supervisor_ready.clone();
        let replace_supervisor = self.supervisor_exited;
        let watches = self.watches.clone();
        self.recovery = Some(tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            tokio::time::timeout(
                STARTUP_TIMEOUT,
                recover(config, ready, replace_supervisor, watches),
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
                                self.supervisor = supervisor.child;
                                self.supervisor_control = supervisor.control;
                                self.supervisor_ready = supervisor.ready;
                            }
                            let logic = recovered.logic;
                            self.logic = logic.child;
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
        loop {
            match self.receive().await? {
                GuiReply::Seats(seats) => return Ok(seats),
                GuiReply::Identity(identity) => self.endpoint.identity = identity,
                GuiReply::Error { reason, .. } => return Err(invalid(reason)),
                GuiReply::Seat(_)
                | GuiReply::Update { .. }
                | GuiReply::Ok { .. }
                | GuiReply::Hosts(_)
                | GuiReply::HostState { .. } => {}
                _ => return Err(invalid("unexpected GUI startup reply")),
            }
        }
    }
    /// Abort startup before stopping and reaping both installed children.
    pub async fn shutdown(mut self) -> Result<()> {
        if let Some(task) = self.recovery.take() {
            task.abort();
            if let Ok(Ok(mut recovered)) = task.await {
                let _ = stop_child(
                    &mut recovered.logic.child,
                    &mut recovered.logic.control,
                    "logic",
                )
                .await;
                if let Some(mut supervisor) = recovered.supervisor {
                    let _ =
                        stop_child(&mut supervisor.child, &mut supervisor.control, "supervisor")
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

async fn start_supervisor(config: &Config) -> Result<StartedSupervisor> {
    let mut child = child(config, "supervisor")?;
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
) -> Result<StartedLogic> {
    let mut child = child(config, "logic")?;
    write_json(
        child.stdin.as_mut().unwrap(),
        &LogicBootstrap {
            config: config.clone(),
            supervisor: ready.clone(),
            recover_seats,
        },
    )
    .await?;
    let mut output = BufReader::new(child.stdout.take().unwrap());
    let line = tokio::time::timeout(STARTUP_TIMEOUT, read_line(&mut output))
        .await
        .map_err(|_| invalid("logic startup timed out waiting for its readiness line"))??
        .ok_or_else(|| invalid("logic exited during startup"))?;
    let endpoint: LogicReady = serde_json::from_slice(&line)?;
    let stream = UnixStream::connect(endpoint.root.join("gui.sock")).await?;
    let io = GuiIo::new(stream, endpoint.capability);
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
) -> Result<Recovered> {
    let supervisor = if replace_supervisor {
        Some(start_supervisor(&config).await?)
    } else {
        None
    };
    let ready = supervisor
        .as_ref()
        .map_or(&ready, |supervisor| &supervisor.ready);
    let mut logic = start_logic(&config, ready, replace_supervisor).await?;
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

#[cfg(test)]
#[allow(
    clippy::disallowed_methods,
    reason = "headless GUI transport fixtures own their Tokio runtime"
)]
mod gui_io_tests {
    use super::*;

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn full_duplex_bursts_keep_all_replies_and_inputs_in_order_without_write_deadline() {
        const COUNT: usize = 1000;
        let (stream, peer) = UnixStream::pair().unwrap();
        let capability = [17; 32];
        let mut io = GuiIo::new(stream, capability);
        let started = std::time::Instant::now();
        let peer = tokio::spawn(async move {
            let (read, mut write) = peer.into_split();
            // More than the socket buffers and eight-entry reply queue. This
            // deliberately refuses to read input until the entire burst is sent.
            for index in 0..COUNT {
                write_json(
                    &mut write,
                    &GuiReply::Error {
                        op: bbt_mux::Ulid::from(index as u128 + 1),
                        reason: format!("reply-{index}:{}", "x".repeat(4096)),
                    },
                )
                .await
                .unwrap();
            }
            let mut reader = BufReader::new(read);
            for index in 0..COUNT {
                let bytes = read_line(&mut reader).await.unwrap().unwrap();
                let request: GuiRequest = serde_json::from_slice(&bytes).unwrap();
                assert_eq!((request.version, request.capability), (1, capability));
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
        });
        let (mut sent, mut received, mut busy) = (0, 0, 0);
        tokio::time::timeout(Duration::from_secs(10), async {
            while sent < COUNT || received < COUNT {
                while sent < COUNT {
                    let mut bytes = vec![b'a'; 4096];
                    bytes[..8].copy_from_slice(&(sent as u64).to_le_bytes());
                    match io.try_send(GuiAction::Input {
                        seat: SeatId::from_u128(7), generation: 7, id: bbt_mux::Ulid::from(sent as u128 + 1), bytes,
                    }) {
                        Ok(()) => sent += 1,
                        Err(error) if error.is::<Busy>() => {busy += 1; break;}
                        Err(error) => panic!("enqueue failed: {error}"),
                    }
                }
                if sent == COUNT && received == COUNT {break;}
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
        }).await.expect("full-duplex transport stalled");
        assert!(busy > 0, "fixture did not fill the bounded writer queue");
        println!(
            "full-duplex: {sent} inputs / {received} replies, ordered; Busy {busy}; no write deadline; elapsed {:?}",
            started.elapsed()
        );
    }

    #[tokio::test]
    async fn writer_task_death_is_disconnected_even_while_the_reader_stays_connected() {
        let (stream, _peer) = UnixStream::pair().unwrap();
        let mut io = GuiIo::new(stream, [18; 32]);
        io.writer_task.abort();
        let event = tokio::time::timeout(Duration::from_secs(1), io.event())
            .await
            .unwrap();
        assert!(
            matches!(event, Event::Disconnected(reason) if reason.contains("GUI writer task stopped"))
        );
        assert!(io.try_send(GuiAction::ListSeats).is_err());
    }

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
        io.try_send(GuiAction::ListSeats).unwrap();
        let line = read_line(&mut BufReader::new(peer)).await.unwrap().unwrap();
        let request: GuiRequest = serde_json::from_slice(&line).unwrap();
        assert!(matches!(request.action, GuiAction::ListSeats));
        assert!(!io.writer_task.is_finished());
    }
}
