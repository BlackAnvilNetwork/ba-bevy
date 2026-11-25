//! Crafting system handlers for zone server.
//!
//! Bridges between protocol messages and ba_items crafting logic.

use ba_items::crafting::{
    self as craft, CraftError, CraftingRanks, CraftRecipe, Durability, RefineRecipe,
};
use ba_items::element::{CraftTier, Element, MaterialCategory};
use ba_items::enchant::Enchantment;
use ba_items::{Inventory, ItemId};
use ba_protocol::crafting as proto;
use rand::Rng;

#[derive(Clone)]
pub struct CraftingState {
    pub gold: u32,
    pub ranks: CraftingRanks,
}

impl Default for CraftingState {
    fn default() -> Self {
        Self {
            gold: 1000,
            ranks: CraftingRanks::default(),
        }
    }
}

pub fn handle_refine<R: Rng>(
    state: &mut CraftingState,
    inventory: &mut Inventory,
    req: &proto::RefineRequest,
    _rng: &mut R,
) -> Result<proto::RefineResult, proto::CraftingError> {
    let category = material_category(req.material_id);
    let tier = CraftTier(material_tier(req.material_id));
    
    let recipe = RefineRecipe {
        output: ItemId(req.output_id),
        output_name: format!("Component {}", req.output_id),
        category,
        tier,
        material: ItemId(req.material_id),
        material_qty: 3,
        gold: 50 * tier.0 as u32,
    };

    let old_rank = state.ranks.get(recipe.category).rank;
    let gold_before = state.gold;

    match craft::refine(inventory, &mut state.gold, &mut state.ranks, &recipe, ItemId(req.rune_id)) {
        Ok(qty) => {
            let new_rank = state.ranks.get(recipe.category);
            let gold_spent = gold_before.saturating_sub(state.gold);
            Ok(proto::RefineResult {
                output_id: req.output_id,
                quantity: qty,
                gold_spent,
                xp_gained: new_rank.xp_for_action(tier, false),
                new_rank: if new_rank.rank > old_rank {
                    Some(new_rank.rank)
                } else {
                    None
                },
            })
        }
        Err(e) => Err(proto::CraftingError {
            action: "refine".into(),
            reason: craft_error_reason(&e),
        }),
    }
}

pub fn handle_craft<R: Rng>(
    state: &mut CraftingState,
    inventory: &mut Inventory,
    req: &proto::CraftRequest,
    _rng: &mut R,
) -> Result<proto::CraftResult, proto::CraftingError> {
    let (output_id, components, category, tier) = recipe_lookup(req.recipe_id);
    
    let recipe = CraftRecipe {
        output: ItemId(output_id),
        output_name: format!("Gear {}", output_id),
        primary_category: category,
        tier: CraftTier(tier),
        components,
        gold: 100 * tier as u32,
    };

    let old_rank = state.ranks.get(recipe.primary_category).rank;
    let gold_before = state.gold;

    match craft::craft(inventory, &mut state.gold, &mut state.ranks, &recipe, ItemId(req.rune_id)) {
        Ok(()) => {
            let new_rank = state.ranks.get(recipe.primary_category);
            let gold_spent = gold_before.saturating_sub(state.gold);
            Ok(proto::CraftResult {
                output_id,
                gold_spent,
                xp_gained: new_rank.xp_for_action(CraftTier(tier), true),
                new_rank: if new_rank.rank > old_rank {
                    Some(new_rank.rank)
                } else {
                    None
                },
            })
        }
        Err(e) => Err(proto::CraftingError {
            action: "craft".into(),
            reason: craft_error_reason(&e),
        }),
    }
}

pub fn handle_enchant<R: Rng>(
    state: &mut CraftingState,
    inventory: &mut Inventory,
    req: &proto::EnchantRequest,
    rng: &mut R,
) -> Result<proto::EnchantResult, proto::CraftingError> {
    let glyph_element = match glyph_element(req.glyph_id) {
        Some(e) => e,
        None => {
            return Err(proto::CraftingError {
                action: "enchant".into(),
                reason: "invalid_glyph".into(),
            });
        }
    };

    let glyph_id = ItemId(req.glyph_id);
    if !inventory.has(glyph_id, 1) {
        return Err(proto::CraftingError {
            action: "enchant".into(),
            reason: "missing_glyph".into(),
        });
    }

    let gear_tier = CraftTier(1); // TODO: look up actual gear tier from slot
    let gold_before = state.gold;

    match craft::enchant(&mut state.gold, glyph_element, gear_tier, rng) {
        Ok(enchantment) => {
            let _ = inventory.remove(glyph_id, 1);
            let gold_spent = gold_before.saturating_sub(state.gold);
            Ok(enchantment_to_proto(&enchantment, gold_spent))
        }
        Err(e) => Err(proto::CraftingError {
            action: "enchant".into(),
            reason: craft_error_reason(&e),
        }),
    }
}

