use azalea::world::MinecraftEntityId;
use bimap::BiMap;

#[derive(Clone)]
pub struct EntityState {
    // 32 bit server-side ID <-> 16 bit client-side ID
    pub entity_id_map: BiMap<MinecraftEntityId, u16>,
    // allocatable (free) ID ranges on the client
    // adjacent free ranges are joined on entity removal, range is inclusive on both sides
    // adding a entity will pick the lowest ID of the smallest range to prevent fragmentation
    // starts with 0 non-allocatable because the player doesn't properly get a server-side ID
    pub c_alloc_id_ranges: Vec<(u16, u16)>,
    // entities that will be updated in the next tick
    // used to prevent flooding the client with thousands of packets
    // side effect: we only iterate the ECS once
    pub entities_update_scheduled: Vec<MinecraftEntityId>,
}

impl Default for EntityState {
    fn default() -> Self {
        EntityState {
            entity_id_map: BiMap::new(),
            c_alloc_id_ranges: vec![(2, u16::MAX)], // 0 reserved for player, 1 causes issues
            entities_update_scheduled: Vec::new(),
        }
    }
}
