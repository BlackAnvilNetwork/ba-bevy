//! Equipment slots and equip mode derivation.
//!
//! The key improvement here: `EquipMode` is derived from the actual equipped
//! items' grip classes, not guessed from string patterns in item names.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{EquipSlot, GripClass, ItemId, JobMask, Quality};

/// An item equipped in a slot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EquippedItem {
    pub item: ItemId,
    pub quality: Quality,
}

impl EquippedItem {
    pub fn new(item: ItemId) -> Self {
        Self {
            item,
            quality: Quality::Normal,
        }
    }

    pub fn new_hq(item: ItemId) -> Self {
        Self {
            item,
            quality: Quality::HighQuality,
        }
    }
}

/// How the character is armed. Derived from equipped items, never guessed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EquipMode {
    #[default]
    Unarmed,
    OneHand,
    DualWield,
    /// Great swords, great axes - both hands on one weapon.
    GreatWeapon,
    /// Staves, spears, polearms - both hands on a shaft.
    PoleArm,
    /// One-handed weapon with a shield.
    OneHandShield,
    /// Bow or crossbow equipped.
    Ranged,
}

impl EquipMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Unarmed => "Hand to hand",
            Self::OneHand => "One-handed",
            Self::DualWield => "Dual wield",
            Self::GreatWeapon => "Great weapon",
            Self::PoleArm => "Pole arm",
            Self::OneHandShield => "Sword and shield",
            Self::Ranged => "Ranged",
        }
    }

    /// Returns true if this mode uses two hands.
    pub fn is_two_handed(self) -> bool {
        matches!(self, Self::GreatWeapon | Self::PoleArm | Self::Ranged)
    }

    /// Returns true if the right hand holds a weapon.
    pub fn has_main_weapon(self) -> bool {
        !matches!(self, Self::Unarmed)
    }

    /// Returns true if the left hand holds its own weapon (dual wield).
    pub fn has_off_weapon(self) -> bool {
        matches!(self, Self::DualWield)
    }

    /// Returns true if the left hand holds a shield.
    pub fn has_shield(self) -> bool {
        matches!(self, Self::OneHandShield)
    }
}

/// Errors that can occur during equip operations.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum EquipError {
    #[error("item cannot be equipped in slot {slot:?}")]
    WrongSlot { slot: EquipSlot },
    #[error("two-handed weapon conflicts with off-hand")]
    TwoHandConflict,
    #[error("job restriction: item requires {required:?}")]
    JobRestricted { required: JobMask },
    #[error("slot already occupied")]
    SlotOccupied,
}

/// A context for checking equip compatibility.
/// This is provided by the catalog, not stored in Equipment.
pub struct EquipContext {
    pub slot: Option<EquipSlot>,
    pub grip: Option<GripClass>,
    pub job_restrict: JobMask,
}

/// Equipment state: what's in each slot.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Equipment {
    head: Option<EquippedItem>,
    body: Option<EquippedItem>,
    hands: Option<EquippedItem>,
    legs: Option<EquippedItem>,
    feet: Option<EquippedItem>,
    ring: Option<EquippedItem>,
    ring2: Option<EquippedItem>,
    earring: Option<EquippedItem>,
    main: Option<EquippedItem>,
    off: Option<EquippedItem>,
    ranged: Option<EquippedItem>,
    ammo: Option<EquippedItem>,
    back: Option<EquippedItem>,
    tool: Option<EquippedItem>,
    main_grip: Option<GripClass>,
    off_grip: Option<GripClass>,
    ranged_grip: Option<GripClass>,
}

impl Equipment {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the item in a slot.
    pub fn get(&self, slot: EquipSlot) -> Option<&EquippedItem> {
        match slot {
            EquipSlot::Head => self.head.as_ref(),
            EquipSlot::Body => self.body.as_ref(),
            EquipSlot::Hands => self.hands.as_ref(),
            EquipSlot::Legs => self.legs.as_ref(),
            EquipSlot::Feet => self.feet.as_ref(),
            EquipSlot::Ring => self.ring.as_ref(),
            EquipSlot::Ring2 => self.ring2.as_ref(),
            EquipSlot::Earring => self.earring.as_ref(),
            EquipSlot::Main => self.main.as_ref(),
            EquipSlot::Off => self.off.as_ref(),
            EquipSlot::Ranged => self.ranged.as_ref(),
            EquipSlot::Ammo => self.ammo.as_ref(),
            EquipSlot::Back => self.back.as_ref(),
            EquipSlot::Tool => self.tool.as_ref(),
        }
    }

