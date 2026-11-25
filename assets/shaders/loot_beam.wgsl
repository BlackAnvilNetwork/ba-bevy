// Loot beam shader: unlit, additive, HDR emissive, with pulse support.
// Ignores fog and renders with additive blending for bloom.

#import bevy_pbr::forward_io::VertexOutput
#import bevy_pbr::mesh_functions::get_world_from_local

struct LootBeamMaterial {
    color: vec4<f32>,
    hdr_multiplier: f32,
    opacity: f32,
    pulse_pos: f32,
    pulse_active: f32,
    ghost_pass: f32,
};

@group(2) @binding(0)
var<uniform> material: LootBeamMaterial;

struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
};

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;
    
    // Get world transform
    let world_from_local = get_world_from_local(vertex.instance_index);
    
    // Transform position to world space
    let world_position = world_from_local * vec4<f32>(vertex.position, 1.0);
    out.world_position = world_position;
    
    // Transform normal
    out.world_normal = normalize((world_from_local * vec4<f32>(vertex.normal, 0.0)).xyz);
    
    // Pass through UVs
    out.uv = vertex.uv;
    
    // Project to clip space
    #ifdef MULTIVIEW
    out.position = bevy_pbr::view_transformations::position_world_to_clip(world_position.xyz);
    #else
    out.position = bevy_pbr::view_transformations::position_world_to_clip(world_position.xyz);
    #endif
    
    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // Get the vertical position (UV.y goes from 0 at base to 1 at top)
    let height = in.uv.y;
    
    // Base color with HDR boost
    var color = material.color.rgb * material.hdr_multiplier;
    
    // Soft alpha falloff from center to edge (radial gradient)
    // For a cylinder, we use the normal to estimate radial position
    let radial = length(in.world_normal.xz);
    let edge_falloff = 1.0 - smoothstep(0.3, 1.0, radial);
    
    // Vertical fade: subtle falloff at top
    let top_fade = 1.0 - smoothstep(0.85, 1.0, height);
    
    // Pulse effect: a bright band traveling up the beam
    var pulse_intensity = 0.0;
    if material.pulse_active > 0.5 {
        let pulse_dist = abs(height - material.pulse_pos);
        pulse_intensity = smoothstep(0.15, 0.0, pulse_dist) * 0.5;
        color += vec3<f32>(pulse_intensity);
    }
    
    // Ghost pass has lower opacity and is always slightly visible
    var final_opacity = material.opacity * edge_falloff * top_fade;
    if material.ghost_pass > 0.5 {
        final_opacity *= 0.6;
    }
    
    // Shimmer effect using height-based noise
    let shimmer = 0.95 + 0.05 * sin(height * 20.0 + material.pulse_pos * 10.0);
    color *= shimmer;
    
    return vec4<f32>(color, final_opacity);
}
