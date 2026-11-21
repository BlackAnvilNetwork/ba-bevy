//! Equipped-mode state machine.
//!
//! Inventory will drive this later. For now the local player can cycle
//! Unarmed → OneHand → DualWield → GreatWeapon → PoleArm (offline `-` / `=`).
//! One-hand + shield is a reserved off-hand, not a cycle step yet.
//!
//! Two-handed splits into:
//! - Great weapons: great swords / great axes
//! - Pole arms: spears / staffs

use super::HeldWeapon;
use crate::player::NoteComposer;
use crate::rpg::Fighter;
use bevy::prelude::*;

/// How the character is armed. Governs idle pose, attack clips, and which
/// hands parent a weapon mesh.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum EquipMode {
    #[default]
    Unarmed,
    OneHand,
    DualWield,
    /// Great swords and great axes. Both hands on one weapon.
    GreatWeapon,
    /// Spears and staffs. Both hands on one shaft.
    PoleArm,
}

/// What occupies the off hand. `Shield` is reserved for a later 1H+shield mode.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum OffhandSlot {
    #[default]
    Empty,
    Weapon,
    #[allow(dead_code)]
    Shield,
}

impl EquipMode {
    pub const CYCLE: [Self; 5] = [
        Self::Unarmed,
        Self::OneHand,
        Self::DualWield,
        Self::GreatWeapon,
        Self::PoleArm,
    ];

    pub fn from_held(held: HeldWeapon) -> Self {
        match held {
            HeldWeapon::Unarmed => Self::Unarmed,
            HeldWeapon::OneHand => Self::OneHand,
            HeldWeapon::DualWield => Self::DualWield,
            HeldWeapon::GreatWeapon => Self::GreatWeapon,
            HeldWeapon::PoleArm => Self::PoleArm,
        }
    }

    pub fn held(self) -> HeldWeapon {
        match self {
            Self::Unarmed => HeldWeapon::Unarmed,
            Self::OneHand => HeldWeapon::OneHand,
            Self::DualWield => HeldWeapon::DualWield,
            Self::GreatWeapon => HeldWeapon::GreatWeapon,
            Self::PoleArm => HeldWeapon::PoleArm,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Unarmed => "Hand to hand",
            Self::OneHand => "One-handed",
            Self::DualWield => "Dual wield",
            Self::GreatWeapon => "Great weapon",
            Self::PoleArm => "Pole arm",
        }
    }

    pub fn offhand(self) -> OffhandSlot {
        match self {
            Self::DualWield => OffhandSlot::Weapon,
            _ => OffhandSlot::Empty,
        }
    }

    /// Right hand holds steel (or the shared 2H grip).
    pub fn right_weapon(self) -> bool {
        !matches!(self, Self::Unarmed)
    }

    /// Left hand holds its own steel (dual). Shield will use this slot later.
    pub fn left_weapon(self) -> bool {
        matches!(self, Self::DualWield)
    }

    /// Great weapons and pole arms both use a shared two-hand grip.
    pub fn two_handed(self) -> bool {
        matches!(self, Self::GreatWeapon | Self::PoleArm)
    }

    /// 1H is the only mode that can take a shield off-hand later.
    pub fn allows_shield(self) -> bool {
        matches!(self, Self::OneHand)
    }

    pub fn stepped(self, dir: isize) -> Self {
        let i = Self::CYCLE
            .iter()
            .position(|m| *m == self)
            .unwrap_or(0) as isize;
        let next = (i + dir).rem_euclid(Self::CYCLE.len() as isize) as usize;
        Self::CYCLE[next]
    }
}

/// Live equipped mode. Draft loadout is the default; cycling overrides it.
#[derive(Resource, Debug, Clone)]
pub struct EquipState {
    pub mode: EquipMode,
    pub from_loadout: EquipMode,
}

impl Default for EquipState {
    fn default() -> Self {
        Self {
            mode: EquipMode::Unarmed,
            from_loadout: EquipMode::Unarmed,
        }
    }
}

impl EquipState {
    pub fn from_loadout(held: HeldWeapon) -> Self {
        let mode = EquipMode::from_held(held);
        Self {
            mode,
            from_loadout: mode,
        }
    }

    pub fn held(&self) -> HeldWeapon {
        self.mode.held()
    }
}

/// Offline playtest: Minus / Equal steps the armed mode.
pub fn cycle_equip_mode(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    note: Res<NoteComposer>,
    mut equip: ResMut<EquipState>,
    mut fighters: Query<&mut Fighter, With<crate::player::Player>>,
) {
    if note.active {
        return;
    }
    let dir = if keys.just_pressed(KeyCode::Equal) {
        1isize
    } else if keys.just_pressed(KeyCode::Minus) {
        -1isize
    } else {
        return;
    };
    equip.mode = equip.mode.stepped(dir);
    info!("Equip mode {}", equip.mode.label());
    if let Ok(mut fighter) = fighters.single_mut() {
        fighter.note_cast(&format!("Equip: {}", equip.mode.label()), time.elapsed_secs());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cycle_visits_every_armed_mode() {
        let mut mode = EquipMode::Unarmed;
        assert_eq!(mode.offhand(), OffhandSlot::Empty);
        assert!(!mode.right_weapon());
        mode = mode.stepped(1);
        assert_eq!(mode, EquipMode::OneHand);
        assert!(mode.right_weapon() && !mode.left_weapon());
        assert_eq!(mode.offhand(), OffhandSlot::Empty);
        mode = mode.stepped(1);
        assert_eq!(mode, EquipMode::DualWield);
        assert!(mode.left_weapon());
        assert_eq!(mode.offhand(), OffhandSlot::Weapon);
        mode = mode.stepped(1);
        assert_eq!(mode, EquipMode::GreatWeapon);
        assert!(mode.two_handed() && mode.right_weapon() && !mode.left_weapon());
        mode = mode.stepped(1);
        assert_eq!(mode, EquipMode::PoleArm);
        assert!(mode.two_handed());
        assert_eq!(mode.label(), "Pole arm");
        mode = mode.stepped(1);
        assert_eq!(mode, EquipMode::Unarmed);
        mode = mode.stepped(-1);
        assert_eq!(mode, EquipMode::PoleArm);
    }

    #[test]
    fn one_hand_can_take_a_shield_later() {
        assert!(EquipMode::OneHand.allows_shield());
        assert!(!EquipMode::DualWield.allows_shield());
        assert!(!EquipMode::GreatWeapon.allows_shield());
        assert!(!EquipMode::PoleArm.allows_shield());
        assert_eq!(EquipMode::OneHand.offhand(), OffhandSlot::Empty);
    }

    #[test]
    fn loadout_roundtrips_held_weapon() {
        let state = EquipState::from_loadout(HeldWeapon::DualWield);
        assert_eq!(state.mode, EquipMode::DualWield);
        assert_eq!(state.held(), HeldWeapon::DualWield);
        assert_eq!(state.from_loadout, EquipMode::DualWield);
        let great = EquipState::from_loadout(HeldWeapon::GreatWeapon);
        assert_eq!(great.mode, EquipMode::GreatWeapon);
        let pole = EquipState::from_loadout(HeldWeapon::PoleArm);
        assert_eq!(pole.mode, EquipMode::PoleArm);
    }
}
