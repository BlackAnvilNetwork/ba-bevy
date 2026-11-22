//! Inventory container with pure add/remove/stack logic.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{ItemId, Quality, Stack, StackRule};

/// Errors that can occur during inventory operations.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum InventoryError {
    #[error("not enough items: need {need}, have {have}")]
    NotEnough { need: u32, have: u32 },
    #[error("inventory full")]
    Full,
    #[error("item not found")]
    NotFound,
}

/// Player inventory: a collection of item stacks.
///
/// Stacking rules:
/// - Normal, HQ, and starter stacks of the same item don't merge.
/// - When consuming, normal stacks are spent before starter stacks.
/// - HQ items are tracked separately and must be explicitly requested.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Inventory {
    stacks: Vec<Stack>,
    max_slots: usize,
}

impl Inventory {
    pub const DEFAULT_MAX_SLOTS: usize = 100;

    pub fn new() -> Self {
        Self {
            stacks: Vec::new(),
            max_slots: Self::DEFAULT_MAX_SLOTS,
        }
    }

    pub fn with_capacity(max_slots: usize) -> Self {
        Self {
            stacks: Vec::new(),
            max_slots,
        }
    }

    pub fn stacks(&self) -> &[Stack] {
        &self.stacks
    }

    pub fn stacks_mut(&mut self) -> &mut Vec<Stack> {
        &mut self.stacks
    }

    /// Count total quantity of an item (all qualities, including starter).
    pub fn count(&self, item: ItemId) -> u32 {
        self.stacks
            .iter()
            .filter(|s| s.item == item)
            .map(|s| s.qty)
            .sum()
    }

    /// Count quantity of an item with specific quality and starter status.
    pub fn count_exact(&self, item: ItemId, quality: Quality, starter: bool) -> u32 {
        self.stacks
            .iter()
            .filter(|s| s.item == item && s.quality == quality && s.starter == starter)
            .map(|s| s.qty)
            .sum()
    }

    /// Count normal (non-HQ, non-starter) quantity of an item.
    pub fn count_normal(&self, item: ItemId) -> u32 {
        self.count_exact(item, Quality::Normal, false)
    }

    /// Count HQ quantity of an item.
    pub fn count_hq(&self, item: ItemId) -> u32 {
        self.count_exact(item, Quality::HighQuality, false)
    }

    /// Returns true if the inventory contains at least `qty` of the item.
    pub fn has(&self, item: ItemId, qty: u32) -> bool {
        self.count(item) >= qty
    }

    /// Add items to inventory. Returns the quantity that couldn't fit.
    pub fn add(&mut self, item: ItemId, qty: u32, stack_rule: StackRule) -> u32 {
        self.add_with_quality(item, qty, Quality::Normal, false, stack_rule)
    }

    /// Add HQ items to inventory.
    pub fn add_hq(&mut self, item: ItemId, qty: u32, stack_rule: StackRule) -> u32 {
        self.add_with_quality(item, qty, Quality::HighQuality, false, stack_rule)
    }

    /// Add starter items to inventory.
    pub fn add_starter(&mut self, item: ItemId, qty: u32, stack_rule: StackRule) -> u32 {
        self.add_with_quality(item, qty, Quality::Normal, true, stack_rule)
    }

    fn add_with_quality(
        &mut self,
        item: ItemId,
        mut qty: u32,
        quality: Quality,
        starter: bool,
        stack_rule: StackRule,
    ) -> u32 {
        let max_stack = stack_rule.max_stack();

        for stack in &mut self.stacks {
            if qty == 0 {
                break;
            }
            if stack.item == item && stack.quality == quality && stack.starter == starter {
                qty = stack.add(qty, max_stack);
            }
        }

        while qty > 0 && self.stacks.len() < self.max_slots {
            let add_qty = qty.min(max_stack);
            self.stacks.push(Stack {
                item,
                qty: add_qty,
                quality,
                starter,
            });
            qty -= add_qty;
        }

        qty
    }

    /// Remove items from inventory (normal before starter, never HQ unless explicit).
    /// Returns error if not enough items.
    pub fn remove(&mut self, item: ItemId, qty: u32) -> Result<(), InventoryError> {
        let available = self.count_normal(item) + self.count_exact(item, Quality::Normal, true);
        if available < qty {
            return Err(InventoryError::NotEnough {
                need: qty,
                have: available,
            });
        }

        let mut remaining = qty;

        for stack in &mut self.stacks {
            if remaining == 0 {
                break;
            }
            if stack.item == item && stack.quality == Quality::Normal && !stack.starter {
                remaining -= stack.remove(remaining);
            }
        }

        for stack in &mut self.stacks {
            if remaining == 0 {
                break;
            }
            if stack.item == item && stack.quality == Quality::Normal && stack.starter {
                remaining -= stack.remove(remaining);
            }
        }

        self.stacks.retain(|s| !s.is_empty());
        Ok(())
    }

