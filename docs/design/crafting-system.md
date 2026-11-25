# Expanded Crafting System

## Quick Reference

| Phase   | Formula                              | Result                                                         |
|---------|--------------------------------------|----------------------------------------------------------------|
| Refine  | Rune + 3 Materials + Gold            | Component (ingot, lumber, leather, cloth, bone, stone, essence)|
| Craft   | Rune + Components + Gold             | Base weapon or armor                                           |
| Enchant | Base gear + Elemental Glyph + Gold   | Enchanted gear: Prefix + Base Name + of Suffix                 |

**Core Rules:**
- Runes and Glyphs are enemy drops. Never crafted.
- Runes are common catalysts. Every refine and gear craft consumes one.
- Glyphs are rare catalysts. One per element. No tiers, no marks, no HQ versions.
- Enchanting rolls one prefix and one suffix from that element's pools. Each rolls a numeric value inside its listed range.
- Applying a new glyph overwrites the old enchantment.
- Each enemy type has a chance to drop only 1 element of items (glyphs/runes).
- **Crafting always succeeds.** No failure chance, no HQ variance. Simplicity over FFXI-style RNG.

---

## 1. Currency and Costs

Gold is earned from enemies, quests, and salvage. It is consumed in every refine, craft, enchant, and salvage action.

| Action             | Gold Cost             |
|--------------------|-----------------------|
| Refine (Tier 1–2)  | 10–50                 |
| Refine (Tier 3–4)  | 100–500               |
| Refine (Tier 5)    | 1,000–2,500           |
| Combo Refine       | 2× base tier cost     |
| Craft (Tier 1–2)   | 25–100                |
| Craft (Tier 3–4)   | 250–1,000             |
| Craft (Tier 5)     | 2,500–5,000           |
| Enchant (any tier) | 500 + 100 × gear tier |
| Salvage            | 50 flat               |
| Repair (per 25%)   | 10 × gear tier        |

Gold costs scale with tier. Higher-tier recipes demand exponentially more gold, keeping low-tier crafting accessible and high-tier crafting a meaningful investment.

### Gold Drop Amounts

| Enemy Tier | Gold per Kill |
|------------|---------------|
| Tier 1     | 5–15          |
| Tier 2     | 15–40         |
| Tier 3     | 40–100        |
| Tier 4     | 100–250       |
| Tier 5     | 250–500       |
| Boss       | 500–2,000     |

---

## 2. Runes — Crafting Catalysts

Eight elemental runes drop from enemies. Each rune is tied to one material category and is required to refine that category's materials and craft gear from those components.

| Rune           | Material Category | Used For                                              |
|----------------|-------------------|-------------------------------------------------------|
| Fire Rune      | Metal             | Ore → Ingots; metal weapons and armor                 |
| Wind Rune      | Wood              | Logs → Lumber; wooden weapons, bows, staves           |
| Ice Rune       | Leather           | Hides → Leather; leather armor and grips              |
| Water Rune     | Cloth             | Fibers → Cloth; cloth armor and capes                 |
| Earth Rune     | Bone              | Bone → Bone components; bone weapons and charms       |
| Lightning Rune | Precious Metal    | Silver/gold/platinum ore → Ingots; jewelry and ornate gear |
| Light Rune     | Stone             | Stone → Stone components; stone weapons, shields, heavy armor |
| Dark Rune      | Void/Essence      | Void shards → Essences; cursed weapons and dark armor |

### Rune Drop Rates

| Source                        | Drop Rate        |
|-------------------------------|------------------|
| Elementals (matching element) | 40%              |
| Beasts, Plants, Constructs    | 8% (random type) |
| Bosses                        | 2–4 guaranteed   |

**Stack size:** 99 per inventory slot.

---

## 3. Glyphs — Enchanting Catalysts

Eight elemental glyphs drop from enemies. Each glyph is tied to one element and determines the prefix/suffix pools when enchanting.

| Glyph           | Enchant Theme                        |
|-----------------|--------------------------------------|
| Fire Glyph      | Offense, burn, crit, execute         |
| Ice Glyph       | Control, defense, slow, freeze       |
| Wind Glyph      | Speed, evasion, ranged, multi-hit    |
| Earth Glyph     | Defense, HP, stagger, thorns         |
| Lightning Glyph | Crit, speed, chain, stun             |
| Water Glyph     | Healing, support, cleanse, drain     |
| Light Glyph     | Holy, shields, buffs, resistance     |
| Dark Glyph      | Drain, debuff, DoT, backstab         |

### Glyph Drop Rates

| Source                        | Drop Rate                              |
|-------------------------------|----------------------------------------|
| Elementals (matching element) | 2%                                     |
| Demons                        | 3% (random type)                       |
| Dragons                       | 2% (random type)                       |
| Skeletons/Undead              | 1% (Dark Glyph only)                   |
| Bosses                        | 1 guaranteed + 25% chance at a second  |

