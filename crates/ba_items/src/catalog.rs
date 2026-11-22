//! Item catalog: definitions and RON loader.
//!
//! The catalog is the single source of truth for item data. It is loaded at
//! startup by both server and client. IDs and slugs are frozen once assigned.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};
use smol_str::SmolStr;
use thiserror::Error;

use crate::{
    AmmoClass, EquipSlot, GripClass, ItemId, ItemKind, ItemStats, JobMask, Rarity, StackRule,
    VisualKey, WeaponFamily,
};

/// Errors that can occur when loading or validating the catalog.
#[derive(Debug, Clone, Error)]
pub enum CatalogError {
    #[error("duplicate item ID {0}")]
    DuplicateId(ItemId),
    #[error("duplicate slug '{0}'")]
    DuplicateSlug(SmolStr),
    #[error("recipe references unknown item ID {0}")]
    UnknownRecipeItem(ItemId),
    #[error("failed to parse RON: {0}")]
    ParseError(String),
    #[error("failed to read file: {0}")]
    IoError(String),
    #[error("visual key '{0}' referenced but not defined")]
    UnknownVisual(String),
}

/// Crafting recipe for an item.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Recipe {
    /// Crafting skill required (e.g. "smithing", "alchemy").
    pub skill: SmolStr,
    /// Minimum skill level to craft.
    pub level: u32,
    /// Gold cost (defaults to 50 + 20 × level).
    #[serde(default)]
    pub gold: Option<u32>,
    /// Required materials: (item_id, quantity).
    #[serde(default)]
    pub materials: Vec<(ItemId, u16)>,
    /// How many items this recipe produces.
    #[serde(default = "default_output_qty")]
    pub output_qty: u16,
}

fn default_output_qty() -> u16 {
    1
}

impl Recipe {
    /// Returns the gold cost (explicit or computed from level).
    pub fn gold_cost(&self) -> u32 {
        self.gold.unwrap_or_else(|| crate::stats::craft_gold_cost(self.level))
    }
}

/// Effect when an item is consumed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConsumeEffect {
    /// Restore HP.
    Heal { amount: i32 },
    /// Restore MP.
    RestoreMp { amount: i32 },
    /// Apply a buff.
    Buff { buff_id: SmolStr, duration_secs: f32 },
    /// Custom effect handled by game code.
    Custom { effect_id: SmolStr },
}

/// Complete definition of an item.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ItemDef {
    /// Unique numeric ID. Frozen once assigned.
    pub id: ItemId,
    /// Unique kebab-case slug. Frozen once assigned.
    pub slug: SmolStr,
    /// Display name (player-facing, dark fantasy style).
    pub name: String,
    /// Flavour description.
    #[serde(default)]
    pub description: String,
    /// What category of item this is.
    #[serde(default)]
    pub kind: ItemKind,
    /// Which equipment slot this fits (if equippable).
    #[serde(default)]
    pub slot: Option<EquipSlot>,
    /// Weapon family for combat skills (if applicable).
    #[serde(default)]
    pub family: Option<WeaponFamily>,
    /// How the weapon is gripped (determines EquipMode).
    #[serde(default)]
    pub grip: Option<GripClass>,
    /// Item rarity (explicit, never inferred from name).
    #[serde(default)]
    pub rarity: Rarity,
    /// Item level for scaling.
    #[serde(default = "default_level")]
    pub level: u32,
    /// Combat stats.
    #[serde(default)]
    pub stats: ItemStats,
    /// Attack delay in milliseconds (for weapons).
    #[serde(default)]
    pub delay_ms: Option<u32>,
    /// Job restrictions. A missing field is every job (`JobMask::ALL`).
    /// `JobMask::NONE` (0) lets nobody equip the item.
    #[serde(default)]
    pub job_restrict: JobMask,
    /// How the item stacks.
    #[serde(default)]
    pub stack: StackRule,
    /// Ammo class (for ranged weapons and ammunition).
    #[serde(default)]
    pub ammo: Option<AmmoClass>,
    /// Crafting recipe (if craftable).
    #[serde(default)]
    pub recipe: Option<Recipe>,
    /// Effect when consumed.
    #[serde(default)]
    pub consume: Option<ConsumeEffect>,
    /// Visual key for rendering (into visual library).
    #[serde(default)]
    pub visual: Option<VisualKey>,
}

