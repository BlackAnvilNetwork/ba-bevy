//! Frozen ID/slug tests to ensure item identifiers never change.
//!
//! Once an item is added to the catalog, its ID and slug are frozen forever.
//! External systems (gateway, persistence, future integrations) depend on
//! ID/slug stability.
//!
//! If this test fails, you have either:
//! 1. Changed an existing ID or slug (NEVER DO THIS)
//! 2. Added a new item (run `cargo insta review` to accept)
//! 3. Removed an item (IDs should be deprecated, not removed)

use crate::Catalog;

const CATALOG_RON: &str = include_str!("../../../assets/items/catalog.ron");

#[test]
fn catalog_loads_successfully() {
    let catalog = Catalog::from_ron(CATALOG_RON).expect("catalog should parse");
    assert!(!catalog.is_empty(), "catalog should have items");
}

#[test]
fn frozen_id_slug_pairs() {
    let catalog = Catalog::from_ron(CATALOG_RON).expect("catalog should parse");
    let pairs = catalog.id_slug_pairs();

    insta::assert_ron_snapshot!("frozen_ids", pairs);
}

#[test]
fn no_duplicate_ids() {
    let catalog = Catalog::from_ron(CATALOG_RON).expect("catalog should parse");
    let ids: Vec<_> = catalog.ids().collect();
    let unique: std::collections::HashSet<_> = ids.iter().collect();
    assert_eq!(ids.len(), unique.len(), "all IDs must be unique");
}

#[test]
fn no_duplicate_slugs() {
    let catalog = Catalog::from_ron(CATALOG_RON).expect("catalog should parse");
    let slugs: Vec<_> = catalog.slugs().cloned().collect();
    let unique: std::collections::HashSet<_> = slugs.iter().collect();
    assert_eq!(slugs.len(), unique.len(), "all slugs must be unique");
}

#[test]
fn all_equippable_items_have_slot() {
    let catalog = Catalog::from_ron(CATALOG_RON).expect("catalog should parse");
    for item in catalog.iter() {
        if item.kind == crate::ItemKind::Equipment {
            assert!(
                item.slot.is_some(),
                "equipment item '{}' (id={}) must have a slot",
                item.slug,
                item.id
            );
        }
    }
}

#[test]
fn weapons_have_grip_class() {
    let catalog = Catalog::from_ron(CATALOG_RON).expect("catalog should parse");
    for item in catalog.iter() {
        if item.is_weapon() && item.kind == crate::ItemKind::Equipment {
            assert!(
                item.grip.is_some(),
                "weapon '{}' (id={}) must have a grip class",
                item.slug,
                item.id
            );
        }
    }
}

#[test]
fn ammo_items_have_ammo_class() {
    let catalog = Catalog::from_ron(CATALOG_RON).expect("catalog should parse");
    for item in catalog.iter() {
        if item.kind == crate::ItemKind::Ammo {
            assert!(
                item.ammo.is_some(),
                "ammo item '{}' (id={}) must have an ammo class",
                item.slug,
                item.id
            );
        }
    }
}

#[test]
fn ranged_weapons_have_ammo_class() {
    let catalog = Catalog::from_ron(CATALOG_RON).expect("catalog should parse");
    for item in catalog.iter() {
        if item.slot == Some(crate::EquipSlot::Ranged) && item.kind == crate::ItemKind::Equipment {
            assert!(
                item.ammo.is_some(),
                "ranged weapon '{}' (id={}) must have an ammo class",
                item.slug,
                item.id
            );
        }
    }
}

#[test]
fn consumables_have_consume_effect() {
    let catalog = Catalog::from_ron(CATALOG_RON).expect("catalog should parse");
    for item in catalog.iter() {
        if item.kind == crate::ItemKind::Consumable {
            assert!(
                item.consume.is_some(),
                "consumable '{}' (id={}) must have a consume effect",
                item.slug,
                item.id
            );
        }
    }
}
