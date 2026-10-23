//! R2 app shell: event-driven native scene, memory-only UI drain and hitch meter.
mod assets;
mod budget;
mod capture;
mod display_link;
mod gpu;
mod live;
mod log;
mod scene;

use bbt_rt::{HitchMeter, Latest, Phases, UiInbox, UiMsg};
use display_link::{Clock, DisplayLink, Mode};
use std::sync::Arc;
use std::time::{Duration, Instant};
use winit::platform::macos::MonitorHandleExtMacOS;
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{ElementState, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy},
    keyboard::{KeyCode, ModifiersState, PhysicalKey},
    window::{Window, WindowId},
};

#[derive(Debug)]
pub enum UiEvent {
    Wake,
    DemoFinished,
    BeginIdle,
    EndIdle,
    EndAnimation,
    Quit,
    BackendStopped(Result<(), String>),
    SmokeDeadline,
}
struct Wake(EventLoopProxy<UiEvent>);
impl bbt_rt::Waker for Wake {
    fn wake(&self) {
        let _ = self.0.send_event(UiEvent::Wake);
    }
}
type Prepared = Result<(gpu::Gpu, scene::Scene), String>;

#[derive(Default)]
struct IdleQuiescence {
    observed: Option<(u64, u64)>,
}
impl IdleQuiescence {
    fn ready(&mut self, producer_finished: bool, pending: bool, counts: (u64, u64)) -> bool {
        if !producer_finished || pending {
            self.observed = None;
            return false;
        }
        let quiet = self.observed == Some(counts);
        self.observed = Some(counts);
        quiet
    }
}

