#!/usr/bin/env python3
import json
import hashlib
from pathlib import Path
import argparse

try:
    from PIL import Image
except ImportError:
    raise SystemExit(
        "This script uses Pillow to check for transparency"
        "(pip install pillow)"
    )

parser = argparse.ArgumentParser(
    prog="texture_maps.py",
    description="Generate bridgetest texture mappings",
    epilog="Do not move this program from its place in the repository!"
)
parser.add_argument('asset_root', help="Path to a unpacked Minecraft 26.2 client jar")
args = parser.parse_args()

asset_root = Path(args.asset_root)
blockstates_dir = asset_root / "assets/minecraft/blockstates"
models_dir = asset_root / "assets/minecraft/models/block"
item_dir = asset_root / "assets/minecraft/models/item"
texture_dir = asset_root / "assets/minecraft/textures"

bridgetest = Path(__file__).parent.parent
face_keys = ["up", "down", "north", "south", "east", "west"]
key_aliases = {
    "up": ["top"],
    "down": ["bottom"],
    "north": ["side"],
    "south": ["side"],
    "east": ["side"],
    "west": ["side"]
}

block_map = {}
nodebox_map = {}
item_map = {}

def load_json(path):
    try:
        with path.open() as f:
            return json.load(f)
    except Exception:
        return {}

special_nodeboxes = load_json(bridgetest/"extra_data/virtual_models.json")

def resolve_model(model_name, seen=None):
    # recursively resolve model parents
    if model_name.startswith("minecraft:"):
        model_name = model_name[len("minecraft:") :]
    if seen is None:
        seen = set()
    if model_name in seen:
        return {}, []
    seen.add(model_name)

    path = models_dir / (model_name.removeprefix("block/") + ".json")
    data = load_json(path)
    if not data:
        if model_name in special_nodeboxes:
            data = special_nodeboxes[model_name]
        else:
            print(f"WARN: Missing model: {model_name}")
            return {}, []

    parent_textures, parent_elements = {}, []
    parent = data.get("parent")
    if parent:
        parent_textures, parent_elements = resolve_model(parent, seen)

    textures = parent_textures.copy()
    textures.update(data.get("textures", {}))

    elements = data.get("elements", parent_elements)

    return textures, elements

def resolve_texture_ref(texture_map, texture):
    # "#name" references another entry
    seen = set()
    while isinstance(texture, str) and (texture.startswith("#") or texture in texture_map):
        if texture in seen:
            break # loop
        seen.add(texture)
        texture = texture_map.get(texture.removeprefix("#"))
    return texture

def get_face_textures(texture_map, x_deg=0, y_deg=0):
    has_particle = ("particle" in texture_map)
    faces = {k: None for k in face_keys}
    for key in face_keys:
        texture = None
        if key in texture_map:
            texture = resolve_texture_ref(texture_map, texture_map[key])
        else:
            # try alternatives
            for key_alias in key_aliases[key]:
                if key_alias in texture_map:
                    texture = resolve_texture_ref(texture_map, texture_map[key_alias])
            if has_particle and texture == None:
                texture = resolve_texture_ref(texture_map, texture_map["particle"])
            elif texture == None:
                faces[rotated_face(key, x_deg, y_deg)] = "blank.png"
                continue
        if isinstance(texture, dict):
            texture = texture["sprite"]
        if texture in ("missingno", "minecraft:missingno"):
            faces[rotated_face(key, x_deg, y_deg)] = "blank.png"
            continue
        texture = texture.replace("minecraft:", "./")
        faces[rotated_face(key, x_deg, y_deg)] = texture+".png"
    return faces

_SINCOS = {0: (0, 1), 90: (1, 0), 180: (0, -1), 270: (-1, 0)}

def rotate_point(point, x_deg, y_deg):
    x, y, z = point
    cx = cy = cz = 8.0
    dx, dy, dz = x - cx, y - cy, z - cz
    if x_deg % 360:
        s, c = _SINCOS[x_deg % 360]
        dy, dz = dy * c + dz * s, -dy * s + dz * c
    if y_deg % 360:
        s, c = _SINCOS[y_deg % 360]
        dx, dz = dx * c - dz * s, dx * s + dz * c
    # fix x handedndess issues
    dx = -dx
    return (dx + cx, dy + cy, dz + cz)

