//! Port of `ba-stick` `atmosphere.rs` + sky lighting helpers for the Bevy opening.
//! Numbers match the stick sky as closely as Bevy PBR allows.

use bevy::{
    camera::Hdr,
    core_pipeline::tonemapping::Tonemapping,
    pbr::{DistanceFog, FogFalloff},
    post_process::{bloom::Bloom, effect_stack::Vignette},
    prelude::*,
};

/// Stick `atmosphere::SUN_DIR` — unit vector *toward* the sun (NW + high). Iso contact shadows.
#[allow(dead_code)]
pub const SUN_DIR: Vec3 = Vec3::new(-0.6835, 0.7071, -0.1810);

/// Stick `atmosphere::ATMO_DEFAULT` (optical-depth slider).
pub const ATMO_DEFAULT: f32 = 0.45;

/// Stick `atmosphere::DRAW_DEFAULT`.
pub const DRAW_DEFAULT: f32 = 0.55;

/// Stick `atmosphere::MIX_CAP`.
#[allow(dead_code)]
pub const MIX_CAP: f32 = 0.72;

/// Stick `PALE_HOLLOW_LOOK`.
pub const HAZE_BASE: f32 = 0.012;
#[allow(dead_code)]
pub const VALLEY_BETA: f32 = 0.018;
#[allow(dead_code)]
pub const SCALE_H: f32 = 10.0;
#[allow(dead_code)]
pub const HORIZON: u32 = 0x918b87;

/// Stick `PALE_HOLLOW` params.
pub const AMBIENT: [f32; 3] = [0.82, 0.86, 0.98];
pub const HAZE: u32 = 0x6e6a7c;
pub const SKY_TOP: u32 = 0x1e1b2c;
pub const VIGNETTE: f32 = 0.32;

/// Stick sky palette keyframes (zenith, horizon) — mirrored in `stick_sky.wesl`.
#[allow(dead_code)]
pub const NIGHT: (u32, u32) = (0x070a1a, 0x1c2140);
#[allow(dead_code)]
pub const TWILIGHT: (u32, u32) = (0x2a2a5c, 0xe0784a);
#[allow(dead_code)]
pub const DAY: (u32, u32) = (0x3a6ec8, 0xa8c8e8);

/// Opening reads as night. 0 is midnight, 0.25 is sunrise.
pub const OPENING_DAY_PHASE: f32 = 0.14;

/// USNO new moons read from the zone server's cache. Empty uses the baked year.
#[derive(Resource, Clone, Debug, Default)]
pub struct PersistedMoon {
    pub new_moons_unix: Vec<i64>,
}

/// Stick `lighting::Kind::Fire` / well fill defaults.
pub const FIRE_RGB: [f32; 3] = [1.0, 0.6, 0.3];
pub const FIRE_RANGE_M: f32 = 9.0;

#[derive(Resource, Clone, Copy, Debug)]
pub struct StickAtmosphere {
    pub day_phase: f32,
    pub atmosphere_amount: f32,
    pub weather_cover: f32,
    pub weather_dark: f32,
    pub weather_haze: f32,
    pub weather_veil: f32,
    pub time: f32,
    /// Synodic age. 0 is new, 0.5 is full.
    pub lunar_age: f32,
    pub lunation_start: i64,
    pub lunation_end: i64,
    /// Zone welcome owns the lunation so every client shares one moon.
    pub moon_from_server: bool,
}

impl Default for StickAtmosphere {
    fn default() -> Self {
        let now = ba_protocol::moon::unix_now();
        let moon = ba_protocol::moon::lunation_at(now, &[]);
        Self {
            day_phase: OPENING_DAY_PHASE,
            atmosphere_amount: ATMO_DEFAULT,
            // Clear weather (stick `SkyWeather::Clear`)
            weather_cover: 0.22,
            weather_dark: 0.0,
            weather_haze: 0.0,
            weather_veil: 0.0,
            time: 0.0,
            lunar_age: moon.age_at(now),
            lunation_start: moon.new_moon_unix,
            lunation_end: moon.next_new_moon_unix,
            moon_from_server: false,
        }
    }
}

