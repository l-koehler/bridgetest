use azalea::container::{ContainerClientExt, ContainerHandle, ContainerHandleRef};
use azalea::inventory::operations::{ClickOperation, PickupClick, ThrowClick};
use azalea::protocol::packets::game::ServerboundSetCarriedItem;
use azalea_client::Client;
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
        } => move_item(
            count,
            from_list,
            from_i,
            to_list,
            to_i,
            mc_client,
            inventory_state,
        ),
        //TODO support workbenches etc
        InventoryAction::Craft { count, craft_inv } => {
            craft_item(mc_client, luanti_conn, inventory_state, count, craft_inv).await
        }
    }
}

// see https://wiki.vg/File:Inventory-slots.png for full indexing of the player inv
fn get_adjusted_index(mt_index: u16, mt_list: &str) -> u16 {
    match mt_list {
        "armor" => mt_index + 5,
        "craft" => mt_index + 1,
        "craftpreview" => 0,
        "offhand" => 45,
        "main" => match mt_index {
            0..=8 => (mt_index % 36) + 36,
            9..=17 => ((mt_index - 9) % 36) + 9,
            18..=26 => mt_index % 36,
            27.. => mt_index,
        },
        _ => panic!("Unknown Inventory List: {}", mt_list), // unreachable unless the mt client sends bad data
    }
}

pub fn drop_item(count: u16, from_list: String, from_i: i16, mc_client: &mut Client) {
    match from_list.as_str() {
        "container" => {
            let maybe_handle = mc_client.get_open_container();
            if maybe_handle.is_none() {
                info!(
                    "[Minetest] Client attempted to drop items from a container while no container was opened"
                );
                return;
            }
            let handle = maybe_handle.unwrap();
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
            let maybe_handle = mc_client.open_inventory();
            if maybe_handle.is_none() {
                info!(
                    "[Minetest] Client attempted to drop items from the inventory while a container was opened",
                );
                return;
            }
            let handle = maybe_handle.unwrap();
            let slot_index = get_adjusted_index(from_i as u16, from_list.as_str());
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

fn pickupclick_c(handle: &ContainerHandleRef, index: i16, count: u16) {
    let is_full_stack = (count == handle.contents().unwrap()[index as usize].count() as u16);
    if is_full_stack {
        handle.click(ClickOperation::Pickup(PickupClick::Left {
            slot: Some(index as u16),
        }));
    } else {
        handle.click(ClickOperation::Pickup(PickupClick::Right {
            slot: Some(index as u16),
        }));
    }
}
fn pickupclick_i(handle: &ContainerHandle, index: u16, count: u16) {
    let is_full_stack = (count == handle.contents().unwrap()[index as usize].count() as u16);
    if is_full_stack {
        handle.click(ClickOperation::Pickup(PickupClick::Left {
            slot: Some(index),
        }));
    } else {
        handle.click(ClickOperation::Pickup(PickupClick::Right {
            slot: Some(index),
        }));
    }
}

pub fn move_item(
    count: u16,
    from_list: String,
    from_i: i16,
    to_list: String,
    to_i: Option<i16>,
    mc_client: &mut Client,
    inventory_state: &mut state::InventoryState,
) {
    match from_list.as_str() {
        "container" => {
            let maybe_handle = mc_client.get_open_container();
            if maybe_handle.is_none() {
                info!(
                    "Client attempted to take items from a container while no container was opened"
                );
                return;
            }
            let handle = maybe_handle.unwrap();
            if handle.contents().is_none() {
                info!("Client attempted to take items from a container without contents");
                return;
            }
            pickupclick_c(&handle, from_i, count);
        }
        _ => {
            let index_from = get_adjusted_index(from_i as u16, from_list.as_str());
            match &inventory_state.inventory_handle {
                Some(arc_mtx_cht) => {
                    let guard = arc_mtx_cht.lock();
                    let handle = guard.unwrap();
                    pickupclick_i(&handle, index_from, count);
                }
                None => {
                    let maybe_handle = mc_client.open_inventory();
                    if maybe_handle.is_none() {
                        info!("Client attempted something silly");
                        return;
                    }
                    let handle = maybe_handle.unwrap();
                    pickupclick_i(&handle, index_from, count);
                }
            }
        }
    }
    match to_list.as_str() {
        "container" => {
            let maybe_handle = mc_client.get_open_container();
            if maybe_handle.is_none() {
                info!(
                    "Client attempted to take items from a container while no container was opened"
                );
                return;
            }
            let handle = maybe_handle.unwrap();
            if handle.contents().is_none() {
                info!("Client attempted to take items from a container without contents");
                return;
            }
            handle.click(ClickOperation::Pickup(PickupClick::Left {
                slot: Some(to_i.unwrap() as u16),
            }));
        }
        _ => {
            let index_to = get_adjusted_index(to_i.unwrap() as u16, to_list.as_str());
            match &inventory_state.inventory_handle {
                Some(arc_mtx_cht) => {
                    let guard = arc_mtx_cht.lock();
                    let handle = guard.unwrap();
                    handle.click(ClickOperation::Pickup(PickupClick::Left {
                        slot: Some(index_to),
                    }));
                }
                None => {
                    let maybe_handle = mc_client.open_inventory();
                    if maybe_handle.is_none() {
                        info!("Client attempted something silly");
                        return;
                    }
                    let handle = maybe_handle.unwrap();
                    handle.click(ClickOperation::Pickup(PickupClick::Left {
                        slot: Some(index_to),
                    }));
                    // we moved a item into the crafting slots, keep the handle around so the inventory won't close
                    // the handle will get dropped on movement as the MT client doesn't notify us of closing the inventory
                    if (1..=5).contains(&index_to) {
                        inventory_state.inventory_handle = Some(Arc::new(Mutex::new(handle)));
                    }
                }
            }
        }
    }
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
    s2c::inventory::refresh_inv(mc_client, luanti_conn, inventory_state).await;
}