# points on a 16³ (px) block
FACE_POINTS = {
    "up": (8, 16, 8),
    "down": (8, 0, 8),
    "north": (8, 8, 0),
    "south": (8, 8, 16),
    "east": (16, 8, 8),
    "west": (0, 8, 8),
}
POINT_TO_FACE = {v: k for k, v in FACE_POINTS.items()}

def rotated_face(face, x_deg, y_deg):
    point = tuple(round(c) for c in rotate_point(FACE_POINTS[face], x_deg, y_deg))
    return POINT_TO_FACE[point]

def resolve_element_faces(element, texture_map, x_deg, y_deg):
    # returns {direction: texture}
    resolved = {}
    for local_dir, face_def in element.get("faces", {}).items():
        if local_dir not in FACE_POINTS:
            continue
        texture = resolve_texture_ref(texture_map, face_def.get("texture"))
        if texture is None:
            continue
        if isinstance(texture, dict):
            texture = texture["sprite"]
        texture = texture.replace("minecraft:", "./") + ".png"
        resolved[rotated_face(local_dir, x_deg, y_deg)] = texture
    return resolved

def merge_face_layer(base, new, is_real):
    # do overlays
    for direction, texture in new.items():
        existing = base.get(direction)
        if existing is None or (is_real and not existing[0]):
            base[direction] = (is_real, texture)

def round_coord(v):
    r = round(v)
    # some models have subpixel precision?
    if r == 0 and v > 0:
        r = 1
    elif r == 16 and v < 16:
        r = 15
    return int(r)

def round_box(box):
    return [round_coord(v) for v in box]

def rotate_and_round_element(from_box, to_box, x_deg, y_deg):
    a = rotate_point(tuple(from_box), x_deg, y_deg)
    b = rotate_point(tuple(to_box), x_deg, y_deg)
    lo = [min(a[i], b[i]) for i in range(3)]
    hi = [max(a[i], b[i]) for i in range(3)]
    return round_box(lo + hi)

def extract_nodebox(elements, x_deg=0, y_deg=0):
    boxes = []
    for el in elements:
        from_box = el.get("from")
        to_box = el.get("to")
        if not from_box or not to_box:
            continue
        boxes.append(rotate_and_round_element(from_box, to_box, x_deg, y_deg))
    return boxes

def is_flower_like(cuboids):
    # planes in X shape
    if len(cuboids) != 2:
        return False
    planes = 0
    for (x1, y1, z1, x2, y2, z2) in cuboids:
        if y1 != 0 or y2 != 16:
            return False
        if x1 == x2 or z1 == z2:
            planes += 1
    return planes == 2

def is_full_cube(cuboids):
    if len(cuboids) != 1:
        return False
    return cuboids[0] == [0, 0, 0, 16, 16, 16]

_transparency_cache = {}

def texture_has_transparency(face_texture):
    if face_texture in _transparency_cache:
        return _transparency_cache[face_texture]
    path = texture_dir / face_texture.removeprefix("./")
    result = False
    try:
        with Image.open(path) as img:
            if img.mode in ("RGBA", "LA") or (img.mode == "P" and "transparency" in img.info):
                alpha = img.convert("RGBA").getchannel("A")
                result = alpha.getextrema()[0] < 250
    except (FileNotFoundError, OSError):
        pass # not a minecraft texture; the blank.png added above
    _transparency_cache[face_texture] = result
    return result

def determine_drawtype(textures, cuboids):
    if not cuboids:
        texture = textures.get("particle", "")
        if "fire" in texture:
            return "fire"
        elif "water" in texture or "lava" in texture:
            return "liquid"
        # rather ugly fallback
        # many weird entity-like blocks (beds, chests, signs, banners) are air otherwise
        elif "block" in texture:
            return "full"
        # only applies to blocks with that weird missingno texture (which is not prefixed with block/)
        return "air"
    if is_flower_like(cuboids):
        return "flower"
    if is_full_cube(cuboids):
        return "full"

    # nodebox name is its hash, prevents duplicates
    key_data = json.dumps(sorted(cuboids)).encode("utf-8")
    key_hash = hashlib.sha1(key_data).hexdigest()[:8]
    key = f"NB_{key_hash}"
    nodebox_map[key] = cuboids
    return key

