// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
struct View {viewport:vec4f, transform:vec4f, bounds:vec4f, color:vec4f}
@group(0) @binding(0) var<uniform> view:View;
@group(0) @binding(1) var<storage,read> edges:array<vec4f>;
@group(0) @binding(2) var<storage,read> clips:array<vec4f>;
struct Vertex {@builtin(position) pos:vec4f,@location(0) point:vec2f}
@vertex fn vs(@builtin(vertex_index) v:u32)->Vertex {
    let corners=array<vec2f,6>(vec2f(0,0),vec2f(1,0),vec2f(0,1),vec2f(0,1),vec2f(1,0),vec2f(1,1));
    let start=view.transform.xy+view.bounds.xy*view.transform.z-vec2f(1);
    let p=start+corners[v]*(view.bounds.zw*view.transform.z+vec2f(2));
    return Vertex(vec4f(p.x/view.viewport.x*2.0-1.0,1.0-p.y/view.viewport.y*2.0,0,1),p);
}
@fragment fn fs(v:Vertex)->@location(0) vec4f {
    for(var i=0u;i<u32(view.viewport.w);i++) {
        if rounded_sdf(v.point/view.viewport.z,clips[i*2u],clips[i*2u+1u])>0.0 {discard;}
    }
    // Green's theorem with a clamped x primitive gives the exact polygon
    // area inside this pixel's unit square. Unlike a distance ramp it retains
    // thin-stroke area and covers diagonal/corner pixels without a wide fringe.
    let pixel=floor(v.point);var area=0.0;
    for(var i=0u;i<u32(view.transform.w);i++) {
        let e=edges[i];
        let a=e.xy*view.transform.z+view.transform.xy-pixel;
        let b=e.zw*view.transform.z+view.transform.xy-pixel;
        let dy=b.y-a.y;
        if abs(dy)<1e-8 {continue;}
        let top=max(min(a.y,b.y),0.0);let bottom=min(max(a.y,b.y),1.0);
        if bottom<=top {continue;}
        let x0=a.x+(b.x-a.x)*(top-a.y)/dy;
        let x1=a.x+(b.x-a.x)*(bottom-a.y)/dy;
        var average=clamp((x0+x1)*0.5,0.0,1.0);
        if abs(x1-x0)>1e-5 {average=(integral(x1)-integral(x0))/(x1-x0);}
        area+=sign(dy)*(bottom-top)*average;
    }
    return vec4f(view.color.rgb,view.color.a*clamp(abs(area),0.0,1.0));
}
fn integral(x:f32)->f32 {
    if x<=0.0 {return 0.0;}
    if x<1.0 {return x*x*0.5;}
    return x-0.5;
}
