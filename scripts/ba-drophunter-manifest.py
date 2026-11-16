"""Build BA female (drophunter) manifest: hair/skin/eyes; Anata peasant shirt locked."""
from __future__ import annotations

import json
import shutil
from pathlib import Path

ROOT = Path(r"C:\Users\raxac\OneDrive\Desktop\apps\black-anvil")
PUBLIC = ROOT / "character-studio/public"
SRC = PUBLIC / "character-assets/drophunter/manifest.json"
OUT = PUBLIC / "character-assets/drophunter/manifest.ba.json"
ANATA_SHIRT = PUBLIC / "loot-assets/anata/female/Chest/shirt.vrm"
SHIRT_THUMB = PUBLIC / "loot-assets/loot/thumbnails/Chest/shirt.png"
PACK_CHEST = PUBLIC / "character-assets/drophunter/chest"

# Anata female shirt.vrm — brown peasant tunic. Keep non-skirt bottoms.
LOCKED = {
    "chest": "anata_shirt",
    "legs": "doublebeltpants",
    "feet": "tallboots",
}
FALLBACKS = {
    "chest": ["anata_shirt", "tuckedshirt", "shirt", "lightshirt"],
    "legs": ["doublebeltpants", "cargopants", "jumpsuitpants", "waistshorts"],
    "feet": ["tallboots", "dressboots", "shortboots", "sneakers"],
}

CUSTOMIZABLE = {"body", "head", "eyes"}
MESH_KEEP = {"body", "head", "eyes", "chest", "legs", "feet"}

ANATA_SHIRT_TRAIT = {
    "id": "anata_shirt",
    "name": "Peasant Shirt",
    "directory": "chest/anata_shirt.vrm",
    "thumbnail": "chest/anata_shirt.png",
    "cullingLayer": 1,
}


def pick_locked(collection: list[dict], slot: str) -> str:
    ids = [c["id"] for c in collection]
    for pid in FALLBACKS.get(slot, [LOCKED.get(slot, "")]):
        if pid in ids:
            return pid
    return ids[0] if ids else ""


def ensure_anata_shirt() -> None:
    PACK_CHEST.mkdir(parents=True, exist_ok=True)
    if not ANATA_SHIRT.is_file():
        raise FileNotFoundError(ANATA_SHIRT)
    shutil.copy2(ANATA_SHIRT, PACK_CHEST / "anata_shirt.vrm")
    if SHIRT_THUMB.is_file():
        shutil.copy2(SHIRT_THUMB, PACK_CHEST / "anata_shirt.png")


def inject_shirt(data: dict) -> None:
    for trait in data["traits"]:
        if trait["trait"] != "chest":
            continue
        coll = trait.setdefault("collection", [])
        if not any(c.get("id") == "anata_shirt" for c in coll):
            coll.insert(0, dict(ANATA_SHIRT_TRAIT))
        return
    raise RuntimeError("chest trait missing from drophunter manifest")


def main() -> None:
    ensure_anata_shirt()
    data = json.loads(SRC.read_text(encoding="utf-8"))
    inject_shirt(data)

    data["animationPath"] = [
        "./loot-assets/animations/2_Idle.fbx",
        "./loot-assets/animations/3_Walking.fbx",
        "./loot-assets/animations/4_Waving.fbx",
        "./loot-assets/animations/1_T-Pose.fbx",
    ]
    data["assetsLocation"] = "./"
    data["traitsDirectory"] = "./character-assets/drophunter/"
    data["thumbnailsDirectory"] = "./character-assets/drophunter/"

    for trait in data["traits"]:
        name = trait["trait"]
        if name in LOCKED:
            LOCKED[name] = pick_locked(trait.get("collection", []), name)

    for trait in data["traits"]:
        name = trait["trait"]
        if name in LOCKED:
            locked_id = LOCKED[name]
            kept = [c for c in trait["collection"] if c["id"] == locked_id]
            if not kept and trait.get("collection"):
                kept = trait["collection"][:1]
                LOCKED[name] = kept[0]["id"]
            trait["collection"] = kept

    data["traits"] = [t for t in data["traits"] if t["trait"] in MESH_KEEP]
    data["requiredTraits"] = ["body", "eyes", "chest", "legs"]
    data["initialTraits"] = ["body", "eyes", "head", "chest", "legs", "feet"]
    data["randomTraits"] = ["body", "head", "eyes"]
    data["ba"] = {
        "gender": "female",
        "customizableTraits": sorted(CUSTOMIZABLE),
        "lockedOutfit": dict(LOCKED),
        "note": "Peasant starter: Anata shirt.vrm, plain pants, tall boots.",
    }

    OUT.write_text(json.dumps(data, indent=2), encoding="utf-8")
    SRC.write_text(json.dumps(data, indent=2), encoding="utf-8")
    print("drophunter locked:", LOCKED)


if __name__ == "__main__":
    main()
