//! Pure timing functions for loot beam animations.
//!
//! All functions take `(grammar, elapsed_secs)` and return deterministic values.
//! This allows scrubbing in debug tools and unit testing without ECS.

use super::grammar::LootGrammar;
use bevy::prelude::*;

/// Timeline constants from the spec (§III).
pub mod timing {
    /// Beam starts appearing at T+0.65s.
    pub const BEAM_START: f32 = 0.65;
    /// Beam eruption ends at T+0.75s.
    pub const BEAM_ERUPTION_END: f32 = 0.75;
    /// Beam settle ends at T+0.85s.
    pub const BEAM_SETTLE_END: f32 = 0.85;
    /// Full settle state at T+1.0s.
    pub const SETTLE_STATE: f32 = 1.0;

    /// Eruption duration (oversaturated white shoot-up).
    pub const ERUPTION_DURATION: f32 = BEAM_ERUPTION_END - BEAM_START;
    /// Settle duration (true color with pulse).
    pub const SETTLE_DURATION: f32 = BEAM_SETTLE_END - BEAM_ERUPTION_END;

    /// Shimmer cycle period for idle animation.
    pub const SHIMMER_PERIOD: f32 = 2.5;
    /// Width oscillation amplitude.
    pub const WIDTH_OSCILLATION: f32 = 0.08;

    /// Spark rise time from base to top.
    pub const SPARK_RISE_TIME: f32 = 1.2;

    /// Mythic rotation speed (radians per second).
    pub const MYTHIC_ROTATION_SPEED: f32 = 0.15;
}

/// The current phase of the beam animation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BeamPhase {
    /// Before T+0.65s: beam not visible yet.
    Hidden,
    /// T+0.65s to T+0.75s: oversaturated white shoot-up.
    Erupting { progress: f32 },
    /// T+0.75s to T+0.85s: settling to true color with upward pulse.
    Settling { progress: f32 },
    /// T+0.85s+: idle shimmer/oscillation.
    Idle { time_in_idle: f32 },
}

/// Compute the current beam phase from elapsed time.
pub fn beam_phase(elapsed: f32) -> BeamPhase {
    if elapsed < timing::BEAM_START {
        BeamPhase::Hidden
    } else if elapsed < timing::BEAM_ERUPTION_END {
        let progress = (elapsed - timing::BEAM_START) / timing::ERUPTION_DURATION;
        BeamPhase::Erupting { progress }
    } else if elapsed < timing::BEAM_SETTLE_END {
        let progress = (elapsed - timing::BEAM_ERUPTION_END) / timing::SETTLE_DURATION;
        BeamPhase::Settling { progress }
    } else {
        let time_in_idle = elapsed - timing::BEAM_SETTLE_END;
        BeamPhase::Idle { time_in_idle }
    }
}

/// Beam state computed from grammar and elapsed time.
#[derive(Debug, Clone, Copy)]
pub struct BeamState {
    /// Visible at all.
    pub visible: bool,
    /// Current height (0 to full).
    pub height: f32,
    /// Current width multiplier (1.0 = base width).
    pub width_mult: f32,
    /// Current color (may be white during eruption).
    pub color: LinearRgba,
    /// Current opacity (0 to 1).
    pub opacity: f32,
    /// Current HDR multiplier.
    pub hdr_mult: f32,
    /// Pulse position (0 to 1, traveling up the beam).
    pub pulse_pos: Option<f32>,
    /// Rotation angle for Mythic (radians).
    pub rotation: f32,
}

