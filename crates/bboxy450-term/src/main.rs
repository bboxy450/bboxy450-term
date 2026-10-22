//! R2 app shell: event-driven native scene, memory-only UI drain and hitch meter.
mod assets;
mod capture;
mod gpu;
mod log;
mod scene;

use bbt_rt::{HitchMeter, Latest, Phases, UiInbox, UiMsg};
use std::sync::Arc;
use std::time::{Duration, Instant};
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
    BeginIdle,
    EndIdle,
    Quit,
}
struct Wake(EventLoopProxy<UiEvent>);
impl bbt_rt::Waker for Wake {
    fn wake(&self) {
        let _ = self.0.send_event(UiEvent::Wake);
    }
}
type Prepared = Result<(gpu::Gpu, scene::Scene), String>;

struct App {
    proxy: EventLoopProxy<UiEvent>,
    inbox: UiInbox,
    logger: log::Logger,
    prepared: Arc<Latest<Prepared>>,
    updates: Arc<Latest<usize>>,
    window: Option<Arc<Window>>,
    gpu: Option<gpu::Gpu>,
    scene: scene::Scene,
    meter: HitchMeter,
    modifiers: ModifiersState,
    overlay: bool,
    fake: bool,
    smoke: bool,
    demo_started: bool,
    idle_scheduled: bool,
    last_update: usize,
    idle_start: Option<(Instant, u64)>,
    error: Option<String>,
    capture: Option<std::path::PathBuf>,
}
impl App {
    fn request_redraw(&self) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
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
        let prepared = self.prepared.clone();
        let scale = window.scale_factor() as f32;
        bbt_rt::handle().spawn(async move {
            let result = async {
                let mut gpu = gpu::Gpu::new(instance, surface, [size.width, size.height]).await?;
                let scene = scene::Scene::new();
                let logical = [size.width as f32 / scale, size.height as f32 / scale];
                let layouts = scene.layout(logical)?;
                gpu.warm(scene.paint(&layouts, logical, None), scale)?;
                Ok((gpu, scene))
            }
            .await;
            prepared.publish(result);
        });
        self.window = Some(window);
        Ok(())
    }
    fn install_gpu(&mut self) -> Result<(), String> {
        if let Some(result) = self.prepared.take() {
            let (mut gpu, scene) = result?;
            self.scene = scene;
            let size = self.window.as_ref().unwrap().inner_size();
            gpu.resize(size.width, size.height);
            self.logger.write(format!(
                "renderer ready: {}, Fifo, frame latency 2, scene seats={}",
                gpu.adapter_name(),
                self.scene.seats.len()
            ));
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
            // R2 consumes only state notifications. Command views and terminal
            // parser buffers join later; no process is spawned by this fixture.
            if let Some(UiMsg::Exited { task, status }) = message {
                self.logger
                    .write(format!("task {} exited {:?}", task.id(), status.code));
            }
        }
        if let Some(sequence) = self.updates.take() {
            self.scene.update(sequence);
            self.last_update = sequence;
        }
    }
    fn demo(&mut self) {
        if !self.fake || self.demo_started {
            return;
        }
        self.demo_started = true;
        let updates = self.updates.clone();
        let sender = self.inbox.sender.clone();
        bbt_rt::handle().spawn(async move {
            tokio::time::sleep(Duration::from_millis(300)).await;
            for sequence in 1..=20 {
                updates.publish(sequence);
                let _ = sender.try_send(UiMsg::Changed { source: 3 });
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        });
    }
    fn redraw(&mut self) -> Result<(), String> {
        if self.gpu.is_none() {
            return Ok(());
        }
        let size = self.window.as_ref().unwrap().inner_size();
        if size.width == 0 || size.height == 0 {
            return Ok(());
        }
        let mut phases = Phases::default();
        let start = Instant::now();
        self.drain();
        phases.drain = start.elapsed();
        let scale = self.window.as_ref().unwrap().scale_factor() as f32;
        let logical = [size.width as f32 / scale, size.height as f32 / scale];
        let start = Instant::now();
        let layouts = self.scene.layout(logical)?;
        phases.layout = start.elapsed();
        let overlay = self.overlay.then(|| {
            let sample = self.meter.samples().back();
            let last = sample.map_or(Phases::default(), |s| s.phases);
            let interval = sample.and_then(|s| s.interval).unwrap_or_default();
            let ms = |duration: Duration| duration.as_secs_f64() * 1000.;
            format!(
                "frames {} · interval {:.2}ms · CPU p99 {:.2}ms\ndrain {:.2} layout {:.2} draw {:.2}ms\nencode {:.2} present {:.2}ms · update {}",
                self.meter.total_frames(), ms(interval), ms(self.meter.cpu_percentile(0.99)),
                ms(last.drain), ms(last.layout), ms(last.draw), ms(last.encode), ms(last.present_wait), self.last_update,
            )
        });
        let start = Instant::now();
        let ops = self.scene.paint(&layouts, logical, overlay.as_deref());
        phases.draw = start.elapsed();
        if let Some((encode, wait)) = self.gpu.as_mut().unwrap().draw(ops, scale)? {
            phases.encode = encode;
            phases.present_wait = wait;
            if let Some((phase, time)) = self.meter.record(Instant::now(), phases) {
                self.logger.write(format!("hitch frame={} phase={phase} {:.3}ms cpu={:.3}ms drain={:.3} layout={:.3} draw={:.3} encode={:.3} present_wait={:.3}",self.meter.total_frames(),time.as_secs_f64()*1000.,phases.cpu().as_secs_f64()*1000.,phases.drain.as_secs_f64()*1000.,phases.layout.as_secs_f64()*1000.,phases.draw.as_secs_f64()*1000.,phases.encode.as_secs_f64()*1000.,phases.present_wait.as_secs_f64()*1000.));
            }
            if self.meter.total_frames() == 1 {
                self.logger.write("first frame: 15 native panes presented");
                self.demo();
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
                let proxy = self.proxy.clone();
                bbt_rt::handle().spawn(async move {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    let _ = proxy.send_event(UiEvent::BeginIdle);
                });
            }
        }
        if !self.inbox.messages.is_empty() || !self.inbox.output.is_empty() {
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
            UiEvent::Wake => {
                if let Err(error) = self.install_gpu() {
                    self.fail(error, event_loop);
                    return;
                }
                self.request_redraw();
            }
            UiEvent::BeginIdle => {
                self.idle_start = Some((Instant::now(), self.meter.total_frames()));
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
            UiEvent::EndIdle => {
                let (start, frames) = self.idle_start.take().expect("idle probe started");
                let delta = self.meter.total_frames() - frames;
                let passed = delta == 0 && self.last_update == 20;
                let text = format!(
                    "R2 smoke {}: idle {:.3}s frames={} updates={} CPU p50={:.3}ms p99={:.3}ms samples={}",
                    if passed { "PASS" } else { "FAIL" },
                    start.elapsed().as_secs_f64(),
                    delta,
                    self.last_update,
                    self.meter.cpu_percentile(0.5).as_secs_f64() * 1000.,
                    self.meter.cpu_percentile(0.99).as_secs_f64() * 1000.,
                    self.meter.samples().len()
                );
                if !passed {
                    self.error = Some("R2 idle/update probe failed".into());
                }
                if self.smoke {
                    self.logger.finish(text, self.proxy.clone());
                } else {
                    self.logger.write(text);
                }
            }
            UiEvent::Quit => event_loop.exit(),
        }
    }
    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(gpu) = &mut self.gpu {
                    gpu.resize(size.width, size.height);
                }
                self.request_redraw();
            }
            WindowEvent::ScaleFactorChanged { .. } => self.request_redraw(),
            WindowEvent::ModifiersChanged(modifiers) => self.modifiers = modifiers.state(),
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
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
    let event_loop = EventLoop::<UiEvent>::with_user_event().build()?;
    let proxy = event_loop.create_proxy();
    let inbox = bbt_rt::init_ui(Wake(proxy.clone()));
    let prepared = Arc::new(Latest::new(inbox.dirty.source(2)));
    let updates = Arc::new(Latest::new(inbox.dirty.source(3)));
    let smoke = std::env::args().any(|arg| arg == "--smoke");
    let fake = smoke || std::env::args().any(|arg| arg == "--fake-source");
    let overlay = std::env::args().any(|arg| arg == "--meter");
    let args: Vec<_> = std::env::args().collect();
    let capture = args
        .windows(2)
        .find(|pair| pair[0] == "--capture")
        .map(|pair| std::path::PathBuf::from(&pair[1]));
    let mut app = App {
        proxy,
        inbox,
        logger: log::Logger::new(),
        prepared,
        updates,
        window: None,
        gpu: None,
        scene: scene::Scene::new(),
        meter: HitchMeter::new(120.),
        modifiers: ModifiersState::empty(),
        overlay,
        fake,
        smoke,
        demo_started: false,
        idle_scheduled: false,
        last_update: 0,
        idle_start: None,
        error: None,
        capture,
    };
    event_loop.run_app(&mut app)?;
    if let Some(error) = app.error {
        return Err(error.into());
    }
    Ok(())
}
