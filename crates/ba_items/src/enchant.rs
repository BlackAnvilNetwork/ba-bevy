//! Enchantment system: glyphs apply prefix + suffix to gear.

use serde::{Deserialize, Serialize};

use crate::element::Element;

/// A stat modifier from an enchantment (prefix or suffix).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnchantStat {
    Attack,
    FireDamage,
    IceDamage,
    WindDamage,
    WaterDamage,
    EarthDamage,
    LightningDamage,
    HolyDamage,
    DarkDamage,
    AreaDamage,
    BurnChance,
    BurnDamage,
    CritChance,
    CritDamage,
    ExecuteDamage,
    FirePenetration,
    Defense,
    MaxHp,
    SlowChance,
    SlowPotency,
    CritVsChilled,
    CooldownReduction,
    IceResistance,
    AttackSpeed,
    Evasion,
    DodgeChance,
    MovementSpeed,
    ProjectileSpeed,
    MultiHitDamage,
    AreaSize,
    StaminaRegen,
    Armor,
    DamageReduction,
    BlockChance,
    StaggerDamage,
    Regen,
    Thorns,
    Knockback,
    ChainChance,
    StunChance,
    EnergyRegen,
    ParalysisChance,
    Healing,
    Lifesteal,
    ResourceRegen,
    DebuffDuration,
    CleanseChance,
    BuffDuration,
    ShieldStrength,
    TeamAura,
    CleansePotency,
    ElementalResistance,
    BackstabDamage,
    DebuffPotency,
    DotDamage,
    DotPotency,
    ResourceDrain,
}

impl EnchantStat {
    pub fn label(self) -> &'static str {
        match self {
            Self::Attack => "Attack",
            Self::FireDamage => "Fire damage",
            Self::IceDamage => "Ice damage",
            Self::WindDamage => "Wind damage",
            Self::WaterDamage => "Water damage",
            Self::EarthDamage => "Earth damage",
            Self::LightningDamage => "Lightning damage",
            Self::HolyDamage => "Holy damage",
            Self::DarkDamage => "Dark damage",
            Self::AreaDamage => "Area damage",
            Self::BurnChance => "Burn chance",
            Self::BurnDamage => "Burn damage",
            Self::CritChance => "Crit chance",
            Self::CritDamage => "Crit damage",
            Self::ExecuteDamage => "Execute damage",
            Self::FirePenetration => "Fire penetration",
            Self::Defense => "Defense",
            Self::MaxHp => "Max HP",
            Self::SlowChance => "Slow chance",
            Self::SlowPotency => "Slow potency",
            Self::CritVsChilled => "Crit vs chilled",
            Self::CooldownReduction => "Cooldown reduction",
            Self::IceResistance => "Ice resistance",
            Self::AttackSpeed => "Attack speed",
            Self::Evasion => "Evasion",
            Self::DodgeChance => "Dodge chance",
            Self::MovementSpeed => "Movement speed",
            Self::ProjectileSpeed => "Projectile speed",
            Self::MultiHitDamage => "Multi-hit damage",
            Self::AreaSize => "Area size",
            Self::StaminaRegen => "Stamina regen",
            Self::Armor => "Armor",
            Self::DamageReduction => "Damage reduction",
            Self::BlockChance => "Block chance",
            Self::StaggerDamage => "Stagger damage",
            Self::Regen => "Regen",
            Self::Thorns => "Thorns",
            Self::Knockback => "Knockback",
            Self::ChainChance => "Chain chance",
            Self::StunChance => "Stun chance",
            Self::EnergyRegen => "Energy regen",
            Self::ParalysisChance => "Paralysis chance",
            Self::Healing => "Healing",
            Self::Lifesteal => "Lifesteal",
            Self::ResourceRegen => "Resource regen",
            Self::DebuffDuration => "Debuff duration",
            Self::CleanseChance => "Cleanse chance",
            Self::BuffDuration => "Buff duration",
            Self::ShieldStrength => "Shield strength",
            Self::TeamAura => "Team stat aura",
            Self::CleansePotency => "Cleanse potency",
            Self::ElementalResistance => "Elemental resistance",
            Self::BackstabDamage => "Backstab damage",
            Self::DebuffPotency => "Debuff potency",
            Self::DotDamage => "DoT damage",
            Self::DotPotency => "DoT potency",
            Self::ResourceDrain => "Resource drain",
        }
    }
}

