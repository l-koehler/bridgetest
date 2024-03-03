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
    
    if keys_pressed != 0 {
        let direction_keys = keys_pressed & 0xf;
        let up_pressed    = ((direction_keys >> 0) & 1) != 0;
        let down_pressed  = ((direction_keys >> 1) & 1) != 0;
        let left_pressed  = ((direction_keys >> 2) & 1) != 0;
        let right_pressed = ((direction_keys >> 3) & 1) != 0;
        let any_pressed = up_pressed || down_pressed || left_pressed || right_pressed;

        //let jump_pressed  = (keys_pressed & (1 << 4)) != 0;
        let aux1_pressed  = (keys_pressed & (1 << 5)) != 0; // i think thats sprint, not sure
        let sneak_pressed = (keys_pressed & (1 << 6)) != 0;
        //let dig_pressed   = (keys_pressed & (1 << 7)) != 0;
        //let place_pressed = (keys_pressed & (1 << 8)) != 0;
        //let zoom_pressed  = (keys_pressed & (1 << 9)) != 0;
        
        if mt_server_state.is_sneaking != sneak_pressed {
            // player started/stopped sneaking, update the mc client
            mt_server_state.is_sneaking = sneak_pressed;
            // TODO: wait on upstream. 27-02-2024 the feature was confirmed, but its not yet on github
        }
        
        // not really sure how to fix this elseif hell
        else if aux1_pressed && any_pressed { // sprinting
            if up_pressed &&left_pressed {
                mc_client.sprint(azalea::SprintDirection::ForwardLeft)
            } else if up_pressed && right_pressed {
                mc_client.sprint(azalea::SprintDirection::ForwardRight)
            } else if up_pressed {
                mc_client.sprint(azalea::SprintDirection::Forward)
            }
        } else if any_pressed { // walking
            if up_pressed && left_pressed {
                mc_client.walk(azalea::WalkDirection::ForwardLeft)
            } else if up_pressed && right_pressed {
                mc_client.walk(azalea::WalkDirection::ForwardRight)
            } else if up_pressed {
                mc_client.walk(azalea::WalkDirection::Forward)
            } else if down_pressed && left_pressed {
                mc_client.walk(azalea::WalkDirection::BackwardLeft)
            } else if down_pressed && right_pressed {
                mc_client.walk(azalea::WalkDirection::BackwardRight)
            } else if down_pressed {
                mc_client.walk(azalea::WalkDirection::Backward)
            } else if left_pressed {
                mc_client.walk(azalea::WalkDirection::Left)
            } else if right_pressed {
                mc_client.walk(azalea::WalkDirection::Right)
            }
        } else {
            mc_client.walk(azalea::WalkDirection::None)
        }
    }
    
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
}
