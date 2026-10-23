//! One IO runtime, with bounded memory-only handoffs to the UI.
//!
//! With the default `runtime` feature, call `init_ui` once at app startup.
//! The app implements [`Waker`] using its
//! event-loop proxy; no winit or renderer dependency belongs in this crate.

#[cfg(feature = "runtime")]
use std::sync::Arc;
use std::sync::OnceLock;
use std::thread::{self, ThreadId};

#[cfg(feature = "runtime")]
use tokio::runtime::{Builder, Handle, Runtime};

#[cfg(feature = "runtime")]
mod commands;
mod meter;
#[cfg(feature = "runtime")]
mod queues;
mod wake;

#[cfg(feature = "runtime")]
pub use commands::{CommandRunner, ExitStatus, OutputChunk, Stream, TaskId, cancel, spawn_command};
pub use meter::{FrameSample, HUD_FRAMES, HitchMeter, HudFrame, PHASE_NAMES, PerfRecord, Phases};
#[cfg(feature = "runtime")]
pub use queues::{Latest, UiMsg, UiReceiver, UiSender, ViewRing, ui_channel};
pub use wake::{DirtySet, DirtySource, Waker};

#[cfg(feature = "runtime")]
static RUNTIME: OnceLock<Runtime> = OnceLock::new();
static UI_THREAD: OnceLock<ThreadId> = OnceLock::new();
#[cfg(feature = "runtime")]
static COMMANDS: OnceLock<CommandRunner> = OnceLock::new();

/// Start the process-wide two-worker IO runtime, or return the existing handle.
#[cfg(feature = "runtime")]
pub fn handle() -> &'static Handle {
    RUNTIME
        .get_or_init(|| {
            Builder::new_multi_thread()
                .worker_threads(2)
                .thread_name("bbt-io")
                .enable_all()
                .build()
                .expect("start bbt IO runtime")
        })
        .handle()
}

/// Register the UI thread at startup. A different thread cannot replace it.
pub fn register_ui_thread() {
    let current = thread::current().id();
    assert_eq!(
        *UI_THREAD.get_or_init(|| current),
        current,
        "UI thread already registered"
    );
}

pub fn is_ui_thread() -> bool {
    UI_THREAD
        .get()
        .is_some_and(|id| *id == thread::current().id())
}

/// Guard a blocking IO entry point; the panic points to its caller.
#[track_caller]
pub fn assert_not_ui_thread() {
    debug_assert!(!is_ui_thread(), "blocking IO on the UI thread");
}

/// Memory-only state the UI drains at frame start, under its own time budget.
#[cfg(feature = "runtime")]
pub struct UiInbox {
    pub sender: UiSender,
    pub messages: UiReceiver,
    pub output: ViewRing<OutputChunk>,
    pub dirty: DirtySet,
}

/// Startup only. Source 0 is messages, source 1 is command output; the remaining
/// sources are available for parsers, stores and other app producers.
#[cfg(feature = "runtime")]
pub fn init_ui(waker: impl Waker) -> UiInbox {
    register_ui_thread();
    handle();
    let dirty = DirtySet::new(1024, Arc::new(waker));
    let (sender, messages) = ui_channel(256, dirty.source(0));
    let output = ViewRing::new(256, dirty.source(1));
    assert!(
        COMMANDS
            .set(CommandRunner::new(sender.clone(), output.clone()))
            .is_ok(),
        "UI already initialized"
    );
    UiInbox {
        sender,
        messages,
        output,
        dirty,
    }
}