/// Stick `atmosphere::atmosphere_scale`.
pub fn atmosphere_scale(t: f32) -> f32 {
    0.15 + (2.2 - 0.15) * t.clamp(0.0, 1.0)
}

/// Stick `atmosphere::far_m`.
pub fn far_m(t: f32) -> f32 {
    piecewise(t, 36.0, 60.0, 90.0)
}

fn piecewise(t: f32, y0: f32, y55: f32, y1: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    if t <= 0.55 {
        y0 + (y55 - y0) * (t / 0.55)
    } else {
        y55 + (y1 - y55) * ((t - 0.55) / 0.45)
    }
}

/// Stick `sky::sun_dir` — rises east (+X), sets west.
pub fn sun_dir(phase: f32) -> Vec3 {
    let a = (phase - 0.25) * std::f32::consts::TAU;
    let (s, c) = a.sin_cos();
    Vec3::new(c, s * 0.92, s * 0.38).normalize_or_zero()
}

/// Moon on the sun's daily arc, east of the sun by the synodic age.
/// Age 0 sits on the sun (new). Age 0.5 is opposite the sun (full).
pub fn moon_dir(day_phase: f32, lunar_age: f32) -> Vec3 {
    sun_dir(day_phase - lunar_age)
}

/// Lit fraction of the disc from the game sun and the real elongation.
pub fn moon_illumination(day_phase: f32, lunar_age: f32) -> f32 {
    let sun = sun_dir(day_phase);
    let moon = moon_dir(day_phase, lunar_age);
    ((1.0 - sun.dot(moon)) * 0.5).clamp(0.0, 1.0)
}

/// Stick `sky::daylight`.
pub fn daylight(phase: f32) -> f32 {
    let e = sun_dir(phase).y;
    ((e + 0.18) / 0.36).clamp(0.0, 1.0)
}

/// Stick `sky::ambient_scale`.
pub fn ambient_scale(phase: f32) -> Vec3 {
    let d = daylight(phase);
    let s = |n: f32| n + (1.0 - n) * d;
    Vec3::new(s(0.42), s(0.46), s(0.62))
}

/// Stick `sky::unit_light` for Clear weather.
pub fn unit_light(phase: f32, cover: f32, dark: f32) -> Vec3 {
    let day = daylight(phase);
    let sun = sun_dir(phase);
    let twi = (1.0 - (sun.y / 0.28).abs()).clamp(0.0, 1.0);
    let night = Vec3::new(0.62, 0.68, 0.92);
    let mut m = night.lerp(Vec3::ONE, day);
    m = m.lerp(Vec3::new(1.06, 0.9, 0.76), twi * 0.7);
    let dim = 1.0 - 0.12 * cover * dark;
    m * dim
}

pub fn hex_rgb(c: u32) -> Color {
    Color::srgb_u8(
        ((c >> 16) & 255) as u8,
        ((c >> 8) & 255) as u8,
        (c & 255) as u8,
    )
}

#[allow(dead_code)]
pub fn hex_linear(c: u32) -> LinearRgba {
    LinearRgba::from(hex_rgb(c))
}

/// Effective aerial β (1/m) after atmosphere slider — stick haze_base * scale.
#[allow(dead_code)]
pub fn haze_density(atmo: &StickAtmosphere) -> f32 {
    HAZE_BASE * atmosphere_scale(atmo.atmosphere_amount)
}

/// Visibility metres for Bevy atmospheric fog (≈ stick far disc).
pub fn fog_visibility(atmo: &StickAtmosphere) -> f32 {
    far_m(DRAW_DEFAULT) * (0.95 + 0.25 * (1.0 - atmo.atmosphere_amount))
}

