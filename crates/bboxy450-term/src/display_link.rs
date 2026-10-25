//! The Metal path owns the drawable supplied by the display link. It never
//! dequeues a second drawable through wgpu for the same animation tick.
use bbt_rt::{DirtySource, Latest};
use objc2::{
    AnyThread, DefinedClass, MainThreadMarker, MainThreadOnly, available, define_class, msg_send,
    rc::Retained, runtime::ProtocolObject,
};
use objc2_app_kit::NSScreen;
use objc2_foundation::{NSObject, NSObjectProtocol, NSRunLoop, NSRunLoopCommonModes};
use objc2_quartz_core::{
    CAFrameRateRange, CAMetalDisplayLink, CAMetalDisplayLinkDelegate, CAMetalDisplayLinkUpdate,
    CAMetalDrawable,
};
use std::{
    cell::{Cell, RefCell},
    ffi::c_void,
    ptr,
    sync::atomic::{AtomicU64, Ordering},
};
use winit::platform::macos::MonitorHandleExtMacOS;

pub type Drawable = Retained<ProtocolObject<dyn CAMetalDrawable>>;
pub struct Tick {
    pub target: f64,
    pub drawable: Option<Drawable>,
}

/// The display-link source allocated by App; ticks are not data invalidations.
pub const TICK_SOURCE: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Rendering work demanded by one coalesced UI wake.
pub enum WakeAction {
    Ignore,
    Tick,
    Invalidate,
}

/// Decide whether a coalesced wake needs a new display-link request.
pub fn wake_action(sources: &[usize], active: bool) -> WakeAction {
    if sources.iter().any(|source| *source != TICK_SOURCE) {
        WakeAction::Invalidate
    } else if !sources.is_empty() && active {
        WakeAction::Tick
    } else {
        WakeAction::Ignore
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Fifo,
    Metal,
    Cv,
}
impl Mode {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "fifo" => Ok(Self::Fifo),
            "metal" => Ok(Self::Metal),
            "cv" => Ok(Self::Cv),
            _ => Err("--pacing must be fifo, metal or cv".into()),
        }
    }
}

#[derive(Default)]
pub struct Clock {
    epoch: Option<f64>,
    elapsed: f64,
    last: Option<f64>,
}
impl Clock {
    pub fn observe(&mut self, target: f64) -> Option<f32> {
        if !target.is_finite() || target <= 0. || self.last.is_some_and(|last| target < last) {
            return None;
        }
        let epoch = *self.epoch.get_or_insert(target - self.elapsed / 1000.);
        self.elapsed = (target - epoch) * 1000.;
        self.last = Some(target);
        Some(self.elapsed as f32)
    }
    pub fn pause(&mut self) {
        self.epoch = None;
        self.last = None;
    }
}

struct MetalState {
    pending: RefCell<Option<Tick>>,
    callbacks: Cell<u64>,
    dirty: DirtySource,
}
define_class!(
    // NSObject has no subclass requirements; this delegate is only scheduled
    // on the main run loop. Its drawable slot is never shared with IO threads.
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = MetalState]
    struct MetalDelegate;
    unsafe impl NSObjectProtocol for MetalDelegate {}
    unsafe impl CAMetalDisplayLinkDelegate for MetalDelegate {
        #[unsafe(method(metalDisplayLink:needsUpdate:))]
        fn needs_update(&self, _link: &CAMetalDisplayLink, update: &CAMetalDisplayLinkUpdate) {
            let state = self.ivars();
            state.callbacks.set(state.callbacks.get() + 1);
            state.pending.replace(Some(Tick {
                target: update.targetTimestamp(),
                drawable: Some(update.drawable()),
            }));
            state.dirty.mark();
        }
    }
);
impl MetalDelegate {
    fn new(dirty: DirtySource, mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(MetalState {
            pending: RefCell::new(None),
            callbacks: Cell::new(0),
            dirty,
        });
        // SAFETY: NSObject's init signature is correct and ivars are installed.
        unsafe { msg_send![super(this), init] }
    }
}

