#!/usr/bin/env python3
# dump all block states
import json
import argparse
from pathlib import Path
from itertools import product

parser = argparse.ArgumentParser(
    prog="dump_block_states.py",
    description="Generate extra_data/block_states.json",
    epilog="Do not move this program from its place in the repository!"
)
parser.add_argument('asset_root', help="Path to a unpacked Minecraft 26.2 client jar")
args = parser.parse_args()

asset_root = Path(args.asset_root)
blockstates_dir = asset_root / "assets/minecraft/blockstates"
bridgetest = Path(__file__).parent.parent

# multiparts dont explicitly include unset fields
# so we add defaults to those
UNSET = object()

def parse_variant_key(key):
    # "" or "prop1=val1,prop2=val2,..."
    if key == "":
        return {}
    return dict(pair.split("=", 1) for pair in key.split(","))

def collect_condition_values(when, out):
    if "OR" in when:
        for sub in when["OR"]:
            collect_condition_values(sub, out)
    elif "AND" in when:
        for sub in when["AND"]:
            collect_condition_values(sub, out)
    else:
        for k, v in when.items():
            out.setdefault(k, set()).update(str(v).split("|"))

def multipart_states(data):
    # these dont list all states explicitly
    literal_values = {}
    for part in data["multipart"]:
        if part.get("when") is not None:
            collect_condition_values(part.get("when"), literal_values)

    domains = {}
    for prop, values in literal_values.items():
        complete = values == {"true", "false"} or "none" in values
        domains[prop] = sorted(values) if complete else sorted(values) + [UNSET]

    props = list(domains.keys())
    for combo in product(*(domains[p] for p in props)):
        yield {p: v for p, v in zip(props, combo) if v is not UNSET}

block_states = []
for path in sorted(blockstates_dir.glob("*.json")):
    block_name = f"minecraft:{path.stem}"
    with path.open() as f:
        data = json.load(f)

    if "variants" in data:
        properties_list = [parse_variant_key(k) for k in data["variants"]]
    elif "multipart" in data:
        properties_list = list(multipart_states(data))
    else:
        print(f"{path.name} has no 'variants' or 'multipart', skipping it!")
        continue

    for properties in properties_list:
        entry = {"name": block_name}
        if properties:
            entry["properties"] = properties
        block_states.append(entry)

out_path = bridgetest / "extra_data/block_states.json"
out_path.write_text(json.dumps(block_states, separators=(",", ":")))
print(f"Got {len(block_states)} block states, wrote to {out_path}")
