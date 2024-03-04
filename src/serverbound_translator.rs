// this contains functions that TAKE data from the client
// and send it to the MC server.
use crate::utils;

use azalea_client::Client;
use azalea_protocol::packets::game::ServerboundGamePacket;
use azalea_protocol::packets::game::serverbound_move_player_pos_rot_packet::ServerboundMovePlayerPosRotPacket;
use alloc::boxed::Box;
use minetest_protocol::wire::command::{TSChatMessageSpec, PlayerposSpec};
use minetest_protocol::wire::types::{PlayerPos, v3f};
use crate::MTServerState;

pub fn send_message(mc_client: &Client, specbox: Box<TSChatMessageSpec>) {
    utils::logger("[Minetest] C->S Forwarding Message sent by client", 1);
    let TSChatMessageSpec { message } = *specbox;
    mc_client.chat(&message);
}

pub async fn playerpos(mc_client: &mut Client, specbox: Box<PlayerposSpec>, mt_server_state: &mut MTServerState) {
    let PlayerposSpec { player_pos } = *specbox;
    let PlayerPos { position, speed: _, pitch, yaw, keys_pressed, fov: _, wanted_range: _ } = player_pos;
    let v3f {x, y, z } = position;
    // this will need to be handled by manually sending
    // a ServerboundMovePlayerPosRotPacket over the mc_conn, the
    // azalea-client library does not give the user direct access
    // to the vectors that will be sent and translating MT C->S movement vectors
    // into whatever azalea is using is too difficult.

    // y_rot: yaw
    // x_rot: pitch
    
    // keys_pressed:
    // https://github.com/minetest/minetest/blob/e734b3f0d8055ff3ae710f3632726a711603bf84/src/player.cpp#L217
    
    let direction_keys = keys_pressed & 0xf;
    let up_pressed    = ((direction_keys >> 0) & 1) != 0;
    let down_pressed  = ((direction_keys >> 1) & 1) != 0;
    let left_pressed  = ((direction_keys >> 2) & 1) != 0;
    let right_pressed = ((direction_keys >> 3) & 1) != 0;

    //let jump_pressed  = (keys_pressed & (1 << 4)) != 0;
    let aux1_pressed  = (keys_pressed & (1 << 5)) != 0;
    let sneak_pressed = (keys_pressed & (1 << 6)) != 0;
    //let dig_pressed   = (keys_pressed & (1 << 7)) != 0;
    //let place_pressed = (keys_pressed & (1 << 8)) != 0;
    //let zoom_pressed  = (keys_pressed & (1 << 9)) != 0;
    
    if mt_server_state.is_sneaking != sneak_pressed {
        // player started/stopped sneaking, update the mc client
        // TODO: wait on upstream. 27-02-2024 the feature was confirmed, but its not yet on github

        //mt_server_state.is_sneaking = sneak_pressed;
        //mc_client.sneak(sneak_pressed);
    }
    
    if keys_pressed != mt_server_state.keys_pressed {
        match (aux1_pressed, up_pressed, down_pressed, left_pressed, right_pressed) {
            (false, true, _, true, _) => mc_client.walk(azalea::WalkDirection::ForwardLeft),
            (false, true, _, _, true) => mc_client.walk(azalea::WalkDirection::ForwardRight),
            (false, true, _, _, _)    => mc_client.walk(azalea::WalkDirection::Forward),
            (false, _, true, true, _) => mc_client.walk(azalea::WalkDirection::BackwardLeft),
            (false, _, true, _, true) => mc_client.walk(azalea::WalkDirection::BackwardRight),
            (false, _, true, _, _)    => mc_client.walk(azalea::WalkDirection::Backward),
            (false, _, _, true, _)    => mc_client.walk(azalea::WalkDirection::Left),
            (false, _, _, _, false)   => mc_client.walk(azalea::WalkDirection::Right),
            (true, true, _, true, _)  => mc_client.sprint(azalea::SprintDirection::ForwardLeft),
            (true, true, _, _, true)  => mc_client.sprint(azalea::SprintDirection::ForwardRight),
            (true, true, _, _, _)     => mc_client.sprint(azalea::SprintDirection::Forward),
            _ => mc_client.walk(azalea::WalkDirection::None),
        }
        mt_server_state.keys_pressed = keys_pressed;
    }

    if (yaw, pitch) != mt_server_state.last_yaw_pitch {
        let movement_packet = ServerboundGamePacket::MovePlayerPosRot {
            0: ServerboundMovePlayerPosRotPacket {
                x: (x as f64) / 10.0,
                y: (y as f64) / 10.0,
                z: (z as f64) / 10.0,
                y_rot: yaw,
                x_rot: pitch,
                on_ground: true // i don't know, thats why the server needs to not have an anticheat
            }
        };
        let _ = mc_client.write_packet(movement_packet);
        mt_server_state.mt_clientside_pos = (x, y, z);
        mt_server_state.last_yaw_pitch = (yaw, pitch);
    }
}
