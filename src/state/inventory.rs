use azalea::container::ContainerHandle;
use azalea::inventory::ItemStack;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct InventoryState {
    // used to determine need for resyncing (on tick)
    pub clientside_fields: Vec<(String, Vec<ItemStack>)>,
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
            clientside_fields: Vec::new(),
            inventory_handle: None,
            container_id: None,
        }
    }
}
