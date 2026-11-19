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
        #[cfg(target_vendor = "apple")]
        let (backends, backend, label) =
            (wgpu::Backends::METAL, "Metal", "bbt-render headless Metal");
        #[cfg(not(target_vendor = "apple"))]
        let (backends, backend, label) = (
            wgpu::Backends::VULKAN | wgpu::Backends::GL,
            "Vulkan/GLES",
            "bbt-render headless Vulkan/GLES",
        );
        if !backends.intersects(wgpu::Instance::enabled_backend_features()) {
            return Err(format!("no {backend} backend compiled for this target").into());
        }
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends,
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
            .map_err(|e| RenderError(format!("{backend} adapter: {e}")))?;
        let adapter_info = adapter.get_info();
        if !backends.contains(adapter_info.backend.into()) {
            return Err(format!("expected {backend}, got {:?}", adapter_info.backend).into());
        }
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some(label),
                ..Default::default()
            })
            .await
            .map_err(|e| RenderError(format!("{backend} device: {e}")))?;
        Ok(Self {
            device,
            queue,
            adapter_info,
        })
    }
}
