"""
Build VRM1 peasant pants that fit neuro/drop bodies.

Native full-length legs mesh (correct skeleton) + Anata linen albedo.
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
        "legs_trait": "legs",
        "base": PUBLIC / "character-assets/drophunter/legs/cargopants.vrm",
        "anata_shirt": ANATA / "female/Chest/shirt.vrm",
        "anata_albedo_image": 1,
        "out": PUBLIC / "character-assets/drophunter/legs/peasant_pants.vrm",
    },
    {
        "pack": "neurohacker",
        "legs_trait": "legs",
        "base": PUBLIC / "character-assets/neurohacker/legs/workingpants.vrm",
        "anata_shirt": ANATA / "male/Chest/shirt.vrm",
        "anata_albedo_image": 1,
        "out": PUBLIC / "character-assets/neurohacker/legs/peasant_pants.vrm",
    },
]

SHADE = [0.28, 0.20, 0.14]
BASE_TINT = [0.75, 0.62, 0.45, 1.0]


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
    img = g.images[image_index]
    old_view = g.bufferViews[img.bufferView]
    blob = bytearray(g.binary_blob())
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

    # Replace every embedded image (albedo + mtoon shade/rim) so old pants art
    # doesn't peek through on multi-texture meshes like workingpants.
    for i in range(len(base.images or [])):
        _replace_image(base, i, albedo, mime)

    mat = base.materials[0]
    if mat.pbrMetallicRoughness:
        mat.pbrMetallicRoughness.baseColorFactor = list(BASE_TINT)
    mtoon = (mat.extensions or {}).get("VRMC_materials_mtoon")
    if isinstance(mtoon, dict):
        mtoon["shadeColorFactor"] = list(SHADE)
        mtoon["outlineWidthFactor"] = 0.0
        # Point shade multiply at albedo slot 0 if it referenced another image.
        if isinstance(mtoon.get("shadeMultiplyTexture"), dict):
            mtoon["shadeMultiplyTexture"]["index"] = 0

    extras = dict(base.extras) if isinstance(base.extras, dict) else {}
    extras["baPeasantPants"] = {
        "baseMesh": job["base"].name,
        "albedoFrom": job["anata_shirt"].name,
    }
    base.extras = extras

    out: Path = job["out"]
    out.parent.mkdir(parents=True, exist_ok=True)
    base.save_binary(str(out))

    thumb = out.with_suffix(".png")
    base_png = job["base"].with_suffix(".png")
    if base_png.is_file():
        shutil.copy2(base_png, thumb)
    elif THUMB.is_file():
        shutil.copy2(THUMB, thumb)

    print(f"wrote {out.relative_to(PUBLIC)}")


def update_manifests() -> None:
    for job in JOBS:
        pack = job["pack"]
        path = PUBLIC / "character-assets" / pack / "manifest.json"
        data = json.loads(path.read_text(encoding="utf-8"))

        peasant = {
            "id": "peasant_pants",
            "name": "Peasant Pants",
            "directory": "legs/peasant_pants.vrm",
            "thumbnail": "legs/peasant_pants.png"
            if (PUBLIC / "character-assets" / pack / "legs/peasant_pants.png").is_file()
            else "legs/peasant_pants.vrm",
            "cullingLayer": 1,
        }

        # Ensure legs trait exists.
        legs = next((t for t in data["traits"] if t["trait"] == "legs"), None)
        if legs is None:
            legs = {
                "trait": "legs",
                "name": "legs",
                "icon": "legs.png",
                "iconSvg": "legs.svg",
                "cullingLayer": 1,
                "cameraTarget": {"distance": 1.4, "height": 0.6},
                "collection": [],
            }
            data["traits"].append(legs)

        coll = [c for c in (legs.get("collection") or []) if c.get("id") != "peasant_pants"]
        for c in coll:
            c["locked"] = True
            c["purchasable"] = True
            c["price"] = c.get("price") or 1
        legs["collection"] = [peasant, *coll]

        req = list(data.get("requiredTraits") or [])
        if "legs" not in req:
            req.append("legs")
        data["requiredTraits"] = req

        initial = list(data.get("initialTraits") or [])
        # body, eyes, head, chest, legs — keep order stable
        ordered = []
        for key in ("body", "eyes", "head", "chest", "legs", "feet", "foot"):
            if key in initial or key in ("chest", "legs") or key in req:
                if key not in ordered and (
                    key in initial or key in ("body", "eyes", "head", "chest", "legs")
                ):
                    ordered.append(key)
        for key in initial:
            if key not in ordered:
                ordered.append(key)
        if "legs" not in ordered:
            # insert after chest
            if "chest" in ordered:
                ordered.insert(ordered.index("chest") + 1, "legs")
            else:
                ordered.append("legs")
        data["initialTraits"] = ordered

        data["ba"] = data.get("ba") or {}
        locked = dict(data["ba"].get("lockedOutfit") or {})
        locked["chest"] = locked.get("chest") or "peasant_shirt"
        locked["legs"] = "peasant_pants"
        data["ba"]["lockedOutfit"] = locked
        data["ba"]["note"] = "VRM1 peasant tunic + pants (native mesh + Anata linen)."

        text = json.dumps(data, indent=2)
        path.write_text(text, encoding="utf-8")
        (path.parent / "manifest.ba.json").write_text(text, encoding="utf-8")
        print(f"manifest {pack}: legs=peasant_pants initial={data['initialTraits']}")


def main() -> None:
    for job in JOBS:
        make_one(job)
    update_manifests()


if __name__ == "__main__":
    main()
