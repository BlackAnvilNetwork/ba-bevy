//! Crafting system: refine, craft, enchant, salvage.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::element::{CraftTier, Element, MaterialCategory};
use crate::enchant::{get_pool, Enchantment};
use crate::{Inventory, InventoryError, ItemId, StackRule};

/// Errors that can occur during crafting operations.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum CraftError {
    #[error("not enough gold: need {need}, have {have}")]
    NotEnoughGold { need: u32, have: u32 },
    #[error("missing rune: need {0:?} Rune")]
    MissingRune(Element),
    #[error("missing glyph: need {0:?} Glyph")]
    MissingGlyph(Element),
    #[error("missing material: need {need} of item {item}, have {have}")]
    MissingMaterial { item: ItemId, need: u32, have: u32 },
    #[error("item not found: {0}")]
    ItemNotFound(ItemId),
    #[error("item not gear: {0}")]
    NotGear(ItemId),
    #[error("item already enchanted")]
    AlreadyEnchanted,
    #[error("inventory full")]
    InventoryFull,
    #[error("crafting rank too low: need {need}, have {have}")]
    RankTooLow { need: u8, have: u8 },
    #[error("inventory error: {0}")]
    Inventory(#[from] InventoryError),
}

/// Crafting rank for a single material category (1-10).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CategoryRank {
    pub category: MaterialCategory,
    pub rank: u8,
    pub xp: u32,
}

impl CategoryRank {
    pub fn new(category: MaterialCategory) -> Self {
        Self {
            category,
            rank: 1,
            xp: 0,
        }
    }

    pub fn xp_to_next(&self) -> Option<u32> {
        match self.rank {
            1 => Some(100),
            2 => Some(250),
            3 => Some(500),
            4 => Some(1000),
            5 => Some(2000),
            6 => Some(3500),
            7 => Some(5500),
            8 => Some(8000),
            9 => Some(12000),
            10 => None,
            _ => None,
        }
    }

    pub fn add_xp(&mut self, xp: u32) {
        self.xp += xp;
        while let Some(needed) = self.xp_to_next() {
            if self.xp >= needed {
                self.xp -= needed;
                self.rank += 1;
            } else {
                break;
            }
        }
    }

    pub fn gold_discount(&self) -> f32 {
        match self.rank {
            1 => 0.0,
            2 => 0.02,
            3 => 0.04,
            4 => 0.06,
            5 => 0.08,
            6 => 0.10,
            7 => 0.12,
            8 => 0.14,
            9 => 0.16,
            10 => 0.20,
            _ => 0.0,
        }
    }

    pub fn yield_bonus(&self) -> u32 {
        match self.rank {
            7 | 8 => 1,
            9 | 10 => 2,
            _ => 0,
        }
    }

    pub fn enchant_range_bonus(&self) -> f32 {
        if self.rank >= 10 {
            0.05
        } else {
            0.0
        }
    }

    pub fn can_craft_tier(&self, tier: CraftTier) -> bool {
        self.rank >= tier.0
    }

    pub fn xp_for_action(&self, action_tier: CraftTier, is_craft: bool) -> u32 {
        let base = if is_craft { 50 } else { 25 };
        let diff = self.rank as i8 - action_tier.0 as i8;
        match diff {
            0 => base,
            1 => base / 2,
            2 => base / 8,
            _ => 0,
        }
    }
}

/// All crafting ranks for a player.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CraftingRanks {
    pub metal: CategoryRank,
    pub wood: CategoryRank,
    pub leather: CategoryRank,
    pub cloth: CategoryRank,
    pub bone: CategoryRank,
    pub precious_metal: CategoryRank,
    pub stone: CategoryRank,
    pub essence: CategoryRank,
}

impl Default for CraftingRanks {
    fn default() -> Self {
        Self {
            metal: CategoryRank::new(MaterialCategory::Metal),
            wood: CategoryRank::new(MaterialCategory::Wood),
            leather: CategoryRank::new(MaterialCategory::Leather),
            cloth: CategoryRank::new(MaterialCategory::Cloth),
            bone: CategoryRank::new(MaterialCategory::Bone),
            precious_metal: CategoryRank::new(MaterialCategory::PreciousMetal),
            stone: CategoryRank::new(MaterialCategory::Stone),
            essence: CategoryRank::new(MaterialCategory::Essence),
        }
    }
}

impl CraftingRanks {
    pub fn get(&self, category: MaterialCategory) -> &CategoryRank {
        match category {
            MaterialCategory::Metal => &self.metal,
            MaterialCategory::Wood => &self.wood,
            MaterialCategory::Leather => &self.leather,
            MaterialCategory::Cloth => &self.cloth,
            MaterialCategory::Bone => &self.bone,
            MaterialCategory::PreciousMetal => &self.precious_metal,
            MaterialCategory::Stone => &self.stone,
            MaterialCategory::Essence => &self.essence,
        }
    }