    /// Remove HQ items from inventory.
    pub fn remove_hq(&mut self, item: ItemId, qty: u32) -> Result<(), InventoryError> {
        let available = self.count_hq(item);
        if available < qty {
            return Err(InventoryError::NotEnough {
                need: qty,
                have: available,
            });
        }

        let mut remaining = qty;
        for stack in &mut self.stacks {
            if remaining == 0 {
                break;
            }
            if stack.item == item && stack.quality == Quality::HighQuality {
                remaining -= stack.remove(remaining);
            }
        }

        self.stacks.retain(|s| !s.is_empty());
        Ok(())
    }

    /// Find the first stack containing this item.
    pub fn find(&self, item: ItemId) -> Option<&Stack> {
        self.stacks.iter().find(|s| s.item == item)
    }

    /// Returns true if inventory is full (no empty slots).
    pub fn is_full(&self) -> bool {
        self.stacks.len() >= self.max_slots
    }

    /// Number of occupied slots.
    pub fn slot_count(&self) -> usize {
        self.stacks.len()
    }

    /// Maximum number of slots.
    pub fn max_slots(&self) -> usize {
        self.max_slots
    }

    /// Remove empty stacks.
    pub fn compact(&mut self) {
        self.stacks.retain(|s| !s.is_empty());
    }

    /// Clear all items.
    pub fn clear(&mut self) {
        self.stacks.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_and_count() {
        let mut inv = Inventory::new();
        let item = ItemId(1);
        let rule = StackRule::Stack(99);

        let overflow = inv.add(item, 50, rule);
        assert_eq!(overflow, 0);
        assert_eq!(inv.count(item), 50);

        let overflow = inv.add(item, 60, rule);
        assert_eq!(overflow, 0);
        assert_eq!(inv.count(item), 110);
        assert_eq!(inv.slot_count(), 2);
    }

    #[test]
    fn remove_normal_before_starter() {
        let mut inv = Inventory::new();
        let item = ItemId(1);
        let rule = StackRule::Stack(99);

        inv.add_starter(item, 10, rule);
        inv.add(item, 20, rule);

        assert_eq!(inv.count(item), 30);

        inv.remove(item, 15).unwrap();
        assert_eq!(inv.count_normal(item), 5);
        assert_eq!(inv.count_exact(item, Quality::Normal, true), 10);

        inv.remove(item, 10).unwrap();
        assert_eq!(inv.count_normal(item), 0);
        assert_eq!(inv.count_exact(item, Quality::Normal, true), 5);
    }

    #[test]
    fn remove_hq_separate() {
        let mut inv = Inventory::new();
        let item = ItemId(1);
        let rule = StackRule::Stack(99);

        inv.add(item, 10, rule);
        inv.add_hq(item, 5, rule);

        assert_eq!(inv.count(item), 15);
        assert_eq!(inv.count_hq(item), 5);

        inv.remove(item, 10).unwrap();
        assert_eq!(inv.count_hq(item), 5);

        inv.remove_hq(item, 3).unwrap();
        assert_eq!(inv.count_hq(item), 2);
    }

    #[test]
    fn remove_not_enough() {
        let mut inv = Inventory::new();
        let item = ItemId(1);
        let rule = StackRule::Stack(99);

        inv.add(item, 5, rule);
        let result = inv.remove(item, 10);
        assert!(matches!(
            result,
            Err(InventoryError::NotEnough { need: 10, have: 5 })
        ));
        assert_eq!(inv.count(item), 5);
    }

    #[test]
    fn inventory_full() {
        let mut inv = Inventory::with_capacity(2);
        let rule = StackRule::Unique;

        inv.add(ItemId(1), 1, rule);
        inv.add(ItemId(2), 1, rule);
        assert!(inv.is_full());

        let overflow = inv.add(ItemId(3), 1, rule);
        assert_eq!(overflow, 1);
    }

    #[test]
    fn has_check() {
        let mut inv = Inventory::new();
        let item = ItemId(1);
        let rule = StackRule::Stack(99);

        assert!(!inv.has(item, 1));
        inv.add(item, 10, rule);
        assert!(inv.has(item, 10));
        assert!(!inv.has(item, 11));
    }
}