pub fn handle_salvage<R: Rng>(
    state: &mut CraftingState,
    inventory: &mut Inventory,
    _req: &proto::SalvageRequest,
    rng: &mut R,
) -> Result<proto::SalvageResult, proto::CraftingError> {
    let primary_component = ItemId(5300); // Iron Ingot as placeholder
    let was_enchanted = false;
    let gold_before = state.gold;

    match craft::salvage(inventory, &mut state.gold, primary_component, was_enchanted, rng) {
        Ok(result) => {
            let gold_spent = gold_before.saturating_sub(state.gold);
            Ok(proto::SalvageResult {
                components: result.components.into_iter().map(|(id, qty)| (id.0, qty)).collect(),
                recovered_rune: result.recovered_rune.map(|e| format!("{:?}", e)),
                recovered_glyph: result.recovered_glyph.map(|e| format!("{:?}", e)),
                gold_spent,
            })
        }
        Err(e) => Err(proto::CraftingError {
            action: "salvage".into(),
            reason: craft_error_reason(&e),
        }),
    }
}

pub fn handle_repair(
    state: &mut CraftingState,
    durability: &mut Durability,
    _req: &proto::RepairRequest,
) -> Result<proto::RepairResult, proto::CraftingError> {
    let missing = durability.max.saturating_sub(durability.current);
    if missing == 0 {
        return Err(proto::CraftingError {
            action: "repair".into(),
            reason: "already_full".into(),
        });
    }

    let gold_cost = missing * 2;
    if state.gold < gold_cost {
        return Err(proto::CraftingError {
            action: "repair".into(),
            reason: "not_enough_gold".into(),
        });
    }

    state.gold -= gold_cost;
    durability.current = durability.max;

    Ok(proto::RepairResult {
        new_durability: durability.current,
        gold_spent: gold_cost,
    })
}

fn craft_error_reason(e: &CraftError) -> String {
    match e {
        CraftError::MissingMaterial { item, need, have } => {
            format!("missing_material_{}_need_{}_have_{}", item.0, need, have)
        }
        CraftError::MissingRune(element) => format!("missing_rune_{:?}", element),
        CraftError::MissingGlyph(element) => format!("missing_glyph_{:?}", element),
        CraftError::NotEnoughGold { need, have } => format!("not_enough_gold_need_{}_have_{}", need, have),
        CraftError::ItemNotFound(id) => format!("item_not_found_{}", id.0),
        CraftError::NotGear(id) => format!("not_gear_{}", id.0),
        CraftError::AlreadyEnchanted => "already_enchanted".into(),
        CraftError::InventoryFull => "inventory_full".into(),
        CraftError::RankTooLow { need, have } => format!("rank_too_low_need_{}_have_{}", need, have),
        CraftError::Inventory(e) => format!("inventory_error_{}", e),
    }
}

fn material_category(material_id: u32) -> MaterialCategory {
    match material_id {
        5200..=5204 => MaterialCategory::Metal,
        5210..=5214 => MaterialCategory::Leather,
        5220..=5224 => MaterialCategory::Wood,
        5230..=5234 => MaterialCategory::Cloth,
        5240..=5244 => MaterialCategory::Bone,
        5250..=5254 => MaterialCategory::PreciousMetal,
        5260..=5264 => MaterialCategory::Stone,
        5270..=5274 => MaterialCategory::Essence,
        _ => MaterialCategory::Metal,
    }
}

fn material_tier(material_id: u32) -> u8 {
    ((material_id % 10) + 1).min(5) as u8
}

fn glyph_element(glyph_id: u32) -> Option<Element> {
    match glyph_id {
        5100 => Some(Element::Fire),
        5101 => Some(Element::Ice),
        5102 => Some(Element::Wind),
        5103 => Some(Element::Water),
        5104 => Some(Element::Earth),
        5105 => Some(Element::Lightning),
        5106 => Some(Element::Light),
        5107 => Some(Element::Dark),
        _ => None,
    }
}

fn recipe_lookup(recipe_id: u32) -> (u32, Vec<(ItemId, u32)>, MaterialCategory, u8) {
    match recipe_id {
        5400 => (5400, vec![(ItemId(5300), 2)], MaterialCategory::Metal, 1),
        5401 => (5401, vec![(ItemId(5301), 2)], MaterialCategory::Metal, 2),
        5410 => (5410, vec![(ItemId(5310), 2)], MaterialCategory::Leather, 1),
        5450 => (5450, vec![(ItemId(5300), 3), (ItemId(5310), 2)], MaterialCategory::Metal, 1),
        _ => (5400, vec![(ItemId(5300), 2)], MaterialCategory::Metal, 1),
    }
}

fn enchantment_to_proto(e: &Enchantment, gold_spent: u32) -> proto::EnchantResult {
    proto::EnchantResult {
        element: format!("{:?}", e.element),
        prefix_name: e.prefix.name.clone(),
        prefix_stat: format!("{:?}", e.prefix.stat),
        prefix_percent: e.prefix.percent,
        suffix_name: e.suffix.name.clone(),
        suffix_stat: format!("{:?}", e.suffix.stat),
        suffix_percent: e.suffix.percent,
        gold_spent,
    }
}
