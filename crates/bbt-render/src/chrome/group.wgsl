// Layer pixels are premultiplied sRGB. CSS parameters:saturate runs in sRGB;
// opacity scales the completed layer, including its alpha, exactly once.
@group(0) @binding(0) var layer: texture_2d<f32>;
struct Parameters {
    effects: vec4f,
    line: vec4f,
    alpha: vec4f,
}
@group(0) @binding(1) var<uniform> parameters: Parameters;

@vertex fn vs(@builtin(vertex_index) vertex: u32) -> @builtin(position) vec4f {
    let p = array<vec2f, 3>(vec2f(-1,-1), vec2f(3,-1), vec2f(-1,3));
    return vec4f(p[vertex], 0, 1);
}
@fragment fn fs(@builtin(position) position: vec4f) -> @location(0) vec4f {
    let c = textureLoad(layer, vec2i(position.xy), 0);
    let rgb = c.rgb / max(c.a, 0.000001);
    let luma = dot(rgb, vec3f(0.213, 0.715, 0.072));
    let color = clamp(vec3f(luma) + (rgb - vec3f(luma)) * parameters.effects.y, vec3f(0), vec3f(1));
    var alpha = c.a * parameters.effects.x;
    if parameters.effects.z > 0. {
        let delta = parameters.line.zw - parameters.line.xy;
        let t = clamp(dot(position.xy - parameters.line.xy, delta) / dot(delta, delta), 0., 1.);
        alpha *= mix(parameters.alpha.x, parameters.alpha.y, t);
    }
    return vec4f(color * alpha, alpha);
}
