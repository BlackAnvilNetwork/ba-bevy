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

---

## 1. Currency and Costs

Gold is earned from enemies, quests, and salvage. It is consumed in every refine, craft, enchant, and salvage action.

| Action             | Gold Cost       |
|--------------------|-----------------|
| Refine (Tier 1–2)  | 10–50           |
| Refine (Tier 3–4)  | 100–500         |
| Refine (Tier 5–6)  | 1,000–5,000     |
| Refine (Tier 7)    | 10,000+         |
| Craft (Tier 1–2)   | 25–100          |
| Craft (Tier 3–4)   | 250–1,000       |
| Craft (Tier 5–6)   | 2,500–10,000    |
| Craft (Tier 7)     | 25,000+         |
| Enchant (any tier) | 500 + 100 × gear tier |
| Salvage            | 50 flat         |

Gold costs scale with tier. Higher-tier recipes demand exponentially more gold, keeping low-tier crafting accessible and high-tier crafting a meaningful investment.

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

**Drop sources:** Elementals drop their matching rune commonly. Beasts, plants, and constructs drop random runes uncommonly. Bosses drop 2–4 runes of various types.

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

**Drop sources:** Elementals drop their matching glyph rarely (≈2%). Demons drop random glyphs rarely (≈3%). Bosses drop 1 guaranteed glyph plus a chance at a second. Glyphs are never sold by vendors.

**Stack size:** 99 per inventory slot.

---

## 4. Materials — By Category and Tier

Materials drop from enemies and are refined into components using the matching rune. Each category has seven tiers. Higher tiers require rarer materials and more gold.

### Metal (Fire Rune)

| Tier | Raw Material              | Component        | Source                |
|------|---------------------------|------------------|-----------------------|
| 1    | Copper Ore ×3             | Copper Ingot     | Cave enemies, mining nodes |
| 2    | Tin Ore ×3                | Tin Ingot        | Cave enemies          |
| 2    | Copper Ingot + Tin Ingot  | Bronze Ingot     | Refine combo          |
| 3    | Iron Ore ×3               | Iron Ingot       | Mountain enemies      |
| 4    | Iron Ingot + Coal ×2      | Steel Ingot      | Refine combo          |
| 5    | Mythril Ore ×3            | Mythril Ingot    | Deep cave enemies     |
| 6    | Cobalt Ore ×3             | Cobalt Ingot     | Volcanic enemies      |
| 7    | Steel Ingot + Dark Essence| Darksteel Ingot  | Refine combo          |

### Wood (Wind Rune)

| Tier | Raw Material        | Component          | Source                |
|------|---------------------|--------------------|-----------------------|
| 1    | Arrowwood Log ×3    | Arrowwood Lumber   | Forest enemies        |
| 2    | Maple Log ×3        | Maple Lumber       | Forest enemies        |
| 3    | Ash Log ×3          | Ash Lumber         | Forest enemies        |
| 4    | Oak Log ×3          | Oak Lumber         | Ancient forest enemies|
| 5    | Mahogany Log ×3     | Mahogany Lumber    | Jungle enemies        |
| 6    | Rosewood Log ×3     | Rosewood Lumber    | Rare tree enemies     |
| 7    | Ebony Log ×3        | Ebony Lumber       | Shadow forest enemies |

### Leather (Ice Rune)

| Tier | Raw Material        | Component          | Source                |
|------|---------------------|--------------------|-----------------------|
| 1    | Sheepskin ×3        | Sheep Leather      | Plains beasts         |
| 2    | Strider Hide ×3     | Strider Leather    | Desert beasts         |
| 3    | Tiger Hide ×3       | Tiger Leather      | Jungle beasts         |
| 4    | Panzer Hide ×3      | Panzer Leather     | Highland beasts       |
| 5    | Buffalo Hide ×3     | Buffalo Leather    | Tundra beasts         |
| 6    | Raptor Hide ×3      | Raptor Leather     | Canyon beasts         |
| 7    | Dragon Hide ×3      | Dragon Leather     | Dragon enemies        |

### Cloth (Water Rune)