Glyphs are never sold by vendors.

**Stack size:** 99 per inventory slot.

---

## 4. Equipment Slots

### Armor Slots (7 total)
| Slot   | Types                        |
|--------|------------------------------|
| Head   | Helm, Hood, Cap, Crown       |
| Chest  | Armor, Robe, Vest, Coat      |
| Hands  | Gloves, Gauntlets            |
| Legs   | Pants, Greaves               |
| Feet   | Boots, Shoes                 |
| Back   | Cape, Cloak                  |
| Shield | Shield (off-hand, optional)  |

### Accessory Slots (4 total)
| Slot      | Types                    |
|-----------|--------------------------|
| Neck      | Necklace, Amulet         |
| Ear       | Earring (1 slot)         |
| Ring      | Ring (2 slots)           |
| Bracelet  | Bracelet (1 slot)        |

### Weapon Slots
| Style       | Slots Used                           |
|-------------|--------------------------------------|
| One-handed  | Main hand only; shield in off-hand   |
| Dual wield  | Main hand + off-hand weapon          |
| Two-handed  | Both hands; no shield                |

**Shield clarification:** Shields are armor (off-hand slot), not weapons. They provide Defense and Block chance, not damage.

---

## 5. Base Stats by Tier

All gear has base stats determined by tier. Enchantment percentages multiply these bases.

### Weapon Base Stats

| Tier | Base Damage | Base Attack | Crit Rate |
|------|-------------|-------------|-----------|
| 1    | 8–12        | 5           | 2%        |
| 2    | 14–20       | 10          | 3%        |
| 3    | 24–32       | 18          | 4%        |
| 4    | 38–48       | 28          | 5%        |
| 5    | 55–70       | 42          | 6%        |

### Armor Base Stats

| Tier | Base Defense | Base HP | Base Resistance |
|------|--------------|---------|-----------------|
| 1    | 3            | 15      | 1%              |
| 2    | 6            | 30      | 2%              |
| 3    | 12           | 50      | 3%              |
| 4    | 20           | 80      | 4%              |
| 5    | 32           | 120     | 5%              |

### Accessory Base Stats

| Tier | Primary Stat | Secondary Stat |
|------|--------------|----------------|
| 1    | +3           | +1             |
| 2    | +6           | +2             |
| 3    | +12          | +4             |
| 4    | +20          | +7             |
| 5    | +32          | +11            |

---

## 6. Weapon Types

Each weapon type has distinct characteristics. Base Damage scales with tier (see §5).

| Type        | Hands | Speed   | Range  | Special                          |
|-------------|-------|---------|--------|----------------------------------|
| Sword       | 1H    | Medium  | Melee  | Balanced; no penalty             |
| Dagger      | 1H    | Fast    | Melee  | +15% crit, −10% damage           |
| Axe         | 1H    | Slow    | Melee  | +20% damage, −15% speed          |
| Mace        | 1H    | Slow    | Melee  | +10% stagger, ignores 10% armor  |
| Rapier      | 1H    | Fast    | Melee  | +10% crit damage, +5% evasion    |
| Spear       | 2H    | Medium  | Reach  | 1.5× range, +10% vs charging     |
| Greatsword  | 2H    | Slow    | Melee  | +40% damage, cleave hits 2       |
| Scythe      | 2H    | Slow    | Reach  | +25% crit damage, lifesteal 3%   |
| Staff       | 2H    | Medium  | Melee  | +20% magic damage, +10 MP        |
| Bow         | 2H    | Medium  | Ranged | 15m range, requires arrows       |
| Knuckles    | 1H    | V.Fast  | Melee  | Dual hit, +25% speed, −20% damage|
| Club        | 1H    | Medium  | Melee  | +15% stun chance                 |
| Hammer      | 2H    | V.Slow  | Melee  | +50% damage, +20% stagger        |
| Knife       | 1H    | V.Fast  | Melee  | Throwable (8m), +20% backstab    |

**Speed tiers:** V.Fast (0.8s), Fast (1.0s), Medium (1.3s), Slow (1.6s), V.Slow (2.0s)

---

## 7. Materials — By Category and Tier

Materials drop from enemies and are refined into components using the matching rune. Categories cap at Tier 5 (no Tier 6–7). Tier 5 is endgame; future expansions may add Tier 6+.

### Material Drop Quantities

| Enemy Tier vs Material Tier | Drop Quantity |
|-----------------------------|---------------|
| Equal tier                  | 1–2           |
| 1 tier below enemy          | 2–3           |
| 2+ tiers below enemy        | 3–4           |

### Metal (Fire Rune)

