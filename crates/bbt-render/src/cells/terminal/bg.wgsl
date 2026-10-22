struct View { screen: vec2<f32>, cell: vec2<f32>, grid: vec2<u32>, padding: vec2<u32> }
@group(0) @binding(0) var<uniform> view: View;
@group(0) @binding(1) var<storage, read> backgrounds: array<u32>;

fn linear(rgb: vec3<f32>) -> vec3<f32> {
    return select(rgb / 12.92, pow((rgb + 0.055) / 1.055, vec3<f32>(2.4)), rgb > vec3<f32>(0.04045));
}
struct Out {
    @builtin(position) position: vec4<f32>,
    @location(0) pixel: vec2<f32>,
}
@vertex fn vs(@builtin(vertex_index) index: u32) -> Out {
    let points = array<vec2<f32>, 6>(vec2(-1., -1.), vec2(1., -1.), vec2(-1., 1.), vec2(-1., 1.), vec2(1., -1.), vec2(1., 1.));
    let p = points[index];
    var out: Out;
    out.position = vec4(p, 0., 1.);
    out.pixel = vec2((p.x + 1.) * 0.5, (1. - p.y) * 0.5) * view.screen;
    return out;
}
@fragment fn fs(input: Out) -> @location(0) vec4<f32> {
    let cell = min(vec2<u32>(input.pixel / view.cell), view.grid - vec2(1u));
    let rgba = unpack4x8unorm(backgrounds[cell.y * view.grid.x + cell.x]);
    return vec4(linear(rgba.rgb), rgba.a);
}
