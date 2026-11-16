"""
Build a VRM1 peasant tunic that fits neuro/drop bodies.

Uses a native long chest mesh (correct skeleton) and swaps in Anata's
brown shirt albedo so it reads medieval without the Anata bind mismatch.
"""
from __future__ import annotations

import json
import shutil
from pathlib import Path

from pygltflib import GLTF2, BufferFormat

ROOT = Path(r"C:\Users\raxac\OneDrive\Desktop\apps\black-anvil")
PUBLIC = ROOT / "character-studio/public"
ANATA = PUBLIC / "loot-assets/anata"
THUMB = PUBLIC / "loot-assets/loot/thumbnails/Chest/shirt.png"

JOBS = [
    {
        "pack": "drophunter",
        "base": PUBLIC / "character-assets/drophunter/chest/dress.vrm",
        "anata_shirt": ANATA / "female/Chest/shirt.vrm",
        "anata_albedo_image": 1,  # shirt jpeg albedo
        "out": PUBLIC / "character-assets/drophunter/chest/peasant_shirt.vrm",
    },
    {
        "pack": "neurohacker",
        # simpleshirt = basic tee/tunic silhouette (dressrobe reads as a housecoat).
        "base": PUBLIC / "character-assets/neurohacker/chest/simpleshirt.vrm",
        "anata_shirt": ANATA / "male/Chest/shirt.vrm",
        "anata_albedo_image": 1,
        "out": PUBLIC / "character-assets/neurohacker/chest/peasant_shirt.vrm",
    },
]

# Warm linen brown for MToon shade + slight base tint.
SHADE = [0.35, 0.24, 0.16]
BASE_TINT = [0.85, 0.72, 0.55, 1.0]


def _load(path: Path) -> GLTF2:
    g = GLTF2().load_binary(str(path))
    g.convert_buffers(BufferFormat.BINARYBLOB)
    return g


def _image_bytes(g: GLTF2, image_index: int) -> tuple[bytes, str]:
    img = g.images[image_index]
    blob = g.binary_blob()
    view = g.bufferViews[img.bufferView]
    start = view.byteOffset or 0
    data = bytes(blob[start : start + view.byteLength])
    return data, img.mimeType or "image/jpeg"


def _replace_image(g: GLTF2, image_index: int, data: bytes, mime: str) -> None:
    """Replace an embedded image; rebuilds the binary blob (single buffer)."""
    img = g.images[image_index]
    old_view = g.bufferViews[img.bufferView]
    blob = bytearray(g.binary_blob())

    old_start = old_view.byteOffset or 0
    old_end = old_start + old_view.byteLength
    # Append new image at end (leave old bytes as unused padding — simplest safe edit).
    # Align to 4 bytes.
    while len(blob) % 4:
        blob.append(0)
    new_start = len(blob)
    blob.extend(data)
    while len(blob) % 4:
        blob.append(0)

    old_view.byteOffset = new_start
    old_view.byteLength = len(data)
    img.mimeType = mime
    img.uri = None
    g.buffers[0].byteLength = len(blob)
    g.set_binary_blob(bytes(blob))


def make_one(job: dict) -> None:
    base = _load(job["base"])
    anata = _load(job["anata_shirt"])
    albedo, mime = _image_bytes(anata, job["anata_albedo_image"])

    # Base VRM1 chests use image 0 as albedo (+ mtoon shade multiply).
    _replace_image(base, 0, albedo, mime)

    mat = base.materials[0]
    if mat.pbrMetallicRoughness:
        mat.pbrMetallicRoughness.baseColorFactor = list(BASE_TINT)
    mtoon = (mat.extensions or {}).get("VRMC_materials_mtoon")
    if isinstance(mtoon, dict):
        mtoon["shadeColorFactor"] = list(SHADE)
        # Drop outline so tunic reads softer / less anime-jacket.
        mtoon["outlineWidthFactor"] = 0.0

    extras = dict(base.extras) if isinstance(base.extras, dict) else {}
    extras["baPeasantShirt"] = {
        "baseMesh": job["base"].name,
        "albedoFrom": job["anata_shirt"].name,
        "note": "VRM1 skeleton + Anata linen albedo",
    }
    base.extras = extras

    out: Path = job["out"]
    out.parent.mkdir(parents=True, exist_ok=True)
    base.save_binary(str(out))

    # Thumbnail: prefer Anata shirt thumb, else copy base png if present.
    thumb = out.with_suffix(".png")
    if THUMB.is_file():
        shutil.copy2(THUMB, thumb)
    else:
        cand = job["base"].with_suffix(".png")
        if cand.is_file():
            shutil.copy2(cand, thumb)

    print(f"wrote {out.relative_to(PUBLIC)} ({len(albedo)} byte albedo)")


def update_manifests() -> None:
    for pack, gender in (("neurohacker", "male"), ("drophunter", "female")):
        path = PUBLIC / "character-assets" / pack / "manifest.json"
        data = json.loads(path.read_text(encoding="utf-8"))
        for trait in data["traits"]:
            if trait["trait"] != "chest":
                continue
            # Keep other items but put peasant_shirt first and unlocked.
            coll = trait.get("collection") or []
            coll = [c for c in coll if c.get("id") != "peasant_shirt"]
            for c in coll:
                if c.get("id", "").startswith("anata_"):
                    c["locked"] = True
                    c["purchasable"] = True
                    c["price"] = c.get("price") or 1
            peasant = {
                "id": "peasant_shirt",
                "name": "Peasant Shirt",
                "directory": "chest/peasant_shirt.vrm",
                "thumbnail": "chest/peasant_shirt.png"
                if (PUBLIC / "character-assets" / pack / "chest/peasant_shirt.png").is_file()
                else "chest/peasant_shirt.vrm",
                "cullingLayer": 1,
            }
            trait["collection"] = [peasant, *coll]
            trait["name"] = "chest"
        data["ba"] = data.get("ba") or {}
        data["ba"]["lockedOutfit"] = {"chest": "peasant_shirt"}
        data["ba"]["starterChest"] = "peasant_shirt"
        data["ba"]["note"] = "VRM1 peasant tunic (native mesh + Anata linen albedo)."
        text = json.dumps(data, indent=2)
        path.write_text(text, encoding="utf-8")
        (path.parent / "manifest.ba.json").write_text(text, encoding="utf-8")
        print(f"manifest {pack}: starter=peasant_shirt")


def main() -> None:
    for job in JOBS:
        make_one(job)
    update_manifests()


if __name__ == "__main__":
    main()
