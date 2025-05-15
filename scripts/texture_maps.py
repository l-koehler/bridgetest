#!/usr/bin/env python3

# this file generates the texture maps in extra_data/
# you shouldn't need to run it, unless you want to update the proxy to support new versions

import json, os
from pathlib import Path
# path to unpacked client jar file
# change as needed
jar_data_root = Path("/home/user/Code/minecraft-1.21.4-client")

if not (jar_data_root.exists() and jar_data_root.is_dir()):
    print(f"Data root does not exist/is not a directory: {jar_data_root}")
    exit(1)
if not (jar_data_root/"version.json").is_file():
    print(f"Data root seems invalid. The directory should contain a version.json file")
    exit(1)
blockstate_dir = jar_data_root/"assets/minecraft/blockstates/"
block_model_dir = jar_data_root/"assets/minecraft/models/block/"
item_model_dir = jar_data_root/"assets/minecraft/models/item/"
print(f"Unpacked client jar at: {jar_data_root}")

# i love deadline-oriented programming, minecraft is such a functional and normal piece of ~~crap~~ software
fake_blocks = [
    "minecraft:glow_item_frame",
    "minecraft:item_frame"
]

json_files = sorted(blockstate_dir.glob('*.json'))

# List<(String, String)> with block_id and model_path
models = []
fakes = 0
for state_file in json_files:
    fp = state_file.open()
    data = json.load(fp)
    fp.close()
    block_id = f"minecraft:{state_file.stem}"
    # fuck this
    if block_id in fake_blocks:
        fakes += 1
        continue
    model = None
    if "variants" in data:
        for variant in data["variants"].values():
            if isinstance(variant, list):
                model = variant[0]["model"]
            else:
                model = variant["model"]
            break
    elif "multipart" in data:
        for part in data["multipart"]:
            apply = part.get("apply")
            if isinstance(apply, list):
                model = apply[0]["model"]
            else:
                model = apply["model"]
            break
    if model != None:
        model = model.replace("minecraft:block/", "")
        model = block_model_dir/f"{model}.json"

    models.append((block_id, model))
# skip none-type, these were accounted for above
missing = [m for m in models if not (m[1] == None or m[1].is_file())]
failures = [m for m in models if m[1] == None]
if len(failures) != 0:
    print(f"Bad Mappings: {failures}")
if len(missing) != 0:
    print(f"Bad Paths: {missing}")

mapping = {}
for model in models:
    fp = open(model[1])
    data = json.load(fp)
    fp.close()
    textures = data.get("textures", {})
    if not textures:
        continue

    # pick the shortest, as that is likely the least specific (don't use _top when a generic texture exists)
    texture_ref = sorted(list(textures.values()), key=len)[0]
    if texture_ref.startswith("minecraft:"):
        texture_ref = texture_ref.split(":", 1)[1]

    # very nice easter egg but it is breaking everything
    if texture_ref == "missingno":
        continue
    
    texture_path = f"./{texture_ref}.png"
    mapping[model[0]] = texture_path

# works as long as this file doesn't get moved around
block_mapping_file = Path(__file__).parent.parent/"extra_data/block_texture_map.json"
print(f"Writing block mapping to: {block_mapping_file}")
fp = open(block_mapping_file, 'w')
json.dump(mapping, fp)
fp.close()

# do the same stuff for items
mapping = {}
for model_file in sorted(item_model_dir.glob("*.json")):
    item_id = f"minecraft:{model_file.stem}"
    fp = open(model_file)
    data = json.load(fp)
    fp.close()

    if "textures" not in data:
        continue # there are a bunch of weird non-items in there. this is fine
    textures = data["textures"]

    texture_ref = sorted(list(textures.values()), key=len)[0]
    if texture_ref.startswith("minecraft:"):
        texture_ref = texture_ref.split(":", 1)[1]

    if texture_ref == "missingno":
        continue
    
    texture_path = f"./{texture_ref}.png"
    mapping[item_id] = texture_path

# add missing mappings (deadline-oriented design strikes again)
mapping["minecraft:compass"] = mapping["minecraft:compass_00"]
mapping["minecraft:clock"] = mapping["minecraft:clock_00"]
mapping["minecraft:recovery_compass"] = mapping["minecraft:recovery_compass_00"]

item_mapping_file = Path(__file__).parent.parent/"extra_data/item_texture_map.json"
print(f"Writing item mapping to: {item_mapping_file}")
fp = open(item_mapping_file, 'w')
json.dump(mapping, fp)
fp.close()