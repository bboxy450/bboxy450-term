#![doc = include_str!("../README.md")]
#![warn(rustdoc::broken_intra_doc_links)]
#![deny(missing_docs)]

#[cfg(feature = "runtime")]
use std::sync::Arc;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread::{self, ThreadId};

#[cfg(feature = "runtime")]
use tokio::runtime::{Builder, Handle, Runtime};

#[cfg(feature = "runtime")]
mod commands;
#[cfg(feature = "runtime")]
pub mod diagnostics;
#[cfg(feature = "runtime")]
pub mod fault;
mod meter;
pub mod error_journal;
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
static UI_BLOCKING_VIOLATIONS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "runtime")]
static COMMANDS: OnceLock<CommandRunner> = OnceLock::new();

/// Start the process-wide two-worker IO runtime, or return the existing handle.
///
/// # Panics
/// Panics if Tokio cannot create the runtime and its worker threads.
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
///
/// # Panics
/// Panics when another thread already owns the UI registration.
pub fn register_ui_thread() {
    let current = thread::current().id();
    assert_eq!(
        *UI_THREAD.get_or_init(|| current),
        current,
        "UI thread already registered"
    );
}

/// Return whether the caller is the registered UI thread; false before registration.
pub fn is_ui_thread() -> bool {
    UI_THREAD
        .get()
        .is_some_and(|id| *id == thread::current().id())
}

/// Guard a blocking IO entry point; the panic points to its caller.
///
/// # Panics
/// In debug builds, panics and counts a violation when called on the registered UI thread.
#[track_caller]
pub fn assert_not_ui_thread() {
    if cfg!(debug_assertions) && is_ui_thread() {
        UI_BLOCKING_VIOLATIONS.fetch_add(1, Ordering::Relaxed);
        panic!("blocking IO on the UI thread");
    }
}

/// R5 counts violations of instrumented application entry points, not all OS
/// or framework waits. Debug guards must be enabled for this witness.
pub fn ui_blocking_violations() -> u64 {
    UI_BLOCKING_VIOLATIONS.load(Ordering::Relaxed)
}

/// Memory-only state the UI drains at frame start, under its own time budget.
#[cfg(feature = "runtime")]
pub struct UiInbox {
    /// Bounded endpoint given to producers of individual UI messages.
    pub sender: UiSender,
    /// Consumer endpoint drained by the registered UI owner.
    pub messages: UiReceiver,
    /// Disposable command-output history; inspect its dropped counter for eviction.
    pub output: ViewRing<OutputChunk>,
    /// Wake-source registry shared by the queues and application producers.
    pub dirty: DirtySet,
}

/// Startup only. Source 0 is messages, source 1 is command output; the remaining
/// sources are available for parsers, stores and other app producers.
///
/// # Panics
/// Panics on a second initialization, a conflicting UI-thread registration or runtime startup failure.
#[cfg(feature = "runtime")]
pub fn init_ui(waker: impl Waker) -> UiInbox {
    register_ui_thread();
    handle();
    let dirty = DirtySet::new(1024, Arc::new(waker));
    let (sender, messages) = ui_channel(256, dirty.deferred_source(0));
    let output = ViewRing::new(256, dirty.deferred_source(1));
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
