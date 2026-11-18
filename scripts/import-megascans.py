"""Import a Quixel texture set or glTF export into the local Stillwater art kit."""
import argparse
import base64
import io
import struct
import json
import re
import shutil
import tempfile
import zipfile
from pathlib import Path
from urllib.parse import unquote

from PIL import Image, ImageOps

ROOT = Path(__file__).resolve().parents[1] / "assets" / "megascans"


def unpack(archive, directory):
    with zipfile.ZipFile(archive) as package:
        for member in package.infolist():
            target = (directory / member.filename).resolve()
            if not target.is_relative_to(directory) or member.is_dir():
                continue
            target.parent.mkdir(parents=True, exist_ok=True)
            with package.open(member) as source, target.open("wb") as dest:
                shutil.copyfileobj(source, dest)


def surface(source, output):
    images = [p for p in source.rglob("*") if p.suffix.lower() in {".png", ".jpg", ".jpeg", ".tif", ".tiff"}]
    def pick(pattern):
        matches = [p for p in images if re.search(pattern, p.stem, re.I)]
        if len(matches) != 1:
            raise ValueError(f"Expected one {pattern} map, found {len(matches)}. Select one resolution and one asset.")
        with Image.open(matches[0]) as image:
            image.thumbnail((2048, 2048), Image.Resampling.LANCZOS)
            return image.convert("RGB"), matches[0].stem.lower()

    color, _ = pick(r"(albedo|base.?color|diffuse)")
    normal, name = pick(r"normal")
    rough, _ = pick(r"roughness")
    if "dx" in name or "directx" in name:
        r, g, b = normal.split()
        normal = Image.merge("RGB", (r, ImageOps.invert(g), b))
    color.save(output / "base.png")
    normal.save(output / "normal.png")
    zero = Image.new("L", rough.size)
    Image.merge("RGB", (zero, rough.convert("L"), zero)).save(output / "metal-rough.png")


def model(source, output):
    files = sorted(p for p in source.rglob("*") if p.suffix.lower() in {".gltf", ".glb"})
    if len(files) != 1:
        raise ValueError("Select one glTF model/LOD with its textures and buffers.")
    path, = files
    binary = None
    if path.suffix.lower() == ".glb":
        raw = path.read_bytes()
        magic, version, length = struct.unpack_from("<4sII", raw)
        if magic != b"glTF" or version != 2 or length != len(raw):
            raise ValueError("Invalid glTF 2 binary")
        offset = 12
        while offset < length:
            size, kind = struct.unpack_from("<I4s", raw, offset)
            payload = raw[offset + 8 : offset + 8 + size]
            if kind == b"JSON":
                doc = json.loads(payload)
            elif kind.startswith(b"BIN"):
                binary = payload
            offset += 8 + size
    else:
        doc = json.loads(path.read_text(encoding="utf-8-sig"))

    def read_uri(uri):
        if uri.startswith("data:"):
            return base64.b64decode(uri.split(",", 1)[1], validate=True)
        dep = (path.parent / unquote(uri)).resolve()
        if not dep.is_relative_to(source) or not dep.is_file():
            raise ValueError(f"Missing local glTF dependency: {uri}")
        return dep.read_bytes()

    buffers = []
    for index, member in enumerate(doc.get("buffers", [])):
        data = read_uri(member["uri"]) if "uri" in member else binary
        if data is None or len(data) < member["byteLength"]:
            raise ValueError("Incomplete glTF buffer")
        buffers.append(data)
        member["uri"] = f"buffer-{index}.bin"
        (output / member["uri"]).write_bytes(data)
    for index, member in enumerate(doc.get("images", [])):
        if "uri" in member:
            data = read_uri(member["uri"])
        else:
            view = doc["bufferViews"][member.pop("bufferView")]
            start = view.get("byteOffset", 0)
            data = buffers[view["buffer"]][start:start + view["byteLength"]]
        member["uri"] = f"texture-{index}.png"
        member["mimeType"] = "image/png"
        with Image.open(io.BytesIO(data)) as image:
            image.thumbnail((2048, 2048), Image.Resampling.LANCZOS)
            image.save(output / member["uri"])
    (output / "scene.gltf").write_text(json.dumps(doc), encoding="utf-8")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("slot", choices=("bank", "gate", "rock", "terrain"))
    parser.add_argument("export", type=Path, help="Fab ZIP, extracted folder, or self-contained GLB")
    args = parser.parse_args()
    ROOT.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(dir=ROOT) as work:
        work = Path(work).resolve()
        source = args.export.resolve()
        if source.is_file():
            incoming = work / "source"
            incoming.mkdir()
            if source.suffix.lower() == ".zip":
                unpack(source, incoming)
            elif source.suffix.lower() == ".glb":
                shutil.copyfile(source, incoming / source.name)
            else:
                parser.error("Supply a ZIP, directory, or GLB")
            source = incoming
        output = work / "ready"
        output.mkdir()
        (model if args.slot in {"rock", "terrain"} else surface)(source, output)
        dest = ROOT / args.slot
        backup = work / "previous"
        if dest.exists():
            dest.rename(backup)
        try:
            output.rename(dest)
        except OSError:
            if backup.exists():
                backup.rename(dest)
            raise
        print(f"Installed {args.slot} at {dest}")


if __name__ == "__main__":
    main()
