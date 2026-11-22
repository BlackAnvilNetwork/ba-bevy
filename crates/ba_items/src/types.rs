//! Core item types and enums.
//!
//! These types are the foundation of the item system. IDs and slugs are frozen
//! once assigned and must never be renumbered or renamed.

use serde::{Deserialize, Serialize};

/// Unique numeric identifier for an item definition.
///
/// Once assigned, an item's ID is frozen and must never be reused or changed.
/// External systems (gateway, persistence) depend on ID stability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ItemId(pub u32);

impl ItemId {
    pub const NONE: ItemId = ItemId(0);

    pub fn is_none(self) -> bool {
        self.0 == 0
    }
}

impl From<u32> for ItemId {
    fn from(id: u32) -> Self {
        ItemId(id)
    }
}

impl std::fmt::Display for ItemId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// What category of item this is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ItemKind {
    /// Equippable gear (weapons, armor, accessories).
    #[default]
    Equipment,
    /// Raw materials for crafting.
    Material,
    /// Intermediate crafting products.
    Intermediate,
    /// Items that are consumed on use (potions, food).
    Consumable,
    /// Ammunition (arrows, bolts).
    Ammo,
    /// Gathering and crafting tools.
    Tool,
    /// Sellable loot with no other use.
    Junk,
    /// Quest-related items.
    Quest,
    /// Non-equippable decorative items and scene props.
    Decor,
    /// Fishing bait (future).
    Bait,
}

/// Equipment slot where an item can be worn or held.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EquipSlot {
    Head,
    Body,
    Hands,
    Legs,
    Feet,
    Ring,
    Ring2,
    Earring,
    /// Main-hand weapon or tool.
    Main,
    /// Off-hand weapon or shield.
    Off,
    /// Ranged weapon (bow, crossbow).
    Ranged,
    /// Ammunition for ranged weapons.
    Ammo,
    /// Worn on back (quiver, backpack).
    Back,
    /// Tool slot for gathering/crafting.
    Tool,
}

impl EquipSlot {
    /// Returns true if this is a hand slot (main or off).
    pub fn is_hand_slot(self) -> bool {
        matches!(self, Self::Main | Self::Off)
    }

    /// Returns true if a ring can go in this slot.
    pub fn accepts_ring(self) -> bool {
        matches!(self, Self::Ring | Self::Ring2)
    }

    /// All slots in display order.
    pub const ALL: [EquipSlot; 14] = [
        Self::Head,
        Self::Body,
        Self::Hands,
        Self::Legs,
        Self::Feet,
        Self::Ring,
        Self::Ring2,
        Self::Earring,
        Self::Main,
        Self::Off,
        Self::Ranged,
        Self::Ammo,
        Self::Back,
        Self::Tool,
    ];
}

/// Weapon family determines which weapon skills are available.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WeaponFamily {
    Sword,
    Greatsword,
    Dagger,
    Axe,
    GreatAxe,
    Club,
    Hammer,
    Staff,
    Polearm,
    Knuckles,
    Bow,
    Crossbow,
}

impl WeaponFamily {
    /// Returns the grip class this family typically uses.
    pub fn default_grip(self) -> GripClass {
        match self {
            Self::Sword | Self::Dagger | Self::Axe | Self::Club | Self::Hammer | Self::Knuckles => {
                GripClass::OneHand
            }
            Self::Greatsword | Self::GreatAxe => GripClass::TwoHandGreat,
            Self::Staff | Self::Polearm => GripClass::TwoHandPole,
            Self::Bow | Self::Crossbow => GripClass::Bow,
        }
    }
}

/// How the weapon is gripped, determining animation and equip mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum GripClass {
    #[default]
    OneHand,
    /// Great swords, great axes - both hands on one weapon, power stance.
    TwoHandGreat,
    /// Staves, spears, polearms - both hands on a shaft.
    TwoHandPole,
    /// Bows and crossbows - special draw/aim animations.
    Bow,
    /// Shields - held in off-hand with blocking pose.
    Shield,
    /// Tools - held during gather/craft animations only.
    Tool,
}

impl GripClass {
    /// Returns true if this grip requires both hands.
    pub fn is_two_handed(self) -> bool {
        matches!(self, Self::TwoHandGreat | Self::TwoHandPole | Self::Bow)
    }

    /// Returns true if this grip can be used in the main hand.
    pub fn is_main_hand(self) -> bool {
        !matches!(self, Self::Shield)
    }

    /// Returns true if this grip can be used in the off hand.
    pub fn is_off_hand(self) -> bool {
        matches!(self, Self::OneHand | Self::Shield)
    }
}