    /// Check if equipping an item is valid, given its context.
    pub fn can_equip(
        &self,
        slot: EquipSlot,
        ctx: &EquipContext,
        player_job: JobMask,
    ) -> Result<(), EquipError> {
        if let Some(required_slot) = ctx.slot {
            let valid = match (required_slot, slot) {
                (EquipSlot::Ring, EquipSlot::Ring2) => true,
                (a, b) => a == b,
            };
            if !valid {
                return Err(EquipError::WrongSlot { slot });
            }
        }

        if !ctx.job_restrict.allows(player_job) {
            return Err(EquipError::JobRestricted {
                required: ctx.job_restrict,
            });
        }

        if let Some(grip) = ctx.grip {
            if grip.is_two_handed() && self.off.is_some() {
                return Err(EquipError::TwoHandConflict);
            }
            if slot == EquipSlot::Off {
                if let Some(main_grip) = self.main_grip {
                    if main_grip.is_two_handed() {
                        return Err(EquipError::TwoHandConflict);
                    }
                }
            }
        }

        Ok(())
    }

    /// Equip an item in a slot, storing its grip class.
    /// Returns the previously equipped item, if any.
    pub fn equip(
        &mut self,
        slot: EquipSlot,
        item: EquippedItem,
        grip: Option<GripClass>,
    ) -> Option<EquippedItem> {
        let old = self.unequip(slot);

        let slot_ref = match slot {
            EquipSlot::Head => &mut self.head,
            EquipSlot::Body => &mut self.body,
            EquipSlot::Hands => &mut self.hands,
            EquipSlot::Legs => &mut self.legs,
            EquipSlot::Feet => &mut self.feet,
            EquipSlot::Ring => &mut self.ring,
            EquipSlot::Ring2 => &mut self.ring2,
            EquipSlot::Earring => &mut self.earring,
            EquipSlot::Main => &mut self.main,
            EquipSlot::Off => &mut self.off,
            EquipSlot::Ranged => &mut self.ranged,
            EquipSlot::Ammo => &mut self.ammo,
            EquipSlot::Back => &mut self.back,
            EquipSlot::Tool => &mut self.tool,
        };
        *slot_ref = Some(item);

        match slot {
            EquipSlot::Main => self.main_grip = grip,
            EquipSlot::Off => self.off_grip = grip,
            EquipSlot::Ranged => self.ranged_grip = grip,
            _ => {}
        }

        old
    }

    /// Unequip an item from a slot. Returns the item if present.
    pub fn unequip(&mut self, slot: EquipSlot) -> Option<EquippedItem> {
        let old = match slot {
            EquipSlot::Head => self.head.take(),
            EquipSlot::Body => self.body.take(),
            EquipSlot::Hands => self.hands.take(),
            EquipSlot::Legs => self.legs.take(),
            EquipSlot::Feet => self.feet.take(),
            EquipSlot::Ring => self.ring.take(),
            EquipSlot::Ring2 => self.ring2.take(),
            EquipSlot::Earring => self.earring.take(),
            EquipSlot::Main => self.main.take(),
            EquipSlot::Off => self.off.take(),
            EquipSlot::Ranged => self.ranged.take(),
            EquipSlot::Ammo => self.ammo.take(),
            EquipSlot::Back => self.back.take(),
            EquipSlot::Tool => self.tool.take(),
        };

        match slot {
            EquipSlot::Main => self.main_grip = None,
            EquipSlot::Off => self.off_grip = None,
            EquipSlot::Ranged => self.ranged_grip = None,
            _ => {}
        }

        old
    }

    /// Derive the equip mode from currently equipped items.
    /// This is the key function that replaces string-based guessing.
    pub fn equip_mode(&self) -> EquipMode {
        if self.ranged.is_some() {
            return EquipMode::Ranged;
        }

        match (self.main.as_ref(), self.main_grip) {
            (None, _) => EquipMode::Unarmed,
            (Some(_), Some(GripClass::TwoHandGreat)) => EquipMode::GreatWeapon,
            (Some(_), Some(GripClass::TwoHandPole)) => EquipMode::PoleArm,
            (Some(_), Some(GripClass::Bow)) => EquipMode::Ranged,
            (Some(_), _) => {
                match self.off_grip {
                    Some(GripClass::Shield) => EquipMode::OneHandShield,
                    Some(GripClass::OneHand) => EquipMode::DualWield,
                    _ if self.off.is_some() => EquipMode::DualWield,
                    _ => EquipMode::OneHand,
                }
            }
        }
    }

    /// Get the grip class of the main-hand weapon.
    pub fn main_grip(&self) -> Option<GripClass> {
        self.main_grip
    }

    /// Get the grip class of the off-hand item.
    pub fn off_grip(&self) -> Option<GripClass> {
        self.off_grip
    }

    /// Iterate over all equipped items.
    pub fn iter(&self) -> impl Iterator<Item = (EquipSlot, &EquippedItem)> {
        EquipSlot::ALL.into_iter().filter_map(|slot| {
            self.get(slot).map(|item| (slot, item))
        })
    }

