// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
struct View { screen: vec2<f32>, cell: vec2<f32>, grid: vec2<u32>, padding: vec2<u32> }
@group(0) @binding(0) var<uniform> view: View;
@group(0) @binding(1) var masks: texture_2d<f32>;
@group(0) @binding(2) var colours: texture_2d<f32>;
struct Vertex {
    @location(0) cell: vec2<u32>, @location(1) bearing: vec2<i32>,
    @location(2) size: vec2<i32>, @location(3) uv: vec4<u32>,
    @location(4) fg: vec4<f32>, @location(5) bg: vec4<f32>, @location(6) flags: u32,
}
struct Out {
    @builtin(position) position: vec4<f32>, @location(0) uv: vec2<f32>,
    @location(1) fg: vec4<f32>, @location(2) bg: vec4<f32>, @location(3) @interpolate(flat) flags: u32,
}
fn linear(v: vec3<f32>) -> vec3<f32> {
    return select(v/12.92,pow((v+0.055)/1.055,vec3(2.4)),v>vec3(0.04045));
}
fn gamma(v: f32) -> f32 {
    return select(v*12.92,1.055*pow(max(v,0.),1./2.4)-0.055,v>0.0031308);
}
fn linear_one(v: f32) -> f32 {
    return select(v/12.92,pow((v+0.055)/1.055,2.4),v>0.04045);
}
@vertex fn vs(input: Vertex, @builtin(vertex_index) index: u32) -> Out {
    let corners=array<vec2<f32>,6>(vec2(0.,0.),vec2(1.,0.),vec2(0.,1.),vec2(0.,1.),vec2(1.,0.),vec2(1.,1.));
    let c=corners[index];
    let pixel=vec2<f32>(input.cell)*view.cell+vec2<f32>(input.bearing)+c*vec2<f32>(input.size);
    var out: Out;
    out.position=vec4(pixel.x/view.screen.x*2.-1.,1.-pixel.y/view.screen.y*2.,0.,1.);
    out.uv=vec2<f32>(input.uv.xy)+c*vec2<f32>(input.uv.zw);
    out.fg=input.fg; out.bg=input.bg; out.flags=input.flags;
    return out;
}
@fragment fn fs(input: Out) -> @location(0) vec4<f32> {
    if (input.flags & 1u) != 0u {
        let sample=textureLoad(colours,vec2<i32>(input.uv),0);
        let a=sample.a*input.fg.a;
        // CoreGraphics stored premultiplied sRGB BGRA. Decode straight colour,
        // then premultiply in linear space so transparent emoji edges stay clean.
        return vec4(linear(sample.rgb/max(sample.a,0.00001))*a,a);
    }
    var a=textureLoad(masks,vec2<i32>(input.uv),0).r*input.fg.a;
    let fg=linear(input.fg.rgb); let bg=linear(input.bg.rgb);
    if (input.flags & 2u) == 0u {
        let lf=dot(fg,vec3(0.2126,0.7152,0.0722));
        let lb=dot(bg,vec3(0.2126,0.7152,0.0722));
        if abs(lf-lb)>0.001 {
            let target_luminance=linear_one(gamma(lf)*a+gamma(lb)*(1.-a));
            a=clamp((target_luminance-lb)/(lf-lb),0.,1.);
        }
    }
    return vec4(fg*a,a);
}