| Tier | Raw Material              | Component        | Source                     | Gold |
|------|---------------------------|------------------|----------------------------|------|
| 1    | Copper Ore ×3             | Copper Ingot     | Cave enemies, mining nodes | 10   |
| 2    | Tin Ore ×3                | Tin Ingot        | Cave enemies               | 25   |
| 2    | Copper Ingot + Tin Ingot  | Bronze Ingot     | Combo (Fire Rune)          | 50   |
| 3    | Iron Ore ×3               | Iron Ingot       | Mountain enemies           | 100  |
| 4    | Iron Ingot + Coal ×2      | Steel Ingot      | Combo (Fire Rune)          | 400  |
| 5    | Mythril Ore ×3            | Mythril Ingot    | Deep cave enemies          | 1,000|

### Wood (Wind Rune)

| Tier | Raw Material        | Component          | Source                 | Gold |
|------|---------------------|--------------------|------------------------|------|
| 1    | Arrowwood Log ×3    | Arrowwood Lumber   | Forest enemies         | 10   |
| 2    | Maple Log ×3        | Maple Lumber       | Forest enemies         | 25   |
| 3    | Ash Log ×3          | Ash Lumber         | Forest enemies         | 100  |
| 4    | Oak Log ×3          | Oak Lumber         | Ancient forest enemies | 250  |
| 5    | Mahogany Log ×3     | Mahogany Lumber    | Jungle enemies         | 1,000|

### Leather (Ice Rune)

| Tier | Raw Material        | Component          | Source          | Gold |
|------|---------------------|--------------------|-----------------|------|
| 1    | Sheepskin ×3        | Sheep Leather      | Plains beasts   | 10   |
| 2    | Strider Hide ×3     | Strider Leather    | Desert beasts   | 25   |
| 3    | Tiger Hide ×3       | Tiger Leather      | Jungle beasts   | 100  |
| 4    | Panzer Hide ×3      | Panzer Leather     | Highland beasts | 250  |
| 5    | Buffalo Hide ×3     | Buffalo Leather    | Tundra beasts   | 1,000|

### Cloth (Water Rune)

| Tier | Raw Material        | Component          | Source            | Gold |
|------|---------------------|--------------------|-------------------|------|
| 1    | Meadow Grass ×3     | Grass Cloth        | Plant enemies     | 10   |
| 2    | Flax Flower ×3      | Linen Cloth        | Plant enemies     | 25   |
| 3    | Cotton ×3           | Cotton Cloth       | Farmland enemies  | 100  |
| 4    | Wool ×3             | Wool Cloth         | Mountain beasts   | 250  |
| 5    | Silk Thread ×3      | Silk Cloth         | Spider enemies    | 1,000|

### Bone (Earth Rune)

| Tier | Raw Material        | Component              | Source           | Gold |
|------|---------------------|------------------------|------------------|------|
| 1    | Bone Chip ×3        | Bone Component         | Skeleton enemies | 10   |
| 2    | Sheep Tooth ×3      | Tooth Component        | Beast enemies    | 25   |
| 3    | Beetle Shell ×3     | Shell Component        | Insect enemies   | 100  |
| 4    | Turtle Shell ×3     | Turtle Shell Component | Reptile enemies  | 250  |
| 5    | Coral Fragment ×3   | Coral Component        | Sea enemies      | 1,000|

### Precious Metal (Lightning Rune)

| Tier | Raw Material              | Component          | Source                 | Gold |
|------|---------------------------|--------------------|------------------------|------|
| 1    | Silver Ore ×3             | Silver Ingot       | Cave enemies           | 15   |
| 2    | Gold Ore ×3               | Gold Ingot         | Deep cave enemies      | 40   |
| 3    | Platinum Ore ×3           | Platinum Ingot     | Volcanic enemies       | 150  |
| 4    | Silver Ingot + Gold Ingot | Electrum Ingot     | Combo (Lightning Rune) | 400  |
| 5    | Orichalcum Ore ×3         | Orichalcum Ingot   | Ancient ruins enemies  | 1,500|

### Stone (Light Rune)

| Tier | Raw Material        | Component            | Source           | Gold |
|------|---------------------|----------------------|------------------|------|
| 1    | Ragstone ×3         | Ragstone Component   | Rocky enemies    | 10   |
| 2    | Flint Stone ×3      | Flint Component      | Cave enemies     | 25   |
| 3    | Granite ×3          | Granite Component    | Mountain enemies | 100  |
| 4    | Marble ×3           | Marble Component     | Ruins enemies    | 250  |
| 5    | Obsidian ×3         | Obsidian Component   | Volcanic enemies | 1,000|

### Void/Essence (Dark Rune)

