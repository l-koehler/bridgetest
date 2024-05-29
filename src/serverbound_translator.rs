use crate::mt_definitions::Dimensions;
// this contains functions that TAKE data from the client
// and send it to the MC server.
use crate::{clientbound_translator, mt_definitions, utils};

use azalea::inventory::operations::{ClickOperation, ThrowClick};
use azalea_client::Client;
use azalea_client::inventory::InventoryComponent;
use azalea_core::position::{ChunkPos, ChunkBlockPos};
use azalea_block::BlockState;
use azalea::container::{ContainerClientExt, ContainerHandleRef};

use alloc::boxed::Box;
use minetest_protocol::MinetestConnection;
use minetest_protocol::wire::command::{TSChatMessageSpec, PlayerposSpec, InteractSpec, GotblocksSpec, PlayeritemSpec, InventoryActionSpec};
use minetest_protocol::wire::types::{v3f, v3s16, InventoryAction, PlayerPos, PointedThing, InventoryLocation};
use minetest_protocol::wire::types;
use crate::MTServerState;


pub fn send_message(mc_client: &Client, specbox: Box<TSChatMessageSpec>) {
    utils::logger("[Minetest] C->S Forwarding Message sent by client", 1);
    let TSChatMessageSpec { message } = *specbox;
    mc_client.chat(&message);
}

pub async fn playerpos(mc_client: &mut Client, specbox: Box<PlayerposSpec>, mt_server_state: &mut MTServerState) {
    let PlayerposSpec { player_pos } = *specbox;
    let PlayerPos { position, speed: _, pitch, yaw, keys_pressed, fov: _, wanted_range: _ } = player_pos;
    let v3f {x: mt_x, y: mt_y, z: mt_z } = position;
    mt_server_state.mt_clientside_pos = (mt_x, mt_y, mt_z);
    mt_server_state.mt_clientside_rot = (yaw, pitch);

    // keys_pressed:
    // https://github.com/minetest/minetest/blob/e734b3f0d8055ff3ae710f3632726a711603bf84/src/player.cpp#L217    
    let direction_keys = keys_pressed & 0xf;
    let up_pressed    = direction_keys & 1;
    let down_pressed  = (direction_keys >> 1) & 1;
    let left_pressed  = (direction_keys >> 2) & 1;
    let right_pressed = (direction_keys >> 3) & 1;

    let jump_pressed  = (keys_pressed & (1 << 4)) != 0;
    let aux1_pressed  = keys_pressed & (1 << 5);
    let sneak_pressed = (keys_pressed & (1 << 6)) != 0;
    let _dig_pressed   = (keys_pressed & (1 << 7)) != 0;
    let _place_pressed = (keys_pressed & (1 << 8)) != 0;
    let _zoom_pressed  = (keys_pressed & (1 << 9)) != 0;

    if keys_pressed != mt_server_state.keys_pressed {
        // always sync rotation over to MC before moving
        // this is also the only occasion where rotation will be
        // sent to the server, as to minimize "rubberbanding"
        // with rotation.
        mc_client.set_direction(yaw, pitch);
        match (aux1_pressed, up_pressed, down_pressed, left_pressed, right_pressed) {
            (0, 1, 0, 1, 0) => mc_client.walk(azalea::WalkDirection::ForwardLeft),
            (0, 1, 0, 0, 1) => mc_client.walk(azalea::WalkDirection::ForwardRight),
            (0, 1, 0, _, _) => mc_client.walk(azalea::WalkDirection::Forward),
            (0, 0, 1, 1, 0) => mc_client.walk(azalea::WalkDirection::BackwardLeft),
            (0, 0, 1, 0, 1) => mc_client.walk(azalea::WalkDirection::BackwardRight),
            (0, 0, 1, _, _) => mc_client.walk(azalea::WalkDirection::Backward),
            (0, _, _, 1, 0) => mc_client.walk(azalea::WalkDirection::Left),
            (0, _, _, 0, 1) => mc_client.walk(azalea::WalkDirection::Right),
            (1, 1, 0, 1, 0) => mc_client.sprint(azalea::SprintDirection::ForwardLeft),
            (1, 1, 0, 0, 1) => mc_client.sprint(azalea::SprintDirection::ForwardRight),
            (1, 1, 0, _, _) => mc_client.sprint(azalea::SprintDirection::Forward),
            _ => mc_client.walk(azalea::WalkDirection::None),
        }
        mt_server_state.keys_pressed = keys_pressed;
    }

    mc_client.set_jumping(jump_pressed);

    if mt_server_state.is_sneaking != sneak_pressed {
        // player started/stopped sneaking, update the mc client
        // TODO: not added to azalea yet, check if this is still accurate:
        // https://github.com/azalea-rs/azalea/commits/sneaking
    };
}

