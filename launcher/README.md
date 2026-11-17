# Black Anvil native launcher — integrated M1 development flow

This is the **native Tauri launcher**, not a second character editor. The
existing React/Three.js Character Studio runs inside the Tauri window as an
editable view; **Play** starts the native Bevy executable.

## Windows development quickstart

From a clean checkout on the `feat/m1-m2-integrated-player-flow` branch:

```powershell
# Terminal 1 — compile the native game
cargo build --bin black_anvil

# Terminal 2 — launcher builds/starts Character Studio automatically
cd launcher
npm ci
npm run tauri dev
```

The Tauri launch hook runs `npm run studio:dev`, which runs
`scripts/run-character-studio.ps1` in **Black Anvil mode** at
`http://127.0.0.1:5173`. No manual browser launch is needed.

1. The native app opens **Black Anvil Home**, not a separate Studio or Tauri greeting.
2. Select **Create character** (or **Customize character**).
3. Customize the existing character, then choose **Save for Black Anvil**.
4. Use the explicit **Back to launcher** button.
5. Select **Play Black Anvil**. The launcher checks the saved draft/model and
   starts `target/debug/black_anvil.exe` from the checkout, once per launcher.
6. Bevy reads the saved local draft and bypasses its *developer* forge only
   when `BA_LAUNCH_MODE=game` was supplied by native Play.

To test the **loopback zone**, start `cargo run -p ba_zone_server` in another
terminal and tick **Connect to local zone server** before Play. Zone protocol
v4 sends sequenced movement **intents**, not client-authoritative positions.

## Testing

```powershell
cargo check -p ba_protocol -p ba_combat -p ba_zone_server -p black_anvil
cargo test -p ba_zone_server
cd scripts
npm ci
node smoke-zone-realtime.mjs   # requires the zone server already running
```

The Studio build for distribution is still handled by
`npm run studio:build` and `npm run tauri build`. **Do not distribute
that bundle as a finished game installer yet:** it currently contains
the launcher/Studio only, not a verified Bevy game package or patcher.

## Authority and explicit operations

- `Create/Customize` opens an **in-app** editor, not a second website.
- `Save for Black Anvil` explicitly writes a local draft and GLB through
  Tauri IPC. This uses the existing dev `active.*` adapter until the
  account-backed character service and immutable content hashes land.
- `Play` explicitly spawns Bevy. No automatic game entry after Save.
- `BA_LAUNCH_MODE=game` is an explicit *local development* handoff. It is
  **not** an authentication credential. Standalone `cargo run` retains
  the existing forge.
- `MoveIntent` communicates movement controls, not a position.
  Zone movement runs on the server's fixed simulation tick; clients render
  prediction plus snapshot correction. Full input-history replay is not yet
  implemented.

## Production gates that remain open

- Discord OAuth and backend-issued one-time Bevy session tickets (#14, #15).
- Account-scoped cloud characters; safe avatar distribution and rigged
  animation to other players (#7, #8, #10).
- Verified executable/assets, signed patches, self-updates and reinstall-safe
  identity (#5, #9).
- Server-owned XP, AI, collision/navigation, persistence, reconnect replay,
  full prediction reconciliation, and scalable spatial indexing (#10–#13).
- Discord local/party voice and text (#33, #34).
- Windows GUI validation and two *separate-machine* end-to-end tests.

**Do not close M1/M2 based on a log-only or tower-GLB smoke test.**