    pub fn get_mut(&mut self, category: MaterialCategory) -> &mut CategoryRank {
        match category {
            MaterialCategory::Metal => &mut self.metal,
            MaterialCategory::Wood => &mut self.wood,
            MaterialCategory::Leather => &mut self.leather,
            MaterialCategory::Cloth => &mut self.cloth,
            MaterialCategory::Bone => &mut self.bone,
            MaterialCategory::PreciousMetal => &mut self.precious_metal,
            MaterialCategory::Stone => &mut self.stone,
            MaterialCategory::Essence => &mut self.essence,
        }
    }
}

/// A refine recipe: rune + 3 materials -> component.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RefineRecipe {
    pub output: ItemId,
    pub output_name: String,
    pub category: MaterialCategory,
    pub tier: CraftTier,
    pub material: ItemId,
    pub material_qty: u32,
    pub gold: u32,
}

impl RefineRecipe {
    pub fn required_rune(&self) -> Element {
        self.category.required_rune()
    }
}

/// A craft recipe: rune + components -> base gear.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CraftRecipe {
    pub output: ItemId,
    pub output_name: String,
    pub primary_category: MaterialCategory,
    pub tier: CraftTier,
    pub components: Vec<(ItemId, u32)>,
    pub gold: u32,
}

impl CraftRecipe {
    pub fn required_rune(&self) -> Element {
        self.primary_category.required_rune()
    }
}

/// Result of a salvage operation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SalvageResult {
    pub components: Vec<(ItemId, u32)>,
    pub recovered_rune: Option<Element>,
    pub recovered_glyph: Option<Element>,
}

/// Execute a refine action.
pub fn refine(
    inventory: &mut Inventory,
    gold: &mut u32,
    ranks: &mut CraftingRanks,
    recipe: &RefineRecipe,
    rune_id: ItemId,
) -> Result<u32, CraftError> {
    let rank = ranks.get(recipe.category);

    if !rank.can_craft_tier(recipe.tier) {
        return Err(CraftError::RankTooLow {
            need: recipe.tier.0,
            have: rank.rank,
        });
    }

    let discount = rank.gold_discount();
    let cost = ((recipe.gold as f32) * (1.0 - discount)).ceil() as u32;

    if *gold < cost {
        return Err(CraftError::NotEnoughGold {
            need: cost,
            have: *gold,
        });
    }

    if !inventory.has(rune_id, 1) {
        return Err(CraftError::MissingRune(recipe.required_rune()));
    }

    let mat_count = inventory.count(recipe.material);
    if mat_count < recipe.material_qty {
        return Err(CraftError::MissingMaterial {
            item: recipe.material,
            need: recipe.material_qty,
            have: mat_count,
        });
    }

    inventory.remove(rune_id, 1)?;
    inventory.remove(recipe.material, recipe.material_qty)?;
    *gold -= cost;

    let yield_bonus = rank.yield_bonus();
    let output_qty = 1 + yield_bonus;

    let overflow = inventory.add(recipe.output, output_qty, StackRule::Stack(99));
    if overflow > 0 {
        inventory.add(rune_id, 1, StackRule::Stack(99));
        inventory.add(recipe.material, recipe.material_qty, StackRule::Stack(99));
        *gold += cost;
        return Err(CraftError::InventoryFull);
    }

    let xp = rank.xp_for_action(recipe.tier, false);
    ranks.get_mut(recipe.category).add_xp(xp);

    Ok(output_qty)
}

/// Execute a craft action.
pub fn craft(
    inventory: &mut Inventory,
    gold: &mut u32,
    ranks: &mut CraftingRanks,
    recipe: &CraftRecipe,
    rune_id: ItemId,
) -> Result<(), CraftError> {
    let rank = ranks.get(recipe.primary_category);

    if !rank.can_craft_tier(recipe.tier) {
        return Err(CraftError::RankTooLow {
            need: recipe.tier.0,
            have: rank.rank,
        });
    }

    let discount = rank.gold_discount();
    let cost = ((recipe.gold as f32) * (1.0 - discount)).ceil() as u32;

    if *gold < cost {
        return Err(CraftError::NotEnoughGold {
            need: cost,
            have: *gold,
        });
    }

    if !inventory.has(rune_id, 1) {
        return Err(CraftError::MissingRune(recipe.required_rune()));
    }

    for (item, qty) in &recipe.components {
        let count = inventory.count(*item);
        if count < *qty {
            return Err(CraftError::MissingMaterial {
                item: *item,
                need: *qty,
                have: count,
            });
        }
    }

    inventory.remove(rune_id, 1)?;
    for (item, qty) in &recipe.components {
        inventory.remove(*item, *qty)?;
    }
    *gold -= cost;

    let overflow = inventory.add(recipe.output, 1, StackRule::Unique);
    if overflow > 0 {
        inventory.add(rune_id, 1, StackRule::Stack(99));
        for (item, qty) in &recipe.components {
            inventory.add(*item, *qty, StackRule::Stack(99));
        }
        *gold += cost;
        return Err(CraftError::InventoryFull);
    }

    let xp = rank.xp_for_action(recipe.tier, true);
    ranks.get_mut(recipe.primary_category).add_xp(xp);

    Ok(())
}

