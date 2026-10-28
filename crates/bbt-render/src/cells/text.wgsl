// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
struct View { screen: vec2<f32>, cell: vec2<f32>, grid: vec2<u32>, padding: vec2<u32> }
@group(0) @binding(0) var<uniform> view: View;
@group(0) @binding(1) var atlas: texture_2d<f32>;
struct Vertex {
    @location(0) cell: vec2<u32>, @location(1) bearing: vec2<i32>,
    @location(2) size: vec2<i32>, @location(3) uv: vec4<u32>,
    @location(4) foreground: vec4<f32>, @location(5) background: vec4<f32>, @location(6) flags: u32,
}
struct Out { @builtin(position) position: vec4<f32>, @location(0) uv: vec2<f32>, @location(1) foreground: vec4<f32> }
@vertex fn vs(input: Vertex, @builtin(vertex_index) index: u32) -> Out {
    let corners = array<vec2<f32>, 6>(vec2(0., 0.), vec2(1., 0.), vec2(0., 1.), vec2(0., 1.), vec2(1., 0.), vec2(1., 1.));
    let corner = corners[index];
    let pixel = vec2<f32>(input.cell) * view.cell + vec2<f32>(input.bearing) + corner * vec2<f32>(input.size);
    var out: Out;
    out.position = vec4(pixel.x / view.screen.x * 2. - 1., 1. - pixel.y / view.screen.y * 2., 0., 1.);
    out.uv = vec2<f32>(input.uv.xy) + corner * vec2<f32>(input.uv.zw);
    out.foreground = input.foreground;
    return out;
}
@fragment fn fs(input: Out) -> @location(0) vec4<f32> {
    let alpha = textureLoad(atlas, vec2<i32>(input.uv), 0).r;
    let rgb = input.foreground.rgb;
    let linear = select(rgb / 12.92, pow((rgb + 0.055) / 1.055, vec3<f32>(2.4)), rgb > vec3<f32>(0.04045));
    return vec4(linear, alpha * input.foreground.a);
}
