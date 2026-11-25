//! Enemy drop tables for materials, runes, and glyphs.

use serde::{Deserialize, Serialize};

use crate::element::Element;
use crate::ItemId;

/// Type of enemy for drop table lookups.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnemyType {
    Beast,
    Bird,
    Reptile,
    Insect,
    Plant,
    Skeleton,
    Undead,
    Elemental(Element),
    Construct,
    Demon,
    Dragon,
    Boss,
}

impl EnemyType {
    pub fn label(self) -> &'static str {
        match self {
            Self::Beast => "Beast",
            Self::Bird => "Bird",
            Self::Reptile => "Reptile",
            Self::Insect => "Insect",
            Self::Plant => "Plant",
            Self::Skeleton => "Skeleton",
            Self::Undead => "Undead",
            Self::Elemental(_) => "Elemental",
            Self::Construct => "Construct",
            Self::Demon => "Demon",
            Self::Dragon => "Dragon",
            Self::Boss => "Boss",
        }
    }

    pub fn rune_drop_rate(self) -> f32 {
        match self {
            Self::Elemental(_) => 0.40,
            Self::Skeleton | Self::Undead => 0.15,
            Self::Dragon => 0.15,
            Self::Demon => 0.10,
            Self::Beast | Self::Bird | Self::Reptile | Self::Insect | Self::Plant | Self::Construct => 0.08,
            Self::Boss => 1.0,
        }
    }

    pub fn glyph_drop_rate(self) -> f32 {
        match self {
            Self::Elemental(_) => 0.02,
            Self::Demon => 0.03,
            Self::Dragon => 0.02,
            Self::Skeleton | Self::Undead => 0.01,
            Self::Boss => 1.0,
            _ => 0.0,
        }
    }

    pub fn fixed_rune_element(self) -> Option<Element> {
        match self {
            Self::Elemental(e) => Some(e),
            Self::Skeleton | Self::Undead => Some(Element::Dark),
            _ => None,
        }
    }

    pub fn fixed_glyph_element(self) -> Option<Element> {
        match self {
            Self::Elemental(e) => Some(e),
            Self::Skeleton | Self::Undead => Some(Element::Dark),
            _ => None,
        }
    }
}

/// A single drop entry in a drop table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DropEntry {
    pub item: ItemId,
    pub min_qty: u32,
    pub max_qty: u32,
    pub chance: f32,
}

impl DropEntry {
    pub fn new(item: ItemId, min: u32, max: u32, chance: f32) -> Self {
        Self {
            item,
            min_qty: min,
            max_qty: max,
            chance,
        }
    }

    pub fn guaranteed(item: ItemId, min: u32, max: u32) -> Self {
        Self::new(item, min, max, 1.0)
    }

    pub fn roll_qty(&self, rng: &mut impl rand::Rng) -> u32 {
        if self.min_qty == self.max_qty {
            self.min_qty
        } else {
            rng.gen_range(self.min_qty..=self.max_qty)
        }
    }

    pub fn should_drop(&self, rng: &mut impl rand::Rng) -> bool {
        if self.chance >= 1.0 {
            true
        } else {
            rng.gen_bool(self.chance as f64)
        }
    }
}

/// A complete drop table for an enemy.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DropTable {
    pub enemy_type: Option<EnemyType>,
    pub tier: u8,
    pub gold_min: u32,
    pub gold_max: u32,
    pub common: Vec<DropEntry>,
    pub uncommon: Vec<DropEntry>,
    pub rare: Vec<DropEntry>,
}

impl DropTable {
    pub fn new(enemy_type: EnemyType, tier: u8) -> Self {
        let (gold_min, gold_max) = gold_range_for_tier(tier);
        Self {
            enemy_type: Some(enemy_type),
            tier,
            gold_min,
            gold_max,
            common: Vec::new(),
            uncommon: Vec::new(),
            rare: Vec::new(),
        }
    }

    pub fn with_gold(mut self, min: u32, max: u32) -> Self {
        self.gold_min = min;
        self.gold_max = max;
        self
    }

    pub fn with_common(mut self, entry: DropEntry) -> Self {
        self.common.push(entry);
        self
    }

    pub fn with_uncommon(mut self, entry: DropEntry) -> Self {
        self.uncommon.push(entry);
        self
    }

    pub fn with_rare(mut self, entry: DropEntry) -> Self {
        self.rare.push(entry);
        self
    }
}

/// Result of rolling drops from a table.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DropResult {
    pub gold: u32,
    pub items: Vec<(ItemId, u32)>,
    pub runes: Vec<Element>,
    pub glyphs: Vec<Element>,
}

