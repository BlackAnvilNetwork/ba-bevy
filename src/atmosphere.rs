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
pub const MIX_CAP: f32 = 0.72;

/// Stick `PALE_HOLLOW_LOOK`.
pub const HAZE_BASE: f32 = 0.012;
#[allow(dead_code)]
pub const VALLEY_BETA: f32 = 0.018;
#[allow(dead_code)]
pub const SCALE_H: f32 = 10.0;
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
}

impl Default for StickAtmosphere {
    fn default() -> Self {
        Self {
            day_phase: OPENING_DAY_PHASE,
            atmosphere_amount: ATMO_DEFAULT,
            // Clear weather (stick `SkyWeather::Clear`)
            weather_cover: 0.22,
            weather_dark: 0.0,
            weather_haze: 0.0,
            weather_veil: 0.0,
            time: 0.0,
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
pub fn haze_density(atmo: &StickAtmosphere) -> f32 {
    HAZE_BASE * atmosphere_scale(atmo.atmosphere_amount)
}

/// Visibility metres for Bevy atmospheric fog (≈ stick far disc).
pub fn fog_visibility(atmo: &StickAtmosphere) -> f32 {
    far_m(DRAW_DEFAULT) * (0.85 + 0.3 * (1.0 - atmo.atmosphere_amount))
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

pub fn tick_atmosphere(time: Res<Time>, mut atmo: ResMut<StickAtmosphere>) {
    atmo.time = time.elapsed_secs();
}

pub fn camera_bundle(atmo: &StickAtmosphere) -> impl Bundle {
    let day = daylight(atmo.day_phase);
    let dusk = hex_linear(HAZE);
    let night = hex_linear(0x14242c);
    let haze = Color::linear_rgba(
        dusk.red + (night.red - dusk.red) * (1.0 - day),
        dusk.green + (night.green - dusk.green) * (1.0 - day),
        dusk.blue + (night.blue - dusk.blue) * (1.0 - day),
        1.0,
    );
    let horizon = hex_rgb(if day < 0.15 { 0x243044 } else { HORIZON });
    let sun = sun_dir(atmo.day_phase);
    let warm = Color::srgb(1.0, 0.55, 0.28);

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
        DistanceFog {
            // Stick MIX_CAP: haze never fully replaces the pixel.
            color: haze.with_alpha(MIX_CAP),
            directional_light_color: warm.with_alpha(0.55),
            directional_light_exponent: 8.0,
            // Aerial reach from stick far-plane + atmosphere slider.
            falloff: FogFalloff::from_visibility_colors(
                fog_visibility(atmo) / (1.0 + haze_density(atmo) * 40.0),
                haze,
                horizon,
            ),
        },
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

/// Cool overhead fill so the ground keeps color after the sun is down.
pub fn spawn_moon_fill(commands: &mut Commands) {
    let dir = Vec3::new(-0.2, 0.9, 0.35).normalize();
    commands.spawn((
        Name::new("MoonFill"),
        DirectionalLight {
            color: Color::linear_rgb(0.42, 0.62, 0.52),
            illuminance: 420.0,
            shadow_maps_enabled: false,
            ..default()
        },
        Transform::from_rotation(Quat::from_rotation_arc(Vec3::NEG_Z, -dir)),
    ));
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
