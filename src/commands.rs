/*
 * This file contains functions that perform specific actions
 * between the MT client and the MC server
 * For example the handshake function, which also creates and returns a
 * Minecraft client.
 */

use crate::utils;
use crate::settings;

use minetest_protocol::wire::command::ToServerCommand;
use minetest_protocol::MinetestConnection;
use minetest_protocol::wire::command::ToClientCommand;
use minetest_protocol::wire::command::HelloSpec;
use minetest_protocol::wire::command::AuthAcceptSpec;
use minetest_protocol::wire::types;

use azalea;
use azalea_client::{Client, Account};

use tokio::sync::mpsc::UnboundedReceiver;

pub async fn handshake(_command: ToServerCommand, _conn: &mut MinetestConnection) -> (azalea::Client, UnboundedReceiver<azalea::Event>) {
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
    println!("[Minecraft] Logging in...");
    
    // TODO: Change this line to allow online accounts
    let mc_account: Account = Account::offline("DEBUG");
    let (mc_client, rx) = Client::join(&mc_account, settings::MC_SERVER_ADDR).await.expect("[Minecraft] Failed to log in!");
    return (mc_client, rx)
}