// CoreVideo ABI from the installed SDK's CVBase.h / CVDisplayLink.h.
#[repr(C)]
struct Smpte {
    subframes: i16,
    subframe_divisor: i16,
    counter: u32,
    type_: u32,
    flags: u32,
    hours: i16,
    minutes: i16,
    seconds: i16,
    frames: i16,
}
#[repr(C)]
struct CvTimestamp {
    version: u32,
    scale: i32,
    video: i64,
    host: u64,
    rate: f64,
    period: i64,
    smpte: Smpte,
    flags: u64,
    reserved: u64,
}
type CvRef = *mut c_void;
type CvCallback = unsafe extern "C" fn(
    CvRef,
    *const CvTimestamp,
    *const CvTimestamp,
    u64,
    *mut u64,
    *mut c_void,
) -> i32;
#[link(name = "CoreVideo", kind = "framework")]
unsafe extern "C" {
    fn CVDisplayLinkCreateWithCGDisplay(display: u32, result: *mut CvRef) -> i32;
    fn CVDisplayLinkSetOutputCallback(
        link: CvRef,
        callback: CvCallback,
        context: *mut c_void,
    ) -> i32;
    fn CVDisplayLinkStart(link: CvRef) -> i32;
    fn CVDisplayLinkStop(link: CvRef) -> i32;
    fn CVDisplayLinkRelease(link: CvRef);
    fn CVGetHostClockFrequency() -> f64;
}
struct CvState {
    ticks: Latest<f64>,
    callbacks: AtomicU64,
    host_frequency: f64,
}
unsafe extern "C" fn cv_update(
    _link: CvRef,
    _now: *const CvTimestamp,
    output: *const CvTimestamp,
    _flags: u64,
    flags_out: *mut u64,
    context: *mut c_void,
) -> i32 {
    // SAFETY: CoreVideo owns these arguments during the callback. DisplayLink
    // stops/releases its CV object before freeing the boxed context.
    let state = unsafe { &*context.cast::<CvState>() };
    let output = unsafe { &*output };
    if !flags_out.is_null() {
        unsafe { *flags_out = 0 };
    }
    state.callbacks.fetch_add(1, Ordering::Relaxed);
    let seconds = if output.flags & 2 != 0 && state.host_frequency > 0. {
        output.host as f64 / state.host_frequency
    } else if output.flags & 1 != 0 && output.scale > 0 {
        output.video as f64 / f64::from(output.scale)
    } else {
        return 0;
    };
    state.ticks.publish(seconds);
    0
}

