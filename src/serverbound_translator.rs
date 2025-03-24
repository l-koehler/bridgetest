// this contains functions that TAKE data from the client
// and send it to the MC server.
use crate::{clientbound_translator, utils};

use azalea::inventory::operations::{ClickOperation, PickupClick, ThrowClick};
use azalea::Vec3;
use azalea::world::{InstanceName, MinecraftEntityId};
use azalea::entity::{metadata::AbstractEntity, Dead, LocalEntity, Position, Physics};
use azalea::ecs::prelude::{With, Without};
use azalea_client::Client;
use azalea_client::inventory::Inventory;
use azalea::container::ContainerClientExt;

use minetest_protocol::MinetestConnection;
use minetest_protocol::wire::command::{TSChatMessageSpec, PlayerposSpec, InteractSpec, PlayeritemSpec, InventoryActionSpec};
use minetest_protocol::wire::types::{v3s16, InventoryAction, PlayerPos, PointedThing};
use minetest_protocol::wire::types;
use crate::MTServerState;

use std::sync::{Arc, Mutex};
use std::f32::consts::PI;

pub fn send_message(mc_client: &Client, specbox: Box<TSChatMessageSpec>) {
    utils::logger("[Minetest] C->S Forwarding Message sent by client", 1);
    let TSChatMessageSpec { message } = *specbox;
    mc_client.chat(&message);
}

pub async fn playerpos(mc_client: &mut Client, specbox: Box<PlayerposSpec>, mt_server_state: &mut MTServerState) {
    // the player moved, if a handle to the inventory is kept we may now drop it.
    // this is needed as (unlike the minecraft client) the minetest client does not seem to send packets on container close
    mt_server_state.inventory_handle = None;

    let PlayerposSpec { player_pos } = *specbox;
    let PlayerPos { position, speed: _, pitch, yaw, keys_pressed, fov: _, wanted_range: _ } = player_pos;

    mc_client.set_direction(yaw, pitch);
    mt_server_state.client_rotation = (yaw, pitch);
    // all coordinates from/to the minetest client are/have to be *10 for some reason
    mt_server_state.mt_clientside_pos = (position.x/10.0, position.y/10.0, position.z/10.0);

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
    let dig_pressed   = (keys_pressed & (1 << 7)) != 0;
    let _place_pressed = (keys_pressed & (1 << 8)) != 0;
    let _zoom_pressed  = (keys_pressed & (1 << 9)) != 0;

    if (direction_keys, aux1_pressed, jump_pressed) != (0, 32, false) {
        mt_server_state.has_moved_since_sync = true;
    }
    
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
            // bitmasking behavior makes this 32/0 instad of 1/0
            (32, 1, 0, 1, 0) => mc_client.sprint(azalea::SprintDirection::ForwardLeft),
            (32, 1, 0, 0, 1) => mc_client.sprint(azalea::SprintDirection::ForwardRight),
            (32, 1, 0, _, _) => mc_client.sprint(azalea::SprintDirection::Forward),
            _ => mc_client.walk(azalea::WalkDirection::None),
        }
        mt_server_state.keys_pressed = keys_pressed;
    }

    mc_client.set_jumping(jump_pressed);

    if mt_server_state.is_sneaking != sneak_pressed {
        mt_server_state.is_sneaking = sneak_pressed
        // player started/stopped sneaking, update the mc client
        // TODO: not added to azalea yet, check if this is still accurate:
        // https://github.com/azalea-rs/azalea/commits/sneaking
        // currently just changes client-side speed, but resyncing makes the player move at normal speed regardless
    };
    
    if !mt_server_state.next_click_no_attack && dig_pressed && !mt_server_state.previous_dig_held {
        attack_crosshair(mc_client);
    }
    
    // if we previously already let go of the button and didn't press it right now either, reset next_no_atk
    if !mt_server_state.previous_dig_held && !dig_pressed {
        mt_server_state.next_click_no_attack = false;
    }
    
    mt_server_state.previous_dig_held = dig_pressed
}

