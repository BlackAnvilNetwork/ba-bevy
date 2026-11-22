//! Visual definitions for item rendering.
//!
//! Each item points to a visual key; multiple items can share visuals with
//! different tints. The visual library is loaded separately by the client.

use serde::{Deserialize, Serialize};
use smol_str::SmolStr;

/// Key into the visual library. Multiple items can share one visual.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct VisualKey(pub SmolStr);

impl VisualKey {
    pub fn new(key: impl Into<SmolStr>) -> Self {
        Self(key.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for VisualKey {
    fn from(s: &str) -> Self {
        Self(SmolStr::new(s))
    }
}

impl std::fmt::Display for VisualKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Which axis the model's long direction aligns with (before correction).
/// Used to normalize all weapons to a common grip orientation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum GripAxis {
    #[default]
    NegZ,
    PosZ,
    PosY,
    NegY,
    PosX,
    NegX,
}

impl GripAxis {
    /// Returns the rotation (in degrees) around X, Y, Z to align this axis to +Y.
    /// This is a simplified version; actual implementation may need quaternions.
    pub fn correction_euler_deg(self) -> (f32, f32, f32) {
        match self {
            Self::NegZ => (90.0, 0.0, 0.0),
            Self::PosZ => (-90.0, 0.0, 0.0),
            Self::PosY => (0.0, 0.0, 0.0),
            Self::NegY => (180.0, 0.0, 0.0),
            Self::PosX => (0.0, 0.0, -90.0),
            Self::NegX => (0.0, 0.0, 90.0),
        }
    }
}

/// Transform applied to a model when held in hand.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct GripTransform {
    /// Offset from the grip bone in metres.
    pub offset: (f32, f32, f32),
    /// Which axis the weapon's long axis is along in the source model.
    pub axis: GripAxis,
    /// Additional rotation around the weapon's long axis (degrees).
    pub roll_deg: f32,
}

impl Default for GripTransform {
    fn default() -> Self {
        Self {
            offset: (0.0, 0.0, 0.0),
            axis: GripAxis::NegZ,
            roll_deg: 0.0,
        }
    }
}

/// Material tier for tinting shared visuals.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MaterialTier {
    #[default]
    Bronze,
    Iron,
    Steel,
    Mythril,
    Darksteel,
    Orichalcum,
    Palace,
}

impl MaterialTier {
    pub fn label(self) -> &'static str {
        match self {
            Self::Bronze => "Bronze",
            Self::Iron => "Iron",
            Self::Steel => "Steel",
            Self::Mythril => "Mythril",
            Self::Darksteel => "Darksteel",
            Self::Orichalcum => "Orichalcum",
            Self::Palace => "Palace",
        }
    }
}

/// Stow position when weapon is not drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum StowSocket {
    /// Worn on back (great weapons, polearms).
    #[default]
    Back,
    /// Worn on left hip (1H swords).
    HipLeft,
    /// Worn on right hip (daggers, tools).
    HipRight,
    /// Quiver position (bows when stowed).
    QuiverSlot,
}

/// Full visual definition for an item, loaded by the client.
/// This is not stored in the catalog but in a separate visual library.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VisualDef {
    /// Path to the GLB model (e.g. "darkmod/weapons/shortsword.glb#Scene0").
    pub model: String,

    /// Uniform scale to apply. Default 0.0254 converts Doom3 units to metres.
    #[serde(default = "default_scale")]
    pub scale: f32,

    /// Grip transform for holding in hand.
    #[serde(default)]
    pub grip: GripTransform,

    /// Material names to hide (collision/shadow geometry).
    #[serde(default)]
    pub hide_materials: Vec<String>,

    /// Optional low-poly model for distant rendering.
    #[serde(default)]
    pub lod: Option<String>,

    /// Where to place when stowed (not drawn).
    #[serde(default)]
    pub stow_socket: StowSocket,

    /// Transform for stowed position.
    #[serde(default)]
    pub stow_transform: Option<GripTransform>,

    /// Variant visuals keyed by name (sheathed, unstrung, broken).
    #[serde(default)]
    pub variants: std::collections::BTreeMap<String, String>,

    /// Material tier tint to apply (if sharing base visual).
    #[serde(default)]
    pub tint: Option<MaterialTier>,
}

fn default_scale() -> f32 {
    0.0254
}

impl Default for VisualDef {
    fn default() -> Self {
        Self {
            model: String::new(),
            scale: default_scale(),
            grip: GripTransform::default(),
            hide_materials: Vec::new(),
            lod: None,
            stow_socket: StowSocket::default(),
            stow_transform: None,
            variants: std::collections::BTreeMap::new(),
            tint: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visual_key_from_str() {
        let key = VisualKey::from("shortsword");
        assert_eq!(key.as_str(), "shortsword");
    }

    #[test]
    fn grip_axis_corrections() {
        let (x, y, z) = GripAxis::PosY.correction_euler_deg();
        assert!((x.abs() + y.abs() + z.abs()) < 0.001);

        let (x, _, _) = GripAxis::NegZ.correction_euler_deg();
        assert!((x - 90.0).abs() < 0.001);
    }

    #[test]
    fn visual_def_defaults() {
        let vis = VisualDef::default();
        assert!((vis.scale - 0.0254).abs() < 0.0001);
        assert_eq!(vis.stow_socket, StowSocket::Back);
    }

    #[test]
    fn ron_round_trip() {
        let vis = VisualDef {
            model: "darkmod/weapons/shortsword.glb".into(),
            scale: 0.0254,
            grip: GripTransform {
                offset: (0.0, -0.02, 0.0),
                axis: GripAxis::NegZ,
                roll_deg: 0.0,
            },
            hide_materials: vec!["collision".into(), "shadow".into()],
            lod: None,
            stow_socket: StowSocket::HipLeft,
            stow_transform: None,
            variants: std::collections::BTreeMap::new(),
            tint: Some(MaterialTier::Iron),
        };

        let ron_str = ron::ser::to_string_pretty(&vis, ron::ser::PrettyConfig::default()).unwrap();
        let parsed: VisualDef = ron::from_str(&ron_str).unwrap();
        assert_eq!(vis.model, parsed.model);
        assert_eq!(vis.tint, parsed.tint);
    }
}
