// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
/// One camera rule for native, resvg and Chrome. The full root clip extends to
/// integer user units; a selected camera crops those units without CSS layout.
#[derive(Debug, Clone, Copy)]
pub struct CaptureMapping {
    pub viewport: [f64; 4],
    pub camera: [f64; 4],
    pub scale: f64,
    pub size: [u32; 2],
    pub transform: [f32; 6],
    pub root_clip: bool,
}
pub fn capture_viewport(
    bounds: [f64; 4],
    camera: Option<[f64; 4]>,
    scale: f64,
) -> Result<CaptureMapping, &'static str> {
    if bounds.iter().any(|v| !v.is_finite()) || bounds[2] <= 0. || bounds[3] <= 0. {
        return Err("finite positive viewBox required");
    }
    let viewport = [bounds[0], bounds[1], bounds[2].ceil(), bounds[3].ceil()];
    let camera = camera.unwrap_or(viewport);
    if camera.iter().any(|v| !v.is_finite())
        || camera[2] <= 0.
        || camera[3] <= 0.
        || !scale.is_finite()
        || scale <= 0.
    {
        return Err("finite positive camera and scale required");
    }
    let width = (camera[2] * scale).ceil();
    let height = (camera[3] * scale).ceil();
    if !width.is_finite()
        || !height.is_finite()
        || width <= 0.
        || height <= 0.
        || width > 4096.
        || height > 4096.
        || width * height > 16_777_216.
    {
        return Err("capture exceeds physical surface bounds");
    }
    let c = camera.map(|v| v as f32);
    let s = scale as f32;
    let root_clip = camera[0] < viewport[0]
        || camera[1] < viewport[1]
        || camera[0] + camera[2] > viewport[0] + viewport[2]
        || camera[1] + camera[3] > viewport[1] + viewport[3];
    let transform = [s, 0., 0., s, -c[0] * s, -c[1] * s];
    if transform.iter().any(|v| !v.is_finite()) {
        return Err("camera transform exceeds finite renderer coordinates");
    }
    Ok(CaptureMapping {
        viewport,
        camera,
        scale,
        size: [width as u32, height as u32],
        transform,
        root_clip,
    })
}
