#!/usr/bin/env python3
import json
import hashlib
from pathlib import Path

asset_root = Path("/home/user/Code/minecraft-1.21.4-client")
blockstates_dir = asset_root / "assets/minecraft/blockstates"
models_dir = asset_root / "assets/minecraft/models/block"
item_dir = asset_root / "assets/minecraft/models/item"

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
    # recursively resolve model parents, merge textures and elements
    if seen is None:
        seen = set()
    if model_name in seen:
        return {}, []
    seen.add(model_name)

    path = models_dir / (model_name.replace("minecraft:block/", "") + ".json")
    data = load_json(path)
    if not data:
        if model_name in special_nodeboxes:
            data = special_nodeboxes[model_name]
        else:
            print(f"WARN: Missing model: {model_name}")
            return {}, []

    # inherit
    parent_textures, parent_elements = {}, []
    parent = data.get("parent")
    if parent:
        parent_textures, parent_elements = resolve_model(parent, seen)

    # merge textures
    textures = parent_textures.copy()
    textures.update(data.get("textures", {}))

    # use elements if present
    elements = data.get("elements", parent_elements)

    return textures, elements

def get_face_textures(texture_map):
    # extract the textures for each face direction
    has_particle = ("particle" in texture_map)
    faces = {k: None for k in face_keys}
    for key in face_keys:
        texture = None
        if key in texture_map:
            texture = texture_map[key]
            if texture[0] == "#":
                texture = texture_map[texture[1::]]
        else:
            # try aliases
            for key_alias in key_aliases[key]:
                if key_alias in texture_map:
                    texture = texture_map[key_alias]
                    if texture[0] == "#":
                        texture = texture_map[texture[1::]]
            # default to particle, then air
            if has_particle and texture == None:
                texture = texture_map["particle"]
                if texture[0] == "#":
                    texture = texture_map[texture[1::]]
            elif texture == None:
                texture = "minecraft:block/air"
        texture = texture.replace("minecraft:", "./")
        faces[key] = texture+".png"
    return faces

def round_box(box):
    return [int(round(v)) for v in box]

def extract_nodebox(elements):
    # get cuboids from element
    boxes = []
    for el in elements:
        from_box = el.get("from")
        to_box = el.get("to")
        if not from_box or not to_box:
            continue
        cuboid = round_box(from_box + to_box)
        boxes.append(cuboid)
    return boxes

def is_flower_like(elements):
    if len(elements) != 2:
        return False
    planes = []
    for el in elements:
        from_x, from_y, from_z = el["from"]
        to_x, to_y, to_z = el["to"]
        if from_y != 0 or to_y != 16:
            return False
        if from_x == to_x or from_z == to_z:
            planes.append((from_x, from_z, to_x, to_z))
    return len(planes) == 2

def is_full_cube(elements):
    if len(elements) != 1:
        return False
    el = elements[0]
    return round_box(el["from"] + el["to"]) == [0, 0, 0, 16, 16, 16]

def determine_drawtype(textures, elements, nodeboxes):
    if not elements:
        texture = [textures['particle'] if "particle" in textures else ""][0]
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
    if is_flower_like(elements):
        return "flower"
    if is_full_cube(elements):
        return "full"

    # nodebox name is its hash, prevents duplicates 
    key_data = json.dumps(sorted(nodeboxes)).encode("utf-8")
    key_hash = hashlib.sha1(key_data).hexdigest()[:8]
    key = f"NB_{key_hash}"
    nodebox_map[key] = nodeboxes
    return key

# blocks and nodeboxes
print("Generating block mappings and nodeboxes...")
for block_file in sorted(blockstates_dir.glob("*.json")):
    block_id = f"minecraft:{block_file.stem}"
    data = load_json(block_file)

    model_name = None
    if "variants" in data:
        first = next(iter(data["variants"].values()), None)
        model_name = first[0]["model"] if isinstance(first, list) else first["model"]
    elif "multipart" in data:
        for part in data["multipart"]:
            apply = part.get("apply")
            model_name = apply[0]["model"] if isinstance(apply, list) else apply["model"]
            break

    if not model_name:
        continue

    textures, elements = resolve_model(model_name)
    face_textures = get_face_textures(textures)
    nodeboxes = extract_nodebox(elements)
    drawtype = determine_drawtype(textures, elements, nodeboxes)
    block_map[block_id] = {
        "textures": face_textures,
        "drawtype": drawtype
    }

# items
print("Generating item mappings...")
for model_file in sorted(item_dir.glob("*.json")):
    item_id = f"minecraft:{model_file.stem}"
    data = load_json(model_file)

    if "textures" not in data:
        continue # there are a bunch of weird non-items in there. this is fine
    textures = data["textures"]

    texture_ref = sorted(list(textures.values()), key=len)[0]
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

# save data
texture_file = bridgetest/"extra_data/block_texture_map.json"
with open(texture_file, "w") as f:
    json.dump(block_map, f, indent=2)

nodebox_file = bridgetest/"extra_data/nodeboxes.json"
with open(nodebox_file, "w") as f:
    json.dump(nodebox_map, f, indent=2)

item_file = bridgetest/"extra_data/item_texture_map.json"
with open(item_file, "w") as f:
    json.dump(item_map, f, indent=2)

print(f"Done! Saved mappings and nodeboxes to: {bridgetest/'extra_data'}")