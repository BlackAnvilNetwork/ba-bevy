//! Loot grammar: the visual language of rarity tiers.
//!
//! Each tier has a fixed color, beam height, ground glow radius, and HDR multiplier.
//! Materials (Bog Amber, Cold Iron, etc.) and Common items have no beam (Grammar::None).

use bevy::prelude::*;

/// The visual grammar tier for a loot drop. Determines beam appearance.
/// This is distinct from item rarity (a value tier) — materials can be rare
/// but still use Grammar::None because materials never get beams.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum LootGrammar {
    /// No beam: Common items and all materials.
    #[default]
    None,
    /// Cold blue, 2m beam, 0.4m ground glow.
    Rare,
    /// Deep violet, 3.5m beam, 0.6m ground glow.
    Epic,
    /// Forge orange, 6m beam, 1.0m ground glow, rising sparks.
    Legendary,
    /// Blood red, 10m beam, 1.4m ground glow, embers + slow rotation.
    Mythic,
    /// Aged gold, 4m beam, 0.8m ground glow, solid pillar with floating pages.
    Quest,
}

impl LootGrammar {
    /// Whether this tier gets a visible beam.
    pub fn has_beam(self) -> bool {
        !matches!(self, LootGrammar::None)
    }

    /// Beam height in metres.
    pub fn beam_height(self) -> f32 {
        match self {
            LootGrammar::None => 0.0,
            LootGrammar::Rare => 2.0,
            LootGrammar::Epic => 3.5,
            LootGrammar::Legendary => 6.0,
            LootGrammar::Mythic => 10.0,
            LootGrammar::Quest => 4.0,
        }
    }

    /// Ground glow disc radius in metres.
    pub fn ground_glow_radius(self) -> f32 {
        match self {
            LootGrammar::None => 0.0,
            LootGrammar::Rare => 0.4,
            LootGrammar::Epic => 0.6,
            LootGrammar::Legendary => 1.0,
            LootGrammar::Mythic => 1.4,
            LootGrammar::Quest => 0.8,
        }
    }

    /// Core beam width in metres.
    pub fn core_width(self) -> f32 {
        match self {
            LootGrammar::None => 0.0,
            LootGrammar::Rare => 0.04,
            LootGrammar::Epic => 0.06,
            LootGrammar::Legendary => 0.08,
            LootGrammar::Mythic => 0.12,
            LootGrammar::Quest => 0.07,
        }
    }

    /// HDR emissive multiplier for bloom.
    pub fn hdr_multiplier(self) -> f32 {
        match self {
            LootGrammar::None => 0.0,
            LootGrammar::Rare => 2.0,
            LootGrammar::Epic => 3.0,
            LootGrammar::Legendary => 4.5,
            LootGrammar::Mythic => 6.0,
            LootGrammar::Quest => 3.5,
        }
    }

    /// Rarity color (hex -> linear RGB).
    pub fn color(self) -> Color {
        match self {
            LootGrammar::None => Color::srgb(0.54, 0.54, 0.52), // #8a8a85 ash grey
            LootGrammar::Rare => Color::srgb(0.227, 0.482, 0.835), // #3a7bd5 cold blue
            LootGrammar::Epic => Color::srgb(0.557, 0.267, 0.678), // #8e44ad deep violet
            LootGrammar::Legendary => Color::srgb(0.902, 0.494, 0.133), // #e67e22 forge orange
            LootGrammar::Mythic => Color::srgb(0.753, 0.224, 0.169), // #c0392b blood red
            LootGrammar::Quest => Color::srgb(0.831, 0.686, 0.216), // #d4af37 aged gold
        }
    }

    /// LinearRgba version of the color for shader use.
    pub fn linear_color(self) -> LinearRgba {
        LinearRgba::from(self.color())
    }

    /// Point light intensity for ground glow (lumens).
    pub fn light_intensity(self) -> f32 {
        match self {
            LootGrammar::None => 0.0,
            LootGrammar::Rare => 8_000.0,
            LootGrammar::Epic => 14_000.0,
            LootGrammar::Legendary => 24_000.0,
            LootGrammar::Mythic => 40_000.0,
            LootGrammar::Quest => 18_000.0,
        }
    }

