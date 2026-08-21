use azalea::Client;
use azalea::inventory::operations::{ClickOperation, PickupClick, ThrowClick};
use azalea::protocol::packets::game::ServerboundSetCarriedItem;
use log::*;

use luanti_protocol::LuantiConnection;
use luanti_protocol::commands::client_to_server::{InventoryActionSpec, PlayerItemSpec};
use luanti_protocol::types::{InventoryAction, InventoryLocation};

use std::sync::{Arc, Mutex};

use crate::s2c;
use crate::state;

pub fn set_mainhand(mc_client: &mut Client, specbox: Box<PlayerItemSpec>) {
    // hotbar_index: 0..8, first..last slot of hotbar
    let PlayerItemSpec { item: hotbar_index } = *specbox;
    let _ = mc_client.write_packet(ServerboundSetCarriedItem { slot: hotbar_index });
}

// inventory actions and crafting
pub async fn inventory_action(
    mc_client: &mut Client,
    luanti_conn: &mut LuantiConnection,
    specbox: Box<InventoryActionSpec>,
    inventory_state: &mut state::InventoryState,
) {
    let InventoryActionSpec { action } = *specbox;
    debug!("C2S InventoryAction received: {:?}", action);
    match action {
        InventoryAction::Drop {
            count,
            from_inv: _,
            from_list,
            from_i,
        } => drop_item(count, from_list, from_i, mc_client),
        InventoryAction::Move {
            count,
            from_inv: _,
            from_list,
            from_i,
            to_inv: _,
            to_list,
            to_i,
        } => {
            move_item(
                count,
                from_list,
                from_i,
                to_list,
                to_i,
                mc_client,
                inventory_state,
                luanti_conn,
            )
            .await
        }
        // crafting tables are implemented as regular containers
        InventoryAction::Craft { count, craft_inv } => {
            craft_item(mc_client, luanti_conn, inventory_state, count, craft_inv).await
        }
    }
}

// see https://wiki.vg/File:Inventory-slots.png for full indexing of the player inv
fn to_inv_index(mt_index: u16, mt_list: &str) -> u16 {
    match mt_list {
        "armor" => mt_index + 5,
        "craft" => mt_index + 1,
        "craftpreview" => 0,
        "offhand" => 45,
        "main" => match mt_index {
            0..=8 => mt_index + 36,
            9..=35 => mt_index,
            _ => unreachable!(),
        },
        _ => unreachable!(),
    }
}

pub fn drop_item(count: u16, from_list: String, from_i: i16, mc_client: &mut Client) {
    match from_list.as_str() {
        "container" => {
            let handle = mc_client.get_inventory().unwrap();
            if handle.id() == 0 {
                info!(
                    "[Minetest] Client attempted to drop items from a container while no container was opened"
                );
                return;
            }
            if handle.contents().is_none() {
                info!("Client attempted to drop items from a container without contents");
                return;
            }
            if handle.contents().unwrap()[from_i as usize].count() <= count.into() {
                handle.click(ClickOperation::Throw(ThrowClick::All {
                    slot: from_i as u16,
                }))
            } else {
                while handle.contents().unwrap()[from_i as usize].count() > 0 {
                    handle.click(ClickOperation::Throw(ThrowClick::Single {
                        slot: from_i as u16,
                    }))
                }
            }
        }
        "main" | "armor" | "offhand" | "craft" | "craftpreview" => {
            let maybe_handle = mc_client.open_inventory().ok().flatten();
            if maybe_handle.is_none() {
                info!(
                    "[Minetest] Client attempted to drop items from the inventory while a container was opened",
                );
                return;
            }
            let handle = maybe_handle.unwrap();
            let slot_index = to_inv_index(from_i as u16, from_list.as_str());
            if handle.contents().unwrap()[slot_index as usize].count() <= count.into() {
                handle.click(ClickOperation::Throw(ThrowClick::All { slot: slot_index }))
            } else {
                while handle.contents().unwrap()[slot_index as usize].count() > 0 {
                    handle.click(ClickOperation::Throw(ThrowClick::Single {
                        slot: slot_index,
                    }))
                }
            }
        }
        _ => unreachable!(),
    }
}

