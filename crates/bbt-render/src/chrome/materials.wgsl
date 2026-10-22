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
fn linear(c:vec3f)->vec3f {return select(c/12.92,pow((c+vec3f(0.055))/1.055,vec3f(2.4)),c>vec3f(0.04045));}
fn srgb(c:vec3f)->vec3f {return select(c*12.92,1.055*pow(max(c,vec3f(0)),vec3f(1.0/2.4))-vec3f(0.055),c>vec3f(0.0031308));}
fn sdf(p:vec2f,i:Instance)->f32 {
    let q=p-i.rect.xy-i.rect.zw*0.5;var r=i.radii.x;if q.x>0.0 {r=i.radii.y;}if q.y>0.0 {r=select(i.radii.w,i.radii.z,q.x>0.0);}r=min(r,min(i.rect.z,i.rect.w)*0.5);let d=abs(q)-i.rect.zw*0.5+vec2f(r);return min(max(d.x,d.y),0.0)+length(max(d,vec2f(0)))-r;
}
fn coverage(v:Vertex)->f32 {let i=items[v.index];if any(v.point<i.clip.xy)||any(v.point>=i.clip.xy+i.clip.zw){discard;}return clamp(0.5-sdf(v.point,i),0.0,1.0);}
fn paint(c:vec4f,a:f32)->vec4f {let alpha=c.a*a;return vec4f(linear(clamp(c.rgb,vec3f(0),vec3f(1)))*alpha,alpha);}
@fragment fn flat(v:Vertex)->@location(0) vec4f {return paint(items[v.index].color,coverage(v));}
@fragment fn gloss(v:Vertex)->@location(0) vec4f {
    let i=items[v.index];let t=clamp((v.point.y-i.rect.y)/i.rect.w,0.0,1.0);var c=i.color.rgb;
    if t<=0.55 {c=mix(mix(c,vec3f(1),i.p1.x),c,t/0.55);}else{c=mix(c,c*(1.0-i.p1.y),(t-0.55)/0.45);}
    if v.point.y<i.rect.y+1.0 {c=mix(c,vec3f(1),i.p1.z);}if v.point.y>i.rect.y+i.rect.w-1.0 {c*=1.0-i.p1.w;}
    return paint(vec4f(c,i.color.a),coverage(v));
}
@fragment fn frosted(v:Vertex)->@location(0) vec4f {
    let i=items[v.index];let blurred=textureSampleLevel(backdrop,backdrop_sampler,v.point/view.xy,0.0);let c=srgb(blurred.rgb);let l=dot(c,vec3f(0.2126,0.7152,0.0722));let saturated=linear(clamp(mix(vec3f(l),c,i.p0.z),vec3f(0),vec3f(1)));
    let alpha=i.color.a+blurred.a*(1.0-i.color.a);let rgb=linear(i.color.rgb)*i.color.a+saturated*blurred.a*(1.0-i.color.a);let mask=coverage(v);return vec4f(rgb*mask,alpha*mask);
}
@vertex fn vs_blur(@builtin(vertex_index) vertex:u32,@builtin(instance_index) index:u32)->Vertex {
    let corners=array<vec2f,3>(vec2f(0,0),vec2f(2,0),vec2f(0,2));let p=corners[vertex]*view.xy;
    return Vertex(vec4f(p.x/view.x*2.0-1.0,1.0-p.y/view.y*2.0,0,1),p,index);
}
fn gaussian(v:Vertex,axis:vec2f)->vec4f {
    let radius=items[v.index].p0.y;
    if radius<0.01 {return textureSampleLevel(backdrop,backdrop_sampler,v.point/view.xy,0.0);}
    let support=i32(ceil(radius*3.0));var sum=vec4f(0);var weight=0.0;
    for(var offset=-support;offset<=support;offset++) {let d=f32(offset);let w=exp(-0.5*d*d/(radius*radius));sum+=textureSampleLevel(backdrop,backdrop_sampler,(v.point+axis*d)/view.xy,0.0)*w;weight+=w;}
    return sum/max(weight,0.00001);
}
@fragment fn blur_x(v:Vertex)->@location(0) vec4f {return gaussian(v,vec2f(1,0));}
@fragment fn blur_y(v:Vertex)->@location(0) vec4f {return gaussian(v,vec2f(0,1));}
fn perimeter_point(t0:f32,r:vec4f)->vec2f {
    let perimeter=2.0*(r.z+r.w);let t=t0-floor(t0/perimeter)*perimeter;
    if t<r.z {return r.xy+vec2f(t,0);}if t<r.z+r.w{return r.xy+vec2f(r.z,t-r.z);}if t<2.0*r.z+r.w{return r.xy+vec2f(2.0*r.z+r.w-t,r.w);}return r.xy+vec2f(0,perimeter-t);
}
fn rounded_perimeter_point(t:f32,i:Instance)->vec2f {
    let p=perimeter_point(t,i.rect);let q=p-i.rect.xy;var centre=vec2f(0);var radius=0.0;
    let radii=min(i.radii,vec4f(min(i.rect.z,i.rect.w)*0.5));
    if q.x<radii.x&&q.y<radii.x {radius=radii.x;centre=i.rect.xy+vec2f(radius);}
    if q.x>i.rect.z-radii.y&&q.y<radii.y {radius=radii.y;centre=i.rect.xy+vec2f(i.rect.z-radius,radius);}
    if q.x>i.rect.z-radii.z&&q.y>i.rect.w-radii.z {radius=radii.z;centre=i.rect.xy+i.rect.zw-vec2f(radius);}
    if q.x<radii.w&&q.y>i.rect.w-radii.w {radius=radii.w;centre=i.rect.xy+vec2f(radius,i.rect.w-radius);}
    if radius>0.0{return centre+normalize(p-centre)*radius;}return p;
}
@fragment fn glow(v:Vertex)->@location(0) vec4f {
    let i=items[v.index];if any(v.point<i.clip.xy)||any(v.point>=i.clip.xy+i.clip.zw){discard;}
    let distance=abs(sdf(v.point,i));if distance>i.p2.w*4.0+1.0 {discard;}
    let state=u32(i.p2.x);var alpha=i.p3.x;
    if state==1u {alpha=mix(i.p3.y,i.p3.z,0.5-0.5*cos(view.z*6.2831853/max(i.p5.y,1.0)));}
    if state==2u {alpha=mix(i.p3.w,i.p4.x,0.5-0.5*cos(view.z*6.2831853/max(i.p5.z,1.0)));}
    if state==3u {alpha=i.p4.y;}
    alpha*=mix(1.0,i.p2.z,i.p2.y);var power=exp(-distance/max(i.p2.w,0.01))*alpha*(1.0-smoothstep(i.p2.w*3.0,max(i.p2.w*4.0,0.01),distance));
    let n=u32(i.p4.z);let perimeter=2.0*(i.rect.z+i.rect.w);let moving=select(0.0,view.z*0.001*i.p4.w,state==1u||state==2u);
    for(var p=0u;p<48u;p++){if p>=n {break;}let t=(f32(p)/max(f32(n),1.0)+fract(sin(f32(p)*17.3+i.p5.w)*43758.5453))*perimeter+moving;let delta=v.point-rounded_perimeter_point(t,i);power+=exp(-dot(delta,delta)/max(2.0*i.p5.x*i.p5.x,0.01))*alpha*0.7;}
    let strength=clamp(power*i.color.a,0.0,1.0);return vec4f(linear(i.color.rgb)*strength,strength);
}