| Tier | Raw Material        | Component            | Source            | Gold |
|------|---------------------|----------------------|-------------------|------|
| 1    | Dark Crystal ×3     | Dark Essence         | Shadow enemies    | 15   |
| 2    | Shadow Flask ×3     | Shadow Essence       | Undead enemies    | 40   |
| 3    | Void Shard ×3       | Void Essence         | Void enemies      | 150  |
| 4    | Abyss Fragment ×3   | Abyss Essence        | Deep void enemies | 400  |
| 5    | Nightmare Shard ×3  | Nightmare Essence    | Nightmare enemies | 1,500|

### Combo Refine Rules

Combo refines (Bronze, Steel, Electrum) follow these rules:
- **Rune:** Uses the rune of the primary output category (Fire for metal alloys, Lightning for precious metal alloys)
- **Gold:** 2× the base tier cost of the output tier
- **Input components are consumed**, not raw materials

---

## 8. Base Gear — Craft Recipes

Crafting base gear consumes one rune (matching the primary material), components, and gold. Base gear has no enchantment until a glyph is applied.

**Primary material rule:** The rune matches the first listed component. Secondary components from other categories do not require additional runes.

### Weapons

| Weapon              | Rune      | Components                           | Tier | Gold  |
|---------------------|-----------|--------------------------------------|------|-------|
| Copper Sword        | Fire      | Copper Ingot ×2, Arrowwood Lumber    | 1    | 25    |
| Bronze Dagger       | Fire      | Bronze Ingot ×1, Sheep Leather       | 2    | 50    |
| Iron Sword          | Fire      | Iron Ingot ×3, Oak Lumber            | 3    | 250   |
| Steel Axe           | Fire      | Steel Ingot ×3, Ash Lumber           | 4    | 500   |
| Mythril Spear       | Fire      | Mythril Ingot ×2, Mahogany Lumber    | 5    | 2,500 |
| Maple Bow           | Wind      | Maple Lumber ×3, Sheep Leather       | 2    | 50    |
| Oak Staff           | Wind      | Oak Lumber ×3, Silver Ingot          | 3    | 250   |
| Mahogany Bow        | Wind      | Mahogany Lumber ×3, Silk Cloth       | 5    | 2,500 |
| Bone Club           | Earth     | Bone Component ×3, Sheep Leather     | 1    | 25    |
| Turtle Shell Shield | Light     | Turtle Shell Component ×3, Iron Ingot| 4    | 500   |
| Coral Knuckles      | Earth     | Coral Component ×3, Silk Cloth       | 5    | 2,500 |
| Silver Dagger       | Lightning | Silver Ingot ×2, Sheep Leather       | 1    | 25    |
| Gold Rapier         | Lightning | Gold Ingot ×3, Silk Cloth            | 2    | 50    |
| Platinum Knife      | Lightning | Platinum Ingot ×2, Wool Cloth        | 3    | 250   |
| Orichalcum Blade    | Lightning | Orichalcum Ingot ×3, Mahogany Lumber | 5    | 2,500 |
| Obsidian Hammer     | Light     | Obsidian Component ×3, Steel Ingot   | 5    | 2,500 |
| Marble Mace         | Light     | Marble Component ×3, Iron Ingot      | 4    | 500   |
| Void Scythe         | Dark      | Void Essence ×3, Steel Ingot         | 3    | 250   |
| Nightmare Blade     | Dark      | Nightmare Essence ×3, Mythril Ingot  | 5    | 2,500 |

### Armor

