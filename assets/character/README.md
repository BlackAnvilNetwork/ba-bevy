# Black Anvil characters

Player creation uses [Character Studio](https://github.com/M3-org/CharacterStudio) with the **neurohacker** pack ([character-assets](https://github.com/memelotsqui/character-assets)).

## Player can customize

- Hair styles + hair colors  
- Body / skin tones  
- Eyes + eye colors  
- Any blendshapes the body VRM exposes  

## Locked (not customizable)

- **Shirt** (`simpleshirt`) + **pants** (`streetpants`) + boots  
- Full loot armor/weapons live in `loot-craft-catalog.json` for crafting later  

## Install / run

```powershell
.\scripts\install-character-studio.ps1
.\scripts\install-neurohacker.ps1
python .\scripts\ba-neuro-manifest.py
.\scripts\run-character-studio.ps1
```

Forge: **Customize body & face** (or `C`) → http://127.0.0.1:5173/  
Optimize before download to atlas + merge meshes.
