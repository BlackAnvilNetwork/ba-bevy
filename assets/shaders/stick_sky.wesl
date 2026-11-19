// Port of ba-stick `sky::SkyCtx::shade` (day/dusk/clear path + sun + clouds).
import bevy_pbr::render::forward_io::VertexOutput;

struct StickSkyUniforms {
    phase: f32,
    time: f32,
    cover: f32,
    dark: f32,
    haze: f32,
    veil: f32,
    marsh: f32,
    _pad1: f32,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> mat: StickSkyUniforms;

const NIGHT_Z: vec3<f32> = vec3(7.0, 10.0, 26.0);
const NIGHT_H: vec3<f32> = vec3(28.0, 33.0, 64.0);
const TWI_Z: vec3<f32> = vec3(42.0, 42.0, 92.0);
const TWI_H: vec3<f32> = vec3(224.0, 120.0, 74.0);
const DAY_Z: vec3<f32> = vec3(58.0, 110.0, 200.0);
const DAY_H: vec3<f32> = vec3(168.0, 200.0, 232.0);
const STORM_Z: vec3<f32> = vec3(60.0, 64.0, 72.0);
const STORM_H: vec3<f32> = vec3(106.0, 110.0, 120.0);
const FOG_C: vec3<f32> = vec3(196.0, 202.0, 212.0);

const BAYER4: array<f32, 16> = array(
    0., 8., 2., 10., 12., 4., 14., 6., 3., 11., 1., 9., 15., 7., 13., 5.
);

fn mix3(a: vec3<f32>, b: vec3<f32>, t: f32) -> vec3<f32> {
    let t2 = clamp(t, 0.0, 1.0);
    return a + (b - a) * t2;
}

fn band(t: f32, steps: f32, x: u32, y: u32) -> f32 {
    let th = (BAYER4[(y & 3u) * 4u + (x & 3u)] + 0.5) / 16.0 - 0.5;
    return clamp(round(t * steps + th) / steps, 0.0, 1.0);
}

fn sun_dir(phase: f32) -> vec3<f32> {
    let a = (phase - 0.25) * 6.28318530718;
    let s = sin(a);
    let c = cos(a);
    return normalize(vec3(c, s * 0.92, s * 0.38));
}

fn daylight(phase: f32) -> f32 {
    let e = sun_dir(phase).y;
    return clamp((e + 0.18) / 0.36, 0.0, 1.0);
}

fn h2(x: i32, y: i32) -> f32 {
    var n = (bitcast<u32>(x) * 0x27d4eb2du) ^ (bitcast<u32>(y) * 0x165667b1u);
    n = n ^ (n >> 15u);
    n = n * 0x85ebca6bu;
    n = n ^ (n >> 13u);
    return f32(n & 0xffffu) / 65536.0;
}

fn vnoise(x: f32, y: f32) -> f32 {
    let xi = floor(x);
    let yi = floor(y);
    let fx = x - xi;
    let fy = y - yi;
    let sx = fx * fx * (3.0 - 2.0 * fx);
    let sy = fy * fy * (3.0 - 2.0 * fy);
    let i = i32(xi);
    let j = i32(yi);
    let a = h2(i, j) + (h2(i + 1, j) - h2(i, j)) * sx;
    let b = h2(i, j + 1) + (h2(i + 1, j + 1) - h2(i, j + 1)) * sx;
    return a + (b - a) * sy;
}

fn fbm(x: f32, y: f32, oct: u32) -> f32 {
    var a = 0.5;
    var f = 1.0;
    var t = 0.0;
    var n = 0.0;
    for (var i = 0u; i < oct; i++) {
        t += a * vnoise(x * f + f32(i) * 17.3, y * f - f32(i) * 9.1);
        n += a;
        a *= 0.5;
        f *= 2.03;
    }
    return t / n;
}

fn cloud_density(d: vec3<f32>, height: f32, scale: f32, speed: f32, seed: f32, time: f32, cover: f32) -> f32 {
    let k = height / max(d.y, 0.02);
    let u = d.x * k * scale + time * speed + seed;
    let v = d.z * k * scale;
    let base = fbm(u * 0.5, v * 0.5, 2u);
    let detail = fbm(u * 2.6 + 3.0, v * 2.6 - 2.0, 3u);
    return base * 0.6 + detail * 0.4 - (0.7 - cover * 0.52);
}

fn shade(d_in: vec3<f32>, px: u32, py: u32) -> vec3<f32> {
    let d = normalize(d_in);
    let sun = sun_dir(mat.phase);
    let day = daylight(mat.phase);
    let twi = clamp(1.0 - abs(sun.y / 0.28), 0.0, 1.0);
    let elev = max(d.y, -0.05);
    let up = band(sqrt(clamp(elev * 1.6, 0.0, 1.0)), 7.0, px, py);

    var zen = mix3(mix3(NIGHT_Z, DAY_Z, day), TWI_Z, twi * 0.5);
    var hor = mix3(mix3(NIGHT_H, DAY_H, day), TWI_H, twi);
    let sun_flat = normalize(vec3(sun.x, 0.0, sun.z));
    let toward = pow(dot(vec3(d.x, 0.0, d.z), sun_flat) * 0.5 + 0.5, 2.0);
    hor = mix3(mix3(NIGHT_H, DAY_H, day), hor, 0.35 + 0.65 * toward);
    var c = mix3(hor, zen, up);

    var storm = mix3(STORM_H, STORM_Z, up);
    storm = vec3(
        storm.x * (0.25 + 0.75 * day),
        storm.y * (0.25 + 0.75 * day),
        storm.z * (0.3 + 0.7 * day),
    );
    c = mix3(c, storm, mat.cover * mat.dark);

    let clear = 1.0 - mat.veil;

    // Sun disc + ring (stick exact thresholds)
    let sd = dot(d, sun);
    if sun.y > -0.1 {
        let warm = clamp(1.0 - sun.y / 0.4, 0.0, 1.0);
        let core = mix3(vec3(255.0, 250.0, 214.0), vec3(255.0, 168.0, 82.0), warm);
        let a = clear * (0.25 + 0.75 * (1.0 - mat.cover * 0.6));
        if sd > 0.9985 {
            c = mix3(c, core, a);
        } else if sd > 0.9965 {
            c = mix3(c, core, a * 0.45);
        } else if sd > 0.985 {
            c = mix3(c, core, band((sd - 0.985) / 0.0115, 3.0, px, py) * a * 0.22);
        }
    }

    // Moon opposite sun
    let moon = -sun;
    let md = dot(d, moon);
    if moon.y > -0.1 && md > 0.9988 {
        let a = clear * (1.0 - day);
        let crater = h2(i32((d.x - moon.x) * 900.0), i32((d.y - moon.y) * 900.0)) > 0.72;
        let mc = select(vec3(222.0, 226.0, 236.0), vec3(176.0, 182.0, 200.0), crater);
        c = mix3(c, mc, a);
    } else if moon.y > -0.1 && md > 0.996 {
        c = mix3(c, vec3(150.0, 160.0, 200.0), clear * (1.0 - day) * 0.25);
    }

    // Two cloud decks
    if d.y > 0.012 {
        let fade = clamp(d.y * 9.0, 0.0, 1.0);
        let sflat = normalize(vec3(sun.x, 0.0, sun.z));
        // back deck
        {
            let height = 1.0;
            let scale = 1.1;
            let speed = 0.010;
            let seed = 0.0;
            let raw = cloud_density(d, height, scale, speed, seed, mat.time, mat.cover);
            let dens = clamp(raw / 0.07, 0.0, 1.0);
            if dens > 0.0 {
                let off = 0.06;
                let toward_s = normalize(vec3(d.x + sflat.x * off, d.y + off * 1.3, d.z + sflat.z * off));
                let dl = cloud_density(toward_s, height, scale, speed, seed, mat.time, mat.cover);
                let light = clamp(0.62 - (dl - raw) * 7.0 - raw * 1.2, 0.0, 1.0);
                let edge = clamp(1.0 - dens, 0.0, 1.0);
                let base_day = vec3(214.0, 220.0, 236.0);
                var lit_c = mix3(mix3(vec3(52.0, 56.0, 80.0), base_day, day), vec3(255.0, 196.0, 150.0), twi * 0.75);
                var shd_c = mix3(mix3(vec3(30.0, 32.0, 52.0), vec3(150.0, 160.0, 186.0), day), vec3(150.0, 96.0, 120.0), twi * 0.7);
                let storm_c = vec3(74.0 * (0.4 + 0.6 * day), 78.0 * (0.4 + 0.6 * day), 88.0 * (0.45 + 0.55 * day));
                lit_c = mix3(lit_c, mix3(storm_c, vec3(170.0, 172.0, 180.0), 0.5), mat.dark);
                shd_c = mix3(shd_c, storm_c, mat.dark);
                let tone = band(light, 4.0, px, py);
                var col = mix3(shd_c, lit_c, tone);
                let face = pow(dot(d, sun) * 0.5 + 0.5, 3.0);
                let rim = band(edge * (0.3 + 0.7 * twi) * (0.4 + 0.6 * face) * (1.0 - mat.dark), 3.0, px, py);
                let rim_c = mix3(vec3(255.0, 252.0, 240.0), vec3(255.0, 170.0, 90.0), twi);
                col = mix3(col, rim_c, rim * 0.8 * max(day, twi));
                let thin = max(0.4 + 0.6 * max(day, twi), mat.cover * mat.dark);
                let a = band(min(dens * 1.6, 1.0) * thin, 4.0, px, py) * fade;
                c = mix3(c, col, a);
            }
        }
        // front deck
        {
            let height = 0.55;
            let scale = 1.6;
            let speed = 0.030;
            let seed = 41.0;
            let raw = cloud_density(d, height, scale, speed, seed, mat.time, mat.cover);
            let dens = clamp(raw / 0.07, 0.0, 1.0);
            if dens > 0.0 {
                let off = 0.06;
                let toward_s = normalize(vec3(d.x + sflat.x * off, d.y + off * 1.3, d.z + sflat.z * off));
                let dl = cloud_density(toward_s, height, scale, speed, seed, mat.time, mat.cover);
                let light = clamp(0.62 - (dl - raw) * 7.0 - raw * 1.2, 0.0, 1.0);
                let edge = clamp(1.0 - dens, 0.0, 1.0);
                let base_day = vec3(236.0, 238.0, 246.0);
                var lit_c = mix3(mix3(vec3(52.0, 56.0, 80.0), base_day, day), vec3(255.0, 196.0, 150.0), twi * 0.75);
                var shd_c = mix3(mix3(vec3(30.0, 32.0, 52.0), vec3(150.0, 160.0, 186.0), day), vec3(150.0, 96.0, 120.0), twi * 0.7);
                let storm_c = vec3(74.0 * (0.4 + 0.6 * day), 78.0 * (0.4 + 0.6 * day), 88.0 * (0.45 + 0.55 * day));
                lit_c = mix3(lit_c, mix3(storm_c, vec3(170.0, 172.0, 180.0), 0.5), mat.dark);
                shd_c = mix3(shd_c, storm_c, mat.dark);
                let tone = band(light, 4.0, px, py);
                var col = mix3(shd_c, lit_c, tone);
                let face = pow(dot(d, sun) * 0.5 + 0.5, 3.0);
                let rim = band(edge * (0.3 + 0.7 * twi) * (0.4 + 0.6 * face) * (1.0 - mat.dark), 3.0, px, py);
                let rim_c = mix3(vec3(255.0, 252.0, 240.0), vec3(255.0, 170.0, 90.0), twi);
                col = mix3(col, rim_c, rim * 0.8 * max(day, twi));
                let thin = max(0.4 + 0.6 * max(day, twi), mat.cover * mat.dark);
                let a = band(min(dens * 1.6, 1.0) * thin, 4.0, px, py) * fade;
                c = mix3(c, col, a);
            }
        }
    }

    if mat.haze > 0.0 {
        let f = vec3(
            FOG_C.x * (0.3 + 0.7 * day),
            FOG_C.y * (0.3 + 0.7 * day),
            FOG_C.z * (0.35 + 0.65 * day),
        );
        c = mix3(c, f, band(mat.haze * (1.0 - elev * 0.35), 4.0, px, py));
    }

    return c / 255.0;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // Dome is camera-centered; world_normal (outward) is the view ray.
    let dir = normalize(in.world_normal);
    let px = u32(in.position.x) & 1023u;
    let py = u32(in.position.y) & 1023u;
    let rgb = shade(dir, px, py);
    return vec4(rgb, 1.0);
}