pub fn set_mainhand(mc_client: &mut Client, specbox: Box<PlayeritemSpec>) {
    // hotbar_index: 0..8, first..last slot of hotbar
    let PlayeritemSpec { item: hotbar_index } = *specbox;
    let mut ecs = mc_client.ecs.lock();
    let mut inventory = mc_client.query::<&mut InventoryComponent>(&mut ecs);
    inventory.selected_hotbar_slot = hotbar_index as u8;
    drop(ecs);
}

// This function only validates the interaction, then splits by node/object
pub async fn interact_generic(conn: &mut MinetestConnection, mc_client: &mut Client, specbox: Box<InteractSpec>, mt_server_state: &mut MTServerState) {
    let InteractSpec { action, item_index: _, pointed_thing, player_pos: _ } = *specbox;
    match pointed_thing {
        PointedThing::Nothing => (), // TODO might still be relevant in some cases (eating), check that
        PointedThing::Node { under_surface, above_surface } => interact_node(conn, action, under_surface, above_surface, mc_client, mt_server_state).await,
        PointedThing::Object { object_id } => interact_object(action, object_id, mc_client).await,
    }
}

async fn interact_object(action: types::InteractAction, object_id: u16, mc_client: &mut Client) {
    match action {
        types::InteractAction::Use => mc_client.attack(azalea_world::MinecraftEntityId(object_id.into())),
        _ => utils::logger(&format!("[Minetest] Client sent unsupported entity interaction: {:?} (entity ID: {})", action, object_id), 2)
    }
}

fn interact_air(action: types::InteractAction, mc_client: &mut Client) {
    match action {
        types::InteractAction::Use => mc_client.block_interact(mc_client.position().into()),
        _ => () // cant place with PointedThing::Nothing
    }
}

fn stop_digging(mc_client: &mut Client) {
    // HACK: azalea does not seem to have a proper way to do this.
    // mining a block that is out-of-range should cancel any current mining
    // (and trigger anticheats)
    mc_client.start_mining(azalea::BlockPos { x: 0, y: 1000, z: 0 })
}

async fn node_rightclick(conn: &mut MinetestConnection, mc_client: &mut Client, under: azalea::BlockPos, above: azalea::BlockPos, mt_server_state: &mut MTServerState) {
    let under_key: (i32, i32, i32) = (under.x, under.y, under.z);
    if mt_server_state.container_map.contains_key(&under_key) {
        let diff =
            ((under.x as f64) - mc_client.position().x).abs() +
            ((under.y as f64) - mc_client.position().y).abs() +
            ((under.z as f64) - mc_client.position().z).abs();
        utils::logger(&format!("[Minetest] Attempting to open Chest at {}, Client is {} blocks away.", under, diff), 1);
        mc_client.block_interact(under)
    } else {
        mc_client.block_interact(above)
    }
}

async fn interact_node(conn: &mut MinetestConnection, action: types::InteractAction, under_surface: v3s16, above_surface: v3s16, mc_client: &mut Client, mt_server_state: &mut MTServerState) {
    let under_blockpos = azalea::BlockPos { x: under_surface.x.into(), y: under_surface.y.into(), z: under_surface.z.into() };
    let above_blockpos = azalea::BlockPos { x: above_surface.x.into(), y: above_surface.y.into(), z: above_surface.z.into() };
    match action {
        types::InteractAction::StartDigging => mc_client.start_mining(under_blockpos),
        types::InteractAction::StopDigging  => stop_digging(mc_client),
        // using a node needs the position of the node that was clicked
        types::InteractAction::Place        => node_rightclick(conn, mc_client, under_blockpos, above_blockpos, mt_server_state).await,
        _ => utils::logger(&format!("[Minetest] Client sent unsupported node interaction: {:?}", action), 2)
    }
}

