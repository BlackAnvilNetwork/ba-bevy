# Black Anvil (Bevy)

Opening vertical slice + Character Studio + **dev/loopback zone prototype**.

> **Not M1/M2 product sign-off.** This branch is experimental scaffolding: Discord-authenticated launcher, durable per-account persistence, signed remote meshes, and full online authority/reconciliation remain open (see PR review / milestone issues).

## Run (single-player)

```bash
cargo run --bin black_anvil
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
cargo run --bin black_anvil
```

| Input | Zone action |
|-------|-------------|
| LMB / F | Engage nearest — server auto-attacks (~0.72s swing) |
| 1–8 | Cast kits.json hotbar (buffs / WS / heals; server math) |
| WASD / Ctrl | Move / sprint |
| Shift or Space | Roll in the movement direction. Costs 35 stamina and protects the first 0.25 seconds |
| E | Speak, gather, collect a ground bundle, or use the ward stone |
| Q / I / L | Use healing supplies / open the pack / toggle common drop labels |
| V or F5 | Switch first- and third-person view |

Combat math: `crates/ba_combat` + `assets/combat/kits.json`. Protocol v5: `crates/ba_protocol` (`Damage` / `Heal` / `BuffApplied` / `CastRejected`).
Remotes render as capsules until a signed content-hash asset cache exists (never load network filesystem paths).  
Autos grant TP; `ws_*` casts spend it. Kits buffs raise server `atk_mul`. Local boss/XP sim is disabled while zone Welcome is verified.

### The Stillwater Boneyard

New online characters arrive at the Lantern Post. Speak to Old Pell, hunt in the Shallows, and gather iron and peat from the banks. Follow the great ribs inland to the Drowned Ox. At the northern arch, defeat Edda and collect **Edda’s Book** from her fallen bundle. The Drowned Gate admits a level-three character carrying the book, the Ox’s Yoke, and three Cold Iron. The ferryman brings you to Cael’s field forge at Sable Rest. Four Cold Iron and one Peat make the first iron edge.

In zone multiplayer, press **E** at the twin blue lanterns to travel. Greymere’s east road connects to the waystone south of Pell at the Lantern Post. The Drowned Gate connects to Sable Rest, where the return ferry waits south of Cael’s forge. Once a character opens the Drowned Gate, they can cross again after spending their iron.

White, gold, and red wind-ups mark normal, heavy, and unblockable attacks. Step out of their footprint or roll through the impact. Watch for the collapse, then collect the bundle with **E**. Rare components cast a blue beam. Each contributor has their own bundle. The authenticated gateway saves unclaimed bundles with the character.

#### Quixel Megascans kit

Stillwater uses locally installed glTF scans at the marsh banks and rock outcrops. Surface sets can also supply the bank and gate masonry. A checkout without scans uses the built-in geometry and materials. Install Pillow for the importer and restart the client after importing.

```powershell
py -m pip install Pillow
py scripts/import-megascans.py terrain "$HOME/Downloads/forest_terrain_tletcalfa_ue_mid.zip"
```

The `terrain` and `rock` slots accept a single model in a glTF ZIP, an extracted folder, or a GLB. The `bank` and `gate` slots take one resolution of a texture set with base color, normal and roughness maps. The importer caps textures at 2K and packs roughness into the green channel. Keep original exports and the generated `assets/megascans/` directory with the private project assets. [Quixel library](https://quixel.com/news/update-megascans-are-going-free-again) · [Fab license](https://www.fab.com/eula)

### Greymere Ashworks (zone playtest)

Set `$env:BA_ZONE_START = "greymere"` in the server terminal before launching to start a fresh playtest here. Existing saved Greymere characters can travel to Stillwater by using E at the eastern road. The forge order appears in the upper right. Engage the **Clinker Imp** near the well with F or left click. Use your job hotbar and step beyond its warning ring before the kiln pulse lands. Players who damage the construct and are alive nearby when it breaks earn XP and can each collect a bundle of raw cinder with E.

* **Kiln ward** — bring two raw cinder to the fire basket beside the well and press G. The equipped ward reduces incoming kiln pulse damage to one third (rounded down).
* **Tempered edge** — follow the east road under the firing gallery, defeat the Kiln Warden, and return to the basket with one kiln heart and two raw cinder. Press G to temper the edge for 25% more auto-attack power.

Both constructs rekindle after six seconds. A defeated player returns to the yard entrance. Character XP, level, materials, and crafted upgrades use the gateway's existing character save when playing with authenticated zone admission. The unauthenticated loopback playtest starts fresh on reconnect.

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

## License (AGPL-3.0)

**Black Anvil, Copyright (C) 2026 Black Anvil Network.** Licensed under the GNU Affero General Public License, version 3 or (at your option) any later version (`AGPL-3.0-or-later`). See [LICENSE](LICENSE).

Borrowed third-party material is **not** AGPL:

- `character-studio/` stays MIT (Atlas Foundation). See `character-studio/LICENSE`.
- `assets/kits/` stays Kenney CC0. See each kit's `License.txt`.

*This is a plain-language summary, not legal advice. The LICENSE text governs.*

- **Network use counts.** If you run a modified version of the zone server, gateway, or client for users over a network, you must offer those users the complete corresponding source of your version (AGPL §13).
- **Distributing the client counts.** Shipping the Bevy client, launcher, or any binary means offering the complete corresponding source, including Rust sources, build scripts and lockfiles (AGPL §6).
- **Keep the notices.** Don't remove copyright lines, license notices or warranty disclaimers.
- **Derivatives stay AGPL.** Modified versions and works based on this code must be licensed under AGPL-3.0-or-later.
- **Mark modified files** with a prominent notice saying what you changed and when (AGPL §5a).
- **No extra restrictions** beyond the license, and contributors grant the patent license in AGPL §11.
