// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Device creation without a window or surface.
use crate::{RenderError, Result};

pub struct DeviceContext {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub adapter_info: wgpu::AdapterInfo,
}

impl DeviceContext {
    pub async fn new() -> Result<Self> {
        if !cfg!(target_os = "macos") {
            return Err("W3.1 requires macOS and the Metal backend".into());
        }
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::METAL,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: None,
                apply_limit_buckets: false,
            })
            .await
            .map_err(|e| RenderError(format!("Metal adapter: {e}")))?;
        let adapter_info = adapter.get_info();
        if adapter_info.backend != wgpu::Backend::Metal {
            return Err(format!("expected Metal, got {:?}", adapter_info.backend).into());
        }
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("bbt-render headless Metal"),
                ..Default::default()
            })
            .await
            .map_err(|e| RenderError(format!("Metal device: {e}")))?;
        Ok(Self {
            device,
            queue,
            adapter_info,
        })
    }
}
