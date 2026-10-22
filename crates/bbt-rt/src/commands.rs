//! Process spawning and pipe reads run exclusively on the IO runtime.

use std::ffi::OsString;
use std::io;
use std::os::unix::process::ExitStatusExt;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::{
    Arc, OnceLock,
    atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering},
};

use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::process::Command;
use tokio::sync::Notify;

use crate::{COMMANDS, UiMsg, UiSender, ViewRing, assert_not_ui_thread, handle};

pub const OUTPUT_CHUNK_BYTES: usize = 16 * 1024;
static NEXT_TASK: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stream {
    Stdout,
    Stderr,
}

#[derive(Clone, Debug)]
pub struct OutputChunk {
    pub task: TaskId,
    pub stream: Stream,
    pub bytes: Arc<[u8]>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExitStatus {
    pub code: Option<i32>,
    pub signal: Option<i32>,
    pub cancelled: bool,
    pub error: Option<String>,
    pub stdout_bytes: u64,
    pub stderr_bytes: u64,
}

#[derive(Debug)]
struct Task {
    id: u64,
    pid: AtomicU32,
    cancelled: AtomicBool,
    notify: Notify,
    status: OnceLock<ExitStatus>,
}

/// Stable task identity and latest completion slot. Holding a task never waits.
/// The slot remains observable even if the bounded UiMsg queue rejects Exited.
#[derive(Clone, Debug)]
pub struct TaskId(Arc<Task>);

impl TaskId {
    pub fn id(&self) -> u64 {
        self.0.id
    }
    pub fn pid(&self) -> Option<u32> {
        match self.0.pid.load(Ordering::Acquire) {
            0 => None,
            pid => Some(pid),
        }
    }
    pub fn status(&self) -> Option<&ExitStatus> {
        self.0.status.get()
    }
    async fn cancelled(&self) {
        loop {
            let notified = self.0.notify.notified();
            if self.0.cancelled.load(Ordering::Acquire) {
                return;
            }
            notified.await;
        }
    }
}

impl PartialEq for TaskId {
    fn eq(&self, other: &Self) -> bool {
        self.id() == other.id()
    }
}
impl Eq for TaskId {}

#[derive(Clone)]
pub struct CommandRunner {
    ui: UiSender,
    output: ViewRing<OutputChunk>,
}

impl CommandRunner {
    pub fn new(ui: UiSender, output: ViewRing<OutputChunk>) -> Self {
        Self { ui, output }
    }

    /// The arguments and explicit environment overrides are passed directly to
    /// the child, without shell interpolation. Returns before spawning or IO.
    pub fn spawn_command(
        &self,
        argv: Vec<OsString>,
        cwd: PathBuf,
        env: Vec<(OsString, OsString)>,
    ) -> TaskId {
        let task = TaskId(Arc::new(Task {
            id: NEXT_TASK.fetch_add(1, Ordering::Relaxed),
            pid: AtomicU32::new(0),
            cancelled: AtomicBool::new(false),
            notify: Notify::new(),
            status: OnceLock::new(),
        }));
        let runner = self.clone();
        let running = task.clone();
        handle().spawn(async move {
            runner.run(running, argv, cwd, env).await;
        });
        task
    }

    async fn run(
        self,
        task: TaskId,
        argv: Vec<OsString>,
        cwd: PathBuf,
        env: Vec<(OsString, OsString)>,
    ) {
        assert_not_ui_thread();
        let mut status = ExitStatus {
            code: None,
            signal: None,
            cancelled: false,
            error: None,
            stdout_bytes: 0,
            stderr_bytes: 0,
        };
        let result = self.run_process(&task, argv, cwd, env, &mut status).await;
        if let Err(error) = result {
            status.error = Some(error.to_string());
        }
        task.0
            .status
            .set(status.clone())
            .expect("task completes only once");
        let _ = self.ui.try_send(UiMsg::Exited { task, status });
        // Completion must wake even when the notification queue is saturated.
        self.output.mark_dirty();
    }

    async fn run_process(
        &self,
        task: &TaskId,
        argv: Vec<OsString>,
        cwd: PathBuf,
        env: Vec<(OsString, OsString)>,
        status: &mut ExitStatus,
    ) -> io::Result<()> {
        if task.0.cancelled.load(Ordering::Acquire) {
            status.cancelled = true;
            return Ok(());
        }
        let (program, args) = argv
            .split_first()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "empty command argv"))?;
        let mut child = Command::new(program)
            .args(args)
            .current_dir(cwd)
            .envs(env)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .process_group(0)
            .kill_on_drop(true)
            .spawn()?;
        let pid = child.id().expect("new child has a PID");
        task.0.pid.store(pid, Ordering::Release);
        let stdout = child.stdout.take().expect("piped stdout");
        let stderr = child.stderr.take().expect("piped stderr");
        let completion = async {
            tokio::join!(
                child.wait(),
                self.pump(task, Stream::Stdout, stdout),
                self.pump(task, Stream::Stderr, stderr)
            )
        };
        tokio::pin!(completion);
        let (exit, stdout, stderr) = tokio::select! {
            result = &mut completion => result,
            () = task.cancelled() => {
                status.cancelled = true;
                kill_group(pid)?;
                completion.await
            }
        };
        let exit = exit?;
        status.code = exit.code();
        status.signal = exit.signal();
        status.stdout_bytes = stdout?;
        status.stderr_bytes = stderr?;
        Ok(())
    }

    async fn pump(
        &self,
        task: &TaskId,
        stream: Stream,
        mut pipe: impl AsyncRead + Unpin,
    ) -> io::Result<u64> {
        let mut buffer = [0; OUTPUT_CHUNK_BYTES];
        let mut total = 0;
        loop {
            let count = pipe.read(&mut buffer).await?;
            if count == 0 {
                break;
            }
            total += count as u64;
            for line in buffer[..count].split_inclusive(|&byte| byte == b'\n') {
                self.publish(task, stream, line.to_vec());
            }
        }
        Ok(total)
    }

    fn publish(&self, task: &TaskId, stream: Stream, bytes: Vec<u8>) {
        let chunk = OutputChunk {
            task: task.clone(),
            stream,
            bytes: bytes.into(),
        };
        self.output.push(chunk.clone());
        let _ = self.ui.try_send(UiMsg::Output(chunk));
    }
}

/// Uses the app's singleton command runner installed by init_ui.
pub fn spawn_command(argv: Vec<OsString>, cwd: PathBuf, env: Vec<(OsString, OsString)>) -> TaskId {
    COMMANDS
        .get()
        .expect("call init_ui before spawn_command")
        .spawn_command(argv, cwd, env)
}

/// Signal cancellation without a lock, syscall or wait on the calling thread.
pub fn cancel(task: &TaskId) {
    if task.status().is_none() {
        task.0.cancelled.store(true, Ordering::Release);
        task.0.notify.notify_one();
    }
}

fn kill_group(pid: u32) -> io::Result<()> {
    assert_not_ui_thread();
    // SAFETY: process_group(0) made this child's PID its PGID. A negative PID
    // targets that group; SIGKILL cannot be handled or ignored by descendants.
    if unsafe { libc::kill(-(pid as i32), libc::SIGKILL) } == -1 {
        let error = io::Error::last_os_error();
        if error.raw_os_error() != Some(libc::ESRCH) {
            return Err(error);
        }
    }
    Ok(())
}