/// Item rarity tier. Never inferred from names; always explicit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Rarity {
    #[default]
    Common,
    Uncommon,
    Rare,
    Epic,
    Legendary,
    Ancient,
    Primal,
}

impl Rarity {
    pub fn label(self) -> &'static str {
        match self {
            Self::Common => "Common",
            Self::Uncommon => "Uncommon",
            Self::Rare => "Rare",
            Self::Epic => "Epic",
            Self::Legendary => "Legendary",
            Self::Ancient => "Ancient",
            Self::Primal => "Primal",
        }
    }
}

/// Type of ammunition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AmmoClass {
    Arrow,
    Bolt,
}

/// Bitmask of jobs that can use an item.
///
/// Bits follow the playable `JobId` order: Time Mage, Knight, Rogue, Cleric,
/// Sorcerer, Fighter, Battle Mage. Druid is not a mask bit. `ALL` is the
/// default so a missing restriction lets every playable job equip the item.
/// `NONE` lets nobody.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct JobMask(pub u16);

impl Default for JobMask {
    fn default() -> Self {
        Self::ALL
    }
}

impl JobMask {
    /// Bits 0 through 6. Bit 7 is unused because Druid is not playable.
    pub const ALL: JobMask = JobMask((1 << 7) - 1);
    pub const NONE: JobMask = JobMask(0);

    pub const TIME_MAGE: JobMask = JobMask(1 << 0);
    pub const KNIGHT: JobMask = JobMask(1 << 1);
    pub const ROGUE: JobMask = JobMask(1 << 2);
    pub const CLERIC: JobMask = JobMask(1 << 3);
    pub const SORCERER: JobMask = JobMask(1 << 4);
    pub const FIGHTER: JobMask = JobMask(1 << 5);
    pub const BATTLE_MAGE: JobMask = JobMask(1 << 6);

    pub fn allows(self, job: JobMask) -> bool {
        self.0 != 0 && job.0 != 0 && (self.0 & job.0) != 0
    }

    pub fn union(self, other: JobMask) -> JobMask {
        JobMask(self.0 | other.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn item_id_none_is_zero() {
        assert!(ItemId::NONE.is_none());
        assert!(!ItemId(1).is_none());
    }

    #[test]
    fn grip_class_two_handed_checks() {
        assert!(!GripClass::OneHand.is_two_handed());
        assert!(GripClass::TwoHandGreat.is_two_handed());
        assert!(GripClass::TwoHandPole.is_two_handed());
        assert!(GripClass::Bow.is_two_handed());
        assert!(!GripClass::Shield.is_two_handed());
    }

    #[test]
    fn weapon_family_default_grips() {
        assert_eq!(WeaponFamily::Sword.default_grip(), GripClass::OneHand);
        assert_eq!(WeaponFamily::Greatsword.default_grip(), GripClass::TwoHandGreat);
        assert_eq!(WeaponFamily::Staff.default_grip(), GripClass::TwoHandPole);
        assert_eq!(WeaponFamily::Bow.default_grip(), GripClass::Bow);
    }

    #[test]
    fn job_mask_allows() {
        assert!(JobMask::ALL.allows(JobMask::KNIGHT));
        assert!(JobMask::ALL.allows(JobMask::TIME_MAGE));
        assert!(!JobMask::NONE.allows(JobMask::KNIGHT));
        assert!(!JobMask::NONE.allows(JobMask::ALL));
        assert_eq!(JobMask::default(), JobMask::ALL);
        assert_eq!(JobMask::TIME_MAGE.0, 1 << 0);
        assert_eq!(JobMask::KNIGHT.0, 1 << 1);
        assert_eq!(JobMask::BATTLE_MAGE.0, 1 << 6);
        assert_eq!(JobMask::ALL.0, (1 << 7) - 1);
        assert!(!JobMask::ALL.allows(JobMask(1 << 7)));
        assert!(JobMask::KNIGHT.allows(JobMask::KNIGHT));
        assert!(!JobMask::KNIGHT.allows(JobMask::ROGUE));
        let knight_rogue = JobMask::KNIGHT.union(JobMask::ROGUE);
        assert!(knight_rogue.allows(JobMask::KNIGHT));
        assert!(knight_rogue.allows(JobMask::ROGUE));
        assert!(!knight_rogue.allows(JobMask::CLERIC));
    }

    #[test]
    fn equip_slot_ring_accepts() {
        assert!(EquipSlot::Ring.accepts_ring());
        assert!(EquipSlot::Ring2.accepts_ring());
        assert!(!EquipSlot::Main.accepts_ring());
    }

    #[test]
    fn rarity_ordering() {
        assert!(Rarity::Common < Rarity::Uncommon);
        assert!(Rarity::Legendary < Rarity::Primal);
    }
}