| Tier | Raw Material        | Component          | Source                |
|------|---------------------|--------------------|-----------------------|
| 1    | Meadow Grass ×3     | Grass Cloth        | Plant enemies         |
| 2    | Flax Flower ×3      | Linen Cloth        | Plant enemies         |
| 3    | Cotton ×3           | Cotton Cloth       | Farmland enemies      |
| 4    | Wool ×3             | Wool Cloth         | Mountain beasts       |
| 5    | Silk Thread ×3      | Silk Cloth         | Spider enemies        |
| 6    | Velvet Thread ×3    | Velvet Cloth       | Rare spider enemies   |
| 7    | Rainbow Thread ×3   | Rainbow Cloth      | Rainbow enemies       |

### Bone (Earth Rune)

| Tier | Raw Material        | Component              | Source            |
|------|---------------------|------------------------|-------------------|
| 1    | Bone Chip ×3        | Bone Component         | Skeleton enemies  |
| 2    | Sheep Tooth ×3      | Tooth Component        | Beast enemies     |
| 3    | Beetle Shell ×3     | Shell Component        | Insect enemies    |
| 4    | Turtle Shell ×3     | Turtle Shell Component | Reptile enemies   |
| 5    | Coral Fragment ×3   | Coral Component        | Sea enemies       |
| 6    | Demon Skull ×3      | Skull Component        | Demon enemies     |
| 7    | Dragon Talon ×3     | Talon Component        | Dragon enemies    |

### Precious Metal (Lightning Rune)

| Tier | Raw Material              | Component          | Source                |
|------|---------------------------|--------------------|-----------------------|
| 1    | Silver Ore ×3             | Silver Ingot       | Cave enemies          |
| 2    | Gold Ore ×3               | Gold Ingot         | Deep cave enemies     |
| 3    | Platinum Ore ×3           | Platinum Ingot     | Volcanic enemies      |
| 4    | Silver Ingot + Gold Ingot | Electrum Ingot     | Refine combo          |
| 5    | Orichalcum Ore ×3         | Orichalcum Ingot   | Ancient ruins enemies |

### Stone (Light Rune)

| Tier | Raw Material        | Component            | Source            |
|------|---------------------|----------------------|-------------------|
| 1    | Ragstone ×3         | Ragstone Component   | Rocky enemies     |
| 2    | Flint Stone ×3      | Flint Component      | Cave enemies      |
| 3    | Granite ×3          | Granite Component    | Mountain enemies  |
| 4    | Marble ×3           | Marble Component     | Ruins enemies     |
| 5    | Obsidian ×3         | Obsidian Component   | Volcanic enemies  |

### Void/Essence (Dark Rune)

| Tier | Raw Material        | Component            | Source            |
|------|---------------------|----------------------|-------------------|
| 1    | Dark Crystal ×3     | Dark Essence         | Shadow enemies    |
| 2    | Shadow Flask ×3     | Shadow Essence       | Undead enemies    |
| 3    | Void Shard ×3       | Void Essence         | Void enemies      |
| 4    | Abyss Fragment ×3   | Abyss Essence        | Deep void enemies |
| 5    | Nightmare Shard ×3  | Nightmare Essence    | Nightmare enemies |

---

## 5. Base Gear — Craft Recipes

Crafting base gear consumes one rune (matching the primary material), components, and gold. Base gear has no enchantment until a glyph is applied.

### Weapons

