#!/usr/bin/env python3
# dump the banner pattern tags the loom's selectable-pattern lists are built from
import json
import argparse
from pathlib import Path

parser = argparse.ArgumentParser(
    prog="banner_patterns.py",
    description="Generate extra_data/banner_patterns.json",
    epilog="Do not move this program from its place in the repository!"
)
parser.add_argument('asset_root', help="Path to a unpacked Minecraft 26.2 client jar")
args = parser.parse_args()

asset_root = Path(args.asset_root)
tags_dir = asset_root / "data/minecraft/tags/banner_pattern"
bridgetest = Path(__file__).parent.parent

# tag values may be plain ids, {"id": ..., "required": ...} objects,
# or "#namespace:path" references to another banner_pattern tag.
# order matters: the server indexes loom button clicks into this exact order
def resolve_tag(name):
    namespace, _, path = name.rpartition(":")
    if namespace not in ("", "minecraft"):
        raise SystemExit(f"can't resolve non-vanilla tag reference {name}")
    values = json.loads((tags_dir / f"{path}.json").read_text())["values"]
    patterns = []
    for value in values:
        if isinstance(value, dict):
            value = value["id"]
        if value.startswith("#"):
            patterns += resolve_tag(value[1:])
        else:
            patterns.append(value)
    return patterns

# a pattern item "minecraft:<x>_banner_pattern" provides the patterns in the
# tag "pattern_item/<x>" (via its provides_banner_patterns item component)
pattern_items = {}
for tag_file in sorted((tags_dir / "pattern_item").glob("*.json")):
    item = f"minecraft:{tag_file.stem}_banner_pattern"
    pattern_items[item] = resolve_tag(f"pattern_item/{tag_file.stem}")

data = {
    "no_item_required": resolve_tag("no_item_required"),
    "pattern_items": pattern_items,
}
out_path = bridgetest / "extra_data/banner_patterns.json"
out_path.write_text(json.dumps(data, indent=2) + "\n")
print(f"wrote {out_path}")