# state related stuff
def mc_variant_key(properties):
    return ",".join(f"{k}={v}" for k, v in sorted(properties.items()))

def parse_variant_key(key):
    if key == "":
        return frozenset()
    return frozenset(tuple(pair.split("=", 1)) for pair in key.split(","))

def pick_variant_value(value):
    # blockstates.json allows a list of random alternatives
    # just pick one
    return value[0] if isinstance(value, list) else value

def apply_model_layer(layers, textures, elements, x_deg, y_deg):
    # fallback whole-face textures, then real per-element faces on top
    if textures:
        merge_face_layer(layers, get_face_textures(textures, x_deg, y_deg), is_real=False)
    for el in elements:
        merge_face_layer(layers, resolve_element_faces(el, textures, x_deg, y_deg), is_real=True)

def resolve_variants_state(block_data, properties):
    # variant keys may omit properties not affecting the model
    # like "waterlogged"
    variants = block_data["variants"]
    full_key = mc_variant_key(properties)
    entry = variants.get(full_key)  # ideally, the key lists every property
    if entry is None:
        state_pairs = frozenset(properties.items())
        for key, candidate in variants.items():
            if parse_variant_key(key) <= state_pairs:
                entry = candidate
                break
    if entry is None:
        # fall back to first variant
        entry = next(iter(variants.values()))
    entry = pick_variant_value(entry)
    model = entry["model"]
    x_deg = entry.get("x", 0)
    y_deg = entry.get("y", 0)
    textures, elements = resolve_model(model)
    cuboids = extract_nodebox(elements, x_deg, y_deg)
    # the guess above is for models that dont define elements[].faces
    # resolve_element_face is used wherever merge_face_layer can apply it
    layers = {}
    apply_model_layer(layers, textures, elements, x_deg, y_deg)
    face_textures = {k: texture for k, (_, texture) in layers.items()}
    return textures, (face_textures or None), elements, cuboids

def value_matches(actual, expected):
    # may be "side|up" for "side" or "up"
    return actual in expected.split("|")

def clause_matches(clause, properties):
    if "OR" in clause:
        return any(clause_matches(sub, properties) for sub in clause["OR"])
    if "AND" in clause:
        return all(clause_matches(sub, properties) for sub in clause["AND"])
    return all(value_matches(properties.get(k), v) for k, v in clause.items())

def resolve_multipart_state(block_data, properties):
    merged_textures = {}
    layers = {}  # {direction: (is_real, texture)}, see merge_face_layer
    merged_elements = []
    for part in block_data["multipart"]:
        when = part.get("when")
        if when is not None and not clause_matches(when, properties):
            continue
        apply_entry = pick_variant_value(part["apply"])
        model = apply_entry["model"]
        x_deg = apply_entry.get("x", 0)
        y_deg = apply_entry.get("y", 0)
        textures, elements = resolve_model(model)
        merged_textures.update(textures)
        # each part rotates independently for some reason
        apply_model_layer(layers, textures, elements, x_deg, y_deg)
        for el in elements:
            from_box = el.get("from")
            to_box = el.get("to")
            if not from_box or not to_box:
                continue
            rotated = rotate_and_round_element(from_box, to_box, x_deg, y_deg)
            merged_elements.append({"from": rotated[0:3], "to": rotated[3:6]})
    cuboids = [el["from"] + el["to"] for el in merged_elements]
    face_textures = {k: texture for k, (_, texture) in layers.items()} or None
    return merged_textures, face_textures, merged_elements, cuboids

def resolve_state(block_data, properties):
    if "variants" in block_data:
        return resolve_variants_state(block_data, properties)
    elif "multipart" in block_data:
        return resolve_multipart_state(block_data, properties)
    else:
        return {}, None, [], []

# redstone wire is evil
# use texture modifiers instead of fixing the horrid mess
def escape_arg(s):
    return s.replace("^", "\\^").replace(":", "\\:")

