# Black Anvil launcher + Character Studio — development integration

The Tauri window is Black Anvil's native entry point. It embeds the **existing**
React/Three.js Character Studio without rebuilding it; **Play** starts the
native Bevy game with a *selected, versioned, hash-checked character*.

**This is a development integration, not a finished Discord-authenticated MMO.**

## Windows developer quickstart

```powershell
# Terminal 1: build Bevy (the game is not yet bundled in the installer)
cargo build --bin black_anvil

# Terminal 2: start the integrated native launcher
cd launcher
npm ci
npm run tauri dev
```

The Tauri dev hook runs `scripts/run-character-studio.ps1` automatically in
Black Anvil mode. The editor appears **inside the native window**, not in an
external browser.

1. Native launch opens the **Black Anvil Home** screen.
2. Select **Create character** or **Customize character**.
3. Customize appearance in the existing Studio, then **Save for Black Anvil**.
4. Select **Back to launcher**, then **Play Black Anvil**.
5. Bevy receives an explicit `BA_LAUNCH_MODE=game` handoff, loads that exact
   selected revision, and skips its redundant *developer* forge menu.

Launching `cargo run --bin black_anvil` directly still uses the developer
forge unless the launcher-specific environment is supplied.

## Versioned local character data

The Tauri save command validates the schema, ID, job, model format/size and GLB
header, computes SHA-256, then publishes immutable revision files:

```text
%LOCALAPPDATA%/BlackAnvil/characters/
  selected.json                   # one atomic selection pointer
  <character-id>/
    revisions/
      r<revision>/
        character.json
        character.glb
  cache/
    <sha256-of-glb>.glb            # same-host dev avatar cache
```

A failed revision write cannot leave mismatched active JSON/GLB. A save retry
for an identical revision is idempotent; a conflicting revision is rejected.
The character's model is checked against its content hash before native Play.

Bevy has a **separate named** `character://` asset reader, pointing at
`BA_CHARACTER_ROOT`; it never needs to overwrite `assets/` inside the game.
The launcher sets `BA_CHARACTER_DRAFT_PATH` and
`BA_CHARACTER_GLB_PATH` for the selected revision. These are development
configuration values, **not authentication or ownership credentials**.

### Same-host multiplayer avatar preview

A server snapshot contains a character identifier and a **content hash**, not
an untrusted file path. When a matching SHA-256 GLB exists in the local
launcher cache, the receiving Bevy client can display that avatar. Otherwise
it displays a capsule placeholder.

**Cross-machine GLB delivery is not implemented.** This same-host path is
deliberately restricted to a local prototype until a backend issues
authenticated, approved asset downloads and re-validates glTF resource budgets.

## Local zone server

```powershell
cargo run -p ba_zone_server
# in another terminal:
cd scripts
npm ci
node smoke-zone-realtime.mjs
```

Select **Connect to local zone server** in launcher Home before Play.
The zone **cannot bind to non-loopback addresses** until Discord-backed
session-ticket admission is implemented.

Shared protocol v4 sends `MoveIntent` (forward/strafe/yaw/sprint + sequence),
**never client positions**. The server advances on its own 20 Hz tick,
acknowledges the last processed input, and sends per-viewer snapshots. Bevy
predicts with the same movement kernel, replays unacknowledged inputs, and
interpolates local/remote display positions.

Greymere spawn and preview movement bounds are matched to
`assets/data/Zones/Zone_GreymereYard.json` and protected by a regression test.
A proper zone-descriptor protocol, collision/navigation, spatial index,
server persistence and real client load/reconnect tests are still required.

## Automated checks

```powershell
cargo check -p ba_protocol -p ba_combat -p ba_zone_server -p black_anvil
cargo test -p ba_protocol -p ba_zone_server
cargo test --manifest-path launcher/src-tauri/Cargo.toml
cd launcher
npm run studio:build
```

GitHub Actions runs these on Windows. Green CI does **not** prove the GUI,
animation rigs, correct skinning or cross-machine gameplay.

## Gates still open (do not mark milestones completed)

- Discord OAuth and backend account/session issuance, one-time launcher→Bevy
  tickets, recovery and Discord party/local voice and text (#14, #15, #33, #34).
- Signed native game + assets bundle, patch/repair/rollback, Windows GUI
  smoke and asset redistribution license review (#5, #9, #36).
- Authenticated character service and cross-machine approved asset delivery;
  independent animated characters on two computers (#7, #8, #10).
- Full server-owned movement collision, combat/XP/drop rules, persistence,
  safe reconnect, input sequence rollover and larger-zone indexing (#10–#13).
- End-to-end graphics and performance validation (M1/M2), with screenshots
  of two *different* Studio avatars. A tower GLB log-only test is not sufficient.

The product direction remains **Discord login → customize → Play** with no
mandatory wallet prompts or implicit cross-component workflow transitions.
