struct Instance {rect:vec4f,fill:vec4f,stroke:vec4f,radii:vec4f,params:vec4f,uv:vec4f,clip:vec4f,clip_radii:vec4f}
@group(0) @binding(0) var<uniform> view:vec4f;
@group(0) @binding(1) var<storage,read> items:array<Instance>;
@group(0) @binding(2) var masks:texture_2d<f32>;
@group(0) @binding(3) var images:texture_2d<f32>;
@group(0) @binding(4) var text_backdrop:texture_2d<f32>;
@group(0) @binding(5) var image_sampler:sampler;
struct Vertex {@builtin(position) pos:vec4f,@location(0) point:vec2f,@location(1) @interpolate(flat) index:u32}
@vertex fn vs(@builtin(vertex_index) v:u32,@builtin(instance_index) i:u32)->Vertex {
    let corners=array<vec2f,6>(vec2f(0,0),vec2f(1,0),vec2f(0,1),vec2f(0,1),vec2f(1,0),vec2f(1,1));
    let item=items[i];let margin=select(1.0,item.params.z*3.0,item.params.x==4.0);
    let p=item.rect.xy-vec2f(margin)+corners[v]*(item.rect.zw+vec2f(margin*2.0));
    return Vertex(vec4f(p.x/view.x*2.0-1.0,1.0-p.y/view.y*2.0,0,1),p,i);
}
fn linear(c:vec3f)->vec3f {return select(c/12.92,pow((c+vec3f(0.055))/1.055,vec3f(2.4)),c>vec3f(0.04045));}
fn srgb1(c:f32)->f32 {return select(c*12.92,1.055*pow(max(c,0.0),1.0/2.4)-0.055,c>0.0031308);}
@fragment fn fs(v:Vertex)->@location(0) vec4f {
    let i=items[v.index];let p=v.point;
    if rounded_sdf(p,i.clip,i.clip_radii)>0.0 {discard;}
    let kind=u32(i.params.x);let d=rounded_sdf(p,i.rect,i.radii);var c=i.fill;var a=clamp(0.5-d,0.0,1.0);
    if kind==4u {let knockout=clamp(0.5+rounded_sdf(p,i.uv,i.stroke),0.0,1.0);return vec4f(c.rgb,c.a*gaussian_box(p,i.rect,i.radii,i.params.z)*knockout);}
    if kind==5u {let ab=i.uv.zw-i.uv.xy;let t=clamp(dot(p-i.uv.xy,ab)/max(dot(ab,ab),0.0001),0.0,1.0);a=clamp(0.5-(length(p-i.uv.xy-t*ab)-i.params.y*0.5),0.0,1.0);}
    if kind==1u {let t=clamp((p.y-i.rect.y)/i.rect.w,0.0,1.0);c=select(mix(i.fill,vec4f(1,1,1,i.fill.a),(1.0-t/0.55)*0.12),vec4f(i.fill.rgb*(1.0-(t-0.55)/0.45*0.18),i.fill.a),t>0.55);}
    if kind==2u||kind==3u {
        let unit=clamp((p-i.rect.xy)/max(i.rect.zw,vec2f(0.001)),vec2f(0),vec2f(0.99999));let pixel=vec2i(i.uv.xy+unit*i.uv.zw);
        if kind==2u {a*=textureLoad(masks,pixel,0).r;} else {
            let point=clamp(i.uv.xy+unit*i.uv.zw,i.uv.xy+vec2f(0.5),i.uv.xy+i.uv.zw-vec2f(0.5));
            let image=textureSampleLevel(images,image_sampler,point/vec2f(textureDimensions(images)),0.0);
            // Interpolate premultiplied RGB, then return straight colour to the
            // chrome source-over blend. Hidden RGB must not fringe the edge.
            c*=vec4f(image.rgb/max(image.a,0.000001),image.a);
        }
    }
    if i.params.y>0.0&&kind<2u {let edge=clamp(d+i.params.y+0.5,0.0,1.0);c=mix(c,i.stroke,edge);}
    if i.params.w>0.0&&p.y<i.rect.y+1.5&&d<0.0 {c=vec4f(mix(c.rgb,vec3f(1),i.params.w),c.a);}
    var alpha=c.a*a;
    if kind==2u {
        let fg=linear(c.rgb);let bg=linear(textureLoad(text_backdrop,vec2i(p),0).rgb);
        let weights=vec3f(0.2126,0.7152,0.0722);let lf=dot(fg,weights);let lb=dot(bg,weights);
        if abs(lf-lb)>0.001 {let wanted=linear(vec3f(srgb1(lf)*alpha+srgb1(lb)*(1.0-alpha))).x;alpha=clamp((wanted-lb)/(lf-lb),0.0,1.0);}
        return vec4f(fg,alpha);
    }
    return vec4f(c.rgb,alpha);
}
