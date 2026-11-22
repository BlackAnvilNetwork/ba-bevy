// Ground glow disc shader: radial gradient falloff for the beam's floor highlight.

#import bevy_pbr::forward_io::VertexOutput
#import bevy_pbr::mesh_functions::get_world_from_local

struct GroundGlowMaterial {
    color: vec4<f32>,
    intensity: f32,
};

@group(2) @binding(0)
var<uniform> material: GroundGlowMaterial;

struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
};

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;
    
    let world_from_local = get_world_from_local(vertex.instance_index);
    let world_position = world_from_local * vec4<f32>(vertex.position, 1.0);
    out.world_position = world_position;
    out.world_normal = normalize((world_from_local * vec4<f32>(vertex.normal, 0.0)).xyz);
    out.uv = vertex.uv;
    
    #ifdef MULTIVIEW
    out.position = bevy_pbr::view_transformations::position_world_to_clip(world_position.xyz);
    #else
    out.position = bevy_pbr::view_transformations::position_world_to_clip(world_position.xyz);
    #endif
    
    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // UVs are set up so center is (0.5, 0.5)
    let center = vec2<f32>(0.5, 0.5);
    let dist = length(in.uv - center) * 2.0; // 0 at center, 1 at edge
    
    // Smooth radial falloff
    let falloff = 1.0 - smoothstep(0.0, 1.0, dist);
    let soft_falloff = falloff * falloff; // Quadratic for softer edge
    
    // Color with HDR boost for bloom
    let color = material.color.rgb * material.intensity * 2.0;
    let alpha = soft_falloff * material.intensity * 0.7;
    
    return vec4<f32>(color, alpha);
}
