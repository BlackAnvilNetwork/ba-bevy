"""
Fit Anata VRM0 chest gear to neuro/drop body size WITHOUT breaking skinning.

Scales via a new root node (scale + Y translation). Mesh buffers and inverse-bind
matrices stay untouched so weights remain valid. Keeps VRM0 for Character Studio.
"""
from __future__ import annotations

from pathlib import Path

from pygltflib import GLTF2, BufferFormat, Node

ROOT = Path(r"C:\Users\raxac\OneDrive\Desktop\apps\black-anvil")
PUBLIC = ROOT / "character-studio/public"
ANATA = PUBLIC / "loot-assets/anata"

GEAR = [
    "shirt",
    "linen_robe",
    "robe",
    "silk_robe",
    "hard_leather_armor",
    "leather_armor",
    "chain_mail",
    "ring_mail",
]

TARGETS = [
    {
        "anata_gender": "female",
        "body": PUBLIC / "character-assets/drophunter/body/drophunter.vrm",
        "out_dir": PUBLIC / "character-assets/drophunter/chest",
        "collar_nudge_y": 0.09,
        # Negative local Z seats the shirt on the back (positive went forward).
        "depth_nudge_z": -0.04,
    },
    {
        "anata_gender": "male",
        "body": PUBLIC / "character-assets/neurohacker/body/neurohacker.vrm",
        "out_dir": PUBLIC / "character-assets/neurohacker/chest",
        "collar_nudge_y": 0.08,
        "depth_nudge_z": -0.04,
    },
]


def _load(path: Path) -> GLTF2:
    g = GLTF2().load_binary(str(path))
    g.convert_buffers(BufferFormat.BINARYBLOB)
    return g


def _hips_y(g: GLTF2) -> float:
    ext = g.extensions or {}
    if "VRM" in ext:
        bones = ext["VRM"].get("humanoid", {}).get("humanBones", [])
        hips = next((b for b in bones if b.get("bone") == "hips"), None)
        if hips is not None:
            t = g.nodes[hips["node"]].translation or [0, 0, 0]
            return float(t[1])
    if "VRMC_vrm" in ext:
        hips = ext["VRMC_vrm"].get("humanoid", {}).get("humanBones", {}).get("hips")
        if hips is not None:
            t = g.nodes[hips["node"]].translation or [0, 0, 0]
            return float(t[1])
    raise RuntimeError("hips bone not found")


def convert_file(
    src: Path,
    body: Path,
    out: Path,
    collar_nudge_y: float,
    depth_nudge_z: float,
) -> None:
    gear = _load(src)
    body_g = _load(body)
    src_hips = _hips_y(gear)
    dst_hips = _hips_y(body_g)
    scale = dst_hips / src_hips
    # With node TRS = T*R*S, root translation is not scaled; hips world Y = ty + scale*src_hips.
    # Choose scale so scale*src_hips == dst_hips, then ty/tz are pure nudges.
    ty = collar_nudge_y
    tz = depth_nudge_z

    if not gear.scenes:
        raise RuntimeError(f"no scenes in {src}")
    scene = gear.scenes[gear.scene or 0]
    old_roots = list(scene.nodes or [])

    root = Node(
        name="BA_FitRoot",
        children=old_roots,
        translation=[0.0, ty, tz],
        scale=[scale, scale, scale],
    )
    root_idx = len(gear.nodes)
    gear.nodes.append(root)
    scene.nodes = [root_idx]

    extras = dict(gear.extras) if isinstance(gear.extras, dict) else {}
    extras["baFit"] = {
        "targetBody": body.name,
        "scale": scale,
        "ty": ty,
        "tz": tz,
        "srcHipsY": src_hips,
        "dstHipsY": dst_hips,
        "method": "rootNodeTRS",
    }
    gear.extras = extras

    out.parent.mkdir(parents=True, exist_ok=True)
    gear.save_binary(str(out))
    print(
        f"  {out.name}: scale={scale:.4f} ty={ty:.4f} tz={tz:.4f} "
        f"(hips {src_hips:.3f}->{dst_hips:.3f})"
    )


def main() -> None:
    for target in TARGETS:
        src_dir = ANATA / target["anata_gender"] / "Chest"
        print(f"=== {target['anata_gender']} → {target['out_dir'].parent.name} ===")
        for stem in GEAR:
            src = src_dir / f"{stem}.vrm"
            if not src.is_file():
                continue
            out = target["out_dir"] / f"anata_{stem}.vrm"
            convert_file(
                src,
                target["body"],
                out,
                target["collar_nudge_y"],
                target.get("depth_nudge_z", 0.0),
            )


if __name__ == "__main__":
    main()
