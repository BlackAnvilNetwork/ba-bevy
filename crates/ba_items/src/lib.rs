//! Black Anvil item system crate.
//!
//! This crate provides the foundational types and logic for items, inventory,
//! and equipment. It is designed to be shared by the server, gateway, and client
//! without depending on Bevy or any rendering code.
//!
//! # Design Principles
//!
//! - **IDs are data, never strings in code.** No `contains("spear")` patterns.
//! - **Rules and visuals are separate.** An `ItemDef` (rules) points to a visual key.
//! - **Pure functions.** Logic is testable without game state.
//! - **Frozen IDs.** Once an item exists, its ID and slug never change.

mod catalog;
mod equipment;
mod inventory;
mod stack;
mod stats;
mod types;
mod visual;

#[cfg(test)]
mod frozen;

pub use catalog::{Catalog, CatalogError, ItemDef, Recipe};
pub use equipment::{Equipment, EquipError, EquippedItem};
pub use inventory::{Inventory, InventoryError};
pub use stack::{Quality, Stack, StackRule};
pub use stats::ItemStats;
pub use types::{
    AmmoClass, EquipSlot, GripClass, ItemId, ItemKind, JobMask, Rarity, WeaponFamily,
};
pub use visual::{GripAxis, GripTransform, VisualDef, VisualKey};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crate_compiles_and_exports_core_types() {
        let id = ItemId(1);
        assert_eq!(id.0, 1);
        let kind = ItemKind::Equipment;
        assert!(matches!(kind, ItemKind::Equipment));
    }
}