/// An enchantment affix (prefix or suffix).
#[derive(Debug, Clone, PartialEq)]
pub struct EnchantAffix {
    pub name: &'static str,
    pub stat: EnchantStat,
    pub min_percent: u8,
    pub max_percent: u8,
}

impl EnchantAffix {
    pub const fn new(name: &'static str, stat: EnchantStat, min: u8, max: u8) -> Self {
        Self {
            name,
            stat,
            min_percent: min,
            max_percent: max,
        }
    }

    pub fn roll(&self, rng: &mut impl rand::Rng) -> RolledAffix {
        let value = rng.gen_range(self.min_percent..=self.max_percent);
        RolledAffix {
            name: self.name.to_string(),
            stat: self.stat,
            percent: value,
        }
    }
}

/// A rolled affix with a specific percentage value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RolledAffix {
    pub name: String,
    pub stat: EnchantStat,
    pub percent: u8,
}

impl RolledAffix {
    pub fn display(&self) -> String {
        format!("{} +{}%", self.stat.label(), self.percent)
    }
}

/// A complete enchantment on gear (prefix + suffix).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Enchantment {
    pub element: Element,
    pub prefix: RolledAffix,
    pub suffix: RolledAffix,
}

impl Enchantment {
    pub fn display_name(&self, base_name: &str) -> String {
        if self.suffix.name.starts_with("of ") {
            format!("{} {} {}", self.prefix.name, base_name, self.suffix.name)
        } else {
            format!("{} {} of {}", self.prefix.name, base_name, self.suffix.name)
        }
    }
}

/// Enchantment pools for each element.
pub struct EnchantPool {
    pub element: Element,
    pub prefixes: &'static [EnchantAffix],
    pub suffixes: &'static [EnchantAffix],
}

impl EnchantPool {
    pub fn roll(&self, rng: &mut impl rand::Rng) -> Enchantment {
        let prefix_idx = rng.gen_range(0..self.prefixes.len());
        let suffix_idx = rng.gen_range(0..self.suffixes.len());
        Enchantment {
            element: self.element,
            prefix: self.prefixes[prefix_idx].roll(rng),
            suffix: self.suffixes[suffix_idx].roll(rng),
        }
    }
}

pub static FIRE_PREFIXES: [EnchantAffix; 4] = [
    EnchantAffix::new("Ember", EnchantStat::Attack, 3, 7),
    EnchantAffix::new("Blazing", EnchantStat::FireDamage, 4, 9),
    EnchantAffix::new("Inferno", EnchantStat::AreaDamage, 5, 10),
    EnchantAffix::new("Scorching", EnchantStat::BurnChance, 3, 6),
];

pub static FIRE_SUFFIXES: [EnchantAffix; 4] = [
    EnchantAffix::new("of Cinders", EnchantStat::BurnDamage, 5, 12),
    EnchantAffix::new("of Ash", EnchantStat::CritChance, 2, 5),
    EnchantAffix::new("of Flame", EnchantStat::ExecuteDamage, 6, 14),
    EnchantAffix::new("of Heat", EnchantStat::FirePenetration, 4, 8),
];

pub static ICE_PREFIXES: [EnchantAffix; 4] = [
    EnchantAffix::new("Frost", EnchantStat::IceDamage, 4, 9),
    EnchantAffix::new("Glacial", EnchantStat::Defense, 3, 7),
    EnchantAffix::new("Permafrost", EnchantStat::MaxHp, 4, 8),
    EnchantAffix::new("Chilling", EnchantStat::SlowChance, 3, 7),
];

pub static ICE_SUFFIXES: [EnchantAffix; 4] = [
    EnchantAffix::new("of Chill", EnchantStat::SlowPotency, 5, 12),
    EnchantAffix::new("of Hail", EnchantStat::CritVsChilled, 4, 10),
    EnchantAffix::new("of Rime", EnchantStat::CooldownReduction, 2, 6),
    EnchantAffix::new("of Snow", EnchantStat::IceResistance, 4, 9),
];

