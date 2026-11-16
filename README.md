# Black Anvil (Bevy)

Opening vertical slice + Character Studio + zone multiplayer MVP.

## Run (single-player)

```bash
cargo run
```

Forge: customize opens Character Studio (`scripts/run-character-studio.ps1`).  
**Save for Black Anvil** writes `assets/character/exports/active.{json,glb}`; Enter loads that avatar.

## Zone multiplayer (M2)

```powershell
# terminal 1
.\scripts\run-zone-server.ps1

# terminal 2 / 3
$env:BA_ZONE = "1"
cargo run
```

| Input | Zone action |
|-------|-------------|
| LMB / F | Engage nearest — server auto-attacks (~0.72s swing) |
| 1–8 | Cast kits.json hotbar (buffs / WS / heals; server math) |
| WASD | Move (server clamps speed / bounds) |

Combat math: `crates/ba_combat` mirrors `src/rpg/fight.rs` and loads `assets/combat/kits.json` for hotbar / buffs / WS. Protocol: `crates/ba_protocol`.  
Remotes spawn the Studio GLB from `appearance.glbRelativePath` when that asset exists locally; otherwise a capsule.  
Autos grant TP; `ws_*` casts spend it. Kits buffs (e.g. berserk) raise server `atk_mul` for their duration.

## Launcher (Studio host)

```powershell
.\scripts\run-character-studio.ps1   # :5173 (dev)
.\scripts\build-launcher-studio.ps1  # character-studio/build for Tauri
cd launcher; npm install; npm run tauri dev   # or tauri build
```

IPC: `save_draft` / `get_draft` — same BaCharacterV1 → `assets/character/exports/active.*`.

## Pipeline gates (M1 / M2)

| Gate | Check |
|------|--------|
| M1 Studio→Bevy | `.\scripts\smoke-character-pipeline.ps1` |
| M2 zone combat | `.\scripts\run-zone-server.ps1` + `node scripts/smoke-zone-realtime.mjs` |

## Controls (yard)

| Input | Action |
|-------|--------|
| Left click | Capture mouse / engage (if zoned) |
| Esc | Release mouse |
| WASD | Move |
| Shift | Sprint |
| E | Talk / advance monologue |

## Slice contents

- Greymere Yard dusk opening, Village Head, forge jobs
- Anata Character Studio BA mode + BaCharacterV1 drafts
- Authoritative zone presence + realtime melee / hotbar casts
