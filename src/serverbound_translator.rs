// this contains functions that TAKE data from the client
// and send it to the MC server.
use crate::utils;

use azalea_client::Client;
use azalea_protocol::packets::game::ServerboundGamePacket;
use azalea_protocol::packets::game::serverbound_move_player_pos_rot_packet::ServerboundMovePlayerPosRotPacket;
use alloc::boxed::Box;
use minetest_protocol::wire::command::{TSChatMessageSpec, PlayerposSpec};
use minetest_protocol::wire::types::{PlayerPos, v3f};

use tokio::sync::mpsc::UnboundedReceiver;
use azalea_client::Event;

pub fn send_message(mc_client: &Client, specbox: Box<TSChatMessageSpec>) {
    utils::logger("[Minetest] C->S Forwarding Message sent by client", 1);
    let TSChatMessageSpec { message } = *specbox;
    mc_client.chat(&message);
}

pub async fn playerpos(mc_client: &Client, specbox: Box<PlayerposSpec>) {
    let PlayerposSpec { player_pos } = *specbox; // vvv what does this do please let me ignore it
    let PlayerPos { position, speed, pitch, yaw, keys_pressed: _, fov: _, wanted_range: _ } = player_pos;
    let v3f {x, y, z } = position;
    // this will need to be handled by manually sending
    // a ServerboundMovePlayerPosRotPacket over the mc_conn, the
    // azalea-client library does not give the user direct access
    // to the vectors that will be sent and translating MT C->S movement vectors
    // into whatever azalea is using is too difficult.

    // y_rot: yaw
    // x_rot: pitch
    // source: https://en.wikipedia.org/wiki/Aircraft_principal_axes
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
    // TODO send that thing over the raw connection
    let _ = mc_client.write_packet(movement_packet);
}
