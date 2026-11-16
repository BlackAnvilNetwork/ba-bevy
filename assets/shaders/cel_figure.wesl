import bevy_pbr::render::forward_io::VertexOutput;

struct CelUniforms {
    color: vec4<f32>,
    light_dir: vec4<f32>,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> mat: CelUniforms;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let n = normalize(in.world_normal);
    let l = normalize(mat.light_dir.xyz);
    let ndotl = dot(n, l);

    // Three hard bands. Cartoon, but the steps stay close so cloth doesn't posterize into candy.
    var band = 0.38;
    if (ndotl > 0.18) {
        band = 0.72;
    }
    if (ndotl > 0.55) {
        band = 1.05;
    }

    let albedo = mat.color.rgb;
    let cool_shadow = vec3<f32>(0.62, 0.68, 0.78);
    var rgb = albedo * mix(cool_shadow, vec3<f32>(1.0, 0.96, 0.9), band);

    // Warm spill from the basin fire, still stepped so it stays a cel edge.
    let fire = vec3<f32>(-2.05, 0.7, 0.35);
    let to_fire = fire - in.world_position.xyz;
    let dist = length(to_fire);
    let fire_ndot = max(dot(n, to_fire / max(dist, 0.001)), 0.0);
    let fire_att = max(1.0 - dist / 7.0, 0.0);
    var fire_band = 0.0;
    if (fire_ndot > 0.35) {
        fire_band = 1.0;
    }
    rgb = rgb + vec3<f32>(0.28, 0.1, 0.03) * fire_band * fire_att;

    return vec4<f32>(rgb, 1.0);
}
