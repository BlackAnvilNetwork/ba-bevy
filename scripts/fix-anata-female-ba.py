"""Fix Anata female manifest for BA gender switch (paths + locked starter gear)."""
from __future__ import annotations

import json
from pathlib import Path

ROOT = Path(r"C:\Users\raxac\OneDrive\Desktop\apps\black-anvil")
PATH = ROOT / "character-studio/public/loot-assets/anata/female/manifest.json"

LOCKED = {
    "Chest": "shirt",
    "Hands": "leather_gloves",
    "Shoes": "leather_boots",
    "Waist": "leather_belt",
}
HIDDEN_EMPTY = {"Neck", "Weapon"}


def main() -> None:
    data = json.loads(PATH.read_text(encoding="utf-8"))

    data["assetsLocation"] = "./loot-assets/"
    data["traitsDirectory"] = "anata/female/"
    data["thumbnailsDirectory"] = "loot/thumbnails/"
    data["traitIconsDirectorySvg"] = "loot/icons/"
    data["animationPath"] = [
        "./loot-assets/animations/1_T-Pose.fbx",
        "./loot-assets/animations/2_Idle.fbx",
        "./loot-assets/animations/3_Walking.fbx",
        "./loot-assets/animations/4_Waving.fbx",
    ]

    new_traits = []
    for trait in data["traits"]:
        name = trait["trait"]
        if name in HIDDEN_EMPTY:
            continue
        if name in LOCKED:
            lid = LOCKED[name]
            kept = [c for c in trait.get("collection", []) if c.get("id") == lid]
            if not kept and trait.get("collection"):
                kept = trait["collection"][:1]
            trait["collection"] = kept
        new_traits.append(trait)

    data["traits"] = new_traits
    data["requiredTraits"] = ["Body"]
    data["initialTraits"] = ["Body", "Chest", "Hands", "Shoes", "Waist"]
    data["randomTraits"] = ["Body", "Head"]
    data["ba"] = {
        "gender": "female",
        "customizableTraits": ["Body", "Head"],
        "lockedOutfit": LOCKED,
        "note": "Anata female body. Headgear customizable; starter shirt/gloves/boots/belt locked.",
    }

    PATH.write_text(json.dumps(data, indent=2), encoding="utf-8")
    print("Wrote", PATH)
    print("Traits:", [t["trait"] for t in data["traits"]])
    for t in data["traits"]:
        ids = [c["id"] for c in t.get("collection", [])]
        print(f"  {t['trait']}: {ids}")


if __name__ == "__main__":
    main()
