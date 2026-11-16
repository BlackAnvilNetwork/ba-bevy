"""Lock Anata male/female to simple medieval peasant starters (for when those packs are used)."""
from __future__ import annotations

import json
from pathlib import Path

ROOT = Path(r"C:\Users\raxac\OneDrive\Desktop\apps\black-anvil\character-studio\public\loot-assets\anata")

LOCKED = {
    "Chest": "shirt",
    "Hands": "linen_gloves",
    "Shoes": "linen_shoes",
    "Waist": "linen_sash",
}
FALLBACKS = {
    "Chest": ["shirt", "linen_robe", "robe"],
    "Hands": ["linen_gloves", "wool_gloves", "gloves", "leather_gloves"],
    "Shoes": ["linen_shoes", "wool_shoes", "shoes", "leather_boots"],
    "Waist": ["linen_sash", "wool_sash", "sash", "leather_belt"],
}
# No Head by default — peasant without helm/hood.
HIDDEN = {"Neck", "Weapon"}


def pick(collection: list[dict], slot: str) -> str:
    ids = [c["id"] for c in collection]
    for pid in FALLBACKS.get(slot, [LOCKED.get(slot, "")]):
        if pid in ids:
            return pid
    return ids[0] if ids else ""


def fix(gender: str) -> None:
    path = ROOT / gender / "manifest.json"
    data = json.loads(path.read_text(encoding="utf-8"))
    data["assetsLocation"] = "./loot-assets/"
    data["traitsDirectory"] = f"anata/{gender}/"
    data["thumbnailsDirectory"] = "loot/thumbnails/"
    data["traitIconsDirectorySvg"] = "loot/icons/"
    data["animationPath"] = [
        "./loot-assets/animations/1_T-Pose.fbx",
        "./loot-assets/animations/2_Idle.fbx",
        "./loot-assets/animations/3_Walking.fbx",
        "./loot-assets/animations/4_Waving.fbx",
    ]

    locked = dict(LOCKED)
    new_traits = []
    for trait in data["traits"]:
        name = trait["trait"]
        if name in HIDDEN:
            continue
        if name in locked:
            locked[name] = pick(trait.get("collection", []), name)
            kept = [c for c in trait.get("collection", []) if c.get("id") == locked[name]]
            if not kept and trait.get("collection"):
                kept = trait["collection"][:1]
                locked[name] = kept[0]["id"]
            trait["collection"] = kept
        new_traits.append(trait)

    data["traits"] = new_traits
    data["requiredTraits"] = ["Body"]
    data["initialTraits"] = ["Body", "Chest", "Hands", "Shoes", "Waist"]
    data["randomTraits"] = ["Body"]
    data["ba"] = {
        "gender": gender,
        "lockedOutfit": locked,
        "note": "Medieval peasant starter: shirt, linen gloves/shoes/sash. No helm.",
    }
    path.write_text(json.dumps(data, indent=2), encoding="utf-8")
    print(gender, locked)


def main() -> None:
    fix("male")
    fix("female")


if __name__ == "__main__":
    main()
