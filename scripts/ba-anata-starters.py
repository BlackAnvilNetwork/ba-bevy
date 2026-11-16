"""BA Anata male/female: peasant shirt.vrm, no pants/sash clutter, local hair copies."""
from __future__ import annotations

import json
import shutil
from pathlib import Path

ROOT = Path(r"C:\Users\raxac\OneDrive\Desktop\apps\black-anvil")
PUBLIC = ROOT / "character-studio/public"
ANATA = PUBLIC / "loot-assets/anata"
CHAR = PUBLIC / "character-assets"
SELECT = PUBLIC / "manifest.json"

# Only lock gear that reads as simple peasant — no sash/belt props, no gloves.
PEASANT = {
    "Chest": ("shirt", ["shirt", "linen_robe", "robe"]),
    "Shoes": ("linen_shoes", ["linen_shoes", "wool_shoes", "shoes", "leather_boots"]),
}

# Hats, belts, weapons — not part of BA starter / hair UI.
DROP_TRAITS = {"Neck", "Weapon", "Waist", "Hands"}

HAIR_SRC = {
    "male": CHAR / "neurohacker" / "head",
    "female": CHAR / "drophunter" / "head",
}


def pick(collection: list[dict], prefer: list[str]) -> str:
    ids = [c["id"] for c in collection]
    for pid in prefer:
        if pid in ids:
            return pid
    return ids[0] if ids else ""


def basic_colors() -> dict | None:
    for pack in ("neurohacker", "drophunter"):
        path = CHAR / pack / "manifest.json"
        if not path.is_file():
            continue
        data = json.loads(path.read_text(encoding="utf-8"))
        for group in data.get("colorCollections") or []:
            if group.get("trait") == "Basic Colors":
                return group
        if data.get("colorCollections"):
            return data["colorCollections"][0]
    return None


def install_hair(gender: str) -> list[dict]:
    """Copy anime hair VRMs into anata/{gender}/Head so paths stay local."""
    src = HAIR_SRC[gender]
    dest = ANATA / gender / "Head"
    dest.mkdir(parents=True, exist_ok=True)

    # Clear old hat VRMs from Head so only hair remains on disk for this BA pack.
    # Keep copies under Head/_hats_backup once.
    backup = dest / "_hats_backup"
    for old in list(dest.glob("*.vrm")) + list(dest.glob("*.png")):
        if old.name.startswith("_"):
            continue
        # Don't backup hair we just manage — only move non-hair loot hats once.
        if old.stem in {p.stem for p in src.glob("*.vrm")}:
            continue
        backup.mkdir(exist_ok=True)
        target = backup / old.name
        if not target.exists():
            shutil.move(str(old), str(target))

    items = []
    for vrm in sorted(src.glob("*.vrm")):
        stem = vrm.stem
        shutil.copy2(vrm, dest / f"{stem}.vrm")
        thumb_src = src / f"{stem}.png"
        thumb_name = f"{stem}.png"
        if thumb_src.is_file():
            shutil.copy2(thumb_src, dest / thumb_name)
        else:
            thumb_name = f"{stem}.vrm"
        items.append(
            {
                "id": stem,
                "name": stem.replace("_", " ").title(),
                "directory": f"Head/{stem}.vrm",
                "thumbnail": f"Head/{thumb_name}",
                "cullingLayer": 1,
                "colorCollection": "Basic Colors",
            }
        )
    return items


def build(gender: str) -> None:
    path = ANATA / gender / "manifest.json"
    data = json.loads(path.read_text(encoding="utf-8"))

    data["assetsLocation"] = "./loot-assets/"
    data["traitsDirectory"] = f"anata/{gender}/"
    data["thumbnailsDirectory"] = f"anata/{gender}/"
    data["traitIconsDirectorySvg"] = "loot/icons/"
    data["animationPath"] = [
        "./loot-assets/animations/2_Idle.fbx",
        "./loot-assets/animations/3_Walking.fbx",
        "./loot-assets/animations/4_Waving.fbx",
        "./loot-assets/animations/1_T-Pose.fbx",
    ]

    hair_items = install_hair(gender)
    locked: dict[str, str] = {}
    new_traits = []

    for trait in data["traits"]:
        name = trait["trait"]
        if name in DROP_TRAITS:
            continue
        if name == "Head":
            trait["name"] = "hair"
            trait["collection"] = hair_items
            new_traits.append(trait)
            continue
        if name in PEASANT:
            prefer = PEASANT[name][1]
            locked_id = pick(trait.get("collection", []), prefer)
            locked[name] = locked_id
            kept = [c for c in trait.get("collection", []) if c.get("id") == locked_id]
            if not kept and trait.get("collection"):
                kept = trait["collection"][:1]
                locked[name] = kept[0]["id"]
            trait["collection"] = kept
        new_traits.append(trait)

    data["traits"] = new_traits

    colors = basic_colors()
    if colors:
        data["colorCollections"] = [colors]

    # Body + shirt + hair + shoes only — no waist sash, gloves, or pants base layer.
    data["requiredTraits"] = ["Body", "Chest"]
    data["initialTraits"] = ["Body", "Chest", "Head", "Shoes"]
    data["randomTraits"] = ["Head"]
    data["ba"] = {
        "gender": gender,
        "lockedOutfit": locked,
        "customizableTraits": ["Body", "Head"],
        "note": "Anata shirt.vrm + local hair copies. No pants/sash/gloves clutter.",
    }

    text = json.dumps(data, indent=2)
    (ANATA / gender / "manifest.ba.json").write_text(text, encoding="utf-8")
    path.write_text(text, encoding="utf-8")
    print(gender, "locked", locked, "hair", len(hair_items))


def write_select() -> None:
    select = {
        "characters": [
            {
                "name": "Male",
                "description": "Anata male — peasant shirt, hair customizable.",
                "portrait": "./loot-assets/anata/male/Body/male.png",
                "manifest": "./loot-assets/anata/male/manifest.json",
                "format": "vrm",
                "baGender": "male",
            },
            {
                "name": "Female",
                "description": "Anata female — peasant shirt, hair customizable.",
                "portrait": "./loot-assets/anata/female/Body/female.png",
                "manifest": "./loot-assets/anata/female/manifest.json",
                "format": "vrm",
                "baGender": "female",
            },
        ],
        "loras": [],
        "sprites": [],
        "thumbnails": [],
        "defaultAnimations": [
            {"name": "Idle", "description": "Idle", "location": "./loot-assets/animations/2_Idle.fbx", "icon": "|"},
            {"name": "Walking", "description": "Walk", "location": "./loot-assets/animations/3_Walking.fbx", "icon": "|"},
        ],
    }
    SELECT.write_text(json.dumps(select, indent=2), encoding="utf-8")


def main() -> None:
    # Rebuild from loot originals if BA already stripped Waist/Hands — restore from git first.
    loot_git = PUBLIC / "loot-assets"
    if (loot_git / ".git").is_dir():
        import subprocess

        subprocess.run(
            ["git", "checkout", "HEAD", "--", "anata/male/manifest.json", "anata/female/manifest.json"],
            cwd=str(loot_git),
            check=False,
        )

    build("male")
    build("female")
    write_select()


if __name__ == "__main__":
    main()