/// Compute beam state from grammar and elapsed time.
pub fn beam_state(grammar: LootGrammar, elapsed: f32) -> BeamState {
    if !grammar.has_beam() {
        return BeamState {
            visible: false,
            height: 0.0,
            width_mult: 1.0,
            color: LinearRgba::WHITE,
            opacity: 0.0,
            hdr_mult: 0.0,
            pulse_pos: None,
            rotation: 0.0,
        };
    }

    let phase = beam_phase(elapsed);
    let base_color = grammar.linear_color();
    let base_height = grammar.beam_height();
    let base_hdr = grammar.hdr_multiplier();

    match phase {
        BeamPhase::Hidden => BeamState {
            visible: false,
            height: 0.0,
            width_mult: 1.0,
            color: base_color,
            opacity: 0.0,
            hdr_mult: 0.0,
            pulse_pos: None,
            rotation: 0.0,
        },
        BeamPhase::Erupting { progress } => {
            // Shoot up with eased curve
            let eased = ease_out_cubic(progress);
            // Oversaturated white during eruption
            let white_mix = 1.0 - progress * progress;
            let color = lerp_color(base_color, LinearRgba::WHITE, white_mix);
            // Extra HDR during eruption (up to 2x)
            let eruption_hdr = base_hdr * (1.0 + white_mix);

            BeamState {
                visible: true,
                height: base_height * eased,
                width_mult: 1.0 + (1.0 - progress) * 0.3, // Wider during eruption
                color,
                opacity: 1.0,
                hdr_mult: eruption_hdr,
                pulse_pos: None,
                rotation: 0.0,
            }
        }
        BeamPhase::Settling { progress } => {
            // Pulse travels up the beam
            let pulse_pos = ease_out_quad(progress);

            BeamState {
                visible: true,
                height: base_height,
                width_mult: 1.0 + (1.0 - progress) * 0.1,
                color: base_color,
                opacity: 1.0,
                hdr_mult: base_hdr,
                pulse_pos: Some(pulse_pos),
                rotation: 0.0,
            }
        }
        BeamPhase::Idle { time_in_idle } => {
            // Shimmer and width oscillation (Quest doesn't shimmer)
            let (shimmer, width_osc) = if grammar.shimmers() {
                let shimmer = 0.5 + 0.5 * (time_in_idle * std::f32::consts::TAU / timing::SHIMMER_PERIOD).sin();
                let width_osc = 1.0 + timing::WIDTH_OSCILLATION * (time_in_idle * 1.7).sin();
                (shimmer, width_osc)
            } else {
                (1.0, 1.0)
            };

            // Mythic rotation
            let rotation = if grammar.has_rotation() {
                time_in_idle * timing::MYTHIC_ROTATION_SPEED
            } else {
                0.0
            };

            // Slight HDR pulse for shimmer
            let hdr_pulse = base_hdr * (0.9 + 0.1 * shimmer);

            BeamState {
                visible: true,
                height: base_height,
                width_mult: width_osc,
                color: base_color,
                opacity: 0.9 + 0.1 * shimmer,
                hdr_mult: hdr_pulse,
                pulse_pos: None,
                rotation,
            }
        }
    }
}

/// Compute spark positions for a beam (Legendary+ only).
/// Returns Vec of (height_fraction, angle_offset) for each spark.
pub fn spark_positions(grammar: LootGrammar, elapsed: f32, count: usize) -> Vec<(f32, f32)> {
    if !grammar.has_sparks() || elapsed < timing::BEAM_SETTLE_END {
        return Vec::new();
    }

    let time_in_idle = elapsed - timing::BEAM_SETTLE_END;
    let mut positions = Vec::with_capacity(count);

    for i in 0..count {
        // Stagger spawn times
        let offset = i as f32 * (timing::SPARK_RISE_TIME / count as f32);
        let t = (time_in_idle + offset) % timing::SPARK_RISE_TIME;
        let height_frac = t / timing::SPARK_RISE_TIME;

        // Spiral angle with some randomness from index
        let base_angle = time_in_idle * 0.5 + i as f32 * 0.618 * std::f32::consts::TAU;
        let lateral_offset = (i as f32 * 1.618).sin() * 0.15;

        positions.push((height_frac, base_angle + lateral_offset));
    }

    positions
}

/// Ground glow state.
#[derive(Debug, Clone, Copy)]
pub struct GroundGlowState {
    pub visible: bool,
    pub radius: f32,
    pub intensity: f32,
    pub color: LinearRgba,
}