| Armor                   | Rune      | Components                           | Tier | Gold  |
|-------------------------|-----------|--------------------------------------|------|-------|
| Sheep Leather Vest      | Ice       | Sheep Leather ×2, Grass Cloth        | 1    | 25    |
| Strider Leather Helm    | Ice       | Strider Leather ×2, Bronze Ingot     | 2    | 50    |
| Tiger Leather Boots     | Ice       | Tiger Leather ×2, Iron Ingot         | 3    | 250   |
| Panzer Leather Gloves   | Ice       | Panzer Leather ×2, Steel Ingot       | 4    | 500   |
| Buffalo Leather Cuirass | Ice       | Buffalo Leather ×3, Mythril Ingot    | 5    | 2,500 |
| Grass Cloth Robe        | Water     | Grass Cloth ×3, Sheep Leather        | 1    | 25    |
| Linen Cloth Cap         | Water     | Linen Cloth ×2, Bronze Ingot         | 2    | 50    |
| Cotton Cloth Pants      | Water     | Cotton Cloth ×2, Iron Ingot          | 3    | 250   |
| Wool Cloth Cape         | Water     | Wool Cloth ×3, Silver Ingot          | 4    | 500   |
| Silk Cloth Hood         | Water     | Silk Cloth ×2, Gold Ingot            | 5    | 2,500 |
| Iron Chainmail          | Fire      | Iron Ingot ×4, Sheep Leather         | 3    | 250   |
| Steel Plate Armor       | Fire      | Steel Ingot ×4, Strider Leather      | 4    | 500   |
| Mythril Chainmail       | Fire      | Mythril Ingot ×4, Panzer Leather     | 5    | 2,500 |
| Bone Helm               | Earth     | Bone Component ×3, Sheep Leather     | 1    | 25    |
| Shell Shield            | Earth     | Shell Component ×3, Bronze Ingot     | 3    | 250   |
| Coral Ring              | Earth     | Coral Component ×2, Silver Ingot     | 5    | 2,500 |
| Silver Earring          | Lightning | Silver Ingot ×1, Ragstone Component  | 1    | 25    |
| Gold Necklace           | Lightning | Gold Ingot ×2, Coral Component       | 2    | 50    |
| Platinum Ring           | Lightning | Platinum Ingot ×1, Marble Component  | 3    | 250   |
| Electrum Bracelet       | Lightning | Electrum Ingot ×2, Obsidian Component| 4    | 500   |
| Orichalcum Crown        | Lightning | Orichalcum Ingot ×2, Nightmare Essence| 5   | 2,500 |
| Ragstone Shield         | Light     | Ragstone Component ×3, Copper Ingot  | 1    | 25    |
| Granite Gauntlets       | Light     | Granite Component ×3, Iron Ingot     | 3    | 250   |
| Obsidian Helm           | Light     | Obsidian Component ×3, Steel Ingot   | 5    | 2,500 |
| Dark Essence Ring       | Dark      | Dark Essence ×2, Silver Ingot        | 1    | 25    |
| Void Essence Amulet     | Dark      | Void Essence ×2, Gold Ingot          | 3    | 250   |
| Nightmare Cloak         | Dark      | Nightmare Essence ×3, Silk Cloth     | 5    | 2,500 |

---

## 9. Enchanting — Glyph Tables

Applying a glyph consumes the glyph, the gold cost, and overwrites any existing enchantment on the gear. The item gains a prefix and a suffix, each rolled from the glyph's pools with a numeric value inside the listed range.

### Enchant Restrictions

- **Any glyph can enchant any gear.** A Fire Glyph on a healing robe is valid (adds offensive stats to a defensive piece).
- **Re-enchanting costs the same** as first enchant (500 + 100 × tier).
- **Overwrite destroys the old enchantment** completely—no trace remains.

### Range Scaling

The percentage range is fixed per ability. Higher-tier gear has higher base stats, so the same percentage is worth more absolute value.

**Example:** Ember (+5% Attack) on Tier 1 Copper Sword (5 base Attack) = +0.25 Attack. On Tier 5 Mythril Spear (42 base Attack) = +2.1 Attack.

### Fire Glyph

| Prefix Pool                     | Suffix Pool                        |
|---------------------------------|------------------------------------|
| Ember — Attack +3–7%            | of Cinders — Burn damage +5–12%    |
| Blazing — Fire damage +4–9%     | of Ash — Crit chance +2–5%         |
| Inferno — Area damage +5–10%    | of Flame — Execute damage +6–14%   |
| Scorching — Burn chance +3–6%   | of Heat — Fire penetration +4–8%   |

### Ice Glyph

| Prefix Pool                     | Suffix Pool                        |
|---------------------------------|------------------------------------|
| Frost — Ice damage +4–9%        | of Chill — Slow potency +5–12%     |
| Glacial — Defense +3–7%         | of Hail — Crit vs chilled +4–10%   |
| Permafrost — Max HP +4–8%       | of Rime — Cooldown reduction +2–6% |
| Chilling — Slow chance +3–7%    | of Snow — Ice resistance +4–9%     |

### Wind Glyph

| Prefix Pool                     | Suffix Pool                        |
|---------------------------------|------------------------------------|
| Gale — Attack speed +3–7%       | of Squalls — Movement speed +2–6%  |
| Zephyr — Evasion +4–9%          | of Currents — Projectile speed +5–10% |
| Tempest — Multi-hit damage +4–8%| of Cyclones — Area size +5–12%     |
| Whirling — Dodge chance +2–5%   | of Breezes — Stamina regen +3–7%   |

### Earth Glyph

| Prefix Pool                     | Suffix Pool                        |
|---------------------------------|------------------------------------|
| Stone — Armor +4–8%             | of Quakes — Stagger damage +5–12%  |
| Granite — Damage reduction +3–6%| of Moss — Regen +4–10%             |
| Bedrock — Block chance +3–7%    | of Bulwarks — Thorns +5–11%        |
| Earthen — Max HP +4–8%          | of Tremors — Knockback +4–9%       |