/// Air color for distance fade — dusk lavender, not grey and not night-black.
pub fn air_color(atmo: &StickAtmosphere) -> Color {
    let day = daylight(atmo.day_phase);
    let night = LinearRgba::new(0.37, 0.39, 0.56, 1.0);
    let dusk = hex_linear(HAZE);
    let noon = hex_linear(0xa8c8e8);
    let mix = |a: LinearRgba, b: LinearRgba, t: f32| {
        let t = t.clamp(0.0, 1.0);
        LinearRgba::new(
            a.red + (b.red - a.red) * t,
            a.green + (b.green - a.green) * t,
            a.blue + (b.blue - a.blue) * t,
            1.0,
        )
    };
    let air = if day < 0.28 {
        mix(night, dusk, day / 0.28)
    } else {
        mix(dusk, noon, (day - 0.28) / 0.72)
    };
    Color::from(air)
}

/// Draw-distance haze. Near ground stays clear; far meshes wash into sky air.
pub fn distance_fog(atmo: &StickAtmosphere, visibility: f32) -> DistanceFog {
    let air = air_color(atmo);
    let day = daylight(atmo.day_phase);
    let end = visibility.max(36.0);
    DistanceFog {
        color: air.with_alpha(1.0),
        directional_light_color: Color::srgb(1.0, 0.55, 0.28).with_alpha(0.22 + 0.28 * day),
        directional_light_exponent: 12.0,
        falloff: FogFalloff::Linear {
            start: (end * 0.32).max(16.0),
            end,
        },
    }
}

/// Stick `lighting::noise1` — smooth 1-D value noise.
pub fn noise1(x: f32) -> f32 {
    let i = x.floor();
    let f = x - i;
    let s = f * f * (3.0 - 2.0 * f);
    let a = hash1(i as i32);
    let b = hash1(i as i32 + 1);
    a + (b - a) * s
}

fn hash1(i: i32) -> f32 {
    let mut n = (i as u32).wrapping_mul(0x27D4_EB2D) ^ 0x1656_67B1;
    n ^= n >> 15;
    n = n.wrapping_mul(0x85EB_CA6B);
    n ^= n >> 13;
    (n >> 8) as f32 / 16_777_216.0
}

/// Fire flicker envelope (stick Fire flicker feel).
pub fn fire_flicker(time: f32, seed: f32) -> f32 {
    let n = noise1(time * 3.1 + seed) * 0.55 + noise1(time * 7.7 + seed * 1.7) * 0.45;
    0.72 + 0.28 * n
}

pub fn load_persisted_moon(mut table: ResMut<PersistedMoon>, mut atmo: ResMut<StickAtmosphere>) {
    let path = ba_protocol::moon::cache_path();
    match ba_protocol::moon::read_cache(&path) {
        Some(cached) => {
            info!(
                "Moon table {} ({} new moons)",
                path.display(),
                cached.new_moons_unix.len()
            );
            table.new_moons_unix = cached.new_moons_unix;
        }
        None => {
            let now = ba_protocol::moon::unix_now();
            if ba_protocol::moon::covered(now, &[]) {
                info!(
                    "No moon cache at {} — using the built-in USNO year",
                    path.display()
                );
            } else {
                info!(
                    "No moon cache at {} — mean month until the zone server fetches USNO",
                    path.display()
                );
            }
        }
    }
    apply_local_lunation(&table, &mut atmo);
    info!(
        "Moon {} age={:.3}",
        ba_protocol::moon::phase_name(atmo.lunar_age),
        atmo.lunar_age
    );
}

fn apply_local_lunation(table: &PersistedMoon, atmo: &mut StickAtmosphere) {
    let now = ba_protocol::moon::unix_now();
    let moon = ba_protocol::moon::lunation_at(now, &table.new_moons_unix);
    atmo.lunation_start = moon.new_moon_unix;
    atmo.lunation_end = moon.next_new_moon_unix;
    atmo.lunar_age = moon.age_at(now);
}

