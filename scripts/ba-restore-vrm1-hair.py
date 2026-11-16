"""BA male/female on neurohacker/drophunter (VRM1 hair works).

Medieval Anata chest gear is copied in; Character Studio rebinds VRM0 clothing
onto the Body skeleton at runtime so shirt + hair both work.
"""
from __future__ import annotations

import json
import shutil
from pathlib import Path

ROOT = Path(r"C:\Users\raxac\OneDrive\Desktop\apps\black-anvil")
PUBLIC = ROOT / "character-studio/public"
SELECT = PUBLIC / "manifest.json"
ANATA = PUBLIC / "loot-assets/anata"
SHIRT_THUMB = PUBLIC / "loot-assets/loot/thumbnails/Chest"

PACKS = {
    "male": {
        "dir": PUBLIC / "character-assets/neurohacker",
        "anata_gender": "male",
        "portrait": "./character-assets/neurohacker/neurohacker_icon.png",
        "desc": "Neurohacker — hair/skin/eyes + medieval Anata gear (runtime-rebound).",
    },
    "female": {
        "dir": PUBLIC / "character-assets/drophunter",
        "anata_gender": "female",
        "portrait": "./character-assets/drophunter/drophunter_icon.png",
        "desc": "Drophunter — hair/skin/eyes + medieval Anata gear (runtime-rebound).",
    },
}

# Starter + craftable medieval chests from Anata loot.
MEDIEVAL_CHEST = [
    "shirt",
    "linen_robe",
    "robe",
    "silk_robe",
    "hard_leather_armor",
    "leather_armor",
    "chain_mail",
    "ring_mail",
]

DROP_FROM_LOADOUT = {"legs", "outer", "accessories", "feet", "foot"}


def install_medieval_chest(pack_dir: Path, anata_gender: str) -> list[dict]:
    src_dir = ANATA / anata_gender / "Chest"
    dest = pack_dir / "chest"
    dest.mkdir(parents=True, exist_ok=True)
    items: list[dict] = []

    for stem in MEDIEVAL_CHEST:
        src = src_dir / f"{stem}.vrm"
        if not src.is_file():
            continue
        out_id = f"anata_{stem}"
        shutil.copy2(src, dest / f"{out_id}.vrm")
        thumb_src = SHIRT_THUMB / f"{stem}.png"
        thumb_name = f"{out_id}.png"
        if thumb_src.is_file():
            shutil.copy2(thumb_src, dest / thumb_name)
        else:
            thumb_name = f"{out_id}.vrm"
        entry = {
            "id": out_id,
            "name": stem.replace("_", " ").title(),
            "directory": f"chest/{out_id}.vrm",
            "thumbnail": f"chest/{thumb_name}",
            "cullingLayer": 1,
        }
        # Non-starter gear stays locked until craft/forge unlocks it (keeps random loadout = shirt).
        if stem != "shirt":
            entry["locked"] = True
            entry["purchasable"] = True
            entry["price"] = 1
        items.append(entry)

    if not items:
        raise RuntimeError(f"No medieval chest VRMs found in {src_dir}")
    return items


def rebuild(gender: str) -> None:
    cfg = PACKS[gender]
    pack_dir: Path = cfg["dir"]
    path = pack_dir / "manifest.json"
    data = json.loads(path.read_text(encoding="utf-8"))

    chest_items = install_medieval_chest(pack_dir, cfg["anata_gender"])
    # Prefer peasant shirt as the locked starter.
    starter = next((c for c in chest_items if c["id"] == "anata_shirt"), chest_items[0])

    data["assetsLocation"] = "./"
    data["traitsDirectory"] = f"./character-assets/{pack_dir.name}/"
    data["thumbnailsDirectory"] = f"./character-assets/{pack_dir.name}/"
    data["animationPath"] = [
        "./loot-assets/animations/2_Idle.fbx",
        "./loot-assets/animations/3_Walking.fbx",
        "./loot-assets/animations/4_Waving.fbx",
        "./loot-assets/animations/1_T-Pose.fbx",
    ]

    locked = {"chest": starter["id"]}
    new_traits = []
    for trait in data["traits"]:
        name = trait["trait"]
        if name in DROP_FROM_LOADOUT:
            continue
        if name == "chest":
            # Full medieval catalog available; starter selection via initialTraits pick.
            trait["collection"] = chest_items
            trait["name"] = "chest"
        new_traits.append(trait)

    data["traits"] = new_traits
    data["requiredTraits"] = ["body", "eyes", "chest"]
    data["initialTraits"] = ["body", "eyes", "head", "chest"]
    data["randomTraits"] = ["body", "head", "eyes"]
    # Force starter shirt id into lockedOutfit for BA UI.
    data["ba"] = {
        "gender": gender,
        "customizableTraits": ["body", "eyes", "head"],
        "lockedOutfit": locked,
        "starterChest": starter["id"],
        "note": "VRM1 body+hair. Anata medieval chest rebinds onto Body at runtime.",
    }

    # Ensure initial chest pick is the shirt when Character Studio randomizes.
    for trait in data["traits"]:
        if trait["trait"] != "chest":
            continue
        # Put starter first so getRandomTraits / first-option loaders hit shirt.
        rest = [c for c in trait["collection"] if c["id"] != starter["id"]]
        trait["collection"] = [starter, *rest]

    text = json.dumps(data, indent=2)
    path.write_text(text, encoding="utf-8")
    (pack_dir / "manifest.ba.json").write_text(text, encoding="utf-8")
    print(gender, "chest", [c["id"] for c in chest_items], "starter", starter["id"])


def write_select() -> None:
    select = {
        "characters": [
            {
                "name": "Male",
                "description": PACKS["male"]["desc"],
                "portrait": PACKS["male"]["portrait"],
                "manifest": "./character-assets/neurohacker/manifest.json",
                "format": "vrm",
                "baGender": "male",
            },
            {
                "name": "Female",
                "description": PACKS["female"]["desc"],
                "portrait": PACKS["female"]["portrait"],
                "manifest": "./character-assets/drophunter/manifest.json",
                "format": "vrm",
                "baGender": "female",
            },
        ],
        "loras": [],
        "sprites": [],
        "thumbnails": [],
        "defaultAnimations": [
            {
                "name": "Idle",
                "description": "Idle",
                "location": "./loot-assets/animations/2_Idle.fbx",
                "icon": "|",
            },
            {
                "name": "Walking",
                "description": "Walk",
                "location": "./loot-assets/animations/3_Walking.fbx",
                "icon": "|",
            },
        ],
    }
    SELECT.write_text(json.dumps(select, indent=2), encoding="utf-8")


def main() -> None:
    rebuild("male")
    rebuild("female")
    write_select()
    # Scale Anata VRM0 chests to neuro/drop bind pose (edits the pack VRM files).
    import subprocess
    import sys

    conv = ROOT / "scripts/ba-convert-anata-shirt.py"
    subprocess.check_call([sys.executable, str(conv)])
    print("wrote", SELECT)


if __name__ == "__main__":
    main()