### Lightning Glyph

| Prefix Pool                     | Suffix Pool                        |
|---------------------------------|------------------------------------|
| Spark — Lightning damage +4–9%  | of Arcs — Chain chance +3–7%       |
| Thunder — Crit damage +5–11%    | of Storms — Stun chance +2–6%      |
| Storm — Attack speed +3–7%      | of Voltage — Energy regen +4–10%   |
| Charged — Energy regen +3–6%    | of Bolts — Paralysis chance +2–5%  |

### Water Glyph

| Prefix Pool                     | Suffix Pool                        |
|---------------------------------|------------------------------------|
| Tide — Healing +4–10%           | of Mists — Evasion +3–7%           |
| Wave — Water damage +4–9%       | of Rain — Resource regen +4–9%     |
| Abyss — Lifesteal +3–7%         | of Tides — Debuff duration +5–12%  |
| Flowing — Resource regen +4–9%  | of Rivers — Cleanse chance +3–6%   |

### Light Glyph

| Prefix Pool                     | Suffix Pool                        |
|---------------------------------|------------------------------------|
| Dawn — Healing +4–9%            | of Grace — Buff duration +5–10%    |
| Radiant — Holy damage +4–10%    | of Sanctuary — Shield strength +5–12% |
| Prism — Elemental resistance +4–8% | of Glory — Team stat aura +2–5% |
| Hallowed — Shield strength +3–7%| of Purity — Cleanse potency +4–9%  |

### Dark Glyph

| Prefix Pool                     | Suffix Pool                        |
|---------------------------------|------------------------------------|
| Shadow — Dark damage +4–9%      | of Night — Backstab damage +5–12%  |
| Void — Debuff potency +5–11%    | of Curses — DoT damage +5–12%      |
| Blood — Lifesteal +3–7%         | of Draining — Resource drain +4–10%|
| Cursed — DoT potency +4–9%      | of Gloom — Debuff duration +5–10%  |

### Enchantment Examples

| Base Gear           | Glyph | Rolled Prefix              | Rolled Suffix                  | Result                                   |
|---------------------|-------|----------------------------|--------------------------------|------------------------------------------|
| Iron Sword          | Fire  | Blazing (Fire damage +7%)  | of Cinders (Burn +10%)         | Blazing Iron Sword of Cinders            |
| Iron Sword          | Fire  | Ember (Attack +5%)         | of Ash (Crit +3%)              | Ember Iron Sword of Ash                  |
| Tiger Leather Boots | Wind  | Gale (Attack speed +6%)    | of Squalls (Movement +4%)      | Gale Tiger Leather Boots of Squalls      |
| Silver Earring      | Water | Tide (Healing +8%)         | of Mists (Evasion +5%)         | Tide Silver Earring of Mists             |
| Obsidian Helm       | Earth | Bedrock (Block +6%)        | of Bulwarks (Thorns +9%)       | Bedrock Obsidian Helm of Bulwarks        |
| Nightmare Blade     | Dark  | Blood (Lifesteal +6%)      | of Night (Backstab +11%)       | Blood Nightmare Blade of Night           |

---

## 10. Crafting Ranks

Each material category has its own crafting rank (1–10). Crafting components and gear at or near your rank grants rank experience. Higher ranks unlock higher-tier recipes and reduce gold costs slightly.

| Rank | Unlocks            | Gold Discount | XP to Next |
|------|--------------------|---------------|------------|
| 1    | Tier 1 recipes     | 0%            | 100        |
| 2    | Tier 2 recipes     | 2%            | 250        |
| 3    | Tier 3 recipes     | 4%            | 500        |
| 4    | Tier 4 recipes     | 6%            | 1,000      |
| 5    | Tier 5 recipes     | 8%            | 2,000      |
| 6    | —                  | 10%           | 3,500      |
| 7    | +1 component yield | 12%           | 5,500      |
| 8    | +1 component yield | 14%           | 8,000      |
| 9    | +2 component yield | 16%           | 12,000     |
| 10   | +5% enchant range  | 20%           | MAX        |

### Rank XP Gains

| Action                        | XP Earned |
|-------------------------------|-----------|
| Refine at current rank        | 25        |
| Refine 1 rank below           | 12        |
| Refine 2 ranks below          | 3         |
| Refine 3+ ranks below         | 0         |
| Craft at current rank         | 50        |
| Craft 1 rank below            | 25        |
| Craft 2 ranks below           | 6         |
| Craft 3+ ranks below          | 0         |