pub fn attack_crosshair(mc_client: &mut Client) {
    let line_origin = mc_client.eye_position();
    let client_instance_name = mc_client.component::<InstanceName>();
    // convert to radians
    let (mut yaw, mut pitch) = mc_client.direction();
    yaw   = utils::normalize_angle(yaw)   * (PI/180.0);
    pitch = utils::normalize_angle(pitch) * (PI/180.0);
    const MAX_DIST: f32 = 10.0;
    let dx = MAX_DIST * pitch.cos() * -yaw.sin();
    let dy = MAX_DIST * pitch.sin();
    let dz = MAX_DIST * pitch.cos() * yaw.cos();
    // Calculate the end point of the line
    let line_end = Vec3 {
        x: line_origin.x + dx as f64,
        y: line_origin.y + dy as f64,
        z: line_origin.z + dz as f64
    };
    // we now have a line-of-sight from line_origin (player head) to line_end
    // collect all entities in range
    let mut ecs = mc_client.ecs.lock();
    // MinecraftEntityId, distance_from_player
    let mut possible_entities: Vec<(MinecraftEntityId, f64)> = Vec::new();
    let mut query = ecs
        .query_filtered::<(&MinecraftEntityId, &Position, &InstanceName, &Physics), (
            With<AbstractEntity>,
            Without<LocalEntity>, // idk what this does but the "official" killaura example has this
            Without<Dead>,
        )>();
    for (&entity_id, position, instance_name, physics) in query.iter(&ecs) {
        if (*instance_name != client_instance_name) || (line_origin.distance_to(&position) > MAX_DIST.into()) {
            // fail early instead of failing with the slower liang–barsky algorithm later
            continue;
        }
        // check if the bounding box is on the line-of-sight
        let bounding_box = physics.bounding_box;
        if utils::liang_barsky_3d(bounding_box, line_origin, line_end) {
            possible_entities.push(
                (entity_id, line_origin.distance_to(&position))
            )
        }
    }
    drop(ecs);
    // either none or Some((minecraftentityid, distance_to_player))
    let closest_entity = possible_entities.iter().min_by(|x, y| x.1.total_cmp(&y.1));
    if let Some(closest_entity) = closest_entity {
        mc_client.attack(closest_entity.0)
    }
}

pub fn set_mainhand(mc_client: &mut Client, specbox: Box<PlayeritemSpec>) {
    // hotbar_index: 0..8, first..last slot of hotbar
    let PlayeritemSpec { item: hotbar_index } = *specbox;
    let mut ecs = mc_client.ecs.lock();
    let mut inventory = mc_client.query::<&mut Inventory>(&mut ecs);
    inventory.selected_hotbar_slot = hotbar_index as u8;
    drop(ecs);
}

// This function only validates the interaction, then splits by node/object
pub async fn interact_generic(conn: &mut MinetestConnection, mc_client: &mut Client, specbox: Box<InteractSpec>, mt_server_state: &mut MTServerState) {
    let InteractSpec { action, item_index: _, pointed_thing, player_pos: _ } = *specbox;
    match pointed_thing {
        PointedThing::Nothing => (), // TODO might still be relevant in some cases (eating), check that
        PointedThing::Node { under_surface, above_surface } => interact_node(conn, action, under_surface, above_surface, mc_client, mt_server_state).await,
        _ => utils::logger("[Minetest] Interacting with objects is currently unsupported/done some other hacky way!", 2)
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
        types::InteractAction::StartDigging => {
            // declare that this button press wasn't for attacking, rather for mining
            // whenever that is set to false, "dig_pressed" switching to true will trigger an attack
            mt_server_state.next_click_no_attack = true;
            mc_client.start_mining(under_blockpos);
        },
        types::InteractAction::StopDigging  => stop_digging(mc_client),
        // using a node needs the position of the node that was clicked
        types::InteractAction::Place        => node_rightclick(conn, mc_client, under_blockpos, above_blockpos, mt_server_state).await,
        _ => utils::logger(&format!("[Minetest] Client sent unsupported node interaction: {:?}", action), 2)
    }
}

