//! Item stacking types and rules.

use serde::{Deserialize, Serialize};

use crate::ItemId;

/// Quality of a crafted item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Quality {
    #[default]
    Normal,
    HighQuality,
}

impl Quality {
    pub fn is_hq(self) -> bool {
        matches!(self, Self::HighQuality)
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Normal => "NQ",
            Self::HighQuality => "HQ",
        }
    }
}

/// How an item stacks in inventory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StackRule {
    /// Item stacks up to this maximum per slot.
    Stack(u32),
    /// Item does not stack; each instance is unique.
    Unique,
}

impl Default for StackRule {
    fn default() -> Self {
        Self::Stack(99)
    }
}

impl StackRule {
    pub const DEFAULT_MAX: u32 = 99;

    /// Returns the maximum stack size, or 1 for unique items.
    pub fn max_stack(self) -> u32 {
        match self {
            Self::Stack(max) => max,
            Self::Unique => 1,
        }
    }

    /// Returns true if this item can stack.
    pub fn can_stack(self) -> bool {
        matches!(self, Self::Stack(_))
    }
}

/// A stack of items in inventory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stack {
    pub item: ItemId,
    pub qty: u32,
    pub quality: Quality,
    /// Starter items don't merge with normal items and are spent last.
    pub starter: bool,
}

impl Stack {
    pub fn new(item: ItemId, qty: u32) -> Self {
        Self {
            item,
            qty,
            quality: Quality::Normal,
            starter: false,
        }
    }

    pub fn new_hq(item: ItemId, qty: u32) -> Self {
        Self {
            item,
            qty,
            quality: Quality::HighQuality,
            starter: false,
        }
    }

    pub fn new_starter(item: ItemId, qty: u32) -> Self {
        Self {
            item,
            qty,
            quality: Quality::Normal,
            starter: true,
        }
    }

    /// Returns true if this stack can merge with another of the same item.
    pub fn can_merge_with(&self, other: &Stack) -> bool {
        self.item == other.item && self.quality == other.quality && self.starter == other.starter
    }

    /// Returns true if this is an HQ stack.
    pub fn is_hq(&self) -> bool {
        self.quality.is_hq()
    }

    /// Returns true if this stack is empty.
    pub fn is_empty(&self) -> bool {
        self.qty == 0
    }

    /// Add quantity, capped at max_stack. Returns overflow.
    pub fn add(&mut self, qty: u32, max_stack: u32) -> u32 {
        let space = max_stack.saturating_sub(self.qty);
        let added = qty.min(space);
        self.qty += added;
        qty - added
    }

    /// Remove quantity. Returns how many were actually removed.
    pub fn remove(&mut self, qty: u32) -> u32 {
        let removed = qty.min(self.qty);
        self.qty -= removed;
        removed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stack_add_overflow() {
        let mut stack = Stack::new(ItemId(1), 90);
        let overflow = stack.add(20, 99);
        assert_eq!(stack.qty, 99);
        assert_eq!(overflow, 11);
    }

    #[test]
    fn stack_remove() {
        let mut stack = Stack::new(ItemId(1), 10);
        let removed = stack.remove(5);
        assert_eq!(removed, 5);
        assert_eq!(stack.qty, 5);
        let removed = stack.remove(10);
        assert_eq!(removed, 5);
        assert_eq!(stack.qty, 0);
    }

    #[test]
    fn stack_merge_rules() {
        let normal = Stack::new(ItemId(1), 10);
        let hq = Stack::new_hq(ItemId(1), 10);
        let starter = Stack::new_starter(ItemId(1), 10);
        let other_normal = Stack::new(ItemId(1), 5);

        assert!(normal.can_merge_with(&other_normal));
        assert!(!normal.can_merge_with(&hq));
        assert!(!normal.can_merge_with(&starter));
        assert!(!hq.can_merge_with(&starter));
    }

    #[test]
    fn stack_rule_max() {
        assert_eq!(StackRule::Stack(64).max_stack(), 64);
        assert_eq!(StackRule::Unique.max_stack(), 1);
        assert!(StackRule::Stack(99).can_stack());
        assert!(!StackRule::Unique.can_stack());
    }
}