pub async fn move_item(
    count: u16,
    from_list: String,
    from_i: i16,
    to_list: String,
    to_i: Option<i16>,
    mc_client: &mut Client,
    inventory_state: &mut state::InventoryState,
    luanti_conn: &mut LuantiConnection,
) {
    let Some(to_i) = to_i else {
        warn!("Client sent InventoryAction::Move without a destination slot, ignoring.");
        return;
    };
    debug!(
        "Moving {} item(s) from {}@{} to {}@{} (Luanti indices/lists)",
        count, from_i, from_list, to_i, to_list
    );

    let Ok(menu) = mc_client.menu() else {
        error!("Client does not have an inventory component!");
        return;
    };
    let offset = menu.player_slots_range().min().unwrap() as u16;

    // pick up item
    // -9 to shift to main-only (to_inv_index includes armor, offhand and 2x2)
    let index_from = if from_list == "container" {
        // implicitly correct by formspecs
        from_i as u16
    } else {
        (to_inv_index(from_i as u16, &from_list) - 9) + offset
    };
    // deposit item somewhere else
    let index_to = if to_list == "container" {
        to_i as u16
    } else {
        (to_inv_index(to_i as u16, &to_list) - 9) + offset
    };

    let Some(src_slot) = menu.slot(index_from as usize) else {
        // client tried to pick up an empty slot
        return;
    };
    // if we didn't pick up all items, right-click to pick up half
    let pickup_click = if src_slot.count() as u16 != count {
        PickupClick::Right {
            slot: Some(index_from),
        }
    } else {
        PickupClick::Left {
            slot: Some(index_from),
        }
    };
    drop(menu);

    // "container" only ever shows up while a container is open
    // the default inventory formspec (armor/craft/craftpreview/offhand/main) has no "container" list
    if from_list == "container" || to_list == "container" {
        // can't use the 2x2 crafting grid at the same time as a real container
        inventory_state.inventory_handle = None;
        let Ok(handle) = mc_client.get_inventory() else {
            error!("Client does not have an inventory component!");
            return;
        };
        handle.click(ClickOperation::Pickup(pickup_click));
        handle.click(ClickOperation::Pickup(PickupClick::Left {
            slot: Some(index_to),
        }));
    } else {
        // hold the handle open across calls in case we need the 2x2 grid again
        if inventory_state.inventory_handle.is_none()
            && let Ok(Some(new_handle)) = mc_client.open_inventory()
        {
            inventory_state.inventory_handle = Some(Arc::new(Mutex::new(new_handle)));
        }
        let Some(handle) = &inventory_state.inventory_handle else {
            error!("Failed to open the player inventory!");
            return;
        };
        let handle = handle.lock().unwrap();
        handle.click(ClickOperation::Pickup(pickup_click));
        handle.click(ClickOperation::Pickup(PickupClick::Left {
            slot: Some(index_to),
        }));
    }

    // unknown state, but not what it was before
    inventory_state.clientside_fields = Vec::new();
    s2c::inventory::refresh_inv(mc_client, luanti_conn, inventory_state, true).await;
}

pub async fn craft_item(
    mc_client: &mut Client,
    luanti_conn: &mut LuantiConnection,
    inventory_state: &mut state::InventoryState,
    count: u16,
    _craft_location: InventoryLocation,
) {
    // we are not deleting the inventory handle, as the user might click craft repeatedly
    match &inventory_state.inventory_handle {
        Some(arc_mtx_cht) => {
            let guard = arc_mtx_cht.lock();
            let handle = guard.unwrap();
            for _ in 0..count {
                handle.click(ClickOperation::Pickup(PickupClick::Left { slot: Some(0) }));
            }
        }
        None => {
            info!("Client attempted to craft without a present inventory handle!");
        }
    }
    s2c::inventory::refresh_inv(mc_client, luanti_conn, inventory_state, true).await;
}
