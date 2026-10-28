// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
struct View { screen: vec2<f32>, cell: vec2<f32>, grid: vec2<u32>, padding: vec2<u32> }
@group(0) @binding(0) var<uniform> view: View;
@group(0) @binding(1) var<storage, read> backgrounds: array<u32>;

fn linear(rgb: vec3<f32>) -> vec3<f32> {
    return select(rgb / 12.92, pow((rgb + 0.055) / 1.055, vec3<f32>(2.4)), rgb > vec3<f32>(0.04045));
}
@vertex fn vs(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let points = array<vec2<f32>, 6>(vec2(-1., -1.), vec2(1., -1.), vec2(-1., 1.), vec2(-1., 1.), vec2(1., -1.), vec2(1., 1.));
    return vec4(points[index], 0., 1.);
}
@fragment fn fs(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let cell = min(vec2<u32>(position.xy / view.cell), view.grid - vec2(1u));
    let rgba = unpack4x8unorm(backgrounds[cell.y * view.grid.x + cell.x]);
    return vec4(linear(rgba.rgb), rgba.a);
}
