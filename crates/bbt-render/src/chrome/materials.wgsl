struct Instance {rect:vec4f,color:vec4f,radii:vec4f,p0:vec4f,p1:vec4f,p2:vec4f,p3:vec4f,p4:vec4f,p5:vec4f,clip:vec4f}
@group(0) @binding(0) var<uniform> view:vec4f;
@group(0) @binding(1) var<storage,read> items:array<Instance>;
@group(0) @binding(2) var backdrop:texture_2d<f32>;
@group(0) @binding(3) var backdrop_sampler:sampler;
struct Vertex {@builtin(position) position:vec4f,@location(0) point:vec2f,@location(1) @interpolate(flat) index:u32}
@vertex fn vs(@builtin(vertex_index) vertex:u32,@builtin(instance_index) index:u32)->Vertex {
    let corners=array<vec2f,6>(vec2f(0,0),vec2f(1,0),vec2f(0,1),vec2f(0,1),vec2f(1,0),vec2f(1,1));let i=items[index];let margin=i.p0.w+1.0;
    let p=i.rect.xy-vec2f(margin)+corners[vertex]*(i.rect.zw+vec2f(margin*2.0));return Vertex(vec4f(p.x/view.x*2.0-1.0,1.0-p.y/view.y*2.0,0,1),p,index);
}
fn coverage(v:Vertex)->f32 {let i=items[v.index];if any(v.point<i.clip.xy)||any(v.point>=i.clip.xy+i.clip.zw){discard;}return clamp(0.5-rounded_sdf(v.point,i.rect,i.radii),0.0,1.0);}
fn paint(c:vec4f,a:f32)->vec4f {let alpha=c.a*a;return vec4f(clamp(c.rgb,vec3f(0),vec3f(1))*alpha,alpha);}
@fragment fn flat(v:Vertex)->@location(0) vec4f {return paint(items[v.index].color,coverage(v));}
@fragment fn gloss(v:Vertex)->@location(0) vec4f {
    let i=items[v.index];let t=clamp((v.point.y-i.rect.y)/i.rect.w,0.0,1.0);var c=i.color.rgb;
    if t<=0.55 {c=mix(mix(c,vec3f(1),i.p1.x),c,t/0.55);}else{c=mix(c,c*(1.0-i.p1.y),(t-0.55)/0.45);}
    if v.point.y<i.rect.y+1.0 {c=mix(c,vec3f(1),i.p1.z);}if v.point.y>i.rect.y+i.rect.w-1.0 {c*=1.0-i.p1.w;}
    return paint(vec4f(c,i.color.a),coverage(v));
}
@fragment fn frosted(v:Vertex)->@location(0) vec4f {
    let i=items[v.index];let blurred=textureSampleLevel(backdrop,backdrop_sampler,v.point/view.xy,0.0);let c=blurred.rgb;let l=dot(c,vec3f(0.2126,0.7152,0.0722));let saturated=clamp(mix(vec3f(l),c,i.p0.z),vec3f(0),vec3f(1));
    let alpha=i.color.a+blurred.a*(1.0-i.color.a);let rgb=i.color.rgb*i.color.a+saturated*blurred.a*(1.0-i.color.a);let mask=coverage(v);return vec4f(rgb*mask,alpha*mask);
}
@vertex fn vs_blur(@builtin(vertex_index) vertex:u32,@builtin(instance_index) index:u32)->Vertex {
    let corners=array<vec2f,3>(vec2f(0,0),vec2f(2,0),vec2f(0,2));let p=corners[vertex]*view.xy;
    return Vertex(vec4f(p.x/view.x*2.0-1.0,1.0-p.y/view.y*2.0,0,1),p,index);
}
// CSS backdrop blur mirrors at the element border-box, not the viewport:
// https://drafts.csswg.org/filter-effects-2/#backdrop-filter-operation
fn mirror_box(point:vec2f,r:vec4f)->vec2f {
    let t=fract((point-r.xy)/r.zw*0.5)*2.0;
    return r.xy+select(t,vec2f(2)-t,t>vec2f(1))*r.zw;
}
fn gaussian(v:Vertex,axis:vec2f)->vec4f {
    let radius=items[v.index].p0.y;
    if radius<0.01 {return textureSampleLevel(backdrop,backdrop_sampler,v.point/view.xy,0.0);}
    let support=i32(ceil(radius*3.0));var sum=vec4f(0);var weight=0.0;
    for(var offset=-support;offset<=support;offset++) {let d=f32(offset);let w=exp(-0.5*d*d/(radius*radius));sum+=textureSampleLevel(backdrop,backdrop_sampler,mirror_box(v.point+axis*d,items[v.index].p2)/view.xy,0.0)*w;weight+=w;}
    return sum/max(weight,0.00001);
}
@fragment fn blur_x(v:Vertex)->@location(0) vec4f {return gaussian(v,vec2f(1,0));}
@fragment fn blur_y(v:Vertex)->@location(0) vec4f {return gaussian(v,vec2f(0,1));}
fn perimeter_point(t0:f32,r:vec4f)->vec2f {
    let perimeter=2.0*(r.z+r.w);let t=t0-floor(t0/perimeter)*perimeter;
    if t<r.z {return r.xy+vec2f(t,0);}if t<r.z+r.w{return r.xy+vec2f(r.z,t-r.z);}if t<2.0*r.z+r.w{return r.xy+vec2f(2.0*r.z+r.w-t,r.w);}return r.xy+vec2f(0,perimeter-t);
}
// The capture harness and shader share the same counter-hash random stream.
fn random(index:u32)->f32 {
    var x=index;x=(x^(x>>16u))*0x7feb352du;x=(x^(x>>15u))*0x846ca68bu;x=x^(x>>16u);
    return f32(x>>8u)/16777216.0;
}
@fragment fn glow(v:Vertex)->@location(0) vec4f {
    let i=items[v.index];if any(v.point<i.clip.xy)||any(v.point>=i.clip.xy+i.clip.zw){discard;}
    let d=rounded_sdf(v.point,i.rect,i.radii);if d>i.p2.w*4.0+1.0 {discard;}
    let state=u32(i.p2.x);let animated=state==1u||state==2u;let time=select(0.0,view.z,animated);
    var alpha=i.p3.x;
    if state==1u {alpha=mix(i.p3.y,i.p3.z,0.5-0.5*cos(time*6.2831853/max(i.p5.y,1.0)));}
    if state==2u {alpha=mix(i.p3.w,i.p4.x,0.5-0.5*cos(time*6.2831853/max(i.p5.z,1.0)));}
    if state==3u {alpha=i.p4.y;}
    alpha=min(1.0,alpha*mix(1.0,i.p2.z,i.p2.y));
    // CSS's fixed 45% outer halo; fill and border are not part of this effect.
    var strength=gaussian_box(v.point,i.rect,i.radii,i.p2.w*0.5)*clamp(0.5+d,0.0,1.0)*i.p0.z;
    let n=u32(i.p4.z);let perimeter=2.0*(i.p1.z+i.p1.w);var speed=0.5;
    if state==1u {speed=1.0;}if state==2u {speed=0.6;}if state==3u {speed=0.2;}
    for(var p=0u;p<48u;p++) {
        if p>=n {break;}
        let index=u32(i.p5.w)+p*4u;
        let u=random(index);let r=0.5+random(index+1u);let tw=random(index+2u)*1000.0;let life=0.4+random(index+3u)*0.6;
        let distance=fract(u+time*0.001*i.p4.w*speed*r/perimeter)*perimeter;
        let jitter=sin((time+tw)/700.0)*3.0*view.w;
        let centre=perimeter_point(distance,i.p1)+vec2f(jitter);
        let coverage=clamp(i.p5.x*r+0.5-length(v.point-centre),0.0,1.0);
        let particle=alpha*(0.6+0.4*sin((time+tw)/300.0))*life*coverage;
        strength=particle+(1.0-particle)*strength;
    }
    strength=clamp(strength*i.color.a,0.0,1.0);
    return vec4f(i.color.rgb*strength,strength);
}
