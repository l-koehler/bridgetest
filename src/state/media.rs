use super::super::s2c::media::{BlockInfo, BlockMapping, LuantiTexture};
use luanti_protocol::types::NodeBox;
use std::collections::HashMap;

#[derive(Clone)]
pub struct MediaState {
    // maps "minecraft:item"
    pub item_texture_map: HashMap<String, LuantiTexture>,
    // maps "minecraft:block" -> state key (utils::variant_key_from_state) -> mapping
    pub block_texture_map: HashMap<String, HashMap<String, BlockMapping>>,
    // maps NB_abc123
    pub nodebox_lookup: HashMap<String, NodeBox>,
    // maps "minecraft:block" -> light/waving/climbable/transparency metadata
    pub block_info: HashMap<String, BlockInfo>,
    // BlockState id -> Luanti content id used by utils::state_to_node
    pub state_content_ids: Vec<u16>,
}

impl Default for MediaState {
    fn default() -> Self {
        MediaState {
            item_texture_map: HashMap::new(),
            block_texture_map: HashMap::new(),
            nodebox_lookup: HashMap::new(),
            block_info: HashMap::new(),
            state_content_ids: Vec::new(),
        }
    }
}