pub fn tick_atmosphere(
    time: Res<Time>,
    persisted: Res<PersistedMoon>,
    mut atmo: ResMut<StickAtmosphere>,
) {
    atmo.time = time.elapsed_secs();
    if !atmo.moon_from_server {
        apply_local_lunation(&persisted, &mut atmo);
    } else {
        let now = ba_protocol::moon::unix_now();
        atmo.lunar_age = ba_protocol::moon::MoonState {
            new_moon_unix: atmo.lunation_start,
            next_new_moon_unix: atmo.lunation_end,
        }
        .age_at(now);
    }
}

pub fn camera_bundle(atmo: &StickAtmosphere) -> impl Bundle {
    let sun = sun_dir(atmo.day_phase);

    (
        Camera3d::default(),
        Camera {
            clear_color: ClearColorConfig::Custom(hex_rgb(SKY_TOP)),
            ..default()
        },
        Hdr,
        Tonemapping::TonyMcMapface,
        Bloom::NATURAL,
        Vignette {
            // stick vignette 0.32 → intensity (Bevy default is 1.0 = crushed)
            intensity: VIGNETTE,
            radius: 1.05,
            smoothness: 0.55,
            roundness: 1.0,
            color: Color::BLACK,
            edge_compensation: 1.0,
            center: Vec2::new(0.5, 0.5),
        },
        distance_fog(atmo, fog_visibility(atmo)),
        Projection::from(PerspectiveProjection {
            fov: 75.0_f32.to_radians(),
            // Stick FAR ~60 m for cull; sky dome is beyond that so keep far open.
            far: 2_000.0,
            ..default()
        }),
        // Used by systems that need sun alignment; actual light entity mirrors this.
        Name::new(format!(
            "StickCam phase={:.2} sun=({:.2},{:.2},{:.2})",
            atmo.day_phase, sun.x, sun.y, sun.z
        )),
    )
}

