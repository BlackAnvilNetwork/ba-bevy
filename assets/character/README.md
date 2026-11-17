# Black Anvil characters

Canonical draft format: [`schema/ba-character-v1.schema.json`](schema/ba-character-v1.schema.json).

Player creation uses Character Studio (vendored in `character-studio/`) with **Anata** male/female packs under `loot-assets`.

## Pipeline

1. Run Studio: `.\scripts\run-character-studio.ps1` (`VITE_BA_MODE=1`)
2. Customize → Save → **Save for Black Anvil**
3. Bridge writes `exports/active.json` + `exports/active.glb` (gitignored)
4. `cargo run` — forge reads the draft; Bevy spawns the GLB on Enter

## Body vs equipment

| Kind | Trait groups | Authority |
|------|----------------|-----------|
| Body | Body, Head, Eyes, Basic Colors | Immutable after finalize (M3) |
| Equipment | Chest, Legs, Shoes, Hands, … | Starter now; server inventory later |

## Optional packs

Neurohacker / drophunter still install into `character-studio/public/character-assets/` (gitignored):

```powershell
.\scripts\install-neurohacker.ps1
.\scripts\install-drophunter.ps1
```
