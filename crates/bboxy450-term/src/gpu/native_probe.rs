// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Private native-window observation, not a redraw/capture request. Band copies
//! share the normal frame submission. Readback starts after 16 produced frames,
//! off UI; no per-frame GPU wait can repair an intermediate drawable. Finite
//! ceiling: 2048 frames, 16 columns, 4096 pixel rows, one focused pane. This
//! observes submitted drawable bytes, not physical scanout. Files stay local.
use super::*;
use serde_json::json;
use std::path::PathBuf;

const BATCH: usize = 16;
const LIMIT: u64 = 2048;

pub(super) struct NativeProbe {
    path: PathBuf,
    pending: Vec<Sample>,
    count: u64,
    control: Option<String>,
    rejected_drawables: u64,
    controls: Vec<(wgpu::TextureFormat, wgpu::RenderPipeline)>,
    surface_configurations: u64,
    layer_resizes: u64,
}

pub(super) struct Sample {
    source: wgpu::Texture,
    source_origin: [u32; 2],
    destination_origin: [u32; 2],
    size: [u32; 2],
    stride: u32,
    buffer: wgpu::Buffer,
    metadata: serde_json::Value,
}

impl NativeProbe {
    pub(super) fn from_args() -> Result<Option<Self>, String> {
        let args: Vec<_> = std::env::args().collect();
        let Some(index) = args.iter().position(|arg| arg == "--native-frame-probe") else {
            return Ok(None);
        };
        if !cfg!(debug_assertions)
            || !args.iter().any(|arg| arg == "--test-account")
            || !args.iter().any(|arg| arg == "--unattended")
            || args.iter().any(|arg| arg == "--headless")
        {
            return Err(
                "native frame probe requires a debug unattended native test account".into(),
            );
        }
        let path = args
            .get(index + 1)
            .ok_or("native frame probe needs DIR")?
            .into();
        let control = args
            .windows(2)
            .find(|v| v[0] == "--native-frame-control")
            .map(|v| v[1].clone());
        if control
            .as_ref()
            .is_some_and(|v| !matches!(v.as_str(), "source" | "copy" | "final"))
        {
            return Err("native frame control must be source, copy or final".into());
        }
        Ok(Some(Self {
            path,
            pending: vec![],
            count: 0,
            control,
            rejected_drawables: 0,
            controls: vec![],
            surface_configurations: 0,
            layer_resizes: 0,
        }))
    }