def redstone_wire_up_texture(properties):
    def connected(direction):
        return properties.get(direction, "none") != "none"

    north, south, east, west = (connected(d) for d in ("north", "south", "east", "west"))

    line = "block/redstone_dust_line0.png"
    if north and south and not east and not west:
        return line
    if east and west and not north and not south:
        return line + "^[transformR90"

    arms = []
    if north:
        arms.append("0,8=" + escape_arg(line + "^[resize:16x8^[transformFY"))
    if south:
        arms.append("0,0=" + escape_arg(line + "^[resize:16x8"))
    if east:
        arms.append("0,0=" + escape_arg(line + "^[transformR90^[resize:8x16^[transformFX"))
    if west:
        arms.append("8,0=" + escape_arg(line + "^[transformR90^[resize:8x16"))

    dot = "0,0=" + escape_arg("block/redstone_dust_dot.png")
    return "[combine:16x16:" + ":".join([dot] + arms)

# blocks and nodeboxes, resolved per reachable block state
print("Loading block state list...")
block_states = load_json(bridgetest/"extra_data/block_states.json")
if not block_states:
    raise SystemExit("extra_data/block_states.json missing/empty. Run tools/dump_block_states.py to fix")

print(f"Resolving {len(block_states)} block states...")
blockstate_json_cache = {}
missing_blockstate_files = set()
for state in block_states:
    block_id = state["name"]
    # default to empty, keeps file size sane
    properties = state.get("properties", {})
    block_name = block_id.removeprefix("minecraft:")
    if block_name not in blockstate_json_cache:
        blockstate_json_cache[block_name] = load_json(blockstates_dir / f"{block_name}.json")
    block_data = blockstate_json_cache[block_name]
    if not block_data:
        missing_blockstate_files.add(block_name)
        continue

    textures, face_textures, elements, cuboids = resolve_state(block_data, properties)
    if face_textures is None:
        face_textures = {k: "blank.png" for k in face_keys}
    drawtype = determine_drawtype(textures, cuboids)
    # we cant check transparency for redstone, thanks to the texture modifiers
    # its always transparent though, so thats not too bad
    if block_id == "minecraft:redstone_wire":
        composite = redstone_wire_up_texture(properties)
        face_textures["up"] = composite
        face_textures["down"] = composite
        cutout = True
    else:
        cutout = any(texture_has_transparency(t) for t in face_textures.values())

    # most blocks show the same texture on all 6 faces, dont store all that
    distinct = set(face_textures.values())
    stored_textures = next(iter(distinct)) if len(distinct) == 1 else face_textures

    key = mc_variant_key(properties)
    block_map.setdefault(block_id, {})[key] = {
        "textures": stored_textures,
        "drawtype": drawtype,
        "cutout": cutout,
    }

for name in sorted(missing_blockstate_files):
    print(f"WARN: No blockstates/{name}.json - every state of minecraft:{name} will be missing")

# items
print("Generating item mappings...")
for model_file in sorted(item_dir.glob("*.json")):
    item_id = f"minecraft:{model_file.stem}"
    data = load_json(model_file)

    if "textures" not in data:
        continue # there are a bunch of weird non-items in there. this is fine
    textures = data["textures"]

    texture_ref = min(textures.values(), key=len)
    if texture_ref.startswith("minecraft:"):
        texture_ref = texture_ref.split(":", 1)[1]

    if texture_ref == "missingno":
        continue

    texture_path = f"./{texture_ref}.png"
    item_map[item_id] = texture_path
# add missing mappings (deadline-oriented design strikes again)
item_map["minecraft:compass"] = item_map["minecraft:compass_00"]
item_map["minecraft:clock"] = item_map["minecraft:clock_00"]
item_map["minecraft:recovery_compass"] = item_map["minecraft:recovery_compass_00"]
# technically this really shouldn't be here but i'm about to do stupid things if i cant get this to work
block_map["minecraft:chain"] = block_map["minecraft:iron_chain"]

# save data
texture_file = bridgetest/"extra_data/block_texture_map.json"
with open(texture_file, "w") as f:
    json.dump(block_map, f, separators=(",", ":"))

nodebox_file = bridgetest/"extra_data/nodeboxes.json"
with open(nodebox_file, "w") as f:
    json.dump(nodebox_map, f, indent=2)

item_file = bridgetest/"extra_data/item_texture_map.json"
with open(item_file, "w") as f:
    json.dump(item_map, f, indent=2)

total_variants = sum(len(v) for v in block_map.values())
print(f"Done: {len(block_map)} blocks, {total_variants} distinct block states, {len(nodebox_map)} distinct nodeboxes")
print(f"Saved mappings and nodeboxes to: {bridgetest/'extra_data'}")
