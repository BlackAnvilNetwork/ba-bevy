# Black Anvil Launcher

Tauri shell that hosts Character Studio and writes `BaCharacterV1` drafts for Bevy.

## Dev

1. Start Studio: `..\scripts\run-character-studio.ps1`
2. In this folder: `npm install` then `npm run tauri dev`

`Save for Black Anvil` uses Tauri `save_draft` when `__TAURI__` is present; otherwise the Vite `/ba/save-draft` bridge.

Drafts land at `assets/character/exports/active.json` + `active.glb` for `cargo run` in the repo root.