pub static WIND_PREFIXES: [EnchantAffix; 4] = [
    EnchantAffix::new("Gale", EnchantStat::AttackSpeed, 3, 7),
    EnchantAffix::new("Zephyr", EnchantStat::Evasion, 4, 9),
    EnchantAffix::new("Tempest", EnchantStat::MultiHitDamage, 4, 8),
    EnchantAffix::new("Whirling", EnchantStat::DodgeChance, 2, 5),
];

pub static WIND_SUFFIXES: [EnchantAffix; 4] = [
    EnchantAffix::new("of Squalls", EnchantStat::MovementSpeed, 2, 6),
    EnchantAffix::new("of Currents", EnchantStat::ProjectileSpeed, 5, 10),
    EnchantAffix::new("of Cyclones", EnchantStat::AreaSize, 5, 12),
    EnchantAffix::new("of Breezes", EnchantStat::StaminaRegen, 3, 7),
];

pub static EARTH_PREFIXES: [EnchantAffix; 4] = [
    EnchantAffix::new("Stone", EnchantStat::Armor, 4, 8),
    EnchantAffix::new("Granite", EnchantStat::DamageReduction, 3, 6),
    EnchantAffix::new("Bedrock", EnchantStat::BlockChance, 3, 7),
    EnchantAffix::new("Earthen", EnchantStat::MaxHp, 4, 8),
];

pub static EARTH_SUFFIXES: [EnchantAffix; 4] = [
    EnchantAffix::new("of Quakes", EnchantStat::StaggerDamage, 5, 12),
    EnchantAffix::new("of Moss", EnchantStat::Regen, 4, 10),
    EnchantAffix::new("of Bulwarks", EnchantStat::Thorns, 5, 11),
    EnchantAffix::new("of Tremors", EnchantStat::Knockback, 4, 9),
];

pub static LIGHTNING_PREFIXES: [EnchantAffix; 4] = [
    EnchantAffix::new("Spark", EnchantStat::LightningDamage, 4, 9),
    EnchantAffix::new("Thunder", EnchantStat::CritDamage, 5, 11),
    EnchantAffix::new("Storm", EnchantStat::AttackSpeed, 3, 7),
    EnchantAffix::new("Charged", EnchantStat::EnergyRegen, 3, 6),
];

pub static LIGHTNING_SUFFIXES: [EnchantAffix; 4] = [
    EnchantAffix::new("of Arcs", EnchantStat::ChainChance, 3, 7),
    EnchantAffix::new("of Storms", EnchantStat::StunChance, 2, 6),
    EnchantAffix::new("of Voltage", EnchantStat::EnergyRegen, 4, 10),
    EnchantAffix::new("of Bolts", EnchantStat::ParalysisChance, 2, 5),
];

pub static WATER_PREFIXES: [EnchantAffix; 4] = [
    EnchantAffix::new("Tide", EnchantStat::Healing, 4, 10),
    EnchantAffix::new("Wave", EnchantStat::WaterDamage, 4, 9),
    EnchantAffix::new("Abyss", EnchantStat::Lifesteal, 3, 7),
    EnchantAffix::new("Flowing", EnchantStat::ResourceRegen, 4, 9),
];

pub static WATER_SUFFIXES: [EnchantAffix; 4] = [
    EnchantAffix::new("of Mists", EnchantStat::Evasion, 3, 7),
    EnchantAffix::new("of Rain", EnchantStat::ResourceRegen, 4, 9),
    EnchantAffix::new("of Tides", EnchantStat::DebuffDuration, 5, 12),
    EnchantAffix::new("of Rivers", EnchantStat::CleanseChance, 3, 6),
];

pub static LIGHT_PREFIXES: [EnchantAffix; 4] = [
    EnchantAffix::new("Dawn", EnchantStat::Healing, 4, 9),
    EnchantAffix::new("Radiant", EnchantStat::HolyDamage, 4, 10),
    EnchantAffix::new("Prism", EnchantStat::ElementalResistance, 4, 8),
    EnchantAffix::new("Hallowed", EnchantStat::ShieldStrength, 3, 7),
];

