# Black Anvil (Bevy)

Opening vertical slice ported from the Unreal `ba-unreal` prototype.

## Run

```bash
cargo run
```

## Controls

| Input | Action |
|-------|--------|
| Left click | Capture mouse |
| Esc | Release mouse |
| WASD | Move |
| Shift | Sprint |
| E | Talk / advance monologue |

## Data

JSON under `assets/data/` (copied from `ba-unreal` `BlackAnvilData`):

- World / Zones / Scenes / NPCs / Cinematics

## Slice contents

- Dusk ruins yard: ground, well, broken wall, cemetery, trees
- Village Head NPC near the well
- Press-E monologue from `Cutscene_OldManAtTheWell.json`
- Ash burst + despawn on final line
