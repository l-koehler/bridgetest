use azalea_registry::Registry;
use azalea_registry::builtin::EntityKind;
use std::collections::HashMap;

fn main() {
    println!("cargo:rerun-if-changed=extra_data/entity_info.json");
    println!("cargo:rerun-if-changed=extra_data/entity_variants.json");

    // warn about entities not mapped to a model/texture
    // defaults helpfully to a red shulker
    let data = std::fs::read_to_string("extra_data/entity_info.json")
        .expect("failed to read extra_data/entity_info.json");
    let entries: HashMap<String, serde_json::Value> =
        serde_json::from_str(&data).expect("extra_data/entity_info.json is invalid");

    let mut id = 0u32;
    while let Some(kind) = EntityKind::from_u32(id) {
        if !entries.contains_key(kind.to_str()) {
            println!(
                "cargo:warning=extra_data/entity_info.json has no entry for {} (falls back to \"_default\")",
                kind.to_str()
            );
        }
        id += 1;
    }
}
