use azalea::Client;
use azalea::Vec3;
use azalea::ecs::prelude::Without;
use azalea::entity::{Dead, LocalEntity, Physics, Position};
use log::*;

use crate::c2s;
use crate::state;
use crate::utils;
use luanti_protocol::commands::client_to_server::{InteractSpec, PlayerPosCommand};
use luanti_protocol::types::{PlayerPos, PointedThing};

use std::f32::consts::PI;

pub async fn playerpos(
    mc_client: &mut Client,
    specbox: Box<PlayerPosCommand>,
    player_state: &mut state::PlayerState,
) {
    let PlayerPosCommand { player_pos } = *specbox;
    let PlayerPos {
        position,
        speed: _,
        pitch,
        yaw,
        keys_pressed,
        fov: _,
        wanted_range: _,
        camera_inverted: _,
        movement_speed: _,
        movement_direction: _,
    } = player_pos;

    // x handedness needs conversion
    let mc_yaw = utils::mirror_yaw(yaw);
    let _ = mc_client.set_direction(mc_yaw, pitch);
    player_state.client_rotation = (yaw, pitch);
    // all coordinates from/to the minetest client are *10 on the wire
    player_state.mt_clientside_pos = (position.x / 10.0, position.y / 10.0, position.z / 10.0);

    // keys_pressed:
    // https://github.com/minetest/minetest/blob/e734b3f0d8055ff3ae710f3632726a711603bf84/src/player.cpp#L217
    let direction_keys = keys_pressed & 0xf;
    let up_pressed = direction_keys & 1;
    let down_pressed = (direction_keys >> 1) & 1;
    let left_pressed = (direction_keys >> 2) & 1;
    let right_pressed = (direction_keys >> 3) & 1;

    let jump_pressed = (keys_pressed & (1 << 4)) != 0;
    let aux1_pressed = keys_pressed & (1 << 5);
    let sneak_pressed = (keys_pressed & (1 << 6)) != 0;
    let dig_pressed = (keys_pressed & (1 << 7)) != 0;
    let _place_pressed = (keys_pressed & (1 << 8)) != 0;
    let _zoom_pressed = (keys_pressed & (1 << 9)) != 0;

    if (direction_keys, aux1_pressed, jump_pressed) != (0, 32, false) {
        player_state.has_moved_since_sync = true;
    }

    if keys_pressed != player_state.keys_pressed {
        // always sync rotation over to MC before moving
        // this is also the only occasion where rotation will be
        // sent to the server, as to minimize "rubberbanding"
        // with rotation.
        let _ = mc_client.set_direction(mc_yaw, pitch);
        // left/right intentionally not swapped despite X-handedness being swapped
        // this is fine
        match (
            aux1_pressed,
            up_pressed,
            down_pressed,
            left_pressed,
            right_pressed,
        ) {
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
        player_state.keys_pressed = keys_pressed;
    }

    let _ = mc_client.set_jumping(jump_pressed);

    if player_state.is_sneaking != sneak_pressed {
        player_state.is_sneaking = sneak_pressed;
        // player started/stopped sneaking, update the mc client
        let _ = mc_client.set_crouching(player_state.is_sneaking);
    };

    if !player_state.next_click_no_attack && dig_pressed && !player_state.previous_dig_held {
        attack_crosshair(mc_client);
    }

    // if we previously already let go of the button and didn't press it right now either, reset next_no_attack
    if !player_state.previous_dig_held && !dig_pressed {
        player_state.next_click_no_attack = false;
    }

    player_state.previous_dig_held = dig_pressed
}

const ATTACK_MAX_DIST: f32 = 10.0;

fn entity_in_crosshair(candidate: (&Position, &Physics), line: (Vec3, Vec3)) -> bool {
    let (position, physics) = candidate;
    // fail early instead of failing with the slower liang–barsky algorithm later
    if (line.0.distance_to(**position) > ATTACK_MAX_DIST.into()) {
        return false;
    }
    // check if the bounding box is on the line-of-sight
    let bounding_box = physics.bounding_box;
    if utils::liang_barsky_3d(bounding_box, line.0, line.1) {
        return true;
    };
    return false;
}

pub fn attack_crosshair(mc_client: &mut Client) {
    let line_origin = mc_client.eye_position().unwrap();

    // convert view direction to radians
    let look_direction = mc_client.direction().unwrap();
    let yaw = utils::normalize_angle(look_direction.y_rot()) * (PI / 180.0);
    let pitch = utils::normalize_angle(look_direction.x_rot()) * (PI / 180.0);

    // Calculate the end point of the line
    let dx = ATTACK_MAX_DIST * pitch.cos() * -yaw.sin();
    let dy = -ATTACK_MAX_DIST * pitch.sin();
    let dz = ATTACK_MAX_DIST * pitch.cos() * yaw.cos();

    let line_end = Vec3 {
        x: line_origin.x + dx as f64,
        y: line_origin.y + dy as f64,
        z: line_origin.z + dz as f64,
    };

    // Get closest entity that matches entity_in_crosshair and isn't dead or local
    let entity = mc_client
        .nearest_entity_by::<(&Position, &Physics), (Without<LocalEntity>, Without<Dead>)>(
            |e: (&Position, &Physics)| entity_in_crosshair(e.clone(), (line_origin, line_end)),
        );
    if let Ok(Some(entity)) = entity {
        mc_client.attack(entity.id())
    }
}

// This function only validates the interaction, then splits by node/object
pub async fn interact(
    mc_client: &mut Client,
    specbox: Box<InteractSpec>,
    player_state: &mut state::PlayerState,
) {
    let InteractSpec {
        action,
        item_index: _,
        pointed_thing,
        player_pos: _,
    } = *specbox;
    match pointed_thing {
        PointedThing::Nothing => (), // TODO might still be relevant in some cases (eating), check that
        PointedThing::Node {
            under_surface,
            above_surface,
        } => {
            c2s::world::interact_node(
                action,
                under_surface,
                above_surface,
                mc_client,
                player_state,
            )
            .await
        }
        _ => warn!("Client tried to interact with object, this is not yet supported!",),
    }
}
