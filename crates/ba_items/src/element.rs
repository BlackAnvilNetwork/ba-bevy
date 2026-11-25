//! Elemental types for runes, glyphs, and enchantments.

use serde::{Deserialize, Serialize};

/// The eight elements used in crafting and enchanting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Element {
    Fire,
    Ice,
    Wind,
    Water,
    Earth,
    Lightning,
    Light,
    Dark,
}

impl Element {
    pub const ALL: [Element; 8] = [
        Self::Fire,
        Self::Ice,
        Self::Wind,
        Self::Water,
        Self::Earth,
        Self::Lightning,
        Self::Light,
        Self::Dark,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Fire => "Fire",
            Self::Ice => "Ice",
            Self::Wind => "Wind",
            Self::Water => "Water",
            Self::Earth => "Earth",
            Self::Lightning => "Lightning",
            Self::Light => "Light",
            Self::Dark => "Dark",
        }
    }

    pub fn rune_name(self) -> &'static str {
        match self {
            Self::Fire => "Fire Rune",
            Self::Ice => "Ice Rune",
            Self::Wind => "Wind Rune",
            Self::Water => "Water Rune",
            Self::Earth => "Earth Rune",
            Self::Lightning => "Lightning Rune",
            Self::Light => "Light Rune",
            Self::Dark => "Dark Rune",
        }
    }

    pub fn glyph_name(self) -> &'static str {
        match self {
            Self::Fire => "Fire Glyph",
            Self::Ice => "Ice Glyph",
            Self::Wind => "Wind Glyph",
            Self::Water => "Water Glyph",
            Self::Earth => "Earth Glyph",
            Self::Lightning => "Lightning Glyph",
            Self::Light => "Light Glyph",
            Self::Dark => "Dark Glyph",
        }
    }

    pub fn material_category(self) -> MaterialCategory {
        match self {
            Self::Fire => MaterialCategory::Metal,
            Self::Ice => MaterialCategory::Leather,
            Self::Wind => MaterialCategory::Wood,
            Self::Water => MaterialCategory::Cloth,
            Self::Earth => MaterialCategory::Bone,
            Self::Lightning => MaterialCategory::PreciousMetal,
            Self::Light => MaterialCategory::Stone,
            Self::Dark => MaterialCategory::Essence,
        }
    }
}

/// Material categories, each associated with one element's rune.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MaterialCategory {
    Metal,
    Wood,
    Leather,
    Cloth,
    Bone,
    PreciousMetal,
    Stone,
    Essence,
}

impl MaterialCategory {
    pub const ALL: [MaterialCategory; 8] = [
        Self::Metal,
        Self::Wood,
        Self::Leather,
        Self::Cloth,
        Self::Bone,
        Self::PreciousMetal,
        Self::Stone,
        Self::Essence,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Metal => "Metal",
            Self::Wood => "Wood",
            Self::Leather => "Leather",
            Self::Cloth => "Cloth",
            Self::Bone => "Bone",
            Self::PreciousMetal => "Precious Metal",
            Self::Stone => "Stone",
            Self::Essence => "Essence",
        }
    }

    pub fn required_rune(self) -> Element {
        match self {
            Self::Metal => Element::Fire,
            Self::Wood => Element::Wind,
            Self::Leather => Element::Ice,
            Self::Cloth => Element::Water,
            Self::Bone => Element::Earth,
            Self::PreciousMetal => Element::Lightning,
            Self::Stone => Element::Light,
            Self::Essence => Element::Dark,
        }
    }

    pub fn station_name(self) -> &'static str {
        match self {
            Self::Metal => "Forge",
            Self::Wood => "Workbench",
            Self::Leather => "Tannery",
            Self::Cloth => "Loom",
            Self::Bone => "Bone Altar",
            Self::PreciousMetal => "Jeweler",
            Self::Stone => "Stonecutter",
            Self::Essence => "Dark Altar",
        }
    }
}

/// Crafting tier (1-5). Higher tiers require rarer materials and more gold.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CraftTier(pub u8);

impl CraftTier {
    pub const T1: CraftTier = CraftTier(1);
    pub const T2: CraftTier = CraftTier(2);
    pub const T3: CraftTier = CraftTier(3);
    pub const T4: CraftTier = CraftTier(4);
    pub const T5: CraftTier = CraftTier(5);
    pub const MAX: CraftTier = CraftTier(5);

    pub fn new(tier: u8) -> Option<Self> {
        if tier >= 1 && tier <= 5 {
            Some(CraftTier(tier))
        } else {
            None
        }
    }

    pub fn refine_gold_cost(self) -> u32 {
        match self.0 {
            1 => 10,
            2 => 25,
            3 => 100,
            4 => 250,
            5 => 1000,
            _ => 10,
        }
    }

    pub fn craft_gold_cost(self) -> u32 {
        match self.0 {
            1 => 25,
            2 => 50,
            3 => 250,
            4 => 500,
            5 => 2500,
            _ => 25,
        }
    }

    pub fn enchant_gold_cost(self) -> u32 {
        500 + 100 * self.0 as u32
    }

    pub fn max_durability(self) -> u32 {
        match self.0 {
            1 => 50,
            2 => 75,
            3 => 100,
            4 => 125,
            5 => 150,
            _ => 50,
        }
    }

    pub fn base_weapon_damage(self) -> (u32, u32) {
        match self.0 {
            1 => (8, 12),
            2 => (14, 20),
            3 => (24, 32),
            4 => (38, 48),
            5 => (55, 70),
            _ => (8, 12),
        }
    }

    pub fn base_weapon_attack(self) -> i16 {
        match self.0 {
            1 => 5,
            2 => 10,
            3 => 18,
            4 => 28,
            5 => 42,
            _ => 5,
        }
    }

    pub fn base_armor_defense(self) -> i16 {
        match self.0 {
            1 => 3,
            2 => 6,
            3 => 12,
            4 => 20,
            5 => 32,
            _ => 3,
        }
    }

    pub fn base_armor_hp(self) -> i16 {
        match self.0 {
            1 => 15,
            2 => 30,
            3 => 50,
            4 => 80,
            5 => 120,
            _ => 15,
        }
    }
}

impl Default for CraftTier {
    fn default() -> Self {
        Self::T1
    }
}

impl std::fmt::Display for CraftTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "T{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn element_material_category_round_trip() {
        for elem in Element::ALL {
            let cat = elem.material_category();
            assert_eq!(cat.required_rune(), elem);
        }
    }

    #[test]
    fn craft_tier_costs() {
        assert_eq!(CraftTier::T1.refine_gold_cost(), 10);
        assert_eq!(CraftTier::T5.refine_gold_cost(), 1000);
        assert_eq!(CraftTier::T3.enchant_gold_cost(), 800);
    }

    #[test]
    fn craft_tier_new_validation() {
        assert!(CraftTier::new(0).is_none());
        assert!(CraftTier::new(1).is_some());
        assert!(CraftTier::new(5).is_some());
        assert!(CraftTier::new(6).is_none());
    }

    #[test]
    fn craft_tier_ordering() {
        assert!(CraftTier::T1 < CraftTier::T2);
        assert!(CraftTier::T4 < CraftTier::T5);
    }
}