pub async fn gotblocks(mc_client: &mut Client, specbox: Box<GotblocksSpec>, mt_conn: &MinetestConnection, current_dimension: mt_definitions::Dimensions) {
    let partial_world = mc_client.partial_world();
    let world_data = partial_world.read();
    for to_send in specbox.blocks {
        let fullheight = world_data.chunks.limited_get(&ChunkPos::new(to_send.x.into(), to_send.z.into()));
        match fullheight {
            Some(chunk_data) => {
                // copying some stuff from clientbound_translator::send_level_chunk
                let mut nodearr: [BlockState; 4096] = [BlockState{id:125};4096];
                let block_y = to_send.y * 16;
                for y in block_y..block_y+16 {
                    for x in 0..16 {
                        for z in 0..16 {
                            let current_state = chunk_data.read().get(&ChunkBlockPos { x: x as u8, y: y as i32, z: z as u8 },
                                                                      mt_definitions::get_y_bounds(&current_dimension).0.into());
                            match current_state {
                                Some(state) => nodearr[x+((y%16) as usize*16)+(z*256)] = state,
                                // Air for unknown Nodes. The existance of the chunk was ensured previously.
                                None => nodearr[x+((y%16) as usize*16)+(z*256)] = BlockState{id:125}
                            }
                        }
                    }
                }
                // call the clientbound translator to send the created node array
                let cave_air_glow = current_dimension == Dimensions::Nether;
                clientbound_translator::initialize_16node_chunk(to_send.x, to_send.y, to_send.z,
                                                                mt_conn, nodearr, cave_air_glow).await;
            },
            // TODO can i request the chunk from the server?
            None => utils::logger(&format!("[Minetest] Client requested {:?}, but the ECS is not aware of this chunk.", to_send), 2),
        }
    }
}

// inventory actions and crafting
pub async fn inventory_generic(conn: &mut MinetestConnection, mc_client: &mut Client, specbox: Box<InventoryActionSpec>, mt_server_state: &mut MTServerState) {
    let InventoryActionSpec { action } = *specbox;
    match action {
        InventoryAction::Drop { count, from_inv, from_list, from_i }
            => drop_item(count, from_list, from_i, mc_client),
        InventoryAction::Move { count, from_inv, from_list, from_i, to_inv, to_list, to_i } if matches!(from_inv, InventoryLocation::CurrentPlayer)
            => move_item(count, from_inv, from_list, from_i, to_inv, to_list, to_i, mc_client),
        _ => utils::logger(&format!("[Minetest] Client attempted unsupported inventory action: {:?}", action), 2),
    }
}

// see https://wiki.vg/File:Inventory-slots.png for full indexing of the player inv
fn get_adjusted_index(mt_index: u16, mt_list: &str) -> u16 {
    match mt_list {
        "armor" => mt_index + 5,
        "crafting" => mt_index + 1,
        "craftpreview" => 0,
        "offhand" => 45,
        "main" => {
            match mt_index {
                0..=8   => (mt_index-9 % 36) + 36,
                9..=17  => (mt_index-9 % 36) + 18,
                18..=26 =>  mt_index-9 % 36,
                27..    => (mt_index-9 % 36) - 17,
            }
        }
        _ => unreachable!()
    }
}

pub fn drop_item(count: u16, from_list: String, from_i: i16, mc_client: &mut Client) {
    match from_list.as_str() {
        "container" => {
            let maybe_handle = mc_client.get_open_container();
            if maybe_handle.is_none() {
                utils::logger("[Minetest] Client attempted to drop items from a container while no container was opened", 2);
                return;
            }
            let handle = maybe_handle.unwrap();
            if handle.contents().is_none() {
                utils::logger("[Minetest] Client attempted to drop items from a container without contents", 2);
                return;
            }
            if handle.contents().unwrap()[from_i as usize].count() <= count.into() {
                handle.click(ClickOperation::Throw(ThrowClick::All { slot: from_i as u16 }))
            } else {
                while handle.contents().unwrap()[from_i as usize].count() > 0 {
                    handle.click(ClickOperation::Throw(ThrowClick::Single { slot: from_i as u16 }))
                }
            }
        },
        "main" | "armor" | "offhand" | "craft" | "craftpreview" => {
            let maybe_handle = mc_client.open_inventory();
            if maybe_handle.is_none() {
                utils::logger("[Minetest] Client attempted to drop items from the inventory while a container was opened", 2);
                return;
            }
            let handle = maybe_handle.unwrap();
            let slot_index = get_adjusted_index(from_i as u16, from_list.as_str());
            if handle.contents().unwrap()[slot_index as usize].count() <= count.into() {
                handle.click(ClickOperation::Throw(ThrowClick::All { slot: slot_index }))
            } else {
                while handle.contents().unwrap()[slot_index as usize].count() > 0 {
                    handle.click(ClickOperation::Throw(ThrowClick::Single { slot: slot_index }))
                }
            }

        }
        _ => unreachable!()
    }
}


