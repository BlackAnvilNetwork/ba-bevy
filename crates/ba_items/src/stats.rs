//! Item statistics.

use serde::{Deserialize, Serialize};

/// Combat and character statistics an item provides.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ItemStats {
    pub str_: i16,
    pub dex: i16,
    pub vit: i16,
    pub agi: i16,
    pub int: i16,
    pub mnd: i16,
    pub atk: i16,
    pub def: i16,
    pub acc: i16,
    pub eva: i16,
    pub mab: i16,
    pub mdb: i16,
    pub hp: i16,
    pub mp: i16,
    pub regen: i16,
    pub heal_potency: i16,
}

impl ItemStats {
    pub const ZERO: ItemStats = ItemStats {
        str_: 0,
        dex: 0,
        vit: 0,
        agi: 0,
        int: 0,
        mnd: 0,
        atk: 0,
        def: 0,
        acc: 0,
        eva: 0,
        mab: 0,
        mdb: 0,
        hp: 0,
        mp: 0,
        regen: 0,
        heal_potency: 0,
    };

    /// Combine stats additively.
    pub fn add(self, other: ItemStats) -> ItemStats {
        ItemStats {
            str_: self.str_.saturating_add(other.str_),
            dex: self.dex.saturating_add(other.dex),
            vit: self.vit.saturating_add(other.vit),
            agi: self.agi.saturating_add(other.agi),
            int: self.int.saturating_add(other.int),
            mnd: self.mnd.saturating_add(other.mnd),
            atk: self.atk.saturating_add(other.atk),
            def: self.def.saturating_add(other.def),
            acc: self.acc.saturating_add(other.acc),
            eva: self.eva.saturating_add(other.eva),
            mab: self.mab.saturating_add(other.mab),
            mdb: self.mdb.saturating_add(other.mdb),
            hp: self.hp.saturating_add(other.hp),
            mp: self.mp.saturating_add(other.mp),
            regen: self.regen.saturating_add(other.regen),
            heal_potency: self.heal_potency.saturating_add(other.heal_potency),
        }
    }

    /// Scale stats by a multiplier (for HQ items: 1.25).
    pub fn scale(self, mul: f32) -> ItemStats {
        let s = |v: i16| (v as f32 * mul).ceil() as i16;
        ItemStats {
            str_: s(self.str_),
            dex: s(self.dex),
            vit: s(self.vit),
            agi: s(self.agi),
            int: s(self.int),
            mnd: s(self.mnd),
            atk: s(self.atk),
            def: s(self.def),
            acc: s(self.acc),
            eva: s(self.eva),
            mab: s(self.mab),
            mdb: s(self.mdb),
            hp: s(self.hp),
            mp: s(self.mp),
            regen: s(self.regen),
            heal_potency: s(self.heal_potency),
        }
    }

    /// Scale gear stat by level ratio: effective = floor(stat × playerLevel / itemLevel).
    pub fn level_scaled(self, player_level: u32, item_level: u32) -> ItemStats {
        if item_level == 0 || player_level >= item_level {
            return self;
        }
        let ratio = player_level as f32 / item_level as f32;
        let s = |v: i16| (v as f32 * ratio).floor() as i16;
        ItemStats {
            str_: s(self.str_),
            dex: s(self.dex),
            vit: s(self.vit),
            agi: s(self.agi),
            int: s(self.int),
            mnd: s(self.mnd),
            atk: s(self.atk),
            def: s(self.def),
            acc: s(self.acc),
            eva: s(self.eva),
            mab: s(self.mab),
            mdb: s(self.mdb),
            hp: s(self.hp),
            mp: s(self.mp),
            regen: s(self.regen),
            heal_potency: s(self.heal_potency),
        }
    }

    /// Returns true if all stats are zero.
    pub fn is_zero(self) -> bool {
        self == Self::ZERO
    }
}

/// HQ stats are NQ × 1.25, rounded up.
pub fn hq_stats(nq: ItemStats) -> ItemStats {
    nq.scale(1.25)
}

/// HQ craft chance: 2% base + 1% per skill level over recipe, capped at 25%.
pub fn hq_chance(skill_level: u32, recipe_level: u32) -> f32 {
    let base = 0.02;
    let bonus = skill_level.saturating_sub(recipe_level) as f32 * 0.01;
    (base + bonus).min(0.25)
}

/// Gold cost to craft at a given level: 50 + 20 × level.
pub fn craft_gold_cost(level: u32) -> u32 {
    50 + 20 * level
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stats_add() {
        let a = ItemStats { atk: 10, def: 5, ..Default::default() };
        let b = ItemStats { atk: 3, hp: 20, ..Default::default() };
        let sum = a.add(b);
        assert_eq!(sum.atk, 13);
        assert_eq!(sum.def, 5);
        assert_eq!(sum.hp, 20);
    }

    #[test]
    fn hq_stats_ceil() {
        let nq = ItemStats { atk: 10, ..Default::default() };
        let hq = hq_stats(nq);
        assert_eq!(hq.atk, 13);
    }

    #[test]
    fn hq_chance_capped() {
        assert!((hq_chance(1, 1) - 0.02).abs() < 0.001);
        assert!((hq_chance(10, 5) - 0.07).abs() < 0.001);
        assert!((hq_chance(50, 10) - 0.25).abs() < 0.001);
    }

    #[test]
    fn craft_gold_cost_formula() {
        assert_eq!(craft_gold_cost(0), 50);
        assert_eq!(craft_gold_cost(1), 70);
        assert_eq!(craft_gold_cost(10), 250);
    }

    #[test]
    fn level_scaled_stats() {
        let base = ItemStats { atk: 100, def: 50, ..Default::default() };
        let scaled = base.level_scaled(5, 10);
        assert_eq!(scaled.atk, 50);
        assert_eq!(scaled.def, 25);
        let not_scaled = base.level_scaled(10, 10);
        assert_eq!(not_scaled.atk, 100);
    }
}