impl DropResult {
    pub fn is_empty(&self) -> bool {
        self.gold == 0 && self.items.is_empty() && self.runes.is_empty() && self.glyphs.is_empty()
    }
}

/// Roll drops from a drop table.
pub fn roll_drops(table: &DropTable, rng: &mut impl rand::Rng) -> DropResult {
    let mut result = DropResult::default();

    result.gold = if table.gold_min == table.gold_max {
        table.gold_min
    } else {
        rng.gen_range(table.gold_min..=table.gold_max)
    };

    for entry in &table.common {
        if entry.should_drop(rng) {
            let qty = entry.roll_qty(rng);
            result.items.push((entry.item, qty));
        }
    }

    for entry in &table.uncommon {
        if entry.should_drop(rng) {
            let qty = entry.roll_qty(rng);
            result.items.push((entry.item, qty));
        }
    }

    for entry in &table.rare {
        if entry.should_drop(rng) {
            let qty = entry.roll_qty(rng);
            result.items.push((entry.item, qty));
        }
    }

    if let Some(enemy_type) = table.enemy_type {
        let rune_rate = enemy_type.rune_drop_rate();
        if rng.gen_bool(rune_rate as f64) {
            let element = enemy_type.fixed_rune_element().unwrap_or_else(|| {
                Element::ALL[rng.gen_range(0..8)]
            });
            result.runes.push(element);
        }

        if matches!(enemy_type, EnemyType::Boss) {
            let extra_runes = rng.gen_range(1..=3);
            for _ in 0..extra_runes {
                let element = Element::ALL[rng.gen_range(0..8)];
                result.runes.push(element);
            }
        }

        let glyph_rate = enemy_type.glyph_drop_rate();
        if glyph_rate > 0.0 && rng.gen_bool(glyph_rate as f64) {
            let element = enemy_type.fixed_glyph_element().unwrap_or_else(|| {
                Element::ALL[rng.gen_range(0..8)]
            });
            result.glyphs.push(element);
        }

        if matches!(enemy_type, EnemyType::Boss) && rng.gen_bool(0.25) {
            let element = Element::ALL[rng.gen_range(0..8)];
            result.glyphs.push(element);
        }
    }

    result
}

fn gold_range_for_tier(tier: u8) -> (u32, u32) {
    match tier {
        1 => (5, 15),
        2 => (15, 40),
        3 => (40, 100),
        4 => (100, 250),
        5 => (250, 500),
        _ => (5, 15),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    #[test]
    fn enemy_type_drop_rates() {
        assert!((EnemyType::Elemental(Element::Fire).rune_drop_rate() - 0.40).abs() < 0.001);
        assert!((EnemyType::Beast.rune_drop_rate() - 0.08).abs() < 0.001);
        assert!((EnemyType::Boss.rune_drop_rate() - 1.0).abs() < 0.001);

        assert!((EnemyType::Elemental(Element::Fire).glyph_drop_rate() - 0.02).abs() < 0.001);
        assert!((EnemyType::Demon.glyph_drop_rate() - 0.03).abs() < 0.001);
        assert!((EnemyType::Beast.glyph_drop_rate()).abs() < 0.001);
    }

    #[test]
    fn drop_entry_roll() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let entry = DropEntry::new(ItemId(1), 1, 3, 1.0);

        let qty = entry.roll_qty(&mut rng);
        assert!(qty >= 1 && qty <= 3);

        let guaranteed = DropEntry::guaranteed(ItemId(1), 2, 2);
        assert_eq!(guaranteed.roll_qty(&mut rng), 2);
    }

    #[test]
    fn roll_drops_basic() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let table = DropTable::new(EnemyType::Beast, 2)
            .with_common(DropEntry::guaranteed(ItemId(100), 1, 2));

        let result = roll_drops(&table, &mut rng);
        assert!(result.gold >= 15 && result.gold <= 40);
        assert!(!result.items.is_empty());
    }

    #[test]
    fn boss_always_drops_runes_and_glyph() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let table = DropTable::new(EnemyType::Boss, 5);

        let result = roll_drops(&table, &mut rng);
        assert!(!result.runes.is_empty());
        assert!(!result.glyphs.is_empty());
    }

    #[test]
    fn elemental_drops_matching_element() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let table = DropTable::new(EnemyType::Elemental(Element::Fire), 3);

        for _ in 0..10 {
            let result = roll_drops(&table, &mut rng);
            for rune in &result.runes {
                assert_eq!(*rune, Element::Fire);
            }
            for glyph in &result.glyphs {
                assert_eq!(*glyph, Element::Fire);
            }
        }
    }
}