// inventory actions and crafting
pub async fn inventory_generic(mc_client: &mut Client, mt_conn: &mut MinetestConnection, specbox: Box<InventoryActionSpec>, mt_server_state: &mut MTServerState) {
    let InventoryActionSpec { action } = *specbox;
    match action {
        InventoryAction::Drop { count, from_inv: _, from_list, from_i }
            => drop_item(count, from_list, from_i, mc_client),
        InventoryAction::Move { count: _, from_inv: _, from_list, from_i, to_inv: _, to_list, to_i }
            => move_item(from_list, from_i, to_list, to_i, mc_client, mt_server_state),
        //TODO support workbenches etc
        InventoryAction::Craft { count: _, craft_inv: _ }
            => craft_item(mc_client, mt_conn, mt_server_state).await,
    }
}

// see https://wiki.vg/File:Inventory-slots.png for full indexing of the player inv
fn get_adjusted_index(mt_index: u16, mt_list: &str) -> u16 {
    match mt_list {
        "armor" => mt_index + 5,
        "craft" => mt_index + 1,
        "craftpreview" => 0,
        "offhand" => 45,
        "main" => {
            match mt_index {
                0..=8   => (mt_index % 36) + 36,
                9..=17  => ((mt_index - 9) % 36) + 9,
                18..=26 => mt_index % 36,
                27..    => mt_index,
            }
        }
        _ => panic!("Unknown Inventory List: {}", mt_list) // unreachable unless the mt client sends bad data
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


pub fn move_item(from_list: String, from_i: i16, to_list: String, to_i: Option<i16>, mc_client: &mut Client, mt_server_state: &mut MTServerState) {
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
            handle.click(ClickOperation::Pickup(PickupClick::Left {
                slot: Some(from_i as u16)
            }));
            handle.click(ClickOperation::Pickup(PickupClick::Left {
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
            handle.click(ClickOperation::Pickup(PickupClick::Left {
                slot: Some(from_i as u16)
            }));
            let maybe_handle = mc_client.open_inventory();
            if maybe_handle.is_none() {
                utils::logger("[Minetest] Client attempted to put items into the inventory while a container was opened", 2);
                return;
            }
            let handle = maybe_handle.unwrap();
            let slot_index = get_adjusted_index(from_i as u16, to_list.as_str());
            handle.click(ClickOperation::Pickup(PickupClick::Left {
                slot: Some(slot_index)
            }));
            // we moved a item into the crafting slots, keep the handle around so the inventory won't close
            if (1..=5).contains(&slot_index) && mt_server_state.inventory_handle.is_none() {
                mt_server_state.inventory_handle = Some(Arc::new(Mutex::new(handle)));
            }
        }
        (_, "container") => {
            let maybe_handle = mc_client.open_inventory();
            if maybe_handle.is_none() {
                utils::logger("[Minetest] Client attempted to put items into the inventory while a container was opened", 2);
                return;
            }
            let handle = maybe_handle.unwrap();
            let slot_index = get_adjusted_index(from_i as u16, from_list.as_str());
            handle.click(ClickOperation::Pickup(PickupClick::Left {
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
            handle.click(ClickOperation::Pickup(PickupClick::Left {
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
            handle.click(ClickOperation::Pickup(PickupClick::Left {
                slot: Some(slot_index)
            }));
            let slot_index = get_adjusted_index(to_i.unwrap() as u16, to_list.as_str());
            handle.click(ClickOperation::Pickup(PickupClick::Left {
                slot: Some(slot_index)
            }));
            // we moved a item into the crafting slots, keep the handle around so the inventory won't close
            // the handle will get dropped on movement as the MT client doesn't notify us of closing the inventory
            if (1..=5).contains(&slot_index) && mt_server_state.inventory_handle.is_none() {
                mt_server_state.inventory_handle = Some(Arc::new(Mutex::new(handle)));
            }
        },
    }
}

pub async fn craft_item(mc_client: &mut Client, mt_conn: &mut MinetestConnection, mt_server_state: &mut MTServerState) {
    // we are not deleting the inventory handle, as the user might click craft repeatedly
    match &mt_server_state.inventory_handle {
        Some(arc_mtx_cht) => {
            let guard = arc_mtx_cht.lock();
            let handle = guard.unwrap();
            handle.click(ClickOperation::Pickup(PickupClick::Left {
                slot: Some(0)
            }));
        },
        None => {
            utils::logger("[Minetest] Client attempted to craft without a present inventory handle!", 2);
        },
    }
    clientbound_translator::refresh_inv(mc_client, mt_conn, mt_server_state).await;
}