/// Execute an enchant action.
pub fn enchant(
    gold: &mut u32,
    glyph_element: Element,
    gear_tier: CraftTier,
    rng: &mut impl rand::Rng,
) -> Result<Enchantment, CraftError> {
    let cost = gear_tier.enchant_gold_cost();

    if *gold < cost {
        return Err(CraftError::NotEnoughGold {
            need: cost,
            have: *gold,
        });
    }

    *gold -= cost;

    let pool = get_pool(glyph_element);
    Ok(pool.roll(rng))
}

/// Execute a salvage action.
pub fn salvage(
    inventory: &mut Inventory,
    gold: &mut u32,
    primary_component: ItemId,
    was_enchanted: bool,
    rng: &mut impl rand::Rng,
) -> Result<SalvageResult, CraftError> {
    const SALVAGE_COST: u32 = 50;

    if *gold < SALVAGE_COST {
        return Err(CraftError::NotEnoughGold {
            need: SALVAGE_COST,
            have: *gold,
        });
    }

    *gold -= SALVAGE_COST;

    let component_qty = rng.gen_range(1..=2);
    inventory.add(primary_component, component_qty, StackRule::Stack(99));

    let recovered_rune = if rng.gen_bool(0.25) {
        Some(Element::Fire)
    } else {
        None
    };

    let recovered_glyph = if was_enchanted && rng.gen_bool(0.10) {
        Some(Element::Fire)
    } else {
        None
    };

    Ok(SalvageResult {
        components: vec![(primary_component, component_qty)],
        recovered_rune,
        recovered_glyph,
    })
}

/// Durability tracking for gear.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Durability {
    pub current: u32,
    pub max: u32,
}

impl Durability {
    pub fn new(tier: CraftTier) -> Self {
        let max = tier.max_durability();
        Self { current: max, max }
    }

    pub fn damage(&mut self, amount: u32) {
        self.current = self.current.saturating_sub(amount);
    }

    pub fn is_broken(&self) -> bool {
        self.current == 0
    }

    pub fn percent(&self) -> f32 {
        if self.max == 0 {
            0.0
        } else {
            self.current as f32 / self.max as f32
        }
    }

    pub fn repair_cost(&self, tier: CraftTier) -> u32 {
        let missing_percent = 1.0 - self.percent();
        let quarters = (missing_percent * 4.0).ceil() as u32;
        quarters * 10 * tier.0 as u32
    }

    pub fn repair(&mut self) {
        self.current = self.max;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn category_rank_xp_progression() {
        let mut rank = CategoryRank::new(MaterialCategory::Metal);
        assert_eq!(rank.rank, 1);
        assert_eq!(rank.xp, 0);

        rank.add_xp(100);
        assert_eq!(rank.rank, 2);
        assert_eq!(rank.xp, 0);

        rank.add_xp(250);
        assert_eq!(rank.rank, 3);
    }

    #[test]
    fn category_rank_gold_discount() {
        let mut rank = CategoryRank::new(MaterialCategory::Metal);
        assert_eq!(rank.gold_discount(), 0.0);

        rank.rank = 5;
        assert!((rank.gold_discount() - 0.08).abs() < 0.001);

        rank.rank = 10;
        assert!((rank.gold_discount() - 0.20).abs() < 0.001);
    }

    #[test]
    fn category_rank_yield_bonus() {
        let mut rank = CategoryRank::new(MaterialCategory::Metal);
        assert_eq!(rank.yield_bonus(), 0);

        rank.rank = 7;
        assert_eq!(rank.yield_bonus(), 1);

        rank.rank = 10;
        assert_eq!(rank.yield_bonus(), 2);
    }

    #[test]
    fn durability_damage_and_repair() {
        let mut dur = Durability::new(CraftTier::T3);
        assert_eq!(dur.max, 100);
        assert_eq!(dur.current, 100);
        assert!(!dur.is_broken());

        dur.damage(50);
        assert_eq!(dur.current, 50);
        assert!((dur.percent() - 0.5).abs() < 0.001);

        dur.damage(100);
        assert!(dur.is_broken());

        dur.repair();
        assert_eq!(dur.current, 100);
    }

    #[test]
    fn durability_repair_cost() {
        let mut dur = Durability::new(CraftTier::T3);
        dur.damage(50);
        let cost = dur.repair_cost(CraftTier::T3);
        assert!(cost > 0);
    }

    #[test]
    fn crafting_ranks_default() {
        let ranks = CraftingRanks::default();
        assert_eq!(ranks.metal.rank, 1);
        assert_eq!(ranks.cloth.rank, 1);
        assert_eq!(ranks.essence.rank, 1);
    }
}