pub static LIGHT_SUFFIXES: [EnchantAffix; 4] = [
    EnchantAffix::new("of Grace", EnchantStat::BuffDuration, 5, 10),
    EnchantAffix::new("of Sanctuary", EnchantStat::ShieldStrength, 5, 12),
    EnchantAffix::new("of Glory", EnchantStat::TeamAura, 2, 5),
    EnchantAffix::new("of Purity", EnchantStat::CleansePotency, 4, 9),
];

pub static DARK_PREFIXES: [EnchantAffix; 4] = [
    EnchantAffix::new("Shadow", EnchantStat::DarkDamage, 4, 9),
    EnchantAffix::new("Void", EnchantStat::DebuffPotency, 5, 11),
    EnchantAffix::new("Blood", EnchantStat::Lifesteal, 3, 7),
    EnchantAffix::new("Cursed", EnchantStat::DotPotency, 4, 9),
];

pub static DARK_SUFFIXES: [EnchantAffix; 4] = [
    EnchantAffix::new("of Night", EnchantStat::BackstabDamage, 5, 12),
    EnchantAffix::new("of Curses", EnchantStat::DotDamage, 5, 12),
    EnchantAffix::new("of Draining", EnchantStat::ResourceDrain, 4, 10),
    EnchantAffix::new("of Gloom", EnchantStat::DebuffDuration, 5, 10),
];

pub fn get_pool(element: Element) -> EnchantPool {
    match element {
        Element::Fire => EnchantPool {
            element,
            prefixes: &FIRE_PREFIXES,
            suffixes: &FIRE_SUFFIXES,
        },
        Element::Ice => EnchantPool {
            element,
            prefixes: &ICE_PREFIXES,
            suffixes: &ICE_SUFFIXES,
        },
        Element::Wind => EnchantPool {
            element,
            prefixes: &WIND_PREFIXES,
            suffixes: &WIND_SUFFIXES,
        },
        Element::Water => EnchantPool {
            element,
            prefixes: &WATER_PREFIXES,
            suffixes: &WATER_SUFFIXES,
        },
        Element::Earth => EnchantPool {
            element,
            prefixes: &EARTH_PREFIXES,
            suffixes: &EARTH_SUFFIXES,
        },
        Element::Lightning => EnchantPool {
            element,
            prefixes: &LIGHTNING_PREFIXES,
            suffixes: &LIGHTNING_SUFFIXES,
        },
        Element::Light => EnchantPool {
            element,
            prefixes: &LIGHT_PREFIXES,
            suffixes: &LIGHT_SUFFIXES,
        },
        Element::Dark => EnchantPool {
            element,
            prefixes: &DARK_PREFIXES,
            suffixes: &DARK_SUFFIXES,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    #[test]
    fn enchant_pool_roll() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let pool = get_pool(Element::Fire);
        let enchant = pool.roll(&mut rng);

        assert_eq!(enchant.element, Element::Fire);
        assert!(enchant.prefix.percent >= 3 && enchant.prefix.percent <= 10);
        assert!(enchant.suffix.percent >= 2 && enchant.suffix.percent <= 14);
    }

    #[test]
    fn enchant_display_name() {
        let enchant = Enchantment {
            element: Element::Fire,
            prefix: RolledAffix {
                name: "Blazing".to_string(),
                stat: EnchantStat::FireDamage,
                percent: 7,
            },
            suffix: RolledAffix {
                name: "of Cinders".to_string(),
                stat: EnchantStat::BurnDamage,
                percent: 10,
            },
        };
        assert_eq!(enchant.display_name("Iron Sword"), "Blazing Iron Sword of Cinders");
    }

    #[test]
    fn all_elements_have_pools() {
        for element in Element::ALL {
            let pool = get_pool(element);
            assert_eq!(pool.prefixes.len(), 4);
            assert_eq!(pool.suffixes.len(), 4);
        }
    }
}
