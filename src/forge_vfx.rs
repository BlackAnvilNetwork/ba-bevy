//! Content-aware forge pixel pass — port of ba-stick `boot-menu-photo-pixels.ts`.
//!
//! Full-screen haze + dense embers driven by hot colors in the forge backdrop.
//! Hot lava/ember regions move faster (rising embers); warmer/smoky haze drifts slower.

use bevy::asset::RenderAssetUsages;
use bevy::image::{Image, ImageSampler};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};

use crate::forge_menu::GameFlow;

const PATCH: u32 = 7;
const MIN_HEAT: f32 = 0.06;

const DENSITY_WARM: f32 = 0.85;
const DENSITY_HOT: f32 = 1.0;
const HAZE_LAYERS: usize = 3;
const HAZE_HZ: f32 = 30.0;
const ALPHA_WARM: f32 = 0.3;
const ALPHA_HOT: f32 = 0.46;
const CYCLE: f32 = 8.0;
const RISE: f32 = 28.0;
const FLOW_CELL: f32 = 24.0;

const EMBER_RATE: f32 = 3400.0;
const EMBER_MAX: usize = 12_000;
const EMBER_LIFE: (f32, f32) = (3.0, 9.0);
const EMBER_RISE: (f32, f32) = (10.0, 36.0);
const BIG_CHANCE: f32 = 0.16;
const BREEZE: f32 = 9.0;

const GUST_EVERY: (f32, f32) = (1.2, 3.5);
const GUST_MAX: usize = 3;
const GUST_DURATION: (f32, f32) = (2.2, 4.5);
const GUST_RADIUS: (f32, f32) = (0.06, 0.12);
const GUST_SWIRL: (f32, f32) = (28.0, 58.0);
const GUST_PUSH: (f32, f32) = (-8.0, 22.0);

const BACKDROP_PATH: &str = "ui/boot/forge-backdrop.png";

#[derive(Component)]
pub struct ForgeVfxLayer;

struct Gust {
    x: f32,
    y: f32,
    age: f32,
    dur: f32,
    r: f32,
    spin: f32,
    swirl: f32,
    push: f32,
}

struct EmberPool {
    n: usize,
    x: Vec<f32>,
    y: Vec<f32>,
    vx: Vec<f32>,
    vy: Vec<f32>,
    age: Vec<f32>,
    life: Vec<f32>,
    phase: Vec<f32>,
    size: Vec<u8>,
    r: Vec<u8>,
    g: Vec<u8>,
    b: Vec<u8>,
}

impl EmberPool {
    fn new(max: usize) -> Self {
        Self {
            n: 0,
            x: vec![0.0; max],
            y: vec![0.0; max],
            vx: vec![0.0; max],
            vy: vec![0.0; max],
            age: vec![0.0; max],
            life: vec![1.0; max],
            phase: vec![0.0; max],
            size: vec![1; max],
            r: vec![0; max],
            g: vec![0; max],
            b: vec![0; max],
        }
    }

    fn remove(&mut self, i: usize) {
        let l = self.n - 1;
        if i != l {
            self.x[i] = self.x[l];
            self.y[i] = self.y[l];
            self.vx[i] = self.vx[l];
            self.vy[i] = self.vy[l];
            self.age[i] = self.age[l];
            self.life[i] = self.life[l];
            self.phase[i] = self.phase[l];
            self.size[i] = self.size[l];
            self.r[i] = self.r[l];
            self.g[i] = self.g[l];
            self.b[i] = self.b[l];
        }
        self.n = l;
    }
}

struct ForgeCache {
    w: u32,
    h: u32,
    s: f32,
    n: usize,
    xs: Vec<u16>,
    ys: Vec<u16>,
    rgb: Vec<u8>,
    phase: Vec<f32>,
    alpha: Vec<f32>,
    hot: Vec<i32>,
    gcell: u32,
    gw: u32,
    gh: u32,
    flow: Vec<f32>,
    haze: Vec<u8>,
    embers: Vec<u8>,
    mask: Vec<u8>,
}

