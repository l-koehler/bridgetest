/*
 * This file contains functions that perform specific actions with the MT client
 */
use crate::utils;

use minetest_protocol::wire::command::ToServerCommand;
use minetest_protocol::MinetestConnection;

// commands that can be sent to the client
use minetest_protocol::wire::command::ToClientCommand;
use minetest_protocol::wire::command::HelloSpec;
use minetest_protocol::wire::command::AuthAcceptSpec;
use minetest_protocol::wire::types;

pub async fn handshake(_command: ToServerCommand, _conn: &mut MinetestConnection) {
    // Got called by C->S Init
    // Send S->C Hello
    let hello_command = ToClientCommand::Hello(
        Box::new(HelloSpec {
            serialization_ver: 0,
            compression_mode: 0,
            proto_ver: 44,
            auth_mechs: types::AuthMechsBitset {
                legacy_password: false,
                srp: false,
                first_srp: true,
            },
            username_legacy: "DEBUG".to_string(),
        })
    );
    let _ = _conn.send(hello_command).await;
    println!("[Minetest] S->C Hello");
    // Wait for a C->S FirstSrp
    // TODO: this is right now just assuming the response is part of the authentication
    let second_response = _conn.recv().await.expect("Client disconnected during authentication!");
    utils::show_mt_command(&second_response);
    // Send S->C AuthAccept
    let auth_accept_command = ToClientCommand::AuthAccept(
        Box::new(AuthAcceptSpec {
            player_pos: types::v3f {
                 x: 0.0,
                 y: 0.0,
                 z: 90.0,
            },
            map_seed: 0,
            recommended_send_interval: 0.1,
            sudo_auth_methods: 0,
        })
    );
    let _ = _conn.send(auth_accept_command).await;
    println!("[Minetest] S->C AuthAccept");
}