/// Compute ground glow state from grammar and elapsed time.
pub fn ground_glow_state(grammar: LootGrammar, elapsed: f32) -> GroundGlowState {
    if !grammar.has_beam() {
        return GroundGlowState {
            visible: false,
            radius: 0.0,
            intensity: 0.0,
            color: LinearRgba::WHITE,
        };
    }

    let phase = beam_phase(elapsed);
    let base_radius = grammar.ground_glow_radius();
    let base_color = grammar.linear_color();

    match phase {
        BeamPhase::Hidden => GroundGlowState {
            visible: false,
            radius: 0.0,
            intensity: 0.0,
            color: base_color,
        },
        BeamPhase::Erupting { progress } => {
            // Expand quickly during eruption
            let eased = ease_out_cubic(progress);
            GroundGlowState {
                visible: true,
                radius: base_radius * eased,
                intensity: 1.0 + (1.0 - progress) * 0.5, // Brighter during eruption
                color: lerp_color(base_color, LinearRgba::WHITE, 0.3 * (1.0 - progress)),
            }
        }
        BeamPhase::Settling { progress } => GroundGlowState {
            visible: true,
            radius: base_radius,
            intensity: 1.0 + (1.0 - progress) * 0.2,
            color: base_color,
        },
        BeamPhase::Idle { time_in_idle } => {
            // Subtle pulse for Legendary+
            let pulse = if grammar.has_sparks() {
                0.9 + 0.1 * (time_in_idle * 0.8).sin()
            } else {
                1.0
            };

            GroundGlowState {
                visible: true,
                radius: base_radius,
                intensity: pulse,
                color: base_color,
            }
        }
    }
}

// Easing functions
fn ease_out_cubic(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

fn ease_out_quad(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t) * (1.0 - t)
}

fn lerp_color(a: LinearRgba, b: LinearRgba, t: f32) -> LinearRgba {
    let t = t.clamp(0.0, 1.0);
    LinearRgba::new(
        a.red + (b.red - a.red) * t,
        a.green + (b.green - a.green) * t,
        a.blue + (b.blue - a.blue) * t,
        a.alpha + (b.alpha - a.alpha) * t,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn beam_phase_transitions() {
        assert!(matches!(beam_phase(0.0), BeamPhase::Hidden));
        assert!(matches!(beam_phase(0.64), BeamPhase::Hidden));
        assert!(matches!(beam_phase(0.65), BeamPhase::Erupting { .. }));
        assert!(matches!(beam_phase(0.70), BeamPhase::Erupting { .. }));
        assert!(matches!(beam_phase(0.75), BeamPhase::Settling { .. }));
        assert!(matches!(beam_phase(0.80), BeamPhase::Settling { .. }));
        assert!(matches!(beam_phase(0.85), BeamPhase::Idle { .. }));
        assert!(matches!(beam_phase(10.0), BeamPhase::Idle { .. }));
    }

    #[test]
    fn beam_state_respects_grammar() {
        let none_state = beam_state(LootGrammar::None, 1.0);
        assert!(!none_state.visible);

        let rare_state = beam_state(LootGrammar::Rare, 1.0);
        assert!(rare_state.visible);
        assert!((rare_state.height - 2.0).abs() < 0.01);

        let mythic_state = beam_state(LootGrammar::Mythic, 1.0);
        assert!(mythic_state.visible);
        assert!((mythic_state.height - 10.0).abs() < 0.01);
    }

    #[test]
    fn quest_does_not_shimmer() {
        let quest1 = beam_state(LootGrammar::Quest, 1.0);
        let quest2 = beam_state(LootGrammar::Quest, 2.5);
        // Quest beam should have consistent opacity (no shimmer)
        assert!((quest1.opacity - quest2.opacity).abs() < 0.01);
    }

    #[test]
    fn sparks_only_for_legendary_plus() {
        assert!(spark_positions(LootGrammar::Rare, 2.0, 10).is_empty());
        assert!(spark_positions(LootGrammar::Epic, 2.0, 10).is_empty());
        assert!(!spark_positions(LootGrammar::Legendary, 2.0, 10).is_empty());
        assert!(!spark_positions(LootGrammar::Mythic, 2.0, 10).is_empty());
    }
}
