//! Crafting protocol messages for client-server communication.

use serde::{Deserialize, Serialize};

/// Client request to refine materials into a component.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefineRequest {
    /// ID of the rune item to consume.
    pub rune_id: u32,
    /// ID of the material to refine.
    pub material_id: u32,
    /// ID of the output component to create.
    pub output_id: u32,
}

/// Client request to craft base gear from components.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CraftRequest {
    /// ID of the rune item to consume.
    pub rune_id: u32,
    /// ID of the gear to craft.
    pub recipe_id: u32,
}

/// Client request to enchant gear with a glyph.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnchantRequest {
    /// ID of the glyph item to consume.
    pub glyph_id: u32,
    /// Inventory slot of the gear to enchant.
    pub gear_slot: u32,
}

/// Client request to salvage gear for components.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SalvageRequest {
    /// Inventory slot of the gear to salvage.
    pub gear_slot: u32,
}

/// Client request to repair gear.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepairRequest {
    /// Inventory slot of the gear to repair.
    pub gear_slot: u32,
}

/// Server response for successful refine.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefineResult {
    /// ID of the component created.
    pub output_id: u32,
    /// Quantity created (1 + yield bonus).
    pub quantity: u32,
    /// Gold spent.
    pub gold_spent: u32,
    /// XP gained in the category.
    pub xp_gained: u32,
    /// New rank if leveled up.
    pub new_rank: Option<u8>,
}

/// Server response for successful craft.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CraftResult {
    /// ID of the gear created.
    pub output_id: u32,
    /// Gold spent.
    pub gold_spent: u32,
    /// XP gained in the category.
    pub xp_gained: u32,
    /// New rank if leveled up.
    pub new_rank: Option<u8>,
}

/// Server response for successful enchant.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnchantResult {
    /// Element of the glyph used.
    pub element: String,
    /// Prefix name rolled.
    pub prefix_name: String,
    /// Prefix stat type.
    pub prefix_stat: String,
    /// Prefix percentage value.
    pub prefix_percent: u8,
    /// Suffix name rolled.
    pub suffix_name: String,
    /// Suffix stat type.
    pub suffix_stat: String,
    /// Suffix percentage value.
    pub suffix_percent: u8,
    /// Gold spent.
    pub gold_spent: u32,
}

/// Server response for successful salvage.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SalvageResult {
    /// Components recovered (item_id, quantity).
    pub components: Vec<(u32, u32)>,
    /// Rune element recovered, if any.
    pub recovered_rune: Option<String>,
    /// Glyph element recovered, if any.
    pub recovered_glyph: Option<String>,
    /// Gold spent on salvage fee.
    pub gold_spent: u32,
}

/// Server response for successful repair.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepairResult {
    /// New durability after repair.
    pub new_durability: u32,
    /// Gold spent.
    pub gold_spent: u32,
}

/// Crafting rank for a single material category.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryRankWire {
    pub category: String,
    pub rank: u8,
    pub xp: u32,
    pub xp_to_next: Option<u32>,
}

/// All crafting ranks for a player.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CraftingRanksWire {
    pub metal: CategoryRankWire,
    pub wood: CategoryRankWire,
    pub leather: CategoryRankWire,
    pub cloth: CategoryRankWire,
    pub bone: CategoryRankWire,
    pub precious_metal: CategoryRankWire,
    pub stone: CategoryRankWire,
    pub essence: CategoryRankWire,
}

impl Default for CategoryRankWire {
    fn default() -> Self {
        Self {
            category: String::new(),
            rank: 1,
            xp: 0,
            xp_to_next: Some(100),
        }
    }
}

/// Crafting-related view sent in Snapshot.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CraftingView {
    /// Player's gold balance.
    pub gold: u32,
    /// Player's crafting ranks.
    pub ranks: CraftingRanksWire,
    /// Whether player is near a crafting station.
    pub near_station: Option<String>,
}

/// Error response for failed crafting action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CraftingError {
    pub action: String,
    pub reason: String,
}