**Yield bonus:** At rank 7+, refining yields one extra component per refine. At rank 9+, two extra. At rank 10, enchantment numeric rolls gain +5% to their maximum range (e.g., Ember's top end becomes 7.35% instead of 7%).

---

## 11. Durability and Repair

All gear has durability that decreases with use.

| Event                  | Durability Loss |
|------------------------|-----------------|
| Take damage (armor)    | 1 per hit       |
| Attack (weapon)        | 1 per swing     |
| Death                  | 10 all gear     |
| Block with shield      | 2 per block     |

### Durability Pools

| Tier | Max Durability |
|------|----------------|
| 1    | 50             |
| 2    | 75             |
| 3    | 100            |
| 4    | 125            |
| 5    | 150            |

### Repair

- **Location:** Any crafting station or blacksmith NPC
- **Cost:** 10 × gear tier per 25% durability restored
- **Broken gear (0 durability):** Cannot be used until repaired; no stat bonuses apply

---

## 12. Salvage

Break down gear into materials at any time for a flat 50 gold fee.

| Item Salvaged   | Returns                                                              |
|-----------------|----------------------------------------------------------------------|
| Base gear       | 1–2 components of its primary material + 25% chance to recover the rune |
| Enchanted gear  | 1–2 components + 25% chance to recover the rune + 10% chance to recover the glyph |

Salvage is the primary sink for unwanted enchanted gear and the primary source of recovered glyphs for re-enchanting.

---

## 13. Inventory and Storage

### Stack Sizes

| Item Type   | Stack Size |
|-------------|------------|
| Runes       | 99         |
| Glyphs      | 99         |
| Materials   | 99         |
| Components  | 99         |
| Gold        | 999,999    |
| Gear        | 1 (no stack)|

### Inventory Slots

| Storage Type     | Slots |
|------------------|-------|
| Personal inventory | 40  |
| Bank (per town)  | 80    |
| Shared bank      | 20    |

---

## 14. Crafting Stations

Crafting requires the appropriate station. Stations are found in towns and sometimes in the field.

| Station    | Actions Available                     | Location             |
|------------|---------------------------------------|----------------------|
| Forge      | Refine Metal, Craft Metal gear        | Blacksmith, mines    |
| Workbench  | Refine Wood, Craft Wood gear          | Carpenter, forests   |
| Tannery    | Refine Leather, Craft Leather gear    | Leatherworker, camps |
| Loom       | Refine Cloth, Craft Cloth gear        | Weaver, towns        |
| Bone Altar | Refine Bone, Craft Bone gear          | Shaman, crypts       |
| Jeweler    | Refine Precious Metal, Craft Jewelry  | Jeweler, markets     |
| Stonecutter| Refine Stone, Craft Stone gear        | Mason, quarries      |
| Dark Altar | Refine Essence, Craft Dark gear       | Void shrines         |
| Enchanter  | Enchant any gear                      | Enchanter, towers    |
| Salvage Pit| Salvage any gear                      | All stations         |

**Field stations** are smaller and limited to Tier 1–3 recipes. Town stations support all tiers.

---

## 15. Enemy Drop Tables

| Enemy Type               | Common Drops                  | Uncommon Drops         | Rare Drops                      |
|--------------------------|-------------------------------|------------------------|---------------------------------|
| Beasts (wolves, bears, tigers) | Hides ×1–2, Bone ×1, Gold | Random Rune (8%)       | —                               |
| Birds                    | Feathers ×1–2 (cloth), Gold   | Random Rune (8%)       | —                               |
| Reptiles                 | Hides ×1–2, Shells ×1, Gold   | Random Rune (8%)       | —                               |
| Insects                  | Shells ×1–2, Fibers ×1, Gold  | Random Rune (8%)       | —                               |
| Plants                   | Fibers ×1–3, Logs ×1, Gold    | Random Rune (8%)       | —                               |
| Skeletons/Undead         | Bone ×1–2, Dark Crystals ×1, Gold | Dark Rune (15%)    | Dark Glyph (1%)                 |
| Elementals               | Gold                          | Matching Rune (40%)    | Matching Glyph (2%)             |
| Constructs               | Stone ×1–2, Metal Ore ×1, Gold| Random Rune (8%)       | —                               |
| Demons                   | Demon Skulls ×1, Dark Essence ×1, Gold | Random Rune (10%) | Random Glyph (3%)            |
| Dragons                  | Dragon Hide ×1–2, Dragon Talon ×1, Gold | Random Rune ×2 (15% each) | Random Glyph (2%)   |
| Bosses                   | Gold ×5, Rare Materials ×2–4  | 2–4 Random Runes (100%)| 1 Guaranteed Glyph + 25% second |

---

## 16. Unique and Boss Gear

Some gear cannot be crafted and only drops from bosses or quests.

### Unique Gear Rules

- **Cannot be salvaged** — no component recovery
- **Cannot be enchanted** — fixed stats, no glyph slots
- **Cannot be traded** — soulbound on pickup
- **Higher base stats** — +20% over equivalent tier crafted gear
- **Special effects** — unique proc or passive not available via enchanting

### Example Uniques

| Item                | Source              | Tier | Special Effect                        |
|---------------------|---------------------|------|---------------------------------------|
| Frostbane Blade     | Ice Elemental Boss  | 4    | Attacks freeze enemies for 1s (10%)   |
| Crown of the Depths | Sea Leviathan       | 5    | Breathe underwater, +50% swim speed   |
| Voidheart Pendant   | Nightmare Lord      | 5    | Absorb 5% damage as MP                |
| Earthshaker Hammer  | Stone Colossus      | 5    | Ground slam AOE on crit               |

Uniques are rare (5–10% boss drop rate) and fill niche roles. Crafted gear with good enchants can match or exceed unique power for general use.

---

## 17. Full Example Crafts

### Example A: Tier 3 Fire Sword

1. Kill mountain enemies until you have 9 Iron Ore and 2 Fire Runes.
2. Refine ×3: Fire Rune + 3 Iron Ore + 100 Gold → Iron Ingot (×3).
3. Kill ancient forest enemies for 3 Oak Logs and a Wind Rune.
4. Refine: Wind Rune + 3 Oak Logs + 250 Gold → Oak Lumber.
5. Craft at Forge: Fire Rune + 3 Iron Ingots + 1 Oak Lumber + 250 Gold → Iron Sword.
6. Kill a fire elemental until it drops a Fire Glyph.
7. Enchant at Enchanter: Iron Sword + Fire Glyph + 800 Gold → rolls prefix and suffix:
   - Prefix: Inferno — Area damage +8%
   - Suffix: of Flame — Execute damage +12%
   - Result: **Inferno Iron Sword of Flame**

**Total cost:** 3 Fire Runes, 1 Wind Rune, 9 Iron Ore, 3 Oak Logs, 1 Fire Glyph, 1,100 Gold.

### Example B: Tier 5 Water Robe

1. Farm spider enemies for Silk Thread ×9 and 3 Water Runes.
2. Refine ×3 at Loom: Water Rune + 3 Silk Thread + 1,000 Gold → Silk Cloth (×3).
3. Farm deep cave enemies for Gold Ore ×3 and a Lightning Rune.
4. Refine at Jeweler: Lightning Rune + 3 Gold Ore + 40 Gold → Gold Ingot.
5. Craft at Loom: Water Rune + 2 Silk Cloth + 1 Gold Ingot + 2,500 Gold → Silk Cloth Hood.
6. Farm water elementals for a Water Glyph.
7. Enchant: Silk Cloth Hood + Water Glyph + 1,000 Gold → rolls:
   - Prefix: Tide — Healing +9%
   - Suffix: of Tides — Debuff duration +8%
   - Result: **Tide Silk Cloth Hood of Tides**

**Total cost:** 4 Water Runes, 1 Lightning Rune, 9 Silk Thread, 3 Gold Ore, 1 Water Glyph, 6,540 Gold.

### Example C: Salvage Loop

1. You craft an Ember Copper Sword of Ash but want a Fire damage roll instead.
2. Salvage the sword at any station for 50 gold → returns 1–2 Copper Ingots + 25% chance at the Fire Rune + 10% chance at the Fire Glyph.
3. Re-craft the sword and re-enchant with another Fire Glyph, hoping for Blazing and of Cinders.
4. Repeat until you get the roll you want.

---

## 18. System Summary

| Feature          | Rule                                                              |
|------------------|-------------------------------------------------------------------|
| Runes            | Enemy drops only. 8 types. Consumed in refine and craft.          |
| Glyphs           | Enemy drops only. 8 types. No tiers. Consumed in enchant.         |
| Materials        | Enemy drops only. 8 categories × 5 tiers.                         |
| Components       | Crafted: Rune + 3 Materials + Gold.                               |
| Base Gear        | Crafted: Rune + Components + Gold.                                |
| Enchanted Gear   | Base gear + Glyph + Gold. Rolls 1 prefix + 1 suffix.              |
| Enchant Overwrite| Applying a new glyph destroys the old enchantment.                |
| Salvage          | Breaks gear into components + chance at rune/glyph.               |
| Crafting Ranks   | 1–10 per material category. Unlocks tiers, discounts gold, boosts yield. |
| Gold             | Earned from enemies, quests, salvage. Spent on every action.      |
| Durability       | Gear degrades with use. Repair at stations for gold.              |
| Stations         | Category-specific stations in towns and field.                    |
| Uniques          | Boss drops with fixed special effects. Cannot be enchanted.       |

This keeps glyph collection to eight items total while the prefix/suffix pools, numeric ranges, gear tiers, and crafting ranks create thousands of distinct enchanted items.