#[derive(Resource)]
pub struct ForgeVfx {
    pub image: Handle<Image>,
    backdrop: Handle<Image>,
    primed: bool,
    clock: f32,
    haze_acc: f32,
    haze_ready: bool,
    spawn_acc: f32,
    next_gust: f32,
    cache: Option<ForgeCache>,
    ep: EmberPool,
    gusts: Vec<Gust>,
    rng: SmallRng,
}

fn ihash(x: i32, y: i32, seed: i32) -> f32 {
    let mut h = (x as u32)
        .wrapping_mul(374761393)
        .wrapping_add((y as u32).wrapping_mul(668265263))
        .wrapping_add((seed as u32).wrapping_mul(2246822519));
    h = (h ^ (h >> 13)).wrapping_mul(1274126177);
    h ^= h >> 16;
    h as f32 / 4294967296.0
}

fn lum(r: f32, g: f32, b: f32) -> f32 {
    r * 0.299 + g * 0.587 + b * 0.114
}

fn is_heat(r: f32, g: f32, b: f32) -> bool {
    r > 82.0 && g > 28.0 && b < 118.0 && r >= g * 0.82
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn smooth01(t: f32) -> f32 {
    let c = t.clamp(0.0, 1.0);
    c * c * (3.0 - 2.0 * c)
}

fn sample_rgb(data: &[u8], w: u32, h: u32, px: f32, py: f32) -> [f32; 3] {
    let x = px.round().clamp(0.0, (w - 1) as f32) as u32;
    let y = py.round().clamp(0.0, (h - 1) as f32) as u32;
    let i = ((y * w + x) * 4) as usize;
    [data[i] as f32, data[i + 1] as f32, data[i + 2] as f32]
}

fn neighbor_radius(w: u32) -> f32 {
    (w as f32 / 28.0).round().max(3.0)
}

fn sample_smooth_color(data: &[u8], w: u32, h: u32, px: f32, py: f32) -> [f32; 3] {
    let r = neighbor_radius(w);
    let taps = [(0.0, 0.0, 0.45), (r, 0.0, 0.14), (-r, 0.0, 0.14), (0.0, r, 0.14), (0.0, -r, 0.14)];
    let mut tr = 0.0;
    let mut tg = 0.0;
    let mut tb = 0.0;
    let mut tw = 0.0;
    for (dx, dy, wt) in taps {
        let [cr, cg, cb] = sample_rgb(data, w, h, px + dx, py + dy);
        tr += cr * wt;
        tg += cg * wt;
        tb += cb * wt;
        tw += wt;
    }
    [tr / tw, tg / tw, tb / tw]
}

fn heat_weight(data: &[u8], w: u32, h: u32, px: f32, py: f32) -> f32 {
    let d = neighbor_radius(w);
    let mut heat = 0.0;
    for (dx, dy) in [(0.0, 0.0), (d, 0.0), (-d, 0.0), (0.0, d), (0.0, -d)] {
        let [r, g, b] = sample_rgb(data, w, h, px + dx, py + dy);
        if is_heat(r, g, b) {
            heat += lum(r, g, b) / 255.0;
        }
    }
    heat / 5.0
}

fn box_blur(src: &[u8], w: u32, h: u32, radius: i32) -> Vec<u8> {
    let n = (w * h) as usize;
    let mut tmp = vec![0u8; n];
    let mut out = vec![0u8; n];
    let d = (2 * radius + 1) as f32;
    let wi = w as i32;
    let hi = h as i32;
    for y in 0..hi {
        let row = y * wi;
        let mut sum = 0i32;
        for x in -radius..=radius {
            sum += src[(row + x.clamp(0, wi - 1)) as usize] as i32;
        }
        for x in 0..wi {
            tmp[(row + x) as usize] = (sum as f32 / d) as u8;
            sum += src[(row + (x + radius + 1).min(wi - 1)) as usize] as i32
                - src[(row + (x - radius).max(0)) as usize] as i32;
        }
    }
    for x in 0..wi {
        let mut sum = 0i32;
        for y in -radius..=radius {
            sum += tmp[(y.clamp(0, hi - 1) * wi + x) as usize] as i32;
        }
        for y in 0..hi {
            out[(y * wi + x) as usize] = (sum as f32 / d) as u8;
            sum += tmp[((y + radius + 1).min(hi - 1) * wi + x) as usize] as i32
                - tmp[((y - radius).max(0) * wi + x) as usize] as i32;
        }
    }
    out
}

fn box_min(src: &[u8], w: u32, h: u32, radius: i32) -> Vec<u8> {
    let wi = w as i32;
    let hi = h as i32;
    let mut tmp = vec![0u8; (w * h) as usize];
    let mut out = vec![0u8; (w * h) as usize];
    for y in 0..hi {
        let row = y * wi;
        for x in 0..wi {
            let mut m = 255u8;
            let x0 = (x - radius).max(0);
            let x1 = (x + radius).min(wi - 1);
            for k in x0..=x1 {
                let v = src[(row + k) as usize];
                if v < m {
                    m = v;
                }
                if m == 0 {
                    break;
                }
            }
            tmp[(row + x) as usize] = m;
        }
    }
    for y in 0..hi {
        let y0 = (y - radius).max(0);
        let y1 = (y + radius).min(hi - 1);
        for x in 0..wi {
            let mut m = 255u8;
            for k in y0..=y1 {
                let v = tmp[(k * wi + x) as usize];
                if v < m {
                    m = v;
                }
                if m == 0 {
                    break;
                }
            }
            out[(y * wi + x) as usize] = m;
        }
    }
    out
}

fn build_heat_mask(
    data: &[u8],
    w: u32,
    h: u32,
    r_min: f32,
    r_range: f32,
    sat_min: f32,
    sat_range: f32,
    erode_r: i32,
) -> Vec<u8> {
    let mut raw = vec![0u8; (w * h) as usize];
    for i in 0..raw.len() {
        let p = i * 4;
        let r = data[p] as f32;
        let g = data[p + 1] as f32;
        let b = data[p + 2] as f32;
        if r < r_min || g > r / 0.97 {
            continue;
        }
        let sat = (r - b) / (r + 1.0);
        let wr = ((r - r_min) / r_range).clamp(0.0, 1.0);
        let ws = ((sat - sat_min) / sat_range).clamp(0.0, 1.0);
        raw[i] = (wr * ws * 255.0) as u8;
    }
    let a = erode_r >> 1;
    let eroded = box_min(&box_min(&raw, w, h, a), w, h, erode_r - a);
    box_blur(&eroded, w, h, (erode_r >> 2).max(1))
}

fn make_blank(w: u32, h: u32) -> Image {
    let mut image = Image::new_fill(
        Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 0],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    image.sampler = ImageSampler::nearest();
    image
}

fn build_cache(data: &[u8], w: u32, h: u32) -> ForgeCache {
    let s = (h as f32 / 940.0).clamp(0.6, 2.2);
    let haze_mask = build_heat_mask(
        data,
        w,
        h,
        90.0,
        50.0,
        0.33,
        0.2,
        ((w as f32 / 190.0).round() as i32).max(4),
    );
    let ember_mask = build_heat_mask(
        data,
        w,
        h,
        125.0,
        50.0,
        0.45,
        0.15,
        ((w as f32 / 120.0).round() as i32).max(4),
    );

    let pw = ((w + PATCH - 1) / PATCH) as usize;
    let ph = ((h + PATCH - 1) / PATCH) as usize;
    let mut heat = vec![0.0f32; pw * ph];
    for cy in 0..ph {
        for cx in 0..pw {
            heat[cy * pw + cx] = heat_weight(
                data,
                w,
                h,
                cx as f32 * PATCH as f32 + PATCH as f32 * 0.5,
                cy as f32 * PATCH as f32 + PATCH as f32 * 0.5,
            );
        }
    }

    let hw_at = |x: f32, y: f32| -> f32 {
        let fx = (x / PATCH as f32 - 0.5).clamp(0.0, pw as f32 - 1.001);
        let fy = (y / PATCH as f32 - 0.5).clamp(0.0, ph as f32 - 1.001);
        let ix = fx as usize;
        let iy = fy as usize;
        let tx = fx - ix as f32;
        let ty = fy - iy as f32;
        let a = heat[iy * pw + ix];
        let b = heat[iy * pw + (ix + 1).min(pw - 1)];
        let c = heat[(iy + 1).min(ph - 1) * pw + ix];
        let d = heat[(iy + 1).min(ph - 1) * pw + (ix + 1).min(pw - 1)];
        lerp(lerp(a, b, tx), lerp(c, d, tx), ty)
    };

    let mut xs = Vec::new();
    let mut ys = Vec::new();
    let mut rgb = Vec::new();
    let mut phase = Vec::new();
    let mut alpha = Vec::new();
    let mut hot = Vec::new();

    for y in 0..h {
        for x in 0..w {
            let hw = hw_at(x as f32, y as f32);
            if hw < MIN_HEAT {
                continue;
            }
            let hm = haze_mask[(y * w + x) as usize];
            if hm < 50 {
                continue;
            }
            let fade = ((hm as f32 - 50.0) / 80.0).min(1.0);
            let k = smooth01((hw - 0.12) / 0.22);
            let dens = lerp(DENSITY_WARM, DENSITY_HOT, k);
            let mut col: Option<[f32; 3]> = None;
            for layer in 0..HAZE_LAYERS {
                if ihash(x as i32, y as i32, 1 + layer as i32 * 7) > dens {
                    continue;
                }
                let c = match col {
                    Some(c) => c,
                    None => {
                        let sampled = sample_smooth_color(data, w, h, x as f32 + 0.5, y as f32 + 0.5);
                        if !is_heat(sampled[0], sampled[1], sampled[2]) {
                            break;
                        }
                        col = Some(sampled);
                        sampled
                    }
                };
                let idx = xs.len() as i32;
                xs.push(x as u16);
                ys.push(y as u16);
                rgb.push(c[0] as u8);
                rgb.push(c[1] as u8);
                rgb.push(c[2] as u8);
                phase.push(ihash(x as i32, y as i32, 2 + layer as i32 * 7));
                alpha.push(lerp(ALPHA_WARM, ALPHA_HOT, k) * fade);
                if layer == 0 && hw > 0.28 && lum(c[0], c[1], c[2]) > 150.0 {
                    hot.push(idx);
                }
            }
        }
    }

    let n = xs.len();
    if hot.is_empty() {
        for i in (0..n).step_by(7) {
            hot.push(i as i32);
        }
    }

    let gcell = ((FLOW_CELL * s).round() as u32).max(8);
    let gw = (w + gcell - 1) / gcell + 2;
    let gh = (h + gcell - 1) / gcell + 2;
    let px = (w * h * 4) as usize;

    ForgeCache {
        w,
        h,
        s,
        n,
        xs,
        ys,
        rgb,
        phase,
        alpha,
        hot,
        gcell,
        gw,
        gh,
        flow: vec![0.0; (gw * gh * 3) as usize],
        haze: vec![0; px],
        embers: vec![0; px],
        mask: ember_mask,
    }
}

fn update_flow(c: &mut ForgeCache, t: f32) {
    let mut k = 0usize;
    for gy in 0..c.gh {
        for gx in 0..c.gw {
            let gx = gx as f32;
            let gy = gy as f32;
            c.flow[k] = (f32::sin(t * 0.55 + gx * 0.33 + gy * 0.19) * 1.5
                + f32::sin(t * 0.31 - gy * 0.27 + 1.7))
                * c.s;
            c.flow[k + 1] = f32::cos(t * 0.47 + gx * 0.23 - gy * 0.37) * 0.8 * c.s;
            c.flow[k + 2] = 0.92
                + 0.08 * f32::sin(t * 0.6 + gx * 0.29 + gy * 0.43)
                + 0.04 * f32::sin(t * 0.23 - gx * 0.17);
            k += 3;
        }
    }
}

fn render_haze(c: &mut ForgeCache, t: f32) {
    let w = c.w;
    let h = c.h;
    c.haze.fill(0);
    let inv = 1.0 / CYCLE;
    let rise = RISE * c.s;
    let inv_cell = 1.0 / c.gcell as f32;
    let stride = (c.gw * 3) as usize;

    for i in 0..c.n {
        let mut u = t * inv + c.phase[i];
        u -= u.floor();
        let e = 1.0 - (2.0 * u - 1.0).abs();
        let env = e * e * (3.0 - 2.0 * e);
        if env < 0.02 {
            continue;
        }

        let x = c.xs[i] as f32;
        let y = c.ys[i] as f32;
        // Hotter source pixels climb faster; cooler/smoky haze drifts slower.
        let heat_boost = lerp(0.55, 1.35, (c.alpha[i] / ALPHA_HOT).clamp(0.0, 1.0));
        let fx = x * inv_cell;
        let fy = y * inv_cell;
        let gx = fx as usize;
        let gy = fy as usize;
        let tx = fx - gx as f32;
        let ty = fy - gy as f32;
        let k00 = gy * stride + gx * 3;
        let k10 = k00 + 3;
        let k01 = k00 + stride;
        let k11 = k01 + 3;
        if k11 + 2 >= c.flow.len() {
            continue;
        }
        let w00 = (1.0 - tx) * (1.0 - ty);
        let w10 = tx * (1.0 - ty);
        let w01 = (1.0 - tx) * ty;
        let w11 = tx * ty;
        let dx = c.flow[k00] * w00 + c.flow[k10] * w10 + c.flow[k01] * w01 + c.flow[k11] * w11;
        let dy =
            c.flow[k00 + 1] * w00 + c.flow[k10 + 1] * w10 + c.flow[k01 + 1] * w01 + c.flow[k11 + 1] * w11;
        let br =
            c.flow[k00 + 2] * w00 + c.flow[k10 + 2] * w10 + c.flow[k01 + 2] * w01 + c.flow[k11 + 2] * w11;

        let px = (x + dx * (0.35 + u)).round() as i32;
        let py = (y - u * rise * heat_boost + dy * 0.6).round() as i32;
        if px < 0 || py < 0 || px >= w as i32 || py >= h as i32 {
            continue;
        }

        let a = (env * c.alpha[i] * br * 255.0) as u8;
        if a < 4 {
            continue;
        }
        let o = ((py as u32 * w + px as u32) * 4) as usize;
        let j = i * 3;
        let cr = (c.rgb[j] as f32 * (1.1 - 0.16 * u)) as u8;
        let cg = (c.rgb[j + 1] as f32 * (1.06 - 0.3 * u)) as u8;
        let cb = (c.rgb[j + 2] as f32 * (1.0 - 0.5 * u)) as u8;
        let a0 = c.haze[o + 3];
        if a0 == 0 {
            c.haze[o] = cr;
            c.haze[o + 1] = cg;
            c.haze[o + 2] = cb;
            c.haze[o + 3] = a;
        } else {
            let na = (a0 as f32 + a as f32 * 0.75).min(255.0) as u8;
            c.haze[o + 3] = na;
            if cr > c.haze[o] {
                c.haze[o] = cr;
            }
            if cg > c.haze[o + 1] {
                c.haze[o + 1] = cg;
            }
            if cb > c.haze[o + 2] {
                c.haze[o + 2] = cb;
            }
        }
    }
}

fn plot(buf: &mut [u8], w: u32, h: u32, x: i32, y: i32, r: u8, g: u8, b: u8, a: u8) {
    if x < 0 || y < 0 || x >= w as i32 || y >= h as i32 {
        return;
    }
    let o = ((y as u32 * w + x as u32) * 4) as usize;
    let a0 = buf[o + 3];
    let na = (a0 as u16 + a as u16).min(255) as u8;
    buf[o + 3] = na;
    if r > buf[o] {
        buf[o] = r;
    }
    if g > buf[o + 1] {
        buf[o + 1] = g;
    }
    if b > buf[o + 2] {
        buf[o + 2] = b;
    }
}

fn spawn_ember(c: &ForgeCache, ep: &mut EmberPool, rng: &mut SmallRng) {
    if ep.n >= EMBER_MAX || c.n == 0 {
        return;
    }
    let s = c.s;
    let mut i = rng.random_range(0..c.n);
    for _ in 0..6 {
        let mi = (c.ys[i] as u32 * c.w + c.xs[i] as u32) as usize;
        if c.mask[mi] >= 180 {
            break;
        }
        i = rng.random_range(0..c.n);
    }
    let x = c.xs[i] as f32 + rng.random_range(-2.0..2.0);
    let y = c.ys[i] as f32;
    let r = c.rgb[i * 3];
    let g = c.rgb[i * 3 + 1];
    let b = c.rgb[i * 3 + 2];
    // Hotter mask → faster rise; cooler/smoky spawn points drift slower.
    let mi = (c.ys[i] as u32 * c.w + c.xs[i] as u32) as usize;
    let heat = (c.mask[mi] as f32 / 255.0).clamp(0.0, 1.0);
    let rise_mul = lerp(0.55, 1.25, heat);

    let k = ep.n;
    ep.n += 1;
    ep.x[k] = x;
    ep.y[k] = y;
    ep.vx[k] = rng.random_range(-3.0..3.0) * s;
    ep.vy[k] = -rng.random_range(EMBER_RISE.0..EMBER_RISE.1) * s * rise_mul;
    ep.age[k] = 0.0;
    ep.life[k] = rng.random_range(EMBER_LIFE.0..EMBER_LIFE.1);
    ep.phase[k] = rng.random_range(0.0..6.283);
    ep.size[k] = if rng.random::<f32>() < BIG_CHANCE { 2 } else { 1 };
    ep.r[k] = lerp(r as f32, 255.0, 0.55) as u8;
    ep.g[k] = lerp(g as f32, 236.0, 0.55) as u8;
    ep.b[k] = lerp(b as f32, 176.0, 0.55) as u8;
}

fn step_embers(c: &mut ForgeCache, ep: &mut EmberPool, gusts: &mut Vec<Gust>, rng: &mut SmallRng, clock: f32, dt: f32, spawn_acc: &mut f32, next_gust: &mut f32) {
    let w = c.w;
    let h = c.h;
    let s = c.s;
    let area_scale = ((w * h) as f32 / (1920.0 * 940.0)).clamp(0.5, 2.5);
    *spawn_acc += dt * EMBER_RATE * area_scale;
    while *spawn_acc >= 1.0 {
        *spawn_acc -= 1.0;
        spawn_ember(c, ep, rng);
    }

    let wind = BREEZE * s * (1.0 + f32::sin(clock * 0.23) * 0.4 + f32::sin(clock * 0.071 + 1.3) * 0.25);
    let steer = (dt * 1.2).min(1.0);
    let drag = 1.0 - 0.12 * dt;

    if clock >= *next_gust {
        *next_gust = clock + rng.random_range(GUST_EVERY.0..GUST_EVERY.1);
        if gusts.len() < GUST_MAX {
            gusts.push(Gust {
                x: rng.random_range(0.08..0.92) * w as f32,
                y: rng.random_range(0.25..0.95) * h as f32,
                age: 0.0,
                dur: rng.random_range(GUST_DURATION.0..GUST_DURATION.1),
                r: rng.random_range(GUST_RADIUS.0..GUST_RADIUS.1) * w as f32,
                spin: if rng.random::<f32>() < 0.5 { -1.0 } else { 1.0 },
                swirl: rng.random_range(GUST_SWIRL.0..GUST_SWIRL.1) * s,
                push: rng.random_range(GUST_PUSH.0..GUST_PUSH.1) * s,
            });
        }
    }
    gusts.retain_mut(|g| {
        g.age += dt;
        if g.age >= g.dur {
            return false;
        }
        g.x += (wind + g.push * 0.3) * dt;
        true
    });

    let steer_gust = (dt * 2.2).min(1.0);
    let min_rise = 7.0 * s;
    c.embers.fill(0);

    let mut i = 0usize;
    while i < ep.n {
        let age = ep.age[i] + dt;
        let p = age / ep.life[i];
        if p >= 1.0 {
            ep.remove(i);
            continue;
        }
        ep.age[i] = age;

        let phase = ep.phase[i];
        let mut vx = ep.vx[i];
        let mut vy_base = ep.vy[i] * drag;
        if vy_base > -min_rise {
            vy_base = -min_rise;
        }
        let mut swy = 0.0;
        let ex = ep.x[i];
        let ey = ep.y[i];
        for g in gusts.iter() {
            let dx = ex - g.x;
            let dy = ey - g.y;
            let inv = 1.0 / g.r;
            let fall = (-(dx * dx + dy * dy) * inv * inv).exp();
            if fall < 0.02 {
                continue;
            }
            let wgt = fall * f32::sin(std::f32::consts::PI * g.age / g.dur);
            let tx = -dy * inv * g.spin * g.swirl + g.push;
            let ty = dx * inv * g.spin * g.swirl;
            vx += (tx - vx) * steer_gust * wgt;
            swy += ty * wgt * 0.6;
        }
        let mut vy = vy_base + swy;
        let vy_cap = vy_base * 0.3;
        if vy > vy_cap {
            vy = vy_cap;
        }
        let local = wind + f32::sin(ex * 0.006 + clock * 0.35) * 3.0 * s;
        vx += (local + f32::sin(age * 1.7 + phase) * 5.0 * s - vx) * steer;
        ep.vx[i] = vx;
        ep.vy[i] = vy_base;
        let x = ep.x[i] + vx * dt;
        let y = ep.y[i] + vy * dt;
        if y < -4.0 || x < -4.0 || x > w as f32 + 4.0 || y > h as f32 + 4.0 {
            ep.remove(i);
            continue;
        }
        ep.x[i] = x;
        ep.y[i] = y;

        let (r, g, b) = if p < 0.35 {
            let k = p / 0.35;
            (
                lerp(ep.r[i] as f32, 255.0, k) as u8,
                lerp(ep.g[i] as f32, 150.0, k) as u8,
                lerp(ep.b[i] as f32, 45.0, k) as u8,
            )
        } else {
            let k = (p - 0.35) / 0.65;
            (
                lerp(255.0, 150.0, k) as u8,
                lerp(150.0, 38.0, k) as u8,
                lerp(45.0, 18.0, k) as u8,
            )
        };

        let fade_in = if age < 0.35 { age / 0.35 } else { 1.0 };
        let fo = if p > 0.55 { 1.0 - (p - 0.55) / 0.45 } else { 1.0 };
        let a = 0.9 * fade_in * fo * fo * (0.88 + 0.12 * f32::sin(age * 7.0 + phase));
        if a < 0.02 {
            i += 1;
            continue;
        }

        let px = x.round() as i32;
        let py = y.round() as i32;
        let mx = px.clamp(0, w as i32 - 1) as u32;
        let my = py.clamp(0, h as i32 - 1) as u32;
        let m = c.mask[(my * w + mx) as usize];
        let gate = (m as f32 - 110.0) / 60.0;
        if gate <= 0.0 {
            i += 1;
            continue;
        }
        let a_byte = (a * 255.0 * gate.clamp(0.0, 1.0)) as u8;
        plot(
            &mut c.embers,
            w,
            h,
            (x - vx * 0.05).round() as i32,
            (y - vy * 0.05).round() as i32,
            r,
            g,
            b,
            (a_byte as f32 * 0.35) as u8,
        );
        plot(&mut c.embers, w, h, px, py, r, g, b, a_byte);
        if ep.size[i] > 1 {
            plot(&mut c.embers, w, h, px + 1, py, r, g, b, a_byte);
            plot(&mut c.embers, w, h, px, py + 1, r, g, b, a_byte);
            plot(&mut c.embers, w, h, px + 1, py + 1, r, g, b, a_byte);
            let gl = (a_byte as f32 * 0.14) as u8;
            plot(&mut c.embers, w, h, px - 1, py, r, g, b, gl);
            plot(&mut c.embers, w, h, px + 2, py, r, g, b, gl);
            plot(&mut c.embers, w, h, px, py - 1, r, g, b, gl);
            plot(&mut c.embers, w, h, px, py + 2, r, g, b, gl);
        }
        i += 1;
    }
}

fn composite(out: &mut [u8], haze: &[u8], embers: &[u8]) {
    out.copy_from_slice(haze);
    for i in (0..out.len()).step_by(4) {
        let ea = embers[i + 3];
        if ea == 0 {
            continue;
        }
        // Additive-ish lighter composite.
        let na = (out[i + 3] as u16 + ea as u16).min(255) as u8;
        out[i + 3] = na;
        out[i] = out[i].max(embers[i]);
        out[i + 1] = out[i + 1].max(embers[i + 1]);
        out[i + 2] = out[i + 2].max(embers[i + 2]);
    }
}

impl ForgeVfx {
    pub fn create(images: &mut Assets<Image>, assets: &AssetServer) -> Self {
        let image = images.add(make_blank(400, 223));
        Self {
            image,
            backdrop: assets.load(BACKDROP_PATH),
            primed: false,
            clock: 0.0,
            haze_acc: 0.0,
            haze_ready: false,
            spawn_acc: 0.0,
            next_gust: 1.5,
            cache: None,
            ep: EmberPool::new(EMBER_MAX),
            gusts: Vec::new(),
            rng: SmallRng::seed_from_u64(0xf0_f0_46_e7),
        }
    }

    fn ensure_primed(&mut self, images: &mut Assets<Image>) -> bool {
        if self.primed {
            return true;
        }
        let (w, h, pixels) = {
            let Some(src) = images.get(&self.backdrop) else {
                return false;
            };
            let Some(data) = src.data.as_ref() else {
                return false;
            };
            let w = src.width();
            let h = src.height();
            if w == 0 || h == 0 || data.len() < (w * h * 4) as usize {
                return false;
            }
            (w, h, data.to_vec())
        };
        // Rebuild overlay at native photo size.
        if let Some(mut out) = images.get_mut(&self.image) {
            *out = make_blank(w, h);
        }
        self.cache = Some(build_cache(&pixels, w, h));
        self.ep.n = 0;
        self.spawn_acc = 0.0;
        self.haze_ready = false;
        self.primed = true;
        true
    }

    pub fn tick(&mut self, images: &mut Assets<Image>, dt: f32) {
        if !self.ensure_primed(images) {
            return;
        }
        let dt = dt.min(0.05);
        self.clock += dt;
        self.haze_acc += dt;

        let Some(cache) = self.cache.as_mut() else {
            return;
        };

        if !self.haze_ready || self.haze_acc >= 1.0 / HAZE_HZ {
            self.haze_acc = 0.0;
            self.haze_ready = true;
            update_flow(cache, self.clock);
            render_haze(cache, self.clock);
        }
        step_embers(
            cache,
            &mut self.ep,
            &mut self.gusts,
            &mut self.rng,
            self.clock,
            dt,
            &mut self.spawn_acc,
            &mut self.next_gust,
        );

        let Some(mut image) = images.get_mut(&self.image) else {
            return;
        };
        let Some(out) = image.data.as_mut() else {
            return;
        };
        if out.len() != cache.haze.len() {
            return;
        }
        composite(out, &cache.haze, &cache.embers);
    }
}

pub fn setup_forge_vfx(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    assets: Res<AssetServer>,
) {
    let vfx = ForgeVfx::create(&mut images, &assets);
    commands.insert_resource(vfx);
}

pub fn tick_forge_vfx(
    flow: Res<GameFlow>,
    mut vfx: ResMut<ForgeVfx>,
    mut images: ResMut<Assets<Image>>,
    time: Res<Time>,
) {
    if !flow.in_menu() {
        return;
    }
    vfx.tick(&mut images, time.delta_secs());
}