| Weapon              | Rune      | Components                           | Tier | Gold   |
|---------------------|-----------|--------------------------------------|------|--------|
| Copper Sword        | Fire      | Copper Ingot ×2, Arrowwood Lumber    | 1    | 25     |
| Bronze Dagger       | Fire      | Bronze Ingot ×1, Sheep Leather       | 2    | 50     |
| Iron Sword          | Fire      | Iron Ingot ×3, Oak Lumber            | 3    | 250    |
| Steel Axe           | Fire      | Steel Ingot ×3, Ash Lumber           | 4    | 500    |
| Mythril Spear       | Fire      | Mythril Ingot ×2, Mahogany Lumber    | 5    | 2,500  |
| Cobalt Greatsword   | Fire      | Cobalt Ingot ×4, Ebony Lumber        | 6    | 5,000  |
| Darksteel Scythe    | Fire      | Darksteel Ingot ×4, Dragon Leather   | 7    | 25,000 |
| Maple Bow           | Wind      | Maple Lumber ×3, Sheep Leather       | 2    | 50     |
| Oak Staff           | Wind      | Oak Lumber ×3, Silver Ingot          | 3    | 250    |
| Rosewood Bow        | Wind      | Rosewood Lumber ×3, Raptor Leather   | 6    | 5,000  |
| Bone Club           | Earth     | Bone Component ×3, Sheep Leather     | 1    | 25     |
| Turtle Shell Shield | Earth     | Turtle Shell Component ×3, Iron Ingot| 3    | 250    |
| Dragon Talon Knuckles| Earth    | Talon Component ×3, Dragon Leather   | 7    | 25,000 |
| Silver Dagger       | Lightning | Silver Ingot ×2, Sheep Leather       | 1    | 25     |
| Gold Rapier         | Lightning | Gold Ingot ×3, Silk Cloth            | 3    | 250    |
| Platinum Knife      | Lightning | Platinum Ingot ×2, Velvet Cloth      | 4    | 500    |
| Orichalcum Blade    | Lightning | Orichalcum Ingot ×3, Rainbow Cloth   | 5    | 2,500  |
| Obsidian Hammer     | Light     | Obsidian Component ×3, Steel Ingot   | 5    | 2,500  |
| Marble Mace         | Light     | Marble Component ×3, Iron Ingot      | 4    | 500    |
| Void Scythe         | Dark      | Void Essence ×2, Darksteel Ingot     | 6    | 5,000  |
| Nightmare Blade     | Dark      | Nightmare Essence ×3, Cobalt Ingot   | 7    | 25,000 |

### Armor

| Armor                   | Rune      | Components                           | Tier | Gold   |
|-------------------------|-----------|--------------------------------------|------|--------|
| Sheep Leather Vest      | Ice       | Sheep Leather ×2, Grass Cloth        | 1    | 25     |
| Strider Leather Helm    | Ice       | Strider Leather ×2, Bronze Ingot     | 2    | 50     |
| Tiger Leather Boots     | Ice       | Tiger Leather ×2, Iron Ingot         | 3    | 250    |
| Panzer Leather Gloves   | Ice       | Panzer Leather ×2, Steel Ingot       | 4    | 500    |
| Buffalo Leather Cuirass | Ice       | Buffalo Leather ×3, Mythril Ingot    | 5    | 2,500  |
| Dragon Leather Coat     | Ice       | Dragon Leather ×3, Darksteel Ingot   | 7    | 25,000 |
| Grass Cloth Robe        | Water     | Grass Cloth ×3, Sheep Leather        | 1    | 25     |
| Linen Cloth Cap         | Water     | Linen Cloth ×2, Bronze Ingot         | 2    | 50     |
| Cotton Cloth Pants      | Water     | Cotton Cloth ×2, Iron Ingot          | 3    | 250    |
| Wool Cloth Cape         | Water     | Wool Cloth ×3, Silver Ingot          | 4    | 500    |
| Silk Cloth Hood         | Water     | Silk Cloth ×2, Gold Ingot            | 5    | 2,500  |
| Rainbow Cloth Vestment  | Water     | Rainbow Cloth ×3, Orichalcum Ingot   | 7    | 25,000 |
| Iron Chainmail          | Fire      | Iron Ingot ×4, Sheep Leather         | 3    | 250    |
| Steel Plate Armor       | Fire      | Steel Ingot ×4, Strider Leather      | 4    | 500    |
| Mythril Chainmail       | Fire      | Mythril Ingot ×4, Panzer Leather     | 5    | 2,500  |
| Cobalt Plate Armor      | Fire      | Cobalt Ingot ×4, Buffalo Leather     | 6    | 5,000  |
| Darksteel Cuirass       | Fire      | Darksteel Ingot ×4, Dragon Leather   | 7    | 25,000 |
| Bone Helm               | Earth     | Bone Component ×3, Sheep Leather     | 1    | 25     |
| Beetle Shell Shield     | Earth     | Shell Component ×3, Bronze Ingot     | 2    | 50     |
| Coral Ring              | Earth     | Coral Component ×2, Silver Ingot     | 5    | 2,500  |
| Demon Skull Helm        | Earth     | Skull Component ×3, Steel Ingot      | 6    | 5,000  |
| Silver Earring          | Lightning | Silver Ingot ×1, Ragstone Component  | 1    | 25     |
| Gold Necklace           | Lightning | Gold Ingot ×2, Coral Component       | 3    | 250    |
| Platinum Ring           | Lightning | Platinum Ingot ×1, Marble Component  | 4    | 500    |
| Electrum Bracelet       | Lightning | Electrum Ingot ×2, Obsidian Component| 5    | 2,500  |
| Ragstone Shield         | Light     | Ragstone Component ×3, Copper Ingot  | 1    | 25     |
| Granite Gauntlets       | Light     | Granite Component ×3, Iron Ingot     | 3    | 250    |
| Obsidian Helm           | Light     | Obsidian Component ×3, Steel Ingot   | 5    | 2,500  |
| Dark Essence Ring       | Dark      | Dark Essence ×2, Silver Ingot        | 3    | 250    |
| Void Essence Amulet     | Dark      | Void Essence ×2, Gold Ingot          | 5    | 2,500  |
| Nightmare Essence Crown | Dark      | Nightmare Essence ×3, Darksteel Ingot| 7    | 25,000 |

