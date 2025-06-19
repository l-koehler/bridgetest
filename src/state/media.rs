use super::super::s2c::media::{BlockMapping, LuantiTexture};
use luanti_protocol::types::NodeBox;
use std::collections::HashMap;

#[derive(Clone)]
pub struct MediaState {
    // maps "minecraft:item"
    pub item_texture_map: HashMap<String, LuantiTexture>,
    // maps "minecraft:block"
    pub block_texture_map: HashMap<String, BlockMapping>,
    // maps NB_abc123
    pub nodebox_lookup: HashMap<String, NodeBox>,
}

impl Default for MediaState {
    fn default() -> Self {
        MediaState {
            item_texture_map: HashMap::new(),
            block_texture_map: HashMap::new(),
            nodebox_lookup: HashMap::new(),
        }
    }
}
