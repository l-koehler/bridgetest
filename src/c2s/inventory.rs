use azalea::Client;
use azalea::container::ContainerHandleRef;
use azalea::inventory::operations::{ClickOperation, PickupClick, QuickMoveClick, ThrowClick};
use azalea::protocol::packets::game::ServerboundSetCarriedItem;
use log::*;

use luanti_protocol::LuantiConnection;
use luanti_protocol::commands::client_to_server::{
    InventoryActionSpec, InventoryFieldsSpec, PlayerItemSpec,
};
use luanti_protocol::types::{InventoryAction, InventoryLocation};

use std::sync::{Arc, Mutex};

use crate::s2c;
use crate::state;

pub fn set_mainhand(mc_client: &mut Client, specbox: Box<PlayerItemSpec>) {
    // hotbar_index: 0..8, first..last slot of hotbar
    let PlayerItemSpec { item: hotbar_index } = *specbox;
    let _ = mc_client.write_packet(ServerboundSetCarriedItem { slot: hotbar_index });
}

// luanti sends this whenever any formspec is submitted or closed
pub fn handle_form_fields(
    mc_client: &mut Client,
    specbox: Box<InventoryFieldsSpec>,
    inventory_state: &mut state::InventoryState,
) {
    let InventoryFieldsSpec {
        client_formspec_name: _,
        fields,
    } = *specbox;
    // "quit" for closing the formspec, anything else is some other action
    if fields.iter().any(|(name, _)| name == "quit") {
        close_open_container(mc_client, inventory_state);
    }
}

pub fn close_open_container(mc_client: &mut Client, inventory_state: &mut state::InventoryState) {
    if inventory_state.container_id.take().is_some() {
        if let Ok(handle) = mc_client.get_inventory() {
            handle.close();
        }
    }
    // drop this to let azalea close our inventory/2x2-grid
    inventory_state.inventory_handle = None;
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
        } => drop_item(count, from_list, from_i, mc_client, inventory_state),
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

// see https://minecraft.wiki/w/Java_Edition_protocol/Packets?section=96#Set_Container_Content for full indexing of the player inv
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

// magically convert a luanti (list,index) thing into an index into the currently-open menu
// needs offset = player_slots_range().min()
fn shift_to_menu(list: &str, index: u16, offset: u16) -> u16 {
    if list == "container" {
        // implicitly correct by formspecs
        return index;
    }
    let raw = to_inv_index(index, list);
    if list == "main" {
        // -9 shift to main-only, since we have armor etc in separate lists for luanti
        (raw - 9) + offset
    } else {
        // craft/craftpreview/armor/offhand only ever appear together with
        // Menu::Player (no container open) - the containers/crafting-table
        // formspecs never show those lists. Menu::Player's own layout
        // already matches to_inv_index's indexing 1:1, so offset is always 9
        // here and no shift is needed. Shifting anyway would also underflow,
        // since these map to raw indices below 9 (e.g. "craftpreview" is
        // always 0).
        raw
    }
}

fn drop_via(handle: &ContainerHandleRef, slot_index: u16, slot_count: i32, count: u16) {
    if slot_count <= count.into() {
        handle.click(ClickOperation::Throw(ThrowClick::All { slot: slot_index }))
    } else {
        for _ in 0..count {
            handle.click(ClickOperation::Throw(ThrowClick::Single {
                slot: slot_index,
            }))
        }
    }
}

pub fn drop_item(
    count: u16,
    from_list: String,
    from_i: i16,
    mc_client: &mut Client,
    inventory_state: &state::InventoryState,
) {
    let Ok(menu) = mc_client.menu() else {
        error!("Client does not have an inventory component!");
        return;
    };
    let offset = menu.player_slots_range().min().unwrap() as u16;
    let slot_index = shift_to_menu(from_list.as_str(), from_i as u16, offset);
    let Some(slot) = menu.slot(slot_index as usize) else {
        return;
    };
    let slot_count = slot.count();
    drop(menu);
    if slot_count <= 0 {
        return;
    }

    // see move_item
    if inventory_state.container_id.is_some() {
        let Ok(handle) = mc_client.get_inventory() else {
            error!("Client does not have an inventory component!");
            return;
        };
        drop_via(&handle, slot_index, slot_count, count);
    } else {
        let Ok(Some(handle)) = mc_client.open_inventory() else {
            info!(
                "[Minetest] Client attempted to drop items from the inventory while a container was opened",
            );
            return;
        };
        drop_via(&handle, slot_index, slot_count, count);
    }
}

// minecraft doesn't let us move arbitrary amounts, so this function may need several clicks
fn perform_move(
    handle: &ContainerHandleRef,
    index_from: u16,
    index_to: u16,
    count: u16,
    src_count: u16,
) {
    handle.click(ClickOperation::Pickup(PickupClick::Left {
        slot: Some(index_from),
    }));
    if count >= src_count {
        // move entire stack
        handle.click(ClickOperation::Pickup(PickupClick::Left {
            slot: Some(index_to),
        }));
    } else {
        // deposit stack item-for item, put remainder back
        for _ in 0..count {
            handle.click(ClickOperation::Pickup(PickupClick::Right {
                slot: Some(index_to),
            }));
        }
        handle.click(ClickOperation::Pickup(PickupClick::Left {
            slot: Some(index_from),
        }));
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
    // translate luanti->minecraft indices
    let offset = menu.player_slots_range().min().unwrap() as u16;
    let index_from = shift_to_menu(from_list.as_str(), from_i as u16, offset);
    let index_to = shift_to_menu(to_list.as_str(), to_i as u16, offset);

    let Some(src_slot) = menu.slot(index_from as usize) else {
        // client tried to pick up an empty slot
        return;
    };
    let src_count = src_slot.count() as u16;
    drop(menu);
    // 0 to src_count items can be moved
    if src_count == 0 {
        return;
    }
    let count = count.min(src_count);

    // we have to use the container instead of our inventory when one is open
    // even when we just shuffle our inventory
    if inventory_state.container_id.is_some() {
        // can't use the 2x2 crafting grid at the same time as a real container
        inventory_state.inventory_handle = None;
        let Ok(handle) = mc_client.get_inventory() else {
            error!("Client does not have an inventory component!");
            return;
        };
        perform_move(&handle, index_from, index_to, count, src_count);
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
        perform_move(&handle, index_from, index_to, count, src_count);
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
    // the crafting result is in slot 0
    if inventory_state.container_id.is_some() {
        match mc_client.get_inventory() {
            Ok(handle) if handle.id() != 0 => {
                for _ in 0..count {
                    handle.click(ClickOperation::QuickMove(QuickMoveClick::Left { slot: 0 }));
                }
            }
            _ => warn!("Client attempted to craft, but its container isn't open anymore!"),
        }
    } else {
        // we are not deleting the inventory handle, as the user might click craft repeatedly
        match &inventory_state.inventory_handle {
            Some(arc_mtx_cht) => {
                let guard = arc_mtx_cht.lock();
                let handle = guard.unwrap();
                for _ in 0..count {
                    handle.click(ClickOperation::QuickMove(QuickMoveClick::Left { slot: 0 }));
                }
            }
            None => {
                info!("Client attempted to craft without a present inventory handle!");
            }
        }
    }
    s2c::inventory::refresh_inv(mc_client, luanti_conn, inventory_state, true).await;
}