---

## 6. Enchanting — Expanded Glyph Tables

Applying a glyph consumes the glyph, the gold cost, and overwrites any existing enchantment on the gear. The item gains a prefix and a suffix, each rolled from the glyph's pools with a numeric value inside the listed range.

**Range scaling:** The percentage range is fixed per ability. Higher-tier gear has higher base stats, so the same percentage is worth more absolute value. An Ember Iron Sword (+5% attack) gains more attack than an Ember Copper Sword (+5% attack).

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

## 7. Crafting Ranks

Each material category has its own crafting rank (1–10). Crafting components and gear at or near your rank grants rank experience. Higher ranks unlock higher-tier recipes and reduce gold costs slightly.

| Rank | Unlocks                              | Gold Discount |
|------|--------------------------------------|---------------|
| 1    | Tier 1 recipes                       | 0%            |
| 2    | Tier 1–2 recipes                     | 2%            |
| 3    | Tier 2–3 recipes                     | 4%            |
| 4    | Tier 3–4 recipes                     | 6%            |
| 5    | Tier 4–5 recipes                     | 8%            |
| 6    | Tier 5–6 recipes                     | 10%           |
| 7    | Tier 6–7 recipes                     | 12%           |
| 8    | Tier 7 recipes, +1 component yield   | 14%           |
| 9    | +1 component yield                   | 16%           |
| 10   | +2 component yield, +5% enchant range| 20%           |

**Rank experience:** Crafting an item at your current rank grants 100% XP. One rank below grants 50%. Two ranks below grants 10%. Three or more ranks below grants 0%.

