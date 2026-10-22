// Layer pixels are premultiplied sRGB. CSS parameters:saturate runs in sRGB;
// opacity scales the completed layer, including its alpha, exactly once.
@group(0) @binding(0) var layer: texture_2d<f32>;
@group(0) @binding(1) var<uniform> parameters: vec4f;

@vertex fn vs(@builtin(vertex_index) vertex: u32) -> @builtin(position) vec4f {
    let p = array<vec2f, 3>(vec2f(-1,-1), vec2f(3,-1), vec2f(-1,3));
    return vec4f(p[vertex], 0, 1);
}
@fragment fn fs(@builtin(position) position: vec4f) -> @location(0) vec4f {
    let c = textureLoad(layer, vec2i(position.xy), 0);
    let rgb = c.rgb / max(c.a, 0.000001);
    let luma = dot(rgb, vec3f(0.213, 0.715, 0.072));
    let color = clamp(vec3f(luma) + (rgb - vec3f(luma)) * parameters.y, vec3f(0), vec3f(1));
    let alpha = c.a * parameters.x;
    return vec4f(color * alpha, alpha);
}
