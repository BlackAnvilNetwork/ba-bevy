import bevy_pbr::render::forward_io::VertexOutput;

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> eye_time: vec4<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var<uniform> arches: array<vec4<f32>, 4>;

fn random_cell(p: vec2<f32>) -> f32 {
    let f = fract(vec3<f32>(p.xyx) * vec3<f32>(0.1031,0.1030,0.0973));
    let h = f + dot(f, f.yzx + 33.33);
    return fract((h.x + h.y)*h.z);
}

// Project each abbey arch into the mirrored sight line. Small surface waves
// break the silhouettes into bands while the reflection follows the camera.
fn arch_mirror(p: vec3<f32>, ray: vec3<f32>, center: vec3<f32>, wave: f32) -> f32 {
    if abs(ray.z)<0.001 {return 0.0;}
    let d=(center.y-p.z)/ray.z;
    if d<0.0 {return 0.0;}
    let height=(p.y+d*ray.y)/center.z;
    let side=abs((p.x+d*ray.x-center.x)/center.z+wave*0.06);
    if height<0.0 || height>7.48 {return 0.0;}
    let t=clamp((4.4-sqrt(max(0.01,19.36-3.4*(height-3.59))))/1.7,0.0,1.0);
    let edge=3.0*(1.0-t*t);
    return 1.0-smoothstep(0.40,0.64,abs(side-edge));
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let p=in.world_position.xyz;
    let uv=p.xz;
    let time=eye_time.w*0.43;
    let row=floor(uv.y*7.0+sin(uv.x*0.14)*0.6+time*0.3);
    let tint=random_cell(vec2<f32>(floor(uv.x*0.73+row*0.37),row));
    let swell=sin(uv.y*4.4+sin(uv.x*0.94+time)+time);
    let ray=normalize(reflect(p-eye_time.xyz,vec3<f32>(0.0,1.0,0.0))+vec3<f32>((tint-0.5)*0.014,swell*0.012,0.0));
    var arch = 0.0;
    for (var i = 0u; i < 4u; i++) {
        arch = max(arch, arch_mirror(p, ray, arches[i].xyz, swell));
    }
    let distance=length(eye_time.xyz-p);
    let haze=smoothstep(7.0,95.0,distance);
    var c=mix(vec3<f32>(0.026,0.045,0.066),vec3<f32>(0.116,0.106,0.154),haze);
    c=c+vec3<f32>(0.023,0.018,0.025)*(swell*0.55+tint);
    c=mix(c,vec3<f32>(0.023,0.032,0.045),arch*0.91);
    let line=smoothstep(0.78,0.96,swell)*smoothstep(0.49,0.85,tint)*(1.0-smoothstep(40.,98.,distance));
    c=c+vec3<f32>(0.031,0.034,0.054)*line;
    return vec4<f32>(c,1.0);
}
