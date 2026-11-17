# Black Anvil (Bevy)

Opening vertical slice + Character Studio + **dev/loopback zone prototype**.

> **Not M1/M2 product sign-off.** This branch is experimental scaffolding: Discord-authenticated launcher, durable per-account persistence, signed remote meshes, and full online authority/reconciliation remain open (see PR review / milestone issues).

## Run (single-player)

```bash
cargo run
```

Forge: customize opens Character Studio (`scripts/run-character-studio.ps1`, bound to `127.0.0.1`).  
**Save for Black Anvil** validates BaCharacterV1 + GLB magic/size, writes character-scoped files, and updates the local `active.*` Bevy pointer (dev only).

## Zone multiplayer (dev prototype)

Default bind is **loopback only** (`127.0.0.1:8787`). Non-local binds require `BA_ZONE_ALLOW_NONLOCAL=1` (unsafe).

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
| WASD | Move (client prediction; server clamps speed / NaNs / bounds) |

Combat math: `crates/ba_combat` + `assets/combat/kits.json`. Protocol v3: `crates/ba_protocol` (`Damage` / `Heal` / `BuffApplied` / `CastRejected`).  
Remotes render as capsules until a signed content-hash asset cache exists (never load network filesystem paths).  
Autos grant TP; `ws_*` casts spend it. Kits buffs raise server `atk_mul`. Local boss/XP sim is disabled while zone Welcome is verified.

## Launcher (Studio-host prototype)

```powershell
.\scripts\run-character-studio.ps1   # :5173 (dev, loopback)
.\scripts\build-launcher-studio.ps1  # character-studio/build for Tauri host
cd launcher; npm install; npm run tauri dev
```

IPC: validated `save_draft` / `get_draft`. Full Home / Discord login / Play→Bevy bootstrap is still outstanding.

## Prototype checks

```powershell
.\scripts\smoke-character-pipeline.ps1
# zone up:
cd scripts; npm install; node smoke-zone-realtime.mjs
cargo test -p ba_zone_server
```

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
- Anata Character Studio BA mode + BaCharacterV1 drafts (colors/blendshapes captured when available)
- Loopback zone presence + realtime melee / hotbar casts (prototype)
