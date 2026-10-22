fn rounded_sdf(p:vec2f,r:vec4f,radii:vec4f)->f32 {
    let q=p-r.xy-r.zw*0.5;var radius=radii.x;
    if q.x>0.0 {radius=radii.y;} if q.y>0.0 {radius=select(radii.w,radii.z,q.x>0.0);}
    radius=min(radius,min(r.z,r.w)*0.5);let d=abs(q)-r.zw*0.5+vec2f(radius);
    return min(max(d.x,d.y),0.0)+length(max(d,vec2f(0)))-radius;
}
// Gaussian convolution of the rounded-rectangle mask, not exp(distance).
// Rectangles integrate analytically; rounded corners integrate horizontal CDFs.
fn erf_approx(x:f32)->f32 {
    let t=1.0/(1.0+0.3275911*abs(x));
    let poly=(((((1.061405429*t-1.453152027)*t)+1.421413741)*t-0.284496736)*t+0.254829592)*t;
    return sign(x)*(1.0-poly*exp(-x*x));
}
fn cdf(x:f32)->f32 {return 0.5+0.5*erf_approx(x*0.7071067812);}
fn gaussian_box(p:vec2f,r:vec4f,radii0:vec4f,sigma:f32)->f32 {
    if sigma<0.01 {return clamp(0.5-rounded_sdf(p,r,radii0),0.0,1.0);}
    let radii=min(radii0,vec4f(min(r.z,r.w)*0.5));
    if all(radii<vec4f(0.01)) {return (cdf((r.x+r.z-p.x)/sigma)-cdf((r.x-p.x)/sigma))*(cdf((r.y+r.w-p.y)/sigma)-cdf((r.y-p.y)/sigma));}
    let y0=max(r.y,p.y-3.5*sigma);let y1=min(r.y+r.w,p.y+3.5*sigma);
    if y1<=y0 {return 0.0;}
    let step=(y1-y0)/32.0;var total=0.0;
    for(var n=0u;n<32u;n++) {
        let y=y0+(f32(n)+0.5)*step;var left=r.x;var right=r.x+r.z;
        if y<r.y+radii.x {let dy=r.y+radii.x-y;left+=radii.x-sqrt(max(radii.x*radii.x-dy*dy,0.0));}
        if y>r.y+r.w-radii.w {let dy=y-(r.y+r.w-radii.w);left+=radii.w-sqrt(max(radii.w*radii.w-dy*dy,0.0));}
        if y<r.y+radii.y {let dy=r.y+radii.y-y;right-=radii.y-sqrt(max(radii.y*radii.y-dy*dy,0.0));}
        if y>r.y+r.w-radii.z {let dy=y-(r.y+r.w-radii.z);right-=radii.z-sqrt(max(radii.z*radii.z-dy*dy,0.0));}
        let dy=(y-p.y)/sigma;
        total+=(cdf((right-p.x)/sigma)-cdf((left-p.x)/sigma))*exp(-0.5*dy*dy)*0.3989422804/sigma*step;
    }
    return total;
}