struct App {
    proxy: EventLoopProxy<UiEvent>,
    inbox: UiInbox,
    logger: log::Logger,
    prepared: Arc<Latest<Prepared>>,
    updates: Arc<Latest<usize>>,
    live_updates: Arc<Latest<Arc<bboxy450_term::model::Model>>>,
    live_state: Option<Arc<bboxy450_term::model::Model>>,
    live_commands: Option<tokio::sync::mpsc::Sender<live::UiMsg>>,
    render_context: Option<tokio::sync::oneshot::Sender<live::RenderContext>>,
    closing: bool,
    smoke_bootstrap: Option<tokio::sync::oneshot::Sender<()>>,
    smoke_wake_seen: bool,
    window: Option<Arc<Window>>,
    pacer: Option<DisplayLink>,
    pacing: Mode,
    clock: Clock,
    occluded: bool,
    maximum_hz: f64,
    budget: budget::Budget,
    gpu: Option<gpu::Gpu>,
    scene: scene::Scene,
    meter: HitchMeter,
    modifiers: ModifiersState,
    overlay: bool,
    fake: bool,
    smoke: bool,
    demo_started: bool,
    demo_finished: bool,
    idle_scheduled: bool,
    idle_quiet: IdleQuiescence,
    redraw_pending: std::cell::Cell<bool>,
    last_update: usize,
    idle_start: Option<(Instant, u64, u64)>,
    error: Option<String>,
    capture: Option<std::path::PathBuf>,
    animate: bool,
    cadence_test: bool,
    animation_start: Option<(Instant, u64)>,
    frozen_ms: f32,
}
impl App {
    fn request_close(&mut self, event_loop: &ActiveEventLoop) {
        if self.closing {
            return;
        }
        self.closing = true;
        self.logger.write("app close requested; stopping IO owner");
        if self.live_commands.take().is_none() {
            event_loop.exit();
        }
        // The IO owner reaps both children, then sends BackendStopped. Never
        // wait for process shutdown on this thread.
    }
    fn request_redraw(&self) {
        if !self.occluded
            && let Some(pacer) = &self.pacer
        {
            pacer.request_frame();
        }
        if let Some(window) = &self.window {
            self.redraw_pending.set(true);
            window.request_redraw();
        }
    }
    fn schedule_idle_check(&self) {
        let proxy = self.proxy.clone();
        bbt_rt::handle().spawn(async move {
            tokio::time::sleep(Duration::from_secs(1)).await;
            let _ = proxy.send_event(UiEvent::BeginIdle);
        });
    }
    fn fail(&mut self, error: String, event_loop: &ActiveEventLoop) {
        self.logger.write(format!("app error: {error}"));
        self.error = Some(error);
        event_loop.exit();
    }
    fn init(&mut self, event_loop: &ActiveEventLoop) -> Result<(), String> {
        let window = Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_title("bboxy450-term")
                        .with_inner_size(LogicalSize::new(1600., 960.))
                        .with_min_inner_size(LogicalSize::new(800., 600.))
                        .with_decorations(false),
                )
                .map_err(|e| e.to_string())?,
        );
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::METAL,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let surface = instance
            .create_surface(window.clone())
            .map_err(|e| e.to_string())?;
        let size = window.inner_size();
        if let Some(monitor) = window.current_monitor() {
            self.maximum_hz = display_link::maximum_hz(&monitor);
            self.meter = HitchMeter::new(self.maximum_hz);
            let max_hz = monitor
                .video_modes()
                .map(|mode| mode.refresh_rate_millihertz())
                .max()
                .unwrap_or(0);
            self.logger.write(format!(
                "panel: {:?}, current={}mHz max_mode={}mHz maximumFramesPerSecond={}",
                monitor.name(),
                monitor.refresh_rate_millihertz().unwrap_or(0),
                max_hz,
                self.maximum_hz
            ));
        }
        let prepared = self.prepared.clone();
        let scale = window.scale_factor() as f32;
        let effects = self.animate;
        let fixture = self.fake || self.animate;
        let render_context = self.render_context.take();
        let bootstrap = if self.smoke {
            let (open, wait) = tokio::sync::oneshot::channel();
            self.smoke_bootstrap = Some(open);
            Some(wait)
        } else {
            None
        };
        bbt_rt::handle().spawn(async move {
            if let Some(wait) = bootstrap {
                let _ = wait.await;
            }
            let result = async {
                let mut gpu = gpu::Gpu::new(instance, surface, [size.width, size.height]).await?;
                if let Some(sender) = render_context {
                    let _ = sender.send(live::RenderContext {
                        context: gpu.shared_context(),
                        scale,
                    });
                }
                let scene = if fixture {
                    scene::Scene::new()
                } else {
                    scene::Scene::live()
                };
                let logical = [size.width as f32 / scale, size.height as f32 / scale];
                let layouts = scene.layout(logical, effects)?;
                gpu.warm(scene.paint(&layouts, logical, None), scale)?;
                Ok((gpu, scene))
            }
            .await;
            prepared.publish(result);
        });
        self.window = Some(window);
        if self.smoke {
            // Force an early producer wake before GPU bootstrap. This catches
            // the live-seat startup race in the real native CI probe.
            self.inbox.dirty.source(5).mark();
        }
        Ok(())
    }
    fn install_gpu(&mut self) -> Result<(), String> {
        if let Some(result) = self.prepared.take() {
            let (mut gpu, scene) = result?;
            self.scene = scene;
            if let Some(state) = &self.live_state {
                self.scene.set_model(state);
            }
            let size = self.window.as_ref().unwrap().inner_size();
            gpu.resize(size.width, size.height);
            self.logger.write(format!(
                "renderer ready: {}, Fifo, frame latency 2, scene seats={}",
                gpu.adapter_name(),
                self.scene.seats.len()
            ));
            self.logger.write(format!(
                "Metal maximumDrawableCount={:?}, configured latency={}",
                gpu.maximum_drawables(),
                gpu.config.desired_maximum_frame_latency
            ));
            let display = self
                .window
                .as_ref()
                .unwrap()
                .current_monitor()
                .map_or(0, |monitor| monitor.native_id());
            let pacer = DisplayLink::new(
                &gpu.surface,
                display,
                self.maximum_hz,
                self.inbox.dirty.source(4),
                self.pacing,
            )?;
            self.logger.write(format!(
                "pacing: {}, requested range={}/{}/{} Hz; initially paused",
                pacer.name(),
                80_f64.min(self.maximum_hz),
                self.maximum_hz,
                self.maximum_hz
            ));
            self.pacer = Some(pacer);
            self.gpu = Some(gpu);
            self.request_redraw();
        }
        Ok(())
    }
    fn drain(&mut self) {
        self.inbox.dirty.drain();
        let start = Instant::now();
        while start.elapsed() < Duration::from_millis(1) {
            let message = self.inbox.messages.try_recv();
            let output = self.inbox.output.pop();
            if message.is_none() && output.is_none() {
                break;
            }
            // These runtime notices carry no process handles or socket IO.
            if let Some(UiMsg::Exited { task, status }) = message {
                self.logger
                    .write(format!("task {} exited {:?}", task.id(), status.code));
            }
        }
        if let Some(sequence) = self.updates.take() {
            self.scene.update(sequence);
            self.last_update = sequence;
        }
        if let Some(state) = self.live_updates.take() {
            self.scene.set_model(&state);
            self.live_state = Some(state);
        }
    }
    fn demo(&mut self) {
        if !self.fake || self.demo_started {
            return;
        }
        self.demo_started = true;
        let updates = self.updates.clone();
        let sender = self.inbox.sender.clone();
        let proxy = self.proxy.clone();
        bbt_rt::handle().spawn(async move {
            tokio::time::sleep(Duration::from_millis(300)).await;
            for sequence in 1..=20 {
                updates.publish(sequence);
                let _ = sender.try_send(UiMsg::Changed { source: 3 });
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
            // Observing update20 alone is insufficient: its Changed notice
            // may not have been posted yet if the producer was descheduled.
            let _ = proxy.send_event(UiEvent::DemoFinished);
        });
    }
    fn redraw(&mut self) -> Result<(), String> {
        self.redraw_pending.set(false);
        if self.gpu.is_none() {
            // A producer may wake us before GPU preparation finishes. Drain
            // admission even on an empty frame, or GPU-ready can lose its wake.
            self.inbox.dirty.drain();
            if self.smoke_wake_seen
                && let Some(open) = self.smoke_bootstrap.take()
            {
                self.logger
                    .write("smoke bootstrap: early producer handled before GPU ready");
                let _ = open.send(());
            }
            self.install_gpu()?;
            if self.gpu.is_none() {
                return Ok(());
            }
        }
        let frame_cpu = Instant::now();
        let size = self.window.as_ref().unwrap().inner_size();
        if size.width == 0 || size.height == 0 {
            return Ok(());
        }
        let mut phases = Phases::default();
        let start = Instant::now();
        self.drain();
        // Drain the coalesced dirty flag before consuming its latest tick, so a
        // concurrent CV callback cannot publish a tick without a future wake.
        let tick = self.pacer.as_ref().and_then(DisplayLink::take_tick);
        let fifo = self.pacer.as_ref().is_none_or(DisplayLink::is_fifo);
        let metal = self.pacer.as_ref().is_some_and(DisplayLink::is_metal);
        if self.occluded
            || (metal && tick.is_none())
            || (self.animate && self.animation_start.is_some() && !fifo && tick.is_none())
        {
            return Ok(());
        }
        phases.drain = start.elapsed();
        let scale = self.window.as_ref().unwrap().scale_factor() as f32;
        let logical = [size.width as f32 / scale, size.height as f32 / scale];
        let start = Instant::now();
        let layouts = self
            .scene
            .layout(logical, self.animation_start.is_some() || self.animate)?;
        phases.layout = start.elapsed();
        let overlay = self.overlay.then(|| {
            let sample = self.meter.samples().back();
            let last = sample.map_or(Phases::default(), |s| s.phases);
            let interval = sample.and_then(|s| s.interval).unwrap_or_default();
            let ms = |duration: Duration| duration.as_secs_f64() * 1000.;
            format!(
                "frames {} · {:.1} Hz · interval {:.2}ms · CPU p99 {:.2}ms\ndrain {:.2} layout {:.2} draw {:.2}ms\nencode {:.2} present {:.2}ms · update {}",
                self.meter.total_frames(), if interval.is_zero(){0.}else{1./interval.as_secs_f64()},ms(interval), ms(self.meter.cpu_percentile(0.99)),
                ms(last.drain), ms(last.layout), ms(last.draw), ms(last.encode), ms(last.present_wait), self.last_update,
            )
        });
        let start = Instant::now();
        let ops = self.scene.paint(&layouts, logical, overlay.as_deref());
        phases.draw = start.elapsed();
        if self.animate {
            if fifo {
                self.frozen_ms = self
                    .animation_start
                    .map_or(0., |(start, _)| start.elapsed().as_secs_f32() * 1000.);
            } else if let Some(time) = tick
                .as_ref()
                .and_then(|tick| self.clock.observe(tick.target))
            {
                self.frozen_ms = time;
            }
        }
        if let Some((encode, wait)) = self.gpu.as_mut().unwrap().draw(
            ops,
            scale,
            self.frozen_ms,
            tick.and_then(|tick| tick.drawable),
            self.meter.total_frames() + 1,
        )? {
            phases.encode = encode;
            phases.present_wait = wait;
            if let Some((phase, time)) = self.meter.record(Instant::now(), phases) {
                self.logger.write(format!("hitch frame={} phase={phase} {:.3}ms cpu={:.3}ms drain={:.3} layout={:.3} draw={:.3} encode={:.3} present_wait={:.3}",self.meter.total_frames(),time.as_secs_f64()*1000.,phases.cpu().as_secs_f64()*1000.,phases.drain.as_secs_f64()*1000.,phases.layout.as_secs_f64()*1000.,phases.draw.as_secs_f64()*1000.,phases.encode.as_secs_f64()*1000.,phases.present_wait.as_secs_f64()*1000.));
            }
            if self.animation_start.is_some() && self.animate {
                // The Metal path never waits for acquisition: include its full
                // redraw wall time, including overlay/import/present overhead.
                let cpu =
                    frame_cpu
                        .elapsed()
                        .saturating_sub(if metal { Duration::ZERO } else { wait });
                self.budget.cpu(self.meter.total_frames(), cpu);
            }
            self.gpu.as_ref().unwrap().collect_gpu(&mut self.budget);
            if !self.animate {
                self.pacer.as_ref().unwrap().set_active(false)?;
            }
            if self.meter.total_frames() == 1 {
                self.logger.write(format!(
                    "first frame: native scene presented, seats={}",
                    self.scene.seats.len()
                ));
                self.demo();
                if self.animate {
                    self.animation_start = Some((Instant::now(), self.meter.total_frames()));
                    self.pacer.as_mut().unwrap().set_active(!self.occluded)?;
                    if self.cadence_test {
                        let proxy = self.proxy.clone();
                        bbt_rt::handle().spawn(async move {
                            tokio::time::sleep(Duration::from_secs(3)).await;
                            let _ = proxy.send_event(UiEvent::EndAnimation);
                        });
                    }
                }
            }
            if (!self.fake || self.last_update == 20)
                && let Some(path) = self.capture.take()
            {
                self.gpu
                    .as_ref()
                    .unwrap()
                    .capture(path, self.logger.clone());
            }
            if self.fake && self.last_update == 20 && !self.idle_scheduled {
                self.idle_scheduled = true;
                self.schedule_idle_check();
            }
        }
        if (self.animate && fifo)
            || !self.inbox.messages.is_empty()
            || !self.inbox.output.is_empty()
        {
            self.request_redraw();
        }
        Ok(())
    }
}
impl ApplicationHandler<UiEvent> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        event_loop.set_control_flow(ControlFlow::Wait);
        if self.window.is_none()
            && let Err(error) = self.init(event_loop)
        {
            self.fail(error, event_loop);
        }
    }
    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: UiEvent) {
        match event {
            UiEvent::DemoFinished => self.demo_finished = true,
            UiEvent::Wake => {
                if self.smoke && self.gpu.is_none() {
                    self.smoke_wake_seen = true;
                }
                if let Err(error) = self.install_gpu() {
                    self.fail(error, event_loop);
                    return;
                }
                self.request_redraw();
            }
            UiEvent::BeginIdle => {
                let counts = (
                    self.meter.total_frames(),
                    self.pacer.as_ref().map_or(0, DisplayLink::callbacks),
                );
                let queued = !self.inbox.messages.is_empty() || !self.inbox.output.is_empty();
                if self.fake
                    && !self.idle_quiet.ready(
                        self.demo_finished && self.last_update == 20,
                        queued || self.redraw_pending.get(),
                        counts,
                    )
                {
                    self.logger.write(format!(
                        "idle probe settling: producer_done={} queued={} redraw_pending={} frames={} callbacks={}",
                        self.demo_finished, queued, self.redraw_pending.get(), counts.0, counts.1,
                    ));
                    if queued {
                        self.request_redraw();
                    }
                    self.schedule_idle_check();
                    return;
                }
                if self.cadence_test {
                    self.gpu.as_ref().unwrap().collect_gpu(&mut self.budget);
                    self.logger.write(self.budget.report());
                }
                self.idle_start = Some((
                    Instant::now(),
                    self.meter.total_frames(),
                    self.pacer.as_ref().map_or(0, DisplayLink::callbacks),
                ));
                self.logger.write(format!(
                    "idle probe begins: frames={} last_update={}",
                    self.meter.total_frames(),
                    self.last_update
                ));
                let proxy = self.proxy.clone();
                bbt_rt::handle().spawn(async move {
                    tokio::time::sleep(Duration::from_secs(10)).await;
                    let _ = proxy.send_event(UiEvent::EndIdle);
                });
            }
            UiEvent::EndAnimation => {
                self.animate = false;
                self.clock.pause();
                let pacer = self.pacer.as_mut().expect("pacer installed");
                if let Err(error) = pacer.set_active(false) {
                    self.fail(error, event_loop);
                    return;
                }
                let (start, frames) = self.animation_start.expect("animation started");
                let seconds = start.elapsed().as_secs_f64();
                let count = self.meter.total_frames() - frames;
                self.logger.write(format!("cadence {}: frames={count} callbacks={} seconds={seconds:.6} hz={:.3} CPU_p50={:.3}ms CPU_p99={:.3}ms",pacer.name(),pacer.callbacks(), count as f64 / seconds, self.meter.cpu_percentile(0.5).as_secs_f64()*1000., self.meter.cpu_percentile(0.99).as_secs_f64()*1000.));
                let mut intervals: Vec<_> = self
                    .meter
                    .samples()
                    .iter()
                    .skip(2)
                    .filter_map(|s| s.interval)
                    .collect();
                intervals.sort_unstable();
                let dropped = intervals
                    .iter()
                    .filter(|time| time.as_secs_f64() > 1.5 / self.maximum_hz)
                    .count();
                self.logger.write(format!("cadence intervals (initial tick excluded): samples={} p50={:.3}ms p99={:.3}ms max={:.3}ms late_intervals={dropped} (>1.5 panel intervals)",intervals.len(),intervals.get(intervals.len()/2).copied().unwrap_or_default().as_secs_f64()*1000.,intervals.get((intervals.len()*99/100).min(intervals.len().saturating_sub(1))).copied().unwrap_or_default().as_secs_f64()*1000.,intervals.last().copied().unwrap_or_default().as_secs_f64()*1000.));
                let proxy = self.proxy.clone();
                bbt_rt::handle().spawn(async move {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    let _ = proxy.send_event(UiEvent::BeginIdle);
                });
            }
            UiEvent::EndIdle => {
                let (start, frames, callbacks) =
                    self.idle_start.take().expect("idle probe started");
                let delta = self.meter.total_frames() - frames;
                let callbacks = self.pacer.as_ref().map_or(0, DisplayLink::callbacks) - callbacks;
                let passed =
                    delta == 0 && callbacks == 0 && (self.cadence_test || self.last_update == 20);
                // Reporting patch supplied by Nyx: retain fast warm encode
                // values too, rather than relying on over-budget hitch logs.
                let mut encode: Vec<_> = self
                    .meter
                    .samples()
                    .iter()
                    .skip(1)
                    .map(|s| s.phases.encode)
                    .collect();
                encode.sort_unstable();
                let encode_percentile = |p: f64| {
                    encode
                        .get(((encode.len() as f64 * p).ceil() as usize).saturating_sub(1))
                        .copied()
                        .unwrap_or_default()
                        .as_secs_f64()
                        * 1000.
                };
                let text = format!(
                    "idle probe {}: idle {:.3}s frames={} callbacks={} updates={} CPU p50={:.3}ms p99={:.3}ms samples={} warm_encode_p50={:.3}ms warm_encode_p99={:.3}ms",
                    if passed { "PASS" } else { "FAIL" },
                    start.elapsed().as_secs_f64(),
                    delta,
                    callbacks,
                    self.last_update,
                    self.meter.cpu_percentile(0.5).as_secs_f64() * 1000.,
                    self.meter.cpu_percentile(0.99).as_secs_f64() * 1000.,
                    self.meter.samples().len(),
                    encode_percentile(0.5),
                    encode_percentile(0.99)
                );
                if !passed {
                    self.error = Some("R2 idle/update probe failed".into());
                }
                if self.smoke || self.cadence_test {
                    self.logger.finish(text, self.proxy.clone());
                } else {
                    self.logger.write(text);
                }
            }
            UiEvent::Quit => event_loop.exit(),
            UiEvent::SmokeDeadline => self.fail(
                "smoke probe did not finish within 30 seconds".into(),
                event_loop,
            ),
            UiEvent::BackendStopped(result) => match result {
                Ok(()) => event_loop.exit(),
                Err(error) => self.fail(error, event_loop),
            },
        }
    }
    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                self.request_close(event_loop);
            }
            WindowEvent::Resized(size) => {
                if let Some(pacer) = &self.pacer {
                    pacer.clear();
                }
                if let Some(gpu) = &mut self.gpu {
                    gpu.resize(size.width, size.height);
                }
                self.request_redraw();
            }
            WindowEvent::ScaleFactorChanged { .. } => self.request_redraw(),
            WindowEvent::Occluded(occluded) => {
                self.occluded = occluded;
                if let Some(pacer) = &mut self.pacer
                    && let Err(error) = pacer.set_active(self.animate && !occluded)
                {
                    self.fail(error, event_loop);
                    return;
                }
                self.clock.pause();
                if !occluded {
                    self.request_redraw();
                }
            }
            WindowEvent::ModifiersChanged(modifiers) => self.modifiers = modifiers.state(),
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                if event.physical_key == PhysicalKey::Code(KeyCode::KeyQ)
                    && self.modifiers.super_key()
                    && !event.repeat
                {
                    self.request_close(event_loop);
                    return;
                }
                if event.physical_key == PhysicalKey::Code(KeyCode::KeyT)
                    && self.modifiers.super_key()
                    && !self.modifiers.alt_key()
                    && !self.modifiers.shift_key()
                    && !event.repeat
                    && let Some(commands) = &self.live_commands
                    && let Err(error) = commands.try_send(live::UiMsg::SpawnZsh)
                {
                    self.logger
                        .write(format!("new seat request rejected: {error}"));
                }
                if event.physical_key == PhysicalKey::Code(KeyCode::KeyF)
                    && self.modifiers.super_key()
                    && self.modifiers.alt_key()
                {
                    self.overlay = !self.overlay;
                    self.logger.write(format!("hitch overlay={}", self.overlay));
                    self.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                if let Err(error) = self.redraw() {
                    self.fail(error, event_loop);
                }
            }
            _ => {}
        }
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let hook_args: Vec<_> = std::env::args().skip(1).collect();
    if let Some(result) = bboxy450_term::cockpit::private_entry(&hook_args) {
        return result.map_err(|error| error as Box<dyn std::error::Error>);
    }
    if hook_args.first().is_some_and(|arg| arg == "hook") {
        bbt_mux::adapters::hook::run_cli(&hook_args[1..]);
        return Ok(());
    }
    if hook_args.first().is_some_and(|arg| arg == "mcp") {
        bbt_mux::mcp::run_cli()?;
        return Ok(());
    }
    let mut event_loop_builder = EventLoop::<UiEvent>::with_user_event();
    #[cfg(target_os = "macos")]
    {
        use winit::platform::macos::EventLoopBuilderExtMacOS;
        // The default Quit menu sends NSApplication::terminate before our
        // async child shutdown can run. Route Command-Q through App instead.
        event_loop_builder.with_default_menu(false);
    }
    let event_loop = event_loop_builder.build()?;
    let proxy = event_loop.create_proxy();
    let inbox = bbt_rt::init_ui(Wake(proxy.clone()));
    let prepared = Arc::new(Latest::new(inbox.dirty.source(2)));
    let updates = Arc::new(Latest::new(inbox.dirty.source(3)));
    let live_updates = Arc::new(Latest::new(inbox.dirty.source(5)));
    let smoke = std::env::args().any(|arg| arg == "--smoke");
    if smoke {
        let timeout_proxy = proxy.clone();
        bbt_rt::handle().spawn(async move {
            tokio::time::sleep(Duration::from_secs(30)).await;
            let _ = timeout_proxy.send_event(UiEvent::SmokeDeadline);
        });
    }
    let fake = smoke || std::env::args().any(|arg| arg == "--fake-source");
    let overlay = std::env::args().any(|arg| arg == "--meter");
    let args: Vec<_> = std::env::args().collect();
    let cadence_test = args.iter().any(|arg| arg == "--cadence-test");
    let animate = cadence_test || args.iter().any(|arg| arg == "--animate");
    if cadence_test && fake {
        return Err("--cadence-test and fake-source/smoke are separate probes".into());
    }
    let pacing = Mode::parse(
        args.windows(2)
            .find(|pair| pair[0] == "--pacing")
            .map_or("metal", |pair| pair[1].as_str()),
    )?;
    let capture = args
        .windows(2)
        .find(|pair| pair[0] == "--capture")
        .map(|pair| std::path::PathBuf::from(&pair[1]));
    let logger = log::Logger::new();
    let (render_context, rendering) = tokio::sync::oneshot::channel();
    let live_commands = if fake || animate {
        None
    } else {
        Some(live::start(
            live_updates.clone(),
            proxy.clone(),
            logger.clone(),
            args.windows(2)
                .find(|pair| pair[0] == "--run-dir")
                .map(|pair| std::path::PathBuf::from(&pair[1])),
            rendering,
        ))
    };
    let mut app = App {
        proxy,
        inbox,
        logger,
        prepared,
        updates,
        live_updates,
        live_state: None,
        live_commands,
        render_context: (!fake && !animate).then_some(render_context),
        closing: false,
        smoke_bootstrap: None,
        smoke_wake_seen: false,
        window: None,
        pacer: None,
        pacing,
        clock: Clock::default(),
        occluded: false,
        maximum_hz: 60.,
        budget: budget::Budget::default(),
        gpu: None,
        scene: if fake || animate {
            scene::Scene::new()
        } else {
            scene::Scene::live()
        },
        meter: HitchMeter::new(120.),
        modifiers: ModifiersState::empty(),
        overlay,
        fake,
        smoke,
        demo_started: false,
        demo_finished: false,
        idle_scheduled: false,
        idle_quiet: IdleQuiescence::default(),
        redraw_pending: std::cell::Cell::new(false),
        last_update: 0,
        idle_start: None,
        error: None,
        capture,
        animate,
        cadence_test,
        animation_start: None,
        frozen_ms: 450.,
    };
    event_loop.run_app(&mut app)?;
    if let Some(error) = app.error {
        return Err(error.into());
    }
    Ok(())
}

#[cfg(test)]
mod smoke_tests {
    use super::IdleQuiescence;

    #[test]
    fn idle_waits_for_producer_queued_work_and_late_frames_before_starting() {
        let mut quiet = IdleQuiescence::default();
        // The final value was rendered, but its producer can still be paused
        // before posting the matching Changed notice. A timer is not a barrier.
        assert!(!quiet.ready(false, false, (21, 21)));
        assert!(!quiet.ready(false, false, (21, 21)));
        assert!(!quiet.ready(true, true, (21, 21)));
        // Draining the notice can leave a native redraw and then a callback.
        assert!(!quiet.ready(true, false, (22, 22)));
        assert!(!quiet.ready(true, false, (23, 22)));
        assert!(!quiet.ready(true, false, (23, 23)));
        // Pending work invalidates an earlier observation even if it has not
        // changed either counter yet. Only a full quiet interval may start idle.
        assert!(!quiet.ready(true, true, (23, 23)));
        assert!(!quiet.ready(true, false, (23, 23)));
        assert!(quiet.ready(true, false, (23, 23)));
    }
}