pub fn spawn_sun(commands: &mut Commands, atmo: &StickAtmosphere) {
    let dir = sun_dir(atmo.day_phase);
    // Stick sun colour warms near horizon
    let warm = (1.0 - dir.y / 0.4).clamp(0.0, 1.0);
    let core = Vec3::new(1.0, 0.98, 0.84).lerp(Vec3::new(1.0, 0.66, 0.32), warm);
    let day = daylight(atmo.day_phase);
    // Dusk illuminance between twilight (~400 lux) and overcast day
    let lux = 350.0 + 4_500.0 * day;

    // Light shines *along* -dir (from sun toward scene)
    let rotation = Quat::from_rotation_arc(Vec3::NEG_Z, -dir);

    commands.spawn((
        Name::new("StickSun"),
        DirectionalLight {
            color: Color::linear_rgb(core.x, core.y, core.z),
            illuminance: lux,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_rotation(rotation),
    ));
}

pub fn spawn_ambient(commands: &mut Commands, atmo: &StickAtmosphere) {
    let scale = ambient_scale(atmo.day_phase);
    let unit = unit_light(atmo.day_phase, atmo.weather_cover, atmo.weather_dark);
    let rgb = Vec3::new(AMBIENT[0], AMBIENT[1], AMBIENT[2]) * scale * unit;
    // Stick ambient is a multiply (~0.4–1.0); Bevy brightness is candela/m².
    let brightness = 28.0 + 90.0 * daylight(atmo.day_phase);

    commands.insert_resource(GlobalAmbientLight {
        color: Color::linear_rgb(rgb.x.clamp(0.0, 1.5), rgb.y.clamp(0.0, 1.5), rgb.z.clamp(0.0, 1.5)),
        brightness,
        ..default()
    });
}

/// Cool fill from the real moon. A floor keeps the yard readable on a new moon.
pub fn moon_fill_lux(day_phase: f32, lunar_age: f32) -> (Vec3, f32) {
    let moon = moon_dir(day_phase, lunar_age);
    let illum = moon_illumination(day_phase, lunar_age);
    let up = moon.y.clamp(0.0, 1.0);
    let lux = 180.0 + 280.0 * illum * (0.35 + 0.65 * up);
    let dir = if moon.y > 0.05 {
        moon
    } else {
        Vec3::new(-0.2, 0.9, 0.35).normalize()
    };
    (dir, lux)
}

#[derive(Component)]
pub struct MoonFill;

/// Cool fill so the ground keeps color after the sun is down.
pub fn spawn_moon_fill(commands: &mut Commands, atmo: &StickAtmosphere) {
    let (dir, lux) = moon_fill_lux(atmo.day_phase, atmo.lunar_age);
    commands.spawn((
        MoonFill,
        Name::new("MoonFill"),
        DirectionalLight {
            color: Color::linear_rgb(0.42, 0.62, 0.52),
            illuminance: lux,
            shadow_maps_enabled: false,
            ..default()
        },
        Transform::from_rotation(Quat::from_rotation_arc(Vec3::NEG_Z, -dir)),
    ));
}

pub fn sync_moon_fill(
    atmo: Res<StickAtmosphere>,
    mut lights: Query<(&mut DirectionalLight, &mut Transform), With<MoonFill>>,
) {
    let (dir, lux) = moon_fill_lux(atmo.day_phase, atmo.lunar_age);
    for (mut light, mut tf) in &mut lights {
        light.illuminance = lux;
        tf.rotation = Quat::from_rotation_arc(Vec3::NEG_Z, -dir);
    }
}

pub fn spawn_well_fire(commands: &mut Commands) {
    commands.spawn((
        Name::new("WellFireLight"),
        PointLight {
            color: Color::linear_rgb(FIRE_RGB[0], FIRE_RGB[1], FIRE_RGB[2]),
            intensity: 95_000.0,
            range: FIRE_RANGE_M,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(-2.05, 0.55, 0.35),
        WellFireLight { seed: 17.3 },
    ));
}

#[derive(Component)]
pub struct WellFireLight {
    pub seed: f32,
}

pub fn flicker_well_fire(
    atmo: Res<StickAtmosphere>,
    mut lights: Query<(&WellFireLight, &mut PointLight)>,
) {
    for (fire, mut light) in &mut lights {
        let f = fire_flicker(atmo.time, fire.seed);
        light.intensity = 95_000.0 * f;
        light.color = Color::linear_rgb(
            FIRE_RGB[0],
            FIRE_RGB[1] * (0.92 + 0.08 * f),
            FIRE_RGB[2] * (0.85 + 0.15 * f),
        );
    }
}

#[cfg(test)]
mod moon_sky_tests {
    use super::*;

    #[test]
    fn full_moon_is_opposite_the_sun_and_new_moon_is_on_it() {
        let sun = sun_dir(OPENING_DAY_PHASE);
        assert!((moon_dir(OPENING_DAY_PHASE, 0.5) + sun).length() < 1e-4);
        assert!((moon_dir(OPENING_DAY_PHASE, 0.0) - sun).length() < 1e-4);
        assert!((moon_illumination(OPENING_DAY_PHASE, 0.5) - 1.0).abs() < 1e-4);
        assert!(moon_illumination(OPENING_DAY_PHASE, 0.0) < 1e-4);
    }

    #[test]
    fn november_2026_gibbous_sits_above_the_opening_horizon() {
        let now = ba_protocol::moon::unix_from_utc(2026, 11, 21, 21, 8);
        let moon = ba_protocol::moon::lunation_at(now, &[]);
        let age = moon.age_at(now);
        let dir = moon_dir(OPENING_DAY_PHASE, age);
        assert!(dir.y > 0.05, "altitude {} age {age}", dir.y);
        assert!(moon_illumination(OPENING_DAY_PHASE, age) > 0.85);
    }
}