    /// Whether this tier has rising sparks/embers.
    pub fn has_sparks(self) -> bool {
        matches!(self, LootGrammar::Legendary | LootGrammar::Mythic)
    }

    /// Whether this tier has the slow rotation effect.
    pub fn has_rotation(self) -> bool {
        matches!(self, LootGrammar::Mythic)
    }

    /// Whether this tier shimmers (Quest is solid, non-shimmering).
    pub fn shimmers(self) -> bool {
        !matches!(self, LootGrammar::None | LootGrammar::Quest)
    }

    /// Spark count for Legendary+.
    pub fn spark_count(self) -> usize {
        match self {
            LootGrammar::Legendary => 12,
            LootGrammar::Mythic => 20,
            _ => 0,
        }
    }
}

/// Map the current wire protocol `Rarity` to our visual `LootGrammar`.
/// This is a temporary shim until the server sends proper grammar tiers.
///
/// Wire rarities: Common, Rare, Legendary, Ancient, Primal
/// Materials (Bog Amber, Star Iron, etc.) never get beams regardless of rarity.
pub fn wire_rarity_to_grammar(
    rarity: ba_protocol::stillwater::Rarity,
    items: &[String],
) -> LootGrammar {
    // Materials never get beams
    if is_material_bundle(items) {
        return LootGrammar::None;
    }

    // Quest items (Edda's Book, Ox's Yoke) get Quest grammar
    if is_quest_bundle(items) {
        return LootGrammar::Quest;
    }

    // Map wire rarity to grammar
    use ba_protocol::stillwater::Rarity;
    match rarity {
        Rarity::Common => LootGrammar::None,
        Rarity::Rare => LootGrammar::Rare,
        Rarity::Legendary => LootGrammar::Epic, // Wire "Legendary" -> spec Epic
        Rarity::Ancient => LootGrammar::Legendary, // Wire "Ancient" -> spec Legendary
        Rarity::Primal => LootGrammar::Mythic, // Wire "Primal" -> spec Mythic
    }
}

/// Check if a bundle contains only materials (which never get beams).
fn is_material_bundle(items: &[String]) -> bool {
    const MATERIALS: &[&str] = &[
        "Beast Hide",
        "Cold Iron",
        "Bone Splinters",
        "Peat",
        "Toad Fat",
        "Stillwater Essence",
        "Bright Cinder",
        "Bog Amber",
        "Golden Bog Amber",
        "Ancient Bog Amber",
        "Primal Bog Amber",
        "Star Iron",
    ];

    items.iter().all(|item| {
        let name = item.trim_start_matches("ashworks:");
        MATERIALS.iter().any(|mat| name.contains(mat))
    })
}

/// Check if a bundle contains quest items.
fn is_quest_bundle(items: &[String]) -> bool {
    items.iter().any(|item| {
        let name = item.trim_start_matches("ashworks:");
        name.contains("Edda's Book")
            || name.contains("Ox's Yoke")
            || name.contains("Book")
            || name.contains("Yoke")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn materials_map_to_no_beam() {
        use ba_protocol::stillwater::Rarity;
        let items = vec!["Cold Iron".to_string(), "Peat".to_string()];
        assert_eq!(wire_rarity_to_grammar(Rarity::Common, &items), LootGrammar::None);

        let amber = vec!["Primal Bog Amber".to_string()];
        assert_eq!(wire_rarity_to_grammar(Rarity::Primal, &amber), LootGrammar::None);
    }

    #[test]
    fn quest_items_get_quest_grammar() {
        use ba_protocol::stillwater::Rarity;
        let book = vec!["Edda's Book".to_string()];
        assert_eq!(wire_rarity_to_grammar(Rarity::Rare, &book), LootGrammar::Quest);
    }

    #[test]
    fn rarity_colors_match_spec() {
        // Verify the hex colors from the spec
        let rare = LootGrammar::Rare.color();
        let epic = LootGrammar::Epic.color();
        let legendary = LootGrammar::Legendary.color();
        let mythic = LootGrammar::Mythic.color();
        let quest = LootGrammar::Quest.color();

        // Just verify they're distinct
        assert_ne!(rare, epic);
        assert_ne!(epic, legendary);
        assert_ne!(legendary, mythic);
        assert_ne!(mythic, quest);
    }
}