    // Adapted from bboxy450-term crates/bbt-render/src/chrome/group.rs::pipeline:
    // the descriptor only; independent constant-output strip shader, no blending.
    // https://github.com/bboxy450/bboxy450-term
    // Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
    pub(super) fn prepare_controls(
        &mut self,
        device: &wgpu::Device,
        final_format: wgpu::TextureFormat,
    ) {
        if self.control.is_none() {
            return;
        }
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("private GPU strip mutation"),
            source: wgpu::ShaderSource::Wgsl("@vertex fn vs(@builtin(vertex_index) i:u32)->@builtin(position) vec4<f32> { let p=array<vec2<f32>,3>(vec2(-1.,-1.),vec2(3.,-1.),vec2(-1.,3.)); return vec4(p[i],0.,1.); } @fragment fn fs()->@location(0) vec4<f32> { return vec4(0.); }".into()),
        });
        for format in [FORMAT, final_format] {
            self.controls.push((
                format,
                device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("native blank-strip broken control"),
                    layout: None,
                    vertex: wgpu::VertexState {
                        module: &shader,
                        entry_point: Some("vs"),
                        buffers: &[],
                        compilation_options: Default::default(),
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &shader,
                        entry_point: Some("fs"),
                        targets: &[Some(wgpu::ColorTargetState {
                            format,
                            blend: None,
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                        compilation_options: Default::default(),
                    }),
                    primitive: Default::default(),
                    depth_stencil: None,
                    multisample: Default::default(),
                    multiview_mask: None,
                    cache: None,
                }),
            ));
        }
    }

    pub(super) fn begin(
        &self,
        device: &wgpu::Device,
        panes: &Panes<'_>,
        destination: &wgpu::Texture,
        scale: f32,
        frame_id: u64,
    ) -> Option<Sample> {
        if self.count >= LIMIT {
            return None;
        }
        let id = panes.focused?;
        let rect = panes.bounds.iter().find(|(seat, _)| *seat == id)?.1;
        let prepared = panes.model.prepared_screens.get(&id)?;
        let screen = &prepared.screen;
        let grid = panes.model.frames.get(&id)?;
        let raster = prepared.raster.as_ref()?;
        if screen.cols == 0 || screen.rows == 0 {
            return None;
        }
        let cell = [
            grid.width / screen.cols as u32,
            grid.height / screen.rows as u32,
        ];
        if cell.contains(&0) {
            return None;
        }
        let x = (rect.x * scale).round().max(0.) as u32;
        let y = (rect.y * scale).round().max(0.) as u32;
        let width = ((rect.w * scale).floor().max(0.) as u32)
            .min(grid.width)
            .min(destination.width().saturating_sub(x));
        let height = ((rect.h * scale).floor().max(0.) as u32)
            .min(grid.height)
            .min(destination.height().saturating_sub(y))
            .min(4096);
        let cols = width / cell[0];
        let rows = height / cell[1];
        if cols == 0 || rows == 0 {
            return None;
        }
        let first = cols.saturating_sub(16);
        let size = [(cols - first) * cell[0], rows * cell[1]];
        let origin = [first * cell[0], 0];
        let expected: Vec<_> = (0..rows)
            .flat_map(|row| {
                (first..cols).filter_map(move |col| {
                    let c = &screen.cells[row as usize][col as usize];
                    // Local Codex provider deliberately emits W, including at the edge.
                    // Other glyphs/colours still participate in source/copy/final parity.
                    (c.c == 'W').then_some([col - first, row])
                })
            })
            .collect();
        let stride = (size[0] * 4).div_ceil(256) * 256;
        // Bounded observer only. Unsupported sampling extents yield no receipt;
        // the driver refuses insufficient coverage rather than blocking a frame.
        if u64::from(stride) * u64::from(size[1]) * 3 > 4 * 1024 * 1024 {
            return None;
        }
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("native per-present source/composition/drawable bands"),
            size: u64::from(stride) * u64::from(size[1]) * 3,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        Some(Sample {
            source: raster.normal.clone(),
            source_origin: origin,
            destination_origin: [x + origin[0], y],
            size,
            stride,
            buffer,
            metadata: json!({"frame":frame_id, "seat":format!("{id:?}"), "scale":scale,
                "grid":[screen.cols,screen.rows], "prepared":[grid.width,grid.height],
                "source":[raster.normal.width(),raster.normal.height()],
                "target":[destination.width(),destination.height()],
                "pane":[x,y,width,height], "origin":origin, "size":size,
                "cell":cell, "stride":stride, "expected":expected,
                "baseline":prepared.baseline, "control":self.control,
                "rejected_drawables":self.rejected_drawables,
                "surface_configurations":self.surface_configurations, "layer_resizes":self.layer_resizes,
                "source_current":panes.model.screens.get(&id).is_some_and(|s|Arc::ptr_eq(s,screen))}),
        })
    }

    pub(super) fn rejected_drawable(&mut self) {
        self.rejected_drawables += 1;
    }
    pub(super) fn surface_configured(&mut self) {
        self.surface_configurations += 1;
    }
    pub(super) fn layer_resized(&mut self) {
        self.layer_resizes += 1;
    }

    pub(super) fn inject(
        &self,
        sample: &Sample,
        stage: &str,
        target: &wgpu::Texture,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        if self.control.as_deref() != Some(stage)
            || sample.metadata["expected"]
                .as_array()
                .is_none_or(Vec::is_empty)
        {
            return;
        }
        let (texture, origin) = if stage == "source" {
            (&sample.source, sample.source_origin)
        } else {
            (target, sample.destination_origin)
        };
        // A real GPU blank strip, not a forged detector verdict. The external
        // checker must reject the corrupted produced frame at the correct stage.
        let pipeline = &self
            .controls
            .iter()
            .find(|(format, _)| *format == texture.format())
            .expect("prepared control format")
            .1;
        let view = texture.create_view(&Default::default());
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("blank only the measured right-edge band"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        pass.set_pipeline(pipeline);
        pass.set_scissor_rect(origin[0], origin[1], sample.size[0], sample.size[1]);
        pass.draw(0..3, 0..1);
    }

    pub(super) fn record(
        &mut self,
        mut sample: Sample,
        target: &wgpu::Texture,
        drawable: &wgpu::Texture,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        sample.metadata["drawable"] = json!([drawable.width(), drawable.height()]);
        for (index, (texture, origin)) in [
            (&sample.source, sample.source_origin),
            (target, sample.destination_origin),
            (drawable, sample.destination_origin),
        ]
        .into_iter()
        .enumerate()
        {
            encoder.copy_texture_to_buffer(
                wgpu::TexelCopyTextureInfo {
                    origin: wgpu::Origin3d {
                        x: origin[0],
                        y: origin[1],
                        z: 0,
                    },
                    ..texture.as_image_copy()
                },
                wgpu::TexelCopyBufferInfo {
                    buffer: &sample.buffer,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: index as u64 * u64::from(sample.stride) * u64::from(sample.size[1]),
                        bytes_per_row: Some(sample.stride),
                        rows_per_image: Some(sample.size[1]),
                    },
                },
                wgpu::Extent3d {
                    width: sample.size[0],
                    height: sample.size[1],
                    depth_or_array_layers: 1,
                },
            );
        }
        self.count += 1;
        self.pending.push(sample);
    }

    #[allow(
        clippy::disallowed_methods,
        reason = "private probe readback and file IO run only on the asserted blocking worker"
    )]
    pub(super) fn submitted(&mut self, device: &wgpu::Device) {
        if self.pending.len() < BATCH {
            return;
        }
        let samples = std::mem::take(&mut self.pending);
        let device = device.clone();
        let path = self.path.clone();
        bbt_rt::handle().spawn_blocking(move || {
            bbt_rt::assert_not_ui_thread();
            // Existing immutable publication/worker discipline; no UI state or
            // drawable retained. Failure creates no receipt: the checker refuses
            // missing/incomplete batches. No operator identity or desktop read.
            let first = samples[0].metadata["frame"].as_u64().unwrap();
            let write = || -> Result<(), String> {
                for sample in samples {
                    let (send, receive) = mpsc::sync_channel(1);
                    sample
                        .buffer
                        .slice(..)
                        .map_async(wgpu::MapMode::Read, move |r| {
                            let _ = send.try_send(r);
                        });
                    // Never request global device idleness: that can contend
                    // with the native surface's resize/configure owner. Poll
                    // completion without a GPU wait; only this IO worker waits
                    // on its own bounded map callback, after 16 copied frames.
                    let deadline = Instant::now() + Duration::from_secs(20);
                    loop {
                        device
                            .poll(wgpu::PollType::Poll)
                            .map_err(|e| e.to_string())?;
                        match receive.recv_timeout(Duration::from_millis(10)) {
                            Ok(result) => {
                                result.map_err(|e| e.to_string())?;
                                break;
                            }
                            Err(mpsc::RecvTimeoutError::Timeout) if Instant::now() < deadline => {}
                            Err(error) => return Err(error.to_string()),
                        }
                    }
                    let bytes = sample
                        .buffer
                        .slice(..)
                        .get_mapped_range()
                        .map_err(|e| e.to_string())?;
                    let stem =
                        path.join(format!("{:06}", sample.metadata["frame"].as_u64().unwrap()));
                    std::fs::write(stem.with_extension("bin"), &bytes)
                        .map_err(|e| e.to_string())?;
                    let staged = stem.with_extension("json.tmp");
                    std::fs::write(&staged, sample.metadata.to_string())
                        .map_err(|e| e.to_string())?;
                    std::fs::rename(staged, stem.with_extension("json"))
                        .map_err(|e| e.to_string())?;
                }
                Ok(())
            };
            if let Err(error) = write() {
                eprintln!("native frame probe: {error}");
                let _ = std::fs::write(path.join(format!("{first:06}.error")), error);
            }
        });
    }
}

impl Sample {
    pub(super) fn surface(&mut self, surface: &wgpu::Surface<'_>, native: bool) {
        self.metadata["native_display_link"] = json!(native);
        // SAFETY: observational access to this frame owner's Metal layer. A
        // contended layer is recorded as unavailable; never wait on its lock.
        let Some(surface) = (unsafe { surface.as_hal::<wgpu::hal::api::Metal>() }) else {
            return;
        };
        if let Some(layer) = surface.render_layer().try_lock() {
            let bounds = layer.bounds();
            let drawable = layer.drawableSize();
            self.metadata["layer"] = json!({"bounds":[bounds.size.width,bounds.size.height],
                "contents_scale":layer.contentsScale(), "drawable":[drawable.width,drawable.height]});
        }
    }
}
