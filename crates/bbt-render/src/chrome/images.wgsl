// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
// One textured quad: a terminal picture (Kitty graphics) over the cells.
// Straight-alpha sRGB texture, premultiplied on the way out, source-over.
struct Quad {
    // Destination rectangle in physical pixels: x, y, w, h.
    rect: vec4f,
    // Clip rectangle in physical pixels: x, y, w, h.
    clip: vec4f,
    // Source rectangle in texture UV: u0, v0, u1, v1.
    uv: vec4f,
    // Target size in physical pixels, unused, unused, unused.
    target: vec4f,
}
@group(0) @binding(0) var picture: texture_2d<f32>;
@group(0) @binding(1) var picture_sampler: sampler;
@group(0) @binding(2) var<uniform> quad: Quad;

struct Vertex {
    @builtin(position) position: vec4f,
    @location(0) uv: vec2f,
    @location(1) pixel: vec2f,
}

@vertex fn vs(@builtin(vertex_index) vertex: u32) -> Vertex {
    let corners = array<vec2f, 6>(
        vec2f(0, 0), vec2f(1, 0), vec2f(0, 1),
        vec2f(0, 1), vec2f(1, 0), vec2f(1, 1),
    );
    let c = corners[vertex];
    let pixel = quad.rect.xy + c * quad.rect.zw;
    let ndc = vec2f(pixel.x / quad.target.x * 2 - 1, 1 - pixel.y / quad.target.y * 2);
    var out: Vertex;
    out.position = vec4f(ndc, 0, 1);
    out.uv = mix(quad.uv.xy, quad.uv.zw, c);
    out.pixel = pixel;
    return out;
}

@fragment fn fs(in: Vertex) -> @location(0) vec4f {
    if in.pixel.x < quad.clip.x || in.pixel.y < quad.clip.y
        || in.pixel.x > quad.clip.x + quad.clip.z || in.pixel.y > quad.clip.y + quad.clip.w {
        discard;
    }
    let c = textureSample(picture, picture_sampler, in.uv);
    return vec4f(c.rgb * c.a, c.a);
}
