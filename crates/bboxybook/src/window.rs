//! Gallery controls only; product seat input routing belongs to W3.2c.
use crate::{Result, gallery};
use bbt_render::{chrome::ChromeRenderer, device::DeviceContext};
use bbt_ui_model::{Density, Theme, draw::*, layout::Rect};
use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalSize, PhysicalPosition},
    event::{ElementState, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{Key, NamedKey},
    window::{Window, WindowId},
};
struct Gallery {
    window: Option<Arc<Window>>,
    surface: Option<wgpu::Surface<'static>>,
    renderer: Option<ChromeRenderer>,
    config: Option<wgpu::SurfaceConfiguration>,
    entries: Vec<gallery::Entry>,
    selected: usize,
    state: usize,
    density: Density,
    theme: Theme,
    pointer: [f32; 2],
    scroll: usize,
    error: Option<String>,
    smoke: bool,
    frames: u8,
}
impl Default for Gallery {
    fn default() -> Self {
        Self {
            window: None,
            surface: None,
            renderer: None,
            config: None,
            entries: gallery::entries(),
            selected: 0,
            state: 0,
            density: Density::Cozy,
            theme: Theme::Dark,
            pointer: [0.0; 2],
            scroll: 0,
            error: None,
            smoke: false,
            frames: 0,
        }
    }
}
impl Gallery {
    fn init(&mut self, event_loop: &ActiveEventLoop) -> Result<()> {
        let window = Arc::new(
            event_loop.create_window(
                Window::default_attributes()
                    .with_title("bboxybook")
                    .with_inner_size(LogicalSize::new(1280.0, 860.0))
                    .with_decorations(false),
            )?,
        );
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::METAL,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let surface = instance.create_surface(window.clone())?;
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: Some(&surface),
            ..Default::default()
        }))?;
        let caps = surface.get_capabilities(&adapter);
        let format = *caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .ok_or("no sRGB surface")?;
        let info = adapter.get_info();
        let (device, queue) = pollster::block_on(adapter.request_device(&Default::default()))?;
        let size = window.inner_size();
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::Fifo,
            desired_maximum_frame_latency: 2,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![format.remove_srgb_suffix()],
            color_space: wgpu::SurfaceColorSpace::Srgb,
        };
        surface.configure(&device, &config);
        self.renderer = Some(ChromeRenderer::with_context(
            DeviceContext {
                device,
                queue,
                adapter_info: info,
            },
            crate::images()?,
        )?);
        self.surface = Some(surface);
        self.config = Some(config);
        self.window = Some(window);
        Ok(())
    }
    fn paint(&mut self) -> Result<()> {
        let window = self.window.as_ref().unwrap();
        let config = self.config.as_ref().unwrap();
        let scale = window.scale_factor() as f32;
        let w = config.width as f32 / scale;
        let h = config.height as f32 / scale;
        let ctx = DrawCtx::new(self.theme, self.density);
        let mut list = DrawList::default();
        list.rect(
            Rect::new(0.0, 0.0, w, h),
            0.0,
            Color::hex(ctx.theme.palette().bg),
        );
        let style = TextStyle {
            scale: TypeScale::Custom(12.0),
            ..Default::default()
        };
        list.rect(
            Rect::new(0.0, 0.0, w, 38.0),
            0.0,
            Color::hex(ctx.theme.palette().surface),
        );
        list.rect(
            Rect::new(12.0, 13.0, 12.0, 12.0),
            6.0,
            Color::hex("#FF5F57"),
        );
        list.text(
            Rect::new(40.0, 9.0, 200.0, 22.0),
            "bboxybook",
            TextStyle {
                weight: 600,
                ..style
            },
            ctx.fg(),
        );
        list.text(
            Rect::new(w - 460.0, 9.0, 280.0, 22.0),
            format!("density: {}", gallery::density_name(self.density)),
            style,
            ctx.fg(),
        );
        list.text(
            Rect::new(w - 160.0, 9.0, 150.0, 22.0),
            format!("theme: {}", gallery::theme_name(self.theme)),
            style,
            ctx.fg(),
        );
        let left = 220.0;
        list.rect(
            Rect::new(0.0, 38.0, left, h - 38.0),
            0.0,
            Color::hex(ctx.theme.palette().sidebar),
        );
        let visible = ((h - 48.0) / 26.0).max(1.0) as usize;
        if self.selected < self.scroll {
            self.scroll = self.selected;
        }
        if self.selected >= self.scroll + visible {
            self.scroll = self.selected + 1 - visible;
        }
        for (i, e) in self
            .entries
            .iter()
            .enumerate()
            .skip(self.scroll)
            .take(visible)
        {
            let y = 46.0 + (i - self.scroll) as f32 * 26.0;
            if i == self.selected {
                list.rect(
                    Rect::new(6.0, y - 1.0, left - 12.0, 24.0),
                    6.0,
                    Color::hex(ctx.theme.palette().fg).alpha(0.09),
                );
            }
            list.text(
                Rect::new(14.0, y + 2.0, left - 24.0, 20.0),
                e.id.as_str(),
                style,
                if i == self.selected {
                    ctx.fg()
                } else {
                    ctx.muted()
                },
            );
        }
        let entry = &self.entries[self.selected];
        let strip_h = 62.0;
        let mut x = left + 16.0;
        let mut y = 48.0;
        for (i, state) in entry.states.iter().enumerate() {
            let tw = (state.chars().count() as f32 * 6.2 + 20.0).min(w - left - 40.0);
            if x + tw > w - 10.0 {
                x = left + 16.0;
                y += 28.0;
            }
            if i == self.state {
                list.rect(
                    Rect::new(x, y, tw, 24.0),
                    12.0,
                    Color::hex(ctx.theme.palette().accent),
                );
            }
            list.text(
                Rect::new(x + 8.0, y + 3.0, tw - 16.0, 20.0),
                state.as_str(),
                style,
                ctx.fg(),
            );
            x += tw + 6.0;
        }
        let bounds = Rect::new(
            left + 16.0,
            (y + strip_h * 0.6).max(100.0),
            (w - left - 32.0).max(1.0),
            (h - y - strip_h * 0.6 - 16.0).max(1.0),
        );
        list.clip(bounds, [0.0; 4]);
        for yy in (0..bounds.h as u32).step_by(16) {
            for xx in (0..bounds.w as u32).step_by(16) {
                let c = if (xx / 16 + yy / 16) % 2 == 0 {
                    ctx.theme.palette().surface
                } else {
                    ctx.theme.palette().surface_2
                };
                list.rect(
                    Rect::new(bounds.x + xx as f32, bounds.y + yy as f32, 16.0, 16.0),
                    0.0,
                    Color::hex(c),
                );
            }
        }
        crate::modules::draw(
            &entry.id,
            &entry.states[self.state],
            bounds,
            &ctx,
            &mut list,
        )?;
        list.unclip();
        let surface = self.surface.as_ref().unwrap();
        let frame = match surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(v)
            | wgpu::CurrentSurfaceTexture::Suboptimal(v) => v,
            wgpu::CurrentSurfaceTexture::Lost | wgpu::CurrentSurfaceTexture::Outdated => {
                surface.configure(&self.renderer.as_ref().unwrap().context.device, config);
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                return Err("surface validation error".into());
            }
        };
        let target = frame.texture.create_view(&Default::default());
        let renderer = self.renderer.as_mut().unwrap();
        let mut encoder = renderer
            .context
            .device
            .create_command_encoder(&Default::default());
        renderer.draw(
            &list,
            bbt_render::chrome::ChromeTarget {
                width: config.width,
                height: config.height,
                scale,
                view: &target,
                format: config.format,
            },
            &mut encoder,
        )?;
        renderer.context.queue.submit([encoder.finish()]);
        renderer.context.queue.present(frame);
        Ok(())
    }
    fn cycle(&mut self, up: bool) {
        if up {
            self.selected = self.selected.saturating_sub(1);
        } else {
            self.selected = (self.selected + 1).min(self.entries.len() - 1);
        }
        self.state = 0;
    }
}
impl ApplicationHandler for Gallery {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none()
            && let Err(e) = self.init(event_loop)
        {
            self.error = Some(e.to_string());
            event_loop.exit();
        }
    }
    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) if size.width > 0 && size.height > 0 => {
                if let Some(config) = &mut self.config {
                    config.width = size.width;
                    config.height = size.height;
                    self.surface
                        .as_ref()
                        .unwrap()
                        .configure(&self.renderer.as_ref().unwrap().context.device, config);
                }
            }
            WindowEvent::CursorMoved {
                position: PhysicalPosition { x, y },
                ..
            } => {
                let s = self.window.as_ref().unwrap().scale_factor();
                self.pointer = [(x / s) as f32, (y / s) as f32];
                return;
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                let [x, y] = self.pointer;
                let w = self.window.as_ref().unwrap().inner_size().width as f32
                    / self.window.as_ref().unwrap().scale_factor() as f32;
                if y < 38.0 {
                    if x < 30.0 {
                        event_loop.exit();
                    } else if x > w - 170.0 {
                        self.theme = if self.theme == Theme::Dark {
                            Theme::Light
                        } else {
                            Theme::Dark
                        };
                    } else if x > w - 470.0 {
                        let all = gallery::DENSITIES;
                        let i = all.iter().position(|d| *d == self.density).unwrap();
                        self.density = all[(i + 1) % 4];
                    } else {
                        let _ = self.window.as_ref().unwrap().drag_window();
                    }
                } else if x < 220.0 {
                    let index = self.scroll + ((y - 46.0) / 26.0).max(0.0) as usize;
                    self.selected = index.min(self.entries.len() - 1);
                    self.state = 0;
                } else {
                    let mut xx = 236.0;
                    let mut yy = 48.0;
                    for (i, s) in self.entries[self.selected].states.iter().enumerate() {
                        let tw = (s.chars().count() as f32 * 6.2 + 20.0).min(w - 260.0);
                        if xx + tw > w - 10.0 {
                            xx = 236.0;
                            yy += 28.0;
                        }
                        if x >= xx && x <= xx + tw && y >= yy && y <= yy + 24.0 {
                            self.state = i;
                            break;
                        }
                        xx += tw + 6.0;
                    }
                }
            }
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                match event.logical_key {
                    Key::Named(NamedKey::Escape) => event_loop.exit(),
                    Key::Named(NamedKey::ArrowUp) => self.cycle(true),
                    Key::Named(NamedKey::ArrowDown) => self.cycle(false),
                    Key::Named(NamedKey::ArrowLeft) => self.state = self.state.saturating_sub(1),
                    Key::Named(NamedKey::ArrowRight) => {
                        self.state =
                            (self.state + 1).min(self.entries[self.selected].states.len() - 1)
                    }
                    Key::Character(ref c) if c == "t" => {
                        self.theme = if self.theme == Theme::Dark {
                            Theme::Light
                        } else {
                            Theme::Dark
                        }
                    }
                    _ => {}
                }
            }
            WindowEvent::RedrawRequested => {
                if let Err(e) = self.paint() {
                    self.error = Some(e.to_string());
                    event_loop.exit();
                } else if self.smoke {
                    self.frames += 1;
                    if self.frames >= 3 {
                        eprintln!(
                            "gallery smoke: 3 surface frames, density/theme/component switched"
                        );
                        event_loop.exit();
                    } else {
                        self.theme = Theme::Light;
                        self.density = Density::Touch;
                        self.selected = self.entries.len() - 1;
                        self.window.as_ref().unwrap().request_redraw();
                    }
                }
                return;
            }
            _ => return,
        }
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}
pub fn run(smoke: bool) -> Result<()> {
    let event_loop = EventLoop::new()?;
    let mut gallery = Gallery {
        smoke,
        ..Default::default()
    };
    event_loop.run_app(&mut gallery)?;
    if let Some(error) = gallery.error {
        return Err(error.into());
    }
    Ok(())
}