**Yield bonus:** At rank 8+, refining yields one extra component per refine. At rank 10, two extra. At rank 10, enchantment numeric rolls gain +5% to their maximum range (e.g., Ember's top end becomes 7.35% instead of 7%).

---

## 8. Salvage

Break down gear into materials at any time for a flat 50 gold fee.

| Item Salvaged   | Returns                                                              |
|-----------------|----------------------------------------------------------------------|
| Base gear       | 1–2 components of its primary material + 25% chance to recover the rune |
| Enchanted gear  | 1–2 components + 25% chance to recover the rune + 10% chance to recover the glyph |

Salvage is the primary sink for unwanted enchanted gear and the primary source of recovered glyphs for re-enchanting.

---

## 9. Enemy Drop Tables

| Enemy Type               | Common Drops                  | Uncommon Drops         | Rare Drops                      |
|--------------------------|-------------------------------|------------------------|---------------------------------|
| Beasts (wolves, bears, tigers) | Hides, Bone, Gold       | Random Rune            | —                               |
| Birds                    | Feathers (cloth), Gold        | Random Rune            | —                               |
| Reptiles                 | Hides, Shells, Gold           | Random Rune            | —                               |
| Insects                  | Shells, Fibers, Gold          | Random Rune            | —                               |
| Plants                   | Fibers, Logs, Gold            | Random Rune            | —                               |
| Skeletons/Undead         | Bone, Dark Crystals, Gold     | Dark Rune              | Dark Glyph (1%)                 |
| Elementals               | Gold                          | Matching Rune (25%)    | Matching Glyph (2%)             |
| Constructs               | Stone, Metal Ore, Gold        | Random Rune            | —                               |
| Demons                   | Demon Skulls, Dark Essence, Gold | Random Rune         | Random Glyph (3%)               |
| Dragons                  | Dragon Hide, Dragon Talon, Gold | Random Rune ×2       | Random Glyph (2%)               |
| Bosses                   | Gold ×5, Rare Materials       | 2–4 Random Runes       | 1 Guaranteed Glyph + 25% chance at a second |

---

## 10. Full Example Crafts

### Example A: Tier 3 Fire Sword

1. Kill mountain enemies until you have 3 Iron Ore and a Fire Rune.
2. Refine: Fire Rune + 3 Iron Ore + 250 Gold → Iron Ingot.
3. Repeat twice more for a total of 3 Iron Ingots.
4. Kill forest enemies for Oak Logs and a Wind Rune.
5. Refine: Wind Rune + 3 Oak Logs + 250 Gold → Oak Lumber.
6. Craft: Fire Rune + 3 Iron Ingots + 1 Oak Lumber + 250 Gold → Iron Sword.
7. Kill an elemental until it drops a Fire Glyph.
8. Enchant: Iron Sword + Fire Glyph + 800 Gold → rolls prefix and suffix:
   - Prefix: Inferno — Area damage +8%
   - Suffix: of Flame — Execute damage +12%
   - Result: **Inferno Iron Sword of Flame**

### Example B: Tier 5 Water Robe

1. Farm spider enemies for Silk Thread ×9 and a Water Rune.
2. Refine ×3: Water Rune + 3 Silk Thread + 2,500 Gold → Silk Cloth (×3).
3. Farm deep cave enemies for Gold Ore ×3 and a Lightning Rune.
4. Refine: Lightning Rune + 3 Gold Ore + 2,500 Gold → Gold Ingot.
5. Craft: Water Rune + 3 Silk Cloth + 1 Gold Ingot + 2,500 Gold → Silk Cloth Hood.
6. Farm water elementals for a Water Glyph.
7. Enchant: Silk Cloth Hood + Water Glyph + 1,000 Gold → rolls:
   - Prefix: Tide — Healing +9%
   - Suffix: of Tides — Debuff duration +8%
   - Result: **Tide Silk Cloth Hood of Tides**

### Example C: Salvage Loop

1. You craft an Ember Copper Sword of Ash but want a Fire damage roll instead.
2. Salvage the sword for 50 gold → returns 1–2 Copper Ingots + 25% chance at the Fire Rune + 10% chance at the Fire Glyph.
3. Re-craft the sword and re-enchant with another Fire Glyph, hoping for Blazing and of Cinders.
4. Repeat until you get the roll you want, or trade the salvage materials to another player.

---

## 11. System Summary

| Feature          | Rule                                                              |
|------------------|-------------------------------------------------------------------|
| Runes            | Enemy drops only. 8 types. Consumed in refine and craft.          |
| Glyphs           | Enemy drops only. 8 types. No tiers. Consumed in enchant.         |
| Materials        | Enemy drops only. 8 categories × 7 tiers.                         |
| Components       | Crafted: Rune + 3 Materials + Gold.                               |
| Base Gear        | Crafted: Rune + Components + Gold.                                |
| Enchanted Gear   | Base gear + Glyph + Gold. Rolls 1 prefix + 1 suffix.              |
| Enchant Overwrite| Applying a new glyph destroys the old enchantment.                |
| Salvage          | Breaks gear into components + chance at rune/glyph.               |
| Crafting Ranks   | 1–10 per material category. Unlocks tiers, discounts gold, boosts yield. |
| Gold             | Earned from enemies, quests, salvage. Spent on every action.      |

This keeps glyph collection to eight items total while the prefix/suffix pools, numeric ranges, gear tiers, and crafting ranks create thousands of distinct enchanted items.
