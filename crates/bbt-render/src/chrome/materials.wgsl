struct Instance {rect:vec4f,color:vec4f,radii:vec4f,p0:vec4f,p1:vec4f,p2:vec4f,p3:vec4f,p4:vec4f,p5:vec4f,clip:vec4f,clip_range:vec4u}
@group(0) @binding(0) var<uniform> view:vec4f;
@group(0) @binding(1) var<storage,read> items:array<Instance>;
@group(0) @binding(2) var backdrop:texture_2d<f32>;
@group(0) @binding(3) var backdrop_sampler:sampler;
struct AncestorClip {rect:vec4f,radii:vec4f}
@group(0) @binding(4) var<storage,read> ancestor_clips:array<AncestorClip>;
struct Vertex {@builtin(position) position:vec4f,@location(0) point:vec2f,@location(1) @interpolate(flat) index:u32}
@vertex fn vs(@builtin(vertex_index) vertex:u32,@builtin(instance_index) index:u32)->Vertex {
    let corners=array<vec2f,6>(vec2f(0,0),vec2f(1,0),vec2f(0,1),vec2f(0,1),vec2f(1,0),vec2f(1,1));let i=items[index];let margin=i.p0.w+1.0;
    let p=i.rect.xy-vec2f(margin)+corners[vertex]*(i.rect.zw+vec2f(margin*2.0));return Vertex(vec4f(p.x/view.x*2.0-1.0,1.0-p.y/view.y*2.0,0,1),p,index);
}
fn clip_mask(v:Vertex)->f32 {
    let i=items[v.index];
    if any(v.point<i.clip.xy)||any(v.point>=i.clip.xy+i.clip.zw){return 0.0;}
    var mask=1.0;
    for(var n=0u;n<i.clip_range.y;n++) {
        let c=ancestor_clips[i.clip_range.x+n];
        if any(v.point<c.rect.xy)||any(v.point>=c.rect.xy+c.rect.zw){return 0.0;}
        // Preserve the existing hard rectangular clip when no corner is rounded.
        if any(c.radii>vec4f(0)) {mask=min(mask,clamp(0.5-rounded_sdf(v.point,c.rect,c.radii),0.0,1.0));}
    }
    return mask;
}
fn coverage(v:Vertex)->f32 {let mask=clip_mask(v);if mask<=0.0 {discard;}let i=items[v.index];return min(mask,clamp(0.5-rounded_sdf(v.point,i.rect,i.radii),0.0,1.0));}
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
    let i=items[v.index];let mask=clip_mask(v);if mask<=0.0 {discard;}
    let d=rounded_sdf(v.point,i.rect,i.radii);if d>i.p2.w*4.0+1.0 {discard;}
    let n=u32(i.p4.z);
    // Halo strength is exactly zero inside its knockout. Particle centres lie
    // on the unsnapped rectangular perimeter; each axis jitters by at most
    // 3*scale, with radius below 1.5*size plus 0.5 coverage. The extra pixel is
    // conservative for fractional coordinates and edge arithmetic.
    let reach=1.5*i.p5.x+3.0*view.w+1.5;
    let inward=min(v.point-i.p1.xy,i.p1.xy+i.p1.zw-v.point);
    if d<=-0.5&&(n==0u||all(inward>vec2f(reach))) {return vec4f(0);}
    let state=u32(i.p2.x);let animated=state==1u||state==2u;let time=select(0.0,view.z,animated);
    var alpha=i.p3.x;
    if state==1u {alpha=mix(i.p3.y,i.p3.z,0.5-0.5*cos(time*6.2831853/max(i.p5.y,1.0)));}
    if state==2u {alpha=mix(i.p3.w,i.p4.x,0.5-0.5*cos(time*6.2831853/max(i.p5.z,1.0)));}
    if state==3u {alpha=i.p4.y;}
    alpha=min(1.0,alpha*mix(1.0,i.p2.z,i.p2.y));
    // CSS's fixed 45% outer halo; fill and border are not part of this effect.
    var strength=gaussian_box(v.point,i.rect,i.radii,i.p2.w*0.5)*clamp(0.5+d,0.0,1.0)*i.p0.z;
    let perimeter=2.0*(i.p1.z+i.p1.w);var speed=0.5;
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
    strength=clamp(strength*i.color.a,0.0,1.0)*mask;
    return vec4f(i.color.rgb*strength,strength);
}
// Border lane (G1-G3): a 1:1 port of Wren's lab/web/shaders.html. All distances and
// wavelengths are CSS px (physical / view.w); u runs 0 at the box edge to 1 at the
// lane edge; s is the arc length clockwise from the start of the top edge. GLSL
// pow(x, 2.) is written x*x (GLSL leaves pow undefined for x < 0).
const LANE_TAU=6.2831853;
fn lane_perim_len(hs:vec2f,r0:f32)->f32 {let r=max(r0,0.001);let b=hs-r;return 4.0*(b.x+b.y)+LANE_TAU*r;}
// Continuous through the corners: lane pixels beyond a corner take the corner arc's angle.
fn lane_perim(q:vec2f,hs:vec2f,r0:f32)->f32 {
    let r=max(r0,0.001);let b=hs-r;let sx=2.0*b.x;let sy=2.0*b.y;let a=0.25*LANE_TAU*r;
    if abs(q.x)<=b.x {return select(sx+2.0*a+sy+(b.x-q.x),q.x+b.x,q.y<0.0);}
    if abs(q.y)<=b.y {return select(2.0*sx+3.0*a+sy+(b.y-q.y),sx+a+(q.y+b.y),q.x>0.0);}
    if q.x>0.0&&q.y<0.0 {let v=q-vec2f(b.x,-b.y);return sx+r*atan2(v.x,-v.y);}
    if q.x>0.0 {let v=q-vec2f(b.x,b.y);return sx+a+sy+r*atan2(v.y,v.x);}
    if q.y>0.0 {let v=q-vec2f(-b.x,b.y);return 2.0*sx+2.0*a+sy+r*atan2(-v.x,v.y);}
    let v=q-vec2f(-b.x,-b.y);return 2.0*sx+3.0*a+2.0*sy+r*atan2(-v.y,-v.x);
}
// Wavelength snapped so a whole number of waves fits the perimeter: the arc
// coordinate wraps P -> 0 at the top-left without a seam (lab 8e90632). floor(x+.5)
// rounds half up like the lab; WGSL round() is half-to-even.
fn lane_wl(lambda:f32,p:f32)->f32 {return p/max(1.0,floor(p/lambda+0.5));}
fn lane_fx(fx:u32,u:f32,s:f32,p:f32,t:f32,amp:f32)->f32 {
    if fx==0u { // silk: tide's thin moving core over swell's faint rolling bloom
        let c0=0.40+0.14*sin(LANE_TAU*(s/lane_wl(190.0,p)-t*0.31))+0.07*sin(LANE_TAU*(s/lane_wl(73.0,p)+t*0.43)+1.3);
        let k=(u-c0)/0.16;let core=exp(-(k*k));
        let crest=0.5+0.5*sin(LANE_TAU*(s/lane_wl(260.0,p)-t*0.22));
        let bloom=exp(-u*2.2)*(0.15+0.55*crest*crest);
        return amp*(core*(0.55+0.45*crest)+bloom);
    }
    if fx==1u { // swell: the band hugs the edge; brightness crests roll along it
        let crest=0.5+0.5*sin(LANE_TAU*(s/lane_wl(230.0,p)-t*0.28));
        return amp*exp(-u*2.4)*(0.22+0.78*crest*crest);
    }
    if fx==2u { // tide: the ribbon displaced by three incommensurate sines
        let c0=0.44+0.15*sin(LANE_TAU*(s/lane_wl(190.0,p)-t*0.31))+0.08*sin(LANE_TAU*(s/lane_wl(71.0,p)+t*0.47)+1.3)+0.04*sin(LANE_TAU*(s/lane_wl(31.0,p)-t*0.83)+2.1);
        let w=0.22+0.05*sin(LANE_TAU*(s/lane_wl(260.0,p)+t*0.2));let k=(u-c0)/w;
        return amp*exp(-(k*k));
    }
    // breathe (control): today's uniform breathing glow, confined to the lane
    return amp*exp(-u*2.0)*(0.55+0.45*(0.5-0.5*cos(t*LANE_TAU/1.8)));
}
@fragment fn lane(v:Vertex)->@location(0) vec4f {
    let i=items[v.index];let mask=clip_mask(v);if mask<=0.0 {discard;}
    let k=view.w;let lane=i.p1.w/k;let d=rounded_sdf(v.point,i.rect,i.radii)/k;
    let in_lane=smoothstep(0.0,0.6,d)*(1.0-smoothstep(lane-0.6,lane,d));  // the effect lives only in [0, L] outside
    if in_lane<=0.0 {discard;}
    let hs=i.rect.zw*0.5/k;let q=(v.point-i.rect.xy)/k-hs;let r=i.radii.x/k;
    let s=lane_perim(q,hs,r);let perimeter=lane_perim_len(hs,r);let u=clamp(d/lane,0.0,1.0);
    let act=u32(i.p1.y);
    var spd=0.0;var amp=0.65;
    if act==0u {spd=0.35;amp=0.32;} if act==1u {spd=1.0;amp=1.0;} if act==2u {spd=0.6;amp=0.9;}
    let time=view.z+i.p2.x;let t=time*0.001*spd;
    var intensity=lane_fx(u32(i.p1.x),u,s,perimeter,t,amp)*in_lane*mix(1.0,1.3,i.p1.z);
    if act==2u {intensity*=0.6+0.4*(0.5-0.5*cos(time/1600.0*LANE_TAU));}  // waiting breathes on top
    intensity=clamp(intensity,0.0,1.0)*i.color.a*mask;
    return vec4f(i.color.rgb*intensity,intensity);
}