    /// Returns true if any weapon is equipped.
    pub fn has_weapon(&self) -> bool {
        self.main.is_some() || self.ranged.is_some()
    }

    /// Clear all equipment.
    pub fn clear(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equip_mode_unarmed() {
        let equip = Equipment::new();
        assert_eq!(equip.equip_mode(), EquipMode::Unarmed);
    }

    #[test]
    fn equip_mode_one_hand() {
        let mut equip = Equipment::new();
        equip.equip(EquipSlot::Main, EquippedItem::new(ItemId(1)), Some(GripClass::OneHand));
        assert_eq!(equip.equip_mode(), EquipMode::OneHand);
    }

    #[test]
    fn equip_mode_dual_wield() {
        let mut equip = Equipment::new();
        equip.equip(EquipSlot::Main, EquippedItem::new(ItemId(1)), Some(GripClass::OneHand));
        equip.equip(EquipSlot::Off, EquippedItem::new(ItemId(2)), Some(GripClass::OneHand));
        assert_eq!(equip.equip_mode(), EquipMode::DualWield);
    }

    #[test]
    fn equip_mode_great_weapon() {
        let mut equip = Equipment::new();
        equip.equip(EquipSlot::Main, EquippedItem::new(ItemId(1)), Some(GripClass::TwoHandGreat));
        assert_eq!(equip.equip_mode(), EquipMode::GreatWeapon);
        assert!(equip.equip_mode().is_two_handed());
    }

    #[test]
    fn equip_mode_polearm() {
        let mut equip = Equipment::new();
        equip.equip(EquipSlot::Main, EquippedItem::new(ItemId(1)), Some(GripClass::TwoHandPole));
        assert_eq!(equip.equip_mode(), EquipMode::PoleArm);
    }

    #[test]
    fn equip_mode_shield() {
        let mut equip = Equipment::new();
        equip.equip(EquipSlot::Main, EquippedItem::new(ItemId(1)), Some(GripClass::OneHand));
        equip.equip(EquipSlot::Off, EquippedItem::new(ItemId(2)), Some(GripClass::Shield));
        assert_eq!(equip.equip_mode(), EquipMode::OneHandShield);
        assert!(equip.equip_mode().has_shield());
    }

    #[test]
    fn equip_mode_ranged() {
        let mut equip = Equipment::new();
        equip.equip(EquipSlot::Ranged, EquippedItem::new(ItemId(1)), Some(GripClass::Bow));
        assert_eq!(equip.equip_mode(), EquipMode::Ranged);
    }

    #[test]
    fn two_hand_conflict() {
        let mut equip = Equipment::new();
        equip.equip(EquipSlot::Main, EquippedItem::new(ItemId(1)), Some(GripClass::TwoHandGreat));

        let ctx = EquipContext {
            slot: Some(EquipSlot::Off),
            grip: Some(GripClass::Shield),
            job_restrict: JobMask::ALL,
        };
        let result = equip.can_equip(EquipSlot::Off, &ctx, JobMask::KNIGHT);
        assert!(matches!(result, Err(EquipError::TwoHandConflict)));
    }

    #[test]
    fn job_restriction() {
        let equip = Equipment::new();
        let ctx = EquipContext {
            slot: Some(EquipSlot::Main),
            grip: Some(GripClass::OneHand),
            job_restrict: JobMask::KNIGHT,
        };
        let result = equip.can_equip(EquipSlot::Main, &ctx, JobMask::ROGUE);
        assert!(matches!(result, Err(EquipError::JobRestricted { .. })));
    }

    #[test]
    fn ring_in_ring2() {
        let equip = Equipment::new();
        let ctx = EquipContext {
            slot: Some(EquipSlot::Ring),
            grip: None,
            job_restrict: JobMask::ALL,
        };
        let result = equip.can_equip(EquipSlot::Ring2, &ctx, JobMask::KNIGHT);
        assert!(result.is_ok());
    }

    #[test]
    fn unequip_returns_item() {
        let mut equip = Equipment::new();
        equip.equip(EquipSlot::Main, EquippedItem::new(ItemId(42)), Some(GripClass::OneHand));
        let old = equip.unequip(EquipSlot::Main);
        assert!(old.is_some());
        assert_eq!(old.unwrap().item, ItemId(42));
        assert!(equip.get(EquipSlot::Main).is_none());
    }

    #[test]
    fn iter_equipped() {
        let mut equip = Equipment::new();
        equip.equip(EquipSlot::Head, EquippedItem::new(ItemId(1)), None);
        equip.equip(EquipSlot::Main, EquippedItem::new(ItemId(2)), Some(GripClass::OneHand));

        let items: Vec<_> = equip.iter().collect();
        assert_eq!(items.len(), 2);
    }
}
