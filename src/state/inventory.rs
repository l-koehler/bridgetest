use azalea::container::ContainerHandle;
use azalea_client::inventory;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct InventoryState {
    // used to determine need for resyncing (on tick)
    pub mt_clientside_player_inv: inventory::Player,
    // never read, only used to not drop the handle.
    // cursed. sorry. just leave it be, it won't break i think
    pub inventory_handle: Option<Arc<Mutex<ContainerHandle>>>,
    // None if no container is open.
    // we could use the ECS, but this is needed for edge detection
    pub container_id: Option<i32>,
}

impl Default for InventoryState {
    fn default() -> Self {
        InventoryState {
            mt_clientside_player_inv: inventory::Player::default(),
            inventory_handle: None,
            container_id: None,
        }
    }
}