enum Backend {
    Fifo,
    Metal {
        link: Retained<CAMetalDisplayLink>,
        delegate: Retained<MetalDelegate>,
    },
    Cv {
        link: CvRef,
        context: Box<CvState>,
    },
}
pub struct DisplayLink {
    backend: Backend,
    active: Cell<bool>,
}
pub fn maximum_hz(monitor: &winit::monitor::MonitorHandle) -> f64 {
    // SAFETY: winit retains this monitor's NSScreen. Read it on main while the
    // monitor handle remains alive; the pointer never leaves this call.
    let screen = monitor
        .ns_screen()
        .and_then(|ptr| unsafe { ptr.cast::<NSScreen>().as_ref() });
    screen
        .filter(|_| available!(macos = 12.0))
        .map(|screen| screen.maximumFramesPerSecond() as f64)
        .filter(|hz| *hz > 0.)
        .unwrap_or_else(|| f64::from(monitor.refresh_rate_millihertz().unwrap_or(60_000)) / 1000.)
}
impl DisplayLink {
    pub fn new(
        surface: &wgpu::Surface<'static>,
        display: u32,
        maximum_hz: f64,
        dirty: DirtySource,
        mode: Mode,
    ) -> Result<Self, String> {
        if mode == Mode::Fifo {
            return Ok(Self {
                backend: Backend::Fifo,
                active: Cell::new(false),
            });
        }
        if mode == Mode::Metal && available!(macos = 14.0) {
            // SAFETY: only inspect this surface's Metal layer; retain it under
            // try_lock, then release both the lock and HAL guard before ObjC.
            let layer = {
                let hal = unsafe { surface.as_hal::<wgpu::hal::api::Metal>() }
                    .ok_or("surface is not Metal")?;
                let layer = hal.render_layer().try_lock().ok_or("Metal layer busy")?;
                layer.clone()
            };
            layer.setMaximumDrawableCount(3);
            layer.setDisplaySyncEnabled(true);
            let delegate = MetalDelegate::new(
                dirty,
                MainThreadMarker::new().ok_or("display link requires main thread")?,
            );
            let link = CAMetalDisplayLink::initWithMetalLayer(CAMetalDisplayLink::alloc(), &layer);
            link.setPaused(true);
            link.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
            link.setPreferredFrameLatency(2.);
            let maximum = maximum_hz as f32;
            link.setPreferredFrameRateRange(CAFrameRateRange::new(
                80_f32.min(maximum),
                maximum,
                maximum,
            ));
            // SAFETY: this main-thread-only delegate is registered on main's
            // common modes, including window tracking. Drop invalidates it.
            unsafe { link.addToRunLoop_forMode(&NSRunLoop::mainRunLoop(), NSRunLoopCommonModes) };
            return Ok(Self {
                backend: Backend::Metal { link, delegate },
                active: Cell::new(false),
            });
        }
        let mut link = ptr::null_mut();
        // SAFETY: display is a live CGDirectDisplayID from winit; output points
        // to a local CF reference slot and the result is owned on success.
        let status = unsafe { CVDisplayLinkCreateWithCGDisplay(display, &mut link) };
        if status != 0 || link.is_null() {
            return Err(format!("CVDisplayLinkCreate: {status}"));
        }
        let mut context = Box::new(CvState {
            ticks: Latest::new(dirty),
            callbacks: AtomicU64::new(0),
            host_frequency: unsafe { CVGetHostClockFrequency() },
        });
        // SAFETY: boxed allocation remains stable through moves; its fields are
        // thread-safe and the callback is stopped/released before context drop.
        let status = unsafe {
            CVDisplayLinkSetOutputCallback(link, cv_update, ptr::from_mut(&mut *context).cast())
        };
        if status != 0 {
            unsafe { CVDisplayLinkRelease(link) };
            return Err(format!("CVDisplayLink callback: {status}"));
        }
        Ok(Self {
            backend: Backend::Cv { link, context },
            active: Cell::new(false),
        })
    }
    pub fn name(&self) -> &'static str {
        match self.backend {
            Backend::Fifo => "fifo",
            Backend::Metal { .. } => "CAMetalDisplayLink",
            Backend::Cv { .. } => "CVDisplayLink",
        }
    }
    pub fn is_fifo(&self) -> bool {
        matches!(self.backend, Backend::Fifo)
    }
    pub fn is_metal(&self) -> bool {
        matches!(self.backend, Backend::Metal { .. })
    }
    /// Inspect demand without starting the native display link.
    pub fn is_active(&self) -> bool {
        self.active.get()
    }
    /// One-shot invalidations also use the display link's drawable. No code
    /// calls CAMetalLayer.nextDrawable once its Metal display link is attached.
    pub fn request_frame(&self) {
        if let Backend::Metal { link, .. } = &self.backend
            && !self.active.get()
        {
            link.setPaused(false);
            self.active.set(true);
        }
    }
    pub fn callbacks(&self) -> u64 {
        match &self.backend {
            Backend::Fifo => 0,
            Backend::Metal { delegate, .. } => delegate.ivars().callbacks.get(),
            Backend::Cv { context, .. } => context.callbacks.load(Ordering::Relaxed),
        }
    }
    pub fn clear(&self) {
        let _ = self.take_tick();
    }
    pub fn take_tick(&self) -> Option<Tick> {
        match &self.backend {
            Backend::Fifo => None,
            Backend::Metal { delegate, .. } => delegate.ivars().pending.borrow_mut().take(),
            Backend::Cv { context, .. } => context.ticks.take().map(|target| Tick {
                target,
                drawable: None,
            }),
        }
    }
    pub fn set_active(&self, active: bool) -> Result<(), String> {
        if active == self.active.get() {
            return Ok(());
        }
        match &self.backend {
            Backend::Fifo => {}
            Backend::Metal { link, .. } => link.setPaused(!active),
            Backend::Cv { link, .. } => {
                // SAFETY: owned valid CV reference; called outside its callback.
                let status = unsafe {
                    if active {
                        CVDisplayLinkStart(*link)
                    } else {
                        CVDisplayLinkStop(*link)
                    }
                };
                if status != 0 {
                    return Err(format!("CVDisplayLink active={active}: {status}"));
                }
            }
        }
        self.active.set(active);
        if !active {
            self.clear();
        }
        Ok(())
    }
}
impl Drop for DisplayLink {
    fn drop(&mut self) {
        let stopped = self.set_active(false).is_ok();
        match std::mem::replace(&mut self.backend, Backend::Fifo) {
            Backend::Fifo => {}
            Backend::Metal { link, .. } => {
                link.invalidate();
                link.setDelegate(None);
            }
            // SAFETY: stop has completed; release the retained CV reference
            // before Rust drops the callback's boxed context.
            Backend::Cv { link, context } => {
                unsafe { CVDisplayLinkRelease(link) };
                // A failed stop must never free memory a callback might still
                // access. Leak only this exceptional shutdown allocation.
                if !stopped {
                    Box::leak(context);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tick_wake_after_pause_does_not_reopen_link() {
        // draw has called set_active(false); its already-queued tick Wake is
        // delivered afterwards, with no new model or input invalidation.
        assert_eq!(wake_action(&[TICK_SOURCE], false), WakeAction::Ignore);
    }
    #[test]
    fn data_racing_a_tick_still_requests_a_frame_after_pause() {
        assert_eq!(
            wake_action(&[TICK_SOURCE, 5], false),
            WakeAction::Invalidate
        );
        assert_eq!(wake_action(&[5], false), WakeAction::Invalidate);
        assert_eq!(wake_action(&[TICK_SOURCE], true), WakeAction::Tick);
        assert_eq!(wake_action(&[], true), WakeAction::Ignore);
        // Unknown sources remain real invalidations; telemetry never gates work.
        assert_eq!(wake_action(&[999], false), WakeAction::Invalidate);
    }
    #[test]
    fn clock_uses_target_time_and_excludes_paused_time() {
        let mut clock = Clock::default();
        assert_eq!(clock.observe(100.), Some(0.));
        assert!((clock.observe(100.016).unwrap() - 16.).abs() < 0.001);
        assert!(clock.observe(99.).is_none());
        assert!(clock.observe(f64::NAN).is_none());
        clock.pause();
        assert!((clock.observe(200.).unwrap() - 16.).abs() < 0.001);
        assert!((clock.observe(200.032).unwrap() - 48.).abs() < 0.001);
    }
    #[test]
    fn core_video_timestamp_matches_sdk_abi() {
        assert_eq!(std::mem::size_of::<Smpte>(), 24);
        assert_eq!(std::mem::size_of::<CvTimestamp>(), 80);
        assert_eq!(std::mem::offset_of!(CvTimestamp, host), 16);
        assert_eq!(std::mem::offset_of!(CvTimestamp, flags), 64);
    }
}