fn default_level() -> u32 {
    1
}

impl ItemDef {
    /// Returns true if this item is equippable.
    pub fn is_equippable(&self) -> bool {
        self.slot.is_some()
    }

    /// Returns true if this item is a weapon.
    pub fn is_weapon(&self) -> bool {
        matches!(self.slot, Some(EquipSlot::Main | EquipSlot::Off | EquipSlot::Ranged))
    }

    /// Returns true if this item stacks.
    pub fn is_stackable(&self) -> bool {
        self.stack.can_stack()
    }
}

/// A collection of item definitions loaded from RON.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CatalogData {
    pub items: Vec<ItemDef>,
}

/// The loaded and validated item catalog.
#[derive(Debug, Clone)]
pub struct Catalog {
    items: Vec<ItemDef>,
    by_id: BTreeMap<ItemId, usize>,
    by_slug: BTreeMap<SmolStr, usize>,
}

impl Catalog {
    /// Load and validate a catalog from RON string.
    pub fn from_ron(ron_str: &str) -> Result<Self, CatalogError> {
        let data: CatalogData =
            ron::from_str(ron_str).map_err(|e| CatalogError::ParseError(e.to_string()))?;
        Self::from_items(data.items)
    }

    /// Load and validate a catalog from a RON file.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, CatalogError> {
        let content =
            std::fs::read_to_string(path).map_err(|e| CatalogError::IoError(e.to_string()))?;
        Self::from_ron(&content)
    }

    /// Build a catalog from a list of item definitions.
    pub fn from_items(items: Vec<ItemDef>) -> Result<Self, CatalogError> {
        let mut by_id = BTreeMap::new();
        let mut by_slug = BTreeMap::new();

        for (idx, item) in items.iter().enumerate() {
            if by_id.insert(item.id, idx).is_some() {
                return Err(CatalogError::DuplicateId(item.id));
            }
            if by_slug.insert(item.slug.clone(), idx).is_some() {
                return Err(CatalogError::DuplicateSlug(item.slug.clone()));
            }
        }

        for item in &items {
            if let Some(ref recipe) = item.recipe {
                for (mat_id, _) in &recipe.materials {
                    if !by_id.contains_key(mat_id) {
                        return Err(CatalogError::UnknownRecipeItem(*mat_id));
                    }
                }
            }
        }

        Ok(Self {
            items,
            by_id,
            by_slug,
        })
    }

    /// Look up an item by ID.
    pub fn get(&self, id: ItemId) -> Option<&ItemDef> {
        self.by_id.get(&id).map(|&idx| &self.items[idx])
    }

    /// Look up an item by slug.
    pub fn get_by_slug(&self, slug: &str) -> Option<&ItemDef> {
        self.by_slug.get(slug).map(|&idx| &self.items[idx])
    }

    /// Iterate over all items.
    pub fn iter(&self) -> impl Iterator<Item = &ItemDef> {
        self.items.iter()
    }

    /// Number of items in the catalog.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Returns true if the catalog is empty.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Get all item IDs in sorted order.
    pub fn ids(&self) -> impl Iterator<Item = ItemId> + '_ {
        self.by_id.keys().copied()
    }

    /// Get all slugs in sorted order.
    pub fn slugs(&self) -> impl Iterator<Item = &SmolStr> {
        self.by_slug.keys()
    }

    /// Collect ID → slug pairs for snapshot testing.
    pub fn id_slug_pairs(&self) -> Vec<(ItemId, SmolStr)> {
        let mut pairs: Vec<_> = self.items.iter().map(|i| (i.id, i.slug.clone())).collect();
        pairs.sort_by_key(|(id, _)| *id);
        pairs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_catalog_ron() -> &'static str {
        r#"
        (
            items: [
                (
                    id: 1,
                    slug: "iron-sword",
                    name: "Iron Sword",
                    description: "A simple iron blade.",
                    kind: equipment,
                    slot: Some(main),
                    family: Some(sword),
                    grip: Some(one_hand),
                    rarity: common,
                    level: 1,
                    stats: (atk: 15),
                    delay_ms: Some(2400),
                ),
                (
                    id: 2,
                    slug: "iron-ore",
                    name: "Iron Ore",
                    description: "Raw iron ore from the mines.",
                    kind: material,
                    rarity: common,
                    stack: stack(99),
                ),
                (
                    id: 3,
                    slug: "minor-healing-potion",
                    name: "Minor Healing Potion",
                    description: "Restores a small amount of health.",
                    kind: consumable,
                    rarity: common,
                    stack: stack(20),
                    consume: Some(heal(amount: 50)),
                ),
            ]
        )
        "#
    }

    #[test]
    fn load_catalog_from_ron() {
        let catalog = Catalog::from_ron(sample_catalog_ron()).unwrap();
        assert_eq!(catalog.len(), 3);

        let sword = catalog.get(ItemId(1)).unwrap();
        assert_eq!(sword.slug.as_str(), "iron-sword");
        assert_eq!(sword.name, "Iron Sword");
        assert!(sword.is_weapon());
        assert_eq!(sword.grip, Some(GripClass::OneHand));

        let ore = catalog.get_by_slug("iron-ore").unwrap();
        assert_eq!(ore.kind, ItemKind::Material);
        assert!(ore.is_stackable());

        let potion = catalog.get(ItemId(3)).unwrap();
        assert!(matches!(potion.consume, Some(ConsumeEffect::Heal { amount: 50 })));
    }

    #[test]
    fn reject_duplicate_id() {
        let ron = r#"
        (items: [
            (id: 1, slug: "sword", name: "Sword"),
            (id: 1, slug: "axe", name: "Axe"),
        ])
        "#;
        let result = Catalog::from_ron(ron);
        assert!(matches!(result, Err(CatalogError::DuplicateId(ItemId(1)))));
    }

    #[test]
    fn reject_duplicate_slug() {
        let ron = r#"
        (items: [
            (id: 1, slug: "sword", name: "Sword"),
            (id: 2, slug: "sword", name: "Other Sword"),
        ])
        "#;
        let result = Catalog::from_ron(ron);
        assert!(matches!(result, Err(CatalogError::DuplicateSlug(_))));
    }

    #[test]
    fn reject_unknown_recipe_material() {
        let ron = r#"
        (items: [
            (id: 1, slug: "sword", name: "Sword",
             recipe: Some((skill: "smithing", level: 1, materials: [(999, 2)]))),
        ])
        "#;
        let result = Catalog::from_ron(ron);
        assert!(matches!(result, Err(CatalogError::UnknownRecipeItem(ItemId(999)))));
    }

    #[test]
    fn recipe_gold_cost() {
        let recipe = Recipe {
            skill: "smithing".into(),
            level: 5,
            gold: None,
            materials: vec![],
            output_qty: 1,
        };
        assert_eq!(recipe.gold_cost(), 50 + 20 * 5);

        let explicit = Recipe {
            gold: Some(100),
            ..recipe
        };
        assert_eq!(explicit.gold_cost(), 100);
    }

    #[test]
    fn id_slug_pairs_sorted() {
        let catalog = Catalog::from_ron(sample_catalog_ron()).unwrap();
        let pairs = catalog.id_slug_pairs();
        assert_eq!(pairs.len(), 3);
        assert_eq!(pairs[0].0, ItemId(1));
        assert_eq!(pairs[1].0, ItemId(2));
        assert_eq!(pairs[2].0, ItemId(3));
    }
}