pub fn move_item(count: u16, from_inv: InventoryLocation, from_list: String, from_i: i16, to_inv: InventoryLocation, to_list: String, to_i: Option<i16>, mc_client: &mut Client) {
    match (from_list.as_str(), to_list.as_str()) {
        ("container", "container") => {
            let maybe_handle = mc_client.get_open_container();
            if maybe_handle.is_none() {
                utils::logger("[Minetest] Client attempted to take items from a container while no container was opened", 2);
                return;
            }
            let handle = maybe_handle.unwrap();
            if handle.contents().is_none() {
                utils::logger("[Minetest] Client attempted to take items from a container without contents", 2);
                return;
            }
            handle.click(ClickOperation::Pickup(azalea::inventory::operations::PickupClick::Left {
                slot: Some(from_i as u16)
            }));
            handle.click(ClickOperation::Pickup(azalea::inventory::operations::PickupClick::Left {
                slot: Some(to_i.unwrap() as u16)
            }))
        }
        ("container", _) => {
            let maybe_handle = mc_client.get_open_container();
            if maybe_handle.is_none() {
                utils::logger("[Minetest] Client attempted to take items from a container while no container was opened", 2);
                return;
            }
            let handle = maybe_handle.unwrap();
            if handle.contents().is_none() {
                utils::logger("[Minetest] Client attempted to take items from a container without contents", 2);
                return;
            }
            handle.click(ClickOperation::Pickup(azalea::inventory::operations::PickupClick::Left {
                slot: Some(from_i as u16)
            }));
            let maybe_handle = mc_client.open_inventory();
            if maybe_handle.is_none() {
                utils::logger("[Minetest] Client attempted to put items into the inventory while a container was opened", 2);
                return;
            }
            let handle = maybe_handle.unwrap();
            let slot_index = get_adjusted_index(from_i as u16, to_list.as_str());
            handle.click(ClickOperation::Pickup(azalea::inventory::operations::PickupClick::Left {
                slot: Some(slot_index)
            }))
        }
        (_, "container") => {
            let maybe_handle = mc_client.open_inventory();
            if maybe_handle.is_none() {
                utils::logger("[Minetest] Client attempted to put items into the inventory while a container was opened", 2);
                return;
            }
            let handle = maybe_handle.unwrap();
            let slot_index = get_adjusted_index(from_i as u16, from_list.as_str());
            handle.click(ClickOperation::Pickup(azalea::inventory::operations::PickupClick::Left {
                slot: Some(slot_index)
            }));
            let maybe_handle = mc_client.get_open_container();
            if maybe_handle.is_none() {
                utils::logger("[Minetest] Client attempted to put items into a container while no container was opened", 2);
                return;
            }
            let handle = maybe_handle.unwrap();
            if handle.contents().is_none() {
                utils::logger("[Minetest] Client attempted to put items into a container without contents", 2);
                return;
            }
            handle.click(ClickOperation::Pickup(azalea::inventory::operations::PickupClick::Left {
                slot: Some(to_i.unwrap() as u16)
            }));
        }
        _ => {
            let maybe_handle = mc_client.open_inventory();
            if maybe_handle.is_none() {
                utils::logger("[Minetest] Client attempted to put items into the inventory while a container was opened", 2);
                return;
            }
            let handle = maybe_handle.unwrap();
            let slot_index = get_adjusted_index(from_i as u16, from_list.as_str());
            handle.click(ClickOperation::Pickup(azalea::inventory::operations::PickupClick::Left {
                slot: Some(slot_index)
            }));
            let slot_index = get_adjusted_index(to_i.unwrap() as u16, to_list.as_str());
            handle.click(ClickOperation::Pickup(azalea::inventory::operations::PickupClick::Left {
                slot: Some(slot_index)
            }));
        },
    }
}
